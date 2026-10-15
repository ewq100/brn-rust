//! Useful intake of the same source reuses the chat that already received it,
//! within the same project scope. Synthetic data; a scripted model answers.
use brn_ai::threads::{ThreadRunRequest, TransientSource, prepare_run, run_prepared_scripted};
use brn_ai::{Provider, ReasoningEffort, Selection};
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use std::path::PathBuf;

const LOCATOR: &str = "/synthetic/mari-crane-email.txt";
const TITLE: &str = "Useful information intake";

fn workspace() -> (tempfile::TempDir, PathBuf, Workspace) {
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
    (root, data, workspace)
}

fn project(workspace: &mut Workspace, title: &str) -> String {
    workspace
        .project_command(
            &format!("project-{title}"),
            ProjectCommand::Create {
                title: title.into(),
            },
        )
        .unwrap()
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Project(_)))
        .unwrap()
        .after
        .id
}

fn request(data: &std::path::Path, chat: &str) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: data.to_owned(),
        thread: chat.to_owned(),
        prompt: "Retain useful information from the supplied source.".into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: 12,
        auth_file: None,
        credentials_dir: None,
        sources: vec![TransientSource::text(
            LOCATOR,
            "Can we move crane delivery to 15 November?",
        )],
        search: None,
        trace: brn_ai::threads::TraceLevel::Off,
    }
}

/// One useful intake run of the synthetic email in `chat`.
async fn supply(data: &std::path::Path, chat: &str) {
    let prepared = prepare_run(request(data, chat)).unwrap();
    let model = MockCompletionModel::from_stream_turns([vec![
        MockStreamEvent::text("Recorded."),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]]);
    run_prepared_scripted(prepared, model).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_same_source_reuses_its_chat_per_scope_and_never_an_archived_or_foreign_one() {
    let (_root, data, mut w) = workspace();
    let harbor = project(&mut w, "Harbor crane");
    let depot = project(&mut w, "Depot");

    // General: the first supply creates a chat, the next one reuses it.
    let (general, reused) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    assert!(!reused);
    supply(&data, &general.id).await;
    let records = w.records().unwrap().len();
    let receipts = w.history().unwrap().len();
    let (again, reused) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    assert_eq!((again.id.as_str(), reused), (general.id.as_str(), true));
    assert_eq!(w.records().unwrap().len(), records, "reuse adds no records");
    assert_eq!(w.history().unwrap().len(), receipts);

    // A project gets its own chat for the same locator, then reuses it.
    let (harbor_chat, reused) = w.source_intake_chat(Some(&harbor), LOCATOR, TITLE).unwrap();
    assert!(!reused);
    assert_ne!(harbor_chat.id, general.id);
    assert_eq!(
        project_for_chat(&w.records().unwrap(), &harbor_chat.id).map(|p| p.id.clone()),
        Some(harbor.clone())
    );
    supply(&data, &harbor_chat.id).await;
    let (again, reused) = w.source_intake_chat(Some(&harbor), LOCATOR, TITLE).unwrap();
    assert_eq!((again.id.as_str(), reused), (harbor_chat.id.as_str(), true));
    // Its Source stays private to that chat.
    assert!(w.records().unwrap().iter().any(|r| matches!(&r.data,
        RecordData::Source(s) if s.locator == LOCATOR && s.chat.as_deref() == Some(harbor_chat.id.as_str()))));
    // The general scope still reuses the general chat.
    assert_eq!(
        w.source_intake_chat(None, LOCATOR, TITLE).unwrap().0.id,
        general.id
    );

    // Another project's chat is never reused.
    let (depot_chat, reused) = w.source_intake_chat(Some(&depot), LOCATOR, TITLE).unwrap();
    assert!(!reused);
    assert!(depot_chat.id != harbor_chat.id && depot_chat.id != general.id);

    // An archived chat is never reused.
    w.archive_chat("archive-general", &general.id).unwrap();
    let (fresh, reused) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    assert!(!reused);
    assert_ne!(fresh.id, general.id);

    // An unknown project is refused.
    let missing = w
        .source_intake_chat(Some("missing"), LOCATOR, TITLE)
        .unwrap_err();
    assert!(missing.to_string().contains("active project"), "{missing}");
}

fn run(workspace: &Workspace, id: &str) -> Run {
    match workspace.store.record(id).unwrap().unwrap().data {
        RecordData::Run(run) => run,
        _ => panic!("Run"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chat_whose_run_is_still_live_is_not_reused_until_its_lease_is_free() {
    let (_root, data, mut w) = workspace();
    let (original, _) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    supply(&data, &original.id).await;
    // Another process is still working on this source there: its prepared
    // Run is Working and holds the lease through its own file handle.
    let live = prepare_run(request(&data, &original.id)).unwrap();
    let live_id = live.run_id().to_owned();
    let before = run(&w, &live_id);
    assert_eq!(before.state, RunState::Working);

    // Before: the original chat was reused and `begin()` fenced the live Run.
    let (other, reused) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    supply(&data, &other.id).await;
    let after = run(&w, &live_id);
    assert_eq!(
        (after.state, after.fence),
        (RunState::Working, before.fence),
        "the live Run is not interrupted"
    );
    assert!(!reused);
    assert_ne!(other.id, original.id);

    // Once its process lets go, the orphan no longer blocks reuse, and the
    // next Run there interrupts it as Continue does.
    drop(live);
    w.archive_chat("archive-other", &other.id).unwrap();
    let (again, reused) = w.source_intake_chat(None, LOCATOR, TITLE).unwrap();
    assert_eq!((again.id.as_str(), reused), (original.id.as_str(), true));
    supply(&data, &original.id).await;
    let recovered = run(&w, &live_id);
    assert_eq!(
        (recovered.state, recovered.fence),
        (RunState::Interrupted, before.fence + 1)
    );
}
