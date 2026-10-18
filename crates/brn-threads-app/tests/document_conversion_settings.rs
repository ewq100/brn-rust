use brn_threads_app::*;
use brn_threads_intake::{
    ConversionResult, Coverage, CoverageStatus, ImportIntent, document_conversion_label,
};
use sha2::{Digest, Sha256};
fn root() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn variants() -> Vec<DocumentConversionSettings> {
    [DocumentConverter::Xberg, DocumentConverter::XbergBrn]
        .into_iter()
        .flat_map(|converter| [false, true].map(|ai| DocumentConversionSettings { converter, ai }))
        .collect()
}
fn converted(settings: &DocumentConversionSettings) -> ConversionResult {
    ConversionResult {
        intent: ImportIntent::FullNote,
        markdown: "# Source\n\nExact source wording.".into(),
        assets: vec![],
        source_sha256: Sha256::digest(b"source bytes").into(),
        source_reference: "synthetic://document-settings".into(),
        converter: document_conversion_label(
            if settings.converter == DocumentConverter::Xberg {
                "xberg"
            } else {
                "xberg_brn"
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
    }
}
#[test]
fn preferences_default_stock_without_ai_and_survive_restore() {
    let root = root();
    let data = root.path().join("data");
    let mut app = Workspace::open(&data).unwrap();
    let original = app.settings().unwrap();
    assert_eq!(
        original.document_conversion,
        DocumentConversionSettings::default()
    );
    let mut old = serde_json::to_value(&original).unwrap();
    old.as_object_mut().unwrap().remove("document_conversion");
    assert_eq!(
        serde_json::from_value::<WorkspaceSettings>(old).unwrap(),
        original
    );
    for (i, settings) in variants().into_iter().enumerate() {
        app.choose_document_conversion(settings.clone()).unwrap();
        app.configure(original.clone()).unwrap();
        assert_eq!(app.settings().unwrap().document_conversion, settings);
        let backup = root.path().join(format!("snapshot-{i}"));
        app.backup(&backup).unwrap();
        let restored = root.path().join(format!("restored-{i}"));
        Store::restore(&backup, &restored).unwrap();
        assert_eq!(
            Workspace::open(&restored)
                .unwrap()
                .settings()
                .unwrap()
                .document_conversion,
            settings
        );
        assert_eq!(
            Workspace::open(&data)
                .unwrap()
                .settings()
                .unwrap()
                .document_conversion,
            settings
        );
    }
}
#[test]
fn variants_keep_one_source_and_separate_protected_notes_and_replay() {
    let root = root();
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    let variants = variants();
    let mut source = None;
    let mut first = None;
    for (i, settings) in variants.iter().enumerate() {
        let result = converted(settings);
        let key = format!("mode-{i}");
        assert!(
            app.full_import_replay_with_conversion(
                &key,
                "Source",
                &result.source_reference,
                b"source bytes",
                None,
                settings
            )
            .unwrap()
            .is_none()
        );
        app.import_full_scoped_with_conversion(&key, "Source", &result, None, settings)
            .unwrap();
        assert!(
            app.full_import_replay_with_conversion(
                &key,
                "Source",
                &result.source_reference,
                b"source bytes",
                None,
                settings
            )
            .unwrap()
            .is_some()
        );
        assert!(matches!(
            app.full_import_replay_with_conversion(
                &key,
                "Source",
                &result.source_reference,
                b"source bytes",
                None,
                &variants[(i + 1) % 4]
            ),
            Err(AppError::Core(Error::ImmutableRequest))
        ));
        let notes = app.notes().unwrap();
        assert_eq!(notes.len(), i + 1);
        for record in &notes {
            let RecordData::Note(note) = &record.data else {
                panic!()
            };
            assert!(note.protected);
            let import = note.import.as_ref().unwrap();
            if let Some(source) = &source {
                assert_eq!(source, &import.source)
            } else {
                source = Some(import.source.clone())
            };
        }
        if let Some(first) = &first {
            assert!(notes.contains(first));
        } else {
            first = Some(notes[0].clone());
        }
        app.import_full_scoped_with_conversion(
            &format!("duplicate-{i}"),
            "Source",
            &result,
            None,
            settings,
        )
        .unwrap();
        assert_eq!(app.notes().unwrap().len(), i + 1);
        assert!(
            app.full_import_replay_with_conversion(
                &format!("duplicate-{i}"),
                "Source",
                &result.source_reference,
                b"source bytes",
                None,
                settings
            )
            .unwrap()
            .is_none()
        );
        assert!(matches!(
            app.full_import_replay_with_conversion(
                &format!("duplicate-{i}"),
                "Source",
                &result.source_reference,
                b"source bytes",
                None,
                &variants[(i + 1) % 4]
            ),
            Err(AppError::Core(Error::ImmutableRequest))
        ));
    }
    let mut historic = converted(&variants[0]);
    historic.source_reference = "synthetic://historical-unlabelled".into();
    app.import_full("historic", "Source", &historic).unwrap();
    assert!(matches!(
        app.full_import_replay_with_conversion(
            "historic",
            "Source",
            &historic.source_reference,
            b"source bytes",
            None,
            &variants[0]
        ),
        Err(AppError::Core(Error::ImmutableRequest))
    ));
}
#[test]
fn selected_identity_mismatch_and_checked_settings_replay() {
    let root = root();
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    let variants = variants();
    let bad = converted(&variants[0]);
    let before = app.records().unwrap();
    assert!(import_report_with_conversion(&bad, &variants[1]).is_err());
    assert!(
        app.import_full_scoped_with_conversion("wrong", "Source", &bad, None, &variants[1])
            .is_err()
    );
    assert_eq!(app.records().unwrap(), before);
    let op = app.store.allocate_operation("checked-preference").unwrap();
    let owner = HostAuthority::owner("settings");
    app.store
        .choose_document_conversion(&op, &variants[3], &owner)
        .unwrap();
    app.store
        .choose_document_conversion(&op, &variants[3], &owner)
        .unwrap();
    assert!(matches!(
        app.store
            .choose_document_conversion(&op, &variants[0], &owner),
        Err(Error::ImmutableRequest)
    ));
}

#[test]
fn useful_source_captured_provenance_reopens_restores_and_keeps_identity() {
    use brn_ai::threads::{ThreadRunRequest, TraceLevel, TransientSource, prepare_run};
    use brn_ai::{Provider, ReasoningEffort, Selection};
    let root = root();
    let data = root.path().join("useful");
    let mut app = Workspace::open(&data).unwrap();
    let mut configured = app.settings().unwrap();
    configured.auth_file = None;
    configured.credentials_dir = None;
    app.configure(configured).unwrap();
    app.choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap();
    let chat = app.new_thread("Source observation").unwrap().id;
    let mut source_id = None;
    for (i, settings) in variants().into_iter().enumerate() {
        app.choose_document_conversion(settings.clone()).unwrap();
        let extracted = converted(&settings);
        let report = import_report_with_conversion(&extracted, &settings).unwrap();
        // The conversion job already owns `settings`. A later preference cannot
        // relabel the completed output during the useful-intake handoff.
        let next = DocumentConversionSettings {
            converter: settings.converter,
            ai: !settings.ai,
        };
        app.choose_document_conversion(next).unwrap();
        let prepared = prepare_run(ThreadRunRequest {
            request_message: None,
            queue_claim: None,
            owner_request: None,
            data_dir: data.clone(),
            thread: chat.clone(),
            prompt: "Read source without a provider dispatch".into(),
            selection: Selection {
                provider: Provider::Chatgpt,
                model: "gpt-6.1-sol".into(),
            },
            effort: ReasoningEffort::Medium,
            max_tool_rounds: 4,
            auth_file: None,
            credentials_dir: None,
            search: None,
            trace: TraceLevel::Off,
            sources: vec![TransientSource {
                reference: extracted.source_reference.clone(),
                text: extracted.markdown,
                fingerprint: Some(
                    extracted
                        .source_sha256
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect(),
                ),
                report: Some(report.clone()),
                assets: vec![],
            }],
        })
        .unwrap();
        assert!(matches!(
            app.choose_document_conversion(settings.clone()).unwrap(),
            ApplyOutcome::Deferred { .. }
        ));
        let saved=app.records().unwrap().into_iter().find(|record| matches!(&record.data,RecordData::Source(source) if source.locator==extracted.source_reference)).unwrap();
        if let Some(id) = &source_id {
            assert_eq!(&saved.id, id);
        } else {
            source_id = Some(saved.id.clone());
        }
        let RecordData::Source(source) = &saved.data else {
            panic!()
        };
        assert_eq!(source.report.as_ref(), Some(&report));
        assert_eq!(
            source.external_version.as_deref(),
            Some(
                extracted
                    .source_sha256
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .as_str()
            )
        );
        drop(prepared);
        let reopened = Workspace::open(&data).unwrap();
        assert_eq!(
            reopened.store.record(&saved.id).unwrap(),
            Some(saved.clone())
        );
        let snapshot = root.path().join(format!("useful-{i}.sqlite3"));
        reopened.backup(&snapshot).unwrap();
        let restored =
            Store::restore(&snapshot, root.path().join(format!("useful-restored-{i}"))).unwrap();
        assert_eq!(restored.record(&saved.id).unwrap(), Some(saved));
        // End the synthetic preparation before the next preference mutation.
        app.pause(true).unwrap();
        app.pause(false).unwrap();
    }
}

#[test]
fn prepared_unapplied_and_source_only_requests_check_variant_before_dispatch() {
    let root = root();
    let mut app = Workspace::open(root.path().join("pending")).unwrap();
    let settings = variants();
    let result = converted(&settings[0]);
    // Construct the same host-bound stored request through a genuine completed
    // import, then prepare its exact semantics under a separate unapplied key.
    let ApplyOutcome::Applied(receipt) = app
        .import_full_scoped_with_conversion("source", "Source", &result, None, &settings[0])
        .unwrap()
    else {
        panic!()
    };
    let pending = app.store.allocate_operation("full-import:pending").unwrap();
    let writes = receipt
        .writes
        .iter()
        .filter_map(|write| {
            if let RecordData::Source(source) = &write.after.data {
                Some(Put {
                    id: write.after.id.clone(),
                    expected_version: Some(write.after.version),
                    archived: false,
                    data: RecordData::Source(source.clone()),
                })
            } else {
                None
            }
        })
        .collect();
    app.store
        .prepare_owner(
            &pending,
            &ChangeRequest {
                reason: "Stored source-only request before apply".into(),
                writes,
                inputs: vec![],
            },
            &HostAuthority::owner("request"),
        )
        .unwrap();
    let history = app.history().unwrap();
    assert!(
        app.full_import_replay_with_conversion(
            "pending",
            "Source",
            &result.source_reference,
            b"source bytes",
            None,
            &settings[0]
        )
        .unwrap()
        .is_none()
    );
    for wrong in [&settings[1], &settings[2]] {
        assert!(matches!(
            app.full_import_replay_with_conversion(
                "pending",
                "Source",
                &result.source_reference,
                b"source bytes",
                None,
                wrong
            ),
            Err(AppError::Core(Error::ImmutableRequest))
        ));
    }
    assert_eq!(app.history().unwrap(), history);
    assert!(app.store.receipt(&pending).unwrap().is_none());
    app.store
        .apply(&pending, &HostAuthority::owner("request"))
        .unwrap();
    assert!(
        app.full_import_replay_with_conversion(
            "pending",
            "Source",
            &result.source_reference,
            b"source bytes",
            None,
            &settings[0]
        )
        .unwrap()
        .is_none()
    );
    assert!(matches!(
        app.full_import_replay_with_conversion(
            "pending",
            "Wrong title",
            &result.source_reference,
            b"source bytes",
            None,
            &settings[0]
        ),
        Err(AppError::Core(Error::ImmutableRequest))
    ));
    assert!(matches!(
        app.full_import_replay_with_conversion(
            "pending",
            "Source",
            &result.source_reference,
            b"changed bytes",
            None,
            &settings[0]
        ),
        Err(AppError::Core(Error::ImmutableRequest))
    ));
    let bad_bundle = app
        .store
        .allocate_operation("full-import:old-bundle")
        .unwrap();
    let source_record = receipt
        .writes
        .iter()
        .find(|write| matches!(write.after.data, RecordData::Source(_)))
        .unwrap()
        .after
        .clone();
    let RecordData::Source(mut source_value) = source_record.data.clone() else {
        panic!()
    };
    source_value
        .report
        .as_mut()
        .unwrap()
        .conversion
        .as_mut()
        .unwrap()
        .engine_identity = "historical-other-bundle".into();
    let current = app.store.record(&source_record.id).unwrap().unwrap();
    app.store
        .prepare_owner(
            &bad_bundle,
            &ChangeRequest {
                reason: "Previously prepared different engine bundle".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: source_record.id,
                    expected_version: Some(current.version),
                    archived: false,
                    data: RecordData::Source(source_value),
                }],
            },
            &HostAuthority::owner("historical fixture"),
        )
        .unwrap();
    assert!(matches!(
        app.full_import_replay_with_conversion(
            "old-bundle",
            "Source",
            &result.source_reference,
            b"source bytes",
            None,
            &settings[0]
        ),
        Err(AppError::Core(Error::ImmutableRequest))
    ));
    assert!(app.store.receipt(&bad_bundle).unwrap().is_none());
    // An allocated operation without preparation has no recorded provenance.
    app.store
        .allocate_operation("full-import:unprepared")
        .unwrap();
    assert!(
        app.full_import_replay_with_conversion(
            "unprepared",
            "Source",
            &result.source_reference,
            b"source bytes",
            None,
            &settings[2]
        )
        .unwrap()
        .is_none()
    );
}
