//! One no-tool frozen-packet judgment using the normal Codex subscription route.
//! The experiment supervisor owns persistent reservations and the outer deadline.
use super::{TraceLevel, trace::TraceCollector, wire_trace};
use crate::auth::OwnedClient;
use crate::{AiError, AiErrorKind, Provider, Selection};
use futures::StreamExt;
use rig::completion::{CompletionRequest, FinishReason};
use rig::{DynModel, operation::Completion};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

fn request(packet: &str, instructions: &str) -> CompletionRequest {
    CompletionRequest::new(packet)
        .preamble(instructions)
        .reasoning(rig::completion::Effort::High)
        .accept_unknown_finish_reasons(false)
}

/// Fixed Sol/high, no tools, exactly one admitted HTTP model attempt, no resend.
/// Errors retain the safe error kind and observed request budget, never raw bodies.
pub async fn judge(auth_file: &Path, packet: &str, instructions: &str) -> Value {
    let started = Instant::now();
    let cancel = CancellationToken::new();
    let budget = Arc::new(wire_trace::CallBudget::default());
    budget.set(1).expect("fixed valid budget");
    let trace = TraceCollector::new(TraceLevel::Summary).map(Arc::new);
    let http = crate::response_limit::ResponseLimit::default();
    let http = Arc::new(http).wrap(wire_trace::wrap(
        rig::http_client::DynHttpClient::new(rig::rig_reqwest::shared()),
        &trace,
        &budget,
    ));
    let operation = async {
        if packet.len() > 1024 * 1024 || instructions.len() > 64 * 1024 {
            return Err(AiError::new(AiErrorKind::ToolRejected));
        }
        let selection = Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        };
        let client = crate::codex_auth::codex_client_pinned(
            auth_file,
            &selection,
            None,
            http,
            cancel.clone(),
        )
        .await?;
        let OwnedClient::Chatgpt(client) = client.inner else {
            return Err(AiError::new(AiErrorKind::ToolRejected));
        };
        let model: DynModel<Completion> = client.completion(selection.model).into();
        let mut stream = model
            .stream(request(packet, instructions))
            .map_err(crate::error::map_provider)?;
        while let Some(item) = stream.next().await {
            item.map_err(crate::error::map_provider)?;
        }
        let response = stream.finish().await.map_err(crate::error::map_provider)?;
        if response.tool_calls().next().is_some()
            || response.error.is_some()
            || response.aborted.is_some()
            || !matches!(response.finish_reason(), Some(FinishReason::Stop))
        {
            return Err(AiError::new(AiErrorKind::InvalidToolUse));
        }
        let returned = response.model().filter(|id| crate::valid_model_id(id));
        if returned.is_some_and(|id| id != "gpt-6.1-sol") {
            return Err(AiError::new(AiErrorKind::ModelRefused));
        }
        let text = response.text();
        if text.len() > 256 * 1024 || text.trim().is_empty() {
            return Err(AiError::new(AiErrorKind::Other));
        }
        // Explicit counters only: catalog/API cost estimates are not charges.
        Ok(json!({"text":text,"returned_model":returned,
            "usage":{"input":response.usage.input_tokens,"output":response.usage.output_tokens,
                "cached_input":response.usage.cached_input_tokens,"reasoning":response.usage.reasoning_tokens},
            "money_cost":null}))
    };
    let result = tokio::time::timeout(Duration::from_secs(45), operation).await;
    cancel.cancel();
    let mut receipt = json!({"format":1,"kind":"frozen-packet-judgment",
        "provider":"Codex","model":"gpt-6.1-sol","effort":"high","tools":[],
        "elapsed_ms":started.elapsed().as_millis(),"model_call_budget":budget.snapshot(),
        "wire_requests":trace.map(|t| t.finish("frozen-reviewer", false, 0).wire_requests)});
    match result {
        Ok(Ok(value)) => {
            receipt["status"] = json!("completed");
            receipt["result"] = value;
        }
        Ok(Err(error)) => {
            receipt["status"] = json!("failed");
            receipt["error"] = json!(error);
        }
        Err(_) => {
            receipt["status"] = json!("timeout");
        }
    }
    receipt
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn judgment_has_no_callable_or_provider_tools_and_fixed_effort() {
        let r = request("SYNTHETIC untrusted packet", "Frozen reviewer instructions");
        assert!(r.tools.is_empty());
        assert!(r.additional_params.is_none());
        let encoded = serde_json::to_string(&r).unwrap();
        assert!(encoded.contains("high"));
    }
}
