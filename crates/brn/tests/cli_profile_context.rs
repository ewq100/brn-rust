//! Actual CLI processes capture strict context requests before opening workspace authority.
use brn_store::{
    work::{
        actions::{ActionData, ActionState},
        proposal_apply::{ApplyOutcome, ApprovalRequest},
        proposals::{ActionChange, ProposalDraft},
    },
    WorkStore,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;
mod support;

struct Fixture {
    data: support::DataDir,
    vault: PathBuf,
    credentials: PathBuf,
    id: Uuid,
}
impl Fixture {
    fn new() -> Self {
        let data = support::data_dir();
        let parent = data.path().parent().unwrap();
        let vault = parent.join("vault");
        fs::create_dir(&vault).unwrap();
        let credentials = parent.join("task.credentials");
        let id = Uuid::new_v4();
        fs::write(
            vault.join("profile.md"),
            format!("---\nbrn_id: {id}\n---\nExact õ profile\r\n\u{009b} terminal control\n"),
        )
        .unwrap();
        Self {
            data,
            vault,
            credentials,
            id,
        }
    }
    fn process(&self, args: &[&str], machine: bool) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_brn"));
        command
            .args(args)
            .arg("--data-dir")
            .arg(self.data.path())
            .arg("--vault")
            .arg(&self.vault)
            .arg("--credentials-dir")
            .arg(&self.credentials);
        if machine {
            command.arg("--json");
        }
        command.output().unwrap()
    }
    fn ok(&self, args: &[&str], name: &str) -> Value {
        let output = self.process(args, true);
        let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(output.status.success(), "{envelope}");
        assert_eq!(envelope["command"], name);
        assert_eq!(envelope["ok"], true);
        envelope["data"].clone()
    }
    fn request(&self) -> Value {
        let profile = self.ok(&["proposals", "source", "profile.md"], "proposals.source");
        json!({"profile": profile["source"], "note_id": self.id, "lens": "person", "action_offset": 0, "relationship_offset": 0, "limit": 25})
    }
    fn file(&self, value: &Value) -> PathBuf {
        let path = self.data.path().parent().unwrap().join("request.json");
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }
}
fn seed(f: &Fixture, person: Option<Uuid>, project: Option<Uuid>, source: Uuid) -> Value {
    let (mut store, _) = WorkStore::open(f.data.path()).unwrap();
    let id = Uuid::new_v4();
    let data = ActionData {
        title: "Full Action õ".into(),
        description: "Exact retained\r\nDescription".into(),
        state: ActionState::Open,
        owner: None,
        related_person: person,
        related_project: project,
        sources: vec![source],
        thread: Some(source),
        due_on: None,
        follow_up_on: None,
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: None,
    };
    let record = store
        .create_proposal(&ProposalDraft {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: None,
            title: "Synthetic approval".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create { id, data }],
        })
        .unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    store
        .record_proposal_prepared(request.operation_id, &[])
        .unwrap();
    store
        .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&[]))
        .unwrap();
    serde_json::to_value(store.action(id).unwrap().unwrap()).unwrap()
}

#[test]
fn actual_process_returns_full_saved_profile_exact_actions_and_lens_distinction_and_quotes_controls(
) {
    let f = Fixture::new();
    let source = Uuid::new_v4();
    fs::write(
        f.vault.join("source.md"),
        format!("---\nbrn_id: {source}\nbrn_kind: source\n---\nExact email"),
    )
    .unwrap();
    let person = seed(&f, Some(f.id), None, source);
    let project = seed(&f, None, Some(f.id), source);
    let mut request = f.request();
    let path = f.file(&request);
    let args = ["context", "inspect", "--file", path.to_str().unwrap()];
    let context = f.ok(&args, "context.inspect");
    assert_eq!(context["request"], request);
    assert_eq!(context["actions"], json!([person]));
    assert_eq!(context["action_total"], 1);
    assert_eq!(
        context["profile"]["text"],
        fs::read_to_string(f.vault.join("profile.md")).unwrap()
    );
    assert_eq!(context["references"].as_array().unwrap().len(), 1);
    assert_eq!(context["references"][0]["matches"][0]["scope"], "source");
    let output = f.process(&args, false);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\\u009b"));
    assert!(!text.contains('\u{009b}'));
    request["lens"] = "project".into();
    f.file(&request);
    assert_eq!(f.ok(&args, "context.inspect")["actions"], json!([project]));
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    let original = fs::read(f.vault.join("profile.md")).unwrap();
    fs::write(f.vault.join("profile.md"), "Changed profile").unwrap();
    let stale = f.process(&args, true);
    assert!(!stale.status.success());
    let envelope: Value = serde_json::from_slice(&stale.stdout).unwrap();
    assert_eq!(envelope["error"]["code"], "CONTEXT_STALE");
    assert_eq!(
        original,
        context["profile"]["text"].as_str().unwrap().as_bytes()
    );
}

fn refusal(f: &Fixture, path: &Path) {
    let output = f.process(
        &["context", "inspect", "--file", path.to_str().unwrap()],
        true,
    );
    assert!(!output.status.success());
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["ok"], false);
    assert_eq!(fs::read_dir(f.data.path()).unwrap().count(), 0);
    assert!(!f.credentials.exists());
}
#[test]
fn invalid_json_schema_bounds_nonregular_and_cli_syntax_refuse_before_startup() {
    let f = Fixture::new();
    let base = json!({"profile": {"path": "profile.md", "fingerprint": {"device": 0, "inode": 0, "len": 10, "sha256": vec![0u8;32]}}, "note_id": f.id, "lens": "person", "action_offset": 0, "relationship_offset": 0, "limit": 25});
    for mutate in [
        |r: &mut Value| r["limit"] = 0.into(),
        |r: &mut Value| r["note_id"] = Uuid::nil().to_string().into(),
        |r: &mut Value| r["lens"] = "inferred".into(),
        |r: &mut Value| r["unknown"] = true.into(),
        |r: &mut Value| r["profile"]["fingerprint"]["unknown"] = true.into(),
        |r: &mut Value| r["profile"]["path"] = "../outside.md".into(),
        |r: &mut Value| r["action_offset"] = u64::MAX.into(),
    ] {
        let mut bad = base.clone();
        mutate(&mut bad);
        let file = f.file(&bad);
        refusal(&f, &file);
    }
    let file = f.file(&base);
    fs::write(&file, "x".repeat(64 * 1024 + 1)).unwrap();
    refusal(&f, &file);
    refusal(&f, &f.vault);
    let fifo = f.vault.join("request.fifo");
    use std::os::unix::ffi::OsStrExt;
    let raw = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(raw.as_ptr(), 0o600) }, 0);
    refusal(&f, &fifo);
    for args in [
        vec!["context"],
        vec!["context", "unknown"],
        vec!["context", "inspect"],
        vec!["context", "inspect", "--file"],
        vec!["context", "inspect", "extra"],
        vec!["context", "inspect", "--unknown", "value"],
    ] {
        assert!(!f.process(&args, true).status.success());
        assert_eq!(fs::read_dir(f.data.path()).unwrap().count(), 0);
        assert!(!f.credentials.exists());
    }
}
