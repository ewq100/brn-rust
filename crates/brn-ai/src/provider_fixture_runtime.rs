//! Offline provider qualification harness; unreachable in the shipped Threads runtime.
use crate::auth::OwnedClient;
use crate::tools::{
    ListActions, ListNotes, ReadAction, ReadConflicts, ReadNote, ReadNoteRange, ReadRawEvidence,
    SearchNotes, ToolRounds,
};
use crate::{AiError, AiErrorKind, Provider, ProviderClient, ReadTools};
use rig::agent::{AgentHook, HookContext, ModelTurnAction, ModelTurnFinished};
use rig::completion::Message;
use rig::message::AssistantContent;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU16, Ordering},
};
use tokio_util::sync::CancellationToken;

use crate::chat::{
    AiAnswer, AiEvent, AiTerminal, HistoryPair, MAX_REWRITE_BYTES, ReasoningEffort, collect_stream,
};
#[derive(Clone, Copy)]
enum RunMode {
    Answer,
    AnswerWithEffort {
        responses: bool,
        effort: ReasoningEffort,
        max_tool_rounds: u16,
        progress: bool,
    },
    Rewrite {
        responses: bool,
        effort: ReasoningEffort,
    },
}

struct RoundHook {
    rounds: Mutex<ToolRounds>,
    limited: Arc<AtomicBool>,
    model_turns: AtomicU16,
    limit: u16,
    progress: Option<Arc<dyn Fn(AiEvent) + Send + Sync>>,
}

impl AgentHook for RoundHook {
    async fn on_model_turn_finished(
        &self,
        _: &HookContext,
        event: ModelTurnFinished<'_>,
    ) -> ModelTurnAction {
        let contains_tools = event
            .content
            .iter()
            .any(|c| matches!(c, AssistantContent::ToolCall(_)));
        let (refused, used) = {
            let mut rounds = self.rounds.lock().expect("run-owned budget");
            (rounds.admit(contains_tools).is_err(), rounds.used())
        };
        // Rig bounds this hook to at most 33 completed responses per run.
        let completed = self.model_turns.fetch_add(1, Ordering::SeqCst) + 1;
        if let Some(emit) = &self.progress {
            emit(AiEvent::BudgetProgress {
                model_turns: completed,
                tool_rounds: used,
                max_tool_rounds: self.limit,
            });
        }
        if refused {
            self.limited.store(true, Ordering::SeqCst);
            return ModelTurnAction::Stop("tool-round budget exhausted".into());
        }
        ModelTurnAction::Continue
    }
}

pub async fn answer(
    client: ProviderClient,
    question: &str,
    history: &[HistoryPair],
    tools: Arc<dyn ReadTools>,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer {
    let model = client.selection().model.clone();
    match client.inner {
        OwnedClient::Chatgpt(client) => {
            answer_model(
                client.completion(model),
                question,
                history,
                tools,
                cancel,
                emit,
            )
            .await
        }
        OwnedClient::Copilot(client) => {
            answer_model(
                client.completion(model),
                question,
                history,
                tools,
                cancel,
                emit,
            )
            .await
        }
    }
}

/// Answers with the captured provider, model and explicit reasoning effort.
/// History, read tools and provisional text use the ordinary Ask stream policy.
pub async fn answer_with_effort(
    client: ProviderClient,
    question: &str,
    history: &[HistoryPair],
    effort: ReasoningEffort,
    tools: Arc<dyn ReadTools>,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer {
    answer_with_tools(
        client,
        question,
        history,
        effort,
        tools,
        None,
        cancel,
        emit,
        &[],
        8,
        false,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn answer_with_tools(
    client: ProviderClient,
    question: &str,
    history: &[HistoryPair],
    effort: ReasoningEffort,
    tools: Arc<dyn ReadTools>,
    proposals: Option<Arc<dyn crate::ProposalTools>>,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
    images: &[crate::VisualImage],
    max_tool_rounds: u16,
    progress: bool,
) -> AiAnswer {
    if !(1..=32).contains(&max_tool_rounds) {
        return AiAnswer {
            text: String::new(),
            terminal: AiTerminal::Failed(AiError::new(AiErrorKind::ToolRejected)),
        };
    }
    let selection = client.selection();
    let model = selection.model.clone();
    let mode = RunMode::AnswerWithEffort {
        responses: selection.provider == Provider::Chatgpt
            || rig::providers::copilot::wire::routes_through_responses(&model),
        effort,
        max_tool_rounds,
        progress,
    };
    match client.inner {
        OwnedClient::Chatgpt(client) => {
            run_model_images(
                client.completion(model),
                question,
                history,
                tools,
                proposals,
                cancel,
                emit,
                mode,
                images,
            )
            .await
        }
        OwnedClient::Copilot(client) => {
            run_model_images(
                client.completion(model),
                question,
                history,
                tools,
                proposals,
                cancel,
                emit,
                mode,
                images,
            )
            .await
        }
    }
}

/// Ask with one captured application proposal capability; real changes still need approval.
#[allow(clippy::too_many_arguments)]
pub async fn answer_with_proposals(
    client: ProviderClient,
    question: &str,
    history: &[HistoryPair],
    effort: ReasoningEffort,
    tools: Arc<dyn ReadTools>,
    proposals: Arc<dyn crate::ProposalTools>,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer {
    answer_with_tools(
        client,
        question,
        history,
        effort,
        tools,
        Some(proposals),
        cancel,
        emit,
        &[],
        8,
        false,
    )
    .await
}

/// Private image collections use the same selected model, tools and proposal
/// lane as text evidence; image interpretation does not create another runtime.
#[allow(clippy::too_many_arguments)]
pub async fn answer_with_proposals_and_images(
    client: ProviderClient,
    question: &str,
    history: &[HistoryPair],
    effort: ReasoningEffort,
    tools: Arc<dyn ReadTools>,
    proposals: Arc<dyn crate::ProposalTools>,
    images: &[crate::VisualImage],
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer {
    answer_with_tools(
        client,
        question,
        history,
        effort,
        tools,
        Some(proposals),
        cancel,
        emit,
        images,
        8,
        false,
    )
    .await
}

/// Investigation through the same Rig lane with a captured tool-round ceiling.
/// Workflow owns time limits, durable metadata and the shared cancellation token.
#[allow(clippy::too_many_arguments)]
pub async fn answer_with_proposals_and_images_with_limit(
    client: ProviderClient,
    question: &str,
    history: &[HistoryPair],
    effort: ReasoningEffort,
    tools: Arc<dyn ReadTools>,
    proposals: Arc<dyn crate::ProposalTools>,
    images: &[crate::VisualImage],
    max_tool_rounds: u16,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer {
    answer_with_tools(
        client,
        question,
        history,
        effort,
        tools,
        Some(proposals),
        cancel,
        emit,
        images,
        max_tool_rounds,
        true,
    )
    .await
}

/// Generates a bounded suggestion for one captured review. Only a successful
/// final response returns raw JSON; workflow validates and version-guards it.
pub async fn rewrite(
    client: ProviderClient,
    prompt: &str,
    effort: ReasoningEffort,
    tools: Arc<dyn ReadTools>,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer {
    if prompt.len() > MAX_REWRITE_BYTES {
        return AiAnswer {
            text: String::new(),
            terminal: AiTerminal::Failed(AiError::new(AiErrorKind::ToolRejected)),
        };
    }
    let selection = client.selection();
    let model = selection.model.clone();
    let mode = RunMode::Rewrite {
        responses: selection.provider == Provider::Chatgpt
            || rig::providers::copilot::wire::routes_through_responses(&model),
        effort,
    };
    match client.inner {
        OwnedClient::Chatgpt(client) => {
            run_model(
                client.completion(model),
                prompt,
                &[],
                tools,
                None,
                cancel,
                emit,
                mode,
            )
            .await
        }
        OwnedClient::Copilot(client) => {
            run_model(
                client.completion(model),
                prompt,
                &[],
                tools,
                None,
                cancel,
                emit,
                mode,
            )
            .await
        }
    }
}

pub(crate) async fn answer_model(
    model: impl Into<rig_core::DynModel<rig_core::operation::Completion>>,
    question: &str,
    history: &[HistoryPair],
    tools: Arc<dyn ReadTools>,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer {
    run_model(
        model,
        question,
        history,
        tools,
        None,
        cancel,
        emit,
        RunMode::Answer,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_model(
    model: impl Into<rig_core::DynModel<rig_core::operation::Completion>>,
    question: &str,
    history: &[HistoryPair],
    tools: Arc<dyn ReadTools>,
    proposals: Option<Arc<dyn crate::ProposalTools>>,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
    mode: RunMode,
) -> AiAnswer {
    run_model_images(
        model,
        question,
        history,
        tools,
        proposals,
        cancel,
        emit,
        mode,
        &[],
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_model_images(
    model: impl Into<rig_core::DynModel<rig_core::operation::Completion>>,
    question: &str,
    history: &[HistoryPair],
    tools: Arc<dyn ReadTools>,
    proposals: Option<Arc<dyn crate::ProposalTools>>,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
    mode: RunMode,
    images: &[crate::VisualImage],
) -> AiAnswer {
    let input = match crate::visual::evidence_message(question, images) {
        Ok(input) => input,
        Err(error) => {
            return AiAnswer {
                text: String::new(),
                terminal: AiTerminal::Failed(error),
            };
        }
    };
    let limited = Arc::new(AtomicBool::new(false));
    let behavior = if matches!(mode, RunMode::Rewrite { .. }) {
        crate::behavior::AgentBehavior::Rewrite
    } else if let Some(backend) = &proposals {
        if backend.private_intake() {
            if backend.knowledge_enabled() {
                crate::behavior::AgentBehavior::PrivateIntakeKnowledge
            } else {
                crate::behavior::AgentBehavior::PrivateIntakeActions
            }
        } else if backend.knowledge_enabled() {
            crate::behavior::AgentBehavior::InboxKnowledgeReview
        } else {
            crate::behavior::AgentBehavior::ActionReview
        }
    } else {
        crate::behavior::AgentBehavior::Ask
    };
    let preamble = behavior.preamble();
    let (max_tool_rounds, progress) = match mode {
        RunMode::AnswerWithEffort {
            max_tool_rounds,
            progress,
            ..
        } => (max_tool_rounds, progress),
        _ => (8, false),
    };
    let rounds = match ToolRounds::new(max_tool_rounds) {
        Ok(rounds) => rounds,
        Err(error) => {
            return AiAnswer {
                text: String::new(),
                terminal: AiTerminal::Failed(error),
            };
        }
    };
    let mut builder = rig::AgentBuilder::new(model)
        .preamble(&preamble)
        .tool(SearchNotes(tools.clone()))
        .tool(ReadNote(tools.clone()))
        .tool(ReadNoteRange(tools.clone()))
        .tool(ReadRawEvidence(tools.clone()))
        .tool(ListNotes(tools.clone()))
        .tool(ReadAction(tools.clone()))
        .tool(ListActions(tools.clone()))
        .tool(ReadConflicts(tools))
        .add_hook(RoundHook {
            rounds: Mutex::new(rounds),
            limited: limited.clone(),
            model_turns: AtomicU16::new(0),
            limit: max_tool_rounds,
            progress: progress.then(|| emit.clone()),
        });
    if behavior.actions()
        && let Some(proposals) = proposals
    {
        if behavior.knowledge() {
            builder = builder
                .tool(crate::proposal_tools::ProposeKnowledge(proposals.clone()))
                .tool(crate::proposal_tools::ReportConflict(proposals.clone()));
        }
        builder = builder.tool(crate::proposal_tools::ProposeActions(proposals));
    }
    if let RunMode::AnswerWithEffort {
        responses, effort, ..
    }
    | RunMode::Rewrite { responses, effort } = mode
    {
        builder = builder.additional_params(if responses {
            serde_json::json!({"reasoning":{"effort":effort.as_str()}})
        } else {
            serde_json::json!({"reasoning_effort":effort.as_str()})
        });
    }
    let agent = builder.build();
    let history = history[history.len().saturating_sub(20)..]
        .iter()
        .flat_map(|pair| {
            [
                Message::user(pair.question.clone()),
                Message::assistant(pair.answer.clone()),
            ]
            .into_iter()
            .take(if pair.answer.is_empty() { 1 } else { 2 })
        })
        .collect::<Vec<_>>();
    let stream = agent
        .prompt(input)
        .history(history)
        .max_turns(usize::from(max_tool_rounds) + 1)
        .max_invalid_tool_call_retries(0)
        .tool_concurrency(2)
        .stream();
    collect_stream(
        stream,
        cancel,
        emit,
        limited,
        matches!(mode, RunMode::Rewrite { .. }).then_some(MAX_REWRITE_BYTES),
    )
    .await
}
