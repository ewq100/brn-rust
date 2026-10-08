use crate::auth::OwnedClient;
use crate::error::map_provider;
use crate::tools::{
    ListActions, ListNotes, ReadAction, ReadConflicts, ReadNote, ReadNoteRange, SearchNotes,
    ToolRounds,
};
use crate::{AiError, AiErrorKind, Provider, ProviderClient, ReadTools};
use futures::StreamExt;
use rig::agent::{
    AgentHook, HookContext, ModelTurnAction, ModelTurnFinished, MultiTurnStreamItem,
    StreamingError, StreamingResult,
};
use rig::completion::{Message, PromptError};
use rig::message::AssistantContent;
use rig::streaming::{Item, StreamEvent};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::sync::CancellationToken;

pub const MAX_REWRITE_BYTES: usize = 50 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Clone, Copy)]
enum RunMode {
    Answer,
    AnswerWithEffort {
        responses: bool,
        effort: ReasoningEffort,
    },
    Rewrite {
        responses: bool,
        effort: ReasoningEffort,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HistoryPair {
    pub question: String,
    pub answer: String,
}

#[derive(Clone, Debug)]
pub enum AiEvent {
    Text(String),
    ToolStarted { name: String },
}

#[derive(Clone, Debug)]
pub enum AiTerminal {
    Completed,
    Interrupted,
    Failed(AiError),
}

#[derive(Clone, Debug)]
pub struct AiAnswer {
    pub text: String,
    pub terminal: AiTerminal,
}

struct RoundHook {
    rounds: Mutex<ToolRounds>,
    limited: Arc<AtomicBool>,
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
        if self
            .rounds
            .lock()
            .expect("run-owned budget")
            .admit(contains_tools)
            .is_err()
        {
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
) -> AiAnswer {
    let selection = client.selection();
    let model = selection.model.clone();
    let mode = RunMode::AnswerWithEffort {
        responses: selection.provider == Provider::Chatgpt
            || rig::providers::copilot::wire::routes_through_responses(&model),
        effort,
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
    let mut builder = rig::AgentBuilder::new(model)
        .preamble(&preamble)
        .tool(SearchNotes(tools.clone()))
        .tool(ReadNote(tools.clone()))
        .tool(ReadNoteRange(tools.clone()))
        .tool(ListNotes(tools.clone()))
        .tool(ReadAction(tools.clone()))
        .tool(ListActions(tools.clone()))
        .tool(ReadConflicts(tools))
        .add_hook(RoundHook {
            rounds: Mutex::new(ToolRounds::default()),
            limited: limited.clone(),
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
    if let RunMode::AnswerWithEffort { responses, effort }
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
        .max_turns(9)
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

fn map_stream_error(error: StreamingError) -> AiError {
    match error {
        StreamingError::Completion(e) | StreamingError::Prompt(PromptError::CompletionError(e)) => {
            map_provider(e)
        }
        StreamingError::Report(report) | StreamingError::Prompt(PromptError::Report(report)) => {
            map_provider(rig::error::ProviderError::Relayed(Box::new(report)))
        }
        StreamingError::Prompt(PromptError::UnknownToolCall { .. }) => {
            AiError::new(AiErrorKind::InvalidToolUse)
        }
        _ => AiError::new(AiErrorKind::Other),
    }
}

pub(crate) async fn collect_stream(
    mut stream: StreamingResult,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
    limited: Arc<AtomicBool>,
    strict_output_limit: Option<usize>,
) -> AiAnswer {
    let mut text = String::new();
    let terminal = loop {
        let item = tokio::select! {
            biased;
            _ = cancel.cancelled() => break AiTerminal::Interrupted,
            item = stream.next() => item,
        };
        match item {
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(Item::Event(
                StreamEvent::Text { text: delta, .. },
            )))) => {
                if strict_output_limit
                    .is_some_and(|limit| delta.len() > limit.saturating_sub(text.len()))
                {
                    break AiTerminal::Failed(AiError::new(AiErrorKind::ToolRejected));
                }
                text.push_str(&delta);
                if strict_output_limit.is_none() {
                    emit(AiEvent::Text(delta));
                }
            }
            Some(Ok(MultiTurnStreamItem::ToolCall { tool_call })) => {
                let name = tool_call.function.name;
                if matches!(
                    name.as_str(),
                    "search_notes"
                        | "read_note"
                        | "read_note_range"
                        | "list_notes"
                        | "read_action"
                        | "list_actions"
                        | "read_conflicts"
                        | "propose_actions"
                        | "propose_knowledge"
                        | "report_conflict"
                ) {
                    emit(AiEvent::ToolStarted {
                        name: name.to_string(),
                    });
                }
            }
            Some(Ok(MultiTurnStreamItem::FinalResponse(_))) => break AiTerminal::Completed,
            Some(Err(error)) => break AiTerminal::Failed(map_stream_error(error)),
            None => break AiTerminal::Failed(AiError::new(AiErrorKind::Other)),
            Some(Ok(_)) => {}
        }
    };
    drop(stream);
    let terminal = if limited.load(Ordering::SeqCst) {
        AiTerminal::Failed(AiError::new(AiErrorKind::ToolLimitReached))
    } else {
        terminal
    };
    if strict_output_limit.is_some() && !matches!(terminal, AiTerminal::Completed) {
        text = String::new();
    }
    AiAnswer { text, terminal }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::agent::PromptResponse;
    use rig::streaming::Transcript;

    fn text_item(text: &str) -> MultiTurnStreamItem {
        let transcript = Transcript::parse(serde_json::json!([
            {"item":"event","value":{"event":"start","part":0,"kind":"text"}},
            {"item":"event","value":{"event":"text","part":0,"text":text}},
            {"item":"event","value":{"event":"end","part":0,
                "content":{"type":"text","text":text}}}
        ]))
        .unwrap();
        let event = transcript
            .events()
            .find(|e| matches!(e, StreamEvent::Text { .. }))
            .unwrap()
            .clone();
        MultiTurnStreamItem::StreamAssistantItem(Item::Event(event))
    }

    fn final_item() -> MultiTurnStreamItem {
        MultiTurnStreamItem::FinalResponse(PromptResponse::new(
            "must not append this output",
            Default::default(),
        ))
    }

    async fn collect(items: Vec<Result<MultiTurnStreamItem, StreamingError>>) -> AiAnswer {
        collect_stream(
            Box::pin(futures::stream::iter(items)),
            CancellationToken::new(),
            Arc::new(|_| {}),
            Arc::new(AtomicBool::new(false)),
            None,
        )
        .await
    }

    #[tokio::test]
    async fn first_error_stops_and_preserves_exact_partial_once() {
        let a = collect(vec![
            Ok(text_item("\u{feff} partial\r\n")),
            Err(StreamingError::Report(rig::error::ErrorReport::new(
                rig::error::ErrorKind::Http,
                "synthetic network error",
            ))),
            Ok(text_item("ignored")),
            Ok(final_item()),
        ])
        .await;
        assert_eq!(a.text, "\u{feff} partial\r\n");
        assert!(
            matches!(
                a.terminal,
                AiTerminal::Failed(AiError {
                    kind: AiErrorKind::Network,
                    ..
                })
            ),
            "{a:?}"
        );
    }

    #[tokio::test]
    async fn eof_without_final_is_failure_and_final_text_is_not_appended() {
        let missing = collect(vec![Ok(text_item("partial"))]).await;
        assert_eq!(missing.text, "partial");
        assert!(matches!(missing.terminal, AiTerminal::Failed(_)));
        let completed = collect(vec![
            Ok(text_item("exact")),
            Ok(final_item()),
            Ok(text_item("ignored")),
        ])
        .await;
        assert_eq!(completed.text, "exact");
        assert!(matches!(completed.terminal, AiTerminal::Completed));
    }

    #[tokio::test]
    async fn completed_response_wins_over_later_stop() {
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let stream = futures::stream::once(async move {
            stop.cancel();
            Ok(final_item())
        });
        let a = collect_stream(
            Box::pin(stream),
            cancel,
            Arc::new(|_| {}),
            Arc::new(AtomicBool::new(false)),
            None,
        )
        .await;
        assert!(matches!(a.terminal, AiTerminal::Completed));
    }

    #[tokio::test]
    async fn limit_flag_wins_over_simultaneous_stop_without_reason_matching() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let limited = Arc::new(AtomicBool::new(true));
        let a = collect_stream(
            Box::pin(futures::stream::pending()),
            cancel,
            Arc::new(|_| {}),
            limited,
            None,
        )
        .await;
        assert!(matches!(
            a.terminal,
            AiTerminal::Failed(AiError {
                kind: AiErrorKind::ToolLimitReached,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn rewrite_exact_output_limit_is_complete_without_emitting_or_appending_final_text() {
        let chunk = "x".repeat(1024 * 1024);
        let stream = futures::stream::iter(0..50)
            .flat_map(move |_| futures::stream::iter([Ok(text_item(&chunk))]))
            .chain(futures::stream::once(async { Ok(final_item()) }));
        let answer = collect_stream(
            Box::pin(stream),
            CancellationToken::new(),
            Arc::new(|_| panic!("Rewrite raw text must not be emitted")),
            Arc::new(AtomicBool::new(false)),
            Some(MAX_REWRITE_BYTES),
        )
        .await;
        assert!(matches!(answer.terminal, AiTerminal::Completed));
        assert_eq!(answer.text.len(), MAX_REWRITE_BYTES);
        assert!(answer.text.bytes().all(|byte| byte == b'x'));
    }

    #[tokio::test]
    async fn rewrite_consumed_completion_wins_later_stop_and_failure_discards_partial() {
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let stream = futures::stream::iter([Ok(text_item("\u{feff}exact 日本語\r\n"))]).chain(
            futures::stream::once(async move {
                stop.cancel();
                Ok(final_item())
            }),
        );
        let answer = collect_stream(
            Box::pin(stream),
            cancel,
            Arc::new(|_| panic!("Rewrite raw text must not be emitted")),
            Arc::new(AtomicBool::new(false)),
            Some(MAX_REWRITE_BYTES),
        )
        .await;
        assert!(matches!(answer.terminal, AiTerminal::Completed));
        assert_eq!(answer.text, "\u{feff}exact 日本語\r\n");
        let failed = collect_stream(
            Box::pin(futures::stream::iter([Ok(text_item("SYNTHETIC_RAW"))])),
            CancellationToken::new(),
            Arc::new(|_| {}),
            Arc::new(AtomicBool::new(false)),
            Some(MAX_REWRITE_BYTES),
        )
        .await;
        assert!(matches!(failed.terminal, AiTerminal::Failed(_)));
        assert!(failed.text.is_empty());
    }
}
