#[cfg(target_os = "macos")]
use serde_json::json;
use serde_json::Value;
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
        let envelope = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("CLI did not produce an envelope: {output:?}"));
        (output.status.code().unwrap(), envelope)
    }
    #[cfg(target_os = "macos")]
    fn write(&self, value: Value) -> Value {
        fs::write(&self.input, serde_json::to_vec(&value).unwrap()).unwrap();
        ok(self.run(&[
            "proposals",
            "create",
            "--file",
            self.input.to_str().unwrap(),
            "--vault",
            self.vault.to_str().unwrap(),
        ]))
    }
    #[cfg(target_os = "macos")]
    fn observe(&self, path: &str) -> Value {
        let view = ok(self.run(&[
            "edit",
            "open",
            path,
            "--vault",
            self.vault.to_str().unwrap(),
        ]));
        assert!(view["observed"].is_object());
        view["observed"].clone()
    }
    #[cfg(target_os = "macos")]
    fn snapshot(&self) -> std::collections::BTreeMap<String, Vec<u8>> {
        fs::read_dir(&self.vault)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().into_string().unwrap(),
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect()
    }
}
#[cfg(target_os = "macos")]
fn ok((code, envelope): (i32, Value)) -> Value {
    assert_eq!(code, 0, "{envelope}");
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["ok"], true);
    envelope["data"].clone()
}

#[test]
fn malformed_undo_commands_refuse_before_workspace_admission() {
    let source = Uuid::new_v4().to_string();
    let operation = Uuid::new_v4().to_string();
    let nil = Uuid::nil().to_string();
    for args in [
        vec!["proposals", "undo-preview", &source],
        vec!["proposals", "undo", &source],
        vec!["proposals", "restore-trash", &source],
        vec![
            "proposals",
            "undo-preview",
            "not-a-uuid",
            "--operation",
            &operation,
        ],
        vec!["proposals", "undo", &nil, "--operation", &operation],
        vec!["proposals", "undo-preview", &source, "--operation", &nil],
        vec!["proposals", "undo", &source, "--operation", &source],
        vec!["proposals", "undo", &source, "--operation", "not-a-uuid"],
        vec![
            "proposals",
            "undo",
            &source,
            "--operation",
            &operation,
            "--member",
            "0",
        ],
        vec![
            "proposals",
            "restore-trash",
            &source,
            "--operation",
            &operation,
        ],
        vec![
            "proposals",
            "undo-preview",
            &source,
            "--operation",
            &operation,
            "--member",
            "64",
        ],
        vec![
            "proposals",
            "restore-trash",
            &source,
            "--operation",
            &operation,
            "--member",
            "64",
        ],
        vec![
            "proposals",
            "restore-trash",
            &source,
            "--operation",
            &operation,
            "--member",
            "-1",
        ],
        vec![
            "proposals",
            "restore-trash",
            &source,
            "--operation",
            &operation,
            "--member",
            "not-a-number",
        ],
        vec![
            "proposals",
            "undo-preview",
            &source,
            "--operation",
            &operation,
            "--member",
            "999999999999999999999999999999",
        ],
        vec![
            "proposals",
            "restore-trash",
            &source,
            "--operation",
            &operation,
            "--member",
            "0",
            "--member",
            "1",
        ],
        vec![
            "proposals",
            "undo",
            &source,
            "--operation",
            &operation,
            "--operation",
            &operation,
        ],
        vec![
            "proposals",
            "undo",
            &source,
            "extra",
            "--operation",
            &operation,
        ],
        vec!["proposals", "undo-preview", "--operation", &operation],
    ] {
        let f = Fixture::new();
        let result = f.run(&args);
        assert_eq!(result.0, 2, "{}", result.1);
        assert_eq!(result.1["ok"], false);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
        assert!(!f.data.with_file_name("data.credentials").exists());
    }
}

#[cfg(target_os = "macos")]
#[test]
fn action_compensation_cli_previews_complete_details_restarts_and_replays_without_overwriting_later_work(
) {
    use brn_workflow::{
        actions::{ActionData, ActionRecord, ActionState},
        proposals::{ActionChange, DraftRequest},
    };
    let f = Fixture::new();
    let id = Uuid::new_v4();
    let original = ActionData {
        title: "Prior details õ\r\n".into(),
        description: "Owner's exact 日本語\r\n".into(),
        state: ActionState::Waiting,
        owner: Some("Leena".into()),
        related_person: None,
        related_project: None,
        sources: Vec::new(),
        thread: None,
        due_on: Some("2026-10-12".into()),
        follow_up_on: Some("2026-10-13".into()),
        dependencies: Vec::new(),
        parent: None,
        follows_up: None,
        priority: None,
    };
    let approve = |change: ActionChange| {
        let input = DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Review Action details".into(),
            changes: Vec::new(),
            sources: Vec::new(),
            action_changes: vec![change],
        };
        fs::write(&f.input, serde_json::to_vec(&input).unwrap()).unwrap();
        let review = ok(f.run(&["proposals", "create", "--file", f.input.to_str().unwrap()]));
        let operation = Uuid::new_v4();
        let receipt = ok(f.run(&[
            "proposals",
            "approve",
            &input.id.to_string(),
            "--review-version",
            &review["version"].as_u64().unwrap().to_string(),
            "--operation",
            &operation.to_string(),
        ]));
        assert_eq!(receipt["outcome"], "applied");
        operation
    };
    let show = || -> ActionRecord {
        serde_json::from_value(ok(f.run(&["actions", "show", &id.to_string()]))).unwrap()
    };
    approve(ActionChange::Create {
        id,
        data: original.clone(),
    });
    let created = show();
    let mut revised = original.clone();
    revised.title = "Later approved details".into();
    revised.state = ActionState::Open;
    revised.due_on = Some("2026-10-15".into());
    let source = approve(ActionChange::Replace {
        before: Box::new(created.clone()),
        data: revised,
    });
    let before = show();
    let operation = Uuid::new_v4();
    let preview = ok(f.run(&[
        "proposals",
        "undo-preview",
        &source.to_string(),
        "--operation",
        &operation.to_string(),
    ]));
    assert_eq!(preview["draft"]["changes"], json!([]));
    assert_eq!(
        preview["draft"]["action_changes"][0]["before"],
        serde_json::to_value(&before).unwrap()
    );
    assert_eq!(
        preview["draft"]["action_changes"][0]["data"],
        serde_json::to_value(&original).unwrap()
    );
    assert_eq!(show(), before, "preview must not compensate");
    let receipt = ok(f.run(&[
        "proposals",
        "undo",
        &source.to_string(),
        "--operation",
        &operation.to_string(),
    ]));
    assert_eq!(receipt["outcome"], "applied");
    let restored = show();
    assert_eq!(restored.data, original);
    assert_eq!(restored.origin, created.origin);
    assert_eq!(restored.version, before.version + 1);
    assert_eq!(restored.waiting_since_ms, Some(restored.updated_at_ms));
    let mut later = restored.data.clone();
    later.description.push_str("Later owner work λ\r\n");
    approve(ActionChange::Replace {
        before: Box::new(restored),
        data: later,
    });
    let current = show();
    assert_eq!(
        ok(f.run(&[
            "proposals",
            "undo",
            &source.to_string(),
            "--operation",
            &operation.to_string()
        ])),
        receipt
    );
    assert_eq!(
        show(),
        current,
        "replay cannot overwrite later approved details"
    );
    let (store, _) = brn_store::WorkStore::open(&f.data).unwrap();
    assert!(store.setting("vault.root").unwrap().is_none());
    assert!(store.conversations().unwrap().is_empty());
    assert!(fs::read_dir(&f.vault).unwrap().next().is_none());
}

#[cfg(target_os = "macos")]
struct Source {
    id: Uuid,
    operation: Uuid,
    old: Value,
    trash: Value,
}
#[cfg(target_os = "macos")]
const OLD: &str = "\u{feff}Original 日本語\r\n";
#[cfg(target_os = "macos")]
const TRASH: &str = "\u{feff}Retained 🦀\r\n";
#[cfg(target_os = "macos")]
const CREATED: &str = "\u{feff}Created λ\r\n";
#[cfg(target_os = "macos")]
const REPLACED: &str = "\u{feff}Replacement 日本語\r\n";

#[cfg(target_os = "macos")]
fn source(f: &Fixture) -> Source {
    fs::write(f.vault.join("old.md"), OLD).unwrap();
    fs::write(f.vault.join("trash.md"), TRASH).unwrap();
    fs::write(f.vault.join("evidence.md"), "Original evidence\r\n").unwrap();
    let old = f.observe("old.md");
    let trash = f.observe("trash.md");
    let evidence = f.observe("evidence.md");
    let id = Uuid::new_v4();
    let operation = Uuid::new_v4();
    f.write(
        json!({"id":id,"group_id":Uuid::new_v4(),"session_id":null,"title":"Mixed exact bytes",
        "changes":[{"kind":"create","path":"new.md","text":CREATED},
            {"kind":"replace","path":"old.md","expected":old,"text":REPLACED},
            {"kind":"trash","path":"trash.md","expected":trash}],
        "sources":[{"path":"evidence.md","fingerprint":evidence}]}),
    );
    let receipt = ok(f.run(&[
        "proposals",
        "approve",
        &id.to_string(),
        "--review-version",
        "1",
        "--operation",
        &operation.to_string(),
    ]));
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(
        fs::read(f.vault.join("new.md")).unwrap(),
        CREATED.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("old.md")).unwrap(),
        REPLACED.as_bytes()
    );
    assert!(!f.vault.join("trash.md").exists());
    Source {
        id,
        operation,
        old,
        trash,
    }
}

#[cfg(target_os = "macos")]
#[test]
fn whole_undo_preview_preserves_files_and_exact_application_replay_preserves_later_bytes() {
    use std::os::unix::fs::MetadataExt;
    let f = Fixture::new();
    let source = source(&f);
    let operation = Uuid::new_v4();
    let snapshot = f.snapshot();
    let journals = ok(f.run(&["proposals", "applies"]));
    let reviews = ok(f.run(&["proposals", "list"]));
    let result = f.run(&[
        "proposals",
        "undo-preview",
        &source.operation.to_string(),
        "--operation",
        &operation.to_string(),
    ]);
    assert_eq!(result.1["command"], "proposals.undo-preview");
    let preview = ok(result);
    assert_eq!(preview["draft"]["id"], operation.to_string());
    assert_eq!(preview["draft"]["group_id"], Value::Null);
    assert_eq!(preview["draft"]["sources"], json!([]));
    assert_eq!(
        preview["binding"]["operation_id"],
        source.operation.to_string()
    );
    assert_eq!(preview["draft"]["changes"][0]["kind"], "trash");
    assert_eq!(preview["draft"]["changes"][0]["before_text"], CREATED);
    assert_eq!(preview["draft"]["changes"][1]["kind"], "replace");
    assert_eq!(preview["draft"]["changes"][1]["before_text"], REPLACED);
    assert_eq!(preview["draft"]["changes"][1]["text"], OLD);
    assert_eq!(preview["draft"]["changes"][2]["kind"], "create");
    assert_eq!(preview["draft"]["changes"][2]["text"], TRASH);
    assert_eq!(
        preview["binding"]["originals"][1]["fingerprint"],
        source.old
    );
    assert_eq!(
        preview["binding"]["originals"][2]["fingerprint"],
        source.trash
    );
    assert_eq!(f.snapshot(), snapshot);
    assert_eq!(ok(f.run(&["proposals", "applies"])), journals);
    assert_eq!(ok(f.run(&["proposals", "list"])), reviews);

    // Original external evidence is historical; Undo binds its inverse members.
    fs::write(f.vault.join("evidence.md"), "Later evidence\r\n").unwrap();
    let result = f.run(&[
        "proposals",
        "undo",
        &source.operation.to_string(),
        "--operation",
        &operation.to_string(),
    ]);
    assert_eq!(result.1["command"], "proposals.undo");
    let receipt = ok(result);
    assert_eq!(receipt["operation_id"], operation.to_string());
    assert_eq!(receipt["proposal_id"], operation.to_string());
    assert_eq!(receipt["outcome"], "applied");
    assert!(!f.vault.join("new.md").exists());
    assert_eq!(fs::read(f.vault.join("old.md")).unwrap(), OLD.as_bytes());
    assert_eq!(
        fs::read(f.vault.join("trash.md")).unwrap(),
        TRASH.as_bytes()
    );
    assert_eq!(
        fs::metadata(f.vault.join("old.md")).unwrap().ino(),
        source.old["inode"].as_u64().unwrap()
    );
    assert_eq!(
        fs::metadata(f.vault.join("trash.md")).unwrap().ino(),
        source.trash["inode"].as_u64().unwrap()
    );
    let inverse = ok(f.run(&["proposals", "show", &operation.to_string()]));
    assert_eq!(inverse["state"], "applied");
    assert_eq!(inverse["comments"], json!([]));
    assert_eq!(
        ok(f.run(&["proposals", "show", &source.id.to_string()]))["state"],
        "applied"
    );
    let activity = ok(f.run(&["activity", "list"]));
    let entry = activity["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["operation_id"] == operation.to_string())
        .unwrap();
    assert_eq!(entry["undo"]["operation_id"], source.operation.to_string());
    assert!(entry["undo"]["trash_member"].is_null());
    let human = f.process(&["activity", "list"], false);
    assert!(human.status.success(), "{human:?}");
    assert!(String::from_utf8(human.stdout)
        .unwrap()
        .contains(&format!("Undo of: {}", source.operation)));

    fs::write(f.vault.join("old.md"), "Later owner bytes\r\n").unwrap();
    let after = f.snapshot();
    assert_eq!(
        ok(f.run(&[
            "proposals",
            "undo",
            &source.operation.to_string(),
            "--operation",
            &operation.to_string()
        ])),
        receipt
    );
    assert_eq!(
        ok(f.run(&["proposals", "reconcile", &operation.to_string()])),
        receipt
    );
    assert_eq!(
        ok(f.run(&[
            "proposals",
            "undo-preview",
            &source.operation.to_string(),
            "--operation",
            &operation.to_string()
        ])),
        preview
    );
    assert_eq!(f.snapshot(), after);
    let conflict = f.run(&[
        "proposals",
        "restore-trash",
        &source.operation.to_string(),
        "--member",
        "2",
        "--operation",
        &operation.to_string(),
    ]);
    assert_eq!(conflict.0, 1);
    assert_eq!(conflict.1["error"]["code"], "OPERATION_CONFLICT");
    assert_eq!(f.snapshot(), after);
}

#[cfg(target_os = "macos")]
#[test]
fn scoped_trash_restore_preserves_later_other_members_and_reports_its_source() {
    use std::os::unix::fs::MetadataExt;
    let f = Fixture::new();
    let source = source(&f);
    fs::write(f.vault.join("new.md"), "Later new note\r\n").unwrap();
    fs::write(f.vault.join("old.md"), "Later old note\r\n").unwrap();
    let operation = Uuid::new_v4();
    let snapshot = f.snapshot();
    let preview = ok(f.run(&[
        "proposals",
        "undo-preview",
        &source.operation.to_string(),
        "--member",
        "2",
        "--operation",
        &operation.to_string(),
    ]));
    assert_eq!(preview["binding"]["trash_member"], 2);
    assert_eq!(preview["draft"]["changes"].as_array().unwrap().len(), 1);
    assert_eq!(preview["draft"]["changes"][0]["path"], "trash.md");
    assert_eq!(preview["draft"]["changes"][0]["text"], TRASH);
    assert_eq!(f.snapshot(), snapshot);
    let result = f.run(&[
        "proposals",
        "restore-trash",
        &source.operation.to_string(),
        "--member",
        "2",
        "--operation",
        &operation.to_string(),
    ]);
    assert_eq!(result.1["command"], "proposals.restore-trash");
    let receipt = ok(result);
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(
        fs::read(f.vault.join("new.md")).unwrap(),
        b"Later new note\r\n"
    );
    assert_eq!(
        fs::read(f.vault.join("old.md")).unwrap(),
        b"Later old note\r\n"
    );
    assert_eq!(
        fs::read(f.vault.join("trash.md")).unwrap(),
        TRASH.as_bytes()
    );
    assert_eq!(
        fs::metadata(f.vault.join("trash.md")).unwrap().ino(),
        source.trash["inode"].as_u64().unwrap()
    );
    let activity = ok(f.run(&["activity", "list"]));
    let entry = activity["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["operation_id"] == operation.to_string())
        .unwrap();
    assert_eq!(entry["undo"]["operation_id"], source.operation.to_string());
    assert_eq!(entry["undo"]["trash_member"], 2);
    let human = f.process(&["activity", "list"], false);
    assert!(human.status.success(), "{human:?}");
    assert!(String::from_utf8(human.stdout).unwrap().contains(&format!(
        "Trash restore of: {} (zero-based member 2)",
        source.operation
    )));
    let after = f.snapshot();
    assert_eq!(
        ok(f.run(&[
            "proposals",
            "restore-trash",
            &source.operation.to_string(),
            "--member",
            "2",
            "--operation",
            &operation.to_string()
        ])),
        receipt
    );
    assert_eq!(f.snapshot(), after);
}

#[cfg(target_os = "macos")]
#[test]
fn changed_whole_member_or_occupied_trash_target_refuses_without_overwriting() {
    let f = Fixture::new();
    let source = source(&f);
    fs::write(f.vault.join("old.md"), "External current bytes\r\n").unwrap();
    let before = f.snapshot();
    let refused = f.run(&[
        "proposals",
        "undo",
        &source.operation.to_string(),
        "--operation",
        &Uuid::new_v4().to_string(),
    ]);
    assert_eq!(refused.0, 1);
    assert_eq!(refused.1["error"]["code"], "CONTEXT_STALE");
    assert_eq!(f.snapshot(), before);

    fs::write(f.vault.join("trash.md"), "Occupied destination 🦀\r\n").unwrap();
    let before = f.snapshot();
    let refused = f.run(&[
        "proposals",
        "restore-trash",
        &source.operation.to_string(),
        "--member",
        "2",
        "--operation",
        &Uuid::new_v4().to_string(),
    ]);
    assert_eq!(refused.0, 1);
    assert_eq!(refused.1["error"]["code"], "CONTEXT_STALE");
    assert_eq!(f.snapshot(), before);
    // A bounded but wrong source member retains the Store Invalid mapping;
    // only malformed syntax is rejected before workspace admission.
    for member in ["0", "1", "3"] {
        let rejected = f.run(&[
            "proposals",
            "restore-trash",
            &source.operation.to_string(),
            "--member",
            member,
            "--operation",
            &Uuid::new_v4().to_string(),
        ]);
        assert_eq!(rejected.0, 1);
        assert_eq!(rejected.1["error"]["code"], "WORKFLOW_ERROR");
        assert_eq!(f.snapshot(), before);
    }
}
