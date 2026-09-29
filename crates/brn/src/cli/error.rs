//! CLI error taxonomy: stable SCREAMING_SNAKE codes, exit codes, and
//! classification of workflow `String` errors by documented sentinel prefix.

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
    // Wired for later phases (deadline flag, SIGINT flag); not produced yet.
    #[allow(dead_code)]
    Timeout(String),
    #[allow(dead_code)]
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

/// Classify a workflow `String` error by its exact documented sentinel prefix;
/// anything unrecognized is an honest `WORKFLOW_ERROR`.
pub fn classify_workflow(message: String) -> CliError {
    let starts = |prefix: &str| message.starts_with(prefix);
    if starts("data directory is already owned") {
        CliError::WorkspaceBusy(message)
    } else if starts("no active index") {
        CliError::IndexMissing(message)
    } else if starts("index is stale") || starts("sources changed during indexing") {
        CliError::IndexStale(message)
    } else if starts("invalid active index pointer") {
        CliError::IndexInvalid(message)
    } else if starts("retrieval profile unavailable") {
        CliError::ProfileUnavailable(message)
    } else if starts("operation ID conflicts") || starts("operation ID already belongs") {
        CliError::OperationConflict(message)
    } else {
        CliError::Workflow(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn sentinel_prefixes_classify_exactly() {
        assert_eq!(
            classify_workflow("data directory is already owned: boom".into()),
            CliError::WorkspaceBusy("data directory is already owned: boom".into())
        );
        assert_eq!(
            classify_workflow("no active index; build it first".into()),
            CliError::IndexMissing("no active index; build it first".into())
        );
        assert_eq!(
            classify_workflow("index is stale after source changes; rebuild it".into()),
            CliError::IndexStale("index is stale after source changes; rebuild it".into())
        );
        assert_eq!(
            classify_workflow("sources changed during indexing; rebuild".into()),
            CliError::IndexStale("sources changed during indexing; rebuild".into())
        );
        assert_eq!(
            classify_workflow("invalid active index pointer; rebuild".into()),
            CliError::IndexInvalid("invalid active index pointer; rebuild".into())
        );
        assert_eq!(
            classify_workflow("retrieval profile unavailable: semantic".into()),
            CliError::ProfileUnavailable("retrieval profile unavailable: semantic".into())
        );
        assert_eq!(
            classify_workflow("operation ID conflicts with question/session/profile".into()),
            CliError::OperationConflict(
                "operation ID conflicts with question/session/profile".into()
            )
        );
        assert_eq!(
            classify_workflow("operation ID already belongs to another command".into()),
            CliError::OperationConflict("operation ID already belongs to another command".into())
        );
    }

    #[test]
    fn non_matching_and_non_prefixed_messages_fall_back() {
        assert_eq!(
            classify_workflow("data directory must be absolute".into()).code(),
            "WORKFLOW_ERROR"
        );
        assert_eq!(classify_workflow("boom".into()).code(), "WORKFLOW_ERROR");
        assert_eq!(classify_workflow(String::new()).code(), "WORKFLOW_ERROR");
        // Exact prefix only: different case or embedded position does not match.
        assert_eq!(
            classify_workflow("Data directory is already owned: x".into()).code(),
            "WORKFLOW_ERROR"
        );
        assert_eq!(
            classify_workflow("prefix data directory is already owned".into()).code(),
            "WORKFLOW_ERROR"
        );
    }
}
