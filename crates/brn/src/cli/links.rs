//! Read-only saved Markdown relationship evidence through AppWorker.
use super::{
    error::CliError, expect_positionals, required_positional, scan, sub_word, usage, CliFailure,
    Globals, Scanned, Tokens,
};
use brn_workflow::{app_worker::AppCommand, vault::EvidencePath};

pub enum LinksCommand {
    Show(String),
}

impl LinksCommand {
    pub fn name(&self) -> &'static str {
        "links.show"
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    if sub_word(tokens, "links", "show")? != "show" {
        return Err(usage("unknown links subcommand"));
    }
    *name = Some("links.show");
    scan(tokens, globals, &[])
}

pub(super) fn parse_command(scanned: &Scanned) -> Result<LinksCommand, CliError> {
    expect_positionals(scanned, 1)?;
    let path = required_positional(scanned, "PATH")?;
    EvidencePath::parse(path).map_err(|error| usage(error.to_string()))?;
    Ok(LinksCommand::Show(path.to_owned()))
}

pub(super) fn prepare(command: &LinksCommand) -> Result<AppCommand, CliFailure> {
    let LinksCommand::Show(path) = command;
    EvidencePath::parse(path).map_err(|error| usage(error.to_string()))?;
    Ok(AppCommand::NoteLinks(path.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, Invocation};

    #[test]
    fn direct_invalid_invocation_refuses_before_operational_and_credential_startup() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("task.credentials");
        let invocation = Invocation {
            json: true,
            data_dir: data.clone(),
            model_dir: None,
            vault: None,
            credentials_dir: Some(credentials.clone()),
            command: Command::Links(LinksCommand::Show("../outside.md".into())),
        };
        assert!(super::super::execute(&invocation).is_err());
        assert_eq!(std::fs::read_dir(&data).unwrap().count(), 0);
        assert!(!credentials.exists());
    }
}
