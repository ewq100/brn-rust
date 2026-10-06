//! Structural DOCX records: genuine extraction is qualified by shared Workflow.
use brn_store::{
    WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind, MAX_INBOX_BINARY_BYTES},
        inbox_processing::*,
        inbox_source::{InboxSourceBinding, InboxSourcePreservation, read_provenance},
        proposal_apply::{ApplyMemberProof, ApplyOutcome, ApprovalRequest},
        proposals::{NoteChange, ProposalDraft, SourceVersion},
    },
};
use sha2::{Digest, Sha256};
use uuid::Uuid;
fn hash(b: &[u8]) -> [u8; 32] {
    Sha256::digest(b).into()
}
fn docx() -> InboxConversionFormat {
    serde_json::from_str("\"docx_text_v1\"").unwrap()
}
fn item(kind: InboxKind, len: u64) -> InboxItem {
    InboxItem {
        capture: InboxCapture {
            id: Uuid::from_u128(1),
            kind,
            title: "Evidence õ".into(),
            original_name: Some("copy.docx".into()),
            copy: InboxCopy {
                directory: "/synthetic/inbox".into(),
                directory_device: 1,
                directory_inode: 2,
                file_device: 1,
                file_inode: 3,
                byte_len: len,
                sha256: if len == 0 {
                    hash(&[])
                } else {
                    hash(b"synthetic original proof")
                },
            },
        },
        received_at_ms: 1,
    }
}
fn outcome(format: InboxConversionFormat, len: u64, sha: [u8; 32]) -> InboxProcessOutcome {
    InboxProcessOutcome::Converted {
        format,
        byte_len: len,
        sha256: sha,
    }
}
fn batch(original: InboxItem, result: InboxProcessOutcome) -> InboxProcessBatch {
    InboxProcessBatch {
        request: ProcessInboxRequest {
            id: Uuid::from_u128(2),
            items: vec![original],
        },
        queued_at_ms: 1,
        entries: vec![InboxProcessEntry {
            outcome: result,
            started_at_ms: Some(2),
            finished_at_ms: Some(3),
        }],
    }
}
fn binding(original: InboxItem, format: InboxConversionFormat, body: &str) -> InboxSourceBinding {
    InboxSourceBinding {
        visual: None,
        batch_id: Uuid::from_u128(2),
        index: 0,
        original,
        format,
        byte_len: body.len() as u64,
        sha256: hash(body.as_bytes()),
        note_id: Uuid::from_u128(3),
    }
}
fn proof(bytes: &[u8]) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode: 9,
        len: bytes.len() as u64,
        sha256: hash(bytes),
    }
}
fn draft(binding: &InboxSourceBinding, text: String) -> ProposalDraft {
    let parent = VaultIdentity {
        device: 1,
        inode: 4,
    };
    ProposalDraft {
        id: Uuid::from_u128(4),
        group_id: None,
        session_id: None,
        vault: Some(VaultRecord {
            id: Uuid::from_u128(5),
            root: "/synthetic/vault".into(),
            identity: parent.clone(),
        }),
        title: "Review converted Source".into(),
        changes: vec![NoteChange::Create {
            path: "Sources/converted.md".into(),
            parent,
            text,
        }],
        sources: vec![],
        action_changes: vec![],
        inbox_source: Some(Box::new(binding.clone())),
        inbox_knowledge: None,
    }
}
#[test]
fn converted_record_exact_kind_format_size_and_empty_hash_matrix() {
    assert_eq!(serde_json::to_string(&docx()).unwrap(), "\"docx_text_v1\"");
    for kind in [
        InboxKind::Text,
        InboxKind::Markdown,
        InboxKind::Email,
        InboxKind::Teams,
        InboxKind::Binary,
    ] {
        for format in [
            InboxConversionFormat::VerbatimMarkdownV1,
            InboxConversionFormat::LiteralTextV1,
            docx(),
        ] {
            let len = if kind == InboxKind::Binary { 22 } else { 4 };
            let (body_len, sha) = if format == InboxConversionFormat::VerbatimMarkdownV1 {
                (len, item(kind, len).capture.copy.sha256)
            } else {
                (20, hash(b"converted"))
            };
            let good = if kind == InboxKind::Binary {
                format == docx()
            } else {
                matches!(
                    (kind, format),
                    (
                        InboxKind::Markdown,
                        InboxConversionFormat::VerbatimMarkdownV1
                    ) | (
                        InboxKind::Text | InboxKind::Email | InboxKind::Teams,
                        InboxConversionFormat::LiteralTextV1
                    )
                )
            };
            assert_eq!(
                batch(item(kind, len), outcome(format, body_len, sha))
                    .validate()
                    .is_ok(),
                good,
                "{kind:?}/{format:?}"
            );
        }
    }
    for len in [22, MAX_INBOX_BINARY_BYTES as u64] {
        batch(
            item(InboxKind::Binary, len),
            outcome(docx(), 1024 * 1024, hash(b"bounded nonempty body")),
        )
        .validate()
        .unwrap();
        batch(item(InboxKind::Binary, len), outcome(docx(), 0, hash(&[])))
            .validate()
            .unwrap();
        assert!(
            batch(
                item(InboxKind::Binary, len),
                outcome(docx(), 0, hash(b"forged"))
            )
            .validate()
            .is_err()
        );
        assert!(
            batch(
                item(InboxKind::Binary, len),
                outcome(docx(), 1024 * 1024 + 1, hash(b"body"))
            )
            .validate()
            .is_err()
        );
    }
    for len in [0, 21, MAX_INBOX_BINARY_BYTES as u64 + 1] {
        assert!(
            batch(item(InboxKind::Binary, len), outcome(docx(), 0, hash(&[])))
                .validate()
                .is_err()
        );
    }
    let mut b = batch(item(InboxKind::Binary, 22), outcome(docx(), 0, hash(&[])));
    b.entries[0].finished_at_ms = Some(1);
    assert!(b.validate().is_err());
    b.entries[0].finished_at_ms = None;
    assert!(b.validate().is_err());
}
#[test]
fn docx_queue_failures_replay_restart_and_failed_update_are_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    for code in [
        "binary_unsupported",
        "docx_invalid",
        "docx_unsupported",
        "docx_limit",
    ] {
        let mut capture = item(InboxKind::Binary, 1).capture;
        capture.id = Uuid::new_v4();
        let saved = store.capture_inbox(&capture).unwrap();
        let r = ProcessInboxRequest {
            id: Uuid::new_v4(),
            items: vec![saved],
        };
        let queued = store.process_inbox(&r).unwrap();
        assert_eq!(store.process_inbox(&r).unwrap(), queued);
        let running = store.start_inbox_processing(r.id, 0).unwrap();
        assert!(
            store
                .finish_inbox_processing(
                    r.id,
                    0,
                    InboxProcessOutcome::Failed {
                        code: "docx_invented".into()
                    }
                )
                .is_err()
        );
        assert_eq!(store.inbox_processing(r.id).unwrap(), Some(running));
        let done = store
            .finish_inbox_processing(r.id, 0, InboxProcessOutcome::Failed { code: code.into() })
            .unwrap();
        assert_eq!(store.process_inbox(&r).unwrap(), done);
        drop(store);
        store = WorkStore::open(dir.path()).unwrap().0;
        assert_eq!(store.inbox_processing(r.id).unwrap(), Some(done));
    }
    let saved = store
        .capture_inbox(&item(InboxKind::Binary, 22).capture)
        .unwrap();
    let r = ProcessInboxRequest {
        id: Uuid::new_v4(),
        items: vec![saved],
    };
    store.process_inbox(&r).unwrap();
    store.start_inbox_processing(r.id, 0).unwrap();
    let done = store
        .finish_inbox_processing(r.id, 0, outcome(docx(), 0, hash(&[])))
        .unwrap();
    assert_eq!(store.process_inbox(&r).unwrap(), done);
    let mut changed = r.clone();
    changed.items[0].capture.title.push('x');
    assert!(store.process_inbox(&changed).is_err());
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.inbox_processing(r.id).unwrap(), Some(done));
}
#[test]
fn source_binding_and_provenance_exact_matrix_and_full_wrapper_bounds() {
    for kind in [
        InboxKind::Text,
        InboxKind::Markdown,
        InboxKind::Email,
        InboxKind::Teams,
        InboxKind::Binary,
    ] {
        for format in [
            InboxConversionFormat::VerbatimMarkdownV1,
            InboxConversionFormat::LiteralTextV1,
            docx(),
        ] {
            let len = if kind == InboxKind::Binary { 22 } else { 4 };
            let mut b = binding(item(kind, len), format, "converted body text");
            if format == InboxConversionFormat::VerbatimMarkdownV1 {
                b.byte_len = len;
                b.sha256 = b.original.capture.copy.sha256;
            }
            let good = if kind == InboxKind::Binary {
                format == docx()
            } else {
                matches!(
                    (kind, format),
                    (
                        InboxKind::Markdown,
                        InboxConversionFormat::VerbatimMarkdownV1
                    ) | (
                        InboxKind::Text | InboxKind::Email | InboxKind::Teams,
                        InboxConversionFormat::LiteralTextV1
                    )
                )
            };
            assert_eq!(b.validate().is_ok(), good, "binding {kind:?}/{format:?}");
            let s = format!(
                "---\nbrn_inbox_source: {}\n---\n",
                serde_json::to_string(&b.provenance()).unwrap()
            );
            assert_eq!(
                read_provenance(&s).is_ok(),
                good,
                "provenance {kind:?}/{format:?}"
            );
        }
    }
    let b = binding(
        item(InboxKind::Binary, MAX_INBOX_BINARY_BYTES as u64),
        docx(),
        "",
    );
    let text = b.markdown("").unwrap();
    b.validate_markdown(&text).unwrap();
    assert_eq!(read_provenance(&text).unwrap(), Some(b.provenance()));
    let header_len = text.len();
    let body = "x".repeat(1024 * 1024 - header_len);
    let mut b = binding(b.original.clone(), docx(), &body);
    let text = b.markdown(&body).unwrap();
    assert_eq!(text.len(), 1024 * 1024);
    let longer = body + "x";
    b.byte_len = longer.len() as u64;
    b.sha256 = hash(longer.as_bytes());
    assert!(b.markdown(&longer).is_err());
    for len in [0, 21, MAX_INBOX_BINARY_BYTES as u64 + 1] {
        let b = binding(item(InboxKind::Binary, len), docx(), "");
        assert!(b.validate().is_err());
        let s = format!(
            "---\nbrn_inbox_source: {}\n---\n",
            serde_json::to_string(&b.provenance()).unwrap()
        );
        assert!(read_provenance(&s).is_err());
    }
    let mut b = binding(item(InboxKind::Binary, 22), docx(), "");
    b.sha256 = hash(b"forged");
    assert!(b.validate().is_err());
}
#[test]
fn historical_docx_source_journal_is_self_contained_but_never_cleanup_authority() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    // Deliberately no catalog/processing or raw binary copy: Store checks full
    // structural historical authority; Workflow qualifies fresh extraction.
    let b = binding(item(InboxKind::Binary, 22), docx(), "# Converted õ\n");
    let text = b.markdown("# Converted õ\n").unwrap();
    let d = draft(&b, text.clone());
    let mut asset_shape = d.clone();
    asset_shape.changes.push(NoteChange::CreateAsset {
        path: "image.png".into(),
        parent: VaultIdentity {
            device: 1,
            inode: 4,
        },
        bytes: vec![255],
    });
    assert!(store.create_proposal(&asset_shape).is_err());
    assert!(store.proposal(d.id).unwrap().is_none());
    let record = store.create_proposal(&d).unwrap();
    let r = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    store.begin_proposal_apply(&r).unwrap();
    let f = proof(text.as_bytes());
    store
        .record_proposal_prepared(r.operation_id, std::slice::from_ref(&f))
        .unwrap();
    store
        .finish_proposal_apply(
            r.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(f.clone()),
                staging: None,
            }]),
        )
        .unwrap();
    let journal = store.proposal_apply(r.operation_id).unwrap().unwrap();
    journal.validate().unwrap();
    let fresh = tempfile::tempdir().unwrap();
    let (mut restored, _) = WorkStore::open(fresh.path()).unwrap();
    assert_eq!(restored.restore_proposal_apply(&journal).unwrap(), journal);
    drop(restored);
    let (restored, _) = WorkStore::open(fresh.path()).unwrap();
    assert_eq!(
        restored.proposal_apply(r.operation_id).unwrap(),
        Some(journal.clone())
    );
    assert!(
        InboxSourcePreservation {
            original: &b.original,
            original_text: "not a docx",
            approval: &journal,
            saved: &SourceVersion {
                path: "Sources/converted.md".into(),
                fingerprint: f
            },
            saved_text: &text
        }
        .validate()
        .is_err()
    );
    assert!(restored.inbox_processing(b.batch_id).unwrap().is_none());
    assert!(
        restored
            .inbox_item(b.original.capture.id)
            .unwrap()
            .is_none()
    );
}

#[test]
fn literal_legacy_source_header_body_and_provenance_are_byte_identical() {
    let mut original = item(InboxKind::Email, 4);
    original.capture.copy.sha256 = hash(b"body");
    let (format, body) = brn_store::work::inbox_source::convert_original(
        InboxKind::Email,
        "body",
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    let b = binding(original, format, &body);
    const OLD: &str = r##"---
brn_id: 00000000-0000-0000-0000-000000000003
brn_kind: source
brn_state: current
brn_inbox_source: {"item_id":"00000000-0000-0000-0000-000000000001","kind":"email","title":"Evidence õ","original_name":"copy.docx","received_at_ms":1,"original_byte_len":4,"original_sha256":[35,13,131,88,220,142,136,144,180,197,141,238,182,41,18,238,47,32,53,122,233,42,92,200,97,185,142,104,254,49,172,181],"format":"literal_text_v1"}
---
```text
body
```
"##;
    assert_eq!(b.markdown(&body).unwrap(), OLD);
    b.validate_markdown(OLD).unwrap();
    assert_eq!(read_provenance(OLD).unwrap(), Some(b.provenance()));
    assert_eq!(
        format!("{:x}", Sha256::digest(OLD.as_bytes())),
        "c411cc0b09b6ae536f6a513b9f9ef3d74c8bd5ad96ebb99f582a20f1e62a8c3f"
    );
}

#[test]
fn rehashed_docx_processing_damage_refuses_reopen_without_restoring_backup() {
    for wrong_hash in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let saved = store
            .capture_inbox(&item(InboxKind::Binary, 22).capture)
            .unwrap();
        let r = ProcessInboxRequest {
            id: Uuid::new_v4(),
            items: vec![saved],
        };
        store.process_inbox(&r).unwrap();
        store.start_inbox_processing(r.id, 0).unwrap();
        let mut done = store
            .finish_inbox_processing(r.id, 0, outcome(docx(), 0, hash(&[])))
            .unwrap();
        drop(store);
        done.entries[0].outcome = if wrong_hash {
            outcome(docx(), 0, hash(b"forged"))
        } else {
            outcome(InboxConversionFormat::LiteralTextV1, 34, hash(b"forged"))
        };
        let bytes = serde_json::to_vec(&done).unwrap();
        let db = dir.path().join("brn.sqlite");
        let raw = rusqlite::Connection::open(&db).unwrap();
        raw.execute(
            "UPDATE inbox_processing SET record_json=?1,record_sha256=?2",
            rusqlite::params![bytes, hash(&bytes).as_slice()],
        )
        .unwrap();
        drop(raw);
        let saved_db = std::fs::read(&db).unwrap();
        let backups = std::fs::read_dir(dir.path().join("backups"))
            .unwrap()
            .count();
        assert!(matches!(
            WorkStore::open(dir.path()),
            Err(brn_store::Error::Invalid(_))
        ));
        assert_eq!(std::fs::read(&db).unwrap(), saved_db);
        assert_eq!(
            std::fs::read_dir(dir.path().join("backups"))
                .unwrap()
                .count(),
            backups
        );
    }
}
