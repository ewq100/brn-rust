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
        &["--headless-check", "startup"][..],
        &["--headless-check", "nope"][..],
        &["--headless-check", "completion"][..],
        &["--headless-check", "cancellation"][..],
        &["--headless-check", "stale"][..],
    ] {
        assert!(!run(args).status.success());
    }
}
#[test]
fn retired_legacy_flag_is_rejected_before_creating_directory() {
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
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown"));
    assert!(!dir.exists());
}
#[test]
fn relative_and_missing_and_file_data_paths_fail() {
    assert!(
        !run(&["--data-dir", "relative", "--headless-check", "startup"])
            .status
            .success()
    );
    assert!(
        !run(&[
            "--data-dir",
            "/definitely/no/such/brn-dir",
            "--headless-check",
            "startup"
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
            "startup"
        ])
        .status
        .success()
    );
}
#[test]
fn headless_check_starts_real_app_worker_and_reopens_current_authority() {
    let dir = std::env::temp_dir().join(format!(
        "brn-shell-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    let data = dir.join("data");
    std::fs::create_dir(&data).unwrap();
    for _ in 0..2 {
        let check = "startup";
        let o = run(&[
            "--data-dir",
            data.to_str().unwrap(),
            "--headless-check",
            check,
        ]);
        assert!(
            o.status.success(),
            "{check}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    assert!(data.join("brn.sqlite").exists());
    assert!(!data.join("brn.sqlite3").exists());
    std::fs::remove_dir_all(&dir).unwrap();
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
        "startup",
    ]);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_dir(&dir).unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not writable"));
}

#[test]
fn legacy_and_mixed_markers_refuse_without_opening_authority_or_writing_probe() {
    let base = std::env::temp_dir().join(format!("brn-desktop-refusal-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&base).unwrap();
    for marker in [
        "brn.sqlite3",
        "brn.sqlite3-wal",
        "brn.sqlite3-shm",
        "brn.sqlite3-journal",
    ] {
        let data = base.join(marker);
        std::fs::create_dir(&data).unwrap();
        let legacy = data.join(marker);
        std::fs::write(&legacy, b"synthetic old authority").unwrap();
        for mixed in [false, true] {
            if mixed {
                std::fs::write(data.join("brn.sqlite"), b"synthetic new marker").unwrap();
            }
            let output = run(&[
                "--data-dir",
                data.to_str().unwrap(),
                "--headless-check",
                "startup",
            ]);
            assert!(!output.status.success());
            assert_eq!(std::fs::read(&legacy).unwrap(), b"synthetic old authority");
            assert_eq!(
                std::fs::read_dir(&data).unwrap().count(),
                if mixed { 2 } else { 1 }
            );
        }
    }
    std::fs::remove_dir_all(base).unwrap();
}
