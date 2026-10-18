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
