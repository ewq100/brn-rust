//! One no-tool frozen-packet judgment using the normal Codex subscription route.
//! The experiment supervisor owns persistent reservations and the outer deadline.
use super::{TraceLevel, trace::TraceCollector, wire_trace};
use crate::auth::OwnedClient;
use crate::{AiError, AiErrorKind, Provider, Selection};
use futures::StreamExt;
use rig::completion::{CompletionRequest, FinishReason};
use rig::{DynModel, operation::Completion};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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
pub async fn judge(
    auth_file: &Path,
    packet: &str,
    instructions: &str,
    timeout_seconds: u64,
) -> Value {
    judge_over(
        auth_file,
        packet,
        instructions,
        timeout_seconds,
        rig::http_client::DynHttpClient::new(rig::rig_reqwest::shared()),
    )
    .await
}

async fn judge_over(
    auth_file: &Path,
    packet: &str,
    instructions: &str,
    timeout_seconds: u64,
    base_http: rig::http_client::DynHttpClient,
) -> Value {
    let started = Instant::now();
    let cancel = CancellationToken::new();
    let budget = Arc::new(wire_trace::CallBudget::default());
    budget.set(1).expect("fixed valid budget");
    let trace = TraceCollector::new(TraceLevel::Summary).map(Arc::new);
    let http = crate::response_limit::ResponseLimit::default();
    let http = Arc::new(http).wrap(wire_trace::wrap(base_http, &trace, &budget));
    let operation = async {
        if !(1..=600).contains(&timeout_seconds)
            || packet.len() > 1024 * 1024
            || instructions.len() > 64 * 1024
        {
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
        if returned != Some("gpt-6.1-sol") {
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
    let result = tokio::time::timeout(Duration::from_secs(timeout_seconds), operation).await;
    cancel.cancel();
    let mut receipt = json!({"format":1,"kind":"frozen-packet-judgment",
        "packet_file_sha256":format!("{:x}", Sha256::digest(packet.as_bytes())),
        "instructions_sha256":format!("{:x}", Sha256::digest(instructions.as_bytes())),
        "provider":"Codex","model":"gpt-6.1-sol","effort":"high","tools":[],"timeout_seconds":timeout_seconds,
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

    #[cfg(unix)]
    #[tokio::test]
    async fn frozen_judgment_uses_one_real_wire_request_and_refuses_tools_or_other_model() {
        use crate::provider_formats_tests::{success, text_sse, tool_sse};
        use rig::http_client::DynHttpClient;
        use rig::test_utils::SequencedHttpClient;
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let file = root.path().join("auth.json");
        let bytes = br#"{"auth_mode":"chatgpt","tokens":{"access_token":"SYNTHETIC_ACCESS","account_id":"SYNTHETIC_ACCOUNT"}}"#;
        std::fs::write(&file, bytes).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        let replies = [
            (
                text_sse(true, "[]")
                    .replace("\"model\":\"synthetic\"", "\"model\":\"gpt-6.1-sol\""),
                "completed",
            ),
            (text_sse(true, "[]"), "failed"),
            (
                text_sse(true, "[]").replace("\"model\":\"synthetic\"", "\"model\":null"),
                "failed",
            ),
            (
                tool_sse(true, &[("read_note", json!({"id":"synthetic"}))]),
                "failed",
            ),
        ];
        for (sse, expected) in replies {
            let http = SequencedHttpClient::new([success(sse)]);
            let result = judge_over(
                &file,
                "SYNTHETIC_PACKET",
                "Frozen instructions",
                120,
                DynHttpClient::new(http.clone()),
            )
            .await;
            assert_eq!(result["status"], expected, "{result}");
            assert_eq!(http.requests().len(), 1);
            assert_eq!(
                result["packet_file_sha256"],
                format!("{:x}", Sha256::digest(b"SYNTHETIC_PACKET"))
            );
            assert_eq!(
                result["instructions_sha256"],
                format!("{:x}", Sha256::digest(b"Frozen instructions"))
            );
            assert_eq!(result["model_call_budget"]["started"], 1);
            assert_eq!(result["wire_requests"][0]["model"], "gpt-6.1-sol");
            assert_eq!(result["wire_requests"][0]["effort"], "high");
            assert!(!result.to_string().contains("SYNTHETIC_ACCESS"));
            assert_eq!(std::fs::read(&file).unwrap(), bytes);
        }
        let timeout = judge_over(
            &file,
            "SYNTHETIC_PACKET",
            "Frozen instructions",
            1,
            DynHttpClient::new(rig::test_utils::SilentStreamingClient),
        )
        .await;
        assert_eq!(timeout["status"], "timeout");
        assert_eq!(timeout["model_call_budget"]["started"], 1);
        assert!(timeout.get("result").is_none());
    }
}
