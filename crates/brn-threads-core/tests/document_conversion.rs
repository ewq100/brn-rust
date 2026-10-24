use brn_threads_core::*;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
fn version(path: &Path) -> i64 {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}
fn canonical(path: &Path) -> Vec<(String, Vec<Vec<rusqlite::types::Value>>)> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tables=conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap().query_map([],|row|row.get::<_,String>(0)).unwrap().collect::<std::result::Result<Vec<_>,_>>().unwrap();
    tables
        .into_iter()
        .map(|table| {
            let mut stmt = conn
                .prepare(&format!("SELECT * FROM \"{table}\" ORDER BY rowid"))
                .unwrap();
            let count = stmt.column_count();
            let rows = stmt
                .query_map([], |row| {
                    (0..count)
                        .map(|i| row.get(i))
                        .collect::<std::result::Result<Vec<rusqlite::types::Value>, _>>()
                })
                .unwrap()
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap();
            (table, rows)
        })
        .collect()
}
#[test]
fn schema9_snapshot_stays9_and_only_unpublished_restore_copy_upgrades() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut store = Store::open(&data).unwrap();
    let op = store
        .allocate_operation("representative-canonical-note")
        .unwrap();
    let owner = HostAuthority::owner("schema compatibility");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "preserve canonical rows".into(),
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("Unicode Ω", "exact\r\nsource")),
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    store.apply(&op, &owner).unwrap();
    drop(store);
    let db = data.join(DATABASE_FILENAME);
    Connection::open(&db)
        .unwrap()
        .pragma_update(None, "user_version", 9)
        .unwrap();
    if let Some(actual) = std::env::var_os("BRN_SCHEMA9_SNAPSHOT") {
        std::fs::copy(actual, &db).unwrap();
    }
    let rows = canonical(&db);
    let bytes = std::fs::read(&db).unwrap();
    assert!(matches!(
        Store::open_existing_without_upgrade(&data),
        Err(Error::UpgradeRequired {
            schema_found: 9,
            schema_required: 11
        })
    ));
    assert_eq!(std::fs::read(&db).unwrap(), bytes);
    let snapshot = root.path().join("schema9.sqlite3");
    std::fs::copy(&db, &snapshot).unwrap();
    let old_bytes = std::fs::read(&snapshot).unwrap();
    let restored = Store::restore(&snapshot, root.path().join("restored")).unwrap();
    assert_eq!(version(&restored.directory().join(DATABASE_FILENAME)), 11);
    assert_eq!(
        canonical(&restored.directory().join(DATABASE_FILENAME)),
        rows
    );
    assert_eq!(std::fs::read(&snapshot).unwrap(), old_bytes);
    assert_eq!(version(&snapshot), 9);
    drop(restored);
    let backups = root.path().join("backups");
    std::fs::create_dir(&backups).unwrap();
    Store::upgrade(&data, &backups).unwrap();
    assert_eq!(version(&db), 11);
    assert_eq!(canonical(&db), rows);
    if let Some(binary) = std::env::var_os("BRN_SCHEMA9_BINARY") {
        let before = std::fs::read(&db).unwrap();
        let output = std::process::Command::new(binary)
            .args(["--json", "--data-dir"])
            .arg(&data)
            .arg("settings")
            .output()
            .unwrap();
        assert!(!output.status.success());
        let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(envelope["ok"], false);
        assert_eq!(std::fs::read(&db).unwrap(), before);
        assert_eq!(canonical(&db), rows);
        if let Some(path) = std::env::var_os("BRN_TASK2_CAPTURES") {
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(
                std::path::PathBuf::from(path).join("old9-refuses10.json"),
                &output.stdout,
            )
            .unwrap();
        }
    }

    let backup = std::fs::read_dir(&backups)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(version(&backup), 9);
    assert_eq!(canonical(&backup), rows);
    let restored = Store::restore(&backup, root.path().join("restored-preupgrade")).unwrap();
    assert_eq!(version(&backup), 9);
    assert_eq!(
        canonical(&restored.directory().join(DATABASE_FILENAME)),
        rows
    );
}
