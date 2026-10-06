use brn_store::{
    Error, WorkStore,
    work::{
        action_completion::{ActionCompletion, CompleteActionRequest},
        actions::{ActionData, ActionPriority, ActionRecord, ActionState},
        proposal_apply::{ApplyOutcome, ApprovalRequest},
        proposals::{ActionChange, ProposalDraft},
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::{cell::Cell, path::Path};
use uuid::Uuid;

fn fixture() -> (tempfile::TempDir, WorkStore) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    (dir, store)
}
fn approved(store: &mut WorkStore) -> ActionRecord {
    let id = Uuid::new_v4();
    let data = ActionData {
        title: "Exact λ\r\n".into(),
        description: "All original fields 日本語\r\n".into(),
        state: ActionState::Waiting,
        owner: Some("Anna Õun".into()),
        related_person: Some(Uuid::new_v4()),
        related_project: Some(Uuid::new_v4()),
        sources: vec![Uuid::new_v4()],
        thread: Some(Uuid::new_v4()),
        due_on: Some("2028-02-29".into()),
        follow_up_on: Some("2028-03-01".into()),
        dependencies: vec![Uuid::new_v4()],
        parent: Some(Uuid::new_v4()),
        follows_up: Some(Uuid::new_v4()),
        priority: Some(ActionPriority::High),
    };
    let review = store
        .create_proposal(&ProposalDraft {
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: None,
            title: "Approved action".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create { id, data }],
        })
        .unwrap();
    let operation_id = Uuid::new_v4();
    store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id,
            expected: review.stamp(),
        })
        .unwrap();
    store.record_proposal_prepared(operation_id, &[]).unwrap();
    store
        .finish_proposal_apply(operation_id, ApplyOutcome::Applied, Some(&[]))
        .unwrap();
    store.action(id).unwrap().unwrap()
}
fn complete_request(before: &ActionRecord) -> CompleteActionRequest {
    CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(before.clone()),
    }
}
fn put_action(dir: &Path, record: &ActionRecord) {
    record.validate().unwrap();
    let origin = serde_json::to_vec(&record.origin).unwrap();
    let bytes = serde_json::to_vec(record).unwrap();
    let state = serde_json::to_value(record.data.state).unwrap();
    Connection::open(dir.join("brn.sqlite")).unwrap().execute("INSERT OR REPLACE INTO actions(id,version,state,created_at_ms,creation_sha256,record_json,record_sha256) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![record.origin.id.to_string(),record.version as i64,state.as_str().unwrap(),record.origin.created_at_ms as i64,Sha256::digest(origin).as_slice(),&bytes,Sha256::digest(&bytes).as_slice()]).unwrap();
}
fn count(dir: &Path) -> i64 {
    Connection::open(dir.join("brn.sqlite"))
        .unwrap()
        .query_row("SELECT count(*) FROM action_completions", [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn exact_completion_preserves_every_field_and_replays_without_republishing_after_restart() {
    let (dir, mut store) = fixture();
    let before = approved(&mut store);
    let request = complete_request(&before);
    let publications = Cell::new(0);
    let completion = store
        .complete_action_with(&request, 0, |candidate| {
            candidate.validate()?;
            assert_eq!(candidate.request, request);
            publications.set(publications.get() + 1);
            Ok(())
        })
        .unwrap();
    let mut expected = before.clone();
    expected.version += 1;
    expected.data.state = ActionState::Completed;
    expected.waiting_since_ms = None;
    expected.completed_at_ms = Some(before.updated_at_ms);
    assert_eq!(completion.after, expected);
    assert_eq!(store.action(before.origin.id).unwrap(), Some(expected));
    assert_eq!(
        store.action_completion_for(&request).unwrap(),
        Some(completion.clone())
    );
    assert_eq!(
        store
            .complete_action_with(&request, u64::MAX, |_| {
                publications.set(99);
                Ok(())
            })
            .unwrap(),
        completion
    );
    assert_eq!(publications.get(), 1);
    assert_eq!(count(dir.path()), 1);
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store
            .complete_action_with(&request, u64::MAX, |_| panic!("replay republished"))
            .unwrap(),
        completion
    );
}

#[test]
fn changed_operation_payload_and_all_fresh_preconditions_refuse_before_publication() {
    let (dir, mut store) = fixture();
    let before = approved(&mut store);
    let request = complete_request(&before);
    store
        .complete_action_with(&request, before.updated_at_ms + 10, |_| Ok(()))
        .unwrap();
    let mut changed = request.clone();
    changed.before.data.description.push('λ');
    changed.before.version += 1;
    changed.before.updated_at_ms += 1;
    assert!(matches!(
        store.action_completion_for(&changed),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        store.complete_action_with(&changed, 0, |_| panic!("conflict published")),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(count(dir.path()), 1);
    for case in ["missing", "drift", "completed", "maximum", "clock"] {
        let (dir, mut fresh_store) = fixture();
        let mut fresh = approved(&mut fresh_store);
        let mut at_ms = 0;
        match case {
            "missing" => fresh.origin.id = Uuid::new_v4(),
            "drift" => {
                fresh.version += 1;
                fresh.updated_at_ms += 1;
                fresh.data.description.push('λ');
            }
            "completed" => {
                fresh = fresh_store
                    .complete_action_with(&complete_request(&fresh), 0, |_| Ok(()))
                    .unwrap()
                    .after;
            }
            "maximum" => {
                fresh.version = i64::MAX as u64;
                put_action(dir.path(), &fresh);
            }
            "clock" => at_ms = u64::MAX,
            _ => unreachable!(),
        }
        fresh.validate().unwrap();
        let retained = fresh_store.action(fresh.origin.id).unwrap();
        let receipts = count(dir.path());
        assert!(
            fresh_store
                .complete_action_with(&complete_request(&fresh), at_ms, |_| panic!(
                    "invalid {case} request published"
                ))
                .is_err()
        );
        assert_eq!(fresh_store.action(fresh.origin.id).unwrap(), retained);
        assert_eq!(count(dir.path()), receipts);
    }
}

#[test]
fn same_version_full_record_drift_and_publisher_failure_leave_no_receipt_or_mutation() {
    let (dir, mut store) = fixture();
    let mut before = approved(&mut store);
    before.version += 1;
    before.updated_at_ms += 1;
    put_action(dir.path(), &before);
    let mut stale = before.clone();
    stale.data.owner = Some("Another owner".into());
    assert!(
        store
            .complete_action_with(&complete_request(&stale), 0, |_| panic!(
                "same-version drift published"
            ))
            .is_err()
    );
    let request = complete_request(&before);
    let calls = Cell::new(0);
    assert!(
        store
            .complete_action_with(&request, 0, |_| {
                calls.set(calls.get() + 1);
                Err(Error::StateChanged("synthetic publication refusal".into()))
            })
            .is_err()
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(store.action(before.origin.id).unwrap(), Some(before));
    assert!(store.action_completion_for(&request).unwrap().is_none());
    assert_eq!(count(dir.path()), 0);
}

#[test]
fn publisher_holds_sqlite_write_exclusion_and_success_commits_both_record_and_receipt() {
    let (dir, mut store) = fixture();
    let before = approved(&mut store);
    let contender = Connection::open(dir.path().join("brn.sqlite")).unwrap();
    contender.busy_timeout(std::time::Duration::ZERO).unwrap();
    let completion = store
        .complete_action_with(&complete_request(&before), 0, |_| {
            let error = contender
                .execute(
                    "UPDATE actions SET version=version WHERE id=?1",
                    [before.origin.id.to_string()],
                )
                .unwrap_err();
            assert!(matches!(
                error.sqlite_error_code(),
                Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
            ));
            Ok(())
        })
        .unwrap();
    assert_eq!(
        store.action(before.origin.id).unwrap(),
        Some(completion.after)
    );
    assert_eq!(count(dir.path()), 1);
}

#[test]
fn checked_snapshot_restores_older_and_fresh_work_and_preserves_newer_completed_work() {
    let (_, mut source) = fixture();
    let before = approved(&mut source);
    let request = complete_request(&before);
    let completion = source
        .complete_action_with(&request, before.updated_at_ms + 10, |_| Ok(()))
        .unwrap();
    for with_before in [true, false] {
        let (dir, mut target) = fixture();
        if with_before {
            put_action(dir.path(), &before);
        }
        assert_eq!(
            target.restore_action_completion(&completion).unwrap(),
            completion
        );
        assert_eq!(
            target.restore_action_completion(&completion).unwrap(),
            completion
        );
        assert_eq!(
            target.action(before.origin.id).unwrap(),
            Some(completion.after.clone())
        );
        assert_eq!(
            target.action_completion_for(&request).unwrap(),
            Some(completion.clone())
        );
        let mut newer = completion.after.clone();
        newer.version += 1;
        newer.updated_at_ms += 1;
        newer.data.description.push('λ');
        put_action(dir.path(), &newer);
        assert_eq!(
            target.restore_action_completion(&completion).unwrap(),
            completion
        );
        assert_eq!(target.action(before.origin.id).unwrap(), Some(newer));
        drop(target);
        let (target, _) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(
            target.action_completion_for(&request).unwrap(),
            Some(completion.clone())
        );
    }
}

#[test]
fn restore_refuses_equal_forks_origin_changes_newer_unfinished_and_older_completed_atomically() {
    let (_, mut source) = fixture();
    let before = approved(&mut source);
    let completion = source
        .complete_action_with(
            &complete_request(&before),
            before.updated_at_ms + 10,
            |_| Ok(()),
        )
        .unwrap();
    let mut equal_fork = completion.after.clone();
    equal_fork.data.description.push('λ');
    let mut other_origin = completion.after.clone();
    other_origin.origin.proposal.id = Uuid::new_v4();
    let mut unfinished = before.clone();
    unfinished.version = completion.after.version + 1;
    unfinished.updated_at_ms += 20;
    let mut before_fork = before.clone();
    before_fork.data.description.push('λ');
    before_fork.version += 1;
    before_fork.updated_at_ms += 1;
    // Older Completed can only be a valid revision after its initial origin.
    let mut later_before = before.clone();
    later_before.version = 3;
    later_before.updated_at_ms += 30;
    let later_request = complete_request(&later_before);
    let mut later_after = later_before.clone();
    later_after.version += 1;
    later_after.data.state = ActionState::Completed;
    later_after.waiting_since_ms = None;
    later_after.completed_at_ms = Some(later_after.updated_at_ms);
    let later = ActionCompletion {
        request: later_request,
        after: later_after,
    };
    let mut equal_before_fork = later.request.before.as_ref().clone();
    equal_before_fork.data.owner = Some("Equal before fork".into());
    for (current, incoming) in [
        (equal_fork, &completion),
        (other_origin, &completion),
        (unfinished, &completion),
        (before_fork, &completion),
        (completion.after.clone(), &later),
        (equal_before_fork, &later),
    ] {
        let (dir, mut target) = fixture();
        put_action(dir.path(), &current);
        assert!(target.restore_action_completion(incoming).is_err());
        assert_eq!(target.action(before.origin.id).unwrap(), Some(current));
        assert_eq!(count(dir.path()), 0);
    }
}

#[test]
fn one_action_cannot_receive_a_second_operation_and_exact_receipt_forks_refuse() {
    let (dir, mut store) = fixture();
    let before = approved(&mut store);
    let completion = store
        .complete_action_with(
            &complete_request(&before),
            before.updated_at_ms + 10,
            |_| Ok(()),
        )
        .unwrap();
    let mut second = completion.clone();
    second.request.operation_id = Uuid::new_v4();
    assert!(store.restore_action_completion(&second).is_err());
    let mut fork = completion.clone();
    fork.after.updated_at_ms += 1;
    fork.after.completed_at_ms = Some(fork.after.updated_at_ms);
    assert!(store.restore_action_completion(&fork).is_err());
    assert_eq!(
        store.action(before.origin.id).unwrap(),
        Some(completion.after)
    );
    assert_eq!(count(dir.path()), 1);
}

#[test]
fn snapshot_validation_rejects_mutated_body_origin_revision_and_terminal_timestamps() {
    let (_, mut store) = fixture();
    let before = approved(&mut store);
    let completion = store
        .complete_action_with(
            &complete_request(&before),
            before.updated_at_ms + 10,
            |_| Ok(()),
        )
        .unwrap();
    for field in [
        "data",
        "origin",
        "version",
        "waiting",
        "completed",
        "clock",
        "operation",
        "before",
    ] {
        let mut bad = completion.clone();
        match field {
            "data" => bad.after.data.sources.push(Uuid::new_v4()),
            "origin" => bad.after.origin.proposal.id = Uuid::new_v4(),
            "version" => bad.after.version += 1,
            "waiting" => bad.after.waiting_since_ms = Some(bad.after.updated_at_ms),
            "completed" => bad.after.completed_at_ms = Some(bad.after.updated_at_ms - 1),
            "clock" => {
                bad.after.updated_at_ms = before.updated_at_ms - 1;
                bad.after.completed_at_ms = Some(bad.after.updated_at_ms);
            }
            "operation" => bad.request.operation_id = Uuid::nil(),
            "before" => *bad.request.before = completion.after.clone(),
            _ => unreachable!(),
        }
        assert!(bad.validate().is_err(), "accepted mutated {field}");
        let (dir, mut fresh) = fixture();
        assert!(fresh.restore_action_completion(&bad).is_err());
        assert!(fresh.action(before.origin.id).unwrap().is_none());
        assert_eq!(count(dir.path()), 0);
    }
    let mut value = serde_json::to_value(&completion).unwrap();
    value["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ActionCompletion>(value).is_err());
    let mut value = serde_json::to_value(&completion.request).unwrap();
    value["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<CompleteActionRequest>(value).is_err());
}

#[test]
fn startup_refuses_readable_receipt_corruption_without_replacing_current_work() {
    for problem in [
        "request-hash",
        "completion-hash",
        "operation-id",
        "action-id",
        "indexed-action-binding",
        "indexed-operation-binding",
        "oversize",
        "semantics",
        "missing-action",
        "unfinished-action",
        "equal-fork",
        "origin-fork",
        "trigger",
        "shape",
        "checked-shape",
        "missing-index",
    ] {
        let (dir, mut store) = fixture();
        let before = approved(&mut store);
        let completion = store
            .complete_action_with(
                &complete_request(&before),
                before.updated_at_ms + 10,
                |_| Ok(()),
            )
            .unwrap();
        drop(store);
        let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
        match problem {
            "request-hash" => {
                raw.execute(
                    "UPDATE action_completions SET request_sha256=zeroblob(32)",
                    [],
                )
                .unwrap();
            }
            "completion-hash" => {
                raw.execute(
                    "UPDATE action_completions SET completion_sha256=zeroblob(32)",
                    [],
                )
                .unwrap();
            }
            "operation-id" => {
                raw.execute(
                    "UPDATE action_completions SET operation_id=replace(operation_id,'-','')",
                    [],
                )
                .unwrap();
            }
            "action-id" => {
                raw.execute(
                    "UPDATE action_completions SET action_id=replace(action_id,'-','')",
                    [],
                )
                .unwrap();
            }
            "indexed-action-binding" => {
                raw.execute(
                    "UPDATE action_completions SET action_id=?1",
                    [Uuid::new_v4().to_string()],
                )
                .unwrap();
            }
            "indexed-operation-binding" => {
                raw.execute(
                    "UPDATE action_completions SET operation_id=?1",
                    [Uuid::new_v4().to_string()],
                )
                .unwrap();
            }
            "oversize" => {
                raw.execute(
                    "UPDATE action_completions SET completion_json=zeroblob(4194304)",
                    [],
                )
                .unwrap();
            }
            "semantics" => {
                let mut bad = completion.clone();
                bad.after.data.title.push('λ');
                let bytes = serde_json::to_vec(&bad).unwrap();
                raw.execute(
                    "UPDATE action_completions SET completion_json=?1,completion_sha256=?2",
                    params![&bytes, Sha256::digest(&bytes).as_slice()],
                )
                .unwrap();
            }
            "missing-action" => {
                raw.execute("DELETE FROM actions", []).unwrap();
            }
            "unfinished-action" => put_action(dir.path(), &before),
            "equal-fork" => {
                let mut bad = completion.after.clone();
                bad.data.owner = Some("Fork".into());
                put_action(dir.path(), &bad);
            }
            "origin-fork" => {
                let mut bad = completion.after.clone();
                bad.origin.proposal.id = Uuid::new_v4();
                put_action(dir.path(), &bad);
            }
            "trigger" => {
                raw.execute_batch("CREATE TRIGGER unexpected_completion AFTER INSERT ON action_completions BEGIN SELECT 1; END;").unwrap();
            }
            "shape" => {
                raw.execute_batch("ALTER TABLE action_completions ADD COLUMN unexpected TEXT;")
                    .unwrap();
            }
            "checked-shape" | "missing-index" => {
                let action_binding = if problem == "missing-index" {
                    "action_id TEXT NOT NULL,"
                } else {
                    "action_id TEXT NOT NULL UNIQUE,"
                };
                let digest_binding = if problem == "checked-shape" {
                    "completion_sha256 BLOB NOT NULL CHECK(length(completion_sha256)=32)"
                } else {
                    "completion_sha256 BLOB NOT NULL"
                };
                raw.execute_batch(&format!("ALTER TABLE action_completions RENAME TO old_completions; CREATE TABLE action_completions(operation_id TEXT PRIMARY KEY,{action_binding} request_sha256 BLOB NOT NULL,completion_json BLOB NOT NULL,{digest_binding}); INSERT INTO action_completions SELECT * FROM old_completions; DROP TABLE old_completions;")).unwrap();
            }
            _ => unreachable!(),
        }
        drop(raw);
        assert!(WorkStore::open(dir.path()).is_err(), "accepted {problem}");
        assert!(dir.path().join("brn.sqlite").exists());
        assert!(
            !std::fs::read_dir(dir.path()).unwrap().any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("brn.sqlite.corrupt-")),
            "{problem} silently replaced current semantic work"
        );
    }
}

#[test]
fn physically_corrupt_main_skips_a_semantically_invalid_completion_backup() {
    let (dir, mut store) = fixture();
    let older = std::fs::read_dir(dir.path().join("backups"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let before = approved(&mut store);
    store
        .complete_action_with(&complete_request(&before), 0, |_| Ok(()))
        .unwrap();
    drop(store);
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    let latest = report.backup;
    drop(store);
    Connection::open(&latest)
        .unwrap()
        .execute(
            "UPDATE action_completions SET completion_sha256=zeroblob(32)",
            [],
        )
        .unwrap();
    std::fs::write(
        dir.path().join("brn.sqlite"),
        b"synthetic physical SQLite corruption",
    )
    .unwrap();
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, Some(older));
    assert!(store.action(before.origin.id).unwrap().is_none());
    assert_eq!(count(dir.path()), 0);
}
