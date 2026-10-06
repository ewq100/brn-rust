//! H5 candidate: clap 4.6.7 replaces BRN's generic argv scanner for a
//! representative subset (globals, `help`, `status`, `search`, `notes show`,
//! nested `inbox add|list|show`, `findings close` with its local `--version`
//! and `models download` with a local boolean flag). BRN keeps its static help
//! text, the exact `--help`/`--json` pre-scan, typed validation,
//! data-directory admission and envelope policy.
//!
//! The file has three marked sections so the evaluation can count them:
//! DECLARATIONS (what replaces BRN's option tables), ADAPTER (compatibility
//! glue clap needs to reproduce BRN's observable contract) and TYPED (copied
//! from `crates/brn/src/cli`; unchanged by any scanner migration).
use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Help,
    Version,
    /// Parsing and admission succeeded; the real CLI would open storage now.
    Run {
        command: &'static str,
        json: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub message: String,
    pub json: bool,
    pub command: Option<&'static str>,
}

// ===== DECLARATIONS =====

const GLOBALS: &[(&str, bool)] = &[
    ("json", false),
    ("data-dir", true),
    ("model-dir", true),
    ("vault", true),
    ("credentials-dir", true),
    ("help", false),
    ("version", false),
];

struct Group {
    word: &'static str,
    expected: &'static str,
    unknown: fn(&str) -> String,
}

const GROUPS: &[Group] = &[
    Group {
        word: "notes",
        expected: "list|show",
        unknown: |word| format!("unknown notes subcommand: {word}"),
    },
    Group {
        word: "inbox",
        expected: "add|add-binary|show|review|removal-preview|remove-original|restore-original|original-removal|original-restore|original-operations|archived-analysis|list|process|processing|candidate|source|visual|interpret-visual|visual-annotation|cancel|analyze-actions|action-analysis|analyze|analysis",
        unknown: |_| "unknown Inbox subcommand".into(),
    },
    Group {
        word: "models",
        expected: "download",
        unknown: |_| "unknown models subcommand".into(),
    },
    Group {
        word: "findings",
        expected: "capture|list|show|inspect|close|conflicts",
        unknown: |_| "unknown findings subcommand".into(),
    },
];

struct Leaf {
    label: &'static str,
    group: Option<&'static str>,
    word: &'static str,
    options: &'static [(&'static str, bool)],
}

const LEAVES: &[Leaf] = &[
    Leaf {
        label: "status",
        group: None,
        word: "status",
        options: &[],
    },
    Leaf {
        label: "search",
        group: None,
        word: "search",
        options: &[("profile", true), ("limit", true), ("scope", true)],
    },
    Leaf {
        label: "notes.show",
        group: Some("notes"),
        word: "show",
        options: &[("scope", true)],
    },
    Leaf {
        label: "inbox.add",
        group: Some("inbox"),
        word: "add",
        options: &[
            ("id", true),
            ("kind", true),
            ("title", true),
            ("original-name", true),
            ("file", true),
        ],
    },
    Leaf {
        label: "inbox.list",
        group: Some("inbox"),
        word: "list",
        options: &[("limit", true), ("after", true)],
    },
    Leaf {
        label: "inbox.show",
        group: Some("inbox"),
        word: "show",
        options: &[],
    },
    Leaf {
        label: "findings.close",
        group: Some("findings"),
        word: "close",
        options: &[("version", true), ("state", true)],
    },
    Leaf {
        label: "models.download",
        group: Some("models"),
        word: "download",
        options: &[("approve-download", false), ("timeout-seconds", true)],
    },
];

/// BRN commands take at most two positionals; a third slot lets the typed
/// layer report the first unexpected one.
const SLOTS: [&str; 3] = ["p0", "p1", "p2"];

fn arg(name: &'static str, takes_value: bool) -> Arg {
    let arg = Arg::new(name).long(name);
    match (takes_value, name) {
        // BRN accepts single-dash text values (`--title -draft`); see ADAPTER.
        (true, _) => arg.action(ArgAction::Set).allow_hyphen_values(true),
        // BRN accepts repeated --help/--version; clap's SetTrue would refuse.
        (false, "help" | "version") => arg.action(ArgAction::Count),
        (false, _) => arg.action(ArgAction::SetTrue),
    }
}

/// Globals are declared per level, not `global(true)`: propagated globals
/// silently let a later level overwrite an earlier value, while BRN refuses
/// a duplicate before or after the command. A leaf-local name shadows a global.
fn tree() -> Command {
    fn globals(local: &'static [(&'static str, bool)]) -> impl Iterator<Item = Arg> {
        GLOBALS
            .iter()
            .filter(|(name, _)| !local.iter().any(|(l, _)| l == name))
            .map(|&(name, value)| arg(name, value))
    }
    let mut root = Command::new("brn")
        .no_binary_name(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .args(globals(&[]))
        .subcommand(
            Command::new("help").arg(
                Arg::new("rest")
                    .num_args(0..)
                    .allow_hyphen_values(true)
                    .trailing_var_arg(true),
            ),
        );
    for group in GROUPS {
        root = root.subcommand(Command::new(group.word).subcommand_required(true));
    }
    for leaf in LEAVES {
        let command = Command::new(leaf.word)
            .args(SLOTS.map(|slot| Arg::new(slot).num_args(1).allow_hyphen_values(true)))
            .args(leaf.options.iter().map(|&(name, value)| arg(name, value)))
            .args(globals(leaf.options));
        root = match leaf.group {
            None => root.subcommand(command),
            Some(group) => root.mut_subcommand(group, |g| g.subcommand(command)),
        };
    }
    root
}

// ===== ADAPTER =====

pub fn parse(args: &[String]) -> Result<Outcome, Failure> {
    // Kept from BRN: exact-token pre-scan; clap's own help action would lose
    // to earlier unknown/duplicate tokens and render clap-generated help.
    let json = args.iter().any(|a| a == "--json");
    if args.iter().any(|a| a == "--help") {
        return Ok(Outcome::Help);
    }
    let walk = walk(args);
    let command = match walk {
        Walk::Leaf(leaf) => Some(leaf.label),
        _ => None,
    };
    run(args, &walk).map_err(|message| Failure {
        message,
        json,
        command,
    })
}

/// How far BRN's root scan gets. clap errors carry no subcommand path, so
/// envelope identity and level-specific messages need this re-walk of the
/// root globals and command words.
enum Walk {
    Root,
    Group(&'static Group),
    Leaf(&'static Leaf),
}

fn walk(args: &[String]) -> Walk {
    let mut seen: Vec<&str> = Vec::new();
    let mut tokens = args.iter();
    let word = loop {
        let Some(token) = tokens.next() else {
            return Walk::Root;
        };
        let Some(rest) = token.strip_prefix("--") else {
            break token;
        };
        let (name, inline) = rest
            .split_once('=')
            .map_or((rest, None), |(n, v)| (n, Some(v)));
        let Some(&(name, takes_value)) = GLOBALS.iter().find(|(g, _)| *g == name) else {
            return Walk::Root;
        };
        let repeatable = matches!(name, "help" | "version");
        if (!takes_value && inline.is_some()) || (!repeatable && seen.contains(&name)) {
            return Walk::Root;
        }
        seen.push(name);
        if takes_value && inline.is_none() && !tokens.next().is_some_and(|v| !v.starts_with("--")) {
            return Walk::Root;
        }
    };
    if let Some(leaf) = LEAVES.iter().find(|l| l.group.is_none() && l.word == word) {
        return Walk::Leaf(leaf);
    }
    let Some(group) = GROUPS.iter().find(|g| g.word == word) else {
        return Walk::Root;
    };
    match tokens.next() {
        Some(sub) => LEAVES
            .iter()
            .find(|l| l.group == Some(group.word) && l.word == sub)
            .map_or(Walk::Group(group), Walk::Leaf),
        None => Walk::Group(group),
    }
}

fn run(args: &[String], walk: &Walk) -> Result<Outcome, String> {
    // clap treats a bare `--` as an escape and drops it; BRN refuses it.
    if args.iter().any(|a| a == "--") {
        return Err(match walk {
            Walk::Root => "unknown option: -- (only global options may precede the command)",
            _ => "unknown option: --",
        }
        .into());
    }
    let root = tree()
        .try_get_matches_from(args)
        .map_err(|error| clap_message(&error, args, walk))?;
    let Some((word, sub)) = root.subcommand() else {
        return match root.get_count("version") {
            0 => Err("expected a command; run brn --help".into()),
            _ => Ok(Outcome::Version),
        };
    };
    if word == "help" {
        return Ok(Outcome::Help);
    }
    let (leaf, matches) = match sub.subcommand() {
        Some((name, m)) => (find_leaf(Some(word), name), m),
        None => (find_leaf(None, word), sub),
    };
    let mut scanned = Scanned::default();
    let mut globals = Globals::default();
    for slot in SLOTS {
        if let Some(value) = matches.get_one::<String>(slot) {
            // A hyphen-accepting slot also swallows unknown long options.
            if value.starts_with("--") {
                return Err(format!("unknown option: {value}"));
            }
            scanned.positionals.push(value.clone());
        }
    }
    for &(name, takes_value) in leaf.options {
        if !takes_value {
            if matches.get_flag(name) {
                scanned.flags.push(name.to_string());
            }
        } else if let Some(value) = option_value(matches, name, args)? {
            scanned.values.push((name.to_string(), value));
        }
    }
    for &(name, takes_value) in GLOBALS {
        let local = leaf.options.iter().any(|(o, _)| *o == name);
        let levels: Vec<&ArgMatches> = if local {
            vec![&root]
        } else {
            vec![&root, matches]
        };
        if takes_value {
            let mut found = None;
            for level in levels {
                if let Some(value) = option_value(level, name, args)? {
                    if found.is_some() {
                        let token = raw_token(args, &format!("--{name}"), false);
                        return Err(format!("duplicate option: {token}"));
                    }
                    found = Some(value);
                }
            }
            globals.set(name, found);
        } else if name == "json" {
            let count = levels.iter().filter(|m| m.get_flag(name)).count();
            if count > 1 {
                return Err("duplicate option: --json".into());
            }
            globals.json = count == 1;
        } else if name == "version" {
            globals.version = levels.iter().any(|m| m.get_count(name) > 0);
        }
    }
    typed(leaf, &scanned, &mut globals)
}

fn find_leaf(group: Option<&str>, word: &str) -> &'static Leaf {
    LEAVES
        .iter()
        .find(|l| l.group == group && l.word == word)
        .expect("clap matched a declared leaf")
}

/// With hyphen values enabled clap accepts `--limit --json`; BRN only takes
/// a separate value that does not start with `--` (an inline `--x=--y` stays).
fn option_value(m: &ArgMatches, name: &str, args: &[String]) -> Result<Option<String>, String> {
    match m.get_one::<String>(name) {
        Some(value)
            if value.starts_with("--")
                && !args.iter().any(|a| *a == format!("--{name}={value}")) =>
        {
            Err(format!("missing value for --{name}"))
        }
        value => Ok(value.cloned()),
    }
}

/// clap reports `--name`; BRN echoes the exact offending token. A duplicate
/// is the last spelling of the name; a rejected inline value the first one.
fn raw_token(args: &[String], name: &str, inline: bool) -> String {
    let inline_name = format!("{name}=");
    let matches = |a: &&String| a.starts_with(&inline_name) || (!inline && *a == name);
    let found = if inline {
        args.iter().find(matches)
    } else {
        args.iter().rev().find(matches)
    };
    found.cloned().unwrap_or_else(|| name.to_string())
}

fn clap_message(error: &clap::Error, args: &[String], walk: &Walk) -> String {
    let context = |kind| match error.get(kind) {
        Some(ContextValue::String(s)) => s.clone(),
        _ => String::new(),
    };
    let invalid = context(ContextKind::InvalidArg);
    // clap renders `--name <name>` or, for counted flags, `--name...`.
    let option = invalid.split(' ').next().unwrap_or_default();
    let option = option.trim_end_matches("...");
    let raw = |name: &str| raw_token(args, name, false);
    match (error.kind(), walk) {
        (ErrorKind::UnknownArgument, Walk::Root) if option.starts_with("--") => format!(
            "unknown option: {} (only global options may precede the command)",
            raw(option)
        ),
        (ErrorKind::UnknownArgument, Walk::Root) => format!("unknown command: {invalid}"),
        // BRN takes a single-dash token as the (unknown) subcommand word.
        (ErrorKind::UnknownArgument, Walk::Group(group)) if !option.starts_with("--") => {
            (group.unknown)(&invalid)
        }
        (ErrorKind::UnknownArgument | ErrorKind::MissingSubcommand, Walk::Group(group)) => {
            format!("missing {0} subcommand ({1})", group.word, group.expected)
        }
        (ErrorKind::UnknownArgument, _) if option.starts_with("--") => {
            format!("unknown option: {}", raw(option))
        }
        (ErrorKind::UnknownArgument, _) => format!("unexpected argument: {invalid}"),
        (ErrorKind::InvalidSubcommand, Walk::Group(group)) => {
            (group.unknown)(&context(ContextKind::InvalidSubcommand))
        }
        (ErrorKind::InvalidSubcommand, _) => {
            format!(
                "unknown command: {}",
                context(ContextKind::InvalidSubcommand)
            )
        }
        (ErrorKind::ArgumentConflict, _) => format!("duplicate option: {}", raw(option)),
        (ErrorKind::InvalidValue, _) => format!("missing value for {option}"),
        // BRN names a leaf-local flag by label but echoes a global's token.
        (ErrorKind::TooManyValues, Walk::Leaf(leaf))
            if leaf
                .options
                .iter()
                .any(|(o, _)| Some(*o) == option.strip_prefix("--")) =>
        {
            format!("{option} does not take a value")
        }
        (ErrorKind::TooManyValues, _) => {
            format!("{} does not take a value", raw_token(args, option, true))
        }
        _ => error.to_string(),
    }
}

// ===== TYPED (copied BRN behavior for the subset; unchanged by migration) =====

#[derive(Default)]
struct Globals {
    json: bool,
    data_dir: Option<String>,
    model_dir: Option<String>,
    vault: Option<String>,
    credentials_dir: Option<String>,
    version: bool,
}

impl Globals {
    fn set(&mut self, name: &str, value: Option<String>) {
        match name {
            "data-dir" => self.data_dir = value,
            "model-dir" => self.model_dir = value,
            "vault" => self.vault = value,
            _ => self.credentials_dir = value,
        }
    }
}

#[derive(Default)]
struct Scanned {
    positionals: Vec<String>,
    flags: Vec<String>,
    values: Vec<(String, String)>,
}

impl Scanned {
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
    fn uuid(&self, name: &str) -> Result<Option<Uuid>, String> {
        match self.value(name) {
            Some(raw) => Uuid::parse_str(raw)
                .map(Some)
                .map_err(|_| format!("invalid --{name} UUID: {raw}")),
            None => Ok(None),
        }
    }
    fn require_generation(&self, name: &str) -> Result<u64, String> {
        let raw = self
            .value(name)
            .ok_or_else(|| format!("missing --{name}"))?;
        let generation = raw
            .parse::<u64>()
            .map_err(|_| format!("invalid --{name} integer: {raw}"))?;
        if generation > Self::MAX_GENERATION {
            return Err(format!(
                "invalid --{name} integer: {raw} (must be at most {})",
                Self::MAX_GENERATION
            ));
        }
        Ok(generation)
    }
}

fn expect_positionals(s: &Scanned, count: usize) -> Result<(), String> {
    match s.positionals.get(count) {
        Some(extra) => Err(format!("unexpected argument: {extra}")),
        None => Ok(()),
    }
}

fn positional_uuid(s: &Scanned, index: usize, label: &str) -> Result<Uuid, String> {
    let raw = s
        .positionals
        .get(index)
        .ok_or_else(|| format!("missing {label}"))?;
    Uuid::parse_str(raw).map_err(|_| format!("invalid {label} UUID: {raw}"))
}

fn required_positional<'a>(s: &'a Scanned, label: &str) -> Result<&'a str, String> {
    s.positionals
        .first()
        .map(String::as_str)
        .ok_or_else(|| format!("missing {label}"))
}

fn scope(raw: Option<&str>) -> Result<(), String> {
    match raw.unwrap_or("current") {
        "current" | "source" | "history" | "all" => Ok(()),
        other => Err(format!(
            "invalid --scope: {other} (expected current|source|history|all)"
        )),
    }
}

fn label(value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
        return Err("Inbox labels must be visible and at most 512 UTF-8 bytes".into());
    }
    Ok(())
}

fn typed(leaf: &Leaf, s: &Scanned, g: &mut Globals) -> Result<Outcome, String> {
    match leaf.label {
        "status" => expect_positionals(s, 0)?,
        "search" => {
            expect_positionals(s, 1)?;
            required_positional(s, "search QUERY")?;
            scope(s.value("scope"))?;
            if let Some(profile) = s.value("profile") {
                if !matches!(profile, "keyword" | "semantic" | "hybrid") {
                    return Err(format!(
                        "invalid --profile: {profile} (expected keyword|semantic|hybrid)"
                    ));
                }
            }
            if let Some(raw) = s.value("limit") {
                raw.parse::<usize>()
                    .ok()
                    .filter(|n| (1..=50).contains(n))
                    .ok_or("--limit must be between 1 and 50")?;
            }
        }
        "notes.show" => {
            expect_positionals(s, 1)?;
            required_positional(s, "PATH.md")?;
            scope(s.value("scope"))?;
        }
        "inbox.add" => {
            expect_positionals(s, 0)?;
            if !matches!(
                s.value("kind").unwrap_or("text"),
                "text" | "markdown" | "email" | "teams"
            ) {
                return Err("--kind must be text|markdown|email|teams".into());
            }
            let id = s.uuid("id")?.ok_or("missing --id")?;
            let title = s.value("title").ok_or("missing --title")?;
            s.value("file").ok_or("missing --file")?;
            if id.is_nil() {
                return Err("Inbox needs a nonnil UUID and exact UTF-8 text up to 1 MiB".into());
            }
            label(title)?;
            s.value("original-name").map(label).transpose()?;
        }
        "inbox.list" => {
            expect_positionals(s, 0)?;
            if let Some(v) = s.value("limit") {
                v.parse::<usize>().map_err(|_| "invalid --limit integer")?;
            }
            s.uuid("after")?;
        }
        "inbox.show" => {
            expect_positionals(s, 1)?;
            if positional_uuid(s, 0, "UUID")?.is_nil() {
                return Err("Inbox UUID must not be nil".into());
            }
        }
        "findings.close" => {
            expect_positionals(s, 1)?;
            positional_uuid(s, 0, "UUID")?;
            s.require_generation("version")?;
            let state = s.value("state").ok_or("missing --state")?;
            if !matches!(state, "open" | "resolved" | "dismissed") {
                return Err("--state must be open|resolved|dismissed|all for listing, or resolved|dismissed for closure".into());
            }
        }
        "models.download" => {
            expect_positionals(s, 0)?;
            if !s.flag("approve-download") {
                return Err("models download requires --approve-download".into());
            }
            if let Some(raw) = s.value("timeout-seconds") {
                let seconds: u64 = raw
                    .parse()
                    .map_err(|_| format!("invalid --timeout-seconds: {raw}"))?;
                if !(1..=3600).contains(&seconds) {
                    return Err(format!(
                        "--timeout-seconds must be between 1 and 3600: {raw}"
                    ));
                }
            }
        }
        _ => unreachable!("declared leaf"),
    }
    if g.version {
        return Ok(Outcome::Version);
    }
    let raw = g
        .data_dir
        .take()
        .ok_or("missing --data-dir (required for commands)")?;
    let data_dir = PathBuf::from(&raw);
    if !data_dir.is_absolute() {
        return Err(format!("--data-dir must be an absolute path: {raw}"));
    }
    if !data_dir.exists() {
        return Err(format!("--data-dir does not exist: {raw}"));
    }
    if !data_dir.is_dir() {
        return Err(format!("--data-dir is not a directory: {raw}"));
    }
    for (name, path) in [
        ("--model-dir", &g.model_dir),
        ("--vault", &g.vault),
        ("--credentials-dir", &g.credentials_dir),
    ] {
        if path
            .as_deref()
            .is_some_and(|p| !PathBuf::from(p).is_absolute())
        {
            return Err(format!("{name} must be an absolute path"));
        }
    }
    if let Some(vault) = g.vault.as_deref().map(PathBuf::from) {
        if !vault.is_dir()
            || vault
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err("--vault must be an existing regular directory".into());
        }
    }
    Ok(Outcome::Run {
        command: leaf.label,
        json: g.json,
    })
}
