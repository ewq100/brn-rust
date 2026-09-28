use brn_store::{
    AnchorState, CommentCapture, CommentStatus, CommentStatusChange, DraftWriteWithComments,
    EditTrace, TextEdit,
};
use brn_workflow::{
    Config,
    worker::{Action, Outcome, Terminal, Worker},
};
use std::time::{Duration, Instant};
use uuid::Uuid;

fn terminal(worker: &Worker) -> Terminal {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(event) = worker.take_terminal() {
            return event;
        }
        assert!(Instant::now() < deadline, "worker terminal timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn worker_captures_saves_changes_status_and_reopens_comment_without_provider() {
    let dir = tempfile::tempdir().unwrap();
    let worker = Worker::start(dir.path().to_path_buf(), Config::default());
    assert!(matches!(
        terminal(&worker).outcome,
        Ok(Outcome::Ready { .. })
    ));
    worker
        .submit(Action::CreateDraft {
            op: Uuid::new_v4(),
            title: "draft".into(),
            text: "alpha beta gamma".into(),
        })
        .unwrap();
    let initial = match terminal(&worker).outcome.unwrap() {
        Outcome::DraftCreated { draft } => draft,
        other => panic!("{other:?}"),
    };
    let capture_op = Uuid::new_v4();
    let capture = CommentCapture {
        op: capture_op,
        draft_id: initial.id,
        expected: initial.stamp,
        generation: initial.stamp.generation,
        text: initial.text.clone(),
        edits: EditTrace::Steps(vec![]),
        range: 6..10,
        quote: "beta".into(),
        body: "Check this claim".into(),
    };
    let capture_job = worker
        .submit(Action::CreateDraftComment { request: capture })
        .unwrap();
    let event = terminal(&worker);
    assert_eq!(event.id, capture_job);
    assert_eq!(event.generation, Some(initial.stamp.generation));
    let (created, snapshots) = match event.outcome.unwrap() {
        Outcome::DraftCommentCreated { result, snapshots } => (result, snapshots),
        other => panic!("{other:?}"),
    };
    assert_eq!(created.op, capture_op);
    assert_eq!(created.submitted_generation, initial.stamp.generation);
    assert_ne!(
        created.saved.draft.stamp.base_revision,
        initial.stamp.base_revision
    );
    let view = &created.saved.comments[0];
    assert_eq!(view.comment.id, created.comment_id);
    assert_eq!(
        view.comment.original_revision_id,
        created.saved.draft.stamp.base_revision
    );
    assert_eq!(view.comment.original_quote, "beta");
    assert_eq!(view.anchor, AnchorState::Anchored { start: 6, end: 10 });
    assert!(snapshots.iter().any(|snapshot| snapshot.revision.id
        == view.comment.original_revision_id
        && snapshot.revision.text == "alpha beta gamma"
        && snapshot.anchors.contains(&(
            created.comment_id,
            AnchorState::Anchored { start: 6, end: 10 }
        ))));

    let write_op = Uuid::new_v4();
    let write = DraftWriteWithComments {
        op: write_op,
        draft_id: initial.id,
        expected: created.saved.draft.stamp,
        generation: created.saved.draft.stamp.generation + 1,
        text: "X alpha beta gamma".into(),
        edits: EditTrace::Steps(vec![TextEdit {
            start: 0,
            end: 0,
            replacement: "X ".into(),
        }]),
        checkpoint: false,
    };
    let write_job = worker
        .submit(Action::WriteDraftWithComments { request: write })
        .unwrap();
    let event = terminal(&worker);
    assert_eq!(event.id, write_job);
    assert_eq!(
        event.generation,
        Some(created.saved.draft.stamp.generation + 1)
    );
    let saved = match event.outcome.unwrap() {
        Outcome::DraftWrittenWithComments {
            op,
            draft_id,
            submitted_generation,
            submitted_text,
            saved,
            snapshots,
        } => {
            assert_eq!(op, write_op);
            assert_eq!(draft_id, initial.id);
            assert_eq!(
                submitted_generation,
                created.saved.draft.stamp.generation + 1
            );
            assert_eq!(submitted_text, "X alpha beta gamma");
            assert!(
                snapshots
                    .iter()
                    .any(|snapshot| snapshot.revision.id == view.comment.original_revision_id)
            );
            saved
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(
        saved.comments[0].anchor,
        AnchorState::Anchored { start: 8, end: 12 }
    );
    let saved_stamp = saved.draft.stamp;

    let status_op = Uuid::new_v4();
    let status_job = worker
        .submit(Action::SetCommentStatus {
            request: CommentStatusChange {
                op: status_op,
                draft_id: initial.id,
                comment_id: created.comment_id,
                expected_status_version: 0,
                status: CommentStatus::Resolved,
            },
        })
        .unwrap();
    let event = terminal(&worker);
    assert_eq!(event.id, status_job);
    match event.outcome.unwrap() {
        Outcome::CommentStatusChanged {
            result,
            saved,
            snapshots,
        } => {
            assert_eq!(result.op, status_op);
            assert_eq!(result.comment.status, CommentStatus::Resolved);
            assert_eq!(result.comment.status_version, 1);
            assert_eq!(saved.draft.stamp, saved_stamp);
            assert_eq!(
                saved.comments[0].anchor,
                AnchorState::Anchored { start: 8, end: 12 }
            );
            assert!(!snapshots.is_empty());
        }
        other => panic!("{other:?}"),
    }
    drop(worker);

    let worker = Worker::start(dir.path().to_path_buf(), Config::default());
    let _ = terminal(&worker);
    let open_job = worker
        .submit(Action::OpenDraftComments { id: initial.id })
        .unwrap();
    let event = terminal(&worker);
    assert_eq!(event.id, open_job);
    match event.outcome.unwrap() {
        Outcome::DraftCommentsOpened {
            id,
            saved,
            snapshots,
        } => {
            assert_eq!(id, initial.id);
            assert_eq!(saved.draft.stamp, saved_stamp);
            assert_eq!(saved.comments[0].comment.status, CommentStatus::Resolved);
            assert_eq!(
                saved.comments[0].anchor,
                AnchorState::Anchored { start: 8, end: 12 }
            );
            assert!(
                snapshots
                    .iter()
                    .any(|snapshot| snapshot.revision.id == view.comment.original_revision_id)
            );
        }
        other => panic!("{other:?}"),
    }

    let reopen_op = Uuid::new_v4();
    let reopen_job = worker
        .submit(Action::SetCommentStatus {
            request: CommentStatusChange {
                op: reopen_op,
                draft_id: initial.id,
                comment_id: created.comment_id,
                expected_status_version: 1,
                status: CommentStatus::Open,
            },
        })
        .unwrap();
    let event = terminal(&worker);
    assert_eq!(event.id, reopen_job);
    match event.outcome.unwrap() {
        Outcome::CommentStatusChanged { result, saved, .. } => {
            assert_eq!(result.op, reopen_op);
            assert_eq!(result.comment.status, CommentStatus::Open);
            assert_eq!(result.comment.status_version, 2);
            assert_eq!(saved.draft.stamp, saved_stamp);
            assert_eq!(
                saved.comments[0].anchor,
                AnchorState::Anchored { start: 8, end: 12 }
            );
        }
        other => panic!("{other:?}"),
    }
    let refresh_job = worker
        .submit(Action::RefreshDraftComments { id: initial.id })
        .unwrap();
    let event = terminal(&worker);
    assert_eq!(event.id, refresh_job);
    match event.outcome.unwrap() {
        Outcome::DraftCommentsRefreshed {
            id,
            saved,
            snapshots,
        } => {
            assert_eq!(id, initial.id);
            assert_eq!(saved.draft.stamp, saved_stamp);
            assert_eq!(saved.comments[0].comment.status, CommentStatus::Open);
            assert!(
                snapshots
                    .iter()
                    .any(|snapshot| snapshot.revision.id == view.comment.original_revision_id)
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn failed_capture_keeps_worker_job_and_generation_correlation() {
    let dir = tempfile::tempdir().unwrap();
    let worker = Worker::start(dir.path().to_path_buf(), Config::default());
    let _ = terminal(&worker);
    worker
        .submit(Action::CreateDraft {
            op: Uuid::new_v4(),
            title: "draft".into(),
            text: "alpha beta".into(),
        })
        .unwrap();
    let draft = match terminal(&worker).outcome.unwrap() {
        Outcome::DraftCreated { draft } => draft,
        other => panic!("{other:?}"),
    };
    let job = worker
        .submit(Action::CreateDraftComment {
            request: CommentCapture {
                op: Uuid::new_v4(),
                draft_id: draft.id,
                expected: draft.stamp,
                generation: draft.stamp.generation,
                text: draft.text.clone(),
                edits: EditTrace::Steps(vec![]),
                range: 6..10,
                quote: "wrong".into(),
                body: "note".into(),
            },
        })
        .unwrap();
    let event = terminal(&worker);
    assert_eq!(event.id, job);
    assert_eq!(event.generation, Some(draft.stamp.generation));
    assert!(event.outcome.is_err());
    worker
        .submit(Action::OpenDraftComments { id: draft.id })
        .unwrap();
    match terminal(&worker).outcome.unwrap() {
        Outcome::DraftCommentsOpened {
            saved, snapshots, ..
        } => {
            assert_eq!(saved.draft, draft);
            assert!(saved.comments.is_empty());
            assert!(snapshots.is_empty());
        }
        other => panic!("{other:?}"),
    }
}
