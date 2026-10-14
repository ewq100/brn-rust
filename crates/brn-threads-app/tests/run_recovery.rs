//! A Working Run whose process ended (crash, kill, quit) must not block every
//! queue. Resume and Dispatch interrupt it once its lease is free; a live Run
//! keeps the existing refusals. Synthetic data only; no provider is contacted.
use brn_ai::threads::{INTERRUPTED_ATTENTION, ThreadRunRequest, TransientSource, prepare_run};
use brn_ai::{Provider, ReasoningEffort, Selection};
use brn_threads_app::*;
use std::path::{Path, PathBuf};

struct World {
    _root: tempfile::TempDir,
    data: PathBuf,
    /// A project chat that receives useful intake.
    intake: String,
    /// A general chat the owner queues a question in.
    questions: String,
}

fn created(receipt: &Receipt) -> String {
    receipt
        .writes
        .iter()
        .find(|w| w.before.is_none() && matches!(w.after.data, RecordData::Thread(_)))
        .map(|w| w.after.id.clone())
        .expect("Thread")
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
    let receipt = workspace
        .project_command(
            "recovery-project",
            ProjectCommand::Create {
                title: "Harbor crane".into(),
            },
        )
        .unwrap();
    let project = receipt
        .writes
        .iter()
        .find(|w| matches!(w.after.data, RecordData::Project(_)))
        .unwrap()
        .after
        .id
        .clone();
    let intake = created(
        &workspace
            .project_command(
                "recovery-chat",
                ProjectCommand::CreateChat {
                    project,
                    title: "Information from mari-email".into(),
                },
            )
            .unwrap(),
    );
    let questions = workspace.new_thread("Questions").unwrap().id;
    World {
        _root: root,
        data,
        intake,
        questions,
    }
}

fn intake_request(data: &Path, chat: &str) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: data.to_owned(),
        thread: chat.to_owned(),
        prompt: "Capture useful information.".into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: 12,
        auth_file: None,
        credentials_dir: None,
        sources: vec![TransientSource::text(
            "synthetic://mari-crane-email",
            "Can we move crane delivery to 15 November?",
        )],
        search: None,
        trace: brn_ai::threads::TraceLevel::Off,
    }
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

fn owner_messages(workspace: &Workspace, chat: &str) -> usize {
    workspace
        .messages(chat)
        .unwrap()
        .iter()
        .filter(|m| matches!(&m.data, RecordData::Message(m) if m.role == "owner"))
        .count()
}

/// The Run is saved Working, then its process "dies" before any terminal result.
fn orphan(w: &World) -> String {
    let prepared = prepare_run(intake_request(&w.data, &w.intake)).unwrap();
    let id = prepared.run_id().to_owned();
    drop(prepared);
    id
}

#[test]
fn resume_after_a_crash_interrupts_the_orphan_once_and_dispatches_the_queue_once() {
    let w = world();
    let orphan = orphan(&w);
    // Restart: the desktop pauses a non-empty queue on startup.
    let mut workspace = Workspace::open(&w.data).unwrap();
    assert_eq!(run(&workspace, &orphan).state, RunState::Working);
    workspace
        .queue_message("queue-1", &w.questions, "When can the crane arrive?")
        .unwrap();
    workspace.recover_queue("startup").unwrap();

    // Before: "Finish or stop current work before resuming the queue".
    workspace.resume_queue("resume-1").unwrap();
    let recovered = run(&workspace, &orphan);
    assert_eq!(
        (recovered.state, recovered.fence),
        (RunState::Interrupted, 2)
    );
    assert_eq!(
        attention(&workspace, &w.intake),
        vec![Attention {
            kind: AttentionKind::Blocker,
            reason: INTERRUPTED_ATTENTION.into(),
            record: Some(orphan.clone()),
        }]
    );
    assert!(
        workspace
            .needs_you()
            .unwrap()
            .iter()
            .any(|chat| chat.id == w.intake)
    );

    let dispatched = workspace.dispatch_queued("dispatch-1").unwrap().unwrap();
    assert_eq!(dispatched.text, "When can the crane arrive?");
    assert!(!matches!(
        workspace.dispatch_queued("dispatch-2"),
        Ok(Some(_))
    ));
    assert_eq!(owner_messages(&workspace, &w.questions), 1);
}

#[test]
fn a_live_run_keeps_blocking_resume_and_dispatch() {
    let w = world();
    let live = prepare_run(intake_request(&w.data, &w.intake)).unwrap();
    let id = live.run_id().to_owned();
    let mut workspace = Workspace::open(&w.data).unwrap();
    workspace
        .queue_message("queue-1", &w.questions, "When can the crane arrive?")
        .unwrap();
    let dispatch = workspace.dispatch_queued("dispatch-1").unwrap_err();
    assert!(
        dispatch.to_string().contains("work is still running"),
        "{dispatch}"
    );
    workspace.recover_queue("pause").unwrap();
    let resume = workspace.resume_queue("resume-1").unwrap_err();
    assert!(
        resume.to_string().contains("Finish or stop current work"),
        "{resume}"
    );
    assert!(workspace.recover_orphaned_runs().unwrap().is_empty());
    assert_eq!(run(&workspace, &id).state, RunState::Working);
    assert!(attention(&workspace, &w.intake).is_empty());
    drop(live);
    // Once its process lets go, the same Resume recovers it.
    workspace.resume_queue("resume-2").unwrap();
    assert_eq!(run(&workspace, &id).state, RunState::Interrupted);
}

#[test]
fn recovery_runs_once_and_cannot_be_undone() {
    let w = world();
    let orphan = orphan(&w);
    let mut workspace = Workspace::open(&w.data).unwrap();
    assert_eq!(workspace.recover_orphaned_runs().unwrap(), vec![orphan]);
    let receipts = workspace.history().unwrap();
    assert!(workspace.recover_orphaned_runs().unwrap().is_empty());
    assert_eq!(workspace.history().unwrap().len(), receipts.len());
    let recovery = receipts
        .iter()
        .rfind(|r| r.reason == "Interrupt work whose process ended")
        .unwrap();
    let undo = workspace
        .undo(recovery.operation.as_str())
        .unwrap_err()
        .to_string();
    assert!(undo.contains("run execution control"), "{undo}");
}

#[test]
fn a_second_orphan_in_the_same_chat_leaves_one_notice() {
    let w = world();
    let first = orphan(&w);
    let mut workspace = Workspace::open(&w.data).unwrap();
    workspace.recover_orphaned_runs().unwrap();
    let second = orphan(&w);
    workspace.recover_orphaned_runs().unwrap();
    assert_eq!(run(&workspace, &first).state, RunState::Interrupted);
    assert_eq!(
        attention(&workspace, &w.intake),
        vec![Attention {
            kind: AttentionKind::Blocker,
            reason: INTERRUPTED_ATTENTION.into(),
            record: Some(second),
        }]
    );
}
