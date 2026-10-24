use brn_intake::document::{DocumentExtraction, DocumentRequest, expected_engine_identity};
use brn_intake::{IntakeLimits, digest};
fn request() -> DocumentRequest {
    DocumentRequest {
        schema: 1,
        backend: env!("BRN_BACKEND").into(),
        kind: "text".into(),
        limits: IntakeLimits::default(),
        bytes: b"Synthetic text".to_vec(),
    }
}
fn result(r: &DocumentRequest) -> DocumentExtraction {
    DocumentExtraction {
        schema: 1,
        backend: r.backend.clone(),
        engine_identity: expected_engine_identity(&r.backend).unwrap(),
        source_sha256: digest(&r.bytes),
        markdown: "Synthetic text".into(),
        assets: vec![],
        gaps: vec![],
    }
}
#[test]
fn envelope_identity_and_finite_limits() {
    let r = request();
    let good = result(&r);
    good.validate_for(&r).unwrap();
    for i in 0..8 {
        let mut r = r.clone();
        match i {
            0 => r.schema = 2,
            1 => r.backend = "legacy".into(),
            2 => r.kind = "rtf".into(),
            3 => r.limits.wall_time_ms = 0,
            4 => r.limits.max_input_bytes += 1,
            5 => r.limits.max_output_bytes = 1,
            6 => r.bytes.clear(),
            _ => r.limits.max_image_pixels = u64::MAX,
        }
        assert!(good.validate_for(&r).is_err(), "case {i}");
    }
    for i in 0..9 {
        let mut bad = good.clone();
        match i {
            0 => bad.schema = 2,
            1 => bad.backend = "wrong".into(),
            2 => bad.engine_identity = "unbound".into(),
            3 => bad.source_sha256 = [0; 32],
            4 => bad.markdown = "x".repeat(brn_intake::MAX_TEXT_BYTES + 1),
            5 => bad.gaps = vec!["gap".into(); brn_intake::MAX_OCCURRENCES + 1],
            6 => bad.gaps.push("bad\0gap".into()),
            7 => bad.gaps.push("x".repeat(4097)),
            _ => bad.markdown.clear(),
        }
        assert!(bad.validate_for(&r).is_err(), "case {i}");
    }
    assert_ne!(
        expected_engine_identity("xberg").unwrap(),
        expected_engine_identity("xberg_brn").unwrap()
    );
    assert!(serde_json::from_str::<DocumentRequest>(r#"{"schema":1,"backend":"xberg","kind":"text","limits":{"wall_time_ms":1e999},"bytes":[65]}"#).is_err());
}
#[test]
fn invalid_assets_are_refused() {
    let r = request();
    let mut bad = result(&r);
    bad.assets.push(brn_intake::ImageAsset {
        id: "asset-wrong".into(),
        sha256: [0; 32],
        width: 1,
        height: 1,
        media_type: "image/png".into(),
        bytes: vec![1],
    });
    assert!(bad.validate_for(&r).is_err());
    bad.assets = vec![bad.assets[0].clone(); brn_intake::MAX_ASSETS + 1];
    assert!(bad.validate_for(&r).is_err());
}

#[test]
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
fn shared_runtime_denies_file_and_network() {
    use std::process::Command;
    let path = std::env::temp_dir().join(format!("document-forbidden-{}", std::process::id()));
    std::fs::write(&path, b"synthetic forbidden").unwrap();
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_brn-xberg-helper"))
        .env_clear()
        .arg("--self-test-restrictions")
        .arg(&path)
        .arg(listener.local_addr().unwrap().port().to_string())
        .output()
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({"file_denied":true,"network_denied":true})
    );
}
#[test]
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
fn shared_runtime_process_group_is_reaped() {
    use std::{
        io::{BufRead, BufReader},
        os::unix::process::CommandExt,
        process::{Command, Stdio},
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn-xberg-helper"))
        .env_clear()
        .arg("--self-test-process-tree")
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .unwrap();
    let group = child.id() as i32;
    let mut lines = BufReader::new(child.stdout.take().unwrap());
    let mut children = Vec::new();
    for _ in 0..3 {
        let mut line = String::new();
        lines.read_line(&mut line).unwrap();
        children.push(line.trim().parse::<i32>().unwrap());
    }
    assert!(children.contains(&group));
    // SAFETY: the just-spawned child leads this private process group.
    assert_eq!(unsafe { libc::kill(-group, libc::SIGTERM) }, 0);
    assert!(!child.wait().unwrap().success());
    for _ in 0..100 {
        if unsafe { libc::kill(-group, 0) } != 0 {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("document helper process group survived cancellation");
}

#[test]
fn checked_png_identity_dimensions_duplicates_and_request_pixel_budget() {
    let r = request();
    let mut good = result(&r);
    let bytes = include_bytes!(concat!(
        env!("BRN_REPO_ROOT"),
        "/crates/brn-intake/tests/fixtures/water-use.png"
    ))
    .to_vec();
    let hash = digest(&bytes);
    // Fixed public fixture's PNG IHDR/source inventory, not converter output.
    let asset = brn_intake::ImageAsset {
        id: format!("asset-{}", brn_intake::hex(&hash)),
        sha256: hash,
        width: 900,
        height: 500,
        media_type: "image/png".into(),
        bytes,
    };
    good.assets.push(asset.clone());
    good.validate_for(&r).unwrap();
    let mut bad = good.clone();
    bad.assets.push(asset);
    assert!(bad.validate_for(&r).is_err());
    let mut bad = good.clone();
    bad.assets[0].width += 1;
    assert!(bad.validate_for(&r).is_err());
    let mut bad = good.clone();
    bad.assets[0].bytes.push(0);
    bad.assets[0].sha256 = digest(&bad.assets[0].bytes);
    bad.assets[0].id = format!("asset-{}", brn_intake::hex(&bad.assets[0].sha256));
    assert!(bad.validate_for(&r).is_err());
    let mut r = r.clone();
    r.limits.max_image_pixels = 1;
    assert!(good.validate_for(&r).is_err());
    r.limits.max_image_pixels = brn_intake::MAX_IMAGE_PIXELS;
    r.limits.max_total_image_pixels = 1;
    assert!(good.validate_for(&r).is_err());
}

#[test]
#[cfg(not(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64"))))]
fn restricted_runtime_reports_platform_unavailable() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_brn-xberg-helper"))
        .env_clear()
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({"error":"native intake restrictions require macOS or Linux x86_64; conversion refused"})
    );
}

#[test]
fn prior_image_service_profile_cannot_impersonate_the_current_helper() {
    let r = request();
    let mut old = result(&r);
    old.engine_identity = old
        .engine_identity
        .strip_suffix("/figures-v1")
        .unwrap()
        .into();
    assert!(old.validate_for(&r).is_err());
}
