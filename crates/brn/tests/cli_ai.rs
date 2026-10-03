use serde_json::Value;
use std::process::Command;
mod support;

#[test]
fn status_preserves_accounts_when_explicit_rediscovery_invalidates_selection() {
    use brn_workflow::{
        app::{App, AppConfig},
        ModelOption, Provider, Selection,
    };

    for case in ["absent", "valid", "stale", "malformed"] {
        let dir = support::data_dir();
        let vault = dir.path().parent().unwrap().join("vault");
        std::fs::create_dir(&vault).unwrap();
        std::fs::write(vault.join("note.md"), "# Synthetic\n").unwrap();
        let mut app = App::open(
            dir.path(),
            AppConfig {
                vault_root: Some(vault.clone()),
                credentials_dir: None,
                model_dir: None,
            },
        )
        .unwrap();
        app.record_models(
            Provider::Copilot,
            &[ModelOption {
                id: "old-model".into(),
                live_qualified: false,
            }],
        )
        .unwrap();
        if case != "absent" {
            app.select(Selection {
                provider: Provider::Copilot,
                model: "old-model".into(),
            })
            .unwrap();
        }
        if case == "stale" {
            app.record_models(
                Provider::Copilot,
                &[ModelOption {
                    id: "new-model".into(),
                    live_qualified: false,
                }],
            )
            .unwrap();
        } else if case == "malformed" {
            app.work_store_mut()
                .set_setting("ai.selection", "SYNTHETIC-PRIVATE-MALFORMED-SELECTION")
                .unwrap();
        }
        let saved = app.work_store().setting("ai.selection").unwrap();
        drop(app);

        let output = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(["ai", "status", "--json", "--data-dir"])
            .arg(dir.path())
            .output()
            .unwrap();
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(0), "{case}: {value}");
        assert_eq!(value["schema_version"], 1);
        let accounts = value["data"]["accounts"].as_array().unwrap();
        assert_eq!(accounts.len(), 2);
        for (account, provider) in accounts.iter().zip(["chatgpt", "copilot"]) {
            assert_eq!(account["provider"], provider);
            assert_eq!(account["connected"], false);
            assert!(account["name"].is_null());
        }
        if matches!(case, "stale" | "malformed") {
            assert!(value["data"]["selection"].is_null());
            assert_eq!(value["data"]["selection_error"]["code"], "AI_MODEL_REFUSED");
            assert!(value["data"]["selection_error"]["message"]
                .as_str()
                .is_some_and(|message| !message.is_empty()));
        } else {
            assert!(value["data"]["selection_error"].is_null());
            if case == "valid" {
                assert_eq!(value["data"]["selection"]["provider"], "copilot");
                assert_eq!(value["data"]["selection"]["model"], "old-model");
            } else {
                assert!(value["data"]["selection"].is_null());
            }
        }
        assert!(!String::from_utf8_lossy(&output.stdout)
            .contains("SYNTHETIC-PRIVATE-MALFORMED-SELECTION"));
        assert!(!String::from_utf8_lossy(&output.stderr)
            .contains("SYNTHETIC-PRIVATE-MALFORMED-SELECTION"));

        if case == "stale" {
            let output = Command::new(env!("CARGO_BIN_EXE_brn"))
                .args(["ask", "synthetic question", "--json", "--data-dir"])
                .arg(dir.path())
                .output()
                .unwrap();
            let refused: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(output.status.code(), Some(1));
            assert_eq!(refused["error"]["code"], "AI_MODEL_REFUSED");
        }
        let app = App::open(
            dir.path(),
            AppConfig {
                vault_root: Some(vault),
                credentials_dir: None,
                model_dir: None,
            },
        )
        .unwrap();
        assert_eq!(app.work_store().setting("ai.selection").unwrap(), saved);
    }
}

#[test]
fn explicit_account_status_and_selection_have_no_codex_requirement() {
    let dir = support::data_dir();
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["ai", "status", "--json", "--data-dir"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["data"]["accounts"].as_array().unwrap().len(), 2);
    assert!(dir.path().join("brn.sqlite").exists());
    assert!(!dir.path().join("brn.sqlite3").exists());
}

#[test]
fn bad_simple_arguments_fail_before_database_creation() {
    for args in [
        vec!["ai", "connect", "unknown"],
        vec![
            "ai",
            "select",
            "--provider",
            "chatgpt",
            "--model",
            "unknown",
        ],
        vec!["ai", "status", "--legacy"],
        vec!["status", "--legacy", "--credentials-dir", "/nonexistent"],
        vec!["models", "download"],
        vec!["ask", "question", "--profile", "keyword"],
        vec!["notes", "show", "../bad.md"],
        vec!["notes", "show", "not-a-note"],
        vec!["search", "q", "--limit", "51"],
    ] {
        let dir = support::data_dir();
        let output = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(&args)
            .args(["--json", "--data-dir"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["error"]["code"], "USAGE");
        assert!(!dir.path().join("brn.sqlite").exists());
        assert!(!dir.path().join("brn.sqlite3").exists());
    }
}

#[test]
fn default_build_download_is_typed_unsupported_without_consent_or_network() {
    if brn_workflow::native_retrieval_compiled() {
        return;
    }
    let dir = support::data_dir();
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args([
            "models",
            "download",
            "--approve-download",
            "--json",
            "--data-dir",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(value["error"]["code"], "SEMANTIC_UNAVAILABLE_IN_BUILD");
    assert!(!dir.path().join("brn.sqlite").exists());
}

#[test]
fn explicit_credential_location_is_saved_by_owner_and_honored_on_reopen() {
    let dir = support::data_dir();
    let configured = dir.path().parent().unwrap().join("chosen-credentials");
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["ai", "status", "--json", "--data-dir"])
        .arg(dir.path())
        .arg("--credentials-dir")
        .arg(&configured)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(configured.is_dir());
    assert!(!dir.path().with_file_name("data.credentials").exists());
    let app = brn_workflow::app::App::open(
        dir.path(),
        brn_workflow::app::AppConfig {
            vault_root: None,
            credentials_dir: None,
            model_dir: None,
        },
    )
    .unwrap();
    assert_eq!(app.auth().credentials_dir(), configured);
    assert_eq!(
        app.work_store().setting("ai.credentials_dir").unwrap(),
        configured.to_str().map(str::to_owned)
    );
    drop(app);
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["ai", "status", "--json", "--data-dir"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!dir.path().with_file_name("data.credentials").exists());
}

#[test]
fn unsafe_credentials_are_typed_and_never_connect() {
    for location in ["data", "repository", "vault"] {
        let dir = support::data_dir();
        let vault = dir.path().parent().unwrap().join("vault");
        std::fs::create_dir(&vault).unwrap();
        let credentials = match location {
            "data" => dir.path().join("credentials"),
            "vault" => vault.join("credentials"),
            _ => std::env::current_dir()
                .unwrap()
                .join("synthetic-never-created-credentials"),
        };
        let output = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(["ai", "connect", "chatgpt", "--json", "--data-dir"])
            .arg(dir.path())
            .arg("--vault")
            .arg(&vault)
            .arg("--credentials-dir")
            .arg(&credentials)
            .output()
            .unwrap();
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(value["error"]["code"], "AI_UNSAFE_CREDENTIALS");
        assert!(!credentials.exists());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("Login:"));
    }
}
