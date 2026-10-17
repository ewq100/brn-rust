#![cfg(any(target_os = "macos", target_os = "linux"))]
//! Owner management through the real CLI; no runtime enablement is implied.
mod support;
use support::{run, Root};

#[test]
fn installed_instruction_package_survives_cli_reopen_and_digest_bound_removal() {
    let root = Root::new();
    let data = root.workspace("data");
    let source = root.join("package");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("SKILL.md"), "---\nname: document-review\ndescription: Check support and unresolved decisions.\n---\nReview evidence; preserve owner decisions.\n").unwrap();
    let list = run(&root, true, &data, &["skills"]);
    assert_eq!(list.code, 0, "{}", list.stdout);
    let before = list.envelope();
    assert_eq!(before["result"]["skills"].as_array().unwrap().len(), 4);
    let install = run(
        &root,
        true,
        &data,
        &["skill-install", "--source", source.to_str().unwrap()],
    );
    assert_eq!(install.code, 0, "{}", install.stdout);
    let info = install.envelope()["result"].clone();
    assert_eq!(info["name"], "document-review");
    assert_eq!(info["origin"], "owner_installed");
    assert_eq!(info["enabled"], false);
    let digest = info["digest"].as_str().unwrap();
    let inspect = run(
        &root,
        true,
        &data,
        &["skill-inspect", "--name", "document-review"],
    );
    assert_eq!(inspect.code, 0, "{}", inspect.stdout);
    let inspected = inspect.envelope();
    assert_eq!(inspected["result"]["info"], info);
    assert!(inspected["result"]["files"]["SKILL.md"]
        .as_str()
        .unwrap()
        .contains("preserve owner decisions"));
    let wrong = run(
        &root,
        true,
        &data,
        &[
            "skill-remove",
            "--name",
            "document-review",
            "--digest",
            &"0".repeat(64),
        ],
    );
    assert_eq!(wrong.code, 6);
    let refusal = wrong.envelope();
    assert_eq!(refusal["status"], "rejected");
    assert_eq!(refusal["error"]["code"], "invalid_request");
    assert_eq!(
        refusal["error"]["details"]["reason"],
        "skill_digest_conflict"
    );
    let remove = run(
        &root,
        true,
        &data,
        &[
            "skill-remove",
            "--name",
            "document-review",
            "--digest",
            digest,
        ],
    );
    assert_eq!(remove.code, 0, "{}", remove.stdout);
    assert_eq!(remove.envelope()["result"]["removed"], true);
    let list = run(&root, true, &data, &["skills"]);
    assert_eq!(
        list.envelope()["result"]["skills"],
        before["result"]["skills"]
    );
}

fn package(root: &Root, data: &std::path::Path) {
    let source = root.join("package");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join("SKILL.md"),
        "---\nname: document-review\ndescription: Review evidence.\n---\nInstructions.\n",
    )
    .unwrap();
    let result = run(
        root,
        true,
        data,
        &["skill-install", "--source", source.to_str().unwrap()],
    );
    assert_eq!(result.code, 0, "{}", result.stdout);
}
#[test]
fn absent_inspect_is_not_found_and_preserves_workspace() {
    let root = Root::new();
    let data = root.workspace("data");
    let result = run(&root, true, &data, &["skill-inspect", "--name", "absent"]);
    assert_eq!(result.code, 6, "{}", result.stdout);
    let value = result.envelope();
    assert_eq!(value["status"], "rejected");
    assert_eq!(value["error"]["code"], "not_found");
    assert!(!data.join("skills").exists());
}
#[test]
fn relative_source_is_usage_before_any_workspace_io() {
    let root = Root::new();
    let data = root.join("not-created");
    let result = run(
        &root,
        true,
        &data,
        &["skill-install", "--source", "relative"],
    );
    assert_eq!(result.code, 2, "{}", result.stdout);
    let value = result.envelope();
    assert_eq!(value["status"], "usage");
    assert_eq!(value["error"]["code"], "invalid_value");
    assert_eq!(value["error"]["details"]["option"], "source");
    assert!(!data.exists());
}
#[test]
fn actual_catalog_lock_contention_is_busy_and_retryable() {
    let root = Root::new();
    let data = root.workspace("data");
    package(&root, &data);
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(data.join("skills/.lock"))
        .unwrap();
    lock.lock().unwrap();
    let result = run(&root, true, &data, &["skills"]);
    assert_eq!(result.code, 7, "{}", result.stdout);
    let value = result.envelope();
    assert_eq!(value["status"], "failed");
    assert_eq!(value["error"]["code"], "busy");
    assert_eq!(value["retry"], "read");
    drop(lock);
    assert_eq!(run(&root, true, &data, &["skills"]).code, 0);
}
#[test]
fn actual_lock_permission_denied_is_io_error_not_caller_refusal() {
    use std::os::unix::fs::PermissionsExt;
    let root = Root::new();
    let data = root.workspace("data");
    package(&root, &data);
    let path = data.join("skills/.lock");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
    let result = run(&root, true, &data, &["skills"]);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(result.code, 1, "{}", result.stdout);
    let value = result.envelope();
    assert_eq!(value["status"], "internal");
    assert_eq!(value["error"]["code"], "io_error");
    assert_eq!(run(&root, true, &data, &["skills"]).code, 0);
}
