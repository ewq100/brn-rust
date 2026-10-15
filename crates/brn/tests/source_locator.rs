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
    json_command(&data, &["init"]);
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
