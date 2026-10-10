#![cfg(target_os = "macos")]
use brn_threads_intake::*;
use std::{path::PathBuf, sync::atomic::AtomicBool};

fn converter(temp_root: PathBuf) -> Converter {
    Converter {
        helper_path: PathBuf::from(
            std::env::var_os("BRN_INTAKE_HELPER").unwrap_or_else(||std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("brn-intake-helper").into_os_string()),
        ),
        pdf_tools: Some(PdfTools {
            pdftotext: PathBuf::from(
                std::env::var_os("BRN_PDFTOTEXT").unwrap_or_else(||"/opt/homebrew/bin/pdftotext".into()),
            ),
            pdfimages: PathBuf::from(
                std::env::var_os("BRN_PDFIMAGES").unwrap_or_else(||"/opt/homebrew/bin/pdfimages".into()),
            ),
        }),
        temp_root,
        limits: brn_intake::IntakeLimits {
            wall_time_ms: 10_000,
            ..Default::default()
        },
    }
}
fn fixture(kind: DocumentKind) -> (&'static [u8], SourceInventory) {
    let (bytes, inventory): (&[u8], &[u8]) = match kind {
        DocumentKind::Pdf => (
            include_bytes!("fixtures/harbor-study.pdf"),
            include_bytes!("fixtures/harbor-study.inventory.json"),
        ),
        DocumentKind::Docx => (
            include_bytes!("fixtures/harbor-process.docx"),
            include_bytes!("fixtures/harbor-process.inventory.json"),
        ),
        _ => panic!("ordinary document fixture only"),
    };
    (bytes, serde_json::from_slice(inventory).unwrap())
}
#[test]
fn ordinary_pdf_and_docx_preserve_checked_content_and_assets_without_original_payload() {
    for kind in [DocumentKind::Pdf, DocumentKind::Docx] {
        let root = tempfile::tempdir().unwrap();
        let converter = converter(root.path().join("transient"));
        let (bytes, inventory) = fixture(kind);
        let result = converter
            .convert(
                ConversionRequest {
                    kind,
                    intent: ImportIntent::FullNote,
                    bytes,
                    source_reference: "synthetic://harbor/source",
                    inventory: Some(&inventory),
                },
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(
            result.coverage.status,
            CoverageStatus::Complete,
            "{kind:?}: {:?}",
            result.coverage.gaps
        );
        assert_eq!(result.coverage.checked_items.len(), inventory.items.len());
        assert_eq!(result.assets.len(), 1);
        assert_eq!(result.source_sha256, brn_intake::digest(bytes));
        assert_eq!(std::fs::read_dir(&converter.temp_root).unwrap().count(), 0);
        let serialized = serde_json::to_value(&result).unwrap();
        assert!(serialized.get("original").is_none());
        assert!(
            serialized["sources"]
                .as_array()
                .unwrap()
                .iter()
                .all(|source| source.get("bytes").is_none())
        );
        let asset = &result.assets[0];
        assert_eq!((asset.width, asset.height), (720, 200));
        assert_eq!(brn_intake::digest(&asset.bytes), asset.sha256);
        let reader = png::Decoder::new(std::io::Cursor::new(&asset.bytes))
            .read_info()
            .unwrap();
        assert_eq!(reader.info().width, 720);
        let headings: &[&str] = match kind {
            DocumentKind::Pdf => &[
                "1. Purpose",
                "2. Findings",
                "3. Limits",
                "4. References",
                "Appendix A.",
            ],
            DocumentKind::Docx => &[
                "Purpose and roles",
                "Prerequisites",
                "Procedure",
                "Gate responsibilities",
                "Exceptions",
                "References",
                "Appendix A.",
            ],
            _ => panic!("ordinary document fixture only"),
        };
        let positions: Vec<_> = headings
            .iter()
            .map(|heading| result.markdown.find(heading).unwrap())
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
#[test]
fn deliberately_omitted_figure_and_unchecked_full_note_are_partial() {
    for kind in [DocumentKind::Pdf, DocumentKind::Docx] {
        let root = tempfile::tempdir().unwrap();
        let converter = converter(root.path().join("transient"));
        let (bytes, inventory) = fixture(kind);
        let mut result = converter
            .convert(
                ConversionRequest {
                    kind,
                    intent: ImportIntent::FullNote,
                    bytes,
                    source_reference: "synthetic://harbor/source",
                    inventory: None,
                },
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(result.coverage.status, CoverageStatus::Partial);
        assert!(
            result
                .coverage
                .gaps
                .iter()
                .any(|gap| gap.kind == GapKind::Unverified)
        );
        result.check_inventory(Some(&inventory)).unwrap();
        assert_eq!(result.coverage.status, CoverageStatus::Complete);
        result.assets.clear();
        result.check_inventory(Some(&inventory)).unwrap();
        assert_eq!(result.coverage.status, CoverageStatus::Partial);
        assert!(
            result
                .coverage
                .gaps
                .iter()
                .any(|gap| gap.kind == GapKind::Omission && gap.locator == "figures")
        );
    }
}
#[test]
fn useful_intent_is_unchecked_conversion_for_host_selection() {
    let root = tempfile::tempdir().unwrap();
    let converter = converter(root.path().join("transient"));
    let (bytes, _) = fixture(DocumentKind::Docx);
    let result = converter
        .convert(
            ConversionRequest {
                kind: DocumentKind::Docx,
                intent: ImportIntent::UsefulInformation,
                bytes,
                source_reference: "synthetic://harbor/process",
                inventory: None,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(result.intent, ImportIntent::UsefulInformation);
    assert_eq!(result.coverage.status, CoverageStatus::Unchecked);
    assert!(!result.markdown.is_empty());
}
#[test]
fn malformed_budget_cancel_and_timeout_leave_no_transient_copy() {
    let root = tempfile::tempdir().unwrap();
    let mut converter = converter(root.path().join("transient"));
    let (bytes, _) = fixture(DocumentKind::Pdf);
    let request = |bytes| ConversionRequest {
        kind: DocumentKind::Pdf,
        intent: ImportIntent::FullNote,
        bytes,
        source_reference: "synthetic://harbor/source",
        inventory: None,
    };
    assert_eq!(
        converter.convert(request(bytes), &AtomicBool::new(true)),
        Err(ConversionError::Cancelled)
    );
    assert_eq!(
        converter.convert(request(b"not a PDF"), &AtomicBool::new(false)),
        Err(ConversionError::Invalid)
    );
    assert_eq!(std::fs::read_dir(&converter.temp_root).unwrap().count(), 0);
    converter.limits.max_input_bytes = 1;
    assert_eq!(
        converter.convert(request(bytes), &AtomicBool::new(false)),
        Err(ConversionError::Budget)
    );
    converter.limits = brn_intake::IntakeLimits {
        max_output_bytes: 1024,
        ..Default::default()
    };
    assert_eq!(
        converter.convert(request(bytes), &AtomicBool::new(false)),
        Err(ConversionError::Budget)
    );
    assert_eq!(std::fs::read_dir(&converter.temp_root).unwrap().count(), 0);
    converter.limits = brn_intake::IntakeLimits {
        wall_time_ms: 1,
        ..Default::default()
    };
    assert_eq!(
        converter.convert(request(bytes), &AtomicBool::new(false)),
        Err(ConversionError::Timeout)
    );
    assert_eq!(std::fs::read_dir(&converter.temp_root).unwrap().count(), 0);
}
#[cfg(unix)]
#[test]
fn restart_expiry_removes_only_owned_old_runs_and_does_not_follow_links() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let converter = converter(root.path().join("transient"));
    std::fs::create_dir_all(&converter.temp_root).unwrap();
    let abandoned = converter
        .temp_root
        .join(format!("threads-intake-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&abandoned).unwrap();
    std::fs::write(abandoned.join("input.pdf"), b"excluded-canary").unwrap();
    let dir = std::fs::File::open(&abandoned).unwrap();
    dir.set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
        .unwrap();
    let external = root.path().join("external");
    std::fs::create_dir(&external).unwrap();
    std::fs::write(external.join("source"), b"keep").unwrap();
    symlink(
        &external,
        converter
            .temp_root
            .join(format!("threads-intake-{}", uuid::Uuid::new_v4())),
    )
    .unwrap();
    std::fs::create_dir(converter.temp_root.join("unrelated")).unwrap();
    assert_eq!(converter.cleanup_expired().unwrap(), 1);
    assert!(!abandoned.exists());
    assert!(external.join("source").exists());
    assert!(converter.temp_root.join("unrelated").exists());
}

#[test]
fn cancellation_after_pdf_copy_removes_raw_input_before_return() {
    use std::{
        sync::{Arc, atomic::Ordering},
        time::{Duration, Instant},
    };
    let root = tempfile::tempdir().unwrap();
    let converter = converter(root.path().join("transient"));
    let directory = converter.temp_root.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancel = cancelled.clone();
    let observed_copy = Arc::new(AtomicBool::new(false));
    let observed = observed_copy.clone();
    let signal = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if std::fs::read_dir(&directory).ok().is_some_and(|entries| {
                entries
                    .filter_map(Result::ok)
                    .any(|entry| entry.path().join("input.pdf").exists())
            }) {
                observed.store(true, Ordering::Release);
                cancelled.store(true, Ordering::Release);
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        cancelled.store(true, Ordering::Release);
    });
    let (bytes, _) = fixture(DocumentKind::Pdf);
    let result = converter.convert(
        ConversionRequest {
            kind: DocumentKind::Pdf,
            intent: ImportIntent::FullNote,
            bytes,
            source_reference: "synthetic://harbor/source",
            inventory: None,
        },
        &cancel,
    );
    signal.join().unwrap();
    assert!(observed_copy.load(Ordering::Acquire));
    assert_eq!(result, Err(ConversionError::Cancelled));
    assert_eq!(std::fs::read_dir(&converter.temp_root).unwrap().count(), 0);
}

#[test]
fn maintained_docx_reader_rejects_malformed_and_tight_package_budget() {
    let root = tempfile::tempdir().unwrap();
    let mut converter = converter(root.path().join("transient"));
    let request = |bytes| ConversionRequest {
        kind: DocumentKind::Docx,
        intent: ImportIntent::FullNote,
        bytes,
        source_reference: "synthetic://harbor/process",
        inventory: None,
    };
    assert_eq!(
        converter.convert(request(b"PK\x03\x04malformed"), &AtomicBool::new(false)),
        Err(ConversionError::Invalid)
    );
    let (bytes, _) = fixture(DocumentKind::Docx);
    converter.limits.max_package_parts = 1;
    assert_eq!(
        converter.convert(request(bytes), &AtomicBool::new(false)),
        Err(ConversionError::Budget)
    );
    assert_eq!(std::fs::read_dir(&converter.temp_root).unwrap().count(), 0);
}

#[test]
fn markdown_text_and_email_preserve_meaning_without_source_payloads() {
    let root=tempfile::tempdir().unwrap(); let converter=converter(root.path().join("scratch"));
    for kind in [DocumentKind::Text,DocumentKind::Markdown] {
        let bytes="日本語\r\n# Process\n\n| Step | Owner |\n|---|---|\n| Check | Ana |\n".as_bytes();
        let result=converter.convert(ConversionRequest{kind,intent:ImportIntent::FullNote,bytes,source_reference:"synthetic://plain",inventory:None},&AtomicBool::new(false)).unwrap();
        assert_eq!(result.markdown.as_bytes(),bytes);assert_eq!(result.coverage.status,CoverageStatus::Partial);assert!(result.assets.is_empty());
    }
    let raw=b"From: Ana <ana@example.invalid>\r\nTo: BRN <brn@example.invalid>\r\nSubject: Harbor schedule\r\nMessage-ID: <harbor-1@example.invalid>\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nThe check moves to Tuesday. Please retain the source reference.\r\n";
    let result=converter.convert(ConversionRequest{kind:DocumentKind::Eml,intent:ImportIntent::UsefulInformation,bytes:raw,source_reference:"synthetic://mail-1",inventory:None},&AtomicBool::new(false)).unwrap();
    assert!(result.markdown.contains("Tuesday"));assert!(result.markdown.contains("Harbor schedule"));assert!(result.sources.iter().all(|source|!source.locator.is_empty()));assert!(serde_json::to_value(result).unwrap().get("original").is_none());assert_eq!(std::fs::read_dir(&converter.temp_root).unwrap().count(),0);
}
