//! Argument parsing, JSON envelope and dispatch types for the brn CLI.
//!
//! Long options only, `--key value` or `--key=value`; global options
//! (`--data-dir`, `--json`, `--codex`, `--model-dir`, `--help`, `--version`)
//! may appear before or after the subcommand. All arguments are validated
//! before any workspace is opened.
pub mod ask;
pub mod documents;
pub mod error;
pub mod retrieval;
pub mod review;
pub mod status;

use crate::cli::error::{classify_workflow, CliError};
use brn_workflow::SearchProfile;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::Ordering;
use uuid::Uuid;

/// A fully validated invocation, ready to execute.
pub struct Invocation {
    pub json: bool,
    pub data_dir: PathBuf,
    pub codex: Option<PathBuf>,
    pub model_dir: Option<PathBuf>,
    pub command: Command,
}

pub enum Command {
    Status,
    Import {
        path: PathBuf,
        approve_for_search: bool,
        operation: Option<Uuid>,
    },
    DocumentsList,
    DocumentsShow {
        source: Uuid,
    },
    DocumentsSetApproval {
        source: Uuid,
        version: Uuid,
        state: brn_workflow::SearchApproval,
        operation: Option<Uuid>,
    },
    IndexBuild,
    Search {
        query: String,
        profile: SearchProfile,
    },
    Ask {
        question: String,
        profile: SearchProfile,
        session: Option<Uuid>,
        operation: Option<Uuid>,
        timeout_seconds: u64,
    },
    ConversationsList,
    ConversationsShow {
        session: Uuid,
    },
    DraftsList,
    DraftsShow {
        draft: Uuid,
    },
    CommentsList {
        draft: Uuid,
    },
    RevisionsList {
        draft: Uuid,
    },
    RevisionsShow {
        revision: Uuid,
    },
    RevisionsDiff {
        draft: Uuid,
        from: Uuid,
        to: Uuid,
    },
}

/// Command result: human text (self-terminated with `\n`) and JSON data.
pub struct Output {
    pub text: String,
    pub data: serde_json::Value,
}

/// Parse failure bundled with the flags needed for a correct envelope.
pub struct ParseFailure {
    pub error: CliError,
    pub json: bool,
    pub command: Option<&'static str>,
}

pub enum Outcome {
    Help,
    Version,
    Run(Box<Invocation>),
}

pub const HELP: &str = "\
brn: agent-facing CLI for BRN workspaces

Usage: brn COMMAND [OPTIONS] --data-dir ABSOLUTE_EXISTING_DIRECTORY

Global options (accepted before or after the command):
  --data-dir DIR     Existing absolute workspace directory (required for commands)
  --json             Print exactly one JSON envelope object on stdout
  --codex PATH       Absolute Codex executable path (does not imply authentication)
  --model-dir DIR    Absolute local model directory
  --help             Show this help (works without --data-dir)
  --version          Show the version (works without --data-dir)

Commands:
  brn status
  brn import PATH [--approve-for-search] [--operation UUID]
  brn documents list
  brn documents show SOURCE_ID
  brn documents set-search-approval SOURCE_ID --version-id VERSION_ID --state approved|draft|withdrawn [--operation UUID]
  brn index build
  brn search QUERY [--profile keyword|semantic|hybrid]
  brn ask QUESTION [--profile keyword|semantic|hybrid] [--session UUID] [--operation UUID] [--timeout-seconds N]
  brn conversations list
  brn conversations show SESSION_ID
  brn drafts list
  brn drafts show DRAFT_ID
  brn comments list --draft DRAFT_ID
  brn revisions list --draft DRAFT_ID
  brn revisions show REVISION_ID
  brn revisions diff --draft DRAFT_ID --from REVISION_ID --to REVISION_ID

Exit codes: 0 success; 1 operational failure; 2 usage error; 124 deadline;
130 interrupted.
";

pub fn version_line() -> String {
    format!("brn {}", env!("CARGO_PKG_VERSION"))
}

pub fn parse(args: &[String]) -> Result<Outcome, ParseFailure> {
    // Usage errors must honor --json even when parsing aborts before reaching
    // the flag, so pre-scan argv. Under this parser a bare "--json" token can
    // never be an option value (values starting with "--" are rejected), so an
    // exact match is unambiguous.
    let json_requested = args.iter().any(|a| a == "--json");
    // An exact "--help" token wins before all validation (group words, required
    // arguments, --data-dir checks) at every command level. Same exactness
    // argument as --json: "--help=value" stays a usage error.
    if args.iter().any(|a| a == "--help") {
        return Ok(Outcome::Help);
    }
    let mut g = Globals::default();
    let mut command: Option<&'static str> = None;
    parse_inner(&mut g, &mut command, args).map_err(|error| ParseFailure {
        error,
        json: json_requested,
        command,
    })
}

#[derive(Default)]
struct Globals {
    json: bool,
    data_dir: Option<String>,
    codex: Option<String>,
    model_dir: Option<String>,
    help: bool,
    version: bool,
}

fn usage(message: impl Into<String>) -> CliError {
    CliError::Usage(message.into())
}

/// Split `--name=value` into `(name, Some(value))`, `--name` into `(name, None)`.
fn split_option(token: &str) -> (&str, Option<&str>) {
    let rest = token.strip_prefix("--").unwrap_or(token);
    match rest.split_once('=') {
        Some((name, value)) => (name, Some(value)),
        None => (rest, None),
    }
}

type Tokens<'a> = &'a mut dyn Iterator<Item = String>;

fn take_value(tokens: Tokens<'_>, option: &str) -> Result<String, CliError> {
    match tokens.next() {
        Some(value) if !value.starts_with("--") => Ok(value),
        _ => Err(usage(format!("missing value for {option}"))),
    }
}

fn set_global(g: &mut Globals, name: &str, value: String, token: &str) -> Result<(), CliError> {
    let slot = match name {
        "data-dir" => &mut g.data_dir,
        "codex" => &mut g.codex,
        _ => &mut g.model_dir,
    };
    if slot.is_some() {
        return Err(usage(format!("duplicate option: {token}")));
    }
    *slot = Some(value);
    Ok(())
}

/// Handle one recognized global option token; returns false if `name` is not global.
fn global_option(
    g: &mut Globals,
    name: &str,
    inline: Option<&str>,
    token: &str,
    tokens: Tokens<'_>,
) -> Result<bool, CliError> {
    match name {
        "help" | "version" => {
            if inline.is_some() {
                return Err(usage(format!("{token} does not take a value")));
            }
            if name == "help" {
                g.help = true;
            } else {
                g.version = true;
            }
        }
        "json" => {
            if inline.is_some() {
                return Err(usage(format!("{token} does not take a value")));
            }
            if g.json {
                return Err(usage(format!("duplicate option: {token}")));
            }
            g.json = true;
        }
        "data-dir" | "codex" | "model-dir" => {
            let value = match inline {
                Some(value) => value.to_string(),
                None => take_value(tokens, token)?,
            };
            set_global(g, name, value, token)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Raw scan result for one command: positionals and named options in order of appearance.
#[derive(Default)]
struct Scanned {
    positionals: Vec<String>,
    flags: Vec<String>,
    values: Vec<(String, String)>,
}

impl Scanned {
    fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
    }
    fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
    fn uuid(&self, name: &str) -> Result<Option<Uuid>, CliError> {
        match self.value(name) {
            Some(raw) => Uuid::parse_str(raw)
                .map(Some)
                .map_err(|_| usage(format!("invalid --{name} UUID: {raw}"))),
            None => Ok(None),
        }
    }
    fn require_uuid(&self, name: &str) -> Result<Uuid, CliError> {
        self.uuid(name)?
            .ok_or_else(|| usage(format!("missing --{name}")))
    }
}

/// Scan remaining tokens for one command. `options` lists command-specific
/// option names and whether each takes a value; global options are always allowed.
fn scan(
    tokens: Tokens<'_>,
    g: &mut Globals,
    options: &[(&str, bool)],
) -> Result<Scanned, CliError> {
    let mut out = Scanned::default();
    while let Some(token) = tokens.next() {
        if token.starts_with("--") {
            let (name, inline) = split_option(&token);
            if global_option(g, name, inline, &token, tokens)? {
                continue;
            }
            let Some(&(label, takes_value)) = options.iter().find(|(n, _)| *n == name) else {
                return Err(usage(format!("unknown option: {token}")));
            };
            if takes_value {
                if out.values.iter().any(|(n, _)| n == label) {
                    return Err(usage(format!("duplicate option: {token}")));
                }
                let value = match inline {
                    Some(value) => value.to_string(),
                    None => take_value(tokens, &token)?,
                };
                out.values.push((label.to_string(), value));
            } else {
                if inline.is_some() {
                    return Err(usage(format!("--{label} does not take a value")));
                }
                if out.flag(label) {
                    return Err(usage(format!("duplicate option: {token}")));
                }
                out.flags.push(label.to_string());
            }
        } else {
            out.positionals.push(token);
        }
    }
    Ok(out)
}

fn expect_positionals(scanned: &Scanned, count: usize) -> Result<(), CliError> {
    match scanned.positionals.get(count) {
        Some(extra) => Err(usage(format!("unexpected argument: {extra}"))),
        None => Ok(()),
    }
}

fn positional_uuid(scanned: &Scanned, index: usize, label: &str) -> Result<Uuid, CliError> {
    let raw = scanned
        .positionals
        .get(index)
        .ok_or_else(|| usage(format!("missing {label}")))?;
    Uuid::parse_str(raw).map_err(|_| usage(format!("invalid {label} UUID: {raw}")))
}

fn required_positional<'a>(scanned: &'a Scanned, label: &str) -> Result<&'a str, CliError> {
    scanned
        .positionals
        .first()
        .map(String::as_str)
        .ok_or_else(|| usage(format!("missing {label}")))
}

fn parse_profile(raw: Option<&str>) -> Result<SearchProfile, CliError> {
    match raw.unwrap_or("keyword") {
        "keyword" => Ok(SearchProfile::Keyword),
        "semantic" => Ok(SearchProfile::Semantic),
        "hybrid" => Ok(SearchProfile::Hybrid),
        other => Err(usage(format!(
            "invalid --profile: {other} (expected keyword|semantic|hybrid)"
        ))),
    }
}

fn parse_state(raw: &str) -> Result<brn_workflow::SearchApproval, CliError> {
    match raw {
        "approved" => Ok(brn_workflow::SearchApproval::Approved),
        "draft" => Ok(brn_workflow::SearchApproval::Draft),
        "withdrawn" => Ok(brn_workflow::SearchApproval::Withdrawn),
        other => Err(usage(format!(
            "invalid --state: {other} (expected approved|draft|withdrawn)"
        ))),
    }
}

fn parse_timeout(raw: &str) -> Result<u64, CliError> {
    let seconds: u64 = raw
        .parse()
        .map_err(|_| usage(format!("invalid --timeout-seconds: {raw}")))?;
    if !(1..=3600).contains(&seconds) {
        return Err(usage(format!(
            "--timeout-seconds must be between 1 and 3600: {raw}"
        )));
    }
    Ok(seconds)
}

fn sub_word(tokens: Tokens<'_>, group: &str, expected: &str) -> Result<String, CliError> {
    tokens
        .next()
        .filter(|t| !t.starts_with("--"))
        .ok_or_else(|| usage(format!("missing {group} subcommand ({expected})")))
}

fn parse_inner(
    g: &mut Globals,
    command: &mut Option<&'static str>,
    args: &[String],
) -> Result<Outcome, CliError> {
    let mut tokens = args.iter().cloned();

    // Pass 1: global options before the top-level command word.
    let mut command_word: Option<String> = None;
    while let Some(token) = tokens.next() {
        if token.starts_with("--") {
            let (name, inline) = split_option(&token);
            if global_option(g, name, inline, &token, &mut tokens)? {
                continue;
            }
            return Err(usage(format!(
                "unknown option: {token} (only global options may precede the command)"
            )));
        }
        command_word = Some(token);
        break;
    }
    let Some(word) = command_word else {
        if g.help {
            return Ok(Outcome::Help);
        }
        if g.version {
            return Ok(Outcome::Version);
        }
        return Err(usage("expected a command; run brn --help"));
    };

    // Pass 2: subcommand words, command-specific options and positionals.
    let scanned = match word.as_str() {
        "help" => return Ok(Outcome::Help),
        "status" => {
            *command = Some("status");
            scan(&mut tokens, g, &[])?
        }
        "import" => {
            *command = Some("import");
            scan(
                &mut tokens,
                g,
                &[("approve-for-search", false), ("operation", true)],
            )?
        }
        "documents" => {
            let sub = sub_word(&mut tokens, "documents", "list|show|set-search-approval")?;
            match sub.as_str() {
                "list" => {
                    *command = Some("documents.list");
                    scan(&mut tokens, g, &[])?
                }
                "show" => {
                    *command = Some("documents.show");
                    scan(&mut tokens, g, &[])?
                }
                "set-search-approval" => {
                    *command = Some("documents.set-search-approval");
                    scan(
                        &mut tokens,
                        g,
                        &[("version-id", true), ("state", true), ("operation", true)],
                    )?
                }
                other => return Err(usage(format!("unknown documents subcommand: {other}"))),
            }
        }
        "index" => {
            let sub = sub_word(&mut tokens, "index", "build")?;
            match sub.as_str() {
                "build" => {
                    *command = Some("index.build");
                    scan(&mut tokens, g, &[])?
                }
                other => return Err(usage(format!("unknown index subcommand: {other}"))),
            }
        }
        "search" => {
            *command = Some("search");
            scan(&mut tokens, g, &[("profile", true)])?
        }
        "ask" => {
            *command = Some("ask");
            scan(
                &mut tokens,
                g,
                &[
                    ("profile", true),
                    ("session", true),
                    ("operation", true),
                    ("timeout-seconds", true),
                ],
            )?
        }
        "conversations" => {
            let sub = sub_word(&mut tokens, "conversations", "list|show")?;
            match sub.as_str() {
                "list" => {
                    *command = Some("conversations.list");
                    scan(&mut tokens, g, &[])?
                }
                "show" => {
                    *command = Some("conversations.show");
                    scan(&mut tokens, g, &[])?
                }
                other => return Err(usage(format!("unknown conversations subcommand: {other}"))),
            }
        }
        "drafts" => {
            let sub = sub_word(&mut tokens, "drafts", "list|show")?;
            match sub.as_str() {
                "list" => {
                    *command = Some("drafts.list");
                    scan(&mut tokens, g, &[])?
                }
                "show" => {
                    *command = Some("drafts.show");
                    scan(&mut tokens, g, &[])?
                }
                other => return Err(usage(format!("unknown drafts subcommand: {other}"))),
            }
        }
        "comments" => {
            let sub = sub_word(&mut tokens, "comments", "list")?;
            match sub.as_str() {
                "list" => {
                    *command = Some("comments.list");
                    scan(&mut tokens, g, &[("draft", true)])?
                }
                other => return Err(usage(format!("unknown comments subcommand: {other}"))),
            }
        }
        "revisions" => {
            let sub = sub_word(&mut tokens, "revisions", "list|show|diff")?;
            match sub.as_str() {
                "list" => {
                    *command = Some("revisions.list");
                    scan(&mut tokens, g, &[("draft", true)])?
                }
                "show" => {
                    *command = Some("revisions.show");
                    scan(&mut tokens, g, &[])?
                }
                "diff" => {
                    *command = Some("revisions.diff");
                    scan(
                        &mut tokens,
                        g,
                        &[("draft", true), ("from", true), ("to", true)],
                    )?
                }
                other => return Err(usage(format!("unknown revisions subcommand: {other}"))),
            }
        }
        other => return Err(usage(format!("unknown command: {other}"))),
    };

    // Build the command from scanned arguments.
    let built = match word.as_str() {
        "status" => {
            expect_positionals(&scanned, 0)?;
            Command::Status
        }
        "import" => {
            let path = required_positional(&scanned, "import PATH")?;
            expect_positionals(&scanned, 1)?;
            Command::Import {
                path: PathBuf::from(path),
                approve_for_search: scanned.flag("approve-for-search"),
                operation: scanned.uuid("operation")?,
            }
        }
        "documents" => match command.unwrap() {
            "documents.list" => {
                expect_positionals(&scanned, 0)?;
                Command::DocumentsList
            }
            "documents.show" => {
                expect_positionals(&scanned, 1)?;
                Command::DocumentsShow {
                    source: positional_uuid(&scanned, 0, "SOURCE_ID")?,
                }
            }
            "documents.set-search-approval" => {
                expect_positionals(&scanned, 1)?;
                let state = scanned
                    .value("state")
                    .ok_or_else(|| usage("missing --state"))?;
                Command::DocumentsSetApproval {
                    source: positional_uuid(&scanned, 0, "SOURCE_ID")?,
                    version: scanned.require_uuid("version-id")?,
                    state: parse_state(state)?,
                    operation: scanned.uuid("operation")?,
                }
            }
            _ => unreachable!(),
        },
        "index" => {
            expect_positionals(&scanned, 0)?;
            Command::IndexBuild
        }
        "search" => {
            let query = required_positional(&scanned, "search QUERY")?.to_string();
            expect_positionals(&scanned, 1)?;
            Command::Search {
                query,
                profile: parse_profile(scanned.value("profile"))?,
            }
        }
        "ask" => {
            let question = required_positional(&scanned, "ask QUESTION")?.to_string();
            expect_positionals(&scanned, 1)?;
            let session = if scanned.value("session").is_some() {
                Some(scanned.require_uuid("session")?)
            } else {
                None
            };
            let timeout_seconds = match scanned.value("timeout-seconds") {
                Some(raw) => parse_timeout(raw)?,
                None => 300,
            };
            Command::Ask {
                question,
                profile: parse_profile(scanned.value("profile"))?,
                session,
                operation: scanned.uuid("operation")?,
                timeout_seconds,
            }
        }
        "conversations" => match command.unwrap() {
            "conversations.list" => {
                expect_positionals(&scanned, 0)?;
                Command::ConversationsList
            }
            "conversations.show" => {
                expect_positionals(&scanned, 1)?;
                Command::ConversationsShow {
                    session: positional_uuid(&scanned, 0, "SESSION_ID")?,
                }
            }
            _ => unreachable!(),
        },
        "drafts" => match command.unwrap() {
            "drafts.list" => {
                expect_positionals(&scanned, 0)?;
                Command::DraftsList
            }
            "drafts.show" => {
                expect_positionals(&scanned, 1)?;
                Command::DraftsShow {
                    draft: positional_uuid(&scanned, 0, "DRAFT_ID")?,
                }
            }
            _ => unreachable!(),
        },
        "comments" => {
            expect_positionals(&scanned, 0)?;
            Command::CommentsList {
                draft: scanned.require_uuid("draft")?,
            }
        }
        "revisions" => match command.unwrap() {
            "revisions.list" => {
                expect_positionals(&scanned, 0)?;
                Command::RevisionsList {
                    draft: scanned.require_uuid("draft")?,
                }
            }
            "revisions.show" => {
                expect_positionals(&scanned, 1)?;
                Command::RevisionsShow {
                    revision: positional_uuid(&scanned, 0, "REVISION_ID")?,
                }
            }
            "revisions.diff" => {
                expect_positionals(&scanned, 0)?;
                Command::RevisionsDiff {
                    draft: scanned.require_uuid("draft")?,
                    from: scanned.require_uuid("from")?,
                    to: scanned.require_uuid("to")?,
                }
            }
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    // Post-parse help/version short-circuits (textual, exit 0, no --data-dir needed).
    if g.help {
        return Ok(Outcome::Help);
    }
    if g.version {
        return Ok(Outcome::Version);
    }

    // Workspace-independent validation before anything is opened.
    let raw_data_dir = g
        .data_dir
        .take()
        .ok_or_else(|| usage("missing --data-dir (required for commands)"))?;
    let data_dir = PathBuf::from(&raw_data_dir);
    if !data_dir.is_absolute() {
        return Err(usage(format!(
            "--data-dir must be an absolute path: {raw_data_dir}"
        )));
    }
    if !data_dir.exists() {
        return Err(usage(format!("--data-dir does not exist: {raw_data_dir}")));
    }
    if !data_dir.is_dir() {
        return Err(usage(format!(
            "--data-dir is not a directory: {raw_data_dir}"
        )));
    }
    let codex = g.codex.take().map(PathBuf::from);
    let model_dir = g.model_dir.take().map(PathBuf::from);
    for (label, path) in [("--codex", &codex), ("--model-dir", &model_dir)] {
        if let Some(path) = path {
            if !path.is_absolute() {
                return Err(usage(format!("{label} must be an absolute path")));
            }
        }
    }
    Ok(Outcome::Run(Box::new(Invocation {
        json: g.json,
        data_dir,
        codex,
        model_dir,
        command: built,
    })))
}

/// Pretty-printed success envelope.
pub fn envelope_ok(command: &str, data: serde_json::Value) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "schema_version": 1,
        "command": command,
        "ok": true,
        "data": data,
    }))
    .expect("envelope serializes")
}

/// Pretty-printed failure envelope; `command` is null when unidentified.
pub fn envelope_err(command: Option<&str>, error: &CliError) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "schema_version": 1,
        "command": command,
        "ok": false,
        "error": { "code": error.code(), "message": error.message() },
    }))
    .expect("envelope serializes")
}

/// Print an error in the mode-appropriate surface.
pub fn report_error(json: bool, command: Option<&str>, error: &CliError) {
    if json {
        println!("{}", envelope_err(command, error));
    } else {
        eprintln!("error: {}", error.message());
    }
}

/// A silent SIGINT during a non-ask command must not look like success: once
/// the command result arrives, a set `CANCEL` flag escalates an `Ok` result to
/// `INTERRUPTED` (exit 130). Ask classifies its own cancellation — a durably
/// completed turn stays a success — and errors pass through unchanged.
pub(crate) fn escalate_interrupt(
    result: Result<Output, CliError>,
    is_ask: bool,
) -> Result<Output, CliError> {
    if !is_ask && result.is_ok() && crate::CANCEL.load(Ordering::SeqCst) {
        return Err(CliError::Interrupted(
            "interrupted before completion".into(),
        ));
    }
    result
}

/// Map a command result to process exit, honoring the output mode.
pub fn finish(json: bool, command: &str, result: Result<Output, CliError>) -> ExitCode {
    match result {
        Ok(output) => {
            if json {
                println!("{}", envelope_ok(command, output.data));
            } else {
                print!("{}", output.text);
            }
            use std::io::Write;
            let _ = std::io::stdout().flush();
            ExitCode::SUCCESS
        }
        Err(error) => {
            report_error(json, Some(command), &error);
            ExitCode::from(error.exit_code())
        }
    }
}

/// Open the workspace for a validated invocation, classifying store errors.
pub fn open_workspace(invocation: &Invocation) -> Result<brn_workflow::Workspace, CliError> {
    brn_workflow::Workspace::open(
        &invocation.data_dir,
        brn_workflow::Config {
            codex: invocation.codex.clone(),
            codex_home: None,
            model_dir: invocation.model_dir.clone(),
        },
    )
    .map_err(classify_workflow)
}

/// Route a validated invocation. Stubs short-circuit before the workspace is
/// opened so they never create, lock or recover a data directory.
pub fn execute(invocation: &Invocation) -> Result<Output, CliError> {
    match &invocation.command {
        Command::Import { .. }
        | Command::DocumentsSetApproval { .. }
        | Command::IndexBuild
        | Command::Search { .. } => return retrieval::run(invocation),
        Command::Ask { .. } | Command::ConversationsList | Command::ConversationsShow { .. } => {
            return ask::run(invocation)
        }
        Command::DraftsList
        | Command::DraftsShow { .. }
        | Command::CommentsList { .. }
        | Command::RevisionsList { .. }
        | Command::RevisionsShow { .. }
        | Command::RevisionsDiff { .. } => return review::run(invocation),
        Command::Status | Command::DocumentsList | Command::DocumentsShow { .. } => {}
    }
    let workspace = open_workspace(invocation)?;
    match &invocation.command {
        Command::Status => status::run(invocation, &workspace),
        Command::DocumentsList => documents::list(invocation, &workspace),
        Command::DocumentsShow { source } => documents::show(invocation, &workspace, *source),
        _ => unreachable!("stub commands returned above"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_result() -> Result<Output, CliError> {
        Ok(Output {
            text: String::new(),
            data: serde_json::json!(null),
        })
    }

    /// One sequential test: CANCEL is process-global, so all cases share a
    /// single thread and the prior flag value is restored at the end.
    #[test]
    fn escalation_only_fires_for_cancelled_non_ask_successes() {
        let was = crate::CANCEL.load(Ordering::SeqCst);
        crate::CANCEL.store(false, Ordering::SeqCst);
        // Ok without CANCEL: unchanged.
        assert!(escalate_interrupt(ok_result(), false).is_ok());
        // Err passes through even with CANCEL set.
        crate::CANCEL.store(true, Ordering::SeqCst);
        assert_eq!(
            escalate_interrupt(Err(CliError::Usage("u".into())), false).err(),
            Some(CliError::Usage("u".into()))
        );
        // Ok + CANCEL + non-ask: escalated to INTERRUPTED.
        assert_eq!(
            escalate_interrupt(ok_result(), false).err(),
            Some(CliError::Interrupted(
                "interrupted before completion".into()
            ))
        );
        // Ok + CANCEL + ask: unchanged (ask classifies its own cancellation).
        assert!(escalate_interrupt(ok_result(), true).is_ok());
        crate::CANCEL.store(was, Ordering::SeqCst);
    }
}
