//! Read-only observations of durable citation evidence through the shared worker.
use super::{
    error::{classify_workflow, CliError},
    expect_positionals, scan, sub_word, usage, CliFailure, Globals, Output, Scanned, Tokens,
};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    knowledge::{CitationReviewCursor, CitationReviewDetailRequest, CitationReviewRequest},
};
use serde::de::DeserializeOwned;
use std::{fs::File, io::Read, os::unix::fs::OpenOptionsExt, path::Path};

pub enum NeedsReviewCommand {
    Citations(CitationReviewRequest),
    Show(CitationReviewDetailRequest),
}
impl NeedsReviewCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Citations(_) => "needs-review.citations",
            Self::Show(_) => "needs-review.show",
        }
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(tokens, "needs-review", "citations|show")?;
    let (label, options): (_, &[(&str, bool)]) = match sub.as_str() {
        "citations" => (
            "needs-review.citations",
            &[("limit", true), ("cursor", true)],
        ),
        "show" => ("needs-review.show", &[("file", true)]),
        _ => return Err(usage("unknown needs-review subcommand")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}

// Both small request schemas are decoded once before opening any workspace.
fn input<T: DeserializeOwned>(path: &Path) -> Result<T, CliError> {
    const MAX: usize = 64 * 1024;
    let io = |error: std::io::Error| CliError::Workflow(error.to_string());
    let mut file = File::options()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(io)?;
    let metadata = file.metadata().map_err(io)?;
    if !metadata.is_file() || metadata.len() > MAX as u64 {
        return Err(usage(
            "citation review needs a regular JSON file up to 64 KiB",
        ));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX {
        return Err(usage("citation review JSON exceeds 64 KiB"));
    }
    serde_json::from_slice(&bytes).map_err(|_| usage("invalid citation review JSON schema"))
}

pub(super) fn validate(command: &NeedsReviewCommand) -> Result<(), CliError> {
    let result = match command {
        NeedsReviewCommand::Citations(request) => request.validate(),
        NeedsReviewCommand::Show(request) => request.validate(),
    };
    result.map_err(|error| usage(error.to_string()))
}

pub(super) fn parse_command(name: &str, scanned: &Scanned) -> Result<NeedsReviewCommand, CliError> {
    expect_positionals(scanned, 0)?;
    let command = match name {
        "needs-review.citations" => NeedsReviewCommand::Citations(CitationReviewRequest {
            limit: scanned
                .value("limit")
                .map(|raw| {
                    raw.parse::<usize>()
                        .map_err(|_| usage("invalid --limit integer"))
                })
                .transpose()?
                .unwrap_or(25),
            cursor: scanned
                .value("cursor")
                .map(|path| input::<CitationReviewCursor>(Path::new(path)))
                .transpose()?,
        }),
        "needs-review.show" => NeedsReviewCommand::Show(input(Path::new(
            scanned
                .value("file")
                .ok_or_else(|| usage("missing --file"))?,
        ))?),
        _ => unreachable!("scanned citation review command"),
    };
    validate(&command)?;
    Ok(command)
}

pub(super) fn prepare(command: &NeedsReviewCommand) -> Result<AppCommand, CliFailure> {
    validate(command)?;
    Ok(match command {
        NeedsReviewCommand::Citations(request) => AppCommand::CitationReview(request.clone()),
        NeedsReviewCommand::Show(request) => AppCommand::CitationReviewDetail(request.clone()),
    })
}

pub(super) fn output(command: &NeedsReviewCommand, event: AppEvent) -> Result<Output, CliFailure> {
    let data = match (command, event) {
        (NeedsReviewCommand::Citations(request), AppEvent::CitationReview(page)) => {
            page.validate_for(request).map_err(classify_workflow)?;
            serde_json::json!(page)
        }
        (NeedsReviewCommand::Show(request), AppEvent::CitationReviewDetail(detail)) => {
            detail.validate_for(request).map_err(classify_workflow)?;
            serde_json::json!(detail)
        }
        _ => {
            return Err(CliError::Workflow(
                "unexpected citation review response; refresh before continuing".into(),
            )
            .into())
        }
    };
    // JSON escaping retains complete exact Unicode/quote bytes without interpreting controls.
    Ok(Output {
        text: serde_json::to_string_pretty(&data).expect("safe DTO"),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, Invocation};

    #[test]
    fn direct_invalid_requests_do_not_open_workspace_or_credentials() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("task.credentials");
        for command in [
            NeedsReviewCommand::Citations(CitationReviewRequest {
                limit: 0,
                cursor: None,
            }),
            NeedsReviewCommand::Show(CitationReviewDetailRequest {
                path: "../outside.md".into(),
                expected_sha256: [0; 32],
            }),
        ] {
            let invocation = Invocation {
                json: true,
                data_dir: data.clone(),
                model_dir: None,
                vault: None,
                credentials_dir: Some(credentials.clone()),
                command: Command::NeedsReview(command),
            };
            let error = super::super::execute(&invocation)
                .err()
                .expect("invalid request refused");
            assert_eq!(error.error.code(), "USAGE");
            assert_eq!(std::fs::read_dir(&data).unwrap().count(), 0);
            assert!(!credentials.exists());
        }
    }
}
