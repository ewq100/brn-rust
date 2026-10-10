use brn_threads_core::*;
use tempfile::TempDir;
fn temp() -> TempDir {
    TempDir::new_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap()
}
fn store() -> (TempDir, Store) {
    let dir = temp();
    let store = Store::open(dir.path().join("data")).unwrap();
    (dir, store)
}
fn owner() -> HostAuthority {
    HostAuthority::owner("synthetic owner")
}
fn ai() -> HostAuthority {
    HostAuthority::maintenance("synthetic run")
}
fn request(writes: Vec<Put>) -> ChangeRequest {
    ChangeRequest {
        reason: "synthetic change".into(),
        writes,
        inputs: vec![],
    }
}
fn created(op: &OperationId, index: usize, data: RecordData) -> Put {
    Put {
        id: op.creation_id(index),
        expected_version: None,
        archived: false,
        data,
    }
}
fn edited(record: &Record, data: RecordData) -> Put {
    Put {
        id: record.id.clone(),
        expected_version: Some(record.version),
        archived: record.archived,
        data,
    }
}
fn apply(
    store: &mut Store,
    op: &OperationId,
    req: &ChangeRequest,
    authority: &HostAuthority,
) -> Receipt {
    store.prepare(op, req).unwrap();
    match store.apply(op, authority).unwrap() {
        ApplyOutcome::Applied(r) => r,
        outcome => panic!("{outcome:?}"),
    }
}
fn note(store: &mut Store, key: &str, protected: bool) -> Record {
    let op = store.allocate_operation(key).unwrap();
    let mut note = Note::working(key, "base writing");
    note.protected = protected;
    apply(
        store,
        &op,
        &request(vec![created(&op, 0, RecordData::Note(note))]),
        &owner(),
    )
    .writes[0]
        .after
        .clone()
}
fn text(record: &Record) -> String {
    match &record.data {
        RecordData::Note(n) => n.markdown.clone(),
        _ => panic!(),
    }
}
fn change_text(record: &Record, text: &str) -> RecordData {
    let RecordData::Note(mut note) = record.data.clone() else {
        panic!()
    };
    note.markdown = text.into();
    RecordData::Note(note)
}

#[test]
fn explicit_selection_refuses_old_and_foreign_identity() {
    let temp = temp();
    std::fs::write(temp.path().join("brn.sqlite"), b"old").unwrap();
    assert!(matches!(
        Store::open(temp.path()),
        Err(Error::UnsafeDirectory(_))
    ));
    assert!(!temp.path().join(DATABASE_FILENAME).exists());
    assert!(matches!(
        Store::open("relative"),
        Err(Error::UnsafeDirectory(_))
    ));
    let (dir, store) = store();
    let data = store.directory().to_owned();
    drop(store);
    let conn = rusqlite::Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    conn.pragma_update(None, "application_id", 123).unwrap();
    drop(conn);
    assert!(matches!(Store::open(data), Err(Error::UnsafeDirectory(_))));
    drop(dir);
}
#[test]
fn lost_prepare_and_apply_responses_replay_across_connections_and_restart() {
    let (dir, mut store) = store();
    let existing = note(&mut store, "existing", true);
    let data = store.directory().to_owned();
    let op = store.allocate_operation("host durable request 42").unwrap();
    let req = request(vec![
        created(
            &op,
            0,
            RecordData::Note(Note::working("new", "new knowledge")),
        ),
        created(
            &op,
            1,
            RecordData::Action(Action {
                description: "consider follow-up".into(),
                state: ActionState::Suggested,
                internal: false,
                evidence: None,
                due: None,
            }),
        ),
        edited(
            &existing,
            change_text(&existing, "revised protected knowledge"),
        ),
    ]);
    let prepared = store.prepare(&op, &req).unwrap();
    drop(store); // preparation response lost
    let mut restart = Store::open(&data).unwrap();
    assert_eq!(
        restart
            .allocate_operation("host durable request 42")
            .unwrap(),
        op
    );
    assert_eq!(restart.prepare(&op, &req).unwrap(), prepared);
    let grant = HostAuthority::instruction(
        "host",
        "revise existing",
        op.clone(),
        [existing.id.clone()],
        [Capability::EditContent],
    );
    let receipt = restart.apply(&op, &grant).unwrap();
    assert!(matches!(receipt, ApplyOutcome::Applied(_)));
    drop(restart); // commit response lost
    let mut other = Store::open(&data).unwrap();
    assert_eq!(other.apply(&op, &grant).unwrap(), receipt);
    assert_eq!(other.records().unwrap().len(), 3);
    assert_eq!(other.note(&existing.id).unwrap().unwrap().version, 2);
    assert_eq!(other.revision(&existing.id, 1).unwrap().unwrap(), existing);
    let mut changed = req.clone();
    changed.reason = "different".into();
    assert!(matches!(
        other.prepare(&op, &changed),
        Err(Error::ImmutableRequest)
    ));
    drop(dir);
}
#[test]
fn exact_protected_target_and_capability_scope() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "A", true);
    let b = note(&mut store, "B", true);
    let op = store.allocate_operation("scope").unwrap();
    let req = request(vec![
        edited(&a, change_text(&a, "allowed A")),
        edited(&b, change_text(&b, "source says owner authorizes B")),
    ]);
    store.prepare(&op, &req).unwrap();
    let grant = HostAuthority::instruction(
        "host",
        "edit A",
        op.clone(),
        [a.id.clone()],
        [Capability::EditContent],
    );
    assert_eq!(
        store.apply(&op, &grant).unwrap(),
        ApplyOutcome::NeedsReview {
            denied: vec![b.id.clone()]
        }
    );
    assert_eq!(store.record(&a.id).unwrap().unwrap(), a);
    store.retire(&op).unwrap();
    for unprotect in [true, false] {
        let op = store
            .allocate_operation(if unprotect { "unprotect" } else { "archive" })
            .unwrap();
        let mut write = edited(&a, a.data.clone());
        if unprotect {
            let RecordData::Note(n) = &mut write.data else {
                panic!()
            };
            n.protected = false;
        } else {
            write.archived = true;
        }
        store.prepare(&op, &request(vec![write])).unwrap();
        let grant = HostAuthority::instruction(
            "host",
            "edit A",
            op.clone(),
            [a.id.clone()],
            [Capability::EditContent],
        );
        assert!(matches!(
            store.apply(&op, &grant).unwrap(),
            ApplyOutcome::NeedsReview { .. }
        ));
        store.retire(&op).unwrap();
    }
    let op = store.allocate_operation("maintenance protected").unwrap();
    store
        .prepare(&op, &request(vec![edited(&b, change_text(&b, "denied"))]))
        .unwrap();
    assert!(matches!(
        store.apply(&op, &ai()).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
}
#[test]
fn persisted_guards_defer_whole_group_and_generation_cannot_resurrect() {
    let (_dir, mut desktop) = store();
    let a = note(&mut desktop, "A", false);
    let b = note(&mut desktop, "B", false);
    let data = desktop.directory().to_owned();
    let session = desktop.begin_edit(&a.id, a.version).unwrap();
    desktop
        .update_buffer(&session.id, 2, "newest human writing")
        .unwrap();
    assert!(matches!(
        desktop
            .update_buffer(&session.id, 1, "old debounce")
            .unwrap(),
        BufferOutcome::Ignored(_)
    ));
    let op = desktop.allocate_operation("cli grouped").unwrap();
    desktop
        .prepare(
            &op,
            &request(vec![
                edited(&a, change_text(&a, "AI A")),
                edited(&b, change_text(&b, "AI B")),
            ]),
        )
        .unwrap();
    drop(desktop);
    let mut cli = Store::open(&data).unwrap();
    assert_eq!(
        cli.apply(&op, &ai()).unwrap(),
        ApplyOutcome::Deferred {
            guarded: vec![a.id.clone()]
        }
    );
    assert_eq!(cli.record(&b.id).unwrap().unwrap(), b);
    assert_eq!(
        cli.recovery_buffers().unwrap()[0].markdown,
        "newest human writing"
    );
    assert!(matches!(
        cli.discard(&session.id, 1).unwrap(),
        BufferOutcome::Ignored(_)
    ));
    assert!(matches!(
        cli.save(&session.id, 1).unwrap(),
        SaveOutcome::GenerationMismatch(_)
    ));
    let saved = cli.save(&session.id, 2).unwrap();
    assert!(matches!(saved, SaveOutcome::Saved(_)));
    assert_eq!(cli.save(&session.id, 2).unwrap(), saved);
    assert!(matches!(
        cli.update_buffer(&session.id, 3, "late recovery").unwrap(),
        BufferOutcome::Ignored(_)
    ));
    assert!(cli.recovery_buffers().unwrap().is_empty());
    assert_eq!(
        cli.apply(&op, &ai()).unwrap(),
        ApplyOutcome::Stale {
            records: vec![a.id.clone()]
        }
    );
    assert_eq!(
        text(&cli.record(&a.id).unwrap().unwrap()),
        "newest human writing"
    );
    assert_eq!(cli.record(&b.id).unwrap().unwrap(), b);
    cli.retire(&op).unwrap();
    let current = cli.record(&a.id).unwrap().unwrap();
    let discard = cli.begin_edit(&a.id, current.version).unwrap();
    cli.update_buffer(&discard.id, 1, "discard me").unwrap();
    cli.discard(&discard.id, 1).unwrap();
    assert!(matches!(
        cli.update_buffer(&discard.id, 99, "late discarded")
            .unwrap(),
        BufferOutcome::Ignored(_)
    ));
    assert!(cli.recovery_buffers().unwrap().is_empty());
}
#[test]
fn stale_open_type_save_preserves_buffer() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "A", false); // clean editor remembers v1, no guard
    let op = store.allocate_operation("AI wins race").unwrap();
    apply(
        &mut store,
        &op,
        &request(vec![edited(&a, change_text(&a, "AI v2"))]),
        &ai(),
    );
    let session = store.begin_edit(&a.id, 1).unwrap();
    store
        .update_buffer(&session.id, 1, "human typed from v1")
        .unwrap();
    let SaveOutcome::Stale(buffer) = store.save(&session.id, 1).unwrap() else {
        panic!()
    };
    assert_eq!(buffer.markdown, "human typed from v1");
    assert!(!buffer.closed);
    assert_eq!(text(&store.record(&a.id).unwrap().unwrap()), "AI v2");
}
#[test]
fn undo_checks_actual_write_set_preserves_link_later_edits_and_lists_direct_dependents() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "A", false);
    let b = note(&mut store, "B", false);
    let op = store.allocate_operation("AI group").unwrap();
    let receipt = apply(
        &mut store,
        &op,
        &request(vec![
            edited(&a, change_text(&a, "AI A")),
            edited(&b, change_text(&b, "AI B")),
        ]),
        &ai(),
    );
    let linkop = store.allocate_operation("unrelated link").unwrap();
    let link = created(
        &linkop,
        0,
        RecordData::Link(Link {
            from: a.id.clone(),
            to: b.id.clone(),
            relation: "context".into(),
        }),
    );
    apply(&mut store, &linkop, &request(vec![link]), &ai());
    let derived = store.allocate_operation("derived summary").unwrap();
    let mut req = request(vec![created(
        &derived,
        0,
        RecordData::Note(Note::working("summary", "based on AI A")),
    )]);
    req.inputs.push(RevisionInput {
        record: a.id.clone(),
        version: 2,
    });
    apply(&mut store, &derived, &req, &ai());
    let undo = store.allocate_operation("undo group").unwrap();
    let ApplyOutcome::Applied(comp) = store.undo(&undo, &op, &ai()).unwrap() else {
        panic!()
    };
    assert_eq!(comp.needs_refresh, vec![derived.creation_id(0)]);
    assert_eq!(text(&store.record(&a.id).unwrap().unwrap()), text(&a));
    assert!(store.record(&linkop.creation_id(0)).unwrap().is_some());
    assert_eq!(
        store.undo(&undo, &op, &ai()).unwrap(),
        ApplyOutcome::Applied(comp)
    );
    assert_eq!(receipt.writes.len(), 2);
    let a = store.record(&a.id).unwrap().unwrap();
    let op = store.allocate_operation("another AI edit").unwrap();
    apply(
        &mut store,
        &op,
        &request(vec![edited(&a, change_text(&a, "AI again"))]),
        &ai(),
    );
    let current = store.record(&a.id).unwrap().unwrap();
    let session = store.begin_edit(&a.id, current.version).unwrap();
    store
        .update_buffer(&session.id, 1, "later human edit")
        .unwrap();
    store.save(&session.id, 1).unwrap();
    let undo = store.allocate_operation("conflicting undo").unwrap();
    assert_eq!(
        store.undo(&undo, &op, &owner()).unwrap(),
        ApplyOutcome::Stale {
            records: vec![a.id.clone()]
        }
    );
    assert_eq!(
        text(&store.record(&a.id).unwrap().unwrap()),
        "later human edit"
    );
    assert!(store.receipt(&undo).unwrap().is_none());
}
#[test]
fn complete_backup_restores_assets_sources_pending_candidates_history_and_buffers() {
    let (dir, mut store) = store();
    let a = note(&mut store, "A", false);
    let op = store.allocate_operation("complete state").unwrap();
    let source = created(
        &op,
        0,
        RecordData::Source(Source {
            locator: "external://synthetic".into(),
            label: "reference".into(),
            observed_at: "2026-10-10".into(),
            external_version: None,
            outcome: "full".into(),
            gaps: vec![],
        }),
    );
    let asset = created(
        &op,
        1,
        RecordData::Asset(Asset {
            note: a.id.clone(),
            source: Some(source.id.clone()),
            media_type: "image/png".into(),
            bytes: vec![0, 1, 2, 255],
        }),
    );
    let thread = created(
        &op,
        2,
        RecordData::Thread(Thread {
            title: "Synthetic thread".into(),
            state: ThreadState::Open,
            attention: vec![Attention {
                kind: AttentionKind::Question,
                reason: "owner question".into(),
                record: Some(a.id.clone()),
            }],
        }),
    );
    apply(
        &mut store,
        &op,
        &request(vec![source, asset, thread, edited(&a, a.data.clone())]),
        &ai(),
    );
    let pending = store.allocate_operation("pending change").unwrap();
    let prepared = store
        .prepare(
            &pending,
            &request(vec![edited(&a, change_text(&a, "pending"))]),
        )
        .unwrap();
    let session = store.begin_edit(&a.id, 1).unwrap();
    store.update_buffer(&session.id, 1, "recovery").unwrap();
    let backup = dir.path().join("backup.sqlite3");
    store.backup(&backup).unwrap();
    let restored = Store::restore(&backup, dir.path().join("restored")).unwrap();
    assert_eq!(restored.records().unwrap(), store.records().unwrap());
    assert_eq!(restored.prepared(&pending).unwrap(), prepared);
    assert_eq!(restored.receipt(&op).unwrap(), store.receipt(&op).unwrap());
    assert_eq!(
        restored.recovery_buffers().unwrap(),
        store.recovery_buffers().unwrap()
    );
    assert_eq!(restored.revision(&a.id, 1).unwrap(), Some(a));
    assert!(Store::restore(&backup, restored.directory()).is_err());
}
#[test]
fn pending_candidate_requires_retirement_and_confirmation_is_revision_bound() {
    let (_dir, mut store) = store();
    let op = store.allocate_operation("confirmed note").unwrap();
    let mut n = Note::working("confirmed", "one");
    n.confirmed = true;
    let receipt = apply(
        &mut store,
        &op,
        &request(vec![created(&op, 0, RecordData::Note(n))]),
        &owner(),
    );
    let a = &receipt.writes[0].after;
    let first = store.allocate_operation("candidate first").unwrap();
    store
        .prepare(&first, &request(vec![edited(a, change_text(a, "two"))]))
        .unwrap();
    let second = store.allocate_operation("candidate second").unwrap();
    let req = request(vec![edited(a, change_text(a, "three"))]);
    assert!(store.prepare(&second, &req).is_err());
    store.retire(&first).unwrap();
    assert!(store.apply(&first, &owner()).is_err());
    apply(&mut store, &second, &req, &ai());
    let RecordData::Note(current) = store.record(&a.id).unwrap().unwrap().data else {
        panic!()
    };
    assert!(!current.confirmed);
}
#[test]
fn wrong_reference_rolls_back_group_and_occupied_backup_remains_intact() {
    let (dir, mut store) = store();
    let a = note(&mut store, "A", false);
    let op = store.allocate_operation("invalid second member").unwrap();
    let req = request(vec![
        edited(&a, change_text(&a, "must roll back")),
        created(
            &op,
            0,
            RecordData::Message(Message {
                thread: a.id.clone(),
                role: "assistant".into(),
                text: "not a thread".into(),
            }),
        ),
    ]);
    store.prepare(&op, &req).unwrap();
    assert!(store.apply(&op, &ai()).is_err());
    assert_eq!(store.record(&a.id).unwrap().unwrap(), a);
    assert!(store.record(&op.creation_id(0)).unwrap().is_none());
    assert!(store.receipt(&op).unwrap().is_none());
    let backup = dir.path().join("occupied.sqlite3");
    std::fs::write(&backup, "occupied").unwrap();
    assert!(store.backup(&backup).is_err());
    assert_eq!(std::fs::read_to_string(backup).unwrap(), "occupied");
}
#[test]
fn two_live_connections_order_dirty_guard_before_grouped_apply() {
    let (_dir, mut desktop) = store();
    let a = note(&mut desktop, "A", false);
    let mut cli = Store::open(desktop.directory()).unwrap();
    let op = cli.allocate_operation("concurrent group").unwrap();
    cli.prepare(&op, &request(vec![edited(&a, change_text(&a, "AI"))]))
        .unwrap();
    let session = desktop.begin_edit(&a.id, a.version).unwrap();
    assert!(matches!(
        cli.apply(&op, &ai()).unwrap(),
        ApplyOutcome::Deferred { .. }
    ));
    desktop.discard(&session.id, 0).unwrap();
    assert!(matches!(
        cli.apply(&op, &ai()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(text(&desktop.record(&a.id).unwrap().unwrap()), "AI");
}
#[test]
fn host_run_fence_blocks_superseded_commit_but_receipt_replay_survives_cancellation() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "A", false);
    let setup = store.allocate_operation("run setup").unwrap();
    let thread = created(
        &setup,
        0,
        RecordData::Thread(Thread {
            title: "run thread".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    let run = created(
        &setup,
        1,
        RecordData::Run(Run {
            thread: thread.id.clone(),
            provider: "synthetic".into(),
            model: "synthetic".into(),
            guide_identity: "v1".into(),
            budget: 3,
            effort: "medium".into(),
            loaded_skills: vec![],
            fence: 1,
            state: RunState::Working,
            progress: "starting".into(),
            operations: vec![],
            sources: vec![],
        }),
    );
    apply(&mut store, &setup, &request(vec![thread, run]), &owner());
    let op = store.allocate_operation("fenced operation").unwrap();
    store
        .prepare(&op, &request(vec![edited(&a, change_text(&a, "from run"))]))
        .unwrap();
    let bound = ai().for_run(setup.creation_id(1), 1);
    let receipt = store.apply(&op, &bound).unwrap();
    assert!(matches!(receipt, ApplyOutcome::Applied(_)));
    let current = store.record(&a.id).unwrap().unwrap();
    let pending = store.allocate_operation("superseded operation").unwrap();
    store
        .prepare(
            &pending,
            &request(vec![edited(&current, change_text(&current, "late output"))]),
        )
        .unwrap();
    let run = store.record(&setup.creation_id(1)).unwrap().unwrap();
    let RecordData::Run(mut next) = run.data.clone() else {
        panic!()
    };
    next.fence = 2;
    next.state = RunState::Cancelled;
    let cancel = store.allocate_operation("host cancel").unwrap();
    apply(
        &mut store,
        &cancel,
        &request(vec![edited(&run, RecordData::Run(next))]),
        &owner(),
    );
    assert_eq!(
        store.apply(&pending, &bound).unwrap(),
        ApplyOutcome::Superseded { run: run.id }
    );
    assert_eq!(store.apply(&op, &bound).unwrap(), receipt);
    assert_eq!(store.record(&a.id).unwrap().unwrap(), current);
    let wrong = HostAuthority::instruction(
        "host",
        "different instruction",
        pending,
        [a.id],
        [Capability::EditContent],
    );
    assert!(matches!(
        store.apply(&op, &wrong).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
}
#[test]
fn direct_owner_metadata_protection_and_lifecycle_changes_make_candidates_stale() {
    for kind in 0..3 {
        let (_dir, mut store) = store();
        let a = note(&mut store, "A", false);
        let proposed = store.allocate_operation("pending AI").unwrap();
        store
            .prepare(
                &proposed,
                &request(vec![edited(&a, change_text(&a, "pending text"))]),
            )
            .unwrap();
        let direct = store.allocate_operation("owner direct").unwrap();
        let mut write = edited(&a, a.data.clone());
        let RecordData::Note(n) = &mut write.data else {
            panic!()
        };
        match kind {
            0 => n.title = "new owner title".into(),
            1 => n.protected = true,
            _ => write.archived = true,
        };
        let req = request(vec![write]);
        assert!(store.prepare_owner(&direct, &req, &ai()).is_err());
        store.prepare_owner(&direct, &req, &owner()).unwrap();
        let updated = store.apply(&direct, &owner()).unwrap();
        assert!(matches!(updated, ApplyOutcome::Applied(_)));
        // Protection also changes authorization; owner replay verifies the base
        // conflict separately from whether a delegated caller needs review.
        assert_eq!(
            store.apply(&proposed, &owner()).unwrap(),
            ApplyOutcome::Stale {
                records: vec![a.id.clone()]
            }
        );
        assert_eq!(store.record(&a.id).unwrap().unwrap().version, 2);
    }
}
#[cfg(unix)]
#[test]
fn data_selection_refuses_symlink_ancestor_without_creating_database() {
    let dir = temp();
    let actual = dir.path().join("actual");
    std::fs::create_dir(&actual).unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&actual, &link).unwrap();
    assert!(matches!(
        Store::open(link.join("data")),
        Err(Error::UnsafeDirectory(_))
    ));
    assert!(!actual.join("data").exists());
}
#[test]
fn full_import_is_protected_and_confirmation_requires_explicit_authority() {
    let (_dir, mut store) = store();
    let op = store.allocate_operation("full import").unwrap();
    let source = created(
        &op,
        0,
        RecordData::Source(Source {
            locator: "external://full".into(),
            label: "full".into(),
            observed_at: "today".into(),
            external_version: None,
            outcome: "Partial".into(),
            gaps: vec!["unsupported figure".into()],
        }),
    );
    let mut full = Note::working("full imported process", "Complete substantive process text");
    full.import = Some(ImportCoverage {
        source: source.id.clone(),
        full_note: true,
        gaps: vec!["unsupported figure".into()],
    });
    assert!(
        store
            .prepare(
                &op,
                &request(vec![
                    source.clone(),
                    created(&op, 1, RecordData::Note(full.clone()))
                ])
            )
            .is_err()
    );
    full.protected = true;
    apply(
        &mut store,
        &op,
        &request(vec![source, created(&op, 1, RecordData::Note(full))]),
        &ai(),
    );
    let imported = store.record(&op.creation_id(1)).unwrap().unwrap();
    let RecordData::Note(mut content) = imported.data.clone() else {
        panic!()
    };
    assert!(!content.confirmed);
    content.confirmed = true;
    let confirm = store.allocate_operation("unauthorized confirm").unwrap();
    store
        .prepare(
            &confirm,
            &request(vec![edited(&imported, RecordData::Note(content))]),
        )
        .unwrap();
    assert!(matches!(
        store.apply(&confirm, &ai()).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    let grant = HostAuthority::instruction(
        "host",
        "confirm imported policy",
        confirm.clone(),
        [imported.id.clone()],
        [Capability::Confirm],
    );
    assert!(matches!(
        store.apply(&confirm, &grant).unwrap(),
        ApplyOutcome::Applied(_)
    ));
}
#[test]
fn human_action_completion_requires_evidence_and_scope_and_thread_resolve_cannot_complete_action() {
    let (_dir, mut store) = store();
    let op = store.allocate_operation("human obligation").unwrap();
    let action = Action {
        description: "human sends report".into(),
        state: ActionState::Open,
        internal: false,
        evidence: None,
        due: Some("Friday".into()),
    };
    let req = request(vec![created(&op, 0, RecordData::Action(action))]);
    store.prepare(&op, &req).unwrap();
    assert!(matches!(
        store.apply(&op, &ai()).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    let grant = HostAuthority::instruction(
        "host",
        "send report Friday",
        op.clone(),
        [op.creation_id(0)],
        [Capability::Commitment],
    );
    store.apply(&op, &grant).unwrap();
    let a = store.record(&op.creation_id(0)).unwrap().unwrap();
    let RecordData::Action(mut done) = a.data.clone() else {
        panic!()
    };
    done.state = ActionState::Done;
    let complete = store.allocate_operation("completion").unwrap();
    assert!(
        store
            .prepare(
                &complete,
                &request(vec![edited(&a, RecordData::Action(done.clone()))])
            )
            .is_err()
    );
    done.evidence = Some("draft only".into());
    store
        .prepare(
            &complete,
            &request(vec![edited(&a, RecordData::Action(done))]),
        )
        .unwrap();
    assert!(matches!(
        store.apply(&complete, &ai()).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert_eq!(store.record(&a.id).unwrap().unwrap(), a);
    let t = store.allocate_operation("resolved discussion").unwrap();
    apply(
        &mut store,
        &t,
        &request(vec![created(
            &t,
            0,
            RecordData::Thread(Thread {
                title: "report discussion".into(),
                state: ThreadState::Resolved,
                attention: vec![],
            }),
        )]),
        &ai(),
    );
    assert_eq!(store.record(&a.id).unwrap().unwrap(), a);
}
#[test]
fn protected_supersede_requires_distinct_capability() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "A", true);
    let b = note(&mut store, "B", false);
    let op = store.allocate_operation("supersede protected").unwrap();
    let RecordData::Note(mut next) = a.data.clone() else {
        panic!()
    };
    next.superseded_by = Some(b.id);
    store
        .prepare(&op, &request(vec![edited(&a, RecordData::Note(next))]))
        .unwrap();
    let edit = HostAuthority::instruction(
        "host",
        "edit A",
        op.clone(),
        [a.id.clone()],
        [Capability::EditContent],
    );
    assert!(matches!(
        store.apply(&op, &edit).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert_eq!(store.record(&a.id).unwrap().unwrap(), a);
}
#[test]
fn comment_utf8_anchors_and_separate_record_versions_are_checked() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "A", false);
    let update = store.allocate_operation("Estonian text").unwrap();
    apply(
        &mut store,
        &update,
        &request(vec![edited(&a, change_text(&a, "Üks õun, teine õun"))]),
        &ai(),
    );
    let current = store.record(&a.id).unwrap().unwrap();
    let op = store.allocate_operation("anchored comment").unwrap();
    let comment = Comment {
        note: a.id.clone(),
        base_version: 2,
        quote: "õun".into(),
        range: Some((5, 9)),
        mapped_version: None,
        mapped_range: None,
        body: "clarify first occurrence".into(),
        unresolved: false,
    };
    apply(
        &mut store,
        &op,
        &request(vec![created(&op, 0, RecordData::Comment(comment.clone()))]),
        &ai(),
    );
    assert_eq!(store.record(&a.id).unwrap().unwrap(), current);
    let mut invalid = comment;
    invalid.range = Some((6, 9));
    let invalid_op = store.allocate_operation("invalid UTF-8 boundary").unwrap();
    store
        .prepare(
            &invalid_op,
            &request(vec![created(&invalid_op, 0, RecordData::Comment(invalid))]),
        )
        .unwrap();
    assert!(store.apply(&invalid_op, &ai()).is_err());
    assert!(store.record(&invalid_op.creation_id(0)).unwrap().is_none());
}
#[test]
fn exact_source_identity_is_unique_and_failed_duplicate_has_no_partial_effect() {
    let (_dir, mut store) = store();
    let source = Source {
        locator: "external://stable-id".into(),
        label: "source".into(),
        observed_at: "today".into(),
        external_version: None,
        outcome: "observed".into(),
        gaps: vec![],
    };
    let first = store.allocate_operation("source first").unwrap();
    apply(
        &mut store,
        &first,
        &request(vec![created(&first, 0, RecordData::Source(source.clone()))]),
        &ai(),
    );
    let op = store.allocate_operation("duplicate source").unwrap();
    store
        .prepare(
            &op,
            &request(vec![
                created(
                    &op,
                    0,
                    RecordData::Note(Note::working("duplicate imported note", "must roll back")),
                ),
                created(&op, 1, RecordData::Source(source)),
            ]),
        )
        .unwrap();
    assert!(store.apply(&op, &ai()).is_err());
    assert!(store.record(&op.creation_id(0)).unwrap().is_none());
    assert!(store.receipt(&op).unwrap().is_none());
}
#[test]
fn assets_require_owner_group_and_inherit_protection_and_dirty_guard() {
    let (_dir, mut store) = store();
    let seed = store.allocate_operation("protected asset seed").unwrap();
    let mut n = Note::working("protected imported figure", "figure");
    n.protected = true;
    let req = request(vec![
        created(&seed, 0, RecordData::Note(n)),
        created(
            &seed,
            1,
            RecordData::Asset(Asset {
                note: seed.creation_id(0),
                source: None,
                media_type: "image/png".into(),
                bytes: vec![1, 2, 3],
            }),
        ),
    ]);
    apply(&mut store, &seed, &req, &owner());
    let note = store.record(&seed.creation_id(0)).unwrap().unwrap();
    let asset = store.record(&seed.creation_id(1)).unwrap().unwrap();
    let RecordData::Asset(mut changed) = asset.data.clone() else {
        panic!()
    };
    changed.bytes = vec![9, 9, 9];
    let missing = store
        .allocate_operation("asset without owning note")
        .unwrap();
    store
        .prepare(
            &missing,
            &request(vec![edited(&asset, RecordData::Asset(changed.clone()))]),
        )
        .unwrap();
    assert!(store.apply(&missing, &owner()).is_err());
    assert_eq!(store.record(&asset.id).unwrap().unwrap(), asset);
    let grouped = store
        .allocate_operation("grouped protected asset change")
        .unwrap();
    let req = request(vec![
        edited(&note, note.data.clone()),
        edited(&asset, RecordData::Asset(changed)),
    ]);
    store.prepare(&grouped, &req).unwrap();
    assert_eq!(
        store.apply(&grouped, &ai()).unwrap(),
        ApplyOutcome::NeedsReview {
            denied: vec![note.id.clone()]
        }
    );
    let session = store.begin_edit(&note.id, note.version).unwrap();
    store
        .update_buffer(&session.id, 1, "human writing")
        .unwrap();
    let grant = HostAuthority::instruction(
        "host",
        "replace figure",
        grouped.clone(),
        [note.id.clone()],
        [Capability::EditContent],
    );
    assert_eq!(
        store.apply(&grouped, &grant).unwrap(),
        ApplyOutcome::Deferred {
            guarded: vec![note.id.clone()]
        }
    );
    assert_eq!(store.record(&asset.id).unwrap().unwrap(), asset);
    store.discard(&session.id, 1).unwrap();
    let ApplyOutcome::Applied(receipt) = store.apply(&grouped, &grant).unwrap() else {
        panic!()
    };
    assert_eq!(receipt.writes.len(), 2);
    assert_eq!(store.record(&note.id).unwrap().unwrap().version, 2);
    let undo = store.allocate_operation("undo grouped asset").unwrap();
    assert!(matches!(
        store.undo(&undo, &grouped, &owner()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(store.record(&asset.id).unwrap().unwrap().data, asset.data);
    let other = self::note(&mut store, "other owner", false);
    let current_asset = store.record(&asset.id).unwrap().unwrap();
    let RecordData::Asset(mut reassigned) = current_asset.data.clone() else {
        panic!()
    };
    reassigned.note = other.id.clone();
    let op = store.allocate_operation("asset reassignment").unwrap();
    store
        .prepare(
            &op,
            &request(vec![
                edited(&other, other.data.clone()),
                edited(&current_asset, RecordData::Asset(reassigned)),
            ]),
        )
        .unwrap();
    assert!(store.apply(&op, &owner()).is_err());
}
fn seed_run(store: &mut Store, key: &str) -> Record {
    let op = store.allocate_operation(key).unwrap();
    let thread = created(
        &op,
        0,
        RecordData::Thread(Thread {
            title: key.into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    let run = created(
        &op,
        1,
        RecordData::Run(Run {
            thread: thread.id.clone(),
            provider: "provider".into(),
            model: "model".into(),
            guide_identity: "guide".into(),
            budget: 3,
            effort: "medium".into(),
            loaded_skills: vec![],
            fence: 1,
            state: RunState::Working,
            progress: "starting".into(),
            operations: vec![],
            sources: vec![],
        }),
    );
    apply(store, &op, &request(vec![thread, run]), &owner());
    store.record(&op.creation_id(1)).unwrap().unwrap()
}
#[test]
fn run_management_is_host_only_monotonic_and_cancellation_cannot_be_undone() {
    let (_dir, mut store) = store();
    let run = seed_run(&mut store, "host run");
    let RecordData::Run(mut next) = run.data.clone() else {
        panic!()
    };
    next.state = RunState::Cancelled;
    next.fence = 2;
    let cancel = store.allocate_operation("host cancellation").unwrap();
    let req = request(vec![edited(&run, RecordData::Run(next.clone()))]);
    store.prepare(&cancel, &req).unwrap();
    assert!(matches!(
        store.apply(&cancel, &ai()).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    let instruction = HostAuthority::instruction(
        "host",
        "model attempts control",
        cancel.clone(),
        [run.id.clone()],
        [Capability::ManageRun],
    );
    assert!(matches!(
        store.apply(&cancel, &instruction).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    let wrong = HostAuthority::runtime("host", "different-run");
    assert!(matches!(
        store.apply(&cancel, &wrong).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    let host = HostAuthority::runtime("coordinator", run.id.clone());
    assert!(matches!(
        store.apply(&cancel, &host).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let cancelled = store.record(&run.id).unwrap().unwrap();
    let undo = store.allocate_operation("undo host cancellation").unwrap();
    assert!(store.undo(&undo, &cancel, &owner()).is_err());
    assert_eq!(store.record(&run.id).unwrap().unwrap(), cancelled);
    assert!(store.receipt(&undo).unwrap().is_none());
    for (key, fence, state) in [
        ("fence reset", 1, RunState::Cancelled),
        ("reactivate old run", 3, RunState::Working),
    ] {
        next.fence = fence;
        next.state = state;
        let op = store.allocate_operation(key).unwrap();
        store
            .prepare(
                &op,
                &request(vec![edited(&cancelled, RecordData::Run(next.clone()))]),
            )
            .unwrap();
        assert!(store.apply(&op, &host).is_err());
        assert_eq!(store.record(&run.id).unwrap().unwrap(), cancelled);
    }
    let late = store
        .allocate_operation("late cancelled invocation")
        .unwrap();
    store
        .prepare(
            &late,
            &request(vec![created(
                &late,
                0,
                RecordData::Note(Note::working("late", "must not commit")),
            )]),
        )
        .unwrap();
    assert_eq!(
        store
            .apply(&late, &ai().for_run(run.id.clone(), 1))
            .unwrap(),
        ApplyOutcome::Superseded {
            run: run.id.clone()
        }
    );
    assert!(store.record(&late.creation_id(0)).unwrap().is_none());
    let create = store.allocate_operation("model creates run").unwrap();
    let mut fresh = next;
    fresh.thread = match &cancelled.data {
        RecordData::Run(run) => run.thread.clone(),
        _ => panic!(),
    };
    store
        .prepare(
            &create,
            &request(vec![created(&create, 0, RecordData::Run(fresh))]),
        )
        .unwrap();
    assert!(matches!(
        store.apply(&create, &ai()).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert!(store.record(&create.creation_id(0)).unwrap().is_none());
}
#[test]
fn undo_identity_is_bound_before_receipt_replay() {
    let (_dir, mut store) = store();
    let first = store.allocate_operation("first original").unwrap();
    apply(
        &mut store,
        &first,
        &request(vec![created(
            &first,
            0,
            RecordData::Note(Note::working("first", "first")),
        )]),
        &ai(),
    );
    let second = store.allocate_operation("second original").unwrap();
    apply(
        &mut store,
        &second,
        &request(vec![created(
            &second,
            0,
            RecordData::Note(Note::working("second", "second")),
        )]),
        &ai(),
    );
    let undo = store.allocate_operation("compensation identity").unwrap();
    let result = store.undo(&undo, &first, &owner()).unwrap();
    assert_eq!(store.undo(&undo, &first, &owner()).unwrap(), result);
    assert!(matches!(
        store.undo(&undo, &second, &owner()),
        Err(Error::ImmutableRequest)
    ));
    assert!(
        !store
            .record(&second.creation_id(0))
            .unwrap()
            .unwrap()
            .archived
    );
    let wrong = HostAuthority::instruction(
        "host",
        "wrong operation",
        second,
        [first.creation_id(0)],
        [Capability::Undo],
    );
    assert!(matches!(
        store.undo(&undo, &first, &wrong).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
}
#[test]
fn ordinary_content_revision_can_explicitly_clear_confirmation_without_confirmation_grant() {
    let (_dir, mut store) = store();
    let op = store.allocate_operation("confirmed ordinary").unwrap();
    let mut n = Note::working("confirmed", "old");
    n.confirmed = true;
    let a = apply(
        &mut store,
        &op,
        &request(vec![created(&op, 0, RecordData::Note(n))]),
        &owner(),
    )
    .writes[0]
        .after
        .clone();
    let RecordData::Note(mut n) = a.data.clone() else {
        panic!()
    };
    n.markdown = "new".into();
    n.confirmed = false;
    let update = store
        .allocate_operation("clear obsolete confirmation with revision")
        .unwrap();
    apply(
        &mut store,
        &update,
        &request(vec![edited(&a, RecordData::Note(n))]),
        &ai(),
    );
    let after = store.record(&a.id).unwrap().unwrap();
    let RecordData::Note(mut n) = after.data.clone() else {
        panic!()
    };
    assert!(!n.confirmed);
    n.confirmed = true;
    let confirm = store.allocate_operation("explicit true denied").unwrap();
    store
        .prepare(
            &confirm,
            &request(vec![edited(&after, RecordData::Note(n))]),
        )
        .unwrap();
    assert!(matches!(
        store.apply(&confirm, &ai()).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
}
#[test]
fn current_candidate_and_receipt_readers_exclude_retired_and_applied_requests() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "seed", false);
    let pending = store.allocate_operation("pending").unwrap();
    store
        .prepare(
            &pending,
            &request(vec![edited(&a, change_text(&a, "pending"))]),
        )
        .unwrap();
    assert_eq!(
        store
            .candidates()
            .unwrap()
            .iter()
            .map(|p| p.operation.clone())
            .collect::<Vec<_>>(),
        vec![pending.clone()]
    );
    assert_eq!(store.receipts().unwrap().len(), 1);
    store.retire(&pending).unwrap();
    assert!(store.candidates().unwrap().is_empty());
}

#[test]
fn workspace_settings_are_owner_only_and_persist_maintenance_review_policy() {
    let (_dir, mut store) = store();
    let settings = WorkspaceSettings {
        provider: "owner provider".into(),
        model: "owner model".into(),
        effort: "medium".into(),
        auth_file: None,
        credentials_dir: None,
        maintenance: false,
        review_first: false,
    };
    let setup = store.allocate_operation("workspace configuration").unwrap();
    let req = request(vec![created(
        &setup,
        0,
        RecordData::Settings(settings.clone()),
    )]);
    store.prepare(&setup, &req).unwrap();
    assert!(matches!(
        store.apply(&setup, &ai()).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    let grant = HostAuthority::instruction(
        "host",
        "untrusted configure",
        setup.clone(),
        [setup.creation_id(0)],
        [Capability::Configure],
    );
    assert!(matches!(
        store.apply(&setup, &grant).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert!(matches!(
        store
            .apply(
                &setup,
                &HostAuthority::runtime("host", setup.creation_id(0))
            )
            .unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    store.apply(&setup, &owner()).unwrap();
    let a = note(&mut store, "owner writing", false);
    let op = store
        .allocate_operation("paused maintenance write")
        .unwrap();
    store
        .prepare(
            &op,
            &request(vec![edited(&a, change_text(&a, "AI while paused"))]),
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &ai()).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert_eq!(store.record(&a.id).unwrap().unwrap(), a);
    let bound = HostAuthority::instruction(
        "host",
        "explicitly revise this note",
        op.clone(),
        [a.id.clone()],
        [Capability::EditContent],
    );
    assert!(matches!(
        store.apply(&op, &bound).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let current_settings = store.record(&setup.creation_id(0)).unwrap().unwrap();
    let mut review = settings.clone();
    review.maintenance = true;
    review.review_first = true;
    let change = store.allocate_operation("review before writes").unwrap();
    apply(
        &mut store,
        &change,
        &request(vec![edited(
            &current_settings,
            RecordData::Settings(review),
        )]),
        &owner(),
    );
    let new = store
        .allocate_operation("new note under review-first")
        .unwrap();
    store
        .prepare(
            &new,
            &request(vec![created(
                &new,
                0,
                RecordData::Note(Note::working("new", "review first")),
            )]),
        )
        .unwrap();
    assert!(matches!(
        store.apply(&new, &ai()).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert!(store.record(&new.creation_id(0)).unwrap().is_none());
    let current_settings = store.record(&setup.creation_id(0)).unwrap().unwrap();
    let mut enabled = settings;
    enabled.maintenance = true;
    let change = store
        .allocate_operation("resume ordinary maintenance")
        .unwrap();
    apply(
        &mut store,
        &change,
        &request(vec![edited(
            &current_settings,
            RecordData::Settings(enabled),
        )]),
        &owner(),
    );
    assert!(matches!(
        store.apply(&new, &ai()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
}
#[test]
fn paused_and_review_first_grants_require_exact_target_and_action() {
    for review_first in [false, true] {
        let (_dir, mut store) = store();
        let setup = store
            .allocate_operation("settings disable standing writes")
            .unwrap();
        let settings = WorkspaceSettings {
            provider: "provider".into(),
            model: "model".into(),
            effort: "medium".into(),
            auth_file: None,
            credentials_dir: None,
            maintenance: review_first,
            review_first,
        };
        apply(
            &mut store,
            &setup,
            &request(vec![created(&setup, 0, RecordData::Settings(settings))]),
            &owner(),
        );
        let a = note(&mut store, "A", false);
        let b = note(&mut store, "B", false);
        let unrelated = store
            .allocate_operation("instruction A attempts B")
            .unwrap();
        store
            .prepare(
                &unrelated,
                &request(vec![
                    edited(&a, change_text(&a, "valid A")),
                    edited(&b, change_text(&b, "unrelated B")),
                ]),
            )
            .unwrap();
        let grant = HostAuthority::instruction(
            "host",
            "edit A only",
            unrelated.clone(),
            [a.id.clone()],
            [Capability::EditContent],
        );
        assert_eq!(
            store.apply(&unrelated, &grant).unwrap(),
            ApplyOutcome::NeedsReview {
                denied: vec![b.id.clone()]
            }
        );
        assert_eq!(store.record(&a.id).unwrap().unwrap(), a);
        assert_eq!(store.record(&b.id).unwrap().unwrap(), b);
        store.retire(&unrelated).unwrap();
        for kind in 0..4 {
            let op = store
                .allocate_operation(&format!("wrong instruction action {kind}"))
                .unwrap();
            let mut write = edited(&a, a.data.clone());
            let cap = match kind {
                0 => {
                    write.data = change_text(&a, "content under archive grant");
                    Capability::Archive
                }
                1 => {
                    let RecordData::Note(n) = &mut write.data else {
                        panic!()
                    };
                    n.title = "title under archive grant".into();
                    Capability::Archive
                }
                2 => {
                    write.archived = true;
                    Capability::EditContent
                }
                _ => {
                    let RecordData::Note(n) = &mut write.data else {
                        panic!()
                    };
                    n.superseded_by = Some(b.id.clone());
                    Capability::EditContent
                }
            };
            store.prepare(&op, &request(vec![write])).unwrap();
            let grant = HostAuthority::instruction(
                "host",
                "specific action only",
                op.clone(),
                [a.id.clone()],
                [cap],
            );
            assert_eq!(
                store.apply(&op, &grant).unwrap(),
                ApplyOutcome::NeedsReview {
                    denied: vec![a.id.clone()]
                }
            );
            assert_eq!(store.record(&a.id).unwrap().unwrap(), a);
            store.retire(&op).unwrap();
        }
        let exact = store.allocate_operation("exact A EditContent").unwrap();
        store
            .prepare(
                &exact,
                &request(vec![edited(
                    &a,
                    change_text(&a, "specifically authorized A"),
                )]),
            )
            .unwrap();
        let grant = HostAuthority::instruction(
            "host",
            "edit A",
            exact.clone(),
            [a.id.clone()],
            [Capability::EditContent],
        );
        assert!(matches!(
            store.apply(&exact, &grant).unwrap(),
            ApplyOutcome::Applied(_)
        ));
        assert_eq!(
            text(&store.record(&a.id).unwrap().unwrap()),
            "specifically authorized A"
        );
        assert_eq!(store.record(&b.id).unwrap().unwrap(), b);
    }
}
#[test]
fn enabled_maintenance_keeps_standing_ordinary_fallback_for_instruction_runs() {
    let (_dir, mut store) = store();
    let setup = store
        .allocate_operation("settings standing enabled")
        .unwrap();
    apply(
        &mut store,
        &setup,
        &request(vec![created(
            &setup,
            0,
            RecordData::Settings(WorkspaceSettings {
                provider: "provider".into(),
                model: "model".into(),
                effort: "medium".into(),
                auth_file: None,
                credentials_dir: None,
                maintenance: true,
                review_first: false,
            }),
        )]),
        &owner(),
    );
    let a = note(&mut store, "A", false);
    let b = note(&mut store, "B", false);
    let op = store
        .allocate_operation("ordinary B under standing scope")
        .unwrap();
    let grant = HostAuthority::instruction(
        "host",
        "specific A context",
        op.clone(),
        [a.id],
        [Capability::EditContent],
    );
    apply(
        &mut store,
        &op,
        &request(vec![edited(
            &b,
            change_text(&b, "standing ordinary maintenance"),
        )]),
        &grant,
    );
    assert_eq!(
        text(&store.record(&b.id).unwrap().unwrap()),
        "standing ordinary maintenance"
    );
}
#[test]
fn host_input_binding_is_durable_before_preparation_and_changed_retry_is_rejected() {
    let (_dir, mut store) = store();
    let data = store.directory().to_owned();
    let first = store
        .allocate_bound_operation("bound intake host request", &"a".repeat(64))
        .unwrap();
    drop(store);
    let mut store = Store::open(data).unwrap();
    assert_eq!(
        store
            .allocate_bound_operation("bound intake host request", &"a".repeat(64))
            .unwrap(),
        first
    );
    assert!(matches!(
        store.allocate_bound_operation("bound intake host request", &"b".repeat(64)),
        Err(Error::ImmutableRequest)
    ));
    assert!(store.prepared(&first).is_err());
}
#[test]
fn failed_candidate_replacement_keeps_original_active_and_guarded() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "A", false);
    let original = store.allocate_operation("original candidate").unwrap();
    let req = request(vec![edited(&a, change_text(&a, "draft"))]);
    store.prepare(&original, &req).unwrap();
    let replacement = store.allocate_operation("invalid replacement").unwrap();
    let invalid = request(vec![created(
        &original,
        0,
        RecordData::Note(Note::working("wrong old creation identity", "invalid")),
    )]);
    assert!(
        store
            .replace_candidate(&original, &replacement, &invalid, &owner())
            .is_err()
    );
    assert_eq!(store.candidates().unwrap()[0].operation, original);
    assert!(matches!(
        store.apply(&original, &ai()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
}
#[test]
fn only_one_working_run_per_thread_and_new_invocation_can_interrupt_prior_atomically() {
    let (_dir, mut store) = store();
    let prior = seed_run(&mut store, "one working invocation");
    let RecordData::Run(mut next) = prior.data.clone() else {
        panic!()
    };
    let new = store.allocate_operation("second invocation").unwrap();
    let req = request(vec![created(&new, 0, RecordData::Run(next.clone()))]);
    store.prepare(&new, &req).unwrap();
    assert!(store.apply(&new, &owner()).is_err());
    assert!(store.record(&new.creation_id(0)).unwrap().is_none());
    let group = store
        .allocate_operation("interrupt and start fresh")
        .unwrap();
    next.state = RunState::Interrupted;
    next.fence = 2;
    let RecordData::Run(fresh) = prior.data.clone() else {
        panic!()
    };
    apply(
        &mut store,
        &group,
        &request(vec![
            edited(&prior, RecordData::Run(next)),
            created(&group, 0, RecordData::Run(fresh)),
        ]),
        &owner(),
    );
    let late = store.allocate_operation("late prior response").unwrap();
    store
        .prepare(
            &late,
            &request(vec![created(
                &late,
                0,
                RecordData::Note(Note::working("late", "late")),
            )]),
        )
        .unwrap();
    assert!(matches!(
        store.apply(&late, &ai().for_run(prior.id, 1)).unwrap(),
        ApplyOutcome::Superseded { .. }
    ));
}
#[test]
fn discarded_and_saved_sessions_clear_buffer_text_but_keep_late_write_tombstones() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "A", false);
    let session = store.begin_edit(&a.id, 1).unwrap();
    store
        .update_buffer(&session.id, 1, "discarded secret fixture")
        .unwrap();
    store.discard(&session.id, 1).unwrap();
    let closed = store.edit_session(&session.id).unwrap();
    assert!(closed.closed);
    assert!(closed.markdown.is_empty());
    assert!(matches!(
        store
            .update_buffer(&session.id, 2, "late secret fixture")
            .unwrap(),
        BufferOutcome::Ignored(_)
    ));
    let session = store.begin_edit(&a.id, 1).unwrap();
    store
        .update_buffer(&session.id, 1, "intentional saved writing")
        .unwrap();
    assert!(matches!(
        store.save(&session.id, 1).unwrap(),
        SaveOutcome::Saved(_)
    ));
    assert!(store.edit_session(&session.id).unwrap().markdown.is_empty());
    assert!(matches!(
        store.save(&session.id, 1).unwrap(),
        SaveOutcome::Saved(_)
    ));
}
#[test]
fn configuration_reads_and_fences_runs_at_the_actual_writer_boundary() {
    let (_dir, mut store) = store();
    let settings = WorkspaceSettings {
        provider: "provider".into(),
        model: "model".into(),
        effort: "medium".into(),
        auth_file: None,
        credentials_dir: None,
        maintenance: true,
        review_first: false,
    };
    let setup = store
        .allocate_operation("settings for shared configuration")
        .unwrap();
    apply(
        &mut store,
        &setup,
        &request(vec![created(
            &setup,
            0,
            RecordData::Settings(settings.clone()),
        )]),
        &owner(),
    );
    let op = store
        .allocate_operation("configure allocated before other connection run")
        .unwrap();
    let mut other = Store::open(store.directory()).unwrap();
    let run = seed_run(&mut other, "peer starts invocation after allocation");
    let mut paused = settings;
    paused.maintenance = false;
    let result = store.configure_workspace(&op, &paused, &owner()).unwrap();
    let ApplyOutcome::Applied(receipt) = &result else {
        panic!()
    };
    assert!(receipt.writes.iter().any(|write| write.after.id == run.id));
    let RecordData::Run(cancelled) = other.record(&run.id).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(cancelled.state, RunState::Cancelled);
    assert_eq!(cancelled.fence, 2);
    assert_eq!(
        store.configure_workspace(&op, &paused, &owner()).unwrap(),
        result
    );
}
#[test]
fn thread_review_attention_can_reference_known_operations_but_links_remain_records_only() {
    let (_dir, mut store) = store();
    let candidate = store.allocate_operation("known review candidate").unwrap();
    store
        .prepare(
            &candidate,
            &request(vec![created(
                &candidate,
                0,
                RecordData::Note(Note::working("pending", "pending")),
            )]),
        )
        .unwrap();
    let op = store.allocate_operation("review attention").unwrap();
    apply(
        &mut store,
        &op,
        &request(vec![created(
            &op,
            0,
            RecordData::Thread(Thread {
                title: "Review exact candidate".into(),
                state: ThreadState::Open,
                attention: vec![Attention {
                    kind: AttentionKind::Review,
                    reason: "owner review".into(),
                    record: Some(candidate.as_str().into()),
                }],
            }),
        )]),
        &ai(),
    );
    let missing = store
        .allocate_operation("unknown attention identity")
        .unwrap();
    store
        .prepare(
            &missing,
            &request(vec![created(
                &missing,
                0,
                RecordData::Thread(Thread {
                    title: "Unknown".into(),
                    state: ThreadState::Open,
                    attention: vec![Attention {
                        kind: AttentionKind::Review,
                        reason: "invalid reference".into(),
                        record: Some("nonexistent-operation".into()),
                    }],
                }),
            )]),
        )
        .unwrap();
    assert!(store.apply(&missing, &ai()).is_err());
    let link = store
        .allocate_operation("operation is not a Link record target")
        .unwrap();
    store
        .prepare(
            &link,
            &request(vec![created(
                &link,
                0,
                RecordData::Link(Link {
                    from: op.creation_id(0),
                    to: candidate.as_str().into(),
                    relation: "invalid".into(),
                }),
            )]),
        )
        .unwrap();
    assert!(store.apply(&link, &ai()).is_err());
}
#[test]
fn save_maps_utf8_comments_and_marks_intersection_unresolved_in_actual_write_set() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "commented", false);
    let textop = store.allocate_operation("initial UTF-8 text").unwrap();
    apply(
        &mut store,
        &textop,
        &request(vec![edited(&a, change_text(&a, "Üks õun ja teine õun"))]),
        &ai(),
    );
    let current = store.record(&a.id).unwrap().unwrap();
    let commentop = store.allocate_operation("original comment").unwrap();
    let comment = Comment {
        note: a.id.clone(),
        base_version: 2,
        quote: "õun".into(),
        range: Some((5, 9)),
        mapped_version: None,
        mapped_range: None,
        body: "first occurrence".into(),
        unresolved: false,
    };
    let receipt = apply(
        &mut store,
        &commentop,
        &request(vec![created(
            &commentop,
            0,
            RecordData::Comment(comment.clone()),
        )]),
        &ai(),
    );
    let id = receipt.writes[0].after.id.clone();
    let session = store.begin_edit(&a.id, current.version).unwrap();
    store
        .update_buffer(&session.id, 1, "Uus: Üks õun ja teine õun")
        .unwrap();
    let SaveOutcome::Saved(saved) = store.save(&session.id, 1).unwrap() else {
        panic!()
    };
    assert!(saved.writes.iter().any(|write| write.after.id == id));
    let RecordData::Comment(mapped) = store.record(&id).unwrap().unwrap().data else {
        panic!()
    };
    assert_eq!(mapped.base_version, 2);
    assert_eq!(mapped.range, comment.range);
    assert_eq!(mapped.mapped_version, Some(3));
    assert_eq!(mapped.mapped_range, Some((10, 14)));
    assert!(!mapped.unresolved);
    let session = store.begin_edit(&a.id, 3).unwrap();
    store
        .update_buffer(&session.id, 1, "Uus: Üks pirn ja teine õun")
        .unwrap();
    let SaveOutcome::Saved(saved) = store.save(&session.id, 1).unwrap() else {
        panic!()
    };
    let RecordData::Comment(mapped) = store.record(&id).unwrap().unwrap().data else {
        panic!()
    };
    assert!(mapped.unresolved);
    assert_eq!(mapped.mapped_range, None);
    assert_eq!(mapped.quote, "õun");
    assert_eq!(mapped.mapped_version, Some(4));
    assert_eq!(saved.writes.len(), 2);
    let mut changed = mapped;
    changed.body = "later comment editing".into();
    let comment_record = store.record(&id).unwrap().unwrap();
    let later = store.allocate_operation("later comment writing").unwrap();
    apply(
        &mut store,
        &later,
        &request(vec![edited(&comment_record, RecordData::Comment(changed))]),
        &ai(),
    );
    let undo = store
        .allocate_operation("undo Save conflicts with later comment edit")
        .unwrap();
    assert_eq!(
        store.undo(&undo, &saved.operation, &owner()).unwrap(),
        ApplyOutcome::Stale { records: vec![id] }
    );
    assert_eq!(
        text(&store.record(&a.id).unwrap().unwrap()),
        "Uus: Üks pirn ja teine õun"
    );
}
#[test]
fn undo_save_restores_supported_anchor_at_new_compensating_revision() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "anchor Undo", false);
    let op = store.allocate_operation("anchor text").unwrap();
    apply(
        &mut store,
        &op,
        &request(vec![edited(&a, change_text(&a, "Üks õun"))]),
        &ai(),
    );
    let commentop = store.allocate_operation("anchor before Save").unwrap();
    apply(
        &mut store,
        &commentop,
        &request(vec![created(
            &commentop,
            0,
            RecordData::Comment(Comment {
                note: a.id.clone(),
                base_version: 2,
                quote: "õun".into(),
                range: Some((5, 9)),
                mapped_version: None,
                mapped_range: None,
                body: "clarify".into(),
                unresolved: false,
            }),
        )]),
        &ai(),
    );
    let session = store.begin_edit(&a.id, 2).unwrap();
    store.update_buffer(&session.id, 1, "Uus: Üks õun").unwrap();
    let SaveOutcome::Saved(saved) = store.save(&session.id, 1).unwrap() else {
        panic!()
    };
    let undo = store.allocate_operation("Undo mapped Save").unwrap();
    assert!(matches!(
        store.undo(&undo, &saved.operation, &owner()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let current = store.record(&a.id).unwrap().unwrap();
    assert_eq!(current.version, 4);
    let RecordData::Comment(comment) = store
        .record(&commentop.creation_id(0))
        .unwrap()
        .unwrap()
        .data
    else {
        panic!()
    };
    assert_eq!(comment.mapped_version, Some(4));
    assert_eq!(comment.mapped_range, Some((5, 9)));
    assert_eq!(comment.base_version, 2);
    assert!(!comment.unresolved);
    assert_eq!(text(&current), "Üks õun");
}
#[test]
fn working_run_start_rechecks_current_selection_and_accepts_planned_settings_overlay() {
    let (_dir, mut store) = store();
    let initial = WorkspaceSettings {
        provider: "Codex".into(),
        model: "selected-old".into(),
        effort: "medium".into(),
        auth_file: None,
        credentials_dir: None,
        maintenance: true,
        review_first: false,
    };
    let setup = store
        .allocate_operation("initial selection and thread")
        .unwrap();
    apply(
        &mut store,
        &setup,
        &request(vec![
            created(&setup, 0, RecordData::Settings(initial.clone())),
            created(
                &setup,
                1,
                RecordData::Thread(Thread {
                    title: "work".into(),
                    state: ThreadState::Open,
                    attention: vec![],
                }),
            ),
        ]),
        &owner(),
    );
    let captured = Run {
        thread: setup.creation_id(1),
        provider: "codex".into(),
        model: "selected-old".into(),
        guide_identity: "guide".into(),
        budget: 2,
        effort: "medium".into(),
        loaded_skills: vec![],
        fence: 1,
        state: RunState::Working,
        progress: "captured before configure".into(),
        operations: vec![],
        sources: vec![],
    };
    let mut changed = initial;
    changed.model = "selected-new".into();
    let configure = store
        .allocate_operation("configure before captured invocation starts")
        .unwrap();
    store
        .configure_workspace(&configure, &changed, &owner())
        .unwrap();
    let stale = store
        .allocate_operation("stale captured selection start")
        .unwrap();
    store
        .prepare_owner(
            &stale,
            &request(vec![created(&stale, 0, RecordData::Run(captured.clone()))]),
            &owner(),
        )
        .unwrap();
    assert!(store.apply(&stale, &owner()).is_err());
    assert!(store.record(&stale.creation_id(0)).unwrap().is_none());
    let current = store.record(&setup.creation_id(0)).unwrap().unwrap();
    let combined = store
        .allocate_operation("new selection and new invocation together")
        .unwrap();
    changed.model = "selected-next".into();
    changed.effort = "high".into();
    let mut fresh = captured;
    fresh.model = changed.model.clone();
    fresh.effort = changed.effort.clone();
    apply(
        &mut store,
        &combined,
        &request(vec![
            edited(&current, RecordData::Settings(changed)),
            created(&combined, 0, RecordData::Run(fresh)),
        ]),
        &owner(),
    );
}

#[test]
fn configuration_replay_checks_immutable_payload_before_receipt() {
    let (_dir, mut store) = store();
    let settings = WorkspaceSettings {
        provider: "Codex".into(),
        model: "selected".into(),
        effort: "medium".into(),
        auth_file: Some("/synthetic/account-a".into()),
        credentials_dir: None,
        maintenance: true,
        review_first: false,
    };
    let seed = store.allocate_operation("settings replay seed").unwrap();
    apply(
        &mut store,
        &seed,
        &request(vec![created(
            &seed,
            0,
            RecordData::Settings(settings.clone()),
        )]),
        &owner(),
    );
    let op = store
        .allocate_operation("configuration immutable retry")
        .unwrap();
    let mut configured = settings;
    configured.maintenance = false;
    let first = store
        .configure_workspace(&op, &configured, &owner())
        .unwrap();
    let data = store.directory().to_owned();
    drop(store);
    let mut store = Store::open(data).unwrap();
    assert_eq!(
        store
            .configure_workspace(&op, &configured, &owner())
            .unwrap(),
        first
    );
    let mut changed = configured.clone();
    changed.auth_file = Some("/synthetic/account-b".into());
    assert!(matches!(
        store.configure_workspace(&op, &changed, &owner()),
        Err(Error::ImmutableRequest)
    ));
    changed = configured.clone();
    changed.maintenance = true;
    assert!(matches!(
        store.configure_workspace(&op, &changed, &owner()),
        Err(Error::ImmutableRequest)
    ));
    assert_eq!(
        store.record(&seed.creation_id(0)).unwrap().unwrap().data,
        RecordData::Settings(configured)
    );
}
#[test]
fn replacement_replay_binds_original_and_request_across_restart_and_application() {
    let (_dir, mut store) = store();
    let a = note(&mut store, "replacement replay note", false);
    let original = store.allocate_operation("replacement original").unwrap();
    store
        .prepare(
            &original,
            &request(vec![edited(&a, change_text(&a, "draft"))]),
        )
        .unwrap();
    let replacement = store
        .allocate_operation("replacement retry identity")
        .unwrap();
    let final_request = request(vec![edited(&a, change_text(&a, "owner final"))]);
    let first = store
        .replace_candidate(&original, &replacement, &final_request, &owner())
        .unwrap();
    let history = store.receipts().unwrap().len();
    let data = store.directory().to_owned();
    drop(store);
    let mut store = Store::open(data).unwrap();
    assert_eq!(
        store
            .replace_candidate(&original, &replacement, &final_request, &owner())
            .unwrap(),
        first
    );
    assert_eq!(store.receipts().unwrap().len(), history);
    let unrelated = store
        .allocate_operation("different original binding")
        .unwrap();
    assert!(matches!(
        store.replace_candidate(&unrelated, &replacement, &final_request, &owner()),
        Err(Error::ImmutableRequest)
    ));
    assert!(matches!(
        store.replace_candidate(&replacement, &replacement, &final_request, &owner()),
        Err(Error::ImmutableRequest)
    ));
    let mut changed = final_request.clone();
    changed.reason.push_str(" altered");
    assert!(matches!(
        store.replace_candidate(&original, &replacement, &changed, &owner()),
        Err(Error::ImmutableRequest)
    ));
    assert!(matches!(
        store.apply(&replacement, &owner()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(
        store
            .replace_candidate(&original, &replacement, &final_request, &owner())
            .unwrap(),
        first
    );
    assert_eq!(store.record(&a.id).unwrap().unwrap().version, 2);
}
#[test]
fn runtime_settings_snapshot_covers_account_changes_and_provider_aliases() {
    let (_dir, mut store) = store();
    let settings = WorkspaceSettings {
        provider: "ChatGPT".into(),
        model: "model".into(),
        effort: "medium".into(),
        auth_file: Some("/synthetic/account-a".into()),
        credentials_dir: None,
        maintenance: true,
        review_first: false,
    };
    let setup = store
        .allocate_operation("runtime snapshot settings thread")
        .unwrap();
    apply(
        &mut store,
        &setup,
        &request(vec![
            created(&setup, 0, RecordData::Settings(settings.clone())),
            created(
                &setup,
                1,
                RecordData::Thread(Thread {
                    title: "invoke".into(),
                    state: ThreadState::Open,
                    attention: vec![],
                }),
            ),
        ]),
        &owner(),
    );
    let captured = store.record(&setup.creation_id(0)).unwrap().unwrap();
    let run = Run {
        thread: setup.creation_id(1),
        provider: "Codex".into(),
        model: "model".into(),
        guide_identity: "guide".into(),
        budget: 2,
        effort: "medium".into(),
        loaded_skills: vec![],
        fence: 1,
        state: RunState::Working,
        progress: "captured selection".into(),
        operations: vec![],
        sources: vec![],
    };
    let start = store
        .allocate_operation("start with captured configuration")
        .unwrap();
    store
        .prepare(
            &start,
            &request(vec![created(&start, 0, RecordData::Run(run.clone()))]),
        )
        .unwrap();
    assert!(
        store
            .apply(
                &start,
                &HostAuthority::runtime("coordinator", start.creation_id(0))
            )
            .is_err()
    );
    let bound = HostAuthority::runtime("coordinator", start.creation_id(0))
        .for_settings(&captured.id, captured.version);
    let receipt = store.apply(&start, &bound).unwrap();
    assert!(matches!(receipt, ApplyOutcome::Applied(_)));
    let mut changed = settings;
    changed.auth_file = Some("/synthetic/account-b".into());
    let configure = store
        .allocate_operation("switch account after capture")
        .unwrap();
    store
        .configure_workspace(&configure, &changed, &owner())
        .unwrap();
    // Committed responses replay after later configuration changes.
    assert_eq!(store.apply(&start, &bound).unwrap(), receipt);
    let stale = store
        .allocate_operation("captured old account starts too late")
        .unwrap();
    store
        .prepare(
            &stale,
            &request(vec![created(&stale, 0, RecordData::Run(run.clone()))]),
        )
        .unwrap();
    let stale_authority = HostAuthority::runtime("coordinator", stale.creation_id(0))
        .for_settings(&captured.id, captured.version);
    assert!(store.apply(&stale, &stale_authority).is_err());
    assert!(store.record(&stale.creation_id(0)).unwrap().is_none());
    let fresh = store.record(&captured.id).unwrap().unwrap();
    let fresh_authority = HostAuthority::runtime("coordinator", stale.creation_id(0))
        .for_settings(&fresh.id, fresh.version);
    assert!(matches!(
        store.apply(&stale, &fresh_authority).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let target = note(&mut store, "late tool result", false);
    let tool = store
        .allocate_operation("tool bound to old account snapshot")
        .unwrap();
    store
        .prepare(
            &tool,
            &request(vec![edited(&target, change_text(&target, "late"))]),
        )
        .unwrap();
    assert!(
        store
            .apply(&tool, &ai().for_settings(&captured.id, captured.version))
            .is_err()
    );
    assert_eq!(store.record(&target.id).unwrap().unwrap(), target);
}
