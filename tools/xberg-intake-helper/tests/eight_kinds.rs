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
        // Independent byte oracle: only the SDK's known fixture image destinations
        // may differ. Do not compute expectations by invoking project() again.
        let mut expected_projected = raw.results[0].content.clone();
        for (position, image) in raw.results[0].images.iter().flatten().enumerate() {
            if !image.data.is_empty() && matches!(image.format.as_ref(), "png" | "jpeg" | "jpg") {
                let extension = if image.format == "png" { "png" } else { "jpg" };
                let name = format!(
                    "{}.{}",
                    brn_intake::hex(&brn_intake::digest(&image.data)),
                    extension
                );
                expected_projected = expected_projected.replace(
                    &format!("](image_{position}.{})", image.format),
                    &format!("]({name})"),
                );
            }
        }
        let expected = if r.backend == "xberg_brn" && matches!(kind, "text" | "markdown") {
            std::str::from_utf8(&r.bytes).unwrap()
        } else {
            &expected_projected
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
fn options_outside_explicit_security_and_image_service_overrides_keep_defaults() {
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
        "images",
    ] {
        actual.as_object_mut().unwrap().remove(key);
        normal.as_object_mut().unwrap().remove(key);
    }
    for key in ["allow_local_file_inputs", "allow_file_uris"] {
        actual["url"].as_object_mut().unwrap().remove(key);
        normal["url"].as_object_mut().unwrap().remove(key);
    }
    assert_eq!(actual, normal, "unallocated SDK policy override");
    let images = engine::config(&r).images.unwrap();
    assert!(images.extract_images);
    assert!(!images.run_ocr_on_images);
    assert_eq!(
        images.max_images_per_page,
        Some(brn_intake::MAX_ASSETS as u32)
    );
    assert_eq!(
        serde_json::to_value(images.output_format).unwrap(),
        serde_json::json!({"type":"native"})
    );
}

#[test]
fn reference_images_relocate_without_rewriting_shared_links_or_definitions() {
    let source = "Ω ![first][a] [ordinary][a] ![a][] ![a]\n\n[a]: source.jpg\n\n`![code][a]`\n";
    assert_eq!(
        project::relocate(source, "source.jpg", "hash.jpg").unwrap(),
        "Ω ![first](hash.jpg) [ordinary][a] ![a](hash.jpg) ![a](hash.jpg)\n\n[a]: source.jpg\n\n`![code][a]`\n"
    );
    let titled = "![figure][a]\n\n[a]: source.jpg \"title\"";
    assert_eq!(
        project::relocate(titled, "source.jpg", "hash.jpg").unwrap(),
        titled
    );
    let remote = "![remote](https://example.test/source.jpg)";
    assert_eq!(
        project::relocate(remote, "source.jpg", "hash.jpg").unwrap(),
        remote
    );
}

#[test]
fn native_office_figures_preserve_source_and_encoded_asset_bytes() {
    for (kind, path, expected) in [
        (
            "docx",
            "tools/xberg-intake-helper/tests/fixtures/jpeg-figure.docx",
            vec![("jpg", "quay-map.jpg")],
        ),
        (
            "docx",
            "crates/brn-intake/tests/fixtures/harbor.docx",
            vec![("png", "water-use.png")],
        ),
        (
            "pptx",
            "crates/brn-intake/tests/fixtures/quay.pptx",
            vec![("png", "quay-map.png"), ("jpg", "quay-map.jpg")],
        ),
    ] {
        let r = request(kind, fixture(path));
        let actual = engine::extract(&r).unwrap();
        capture(&format!("retained-{kind}-{}", expected[0].0), &actual);
        assert_eq!(actual.source_sha256, brn_intake::digest(&r.bytes));
        assert_eq!(
            actual.assets.len(),
            expected.len(),
            "{path}: {:?}",
            actual.gaps
        );
        for (extension, name) in expected {
            let bytes = fixture(&format!("crates/brn-intake/tests/fixtures/{name}"));
            let hash = brn_intake::digest(&bytes);
            let asset = actual
                .assets
                .iter()
                .find(|a| a.sha256 == hash)
                .expect("exact source-native image");
            assert_eq!(asset.bytes, bytes);
            let url = format!("{}.{}", brn_intake::hex(&hash), extension);
            assert!(
                actual.markdown.contains(&format!("]({url})")),
                "{path}: {}",
                actual.markdown
            );
        }
        assert!(!actual.markdown.contains("](image_"), "{path}");
    }
}

#[test]
fn synthetic_pdf_retains_a_raster_with_its_source_hash() {
    let r = request(
        "pdf",
        fixture("tools/xberg-intake-helper/tests/fixtures/four-pixels.pdf"),
    );
    let actual = engine::extract(&r).unwrap();
    capture("retained-four-pixels-pdf", &actual);
    assert_eq!(actual.source_sha256, brn_intake::digest(&r.bytes));
    assert!(actual.markdown.contains("Synthetic retained figure"));
    assert_eq!(actual.assets.len(), 1, "{:?}", actual.gaps);
    let asset = &actual.assets[0];
    assert_eq!((asset.width, asset.height), (2, 2));
    assert!(actual.markdown.contains(&format!(
        "]({})",
        brn_intake::asset_file_name(asset).unwrap()
    )));
}

#[test]
fn actual_office_invalid_image_and_tight_image_quotas_fail_closed() {
    let mut r = request(
        "docx",
        fixture("tools/xberg-intake-helper/tests/fixtures/jpeg-figure.docx"),
    );
    let original = r.clone();
    r.limits.max_image_pixels = 1;
    assert!(engine::extract(&r).is_err());
    r = original.clone();
    r.limits.max_decoded_bytes = 16;
    assert!(engine::extract(&r).is_err());
    r = original.clone();
    r.limits.max_output_bytes = 128;
    assert!(engine::extract(&r).is_err());
    r = original.clone();
    r.bytes = fixture("crates/brn-intake/tests/fixtures/hostile-inflate.docx");
    assert!(
        engine::extract(&r).is_err(),
        "archive expansion budget must precede Office parsing"
    );
    r = original;
    r.bytes = fixture("tools/xberg-intake-helper/tests/fixtures/invalid-image.docx");
    // SDK may classify arbitrary invalid bytes as an unknown format; that must
    // produce an explicit gap, never an adopted figure or successful silent loss.
    match engine::extract(&r) {
        Err(_) => {}
        Ok(result) => {
            assert!(result.assets.is_empty());
            assert!(!result.gaps.is_empty());
        }
    }
}

#[test]
fn projection_preserves_reference_occurrences_and_reports_unsupported_images() {
    let r = request(
        "docx",
        fixture("tools/xberg-intake-helper/tests/fixtures/jpeg-figure.docx"),
    );
    let runtime = engine::runtime().unwrap();
    let mut raw = runtime
        .block_on(xberg::extract(
            xberg::ExtractInput::from_bytes(r.bytes.clone(), r.mime_type().unwrap(), None),
            &engine::config(&r),
        ))
        .unwrap()
        .results
        .remove(0);
    drop(runtime);
    raw.images.as_mut().unwrap().truncate(1); // isolate one SDK image for the reference-syntax witness
    assert_eq!(raw.images.as_ref().unwrap().len(), 1);
    let image = &raw.images.as_ref().unwrap()[0];
    let old = format!("image_0.{}", image.format);
    let expected_name = format!("{}.jpg", brn_intake::hex(&brn_intake::digest(&image.data)));
    raw.content = format!("Before Ω ![figure][ref] [ordinary][ref] ![ref][]\n\n[ref]: {old}\n");
    let result = project::project(&r, raw.clone()).unwrap();
    assert_eq!(
        result.markdown,
        format!(
            "Before Ω ![figure]({expected_name}) [ordinary][ref] ![ref]({expected_name})\n\n[ref]: {old}\n"
        )
    );
    assert_eq!(result.assets.len(), 1);
    assert!(
        !result
            .gaps
            .iter()
            .any(|gap| gap.contains("image references that could not be linked")),
        "{:?}",
        result.gaps
    );
    raw.images.as_mut().unwrap()[0].format = "webp".into();
    let result = project::project(&r, raw.clone()).unwrap();
    assert!(result.assets.is_empty());
    assert!(
        result
            .gaps
            .iter()
            .any(|gap| gap.contains("unsupported (PNG/JPEG only)"))
    );
    raw.images.as_mut().unwrap()[0].data = Vec::new().into();
    let result = project::project(&r, raw).unwrap();
    assert!(result.assets.is_empty());
    assert!(
        result
            .gaps
            .iter()
            .any(|gap| gap.contains("no retained image bytes"))
    );
}
