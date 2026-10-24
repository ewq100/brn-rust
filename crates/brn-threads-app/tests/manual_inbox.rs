#![cfg(unix)]
use brn_threads_app::*;
use brn_threads_intake::{ConversionResult, Coverage, CoverageStatus, ImportIntent};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

fn world() -> (tempfile::TempDir, PathBuf, Workspace) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let folder = root.path().join("inbox");
    fs::create_dir(&folder).unwrap();
    let mut workspace =
        Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    workspace
        .choose_inbox(&folder, InboxDestination::General)
        .unwrap();
    (root, folder, workspace)
}
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
fn notes(w: &Workspace) -> Vec<Record> {
    w.records()
        .unwrap()
        .into_iter()
        .filter(|r| !r.archived && matches!(r.data, RecordData::Note(_)))
        .collect()
}
#[test]
fn explicit_general_import_preserves_files_and_replays_after_reopen() {
    let (root, folder, mut w) = world();
    let bytes = b"# Synthetic meeting\n\nOwner evidence only.\n";
    fs::write(folder.join("meeting.md"), bytes).unwrap();
    let stop = AtomicBool::new(false);
    assert_eq!(w.scan_inbox(&stop, converted).unwrap().imported, 1);
    let before = notes(&w);
    assert_eq!(before.len(), 1);
    let RecordData::Note(note) = &before[0].data else {
        panic!()
    };
    assert!(note.chat.is_none());
    assert_eq!(note.markdown, String::from_utf8_lossy(bytes));
    assert_eq!(fs::read(folder.join("meeting.md")).unwrap(), bytes);
    drop(w);
    let mut w = Workspace::open_existing_no_upgrade(root.path().join("data")).unwrap();
    assert_eq!(
        w.scan_inbox(&stop, |_, _| panic!("replay must not convert"))
            .unwrap()
            .imported,
        1
    );
    assert_eq!(notes(&w), before);
}
#[test]
fn changed_content_and_profile_are_new_variants_without_mutating_old_notes() {
    let (_root, folder, mut w) = world();
    let stop = AtomicBool::new(false);
    fs::write(folder.join("note.txt"), "First exact source").unwrap();
    w.scan_inbox(&stop, converted).unwrap();
    let first = notes(&w)[0].clone();
    fs::write(folder.join("note.txt"), "Second exact source").unwrap();
    w.scan_inbox(&stop, converted).unwrap();
    assert_eq!(notes(&w).len(), 2);
    assert!(notes(&w).contains(&first));
    w.choose_document_conversion(DocumentConversionSettings {
        converter: DocumentConverter::XbergBrn,
        ai: false,
    })
    .unwrap();
    assert_eq!(w.scan_inbox(&stop, converted).unwrap().imported, 1);
    assert_eq!(notes(&w).len(), 3);
    w.scan_inbox(&stop, |_, _| panic!("profile replay"))
        .unwrap();
    assert_eq!(notes(&w).len(), 3);
}
#[test]
fn failed_file_retries_and_other_files_still_import() {
    let (_root, folder, mut w) = world();
    fs::write(folder.join("a.txt"), "A").unwrap();
    fs::write(folder.join("b.txt"), "B").unwrap();
    let stop = AtomicBool::new(false);
    let result = w
        .scan_inbox(&stop, |doc, s| {
            if doc.source_reference.ends_with("a.txt") {
                Err(AppError::Unavailable("Synthetic decoder failure".into()))
            } else {
                converted(doc, s)
            }
        })
        .unwrap();
    assert_eq!((result.failed, result.imported), (1, 1));
    assert_eq!(w.scan_inbox(&stop, converted).unwrap().imported, 2);
    assert_eq!(notes(&w).len(), 2);
}
#[test]
fn interruption_keeps_pending_then_resume_does_not_duplicate_completed_work() {
    let (_root, folder, mut w) = world();
    fs::write(folder.join("a.txt"), "A").unwrap();
    fs::write(folder.join("b.txt"), "B").unwrap();
    let stop = AtomicBool::new(false);
    let result = w
        .scan_inbox(&stop, |doc, s| {
            stop.store(true, Ordering::Release);
            converted(doc, s)
        })
        .unwrap();
    assert_eq!(result.pending, 2);
    assert!(notes(&w).is_empty());
    stop.store(false, Ordering::Release);
    assert_eq!(w.scan_inbox(&stop, converted).unwrap().imported, 2);
}
#[test]
fn committed_import_with_missing_ledger_success_recovers_without_conversion() {
    let (_root, folder, mut w) = world();
    fs::write(folder.join("note.txt"), "Exact source").unwrap();
    let stop = AtomicBool::new(false);
    w.scan_inbox(&stop, converted).unwrap();
    let mut state = w.inbox().unwrap().unwrap();
    state.entries[0].status = InboxStatus::Pending;
    state.entries[0].note = None;
    w.store
        .save_inbox_state(
            Some(state.revision),
            &state,
            &HostAuthority::owner("synthetic crash after canonical commit"),
        )
        .unwrap();
    assert_eq!(
        w.scan_inbox(&stop, |_, _| panic!("must recover canonical receipt"))
            .unwrap()
            .imported,
        1
    );
    assert_eq!(notes(&w).len(), 1);
}
#[test]
fn changing_file_during_conversion_is_not_imported() {
    let (_root, folder, mut w) = world();
    let path = folder.join("note.txt");
    fs::write(&path, "Before").unwrap();
    let result = w
        .scan_inbox(&AtomicBool::new(false), |doc, s| {
            fs::write(&path, "After").unwrap();
            converted(doc, s)
        })
        .unwrap();
    assert_eq!(result.failed, 1);
    assert!(notes(&w).is_empty());
    assert_eq!(fs::read_to_string(path).unwrap(), "After");
}
#[test]
fn symlinks_directories_fifo_and_escape_paths_are_refused() {
    use std::os::unix::fs::symlink;
    let (root, folder, mut w) = world();
    fs::write(root.path().join("external.txt"), "Outside").unwrap();
    symlink(root.path().join("external.txt"), folder.join("link.txt")).unwrap();
    fs::create_dir(folder.join("directory.txt")).unwrap();
    use std::os::unix::ffi::OsStrExt;
    let fifo = std::ffi::CString::new(folder.join("fifo.txt").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let result = w
        .scan_inbox(&AtomicBool::new(false), |_, _| {
            panic!("unsafe file reached converter")
        })
        .unwrap();
    assert_eq!(result.failed, 3);
    let alias = root.path().join("alias");
    symlink(&folder, &alias).unwrap();
    assert!(w.choose_inbox(&alias, InboxDestination::General).is_err());
    assert!(
        w.choose_inbox(root.path(), InboxDestination::General)
            .is_err()
    );
    assert!(notes(&w).is_empty());
}
#[test]
fn replaced_folder_requires_explicit_choice_and_oversized_scan_does_not_mutate_state() {
    let (root, folder, mut w) = world();
    fs::rename(&folder, root.path().join("old-inbox")).unwrap();
    fs::create_dir(&folder).unwrap();
    assert!(w.scan_inbox(&AtomicBool::new(false), converted).is_err());
    w.choose_inbox(&folder, InboxDestination::General).unwrap();
    let state = w.inbox().unwrap();
    for i in 0..257 {
        fs::write(folder.join(format!("{i}.txt")), "x").unwrap();
    }
    assert!(w.scan_inbox(&AtomicBool::new(false), converted).is_err());
    assert_eq!(w.inbox().unwrap(), state);
}
#[test]
fn settings_change_during_conversion_fails_before_knowledge_write() {
    let (root, folder, mut w) = world();
    fs::write(folder.join("note.txt"), "Exact source").unwrap();
    let mut other = Workspace::open_existing_no_upgrade(root.path().join("data")).unwrap();
    let result = w
        .scan_inbox(&AtomicBool::new(false), |doc, s| {
            other
                .choose_document_conversion(DocumentConversionSettings {
                    converter: DocumentConverter::XbergBrn,
                    ai: false,
                })
                .unwrap();
            converted(doc, s)
        })
        .unwrap();
    assert_eq!(result.failed, 1);
    assert!(notes(&w).is_empty());
}
#[test]
fn concurrent_scan_and_folder_change_are_refused_without_waiting() {
    let (root, folder, mut w) = world();
    fs::write(folder.join("note.txt"), "Exact source").unwrap();
    let mut other = Workspace::open_existing_no_upgrade(root.path().join("data")).unwrap();
    w.scan_inbox(&AtomicBool::new(false), |doc, s| {
        assert!(
            other
                .scan_inbox(&AtomicBool::new(false), converted)
                .is_err()
        );
        assert!(
            other
                .choose_inbox(&folder, InboxDestination::General)
                .is_err()
        );
        converted(doc, s)
    })
    .unwrap();
}

#[test]
fn unrelated_settings_change_preserves_committed_conversion_recovery() {
    let (_root, folder, mut w) = world();
    fs::write(folder.join("note.txt"), "Exact source").unwrap();
    let stop = AtomicBool::new(false);
    w.scan_inbox(&stop, converted).unwrap();
    let before = notes(&w);
    let old_key = w.inbox().unwrap().unwrap().entries[0].operation_key.clone();
    w.choose_theme("paper").unwrap();
    assert_eq!(
        w.scan_inbox(&stop, |_, _| panic!("theme must not trigger conversion"))
            .unwrap()
            .imported,
        1
    );
    assert_eq!(notes(&w), before);
    assert_eq!(
        w.inbox().unwrap().unwrap().entries[0].operation_key,
        old_key
    );
}

#[test]
fn no_scan_without_explicit_destination_and_oversized_file_never_converts() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut w = Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    assert!(
        w.scan_inbox(&AtomicBool::new(false), |_, _| panic!("no consent"))
            .is_err()
    );
    let folder = root.path().join("inbox");
    fs::create_dir(&folder).unwrap();
    w.choose_inbox(&folder, InboxDestination::General).unwrap();
    fs::File::create(folder.join("huge.txt"))
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        w.scan_inbox(&AtomicBool::new(false), |_, _| panic!("oversized source"))
            .unwrap()
            .failed,
        1
    );
}

#[test]
fn unrelated_settings_revision_during_conversion_does_not_invalidate_profile() {
    let (root, folder, mut w) = world();
    fs::write(folder.join("note.txt"), "Exact source").unwrap();
    let mut other = Workspace::open_existing_no_upgrade(root.path().join("data")).unwrap();
    let result = w
        .scan_inbox(&AtomicBool::new(false), |doc, s| {
            other.choose_theme("paper").unwrap();
            converted(doc, s)
        })
        .unwrap();
    assert_eq!(result.imported, 1);
    assert_eq!(notes(&w).len(), 1);
    assert_eq!(
        w.scan_inbox(&AtomicBool::new(false), converted)
            .unwrap()
            .imported,
        1
    );
}

#[test]
fn folder_reselection_reuses_canonical_receipt_without_reconversion() {
    let (root, folder, mut w) = world();
    fs::write(folder.join("note.txt"), "Exact source").unwrap();
    let stop = AtomicBool::new(false);
    w.scan_inbox(&stop, converted).unwrap();
    let old = w.inbox().unwrap().unwrap();
    let other = root.path().join("other-inbox");
    fs::create_dir(&other).unwrap();
    w.choose_inbox(&other, InboxDestination::General).unwrap();
    w.choose_inbox(&folder, InboxDestination::General).unwrap();
    assert_ne!(w.inbox().unwrap().unwrap().generation, old.generation);
    assert_eq!(
        w.scan_inbox(&stop, |_, _| panic!(
            "reselection must recover canonical receipt"
        ))
        .unwrap()
        .imported,
        1
    );
    assert_eq!(notes(&w).len(), 1);
    assert_eq!(
        w.inbox().unwrap().unwrap().entries[0].operation_key,
        old.entries[0].operation_key
    );
}

#[test]
fn observation_only_import_receipt_recovers_without_reconversion() {
    let (_root, folder, mut w) = world();
    let path = folder.join("note.txt");
    let bytes = b"Exact source".to_vec();
    fs::write(&path, &bytes).unwrap();
    let settings = w.settings().unwrap().document_conversion;
    let snapshot = converted(
        InboxDocument {
            kind: brn_threads_intake::DocumentKind::Text,
            bytes,
            source_reference: path.display().to_string(),
        },
        &settings,
    )
    .unwrap();
    w.import_full_scoped_with_conversion("prior-manual-import", "note", &snapshot, None, &settings)
        .unwrap();
    let stop = AtomicBool::new(false);
    assert_eq!(w.scan_inbox(&stop, converted).unwrap().imported, 1);
    assert_eq!(notes(&w).len(), 1);
    assert_eq!(
        w.scan_inbox(&stop, |_, _| panic!("observation receipt recovery"))
            .unwrap()
            .imported,
        1
    );
    assert_eq!(notes(&w).len(), 1);
}

#[test]
fn disappeared_file_reappears_without_reconversion_after_unrelated_settings() {
    let (root, folder, mut w) = world();
    let file = folder.join("note.txt");
    fs::write(&file, "Exact source").unwrap();
    let stop = AtomicBool::new(false);
    w.scan_inbox(&stop, converted).unwrap();
    let before = notes(&w);
    fs::rename(&file, root.path().join("held.txt")).unwrap();
    assert_eq!(
        w.scan_inbox(&stop, converted).unwrap(),
        InboxScanSummary::default()
    );
    w.choose_theme("paper").unwrap();
    fs::rename(root.path().join("held.txt"), &file).unwrap();
    assert_eq!(
        w.scan_inbox(&stop, |_, _| panic!(
            "canonical receipt survives empty ledger"
        ))
        .unwrap()
        .imported,
        1
    );
    assert_eq!(notes(&w), before);
}

#[test]
fn undone_import_is_not_resurrected_after_file_disappears_and_settings_change() {
    let (root, folder, mut w) = world();
    let file = folder.join("note.txt");
    fs::write(&file, "Exact source").unwrap();
    let stop = AtomicBool::new(false);
    w.scan_inbox(&stop, converted).unwrap();
    let key = w.inbox().unwrap().unwrap().entries[0]
        .operation_key
        .clone()
        .unwrap();
    let op = w
        .store
        .operation_by_host_key(&format!("full-import:{key}"))
        .unwrap()
        .unwrap();
    assert!(matches!(
        w.undo(op.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(notes(&w).is_empty());
    fs::rename(&file, root.path().join("held.txt")).unwrap();
    w.scan_inbox(&stop, converted).unwrap();
    w.choose_theme("paper").unwrap();
    fs::rename(root.path().join("held.txt"), &file).unwrap();
    assert_eq!(
        w.scan_inbox(&stop, |_, _| panic!("Undo must not be bypassed"))
            .unwrap()
            .failed,
        1
    );
    assert!(notes(&w).is_empty());
    let other = root.path().join("other-inbox");
    fs::create_dir(&other).unwrap();
    w.choose_inbox(&other, InboxDestination::General).unwrap();
    w.choose_inbox(&folder, InboxDestination::General).unwrap();
    assert_eq!(
        w.scan_inbox(&stop, |_, _| panic!("reselection must not bypass Undo"))
            .unwrap()
            .failed,
        1
    );
    assert!(notes(&w).is_empty());
}

#[test]
fn atomic_profile_fence_refuses_prepared_write_after_conversion_choice_changes() {
    let (_root, _folder, mut w) = world();
    let old = w.settings().unwrap().document_conversion;
    let op = w
        .store
        .allocate_operation("synthetic-profile-fence")
        .unwrap();
    let owner = HostAuthority::owner("synthetic fence witness");
    w.store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Synthetic prepared write".into(),
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working(
                        "Fence sentinel",
                        "Must not commit with stale profile",
                    )),
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    w.choose_document_conversion(DocumentConversionSettings {
        converter: DocumentConverter::XbergBrn,
        ai: false,
    })
    .unwrap();
    let generation = w.inbox().unwrap().unwrap().generation;
    assert!(
        w.store
            .apply_inbox_import(&op, &old, &generation, &owner)
            .is_err()
    );
    assert!(w.store.receipt(&op).unwrap().is_none());
    assert!(notes(&w).is_empty());
}

#[test]
fn converter_cancellation_remains_pending_instead_of_failed() {
    let (_root, folder, mut w) = world();
    fs::write(folder.join("note.txt"), "Exact source").unwrap();
    let stop = AtomicBool::new(false);
    let result = w
        .scan_inbox(&stop, |_, _| {
            stop.store(true, Ordering::Release);
            Err(AppError::Unavailable("Cancelled".into()))
        })
        .unwrap();
    assert_eq!(result.pending, 1);
    assert_eq!(result.failed, 0);
    assert!(notes(&w).is_empty());
}

#[test]
fn failed_conversions_still_consume_bounded_scan_byte_allowance() {
    let (_root, folder, mut w) = world();
    for index in 0..5 {
        fs::File::create(folder.join(format!("{index}.txt")))
            .unwrap()
            .set_len(16 * 1024 * 1024)
            .unwrap();
    }
    let mut calls = 0;
    let result = w
        .scan_inbox(&AtomicBool::new(false), |_, _| {
            calls += 1;
            Err(AppError::Unavailable("Synthetic bounded failure".into()))
        })
        .unwrap();
    assert_eq!(calls, 4);
    assert_eq!(result.failed, 4);
    assert_eq!(result.pending, 1);
    assert!(notes(&w).is_empty());
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 5);
}

#[test]
fn changed_folder_generation_fences_old_worker_without_overwriting_new_choice() {
    let (root, folder, mut w) = world();
    fs::write(folder.join("note.txt"), "Exact source").unwrap();
    let mut other = Workspace::open_existing_no_upgrade(root.path().join("data")).unwrap();
    let result = w.scan_inbox(&AtomicBool::new(false), |doc, s| {
        let mut next = other.inbox().unwrap().unwrap();
        let expected = next.revision;
        next.generation = "synthetic-new-explicit-choice".into();
        next.entries.clear();
        other
            .store
            .save_inbox_state(
                Some(expected),
                &next,
                &HostAuthority::owner("synthetic concurrent reconfiguration"),
            )
            .unwrap();
        converted(doc, s)
    });
    assert!(result.is_err());
    assert!(notes(&w).is_empty());
    let next = w.inbox().unwrap().unwrap();
    assert_eq!(next.generation, "synthetic-new-explicit-choice");
    assert!(next.entries.is_empty());
}
