//! The per-response bound on a live run: a model that keeps sending bytes
//! ends at BRN's duration or size limit with an honest failure, no tool
//! dispatch and no re-send; normal completion and Stop are unchanged.
use super::*;
use std::sync::atomic::AtomicUsize;
use std::time::{Duration, Instant};

const TEXT_DELTA: &str = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"x \",\"item_id\":\"msg_l\",\"output_index\":0,\"content_index\":0,\"sequence_number\":1}\n\n";
const KEEPALIVE: &str = ": keep-alive\n\n";

/// Finite scripted replies first, then a stream that starts with `head` and
/// repeats `repeat` every `every` without ever ending.
#[derive(Clone)]
struct Trickle {
    script: Arc<Vec<String>>,
    head: String,
    repeat: String,
    every: Duration,
    sent: Arc<AtomicUsize>,
}

impl Trickle {
    fn new(script: Vec<String>, head: &str, repeat: &str, every: Duration) -> Self {
        Self {
            script: Arc::new(script),
            head: head.into(),
            repeat: repeat.into(),
            every,
            sent: Arc::default(),
        }
    }
    fn sent(&self) -> usize {
        self.sent.load(Ordering::SeqCst)
    }
}

impl rig::http_client::HttpClientExt for Trickle {
    fn send<T, U>(
        &self,
        _req: rig::http_client::Request<T>,
    ) -> impl Future<
        Output = rig::http_client::Result<
            rig::http_client::Response<rig::http_client::LazyBody<U>>,
        >,
    > + Send
    + 'static
    where
        T: Into<bytes::Bytes> + Send,
        U: From<bytes::Bytes> + Send + 'static,
    {
        std::future::ready(Err(rig::http_client::Error::StreamEnded))
    }

    fn send_multipart<U>(
        &self,
        _req: rig::http_client::Request<rig::http_client::MultipartForm>,
    ) -> impl Future<
        Output = rig::http_client::Result<
            rig::http_client::Response<rig::http_client::LazyBody<U>>,
        >,
    > + Send
    + 'static
    where
        U: From<bytes::Bytes> + Send + 'static,
    {
        std::future::ready(Err(rig::http_client::Error::StreamEnded))
    }

    fn send_streaming<T>(
        &self,
        _req: rig::http_client::Request<T>,
    ) -> impl Future<Output = rig::http_client::Result<rig::http_client::StreamingResponse>> + Send
    where
        T: Into<bytes::Bytes> + Send,
    {
        let n = self.sent.fetch_add(1, Ordering::SeqCst);
        let body: rig::http_client::BoxedStream = match self.script.get(n) {
            Some(reply) => Box::pin(futures::stream::iter([Ok(bytes::Bytes::from(
                reply.clone(),
            ))])),
            None => {
                let (repeat, every) = (bytes::Bytes::from(self.repeat.clone()), self.every);
                Box::pin(futures::StreamExt::chain(
                    futures::stream::iter([Ok(bytes::Bytes::from(self.head.clone()))]),
                    futures::stream::unfold(repeat, move |repeat| async move {
                        tokio::time::sleep(every).await;
                        Some((Ok(repeat.clone()), repeat))
                    }),
                ))
            }
        };
        std::future::ready(
            rig::http_client::Response::builder()
                .status(200)
                .header("content-type", "text/event-stream")
                .body(body)
                .map_err(rig::http_client::Error::Protocol),
        )
    }
}

type Events = Arc<std::sync::Mutex<Vec<AiEvent>>>;

fn recorder() -> (Events, Arc<dyn Fn(AiEvent) + Send + Sync>) {
    let events: Events = Arc::default();
    let seen = events.clone();
    (
        events,
        Arc::new(move |event| seen.lock().unwrap().push(event)),
    )
}

fn limit_code(answer: &AiAnswer) -> (AiErrorKind, String, bool) {
    let AiTerminal::Failed(error) = &answer.terminal else {
        panic!("{:?}", answer.terminal);
    };
    let detail = error.detail.clone().unwrap();
    (error.kind, detail.code.unwrap(), detail.retryable)
}

/// The Copilot chat route over the guarded transport, as `run_prepared_over`
/// builds it, without credentials.
async fn copilot_run(
    http: Trickle,
    limits: (Duration, u64),
    cancel: CancellationToken,
) -> (AiAnswer, Vec<AiEvent>) {
    let (_root, request, _) = tests::fixture();
    let (scope, input) = begin(&request).unwrap();
    scope.response_limit.set(limits.0, limits.1);
    let client = rig::providers::openai::OpenAIConfig::with_key(
        &rig::providers::copilot::DIALECT,
        "synthetic-not-a-credential",
    )
    .connect(scope.response_limit.wrap(DynHttpClient::new(http)));
    let (events, emit) = recorder();
    let answer = run_copilot(
        &client,
        "synthetic-model".into(),
        scope,
        &input,
        ReasoningEffort::Medium,
        4,
        cancel,
        emit,
    )
    .await;
    let events = events.lock().unwrap().clone();
    (answer, events)
}

/// A Copilot chat stream that stays active with keep-alives but never
/// answers ends at the duration, is not retried on Responses and is not
/// re-sent.
#[tokio::test]
async fn continuously_active_copilot_stream_ends_at_the_duration() {
    let http = Trickle::new(vec![], "", KEEPALIVE, Duration::from_millis(5));
    let started = Instant::now();
    let (answer, events) = copilot_run(
        http.clone(),
        (Duration::from_millis(300), u64::MAX),
        CancellationToken::new(),
    )
    .await;
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(
        limit_code(&answer),
        (
            AiErrorKind::TimeLimitReached,
            "brn_response_duration_exceeded".into(),
            false
        )
    );
    assert_eq!(http.sent(), 1, "no Responses retry and no re-send");
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, AiEvent::AnswerFinalized(_)))
    );
    let AiTerminal::Failed(error) = &answer.terminal else {
        unreachable!()
    };
    assert_eq!(
        failed_attention(error),
        "Run failed: A model response reached BRN's time limit. Continue starts fresh."
    );
}

/// Stop ends a continuously active stream promptly and is reported as
/// Stop, not as a limit.
#[tokio::test]
async fn stop_during_a_continuously_active_stream_interrupts_promptly() {
    let http = Trickle::new(vec![], "", KEEPALIVE, Duration::from_millis(5));
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        stop.cancel();
    });
    let started = Instant::now();
    let (answer, _) = copilot_run(http.clone(), (Duration::from_secs(60), u64::MAX), cancel).await;
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(
        matches!(answer.terminal, AiTerminal::Interrupted),
        "{:?}",
        answer.terminal
    );
    assert_eq!(http.sent(), 1);
}

#[cfg(unix)]
async fn codex_run(
    http: DynHttpClient,
    limits: (Duration, u64),
) -> (tempfile::TempDir, PathBuf, ThreadResult, Vec<AiEvent>) {
    let (root, mut request, _) = tests::fixture();
    request.auth_file = Some(cache_tests::codex_file(&root));
    cache_tests::configure(&request, "Chatgpt");
    request.max_tool_rounds = 4;
    let data = request.data_dir.clone();
    let prepared = prepare_run(request).unwrap();
    prepared.scope.response_limit.set(limits.0, limits.1);
    let (events, emit) = recorder();
    let result = run_prepared_over(prepared, CancellationToken::new(), emit, Some(http))
        .await
        .unwrap();
    let events = events.lock().unwrap().clone();
    (root, data, result, events)
}

/// A Codex response that streams text without end ends at the duration. The
/// progress an earlier round retained is kept on the failed run.
#[cfg(unix)]
#[tokio::test]
async fn codex_text_without_end_fails_at_the_duration_and_keeps_progress() {
    let http = Trickle::new(
        vec![cache_tests::tool_turn(1, "r1")],
        TEXT_DELTA,
        TEXT_DELTA,
        Duration::from_millis(10),
    );
    let (_root, data, result, events) = codex_run(
        DynHttpClient::new(http.clone()),
        (Duration::from_millis(400), u64::MAX),
    )
    .await;
    assert_eq!(limit_code(&result.answer).0, AiErrorKind::TimeLimitReached);
    assert_eq!(http.sent(), 2, "the limited response is not re-sent");
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, AiEvent::AnswerFinalized(_)))
    );
    let store = Store::open(&data).unwrap();
    let RecordData::Run(run) = store.record(&result.run_id).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(run.state, RunState::Failed);
    assert_eq!(run.progress, "step 1", "accepted progress survives");
}

/// Tool arguments count toward the output limit while they stream; the call
/// they would form is never dispatched.
#[cfg(unix)]
#[tokio::test]
async fn oversized_tool_arguments_fail_at_the_size_limit_without_dispatch() {
    let head = "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"sequence_number\":1,\"item\":{\"type\":\"function_call\",\"id\":\"fc_big\",\"call_id\":\"call_big\",\"name\":\"retain_progress\",\"arguments\":\"\",\"status\":\"in_progress\"}}\n\n";
    let repeat = format!(
        "data: {{\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_big\",\"sequence_number\":2,\"delta\":\"{}\"}}\n\n",
        "a".repeat(4096)
    );
    let http = Trickle::new(vec![], head, &repeat, Duration::from_millis(1));
    let started = Instant::now();
    let (_root, _, result, events) = codex_run(
        DynHttpClient::new(http.clone()),
        (Duration::from_secs(60), 64 * 1024),
    )
    .await;
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(
        limit_code(&result.answer),
        (
            AiErrorKind::Other,
            "brn_response_output_exceeded".into(),
            false
        )
    );
    assert_eq!(http.sent(), 1);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, AiEvent::ToolStarted { .. })),
        "{events:?}"
    );
    let AiTerminal::Failed(error) = &result.answer.terminal else {
        unreachable!()
    };
    assert_eq!(
        failed_attention(error),
        "Run failed: A model response reached BRN's size limit. Continue starts fresh."
    );
}

/// A normal run under short limits that it does not reach completes as
/// before.
#[cfg(unix)]
#[tokio::test]
async fn normal_completion_within_the_limits_is_unchanged() {
    let http = cache_tests::script();
    let (_root, _, result, events) = codex_run(
        DynHttpClient::new(http.clone()),
        (Duration::from_secs(5), 64 * 1024),
    )
    .await;
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer.terminal
    );
    assert_eq!(result.answer.text, "Done");
    assert_eq!(http.requests().len(), 3);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AiEvent::AnswerFinalized(_)))
    );
}
