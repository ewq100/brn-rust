//! Saved note identity inspection and full proposal preparation through AppWorker.
use super::{
    error::{classify_workflow, CliError},
    expect_positionals, required_positional, scan, sub_word, usage, CliFailure, Globals, Scanned,
    Tokens,
};
use brn_workflow::{app_worker::AppCommand, knowledge::IdentityRequest, vault::VaultPath};

pub enum IdentityCommand {
    Show(String),
    Prepare(IdentityRequest),
}

impl IdentityCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Show(_) => "identity.show",
            Self::Prepare(_) => "identity.prepare",
        }
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(tokens, "identity", "show|prepare")?;
    let (label, options): (&str, &[(&str, bool)]) = match sub.as_str() {
        "show" => ("identity.show", &[]),
        "prepare" => (
            "identity.prepare",
            &[("note-id", true), ("proposal", true), ("title", true)],
        ),
        _ => return Err(usage("unknown identity subcommand")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}

pub(super) fn parse_command(name: &str, scanned: &Scanned) -> Result<IdentityCommand, CliError> {
    expect_positionals(scanned, 1)?;
    let path = required_positional(scanned, "PATH")?;
    VaultPath::parse(path).map_err(|error| usage(error.to_string()))?;
    match name {
        "identity.show" => Ok(IdentityCommand::Show(path.to_owned())),
        "identity.prepare" => {
            let request = IdentityRequest {
                path: path.to_owned(),
                note_id: scanned.require_uuid("note-id")?,
                proposal_id: scanned.require_uuid("proposal")?,
                title: scanned
                    .value("title")
                    .ok_or_else(|| usage("missing --title"))?
                    .to_owned(),
            };
            request.validate().map_err(|error| usage(error.message))?;
            Ok(IdentityCommand::Prepare(request))
        }
        _ => unreachable!(),
    }
}

/// Keep direct Invocation construction subject to the same pre-startup rules.
pub(super) fn prepare(command: &IdentityCommand) -> Result<AppCommand, CliFailure> {
    match command {
        IdentityCommand::Show(path) => {
            VaultPath::parse(path).map_err(|error| usage(error.to_string()))?;
            Ok(AppCommand::NoteIdentity(path.clone()))
        }
        IdentityCommand::Prepare(request) => {
            request.validate().map_err(classify_workflow)?;
            Ok(AppCommand::PrepareNoteIdentity(request.clone()))
        }
    }
}
