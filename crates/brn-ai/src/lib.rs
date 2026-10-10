#[cfg(any(test, feature = "capability-spike"))]
mod action_candidates;
mod auth;
#[cfg(any(test, feature = "capability-spike"))]
mod behavior;
#[cfg(any(test, feature = "capability-spike"))]
pub mod capability_probe;
mod chat;
pub mod codex_auth;
mod error;
#[cfg(any(test, feature = "capability-spike"))]
mod proposal_tools;
#[cfg(any(test, feature = "capability-spike"))]
mod provider_fixture_runtime;
#[cfg(test)]
mod provider_formats_tests;
pub mod threads;
pub mod threads_guides;
#[cfg(any(test, feature = "capability-spike"))]
mod tools;
#[cfg(any(test, feature = "capability-spike"))]
mod visual;

#[cfg(any(test, feature = "capability-spike"))]
pub use visual::{VisualImage, interpret_visual};

#[cfg(any(test, feature = "capability-spike"))]
pub use action_candidates::{
    ActionCandidate, ActionCandidateData, ActionCandidatePriority, ActionCandidateState, ActionRef,
    CheckedActionRef,
};
pub use auth::{Auth, ProviderClient};
pub use chat::{AiAnswer, AiEvent, AiTerminal, HistoryPair, MAX_REWRITE_BYTES, ReasoningEffort};
#[cfg(any(test, feature = "capability-spike"))]
pub use proposal_tools::{
    ACTION_PROPOSAL_BYTES, ActionProposalArgs, CONFLICT_REPORT_BYTES, ConflictArgs, ConflictQuote,
    KNOWLEDGE_PROPOSAL_BYTES, KnowledgeProposalArgs, KnowledgeQuoteArgs, ProposalTools,
};
#[cfg(any(test, feature = "capability-spike"))]
pub use provider_fixture_runtime::{
    answer, answer_with_effort, answer_with_proposals, answer_with_proposals_and_images,
    answer_with_proposals_and_images_with_limit, rewrite,
};
#[cfg(any(test, feature = "capability-spike"))]
pub use tools::{
    ConflictKnowledge, NoteEntry, NoteFacts, NotePage, NoteRangeRequest, Passage,
    READ_ACTION_BYTES, READ_NOTE_BYTES, RawEvidence, RawEvidenceRequest, ReadScope, ReadTools,
    ToolNote, ToolNoteRange, ToolSearch, capped_text,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Chatgpt,
    Copilot,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Selection {
    pub provider: Provider,
    pub model: String,
}

impl Selection {
    /// Checks an explicit model identifier; does not select or persist a model.
    pub fn validate(&self) -> AiResult<()> {
        if valid_model_id(&self.model) {
            Ok(())
        } else {
            Err(AiError::new(AiErrorKind::ModelRefused))
        }
    }
}

pub(crate) fn valid_model_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AccountStatus {
    pub provider: Provider,
    pub name: Option<String>,
    /// Local credentials are present, not a guarantee of upstream validity.
    pub connected: bool,
}

// Deliberately no Debug: device codes belong only on the active login surface.
#[derive(Clone)]
pub struct LoginPrompt {
    pub verification_uri: String,
    pub user_code: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ModelOption {
    pub id: String,
    pub live_qualified: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiErrorKind {
    QuoteNotFound,
    QuoteAmbiguous,
    QuoteOccurrenceInvalid,
    ReconnectNeeded,
    CodeExpired,
    RateLimited,
    Network,
    ModelRefused,
    InvalidToolUse,
    ToolLimitReached,
    TimeLimitReached,
    UnsafeCredentials,
    ToolRejected,
    IndexStale,
    Storage,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AiError {
    pub kind: AiErrorKind,
    pub retry_after_seconds: Option<u64>,
}
pub type AiResult<T> = std::result::Result<T, AiError>;
