//! The bound on each provider response of one run: a total duration from send
//! and an aggregate size of every received byte (answer text, reasoning,
//! tool-call arguments and framing alike). Rig's idle bound only ends a silent
//! stream, and the answer cap counts only answer text, so a model that keeps
//! sending would otherwise run and buffer without end.
//!
//! One guard serves a run's transport, shared by its summary and helpers. A
//! response past either limit ends as a transport error, so Rig fails the turn
//! before any of its tool calls run. The guard then refuses every later request
//! of the run: a request the provider may have processed is never sent again.
use crate::{AiError, AiErrorKind, AiFailureDetail};
use futures::StreamExt;
use rig::http_client::{
    self, DynHttpClient, HttpClientExt, LazyBody, MultipartForm, Request, Response,
    StreamingResponse,
};
use rig_core::wasm_compat::WasmCompatSend;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::task::Poll;
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::bytes::Bytes;

/// The longest one model response may take, opening included. About 32k
/// output tokens (BRN's 128 KiB answer cap, or a large write proposal) at a
/// slow 40 tokens per second take under 14 minutes, leaving room for a long
/// reasoning phase; Rig's idle bound (300 s) still ends a silent response.
pub(crate) const RESPONSE_DURATION: Duration = Duration::from_secs(20 * 60);
/// The most one model response may send, framing included. A streamed token
/// costs about 200-250 framing bytes, so even a full 128k-token response is
/// about 26-32 MB on the wire; the bound also caps what Rig buffers for tool
/// arguments.
pub(crate) const RESPONSE_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;

const OPEN: u8 = 0;
const DURATION: u8 = 1;
const OUTPUT: u8 = 2;

pub(crate) struct ResponseLimit {
    duration_ms: AtomicU64,
    output_bytes: AtomicU64,
    tripped: AtomicU8,
}

impl Default for ResponseLimit {
    fn default() -> Self {
        Self::new(RESPONSE_DURATION, RESPONSE_OUTPUT_BYTES)
    }
}

impl ResponseLimit {
    pub(crate) fn new(duration: Duration, output_bytes: u64) -> Self {
        Self {
            duration_ms: AtomicU64::new(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)),
            output_bytes: AtomicU64::new(output_bytes),
            tripped: AtomicU8::new(OPEN),
        }
    }

    /// Short synthetic limits for deterministic tests, before the run sends.
    #[cfg(test)]
    pub(crate) fn set(&self, duration: Duration, output_bytes: u64) {
        self.duration_ms.store(
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
            Ordering::SeqCst,
        );
        self.output_bytes.store(output_bytes, Ordering::SeqCst);
    }

    /// `inner`, with every response bounded by this guard.
    pub(crate) fn wrap(self: &Arc<Self>, inner: DynHttpClient) -> DynHttpClient {
        DynHttpClient::new(Limited {
            inner,
            guard: self.clone(),
        })
    }

    /// The run's failure once any response reached a limit.
    pub(crate) fn failure(&self) -> Option<AiError> {
        let (kind, code) = match self.tripped.load(Ordering::SeqCst) {
            DURATION => (
                AiErrorKind::TimeLimitReached,
                "brn_response_duration_exceeded",
            ),
            OUTPUT => (AiErrorKind::Other, "brn_response_output_exceeded"),
            _ => return None,
        };
        Some(AiError::new(kind).with_detail(AiFailureDetail {
            origin: "response_limit".into(),
            code: Some(code.into()),
            ..Default::default()
        }))
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms.load(Ordering::SeqCst))
    }

    /// The first limit reached names the run's failure.
    fn trip(&self, cause: u8) -> http_client::Error {
        let _ = self
            .tripped
            .compare_exchange(OPEN, cause, Ordering::SeqCst, Ordering::SeqCst);
        refused()
    }

    /// The deadline of a new response, or `None` once a limit was reached.
    fn admit(&self) -> Option<Instant> {
        (self.tripped.load(Ordering::SeqCst) == OPEN).then(|| Instant::now() + self.duration())
    }
}

#[derive(Debug)]
struct Exceeded;
impl std::fmt::Display for Exceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BRN response limit reached")
    }
}
impl std::error::Error for Exceeded {}

fn refused() -> http_client::Error {
    http_client::Error::Instance(Box::new(Exceeded))
}

struct Limited {
    inner: DynHttpClient,
    guard: Arc<ResponseLimit>,
}

impl HttpClientExt for Limited {
    /// A whole-body reply is held to the duration; its transport reads it.
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        T: Into<Bytes> + WasmCompatSend,
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        let admitted = self.guard.admit();
        // Built but never polled when refused, so nothing is sent.
        let sending = self.inner.send(req);
        let guard = self.guard.clone();
        async move {
            let deadline = admitted.ok_or_else(refused)?;
            let response = tokio::time::timeout_at(deadline, sending)
                .await
                .map_err(|_| guard.trip(DURATION))??;
            Ok(response.map(|body| -> LazyBody<U> {
                Box::pin(async move {
                    tokio::time::timeout_at(deadline, body)
                        .await
                        .map_err(|_| guard.trip(DURATION))?
                })
            }))
        }
    }

    fn send_multipart<U>(
        &self,
        req: Request<MultipartForm>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        self.inner.send_multipart(req)
    }

    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<StreamingResponse>> + WasmCompatSend
    where
        T: Into<Bytes> + WasmCompatSend,
    {
        let req: Request<Bytes> = req.map(Into::into);
        let guard = self.guard.clone();
        async move {
            let deadline = guard.admit().ok_or_else(refused)?;
            let response = tokio::time::timeout_at(deadline, self.inner.send_streaming(req))
                .await
                .map_err(|_| guard.trip(DURATION))??;
            let mut remaining = guard.output_bytes.load(Ordering::SeqCst);
            Ok(response.map(|mut body| -> http_client::BoxedStream {
                let mut timer = Box::pin(tokio::time::sleep_until(deadline));
                let mut ended = false;
                Box::pin(futures::stream::poll_fn(move |cx| {
                    if ended {
                        return Poll::Ready(None);
                    }
                    // The timer wakes a reader waiting on a busy or silent body alike.
                    if timer.as_mut().poll(cx).is_ready() {
                        ended = true;
                        return Poll::Ready(Some(Err(guard.trip(DURATION))));
                    }
                    match body.poll_next_unpin(cx) {
                        Poll::Ready(Some(Ok(chunk))) => {
                            match remaining.checked_sub(chunk.len() as u64) {
                                Some(left) => {
                                    remaining = left;
                                    Poll::Ready(Some(Ok(chunk)))
                                }
                                // The chunk past the limit is never passed on.
                                None => {
                                    ended = true;
                                    Poll::Ready(Some(Err(guard.trip(OUTPUT))))
                                }
                            }
                        }
                        other => other,
                    }
                }))
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chunks of `chunk` bytes every `every`, forever: a continuously active body.
    #[derive(Clone)]
    struct Endless {
        chunk: usize,
        every: Duration,
        sent: Arc<AtomicU64>,
    }

    impl HttpClientExt for Endless {
        fn send<T, U>(
            &self,
            _: Request<T>,
        ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
        where
            T: Into<Bytes> + WasmCompatSend,
            U: From<Bytes> + WasmCompatSend + 'static,
        {
            let every = self.every;
            async move {
                let body: LazyBody<U> = Box::pin(async move {
                    tokio::time::sleep(every * 1000).await;
                    Ok(U::from(Bytes::new()))
                });
                Ok(Response::new(body))
            }
        }
        fn send_multipart<U>(
            &self,
            _: Request<MultipartForm>,
        ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
        where
            U: From<Bytes> + WasmCompatSend + 'static,
        {
            std::future::ready(Err(refused()))
        }
        fn send_streaming<T>(
            &self,
            _: Request<T>,
        ) -> impl Future<Output = http_client::Result<StreamingResponse>> + WasmCompatSend
        where
            T: Into<Bytes> + WasmCompatSend,
        {
            self.sent.fetch_add(1, Ordering::SeqCst);
            let (chunk, every) = (self.chunk, self.every);
            let body: http_client::BoxedStream =
                Box::pin(futures::stream::unfold((), move |()| async move {
                    tokio::time::sleep(every).await;
                    Some((Ok(Bytes::from(vec![b'x'; chunk])), ()))
                }));
            std::future::ready(Ok(Response::new(body)))
        }
    }

    fn endless(chunk: usize, every: Duration) -> (Endless, Arc<AtomicU64>) {
        let sent = Arc::new(AtomicU64::new(0));
        (
            Endless {
                chunk,
                every,
                sent: sent.clone(),
            },
            sent,
        )
    }

    fn request() -> Request<Bytes> {
        Request::builder()
            .uri("https://synthetic.invalid/responses")
            .body(Bytes::new())
            .unwrap()
    }

    async fn drain(body: &mut http_client::BoxedStream) -> (u64, bool) {
        let mut received = 0;
        while let Some(chunk) = body.next().await {
            match chunk {
                Ok(chunk) => received += chunk.len() as u64,
                Err(_) => return (received, true),
            }
        }
        (received, false)
    }

    fn code(guard: &ResponseLimit) -> Option<(AiErrorKind, String)> {
        guard
            .failure()
            .map(|error| (error.kind, error.detail.unwrap().code.unwrap()))
    }

    #[tokio::test]
    async fn continuously_active_body_ends_at_the_duration_and_refuses_resend() {
        let guard = Arc::new(ResponseLimit::new(Duration::from_millis(200), u64::MAX));
        let (inner, sent) = endless(16, Duration::from_millis(5));
        let http = guard.wrap(DynHttpClient::new(inner));
        let started = std::time::Instant::now();
        let mut body = http.send_streaming(request()).await.unwrap().into_body();
        let (received, failed) = drain(&mut body).await;
        assert!(failed && received > 0);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(
            code(&guard),
            Some((
                AiErrorKind::TimeLimitReached,
                "brn_response_duration_exceeded".into()
            ))
        );
        assert!(http.send_streaming(request()).await.is_err());
        assert_eq!(sent.load(Ordering::SeqCst), 1, "never sent again");
    }

    #[tokio::test]
    async fn the_chunk_past_the_output_limit_is_never_passed_on() {
        let guard = Arc::new(ResponseLimit::new(Duration::from_secs(60), 1000));
        let (inner, _) = endless(300, Duration::from_millis(1));
        let http = guard.wrap(DynHttpClient::new(inner));
        let mut body = http.send_streaming(request()).await.unwrap().into_body();
        assert_eq!(drain(&mut body).await, (900, true));
        assert!(body.next().await.is_none(), "an ended body stays ended");
        assert_eq!(
            code(&guard),
            Some((AiErrorKind::Other, "brn_response_output_exceeded".into()))
        );
    }

    #[tokio::test]
    async fn a_whole_body_reply_is_held_to_the_duration() {
        let guard = Arc::new(ResponseLimit::new(Duration::from_millis(100), u64::MAX));
        let (inner, _) = endless(1, Duration::from_millis(5));
        let http = guard.wrap(DynHttpClient::new(inner));
        let response = http.send::<_, Vec<u8>>(request()).await.unwrap();
        assert!(response.into_body().await.is_err());
        assert_eq!(code(&guard).unwrap().0, AiErrorKind::TimeLimitReached);
    }

    #[tokio::test]
    async fn responses_within_both_limits_pass_unchanged() {
        let guard = Arc::new(ResponseLimit::default());
        let http = guard.wrap(DynHttpClient::new(
            rig::test_utils::SequencedHttpClient::new([
                rig::test_utils::MockHttpResponse::success_typed(
                    "data: synthetic\n\n",
                    "text/event-stream",
                ),
            ]),
        ));
        let mut body = http.send_streaming(request()).await.unwrap().into_body();
        assert_eq!(drain(&mut body).await, (17, false));
        assert!(guard.failure().is_none());
    }
}
