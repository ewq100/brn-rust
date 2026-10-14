//! Host-driven intake: the host searches the run's chat from the supplied
//! source and puts the source and the related pages in the first prompt. A
//! scripted model drives the real tools over the real search worker; no
//! provider is contacted.
use brn_ai::threads::{
    PreparedThreadRun, ThreadRunRequest, TraceLevel, TransientSource, prepare_run,
    run_prepared_scripted,
};
use brn_ai::{AiTerminal, Provider, ReasoningEffort, Selection};
use brn_threads_app::search::SearchHandle;
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const KITCHEN: &str = "# Kitchen renovation\n\nBudget 9,000 EUR. Electrician: to be chosen.\n";
const SOURCE: &str = "Email from Tonis Kask (electrician), 5 October 2026\n\nRewiring the kitchen will cost 3,800 EUR. The fuse board must be replaced first.\n";
const REFERENCE: &str = "synthetic://electrician-email";

struct World {
    _root: tempfile::TempDir,
    data: PathBuf,
    workspace: Workspace,
    handle: SearchHandle,
    chat: String,
    kitchen: String,
    private: String,
}

fn world(kitchen_text: &str) -> World {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut workspace = Workspace::open(&data).unwrap();
    let settings = workspace.settings().unwrap();
    workspace
        .configure(WorkspaceSettings {
            provider: "Codex".into(),
            model: "gpt-6.1-sol".into(),
            effort: "medium".into(),
            auth_file: None,
            credentials_dir: None,
            ..settings
        })
        .unwrap();
    workspace
        .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap();
    let chat = workspace.new_thread("House").unwrap().id;
    let other = workspace.new_thread("Other chat").unwrap().id;
    let owner = HostAuthority::owner("synthetic host intake fixture");
    let op = workspace
        .store
        .allocate_operation("host-intake:seed")
        .unwrap();
    let mut writes = vec![
        Note::working("Kitchen renovation", kitchen_text),
        Note {
            chat: Some(other),
            ..Note::working(
                "Private electrician",
                "zzprivatemarker Tonis Kask electrician kitchen rewiring quote.\n",
            )
        },
    ];
    writes.extend((0..150).map(|i| {
        Note::working(
            format!("Harbor project {i}"),
            format!("# Harbor project {i}\n\nQuay logistics record {i}.\n"),
        )
    }));
    let writes = writes
        .into_iter()
        .enumerate()
        .map(|(j, note)| Put {
            id: op.creation_id(j),
            expected_version: None,
            archived: false,
            data: RecordData::Note(note),
        })
        .collect();
    workspace
        .store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Synthetic notes".into(),
                writes,
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = workspace.store.apply(&op, &owner).unwrap() else {
        panic!("seed not applied");
    };
    let id_of = |title: &str| {
        receipt
            .writes
            .iter()
            .find(|w| matches!(&w.after.data, RecordData::Note(n) if n.title == title))
            .unwrap()
            .after
            .id
            .clone()
    };
    let (kitchen, private) = (id_of("Kitchen renovation"), id_of("Private electrician"));
    let handle = workspace.search_handle().unwrap();
    handle.prepare().wait().unwrap();
    World {
        _root: root,
        data,
        workspace,
        handle,
        chat,
        kitchen,
        private,
    }
}

fn request(w: &World, source: &str, search: bool) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: w.data.clone(),
        thread: w.chat.clone(),
        prompt: "Process the attached source and keep what helps current work.".into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: 12,
        auth_file: None,
        credentials_dir: None,
        sources: vec![TransientSource {
            reference: REFERENCE.into(),
            text: source.into(),
            fingerprint: Some("v1".into()),
            report: None,
            assets: vec![],
        }],
        search: search.then(|| Arc::new(w.handle.clone()) as _),
        trace: TraceLevel::Full,
    }
}

fn call(id: &str, name: &str, args: Value) -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::tool_call(id, name, args),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}

fn finish(text: &str) -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::text(text),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}

fn operation(data: &Path, prepared: &PreparedThreadRun, key: &str) -> String {
    Store::open(data)
        .unwrap()
        .allocate_operation(&format!("{}:proposal:{key}", prepared.run_id()))
        .unwrap()
        .as_str()
        .to_owned()
}

/// The Source record the run created for the supplied source, and its version.
fn source_record(data: &Path, chat: &str) -> (String, u64) {
    let store = Store::open(data).unwrap();
    let record = store
        .records_for_chat(Some(chat))
        .unwrap()
        .into_iter()
        .find(|r| matches!(&r.data, RecordData::Source(s) if s.locator == REFERENCE))
        .expect("the run records its source");
    (record.id, record.version)
}

/// The first prompt the model saw, as debug text.
fn first_prompt(model: &MockCompletionModel) -> String {
    format!("{:?}", model.requests()[0].chat_history)
}

fn update(note: &str, version: u64, source: &(String, u64)) -> Value {
    let markdown = format!(
        "{KITCHEN}\nElectrician Tonis Kask quoted 3,800 EUR for rewiring; the fuse board is replaced first.\n"
    );
    json!({
        "reason":"Record the electrician quote",
        "writes":[{"id":note,"expected_version":version,"archived":false,
            "data":{"Note":{"title":"Kitchen renovation","markdown":markdown,"protected":false,
                "confirmed":false,"superseded_by":null,"import":null}}}],
        "inputs":[{"record":note,"version":version},{"record":source.0,"version":source.1}]
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn the_first_prompt_carries_the_source_and_the_related_page_at_its_version() {
    let w = world(KITCHEN);
    let prepared = prepare_run(request(&w, SOURCE, true)).unwrap();
    let source = source_record(&w.data, &w.chat);
    let model = MockCompletionModel::from_stream_turns([finish("Kept in the chat.")]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    let prompt = first_prompt(&model);
    assert!(prompt.contains("Rewiring the kitchen will cost 3,800 EUR"));
    assert!(prompt.contains(&w.kitchen) && prompt.contains("Budget 9,000 EUR"));
    assert!(prompt.contains(&source.0));
    let trace = result.trace.unwrap();
    let context = trace.host_context.expect("host context traced");
    assert_eq!(context["retrieval"], "ok");
    assert_eq!(context["pages"][0]["id"], w.kitchen.as_str());
    assert_eq!(context["pages"][0]["version"], 1);
    assert!(context["queries"].as_u64().unwrap() >= 2);
    assert!(trace.calls.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_scripted_model_completes_the_intake_in_two_rounds() {
    let w = world(KITCHEN);
    let prepared = prepare_run(request(&w, SOURCE, true)).unwrap();
    let source = source_record(&w.data, &w.chat);
    let op = operation(&w.data, &prepared, "quote");
    let model = MockCompletionModel::from_stream_turns([
        call(
            "p",
            "prepare_changes",
            json!({"request_key":"quote","request":update(&w.kitchen, 1, &source)}),
        ),
        call("a", "apply_changes", json!({"operation":op})),
        finish("Recorded the electrician quote."),
    ]);
    let result = run_prepared_scripted(prepared, model).await.unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer
    );
    let trace = result.trace.unwrap();
    let calls: Vec<(&str, u16, &str)> = trace
        .calls
        .iter()
        .map(|c| (c.tool.as_str(), c.round, c.outcome.as_str()))
        .collect();
    assert_eq!(
        calls,
        [
            ("prepare_changes", 1, "success"),
            ("apply_changes", 2, "Applied")
        ],
        "{:?}",
        trace.calls
    );
    let RecordData::Note(note) = w.workspace.store.record(&w.kitchen).unwrap().unwrap().data else {
        panic!()
    };
    assert!(note.markdown.contains("3,800 EUR"));
    assert_eq!(
        w.workspace
            .store
            .record(&w.kitchen)
            .unwrap()
            .unwrap()
            .version,
        2
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn another_chats_private_note_never_reaches_the_prompt_or_the_trace() {
    let w = world(KITCHEN);
    let prepared = prepare_run(request(&w, SOURCE, true)).unwrap();
    let model = MockCompletionModel::from_stream_turns([finish("Kept in the chat.")]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    let prompt = first_prompt(&model);
    let trace = serde_json::to_string(&result.trace.unwrap()).unwrap();
    for text in [&prompt, &trace] {
        assert!(!text.contains(&w.private), "{text}");
        assert!(!text.contains("Private electrician") && !text.contains("zzprivatemarker"));
    }
    // The shared search would match it by its words, so the scope filter is
    // what keeps it out.
    assert_eq!(
        w.handle
            .search("zzprivatemarker", Some(&w.chat), 5)
            .unwrap()
            .total,
        0
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn source_and_page_sizes_are_capped_and_the_rest_stays_reachable() {
    let big_page = format!(
        "{KITCHEN}\n{}",
        "Cabinet fitting detail kitchen. ".repeat(6_000)
    );
    let w = world(&big_page);
    let big_source = format!(
        "{SOURCE}\n{}",
        "Further kitchen electrician note. ".repeat(8_000)
    );
    let prepared = prepare_run(request(&w, &big_source, true)).unwrap();
    let model = MockCompletionModel::from_stream_turns([finish("Kept in the chat.")]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    let prompt = first_prompt(&model);
    let trace = result.trace.unwrap();
    let context = trace.host_context.expect("host context traced");
    let bytes = context["prompt_bytes"].as_u64().unwrap();
    assert!(
        bytes < 64 * 1024,
        "the context stays bounded: {bytes} bytes for a {} byte source and a {} byte page",
        big_source.len(),
        big_page.len()
    );
    assert!(prompt.contains("text_truncated") && prompt.contains("read_source"));
    assert!(prompt.contains("next_start") && prompt.contains("read_note"));
    assert!(context["pages"][0]["bytes"].as_u64().unwrap() <= 20 * 1024);
}

#[tokio::test(flavor = "multi_thread")]
async fn without_search_the_source_is_still_present_and_the_tools_remain() {
    let w = world(KITCHEN);
    let prepared = prepare_run(request(&w, SOURCE, false)).unwrap();
    let model = MockCompletionModel::from_stream_turns([finish("Kept in the chat.")]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    let prompt = first_prompt(&model);
    assert!(prompt.contains("Rewiring the kitchen will cost 3,800 EUR"));
    assert!(prompt.contains("unavailable"));
    assert!(!prompt.contains("Budget 9,000 EUR"));
    assert_eq!(
        result.trace.unwrap().host_context.unwrap()["retrieval"],
        "unavailable"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chat_without_sources_gets_no_host_context() {
    let w = world(KITCHEN);
    let mut plain = request(&w, SOURCE, true);
    plain.sources.clear();
    let prepared = prepare_run(plain).unwrap();
    let model = MockCompletionModel::from_stream_turns([finish("Hello.")]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(result.trace.unwrap().host_context.is_none());
    let prompt = first_prompt(&model);
    assert!(!prompt.contains("Host intake context"));
}
