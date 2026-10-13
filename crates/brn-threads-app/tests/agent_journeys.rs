//! Scripted agent journeys at 10,000 synthetic notes (plan Task 3). A
//! scripted model drives the real runtime tools; the host search handle is
//! the real worker over the persisted index. No provider is contacted.
use brn_ai::threads::{PreparedThreadRun, ThreadRunRequest, prepare_run, run_prepared_scripted};
use brn_ai::{AiTerminal, Provider, ReasoningEffort, Selection};
use brn_threads_app::search::SearchHandle;
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const NOTES: usize = 10_000;
const TARGET: &str = "# Harbor berth\n\nThe berth serves feeder vessels.\n\n## Loading depth\n\nThe quay needs at least 1.2 metres of dredged depth before loading.\n";

struct World {
    _root: tempfile::TempDir,
    data: PathBuf,
    workspace: Workspace,
    handle: SearchHandle,
    chat: String,
    target: String,
    private: String,
}

fn world() -> World {
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
    let chat = workspace.new_thread("Harbor questions").unwrap().id;
    let other = workspace.new_thread("Private chat").unwrap().id;
    let owner = HostAuthority::owner("synthetic journey fixture");
    let mut target = String::new();
    let mut private = String::new();
    for batch in 0..NOTES / 500 {
        let op = workspace
            .store
            .allocate_operation(&format!("journey-batch:{batch}"))
            .unwrap();
        let writes = (0..500)
            .map(|j| {
                let i = batch * 500 + j;
                let note = match i {
                    4321 => Note::working("Harbor berth", TARGET),
                    7000 => Note {
                        chat: Some(other.clone()),
                        ..Note::working("Private attachment", "zzprivatemarker quay depth.\n")
                    },
                    _ => Note::working(
                        format!("Project {i}"),
                        format!(
                            "# Project {i}\n\nHarbor logistics record {i}. Loading is planned by week {}.\n",
                            i % 52
                        ),
                    ),
                };
                Put {
                    id: op.creation_id(j),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(note),
                }
            })
            .collect();
        let request = ChangeRequest {
            reason: "Synthetic journey notes".into(),
            writes,
            inputs: vec![],
        };
        workspace
            .store
            .prepare_owner(&op, &request, &owner)
            .unwrap();
        let ApplyOutcome::Applied(receipt) = workspace.store.apply(&op, &owner).unwrap() else {
            panic!("batch not applied");
        };
        for write in receipt.writes {
            if let RecordData::Note(note) = &write.after.data {
                match note.title.as_str() {
                    "Harbor berth" => target = write.after.id.clone(),
                    "Private attachment" => private = write.after.id.clone(),
                    _ => {}
                }
            }
        }
    }
    let handle = workspace.search_handle().unwrap();
    handle.prepare().wait().unwrap();
    World {
        _root: root,
        data,
        workspace,
        handle,
        chat,
        target,
        private,
    }
}

fn request(data: &Path, chat: &str, handle: &SearchHandle, rounds: u16) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: data.to_owned(),
        thread: chat.to_owned(),
        prompt: "How deep must the quay be dredged before loading?".into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: rounds,
        auth_file: None,
        credentials_dir: None,
        sources: vec![],
        search: Some(Arc::new(handle.clone())),
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

fn search(query: &str) -> Value {
    json!({"query":query,"limit":5,"mode":"hybrid","cursor":null})
}

/// Allocates the operation ID a run will use for a `proposal` or `refresh`
/// with `request_key`, so the script can apply it (allocation by host key
/// is idempotent).
fn operation(data: &Path, prepared: &PreparedThreadRun, kind: &str, key: &str) -> String {
    Store::open(data)
        .unwrap()
        .allocate_operation(&format!("{}:{kind}:{key}", prepared.run_id()))
        .unwrap()
        .as_str()
        .to_owned()
}

fn change(note: &str, version: u64, markdown: &str) -> Value {
    json!({
        "reason":"Record the confirmed loading depth",
        "writes":[{"id":note,"expected_version":version,"archived":false,
            "data":{"Note":{"title":"Harbor berth","markdown":markdown,"protected":false,
                "confirmed":false,"superseded_by":null,"import":null}}}],
        "inputs":[{"record":note,"version":version}]
    })
}

/// The newest message of each model request: the prompt first, then the
/// result of the preceding tool call.
fn seen(model: &MockCompletionModel) -> Vec<String> {
    model
        .requests()
        .iter()
        .map(|request| format!("{:?}", request.chat_history.last()))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn retrieval_finds_and_reads_the_target_section_within_three_rounds() {
    let w = world();
    let best = w
        .handle
        .search("quay dredged depth", Some(&w.chat), 5)
        .unwrap()
        .matches
        .remove(0);
    assert_eq!(
        (best.note.as_str(), best.heading_path.as_str()),
        (w.target.as_str(), "Harbor berth › Loading depth")
    );
    let model = MockCompletionModel::from_stream_turns([
        call("s", "search_notes", search("quay dredged depth")),
        call(
            "r",
            "read_note",
            json!({"id":w.target,"version":best.version,"start":best.start,"max_bytes":2048,"section":false}),
        ),
        finish("At least 1.2 metres."),
    ]);
    let prepared = prepare_run(request(&w.data, &w.chat, &w.handle, 3)).unwrap();
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer
    );
    let seen = seen(&model);
    assert!(seen[1].contains(&w.target) && seen[1].contains("Loading depth"));
    assert!(seen[2].contains("1.2 metres"));
}

#[tokio::test(flavor = "multi_thread")]
async fn maintenance_applies_within_eight_rounds_and_finds_its_own_change() {
    let w = world();
    let prepared = prepare_run(request(&w.data, &w.chat, &w.handle, 8)).unwrap();
    let op = operation(&w.data, &prepared, "proposal", "depth");
    let updated = format!("{TARGET}\nConfirmed by the survey: zzsurveyconfirmed.\n");
    let model = MockCompletionModel::from_stream_turns([
        call("s", "search_notes", search("quay loading depth")),
        call(
            "r",
            "read_note",
            json!({"id":w.target,"version":1,"start":0,"max_bytes":4096,"section":false}),
        ),
        call(
            "p",
            "prepare_changes",
            json!({"request_key":"depth","request":change(&w.target, 1, &updated)}),
        ),
        call("a", "apply_changes", json!({"operation":op})),
        call("f", "search_notes", search("zzsurveyconfirmed")),
        finish("Recorded."),
    ]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer
    );
    let note = w.workspace.store.record(&w.target).unwrap().unwrap();
    assert_eq!(note.version, 2);
    let seen = seen(&model);
    assert!(
        seen[4].contains(r#""outcome": String("Applied")"#),
        "{}",
        seen[4]
    );
    assert!(seen[5].contains(&w.target));
    assert!(seen[5].contains("zzsurveyconfirmed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stale_candidate_is_refreshed_from_current_search_results() {
    let mut w = world();
    // First run prepares a candidate and stops.
    let first = prepare_run(request(&w.data, &w.chat, &w.handle, 8)).unwrap();
    let old = operation(&w.data, &first, "proposal", "depth");
    let model = MockCompletionModel::from_stream_turns([
        call("s", "search_notes", search("quay loading depth")),
        call(
            "p",
            "prepare_changes",
            json!({"request_key":"depth","request":change(&w.target, 1, &format!("{TARGET}\nDraft.\n"))}),
        ),
        finish("Prepared."),
    ]);
    run_prepared_scripted(first, model).await.unwrap();
    // The owner saves meanwhile, so the candidate is stale.
    let session = w.workspace.store.begin_edit(&w.target, 1).unwrap();
    let owner_text = format!("{TARGET}\nOwner note: zzownerline.\n");
    w.workspace
        .store
        .update_buffer(&session.id, 1, &owner_text)
        .unwrap();
    w.workspace.store.save(&session.id, 1).unwrap();
    let second = prepare_run(request(&w.data, &w.chat, &w.handle, 8)).unwrap();
    let fresh = operation(&w.data, &second, "refresh", "depth-2");
    let refreshed = format!("{owner_text}\nSurvey: zzrefreshedline.\n");
    let model = MockCompletionModel::from_stream_turns([
        call("a", "apply_changes", json!({"operation":old})),
        call("s", "search_notes", search("zzownerline")),
        call(
            "r",
            "read_note",
            json!({"id":w.target,"version":2,"start":0,"max_bytes":4096,"section":false}),
        ),
        call(
            "f",
            "refresh_candidate",
            json!({"operation":old,"request_key":"depth-2","request":change(&w.target, 2, &refreshed)}),
        ),
        call("b", "apply_changes", json!({"operation":fresh})),
        finish("Refreshed and applied."),
    ]);
    let result = run_prepared_scripted(second, model.clone()).await.unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer
    );
    let seen = seen(&model);
    assert!(seen[1].contains(r#""outcome": String("Stale")"#));
    assert!(seen[2].contains(&w.target) && seen[2].contains("Number(2)"));
    assert!(
        seen[5].contains(r#""outcome": String("Applied")"#),
        "{}",
        seen[5]
    );
    let RecordData::Note(note) = w.workspace.store.record(&w.target).unwrap().unwrap().data else {
        panic!()
    };
    assert!(note.markdown.contains("zzownerline") && note.markdown.contains("zzrefreshedline"));
}

#[tokio::test(flavor = "multi_thread")]
async fn another_chats_private_note_stays_invisible_to_search_and_connections() {
    let w = world();
    let model = MockCompletionModel::from_stream_turns([
        call("s", "search_notes", search("zzprivatemarker")),
        call(
            "c",
            "note_connections",
            json!({"id":w.private,"hops":1,"sources":true}),
        ),
        finish("Nothing visible."),
    ]);
    let prepared = prepare_run(request(&w.data, &w.chat, &w.handle, 3)).unwrap();
    run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    let seen = seen(&model);
    assert!(seen.len() >= 2);
    for text in &seen[1..] {
        assert!(!text.contains("Private attachment"), "{text}");
    }
    assert_eq!(
        w.handle
            .search("zzprivatemarker", Some(&w.chat), 5)
            .unwrap()
            .total,
        0
    );
}
