use super::*;

fn ready() -> AiState {
    AiState {
        ready: true,
        vault_bound: true,
        selection: Some(Selection {
            provider: Provider::Copilot,
            model: "synthetic-explicit".into(),
        }),
        effort: Some(ReasoningEffort::Medium),
        ..AiState::default()
    }
}
fn progress(request: &AskRequest, models: u16, rounds: u16) -> AppEvent {
    AppEvent::Chat(ChatEvent::BudgetProgress {
        id: request.id,
        generation: request.generation,
        budget: request.budget.unwrap(),
        model_turns: models,
        tool_rounds: rounds,
    })
}
fn finished(request: &AskRequest, conversation: Uuid) -> WorkTurn {
    WorkTurn {
        id: request.id,
        conversation_id: conversation,
        question: request.question.clone(),
        answer: "Retained partial õ".into(),
        provider: "copilot".into(),
        model: request.selection.model.clone(),
        effort: Some("medium".into()),
        started_at_ms: Some(1),
        finished_at_ms: Some(2),
        status: WorkTurnStatus::Interrupted,
        error_code: Some("time_limit_reached".into()),
    }
}

#[test]
fn default_and_bounded_choices_are_captured_and_frozen_until_finalization() {
    let mut state = ready();
    assert_eq!(
        state.work_budget,
        WorkBudget {
            max_tool_rounds: 8,
            timeout_seconds: 300
        }
    );
    assert!(!state.select_work_rounds(0));
    assert!(!state.select_work_rounds(7));
    assert!(!state.select_work_seconds(3601));
    assert!(state.select_work_rounds(16));
    assert!(state.select_work_seconds(180));
    let selection = state.selection.clone();
    let request = state.ask("Exact owner question õ".into()).unwrap();
    assert_eq!(
        request.budget,
        Some(WorkBudget {
            max_tool_rounds: 16,
            timeout_seconds: 180
        })
    );
    assert_eq!(state.selection, selection);
    assert_eq!(request.question, "Exact owner question õ");
    assert!(!state.select_work_rounds(32));
    assert!(!state.select_work_seconds(60));
    assert_eq!(
        state.active.as_ref().unwrap().request.budget(),
        request.budget
    );
    state.stop();
    assert!(!state.select_work_seconds(60));
    state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::Finished {
            id: request.id,
            generation: request.generation,
            turn: finished(&request, Uuid::new_v4()),
        }),
    );
    assert!(state.select_work_seconds(60));
    assert_eq!(request.budget.unwrap().timeout_seconds, 180);
}

#[test]
fn progress_requires_exact_uuid_generation_and_captured_budget_and_remains_monotonic() {
    let mut state = ready();
    let request = state.ask("Track me".into()).unwrap();
    assert!(
        state
            .active
            .as_ref()
            .unwrap()
            .budget_label()
            .contains("awaiting admission")
    );
    state.apply(request.id, progress(&request, 0, 0));
    state.apply(request.id, progress(&request, 2, 1));
    let mut stale = request.clone();
    stale.generation = stale.generation.wrapping_add(1);
    state.apply(request.id, progress(&stale, 3, 2));
    stale = request.clone();
    stale.id = Uuid::new_v4();
    state.apply(stale.id, progress(&stale, 3, 2));
    stale = request.clone();
    stale.budget.as_mut().unwrap().max_tool_rounds = 16;
    state.apply(request.id, progress(&stale, 3, 2));
    state.apply(Uuid::new_v4(), progress(&request, 3, 2));
    state.apply(request.id, progress(&request, 1, 0));
    state.apply(request.id, progress(&request, 10, 9));
    state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::BudgetStopping {
            id: request.id,
            generation: request.generation.wrapping_add(1),
        }),
    );
    state.apply(
        Uuid::new_v4(),
        AppEvent::Chat(ChatEvent::BudgetStopping {
            id: request.id,
            generation: request.generation,
        }),
    );
    let active = state.active.as_ref().unwrap();
    assert_eq!(active.budget_progress, Some((2, 1)));
    assert!(!active.stopping);
    assert!(
        active
            .budget_label()
            .contains("2 model requests admitted · 1 tool rounds admitted")
    );
}

#[test]
fn timeout_preserves_partial_stop_and_frozen_ceiling_through_navigation_and_drain() {
    let mut state = ready();
    let request = state.ask("Retain exact answer".into()).unwrap();
    state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::Text {
            id: request.id,
            generation: request.generation,
            text: "Partial õ".into(),
        }),
    );
    state.apply(request.id, progress(&request, 1, 1));
    state.navigate(Some(Uuid::new_v4()));
    state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::BudgetStopping {
            id: request.id,
            generation: request.generation,
        }),
    );
    let active = state.active.as_ref().unwrap();
    assert_eq!(
        active.status_label(),
        "Time limit reached; stopping and finalizing"
    );
    assert_eq!(active.partial, "Partial õ");
    assert!(!state.can_ask());
    assert_eq!(state.stop(), Some(request.id));
    assert!(
        state
            .stop_controls()
            .iter()
            .any(|(_, command)| matches!(command, AppCommand::CancelTurn(id) if *id == request.id))
    );
    state.apply(request.id, progress(&request, 2, 2));
    state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::Text {
            id: request.id,
            generation: request.generation,
            text: "late".into(),
        }),
    );
    assert_eq!(state.active.as_ref().unwrap().partial, "Partial õ");
    assert_eq!(state.active.as_ref().unwrap().budget_progress, Some((1, 1)));
    state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::Finished {
            id: request.id,
            generation: request.generation,
            turn: finished(&request, Uuid::new_v4()),
        }),
    );
    assert!(state.active.is_none());
    assert!(state.turns.is_empty());
    assert!(state.notice.contains("Time limit reached"));
    state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::BudgetStopping {
            id: request.id,
            generation: request.generation,
        }),
    );
    assert!(state.active.is_none());
}

#[test]
fn history_uses_recorded_metadata_or_unavailable_and_rejects_late_lookup() {
    let mut state = ready();
    let request = state.ask("A prior turn".into()).unwrap();
    let conversation = Uuid::new_v4();
    let turn = finished(&request, conversation);
    let commands = state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::Finished {
            id: request.id,
            generation: request.generation,
            turn: turn.clone(),
        }),
    );
    let lookup = commands
        .iter()
        .find(|(_, c)| matches!(c, AppCommand::RunBudget(id) if *id == turn.id))
        .unwrap()
        .0;
    state.apply(
        lookup,
        AppEvent::RunBudget {
            id: turn.id,
            budget: request.budget,
        },
    );
    assert!(
        state
            .recorded_budget_label(turn.id)
            .contains("8 tool rounds · 300 seconds")
    );
    let lookup = state.request_run_budget(turn.id).0;
    state.apply(
        lookup,
        AppEvent::RunBudget {
            id: turn.id,
            budget: None,
        },
    );
    assert_eq!(
        state.recorded_budget_label(turn.id),
        "Budget unavailable in recorded history"
    );
    let stale = state.request_run_budget(turn.id).0;
    let (history, _) = state.navigate(Some(conversation)).unwrap();
    let commands = state.apply(history, AppEvent::Turns(vec![turn.clone()]));
    let current = commands
        .iter()
        .find(|(_, c)| matches!(c, AppCommand::RunBudget(id) if *id == turn.id))
        .unwrap()
        .0;
    state.apply(
        stale,
        AppEvent::RunBudget {
            id: turn.id,
            budget: request.budget,
        },
    );
    assert!(!state.run_budgets.contains_key(&turn.id));
    state.apply(
        current,
        AppEvent::RunBudget {
            id: Uuid::new_v4(),
            budget: request.budget,
        },
    );
    assert!(!state.run_budgets.contains_key(&turn.id));
    let lookup = state.request_run_budget(turn.id).0;
    state.apply(
        lookup,
        AppEvent::RunBudget {
            id: turn.id,
            budget: None,
        },
    );
    assert_eq!(state.run_budgets[&turn.id], None);
}

#[cfg(target_os = "macos")]
#[test]
fn inbox_investigation_captures_budget_and_recorded_analysis_keeps_metadata_separate() {
    let (_fixture, source) = super::inbox_analysis_state_tests::source_fixture();
    let mut state = ready();
    state.open_inbox();
    let (inspection, _) = state
        .inspect_inbox_analysis_source(source.source.path.clone())
        .unwrap();
    state.apply(inspection, AppEvent::ProposalSource(Box::new(source)));
    assert!(state.select_work_rounds(4));
    assert!(state.select_work_seconds(60));
    let (id, AppCommand::AnalyzeInboxActions(request)) = state.analyze_inbox_source().unwrap()
    else {
        panic!("owned Inbox investigation");
    };
    assert_eq!(
        request.budget,
        Some(WorkBudget {
            max_tool_rounds: 4,
            timeout_seconds: 60
        })
    );
    assert!(!state.select_work_rounds(16));
    let mut record = super::inbox_analysis_state_tests::symbolic_analysis(&request);
    record.budget = request.budget;
    let capture = serde_json::to_vec(&record.job.capture).unwrap();
    let turn = record.turn.clone().unwrap();
    state.apply(
        id,
        AppEvent::Chat(ChatEvent::Finished {
            id,
            generation: request.generation,
            turn,
        }),
    );
    let (inspection, _) = state.inspect_inbox_analysis(id).unwrap();
    state.apply(inspection, AppEvent::InboxActionAnalysis(Box::new(record)));
    assert_eq!(
        state.inbox_analysis.record.as_ref().unwrap().budget,
        request.budget
    );
    assert_eq!(
        serde_json::to_vec(&state.inbox_analysis.record.as_ref().unwrap().job.capture).unwrap(),
        capture
    );
}
