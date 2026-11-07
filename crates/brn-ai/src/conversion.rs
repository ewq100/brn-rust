//! Host-owned, tool-free document conversion transport. Source content never supplies authority.
use crate::auth::{AccountPin, OwnedClient};
use crate::chat::collect_stream;
use crate::provider::{ProviderOperation, copilot_completion};
use crate::{AiError, AiErrorKind, AiTerminal, Provider, ReasoningEffort, Selection};
use brn_threads_intake::ConversionError;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{FinishReason, Message, PromptError};
use rig::http_client::DynHttpClient;
use std::{
    path::{Path, PathBuf},
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

/// The account that refines documents, fixed when the job starts. Each
/// provider uses only its own sign-in: Copilot never falls back to a Codex
/// login, and a changed Copilot account refuses rather than serves the job.
#[derive(Clone, Debug)]
pub enum ConversionAccount {
    Chatgpt {
        auth_file: Option<PathBuf>,
        credentials_dir: Option<PathBuf>,
    },
    Copilot {
        credentials_dir: Option<PathBuf>,
        account: Option<AccountPin>,
    },
    /// The saved provider cannot refine documents.
    Unsupported,
}
impl ConversionAccount {
    /// Captures the selected account now. Reads only the selected provider's
    /// own credentials; call it only when refinement is On.
    pub fn capture(
        provider: Option<Provider>,
        auth_file: Option<&Path>,
        credentials_dir: Option<&Path>,
    ) -> Self {
        match provider {
            Some(Provider::Chatgpt) => Self::Chatgpt {
                auth_file: auth_file.map(Path::to_owned),
                credentials_dir: credentials_dir.map(Path::to_owned),
            },
            Some(Provider::Copilot) => Self::Copilot {
                account: Provider::Copilot.capture_account(credentials_dir),
                credentials_dir: credentials_dir.map(Path::to_owned),
            },
            None => Self::Unsupported,
        }
    }
}

/// The selected account's client for the fixed refinement model.
async fn conversion_client(
    account: &ConversionAccount,
    transport: Option<DynHttpClient>,
    cancel: &CancellationToken,
) -> Result<rig_core::DynModel<rig_core::operation::Completion>, AiError> {
    let (provider, auth_file, credentials_dir, pin) = match account {
        ConversionAccount::Chatgpt {
            auth_file,
            credentials_dir,
        } => (
            Provider::Chatgpt,
            auth_file.as_deref(),
            credentials_dir.as_deref(),
            None,
        ),
        ConversionAccount::Copilot {
            credentials_dir: Some(dir),
            account: Some(pin),
        } => (Provider::Copilot, None, Some(dir.as_path()), Some(pin)),
        ConversionAccount::Copilot { .. } | ConversionAccount::Unsupported => {
            return Err(AiError::new(AiErrorKind::ReconnectNeeded));
        }
    };
    let selection = Selection {
        provider,
        model: MODEL.into(),
    };
    let (client, _) = ProviderOperation {
        selection: &selection,
        auth_file,
        credentials_dir,
        account: pin,
        cache_key: None,
    }
    .client(transport, cancel)
    .await?;
    match (provider, client.inner) {
        (Provider::Chatgpt, OwnedClient::Chatgpt(client)) => Ok(client.completion(MODEL).into()),
        (Provider::Copilot, OwnedClient::Copilot(client)) => {
            // Copilot serves GPT-6 Luna on Responses only; no run resend/fallback.
            Ok(copilot_completion(&client, MODEL.into(), true).into())
        }
        _ => Err(AiError::new(AiErrorKind::ModelRefused)),
    }
}

/// The same fixed account/model stream path, with response checking inside its
/// deadline. The parser supplies no tools, history or write authority.
pub(crate) async fn conversion_response<T>(
    message: Message,
    preamble: &str,
    account: &ConversionAccount,
    cancel: CancellationToken,
    transport: Option<DynHttpClient>,
    timeout: Duration,
    parse: fn(&str) -> Result<T, ConversionFailure>,
) -> Result<T, ConversionFailure> {
    if cancel.is_cancelled() {
        return Err(ConversionFailure::Interrupted);
    }
    let operation = async {
        let model = conversion_client(account, transport, &cancel)
            .await
            .map_err(ConversionFailure::Provider)?;
        let agent = rig::AgentBuilder::new(model)
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
