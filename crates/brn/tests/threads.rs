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
/// A fresh test workspace (`init --purpose test`) for a writing test.
fn workspace(root: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
    let data = root.path().join(name);
    json_command(&data, &["init", "--purpose", "test"]);
    data
}
/// `begin-edit --token-out`: the session ID; the token is in `token`.
fn begin(data: &Path, note: &str, base: &str, token: &Path) -> String {
    let session = json_command(
        data,
        &[
            "begin-edit",
            "--note",
            note,
            "--base",
            base,
            "--token-out",
            token.to_str().unwrap(),
        ],
    );
    assert_eq!(session["holder"], "caller");
    session["id"].as_str().unwrap().to_owned()
}
#[test]
fn explicit_delegation_choices_survive_reopen_and_pause_without_implicit_consent() {
    let root = root();
    let data = root.path().join("delegation");
    let initial = json_command(&data, &["init", "--purpose", "test"]);
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
    let other = workspace(&root, "review-first");
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
    let data = workspace(&root, "data");
    let note = json_command(
        &data,
        &["note-create", "--title", "Journey", "--text", "alpha"],
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let first = root.path().join("first.token");
    let token = first.to_str().unwrap();
    let session = begin(&data, &note, "1", &first);
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
            "--token-file",
            token,
        ],
    );
    assert_eq!(update["outcome"], "Updated");
    assert_eq!(json_command(&data, &["recovery"])[0]["markdown"], "beta");
    let save = [
        "save",
        "--session",
        &session,
        "--generation",
        "1",
        "--token-file",
        token,
    ];
    let saved = json_command(&data, &save);
    assert_eq!(saved["outcome"], "Saved");
    assert_eq!(saved, json_command(&data, &save));
    let receipt = saved["receipt"]["operation"].as_str().unwrap().to_owned();
    let second = root.path().join("second.token");
    let token = second.to_str().unwrap();
    let session = begin(&data, &note, "2", &second);
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
            "--token-file",
            token,
        ],
    );
    json_command(
        &data,
        &[
            "save",
            "--session",
            &session,
            "--generation",
            "1",
            "--token-file",
            token,
        ],
    );
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
fn a_second_begin_edit_reports_the_open_session_as_structured_json() {
    let root = root();
    let data = workspace(&root, "data");
    let note = json_command(
        &data,
        &["note-create", "--title", "Survey", "--text", "alpha"],
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let open = json_command(&data, &["begin-edit", "--note", &note, "--base", "1"]);
    let second = command(&data, &["begin-edit", "--note", &note, "--base", "1"]);
    assert!(!second.status.success());
    // Before: `SQLite: UNIQUE constraint failed: edit_sessions.note`.
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(!stderr.contains("SQLite"), "{stderr}");
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(error["error"]["kind"], "edit_session_open");
    assert_eq!(error["error"]["note"], note.as_str());
    // Owner decision B3 part 1: the open session is not named.
    assert!(error["error"].get("session").is_none(), "{stderr}");
    assert_eq!(error["error"]["base_version"], 1);
    assert!(error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("open unsaved edit"));
    // The open session is untouched and still recoverable.
    let recovery = json_command(&data, &["recovery"]);
    assert_eq!(recovery.as_array().unwrap().len(), 1);
    assert_eq!(recovery[0]["id"], open["id"]);
}
#[cfg(target_os = "macos")]
#[test]
fn full_markdown_import_export_and_backup_are_fresh_and_readable() {
    let root = root();
    let data = workspace(&root, "data");
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
    // creation semantics: default-mode reads and writes initialise a fresh folder.
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
    json_command(&data, &["init", "--purpose", "test"]);
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
    json_command(&data, &["init", "--purpose", "test"]);
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
    json_command(&data, &["init", "--purpose", "test"]);
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
    json_command(&data, &["init", "--purpose", "test"]);
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
#[cfg(target_os = "macos")]
#[test]
fn useful_intake_for_a_project_stays_in_one_project_chat_across_retries() {
    let root = root();
    let data = root.path().join("useful");
    json_command(&data, &["init", "--purpose", "test"]);
    // A missing auth file makes every provider step fail before any network.
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
    let created = json_command(&data, &["project-create", "--title", "Harbor crane"]);
    let project = created["writes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["after"]["data"].get("Project").is_some())
        .unwrap()["after"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let email = root.path().join("mari-email.txt");
    std::fs::write(
        &email,
        "From: Mari Example <mari@example.invalid>\nSubject: Crane delivery\n\nThe berth survey measured 1.4 m.\n",
    )
    .unwrap();
    let source = email.to_str().unwrap();
    let threads_before = json_command(&data, &["threads"]).as_array().unwrap().len();
    let search_before =
        json_command(&data, &["search", "--query", "berth survey"])["total"].clone();
    let supply = || {
        json_command(
            &data,
            &[
                "intake",
                "--source",
                source,
                "--intent",
                "useful",
                "--project",
                &project,
            ],
        )
    };
    let first = supply();
    let second = supply();
    assert!(first["terminal"]
        .as_str()
        .unwrap()
        .starts_with("Failed:ReconnectNeeded"));
    let chat = first["thread"].as_str().unwrap().to_owned();
    assert_eq!(
        (first["reused_chat"].clone(), second["reused_chat"].clone()),
        (json!(false), json!(true))
    );
    assert_eq!(second["thread"], chat.as_str());

    // Before: two general "Useful information intake" threads.
    let threads = json_command(&data, &["threads"]);
    assert_eq!(threads.as_array().unwrap().len(), threads_before + 1);
    let projects = json_command(&data, &["projects"]);
    assert!(projects[0]["data"]["Project"]["chats"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c == chat.as_str()));
    let records = json_command(&data, &["records"]);
    let records = records.as_array().unwrap();
    // Before: the Source was general (`chat: null`).
    let sources: Vec<_> = records
        .iter()
        .filter_map(|r| r["data"].get("Source"))
        .filter(|s| s["locator"] == source)
        .collect();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["chat"], chat.as_str());
    let owner_messages = records
        .iter()
        .filter_map(|r| r["data"].get("Message"))
        .filter(|m| m["thread"] == chat.as_str() && m["role"] == "owner")
        .count();
    assert_eq!(owner_messages, 1, "a retry does not repost the instruction");
    // One failure notice naming its cause, not one per attempt.
    let attention =
        json_command(&data, &["read", "--id", &chat])["data"]["Thread"]["attention"].clone();
    let blockers: Vec<_> = attention
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["kind"] == "Blocker")
        .collect();
    assert_eq!(blockers.len(), 1, "{attention}");
    assert_eq!(
        blockers[0]["reason"],
        "Run failed: The provider account needs reconnecting. Continue when the provider is ready."
    );
    assert_eq!(
        json_command(&data, &["search", "--query", "berth survey"])["total"],
        search_before
    );

    let both = command(
        &data,
        &[
            "intake",
            "--source",
            source,
            "--intent",
            "useful",
            "--project",
            &project,
            "--thread",
            &chat,
        ],
    );
    assert!(!both.status.success());
    assert!(String::from_utf8_lossy(&both.stderr).contains("either --project or --thread"));
}
#[test]
fn ask_refuses_an_unknown_trace_level_before_any_provider_call() {
    let root = root();
    let data = root.path().join("trace");
    json_command(&data, &["init", "--purpose", "test"]);
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

#[test]
fn helper_setting_is_off_by_default_explicit_and_kept_until_turned_off() {
    let root = root();
    let data = root.path().join("helper");
    let credentials = root.path().join("copilot-credentials");
    let credentials = credentials.to_str().unwrap();
    assert!(
        json_command(&data, &["init", "--purpose", "test"])["settings"]
            .get("helper")
            .is_none()
    );
    let help = Command::new(env!("CARGO_BIN_EXE_brn"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&help.stdout).contains(
        "Experimental: helper sub-agents are tested only with gpt-6-luna at high effort; other models may give unreliable results. Best for long-document digests; runs that delegate are slower"
    ));
    let main = [
        "configure",
        "--provider",
        "Copilot",
        "--model",
        "synthetic-main",
        "--effort",
        "medium",
        "--credentials-dir",
        credentials,
    ];
    json_command(&data, &main);
    assert!(json_command(&data, &["settings"]).get("helper").is_none());
    let mut with_helper = main.to_vec();
    with_helper.extend([
        "--helper-provider",
        "Copilot",
        "--helper-model",
        "synthetic-helper",
        "--helper-effort",
        "high",
    ]);
    json_command(&data, &with_helper);
    let helper = json_command(&data, &["settings"])["helper"].clone();
    assert_eq!(
        helper,
        json!({"provider":"Copilot","model":"synthetic-helper","effort":"high",
            "auth_file":null,"credentials_dir":credentials})
    );
    // Reconfiguring the main model keeps the explicit helper selection.
    json_command(&data, &main);
    assert_eq!(json_command(&data, &["settings"])["helper"], helper);
    // A Copilot helper never takes a Codex auth file, and partial or relative
    // selections are refused rather than completed.
    let mut wrong = main.to_vec();
    wrong.extend([
        "--helper-provider",
        "Copilot",
        "--helper-model",
        "synthetic-helper",
        "--helper-effort",
        "high",
        "--helper-auth-file",
        "/synthetic/auth.json",
    ]);
    assert!(!command(&data, &wrong).status.success());
    let mut partial = main.to_vec();
    partial.extend(["--helper-model", "synthetic-helper"]);
    assert!(!command(&data, &partial).status.success());
    let mut relative = main.to_vec();
    relative.extend([
        "--helper-provider",
        "Codex",
        "--helper-model",
        "synthetic-helper",
        "--helper-effort",
        "high",
        "--helper-auth-file",
        "auth.json",
    ]);
    assert!(!command(&data, &relative).status.success());
    assert_eq!(json_command(&data, &["settings"])["helper"], helper);
    let mut off = main.to_vec();
    off.extend(["--helper", "off"]);
    json_command(&data, &off);
    assert!(json_command(&data, &["settings"]).get("helper").is_none());
}
