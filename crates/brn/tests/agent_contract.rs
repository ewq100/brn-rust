//! Process-level checks of the `--json` machine contract, `--NAME-file`
//! input and `--key` retries, through the built `brn` only.
#[cfg(unix)]
mod support;
use serde_json::{json, Value};
use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

/// A private test root; children get HOME, CODEX_HOME and temp dirs inside it.
struct Root {
    _dir: tempfile::TempDir,
    path: PathBuf,
}
impl Root {
    fn new() -> Self {
        let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let path = dir.path().canonicalize().unwrap();
        for sub in ["home", "tmp"] {
            std::fs::create_dir(path.join(sub)).unwrap();
        }
        Self { _dir: dir, path }
    }
    fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
    fn file(&self, name: &str, bytes: &[u8]) -> String {
        let path = self.join(name);
        std::fs::write(&path, bytes).unwrap();
        path.to_str().unwrap().to_owned()
    }
    fn command(&self, args: &[&OsStr]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_brn"));
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("BRN_") {
                command.env_remove(key);
            }
        }
        let tmp = self.join("tmp");
        command
            .env("HOME", self.join("home"))
            .env("CODEX_HOME", self.join("home"))
            .env("TMPDIR", &tmp)
            .env("TMP", &tmp)
            .env("TEMP", &tmp)
            .args(args);
        command
    }
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}
impl Run {
    fn envelope(&self) -> Value {
        assert_eq!(
            self.stdout.matches('\n').count(),
            1,
            "one line: {}",
            self.stdout
        );
        let value: Value = serde_json::from_str(&self.stdout).unwrap();
        assert_eq!(value["contract"], 3);
        assert_eq!(value["exit"], self.code);
        assert_eq!(value["ok"], self.code == 0);
        assert_eq!(value.as_object().unwrap().len(), 16, "{value}");
        assert_eq!(value["authority"]["enforced"], false, "{value}");
        assert!(!value["retry"].is_null());
        assert!(self.stderr.is_empty() || !self.stderr.contains("panicked"));
        value
    }
}

fn status_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(-1)
}

/// Runs to completion within 30 s, feeding `stdin` when given.
fn run_os(root: &Root, args: &[&OsStr], stdin: Option<&[u8]>) -> Run {
    let mut command = root.command(args);
    command
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    if let Some(bytes) = stdin {
        let mut input = child.stdin.take().unwrap();
        let bytes = bytes.to_vec();
        std::thread::spawn(move || {
            let _ = input.write_all(&bytes);
        });
    }
    wait_output(child, Duration::from_secs(30))
}
fn wait_output(child: Child, limit: Duration) -> Run {
    let id = child.id();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    let output = match receiver.recv_timeout(limit) {
        Ok(output) => output.unwrap(),
        Err(_) => {
            // SAFETY: signals only the child this test spawned.
            unsafe { libc::kill(id as i32, libc::SIGKILL) };
            panic!("brn did not finish within {limit:?}");
        }
    };
    Run {
        code: status_code(output.status),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}
fn args<'a>(json: bool, data: &'a Path, rest: &'a [&'a str]) -> Vec<&'a OsStr> {
    let mut args: Vec<&OsStr> = vec![];
    if json {
        args.push(OsStr::new("--json"));
    }
    args.push(OsStr::new("--data-dir"));
    args.push(data.as_os_str());
    args.extend(rest.iter().map(OsStr::new));
    args
}
fn run(root: &Root, json: bool, data: &Path, rest: &[&str], stdin: Option<&[u8]>) -> Run {
    run_os(root, &args(json, data, rest), stdin)
}
/// One machine-mode call: the envelope.
fn call(root: &Root, data: &Path, rest: &[&str]) -> Value {
    run(root, true, data, rest, None).envelope()
}
fn call_in(root: &Root, data: &Path, rest: &[&str], stdin: &[u8]) -> Value {
    run(root, true, data, rest, Some(stdin)).envelope()
}
/// A fresh test workspace (`init --purpose test`) for a writing test.
fn workspace(root: &Root, name: &str) -> PathBuf {
    let data = root.join(name);
    let run = run(root, true, &data, &["init", "--purpose", "test"], None);
    assert_eq!(run.code, 0, "init --purpose test: {}", run.stdout);
    data
}
/// `begin-edit … --token-out FILE`: the envelope and the token file.
fn begin(root: &Root, data: &Path, note: &str, base: &str) -> (Value, String) {
    let token = root.join(&format!("token-{note}-{base}-{}", next()));
    let token = token.to_str().unwrap().to_owned();
    let envelope = call(
        root,
        data,
        &[
            "begin-edit",
            "--note",
            note,
            "--base",
            base,
            "--token-out",
            &token,
        ],
    );
    (envelope, token)
}
fn next() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::SeqCst)
}
/// A machine-mode call that must succeed: the legacy result.
fn ok(root: &Root, data: &Path, rest: &[&str]) -> Value {
    let envelope = call(root, data, rest);
    assert_eq!(envelope["status"], "ok", "{rest:?}: {envelope}");
    envelope["result"].clone()
}
fn id(value: &Value) -> String {
    value["id"].as_str().unwrap().to_owned()
}
fn note(root: &Root, data: &Path, title: &str, text: &str) -> Value {
    ok(
        root,
        data,
        &["note-create", "--title", title, "--text", text],
    )
}
fn count(root: &Root, data: &Path, command: &str) -> usize {
    ok(root, data, &[command]).as_array().unwrap().len()
}
fn markdown(root: &Root, data: &Path, note: &str) -> String {
    ok(root, data, &["read", "--id", note])["data"]["Note"]["markdown"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn assert_error(envelope: &Value, status: &str, code: &str) {
    assert_eq!(envelope["status"], status, "{envelope}");
    assert_eq!(envelope["error"]["code"], code, "{envelope}");
    assert_eq!(envelope["result"], Value::Null);
    assert_eq!(envelope["outcome"], Value::Null);
    assert_eq!(envelope["changed"], json!([]));
    if matches!(status, "usage" | "rejected") {
        assert_eq!(envelope["retry"], "refuse", "{envelope}");
    }
}

#[test]
fn json_success_envelope_has_every_field_and_result_matches_default_mode() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let init = call(&root, &data, &["init"]);
    assert_eq!(
        (&init["outcome"], &init["operation"], &init["changed"]),
        (&Value::Null, &Value::Null, &json!([]))
    );
    assert_eq!(init["retry"], "converge");
    let created = call(
        &root,
        &data,
        &["note-create", "--title", "N", "--text", "body"],
    );
    assert_eq!(created["outcome"], "Applied");
    assert_eq!(created["retry"], "duplicate");
    let operation = created["operation"].as_str().unwrap();
    assert_eq!(created["changed"][0]["kind"], "Note");
    assert_eq!(created["changed"][0]["created"], true);
    assert_eq!(created["changed"][0]["id"], created["result"]["id"]);
    assert_eq!(id(&created["result"]), format!("{operation}:0"));
    let note = id(&created["result"]);
    let default = run(&root, false, &data, &["read", "--id", &note], None);
    assert_eq!(default.code, 0);
    let machine = call(&root, &data, &["read", "--id", &note]);
    assert_eq!(
        machine["result"],
        serde_json::from_str::<Value>(&default.stdout).unwrap()
    );
    assert_eq!(machine["retry"], "read");
    assert_eq!(machine["outcome"], Value::Null);
    let project = call(&root, &data, &["project-create", "--title", "P"]);
    assert_eq!(project["operation"], project["result"]["operation"]);
    assert_eq!(project["changed"][0]["kind"], "Project");
    assert_eq!(project["outcome"], "Applied");
    let pause = call(&root, &data, &["pause"]);
    assert_eq!(pause["outcome"], "Applied");
    assert_eq!(pause["operation"], pause["result"]["receipt"]["operation"]);
    assert!(!pause["changed"].as_array().unwrap().is_empty());
    let begin = call(
        &root,
        &data,
        &["begin-edit", "--note", &note, "--base", "1"],
    );
    assert_eq!(
        (
            &begin["status"],
            &begin["outcome"],
            &begin["operation"],
            &begin["changed"]
        ),
        (&json!("ok"), &Value::Null, &Value::Null, &json!([]))
    );
    let notes = call(&root, &data, &["notes"]);
    assert_eq!(notes["outcome"], Value::Null);
    assert_eq!(notes["conflict"], Value::Null);
    assert_eq!(notes["error"], Value::Null);
}

#[test]
fn usage_errors_are_json_with_stable_codes_and_create_nothing() {
    let root = Root::new();
    let data = root.join("absent");
    let d = data.to_str().unwrap();
    let source = root.join("missing-source.md");
    let s = source.to_str().unwrap();
    let rows: Vec<(Vec<&str>, &str, Option<&str>, bool)> = vec![
        (vec![], "missing_data_dir", None, true),
        (vec!["--data-dir"], "missing_data_dir", None, true),
        (
            vec!["--data-dir", "relative"],
            "invalid_data_dir",
            None,
            true,
        ),
        (vec!["--data-dir", d], "missing_command", None, true),
        (
            vec!["--data-dir", d, "bogus"],
            "unknown_command",
            None,
            true,
        ),
        (
            vec!["--data-dir", d, "notes", "oops"],
            "malformed_option",
            None,
            false,
        ),
        (
            vec!["--data-dir", d, "read", "--id"],
            "missing_value",
            Some("id"),
            false,
        ),
        (
            vec!["--data-dir", d, "read", "--id", "a", "--id", "b"],
            "repeated_option",
            Some("id"),
            false,
        ),
        (
            vec!["--data-dir", d, "notes", "--bogus", "x"],
            "unknown_option",
            Some("bogus"),
            false,
        ),
        (
            vec!["--data-dir", d, "note-create", "--title", "T"],
            "missing_option",
            Some("text"),
            false,
        ),
        (
            vec![
                "--data-dir",
                d,
                "note-create",
                "--title",
                "T",
                "--text",
                "x",
                "--text-file",
                "/x",
            ],
            "conflicting_options",
            None,
            false,
        ),
        (
            vec![
                "--data-dir",
                d,
                "note-create",
                "--title",
                "T",
                "--text",
                "x",
                "--key",
                "bad key",
            ],
            "invalid_value",
            Some("key"),
            false,
        ),
        (
            vec!["--data-dir", d, "read", "--id", "x", "--key", "k1"],
            "unknown_option",
            Some("key"),
            false,
        ),
        (
            // Round 2: --key is accepted only with --seed-chat yes.
            vec!["--data-dir", d, "knowledge-check", "--key", "k1"],
            "conflicting_options",
            None,
            false,
        ),
        (
            vec![
                "--data-dir",
                d,
                "intake",
                "--source",
                s,
                "--intent",
                "useful",
                "--key",
                "k1",
            ],
            "conflicting_options",
            None,
            false,
        ),
        (
            vec!["--data-dir", d, "intake", "--source", s, "--intent", "chat"],
            "missing_option",
            Some("thread"),
            false,
        ),
        (
            vec![
                "--data-dir",
                d,
                "intake",
                "--source",
                s,
                "--intent",
                "project",
            ],
            "missing_option",
            Some("project"),
            false,
        ),
        (
            vec![
                "--data-dir",
                d,
                "intake",
                "--source",
                s,
                "--intent",
                "useful",
                "--trace",
                "verbose",
            ],
            "invalid_value",
            Some("trace"),
            false,
        ),
        (
            vec!["--data-dir", d, "delegation", "--choice", "automatic"],
            "invalid_value",
            Some("choice"),
            false,
        ),
    ];
    for (rest, code, option, null_command) in &rows {
        let mut argv = vec![OsStr::new("--json")];
        argv.extend(rest.iter().map(OsStr::new));
        let envelope = run_os(&root, &argv, None).envelope();
        assert_eq!(envelope["exit"], 2, "{rest:?}: {envelope}");
        assert_error(&envelope, "usage", code);
        assert_eq!(
            envelope["error"]["details"]["option"],
            json!(option),
            "{rest:?}"
        );
        assert_eq!(envelope["command"].is_null(), *null_command, "{rest:?}");
        assert!(!data.exists(), "{rest:?} created the data directory");
        // Default mode: the same text on stderr, exit 1, nothing created.
        let default = run_os(
            &root,
            &rest.iter().map(OsStr::new).collect::<Vec<_>>(),
            None,
        );
        assert_eq!(default.code, 1, "{rest:?}");
        assert_eq!(default.stdout, "");
        assert_eq!(
            default.stderr,
            format!("{}\n", envelope["error"]["message"].as_str().unwrap())
        );
        assert!(!data.exists());
    }
    let key = call(
        &root,
        &data,
        &[
            "note-create",
            "--title",
            "T",
            "--text",
            "x",
            "--key",
            "bad key",
        ],
    );
    assert_eq!(key["key"], Value::Null);
    let mixed = call(
        &root,
        &data,
        &[
            "note-create",
            "--title",
            "T",
            "--text",
            "x",
            "--text-file",
            "/x",
        ],
    );
    assert_eq!(
        mixed["error"]["details"]["options"],
        json!(["text", "text-file"])
    );
    assert_eq!(
        mixed["error"]["message"],
        "Choose either --text or --text-file"
    );
    let useful = call(
        &root,
        &data,
        &["intake", "--source", s, "--intent", "useful", "--key", "k1"],
    );
    assert_eq!(
        useful["error"]["message"],
        "--key is not accepted with --intent useful"
    );
    for (intent, missing) in [
        ("chat", "Missing --thread"),
        ("project", "Missing --project"),
    ] {
        let default = run(
            &root,
            false,
            &data,
            &["intake", "--source", s, "--intent", intent],
            None,
        );
        assert_eq!(default.stderr, format!("{missing}\n"));
    }
    let trace = run(
        &root,
        false,
        &data,
        &[
            "intake", "--source", s, "--intent", "useful", "--trace", "verbose",
        ],
        None,
    );
    assert_eq!(trace.stderr, "--trace takes summary or full\n");
    let malformed = call(&root, &data, &["notes", "oops"]);
    assert_eq!(malformed["error"]["details"]["position"], 5);
    let unknown = call(&root, &data, &["notes", "--bogus", "x"]);
    // Round 2b: `notes` takes the page options.
    assert_eq!(
        unknown["error"]["details"]["allowed"],
        json!(["limit", "cursor"])
    );
    let bare = call(&root, &data, &["settings", "--bogus", "x"]);
    assert_eq!(bare["error"]["details"]["allowed"], json!([]));
    let private = call(&root, &data, &["notes", "--Secret Text", "x"]);
    assert!(private["error"]["details"].get("option").is_none());
    // `--json` is special only as the first argument.
    let second = run(&root, false, &data, &["--json", "read"], None);
    assert_eq!(
        (second.code, second.stderr.as_str()),
        (1, "Unknown command; run --help\n")
    );
    assert!(second.stdout.is_empty());
    assert!(!data.exists());
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bad = OsStr::from_bytes(b"\xff");
        let rest: Vec<&OsStr> = ["note-create", "--title", "T", "--text"]
            .iter()
            .map(OsStr::new)
            .chain([bad])
            .collect();
        let mut machine = args(true, &data, &[]);
        machine.extend(&rest);
        let envelope = run_os(&root, &machine, None).envelope();
        assert_error(&envelope, "usage", "malformed_option");
        assert_eq!(envelope["command"], Value::Null);
        assert_eq!(envelope["error"]["details"], json!({"position": 8}));
        assert_eq!(
            envelope["error"]["message"],
            "Arguments must be valid UTF-8"
        );
        let mut plain = args(false, &data, &[]);
        plain.extend(&rest);
        let default = run_os(&root, &plain, None);
        assert_eq!(
            (default.code, default.stderr.as_str()),
            (1, "Arguments must be valid UTF-8\n")
        );
        assert!(!data.exists());
    }
}

/// argv after the data directory, stdin, expected code and details.
type Case<'a> = (Vec<&'a str>, Option<&'a [u8]>, &'a str, Value);
/// The same with the expected message.
type Refusal<'a> = (Vec<&'a str>, Option<&'a [u8]>, &'a str, Value, &'a str);

#[test]
fn caller_input_mistakes_after_open_are_usage_or_rejected_not_internal() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let project = ok(&root, &data, &["project-create", "--title", "P"]);
    let project = project["writes"][0]["after"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let preview = ok(
        &root,
        &data,
        &["project-delete-preview", "--project", &project],
    );
    let preview = root.file("preview.json", preview.to_string().as_bytes());
    let not_json = root.file("not-json.json", b"not json");
    let missing = root.join("missing-preview.json");
    let missing = missing.to_str().unwrap();
    let cases: Vec<Case> = vec![
        (
            vec![
                "project-delete",
                "--preview-file",
                missing,
                "--actions",
                "{}",
            ],
            None,
            "invalid_input",
            json!({"option":"preview-file","reason":"unreadable"}),
        ),
        (
            vec![
                "project-delete",
                "--preview-file",
                &not_json,
                "--actions",
                "{}",
            ],
            None,
            "invalid_input",
            json!({"option":"preview-file","reason":"invalid_json"}),
        ),
        (
            vec![
                "project-delete",
                "--preview-file",
                &preview,
                "--actions",
                "{",
            ],
            None,
            "invalid_value",
            json!("actions"),
        ),
        (
            vec![
                "project-delete",
                "--preview-file",
                &preview,
                "--actions-file",
                "-",
            ],
            Some(b"{"),
            "invalid_value",
            json!("actions"),
        ),
    ];
    for (rest, stdin, code, details) in &cases {
        let output = run(&root, true, &data, rest, *stdin);
        let envelope = output.envelope();
        assert_eq!(envelope["exit"], 2, "{rest:?}");
        assert_error(&envelope, "usage", code);
        if details.is_object() {
            assert_eq!(envelope["error"]["details"], *details);
        } else {
            assert_eq!(envelope["error"]["details"]["option"], *details);
        }
        let default = run(&root, false, &data, rest, *stdin);
        assert_eq!(default.code, 1);
        assert_eq!(
            default.stderr,
            format!("{}\n", envelope["error"]["message"].as_str().unwrap())
        );
    }
    let parent = root.join("missing-parent");
    let destination = parent.join("dest");
    let destination = destination.to_str().unwrap();
    for command in ["backup", "export"] {
        let envelope = call(&root, &data, &[command, "--destination", destination]);
        assert_eq!(envelope["exit"], 6);
        assert_error(&envelope, "rejected", "invalid_request");
        assert_eq!(
            envelope["error"]["details"],
            json!({"option":"destination","reason":"parent_missing"})
        );
        let default = run(
            &root,
            false,
            &data,
            &[command, "--destination", destination],
            None,
        );
        assert_eq!(default.code, 1);
        // Export names its random staging folder; compare the text before it.
        let stable = |text: &str| text.split(".brn-export-").next().unwrap().to_owned();
        let message = envelope["error"]["message"].as_str().unwrap();
        assert_eq!(stable(&default.stderr), stable(&format!("{message}\n")));
    }
    // The refusal holds only while its cause does.
    std::fs::create_dir(&parent).unwrap();
    assert_eq!(
        call(&root, &data, &["export", "--destination", destination])["exit"],
        0
    );
    let again = call(&root, &data, &["export", "--destination", destination]);
    assert_error(&again, "rejected", "destination_exists");
}

#[test]
fn text_input_refusals_create_nothing() {
    let root = Root::new();
    let data = root.join("absent");
    let nul = root.file("nul.md", b"a\0b");
    let latin1 = root.file("latin1.md", b"caf\xe9");
    let large = root.file("large.md", &vec![b'a'; 16 * 1024 * 1024 + 1]);
    let dir = root.join("folder");
    std::fs::create_dir(&dir).unwrap();
    let dir = dir.to_str().unwrap().to_owned();
    let cases: Vec<Refusal> = vec![
        (
            vec!["--text-file", &nul],
            None,
            "invalid_input",
            json!({"option":"text-file","reason":"contains_nul"}),
            "--text-file: contains NUL",
        ),
        (
            vec!["--text-file", &latin1],
            None,
            "invalid_input",
            json!({"option":"text-file","reason":"not_utf8"}),
            "--text-file: not valid UTF-8",
        ),
        (
            vec!["--text-file", &large],
            None,
            "invalid_input",
            json!({"option":"text-file","reason":"too_large","max_bytes":16777216}),
            "--text-file: larger than 16777216 bytes",
        ),
        (
            vec!["--text-file", &dir],
            None,
            "invalid_input",
            json!({"option":"text-file","reason":"is_directory"}),
            "--text-file: is a directory",
        ),
        (
            vec!["--text-file", "/nonexistent/brn-text.md"],
            None,
            "invalid_input",
            json!({"option":"text-file","reason":"unreadable"}),
            "--text-file: cannot read the file",
        ),
        (
            vec!["--text-file", "notes.md"],
            None,
            "invalid_value",
            json!({"option":"text-file","expected":"absolute path or -"}),
            "--text-file takes an absolute path or -",
        ),
        (
            vec!["--title-file", "-", "--text-file", "-"],
            Some(b"x"),
            "invalid_input",
            json!({"option":"text-file","reason":"stdin_reused"}),
            "--text-file -: stdin is already read by --title-file",
        ),
        (
            vec!["--text", "x", "--text-file", "-"],
            Some(b"x"),
            "conflicting_options",
            json!({"options":["text","text-file"]}),
            "Choose either --text or --text-file",
        ),
    ];
    for (options, stdin, code, details, message) in &cases {
        let mut rest = vec!["note-create"];
        if !options.contains(&"--title-file") {
            rest.extend(["--title", "T"]);
        }
        rest.extend(options);
        let envelope = run(&root, true, &data, &rest, *stdin).envelope();
        assert_eq!(envelope["exit"], 2, "{rest:?}: {envelope}");
        assert_error(&envelope, "usage", code);
        assert_eq!(envelope["error"]["details"], *details);
        assert_eq!(envelope["error"]["message"], *message);
        assert!(!data.exists());
        let default = run(&root, false, &data, &rest, *stdin);
        assert_eq!(
            (default.code, default.stderr.clone()),
            (1, format!("{message}\n"))
        );
        assert!(!data.exists());
    }
}

#[cfg(unix)]
#[test]
fn terminal_sources_are_refused_without_blocking() {
    use std::os::unix::io::FromRawFd;
    if std::env::var_os("BRN_SKIP_PTY_TESTS").is_some_and(|v| v == "1") {
        return;
    }
    let fail = |call: &str| panic!("{call} failed: {}", std::io::Error::last_os_error());
    // SAFETY: plain libc pseudo-terminal calls on descriptors this test owns.
    let master = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
    if master < 0 {
        fail("posix_openpt");
    }
    if unsafe { libc::grantpt(master) } != 0 {
        fail("grantpt");
    }
    if unsafe { libc::unlockpt(master) } != 0 {
        fail("unlockpt");
    }
    let name = unsafe { libc::ptsname(master) };
    if name.is_null() {
        fail("ptsname");
    }
    let slave = unsafe { std::ffi::CStr::from_ptr(name) }
        .to_str()
        .unwrap()
        .to_owned();
    let root = Root::new();
    let data = root.join("absent");
    let start = Instant::now();
    let path = run(
        &root,
        true,
        &data,
        &["note-create", "--title", "T", "--text-file", &slave],
        None,
    );
    let envelope = path.envelope();
    assert_error(&envelope, "usage", "invalid_input");
    assert_eq!(
        envelope["error"]["details"],
        json!({"option":"text-file","reason":"is_terminal"})
    );
    let default = run(
        &root,
        false,
        &data,
        &["note-create", "--title", "T", "--text-file", &slave],
        None,
    );
    assert_eq!(default.stderr, "--text-file: is a terminal\n");
    let open_slave = || {
        let path = std::ffi::CString::new(slave.clone()).unwrap();
        let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR | libc::O_NOCTTY) };
        if fd < 0 {
            fail("open slave");
        }
        unsafe { std::fs::File::from_raw_fd(fd) }
    };
    for (json, expected) in [
        (true, None),
        (false, Some("--text-file -: stdin is a terminal\n")),
    ] {
        let mut command = root.command(&args(
            json,
            &data,
            &["note-create", "--title", "T", "--text-file", "-"],
        ));
        command
            .stdin(Stdio::from(open_slave()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = wait_output(command.spawn().unwrap(), Duration::from_secs(5));
        match expected {
            None => {
                let envelope = output.envelope();
                assert_eq!(
                    envelope["error"]["details"],
                    json!({"option":"text-file","reason":"is_terminal"})
                );
            }
            Some(text) => assert_eq!(output.stderr, text),
        }
    }
    assert!(start.elapsed() < Duration::from_secs(20));
    assert!(!data.exists());
    unsafe { libc::close(master) };
}

const UNICODE: &str = "--leading dashes $HOME ${PATH} $(whoami) `echo INJECTED`\r\n\
“curly” and \"straight\" 'quotes'\n\ttab\n👩‍💻 ❤️ مرحبا 你好\n";

#[test]
fn text_files_and_stdin_preserve_exact_multiline_unicode() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let path = root.file("text.md", UNICODE.as_bytes());
    let title = root.file("title.txt", "Plan “β” 👩‍💻".as_bytes());
    let from_file = ok(
        &root,
        &data,
        &["note-create", "--title-file", &title, "--text-file", &path],
    );
    let note = id(&from_file);
    assert_eq!(markdown(&root, &data, &note), UNICODE);
    assert_eq!(from_file["data"]["Note"]["title"], "Plan “β” 👩‍💻");
    let from_stdin = call_in(
        &root,
        &data,
        &["note-create", "--title", "S", "--text-file", "-"],
        UNICODE.as_bytes(),
    );
    assert_eq!(markdown(&root, &data, &id(&from_stdin["result"])), UNICODE);
    let (session, token) = begin(&root, &data, &note, "1");
    let session = id(&session["result"]);
    let edited = format!("{UNICODE}more\n");
    let update = call_in(
        &root,
        &data,
        &[
            "update-buffer",
            "--session",
            &session,
            "--generation",
            "1",
            "--text-file",
            "-",
            "--token-file",
            &token,
        ],
        edited.as_bytes(),
    );
    assert_eq!(update["outcome"], "Updated");
    ok(
        &root,
        &data,
        &[
            "save",
            "--session",
            &session,
            "--generation",
            "1",
            "--token-file",
            &token,
        ],
    );
    assert_eq!(markdown(&root, &data, &note), edited);
}

#[test]
fn large_text_beyond_argv_limits_is_saved_through_a_file() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let text: String = "line of synthetic text\n".repeat(2 * 1024 * 1024 / 23 + 1);
    assert!(text.len() > 2 * 1024 * 1024);
    let path = root.file("large.md", text.as_bytes());
    let created = ok(
        &root,
        &data,
        &["note-create", "--title", "Large", "--text-file", &path],
    );
    assert_eq!(markdown(&root, &data, &id(&created)), text);
}

#[test]
fn stale_generation_closed_and_ignored_exit_3_with_current_versions() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let note = id(&note(&root, &data, "N", "one"));
    let edit = |base: &str, text: &str| {
        let (session, token) = begin(&root, &data, &note, base);
        let session = id(&session["result"]);
        ok(
            &root,
            &data,
            &[
                "update-buffer",
                "--session",
                &session,
                "--generation",
                "1",
                "--text",
                text,
                "--token-file",
                &token,
            ],
        );
        (session, token)
    };
    let save = |(session, token): &(String, String), generation: &str| {
        call(
            &root,
            &data,
            &[
                "save",
                "--session",
                session,
                "--generation",
                generation,
                "--token-file",
                token,
            ],
        )
    };
    let a = edit("1", "two");
    assert_eq!(save(&a, "1")["status"], "ok");
    let b = edit("2", "three");
    assert_eq!(save(&b, "1")["status"], "ok");
    // A session based on version 2 while the note is at version 3.
    let stale = edit("2", "late");
    let buffer = |text: &str| {
        call(
            &root,
            &data,
            &[
                "update-buffer",
                "--session",
                &stale.0,
                "--generation",
                "1",
                "--text",
                text,
                "--token-file",
                &stale.1,
            ],
        )
    };
    let identical = buffer("late");
    assert_eq!(
        (identical["exit"].clone(), identical["outcome"].clone()),
        (json!(0), json!("Ignored"))
    );
    assert_eq!(identical["retry"], "replay");
    let older = buffer("other");
    assert_eq!(
        (older["exit"].clone(), older["status"].clone()),
        (json!(3), json!("stale"))
    );
    let mismatch = save(&stale, "2");
    assert_eq!(
        (mismatch["exit"].clone(), mismatch["outcome"].clone()),
        (json!(3), json!("GenerationMismatch"))
    );
    let saved = save(&stale, "1");
    assert_eq!(
        (saved["exit"].clone(), saved["outcome"].clone()),
        (json!(3), json!("Stale"))
    );
    assert_eq!(saved["operation"], Value::Null);
    assert_eq!(saved["retry"], "replay");
    assert_eq!(
        saved["conflict"],
        json!([{"id":note,"kind":"Note","version":3,"archived":false,"open_edit":true}])
    );
    assert_eq!(saved["result"]["session"]["markdown"], "late");
    let discard = || {
        call(
            &root,
            &data,
            &[
                "discard",
                "--session",
                &stale.0,
                "--generation",
                "1",
                "--token-file",
                &stale.1,
            ],
        )
    };
    let discarded = discard();
    assert_eq!(
        (discarded["exit"].clone(), discarded["outcome"].clone()),
        (json!(0), json!("Discarded"))
    );
    let again = discard();
    assert_eq!(
        (again["exit"].clone(), again["outcome"].clone()),
        (json!(3), json!("Ignored"))
    );
    assert_eq!(again["retry"], "refuse");
    let closed = save(&stale, "1");
    assert_eq!(
        (closed["exit"].clone(), closed["outcome"].clone()),
        (json!(3), json!("Closed"))
    );
    assert_eq!(closed["conflict"][0]["open_edit"], false);
    assert_eq!(markdown(&root, &data, &note), "three");
}

#[test]
fn open_edit_session_reports_edit_session_open() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let note = id(&note(&root, &data, "N", "one"));
    let (session, token) = begin(&root, &data, &note, "1");
    let session = id(&session["result"]);
    let envelope = call(
        &root,
        &data,
        &["begin-edit", "--note", &note, "--base", "1"],
    );
    assert_eq!(envelope["exit"], 6);
    assert_error(&envelope, "rejected", "edit_session_open");
    assert_eq!(
        envelope["error"]["details"],
        json!({"note":note,"base_version":1,"holder":"caller"})
    );
    let secret = std::fs::read_to_string(&token).unwrap();
    assert!(!envelope.to_string().contains(secret.trim()));
    // Owner decision B3 part 1: the open session's ID is never named.
    assert!(!envelope.to_string().contains(&session));
    let message = envelope["error"]["message"].as_str().unwrap();
    assert_eq!(
        message,
        "This note has an open unsaved edit session (details.note, details.base_version, details.holder). Write to it only with the token your own successful begin-edit returned; otherwise report it to the owner."
    );
    assert!(!message.contains("Save or discard"));
    let default = run(
        &root,
        false,
        &data,
        &["begin-edit", "--note", &note, "--base", "1"],
        None,
    );
    // Default mode prints the daily-workflow structured line, without the session.
    assert_eq!(default.code, 1);
    let line: Value = serde_json::from_str(default.stderr.trim()).unwrap();
    assert_eq!(
        line,
        json!({"error":{
            "kind":"edit_session_open",
            "message":"This note already has an open unsaved edit. Save or discard that session first; the recovery command lists it.",
            "note":note,
            "base_version":1,
        }})
    );
    assert!(!default.stderr.contains(&session));
}

fn deferred_undo_setup(root: &Root, data: &Path) -> (String, String, (String, String)) {
    let created = call(
        root,
        data,
        &["note-create", "--title", "N", "--text", "one"],
    );
    let operation = created["operation"].as_str().unwrap().to_owned();
    let note = id(&created["result"]);
    let (session, token) = begin(root, data, &note, "1");
    (operation, note, (id(&session["result"]), token))
}
fn discard_zero(root: &Root, data: &Path, (session, token): &(String, String)) {
    ok(
        root,
        data,
        &[
            "discard",
            "--session",
            session,
            "--generation",
            "0",
            "--token-file",
            token,
        ],
    );
}
fn unapplied(root: &Root, data: &Path) -> Vec<String> {
    ok(root, data, &["candidates"])
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["operation"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn deferred_undo_exits_4_and_keyed_retry_applies_the_same_compensation() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let (operation, note, session) = deferred_undo_setup(&root, &data);
    let undo = ["undo", "--operation", operation.as_str(), "--key", "u1"];
    let deferred = call(&root, &data, &undo);
    assert_eq!(
        (deferred["exit"].clone(), deferred["status"].clone()),
        (json!(4), json!("deferred"))
    );
    assert_eq!(deferred["outcome"], "Deferred");
    assert_eq!(deferred["retry"], "replay");
    assert_eq!(deferred["key"], "u1");
    let compensation = deferred["operation"].as_str().unwrap().to_owned();
    assert_eq!(deferred["conflict"][0]["kind"], "Note");
    assert_eq!(deferred["conflict"][0]["id"], note);
    assert_eq!(deferred["conflict"][0]["open_edit"], true);
    discard_zero(&root, &data, &session);
    for _ in 0..2 {
        let applied = call(&root, &data, &undo);
        assert_eq!(
            (applied["exit"].clone(), applied["outcome"].clone()),
            (json!(0), json!("Applied"))
        );
        assert_eq!(applied["operation"], compensation);
        assert_eq!(applied["retry"], "replay");
    }
    assert!(!unapplied(&root, &data).contains(&compensation));
}

#[test]
fn unkeyed_deferred_undo_reports_duplicate() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let (operation, _note, session) = deferred_undo_setup(&root, &data);
    let undo = ["undo", "--operation", operation.as_str()];
    let deferred = call(&root, &data, &undo);
    assert_eq!(deferred["exit"], 4);
    assert_eq!(deferred["retry"], "duplicate");
    let first = deferred["operation"].as_str().unwrap().to_owned();
    discard_zero(&root, &data, &session);
    let applied = call(&root, &data, &undo);
    assert_eq!(applied["outcome"], "Applied");
    assert_ne!(applied["operation"], first);
    // What `duplicate` promises: the first attempt stays as an unapplied candidate.
    assert!(unapplied(&root, &data).contains(&first));
}

#[test]
fn not_found_destination_exists_and_unsafe_directory_are_rejected() {
    let root = Root::new();
    let data = workspace(&root, "data");
    note(&root, &data, "N", "one");
    let missing = call(&root, &data, &["read", "--id", "no-such-record"]);
    assert_error(&missing, "rejected", "not_found");
    assert_eq!(missing["error"]["details"], json!({}));
    assert_eq!(
        run(
            &root,
            false,
            &data,
            &["read", "--id", "no-such-record"],
            None
        )
        .stdout,
        "null\n"
    );
    let backup = root.join("backup.sqlite3");
    let backup = backup.to_str().unwrap();
    let export = root.join("export");
    let export = export.to_str().unwrap();
    for (command, destination) in [("backup", backup), ("export", export)] {
        assert_eq!(
            call(&root, &data, &[command, "--destination", destination])["exit"],
            0
        );
        let again = call(&root, &data, &[command, "--destination", destination]);
        assert_eq!(again["exit"], 6);
        assert_error(&again, "rejected", "destination_exists");
        assert_eq!(again["error"]["details"], json!({"option":"destination"}));
    }
    // Round 2: a relative destination is refused before opening.
    let relative = call(
        &root,
        &data,
        &["export", "--destination", "relative-export"],
    );
    assert_error(&relative, "usage", "invalid_value");
    let occupied = root.join("occupied");
    std::fs::create_dir(&occupied).unwrap();
    std::fs::write(occupied.join("file"), b"synthetic").unwrap();
    let restore = call(&root, &occupied, &["restore", "--source", backup]);
    assert_error(&restore, "rejected", "unsafe_directory");
    assert_eq!(restore["error"]["details"], json!({}));
    let old = root.join("old");
    std::fs::create_dir(&old).unwrap();
    std::fs::write(old.join("brn.sqlite"), b"old format synthetic bytes").unwrap();
    assert_error(
        &call(&root, &old, &["init"]),
        "rejected",
        "unsafe_directory",
    );
}

#[test]
fn symlinked_data_dir_is_data_folder_unusable() {
    let root = Root::new();
    let real = workspace(&root, "real");
    let link = root.join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let refused = call(&root, &link, &["notes"]);
    assert_error(&refused, "rejected", "data_folder_unusable");
    assert_eq!(refused["exit"], 6);
    // `details` never echo the --data-dir value.
    assert_eq!(
        refused["error"]["details"],
        json!({"problem":"symlink_refused"})
    );
    let file = root.join("file");
    std::fs::write(&file, b"synthetic").unwrap();
    let refused = call(&root, &file, &["init"]);
    assert_error(&refused, "rejected", "data_folder_unusable");
    assert_eq!(
        refused["error"]["details"],
        json!({"problem":"not_a_directory"})
    );
    // Default mode keeps the core's owner-terms message and exit 1.
    let default = run(&root, false, &link, &["notes"], None);
    assert_eq!(default.code, 1);
    assert!(default
        .stderr
        .starts_with("BRN cannot use the data folder "));
}

#[test]
fn ask_refused_before_the_provider_is_rejected_not_failed() {
    let root = Root::new();
    let data = workspace(&root, "data");
    // Round 2: a CLI-created workspace has no credential source until configured.
    let creds = root.join("creds");
    std::fs::create_dir(&creds).unwrap();
    ok(
        &root,
        &data,
        &[
            "configure",
            "--provider",
            "Copilot",
            "--model",
            "synthetic",
            "--effort",
            "low",
            "--credentials-dir",
            creds.to_str().unwrap(),
        ],
    );
    let refused = call(
        &root,
        &data,
        &["ask", "--thread", "no-such-chat", "--prompt", "Hello"],
    );
    assert_eq!(refused["exit"], 6);
    assert_error(&refused, "rejected", "invalid_request");
    assert_eq!(refused["error"]["details"], json!({"kind":"tool_rejected"}));
    let default = run(
        &root,
        false,
        &data,
        &["ask", "--thread", "no-such-chat", "--prompt", "Hello"],
        None,
    );
    assert_eq!(
        (default.code, default.stderr.as_str()),
        (1, "Provider run failed: ToolRejected\n")
    );
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let prompt = root.file("prompt.md", &vec![b'p'; 128 * 1024 + 1]);
    let large = call(
        &root,
        &data,
        &["ask", "--thread", &chat, "--prompt-file", &prompt],
    );
    assert_error(&large, "rejected", "invalid_request");
    assert_eq!(large["error"]["details"], json!({"kind":"tool_rejected"}));
    // Project boundaries are refused offline too.
    let scoped = call(
        &root,
        &data,
        &["search", "--query", "x", "--project", &chat],
    );
    assert_error(&scoped, "rejected", "invalid_request");
    // Round 2 adds details.reason (additive).
    assert_eq!(
        scoped["error"]["details"],
        json!({"option":"project","reason":"inactive_target"})
    );
    let both = call(
        &root,
        &data,
        &[
            "search",
            "--query",
            "x",
            "--project",
            &chat,
            "--thread",
            &chat,
        ],
    );
    assert_error(&both, "usage", "conflicting_options");
    assert_eq!(
        both["error"]["details"],
        json!({"options":["project","thread"]})
    );
}

fn queue_length(root: &Root, data: &Path) -> usize {
    ok(root, data, &["records"])
        .as_array()
        .unwrap()
        .iter()
        .find_map(|r| r["data"]["ChatControl"]["queue"].as_array().map(Vec::len))
        .unwrap_or(0)
}

#[test]
fn keyed_creates_replay_and_reuse_with_other_inputs_is_refused() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Base chat"]));
    let other = id(&ok(
        &root,
        &data,
        &["thread-start", "--title", "Other chat"],
    ));
    let rows: Vec<(Vec<&str>, Vec<&str>)> = vec![
        (
            vec!["note-create", "--title", "N", "--text", "x", "--key", "n1"],
            vec!["note-create", "--title", "N", "--text", "y", "--key", "n1"],
        ),
        (
            vec!["thread-start", "--title", "T", "--key", "t1"],
            vec!["thread-start", "--title", "T2", "--key", "t1"],
        ),
        (
            vec!["queue", "--thread", &chat, "--text", "q", "--key", "q1"],
            vec!["queue", "--thread", &other, "--text", "q", "--key", "q1"],
        ),
        (
            vec!["archive", "--thread", &chat, "--key", "a1"],
            vec!["archive", "--thread", &other, "--key", "a1"],
        ),
        (
            vec!["unarchive", "--thread", &chat, "--key", "u1"],
            vec!["unarchive", "--thread", &other, "--key", "u1"],
        ),
        (
            vec!["project-create", "--title", "P", "--key", "p1"],
            vec!["project-create", "--title", "P2", "--key", "p1"],
        ),
    ];
    let counts = |root: &Root| {
        (
            count(root, &data, "notes"),
            count(root, &data, "threads"),
            queue_length(root, &data),
            count(root, &data, "projects"),
        )
    };
    for (first, conflicting) in &rows {
        let once = call(&root, &data, first);
        assert_eq!(once["status"], "ok", "{first:?}: {once}");
        assert_eq!(once["retry"], "replay");
        assert!(once["operation"].is_string());
        let before = counts(&root);
        let twice = call(&root, &data, first);
        assert_eq!(twice["operation"], once["operation"], "{first:?}");
        assert_eq!(twice["result"], once["result"], "{first:?}");
        assert_eq!(counts(&root), before, "{first:?}");
        let refused = call(&root, &data, conflicting);
        assert_eq!(refused["exit"], 6, "{conflicting:?}: {refused}");
        assert_error(&refused, "rejected", "key_conflict");
        let key = first.last().unwrap();
        assert_eq!(refused["error"]["details"], json!({"key": key}));
        assert_eq!(refused["key"], *key);
        assert_eq!(counts(&root), before);
    }
    let unkeyed = call(
        &root,
        &data,
        &["note-create", "--title", "N", "--text", "x"],
    );
    assert_eq!(unkeyed["retry"], "duplicate");
    assert_eq!(unkeyed["key"], Value::Null);
}

#[cfg(unix)]
fn interrupt_after_writing(mut command: Command, bytes: &[u8]) -> (Run, Instant) {
    support::isolate(&mut command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    // More than a pipe holds: once written, the child is reading and its handler is installed.
    input.write_all(bytes).unwrap();
    let sent = Instant::now();
    // SAFETY: signals only the child this test spawned.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    let output = wait_output(child, Duration::from_secs(20));
    drop(input);
    (output, sent)
}

#[cfg(unix)]
#[test]
fn sigint_while_stdin_stays_open_is_interrupted_and_creates_nothing() {
    let root = Root::new();
    let data = root.join("absent");
    let command = root.command(&args(
        true,
        &data,
        &["note-create", "--title", "T", "--text-file", "-"],
    ));
    let (output, sent) = interrupt_after_writing(command, &vec![b'a'; 256 * 1024]);
    // Prompt in practice; the bound only catches a hang on a loaded runner.
    assert!(sent.elapsed() < Duration::from_secs(20));
    assert_eq!(output.code, 130);
    let envelope = output.envelope();
    assert_error(&envelope, "interrupted", "interrupted");
    assert_eq!(envelope["retry"], "duplicate");
    assert!(!data.exists());
}

#[cfg(unix)]
#[test]
fn sigint_while_a_fifo_has_no_writer_is_interrupted() {
    let root = Root::new();
    let data = root.join("absent");
    let fifo = root.join("input.fifo");
    let path = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    // SAFETY: creates a FIFO inside the test root.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut command = root.command(&args(
        true,
        &data,
        &[
            "note-create",
            "--title-file",
            "-",
            "--text-file",
            fifo.to_str().unwrap(),
        ],
    ));
    support::isolate(&mut command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    // Once 256 KiB of title is written the child is inside input resolution:
    // either reading stdin or waiting on the FIFO, and both points give 130.
    input.write_all(&vec![b't'; 256 * 1024]).unwrap();
    drop(input);
    // SAFETY: signals only the child this test spawned.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    let output = wait_output(child, Duration::from_secs(20));
    assert_eq!(output.code, 130);
    assert_error(&output.envelope(), "interrupted", "interrupted");
    assert!(!data.exists());
}

#[cfg(unix)]
#[test]
fn refuse_retry_after_an_interrupted_first_attempt_applies_once() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let receipt = ok(&root, &data, &["project-create", "--title", "P"]);
    let project = receipt["writes"][0]["after"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let version = ok(&root, &data, &["read", "--id", &project])["version"]
        .as_u64()
        .unwrap();
    let base = version.to_string();
    let settings = [
        "project-settings",
        "--project",
        &project,
        "--base",
        &base,
        "--title",
        "T",
        "--knowledge",
        "yes",
        "--instructions-file",
        "-",
    ];
    let instructions = vec![b'i'; 96 * 1024];
    let (interrupted, _) =
        interrupt_after_writing(root.command(&args(true, &data, &settings)), &instructions);
    assert_eq!(interrupted.code, 130);
    let envelope = interrupted.envelope();
    assert_eq!(envelope["status"], "interrupted");
    assert_eq!(envelope["retry"], "refuse");
    assert_eq!(
        ok(&root, &data, &["read", "--id", &project])["version"],
        version
    );
    let applied = call_in(&root, &data, &settings, &instructions);
    assert_eq!(applied["exit"], 0, "{applied}");
    assert_eq!(
        ok(&root, &data, &["read", "--id", &project])["version"],
        version + 1
    );
    let again = call_in(&root, &data, &settings, &instructions);
    assert_eq!(again["exit"], 6);
    assert_error(&again, "rejected", "invalid_request");
    assert_eq!(
        ok(&root, &data, &["read", "--id", &project])["version"],
        version + 1
    );
}

/// Markdown intake through the converter, with the child's temp dir in the root.
#[cfg(target_os = "macos")]
fn markdown_source(root: &Root) -> String {
    root.file(
        "source.md",
        b"# Harbor\n\n| Step | Owner |\n|---|---|\n| Check | Ana |\n",
    )
}

#[cfg(unix)]
#[test]
fn keyed_keep_in_brn_retry_replays() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    #[cfg(target_os = "macos")]
    let note = {
        let source = markdown_source(&root);
        let attached = call(
            &root,
            &data,
            &[
                "intake", "--source", &source, "--intent", "chat", "--thread", &chat,
            ],
        );
        assert_eq!(attached["outcome"], "Applied", "{attached}");
        attached["changed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["kind"] == "Note")
            .unwrap()
            .clone()
    };
    #[cfg(not(target_os = "macos"))]
    let note = json!({"id": support::chat_attachment_fixture(&data, &chat, "keep-fixture", "Harbor", "Chat only."), "version": 1});
    let version = note["version"].to_string();
    let keep = [
        "keep-in-brn",
        "--note",
        note["id"].as_str().unwrap(),
        "--base",
        &version,
        "--key",
        "k1",
    ];
    let first = call(&root, &data, &keep);
    assert_eq!(first["outcome"], "Applied", "{first}");
    assert_eq!(first["retry"], "replay");
    let second = call(&root, &data, &keep);
    assert_eq!(second["outcome"], "Applied");
    assert_eq!(second["operation"], first["operation"]);
}

#[cfg(target_os = "macos")]
#[test]
fn keyed_markdown_full_import_replays_and_title_change_conflicts() {
    let root = Root::new();
    // Round 2: machine-mode reads never create a workspace; init does.
    let data = workspace(&root, "data");
    let settings = ok(&root, &data, &["settings"]);
    if let Some(auth) = settings["auth_file"].as_str() {
        assert!(
            !Path::new(auth).is_file(),
            "synthetic HOME must hold no credentials"
        );
    }
    let source = markdown_source(&root);
    let import = [
        "intake", "--source", &source, "--intent", "full", "--title", "Harbor", "--key", "i1",
    ];
    let first = call(&root, &data, &import);
    assert_eq!(first["outcome"], "Applied", "{first}");
    assert_eq!(first["retry"], "replay");
    let notes = count(&root, &data, "notes");
    let second = call(&root, &data, &import);
    assert_eq!(second["operation"], first["operation"]);
    assert_eq!(count(&root, &data, "notes"), notes);
    let retitled = [
        "intake", "--source", &source, "--intent", "full", "--title", "Other", "--key", "i1",
    ];
    let refused = call(&root, &data, &retitled);
    assert_error(&refused, "rejected", "key_conflict");
    assert_eq!(refused["error"]["details"], json!({"key":"i1"}));
    // Unkeyed full imports keep today's behaviour: a new operation each time.
    let unkeyed = call(
        &root,
        &data,
        &[
            "intake", "--source", &source, "--intent", "full", "--title", "Harbor",
        ],
    );
    assert_eq!(unkeyed["retry"], "duplicate");
    assert_ne!(unkeyed["operation"], first["operation"]);
}

#[cfg(unix)]
#[test]
fn closed_stdout_does_not_panic() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let path = root.file("big.md", &vec![b'x'; 200 * 1024]);
    let note = id(&ok(
        &root,
        &data,
        &["note-create", "--title", "Big", "--text-file", &path],
    ));
    for json in [false, true] {
        let mut command = root.command(&args(json, &data, &["read", "--id", &note]));
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();
        drop(child.stdout.take());
        let output = wait_output(child, Duration::from_secs(30));
        assert_eq!(output.code, 0, "json={json}: {}", output.stderr);
        assert!(!output.stderr.contains("panicked"));
    }
}

#[test]
fn default_mode_domain_outcomes_still_exit_zero() {
    let root = Root::new();
    let data = workspace(&root, "data");
    let created = call(
        &root,
        &data,
        &["note-create", "--title", "N", "--text", "one"],
    );
    let operation = created["operation"].as_str().unwrap().to_owned();
    let undo = ["undo", "--operation", operation.as_str()];
    assert_eq!(run(&root, false, &data, &undo, None).code, 0);
    let stale = run(&root, false, &data, &undo, None);
    assert_eq!(stale.code, 0);
    assert_eq!(
        serde_json::from_str::<Value>(&stale.stdout).unwrap()["outcome"],
        "Stale"
    );
    assert_eq!(call(&root, &data, &undo)["exit"], 3);
}

fn section<'a>(readme: &'a str, heading: &str) -> &'a str {
    let start = readme.find(&format!("\n### {heading}\n")).unwrap();
    let rest = &readme[start + 1..];
    let end = rest[4..].find("\n### ").map_or(rest.len(), |i| i + 4);
    &rest[..end]
}
fn cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split(" | ")
        .map(|cell| cell.trim().to_owned())
        .collect()
}
fn first_word(cell: &str) -> String {
    cell.split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches('`')
        .to_owned()
}

#[test]
fn machine_description_lists_every_parsed_command_and_matches_readme() {
    let root = Root::new();
    let help = run_os(&root, &[OsStr::new("--json"), OsStr::new("--help")], None);
    assert_eq!(help.code, 0);
    assert_eq!(help.stdout.matches('\n').count(), 1);
    let description: Value = serde_json::from_str(&help.stdout).unwrap();
    let commands = description["commands"].as_array().unwrap();
    for command in commands {
        let name = command["name"].as_str().unwrap();
        let data = root.join(&format!("dispatch-{name}"));
        let required = command["required"].as_array().unwrap();
        if let Some(first) = required.first() {
            let envelope = call(&root, &data, &[name]);
            assert_error(&envelope, "usage", "missing_option");
            assert_eq!(envelope["error"]["details"]["option"], *first, "{name}");
            assert_eq!(envelope["command"], name);
            assert!(!data.exists(), "{name}");
        } else {
            let envelope = call(&root, &data, &[name]);
            assert_eq!(envelope["command"], name, "{envelope}");
            assert_ne!(envelope["error"]["code"], "unknown_command");
        }
    }
    let readme =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap();
    let rows = |heading: &str, columns: usize| -> Vec<Vec<String>> {
        section(&readme, heading)
            .lines()
            .filter(|line| line.starts_with("| ") && !line.starts_with("|---"))
            .map(cells)
            .filter(|cells| cells.len() == columns)
            .skip(1)
            .collect()
    };
    let statuses: Vec<(String, String)> = rows("Statuses", 4)
        .iter()
        .map(|cells| (cells[0].clone(), first_word(&cells[1])))
        .collect();
    let described: Vec<(String, String)> = description["statuses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["exit"].to_string(),
                s["status"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(statuses, described);
    let errors: Vec<(String, String)> = rows("Error codes", 4)
        .iter()
        .map(|cells| (first_word(&cells[0]), first_word(&cells[1])))
        .collect();
    let described: Vec<(String, String)> = description["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["code"].as_str().unwrap().to_owned(),
                e["status"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(errors, described);
    let retry_rows = rows("Retries", 5);
    let mut expected = vec![];
    for command in commands {
        let name = command["name"].as_str().unwrap();
        let owner = match command["owner_decision_options"]
            .as_array()
            .unwrap()
            .as_slice()
        {
            [] if command["owner_decision"] == true => "yes".to_owned(),
            [] => "no".to_owned(),
            options => format!(
                "with {}",
                options
                    .iter()
                    .map(|o| format!("--{}", o.as_str().unwrap()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        let keyed = command["retry"]["keyed"].as_str().unwrap_or("—").to_owned();
        let key = if command["key"] == true { "yes" } else { "no" };
        expected.push((
            format!("`{name}`"),
            key.to_owned(),
            command["retry"]["default"].as_str().unwrap().to_owned(),
            keyed,
            owner.clone(),
        ));
        for variant in command["retry"]["variants"].as_array().unwrap() {
            let when = &variant["when"];
            expected.push((
                format!(
                    "`{name}` (--{} {})",
                    when["option"].as_str().unwrap(),
                    when["equals"].as_str().unwrap()
                ),
                if variant["key"] == true { "yes" } else { "no" }.to_owned(),
                variant["retry"].as_str().unwrap().to_owned(),
                variant["keyed"].as_str().unwrap_or("—").to_owned(),
                owner.clone(),
            ));
        }
    }
    let readme_rows: Vec<_> = retry_rows
        .iter()
        .map(|c| {
            (
                c[0].clone(),
                c[1].clone(),
                first_word(&c[2]),
                first_word(&c[3]),
                c[4].clone(),
            )
        })
        .collect();
    assert_eq!(readme_rows, expected);
    // Every global option, `actor` included, is named where the README lists them.
    let globals = readme
        .lines()
        .find(|line| line.contains("The global options"))
        .expect("README global options sentence");
    for option in description["global_options"].as_array().unwrap() {
        let name = option["option"].as_str().unwrap();
        assert!(globals.contains(&format!("`--{name}`")), "{name}");
    }
    assert_eq!(description["global_options"][2]["env"], "BRN_ACTOR");
    assert!(readme.contains(&format!(
        "`{}`",
        description["global_options"][2]["grammar"]["pattern"]
            .as_str()
            .unwrap()
    )));
    // The envelope's authority keys and the edit token pattern are documented.
    let envelope = section(&readme, "Envelope");
    for key in ["workspace_purpose", "cli_authority", "enforced", "actor"] {
        assert!(envelope.contains(&format!("`{key}`")), "{key}");
    }
    assert_eq!(description["authority"]["enforced"], false);
    assert!(readme.contains(&format!(
        "`{}`",
        description["edit_token"]["pattern"].as_str().unwrap()
    )));
    let rule = format!("> {}", description["owner_decision_rule"].as_str().unwrap());
    assert!(
        readme.lines().any(|line| line == rule),
        "README lacks the owner rule line"
    );
}
