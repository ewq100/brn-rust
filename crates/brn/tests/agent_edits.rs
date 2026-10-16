//! Process checks of owner decision B3 part 1: stale edits stay conflicts,
//! `edit_session_open` never names the open session, and the README's
//! stale-save recipe completes as written.
mod support;
use serde_json::Value;
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

fn begin(root: &Root, data: &Path, note: &str, base: u64) -> Value {
    ok(
        root,
        data,
        &["begin-edit", "--note", note, "--base", &base.to_string()],
    )
}
fn buffer(root: &Root, data: &Path, session: &str, text: &str) {
    let file = root.file(&format!("{session}.md"), text.as_bytes());
    let argv = [
        "update-buffer",
        "--session",
        session,
        "--generation",
        "1",
        "--text-file",
        &file,
    ];
    assert_eq!(call(root, data, &argv)["outcome"], "Updated");
}
fn save(root: &Root, data: &Path, session: &str, generation: &str) -> Value {
    call(
        root,
        data,
        &["save", "--session", session, "--generation", generation],
    )
}

#[test]
fn edit_session_open_never_names_the_session() {
    let root = Root::new();
    let data = root.join("data");
    let note = note(&root, &data, "N", "one");
    let session = id(&begin(&root, &data, &note, 1));
    let argv = ["begin-edit", "--note", note.as_str(), "--base", "1"];
    let machine = run(&root, true, &data, &argv);
    let envelope = machine.envelope();
    assert_error(&envelope, "rejected", "edit_session_open");
    assert_eq!(envelope["error"]["details"]["note"], note.as_str());
    assert_eq!(envelope["error"]["details"]["base_version"], 1);
    let default = run(&root, false, &data, &argv);
    assert_eq!(default.code, 1);
    for text in [
        &machine.stdout,
        &machine.stderr,
        &default.stdout,
        &default.stderr,
    ] {
        assert!(!text.contains(&session), "the session ID leaked: {text}");
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
    let data = root.join("data");
    let note = note(&root, &data, "N", "base\n");
    // Another writer saves version 2; my session was based on version 1.
    let other = id(&begin(&root, &data, &note, 1));
    buffer(&root, &data, &other, "base\nother\n");
    assert_eq!(save(&root, &data, &other, "1")["outcome"], "Saved");
    let mine = id(&begin(&root, &data, &note, 1));
    buffer(&root, &data, &mine, "base\nmine\n");
    let stale = save(&root, &data, &mine, "1");
    assert_eq!(stale["exit"], 3, "{stale}");
    // 1. Keep result.session.markdown.
    let kept = stale["result"]["session"]["markdown"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(kept, "base\nmine\n");
    // 2. discard --session <result.session.id> --generation <result.session.generation>.
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
        ],
    );
    assert_eq!(discarded["outcome"], "Discarded", "{discarded}");
    // 3. begin-edit --note N --base <conflict[0].version>.
    let current = stale["conflict"][0]["version"].as_u64().unwrap();
    assert_eq!(current, 2);
    let new = id(&begin(&root, &data, &note, current));
    // 4.-5. Reapply and update-buffer --generation 1.
    let merged = "base\nother\nmine\n";
    buffer(&root, &data, &new, merged);
    // 6. save --session NEW --generation 1.
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
    let data = root.join("data");
    let note = note(&root, &data, "N", "one");
    let session = id(&begin(&root, &data, &note, 1));
    buffer(&root, &data, &session, "two");
    let first = save(&root, &data, &session, "1");
    assert_eq!(first["outcome"], "Saved");
    let again = save(&root, &data, &session, "1");
    assert_eq!(again["outcome"], "Saved");
    assert_eq!(again["operation"], first["operation"]);
    assert_eq!(again["retry"], "replay");
}
