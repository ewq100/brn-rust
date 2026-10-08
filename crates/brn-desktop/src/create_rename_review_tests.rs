//! Complete-record destination capture and application correlation, without I/O.
use super::*;
use brn_workflow::app_worker::{AppCommand, AppEvent};

pub(crate) fn renamed(before: &ProposalRecord, index: usize, path: &str) -> ProposalRecord {
    let mut after = before.clone();
    let NoteChange::Create { path: target, .. } = &mut after.draft.changes[index] else {
        panic!("Create fixture required")
    };
    if target != path {
        *target = path.into();
        after.version += 1;
        after.updated_at_ms += 1;
    }
    brn_workflow::proposals::validate_create_rename_transition(before, &after, index, path)
        .unwrap();
    after
}

fn state(record: ProposalRecord) -> crate::ai::AiState {
    let mut state = crate::ai::AiState::default();
    state.ready = true;
    state.vault_bound = true;
    state.review = Some(ProposalReview::new(record));
    state
}

#[test]
fn mixed_review_capture_keeps_every_other_member_and_fences_typing_until_exact_ack() {
    let before = super::action_tests::fixture();
    let after = renamed(&before, 0, "owner õ.md");
    let mut review = ProposalReview::new(before.clone());
    assert!(review.create_rename_eligible(0));
    assert!(!review.create_rename_eligible(1));
    let (id, request) = review
        .prepare_create_rename(0, "owner õ.md".into())
        .unwrap();
    assert_eq!(request.expected, before.stamp());
    assert_eq!(request.change_index, 0);
    assert_eq!(request.path, "owner õ.md");
    assert!(review.pending() && !review.can_mutate() && !review.can_leave());
    assert!(review.edit_title("later".into(), Instant::now()).is_err());
    assert!(review.edit_text(0, "later".into(), Instant::now()).is_err());
    let data = review.action_data()[0].clone();
    assert!(review.edit_action(0, data, Instant::now()).is_err());
    assert!(review.prepare_edit().is_none() && !review.discard_local());
    assert!(review.acknowledge_create_rename(id, after.clone()));
    assert_eq!(review.record, after);
    assert_eq!(review.texts(), record_texts(&before));
    assert_eq!(
        review.record.draft.action_changes,
        before.draft.action_changes
    );
    assert_eq!(review.record.draft.sources, before.draft.sources);
    assert_eq!(review.record.comments, before.comments);
    assert!(review.can_mutate() && review.can_leave());
}

#[test]
fn knowledge_history_pair_renames_only_successor_and_keeps_history_readonly() {
    let (_, before) = super::predecessor_tests::fixture();
    let after = renamed(&before, 0, "new knowledge.md");
    let mut review = ProposalReview::new(before.clone());
    assert!(review.create_rename_eligible(0));
    assert!(!review.create_rename_eligible(1));
    let (id, _) = review
        .prepare_create_rename(0, "new knowledge.md".into())
        .unwrap();
    assert!(review.acknowledge_create_rename(id, after));
    assert_eq!(review.record.draft.changes[1], before.draft.changes[1]);
    assert_eq!(
        review.record.draft.inbox_knowledge,
        before.draft.inbox_knowledge
    );
    assert!(review.member_readonly(1));
    assert!(
        review
            .edit_text(1, "History edit".into(), Instant::now())
            .is_err()
    );
}

#[test]
fn same_destination_is_byte_exact_and_rejects_a_version_or_timestamp_change() {
    let before = super::tests::fixture();
    let mut review = ProposalReview::new(before.clone());
    let (id, _) = review.prepare_create_rename(0, "new.md".into()).unwrap();
    assert!(review.acknowledge_create_rename(id, before.clone()));
    assert_eq!(review.record, before);
    for changed_version in [false, true] {
        let mut review = ProposalReview::new(before.clone());
        let (id, _) = review.prepare_create_rename(0, "new.md".into()).unwrap();
        let mut forged = before.clone();
        if changed_version {
            forged.version += 1;
        } else {
            forged.updated_at_ms += 1;
        }
        assert!(!review.acknowledge_create_rename(id, forged));
        assert_eq!(review.record, before);
        assert!(!review.pending() && review.error.is_some());
    }
}

#[test]
fn dirty_invalid_type_index_and_cross_folder_requests_do_not_capture() {
    let before = super::tests::fixture();
    for (index, path) in [
        (1, "replace.md"),
        (99, "absent.md"),
        (0, "folder/new.md"),
        (0, ".hidden.md"),
        (0, "new.txt"),
        (0, "../new.md"),
    ] {
        let mut review = ProposalReview::new(before.clone());
        assert!(review.prepare_create_rename(index, path.into()).is_none());
        assert_eq!(review.record, before);
        assert!(!review.pending());
    }
    let mut review = ProposalReview::new(before.clone());
    assert!(review.prepare_create_rename(0, "x".repeat(4097)).is_none());
    assert!(review.retry());
    review
        .edit_title(
            "Owner title awaiting acknowledgement".into(),
            Instant::now(),
        )
        .unwrap();
    assert!(
        review
            .prepare_create_rename(0, "renamed.md".into())
            .is_none()
    );
    let (id, edit) = review.prepare_edit().unwrap();
    let mut edited = before;
    edited.draft.title = edit.title;
    edited.version += 1;
    edited.updated_at_ms += 1;
    assert!(review.acknowledge_edit(id, edited));
    assert!(
        review
            .prepare_create_rename(0, "renamed.md".into())
            .is_some()
    );
}

#[test]
fn wrong_id_path_member_and_preserved_bytes_refuse_without_replacing_owner_review() {
    let before = super::tests::fixture();
    let after = renamed(&before, 0, "correct.md");
    let mut review = ProposalReview::new(before.clone());
    let (id, _) = review
        .prepare_create_rename(0, "correct.md".into())
        .unwrap();
    assert!(!review.acknowledge_create_rename(Uuid::new_v4(), after.clone()));
    assert!(review.pending());
    for mutation in 0..6 {
        let mut review = ProposalReview::new(before.clone());
        let (id, _) = review
            .prepare_create_rename(0, "correct.md".into())
            .unwrap();
        let mut bad = after.clone();
        match mutation {
            0 => {
                if let NoteChange::Create { path, .. } = &mut bad.draft.changes[0] {
                    *path = "other.md".into();
                }
            }
            1 => bad.comments[0].text.push_str("changed"),
            2 => bad.draft.changes.swap(0, 1),
            3 => bad.draft.id = Uuid::new_v4(),
            4 => bad.version += 1,
            _ => bad.draft.title.push_str("changed"),
        }
        assert!(!review.acknowledge_create_rename(id, bad));
        assert_eq!(review.record, before);
        assert_eq!(review.title(), before.draft.title);
        assert_eq!(review.texts(), record_texts(&before));
        assert!(!review.pending());
        assert!(!review.can_leave());
    }
    assert!(review.acknowledge_create_rename(id, after));
}

#[test]
fn buffered_owner_work_and_newer_observation_survive_destination_acknowledgement() {
    let before = super::tests::fixture();
    let after = renamed(&before, 0, "renamed.md");
    let mut review = ProposalReview::new(before.clone());
    let (id, _) = review
        .prepare_create_rename(0, "renamed.md".into())
        .unwrap();
    assert!(review.observe(after.clone()));
    assert_eq!(review.record, before);
    assert!(review.acknowledge_create_rename(id, after.clone()));
    assert!(review.observed.is_none() && review.can_leave());

    for generation_only in [false, true] {
        let mut review = ProposalReview::new(before.clone());
        let (id, _) = review
            .prepare_create_rename(0, "renamed.md".into())
            .unwrap();
        if generation_only {
            review.generation += 1;
        } else {
            review.title = "Exact retained owner õ\r\n".into();
        }
        let title = review.title().to_owned();
        assert!(!review.acknowledge_create_rename(id, after.clone()));
        assert_eq!(review.title(), title);
        assert_eq!(review.record, before);
        assert_eq!(review.observed, Some(after.clone()));
        assert!(!review.can_leave() && !review.pending());
    }
    let mut review = ProposalReview::new(before.clone());
    let (id, _) = review
        .prepare_create_rename(0, "renamed.md".into())
        .unwrap();
    let mut newer = before;
    newer.version = after.version + 1;
    newer.updated_at_ms = after.updated_at_ms + 1;
    newer.draft.title = "Newer observed owner title".into();
    assert!(review.observe(newer.clone()));
    assert!(review.acknowledge_create_rename(id, after));
    assert_eq!(review.observed, Some(newer.clone()));
    assert!(!review.can_leave());

    let before = super::tests::fixture();
    let after = renamed(&before, 0, "renamed.md");
    let mut review = ProposalReview::new(before.clone());
    let (id, _) = review
        .prepare_create_rename(0, "renamed.md".into())
        .unwrap();
    // Both a later observation and buffered local owner text must survive.
    let mut newer = before;
    newer.version = after.version + 1;
    newer.updated_at_ms = after.updated_at_ms + 1;
    newer.draft.title = "Later observed title".into();
    assert!(review.observe(newer.clone()));
    review.title = "Buffered owner title".into();
    assert!(!review.acknowledge_create_rename(id, after));
    assert_eq!(review.observed, Some(newer));
    assert_eq!(review.title(), "Buffered owner title");
}

#[test]
fn state_correlates_rename_and_keeps_failed_or_stale_acknowledgements_local() {
    let before = super::tests::fixture();
    let after = renamed(&before, 0, "renamed.md");
    let mut state = state(before.clone());
    let (id, command) = state
        .rename_proposal_create(0, "renamed.md".into())
        .unwrap();
    assert!(
        matches!(command, AppCommand::RenameProposalCreate(request) if request.expected == before.stamp() && request.change_index == 0 && request.path == "renamed.md")
    );
    assert!(!state.review_editable() && !state.review_can_leave());
    assert!(state.open_review(Uuid::new_v4()).is_none());
    state.apply(Uuid::new_v4(), AppEvent::Proposal(after.clone()));
    assert_eq!(state.review.as_ref().unwrap().record, before);
    state.apply(
        id,
        AppEvent::Failed(brn_workflow::WorkflowError {
            kind: brn_workflow::ErrorKind::ContextStale,
            message: "occupied destination".into(),
        }),
    );
    assert_eq!(state.review.as_ref().unwrap().record, before);
    assert!(!state.review.as_ref().unwrap().pending());
    assert!(!state.review_can_leave());
    assert!(state.review.as_mut().unwrap().retry());
    let (current, _) = state
        .rename_proposal_create(0, "renamed.md".into())
        .unwrap();
    state.apply(id, AppEvent::Proposal(after.clone()));
    assert!(state.review.as_ref().unwrap().pending());
    state.apply(current, AppEvent::Proposal(after.clone()));
    assert_eq!(state.review.as_ref().unwrap().record, after);

    state.review = Some(ProposalReview::new(before.clone()));
    let (id, _) = state
        .rename_proposal_create(0, "renamed.md".into())
        .unwrap();
    state.review_generation += 1;
    state.review = Some(ProposalReview::new(before.clone()));
    state.apply(id, AppEvent::Proposal(after));
    assert_eq!(state.review.as_ref().unwrap().record, before);
}

#[test]
fn state_preserves_readiness_vault_active_rewrite_and_other_review_guards() {
    let before = super::tests::fixture();
    for guard in 0..7 {
        let mut state = state(before.clone());
        match guard {
            0 => state.ready = false,
            1 => state.vault_bound = false,
            2 => {
                state
                    .review
                    .as_mut()
                    .unwrap()
                    .edit_title("Owner typing".into(), Instant::now())
                    .unwrap();
            }
            3 => {
                state.pending.insert(
                    Uuid::new_v4(),
                    crate::ai::Pending::ReviewMutation {
                        id: before.draft.id,
                        generation: state.review_generation,
                    },
                );
            }
            4 => {
                state.pending.insert(
                    Uuid::new_v4(),
                    crate::ai::Pending::AppliedReview {
                        id: before.draft.id,
                        generation: state.review_generation,
                    },
                );
            }
            5 => {
                state.rewrite = Some(crate::ai::ActiveRewrite {
                    request: brn_workflow::proposal_rewrite::RewriteRequest {
                        id: Uuid::new_v4(),
                        expected: before.stamp(),
                        selection: brn_workflow::Selection {
                            provider: brn_workflow::Provider::Chatgpt,
                            model: "synthetic".into(),
                        },
                        effort: brn_workflow::ReasoningEffort::Low,
                        generation: 1,
                    },
                    job: None,
                    tool: None,
                    stopping: false,
                });
            }
            _ => {
                state.active = Some(crate::ai::ActiveTurn {
                    request: brn_workflow::chat_worker::AskRequest {
                        id: Uuid::new_v4(),
                        conversation: None,
                        question: "Synthetic active".into(),
                        selection: brn_workflow::Selection {
                            provider: brn_workflow::Provider::Chatgpt,
                            model: "synthetic".into(),
                        },
                        effort: Some(brn_workflow::ReasoningEffort::Low),
                        budget: None,
                        generation: 1,
                    }
                    .into(),
                    partial: String::new(),
                    tool: None,
                    stopping: false,
                    budget_progress: None,
                    time_limit_reached: false,
                });
            }
        }
        assert!(
            state
                .rename_proposal_create(0, "renamed.md".into())
                .is_none(),
            "guard {guard}"
        );
        assert_eq!(state.review.as_ref().unwrap().record, before);
        assert!(!state.review.as_ref().unwrap().create_rename_pending());
    }
}

#[test]
fn state_same_path_acknowledgement_retains_exact_review_and_reports_no_revision() {
    let before = super::tests::fixture();
    let mut state = state(before.clone());
    let (id, _) = state.rename_proposal_create(0, "new.md".into()).unwrap();
    state.apply(id, AppEvent::Proposal(before.clone()));
    assert_eq!(state.review.as_ref().unwrap().record, before);
    assert!(state.review_can_leave());
    assert!(state.notice.contains("unchanged"));
}

#[test]
fn acknowledged_raw_action_fields_keep_their_exact_bytes_through_rename() {
    let before = super::action_tests::fixture();
    let mut review = ProposalReview::new(before);
    let mut fields = review.action_fields()[0].clone();
    fields.values[5] = format!("  {}  ", fields.values[5].replace(' ', " \t "));
    review
        .edit_action_fields(0, fields.clone(), Instant::now())
        .unwrap();
    let (id, edit) = review.prepare_edit().unwrap();
    let acknowledged = super::action_tests::reply(&review.record, &edit);
    assert!(review.acknowledge_edit(id, acknowledged.clone()));
    assert!(review.can_mutate());
    let (id, _) = review
        .prepare_create_rename(0, "renamed.md".into())
        .unwrap();
    assert!(review.acknowledge_create_rename(id, renamed(&acknowledged, 0, "renamed.md")));
    assert_eq!(review.action_fields()[0], fields);
    assert!(review.can_mutate());
}

#[test]
fn observed_current_review_cannot_clear_a_create_rename_failure_navigation_fence() {
    for malformed in [false, true] {
        for changed in [false, true] {
            let before = super::tests::fixture();
            let mut state = state(before.clone());
            let (id, _) = state
                .rename_proposal_create(0, "retained.md".into())
                .unwrap();
            let failure = if malformed {
                let mut after = renamed(&before, 0, "retained.md");
                after.comments[0].text.push_str("forged");
                AppEvent::Proposal(after)
            } else {
                AppEvent::Failed(brn_workflow::WorkflowError::msg("Synthetic rename refusal"))
            };
            state.apply(id, failure);
            assert!(!state.review_can_leave());
            let mut current = before.clone();
            if changed {
                current.version += 1;
                current.updated_at_ms += 1;
                current.draft.title = "Observed newer review title".into();
            }
            let (id, _) = state.refresh_review().unwrap();
            state.apply(id, AppEvent::Proposal(current.clone()));
            let review = state.review.as_mut().unwrap();
            assert_eq!(review.record, before);
            assert_eq!(review.title(), before.draft.title);
            assert_eq!(review.texts(), record_texts(&before));
            assert!(review.error.is_some());
            assert!(!review.can_leave() && !review.can_mutate());
            if changed {
                assert_eq!(review.observed, Some(current.clone()));
                assert!(!review.retry());
                assert!(review.discard_local());
                assert_eq!(review.record, current);
            } else {
                assert!(review.observed.is_none());
                assert!(review.retry());
                assert_eq!(review.record, before);
            }
            assert!(review.can_leave() && review.can_mutate());
        }
    }
}
