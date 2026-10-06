use brn_store::{
    WorkStore,
    files::{VaultIdentity, VaultRecord},
    work::proposals::*,
};
use serde_json::json;
use uuid::Uuid;

fn parent() -> VaultIdentity {
    VaultIdentity {
        device: 1,
        inode: 2,
    }
}
fn draft(changes: Vec<NoteChange>) -> ProposalDraft {
    ProposalDraft {
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: Some(VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: parent(),
        }),
        title: "Complete opaque bytes".into(),
        changes,
        sources: vec![],
        action_changes: vec![],
    }
}
#[test]
fn asset_codec_retains_non_utf8_and_empty_bytes_without_changing_markdown_wire() {
    for payload in ["AP8=", ""] {
        let wire = json!({"kind":"create_asset", "path":"assets/original.bin", "parent":{"device":1,"inode":2}, "bytes":payload});
        let asset: NoteChange =
            serde_json::from_value(wire.clone()).expect("typed opaque asset must deserialize");
        assert_eq!(serde_json::to_value(&asset).unwrap(), wire);
        assert_eq!(asset.text(), None);
        let directory = tempfile::tempdir().unwrap();
        let (mut store, _) = WorkStore::open(directory.path()).unwrap();
        let request = draft(vec![asset]);
        let review = store.create_proposal(&request).unwrap();
        assert_eq!(review.draft, request);
        drop(store);
        let (store, _) = WorkStore::open(directory.path()).unwrap();
        assert_eq!(store.proposal(request.id).unwrap(), Some(review));
    }
    let legacy =
        r#"{"kind":"create","path":"note.md","parent":{"device":1,"inode":2},"text":"Exact\r\n"}"#;
    let value: NoteChange = serde_json::from_str(legacy).unwrap();
    assert_eq!(serde_json::to_string(&value).unwrap(), legacy);
}
#[test]
fn asset_codec_refuses_noncanonical_or_foreign_payloads() {
    for payload in [
        json!([0, 255]),
        json!("AP8"),
        json!("AP9="),
        json!("AP8=\n"),
        json!("AP8=="),
        json!("-P8="),
    ] {
        let wire = json!({"kind":"create_asset", "path":"asset.bin", "parent":{"device":1,"inode":2}, "bytes":payload});
        assert!(serde_json::from_value::<NoteChange>(wire).is_err());
    }
}

use brn_store::{
    Error,
    files::FileFingerprint,
    work::{proposal_apply::*, proposal_rewrite::*},
};
use sha2::{Digest, Sha256};
fn proof(bytes: &[u8], inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: bytes.len() as u64,
        sha256: Sha256::digest(bytes).into(),
    }
}
fn create(bytes: Vec<u8>) -> NoteChange {
    NoteChange::CreateAsset {
        path: "assets/create.bin".into(),
        parent: parent(),
        bytes,
    }
}
fn replace(before_bytes: Vec<u8>, bytes: Vec<u8>) -> NoteChange {
    NoteChange::ReplaceAsset {
        path: "assets/replace.bin".into(),
        parent: parent(),
        before: proof(&before_bytes, 3),
        before_bytes,
        bytes,
    }
}
fn trash(before_bytes: Vec<u8>) -> NoteChange {
    NoteChange::TrashAsset {
        path: "assets/trash.bin".into(),
        parent: parent(),
        before: proof(&before_bytes, 4),
        before_bytes,
    }
}
fn begin(store: &mut WorkStore, draft: &ProposalDraft) -> ApplyJournal {
    let record = store.create_proposal(draft).unwrap();
    store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        })
        .unwrap()
}
fn prepared(journal: &ApplyJournal) -> Vec<FileFingerprint> {
    journal
        .approved
        .draft
        .changes
        .iter()
        .enumerate()
        .map(|(i, change)| {
            if let Some(original) = journal.undo.as_ref().and_then(|u| u.originals[i].as_ref()) {
                original.fingerprint.clone()
            } else if let Some(bytes) = change.candidate_bytes() {
                proof(bytes, 100 + i as u64)
            } else {
                change.before().unwrap().clone()
            }
        })
        .collect()
}
fn observations(journal: &ApplyJournal, applied: bool) -> Vec<ApplyMemberProof> {
    journal
        .approved
        .draft
        .changes
        .iter()
        .zip(journal.prepared.as_ref().unwrap())
        .enumerate()
        .map(|(i, (change, new))| {
            if applied {
                ApplyMemberProof {
                    destination: change.candidate_bytes().map(|_| new.clone()),
                    staging: change.before().cloned(),
                }
            } else {
                ApplyMemberProof {
                    destination: change.before().cloned(),
                    staging: journal
                        .undo
                        .as_ref()
                        .and_then(|u| u.originals[i].as_ref())
                        .map(|o| o.fingerprint.clone())
                        .or_else(|| change.candidate_bytes().map(|_| new.clone())),
                }
            }
        })
        .collect()
}
fn complete(store: &mut WorkStore, journal: ApplyJournal) -> ApplyJournal {
    let prepared = prepared(&journal);
    let journal = store
        .record_proposal_prepared(journal.request.operation_id, &prepared)
        .unwrap();
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observations(&journal, true)),
        )
        .unwrap();
    store
        .proposal_apply(journal.request.operation_id)
        .unwrap()
        .unwrap()
}
fn edit(record: &ProposalRecord) -> ProposalEdit {
    ProposalEdit {
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: record
            .draft
            .changes
            .iter()
            .map(|c| c.text().map(str::to_owned))
            .collect(),
        action_data: record
            .draft
            .action_changes
            .iter()
            .map(|change| change.data().clone())
            .collect(),
    }
}
#[test]
fn codec_exact_max_and_raw_bounds_are_checked_in_both_directions() {
    let member = create(vec![255; MAX_ASSET_BYTES]);
    let encoded = serde_json::to_string(&member).unwrap();
    assert!(encoded.contains("\"bytes\":\"////"));
    let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(value["bytes"].as_str().unwrap().len(), 22_369_624);
    assert_eq!(
        serde_json::from_str::<NoteChange>(&encoded).unwrap(),
        member
    );
    assert!(serde_json::to_vec(&create(vec![0; MAX_ASSET_BYTES + 1])).is_err());
    for length in [22_369_624, 22_369_628] {
        // The first is within the encoded cap but would decode to raw cap+2.
        let value = json!({"kind":"create_asset","path":"a.bin","parent":{"device":1,"inode":2},"bytes":"A".repeat(length)});
        assert!(serde_json::from_value::<NoteChange>(value).is_err());
    }
}
#[test]
fn asset_paths_domains_sources_and_before_proofs_refuse_without_sql_work() {
    let directory = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(directory.path()).unwrap();
    for path in [
        "",
        "a.MD",
        "/a.bin",
        "../a.bin",
        "x/../a.bin",
        ".brn/a.bin",
        "x/.hidden",
        "x//a.bin",
        "x\\a.bin",
        "x\0a.bin",
    ] {
        assert!(validate_asset_path(path).is_err());
        let mut member = create(vec![0, 255]);
        if let NoteChange::CreateAsset { path: p, .. } = &mut member {
            *p = path.into();
        }
        let draft = draft(vec![member]);
        assert!(store.create_proposal(&draft).is_err());
        assert!(store.proposal(draft.id).unwrap().is_none());
    }
    for path in [
        "assets/a.bin",
        "assets/no-extension",
        "archive/original.PDF",
    ] {
        validate_asset_path(path).unwrap();
    }
    let mut wrong = replace(vec![0, 255], vec![1]);
    if let NoteChange::ReplaceAsset { before, .. } = &mut wrong {
        before.sha256 = [0; 32];
    }
    let d = draft(vec![wrong]);
    assert!(store.create_proposal(&d).is_err());
    assert!(store.proposal(d.id).unwrap().is_none());
    let mut d = draft(vec![create(vec![0])]);
    d.sources.push(SourceVersion {
        path: "source.md".into(),
        fingerprint: proof(&vec![0; brn_store::MAX_NOTE_BYTES + 1], 55),
    });
    assert!(store.create_proposal(&d).is_err());
    d.sources[0].path = "source.bin".into();
    d.sources[0].fingerprint = proof(&[0], 55);
    assert!(store.create_proposal(&d).is_err());
    let d = draft(vec![NoteChange::Create {
        path: "still.md".into(),
        parent: parent(),
        text: "a".repeat(brn_store::MAX_NOTE_BYTES + 1),
    }]);
    assert!(store.create_proposal(&d).is_err());
    let mut duplicate = trash(vec![1]);
    if let NoteChange::TrashAsset { before, .. } = &mut duplicate {
        before.inode = 3;
    }
    assert!(
        store
            .create_proposal(&draft(vec![replace(vec![1], vec![2]), duplicate]))
            .is_err()
    );
}
#[test]
fn asset_null_edit_and_owned_rewrite_preserve_full_capture_bytes_and_bindings() {
    let directory = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(directory.path()).unwrap();
    let d = draft(vec![
        create(vec![0, 255]),
        replace(vec![128], vec![129]),
        trash(vec![]),
        NoteChange::Create {
            path: "note.md".into(),
            parent: parent(),
            text: "body".into(),
        },
    ]);
    let mut d = d;
    d.action_changes.push(ActionChange::Create {
        id: Uuid::new_v4(),
        data: action_data(String::new()),
    });
    let record = store.create_proposal(&d).unwrap();
    let mut result = edit(&record);
    result.texts[0] = Some("fake bytes".into());
    assert!(validate_result(&record, &result).is_err());
    assert!(store.edit_proposal(&result).is_err());
    result.texts[0] = None;
    result.texts[3] = Some("rewritten".into());
    result.action_data[0].description = "Rewritten Action".into();
    let spec = RewriteSpec {
        id: Uuid::new_v4(),
        expected: record.stamp(),
        provider: "chatgpt".into(),
        model: "synthetic".into(),
        effort: "high".into(),
    };
    let (job, capture) = store.begin_proposal_rewrite(&spec).unwrap();
    assert_eq!(capture, Some(record.clone()));
    assert_eq!(
        job.capture_sha256,
        <[u8; 32]>::from(Sha256::digest(serde_json::to_vec(&record).unwrap()))
    );
    let completed = store
        .finish_proposal_rewrite(spec.id, &RewriteOutcome::Completed(result))
        .unwrap();
    assert_eq!(completed.status, RewriteStatus::Completed);
    let edited = store.proposal(d.id).unwrap().unwrap();
    assert_eq!(&edited.draft.changes[..3], &record.draft.changes[..3]);
    assert_eq!(edited.draft.changes[3].text(), Some("rewritten"));
    assert_eq!(
        edited.draft.action_changes[0].data().description,
        "Rewritten Action"
    );
    assert_eq!(store.create_proposal(&d).unwrap(), edited);
    let mut fork = d;
    if let NoteChange::CreateAsset { bytes, .. } = &mut fork.changes[0] {
        bytes[0] = 1;
    }
    assert!(matches!(
        store.create_proposal(&fork),
        Err(Error::OperationConflict(_))
    ));
}
#[test]
fn asset_proof_preparation_terminal_pairs_and_note_bounds_are_exact() {
    let directory = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(directory.path()).unwrap();
    let journal = begin(
        &mut store,
        &draft(vec![
            create(vec![255; brn_store::MAX_NOTE_BYTES + 1]),
            replace(vec![0, 255], vec![2]),
            trash(vec![3]),
        ]),
    );
    let valid = prepared(&journal);
    for index in 0..3 {
        let mut damaged = valid.clone();
        damaged[index].sha256 = [0; 32];
        assert!(
            store
                .record_proposal_prepared(journal.request.operation_id, &damaged)
                .is_err()
        );
        assert_eq!(
            store.proposal_apply(journal.request.operation_id).unwrap(),
            Some(journal.clone())
        );
    }
    let mut alias = valid.clone();
    alias[0].inode = 3;
    assert!(
        store
            .record_proposal_prepared(journal.request.operation_id, &alias)
            .is_err()
    );
    let journal = store
        .record_proposal_prepared(journal.request.operation_id, &valid)
        .unwrap();
    let mut wrong = observations(&journal, true);
    wrong[1].staging = None;
    assert!(
        store
            .finish_proposal_apply(
                journal.request.operation_id,
                ApplyOutcome::Applied,
                Some(&wrong)
            )
            .is_err()
    );
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observations(&journal, true)),
        )
        .unwrap();
    let terminal = store
        .proposal_apply(journal.request.operation_id)
        .unwrap()
        .unwrap();
    terminal.validate().unwrap();
    let mut damaged = terminal;
    damaged.observations.as_mut().unwrap()[2]
        .staging
        .as_mut()
        .unwrap()
        .len = (MAX_ASSET_BYTES + 1) as u64;
    assert!(damaged.validate().is_err());
}
#[test]
fn separate_asset_raw_and_encoded_mixed_budget_refuse_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(directory.path()).unwrap();
    let mut d = draft(vec![
        replace(vec![255; MAX_ASSET_BYTES], vec![0; MAX_ASSET_BYTES]),
        create(vec![1]),
    ]);
    assert!(store.create_proposal(&d).is_err());
    assert!(store.proposal(d.id).unwrap().is_none());
    d.changes.pop();
    for i in 0..2 {
        d.changes.push(NoteChange::Create {
            path: format!("{i}.md"),
            parent: parent(),
            text: "\0".repeat(brn_store::MAX_NOTE_BYTES),
        });
    }
    // Raw asset and text budgets each fit; actual JSON exceeds old encoded cap.
    assert!(store.create_proposal(&d).is_err());
    assert!(store.proposal(d.id).unwrap().is_none());
}
#[test]
fn max_two_body_asset_and_escaped_markdown_settle_inverse_and_fresh_restore() {
    let directory = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(directory.path()).unwrap();
    let d = draft(vec![
        replace(vec![255; MAX_ASSET_BYTES], vec![0; MAX_ASSET_BYTES]),
        NoteChange::Create {
            path: "explanation.md".into(),
            parent: parent(),
            text: "\0".repeat(800_000),
        },
    ]);
    let started = begin(&mut store, &d);
    let terminal = complete(&mut store, started);
    let bytes = serde_json::to_vec(&terminal).unwrap();
    assert!(bytes.len() < 51_108_864);
    assert!(bytes.len() > 44_739_252);
    let fresh = tempfile::tempdir().unwrap();
    let (mut restored, _) = WorkStore::open(fresh.path()).unwrap();
    assert_eq!(
        restored.restore_proposal_apply(&terminal).unwrap(),
        terminal
    );
    let request = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: terminal.request.operation_id,
        trash_member: None,
    };
    let inverse = restored.begin_proposal_undo(&request).unwrap();
    let inverse = complete(&mut restored, inverse);
    inverse.validate().unwrap();
    assert_eq!(
        inverse.approved.draft.changes[0].candidate_bytes(),
        Some(vec![255; MAX_ASSET_BYTES].as_slice())
    );
    assert_eq!(
        inverse.approved.draft.changes[0].before_bytes(),
        Some(vec![0; MAX_ASSET_BYTES].as_slice())
    );
    assert!(serde_json::to_vec(&inverse).unwrap().len() < 51_108_864);
    drop(restored);
    let (restored, _) = WorkStore::open(fresh.path()).unwrap();
    assert_eq!(
        restored.proposal_apply(request.operation_id).unwrap(),
        Some(inverse)
    );
}
#[test]
fn all_asset_inverse_kinds_repeat_and_scoped_trash_is_direct() {
    let directory = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(directory.path()).unwrap();
    let first = begin(
        &mut store,
        &draft(vec![
            create(vec![255, 0]),
            replace(vec![128], vec![129]),
            trash(vec![]),
        ]),
    );
    let mut source = complete(&mut store, first);
    let scoped = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: source.request.operation_id,
        trash_member: Some(2),
    };
    let preview = store.preview_proposal_undo(&scoped).unwrap();
    assert!(
        matches!(&preview.draft.changes[..],[NoteChange::CreateAsset{bytes,..}] if bytes.is_empty())
    );
    let scoped_journal = store.begin_proposal_undo(&scoped).unwrap();
    complete(&mut store, scoped_journal);
    for cycle in 0..4 {
        let fresh = tempfile::tempdir().unwrap();
        let (mut restored, _) = WorkStore::open(fresh.path()).unwrap();
        restored.restore_proposal_apply(&source).unwrap();
        let request = UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: source.request.operation_id,
            trash_member: None,
        };
        let inverse = restored.begin_proposal_undo(&request).unwrap();
        assert_eq!(
            inverse.undo.as_ref().unwrap().operation_id,
            source.request.operation_id
        );
        let terminal = complete(&mut restored, inverse);
        let expected = if cycle % 2 == 0 { 128 } else { 129 };
        assert_eq!(
            terminal.approved.draft.changes[1].candidate_bytes(),
            Some([expected].as_slice())
        );
        assert!(
            !serde_json::to_string(&terminal)
                .unwrap()
                .contains("approved\":{\"approved")
        );
        assert_eq!(restored.begin_proposal_undo(&request).unwrap(), terminal);
        source = terminal;
    }
}
#[test]
fn sixty_four_asset_repair_attempts_and_max_width_proofs_settle_and_restore() {
    let directory = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(directory.path()).unwrap();
    let changes = (0..64)
        .map(|i| NoteChange::ReplaceAsset {
            path: format!("asset/{i}.bin"),
            parent: parent(),
            before: proof(&[128], u64::MAX - i),
            before_bytes: vec![128],
            bytes: vec![255],
        })
        .collect();
    let mut journal = begin(&mut store, &draft(changes));
    let proofs = prepared(&journal);
    journal = store
        .record_proposal_prepared(journal.request.operation_id, &proofs)
        .unwrap();
    for i in 0..64 {
        let observed = observations(&journal, i % 2 == 1);
        let preview = journal.repair_preview(&observed).unwrap();
        let request = RepairRequest {
            id: Uuid::new_v4(),
            operation_id: journal.request.operation_id,
            expected: preview.expected,
            direction: if i % 2 == 1 {
                RepairDirection::Finish
            } else {
                RepairDirection::Restore
            },
        };
        journal = store.begin_proposal_repair(&request, &observed).unwrap();
    }
    assert_eq!(journal.repair.as_ref().unwrap().attempts.len(), 64);
    let observed = observations(&journal, true);
    let preview = journal.repair_preview(&observed).unwrap();
    assert!(
        store
            .begin_proposal_repair(
                &RepairRequest {
                    id: Uuid::new_v4(),
                    operation_id: journal.request.operation_id,
                    expected: preview.expected,
                    direction: RepairDirection::Finish
                },
                &observed
            )
            .is_err()
    );
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observed),
        )
        .unwrap();
    let terminal = store
        .proposal_apply(journal.request.operation_id)
        .unwrap()
        .unwrap();
    terminal.validate().unwrap();
    let fresh = tempfile::tempdir().unwrap();
    let (mut fresh_store, _) = WorkStore::open(fresh.path()).unwrap();
    assert_eq!(
        fresh_store.restore_proposal_apply(&terminal).unwrap(),
        terminal
    );
}

#[test]
fn ordinary_asset_admission_reserves_terminal_metadata_at_large_path_boundary() {
    let directory = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(directory.path()).unwrap();
    let long = std::iter::repeat_n("p".repeat(200), 18)
        .collect::<Vec<_>>()
        .join("/");
    let changes = (0..64)
        .map(|i| NoteChange::ReplaceAsset {
            path: format!("{long}/{i}.bin"),
            parent: parent(),
            before: proof(&[128], u64::MAX - i),
            before_bytes: vec![128],
            bytes: vec![255],
        })
        .collect();
    let journal = begin(&mut store, &draft(changes));
    let prepared = prepared(&journal);
    let journal = store
        .record_proposal_prepared(journal.request.operation_id, &prepared)
        .unwrap();
    let observed = observations(&journal, true);
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observed),
        )
        .expect("admitted asset must fit its complete terminal proofs");
    let terminal = store
        .proposal_apply(journal.request.operation_id)
        .unwrap()
        .unwrap();
    terminal.validate().unwrap();
}

#[test]
fn fixed_pre_asset_journal_bytes_and_hash_arrays_remain_identical() {
    let bytes = include_bytes!("fixtures/proposal-apply-legacy-asset-guard.json");
    let journal: ApplyJournal = serde_json::from_slice(bytes).unwrap();
    journal.validate().unwrap();
    assert_eq!(serde_json::to_vec(&journal).unwrap(), bytes);
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "cea198b752e92197595bd3074fed97010cf4cf2d68b16ab906fcfc6f94e04a47"
    );
}
#[test]
fn valid_inbox_bindings_do_not_authorize_asset_shapes() {
    use brn_store::work::{inbox::*, inbox_actions::InboxKnowledgeBinding, inbox_source::*};
    use std::sync::atomic::AtomicBool;
    let directory = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(directory.path()).unwrap();
    let original = store
        .capture_inbox(&InboxCapture {
            id: Uuid::new_v4(),
            kind: InboxKind::Text,
            title: "Exact source".into(),
            original_name: None,
            copy: InboxCopy {
                directory: "/synthetic/inbox".into(),
                directory_device: 1,
                directory_inode: 2,
                file_device: 1,
                file_inode: 3,
                byte_len: 1,
                sha256: proof(b"x", 3).sha256,
            },
        })
        .unwrap();
    let (format, text) = convert_original(InboxKind::Text, "x", &AtomicBool::new(false)).unwrap();
    let source = InboxSourceBinding {
        batch_id: Uuid::new_v4(),
        index: 0,
        original,
        format,
        byte_len: text.len() as u64,
        sha256: proof(text.as_bytes(), 3).sha256,
        note_id: Uuid::new_v4(),
    };
    source
        .validate_markdown(&source.markdown(&text).unwrap())
        .unwrap();
    let knowledge = InboxKnowledgeBinding {
        analysis_id: Uuid::new_v4(),
        note_id: Uuid::new_v4(),
        source: SourceVersion {
            path: "source.md".into(),
            fingerprint: proof(b"x", 3),
        },
        supersedes: None,
        citations: vec![brn_store::note_provenance::VaultCitation {
            note_id: Uuid::new_v4(),
            sha256: proof(b"x", 3).sha256,
            start_byte: 0,
            end_byte: 1,
            quote: "x".into(),
        }],
    };
    knowledge.validate().unwrap();
    for change in [
        create(vec![0, 255]),
        replace(vec![128], vec![129]),
        trash(vec![0]),
    ] {
        let mut d = draft(vec![change]);
        d.inbox_source = Some(Box::new(source.clone()));
        assert!(store.create_proposal(&d).is_err());
        assert!(store.proposal(d.id).unwrap().is_none());
        d.inbox_source = None;
        d.inbox_knowledge = Some(Box::new(knowledge.clone()));
        d.group_id = Some(knowledge.analysis_id);
        d.sources = vec![knowledge.source.clone()];
        assert!(store.create_proposal(&d).is_err());
        assert!(store.proposal(d.id).unwrap().is_none());
    }
}

fn action_data(description: String) -> brn_store::work::actions::ActionData {
    brn_store::work::actions::ActionData {
        title: "Exact Action".into(),
        description,
        state: brn_store::work::actions::ActionState::Open,
        owner: None,
        related_person: None,
        related_project: None,
        sources: vec![],
        thread: None,
        due_on: None,
        follow_up_on: None,
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: None,
    }
}
#[test]
fn asset_admission_reserves_complete_action_snapshots_not_just_review_encoding() {
    let directory = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(directory.path()).unwrap();
    let mut d = draft(vec![replace(
        vec![0; MAX_ASSET_BYTES],
        vec![255; MAX_ASSET_BYTES],
    )]);
    d.action_changes = (0..63)
        .map(|_| ActionChange::Create {
            id: Uuid::new_v4(),
            data: action_data("a".repeat(64 * 1024)),
        })
        .collect();
    assert!(serde_json::to_vec(&d).unwrap().len() < 50_593_792);
    assert!(
        matches!(store.create_proposal(&d),Err(Error::Invalid(message)) if message.contains("encoded recovery settlement budget"))
    );
    assert!(store.proposal(d.id).unwrap().is_none());
}
