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
fn request(writes: Vec<Put>) -> ChangeRequest {
    ChangeRequest {
        reason: "synthetic product witness".into(),
        writes,
        inputs: vec![],
    }
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
        locator: "external://review".into(),
        label: "review".into(),
        observed_at: "today".into(),
        external_version: None,
        outcome: "full".into(),
        gaps: vec![],
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
