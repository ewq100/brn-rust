//! Lifecycle UI guards over exact synthetic worker replies; no inference or vault effects.
use super::*;
use brn_workflow::conversations::{
    ConversationFilter, ConversationLifecycle, ConversationLifecycleReceipt,
    ConversationLifecycleRequest, ConversationLifecycleResult, ConversationStamp,
    ConversationState, ConversationSummary,
};

pub(crate) fn summary(id: Uuid, version: u64, state: ConversationState) -> ConversationSummary {
    ConversationSummary {
        conversation: WorkConversation {
            id,
            title: "Retained rehearsal õ\r\n".into(),
            turns: 3,
            created_at_ms: 100,
            last_activity_at_ms: Some(200),
        },
        lifecycle: ConversationLifecycle {
            stamp: ConversationStamp { id, version },
            state,
        },
    }
}
pub(crate) fn ready(id: Uuid) -> AiState {
    let mut state = AiState {
        ready: true,
        vault_bound: true,
        conversation: Some(id),
        generation: 41,
        selection: Some(Selection {
            provider: Provider::Chatgpt,
            model: "synthetic-model".into(),
        }),
        effort: Some(ReasoningEffort::High),
        ..Default::default()
    };
    let selected = summary(id, 1, ConversationState::Active);
    state
        .session_history
        .lifecycle
        .insert(id, selected.lifecycle);
    state.session_history.summaries.push(selected);
    state
}
/// Existing presentation fixtures must now acknowledge the selected lifecycle
/// rather than infer Active from a Turns payload or a completed historical turn.
pub(crate) fn acknowledge_active(state: &mut AiState) {
    if let Some(id) = state.conversation {
        let query = state.request_selected_lifecycle().unwrap();
        state.apply(
            query.0,
            AppEvent::ConversationLifecycle(summary(id, 1, ConversationState::Active).lifecycle),
        );
    }
}
pub(crate) fn acknowledge_legacy_review(state: &mut AiState) {
    let query = state.request_review_lifecycle().unwrap();
    state.apply(
        query.0,
        AppEvent::Failed(brn_workflow::WorkflowError {
            kind: brn_workflow::ErrorKind::NotFound,
            message: "Synthetic legacy proposal session is absent".into(),
        }),
    );
}
pub(crate) fn result(request: &ConversationLifecycleRequest) -> ConversationLifecycleResult {
    let before = ConversationLifecycle {
        stamp: request.expected,
        state: match request.target {
            ConversationState::Active => ConversationState::Archived,
            ConversationState::Archived => ConversationState::Active,
        },
    };
    let after = ConversationLifecycle {
        stamp: ConversationStamp {
            id: request.expected.id,
            version: request.expected.version + 1,
        },
        state: request.target,
    };
    ConversationLifecycleResult {
        receipt: ConversationLifecycleReceipt {
            request: request.clone(),
            before,
            after,
        },
        current: after,
    }
}
fn change(state: &mut AiState, target: ConversationState) -> (Uuid, ConversationLifecycleRequest) {
    let (id, AppCommand::SetConversationLifecycle(request)) =
        state.set_selected_session_lifecycle(target).unwrap()
    else {
        panic!("exact lifecycle command")
    };
    assert_eq!(id, request.operation_id);
    (id, request)
}
pub(crate) fn turn(conversation_id: Uuid) -> WorkTurn {
    WorkTurn {
        id: Uuid::new_v4(),
        conversation_id,
        question: "Exact owner question õ\r\n".into(),
        answer: "Retained partial answer õ\r\n".into(),
        provider: "chatgpt".into(),
        model: "synthetic-model".into(),
        effort: Some("high".into()),
        started_at_ms: Some(150),
        finished_at_ms: Some(200),
        status: WorkTurnStatus::Failed,
        error_code: Some("time_limit_reached".into()),
    }
}

#[test]
fn session_lifecycle_archive_and_restore_keep_history_budgets_and_exact_owner_buffers() {
    let id = Uuid::new_v4();
    let mut state = ready(id);
    let turn = turn(id);
    state.turns.push(turn.clone());
    state
        .run_budgets
        .insert(turn.id, Some(WorkBudget::default()));
    state.unsaved = Some(turn.clone());
    state.editor = Some(SimpleEditor::new(super::tests::editor_view(
        "owner.md",
        "Saved õ\r\n",
    )));
    state
        .editor
        .as_mut()
        .unwrap()
        .edit("Unacknowledged owner edit õ\r\n".into(), Instant::now())
        .unwrap();
    let (record, _) = crate::review::predecessor_tests::fixture();
    state.review = Some(crate::review::ProposalReview::new(record));
    state
        .review
        .as_mut()
        .unwrap()
        .edit_text(0, "Unacknowledged review õ\r\n".into(), Instant::now())
        .unwrap();
    state.review_error = Some("Retained review diagnostic".into());
    state.notice = "Retained partial/error diagnostic".into();
    let (operation, request) = change(&mut state, ConversationState::Archived);
    assert_eq!(request.expected, ConversationStamp { id, version: 1 });
    assert!(state.navigate(None).is_none());
    assert!(state.navigate(Some(Uuid::new_v4())).is_none());
    assert!(
        state
            .select_session_filter(ConversationFilter::Archived)
            .is_none()
    );
    assert_eq!(state.conversation, Some(id));
    assert_eq!(state.generation, 41);
    let followups = state.apply(
        operation,
        AppEvent::ConversationLifecycleChanged(result(&request)),
    );
    assert_eq!(followups.len(), 1);
    assert!(matches!(
        followups[0].1,
        AppCommand::ConversationSummaries(ConversationFilter::Active)
    ));
    assert_eq!(state.conversation, Some(id));
    assert_eq!(state.generation, 41);
    assert_eq!(state.turns[0].answer.as_bytes(), turn.answer.as_bytes());
    assert_eq!(state.turns[0].started_at_ms, Some(150));
    assert_eq!(state.run_budgets[&turn.id], Some(WorkBudget::default()));
    assert_eq!(state.unsaved.as_ref().unwrap().answer, turn.answer);
    assert_eq!(
        state.editor.as_ref().unwrap().text,
        "Unacknowledged owner edit õ\r\n"
    );
    assert_eq!(
        state.review.as_ref().unwrap().text(0),
        Some("Unacknowledged review õ\r\n")
    );
    assert_eq!(
        state.review_error.as_deref(),
        Some("Retained review diagnostic")
    );
    assert_eq!(state.notice, "Retained partial/error diagnostic");
    assert!(state.session_history.summaries.is_empty());
    assert!(!state.session_allows_new_work());
    assert!(state.ask("forbidden".into()).is_none());
    let (restore, request) = change(&mut state, ConversationState::Active);
    assert_eq!(request.expected.version, 2);
    state.apply(
        restore,
        AppEvent::ConversationLifecycleChanged(result(&request)),
    );
    assert_eq!(state.selected_session_lifecycle().unwrap().stamp.version, 3);
    assert!(state.session_allows_new_work());
    assert_eq!(state.turns[0].answer, turn.answer);
    assert_eq!(state.conversation, Some(id));
    assert!(state.active.is_none());
}

#[test]
fn session_lifecycle_archived_gate_blocks_ask_inbox_and_bound_rewrite_until_restore() {
    let id = Uuid::new_v4();
    let mut state = ready(id);
    let (mut record, _) = crate::review::predecessor_tests::fixture();
    record.draft.session_id = Some(id);
    state.review = Some(crate::review::ProposalReview::new(record));
    state.open_analysis_view();
    state.inbox_analysis.source = Some(brn_workflow::proposals::ProposalSource {
        source: brn_workflow::proposals::SourceVersion {
            path: "source.md".into(),
            fingerprint: serde_json::from_value(
                serde_json::json!({"device":1,"inode":2,"len":4,"sha256":vec![0;32]}),
            )
            .unwrap(),
        },
        text: "body".into(),
    });
    assert!(state.can_ask());
    assert!(state.can_analyze_inbox_source());
    assert!(state.can_rewrite());
    let (operation, request) = change(&mut state, ConversationState::Archived);
    assert!(!state.can_ask());
    assert!(!state.can_analyze_inbox_source());
    assert!(!state.can_rewrite());
    state.apply(
        operation,
        AppEvent::ConversationLifecycleChanged(result(&request)),
    );
    assert!(!state.can_ask());
    assert!(state.analyze_inbox_source().is_none());
    assert!(state.start_rewrite().is_none());
    let (operation, request) = change(&mut state, ConversationState::Active);
    state.apply(
        operation,
        AppEvent::ConversationLifecycleChanged(result(&request)),
    );
    assert!(state.can_ask());
    assert!(state.can_analyze_inbox_source());
    assert!(state.can_rewrite());
    assert!(state.active.is_none());
    assert!(state.rewrite.is_none());
}

#[test]
fn session_lifecycle_receipts_require_exact_operation_transition_current_and_generations() {
    for mode in 0..8 {
        let id = Uuid::new_v4();
        let mut state = ready(id);
        let (operation, request) = change(&mut state, ConversationState::Archived);
        let mut reply = result(&request);
        match mode {
            0 => reply.receipt.request.operation_id = Uuid::new_v4(),
            1 => reply.receipt.before.stamp.version += 1,
            2 => reply.receipt.before.state = ConversationState::Archived,
            3 => reply.receipt.after.stamp.version += 1,
            4 => reply.current.stamp.id = Uuid::new_v4(),
            5 => reply.current.state = ConversationState::Active,
            6 => {
                reply.current.stamp.version = 3;
                reply.current.state = ConversationState::Archived;
            }
            _ => reply.current.stamp.version = 0,
        }
        state.apply(
            Uuid::new_v4(),
            AppEvent::ConversationLifecycleChanged(reply.clone()),
        );
        assert!(state.session_change_pending());
        assert!(
            state
                .apply(operation, AppEvent::ConversationLifecycleChanged(reply))
                .is_empty()
        );
        assert!(!state.session_change_pending());
        assert!(state.session_history.error.is_some());
        assert_eq!(
            state.selected_session_lifecycle().unwrap().state,
            ConversationState::Active
        );
        assert_eq!(state.selected_session_lifecycle().unwrap().stamp.version, 1);
        assert_eq!(state.conversation, Some(id));
    }
    let id = Uuid::new_v4();
    let mut state = ready(id);
    let (operation, request) = change(&mut state, ConversationState::Archived);
    let newer = Uuid::new_v4();
    state.conversation = Some(newer);
    state.generation += 1;
    state.session_history.filter = ConversationFilter::Archived;
    state.apply(
        operation,
        AppEvent::ConversationLifecycleChanged(result(&request)),
    );
    assert!(!state.session_change_pending());
    assert_eq!(state.conversation, Some(newer));
    assert_eq!(state.session_history.filter, ConversationFilter::Archived);
    assert_eq!(state.session_history.lifecycle[&id].stamp.version, 1);
}

#[test]
fn session_lifecycle_replay_current_and_late_acks_never_regress_newer_metadata() {
    let id = Uuid::new_v4();
    let mut state = ready(id);
    let (operation, request) = change(&mut state, ConversationState::Archived);
    let original = result(&request);
    let mut replay = original.clone();
    replay.current = summary(id, 3, ConversationState::Active).lifecycle;
    state.apply(operation, AppEvent::ConversationLifecycleChanged(replay));
    assert_eq!(state.selected_session_lifecycle().unwrap().stamp.version, 3);
    assert!(state.can_ask());
    state.apply(operation, AppEvent::ConversationLifecycleChanged(original));
    assert_eq!(state.selected_session_lifecycle().unwrap().stamp.version, 3);
    assert!(state.can_ask());
    let (operation, request) = change(&mut state, ConversationState::Archived);
    state
        .session_history
        .lifecycle
        .insert(id, summary(id, 5, ConversationState::Active).lifecycle);
    state.apply(
        operation,
        AppEvent::ConversationLifecycleChanged(result(&request)),
    );
    assert_eq!(state.selected_session_lifecycle().unwrap().stamp.version, 5);
    assert!(state.session_history.error.is_some());
}

#[test]
fn session_lifecycle_lists_freeze_generations_validate_whole_shapes_and_keep_selected_archived_history()
 {
    let id = Uuid::new_v4();
    let mut state = ready(id);
    let old = state.refresh_session_summaries();
    let current = state
        .select_session_filter(ConversationFilter::Archived)
        .unwrap();
    state.apply(
        old.0,
        AppEvent::ConversationSummaries {
            filter: ConversationFilter::Active,
            summaries: vec![summary(id, 1, ConversationState::Active)],
        },
    );
    assert!(state.session_history.summaries.is_empty());
    state.apply(
        current.0,
        AppEvent::ConversationSummaries {
            filter: ConversationFilter::Archived,
            summaries: vec![summary(id, 2, ConversationState::Archived)],
        },
    );
    assert_eq!(state.conversation, Some(id));
    assert!(!state.can_ask());
    let query = state
        .select_session_filter(ConversationFilter::Active)
        .unwrap();
    state.apply(
        query.0,
        AppEvent::ConversationSummaries {
            filter: ConversationFilter::Active,
            summaries: vec![summary(id, 1, ConversationState::Active)],
        },
    );
    assert!(state.session_history.summaries.is_empty());
    assert!(!state.can_ask());
    let query = state.refresh_session_summaries();
    let other = Uuid::new_v4();
    let mut invalid = summary(other, 1, ConversationState::Active);
    invalid.lifecycle.stamp.id = id;
    state.apply(
        query.0,
        AppEvent::ConversationSummaries {
            filter: ConversationFilter::Active,
            summaries: vec![summary(other, 1, ConversationState::Active), invalid],
        },
    );
    assert!(!state.session_history.lifecycle.contains_key(&other));
    assert!(state.session_history.error.is_some());
    assert_eq!(state.session_history.lifecycle[&id].stamp.version, 2);
    assert_eq!(state.conversation, Some(id));
}

#[test]
fn session_lifecycle_selected_reads_and_legacy_review_absence_have_distinct_guards() {
    let id = Uuid::new_v4();
    let mut state = ready(id);
    state.session_history.lifecycle.clear();
    let (history, _) = state.navigate(Some(id)).unwrap();
    assert!(!state.can_ask());
    let queries = state.apply(history, AppEvent::Turns(vec![]));
    let query = queries
        .into_iter()
        .find(
            |(_, command)| matches!(command,AppCommand::ConversationLifecycle(value) if *value==id),
        )
        .unwrap();
    state.apply(
        query.0,
        AppEvent::ConversationLifecycle(summary(id, 2, ConversationState::Archived).lifecycle),
    );
    assert!(!state.can_ask());
    assert!(state.can_change_session_lifecycle());
    let old = state.request_selected_lifecycle().unwrap();
    state.navigate(None);
    state.apply(
        old.0,
        AppEvent::ConversationLifecycle(summary(id, 3, ConversationState::Active).lifecycle),
    );
    assert_eq!(state.session_history.lifecycle[&id].stamp.version, 2);
    let (mut record, _) = crate::review::predecessor_tests::fixture();
    let legacy = Uuid::new_v4();
    record.draft.session_id = Some(legacy);
    state.review = Some(crate::review::ProposalReview::new(record));
    assert!(!state.can_rewrite());
    let query = state.request_review_lifecycle().unwrap();
    state.apply(
        query.0,
        AppEvent::Failed(brn_workflow::WorkflowError {
            kind: brn_workflow::ErrorKind::NotFound,
            message: "Historical session is unavailable".into(),
        }),
    );
    assert!(state.can_rewrite());
    assert!(!state.session_history.lifecycle.contains_key(&legacy));
    assert!(
        !state
            .session_history
            .summaries
            .iter()
            .any(|row| row.conversation.id == legacy)
    );
    // Absence is only legacy eligibility when there is no retained lifecycle.
    // A delayed NotFound cannot overturn a known archived version.
    state.session_history.lifecycle.insert(
        legacy,
        summary(legacy, 2, ConversationState::Archived).lifecycle,
    );
    let query = state.request_review_lifecycle().unwrap();
    state.apply(
        query.0,
        AppEvent::Failed(brn_workflow::WorkflowError {
            kind: brn_workflow::ErrorKind::NotFound,
            message: "Older absence observation".into(),
        }),
    );
    assert!(!state.can_rewrite());
}

#[test]
fn session_lifecycle_busy_and_failure_preserve_partial_work_and_clear_only_the_exact_pending_attempt()
 {
    let id = Uuid::new_v4();
    let mut state = ready(id);
    let request = state.ask("Hold the existing lane".into()).unwrap();
    state.active.as_mut().unwrap().partial = "Exact live partial õ\r\n".into();
    assert!(!state.can_change_session_lifecycle());
    assert!(
        state
            .set_selected_session_lifecycle(ConversationState::Archived)
            .is_none()
    );
    state.stop();
    assert!(!state.can_change_session_lifecycle());
    assert_eq!(state.active.as_ref().unwrap().request.id(), request.id);
    assert_eq!(
        state.active.as_ref().unwrap().partial,
        "Exact live partial õ\r\n"
    );
    state.active = None;
    state.unsaved = Some(turn(id));
    state.notice = "Retained earlier error".into();
    let (operation, _) = change(&mut state, ConversationState::Archived);
    state.apply(
        Uuid::new_v4(),
        AppEvent::ConversationLifecycleChanged(result(&ConversationLifecycleRequest {
            operation_id: Uuid::new_v4(),
            expected: ConversationStamp { id, version: 1 },
            target: ConversationState::Archived,
        })),
    );
    assert!(state.session_change_pending());
    state.apply(
        operation,
        AppEvent::Failed(brn_workflow::WorkflowError {
            kind: brn_workflow::ErrorKind::ToolsBusy,
            message: "Existing investigation is still draining.".into(),
        }),
    );
    assert!(!state.session_change_pending());
    assert!(!state.pending.contains_key(&operation));
    assert_eq!(
        state.session_history.error.as_deref(),
        Some("Existing investigation is still draining.")
    );
    assert_eq!(state.notice, "Retained earlier error");
    assert_eq!(
        state.unsaved.as_ref().unwrap().answer,
        "Retained partial answer õ\r\n"
    );
    assert_eq!(state.conversation, Some(id));
    assert_eq!(state.selected_session_lifecycle().unwrap().stamp.version, 1);
}
