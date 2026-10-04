#[cfg(target_os = "macos")]
use serde_json::json;
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};
use uuid::Uuid;

mod support;

struct Fixture {
    data: support::DataDir,
    vault: PathBuf,
    #[cfg(target_os = "macos")]
    input: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let data = support::data_dir();
        let parent = data.path().parent().unwrap();
        let vault = parent.join("vault");
        fs::create_dir(&vault).unwrap();
        Self {
            #[cfg(target_os = "macos")]
            input: parent.join("draft.json"),
            data,
            vault,
        }
    }

    fn process(&self, args: &[&str], json: bool) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_brn"));
        command
            .args(args)
            .arg("--data-dir")
            .arg(self.data.path())
            .arg("--vault")
            .arg(&self.vault);
        if json {
            command.arg("--json");
        }
        command.output().unwrap()
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = self.process(args, true);
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap_or_else(|_| panic!("{output:?}")),
        )
    }

    #[cfg(target_os = "macos")]
    fn prepare(&self, note: Uuid, proposal: Uuid) -> (i32, Value) {
        self.run(&[
            "identity",
            "prepare",
            "note.md",
            "--note-id",
            &note.to_string(),
            "--proposal",
            &proposal.to_string(),
            "--title",
            "Identity 日本語\r\nλ",
        ])
    }

    #[cfg(target_os = "macos")]
    fn create(&self, draft: &Value) -> (i32, Value) {
        fs::write(&self.input, serde_json::to_vec_pretty(draft).unwrap()).unwrap();
        self.run(&[
            "proposals",
            "create",
            "--file",
            self.input.to_str().unwrap(),
        ])
    }

    #[cfg(target_os = "macos")]
    fn assert_no_review_work(&self) {
        assert_eq!(ok(self.run(&["proposals", "list"])), json!([]));
        assert_eq!(ok(self.run(&["edit", "list"])), json!([]));
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
fn invalid_identity_arguments_refuse_before_storage_or_credentials_open() {
    let note = Uuid::new_v4().to_string();
    let proposal = Uuid::new_v4().to_string();
    let nil = Uuid::nil().to_string();
    let oversized_title = "õ".repeat(257);
    let prepare = || {
        vec![
            "identity",
            "prepare",
            "note.md",
            "--note-id",
            &note,
            "--proposal",
            &proposal,
            "--title",
            "Review",
        ]
    };
    let mut cases = vec![
        vec!["identity"],
        vec!["identity", "unknown"],
        vec!["identity", "show"],
        vec!["identity", "show", "note.md", "extra"],
        vec!["identity", "show", "note.md", "--note-id", &note],
        vec!["identity", "show", "../escape.md"],
        vec!["identity", "show", "/absolute.md"],
        vec!["identity", "show", ".hidden.md"],
        vec!["identity", "show", "archive/old.md"],
        vec!["identity", "show", "note.txt"],
        vec!["identity", "prepare", "note.md"],
        vec!["identity", "prepare", "note.md", "--note-id", &note],
    ];
    for (index, value) in [
        (2, "../escape.md"),
        (4, "not-a-uuid"),
        (4, &nil),
        (6, &nil),
        (6, "not-a-uuid"),
        (8, ""),
        (8, "   "),
        (8, &oversized_title),
    ] {
        let mut args = prepare();
        args[index] = value;
        cases.push(args);
    }
    for (option, value) in [
        ("--note-id", note.as_str()),
        ("--proposal", proposal.as_str()),
        ("--title", "Duplicate"),
    ] {
        let mut args = prepare();
        args.extend([option, value]);
        cases.push(args);
    }
    for args in cases {
        let fixture = Fixture::new();
        let (code, envelope) = fixture.run(&args);
        assert_eq!(code, 2, "{args:?}: {envelope}");
        assert_eq!(envelope["ok"], false);
        assert_eq!(envelope["error"]["code"], "USAGE");
        assert_eq!(fs::read_dir(fixture.data.path()).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&fixture.vault).unwrap().count(), 0);
        assert!(!fixture
            .data
            .path()
            .with_file_name("data.credentials")
            .exists());
    }
}

#[cfg(target_os = "macos")]
#[test]
fn show_and_prepare_return_complete_exact_json_without_admitting_review_or_changing_notes() {
    for original in [
        "\u{feff}---\r\ncustom: λ\r\nsummary: |\r\n  Preserve 日本語\r\n---\r\n# Õun\r\nBody 🦀\r\n",
        "\u{feff}# Õun\r\nBody 日本語\r\nbrn_id: body content, not metadata",
        "---\ncustom: \"õ λ\"\n---\nBody 日本語 without final newline",
    ] {
        let fixture = Fixture::new();
        fs::write(fixture.vault.join("note.md"), original).unwrap();
        let (code, envelope) = fixture.run(&["identity", "show", "note.md"]);
        assert_eq!(envelope["command"], "identity.show");
        let shown = ok((code, envelope));
        assert_eq!(shown["path"], "note.md");
        assert!(shown["note_id"].is_null());
        assert_eq!(shown["sha256"].as_array().unwrap().len(), 32);
        let note = Uuid::new_v4();
        let proposal = Uuid::new_v4();
        let (code, envelope) = fixture.prepare(note, proposal);
        assert_eq!(envelope["command"], "identity.prepare");
        let draft = ok((code, envelope));
        assert_eq!(draft["id"], proposal.to_string());
        assert_eq!(draft["title"], "Identity 日本語\r\nλ");
        assert!(draft["group_id"].is_null());
        assert!(draft["session_id"].is_null());
        assert_eq!(draft["changes"].as_array().unwrap().len(), 1);
        let change = &draft["changes"][0];
        assert_eq!(change["kind"], "replace");
        assert_eq!(change["path"], "note.md");
        assert_eq!(change["expected"]["sha256"], shown["sha256"]);
        assert_eq!(change["expected"]["len"], original.len());
        assert_eq!(draft["sources"], json!([{"path":"note.md","fingerprint":change["expected"]}]));
        let proposed = change["text"].as_str().unwrap();
        assert!(proposed.contains(&format!("brn_id: {note}")));
        // Removing only the inserted field/frontmatter gives the original full
        // bytes; no normalization or truncation is tolerated by this process test.
        let separator = if original.contains("\r\n") { "\r\n" } else { "\n" };
        let without_field = proposed.replace(&format!("brn_id: {note}{separator}"), "");
        let restored = if original.trim_start_matches('\u{feff}').starts_with("---") {
            without_field
        } else {
            without_field.replacen(&format!("---{separator}---{separator}"), "", 1)
        };
        assert_eq!(restored, original);
        assert_eq!(ok(fixture.prepare(note, proposal)), draft);
        fixture.assert_no_review_work();
        assert_eq!(fs::read(fixture.vault.join("note.md")).unwrap(), original.as_bytes());
        assert_eq!(fs::read_dir(&fixture.vault).unwrap().count(), 1);

        let output = fixture.process(&[
            "identity", "prepare", "note.md", "--note-id", &note.to_string(), "--proposal",
            &proposal.to_string(), "--title", "Identity 日本語\r\nλ",
        ], false);
        assert!(output.status.success(), "{output:?}");
        let plain: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(plain, draft);
        assert!(plain.get("schema_version").is_none());
    }
}

#[cfg(target_os = "macos")]
#[test]
fn prepared_identity_uses_existing_creation_approval_restart_and_exact_undo_boundary() {
    let fixture = Fixture::new();
    let original = "\u{feff}---\r\ncustom: õ λ\r\n---\r\n正文 日本語\r\n";
    fs::write(fixture.vault.join("note.md"), original).unwrap();
    let note = Uuid::new_v4();
    let proposal = Uuid::new_v4();
    let draft = ok(fixture.prepare(note, proposal));
    let created = ok(fixture.create(&draft));
    assert_eq!(created["version"], 1);
    assert_eq!(created["state"], "draft");
    assert_eq!(created["draft"]["changes"][0]["before_text"], original);
    assert_eq!(
        created["draft"]["changes"][0]["text"],
        draft["changes"][0]["text"]
    );
    assert_eq!(
        fs::read(fixture.vault.join("note.md")).unwrap(),
        original.as_bytes()
    );
    assert!(ok(fixture.run(&["identity", "show", "note.md"]))["note_id"].is_null());
    assert_eq!(ok(fixture.create(&draft)), created);

    let operation = Uuid::new_v4();
    let receipt = ok(fixture.run(&[
        "proposals",
        "approve",
        &proposal.to_string(),
        "--review-version",
        "1",
        "--operation",
        &operation.to_string(),
    ]));
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(receipt["operation_id"], operation.to_string());
    assert_eq!(
        fs::read_to_string(fixture.vault.join("note.md")).unwrap(),
        draft["changes"][0]["text"]
    );
    // Every command is a fresh process, exercising persisted identity and receipt.
    assert_eq!(
        ok(fixture.run(&["identity", "show", "note.md"]))["note_id"],
        note.to_string()
    );
    let undo = Uuid::new_v4();
    let preview = ok(fixture.run(&[
        "proposals",
        "undo-preview",
        &operation.to_string(),
        "--operation",
        &undo.to_string(),
    ]));
    assert_eq!(preview["draft"]["changes"][0]["text"], original);
    assert_eq!(
        fs::read_to_string(fixture.vault.join("note.md")).unwrap(),
        draft["changes"][0]["text"]
    );
    let undo_receipt = ok(fixture.run(&[
        "proposals",
        "undo",
        &operation.to_string(),
        "--operation",
        &undo.to_string(),
    ]));
    assert_eq!(undo_receipt["outcome"], "applied");
    assert_eq!(
        fs::read(fixture.vault.join("note.md")).unwrap(),
        original.as_bytes()
    );
    assert!(ok(fixture.run(&["identity", "show", "note.md"]))["note_id"].is_null());
    assert_eq!(
        ok(fixture.run(&[
            "proposals",
            "undo",
            &operation.to_string(),
            "--operation",
            &undo.to_string(),
        ])),
        undo_receipt
    );
    assert_eq!(
        fs::read(fixture.vault.join("note.md")).unwrap(),
        original.as_bytes()
    );
}

#[cfg(target_os = "macos")]
#[test]
fn preparation_source_cas_refuses_changed_note_without_creating_a_proposal() {
    let fixture = Fixture::new();
    fs::write(fixture.vault.join("note.md"), "Original õ\r\n").unwrap();
    let proposal = Uuid::new_v4();
    let draft = ok(fixture.prepare(Uuid::new_v4(), proposal));
    let external = "\u{feff}External current 日本語\r\n";
    fs::write(fixture.vault.join("note.md"), external).unwrap();
    let (code, envelope) = fixture.create(&draft);
    assert_eq!(code, 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "CONTEXT_STALE");
    fixture.assert_no_review_work();
    assert_eq!(
        fs::read(fixture.vault.join("note.md")).unwrap(),
        external.as_bytes()
    );
    let shown = ok(fixture.run(&["identity", "show", "note.md"]));
    assert!(shown["note_id"].is_null());
    assert_ne!(shown["sha256"], draft["changes"][0]["expected"]["sha256"]);
}

#[cfg(target_os = "macos")]
#[test]
fn malformed_nil_duplicate_and_conflicting_managed_identity_are_reported_without_guessing() {
    let fixture = Fixture::new();
    let note = Uuid::new_v4();
    for original in [
        "---\nbrn_id: not-a-uuid\n---\nBody 日本語".to_owned(),
        format!("---\nbrn_id: {}\n---\nBody 日本語", Uuid::nil()),
        format!("---\nbrn_id: {note}\nbrn_id: {note}\n---\nBody 日本語"),
        format!("---\nbrn_id: [{note}]\n---\nBody 日本語"),
    ] {
        fs::write(fixture.vault.join("note.md"), &original).unwrap();
        for result in [
            fixture.run(&["identity", "show", "note.md"]),
            fixture.prepare(Uuid::new_v4(), Uuid::new_v4()),
        ] {
            assert_eq!(result.0, 1, "{}", result.1);
            assert_eq!(result.1["ok"], false);
            assert!(result.1.get("data").is_none());
        }
        fixture.assert_no_review_work();
        assert_eq!(
            fs::read(fixture.vault.join("note.md")).unwrap(),
            original.as_bytes()
        );
    }
    let quoted =
        format!("\u{feff}---\r\ncustom: λ\r\nbrn_id: \"{note}\"\r\n---\r\nBody 日本語\r\n");
    fs::write(fixture.vault.join("note.md"), &quoted).unwrap();
    assert_eq!(
        ok(fixture.run(&["identity", "show", "note.md"]))["note_id"],
        note.to_string()
    );
    let different = fixture.prepare(Uuid::new_v4(), Uuid::new_v4());
    assert_eq!(different.0, 1, "{}", different.1);
    assert_eq!(
        fs::read(fixture.vault.join("note.md")).unwrap(),
        quoted.as_bytes()
    );
    fixture.assert_no_review_work();
}
