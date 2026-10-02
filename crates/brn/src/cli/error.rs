//! CLI error taxonomy: stable SCREAMING_SNAKE codes, exit codes, and mapping
//! of typed workflow error categories to those codes.
use brn_workflow::{ErrorKind, WorkflowError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    Typed(ErrorKind, String),
    Usage(String),
    WorkspaceBusy(String),
    NotFound(String),
    IndexMissing(String),
    IndexStale(String),
    EvidenceStale(String),
    ContextStale(String),
    IndexInvalid(String),
    ProfileUnavailable(String),
    OperationConflict(String),
    Timeout(String),
    Interrupted(String),
    Workflow(String),
    NoteStateChanged(String),
    NoteConflict(String),
    NoteMissing(String),
    NoteUnsupported(String),
    NoteSaveUncertain(String),
    NoteIo(String),
    NoteStorage(String),
    VaultBusy(String),
    VaultUnavailable(String),
}

impl CliError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Typed(kind, _) => match kind {
                ErrorKind::WorkspaceModeConflict => "WORKSPACE_MODE_CONFLICT",
                ErrorKind::WorkspaceModeRequired => "WORKSPACE_MODE_REQUIRED",
                ErrorKind::LegacyAiRetired => "LEGACY_AI_RETIRED",
                ErrorKind::SelectionRequired => "AI_SELECTION_REQUIRED",
                ErrorKind::ReconnectNeeded => "AI_RECONNECT_NEEDED",
                ErrorKind::CodeExpired => "AI_CODE_EXPIRED",
                ErrorKind::RateLimited => "AI_RATE_LIMITED",
                ErrorKind::Network => "AI_NETWORK",
                ErrorKind::ModelRefused => "AI_MODEL_REFUSED",
                ErrorKind::InvalidToolUse => "AI_INVALID_TOOL_USE",
                ErrorKind::ToolLimitReached => "AI_TOOL_LIMIT_REACHED",
                ErrorKind::UnsafeCredentials => "AI_UNSAFE_CREDENTIALS",
                ErrorKind::AiStorage => "AI_STORAGE_ERROR",
                ErrorKind::AiIndexStale => "AI_INDEX_STALE",
                ErrorKind::ToolRejected => "AI_TOOL_REJECTED",
                ErrorKind::VaultNotBound => "VAULT_NOT_BOUND",
                ErrorKind::VaultUnavailable => "VAULT_UNAVAILABLE",
                ErrorKind::ModelInvalid => "MODEL_INVALID",
                ErrorKind::SemanticUnavailableInBuild => "SEMANTIC_UNAVAILABLE_IN_BUILD",
                ErrorKind::ModelDownloadFailed => "MODEL_DOWNLOAD_FAILED",
                ErrorKind::ToolsBusy => "TOOLS_BUSY",
                _ => "WORKFLOW_ERROR",
            },
            Self::NoteStateChanged(_) => "NOTE_STATE_CHANGED",
            Self::NoteConflict(_) => "NOTE_CONFLICT",
            Self::NoteMissing(_) => "NOTE_MISSING",
            Self::NoteUnsupported(_) => "NOTE_UNSUPPORTED",
            Self::NoteSaveUncertain(_) => "NOTE_SAVE_UNCERTAIN",
            Self::NoteIo(_) => "NOTE_IO_ERROR",
            Self::NoteStorage(_) => "NOTE_STORAGE_ERROR",
            Self::VaultBusy(_) => "VAULT_BUSY",
            Self::VaultUnavailable(_) => "VAULT_UNAVAILABLE",
            Self::Usage(_) => "USAGE",
            Self::WorkspaceBusy(_) => "WORKSPACE_BUSY",
            Self::NotFound(_) => "NOT_FOUND",
            Self::IndexMissing(_) => "INDEX_MISSING",
            Self::IndexStale(_) => "INDEX_STALE",
            Self::EvidenceStale(_) => "EVIDENCE_STALE",
            Self::ContextStale(_) => "CONTEXT_STALE",
            Self::IndexInvalid(_) => "INDEX_INVALID",
            Self::ProfileUnavailable(_) => "PROFILE_UNAVAILABLE",
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
            | Self::WorkspaceBusy(m)
            | Self::NotFound(m)
            | Self::IndexMissing(m)
            | Self::IndexStale(m)
            | Self::EvidenceStale(m)
            | Self::ContextStale(m)
            | Self::IndexInvalid(m)
            | Self::ProfileUnavailable(m)
            | Self::OperationConflict(m)
            | Self::Timeout(m)
            | Self::Interrupted(m)
            | Self::NoteStateChanged(m)
            | Self::NoteConflict(m)
            | Self::NoteMissing(m)
            | Self::NoteUnsupported(m)
            | Self::NoteSaveUncertain(m)
            | Self::NoteIo(m)
            | Self::NoteStorage(m)
            | Self::VaultBusy(m)
            | Self::VaultUnavailable(m)
            | Self::Workflow(m) => m,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Timeout(_) => 124,
            Self::Interrupted(_) => 130,
            _ => 1,
        }
    }
}

/// Map a typed workflow failure to its stable CLI code. The category was
/// decided at the source; wording is never inspected, so uncategorized
/// failures are an honest `WORKFLOW_ERROR`.
pub fn classify_workflow(error: WorkflowError) -> CliError {
    match error.kind {
        ErrorKind::WorkspaceBusy => CliError::WorkspaceBusy(error.message),
        ErrorKind::IndexMissing => CliError::IndexMissing(error.message),
        ErrorKind::IndexStale => CliError::IndexStale(error.message),
        ErrorKind::EvidenceStale => CliError::EvidenceStale(error.message),
        ErrorKind::ContextStale => CliError::ContextStale(error.message),
        ErrorKind::IndexInvalid => CliError::IndexInvalid(error.message),
        ErrorKind::ProfileUnavailable => CliError::ProfileUnavailable(error.message),
        ErrorKind::OperationConflict => CliError::OperationConflict(error.message),
        ErrorKind::Cancelled => CliError::Interrupted(error.message),
        ErrorKind::Other => CliError::Workflow(error.message),
        ErrorKind::NotFound => CliError::NotFound(error.message),
        ErrorKind::WorkspaceModeConflict
        | ErrorKind::WorkspaceModeRequired
        | ErrorKind::LegacyAiRetired
        | ErrorKind::SelectionRequired
        | ErrorKind::ReconnectNeeded
        | ErrorKind::CodeExpired
        | ErrorKind::RateLimited
        | ErrorKind::Network
        | ErrorKind::InvalidToolUse
        | ErrorKind::ToolLimitReached
        | ErrorKind::AiStorage
        | ErrorKind::AiIndexStale
        | ErrorKind::ModelDownloadFailed
        | ErrorKind::VaultNotBound
        | ErrorKind::VaultUnavailable
        | ErrorKind::ToolRejected
        | ErrorKind::UnsafeCredentials
        | ErrorKind::ModelRefused
        | ErrorKind::ModelInvalid
        | ErrorKind::SemanticUnavailableInBuild
        | ErrorKind::ToolsBusy => CliError::Typed(error.kind, error.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_evidence_and_context_have_stable_codes() {
        for (kind, code) in [
            (ErrorKind::EvidenceStale, "EVIDENCE_STALE"),
            (ErrorKind::ContextStale, "CONTEXT_STALE"),
        ] {
            let mapped = classify_workflow(WorkflowError {
                kind,
                message: "exact message".into(),
            });
            assert_eq!(mapped.code(), code);
            assert_eq!(mapped.message(), "exact message");
            assert_eq!(mapped.exit_code(), 1);
        }
    }

    #[test]
    fn uncategorized_codes_stay_stable() {
        assert_eq!(
            classified(ErrorKind::Other, "data directory must be absolute").code(),
            "WORKFLOW_ERROR"
        );
    }

    #[test]
    fn codes_and_exit_codes_map_stably() {
        let e = CliError::Usage("bad".into());
        assert_eq!((e.code(), e.exit_code()), ("USAGE", 2));
        assert_eq!(CliError::Timeout("t".into()).exit_code(), 124);
        assert_eq!(CliError::Interrupted("i".into()).exit_code(), 130);
        for e in [
            CliError::WorkspaceBusy("b".into()),
            CliError::NotFound("n".into()),
            CliError::IndexMissing("a".into()),
            CliError::IndexStale("s".into()),
            CliError::IndexInvalid("v".into()),
            CliError::ProfileUnavailable("p".into()),
            CliError::OperationConflict("o".into()),
            CliError::Workflow("w".into()),
        ] {
            assert_eq!(e.exit_code(), 1);
        }
        assert_eq!(CliError::WorkspaceBusy("b".into()).code(), "WORKSPACE_BUSY");
        assert_eq!(CliError::NotFound("n".into()).code(), "NOT_FOUND");
        assert_eq!(CliError::IndexMissing("a".into()).code(), "INDEX_MISSING");
        assert_eq!(CliError::IndexStale("s".into()).code(), "INDEX_STALE");
        assert_eq!(CliError::IndexInvalid("v".into()).code(), "INDEX_INVALID");
        assert_eq!(
            CliError::ProfileUnavailable("p".into()).code(),
            "PROFILE_UNAVAILABLE"
        );
        assert_eq!(
            CliError::OperationConflict("o".into()).code(),
            "OPERATION_CONFLICT"
        );
        assert_eq!(CliError::Timeout("t".into()).code(), "TIMEOUT");
        assert_eq!(CliError::Interrupted("i".into()).code(), "INTERRUPTED");
        assert_eq!(CliError::Workflow("w".into()).code(), "WORKFLOW_ERROR");
    }

    fn classified(kind: ErrorKind, message: &str) -> CliError {
        classify_workflow(WorkflowError {
            kind,
            message: message.into(),
        })
    }

    #[test]
    fn typed_kinds_map_to_exact_stable_variants() {
        assert_eq!(
            classified(
                ErrorKind::WorkspaceBusy,
                "data directory is already owned: boom"
            ),
            CliError::WorkspaceBusy("data directory is already owned: boom".into())
        );
        assert_eq!(
            classified(ErrorKind::IndexMissing, "no active index; build it first"),
            CliError::IndexMissing("no active index; build it first".into())
        );
        assert_eq!(
            classified(
                ErrorKind::IndexStale,
                "index is stale after source changes; rebuild it"
            ),
            CliError::IndexStale("index is stale after source changes; rebuild it".into())
        );
        assert_eq!(
            classified(
                ErrorKind::IndexStale,
                "sources changed during indexing; rebuild"
            ),
            CliError::IndexStale("sources changed during indexing; rebuild".into())
        );
        assert_eq!(
            classified(
                ErrorKind::IndexInvalid,
                "invalid active index pointer; rebuild"
            ),
            CliError::IndexInvalid("invalid active index pointer; rebuild".into())
        );
        assert_eq!(
            classified(
                ErrorKind::ProfileUnavailable,
                "retrieval profile unavailable: semantic"
            ),
            CliError::ProfileUnavailable("retrieval profile unavailable: semantic".into())
        );
        assert_eq!(
            classified(
                ErrorKind::OperationConflict,
                "operation ID conflicts with question/session/profile"
            ),
            CliError::OperationConflict(
                "operation ID conflicts with question/session/profile".into()
            )
        );
        assert_eq!(
            classified(
                ErrorKind::OperationConflict,
                "operation ID already belongs to another command"
            ),
            CliError::OperationConflict("operation ID already belongs to another command".into())
        );
        assert_eq!(
            classified(ErrorKind::Cancelled, "operation cancelled"),
            CliError::Interrupted("operation cancelled".into())
        );
    }

    #[test]
    fn kind_decides_not_wording() {
        // Same kind with different messages must yield the same code.
        assert_eq!(
            classified(
                ErrorKind::WorkspaceBusy,
                "data directory is already owned: a"
            )
            .code(),
            classified(ErrorKind::WorkspaceBusy, "lock held by another process").code()
        );
        // Uncategorized failures are an honest WORKFLOW_ERROR.
        assert_eq!(
            classified(ErrorKind::Other, "boom").code(),
            "WORKFLOW_ERROR"
        );
        assert_eq!(classified(ErrorKind::Other, "").code(), "WORKFLOW_ERROR");
        // The key anti-string-matching regression: wording that resembles a
        // category must never classify when the kind is Other. This is exactly
        // what a store `Invalid` carrying "already owned" text becomes.
        let lookalike = classified(ErrorKind::Other, "data directory is already owned: fake");
        assert_eq!(lookalike.code(), "WORKFLOW_ERROR");
        assert_eq!(
            lookalike,
            CliError::Workflow("data directory is already owned: fake".into())
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
            (ErrorKind::WorkspaceModeRequired, "WORKSPACE_MODE_REQUIRED"),
            (ErrorKind::LegacyAiRetired, "LEGACY_AI_RETIRED"),
            (ErrorKind::SelectionRequired, "AI_SELECTION_REQUIRED"),
            (ErrorKind::VaultNotBound, "VAULT_NOT_BOUND"),
            (ErrorKind::VaultUnavailable, "VAULT_UNAVAILABLE"),
            (ErrorKind::ModelDownloadFailed, "MODEL_DOWNLOAD_FAILED"),
            (
                ErrorKind::SemanticUnavailableInBuild,
                "SEMANTIC_UNAVAILABLE_IN_BUILD",
            ),
        ] {
            assert_eq!(classified(kind, "wording does not matter").code(), code);
        }
    }
}
