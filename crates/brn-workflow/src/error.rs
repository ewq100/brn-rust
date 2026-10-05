//! Typed workflow error categories. The CLI derives stable machine codes from
//! these kinds instead of matching English message prefixes, so classification
//! is decided at the source and never depends on wording.

use brn_retrieval as retrieval;
use brn_store as store;

/// Coarse machine-readable category of a workflow failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    WorkspaceBusy,
    WorkspaceModeConflict,
    SelectionRequired,
    ReconnectNeeded,
    CodeExpired,
    RateLimited,
    Network,
    InvalidToolUse,
    ToolLimitReached,
    AiStorage,
    AiIndexStale,
    ModelDownloadFailed,
    VaultNotBound,
    VaultUnavailable,
    ToolRejected,
    UnsafeCredentials,
    ModelRefused,
    ModelInvalid,
    SemanticUnavailableInBuild,
    ToolsBusy,
    IndexStale,
    ContextStale,
    SaveUncertain,
    InboxUnavailable,
    InboxUncertain,
    ProfileUnavailable,
    OperationConflict,
    NotFound,
    Cancelled,
    Other,
}

/// A workflow failure carrying its category and the exact human message.
#[derive(Debug, Clone)]
pub struct WorkflowError {
    pub kind: ErrorKind,
    pub message: String,
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for WorkflowError {}

impl WorkflowError {
    /// Projects the closed persisted AI category, never English wording.
    pub fn recorded_ai_failure(code: Option<&str>) -> Self {
        let kind = code
            .and_then(|code| serde_json::from_value(serde_json::Value::String(code.into())).ok())
            .unwrap_or(brn_ai::AiErrorKind::Other);
        brn_ai::AiError::new(kind).into()
    }

    pub(crate) fn typed(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    /// Uncategorized workflow failure (honest CLI `WORKFLOW_ERROR` fallback).
    pub fn msg(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Other,
            message: message.into(),
        }
    }
    /// Genuine cancellation of an in-flight operation.
    pub(crate) fn cancelled() -> Self {
        Self {
            kind: ErrorKind::Cancelled,
            message: "operation cancelled".into(),
        }
    }
}

impl From<String> for WorkflowError {
    fn from(message: String) -> Self {
        Self::msg(message)
    }
}
impl From<&str> for WorkflowError {
    fn from(message: &str) -> Self {
        Self::msg(message)
    }
}
impl From<WorkflowError> for String {
    fn from(e: WorkflowError) -> Self {
        e.message
    }
}
impl From<store::Error> for WorkflowError {
    fn from(e: store::Error) -> Self {
        match e {
            store::Error::WorkspaceBusy(message) => Self {
                kind: ErrorKind::WorkspaceBusy,
                message,
            },
            store::Error::WorkspaceModeConflict(message) => Self {
                kind: ErrorKind::WorkspaceModeConflict,
                message,
            },
            store::Error::OperationConflict(message) => Self {
                kind: ErrorKind::OperationConflict,
                message,
            },
            store::Error::StateChanged(message) => Self::typed(ErrorKind::ContextStale, message),
            store::Error::SaveUncertain(message) => Self::typed(ErrorKind::SaveUncertain, message),
            store::Error::NotFound(message) => Self {
                kind: ErrorKind::NotFound,
                message,
            },
            other => Self::msg(other.to_string()),
        }
    }
}

impl From<brn_ai::AiError> for WorkflowError {
    fn from(e: brn_ai::AiError) -> Self {
        use brn_ai::AiErrorKind;
        let kind = match e.kind {
            AiErrorKind::UnsafeCredentials => ErrorKind::UnsafeCredentials,
            AiErrorKind::ModelRefused => ErrorKind::ModelRefused,
            AiErrorKind::ToolRejected => ErrorKind::ToolRejected,
            AiErrorKind::IndexStale => ErrorKind::AiIndexStale,
            AiErrorKind::ReconnectNeeded => ErrorKind::ReconnectNeeded,
            AiErrorKind::CodeExpired => ErrorKind::CodeExpired,
            AiErrorKind::RateLimited => ErrorKind::RateLimited,
            AiErrorKind::Network => ErrorKind::Network,
            AiErrorKind::InvalidToolUse => ErrorKind::InvalidToolUse,
            AiErrorKind::ToolLimitReached => ErrorKind::ToolLimitReached,
            AiErrorKind::Storage => ErrorKind::AiStorage,
            AiErrorKind::Other => ErrorKind::Other,
        };
        Self::typed(kind, e.to_string())
    }
}

impl From<crate::library::LibraryError> for WorkflowError {
    fn from(e: crate::library::LibraryError) -> Self {
        match e {
            crate::library::LibraryError::Index(e) => e.into(),
            crate::library::LibraryError::Io(e) => Self::msg(e.to_string()),
        }
    }
}
impl From<retrieval::Error> for WorkflowError {
    fn from(e: retrieval::Error) -> Self {
        let message = e.to_string();
        match e {
            retrieval::Error::Unavailable(_) => Self {
                kind: ErrorKind::ProfileUnavailable,
                message,
            },
            retrieval::Error::Cancelled => Self {
                kind: ErrorKind::Cancelled,
                message,
            },
            retrieval::Error::ModelMismatch => Self {
                kind: ErrorKind::IndexStale,
                message,
            },
            retrieval::Error::EmbedderPoisoned => Self {
                kind: ErrorKind::ModelInvalid,
                message,
            },
            #[cfg(feature = "native-retrieval")]
            retrieval::Error::ModelInstall(_) => Self {
                kind: ErrorKind::ModelDownloadFailed,
                message,
            },
            _ => Self::msg(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_busy_and_conflict_map_to_typed_kinds() {
        let busy = WorkflowError::from(store::Error::WorkspaceBusy(
            "data directory is already owned: lock".into(),
        ));
        assert_eq!(busy.kind, ErrorKind::WorkspaceBusy);
        assert_eq!(busy.message, "data directory is already owned: lock");
        let conflict = WorkflowError::from(store::Error::OperationConflict(
            "operation ID conflicts with local mutation".into(),
        ));
        assert_eq!(conflict.kind, ErrorKind::OperationConflict);
        assert_eq!(
            conflict.message,
            "operation ID conflicts with local mutation"
        );
    }

    #[test]
    fn similar_store_invalid_wording_stays_uncategorized() {
        // Word resemblance alone must never classify: an uncategorized store
        // error carrying "already owned" text stays Other.
        let e = WorkflowError::from(store::Error::Invalid(
            "data directory is already owned: fake".into(),
        ));
        assert_eq!(e.kind, ErrorKind::Other);
        assert_eq!(e.message, "data directory is already owned: fake");
        assert_eq!(
            WorkflowError::from(store::Error::Invalid(
                "operation ID conflicts with a hypothetical rule".into(),
            ))
            .kind,
            ErrorKind::Other
        );
        assert_eq!(
            WorkflowError::from(store::Error::Io(std::io::Error::other("x"))).kind,
            ErrorKind::Other
        );
    }

    #[test]
    fn retrieval_errors_map_by_variant() {
        let e = WorkflowError::from(retrieval::Error::Unavailable("semantic"));
        assert_eq!(e.kind, ErrorKind::ProfileUnavailable);
        assert_eq!(e.message, "retrieval profile unavailable: semantic");
        assert_eq!(
            WorkflowError::from(retrieval::Error::Cancelled).kind,
            ErrorKind::Cancelled
        );
        assert_eq!(
            WorkflowError::from(retrieval::Error::Invalid("bad")).kind,
            ErrorKind::Other
        );
    }

    #[test]
    fn strings_and_display_relay_messages() {
        let e: WorkflowError = "plain failure".into();
        assert_eq!(e.kind, ErrorKind::Other);
        assert_eq!(e.to_string(), "plain failure");
        let s: String = WorkflowError::msg("relay").into();
        assert_eq!(s, "relay");
    }
}
