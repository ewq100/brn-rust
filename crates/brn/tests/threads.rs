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
fn root() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
#[test]
fn save_recovers_across_processes_replays_and_undo_keeps_later_edit() {
    let root = root();
    let data = root.path().join("data");
    let note = json_command(
        &data,
        &["note-create", "--title", "Journey", "--text", "alpha"],
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let session = json_command(&data, &["begin-edit", "--note", &note, "--base", "1"])["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let update = json_command(
        &data,
        &[
            "update-buffer",
            "--session",
            &session,
            "--generation",
            "1",
            "--text",
            "beta",
        ],
    );
    assert_eq!(update["outcome"], "Updated");
    assert_eq!(json_command(&data, &["recovery"])[0]["markdown"], "beta");
    let saved = json_command(&data, &["save", "--session", &session, "--generation", "1"]);
    assert_eq!(saved["outcome"], "Saved");
    assert_eq!(
        saved,
        json_command(&data, &["save", "--session", &session, "--generation", "1"])
    );
    let receipt = saved["receipt"]["operation"].as_str().unwrap().to_owned();
    let session = json_command(&data, &["begin-edit", "--note", &note, "--base", "2"])["id"]
        .as_str()
        .unwrap()
        .to_owned();
    json_command(
        &data,
        &[
            "update-buffer",
            "--session",
            &session,
            "--generation",
            "1",
            "--text",
            "later",
        ],
    );
    json_command(&data, &["save", "--session", &session, "--generation", "1"]);
    assert_eq!(
        json_command(&data, &["undo", "--operation", &receipt])["outcome"],
        "Stale"
    );
    assert_eq!(
        json_command(&data, &["read", "--id", &note])["data"]["Note"]["markdown"],
        "later"
    );
}
#[test]
fn full_markdown_import_export_and_backup_are_fresh_and_readable() {
    let root = root();
    let data = root.path().join("data");
    let source = root.path().join("source.md");
    std::fs::write(
        &source,
        "# Harbor\n\n| Step | Owner |\n|---|---|\n| Check | Ana |\n",
    )
    .unwrap();
    let result = json_command(
        &data,
        &[
            "intake",
            "--source",
            source.to_str().unwrap(),
            "--intent",
            "full",
            "--title",
            "Harbor process",
        ],
    );
    assert_eq!(result["outcome"], "Applied");
    let notes = json_command(&data, &["notes"]);
    assert_eq!(notes.as_array().unwrap().len(), 1);
    assert_eq!(notes[0]["data"]["Note"]["protected"], true);
    assert_eq!(
        json_command(
            &data,
            &[
                "intake",
                "--source",
                source.to_str().unwrap(),
                "--intent",
                "full",
                "--title",
                "Harbor process"
            ]
        ),
        result
    );
    let export = root.path().join("export");
    let manifest = json_command(
        &data,
        &["export", "--destination", export.to_str().unwrap()],
    );
    assert_eq!(manifest["files"].as_array().unwrap().len(), 1);
    assert!(!command(
        &data,
        &["export", "--destination", export.to_str().unwrap()]
    )
    .status
    .success());
    let backup = root.path().join("backup.sqlite");
    json_command(
        &data,
        &["backup", "--destination", backup.to_str().unwrap()],
    );
    assert!(!command(
        &data,
        &["backup", "--destination", backup.to_str().unwrap()]
    )
    .status
    .success());
    let restored = root.path().join("restored");
    json_command(
        &restored,
        &["restore", "--source", backup.to_str().unwrap()],
    );
    assert_eq!(json_command(&restored, &["notes"]), notes);
    assert!(!command(
        &restored,
        &["restore", "--source", backup.to_str().unwrap()]
    )
    .status
    .success());
}
#[test]
fn explicit_directory_old_format_and_invalid_commands_refuse_without_touching_data() {
    let root = root();
    let old = root.path().join("old");
    std::fs::create_dir(&old).unwrap();
    std::fs::write(old.join("brn.sqlite"), b"old format synthetic bytes").unwrap();
    assert!(!command(&old, &["init"]).status.success());
    assert_eq!(
        std::fs::read(old.join("brn.sqlite")).unwrap(),
        b"old format synthetic bytes"
    );
    assert!(!Command::new(env!("CARGO_BIN_EXE_brn"))
        .arg("init")
        .output()
        .unwrap()
        .status
        .success());
    let data = root.path().join("fresh");
    assert!(!command(&data, &["init", "--vault", "/tmp/old"])
        .status
        .success());
    assert!(!data.exists());
    assert_eq!(json_command(&data, &["settings"])["model"], "gpt-6.1-sol");
    assert_eq!(json_command(&data, &["pause"])["outcome"], "Applied");
    assert_eq!(
        json_command(&data, &["settings"])["maintenance"],
        json!(false)
    );
}
