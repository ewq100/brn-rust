use brn_threads_intake::*;
use std::sync::atomic::AtomicBool;
#[cfg(target_os = "macos")]
fn helpers() -> DocumentHelpers {
    DocumentHelpers::discover().expect("declared document helpers must resolve")
}
#[test]
#[cfg(target_os = "macos")]
fn both_helpers_all_eight_use_checked_identity_and_leave_no_scratch() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let converter = Converter {
        temp_root: root.path().join("scratch"),
        limits: brn_intake::IntakeLimits {
            wall_time_ms: 20000,
            ..Default::default()
        },
    };
    for backend in ["xberg", "xberg_brn"] {
        for (kind, bytes) in [
            (
                DocumentKind::Pdf,
                include_bytes!("fixtures/harbor-study.pdf").as_slice(),
            ),
            (
                DocumentKind::Docx,
                include_bytes!("../../brn-intake/tests/fixtures/harbor.docx").as_slice(),
            ),
            (
                DocumentKind::Pptx,
                include_bytes!("../../brn-intake/tests/fixtures/quay.pptx").as_slice(),
            ),
            (
                DocumentKind::Xlsx,
                include_bytes!("../../brn-intake/tests/fixtures/forecast.xlsx").as_slice(),
            ),
            (
                DocumentKind::Eml,
                include_bytes!("../../brn-intake/tests/fixtures/single.eml").as_slice(),
            ),
            (DocumentKind::Html, b"<p>Synthetic HTML</p>".as_slice()),
            (DocumentKind::Text, b"Synthetic text\r\n".as_slice()),
            (DocumentKind::Markdown, b"# Synthetic Markdown\n".as_slice()),
        ] {
            let result = converter
                .convert_document(
                    ConversionRequest {
                        kind,
                        bytes,
                        intent: ImportIntent::FullNote,
                        source_reference: "synthetic://task2",
                    },
                    &helpers(),
                    backend,
                    &AtomicBool::new(false),
                )
                .unwrap();
            assert_eq!(result.source_sha256, brn_intake::digest(bytes));
            assert_eq!(
                result.converter,
                document_conversion_label(backend, false).unwrap()
            );
            assert_eq!(result.coverage.status, CoverageStatus::Unchecked);
            assert!(!result.markdown.is_empty());
            assert_eq!(std::fs::read_dir(&converter.temp_root).unwrap().count(), 0);
        }
    }
}
#[test]
#[cfg(target_os = "macos")]
fn wrong_backend_missing_helper_and_cancel_cannot_fall_back() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let converter = Converter {
        temp_root: root.path().join("scratch"),
        limits: Default::default(),
    };
    let request = || ConversionRequest {
        kind: DocumentKind::Text,
        bytes: b"Source",
        intent: ImportIntent::FullNote,
        source_reference: "synthetic://task2",
    };
    let mut paths = helpers();
    paths.brn = paths.stock.clone();
    assert!(matches!(
        converter.convert_document(request(), &paths, "xberg_brn", &AtomicBool::new(false)),
        Err(ConversionError::Invalid)
    ));
    paths.stock = "/missing/task2-helper".into();
    assert!(matches!(
        converter.convert_document(request(), &paths, "xberg", &AtomicBool::new(false)),
        Err(ConversionError::Unavailable)
    ));
    assert!(matches!(
        converter.convert_document(request(), &helpers(), "unknown", &AtomicBool::new(false)),
        Err(ConversionError::Protocol)
    ));
    assert!(matches!(
        converter.convert_document(request(), &helpers(), "xberg", &AtomicBool::new(true)),
        Err(ConversionError::Cancelled)
    ));
}

#[test]
#[cfg(target_os = "macos")]
fn successful_process_with_wrong_response_identity_is_rejected_and_limits_are_enforced() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut converter = Converter {
        temp_root: root.path().join("scratch"),
        limits: Default::default(),
    };
    let request = || ConversionRequest {
        kind: DocumentKind::Text,
        bytes: b"fixed bytes",
        intent: ImportIntent::FullNote,
        source_reference: "synthetic://protocol",
    };
    let response = brn_intake::document::DocumentExtraction {
        schema: 1,
        backend: "xberg".into(),
        engine_identity: expected_engine_identity("xberg").unwrap(),
        source_sha256: brn_intake::digest(b"fixed bytes"),
        markdown: "Fixed result".into(),
        assets: vec![],
        gaps: vec![],
    };
    let path = root.path().join("synthetic-helper");
    let paths = DocumentHelpers {
        stock: path.clone(),
        brn: path.clone(),
    };
    for kind in 0..5 {
        let mut value = serde_json::to_value(&response).unwrap();
        match kind {
            0 => {}
            1 => value["schema"] = serde_json::json!(2),
            2 => value["backend"] = serde_json::json!("xberg_brn"),
            3 => value["source_sha256"] = serde_json::json!(vec![0; 32]),
            _ => value["engine_identity"] = serde_json::json!("unknown-bundle"),
        }
        let json = serde_json::to_string(&value).unwrap();
        let quoted = json.replace('\'', "'\\''");
        std::fs::write(
            &path,
            format!("#!/bin/sh\n/bin/cat >/dev/null\nprintf '%s' '{quoted}'\n"),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let actual =
            converter.convert_document(request(), &paths, "xberg", &AtomicBool::new(false));
        if kind == 0 {
            assert_eq!(actual.unwrap().markdown, "Fixed result")
        } else {
            assert_eq!(actual.unwrap_err(), ConversionError::Protocol)
        }
        assert_eq!(std::fs::read_dir(&converter.temp_root).unwrap().count(), 0);
    }
    converter.limits.max_input_bytes = 1;
    assert_eq!(
        converter
            .convert_document(request(), &paths, "xberg", &AtomicBool::new(false))
            .unwrap_err(),
        ConversionError::Budget
    );
    converter.limits = brn_intake::IntakeLimits {
        max_package_parts: 0,
        ..Default::default()
    };
    assert_eq!(
        converter
            .convert_document(request(), &paths, "xberg", &AtomicBool::new(false))
            .unwrap_err(),
        ConversionError::Budget
    );
}

#[test]
#[cfg(not(target_os = "macos"))]
fn restricted_document_conversion_is_unavailable_on_this_platform() {
    let root = tempfile::tempdir().unwrap();
    let converter = Converter {
        temp_root: root.path().join("scratch"),
        limits: Default::default(),
    };
    let paths = DocumentHelpers {
        stock: std::env::current_exe().unwrap(),
        brn: std::env::current_exe().unwrap(),
    };
    assert_eq!(
        converter
            .convert_document(
                ConversionRequest {
                    kind: DocumentKind::Text,
                    bytes: b"Synthetic text",
                    intent: ImportIntent::FullNote,
                    source_reference: "synthetic://unavailable",
                },
                &paths,
                "xberg",
                &AtomicBool::new(false)
            )
            .unwrap_err(),
        ConversionError::Unavailable
    );
}
