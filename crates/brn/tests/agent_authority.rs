//! Round 3, B1 phase 1: workspace purpose (`init --purpose test`, creation
//! only), the caller's actor on receipts, and the `authority` every
//! envelope reports (`enforced: false`). Synthetic data only.
mod support;
use serde_json::{json, Value};
use support::*;

fn authority(purpose: Option<&str>, actor: Option<&str>) -> Value {
    json!({
        "workspace_purpose": purpose,
        "cli_authority": purpose.map(|_| "full"),
        "enforced": false,
        "actor": actor,
    })
}

#[test]
fn every_envelope_reports_authority_and_nulls_where_nothing_opened() {
    let root = Root::new();
    let test = root.workspace("test");
    let owner = root.join("owner");
    let init = call(&root, &owner, &["init"]);
    assert_eq!(init["authority"], authority(Some("owner"), None));
    assert_eq!(
        call(&root, &test, &["notes"])["authority"],
        authority(Some("test"), None)
    );
    // Opened, then refused: the purpose is known.
    let missing = call(&root, &test, &["read", "--id", "no-such-record"]);
    assert_error(&missing, "rejected", "not_found");
    assert_eq!(missing["authority"], authority(Some("test"), None));
    // Nothing opened: usage errors, missing_workspace, restore.
    let usage = call(&root, &test, &["notes", "--limit", "zero"]);
    assert_error(&usage, "usage", "invalid_value");
    assert_eq!(usage["authority"], authority(None, None));
    let absent = call(&root, &root.join("absent"), &["notes"]);
    assert_error(&absent, "rejected", "missing_workspace");
    assert_eq!(absent["authority"], authority(None, None));
    let refused = run_argv(&root, &["--json", "notes"], None).envelope();
    assert_eq!(refused["authority"], authority(None, None));
    // The description documents it.
    let help = run_argv(&root, &["--json", "--help"], None);
    let description: Value = serde_json::from_str(&help.stdout).unwrap();
    let described = &description["authority"];
    assert_eq!(described["purposes"], json!(["owner", "test"]));
    assert_eq!(described["default"], "owner");
    assert_eq!(described["created_by"], "init --purpose test");
    assert_eq!(described["changed_by"], "desktop only");
    assert_eq!(described["restore_yields"], "owner");
    assert_eq!(
        described["cli_authority"],
        json!({"owner": "full", "test": "full"})
    );
    assert_eq!(described["phase2_owner"], "proposals");
    assert_eq!(described["enforced"], false);
    assert_eq!(described["actor"]["run_receipts"], "none");
    assert_eq!(
        described["holders"],
        json!({"caller": "token required", "desktop": "owner in the desktop app only"})
    );
}

#[test]
fn purpose_test_is_creation_only_and_test_is_the_only_value() {
    let root = Root::new();
    let data = root.join("data");
    let created = call(&root, &data, &["init", "--purpose", "test"]);
    assert_eq!(created["status"], "ok", "{created}");
    assert_eq!(created["retry"], "refuse");
    assert_eq!(created["authority"]["workspace_purpose"], "test");
    assert_eq!(created["result"]["settings"]["auth_file"], Value::Null);
    let before = ok(&root, &data, &["history"]);
    let again = call(&root, &data, &["init", "--purpose", "test"]);
    assert_error(&again, "rejected", "invalid_request");
    assert_eq!(
        again["error"]["details"],
        json!({"reason": "workspace_exists"})
    );
    assert_eq!(ok(&root, &data, &["history"]), before);
    let default = run(&root, false, &data, &["init", "--purpose", "test"]);
    assert_eq!(default.code, 1);
    // An owner workspace cannot become a test workspace from the CLI.
    let owner = root.join("owner");
    ok(&root, &owner, &["init"]);
    let promoted = call(&root, &owner, &["init", "--purpose", "test"]);
    assert_eq!(promoted["error"]["details"]["reason"], "workspace_exists");
    assert_eq!(
        call(&root, &owner, &["notes"])["authority"]["workspace_purpose"],
        "owner"
    );
    // `owner` is not a value: plain init is the owner form.
    let fresh = root.join("fresh");
    let named = call(&root, &fresh, &["init", "--purpose", "owner"]);
    assert_error(&named, "usage", "invalid_value");
    assert_eq!(named["error"]["details"]["option"], "purpose");
    assert!(!fresh.exists());
    // A non-empty folder without a workspace stays unsafe_directory.
    let foreign = root.join("foreign");
    std::fs::create_dir(&foreign).unwrap();
    std::fs::write(foreign.join("file.txt"), "x").unwrap();
    assert_error(
        &call(&root, &foreign, &["init", "--purpose", "test"]),
        "rejected",
        "unsafe_directory",
    );
    // An empty folder is a new folder.
    let empty = root.join("empty");
    std::fs::create_dir(&empty).unwrap();
    assert_eq!(
        call(&root, &empty, &["init", "--purpose", "test"])["authority"]["workspace_purpose"],
        "test"
    );
}

#[test]
fn the_actor_option_wins_over_the_variable_and_bad_values_are_refused_before_opening() {
    let mut root = Root::new();
    let data = root.workspace("data");
    root.env = vec![("BRN_ACTOR".into(), "from-env".into())];
    let by_env = call(
        &root,
        &data,
        &["note-create", "--title", "E", "--text", "e"],
    );
    assert_eq!(by_env["authority"]["actor"], "from-env");
    let by_option = call(
        &root,
        &data,
        &[
            "note-create",
            "--title",
            "O",
            "--text",
            "o",
            "--actor",
            "from-option",
        ],
    );
    assert_eq!(by_option["authority"]["actor"], "from-option");
    let history = ok(&root, &data, &["history", "--limit", "100"]);
    let actor_of = |envelope: &Value| {
        let operation = envelope["operation"].as_str().unwrap();
        history["receipts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["operation"] == operation)
            .unwrap()["actor"]
            .clone()
    };
    assert_eq!(actor_of(&by_env), "from-env");
    assert_eq!(actor_of(&by_option), "from-option");
    // A bad variable is refused, naming its source, before anything opens.
    root.env = vec![("BRN_ACTOR".into(), "has space".into())];
    let fresh = root.join("fresh");
    let bad_env = call(&root, &fresh, &["notes"]);
    assert_error(&bad_env, "usage", "invalid_value");
    assert_eq!(bad_env["error"]["details"]["option"], "actor");
    assert_eq!(bad_env["error"]["details"]["source"], "env");
    assert!(!bad_env.to_string().contains("has space"));
    assert!(!fresh.exists());
    root.env = vec![];
    let long = "a".repeat(65);
    let bad_option = call(&root, &fresh, &["--actor", &long, "notes"]);
    assert_error(&bad_option, "usage", "invalid_value");
    assert_eq!(bad_option["error"]["details"]["option"], "actor");
    assert!(bad_option["error"]["details"].get("source").is_none());
    assert!(!fresh.exists());
}

#[test]
fn actors_show_in_history_and_operation_and_a_keyed_retry_keeps_the_first() {
    let root = Root::new();
    let data = root.workspace("data");
    let argv = |actor: &'static str| {
        vec![
            "--actor",
            actor,
            "note-create",
            "--title",
            "Plan",
            "--text",
            "same",
            "--key",
            "plan-1",
        ]
    };
    let first = call(&root, &data, &argv("agent-7"));
    assert_eq!(first["status"], "ok", "{first}");
    let retry = call(&root, &data, &argv("agent-8"));
    assert_eq!(retry["operation"], first["operation"]);
    assert_eq!(retry["retry"], "replay");
    assert_eq!(retry["authority"]["actor"], "agent-8");
    let operation = first["operation"].as_str().unwrap();
    let info = ok(&root, &data, &["operation", "--id", operation]);
    assert_eq!(info["actor"], "agent-7");
    assert_eq!(
        ok(
            &root,
            &data,
            &["operation", "--key", "plan-1", "--command", "note-create"]
        )["actor"],
        "agent-7"
    );
    let page = ok(&root, &data, &["history", "--limit", "100"]);
    let receipts = page["receipts"].as_array().unwrap();
    let mine: Vec<_> = receipts
        .iter()
        .filter(|r| r["operation"] == operation)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0]["actor"], "agent-7");
    // Receipts committed without an actor show null (the bootstrap ones).
    assert!(receipts.iter().any(|r| r["actor"].is_null()));
    // An unapplied lookup has no actor.
    let unnamed = call(
        &root,
        &data,
        &["note-create", "--title", "N", "--text", "n"],
    );
    let info = ok(
        &root,
        &data,
        &["operation", "--id", unnamed["operation"].as_str().unwrap()],
    );
    assert_eq!(info["actor"], Value::Null);
}

#[test]
fn a_restored_copy_of_a_test_workspace_is_an_owner_workspace() {
    let root = Root::new();
    let data = root.workspace("data");
    note(&root, &data, "N", "kept");
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
    assert_eq!(restored["status"], "ok", "{restored}");
    assert_eq!(restored["result"]["workspace_purpose"], "owner");
    assert_eq!(restored["authority"], authority(None, None));
    // reads only: the restored copy is an owner workspace.
    let notes = call(&root, &copy, &["notes"]);
    assert_eq!(notes["authority"]["workspace_purpose"], "owner");
    assert_eq!(notes["result"].as_array().unwrap().len(), 1);
    // The source stays a test workspace.
    assert_eq!(
        call(&root, &data, &["notes"])["authority"]["workspace_purpose"],
        "test"
    );
}
