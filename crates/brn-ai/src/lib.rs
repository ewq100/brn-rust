mod auth;
mod error;

pub use auth::{Auth, ProviderClient};

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
        if valid_model_id(&self.model)
            && (self.provider != Provider::Chatgpt || self.model == "gpt-5.5")
        {
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
    ReconnectNeeded,
    CodeExpired,
    RateLimited,
    Network,
    ModelRefused,
    InvalidToolUse,
    ToolLimitReached,
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
