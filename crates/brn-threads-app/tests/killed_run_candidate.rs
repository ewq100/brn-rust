//! The owner's `native-b1-kill` case: a useful-intake Run prepares a candidate
//! through the real runtime tools, then its process is killed before the
//! terminal result. However that Run ends afterwards, its chat gets one
//! Interrupted notice, a Review item for the candidate, and loses the leftover
//! lease file. A live Run the owner stops in its own chat gets no notice and
//! keeps its lease. The candidate is never applied, dismissed
//! or retired for the owner. Synthetic data and a scripted model; no provider
//! is contacted.
use brn_ai::threads::{
    INTERRUPTED_ATTENTION, ThreadRunRequest, TransientSource, prepare_run, run_prepared_scripted,
    run_prepared_scripted_then_abandon,
};
use brn_ai::{AiTerminal, Provider, ReasoningEffort, Selection};
use brn_threads_app::run_attention::{FAILED_PREFIX, REVIEW_ATTENTION};
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
    killed_run(data, chat, intake_request(data, chat)).await
}

/// As [`killed_intake`], for an exact `request` in `chat`.
async fn killed_run(data: &Path, chat: &str, request: ThreadRunRequest) -> (String, String) {
    let prepared = prepare_run(request).unwrap();
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
    let stopped = run(&workspace, &id);
    // Before (round 4): no attention, so nothing reached Needs you. Before
    // slice D: Cancelled, while its notice said "Run interrupted".
    assert_eq!((stopped.state, stopped.fence), (RunState::Interrupted, 2));
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
    // Before (round 4): only orphan recovery removed the leftover file.
    assert!(!lease_file(&workspace, &id).exists());
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
    // another process is stopped too; its fenced finish writes nothing.
    let stopped = run(&workspace, &id);
    assert_eq!((stopped.state, stopped.fence), (RunState::Interrupted, 2));
    assert_eq!(attention(&workspace, &w.intake), vec![interrupted(&id)]);
    // A live lease is never removed; its holder drops it.
    assert!(brn_ai::run_lease::run_is_live(&derived, &id));
    assert!(lease_file(&workspace, &id).exists());
    drop(live);
    assert!(!lease_file(&workspace, &id).exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn stop_after_a_failure_treats_a_lease_free_run_as_interrupted_work() {
    // `leftover`: a SIGKILL left the unlocked file. Otherwise the failed
    // worker dropped its lease, which removed the file.
    for leftover in [true, false] {
        let w = world();
        let (id, operation) = killed_intake(&w.data, &w.intake).await;
        let mut workspace = Workspace::open(&w.data).unwrap();
        if !leftover {
            std::fs::remove_file(lease_file(&workspace, &id)).unwrap();
        }
        // The desktop's `pause_failed_worker` call.
        let stop = |workspace: &mut Workspace| {
            workspace
                .stop_chat_run("worker-failure:1", &id, Some("partial text"))
                .unwrap()
        };
        let receipt = stop(&mut workspace);
        let stopped = run(&workspace, &id);
        assert_eq!(
            (stopped.state, stopped.fence),
            (RunState::Interrupted, 2),
            "{leftover}"
        );
        // Before: `[Review(op)]` and the lease file left.
        let expected = vec![review(&operation), interrupted(&id)];
        assert_eq!(attention(&workspace, &w.intake), expected, "{leftover}");
        assert!(!lease_file(&workspace, &id).exists(), "{leftover}");
        let partials = |workspace: &Workspace| {
            workspace
                .messages(&w.intake)
                .unwrap()
                .iter()
                .filter(|m| matches!(&m.data, RecordData::Message(m) if m.role == "assistant-partial" && m.text == "partial text"))
                .count()
        };
        assert_eq!(partials(&workspace), 1);
        assert_reviewable(&workspace, &operation);
        // The same key replays the same receipt and adds nothing.
        let receipts = workspace.history().unwrap().len();
        assert_eq!(stop(&mut workspace), receipt);
        assert_eq!(workspace.history().unwrap().len(), receipts);
        assert_eq!(attention(&workspace, &w.intake), expected);
        assert_eq!(partials(&workspace), 1);
    }
}

fn scripted_failure() -> MockCompletionModel {
    MockCompletionModel::from_stream_turns([vec![MockStreamEvent::error(
        "synthetic provider failure",
    )]])
}

/// The one terminal notice the chat keeps after `run` failed.
fn assert_failed_notice(workspace: &Workspace, chat: &str, run: &str, operation: &str) {
    let items = attention(workspace, chat);
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[0], review(operation));
    assert_eq!(items[1].kind, AttentionKind::Blocker);
    assert_eq!(items[1].record.as_deref(), Some(run));
    assert!(items[1].reason.starts_with(FAILED_PREFIX), "{items:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn resume_after_a_killed_queued_run_leaves_one_notice_and_redispatches_once() {
    let w = world();
    let mut workspace = Workspace::open(&w.data).unwrap();
    workspace
        .queue_message("queue-1", &w.intake, QUESTION)
        .unwrap();
    let queued = workspace.dispatch_queued("dispatch-1").unwrap().unwrap();
    drop(workspace);
    // As the desktop's `start_run` builds a queued item's request.
    let queued_request = |q: &QueuedMessage| ThreadRunRequest {
        request_message: q.message.clone(),
        queue_claim: q.dispatch.clone(),
        ..request(&w.data, &q.thread, &q.text, vec![])
    };
    let (id, operation) = killed_run(&w.data, &w.intake, queued_request(&queued)).await;
    let mut workspace = Workspace::open(&w.data).unwrap();
    let pending = |workspace: &Workspace| match workspace.chat_control().unwrap().data {
        RecordData::ChatControl(c) => c.pending,
        _ => panic!("ChatControl"),
    };
    assert_eq!(
        pending(&workspace).and_then(|q| q.run).as_deref(),
        Some(id.as_str())
    );
    // The desktop's Resume: stop the pending item's stale Run, resume, dispatch.
    let receipt = workspace
        .stop_chat_run("resume-stale:1", &id, None)
        .unwrap();
    let stopped = run(&workspace, &id);
    assert_eq!((stopped.state, stopped.fence), (RunState::Interrupted, 2));
    // Before: `[Review(op)]` and the lease file left.
    let expected = vec![review(&operation), interrupted(&id)];
    assert_eq!(attention(&workspace, &w.intake), expected);
    assert!(!lease_file(&workspace, &id).exists());
    workspace.resume_queue("resume-1").unwrap();
    let again = workspace.dispatch_queued("dispatch-2").unwrap().unwrap();
    assert_eq!(attention(&workspace, &w.intake), expected);
    assert_reviewable(&workspace, &operation);
    // The same item and the same owner Message: nothing is reposted.
    assert_eq!(
        (&again.id, &again.message, &again.text),
        (&queued.id, &queued.message, &queued.text)
    );
    assert_eq!(owner_messages(&workspace, &w.intake), 1);
    assert!(!matches!(
        workspace.dispatch_queued("dispatch-3"),
        Ok(Some(_))
    ));
    // A replayed stop adds nothing.
    let receipts = workspace.history().unwrap().len();
    assert_eq!(
        workspace
            .stop_chat_run("resume-stale:1", &id, None)
            .unwrap(),
        receipt
    );
    assert_eq!(workspace.history().unwrap().len(), receipts);
    // The item runs once more and fails: its result replaces the notice.
    let prepared = prepare_run(queued_request(&again)).unwrap();
    let rerun = prepared.run_id().to_owned();
    let result = run_prepared_scripted(prepared, scripted_failure())
        .await
        .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Failed(_)));
    assert_failed_notice(&workspace, &w.intake, &rerun, &operation);
    assert_eq!(owner_messages(&workspace, &w.intake), 1);
    assert_reviewable(&workspace, &operation);
}

#[test]
fn a_deliberate_stop_of_a_live_run_adds_no_notice_and_keeps_its_lease() {
    let w = world();
    let live = prepare_run(intake_request(&w.data, &w.intake)).unwrap();
    let id = live.run_id().to_owned();
    let mut workspace = Workspace::open(&w.data).unwrap();
    let derived = workspace.store.derived_directory().unwrap();
    workspace.stop_chat_run("stop-1", &id, None).unwrap();
    let stopped = run(&workspace, &id);
    assert_eq!((stopped.state, stopped.fence), (RunState::Interrupted, 2));
    assert!(attention(&workspace, &w.intake).is_empty());
    assert!(brn_ai::run_lease::run_is_live(&derived, &id));
    assert!(lease_file(&workspace, &id).exists());
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
async fn continue_after_the_settings_stop_clears_its_notice_when_the_run_starts() {
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
    // Owner decision 1: gone as the Run starts, not only at its result.
    assert_eq!(attention(&workspace, &w.intake), vec![review(&operation)]);
    let result = run_prepared_scripted(
        prepared,
        MockCompletionModel::from_stream_turns([finish("The crane note waits for your review.")]),
    )
    .await
    .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    // Before (round 4): the stopped Run's notice stayed beside the success.
    assert_eq!(attention(&workspace, &w.intake), vec![review(&operation)]);
    assert_reviewable(&workspace, &operation);
}

/// The owner's stop, archive or steer of Run `id` in the intake chat.
fn stop_path(path: &str, workspace: &mut Workspace, w: &World, id: &str) -> Receipt {
    match path {
        "stop" => workspace.stop_chat_run("stop-1", id, None).unwrap(),
        "archive" => workspace.archive_chat("archive-1", &w.intake).unwrap(),
        _ => workspace
            .stop_and_post("steer-1", id, &w.intake, "Use the berth survey instead.")
            .unwrap(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn stop_archive_and_steer_of_a_killed_run_review_its_candidate_and_remove_the_lease() {
    for path in ["stop", "archive", "steer"] {
        let w = world();
        let (id, operation) = killed_intake(&w.data, &w.intake).await;
        let mut workspace = Workspace::open(&w.data).unwrap();
        let receipt = stop_path(path, &mut workspace, &w, &id);
        let stopped = run(&workspace, &id);
        assert_eq!(
            (stopped.state, stopped.fence),
            (RunState::Interrupted, 2),
            "{path}"
        );
        // No process held this Run: Review item and lease removal, as orphan
        // recovery gives them. Archive is a deliberate owner stop, so it adds
        // no notice (owner decision 2; round 5 C added one). Stop and Steer
        // add the notice. The archived chat's own write carries the Review.
        let expected = if path == "archive" {
            vec![review(&operation)]
        } else {
            vec![review(&operation), interrupted(&id)]
        };
        assert_eq!(attention(&workspace, &w.intake), expected, "{path}");
        assert!(!lease_file(&workspace, &id).exists(), "{path}");
        assert_eq!(
            workspace.store.record(&w.intake).unwrap().unwrap().archived,
            path == "archive"
        );
        assert_reviewable(&workspace, &operation);
        // The same key replays the same receipt and changes nothing.
        let receipts = workspace.history().unwrap().len();
        assert_eq!(stop_path(path, &mut workspace, &w, &id), receipt, "{path}");
        assert_eq!(workspace.history().unwrap().len(), receipts);
        assert_eq!(attention(&workspace, &w.intake), expected);
    }
}

#[test]
fn stop_archive_and_steer_of_a_live_run_add_no_notice() {
    for path in ["stop", "archive", "steer"] {
        let w = world();
        let live = prepare_run(intake_request(&w.data, &w.intake)).unwrap();
        let id = live.run_id().to_owned();
        let mut workspace = Workspace::open(&w.data).unwrap();
        let derived = workspace.store.derived_directory().unwrap();
        stop_path(path, &mut workspace, &w, &id);
        let stopped = run(&workspace, &id);
        assert_eq!(
            (stopped.state, stopped.fence),
            (RunState::Interrupted, 2),
            "{path}"
        );
        // The owner stopped this chat's live work and sees that: no notice.
        assert!(attention(&workspace, &w.intake).is_empty(), "{path}");
        assert!(brn_ai::run_lease::run_is_live(&derived, &id), "{path}");
        assert!(lease_file(&workspace, &id).exists(), "{path}");
        drop(live);
    }
}

/// Owner decision 2 (16 October 2026): archiving preserves outstanding
/// attention and adds no new notice, even for a lease-free orphan. The
/// Review item for the Run's unapplied proposal is still added, and its free
/// lease file is still removed.
#[tokio::test(flavor = "multi_thread")]
async fn archive_keeps_existing_attention_and_adds_no_notice_for_a_killed_run() {
    // `recovered`: the owner's Resume already recorded the orphan's notice.
    for recovered in [false, true] {
        for kind in ["archive", "archive-progress"] {
            let w = world();
            let (id, operation) = killed_intake(&w.data, &w.intake).await;
            let mut workspace = Workspace::open(&w.data).unwrap();
            if recovered {
                workspace.recover_queue("startup").unwrap();
                workspace.resume_queue("resume-1").unwrap();
            }
            let archive = |workspace: &mut Workspace| match kind {
                "archive" => workspace.archive_chat("archive-1", &w.intake).unwrap(),
                _ => workspace
                    .archive_chat_progress("archive-1", &w.intake, &id, "Partial crane notes")
                    .unwrap(),
            };
            let receipt = archive(&mut workspace);
            let stopped = run(&workspace, &id);
            // Interrupted, as the owner's Stop records it (review-D).
            assert_eq!(
                (stopped.state, stopped.fence),
                (RunState::Interrupted, 2),
                "{kind} {recovered}"
            );
            // Before (round 5 C): `[Review(op), Blocker(id)]` for the orphan.
            // An existing notice stays; Archive adds none.
            let expected = if recovered {
                vec![review(&operation), interrupted(&id)]
            } else {
                vec![review(&operation)]
            };
            assert_eq!(
                attention(&workspace, &w.intake),
                expected,
                "{kind} {recovered}"
            );
            assert!(workspace.store.record(&w.intake).unwrap().unwrap().archived);
            assert!(!lease_file(&workspace, &id).exists(), "{kind} {recovered}");
            assert_reviewable(&workspace, &operation);
            // A replay returns the recorded receipt and adds nothing.
            let receipts = workspace.history().unwrap().len();
            assert_eq!(archive(&mut workspace), receipt);
            assert_eq!(workspace.history().unwrap().len(), receipts);
            assert_eq!(attention(&workspace, &w.intake), expected);
        }
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
                assert!(!lease_file(&workspace, &id).exists());
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
        // Before: the leftover lease file stayed.
        assert!(!lease_file(&workspace, &id).exists(), "{path}");
    }
}

/// The applied "Start a fresh invocation" change that saved Run `run`.
fn start_receipt(workspace: &Workspace, run: &str) -> Receipt {
    workspace
        .history()
        .unwrap()
        .into_iter()
        .find(|r| {
            r.reason == "Start a fresh invocation"
                && r.writes
                    .iter()
                    .any(|w| w.after.id == run && w.before.is_none())
        })
        .expect("the Run's start")
}

/// Replays the change that started `run`: the recorded receipt, nothing new.
fn assert_start_replays(workspace: &mut Workspace, chat: &str, run: &str) {
    let receipt = start_receipt(workspace, run);
    let (receipts, before) = (
        workspace.history().unwrap().len(),
        attention(workspace, chat),
    );
    assert_eq!(
        workspace
            .store
            .apply(
                &receipt.operation,
                &HostAuthority::owner("owner started or continued a Thread")
            )
            .unwrap(),
        ApplyOutcome::Applied(receipt)
    );
    assert_eq!(workspace.history().unwrap().len(), receipts);
    assert_eq!(attention(workspace, chat), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn continue_in_the_killed_chat_starts_without_a_notice_and_removes_the_lease() {
    let w = world();
    let (id, operation) = killed_intake(&w.data, &w.intake).await;
    let prepared = prepare_run(request(&w.data, &w.intake, "Continue", vec![])).unwrap();
    let continued = prepared.run_id().to_owned();
    let mut workspace = Workspace::open(&w.data).unwrap();
    let fenced = run(&workspace, &id);
    assert_eq!((fenced.state, fenced.fence), (RunState::Interrupted, 2));
    // Owner decision 1: starting the next Run clears the chat's notice, so
    // the fenced orphan gets none. Before (round 5 C): `[Review(op),
    // Blocker(id)]` until the new Run's result. The Review stays.
    assert_eq!(attention(&workspace, &w.intake), vec![review(&operation)]);
    assert!(!lease_file(&workspace, &id).exists());
    assert_start_replays(&mut workspace, &w.intake, &continued);
    // The new Run holds its own lease.
    let derived = workspace.store.derived_directory().unwrap();
    assert!(brn_ai::run_lease::run_is_live(&derived, &continued));
    let result = run_prepared_scripted(prepared, scripted_failure())
        .await
        .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Failed(_)));
    // Its failure records exactly one new notice.
    assert_failed_notice(&workspace, &w.intake, &continued, &operation);
    assert_reviewable(&workspace, &operation);
}

/// Owner decision 1 (16 October 2026): the notice orphan recovery recorded
/// clears when the chat's next Run starts. Reviews and Questions stay, and a
/// failing next Run records exactly one new notice.
#[tokio::test(flavor = "multi_thread")]
async fn continue_after_a_crash_clears_the_notice_when_the_next_run_starts() {
    for outcome in ["failed", "completed"] {
        let w = world();
        let (id, operation) = killed_intake(&w.data, &w.intake).await;
        let mut workspace = Workspace::open(&w.data).unwrap();
        // Restart, then the owner's Resume recovers the orphan.
        workspace.recover_queue("startup").unwrap();
        workspace.resume_queue("resume-1").unwrap();
        assert_eq!(
            attention(&workspace, &w.intake),
            vec![review(&operation), interrupted(&id)],
            "{outcome}"
        );
        // An owner Question in the same chat.
        let question = Attention {
            kind: AttentionKind::Question,
            reason: QUESTION.into(),
            record: None,
        };
        let chat = workspace.store.record(&w.intake).unwrap().unwrap();
        let RecordData::Thread(mut thread) = chat.data.clone() else {
            panic!("Thread")
        };
        thread.attention.push(question.clone());
        assert!(matches!(
            workspace
                .owner_change(
                    "ask-owner",
                    &ChangeRequest {
                        reason: "Ask the owner".into(),
                        writes: vec![Put {
                            id: chat.id.clone(),
                            expected_version: Some(chat.version),
                            archived: false,
                            data: RecordData::Thread(thread),
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap(),
            ApplyOutcome::Applied(_)
        ));
        drop(workspace);
        // Continue in the chat.
        let prepared = prepare_run(request(&w.data, &w.intake, "Continue", vec![])).unwrap();
        let continued = prepared.run_id().to_owned();
        let mut workspace = Workspace::open(&w.data).unwrap();
        // Before: the notice stayed until the new Run's result.
        let kept = vec![review(&operation), question.clone()];
        assert_eq!(attention(&workspace, &w.intake), kept, "{outcome}");
        assert!(
            workspace
                .needs_you()
                .unwrap()
                .iter()
                .any(|chat| chat.id == w.intake),
            "the Review and the Question still need the owner"
        );
        assert_start_replays(&mut workspace, &w.intake, &continued);
        let model = if outcome == "failed" {
            scripted_failure()
        } else {
            MockCompletionModel::from_stream_turns([finish("The crane note waits for review.")])
        };
        let result = run_prepared_scripted(prepared, model).await.unwrap();
        let items = attention(&workspace, &w.intake);
        if outcome == "failed" {
            assert!(matches!(result.answer.terminal, AiTerminal::Failed(_)));
            // Exactly one new notice, for the new Run.
            let notices: Vec<_> = items
                .iter()
                .filter(|a| a.kind == AttentionKind::Blocker)
                .collect();
            assert_eq!(notices.len(), 1, "{items:?}");
            assert_eq!(notices[0].record.as_deref(), Some(continued.as_str()));
            assert!(notices[0].reason.starts_with(FAILED_PREFIX), "{items:?}");
            assert_eq!(items[..2], kept[..]);
        } else {
            assert!(matches!(result.answer.terminal, AiTerminal::Completed));
            assert_eq!(items, kept);
        }
        assert_reviewable(&workspace, &operation);
    }
}

#[test]
fn continue_beside_a_live_run_adds_no_notice_and_keeps_its_lease() {
    let w = world();
    let live = prepare_run(intake_request(&w.data, &w.intake)).unwrap();
    let id = live.run_id().to_owned();
    let prepared = prepare_run(request(&w.data, &w.intake, "Continue", vec![])).unwrap();
    let workspace = Workspace::open(&w.data).unwrap();
    let fenced = run(&workspace, &id);
    assert_eq!((fenced.state, fenced.fence), (RunState::Interrupted, 2));
    assert!(attention(&workspace, &w.intake).is_empty());
    let derived = workspace.store.derived_directory().unwrap();
    assert!(brn_ai::run_lease::run_is_live(&derived, &id));
    assert!(lease_file(&workspace, &id).exists());
    drop(prepared);
    drop(live);
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
