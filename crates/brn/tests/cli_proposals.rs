use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
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
            vault,
            input: owner.path().join("input.json"),
            _owner: owner,
        }
    }
    fn input(&self, value: &Value) {
        fs::write(&self.input, serde_json::to_vec(value).unwrap()).unwrap();
    }
    fn run(&self, args: &[&str]) -> (i32, Value) {
        run(&self.data, args)
    }
    fn write(&self, sub: &str) -> (i32, Value) {
        self.run(&[
            "proposals",
            sub,
            "--file",
            self.input.to_str().unwrap(),
            "--vault",
            self.vault.to_str().unwrap(),
        ])
    }
}
fn run(data: &Path, args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .args(["--json", "--data-dir"])
        .arg(data)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{out:?}")),
    )
}
fn ok(result: (i32, Value)) -> Value {
    assert_eq!(result.0, 0, "{}", result.1);
    assert_eq!(result.1["ok"], true);
    result.1["data"].clone()
}

#[cfg(target_os = "macos")]
#[test]
fn review_across_processes_preserves_comments_newer_work_and_vault_bytes() {
    let f = Fixture::new();
    let id = Uuid::new_v4();
    let group = Uuid::new_v4();
    let text = "\u{feff}Draft λ\r\n";
    let input = json!({"id":id, "group_id":group, "session_id":null,
        "title":"Review", "changes":[{"kind":"create","path":"new.md","text":text}], "sources":[]});
    f.input(&input);
    let created = ok(f.write("create"));
    assert_eq!(created["version"], 1);
    assert_eq!(created["draft"]["changes"][0]["text"], text);
    assert!(!f.vault.join("new.md").exists());
    let comment_id = Uuid::new_v4();
    f.input(
        &json!({"expected":{"id":id,"version":1},"comment":{"id":comment_id,"text":"Keep λ",
        "target":{"kind":"text","anchor":{"change_index":0,"start":9,"end":11,"quote":"λ"}}}}),
    );
    let commented = ok(f.write("comment"));
    assert_eq!(commented["version"], 2);
    f.input(&json!({"expected":{"id":id,"version":1},"title":"Late Rewrite","texts":["late"]}));
    let late = f.write("rewrite-result");
    assert_eq!(late.0, 1);
    assert_eq!(late.1["error"]["code"], "CONTEXT_STALE");
    f.input(&json!({"expected":{"id":id,"version":2},"title":"My review","texts":["\u{feff}Draft new λ\r\n"]}));
    let edited = ok(f.write("edit"));
    assert_eq!(edited["version"], 3);
    assert_eq!(edited["comments"][0]["target"]["kind"], "unresolved");
    let list = ok(f.run(&["proposals", "list", "--group", &group.to_string()]));
    assert_eq!(list, json!([edited.clone()]));
    f.input(&input);
    assert_eq!(ok(f.write("create")), edited);
    let rejected = ok(f.run(&[
        "proposals",
        "reject",
        &id.to_string(),
        "--review-version",
        "3",
    ]));
    assert_eq!(rejected["version"], 4);
    assert_eq!(rejected["state"], "rejected");
    assert_eq!(rejected["comments"], edited["comments"]);
    assert_eq!(ok(f.run(&["proposals", "show", &id.to_string()])), rejected);
    assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
}

#[test]
fn invalid_typed_inputs_and_unknown_approval_fail_before_workspace_open() {
    for input in [
        json!({"id":"not a uuid"}),
        json!({"id":Uuid::new_v4(),"group_id":null,"session_id":null,
        "title":"Review","changes":[{"kind":"create","path":"../escape.md","text":"x"}],"sources":[]}),
        json!({"id":Uuid::new_v4(),"group_id":null,"session_id":null,
        "title":"Review","changes":[{"kind":"create","path":"new.md","text":"x","extra":"no"}],"sources":[]}),
    ] {
        let f = Fixture::new();
        f.input(&input);
        assert_ne!(f.write("create").0, 0);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
    }
    let f = Fixture::new();
    assert_eq!(
        f.run(&["proposals", "approve", &Uuid::new_v4().to_string()])
            .0,
        2
    );
    assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
}

#[test]
fn nonregular_proposal_input_cannot_block_before_admission() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    let path = CString::new(f.input.as_os_str().as_bytes()).unwrap();
    // SAFETY: a valid NUL-terminated path in this test's exclusive synthetic folder.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["proposals", "create", "--file"])
        .arg(&f.input)
        .args(["--json", "--data-dir"])
        .arg(&f.data)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut blocked = false;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            blocked = true;
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = child.wait_with_output().unwrap();
    assert!(!blocked, "preparation blocked on a nonregular input");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_dir(&f.data).unwrap().count(), 0);
}

#[cfg(target_os = "macos")]
#[test]
fn failed_creation_does_not_store_a_partial_member_set_or_overwrite_an_occupant() {
    let f = Fixture::new();
    fs::write(f.vault.join("occupied.md"), "External bytes\r\n").unwrap();
    f.input(
        &json!({"id":Uuid::new_v4(),"group_id":null,"session_id":null,"title":"Two members",
        "changes":[{"kind":"create","path":"first.md","text":"First"},
                   {"kind":"create","path":"occupied.md","text":"Would overwrite"}],"sources":[]}),
    );
    let failed = f.write("create");
    assert_eq!(failed.0, 1);
    assert_eq!(failed.1["error"]["code"], "CONTEXT_STALE");
    assert_eq!(ok(f.run(&["proposals", "list"])), json!([]));
    assert!(!f.vault.join("first.md").exists());
    assert_eq!(
        fs::read_to_string(f.vault.join("occupied.md")).unwrap(),
        "External bytes\r\n"
    );
}
