//! Actual AppWorker subprocess replay/preflight checks. No model or account HTTP.
use brn_workflow::{
    app::{App, AppConfig},
    Provider, Selection, WorkTurnStatus,
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    pub data: PathBuf,
    pub vault: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = tempfile::Builder::new()
            .tempdir_in(std::env::temp_dir().canonicalize().unwrap())
            .unwrap();
        let data = base.path().join("data");
        let vault = base.path().join("vault");
        std::fs::create_dir(&data).unwrap();
        std::fs::create_dir(&vault).unwrap();
        std::fs::write(vault.join("plan.md"), "# Synthetic\nlaunch Tuesday\n").unwrap();
        Self {
            _owner: base,
            data,
            vault,
        }
    }
    pub fn app(&self) -> App {
        App::open(
            &self.data,
            AppConfig {
                vault_root: Some(self.vault.clone()),
                credentials_dir: None,
                model_dir: None,
            },
        )
        .unwrap()
    }
}
pub fn run(data: &Path, args: &[&str]) -> (Output, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .args(["--data-dir"])
        .arg(data)
        .arg("--json")
        .output()
        .unwrap();
    let json = serde_json::from_slice(&output.stdout).unwrap();
    (output, json)
}

#[test]
fn terminal_replay_without_selection_auth_or_available_vault_preserves_frozen_model() {
    for obsolete in [false, true] {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        let op = Uuid::new_v4();
        let turn = app
            .work_store_mut()
            .begin_turn(op, None, "q", "copilot", "formerly-discovered")
            .unwrap();
        app.work_store_mut()
            .finish_turn(op, WorkTurnStatus::Completed, "durable answer", None)
            .unwrap();
        if obsolete {
            app.work_store_mut()
                .set_setting(
                    "ai.selection",
                    r#"{"provider":"copilot","model":"no-longer-discovered"}"#,
                )
                .unwrap();
            app.work_store_mut()
                .set_setting("ai.models.copilot", "[]")
                .unwrap();
        }
        drop(app);
        std::fs::remove_dir_all(&fixture.vault).unwrap();
        let (out, value) = run(&fixture.data, &["ask", "q", "--operation", &op.to_string()]);
        assert!(out.status.success(), "{value}");
        assert_eq!(
            value["data"]["session_id"],
            turn.conversation_id.to_string()
        );
        assert_eq!(value["data"]["model"], "formerly-discovered");
        assert_eq!(value["data"]["answer"], "durable answer");
        assert!(value["data"].get("provider_turn_id").is_none());
        let (out, conflict) = run(
            &fixture.data,
            &["ask", "changed", "--operation", &op.to_string()],
        );
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(conflict["error"]["code"], "OPERATION_CONFLICT");
    }
}

#[test]
fn new_requests_use_explicit_selection_and_persist_safe_auth_failure() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    app.select(Selection {
        provider: Provider::Chatgpt,
        model: "gpt-5.5".into(),
    })
    .unwrap();
    drop(app);
    let op = Uuid::new_v4();
    let (out, failed) = run(&fixture.data, &["ask", "q", "--operation", &op.to_string()]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(failed["error"]["code"], "AI_RECONNECT_NEEDED");
    let context = &failed["error"]["context"];
    assert_eq!(context["operation_id"], op.to_string());
    assert_eq!(context["recorded_status"], "failed");
    assert_eq!(context["provider_outcome"], "unknown");
    assert_eq!(context["saved"], true);
    let (out, replay) = run(&fixture.data, &["ask", "q", "--operation", &op.to_string()]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(replay["error"]["context"], *context);
}

#[test]
fn recorded_interrupted_turn_never_resubmits() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    let op = Uuid::new_v4();
    app.work_store_mut()
        .begin_turn(op, None, "q", "chatgpt", "gpt-5.5")
        .unwrap();
    app.work_store_mut()
        .finish_turn(op, WorkTurnStatus::Interrupted, "partial", None)
        .unwrap();
    drop(app);
    let (out, value) = run(&fixture.data, &["ask", "q", "--operation", &op.to_string()]);
    assert_eq!(out.status.code(), Some(130));
    assert_eq!(value["error"]["context"]["partial"], "partial");
    assert_eq!(value["error"]["context"]["receipt"]["answer"], "partial");
    assert_eq!(value["error"]["context"]["provider_outcome"], "unknown");
}

#[test]
fn new_unknown_conversation_is_not_found_with_operation_context() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    app.select(Selection {
        provider: Provider::Chatgpt,
        model: "gpt-5.5".into(),
    })
    .unwrap();
    drop(app);
    let op = Uuid::new_v4();
    let session = Uuid::new_v4();
    let (out, value) = run(
        &fixture.data,
        &[
            "ask",
            "q",
            "--operation",
            &op.to_string(),
            "--session",
            &session.to_string(),
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(value["error"]["code"], "NOT_FOUND");
    assert_eq!(value["error"]["context"]["operation_id"], op.to_string());
    assert_eq!(value["error"]["context"]["session_id"], session.to_string());
    assert_eq!(value["error"]["context"]["saved"], false);
}

#[test]
fn timeout_parser_rejects_out_of_range_before_open() {
    let fixture = Fixture::new();
    for raw in ["0", "3601", "bad"] {
        let (out, value) = run(&fixture.data, &["ask", "q", "--timeout-seconds", raw]);
        assert_eq!(out.status.code(), Some(2));
        assert_eq!(value["error"]["code"], "USAGE");
        assert!(!fixture.data.join("brn.sqlite").exists());
    }
}
