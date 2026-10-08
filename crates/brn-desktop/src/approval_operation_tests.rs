use super::*;
use brn_workflow::{
    proposal_apply::{
        ApplyMemberPhase, RepairDirection, RepairPreview, RepairReceipt, UndoPreview, UndoRequest,
    },
    proposals::{NoteChange, ProposalStamp},
};
use serde_json::json;

const ORIGINAL: &str = "\u{feff}Original 日本語\r\nλ";
const CHANGED: &str = "\u{feff}Changed Русский\r\n";
const ORIGINAL_HASH: [u8; 32] = [
    129, 206, 162, 14, 71, 26, 124, 80, 217, 167, 188, 59, 241, 254, 47, 241, 83, 19, 125, 43, 171,
    181, 15, 113, 244, 190, 31, 173, 175, 195, 143, 159,
];
const CHANGED_HASH: [u8; 32] = [
    241, 5, 212, 24, 33, 241, 222, 118, 156, 146, 38, 241, 158, 1, 20, 104, 118, 242, 30, 37, 90,
    12, 3, 173, 167, 45, 71, 109, 124, 239, 126, 154,
];

fn undo_fixture(scope: Option<usize>) -> (UndoRequest, UndoPreview) {
    let request = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: Uuid::new_v4(),
        trash_member: scope,
    };
    let parent = json!({"device":1,"inode":1});
    let original =
        |inode| json!({"device":1,"inode":inode,"len":ORIGINAL.len(),"sha256":ORIGINAL_HASH});
    let changed =
        |inode| json!({"device":1,"inode":inode,"len":CHANGED.len(),"sha256":CHANGED_HASH});
    let mut preview: UndoPreview = serde_json::from_value(json!({
        "draft": {
            "id":request.operation_id,"group_id":null,"session_id":Uuid::new_v4(),
            "vault":{"id":Uuid::new_v4(),"root":"/synthetic/capture-vault","identity":parent},
            "title":"Undo: 日本語\r\nλ",
            "changes":[
                {"kind":"create","path":"restored-日本語.md","parent":parent,"text":ORIGINAL},
                {"kind":"replace","path":"replaced.md","parent":parent,"before":changed(2),"before_text":CHANGED,"text":ORIGINAL},
                {"kind":"trash","path":"created.md","parent":parent,"before":changed(3),"before_text":CHANGED}
            ],
            "sources":[]
        },
        "binding": {
            "operation_id":request.target_operation_id,"trash_member":scope,
            "originals":[
                {"member_id":Uuid::new_v4(),"fingerprint":original(4)},
                {"member_id":Uuid::new_v4(),"fingerprint":original(5)},
                null
            ]
        }
    }))
    .unwrap();
    if scope.is_some() {
        preview.draft.changes.truncate(1);
        preview.binding.originals.truncate(1);
    }
    (request, preview)
}

fn undo_receipt(capture: &UndoCapture, outcome: ApplyOutcome) -> ApplyReceipt {
    let id = capture.request().operation_id;
    ApplyReceipt {
        operation_id: id,
        proposal_id: id,
        approved_version: 1,
        stamp: ProposalStamp { id, version: 3 },
        outcome,
    }
}

fn action_undo_fixture() -> (UndoRequest, UndoPreview) {
    let (request, mut preview) = undo_fixture(None);
    preview.draft.changes.clear();
    preview.draft.vault = None;
    preview.binding.originals.clear();
    preview.draft.action_changes = crate::review::action_tests::fixture()
        .draft
        .action_changes
        .into_iter()
        .filter(|change| matches!(change, ActionChange::Replace { .. }))
        .collect();
    (request, preview)
}

#[test]
fn action_only_undo_capture_keeps_complete_replace_and_refuses_unsupported_or_oversized_work() {
    let (request, preview) = action_undo_fixture();
    let capture = UndoCapture::new(request.clone(), preview.clone()).unwrap();
    assert_eq!(capture.preview(), &preview);
    assert!(
        matches!(capture.command(), AppCommand::UndoProposal(submitted) if submitted == request)
    );
    assert!(capture.accepts_receipt(&undo_receipt(&capture, ApplyOutcome::Applied)));
    let mut wrongs = vec![];
    let mut wrong = preview.clone();
    wrong.draft.action_changes.clear();
    wrongs.push(wrong);
    let mut wrong = preview.clone();
    let change = &wrong.draft.action_changes[0];
    wrong.draft.action_changes[0] = ActionChange::Create {
        id: change.id(),
        data: change.data().clone(),
    };
    wrongs.push(wrong);
    let mut wrong = preview.clone();
    let (_, files) = undo_fixture(None);
    wrong.draft.changes = files.draft.changes;
    wrong.binding.originals = files.binding.originals;
    wrongs.push(wrong);
    let mut wrong = preview.clone();
    wrong.binding.originals.push(None);
    wrongs.push(wrong);
    let mut wrong = preview.clone();
    wrong.binding.operation_id = Uuid::new_v4();
    wrongs.push(wrong);
    let mut wrong = preview.clone();
    wrong.draft.id = Uuid::new_v4();
    wrongs.push(wrong);
    let mut wrong = preview.clone();
    if let ActionChange::Replace { before, .. } = &mut wrong.draft.action_changes[0] {
        before.data.state = brn_workflow::actions::ActionState::Completed;
        before.completed_at_ms = Some(before.updated_at_ms);
    }
    wrongs.push(wrong);
    let mut bounded = preview.clone();
    bounded.draft.action_changes.resize(
        MAX_PROPOSAL_CHANGES,
        bounded.draft.action_changes[0].clone(),
    );
    assert!(UndoCapture::new(request.clone(), bounded.clone()).is_some());
    bounded
        .draft
        .action_changes
        .push(bounded.draft.action_changes[0].clone());
    wrongs.push(bounded);
    for wrong in wrongs {
        assert!(UndoCapture::new(request.clone(), wrong).is_none());
    }
    let scoped = UndoRequest {
        trash_member: Some(0),
        ..request
    };
    let mut scoped_preview = preview;
    scoped_preview.binding.trash_member = scoped.trash_member;
    assert!(UndoCapture::new(scoped, scoped_preview).is_none());
}

fn repair_fixture() -> RepairPreview {
    let (request, preview) = undo_fixture(None);
    RepairPreview {
        operation_id: request.target_operation_id,
        expected: std::array::from_fn(|index| index as u8),
        approved: preview.draft,
        phases: vec![
            ApplyMemberPhase::Before,
            ApplyMemberPhase::Applied,
            ApplyMemberPhase::Before,
        ],
    }
}

#[test]
fn undo_capture_freezes_full_inverse_and_uses_one_identified_request() {
    let (request, preview) = undo_fixture(None);
    let capture = UndoCapture::new(request.clone(), preview.clone()).unwrap();
    assert_eq!(capture.request(), &request);
    assert_eq!(capture.preview(), &preview);
    assert_eq!(capture.clone(), capture);
    assert_eq!(capture.preview().draft.changes[0].text(), Some(ORIGINAL));
    assert_eq!(capture.preview().draft.changes[1].text(), Some(ORIGINAL));
    assert_eq!(capture.preview().draft.changes[2].text(), None);
    for _ in 0..2 {
        let AppCommand::UndoProposal(submitted) = capture.command() else {
            panic!("Undo command");
        };
        assert_eq!(submitted, request);
    }
    let mut later = preview.clone();
    later.draft.title = "Later title".into();
    if let NoteChange::Create { text, .. } = &mut later.draft.changes[0] {
        *text = "Later typing".into();
    }
    later.binding.originals[0]
        .as_mut()
        .unwrap()
        .fingerprint
        .inode += 1;
    assert_eq!(capture.preview(), &preview);
    assert_ne!(capture.preview(), &later);
}

#[test]
fn scoped_trash_capture_keeps_original_member_index_and_rejects_noncreate_inverse() {
    // 53 names the original source member, not index zero in a filtered Trash list.
    let (request, preview) = undo_fixture(Some(53));
    let capture = UndoCapture::new(request.clone(), preview.clone()).unwrap();
    assert_eq!(capture.request().trash_member, Some(53));
    assert_eq!(capture.preview().binding.trash_member, Some(53));
    assert_eq!(capture.preview().draft.changes.len(), 1);
    assert!(capture.preview().binding.originals[0].is_some());
    let (_, whole) = undo_fixture(None);
    for change in whole.draft.changes.into_iter().skip(1) {
        let mut wrong = preview.clone();
        wrong.draft.changes[0] = change;
        assert!(UndoCapture::new(request.clone(), wrong).is_none());
    }
    let mut wrong = preview.clone();
    wrong.binding.originals[0] = None;
    assert!(UndoCapture::new(request.clone(), wrong).is_none());
    let mut wrong = preview;
    wrong.draft.changes.push(wrong.draft.changes[0].clone());
    wrong
        .binding
        .originals
        .push(wrong.binding.originals[0].clone());
    assert!(UndoCapture::new(request, wrong).is_none());
}

#[test]
fn undo_capture_refuses_misbound_ids_scope_group_and_member_counts() {
    let (request, preview) = undo_fixture(None);
    let mut wrong_requests = vec![];
    let mut wrong = request.clone();
    wrong.operation_id = Uuid::nil();
    wrong_requests.push(wrong);
    let mut wrong = request.clone();
    wrong.target_operation_id = Uuid::nil();
    wrong_requests.push(wrong);
    let mut wrong = request.clone();
    wrong.target_operation_id = wrong.operation_id;
    wrong_requests.push(wrong);
    let mut wrong = request.clone();
    wrong.trash_member = Some(MAX_PROPOSAL_CHANGES);
    wrong_requests.push(wrong);
    for wrong in wrong_requests {
        assert!(UndoCapture::new(wrong, preview.clone()).is_none());
    }
    let mut wrong_previews = vec![];
    let mut wrong = preview.clone();
    wrong.draft.id = Uuid::new_v4();
    wrong_previews.push(wrong);
    let mut wrong = preview.clone();
    wrong.binding.operation_id = Uuid::new_v4();
    wrong_previews.push(wrong);
    let mut wrong = preview.clone();
    wrong.binding.trash_member = Some(0);
    wrong_previews.push(wrong);
    let mut wrong = preview.clone();
    wrong.draft.group_id = Some(Uuid::new_v4());
    wrong_previews.push(wrong);
    let mut wrong = preview.clone();
    wrong.binding.originals.pop();
    wrong_previews.push(wrong);
    let mut wrong = preview.clone();
    wrong.draft.changes.clear();
    wrong.binding.originals.clear();
    wrong_previews.push(wrong);
    let mut wrong = preview;
    wrong
        .draft
        .changes
        .resize(65, wrong.draft.changes[0].clone());
    wrong.binding.originals.resize(65, None);
    wrong_previews.push(wrong);
    for wrong in wrong_previews {
        assert!(UndoCapture::new(request.clone(), wrong).is_none());
    }
}

#[test]
fn undo_receipts_bind_inverse_admission_and_keep_actual_outcome() {
    let (request, preview) = undo_fixture(None);
    let capture = UndoCapture::new(request, preview).unwrap();
    for outcome in [
        ApplyOutcome::Applied,
        ApplyOutcome::NotApplied,
        ApplyOutcome::Uncertain,
    ] {
        let exact = undo_receipt(&capture, outcome);
        assert!(capture.accepts_receipt(&exact));
        let mut wrong_receipts = vec![];
        let mut wrong = exact.clone();
        wrong.operation_id = capture.request().target_operation_id;
        wrong_receipts.push(wrong);
        let mut wrong = exact.clone();
        wrong.proposal_id = capture.request().target_operation_id;
        wrong_receipts.push(wrong);
        let mut wrong = exact.clone();
        wrong.approved_version = 2;
        wrong_receipts.push(wrong);
        let mut wrong = exact.clone();
        wrong.stamp.id = Uuid::new_v4();
        wrong_receipts.push(wrong);
        let mut wrong = exact.clone();
        wrong.stamp.version = 2;
        wrong_receipts.push(wrong);
        for wrong in wrong_receipts {
            assert!(!capture.accepts_receipt(&wrong));
        }
        let mut reconciled = exact;
        reconciled.stamp.version = 4;
        assert!(capture.accepts_receipt(&reconciled));
    }
}

#[test]
fn repair_captures_freeze_full_preview_hash_direction_and_attempt_identity() {
    let preview = repair_fixture();
    for direction in [RepairDirection::Finish, RepairDirection::Restore] {
        let capture = RepairCapture::new(preview.clone(), direction).unwrap();
        assert_eq!(capture.preview(), &preview);
        assert_eq!(capture.request().operation_id, preview.operation_id);
        assert_eq!(capture.request().expected, preview.expected);
        assert_eq!(capture.request().direction, direction);
        assert!(!capture.request().id.is_nil());
        assert_ne!(capture.request().id, preview.operation_id);
        assert_eq!(capture.clone(), capture);
        for _ in 0..2 {
            let AppCommand::RepairProposal(request) = capture.command() else {
                panic!("Repair command");
            };
            assert_eq!(&request, capture.request());
        }
        let mut later = preview.clone();
        later.expected[0] ^= 255;
        later.phases.rotate_left(1);
        later.approved.title = "Later title".into();
        assert_eq!(capture.preview(), &preview);
        assert_ne!(capture.preview(), &later);
        for outcome in [
            None,
            Some(ApplyOutcome::Applied),
            Some(ApplyOutcome::NotApplied),
            Some(ApplyOutcome::Uncertain),
        ] {
            let receipt = RepairReceipt {
                id: capture.request().id,
                operation_id: preview.operation_id,
                direction,
                outcome,
            };
            assert!(capture.accepts_receipt(&receipt));
            let mut wrong = receipt.clone();
            wrong.id = Uuid::new_v4();
            assert!(!capture.accepts_receipt(&wrong));
            let mut wrong = receipt.clone();
            wrong.operation_id = Uuid::new_v4();
            assert!(!capture.accepts_receipt(&wrong));
            let mut wrong = receipt;
            wrong.direction = match direction {
                RepairDirection::Finish => RepairDirection::Restore,
                RepairDirection::Restore => RepairDirection::Finish,
            };
            assert!(!capture.accepts_receipt(&wrong));
        }
    }
}

#[test]
fn repair_capture_refuses_bad_operation_and_incomplete_or_unbounded_phases() {
    let preview = repair_fixture();
    let mut invalid = vec![];
    let mut wrong = preview.clone();
    wrong.operation_id = Uuid::nil();
    invalid.push(wrong);
    let mut wrong = preview.clone();
    wrong.phases.pop();
    invalid.push(wrong);
    let mut wrong = preview.clone();
    wrong.phases.push(ApplyMemberPhase::Before);
    invalid.push(wrong);
    let mut wrong = preview.clone();
    wrong.approved.changes.clear();
    wrong.phases.clear();
    invalid.push(wrong);
    let mut maximum = preview;
    maximum
        .approved
        .changes
        .resize(MAX_PROPOSAL_CHANGES, maximum.approved.changes[0].clone());
    maximum
        .phases
        .resize(MAX_PROPOSAL_CHANGES, ApplyMemberPhase::Before);
    assert!(RepairCapture::new(maximum.clone(), RepairDirection::Finish).is_some());
    maximum
        .approved
        .changes
        .push(maximum.approved.changes[0].clone());
    maximum.phases.push(ApplyMemberPhase::Before);
    invalid.push(maximum);
    for wrong in invalid {
        assert!(RepairCapture::new(wrong, RepairDirection::Restore).is_none());
    }
}
