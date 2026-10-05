use brn_store::{Error, WorkStore, work::inbox::*};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::MetadataExt, path::Path};
use uuid::Uuid;

const ORIGINAL: &str = "\u{feff}From: synthetic@example.invalid\r\nSubject: Täpne Õun\r\n\r\nOriginal source λ 日本語\r\n";
fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn capture(data: &Path, kind: InboxKind) -> InboxCapture {
    let id = Uuid::new_v4();
    let directory = data.join("original-copies");
    std::fs::create_dir_all(&directory).unwrap();
    let directory = directory.canonicalize().unwrap();
    let file = directory.join(format!("{id}.txt"));
    std::fs::write(&file, ORIGINAL).unwrap();
    let parent = std::fs::metadata(&directory).unwrap();
    let meta = std::fs::metadata(&file).unwrap();
    InboxCapture {
        id,
        kind,
        title: "Täpsed küsimused 日本語".into(),
        original_name: Some("supplied \"copy\".eml".into()),
        copy: InboxCopy {
            directory,
            directory_device: parent.dev(),
            directory_inode: parent.ino(),
            file_device: meta.dev(),
            file_inode: meta.ino(),
            byte_len: meta.len(),
            sha256: digest(ORIGINAL.as_bytes()),
        },
    }
}
fn raw(data: &Path) -> Connection {
    Connection::open(data.join("brn.sqlite")).unwrap()
}
fn rewrite(conn: &Connection, item: &InboxItem) {
    let bytes = serde_json::to_vec(item).unwrap();
    conn.execute(
        "UPDATE inbox_items SET received_at_ms=?2,record_json=?3,record_sha256=?4 WHERE id=?1",
        params![
            item.capture.id.to_string(),
            item.received_at_ms as i64,
            bytes,
            digest(&bytes).as_slice()
        ],
    )
    .unwrap();
}
fn backups(data: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = std::fs::read_dir(data.join("backups"))
        .unwrap()
        .map(|r| {
            let e = r.unwrap();
            (
                e.file_name().into_string().unwrap(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

#[test]
fn full_original_metadata_replay_and_restart_never_write_payload_or_refresh_time() {
    let data = fixture();
    let input = capture(data.path(), InboxKind::Email);
    let original_path = input.copy.directory.join(input.copy_name());
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut item = store.capture_inbox(&input).unwrap();
    assert_eq!(item.capture, input);
    item.received_at_ms = 1;
    rewrite(&raw(data.path()), &item);
    assert_eq!(store.capture_inbox(&input).unwrap(), item);
    for field in 0..7 {
        let mut different = input.clone();
        match field {
            0 => different.title.push('λ'),
            1 => different.kind = InboxKind::Text,
            2 => different.original_name = None,
            3 => different.copy.directory = "/synthetic/another-intake".into(),
            4 => different.copy.file_inode += 1,
            5 => different.copy.sha256[0] ^= 1,
            _ => different.copy.byte_len += 1,
        }
        assert!(matches!(
            store.capture_inbox(&different),
            Err(Error::OperationConflict(_))
        ));
        assert_eq!(store.inbox_item(input.id).unwrap(), Some(item.clone()));
    }
    let bytes: Vec<u8> = raw(data.path())
        .query_row("SELECT record_json FROM inbox_items", [], |r| r.get(0))
        .unwrap();
    assert!(!String::from_utf8(bytes).unwrap().contains(ORIGINAL));
    assert_eq!(std::fs::read(&original_path).unwrap(), ORIGINAL.as_bytes());
    drop(store);
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.inbox_item(input.id).unwrap(), Some(item.clone()));
    assert_eq!(
        store
            .inbox_items(&InboxListRequest::default())
            .unwrap()
            .entries,
        vec![item.clone()]
    );
    // Catalog proof is not a claim that the original is still present. Workflow
    // must freshly inspect copies; storage deliberately performs no file I/O.
    std::fs::remove_file(original_path).unwrap();
    assert_eq!(store.inbox_item(input.id).unwrap(), Some(item));
}

#[test]
fn fifo_pages_are_bounded_and_tied_uuid_order_resumes_with_distinct_equal_content_copies() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut expected = vec![];
    for kind in [
        InboxKind::Text,
        InboxKind::Markdown,
        InboxKind::Email,
        InboxKind::Teams,
        InboxKind::Text,
    ] {
        let mut item = store.capture_inbox(&capture(data.path(), kind)).unwrap();
        item.received_at_ms = 7;
        rewrite(&raw(data.path()), &item);
        expected.push(item);
    }
    expected.sort_by_key(|r| r.capture.id);
    let mut actual = vec![];
    let mut after = None;
    loop {
        let page = store
            .inbox_items(&InboxListRequest { limit: 2, after })
            .unwrap();
        assert_eq!(page.total_count, 5);
        assert!(page.entries.len() <= 2);
        if let Some(next) = page.next_after {
            assert_eq!(Some(next), page.entries.last().map(|r| r.capture.id));
        }
        after = page.next_after;
        actual.extend(page.entries);
        if after.is_none() {
            break;
        }
    }
    assert_eq!(actual, expected);
    assert!(
        actual
            .windows(2)
            .all(|pair| pair[0].capture.copy.file_inode != pair[1].capture.copy.file_inode)
    );
    assert!(
        actual
            .iter()
            .all(|r| r.capture.copy.sha256 == digest(ORIGINAL.as_bytes()))
    );
    assert!(
        store
            .inbox_items(&InboxListRequest {
                limit: 100,
                after: Some(expected.last().unwrap().capture.id)
            })
            .unwrap()
            .entries
            .is_empty()
    );
    for request in [
        InboxListRequest {
            limit: 0,
            after: None,
        },
        InboxListRequest {
            limit: 101,
            after: None,
        },
        InboxListRequest {
            limit: usize::MAX,
            after: None,
        },
        InboxListRequest {
            limit: 1,
            after: Some(Uuid::nil()),
        },
    ] {
        assert!(matches!(
            store.inbox_items(&request),
            Err(Error::Invalid(_))
        ));
    }
    assert!(matches!(
        store.inbox_items(&InboxListRequest {
            limit: 1,
            after: Some(Uuid::new_v4())
        }),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        store.inbox_item(Uuid::nil()),
        Err(Error::Invalid(_))
    ));
    assert_eq!(store.inbox_item(Uuid::new_v4()).unwrap(), None);
}

#[test]
fn whole_utf8_labels_proofs_and_empty_copy_bounds_refuse_before_any_catalog_insert() {
    let data = fixture();
    let original = capture(data.path(), InboxKind::Text);
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    for mutation in 0..18 {
        let mut bad = original.clone();
        bad.id = Uuid::new_v4();
        match mutation {
            0 => bad.id = Uuid::nil(),
            1 => bad.title = " \t".into(),
            2 => bad.title = "λ".repeat(257),
            3 => bad.title.push('\n'),
            4 => bad.original_name = Some("".into()),
            5 => bad.original_name = Some("λ".repeat(257)),
            6 => bad.original_name = Some("a\0b".into()),
            7 => bad.copy.directory = "relative/inbox".into(),
            8 => bad.copy.directory = "/synthetic/../other".into(),
            9 => bad.copy.directory = "/synthetic/./inbox".into(),
            10 => bad.copy.directory = "/synthetic//inbox".into(),
            11 => bad.copy.directory = format!("/{}", "λ".repeat(2048)).into(),
            12 => bad.copy.directory_inode = 0,
            13 => bad.copy.file_inode = 0,
            14 => bad.copy.file_device = bad.copy.directory_device + 1,
            15 => bad.copy.file_inode = bad.copy.directory_inode,
            16 => bad.copy.byte_len = 1024 * 1024 + 1,
            _ => bad.copy.byte_len = 0,
        }
        assert!(
            matches!(store.capture_inbox(&bad), Err(Error::Invalid(_))),
            "mutation {mutation}"
        );
    }
    assert_eq!(
        store
            .inbox_items(&InboxListRequest::default())
            .unwrap()
            .total_count,
        0
    );
    let mut maximum = original.clone();
    maximum.title = "λ".repeat(256);
    maximum.original_name = Some("q".repeat(512));
    maximum.copy.byte_len = 1024 * 1024;
    assert_eq!(store.capture_inbox(&maximum).unwrap().capture, maximum);
    let mut empty = original.clone();
    empty.id = Uuid::new_v4();
    empty.copy.byte_len = 0;
    empty.copy.sha256 = digest(&[]);
    assert_eq!(store.capture_inbox(&empty).unwrap().capture, empty);
    // The user label is never a destination, even if it looks like one.
    let mut labeled = original;
    labeled.id = Uuid::new_v4();
    labeled.original_name = Some("../../label only.eml".into());
    assert_eq!(labeled.copy_name(), format!("{}.txt", labeled.id));
    store.capture_inbox(&labeled).unwrap();
}

#[test]
fn readable_record_damage_refuses_reads_and_startup_without_replacing_or_backing_up() {
    for damage in 0..11 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let item = store
            .capture_inbox(&capture(data.path(), InboxKind::Teams))
            .unwrap();
        let conn = raw(data.path());
        match damage {
            0 => {
                conn.execute(
                    "UPDATE inbox_items SET record_sha256=?1",
                    [[0u8; 32].as_slice()],
                )
                .unwrap();
            }
            1 => {
                conn.execute(
                    "UPDATE inbox_items SET capture_sha256=?1",
                    [[0u8; 32].as_slice()],
                )
                .unwrap();
            }
            2 => {
                conn.execute("UPDATE inbox_items SET id=?1", [Uuid::new_v4().to_string()])
                    .unwrap();
            }
            3 => {
                conn.execute("UPDATE inbox_items SET received_at_ms=received_at_ms+1", [])
                    .unwrap();
            }
            4..=8 => {
                let mut value = serde_json::to_value(&item).unwrap();
                match damage {
                    4 => {
                        value["capture"]["copy"]["write_allowed"] = true.into();
                    }
                    5 => {
                        value["capture"]
                            .as_object_mut()
                            .unwrap()
                            .remove("original_name");
                    }
                    6 => {
                        value["capture"]["copy"]["byte_len"] = (1024 * 1024 + 1).into();
                    }
                    7 => {
                        value["received_at_ms"] = u64::MAX.into();
                    }
                    _ => {
                        value["capture"]["kind"] = "document".into();
                    }
                }
                let bytes = serde_json::to_vec(&value).unwrap();
                conn.execute(
                    "UPDATE inbox_items SET record_json=?1,record_sha256=?2",
                    params![bytes, digest(&bytes).as_slice()],
                )
                .unwrap();
            }
            9 => {
                conn.execute("UPDATE inbox_items SET record_json=zeroblob(40961)", [])
                    .unwrap();
            }
            _ => {
                conn.pragma_update(None, "ignore_check_constraints", true)
                    .unwrap();
                conn.execute("UPDATE inbox_items SET received_at_ms=-1", [])
                    .unwrap();
            }
        }
        assert!(
            store.inbox_items(&InboxListRequest::default()).is_err(),
            "read damage {damage}"
        );
        let raw_id: String = conn
            .query_row("SELECT id FROM inbox_items", [], |r| r.get(0))
            .unwrap();
        assert!(
            store.inbox_item(Uuid::parse_str(&raw_id).unwrap()).is_err(),
            "item damage {damage}"
        );
        drop(store);
        // Fully checkpoint so refusal can be compared to exact database bytes.
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        drop(conn);
        let before = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        let backup_before = backups(data.path());
        let error = match WorkStore::open(data.path()) {
            Ok(_) => panic!("damage {damage} accepted"),
            Err(e) => e,
        };
        assert!(
            !matches!(error, Error::WorkspaceBusy(_)),
            "unexpected contention"
        );
        assert_eq!(
            std::fs::read(data.path().join("brn.sqlite")).unwrap(),
            before,
            "damage {damage}"
        );
        assert_eq!(backups(data.path()), backup_before, "damage {damage}");
        assert!(!std::fs::read_dir(data.path()).unwrap().any(|r| {
            r.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("brn.sqlite.corrupt-")
        }));
        assert_eq!(
            std::fs::read(item.capture.copy.directory.join(item.capture.copy_name())).unwrap(),
            ORIGINAL.as_bytes()
        );
    }
}

#[test]
fn unexpected_owned_schema_objects_and_missing_index_refuse_before_backup() {
    for sql in [
        "DROP INDEX inbox_items_page",
        "CREATE TRIGGER unexpected_inbox_trigger AFTER INSERT ON inbox_items BEGIN DELETE FROM inbox_items; END;",
        "CREATE INDEX unexpected_inbox_index ON inbox_items(id)",
        "ALTER TABLE inbox_items ADD COLUMN undocumented TEXT",
        "DROP TABLE inbox_items",
    ] {
        let data = fixture();
        let (store, _) = WorkStore::open(data.path()).unwrap();
        drop(store);
        let conn = raw(data.path());
        conn.execute_batch(sql).unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        drop(conn);
        let before = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        let backup_before = backups(data.path());
        assert!(WorkStore::open(data.path()).is_err(), "accepted {sql}");
        assert_eq!(
            std::fs::read(data.path().join("brn.sqlite")).unwrap(),
            before
        );
        assert_eq!(backups(data.path()), backup_before);
    }
}

#[test]
fn valid_catalog_backup_restores_physical_damage_and_skips_semantically_invalid_candidate() {
    let data = fixture();
    let input = capture(data.path(), InboxKind::Markdown);
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let item = store.capture_inbox(&input).unwrap();
    drop(store);
    let (store, report) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.inbox_item(input.id).unwrap(), Some(item.clone()));
    drop(store);
    let bad = data.path().join("backups/brn-9999999999999.sqlite");
    std::fs::copy(&report.backup, &bad).unwrap();
    let conn = Connection::open(&bad).unwrap();
    conn.execute(
        "UPDATE inbox_items SET record_sha256=?1",
        [[0u8; 32].as_slice()],
    )
    .unwrap();
    drop(conn);
    let bad_bytes = std::fs::read(&bad).unwrap();
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic physical damage").unwrap();
    let (store, restored) = WorkStore::open(data.path()).unwrap();
    assert_eq!(restored.restored_from, Some(report.backup));
    assert!(restored.corrupt_moved_to.is_some());
    assert_eq!(store.inbox_item(input.id).unwrap(), Some(item));
    assert_eq!(std::fs::read(bad).unwrap(), bad_bytes);
    assert_eq!(
        std::fs::read(input.copy.directory.join(input.copy_name())).unwrap(),
        ORIGINAL.as_bytes()
    );
}
