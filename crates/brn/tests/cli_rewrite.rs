//! Typed preflight and offline durable Rewrite replay. No account or model calls.
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};
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
            input: owner.path().join("request.json"),
            _owner: owner,
        }
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(args)
            .args(["--json", "--data-dir"])
            .arg(&self.data)
            .output()
            .unwrap();
        let value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("CLI omitted its typed envelope: {output:?}"));
        (output.status.code().unwrap(), value)
    }

    fn rewrite(&self, request: &Value) -> (i32, Value) {
        fs::write(&self.input, serde_json::to_vec(request).unwrap()).unwrap();
        self.run(&[
            "proposals",
            "rewrite",
            "--file",
            self.input.to_str().unwrap(),
        ])
    }

    fn assert_unopened(&self) {
        assert_eq!(fs::read_dir(&self.data).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&self.vault).unwrap().count(), 0);
        assert!(!self.data.with_file_name("data.credentials").exists());
    }
}

fn request() -> Value {
    json!({"id":Uuid::new_v4(),"expected":{"id":Uuid::new_v4(),"version":1},
        "selection":{"provider":"chatgpt","model":"gpt-5.5"},"effort":"high","generation":19})
}

#[test]
fn malformed_rewrite_syntax_and_schema_refuse_before_workspace_startup() {
    let nil = Uuid::nil().to_string();
    for args in [
        vec!["proposals", "rewrite"],
        vec!["proposals", "rewrite", "extra", "--file", "missing.json"],
        vec![
            "proposals",
            "rewrite",
            "--file",
            "one.json",
            "--file",
            "two.json",
        ],
        vec!["proposals", "rewrite-status"],
        vec!["proposals", "rewrite-status", "invalid"],
        vec!["proposals", "rewrite-status", &nil],
        vec!["proposals", "rewrite-status", &nil, "extra"],
    ] {
        let fixture = Fixture::new();
        let result = fixture.run(&args);
        assert_eq!(result.0, 2, "{}", result.1);
        fixture.assert_unopened();
    }
    let valid = request();
    let changed = |field: &str, value: Value| {
        let mut result = valid.clone();
        result[field] = value;
        result
    };
    for (value, exit, code) in [
        (json!({}), 2, "USAGE"),
        (changed("unexpected", json!(true)), 2, "USAGE"),
        (changed("effort", json!("maximum")), 2, "USAGE"),
        (changed("effort", Value::Null), 2, "USAGE"),
        (changed("generation", json!(-1)), 2, "USAGE"),
        (changed("generation", json!("19")), 2, "USAGE"),
        (changed("id", json!("invalid")), 2, "USAGE"),
        (changed("id", json!(Uuid::nil())), 1, "AI_TOOL_REJECTED"),
        (
            changed("expected", json!({"id":Uuid::nil(),"version":1})),
            1,
            "AI_TOOL_REJECTED",
        ),
        (
            changed("expected", json!({"id":Uuid::new_v4(),"version":0})),
            1,
            "AI_TOOL_REJECTED",
        ),
        (
            changed(
                "selection",
                json!({"provider":"automatic","model":"gpt-5.5"}),
            ),
            2,
            "USAGE",
        ),
        (
            changed(
                "selection",
                json!({"provider":"chatgpt","model":"gpt-5.5","unexpected":true}),
            ),
            2,
            "USAGE",
        ),
        (
            changed("selection", json!({"provider":"chatgpt","model":""})),
            1,
            "AI_MODEL_REFUSED",
        ),
    ] {
        let fixture = Fixture::new();
        let result = fixture.rewrite(&value);
        assert_eq!(result.0, exit, "{}", result.1);
        assert_eq!(result.1["command"], "proposals.rewrite");
        assert_eq!(result.1["error"]["code"], code);
        fixture.assert_unopened();
    }
    for bytes in [&b"{"[..], &b"\xff"[..], &b""[..]] {
        let fixture = Fixture::new();
        fs::write(&fixture.input, bytes).unwrap();
        assert_eq!(
            fixture
                .run(&[
                    "proposals",
                    "rewrite",
                    "--file",
                    fixture.input.to_str().unwrap()
                ])
                .0,
            2
        );
        fixture.assert_unopened();
    }
    let fixture = Fixture::new();
    fs::File::create(&fixture.input)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        fixture
            .run(&[
                "proposals",
                "rewrite",
                "--file",
                fixture.input.to_str().unwrap()
            ])
            .0,
        2
    );
    fixture.assert_unopened();
}

#[test]
fn rewrite_input_rejects_a_writerless_fifo_without_blocking() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    let fixture = Fixture::new();
    let path = CString::new(fixture.input.as_os_str().as_bytes()).unwrap();
    // SAFETY: This is a NUL-terminated path in an exclusively owned fixture.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["proposals", "rewrite", "--file"])
        .arg(&fixture.input)
        .args(["--json", "--data-dir"])
        .arg(&fixture.data)
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
    assert!(!blocked, "Rewrite input waited for a FIFO writer");
    assert_eq!(output.status.code(), Some(2));
    fixture.assert_unopened();
}

#[test]
fn rewrite_history_without_vault_or_selection_returns_typed_absence() {
    let fixture = Fixture::new();
    let result = fixture.run(&["proposals", "rewrite-status", &Uuid::new_v4().to_string()]);
    assert_eq!(result.0, 0, "{}", result.1);
    assert_eq!(result.1["command"], "proposals.rewrite-status");
    assert_eq!(result.1["ok"], true);
    assert!(result.1["data"].is_null());
    assert_eq!(fs::read_dir(&fixture.vault).unwrap().count(), 0);
    assert_eq!(
        fs::read_dir(fixture.data.with_file_name("data.credentials"))
            .unwrap()
            .count(),
        0
    );
}

#[cfg(target_os = "macos")]
mod recorded {
    use super::*;
    use brn_store::work::proposal_rewrite::{RewriteOutcome, RewriteSpec, RewriteStatus};
    use brn_workflow::{
        app::{App, AppConfig},
        proposals::{DraftNoteChange, DraftRequest, ProposalEdit},
    };

    fn seeded(status: RewriteStatus) -> (Fixture, Value, Value) {
        let fixture = Fixture::new();
        let mut app = App::open(
            &fixture.data,
            AppConfig {
                vault_root: Some(fixture.vault.clone()),
                credentials_dir: None,
                model_dir: None,
            },
        )
        .unwrap();
        let record = app
            .create_proposal(&DraftRequest {
                inbox_knowledge: None,
                inbox_source: None,
                action_changes: Vec::new(),
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Synthetic draft".into(),
                changes: vec![DraftNoteChange::Create {
                    path: "new.md".into(),
                    text: "Temporary draft body 日本語\r\n".into(),
                }],
                sources: vec![],
            })
            .unwrap();
        let spec = RewriteSpec {
            id: Uuid::new_v4(),
            expected: record.stamp(),
            provider: "copilot".into(),
            model: "previously-discovered".into(),
            effort: "high".into(),
        };
        app.work_store_mut().begin_proposal_rewrite(&spec).unwrap();
        let edit = ProposalEdit {
            action_data: Vec::new(),
            expected: record.stamp(),
            title: "Rewritten".into(),
            texts: vec![Some("Complete rewritten body λ\r\n".into())],
        };
        if status == RewriteStatus::Stale {
            app.edit_proposal(&ProposalEdit {
                action_data: Vec::new(),
                title: "Newer user work".into(),
                texts: vec![Some("User text".into())],
                ..edit.clone()
            })
            .unwrap();
        }
        let outcome = match status {
            RewriteStatus::Completed | RewriteStatus::Stale => {
                Some(RewriteOutcome::Completed(edit))
            }
            RewriteStatus::Failed => Some(RewriteOutcome::Failed("model_refused".into())),
            RewriteStatus::Interrupted => Some(RewriteOutcome::Interrupted),
            RewriteStatus::Running => None,
        };
        if let Some(outcome) = outcome {
            app.work_store_mut()
                .finish_proposal_rewrite(spec.id, &outcome)
                .unwrap();
        }
        let request = json!({"id":spec.id,"expected":spec.expected,
            "selection":{"provider":spec.provider,"model":spec.model},"effort":spec.effort,"generation":23});
        let current = serde_json::to_value(app.proposal(record.draft.id).unwrap()).unwrap();
        drop(app);
        // Terminal replay/history must precede missing-vault and discovery checks.
        fs::remove_dir_all(&fixture.vault).unwrap();
        (fixture, request, current)
    }

    #[test]
    fn completed_failed_stale_and_interrupted_replay_keep_safe_jobs_and_newer_reviews() {
        for (status, exit, code) in [
            (RewriteStatus::Completed, 0, None),
            (RewriteStatus::Failed, 1, Some("AI_MODEL_REFUSED")),
            (RewriteStatus::Stale, 1, Some("CONTEXT_STALE")),
            (RewriteStatus::Interrupted, 130, Some("INTERRUPTED")),
        ] {
            let (fixture, request, current) = seeded(status);
            let id = request["id"].as_str().unwrap();
            let history = fixture.run(&["proposals", "rewrite-status", id]);
            assert_eq!(history.0, 0, "{}", history.1);
            let job = history.1["data"].clone();
            assert_eq!(job["status"], serde_json::to_value(status).unwrap());
            let replay = fixture.rewrite(&request);
            assert_eq!(replay.0, exit, "{}", replay.1);
            if let Some(code) = code {
                assert_eq!(replay.1["error"]["code"], code);
                assert_eq!(replay.1["error"]["context"]["receipt"], job);
                assert_eq!(replay.1["error"]["context"]["generation"], 23);
                assert_eq!(replay.1["error"]["context"]["saved"], true);
            } else {
                assert_eq!(replay.1["data"], job);
            }
            let text = replay.1.to_string();
            assert!(!text.contains("Temporary draft body"));
            assert!(!text.contains("Complete rewritten body"));
            let mut new_generation = request.clone();
            new_generation["generation"] = json!(24);
            let replay_generation = fixture.rewrite(&new_generation);
            assert_eq!(replay_generation.0, exit, "{}", replay_generation.1);
            if code.is_some() {
                assert_eq!(replay_generation.1["error"]["context"]["generation"], 24);
                assert_eq!(replay_generation.1["error"]["context"]["receipt"], job);
            } else {
                assert_eq!(replay_generation.1["data"], job);
            }
            assert_eq!(
                fixture
                    .run(&[
                        "proposals",
                        "show",
                        current["draft"]["id"].as_str().unwrap()
                    ])
                    .1["data"],
                current
            );
            let mut changed = request.clone();
            changed["effort"] = json!("low");
            let conflict = fixture.rewrite(&changed);
            assert_eq!(conflict.0, 1, "{}", conflict.1);
            assert_eq!(conflict.1["error"]["code"], "OPERATION_CONFLICT");
            assert_eq!(
                fixture.run(&["proposals", "rewrite-status", id]).1["data"],
                job
            );
            assert!(!fixture.vault.exists());
            assert_eq!(
                fs::read_dir(fixture.data.with_file_name("data.credentials"))
                    .unwrap()
                    .count(),
                0
            );
        }
    }

    #[test]
    fn startup_interrupts_a_recorded_running_rewrite_and_replay_never_retries() {
        let (fixture, request, current) = seeded(RewriteStatus::Running);
        let replay = fixture.rewrite(&request);
        assert_eq!(replay.0, 130, "{}", replay.1);
        let job = replay.1["error"]["context"]["receipt"].clone();
        assert_eq!(job["status"], "interrupted");
        assert_eq!(fixture.rewrite(&request), replay);
        assert_eq!(
            fixture
                .run(&[
                    "proposals",
                    "show",
                    current["draft"]["id"].as_str().unwrap()
                ])
                .1["data"],
            current
        );
        assert_eq!(
            fs::read_dir(fixture.data.with_file_name("data.credentials"))
                .unwrap()
                .count(),
            0
        );
    }
}
