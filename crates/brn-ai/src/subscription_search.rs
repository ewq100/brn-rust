//! Explicit subscription research entry point.
//!
//! No provider turn is started until the installed App Server can demonstrate
//! the required model-visible capability boundary. In particular, a read-only
//! sandbox or an instruction to avoid tools does not establish that boundary.

use std::fmt;

use tokio_util::sync::CancellationToken;

use crate::{Provider, Selection};

/// The owner-supplied query and the already-selected provider and model.
/// BRN workspace content and conversation history are deliberately absent.
#[derive(Clone, Debug)]
pub struct Request {
    pub selection: Selection,
    pub query: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    pub provider: Provider,
    pub model: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub answer: String,
    pub sources: Vec<Source>,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchError {
    Cancelled,
    InvalidQuery,
    InvalidModel,
    UnsupportedProvider,
    CapabilityBoundaryUnverified,
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Cancelled => "Web search was cancelled.",
            Self::InvalidQuery => "Enter a non-empty web search query of at most 16 KiB.",
            Self::InvalidModel => "The selected model is invalid.",
            Self::UnsupportedProvider => "Web search is unavailable for the selected subscription provider. Select a supported provider; BRN will not switch providers automatically.",
            Self::CapabilityBoundaryUnverified => "ChatGPT web search is unavailable because BRN cannot verify that shell, arbitrary file access, connected apps, MCP servers and skills are disabled for this App Server version. No search was sent.",
        })
    }
}

impl std::error::Error for SearchError {}

/// Fails closed without launching a process or submitting the query. This is
/// intentionally immediate, so no provider timeout or process cleanup applies.
/// Ordinary provider-account instructions may apply once a safe route exists.
pub async fn search(
    request: Request,
    cancel: CancellationToken,
) -> Result<SearchResult, SearchError> {
    if cancel.is_cancelled() {
        return Err(SearchError::Cancelled);
    }
    if request.query.trim().is_empty() || request.query.len() > 16 * 1024 {
        return Err(SearchError::InvalidQuery);
    }
    if request.selection.provider != Provider::Chatgpt {
        return Err(SearchError::UnsupportedProvider);
    }
    request
        .selection
        .validate()
        .map_err(|_| SearchError::InvalidModel)?;
    Err(SearchError::CapabilityBoundaryUnverified)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(provider: Provider, query: &str) -> Request {
        Request {
            selection: Selection {
                provider,
                model: "model".into(),
            },
            query: query.into(),
        }
    }

    #[tokio::test]
    async fn blocks_unverified_chatgpt_without_a_turn() {
        assert_eq!(
            search(
                request(Provider::Chatgpt, "public query"),
                CancellationToken::new()
            )
            .await,
            Err(SearchError::CapabilityBoundaryUnverified)
        );
    }

    #[tokio::test]
    async fn rejects_unsupported_provider_and_invalid_inputs() {
        assert_eq!(
            search(
                request(Provider::Copilot, "query"),
                CancellationToken::new()
            )
            .await,
            Err(SearchError::UnsupportedProvider)
        );
        assert_eq!(
            search(request(Provider::Chatgpt, "  "), CancellationToken::new()).await,
            Err(SearchError::InvalidQuery)
        );
        assert_eq!(
            search(
                request(Provider::Chatgpt, &"a".repeat(16 * 1024 + 1)),
                CancellationToken::new()
            )
            .await,
            Err(SearchError::InvalidQuery)
        );
        let mut invalid = request(Provider::Chatgpt, "query");
        invalid.selection.model.clear();
        assert_eq!(
            search(invalid, CancellationToken::new()).await,
            Err(SearchError::InvalidModel)
        );
    }

    #[tokio::test]
    async fn honours_cancellation_before_work() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            search(request(Provider::Chatgpt, "query"), cancel).await,
            Err(SearchError::Cancelled)
        );
    }
}
