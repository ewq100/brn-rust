//! One file is one source however its path is spelled on the command line.
//! Synthetic data; a missing auth file makes every provider step fail offline.
use serde_json::{json, Value};
use std::{
    path::Path,
    process::{Command, Output},
};
fn command(data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brn"))
        .arg("--data-dir")
        .arg(data)
        .args(args)
        .output()
        .unwrap()
}
fn json_command(data: &Path, args: &[&str]) -> Value {
    let output = command(data, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[cfg(unix)]
#[test]
fn useful_intake_of_one_markdown_file_through_two_spellings_reuses_its_chat() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("useful");
    json_command(&data, &["init", "--purpose", "test"]);
    let missing = root.path().join("missing-auth.json");
    json_command(
        &data,
        &[
            "configure",
            "--provider",
            "Codex",
            "--model",
            "gpt-6.1-sol",
            "--effort",
            "medium",
            "--auth-file",
            missing.to_str().unwrap(),
        ],
    );
    let real = root.path().join("real");
    std::fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, root.path().join("alias")).unwrap();
    std::fs::write(
        real.join("crane.md"),
        "# Crane delivery\n\nThe berth survey measured 1.4 m.\n",
    )
    .unwrap();
    let alias = root.path().join("alias/crane.md");
    let real = real.join("crane.md");
    let supply = |source: &Path| {
        json_command(
            &data,
            &[
                "intake",
                "--source",
                source.to_str().unwrap(),
                "--intent",
                "useful",
            ],
        )
    };
    let first = supply(&alias);
    let second = supply(&real);
    assert!(first["terminal"]
        .as_str()
        .unwrap()
        .starts_with("Failed:ReconnectNeeded"));
    // Before: a second chat (`reused_chat` false) for the real spelling.
    assert_eq!(
        (first["reused_chat"].clone(), second["reused_chat"].clone()),
        (json!(false), json!(true))
    );
    assert_eq!(second["thread"], first["thread"]);
    let records = json_command(&data, &["records"]);
    let sources: Vec<_> = records
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["data"].get("Source"))
        .collect();
    assert_eq!(sources.len(), 1, "{sources:?}");
    assert_eq!(sources[0]["locator"], real.to_str().unwrap());
}
/// A fresh store configured for Codex with a missing auth file, so every
/// provider step fails offline.
fn offline_store(root: &Path) -> std::path::PathBuf {
    let data = root.join("useful");
    json_command(&data, &["init", "--purpose", "test"]);
    let missing = root.join("missing-auth.json");
    json_command(
        &data,
        &[
            "configure",
            "--provider",
            "Codex",
            "--model",
            "gpt-6.1-sol",
            "--effort",
            "medium",
            "--auth-file",
            missing.to_str().unwrap(),
        ],
    );
    data
}
#[test]
fn two_cli_useful_intakes_of_one_source_post_one_instruction() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = offline_store(root.path());
    let source = root.path().join("crane.md");
    std::fs::write(
        &source,
        "# Crane delivery\n\nThe berth survey measured 1.4 m.\n",
    )
    .unwrap();
    let supply = || {
        json_command(
            &data,
            &[
                "intake",
                "--source",
                source.to_str().unwrap(),
                "--intent",
                "useful",
            ],
        )
    };
    let first = supply();
    let second = supply();
    assert_eq!(second["thread"], first["thread"]);
    let chat = first["thread"].as_str().unwrap();
    let records = json_command(&data, &["records"]);
    let records = records.as_array().unwrap();
    let owner: Vec<_> = records
        .iter()
        .filter(|r| {
            r["data"]
                .get("Message")
                .is_some_and(|m| m["thread"] == chat && m["role"] == "owner")
        })
        .collect();
    assert_eq!(owner.len(), 1, "{owner:?}");
    // The desktop's wording, so a desktop supply in this chat reuses it.
    assert_eq!(
        owner[0]["data"]["Message"]["text"],
        brn_threads_app::SOURCE_INTAKE_INSTRUCTION
    );
    for run in [&first["run"], &second["run"]] {
        let run = records
            .iter()
            .find(|r| r["id"] == *run)
            .and_then(|r| r["data"].get("Run"))
            .unwrap();
        assert_eq!(run["instruction"], owner[0]["id"]);
    }
}
