use brn_store::{OperationStatus, Store};
use brn_workflow::{
    Config, Workspace,
    worker::{Action, Outcome, Worker},
};
use std::time::{Duration, Instant};
use uuid::Uuid;

fn workspace() -> (tempfile::TempDir, Workspace) {
    let dir = tempfile::tempdir().unwrap();
    let ws = Workspace::open(dir.path(), Config::default()).unwrap();
    (dir, ws)
}

#[test]
fn revision_diff_preserves_newline_only_changes() {
    let (_dir, mut ws) = workspace();
    let d = ws.create_draft(Uuid::new_v4(), "title", "line").unwrap();
    let next = ws
        .checkpoint_draft(Uuid::new_v4(), d.id, d.stamp, 1, "line\n")
        .unwrap();
    let diff = ws
        .compare_draft_revisions(d.id, d.stamp.base_revision, next.stamp.base_revision)
        .unwrap();
    assert!(diff.contains("-line"), "{diff}");
    assert!(diff.contains("+line"), "{diff}");
    assert!(diff.contains("\\ No newline at end of file"), "{diff}");
    assert!(diff.contains(&d.stamp.base_revision.to_string()));
    assert!(diff.contains(&next.stamp.base_revision.to_string()));
    assert!(
        ws.compare_draft_revisions(d.id, next.stamp.base_revision, next.stamp.base_revision)
            .unwrap()
            .contains("No changes")
    );
}

#[test]
fn comparison_rejects_cross_draft_revisions() {
    let (_dir, mut ws) = workspace();
    let a = ws.create_draft(Uuid::new_v4(), "a", "a").unwrap();
    let b = ws.create_draft(Uuid::new_v4(), "b", "b").unwrap();
    assert!(
        ws.compare_draft_revisions(a.id, a.stamp.base_revision, b.stamp.base_revision)
            .is_err()
    );
}

fn terminal(worker: &Worker) -> brn_workflow::worker::Terminal {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(event) = worker.take_terminal() {
            return event;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn worker_saves_checkpoints_and_reopens_without_provider() {
    let dir = tempfile::tempdir().unwrap();
    let worker = Worker::start(dir.path().to_path_buf(), Config::default());
    assert!(matches!(
        terminal(&worker).outcome,
        Ok(Outcome::Ready { .. })
    ));
    worker
        .submit(Action::CreateDraft {
            op: Uuid::new_v4(),
            title: "title".into(),
            text: "start".into(),
        })
        .unwrap();
    let d = match terminal(&worker).outcome.unwrap() {
        Outcome::DraftCreated { draft } => draft,
        x => panic!("{x:?}"),
    };
    worker
        .submit(Action::SaveDraft {
            op: Uuid::new_v4(),
            id: d.id,
            expected: d.stamp,
            generation: 1,
            text: "working".into(),
        })
        .unwrap();
    let saved = match terminal(&worker).outcome.unwrap() {
        Outcome::DraftSaved {
            draft,
            submitted_generation,
            ..
        } => {
            assert_eq!(submitted_generation, 1);
            draft
        }
        x => panic!("{x:?}"),
    };
    worker
        .submit(Action::CheckpointDraft {
            op: Uuid::new_v4(),
            id: d.id,
            expected: saved.stamp,
            generation: 1,
            text: "working".into(),
        })
        .unwrap();
    let checked = match terminal(&worker).outcome.unwrap() {
        Outcome::DraftCheckpointed { draft, .. } => draft,
        x => panic!("{x:?}"),
    };
    drop(worker);
    let worker = Worker::start(dir.path().to_path_buf(), Config::default());
    let _ = terminal(&worker);
    worker.submit(Action::OpenDraft { id: d.id }).unwrap();
    let reopened = match terminal(&worker).outcome.unwrap() {
        Outcome::DraftOpened { id, draft } => {
            assert_eq!(id, d.id);
            draft
        }
        x => panic!("{x:?}"),
    };
    assert_eq!(reopened, checked);
}

#[test]
fn candidate_is_separate_from_concurrent_user_edits() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let session = store
        .create_session(
            Uuid::new_v4(),
            "codex",
            "local",
            None,
            Some("thread"),
            b"{}",
        )
        .unwrap();
    let turn = Uuid::new_v4();
    store
        .prepare_turn(turn, session, "question", "grounded", "[]")
        .unwrap();
    store
        .complete_turn(turn, OperationStatus::Completed, "answer\n", None)
        .unwrap();
    drop(store);
    let mut ws = Workspace::open(dir.path(), Config::default()).unwrap();
    let d = ws.create_draft(Uuid::new_v4(), "title", "root").unwrap();
    let saved = ws
        .save_draft(Uuid::new_v4(), d.id, d.stamp, 1, "own edit")
        .unwrap();
    let candidate = ws
        .candidate_from_turn(Uuid::new_v4(), d.id, d.stamp.base_revision, turn)
        .unwrap();
    assert_eq!(candidate.text, "answer\n");
    assert_eq!(candidate.origin_turn, Some(turn));
    assert_eq!(ws.draft(d.id).unwrap().unwrap(), saved);
    drop(ws);
    let reopened = Workspace::open(dir.path(), Config::default()).unwrap();
    assert_eq!(reopened.draft(d.id).unwrap().unwrap(), saved);
    assert_eq!(
        reopened.draft_revision(candidate.id).unwrap().unwrap(),
        candidate
    );
}
