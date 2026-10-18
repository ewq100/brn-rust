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

// B3: read-only resolution of the existing full import result; importer behavior is unchanged.
fn b3_import_world() -> (tempfile::TempDir, Workspace, ConversionResult) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let app = Workspace::open_without_default_credentials(root.path().join("b3-import-resolution"))
        .unwrap();
    let converted = ConversionResult {
        intent: ImportIntent::FullNote,
        markdown: "# Exact retained X\nOriginal substantive wording.".into(),
        assets: vec![],
        source_sha256: Sha256::digest(b"exact original X").into(),
        source_reference: "synthetic://b3-import-x".into(),
        converter: "synthetic existing converter".into(),
        sources: vec![],
        coverage: Coverage {
            status: CoverageStatus::Partial,
            checked_items: vec![],
            gaps: vec![],
        },
    };
    (root, app, converted)
}
fn b3_import_receipt(
    app: &mut Workspace,
    key: &str,
    converted: &ConversionResult,
    scope: Option<&str>,
) -> Receipt {
    let ApplyOutcome::Applied(receipt) = app
        .import_full_scoped(key, "B3 canonical full Note", converted, scope)
        .unwrap()
    else {
        panic!("applied import")
    };
    receipt
}
fn b3_import_note(receipt: &Receipt) -> Record {
    receipt
        .writes
        .iter()
        .find(|w| matches!(w.after.data, RecordData::Note(_)))
        .unwrap()
        .after
        .clone()
}
fn b3_import_put(
    app: &mut Workspace,
    key: &str,
    record: &Record,
    archived: bool,
    data: RecordData,
) {
    assert!(matches!(
        app.owner_change(
            key,
            &ChangeRequest {
                reason: "Explicit synthetic owner change".into(),
                writes: vec![Put {
                    id: record.id.clone(),
                    expected_version: Some(record.version),
                    archived,
                    data
                }],
                inputs: vec![]
            }
        )
        .unwrap(),
        ApplyOutcome::Applied(_)
    ));
}

#[test]
fn b3_import_resolution_reads_initial_observation_and_current_owner_edited_note_without_writes() {
    let (_root, mut app, converted) = b3_import_world();
    let first = b3_import_receipt(&mut app, "b3-resolution-first", &converted, None);
    let original = b3_import_note(&first);
    assert_eq!(
        app.full_import_note_in_scope(&first, &converted.source_sha256, None)
            .unwrap(),
        original
    );
    let repeated = b3_import_receipt(&mut app, "b3-resolution-observation", &converted, None);
    assert!(
        repeated
            .writes
            .iter()
            .all(|w| !matches!(w.after.data, RecordData::Note(_)))
    );
    assert_eq!(
        app.full_import_note_in_scope(&repeated, &converted.source_sha256, None)
            .unwrap(),
        original
    );
    let RecordData::Note(mut revised) = original.data.clone() else {
        panic!("Note")
    };
    revised.title = "Legitimate owner title".into();
    revised
        .markdown
        .push_str("\nLegitimate owner edit stays current.");
    b3_import_put(
        &mut app,
        "b3-resolution-owner-edit",
        &original,
        false,
        RecordData::Note(revised),
    );
    let current = app.store.note(&original.id).unwrap().unwrap();
    let before = (app.records().unwrap(), app.store.receipts().unwrap());
    assert_eq!(
        app.full_import_note_in_scope(&repeated, &converted.source_sha256, None)
            .unwrap(),
        current
    );
    assert_eq!(
        (app.records().unwrap(), app.store.receipts().unwrap()),
        before,
        "resolver is read-only"
    );
    assert_eq!(app.store.note(&original.id).unwrap().unwrap(), current);
}

#[test]
fn b3_import_resolution_binds_requested_x_after_y_without_using_current_external_version() {
    let (_root, mut app, mut converted) = b3_import_world();
    let x = converted.clone();
    let first_x = b3_import_receipt(&mut app, "b3-resolution-x", &x, None);
    let note_x = b3_import_note(&first_x);
    converted.source_sha256 = Sha256::digest(b"different original Y").into();
    converted.markdown = "# Later retained Y\nDifferent original bytes.".into();
    let first_y = b3_import_receipt(&mut app, "b3-resolution-y", &converted, None);
    let note_y = b3_import_note(&first_y);
    assert_ne!(note_x.id, note_y.id);
    assert_eq!(
        app.full_import_note_in_scope(&first_y, &converted.source_sha256, None)
            .unwrap(),
        note_y
    );
    let again_x = b3_import_receipt(&mut app, "b3-resolution-old-x-again", &x, None);
    assert!(
        again_x
            .writes
            .iter()
            .all(|w| !matches!(w.after.data, RecordData::Note(_)))
    );
    let source = again_x
        .writes
        .iter()
        .find_map(|w| match &w.after.data {
            RecordData::Source(source) => Some(source),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        source.external_version.as_deref(),
        Some(format!("{:x}", Sha256::digest(b"different original Y")).as_str())
    );
    assert_eq!(
        app.full_import_note_in_scope(&again_x, &x.source_sha256, None)
            .unwrap(),
        note_x
    );
    assert!(
        app.full_import_note_in_scope(&again_x, &[0xef; 32], None)
            .is_err(),
        "an absent SHA relation never falls back to latest/source title/path"
    );
    assert_eq!(app.notes().unwrap().len(), 2);
}

#[test]
fn b3_import_resolution_rejects_fabricated_receipts_and_hidden_scope_without_metadata() {
    let (_root, mut app, converted) = b3_import_world();
    let project = app
        .project_command(
            "b3-resolution-private-project",
            ProjectCommand::Create {
                title: "HIDDEN PROJECT TITLE".into(),
            },
        )
        .unwrap()
        .writes[0]
        .after
        .id
        .clone();
    let receipt = b3_import_receipt(
        &mut app,
        "b3-resolution-private",
        &converted,
        Some(&project),
    );
    let note = b3_import_note(&receipt);
    assert_eq!(
        app.full_import_note_in_scope(&receipt, &converted.source_sha256, Some(&project))
            .unwrap(),
        note
    );
    let hidden = app
        .full_import_note_in_scope(&receipt, &converted.source_sha256, None)
        .unwrap_err()
        .to_string();
    assert_eq!(
        hidden,
        "Imported reference is no longer available in this writing scope"
    );
    assert!(!hidden.contains(&project) && !hidden.contains(&note.id) && !hidden.contains("HIDDEN"));
    let mut fabricated = receipt.clone();
    if let RecordData::Source(source) = &mut fabricated
        .writes
        .iter_mut()
        .find(|w| matches!(w.after.data, RecordData::Source(_)))
        .unwrap()
        .after
        .data
    {
        source.chat = None;
        source.label = "FABRICATED RECEIPT TITLE".into();
    }
    assert_eq!(
        app.full_import_note_in_scope(&fabricated, &converted.source_sha256, None)
            .unwrap_err()
            .to_string(),
        hidden
    );
    assert_eq!(
        app.full_import_note_in_scope(&fabricated, &converted.source_sha256, Some(&project))
            .unwrap_err()
            .to_string(),
        hidden,
        "even a currently visible scope cannot make an altered receipt payload authoritative"
    );
    fabricated = receipt.clone();
    fabricated.operation = app
        .store
        .allocate_operation("b3-unapplied-receipt-operation")
        .unwrap();
    assert_eq!(
        app.full_import_note_in_scope(&fabricated, &converted.source_sha256, Some(&project))
            .unwrap_err()
            .to_string(),
        hidden
    );
}

#[test]
fn b3_import_resolution_rejects_archived_source_note_or_link_and_superseded_note() {
    for part in [
        "source",
        "note",
        "link",
        "superseded",
        "note_moved",
        "link_moved",
    ] {
        let (_root, mut app, converted) = b3_import_world();
        let receipt = b3_import_receipt(&mut app, "b3-resolution-live", &converted, None);
        let note = b3_import_note(&receipt);
        if part == "superseded" {
            let next = app
                .new_note("Independent next Note", "Current owner work")
                .unwrap();
            let RecordData::Note(mut data) = note.data.clone() else {
                panic!("Note")
            };
            data.superseded_by = Some(next.id);
            b3_import_put(
                &mut app,
                "b3-resolution-superseded",
                &note,
                false,
                RecordData::Note(data),
            );
        } else if part == "note_moved" {
            let thread = app.new_thread("Separate private context").unwrap();
            let RecordData::Thread(mut data) = thread.data.clone() else {
                panic!("Thread")
            };
            data.private_context = true;
            b3_import_put(
                &mut app,
                "b3-resolution-new-private-context",
                &thread,
                false,
                RecordData::Thread(data),
            );
            let RecordData::Note(mut data) = note.data.clone() else {
                panic!("Note")
            };
            data.chat = Some(thread.id);
            b3_import_put(
                &mut app,
                "b3-resolution-note-moved",
                &note,
                false,
                RecordData::Note(data),
            );
        } else if part == "link_moved" {
            let mut another = converted.clone();
            another.source_reference = "synthetic://b3-distinct-source-same-bytes".into();
            let other = b3_import_receipt(&mut app, "b3-resolution-other-source", &another, None);
            let other_source = other
                .writes
                .iter()
                .find(|w| matches!(w.after.data, RecordData::Source(_)))
                .unwrap()
                .after
                .id
                .clone();
            let link = receipt
                .writes
                .iter()
                .find(|w| matches!(w.after.data, RecordData::Link(_)))
                .unwrap()
                .after
                .clone();
            let RecordData::Link(mut data) = link.data.clone() else {
                panic!("Link")
            };
            data.to = other_source;
            b3_import_put(
                &mut app,
                "b3-resolution-link-moved",
                &link,
                false,
                RecordData::Link(data),
            );
        } else {
            let record = receipt
                .writes
                .iter()
                .find(|w| match part {
                    "source" => matches!(w.after.data, RecordData::Source(_)),
                    "note" => matches!(w.after.data, RecordData::Note(_)),
                    "link" => matches!(w.after.data, RecordData::Link(_)),
                    _ => false,
                })
                .unwrap()
                .after
                .clone();
            b3_import_put(
                &mut app,
                "b3-resolution-archive",
                &record,
                true,
                record.data.clone(),
            );
        }
        let before = (app.records().unwrap(), app.store.receipts().unwrap());
        assert_eq!(
            app.full_import_note_in_scope(&receipt, &converted.source_sha256, None)
                .unwrap_err()
                .to_string(),
            "Imported reference is no longer available in this writing scope",
            "{part}"
        );
        assert_eq!(
            (app.records().unwrap(), app.store.receipts().unwrap()),
            before
        );
    }
}

#[test]
fn b3_import_resolution_rejects_ambiguous_canonical_notes_instead_of_taking_first() {
    let (_root, mut app, converted) = b3_import_world();
    let receipt = b3_import_receipt(&mut app, "b3-resolution-ambiguous-base", &converted, None);
    let original = b3_import_note(&receipt);
    let duplicate = app
        .new_note("Another explicit owner Note", "Independent owner content")
        .unwrap();
    let RecordData::Note(mut same_source) = original.data.clone() else {
        panic!("Note")
    };
    same_source.title = "Another owner Note with the same recorded import linkage".into();
    let source = same_source.import.as_ref().unwrap().source.clone();
    let operation = app
        .store
        .allocate_operation("b3-resolution-ambiguous-links")
        .unwrap();
    let relation = receipt
        .writes
        .iter()
        .find_map(|w| match &w.after.data {
            RecordData::Link(link) => Some(link.relation.clone()),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        app.owner_change(
            "b3-resolution-ambiguous-links",
            &ChangeRequest {
                reason: "Synthetic ambiguous imported association".into(),
                writes: vec![
                    Put {
                        id: duplicate.id.clone(),
                        expected_version: Some(duplicate.version),
                        archived: false,
                        data: RecordData::Note(same_source)
                    },
                    Put {
                        id: operation.creation_id(0),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Link(Link {
                            from: duplicate.id.clone(),
                            to: source,
                            relation
                        })
                    }
                ],
                inputs: vec![]
            }
        )
        .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let before = (app.records().unwrap(), app.store.receipts().unwrap());
    assert!(
        app.full_import_note_in_scope(&receipt, &converted.source_sha256, None)
            .is_err()
    );
    assert_eq!(
        (app.records().unwrap(), app.store.receipts().unwrap()),
        before
    );
}

#[test]
fn b3_import_resolution_rejects_canonical_scope_movement_without_global_fallback() {
    let (_root, mut app, converted) = b3_import_world();
    let thread = app.new_thread("Private import conversation").unwrap();
    let RecordData::Thread(mut private) = thread.data.clone() else {
        panic!("Thread")
    };
    private.private_context = true;
    b3_import_put(
        &mut app,
        "b3-private-import-conversation",
        &thread,
        false,
        RecordData::Thread(private),
    );
    let receipt = b3_import_receipt(
        &mut app,
        "b3-resolution-moving",
        &converted,
        Some(&thread.id),
    );
    let canonical = app
        .full_import_note_in_scope(&receipt, &converted.source_sha256, Some(&thread.id))
        .unwrap();
    let project = app
        .project_command(
            "b3-resolution-move-project",
            ProjectCommand::Create {
                title: "New canonical scope".into(),
            },
        )
        .unwrap()
        .writes[0]
        .after
        .id
        .clone();
    app.project_command(
        "b3-resolution-move-conversation",
        ProjectCommand::MoveChat {
            chat: thread.id.clone(),
            project: Some(project.clone()),
        },
    )
    .unwrap();
    assert!(
        app.full_import_note_in_scope(&receipt, &converted.source_sha256, Some(&thread.id))
            .is_err()
    );
    assert!(
        app.full_import_note_in_scope(&receipt, &converted.source_sha256, Some(&project))
            .is_err()
    );
    assert!(
        app.full_import_note_in_scope(&receipt, &converted.source_sha256, None)
            .is_err()
    );
    assert_eq!(
        app.store.note(&canonical.id).unwrap().unwrap(),
        canonical,
        "scope refusal never rewrites retained Note content"
    );
}

#[test]
fn combined_reference_resolution_keeps_exact_conversion_variant_after_reimport() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open(root.path().join("combined-reference")).unwrap();
    let mut expected = Vec::new();
    for (index, (converter, ai)) in [
        (DocumentConverter::Xberg, false),
        (DocumentConverter::Xberg, true),
        (DocumentConverter::XbergBrn, false),
        (DocumentConverter::XbergBrn, true),
    ]
    .into_iter()
    .enumerate()
    {
        let settings = DocumentConversionSettings { converter, ai };
        let result = brn_threads_intake::ConversionResult {
            intent: brn_threads_intake::ImportIntent::FullNote,
            markdown: format!(
                "# Retained variant {index}\n\nSame external bytes, selected output."
            ),
            assets: vec![],
            source_sha256: [91; 32],
            source_reference: "synthetic://combined-reference-variants".into(),
            converter: brn_threads_intake::document_conversion_label(
                if converter == DocumentConverter::Xberg {
                    "xberg"
                } else {
                    "xberg_brn"
                },
                ai,
            )
            .unwrap(),
            sources: vec![],
            coverage: brn_threads_intake::Coverage {
                status: brn_threads_intake::CoverageStatus::Unchecked,
                checked_items: vec![],
                gaps: vec![],
            },
        };
        let ApplyOutcome::Applied(receipt) = app
            .import_full_scoped_with_conversion(
                &format!("variant-{index}"),
                "Retained source",
                &result,
                None,
                &settings,
            )
            .unwrap()
        else {
            panic!("checked import")
        };
        let note = receipt
            .writes
            .iter()
            .find(|write| matches!(write.after.data, RecordData::Note(_)))
            .unwrap()
            .after
            .clone();
        assert_eq!(
            app.full_import_note_in_scope(&receipt, &result.source_sha256, None)
                .unwrap(),
            note
        );
        expected.push((settings, result, note));
    }
    assert_eq!(app.notes().unwrap().len(), 4);
    for (index, (settings, result, note)) in expected.iter().enumerate() {
        let ApplyOutcome::Applied(receipt) = app
            .import_full_scoped_with_conversion(
                &format!("reimport-{index}"),
                "Retained source",
                result,
                None,
                settings,
            )
            .unwrap()
        else {
            panic!("checked reimport")
        };
        assert!(
            !receipt
                .writes
                .iter()
                .any(|write| matches!(write.after.data, RecordData::Note(_))),
            "observation-only receipt must resolve the existing Note"
        );
        assert_eq!(
            app.full_import_note_in_scope(&receipt, &result.source_sha256, None)
                .unwrap(),
            *note
        );
        assert_eq!(app.notes().unwrap().len(), 4);
    }
}
