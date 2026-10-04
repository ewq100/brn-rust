//! Saved link inspection and exact proposal preparation through AppWorker.
use super::{
    error::{classify_workflow, CliError},
    expect_positionals, required_positional, scan, sub_word, usage, CliFailure, Globals, Scanned,
    Tokens,
};
use brn_workflow::{app_worker::AppCommand, knowledge::LinkRequest, vault::EvidencePath};
use std::{fs::File, io::Read, os::unix::fs::OpenOptionsExt, path::Path};

pub enum LinksCommand {
    Show(String),
    Prepare(LinkRequest),
}

impl LinksCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Show(_) => "links.show",
            Self::Prepare(_) => "links.prepare",
        }
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(tokens, "links", "show|prepare")?;
    let (label, options): (_, &[(&str, bool)]) = match sub.as_str() {
        "show" => ("links.show", &[]),
        "prepare" => ("links.prepare", &[("file", true)]),
        _ => return Err(usage("unknown links subcommand")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}

/// Open and decode once before any operational or credential storage opens.
fn input(path: &Path) -> Result<LinkRequest, CliError> {
    const MAX_JSON_BYTES: usize = brn_workflow::MAX_NOTE_BYTES * 8;
    let io = |error: std::io::Error| CliError::Workflow(error.to_string());
    let mut file = File::options()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(io)?;
    let meta = file.metadata().map_err(io)?;
    if !meta.is_file() || meta.len() > MAX_JSON_BYTES as u64 {
        return Err(usage("link input requires a regular JSON file up to 8 MiB"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_JSON_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(usage("link JSON exceeds 8 MiB"));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| usage("link input does not match its typed JSON schema"))
}

pub(super) fn parse_command(name: &str, scanned: &Scanned) -> Result<LinksCommand, CliError> {
    match name {
        "links.show" => {
            expect_positionals(scanned, 1)?;
            let path = required_positional(scanned, "PATH")?;
            EvidencePath::parse(path).map_err(|error| usage(error.to_string()))?;
            Ok(LinksCommand::Show(path.to_owned()))
        }
        "links.prepare" => {
            expect_positionals(scanned, 0)?;
            let path = scanned
                .value("file")
                .ok_or_else(|| usage("missing --file"))?;
            let request = input(Path::new(path))?;
            request.validate().map_err(|error| usage(error.message))?;
            Ok(LinksCommand::Prepare(request))
        }
        _ => unreachable!("scanned link command"),
    }
}

pub(super) fn prepare(command: &LinksCommand) -> Result<AppCommand, CliFailure> {
    match command {
        LinksCommand::Show(path) => {
            EvidencePath::parse(path).map_err(|error| usage(error.to_string()))?;
            Ok(AppCommand::NoteLinks(path.clone()))
        }
        LinksCommand::Prepare(request) => {
            request.validate().map_err(classify_workflow)?;
            Ok(AppCommand::PrepareNoteLink(request.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, Invocation};
    use uuid::Uuid;

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

    #[test]
    fn direct_invalid_preparation_refuses_before_operational_and_credential_startup() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("task.credentials");
        let request = LinkRequest {
            path: "current.md".into(),
            target_note_id: Uuid::nil(),
            expected_target_sha256: [0; 32],
            proposal_id: Uuid::new_v4(),
            title: "Link evidence".into(),
            label: "Original õ".into(),
        };
        let invocation = Invocation {
            json: true,
            data_dir: data.clone(),
            model_dir: None,
            vault: None,
            credentials_dir: Some(credentials.clone()),
            command: Command::Links(LinksCommand::Prepare(request)),
        };
        assert!(super::super::execute(&invocation).is_err());
        assert_eq!(std::fs::read_dir(&data).unwrap().count(), 0);
        assert!(!credentials.exists());
    }
}
