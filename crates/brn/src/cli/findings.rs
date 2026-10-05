//! Tentative retained review work through the shared application worker.
use super::{
    error::CliError, expect_positionals, parse_scope, positional_uuid, required_positional, scan,
    sub_word, usage, CliFailure, Globals, Output, Scanned, Tokens,
};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    findings::{
        CaptureFindingRequest, CloseFindingRequest, FindingInspection, FindingListRequest,
        FindingOrigin, FindingRecord, FindingStamp, FindingState, NoteConflictCursor,
        NoteConflictRequest,
    },
    library::KnowledgeScope,
    proposals::SourceVersion,
    vault::{EvidencePath, VaultPath},
};
use std::{fmt::Write as _, fs::File, io::Read, os::unix::fs::OpenOptionsExt, path::Path};
use uuid::Uuid;

pub enum FindingsCommand {
    Capture(CaptureFindingRequest),
    List(FindingListRequest),
    Show(Uuid),
    Inspect(Uuid),
    Close(CloseFindingRequest),
    Conflicts(NoteConflictRequest),
}
impl FindingsCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Capture(_) => "findings.capture",
            Self::List(_) => "findings.list",
            Self::Show(_) => "findings.show",
            Self::Inspect(_) => "findings.inspect",
            Self::Close(_) => "findings.close",
            Self::Conflicts(_) => "findings.conflicts",
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
        "findings",
        "capture|list|show|inspect|close|conflicts",
    )?;
    let (label, options): (_, &[(&str, bool)]) = match sub.as_str() {
        "capture" => ("findings.capture", &[("file", true)]),
        "list" => (
            "findings.list",
            &[("state", true), ("limit", true), ("before", true)],
        ),
        "show" => ("findings.show", &[]),
        "inspect" => ("findings.inspect", &[]),
        "close" => ("findings.close", &[("version", true), ("state", true)]),
        "conflicts" => (
            "findings.conflicts",
            &[("scope", true), ("limit", true), ("cursor", true)],
        ),
        _ => return Err(usage("unknown findings subcommand")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}

fn input(path: &Path) -> Result<CaptureFindingRequest, CliError> {
    const MAX_JSON_BYTES: usize = 64 * 1024;
    let io = |error: std::io::Error| CliError::Workflow(error.to_string());
    let mut file = File::options()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(io)?;
    let meta = file.metadata().map_err(io)?;
    if !meta.is_file() || meta.len() > MAX_JSON_BYTES as u64 {
        return Err(usage(
            "finding capture requires a regular JSON file up to 64 KiB",
        ));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_JSON_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(usage("finding capture JSON exceeds 64 KiB"));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| usage("finding capture does not match its typed JSON schema"))
}

fn state(raw: &str) -> Result<FindingState, CliError> {
    match raw {
        "open" => Ok(FindingState::Open),
        "resolved" => Ok(FindingState::Resolved),
        "dismissed" => Ok(FindingState::Dismissed),
        _ => Err(usage("--state must be open|resolved|dismissed|all for listing, or resolved|dismissed for closure")),
    }
}

fn cursor(raw: &str) -> Result<NoteConflictCursor, CliError> {
    if raw.len() > 8192 {
        return Err(usage("conflict cursor JSON exceeds 8192 bytes"));
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fingerprint {
        device: u64,
        inode: u64,
        len: u64,
        sha256: [u8; 32],
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Source {
        path: String,
        fingerprint: Fingerprint,
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Cursor {
        vault_id: Uuid,
        path: String,
        note_id: Uuid,
        scope: KnowledgeScope,
        source: Source,
        before: Uuid,
    }
    let cursor: Cursor = serde_json::from_str(raw)
        .map_err(|_| usage("conflict cursor does not match its typed JSON schema"))?;
    let proof = cursor.source.fingerprint;
    Ok(NoteConflictCursor {
        vault_id: cursor.vault_id,
        path: cursor.path,
        note_id: cursor.note_id,
        scope: cursor.scope,
        source: SourceVersion {
            path: cursor.source.path,
            fingerprint: brn_workflow::editor::FileFingerprint {
                device: proof.device,
                inode: proof.inode,
                len: proof.len,
                sha256: proof.sha256,
            },
        },
        before: cursor.before,
    })
}

pub(super) fn validate(command: &FindingsCommand) -> Result<(), CliError> {
    match command {
        FindingsCommand::Capture(request) => {
            request.validate().map_err(|error| usage(error.to_string()))
        }
        FindingsCommand::List(request) => {
            request.validate().map_err(|error| usage(error.to_string()))
        }
        FindingsCommand::Close(request) => {
            request.validate().map_err(|error| usage(error.to_string()))
        }
        FindingsCommand::Conflicts(request) => {
            if !(1..=100).contains(&request.limit) || request.path.len() > 512 {
                return Err(usage(
                    "conflict lookup needs a path up to 512 bytes and --limit 1..100",
                ));
            }
            match request.scope {
                KnowledgeScope::Current => VaultPath::parse(&request.path).map(|_| ()),
                _ => EvidencePath::parse(&request.path).map(|_| ()),
            }
            .map_err(|error| usage(error.to_string()))?;
            if let Some(cursor) = &request.cursor {
                if cursor.vault_id.is_nil()
                    || cursor.note_id.is_nil()
                    || cursor.before.is_nil()
                    || cursor.path != request.path
                    || cursor.scope != request.scope
                    || cursor.source.path != request.path
                    || cursor.source.fingerprint.len > 1024 * 1024
                    || serde_json::to_vec(cursor)
                        .map_err(|_| usage("invalid conflict cursor"))?
                        .len()
                        > 8192
                {
                    return Err(usage("conflict cursor must bind this path and scope with bounded exact proof and nonnil identities"));
                }
            }
            Ok(())
        }
        FindingsCommand::Show(id) | FindingsCommand::Inspect(id) => {
            if id.is_nil() {
                Err(usage("finding UUID must not be nil"))
            } else {
                Ok(())
            }
        }
    }
}

pub(super) fn parse_command(name: &str, scanned: &Scanned) -> Result<FindingsCommand, CliError> {
    let command = match name {
        "findings.capture" => {
            expect_positionals(scanned, 0)?;
            let file = scanned
                .value("file")
                .ok_or_else(|| usage("missing --file"))?;
            FindingsCommand::Capture(input(Path::new(file))?)
        }
        "findings.list" => {
            expect_positionals(scanned, 0)?;
            FindingsCommand::List(FindingListRequest {
                state: match scanned.value("state") {
                    Some("all") => None,
                    raw => Some(state(raw.unwrap_or("open"))?),
                },
                limit: scanned
                    .value("limit")
                    .map(|raw| {
                        raw.parse::<usize>()
                            .map_err(|_| usage("invalid --limit integer"))
                    })
                    .transpose()?
                    .unwrap_or(25),
                before: scanned.uuid("before")?,
            })
        }
        "findings.show" | "findings.inspect" => {
            expect_positionals(scanned, 1)?;
            let id = positional_uuid(scanned, 0, "UUID")?;
            if name == "findings.show" {
                FindingsCommand::Show(id)
            } else {
                FindingsCommand::Inspect(id)
            }
        }
        "findings.close" => {
            expect_positionals(scanned, 1)?;
            FindingsCommand::Close(CloseFindingRequest {
                expected: FindingStamp {
                    id: positional_uuid(scanned, 0, "UUID")?,
                    version: scanned.require_generation("version")?,
                },
                state: state(
                    scanned
                        .value("state")
                        .ok_or_else(|| usage("missing --state"))?,
                )?,
            })
        }
        "findings.conflicts" => {
            expect_positionals(scanned, 1)?;
            FindingsCommand::Conflicts(NoteConflictRequest {
                path: required_positional(scanned, "PATH.md")?.to_owned(),
                scope: parse_scope(scanned.value("scope"))?,
                limit: scanned
                    .value("limit")
                    .map(|raw| {
                        raw.parse::<usize>()
                            .map_err(|_| usage("invalid --limit integer"))
                    })
                    .transpose()?
                    .unwrap_or(10),
                cursor: scanned.value("cursor").map(cursor).transpose()?,
            })
        }
        _ => unreachable!("scanned findings command"),
    };
    validate(&command)?;
    Ok(command)
}

pub(super) fn prepare(command: &FindingsCommand) -> Result<AppCommand, CliFailure> {
    validate(command)?;
    Ok(match command {
        FindingsCommand::Capture(request) => AppCommand::CaptureFinding(request.clone()),
        FindingsCommand::List(request) => AppCommand::Findings(request.clone()),
        FindingsCommand::Show(id) => AppCommand::Finding(*id),
        FindingsCommand::Inspect(id) => AppCommand::InspectFinding(*id),
        FindingsCommand::Close(request) => AppCommand::CloseFinding(request.clone()),
        FindingsCommand::Conflicts(request) => AppCommand::NoteConflicts(Box::new(request.clone())),
    })
}

// JSON string notation preserves exact scalar/quote contents without terminal
// control interpretation or truncation; the structured envelope retains the DTO.
fn exact(value: &str) -> String {
    serde_json::to_string(value).expect("string serializes")
}
fn state_name(value: FindingState) -> &'static str {
    match value {
        FindingState::Open => "open",
        FindingState::Resolved => "resolved",
        FindingState::Dismissed => "dismissed",
    }
}
fn source_text(text: &mut String, source: &SourceVersion) {
    let proof = &source.fingerprint;
    let hash: String = proof
        .sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    writeln!(
        text,
        "Path: {}\nDevice: {}\nInode: {}\nBytes: {}\nSHA-256: {hash}",
        exact(&source.path),
        proof.device,
        proof.inode,
        proof.len
    )
    .expect("String write");
}
fn record_text(text: &mut String, record: &FindingRecord) {
    let draft = &record.draft;
    writeln!(
        text,
        "Finding: {}\nState: {}\nVersion: {}\nTitle: {}\nSummary: {}",
        draft.request.id,
        state_name(record.state),
        record.version,
        exact(&draft.title),
        exact(&draft.summary)
    )
    .expect("String write");
    writeln!(text, "Origin: {}\nCaptured vault: {}\nVault UUID: {}\nVault device: {}\nVault inode: {}\nCreated: {} ms\nUpdated: {} ms", serde_json::to_string(&draft.request.origin).expect("DTO serializes"), exact(&draft.vault.root.to_string_lossy()), draft.vault.id, draft.vault.identity.device, draft.vault.identity.inode, record.created_at_ms, record.updated_at_ms).expect("String write");
    for (index, evidence) in draft.evidence.iter().enumerate() {
        writeln!(text, "\nRetained evidence {index}").expect("String write");
        source_text(text, &evidence.source);
        writeln!(
            text,
            "Observed UUID: {}",
            evidence
                .note_id
                .map_or_else(|| "unmanaged".into(), |id| id.to_string())
        )
        .expect("String write");
        if let Some(quote) = &evidence.quote {
            writeln!(
                text,
                "Quote bytes: {}..{}\nExact quote: {}",
                quote.start_byte,
                quote.end_byte,
                exact(&quote.quote)
            )
            .expect("String write");
        }
    }
}

fn inspection_text(text: &mut String, inspection: &FindingInspection) {
    record_text(text, &inspection.record);
    for observation in &inspection.evidence {
        writeln!(
            text,
            "\nFresh observation {}: {:?}",
            observation.index, observation.outcome
        )
        .expect("String write");
        if let Some(source) = &observation.observed {
            source_text(text, source);
        }
        if let Some(reason) = &observation.reason {
            writeln!(text, "Reason: {}", exact(reason)).expect("String write");
        }
    }
}

pub(super) fn output(command: &FindingsCommand, event: AppEvent) -> Result<Output, CliFailure> {
    let mut text = String::new();
    let data = match (command, event) {
        (FindingsCommand::Capture(request), AppEvent::Finding(record))
            if record.draft.request == *request =>
        {
            record_text(&mut text, &record);
            serde_json::json!(record)
        }
        (FindingsCommand::Show(id), AppEvent::Finding(record))
            if record.draft.request.id == *id =>
        {
            record_text(&mut text, &record);
            serde_json::json!(record)
        }
        (FindingsCommand::Close(request), AppEvent::Finding(record))
            if record.draft.request.id == request.expected.id
                && record.state == request.state
                && record.version == 2 =>
        {
            record_text(&mut text, &record);
            serde_json::json!(record)
        }
        (FindingsCommand::List(request), AppEvent::Findings(page))
            if page.entries.len() <= request.limit
                && page
                    .entries
                    .iter()
                    .all(|record| request.state.is_none_or(|state| record.state == state)) =>
        {
            writeln!(text, "Open findings: {}", page.open_count).expect("String write");
            if page.entries.is_empty() {
                text.push_str("No findings in this page.\n");
            }
            for record in &page.entries {
                text.push('\n');
                record_text(&mut text, record);
            }
            if let Some(cursor) = page.next_before {
                writeln!(text, "Next older page cursor: {cursor}").expect("String write");
            }
            serde_json::json!(page)
        }
        (FindingsCommand::Inspect(id), AppEvent::FindingInspection(inspection))
            if inspection.record.draft.request.id == *id =>
        {
            inspection_text(&mut text, &inspection);
            serde_json::json!(inspection)
        }
        (FindingsCommand::Conflicts(request), AppEvent::NoteConflicts(page))
            if page.path == request.path
                && page.scope == request.scope
                && page.source.path == request.path
                && !page.note_id.is_nil()
                && page.entries.len() <= request.limit
                && page.entries.iter().all(|inspection| {
                    inspection.record.state == FindingState::Open
                        && matches!(
                            inspection.record.draft.request.origin,
                            FindingOrigin::InboxConflict { .. }
                        )
                })
                && page.next_cursor.as_ref().is_none_or(|cursor| {
                    cursor.path == page.path
                        && cursor.scope == page.scope
                        && cursor.note_id == page.note_id
                        && cursor.source == page.source
                }) =>
        {
            writeln!(text, "Conflict lookup path: {}\nScope: {}\nManaged UUID: {}\nOpen conflicts matching this note: {}", exact(&page.path), serde_json::to_string(&page.scope).expect("scope serializes"), page.note_id, page.open_count).expect("String write");
            text.push_str("Fresh lookup note proof:\n");
            source_text(&mut text, &page.source);
            text.push_str(
                "Findings are tentative; fresh observations do not establish a winner.\n",
            );
            if page.entries.is_empty() {
                text.push_str("No conflicts in this page.\n");
            }
            for inspection in &page.entries {
                text.push('\n');
                inspection_text(&mut text, inspection);
            }
            if let Some(cursor) = &page.next_cursor {
                writeln!(
                    text,
                    "Next page cursor JSON: {}",
                    serde_json::to_string(cursor).expect("cursor serializes")
                )
                .expect("String write");
            }
            serde_json::json!(page)
        }
        _ => {
            return Err(CliError::Workflow(
                "unexpected application reply for finding command".into(),
            )
            .into())
        }
    };
    Ok(Output { text, data })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, Invocation};
    use brn_workflow::findings::FindingOrigin;

    #[test]
    fn global_version_banner_and_command_local_review_version_remain_distinct() {
        for args in [
            vec!["--version"],
            vec!["findings", "list", "--version"],
            vec!["--version", "findings", "list"],
        ] {
            let args = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(matches!(
                super::super::parse(&args),
                Ok(super::super::Outcome::Version)
            ));
        }
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let id = Uuid::new_v4().to_string();
        let args = [
            "findings",
            "close",
            &id,
            "--version=1",
            "--state",
            "resolved",
            "--data-dir",
            owner.path().to_str().unwrap(),
        ]
        .map(str::to_owned);
        let super::super::Outcome::Run(invocation) = super::super::parse(&args)
            .unwrap_or_else(|_| panic!("local exact review stamp parses"))
        else {
            panic!("review stamp must not become a version banner")
        };
        let Command::Findings(FindingsCommand::Close(request)) = invocation.command else {
            panic!("closure command")
        };
        assert_eq!(request.expected.version, 1);
        assert_eq!(request.expected.id.to_string(), id);
        assert_eq!(std::fs::read_dir(owner.path()).unwrap().count(), 0);
    }

    #[test]
    fn directly_constructed_invalid_findings_refuse_before_workspace_authority() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("task.credentials");
        let commands = [
            FindingsCommand::Capture(CaptureFindingRequest {
                id: Uuid::nil(),
                origin: FindingOrigin::IdentityAmbiguity {
                    note_id: Uuid::new_v4(),
                },
            }),
            FindingsCommand::List(FindingListRequest {
                state: None,
                limit: 101,
                before: None,
            }),
            FindingsCommand::Conflicts(NoteConflictRequest {
                path: "../escape.md".into(),
                scope: KnowledgeScope::All,
                limit: 10,
                cursor: None,
            }),
            FindingsCommand::Conflicts(NoteConflictRequest {
                path: "current.md".into(),
                scope: KnowledgeScope::Current,
                limit: 101,
                cursor: None,
            }),
            FindingsCommand::Show(Uuid::nil()),
            FindingsCommand::Inspect(Uuid::nil()),
            FindingsCommand::Close(CloseFindingRequest {
                expected: FindingStamp {
                    id: Uuid::new_v4(),
                    version: 0,
                },
                state: FindingState::Resolved,
            }),
        ];
        // A legacy marker would otherwise produce WORKSPACE_MODE_CONFLICT. The
        // usage refusal must precede even that authority classification.
        std::fs::write(data.join("brn.sqlite3"), b"synthetic marker").unwrap();
        for command in commands {
            let invocation = Invocation {
                json: true,
                data_dir: data.clone(),
                model_dir: None,
                vault: None,
                credentials_dir: Some(credentials.clone()),
                command: Command::Findings(command),
            };
            let failure = super::super::execute(&invocation)
                .err()
                .expect("invalid input refuses");
            assert_eq!(failure.error.code(), "USAGE");
            assert_eq!(std::fs::read_dir(&data).unwrap().count(), 1);
            assert_eq!(
                std::fs::read(data.join("brn.sqlite3")).unwrap(),
                b"synthetic marker"
            );
            assert!(!credentials.exists());
        }
    }

    #[test]
    fn conflict_parser_preserves_default_current_and_explicit_scopes_without_startup() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        for (scope, expected) in [
            (None, KnowledgeScope::Current),
            (Some("source"), KnowledgeScope::Source),
            (Some("history"), KnowledgeScope::History),
            (Some("all"), KnowledgeScope::All),
        ] {
            let path = if scope.is_none() {
                "note.md"
            } else {
                "archive/note.md"
            };
            let mut args = vec![
                "findings".into(),
                "conflicts".into(),
                path.into(),
                "--data-dir".into(),
                owner.path().to_str().unwrap().into(),
            ];
            if let Some(scope) = scope {
                args.extend([
                    "--scope".into(),
                    scope.into(),
                    "--limit".into(),
                    "100".into(),
                ]);
            }
            let super::super::Outcome::Run(invocation) =
                super::super::parse(&args).unwrap_or_else(|_| panic!("scope parses"))
            else {
                panic!("run")
            };
            let Command::Findings(FindingsCommand::Conflicts(request)) = invocation.command else {
                panic!("conflict lookup")
            };
            assert_eq!(request.path, path);
            assert_eq!(request.scope, expected);
            assert_eq!(request.limit, if scope.is_none() { 10 } else { 100 });
            assert!(request.cursor.is_none());
            assert!(matches!(
                prepare(&FindingsCommand::Conflicts(request))
                    .unwrap_or_else(|_| panic!("valid lookup")),
                AppCommand::NoteConflicts(_)
            ));
            assert_eq!(std::fs::read_dir(owner.path()).unwrap().count(), 0);
        }
    }
}
