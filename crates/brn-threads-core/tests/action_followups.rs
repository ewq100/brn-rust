use brn_threads_core::*;
use rusqlite::Connection;

fn legacy_action() -> Action {
    serde_json::from_str(r#"{"description":"Ask supplier after approval","state":"Suggested","internal":false,"evidence":null,"due":null}"#).unwrap()
}

#[test]
fn legacy_actions_keep_unknown_facts_and_exact_serialization() {
    let action = legacy_action();
    assert_eq!(
        (&action.owner, &action.waiting_on, &action.follow_up),
        (&None, &None, &None)
    );
    assert_eq!(
        serde_json::to_string(&action).unwrap(),
        r#"{"description":"Ask supplier after approval","state":"Suggested","internal":false,"evidence":null,"due":null}"#
    );
}

#[test]
fn follow_up_fields_reject_invalid_dates_and_labels_at_checked_prepare() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let oversized = "õ".repeat(257);
    for (index, field, value) in [
        (0, "follow_up", "2027-02-29"),
        (1, "follow_up", "2028-2-29"),
        (2, "follow_up", "next Friday"),
        (3, "follow_up", "2028-02-29T00:00:00Z"),
        (4, "owner", " "),
        (5, "waiting_on", "Supplier\nApproval"),
        (6, "owner", oversized.as_str()),
        (7, "waiting_on", oversized.as_str()),
    ] {
        let mut value_json = serde_json::to_value(legacy_action()).unwrap();
        value_json[field] = value.into();
        let action: Action = serde_json::from_value(value_json).unwrap();
        let operation = store
            .allocate_operation(&format!("invalid-{index}"))
            .unwrap();
        let result = store.prepare(
            &operation,
            &ChangeRequest {
                reason: "Invalid fixture".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: operation.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Action(action),
                }],
            },
        );
        assert!(
            matches!(result, Err(Error::Invalid(_))),
            "{field}={value:?}"
        );
        assert!(store.record(&operation.creation_id(0)).unwrap().is_none());
    }
    let mut action = legacy_action();
    action.owner = Some("Märta".into());
    action.waiting_on = Some("Supplier approval".into());
    action.follow_up = Some("2028-02-29".into());
    let operation = store.allocate_operation("valid leap day").unwrap();
    store
        .prepare(
            &operation,
            &ChangeRequest {
                reason: "Supported follow-up".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: operation.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Action(action),
                }],
            },
        )
        .unwrap();
}

#[test]
fn schema10_actions_upgrade_only_after_external_snapshot_without_rewriting_records() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut store = Store::open(&data).unwrap();
    let operation = store.allocate_operation("legacy action").unwrap();
    let id = operation.creation_id(0);
    let request = ChangeRequest {
        reason: "Old Action".into(),
        inputs: vec![],
        writes: vec![Put {
            id: id.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Action(legacy_action()),
        }],
    };
    let owner = HostAuthority::owner("fixture");
    store.prepare_owner(&operation, &request, &owner).unwrap();
    assert!(matches!(
        store.apply(&operation, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let expected = store.record(&id).unwrap().unwrap();
    drop(store);
    let database = data.join(DATABASE_FILENAME);
    let conn = Connection::open(&database).unwrap();
    conn.pragma_update(None, "user_version", 10).unwrap();
    let body: String = conn
        .query_row("SELECT body FROM records WHERE id=?1", [&id], |r| r.get(0))
        .unwrap();
    let revision: String = conn
        .query_row("SELECT body FROM revisions WHERE id=?1", [&id], |r| {
            r.get(0)
        })
        .unwrap();
    let saved_request: String = conn
        .query_row(
            "SELECT request FROM operations WHERE id=?1",
            [operation.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    drop(conn);
    let bytes = std::fs::read(&database).unwrap();
    assert!(matches!(
        Store::open_existing_without_upgrade(&data),
        Err(Error::UpgradeRequired {
            schema_found: 10,
            schema_required: 12
        })
    ));
    assert_eq!(std::fs::read(&database).unwrap(), bytes);
    let backup = root.path().join("backups");
    std::fs::create_dir(&backup).unwrap();
    Store::upgrade(&data, &backup).unwrap();
    let store = Store::open_existing_without_upgrade(&data).unwrap();
    assert_eq!(store.record(&id).unwrap().unwrap(), expected);
    let conn = Connection::open(&database).unwrap();
    assert_eq!(
        conn.query_row("SELECT body FROM records WHERE id=?1", [&id], |r| r
            .get::<_, String>(0))
            .unwrap(),
        body
    );
    assert_eq!(
        conn.query_row("SELECT body FROM revisions WHERE id=?1", [&id], |r| r
            .get::<_, String>(0))
            .unwrap(),
        revision
    );
    assert_eq!(
        conn.query_row(
            "SELECT request FROM operations WHERE id=?1",
            [operation.as_str()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        saved_request
    );
    let snapshots: Vec<_> = std::fs::read_dir(&backup)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(snapshots.len(), 1);
    let snapshot = Connection::open(&snapshots[0]).unwrap();
    assert_eq!(
        snapshot
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        10
    );
    assert_eq!(
        snapshot
            .query_row("SELECT body FROM records WHERE id=?1", [&id], |r| r
                .get::<_, String>(0))
            .unwrap(),
        body
    );
}
