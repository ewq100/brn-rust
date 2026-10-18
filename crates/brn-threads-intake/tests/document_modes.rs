use brn_threads_intake::*;
use std::sync::atomic::AtomicBool;
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
fn helpers() -> DocumentHelpers {
    DocumentHelpers::discover().expect("declared document helpers must resolve")
}
#[test]
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
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
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
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
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
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
#[cfg(not(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64"))))]
fn restricted_document_conversion_is_unavailable_on_this_platform() {
    let root = tempfile::tempdir().unwrap();
    let converter = Converter {
        temp_root: root.path().join("scratch"),
        limits: Default::default(),
    };
    // A bounded marker executable on unsupported Unix; never launch libtest itself.
    // On other platforms a non-executable fixture still witnesses that scratch
    // must not even be created before the platform refusal.
    let marker = root.path().join("dispatched");
    let executable = root.path().join("must-not-dispatch");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let quoted = marker.to_str().unwrap().replace('\'', "'\\''");
        std::fs::write(
            &executable,
            format!("#!/bin/sh\nprintf dispatched > '{quoted}'\nexit 91\n"),
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    #[cfg(not(unix))]
    std::fs::write(&executable, b"not an executable; must not dispatch").unwrap();
    let paths = DocumentHelpers {
        stock: executable.clone(),
        brn: executable,
    };
    for backend in ["xberg", "xberg_brn"] {
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
                    backend,
                    &AtomicBool::new(false)
                )
                .unwrap_err(),
            ConversionError::Unavailable
        );
        assert!(!converter.temp_root.exists());
        assert!(!marker.exists());
    }
}

#[test]
fn pure_validation_precedes_platform_refusal_without_scratch_or_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let mut converter = Converter {
        temp_root: root.path().join("scratch"),
        limits: Default::default(),
    };
    let paths = DocumentHelpers {
        stock: root.path().join("never-dispatch"),
        brn: root.path().join("never-dispatch-brn"),
    };
    let request = || ConversionRequest {
        kind: DocumentKind::Text,
        bytes: b"Synthetic text",
        intent: ImportIntent::FullNote,
        source_reference: "synthetic://validation",
    };
    for backend in ["xberg", "xberg_brn"] {
        assert_eq!(
            converter
                .convert_document(request(), &paths, backend, &AtomicBool::new(true))
                .unwrap_err(),
            ConversionError::Cancelled
        );
        let empty = ConversionRequest {
            bytes: b"",
            ..request()
        };
        assert_eq!(
            converter
                .convert_document(empty, &paths, backend, &AtomicBool::new(false))
                .unwrap_err(),
            ConversionError::Budget
        );
        let invalid = ConversionRequest {
            source_reference: "",
            ..request()
        };
        assert_eq!(
            converter
                .convert_document(invalid, &paths, backend, &AtomicBool::new(false))
                .unwrap_err(),
            ConversionError::Invalid
        );
    }
    assert_eq!(
        converter
            .convert_document(request(), &paths, "unknown", &AtomicBool::new(false))
            .unwrap_err(),
        ConversionError::Protocol
    );
    converter.limits.max_input_bytes = 0;
    assert_eq!(
        converter
            .convert_document(request(), &paths, "xberg", &AtomicBool::new(true))
            .unwrap_err(),
        ConversionError::Budget
    );
    assert!(!converter.temp_root.exists());
}
