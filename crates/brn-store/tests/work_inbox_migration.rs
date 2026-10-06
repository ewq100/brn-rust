use brn_store::{
    WorkStore,
    work::{
        actions::{ActionData, ActionState},
        proposal_apply::{ApplyOutcome, ApprovalRequest},
        proposals::{ActionChange, ProposalDraft},
    },
};
use rusqlite::Connection;
use uuid::Uuid;

fn upgrade(restored: bool) {
    let parent = std::env::temp_dir().canonicalize().unwrap();
    let dir = tempfile::tempdir_in(parent).unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    store.set_setting("synthetic", "täpne\r\nλ").unwrap();
    store
        .put_unsaved_edit("recovery.md", [3; 32], "\u{feff}unfinished\r\nÕun")
        .unwrap();
    let unfinished = store.unsaved_edit("recovery.md").unwrap().unwrap();
    let action_id = Uuid::new_v4();
    let record = store
        .create_proposal(&ProposalDraft {
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: None,
            title: "Approved migration fixture".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create {
                id: action_id,
                data: ActionData {
                    title: "Send exact reply\r\nλ".into(),
                    description: "Waiting for the human".into(),
                    state: ActionState::Waiting,
                    owner: Some("Õun".into()),
                    related_person: None,
                    related_project: None,
                    sources: vec![],
                    thread: None,
                    due_on: Some("2028-02-29".into()),
                    follow_up_on: None,
                    dependencies: vec![],
                    parent: None,
                    follows_up: None,
                    priority: None,
                },
            }],
        })
        .unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    store
        .record_proposal_prepared(request.operation_id, &[])
        .unwrap();
    let receipt = store
        .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&[]))
        .unwrap();
    let proposal = store.proposal(record.draft.id).unwrap().unwrap();
    let before = store.action(action_id).unwrap().unwrap();
    let completion = brn_store::work::action_completion::CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(before),
    };
    let completed = store
        .complete_action_with(&completion, 0, |_| Ok(()))
        .unwrap();
    let action = store.action(action_id).unwrap().unwrap();
    drop(store);
    let db = dir.path().join("brn.sqlite");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch(
        "DROP TABLE inbox_original_operations; DROP TABLE inbox_actions; DROP TABLE inbox_processing; DROP TABLE inbox_items; PRAGMA user_version=11;",
    )
    .unwrap();
    let backup = dir.path().join("backups/brn-9999999999999.sqlite");
    if restored {
        conn.backup("main", &backup, None).unwrap();
    }
    drop(conn);
    let backup_bytes = restored.then(|| std::fs::read(&backup).unwrap());
    if restored {
        std::fs::write(&db, b"synthetic physical V11 damage").unwrap();
    }
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, restored.then_some(backup.clone()));
    assert_eq!(
        store.setting("synthetic").unwrap().as_deref(),
        Some("täpne\r\nλ")
    );
    assert_eq!(store.unsaved_edit("recovery.md").unwrap(), Some(unfinished));
    assert_eq!(store.action(action_id).unwrap(), Some(action));
    assert_eq!(
        store
            .complete_action_with(&completion, 0, |_| panic!(
                "migration replay must not publish again"
            ))
            .unwrap(),
        completed
    );
    assert_eq!(store.proposal(record.draft.id).unwrap(), Some(proposal));
    assert_eq!(
        store
            .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&[]))
            .unwrap(),
        receipt
    );
    let conn = Connection::open(&db).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM inbox_items", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    if let Some(bytes) = backup_bytes {
        assert_eq!(std::fs::read(backup).unwrap(), bytes);
    }
}

#[test]
fn additive_v11_upgrade_preserves_exact_action_completion_and_unfinished_work() {
    upgrade(false);
}

#[test]
fn validated_v11_backup_restores_then_upgrades_without_losing_action_completion() {
    upgrade(true);
}

#[test]
fn inbox_catalog_adds_v12_without_changing_existing_exact_work() {
    let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    store.set_setting("synthetic", "\u{feff}Õun\r\nλ").unwrap();
    store
        .put_unsaved_edit("retained.md", [3; 32], "\u{feff}exact\r\nλ")
        .unwrap();
    let work = store.unsaved_edit("retained.md").unwrap();
    drop(store);
    let conn = Connection::open(data.path().join("brn.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM inbox_items", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(conn);
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(
        store.setting("synthetic").unwrap().as_deref(),
        Some("\u{feff}Õun\r\nλ")
    );
    assert_eq!(store.unsaved_edit("retained.md").unwrap(), work);
}
