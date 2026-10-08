//! Strict lifecycle requests prepared before application state is opened.
use super::{error::CliError, usage};
use brn_workflow::{
    app_worker::AppCommand,
    conversations::{ConversationFilter, ConversationLifecycleRequest, ConversationState},
};
use std::path::PathBuf;

pub(super) fn filter(value: Option<&str>) -> Result<ConversationFilter, CliError> {
    match value.unwrap_or("active") {
        "active" => Ok(ConversationFilter::Active),
        "archived" => Ok(ConversationFilter::Archived),
        "all" => Ok(ConversationFilter::All),
        _ => Err(usage("--state must be active, archived or all")),
    }
}

pub(super) fn prepare(
    target: ConversationState,
    file: &PathBuf,
) -> Result<AppCommand, super::CliFailure> {
    // Reuse the regular-descriptor/nonblocking bounded typed JSON reader.
    let request: ConversationLifecycleRequest = super::proposals::input(file)?;
    request
        .validate()
        .map_err(|error| usage(error.to_string()))?;
    if request.target != target {
        return Err(usage("lifecycle request target must match archive/restore subcommand").into());
    }
    Ok(AppCommand::SetConversationLifecycle(request))
}
