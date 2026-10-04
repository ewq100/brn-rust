use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
mod support;

const SOURCE_ID: &str = "11111111-1111-4111-8111-111111111111";
const TARGET_ID: &str = "22222222-2222-4222-8222-222222222222";

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

fn vault(data: &Path) -> (PathBuf, String) {
    let vault = data.parent().unwrap().join("vault");
    fs::create_dir(&vault).unwrap();
    let text=format!("\u{feff}---\r\nbrn_id: {SOURCE_ID}\r\ncustom: '[ignored](bad.md)'\r\n...\r\n[stable](brn://note/{TARGET_ID})\r\n[path](target.md)\r\n");
    fs::write(vault.join("current.md"), &text).unwrap();
    fs::write(
        vault.join("target.md"),
        format!("---\nbrn_id: {TARGET_ID}\n---\nTarget õ\r\n"),
    )
    .unwrap();
    (vault, text)
}

#[test]
fn cli_projects_exact_saved_link_proofs_without_changing_notes() {
    let data = support::data_dir();
    let (vault, text) = vault(data.path());
    let (code, output) = run(
        data.path(),
        &[
            "links",
            "show",
            "current.md",
            "--vault",
            vault.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{output}");
    assert_eq!(output["command"], "links.show");
    assert_eq!(output["data"]["source"]["note_id"], SOURCE_ID);
    assert_eq!(output["data"]["source_outcome"], "unique");
    let links = output["data"]["links"].as_array().unwrap();
    assert_eq!(links.len(), 2);
    for link in links {
        assert_eq!(link["outcome"], "resolved");
        assert_eq!(link["matches"][0]["note_id"], TARGET_ID);
        for evidence in link["evidence"].as_array().unwrap() {
            let start = evidence["start_byte"].as_u64().unwrap() as usize;
            let end = evidence["end_byte"].as_u64().unwrap() as usize;
            assert_eq!(&text[start..end], evidence["quote"].as_str().unwrap());
        }
    }
    assert_eq!(fs::read(vault.join("current.md")).unwrap(), text.as_bytes());
}

#[test]
fn cli_uuid_links_survive_move_and_index_rebuild_while_duplicate_resolution_refuses_guessing() {
    let data = support::data_dir();
    let (vault, text) = vault(data.path());
    assert_eq!(
        run(
            data.path(),
            &[
                "links",
                "show",
                "current.md",
                "--vault",
                vault.to_str().unwrap()
            ]
        )
        .0,
        0
    );
    fs::rename(vault.join("target.md"), vault.join("moved.md")).unwrap();
    fs::remove_file(data.path().join("index.sqlite")).unwrap();
    let (code, moved) = run(data.path(), &["links", "show", "current.md"]);
    assert_eq!(code, 0, "{moved}");
    assert_eq!(moved["data"]["links"][0]["outcome"], "resolved");
    assert_eq!(moved["data"]["links"][0]["target_path"], "moved.md");
    assert_eq!(moved["data"]["links"][1]["outcome"], "absent");
    fs::copy(vault.join("moved.md"), vault.join("duplicate.md")).unwrap();
    let (code, duplicate) = run(data.path(), &["links", "show", "current.md"]);
    assert_eq!(code, 0, "{duplicate}");
    assert_eq!(duplicate["data"]["links"][0]["outcome"], "ambiguous");
    assert!(duplicate["data"]["links"][0]["target_path"].is_null());
    assert_eq!(
        duplicate["data"]["links"][0]["matches"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(fs::read(vault.join("current.md")).unwrap(), text.as_bytes());
}

#[test]
fn invalid_link_commands_fail_before_opening_operational_or_credential_storage() {
    let data = support::data_dir();
    let credentials = data.path().parent().unwrap().join(format!(
        "{}-links.credentials",
        data.path().file_name().unwrap().to_str().unwrap()
    ));
    for args in [
        &["links", "show"][..],
        &["links", "show", "../outside.md"],
        &["links", "show", ".hidden.md"],
        &["links", "show", "current.md", "extra"],
        &["links", "unknown", "current.md"],
    ] {
        let mut args = args.to_vec();
        args.extend(["--credentials-dir", credentials.to_str().unwrap()]);
        let (code, output) = run(data.path(), &args);
        assert_eq!(code, 2, "{output}");
        assert_eq!(fs::read_dir(data.path()).unwrap().count(), 0);
        assert!(!credentials.exists());
    }
}
