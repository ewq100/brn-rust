#![cfg(all(feature = "helper", target_os = "macos"))]
use brn_intake::{Extraction, HelperRequest};
use std::{
    io::Write,
    process::{Command, Stdio},
};
#[test]
fn constrained_process_parses_public_payload_after_native_activation() {
    for (kind, bytes) in [
        (
            "eml",
            include_bytes!(
                "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml"
            )
            .as_slice(),
        ),
        ("pptx", include_bytes!("fixtures/quay.pptx").as_slice()),
    ] {
        let mut process = Command::new(env!("CARGO_BIN_EXE_brn-intake-helper"))
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let request = HelperRequest {
            limits: None,
            kind: kind.into(),
            bytes: bytes.to_vec(),
        };
        process
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&request).unwrap())
            .unwrap();
        let output = process.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let extraction: Extraction = serde_json::from_slice(&output.stdout).unwrap();
        extraction.validate().unwrap();
        assert_eq!(extraction.occurrences.len(), 3);
    }
}
#[test]
fn native_restriction_denies_existing_synthetic_secret_and_live_loopback() {
    let path =
        std::env::temp_dir().join(format!("brn-intake-forbidden-{}.txt", std::process::id()));
    std::fs::write(&path, b"synthetic forbidden content").unwrap();
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_brn-intake-helper"))
        .env_clear()
        .arg("--self-test-restrictions")
        .arg(&path)
        .arg(listener.local_addr().unwrap().port().to_string())
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["file_denied"], true);
    assert_eq!(result["network_denied"], true);
}

#[test]
fn owned_process_group_cancellation_joins_the_helper_and_descendants() {
    use std::{
        io::{BufRead, BufReader},
        os::unix::process::CommandExt,
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_brn-intake-helper"));
    command
        .env_clear()
        .arg("--self-test-process-tree")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut process = command.spawn().unwrap();
    let group = process.id() as i32;
    let mut output = BufReader::new(process.stdout.take().unwrap());
    let mut children = Vec::new();
    for _ in 0..3 {
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        children.push(line.trim().parse::<i32>().unwrap());
    }
    assert!(children.contains(&group));
    // SAFETY: process_group(0) makes this newly spawned child the verified group leader.
    assert_eq!(unsafe { libc::kill(-group, libc::SIGTERM) }, 0);
    assert!(!process.wait().unwrap().success());
    for _ in 0..100 {
        if unsafe { libc::kill(-group, 0) } != 0 {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("synthetic helper process group still exists after cancellation");
}
