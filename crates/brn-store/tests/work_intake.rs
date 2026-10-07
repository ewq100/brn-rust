use base64::{Engine, engine::general_purpose::STANDARD};
use brn_intake::{Extraction, ImageAsset, ImageOccurrence, SourceNode};
use brn_store::{
    Error, WorkStore,
    work::{
        inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind},
        inbox_actions::IntakeCitation,
        inbox_processing::{MAX_PROCESS_BATCH, ProcessInboxRequest},
        intake::IntakeSnapshot,
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

const ORIGINAL: &[u8] =
    b"From: synthetic@example.test\r\nSubject: exact bytes\r\n\r\nBody \xff\x00";

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

fn raw(data: &Path) -> Connection {
    Connection::open(data.join("brn.sqlite")).unwrap()
}

fn item() -> InboxItem {
    InboxItem {
        capture: InboxCapture {
            id: Uuid::new_v4(),
            kind: InboxKind::Email,
            title: "Synthetic email".into(),
            original_name: Some("../../display-only.eml".into()),
            copy: InboxCopy {
                directory: "/synthetic/private-intake".into(),
                directory_device: 1,
                directory_inode: 2,
                file_device: 1,
                file_inode: 3,
                byte_len: ORIGINAL.len() as u64,
                sha256: digest(ORIGINAL),
            },
        },
        received_at_ms: 7,
    }
}

fn extraction() -> Extraction {
    let image = STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAIAAACQkWg2AAAAGklEQVR4nGP4z8BAEiJN9agGYjUMSkeNPA0Aker/AYCSZqUAAAAASUVORK5CYII=").unwrap();
    let asset = ImageAsset {
        id: format!("asset-{}", brn_intake::hex(&digest(&image))),
        sha256: digest(&image),
        width: 16,
        height: 16,
        media_type: "image/png".into(),
        bytes: image,
    };
    let image_markdown = format!(
        "![Synthetic image](assets/{})\n",
        brn_intake::asset_file_name(&asset).unwrap()
    );
    let mut markdown = "Exact extracted body λ\nDocument content λ\n".to_owned();
    let mut occurrences = vec![];
    for index in 0..2 {
        let start = markdown.len();
        markdown.push_str(&image_markdown);
        occurrences.push(ImageOccurrence {
            id: format!("occurrence-{index}"),
            source_id: "source-docx".into(),
            asset_id: asset.id.clone(),
            locator: format!("document/body/paragraph/{index}/image"),
            alt: Some("Synthetic image".into()),
            start,
            end: markdown.len(),
        });
    }
    Extraction {
        limits: Default::default(),
        consumed: None,

        schema: 1,
        converter: "synthetic-adapter/1".into(),
        original_sha256: digest(ORIGINAL),
        markdown,
        sources: vec![
            SourceNode {
                id: "source-1".into(),
                parent: None,
                name: "synthetic.eml".into(),
                media_type: "message/rfc822".into(),
                locator: "original".into(),
                status: "complete".into(),
                bytes: ORIGINAL.into(),
                text: "Exact extracted body λ\n".into(),
            },
            SourceNode {
                id: "source-docx".into(),
                parent: Some("source-1".into()),
                name: "document.docx".into(),
                media_type:
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document".into(),
                locator: "mime/attachment/1".into(),
                status: "complete".into(),
                bytes: b"opaque synthetic document \xff\x00".into(),
                text: "Document content λ".into(),
            },
            SourceNode {
                id: "source-xlsx".into(),
                parent: Some("source-1".into()),
                name: "retained.xlsx".into(),
                media_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                    .into(),
                locator: "mime/attachment/2".into(),
                status: "unprocessed".into(),
                bytes: b"opaque synthetic spreadsheet \x00\xfe".into(),
                text: String::new(),
            },
        ],
        assets: vec![asset],
        occurrences,
        gaps: vec![
            "Synthetic extraction does not decode the opaque suffix.".into(),
            "retained.xlsx: unsupported spreadsheet format; original retained.".into(),
        ],
    }
}

fn admitted(store: &mut WorkStore) -> IntakeSnapshot {
    let original = store.restore_inbox(&item()).unwrap();
    let batch_id = Uuid::new_v4();
    store
        .process_inbox(&ProcessInboxRequest {
            limits: None,

            id: batch_id,
            items: vec![original.clone()],
        })
        .unwrap();
    IntakeSnapshot {
        id: Uuid::new_v4(),
        batch_id,
        index: 0,
        original,
        extraction: extraction(),
    }
}

fn record(conn: &Connection) -> Vec<u8> {
    conn.query_row("SELECT record_json FROM intake_snapshots", [], |r| r.get(0))
        .unwrap()
}

fn backups(data: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = std::fs::read_dir(data.join("backups"))
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().into_string().unwrap(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

#[test]
fn exact_manifest_and_opaque_bytes_replay_restart_and_backup_without_queue_or_files() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let snapshot = admitted(&mut store);
    snapshot.validate().unwrap();
    assert_eq!(store.save_intake_snapshot(&snapshot).unwrap(), snapshot);
    let conn = raw(data.path());
    let bytes = record(&conn);
    assert_eq!(bytes, serde_json::to_vec(&snapshot).unwrap());
    assert_eq!(snapshot.digest().unwrap(), digest(&bytes));
    assert_eq!(store.save_intake_snapshot(&snapshot).unwrap(), snapshot);
    assert_eq!(record(&conn), bytes);
    // Queue/catalog rows and original files are not the authority for history.
    conn.execute("DELETE FROM inbox_processing", []).unwrap();
    conn.execute("DELETE FROM inbox_items", []).unwrap();
    assert_eq!(store.save_intake_snapshot(&snapshot).unwrap(), snapshot);
    drop(conn);
    drop(store);
    let (store, report) = WorkStore::open(data.path()).unwrap();
    assert_eq!(
        store.intake_snapshot(snapshot.id).unwrap(),
        Some(snapshot.clone())
    );
    assert_eq!(
        store.intake_snapshot_for(snapshot.batch_id, 0).unwrap(),
        Some(snapshot.clone())
    );
    assert_eq!(record(&raw(data.path())), bytes);
    let backup = Connection::open(report.backup).unwrap();
    assert_eq!(record(&backup), bytes);
    assert_eq!(snapshot.extraction.sources[0].bytes, ORIGINAL);
    assert_eq!(
        snapshot.extraction.sources[1].bytes,
        b"opaque synthetic document \xff\x00"
    );
    assert_eq!(
        snapshot.extraction.sources[2].bytes,
        b"opaque synthetic spreadsheet \x00\xfe"
    );
    assert_eq!(snapshot.extraction.sources[2].status, "unprocessed");
    assert_eq!(snapshot.extraction.assets.len(), 1);
    assert_eq!(snapshot.extraction.occurrences.len(), 2);
    assert_eq!(
        snapshot.extraction.occurrences[0].asset_id,
        snapshot.extraction.occurrences[1].asset_id
    );
    assert_ne!(
        snapshot.extraction.occurrences[0].locator,
        snapshot.extraction.occurrences[1].locator
    );
}

#[test]
fn id_and_batch_slot_reuse_refuse_changed_complete_snapshots() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let snapshot = admitted(&mut store);
    store.save_intake_snapshot(&snapshot).unwrap();
    let bytes = record(&raw(data.path()));
    for change in 0..6 {
        let mut changed = snapshot.clone();
        match change {
            0 => changed.extraction.markdown.push('λ'),
            1 => changed.extraction.converter.push_str("-upgraded"),
            2 => changed.original.received_at_ms += 1,
            3 => changed.batch_id = Uuid::new_v4(),
            4 => changed.id = Uuid::new_v4(),
            _ => changed.index = 1,
        }
        assert!(
            matches!(
                store.save_intake_snapshot(&changed),
                Err(Error::OperationConflict(_))
            ),
            "change {change}"
        );
        assert_eq!(record(&raw(data.path())), bytes);
        assert_eq!(
            store.intake_snapshot(snapshot.id).unwrap(),
            Some(snapshot.clone())
        );
    }
}

#[test]
fn admission_requires_exact_batch_item_and_root_bytes_before_any_write() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let snapshot = admitted(&mut store);
    for change in 0..10 {
        let mut changed = snapshot.clone();
        match change {
            0 => changed.id = Uuid::nil(),
            1 => changed.batch_id = Uuid::nil(),
            2 => changed.index = MAX_PROCESS_BATCH,
            3 => changed.index = 1,
            4 => changed.batch_id = Uuid::new_v4(),
            5 => changed.original.received_at_ms += 1,
            6 => changed.original.capture.copy.byte_len += 1,
            7 => changed.original.capture.copy.sha256[0] ^= 1,
            8 => changed.extraction.original_sha256[0] ^= 1,
            _ => changed.extraction.sources[0].bytes.push(0),
        }
        assert!(
            store.save_intake_snapshot(&changed).is_err(),
            "change {change}"
        );
        assert_eq!(
            raw(data.path())
                .query_row("SELECT count(*) FROM intake_snapshots", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    assert_eq!(store.save_intake_snapshot(&snapshot).unwrap(), snapshot);
    assert!(store.intake_snapshot(Uuid::nil()).is_err());
    assert!(store.intake_snapshot_for(Uuid::nil(), 0).is_err());
    assert!(
        store
            .intake_snapshot_for(snapshot.batch_id, MAX_PROCESS_BATCH)
            .is_err()
    );
    assert_eq!(store.intake_snapshot(Uuid::new_v4()).unwrap(), None);
    assert_eq!(store.intake_snapshot_for(Uuid::new_v4(), 0).unwrap(), None);
}

#[test]
fn readable_hash_record_and_row_damage_refuse_without_replacing_main_or_backups() {
    for damage in 0..11 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let snapshot = admitted(&mut store);
        store.save_intake_snapshot(&snapshot).unwrap();
        let conn = raw(data.path());
        match damage {
            0 => {
                conn.execute(
                    "UPDATE intake_snapshots SET record_sha256=?1",
                    [[0u8; 32].as_slice()],
                )
                .unwrap();
            }
            1 => {
                conn.execute(
                    "UPDATE intake_snapshots SET batch_id=?1",
                    [Uuid::new_v4().to_string()],
                )
                .unwrap();
            }
            2 => {
                conn.execute("UPDATE intake_snapshots SET item_index=1", [])
                    .unwrap();
            }
            3..=7 => {
                let mut value = serde_json::to_value(&snapshot).unwrap();
                match damage {
                    3 => value["unowned_field"] = true.into(),
                    4 => {
                        value["original"]["capture"]["copy"]["byte_len"] =
                            (ORIGINAL.len() as u64 + 1).into()
                    }
                    5 => value["extraction"]["schema"] = 999.into(),
                    6 => value["extraction"]["sources"][0]["bytes"] = "AA==".into(),
                    _ => {
                        value.as_object_mut().unwrap().remove("index");
                    }
                }
                let bytes = serde_json::to_vec(&value).unwrap();
                conn.execute(
                    "UPDATE intake_snapshots SET record_json=?1,record_sha256=?2",
                    params![bytes, digest(&bytes).as_slice()],
                )
                .unwrap();
            }
            8 => {
                let bytes = serde_json::to_string_pretty(&snapshot)
                    .unwrap()
                    .into_bytes();
                conn.execute(
                    "UPDATE intake_snapshots SET record_json=?1,record_sha256=?2",
                    params![bytes, digest(&bytes).as_slice()],
                )
                .unwrap();
            }
            9 => {
                conn.pragma_update(None, "ignore_check_constraints", true)
                    .unwrap();
                conn.execute("UPDATE intake_snapshots SET item_index=-1", [])
                    .unwrap();
            }
            _ => {
                conn.execute(
                    "UPDATE intake_snapshots SET record_json=zeroblob(?1)",
                    [(brn_intake::MAX_OUTPUT_BYTES + 64 * 1024 + 1) as i64],
                )
                .unwrap();
            }
        }
        assert!(
            store.intake_snapshot(snapshot.id).is_err(),
            "read damage {damage}"
        );
        assert!(
            store.save_intake_snapshot(&snapshot).is_err(),
            "replay damage {damage}"
        );
        drop(store);
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        drop(conn);
        let before = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        let backup_before = backups(data.path());
        assert!(
            WorkStore::open(data.path()).is_err(),
            "startup damage {damage}"
        );
        assert_eq!(
            std::fs::read(data.path().join("brn.sqlite")).unwrap(),
            before
        );
        assert_eq!(backups(data.path()), backup_before);
        assert!(!std::fs::read_dir(data.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("brn.sqlite.corrupt-")
        }));
    }
}

#[test]
fn unexpected_schema_objects_refuse_reads_and_startup_before_backup() {
    for sql in [
        "CREATE INDEX unexpected_intake_index ON intake_snapshots(batch_id)",
        "CREATE TRIGGER unexpected_intake_trigger AFTER INSERT ON intake_snapshots BEGIN DELETE FROM intake_snapshots; END;",
        "ALTER TABLE intake_snapshots ADD COLUMN undocumented TEXT",
        "DROP TABLE intake_snapshots",
    ] {
        let data = fixture();
        let (store, _) = WorkStore::open(data.path()).unwrap();
        let conn = raw(data.path());
        conn.execute_batch(sql).unwrap();
        assert!(
            store.intake_snapshot(Uuid::new_v4()).is_err(),
            "accepted {sql}"
        );
        drop(store);
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
fn v15_migration_adds_snapshot_storage_without_rewriting_old_canonical_records() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let original = store.restore_inbox(&item()).unwrap();
    drop(store);
    let conn = raw(data.path());
    let old: Vec<u8> = conn
        .query_row("SELECT record_json FROM inbox_items", [], |r| r.get(0))
        .unwrap();
    conn.execute_batch(
        "DROP TABLE intake_snapshots; PRAGMA user_version=15; PRAGMA wal_checkpoint(TRUNCATE);",
    )
    .unwrap();
    drop(conn);
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(
        store.inbox_item(original.capture.id).unwrap(),
        Some(original)
    );
    assert_eq!(
        raw(data.path())
            .query_row("SELECT record_json FROM inbox_items", [], |r| r
                .get::<_, Vec<u8>>(0))
            .unwrap(),
        old
    );
    assert_eq!(
        raw(data.path())
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        16
    );
    let snapshot = admitted(&mut store);
    assert_eq!(store.save_intake_snapshot(&snapshot).unwrap(), snapshot);
}

#[test]
fn backup_recovery_restores_all_evidence_and_skips_invalid_snapshot_candidates() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let snapshot = admitted(&mut store);
    store.save_intake_snapshot(&snapshot).unwrap();
    drop(store);
    let (store, report) = WorkStore::open(data.path()).unwrap();
    let bytes = record(&raw(data.path()));
    drop(store);
    let invalid_backup = data.path().join("backups/brn-9999999999999.sqlite");
    std::fs::copy(&report.backup, &invalid_backup).unwrap();
    let conn = Connection::open(&invalid_backup).unwrap();
    conn.execute(
        "UPDATE intake_snapshots SET record_sha256=?1",
        [[0u8; 32].as_slice()],
    )
    .unwrap();
    drop(conn);
    let invalid_bytes = std::fs::read(&invalid_backup).unwrap();
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic physical damage").unwrap();
    let (store, restored) = WorkStore::open(data.path()).unwrap();
    assert_eq!(restored.restored_from, Some(report.backup));
    assert!(restored.corrupt_moved_to.is_some());
    assert_eq!(store.intake_snapshot(snapshot.id).unwrap(), Some(snapshot));
    assert_eq!(record(&raw(data.path())), bytes);
    assert_eq!(std::fs::read(invalid_backup).unwrap(), invalid_bytes);
}

#[test]
fn complete_private_mirror_restores_without_queue_and_keeps_immutable_conflicts() {
    let original_data = fixture();
    let (mut original_store, _) = WorkStore::open(original_data.path()).unwrap();
    let snapshot = admitted(&mut original_store);
    original_store.save_intake_snapshot(&snapshot).unwrap();
    let restored_data = fixture();
    let (mut restored, _) = WorkStore::open(restored_data.path()).unwrap();
    assert!(restored.save_intake_snapshot(&snapshot).is_err());
    assert_eq!(
        restored.restore_intake_snapshot(&snapshot).unwrap(),
        snapshot
    );
    assert_eq!(
        restored.restore_intake_snapshot(&snapshot).unwrap(),
        snapshot
    );
    let mut changed = snapshot.clone();
    changed.extraction.markdown.push('λ');
    assert!(matches!(
        restored.restore_intake_snapshot(&changed),
        Err(Error::OperationConflict(_))
    ));
    changed = snapshot.clone();
    changed.id = Uuid::new_v4();
    assert!(matches!(
        restored.restore_intake_snapshot(&changed),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(
        restored.intake_snapshot(snapshot.id).unwrap(),
        Some(snapshot)
    );
}

#[test]
fn private_quotes_bind_exact_processed_node_and_locator_and_refuse_ambiguous_or_generated_text() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut snapshot = admitted(&mut store);
    let quote = "Document content λ";
    let start = snapshot.extraction.markdown.find(quote).unwrap();
    let citation = IntakeCitation::resolve(&snapshot, start, start + quote.len(), None).unwrap();
    assert_eq!(citation.source_id, "source-docx");
    assert_eq!(citation.locator, "mime/attachment/1");
    assert_eq!(citation.start, 0);
    assert_eq!(citation.end, quote.len());
    assert_eq!(citation.quote, quote);
    assert!(
        IntakeCitation::resolve(&snapshot, start, start + quote.len(), Some("source-xlsx"))
            .is_err()
    );
    snapshot
        .extraction
        .markdown
        .push_str("Generated extraction gap label");
    let generated = snapshot
        .extraction
        .markdown
        .find("Generated extraction gap label")
        .unwrap();
    assert!(
        IntakeCitation::resolve(
            &snapshot,
            generated,
            snapshot.extraction.markdown.len(),
            None
        )
        .is_err()
    );
    let mut duplicate = snapshot.extraction.sources[1].clone();
    duplicate.id = "source-another-docx".into();
    duplicate.locator = "mime/attachment/3".into();
    snapshot.extraction.sources.push(duplicate);
    assert!(IntakeCitation::resolve(&snapshot, start, start + quote.len(), None).is_err());
    assert!(
        IntakeCitation::resolve(&snapshot, start, start + quote.len(), Some("source-docx"))
            .is_err()
    );
    // Even explicit node selection cannot claim another identical attachment's span.
    snapshot
        .extraction
        .markdown
        .push_str("\nDocument content λ\n");
    let last = snapshot.extraction.markdown.rfind(quote).unwrap();
    assert!(
        IntakeCitation::resolve(&snapshot, last, last + quote.len(), Some("source-docx")).is_err()
    );
    snapshot.extraction.markdown = "aaaaa".into();
    for source in &mut snapshot.extraction.sources {
        source.text.clear();
    }
    snapshot.extraction.sources[0].text = "aaaa".into();
    assert!(IntakeCitation::resolve(&snapshot, 2, 3, Some("source-1")).is_err());
}
