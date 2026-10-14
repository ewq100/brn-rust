use brn_threads_core::*;
use rusqlite::Connection;
#[test]
fn schema_four_requires_a_verified_external_snapshot_before_project_adoption() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut store = Store::open(&data).unwrap();
    let op = store.allocate_operation("old-note").unwrap();
    let owner = HostAuthority::owner("old owner");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "old note".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("Legacy", "Retained schema-four writing")),
                }],
            },
            &owner,
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner).unwrap() else {
        panic!()
    };
    drop(store);
    let db = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    db.execute_batch("DROP INDEX project_records; DROP INDEX context_link_from; DROP INDEX context_link_to; DROP INDEX context_import_source; DROP INDEX context_asset_source; PRAGMA user_version=4;")
        .unwrap();
    drop(db);
    assert!(Store::open_reader(&data).is_err());
    assert!(Store::open(&data).is_err());
    let conn = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    assert_eq!(
        conn.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        4
    );
    drop(conn);
    let backups = root.path().join("backups");
    std::fs::create_dir(&backups).unwrap();
    Store::upgrade(&data, &backups).unwrap();
    let backup = std::fs::read_dir(&backups)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let conn = Connection::open(&backup).unwrap();
    assert_eq!(
        conn.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        4
    );
    let before: String = conn
        .query_row("SELECT body FROM records", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Record>(&before).unwrap(),
        receipt.writes[0].after
    );
    drop(conn);
    let reopened = Store::open(&data).unwrap();
    assert_eq!(
        reopened
            .record(&receipt.writes[0].after.id)
            .unwrap()
            .unwrap(),
        receipt.writes[0].after
    );
    let restored = Store::restore(&backup, root.path().join("restored")).unwrap();
    assert_eq!(
        restored.record(&receipt.writes[0].after.id).unwrap(),
        reopened.record(&receipt.writes[0].after.id).unwrap()
    );
    assert!(Store::open_reader(&data).is_ok());
}
