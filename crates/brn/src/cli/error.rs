//! Stable machine-readable categories; classification never inspects wording.
use brn_workflow::{ErrorKind, WorkflowError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    Typed(ErrorKind, String),
    Usage(String),
    OperationConflict(String),
    Timeout(String),
    Interrupted(String),
    Workflow(String),
}
impl CliError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Typed(kind, _) => match kind {
                ErrorKind::WorkspaceBusy => "WORKSPACE_BUSY",
                ErrorKind::WorkspaceModeConflict => "WORKSPACE_MODE_CONFLICT",
                ErrorKind::SelectionRequired => "AI_SELECTION_REQUIRED",
                ErrorKind::ReconnectNeeded => "AI_RECONNECT_NEEDED",
                ErrorKind::CodeExpired => "AI_CODE_EXPIRED",
                ErrorKind::RateLimited => "AI_RATE_LIMITED",
                ErrorKind::Network => "AI_NETWORK",
                ErrorKind::ModelRefused => "AI_MODEL_REFUSED",
                ErrorKind::InvalidToolUse => "AI_INVALID_TOOL_USE",
                ErrorKind::ToolLimitReached => "AI_TOOL_LIMIT_REACHED",
                ErrorKind::TimeLimitReached => "AI_TIME_LIMIT_REACHED",
                ErrorKind::UnsafeCredentials => "AI_UNSAFE_CREDENTIALS",
                ErrorKind::AiStorage => "AI_STORAGE_ERROR",
                ErrorKind::AiIndexStale => "AI_INDEX_STALE",
                ErrorKind::ToolRejected => "AI_TOOL_REJECTED",
                ErrorKind::QuoteNotFound => "AI_QUOTE_NOT_FOUND",
                ErrorKind::QuoteAmbiguous => "AI_QUOTE_AMBIGUOUS",
                ErrorKind::QuoteOccurrenceInvalid => "AI_QUOTE_OCCURRENCE_INVALID",
                ErrorKind::VaultNotBound => "VAULT_NOT_BOUND",
                ErrorKind::VaultUnavailable => "VAULT_UNAVAILABLE",
                ErrorKind::ModelInvalid => "MODEL_INVALID",
                ErrorKind::SemanticUnavailableInBuild => "SEMANTIC_UNAVAILABLE_IN_BUILD",
                ErrorKind::ModelDownloadFailed => "MODEL_DOWNLOAD_FAILED",
                ErrorKind::ToolsBusy => "TOOLS_BUSY",
                ErrorKind::SaveUncertain => "SAVE_UNCERTAIN",
                ErrorKind::InboxUnavailable => "INBOX_UNAVAILABLE",
                ErrorKind::InboxUncertain => "INBOX_UNCERTAIN",
                ErrorKind::IndexStale => "INDEX_STALE",
                ErrorKind::ContextStale => "CONTEXT_STALE",
                ErrorKind::ProfileUnavailable => "PROFILE_UNAVAILABLE",
                ErrorKind::OperationConflict => "OPERATION_CONFLICT",
                ErrorKind::NotFound => "NOT_FOUND",
                ErrorKind::Cancelled => "INTERRUPTED",
                ErrorKind::Other => "WORKFLOW_ERROR",
            },
            Self::Usage(_) => "USAGE",
            Self::OperationConflict(_) => "OPERATION_CONFLICT",
            Self::Timeout(_) => "TIMEOUT",
            Self::Interrupted(_) => "INTERRUPTED",
            Self::Workflow(_) => "WORKFLOW_ERROR",
        }
    }
    pub fn message(&self) -> &str {
        match self {
            Self::Typed(_, m)
            | Self::Usage(m)
            | Self::OperationConflict(m)
            | Self::Timeout(m)
            | Self::Interrupted(m)
            | Self::Workflow(m) => m,
        }
    }
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Timeout(_) | Self::Typed(ErrorKind::TimeLimitReached, _) => 124,
            Self::Interrupted(_) | Self::Typed(ErrorKind::Cancelled, _) => 130,
            _ => 1,
        }
    }
}

pub fn classify_workflow(error: WorkflowError) -> CliError {
    match error.kind {
        ErrorKind::Cancelled => CliError::Interrupted(error.message),
        ErrorKind::Other => CliError::Workflow(error.message),
        kind => CliError::Typed(kind, error.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_decides_code_and_exact_message() {
        for message in ["data directory is already owned: fake", ""] {
            let error = classify_workflow(WorkflowError {
                kind: ErrorKind::Other,
                message: message.into(),
            });
            assert_eq!(error.code(), "WORKFLOW_ERROR");
            assert_eq!(error.message(), message);
        }
        for (kind, code) in [
            (ErrorKind::WorkspaceBusy, "WORKSPACE_BUSY"),
            (ErrorKind::ContextStale, "CONTEXT_STALE"),
            (ErrorKind::IndexStale, "INDEX_STALE"),
        ] {
            let error = classify_workflow(WorkflowError {
                kind,
                message: "exact bytes".into(),
            });
            assert_eq!(error.code(), code);
            assert_eq!(error.message(), "exact bytes");
            assert_eq!(error.exit_code(), 1);
        }
        assert_eq!(CliError::Usage("bad".into()).exit_code(), 2);
        assert_eq!(CliError::Timeout("deadline".into()).exit_code(), 124);
        assert_eq!(
            classify_workflow(WorkflowError {
                kind: ErrorKind::Cancelled,
                message: "cancelled".into()
            })
            .exit_code(),
            130
        );
    }

    #[test]
    fn every_ai_category_and_simple_authority_kind_has_a_stable_typed_code() {
        use brn_workflow::{AiError, AiErrorKind};
        for (kind, code) in [
            (AiErrorKind::ReconnectNeeded, "AI_RECONNECT_NEEDED"),
            (AiErrorKind::CodeExpired, "AI_CODE_EXPIRED"),
            (AiErrorKind::RateLimited, "AI_RATE_LIMITED"),
            (AiErrorKind::Network, "AI_NETWORK"),
            (AiErrorKind::ModelRefused, "AI_MODEL_REFUSED"),
            (AiErrorKind::InvalidToolUse, "AI_INVALID_TOOL_USE"),
            (AiErrorKind::ToolLimitReached, "AI_TOOL_LIMIT_REACHED"),
            (AiErrorKind::UnsafeCredentials, "AI_UNSAFE_CREDENTIALS"),
            (AiErrorKind::Storage, "AI_STORAGE_ERROR"),
            (AiErrorKind::ToolRejected, "AI_TOOL_REJECTED"),
            (AiErrorKind::QuoteNotFound, "AI_QUOTE_NOT_FOUND"),
            (AiErrorKind::QuoteAmbiguous, "AI_QUOTE_AMBIGUOUS"),
            (
                AiErrorKind::QuoteOccurrenceInvalid,
                "AI_QUOTE_OCCURRENCE_INVALID",
            ),
            (AiErrorKind::IndexStale, "AI_INDEX_STALE"),
            (AiErrorKind::Other, "WORKFLOW_ERROR"),
        ] {
            let workflow: WorkflowError = AiError::new(kind).into();
            assert_eq!(classify_workflow(workflow).code(), code);
            let stored = serde_json::to_value(kind).unwrap();
            let projected = WorkflowError::recorded_ai_failure(stored.as_str());
            assert_eq!(classify_workflow(projected).code(), code);
        }
        for (kind, code) in [
            (ErrorKind::WorkspaceModeConflict, "WORKSPACE_MODE_CONFLICT"),
            (ErrorKind::SelectionRequired, "AI_SELECTION_REQUIRED"),
            (ErrorKind::VaultNotBound, "VAULT_NOT_BOUND"),
            (ErrorKind::VaultUnavailable, "VAULT_UNAVAILABLE"),
            (ErrorKind::SaveUncertain, "SAVE_UNCERTAIN"),
            (ErrorKind::ModelDownloadFailed, "MODEL_DOWNLOAD_FAILED"),
            (
                ErrorKind::SemanticUnavailableInBuild,
                "SEMANTIC_UNAVAILABLE_IN_BUILD",
            ),
        ] {
            assert_eq!(
                classify_workflow(WorkflowError {
                    kind,
                    message: "wording does not matter".into()
                })
                .code(),
                code
            );
        }
    }
}
