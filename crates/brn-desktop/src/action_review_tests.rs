use super::*;
use brn_workflow::actions::{ActionOrigin, ActionPriority, ActionRecord, ActionState};

pub(crate) fn fixture() -> ProposalRecord {
    let data = ActionData {
        title: "\u{feff} Exact title 日本語\r\n λ ".into(),
        description: " Exact description 🧭\r\n ".into(),
        state: ActionState::Waiting,
        owner: Some(" Owner λ ".into()),
        related_person: Some(Uuid::from_u128(10)),
        related_project: Some(Uuid::from_u128(11)),
        sources: vec![Uuid::from_u128(12), Uuid::from_u128(13)],
        thread: Some(Uuid::from_u128(14)),
        due_on: Some("2028-02-29".into()),
        follow_up_on: Some("2028-03-01".into()),
        dependencies: vec![Uuid::from_u128(15), Uuid::from_u128(16)],
        parent: Some(Uuid::from_u128(17)),
        follows_up: Some(Uuid::from_u128(18)),
        priority: Some(ActionPriority::High),
    };
    let before = ActionRecord {
        origin: ActionOrigin {
            id: Uuid::from_u128(2),
            proposal: brn_workflow::proposals::ProposalStamp {
                id: Uuid::from_u128(3),
                version: 4,
            },
            data: data.clone(),
            created_at_ms: 7,
        },
        version: 2,
        data: ActionData {
            state: ActionState::Open,
            title: "Before title".into(),
            ..data.clone()
        },
        updated_at_ms: 8,
        waiting_since_ms: None,
        completed_at_ms: None,
    };
    let mut record = super::tests::fixture();
    record.draft.group_id = None;
    record.draft.action_changes = vec![
        ActionChange::Create {
            id: Uuid::from_u128(1),
            data: data.clone(),
        },
        ActionChange::Replace {
            before: Box::new(before),
            data,
        },
    ];
    validate_review_edit(&record, &full_edit(&record)).unwrap();
    record
}

fn full_edit(record: &ProposalRecord) -> ProposalEdit {
    ProposalEdit {
        action_data: record_actions(record),
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: record_texts(record),
    }
}

pub(crate) fn reply(record: &ProposalRecord, edit: &ProposalEdit) -> ProposalRecord {
    let mut reply = record.clone();
    reply.version += u64::from(
        edit.action_data != record_actions(record)
            || edit.title != record.draft.title
            || edit.texts != record_texts(record),
    );
    reply.updated_at_ms += 1;
    reply.draft.title = edit.title.clone();
    for (change, data) in reply.draft.action_changes.iter_mut().zip(&edit.action_data) {
        *change.data_mut() = data.clone();
    }
    reply
}

#[test]
fn exact_all_member_capture_and_action_only_review_are_clean() {
    for action_only in [false, true] {
        let mut record = fixture();
        if action_only {
            record.draft.changes.clear();
            record.draft.sources.clear();
            record.draft.vault = None;
        }
        let review = ProposalReview::new(record.clone());
        assert!(review.can_leave() && review.can_mutate());
        assert_eq!(review.action_data(), record_actions(&record));
        let capture = crate::approval::ApprovalCapture::new(vec![record.clone()], None).unwrap();
        assert_eq!(capture.records(), &[record]);
        assert_eq!(capture.records()[0].draft.action_changes.len(), 2);
    }
}

#[test]
fn full_action_edits_preserve_later_raw_typing_across_acknowledgement() {
    let baseline = fixture();
    let mut review = ProposalReview::new(baseline.clone());
    let now = Instant::now();
    let mut data = review.action_data()[1].clone();
    data.title.push_str("first");
    data.state = ActionState::Blocked;
    review.edit_action(1, data.clone(), now).unwrap();
    let (id, submitted) = review.prepare_edit().unwrap();
    assert_eq!(
        submitted.action_data,
        vec![baseline.draft.action_changes[0].data().clone(), data]
    );
    assert_eq!(submitted.texts, record_texts(&baseline));
    let mut fields = review.action_fields()[0].clone();
    fields.values[3] = "incomplete-uuid".into();
    review.edit_action_fields(0, fields.clone(), now).unwrap();
    let acknowledged = reply(&baseline, &submitted);
    assert!(review.acknowledge_edit(id, acknowledged.clone()));
    assert_eq!(review.action_fields()[0], fields);
    assert!(review.dirty() && !review.can_leave() && !review.can_mutate());
    assert!(review.prepare_edit().is_none());
    assert_eq!(review.record, acknowledged);
    fields.values[3] = Uuid::from_u128(20).to_string();
    review.edit_action_fields(0, fields.clone(), now).unwrap();
    review.retry();
    let (id, submitted) = review.prepare_edit().unwrap();
    assert_eq!(
        submitted.action_data[0].related_person,
        Some(Uuid::from_u128(20))
    );
    assert!(review.acknowledge_edit(id, reply(&acknowledged, &submitted)));
    assert!(review.can_leave());
    assert_eq!(review.action_fields()[0], fields);
}

#[test]
fn acknowledgement_and_observation_refuse_every_changed_action_binding_and_data() {
    let baseline = fixture();
    let now = Instant::now();
    let mut review = ProposalReview::new(baseline.clone());
    let mut data = review.action_data()[0].clone();
    data.description.push_str("edited");
    review.edit_action(0, data.clone(), now).unwrap();
    let (_, submitted) = review.prepare_edit().unwrap();
    let good = reply(&baseline, &submitted);
    let mut bad = vec![];
    let mut changed = good.clone();
    changed.draft.action_changes.swap(0, 1);
    bad.push(changed);
    let mut changed = good.clone();
    changed.draft.action_changes.pop();
    bad.push(changed);
    let mut changed = good.clone();
    if let ActionChange::Create { id, .. } = &mut changed.draft.action_changes[0] {
        *id = Uuid::new_v4();
    }
    bad.push(changed);
    let mut changed = good.clone();
    changed.draft.action_changes[1].data_mut().owner = None;
    bad.push(changed);
    for mutate in [0, 1, 2, 3, 4, 5, 6] {
        let mut changed = good.clone();
        if let ActionChange::Replace { before, .. } = &mut changed.draft.action_changes[1] {
            match mutate {
                0 => before.origin.id = Uuid::new_v4(),
                1 => before.origin.proposal.version += 1,
                2 => before.origin.data.description.push('!'),
                3 => before.origin.created_at_ms += 1,
                4 => before.version += 1,
                5 => before.updated_at_ms += 1,
                _ => before.data.description.push('!'),
            }
        }
        bad.push(changed);
    }
    for changed in bad {
        let mut review = ProposalReview::new(baseline.clone());
        review.edit_action(0, data.clone(), now).unwrap();
        let (id, _) = review.prepare_edit().unwrap();
        assert!(!review.acknowledge_edit(id, changed));
        assert_eq!(review.record, baseline);
        assert_eq!(review.action_data()[0], data);
        assert!(review.dirty() && !review.pending() && review.error.is_some());
    }
    let mut foreign = baseline.clone();
    foreign.draft.action_changes[1] = baseline.draft.action_changes[0].clone();
    let mut clean = ProposalReview::new(baseline.clone());
    assert!(!clean.observe(foreign));
    assert_eq!(clean.record, baseline);
}

#[test]
fn action_conflicts_require_discard_and_clean_observations_adopt_all_data() {
    let baseline = fixture();
    let now = Instant::now();
    let mut review = ProposalReview::new(baseline.clone());
    let mut edit = full_edit(&baseline);
    edit.action_data[1].title = "remote".into();
    let remote = reply(&baseline, &edit);
    assert!(review.observe(remote.clone()));
    assert_eq!(review.action_data(), edit.action_data);
    assert!(review.can_leave());
    review
        .edit_action(
            0,
            ActionData {
                title: "local".into(),
                ..edit.action_data[0].clone()
            },
            now,
        )
        .unwrap();
    let mut edit = full_edit(&remote);
    edit.action_data[0].description = "remote update".into();
    let latest = reply(&remote, &edit);
    assert!(review.observe(latest.clone()));
    assert_eq!(review.action_data()[0].title, "local");
    assert!(!review.can_leave() && review.prepare_edit().is_none());
    assert!(review.discard_local());
    assert_eq!(review.record, latest);
    assert_eq!(review.action_data(), edit.action_data);
    assert!(review.can_leave());
}

#[test]
fn invalid_fields_keep_exact_bytes_and_cannot_be_approved_or_silently_normalized() {
    let baseline = fixture();
    let now = Instant::now();
    for (field, invalid) in [
        (0, " \r\n"),
        (3, "bad UUID"),
        (7, "2027-02-29"),
        (9, "partial-"),
    ] {
        let mut review = ProposalReview::new(baseline.clone());
        let mut form = review.action_fields()[0].clone();
        form.values[field] = invalid.into();
        review.edit_action_fields(0, form.clone(), now).unwrap();
        assert!(review.prepare_edit().is_none());
        assert_eq!(review.action_fields()[0], form);
        assert!(review.error.is_some() && !review.can_leave() && !review.can_mutate());
        assert!(review.discard_local());
        assert_eq!(review.action_data(), record_actions(&baseline));
    }
    let mut grouped = baseline.clone();
    let group = Uuid::new_v4();
    grouped.draft.group_id = Some(group);
    assert!(crate::approval::ApprovalCapture::new(vec![grouped], Some(group)).is_some());
    let mut invalid = baseline;
    invalid.draft.action_changes[1].data_mut().state = ActionState::Completed;
    assert!(crate::approval::ApprovalCapture::new(vec![invalid], None).is_none());
}

#[test]
fn typed_action_edit_never_normalizes_invalid_optional_data() {
    let mut review = ProposalReview::new(fixture());
    let mut data = review.action_data()[0].clone();
    data.owner = Some(String::new());
    review.edit_action(0, data.clone(), Instant::now()).unwrap();
    assert_eq!(review.action_data()[0], data);
    assert!(review.prepare_edit().is_none());
    assert_eq!(review.action_data()[0].owner, Some(String::new()));
}

fn action_state() -> crate::ai::AiState {
    let mut record = fixture();
    record.draft.changes.clear();
    record.draft.sources.clear();
    record.draft.vault = None;
    let mut state = crate::ai::AiState::default();
    state.ready = true;
    state.vault_bound = false;
    state.review = Some(ProposalReview::new(record));
    state
}

#[test]
fn action_only_no_vault_capture_and_confirmation_keep_full_binding_and_all_existing_guards() {
    let mut state = action_state();
    let capture = state.capture_approval(false).unwrap();
    assert_eq!(capture.records()[0], state.review.as_ref().unwrap().record);
    assert!(state.confirm_approval(&capture).is_some());
    assert!(
        state.application_busy()
            && state.capture_approval(false).is_none()
            && state.confirm_approval(&capture).is_none()
    );
    for guard in 0..7 {
        let mut state = action_state();
        let capture = state.capture_approval(false).unwrap();
        match guard {
            0 => state.ready = false,
            1 => {
                state
                    .review
                    .as_mut()
                    .unwrap()
                    .edit_action(
                        0,
                        ActionData {
                            title: "dirty".into(),
                            ..capture.records()[0].draft.action_changes[0].data().clone()
                        },
                        Instant::now(),
                    )
                    .unwrap();
            }
            2 => {
                let review = state.review.as_mut().unwrap();
                review.edit_title("queued".into(), Instant::now()).unwrap();
                review.prepare_edit().unwrap();
            }
            3 => {
                state.pending.insert(
                    Uuid::new_v4(),
                    crate::ai::Pending::ReviewMutation {
                        id: capture.records()[0].draft.id,
                        generation: state.review_generation,
                    },
                );
            }
            4 => {
                let mut changed = capture.records()[0].clone();
                changed.version += 1;
                changed.draft.action_changes[1]
                    .data_mut()
                    .description
                    .push_str("changed");
                state.review = Some(ProposalReview::new(changed));
            }
            5 => {
                state.rewrite = Some(crate::ai::ActiveRewrite {
                    request: brn_workflow::proposal_rewrite::RewriteRequest {
                        id: Uuid::new_v4(),
                        expected: capture.records()[0].stamp(),
                        selection: brn_workflow::Selection {
                            provider: brn_workflow::Provider::Chatgpt,
                            model: "gpt-6-luna".into(),
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
                state.pending.insert(
                    Uuid::new_v4(),
                    crate::ai::Pending::AppliedReview {
                        id: capture.records()[0].draft.id,
                        generation: state.review_generation,
                    },
                );
            }
        }
        assert!(state.confirm_approval(&capture).is_none());
        if guard != 4 {
            assert!(state.capture_approval(false).is_none());
        }
    }
    let mut state = action_state();
    state.review = Some(ProposalReview::new(fixture()));
    let capture = crate::approval::ApprovalCapture::new(
        vec![state.review.as_ref().unwrap().record.clone()],
        None,
    )
    .unwrap();
    assert!(state.capture_approval(false).is_none() && state.confirm_approval(&capture).is_none());
    let mut state = action_state();
    let mut sourced = state.review.as_ref().unwrap().record.clone();
    sourced.draft.sources = fixture().draft.sources;
    sourced.draft.vault = fixture().draft.vault;
    state.review = Some(ProposalReview::new(sourced.clone()));
    let capture = crate::approval::ApprovalCapture::new(vec![sourced], None).unwrap();
    assert!(state.capture_approval(false).is_none() && state.confirm_approval(&capture).is_none());
    let mut state = action_state();
    let mut grouped = state.review.as_ref().unwrap().record.clone();
    grouped.draft.group_id = Some(Uuid::new_v4());
    state.review = Some(ProposalReview::new(grouped));
    assert!(state.capture_approval(true).is_some());
}

#[test]
fn action_groups_capture_exact_mixed_members_and_preserve_unselected_arrivals() {
    let group = Uuid::new_v4();
    let mut state = action_state();
    state.vault_bound = true;
    let mut operational = state.review.as_ref().unwrap().record.clone();
    operational.draft.group_id = Some(group);
    let mut note = super::tests::fixture();
    note.draft.group_id = Some(group);
    let expected = vec![operational.clone(), note.clone()];
    state.proposals = expected.clone();
    state.review = Some(ProposalReview::new(operational.clone()));
    let capture = state.capture_approval(true).unwrap();
    assert_eq!(capture.records(), expected);
    assert_eq!(
        capture.records()[0].draft.action_changes,
        operational.draft.action_changes
    );
    let mut arrival = super::tests::fixture();
    arrival.draft.group_id = Some(group);
    state.proposals.push(arrival);
    assert!(state.confirm_approval(&capture).is_some());
    let mut foreign = note.clone();
    foreign.draft.id = Uuid::new_v4();
    foreign.draft.vault.as_mut().unwrap().id = Uuid::new_v4();
    assert!(
        crate::approval::ApprovalCapture::new(vec![operational, note, foreign], Some(group))
            .is_none()
    );
}

#[test]
fn file_free_action_bound_to_a_vault_does_not_use_the_vaultless_ui_exception() {
    let mut state = action_state();
    let mut bound = state.review.as_ref().unwrap().record.clone();
    bound.draft.vault = fixture().draft.vault;
    state.review = Some(ProposalReview::new(bound.clone()));
    let capture = crate::approval::ApprovalCapture::new(vec![bound], None).unwrap();
    assert!(state.capture_approval(false).is_none());
    assert!(state.confirm_approval(&capture).is_none());
}

#[test]
fn action_edit_noop_acknowledgement_checks_version_and_immutable_bindings() {
    let baseline = fixture();
    let now = Instant::now();
    let mut review = ProposalReview::new(baseline.clone());
    let data = review.action_data()[0].clone();
    review
        .edit_action(
            0,
            ActionData {
                title: "temporary".into(),
                ..data.clone()
            },
            now,
        )
        .unwrap();
    review.edit_action(0, data, now).unwrap();
    assert!(review.dirty());
    let (id, edit) = review.prepare_edit().unwrap();
    let same = reply(&baseline, &edit);
    assert_eq!(same.version, baseline.version);
    assert!(review.acknowledge_edit(id, same));
    assert!(review.can_leave());
}
