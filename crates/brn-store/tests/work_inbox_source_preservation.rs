use brn_store::{
    WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind},
        inbox_processing::{InboxConversionFormat, InboxProcessOutcome},
        inbox_source::{InboxSourceBinding, InboxSourcePreservation, convert_original},
        proposal_apply::{
            ApplyJournal, ApplyMemberProof, ApplyOutcome, ApprovalRequest, UndoRequest,
        },
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
    fn proof(&self) -> InboxSourcePreservation<'_> {
        InboxSourcePreservation {
            original: &self.original,
            original_text: &self.original_text,
            approval: &self.approval,
            saved: &self.saved,
            saved_text: &self.saved_text,
        }
    }
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

#[test]
fn converter_preserves_all_kinds_exact_bytes_and_safe_fences() {
    for (text, literal) in [
        ("", "```text\n\n```\n"),
        ("body", "```text\nbody\n```\n"),
        ("body\r\n", "```text\nbody\r\n```\n"),
        (
            "\u{feff}õ 🦀\r\n```\n~~~~",
            "````text\n\u{feff}õ 🦀\r\n```\n~~~~\n````\n",
        ),
        ("``````\n~~~\n", "~~~~text\n``````\n~~~\n~~~~\n"),
        (
            "---\nbrn_id: imported\n---\n",
            "```text\n---\nbrn_id: imported\n---\n```\n",
        ),
    ] {
        for kind in [
            InboxKind::Text,
            InboxKind::Email,
            InboxKind::Teams,
            InboxKind::Markdown,
        ] {
            let expected = if kind == InboxKind::Markdown {
                (InboxConversionFormat::VerbatimMarkdownV1, text)
            } else {
                (InboxConversionFormat::LiteralTextV1, literal)
            };
            let (format, actual) = convert_original(kind, text, &AtomicBool::new(false)).unwrap();
            assert_eq!((format, actual.as_str()), expected);
            assert_eq!(
                convert_original(kind, text, &AtomicBool::new(true)),
                Err(InboxProcessOutcome::Cancelled)
            );
        }
    }
    let exact = "x".repeat(brn_store::MAX_NOTE_BYTES);
    assert_eq!(
        convert_original(InboxKind::Markdown, &exact, &AtomicBool::new(false))
            .unwrap()
            .1,
        exact
    );
    assert_eq!(
        convert_original(InboxKind::Text, &exact, &AtomicBool::new(false)),
        Err(InboxProcessOutcome::Failed {
            code: "candidate_too_large".into()
        })
    );
    let boundary = "x".repeat(brn_store::MAX_NOTE_BYTES - 13);
    assert_eq!(
        convert_original(InboxKind::Text, &boundary, &AtomicBool::new(false))
            .unwrap()
            .1
            .len(),
        brn_store::MAX_NOTE_BYTES
    );
    assert!(convert_original(InboxKind::Text, &(boundary + "x"), &AtomicBool::new(false)).is_err());
}

#[test]
fn preservation_accepts_all_kinds_current_history_archive_and_fresh_headers() {
    for kind in [
        InboxKind::Text,
        InboxKind::Email,
        InboxKind::Teams,
        InboxKind::Markdown,
    ] {
        let (_dir, mut store) = fixture();
        let mut w = witness(
            &mut store,
            kind,
            "\u{feff}---\r\nbrn_id: imported\r\n---\r\nõ 🦀\r\n````\r\n",
        );
        w.proof().validate().unwrap();
        let binding = w
            .approval
            .approved
            .draft
            .inbox_source
            .as_ref()
            .unwrap()
            .clone();
        let header_end = w.saved_text.find("\n---\n").unwrap() + 5;
        let body = w.saved_text[header_end..].to_owned();
        w.saved_text = format!(
            "\u{feff}---\r\nowner: 'Exact extra metadata'\r\nbrn_id: '{}'\r\nbrn_kind: \"source\" # saved evidence\r\nbrn_state: 'history'\r\nbrn_inbox_source: {}\r\n---\r\n{body}",
            binding.note_id,
            serde_json::to_string(&binding.provenance()).unwrap()
        );
        w.saved.path = "archive/kept source.MD".into();
        w.saved.fingerprint.inode = 99;
        w.rebind_saved();
        w.proof().validate().unwrap();
        assert!(
            binding.validate_markdown(&w.saved_text).is_err(),
            "proposal admission remains strict"
        );
        w.saved_text = w
            .saved_text
            .replace("brn_state: 'history'", "brn_state: 'current'");
        w.rebind_saved();
        w.proof().validate().unwrap();
    }
}

#[test]
fn preservation_refuses_original_saved_and_approval_proof_damage() {
    let (_dir, mut store) = fixture();
    let valid = witness(&mut store, InboxKind::Text, "Exact õ\r\n");
    for mode in 0..18 {
        let mut original = valid.original.clone();
        let mut original_text = valid.original_text.clone();
        let mut approval = valid.approval.clone();
        let mut saved = valid.saved.clone();
        let mut saved_text = valid.saved_text.clone();
        match mode {
            0 => original.capture.title.push('x'),
            1 => original.capture.copy.file_inode += 1,
            2 => original.received_at_ms += 1,
            3 => original_text.push('x'),
            4 => saved.fingerprint.len += 1,
            5 => saved.fingerprint.sha256[0] ^= 1,
            6 => saved.path = "../escape.md".into(),
            7 => saved.path = ".hidden/source.md".into(),
            8 => saved.path = "/absolute.md".into(),
            9 => saved_text = saved_text.replace("brn_kind: source", "brn_kind: knowledge"),
            10 => saved_text = saved_text.replace("brn_state: current", "brn_state: uncertain"),
            11 => {
                saved_text = saved_text.replace(
                    &approval
                        .approved
                        .draft
                        .inbox_source
                        .as_ref()
                        .unwrap()
                        .note_id
                        .to_string(),
                    &Uuid::new_v4().to_string(),
                )
            }
            12 => saved_text.push('x'),
            13 => {
                let provenance = approval
                    .approved
                    .draft
                    .inbox_source
                    .as_ref()
                    .unwrap()
                    .provenance();
                let mut changed = provenance.clone();
                changed.original_sha256[0] ^= 1;
                saved_text = saved_text.replace(
                    &serde_json::to_string(&provenance).unwrap(),
                    &serde_json::to_string(&changed).unwrap(),
                );
            }
            14 => approval.receipt = None,
            15 => approval.request.expected.version += 1,
            16 => {
                approval.observations.as_mut().unwrap()[0]
                    .destination
                    .as_mut()
                    .unwrap()
                    .inode += 1
            }
            _ => approval.approved.draft.inbox_source = None,
        }
        if (9..=13).contains(&mode) {
            saved.fingerprint = fingerprint(&saved_text, saved.fingerprint.inode);
        }
        assert!(
            InboxSourcePreservation {
                original: &original,
                original_text: &original_text,
                approval: &approval,
                saved: &saved,
                saved_text: &saved_text
            }
            .validate()
            .is_err(),
            "damage mode {mode}"
        );
    }
}

#[test]
fn preservation_reconstructs_original_instead_of_trusting_consistent_conversion_claims() {
    let (_dir, mut store) = fixture();
    let mut w = witness(&mut store, InboxKind::Email, "Original body");
    let binding = w.approval.approved.draft.inbox_source.as_mut().unwrap();
    let invented = "```text\nDifferent body\n```\n";
    binding.byte_len = invented.len() as u64;
    binding.sha256 = hash(invented.as_bytes());
    let text = binding.markdown(invented).unwrap();
    let NoteChange::Create { text: approved, .. } = &mut w.approval.approved.draft.changes[0]
    else {
        unreachable!()
    };
    *approved = text.clone();
    w.approval.creation_sha256 = hash(&serde_json::to_vec(&w.approval.approved.draft).unwrap());
    let proof = fingerprint(&text, 5);
    w.approval.prepared = Some(vec![proof.clone()]);
    w.approval.observations.as_mut().unwrap()[0].destination = Some(proof.clone());
    w.saved.fingerprint = proof;
    w.saved_text = text;
    w.approval.validate().unwrap();
    assert!(
        w.proof().validate().is_err(),
        "a coherent approval/receipt does not prove original conversion"
    );
}

#[test]
fn preservation_rejects_valid_not_applied_and_undo_journals() {
    let (_dir, mut store) = fixture();
    let w = witness(&mut store, InboxKind::Markdown, "# Original\n");
    let undo = store
        .begin_proposal_undo(&UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: w.approval.request.operation_id,
            trash_member: None,
        })
        .unwrap();
    store
        .refuse_proposal_before_effects(undo.request.operation_id, None)
        .unwrap();
    let undo = store
        .proposal_apply(undo.request.operation_id)
        .unwrap()
        .unwrap();
    undo.validate().unwrap();
    assert!(
        InboxSourcePreservation {
            approval: &undo,
            ..w.proof()
        }
        .validate()
        .is_err()
    );
    let mut refused = w.approval.clone();
    refused.receipt.as_mut().unwrap().outcome = ApplyOutcome::NotApplied;
    refused.observations.as_mut().unwrap()[0].destination = None;
    refused.validate().unwrap();
    assert!(
        InboxSourcePreservation {
            approval: &refused,
            ..w.proof()
        }
        .validate()
        .is_err()
    );
    let mut uncertain = w.approval.clone();
    uncertain.receipt.as_mut().unwrap().outcome = ApplyOutcome::Uncertain;
    uncertain.validate().unwrap();
    assert!(
        InboxSourcePreservation {
            approval: &uncertain,
            ..w.proof()
        }
        .validate()
        .is_err()
    );
    let applied_undo = store
        .begin_proposal_undo(&UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: w.approval.request.operation_id,
            trash_member: None,
        })
        .unwrap();
    let NoteChange::Trash { before, .. } = &applied_undo.approved.draft.changes[0] else {
        unreachable!()
    };
    store
        .record_proposal_prepared(
            applied_undo.request.operation_id,
            std::slice::from_ref(before),
        )
        .unwrap();
    store
        .finish_proposal_apply(
            applied_undo.request.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: None,
                staging: Some(before.clone()),
            }]),
        )
        .unwrap();
    let applied_undo = store
        .proposal_apply(applied_undo.request.operation_id)
        .unwrap()
        .unwrap();
    applied_undo.validate().unwrap();
    assert!(
        InboxSourcePreservation {
            approval: &applied_undo,
            ..w.proof()
        }
        .validate()
        .is_err()
    );
    assert_eq!(
        store
            .inbox_source_approval_ids(w.original.capture.id)
            .unwrap(),
        vec![w.approval.request.operation_id],
        "inventory retains historical approval, not current filesystem availability"
    );
}

#[test]
fn preservation_does_not_relax_strict_source_proposal_admission() {
    let (_dir, mut store) = fixture();
    let mut w = witness(&mut store, InboxKind::Text, "Exact body");
    w.saved_text = w
        .saved_text
        .replace("brn_state: current", "brn_state: history");
    w.rebind_saved();
    w.proof().validate().unwrap();
    let mut draft = w.approval.approved.draft.clone();
    draft.id = Uuid::new_v4();
    let NoteChange::Create { text, .. } = &mut draft.changes[0] else {
        unreachable!()
    };
    *text = w.saved_text;
    assert!(store.create_proposal(&draft).is_err());
}

#[test]
fn checked_source_inventory_excludes_running_refused_and_unbound_approvals() {
    let (_dir, mut store) = fixture();
    let w = witness(&mut store, InboxKind::Text, "Exact body");
    let mut draft = w.approval.approved.draft.clone();
    draft.id = Uuid::new_v4();
    let review = store.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    assert_eq!(
        store
            .inbox_source_approval_ids(w.original.capture.id)
            .unwrap(),
        vec![w.approval.request.operation_id]
    );
    store
        .refuse_proposal_before_effects(request.operation_id, None)
        .unwrap();
    assert_eq!(
        store
            .inbox_source_approval_ids(w.original.capture.id)
            .unwrap(),
        vec![w.approval.request.operation_id]
    );
    draft.id = Uuid::new_v4();
    draft.inbox_source = None;
    let review = store.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    store
        .record_proposal_prepared(
            request.operation_id,
            std::slice::from_ref(&w.saved.fingerprint),
        )
        .unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(w.saved.fingerprint.clone()),
                staging: None,
            }]),
        )
        .unwrap();
    assert_eq!(
        store
            .inbox_source_approval_ids(w.original.capture.id)
            .unwrap(),
        vec![w.approval.request.operation_id]
    );
}

#[test]
fn checked_source_inventory_returns_sorted_matching_applied_ids_and_survives_restart() {
    let (dir, mut store) = fixture();
    let a = witness(&mut store, InboxKind::Text, "a");
    let other = witness(&mut store, InboxKind::Text, "other");
    let mut another = a.approval.clone();
    another.request.operation_id = Uuid::new_v4();
    another.receipt.as_mut().unwrap().operation_id = another.request.operation_id;
    store.restore_proposal_apply(&another).unwrap();
    let undo = store
        .begin_proposal_undo(&UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: a.approval.request.operation_id,
            trash_member: None,
        })
        .unwrap();
    store
        .refuse_proposal_before_effects(undo.request.operation_id, None)
        .unwrap();
    let mut expected = vec![
        a.approval.request.operation_id,
        another.request.operation_id,
    ];
    expected.sort();
    assert_eq!(
        store
            .inbox_source_approval_ids(a.original.capture.id)
            .unwrap(),
        expected
    );
    assert_eq!(
        store
            .inbox_source_approval_ids(other.original.capture.id)
            .unwrap(),
        vec![other.approval.request.operation_id]
    );
    assert!(
        store
            .inbox_source_approval_ids(Uuid::new_v4())
            .unwrap()
            .is_empty()
    );
    assert!(store.inbox_source_approval_ids(Uuid::nil()).is_err());
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store
            .inbox_source_approval_ids(a.original.capture.id)
            .unwrap(),
        expected
    );
}

#[test]
fn checked_source_inventory_does_not_hide_unrelated_corrupt_rows() {
    for semantic in [false, true] {
        let (dir, mut store) = fixture();
        let target = witness(&mut store, InboxKind::Text, "target");
        let other = witness(&mut store, InboxKind::Text, "unrelated");
        let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
        let mut bad = other.approval.clone();
        bad.receipt.as_mut().unwrap().approved_version += 1;
        let bytes = serde_json::to_vec(&bad).unwrap();
        let digest = if semantic { hash(&bytes) } else { [0; 32] };
        raw.execute(
            "UPDATE proposal_applies SET journal_json=?1,journal_sha256=?2 WHERE operation_id=?3",
            params![
                bytes,
                digest.as_slice(),
                other.approval.request.operation_id.to_string()
            ],
        )
        .unwrap();
        assert!(
            store
                .inbox_source_approval_ids(target.original.capture.id)
                .is_err(),
            "semantic damage={semantic}"
        );
    }
}

#[test]
fn binary_source_preservation_cannot_reuse_a_text_approval() {
    let (_dir, mut store) = fixture();
    let mut w = witness(&mut store, InboxKind::Text, "synthetic exact body");
    w.original.capture.kind = InboxKind::Binary;
    assert!(
        InboxSourcePreservation {
            original: &w.original,
            original_text: &w.original_text,
            approval: &w.approval,
            saved: &w.saved,
            saved_text: &w.saved_text,
        }
        .validate()
        .unwrap_err()
        .to_string()
        .contains("Binary")
    );
}
