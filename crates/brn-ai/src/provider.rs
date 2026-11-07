//! Existing provider policy shared by distinct model operations.
//!
//! This module owns saved-setting normalization, account binding and client
//! construction, and BRN's Copilot wire/fallback rules. Operations still own
//! their transports, limits, tools, cancellation and checked effects. In
//! particular, constructing a client never adds the run resend decorator.
use crate::{
    AccountPin, AiAnswer, AiError, AiErrorKind, AiResult, AiTerminal, Auth, Provider,
    ProviderClient, ReasoningEffort, Selection,
};
use rig::http_client::DynHttpClient;
use rig::providers::{copilot, openai};
use std::path::Path;
use tokio_util::sync::CancellationToken;

impl Provider {
    /// The provider a saved setting names ("Codex"/"ChatGPT" or "Copilot").
    pub fn from_setting(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
            "codex" | "chatgpt" => Some(Self::Chatgpt),
            "copilot" => Some(Self::Copilot),
            _ => None,
        }
    }
}

impl Provider {
    /// Capture only this provider's Copilot account at operation start. `None`
    /// never adopts a later sign-in. Other providers retain their existing
    /// auth-file/client contracts; this is not a universal account snapshot.
    pub fn capture_account(self, credentials_dir: Option<&Path>) -> Option<AccountPin> {
        if self != Self::Copilot {
            return None;
        }
        Auth::open(credentials_dir?).ok()?.copilot_account().ok()
    }
}
impl ReasoningEffort {
    /// Saved effort names are deliberately case-sensitive, as before.
    pub fn from_setting(text: &str) -> Option<Self> {
        match text {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }
}

/// One operation's explicit selection and credential references, never raw
/// credentials. Capture `account` when the operation starts, before spawning
/// work. Summaries reuse their parent's already constructed client instead.
pub(crate) struct ProviderOperation<'a> {
    pub selection: &'a Selection,
    pub auth_file: Option<&'a Path>,
    pub credentials_dir: Option<&'a Path>,
    pub account: Option<&'a AccountPin>,
    pub cache_key: Option<&'a str>,
}
impl ProviderOperation<'_> {
    /// Authenticate only the selected route and check the captured Copilot
    /// account both before and after authentication. The optional bootstrap
    /// digest keeps the existing summary-cache identity; it is not a secret.
    pub async fn client(
        self,
        transport: Option<DynHttpClient>,
        cancel: &CancellationToken,
    ) -> AiResult<(ProviderClient, Option<String>)> {
        let Self {
            selection,
            auth_file,
            credentials_dir,
            account,
            cache_key,
        } = self;
        let http = transport.unwrap_or_else(|| DynHttpClient::new(rig::rig_reqwest::shared()));
        // A Codex auth file only serves ChatGPT; Copilot always uses its credentials folder.
        let codex_file = auth_file.filter(|_| selection.provider == Provider::Chatgpt);
        if let Some(file) = codex_file {
            crate::codex_auth::codex_client_pinned(file, selection, cache_key, http, cancel.clone())
                .await
                .map(|client| (client, None))
        } else if let Some(dir) = credentials_dir {
            let auth = Auth::open(dir)?.with_http(http);
            if selection.provider != Provider::Copilot {
                let client = auth
                    .client_pinned(selection, cache_key, cancel.clone())
                    .await?;
                return Ok((client, None));
            }
            // No account captured at start (signed out or unreadable then) never
            // adopts a sign-in made since.
            let started = account
                .cloned()
                .ok_or_else(|| AiError::new(AiErrorKind::ReconnectNeeded))?;
            auth.check_copilot_account(&started)?;
            let client = auth
                .client_pinned(selection, cache_key, cancel.clone())
                .await?;
            auth.check_copilot_account(&started)?;
            let fingerprint = started.fingerprint().map(str::to_owned);
            Ok((client, fingerprint))
        } else {
            Err(AiError::new(AiErrorKind::ReconnectNeeded))
        }
    }
}

/// Use Rig's existing conversation route choice, not catalogue presence or
/// entitlement. Search and refinement retain their Responses-only contracts.
pub(crate) fn copilot_conversation_uses_responses(model: &str) -> bool {
    copilot::wire::routes_through_responses(model)
}

/// The Copilot completion model, forced onto the Responses route when asked.
/// Rig 0.46 selects that route only for Codex IDs, yet Copilot serves some
/// other models there exclusively.
///
/// Tools are sent non-strict on both routes, as on ChatGPT. Rig's Copilot
/// Responses default sanitizes schemas for strict mode, which turns an open
/// object such as `prepare_changes` record `data` into `{}` the model cannot
/// fill. BRN validates every tool call's arguments itself.
pub(crate) fn copilot_completion(
    client: &openai::OpenAI,
    model: String,
    responses: bool,
) -> rig_core::driver::Model<openai::wire::OpenAiWire> {
    let mut completion = client.completion(model.clone());
    if responses {
        let config = client.config();
        let provider =
            openai::OpenAIConfig::with_key(&copilot::wire::DIALECT, config.api_key.clone())
                .with_base_url(config.base_url.clone())
                .with_route(openai::wire::Route::Responses);
        completion.wire = openai::wire::OpenAiWire::new(provider, model);
    }
    if let openai::wire::OpenAiWire::Responses(wire) = &mut completion.wire {
        wire.strict_tools = false;
    }
    completion
}

/// One empty Chat refusal may retry the SAME Copilot model on Responses.
/// Shared by parent/helper chat and the tool-free semantic compactor.
pub(crate) fn copilot_responses_retry(
    responses: bool,
    answer: &AiAnswer,
    cancel: &CancellationToken,
) -> bool {
    !responses
        && answer.text.is_empty()
        && matches!(&answer.terminal,AiTerminal::Failed(error) if error.kind==AiErrorKind::ModelRefused)
        && !cancel.is_cancelled()
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
