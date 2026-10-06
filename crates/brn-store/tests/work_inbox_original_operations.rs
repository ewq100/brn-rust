use brn_store::work::inbox_original_operations::*;
use brn_store::{
    WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind},
        inbox_source::{InboxSourceBinding, convert_original},
        proposal_apply::{ApplyJournal, ApplyMemberProof, ApplyOutcome, ApprovalRequest},
        proposals::{NoteChange, ProposalDraft, SourceVersion},
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use uuid::Uuid;

fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn fingerprint(text: &str, inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: text.len() as u64,
        sha256: hash(text.as_bytes()),
    }
}
fn fixture() -> (tempfile::TempDir, WorkStore) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    (dir, store)
}
struct Witness {
    original: InboxItem,
    original_text: String,
    approval: ApplyJournal,
    saved: SourceVersion,
    saved_text: String,
}
impl Witness {
    fn rebind_saved(&mut self) {
        self.saved.fingerprint = fingerprint(&self.saved_text, self.saved.fingerprint.inode);
    }
}
fn witness(store: &mut WorkStore, kind: InboxKind, text: &str) -> Witness {
    let original = store
        .capture_inbox(&InboxCapture {
            id: Uuid::new_v4(),
            kind,
            title: "Synthetic exact õ".into(),
            original_name: Some("original.txt".into()),
            copy: InboxCopy {
                directory: "/synthetic/inbox".into(),
                directory_device: 1,
                directory_inode: 2,
                file_device: 1,
                file_inode: 3,
                byte_len: text.len() as u64,
                sha256: hash(text.as_bytes()),
            },
        })
        .unwrap();
    let (format, converted) = convert_original(kind, text, &AtomicBool::new(false)).unwrap();
    let binding = InboxSourceBinding {
        visual: None,
        original: original.clone(),
        batch_id: Uuid::new_v4(),
        index: 0,
        note_id: Uuid::new_v4(),
        format,
        byte_len: converted.len() as u64,
        sha256: hash(converted.as_bytes()),
    };
    let saved_text = binding.markdown(&converted).unwrap();
    let parent = VaultIdentity {
        device: 1,
        inode: 4,
    };
    let review = store
        .create_proposal(&ProposalDraft {
            inbox_source: Some(Box::new(binding)),
            inbox_knowledge: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: Some(VaultRecord {
                id: Uuid::new_v4(),
                root: "/synthetic/vault".into(),
                identity: parent.clone(),
            }),
            title: "Preserve the exact original".into(),
            changes: vec![NoteChange::Create {
                path: "Sources/original.md".into(),
                parent,
                text: saved_text.clone(),
            }],
            sources: vec![],
            action_changes: vec![],
        })
        .unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    let saved = SourceVersion {
        path: "Sources/original.md".into(),
        fingerprint: fingerprint(&saved_text, 5),
    };
    store
        .record_proposal_prepared(
            request.operation_id,
            std::slice::from_ref(&saved.fingerprint),
        )
        .unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(saved.fingerprint.clone()),
                staging: None,
            }]),
        )
        .unwrap();
    Witness {
        original,
        original_text: text.into(),
        approval: store.proposal_apply(request.operation_id).unwrap().unwrap(),
        saved,
        saved_text,
    }
}

fn evidence(w: &Witness) -> InboxQualifiedRemovalEvidence {
    InboxQualifiedRemovalEvidence {
        item: w.original.clone(),
        original: InboxQualifiedOriginal::Available {
            text: w.original_text.clone(),
        },
        source: Some(InboxApprovedSource {
            approval: w.approval.clone(),
            saved: InboxSourceProof {
                source: w.saved.clone(),
                text: w.saved_text.clone(),
            },
        }),
        blockers: [],
        needs_owner_confirmation: true,
    }
}
fn removal(w: &Witness, parent: Option<InboxOriginalParent>, at: u64) -> InboxOriginalOperation {
    let evidence = evidence(w);
    InboxOriginalOperation::Remove(Box::new(InboxOriginalRemovalRecord {
        request: RemoveInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            item_id: w.original.capture.id,
            preview_digest: evidence.digest().unwrap(),
            previous_restore: parent,
            confirmation: InboxRemovalConfirmation {
                version: 1,
                exact_copy_removal_intended: true,
            },
        },
        evidence,
        namespace: InboxOriginalNamespace {
            data_device: 1,
            data_inode: 99,
        },
        prepared_at_ms: at,
        removed_at_ms: Some(at),
    }))
}
fn restore(parent: &InboxOriginalOperation, at: u64) -> InboxOriginalOperation {
    InboxOriginalOperation::Restore(Box::new(InboxOriginalRestoreRecord {
        request: RestoreInboxOriginalRequest {
            operation_id: Uuid::new_v4(),
            removal_operation_id: parent.summary().unwrap().operation_id,
            removal_digest: parent.summary().unwrap().record_sha256,
        },
        original: parent.original().clone(),
        namespace: parent.namespace().clone(),
        prepared_at_ms: at,
        restored_at_ms: Some(at),
    }))
}
#[test]
fn lean_records_import_without_processing_or_proposals_and_reopen() {
    let (_owner, mut creator) = fixture();
    let w = witness(&mut creator, InboxKind::Email, "Exact õ\r\n");
    let r = removal(&w, None, w.original.received_at_ms);
    let id = r.summary().unwrap().operation_id;
    let (dir, mut store) = fixture();
    let page = store
        .restore_inbox_original_operations(std::slice::from_ref(&r), &[id])
        .unwrap();
    assert_eq!(page.history.len(), 1);
    assert_eq!(page.selected.len(), 1);
    assert_eq!(page.parsed_bodies, 1);
    assert_eq!(
        store.inbox_item(w.original.capture.id).unwrap(),
        Some(w.original.clone())
    );
    assert!(
        store
            .proposal_apply(w.approval.request.operation_id)
            .unwrap()
            .is_none()
    );
    let replay = store
        .restore_inbox_original_operations(std::slice::from_ref(&r), &[id])
        .unwrap();
    assert_eq!(
        replay.parsed_bodies, 3,
        "initial inventory + one keyed CAS + final inventory"
    );
    assert_eq!(replay.selected[0].digest().unwrap(), r.digest().unwrap());
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.inbox_original_operations(&[id]).unwrap().selected[0]
            .digest()
            .unwrap(),
        r.digest().unwrap()
    );
}
#[test]
fn pure_source_witness_and_owner_confirmation_are_exact() {
    let (_owner, mut store) = fixture();
    let mut w = witness(&mut store, InboxKind::Text, "Body\r\n");
    let binding = w.approval.approved.draft.inbox_source.as_ref().unwrap();
    let body =
        w.saved_text[brn_store::note_identity::body_start(&w.saved_text).unwrap()..].to_owned();
    w.saved_text = format!(
        "---\nowner: 'kept'\nbrn_id: {}\nbrn_kind: source\nbrn_state: history\nbrn_inbox_source: {}\n---\n{}",
        binding.note_id,
        serde_json::to_string(&binding.provenance()).unwrap(),
        body
    );
    w.saved.path = "archive/source.md".into();
    w.saved.fingerprint.inode = 999;
    w.rebind_saved();
    let valid = removal(&w, None, w.original.received_at_ms);
    valid.validate().unwrap();
    for mode in 0..9 {
        let mut damaged = valid.clone();
        let InboxOriginalOperation::Remove(r) = &mut damaged else {
            unreachable!()
        };
        match mode {
            0 => r.request.confirmation.exact_copy_removal_intended = false,
            1 => r.request.confirmation.version = 2,
            2 => r.request.preview_digest[0] ^= 1,
            3 => r.request.item_id = Uuid::new_v4(),
            4 => r.evidence.needs_owner_confirmation = false,
            5 => r.evidence.source = None,
            6 => r.evidence.source.as_mut().unwrap().saved.text.push('x'),
            7 => r.evidence.item.capture.copy.file_inode += 1,
            _ => r.removed_at_ms = Some(r.prepared_at_ms - 1),
        }
        assert!(damaged.validate().is_err(), "mode {mode}");
    }
}
#[test]
fn causal_chain_bulk_import_is_order_independent_and_atomic() {
    let (_owner, mut creator) = fixture();
    let w = witness(&mut creator, InboxKind::Markdown, "body");
    let at = w.original.received_at_ms;
    let a = removal(&w, None, at);
    let b = restore(&a, at + 1);
    let c = removal(
        &w,
        Some(InboxOriginalParent {
            operation_id: b.summary().unwrap().operation_id,
            record_sha256: b.summary().unwrap().record_sha256,
        }),
        at + 2,
    );
    let (_dir, mut store) = fixture();
    assert!(
        store
            .restore_inbox_original_operations(std::slice::from_ref(&c), &[])
            .is_err()
    );
    assert!(store.inbox_item(w.original.capture.id).unwrap().is_none());
    let got = store
        .restore_inbox_original_operations(&[c.clone(), a.clone(), b.clone()], &[])
        .unwrap();
    assert_eq!(
        got.history
            .iter()
            .map(|s| s.operation_id)
            .collect::<Vec<_>>(),
        [
            a.summary().unwrap().operation_id,
            b.summary().unwrap().operation_id,
            c.summary().unwrap().operation_id
        ]
    );
    let fork = restore(&a, at + 3);
    assert!(
        store
            .restore_inbox_original_operations(&[fork], &[])
            .is_err()
    );
    let mut bad = c.clone();
    let InboxOriginalOperation::Remove(r) = &mut bad else {
        unreachable!()
    };
    r.request.previous_restore.as_mut().unwrap().record_sha256[0] ^= 1;
    assert!(
        store
            .restore_inbox_original_operations(&[bad], &[])
            .is_err()
    );
    assert_eq!(
        store.inbox_original_operations(&[]).unwrap().history.len(),
        3
    );
}
#[test]
fn readable_metadata_body_schema_and_catalog_damage_never_restore_older_work() {
    for mode in 0..9 {
        let (_owner, mut creator) = fixture();
        let w = witness(&mut creator, InboxKind::Text, "body");
        let r = removal(&w, None, w.original.received_at_ms);
        let id = r.summary().unwrap().operation_id;
        let (dir, mut store) = fixture();
        store.restore_inbox_original_operations(&[r], &[]).unwrap();
        drop(store);
        let db = dir.path().join("brn.sqlite");
        let raw = Connection::open(&db).unwrap();
        match mode {
            0 => {
                raw.execute(
                    "UPDATE inbox_original_operations SET item_id=?1",
                    [Uuid::new_v4().to_string()],
                )
                .unwrap();
            }
            1 => {
                raw.execute("UPDATE inbox_original_operations SET prepared_at_ms=-1", [])
                    .unwrap();
            }
            2 => {
                raw.execute(
                    "UPDATE inbox_original_operations SET original_sha256=zeroblob(32)",
                    [],
                )
                .unwrap();
            }
            3 => {
                raw.execute("UPDATE inbox_original_operations SET format=1", [])
                    .unwrap();
            }
            4 => {
                let bytes: Vec<u8> = raw
                    .query_row(
                        "SELECT record_json FROM inbox_original_operations",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                value["record"]["evidence"]["needs_owner_confirmation"] = serde_json::json!(false);
                let bytes = serde_json::to_vec(&value).unwrap();
                raw.execute(
                    "UPDATE inbox_original_operations SET record_json=?1,body_sha256=?2",
                    params![bytes, hash(&bytes).as_slice()],
                )
                .unwrap();
            }
            5 => {
                raw.execute_batch("DROP INDEX inbox_original_operations_item;")
                    .unwrap();
            }
            6 => {
                raw.execute_batch("CREATE TRIGGER bad_original AFTER INSERT ON inbox_original_operations BEGIN SELECT 1; END;").unwrap();
            }
            7 => {
                raw.execute("DELETE FROM inbox_items", []).unwrap();
            }
            _ => {
                raw.execute(
                    "UPDATE inbox_original_operations SET record_json=zeroblob(?1)",
                    [MAX_NEW_OPERATION_BYTES as i64 + 1],
                )
                .unwrap();
            }
        }
        drop(raw);
        let before = std::fs::read(&db).unwrap();
        let backups = std::fs::read_dir(dir.path().join("backups"))
            .unwrap()
            .count();
        assert!(WorkStore::open(dir.path()).is_err(), "mode {mode}, {id}");
        assert_eq!(before, std::fs::read(&db).unwrap());
        assert_eq!(
            backups,
            std::fs::read_dir(dir.path().join("backups"))
                .unwrap()
                .count()
        );
    }
}
#[test]
fn streaming_error_and_selection_errors_do_not_commit_partial_family() {
    let (_owner, mut creator) = fixture();
    let w = witness(&mut creator, InboxKind::Email, "body");
    let r = removal(&w, None, w.original.received_at_ms);
    let id = r.summary().unwrap().operation_id;
    let (_dir, mut store) = fixture();
    let records = vec![
        Ok(r.clone()),
        Err(brn_store::Error::Invalid("synthetic iterator fault".into())),
    ];
    assert!(
        store
            .restore_inbox_original_operation_records(records, &[])
            .is_err()
    );
    assert!(store.inbox_item(w.original.capture.id).unwrap().is_none());
    assert!(
        store
            .restore_inbox_original_operations(std::slice::from_ref(&r), &[Uuid::new_v4()])
            .is_err()
    );
    assert!(store.inbox_item(w.original.capture.id).unwrap().is_none());
    store.restore_inbox_original_operations(&[r], &[]).unwrap();
    assert!(store.inbox_original_operations(&[id, id]).is_err());
    assert!(
        store
            .inbox_original_operations(&vec![Uuid::new_v4(); 129])
            .is_err()
    );
}
#[test]
fn inventory_parses_each_body_once_and_streams_more_than_64mib() {
    use std::time::Instant;
    let (_owner, mut creator) = fixture();
    let w = witness(&mut creator, InboxKind::Text, "body");
    let root = removal(&w, None, w.original.received_at_ms);
    let mut records = vec![root];
    for i in 1..256 {
        let parent = records.last().unwrap();
        let at = w.original.received_at_ms + i;
        let next = if i % 2 == 1 {
            restore(parent, at)
        } else {
            removal(
                &w,
                Some(InboxOriginalParent {
                    operation_id: parent.summary().unwrap().operation_id,
                    record_sha256: parent.summary().unwrap().record_sha256,
                }),
                at,
            )
        };
        records.push(next);
    }
    let (_dir, mut store) = fixture();
    let began = Instant::now();
    store
        .restore_inbox_original_operation_records(records.into_iter().rev().map(Ok), &[])
        .unwrap();
    let imported = began.elapsed();
    let began = Instant::now();
    let page = store.inbox_original_operations(&[]).unwrap();
    assert_eq!(page.parsed_bodies, 256);
    assert_eq!(page.history.len(), 256);
    eprintln!(
        "synthetic 256-row family import={imported:?}, checked inventory={:?}, full bodies={}",
        began.elapsed(),
        page.parsed_bodies
    );
    // Four independent valid near-limit escaped witnesses exceed 64 MiB overall.
    // Iterator creates and drops one full operation at a time; no retained family bodies.
    let (_owner, mut creator) = fixture();
    let text = "\u{0001}".repeat(brn_store::MAX_NOTE_BYTES - 2048);
    let w = witness(&mut creator, InboxKind::Text, &text);
    let base = removal(&w, None, w.original.received_at_ms);
    let body_len = serde_json::to_vec(&base).unwrap().len();
    assert!(body_len * 4 > MAX_ORIGINAL_OPERATION_BYTES);
    let (_dir, mut store) = fixture();
    let began = Instant::now();
    let mut parent: Option<InboxOriginalOperation> = None;
    // One four-Remove alternating chain, with small Restores, reuses one original.
    let records = (0..7).map(|i| {
        let next = if i == 0 {
            base.clone()
        } else if i % 2 == 1 {
            restore(parent.as_ref().unwrap(), w.original.received_at_ms + i)
        } else {
            let p = parent.as_ref().unwrap().summary().unwrap();
            removal(
                &w,
                Some(InboxOriginalParent {
                    operation_id: p.operation_id,
                    record_sha256: p.record_sha256,
                }),
                w.original.received_at_ms + i,
            )
        };
        parent = Some(next.clone());
        Ok(next)
    });
    let page = store
        .restore_inbox_original_operation_records(records, &[])
        .unwrap();
    assert_eq!(page.parsed_bodies, 7);
    assert_eq!(page.history.len(), 7);
    eprintln!(
        "synthetic escaped operation={body_len} bytes, four Removes={} bytes, streamed import={:?}",
        body_len * 4,
        began.elapsed()
    );
    let mut visited = 0;
    let began_visit = Instant::now();
    let visit = store
        .visit_inbox_original_operations::<brn_store::Error>(|_| {
            visited += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(visited, 7);
    assert_eq!(visit.parsed_bodies, 14);
    assert!(visit.selected.is_empty());
    eprintln!(
        "synthetic escaped streamed visit={:?}, full bodies={}",
        began_visit.elapsed(),
        visit.parsed_bodies
    );
    let ids = page
        .history
        .iter()
        .filter(|s| s.kind == InboxOriginalOperationKind::Remove)
        .map(|s| s.operation_id)
        .collect::<Vec<_>>();
    assert!(
        store.inbox_original_operations(&ids).is_err(),
        "returned-body aggregate still refuses >64 MiB"
    );
    let began = Instant::now();
    let page = store.inbox_original_operations(&[]).unwrap();
    assert_eq!(page.parsed_bodies, 7);
    eprintln!(
        "synthetic escaped checked inventory={:?}, full bodies={}",
        began.elapsed(),
        page.parsed_bodies
    );
}
#[test]
fn checked_visit_is_two_pass_snapshot_and_no_callbacks_precede_validation() {
    let (_owner, mut creator) = fixture();
    let w = witness(&mut creator, InboxKind::Text, "body");
    let a = removal(&w, None, w.original.received_at_ms);
    let b = restore(&a, w.original.received_at_ms + 1);
    let bid = b.summary().unwrap().operation_id;
    let (dir, mut store) = fixture();
    store
        .restore_inbox_original_operations(&[b.clone(), a.clone()], &[])
        .unwrap();
    let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
    let mut ids = Vec::new();
    let page = store
        .visit_inbox_original_operations::<brn_store::Error>(|op| {
            ids.push(op.summary()?.operation_id);
            if ids.len() == 1 {
                raw.execute(
                    "UPDATE inbox_original_operations SET kind='damaged' WHERE id=?1",
                    [bid.to_string()],
                )?;
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(ids, [a.summary().unwrap().operation_id, bid]);
    assert_eq!(page.parsed_bodies, 4);
    assert!(page.selected.is_empty());
    let mut called = 0;
    assert!(
        store
            .visit_inbox_original_operations::<brn_store::Error>(|_| {
                called += 1;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(
        called, 0,
        "unrelated second-body damage is checked before any consumer call"
    );
    raw.execute(
        "UPDATE inbox_original_operations SET kind='restore' WHERE id=?1",
        [bid.to_string()],
    )
    .unwrap();
    #[derive(Debug)]
    enum ConsumerError {
        Store(brn_store::Error),
        Stop,
    }
    impl From<brn_store::Error> for ConsumerError {
        fn from(e: brn_store::Error) -> Self {
            Self::Store(e)
        }
    }
    let error = store
        .visit_inbox_original_operations::<ConsumerError>(|_| Err(ConsumerError::Stop))
        .unwrap_err();
    assert!(matches!(error, ConsumerError::Stop));
    let error = store
        .visit_inbox_original_operations::<ConsumerError>(|_| Ok(()))
        .unwrap();
    assert_eq!(error.parsed_bodies, 4);
    // Ensure the conversion variant carries its checked Store error, too.
    raw.execute("UPDATE inbox_original_operations SET kind='damaged'", [])
        .unwrap();
    let error = store
        .visit_inbox_original_operations::<ConsumerError>(|_| Ok(()))
        .unwrap_err();
    let ConsumerError::Store(error) = error else {
        panic!("Store error")
    };
    assert!(matches!(error, brn_store::Error::Invalid(_)));
}

#[test]
fn intent_terminal_replay_never_rewrites_immutable_witness_or_clocks() {
    let (_owner, mut creator) = fixture();
    let w = witness(&mut creator, InboxKind::Text, "body");
    let settled = removal(&w, None, w.original.received_at_ms);
    let id = settled.summary().unwrap().operation_id;
    let mut intent = settled.clone();
    let InboxOriginalOperation::Remove(r) = &mut intent else {
        unreachable!()
    };
    r.removed_at_ms = None;
    let (_dir, mut store) = fixture();
    store
        .restore_inbox_original_operations(std::slice::from_ref(&intent), &[])
        .unwrap();
    let result = store
        .restore_inbox_original_operations(std::slice::from_ref(&settled), &[id])
        .unwrap();
    assert_eq!(result.parsed_bodies, 3);
    assert_eq!(
        result.selected[0].digest().unwrap(),
        settled.digest().unwrap()
    );
    let replay = store
        .restore_inbox_original_operations(&[intent], &[id])
        .unwrap();
    assert_eq!(replay.parsed_bodies, 3);
    assert_eq!(
        replay.selected[0].digest().unwrap(),
        settled.digest().unwrap()
    );
    let mut changed = settled.clone();
    let InboxOriginalOperation::Remove(r) = &mut changed else {
        unreachable!()
    };
    let source = r.evidence.source.as_mut().unwrap();
    source.saved.text = source
        .saved
        .text
        .replacen("---\n", "---\nowner: exact\n", 1);
    source.saved.source.fingerprint =
        fingerprint(&source.saved.text, source.saved.source.fingerprint.inode);
    r.request.preview_digest = r.evidence.digest().unwrap();
    changed.validate().unwrap();
    assert!(matches!(
        store.restore_inbox_original_operations(&[changed], &[]),
        Err(brn_store::Error::OperationConflict(_))
    ));
    assert_eq!(
        store.inbox_original_operations(&[id]).unwrap().selected[0]
            .digest()
            .unwrap(),
        settled.digest().unwrap()
    );
}

#[test]
fn binary_forged_new_remove_and_restore_records_are_unsupported() {
    let (_owner, mut store) = fixture();
    let w = witness(&mut store, InboxKind::Text, "body");
    let remove = removal(&w, None, w.original.received_at_ms);
    let mut restored = restore(&remove, w.original.received_at_ms);
    let InboxOriginalOperation::Restore(record) = &mut restored else {
        unreachable!()
    };
    record.original.capture.kind = InboxKind::Binary;
    assert!(
        record
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Binary")
    );
    let InboxOriginalOperation::Remove(mut record) = remove else {
        unreachable!()
    };
    record.evidence.item.capture.kind = InboxKind::Binary;
    assert!(
        record
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Binary")
    );
    assert!(record.evidence.digest().is_err());
    let (_fresh, mut target) = fixture();
    assert!(
        target
            .restore_inbox_original_operations(&[InboxOriginalOperation::Remove(record)], &[])
            .is_err()
    );
    assert!(target.inbox_item(w.original.capture.id).unwrap().is_none());
}
