#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
use std::{
    io::Write,
    process::{Command, Stdio},
};
#[path = "../src/engine.rs"]
mod engine;
#[path = "../src/project.rs"]
mod project;
use brn_intake::{
    IntakeLimits,
    document::{DocumentExtraction, DocumentRequest},
};
use std::path::PathBuf;
fn request(kind: &str, bytes: Vec<u8>) -> DocumentRequest {
    DocumentRequest {
        schema: 1,
        backend: env!("BRN_BACKEND").into(),
        kind: kind.into(),
        limits: IntakeLimits::default(),
        bytes,
    }
}
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(PathBuf::from(env!("BRN_REPO_ROOT")).join(name)).unwrap()
}
fn capture(name: &str, value: impl serde::Serialize) {
    if let Some(root) = std::env::var_os("BRN_TASK1_CAPTURES") {
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            PathBuf::from(root).join(format!("{name}.json")),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn eight_kinds_equal_normal_public_sdk() {
    for (kind, path) in [
        (
            "pdf",
            "crates/brn-threads-intake/tests/fixtures/harbor-study.pdf",
        ),
        ("docx", "crates/brn-intake/tests/fixtures/harbor.docx"),
        ("pptx", "crates/brn-intake/tests/fixtures/quay.pptx"),
        ("xlsx", "crates/brn-intake/tests/fixtures/forecast.xlsx"),
        ("eml", "crates/brn-intake/tests/fixtures/single.eml"),
        ("html", "crates/brn-intake/tests/fixtures/textual.html"),
        ("text", ""),
        ("markdown", ""),
    ] {
        let bytes = if path.is_empty() {
            b"Synthetic source".to_vec()
        } else {
            fixture(path)
        };
        let r = request(kind, bytes);
        let runtime = engine::runtime().unwrap();
        let raw = runtime
            .block_on(xberg::extract(
                xberg::ExtractInput::from_bytes(r.bytes.clone(), r.mime_type().unwrap(), None),
                &engine::config(&r),
            ))
            .unwrap();
        drop(runtime);
        capture(&format!("{kind}-sdk"), &raw);
        assert!(raw.errors.is_empty(), "{kind}: {:?}", raw.errors);
        assert_eq!(raw.results.len(), 1, "{kind}");
        let actual: DocumentExtraction = engine::extract(&r).unwrap();
        capture(&format!("{kind}-envelope"), &actual);
        // These ordinary fixtures return no retained bytes with images disabled.
        assert!(
            actual.assets.is_empty(),
            "unexpected asset fixture, retain raw and add exact relocation expectation"
        );
        let expected = if r.backend == "xberg_brn" && matches!(kind, "text" | "markdown") {
            std::str::from_utf8(&r.bytes).unwrap()
        } else {
            &raw.results[0].content
        };
        assert_eq!(
            actual.markdown, expected,
            "normal SDK/explicit identity {kind}"
        );
        actual.validate_for(&r).unwrap();
        #[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
        {
            let out = helper(&r);
            assert!(
                out.status.success(),
                "restricted {kind}: {}",
                String::from_utf8_lossy(&out.stdout)
            );
            let restricted: DocumentExtraction = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(restricted, actual, "restricted {kind}");
        }
    }
}
#[test]
fn nine_utf8_vectors_have_only_explicit_brn_identity_adjustment() {
    let vectors = [
        ("text", " Alpha Ω\r\n\r\nBeta\r\n"),
        ("text", "\u{feff}Café e\u{301} Ω\n"),
        (
            "text",
            "# Literal heading\n\n[link](https://example.test) <b>literal</b>\n",
        ),
        ("text", "No final newline Ω"),
        ("text", "\n\nBody\n\n\n"),
        (
            "markdown",
            "# Title\r\n\r\nA [link](https://example.test).\r\n",
        ),
        (
            "markdown",
            "---\ntitle: Ω\n---\n\n```rust\nlet x = \"<tag>\";\n```\n\n![alt](image.png)\n\n<b>Literal HTML</b>\n",
        ),
        ("markdown", "\u{feff}# Café Ω\n\nTail\n"),
        ("markdown", "\n\n# Title\n\n\nTail\n\n"),
    ];
    for (i, (kind, text)) in vectors.into_iter().enumerate() {
        let r = request(kind, text.as_bytes().to_vec());
        let actual: DocumentExtraction = engine::extract(&r).unwrap();
        let runtime = engine::runtime().unwrap();
        let raw = runtime
            .block_on(xberg::extract(
                xberg::ExtractInput::from_bytes(r.bytes.clone(), r.mime_type().unwrap(), None),
                &engine::config(&r),
            ))
            .unwrap();
        drop(runtime);
        let expected = if r.backend == "xberg_brn" {
            text
        } else {
            &raw.results[0].content
        };
        assert_eq!(actual.markdown, expected, "vector {i}");
        capture(&format!("utf8-{i}"), &actual);
    }
}
#[test]
fn actual_office_bundle_difference_controls() {
    for (name, kind, path) in [
        (
            "prefix",
            "docx",
            "crates/brn-intake/tests/fixtures/prefix-variation.docx",
        ),
        ("quay", "pptx", "crates/brn-intake/tests/fixtures/quay.pptx"),
    ] {
        let r = request(kind, fixture(path));
        let result = engine::extract(&r);
        capture(&format!("difference-{name}"), &result);
        if name == "prefix" && r.backend == "xberg" {
            let runtime = engine::runtime().unwrap();
            let raw = runtime
                .block_on(xberg::extract(
                    xberg::ExtractInput::from_bytes(r.bytes.clone(), r.mime_type().unwrap(), None),
                    &engine::config(&r),
                ))
                .unwrap();
            drop(runtime);
            capture("difference-prefix-empty-sdk", &raw);
            assert!(raw.errors.is_empty());
            assert_eq!(raw.results.len(), 1);
            assert_eq!(raw.results[0].content, "");
            assert_eq!(result.unwrap_err(), "document text budget or empty result");
        } else {
            let actual = result.unwrap();
            if name == "prefix" {
                assert!(actual.markdown.contains("Harbor water-saving pilot"));
            } else {
                // Quay has two source-identical opening sentences; only slide9 has
                // the authored inspection item. Declared order puts inspection before
                // the second opening sentence, numeric stock order puts it after.
                let openings: Vec<_> = actual
                    .markdown
                    .match_indices("Dock review is pending.")
                    .map(|(i, _)| i)
                    .collect();
                assert_eq!(openings.len(), 2);
                let inspection = actual
                    .markdown
                    .find("Inspect pier A")
                    .expect("slide9 content");
                assert_eq!(inspection < openings[1], r.backend == "xberg_brn");
            }
        }
    }
}
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
fn helper(r: &DocumentRequest) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn-xberg-helper"))
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(r).unwrap())
        .unwrap();
    child.wait_with_output().unwrap()
}
#[test]
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
fn restricted_protocol_and_backend_identity() {
    let r = request("text", b"Synthetic restricted text".to_vec());
    let out = helper(&r);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let actual: DocumentExtraction = serde_json::from_slice(&out.stdout).unwrap();
    actual.validate_for(&r).unwrap();
    for i in 0..4 {
        let mut bad = r.clone();
        match i {
            0 => {
                bad.backend = if r.backend == "xberg" {
                    "xberg_brn"
                } else {
                    "xberg"
                }
                .into()
            }
            1 => bad.schema = 99,
            2 => bad.kind = "rtf".into(),
            _ => bad.limits.max_input_bytes = 1,
        }
        let out = helper(&bad);
        assert!(!out.status.success());
        assert!(
            serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap()["error"].is_string()
        );
    }
    let out = Command::new(env!("CARGO_BIN_EXE_brn-xberg-helper"))
        .env_clear()
        .arg("--prepare")
        .output()
        .unwrap();
    assert!(!out.status.success());
}
#[test]
fn asset_relocation_changes_only_exact_image_destination() {
    let raw = "A source.png prose [link](source.png) ![first](source.png) ![second](source.png)\n";
    assert_eq!(
        project::relocate(raw, "source.png", "hash.png").unwrap(),
        "A source.png prose [link](source.png) ![first](hash.png) ![second](hash.png)\n"
    );
}

#[test]
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
fn unrelated_environment_is_refused_alone_and_with_cf_marker() {
    for cf in [false, true] {
        let r = request("text", b"Must not extract".to_vec());
        let mut command = Command::new(env!("CARGO_BIN_EXE_brn-xberg-helper"));
        command
            .env_clear()
            .env("BRN_SYNTHETIC_INHERITED_SENTINEL", "synthetic")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        if cf {
            command.env("__CF_USER_TEXT_ENCODING", "synthetic");
        }
        let mut child = command.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&r).unwrap())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(!out.status.success());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap(),
            serde_json::json!({"error":"helper requires an empty inherited environment"})
        );
    }
}

#[test]
fn all_non_security_sdk_options_keep_published_defaults() {
    let r = request("text", b"policy witness".to_vec());
    let mut actual = serde_json::to_value(engine::config(&r)).unwrap();
    let mut normal = serde_json::to_value(xberg::ExtractionConfig::default()).unwrap();
    // Exact common host/execution/service overrides. Everything else, including
    // quality processing, postprocessor, text preprocessing and HTML options, must
    // remain the published default rather than an O1 measurement profile.
    for key in [
        "use_cache",
        "extraction_timeout_secs",
        "output_format",
        "max_concurrent_extractions",
        "concurrency",
        "disable_ocr",
        "ocr_embedded_images",
        "qr_codes",
        "max_archive_depth",
        "max_embedded_file_bytes",
        "security_limits",
    ] {
        actual.as_object_mut().unwrap().remove(key);
        normal.as_object_mut().unwrap().remove(key);
    }
    for key in ["allow_local_file_inputs", "allow_file_uris"] {
        actual["url"].as_object_mut().unwrap().remove(key);
        normal["url"].as_object_mut().unwrap().remove(key);
    }
    assert_eq!(actual, normal, "unallocated SDK policy override");
}
