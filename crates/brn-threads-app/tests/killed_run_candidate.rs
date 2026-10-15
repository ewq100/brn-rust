//! The owner's `native-b1-kill` case: a useful-intake Run prepares a candidate
//! through the real runtime tools, then its process is killed before the
//! terminal result. However that Run ends afterwards, its chat gets one
//! Interrupted notice where the stop came from elsewhere, and a Review item
//! for the candidate in every case. The candidate is never applied, dismissed
//! or retired for the owner. Synthetic data and a scripted model; no provider
//! is contacted.
use brn_ai::threads::{
    INTERRUPTED_ATTENTION, ThreadRunRequest, TransientSource, prepare_run, run_prepared_scripted,
    run_prepared_scripted_then_abandon,
};
use brn_ai::{AiTerminal, Provider, ReasoningEffort, Selection};
use brn_threads_app::run_attention::REVIEW_ATTENTION;
use brn_threads_app::*;
use brn_threads_intake::{ConversionResult, Coverage, CoverageStatus, ImportIntent};
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const INTAKE_TITLE: &str = "Information from b1-source";
const KEY: &str = "crane-arrival-source-capture-1";
const QUESTION: &str = "When can the crane arrive?";

struct World {
    _root: tempfile::TempDir,
    data: PathBuf,
    project: String,
    /// The project chat the killed intake ran in.
    intake: String,
    /// A general chat the owner queues a question in.
    questions: String,
}

/// A workspace with offline settings: no sign-in file and no credentials, so
/// any provider run fails before network access. Delegation stays pending.
fn workspace_at(data: &Path) -> Workspace {
    let mut workspace = Workspace::open(data).unwrap();
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
    assert!(!workspace.settings().unwrap().delegation_chosen);
    workspace
}

fn world() -> World {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut workspace = workspace_at(&data);
    let project = workspace
        .project_command(
            "killed-project",
            ProjectCommand::Create {
                title: "Harbor crane".into(),
            },
        )
        .unwrap()
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Project(_)))
        .unwrap()
        .after
        .id;
    let intake = workspace
        .project_command(
            "killed-chat",
            ProjectCommand::CreateChat {
                project: project.clone(),
                title: INTAKE_TITLE.into(),
            },
        )
        .unwrap()
        .writes
        .into_iter()
        .find(|w| w.before.is_none() && matches!(w.after.data, RecordData::Thread(_)))
        .unwrap()
        .after
        .id;
    let questions = workspace.new_thread("Questions").unwrap().id;
    World {
        _root: root,
        data,
        project,
        intake,
        questions,
    }
}

fn request(
    data: &Path,
    chat: &str,
    prompt: &str,
    sources: Vec<TransientSource>,
) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: data.to_owned(),
        thread: chat.to_owned(),
        prompt: prompt.into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: 12,
        auth_file: None,
        credentials_dir: None,
        sources,
        search: None,
        trace: brn_ai::threads::TraceLevel::Off,
    }
}

fn intake_request(data: &Path, chat: &str) -> ThreadRunRequest {
    request(
        data,
        chat,
        "Capture useful information.",
        vec![TransientSource::text(
            "/private/tmp/brn-dwr-data/b1-source.md",
            "# Crane note\n\nSynthetic: the crane arrives on 22 November.\n",
        )],
    )
}

fn call(id: &str, name: &str, args: serde_json::Value) -> Vec<MockStreamEvent> {
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

fn lease_file(workspace: &Workspace, run: &str) -> PathBuf {
    workspace
        .store
        .derived_directory()
        .unwrap()
        .join("run-leases")
        .join(format!("{}.lock", run.replace(':', "_")))
}

fn run(workspace: &Workspace, id: &str) -> Run {
    match workspace.store.record(id).unwrap().unwrap().data {
        RecordData::Run(run) => run,
        _ => panic!("Run"),
    }
}

fn attention(workspace: &Workspace, chat: &str) -> Vec<Attention> {
    match workspace.store.record(chat).unwrap().unwrap().data {
        RecordData::Thread(thread) => thread.attention,
        _ => panic!("Thread"),
    }
}

fn review(operation: &str) -> Attention {
    Attention {
        kind: AttentionKind::Review,
        reason: REVIEW_ATTENTION.into(),
        record: Some(operation.into()),
    }
}

fn interrupted(run: &str) -> Attention {
    Attention {
        kind: AttentionKind::Blocker,
        reason: INTERRUPTED_ATTENTION.into(),
        record: Some(run.into()),
    }
}

fn owner_messages(workspace: &Workspace, chat: &str) -> usize {
    workspace
        .messages(chat)
        .unwrap()
        .iter()
        .filter(|m| matches!(&m.data, RecordData::Message(m) if m.role == "owner"))
        .count()
}

/// Still listed, unretired and without a receipt: the owner can review it.
fn assert_reviewable(workspace: &Workspace, operation: &str) {
    assert!(
        workspace
            .candidates()
            .unwrap()
            .iter()
            .any(|c| c.operation.as_str() == operation),
        "candidate {operation} is still active"
    );
    let op = workspace.store.operation(operation).unwrap();
    assert!(workspace.store.receipt(&op).unwrap().is_none());
}

/// A useful intake in `chat` prepares one new note through the real
/// `prepare_changes` tool, then its process dies before the terminal result.
/// Returns the Run and its candidate operation.
async fn killed_intake(data: &Path, chat: &str) -> (String, String) {
    let prepared = prepare_run(intake_request(data, chat)).unwrap();
    let id = prepared.run_id().to_owned();
    let model = MockCompletionModel::from_stream_turns([
        call(
            "p",
            "prepare_changes",
            json!({"request_key":KEY,"request":{"reason":"Record the reported crane arrival date","writes":[
                {"id":"new:0","expected_version":null,"archived":false,"data":{"Note":{"title":"Crane arrival — reported date","markdown":"# Crane arrival — reported date\n\nThe source reports that the crane arrives on 22 November.\n","protected":false,"confirmed":false,"superseded_by":null,"import":null}}}
            ],"inputs":[]}}),
        ),
        finish("I prepared one note about the crane arrival."),
    ]);
    assert_eq!(
        run_prepared_scripted_then_abandon(prepared, model)
            .await
            .unwrap(),
        id
    );
    let workspace = Workspace::open(data).unwrap();
    let operation = Store::open(data)
        .unwrap()
        .allocate_operation(&format!("{id}:proposal:{KEY}"))
        .unwrap()
        .as_str()
        .to_owned();
    // Dropping the scope removed the file; a SIGKILL leaves it, unlocked.
    let lease = lease_file(&workspace, &id);
    assert!(!lease.exists());
    std::fs::write(&lease, b"").unwrap();
    assert_killed(&workspace, chat, &id, &operation);
    (id, operation)
}

/// The store the owner found after `pkill -9`.
fn assert_killed(workspace: &Workspace, chat: &str, id: &str, operation: &str) {
    let killed = run(workspace, id);
    assert_eq!((killed.state, killed.fence), (RunState::Working, 1));
    assert_eq!(
        killed
            .operations
            .iter()
            .map(OperationId::as_str)
            .collect::<Vec<_>>(),
        [operation]
    );
    assert!(attention(workspace, chat).is_empty());
    assert_reviewable(workspace, operation);
    assert!(lease_file(workspace, id).exists());
    let derived = workspace.store.derived_directory().unwrap();
    assert!(!brn_ai::run_lease::run_is_live(&derived, id));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_delegation_choice_after_a_kill_leaves_one_notice_and_a_review_for_the_candidate() {
    let w = world();
    let (id, operation) = killed_intake(&w.data, &w.intake).await;
    let mut workspace = Workspace::open(&w.data).unwrap();
    // The desktop's Home choice calls this same service method.
    let ApplyOutcome::Applied(receipt) = workspace
        .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap()
    else {
        panic!("applied")
    };
    let cancelled = run(&workspace, &id);
    // Before: Cancelled, with no attention, so nothing reached Needs you.
    assert_eq!((cancelled.state, cancelled.fence), (RunState::Cancelled, 2));
    let expected = vec![review(&operation), interrupted(&id)];
    assert_eq!(attention(&workspace, &w.intake), expected);
    assert_reviewable(&workspace, &operation);
    assert!(
        workspace
            .needs_you()
            .unwrap()
            .iter()
            .any(|chat| chat.id == w.intake)
    );
    // Choosing again, or replaying the same operation, adds nothing.
    workspace
        .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap();
    let receipts = workspace.history().unwrap().len();
    assert_eq!(
        workspace
            .store
            .choose_delegation(
                &receipt.operation,
                DelegationChoice::MaintainOrdinaryNotes,
                &HostAuthority::owner("owner delegation choice retry"),
            )
            .unwrap(),
        ApplyOutcome::Applied(receipt)
    );
    assert_eq!(workspace.history().unwrap().len(), receipts);
    assert_eq!(attention(&workspace, &w.intake), expected);
    assert_reviewable(&workspace, &operation);
    // Recorded limitation: only orphan recovery removes the leftover file.
    assert!(lease_file(&workspace, &id).exists());
}

#[test]
fn the_settings_change_also_stops_a_run_that_is_still_live() {
    let w = world();
    let live = prepare_run(intake_request(&w.data, &w.intake)).unwrap();
    let id = live.run_id().to_owned();
    let mut workspace = Workspace::open(&w.data).unwrap();
    let derived = workspace.store.derived_directory().unwrap();
    assert!(brn_ai::run_lease::run_is_live(&derived, &id));
    workspace
        .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap();
    // Recorded, not changed: settings never consult leases, so a Run live in
    // another process is cancelled too; its fenced finish writes nothing.
    let stopped = run(&workspace, &id);
    assert_eq!((stopped.state, stopped.fence), (RunState::Cancelled, 2));
    assert_eq!(attention(&workspace, &w.intake), vec![interrupted(&id)]);
    assert!(brn_ai::run_lease::run_is_live(&derived, &id));
    drop(live);
}

#[tokio::test(flavor = "multi_thread")]
async fn resume_or_dispatch_after_a_kill_adds_the_review_and_removes_the_free_lease_file() {
    for path in ["resume", "dispatch"] {
        let w = world();
        let (id, operation) = killed_intake(&w.data, &w.intake).await;
        let mut workspace = Workspace::open(&w.data).unwrap();
        workspace
            .queue_message("queue-1", &w.questions, QUESTION)
            .unwrap();
        if path == "resume" {
            // Restart with a saved queue: paused until the owner resumes.
            workspace.recover_queue("startup").unwrap();
            workspace.resume_queue("resume-1").unwrap();
        }
        let dispatched = workspace.dispatch_queued("dispatch-1").unwrap();
        assert_eq!(dispatched.map(|q| q.text).as_deref(), Some(QUESTION));
        let recovered = run(&workspace, &id);
        assert_eq!(
            (recovered.state, recovered.fence),
            (RunState::Interrupted, 2),
            "{path}"
        );
        // Before: only the notice; the candidate had no Review item.
        assert_eq!(
            attention(&workspace, &w.intake),
            vec![review(&operation), interrupted(&id)],
            "{path}"
        );
        assert_reviewable(&workspace, &operation);
        assert!(!lease_file(&workspace, &id).exists(), "{path}");
        let receipts = workspace.history().unwrap().len();
        assert!(workspace.recover_orphaned_runs().unwrap().is_empty());
        assert_eq!(workspace.history().unwrap().len(), receipts);
        assert!(!matches!(
            workspace.dispatch_queued("dispatch-2"),
            Ok(Some(_))
        ));
        assert_eq!(owner_messages(&workspace, &w.questions), 1);
        // Exact-candidate review still works and leaves the notice.
        assert!(matches!(
            workspace.review_apply(&operation).unwrap(),
            ApplyOutcome::Applied(_)
        ));
        assert_eq!(attention(&workspace, &w.intake), vec![interrupted(&id)]);
        assert!(
            !workspace
                .candidates()
                .unwrap()
                .iter()
                .any(|c| c.operation.as_str() == operation)
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_later_run_replaces_the_notice_of_a_run_the_settings_change_cancelled() {
    let w = world();
    let (id, operation) = killed_intake(&w.data, &w.intake).await;
    let mut workspace = Workspace::open(&w.data).unwrap();
    workspace
        .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap();
    assert_eq!(
        attention(&workspace, &w.intake),
        vec![review(&operation), interrupted(&id)]
    );
    let prepared = prepare_run(request(&w.data, &w.intake, "Continue", vec![])).unwrap();
    let result = run_prepared_scripted(
        prepared,
        MockCompletionModel::from_stream_turns([finish("The crane note waits for your review.")]),
    )
    .await
    .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    // Before: the Cancelled Run's notice stayed beside the success.
    assert_eq!(attention(&workspace, &w.intake), vec![review(&operation)]);
    assert_reviewable(&workspace, &operation);
}

#[tokio::test(flavor = "multi_thread")]
async fn stop_archive_and_steer_keep_the_candidate_in_review_without_a_notice() {
    for path in ["stop", "archive", "steer"] {
        let w = world();
        let (id, operation) = killed_intake(&w.data, &w.intake).await;
        let mut workspace = Workspace::open(&w.data).unwrap();
        let act = |workspace: &mut Workspace| match path {
            "stop" => workspace.stop_chat_run("stop-1", &id, None).unwrap(),
            "archive" => workspace.archive_chat("archive-1", &w.intake).unwrap(),
            _ => workspace
                .stop_and_post("steer-1", &id, &w.intake, "Use the berth survey instead.")
                .unwrap(),
        };
        let receipt = act(&mut workspace);
        let stopped = run(&workspace, &id);
        assert_eq!(
            (stopped.state, stopped.fence),
            (RunState::Interrupted, 2),
            "{path}"
        );
        // The owner stopped this chat's work and sees that: no notice.
        assert_eq!(
            attention(&workspace, &w.intake),
            vec![review(&operation)],
            "{path}"
        );
        assert_eq!(
            workspace.store.record(&w.intake).unwrap().unwrap().archived,
            path == "archive"
        );
        assert_reviewable(&workspace, &operation);
        // The same key replays the same receipt and changes nothing.
        let receipts = workspace.history().unwrap().len();
        assert_eq!(act(&mut workspace), receipt, "{path}");
        assert_eq!(workspace.history().unwrap().len(), receipts);
        assert_eq!(attention(&workspace, &w.intake), vec![review(&operation)]);
    }
}

fn full_note() -> ConversionResult {
    ConversionResult {
        intent: ImportIntent::FullNote,
        markdown: "# Crane schedule\n\nSynthetic: delivery window 20–24 November.\n".into(),
        assets: vec![],
        source_sha256: [9; 32],
        source_reference: "synthetic://crane-schedule".into(),
        converter: "synthetic".into(),
        sources: vec![],
        coverage: Coverage {
            status: CoverageStatus::Complete,
            checked_items: vec![],
            gaps: vec![],
        },
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn project_changes_and_project_imports_leave_the_notice_and_the_review() {
    for path in ["configure", "import", "delete"] {
        let w = world();
        let (id, operation) = killed_intake(&w.data, &w.intake).await;
        let mut workspace = Workspace::open(&w.data).unwrap();
        let expected = vec![review(&operation), interrupted(&id)];
        let mut fence = 2;
        match path {
            "configure" => {
                let version = workspace.store.record(&w.project).unwrap().unwrap().version;
                workspace
                    .project_command(
                        "configure-1",
                        ProjectCommand::Configure {
                            project: w.project.clone(),
                            version,
                            title: "Harbor crane".into(),
                            instructions: "Keep delivery dates exact.".into(),
                            use_brn_knowledge: true,
                        },
                    )
                    .unwrap();
            }
            "import" => {
                let outcome = workspace
                    .import_full_scoped(
                        "import-1",
                        "Crane schedule",
                        &full_note(),
                        Some(&w.project),
                    )
                    .unwrap();
                assert!(matches!(outcome, ApplyOutcome::Applied(_)), "{outcome:?}");
            }
            _ => {
                let preview = workspace.project_delete_preview(&w.project).unwrap();
                assert!(preview.unfinished_actions.is_empty());
                workspace
                    .project_command(
                        "delete-1",
                        ProjectCommand::Delete {
                            preview,
                            actions: BTreeMap::new(),
                        },
                    )
                    .unwrap();
                // The trashed chat keeps both, so Restore brings them back.
                assert!(workspace.store.record(&w.intake).unwrap().unwrap().archived);
                assert_eq!(attention(&workspace, &w.intake), expected);
                let version = workspace.store.record(&w.project).unwrap().unwrap().version;
                workspace
                    .project_command(
                        "restore-1",
                        ProjectCommand::Restore {
                            project: w.project.clone(),
                            version,
                        },
                    )
                    .unwrap();
                assert!(!workspace.store.record(&w.intake).unwrap().unwrap().archived);
                // Restore fences the Run again as it leaves Trash.
                fence = 3;
            }
        }
        let stopped = run(&workspace, &id);
        assert_eq!(
            (stopped.state, stopped.fence),
            (RunState::Interrupted, fence),
            "{path}"
        );
        assert_eq!(attention(&workspace, &w.intake), expected, "{path}");
        assert_reviewable(&workspace, &operation);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn continue_in_the_killed_chat_keeps_the_candidate_in_review() {
    let w = world();
    let (id, operation) = killed_intake(&w.data, &w.intake).await;
    let prepared = prepare_run(request(&w.data, &w.intake, "Continue", vec![])).unwrap();
    let workspace = Workspace::open(&w.data).unwrap();
    let fenced = run(&workspace, &id);
    assert_eq!((fenced.state, fenced.fence), (RunState::Interrupted, 2));
    // A fresh Run follows at once, so no notice; before: no Review either.
    assert_eq!(attention(&workspace, &w.intake), vec![review(&operation)]);
    let result = run_prepared_scripted(
        prepared,
        MockCompletionModel::from_stream_turns([finish("The crane note waits for your review.")]),
    )
    .await
    .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    assert_eq!(attention(&workspace, &w.intake), vec![review(&operation)]);
    assert_reviewable(&workspace, &operation);
}

/// Writes a fresh store for the owner's offline native re-check. `settings`:
/// delegation pending, so Home offers the two choices. `resume`: delegation
/// chosen first, then a queued question paused as after a restart, so the
/// Resume button shows and is enabled.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "writes a store for the owner's native check; set BRN_DWR_NATIVE_DATA and BRN_DWR_NATIVE_MODE"]
async fn native_check_store_with_a_killed_intake_and_its_candidate() {
    let data = PathBuf::from(std::env::var("BRN_DWR_NATIVE_DATA").expect("BRN_DWR_NATIVE_DATA"));
    let mode = std::env::var("BRN_DWR_NATIVE_MODE").expect("BRN_DWR_NATIVE_MODE");
    assert!(
        data.is_absolute() && data.starts_with("/private/tmp/brn-dwr-data/") && !data.exists(),
        "BRN_DWR_NATIVE_DATA must be a new folder under /private/tmp/brn-dwr-data/"
    );
    assert!(matches!(mode.as_str(), "settings" | "resume"), "{mode}");
    let mut workspace = workspace_at(&data);
    if mode == "resume" {
        workspace
            .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
            .unwrap();
    }
    let intake = workspace.new_thread(INTAKE_TITLE).unwrap().id;
    drop(workspace);
    let (id, operation) = killed_intake(&data, &intake).await;
    let mut workspace = Workspace::open(&data).unwrap();
    if mode == "resume" {
        let questions = workspace.new_thread("Questions").unwrap().id;
        workspace
            .queue_message("native-question", &questions, QUESTION)
            .unwrap();
        workspace.recover_queue("native-restart").unwrap();
        println!("questions chat: {questions}");
    }
    assert_killed(&workspace, &intake, &id, &operation);
    println!("intake chat: {intake}\nrun: {id}\noperation: {operation}");
}
