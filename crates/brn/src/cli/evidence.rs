//! Explicit read-only saved evidence, including archived Markdown.
use super::{
    error::CliError, expect_positionals, required_positional, scan, sub_word, usage, CliFailure,
    Globals, Scanned, Tokens,
};
use brn_workflow::{app_worker::AppCommand, knowledge::RawEvidenceRequest, vault::EvidencePath};

pub enum EvidenceCommand {
    Read(String),
    ReadRaw(RawEvidenceRequest),
}

impl EvidenceCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Read(_) => "evidence.read",
            Self::ReadRaw(_) => "evidence.read-raw",
        }
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    match sub_word(tokens, "evidence", "read|read-raw")?.as_str() {
        "read" => {
            *name = Some("evidence.read");
            scan(tokens, globals, &[])
        }
        "read-raw" => {
            *name = Some("evidence.read-raw");
            scan(tokens, globals, &[("file", true)])
        }
        _ => Err(usage("unknown evidence subcommand")),
    }
}

pub(super) fn parse_command(name: &str, scanned: &Scanned) -> Result<EvidenceCommand, CliError> {
    if name == "evidence.read-raw" {
        expect_positionals(scanned, 0)?;
        let file = scanned
            .value("file")
            .ok_or_else(|| usage("missing --file"))?;
        let request: RawEvidenceRequest =
            super::input::read_small_json_file(std::path::Path::new(file), "raw evidence")?;
        request
            .validate()
            .map_err(|error| usage(error.to_string()))?;
        EvidencePath::parse(&request.path).map_err(|error| usage(error.to_string()))?;
        return Ok(EvidenceCommand::ReadRaw(request));
    }
    expect_positionals(scanned, 1)?;
    let path = required_positional(scanned, "PATH")?;
    EvidencePath::parse(path).map_err(|error| usage(error.to_string()))?;
    Ok(EvidenceCommand::Read(path.to_owned()))
}

/// Direct Invocation construction must also validate before worker startup.
pub(super) fn prepare(command: &EvidenceCommand) -> Result<AppCommand, CliFailure> {
    match command {
        EvidenceCommand::Read(path) => {
            EvidencePath::parse(path).map_err(|error| usage(error.to_string()))?;
            Ok(AppCommand::EvidenceNote(path.clone()))
        }
        EvidenceCommand::ReadRaw(request) => {
            request
                .validate()
                .map_err(|error| usage(error.to_string()))?;
            EvidencePath::parse(&request.path).map_err(|error| usage(error.to_string()))?;
            Ok(AppCommand::RawEvidence(request.clone()))
        }
    }
}
