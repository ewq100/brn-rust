use brn_threads_app::*;
use brn_threads_intake::{
    ConversionResult, Coverage, CoverageGap, CoverageStatus, GapKind, ImportIntent,
};
use sha2::{Digest, Sha256};

#[test]
fn separate_conversion_candidates_preserve_one_protected_full_import_and_undo() {
    let root =
        tempfile::TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    let mut app = Workspace::open(root.path().join("candidate-import-dedup")).unwrap();
    let mut converted = ConversionResult {
        intent: ImportIntent::FullNote,
        markdown: "# Original candidate\n\nAll source wording.".into(),
        assets: vec![],
        source_sha256: Sha256::digest(b"one exact source").into(),
        source_reference: "synthetic://same-document".into(),
        converter: "synthetic Luna candidate".into(),
        sources: vec![],
        coverage: Coverage {
            status: CoverageStatus::Partial,
            checked_items: vec![],
            gaps: vec![CoverageGap {
                kind: GapKind::Unverified,
                locator: "document".into(),
                detail: "Independent coverage unverified".into(),
            }],
        },
    };
    let ApplyOutcome::Applied(first) = app
        .import_full("invocation-1", "One document", &converted)
        .unwrap()
    else {
        panic!("applied")
    };
    let original = app.notes().unwrap();
    assert_eq!(original.len(), 1);
    converted.markdown = "# Different valid formatting\n\nAll source wording.".into();
    let ApplyOutcome::Applied(second) = app
        .import_full("invocation-2", "One document", &converted)
        .unwrap()
    else {
        panic!("applied")
    };
    assert_ne!(first.operation, second.operation);
    assert_eq!(app.notes().unwrap(), original);
    let RecordData::Note(note) = &original[0].data else {
        panic!("note")
    };
    assert!(note.protected);
    assert!(matches!(
        app.undo(second.operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(app.notes().unwrap(), original);
}

#[test]
fn full_import_persists_detected_loss_separately_from_unverified_coverage() {
    let root =
        tempfile::TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
    let data = root.path().join("data");
    let mut app = Workspace::open(&data).unwrap();
    for (index, kind, status, summary) in [
        (
            0,
            GapKind::Omission,
            CoverageStatus::Partial,
            "Detected omissions · Partial",
        ),
        (
            1,
            GapKind::Unverified,
            CoverageStatus::Unchecked,
            "Coverage unverified",
        ),
        (
            2,
            GapKind::Limitation,
            CoverageStatus::Partial,
            "Partial · inspection required",
        ),
    ] {
        let converted = ConversionResult {
            intent: ImportIntent::FullNote,
            markdown: "# Retained prose\nAll converted substantive text remains.".into(),
            assets: vec![],
            source_sha256: Sha256::digest(format!("source-{index}")).into(),
            source_reference: format!("synthetic://coverage-{index}"),
            converter: "bounded coverage witness".into(),
            sources: vec![],
            coverage: Coverage {
                status,
                checked_items: vec![],
                gaps: vec![CoverageGap {
                    kind,
                    locator: "document/body".into(),
                    detail: "Independent coverage observation".into(),
                }],
            },
        };
        assert!(matches!(
            app.import_full(
                &format!("coverage-{index}"),
                "Imported coverage",
                &converted
            )
            .unwrap(),
            ApplyOutcome::Applied(_)
        ));
        let note = app
            .records()
            .unwrap()
            .into_iter()
            .find_map(|record| match record.data {
                RecordData::Note(note)
                    if note.import.as_ref().is_some_and(|import| {
                        import
                            .report
                            .as_ref()
                            .is_some_and(|report| report.summary() == summary)
                    }) =>
                {
                    Some(note)
                }
                _ => None,
            })
            .unwrap();
        assert!(note.protected);
        assert!(note.import.unwrap().full_note);
    }
    let backup = root.path().join("backup.sqlite3");
    app.backup(&backup).unwrap();
    drop(app);
    let restored = Store::restore(&backup, root.path().join("restored")).unwrap();
    let records = restored.records().unwrap();
    for (index, kind) in [
        (0, ImportIssueKind::Omission),
        (1, ImportIssueKind::Unverified),
        (2, ImportIssueKind::Limitation),
    ] {
        let source = records.iter().find(|record| matches!(&record.data, RecordData::Source(source) if source.locator == format!("synthetic://coverage-{index}"))).unwrap();
        let RecordData::Source(source_data) = &source.data else {
            panic!()
        };
        let report = source_data.report.as_ref().unwrap();
        assert_eq!(report.issues[0].kind, kind);
        assert_eq!(report.issues[0].locator, "document/body");
        let note = records
            .iter()
            .find_map(|record| match &record.data {
                RecordData::Note(note)
                    if note
                        .import
                        .as_ref()
                        .is_some_and(|import| import.source == source.id) =>
                {
                    Some(note)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(note.import.as_ref().unwrap().report.as_ref(), Some(report));
    }
}
