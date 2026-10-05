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

    const ROUTES: [(Provider, &str, bool); 4] = [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Chatgpt, "gpt-6-luna", true),
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

    const LANGUAGE_INSTRUCTION: &str = "Normally answer in the language of the current user question unless the user asks for another language. English and Estonian content may be mixed; preserve exact source quotes in their original language.";
    const MIXED_SOURCE: &str = "\u{feff}English quote: \"The deadline is Friday.\"\r\nEestikeelne tsitaat: „Tähtaeg on reede.“\r\n";

    #[derive(Default)]
    struct MixedLanguageNotes {
        calls: AtomicUsize,
    }

    impl ReadTools for MixedLanguageNotes {
        fn search_notes(&self, _: &str, _: usize) -> AiResult<ToolSearch> {
            panic!("only the captured read_note call is expected")
        }

        fn read_note(&self, path: &str) -> AiResult<ToolNote> {
            assert_eq!(path, "tähtaeg.md");
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ToolNote {
                path: path.into(),
                text: MIXED_SOURCE.into(),
                truncated: false,
            })
        }

        fn list_notes(&self, _: Option<&str>, _: Option<&str>) -> AiResult<NotePage> {
            panic!("only the captured read_note call is expected")
        }
    }

    async fn assert_ask_language_contract(with_effort: bool) {
        let questions = [
            "\u{feff}What is the tähtaeg? Quote both source languages exactly.\r\n",
            "\u{feff}Millal on deadline? Tsiteeri mõlemat allikat täpselt.\r\n",
            "Please answer in Estonian, including the exact English and Estonian quotes.\r\n",
            "Palun vasta inglise keeles ja säilita mõlemad täpsed tsitaadid.\r\n",
        ];
        let history = [HistoryPair {
            question: "Varasem eestikeelne küsimus. Earlier English question.".into(),
            answer: "Earlier mixed answer. Varasem vastus.".into(),
        }];
        for (provider, model, responses) in ROUTES {
            for question in questions {
                let (_root, client, http) = client(
                    provider,
                    model,
                    vec![
                        success(tool_sse(
                            responses,
                            &[("read_note", json!({"path":"tähtaeg.md"}))],
                        )),
                        success(text_sse(responses, MIXED_SOURCE)),
                    ],
                )
                .await;
                let notes = Arc::new(MixedLanguageNotes::default());
                let answer = if with_effort {
                    answer_with_effort(
                        client,
                        question,
                        &history,
                        ReasoningEffort::Medium,
                        notes.clone(),
                        CancellationToken::new(),
                        Arc::new(|_| {}),
                    )
                    .await
                } else {
                    answer(
                        client,
                        question,
                        &history,
                        notes.clone(),
                        CancellationToken::new(),
                        Arc::new(|_| {}),
                    )
                    .await
                };
                assert!(
                    matches!(answer.terminal, AiTerminal::Completed),
                    "{answer:?}"
                );
                assert_eq!(answer.text, MIXED_SOURCE);
                assert_eq!(notes.calls.load(Ordering::SeqCst), 1);
                let bodies = http.bodies();
                assert_eq!(bodies.len(), 2);
                for body in &bodies {
                    assert_eq!(body["model"], model);
                    let messages = body[if responses { "input" } else { "messages" }]
                        .as_array()
                        .unwrap();
                    let preamble = if provider == Provider::Chatgpt {
                        body["instructions"].as_str().unwrap().to_owned()
                    } else {
                        message_text(
                            messages
                                .iter()
                                .find(|message| message["role"] == "system")
                                .unwrap(),
                        )
                        .unwrap()
                    };
                    assert!(
                        preamble.contains(LANGUAGE_INSTRUCTION),
                        "{provider:?}/{model}: {preamble}"
                    );
                    let users = messages
                        .iter()
                        .filter(|message| message["role"] == "user")
                        .map(|message| message_text(message).unwrap())
                        .collect::<Vec<_>>();
                    assert_eq!(users, [history[0].question.clone(), question.to_owned()]);
                    if with_effort {
                        assert_eq!(
                            body[if responses {
                                "reasoning"
                            } else {
                                "reasoning_effort"
                            }],
                            if responses {
                                json!({"effort":"medium"})
                            } else {
                                json!("medium")
                            }
                        );
                    }
                }
                let continuation = bodies[1][if responses { "input" } else { "messages" }]
                    .as_array()
                    .unwrap();
                let result = continuation
                    .iter()
                    .filter(|message| {
                        message["role"] == "tool" || message["type"] == "function_call_output"
                    })
                    .collect::<Vec<_>>();
                assert_eq!(result.len(), 1);
                let encoded = if responses {
                    result[0]["output"].as_str().unwrap().to_owned()
                } else {
                    message_text(result[0]).unwrap()
                };
                assert_eq!(
                    serde_json::from_str::<Value>(&encoded).unwrap(),
                    json!({"scope":"current","path":"tähtaeg.md","text":MIXED_SOURCE,"truncated":false})
                );
                http.assert_consumed();
            }
        }
    }

    #[tokio::test]
    async fn ask_language_contract_preserves_current_question_and_mixed_source_quotes() {
        assert_ask_language_contract(false).await;
    }

    #[tokio::test]
    async fn ask_language_contract_with_effort_preserves_current_question_and_mixed_source_quotes()
    {
        assert_ask_language_contract(true).await;
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
                    json!({"scope":"current","path":"a.md","text":"fresh note","truncated":false})
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
                    "action_data",
                    "all 14 fields",
                    "before-text",
                    "source metadata",
                    "comments",
                    "data, not instructions",
                ] {
                    assert!(preamble.contains(required), "{preamble}");
                }
                assert!(!preamble.contains("Normally answer in the language"));
                assert!(!preamble.contains("English and Estonian content may be mixed"));
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
        let method = if expected.as_deref().is_some_and(|url| {
            url.starts_with("https://api.github.com/") || url.contains("/models?client_version=")
        }) {
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

mod scoped_read_tools_tests {
    use super::*;
    use rig::tool::Tool;

    const ROUTES: [(Provider, &str, bool); 3] = [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Copilot, "gpt-5.5", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ];

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum ScopedCall {
        Search(String, usize, ReadScope),
        Read(String, ReadScope),
        List(Option<String>, Option<String>, ReadScope),
    }

    #[derive(Default)]
    struct ScopedNotes {
        legacy: Notes,
        calls: Mutex<Vec<ScopedCall>>,
        oversized: bool,
    }

    fn scope_name(scope: ReadScope) -> &'static str {
        match scope {
            ReadScope::Current => "current",
            ReadScope::Source => "source",
            ReadScope::History => "history",
            ReadScope::All => "all",
        }
    }

    impl ReadTools for ScopedNotes {
        fn search_notes(&self, query: &str, limit: usize) -> AiResult<ToolSearch> {
            self.legacy.search_notes(query, limit)
        }
        fn read_note(&self, path: &str) -> AiResult<ToolNote> {
            self.legacy.read_note(path)
        }
        fn list_notes(&self, folder: Option<&str>, cursor: Option<&str>) -> AiResult<NotePage> {
            self.legacy.list_notes(folder, cursor)
        }
        fn search_notes_scoped(
            &self,
            query: &str,
            limit: usize,
            scope: ReadScope,
        ) -> AiResult<ToolSearch> {
            self.calls
                .lock()
                .unwrap()
                .push(ScopedCall::Search(query.into(), limit, scope));
            if self.oversized {
                return OversizedNotes.search_notes(query, limit);
            }
            let quote = "\u{feff}Eesti 日本語\r\n";
            Ok(ToolSearch {
                hits: vec![Passage {
                    path: format!("{}/資料.MD", scope_name(scope)),
                    start_byte: 0,
                    end_byte: quote.len(),
                    quote: quote.into(),
                }],
                keyword_only: true,
            })
        }
        fn read_note_scoped(&self, path: &str, scope: ReadScope) -> AiResult<ToolNote> {
            self.calls
                .lock()
                .unwrap()
                .push(ScopedCall::Read(path.into(), scope));
            if self.oversized {
                return OversizedNotes.read_note(path);
            }
            Ok(ToolNote {
                path: path.into(),
                text: format!("\u{feff}Exact {} Eesti 日本語 🦀\r\n", scope_name(scope)),
                truncated: false,
            })
        }
        fn list_notes_scoped(
            &self,
            folder: Option<&str>,
            cursor: Option<&str>,
            scope: ReadScope,
        ) -> AiResult<NotePage> {
            self.calls.lock().unwrap().push(ScopedCall::List(
                folder.map(str::to_owned),
                cursor.map(str::to_owned),
                scope,
            ));
            if self.oversized {
                return OversizedNotes.list_notes(folder, cursor);
            }
            Ok(NotePage {
                notes: vec![NoteEntry {
                    path: format!("{}/資料.MD", scope_name(scope)),
                    title: "\u{feff}Eesti 日本語\r\n".into(),
                }],
                next_cursor: Some(format!("{}/next.md", scope_name(scope))),
            })
        }
    }

    fn tool_results(body: &Value, responses: bool) -> Vec<(String, String)> {
        body[if responses { "input" } else { "messages" }]
            .as_array()
            .unwrap()
            .iter()
            .filter(|message| {
                message["type"] == "function_call_output" || message["role"] == "tool"
            })
            .map(|message| {
                let value = &message[if responses { "output" } else { "content" }];
                let text = value
                    .as_str()
                    .or_else(|| value[0]["text"].as_str())
                    .unwrap();
                (
                    message[if responses { "call_id" } else { "tool_call_id" }]
                        .as_str()
                        .unwrap()
                        .into(),
                    text.into(),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn explicit_scopes_reach_all_three_tools_and_keep_flattened_exact_outputs_on_each_route()
    {
        for (provider, model, responses) in ROUTES {
            for scope in [
                ReadScope::Current,
                ReadScope::Source,
                ReadScope::History,
                ReadScope::All,
            ] {
                let name = scope_name(scope);
                let (_root, client, http) = client(
                    provider,
                    model,
                    vec![
                        success(tool_sse(
                            responses,
                            &[
                                (
                                    "search_notes",
                                    json!({"query":"Eesti 日本語 λ","limit":3,"scope":name}),
                                ),
                                ("read_note", json!({"path":"archive/資料.MD","scope":name})),
                                (
                                    "list_notes",
                                    json!({"folder":"archive","cursor":"previous.md","scope":name}),
                                ),
                            ],
                        )),
                        success(text_sse(responses, "exact final")),
                    ],
                )
                .await;
                let notes = Arc::new(ScopedNotes::default());
                for parameters in [
                    crate::tools::SearchNotes(notes.clone()).parameters(),
                    crate::tools::ReadNote(notes.clone()).parameters(),
                    crate::tools::ListNotes(notes.clone()).parameters(),
                ] {
                    assert!(
                        !parameters["required"]
                            .as_array()
                            .unwrap()
                            .contains(&json!("scope"))
                    );
                }
                let (answer, events) = run(client, notes.clone(), CancellationToken::new()).await;
                assert!(
                    matches!(answer.terminal, AiTerminal::Completed),
                    "{provider:?}/{name}: {answer:?}"
                );
                assert_eq!(answer.text, "exact final");
                assert_eq!(notes.legacy.calls.load(Ordering::SeqCst), 0);
                let calls = notes.calls.lock().unwrap();
                assert_eq!(calls.len(), 3);
                for expected in [
                    ScopedCall::Search("Eesti 日本語 λ".into(), 3, scope),
                    ScopedCall::Read("archive/資料.MD".into(), scope),
                    ScopedCall::List(Some("archive".into()), Some("previous.md".into()), scope),
                ] {
                    assert!(calls.contains(&expected), "{calls:?}");
                }
                assert_eq!(
                    events
                        .iter()
                        .filter(|e| matches!(e, AiEvent::ToolStarted { .. }))
                        .count(),
                    3
                );
                let bodies = http.bodies();
                assert_eq!(bodies.len(), 2);
                for body in &bodies {
                    assert_eq!(body["model"], model);
                    for tool in body["tools"].as_array().unwrap() {
                        let definition = if responses { tool } else { &tool["function"] };
                        let parameters = &definition["parameters"];
                        assert_eq!(parameters["additionalProperties"], false);
                        if matches!(
                            definition["name"].as_str(),
                            Some("read_action" | "list_actions")
                        ) {
                            assert!(parameters["properties"]["scope"].is_null());
                            continue;
                        }
                        assert_eq!(parameters["properties"]["scope"]["type"], "string");
                        assert_eq!(
                            parameters["properties"]["scope"]["enum"],
                            json!(["current", "source", "history", "all"])
                        );
                        // Rig's pinned Copilot Responses dialect normalizes
                        // strict schemas so every property becomes required.
                        // Rust omission compatibility remains independently tested.
                        assert_eq!(
                            parameters["required"]
                                .as_array()
                                .unwrap()
                                .contains(&json!("scope")),
                            provider == Provider::Copilot && responses,
                            "{provider:?}/{model}: {parameters}"
                        );
                    }
                }
                let results = tool_results(&bodies[1], responses);
                assert_eq!(results.len(), 3);
                let payload = |id| {
                    serde_json::from_str::<Value>(
                        &results.iter().find(|(call, _)| call == id).unwrap().1,
                    )
                    .unwrap()
                };
                let quote = "\u{feff}Eesti 日本語\r\n";
                assert_eq!(
                    payload("call_0"),
                    json!({"scope":name,"hits":[{"path":format!("{name}/資料.MD"),"start_byte":0,"end_byte":quote.len(),"quote":quote}],"keyword_only":true})
                );
                assert_eq!(
                    payload("call_1"),
                    json!({"scope":name,"path":"archive/資料.MD","text":format!("\u{feff}Exact {name} Eesti 日本語 🦀\r\n"),"truncated":false})
                );
                assert_eq!(
                    payload("call_2"),
                    json!({"scope":name,"notes":[{"path":format!("{name}/資料.MD"),"title":quote}],"next_cursor":format!("{name}/next.md")})
                );
                http.assert_consumed();
            }
        }
    }

    #[tokio::test]
    async fn legacy_implementers_keep_omitted_current_and_refuse_explicit_other_scopes_on_each_route()
     {
        for (provider, model, responses) in ROUTES {
            for scope in [None, Some("source"), Some("history"), Some("all")] {
                let mut calls = vec![
                    ("search_notes", json!({"query":"fresh","limit":1})),
                    ("read_note", json!({"path":"a.md"})),
                    ("list_notes", json!({})),
                ];
                if let Some(scope) = scope {
                    for (_, args) in &mut calls {
                        args["scope"] = json!(scope);
                    }
                }
                let (_root, client, http) = client(
                    provider,
                    model,
                    vec![
                        success(tool_sse(responses, &calls)),
                        success(text_sse(responses, "safe final")),
                    ],
                )
                .await;
                let notes = Arc::new(Notes::default());
                let (answer, _) = run(client, notes.clone(), CancellationToken::new()).await;
                assert!(
                    matches!(answer.terminal, AiTerminal::Completed),
                    "{answer:?}"
                );
                let bodies = http.bodies();
                assert_eq!(bodies.len(), 2);
                let results = tool_results(&bodies[1], responses);
                assert_eq!(results.len(), 3);
                if scope.is_none() {
                    assert_eq!(notes.calls.load(Ordering::SeqCst), 3);
                    let payloads = results
                        .iter()
                        .map(|(_, text)| serde_json::from_str::<Value>(text).unwrap())
                        .collect::<Vec<_>>();
                    assert!(payloads.contains(&json!({"scope":"current","hits":[{"path":"a.md","start_byte":0,"end_byte":5,"quote":"fresh"}],"keyword_only":true})));
                    assert!(payloads.contains(&json!({"scope":"current","path":"a.md","text":"fresh note","truncated":false})));
                    assert!(
                        payloads
                            .contains(&json!({"scope":"current","notes":[],"next_cursor":null}))
                    );
                } else {
                    assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
                    assert!(
                        results.iter().all(|(_, text)| text == "the tool failed"),
                        "{results:?}"
                    );
                }
                http.assert_consumed();
            }
        }
    }

    #[tokio::test]
    async fn invalid_scope_and_unknown_properties_refuse_before_any_underlying_read_on_each_route()
    {
        for (provider, model, responses) in ROUTES {
            for invalid in [
                json!("SYNTHETIC_SECRET"),
                json!(null),
                json!(42),
                json!("CURRENT"),
                json!({"scope":"current"}),
            ] {
                let calls = [
                    (
                        "search_notes",
                        json!({"query":"fresh","limit":1,"scope":invalid}),
                    ),
                    ("read_note", json!({"path":"a.md","scope":invalid})),
                    ("list_notes", json!({"scope":invalid})),
                ];
                assert_safe_refusal(provider, model, responses, &calls).await;
            }
            let calls = [
                (
                    "search_notes",
                    json!({"query":"fresh","limit":1,"scope":"source","unknown":"SYNTHETIC_SECRET"}),
                ),
                (
                    "read_note",
                    json!({"path":"a.md","scope":"history","unknown":"SYNTHETIC_SECRET"}),
                ),
                (
                    "list_notes",
                    json!({"scope":"all","unknown":"SYNTHETIC_SECRET"}),
                ),
            ];
            assert_safe_refusal(provider, model, responses, &calls).await;
        }
    }

    async fn assert_safe_refusal(
        provider: Provider,
        model: &str,
        responses: bool,
        calls: &[(&str, Value)],
    ) {
        let (_root, client, http) = client(
            provider,
            model,
            vec![
                success(tool_sse(responses, calls)),
                success(text_sse(responses, "safe continuation")),
            ],
        )
        .await;
        let notes = Arc::new(ScopedNotes::default());
        let (answer, events) = run(client, notes.clone(), CancellationToken::new()).await;
        assert!(
            matches!(answer.terminal, AiTerminal::Completed),
            "{answer:?}"
        );
        assert_eq!(answer.text, "safe continuation");
        assert_eq!(notes.legacy.calls.load(Ordering::SeqCst), 0);
        assert!(notes.calls.lock().unwrap().is_empty());
        let bodies = http.bodies();
        assert_eq!(bodies.len(), 2);
        let results = tool_results(&bodies[1], responses);
        assert_eq!(results.len(), calls.len());
        // Rig returns argument parsing detail to the model that generated the
        // invalid argument. Those results never enter local progress/AiError;
        // adapter execution failures instead use its fixed failed-tool copy.
        assert!(
            results.iter().all(|(_, text)| text == "the tool failed"
                || text.starts_with("failed to parse tool arguments: ")),
            "{results:?}"
        );
        assert!(!format!("{answer:?} {events:?}").contains("SYNTHETIC_SECRET"));
        http.assert_consumed();
    }

    #[tokio::test]
    async fn explicit_scope_keeps_utf8_truncation_and_search_page_result_caps_on_each_route() {
        for (provider, model, responses) in ROUTES {
            let (_root, client, http) = client(
                provider,
                model,
                vec![
                    success(tool_sse(
                        responses,
                        &[
                            (
                                "read_note",
                                json!({"path":"archive/a.md","scope":"history"}),
                            ),
                            (
                                "list_notes",
                                json!({"folder":"archive","cursor":"page-2","scope":"history"}),
                            ),
                            (
                                "search_notes",
                                json!({"query":"x","limit":10,"scope":"history"}),
                            ),
                        ],
                    )),
                    success(text_sse(responses, "handled")),
                ],
            )
            .await;
            let notes = Arc::new(ScopedNotes {
                oversized: true,
                ..Default::default()
            });
            let (answer, _) = run(client, notes.clone(), CancellationToken::new()).await;
            assert!(
                matches!(answer.terminal, AiTerminal::Completed),
                "{answer:?}"
            );
            assert_eq!(notes.calls.lock().unwrap().len(), 3);
            assert_eq!(notes.legacy.calls.load(Ordering::SeqCst), 0);
            let bodies = http.bodies();
            assert_eq!(bodies.len(), 2);
            let results = tool_results(&bodies[1], responses);
            assert_eq!(results.len(), 3);
            let note = serde_json::from_str::<Value>(
                &results.iter().find(|(id, _)| id == "call_0").unwrap().1,
            )
            .unwrap();
            assert_eq!(
                note,
                json!({"scope":"history","path":"archive/a.md","text":"x".repeat(49_999),"truncated":true})
            );
            for id in ["call_1", "call_2"] {
                assert_eq!(
                    results.iter().find(|(call, _)| call == id).unwrap().1,
                    "the tool failed"
                );
            }
            http.assert_consumed();
        }
    }

    #[tokio::test]
    async fn explicit_scope_keeps_query_byte_limit_hit_limit_and_empty_path_validation_on_each_route()
     {
        for (provider, model, responses) in ROUTES {
            assert_safe_refusal(
                provider,
                model,
                responses,
                &[
                    (
                        "search_notes",
                        json!({"query":"","limit":1,"scope":"source"}),
                    ),
                    (
                        "search_notes",
                        json!({"query":"é".repeat(257),"limit":1,"scope":"source"}),
                    ),
                    (
                        "search_notes",
                        json!({"query":"x","limit":0,"scope":"source"}),
                    ),
                    (
                        "search_notes",
                        json!({"query":"x","limit":11,"scope":"source"}),
                    ),
                    ("read_note", json!({"path":"","scope":"source"})),
                ],
            )
            .await;
        }
    }
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
            assert_eq!(
                names,
                [
                    "list_actions",
                    "list_notes",
                    "read_action",
                    "read_note",
                    "search_notes"
                ]
            );
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
        for (provider, model, responses) in
            ROUTES
                .into_iter()
                .chain([(Provider::Chatgpt, "gpt-6-luna", true)])
        {
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
        for (provider, model, responses) in
            ROUTES
                .into_iter()
                .chain([(Provider::Chatgpt, "gpt-6-luna", true)])
        {
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
                assert_eq!(http.bodies()[0]["model"], model);
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

mod subscription_catalog_tests {
    use super::*;
    use base64::Engine;

    fn catalog_url() -> String {
        format!(
            "https://chatgpt.com/backend-api/codex/models?client_version={}",
            env!("CARGO_PKG_VERSION")
        )
    }

    fn auth(reply: MockHttpResponse, expired: bool) -> (tempfile::TempDir, Auth, ScriptHttp) {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let dir = root.path().join("credentials");
        let url = catalog_url();
        let mut unary = vec![];
        if expired {
            unary.push(("https://auth.openai.com/oauth/token", MockHttpResponse::success(json!({
                "access_token":"SYNTHETIC_ACCESS", "refresh_token":"SYNTHETIC_REFRESH", "expires_in":3600,
                "id_token": format!("e30.{}.synthetic", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
                    serde_json::to_vec(&json!({"https://api.openai.com/auth":{"chatgpt_account_id":"synthetic-account"}})).unwrap()))
            }).to_string())));
        }
        unary.push((url.as_str(), reply));
        let http = ScriptHttp::new(unary, vec![]);
        let auth = Auth::open(&dir).unwrap().with_http(http.clone());
        let path = dir.join("chatgpt.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&json!({
                "access_token": if expired { "SYNTHETIC_OLD" } else { "SYNTHETIC_ACCESS" },
                "refresh_token": "SYNTHETIC_REFRESH",
                "expires_at": if expired { 1 } else { 4102444800_i64 },
                "account_id": "synthetic-account"
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
        (root, auth, http)
    }

    #[tokio::test]
    async fn authenticated_subscription_catalog_preserves_wire_identity_and_server_order() {
        for expired in [false, true] {
            let (_root, auth, http) = auth(MockHttpResponse::success(json!({"models":[
                {"slug":"late-model","visibility":"list","priority":20,"supported_in_api":true},
                {"slug":"hidden-model","visibility":"hide","priority":0},
                {"slug":"future.subscription-model","visibility":"list","priority":-1,"supported_in_api":false},
                {"slug":"tie-one","visibility":"list","priority":3},
                {"slug":"not-listed","visibility":"none","priority":0},
                {"slug":"tie-two","visibility":"list","priority":3}
            ]}).to_string()), expired);
            let models = auth
                .models(Provider::Chatgpt, CancellationToken::new())
                .await
                .unwrap();
            assert_eq!(
                models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
                [
                    "future.subscription-model",
                    "tie-one",
                    "tie-two",
                    "late-model"
                ]
            );
            assert!(models.iter().all(|m| !m.live_qualified));
            let requests = http.unary.requests();
            assert_eq!(requests.len(), if expired { 2 } else { 1 });
            let request = requests.last().unwrap();
            assert_eq!(request.uri, catalog_url());
            assert!(request.body.is_empty());
            assert_eq!(request.headers["authorization"], "Bearer SYNTHETIC_ACCESS");
            assert_eq!(request.headers["chatgpt-account-id"], "synthetic-account");
            assert_eq!(request.headers["originator"], "rig");
            assert_eq!(
                request.headers["user-agent"],
                format!(
                    "rig/0.43.0 ({} {}; rig)",
                    std::env::consts::OS,
                    std::env::consts::ARCH
                )
            );
            http.assert_consumed();
        }
    }

    #[tokio::test]
    async fn malformed_subscription_catalog_is_an_atomic_safe_failure() {
        for body in [
            "SYNTHETIC_SECRET",
            "{}",
            r#"{"data":[{"id":"old-api-model"}]}"#,
            r#"{"models":null}"#,
            r#"{"models":{}}"#,
            r#"{"models":["SYNTHETIC_SECRET"]}"#,
            r#"{"models":[{"slug":"good","visibility":"list","priority":1},{"slug":"bad\n","visibility":"hide","priority":1}]}"#,
            r#"{"models":[{"slug":"good","visibility":"list","priority":1},{"slug":"good","visibility":"hide","priority":2}]}"#,
            r#"{"models":[{"slug":"good","visibility":"unknown","priority":1}]}"#,
            r#"{"models":[{"slug":"good","priority":1}]}"#,
            r#"{"models":[{"slug":"good","visibility":"list"}]}"#,
            r#"{"models":[{"slug":42,"visibility":"list","priority":1}]}"#,
            r#"{"models":[{"slug":"good","visibility":"list","priority":"1"}]}"#,
            r#"{"models":[{"slug":"good","visibility":"list","priority":1.5}]}"#,
            r#"{"models":[{"slug":"good","visibility":"list","priority":2147483648}]}"#,
        ] {
            let (_root, auth, http) = auth(MockHttpResponse::success(body), false);
            let error = auth
                .models(Provider::Chatgpt, CancellationToken::new())
                .await
                .unwrap_err();
            assert_eq!(error.kind, AiErrorKind::Other);
            assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SECRET"));
            http.assert_consumed();
        }
        let (_root, auth, http) = auth(MockHttpResponse::success(r#"{"models":[]}"#), false);
        assert!(
            auth.models(Provider::Chatgpt, CancellationToken::new())
                .await
                .unwrap()
                .is_empty()
        );
        http.assert_consumed();
    }

    #[tokio::test]
    async fn subscription_catalog_statuses_are_safe_without_fallback_or_retry() {
        for transport_error in [false, true] {
            for (status, body, kind, retry) in [
                (401, "SYNTHETIC_SECRET", AiErrorKind::ReconnectNeeded, None),
                (403, "SYNTHETIC_SECRET", AiErrorKind::ReconnectNeeded, None),
                (
                    429,
                    r#"{"error":{"resets_in_seconds":42,"message":"SYNTHETIC_SECRET"}}"#,
                    AiErrorKind::RateLimited,
                    Some(42),
                ),
                (
                    400,
                    r#"{"error":{"code":"unsupported_api_for_model","message":"SYNTHETIC_SECRET"}}"#,
                    AiErrorKind::ModelRefused,
                    None,
                ),
                (500, "SYNTHETIC_SECRET", AiErrorKind::Other, None),
            ] {
                let status = http_client::StatusCode::from_u16(status).unwrap();
                let reply = if transport_error {
                    MockHttpResponse::error(status, body)
                } else {
                    MockHttpResponse::ErrorResponse(status, body.into())
                };
                let (_root, auth, http) = auth(reply, false);
                let error = auth
                    .models(Provider::Chatgpt, CancellationToken::new())
                    .await
                    .unwrap_err();
                assert_eq!((error.kind, error.retry_after_seconds), (kind, retry));
                assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SECRET"));
                http.assert_consumed();
            }
        }
    }
}

mod action_read_tool_tests {
    use super::*;

    #[derive(Default)]
    struct Actions {
        calls: AtomicUsize,
    }
    impl ReadTools for Actions {
        fn search_notes(&self, _: &str, _: usize) -> AiResult<ToolSearch> {
            panic!("unexpected note search")
        }
        fn read_note(&self, _: &str) -> AiResult<ToolNote> {
            panic!("unexpected note read")
        }
        fn list_notes(&self, _: Option<&str>, _: Option<&str>) -> AiResult<NotePage> {
            panic!("unexpected note list")
        }
        fn read_action(&self, id: &str) -> AiResult<Value> {
            assert_eq!(id, "9ecbe87a-8797-440c-9e08-25b424ef8b9c");
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(
                json!({"origin":{"id":id,"data":{"title":"Exact origin õ\r\n"}},
                "data":{"description":"\u{feff}Exact current 🦀\r\n"},"version":2}),
            )
        }
        fn list_actions(
            &self,
            state: Option<&str>,
            limit: usize,
            cursor: Option<&str>,
        ) -> AiResult<Value> {
            assert_eq!((state, limit, cursor), (Some("waiting"), 1, Some("opaque")));
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(
                json!({"entries":[{"data":{"state":"waiting","owner":"  Õ  "}}],"next_cursor":"next-opaque"}),
            )
        }
    }

    #[tokio::test]
    async fn registered_action_read_tools_continue_exactly_on_both_rig_routes_and_rewrite() {
        for (provider, model, responses) in [
            (Provider::Chatgpt, "gpt-6-luna", true),
            (Provider::Copilot, "gpt-5.5", false),
            (Provider::Copilot, "gpt-5.3-codex", true),
        ] {
            for rewriting in [false, true] {
                let (_root, client, http) = client(
                    provider,
                    model,
                    vec![
                        success(tool_sse(
                            responses,
                            &[
                                (
                                    "read_action",
                                    json!({"id":"9ecbe87a-8797-440c-9e08-25b424ef8b9c"}),
                                ),
                                (
                                    "list_actions",
                                    json!({"state":"waiting","limit":1,"cursor":"opaque"}),
                                ),
                            ],
                        )),
                        success(text_sse(responses, "exact final")),
                    ],
                )
                .await;
                let tools = Arc::new(Actions::default());
                let events = Arc::new(Mutex::new(Vec::new()));
                let captured = events.clone();
                let emit = Arc::new(move |e| captured.lock().unwrap().push(e));
                let result = if rewriting {
                    rewrite(
                        client,
                        "captured review",
                        ReasoningEffort::High,
                        tools.clone(),
                        CancellationToken::new(),
                        emit,
                    )
                    .await
                } else {
                    answer_with_effort(
                        client,
                        "current work",
                        &[],
                        ReasoningEffort::High,
                        tools.clone(),
                        CancellationToken::new(),
                        emit,
                    )
                    .await
                };
                assert!(
                    matches!(result.terminal, AiTerminal::Completed),
                    "{provider:?} rewrite={rewriting}: {:?}",
                    result.terminal
                );
                assert_eq!(result.text, "exact final");
                assert_eq!(tools.calls.load(Ordering::SeqCst), 2);
                assert_eq!(
                    events
                        .lock()
                        .unwrap()
                        .iter()
                        .filter(|e| matches!(e, AiEvent::ToolStarted { .. }))
                        .count(),
                    2
                );
                http.assert_consumed();
                let bodies = http.bodies();
                let continuation = bodies[1].to_string();
                assert!(
                    continuation.contains("Exact origin õ")
                        && continuation.contains("Exact current 🦀")
                        && continuation.contains("next-opaque")
                );
                for tool in bodies[0]["tools"].as_array().unwrap() {
                    let definition = if responses { tool } else { &tool["function"] };
                    if ["read_action", "list_actions"]
                        .contains(&definition["name"].as_str().unwrap())
                    {
                        assert_eq!(definition["parameters"]["additionalProperties"], false);
                    }
                }
            }
        }
    }

    #[derive(Default)]
    struct BoundedActions {
        calls: AtomicUsize,
        oversized: bool,
    }
    impl ReadTools for BoundedActions {
        fn search_notes(&self, _: &str, _: usize) -> AiResult<ToolSearch> {
            panic!("unexpected")
        }
        fn read_note(&self, _: &str) -> AiResult<ToolNote> {
            panic!("unexpected")
        }
        fn list_notes(&self, _: Option<&str>, _: Option<&str>) -> AiResult<NotePage> {
            panic!("unexpected")
        }
        fn read_action(&self, _: &str) -> AiResult<Value> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(json!({"description":"x".repeat(READ_ACTION_BYTES+1)}))
        }
        fn list_actions(
            &self,
            state: Option<&str>,
            limit: usize,
            cursor: Option<&str>,
        ) -> AiResult<Value> {
            assert_eq!((state, limit, cursor), (None, 20, None));
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.oversized {
                Ok(
                    json!({"entries":["x".repeat(READ_ACTION_BYTES+1)],"next_cursor":"never fabricated"}),
                )
            } else {
                Ok(json!({"entries":[],"next_cursor":null}))
            }
        }
    }
    fn results(body: &Value, responses: bool) -> Vec<String> {
        body[if responses { "input" } else { "messages" }]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["type"] == "function_call_output" || m["role"] == "tool")
            .map(|m| {
                let v = &m[if responses { "output" } else { "content" }];
                v.as_str().or_else(|| v[0]["text"].as_str()).unwrap().into()
            })
            .collect()
    }
    #[tokio::test]
    async fn action_read_tool_argument_and_output_bounds_refuse_whole_results_without_retry() {
        for (provider, model, responses) in [
            (Provider::Chatgpt, "gpt-6-luna", true),
            (Provider::Copilot, "gpt-5.5", false),
            (Provider::Copilot, "gpt-5.3-codex", true),
        ] {
            for (name, args) in [
                ("read_action", json!({})),
                ("read_action", json!({"id":null})),
                ("read_action", json!({"id":1})),
                ("read_action", json!({"id":""})),
                ("read_action", json!({"id":"x".repeat(65)})),
                ("read_action", json!({"id":"x","scope":"all"})),
                ("list_actions", json!({"limit":0})),
                ("list_actions", json!({"limit":21})),
                ("list_actions", json!({"limit":-1})),
                ("list_actions", json!({"limit":1.5})),
                ("list_actions", json!({"state":false})),
                ("list_actions", json!({"state":"x".repeat(17)})),
                ("list_actions", json!({"cursor":false})),
                ("list_actions", json!({"cursor":"õ".repeat(129)})),
                ("list_actions", json!({"unknown":"SYNTHETIC_SECRET"})),
            ] {
                let (_root, client, http) = client(
                    provider,
                    model,
                    vec![
                        success(tool_sse(responses, &[(name, args)])),
                        success(text_sse(responses, "safe continuation")),
                    ],
                )
                .await;
                let tools = Arc::new(BoundedActions::default());
                let (answer, events) = run(client, tools.clone(), CancellationToken::new()).await;
                assert!(matches!(answer.terminal, AiTerminal::Completed));
                assert_eq!(tools.calls.load(Ordering::SeqCst), 0);
                let replies = results(&http.bodies()[1], responses);
                assert_eq!(replies.len(), 1);
                assert!(
                    replies[0] == "the tool failed"
                        || replies[0].starts_with("failed to parse tool arguments: ")
                );
                assert!(!format!("{answer:?} {events:?}").contains("SYNTHETIC_SECRET"));
                http.assert_consumed();
            }
            for (name, args, oversized) in [
                ("read_action", json!({"id":"x"}), true),
                ("list_actions", json!({}), true),
                ("list_actions", json!({}), false),
            ] {
                let (_root, client, http) = client(
                    provider,
                    model,
                    vec![
                        success(tool_sse(responses, &[(name, args)])),
                        success(text_sse(responses, "safe continuation")),
                    ],
                )
                .await;
                let tools = Arc::new(BoundedActions {
                    oversized,
                    ..BoundedActions::default()
                });
                let (answer, _) = run(client, tools.clone(), CancellationToken::new()).await;
                assert!(matches!(answer.terminal, AiTerminal::Completed));
                assert_eq!(tools.calls.load(Ordering::SeqCst), 1);
                let replies = results(&http.bodies()[1], responses);
                assert_eq!(replies.len(), 1);
                if oversized {
                    assert_eq!(replies[0], "the tool failed");
                } else {
                    assert_eq!(
                        serde_json::from_str::<Value>(&replies[0]).unwrap(),
                        json!({"entries":[],"next_cursor":null})
                    );
                }
                http.assert_consumed();
            }
        }
    }
}

mod action_proposal_tool_tests {
    use super::*;

    fn args() -> Value {
        json!({"id":"abc8e3e6-5419-4a09-a5f7-b7d9e98f8f29", "title":"Exact review õ\r\n",
        "source_paths":["archive/source.md"], "action_changes":[{
            "kind":"create","id":"5b344a65-e247-4b2c-9941-c4b52c405bdb", "data":{
                "title":"  Whole action õ  ", "description":"\u{feff}Exact 🦀\r\n", "state":"waiting",
                "owner":null,"related_person":null,"related_project":null,"sources":[],"thread":null,
                "due_on":"2028-02-29","follow_up_on":null,"dependencies":[],"parent":null,"follows_up":null,"priority":null
            }
        }]})
    }
    #[derive(Default)]
    struct Proposals(AtomicUsize);
    impl ReadTools for Proposals {
        fn search_notes(&self, _: &str, _: usize) -> AiResult<ToolSearch> {
            panic!("unexpected read")
        }
        fn read_note(&self, _: &str) -> AiResult<ToolNote> {
            panic!("unexpected read")
        }
        fn list_notes(&self, _: Option<&str>, _: Option<&str>) -> AiResult<NotePage> {
            panic!("unexpected read")
        }
    }
    impl ProposalTools for Proposals {
        fn propose_actions(&self, input: ActionProposalArgs) -> AiResult<Value> {
            assert_eq!(serde_json::to_value(input).unwrap(), args());
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(
                json!({"stamp":{"id":"abc8e3e6-5419-4a09-a5f7-b7d9e98f8f29","version":1},"state":"draft","session_id":"29103b44-f7b7-44f9-975c-8832526b8b2b","action_ids":["5b344a65-e247-4b2c-9941-c4b52c405bdb"],"sources":[]}),
            )
        }
    }
    #[tokio::test]
    async fn registered_action_proposal_tool_is_ask_only_and_continues_exactly_on_all_rig_routes() {
        for (provider, model, responses) in [
            (Provider::Chatgpt, "gpt-6-luna", true),
            (Provider::Copilot, "gpt-5.5", false),
            (Provider::Copilot, "gpt-5.3-codex", true),
        ] {
            let (_root, client, http) = client(
                provider,
                model,
                vec![
                    success(tool_sse(responses, &[("propose_actions", args())])),
                    success(text_sse(responses, "Review ready")),
                ],
            )
            .await;
            let tools = Arc::new(Proposals::default());
            let events = Arc::new(Mutex::new(Vec::new()));
            let captured = events.clone();
            let answer = answer_with_proposals(
                client,
                "Suggest an Action; do not approve it",
                &[],
                ReasoningEffort::High,
                tools.clone(),
                tools.clone(),
                CancellationToken::new(),
                Arc::new(move |e| captured.lock().unwrap().push(e)),
            )
            .await;
            assert!(
                matches!(answer.terminal, AiTerminal::Completed),
                "{provider:?}/{model}: {:?}",
                answer.terminal
            );
            assert_eq!(answer.text, "Review ready");
            assert_eq!(tools.0.load(Ordering::SeqCst), 1);
            assert!(
                matches!(events.lock().unwrap().as_slice(),[AiEvent::ToolStarted{name},AiEvent::Text(text)] if name=="propose_actions" && text=="Review ready")
            );
            http.assert_consumed();
            let bodies = http.bodies();
            let continuation = bodies[1].to_string();
            assert!(
                continuation.contains("abc8e3e6-5419-4a09-a5f7-b7d9e98f8f29")
                    && continuation.contains("draft")
            );
            let definitions = bodies[0]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| if responses { t } else { &t["function"] })
                .collect::<Vec<_>>();
            let proposal = definitions
                .iter()
                .find(|t| t["name"] == "propose_actions")
                .unwrap();
            assert_eq!(proposal["parameters"]["additionalProperties"], false);
            let mut required = proposal["parameters"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect::<Vec<_>>();
            required.sort_unstable();
            assert_eq!(required, ["action_changes", "id", "source_paths", "title"]);
            let members = &proposal["parameters"]["properties"]["action_changes"]["items"];
            let variants = members["anyOf"]
                .as_array()
                .expect("strict-compatible tagged union");
            assert_eq!(variants.len(), 2);
            fn closed(v: &Value, count: usize) {
                let fields = v["properties"].as_object().unwrap();
                assert_eq!(fields.len(), count);
                assert_eq!(v["additionalProperties"], false);
                let mut required = v["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect::<Vec<_>>();
                required.sort_unstable();
                assert_eq!(
                    required,
                    fields.keys().map(String::as_str).collect::<Vec<_>>()
                );
            }
            for variant in variants {
                closed(variant, 3);
                closed(&variant["properties"]["data"], 14);
            }
            assert_eq!(
                variants[0]["properties"]["kind"],
                json!({"type":"string","enum":["create"]})
            );
            assert_eq!(
                variants[1]["properties"]["kind"],
                json!({"type":"string","enum":["replace"]})
            );
            let before = &variants[1]["properties"]["before"];
            closed(before, 6);
            closed(&before["properties"]["data"], 14);
            closed(&before["properties"]["origin"], 4);
            closed(&before["properties"]["origin"]["properties"]["data"], 14);
            let schema = proposal["parameters"].to_string();
            for unsupported in ["oneOf", "uniqueItems", "const"] {
                assert!(!schema.contains(&format!("\"{unsupported}\":")));
            }

            let (_root, client, http) = super::client(
                provider,
                model,
                vec![success(tool_sse(responses, &[("propose_actions", args())]))],
            )
            .await;
            let refused = rewrite(
                client,
                "captured review",
                ReasoningEffort::High,
                tools.clone(),
                CancellationToken::new(),
                Arc::new(|_| {}),
            )
            .await;
            assert!(matches!(
                refused.terminal,
                AiTerminal::Failed(AiError {
                    kind: AiErrorKind::InvalidToolUse,
                    ..
                })
            ));
            assert_eq!(tools.0.load(Ordering::SeqCst), 1);
            http.assert_consumed();
            assert!(
                http.bodies()[0]["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|t| {
                        let d = if responses { t } else { &t["function"] };
                        d["name"] != "propose_actions"
                    })
            );
        }
    }
}

mod knowledge_proposal_tool_tests {
    use super::*;

    const ROUTES: [(Provider, &str, bool); 3] = [
        (Provider::Chatgpt, "gpt-6-luna", true),
        (Provider::Copilot, "gpt-5.5", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ];
    fn args() -> Value {
        json!({"id":"abc8e3e6-5419-4a09-a5f7-b7d9e98f8f29","title":"Whole knowledge õ\r\n",
            "path":"knowledge/derived.md","note_id":"5b344a65-e247-4b2c-9941-c4b52c405bdb",
            "text":"\u{feff}# Candidate 🦀\r\nWhole candidate.\r\n",
            "quotes":[{"start_byte":0,"end_byte":12},{"start_byte":24,"end_byte":48}]})
    }
    fn receipt() -> Value {
        json!({"stamp":{"id":"abc8e3e6-5419-4a09-a5f7-b7d9e98f8f29","version":1},
            "state":"draft","note_id":"5b344a65-e247-4b2c-9941-c4b52c405bdb",
            "source":"approved/source.md"})
    }
    struct Proposals {
        calls: AtomicUsize,
        enabled: bool,
        refuse: bool,
    }
    impl Proposals {
        fn new(enabled: bool, refuse: bool) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                enabled,
                refuse,
            }
        }
    }
    impl ReadTools for Proposals {
        fn search_notes(&self, _: &str, _: usize) -> AiResult<ToolSearch> {
            panic!("unexpected read")
        }
        fn read_note(&self, _: &str) -> AiResult<ToolNote> {
            panic!("unexpected read")
        }
        fn list_notes(&self, _: Option<&str>, _: Option<&str>) -> AiResult<NotePage> {
            panic!("unexpected read")
        }
    }
    impl ProposalTools for Proposals {
        fn propose_actions(&self, _: ActionProposalArgs) -> AiResult<Value> {
            panic!("unexpected Action proposal")
        }
        fn knowledge_enabled(&self) -> bool {
            self.enabled
        }
        fn propose_knowledge(&self, input: KnowledgeProposalArgs) -> AiResult<Value> {
            assert_eq!(serde_json::to_value(input).unwrap(), args());
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.refuse {
                Err(AiError::new(AiErrorKind::ToolRejected))
            } else {
                Ok(receipt())
            }
        }
    }
    fn definition(body: &Value, responses: bool, name: &str) -> Option<Value> {
        body["tools"].as_array().unwrap().iter().find_map(|tool| {
            let tool = if responses { tool } else { &tool["function"] };
            (tool["name"] == name).then(|| tool.clone())
        })
    }
    fn closed(schema: &Value, fields: &[&str]) {
        assert_eq!(schema["additionalProperties"], false);
        let properties = schema["properties"].as_object().unwrap();
        assert_eq!(properties.len(), fields.len());
        let mut required = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();
        required.sort_unstable();
        let mut expected = fields.to_vec();
        expected.sort_unstable();
        assert_eq!(required, expected);
        assert_eq!(
            required,
            properties.keys().map(String::as_str).collect::<Vec<_>>()
        );
    }
    fn replies(body: &Value, responses: bool) -> Vec<String> {
        body[if responses { "input" } else { "messages" }]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["type"] == "function_call_output" || item["role"] == "tool")
            .map(|item| {
                let content = &item[if responses { "output" } else { "content" }];
                content
                    .as_str()
                    .or_else(|| content[0]["text"].as_str())
                    .unwrap()
                    .to_owned()
            })
            .collect()
    }
    fn preamble(body: &Value, provider: Provider, responses: bool) -> String {
        if provider == Provider::Chatgpt {
            body["instructions"].as_str().unwrap().to_owned()
        } else {
            body[if responses { "input" } else { "messages" }]
                .as_array()
                .unwrap()
                .iter()
                .find(|message| message["role"] == "system")
                .unwrap()["content"]
                .to_string()
        }
    }
    #[tokio::test]
    async fn enabled_knowledge_tool_registers_closed_schema_and_continues_on_all_rig_routes() {
        for (provider, model, responses) in ROUTES {
            let (_root, client, http) = client(
                provider,
                model,
                vec![
                    success(tool_sse(responses, &[("propose_knowledge", args())])),
                    success(text_sse(responses, "Knowledge review ready")),
                ],
            )
            .await;
            let tools = Arc::new(Proposals::new(true, false));
            let events = Arc::new(Mutex::new(Vec::new()));
            let captured = events.clone();
            let result = answer_with_proposals(
                client,
                "Analyze the selected approved Inbox Source",
                &[],
                ReasoningEffort::High,
                tools.clone(),
                tools.clone(),
                CancellationToken::new(),
                Arc::new(move |event| captured.lock().unwrap().push(event)),
            )
            .await;
            assert!(
                matches!(result.terminal, AiTerminal::Completed),
                "{provider:?}/{model}: {:?}",
                result.terminal
            );
            assert_eq!(result.text, "Knowledge review ready");
            assert_eq!(tools.calls.load(Ordering::SeqCst), 1);
            assert!(matches!(events.lock().unwrap().as_slice(),
                [AiEvent::ToolStarted { name }, AiEvent::Text(text)]
                if name == "propose_knowledge" && text == "Knowledge review ready"));
            http.assert_consumed();
            let bodies = http.bodies();
            let outputs = replies(&bodies[1], responses);
            assert_eq!(outputs.len(), 1);
            assert_eq!(
                serde_json::from_str::<Value>(&outputs[0]).unwrap(),
                receipt()
            );
            let proposal = definition(&bodies[0], responses, "propose_knowledge").unwrap();
            closed(
                &proposal["parameters"],
                &["id", "title", "path", "note_id", "text", "quotes"],
            );
            closed(
                &proposal["parameters"]["properties"]["quotes"]["items"],
                &["start_byte", "end_byte"],
            );
            let schema = proposal["parameters"].to_string();
            for unsupported in ["const", "oneOf", "uniqueItems"] {
                assert!(!schema.contains(&format!("\"{unsupported}\":")));
            }
            let preamble = preamble(&bodies[0], provider, responses);
            assert!(preamble.contains("Workflow adds exact saved citations"));
            assert!(preamble.contains("explicitly selected approved Inbox Source"));
            assert!(definition(&bodies[0], responses, "propose_actions").is_some());
        }
    }

    #[tokio::test]
    async fn knowledge_tool_is_absent_and_cannot_dispatch_for_ordinary_ask_read_only_and_rewrite() {
        for (provider, model, responses) in ROUTES {
            for route in [
                "ordinary_proposals",
                "read_only",
                "read_only_effort",
                "rewrite",
            ] {
                let (_root, client, http) = client(
                    provider,
                    model,
                    vec![success(tool_sse(
                        responses,
                        &[("propose_knowledge", args())],
                    ))],
                )
                .await;
                // Even an enabled backend passed as read tools must not add proposals.
                let tools = Arc::new(Proposals::new(route != "ordinary_proposals", false));
                let emit = Arc::new(|_| {});
                let cancel = CancellationToken::new();
                let result = match route {
                    "ordinary_proposals" => {
                        answer_with_proposals(
                            client,
                            "ordinary Ask",
                            &[],
                            ReasoningEffort::High,
                            tools.clone(),
                            tools.clone(),
                            cancel,
                            emit,
                        )
                        .await
                    }
                    "read_only" => {
                        answer(client, "read-only Ask", &[], tools.clone(), cancel, emit).await
                    }
                    "read_only_effort" => {
                        answer_with_effort(
                            client,
                            "read-only Ask",
                            &[],
                            ReasoningEffort::High,
                            tools.clone(),
                            cancel,
                            emit,
                        )
                        .await
                    }
                    "rewrite" => {
                        rewrite(
                            client,
                            "captured review",
                            ReasoningEffort::High,
                            tools.clone(),
                            cancel,
                            emit,
                        )
                        .await
                    }
                    _ => unreachable!(),
                };
                assert!(
                    matches!(
                        result.terminal,
                        AiTerminal::Failed(AiError {
                            kind: AiErrorKind::InvalidToolUse,
                            ..
                        })
                    ),
                    "{provider:?}/{model}/{route}: {:?}",
                    result.terminal
                );
                assert_eq!(tools.calls.load(Ordering::SeqCst), 0);
                http.assert_consumed();
                let bodies = http.bodies();
                assert!(definition(&bodies[0], responses, "propose_knowledge").is_none());
                assert!(!preamble(&bodies[0], provider, responses).contains("propose_knowledge"));
                assert_eq!(
                    definition(&bodies[0], responses, "propose_actions").is_some(),
                    route == "ordinary_proposals"
                );
            }
        }
    }

    #[tokio::test]
    async fn knowledge_tool_shares_the_eight_round_budget_and_never_dispatches_the_ninth() {
        for (provider, model, responses) in ROUTES {
            for ninth in [false, true] {
                let mut streams = (0..if ninth { 9 } else { 8 })
                    .map(|round| {
                        success(tool_sse_with_prefix(
                            responses,
                            &[("propose_knowledge", args())],
                            &format!("knowledge_{round}_"),
                        ))
                    })
                    .collect::<Vec<_>>();
                if !ninth {
                    streams.push(success(text_sse(responses, "Eight review receipts")));
                }
                let (_root, client, http) = client(provider, model, streams).await;
                let tools = Arc::new(Proposals::new(true, false));
                let result = answer_with_proposals(
                    client,
                    "Selected source",
                    &[],
                    ReasoningEffort::High,
                    tools.clone(),
                    tools.clone(),
                    CancellationToken::new(),
                    Arc::new(|_| {}),
                )
                .await;
                if ninth {
                    assert!(
                        matches!(
                            result.terminal,
                            AiTerminal::Failed(AiError {
                                kind: AiErrorKind::ToolLimitReached,
                                ..
                            })
                        ),
                        "{provider:?}/{model}: {:?}",
                        result.terminal
                    );
                } else {
                    assert!(
                        matches!(result.terminal, AiTerminal::Completed),
                        "{provider:?}/{model}: {:?}",
                        result.terminal
                    );
                    assert_eq!(result.text, "Eight review receipts");
                }
                assert_eq!(tools.calls.load(Ordering::SeqCst), 8);
                http.assert_consumed();
            }
        }
    }

    #[tokio::test]
    async fn knowledge_protocol_and_owner_refusals_continue_safely_on_all_rig_routes() {
        for (provider, model, responses) in ROUTES {
            for rejection in ["unknown", "missing", "range", "owner"] {
                let mut input = args();
                match rejection {
                    "unknown" => input["approve"] = json!(true),
                    "missing" => {
                        input.as_object_mut().unwrap().remove("quotes");
                    }
                    "range" => input["quotes"][0]["end_byte"] = json!(16 * 1024 + 1),
                    "owner" => {}
                    _ => unreachable!(),
                }
                let (_root, client, http) = client(
                    provider,
                    model,
                    vec![
                        success(tool_sse(responses, &[("propose_knowledge", input)])),
                        success(text_sse(responses, "Review refused")),
                    ],
                )
                .await;
                let tools = Arc::new(Proposals::new(true, true));
                let result = answer_with_proposals(
                    client,
                    "Selected source",
                    &[],
                    ReasoningEffort::High,
                    tools.clone(),
                    tools.clone(),
                    CancellationToken::new(),
                    Arc::new(|_| {}),
                )
                .await;
                assert!(
                    matches!(result.terminal, AiTerminal::Completed),
                    "{provider:?}/{model}/{rejection}: {:?}",
                    result.terminal
                );
                assert_eq!(result.text, "Review refused");
                assert_eq!(
                    tools.calls.load(Ordering::SeqCst),
                    usize::from(rejection == "owner")
                );
                http.assert_consumed();
                let outputs = replies(&http.bodies()[1], responses);
                assert_eq!(outputs.len(), 1);
                // Rig parse errors are transient; application refusals use its safe fixed result.
                if ["range", "owner"].contains(&rejection) {
                    assert_eq!(outputs[0], "the tool failed");
                }
                assert!(!outputs[0].contains("state"));
            }
        }
    }
}
