use super::*;
use brn_workflow::{
    MAX_NOTE_BYTES,
    proposal_apply::{ApplyOutcome, GroupApprovalStop},
    proposals::{NoteChange, ProposalStamp},
};
use serde_json::json;

fn record(group: Option<Uuid>) -> ProposalRecord {
    let empty = [
        227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65,
        228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85,
    ];
    let parent = json!({"device":1,"inode":1});
    let before = |inode| json!({"device":1,"inode":inode,"len":0,"sha256":empty});
    serde_json::from_value(json!({
        "draft": {
            "id":Uuid::new_v4(), "group_id":group, "session_id":Uuid::new_v4(),
            "vault":{"id":Uuid::from_u128(7),"root":"/synthetic/approval-vault","identity":parent},
            "title":"\u{feff}Approve 日本語\r\nλ",
            "changes":[
                {"kind":"create","path":"new.md","parent":parent,"text":"\u{feff}Full 日本語\r\nλ"},
                {"kind":"replace","path":"existing.md","parent":parent,"before":before(2),"before_text":"","text":"\u{feff}Замена λ\r\n"},
                {"kind":"trash","path":"trash.md","parent":parent,"before":before(3),"before_text":""}
            ],
            "sources":[{"path":"source.md","fingerprint":before(99)}]
        },
        "version":2,"state":"draft","created_at_ms":1,"updated_at_ms":2,
        "comments":[{"id":Uuid::new_v4(),"text":"Exact temporary comment λ\r\n","target":{"kind":"proposal"}}]
    })).unwrap()
}

fn group(count: usize) -> ApprovalCapture {
    let id = Uuid::new_v4();
    ApprovalCapture::new((0..count).map(|_| record(Some(id))).collect(), Some(id)).unwrap()
}

fn receipt(capture: &ApprovalCapture, index: usize, outcome: ApplyOutcome) -> ApplyReceipt {
    let request = &capture.requests()[index];
    ApplyReceipt {
        operation_id: request.operation_id,
        proposal_id: request.expected.id,
        approved_version: request.expected.version,
        stamp: ProposalStamp {
            id: request.expected.id,
            version: request.expected.version + 2,
        },
        outcome,
    }
}

fn result(receipts: Vec<ApplyReceipt>, stop: Option<Uuid>) -> GroupApprovalResult {
    GroupApprovalResult {
        receipts,
        stopped: stop.map(|operation_id| GroupApprovalStop {
            operation_id,
            message: "Synthetic refusal".into(),
        }),
    }
}

#[test]
fn individual_capture_preserves_complete_review_and_reuses_exact_command_ids() {
    let original = record(Some(Uuid::new_v4()));
    let capture = ApprovalCapture::new(vec![original.clone()], None).unwrap();
    assert_eq!(capture.records(), std::slice::from_ref(&original));
    assert_eq!(capture.group_id(), None);
    assert_eq!(capture.requests().len(), 1);
    assert_eq!(capture.requests()[0].expected, original.stamp());
    assert!(!capture.requests()[0].operation_id.is_nil());
    assert_eq!(capture.clone(), capture);
    for _ in 0..2 {
        let AppCommand::ApproveProposal(request) = capture.command() else {
            panic!("individual");
        };
        assert_eq!(request, capture.requests()[0]);
    }
    let mut later = original.clone();
    later.version += 1;
    later.draft.title = "Later title".into();
    later.comments.clear();
    if let NoteChange::Create { text, .. } = &mut later.draft.changes[0] {
        *text = "Later text".into();
    }
    assert_eq!(capture.records(), &[original]);
    assert_ne!(capture.records()[0], later);
    assert!(!capture.accepts_group(&result(
        vec![receipt(&capture, 0, ApplyOutcome::Applied)],
        None
    )));
}

#[test]
fn captured_group_preserves_caller_order_and_never_includes_later_records() {
    let group_id = Uuid::new_v4();
    let records = vec![
        record(Some(group_id)),
        record(Some(group_id)),
        record(Some(group_id)),
    ];
    let capture = ApprovalCapture::new(records.clone(), Some(group_id)).unwrap();
    assert_eq!(capture.records(), records);
    assert_eq!(capture.group_id(), Some(group_id));
    let ids = capture
        .requests()
        .iter()
        .map(|request| request.operation_id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), records.len());
    assert!(!ids.contains(&Uuid::nil()));
    for (request, record) in capture.requests().iter().zip(&records) {
        assert_eq!(request.expected, record.stamp());
    }
    let later = record(Some(group_id));
    for _ in 0..2 {
        let AppCommand::ApproveProposalGroup(request) = capture.command() else {
            panic!("group");
        };
        request.validate().unwrap();
        assert_eq!(request.group_id, group_id);
        assert_eq!(request.approvals, capture.requests());
        assert!(
            !request
                .approvals
                .iter()
                .any(|request| request.expected.id == later.draft.id)
        );
    }
}

#[test]
fn receipt_matches_only_its_captured_operation_proposal_and_approved_stamp() {
    let capture = group(2);
    for outcome in [
        ApplyOutcome::Applied,
        ApplyOutcome::NotApplied,
        ApplyOutcome::Uncertain,
    ] {
        let exact = receipt(&capture, 0, outcome);
        assert!(capture.accepts_receipt(0, &exact));
        assert!(receipt_matches(&capture.requests()[0], &exact));
        assert!(!capture.accepts_receipt(1, &exact));
        assert!(!capture.accepts_receipt(2, &exact));
        let mut wrong = exact.clone();
        wrong.operation_id = Uuid::new_v4();
        assert!(!capture.accepts_receipt(0, &wrong));
        let mut wrong = exact.clone();
        wrong.proposal_id = Uuid::new_v4();
        assert!(!capture.accepts_receipt(0, &wrong));
        let mut wrong = exact.clone();
        wrong.approved_version += 1;
        assert!(!capture.accepts_receipt(0, &wrong));
        let mut wrong = exact.clone();
        wrong.stamp.id = Uuid::new_v4();
        assert!(!capture.accepts_receipt(0, &wrong));
        let mut wrong = exact.clone();
        wrong.stamp.version -= 1;
        assert!(!capture.accepts_receipt(0, &wrong));
        let mut settled = exact.clone();
        settled.stamp.version += 1;
        assert!(capture.accepts_receipt(0, &settled));
    }
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: ProposalStamp {
            id: Uuid::new_v4(),
            version: u64::MAX,
        },
    };
    let impossible = ApplyReceipt {
        operation_id: request.operation_id,
        proposal_id: request.expected.id,
        approved_version: u64::MAX,
        stamp: request.expected,
        outcome: ApplyOutcome::Applied,
    };
    assert!(!receipt_matches(&request, &impossible));
}

#[test]
fn capture_refuses_invalid_members_groups_duplicate_ids_and_mismatched_vaults() {
    let id = Uuid::new_v4();
    let original = record(Some(id));
    assert!(ApprovalCapture::new(vec![], None).is_none());
    assert!(ApprovalCapture::new(vec![], Some(id)).is_none());
    assert!(ApprovalCapture::new(vec![original.clone(), record(Some(id))], None).is_none());
    assert!(ApprovalCapture::new(vec![original.clone()], Some(Uuid::nil())).is_none());
    assert!(ApprovalCapture::new(vec![original.clone()], Some(Uuid::new_v4())).is_none());
    assert!(ApprovalCapture::new(vec![record(None)], Some(id)).is_none());
    assert!(ApprovalCapture::new(vec![original.clone(), original.clone()], Some(id)).is_none());
    for state in [
        ProposalState::Rejected,
        ProposalState::Applying,
        ProposalState::Uncertain,
        ProposalState::Applied,
    ] {
        let mut wrong = original.clone();
        wrong.state = state;
        assert!(ApprovalCapture::new(vec![wrong], None).is_none());
    }
    let mut wrong_vaults = vec![];
    let mut wrong = record(Some(id));
    wrong.draft.vault.as_mut().unwrap().id = Uuid::new_v4();
    wrong_vaults.push(wrong);
    let mut wrong = record(Some(id));
    wrong.draft.vault.as_mut().unwrap().root = "/synthetic/other-vault".into();
    wrong_vaults.push(wrong);
    let mut wrong = record(Some(id));
    wrong.draft.vault.as_mut().unwrap().identity.inode += 1;
    wrong_vaults.push(wrong);
    let mut wrong = record(Some(id));
    wrong.draft.vault.as_mut().unwrap().identity.device += 1;
    wrong_vaults.push(wrong);
    for wrong in wrong_vaults {
        assert!(ApprovalCapture::new(vec![original.clone(), wrong], Some(id)).is_none());
    }
    let maximum =
        ApprovalCapture::new((0..64).map(|_| record(Some(id))).collect(), Some(id)).unwrap();
    assert_eq!(maximum.records().len(), 64);
    assert_eq!(
        maximum
            .requests()
            .iter()
            .map(|request| request.operation_id)
            .collect::<HashSet<_>>()
            .len(),
        64
    );
    assert!(ApprovalCapture::new((0..65).map(|_| record(Some(id))).collect(), Some(id)).is_none());
}

#[test]
fn full_review_validation_refuses_corrupt_bindings_and_byte_or_comment_overflow() {
    let original = record(None);
    let mut invalid = vec![];
    let mut wrong = original.clone();
    wrong.draft.id = Uuid::nil();
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.draft.vault.as_mut().unwrap().id = Uuid::nil();
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.draft.vault.as_mut().unwrap().root = "relative/vault".into();
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.draft.session_id = Some(Uuid::nil());
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.version = 0;
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.version = u64::MAX - 2;
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.draft.title.clear();
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.draft.title = "λ".repeat(257);
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.draft.changes.clear();
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.comments[0].text = "x".repeat(16 * 1024 + 1);
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.comments[0].id = Uuid::nil();
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.comments.push(wrong.comments[0].clone());
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.comments = (0..65)
        .map(|_| {
            let mut comment = original.comments[0].clone();
            comment.id = Uuid::new_v4();
            comment
        })
        .collect();
    invalid.push(wrong);
    let mut wrong = original.clone();
    wrong.draft.sources[0].fingerprint.len = (MAX_NOTE_BYTES + 1) as u64;
    invalid.push(wrong);
    let mut wrong = original.clone();
    if let NoteChange::Create { text, .. } = &mut wrong.draft.changes[0] {
        *text = "x".repeat(MAX_NOTE_BYTES + 1);
    }
    invalid.push(wrong);
    let mut wrong = original.clone();
    if let NoteChange::Create { path, .. } = &mut wrong.draft.changes[0] {
        *path = ".brn/hidden.md".into();
    }
    invalid.push(wrong);
    let mut wrong = original.clone();
    if let NoteChange::Create { path, .. } = &mut wrong.draft.changes[0] {
        *path = "EXISTING.md".into();
    }
    invalid.push(wrong);
    let mut wrong = original.clone();
    if let NoteChange::Replace { before, .. } = &mut wrong.draft.changes[1] {
        before.len += 1;
    }
    invalid.push(wrong);
    let mut wrong = original.clone();
    if let NoteChange::Trash { before, .. } = &mut wrong.draft.changes[2] {
        before.sha256[0] ^= 1;
    }
    invalid.push(wrong);
    let mut wrong = original.clone();
    if let NoteChange::Replace { before, .. } = &mut wrong.draft.changes[1] {
        before.inode = 3;
    }
    invalid.push(wrong);
    for wrong in invalid {
        assert!(ApprovalCapture::new(vec![wrong], None).is_none());
    }
    let mut oversized = original.clone();
    oversized.draft.changes = (0..8)
        .map(|index| {
            let mut change = original.draft.changes[0].clone();
            if let NoteChange::Create { path, text, .. } = &mut change {
                *path = format!("full-{index}.md");
                *text = "x".repeat(MAX_NOTE_BYTES);
            }
            change
        })
        .collect();
    assert!(ApprovalCapture::new(vec![oversized], None).is_none());
    let mut maximum = original;
    maximum.draft.title = "λ".repeat(256);
    if let NoteChange::Create { text, .. } = &mut maximum.draft.changes[0] {
        *text = "λ".repeat(MAX_NOTE_BYTES / 2);
    }
    let capture = ApprovalCapture::new(vec![maximum.clone()], None).unwrap();
    assert_eq!(capture.records(), &[maximum]);
}

#[test]
fn complete_group_requires_every_ordered_receipt_applied_and_bound_to_capture() {
    let capture = group(3);
    let applied = (0..3)
        .map(|index| receipt(&capture, index, ApplyOutcome::Applied))
        .collect::<Vec<_>>();
    assert!(capture.accepts_group(&result(applied.clone(), None)));
    assert!(!capture.accepts_group(&result(vec![], None)));
    assert!(!capture.accepts_group(&result(applied[..2].to_vec(), None)));
    let mut reordered = applied.clone();
    reordered.swap(0, 1);
    assert!(!capture.accepts_group(&result(reordered, None)));
    let mut duplicated = applied.clone();
    duplicated[1] = applied[0].clone();
    assert!(!capture.accepts_group(&result(duplicated, None)));
    let mut unknown = applied.clone();
    unknown[1].operation_id = Uuid::new_v4();
    assert!(!capture.accepts_group(&result(unknown, None)));
    let mut late = applied.clone();
    late.push(receipt(&group(1), 0, ApplyOutcome::Applied));
    assert!(!capture.accepts_group(&result(late, None)));
    for outcome in [ApplyOutcome::NotApplied, ApplyOutcome::Uncertain] {
        let mut incomplete = applied.clone();
        incomplete[2].outcome = outcome;
        assert!(!capture.accepts_group(&result(incomplete, None)));
    }
}

#[test]
fn group_accepts_exact_prefix_refusal_and_errors_after_recorded_applied_receipts() {
    let capture = group(3);
    let ids = capture
        .requests()
        .iter()
        .map(|request| request.operation_id)
        .collect::<Vec<_>>();
    let first = receipt(&capture, 0, ApplyOutcome::Applied);
    let second = receipt(&capture, 1, ApplyOutcome::Applied);
    let third = receipt(&capture, 2, ApplyOutcome::Applied);
    // Failure before admission; after an Applied ledger receipt; or before the
    // next member is admitted. All are emitted by the shared group workflow.
    for scenario in [
        result(vec![], Some(ids[0])),
        result(vec![first.clone()], Some(ids[0])),
        result(vec![first.clone()], Some(ids[1])),
        result(vec![first.clone(), second.clone()], Some(ids[1])),
        result(vec![first.clone(), second.clone()], Some(ids[2])),
        result(vec![first.clone(), second.clone(), third], Some(ids[2])),
    ] {
        assert!(capture.accepts_group(&scenario), "{scenario:?}");
    }
    for outcome in [ApplyOutcome::NotApplied, ApplyOutcome::Uncertain] {
        assert!(capture.accepts_group(&result(vec![receipt(&capture, 0, outcome)], Some(ids[0]))));
        assert!(capture.accepts_group(&result(
            vec![first.clone(), receipt(&capture, 1, outcome)],
            Some(ids[1])
        )));
        assert!(capture.accepts_group(&result(
            vec![first.clone(), second.clone(), receipt(&capture, 2, outcome)],
            Some(ids[2])
        )));
    }
}

#[test]
fn group_rejects_skipped_members_unknown_stops_and_continuation_after_non_applied() {
    let capture = group(3);
    let ids = capture
        .requests()
        .iter()
        .map(|request| request.operation_id)
        .collect::<Vec<_>>();
    let first = receipt(&capture, 0, ApplyOutcome::Applied);
    let second = receipt(&capture, 1, ApplyOutcome::Applied);
    for scenario in [
        result(vec![], Some(ids[1])),
        result(vec![first.clone()], Some(ids[2])),
        result(vec![first.clone(), second.clone()], Some(ids[0])),
        result(vec![first.clone()], Some(Uuid::new_v4())),
        result(vec![], Some(Uuid::nil())),
    ] {
        assert!(!capture.accepts_group(&scenario), "{scenario:?}");
    }
    for outcome in [ApplyOutcome::NotApplied, ApplyOutcome::Uncertain] {
        assert!(!capture.accepts_group(&result(vec![receipt(&capture, 0, outcome)], Some(ids[1]))));
        assert!(!capture.accepts_group(&result(
            vec![first.clone(), receipt(&capture, 1, outcome)],
            Some(ids[2])
        )));
        assert!(!capture.accepts_group(&result(
            vec![receipt(&capture, 0, outcome), second.clone()],
            Some(ids[1])
        )));
        assert!(!capture.accepts_group(&result(
            vec![
                first.clone(),
                receipt(&capture, 1, outcome),
                receipt(&capture, 2, ApplyOutcome::Applied)
            ],
            Some(ids[2])
        )));
    }
    let single = group(1);
    assert!(single.accepts_group(&result(vec![], Some(single.requests()[0].operation_id))));
    assert!(!single.accepts_group(&result(vec![], None)));
}

#[test]
fn selected_group_preserves_exact_stamps_and_requires_selected_source_dependency() {
    let id = Uuid::new_v4();
    let source = record(Some(id));
    let mut dependent = crate::review::action_tests::fixture();
    dependent.draft.group_id = Some(id);
    dependent.draft.vault = source.draft.vault.clone();
    dependent.draft.changes.clear();
    dependent.draft.intake = Some(Box::new(brn_workflow::inbox_actions::InboxIntakeBinding {
        snapshot_id: Uuid::new_v4(),
        snapshot_sha256: [7; 32],
        source_proposal: source.stamp(),
        source_path: "source.md".into(),
        source_note_id: Uuid::new_v4(),
        source_text_sha256: [9; 32],
        assets: vec![],
        occurrences: vec![],
    }));
    let other = record(Some(id));
    let capture = ApprovalCapture::new(
        vec![source.clone(), dependent.clone(), other.clone()],
        Some(id),
    )
    .unwrap();
    assert!(
        capture
            .select(&[dependent.draft.id].into_iter().collect())
            .is_none()
    );
    let subset = capture
        .select(&[source.draft.id, dependent.draft.id].into_iter().collect())
        .unwrap();
    assert_eq!(subset.records(), &[source, dependent]);
    assert_eq!(subset.requests(), &capture.requests()[..2]);
    assert!(capture.select(&HashSet::new()).is_none());
    assert!(
        capture
            .select(&[Uuid::new_v4()].into_iter().collect())
            .is_none()
    );
}
