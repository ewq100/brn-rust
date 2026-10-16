//! The command table: one entry per command, read by `parse()`, the envelope
//! and `brn --json --help`, so the grammar and the description cannot drift.
use crate::contract::{Code, Status};
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
            Class::Converge => "Reaches the same end state. It may record a new operation and bump versions, but creates no new records and leaves no unapplied candidate. init and upgrade on a current workspace record nothing. model-install into a destination that already holds the verified model downloads nothing and returns downloaded_bytes: 0.",
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
}

/// Text options that also accept `--NAME-file ABSOLUTE_PATH|-`, read in this order.
pub(crate) const TEXT_OPTIONS: [&str; 5] = ["title", "text", "prompt", "instructions", "actions"];
pub(crate) const KEY_PATTERN: &str = "^[A-Za-z0-9._:-]{1,128}$";
pub(crate) const OWNER_DECISION_RULE: &str = "These commands record an owner decision. Running the CLI does not make a caller the owner. Run them only on the owner's explicit instruction for that exact operation.";

pub(crate) const APPLY_OUTCOMES: &[&str] =
    &["Applied", "Stale", "Deferred", "NeedsReview", "Superseded"];
pub(crate) const SAVE_OUTCOMES: &[&str] =
    &["Saved", "Stale", "GenerationMismatch", "Closed", "Deferred"];
pub(crate) const BUFFER_OUTCOMES: &[&str] = &["Updated", "Ignored"];
pub(crate) const DISCARD_OUTCOMES: &[&str] = &["Discarded", "Ignored"];
pub(crate) const DISMISS_OUTCOMES: &[&str] = &["Dismissed"];
pub(crate) const RECEIPT_OUTCOMES: &[&str] = &["Applied"];
pub(crate) const TERMINAL_OUTCOMES: &[&str] = &["Completed", "Interrupted", "Failed"];
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
    RECEIPT_OUTCOMES,
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
    cmd("init", &[], &[], NO, Converge, None, NONE, &[]),
    read("records", &[], &[]),
    read("notes", &[], &[]),
    read("threads", &[], &[]),
    read("needs-you", &[], &[]),
    read("history", &[], &[]),
    read("candidates", &[], &[]),
    read("settings", &[], &[]),
    setting("pause", &[], &[]),
    setting("resume", &[], &[]),
    read("recovery", &[], &[]),
    read("topics", &[], &[]),
    cmd(
        "topics-create",
        &[],
        &[],
        YES,
        Duplicate,
        None,
        ReceiptAt::NoteAndPointer,
        APPLY_OUTCOMES,
    ),
    read("projects", &["trash"], &[]),
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
    project(
        "chat-move",
        &["thread", "project", "key"],
        &["thread"],
        Converge,
    ),
    project(
        "project-link",
        &["project", "note", "key"],
        &["project", "note"],
        Converge,
    ),
    project(
        "project-unlink",
        &["project", "note", "key"],
        &["project", "note"],
        Converge,
    ),
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
        retry: Retry {
            default: Read,
            keyed: None,
            variants: &[Variant {
                option: "seed-chat",
                equals: "yes",
                retry: Duplicate,
                key: false,
                receipt: None,
            }],
        },
        ..read("knowledge-check", &["seed-chat"], &[])
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
    setting("delegation", &["choice"], &["choice"]),
    read("read", &["id"], &["id"]),
    read(
        "search",
        &["query", "limit", "cursor", "thread", "project"],
        &["query"],
    ),
    cmd(
        "begin-edit",
        &["note", "base"],
        &["note", "base"],
        NO,
        Refuse,
        None,
        NONE,
        &[],
    ),
    cmd(
        "update-buffer",
        &["session", "generation", "text", "text-file"],
        &["session", "generation", "text"],
        NO,
        Replay,
        None,
        NONE,
        BUFFER_OUTCOMES,
    ),
    cmd(
        "save",
        &["session", "generation"],
        &["session", "generation"],
        NO,
        Replay,
        None,
        ReceiptAt::Field,
        SAVE_OUTCOMES,
    ),
    cmd(
        "discard",
        &["session", "generation"],
        &["session", "generation"],
        NO,
        Refuse,
        None,
        NONE,
        DISCARD_OUTCOMES,
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
        RECEIPT_OUTCOMES,
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
        ],
        &["thread", "prompt"],
        Owner::WithOptions(&["edit-note", "record-decision"]),
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
        return variant.retry;
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

pub(crate) fn key_is_valid(key: &str) -> bool {
    (1..=128).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

fn when(option: &str, equals: &str) -> Value {
    json!({"option":option,"equals":equals})
}

/// The machine description printed by `brn --json --help`.
pub(crate) fn description() -> Value {
    let commands: Vec<Value> = COMMANDS
        .iter()
        .map(|spec| {
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
                    })).collect::<Vec<_>>(),
                },
                "outcomes": spec.outcomes,
            })
        })
        .collect();
    json!({
        "contract": crate::contract::CONTRACT,
        "usage": "brn [--json] --data-dir ABSOLUTE_DIRECTORY COMMAND [--option VALUE]...",
        "statuses": Status::ALL.iter().map(|s| json!({
            "status": s.as_str(), "exit": s.exit(), "meaning": s.meaning(), "action": s.action(),
        })).collect::<Vec<_>>(),
        "errors": Code::ALL.iter().map(|c| json!({
            "code": c.as_str(), "status": c.status().as_str(), "meaning": c.meaning(),
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
                    assert_eq!(
                        retry_of(spec, &given).as_str(),
                        variant["retry"],
                        "{}",
                        spec.name
                    );
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
            assert!(!spec.options.contains(&"key"), "{}", spec.name);
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
            ("delegation", &["choice"]),
            ("read", &["id"]),
            ("search", &["query"]),
            ("begin-edit", &["note", "base"]),
            ("keep-in-brn", &["note", "base"]),
            ("update-buffer", &["session", "generation", "text"]),
            ("save", &["session", "generation"]),
            ("discard", &["session", "generation"]),
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
            "apply",
            "dismiss",
            "undo",
            "configure",
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
                "ask" => json!(["edit-note", "record-decision"]),
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
                "apply" | "keep-in-brn" | "delegation" | "configure" | "pause" | "resume"
                | "topics-choose" | "undo" | "topics-create" => APPLY_OUTCOMES,
                "intake" => INTAKE_OUTCOMES,
                "save" => SAVE_OUTCOMES,
                "update-buffer" => BUFFER_OUTCOMES,
                "discard" => DISCARD_OUTCOMES,
                "dismiss" => DISMISS_OUTCOMES,
                "ask" => TERMINAL_OUTCOMES,
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
        assert_eq!(value["statuses"].as_array().unwrap().len(), 9);
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
