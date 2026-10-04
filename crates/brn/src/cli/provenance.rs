//! Durable citation inspection and proposal preparation through the shared worker.
use super::{
    error::{classify_workflow, CliError},
    expect_positionals, required_positional, scan, sub_word, usage, CliFailure, Globals, Scanned,
    Tokens,
};
use brn_workflow::{
    app_worker::AppCommand,
    knowledge::{CitationRequest, ProvenanceRequest},
    vault::EvidencePath,
};
use serde::de::DeserializeOwned;
use std::{fs::File, io::Read, os::unix::fs::OpenOptionsExt, path::Path};

pub enum ProvenanceCommand {
    Show(String),
    Capture(CitationRequest),
    Prepare(ProvenanceRequest),
}

impl ProvenanceCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Show(_) => "provenance.show",
            Self::Capture(_) => "provenance.capture",
            Self::Prepare(_) => "provenance.prepare",
        }
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(tokens, "provenance", "show|capture|prepare")?;
    let (label, options): (_, &[(&str, bool)]) = match sub.as_str() {
        "show" => ("provenance.show", &[]),
        "capture" => ("provenance.capture", &[("file", true)]),
        "prepare" => ("provenance.prepare", &[("file", true)]),
        _ => return Err(usage("unknown provenance subcommand")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}

/// Decode exactly once before any operational or credential storage opens.
fn input<T: DeserializeOwned>(path: &Path) -> Result<T, CliError> {
    // JSON escaping can expand the bounded Markdown metadata. The request has
    // its own encoded cap; decoded domain bounds are checked separately.
    const MAX_JSON_BYTES: usize = brn_workflow::MAX_NOTE_BYTES * 8;
    let io = |error: std::io::Error| CliError::Workflow(error.to_string());
    let mut file = File::options()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(io)?;
    let meta = file.metadata().map_err(io)?;
    if !meta.is_file() || meta.len() > MAX_JSON_BYTES as u64 {
        return Err(usage(
            "provenance input requires a regular JSON file up to 8 MiB",
        ));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_JSON_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(usage("provenance JSON exceeds 8 MiB"));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| usage("provenance input does not match its typed JSON schema"))
}

pub(super) fn parse_command(name: &str, s: &Scanned) -> Result<ProvenanceCommand, CliError> {
    if name == "provenance.show" {
        expect_positionals(s, 1)?;
        let path = required_positional(s, "PATH")?;
        EvidencePath::parse(path).map_err(|error| usage(error.to_string()))?;
        return Ok(ProvenanceCommand::Show(path.to_owned()));
    }
    expect_positionals(s, 0)?;
    let path = s.value("file").ok_or_else(|| usage("missing --file"))?;
    match name {
        "provenance.capture" => {
            let request: CitationRequest = input(Path::new(path))?;
            request.validate().map_err(|error| usage(error.message))?;
            Ok(ProvenanceCommand::Capture(request))
        }
        "provenance.prepare" => {
            let request: ProvenanceRequest = input(Path::new(path))?;
            request.validate().map_err(|error| usage(error.message))?;
            Ok(ProvenanceCommand::Prepare(request))
        }
        _ => unreachable!("scanned provenance command"),
    }
}

/// Direct Invocation construction must obey the same pre-startup validation.
pub(super) fn prepare(command: &ProvenanceCommand) -> Result<AppCommand, CliFailure> {
    match command {
        ProvenanceCommand::Show(path) => {
            EvidencePath::parse(path).map_err(|error| usage(error.to_string()))?;
            Ok(AppCommand::NoteProvenance(path.clone()))
        }
        ProvenanceCommand::Capture(request) => {
            request.validate().map_err(classify_workflow)?;
            Ok(AppCommand::CaptureCitation(request.clone()))
        }
        ProvenanceCommand::Prepare(request) => {
            request.validate().map_err(classify_workflow)?;
            Ok(AppCommand::PrepareNoteProvenance(request.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, Invocation};
    use uuid::Uuid;

    #[test]
    fn direct_invalid_invocations_leave_operational_and_credential_directories_unopened() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("task.credentials");
        let capture = CitationRequest {
            note_id: Uuid::nil(),
            expected_sha256: [0; 32],
            start_byte: 0,
            end_byte: 1,
        };
        let provenance = ProvenanceRequest {
            path: "note.md".into(),
            proposal_id: Uuid::new_v4(),
            title: "Cite source".into(),
            citations: Vec::new(),
        };
        for command in [
            ProvenanceCommand::Show("../outside.md".into()),
            ProvenanceCommand::Capture(capture),
            ProvenanceCommand::Prepare(provenance),
        ] {
            let invocation = Invocation {
                json: true,
                data_dir: data.clone(),
                model_dir: None,
                vault: None,
                credentials_dir: Some(credentials.clone()),
                command: Command::Provenance(command),
            };
            assert!(super::super::execute(&invocation).is_err());
            assert_eq!(std::fs::read_dir(&data).unwrap().count(), 0);
            assert!(!credentials.exists());
        }
    }
}
