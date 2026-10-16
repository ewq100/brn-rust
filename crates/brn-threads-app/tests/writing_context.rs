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
