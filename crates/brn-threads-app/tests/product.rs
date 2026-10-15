use brn_threads_app::*;
use brn_threads_intake::{ConversionResult, Coverage, CoverageStatus, ImportIntent, RetainedAsset};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
fn temp() -> TempDir {
    TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap()
}
fn workspace() -> (TempDir, Workspace) {
    let dir = temp();
    let workspace = Workspace::open(dir.path().join("data")).unwrap();
    (dir, workspace)
}
fn outcome(outcome: ApplyOutcome) -> Receipt {
    let ApplyOutcome::Applied(receipt) = outcome else {
        panic!("{outcome:?}")
    };
    receipt
}
fn put(record: &Record, data: RecordData) -> Put {
    Put {
        id: record.id.clone(),
        expected_version: Some(record.version),
        archived: record.archived,
        data,
    }
}
#[test]
fn selecting_action_group_keeps_shared_source_guard_and_refuses_newer_evidence() {
    let (_dir, mut app) = workspace();
    let note = app
        .new_note("Independent summary", "Original summary")
        .unwrap();
    let source_op = app
        .store
        .allocate_operation("group source evidence")
        .unwrap();
    let source = outcome(
        app.owner_change(
            "group source evidence",
            &request(vec![Put {
                id: source_op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Source(Source {
                    chat: None,
                    locator: "synthetic://group-source".into(),
                    label: "Study".into(),
                    observed_at: "first".into(),
                    external_version: Some("v1".into()),
                    outcome: "Partial".into(),
                    gaps: vec![],
                    report: Some(ImportReport {
                        status: ImportStatus::Partial,
                        issues: vec![],
                    }),
                }),
            }]),
        )
        .unwrap(),
    )
    .writes[0]
        .after
        .clone();
    let operation = app
        .store
        .allocate_operation("shared source candidate")
        .unwrap();
    let action = operation.creation_id(0);
    let mut proposed_note = Note::working("Independent summary", "Updated summary");
    proposed_note.import = Some(ImportCoverage {
        source: source.id.clone(),
        full_note: false,
        gaps: vec![],
        report: None,
    });
    let mut req = request(vec![
        put(&note, RecordData::Note(proposed_note)),
        Put {
            id: action.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Action(Action {
                kept_outside_project: false,
                chat: None,
                description: "Check supported evidence".into(),
                state: ActionState::Suggested,
                internal: true,
                evidence: None,
                due: None,
            }),
        },
        put(&source, source.data.clone()),
    ]);
    req.inputs.push(RevisionInput {
        expectation: InputExpectation::Current,
        record: source.id.clone(),
        version: source.version,
    });
    app.store.prepare(&operation, &req).unwrap();
    let groups = app.review_groups(operation.as_str()).unwrap();
    assert!(groups.ungrouped_writes.is_empty());
    assert!(
        groups
            .groups
            .iter()
            .all(|group| group.writes.contains(&source.id))
    );
    let selected = app.review_select(operation.as_str(), &[action]).unwrap();
    assert!(
        selected
            .request
            .writes
            .iter()
            .any(|write| write.id == source.id && write.expected_version == Some(source.version))
    );
    assert!(
        !selected
            .request
            .writes
            .iter()
            .any(|write| write.id == note.id)
    );
    let RecordData::Source(mut newer) = source.data.clone() else {
        panic!()
    };
    newer.external_version = Some("v2".into());
    outcome(
        app.owner_change(
            "source changed after group selection",
            &request(vec![put(&source, RecordData::Source(newer))]),
        )
        .unwrap(),
    );
    assert!(
        matches!(app.review_apply(selected.operation.as_str()).unwrap(), ApplyOutcome::Stale { records } if records.contains(&source.id))
    );
    assert!(
        app.records()
            .unwrap()
            .iter()
            .all(|record| !matches!(record.data, RecordData::Action(_)))
    );
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
}
#[test]
fn independent_whole_note_and_action_selection_requires_new_displayed_candidate() {
    let (_dir, mut app) = workspace();
    let note = app
        .new_note("Protected wording", "Preserve original wording")
        .unwrap();
    let RecordData::Note(mut protected) = note.data.clone() else {
        panic!()
    };
    protected.protected = true;
    outcome(
        app.owner_change(
            "protect selection fixture",
            &request(vec![put(&note, RecordData::Note(protected))]),
        )
        .unwrap(),
    );
    let note = app.store.record(&note.id).unwrap().unwrap();
    let action_op = app
        .store
        .allocate_operation("human Action fixture")
        .unwrap();
    let action = action_op.creation_id(0);
    outcome(
        app.owner_change(
            "human Action fixture",
            &request(vec![Put {
                id: action.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Action(Action {
                    kept_outside_project: false,
                    chat: None,
                    description: "Human checks wording".into(),
                    state: ActionState::Waiting,
                    internal: false,
                    evidence: None,
                    due: None,
                }),
            }]),
        )
        .unwrap(),
    );
    let action = app.store.record(&action).unwrap().unwrap();
    let original = app
        .store
        .allocate_operation("independent note Action proposal")
        .unwrap();
    let RecordData::Note(mut proposed) = note.data.clone() else {
        panic!()
    };
    proposed.markdown = format!(
        "Complete selected wording. ![Managed](brn-asset://{})",
        original.creation_id(0)
    );
    let RecordData::Action(mut done) = action.data.clone() else {
        panic!()
    };
    done.state = ActionState::Done;
    done.evidence = Some("Owner review confirms the human result".into());
    let original_request = request(vec![
        put(&note, RecordData::Note(proposed)),
        put(&action, RecordData::Action(done)),
        Put {
            id: original.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Asset(Asset {
                note: note.id.clone(),
                source: None,
                media_type: "image/png".into(),
                bytes: vec![1, 2, 3],
            }),
        },
    ]);
    app.store.prepare(&original, &original_request).unwrap();
    let groups = app.review_groups(original.as_str()).unwrap();
    assert_eq!(groups.groups.len(), 2);
    assert!(groups.ungrouped_writes.is_empty());
    assert!(
        groups
            .groups
            .iter()
            .all(|group| group.required_groups.is_empty())
    );
    let selected = app
        .review_select(original.as_str(), std::slice::from_ref(&note.id))
        .unwrap();
    assert_ne!(selected.operation, original);
    assert_ne!(
        selected.request_hash,
        app.store.prepared(&original).unwrap().request_hash
    );
    assert_eq!(
        app.store.prepared(&original).unwrap().request,
        original_request
    );
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
    assert_eq!(app.store.record(&action.id).unwrap().unwrap(), action);
    assert!(app.review_apply(original.as_str()).is_err());
    assert!(
        selected
            .request
            .writes
            .iter()
            .any(|write| matches!(write.data, RecordData::Asset(_)))
    );
    assert!(
        !selected
            .request
            .writes
            .iter()
            .any(|write| write.id == action.id)
    );
    let applied = outcome(app.review_apply(selected.operation.as_str()).unwrap());
    assert_eq!(app.store.record(&action.id).unwrap().unwrap(), action);
    assert!(
        matches!(&app.store.record(&note.id).unwrap().unwrap().data, RecordData::Note(note) if note.markdown.contains("Complete selected wording"))
    );
    outcome(app.undo(applied.operation.as_str()).unwrap());
    assert_eq!(app.store.record(&action.id).unwrap().unwrap(), action);
    let current = app.store.record(&note.id).unwrap().unwrap();
    let next = app
        .store
        .allocate_operation("select only Action independently")
        .unwrap();
    let RecordData::Note(mut excluded) = current.data.clone() else {
        panic!()
    };
    excluded.markdown = "Must remain excluded".into();
    let RecordData::Action(mut done) = action.data.clone() else {
        panic!()
    };
    done.state = ActionState::Done;
    done.evidence = Some("Explicit owner completion review".into());
    app.store
        .prepare(
            &next,
            &request(vec![
                put(&current, RecordData::Note(excluded)),
                put(&action, RecordData::Action(done)),
            ]),
        )
        .unwrap();
    let selected = app
        .review_select(next.as_str(), std::slice::from_ref(&action.id))
        .unwrap();
    outcome(app.review_apply(selected.operation.as_str()).unwrap());
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), current);
    assert!(
        matches!(app.store.record(&action.id).unwrap().unwrap().data, RecordData::Action(action) if action.state == ActionState::Done)
    );
}

#[test]
fn review_selection_preserves_structural_new_record_dependencies_and_exact_bases() {
    let (_dir, mut app) = workspace();
    let original = app.store.allocate_operation("coupled new groups").unwrap();
    let first = original.creation_id(0);
    let second = original.creation_id(1);
    app.store
        .prepare(
            &original,
            &request(vec![
                Put {
                    id: first.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("New note", "Whole content")),
                },
                Put {
                    id: second.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Action(Action {
                        kept_outside_project: false,
                        chat: None,
                        description: "New suggested work".into(),
                        state: ActionState::Suggested,
                        internal: false,
                        evidence: None,
                        due: None,
                    }),
                },
                Put {
                    id: original.creation_id(2),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Link(Link {
                        from: first.clone(),
                        to: second.clone(),
                        relation: "related".into(),
                    }),
                },
            ]),
        )
        .unwrap();
    let groups = app.review_groups(original.as_str()).unwrap();
    assert!(
        groups
            .groups
            .iter()
            .find(|group| group.id == first)
            .unwrap()
            .required_groups
            .contains(&second)
    );
    assert!(
        app.review_select(original.as_str(), std::slice::from_ref(&first))
            .is_err()
    );
    assert_eq!(app.candidates().unwrap()[0].operation, original);
    let selected = app
        .review_select(original.as_str(), &[first, second])
        .unwrap();
    assert!(
        selected
            .request
            .writes
            .iter()
            .all(|write| write.id.starts_with(selected.operation.as_str()))
    );
    outcome(app.review_apply(selected.operation.as_str()).unwrap());
    let existing = app.new_note("Exact base", "Initial").unwrap();
    let independent = app.new_note("Independent", "Other").unwrap();
    let original = app.store.allocate_operation("stale subset").unwrap();
    let RecordData::Note(mut proposed) = existing.data.clone() else {
        panic!()
    };
    proposed.markdown = "Candidate from old base".into();
    app.store
        .prepare(
            &original,
            &request(vec![
                put(&existing, RecordData::Note(proposed)),
                put(&independent, independent.data.clone()),
            ]),
        )
        .unwrap();
    let session = app
        .store
        .begin_edit(&existing.id, existing.version)
        .unwrap();
    app.store
        .update_buffer(&session.id, 1, "Later owner text")
        .unwrap();
    app.store.save(&session.id, 1).unwrap();
    let selected = app
        .review_select(original.as_str(), std::slice::from_ref(&existing.id))
        .unwrap();
    assert!(matches!(
        app.review_apply(selected.operation.as_str()).unwrap(),
        ApplyOutcome::Stale { .. }
    ));
    assert!(
        matches!(app.store.record(&existing.id).unwrap().unwrap().data, RecordData::Note(note) if note.markdown == "Later owner text")
    );
}
fn request(writes: Vec<Put>) -> ChangeRequest {
    ChangeRequest {
        reason: "synthetic product witness".into(),
        writes,
        inputs: vec![],
    }
}
#[test]
fn update_and_undo_expose_direct_work_and_one_recoverable_refresh_followup() {
    let (dir, mut app) = workspace();
    outcome(
        app.choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
            .unwrap(),
    );
    let input = app.new_note("Input evidence", "Original fact").unwrap();
    let derived = app
        .store
        .allocate_operation("derive summary and suggested Action")
        .unwrap();
    let mut req = request(vec![
        Put {
            id: derived.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note::working("Derived summary", "Based on original fact")),
        },
        Put {
            id: derived.creation_id(1),
            expected_version: None,
            archived: false,
            data: RecordData::Action(Action {
                kept_outside_project: false,
                chat: None,
                description: "Consider original fact".into(),
                state: ActionState::Suggested,
                internal: false,
                evidence: None,
                due: None,
            }),
        },
    ]);
    req.inputs.push(RevisionInput {
        expectation: InputExpectation::Current,
        record: input.id.clone(),
        version: input.version,
    });
    app.store.prepare(&derived, &req).unwrap();
    outcome(
        app.store
            .apply(&derived, &HostAuthority::maintenance("fixture"))
            .unwrap(),
    );
    let session = app.store.begin_edit(&input.id, input.version).unwrap();
    app.store
        .update_buffer(&session.id, 1, "Meaningful newer fact")
        .unwrap();
    let SaveOutcome::Saved(receipt) = app.store.save(&session.id, 1).unwrap() else {
        panic!()
    };
    let expected = vec![derived.creation_id(0), derived.creation_id(1)];
    assert_eq!(receipt.needs_refresh, expected);
    assert_eq!(
        app.refresh_records(receipt.operation.as_str())
            .unwrap()
            .iter()
            .map(|record| record.id.clone())
            .collect::<Vec<_>>(),
        expected
    );
    let followup = app.refresh_followup(receipt.operation.as_str()).unwrap();
    assert!(
        matches!(&followup.data, RecordData::Thread(thread) if thread.state == ThreadState::Open && thread.attention.is_empty())
    );
    assert_eq!(
        app.refresh_followup(receipt.operation.as_str()).unwrap(),
        followup
    );
    let texts = app.messages(&followup.id).unwrap();
    assert!(
        matches!(&texts[0].data, RecordData::Message(message) if message.text.contains("does not itself change"))
    );
    assert!(app.records().unwrap().iter().any(|record| matches!(&record.data, RecordData::Link(link) if link.from == followup.id && link.to == input.id && link.relation == "changed-input")));
    drop(app);
    let mut app = Workspace::open(dir.path().join("data")).unwrap();
    assert_eq!(
        app.refresh_followup(receipt.operation.as_str()).unwrap(),
        followup
    );
    let reversal = outcome(app.undo(receipt.operation.as_str()).unwrap());
    assert_eq!(reversal.needs_refresh, expected);
    assert!(
        matches!(app.store.record(&derived.creation_id(0)).unwrap().unwrap().data, RecordData::Note(note) if note.markdown == "Based on original fact")
    );
}
#[test]
fn dismiss_stale_proposal_releases_slot_preserves_current_and_other_attention_on_retry() {
    let (directory, mut app) = workspace();
    let note = app.new_note("Working plan", "Monday").unwrap();
    let proposal = app.store.allocate_operation("old proposal").unwrap();
    let RecordData::Note(mut proposed) = note.data.clone() else {
        panic!()
    };
    proposed.markdown = "Tuesday".into();
    app.store
        .prepare(
            &proposal,
            &request(vec![put(&note, RecordData::Note(proposed))]),
        )
        .unwrap();
    let thread = app.new_thread("Review plan").unwrap();
    let RecordData::Thread(mut discussion) = thread.data.clone() else {
        panic!()
    };
    discussion.attention = vec![
        Attention {
            kind: AttentionKind::Review,
            reason: "Review old proposal".into(),
            record: Some(proposal.as_str().into()),
        },
        Attention {
            kind: AttentionKind::Question,
            reason: "Independent question".into(),
            record: Some(note.id.clone()),
        },
    ];
    outcome(
        app.owner_change(
            "attention",
            &request(vec![put(&thread, RecordData::Thread(discussion))]),
        )
        .unwrap(),
    );
    let session = app.store.begin_edit(&note.id, note.version).unwrap();
    app.store
        .update_buffer(&session.id, 1, "Wednesday owner edit")
        .unwrap();
    assert!(matches!(
        app.store.save(&session.id, 1).unwrap(),
        SaveOutcome::Saved(_)
    ));
    assert!(matches!(
        app.review_apply(proposal.as_str()).unwrap(),
        ApplyOutcome::Stale { .. }
    ));
    app.review_dismiss(proposal.as_str()).unwrap();
    assert!(
        app.history()
            .unwrap()
            .last()
            .unwrap()
            .reason
            .starts_with("Dismissed proposal")
    );
    drop(app);
    let mut app = Workspace::open(directory.path().join("data")).unwrap();
    app.review_dismiss(proposal.as_str()).unwrap();
    let current = app.store.record(&note.id).unwrap().unwrap();
    let RecordData::Note(mut data) = current.data.clone() else {
        panic!()
    };
    assert_eq!(data.markdown, "Wednesday owner edit");
    assert_eq!(current.version, 2);
    let RecordData::Thread(discussion) = app.store.record(&thread.id).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(discussion.attention.len(), 1);
    assert_eq!(discussion.attention[0].reason, "Independent question");
    assert!(app.review_apply(proposal.as_str()).is_err());
    let next = app.store.allocate_operation("fresh proposal").unwrap();
    data.markdown = "Thursday new proposal".into();
    app.store
        .prepare(&next, &request(vec![put(&current, RecordData::Note(data))]))
        .unwrap();
    outcome(app.review_apply(next.as_str()).unwrap());
    assert!(app.review_dismiss(next.as_str()).is_err());
}
fn converted() -> ConversionResult {
    let bytes = vec![1, 2, 3, 4];
    ConversionResult{intent:ImportIntent::FullNote,markdown:"# Complete process\n\nFirst step: retain all substantive prose.\n\n| Input | Result |\n| --- | --- |\n| One | Two |\n\n![Managed diagram](diagram.png)\n".into(),assets:vec![RetainedAsset{name:"diagram.png".into(),media_type:"image/png".into(),sha256:Sha256::digest(&bytes).into(),width:1,height:1,bytes}],source_sha256:Sha256::digest(b"synthetic original bytes").into(),source_reference:"external://process-document".into(),converter:"synthetic fixture".into(),sources:vec![],coverage:Coverage{status:CoverageStatus::Complete,checked_items:vec!["prose".into(),"table".into(),"figure".into()],gaps:vec![]}}
}
#[test]
fn current_search_excludes_pending_revisions_and_rebuilds_after_save_archive_restore() {
    let (dir, mut app) = workspace();
    let a = app
        .new_note("Plan", "Estonian project õun and originalneedle")
        .unwrap();
    assert_eq!(app.search("originalneedle").unwrap()[0].note, a.id);
    let pending = app.store.allocate_operation("pending candidate").unwrap();
    let RecordData::Note(mut proposed) = a.data.clone() else {
        panic!()
    };
    proposed.markdown = "pendingonlyneedle".into();
    app.store
        .prepare(
            &pending,
            &request(vec![put(&a, RecordData::Note(proposed))]),
        )
        .unwrap();
    assert!(app.search("pendingonlyneedle").unwrap().is_empty());
    let session = app.store.begin_edit(&a.id, a.version).unwrap();
    app.store
        .update_buffer(&session.id, 1, "currentonlyneedle õun")
        .unwrap();
    assert!(matches!(
        app.store.save(&session.id, 1).unwrap(),
        SaveOutcome::Saved(_)
    ));
    assert!(app.search("originalneedle").unwrap().is_empty());
    let hits = app.search("currentonlyneedle").unwrap();
    let current = app.store.record(&a.id).unwrap().unwrap();
    assert_eq!(hits[0].version, current.version);
    let RecordData::Note(n) = &current.data else {
        panic!()
    };
    assert_eq!(
        n.markdown.get(hits[0].start..hits[0].end),
        Some(hits[0].quote.as_str())
    );
    let backup = dir.path().join("backup.sqlite3");
    app.backup(&backup).unwrap();
    let mut archived = put(&current, current.data.clone());
    archived.archived = true;
    outcome(
        app.owner_change("archive note", &request(vec![archived]))
            .unwrap(),
    );
    assert!(app.search("currentonlyneedle").unwrap().is_empty());
    let restored = dir.path().join("restored");
    drop(Store::restore(&backup, &restored).unwrap());
    let mut app = Workspace::open(&restored).unwrap();
    assert_eq!(app.search("currentonlyneedle").unwrap()[0].note, a.id);
}
#[test]
fn full_import_retains_assets_source_identity_and_rejects_changed_retry_input() {
    let (dir, mut app) = workspace();
    let input = converted();
    let receipt = outcome(
        app.import_full("import identity", "Process", &input)
            .unwrap(),
    );
    assert_eq!(
        app.import_full("import identity", "Process", &input)
            .unwrap(),
        ApplyOutcome::Applied(receipt.clone())
    );
    let records = app.records().unwrap();
    let note = records
        .iter()
        .find(|r| matches!(r.data, RecordData::Note(_)))
        .unwrap()
        .clone();
    let RecordData::Note(n) = &note.data else {
        panic!()
    };
    assert!(n.protected);
    assert!(!n.confirmed);
    assert!(
        n.markdown
            .contains("First step: retain all substantive prose")
    );
    assert!(n.markdown.contains("| One | Two |"));
    assert!(n.markdown.contains("brn-asset://"));
    let source = records
        .iter()
        .find(|r| matches!(r.data, RecordData::Source(_)))
        .unwrap();
    let RecordData::Source(s) = &source.data else {
        panic!()
    };
    assert_eq!(
        s.external_version,
        Some(
            input
                .source_sha256
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        )
    );
    let mut changed = input.clone();
    changed.markdown.push_str("different conversion result");
    assert!(matches!(
        app.import_full("import identity", "Process", &changed),
        Err(AppError::Core(Error::ImmutableRequest))
    ));
    assert!(matches!(
        app.import_full("import identity", "Different title", &input),
        Err(AppError::Core(Error::ImmutableRequest))
    ));
    outcome(
        app.import_full("second host request", "Process", &input)
            .unwrap(),
    );
    assert_eq!(app.notes().unwrap().len(), 1);
    assert_eq!(
        app.store.record(&note.id).unwrap().unwrap().version,
        note.version
    );
    assert_eq!(
        app.records()
            .unwrap()
            .iter()
            .filter(|r| matches!(r.data, RecordData::Source(_)))
            .count(),
        1
    );
    assert_eq!(
        app.records()
            .unwrap()
            .iter()
            .filter(|r| matches!(r.data, RecordData::Asset(_)))
            .count(),
        1
    );
    let data = app.store.directory().to_owned();
    drop(app);
    let mut app = Workspace::open(data).unwrap();
    outcome(
        app.import_full("second host request", "Process", &input)
            .unwrap(),
    );
    assert_eq!(app.notes().unwrap().len(), 1);
    let snapshot = dir.path().join("snapshot");
    let manifest = app.export_now(&snapshot).unwrap();
    let note_file = manifest
        .files
        .iter()
        .find(|file| file.record == note.id)
        .unwrap();
    let markdown = std::fs::read_to_string(snapshot.join(&note_file.path)).unwrap();
    assert!(markdown.contains("assets/"));
    assert!(!markdown.contains("brn-asset://"));
    let figure = manifest
        .files
        .iter()
        .find(|file| file.path.starts_with("assets/"))
        .unwrap();
    assert_eq!(
        std::fs::read(snapshot.join(&figure.path)).unwrap(),
        input.assets[0].bytes
    );
    let backup = dir.path().join("full-backup.sqlite3");
    app.backup(&backup).unwrap();
    let bytes = std::fs::read(&backup).unwrap();
    assert!(app.backup(&backup).is_err());
    assert_eq!(std::fs::read(&backup).unwrap(), bytes);
    assert!(!std::fs::read_dir(dir.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".brn-backup-stage-")
    }));
    app.new_note("Later source note", "Must not enter the earlier snapshot")
        .unwrap();
    let restored = dir.path().join("restored-full");
    drop(Store::restore(&backup, &restored).unwrap());
    let restored = Workspace::open(&restored).unwrap();
    let restored_manifest = restored
        .export_now(&dir.path().join("restored-snapshot"))
        .unwrap();
    assert_eq!(
        serde_json::to_value(restored_manifest).unwrap(),
        serde_json::to_value(&manifest).unwrap()
    );
}
#[test]
fn export_refuses_occupied_or_incomplete_outputs_without_partial_publication() {
    let (dir, mut app) = workspace();
    app.new_note("Broken figure", "![Missing](brn-asset://missing)")
        .unwrap();
    let fresh = dir.path().join("failed-snapshot");
    assert!(app.export_now(&fresh).is_err());
    assert!(!fresh.exists());
    assert!(!std::fs::read_dir(dir.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".brn-export-")
    }));
    let occupied = dir.path().join("occupied");
    std::fs::create_dir(&occupied).unwrap();
    std::fs::write(occupied.join("owner.txt"), "keep me").unwrap();
    assert!(app.export_now(&occupied).is_err());
    assert_eq!(
        std::fs::read_to_string(occupied.join("owner.txt")).unwrap(),
        "keep me"
    );
}
#[test]
fn configuration_and_pause_fence_runs_while_thread_resolution_preserves_actions() {
    let (_dir, mut app) = workspace();
    assert_eq!(app.settings().unwrap().provider, "Codex");
    assert_eq!(app.settings().unwrap().model, "gpt-6.1-sol");
    let thread = app.new_thread("Prepare report").unwrap();
    let op = app
        .store
        .allocate_operation("host run and human Action")
        .unwrap();
    let run = Run {
        instruction: None,
        queue_claim: None,
        thread: thread.id.clone(),
        provider: "Codex".into(),
        model: "gpt-6.1-sol".into(),
        guide_identity: "guide".into(),
        budget: 3,
        effort: "medium".into(),
        loaded_skills: vec![],
        fence: 1,
        state: RunState::Working,
        progress: "starting".into(),
        operations: vec![],
        sources: vec![],
    };
    let action = Action {
        kept_outside_project: false,
        chat: None,
        description: "Human sends report".into(),
        state: ActionState::Waiting,
        internal: false,
        evidence: None,
        due: None,
    };
    let req = request(vec![
        Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Run(run),
        },
        Put {
            id: op.creation_id(1),
            expected_version: None,
            archived: false,
            data: RecordData::Action(action),
        },
    ]);
    app.store
        .prepare_owner(&op, &req, &HostAuthority::owner("host"))
        .unwrap();
    outcome(app.store.apply(&op, &HostAuthority::owner("host")).unwrap());
    let receipt = outcome(app.pause(true).unwrap());
    assert!(
        receipt
            .writes
            .iter()
            .any(|w| matches!(w.after.data, RecordData::Settings(_)))
    );
    let RecordData::Run(stopped) = app.store.record(&op.creation_id(0)).unwrap().unwrap().data
    else {
        panic!()
    };
    assert_eq!(stopped.state, RunState::Cancelled);
    assert_eq!(stopped.fence, 2);
    assert!(!app.settings().unwrap().maintenance);
    let mut settings = app.settings().unwrap();
    settings.model = "explicit different model".into();
    outcome(app.configure(settings).unwrap());
    assert_eq!(app.settings().unwrap().model, "explicit different model");
    outcome(app.resolve_thread(&thread.id).unwrap());
    let RecordData::Action(action) = app.store.record(&op.creation_id(1)).unwrap().unwrap().data
    else {
        panic!()
    };
    assert_eq!(action.state, ActionState::Waiting);
}
#[test]
fn message_history_uses_durable_receipt_order_and_preserves_creation_order_after_edit() {
    let (_dir, mut app) = workspace();
    let thread = app.new_thread("Conversation").unwrap();
    let first = app.post_message(&thread.id, "first").unwrap();
    let second = app.post_message(&thread.id, "second").unwrap();
    let third = app.post_message(&thread.id, "third").unwrap();
    let first_record = app
        .store
        .record(&first.writes[0].after.id)
        .unwrap()
        .unwrap();
    let RecordData::Message(mut message) = first_record.data.clone() else {
        panic!()
    };
    message.text = "first edited later".into();
    outcome(
        app.owner_change(
            "edit first message",
            &request(vec![put(&first_record, RecordData::Message(message))]),
        )
        .unwrap(),
    );
    let rows = app.messages(&thread.id).unwrap();
    assert_eq!(
        rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
        vec![
            first.writes[0].after.id.clone(),
            second.writes[0].after.id.clone(),
            third.writes[0].after.id.clone()
        ]
    );
}
#[test]
fn review_replacement_remaps_created_records_refs_and_applies_exact_final_text() {
    let (_dir, mut app) = workspace();
    let original = app
        .store
        .allocate_operation("review creation group")
        .unwrap();
    let source = Source {
        chat: None,
        locator: "external://review".into(),
        label: "review".into(),
        observed_at: "today".into(),
        external_version: None,
        outcome: "full".into(),
        gaps: vec![],
        report: None,
    };
    let mut note = Note::working(
        "Candidate",
        format!("draft ![figure](brn-asset://{})", original.creation_id(2)),
    );
    note.protected = true;
    note.import = Some(ImportCoverage {
        source: original.creation_id(1),
        full_note: true,
        gaps: vec![],
        report: None,
    });
    let req = request(vec![
        Put {
            id: original.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Note(note),
        },
        Put {
            id: original.creation_id(1),
            expected_version: None,
            archived: false,
            data: RecordData::Source(source),
        },
        Put {
            id: original.creation_id(2),
            expected_version: None,
            archived: false,
            data: RecordData::Asset(Asset {
                note: original.creation_id(0),
                source: Some(original.creation_id(1)),
                media_type: "image/png".into(),
                bytes: vec![1, 2, 3],
            }),
        },
        Put {
            id: original.creation_id(3),
            expected_version: None,
            archived: false,
            data: RecordData::Link(Link {
                from: original.creation_id(0),
                to: original.creation_id(1),
                relation: "source".into(),
            }),
        },
    ]);
    app.store.prepare(&original, &req).unwrap();
    let replacement = app
        .review_replace(
            original.as_str(),
            &original.creation_id(0),
            &format!(
                "final owner text ![figure](brn-asset://{})",
                original.creation_id(2)
            ),
        )
        .unwrap();
    assert_eq!(app.candidates().unwrap().len(), 1);
    assert_ne!(replacement.operation, original);
    assert!(app.review_apply(original.as_str()).is_err());
    outcome(app.review_apply(replacement.operation.as_str()).unwrap());
    let rows = app.records().unwrap();
    let imported = rows
        .iter()
        .find(|r| matches!(r.data, RecordData::Note(_)))
        .unwrap();
    let RecordData::Note(n) = &imported.data else {
        panic!()
    };
    assert!(n.markdown.starts_with("final owner text"));
    assert!(!n.markdown.contains(original.as_str()));
    for row in rows {
        assert!(!row.id.starts_with(original.as_str()));
    }
}
#[test]
fn export_keeps_asset_id_prefixes_distinct() {
    let (dir, mut app) = workspace();
    let op = app.store.allocate_operation("prefix assets").unwrap();
    let note = Note::working(
        "Figures",
        format!(
            "![One](brn-asset://{})\n![Twenty](brn-asset://{})",
            op.creation_id(2),
            op.creation_id(20)
        ),
    );
    let req = request(vec![
        Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Note(note),
        },
        Put {
            id: op.creation_id(2),
            expected_version: None,
            archived: false,
            data: RecordData::Asset(Asset {
                note: op.creation_id(0),
                source: None,
                media_type: "image/png".into(),
                bytes: vec![2],
            }),
        },
        Put {
            id: op.creation_id(20),
            expected_version: None,
            archived: false,
            data: RecordData::Asset(Asset {
                note: op.creation_id(0),
                source: None,
                media_type: "image/png".into(),
                bytes: vec![20],
            }),
        },
    ]);
    app.store
        .prepare_owner(&op, &req, &HostAuthority::owner("owner"))
        .unwrap();
    outcome(
        app.store
            .apply(&op, &HostAuthority::owner("owner"))
            .unwrap(),
    );
    let destination = dir.path().join("prefix-snapshot");
    let manifest = app.export_now(&destination).unwrap();
    let note = manifest
        .files
        .iter()
        .find(|file| file.record == op.creation_id(0))
        .unwrap();
    let markdown = std::fs::read_to_string(destination.join(&note.path)).unwrap();
    for id in [op.creation_id(2), op.creation_id(20)] {
        let asset = manifest
            .files
            .iter()
            .find(|file| file.record == id)
            .unwrap();
        assert!(markdown.contains(&format!("]({})", asset.path)));
    }
}
#[test]
fn accepted_review_clears_only_matching_attention_and_replay_finishes_cleanup_once() {
    let (_dir, mut app) = workspace();
    let note = app.new_note("Review note", "before").unwrap();
    let op = app
        .store
        .allocate_operation("review with attention")
        .unwrap();
    let RecordData::Note(mut n) = note.data.clone() else {
        panic!()
    };
    n.markdown = "accepted candidate".into();
    app.store
        .prepare(&op, &request(vec![put(&note, RecordData::Note(n))]))
        .unwrap();
    let thread = app.new_thread("Review and question").unwrap();
    let RecordData::Thread(mut t) = thread.data.clone() else {
        panic!()
    };
    t.attention = vec![
        Attention {
            kind: AttentionKind::Review,
            reason: "accept revision".into(),
            record: Some(op.as_str().into()),
        },
        Attention {
            kind: AttentionKind::Question,
            reason: "separate owner question".into(),
            record: Some(note.id.clone()),
        },
    ];
    outcome(
        app.owner_change(
            "attach exact review attention",
            &request(vec![put(&thread, RecordData::Thread(t))]),
        )
        .unwrap(),
    );
    // Lose the accepted review response before its app cleanup follow-up.
    outcome(
        app.store
            .apply(&op, &HostAuthority::owner("owner review"))
            .unwrap(),
    );
    let applied = app.review_apply(op.as_str()).unwrap();
    assert!(matches!(applied, ApplyOutcome::Applied(_)));
    let RecordData::Thread(t) = app.store.record(&thread.id).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(t.attention.len(), 1);
    assert_eq!(t.attention[0].reason, "separate owner question");
    let history = app.history().unwrap().len();
    assert_eq!(app.review_apply(op.as_str()).unwrap(), applied);
    assert_eq!(app.history().unwrap().len(), history);
    assert_eq!(app.needs_you().unwrap()[0].id, thread.id);
}
#[test]
fn replacement_attention_follows_new_candidate_and_acceptance_preserves_other_questions() {
    let (_dir, mut app) = workspace();
    let note = app.new_note("Draft", "before").unwrap();
    let op = app
        .store
        .allocate_operation("candidate to replace")
        .unwrap();
    let RecordData::Note(mut n) = note.data.clone() else {
        panic!()
    };
    n.markdown = "model draft".into();
    app.store
        .prepare(&op, &request(vec![put(&note, RecordData::Note(n))]))
        .unwrap();
    let thread = app.new_thread("Review draft").unwrap();
    let RecordData::Thread(mut t) = thread.data.clone() else {
        panic!()
    };
    t.attention = vec![
        Attention {
            kind: AttentionKind::Review,
            reason: "review exact candidate".into(),
            record: Some(op.as_str().into()),
        },
        Attention {
            kind: AttentionKind::Question,
            reason: "separate question".into(),
            record: Some(note.id.clone()),
        },
    ];
    outcome(
        app.owner_change(
            "attach replacement attention",
            &request(vec![put(&thread, RecordData::Thread(t))]),
        )
        .unwrap(),
    );
    let replacement = app
        .review_replace(op.as_str(), &note.id, "owner final draft")
        .unwrap();
    let RecordData::Thread(t) = app.store.record(&thread.id).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(
        t.attention[0].record,
        Some(replacement.operation.as_str().into())
    );
    assert_eq!(t.attention[1].reason, "separate question");
    outcome(app.review_apply(replacement.operation.as_str()).unwrap());
    let RecordData::Thread(t) = app.store.record(&thread.id).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(t.attention.len(), 1);
    assert_eq!(t.attention[0].reason, "separate question");
}

#[test]
fn export_rejects_complete_unknown_asset_url_even_when_known_identity_is_its_prefix() {
    let (dir, mut app) = workspace();
    let op = app
        .store
        .allocate_operation("exact asset export witness")
        .unwrap();
    let req = request(vec![
        Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note::working(
                "known and missing figure",
                format!(
                    "![Known](brn-asset://{})\n![Missing](brn-asset://{})",
                    op.creation_id(2),
                    op.creation_id(200)
                ),
            )),
        },
        Put {
            id: op.creation_id(2),
            expected_version: None,
            archived: false,
            data: RecordData::Asset(Asset {
                note: op.creation_id(0),
                source: None,
                media_type: "image/png".into(),
                bytes: vec![2],
            }),
        },
    ]);
    app.store
        .prepare_owner(&op, &req, &HostAuthority::owner("fixture"))
        .unwrap();
    outcome(
        app.store
            .apply(&op, &HostAuthority::owner("fixture"))
            .unwrap(),
    );
    let destination = dir.path().join("missing-prefix-snapshot");
    assert!(app.export_now(&destination).is_err());
    assert!(!destination.exists());
    assert!(std::fs::read_dir(dir.path()).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".brn-export-")
    }));
    let current = app.store.record(&op.creation_id(0)).unwrap().unwrap();
    for suffix in [".png00", "/tail", "?query=1"] {
        let markdown = format!("![Invalid](brn-asset://{}{suffix})", op.creation_id(2));
        let change = request(vec![put(
            &current,
            RecordData::Note(Note::working("missing", markdown)),
        )]);
        // Each independent fixture candidate gets the current base before changing it.
        let current = app.store.record(&current.id).unwrap().unwrap();
        let mut change = change;
        change.writes[0].expected_version = Some(current.version);
        outcome(
            app.owner_change(&format!("missing asset suffix {suffix}"), &change)
                .unwrap(),
        );
        assert!(app.export_now(&destination).is_err());
        assert!(!destination.exists());
    }
}
#[test]
fn review_remap_requires_exact_known_asset_token_and_keeps_failed_original_active() {
    let (_dir, mut app) = workspace();
    let original = app
        .store
        .allocate_operation("exact review asset witness")
        .unwrap();
    let req = request(vec![
        Put {
            id: original.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note::working(
                "figure candidate",
                format!("![Known](brn-asset://{})", original.creation_id(2)),
            )),
        },
        Put {
            id: original.creation_id(2),
            expected_version: None,
            archived: false,
            data: RecordData::Asset(Asset {
                note: original.creation_id(0),
                source: None,
                media_type: "image/png".into(),
                bytes: vec![2],
            }),
        },
    ]);
    app.store.prepare(&original, &req).unwrap();
    let missing = format!("![Missing](brn-asset://{})", original.creation_id(200));
    assert!(
        app.review_replace(original.as_str(), &original.creation_id(0), &missing)
            .is_err()
    );
    assert_eq!(app.store.candidates().unwrap()[0].operation, original);
    let exact = format!(
        "Owner final ![Known](brn-asset://{})",
        original.creation_id(2)
    );
    let replacement = app
        .review_replace(original.as_str(), &original.creation_id(0), &exact)
        .unwrap();
    let asset = replacement
        .request
        .writes
        .iter()
        .find(|write| matches!(write.data, RecordData::Asset(_)))
        .unwrap();
    let note = replacement
        .request
        .writes
        .iter()
        .find_map(|write| {
            if let RecordData::Note(note) = &write.data {
                Some(note)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        note.markdown,
        format!("Owner final ![Known](brn-asset://{})", asset.id)
    );
    assert!(!note.markdown.contains(&original.creation_id(2)));
    outcome(app.review_apply(replacement.operation.as_str()).unwrap());
}

#[test]
fn selected_review_groups_and_replacement_preserve_current_or_historical_note_inputs() {
    for expectation in [InputExpectation::Current, InputExpectation::Historical] {
        let (_dir, mut app) = workspace();
        let input = app.new_note("Evidence", "Monday").unwrap();
        let operation = app
            .store
            .allocate_operation("derived review groups")
            .unwrap();
        let note_id = operation.creation_id(0);
        let action_id = operation.creation_id(1);
        let mut proposed = request(vec![
            Put {
                id: note_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Note(Note::working("Summary", "Delivery Monday")),
            },
            Put {
                id: action_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Action(Action {
                    kept_outside_project: false,
                    chat: None,
                    description: "Prepare delivery Monday".into(),
                    state: ActionState::Suggested,
                    internal: true,
                    evidence: None,
                    due: None,
                }),
            },
        ]);
        proposed.inputs.push(RevisionInput {
            record: input.id.clone(),
            version: input.version,
            expectation,
        });
        app.store.prepare(&operation, &proposed).unwrap();
        let selected = app.review_select(operation.as_str(), &[action_id]).unwrap();
        assert_eq!(selected.request.inputs, proposed.inputs);
        assert_eq!(selected.request.writes.len(), 1);
        assert_eq!(
            selected.request.writes[0].id,
            selected.operation.creation_id(0)
        );
        outcome(
            app.owner_change(
                "Friday input",
                &request(vec![put(
                    &input,
                    RecordData::Note(Note::working("Evidence", "Friday")),
                )]),
            )
            .unwrap(),
        );
        let applied = app.review_apply(selected.operation.as_str()).unwrap();
        match expectation {
            InputExpectation::ChatContext => unreachable!("These cases exercise note inputs"),
            InputExpectation::Current => {
                assert_eq!(
                    applied,
                    ApplyOutcome::Stale {
                        records: vec![input.id]
                    }
                );
                assert!(app.store.receipt(&selected.operation).unwrap().is_none());
                assert!(
                    app.store
                        .record(&selected.operation.creation_id(0))
                        .unwrap()
                        .is_none()
                );
            }
            InputExpectation::Historical => {
                let receipt = outcome(applied);
                assert_eq!(receipt.writes.len(), 1);
                assert!(
                    matches!(&receipt.writes[0].after.data, RecordData::Action(action) if action.description == "Prepare delivery Monday")
                );
            }
        }
        assert!(app.store.record(&note_id).unwrap().is_none());
    }
}

#[test]
fn missing_storage_never_opens_an_empty_workspace_or_rewrites_evidence() {
    let (_root, app) = workspace();
    let directory = app.store.directory().to_owned();
    drop(app);
    let marker = std::fs::read(directory.join(DATA_MARKER)).unwrap();
    std::fs::remove_file(directory.join(DATABASE_FILENAME)).unwrap();
    std::fs::write(
        directory.join("threads.sqlite3-wal"),
        b"app recovery evidence",
    )
    .unwrap();
    let error = Workspace::open(&directory)
        .err()
        .expect("missing database must fail")
        .to_string();
    assert!(
        error.contains("missing") && error.contains("backup") && error.contains("fresh"),
        "{error}"
    );
    assert!(!directory.join(DATABASE_FILENAME).exists());
    assert_eq!(std::fs::read(directory.join(DATA_MARKER)).unwrap(), marker);
    assert_eq!(
        std::fs::read(directory.join("threads.sqlite3-wal")).unwrap(),
        b"app recovery evidence"
    );
}
#[test]
fn an_open_edit_session_is_a_typed_app_error_naming_note_and_session() {
    let (_dir, mut app) = workspace();
    let note = app.new_note("Berth survey", "Survey scheduled").unwrap();
    let open = app.store.begin_edit(&note.id, note.version).unwrap();
    app.store.update_buffer(&open.id, 1, "Depth 1.4 m").unwrap();
    // The desktop reaches this through `AppResult`; it must not match text.
    let error: AppError = app
        .store
        .begin_edit(&note.id, note.version)
        .unwrap_err()
        .into();
    assert!(
        matches!(&error, AppError::Core(Error::EditSessionOpen { note: id, session, base_version })
            if *id == note.id && *session == open.id && *base_version == note.version),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "this note already has an open unsaved edit"
    );
    let kept = app.store.edit_session(&open.id).unwrap();
    assert_eq!(
        (kept.generation, kept.markdown.as_str(), kept.closed),
        (1, "Depth 1.4 m", false)
    );
}
