//! CLI error taxonomy: stable SCREAMING_SNAKE codes, exit codes, and mapping
//! of typed workflow error categories to those codes.
use brn_workflow::{ErrorKind, WorkflowError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    Usage(String),
    WorkspaceBusy(String),
    NotFound(String),
    IndexMissing(String),
    IndexStale(String),
    IndexInvalid(String),
    ProfileUnavailable(String),
    OperationConflict(String),
    Timeout(String),
    Interrupted(String),
    Workflow(String),
}

impl CliError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Usage(_) => "USAGE",
            Self::WorkspaceBusy(_) => "WORKSPACE_BUSY",
            Self::NotFound(_) => "NOT_FOUND",
            Self::IndexMissing(_) => "INDEX_MISSING",
            Self::IndexStale(_) => "INDEX_STALE",
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
            Self::Usage(m)
            | Self::WorkspaceBusy(m)
            | Self::NotFound(m)
            | Self::IndexMissing(m)
            | Self::IndexStale(m)
            | Self::IndexInvalid(m)
            | Self::ProfileUnavailable(m)
            | Self::OperationConflict(m)
            | Self::Timeout(m)
            | Self::Interrupted(m)
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
        ErrorKind::IndexInvalid => CliError::IndexInvalid(error.message),
        ErrorKind::ProfileUnavailable => CliError::ProfileUnavailable(error.message),
        ErrorKind::OperationConflict => CliError::OperationConflict(error.message),
        ErrorKind::Cancelled => CliError::Interrupted(error.message),
        ErrorKind::Other => CliError::Workflow(error.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
