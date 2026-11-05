//! Opt-in host-path measurements with deterministic conversion, never model/codec timing.
//! Run explicitly with `--ignored --nocapture --test-threads=1` on an idle machine.
#![cfg(unix)]
use brn_threads_app::*;
use brn_threads_intake::{ConversionResult, Coverage, CoverageStatus, ImportIntent};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

fn converted(
    doc: InboxDocument,
    settings: &DocumentConversionSettings,
) -> AppResult<ConversionResult> {
    Ok(ConversionResult {
        intent: ImportIntent::FullNote,
        markdown: String::from_utf8(doc.bytes.clone()).unwrap(),
        assets: vec![],
        source_sha256: Sha256::digest(&doc.bytes).into(),
        source_reference: doc.source_reference,
        converter: brn_threads_intake::document_conversion_label(
            match settings.converter {
                DocumentConverter::Xberg => "xberg",
                DocumentConverter::XbergBrn => "xberg_brn",
            },
            settings.ai,
        )
        .unwrap(),
        sources: vec![],
        coverage: Coverage {
            status: CoverageStatus::Unchecked,
            checked_items: vec![],
            gaps: vec![],
        },
    })
}

#[test]
#[ignore = "explicit synthetic host-path timing; no codec or live-model qualification"]
fn bounded_inbox_scan_cancel_reopen_and_exact_replay_profile() {
    for repeat in 0..2 {
        for files in [16, 64, 256] {
            let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = root.path().join("data");
            let folder = root.path().join("inbox");
            std::fs::create_dir(&folder).unwrap();
            let contents: Vec<_> = (0..files)
                .map(|i| format!("# Synthetic inbox {i}\n\nExact retained body {i}.\n"))
                .collect();
            for (i, content) in contents.iter().enumerate() {
                std::fs::write(folder.join(format!("note-{i:03}.txt")), content).unwrap();
            }
            let mut workspace = Workspace::open_without_default_credentials(&data).unwrap();
            workspace
                .choose_inbox(&folder, InboxDestination::General)
                .unwrap();
            let cancel = AtomicBool::new(false);
            let mut conversions = 0;
            let start = Instant::now();
            let stopped = workspace
                .scan_inbox(&cancel, |doc, settings| {
                    conversions += 1;
                    if conversions == files / 2 {
                        cancel.store(true, Ordering::Release);
                    }
                    converted(doc, settings)
                })
                .unwrap();
            let interrupted_ms = start.elapsed().as_secs_f64() * 1000.;
            // Cancellation after conversion and before commit must retain the in-flight item.
            assert_eq!(stopped.imported, files / 2 - 1);
            assert_eq!(stopped.pending, files - stopped.imported);
            assert_eq!(stopped.failed, 0);
            drop(workspace);
            let start = Instant::now();
            let mut workspace = Workspace::open_without_default_credentials(&data).unwrap();
            let open_ms = start.elapsed().as_secs_f64() * 1000.;
            cancel.store(false, Ordering::Release);
            let mut resumed_conversions = 0;
            let start = Instant::now();
            let resumed = workspace
                .scan_inbox(&cancel, |doc, settings| {
                    resumed_conversions += 1;
                    converted(doc, settings)
                })
                .unwrap();
            let resume_ms = start.elapsed().as_secs_f64() * 1000.;
            assert_eq!(
                resumed,
                InboxScanSummary {
                    imported: files,
                    unchanged: stopped.imported,
                    pending: 0,
                    failed: 0
                }
            );
            assert_eq!(resumed_conversions, files - stopped.imported);
            let records = workspace.records().unwrap();
            let notes: Vec<_> = records
                .iter()
                .filter_map(|r| match &r.data {
                    RecordData::Note(n) if !r.archived => Some(n),
                    _ => None,
                })
                .collect();
            assert_eq!(notes.len(), files);
            assert!(notes.iter().all(|n| n.chat.is_none() && n.protected));
            for (i, content) in contents.iter().enumerate() {
                assert_eq!(notes.iter().filter(|n| n.markdown == *content).count(), 1);
                assert_eq!(
                    std::fs::read_to_string(folder.join(format!("note-{i:03}.txt"))).unwrap(),
                    *content
                );
            }
            let before = workspace.records().unwrap();
            drop(workspace);
            let mut workspace = Workspace::open_without_default_credentials(&data).unwrap();
            let start = Instant::now();
            let replay = workspace
                .scan_inbox(&cancel, |_, _| panic!("committed files must not reconvert"))
                .unwrap();
            let replay_ms = start.elapsed().as_secs_f64() * 1000.;
            assert_eq!(
                replay,
                InboxScanSummary {
                    imported: files,
                    unchanged: files,
                    pending: 0,
                    failed: 0
                }
            );
            assert_eq!(
                workspace.records().unwrap(),
                before,
                "replay must not mutate accepted records"
            );
            eprintln!(
                "INBOX_PROFILE {}",
                serde_json::json!({"files":files,"repeat":repeat,"interrupted_ms":interrupted_ms,"open_ms":open_ms,"resume_ms":resume_ms,"replay_ms":replay_ms,"conversion":"deterministic text fixture"})
            );
        }
    }
}
