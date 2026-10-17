#![cfg(all(feature = "helper", target_os = "macos"))]
use brn_intake::{Extraction, HelperRequest};
use std::{
    io::Write,
    process::{Command, Stdio},
};
#[test]
fn constrained_process_parses_public_payload_after_native_activation() {
    for (kind, bytes) in [
        ("eml", include_bytes!("fixtures/plural.eml").as_slice()),
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

/// Send one request to the real restricted helper (empty environment, piped protocol).
fn restricted_html(bytes: &[u8]) -> std::process::Output {
    let mut process = Command::new(env!("CARGO_BIN_EXE_brn-intake-helper"))
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let request = HelperRequest {
        limits: None,
        kind: "html".into(),
        bytes: bytes.to_vec(),
    };
    process
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    process.wait_with_output().unwrap()
}

#[test]
fn restricted_helper_converts_textual_html_with_real_xberg() {
    let html = include_bytes!("fixtures/textual.html");
    let output = restricted_html(html);
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let extraction: Extraction = serde_json::from_slice(&output.stdout).unwrap();
    extraction.validate().unwrap();
    assert_eq!(extraction.converter, brn_intake::HTML_CONVERTER);
    assert_eq!(extraction.sources[0].bytes, html);
    assert_eq!(extraction.original_sha256, brn_intake::digest(html));
    assert_eq!(extraction.sources[0].media_type, "text/html");
    let mut cursor = 0;
    for fact in [
        "# Tide observations",
        "Gezeitenstr\u{f6}me",
        "[station notes](https://example.test/notes#tides)",
        "## Readings",
        "- morning high",
        "- evening low",
        "> Readings are provisional.",
        "```",
        "level = 3.2",
        "Last line.",
    ] {
        let offset = extraction.markdown[cursor..]
            .find(fact)
            .unwrap_or_else(|| panic!("missing {fact:?} in {:?}", extraction.markdown));
        cursor += offset + fact.len();
    }
}

#[test]
fn restricted_helper_refuses_rich_html_before_backend_with_static_labels() {
    for html in [
        b"<table><tr><td>x</td></tr></table>".as_slice(),
        b"<p><img src=\"https://x.test/a.png\"></p>",
        b"< img src=\"x\">",
        b"<p>a<p>b",
    ] {
        let output = restricted_html(html);
        assert!(!output.status.success());
        let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let error = response["error"].as_str().unwrap();
        assert!(error.starts_with("HTML admission:"), "{error}");
        assert!(!error.contains("x.test"));
    }
}

#[test]
fn restricted_helper_large_html_takes_the_joined_thread_path() {
    let paragraph = "<p>tide gauge reading steady</p>\n";
    let count = 512 * 1024 / paragraph.len() + 64;
    let html = format!("{}<p>final marker</p>", paragraph.repeat(count));
    assert!(html.len() >= 512 * 1024);
    let started = std::time::Instant::now();
    let output = restricted_html(html.as_bytes());
    let elapsed = started.elapsed();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let extraction: Extraction = serde_json::from_slice(&output.stdout).unwrap();
    extraction.validate().unwrap();
    assert_eq!(
        extraction
            .markdown
            .matches("tide gauge reading steady")
            .count(),
        count
    );
    // Measurement only (no hard RSS claim): the waited children's peak resident size.
    let mut usage = unsafe { std::mem::zeroed::<libc::rusage>() };
    assert_eq!(
        unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage) },
        0
    );
    eprintln!(
        "large-html input_bytes={} elapsed_ms={} children_max_rss_bytes={}",
        html.len(),
        elapsed.as_millis(),
        usage.ru_maxrss
    );
}
