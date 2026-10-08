use super::*;

pub(crate) fn fixture() -> ProposalRecord {
    let mut record = super::tests::fixture();
    let parent = record.draft.vault.as_ref().unwrap().identity.clone();
    let replace_before = record.draft.changes[1].before().unwrap().clone();
    let before = record.draft.changes[2].before().unwrap().clone();
    record.draft.changes.truncate(1);
    record.draft.changes.extend([
        NoteChange::CreateAsset {
            path: "created.bin".into(),
            parent: parent.clone(),
            bytes: vec![0, 255, 1, 128],
        },
        NoteChange::ReplaceAsset {
            path: "replaced.bin".into(),
            parent: parent.clone(),
            before: replace_before,
            before_bytes: Vec::new(),
            bytes: vec![254, 0, 127],
        },
        NoteChange::TrashAsset {
            path: "trashed.bin".into(),
            parent,
            before,
            before_bytes: Vec::new(),
        },
    ]);
    record
}

#[test]
fn creation_acknowledgement_keeps_asset_kind_path_payload_and_expected_proof() {
    use brn_workflow::proposals::{DraftNoteChange, DraftRequest};
    let record = fixture();
    let request = DraftRequest {
        intake: None,
        id: record.draft.id,
        group_id: record.draft.group_id,
        session_id: record.draft.session_id,
        title: record.draft.title.clone(),
        changes: record
            .draft
            .changes
            .iter()
            .map(|change| match change {
                NoteChange::Create { path, text, .. } => DraftNoteChange::Create {
                    path: path.clone(),
                    text: text.clone(),
                },
                NoteChange::CreateAsset { path, bytes, .. } => DraftNoteChange::CreateAsset {
                    path: path.clone(),
                    bytes: bytes.clone(),
                },
                NoteChange::ReplaceAsset {
                    path,
                    before,
                    bytes,
                    ..
                } => DraftNoteChange::ReplaceAsset {
                    path: path.clone(),
                    expected: before.clone(),
                    bytes: bytes.clone(),
                },
                NoteChange::TrashAsset { path, before, .. } => DraftNoteChange::TrashAsset {
                    path: path.clone(),
                    expected: before.clone(),
                },
                _ => unreachable!("mixed asset fixture"),
            })
            .collect(),
        sources: record.draft.sources.clone(),
        action_changes: Vec::new(),
        inbox_source: None,
        inbox_visual: None,
        inbox_knowledge: None,
    };
    assert!(crate::draft::creation_matches(&request, &record));
    for member in 1..4 {
        let mut wrong = record.clone();
        match &mut wrong.draft.changes[member] {
            NoteChange::CreateAsset { bytes, .. } => bytes[0] ^= 1,
            NoteChange::ReplaceAsset { before, .. } => before.inode += 1,
            NoteChange::TrashAsset { path, .. } => *path = "other.bin".into(),
            _ => unreachable!(),
        }
        assert!(!crate::draft::creation_matches(&request, &wrong));
    }
    let mut wrong_kind = record.clone();
    wrong_kind.draft.changes[1] = NoteChange::Create {
        path: "created.bin".into(),
        parent: record.draft.changes[1].parent().clone(),
        text: String::new(),
    };
    assert!(!crate::draft::creation_matches(&request, &wrong_kind));
}

#[test]
fn mixed_review_preserves_asset_slots_and_refuses_text_or_anchor_reinterpretation() {
    let baseline = fixture();
    let mut review = ProposalReview::new(baseline.clone());
    for index in 1..4 {
        assert!(review.texts()[index].is_none());
        assert!(selection_target(&review, index, 0..1).is_none());
        assert!(
            review
                .edit_text(index, "text-like bytes".into(), Instant::now())
                .is_err()
        );
    }
    assert_eq!(review.record, baseline);
    review
        .edit_title("Reviewed assets".into(), Instant::now())
        .unwrap();
    review
        .edit_text(0, "Edited Markdown λ".into(), Instant::now())
        .unwrap();
    let copied: serde_json::Value = serde_json::from_str(&review.copy_local().unwrap()).unwrap();
    assert_eq!(copied["texts"].as_array().unwrap().len(), 4);
    assert!(
        copied["texts"]
            .as_array()
            .unwrap()
            .iter()
            .skip(1)
            .all(serde_json::Value::is_null)
    );
    let (id, edit) = review.prepare_edit().unwrap();
    assert_eq!(edit.texts.len(), 4);
    assert_eq!(edit.texts[0].as_deref(), Some("Edited Markdown λ"));
    assert!(edit.texts[1..].iter().all(Option::is_none));
    let mut acknowledged = baseline.clone();
    acknowledged.draft.title = edit.title.clone();
    if let NoteChange::Create { text, .. } = &mut acknowledged.draft.changes[0] {
        *text = edit.texts[0].clone().unwrap();
    }
    acknowledged.version += 1;
    acknowledged.updated_at_ms += 1;
    assert!(review.acknowledge_edit(id, acknowledged));
    assert!(review.can_leave());
    assert_eq!(
        review.record.draft.changes[1..],
        baseline.draft.changes[1..]
    );
}

#[test]
fn asset_candidate_before_and_parent_are_immutable_acknowledgement_bindings() {
    for member in 1..4 {
        let baseline = fixture();
        let mut review = ProposalReview::new(baseline.clone());
        review
            .edit_title("Owner title".into(), Instant::now())
            .unwrap();
        let (id, _) = review.prepare_edit().unwrap();
        let mut forged = baseline.clone();
        forged.version += 1;
        forged.updated_at_ms += 1;
        forged.draft.title = "Owner title".into();
        match &mut forged.draft.changes[member] {
            NoteChange::CreateAsset { bytes, .. } => bytes[0] ^= 1,
            NoteChange::ReplaceAsset { before_bytes, .. } => before_bytes.push(0),
            NoteChange::TrashAsset { parent, .. } => parent.inode += 1,
            _ => unreachable!(),
        }
        assert!(!review.acknowledge_edit(id, forged.clone()));
        assert!(!review.observe(forged));
        assert_eq!(review.record, baseline);
        assert!(review.dirty() && !review.can_leave());
    }
}

#[test]
fn scoped_asset_trash_restore_accepts_the_existing_exact_undo_capture() {
    use crate::approval::UndoCapture;
    use brn_workflow::proposal_apply::{UndoBinding, UndoOriginal, UndoPreview, UndoRequest};
    let record = fixture();
    let mut draft = record.draft.clone();
    let request = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: Uuid::new_v4(),
        trash_member: Some(3),
    };
    draft.id = request.operation_id;
    draft.group_id = None;
    draft.sources.clear();
    let original = record.draft.changes[3].before().unwrap().clone();
    draft.changes = vec![NoteChange::CreateAsset {
        path: "trashed.bin".into(),
        parent: record.draft.changes[3].parent().clone(),
        bytes: Vec::new(),
    }];
    let preview = UndoPreview {
        draft,
        binding: UndoBinding {
            operation_id: request.target_operation_id,
            trash_member: request.trash_member,
            originals: vec![Some(UndoOriginal {
                member_id: Uuid::new_v4(),
                fingerprint: original,
            })],
        },
    };
    let capture = UndoCapture::new(request.clone(), preview.clone()).unwrap();
    assert!(
        matches!(capture.command(), brn_workflow::app_worker::AppCommand::UndoProposal(actual) if actual == request)
    );
    let mut wrong = preview;
    wrong.binding.trash_member = Some(1);
    assert!(UndoCapture::new(request, wrong).is_none());
}
