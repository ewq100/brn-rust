//! The command table: one entry per command, read by `parse()`, the envelope
//! and `brn --json --help`, so the grammar and the description cannot drift.
use crate::contract::{CliError, Code, Status};
use crate::Invocation;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// What an identical retry of the same argv does to state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Class {
    Read,
    Replay,
    Converge,
    Refuse,
    Duplicate,
}
impl Class {
    pub(crate) const ALL: [Class; 5] = [
        Class::Read,
        Class::Replay,
        Class::Converge,
        Class::Refuse,
        Class::Duplicate,
    ];
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Class::Read => "read",
            Class::Replay => "replay",
            Class::Converge => "converge",
            Class::Refuse => "refuse",
            Class::Duplicate => "duplicate",
        }
    }
    fn meaning(self) -> &'static str {
        match self {
            Class::Read => "Changes nothing (a fresh data directory may have been initialised).",
            Class::Replay => "The retry uses the same operation identity. If that operation was applied, the retry returns its receipt (same operation) and changes nothing; a replayed receipt shows the original versions, even after later edits. If it was not applied (the first attempt was Deferred, Stale, failed, interrupted or lost before commit), the retry re-attempts exactly that request. Under that identity it applies at most once and never creates another record, message or candidate.",
            Class::Converge => "Reaches the same end state. It may record a new operation and bump versions, but creates no new records and leaves no unapplied candidate. A no-op archive, resolve, unarchive, chat-move, project-link or project-unlink returns Unchanged and records nothing; a keyed retry of an earlier change still replays its receipt. init and upgrade on a current workspace record nothing. For reclaim-edit, a retry reaches the same session and buffer but rotates the token: only the latest returned token is valid; discard any earlier one. model-install into a destination that already holds the verified model downloads nothing and returns downloaded_bytes: 0.",
            Class::Refuse => "At most once, never a duplicate. If the first attempt took effect, an identical retry changes nothing and returns a non-ok status (stale or rejected) instead of the original result. If it did not take effect (for example interrupted, internal, or a crash before commit), the retry performs the change once and can return ok. After a lost response, read state (read, projects, recovery, or check the destination) to tell which happened. Every usage and rejected envelope also reports refuse: the same argv meets the same refusal while the state that caused it is unchanged.",
            Class::Duplicate => "Repeats the change: another record, message, model turn or unapplied candidate. Never retry blindly.",
        }
    }
}

/// Where the envelope finds the operation and the changed records (README table 4.2b).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReceiptAt {
    None,
    /// The result is a Receipt.
    Root,
    /// `result.receipt` when the outcome is Applied or Saved.
    Field,
    /// The command arm reports the receipt itself.
    Arm,
    /// `topics-create`: the created note plus the pointer's ApplyOutcome.
    NoteAndPointer,
    /// `result.terminal` of a provider run.
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    No,
    Yes,
    WithOptions(&'static [&'static str]),
}

pub(crate) struct RequiredWhen {
    pub(crate) option: &'static str,
    pub(crate) equals: &'static str,
    pub(crate) required: &'static [&'static str],
}

pub(crate) struct Variant {
    pub(crate) option: &'static str,
    pub(crate) equals: &'static str,
    pub(crate) retry: Class,
    /// `false`: `--key` is refused for this variant.
    pub(crate) key: bool,
    /// The class under `--key`, when the variant takes a key.
    pub(crate) keyed: Option<Class>,
    /// Classification override; not part of the description.
    pub(crate) receipt: Option<ReceiptAt>,
}

pub(crate) struct Retry {
    pub(crate) default: Class,
    pub(crate) keyed: Option<Class>,
    pub(crate) variants: &'static [Variant],
}

pub(crate) struct Spec {
    pub(crate) name: &'static str,
    pub(crate) alias_of: Option<&'static str>,
    pub(crate) options: &'static [&'static str],
    /// Checked in this order; `NAME-file` satisfies `NAME`.
    pub(crate) required: &'static [&'static str],
    pub(crate) required_when: &'static [RequiredWhen],
    pub(crate) keyed: bool,
    pub(crate) owner: Owner,
    pub(crate) retry: Retry,
    pub(crate) outcomes: &'static [&'static str],
    pub(crate) receipt: ReceiptAt,
    /// Page options and cursor form, when the command pages.
    pub(crate) page: Option<PageSpec>,
    /// `Some(true)` for bounded page commands; `Some(false)` for lists that
    /// return everything without page options (see `bounded_form`).
    pub(crate) bounded: Option<bool>,
    /// The bounded form of an unbounded list, when it has one.
    pub(crate) bounded_form: Option<&'static str>,
    /// `(OPTION, NEEDS)`: OPTION is accepted only together with NEEDS.
    pub(crate) needs: &'static [(&'static str, &'static str)],
    /// `--key` is accepted only with `--OPTION VALUE`.
    pub(crate) key_requires: Option<(&'static str, &'static str)>,
}

/// The `--limit` range and `--cursor` form of a page command.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PageSpec {
    pub(crate) min: u64,
    pub(crate) max: u64,
    pub(crate) default: Option<u64>,
    pub(crate) cursor: CursorForm,
}

/// The syntax of an opaque page position, checked before opening.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CursorForm {
    /// A receipt or operation position: 1–18 digits.
    Rowid,
    /// A record ID: `[A-Za-z0-9._:-]{1,256}`.
    Id,
    /// A note version, a decimal of at least 1.
    Version,
    /// A byte offset, a decimal.
    Byte,
    /// `VERSION.START` of a paged `read`.
    VersionStart,
    /// The core search cursor, passed back unchanged.
    Opaque,
}
impl CursorForm {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            CursorForm::Rowid => "rowid",
            CursorForm::Id => "id",
            CursorForm::Version => "version",
            CursorForm::Byte => "byte",
            CursorForm::VersionStart => "version.start",
            CursorForm::Opaque => "opaque",
        }
    }
    fn accepts(self, value: &str) -> bool {
        let digits = |v: &str, max: usize| {
            (1..=max).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_digit())
        };
        match self {
            CursorForm::Rowid | CursorForm::Byte => digits(value, 18),
            CursorForm::Id => {
                (1..=256).contains(&value.len())
                    && value.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-')
                    })
            }
            CursorForm::Version => digits(value, 18) && value.parse::<u64>().is_ok_and(|v| v >= 1),
            CursorForm::VersionStart => value.split_once('.').is_some_and(|(v, start)| {
                digits(v, 18) && !v.starts_with('0') && digits(start, 18)
            }),
            CursorForm::Opaque => !value.is_empty(),
        }
    }
    fn expected(self) -> &'static str {
        match self {
            CursorForm::Rowid => "a position from a previous page (digits)",
            CursorForm::Id => "a record ID from a previous page",
            CursorForm::Version => "a note version from a previous page",
            CursorForm::Byte => "a byte offset from a previous page",
            CursorForm::VersionStart => "VERSION.START from a previous page",
            CursorForm::Opaque => "the cursor from a previous page",
        }
    }
}

/// The value grammar of one option, checked before the workspace opens.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Grammar {
    Enum(&'static [&'static str]),
    Int(u64, u64),
    Cursor(CursorForm),
    /// An absolute filesystem path.
    AbsolutePath,
    /// An existing after-open check moved before opening, with its message:
    /// a decimal that parses as an integer up to `max`.
    Number {
        max: u64,
        message: &'static str,
    },
    /// An existing after-open choice moved before opening, with its message.
    Choice {
        values: &'static [&'static str],
        ignore_case: bool,
        message: &'static str,
    },
    /// A service range moved before opening. A value that does not parse as
    /// an integer is `invalid_value` with `syntax`; a value outside
    /// `min..=max` gets `refusal(value)`, the exact error the service (or the
    /// CLI's own conversion) returned after opening.
    Service {
        syntax: &'static str,
        min: u64,
        max: u64,
        refusal: fn(u64) -> CliError,
    },
}
impl Grammar {
    fn accepts(self, value: &str) -> bool {
        match self {
            Grammar::Enum(values) => values.contains(&value),
            Grammar::Int(min, max) => {
                (1..=20).contains(&value.len())
                    && value.bytes().all(|b| b.is_ascii_digit())
                    && value.parse::<u64>().is_ok_and(|v| (min..=max).contains(&v))
            }
            Grammar::Cursor(form) => form.accepts(value),
            Grammar::AbsolutePath => std::path::Path::new(value).is_absolute(),
            Grammar::Number { max, .. } => value.parse::<u64>().is_ok_and(|v| v <= max),
            Grammar::Service { min, max, .. } => {
                value.parse::<u64>().is_ok_and(|v| (min..=max).contains(&v))
            }
            Grammar::Choice {
                values,
                ignore_case,
                ..
            } => {
                let value = if ignore_case {
                    value.to_lowercase()
                } else {
                    value.to_owned()
                };
                values.contains(&value.as_str())
            }
        }
    }
    fn expected(self, option: &str) -> String {
        match self {
            Grammar::Number { message, .. } | Grammar::Choice { message, .. } => message.to_owned(),
            Grammar::Service { syntax, .. } => syntax.to_owned(),
            Grammar::Enum(values) => format!("--{option} takes {}", values.join("|")),
            Grammar::Int(min, max) => format!("--{option} takes an integer {min}..{max}"),
            Grammar::Cursor(form) => format!("--{option} takes {}", form.expected()),
            Grammar::AbsolutePath => format!("--{option} must be an absolute path"),
        }
    }
    fn describe(self, option: &str) -> Value {
        match self {
            Grammar::Enum(values) => json!({"option": option, "grammar": "enum", "values": values}),
            Grammar::Int(min, max) => {
                json!({"option": option, "grammar": "integer", "min": min, "max": max})
            }
            Grammar::Cursor(form) => {
                json!({"option": option, "grammar": "cursor", "form": form.as_str()})
            }
            Grammar::Number { max, .. } => {
                json!({"option": option, "grammar": "integer", "min": 0, "max": max})
            }
            Grammar::Service { min, max, .. } => {
                json!({"option": option, "grammar": "integer", "min": min, "max": max})
            }
            Grammar::AbsolutePath => json!({"option": option, "grammar": "absolute_path"}),
            Grammar::Choice {
                values,
                ignore_case,
                ..
            } => {
                json!({"option": option, "grammar": "enum", "values": values, "ignore_case": ignore_case})
            }
        }
    }
}

/// `(command, option, names, note)`: an option that records an owner decision
/// when it names that kind of record.
pub(crate) const OWNER_DECISION_WHEN: &[(&str, &str, &str, &str)] = &[(
    "intake",
    "thread",
    "Project",
    "attaches the document to that project",
)];
pub(crate) const RUN_STATES: &[&str] =
    &["working", "interrupted", "completed", "cancelled", "failed"];
const PAGE_100: Grammar = Grammar::Int(1, 100);
const fn integer(message: &'static str) -> Grammar {
    Grammar::Number {
        max: u64::MAX,
        message,
    }
}
const BASE: Grammar = integer("--base must be an integer");
const GENERATION: Grammar = integer("--generation must be an integer");
const PROVIDER: Grammar = Grammar::Choice {
    values: &["codex", "chatgpt", "copilot"],
    ignore_case: true,
    message: "Provider must be Codex or Copilot",
};
const EFFORT: Grammar = Grammar::Choice {
    values: &["low", "medium", "high"],
    ignore_case: false,
    message: "Effort must be low, medium or high",
};
/// `connections --hops`: 1 or 2 (the search service's range). Past 255 the
/// CLI's `u8` conversion refused it; 0 and 3..=255 reached the service.
const HOPS: Grammar = Grammar::Service {
    syntax: "--hops must be an integer",
    min: 1,
    max: 2,
    refusal: hops_refusal,
};
fn hops_refusal(value: u64) -> CliError {
    match u8::try_from(value) {
        Err(error) => CliError::invalid_value("hops", error.to_string()),
        Ok(_) => {
            brn_threads_app::AppError::Invalid("connections need a note ID and 1 or 2 hops".into())
                .into()
        }
    }
}
/// `ask --budget`: 1..32 tool rounds (the run service's range, `begin` in
/// brn-ai `threads.rs`). Past 65535 the CLI's `u16` parse refused it.
const BUDGET: Grammar = Grammar::Service {
    syntax: "Invalid budget",
    min: 1,
    max: 32,
    refusal: budget_refusal,
};
fn budget_refusal(value: u64) -> CliError {
    if value > u64::from(u16::MAX) {
        return CliError::invalid_value("budget", "Invalid budget");
    }
    crate::contract::run_error(brn_ai::AiError::new(brn_ai::AiErrorKind::ToolRejected))
}
const NOTE_BYTES: Grammar = Grammar::Int(1, brn_threads_app::MAX_NOTE_READ_BYTES as u64);
/// `(command, option, grammar)`, checked in `parse()` for every present option.
pub(crate) static VALUES: &[(&str, &str, Grammar)] = &[
    ("history", "limit", PAGE_100),
    ("history", "cursor", Grammar::Cursor(CursorForm::Rowid)),
    ("candidates", "limit", PAGE_100),
    ("candidates", "cursor", Grammar::Cursor(CursorForm::Rowid)),
    (
        "records",
        "kind",
        Grammar::Enum(&brn_threads_app::RECORD_KINDS),
    ),
    ("records", "limit", PAGE_100),
    ("records", "cursor", Grammar::Cursor(CursorForm::Id)),
    ("revisions", "limit", Grammar::Int(1, 32)),
    ("revisions", "cursor", Grammar::Cursor(CursorForm::Version)),
    ("revision", "version", Grammar::Cursor(CursorForm::Version)),
    ("revision", "limit", NOTE_BYTES),
    ("revision", "cursor", Grammar::Cursor(CursorForm::Byte)),
    ("notes", "limit", PAGE_100),
    ("notes", "cursor", Grammar::Cursor(CursorForm::Id)),
    ("threads", "limit", PAGE_100),
    ("threads", "cursor", Grammar::Cursor(CursorForm::Id)),
    ("needs-you", "limit", PAGE_100),
    ("needs-you", "cursor", Grammar::Cursor(CursorForm::Id)),
    ("recovery", "limit", PAGE_100),
    ("recovery", "cursor", Grammar::Cursor(CursorForm::Id)),
    ("projects", "limit", PAGE_100),
    ("projects", "cursor", Grammar::Cursor(CursorForm::Id)),
    ("messages", "limit", PAGE_100),
    ("messages", "cursor", Grammar::Cursor(CursorForm::Rowid)),
    ("runs", "state", Grammar::Enum(RUN_STATES)),
    ("runs", "limit", PAGE_100),
    ("runs", "cursor", Grammar::Cursor(CursorForm::Id)),
    ("read", "limit", NOTE_BYTES),
    ("read", "cursor", Grammar::Cursor(CursorForm::VersionStart)),
    // Checks that ran after opening before round 2, with the same messages.
    ("project-settings", "base", BASE),
    ("project-restore", "base", BASE),
    ("begin-edit", "base", BASE),
    ("keep-in-brn", "base", BASE),
    ("ask", "base", BASE),
    ("update-buffer", "generation", GENERATION),
    ("save", "generation", GENERATION),
    ("discard", "generation", GENERATION),
    ("connections", "hops", HOPS),
    ("search", "limit", integer("--limit must be an integer")),
    ("ask", "budget", BUDGET),
    (
        "projects",
        "trash",
        Grammar::Choice {
            values: &["yes", "no"],
            ignore_case: false,
            message: "--trash takes yes|no",
        },
    ),
    (
        "project-settings",
        "knowledge",
        Grammar::Choice {
            values: &["yes", "no"],
            ignore_case: false,
            message: "--knowledge takes yes|no",
        },
    ),
    (
        "knowledge-check",
        "seed-chat",
        Grammar::Choice {
            values: &["yes"],
            ignore_case: false,
            message: "--seed-chat takes yes",
        },
    ),
    (
        "connections",
        "sources",
        Grammar::Choice {
            values: &["yes"],
            ignore_case: false,
            message: "--sources takes yes",
        },
    ),
    (
        "intake",
        "intent",
        Grammar::Choice {
            values: &["useful", "full", "chat", "project"],
            ignore_case: false,
            message: "Choose useful, full, or chat with --thread",
        },
    ),
    (
        "document-conversion",
        "converter",
        Grammar::Enum(&["xberg", "xberg-brn"]),
    ),
    ("document-conversion", "ai", Grammar::Enum(&["off", "on"])),
    ("configure", "provider", PROVIDER),
    ("configure", "helper-provider", PROVIDER),
    ("configure", "effort", EFFORT),
    ("configure", "helper-effort", EFFORT),
    // Help always said ABSOLUTE; a relative path is refused before opening.
    ("backup", "destination", Grammar::AbsolutePath),
    ("export", "destination", Grammar::AbsolutePath),
    ("restore", "source", Grammar::AbsolutePath),
    ("skill-install", "source", Grammar::AbsolutePath),
    ("upgrade", "backup-directory", Grammar::AbsolutePath),
    ("init", "purpose", Grammar::Enum(&["test"])),
    ("begin-edit", "token-out", Grammar::AbsolutePath),
    ("reclaim-edit", "token-out", Grammar::AbsolutePath),
    ("model-install", "destination", Grammar::AbsolutePath),
];

/// The first present option whose value does not fit its grammar.
pub(crate) fn check_values(
    command: &str,
    options: &BTreeMap<String, String>,
) -> Result<(), CliError> {
    for (name, option, grammar) in VALUES {
        if *name != command {
            continue;
        }
        if let Some(value) = options.get(*option) {
            if !grammar.accepts(value) {
                if let (Grammar::Service { refusal, .. }, Ok(number)) =
                    (grammar, value.parse::<u64>())
                {
                    return Err(refusal(number));
                }
                let expected = grammar.expected(option);
                return Err(CliError::invalid_value(option, expected));
            }
        }
    }
    Ok(())
}
fn values_of(command: &str) -> Vec<Value> {
    VALUES
        .iter()
        .filter(|(name, _, _)| *name == command)
        .map(|(_, option, grammar)| grammar.describe(option))
        .collect()
}

/// Text options that also accept `--NAME-file ABSOLUTE_PATH|-`, read in this order.
pub(crate) const TEXT_OPTIONS: [&str; 5] = ["title", "text", "prompt", "instructions", "actions"];
pub(crate) const KEY_PATTERN: &str = "^[A-Za-z0-9._:-]{1,128}$";
pub(crate) const ACTOR_PATTERN: &str = "^[A-Za-z0-9._:-]{1,64}$";
pub(crate) const ACTOR_ENV: &str = "BRN_ACTOR";
pub(crate) const TOKEN_PATTERN: &str = "^brnet1_[0-9a-f]{64}$";
pub(crate) const OWNER_DECISION_RULE: &str = "These commands record an owner decision. Running the CLI does not make a caller the owner. Run them only on the owner's explicit instruction for that exact operation.";

pub(crate) const APPLY_OUTCOMES: &[&str] =
    &["Applied", "Stale", "Deferred", "NeedsReview", "Superseded"];
pub(crate) const SAVE_OUTCOMES: &[&str] =
    &["Saved", "Stale", "GenerationMismatch", "Closed", "Deferred"];
pub(crate) const BUFFER_OUTCOMES: &[&str] = &["Updated", "Ignored"];
pub(crate) const DISCARD_OUTCOMES: &[&str] = &["Discarded", "Ignored"];
pub(crate) const DISMISS_OUTCOMES: &[&str] = &["Dismissed"];
pub(crate) const RECEIPT_OUTCOMES: &[&str] = &["Applied"];
/// archive, resolve, unarchive, chat-move, project-link, project-unlink.
pub(crate) const CONVERGE_OUTCOMES: &[&str] = &["Applied", "Unchanged"];
pub(crate) const TERMINAL_OUTCOMES: &[&str] = &["Completed", "Interrupted", "Failed"];
/// `read --limit`: a NoteReadOutcome.
pub(crate) const READ_OUTCOMES: &[&str] = &["Read", "Stale"];
/// `intake` returns an ApplyOutcome, or a run terminal with `--intent useful`.
pub(crate) const INTAKE_OUTCOMES: &[&str] = &[
    "Applied",
    "Stale",
    "Deferred",
    "NeedsReview",
    "Superseded",
    "Completed",
    "Interrupted",
    "Failed",
];

#[allow(clippy::too_many_arguments)]
const fn cmd(
    name: &'static str,
    options: &'static [&'static str],
    required: &'static [&'static str],
    owner: Owner,
    default: Class,
    keyed: Option<Class>,
    receipt: ReceiptAt,
    outcomes: &'static [&'static str],
) -> Spec {
    Spec {
        name,
        alias_of: None,
        options,
        required,
        required_when: &[],
        keyed: keyed.is_some(),
        owner,
        retry: Retry {
            default,
            keyed,
            variants: &[],
        },
        outcomes,
        receipt,
        page: None,
        bounded: None,
        bounded_form: None,
        needs: &[],
        key_requires: None,
    }
}
const fn paged(min: u64, max: u64, default: Option<u64>, cursor: CursorForm) -> Option<PageSpec> {
    Some(PageSpec {
        min,
        max,
        default,
        cursor,
    })
}
/// A list that prints everything without page options and pages with
/// `--limit N [--cursor ID]` (record or session ID order).
const fn list(name: &'static str, bounded_form: &'static str) -> Spec {
    Spec {
        page: paged(1, 100, Some(20), CursorForm::Id),
        bounded: Some(false),
        bounded_form: Some(bounded_form),
        ..read(name, &["limit", "cursor"], &[])
    }
}

use Class::{Converge, Duplicate, Read, Refuse, Replay};
const NO: Owner = Owner::No;
const YES: Owner = Owner::Yes;
const NONE: ReceiptAt = ReceiptAt::None;

const fn read(
    name: &'static str,
    options: &'static [&'static str],
    required: &'static [&'static str],
) -> Spec {
    cmd(name, options, required, NO, Read, None, NONE, &[])
}
const fn project(
    name: &'static str,
    options: &'static [&'static str],
    required: &'static [&'static str],
    default: Class,
) -> Spec {
    cmd(
        name,
        options,
        required,
        YES,
        default,
        Some(Replay),
        ReceiptAt::Root,
        RECEIPT_OUTCOMES,
    )
}
const fn setting(
    name: &'static str,
    options: &'static [&'static str],
    required: &'static [&'static str],
) -> Spec {
    cmd(
        name,
        options,
        required,
        YES,
        Converge,
        None,
        ReceiptAt::Field,
        APPLY_OUTCOMES,
    )
}

const ARCHIVE: Spec = cmd(
    "archive",
    &["thread", "key"],
    &["thread"],
    YES,
    Converge,
    Some(Replay),
    ReceiptAt::Root,
    CONVERGE_OUTCOMES,
);
const CONFIGURE_OPTIONS: &[&str] = &[
    "provider",
    "model",
    "effort",
    "auth-file",
    "credentials-dir",
    "helper",
    "helper-provider",
    "helper-model",
    "helper-effort",
    "helper-auth-file",
    "helper-credentials-dir",
];

pub(crate) static COMMANDS: &[Spec] = &[
    Spec {
        retry: Retry {
            default: Converge,
            keyed: None,
            variants: &[Variant {
                option: "purpose",
                equals: "test",
                retry: Refuse,
                key: false,
                keyed: None,
                receipt: None,
            }],
        },
        ..cmd("init", &["purpose"], &[], NO, Converge, None, NONE, &[])
    },
    Spec {
        page: paged(1, 100, Some(20), CursorForm::Id),
        bounded: Some(false),
        bounded_form: Some("records --limit N [--kind KIND] [--cursor ID]"),
        ..read("records", &["kind", "limit", "cursor"], &[])
    },
    list("notes", "notes --limit N [--cursor ID]"),
    list("threads", "threads --limit N [--cursor ID]"),
    list("needs-you", "needs-you --limit N [--cursor ID]"),
    Spec {
        page: paged(1, 100, Some(20), CursorForm::Rowid),
        bounded: Some(false),
        bounded_form: Some("history --limit N [--record ID] [--cursor C]"),
        ..read("history", &["record", "limit", "cursor"], &[])
    },
    Spec {
        page: paged(1, 100, Some(20), CursorForm::Rowid),
        bounded: Some(false),
        bounded_form: Some("candidates --limit N [--cursor C]"),
        ..read("candidates", &["limit", "cursor"], &[])
    },
    read("skills", &["project"], &[]),
    read("skill-inspect", &["name", "project"], &["name"]),
    cmd(
        "skill-install",
        &["source"],
        &["source"],
        YES,
        Converge,
        None,
        NONE,
        &[],
    ),
    cmd(
        "skill-remove",
        &["name", "digest"],
        &["name", "digest"],
        YES,
        Converge,
        None,
        NONE,
        &[],
    ),
    setting(
        "skill-enable",
        &["id", "digest", "project"],
        &["id", "digest"],
    ),
    setting(
        "skill-disable",
        &["id", "digest", "project"],
        &["id", "digest"],
    ),
    setting("skill-select", &["id", "project"], &["id"]),
    setting("skill-clear", &["id", "project"], &["id"]),
    read("mcp-status", &["thread"], &[]),
    cmd(
        "mcp-discover",
        &["endpoint"],
        &["endpoint"],
        YES,
        Converge,
        None,
        NONE,
        &[],
    ),
    setting("mcp-configure", &["policy"], &["policy"]),
    cmd(
        "mcp-disable",
        &[],
        &[],
        YES,
        Converge,
        None,
        ReceiptAt::Field,
        CONVERGE_OUTCOMES,
    ),
    read("settings", &[], &[]),
    setting("pause", &[], &[]),
    setting("resume", &[], &[]),
    list("recovery", "recovery --limit N [--cursor ID]"),
    read("topics", &[], &[]),
    cmd(
        "topics-create",
        &["key"],
        &[],
        YES,
        Duplicate,
        Some(Converge),
        ReceiptAt::NoteAndPointer,
        APPLY_OUTCOMES,
    ),
    Spec {
        page: paged(1, 100, Some(20), CursorForm::Id),
        bounded: Some(false),
        bounded_form: Some("projects [--trash yes] --limit N [--cursor ID]"),
        ..read("projects", &["trash", "limit", "cursor"], &[])
    },
    project(
        "project-create",
        &["title", "key", "title-file"],
        &["title"],
        Duplicate,
    ),
    project(
        "project-chat",
        &["project", "title", "key", "title-file"],
        &["project", "title"],
        Duplicate,
    ),
    project(
        "project-document",
        &["project", "title", "text", "key", "title-file", "text-file"],
        &["project", "title", "text"],
        Duplicate,
    ),
    project(
        "project-settings",
        &[
            "project",
            "base",
            "title",
            "instructions",
            "knowledge",
            "key",
            "title-file",
            "instructions-file",
        ],
        &["project", "base", "title", "instructions", "knowledge"],
        Refuse,
    ),
    Spec {
        outcomes: CONVERGE_OUTCOMES,
        ..project(
            "chat-move",
            &["thread", "project", "key"],
            &["thread"],
            Converge,
        )
    },
    Spec {
        outcomes: CONVERGE_OUTCOMES,
        ..project(
            "project-link",
            &["project", "note", "key"],
            &["project", "note"],
            Converge,
        )
    },
    Spec {
        outcomes: CONVERGE_OUTCOMES,
        ..project(
            "project-unlink",
            &["project", "note", "key"],
            &["project", "note"],
            Converge,
        )
    },
    read("project-delete-preview", &["project"], &["project"]),
    project(
        "project-delete",
        &["preview-file", "actions", "key", "actions-file"],
        &["preview-file", "actions"],
        Refuse,
    ),
    project(
        "project-restore",
        &["project", "base", "key"],
        &["project", "base"],
        Refuse,
    ),
    setting("topics-choose", &["note"], &["note"]),
    read(
        "connections",
        &["note", "hops", "sources", "thread", "project"],
        &["note"],
    ),
    Spec {
        keyed: true,
        key_requires: Some(("seed-chat", "yes")),
        retry: Retry {
            default: Read,
            keyed: None,
            variants: &[Variant {
                option: "seed-chat",
                equals: "yes",
                retry: Duplicate,
                key: true,
                keyed: Some(Replay),
                receipt: None,
            }],
        },
        ..read("knowledge-check", &["seed-chat", "key"], &[])
    },
    cmd(
        "model-install",
        &["destination"],
        &["destination"],
        YES,
        Converge,
        None,
        NONE,
        &[],
    ),
    cmd(
        "note-create",
        &["title", "text", "key", "title-file", "text-file"],
        &["title", "text"],
        NO,
        Duplicate,
        Some(Replay),
        ReceiptAt::Arm,
        RECEIPT_OUTCOMES,
    ),
    cmd(
        "thread-start",
        &["title", "key", "title-file"],
        &["title"],
        NO,
        Duplicate,
        Some(Replay),
        ReceiptAt::Arm,
        RECEIPT_OUTCOMES,
    ),
    setting(
        "document-conversion",
        &["converter", "ai"],
        &["converter", "ai"],
    ),
    setting("delegation", &["choice"], &["choice"]),
    Spec {
        outcomes: READ_OUTCOMES,
        page: paged(
            1,
            brn_threads_app::MAX_NOTE_READ_BYTES as u64,
            None,
            CursorForm::VersionStart,
        ),
        bounded: Some(false),
        bounded_form: Some("read --id ID --limit BYTES [--cursor VERSION.START]"),
        needs: &[("cursor", "limit")],
        ..read("read", &["id", "limit", "cursor"], &["id"])
    },
    Spec {
        page: paged(1, 32, Some(32), CursorForm::Version),
        bounded: Some(true),
        ..read("revisions", &["note", "limit", "cursor"], &["note"])
    },
    Spec {
        page: paged(
            1,
            brn_threads_app::MAX_NOTE_READ_BYTES as u64,
            Some(65536),
            CursorForm::Byte,
        ),
        bounded: Some(true),
        ..read(
            "revision",
            &["note", "version", "limit", "cursor"],
            &["note", "version"],
        )
    },
    Spec {
        page: paged(1, 100, Some(20), CursorForm::Id),
        bounded: Some(true),
        ..read("runs", &["thread", "state", "limit", "cursor"], &[])
    },
    Spec {
        page: paged(1, 100, Some(20), CursorForm::Rowid),
        bounded: Some(true),
        ..read("messages", &["thread", "limit", "cursor"], &["thread"])
    },
    Spec {
        bounded: Some(true),
        ..read("operation", &["id", "key", "command"], &[])
    },
    Spec {
        page: paged(1, 50, Some(12), CursorForm::Opaque),
        bounded: Some(true),
        ..read(
            "search",
            &["query", "limit", "cursor", "thread", "project"],
            &["query"],
        )
    },
    cmd(
        "begin-edit",
        &["note", "base", "token-out"],
        &["note", "base"],
        NO,
        Refuse,
        None,
        NONE,
        &[],
    ),
    cmd(
        "update-buffer",
        &["session", "generation", "text", "text-file", "token-file"],
        &["session", "generation", "text", "token-file"],
        NO,
        Replay,
        None,
        NONE,
        BUFFER_OUTCOMES,
    ),
    cmd(
        "save",
        &["session", "generation", "token-file"],
        &["session", "generation", "token-file"],
        NO,
        Replay,
        None,
        ReceiptAt::Field,
        SAVE_OUTCOMES,
    ),
    cmd(
        "discard",
        &["session", "generation", "token-file"],
        &["session", "generation", "token-file"],
        NO,
        Refuse,
        None,
        NONE,
        DISCARD_OUTCOMES,
    ),
    cmd(
        "reclaim-edit",
        &["session", "token-out"],
        &["session"],
        NO,
        Converge,
        None,
        NONE,
        &[],
    ),
    cmd(
        "apply",
        &["operation"],
        &["operation"],
        YES,
        Replay,
        None,
        ReceiptAt::Field,
        APPLY_OUTCOMES,
    ),
    cmd(
        "dismiss",
        &["operation"],
        &["operation"],
        YES,
        Replay,
        None,
        NONE,
        DISMISS_OUTCOMES,
    ),
    cmd(
        "undo",
        &["operation", "key"],
        &["operation"],
        YES,
        Duplicate,
        Some(Replay),
        ReceiptAt::Arm,
        APPLY_OUTCOMES,
    ),
    ARCHIVE,
    Spec {
        name: "resolve",
        alias_of: Some("archive"),
        ..ARCHIVE
    },
    cmd(
        "unarchive",
        &["thread", "key"],
        &["thread"],
        YES,
        Converge,
        Some(Replay),
        ReceiptAt::Root,
        CONVERGE_OUTCOMES,
    ),
    cmd(
        "keep-in-brn",
        &["note", "base", "key"],
        &["note", "base"],
        YES,
        Duplicate,
        Some(Replay),
        ReceiptAt::Field,
        APPLY_OUTCOMES,
    ),
    cmd(
        "queue",
        &["thread", "text", "key", "text-file"],
        &["thread", "text"],
        NO,
        Duplicate,
        Some(Replay),
        ReceiptAt::Root,
        RECEIPT_OUTCOMES,
    ),
    cmd(
        "ask",
        &[
            "thread",
            "prompt",
            "budget",
            "edit-note",
            "base",
            "record-decision",
            "trace",
            "prompt-file",
            "mcp-tool",
            "mcp-input",
        ],
        &["thread", "prompt"],
        Owner::WithOptions(&["edit-note", "record-decision", "mcp-tool"]),
        Duplicate,
        None,
        ReceiptAt::Terminal,
        TERMINAL_OUTCOMES,
    ),
    Spec {
        required_when: &[
            RequiredWhen {
                option: "intent",
                equals: "chat",
                required: &["thread"],
            },
            RequiredWhen {
                option: "intent",
                equals: "project",
                required: &["project"],
            },
        ],
        retry: Retry {
            default: Duplicate,
            keyed: Some(Replay),
            variants: &[Variant {
                option: "intent",
                equals: "useful",
                retry: Duplicate,
                key: false,
                keyed: None,
                receipt: Some(ReceiptAt::Terminal),
            }],
        },
        ..cmd(
            "intake",
            &[
                "source",
                "intent",
                "title",
                "thread",
                "project",
                "trace",
                "key",
                "title-file",
            ],
            &["source", "intent"],
            Owner::WithOptions(&["project"]),
            Duplicate,
            Some(Replay),
            ReceiptAt::Field,
            INTAKE_OUTCOMES,
        )
    },
    setting(
        "configure",
        CONFIGURE_OPTIONS,
        &["provider", "model", "effort"],
    ),
    cmd(
        "export",
        &["destination"],
        &["destination"],
        NO,
        Refuse,
        None,
        NONE,
        &[],
    ),
    cmd(
        "backup",
        &["destination"],
        &["destination"],
        NO,
        Refuse,
        None,
        NONE,
        &[],
    ),
    cmd(
        "restore",
        &["source"],
        &["source"],
        NO,
        Refuse,
        None,
        NONE,
        &[],
    ),
    cmd(
        "upgrade",
        &["backup-directory"],
        &["backup-directory"],
        YES,
        Converge,
        None,
        NONE,
        &[],
    ),
];

pub(crate) fn spec(name: &str) -> Option<&'static Spec> {
    COMMANDS.iter().find(|spec| spec.name == name)
}

fn variant<'a>(spec: &'a Spec, args: &BTreeMap<String, String>) -> Option<&'a Variant> {
    spec.retry
        .variants
        .iter()
        .find(|v| args.get(v.option).is_some_and(|value| value == v.equals))
}

fn retry_of(spec: &Spec, args: &BTreeMap<String, String>) -> Class {
    if let Some(variant) = variant(spec, args) {
        return match variant.keyed {
            Some(keyed) if args.contains_key("key") => keyed,
            _ => variant.retry,
        };
    }
    match spec.retry.keyed {
        Some(keyed) if args.contains_key("key") => keyed,
        _ => spec.retry.default,
    }
}

/// The class an identical retry of this invocation has (first matching
/// variant, else `keyed` with `--key`, else `default`).
pub(crate) fn retry_for(inv: &Invocation) -> Class {
    spec(&inv.command).map_or(Class::Refuse, |spec| retry_of(spec, &inv.args))
}

pub(crate) fn receipt_for(inv: &Invocation) -> ReceiptAt {
    let Some(spec) = spec(&inv.command) else {
        return ReceiptAt::None;
    };
    variant(spec, &inv.args)
        .and_then(|v| v.receipt)
        .unwrap_or(spec.receipt)
}

/// The matching variant that refuses `--key`, on commands that take a key.
pub(crate) fn key_refused<'a>(
    spec: &'a Spec,
    args: &BTreeMap<String, String>,
) -> Option<&'a Variant> {
    if !spec.keyed || !args.contains_key("key") {
        return None;
    }
    variant(spec, args).filter(|v| !v.key)
}

/// The unprefixed host key a `--key` write passes to the shared service:
/// `cli-key:{command}:{KEY}`, with an alias resolved to its command.
pub(crate) fn cli_key(command: &str, key: &str) -> String {
    let command = spec(command)
        .and_then(|spec| spec.alias_of)
        .unwrap_or(command);
    format!("cli-key:{command}:{key}")
}

/// The host key the shared service stores for `COMMAND --key KEY`, for
/// lookup only (`operation --key`). Each prefix is added by the app function
/// named in the comment; project commands keep the caller's raw key.
pub(crate) fn stored_host_key(command: &str, key: &str) -> Option<String> {
    let canonical = spec(command)?.alias_of.unwrap_or(command);
    let prefix = match canonical {
        // brn-threads-app lib.rs `new_note_with_key`
        "note-create" => "new-note:",
        // lib.rs `new_thread_with_key`
        "thread-start" => "new-thread:",
        // lib.rs `undo_with_key`
        "undo" => "owner-undo:",
        // chat.rs `chat_command`
        "archive" | "unarchive" | "queue" => "chat-control:",
        // intake.rs `keep_in_brn`
        "keep-in-brn" => "keep-in-brn:",
        // intake.rs `import_full_scoped`
        "intake" => "full-import:",
        // keyed.rs `create_topics_note_with_key` (`new_note_with_key("topics:{key}")`)
        "topics-create" => "new-note:topics:",
        // keyed.rs `seed_knowledge_review_with_key`: the message operation
        "knowledge-check" => "knowledge-review-message:",
        // projects.rs `project_command`: one raw-key scope for every project command
        "project-create" | "project-chat" | "project-document" | "project-settings"
        | "chat-move" | "project-link" | "project-unlink" | "project-delete"
        | "project-restore" => return Some(format!("project:{key}")),
        _ => return None,
    };
    Some(format!("{prefix}{}", cli_key(canonical, key)))
}

pub(crate) fn key_is_valid(key: &str) -> bool {
    (1..=128).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

fn when(option: &str, equals: &str) -> Value {
    json!({"option":option,"equals":equals})
}

/// One command's entry in the machine description (also `brn --json COMMAND --help`).
pub(crate) fn describe(spec: &Spec) -> Value {
    let (owner, owner_options): (bool, &[&str]) = match spec.owner {
        Owner::No => (false, &[]),
        Owner::Yes => (true, &[]),
        Owner::WithOptions(options) => (false, options),
    };
    json!({
        "name": spec.name,
        "alias_of": spec.alias_of,
        "options": spec.options,
        "required": spec.required,
        "required_when": spec.required_when.iter().map(|r| json!({
            "when": when(r.option, r.equals), "required": r.required,
        })).collect::<Vec<_>>(),
        "key": spec.keyed,
        "owner_decision": owner,
        "owner_decision_options": owner_options,
        "retry": {
            "default": spec.retry.default.as_str(),
            "keyed": spec.retry.keyed.map(Class::as_str),
            "variants": spec.retry.variants.iter().map(|v| json!({
                "when": when(v.option, v.equals), "retry": v.retry.as_str(), "key": v.key,
                "keyed": v.keyed.map(Class::as_str),
            })).collect::<Vec<_>>(),
        },
        "outcomes": spec.outcomes,
        "page": spec.page.map(|p| json!({
            "limit": {"min": p.min, "max": p.max, "default": p.default},
            "cursor": p.cursor.as_str(),
        })),
        "bounded": spec.bounded,
        "bounded_form": spec.bounded_form,
        "values": values_of(spec.name),
        "key_requires": spec.key_requires.map(|(option, equals)| when(option, equals)),
        "summary": crate::help::summary(spec.name),
        "example": crate::help::example(spec.name),
        "note": crate::help::note(spec.name),
        "owner_decision_when": OWNER_DECISION_WHEN.iter().filter(|(name, ..)| *name == spec.name).map(|(_, option, names, note)| json!({"option": option, "names": names, "note": note})).collect::<Vec<_>>(),
        "needs": spec.needs.iter().map(|(option, needs)| json!({"option": option, "needs": needs})).collect::<Vec<_>>(),
    })
}

/// The machine description printed by `brn --json --help`.
pub(crate) fn description() -> Value {
    let commands: Vec<Value> = COMMANDS.iter().map(describe).collect();
    json!({
        "contract": crate::contract::CONTRACT,
        "usage": "brn [--json] --data-dir ABSOLUTE_DIRECTORY [--timeout SECONDS] [--actor NAME] COMMAND [--option VALUE]...",
        "statuses": Status::ALL.iter().map(|s| json!({
            "status": s.as_str(), "exit": s.exit(), "meaning": s.meaning(), "action": s.action(),
        })).collect::<Vec<_>>(),
        "errors": Code::ALL.iter().map(|c| json!({
            "code": c.as_str(), "status": c.status().as_str(), "meaning": c.meaning(), "reserved": c.reserved(),
        })).collect::<Vec<_>>(),
        "retry_classes": Class::ALL.iter().map(|c| json!({"retry": c.as_str(), "meaning": c.meaning()})).collect::<Vec<_>>(),
        "owner_decision_rule": OWNER_DECISION_RULE,
        "input": {
            "text_options": TEXT_OPTIONS,
            "file_suffix": "-file",
            "stdin": "-",
            "max_bytes": brn_intake::MAX_INPUT_BYTES,
            "encoding": "UTF-8 without NUL; bytes used exactly",
        },
        "key": {"pattern": KEY_PATTERN},
        "global_options": [
            {"option": "data-dir", "value": "ABSOLUTE_DIRECTORY", "required": true, "position": "anywhere in an option-name position"},
            {"option": "timeout", "value": "SECONDS", "grammar": {"grammar": "integer", "min": 1, "max": 86400}, "required": false, "position": "anywhere in an option-name position"},
            {"option": "actor", "value": "NAME", "env": ACTOR_ENV, "grammar": {"grammar": "pattern", "pattern": ACTOR_PATTERN}, "required": false, "position": "anywhere in an option-name position"},
        ],
        "authority": {
            "purposes": ["owner", "test"],
            "default": "owner",
            "created_by": "init --purpose test",
            "changed_by": "desktop only",
            "restore_yields": "owner",
            "cli_authority": {"owner": "full", "test": "full"},
            "phase2_owner": "proposals",
            "enforced": false,
            "actor": {"option": "actor", "env": ACTOR_ENV, "pattern": ACTOR_PATTERN, "run_receipts": "none"},
            "holders": {"caller": "token required", "desktop": "owner in the desktop app only"},
            "limit": "agent overreach, not same-OS-user code",
        },
        "edit_token": {
            "pattern": TOKEN_PATTERN,
            "returned_by": ["begin-edit", "reclaim-edit"],
            "returned_in": "result.token, or the --token-out file (owner-only, created new)",
            "presented_with": "--token-file ABSOLUTE_FILE or --token-file - (stdin); never argv or environment",
        },
        "command_help": "brn [--json] COMMAND --help prints one command's help (JSON: {contract, command}); it needs no --data-dir and opens nothing.",
        "commands": commands,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn command(name: &str) -> Value {
        description()["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap()
            .clone()
    }

    #[test]
    fn retry_for_agrees_with_the_description_for_every_command_and_variant() {
        for spec in COMMANDS {
            let described = command(spec.name);
            let retry = &described["retry"];
            assert_eq!(
                retry_of(spec, &args(&[])).as_str(),
                retry["default"],
                "{}",
                spec.name
            );
            let keyed = retry_of(spec, &args(&[("key", "k")]));
            match retry["keyed"].as_str() {
                Some(class) => assert_eq!(keyed.as_str(), class, "{}", spec.name),
                None => assert_eq!(keyed.as_str(), retry["default"], "{}", spec.name),
            }
            for variant in retry["variants"].as_array().unwrap() {
                let option = variant["when"]["option"].as_str().unwrap();
                let equals = variant["when"]["equals"].as_str().unwrap();
                for with_key in [false, true] {
                    let mut given = args(&[(option, equals)]);
                    if with_key {
                        given.insert("key".into(), "k".into());
                    }
                    let expected = match variant["keyed"].as_str() {
                        Some(keyed) if with_key => keyed,
                        _ => variant["retry"].as_str().unwrap(),
                    };
                    assert_eq!(retry_of(spec, &given).as_str(), expected, "{}", spec.name);
                }
                // A different value matches no variant.
                let other = retry_of(spec, &args(&[(option, "other-value")]));
                assert_eq!(other.as_str(), retry["default"], "{}", spec.name);
            }
        }
    }

    #[test]
    fn key_refusal_applies_only_to_commands_that_take_a_key() {
        for spec in COMMANDS.iter().filter(|s| !s.keyed) {
            for variant in spec.retry.variants {
                let given = args(&[(variant.option, variant.equals), ("key", "k")]);
                assert!(key_refused(spec, &given).is_none(), "{}", spec.name);
            }
            // `operation --key` looks a key up; it does not take one.
            if spec.name != "operation" {
                assert!(!spec.options.contains(&"key"), "{}", spec.name);
            }
        }
        let intake = spec("intake").unwrap();
        assert!(key_refused(intake, &args(&[("intent", "useful"), ("key", "k")])).is_some());
        assert!(key_refused(intake, &args(&[("intent", "full"), ("key", "k")])).is_none());
        assert!(key_refused(intake, &args(&[("intent", "useful")])).is_none());
        for spec in COMMANDS.iter().filter(|s| s.keyed) {
            assert!(spec.options.contains(&"key"), "{}", spec.name);
        }
    }

    #[test]
    fn required_lists_follow_the_first_use_order() {
        let expected: &[(&str, &[&str])] = &[
            ("project-create", &["title"]),
            ("project-chat", &["project", "title"]),
            ("project-document", &["project", "title", "text"]),
            (
                "project-settings",
                &["project", "base", "title", "instructions", "knowledge"],
            ),
            ("chat-move", &["thread"]),
            ("project-link", &["project", "note"]),
            ("project-unlink", &["project", "note"]),
            ("project-delete-preview", &["project"]),
            ("project-delete", &["preview-file", "actions"]),
            ("project-restore", &["project", "base"]),
            ("topics-choose", &["note"]),
            ("connections", &["note"]),
            ("model-install", &["destination"]),
            ("export", &["destination"]),
            ("backup", &["destination"]),
            ("note-create", &["title", "text"]),
            ("thread-start", &["title"]),
            ("document-conversion", &["converter", "ai"]),
            ("delegation", &["choice"]),
            ("read", &["id"]),
            ("skill-inspect", &["name"]),
            ("skill-install", &["source"]),
            ("skill-remove", &["name", "digest"]),
            ("skill-enable", &["id", "digest"]),
            ("skill-disable", &["id", "digest"]),
            ("skill-select", &["id"]),
            ("skill-clear", &["id"]),
            ("mcp-discover", &["endpoint"]),
            ("mcp-configure", &["policy"]),
            ("revisions", &["note"]),
            ("revision", &["note", "version"]),
            ("search", &["query"]),
            ("messages", &["thread"]),
            ("begin-edit", &["note", "base"]),
            ("keep-in-brn", &["note", "base"]),
            (
                "update-buffer",
                &["session", "generation", "text", "token-file"],
            ),
            ("save", &["session", "generation", "token-file"]),
            ("discard", &["session", "generation", "token-file"]),
            ("reclaim-edit", &["session"]),
            ("apply", &["operation"]),
            ("undo", &["operation"]),
            ("dismiss", &["operation"]),
            ("archive", &["thread"]),
            ("resolve", &["thread"]),
            ("unarchive", &["thread"]),
            ("queue", &["thread", "text"]),
            ("ask", &["thread", "prompt"]),
            ("intake", &["source", "intent"]),
            ("configure", &["provider", "model", "effort"]),
            ("restore", &["source"]),
            ("upgrade", &["backup-directory"]),
        ];
        for (name, required) in expected {
            assert_eq!(spec(name).unwrap().required, *required, "{name}");
        }
        for spec in COMMANDS {
            if !expected.iter().any(|(name, _)| *name == spec.name) {
                assert!(spec.required.is_empty(), "{}", spec.name);
            }
            for option in spec.required {
                assert!(spec.options.contains(option), "{}", spec.name);
            }
        }
    }

    #[test]
    fn owner_marks_match_the_owner_decision_table() {
        let yes = [
            "mcp-discover",
            "mcp-configure",
            "mcp-disable",
            "skill-install",
            "skill-remove",
            "skill-enable",
            "skill-disable",
            "skill-select",
            "skill-clear",
            "apply",
            "dismiss",
            "undo",
            "configure",
            "document-conversion",
            "delegation",
            "pause",
            "resume",
            "project-create",
            "project-chat",
            "project-document",
            "project-settings",
            "chat-move",
            "project-link",
            "project-unlink",
            "project-delete",
            "project-restore",
            "keep-in-brn",
            "archive",
            "resolve",
            "unarchive",
            "topics-create",
            "topics-choose",
            "upgrade",
            "model-install",
        ];
        for described in description()["commands"].as_array().unwrap() {
            let name = described["name"].as_str().unwrap();
            assert_eq!(described["owner_decision"], yes.contains(&name), "{name}");
            let options = match name {
                "ask" => json!(["edit-note", "record-decision", "mcp-tool"]),
                "intake" => json!(["project"]),
                _ => json!([]),
            };
            assert_eq!(described["owner_decision_options"], options, "{name}");
        }
        assert_eq!(command("queue")["owner_decision"], false);
        assert_eq!(command("intake")["owner_decision"], false);
        let mut archive = command("archive");
        let mut resolve = command("resolve");
        assert_eq!(resolve["alias_of"], "archive");
        for value in [&mut archive, &mut resolve] {
            let object = value.as_object_mut().unwrap();
            object.remove("name");
            object.remove("alias_of");
            // Help text differs per name; the contract fields are identical.
            object.remove("summary");
            object.remove("example");
        }
        assert_eq!(archive, resolve);
    }

    #[test]
    fn only_intake_has_conditional_requirements() {
        for spec in COMMANDS.iter().filter(|s| s.name != "intake") {
            assert!(spec.required_when.is_empty(), "{}", spec.name);
        }
        assert_eq!(
            command("intake")["required_when"],
            json!([
                {"when":{"option":"intent","equals":"chat"},"required":["thread"]},
                {"when":{"option":"intent","equals":"project"},"required":["project"]},
            ])
        );
    }

    #[test]
    fn outcomes_list_every_label_of_the_returned_types() {
        assert_eq!(
            INTAKE_OUTCOMES,
            [APPLY_OUTCOMES, TERMINAL_OUTCOMES].concat()
        );
        assert!(INTAKE_OUTCOMES.contains(&"Deferred"));
        for spec in COMMANDS {
            let expected: &[&str] = match spec.name {
                "apply"
                | "keep-in-brn"
                | "document-conversion"
                | "delegation"
                | "configure"
                | "pause"
                | "resume"
                | "topics-choose"
                | "undo"
                | "topics-create"
                | "skill-enable"
                | "skill-disable"
                | "skill-select"
                | "skill-clear"
                | "mcp-configure" => APPLY_OUTCOMES,
                "intake" => INTAKE_OUTCOMES,
                "save" => SAVE_OUTCOMES,
                "update-buffer" => BUFFER_OUTCOMES,
                "discard" => DISCARD_OUTCOMES,
                "dismiss" => DISMISS_OUTCOMES,
                "ask" => TERMINAL_OUTCOMES,
                "read" => READ_OUTCOMES,
                "archive" | "resolve" | "unarchive" | "chat-move" | "project-link"
                | "project-unlink" | "mcp-disable" => CONVERGE_OUTCOMES,
                _ if spec.receipt == ReceiptAt::Root || spec.receipt == ReceiptAt::Arm => {
                    RECEIPT_OUTCOMES
                }
                _ => &[],
            };
            assert_eq!(spec.outcomes, expected, "{}", spec.name);
        }
    }

    #[test]
    fn text_options_have_file_variants_and_class_meanings_are_pinned() {
        for spec in COMMANDS {
            for name in TEXT_OPTIONS {
                let file = format!("{name}-file");
                assert_eq!(
                    spec.options.contains(&name),
                    spec.options.contains(&file.as_str()),
                    "{} {name}",
                    spec.name
                );
            }
        }
        let refuse = Class::Refuse.meaning();
        assert!(
            refuse.contains("If the first attempt took effect, an identical retry changes nothing")
        );
        assert!(refuse.contains("the retry performs the change once and can return ok"));
        assert!(!description().to_string().contains('`'));
        assert_eq!(command("model-install")["retry"]["default"], "converge");
    }

    #[test]
    fn description_is_one_line_with_the_owner_rule() {
        let text = description().to_string();
        assert!(!text.contains('\n'));
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["owner_decision_rule"], OWNER_DECISION_RULE);
        assert!(value.get("owner_decision").is_none());
        assert_eq!(value["key"]["pattern"], KEY_PATTERN);
        assert_eq!(value["statuses"].as_array().unwrap().len(), 10);
    }

    #[test]
    fn cli_keys_are_byte_identical_to_round_one() {
        for (command, stored) in [
            ("note-create", "new-note:cli-key:note-create:K"),
            ("thread-start", "new-thread:cli-key:thread-start:K"),
            ("undo", "owner-undo:cli-key:undo:K"),
            ("archive", "chat-control:cli-key:archive:K"),
            ("resolve", "chat-control:cli-key:archive:K"),
            ("unarchive", "chat-control:cli-key:unarchive:K"),
            ("queue", "chat-control:cli-key:queue:K"),
            ("keep-in-brn", "keep-in-brn:cli-key:keep-in-brn:K"),
            ("intake", "full-import:cli-key:intake:K"),
            ("topics-create", "new-note:topics:cli-key:topics-create:K"),
            (
                "knowledge-check",
                "knowledge-review-message:cli-key:knowledge-check:K",
            ),
            ("project-create", "project:K"),
            ("project-restore", "project:K"),
        ] {
            assert_eq!(
                stored_host_key(command, "K").as_deref(),
                Some(stored),
                "{command}"
            );
        }
        assert_eq!(cli_key("resolve", "K"), "cli-key:archive:K");
        assert_eq!(cli_key("note-create", "K"), "cli-key:note-create:K");
        for spec in COMMANDS {
            assert_eq!(
                stored_host_key(spec.name, "K").is_some(),
                spec.keyed,
                "{}",
                spec.name
            );
        }
    }

    #[test]
    fn global_options_are_not_command_options_and_every_command_has_help() {
        for spec in COMMANDS {
            for global in ["data-dir", "timeout", "actor", "help"] {
                assert!(
                    !spec.options.contains(&global),
                    "{} declares {global}",
                    spec.name
                );
            }
            assert!(!crate::help::summary(spec.name).is_empty(), "{}", spec.name);
            assert!(
                crate::help::example(spec.name).contains(spec.name),
                "{}",
                spec.name
            );
        }
        assert_eq!(crate::help::TEXT.len(), COMMANDS.len());
    }

    #[test]
    fn key_validation_matches_the_published_pattern() {
        for good in ["a", "plan-2026-10-15", "A.b_c:d-e", &"k".repeat(128)] {
            assert!(key_is_valid(good), "{good}");
        }
        for bad in ["", "trip 2026", "naïve", "a/b", "a\nb", &"k".repeat(129)] {
            assert!(!key_is_valid(bad), "{bad}");
        }
    }
}
