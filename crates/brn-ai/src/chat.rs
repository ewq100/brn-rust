//! Common transport stream contract, shared by the final-use runtime and fixtures.
use crate::error::map_provider;
use crate::{AiError, AiErrorKind};
use futures::StreamExt;
use rig::agent::{MultiTurnStreamItem, StreamingError, StreamingResult};
use rig::completion::PromptError;
use rig::streaming::{Item, StreamEvent};
use std::sync::{
    Arc,
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HistoryPair {
    pub question: String,
    pub answer: String,
}

#[derive(Clone, Debug)]
pub enum AiEvent {
    Text(String),
    ToolStarted {
        name: String,
    },
    /// Completed model responses and admitted tool rounds, never individual tools.
    BudgetProgress {
        model_turns: u16,
        tool_rounds: u16,
        max_tool_rounds: u16,
    },
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
                        | "read_raw_evidence"
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
