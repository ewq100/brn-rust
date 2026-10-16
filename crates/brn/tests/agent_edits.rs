//! Process checks of owner decision B3: stale edits stay conflicts,
//! `edit_session_open` never names the open session, the README's
//! stale-save recipe completes as written, and (round 3) edit writes need
//! the caller's own session token, which never leaks.
mod support;
use serde_json::{json, Value};
use std::path::Path;
use support::*;

const README: &str = include_str!("../README.md");

/// The first code span of each numbered step of the README's stale-save
/// recipe, in order (step 4, "reapply", has none).
fn recipe_commands() -> Vec<String> {
    let start = README.find("- **Stale save** (exit 3).").expect("recipe");
    README[start..]
        .lines()
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .filter_map(|line| {
            let line = line.trim_start();
            let (number, rest) = line.split_once(". ")?;
            number.parse::<u8>().ok()?;
            let command = rest.split('`').nth(1)?.split_whitespace().next()?;
            Some(command.to_owned())
        })
        .collect()
}

/// `begin-edit --token-out`: the session ID and its token file.
fn begin(root: &Root, data: &Path, note: &str, base: u64) -> (String, String) {
    let (session, token) = begin_edit(root, data, note, base);
    (id(&session), token)
}
fn buffer(root: &Root, data: &Path, (session, token): &(String, String), text: &str) {
    let file = root.file(&format!("{session}.md"), text.as_bytes());
    let argv = [
        "update-buffer",
        "--session",
        session,
        "--generation",
        "1",
        "--text-file",
        &file,
        "--token-file",
        token,
    ];
    assert_eq!(call(root, data, &argv)["outcome"], "Updated");
}
fn save(root: &Root, data: &Path, (session, token): &(String, String), generation: &str) -> Value {
    call(
        root,
        data,
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
}
fn secret(token_file: &str) -> String {
    let text = std::fs::read_to_string(token_file).unwrap();
    assert!(text.ends_with('\n'));
    text.trim_end().to_owned()
}

#[test]
fn edit_session_open_never_names_the_session() {
    let root = Root::new();
    let data = root.workspace("data");
    let note = note(&root, &data, "N", "one");
    let (session, token) = begin(&root, &data, &note, 1);
    let argv = ["begin-edit", "--note", note.as_str(), "--base", "1"];
    let machine = run(&root, true, &data, &argv);
    let envelope = machine.envelope();
    assert_error(&envelope, "rejected", "edit_session_open");
    assert_eq!(envelope["error"]["details"]["note"], note.as_str());
    assert_eq!(envelope["error"]["details"]["base_version"], 1);
    assert_eq!(envelope["error"]["details"]["holder"], "caller");
    let default = run(&root, false, &data, &argv);
    assert_eq!(default.code, 1);
    let token = secret(&token);
    for text in [
        &machine.stdout,
        &machine.stderr,
        &default.stdout,
        &default.stderr,
    ] {
        assert!(!text.contains(&session), "the session ID leaked: {text}");
        assert!(!text.contains(&token), "the token leaked: {text}");
    }
}

#[test]
fn stale_save_recovery_recipe_from_the_readme_completes() {
    assert_eq!(
        recipe_commands(),
        [
            "result.session.markdown",
            "discard",
            "begin-edit",
            "update-buffer",
            "save"
        ],
        "the README recipe changed; update this test with it"
    );
    let root = Root::new();
    let data = root.workspace("data");
    let note = note(&root, &data, "N", "base\n");
    // Another writer saves version 2; my session was based on version 1.
    let other = begin(&root, &data, &note, 1);
    buffer(&root, &data, &other, "base\nother\n");
    assert_eq!(save(&root, &data, &other, "1")["outcome"], "Saved");
    let mine = begin(&root, &data, &note, 1);
    buffer(&root, &data, &mine, "base\nmine\n");
    let stale = save(&root, &data, &mine, "1");
    assert_eq!(stale["exit"], 3, "{stale}");
    // 1. Keep result.session.markdown.
    let kept = stale["result"]["session"]["markdown"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(kept, "base\nmine\n");
    // 2. discard --session <result.session.id> --generation <result.session.generation> --token-file OLD.token.
    let session = &stale["result"]["session"];
    let generation = session["generation"].to_string();
    let discarded = call(
        &root,
        &data,
        &[
            "discard",
            "--session",
            session["id"].as_str().unwrap(),
            "--generation",
            &generation,
            "--token-file",
            &mine.1,
        ],
    );
    assert_eq!(discarded["outcome"], "Discarded", "{discarded}");
    // 3. begin-edit --note N --base <conflict[0].version> --token-out NEW.token.
    let current = stale["conflict"][0]["version"].as_u64().unwrap();
    assert_eq!(current, 2);
    let new = begin(&root, &data, &note, current);
    // 4.-5. Reapply and update-buffer --generation 1 --token-file NEW.token.
    let merged = "base\nother\nmine\n";
    buffer(&root, &data, &new, merged);
    // 6. save --session NEW --generation 1 --token-file NEW.token.
    let saved = save(&root, &data, &new, "1");
    assert_eq!(saved["outcome"], "Saved", "{saved}");
    assert_eq!(saved["changed"][0]["version"], 3);
    let read = ok(&root, &data, &["read", "--id", &note]);
    assert_eq!(read["data"]["Note"]["markdown"], merged);
    assert_eq!(save(&root, &data, &mine, "1")["outcome"], "Closed");
}

#[test]
fn recipe_save_retry_replays_saved() {
    let root = Root::new();
    let data = root.workspace("data");
    let note = note(&root, &data, "N", "one");
    let session = begin(&root, &data, &note, 1);
    buffer(&root, &data, &session, "two");
    let first = save(&root, &data, &session, "1");
    assert_eq!(first["outcome"], "Saved");
    let again = save(&root, &data, &session, "1");
    assert_eq!(again["outcome"], "Saved");
    assert_eq!(again["operation"], first["operation"]);
    assert_eq!(again["retry"], "replay");
}

#[test]
fn tokens_arrive_once_by_result_or_owner_only_file_and_are_read_from_a_file_or_stdin() {
    let root = Root::new();
    let data = root.workspace("data");
    let note_a = note(&root, &data, "A", "alpha");
    // Without --token-out the token is result.token, once.
    let inline = ok(
        &root,
        &data,
        &["begin-edit", "--note", &note_a, "--base", "1"],
    );
    let token = inline["token"].as_str().unwrap().to_owned();
    assert!(
        token.starts_with("brnet1_") && token.len() == 71,
        "{inline}"
    );
    assert_eq!(inline["token_file"], Value::Null);
    assert_eq!(inline["holder"], "caller");
    assert_eq!(inline["generation"], 0);
    // The same token through stdin.
    let session = id(&inline);
    let piped = run_argv(
        &root,
        &argv(
            true,
            &data,
            &[
                "update-buffer",
                "--session",
                &session,
                "--generation",
                "1",
                "--text",
                "alpha, by stdin token",
                "--token-file",
                "-",
            ],
        ),
        Some(format!("{token}\n").as_bytes()),
    )
    .envelope();
    assert_eq!(piped["outcome"], "Updated", "{piped}");
    // Without the trailing newline too.
    let file = root.file("bare.token", token.as_bytes());
    let saved = call(
        &root,
        &data,
        &[
            "save",
            "--session",
            &session,
            "--generation",
            "1",
            "--token-file",
            &file,
        ],
    );
    assert_eq!(saved["outcome"], "Saved", "{saved}");
    // With --token-out the token goes to a new 0600 file.
    let note_b = note(&root, &data, "B", "beta");
    let (session, token_file) = begin_edit(&root, &data, &note_b, 1);
    assert_eq!(session["token_file"], token_file.as_str());
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(&token_file).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600, "{mode:o}");
    assert!(EditTokenLike::matches(&secret(&token_file)));
}

/// `^brnet1_[0-9a-f]{64}$`.
struct EditTokenLike;
impl EditTokenLike {
    fn matches(text: &str) -> bool {
        text.strip_prefix("brnet1_").is_some_and(|hex| {
            hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        })
    }
}

#[test]
fn token_refusals_name_no_token_and_write_nothing() {
    let root = Root::new();
    let data = root.workspace("data");
    let note = note(&root, &data, "N", "one");
    let (session, token) = begin(&root, &data, &note, 1);
    let write = |token_file: &str| {
        call(
            &root,
            &data,
            &[
                "update-buffer",
                "--session",
                &session,
                "--generation",
                "1",
                "--text",
                "x",
                "--token-file",
                token_file,
            ],
        )
    };
    // argv has no token form.
    let argv_token = call(
        &root,
        &data,
        &[
            "save",
            "--session",
            &session,
            "--generation",
            "0",
            "--token",
            &secret(&token),
        ],
    );
    assert_error(&argv_token, "usage", "unknown_option");
    assert!(!argv_token.to_string().contains(&secret(&token)));
    // Missing, relative, malformed.
    let missing = call(
        &root,
        &data,
        &["save", "--session", &session, "--generation", "0"],
    );
    assert_error(&missing, "usage", "missing_option");
    assert_eq!(missing["error"]["details"]["option"], "token-file");
    let relative = write("edit.token");
    assert_error(&relative, "usage", "invalid_value");
    assert_eq!(relative["error"]["details"]["option"], "token-file");
    let malformed_text = "brnet1_not-a-token-but-a-secret-looking-value";
    let malformed = write(&root.file("bad.token", malformed_text.as_bytes()));
    assert_error(&malformed, "usage", "invalid_input");
    assert_eq!(
        malformed["error"]["details"],
        json!({"option":"token-file","reason":"token_format"})
    );
    assert!(!malformed.to_string().contains(malformed_text));
    let unreadable = write(root.join("absent.token").to_str().unwrap());
    assert_eq!(unreadable["error"]["details"]["reason"], "unreadable");
    // A malformed token is refused before anything opens: no folder appears.
    let fresh = root.join("never-created");
    let before_open = call(
        &root,
        &fresh,
        &[
            "discard",
            "--session",
            "s",
            "--generation",
            "0",
            "--token-file",
            &root.file("bad2.token", b"nope"),
        ],
    );
    assert_eq!(before_open["error"]["details"]["reason"], "token_format");
    assert!(!fresh.exists());
    // stdin can feed one input only.
    let reused = call(
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
            "-",
        ],
    );
    assert_error(&reused, "usage", "invalid_input");
    assert_eq!(reused["error"]["details"]["reason"], "stdin_reused");
    // Another session's token: mismatch, nothing written.
    let other_note = self::note(&root, &data, "O", "other");
    let (_, other_token) = begin(&root, &data, &other_note, 1);
    let mismatch = write(&other_token);
    assert_error(&mismatch, "rejected", "session_token_invalid");
    assert_eq!(mismatch["error"]["details"], json!({"reason":"mismatch"}));
    assert_eq!(mismatch["exit"], 6);
    let sessions = ok(&root, &data, &["recovery"]);
    let mine = sessions
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == session.as_str())
        .unwrap();
    assert_eq!(mine["generation"], 0);
    assert_eq!(mine["holder"], "caller");
    assert!(mine.get("token").is_none());
    // --token-out refusals happen before opening and leave no session.
    let note2 = self::note(&root, &data, "T", "two");
    let taken = root.file("taken.token", b"");
    let exists = call(
        &root,
        &data,
        &[
            "begin-edit",
            "--note",
            &note2,
            "--base",
            "1",
            "--token-out",
            &taken,
        ],
    );
    assert_error(&exists, "rejected", "destination_exists");
    assert_eq!(exists["error"]["details"], json!({"option":"token-out"}));
    let orphan = root.join("no-such-folder/edit.token");
    let parent = call(
        &root,
        &data,
        &[
            "begin-edit",
            "--note",
            &note2,
            "--base",
            "1",
            "--token-out",
            orphan.to_str().unwrap(),
        ],
    );
    assert_error(&parent, "rejected", "invalid_request");
    assert_eq!(
        parent["error"]["details"],
        json!({"option":"token-out","reason":"parent_missing"})
    );
    let open: Vec<_> = ok(&root, &data, &["recovery"])
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["note"].clone())
        .collect();
    assert!(!open.contains(&json!(note2)), "{open:?}");
}

#[test]
fn a_restored_copy_holds_its_sessions_for_the_desktop() {
    let root = Root::new();
    let data = root.workspace("data");
    let note = note(&root, &data, "N", "one");
    let (session, token) = begin(&root, &data, &note, 1);
    let backup = root.join("backup.sqlite3");
    ok(
        &root,
        &data,
        &["backup", "--destination", backup.to_str().unwrap()],
    );
    let copy = root.join("copy");
    let restored = call(
        &root,
        &copy,
        &["restore", "--source", backup.to_str().unwrap()],
    );
    assert_eq!(
        restored["result"]["workspace_purpose"], "owner",
        "{restored}"
    );
    let listed = ok(&root, &copy, &["recovery"]);
    assert_eq!(listed[0]["id"], session.as_str());
    assert_eq!(listed[0]["holder"], "desktop");
    let desktop = call(
        &root,
        &copy,
        &[
            "save",
            "--session",
            &session,
            "--generation",
            "0",
            "--token-file",
            &token,
        ],
    );
    assert_error(&desktop, "rejected", "session_token_invalid");
    assert_eq!(desktop["error"]["details"], json!({"reason":"desktop"}));
    assert_eq!(desktop["authority"]["workspace_purpose"], "owner");
    let open = call(
        &root,
        &copy,
        &["begin-edit", "--note", &note, "--base", "1"],
    );
    assert_eq!(open["error"]["details"]["holder"], "desktop");
    // The original still takes its token.
    assert_eq!(
        call(
            &root,
            &data,
            &[
                "discard",
                "--session",
                &session,
                "--generation",
                "0",
                "--token-file",
                &token,
            ],
        )["outcome"],
        "Discarded"
    );
}

#[test]
fn reclaim_rotates_the_token_in_test_workspaces_only() {
    let root = Root::new();
    let data = root.workspace("data");
    let note = note(&root, &data, "N", "one");
    let (session, old) = begin(&root, &data, &note, 1);
    let new = root.join("new.token");
    let reclaimed = call(
        &root,
        &data,
        &[
            "reclaim-edit",
            "--session",
            &session,
            "--token-out",
            new.to_str().unwrap(),
        ],
    );
    assert_eq!(reclaimed["status"], "ok", "{reclaimed}");
    assert_eq!(reclaimed["retry"], "converge");
    assert_eq!(reclaimed["result"]["id"], session.as_str());
    assert_eq!(reclaimed["result"]["holder"], "caller");
    let new = new.to_str().unwrap().to_owned();
    assert_ne!(secret(&new), secret(&old));
    let stale = save(&root, &data, &(session.clone(), old), "0");
    assert_error(&stale, "rejected", "session_token_invalid");
    assert_eq!(stale["error"]["details"]["reason"], "mismatch");
    assert_eq!(
        save(&root, &data, &(session.clone(), new.clone()), "0")["outcome"],
        "Saved"
    );
    // A closed session cannot be reclaimed.
    let closed = call(&root, &data, &["reclaim-edit", "--session", &session]);
    assert_error(&closed, "rejected", "invalid_request");
    assert_eq!(closed["error"]["details"]["reason"], "edit_closed");
    // An owner workspace (plain init) refuses, and writes nothing.
    let owner = root.join("owner");
    ok(&root, &owner, &["init"]);
    let owned = self::note(&root, &owner, "N", "one");
    let (session, _token) = begin(&root, &owner, &owned, 1);
    let refused = call(&root, &owner, &["reclaim-edit", "--session", &session]);
    assert_error(&refused, "rejected", "test_workspace_required");
    assert_eq!(
        refused["error"]["details"],
        json!({"workspace_purpose":"owner"})
    );
    assert_eq!(refused["authority"]["workspace_purpose"], "owner");
}

/// T-leak: the token appears only in the `begin-edit` result or its
/// `--token-out` file: never in another envelope, stderr, export, backup,
/// the database or its WAL.
#[test]
fn the_token_never_leaks_into_outputs_exports_backups_or_the_database() {
    let root = Root::new();
    let data = root.workspace("data");
    let note = note(&root, &data, "N", "one");
    let inline = run(
        &root,
        true,
        &data,
        &["begin-edit", "--note", &note, "--base", "1"],
    );
    let inline_envelope = inline.envelope();
    let inline_token = inline_envelope["result"]["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let inline_session = id(&inline_envelope["result"]);
    let other = self::note(&root, &data, "M", "two");
    let (session, file) = begin(&root, &data, &other, 1);
    let token = secret(&file);
    let mut outputs: Vec<String> = vec![];
    fn record(outputs: &mut Vec<String>, run: Run) {
        outputs.push(run.stdout);
        outputs.push(run.stderr);
    }
    let text = root.file("text.md", b"two, edited");
    for argv in [
        vec![
            "update-buffer",
            "--session",
            &session,
            "--generation",
            "1",
            "--text-file",
            &text,
            "--token-file",
            &file,
        ],
        vec!["recovery"],
        vec!["recovery", "--limit", "5"],
        vec![
            "update-buffer",
            "--session",
            &inline_session,
            "--generation",
            "1",
            "--text",
            "x",
            "--token-file",
            &file,
        ],
        vec![
            "save",
            "--session",
            &session,
            "--generation",
            "1",
            "--token-file",
            &file,
        ],
        vec!["history"],
        vec!["history", "--limit", "10"],
        vec!["begin-edit", "--note", &note, "--base", "1"],
    ] {
        record(&mut outputs, run(&root, true, &data, &argv));
        record(&mut outputs, run(&root, false, &data, &argv));
    }
    let operation = outputs
        .iter()
        .find_map(|out| {
            let value: Value = serde_json::from_str(out.trim()).ok()?;
            (value["outcome"] == "Saved").then(|| value["operation"].as_str().unwrap().to_owned())
        })
        .unwrap();
    record(
        &mut outputs,
        run(&root, true, &data, &["operation", "--id", &operation]),
    );
    let export = root.join("export");
    let backup = root.join("backup.sqlite3");
    record(
        &mut outputs,
        run(
            &root,
            true,
            &data,
            &["export", "--destination", export.to_str().unwrap()],
        ),
    );
    record(
        &mut outputs,
        run(
            &root,
            true,
            &data,
            &["backup", "--destination", backup.to_str().unwrap()],
        ),
    );
    let mut bytes: Vec<Vec<u8>> = outputs.into_iter().map(String::into_bytes).collect();
    let mut stack = vec![export.clone(), data.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                bytes.push(std::fs::read(&path).unwrap());
            }
        }
    }
    bytes.push(std::fs::read(&backup).unwrap());
    for secret in [&token, &inline_token] {
        for (index, blob) in bytes.iter().enumerate() {
            assert!(
                !blob
                    .windows(secret.len())
                    .any(|window| window == secret.as_bytes()),
                "token found in output {index}"
            );
        }
    }
}
