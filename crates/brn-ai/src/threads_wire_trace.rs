//! Passive, opt-in wire identity observation. Never retain headers or prompt bytes.
use super::trace::TraceCollector;
use rig::http_client::{
    self, DynHttpClient, HttpClientExt, LazyBody, MultipartForm, Request, Response,
    StreamingResponse,
};
use rig_core::wasm_compat::WasmCompatSend;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use tokio_util::bytes::Bytes;

#[derive(Default)]
pub(super) struct CallBudget {
    limit: AtomicU64,
    remaining: AtomicU64,
    refused: AtomicBool,
}
impl CallBudget {
    pub(super) fn set(&self, limit: u64) -> crate::AiResult<()> {
        if !(1..=10000).contains(&limit) || self.limit.load(Ordering::SeqCst) != 0 {
            return Err(crate::AiError::new(crate::AiErrorKind::ToolRejected));
        }
        self.remaining.store(limit, Ordering::SeqCst);
        self.limit.store(limit, Ordering::SeqCst);
        Ok(())
    }
    fn reserve(&self) -> bool {
        if self.limit.load(Ordering::SeqCst) == 0 {
            return true;
        }
        if self
            .remaining
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok()
        {
            true
        } else {
            self.refused.store(true, Ordering::SeqCst);
            false
        }
    }
    pub(super) fn refused(&self) -> bool {
        self.refused.load(Ordering::SeqCst)
    }
    pub(super) fn snapshot(&self) -> Option<serde_json::Value> {
        let limit = self.limit.load(Ordering::SeqCst);
        (limit > 0).then(|| {
            serde_json::json!({"limit":limit,
            "started":limit - self.remaining.load(Ordering::SeqCst),
            "refused":self.refused.load(Ordering::SeqCst)})
        })
    }
}

struct ObservedHttp {
    inner: DynHttpClient,
    trace: Option<Arc<TraceCollector>>,
    budget: Arc<CallBudget>,
}

pub(super) fn wrap(
    inner: DynHttpClient,
    trace: &Option<Arc<TraceCollector>>,
    budget: &Arc<CallBudget>,
) -> DynHttpClient {
    DynHttpClient::new(ObservedHttp {
        inner,
        trace: trace.clone(),
        budget: budget.clone(),
    })
}

impl ObservedHttp {
    fn observe(&self, request: &Request<Bytes>) -> std::io::Result<()> {
        let api = match request.uri().path().trim_end_matches('/') {
            path if path.ends_with("/responses") => "responses",
            path if path.ends_with("/chat/completions") => "chat_completions",
            _ => {
                // A future model endpoint must not silently escape an enabled cap.
                let body: serde_json::Value =
                    serde_json::from_slice(request.body()).unwrap_or_default();
                if self.budget.limit.load(Ordering::SeqCst) > 0 && body.get("model").is_some() {
                    self.budget.refused.store(true, Ordering::SeqCst);
                    return Err(std::io::Error::other(
                        "unsupported model endpoint under call limit",
                    ));
                }
                return Ok(());
            }
        };
        if !self.budget.reserve() {
            if let Some(trace) = &self.trace {
                trace.stop("model_call_budget");
            }
            return Err(std::io::Error::other(
                "model-call limit reached before dispatch",
            ));
        }
        let Some(trace) = &self.trace else {
            return Ok(());
        };
        let body: serde_json::Value = serde_json::from_slice(request.body()).unwrap_or_default();
        let model = body
            .get("model")
            .and_then(|v| v.as_str())
            .filter(|s| crate::valid_model_id(s));
        let effort = body
            .pointer("/reasoning/effort")
            .or_else(|| body.get("reasoning_effort"))
            .and_then(|v| v.as_str())
            .filter(|s| matches!(*s, "low" | "medium" | "high"));
        trace.wire_request(api, model, effort);
        Ok(())
    }
}

impl HttpClientExt for ObservedHttp {
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        T: Into<Bytes> + WasmCompatSend,
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        let req = req.map(Into::into);
        let admitted = self.observe(&req);
        let inner = self.inner.clone();
        async move {
            admitted.map_err(|error| http_client::Error::Instance(Box::new(error)))?;
            inner.send(req).await
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
        let req = req.map(Into::into);
        let admitted = self.observe(&req);
        async move {
            admitted.map_err(|error| http_client::Error::Instance(Box::new(error)))?;
            self.inner.send_streaming(req).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::threads::TraceLevel;
    use rig::test_utils::SequencedHttpClient;
    #[test]
    fn records_only_bounded_identity_and_never_auth_or_prompt() {
        let trace = Arc::new(TraceCollector::new(TraceLevel::Full).unwrap());
        let client = ObservedHttp {
            inner: DynHttpClient::new(SequencedHttpClient::new([])),
            trace: Some(trace.clone()),
            budget: Arc::new(CallBudget::default()),
        };
        for (path, body) in [
            (
                "/responses",
                r#"{"model":"gpt-test","reasoning":{"effort":"high"},"input":"SECRET-PROMPT"}"#,
            ),
            (
                "/chat/completions",
                r#"{"model":"gpt-test","reasoning_effort":"low"}"#,
            ),
            ("/token", r#"{"model":"AUTH-SECRET"}"#),
        ] {
            let request = Request::builder()
                .uri(format!("https://synthetic.invalid{path}"))
                .header("authorization", "SECRET-TOKEN")
                .body(Bytes::from(body))
                .unwrap();
            client.observe(&request).unwrap();
        }
        let result = trace.finish("test", false, 8);
        assert_eq!(result.wire_requests.len(), 2);
        assert_eq!(result.wire_requests[0].model.as_deref(), Some("gpt-test"));
        assert_eq!(result.wire_requests[1].effort.as_deref(), Some("low"));
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains("SECRET"));
        assert!(!text.contains("synthetic.invalid"));
    }
    #[test]
    fn shared_budget_is_atomic_and_never_refunds_uncertain_requests() {
        let budget = Arc::new(CallBudget::default());
        budget.set(3).unwrap();
        let handles: Vec<_> = (0..16)
            .map(|_| {
                let budget = budget.clone();
                std::thread::spawn(move || budget.reserve())
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .map(|h| usize::from(h.join().unwrap()))
                .sum::<usize>(),
            3
        );
        assert!(budget.refused());
        let other = Arc::new(CallBudget::default());
        other.set(2).unwrap();
        assert!(other.reserve());
        assert!(other.reserve());
        assert!(!other.reserve());
        assert_eq!(other.snapshot().unwrap()["started"], 2);
        assert!(other.refused());
        assert!(other.set(3).is_err());
    }
}
