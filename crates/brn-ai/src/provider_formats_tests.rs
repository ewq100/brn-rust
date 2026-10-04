use crate::*;
use bytes::Bytes;
use rig::http_client::{
    self, HttpClientExt, LazyBody, MultipartForm, Request, Response, StreamingResponse,
};
use rig::test_utils::{CapturedHttpRequest, MockHttpResponse, SequencedHttpClient};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::os::unix::fs::PermissionsExt;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tokio_util::sync::CancellationToken;

mod ask_effort_tests {
    use super::*;

    const ROUTES: [(Provider, &str, bool); 3] = [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Copilot, "gpt-5.5", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ];

    fn message_text(message: &Value) -> Option<String> {
        let content = &message["content"];
        content.as_str().map(str::to_owned).or_else(|| {
            let parts = content.as_array()?;
            let texts = parts
                .iter()
                .map(|part| part["text"].as_str())
                .collect::<Option<Vec<_>>>()?;
            (!texts.is_empty()).then(|| texts.concat())
        })
    }

    fn partial_sse(responses: bool, text: &str) -> String {
        if responses {
            event(json!({"type":"response.output_text.delta","delta":text,
                "item_id":"msg_synthetic","output_index":0,"content_index":0,"sequence_number":1}))
        } else {
            event(json!({"id":"synthetic","object":"chat.completion.chunk",
                "created":1,"model":"synthetic",
                "choices":[{"index":0,"delta":{"role":"assistant","content":text},"finish_reason":null}]}))
        }
    }

    #[tokio::test]
    async fn explicit_ask_effort_preserves_model_last_twenty_pairs_and_tool_continuation() {
        let question = "\u{feff}Current 日本語 question\r\nλ";
        let output = "\u{feff}Provisional 日本語 answer\r\nλ";
        let history = (0..23)
            .map(|i| HistoryPair {
                question: format!("\u{feff}question-{i} 日本語\r\nλ"),
                answer: format!("answer-{i} λ\r\n"),
            })
            .collect::<Vec<_>>();
        let expected = history[3..]
            .iter()
            .flat_map(|pair| {
                [
                    ("user".to_owned(), pair.question.clone()),
                    ("assistant".to_owned(), pair.answer.clone()),
                ]
            })
            .chain([("user".to_owned(), question.to_owned())])
            .collect::<Vec<_>>();
        for (provider, model, responses) in ROUTES {
            for effort in [
                ReasoningEffort::Low,
                ReasoningEffort::Medium,
                ReasoningEffort::High,
            ] {
                let (_root, client, http) = client(
                    provider,
                    model,
                    vec![
                        success(tool_sse(
                            responses,
                            &[("read_note", json!({"path":"a.md"}))],
                        )),
                        success(text_sse(responses, output)),
                    ],
                )
                .await;
                let notes = Arc::new(Notes::default());
                let events = Arc::new(Mutex::new(vec![]));
                let sink = events.clone();
                let answer = answer_with_effort(
                    client,
                    question,
                    &history,
                    effort,
                    notes.clone(),
                    CancellationToken::new(),
                    Arc::new(move |event| sink.lock().unwrap().push(event)),
                )
                .await;
                assert!(
                    matches!(answer.terminal, AiTerminal::Completed),
                    "{answer:?}"
                );
                assert_eq!(answer.text, output);
                assert_eq!(notes.calls.load(Ordering::SeqCst), 1);
                assert!(matches!(events.lock().unwrap().as_slice(),
                    [AiEvent::ToolStarted { name }, AiEvent::Text(text)]
                    if name == "read_note" && text == output));
                let bodies = http.bodies();
                assert_eq!(bodies.len(), 2);
                for body in &bodies {
                    assert_eq!(body["model"], model);
                    if responses {
                        assert_eq!(body["reasoning"]["effort"], effort.as_str());
                        assert!(body.get("reasoning_effort").is_none());
                    } else {
                        assert_eq!(body["reasoning_effort"], effort.as_str());
                        assert!(body.get("reasoning").is_none());
                    }
                    let messages = body[if responses { "input" } else { "messages" }]
                        .as_array()
                        .unwrap();
                    let text_messages = messages
                        .iter()
                        .filter(|message| {
                            matches!(message["role"].as_str(), Some("user" | "assistant"))
                        })
                        .filter_map(|message| {
                            message_text(message)
                                .map(|text| (message["role"].as_str().unwrap().to_owned(), text))
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(text_messages, expected);
                }
                let continuation = bodies[1][if responses { "input" } else { "messages" }]
                    .as_array()
                    .unwrap();
                let results = continuation
                    .iter()
                    .filter(|message| {
                        message["role"] == "tool" || message["type"] == "function_call_output"
                    })
                    .collect::<Vec<_>>();
                assert_eq!(results.len(), 1);
                let encoded = if responses {
                    results[0]["output"].as_str().unwrap().to_owned()
                } else {
                    message_text(results[0]).unwrap()
                };
                assert_eq!(
                    serde_json::from_str::<Value>(&encoded).unwrap(),
                    json!({"path":"a.md","text":"fresh note","truncated":false})
                );
                http.assert_consumed();
            }
        }
    }

    #[tokio::test]
    async fn explicit_ask_parser_failure_retains_exact_provisional_text_without_retry() {
        let partial = "\u{feff}Partial 日本語\r\nλ";
        for (provider, model, responses) in ROUTES {
            let malformed = if responses {
                "data: {\"type\":\"response.output_text.delta\",\"delta\":42}\n\n"
            } else {
                "data: {\"choices\":42}\n\n"
            };
            let sse = partial_sse(responses, partial)
                + malformed
                + &text_sse(responses, "must not appear");
            let (_root, client, http) = client(provider, model, vec![success(sse)]).await;
            let events = Arc::new(Mutex::new(vec![]));
            let sink = events.clone();
            let notes = Arc::new(Notes::default());
            let answer = answer_with_effort(
                client,
                "q",
                &[],
                ReasoningEffort::Medium,
                notes.clone(),
                CancellationToken::new(),
                Arc::new(move |event| sink.lock().unwrap().push(event)),
            )
            .await;
            assert!(
                matches!(
                    answer.terminal,
                    AiTerminal::Failed(AiError {
                        kind: AiErrorKind::Other,
                        ..
                    })
                ),
                "{answer:?}"
            );
            assert_eq!(answer.text, partial);
            assert!(matches!(events.lock().unwrap().as_slice(),
                [AiEvent::Text(text)] if text == partial));
            assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
            assert_eq!(http.bodies().len(), 1);
            http.assert_consumed();
        }
    }

    #[tokio::test]
    async fn explicit_ask_stop_retains_provisional_text_and_skips_final_completion() {
        let partial = "\u{feff}Stop 日本語\r\nλ";
        for (provider, model, responses) in ROUTES {
            let (_root, client, http) =
                client(provider, model, vec![success(text_sse(responses, partial))]).await;
            let cancel = CancellationToken::new();
            let stop = cancel.clone();
            let events = Arc::new(Mutex::new(vec![]));
            let sink = events.clone();
            let answer = answer_with_effort(
                client,
                "q",
                &[],
                ReasoningEffort::High,
                Arc::new(Notes::default()),
                cancel,
                Arc::new(move |event| {
                    if matches!(event, AiEvent::Text(_)) {
                        stop.cancel();
                    }
                    sink.lock().unwrap().push(event);
                }),
            )
            .await;
            assert!(
                matches!(answer.terminal, AiTerminal::Interrupted),
                "{answer:?}"
            );
            assert_eq!(answer.text, partial);
            assert!(matches!(events.lock().unwrap().as_slice(),
                [AiEvent::Text(text)] if text == partial));
            assert_eq!(http.bodies().len(), 1);
            http.assert_consumed();
        }
    }
}

#[cfg(test)]
mod rewrite_tests {
    use super::*;
    use crate::auth::OwnedClient;
    use futures::StreamExt;

    const ROUTES: [(Provider, &str, bool); 3] = [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Copilot, "gpt-5.5", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ];

    async fn run(
        client: ProviderClient,
        prompt: &str,
        effort: ReasoningEffort,
        notes: Arc<dyn ReadTools>,
        cancel: CancellationToken,
    ) -> (AiAnswer, Vec<AiEvent>) {
        let events = Arc::new(Mutex::new(vec![]));
        let sink = events.clone();
        let answer = rewrite(
            client,
            prompt,
            effort,
            notes,
            cancel,
            Arc::new(move |event| {
                sink.lock().unwrap().push(event);
            }),
        )
        .await;
        let events = events.lock().unwrap().clone();
        (answer, events)
    }

    #[tokio::test]
    async fn rewrite_effort_model_and_read_only_continuation_use_exact_wire_routes() {
        let prompt = "Captured review: \u{feff}日本語\r\nλ";
        let output = json!({"title":"日本語","texts":["\u{feff}Full λ\r\n",null]}).to_string();
        for (provider, model, responses) in ROUTES {
            for effort in [
                ReasoningEffort::Low,
                ReasoningEffort::Medium,
                ReasoningEffort::High,
            ] {
                let (_root, client, http) = super::client(
                    provider,
                    model,
                    vec![
                        success(tool_sse(
                            responses,
                            &[("read_note", json!({"path":"a.md"}))],
                        )),
                        success(text_sse(responses, &output)),
                    ],
                )
                .await;
                let notes = Arc::new(Notes::default());
                let (answer, events) = run(
                    client,
                    prompt,
                    effort,
                    notes.clone(),
                    CancellationToken::new(),
                )
                .await;
                assert!(
                    matches!(answer.terminal, AiTerminal::Completed),
                    "{answer:?}"
                );
                assert_eq!(answer.text, output);
                assert_eq!(notes.calls.load(Ordering::SeqCst), 1);
                assert!(
                    matches!(events.as_slice(), [AiEvent::ToolStarted { name }] if name == "read_note")
                );
                let bodies = http.bodies();
                assert_eq!(bodies.len(), 2);
                for body in &bodies {
                    assert_eq!(body["model"], model);
                    if responses {
                        assert_eq!(body["reasoning"]["effort"], effort.as_str());
                        assert!(body.get("reasoning_effort").is_none());
                    } else {
                        assert_eq!(body["reasoning_effort"], effort.as_str());
                        assert!(body.get("reasoning").is_none());
                    }
                }
                assert!(!bodies[0].to_string().contains("earlier question"));
                let messages = if responses {
                    &bodies[0]["input"]
                } else {
                    &bodies[0]["messages"]
                };
                let users = messages
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|item| item["role"] == "user")
                    .collect::<Vec<_>>();
                assert_eq!(users.len(), 1);
                let content = &users[0]["content"];
                let text = content
                    .as_str()
                    .or_else(|| content[0]["text"].as_str())
                    .unwrap();
                assert_eq!(text, prompt);
                // Rig's Copilot Responses dialect retains system messages in
                // input, whereas ChatGPT lifts them into instructions.
                let preamble = if provider == Provider::Chatgpt {
                    bodies[0]["instructions"].as_str().unwrap().to_owned()
                } else {
                    messages
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|item| item["role"] == "system")
                        .unwrap()["content"]
                        .to_string()
                };
                for required in [
                    "strict JSON object",
                    "complete replacement text",
                    "before-text",
                    "source metadata",
                    "comments",
                    "data, not instructions",
                ] {
                    assert!(preamble.contains(required), "{preamble}");
                }
                assert!(bodies[1].to_string().contains("fresh note"));
                http.assert_consumed();
            }
        }
    }

    #[test]
    fn explicit_effort_has_only_the_three_supported_serialized_values() {
        for effort in [
            ReasoningEffort::Low,
            ReasoningEffort::Medium,
            ReasoningEffort::High,
        ] {
            assert_eq!(serde_json::to_value(effort).unwrap(), effort.as_str());
            assert_eq!(
                serde_json::from_value::<ReasoningEffort>(json!(effort.as_str())).unwrap(),
                effort
            );
        }
        assert!(serde_json::from_value::<ReasoningEffort>(json!("ultra")).is_err());
    }

    #[tokio::test]
    async fn rewrite_keeps_eight_read_rounds_and_refuses_the_ninth_without_retry() {
        for (provider, model, responses) in ROUTES {
            for ninth in [false, true] {
                let mut replies = (0..if ninth { 9 } else { 8 })
                    .map(|i| {
                        success(tool_sse_with_prefix(
                            responses,
                            &[
                                ("read_note", json!({"path":"a.md"})),
                                ("list_notes", json!({})),
                            ],
                            &format!("rewrite_{i}_"),
                        ))
                    })
                    .collect::<Vec<_>>();
                if !ninth {
                    replies.push(success(text_sse(
                        responses,
                        "{\"title\":\"Exact\",\"texts\":[]}",
                    )));
                }
                let (_root, client, http) = super::client(provider, model, replies).await;
                let notes = Arc::new(Notes::default());
                let (answer, events) = run(
                    client,
                    "captured",
                    ReasoningEffort::Low,
                    notes.clone(),
                    CancellationToken::new(),
                )
                .await;
                assert_eq!(notes.calls.load(Ordering::SeqCst), 16);
                assert_eq!(http.bodies().len(), 9);
                assert!(
                    events
                        .iter()
                        .all(|event| matches!(event, AiEvent::ToolStarted { .. }))
                );
                if ninth {
                    assert!(
                        matches!(
                            answer.terminal,
                            AiTerminal::Failed(AiError {
                                kind: AiErrorKind::ToolLimitReached,
                                ..
                            })
                        ),
                        "{answer:?}"
                    );
                    assert!(answer.text.is_empty());
                } else {
                    assert!(
                        matches!(answer.terminal, AiTerminal::Completed),
                        "{answer:?}"
                    );
                }
                http.assert_consumed();
            }
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![success(tool_sse(
                    responses,
                    &[("write_note", json!({"path":"a.md"}))],
                ))],
            )
            .await;
            let notes = Arc::new(Notes::default());
            let (answer, events) = run(
                client,
                "captured",
                ReasoningEffort::High,
                notes.clone(),
                CancellationToken::new(),
            )
            .await;
            assert!(
                matches!(
                    answer.terminal,
                    AiTerminal::Failed(AiError {
                        kind: AiErrorKind::InvalidToolUse,
                        ..
                    })
                ),
                "{answer:?}"
            );
            assert!(answer.text.is_empty() && events.is_empty());
            assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
            assert_eq!(http.bodies().len(), 1);
            http.assert_consumed();
        }
    }

    fn partial_sse(responses: bool, text: &str) -> String {
        text_sse(responses, text)
            .split("\n\n")
            .next()
            .unwrap()
            .to_owned()
            + "\n\n"
    }

    #[tokio::test]
    async fn rewrite_failure_refusal_and_malformed_streams_discard_raw_partials_without_retry() {
        for (provider, model, responses) in ROUTES {
            let malformed = if responses {
                "data: {\"type\":\"response.output_text.delta\",\"delta\":42}\n\n"
            } else {
                "data: {\"choices\":42}\n\n"
            };
            for (reply, kind) in [
                (
                    success(
                        partial_sse(responses, "SYNTHETIC_RAW\r\nλ")
                            + malformed
                            + &text_sse(responses, "ignored"),
                    ),
                    AiErrorKind::Other,
                ),
                (
                    success(partial_sse(responses, "SYNTHETIC_RAW")),
                    AiErrorKind::Other,
                ),
                (
                    MockHttpResponse::error(
                        http_client::StatusCode::BAD_REQUEST,
                        r#"{"error":{"code":"unsupported_api_for_model","message":"SYNTHETIC_RAW"}}"#,
                    ),
                    AiErrorKind::ModelRefused,
                ),
                (
                    MockHttpResponse::error(
                        http_client::StatusCode::TOO_MANY_REQUESTS,
                        r#"{"error":{"message":"SYNTHETIC_RAW"}}"#,
                    ),
                    AiErrorKind::RateLimited,
                ),
            ] {
                let (_root, client, http) = super::client(provider, model, vec![reply]).await;
                let (answer, events) = run(
                    client,
                    "captured",
                    ReasoningEffort::High,
                    Arc::new(Notes::default()),
                    CancellationToken::new(),
                )
                .await;
                assert!(
                    matches!(answer.terminal,AiTerminal::Failed(AiError{kind:actual,..}) if actual==kind),
                    "{answer:?}"
                );
                assert!(answer.text.is_empty() && events.is_empty());
                assert!(!format!("{answer:?}").contains("SYNTHETIC_RAW"));
                assert_eq!(http.bodies().len(), 1);
                http.assert_consumed();
            }
            let (_root, client, http) =
                super::client_replies(provider, model, vec![Err(http_client::Error::StreamEnded)])
                    .await;
            let (answer, events) = run(
                client,
                "captured",
                ReasoningEffort::Low,
                Arc::new(Notes::default()),
                CancellationToken::new(),
            )
            .await;
            assert!(
                matches!(
                    answer.terminal,
                    AiTerminal::Failed(AiError {
                        kind: AiErrorKind::Network,
                        ..
                    })
                ),
                "{answer:?}"
            );
            assert!(answer.text.is_empty() && events.is_empty());
            assert_eq!(http.bodies().len(), 1);
            http.assert_consumed();
        }
    }

    #[derive(Clone)]
    struct ChunkedHttp {
        http: ScriptHttp,
        chunks: Vec<Bytes>,
        cancel: Option<CancellationToken>,
    }
    impl HttpClientExt for ChunkedHttp {
        fn send<T, U>(
            &self,
            request: Request<T>,
        ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + Send + 'static
        where
            T: Into<Bytes> + Send,
            U: From<Bytes> + Send + 'static,
        {
            self.http.send(request)
        }
        fn send_multipart<U>(
            &self,
            request: Request<MultipartForm>,
        ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + Send + 'static
        where
            U: From<Bytes> + Send + 'static,
        {
            self.http.send_multipart(request)
        }
        fn send_streaming<T>(
            &self,
            request: Request<T>,
        ) -> impl Future<Output = http_client::Result<StreamingResponse>> + Send
        where
            T: Into<Bytes> + Send,
        {
            let response = self.http.send_streaming(request);
            let chunks = self.chunks.clone();
            let cancel = self.cancel.clone();
            async move {
                let (parts, _) = response.await?.into_parts();
                let chunks = futures::stream::iter(chunks.into_iter().map(Ok));
                let tail = match cancel {
                    Some(cancel) => {
                        futures::future::Either::Left(futures::stream::once(async move {
                            cancel.cancel();
                            futures::future::pending::<http_client::Result<Bytes>>().await
                        }))
                    }
                    None => futures::future::Either::Right(futures::stream::empty()),
                };
                Ok(Response::from_parts(
                    parts,
                    Box::pin(chunks.chain(tail)) as http_client::BoxedStream,
                ))
            }
        }
    }
    fn with_chunks(
        mut client: ProviderClient,
        http: ScriptHttp,
        chunks: Vec<Bytes>,
        cancel: Option<CancellationToken>,
    ) -> ProviderClient {
        let transport = ChunkedHttp {
            http,
            chunks,
            cancel,
        };
        client.inner = match client.inner {
            OwnedClient::Chatgpt(inner) => {
                OwnedClient::Chatgpt(Box::new((*inner).with_http(transport)))
            }
            OwnedClient::Copilot(inner) => OwnedClient::Copilot(inner.with_http(transport)),
        };
        client
    }

    #[tokio::test]
    async fn rewrite_stop_discards_partial_and_precancelled_sends_no_completion() {
        for (provider, model, responses) in ROUTES {
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![success(text_sse(responses, "unused"))],
            )
            .await;
            let cancel = CancellationToken::new();
            let client = with_chunks(
                client,
                http.clone(),
                vec![Bytes::from(partial_sse(responses, "SYNTHETIC_RAW\r\nλ"))],
                Some(cancel.clone()),
            );
            let (answer, events) = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                run(
                    client,
                    "captured",
                    ReasoningEffort::Medium,
                    Arc::new(Notes::default()),
                    cancel,
                ),
            )
            .await
            .unwrap();
            assert!(
                matches!(answer.terminal, AiTerminal::Interrupted),
                "{answer:?}"
            );
            assert!(answer.text.is_empty() && events.is_empty());
            assert_eq!(http.bodies().len(), 1);
            http.assert_consumed();

            let (_root, client, http) = super::client(provider, model, vec![]).await;
            let cancel = CancellationToken::new();
            cancel.cancel();
            let (answer, events) = run(
                client,
                "captured",
                ReasoningEffort::Low,
                Arc::new(Notes::default()),
                cancel,
            )
            .await;
            assert!(matches!(answer.terminal, AiTerminal::Interrupted));
            assert!(answer.text.is_empty() && events.is_empty() && http.bodies().is_empty());
            http.assert_consumed();
        }
    }

    #[tokio::test]
    async fn rewrite_input_and_actual_streamed_output_have_hard_byte_bounds() {
        let oversized = "λ".repeat(MAX_REWRITE_BYTES / 2 + 1);
        for (provider, model, _) in ROUTES {
            let (_root, client, http) = super::client(provider, model, vec![]).await;
            let (answer, events) = run(
                client,
                &oversized,
                ReasoningEffort::Low,
                Arc::new(Notes::default()),
                CancellationToken::new(),
            )
            .await;
            assert!(matches!(
                answer.terminal,
                AiTerminal::Failed(AiError {
                    kind: AiErrorKind::ToolRejected,
                    ..
                })
            ));
            assert!(answer.text.is_empty() && events.is_empty() && http.bodies().is_empty());
            http.assert_consumed();
        }
        drop(oversized);
        let exact = "x".repeat(MAX_REWRITE_BYTES);
        let (_root, client, http) = super::client(
            Provider::Chatgpt,
            "gpt-5.5",
            vec![success(text_sse(
                true,
                "{\"title\":\"Exact\",\"texts\":[]}",
            ))],
        )
        .await;
        let (answer, events) = run(
            client,
            &exact,
            ReasoningEffort::Low,
            Arc::new(Notes::default()),
            CancellationToken::new(),
        )
        .await;
        assert!(
            matches!(answer.terminal, AiTerminal::Completed),
            "{answer:?}"
        );
        assert!(events.is_empty());
        assert_eq!(http.requests.lock().unwrap().len(), 1);
        http.assert_consumed();
        drop(exact);
        drop(http);
        let chunk = Bytes::from(partial_sse(false, &"x".repeat(1024 * 1024)));
        let mut chunks = vec![chunk; 50];
        chunks.push(Bytes::from(partial_sse(false, "λ")));
        let (_root, client, http) = super::client(
            Provider::Copilot,
            "gpt-5.5",
            vec![success(text_sse(false, "unused"))],
        )
        .await;
        let client = with_chunks(client, http.clone(), chunks, None);
        let (answer, events) = run(
            client,
            "captured",
            ReasoningEffort::High,
            Arc::new(Notes::default()),
            CancellationToken::new(),
        )
        .await;
        assert!(
            matches!(
                answer.terminal,
                AiTerminal::Failed(AiError {
                    kind: AiErrorKind::ToolRejected,
                    ..
                })
            ),
            "{answer:?}"
        );
        assert!(answer.text.is_empty() && events.is_empty());
        assert_eq!(http.bodies().len(), 1);
        http.assert_consumed();
    }
}

// Each entry is one HTTP request/response, never one chunk of a shared stream.
type ScriptedStreamReply = (String, http_client::Result<MockHttpResponse>);

#[derive(Clone)]
struct ScriptHttp {
    unary: SequencedHttpClient,
    unary_urls: Arc<Mutex<VecDeque<String>>>,
    streams: Arc<Mutex<VecDeque<ScriptedStreamReply>>>,
    requests: Arc<Mutex<Vec<CapturedHttpRequest>>>,
    unexpected: Arc<AtomicUsize>,
}

impl ScriptHttp {
    fn new(unary: Vec<(&str, MockHttpResponse)>, streams: Vec<ScriptedStreamReply>) -> Self {
        let (urls, replies): (Vec<_>, Vec<_>) = unary.into_iter().unzip();
        Self {
            unary: SequencedHttpClient::new(replies),
            unary_urls: Arc::new(Mutex::new(urls.into_iter().map(str::to_owned).collect())),
            streams: Arc::new(Mutex::new(streams.into())),
            requests: Arc::default(),
            unexpected: Arc::default(),
        }
    }
    fn reject(&self) -> http_client::Error {
        self.unexpected.fetch_add(1, Ordering::SeqCst);
        http_client::Error::StreamEnded
    }
    fn assert_consumed(&self) {
        assert_eq!(self.unexpected.load(Ordering::SeqCst), 0);
        assert!(self.unary_urls.lock().unwrap().is_empty());
        assert_eq!(self.unary.remaining_responses(), 0);
        assert!(self.streams.lock().unwrap().is_empty());
    }
    fn bodies(&self) -> Vec<Value> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| serde_json::from_slice(&r.body).unwrap())
            .collect()
    }
}

impl HttpClientExt for ScriptHttp {
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + Send + 'static
    where
        T: Into<Bytes> + Send,
        U: From<Bytes> + Send + 'static,
    {
        let expected = self.unary_urls.lock().unwrap().pop_front();
        let method = if expected
            .as_deref()
            .is_some_and(|url| url.starts_with("https://api.github.com/"))
        {
            "GET"
        } else {
            "POST"
        };
        if expected.as_deref() != Some(req.uri().to_string().as_str())
            || req.method().as_str() != method
        {
            let err = self.reject();
            return futures::future::Either::Left(async move { Err(err) });
        }
        futures::future::Either::Right(self.unary.send(req))
    }
    fn send_multipart<U>(
        &self,
        _: Request<MultipartForm>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + Send + 'static
    where
        U: From<Bytes> + Send + 'static,
    {
        let err = self.reject();
        async move { Err(err) }
    }
    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<StreamingResponse>> + Send
    where
        T: Into<Bytes> + Send,
    {
        let (parts, body) = req.into_parts();
        let body: Bytes = body.into();
        self.requests.lock().unwrap().push(CapturedHttpRequest {
            uri: parts.uri.to_string(),
            headers: parts.headers.clone(),
            body: body.clone(),
        });
        let reply = self.streams.lock().unwrap().pop_front();
        let valid = reply
            .as_ref()
            .is_some_and(|(url, _)| url == &parts.uri.to_string() && parts.method == "POST");
        let err = (!valid).then(|| self.reject());
        async move {
            if let Some(err) = err {
                return Err(err);
            }
            let (_, reply) = reply.unwrap();
            // Rig handles status and SSE framing; a fresh client owns this one reply.
            SequencedHttpClient::new([reply?])
                .send_streaming(Request::from_parts(parts, body))
                .await
        }
    }
}

#[derive(Default)]
struct Notes {
    calls: AtomicUsize,
}
impl ReadTools for Notes {
    fn search_notes(&self, _: &str, _: usize) -> AiResult<ToolSearch> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ToolSearch {
            hits: vec![Passage {
                path: "a.md".into(),
                start_byte: 0,
                end_byte: 5,
                quote: "fresh".into(),
            }],
            keyword_only: true,
        })
    }
    fn read_note(&self, path: &str) -> AiResult<ToolNote> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ToolNote {
            path: path.into(),
            text: "fresh note".into(),
            truncated: false,
        })
    }
    fn list_notes(&self, _: Option<&str>, _: Option<&str>) -> AiResult<NotePage> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(NotePage {
            notes: vec![],
            next_cursor: None,
        })
    }
}

fn event(value: Value) -> String {
    format!("data: {value}\n\n")
}

fn response_end(output: Vec<Value>) -> String {
    event(
        json!({"type":"response.completed","sequence_number":100,"response":{
            "id":"resp_synthetic", "object":"response", "created_at":1,
            "status":"completed", "error":null, "incomplete_details":null,
            "instructions":null, "max_output_tokens":null, "model":"synthetic",
            "usage":{"input_tokens":1,"input_tokens_details":{"cached_tokens":0},
                "output_tokens":1,"output_tokens_details":{"reasoning_tokens":0},"total_tokens":2},
            "output":output, "tools":[]
        }}),
    ) + "data: [DONE]\n\n"
}

fn text_sse(responses: bool, text: &str) -> String {
    if responses {
        event(json!({"type":"response.output_text.delta","delta":text,
            "item_id":"msg_synthetic","output_index":0,"content_index":0,"sequence_number":1}))
            + &response_end(vec![json!({
                "type":"message","id":"msg_synthetic","status":"completed",
                "role":"assistant", "content":[{"type":"output_text","annotations":[],"text":text}]
            })])
    } else {
        event(json!({"id":"synthetic","object":"chat.completion.chunk",
            "created":1,"model":"synthetic",
            "choices":[{"index":0,"delta":{"role":"assistant","content":text},"finish_reason":null}]
        })) + &event(json!({"id":"synthetic","object":"chat.completion.chunk",
            "created":1,"model":"synthetic",
            "choices":[{"index":0,"delta":{},"finish_reason":"stop"}]
        })) + "data: [DONE]\n\n"
    }
}

fn tool_sse(responses: bool, calls: &[(&str, Value)]) -> String {
    tool_sse_with_prefix(responses, calls, "")
}

fn tool_sse_with_prefix(responses: bool, calls: &[(&str, Value)], prefix: &str) -> String {
    if responses {
        let mut s = String::new();
        let mut output = vec![];
        for (i, (name, args)) in calls.iter().enumerate() {
            let item = json!({"type":"function_call","id":format!("{prefix}fc_{i}"),
                "arguments":args.to_string(),"call_id":format!("{prefix}call_{i}"),
                "name":name,"status":"completed"});
            s += &event(json!({"type":"response.output_item.added","output_index":i,
            "sequence_number":i*3+1,"item":{
                "type":"function_call","id":format!("{prefix}fc_{i}"),"arguments":"",
                "call_id":format!("{prefix}call_{i}"),"name":name,"status":"in_progress"
            }}));
            s += &event(json!({"type":"response.function_call_arguments.delta",
                "output_index":i,"item_id":format!("{prefix}fc_{i}"),
                "sequence_number":i*3+2,"delta":args.to_string()}));
            s += &event(json!({"type":"response.output_item.done",
                "output_index":i,"sequence_number":i*3+3,"item":item}));
            output.push(item);
        }
        s + &response_end(output)
    } else {
        event(json!({"id":"synthetic","object":"chat.completion.chunk",
            "created":1,"model":"synthetic","choices":[{"index":0,"delta":{
                "role":"assistant","tool_calls":calls.iter().enumerate().map(|(i,(name,args))|
                    json!({"index":i,"id":format!("{prefix}call_{i}"),"type":"function",
                        "function":{"name":name,"arguments":args.to_string()}})
                ).collect::<Vec<_>>()
            },"finish_reason":null}]
        })) + &event(json!({"id":"synthetic","object":"chat.completion.chunk",
            "created":1,"model":"synthetic",
            "choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]
        })) + "data: [DONE]\n\n"
    }
}

async fn client(
    provider: Provider,
    model: &str,
    streams: Vec<MockHttpResponse>,
) -> (tempfile::TempDir, ProviderClient, ScriptHttp) {
    client_replies(provider, model, streams.into_iter().map(Ok).collect()).await
}

async fn client_replies(
    provider: Provider,
    model: &str,
    streams: Vec<http_client::Result<MockHttpResponse>>,
) -> (tempfile::TempDir, ProviderClient, ScriptHttp) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let dir = root.path().join("credentials");
    let url = match provider {
        Provider::Chatgpt => "https://chatgpt.com/backend-api/codex/responses",
        Provider::Copilot if model.contains("codex") => "https://example.invalid/responses",
        Provider::Copilot => "https://example.invalid/chat/completions",
    };
    let unary = if provider == Provider::Copilot {
        vec![
            (
                "https://github.com/login/device/code",
                MockHttpResponse::success(
                    r#"{"device_code":"SYNTHETIC_DEVICE","user_code":"SYNTHETIC_CODE","verification_uri":"https://example.invalid/device","interval":1,"expires_in":900}"#,
                ),
            ),
            (
                "https://github.com/login/oauth/access_token",
                MockHttpResponse::success(r#"{"access_token":"SYNTHETIC_GITHUB"}"#),
            ),
            (
                "https://api.github.com/copilot_internal/v2/token",
                MockHttpResponse::success(
                    r#"{"token":"SYNTHETIC_SESSION","expires_at":4102444800,"endpoints":{"api":"https://example.invalid"}}"#,
                ),
            ),
            (
                "https://api.github.com/user",
                MockHttpResponse::success(r#"{"login":"synthetic"}"#),
            ),
        ]
    } else {
        vec![(
            "https://auth.openai.com/oauth/token",
            MockHttpResponse::success(
                r#"{"access_token":"SYNTHETIC_ACCESS","refresh_token":"SYNTHETIC_REFRESH","expires_in":3600}"#,
            ),
        )]
    };
    let http = ScriptHttp::new(
        unary,
        streams.into_iter().map(|s| (url.into(), s)).collect(),
    );
    let auth = Auth::open(&dir).unwrap().with_http(http.clone());
    if provider == Provider::Chatgpt {
        let path = dir.join("chatgpt.json");
        std::fs::write(&path, r#"{"access_token":"SYNTHETIC_OLD","refresh_token":"SYNTHETIC_REFRESH","expires_at":1,"account_id":"synthetic-account"}"#).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    } else {
        auth.connect(provider, Arc::new(|_| {}), CancellationToken::new())
            .await
            .unwrap();
    }
    let client = auth
        .client(
            &Selection {
                provider,
                model: model.into(),
            },
            CancellationToken::new(),
        )
        .await
        .unwrap();
    (root, client, http)
}

fn success(sse: String) -> MockHttpResponse {
    let mut headers = http_client::HeaderMap::new();
    headers.insert("content-type", "text/event-stream".parse().unwrap());
    MockHttpResponse::SuccessWithHeaders(sse.into(), headers)
}

async fn run(
    client: ProviderClient,
    notes: Arc<dyn ReadTools>,
    cancel: CancellationToken,
) -> (AiAnswer, Vec<AiEvent>) {
    let events = Arc::new(Mutex::new(vec![]));
    let sink = events.clone();
    let answer = answer(
        client,
        "current question",
        &[HistoryPair {
            question: "earlier question".into(),
            answer: "earlier answer".into(),
        }],
        notes,
        cancel,
        Arc::new(move |e| sink.lock().unwrap().push(e)),
    )
    .await;
    let events = events.lock().unwrap().clone();
    (answer, events)
}

#[tokio::test]
async fn actual_subscription_formats_continue_tools_and_text_only_history() {
    for (provider, model, responses) in [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Copilot, "gpt-4o", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ] {
        let (_root, client, http) = client(
            provider,
            model,
            vec![
                success(tool_sse(
                    responses,
                    &[
                        ("search_notes", json!({"query":"fresh","limit":1})),
                        ("read_note", json!({"path":"a.md"})),
                    ],
                )),
                success(text_sse(responses, "exact final")),
            ],
        )
        .await;
        let notes = Arc::new(Notes::default());
        let (a, events) = run(client, notes.clone(), CancellationToken::new()).await;
        assert!(
            matches!(a.terminal, AiTerminal::Completed),
            "{provider:?} {model}: {a:?}"
        );
        assert_eq!(a.text, "exact final");
        assert_eq!(notes.calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, AiEvent::ToolStarted { .. }))
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter_map(|e| match e {
                    AiEvent::Text(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect::<String>(),
            a.text
        );
        http.assert_consumed();
        if provider == Provider::Chatgpt {
            assert!(
                String::from_utf8_lossy(&http.unary.requests()[0].body)
                    .contains("grant_type=refresh_token")
            );
        } else {
            assert_eq!(http.unary.requests().len(), 4);
        }
        let bodies = http.bodies();
        for body in &bodies {
            assert_eq!(body["model"], model);
            assert_eq!(body["stream"], true);
            let mut names = body["tools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| {
                    if responses {
                        t["name"].as_str().unwrap()
                    } else {
                        t["function"]["name"].as_str().unwrap()
                    }
                })
                .collect::<Vec<_>>();
            names.sort();
            assert_eq!(names, ["list_notes", "read_note", "search_notes"]);
            for tool in body["tools"].as_array().unwrap() {
                let definition = if responses { tool } else { &tool["function"] };
                assert_eq!(definition["parameters"]["additionalProperties"], false);
                if definition["name"] == "search_notes" {
                    assert_eq!(
                        definition["parameters"]["properties"]["limit"]["type"],
                        "integer"
                    );
                    assert_eq!(
                        definition["parameters"]["properties"]["limit"]["maximum"],
                        10
                    );
                }
            }
        }
        let history = if responses {
            &bodies[0]["input"]
        } else {
            &bodies[0]["messages"]
        };
        let serialized = history.to_string();
        assert!(serialized.contains("earlier question") && serialized.contains("earlier answer"));
        assert!(
            serialized.contains("\"role\":\"user\"")
                && serialized.contains("\"role\":\"assistant\"")
        );
        assert!(!serialized.contains("function_call") && !serialized.contains("\"role\":\"tool\""));
        let continuation = bodies[1].to_string();
        assert!(continuation.contains("fresh note") && continuation.contains("keyword_only"));
        assert!(continuation.contains("call_0") && continuation.contains("call_1"));
    }
}

#[tokio::test]
async fn eight_rounds_allow_final_ninth_call_parallel_tools_count_once() {
    for (provider, model, responses) in [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Copilot, "gpt-4o", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ] {
        let mut streams = (0..8)
            .map(|i| {
                success(tool_sse_with_prefix(
                    responses,
                    &[
                        ("read_note", json!({"path":"a.md"})),
                        ("list_notes", json!({})),
                    ],
                    &format!("round_{i}_"),
                ))
            })
            .collect::<Vec<_>>();
        streams.push(success(text_sse(responses, "after eight")));
        let (_root, client, http) = client(provider, model, streams).await;
        let notes = Arc::new(Notes::default());
        let (a, _) = run(client, notes.clone(), CancellationToken::new()).await;
        assert!(matches!(a.terminal, AiTerminal::Completed), "{a:?}");
        assert_eq!(a.text, "after eight");
        assert_eq!(notes.calls.load(Ordering::SeqCst), 16);
        assert_eq!(http.bodies().len(), 9);
        http.assert_consumed();
    }
}

#[tokio::test]
async fn ninth_tool_round_dispatches_zero_tools() {
    for (provider, model, responses) in [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Copilot, "gpt-4o", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ] {
        let streams = (0..9)
            .map(|i| {
                success(tool_sse_with_prefix(
                    responses,
                    &[
                        ("read_note", json!({"path":"a.md"})),
                        ("list_notes", json!({})),
                    ],
                    &format!("round_{i}_"),
                ))
            })
            .collect();
        let (_root, client, http) = client(provider, model, streams).await;
        let notes = Arc::new(Notes::default());
        let (a, events) = run(client, notes.clone(), CancellationToken::new()).await;
        assert_eq!(notes.calls.load(Ordering::SeqCst), 16);
        assert!(
            matches!(
                a.terminal,
                AiTerminal::Failed(AiError {
                    kind: AiErrorKind::ToolLimitReached,
                    ..
                })
            ),
            "{a:?}"
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, AiEvent::ToolStarted { .. }))
                .count(),
            16
        );
        assert_eq!(http.bodies().len(), 9);
        http.assert_consumed();
    }
}

#[tokio::test]
async fn http_and_malformed_sse_errors_have_no_retry_or_raw_retention() {
    for (provider, model, responses) in [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Copilot, "gpt-4o", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ] {
        for (reply, kind, delay) in [
            (
                MockHttpResponse::error(
                    http_client::StatusCode::UNAUTHORIZED,
                    r#"{"error":{"message":"SYNTHETIC_SECRET"}}"#,
                ),
                AiErrorKind::ReconnectNeeded,
                None,
            ),
            (
                MockHttpResponse::error(
                    http_client::StatusCode::TOO_MANY_REQUESTS,
                    r#"{"error":{"resets_in_seconds":42,"message":"SYNTHETIC_SECRET"}}"#,
                ),
                AiErrorKind::RateLimited,
                Some(42),
            ),
            (
                success(if responses {
                    "data: {\"type\":\"response.output_text.delta\",\"delta\":42}\n\n".into()
                } else {
                    "data: {\"choices\":42}\n\n".into()
                }),
                AiErrorKind::Other,
                None,
            ),
            (
                MockHttpResponse::error(
                    http_client::StatusCode::BAD_REQUEST,
                    r#"{"error":{"code":"model_not_supported","message":"SYNTHETIC_SECRET"}}"#,
                ),
                AiErrorKind::ModelRefused,
                None,
            ),
        ] {
            let (_root, client, http) = client(provider, model, vec![reply]).await;
            let (a, _) = run(client, Arc::new(Notes::default()), CancellationToken::new()).await;
            let AiTerminal::Failed(e) = a.terminal else {
                panic!("{a:?}")
            };
            assert_eq!(e.kind, kind);
            assert_eq!(e.retry_after_seconds, delay);
            assert!(!format!("{e:?} {e}").contains("SYNTHETIC_SECRET"));
            assert_eq!(http.bodies().len(), 1);
            http.assert_consumed();
        }
    }
}

#[tokio::test]
async fn stop_after_partial_does_not_consume_final_or_duplicate_text() {
    let (_root, client, http) = client(
        Provider::Chatgpt,
        "gpt-5.5",
        vec![success(text_sse(true, "partial"))],
    )
    .await;
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    let events = Arc::new(Mutex::new(vec![]));
    let sink = events.clone();
    let a = answer(
        client,
        "q",
        &[],
        Arc::new(Notes::default()),
        cancel,
        Arc::new(move |e| {
            if matches!(e, AiEvent::Text(_)) {
                stop.cancel();
            }
            sink.lock().unwrap().push(e);
        }),
    )
    .await;
    assert!(matches!(a.terminal, AiTerminal::Interrupted), "{a:?}");
    assert_eq!(a.text, "partial");
    assert_eq!(events.lock().unwrap().len(), 1);
    http.assert_consumed();
}

#[tokio::test]
async fn unknown_tool_fails_without_dispatch_retry_or_name_leak() {
    let (_root, client, http) = client(
        Provider::Copilot,
        "gpt-4o",
        vec![success(tool_sse(
            false,
            &[("synthetic_untrusted_tool", json!({"secret":"synthetic"}))],
        ))],
    )
    .await;
    let notes = Arc::new(Notes::default());
    let (a, events) = run(client, notes.clone(), CancellationToken::new()).await;
    assert!(
        matches!(
            a.terminal,
            AiTerminal::Failed(AiError {
                kind: AiErrorKind::InvalidToolUse,
                ..
            })
        ),
        "{a:?}"
    );
    assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
    assert!(events.is_empty());
    http.assert_consumed();
}

#[tokio::test]
async fn rust_argument_validation_and_safe_failed_results_continue_without_retry() {
    for (name, args) in [
        ("search_notes", json!({"query":"","limit":1})),
        ("search_notes", json!({"query":"é".repeat(257),"limit":1})),
        ("search_notes", json!({"query":"x","limit":0})),
        ("search_notes", json!({"query":"x","limit":11})),
        (
            "search_notes",
            json!({"query":"x","limit":1,"unknown":true}),
        ),
        ("search_notes", json!({"query":"x","limit":1.5})),
        ("read_note", json!({"path":""})),
        ("read_note", json!({"path":42})),
        ("read_note", json!({"path":"a.md","unknown":true})),
        ("list_notes", json!({"folder":42})),
        ("list_notes", json!({"cursor":false})),
        ("list_notes", json!({"unknown":true})),
    ] {
        let (_root, client, http) = client(
            Provider::Copilot,
            "gpt-4o",
            vec![
                success(tool_sse(false, &[(name, args)])),
                success(text_sse(false, "safe continuation")),
            ],
        )
        .await;
        let notes = Arc::new(Notes::default());
        let (a, _) = run(client, notes.clone(), CancellationToken::new()).await;
        assert!(matches!(a.terminal, AiTerminal::Completed), "{a:?}");
        assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
        assert_eq!(a.text, "safe continuation");
        assert_eq!(http.bodies().len(), 2);
        http.assert_consumed();
    }
}

#[tokio::test]
async fn actual_sse_first_error_preserves_partial_and_discards_later_text() {
    let prefix = text_sse(false, "\u{feff}partial\r\n")
        .split("\n\n")
        .next()
        .unwrap()
        .to_owned();
    let sse = prefix + "\n\n" + "data: {\"choices\":42}\n\n" + &text_sse(false, "must not appear");
    let (_root, client, http) = client(Provider::Copilot, "gpt-4o", vec![success(sse)]).await;
    let (a, events) = run(client, Arc::new(Notes::default()), CancellationToken::new()).await;
    assert!(matches!(a.terminal, AiTerminal::Failed(_)), "{a:?}");
    assert_eq!(a.text, "\u{feff}partial\r\n");
    assert_eq!(
        events
            .iter()
            .filter_map(|e| match e {
                AiEvent::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect::<String>(),
        a.text
    );
    http.assert_consumed();
}

#[tokio::test]
async fn history_is_limited_to_last_twenty_text_pairs() {
    let (_root, client, http) = client(
        Provider::Copilot,
        "gpt-4o",
        vec![success(text_sse(false, "answer"))],
    )
    .await;
    let history = (0..25)
        .map(|i| HistoryPair {
            question: format!("question-{i}"),
            answer: format!("answer-{i}"),
        })
        .collect::<Vec<_>>();
    let a = answer(
        client,
        "new",
        &history,
        Arc::new(Notes::default()),
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    assert!(matches!(a.terminal, AiTerminal::Completed));
    let bodies = http.bodies();
    let messages = bodies[0]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 42); // preamble, twenty pairs, current question
    assert_eq!(messages[1]["content"], "question-5");
    assert_eq!(messages[40]["content"][0]["text"], "answer-24");
    assert_eq!(messages[41]["content"], "new");
    assert!(
        messages[1..41]
            .iter()
            .enumerate()
            .all(|(i, m)| m["role"] == if i % 2 == 0 { "user" } else { "assistant" })
    );
    http.assert_consumed();
}

#[tokio::test]
async fn empty_partial_history_keeps_question_but_omits_empty_assistant_on_wire() {
    let (_root, client, http) = client(
        Provider::Copilot,
        "gpt-4o",
        vec![success(text_sse(false, "answer"))],
    )
    .await;
    let result = answer(
        client,
        "new",
        &[HistoryPair {
            question: "stopped-before-text".into(),
            answer: String::new(),
        }],
        Arc::new(Notes::default()),
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    assert!(matches!(result.terminal, AiTerminal::Completed));
    let bodies = http.bodies();
    let messages = bodies[0]["messages"].as_array().unwrap();
    assert_eq!(
        messages.len(),
        3,
        "preamble, retained question, new question only"
    );
    assert_eq!(messages[1]["content"], "stopped-before-text");
    assert!(messages.iter().all(|m| m["role"] != "assistant"));
    http.assert_consumed();
}

struct OversizedNotes;
impl ReadTools for OversizedNotes {
    fn search_notes(&self, _: &str, _: usize) -> AiResult<ToolSearch> {
        Ok(ToolSearch {
            hits: (0..11)
                .map(|_| Passage {
                    path: "a.md".into(),
                    start_byte: 0,
                    end_byte: 1,
                    quote: "x".into(),
                })
                .collect(),
            keyword_only: true,
        })
    }
    fn read_note(&self, path: &str) -> AiResult<ToolNote> {
        Ok(ToolNote {
            path: path.into(),
            text: format!("{}é", "x".repeat(49_999)),
            truncated: false,
        })
    }
    fn list_notes(&self, _: Option<&str>, _: Option<&str>) -> AiResult<NotePage> {
        Ok(NotePage {
            notes: vec![
                NoteEntry {
                    path: "a.md".into(),
                    title: "a".into(),
                };
                201
            ],
            next_cursor: None,
        })
    }
}

#[tokio::test]
async fn actual_tools_enforce_utf8_note_cap_and_adapter_result_caps() {
    let (_root, client, http) = client(
        Provider::Copilot,
        "gpt-4o",
        vec![
            success(tool_sse(
                false,
                &[
                    ("read_note", json!({"path":"a.md"})),
                    ("list_notes", json!({"folder":"notes","cursor":"page-2"})),
                    ("search_notes", json!({"query":"x","limit":10})),
                ],
            )),
            success(text_sse(false, "handled")),
        ],
    )
    .await;
    let (a, _) = run(client, Arc::new(OversizedNotes), CancellationToken::new()).await;
    assert!(matches!(a.terminal, AiTerminal::Completed), "{a:?}");
    let bodies = http.bodies();
    let results = bodies[1]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["role"] == "tool")
        .collect::<Vec<_>>();
    assert_eq!(results.len(), 3);
    let note: Value = serde_json::from_str(results[0]["content"].as_str().unwrap()).unwrap();
    assert_eq!(note["text"].as_str().unwrap(), "x".repeat(49_999));
    assert_eq!(note["truncated"], true);
    for result in &results[1..] {
        let content = result["content"].as_str().unwrap();
        assert_eq!(content, "the tool failed");
        assert!(!content.contains("a.md"));
    }
    http.assert_consumed();
}

#[tokio::test]
async fn unexpected_absolute_urls_fail_closed_in_private_transport() {
    let http = ScriptHttp::new(vec![], vec![]);
    let request = Request::builder()
        .uri("https://unexpected.invalid/auth")
        .body(Vec::<u8>::new())
        .unwrap();
    assert!(http.send::<_, Vec<u8>>(request).await.is_err());
    assert_eq!(http.unexpected.load(Ordering::SeqCst), 1);
    assert_eq!(http.unary.remaining_responses(), 0);
}

#[tokio::test]
async fn response_less_transport_failure_is_safe_network_without_retry() {
    for (provider, model) in [
        (Provider::Chatgpt, "gpt-5.5"),
        (Provider::Copilot, "gpt-4o"),
        (Provider::Copilot, "gpt-5.3-codex"),
    ] {
        let (_root, client, http) =
            client_replies(provider, model, vec![Err(http_client::Error::StreamEnded)]).await;
        let (a, _) = run(client, Arc::new(Notes::default()), CancellationToken::new()).await;
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
        assert_eq!(http.bodies().len(), 1);
        http.assert_consumed();
    }
}

#[tokio::test]
async fn pre_cancelled_turn_sends_no_completion() {
    let (_root, client, http) = client(Provider::Copilot, "gpt-4o", vec![]).await;
    let cancel = CancellationToken::new();
    cancel.cancel();
    let (a, events) = run(client, Arc::new(Notes::default()), cancel).await;
    assert!(matches!(a.terminal, AiTerminal::Interrupted));
    assert!(a.text.is_empty() && events.is_empty() && http.bodies().is_empty());
    http.assert_consumed();
}

mod capability_probe_formats {
    use super::*;
    use crate::auth::OwnedClient;
    use crate::capability_probe::{ProbeKind, ProbeReport, ProbeTerminal, run_probe};
    use base64::Engine as _;
    use futures::StreamExt as _;

    const ROUTES: [(Provider, &str, bool); 3] = [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Copilot, "gpt-5.5", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ];

    fn failure(report: &ProbeReport, kind: AiErrorKind) {
        assert!(
            matches!(&report.terminal, ProbeTerminal::Failed(error) if error.kind == kind),
            "{report:?}"
        );
    }

    fn partial_sse(responses: bool, text: &str) -> String {
        text_sse(responses, text)
            .split("\n\n")
            .next()
            .unwrap()
            .to_owned()
            + "\n\n"
    }

    #[tokio::test]
    async fn probe_low_high_freeze_route_effort_model_and_single_synthetic_read() {
        for (provider, model, responses) in ROUTES {
            for (kind, effort) in [(ProbeKind::Low, "low"), (ProbeKind::High, "high")] {
                let (_root, client, http) = super::client(
                    provider,
                    model,
                    vec![
                        success(tool_sse(
                            responses,
                            &[("read_note", json!({"path":"probe.md"}))],
                        )),
                        success(text_sse(responses, "orchard-827")),
                    ],
                )
                .await;
                let report = run_probe(client, kind, CancellationToken::new()).await;
                assert_eq!(report.terminal, ProbeTerminal::Completed, "{report:?}");
                assert_eq!(
                    report.selection,
                    Selection {
                        provider,
                        model: model.into()
                    }
                );
                assert_eq!(report.kind, kind);
                assert_eq!(report.text, "orchard-827");
                assert_eq!(report.read_tool_calls, 1);
                assert!(!report.web_search_observed && report.citations.is_empty());
                let bodies = http.bodies();
                assert_eq!(bodies.len(), 2);
                for body in &bodies {
                    assert_eq!(body["model"], model);
                    assert_eq!(body["stream"], true);
                    if responses {
                        assert_eq!(body["reasoning"]["effort"], effort);
                        assert!(body.get("reasoning_effort").is_none());
                    } else {
                        assert_eq!(body["reasoning_effort"], effort);
                        assert!(body.get("reasoning").is_none());
                    }
                    let tools = body["tools"].as_array().unwrap();
                    assert_eq!(tools.len(), 1);
                    let tool = if responses {
                        &tools[0]
                    } else {
                        &tools[0]["function"]
                    };
                    assert_eq!(tool["name"], "read_note");
                    assert_eq!(tool["parameters"]["additionalProperties"], false);
                    assert_eq!(
                        tool["parameters"]["properties"]["path"]["enum"],
                        json!(["probe.md"])
                    );
                }
                let history = if responses { "input" } else { "messages" };
                assert!(!bodies[0][history].to_string().contains("orchard-827"));
                let continuation = bodies[1][history].as_array().unwrap();
                let results = continuation
                    .iter()
                    .filter(|item| {
                        if responses {
                            item["type"] == "function_call_output"
                        } else {
                            item["role"] == "tool"
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(results.len(), 1);
                assert_eq!(
                    results[0][if responses { "output" } else { "content" }],
                    "orchard-827"
                );
                for request in http.requests.lock().unwrap().iter() {
                    assert!(!request.headers.contains_key("copilot-vision-request"));
                }
                http.assert_consumed();
            }
        }
    }

    #[tokio::test]
    async fn probe_read_rejects_invalid_parallel_repeated_or_missing_tools_without_retry() {
        for (provider, model, responses) in ROUTES {
            for calls in [
                vec![("read_note", json!({"path":"other.md"}))],
                vec![(
                    "read_note",
                    json!({"path":"probe.md","extra":"SYNTHETIC_SECRET"}),
                )],
                vec![("read_note", json!({"path":1}))],
                vec![("write_note", json!({"path":"probe.md"}))],
                vec![
                    ("read_note", json!({"path":"probe.md"})),
                    ("read_note", json!({"path":"probe.md"})),
                ],
            ] {
                let kind = if calls.len() == 2 {
                    AiErrorKind::ToolLimitReached
                } else {
                    AiErrorKind::InvalidToolUse
                };
                let (_root, client, http) =
                    client(provider, model, vec![success(tool_sse(responses, &calls))]).await;
                let report = run_probe(client, ProbeKind::Low, CancellationToken::new()).await;
                failure(&report, kind);
                assert_eq!(report.read_tool_calls, 0);
                assert_eq!(http.bodies().len(), 1);
                assert!(
                    !serde_json::to_string(&report)
                        .unwrap()
                        .contains("SYNTHETIC_SECRET")
                );
                http.assert_consumed();
            }
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![
                    success(tool_sse(
                        responses,
                        &[("read_note", json!({"path":"probe.md"}))],
                    )),
                    success(tool_sse_with_prefix(
                        responses,
                        &[("read_note", json!({"path":"probe.md"}))],
                        "again_",
                    )),
                ],
            )
            .await;
            let report = run_probe(client, ProbeKind::High, CancellationToken::new()).await;
            failure(&report, AiErrorKind::ToolLimitReached);
            assert_eq!(report.read_tool_calls, 1);
            assert_eq!(http.bodies().len(), 2);
            http.assert_consumed();

            let (_root, client, http) = super::client(
                provider,
                model,
                vec![success(text_sse(responses, "guessed"))],
            )
            .await;
            let report = run_probe(client, ProbeKind::Low, CancellationToken::new()).await;
            failure(&report, AiErrorKind::InvalidToolUse);
            assert_eq!(report.text, "guessed");
            assert_eq!(report.read_tool_calls, 0);
            assert_eq!(http.bodies().len(), 1);
            http.assert_consumed();
        }
    }

    #[tokio::test]
    async fn probe_image_serializes_exact_fixture_png_and_copilot_vision_header() {
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(include_bytes!("fixtures/capability.png"));
        for (provider, model, responses) in ROUTES {
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![success(text_sse(responses, "upper: red; lower: blue"))],
            )
            .await;
            let report = run_probe(client, ProbeKind::Image, CancellationToken::new()).await;
            assert_eq!(report.terminal, ProbeTerminal::Completed, "{report:?}");
            assert_eq!(report.text, "upper: red; lower: blue");
            assert_eq!(report.read_tool_calls, 0);
            let bodies = http.bodies();
            assert_eq!(bodies.len(), 1);
            assert_eq!(bodies[0]["model"], model);
            let content = bodies[0][if responses { "input" } else { "messages" }]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|message| message["content"].as_array().into_iter().flatten());
            let images = content
                .filter(|part| {
                    part["type"]
                        == if responses {
                            "input_image"
                        } else {
                            "image_url"
                        }
                })
                .collect::<Vec<_>>();
            assert_eq!(images.len(), 1);
            let url = if responses {
                &images[0]["image_url"]
            } else {
                &images[0]["image_url"]["url"]
            };
            assert_eq!(url, &json!(format!("data:image/png;base64,{encoded}")));
            let detail = if responses {
                &images[0]["detail"]
            } else {
                &images[0]["image_url"]["detail"]
            };
            assert_eq!(detail, "high");
            let requests = http.requests.lock().unwrap();
            if provider == Provider::Copilot {
                assert_eq!(
                    requests[0].headers.get("copilot-vision-request").unwrap(),
                    "true"
                );
            } else {
                assert!(!requests[0].headers.contains_key("copilot-vision-request"));
            }
            drop(requests);
            http.assert_consumed();
        }
    }

    fn web_sse(completed: bool, terminal_only: bool) -> String {
        let hosted = json!({"type":"web_search_call","id":"ws_probe","status":if completed {"completed"} else {"in_progress"},"action":{"type":"search","queries":["SQLite"]}});
        let message = json!({"type":"message","id":"msg_synthetic","status":"completed","role":"assistant","content":[{
            "type":"output_text","text":"Synthetic SQLite release.","annotations":[
                {"type":"url_citation","start_index":0,"end_index":9,"url":"https://www.sqlite.org/changes.html","title":"SQLite changes"},
                {"type":"url_citation","start_index":0,"end_index":9,"url":"https://www.sqlite.org/changes.html","title":"Duplicate"},
                {"type":"url_citation","start_index":0,"end_index":9,"url":"https://user:SYNTHETIC_SECRET@example.invalid/","title":"Unsafe"},
                {"type":"url_citation","start_index":0,"end_index":9,"url":"javascript:alert(1)","title":"Unsafe"}
            ]
        }]});
        if terminal_only {
            return response_end(vec![hosted, message]);
        }
        event(
            json!({"type":"response.output_item.done","sequence_number":1,"output_index":0,"item":hosted}),
        ) + &partial_sse(true, "Synthetic SQLite release.")
            .replace("\"output_index\":0", "\"output_index\":1")
            + &event(
                json!({"type":"response.output_item.done","sequence_number":3,"output_index":1,"item":message}),
            )
            + &response_end(vec![hosted, message])
    }

    #[tokio::test]
    async fn probe_responses_web_parses_hosted_search_and_native_text_citations_once() {
        for (provider, model) in [
            (Provider::Chatgpt, "gpt-5.5"),
            (Provider::Copilot, "gpt-5.3-codex"),
        ] {
            for (completed, terminal_only) in
                [(false, false), (true, false), (false, true), (true, true)]
            {
                let (_root, client, http) = client(
                    provider,
                    model,
                    vec![success(web_sse(completed, terminal_only))],
                )
                .await;
                let report = run_probe(client, ProbeKind::Web, CancellationToken::new()).await;
                assert_eq!(report.terminal, ProbeTerminal::Completed, "{report:?}");
                assert_eq!(report.text, "Synthetic SQLite release.");
                assert_eq!(report.web_search_observed, completed);
                assert_eq!(report.read_tool_calls, 0);
                assert_eq!(report.citations.len(), 1, "{report:?}");
                assert_eq!(
                    report.citations[0].url,
                    "https://www.sqlite.org/changes.html"
                );
                assert_eq!(report.citations[0].title.as_deref(), Some("SQLite changes"));
                assert!(
                    !serde_json::to_string(&report)
                        .unwrap()
                        .contains("SYNTHETIC_SECRET")
                );
                let bodies = http.bodies();
                assert_eq!(bodies.len(), 1);
                assert_eq!(bodies[0]["model"], model);
                assert_eq!(bodies[0]["tools"], json!([{"type":"web_search"}]));
                http.assert_consumed();
            }
        }
        let (_root, client, http) = super::client(Provider::Copilot, "gpt-5.5", vec![]).await;
        let report = run_probe(client, ProbeKind::Web, CancellationToken::new()).await;
        failure(&report, AiErrorKind::ModelRefused);
        assert!(report.text.is_empty() && report.citations.is_empty() && http.bodies().is_empty());
        http.assert_consumed();
    }

    #[tokio::test]
    async fn probe_provider_failures_are_safe_and_never_retry_or_fallback() {
        for (provider, model, responses) in ROUTES {
            for (reply, kind, delay) in [
                (
                    Ok(MockHttpResponse::error(
                        http_client::StatusCode::BAD_REQUEST,
                        r#"{"error":{"code":"unsupported_api_for_model","message":"SYNTHETIC_SECRET"}}"#,
                    )),
                    AiErrorKind::ModelRefused,
                    None,
                ),
                (
                    Ok(MockHttpResponse::error(
                        http_client::StatusCode::UNAUTHORIZED,
                        r#"{"error":{"message":"SYNTHETIC_SECRET"}}"#,
                    )),
                    AiErrorKind::ReconnectNeeded,
                    None,
                ),
                (
                    Ok(MockHttpResponse::error(
                        http_client::StatusCode::TOO_MANY_REQUESTS,
                        r#"{"error":{"resets_in_seconds":42,"message":"SYNTHETIC_SECRET"}}"#,
                    )),
                    AiErrorKind::RateLimited,
                    Some(42),
                ),
                (
                    Ok(MockHttpResponse::error(
                        http_client::StatusCode::BAD_REQUEST,
                        r#"{"error":{"code":"model_not_supported","message":"SYNTHETIC_SECRET"}}"#,
                    )),
                    AiErrorKind::ModelRefused,
                    None,
                ),
                (
                    Err(http_client::Error::StreamEnded),
                    AiErrorKind::Network,
                    None,
                ),
                (
                    Ok(success(if responses {
                        "data: {\"type\":\"response.output_text.delta\",\"delta\":42,\"secret\":\"SYNTHETIC_SECRET\"}\n\n".into()
                    } else {
                        "data: {\"choices\":42,\"secret\":\"SYNTHETIC_SECRET\"}\n\n".into()
                    })),
                    AiErrorKind::Other,
                    None,
                ),
            ] {
                let (_root, client, http) = client_replies(provider, model, vec![reply]).await;
                let report = run_probe(client, ProbeKind::Image, CancellationToken::new()).await;
                failure(&report, kind);
                let ProbeTerminal::Failed(error) = &report.terminal else {
                    unreachable!()
                };
                assert_eq!(error.retry_after_seconds, delay);
                assert_eq!(
                    report.selection,
                    Selection {
                        provider,
                        model: model.into()
                    }
                );
                assert!(report.text.is_empty() && report.citations.is_empty());
                assert!(
                    !serde_json::to_string(&report)
                        .unwrap()
                        .contains("SYNTHETIC_SECRET")
                );
                assert_eq!(http.bodies().len(), 1);
                http.assert_consumed();
            }
        }
    }

    #[tokio::test]
    async fn probe_first_parser_error_keeps_exact_partial_and_discards_later_text() {
        for (provider, model, responses) in ROUTES {
            let malformed = if responses {
                "data: {\"type\":\"response.output_text.delta\",\"delta\":42}\n\n"
            } else {
                "data: {\"choices\":42}\n\n"
            };
            let sse = partial_sse(responses, "\u{feff}partial\r\nλ")
                + malformed
                + &text_sse(responses, "must not appear");
            let (_root, client, http) = super::client(provider, model, vec![success(sse)]).await;
            let report = run_probe(client, ProbeKind::Image, CancellationToken::new()).await;
            failure(&report, AiErrorKind::Other);
            assert_eq!(report.text, "\u{feff}partial\r\nλ");
            assert_eq!(http.bodies().len(), 1);
            http.assert_consumed();
        }
    }

    #[derive(Clone)]
    struct CancelAfterChunk {
        http: ScriptHttp,
        prefix: String,
        cancel: CancellationToken,
    }
    impl HttpClientExt for CancelAfterChunk {
        fn send<T, U>(
            &self,
            request: Request<T>,
        ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + Send + 'static
        where
            T: Into<Bytes> + Send,
            U: From<Bytes> + Send + 'static,
        {
            self.http.send(request)
        }
        fn send_multipart<U>(
            &self,
            request: Request<MultipartForm>,
        ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + Send + 'static
        where
            U: From<Bytes> + Send + 'static,
        {
            self.http.send_multipart(request)
        }
        fn send_streaming<T>(
            &self,
            request: Request<T>,
        ) -> impl Future<Output = http_client::Result<StreamingResponse>> + Send
        where
            T: Into<Bytes> + Send,
        {
            let response = self.http.send_streaming(request);
            let prefix = self.prefix.clone();
            let cancel = self.cancel.clone();
            async move {
                let (parts, _) = response.await?.into_parts();
                let chunks = futures::stream::once(async move { Ok(Bytes::from(prefix)) }).chain(
                    futures::stream::once(async move {
                        cancel.cancel();
                        futures::future::pending::<http_client::Result<Bytes>>().await
                    }),
                );
                Ok(Response::from_parts(
                    parts,
                    Box::pin(chunks) as http_client::BoxedStream,
                ))
            }
        }
    }

    #[tokio::test]
    async fn probe_cancellation_retains_parsed_partial_and_precancelled_sends_no_request() {
        for (provider, model, responses) in ROUTES {
            let (_root, mut client, http) = super::client(
                provider,
                model,
                vec![success(text_sse(responses, "unused"))],
            )
            .await;
            let cancel = CancellationToken::new();
            let transport = CancelAfterChunk {
                http: http.clone(),
                prefix: partial_sse(responses, "partial\r\nλ"),
                cancel: cancel.clone(),
            };
            client.inner = match client.inner {
                OwnedClient::Chatgpt(inner) => {
                    OwnedClient::Chatgpt(Box::new((*inner).with_http(transport)))
                }
                OwnedClient::Copilot(inner) => OwnedClient::Copilot(inner.with_http(transport)),
            };
            let report = run_probe(client, ProbeKind::Image, cancel).await;
            assert_eq!(report.terminal, ProbeTerminal::Interrupted, "{report:?}");
            assert_eq!(report.text, "partial\r\nλ");
            assert_eq!(http.bodies().len(), 1);
            http.assert_consumed();

            let (_root, client, http) = super::client(provider, model, vec![]).await;
            let cancel = CancellationToken::new();
            cancel.cancel();
            let report = run_probe(client, ProbeKind::Low, cancel).await;
            assert_eq!(report.terminal, ProbeTerminal::Interrupted);
            assert!(report.text.is_empty() && http.bodies().is_empty());
            assert_eq!(report.read_tool_calls, 0);
            http.assert_consumed();
        }
    }
}
