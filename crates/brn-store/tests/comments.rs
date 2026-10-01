use brn_store::{
    AnchorState, CommentCapture, CommentStatus, CommentStatusChange, DraftWriteWithComments,
    EditTrace, Store, TextEdit,
};
use rusqlite::Connection;
use uuid::Uuid;

fn setup(text: &str) -> (tempfile::TempDir, Store, brn_store::Draft) {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let draft = store.create_draft(Uuid::new_v4(), "draft", text).unwrap();
    (dir, store, draft)
}

fn capture(
    store: &mut Store,
    draft: &brn_store::Draft,
    range: std::ops::Range<usize>,
    quote: &str,
) -> brn_store::CommentCreated {
    store
        .create_draft_comment(CommentCapture {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: draft.stamp,
            generation: draft.stamp.generation,
            text: draft.text.clone(),
            edits: EditTrace::Steps(vec![]),
            range,
            quote: quote.into(),
            body: "note".into(),
        })
        .unwrap()
}

#[test]
fn dirty_capture_checkpoints_exact_quote_and_survives_reopen() {
    let (dir, mut store, draft) = setup("a TWO b");
    let request = CommentCapture {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: draft.stamp,
        generation: 1,
        text: "x a TWO b".into(),
        edits: EditTrace::Steps(vec![TextEdit {
            start: 0,
            end: 0,
            replacement: "x ".into(),
        }]),
        range: 4..7,
        quote: "TWO".into(),
        body: "  note\n".into(),
    };
    let created = store.create_draft_comment(request.clone()).unwrap();
    assert_eq!(created.saved.draft.text, "x a TWO b");
    assert_eq!(
        created.saved.comments[0].anchor,
        AnchorState::Anchored { start: 4, end: 7 }
    );
    let comment = &created.saved.comments[0].comment;
    assert_eq!(comment.body, "  note\n");
    assert_eq!(comment.original_quote, "TWO");
    assert_eq!(
        comment.original_revision_id,
        created.saved.draft.stamp.base_revision
    );
    assert_eq!(comment.original_sha256, created.saved.draft.sha256);
    assert_eq!(
        store
            .comment_anchor_snapshots(draft.id)
            .unwrap()
            .last()
            .unwrap()
            .anchors,
        vec![(comment.id, AnchorState::Anchored { start: 4, end: 7 })]
    );
    drop(store);
    let (store, _) = Store::open(dir.path()).unwrap();
    assert_eq!(store.draft_comments(draft.id).unwrap(), created.saved);
}

#[test]
fn traced_save_keeps_delete_then_retype_unresolved_and_retry_frozen() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let created = store
        .create_draft_comment(CommentCapture {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: draft.stamp,
            generation: 0,
            text: draft.text.clone(),
            edits: EditTrace::Steps(vec![]),
            range: 2..5,
            quote: "TWO".into(),
            body: "note".into(),
        })
        .unwrap();
    let saved = created.saved.draft;
    let request = DraftWriteWithComments {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: saved.stamp,
        generation: 3,
        text: "z TWO b".into(),
        checkpoint: true,
        edits: EditTrace::Steps(vec![
            TextEdit {
                start: 2,
                end: 5,
                replacement: String::new(),
            },
            TextEdit {
                start: 0,
                end: 1,
                replacement: "z".into(),
            },
            TextEdit {
                start: 2,
                end: 2,
                replacement: "TWO".into(),
            },
        ]),
    };
    let result = store.write_draft_with_comments(request.clone()).unwrap();
    assert_eq!(result.comments[0].anchor, AnchorState::Deleted);
    store
        .set_comment_status(CommentStatusChange {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            comment_id: created.comment_id,
            expected_status_version: 0,
            status: CommentStatus::Resolved,
        })
        .unwrap();
    assert_eq!(store.write_draft_with_comments(request).unwrap(), result);
    assert_eq!(
        store.draft_comments(draft.id).unwrap().comments[0]
            .comment
            .status,
        CommentStatus::Resolved
    );
}

#[test]
fn capture_validation_and_post_checkpoint_failure_leave_all_rows_unchanged() {
    let (dir, mut store, draft) = setup("a 🙂 b");
    let baseline = store.draft_revisions(draft.id).unwrap().len();
    for (range, quote, body) in [
        (2..2, "", "note"),
        (3..6, "🙂", "note"),
        (2..9, "🙂", "note"),
        (2..6, "wrong", "note"),
        (2..6, "🙂", " \n"),
        (2..6, "🙂", "x".repeat(65_537).as_str()),
    ] {
        let request = CommentCapture {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: draft.stamp,
            generation: 0,
            text: draft.text.clone(),
            edits: EditTrace::Steps(vec![]),
            range,
            quote: quote.into(),
            body: body.into(),
        };
        assert!(store.create_draft_comment(request).is_err());
    }
    assert_eq!(store.draft_revisions(draft.id).unwrap().len(), baseline);
    assert!(store.draft_comments(draft.id).unwrap().comments.is_empty());
    let db = store.database_path().to_owned();
    let conn = Connection::open(db).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_comment_insert BEFORE INSERT ON draft_comments BEGIN SELECT RAISE(ABORT,'forced'); END;").unwrap();
    let request = CommentCapture {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: draft.stamp,
        generation: 0,
        text: draft.text.clone(),
        edits: EditTrace::Steps(vec![]),
        range: 2..6,
        quote: "🙂".into(),
        body: "note".into(),
    };
    let failed_op = request.op;
    assert!(store.create_draft_comment(request).is_err());
    conn.execute_batch("DROP TRIGGER fail_comment_insert")
        .unwrap();
    let operation_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM operations WHERE id=?1",
            [failed_op.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(operation_count, 0);
    assert_eq!(store.draft_revisions(draft.id).unwrap().len(), baseline);
    assert_eq!(store.draft(draft.id).unwrap().unwrap(), draft);
    assert!(store.draft_comments(draft.id).unwrap().comments.is_empty());
    drop(store);
    assert!(Store::open(dir.path()).is_ok());
}

#[test]
fn two_comments_snapshots_and_legacy_writers_keep_current_stamp() {
    let (_dir, mut store, draft) = setup("one TWO three FOUR");
    let first = capture(&mut store, &draft, 4..7, "TWO");
    let second = capture(&mut store, &first.saved.draft, 14..18, "FOUR");
    let snapshots = store.comment_anchor_snapshots(draft.id).unwrap();
    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].anchors.len(), 1);
    assert_eq!(snapshots[1].anchors.len(), 2);
    let saved = store
        .save_draft(
            Uuid::new_v4(),
            draft.id,
            second.saved.draft.stamp,
            1,
            "x one TWO three FOUR",
        )
        .unwrap();
    let current = store.draft_comments(draft.id).unwrap();
    assert_eq!(current.draft, saved);
    assert_eq!(
        current.comments[0].anchor,
        AnchorState::Anchored { start: 6, end: 9 }
    );
    assert_eq!(
        current.comments[1].anchor,
        AnchorState::Anchored { start: 16, end: 20 }
    );
    let checkpoint = store
        .checkpoint_draft(Uuid::new_v4(), draft.id, saved.stamp, 1, &saved.text)
        .unwrap();
    assert_eq!(store.draft_comments(draft.id).unwrap().draft, checkpoint);
    assert_eq!(
        store
            .comment_anchor_snapshots(draft.id)
            .unwrap()
            .last()
            .unwrap()
            .anchors
            .len(),
        2
    );
}

#[test]
fn delete_recover_redo_and_history_limit_persist_distinct_states() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let created = capture(&mut store, &draft, 2..5, "TWO");
    let current = created.saved.draft;
    let deleted = store
        .write_draft_with_comments(DraftWriteWithComments {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: current.stamp,
            generation: 1,
            text: "a  b".into(),
            edits: EditTrace::Steps(vec![TextEdit {
                start: 2,
                end: 5,
                replacement: String::new(),
            }]),
            checkpoint: false,
        })
        .unwrap();
    assert_eq!(deleted.comments[0].anchor, AnchorState::Deleted);
    let recovered = store
        .write_draft_with_comments(DraftWriteWithComments {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: deleted.draft.stamp,
            generation: 2,
            text: "a TWO b".into(),
            edits: EditTrace::Steps(vec![TextEdit {
                start: 2,
                end: 2,
                replacement: "TWO".into(),
            }]),
            checkpoint: false,
        })
        .unwrap();
    assert_eq!(
        recovered.comments[0].anchor,
        AnchorState::Anchored { start: 2, end: 5 }
    );
    let redone = store
        .write_draft_with_comments(DraftWriteWithComments {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: recovered.draft.stamp,
            generation: 3,
            text: "a  b".into(),
            edits: EditTrace::Steps(vec![TextEdit {
                start: 2,
                end: 5,
                replacement: String::new(),
            }]),
            checkpoint: true,
        })
        .unwrap();
    assert_eq!(redone.comments[0].anchor, AnchorState::Deleted);
    let lost = store
        .write_draft_with_comments(DraftWriteWithComments {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: redone.draft.stamp,
            generation: 4,
            text: "z TWO b".into(),
            edits: EditTrace::HistoryLost,
            checkpoint: false,
        })
        .unwrap();
    assert_eq!(
        lost.comments[0].anchor,
        AnchorState::Ambiguous {
            reason: brn_store::AmbiguityReason::HistoryLimit
        }
    );
}

#[test]
fn status_cas_noop_and_request_conflicts_preserve_text_and_location() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let created = capture(&mut store, &draft, 2..5, "TWO");
    let before = store.draft_comments(draft.id).unwrap();
    let no_op = store
        .set_comment_status(CommentStatusChange {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            comment_id: created.comment_id,
            expected_status_version: 0,
            status: CommentStatus::Open,
        })
        .unwrap();
    assert_eq!(no_op.comment.status_version, 0);
    let request = CommentStatusChange {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        comment_id: created.comment_id,
        expected_status_version: 0,
        status: CommentStatus::Resolved,
    };
    let resolved = store.set_comment_status(request.clone()).unwrap();
    assert_eq!(resolved.comment.status_version, 1);
    assert!(
        store
            .set_comment_status(CommentStatusChange {
                op: Uuid::new_v4(),
                ..request.clone()
            })
            .is_err()
    );
    assert_eq!(store.set_comment_status(request.clone()).unwrap(), resolved);
    assert!(
        store
            .set_comment_status(CommentStatusChange {
                status: CommentStatus::Open,
                ..request
            })
            .is_err()
    );
    let after = store.draft_comments(draft.id).unwrap();
    assert_eq!(after.draft, before.draft);
    assert_eq!(after.comments[0].anchor, before.comments[0].anchor);
    assert_eq!(
        after.comments[0].comment.original_quote,
        before.comments[0].comment.original_quote
    );
}

#[test]
fn stale_generation_cross_draft_and_changed_replay_are_rejected() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let created = capture(&mut store, &draft, 2..5, "TWO");
    let other = store
        .create_draft(Uuid::new_v4(), "other", "plain")
        .unwrap();
    let stale = CommentCapture {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: draft.stamp,
        generation: 0,
        text: draft.text.clone(),
        edits: EditTrace::Steps(vec![]),
        range: 2..5,
        quote: "TWO".into(),
        body: "late".into(),
    };
    assert!(store.create_draft_comment(stale).is_err());
    let generation_overflow = DraftWriteWithComments {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: created.saved.draft.stamp,
        generation: u64::MAX,
        text: draft.text.clone(),
        edits: EditTrace::Steps(vec![]),
        checkpoint: false,
    };
    assert!(
        store
            .write_draft_with_comments(generation_overflow)
            .is_err()
    );
    assert!(
        store
            .set_comment_status(CommentStatusChange {
                op: Uuid::new_v4(),
                draft_id: other.id,
                comment_id: created.comment_id,
                expected_status_version: 0,
                status: CommentStatus::Resolved
            })
            .is_err()
    );
    assert!(
        store
            .set_comment_status(CommentStatusChange {
                op: Uuid::new_v4(),
                draft_id: draft.id,
                comment_id: created.comment_id,
                expected_status_version: u64::MAX,
                status: CommentStatus::Resolved
            })
            .is_err()
    );
    let request = DraftWriteWithComments {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: created.saved.draft.stamp,
        generation: 1,
        text: "x a TWO b".into(),
        edits: EditTrace::Steps(vec![TextEdit {
            start: 0,
            end: 0,
            replacement: "x ".into(),
        }]),
        checkpoint: false,
    };
    let result = store.write_draft_with_comments(request.clone()).unwrap();
    let before = store.draft_comments(draft.id).unwrap();
    assert!(
        store
            .write_draft_with_comments(DraftWriteWithComments {
                edits: EditTrace::HistoryLost,
                ..request.clone()
            })
            .is_err()
    );
    assert!(
        store
            .write_draft_with_comments(DraftWriteWithComments {
                checkpoint: true,
                ..request.clone()
            })
            .is_err()
    );
    assert_eq!(store.write_draft_with_comments(request).unwrap(), result);
    assert_eq!(store.draft_comments(draft.id).unwrap(), before);
}

#[test]
fn malformed_trace_and_cumulative_replacement_limit_leave_rows_unchanged() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let created = capture(&mut store, &draft, 2..5, "TWO");
    let base = created.saved.draft;
    let malformed = DraftWriteWithComments {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: base.stamp,
        generation: 1,
        text: "z TWO b".into(),
        edits: EditTrace::Steps(vec![TextEdit {
            start: 0,
            end: 1,
            replacement: "x".into(),
        }]),
        checkpoint: true,
    };
    assert!(store.write_draft_with_comments(malformed).is_err());
    assert_eq!(store.draft_comments(draft.id).unwrap().draft, base);
    // First enter a full-size valid buffer through HistoryLost, then submit
    // nine individually valid whole-document replacements. Their sum exceeds 8 MiB.
    let large = "a".repeat(1_048_576);
    let saved = store
        .write_draft_with_comments(DraftWriteWithComments {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: base.stamp,
            generation: 1,
            text: large.clone(),
            edits: EditTrace::HistoryLost,
            checkpoint: false,
        })
        .unwrap();
    let mut steps = Vec::new();
    for index in 0..9 {
        steps.push(TextEdit {
            start: 0,
            end: large.len(),
            replacement: if index % 2 == 0 {
                "b".repeat(large.len())
            } else {
                large.clone()
            },
        });
    }
    let request = DraftWriteWithComments {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: saved.draft.stamp,
        generation: 10,
        text: "b".repeat(large.len()),
        edits: EditTrace::Steps(steps),
        checkpoint: true,
    };
    assert!(store.write_draft_with_comments(request).is_err());
    assert_eq!(store.draft_comments(draft.id).unwrap(), saved);
}

#[test]
fn invalid_original_and_receipt_identity_fail_reads_and_retries() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let request = CommentCapture {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: draft.stamp,
        generation: 0,
        text: draft.text.clone(),
        edits: EditTrace::Steps(vec![]),
        range: 2..5,
        quote: "TWO".into(),
        body: "note".into(),
    };
    let created = store.create_draft_comment(request.clone()).unwrap();
    let conn = Connection::open(store.database_path()).unwrap();
    conn.execute(
        "UPDATE draft_comments SET original_sha256=?2 WHERE id=?1",
        rusqlite::params![created.comment_id.to_string(), vec![0u8; 32]],
    )
    .unwrap();
    assert!(store.draft_comments(draft.id).is_err());
    assert!(store.create_draft_comment(request.clone()).is_err());
    conn.execute(
        "UPDATE draft_comments SET original_sha256=?2,original_start=1 WHERE id=?1",
        rusqlite::params![
            created.comment_id.to_string(),
            created.saved.draft.sha256.as_slice()
        ],
    )
    .unwrap();
    assert!(store.draft_comments(draft.id).is_err());
    conn.execute(
        "UPDATE draft_comments SET original_start=2 WHERE id=?1",
        [created.comment_id.to_string()],
    )
    .unwrap();
    let mut receipt: serde_json::Value = conn
        .query_row(
            "SELECT result_json FROM comment_results WHERE operation_id=?1",
            [request.op.to_string()],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .map(|b| serde_json::from_slice(&b).unwrap())
        .unwrap();
    receipt["comment_id"] = serde_json::json!(Uuid::new_v4());
    conn.execute(
        "UPDATE comment_results SET result_json=?2 WHERE operation_id=?1",
        rusqlite::params![
            request.op.to_string(),
            serde_json::to_vec(&receipt).unwrap()
        ],
    )
    .unwrap();
    assert!(store.create_draft_comment(request).is_err());
}

#[test]
fn cross_draft_checkpoint_snapshot_is_rejected_on_read() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let created = capture(&mut store, &draft, 2..5, "TWO");
    let other = store
        .create_draft(Uuid::new_v4(), "other", "a TWO b")
        .unwrap();
    let conn = Connection::open(store.database_path()).unwrap();
    conn.execute(
        "UPDATE draft_revision_comment_anchors SET revision_id=?2 WHERE comment_id=?1",
        rusqlite::params![
            created.comment_id.to_string(),
            other.stamp.base_revision.to_string()
        ],
    )
    .unwrap();
    assert!(store.comment_anchor_snapshots(draft.id).is_err());
}

#[test]
fn checkpoint_receipt_cannot_name_another_valid_quote_occurrence() {
    let (_dir, mut store, draft) = setup("TWO x TWO");
    let created = capture(&mut store, &draft, 0..3, "TWO");
    let request = DraftWriteWithComments {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: created.saved.draft.stamp,
        generation: 0,
        text: draft.text.clone(),
        edits: EditTrace::Steps(vec![]),
        checkpoint: true,
    };
    let saved = store.write_draft_with_comments(request.clone()).unwrap();
    assert_eq!(
        saved.comments[0].anchor,
        AnchorState::Anchored { start: 0, end: 3 }
    );
    let conn = Connection::open(store.database_path()).unwrap();
    let mut receipt: serde_json::Value = conn
        .query_row(
            "SELECT result_json FROM comment_results WHERE operation_id=?1",
            [request.op.to_string()],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .map(|b| serde_json::from_slice(&b).unwrap())
        .unwrap();
    receipt["comments"][0]["anchor"]["Anchored"]["start"] = serde_json::json!(6);
    receipt["comments"][0]["anchor"]["Anchored"]["end"] = serde_json::json!(9);
    conn.execute(
        "UPDATE comment_results SET result_json=?2 WHERE operation_id=?1",
        rusqlite::params![
            request.op.to_string(),
            serde_json::to_vec(&receipt).unwrap()
        ],
    )
    .unwrap();
    assert!(store.write_draft_with_comments(request).is_err());
}

#[test]
fn ordinary_save_retry_rejects_any_changed_frozen_projection() {
    let (_dir, mut store, draft) = setup("TWO x TWO");
    let created = capture(&mut store, &draft, 0..3, "TWO");
    let request = DraftWriteWithComments {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: created.saved.draft.stamp,
        generation: 1,
        text: draft.text.clone(),
        edits: EditTrace::Steps(vec![]),
        checkpoint: false,
    };
    let saved = store.write_draft_with_comments(request.clone()).unwrap();
    let conn = Connection::open(store.database_path()).unwrap();
    let original: Vec<u8> = conn
        .query_row(
            "SELECT result_json FROM comment_results WHERE operation_id=?1",
            [request.op.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    for mutation in 0..4 {
        let mut receipt: serde_json::Value = serde_json::from_slice(&original).unwrap();
        match mutation {
            0 => {
                receipt["comments"][0]["anchor"]["Anchored"]["start"] = serde_json::json!(6);
                receipt["comments"][0]["anchor"]["Anchored"]["end"] = serde_json::json!(9);
            }
            1 => {
                receipt["comments"] = serde_json::json!([]);
            }
            2 => {
                receipt["comments"][0]["anchor"] = serde_json::json!("Deleted");
            }
            3 => {
                receipt["comments"][0]["comment"]["status"] = serde_json::json!("Resolved");
            }
            _ => unreachable!(),
        }
        conn.execute(
            "UPDATE comment_results SET result_json=?2 WHERE operation_id=?1",
            rusqlite::params![
                request.op.to_string(),
                serde_json::to_vec(&receipt).unwrap()
            ],
        )
        .unwrap();
        assert!(
            store.write_draft_with_comments(request.clone()).is_err(),
            "mutation {mutation}"
        );
        conn.execute(
            "UPDATE comment_results SET result_json=?2 WHERE operation_id=?1",
            rusqlite::params![request.op.to_string(), &original],
        )
        .unwrap();
        assert_eq!(store.draft_comments(draft.id).unwrap(), saved);
    }
    assert_eq!(store.write_draft_with_comments(request).unwrap(), saved);
}

#[test]
fn sqlite_rejects_ambiguous_null_reason_in_current_and_checkpoint_tables() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let created = capture(&mut store, &draft, 2..5, "TWO");
    let conn = Connection::open(store.database_path()).unwrap();
    assert!(conn.execute("UPDATE draft_comment_anchors SET location='ambiguous',reason=NULL,start=NULL,end=NULL WHERE comment_id=?1", [created.comment_id.to_string()]).is_err());
    assert!(conn.execute("UPDATE draft_revision_comment_anchors SET location='ambiguous',reason=NULL,start=NULL,end=NULL WHERE comment_id=?1", [created.comment_id.to_string()]).is_err());
    assert_eq!(conn.execute("UPDATE draft_comment_anchors SET location='ambiguous',reason='touched',start=NULL,end=NULL WHERE comment_id=?1", [created.comment_id.to_string()]).unwrap(), 1);
    assert_eq!(conn.execute("UPDATE draft_revision_comment_anchors SET location='ambiguous',reason='touched',start=NULL,end=NULL WHERE comment_id=?1", [created.comment_id.to_string()]).unwrap(), 1);
}

#[test]
fn capture_status_and_write_retries_verify_persisted_receipt_digest() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let capture = CommentCapture {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: draft.stamp,
        generation: 0,
        text: draft.text.clone(),
        edits: EditTrace::Steps(vec![]),
        range: 2..5,
        quote: "TWO".into(),
        body: "note".into(),
    };
    let created = store.create_draft_comment(capture.clone()).unwrap();
    let status = CommentStatusChange {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        comment_id: created.comment_id,
        expected_status_version: 0,
        status: CommentStatus::Resolved,
    };
    let changed = store.set_comment_status(status.clone()).unwrap();
    let write = DraftWriteWithComments {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: created.saved.draft.stamp,
        generation: 1,
        text: "x a TWO b".into(),
        edits: EditTrace::Steps(vec![TextEdit {
            start: 0,
            end: 0,
            replacement: "x ".into(),
        }]),
        checkpoint: false,
    };
    let saved = store.write_draft_with_comments(write.clone()).unwrap();
    let conn = Connection::open(store.database_path()).unwrap();
    for op in [capture.op, status.op, write.op] {
        let original: Vec<u8> = conn
            .query_row(
                "SELECT result_sha256 FROM comment_results WHERE operation_id=?1",
                [op.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "UPDATE comment_results SET result_sha256=?2 WHERE operation_id=?1",
            rusqlite::params![op.to_string(), vec![0u8; 32]],
        )
        .unwrap();
        let retry = if op == capture.op {
            store.create_draft_comment(capture.clone()).map(|_| ())
        } else if op == status.op {
            store.set_comment_status(status.clone()).map(|_| ())
        } else {
            store.write_draft_with_comments(write.clone()).map(|_| ())
        };
        assert!(retry.is_err());
        conn.execute(
            "UPDATE comment_results SET result_sha256=?2 WHERE operation_id=?1",
            rusqlite::params![op.to_string(), original],
        )
        .unwrap();
    }
    assert_eq!(store.create_draft_comment(capture).unwrap(), created);
    assert_eq!(store.set_comment_status(status).unwrap(), changed);
    assert_eq!(store.write_draft_with_comments(write).unwrap(), saved);
    assert_eq!(store.draft_comments(draft.id).unwrap().draft, saved.draft);
}

#[test]
fn separate_process_reopens_comment_mapping_and_lifecycle() {
    let (dir, mut store, draft) = setup("a TWO b");
    let created = capture(&mut store, &draft, 2..5, "TWO");
    let saved = store
        .write_draft_with_comments(DraftWriteWithComments {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: created.saved.draft.stamp,
            generation: 1,
            text: "x a TWO b".into(),
            edits: EditTrace::Steps(vec![TextEdit {
                start: 0,
                end: 0,
                replacement: "x ".into(),
            }]),
            checkpoint: true,
        })
        .unwrap();
    store
        .set_comment_status(CommentStatusChange {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            comment_id: created.comment_id,
            expected_status_version: 0,
            status: CommentStatus::Resolved,
        })
        .unwrap();
    drop(store);
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("child_reopen_fixture")
        .arg("--nocapture")
        .env("BRN_COMMENT_REOPEN_DIR", dir.path())
        .env("BRN_COMMENT_REOPEN_DRAFT", draft.id.to_string())
        .env("BRN_COMMENT_REOPEN_COMMENT", created.comment_id.to_string())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("child_reopen_fixture ... ok"));
    assert_eq!(
        saved.comments[0].anchor,
        AnchorState::Anchored { start: 4, end: 7 }
    );
}

#[test]
fn child_reopen_fixture() {
    let Ok(path) = std::env::var("BRN_COMMENT_REOPEN_DIR") else {
        return;
    };
    let draft = Uuid::parse_str(&std::env::var("BRN_COMMENT_REOPEN_DRAFT").unwrap()).unwrap();
    let comment = Uuid::parse_str(&std::env::var("BRN_COMMENT_REOPEN_COMMENT").unwrap()).unwrap();
    let (store, _) = Store::open(path).unwrap();
    let current = store.draft_comments(draft).unwrap();
    assert_eq!(current.draft.text, "x a TWO b");
    assert_eq!(current.comments[0].comment.id, comment);
    assert_eq!(current.comments[0].comment.status, CommentStatus::Resolved);
    assert_eq!(
        current.comments[0].anchor,
        AnchorState::Anchored { start: 4, end: 7 }
    );
    assert_eq!(store.comment_anchor_snapshots(draft).unwrap().len(), 2);
}

#[test]
fn v4_draft_save_retry_receipt_survives_v5_migration() {
    let (dir, mut store, draft) = setup("old");
    let op = Uuid::new_v4();
    let saved = store
        .save_draft(op, draft.id, draft.stamp, 1, "new")
        .unwrap();
    drop(store);
    let conn = Connection::open(dir.path().join("brn.sqlite3")).unwrap();
    conn.execute_batch(
        "DROP TABLE note_search_results; DROP TABLE note_search_snapshots;
         DROP TABLE note_decision_recoveries; DROP TABLE note_write_destinations;
         DROP TABLE note_receipts; DROP TABLE note_shadowed_sources; DROP TABLE note_recovery_pairs;
         DROP TABLE note_save_intents; DROP TABLE note_write_inputs; DROP TABLE note_results;
         DROP TABLE note_buffers; DROP TABLE notes; DROP TABLE note_vaults;
         DROP TABLE chat_turns;
         CREATE TABLE chat_turns (operation_id TEXT PRIMARY KEY REFERENCES operations(id), session_id TEXT NOT NULL REFERENCES sessions(id), question TEXT NOT NULL, profile TEXT NOT NULL, evidence_json TEXT NOT NULL, answer TEXT, provider_turn_id TEXT, usage_json TEXT);
         DROP TABLE comment_results; DROP TABLE draft_revision_comment_anchors;
         DROP TABLE draft_comment_anchors; DROP TABLE draft_comments;
         PRAGMA user_version=4;"
    ).unwrap();
    drop(conn);
    let (mut store, report) = Store::open(dir.path()).unwrap();
    assert_eq!(report.migrated_from, Some(4));
    assert_eq!(
        store
            .save_draft(op, draft.id, draft.stamp, 1, "new")
            .unwrap(),
        saved
    );
    assert_eq!(store.draft_comments(draft.id).unwrap().draft, saved);
    let created = capture(&mut store, &saved, 0..3, "new");
    let conn = Connection::open(store.database_path()).unwrap();
    assert!(conn.execute("UPDATE draft_comment_anchors SET location='ambiguous',reason=NULL,start=NULL,end=NULL WHERE comment_id=?1", [created.comment_id.to_string()]).is_err());
    assert!(conn.execute("UPDATE draft_revision_comment_anchors SET location='ambiguous',reason=NULL,start=NULL,end=NULL WHERE comment_id=?1", [created.comment_id.to_string()]).is_err());
}

#[test]
fn old_capture_and_status_receipts_replay_after_later_writes_without_mutation() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let capture = CommentCapture {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: draft.stamp,
        generation: 0,
        text: draft.text.clone(),
        edits: EditTrace::Steps(vec![]),
        range: 2..5,
        quote: "TWO".into(),
        body: "note".into(),
    };
    let created = store.create_draft_comment(capture.clone()).unwrap();
    let status = CommentStatusChange {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        comment_id: created.comment_id,
        expected_status_version: 0,
        status: CommentStatus::Resolved,
    };
    let resolved = store.set_comment_status(status.clone()).unwrap();
    let later = store
        .write_draft_with_comments(DraftWriteWithComments {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: created.saved.draft.stamp,
            generation: 1,
            text: "x a TWO b".into(),
            edits: EditTrace::Steps(vec![TextEdit {
                start: 0,
                end: 0,
                replacement: "x ".into(),
            }]),
            checkpoint: true,
        })
        .unwrap();
    store
        .set_comment_status(CommentStatusChange {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            comment_id: created.comment_id,
            expected_status_version: 1,
            status: CommentStatus::Open,
        })
        .unwrap();
    let current = store.draft_comments(draft.id).unwrap();
    let revisions = store.draft_revisions(draft.id).unwrap();
    assert_eq!(store.create_draft_comment(capture).unwrap(), created);
    assert_eq!(store.set_comment_status(status).unwrap(), resolved);
    assert_eq!(store.draft_comments(draft.id).unwrap(), current);
    assert_eq!(store.draft_revisions(draft.id).unwrap(), revisions);
    assert_eq!(later.comments[0].comment.status, CommentStatus::Resolved);
}

#[test]
fn unicode_multistep_save_keeps_exact_byte_range_and_checkpoint_snapshot() {
    let (_dir, mut store, draft) = setup("🙂e\u{301}\r\n猫\n");
    let created = capture(&mut store, &draft, 9..12, "猫");
    let saved = store
        .write_draft_with_comments(DraftWriteWithComments {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: created.saved.draft.stamp,
            generation: 2,
            text: "é🙂e\u{301}\r\n猫\n!".into(),
            checkpoint: true,
            edits: EditTrace::Steps(vec![
                TextEdit {
                    start: 0,
                    end: 0,
                    replacement: "é".into(),
                },
                TextEdit {
                    start: 15,
                    end: 15,
                    replacement: "!".into(),
                },
            ]),
        })
        .unwrap();
    assert_eq!(
        saved.comments[0].anchor,
        AnchorState::Anchored { start: 11, end: 14 }
    );
    assert_eq!(
        store
            .comment_anchor_snapshots(draft.id)
            .unwrap()
            .last()
            .unwrap()
            .anchors[0]
            .1,
        saved.comments[0].anchor
    );
}

#[test]
fn sql_constraints_and_current_target_validation_reject_corruption() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let created = capture(&mut store, &draft, 2..5, "TWO");
    let conn = Connection::open(store.database_path()).unwrap();
    assert!(
        conn.execute(
            "UPDATE draft_comment_anchors SET location='deleted',start=2 WHERE comment_id=?1",
            [created.comment_id.to_string()]
        )
        .is_err()
    );
    conn.execute(
        "UPDATE draft_comment_anchors SET target_sha256=?2 WHERE comment_id=?1",
        rusqlite::params![created.comment_id.to_string(), vec![0u8; 32]],
    )
    .unwrap();
    assert!(store.draft_comments(draft.id).is_err());
    conn.execute(
        "UPDATE draft_comment_anchors SET target_sha256=?2 WHERE comment_id=?1",
        rusqlite::params![
            created.comment_id.to_string(),
            created.saved.draft.sha256.as_slice()
        ],
    )
    .unwrap();
    assert_eq!(store.draft_comments(draft.id).unwrap(), created.saved);
}

#[test]
fn stored_status_version_at_sqlite_max_cannot_wrap() {
    let (_dir, mut store, draft) = setup("a TWO b");
    let created = capture(&mut store, &draft, 2..5, "TWO");
    let conn = Connection::open(store.database_path()).unwrap();
    conn.execute(
        "UPDATE draft_comments SET status_version=?2 WHERE id=?1",
        rusqlite::params![created.comment_id.to_string(), i64::MAX],
    )
    .unwrap();
    let request = CommentStatusChange {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        comment_id: created.comment_id,
        expected_status_version: i64::MAX as u64,
        status: CommentStatus::Resolved,
    };
    assert!(store.set_comment_status(request).is_err());
    assert_eq!(
        store.draft_comments(draft.id).unwrap().comments[0]
            .comment
            .status_version,
        i64::MAX as u64
    );
}

#[test]
fn traced_checkpoint_without_comments_has_replayable_receipt() {
    let (_dir, mut store, draft) = setup("plain");
    let request = DraftWriteWithComments {
        op: Uuid::new_v4(),
        draft_id: draft.id,
        expected: draft.stamp,
        generation: 1,
        text: "plain!".into(),
        edits: EditTrace::Steps(vec![TextEdit {
            start: 5,
            end: 5,
            replacement: "!".into(),
        }]),
        checkpoint: true,
    };
    let saved = store.write_draft_with_comments(request.clone()).unwrap();
    assert!(saved.comments.is_empty());
    assert_eq!(store.write_draft_with_comments(request).unwrap(), saved);
}

#[test]
fn external_operation_api_cannot_claim_comment_kinds() {
    let (_dir, mut store, _draft) = setup("plain");
    for kind in ["comment.capture", "comment.status", "draft.write.comments"] {
        assert!(
            store
                .begin_operation(Uuid::new_v4(), kind, b"payload")
                .is_err()
        );
    }
}
