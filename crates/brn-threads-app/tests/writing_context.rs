use brn_threads_app::*;

#[test]
fn explicit_conversations_are_scoped_durable_and_inspection_does_not_create() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut app = Workspace::open_without_default_credentials(&data).unwrap();
    let note = app.new_note("Shared", "Saved").unwrap();
    let unrelated = app.new_thread("Arbitrary first link").unwrap();
    // Existing unrelated links are context, not an explicit writing choice.
    let link_id = app
        .store
        .allocate_operation("old-link")
        .unwrap()
        .creation_id(0);
    app.owner_change(
        "old-link",
        &ChangeRequest {
            reason: "Synthetic pre-existing link".into(),
            writes: vec![Put {
                id: link_id,
                expected_version: None,
                archived: false,
                data: RecordData::Link(Link {
                    from: unrelated.id.clone(),
                    to: note.id.clone(),
                    relation: "discusses".into(),
                }),
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    assert_eq!(
        app.active_writing_context(&note.id, None)
            .unwrap()
            .conversation,
        None
    );
    let count = app.records().unwrap().len();
    app.active_writing_context(&note.id, None).unwrap();
    assert_eq!(app.records().unwrap().len(), count);
    let mut scopes = Vec::new();
    for key in ["P1", "P2"] {
        let p = app
            .project_command(key, ProjectCommand::Create { title: key.into() })
            .unwrap()
            .writes[0]
            .after
            .id
            .clone();
        app.project_command(
            &format!("{key}-link"),
            ProjectCommand::LinkDocument {
                project: p.clone(),
                note: note.id.clone(),
            },
        )
        .unwrap();
        let first = app.ensure_writing_context(key, &note.id, Some(&p)).unwrap();
        let next = app
            .new_document_chat("Explicit second writing conversation", &note.id, Some(&p))
            .unwrap();
        app.link_writing_thread(&format!("{key}-explicit-link"), &note.id, 1, &next.id)
            .unwrap();
        assert_ne!(first.conversation.as_deref(), Some(next.id.as_str()));
        let snap = app
            .snapshot_writing_request(&format!("{key}-request"), &note.id, 1, &next.id, &[])
            .unwrap();
        assert_eq!(snap.setup.unwrap().chat, Some(p.clone()));
        let records = app.records().unwrap();
        let history = app.history().unwrap();
        app.select_writing_conversation(&format!("{key}-same"), &note.id, &next.id)
            .unwrap();
        assert_eq!(
            app.records().unwrap(),
            records,
            "repeated active choice writes nothing"
        );
        assert_eq!(app.history().unwrap(), history);
        let first_id = first.conversation.unwrap();
        app.select_writing_conversation(&format!("{key}-choose-first"), &note.id, &first_id)
            .unwrap();
        assert_eq!(
            app.active_writing_context(&note.id, Some(&p))
                .unwrap()
                .conversation
                .as_deref(),
            Some(first_id.as_str())
        );
        let choice = app.history().unwrap().last().unwrap().operation.clone();
        assert!(matches!(
            app.undo(choice.as_str()).unwrap(),
            ApplyOutcome::Applied(_)
        ));
        assert_eq!(
            app.active_writing_context(&note.id, Some(&p))
                .unwrap()
                .conversation
                .as_deref(),
            Some(next.id.as_str()),
            "Undo ignores the archived choice without creating another selection"
        );
        scopes.push((p, next.id));
    }
    drop(app);
    let mut app = Workspace::open_without_default_credentials(&data).unwrap();
    for (p, thread) in scopes {
        let current = app.active_writing_context(&note.id, Some(&p)).unwrap();
        assert_eq!(current.scope, Some(p.clone()));
        assert_eq!(current.conversation, Some(thread.clone()));
        assert_eq!(
            app.ensure_writing_context("closed-chat-restart", &note.id, Some(&p))
                .unwrap(),
            current
        );
    }
    assert_eq!(
        app.active_writing_context(&note.id, None)
            .unwrap()
            .conversation,
        None
    );
    assert_eq!(app.store.note(&note.id).unwrap().unwrap().version, 1);
}

#[test]
fn knowledge_off_display_batch_keeps_only_the_actual_linked_working_note_exception() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    let note = app.new_note("Working", "Saved working text").unwrap();
    let project = app
        .project_command(
            "project",
            ProjectCommand::Create {
                title: "Project".into(),
            },
        )
        .unwrap()
        .writes[0]
        .after
        .clone();
    let chat = app
        .project_command(
            "chat",
            ProjectCommand::CreateChat {
                project: project.id.clone(),
                title: "Writing".into(),
            },
        )
        .unwrap()
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Thread(_)))
        .unwrap()
        .after;
    app.link_writing_thread("explicit-working-link", &note.id, note.version, &chat.id)
        .unwrap();
    let project = app.store.record(&project.id).unwrap().unwrap();
    let RecordData::Project(p) = project.data else {
        panic!()
    };
    app.project_command(
        "knowledge-off",
        ProjectCommand::Configure {
            project: project.id.clone(),
            version: project.version,
            title: p.title,
            instructions: p.instructions,
            use_brn_knowledge: false,
        },
    )
    .unwrap();
    assert!(
        app.store
            .record_visible_in_chat(&note.id, Some(&chat.id))
            .unwrap()
    );
    assert!(
        !app.store
            .record_visible_in_chat(&note.id, Some(&project.id))
            .unwrap()
    );
    let snapshot = app
        .snapshot_writing_request("request", &note.id, note.version, &chat.id, &[])
        .unwrap();
    assert_eq!(
        app.active_writing_context(&note.id, Some(&project.id))
            .unwrap()
            .conversation
            .as_deref(),
        Some(chat.id.as_str())
    );
    let (token, batch) = app
        .writing_context_review(&note.id, Some(&project.id), &[])
        .unwrap();
    assert_eq!(token, app.store.change_token().unwrap());
    assert_eq!(
        batch.requests.get(&snapshot.request_message),
        Some(&snapshot)
    );
    let (_, general) = app.writing_context_review(&note.id, None, &[]).unwrap();
    assert!(
        !general.requests.contains_key(&snapshot.request_message),
        "a public working Note remains openable generally, but its private-scope request must stay hidden"
    );
    assert!(
        app.writing_request_in_context(&snapshot.request_message, &note.id, None)
            .is_err()
    );
}

fn recovery_workspace() -> (tempfile::TempDir, Workspace) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let app = Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    (root, app)
}

fn change_note(app: &mut Workspace, key: &str, id: &str, change: impl FnOnce(&mut Note)) -> Record {
    let current = app.store.note(id).unwrap().unwrap();
    let RecordData::Note(mut note) = current.data else {
        panic!("expected Note")
    };
    change(&mut note);
    let ApplyOutcome::Applied(receipt) = app
        .owner_change(
            key,
            &ChangeRequest {
                reason: "Synthetic recovery-copy fixture change".into(),
                writes: vec![Put {
                    id: id.into(),
                    expected_version: Some(current.version),
                    archived: false,
                    data: RecordData::Note(note),
                }],
                inputs: vec![],
            },
        )
        .unwrap()
    else {
        panic!("synthetic owner change was not applied")
    };
    receipt.writes[0].after.clone()
}

#[test]
fn recovery_copy_refuses_private_basis_current_source_and_captured_scope() {
    let (_root, mut app) = recovery_workspace();
    let note = app.new_note("Working", "General v1").unwrap();
    let chat = app.new_thread("Private context").unwrap();

    // A private historical basis stays private even after the Note is retained
    // generally in a later version.
    let private_v2 = change_note(&mut app, "private-v2", &note.id, |note| {
        note.chat = Some(chat.id.clone());
        note.markdown = "Private historical basis".into();
    });
    change_note(&mut app, "retain-general", &note.id, |note| {
        note.chat = None
    });
    let records = app.records().unwrap();
    let history = app.history().unwrap();
    let error = app
        .save_recovery_copy(
            "private-historical-basis",
            &note.id,
            private_v2.version,
            None,
            "Recovered",
            "private draft",
        )
        .unwrap_err();
    assert!(error.to_string().contains("private work"));
    assert_eq!(app.records().unwrap(), records);
    assert_eq!(app.history().unwrap(), history);

    // A general historical basis cannot be copied after its current source
    // enters private scope.
    let current_private = change_note(&mut app, "private-current", &note.id, |note| {
        note.chat = Some(chat.id.clone());
    });
    let records = app.records().unwrap();
    let history = app.history().unwrap();
    let error = app
        .save_recovery_copy(
            "private-current-source",
            &note.id,
            note.version,
            None,
            "Recovered",
            "draft from general basis",
        )
        .unwrap_err();
    assert!(error.to_string().contains("private work"));
    assert_eq!(app.records().unwrap(), records);
    assert_eq!(app.history().unwrap(), history);
    assert_eq!(app.store.note(&note.id).unwrap(), Some(current_private));

    // A public Note opened under a private/project context still refuses the
    // general-only action.
    let public = app.new_note("Public in private context", "Saved").unwrap();
    assert!(
        app.save_recovery_copy(
            "private-captured-context",
            &public.id,
            public.version,
            Some(&chat.id),
            "Recovered",
            "draft in scoped context",
        )
        .unwrap_err()
        .to_string()
        .contains("private work")
    );
}

#[test]
fn recovery_copy_preserves_public_stale_basis_and_replays_immutable_intent() {
    let (root, mut app) = recovery_workspace();
    let note = app.new_note("Source", "Saved v1").unwrap();
    let private_chat = app.new_thread("Private later source scope").unwrap();
    let current = change_note(&mut app, "source-v2", &note.id, |note| {
        note.markdown = "Saved v2".into();
    });
    let text = "õun 私密\nRecovered exact bytes\n";
    let receipt = app
        .save_recovery_copy(
            "session-7:3",
            &note.id,
            note.version,
            None,
            "Recovered Source",
            text,
        )
        .unwrap();
    assert_eq!(receipt.writes.len(), 1);
    let copy = &receipt.writes[0].after;
    assert!(matches!(&copy.data, RecordData::Note(created)
        if created.title == "Recovered Source" && created.markdown == text
            && created.chat.is_none() && !created.protected && created.import.is_none()));
    let actual = app.store.prepared(&receipt.operation).unwrap();
    assert_eq!(
        actual.request.inputs,
        vec![
            RevisionInput {
                record: note.id.clone(),
                version: note.version,
                expectation: InputExpectation::Historical,
            },
            RevisionInput {
                record: note.id.clone(),
                version: current.version,
                expectation: InputExpectation::Current,
            },
        ],
        "the emitted copy request binds its old text basis and observed current source"
    );

    let same_version = app.new_note("Same-version source", "Saved once").unwrap();
    let same_copy = app
        .save_recovery_copy(
            "same-version-session:1",
            &same_version.id,
            same_version.version,
            None,
            "Same-version recovery",
            "Same-version text",
        )
        .unwrap();
    let same_request = app.store.prepared(&same_copy.operation).unwrap();
    assert_eq!(
        same_request.request.inputs,
        vec![
            RevisionInput {
                record: same_version.id.clone(),
                version: same_version.version,
                expectation: InputExpectation::Historical,
            },
            RevisionInput {
                record: same_version.id.clone(),
                version: same_version.version,
                expectation: InputExpectation::Current,
            },
        ]
    );

    // Clone the actual service request under a fresh prepared operation. Its
    // Current fence must reject a privacy change after preparation.
    let race_operation = app
        .store
        .allocate_operation("recovery-copy-current-race")
        .unwrap();
    let mut race_request = actual.request.clone();
    let race_copy_id = race_operation.creation_id(0);
    assert_eq!(race_request.writes.len(), 1);
    race_request.writes[0].id = race_copy_id.clone();
    let authority = HostAuthority::owner("recovery-copy current-fence test");
    app.store
        .prepare_owner(&race_operation, &race_request, &authority)
        .unwrap();

    let mut independent =
        Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    let private_source = change_note(
        &mut independent,
        "source-becomes-private",
        &note.id,
        |note| {
            note.chat = Some(private_chat.id.clone());
        },
    );
    assert_eq!(
        app.store.note(&note.id).unwrap(),
        Some(private_source.clone())
    );
    let records_before_apply = app.records().unwrap();
    let history_before_apply = app.history().unwrap();
    assert!(matches!(
        app.store.apply(&race_operation, &authority).unwrap(),
        ApplyOutcome::Stale { records } if records.contains(&note.id)
    ));
    assert_eq!(app.records().unwrap(), records_before_apply);
    assert_eq!(app.history().unwrap(), history_before_apply);
    assert_eq!(app.store.record(&race_copy_id).unwrap(), None);
    assert_eq!(app.store.receipt(&race_operation).unwrap(), None);
    assert_eq!(app.store.note(&note.id).unwrap(), Some(private_source));
    drop(independent);
    drop(app);

    // Reopening and retrying the original identical intent returns its exact
    // accepted receipt before the now-private source is revalidated.
    let mut reopened =
        Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    assert_eq!(
        reopened
            .save_recovery_copy(
                "session-7:3",
                &note.id,
                note.version,
                None,
                "Recovered Source",
                text,
            )
            .unwrap(),
        receipt
    );
    assert_eq!(reopened.store.note(&copy.id).unwrap().unwrap(), *copy);
    let records_before_changed_intent = reopened.records().unwrap();
    let history_before_changed_intent = reopened.history().unwrap();
    assert!(
        reopened
            .save_recovery_copy(
                "session-7:3",
                &note.id,
                note.version,
                None,
                "Recovered Source",
                "changed intent",
            )
            .is_err()
    );
    assert_eq!(reopened.records().unwrap(), records_before_changed_intent);
    assert_eq!(reopened.history().unwrap(), history_before_changed_intent);
}

#[test]
fn recovery_copy_fails_closed_for_missing_wrong_kind_archived_and_superseded_basis() {
    let (_root, mut app) = recovery_workspace();
    let note = app.new_note("Source", "Saved").unwrap();
    let thread = app.new_thread("Not a Note").unwrap();
    let cases = [
        ("missing-source", "missing-note", 1),
        ("missing-basis", note.id.as_str(), 99),
        ("wrong-kind", thread.id.as_str(), thread.version),
    ];
    for (key, source, version) in cases {
        let records = app.records().unwrap();
        let history = app.history().unwrap();
        assert!(
            app.save_recovery_copy(key, source, version, None, "Recovered", "draft")
                .is_err()
        );
        assert_eq!(app.records().unwrap(), records);
        assert_eq!(app.history().unwrap(), history);
    }

    let current = app.store.note(&note.id).unwrap().unwrap();
    let replacement = app.new_note("Replacement", "Current source").unwrap();
    let mut retired = current.clone();
    let RecordData::Note(note_data) = &mut retired.data else {
        panic!("expected Note")
    };
    note_data.superseded_by = Some(replacement.id);
    let ApplyOutcome::Applied(_) = app
        .owner_change(
            "supersede-source",
            &ChangeRequest {
                reason: "Synthetic supersession".into(),
                writes: vec![Put {
                    id: note.id.clone(),
                    expected_version: Some(current.version),
                    archived: false,
                    data: retired.data,
                }],
                inputs: vec![],
            },
        )
        .unwrap()
    else {
        panic!("synthetic supersession was not applied")
    };
    let records = app.records().unwrap();
    let history = app.history().unwrap();
    assert!(
        app.save_recovery_copy(
            "superseded-source",
            &note.id,
            current.version,
            None,
            "Recovered",
            "draft",
        )
        .is_err()
    );
    assert_eq!(app.records().unwrap(), records);
    assert_eq!(app.history().unwrap(), history);

    let archived = app.new_note("Archived source", "Saved").unwrap();
    app.owner_change(
        "archive-source",
        &ChangeRequest {
            reason: "Synthetic archive".into(),
            writes: vec![Put {
                id: archived.id.clone(),
                expected_version: Some(archived.version),
                archived: true,
                data: archived.data.clone(),
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    let records = app.records().unwrap();
    let history = app.history().unwrap();
    assert!(
        app.save_recovery_copy(
            "archived-source",
            &archived.id,
            archived.version,
            None,
            "Recovered",
            "draft",
        )
        .is_err()
    );
    assert_eq!(app.records().unwrap(), records);
    assert_eq!(app.history().unwrap(), history);
}
