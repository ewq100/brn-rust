//! `drafts`, `comments`, `revisions` — stub surface for the review phase.
//! Dispatch is wired; implement here later.
use crate::cli::{error::CliError, Invocation, Output};

pub fn run(_invocation: &Invocation) -> Result<Output, CliError> {
    Err(CliError::Workflow("not implemented in this build".into()))
}
