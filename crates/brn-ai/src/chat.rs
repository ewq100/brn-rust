//! Common transport stream contract, shared by the final-use runtime and fixtures.
use crate::error::{map_provider, safe_token};
use crate::{AiError, AiErrorKind, AiFailureDetail};
use futures::StreamExt;
use rig::agent::{MultiTurnStreamItem, StreamingResult};
use rig::completion::{FinishReason, PromptError};
use rig::streaming::{Item, StreamEvent};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::sync::CancellationToken;
pub const MAX_ANSWER_BYTES: usize = 128 * 1024;
#[cfg(any(test, feature = "capability-spike"))]
pub const MAX_REWRITE_BYTES: usize = 50 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    pub const fn typed(self) -> rig::completion::Effort {
        match self {
            Self::Low => rig::completion::Effort::Low,
            Self::Medium => rig::completion::Effort::Medium,
            Self::High => rig::completion::Effort::High,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[cfg(any(test, feature = "capability-spike"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HistoryPair {
    pub question: String,
    pub answer: String,
}

#[derive(Clone, Debug)]
pub enum AiEvent {
    Text(String),
    /// Successful final content replaces any provisional stream preview.
    AnswerFinalized(String),
    ToolStarted {
        name: String,
    },
    /// Bounded owner-visible execution metadata, separate from the answer.
    Activity(String),
    /// A provider's explicitly public summary, never raw reasoning deltas.
    ReasoningSummary(String),
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

fn map_stream_error(error: PromptError) -> AiError {
    match error {
        PromptError::Provider(e) => map_provider(e),
        PromptError::Report(report) => {
            map_provider(rig::error::ProviderError::Relayed(Box::new(report)))
        }
        PromptError::UnknownToolCall { .. } => {
            AiError::new(AiErrorKind::InvalidToolUse).with_detail(origin("unknown_tool"))
        }
        PromptError::MaxTurns { .. } => {
            AiError::new(AiErrorKind::ToolLimitReached).with_detail(origin("max_turns"))
        }
        PromptError::Memory(_) => AiError::new(AiErrorKind::Other).with_detail(origin("memory")),
        PromptError::Cancelled { .. } => {
            AiError::new(AiErrorKind::Other).with_detail(origin("cancelled"))
        }
        _ => AiError::new(AiErrorKind::Other).with_detail(origin("prompt")),
    }
}

fn origin(origin: &str) -> AiFailureDetail {
    AiFailureDetail {
        origin: origin.to_owned(),
        ..Default::default()
    }
}

/// A run that ended on a non-stop finish, labelled by the reason only.
fn finish_failure(reason: &FinishReason) -> AiError {
    let code = match reason {
        FinishReason::Length => Some("length".to_owned()),
        FinishReason::ContentFilter => Some("content_filter".to_owned()),
        FinishReason::ToolCalls => Some("tool_calls".to_owned()),
        FinishReason::Other(reason) => safe_token(&reason.replace(": ", ":")),
        _ => None,
    };
    AiError::new(AiErrorKind::Other).with_detail(AiFailureDetail {
        code,
        ..origin("finish")
    })
}

fn with_phase(mut error: AiError, phase: &str) -> AiError {
    let detail = error
        .detail
        .get_or_insert_with(|| Box::new(origin("unknown")));
    detail.phase = Some(phase.to_owned());
    error
}

pub(crate) async fn collect_stream(
    mut stream: StreamingResult,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
    limited: Arc<AtomicBool>,
    strict_output_limit: Option<usize>,
) -> AiAnswer {
    let mut text = String::new();
    let output_limit = strict_output_limit.unwrap_or(MAX_ANSWER_BYTES);
    let mut phase = "none";
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
                phase = "text";
                if delta.len() > output_limit.saturating_sub(text.len()) {
                    break AiTerminal::Failed(AiError::new(AiErrorKind::ToolRejected));
                }
                text.push_str(&delta);
                if strict_output_limit.is_none() {
                    emit(AiEvent::Text(delta));
                }
            }
            Some(Ok(MultiTurnStreamItem::ToolCall { tool_call })) => {
                phase = "tool_call";
                let name = tool_call.function.name;
                emit(AiEvent::ToolStarted {
                    name: name.to_string(),
                });
            }
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(Item::Event(StreamEvent::End {
                content: rig::message::AssistantContent::Reasoning(reasoning),
                ..
            })))) => {
                phase = "reasoning";
                if let Some(summary) = public_reasoning_summary(&reasoning) {
                    emit(AiEvent::ReasoningSummary(summary));
                }
            }
            Some(Ok(MultiTurnStreamItem::FinalResponse(response))) => {
                // Rig may accept useful text from a Length turn. BRN stores only
                // successful natural completion; retained preview remains partial.
                if let Some(reason) = response
                    .completion_calls
                    .last()
                    .and_then(|call| call.finish_reason.as_ref())
                    .filter(|reason| *reason != &FinishReason::Stop)
                {
                    break AiTerminal::Failed(with_phase(finish_failure(reason), phase));
                }
                let final_text = response.output();
                if final_text.len() > output_limit {
                    break AiTerminal::Failed(AiError::new(AiErrorKind::ToolRejected));
                }
                text = final_text;
                if strict_output_limit.is_none() {
                    emit(AiEvent::AnswerFinalized(text.clone()));
                }
                break AiTerminal::Completed;
            }
            Some(Err(error)) => {
                break AiTerminal::Failed(with_phase(map_stream_error(error), phase));
            }
            None => {
                break AiTerminal::Failed(with_phase(
                    AiError::new(AiErrorKind::Other).with_detail(origin("stream_end")),
                    phase,
                ));
            }
            Some(Ok(MultiTurnStreamItem::ToolResult { .. })) => phase = "tool_result",
            Some(Ok(MultiTurnStreamItem::CompletionCall(_))) => phase = "turn_end",
            Some(Ok(_)) => phase = "other",
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

fn public_reasoning_summary(reasoning: &rig::message::Reasoning) -> Option<String> {
    if reasoning.redacted {
        return None;
    }
    let native = reasoning.native.as_ref()?;
    if native.item.get("type")?.as_str()? != "reasoning" {
        return None;
    }
    let mut text = String::new();
    for part in native.item.get("summary")?.as_array()?.iter().take(8) {
        if part.get("type").and_then(serde_json::Value::as_str) != Some("summary_text") {
            continue;
        }
        if let Some(summary) = part.get("text").and_then(serde_json::Value::as_str) {
            if !text.is_empty() {
                text.push('\n');
            }
            for ch in summary.chars() {
                if text.len() + ch.len_utf8() > 8192 {
                    break;
                }
                text.push(ch);
            }
            if text.len() >= 8192 {
                break;
            }
        }
    }
    (!text.trim().is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::agent::PromptResponse;
    use rig::streaming::Transcript;

    #[test]
    fn activity_uses_only_explicit_public_summaries_and_keeps_raw_reasoning_private() {
        use rig::message::{Fingerprint, Native, Reasoning};
        let mut reasoning = Reasoning::new("PRIVATE RAW REASONING");
        assert!(public_reasoning_summary(&reasoning).is_none());
        reasoning.native = Some(Native {
            item: serde_json::json!({"type":"reasoning","summary":[],"content":[{"type":"reasoning_text","text":"PRIVATE RAW REASONING"}],"encrypted_content":"PRIVATE CIPHERTEXT"}),
            fingerprint: Fingerprint::of(&reasoning),
        });
        assert!(public_reasoning_summary(&reasoning).is_none());
        reasoning.native.as_mut().unwrap().item["summary"] =
            serde_json::json!([{"type":"summary_text","text":"Compare the saved route times."}]);
        assert_eq!(
            public_reasoning_summary(&reasoning).as_deref(),
            Some("Compare the saved route times.")
        );
        reasoning.redacted = true;
        assert!(public_reasoning_summary(&reasoning).is_none());
        reasoning.redacted = false;
        reasoning.native.as_mut().unwrap().item["summary"] =
            serde_json::json!([{"type":"summary_text","text":"õ".repeat(10000)}]);
        let bounded = public_reasoning_summary(&reasoning).unwrap();
        assert!(bounded.len() <= 8192);
        assert!(bounded.chars().all(|ch| ch == 'õ'));
    }

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

    fn final_item(text: &str) -> MultiTurnStreamItem {
        MultiTurnStreamItem::FinalResponse(PromptResponse::new(text, Default::default()))
    }

    async fn collect(items: Vec<Result<MultiTurnStreamItem, PromptError>>) -> AiAnswer {
        collect_stream(
            Box::pin(futures::stream::iter(items)),
            CancellationToken::new(),
            Arc::new(|_| {}),
            Arc::new(AtomicBool::new(false)),
            None,
        )
        .await
    }

    fn failure(answer: &AiAnswer) -> &AiError {
        match &answer.terminal {
            AiTerminal::Failed(error) => error,
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn failures_record_their_origin_and_the_last_stream_phase() {
        let ended = collect(vec![Ok(text_item("partial"))]).await;
        let detail = failure(&ended).detail.clone().unwrap();
        assert_eq!(
            (detail.origin.as_str(), detail.phase.as_deref()),
            ("stream_end", Some("text"))
        );
        let failed = collect(vec![Err(PromptError::Provider(
            rig::error::ProviderError::from_provider_body(
                r#"{"type":"response.failed","response":{"error":{"code":"server_error","message":"SYNTHETIC_SECRET"}}}"#,
            ),
        ))])
        .await;
        let error = failure(&failed);
        assert_eq!(error.kind, AiErrorKind::Other);
        let detail = error.detail.clone().unwrap();
        assert_eq!(
            (
                detail.origin.as_str(),
                detail.code.as_deref(),
                detail.phase.as_deref(),
                detail.retryable
            ),
            ("response_failed", Some("server_error"), Some("none"), true)
        );
        assert!(
            !serde_json::to_string(error)
                .unwrap()
                .contains("SYNTHETIC_SECRET")
        );
        assert_eq!(
            error.to_string(),
            AiError::new(AiErrorKind::Other).to_string()
        );
    }

    #[test]
    fn non_stop_finishes_keep_only_the_reason_label() {
        for (reason, code) in [
            (FinishReason::Length, Some("length")),
            (FinishReason::ContentFilter, Some("content_filter")),
            (FinishReason::ToolCalls, Some("tool_calls")),
            (
                FinishReason::Other("incomplete: max_output_tokens".into()),
                Some("incomplete:max_output_tokens"),
            ),
            (
                FinishReason::Other("SYNTHETIC_SECRET free text".into()),
                None,
            ),
        ] {
            let detail = finish_failure(&reason).detail.unwrap();
            assert_eq!(
                (detail.origin.as_str(), detail.code.as_deref()),
                ("finish", code)
            );
        }
    }

    #[tokio::test]
    async fn first_error_stops_and_preserves_exact_partial_once() {
        let a = collect(vec![
            Ok(text_item("\u{feff} partial\r\n")),
            Err(PromptError::Report(rig::error::ErrorReport::new(
                rig::error::ErrorKind::Http,
                "synthetic network error",
            ))),
            Ok(text_item("ignored")),
            Ok(final_item("authoritative")),
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
    async fn eof_fails_and_final_text_replaces_preview() {
        let missing = collect(vec![Ok(text_item("partial"))]).await;
        assert_eq!(missing.text, "partial");
        assert!(matches!(missing.terminal, AiTerminal::Failed(_)));
        let completed = collect(vec![
            Ok(text_item("exact")),
            Ok(final_item("authoritative")),
            Ok(text_item("ignored")),
        ])
        .await;
        assert_eq!(completed.text, "authoritative");
        assert!(matches!(completed.terminal, AiTerminal::Completed));
    }

    #[tokio::test]
    async fn final_answer_reconciles_preview_and_ordinary_stream_agrees() {
        for (preview, finalized) in [("A", "B"), ("ordinary", "ordinary")] {
            let events = Arc::new(std::sync::Mutex::new(Vec::new()));
            let captured = events.clone();
            let answer = collect_stream(
                Box::pin(futures::stream::iter([
                    Ok(text_item(preview)),
                    Ok(final_item(finalized)),
                ])),
                CancellationToken::new(),
                Arc::new(move |e| captured.lock().unwrap().push(e)),
                Arc::new(AtomicBool::new(false)),
                None,
            )
            .await;
            assert!(matches!(answer.terminal, AiTerminal::Completed));
            assert_eq!(answer.text, finalized);
            assert!(
                matches!(events.lock().unwrap().as_slice(), [AiEvent::Text(a), AiEvent::AnswerFinalized(b)] if a == preview && b == finalized)
            );
        }
    }

    #[tokio::test]
    async fn both_provisional_and_authoritative_answers_are_bounded() {
        for items in [
            vec![Ok(text_item(&"x".repeat(MAX_ANSWER_BYTES + 1)))],
            vec![
                Ok(text_item("small preview")),
                Ok(final_item(&"x".repeat(MAX_ANSWER_BYTES + 1))),
            ],
        ] {
            let answer = collect(items).await;
            assert!(matches!(answer.terminal, AiTerminal::Failed(_)));
        }
        let answer = collect(vec![Ok(final_item(&"x".repeat(MAX_ANSWER_BYTES)))]).await;
        assert!(matches!(answer.terminal, AiTerminal::Completed));
        assert_eq!(answer.text.len(), MAX_ANSWER_BYTES);
    }

    #[tokio::test]
    async fn cancellation_keeps_preview_partial_without_final_event() {
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let stream = futures::stream::iter([Ok(text_item("partial"))]).chain(
            futures::stream::once(async move {
                stop.cancel();
                std::future::pending::<Result<MultiTurnStreamItem, PromptError>>().await
            }),
        );
        let answer = collect_stream(
            Box::pin(stream),
            cancel,
            Arc::new(|e| assert!(!matches!(e, AiEvent::AnswerFinalized(_)))),
            Arc::new(AtomicBool::new(false)),
            None,
        )
        .await;
        assert!(matches!(answer.terminal, AiTerminal::Interrupted));
        assert_eq!(answer.text, "partial");
    }

    #[tokio::test]
    async fn completed_response_wins_over_later_stop() {
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let stream = futures::stream::once(async move {
            stop.cancel();
            Ok(final_item("authoritative"))
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
    async fn rewrite_exact_final_output_limit_is_complete_without_emitting() {
        let chunk = "x".repeat(1024 * 1024);
        let stream = futures::stream::iter(0..50)
            .flat_map(move |_| futures::stream::iter([Ok(text_item(&chunk))]))
            .chain(futures::stream::once(async {
                Ok(final_item(&"x".repeat(MAX_REWRITE_BYTES)))
            }));
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
                Ok(final_item("\u{feff}exact 日本語\r\n"))
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
