//! The CLI names an unusable data folder in owner terms, without OS error
//! text, and still exits 1. Synthetic folders only.
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
fn assert_refused(data: &Path, reason: &str) {
    let output = command(data, &["notes"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(
        stderr.starts_with(&format!(
            "BRN cannot use the data folder {}: ",
            data.display()
        )),
        "{stderr}"
    );
    assert!(stderr.contains(reason), "{stderr}");
    assert!(!stderr.contains("os error"), "{stderr}");
}
#[test]
fn a_file_given_as_the_data_folder_is_named() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let file = root.path().join("notes.txt");
    std::fs::write(&file, b"synthetic").unwrap();
    assert_refused(&file, "is a file rather than a folder");
}
/// The evidence case: `--data-dir /data` on the sealed macOS system volume.
#[cfg(target_os = "macos")]
#[test]
fn a_data_folder_on_a_read_only_disk_is_named() {
    let data = Path::new("/").join(format!("brn-dwr-readonly-{}", uuid::Uuid::new_v4()));
    if std::fs::create_dir(&data).is_ok() {
        std::fs::remove_dir(&data).unwrap();
        eprintln!("skipped: / is writable here");
        return;
    }
    assert_refused(&data, "it is on a read-only disk");
}
/// PR #156 LOW 4: in machine mode a read on a symlinked data folder is
/// `data_folder_unusable` `symlink_refused`, also when the link points to an
/// empty folder or nowhere: never `missing_workspace`, and nothing is created.
#[cfg(unix)]
#[test]
fn a_symlink_to_an_empty_or_missing_folder_is_refused_as_a_symlink() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let empty = root.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    let to_empty = root.path().join("to-empty");
    std::os::unix::fs::symlink(&empty, &to_empty).unwrap();
    let dangling = root.path().join("dangling");
    std::os::unix::fs::symlink(root.path().join("nowhere"), &dangling).unwrap();
    // A folder below a symlinked ancestor counts too.
    let below = to_empty.join("data");
    for data in [&to_empty, &dangling, &below] {
        let output = Command::new(env!("CARGO_BIN_EXE_brn"))
            .arg("--json")
            .arg("--data-dir")
            .arg(data)
            .arg("notes")
            .output()
            .unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            envelope["error"]["code"], "data_folder_unusable",
            "{envelope}"
        );
        assert_eq!(
            envelope["error"]["details"],
            serde_json::json!({"problem": "symlink_refused"})
        );
        assert_eq!(output.status.code(), Some(6));
    }
    assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
    assert!(!root.path().join("nowhere").exists());
}
