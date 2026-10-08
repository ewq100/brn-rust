//! Argument parsing and output policy for the shared BRN application CLI.
//! Global long options may appear before or after the command. Inputs are
//! validated before AppWorker opens operational storage.
pub mod actions;
pub mod activity;
pub mod ai;
pub mod editor;
pub mod error;
pub mod evidence;
pub mod findings;
pub mod identity;
pub mod inbox;
mod input;
pub mod library;
pub mod links;
pub(crate) mod out;
pub mod proposals;
pub mod provenance;
pub mod relationships;

use crate::cli::error::CliError;
use brn_workflow::{
    library::{KnowledgeScope, SearchMode},
    vault::{EvidencePath, VaultPath},
};
use std::path::PathBuf;
use std::process::ExitCode;
use uuid::Uuid;

pub struct Invocation {
    pub json: bool,
    pub data_dir: PathBuf,
    pub model_dir: Option<PathBuf>,
    pub vault: Option<PathBuf>,
    pub credentials_dir: Option<PathBuf>,
    pub command: Command,
}

pub enum Command {
    Activity(brn_workflow::activity::ActivityRequest),
    Editor(editor::EditorCommand),
    Identity(identity::IdentityCommand),
    Evidence(evidence::EvidenceCommand),
    Provenance(provenance::ProvenanceCommand),
    Links(links::LinksCommand),
    Findings(findings::FindingsCommand),
    Actions(actions::ActionsCommand),
    Inbox(inbox::InboxCommand),
    Relationships(brn_workflow::knowledge::RelationshipRequest),
    Proposals(proposals::ProposalCommand),
    Ai(ai::AiCommand),
    ModelDownload {
        timeout_seconds: u64,
    },
    NotesList {
        folder: Option<String>,
        cursor: Option<String>,
        scope: KnowledgeScope,
    },
    NotePath {
        path: String,
        scope: KnowledgeScope,
    },
    Status,
    Search {
        query: String,
        profile: Option<SearchMode>,
        limit: Option<usize>,
        scope: KnowledgeScope,
    },
    Ask {
        question: String,
        session: Option<Uuid>,
        operation: Option<Uuid>,
        timeout_seconds: u64,
    },
    ConversationsList,
    ConversationsShow {
        session: Uuid,
    },
}

pub struct Output {
    pub text: String,
    pub data: serde_json::Value,
}

/// Optional machine-readable context extends the existing error envelope.
#[derive(Debug)]
pub struct CliFailure {
    pub error: CliError,
    pub context: Option<serde_json::Value>,
}
impl From<CliError> for CliFailure {
    fn from(error: CliError) -> Self {
        Self {
            error,
            context: None,
        }
    }
}

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
brn: agent-facing CLI for BRN

Usage: brn COMMAND [OPTIONS] --data-dir ABSOLUTE_EXISTING_DIRECTORY

Global options (accepted before or after the command):
  --data-dir DIR     Existing absolute operational directory (required)
  --json             Print one JSON envelope object on stdout
  --model-dir DIR    Absolute local model directory
  --vault DIR        Initial Markdown vault binding
  --credentials-dir DIR  Absolute protected credential directory
  --help             Show help without opening storage
  --version          Show the version without opening storage

Commands:
  brn activity list [--limit N] [--before OPERATION_UUID]
  brn identity show PATH
  brn identity prepare PATH --note-id UUID --proposal UUID --title TITLE
  brn identity inventory
  brn identity resolve NOTE_ID
  brn evidence read PATH
  brn provenance show PATH
  brn provenance capture --file REQUEST.json
  brn provenance prepare --file REQUEST.json
  brn links show PATH
  brn links prepare --file REQUEST.json
  brn relationships list [--scope current|source|history|all] [--offset N] [--limit N]
  brn findings capture --file REQUEST.json
  brn findings list [--state open|resolved|dismissed|all] [--limit N] [--before UUID]
  brn findings show UUID
  brn findings inspect UUID
  brn findings conflicts PATH [--scope current|source|history|all] [--limit N] [--cursor JSON]
  brn findings close UUID --version N --state resolved|dismissed
  brn inbox add --id UUID --title TITLE --file TEXT_FILE [--kind text|markdown|email|teams] [--original-name LABEL]
  brn inbox add-binary --id UUID --title TITLE --file BINARY_FILE [--original-name LABEL]
  brn inbox list [--limit N] [--after UUID]
  brn inbox show UUID
  brn inbox review UUID
  brn inbox removal-preview UUID
  brn inbox remove-original REQUEST_JSON
  brn inbox restore-original REQUEST_JSON
  brn inbox original-removal OPERATION_UUID
  brn inbox original-restore OPERATION_UUID
  brn inbox original-operations ITEM_UUID
  brn inbox archived-analysis ANALYSIS_UUID
  brn inbox process --file REQUEST_JSON
  brn inbox processing UUID
  brn inbox candidate UUID INDEX
  brn inbox export BATCH_UUID INDEX SOURCE_ID --output PATH
  brn inbox intake-binding SOURCE_PROPOSAL_UUID
  brn inbox source --file REQUEST_JSON
  brn inbox visual SOURCE_PATH
  brn inbox interpret-visual --file REQUEST_JSON [--timeout-seconds N]
  brn inbox visual-annotation ANALYSIS_UUID
  brn inbox cancel UUID
  brn inbox analyze-actions --file REQUEST_JSON [--timeout-seconds N]
  brn inbox action-analysis UUID
  brn inbox analyze --file REQUEST_JSON [--timeout-seconds N]
  brn inbox analysis UUID
  brn actions complete --file REQUEST.json
  brn actions show UUID
  brn actions list [--state open|waiting|blocked|completed|all] [--limit N] [--before-created-at-ms N --before-id UUID]
  brn actions dashboard [--as-of YYYY-MM-DD] [--filter active|open|waiting|blocked|completed|overdue|follow-up|all] [--limit N] [--before-created-at-ms N --before-id UUID]
  brn proposals create --file DRAFT.json
  brn proposals source PATH
  brn proposals asset PATH
  brn proposals list [--group UUID]
  brn proposals show PROPOSAL_ID
  brn proposals edit --file EDIT.json
  brn proposals rewrite --file REQUEST.json
  brn proposals rewrite-status JOB_UUID
  brn proposals rewrite-result --file EDIT.json
  brn proposals comment --file COMMENT.json
  brn proposals comment-update --file COMMENT.json
  brn proposals comment-remove PROPOSAL_ID --review-version N --comment UUID
  brn proposals reject PROPOSAL_ID --review-version N
  brn proposals approve PROPOSAL_ID --review-version N --operation UUID
  brn proposals reconcile OPERATION_UUID
  brn proposals approve-group --file APPROVALS.json
  brn proposals applies
  brn proposals undo-preview TARGET_OPERATION_UUID --operation NEW_UUID [--member INDEX]
  brn proposals undo TARGET_OPERATION_UUID --operation NEW_UUID
  brn proposals restore-trash TARGET_OPERATION_UUID --member INDEX --operation NEW_UUID
  brn proposals repair-preview OPERATION_UUID
  brn proposals repair --file REQUEST.json
  brn edit open PATH
  brn edit recover PATH --baseline UUID --expected-generation N --generation N --file F
  brn edit save PATH --baseline UUID --expected-generation N --generation N --file F --operation UUID [--copy PATH]
  brn edit reload PATH --baseline UUID --expected-generation N --observed-file F [--discard]
  brn edit list
  brn edit reconcile OPERATION
  brn ai connect chatgpt|copilot [--timeout-seconds N]
  brn ai disconnect chatgpt|copilot
  brn ai status
  brn ai models chatgpt|copilot [--timeout-seconds N]
  brn ai select --provider chatgpt|copilot --model MODEL
  brn ai effort [low|medium|high]
  brn models download --approve-download [--model-dir DIR] [--timeout-seconds N]
  brn notes list [--folder FOLDER] [--cursor PATH] [--scope current|source|history|all]
  brn notes show PATH.md [--scope current|source|history|all]
  brn status
  brn search QUERY [--profile keyword|semantic|hybrid] [--limit N] [--scope current|source|history|all]
  brn ask QUESTION [--session UUID] [--operation UUID] [--timeout-seconds N]
  brn conversations list
  brn conversations show SESSION_ID

Exit codes: 0 success; 1 operational failure; 2 usage error; 124 deadline;
130 interrupted. A completed operation whose stdout result cannot be
 delivered exits 1 with OUTPUT_DELIVERY_ERROR on stderr; a closed pipe stays
 a quiet 0.
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
    model_dir: Option<String>,
    vault: Option<String>,
    credentials_dir: Option<String>,
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
        "vault" => &mut g.vault,
        "credentials-dir" => &mut g.credentials_dir,
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
            // A bare "--help" token never reaches the parser (exact-token
            // pre-scan in `parse`); this arm still rejects "--help=value".
            if inline.is_some() {
                return Err(usage(format!("{token} does not take a value")));
            }
            if name == "version" {
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
        "data-dir" | "model-dir" | "vault" | "credentials-dir" => {
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
    // The store persists generations as signed 64-bit integers. The input is
    // parsed as u64 so values above this bound can be rejected as usage errors
    // before any workspace access.
    const MAX_GENERATION: u64 = i64::MAX as u64;

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
    fn generation(&self, name: &str) -> Result<Option<u64>, CliError> {
        match self.value(name) {
            Some(raw) => {
                let generation = raw
                    .parse::<u64>()
                    .map_err(|_| usage(format!("invalid --{name} integer: {raw}")))?;
                if generation > Self::MAX_GENERATION {
                    return Err(usage(format!(
                        "invalid --{name} integer: {raw} (must be at most {})",
                        Self::MAX_GENERATION
                    )));
                }
                Ok(Some(generation))
            }
            None => Ok(None),
        }
    }
    fn require_generation(&self, name: &str) -> Result<u64, CliError> {
        self.generation(name)?
            .ok_or_else(|| usage(format!("missing --{name}")))
    }
}

/// Scan remaining tokens for one command. `options` lists command-specific
/// option names and whether each takes a value. Declared command options own
/// their local names (finding closure's --version is a review stamp).
fn scan(
    tokens: Tokens<'_>,
    g: &mut Globals,
    options: &[(&str, bool)],
) -> Result<Scanned, CliError> {
    let mut out = Scanned::default();
    while let Some(token) = tokens.next() {
        if token.starts_with("--") {
            let (name, inline) = split_option(&token);
            let option = options.iter().find(|(n, _)| *n == name);
            if option.is_none() && global_option(g, name, inline, &token, tokens)? {
                continue;
            }
            let Some(&(label, takes_value)) = option else {
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

fn parse_profile(raw: Option<&str>) -> Result<SearchMode, CliError> {
    match raw.unwrap_or("keyword") {
        "keyword" => Ok(SearchMode::Keyword),
        "semantic" => Ok(SearchMode::Semantic),
        "hybrid" => Ok(SearchMode::Hybrid),
        other => Err(usage(format!(
            "invalid --profile: {other} (expected keyword|semantic|hybrid)"
        ))),
    }
}

fn parse_scope(raw: Option<&str>) -> Result<KnowledgeScope, CliError> {
    match raw.unwrap_or("current") {
        "current" => Ok(KnowledgeScope::Current),
        "source" => Ok(KnowledgeScope::Source),
        "history" => Ok(KnowledgeScope::History),
        "all" => Ok(KnowledgeScope::All),
        other => Err(usage(format!(
            "invalid --scope: {other} (expected current|source|history|all)"
        ))),
    }
}

/// Keep parsed and directly constructed library commands under the same
/// contained-path rules before the worker opens operational storage.
pub(super) fn validate_library(command: &Command) -> Result<(), CliError> {
    let validate_path = |scope, path: &str| {
        match scope {
            KnowledgeScope::Current => VaultPath::parse(path).map(|_| ()),
            _ => EvidencePath::parse(path).map(|_| ()),
        }
        .map_err(|error| usage(error.to_string()))
    };
    match command {
        Command::NotesList {
            folder,
            cursor,
            scope,
        } => {
            if let Some(folder) = folder {
                match scope {
                    KnowledgeScope::Current => VaultPath::validate_folder(folder),
                    _ => EvidencePath::validate_folder(folder),
                }
                .map_err(|error| usage(error.to_string()))?;
            }
            if let Some(cursor) = cursor {
                validate_path(*scope, cursor)?;
            }
        }
        Command::NotePath { path, scope } => validate_path(*scope, path)?,
        Command::Search {
            limit: Some(limit), ..
        } if !(1..=50).contains(limit) => {
            return Err(usage("--limit must be between 1 and 50"));
        }
        _ => {}
    }
    Ok(())
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
    let mut command_word = None;
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
        if g.version {
            return Ok(Outcome::Version);
        }
        return Err(usage("expected a command; run brn --help"));
    };
    let scanned = match word.as_str() {
        "help" => return Ok(Outcome::Help),
        "edit" => editor::scan_command(&mut tokens, g, command)?,
        "identity" => identity::scan_command(&mut tokens, g, command)?,
        "evidence" => evidence::scan_command(&mut tokens, g, command)?,
        "provenance" => provenance::scan_command(&mut tokens, g, command)?,
        "links" => links::scan_command(&mut tokens, g, command)?,
        "findings" => findings::scan_command(&mut tokens, g, command)?,
        "actions" => actions::scan_command(&mut tokens, g, command)?,
        "inbox" => inbox::scan_command(&mut tokens, g, command)?,
        "relationships" => relationships::scan_command(&mut tokens, g, command)?,
        "proposals" => proposals::scan_command(&mut tokens, g, command)?,
        "activity" => activity::scan_command(&mut tokens, g, command)?,
        "ai" => ai::scan_command(&mut tokens, g, command)?,
        "models" => {
            if sub_word(&mut tokens, "models", "download")? != "download" {
                return Err(usage("unknown models subcommand"));
            }
            *command = Some("models.download");
            scan(
                &mut tokens,
                g,
                &[("approve-download", false), ("timeout-seconds", true)],
            )?
        }
        "notes" => {
            let sub = sub_word(&mut tokens, "notes", "list|show")?;
            match sub.as_str() {
                "list" => {
                    *command = Some("notes.list");
                    scan(
                        &mut tokens,
                        g,
                        &[("folder", true), ("cursor", true), ("scope", true)],
                    )?
                }
                "show" => {
                    *command = Some("notes.show");
                    scan(&mut tokens, g, &[("scope", true)])?
                }
                other => return Err(usage(format!("unknown notes subcommand: {other}"))),
            }
        }
        "status" => {
            *command = Some("status");
            scan(&mut tokens, g, &[])?
        }
        "search" => {
            *command = Some("search");
            scan(
                &mut tokens,
                g,
                &[("profile", true), ("limit", true), ("scope", true)],
            )?
        }
        "ask" => {
            *command = Some("ask");
            scan(
                &mut tokens,
                g,
                &[
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
        other => return Err(usage(format!("unknown command: {other}"))),
    };
    let built = match word.as_str() {
        "edit" => Command::Editor(editor::parse_command(command.unwrap(), &scanned)?),
        "identity" => Command::Identity(identity::parse_command(command.unwrap(), &scanned)?),
        "evidence" => Command::Evidence(evidence::parse_command(&scanned)?),
        "provenance" => Command::Provenance(provenance::parse_command(command.unwrap(), &scanned)?),
        "links" => Command::Links(links::parse_command(command.unwrap(), &scanned)?),
        "findings" => Command::Findings(findings::parse_command(command.unwrap(), &scanned)?),
        "actions" => Command::Actions(actions::parse_command(command.unwrap(), &scanned)?),
        "inbox" => Command::Inbox(inbox::parse_command(command.unwrap(), &scanned)?),
        "relationships" => Command::Relationships(relationships::parse_command(&scanned)?),
        "proposals" => Command::Proposals(proposals::parse_command(command.unwrap(), &scanned)?),
        "activity" => Command::Activity(activity::parse_command(&scanned)?),
        "ai" => Command::Ai(ai::parse_command(command.unwrap(), &scanned)?),
        "models" => {
            expect_positionals(&scanned, 0)?;
            if !scanned.flag("approve-download") {
                return Err(usage("models download requires --approve-download"));
            }
            Command::ModelDownload {
                timeout_seconds: scanned
                    .value("timeout-seconds")
                    .map(parse_timeout)
                    .transpose()?
                    .unwrap_or(300),
            }
        }
        "notes" => match command.unwrap() {
            "notes.list" => {
                expect_positionals(&scanned, 0)?;
                Command::NotesList {
                    folder: scanned.value("folder").map(str::to_owned),
                    cursor: scanned.value("cursor").map(str::to_owned),
                    scope: parse_scope(scanned.value("scope"))?,
                }
            }
            "notes.show" => {
                expect_positionals(&scanned, 1)?;
                let path = required_positional(&scanned, "PATH.md")?;
                Command::NotePath {
                    path: path.to_owned(),
                    scope: parse_scope(scanned.value("scope"))?,
                }
            }
            _ => unreachable!(),
        },
        "status" => {
            expect_positionals(&scanned, 0)?;
            Command::Status
        }
        "search" => {
            expect_positionals(&scanned, 1)?;
            Command::Search {
                query: required_positional(&scanned, "search QUERY")?.to_owned(),
                scope: parse_scope(scanned.value("scope"))?,
                profile: scanned
                    .value("profile")
                    .map(|p| parse_profile(Some(p)))
                    .transpose()?,
                limit: scanned
                    .value("limit")
                    .map(|raw| {
                        raw.parse::<usize>()
                            .ok()
                            .filter(|n| (1..=50).contains(n))
                            .ok_or_else(|| usage("--limit must be between 1 and 50"))
                    })
                    .transpose()?,
            }
        }
        "ask" => {
            expect_positionals(&scanned, 1)?;
            Command::Ask {
                question: required_positional(&scanned, "ask QUESTION")?.to_owned(),
                session: scanned.uuid("session")?,
                operation: scanned.uuid("operation")?,
                timeout_seconds: scanned
                    .value("timeout-seconds")
                    .map(parse_timeout)
                    .transpose()?
                    .unwrap_or(300),
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
        _ => unreachable!(),
    };
    validate_library(&built)?;
    // Post-parse version short-circuit (textual, exit 0, no --data-dir needed).
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
    let model_dir = g.model_dir.take().map(PathBuf::from);
    let vault = g.vault.take().map(PathBuf::from);
    let credentials_dir = g.credentials_dir.take().map(PathBuf::from);
    for (label, path) in [
        ("--model-dir", &model_dir),
        ("--vault", &vault),
        ("--credentials-dir", &credentials_dir),
    ] {
        if let Some(path) = path {
            if !path.is_absolute() {
                return Err(usage(format!("{label} must be an absolute path")));
            }
        }
    }
    if let Some(vault) = &vault {
        if !vault.is_dir()
            || vault
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(usage("--vault must be an existing regular directory"));
        }
    }
    Ok(Outcome::Run(Box::new(Invocation {
        json: g.json,
        data_dir,
        model_dir,
        vault,
        credentials_dir,
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
/// `context` is additive-optional and nested inside the existing error object.
pub fn envelope_err(
    command: Option<&str>,
    error: &CliError,
    context: Option<&serde_json::Value>,
) -> String {
    let mut err = serde_json::json!({ "code": error.code(), "message": error.message() });
    if let Some(context) = context {
        err["context"] = context.clone();
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "schema_version": 1,
        "command": command,
        "ok": false,
        "error": err,
    }))
    .expect("envelope serializes")
}

/// Print an error in the mode-appropriate surface. The error's exit code is
/// unaffected by report delivery (callers apply it); in `--json` mode a
/// non-pipe failure of the envelope write adds a best-effort stderr
/// `OUTPUT_DELIVERY_ERROR` line, and a closed consumer pipe stays quiet.
pub fn report_error(json: bool, command: Option<&str>, failure: &CliFailure) {
    let (_, diagnostic) = if json {
        let mut stdout = std::io::stdout();
        out::report_error_to(true, command, failure, &mut stdout)
    } else {
        let mut stderr = std::io::stderr();
        out::report_error_to(false, command, failure, &mut stderr)
    };
    emit_delivery_diagnostic(diagnostic.as_deref());
}

/// Best-effort stderr emit; a failing stderr never changes the outcome.
fn emit_delivery_diagnostic(diagnostic: Option<&str>) {
    use std::io::Write as _;
    if let Some(diagnostic) = diagnostic {
        let _ = writeln!(std::io::stderr(), "{diagnostic}");
    }
}

/// Map a command result to process exit, honoring the output mode.
///
/// Output policy: a closed or broken consumer pipe is not a domain failure —
/// a completed operation exits 0 quietly. Any other stdout write or flush
/// error after a completed operation exits 1 with a best-effort stderr
/// diagnostic identifying `OUTPUT_DELIVERY_ERROR`, saying the operation
/// completed but its result could not be delivered (with the operation id
/// when one is a string); nothing further is written to stdout. The error
/// path keeps the original nonzero exit code whatever happens to the report
/// write. Progress/delta writes to stderr are best-effort and never affect
/// lifecycle or outcome handling.
pub fn finish(json: bool, command: &str, result: Result<Output, CliFailure>) -> ExitCode {
    match result {
        Ok(output) => {
            let mut stdout = std::io::stdout();
            let (code, diagnostic) = out::finish_ok_to(json, command, output, &mut stdout);
            emit_delivery_diagnostic(diagnostic.as_deref());
            code
        }
        Err(failure) => {
            let code = failure.error.exit_code();
            report_error(json, Some(command), &failure);
            ExitCode::from(code)
        }
    }
}

/// Every operation uses the sole application owner.
pub fn execute(invocation: &Invocation) -> Result<Output, CliFailure> {
    if let Command::Findings(command) = &invocation.command {
        findings::validate(command)?;
    }
    if let Command::Actions(command) = &invocation.command {
        actions::validate(command)?;
    }
    library::validate_workspace(invocation)?;
    library::run(invocation)
}
