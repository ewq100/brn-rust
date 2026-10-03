use std::path::PathBuf;
use std::process::Command;
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_brn-desktop"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn help_is_useful() {
    let o = run(&["--help"]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("--headless-check"));
}
#[test]
fn bad_arguments_fail() {
    for a in ["--bogus", "--data-dir"] {
        assert!(!run(&[a]).status.success());
    }
    for args in [
        &["--help", "extra"][..],
        &["--help", "--help"][..],
        &["--headless-check", "completion"][..],
        &["--headless-check", "nope"][..],
    ] {
        assert!(!run(args).status.success());
    }
}
#[test]
fn legacy_vault_conflict_is_rejected_before_creating_directory() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("task7-conflict-must-not-exist");
    assert!(!dir.exists());
    let output = run(&[
        "--legacy",
        "--vault",
        "/synthetic/vault",
        "--data-dir",
        dir.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("incompatible"));
    assert!(!dir.exists());
}
#[test]
fn relative_and_missing_and_file_data_paths_fail() {
    assert!(
        !run(&["--data-dir", "relative", "--headless-check", "completion"])
            .status
            .success()
    );
    assert!(
        !run(&[
            "--data-dir",
            "/definitely/no/such/brn-dir",
            "--headless-check",
            "completion"
        ])
        .status
        .success()
    );
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    assert!(
        !run(&[
            "--data-dir",
            file.to_str().unwrap(),
            "--headless-check",
            "completion"
        ])
        .status
        .success()
    );
}
#[test]
fn headless_checks_complete_on_existing_directory() {
    let dir = std::env::temp_dir().join(format!(
        "brn-shell-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    for check in ["completion", "cancellation", "stale"] {
        let o = run(&[
            "--data-dir",
            dir.to_str().unwrap(),
            "--headless-check",
            check,
        ]);
        assert!(
            o.status.success(),
            "{check}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    std::fs::remove_dir(&dir).unwrap();
}
#[cfg(unix)]
#[test]
fn unwritable_directory_reports_error() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!(
        "brn-shell-readonly-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    let output = run(&[
        "--data-dir",
        dir.to_str().unwrap(),
        "--headless-check",
        "completion",
    ]);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_dir(&dir).unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not writable"));
}
