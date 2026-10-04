//! Exact Action application through real CLI processes; no storage seeding.
//! Ordinary approval receipts currently require the macOS file adapter.
#![cfg(target_os = "macos")]
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    credentials: PathBuf,
    vault: PathBuf,
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
            credentials: owner.path().join("credentials"),
            vault,
            input: owner.path().join("input.json"),
            _owner: owner,
        }
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(args)
            .args(["--json", "--data-dir"])
            .arg(&self.data)
            .arg("--credentials-dir")
            .arg(&self.credentials)
            .output()
            .unwrap();
        let envelope: Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("missing envelope: {output:?}"));
        assert_eq!(envelope["schema_version"], 1);
        (output.status.code().unwrap(), envelope)
    }

    fn write(&self, sub: &str, input: &Value, bind: bool) -> (i32, Value) {
        fs::write(&self.input, serde_json::to_vec(input).unwrap()).unwrap();
        let mut args = vec!["proposals", sub, "--file", self.input.to_str().unwrap()];
        if bind {
            args.extend(["--vault", self.vault.to_str().unwrap()]);
        }
        self.run(&args)
    }

    fn show(&self, id: Uuid) -> Value {
        ok(self.run(&["actions", "show", &id.to_string()]))
    }

    fn approve(&self, id: Uuid, version: u64, operation: Uuid) -> (i32, Value) {
        self.run(&[
            "proposals",
            "approve",
            &id.to_string(),
            "--review-version",
            &version.to_string(),
            "--operation",
            &operation.to_string(),
        ])
    }

    fn quiet(&self) {
        let status = ok(self.run(&["status"]));
        assert_eq!(status["model_installed"], false);
        assert_eq!(
            ok(self.run(&["conversations", "list"]))["conversations"],
            json!([])
        );
        assert_eq!(fs::read_dir(&self.credentials).unwrap().count(), 0);
    }
}

fn ok(result: (i32, Value)) -> Value {
    assert_eq!(result.0, 0, "{}", result.1);
    assert_eq!(result.1["ok"], true);
    result.1["data"].clone()
}

fn refused(result: (i32, Value), code: &str) {
    assert_eq!(result.0, 1, "{}", result.1);
    assert_eq!(result.1["ok"], false);
    assert_eq!(result.1["error"]["code"], code);
}

fn data(title: &str) -> Value {
    json!({"title":title, "description":"\u{feff}Exact \"quotes\" 日本語 õ\r\n\t\u{0001}\u{001b}",
        "state":"waiting", "owner":"  Zoë\t\u{0000}  ", "related_person":null,
        "related_project":null,"sources":[],"thread":null,"due_on":"2028-02-29",
        "follow_up_on":"2028-03-01","dependencies":[],"parent":null,"follows_up":null,
        "priority":"high"})
}

fn draft(id: Uuid, group: Option<Uuid>, changes: Value) -> Value {
    json!({"id":id,"group_id":group,"session_id":null,"title":"Exact Action review λ",
        "changes":[],"sources":[],"action_changes":changes})
}

fn create(id: Uuid, data: Value) -> Value {
    json!({"kind":"create","id":id,"data":data})
}

#[test]
fn vaultless_review_approval_replace_and_replay_preserve_exact_records() {
    let f = Fixture::new();
    let id = Uuid::new_v4();
    let proposal = Uuid::new_v4();
    let initial = data("  Waiting \"λ\"\r\n\u{0007}  ");
    let input = draft(proposal, None, json!([create(id, initial.clone())]));
    let created = ok(f.write("create", &input, false));
    assert!(created["draft"]["vault"].is_null());
    assert_eq!(created["draft"]["action_changes"][0]["data"], initial);
    assert_eq!(ok(f.run(&["actions", "list"]))["entries"], json!([]));
    let commented = ok(f.write("comment", &json!({"expected":{"id":proposal,"version":1},
        "comment":{"id":Uuid::new_v4(),"text":"Keep exact owner\r\nλ","target":{"kind":"proposal"}}}), false));
    assert_eq!(commented["version"], 2);
    let mut reviewed = initial.clone();
    reviewed["description"] = json!("\u{feff}Reviewed 日本語\r\n\t\u{001b}");
    let edited = ok(f.write(
        "edit",
        &json!({"expected":{"id":proposal,"version":2},
        "title":"Reviewed exact λ","texts":[],"action_data":[reviewed.clone()]}),
        false,
    ));
    assert_eq!(edited["version"], 3);
    assert_eq!(edited["comments"], commented["comments"]);
    assert_eq!(ok(f.write("create", &input, false)), edited);
    let mut conflicting = input.clone();
    conflicting["action_changes"][0]["data"]["owner"] = json!("different creation");
    refused(f.write("create", &conflicting, false), "OPERATION_CONFLICT");
    refused(f.approve(proposal, 2, Uuid::new_v4()), "CONTEXT_STALE");
    assert_eq!(ok(f.run(&["actions", "list"]))["entries"], json!([]));
    assert_eq!(ok(f.run(&["proposals", "applies"])), json!([]));
    assert_eq!(ok(f.run(&["activity", "list"]))["entries"], json!([]));

    let operation = Uuid::new_v4();
    let receipt = ok(f.approve(proposal, 3, operation));
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(receipt["approved_version"], 3);
    let history = ok(f.run(&["activity", "list"]));
    assert_eq!(history["entries"].as_array().unwrap().len(), 1);
    assert_eq!(history["entries"][0]["operation_id"], json!(operation));
    assert_eq!(history["entries"][0]["summary"], "Created 1 action.");
    let before = f.show(id); // Every invocation starts and joins a new AppWorker.
    assert_eq!(before["data"], reviewed);
    assert_eq!(before["origin"]["data"], reviewed);
    assert_eq!(
        before["origin"]["proposal"],
        json!({"id":proposal,"version":3})
    );
    assert_eq!(
        before["waiting_since_ms"],
        before["origin"]["created_at_ms"]
    );
    assert!(before["waiting_since_ms"].is_u64());
    assert_eq!(before["version"], 1);
    assert_eq!(ok(f.approve(proposal, 3, operation)), receipt);
    assert_eq!(
        ok(f.run(&["proposals", "reconcile", &operation.to_string()])),
        receipt
    );
    assert_eq!(f.show(id), before);
    let applied = ok(f.run(&["proposals", "show", &proposal.to_string()]));
    assert_eq!(applied["comments"], json!([]));
    assert_eq!(applied["draft"]["action_changes"][0]["data"], reviewed);

    let replacement = Uuid::new_v4();
    let mut after = reviewed.clone();
    after["title"] = json!("Replacement \"õ\"\r\n");
    after["priority"] = json!("low");
    let replacement_input = draft(
        replacement,
        None,
        json!([{"kind":"replace","before":before,"data":after}]),
    );
    let replacing = ok(f.write("create", &replacement_input, false));
    assert_eq!(replacing["draft"]["action_changes"][0]["before"], before);
    assert_eq!(f.show(id), before);
    let replace_operation = Uuid::new_v4();
    let replaced_receipt = ok(f.approve(replacement, 1, replace_operation));
    let history = ok(f.run(&["activity", "list"]));
    let entries = history["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry["operation_id"] == json!(replace_operation))
            .unwrap()["summary"],
        "Updated 1 action."
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry["operation_id"] == json!(operation))
            .unwrap()["summary"],
        "Created 1 action."
    );
    let current = f.show(id);
    assert_eq!(current["version"], 2);
    assert_eq!(current["data"], after);
    assert_eq!(current["origin"], before["origin"]);
    assert_eq!(current["waiting_since_ms"], before["waiting_since_ms"]);
    assert_eq!(
        ok(f.approve(replacement, 1, replace_operation)),
        replaced_receipt
    );
    assert_eq!(ok(f.approve(proposal, 3, operation)), receipt);
    refused(f.approve(proposal, 5, operation), "OPERATION_CONFLICT");
    assert_eq!(f.show(id), current);
    assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
    assert!(!f.data.join("index.sqlite").exists());
    f.quiet();
}

#[test]
fn captured_action_group_excludes_arrivals_and_stale_full_before_blocks_all_members() {
    let f = Fixture::new();
    let group = Uuid::new_v4();
    let ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    let proposals = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    for i in 0..2 {
        ok(f.write(
            "create",
            &draft(
                proposals[i],
                Some(group),
                json!([create(ids[i], data(&format!("Member {i}")))]),
            ),
            false,
        ));
    }
    let captured = json!({"group_id":group,"approvals":proposals[..2].iter().map(|id|
        json!({"operation_id":Uuid::new_v4(),"expected":{"id":id,"version":1}})).collect::<Vec<_>>()});
    ok(f.write(
        "create",
        &draft(
            proposals[2],
            Some(group),
            json!([create(ids[2], data("Arrival"))]),
        ),
        false,
    ));
    let result = ok(f.write("approve-group", &captured, false));
    assert_eq!(result["receipts"].as_array().unwrap().len(), 2);
    assert!(result["stopped"].is_null());
    assert_eq!(ok(f.write("approve-group", &captured, false)), result);
    refused(
        f.run(&["actions", "show", &ids[2].to_string()]),
        "NOT_FOUND",
    );
    assert_eq!(
        ok(f.run(&["proposals", "show", &proposals[2].to_string()]))["state"],
        "draft"
    );

    let before = f.show(ids[0]);
    let blocked = Uuid::new_v4();
    let sibling = Uuid::new_v4();
    let stale_input = draft(
        blocked,
        None,
        json!([
        create(sibling, data("Must not partially appear")),
        {"kind":"replace","before":before,"data":data("Stale replacement")}]),
    );
    let stale = ok(f.write("create", &stale_input, false));
    let competing = Uuid::new_v4();
    ok(f.write(
        "create",
        &draft(
            competing,
            None,
            json!([{"kind":"replace","before":before,"data":data("Winning replacement")}]),
        ),
        false,
    ));
    ok(f.approve(competing, 1, Uuid::new_v4()));
    let current = f.show(ids[0]);
    let journals = ok(f.run(&["proposals", "applies"]));
    refused(f.approve(blocked, 1, Uuid::new_v4()), "CONTEXT_STALE");
    assert_eq!(
        ok(f.run(&["proposals", "show", &blocked.to_string()])),
        stale
    );
    assert_eq!(ok(f.run(&["proposals", "applies"])), journals);
    refused(
        f.run(&["actions", "show", &sibling.to_string()]),
        "NOT_FOUND",
    );
    assert_eq!(f.show(ids[0]), current);

    // A valid higher-version before-record fork must fail full-record CAS even
    // when its ID and revision equal the current baseline.
    let mut fork = current.clone();
    fork["data"]["description"] = json!("Invented equal-version baseline");
    let bad = Uuid::new_v4();
    refused(
        f.write(
            "create",
            &draft(
                bad,
                None,
                json!([{"kind":"replace","before":fork,"data":data("Refused fork")}]),
            ),
            false,
        ),
        "CONTEXT_STALE",
    );
    refused(f.run(&["proposals", "show", &bad.to_string()]), "NOT_FOUND");
    assert_eq!(f.show(ids[0]), current);
    assert_eq!(
        ok(f.run(&["actions", "list"]))["entries"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    f.quiet();
}

#[test]
fn mixed_full_action_references_apply_and_changed_source_refuses_without_partial_effects() {
    let f = Fixture::new();
    let source_id = Uuid::new_v4();
    let source_text =
        format!("\u{feff}---\r\nbrn_id: {source_id}\r\nbrn_kind: source\r\n---\r\nOriginal õ\r\n");
    fs::write(f.vault.join("source.md"), &source_text).unwrap();
    let observed = ok(f.run(&[
        "edit",
        "open",
        "source.md",
        "--vault",
        f.vault.to_str().unwrap(),
    ]))["observed"]
        .clone();
    let root = Uuid::new_v4();
    let child = Uuid::new_v4();
    let proposal = Uuid::new_v4();
    let mut full = data("Full Action λ");
    for key in ["related_person", "related_project", "thread"] {
        full[key] = json!(source_id);
    }
    full["sources"] = json!([source_id]);
    full["dependencies"] = json!([root]);
    full["parent"] = json!(root);
    full["follows_up"] = json!(root);
    let note = "\u{feff}Reviewed mixed note 日本語\r\n";
    let mut input = draft(
        proposal,
        None,
        json!([create(root, data("Root")), create(child, full.clone())]),
    );
    input["changes"] = json!([{"kind":"create","path":"created.md","text":note}]);
    input["sources"] = json!([{"path":"source.md","fingerprint":observed}]);
    let reviewed = ok(f.write("create", &input, true));
    assert!(reviewed["draft"]["vault"].is_object());
    assert_eq!(ok(f.run(&["actions", "list"]))["entries"], json!([]));
    assert!(!f.vault.join("created.md").exists());
    let operation = Uuid::new_v4();
    let receipt = ok(f.approve(proposal, 1, operation));
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(f.show(child)["data"], full);
    assert_eq!(
        ok(f.run(&["activity", "list"]))["entries"][0]["summary"],
        "Created 1 note; created 2 actions."
    );
    assert_eq!(f.show(child)["origin"]["data"], full);
    assert_eq!(
        fs::read(f.vault.join("created.md")).unwrap(),
        note.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        source_text.as_bytes()
    );
    let journals = ok(f.run(&["proposals", "applies"]));
    assert_eq!(journals[0]["receipt"], receipt);
    assert_eq!(journals[0]["approved"]["draft"], reviewed["draft"]);
    assert_eq!(ok(f.approve(proposal, 1, operation)), receipt);

    let before = f.show(child);
    let blocked = Uuid::new_v4();
    let absent = Uuid::new_v4();
    let mut proposed = full.clone();
    proposed["description"] = json!("Must remain proposal-only 日本語\r\n");
    let mut failing = draft(
        blocked,
        None,
        json!([
        {"kind":"replace","before":before,"data":proposed}, create(absent, data("Not installed"))]),
    );
    failing["changes"] = json!([{"kind":"create","path":"blocked.md","text":"Not installed\r\n"}]);
    failing["sources"] = input["sources"].clone();
    let failed_review = ok(f.write("create", &failing, false));
    fs::write(f.vault.join("source.md"), "Changed external source 🦀\r\n").unwrap();
    refused(f.approve(blocked, 1, Uuid::new_v4()), "CONTEXT_STALE");
    assert_eq!(f.show(child), before);
    refused(
        f.run(&["actions", "show", &absent.to_string()]),
        "NOT_FOUND",
    );
    assert_eq!(
        ok(f.run(&["proposals", "show", &blocked.to_string()])),
        failed_review
    );
    assert_eq!(ok(f.run(&["proposals", "applies"])), journals);
    assert!(!f.vault.join("blocked.md").exists());
    assert_eq!(
        fs::read(f.vault.join("created.md")).unwrap(),
        note.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        "Changed external source 🦀\r\n".as_bytes()
    );
    f.quiet();
}
