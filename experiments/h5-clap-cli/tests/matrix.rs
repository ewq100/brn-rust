//! Contract matrix: the real BRN binary (`BRN_BIN`) is the oracle; the clap
//! candidate must match its help/version/usage outcome, envelope command and
//! message. Every case stops before storage; an existing empty data directory
//! must stay empty. Synthetic argv only; no workspace, provider or network.
use h5_clap_cli::{parse, Outcome};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const U: &str = "123e4567-e89b-12d3-a456-426614174000";
const NIL: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Seen {
    Help,
    Version,
    Usage {
        command: Option<String>,
        message: String,
    },
    Other(String),
}

/// (id, argv, reason when the candidate is expected to diverge)
fn cases() -> Vec<(&'static str, Vec<&'static str>, Option<&'static str>)> {
    let close = |extra: &[&'static str]| {
        let mut v = vec![
            "findings",
            "close",
            U,
            "--version",
            "3",
            "--state",
            "resolved",
        ];
        v.extend_from_slice(extra);
        v
    };
    vec![
        // Exact --help precedence, even with invalid or duplicate input.
        ("help.bare", vec!["--help"], None),
        ("help.word", vec!["help"], None),
        ("help.word-rest", vec!["help", "bogus", "--x"], None),
        ("help.leaf", vec!["status", "--help"], None),
        ("help.nested", vec!["inbox", "add", "--help"], None),
        ("help.group", vec!["inbox", "--help"], None),
        ("help.unknown-command", vec!["bogus", "--help"], None),
        ("help.after-unknown", vec!["status", "--bogus", "--help"], None),
        (
            "help.after-duplicate",
            vec!["status", "--data-dir", "{DATA}", "--data-dir", "{DATA}", "--help"],
            None,
        ),
        ("help.duplicate-json", vec!["--json", "--json", "--help"], None),
        ("help.invalid-uuid", vec!["inbox", "add", "--id", "bad", "--help"], None),
        ("help.as-missing-value", vec!["status", "--data-dir", "--help"], None),
        ("help.with-json", vec!["search", "--help", "--json"], None),
        ("help.value-rejected", vec!["--help=x"], None),
        ("help.value-rejected-leaf", vec!["status", "--help=", "--json"], None),
        // Version: global, post-parse, local findings close --version.
        ("version.bare", vec!["--version"], None),
        ("version.repeat", vec!["--version", "--version"], None),
        ("version.leaf", vec!["status", "--version"], None),
        ("version.after-parse-error", vec!["--version", "status", "--bogus"], None),
        ("version.value-rejected", vec!["--version=x"], None),
        ("version.before-local", {
            let mut v = vec!["--version"];
            v.extend(close(&[]));
            v
        }, None),
        ("version.local-review-stamp", close(&["--data-dir", "{MISSING}"]), None),
        ("version.local-duplicate", close(&["--version", "4"]), None),
        ("version.local-missing-state", vec!["findings", "close", U, "--version", "3"], None),
        // Late --json still selects the envelope for usage errors.
        ("json.late-unknown-command", vec!["bogus", "--json"], None),
        ("json.late-unknown-option", vec!["status", "--bogus", "--json"], None),
        ("json.as-missing-value", vec!["status", "--data-dir", "--json"], None),
        ("json.duplicate-root", vec!["--json", "--json", "status"], None),
        ("json.duplicate-across", vec!["--json", "status", "--json"], None),
        ("json.value-rejected", vec!["status", "--json=1"], None),
        ("json.nested-invalid", vec!["inbox", "add", "--id", "bad", "--json"], None),
        // Duplicate, unknown, global and value scanning.
        (
            "dup.leaf-global",
            vec!["status", "--data-dir", "{DATA}", "--data-dir", "{DATA}"],
            None,
        ),
        (
            "dup.across-levels",
            vec!["--data-dir", "{DATA}", "status", "--data-dir", "{DATA}"],
            None,
        ),
        (
            "dup.inline-across-levels",
            vec!["--data-dir={DATA}", "status", "--data-dir={DATA}"],
            None,
        ),
        (
            "dup.then-unknown",
            vec!["--data-dir", "{DATA}", "status", "--data-dir", "{DATA}", "--bogus"],
            Some("cross-level duplicates are only detectable after clap accepts the whole argv, so a later clap error wins"),
        ),
        ("unknown.before-command", vec!["--bogus", "status"], None),
        ("unknown.leaf", vec!["status", "--bogus", "--data-dir", "{DATA}"], None),
        ("unknown.leaf-inline", vec!["status", "--bogus=x"], None),
        ("unknown.after-positional", vec!["search", "q", "--bogus"], None),
        ("unknown.single-dash-word", vec!["-x", "status"], None),
        ("escape.leaf", vec!["status", "--"], None),
        ("escape.before-command", vec!["--", "status"], None),
        (
            "escape.after-earlier-error",
            vec!["status", "--bogus", "--"],
            Some("clap drops a bare `--`, so the adapter must reject it before clap sees earlier tokens"),
        ),
        ("value.missing-end", vec!["search", "q", "--limit"], None),
        ("value.double-dash", vec!["search", "q", "--limit", "--json"], None),
        ("value.single-dash", vec!["search", "q", "--limit", "-3"], None),
        (
            "value.inline-double-dash",
            vec!["inbox", "add", "--id", U, "--title=--draft", "--file", "f", "--data-dir", "{MISSING}"],
            None,
        ),
        (
            "value.single-dash-title",
            vec!["inbox", "add", "--id", U, "--title", "- agenda", "--file", "f", "--data-dir", "{MISSING}"],
            None,
        ),
        ("value.empty-inline-data-dir", vec!["status", "--data-dir="], None),
        ("positional.single-dash", vec!["search", "-foo", "--data-dir", "{MISSING}"], None),
        ("positional.unexpected", vec!["status", "extra"], None),
        ("positional.unexpected-dash", vec!["status", "-x"], None),
        (
            "positional.many",
            vec!["status", "a", "b", "c", "d"],
            Some("clap's bounded positional slots report the first overflow slot, BRN the first extra argument"),
        ),
        (
            "value.duplicate-missing",
            vec!["status", "--data-dir", "{DATA}", "--data-dir"],
            Some("with hyphen values clap takes the appended `--json` as the second value and reports the duplicate first"),
        ),
        ("positional.then-unknown", vec!["status", "x", "--bogus"], None),
        ("positional.after-options", vec!["search", "--limit", "3", "q", "--data-dir", "{MISSING}"], None),
        ("positional.dash-only", vec!["search", "-", "--data-dir", "{MISSING}"], None),
        ("positional.unicode", vec!["search", "é —", "--data-dir", "{MISSING}"], None),
        ("json.after-leaf-word", vec!["findings", "close", "--json"], None),
        ("version.local-inline", vec!["findings", "close", U, "--version=3", "--state", "resolved", "--data-dir", "{MISSING}"], None),
        ("version.local-generation-bound", vec!["findings", "close", U, "--version", "9223372036854775808", "--state", "resolved"], None),
        ("command.none", vec![], None),
        ("command.unknown", vec!["bogus"], None),
        ("group.missing", vec!["inbox"], None),
        ("group.unknown", vec!["inbox", "nope"], None),
        ("group.option-before-sub", vec!["inbox", "--json", "add"], None),
        ("group.findings-unknown", vec!["findings", "nope"], None),
        ("group.notes-unknown", vec!["notes", "nope"], None),
        // Admission after a complete parse, still before storage.
        ("admit.missing-data-dir", vec!["status"], None),
        ("admit.relative-data-dir", vec!["status", "--data-dir", "relative/dir"], None),
        ("admit.missing-directory", vec!["status", "--data-dir", "{MISSING}"], None),
        ("admit.relative-vault", vec!["status", "--vault", "rel", "--data-dir", "{DATA}"], None),
        // Representative nested command and typed-validation order.
        (
            "inbox.add-valid",
            vec!["inbox", "add", "--id", U, "--title", "T", "--file", "f", "--data-dir", "{MISSING}"],
            None,
        ),
        ("inbox.add-missing-id", vec!["inbox", "add", "--title", "T"], None),
        ("inbox.add-kind-before-id", vec!["inbox", "add", "--id", "bad", "--kind", "nope"], None),
        ("inbox.add-id-before-title", vec!["inbox", "add", "--id", "bad"], None),
        (
            "inbox.add-duplicate",
            vec!["inbox", "add", "--id", U, "--file", "f", "--file", "g"],
            None,
        ),
        ("inbox.add-nil", vec!["inbox", "add", "--id", NIL, "--title", "T", "--file", "f"], None),
        ("inbox.add-blank-title", vec!["inbox", "add", "--id", U, "--title", " ", "--file", "f"], None),
        ("inbox.add-positional", vec!["inbox", "add", "pos"], None),
        ("inbox.show-bad", vec!["inbox", "show", "bad"], None),
        ("inbox.show-two", vec!["inbox", "show", U, U], None),
        ("inbox.show-nil", vec!["inbox", "show", NIL], None),
        ("inbox.list-limit", vec!["inbox", "list", "--limit", "x"], None),
        ("inbox.list-valid", vec!["inbox", "list", "--data-dir", "{MISSING}"], None),
        ("search.limit-range", vec!["search", "q", "--limit", "51"], None),
        ("search.scope", vec!["search", "q", "--scope", "nope"], None),
        ("notes.show-valid", vec!["notes", "show", "a.md", "--data-dir", "{MISSING}"], None),
    ]
}

fn oracle(bin: &Path, args: &[String], data: &Path) -> Seen {
    let out = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("BRN binary runs");
    assert_eq!(
        fs::read_dir(data).unwrap().count(),
        0,
        "BRN touched the data directory for {args:?}"
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    match out.status.code() {
        Some(0) if stdout.starts_with("brn: agent-facing CLI") => Seen::Help,
        Some(0) if stdout.starts_with("brn ") => Seen::Version,
        Some(2) if args.iter().any(|a| a == "--json") => {
            let v: Value = serde_json::from_str(&stdout).expect("one usage envelope");
            assert_eq!(v["error"]["code"], "USAGE");
            assert!(stderr.is_empty());
            Seen::Usage {
                command: v["command"].as_str().map(str::to_owned),
                message: v["error"]["message"].as_str().unwrap().to_owned(),
            }
        }
        Some(2) => Seen::Usage {
            command: None,
            message: stderr
                .strip_prefix("error: ")
                .and_then(|s| s.strip_suffix('\n'))
                .unwrap_or(&stderr)
                .to_owned(),
        },
        code => Seen::Other(format!("exit {code:?}: {stdout}{stderr}")),
    }
}

fn candidate(args: &[String]) -> Seen {
    match parse(args) {
        Ok(Outcome::Help) => Seen::Help,
        Ok(Outcome::Version) => Seen::Version,
        Ok(Outcome::Run { command, .. }) => Seen::Other(format!("run {command}")),
        Err(f) => Seen::Usage {
            command: f.command.filter(|_| f.json).map(str::to_owned),
            message: f.message,
        },
    }
}

#[test]
fn candidate_matches_brn_contract_matrix() {
    let bin = PathBuf::from(
        std::env::var("BRN_BIN").expect("set BRN_BIN to an absolute built brn binary"),
    );
    assert!(bin.is_absolute() && bin.is_file(), "BRN_BIN: {bin:?}");
    let parent = std::env::temp_dir().canonicalize().unwrap();
    let data = parent.join(format!("h5-clap-cli-{}", std::process::id()));
    fs::create_dir(&data).expect("fresh owned data directory");
    let missing = parent.join("h5-clap-cli-missing-directory");
    assert!(!missing.exists());
    let (data_s, missing_s) = (data.to_str().unwrap(), missing.to_str().unwrap());

    let mut rows = Vec::new();
    let mut unexpected = Vec::new();
    for (id, argv, expected_divergence) in cases() {
        let args: Vec<String> = argv
            .iter()
            .map(|a| a.replace("{DATA}", data_s).replace("{MISSING}", missing_s))
            .collect();
        let mut json = args.clone();
        if !json.iter().any(|a| a == "--json") {
            json.push("--json".into());
        }
        let brn = (oracle(&bin, &args, &data), oracle(&bin, &json, &data));
        let clap = (candidate(&args), candidate(&json));
        for seen in [&brn.0, &brn.1] {
            assert!(
                !matches!(seen, Seen::Other(_)),
                "{id}: oracle left the parse boundary: {seen:?}"
            );
        }
        let diverged = brn != clap;
        if diverged != expected_divergence.is_some() {
            unexpected.push(format!("{id}: brn={brn:?} clap={clap:?}"));
        }
        rows.push(format!(
            "| `{id}` | {} | {} |",
            describe(&brn.1),
            if diverged {
                format!("DIVERGES: {}", describe(&clap.1))
            } else {
                "same".into()
            }
        ));
    }
    fs::remove_dir(&data).unwrap();
    println!("| Case | BRN (--json) | clap candidate |\n| --- | --- | --- |");
    for row in &rows {
        println!(
            "{}",
            row.replace(data_s, "$DATA").replace(missing_s, "$MISSING")
        );
    }
    println!("cases: {}", rows.len());
    assert!(
        unexpected.is_empty(),
        "unexpected results:\n{}",
        unexpected.join("\n")
    );
}

fn describe(seen: &Seen) -> String {
    match seen {
        Seen::Usage { command, message } => format!(
            "usage `{}`: {}",
            command.as_deref().unwrap_or("null"),
            message.replace('|', "\\|")
        ),
        other => format!("{other:?}"),
    }
}
