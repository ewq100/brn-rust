//! Real process lifecycle requests, exact replay and unopened-state refusal.
use brn_store::work::{WorkBudget, WorkStore, WorkTurnStatus};
use serde_json::{json, Value};
use std::{path::Path, process::Command};
use uuid::Uuid;
mod support;

fn run(data: &Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .args(["--json", "--data-dir"])
        .arg(data)
        .output()
        .unwrap();
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

#[test]
fn archive_restart_restore_and_historical_operation_replay_preserve_exact_history() {
    let data = support::data_dir();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let operation = Uuid::new_v4();
    let budget = WorkBudget {
        max_tool_rounds: 4,
        timeout_seconds: 180,
    };
    let (turn, _) = store
        .begin_turn_with_effort_and_budget(
            operation,
            None,
            "Exact question õ\r\n",
            "chatgpt",
            "synthetic-model",
            Some("medium"),
            Some(budget),
        )
        .unwrap();
    store
        .finish_turn(turn.id, WorkTurnStatus::Completed, "Retained λ\r\n", None)
        .unwrap();
    let original = serde_json::to_value(&store.conversations().unwrap()[0]).unwrap();
    let lifecycle = store.conversation_lifecycle(turn.conversation_id).unwrap();
    drop(store);
    let file = data.path().parent().unwrap().join("lifecycle.json");
    let archive =
        json!({"operation_id":Uuid::new_v4(),"expected":lifecycle.stamp,"target":"archived"});
    std::fs::write(&file, serde_json::to_vec(&archive).unwrap()).unwrap();
    let file_arg = file.to_str().unwrap();
    let (code, result) = run(
        data.path(),
        &["conversations", "archive", "--file", file_arg],
    );
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["data"]["result"]["current"]["state"], "archived");
    let (code, active) = run(data.path(), &["conversations", "list"]);
    assert_eq!(code, 0, "{active}");
    assert_eq!(active["data"]["conversations"], json!([]));
    let (code, archived) = run(
        data.path(),
        &["conversations", "list", "--state", "archived"],
    );
    assert_eq!(code, 0, "{archived}");
    assert_eq!(archived["data"]["conversations"], json!([original.clone()]));
    let (code, shown) = run(
        data.path(),
        &["conversations", "show", &turn.conversation_id.to_string()],
    );
    assert_eq!(code, 0, "{shown}");
    assert_eq!(
        shown["data"]["turns"][0]["question"],
        "Exact question õ\r\n"
    );
    assert_eq!(shown["data"]["turns"][0]["answer"], "Retained λ\r\n");
    assert_eq!(shown["data"]["turns"][0]["budget"], json!(budget));
    let (code, replay) = run(
        data.path(),
        &[
            "ask",
            "Exact question õ\r\n",
            "--operation",
            &operation.to_string(),
        ],
    );
    assert_eq!(code, 0, "{replay}");
    assert_eq!(replay["data"]["answer"], "Retained λ\r\n");
    let restore = json!({"operation_id":Uuid::new_v4(),"expected":result["data"]["result"]["current"]["stamp"],"target":"active"});
    std::fs::write(&file, serde_json::to_vec(&restore).unwrap()).unwrap();
    let (code, restored) = run(
        data.path(),
        &["conversations", "restore", "--file", file_arg],
    );
    assert_eq!(code, 0, "{restored}");
    std::fs::write(&file, serde_json::to_vec(&archive).unwrap()).unwrap();
    let (code, replay) = run(
        data.path(),
        &["conversations", "archive", "--file", file_arg],
    );
    assert_eq!(code, 0, "{replay}");
    assert_eq!(
        replay["data"]["result"]["receipt"],
        result["data"]["result"]["receipt"]
    );
    assert_eq!(
        replay["data"]["result"]["current"],
        restored["data"]["result"]["current"]
    );
    let (code, active) = run(data.path(), &["conversations", "list", "--state", "all"]);
    assert_eq!(code, 0, "{active}");
    assert_eq!(active["data"]["conversations"], json!([original]));
}

#[test]
fn malformed_lifecycle_files_and_targets_refuse_before_opening_state() {
    let data = support::data_dir();
    let file = data.path().parent().unwrap().join("request.json");
    let valid = json!({"operation_id":Uuid::new_v4(),"expected":{"id":Uuid::new_v4(),"version":1},"target":"archived"});
    let mut unknown = valid.clone();
    unknown["surprise"] = json!(true);
    let mut nil = valid.clone();
    nil["operation_id"] = json!(Uuid::nil());
    let mut zero = valid.clone();
    zero["expected"]["version"] = json!(0);
    let mut overflow = valid.clone();
    overflow["expected"]["version"] = json!(u64::MAX);
    let mut wrong_target = valid.clone();
    wrong_target["target"] = json!("active");
    let mut unknown_nested = valid.clone();
    unknown_nested["expected"]["hidden"] = json!(0);
    let duplicate = format!("{{\"operation_id\":\"{}\",\"operation_id\":\"{}\",\"expected\":{{\"id\":\"{}\",\"version\":1}},\"target\":\"archived\"}}",Uuid::new_v4(),Uuid::new_v4(),Uuid::new_v4());
    for text in [
        unknown.to_string(),
        nil.to_string(),
        zero.to_string(),
        overflow.to_string(),
        wrong_target.to_string(),
        unknown_nested.to_string(),
        duplicate,
        "{bad".into(),
    ] {
        std::fs::write(&file, text).unwrap();
        let (code, failure) = run(
            data.path(),
            &["conversations", "archive", "--file", file.to_str().unwrap()],
        );
        assert_eq!(code, 2, "{failure}");
        assert!(
            !data.path().join("brn.sqlite").exists(),
            "malformed request opened work state"
        );
    }
    let (code, failure) = run(
        data.path(),
        &["conversations", "list", "--state", "discarded"],
    );
    assert_eq!(code, 2, "{failure}");
    assert!(!data.path().join("brn.sqlite").exists());
    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&file)
        .unwrap();
    file.set_len((64 * 1024 * 1024) + 1).unwrap();
    drop(file);
    let (code, failure) = run(
        data.path(),
        &[
            "conversations",
            "archive",
            "--file",
            data.path()
                .parent()
                .unwrap()
                .join("request.json")
                .to_str()
                .unwrap(),
        ],
    );
    assert_eq!(code, 2, "{failure}");
    assert!(!data.path().join("brn.sqlite").exists());
}
