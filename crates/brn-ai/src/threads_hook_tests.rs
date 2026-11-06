//! Production hook decisions through Rig's `HookTestHarness`, without a model,
//! transport or tool handler. Effects of these decisions on canonical state
//! stay covered by the scripted-run tests in `threads_argument_correction_tests`
//! and `threads_budget_tests`.
use super::*;
use rig::completion::ResponseIdentity;
use rig::effect::{EffectId, EffectKind};
use rig::message::ToolName;
use rig::test_utils::HookTestHarness;

struct Hooked {
    _root: tempfile::TempDir,
    _scope: Arc<Scope>,
    harness: HookTestHarness,
    invalid: Arc<AtomicBool>,
    limited: Arc<AtomicBool>,
    events: Arc<Mutex<Vec<AiEvent>>>,
}

fn batch_hook(limit: u16, web: bool) -> Hooked {
    let (root, request, _) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let invalid = Arc::new(AtomicBool::new(false));
    let limited = Arc::new(AtomicBool::new(false));
    let events = Arc::new(Mutex::new(Vec::new()));
    let seen = events.clone();
    let harness = HookTestHarness::new()
        .with_streaming(true)
        .with_hook(BatchHook {
            prepare: PrepareChanges::for_scope(&scope),
            external: None,
            skills: Some(scope.skills.clone()),
            invalid: invalid.clone(),
            limited: limited.clone(),
            rounds: AtomicU16::new(0),
            limit,
            emit: Arc::new(move |event| seen.lock().unwrap().push(event)),
            trace: None,
            allowed: None,
            web,
            tokens: None,
            turn_calls: AtomicU16::new(0),
            corrections: AtomicU16::new(0),
            rejected_batch: Mutex::new(None),
        });
    Hooked {
        _root: root,
        _scope: scope,
        harness,
        invalid,
        limited,
        events,
    }
}

fn calls(calls: &[(&str, Value)]) -> Vec<AssistantContent> {
    calls
        .iter()
        .enumerate()
        .map(|(index, (name, args))| {
            AssistantContent::tool_call(
                format!("call_{index}"),
                ToolName::new(*name).unwrap(),
                args.clone(),
            )
        })
        .collect()
}

async fn finish_turn(hooked: &Hooked, turn: usize, content: &[(&str, Value)]) -> ModelTurnAction {
    hooked.harness.set_turn(turn);
    let content = calls(content);
    let identity = ResponseIdentity::default();
    hooked
        .harness
        .hooks()
        .on_model_turn_finished(
            hooked.harness.context(),
            ModelTurnFinished {
                turn,
                content: &content,
                usage: Default::default(),
                identity: &identity,
                finish_reason: None,
                max_tokens: None,
                raw: &Value::Null,
            },
        )
        .await
}

async fn dispatch(hooked: &Hooked, turn: usize, kind: &EffectKind) -> DispatchAction {
    hooked
        .harness
        .hooks()
        .on_dispatch(
            hooked.harness.context(),
            DispatchEvent {
                id: EffectId::from_raw(turn as u64),
                kind,
                turn,
                call_id: None,
                context: None,
            },
        )
        .await
}

fn tool(name: &str, args: Value) -> EffectKind {
    EffectKind::ToolCall {
        name: name.into(),
        args: args.to_string(),
    }
}

fn denial(action: &DispatchAction) -> Option<(rig::error::ErrorKind, String)> {
    match action {
        DispatchAction::Deny(report) => Some((report.kind, report.message.clone())),
        _ => None,
    }
}

fn shape_error() -> Value {
    json!({"ids":7,"query":null})
}

#[tokio::test]
async fn rejected_batch_denies_every_sibling_with_identical_no_effect_feedback() {
    let hooked = batch_hook(8, false);
    let sibling = json!({"summary":"MUST_NOT_EXECUTE"});
    let action = finish_turn(
        &hooked,
        1,
        &[
            ("read_records", shape_error()),
            ("retain_progress", sibling.clone()),
        ],
    )
    .await;
    assert!(matches!(action, ModelTurnAction::Continue), "{action:?}");
    let first = denial(&dispatch(&hooked, 1, &tool("read_records", shape_error())).await);
    let second = denial(&dispatch(&hooked, 1, &tool("retain_progress", sibling)).await);
    let (kind, feedback) = first.clone().expect("the invalid call is denied");
    assert_eq!(kind, rig::error::ErrorKind::Denied);
    assert!(feedback.starts_with("Skipped: no tool in this batch executed."));
    assert!(feedback.contains("correction 1/2"), "{feedback}");
    assert_eq!(first, second, "every sibling sees the same feedback");
    assert!(!hooked.invalid.load(Ordering::SeqCst));
    {
        let events = hooked.events.lock().unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, AiEvent::Activity(t) if t.starts_with("> "))),
            "a skipped sibling records no dispatch: {events:?}"
        );
        assert!(
            !format!("{events:?}").contains("MUST_NOT_EXECUTE"),
            "denials do not log sibling arguments"
        );
    }
    // A later valid turn clears the rejection; the same call now proceeds.
    let action = finish_turn(&hooked, 2, &[("retain_progress", json!({"summary":"ok"}))]).await;
    assert!(matches!(action, ModelTurnAction::Continue));
    let next = dispatch(
        &hooked,
        2,
        &tool("retain_progress", json!({"summary":"ok"})),
    )
    .await;
    assert!(matches!(next, DispatchAction::Proceed), "{next:?}");
}

#[tokio::test]
async fn correction_allowance_is_invocation_total_and_then_stops_the_run() {
    let hooked = batch_hook(8, false);
    for turn in 1..=2 {
        let action = finish_turn(&hooked, turn, &[("read_records", shape_error())]).await;
        assert!(matches!(action, ModelTurnAction::Continue), "{action:?}");
        // A valid turn between corrections never replenishes the allowance.
        let action = finish_turn(
            &hooked,
            turn + 10,
            &[("retain_progress", json!({"summary":"ok"}))],
        )
        .await;
        assert!(matches!(action, ModelTurnAction::Continue));
    }
    let action = finish_turn(&hooked, 3, &[("read_records", shape_error())]).await;
    assert!(
        matches!(&action, ModelTurnAction::Stop(reason) if reason == "tool argument correction allowance exhausted"),
        "{action:?}"
    );
    assert!(hooked.invalid.load(Ordering::SeqCst));
}

#[tokio::test]
async fn forbidden_calls_stop_before_dispatch_and_are_never_corrected() {
    for (web, call) in [
        (false, ("web_search", json!({"query":"synthetic"}))),
        (true, ("unoffered_tool", json!({}))),
        (true, ("read_records", json!("not an object"))),
    ] {
        let hooked = batch_hook(8, web);
        let action = finish_turn(&hooked, 1, std::slice::from_ref(&call)).await;
        assert!(
            matches!(&action, ModelTurnAction::Stop(reason) if reason == "invalid tool batch"),
            "{call:?}: {action:?}"
        );
        assert!(hooked.invalid.load(Ordering::SeqCst), "{call:?}");
        assert!(
            !hooked.events.lock().unwrap().iter().any(
                |e| matches!(e, AiEvent::Activity(t) if t.starts_with("Tool arguments rejected"))
            ),
            "{call:?}: a forbidden call gets no correction"
        );
    }
}

#[tokio::test]
async fn tool_round_budget_stops_the_turn_after_the_limit() {
    let hooked = batch_hook(2, false);
    let progress = [("retain_progress", json!({"summary":"ok"}))];
    for turn in 1..=2 {
        let action = finish_turn(&hooked, turn, &progress).await;
        assert!(matches!(action, ModelTurnAction::Continue), "{action:?}");
    }
    // A text-only turn spends no round.
    assert!(matches!(
        finish_turn(&hooked, 3, &[]).await,
        ModelTurnAction::Continue
    ));
    let action = finish_turn(&hooked, 4, &progress).await;
    assert!(
        matches!(&action, ModelTurnAction::Stop(reason) if reason == "tool budget exhausted"),
        "{action:?}"
    );
    assert!(hooked.limited.load(Ordering::SeqCst));
    assert!(!hooked.invalid.load(Ordering::SeqCst));
}

#[tokio::test]
async fn stale_scope_or_oversized_request_stops_the_model_call_before_dispatch() {
    let (_root, request, _) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let exhausted = Arc::new(AtomicU16::new(0));
    let harness = HookTestHarness::new().with_hook(context_budget::BudgetHook {
        budget: context_budget::Budget::for_scope(&scope).unwrap(),
        exhausted: exhausted.clone(),
        scope: scope.clone(),
    });
    let call = |prompt: String| EffectKind::Completion {
        request: Arc::new(rig::completion::CompletionRequest::new(prompt)),
        stream: true,
    };
    let hooked = |kind: EffectKind| {
        let harness = &harness;
        async move {
            harness
                .hooks()
                .on_dispatch(
                    harness.context(),
                    DispatchEvent {
                        id: EffectId::from_raw(1),
                        kind: &kind,
                        turn: 1,
                        call_id: None,
                        context: None,
                    },
                )
                .await
        }
    };
    assert!(matches!(
        hooked(call("synthetic".into())).await,
        DispatchAction::Proceed
    ));
    // Tool calls are not model requests; the budget hook leaves them alone.
    assert!(matches!(
        hooked(tool("retain_progress", json!({}))).await,
        DispatchAction::Proceed
    ));
    let oversized = hooked(call("x".repeat(64 * 1024 * 1024))).await;
    assert_eq!(
        denial(&oversized),
        Some((
            rig::error::ErrorKind::Cancelled,
            "context request estimate exhausted".into()
        ))
    );
    assert_eq!(exhausted.load(Ordering::SeqCst), 1);
    exhausted.store(0, Ordering::SeqCst);
    // Another writer advanced the Run fence: the Run is no longer current.
    scope.fence.fetch_add(1, Ordering::SeqCst);
    assert_eq!(
        denial(&hooked(call("synthetic".into())).await),
        Some((
            rig::error::ErrorKind::Cancelled,
            "run no longer current".into()
        ))
    );
    assert_eq!(
        exhausted.load(Ordering::SeqCst),
        0,
        "staleness is not budget"
    );
}
