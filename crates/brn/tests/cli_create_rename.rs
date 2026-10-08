use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};
use uuid::Uuid;

struct Fixture {
    _root: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    input: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("data");
        let vault = root.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        let input = root.path().join("input.json");
        Self {
            _root: root,
            data,
            vault,
            input,
        }
    }
    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(args)
            .arg("--data-dir")
            .arg(&self.data)
            .arg("--json")
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap_or_else(|_| panic!("{output:?}")),
        )
    }
    fn input(&self, command: &str, value: Value) -> (i32, Value) {
        fs::write(&self.input, serde_json::to_vec(&value).unwrap()).unwrap();
        self.run(&[
            "proposals",
            command,
            "--file",
            self.input.to_str().unwrap(),
            "--vault",
            self.vault.to_str().unwrap(),
        ])
    }
}
#[cfg(target_os = "macos")]
fn ok((code, value): (i32, Value)) -> Value {
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["ok"], true);
    value["data"].clone()
}

#[test]
fn malformed_create_rename_refuses_before_workspace_admission() {
    for request in [
        json!({"expected":{"id":Uuid::nil(),"version":1},"change_index":0,"path":"new.md"}),
        json!({"expected":{"id":Uuid::new_v4(),"version":0},"change_index":0,"path":"new.md"}),
        json!({"expected":{"id":Uuid::new_v4(),"version":1},"change_index":64,"path":"new.md"}),
        json!({"expected":{"id":Uuid::new_v4(),"version":1},"change_index":0,"path":"../new.md"}),
        json!({"expected":{"id":Uuid::new_v4(),"version":1},"change_index":0,"path":".hidden.md"}),
        json!({"expected":{"id":Uuid::new_v4(),"version":1},"change_index":0,"path":"new.md","extra":true}),
    ] {
        let f = Fixture::new();
        let (code, value) = f.input("rename-create", request);
        assert_ne!(code, 0, "{value}");
        assert_eq!(value["ok"], false);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn create_rename_cli_revises_exact_version_applies_new_path_and_replays_original_after_restart() {
    let f = Fixture::new();
    let id = Uuid::new_v4();
    let note = Uuid::new_v4();
    let text = brn_store::note_identity::assign("# Owner λ\r\n\r\nRetained 🦀\r\n", note).unwrap();
    let original = json!({"id":id,"group_id":null,"session_id":null,"title":"Correct filename","changes":[{"kind":"create","path":"original.md","text":text}],"sources":[]});
    let created = ok(f.input("create", original.clone()));
    let renamed=ok(f.input("rename-create",json!({"expected":{"id":id,"version":created["version"]},"change_index":0,"path":"corrected.md"})));
    assert_eq!(renamed["version"], 2);
    assert_eq!(renamed["draft"]["changes"][0]["text"], text);
    assert_eq!(
        ok(f.input(
            "rename-create",
            json!({"expected":{"id":id,"version":2},"change_index":0,"path":"corrected.md"})
        )),
        renamed
    );
    assert_ne!(
        f.input(
            "rename-create",
            json!({"expected":{"id":id,"version":1},"change_index":0,"path":"wrong.md"})
        )
        .0,
        0
    );
    assert_eq!(ok(f.input("create", original.clone())), renamed);
    assert!(fs::read_dir(&f.vault).unwrap().next().is_none());
    let operation = Uuid::new_v4().to_string();
    let id = id.to_string();
    assert_ne!(
        f.run(&[
            "proposals",
            "approve",
            &id,
            "--review-version",
            "1",
            "--operation",
            &Uuid::new_v4().to_string()
        ])
        .0,
        0
    );
    let receipt = ok(f.run(&[
        "proposals",
        "approve",
        &id,
        "--review-version",
        "2",
        "--operation",
        &operation,
    ]));
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(
        fs::read_to_string(f.vault.join("corrected.md")).unwrap(),
        text
    );
    assert!(!f.vault.join("original.md").exists());
    let current = ok(f.input("create", original));
    assert_eq!(current["state"], "applied");
    assert_eq!(current["draft"], renamed["draft"]);
    assert_eq!(
        ok(f.run(&[
            "proposals",
            "approve",
            &id,
            "--review-version",
            "2",
            "--operation",
            &operation
        ])),
        receipt
    );
    assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 1);
}
