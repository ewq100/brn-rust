//! Single-image transport and provisional interpretation; workflow owns evidence and authority.

use crate::auth::OwnedClient;
use crate::behavior::AgentBehavior;
use crate::chat::collect_stream;
use crate::{
    AiAnswer, AiError, AiErrorKind, AiEvent, AiResult, AiTerminal, Provider, ProviderClient,
    ReasoningEffort,
};
use base64::Engine as _;
use futures::StreamExt;
use rig::agent::{MultiTurnStreamItem, StreamingError};
use rig::completion::{FinishReason, Message};
use rig::message::{ImageDetail, ImageMediaType, UserContent};
use std::sync::{Arc, atomic::AtomicBool};
use tokio_util::sync::CancellationToken;

const MAX_IMAGE_BYTES: usize = 1024 * 1024;
const MAX_PROMPT_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 16 * 1024;
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Owned opaque PNG transport. This checks only the byte bound and signature,
/// never complete PNG integrity, decoded dimensions, provenance or freshness.
/// Workflow must validate that evidence before constructing this input.
// Deliberately no Debug: image payloads must not be dumped in diagnostics.
#[derive(Clone, PartialEq, Eq)]
pub struct VisualImage(Vec<u8>);

impl VisualImage {
    /// Private extraction images share the helper's aggregate 16 MiB budget.
    /// Complete decoding and evidence checks occur before this transport layer.
    pub fn png_evidence(bytes: Vec<u8>) -> AiResult<Self> {
        if bytes.len() > 16 * 1024 * 1024 || !bytes.starts_with(PNG_SIGNATURE) {
            return Err(AiError::new(AiErrorKind::ToolRejected));
        }
        Ok(Self(bytes))
    }
    pub fn png(bytes: Vec<u8>) -> AiResult<Self> {
        if bytes.len() > MAX_IMAGE_BYTES || !bytes.starts_with(PNG_SIGNATURE) {
            return Err(AiError::new(AiErrorKind::ToolRejected));
        }
        Ok(Self(bytes))
    }

    pub fn png_bytes(&self) -> &[u8] {
        &self.0
    }
}

pub(crate) fn evidence_message(prompt: &str, images: &[VisualImage]) -> AiResult<Message> {
    if images.len() > 32
        || images
            .iter()
            .try_fold(0usize, |total, image| {
                total.checked_add(image.png_bytes().len())
            })
            .is_none_or(|total| total > 16 * 1024 * 1024)
    {
        return Err(AiError::new(AiErrorKind::ToolRejected));
    }
    let mut content = vec![UserContent::text(prompt)];
    for image in images {
        content.push(UserContent::image_base64(
            base64::engine::general_purpose::STANDARD.encode(image.png_bytes()),
            Some(ImageMediaType::PNG),
            Some(ImageDetail::High),
        ));
    }
    Ok(Message::User { content })
}

/// Uses the captured provider/model/effort and no tools or history. Completed
/// bounded raw output is provisional: workflow parses it and verifies evidence
/// before constructing a separate proposal requiring exact human approval.
pub async fn interpret_visual(
    client: ProviderClient,
    prompt: &str,
    image: &VisualImage,
    effort: ReasoningEffort,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer {
    if prompt.len() > MAX_PROMPT_BYTES {
        return AiAnswer {
            text: String::new(),
            terminal: AiTerminal::Failed(AiError::new(AiErrorKind::ToolRejected)),
        };
    }
    let selection = client.selection();
    let model = selection.model.clone();
    let responses = selection.provider == Provider::Chatgpt
        || rig::providers::copilot::wire::routes_through_responses(&model);
    let model: rig_core::DynModel<rig_core::operation::Completion> = match client.inner {
        OwnedClient::Chatgpt(client) => client.completion(model).into(),
        OwnedClient::Copilot(client) => client.completion(model).into(),
    };
    let agent = rig::AgentBuilder::new(model)
        .preamble(AgentBehavior::VisualInterpretation.preamble())
        .additional_params(if responses {
            serde_json::json!({"reasoning":{"effort":effort.as_str()}})
        } else {
            serde_json::json!({"reasoning_effort":effort.as_str()})
        })
        .build();
    let input = Message::User {
        content: vec![
            UserContent::text(prompt),
            UserContent::image_base64(
                base64::engine::general_purpose::STANDARD.encode(image.png_bytes()),
                Some(ImageMediaType::PNG),
                Some(ImageDetail::High),
            ),
        ],
    };
    let stream = agent
        .prompt(input)
        .max_turns(1)
        .max_invalid_tool_call_retries(0)
        .stream();
    // Rig accepts some truncated textual turns for ordinary chat. A strict
    // visual response must finish naturally even if its partial JSON parses.
    let stream = stream.map(require_natural_finish);
    collect_stream(
        Box::pin(stream),
        cancel,
        emit,
        Arc::new(AtomicBool::new(false)),
        Some(MAX_OUTPUT_BYTES),
    )
    .await
}

// Rig defines the fixed streaming item/error representation; this adapter must
// return that same contract rather than replace or box its upstream error.
#[allow(clippy::result_large_err)]
fn require_natural_finish(
    item: Result<MultiTurnStreamItem, StreamingError>,
) -> Result<MultiTurnStreamItem, StreamingError> {
    match item {
        Ok(MultiTurnStreamItem::FinalResponse(ref response))
            if !response
                .completion_calls
                .last()
                .is_some_and(|call| call.finish_reason.as_ref() == Some(&FinishReason::Stop)) =>
        {
            Err(StreamingError::Completion(
                rig::error::ProviderError::Response(
                    "visual response did not finish naturally".into(),
                ),
            ))
        }
        item => item,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_transport_refuses_oversized_collection_before_model_admission() {
        let image = VisualImage::png_evidence(PNG_SIGNATURE.to_vec()).unwrap();
        assert!(evidence_message("exact private scope", &vec![image; 33]).is_err());
        let mut bytes = vec![0; 9 * 1024 * 1024];
        bytes[..PNG_SIGNATURE.len()].copy_from_slice(PNG_SIGNATURE);
        let image = VisualImage::png_evidence(bytes).unwrap();
        assert!(evidence_message("exact private scope", &[image.clone(), image]).is_err());
        let mut oversized = vec![0; 16 * 1024 * 1024 + 1];
        oversized[..PNG_SIGNATURE.len()].copy_from_slice(PNG_SIGNATURE);
        assert!(VisualImage::png_evidence(oversized).is_err());
    }

    #[test]
    fn png_transport_checks_only_exact_signature_and_encoded_byte_bound() {
        for bytes in [
            vec![],
            b"PNG".to_vec(),
            b"\x89PNG\r\n\x1a".to_vec(),
            vec![0; 32],
        ] {
            assert!(matches!(
                VisualImage::png(bytes),
                Err(AiError {
                    kind: AiErrorKind::ToolRejected,
                    ..
                })
            ));
        }
        let signature_only = VisualImage::png(PNG_SIGNATURE.to_vec()).unwrap();
        assert_eq!(signature_only.png_bytes(), PNG_SIGNATURE);
        assert!(signature_only == signature_only.clone());
        let mut exact = vec![0; MAX_IMAGE_BYTES];
        exact[..PNG_SIGNATURE.len()].copy_from_slice(PNG_SIGNATURE);
        assert_eq!(VisualImage::png(exact.clone()).unwrap().png_bytes(), exact);
        exact.push(0);
        assert!(VisualImage::png(exact).is_err());
    }
}
