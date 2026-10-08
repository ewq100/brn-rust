use super::*;
use brn_store::work::inbox_actions::InboxSupersedesBinding;
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    inbox_actions::InboxKnowledgeBinding,
    proposals::{SourceVersion, validate_knowledge_predecessor_transition},
};

pub(crate) fn fixture() -> (ProposalRecord, ProposalRecord) {
    let mut before = super::tests::fixture();
    let analysis_id = Uuid::new_v4();
    let note_id = Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap();
    let source_id = Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap();
    let mut source = before.draft.sources[0].clone();
    source.fingerprint.len = 4;
    source.fingerprint.sha256 = [1; 32];
    let citation = brn_store::note_provenance::VaultCitation {
        note_id: source_id,
        sha256: [1; 32],
        start_byte: 0,
        end_byte: 4,
        quote: "body".into(),
    };
    let text = brn_store::note_provenance::write(&format!("---\nbrn_id: {note_id}\nbrn_kind: knowledge\nbrn_state: current\n---\nExact owner wording õ\r\n"), std::slice::from_ref(&citation)).unwrap();
    let parent = before.draft.changes[0].parent().clone();
    before.draft.group_id = Some(analysis_id);
    before.draft.sources = vec![source.clone()];
    before.draft.changes = vec![NoteChange::Create {
        path: "successor.md".into(),
        parent: parent.clone(),
        text,
    }];
    before.draft.inbox_knowledge = Some(Box::new(InboxKnowledgeBinding {
        analysis_id,
        note_id,
        source: Some(source),
        intake: None,
        intake_citations: vec![],
        supersedes: None,
        citations: vec![citation],
    }));
    before.comments[0].target =
        brn_workflow::proposals::CommentTarget::Text(brn_workflow::proposals::TextAnchor {
            change_index: 0,
            start: before.draft.changes[0]
                .text()
                .unwrap()
                .find("Exact owner")
                .unwrap(),
            end: before.draft.changes[0]
                .text()
                .unwrap()
                .find("Exact owner")
                .unwrap()
                + "Exact owner".len(),
            quote: "Exact owner".into(),
        });
    let mut after = before.clone();
    let predecessor_id = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
    let previous = "---\nbrn_id: 11111111-1111-4111-8111-111111111111\nbrn_kind: knowledge\nbrn_state: current\n---\nPrevious owner wording.\n";
    let mut proof = after.draft.sources[0].fingerprint.clone();
    proof.inode = 42;
    proof.len = previous.len() as u64;
    proof.sha256 = [
        242, 69, 168, 24, 243, 64, 70, 242, 95, 236, 234, 101, 17, 46, 78, 190, 53, 126, 1, 19,
        160, 35, 55, 236, 33, 66, 184, 155, 8, 188, 159, 87,
    ];
    let source = SourceVersion {
        path: "previous.md".into(),
        fingerprint: proof.clone(),
    };
    after.draft.inbox_knowledge.as_mut().unwrap().supersedes = Some(InboxSupersedesBinding {
        note_id: predecessor_id,
        source: source.clone(),
    });
    after.draft.sources.push(source);
    if let NoteChange::Create { text, .. } = &mut after.draft.changes[0] {
        text.push_str(&format!(
            "\n\nPrevious version: [History](brn://note/{predecessor_id})\n"
        ));
    }
    after.draft.changes.push(NoteChange::Replace {
        path: "previous.md".into(),
        parent,
        before: proof,
        before_text: previous.into(),
        text: brn_store::note_metadata::to_history(previous).unwrap(),
    });
    after.version += 1;
    after.updated_at_ms += 1;
    validate_knowledge_predecessor_transition(&before, &after).unwrap();
    (before, after)
}

#[test]
fn attachment_capture_blocks_typing_navigation_and_recovery_then_adopts_exact_pair() {
    let (before, after) = fixture();
    let mut review = ProposalReview::new(before.clone());
    let (id, request) = review.prepare_predecessor("previous.md".into()).unwrap();
    assert_eq!(request.expected, before.stamp());
    assert!(review.pending() && !review.can_leave() && !review.can_mutate());
    assert!(
        review
            .edit_text(0, "late typing".into(), Instant::now())
            .is_err()
    );
    assert!(
        review
            .edit_title("late title".into(), Instant::now())
            .is_err()
    );
    assert!(review.prepare_edit().is_none());
    assert!(!review.discard_local());
    assert!(review.acknowledge_predecessor(id, after.clone()));
    assert_eq!(review.record, after);
    assert_eq!(review.record.comments, before.comments);
    assert!(
        review
            .text(0)
            .unwrap()
            .starts_with(before.draft.changes[0].text().unwrap())
    );
    assert!(review.can_leave() && review.can_mutate());
    assert!(!review.predecessor_eligible());
    assert!(review.member_readonly(1));
    assert!(
        review
            .edit_text(1, "rewrite History".into(), Instant::now())
            .is_err()
    );
    let text = review
        .text(0)
        .unwrap()
        .replace("Exact owner wording", "Owner refined wording");
    review.edit_text(0, text, Instant::now()).unwrap();
    assert!(review.prepare_edit().is_some());
}

#[test]
fn dirty_review_and_wrong_acknowledgement_never_attach_or_replace_local_work() {
    let (before, after) = fixture();
    let mut review = ProposalReview::new(before.clone());
    review
        .edit_title("Unacknowledged owner title".into(), Instant::now())
        .unwrap();
    assert!(review.prepare_predecessor("previous.md".into()).is_none());
    assert!(review.discard_local());
    let (id, _) = review.prepare_predecessor("other.md".into()).unwrap();
    assert!(!review.acknowledge_predecessor(id, after.clone()));
    assert_eq!(review.record, before);
    assert!(review.error.is_some() && !review.pending());
    assert!(review.retry());
    let (current, _) = review.prepare_predecessor("previous.md".into()).unwrap();
    assert!(!review.acknowledge_predecessor(id, after.clone()));
    assert!(review.pending());
    let mut wrong = after;
    wrong.comments[0].text.push_str("forged");
    assert!(!review.acknowledge_predecessor(current, wrong));
    assert_eq!(review.record, before);
}

#[test]
fn observation_and_buffered_local_conflict_are_retained_across_attachment_acknowledgement() {
    let (before, after) = fixture();
    let mut review = ProposalReview::new(before.clone());
    let (id, _) = review.prepare_predecessor("previous.md".into()).unwrap();
    assert!(review.observe(after.clone()));
    assert_eq!(review.record, before);
    assert!(review.observed.is_some());
    assert!(review.acknowledge_predecessor(id, after.clone()));
    assert!(review.observed.is_none() && review.can_leave());

    let mut review = ProposalReview::new(before.clone());
    let (id, _) = review.prepare_predecessor("previous.md".into()).unwrap();
    // Simulate a buffered native change bypassing the ordinary typing fence.
    review.title = "Retain this exact local conflict õ".into();
    assert!(!review.acknowledge_predecessor(id, after.clone()));
    assert_eq!(review.title(), "Retain this exact local conflict õ");
    assert_eq!(review.record, before);
    assert_eq!(review.observed, Some(after));
    assert!(!review.can_leave());
    assert!(review.discard_local());
    assert!(review.history_member(1));
}

#[test]
fn newer_observation_cannot_be_erased_by_an_older_attachment_acknowledgement() {
    let (before, after) = fixture();
    let mut review = ProposalReview::new(before.clone());
    let (id, _) = review.prepare_predecessor("previous.md".into()).unwrap();
    let mut newer = before;
    newer.version = after.version + 1;
    newer.updated_at_ms = after.updated_at_ms + 1;
    newer.draft.title = "Newer observed owner title".into();
    assert!(review.observe(newer.clone()));
    assert!(review.acknowledge_predecessor(id, after.clone()));
    assert_eq!(review.record, after);
    assert_eq!(review.observed, Some(newer));
    assert!(!review.can_leave() && !review.can_mutate());
}

#[test]
fn application_state_correlates_dedicated_operation_and_rejects_stale_generation() {
    let (before, after) = fixture();
    let mut state = crate::ai::AiState::default();
    state.ready = true;
    state.vault_bound = true;
    state.review = Some(ProposalReview::new(before.clone()));
    let (id, command) = state
        .attach_knowledge_predecessor("previous.md".into())
        .unwrap();
    assert!(
        matches!(command, AppCommand::AttachInboxKnowledgePredecessor(request) if request.expected == before.stamp())
    );
    assert!(!state.review_editable() && !state.review_can_leave());
    assert!(state.open_review(Uuid::new_v4()).is_none());
    state.apply(Uuid::new_v4(), AppEvent::Proposal(after.clone()));
    assert_eq!(state.review.as_ref().unwrap().record, before);
    state.apply(id, AppEvent::Proposal(after.clone()));
    assert_eq!(state.review.as_ref().unwrap().record, after);

    state.review = Some(ProposalReview::new(before.clone()));
    let (id, _) = state
        .attach_knowledge_predecessor("previous.md".into())
        .unwrap();
    state.review_generation += 1;
    state.review = Some(ProposalReview::new(before.clone()));
    state.apply(id, AppEvent::Proposal(after));
    assert_eq!(state.review.as_ref().unwrap().record, before);
}
