//! Explicit input preparation and protocol mapping; originals are owned by AppWorker.
use super::{
    error::CliError, expect_positionals, parse_timeout, positional_uuid, scan, sub_word, usage,
    CliFailure, Globals, Output, Scanned, Tokens,
};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    inbox::{CaptureInboxRequest, InboxKind, InboxListRequest},
    inbox_actions::{InboxActionRequest, InboxAnalysisPurpose},
    inbox_processing::{InboxCandidateRequest, InboxSourceRequest, ProcessInboxRequest},
};
use std::{fmt::Write as _, path::PathBuf};
use uuid::Uuid;
pub enum InboxCommand {
    Add {
        id: Uuid,
        kind: InboxKind,
        title: String,
        original_name: Option<String>,
        input: PathBuf,
    },
    Show(Uuid),
    Review(Uuid),
    RemovalPreview(Uuid),
    List(InboxListRequest),
    Process(PathBuf),
    Processing(Uuid),
    Candidate(InboxCandidateRequest),
    Source(PathBuf),
    Cancel(Uuid),
    AnalyzeActions {
        input: PathBuf,
        timeout_seconds: u64,
    },
    ActionAnalysis(Uuid),
    Analyze {
        input: PathBuf,
        timeout_seconds: u64,
    },
    Analysis(Uuid),
}
impl InboxCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Add { .. } => "inbox.add",
            Self::Show(_) => "inbox.show",
            Self::Review(_) => "inbox.review",
            Self::RemovalPreview(_) => "inbox.removal-preview",
            Self::List(_) => "inbox.list",
            Self::Process(_) => "inbox.process",
            Self::Processing(_) => "inbox.processing",
            Self::Candidate(_) => "inbox.candidate",
            Self::Source(_) => "inbox.source",
            Self::Cancel(_) => "inbox.cancel",
            Self::AnalyzeActions { .. } => "inbox.analyze-actions",
            Self::ActionAnalysis(_) => "inbox.action-analysis",
            Self::Analyze { .. } => "inbox.analyze",
            Self::Analysis(_) => "inbox.analysis",
        }
    }
}
pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(
        tokens,
        "inbox",
        "add|show|review|removal-preview|list|process|processing|candidate|source|cancel|analyze-actions|action-analysis|analyze|analysis",
    )?;
    let (label, options): (_, &[(&str, bool)]) = match sub.as_str() {
        "add" => (
            "inbox.add",
            &[
                ("id", true),
                ("kind", true),
                ("title", true),
                ("original-name", true),
                ("file", true),
            ],
        ),
        "show" => ("inbox.show", &[]),
        "review" => ("inbox.review", &[]),
        "removal-preview" => ("inbox.removal-preview", &[]),
        "list" => ("inbox.list", &[("limit", true), ("after", true)]),
        "process" => ("inbox.process", &[("file", true)]),
        "processing" => ("inbox.processing", &[]),
        "candidate" => ("inbox.candidate", &[]),
        "source" => ("inbox.source", &[("file", true)]),
        "cancel" => ("inbox.cancel", &[]),
        "analyze-actions" => (
            "inbox.analyze-actions",
            &[("file", true), ("timeout-seconds", true)],
        ),
        "action-analysis" => ("inbox.action-analysis", &[]),
        "analyze" => (
            "inbox.analyze",
            &[("file", true), ("timeout-seconds", true)],
        ),
        "analysis" => ("inbox.analysis", &[]),
        _ => return Err(usage("unknown Inbox subcommand")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}
fn metadata(command: &InboxCommand) -> Result<(), CliError> {
    match command {
        InboxCommand::Add {
            id,
            kind,
            title,
            original_name,
            ..
        } => CaptureInboxRequest {
            id: *id,
            kind: *kind,
            title: title.clone(),
            original_name: original_name.clone(),
            text: String::new(),
        }
        .validate()
        .map_err(|e| usage(e.message)),
        InboxCommand::Show(id) | InboxCommand::Review(id) | InboxCommand::RemovalPreview(id)
            if id.is_nil() =>
        {
            Err(usage("Inbox UUID must not be nil"))
        }
        InboxCommand::Processing(id) | InboxCommand::Cancel(id) if id.is_nil() => {
            Err(usage("Inbox processing UUID must not be nil"))
        }
        InboxCommand::ActionAnalysis(id) | InboxCommand::Analysis(id) if id.is_nil() => {
            Err(usage("Inbox analysis UUID must not be nil"))
        }
        InboxCommand::AnalyzeActions {
            timeout_seconds, ..
        }
        | InboxCommand::Analyze {
            timeout_seconds, ..
        } if !(1..=3600).contains(timeout_seconds) => Err(usage(
            "analysis deadline must be between 1 and 3600 seconds",
        )),
        InboxCommand::Candidate(r) if r.batch_id.is_nil() || r.index >= 8 => {
            Err(usage("Inbox candidate needs a UUID and index 0 to 7"))
        }
        InboxCommand::List(r) => r.validate().map_err(|e| usage(e.to_string())),
        _ => Ok(()),
    }
}
pub(super) fn parse_command(name: &str, s: &Scanned) -> Result<InboxCommand, CliError> {
    let command = match name {
        "inbox.add" => {
            expect_positionals(s, 0)?;
            let kind = match s.value("kind").unwrap_or("text") {
                "text" => InboxKind::Text,
                "markdown" => InboxKind::Markdown,
                "email" => InboxKind::Email,
                "teams" => InboxKind::Teams,
                _ => return Err(usage("--kind must be text|markdown|email|teams")),
            };
            InboxCommand::Add {
                id: s.uuid("id")?.ok_or_else(|| usage("missing --id"))?,
                kind,
                title: s
                    .value("title")
                    .ok_or_else(|| usage("missing --title"))?
                    .into(),
                original_name: s.value("original-name").map(str::to_owned),
                input: PathBuf::from(s.value("file").ok_or_else(|| usage("missing --file"))?),
            }
        }
        "inbox.show" | "inbox.review" | "inbox.removal-preview" => {
            expect_positionals(s, 1)?;
            let id = positional_uuid(s, 0, "UUID")?;
            if name == "inbox.removal-preview" {
                InboxCommand::RemovalPreview(id)
            } else if name == "inbox.review" {
                InboxCommand::Review(id)
            } else {
                InboxCommand::Show(id)
            }
        }
        "inbox.list" => {
            expect_positionals(s, 0)?;
            InboxCommand::List(InboxListRequest {
                limit: s
                    .value("limit")
                    .map(|v| {
                        v.parse::<usize>()
                            .map_err(|_| usage("invalid --limit integer"))
                    })
                    .transpose()?
                    .unwrap_or(25),
                after: s.uuid("after")?,
            })
        }
        "inbox.process" => {
            expect_positionals(s, 0)?;
            InboxCommand::Process(PathBuf::from(
                s.value("file").ok_or_else(|| usage("missing --file"))?,
            ))
        }
        "inbox.source" => {
            expect_positionals(s, 0)?;
            InboxCommand::Source(PathBuf::from(
                s.value("file").ok_or_else(|| usage("missing --file"))?,
            ))
        }
        "inbox.processing" => {
            expect_positionals(s, 1)?;
            InboxCommand::Processing(positional_uuid(s, 0, "UUID")?)
        }
        "inbox.cancel" => {
            expect_positionals(s, 1)?;
            InboxCommand::Cancel(positional_uuid(s, 0, "UUID")?)
        }
        "inbox.action-analysis" | "inbox.analysis" => {
            expect_positionals(s, 1)?;
            let id = positional_uuid(s, 0, "UUID")?;
            if name == "inbox.analysis" {
                InboxCommand::Analysis(id)
            } else {
                InboxCommand::ActionAnalysis(id)
            }
        }
        "inbox.analyze-actions" | "inbox.analyze" => {
            expect_positionals(s, 0)?;
            let input = PathBuf::from(s.value("file").ok_or_else(|| usage("missing --file"))?);
            let timeout_seconds = s
                .value("timeout-seconds")
                .map(parse_timeout)
                .transpose()?
                .unwrap_or(300);
            if name == "inbox.analyze" {
                InboxCommand::Analyze {
                    input,
                    timeout_seconds,
                }
            } else {
                InboxCommand::AnalyzeActions {
                    input,
                    timeout_seconds,
                }
            }
        }
        "inbox.candidate" => {
            expect_positionals(s, 2)?;
            InboxCommand::Candidate(InboxCandidateRequest {
                batch_id: positional_uuid(s, 0, "UUID")?,
                index: s.positionals[1]
                    .parse()
                    .map_err(|_| usage("invalid candidate INDEX"))?,
            })
        }
        _ => unreachable!("scanned Inbox command"),
    };
    metadata(&command)?;
    Ok(command)
}
pub(super) fn prepare(command: &InboxCommand) -> Result<AppCommand, CliFailure> {
    metadata(command)?;
    Ok(match command {
        InboxCommand::Add {
            id,
            kind,
            title,
            original_name,
            input,
        } => {
            let request = CaptureInboxRequest {
                id: *id,
                kind: *kind,
                title: title.clone(),
                original_name: original_name.clone(),
                text: super::input::read_text_file(input, "Inbox original")?,
            };
            request
                .validate()
                .map_err(super::error::classify_workflow)?;
            AppCommand::CaptureInbox(request)
        }
        InboxCommand::Show(id) => AppCommand::InboxItem(*id),
        InboxCommand::Review(id) => AppCommand::InboxReview(*id),
        InboxCommand::RemovalPreview(id) => AppCommand::PreviewInboxRemoval(*id),
        InboxCommand::List(r) => AppCommand::InboxItems(r.clone()),
        InboxCommand::Process(file) => {
            let text = super::input::read_text_file(file, "Inbox processing request")?;
            let request: ProcessInboxRequest = serde_json::from_str(&text)
                .map_err(|_| usage("invalid Inbox processing request JSON"))?;
            request.validate().map_err(|e| usage(e.to_string()))?;
            AppCommand::ProcessInbox(request)
        }
        InboxCommand::Processing(id) => AppCommand::InboxProcessing(*id),
        InboxCommand::Candidate(r) => AppCommand::InboxCandidate(r.clone()),
        InboxCommand::Source(file) => {
            let text = super::input::read_text_file(file, "Inbox source request")?;
            let request: InboxSourceRequest = serde_json::from_str(&text)
                .map_err(|_| usage("invalid Inbox source request JSON"))?;
            request.validate().map_err(|e| usage(e.to_string()))?;
            AppCommand::PrepareInboxSource(request)
        }
        InboxCommand::Cancel(id) => AppCommand::CancelInboxProcessing(*id),
        InboxCommand::AnalyzeActions { input, .. } | InboxCommand::Analyze { input, .. } => {
            let request: InboxActionRequest = super::proposals::input(input)?;
            let required = if matches!(command, InboxCommand::Analyze { .. }) {
                InboxAnalysisPurpose::KnowledgeAndActions
            } else {
                InboxAnalysisPurpose::Actions
            };
            if request.purpose != required {
                return Err(usage(match required {
                    InboxAnalysisPurpose::Actions => {
                        "inbox analyze-actions requires purpose actions (or its omitted default)"
                    }
                    InboxAnalysisPurpose::KnowledgeAndActions => {
                        "inbox analyze requires explicit purpose knowledge_and_actions"
                    }
                })
                .into());
            }
            request
                .validate()
                .map_err(super::error::classify_workflow)?;
            AppCommand::AnalyzeInboxActions(Box::new(request))
        }
        InboxCommand::ActionAnalysis(id) | InboxCommand::Analysis(id) => {
            AppCommand::InboxActionAnalysis(*id)
        }
    })
}
pub(super) fn output(command: &AppCommand, event: AppEvent) -> Result<Output, CliFailure> {
    let data = match (command, event) {
        (AppCommand::CaptureInbox(r), AppEvent::InboxCaptured(item))
            if r.validate_receipt(&item).is_ok() =>
        {
            serde_json::json!(*item)
        }
        (AppCommand::InboxItem(id), AppEvent::InboxItem(item)) if item.item.capture.id == *id => {
            serde_json::json!(*item)
        }
        (AppCommand::InboxReview(id), AppEvent::InboxReview(review))
            if review.manifest.original.capture.id == *id
                && review.needs_semantic_review
                && review
                    .manifest
                    .digest()
                    .is_ok_and(|digest| digest == review.digest) =>
        {
            serde_json::json!(*review)
        }
        (AppCommand::PreviewInboxRemoval(id), AppEvent::InboxRemovalPreview(preview))
            if preview.evidence.snapshot.review.original.capture.id == *id
                && preview.evidence.needs_owner_attestation
                && preview
                    .evidence
                    .digest()
                    .is_ok_and(|digest| digest == preview.digest) =>
        {
            serde_json::json!(*preview)
        }
        (AppCommand::InboxItems(r), AppEvent::InboxItems(page))
            if page.entries.len() <= r.limit =>
        {
            serde_json::json!(*page)
        }
        (AppCommand::ProcessInbox(r), AppEvent::InboxProcessing(batch))
            if batch.request == *r && batch.pending_count() == 0 =>
        {
            serde_json::json!(*batch)
        }
        (
            AppCommand::InboxProcessing(id) | AppCommand::CancelInboxProcessing(id),
            AppEvent::InboxProcessing(batch),
        ) if batch.request.id == *id => serde_json::json!(*batch),
        (AppCommand::InboxCandidate(r), AppEvent::InboxCandidate(preview))
            if preview.request == *r =>
        {
            serde_json::json!(*preview)
        }
        (AppCommand::PrepareInboxSource(r), AppEvent::InboxSourceDraft(draft))
            if r.validate_draft(&draft).is_ok() =>
        {
            serde_json::json!(*draft)
        }
        (AppCommand::InboxActionAnalysis(id), AppEvent::InboxActionAnalysis(analysis))
            if analysis.job.capture.id == *id =>
        {
            serde_json::json!(*analysis)
        }
        _ => {
            return Err(CliError::Workflow(
                "unexpected application reply for Inbox command".into(),
            )
            .into());
        }
    };
    let pretty = serde_json::to_string_pretty(&data).expect("Inbox DTO serializes");
    let mut text = String::with_capacity(pretty.len() + 1);
    for ch in pretty.chars() {
        if ch.is_control() && ch >= '\u{007f}' {
            write!(&mut text, "\\u{:04x}", ch as u32).expect("write to String");
        } else {
            text.push(ch);
        }
    }
    text.push('\n');
    Ok(Output { text, data })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, Invocation, Outcome};
    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|t| (*t).into()).collect()
    }
    #[test]
    fn bounded_parser_rejects_invalid_intake_metadata_and_paging() {
        let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data_path = data.path().to_str().unwrap();
        let id = Uuid::new_v4().to_string();
        for tokens in [
            vec![
                "inbox", "add", "--id", &id, "--title", "Exact õ", "--file", "copy.txt",
            ],
            vec!["inbox", "show", &id],
            vec!["inbox", "review", &id],
            vec!["inbox", "removal-preview", &id],
            vec!["inbox", "process", "--file", "request.json"],
            vec!["inbox", "processing", &id],
            vec!["inbox", "candidate", &id, "0"],
            vec!["inbox", "source", "--file", "request.json"],
            vec!["inbox", "cancel", &id],
            vec![
                "inbox",
                "analyze-actions",
                "--file",
                "analysis.json",
                "--timeout-seconds",
                "9",
            ],
            vec!["inbox", "action-analysis", &id],
            vec![
                "inbox",
                "analyze",
                "--file",
                "analysis.json",
                "--timeout-seconds",
                "11",
            ],
            vec!["inbox", "analysis", &id],
            vec!["inbox", "list", "--after", &id, "--limit", "100"],
        ] {
            let mut a = args(&tokens);
            a.extend(args(&["--data-dir", data_path]));
            assert!(matches!(crate::cli::parse(&a), Ok(Outcome::Run(_))));
        }
        for tokens in [
            vec![
                "inbox", "add", "--id", &id, "--title", "x", "--kind", "pdf", "--file", "x",
            ],
            vec!["inbox", "add", "--id", &id, "--title", " \t", "--file", "x"],
            vec!["inbox", "list", "--limit", "101"],
            vec![
                "inbox",
                "list",
                "--after",
                "00000000-0000-0000-0000-000000000000",
            ],
            vec!["inbox", "show", "../outside"],
            vec!["inbox", "review", "00000000-0000-0000-0000-000000000000"],
            vec!["inbox", "candidate", &id, "8"],
            vec!["inbox", "candidate", &id, "-1"],
            vec!["inbox", "process"],
            vec!["inbox", "source"],
            vec!["inbox", "source", "unexpected", "--file", "request.json"],
            vec!["inbox", "analyze-actions"],
            vec!["inbox", "analyze"],
            vec!["inbox", "analyze", "unexpected", "--file", "analysis.json"],
            vec![
                "inbox",
                "analyze",
                "--file",
                "analysis.json",
                "--purpose",
                "knowledge_and_actions",
            ],
            vec![
                "inbox",
                "analyze",
                "--file",
                "analysis.json",
                "--timeout-seconds",
                "3601",
            ],
            vec![
                "inbox",
                "analyze-actions",
                "--file",
                "analysis.json",
                "--timeout-seconds",
                "0",
            ],
            vec![
                "inbox",
                "action-analysis",
                "00000000-0000-0000-0000-000000000000",
            ],
            vec!["inbox", "analysis", "00000000-0000-0000-0000-000000000000"],
            vec!["inbox", "analysis", "not-a-uuid"],
        ] {
            let mut a = args(&tokens);
            a.extend(args(&["--data-dir", data_path]));
            assert!(crate::cli::parse(&a).is_err());
        }
    }
    #[test]
    fn semantic_parser_preserves_command_names_and_explicit_deadline() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().to_str().unwrap();
        let id = Uuid::new_v4();
        for seconds in [None, Some("17")] {
            let mut tokens = args(&[
                "inbox",
                "analyze",
                "--file",
                "analysis.json",
                "--data-dir",
                data,
            ]);
            if let Some(value) = seconds {
                tokens.extend(args(&["--timeout-seconds", value]));
            }
            let Outcome::Run(invocation) = crate::cli::parse(&tokens)
                .unwrap_or_else(|_| panic!("semantic analysis arguments"))
            else {
                panic!("semantic analysis invocation");
            };
            let Command::Inbox(command) = invocation.command else {
                panic!("Inbox command");
            };
            assert_eq!(command.name(), "inbox.analyze");
            let InboxCommand::Analyze {
                input,
                timeout_seconds,
            } = command
            else {
                panic!("semantic analysis variant");
            };
            assert_eq!(input, PathBuf::from("analysis.json"));
            assert_eq!(timeout_seconds, if seconds.is_some() { 17 } else { 300 });
        }
        let tokens = args(&["inbox", "analysis", &id.to_string(), "--data-dir", data]);
        let Outcome::Run(invocation) =
            crate::cli::parse(&tokens).unwrap_or_else(|_| panic!("semantic inspection arguments"))
        else {
            panic!("semantic inspection invocation");
        };
        assert!(
            matches!(invocation.command, Command::Inbox(InboxCommand::Analysis(actual)) if actual == id)
        );
    }
    #[test]
    fn invalid_direct_requests_and_input_files_leave_application_state_unopened() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("credentials");
        let invalid = owner.path().join("invalid.txt");
        std::fs::write(&invalid, [0xff]).unwrap();
        let invalid_json = owner.path().join("invalid.json");
        std::fs::write(&invalid_json, "{\"id\":\"not-an-id\",\"items\":[]}").unwrap();
        let oversized = owner.path().join("oversized.txt");
        std::fs::write(&oversized, vec![b'x'; brn_workflow::MAX_NOTE_BYTES + 1]).unwrap();
        let make = |input| InboxCommand::Add {
            id: Uuid::new_v4(),
            kind: InboxKind::Text,
            title: "Explicit".into(),
            original_name: None,
            input,
        };
        for command in [
            InboxCommand::Show(Uuid::nil()),
            InboxCommand::RemovalPreview(Uuid::nil()),
            InboxCommand::Source(invalid_json.clone()),
            InboxCommand::Process(invalid_json.clone()),
            InboxCommand::AnalyzeActions {
                input: invalid_json.clone(),
                timeout_seconds: 300,
            },
            InboxCommand::Analyze {
                input: invalid_json.clone(),
                timeout_seconds: 300,
            },
            InboxCommand::Analyze {
                input: invalid_json.clone(),
                timeout_seconds: 0,
            },
            InboxCommand::AnalyzeActions {
                input: invalid_json,
                timeout_seconds: 0,
            },
            InboxCommand::ActionAnalysis(Uuid::nil()),
            InboxCommand::Analysis(Uuid::nil()),
            InboxCommand::List(InboxListRequest {
                limit: 0,
                after: None,
            }),
            make(invalid),
            make(oversized),
            make(owner.path().to_owned()),
            make(owner.path().join("absent")),
        ] {
            let i = Invocation {
                json: true,
                data_dir: data.clone(),
                vault: None,
                credentials_dir: Some(credentials.clone()),
                model_dir: None,
                command: Command::Inbox(command),
            };
            assert!(crate::cli::execute(&i).is_err());
            assert_eq!(std::fs::read_dir(&data).unwrap().count(), 0);
            assert!(!credentials.exists());
        }
    }
    #[test]
    fn analysis_purpose_mismatch_or_omission_refuses_before_application_open_without_rewriting_input(
    ) {
        let _cancel = crate::tests::CancelTestGuard::with(false);
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("credentials");
        let input = owner.path().join("analysis.json");
        for (semantic, purpose) in [
            (true, None),
            (true, Some("actions")),
            (false, Some("knowledge_and_actions")),
            (true, Some("unknown")),
        ] {
            // These complete DTO-shaped bytes isolate the purpose refusal. Their
            // source is deliberately unusable, so no admission/provider is possible.
            let mut value = serde_json::json!({
                "id": Uuid::new_v4(),
                "conversation": null,
                "source": {
                    "source": {
                        "path": "source.md",
                        "fingerprint": {"device": 1, "inode": 2, "len": 0, "sha256": vec![0u8; 32]}
                    },
                    "text": ""
                },
                "selection": {"provider": "chatgpt", "model": "gpt-6-luna"},
                "effort": "medium",
                "generation": 5
            });
            if let Some(purpose) = purpose {
                value["purpose"] = serde_json::json!(purpose);
            }
            let exact = serde_json::to_vec(&value).unwrap();
            std::fs::write(&input, &exact).unwrap();
            let command = if semantic {
                InboxCommand::Analyze {
                    input: input.clone(),
                    timeout_seconds: 30,
                }
            } else {
                InboxCommand::AnalyzeActions {
                    input: input.clone(),
                    timeout_seconds: 30,
                }
            };
            let invocation = Invocation {
                json: true,
                data_dir: data.clone(),
                vault: None,
                credentials_dir: Some(credentials.clone()),
                model_dir: None,
                command: Command::Inbox(command),
            };
            let error = crate::cli::execute(&invocation).err().unwrap();
            assert_eq!(error.error.code(), "USAGE");
            if purpose != Some("unknown") {
                assert!(error.error.message().contains("purpose"));
            }
            assert_eq!(std::fs::read_dir(&data).unwrap().count(), 0);
            assert!(!credentials.exists());
            assert_eq!(std::fs::read(&input).unwrap(), exact);
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn source_dispatch_returns_a_complete_bound_draft_and_rejects_forged_replies() {
        let _cancel = crate::tests::CancelTestGuard::with(false);
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        std::fs::create_dir(&data).unwrap();
        std::fs::create_dir(&vault).unwrap();
        let credentials = owner.path().join("credentials");
        let invocation = |command| Invocation {
            json: true,
            data_dir: data.clone(),
            vault: Some(vault.clone()),
            credentials_dir: Some(credentials.clone()),
            model_dir: None,
            command: Command::Inbox(command),
        };
        let original = owner.path().join("copy.txt");
        let exact = "\u{feff}From: synthetic@example.invalid\r\n\r\n````\nBody õ 日本語\0";
        std::fs::write(&original, exact).unwrap();
        let captured = crate::cli::execute(&invocation(InboxCommand::Add {
            id: Uuid::new_v4(),
            kind: InboxKind::Email,
            title: "Exact source".into(),
            original_name: Some("message.eml".into()),
            input: original.clone(),
        }))
        .unwrap();
        let process = ProcessInboxRequest {
            id: Uuid::new_v4(),
            items: vec![serde_json::from_value(captured.data.clone()).unwrap()],
        };
        let process_file = owner.path().join("process.json");
        std::fs::write(&process_file, serde_json::to_vec(&process).unwrap()).unwrap();
        crate::cli::execute(&invocation(InboxCommand::Process(process_file))).unwrap();
        let request = InboxSourceRequest {
            candidate: InboxCandidateRequest {
                batch_id: process.id,
                index: 0,
            },
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "source.md".into(),
            title: "Review this original".into(),
        };
        let request_file = owner.path().join("source.json");
        std::fs::write(&request_file, serde_json::to_vec(&request).unwrap()).unwrap();
        let source_args = args(&[
            "inbox",
            "source",
            "--file",
            request_file.to_str().unwrap(),
            "--data-dir",
            data.to_str().unwrap(),
            "--vault",
            vault.to_str().unwrap(),
            "--credentials-dir",
            credentials.to_str().unwrap(),
        ]);
        let Outcome::Run(parsed) =
            crate::cli::parse(&source_args).unwrap_or_else(|_| panic!("source arguments parse"))
        else {
            panic!("source invocation");
        };
        assert!(matches!(&parsed.command, Command::Inbox(c) if c.name() == "inbox.source"));
        let result = crate::cli::execute(&parsed).unwrap();
        let draft: brn_workflow::proposals::DraftRequest =
            serde_json::from_value(result.data).unwrap();
        request.validate_draft(&draft).unwrap();
        let binding = draft.inbox_source.as_ref().unwrap();
        assert_eq!(binding.original, process.items[0]);
        assert!(
            matches!(&draft.changes[0], brn_workflow::proposals::DraftNoteChange::Create { text, .. } if text.contains(exact))
        );
        let command = AppCommand::PrepareInboxSource(request);
        assert!(output(
            &command,
            AppEvent::InboxSourceDraft(Box::new(draft.clone()))
        )
        .is_ok());
        for field in ["id", "group_id", "title", "inbox_source", "changes"] {
            let mut forged = serde_json::to_value(&draft).unwrap();
            forged[field] = match field {
                "id" | "group_id" => serde_json::json!(Uuid::new_v4()),
                "title" => serde_json::json!("Another title"),
                "inbox_source" => serde_json::Value::Null,
                "changes" => serde_json::json!([]),
                _ => unreachable!(),
            };
            let forged = serde_json::from_value(forged).unwrap();
            assert!(output(&command, AppEvent::InboxSourceDraft(Box::new(forged))).is_err());
        }
        assert!(!vault.join("source.md").exists());
        assert_eq!(std::fs::read(original).unwrap(), exact.as_bytes());
        assert_eq!(std::fs::read_dir(&credentials).unwrap().count(), 0);

        // Qualify new CLI full-proof export, retained-analysis inspection and
        // exact replay without a provider route, current choice or saved Source.
        let draft_file = owner.path().join("draft.json");
        std::fs::write(&draft_file, serde_json::to_vec(&draft).unwrap()).unwrap();
        let proposal_invocation = |command| Invocation {
            json: true,
            data_dir: data.clone(),
            vault: Some(vault.clone()),
            credentials_dir: Some(credentials.clone()),
            model_dir: None,
            command: Command::Proposals(command),
        };
        let created = crate::cli::execute(&proposal_invocation(
            super::super::proposals::ProposalCommand::Create(draft_file),
        ))
        .unwrap();
        let review: brn_workflow::proposals::ProposalRecord =
            serde_json::from_value(created.data).unwrap();
        crate::cli::execute(&proposal_invocation(
            super::super::proposals::ProposalCommand::Approve(
                brn_workflow::proposal_apply::ApprovalRequest {
                    operation_id: Uuid::new_v4(),
                    expected: review.stamp(),
                },
            ),
        ))
        .unwrap();
        let exported = crate::cli::execute(&proposal_invocation(
            super::super::proposals::ProposalCommand::Source("source.md".into()),
        ))
        .unwrap();
        let source: brn_workflow::proposals::ProposalSource =
            serde_json::from_value(exported.data).unwrap();
        source.validate().unwrap();
        assert!(source.text.contains(exact));
        let result = crate::cli::execute(&invocation(InboxCommand::RemovalPreview(
            process.items[0].capture.id,
        )))
        .unwrap();
        let preview: brn_workflow::inbox_removal::InboxRemovalPreview =
            serde_json::from_value(result.data).unwrap();
        assert!(preview.evidence.needs_owner_attestation);
        assert!(preview.evidence.blockers.is_empty());
        assert_eq!(preview.evidence.sources[0].saved, source);
        let command = AppCommand::PreviewInboxRemoval(process.items[0].capture.id);
        assert!(output(
            &command,
            AppEvent::InboxRemovalPreview(Box::new(preview.clone()))
        )
        .is_ok());
        let mut forged = preview.clone();
        forged.digest[0] ^= 1;
        assert!(output(&command, AppEvent::InboxRemovalPreview(Box::new(forged))).is_err());
        assert!(output(
            &AppCommand::PreviewInboxRemoval(Uuid::new_v4()),
            AppEvent::InboxRemovalPreview(Box::new(preview))
        )
        .is_err());
        std::fs::remove_file(vault.join("source.md")).unwrap();
        for purpose in [
            InboxAnalysisPurpose::Actions,
            InboxAnalysisPurpose::KnowledgeAndActions,
        ] {
            let request = InboxActionRequest {
                purpose,
                id: Uuid::new_v4(),
                conversation: None,
                source: Box::new(source.clone()),
                selection: brn_workflow::Selection {
                    provider: brn_workflow::Provider::Chatgpt,
                    model: "gpt-6-luna".into(),
                },
                effort: brn_workflow::ReasoningEffort::Medium,
                generation: 77,
            };
            let capture = brn_workflow::inbox_actions::InboxActionCapture {
                purpose,
                id: request.id,
                conversation: None,
                source: source.source.clone(),
                source_text: source.text.clone(),
                provider: "chatgpt".into(),
                model: request.selection.model.clone(),
                effort: "medium".into(),
            };
            {
                let (mut store, _) = brn_store::WorkStore::open(&data).unwrap();
                let job = store
                    .reserve_inbox_action(&capture, "Synthetic retained analysis question")
                    .unwrap();
                store.begin_inbox_action_turn(&job).unwrap();
                store
                    .finish_turn(
                        request.id,
                        brn_workflow::WorkTurnStatus::Completed,
                        "No Action supported; other consequences still need review.",
                        None,
                    )
                    .unwrap();
            }
            let analysis_file = owner.path().join("analysis.json");
            let mut json = serde_json::to_value(&request).unwrap();
            if purpose == InboxAnalysisPurpose::Actions {
                json.as_object_mut().unwrap().remove("purpose");
            }
            std::fs::write(&analysis_file, serde_json::to_vec(&json).unwrap()).unwrap();
            let analyze_command = |input| {
                if purpose == InboxAnalysisPurpose::Actions {
                    InboxCommand::AnalyzeActions {
                        input,
                        timeout_seconds: 30,
                    }
                } else {
                    InboxCommand::Analyze {
                        input,
                        timeout_seconds: 30,
                    }
                }
            };
            let AppCommand::AnalyzeInboxActions(prepared) =
                prepare(&analyze_command(analysis_file.clone())).unwrap()
            else {
                panic!("prepared bound analysis");
            };
            assert_eq!(*prepared, request);
            let replay =
                crate::cli::execute(&invocation(analyze_command(analysis_file.clone()))).unwrap();
            assert_eq!(replay.data["operation_id"], serde_json::json!(request.id));
            assert_eq!(replay.data["status"], "completed");
            let inspected =
                crate::cli::execute(&invocation(InboxCommand::Analysis(request.id))).unwrap();
            let analysis: brn_workflow::inbox_actions::InboxActionAnalysis =
                serde_json::from_value(inspected.data.clone()).unwrap();
            assert_eq!(analysis.job.capture, capture);
            assert_eq!(analysis.job.capture.purpose, purpose);
            assert_eq!(
                analysis.job.question,
                "Synthetic retained analysis question"
            );
            assert_eq!(
                analysis.turn.unwrap().status,
                brn_workflow::WorkTurnStatus::Completed
            );
            assert!(analysis.proposals.is_empty());
            assert!(analysis.needs_semantic_review);
            assert_eq!(
                crate::cli::execute(&invocation(InboxCommand::ActionAnalysis(request.id)))
                    .unwrap()
                    .data,
                inspected.data
            );
            let mut new_generation = request.clone();
            new_generation.generation += 91;
            std::fs::write(&analysis_file, serde_json::to_vec(&new_generation).unwrap()).unwrap();
            assert_eq!(
                crate::cli::execute(&invocation(analyze_command(analysis_file.clone())))
                    .unwrap()
                    .data,
                replay.data
            );
            assert_eq!(
                crate::cli::execute(&invocation(InboxCommand::Analysis(request.id)))
                    .unwrap()
                    .data,
                inspected.data
            );
            let mut changed = request;
            changed.effort = brn_workflow::ReasoningEffort::High;
            std::fs::write(&analysis_file, serde_json::to_vec(&changed).unwrap()).unwrap();
            let failure = crate::cli::execute(&invocation(analyze_command(analysis_file)))
                .err()
                .unwrap();
            assert_eq!(failure.error.code(), "OPERATION_CONFLICT");
            assert_eq!(std::fs::read_dir(&credentials).unwrap().count(), 0);
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn actual_cli_owner_copy_show_replay_and_paged_inventory_preserve_exact_input() {
        let _cancel = crate::tests::CancelTestGuard::with(false);
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("credentials");
        let input = owner.path().join("copy.txt");
        let exact = "\u{feff}quoted õ\r\n\u{009b}sentences 日本語";
        std::fs::write(&input, exact).unwrap();
        let id = Uuid::new_v4();
        let invocation = |command| Invocation {
            json: true,
            data_dir: data.clone(),
            vault: None,
            credentials_dir: Some(credentials.clone()),
            model_dir: None,
            command: Command::Inbox(command),
        };
        let add = || InboxCommand::Add {
            id,
            kind: InboxKind::Email,
            title: "Email õ".into(),
            original_name: Some("../../label.eml".into()),
            input: input.clone(),
        };
        let first = crate::cli::execute(&invocation(add())).unwrap();
        assert_eq!(
            crate::cli::execute(&invocation(add())).unwrap().data,
            first.data
        );
        let shown = crate::cli::execute(&invocation(InboxCommand::Show(id))).unwrap();
        assert_eq!(shown.data["item"], first.data);
        assert_eq!(shown.data["original"]["text"], exact);
        let review = crate::cli::execute(&invocation(InboxCommand::Review(id))).unwrap();
        assert_eq!(review.data["manifest"]["original"], first.data);
        assert_eq!(review.data["original"]["text"], exact);
        assert_eq!(review.data["needs_semantic_review"], true);
        assert_eq!(review.data["manifest"]["analyses"], serde_json::json!([]));
        assert!(!review.text.contains('\u{009b}'));
        assert!(review.text.contains("\\u009b"));

        assert!(!shown.text.contains('\u{009b}'));
        assert!(shown.text.contains("\\u009b"));
        let listed =
            crate::cli::execute(&invocation(InboxCommand::List(InboxListRequest::default())))
                .unwrap();
        assert_eq!(listed.data["total_count"], 1);
        assert_eq!(listed.data["entries"][0]["availability"], "available");
        assert_eq!(listed.data["issues"], serde_json::json!([]));
        let process = ProcessInboxRequest {
            id: Uuid::new_v4(),
            items: vec![serde_json::from_value(first.data.clone()).unwrap()],
        };
        let input_json = owner.path().join("request.json");
        std::fs::write(&input_json, serde_json::to_vec(&process).unwrap()).unwrap();
        let result =
            crate::cli::execute(&invocation(InboxCommand::Process(input_json.clone()))).unwrap();
        assert_eq!(result.data["entries"][0]["outcome"]["state"], "converted");
        let processed_review = crate::cli::execute(&invocation(InboxCommand::Review(id))).unwrap();
        assert_eq!(
            processed_review.data["manifest"]["processing"][0],
            result.data
        );
        assert_ne!(processed_review.data["digest"], review.data["digest"]);
        assert_eq!(processed_review.data["needs_semantic_review"], true);

        assert_eq!(
            crate::cli::execute(&invocation(InboxCommand::Process(input_json)))
                .unwrap()
                .data,
            result.data
        );
        assert_eq!(
            crate::cli::execute(&invocation(InboxCommand::Processing(process.id)))
                .unwrap()
                .data,
            result.data
        );
        let preview = crate::cli::execute(&invocation(InboxCommand::Candidate(
            InboxCandidateRequest {
                batch_id: process.id,
                index: 0,
            },
        )))
        .unwrap();
        assert!(preview.data["markdown"].as_str().unwrap().contains(exact));
        assert_eq!(preview.data["needs_semantic_review"], true);
        assert!(!preview.text.contains('\u{009b}'));
        assert_eq!(
            crate::cli::execute(&invocation(InboxCommand::Cancel(process.id)))
                .unwrap()
                .data,
            result.data
        );
        assert_eq!(std::fs::read(input).unwrap(), exact.as_bytes());
        assert_eq!(std::fs::read_dir(credentials).unwrap().count(), 0);
        assert!(!data.join("index.sqlite").exists());
    }
}
