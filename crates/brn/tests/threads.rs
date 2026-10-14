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
fn explicit_delegation_choices_survive_reopen_and_pause_without_implicit_consent() {
    let root = root();
    let data = root.path().join("delegation");
    let initial = json_command(&data, &["init"]);
    assert_eq!(initial["settings"]["delegation_chosen"], false);
    assert_eq!(initial["settings"]["maintenance"], false);
    assert_eq!(
        json_command(&data, &["settings"])["delegation_chosen"],
        false
    );
    let resume = command(&data, &["resume"]);
    assert!(!resume.status.success());
    assert!(String::from_utf8_lossy(&resume.stderr)
        .to_lowercase()
        .contains("choose"));
    // Manual owner work and provider configuration do not choose delegation.
    json_command(
        &data,
        &["note-create", "--title", "Manual", "--text", "Owner text"],
    );
    json_command(
        &data,
        &[
            "configure",
            "--provider",
            "Copilot",
            "--model",
            "synthetic-model",
            "--effort",
            "low",
        ],
    );
    assert_eq!(
        json_command(&data, &["settings"])["delegation_chosen"],
        false
    );
    assert_eq!(
        json_command(&data, &["delegation", "--choice", "maintain"])["outcome"],
        "Applied"
    );
    let selected = json_command(&data, &["settings"]);
    assert_eq!(selected["delegation_chosen"], true);
    assert_eq!(selected["maintenance"], true);
    assert_eq!(selected["review_first"], false);
    assert_eq!(selected["provider"], "Copilot");
    assert_eq!(selected["model"], "synthetic-model");
    assert_eq!(json_command(&data, &["pause"])["outcome"], "Applied");
    let paused = json_command(&data, &["settings"]);
    assert_eq!(paused["delegation_chosen"], true);
    assert_eq!(paused["maintenance"], false);
    assert_eq!(json_command(&data, &["resume"])["outcome"], "Applied");
    assert_eq!(
        json_command(&data, &["delegation", "--choice", "review-first"])["outcome"],
        "Applied"
    );
    let reviewed = json_command(&data, &["settings"]);
    assert_eq!(reviewed["delegation_chosen"], true);
    assert_eq!(reviewed["maintenance"], true);
    assert_eq!(reviewed["review_first"], true);
    assert_eq!(reviewed["provider"], "Copilot");
    assert_eq!(
        json_command(&data, &["notes"])[0]["data"]["Note"]["markdown"],
        "Owner text"
    );
    let other = root.path().join("review-first");
    assert_eq!(
        json_command(&other, &["delegation", "--choice", "review-first"])["outcome"],
        "Applied"
    );
    assert_eq!(json_command(&other, &["settings"])["review_first"], true);
}

#[test]
fn delegation_help_and_invalid_values_are_explicit_and_leave_data_unopened() {
    let help = Command::new(env!("CARGO_BIN_EXE_brn"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(
        String::from_utf8_lossy(&help.stdout).contains("delegation --choice maintain|review-first")
    );
    let root = root();
    for (index, args) in [
        vec!["delegation"],
        vec!["delegation", "--choice", "automatic"],
        vec!["delegation", "--choice", "Maintain"],
        vec!["delegation", "--choice", "maintain", "--unexpected", "yes"],
        vec![
            "delegation",
            "--choice",
            "maintain",
            "--choice",
            "review-first",
        ],
    ]
    .iter()
    .enumerate()
    {
        let data = root.path().join(format!("invalid-{index}"));
        let output = command(&data, args);
        assert!(!output.status.success());
        if index == 0 {
            assert!(String::from_utf8_lossy(&output.stderr).contains("Missing --choice"));
        } else if index < 3 {
            assert!(String::from_utf8_lossy(&output.stderr).contains("maintain or review-first"));
        }
        assert!(!data.exists());
    }
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
    let observed_again = json_command(
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
    assert_eq!(observed_again["outcome"], "Applied");
    assert_ne!(
        observed_again["receipt"]["operation"],
        result["receipt"]["operation"]
    );
    assert_eq!(json_command(&data, &["notes"]).as_array().unwrap().len(), 1);
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
    let bytes = std::fs::read(&backup).unwrap();
    assert!(!command(
        &data,
        &["backup", "--destination", backup.to_str().unwrap()]
    )
    .status
    .success());
    assert_eq!(std::fs::read(&backup).unwrap(), bytes);
    assert!(!std::fs::read_dir(root.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".brn-backup-stage-")
    }));
    json_command(
        &data,
        &[
            "note-create",
            "--title",
            "After backup",
            "--text",
            "Later source write",
        ],
    );
    let restored = root.path().join("nested/missing/restored");
    json_command(
        &restored,
        &["restore", "--source", backup.to_str().unwrap()],
    );
    assert_eq!(json_command(&restored, &["notes"]), notes);
    assert_eq!(std::fs::read(&backup).unwrap(), bytes);
    assert!(!std::fs::read_dir(restored.parent().unwrap())
        .unwrap()
        .any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".brn-restore-stage-")));
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

#[test]
fn missing_storage_refuses_init_and_reads_preserving_marker_and_recovery_evidence() {
    let root = root();
    let data = root.path().join("data");
    json_command(&data, &["init"]);
    let marker = std::fs::read(data.join(brn_threads_app::DATA_MARKER)).unwrap();
    std::fs::remove_file(data.join(brn_threads_app::DATABASE_FILENAME)).unwrap();
    std::fs::write(data.join("threads.sqlite3-wal"), b"cli WAL evidence").unwrap();
    std::fs::write(data.join("threads.sqlite3-shm"), b"cli SHM evidence").unwrap();
    for args in [["init"], ["notes"], ["recovery"]] {
        let output = command(&data, &args);
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("missing") && error.contains("backup") && error.contains("fresh"),
            "{error}"
        );
        assert!(!data.join(brn_threads_app::DATABASE_FILENAME).exists());
        assert_eq!(
            std::fs::read(data.join(brn_threads_app::DATA_MARKER)).unwrap(),
            marker
        );
        assert_eq!(
            std::fs::read(data.join("threads.sqlite3-wal")).unwrap(),
            b"cli WAL evidence"
        );
        assert_eq!(
            std::fs::read(data.join("threads.sqlite3-shm")).unwrap(),
            b"cli SHM evidence"
        );
    }
}

#[test]
fn explicit_backup_inside_workspace_is_refused_and_next_open_preserves_records() {
    let root = root();
    let data = root.path().join("workspace");
    json_command(&data, &["init"]);
    json_command(
        &data,
        &[
            "note-create",
            "--title",
            "Synthetic",
            "--text",
            "Preserved content",
        ],
    );
    let notes = json_command(&data, &["notes"]);
    let inside = data.join("backup.sqlite3");
    let output = command(
        &data,
        &["backup", "--destination", inside.to_str().unwrap()],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("outside the workspace"));
    assert!(!inside.exists());
    assert_eq!(json_command(&data, &["notes"]), notes);
}
#[test]
fn search_pages_report_exact_counts_and_reject_bad_cursors() {
    let root = root();
    let data = root.path().join("search");
    json_command(&data, &["init"]);
    for i in 0..14 {
        json_command(
            &data,
            &[
                "note-create",
                "--title",
                &format!("Entry {i}"),
                "--text",
                &format!("# Log\n\nA curlew sighting number {i}."),
            ],
        );
    }
    let first = json_command(&data, &["search", "--query", "curlews"]);
    assert_eq!(
        (first["total"].as_u64(), first["shown"].as_u64()),
        (Some(14), Some(12))
    );
    assert_eq!(first["cursor"], "12");
    assert_eq!(first["hits"][0]["heading_path"], "Log");
    assert!(first["hits"][0]["snippet"]
        .as_str()
        .unwrap()
        .contains("curlew"));
    let second = json_command(&data, &["search", "--query", "curlews", "--cursor", "12"]);
    assert_eq!(
        (second["shown"].as_u64(), second["offset"].as_u64()),
        (Some(2), Some(12))
    );
    assert_eq!(second["cursor"], Value::Null);
    assert!(
        !command(&data, &["search", "--query", "curlew", "--cursor", "7"])
            .status
            .success()
    );
    assert!(!command(
        &data,
        &["search", "--query", "curlew", "--thread", "missing"]
    )
    .status
    .success());
    let chat = json_command(&data, &["thread-start", "--title", "Birds"]);
    let scoped = json_command(
        &data,
        &[
            "search",
            "--query",
            "curlew",
            "--limit",
            "50",
            "--thread",
            chat["id"].as_str().unwrap(),
        ],
    );
    assert_eq!(scoped["shown"], 14);
}
#[test]
fn topics_connections_and_knowledge_check_commands() {
    let root = root();
    let data = root.path().join("topics");
    json_command(&data, &["init"]);
    assert_eq!(json_command(&data, &["topics"]), Value::Null);
    let created = json_command(&data, &["topics-create"]);
    assert_eq!(created["pointer"]["outcome"], "Applied");
    let id = created["note"]["id"].as_str().unwrap().to_owned();
    let topics = json_command(&data, &["topics"]);
    assert_eq!(
        (
            topics["note"].as_str(),
            topics["entries"].as_array().map(Vec::len)
        ),
        (Some(id.as_str()), Some(0))
    );
    assert_eq!(
        json_command(&data, &["settings"])["topics_note"],
        id.as_str()
    );
    let other = json_command(&data, &["note-create", "--title", "Twin", "--text", "Same"]);
    json_command(
        &data,
        &["note-create", "--title", "Twins", "--text", "Same"],
    );
    assert_eq!(
        json_command(
            &data,
            &["topics-choose", "--note", other["id"].as_str().unwrap()]
        )["outcome"],
        "Applied"
    );
    let around = json_command(&data, &["connections", "--note", &id, "--hops", "2"]);
    assert_eq!(around["centre"]["title"], "Topics");
    assert_eq!(around["hops"].as_array().unwrap().len(), 2);
    assert!(
        !command(&data, &["connections", "--note", &id, "--hops", "3"])
            .status
            .success()
    );
    let check = json_command(&data, &["knowledge-check", "--seed-chat", "yes"]);
    assert_eq!(check["counts"]["DuplicateTitle"], 1);
    assert_eq!(check["counts"]["DuplicateContent"], 1);
    let chat = check["review_chat"].as_str().unwrap();
    let threads = json_command(&data, &["threads"]);
    assert!(threads.as_array().unwrap().iter().any(|t| t["id"] == chat));
}
#[test]
fn ask_refuses_an_unknown_trace_level_before_any_provider_call() {
    let root = root();
    let data = root.path().join("trace");
    json_command(&data, &["init"]);
    let chat = json_command(&data, &["thread-start", "--title", "Trace"]);
    let output = command(
        &data,
        &[
            "ask",
            "--thread",
            chat["id"].as_str().unwrap(),
            "--prompt",
            "Hi",
            "--trace",
            "verbose",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--trace"));
}
