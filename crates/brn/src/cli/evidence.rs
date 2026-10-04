//! Explicit read-only saved evidence, including archived Markdown.
use super::{
    error::CliError, expect_positionals, required_positional, scan, sub_word, usage, CliFailure,
    Globals, Scanned, Tokens,
};
use brn_workflow::{app_worker::AppCommand, vault::EvidencePath};

pub enum EvidenceCommand {
    Read(String),
}

impl EvidenceCommand {
    pub fn name(&self) -> &'static str {
        "evidence.read"
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    if sub_word(tokens, "evidence", "read")? != "read" {
        return Err(usage("unknown evidence subcommand"));
    }
    *name = Some("evidence.read");
    scan(tokens, globals, &[])
}

pub(super) fn parse_command(scanned: &Scanned) -> Result<EvidenceCommand, CliError> {
    expect_positionals(scanned, 1)?;
    let path = required_positional(scanned, "PATH")?;
    EvidencePath::parse(path).map_err(|error| usage(error.to_string()))?;
    Ok(EvidenceCommand::Read(path.to_owned()))
}

/// Direct Invocation construction must also validate before worker startup.
pub(super) fn prepare(command: &EvidenceCommand) -> Result<AppCommand, CliFailure> {
    let EvidenceCommand::Read(path) = command;
    EvidencePath::parse(path).map_err(|error| usage(error.to_string()))?;
    Ok(AppCommand::EvidenceNote(path.clone()))
}
