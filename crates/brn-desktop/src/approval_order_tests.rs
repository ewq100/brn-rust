use super::*;
use brn_workflow::proposal_apply::GroupApprovalStop;
use serde_json::json;

pub(crate) fn member(group: Uuid, title: &str) -> ProposalRecord {
    serde_json::from_value(json!({
        "draft": {"id":Uuid::new_v4(), "group_id":group, "session_id":null,
            "vault":{"id":Uuid::from_u128(7),"root":"/synthetic/approval-vault","identity":{"device":1,"inode":1}},
            "title":title,
            "changes":[{"kind":"create","path":format!("{title}.md"),"parent":{"device":1,"inode":1},"text":"\u{feff}Exact owner wording 日本語\r\nλ"}],
            "sources":[]},
        "version":2,"state":"draft","created_at_ms":1,"updated_at_ms":2,
        "comments":[{"id":Uuid::new_v4(),"text":"Exact temporary comment λ\r\n","target":{"kind":"proposal"}}]
    })).unwrap()
}

pub(crate) fn source(group: Uuid, title: &str) -> ProposalRecord {
    use brn_workflow::inbox_processing::InboxSourceBinding;
    let original = "\u{feff}Exact Source 日本語\r\nλ";
    // SHA-256 of the complete synthetic original, including BOM and CRLF.
    let hash: [u8; 32] = [
        229, 184, 161, 241, 218, 154, 23, 136, 88, 163, 160, 189, 86, 245, 184, 215, 42, 110, 142,
        90, 61, 23, 86, 192, 201, 57, 90, 42, 234, 199, 15, 55,
    ];
    let binding: InboxSourceBinding = serde_json::from_value(json!({
        "batch_id":Uuid::new_v4(), "index":0,
        "original":{"capture":{"id":Uuid::new_v4(), "kind":"markdown", "title":title,
            "original_name":"synthetic.md", "copy":{"directory":"/synthetic/intake",
                "directory_device":1,"directory_inode":2,"file_device":1,"file_inode":3,
                "byte_len":original.len(),"sha256":hash}},"received_at_ms":1},
        "format":"verbatim_markdown_v1","byte_len":original.len(),"sha256":hash,"note_id":Uuid::new_v4()
    })).unwrap();
    let mut record = member(group, title);
    if let NoteChange::Create { text, .. } = &mut record.draft.changes[0] {
        *text = binding.markdown(original).unwrap();
    }
    record.draft.inbox_source = Some(Box::new(binding));
    record
}

#[test]
fn captured_order_moves_exact_pairs_and_selection_command_and_receipts_follow_that_order() {
    let group = Uuid::new_v4();
    let records = [member(group, "A"), member(group, "B"), member(group, "C")];
    let capture = ApprovalCapture::new(records.to_vec(), Some(group)).unwrap();
    let ordered = capture
        .move_earlier(records[2].draft.id)
        .unwrap()
        .move_earlier(records[2].draft.id)
        .unwrap();
    assert_eq!(
        ordered.records(),
        &[records[2].clone(), records[0].clone(), records[1].clone()]
    );
    assert_eq!(
        ordered.requests(),
        &[
            capture.requests()[2].clone(),
            capture.requests()[0].clone(),
            capture.requests()[1].clone()
        ]
    );
    assert_eq!(
        ordered
            .move_later(records[2].draft.id)
            .unwrap()
            .move_later(records[2].draft.id)
            .unwrap(),
        capture
    );
    let selected = ordered
        .select(
            &[records[1].draft.id, records[2].draft.id]
                .into_iter()
                .collect(),
        )
        .unwrap();
    assert_eq!(
        selected.records(),
        &[records[2].clone(), records[1].clone()]
    );
    assert_eq!(
        selected.requests(),
        &[capture.requests()[2].clone(), capture.requests()[1].clone()]
    );
    let AppCommand::ApproveProposalGroup(command) = selected.command() else {
        panic!("group command")
    };
    assert_eq!(command.group_id, group);
    assert_eq!(command.approvals, selected.requests());
    let receipts: Vec<_> = selected
        .requests()
        .iter()
        .map(|request| ApplyReceipt {
            operation_id: request.operation_id,
            proposal_id: request.expected.id,
            approved_version: request.expected.version,
            stamp: ProposalStamp {
                id: request.expected.id,
                version: request.expected.version + 2,
            },
            outcome: ApplyOutcome::Applied,
        })
        .collect();
    assert!(selected.accepts_group(&GroupApprovalResult {
        receipts: receipts.clone(),
        stopped: None
    }));
    assert!(selected.accepts_group(&GroupApprovalResult {
        receipts: receipts[..1].to_vec(),
        stopped: Some(GroupApprovalStop {
            operation_id: selected.requests()[1].operation_id,
            message: "Synthetic stop".into()
        })
    }));
    assert!(!selected.accepts_group(&GroupApprovalResult {
        receipts: vec![receipts[1].clone(), receipts[0].clone()],
        stopped: None
    }));
    assert!(!selected.accepts_group(&GroupApprovalResult {
        receipts: receipts[..1].to_vec(),
        stopped: Some(GroupApprovalStop {
            operation_id: Uuid::new_v4(),
            message: "Mismatched stop".into()
        })
    }));
    assert_eq!(capture.records(), records);
}

#[test]
fn sources_canonicalize_stably_and_neither_move_nor_allow_crossing() {
    let group = Uuid::new_v4();
    let a = member(group, "A");
    let b = member(group, "B");
    let s1 = source(group, "Source1");
    let s2 = source(group, "Source2");
    let capture = ApprovalCapture::new(
        vec![a.clone(), s1.clone(), b.clone(), s2.clone()],
        Some(group),
    )
    .unwrap();
    assert_eq!(
        capture.records(),
        &[s1.clone(), s2.clone(), a.clone(), b.clone()]
    );
    for (record, request) in capture.records().iter().zip(capture.requests()) {
        assert_eq!(request.expected, record.stamp());
    }
    for source in [&s1, &s2] {
        assert!(capture.move_earlier(source.draft.id).is_none());
        assert!(capture.move_later(source.draft.id).is_none());
    }
    assert!(capture.move_earlier(a.draft.id).is_none());
    assert!(capture.move_later(b.draft.id).is_none());
    assert!(capture.move_earlier(Uuid::new_v4()).is_none());
    assert!(capture.move_later(Uuid::new_v4()).is_none());
    assert_eq!(
        capture.move_later(a.draft.id).unwrap().records(),
        &[s1, s2, b, a.clone()]
    );
    for group in [None, Some(group)] {
        let single = ApprovalCapture::new(vec![a.clone()], group).unwrap();
        assert!(single.move_earlier(a.draft.id).is_none());
        assert!(single.move_later(a.draft.id).is_none());
    }
    assert!(ApprovalCapture::new(vec![], Some(group)).is_none());
}

#[test]
fn moved_selection_still_requires_each_captured_pending_source_dependency() {
    let group = Uuid::new_v4();
    let source1 = source(group, "Source1");
    let source2 = source(group, "Source2");
    let mut dependent = crate::review::action_tests::fixture();
    dependent.draft.group_id = Some(group);
    dependent.draft.vault = source1.draft.vault.clone();
    dependent.draft.changes.clear();
    dependent.draft.intake = Some(Box::new(brn_workflow::inbox_actions::InboxIntakeBinding {
        snapshot_id: Uuid::new_v4(),
        snapshot_sha256: [7; 32],
        source_proposal: source1.stamp(),
        source_path: source1.draft.changes[0].path().into(),
        source_note_id: source1.draft.inbox_source.as_ref().unwrap().note_id,
        source_text_sha256: [9; 32],
        assets: vec![],
        occurrences: vec![],
    }));
    let mut dependent2 = dependent.clone();
    dependent2.draft.id = Uuid::new_v4();
    dependent2.draft.intake.as_mut().unwrap().source_proposal = source2.stamp();
    let capture = ApprovalCapture::new(
        vec![
            dependent.clone(),
            source1.clone(),
            dependent2.clone(),
            source2.clone(),
        ],
        Some(group),
    )
    .unwrap();
    let ordered = capture.move_earlier(dependent2.draft.id).unwrap();
    assert!(
        ordered
            .select(
                &[dependent.draft.id, dependent2.draft.id, source1.draft.id]
                    .into_iter()
                    .collect()
            )
            .is_none()
    );
    let selected = ordered
        .select(
            &[
                dependent.draft.id,
                dependent2.draft.id,
                source1.draft.id,
                source2.draft.id,
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();
    assert_eq!(
        selected.records(),
        &[source1, source2, dependent2, dependent]
    );
    assert_eq!(selected.requests(), ordered.requests());
}
