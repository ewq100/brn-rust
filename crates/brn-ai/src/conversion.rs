//! Host-owned, tool-free document conversion transport. Source content never supplies authority.
use crate::auth::OwnedClient;
use crate::chat::collect_stream;
use crate::{AiError, AiErrorKind, AiTerminal, Auth, Provider, ReasoningEffort, Selection};
use brn_threads_intake::ConversionError;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{FinishReason, Message, PromptError};
use rig::http_client::DynHttpClient;
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

pub const MODEL: &str = "gpt-6-luna";
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// Deliberately payload-free: source wording, paths and provider bodies stay transient.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionProgress {
    Preparing,
    Converting,
    Checking,
}
impl ConversionProgress {
    pub fn label(self) -> &'static str {
        match self {
            Self::Preparing => "Reading the document…",
            Self::Converting => "Converting with GPT-6 Luna (high)…",
            Self::Checking => "Checking converted content and figures…",
        }
    }
}

#[derive(Debug)]
pub enum ConversionFailure {
    Intake(ConversionError),
    Provider(AiError),
    Interrupted,
    Timeout,
    InvalidCandidate,
    Budget,
    Worker,
}
impl std::fmt::Display for ConversionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Intake(e) => write!(f, "Document conversion failed: {e}"),
            Self::Provider(e) => write!(f, "GPT-6 Luna conversion failed: {e}"),
            Self::Interrupted => f.write_str("Document conversion was stopped."),
            Self::Timeout => f.write_str("GPT-6 Luna conversion exceeded its time limit."),
            Self::InvalidCandidate => {
                f.write_str("GPT-6 Luna returned an invalid conversion candidate.")
            }
            Self::Budget => {
                f.write_str("The document exceeds the bounded GPT-6 Luna conversion profile.")
            }
            Self::Worker => f.write_str("Document conversion worker did not complete."),
        }
    }
}
impl std::error::Error for ConversionFailure {}
impl From<ConversionError> for ConversionFailure {
    fn from(error: ConversionError) -> Self {
        if error == ConversionError::Cancelled {
            Self::Interrupted
        } else {
            Self::Intake(error)
        }
    }
}

/// The same fixed account/model stream path, with response checking inside its
/// deadline. The parser supplies no tools, history or write authority.
pub(crate) async fn conversion_response<T>(
    message: Message,
    preamble: &str,
    credentials: (Option<&std::path::Path>, Option<&std::path::Path>),
    cancel: CancellationToken,
    transport: Option<DynHttpClient>,
    timeout: Duration,
    parse: fn(&str) -> Result<T, ConversionFailure>,
) -> Result<T, ConversionFailure> {
    if cancel.is_cancelled() {
        return Err(ConversionFailure::Interrupted);
    }
    let (auth_file, credentials_dir) = credentials;
    let operation = async {
        let selection = Selection {
            provider: Provider::Chatgpt,
            model: MODEL.into(),
        };
        let client = if let Some(file) = auth_file {
            crate::codex_auth::codex_client_pinned(
                file,
                &selection,
                None,
                transport.unwrap_or_else(|| DynHttpClient::new(rig::rig_reqwest::shared())),
                cancel.clone(),
            )
            .await
        } else if let Some(dir) = credentials_dir {
            let auth = Auth::open(dir).map_err(ConversionFailure::Provider)?;
            let auth = match transport {
                Some(http) => auth.with_http(http),
                None => auth,
            };
            auth.client(&selection, cancel.clone()).await
        } else {
            Err(AiError::new(AiErrorKind::ReconnectNeeded))
        }
        .map_err(ConversionFailure::Provider)?;
        let OwnedClient::Chatgpt(client) = client.inner else {
            return Err(ConversionFailure::Provider(AiError::new(
                AiErrorKind::ModelRefused,
            )));
        };
        let agent = rig::AgentBuilder::new(client.completion(MODEL))
            .preamble(preamble)
            // Rig's qualified subscription route rejects generation-token caps.
            // Bound candidate bytes, model calls and wall time in the host instead.
            .options(
                rig::completion::GenerationOptions::new().reasoning(ReasoningEffort::High.typed()),
            )
            .accept_unknown_finish_reasons(false)
            .build();
        let stream = agent
            .prompt(message)
            .max_turns(1)
            .max_invalid_tool_call_retries(0)
            .max_consecutive_malformed_tool_calls(0)
            .unhandled_invalid_tool_call(rig::run::UnhandledInvalidToolCall::Fail)
            .stream()
            .map(require_natural_finish);
        let answer = collect_stream(
            Box::pin(stream),
            cancel.clone(),
            Arc::new(|_| {}),
            Arc::new(AtomicBool::new(false)),
            Some(MAX_OUTPUT_BYTES),
        )
        .await;
        match answer.terminal {
            AiTerminal::Completed => parse(&answer.text),
            AiTerminal::Interrupted => Err(ConversionFailure::Interrupted),
            AiTerminal::Failed(error) => Err(ConversionFailure::Provider(error)),
        }
    };
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(ConversionFailure::Interrupted),
        result = tokio::time::timeout(timeout, operation) => {
            result.map_err(|_| ConversionFailure::Timeout)?
        }
    }
}

#[allow(clippy::result_large_err)]
fn require_natural_finish(
    item: Result<MultiTurnStreamItem, PromptError>,
) -> Result<MultiTurnStreamItem, PromptError> {
    match item {
        Ok(MultiTurnStreamItem::FinalResponse(ref response))
            if !response
                .completion_calls
                .last()
                .is_some_and(|call| call.finish_reason.as_ref() == Some(&FinishReason::Stop)) =>
        {
            Err(PromptError::Provider(rig::error::ProviderError::Response(
                "conversion response did not finish naturally".into(),
            )))
        }
        item => item,
    }
}
