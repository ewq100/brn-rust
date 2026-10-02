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
