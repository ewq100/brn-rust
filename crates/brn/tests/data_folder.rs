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
