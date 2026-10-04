use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    #[cfg(target_os = "macos")]
    input: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        Self {
            data,
            vault,
            #[cfg(target_os = "macos")]
            input: owner.path().join("input.json"),
            _owner: owner,
        }
    }
    fn process(&self, args: &[&str], json: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_brn"));
        command.args(args).arg("--data-dir").arg(&self.data);
        if json {
            command.arg("--json");
        }
        command.output().unwrap()
    }
    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = self.process(args, true);
        let envelope =
            serde_json::from_slice(&output.stdout).unwrap_or_else(|_| panic!("{output:?}"));
        (output.status.code().unwrap(), envelope)
    }
    #[cfg(target_os = "macos")]
    fn write(&self, sub: &str, value: Value) -> Value {
        fs::write(&self.input, serde_json::to_vec(&value).unwrap()).unwrap();
        ok(self.run(&[
            "proposals",
            sub,
            "--file",
            self.input.to_str().unwrap(),
            "--vault",
            self.vault.to_str().unwrap(),
        ]))
    }
}
fn ok((code, envelope): (i32, Value)) -> Value {
    assert_eq!(code, 0, "{envelope}");
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["ok"], true);
    envelope["data"].clone()
}

#[test]
fn invalid_activity_pagination_refuses_before_creating_storage() {
    let nil = Uuid::nil().to_string();
    for args in [
        vec!["activity"],
        vec!["activity", "show"],
        vec!["activity", "list", "extra"],
        vec!["activity", "list", "--limit", "0"],
        vec!["activity", "list", "--limit", "101"],
        vec!["activity", "list", "--limit", "not-a-number"],
        vec!["activity", "list", "--limit", "-1"],
        vec!["activity", "list", "--limit", "999999999999999999999999"],
        vec!["activity", "list", "--before", "not-a-uuid"],
        vec!["activity", "list", "--before", &nil],
        vec!["activity", "list", "--limit", "1", "--limit", "2"],
        vec!["activity", "list", "--unknown"],
    ] {
        let f = Fixture::new();
        let result = f.run(&args);
        assert_eq!(result.0, 2, "{}", result.1);
        assert_eq!(result.1["ok"], false);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
    }
}

#[test]
fn empty_activity_has_a_typed_page_and_clear_human_output_without_a_vault() {
    let f = Fixture::new();
    let result = f.run(&["activity", "list"]);
    assert_eq!(result.1["command"], "activity.list");
    assert_eq!(ok(result), json!({"entries":[],"next_before":null}));
    let human = f.process(&["activity", "list", "--limit", "100"], false);
    assert!(human.status.success(), "{human:?}");
    assert_eq!(
        String::from_utf8(human.stdout).unwrap(),
        "No approved durable changes.\n"
    );
    assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
}

#[cfg(target_os = "macos")]
#[test]
fn approved_activity_pages_remain_stable_and_exclude_review_bodies_and_comments() {
    let f = Fixture::new();
    let group = Uuid::new_v4();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let first_operation = Uuid::new_v4();
    let second_operation = Uuid::new_v4();
    let title = "Approved 日本語\n\u{1b}[31m";
    let body = "\u{feff}PRIVATE_NOTE_BODY_FIXTURE 日本語\r\n";
    f.write(
        "create",
        json!({"id":first,"group_id":group,"session_id":null,"title":title,
        "changes":[{"kind":"create","path":"first.md","text":body}],"sources":[]}),
    );
    f.write(
        "comment",
        json!({"expected":{"id":first,"version":1},"comment":{
        "id":Uuid::new_v4(),"text":"PRIVATE_REVIEW_COMMENT_FIXTURE","target":{"kind":"proposal"}}}),
    );
    ok(f.run(&[
        "proposals",
        "approve",
        &first.to_string(),
        "--review-version",
        "2",
        "--operation",
        &first_operation.to_string(),
    ]));
    f.write("create", json!({"id":second,"group_id":group,"session_id":null,"title":"Second approval",
        "changes":[{"kind":"create","path":"second.md","text":"Another private body fixture"}],"sources":[]}));
    ok(f.run(&[
        "proposals",
        "approve",
        &second.to_string(),
        "--review-version",
        "1",
        "--operation",
        &second_operation.to_string(),
    ]));
    let draft = Uuid::new_v4();
    f.write(
        "create",
        json!({"id":draft,"group_id":null,"session_id":null,"title":"Unapproved draft",
        "changes":[{"kind":"create","path":"draft.md","text":"Unapproved"}],"sources":[]}),
    );
    let rejected = Uuid::new_v4();
    f.write(
        "create",
        json!({"id":rejected,"group_id":null,"session_id":null,"title":"Rejected draft",
        "changes":[{"kind":"create","path":"rejected.md","text":"Rejected"}],"sources":[]}),
    );
    ok(f.run(&[
        "proposals",
        "reject",
        &rejected.to_string(),
        "--review-version",
        "1",
    ]));

    let full = ok(f.run(&["activity", "list"]));
    assert_eq!(full["entries"].as_array().unwrap().len(), 2);
    assert!(full["next_before"].is_null());
    let entries = full["entries"].as_array().unwrap();
    let ordered = |entry: &Value| {
        (
            entry["approved_at_ms"].as_u64().unwrap(),
            entry["operation_id"].as_str().unwrap().to_owned(),
        )
    };
    assert!(ordered(&entries[0]) >= ordered(&entries[1]));
    for entry in entries {
        let operation = entry["operation_id"].as_str().unwrap();
        assert!([first_operation.to_string(), second_operation.to_string()]
            .contains(&operation.to_owned()));
        assert_eq!(entry["group_id"], group.to_string());
        assert!(entry["session_id"].is_null());
        assert!(!entry["summary"].as_str().unwrap().is_empty());
        assert_eq!(entry["changes"].as_array().unwrap().len(), 1);
        assert_eq!(entry["changes"][0]["kind"], "created");
        assert!(entry.get("comments").is_none());
        assert!(entry.get("prepared").is_none());
        assert!(entry.get("draft").is_none());
        assert!(entry["changes"][0].get("text").is_none());
        if operation == first_operation.to_string() {
            assert_eq!(entry["proposal_id"], first.to_string());
            assert_eq!(entry["title"], title);
            assert_eq!(entry["changes"][0]["path"], "first.md");
        }
    }
    let one = ok(f.run(&["activity", "list", "--limit", "1"]));
    assert_eq!(one["entries"], json!([entries[0]]));
    assert_eq!(one["next_before"], entries[0]["operation_id"]);
    let cursor = one["next_before"].as_str().unwrap();
    let older = ok(f.run(&["activity", "list", "--limit", "1", "--before", cursor]));
    assert_eq!(older["entries"], json!([entries[1]]));
    assert!(older["next_before"].is_null());
    let end = ok(f.run(&[
        "activity",
        "list",
        "--before",
        entries[1]["operation_id"].as_str().unwrap(),
    ]));
    assert_eq!(end, json!({"entries":[],"next_before":null}));
    let stale = f.run(&["activity", "list", "--before", &Uuid::new_v4().to_string()]);
    assert_eq!(stale.0, 1);

    let review_before = ok(f.run(&["proposals", "show", &first.to_string()]));
    fs::write(f.vault.join("first.md"), "Later external bytes\r\n").unwrap();
    assert_eq!(ok(f.run(&["activity", "list", "--limit", "100"])), full);
    assert_eq!(
        ok(f.run(&["proposals", "show", &first.to_string()])),
        review_before
    );
    assert_eq!(
        fs::read(f.vault.join("first.md")).unwrap(),
        b"Later external bytes\r\n"
    );
    assert!(!f.vault.join("draft.md").exists());
    assert!(!f.vault.join("rejected.md").exists());
    let human = f.process(&["activity", "list"], false);
    assert!(human.status.success(), "{human:?}");
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("Approved at admission:"));
    assert!(human.contains("Approved 日本語\\n\\u{1b}[31m"));
    assert!(human.contains("Created: first.md"));
    assert!(human.contains(&format!("Operation: {first_operation}")));
    assert!(human.contains(&format!("Proposal: {first}")));
    assert!(human.contains(&format!("Group: {group}")));
    assert!(!human.contains('\u{1b}'));
    for value in [&human, &serde_json::to_string(&full).unwrap()] {
        assert!(!value.contains("PRIVATE_NOTE_BODY_FIXTURE"));
        assert!(!value.contains("PRIVATE_REVIEW_COMMENT_FIXTURE"));
        assert!(!value.contains("Another private body fixture"));
    }
}
