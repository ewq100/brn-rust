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
fn terminal_replay_without_current_choices_auth_or_available_vault_preserves_frozen_choices() {
    for (obsolete, effort) in [
        (false, None),
        (true, None),
        (false, Some("low")),
        (true, Some("high")),
    ] {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        let op = Uuid::new_v4();
        let turn = app
            .work_store_mut()
            .begin_turn_with_effort(op, None, "q", "copilot", "formerly-discovered", effort)
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
            app.work_store_mut()
                .set_setting("ai.effort", "SYNTHETIC-MALFORMED-EFFORT")
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
        assert_eq!(value["data"]["effort"], serde_json::json!(effort));
        assert_eq!(value["data"]["answer"], "durable answer");
        assert!(value["data"].get("provider_turn_id").is_none());
        let (out, conflict) = run(
            &fixture.data,
            &["ask", "changed", "--operation", &op.to_string()],
        );
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(conflict["error"]["code"], "OPERATION_CONFLICT");
        let app = App::open(
            &fixture.data,
            AppConfig {
                vault_root: None,
                credentials_dir: None,
                model_dir: None,
            },
        )
        .unwrap();
        let recorded = app.work_store().turn(op).unwrap().unwrap();
        assert_eq!(recorded.effort.as_deref(), effort);
        assert_eq!(recorded.answer, "durable answer");
        assert_eq!(
            std::fs::read_dir(app.auth().credentials_dir())
                .unwrap()
                .count(),
            0
        );
        assert!(!out
            .stdout
            .windows(b"SYNTHETIC-MALFORMED-EFFORT".len())
            .any(|bytes| bytes == b"SYNTHETIC-MALFORMED-EFFORT"));
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
    assert!(run(&fixture.data, &["ai", "effort", "high"])
        .0
        .status
        .success());
    let op = Uuid::new_v4();
    let (out, failed) = run(&fixture.data, &["ask", "q", "--operation", &op.to_string()]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(failed["error"]["code"], "AI_RECONNECT_NEEDED");
    let context = &failed["error"]["context"];
    assert_eq!(context["operation_id"], op.to_string());
    assert_eq!(context["recorded_status"], "failed");
    assert_eq!(context["provider_outcome"], "unknown");
    assert_eq!(context["saved"], true);
    assert_eq!(context["receipt"]["effort"], "high");
    assert!(run(&fixture.data, &["ai", "effort", "low"])
        .0
        .status
        .success());
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
    assert!(run(&fixture.data, &["ai", "effort", "medium"])
        .0
        .status
        .success());
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

#[test]
fn fresh_ask_without_explicit_effort_refuses_before_turn_or_credential_admission() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    app.select(Selection {
        provider: Provider::Chatgpt,
        model: "gpt-5.5".into(),
    })
    .unwrap();
    drop(app);
    let id = Uuid::new_v4();
    let (out, value) = run(&fixture.data, &["ask", "q", "--operation", &id.to_string()]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(value["error"]["code"], "AI_SELECTION_REQUIRED");
    assert_eq!(value["error"]["context"]["saved"], false);
    let app = fixture.app();
    assert!(app.work_store().turn(id).unwrap().is_none());
    assert_eq!(
        std::fs::read_dir(app.auth().credentials_dir())
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn invalid_work_budgets_refuse_before_workspace_creation() {
    let owner = tempfile::tempdir().unwrap();
    for (index, args) in [
        vec!["ask", "q", "--max-tool-rounds", "0"],
        vec!["ask", "q", "--max-tool-rounds", "33"],
        vec!["ask", "q", "--work-timeout-seconds", "0"],
        vec!["ask", "q", "--work-timeout-seconds", "3601"],
        vec![
            "ask",
            "q",
            "--max-tool-rounds",
            "1",
            "--max-tool-rounds",
            "2",
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let data = owner.path().join(format!("unopened-{index}"));
        let (output, value) = run(&data, &args);
        assert_eq!(output.status.code(), Some(2), "{value}");
        assert!(!data.exists());
    }
}

#[test]
fn configured_budget_survives_failure_restart_and_omitted_replay() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    app.select(Selection {
        provider: Provider::Chatgpt,
        model: "synthetic-no-auth".into(),
    })
    .unwrap();
    app.work_store_mut()
        .set_setting("ai.effort", "medium")
        .unwrap();
    drop(app);
    let op = Uuid::new_v4();
    let (output, value) = run(
        &fixture.data,
        &[
            "ask",
            "q",
            "--operation",
            &op.to_string(),
            "--max-tool-rounds",
            "2",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{value}");
    assert_eq!(value["error"]["code"], "AI_RECONNECT_NEEDED");
    // Supplying one flag fills the other with the documented fresh default.
    let budget = serde_json::json!({"max_tool_rounds": 2, "timeout_seconds": 300});
    assert_eq!(value["error"]["context"]["budget"], budget);
    assert_eq!(value["error"]["context"]["receipt"]["budget"], budget);
    let (output, replay) = run(&fixture.data, &["ask", "q", "--operation", &op.to_string()]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        replay["error"]["context"]["receipt"],
        value["error"]["context"]["receipt"]
    );
    let (output, conflict) = run(
        &fixture.data,
        &[
            "ask",
            "q",
            "--operation",
            &op.to_string(),
            "--max-tool-rounds",
            "3",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(conflict["error"]["code"], "OPERATION_CONFLICT");
    let app = fixture.app();
    assert_eq!(
        serde_json::to_value(app.work_store().run_budget(op).unwrap()).unwrap(),
        budget
    );
    assert_eq!(
        std::fs::read_dir(app.auth().credentials_dir())
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn recorded_time_limit_and_history_keep_budget_without_provider_or_vault() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    let op = Uuid::new_v4();
    let budget = brn_workflow::WorkBudget {
        max_tool_rounds: 4,
        timeout_seconds: 60,
    };
    let (turn, _) = app
        .work_store_mut()
        .begin_turn_with_effort_and_budget(
            op,
            None,
            "bounded synthetic",
            "chatgpt",
            "former-model",
            Some("medium"),
            Some(budget),
        )
        .unwrap();
    app.work_store_mut()
        .finish_turn(
            op,
            WorkTurnStatus::Failed,
            "retained partial",
            Some("time_limit_reached"),
        )
        .unwrap();
    drop(app);
    std::fs::remove_dir_all(&fixture.vault).unwrap();
    let (output, value) = run(
        &fixture.data,
        &["ask", "bounded synthetic", "--operation", &op.to_string()],
    );
    assert_eq!(output.status.code(), Some(124), "{value}");
    assert_eq!(value["error"]["code"], "AI_TIME_LIMIT_REACHED");
    assert_eq!(
        value["error"]["context"]["receipt"]["answer"],
        "retained partial"
    );
    assert_eq!(
        value["error"]["context"]["budget"],
        serde_json::json!(budget)
    );
    let (output, history) = run(
        &fixture.data,
        &["conversations", "show", &turn.conversation_id.to_string()],
    );
    assert!(output.status.success(), "{history}");
    assert_eq!(
        history["data"]["turns"][0]["budget"],
        serde_json::json!(budget)
    );
}
