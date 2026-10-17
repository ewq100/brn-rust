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
    assert_ne!(wrong.code, 0);
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
