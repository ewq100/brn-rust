//! `import`, `documents set-search-approval`, `index build`, `search` — stub
//! surface for the retrieval phase. Dispatch is wired; implement here later.
use crate::cli::{error::CliError, Invocation, Output};

pub fn run(_invocation: &Invocation) -> Result<Output, CliError> {
    Err(not_implemented())
}

pub fn not_implemented() -> CliError {
    CliError::Workflow("not implemented in this build".into())
}
