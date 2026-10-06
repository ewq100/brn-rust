//! Actual CLI envelopes and pre-start refusal; synthetic input, no provider.
use serde_json::{json, Value};
use std::{fs, process::Command};
mod support;

#[test]
fn visual_help_and_invalid_commands_preserve_command_identity_without_opening_state() {
    let data = support::data_dir();
    for (tokens, name, usage) in [
        (
            vec!["inbox", "visual"],
            "inbox.visual",
            "brn inbox visual SOURCE_PATH",
        ),
        (
            vec!["inbox", "interpret-visual"],
            "inbox.interpret-visual",
            "brn inbox interpret-visual --file REQUEST_JSON",
        ),
        (
            vec!["inbox", "visual-annotation"],
            "inbox.visual-annotation",
            "brn inbox visual-annotation ANALYSIS_UUID",
        ),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(&tokens)
            .arg("--help")
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(String::from_utf8(out.stdout).unwrap().contains(usage));
        let out = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(&tokens)
            .args(["--json", "--data-dir"])
            .arg(data.path())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stderr.is_empty());
        let value: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["command"], name);
        assert_eq!(value["error"]["code"], "USAGE");
        assert_eq!(fs::read_dir(data.path()).unwrap().count(), 0);
    }
}

#[test]
fn visual_subprocess_requires_purpose_complete_asset_and_explicit_choices_before_startup() {
    let owner = support::data_dir();
    let data = owner.path().join("data");
    fs::create_dir(&data).unwrap();
    let credentials = owner.path().join("credentials");
    let input = owner.path().join("visual.json");
    for field in [
        "purpose",
        "selection",
        "model",
        "effort",
        "visual_asset",
        "unknown",
    ] {
        let mut value = json!({
            "id":uuid::Uuid::new_v4(),"conversation":null,"purpose":"visual_interpretation",
            "source":{"source":{"path":"source.md","fingerprint":{"device":1,"inode":2,"len":0,"sha256":vec![0u8;32]}},"text":""},
            "visual_asset":{"path":"image.png","fingerprint":{"device":1,"inode":3,"len":0,"sha256":vec![0u8;32]}},
            "selection":{"provider":"chatgpt","model":"gpt-6-luna"},"effort":"medium","generation":73
        });
        match field {
            "model" => {
                value["selection"].as_object_mut().unwrap().remove("model");
            }
            "unknown" => {
                value["approve"] = json!(true);
            }
            _ => {
                value.as_object_mut().unwrap().remove(field);
            }
        }
        let exact = serde_json::to_vec(&value).unwrap();
        fs::write(&input, &exact).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(["--json", "--data-dir"])
            .arg(&data)
            .arg("--credentials-dir")
            .arg(&credentials)
            .args(["inbox", "interpret-visual", "--file"])
            .arg(&input)
            .args(["--timeout-seconds", "17"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(out.stderr.is_empty());
        let envelope: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(envelope["ok"], false);
        assert_eq!(envelope["command"], "inbox.interpret-visual");
        assert_eq!(fs::read(&input).unwrap(), exact);
        assert_eq!(fs::read_dir(&data).unwrap().count(), 0);
        assert!(!credentials.exists());
    }
}
