//! Fresh saved-evidence observations through actual CLI processes; synthetic only.
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};
use uuid::Uuid;

mod support;

struct Fixture {
    data: support::DataDir,
    vault: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let data = support::data_dir();
        let vault = data.path().parent().unwrap().join("vault");
        fs::create_dir(&vault).unwrap();
        Self { data, vault }
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

    fn resolve(&self, id: Uuid) -> Value {
        let (code, envelope) = self.run(&["identity", "resolve", &id.to_string()]);
        assert_eq!(envelope["command"], "identity.resolve");
        ok((code, envelope))
    }

    fn inventory(&self) -> Value {
        let (code, envelope) = self.run(&["identity", "inventory"]);
        assert_eq!(envelope["command"], "identity.inventory");
        ok((code, envelope))
    }

    fn assert_no_review_work(&self) {
        assert_eq!(ok(self.run(&["proposals", "list"])), json!([]));
        assert_eq!(ok(self.run(&["edit", "list"])), json!([]));
    }
}

fn ok((code, envelope): (i32, Value)) -> Value {
    assert_eq!(code, 0, "{envelope}");
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["ok"], true);
    envelope["data"].clone()
}

fn managed(id: Uuid) -> String {
    format!("\u{feff}---\r\nbrn_id: {id}\r\ncustom: λ\r\n---\r\n正文 日本語 🦀\r\n")
}

#[test]
fn malformed_inventory_resolution_and_evidence_arguments_never_open_storage() {
    let id = Uuid::new_v4().to_string();
    let nil = Uuid::nil().to_string();
    let cases: &[&[&str]] = &[
        &["identity", "inventory", "extra"],
        &["identity", "inventory", "--note-id", &id],
        &["identity", "inventory", "--all"],
        &["identity", "resolve"],
        &["identity", "resolve", "not-a-uuid"],
        &["identity", "resolve", &nil],
        &["identity", "resolve", &id, &id],
        &["identity", "resolve", &id, "--note-id", &id],
        &["evidence"],
        &["evidence", "unknown", "note.md"],
        &["evidence", "read"],
        &["evidence", "read", "note.md", "extra"],
        &["evidence", "read", "archive/old.md", "--all"],
        &["evidence", "read", "../escape.md"],
        &["evidence", "read", "/absolute.md"],
        &["evidence", "read", ".hidden.md"],
        &["evidence", "read", "archive/.hidden.md"],
        &["evidence", "read", "archive/../escape.md"],
        &["evidence", "read", "archive//old.md"],
        &["evidence", "read", "archive\\old.md"],
        &["evidence", "read", "archive/old.txt"],
    ];
    for args in cases {
        let fixture = Fixture::new();
        let (code, envelope) = fixture.run(args);
        assert_eq!(code, 2, "{args:?}: {envelope}");
        assert_eq!(envelope["error"]["code"], "USAGE");
        assert_eq!(fs::read_dir(fixture.data.path()).unwrap().count(), 0);
        assert!(!fixture
            .data
            .path()
            .with_file_name("data.credentials")
            .exists());
    }
    for args in [
        vec!["identity", "inventory"],
        vec!["identity", "resolve", &id],
        vec!["evidence", "read", "archive/old.md"],
    ] {
        let fixture = Fixture::new();
        let mut duplicate = args;
        duplicate.extend(["--data-dir", fixture.data.path().to_str().unwrap()]);
        let (code, envelope) = fixture.run(&duplicate);
        assert_eq!(code, 2, "{envelope}");
        assert_eq!(envelope["error"]["code"], "USAGE");
        assert_eq!(fs::read_dir(fixture.data.path()).unwrap().count(), 0);
    }
}

#[test]
fn inventory_reports_current_and_archive_duplicates_without_guessing_or_mutating() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.vault.join("archive")).unwrap();
    let id = Uuid::new_v4();
    let text = managed(id);
    let paths = ["current.md", "duplicate.md", "archive/old.md"];
    for path in paths {
        fs::write(fixture.vault.join(path), &text).unwrap();
    }
    let unmanaged = "\u{feff}Unmanaged õ\r\n日本語 without final newline";
    fs::write(fixture.vault.join("unmanaged.md"), unmanaged).unwrap();
    let inventory = fixture.inventory();
    assert_eq!(inventory["notes"].as_array().unwrap().len(), 4);
    assert_eq!(inventory["issues"], json!([]));
    assert_eq!(inventory["duplicates"].as_array().unwrap().len(), 1);
    assert_eq!(inventory["duplicates"][0]["note_id"], id.to_string());
    assert_eq!(
        inventory["duplicates"][0]["paths"],
        json!(["archive/old.md", "current.md", "duplicate.md"])
    );
    let resolution = fixture.resolve(id);
    assert_eq!(resolution["outcome"], "ambiguous");
    assert_eq!(resolution["note_id"], id.to_string());
    assert_eq!(resolution["matches"].as_array().unwrap().len(), 3);
    assert_eq!(resolution["issues"], json!([]));
    assert_eq!(fixture.resolve(Uuid::new_v4())["outcome"], "absent");
    let plain = fixture.process(&["identity", "inventory"], false);
    assert!(plain.status.success(), "{plain:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&plain.stdout).unwrap(),
        inventory
    );
    fixture.assert_no_review_work();
    for path in paths {
        assert_eq!(fs::read(fixture.vault.join(path)).unwrap(), text.as_bytes());
    }
    assert_eq!(
        fs::read(fixture.vault.join("unmanaged.md")).unwrap(),
        unmanaged.as_bytes()
    );
}

#[test]
fn resolution_rereads_equal_size_retained_mtime_identity_changes_moves_and_removals() {
    let fixture = Fixture::new();
    let old = Uuid::new_v4();
    let new = Uuid::new_v4();
    let path = fixture.vault.join("current.md");
    let old_text = managed(old);
    let new_text = managed(new);
    assert_eq!(old_text.len(), new_text.len());
    fs::write(&path, &old_text).unwrap();
    let before = fs::metadata(&path).unwrap();
    let old_resolution = fixture.resolve(old);
    assert_eq!(old_resolution["outcome"], "unique");
    fs::write(&path, &new_text).unwrap();
    fs::File::open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(before.modified().unwrap()))
        .unwrap();
    let after = fs::metadata(&path).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());
    assert_eq!(fixture.resolve(old)["outcome"], "absent");
    let fresh = fixture.resolve(new);
    assert_eq!(fresh["outcome"], "unique");
    assert_eq!(fresh["matches"][0]["path"], "current.md");
    assert_ne!(
        fresh["matches"][0]["sha256"],
        old_resolution["matches"][0]["sha256"]
    );
    fs::create_dir(fixture.vault.join("archive")).unwrap();
    let moved = fixture.vault.join("archive/moved.md");
    fs::rename(&path, &moved).unwrap();
    let resolution = fixture.resolve(new);
    assert_eq!(resolution["outcome"], "unique");
    assert_eq!(resolution["matches"][0]["path"], "archive/moved.md");
    assert_eq!(fixture.inventory()["notes"].as_array().unwrap().len(), 1);
    assert_eq!(fs::read(&moved).unwrap(), new_text.as_bytes());
    fs::remove_file(moved).unwrap();
    assert_eq!(fixture.resolve(new)["outcome"], "absent");
    fixture.assert_no_review_work();
}

#[test]
fn malformed_oversized_and_non_utf8_evidence_prevents_certifying_unique_or_absent() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.vault.join("archive")).unwrap();
    let id = Uuid::new_v4();
    fs::write(fixture.vault.join("good.md"), managed(id)).unwrap();
    let invalid = [
        (
            "malformed.md",
            b"---\nbrn_id: not-a-uuid\n---\nBody".to_vec(),
        ),
        ("archive/nonutf8.md", vec![0xff, 0xfe]),
        (
            "archive/oversized.md",
            vec![b'x'; brn_workflow::MAX_NOTE_BYTES + 1],
        ),
    ];
    for (path, bytes) in &invalid {
        fs::write(fixture.vault.join(path), bytes).unwrap();
    }
    let inventory = fixture.inventory();
    assert_eq!(inventory["notes"].as_array().unwrap().len(), 1);
    assert_eq!(inventory["duplicates"], json!([]));
    let issues = inventory["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 3);
    for (path, _) in &invalid {
        let issue = issues.iter().find(|issue| issue["path"] == *path).unwrap();
        assert!(!issue["reason"].as_str().unwrap().is_empty());
    }
    for target in [id, Uuid::new_v4()] {
        let resolution = fixture.resolve(target);
        assert_eq!(resolution["outcome"], "incomplete");
        assert_eq!(
            resolution["matches"].as_array().unwrap().len(),
            usize::from(target == id)
        );
        assert_eq!(resolution["issues"], inventory["issues"]);
    }
    fixture.assert_no_review_work();
    for (path, bytes) in invalid {
        assert_eq!(fs::read(fixture.vault.join(path)).unwrap(), bytes);
    }
}

#[test]
fn explicit_archived_evidence_preserves_full_bytes_and_does_not_expand_current_reads() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.vault.join("archive")).unwrap();
    let path = "archive/old.md";
    for text in [
        managed(Uuid::new_v4()),
        "\u{feff}Saved õ\r\n正文 日本語 without final newline".to_owned(),
    ] {
        fs::write(fixture.vault.join(path), &text).unwrap();
        let (code, envelope) = fixture.run(&["evidence", "read", path]);
        assert_eq!(envelope["command"], "evidence.read");
        assert_eq!(ok((code, envelope)), json!({"path":path,"text":text}));
        let plain = fixture.process(&["evidence", "read", path], false);
        assert!(plain.status.success(), "{plain:?}");
        assert_eq!(plain.stdout, text.as_bytes());
        for args in [["notes", "show", path], ["identity", "show", path]] {
            let (code, envelope) = fixture.run(&args);
            assert_eq!(code, 2, "{envelope}");
            assert_eq!(envelope["error"]["code"], "USAGE");
        }
        assert_eq!(fs::read(fixture.vault.join(path)).unwrap(), text.as_bytes());
    }
    assert_eq!(ok(fixture.run(&["notes", "list"]))["notes"], json!([]));
    fixture.assert_no_review_work();
}
