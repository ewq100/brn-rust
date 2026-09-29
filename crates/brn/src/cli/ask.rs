//! `ask`, `conversations` — stub surface for the grounded-chat phase.
//! Dispatch is wired; implement here later (SIGINT: `crate::CANCEL`).
use crate::cli::{error::CliError, Invocation, Output};

pub fn run(_invocation: &Invocation) -> Result<Output, CliError> {
    Err(CliError::Workflow("not implemented in this build".into()))
}
