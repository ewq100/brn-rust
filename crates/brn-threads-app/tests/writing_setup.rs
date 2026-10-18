use brn_threads_app::*;
use sha2::{Digest, Sha256};

fn fixture() -> (tempfile::TempDir, Workspace, Record, Record) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    let note = app
        .new_note("Shared working Note", "Original saved text")
        .unwrap();
    let thread = app.new_thread("General writing").unwrap();
    app.link_writing_thread("link-general", &note.id, 1, &thread.id)
        .unwrap();
    (root, app, note, thread)
}
fn configuration(
    note: &str,
    chat: Option<&str>,
    brief: &str,
    refs: &[(&str, WritingPurpose)],
) -> WritingSetup {
    WritingSetup {
        note: note.into(),
        chat: chat.map(str::to_owned),
        brief: Some(brief.into()),
        references: refs
            .iter()
            .map(|(note, purpose)| WritingReference {
                note: (*note).into(),
                purpose: *purpose,
            })
            .collect(),
    }
}
fn project(app: &mut Workspace, key: &str) -> (String, String) {
    let p = app
        .project_command(key, ProjectCommand::Create { title: key.into() })
        .unwrap()
        .writes[0]
        .after
        .id
        .clone();
    let c = app
        .project_command(
            &format!("{key}-chat"),
            ProjectCommand::CreateChat {
                project: p.clone(),
                title: "Writing".into(),
            },
        )
        .unwrap()
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Thread(_)))
        .unwrap()
        .after
        .id;
    (p, c)
}
fn link(app: &mut Workspace, note: &Record, project: &str, chat: &str) {
    app.project_command(
        &format!("{project}-document"),
        ProjectCommand::LinkDocument {
            project: project.into(),
            note: note.id.clone(),
        },
    )
    .unwrap();
    app.link_writing_thread(&format!("link-{chat}"), &note.id, note.version, chat)
        .unwrap();
}
fn snapshot(app: &mut Workspace, key: &str, note: &Record, thread: &str) -> WritingSnapshot {
    app.snapshot_writing_request(key, &note.id, note.version, thread, &[])
        .unwrap()
}
fn config_record(receipt: &Receipt) -> Record {
    receipt
        .writes
        .iter()
        .find(|w| matches!(w.after.data, RecordData::WritingSetup(_)))
        .unwrap()
        .after
        .clone()
}

#[test]
fn scopes_are_separate_in_lists_search_export_and_requests_without_note_mutation() {
    let (root, mut app, note, general) = fixture();
    let reference = app
        .new_note("Evidence title", "Reference content must not be copied")
        .unwrap();
    let (a, ca) = project(&mut app, "project-a");
    let (b, cb) = project(&mut app, "project-b");
    link(&mut app, &note, &a, &ca);
    link(&mut app, &note, &b, &cb);
    let private = app.new_thread("Private writing").unwrap();
    let mut private_data = private.data.clone();
    if let RecordData::Thread(t) = &mut private_data {
        t.private_context = true;
    }
    app.owner_change(
        "existing-private-fixture",
        &ChangeRequest {
            reason: "Existing host-owned private context".into(),
            writes: vec![Put {
                id: private.id.clone(),
                expected_version: Some(private.version),
                archived: false,
                data: private_data,
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    app.link_writing_thread("link-private", &note.id, 1, &private.id)
        .unwrap();
    let mut configs = vec![];
    for (index, scope) in [
        None,
        Some(ca.as_str()),
        Some(b.as_str()),
        Some(private.id.as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        let brief = format!("secret-brief-{index}");
        let receipt = app
            .set_writing_setup(
                &brief,
                None,
                &configuration(
                    &note.id,
                    scope,
                    &brief,
                    &[(&reference.id, WritingPurpose::Evidence)],
                ),
            )
            .unwrap();
        configs.push(config_record(&receipt));
    }
    // A project chat and its Project ID address one canonical setup.
    assert_eq!(
        app.writing_setup(&note.id, Some(&ca)).unwrap().record,
        Some(configs[1].clone())
    );
    assert_eq!(
        app.writing_setup(&note.id, Some(&a)).unwrap().record,
        Some(configs[1].clone())
    );
    for (index, chat) in [
        general.id.as_str(),
        ca.as_str(),
        cb.as_str(),
        private.id.as_str(),
    ]
    .into_iter()
    .enumerate()
    {
        let visible = app.store.records_for_chat(Some(chat)).unwrap();
        let ids: Vec<_> = visible
            .iter()
            .filter(|r| matches!(r.data, RecordData::WritingSetup(_)))
            .map(|r| &r.id)
            .collect();
        assert_eq!(ids, vec![&configs[index].id]);
        let capture = snapshot(&mut app, &format!("request-{index}"), &note, chat);
        let text = capture.message_text().unwrap();
        for other in 0..4 {
            assert_eq!(
                text.contains(&format!("secret-brief-{other}")),
                other == index
            );
        }
        assert!(!text.contains("Reference content must not be copied"));
        assert!(!text.contains("Evidence title"));
        assert!(
            app.search_page("secret-brief", Some(chat), 12, None)
                .unwrap()
                .hits
                .is_empty()
        );
    }
    assert!(
        app.store
            .records_for_chat(None)
            .unwrap()
            .iter()
            .filter(|r| matches!(r.data, RecordData::WritingSetup(_)))
            .all(|r| r.id == configs[0].id)
    );
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
    let destination = root.path().join("export");
    app.export_now(&destination).unwrap();
    for entry in std::fs::read_dir(&destination).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let text = std::fs::read_to_string(path).unwrap();
            assert!(!text.contains("secret-brief"));
        }
    }
    // Moving the conversation must not move its saved project setup/request metadata.
    let old = app
        .messages(&ca)
        .unwrap()
        .into_iter()
        .find(|r| matches!(&r.data, RecordData::Message(m) if m.role == WRITING_REQUEST_ROLE))
        .unwrap();
    app.project_command(
        "move-a-to-b",
        ProjectCommand::MoveChat {
            chat: ca.clone(),
            project: Some(b),
        },
    )
    .unwrap();
    assert!(
        !app.store
            .record_visible_in_chat(&old.id, Some(&ca))
            .unwrap()
    );
    assert!(
        !app.store
            .records_for_chat(Some(&ca))
            .unwrap()
            .iter()
            .any(|r| r.id == configs[1].id || r.id == old.id)
    );
    assert!(!app.messages(&ca).unwrap().iter().any(|r| r.id == old.id));
    assert!(
        !app.store
            .message_page(&ca, 0, 1)
            .unwrap()
            .0
            .iter()
            .any(|(_, r)| r.id == old.id)
    );
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
}

#[test]
fn checked_setup_replay_conflicts_restart_undo_and_backup_are_durable() {
    let (root, mut app, note, thread) = fixture();
    let input = configuration(&note.id, None, "First brief", &[]);
    let first = app.set_writing_setup("create", None, &input).unwrap();
    let first_record = config_record(&first);
    assert_eq!(
        app.set_writing_setup("create", None, &input).unwrap(),
        first
    );
    assert!(
        app.set_writing_setup("create", None, &configuration(&note.id, None, "Other", &[]))
            .is_err()
    );
    assert!(app.messages(&thread.id).unwrap().is_empty());
    assert!(app.writing_comments(&note.id).unwrap().is_empty());
    let mut other = Workspace::open(root.path().join("data")).unwrap();
    let updated = configuration(&note.id, None, "Later brief", &[]);
    let second = other
        .set_writing_setup("edit", Some(first_record.version), &updated)
        .unwrap();
    assert!(
        app.set_writing_setup("stale", Some(first_record.version), &input)
            .is_err()
    );
    assert!(matches!(
        app.undo(first.operation.as_str()).unwrap(),
        ApplyOutcome::Stale { .. }
    ));
    let backup = root.path().join("complete.sqlite3");
    app.store.backup(&backup).unwrap();
    drop(other);
    drop(app);
    let mut restored = Store::restore(&backup, root.path().join("restored")).unwrap();
    assert_eq!(
        restored.receipt(&second.operation).unwrap(),
        Some(second.clone())
    );
    assert_eq!(restored.record(&note.id).unwrap().unwrap(), note);
    let undo = restored.allocate_operation("restored-undo").unwrap();
    assert!(matches!(
        restored
            .undo(&undo, &second.operation, &HostAuthority::owner("Undo"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    drop(restored);
    let mut reopened = Workspace::open(root.path().join("data")).unwrap();
    assert_eq!(
        reopened
            .set_writing_setup("edit", Some(first_record.version), &updated)
            .unwrap(),
        second
    );
    assert!(matches!(
        reopened.undo(second.operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(
        reopened
            .writing_setup(&note.id, None)
            .unwrap()
            .record
            .unwrap()
            .data,
        first_record.data
    );
    let restored_version = reopened
        .writing_setup(&note.id, None)
        .unwrap()
        .record
        .unwrap()
        .version;
    let third = reopened
        .set_writing_setup("after-undo", Some(restored_version), &input)
        .unwrap();
    assert_eq!(config_record(&third).id, first_record.id);
    assert_eq!(reopened.store.record(&note.id).unwrap().unwrap(), note);
}

#[test]
fn setup_edits_stale_v2_candidates_and_replay_does_not_recapture() {
    let (root, mut app, note, thread) = fixture();
    let request = snapshot(&mut app, "capture", &note, &thread.id);
    let setup = request.setup.as_ref().unwrap();
    let proposal = app.store.allocate_operation("candidate").unwrap();
    let prepared = app
        .store
        .prepare(&proposal, &request.change_request("Proposed").unwrap())
        .unwrap();
    let next = configuration(&note.id, None, "Changed brief", &[]);
    app.set_writing_setup("change", Some(setup.input.version), &next)
        .unwrap();
    assert!(
        matches!(app.store.apply(&proposal,&HostAuthority::owner("review")).unwrap(),ApplyOutcome::Stale{records} if records.contains(&setup.input.record))
    );
    drop(app);
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    assert_eq!(snapshot(&mut app, "capture", &note, &thread.id), request);
    assert_eq!(app.store.prepared(&proposal).unwrap(), prepared);
    assert_eq!(
        app.writing_request(&request.request_message).unwrap(),
        request
    );
    assert_eq!(
        snapshot(&mut app, "refresh", &note, &thread.id)
            .setup
            .unwrap()
            .brief
            .as_deref(),
        Some("Changed brief")
    );
}

#[test]
fn revoked_archived_and_missing_references_are_unavailable_and_excluded_from_capture() {
    let (_root, mut app, note, _thread) = fixture();
    let (p, c) = project(&mut app, "scope");
    link(&mut app, &note, &p, &c);
    let public = app.new_note("Reference", "Do not copy me").unwrap();
    let receipt = app
        .set_writing_setup(
            "refs",
            None,
            &configuration(
                &note.id,
                Some(&p),
                "Brief",
                &[(&public.id, WritingPurpose::Requirements)],
            ),
        )
        .unwrap();
    let capture = snapshot(&mut app, "before", &note, &c);
    assert_eq!(capture.setup.as_ref().unwrap().references[0].version, 1);
    let project = app.store.record(&p).unwrap().unwrap();
    let RecordData::Project(data) = project.data else {
        panic!()
    };
    app.project_command(
        "revoke-general",
        ProjectCommand::Configure {
            project: p.clone(),
            version: project.version,
            title: data.title,
            instructions: data.instructions,
            use_brn_knowledge: false,
        },
    )
    .unwrap();
    let view = app.writing_setup(&note.id, Some(&p)).unwrap();
    assert_eq!(view.references[0].version, None);
    let current = snapshot(&mut app, "after", &note, &c);
    let text = current.message_text().unwrap();
    assert!(!text.contains(&public.id));
    assert!(current.setup.unwrap().references.is_empty());
    assert_eq!(
        app.writing_request(&capture.request_message).unwrap(),
        capture
    );
    // The owner can remove an unavailable link, but cannot add a new hidden one.
    let hidden = app.new_note("Hidden", "secret").unwrap();
    assert!(
        app.set_writing_setup(
            "hidden",
            Some(config_record(&receipt).version),
            &configuration(
                &note.id,
                Some(&p),
                "Brief",
                &[(&hidden.id, WritingPurpose::Style)]
            )
        )
        .is_err()
    );
    app.set_writing_setup(
        "remove",
        Some(config_record(&receipt).version),
        &configuration(&note.id, Some(&p), "Brief", &[]),
    )
    .unwrap();
}

#[test]
fn overflow_is_explicit_and_leaves_no_default_setup_or_request() {
    let (_root, mut app, note, thread) = fixture();
    let before = app.records().unwrap();
    let error = app
        .snapshot_writing_request_with_feedback(
            "overflow",
            &note.id,
            1,
            &thread.id,
            &[],
            &"é".repeat(MAX_WRITING_REQUEST_BYTES),
        )
        .unwrap_err();
    assert!(error.to_string().contains("nothing was truncated"));
    assert_eq!(app.records().unwrap(), before);
    assert!(
        app.set_writing_setup(
            "too-large",
            None,
            &configuration(&note.id, None, &"é".repeat(MAX_WRITING_REQUEST_BYTES), &[])
        )
        .is_err()
    );
    assert_eq!(app.records().unwrap(), before);
}

#[test]
fn v1_prepared_request_replay_keeps_original_bytes_hash_and_inputs() {
    let (root, mut app, note, thread) = fixture();
    let binding = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(
                "request",
                (&note.id, note.version, &thread.id, Vec::<String>::new())
            ))
            .unwrap()
        )
    );
    let op = app
        .store
        .allocate_bound_operation("writing-request:old", &binding)
        .unwrap();
    let legacy = WritingSnapshot {
        request_message: op.creation_id(0),
        thread: thread.id.clone(),
        note: note.clone(),
        comments: vec![],
        messages: vec![],
        general_feedback: String::new(),
        setup: None,
    };
    let bytes = legacy.message_text().unwrap();
    assert!(bytes.starts_with("{\"version\":1,"));
    let request = ChangeRequest {
        reason: "Legacy immutable capture".into(),
        writes: vec![Put {
            id: legacy.request_message.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: thread.id.clone(),
                role: WRITING_REQUEST_ROLE.into(),
                text: bytes.clone(),
            }),
        }],
        inputs: vec![],
    };
    let owner = HostAuthority::owner("fixture");
    let prepared = app.store.prepare_owner(&op, &request, &owner).unwrap();
    let hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&request).unwrap())
    );
    assert_eq!(prepared.request_hash, hash);
    // Lost prepare response: no receipt exists until reopening and retrying.
    drop(app);
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    assert_eq!(snapshot(&mut app, "old", &note, &thread.id), legacy);
    assert_eq!(app.store.prepared(&op).unwrap(), prepared);
    assert_eq!(
        app.writing_request(&legacy.request_message)
            .unwrap()
            .message_text()
            .unwrap(),
        bytes
    );
    let proposal = app.store.allocate_operation("legacy-proposal").unwrap();
    let candidate = app
        .store
        .prepare(&proposal, &legacy.change_request("Legacy result").unwrap())
        .unwrap();
    snapshot(&mut app, "new-v2", &note, &thread.id);
    let setup = app.writing_setup(&note.id, None).unwrap().record.unwrap();
    app.set_writing_setup(
        "new-config",
        Some(setup.version),
        &configuration(&note.id, None, "New brief", &[]),
    )
    .unwrap();
    drop(app);
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    assert_eq!(app.store.prepared(&proposal).unwrap(), candidate);
    assert!(matches!(
        app.store.apply(&proposal, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(app.store.prepared(&op).unwrap().request_hash, hash);
}

#[test]
fn ordered_reference_versions_and_purposes_survive_reopen_and_derived_regeneration() {
    let (root, mut app, note, thread) = fixture();
    let first = app
        .new_note("Requirements", "requirements content")
        .unwrap();
    let second = app.new_note("Evidence", "evidence content").unwrap();
    let third = app.new_note("Style", "style content").unwrap();
    let references = [
        (&third.id[..], WritingPurpose::Style),
        (&first.id[..], WritingPurpose::Requirements),
        (&second.id[..], WritingPurpose::Evidence),
    ];
    app.set_writing_setup(
        "ordered",
        None,
        &configuration(&note.id, None, "Brief", &references),
    )
    .unwrap();
    let capture = snapshot(&mut app, "ordered-capture", &note, &thread.id);
    let metadata = capture.setup.as_ref().unwrap();
    assert_eq!(
        metadata
            .references
            .iter()
            .map(|r| (r.note.as_str(), r.version, r.purpose))
            .collect::<Vec<_>>(),
        references
            .iter()
            .map(|(id, p)| (*id, 1, *p))
            .collect::<Vec<_>>()
    );
    let current = app.store.record(&first.id).unwrap().unwrap();
    let mut updated = current.data.clone();
    let RecordData::Note(n) = &mut updated else {
        panic!()
    };
    n.markdown = "updated requirements".into();
    app.owner_change(
        "update-reference",
        &ChangeRequest {
            reason: "Reference edit".into(),
            writes: vec![Put {
                id: first.id.clone(),
                expected_version: Some(1),
                archived: false,
                data: updated,
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    assert_eq!(
        app.writing_request(&capture.request_message).unwrap(),
        capture
    );
    let next = snapshot(&mut app, "new-reference-version", &note, &thread.id);
    assert_eq!(next.setup.as_ref().unwrap().references[1].version, 2);
    let mut archived = app.store.record(&third.id).unwrap().unwrap();
    archived.archived = true;
    app.owner_change(
        "archive-reference",
        &ChangeRequest {
            reason: "Archive reference".into(),
            writes: vec![Put {
                id: archived.id.clone(),
                expected_version: Some(archived.version),
                archived: true,
                data: archived.data,
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    assert!(
        app.writing_setup(&note.id, None).unwrap().references[0]
            .version
            .is_none()
    );
    assert_eq!(
        snapshot(&mut app, "after-archive", &note, &thread.id)
            .setup
            .unwrap()
            .references
            .len(),
        2
    );
    app.search("Original").unwrap();
    let derived = app.store.derived_directory().unwrap();
    drop(app);
    std::fs::remove_dir_all(&derived).unwrap();
    let mut app = Workspace::open(root.path().join("data")).unwrap();
    assert_eq!(
        app.writing_request(&capture.request_message).unwrap(),
        capture
    );
    assert_eq!(
        app.writing_setup(&note.id, None).unwrap().references.len(),
        3
    );
    assert_eq!(app.search("Original").unwrap().len(), 1);
}

#[test]
fn ordinary_chat_setup_read_set_capture_and_undo_keep_existing_general_identity() {
    let (_root, mut app, note, thread) = fixture();
    let before = snapshot(&mut app, "before-ordinary-setup", &note, &thread.id);
    let setup = before.setup.unwrap();
    assert_eq!(
        app.writing_setup(&note.id, Some(&thread.id))
            .unwrap()
            .record
            .unwrap()
            .id,
        setup.input.record
    );
    let receipt = app
        .set_writing_setup(
            "ordinary-chat",
            Some(setup.input.version),
            &configuration(&note.id, Some(&thread.id), "General brief", &[]),
        )
        .unwrap();
    let saved = config_record(&receipt);
    assert!(matches!(&saved.data,RecordData::WritingSetup(s) if s.chat.is_none()));
    assert_eq!(
        app.writing_setup(&note.id, None).unwrap().record,
        Some(saved.clone())
    );
    assert!(!app.store.chat_has_private_context(&thread.id).unwrap());
    assert_eq!(
        message_scope(&thread, &app.records().unwrap()),
        MessageScope::General
    );
    app.post_message(&thread.id, "scope-parity-search-witness")
        .unwrap();
    assert_eq!(
        app.search_page("scope-parity-search-witness", None, 10, None)
            .unwrap()
            .message_total,
        1
    );
    let after = snapshot(&mut app, "after-ordinary-setup", &note, &thread.id)
        .setup
        .unwrap();
    assert_eq!(after.chat, None);
    assert_eq!(after.input.record, saved.id);
    assert!(matches!(
        app.undo(receipt.operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(!app.store.chat_has_private_context(&thread.id).unwrap());
    assert_eq!(
        message_scope(&thread, &app.records().unwrap()),
        MessageScope::General
    );
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
}

#[test]
fn setup_creation_and_undo_cannot_make_ordinary_chat_permanently_private() {
    let (_root, mut app, note, thread) = fixture();
    let receipt = app
        .set_writing_setup(
            "new-in-chat",
            None,
            &configuration(&note.id, Some(&thread.id), "General brief", &[]),
        )
        .unwrap();
    assert!(!app.store.chat_has_private_context(&thread.id).unwrap());
    assert!(matches!(
        app.undo(receipt.operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(!app.store.chat_has_private_context(&thread.id).unwrap());
    assert_eq!(
        writing_request_scope(&app.records().unwrap(), &thread.id).unwrap(),
        None
    );
}

#[test]
fn knowledge_off_keeps_conversation_linked_working_note_but_not_linked_references() {
    let (_root, mut app, note, _general) = fixture();
    let (p, c) = project(&mut app, "knowledge-off");
    app.link_writing_thread("working-note-link", &note.id, note.version, &c)
        .unwrap();
    let project = app.store.record(&p).unwrap().unwrap();
    let RecordData::Project(data) = project.data else {
        panic!()
    };
    app.project_command(
        "knowledge-off-config",
        ProjectCommand::Configure {
            project: p.clone(),
            version: project.version,
            title: data.title,
            instructions: data.instructions,
            use_brn_knowledge: false,
        },
    )
    .unwrap();
    assert!(
        app.store
            .record_visible_in_chat(&note.id, Some(&c))
            .unwrap()
    );
    assert!(
        !app.store
            .record_visible_in_chat(&note.id, Some(&p))
            .unwrap()
    );
    assert!(
        app.writing_setup(&note.id, Some(&c))
            .unwrap()
            .record
            .is_none()
    );
    let captured = snapshot(&mut app, "knowledge-off-capture", &note, &c);
    let setup = captured.setup.unwrap();
    assert_eq!(setup.chat, Some(p.clone()));
    let reference = app
        .new_note("General reference", "Not project knowledge")
        .unwrap();
    app.link_writing_thread("reference-chat-link", &reference.id, 1, &c)
        .unwrap();
    assert!(
        app.store
            .record_visible_in_chat(&reference.id, Some(&c))
            .unwrap()
    );
    assert!(
        app.set_writing_setup(
            "strict-reference",
            Some(setup.input.version),
            &configuration(
                &note.id,
                Some(&c),
                "Brief",
                &[(&reference.id, WritingPurpose::Evidence)]
            )
        )
        .is_err()
    );
    let edited = app
        .set_writing_setup(
            "knowledge-off-edit",
            Some(setup.input.version),
            &configuration(&note.id, Some(&c), "Allowed working brief", &[]),
        )
        .unwrap();
    let current_chat = app.store.record(&c).unwrap().unwrap();
    let mut attention = current_chat.data;
    if let RecordData::Thread(t) = &mut attention {
        t.attention.push(Attention {
            kind: AttentionKind::Question,
            reason: "Unrelated attention".into(),
            record: None,
        });
    }
    app.owner_change(
        "unrelated-attention",
        &ChangeRequest {
            reason: "Attention must not change writing context".into(),
            writes: vec![Put {
                id: c.clone(),
                expected_version: Some(current_chat.version),
                archived: false,
                data: attention,
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    assert!(matches!(
        app.undo(edited.operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
    assert!(
        !app.store
            .records_for_chat(Some(&p))
            .unwrap()
            .iter()
            .any(|r| r.id == note.id)
    );
}

#[test]
fn inactive_conversations_keep_v1_v2_setup_inspection_but_refuse_new_writes() {
    for (state, archived, private) in [
        (ThreadState::Resolved, false, false),
        (ThreadState::Open, true, false),
        (ThreadState::Resolved, false, true),
    ] {
        let (_root, mut app, note, thread) = fixture();
        if private {
            let mut data = thread.data.clone();
            let RecordData::Thread(t) = &mut data else {
                panic!()
            };
            t.private_context = true;
            app.owner_change(
                "existing-private-context",
                &ChangeRequest {
                    reason: "Existing host privacy".into(),
                    writes: vec![Put {
                        id: thread.id.clone(),
                        expected_version: Some(thread.version),
                        archived: false,
                        data,
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        }
        let v2 = snapshot(&mut app, "active-v2", &note, &thread.id);
        let marker = app.store.allocate_operation("active-v1").unwrap();
        let mut v1 = v2.clone();
        v1.setup = None;
        v1.request_message = marker.creation_id(0);
        let original_v1 = v1.message_text().unwrap();
        app.owner_change(
            "active-v1",
            &ChangeRequest {
                reason: "Legacy request fixture".into(),
                writes: vec![Put {
                    id: v1.request_message.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: thread.id.clone(),
                        role: WRITING_REQUEST_ROLE.into(),
                        text: original_v1.clone(),
                    }),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
        let current = app.store.record(&thread.id).unwrap().unwrap();
        let mut data = current.data;
        let RecordData::Thread(t) = &mut data else {
            panic!()
        };
        t.state = state;
        app.owner_change(
            "inactive",
            &ChangeRequest {
                reason: "Legacy inactive conversation".into(),
                writes: vec![Put {
                    id: thread.id.clone(),
                    expected_version: Some(current.version),
                    archived,
                    data,
                }],
                inputs: vec![],
            },
        )
        .unwrap();
        assert!(
            app.store
                .record_visible_in_chat(&v1.request_message, Some(&thread.id))
                .unwrap()
        );
        assert!(
            app.store
                .record_visible_in_chat(&v2.request_message, Some(&thread.id))
                .unwrap()
        );
        let visible = app.store.records_for_chat(Some(&thread.id)).unwrap();
        for id in [
            &v1.request_message,
            &v2.request_message,
            &v2.setup.as_ref().unwrap().input.record,
        ] {
            assert!(visible.iter().any(|r| &r.id == id));
        }
        let messages = app.messages(&thread.id).unwrap();
        assert_eq!(messages.len(), 2);
        let (first, more) = app.store.message_page(&thread.id, 0, 1).unwrap();
        assert!(more);
        let (second, more) = app.store.message_page(&thread.id, first[0].0, 1).unwrap();
        assert!(!more);
        assert_eq!(first.len() + second.len(), 2);
        assert!(
            app.writing_setup(&note.id, Some(&thread.id))
                .unwrap()
                .record
                .is_some()
        );
        assert!(
            app.snapshot_writing_request("inactive-new-capture", &note.id, 1, &thread.id, &[])
                .is_err()
        );
        let version = v2.setup.as_ref().unwrap().input.version;
        assert!(
            app.set_writing_setup(
                "inactive-new-edit",
                Some(version),
                &configuration(&note.id, Some(&thread.id), "Refused", &[])
            )
            .is_err()
        );
        assert_eq!(
            app.writing_request(&v1.request_message)
                .unwrap()
                .message_text()
                .unwrap(),
            original_v1
        );
    }
}

#[test]
fn scoped_message_pages_cross_filtered_batches_without_losing_visible_lookahead() {
    for hidden in [1, 5] {
        let (_root, mut app, note, thread) = fixture();
        for i in 0..hidden {
            snapshot(&mut app, &format!("former-{i}"), &note, &thread.id);
        }
        let (p, _) = project(&mut app, "destination");
        app.project_command(
            "move",
            ProjectCommand::MoveChat {
                chat: thread.id.clone(),
                project: Some(p),
            },
        )
        .unwrap();
        let mut expected = Vec::new();
        for text in ["Visible first", "Visible second", "Visible third"] {
            let receipt = app.post_message(&thread.id, text).unwrap();
            expected.push(receipt.writes[0].after.id.clone());
        }
        let (first, more) = app.store.message_page(&thread.id, 0, 2).unwrap();
        assert_eq!(
            first.iter().map(|(_, r)| &r.id).collect::<Vec<_>>(),
            expected[..2].iter().collect::<Vec<_>>()
        );
        assert!(
            more,
            "a later visible message must remain after hidden={hidden}"
        );
        let (last, more) = app
            .store
            .message_page(&thread.id, first.last().unwrap().0, 2)
            .unwrap();
        assert_eq!(
            last.iter().map(|(_, r)| &r.id).collect::<Vec<_>>(),
            expected[2..].iter().collect::<Vec<_>>()
        );
        assert!(!more);
        assert_eq!(
            app.messages(&thread.id)
                .unwrap()
                .iter()
                .map(|r| &r.id)
                .collect::<Vec<_>>(),
            expected.iter().collect::<Vec<_>>()
        );
    }
}

#[test]
fn malformed_legacy_history_remains_visible_but_invalid_v2_metadata_is_hidden() {
    let (_root, mut app, _note, thread) = fixture();
    let mut ids = Vec::new();
    for (key, text) in [
        ("legacy", "retained legacy request body"),
        (
            "invalid-v2",
            r#"{"version":2,"snapshot":{"brief":"unavailable setup metadata"}}"#,
        ),
    ] {
        let op = app.store.allocate_operation(key).unwrap();
        let id = op.creation_id(0);
        app.owner_change(
            key,
            &ChangeRequest {
                reason: "Retained opaque request fixture".into(),
                writes: vec![Put {
                    id: id.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: thread.id.clone(),
                        role: WRITING_REQUEST_ROLE.into(),
                        text: text.into(),
                    }),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
        ids.push(id);
    }
    assert_eq!(
        app.messages(&thread.id)
            .unwrap()
            .iter()
            .map(|r| &r.id)
            .collect::<Vec<_>>(),
        vec![&ids[0]]
    );
    let (page, more) = app.store.message_page(&thread.id, 0, 1).unwrap();
    assert_eq!(
        page.iter().map(|(_, r)| &r.id).collect::<Vec<_>>(),
        vec![&ids[0]]
    );
    assert!(!more);
    let rows = app.store.records_for_chat(Some(&thread.id)).unwrap();
    assert!(rows.iter().any(|r| r.id == ids[0]));
    assert!(!rows.iter().any(|r| r.id == ids[1]));
}

#[test]
fn archived_project_with_retained_messages_keeps_v1_inspection_and_fences_v2_metadata() {
    let (_root, mut app, note, _) = fixture();
    let (p, c) = project(&mut app, "trash-visibility");
    link(&mut app, &note, &p, &c);
    let v2 = snapshot(&mut app, "v2", &note, &c);
    let op = app.store.allocate_operation("v1").unwrap();
    let mut v1 = v2.clone();
    v1.setup = None;
    v1.request_message = op.creation_id(0);
    let text = v1.message_text().unwrap();
    app.owner_change(
        "v1",
        &ChangeRequest {
            reason: "Retained v1 request".into(),
            writes: vec![Put {
                id: v1.request_message.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Message(Message {
                    thread: c.clone(),
                    role: WRITING_REQUEST_ROLE.into(),
                    text: text.clone(),
                }),
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    // The checked store can retain active history under an archived Project.
    // ProjectCommand::Delete separately archives its removed messages as before.
    let project = app.store.record(&p).unwrap().unwrap();
    app.owner_change(
        "archived-project",
        &ChangeRequest {
            reason: "Archived Project with retained history".into(),
            writes: vec![Put {
                id: p,
                expected_version: Some(project.version),
                archived: true,
                data: project.data,
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    let (page, more) = app.store.message_page(&c, 0, 10).unwrap();
    assert!(!more);
    assert_eq!(
        page.iter().map(|(_, r)| &r.id).collect::<Vec<_>>(),
        vec![&v1.request_message]
    );
    assert_eq!(
        app.messages(&c).unwrap(),
        page.into_iter().map(|(_, r)| r).collect::<Vec<_>>()
    );
    assert!(
        !app.store
            .record_visible_in_chat(&v2.request_message, Some(&c))
            .unwrap()
    );
    assert!(
        matches!(app.store.record(&v1.request_message).unwrap().unwrap().data, RecordData::Message(m) if m.text == text)
    );
}

#[test]
fn unrelated_note_write_with_request_provenance_does_not_depend_on_writing_setup() {
    let (_root, mut app, note, thread) = fixture();
    let capture = snapshot(&mut app, "capture", &note, &thread.id);
    let other = app.new_note("Unrelated", "Before").unwrap();
    let mut data = other.data.clone();
    if let RecordData::Note(n) = &mut data {
        n.markdown = "After".into();
    }
    let op = app.store.allocate_operation("unrelated-candidate").unwrap();
    let request = ChangeRequest {
        reason: "Owner edit with retained request provenance".into(),
        writes: vec![Put {
            id: other.id.clone(),
            expected_version: Some(other.version),
            archived: false,
            data,
        }],
        inputs: vec![RevisionInput {
            record: capture.request_message,
            version: 1,
            expectation: InputExpectation::Current,
        }],
    };
    let owner = HostAuthority::owner("unrelated owner edit");
    app.store.prepare_owner(&op, &request, &owner).unwrap();
    let setup = capture.setup.unwrap();
    app.set_writing_setup(
        "later-setup",
        Some(setup.input.version),
        &configuration(&note.id, None, "Changed independently", &[]),
    )
    .unwrap();
    assert!(matches!(
        app.store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(
        matches!(app.store.record(&other.id).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "After")
    );
}

#[test]
fn stored_v2_context_rejects_forged_binding_and_reference_metadata() {
    let (_root, mut app, note, thread) = fixture();
    let reference = app.new_note("Reference", "Saved evidence").unwrap();
    let style = app.new_note("Style", "Saved style").unwrap();
    let other = app
        .new_note("Other working Note", "Canonical unrelated text")
        .unwrap();
    app.set_writing_setup(
        "setup",
        None,
        &configuration(
            &note.id,
            None,
            "Brief",
            &[
                (&reference.id, WritingPurpose::Evidence),
                (&style.id, WritingPurpose::Style),
            ],
        ),
    )
    .unwrap();
    let captured = snapshot(&mut app, "canonical", &note, &thread.id);
    for (index, (pointer, value)) in [
        (
            "/snapshot/setup/input/expectation",
            serde_json::json!("Historical"),
        ),
        ("/snapshot/setup/input/version", serde_json::json!(0)),
        ("/snapshot/setup/input/record", serde_json::json!(note.id)),
        ("/snapshot/setup/chat", serde_json::json!("")),
        ("/snapshot/setup/references/0/version", serde_json::json!(0)),
        (
            "/snapshot/setup/references/0/note",
            serde_json::json!(note.id),
        ),
        ("/snapshot/setup/references/0/note", serde_json::json!("")),
    ]
    .into_iter()
    .enumerate()
    {
        let key = format!("forged-{index}");
        let op = app.store.allocate_operation(&key).unwrap();
        let id = op.creation_id(0);
        let mut payload: serde_json::Value =
            serde_json::from_str(&captured.message_text().unwrap()).unwrap();
        payload["snapshot"]["request_message"] = serde_json::json!(id);
        if pointer == "/snapshot/setup/input/expectation" {
            // Current is deliberately omitted from canonical serialized input.
            payload["snapshot"]["setup"]["input"]["expectation"] = value;
        } else {
            *payload.pointer_mut(pointer).unwrap() = value;
        }
        let outcome = app
            .owner_change(
                &key,
                &ChangeRequest {
                    reason: "Retained modified request fixture".into(),
                    writes: vec![Put {
                        id: id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Message(Message {
                            thread: thread.id.clone(),
                            role: WRITING_REQUEST_ROLE.into(),
                            text: serde_json::to_string(&payload).unwrap(),
                        }),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        assert!(matches!(outcome, ApplyOutcome::Applied(_)));
        assert!(
            WritingSnapshot::from_message(&app.store.record(&id).unwrap().unwrap()).is_err(),
            "{pointer}"
        );
        assert!(app.writing_request(&id).is_err(), "{pointer}");
    }
    for (index, (pointer, value)) in [
        ("/snapshot/note", serde_json::to_value(&other).unwrap()),
        (
            "/snapshot/setup/references",
            serde_json::json!([
                {"note": style.id, "version": 1, "purpose": "Style"},
                {"note": reference.id, "version": 1, "purpose": "Evidence"}
            ]),
        ),
        (
            "/snapshot/setup/brief",
            serde_json::json!("Not the saved brief"),
        ),
        ("/snapshot/setup/chat", serde_json::json!("another-scope")),
        (
            "/snapshot/setup/input/record",
            serde_json::json!(reference.id),
        ),
        ("/snapshot/setup/input/version", serde_json::json!(999)),
        (
            "/snapshot/setup/references/0/purpose",
            serde_json::json!("Style"),
        ),
        (
            "/snapshot/setup/references/0/version",
            serde_json::json!(999),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let key = format!("noncanonical-{index}");
        let op = app.store.allocate_operation(&key).unwrap();
        let id = op.creation_id(0);
        let mut payload: serde_json::Value =
            serde_json::from_str(&captured.message_text().unwrap()).unwrap();
        payload["snapshot"]["request_message"] = serde_json::json!(id);
        *payload.pointer_mut(pointer).unwrap() = value;
        let outcome = app
            .owner_change(
                &key,
                &ChangeRequest {
                    reason: "Retained noncanonical request fixture".into(),
                    writes: vec![Put {
                        id: id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Message(Message {
                            thread: thread.id.clone(),
                            role: WRITING_REQUEST_ROLE.into(),
                            text: serde_json::to_string(&payload).unwrap(),
                        }),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        assert!(matches!(outcome, ApplyOutcome::Applied(_)));
        assert!(
            WritingSnapshot::from_message(&app.store.record(&id).unwrap().unwrap()).is_ok(),
            "well formed {pointer}"
        );
        assert!(
            app.writing_request(&id).is_err(),
            "canonical evidence {pointer}"
        );
    }
    let key = "duplicate-reference";
    let op = app.store.allocate_operation(key).unwrap();
    let mut invalid = captured.clone();
    invalid.request_message = op.creation_id(0);
    let reference = invalid.setup.as_ref().unwrap().references[0].clone();
    invalid.setup.as_mut().unwrap().references.push(reference);
    assert!(invalid.message_text().is_err());
    assert_eq!(
        app.writing_request(&captured.request_message).unwrap(),
        captured
    );
}

#[test]
fn setup_size_boundary_is_exact_and_overflow_does_not_mutate_saved_configuration() {
    let (_root, mut app, note, thread) = fixture();
    let mut setup = configuration(&note.id, Some(&thread.id), "é", &[]);
    // Stored identity is canonical general scope, so size the actual setup bytes.
    setup.chat = None;
    let length = serde_json::to_vec(&setup).unwrap().len();
    setup
        .brief
        .as_mut()
        .unwrap()
        .push_str(&"x".repeat(MAX_WRITING_REQUEST_BYTES - length));
    assert_eq!(
        serde_json::to_vec(&setup).unwrap().len(),
        MAX_WRITING_REQUEST_BYTES
    );
    let receipt = app.set_writing_setup("at-boundary", None, &setup).unwrap();
    let saved = config_record(&receipt);
    assert_eq!(saved.data, RecordData::WritingSetup(setup.clone()));
    let before = app.records().unwrap();
    setup.brief.as_mut().unwrap().push('é');
    assert!(
        app.set_writing_setup("overflow", Some(saved.version), &setup)
            .is_err()
    );
    assert_eq!(app.records().unwrap(), before);
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
}

#[test]
fn first_capture_undo_archives_exact_records_after_conversation_inactivation_or_move() {
    for scope in ["general", "private", "project"] {
        for change in ["archive", "resolve", "move"] {
            let (root, mut app, note, ordinary) = fixture();
            let chat = if scope == "project" {
                let (p, c) = project(&mut app, "original-project");
                link(&mut app, &note, &p, &c);
                c
            } else {
                ordinary.id.clone()
            };
            if scope == "private" {
                let current = app.store.record(&chat).unwrap().unwrap();
                let mut data = current.data;
                if let RecordData::Thread(t) = &mut data {
                    t.private_context = true;
                }
                app.owner_change(
                    "existing-private",
                    &ChangeRequest {
                        reason: "Existing private host context".into(),
                        writes: vec![Put {
                            id: chat.clone(),
                            expected_version: Some(current.version),
                            archived: false,
                            data,
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            }
            let capture = snapshot(&mut app, "first", &note, &chat);
            let original = app
                .history()
                .unwrap()
                .into_iter()
                .find(|r| {
                    r.writes
                        .iter()
                        .any(|w| w.after.id == capture.request_message)
                })
                .unwrap();
            assert!(original.writes.iter().all(|w| w.before.is_none()));
            if change == "move" {
                let (p, _) = project(&mut app, "destination");
                app.project_command(
                    "move",
                    ProjectCommand::MoveChat {
                        chat: chat.clone(),
                        project: Some(p),
                    },
                )
                .unwrap();
            } else {
                let current = app.store.record(&chat).unwrap().unwrap();
                let mut data = current.data;
                if let RecordData::Thread(t) = &mut data
                    && change == "resolve"
                {
                    t.state = ThreadState::Resolved;
                }
                app.owner_change(
                    "inactive",
                    &ChangeRequest {
                        reason: "Conversation is no longer active".into(),
                        writes: vec![Put {
                            id: chat.clone(),
                            expected_version: Some(current.version),
                            archived: change == "archive",
                            data,
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            }
            let thread_after = app.store.record(&chat).unwrap().unwrap();
            let (compensation, outcome) = app
                .undo_with_key("undo-first", original.operation.as_str())
                .unwrap();
            let ApplyOutcome::Applied(undo) = &outcome else {
                panic!("{scope}/{change}: {outcome:?}")
            };
            assert_eq!(undo.writes.len(), original.writes.len());
            for written in &original.writes {
                let retained = app.store.record(&written.after.id).unwrap().unwrap();
                assert!(retained.archived);
                assert_eq!(retained.data, written.after.data);
                assert_eq!(retained.version, written.after.version + 1);
            }
            assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
            assert_eq!(app.store.record(&chat).unwrap().unwrap(), thread_after);
            assert_eq!(
                app.store.receipt(&original.operation).unwrap(),
                Some(original.clone())
            );
            let prepared = app.store.prepared(&compensation).unwrap();
            drop(app);
            let mut reopened = Workspace::open(root.path().join("data")).unwrap();
            assert_eq!(
                reopened
                    .undo_with_key("undo-first", original.operation.as_str())
                    .unwrap(),
                (compensation.clone(), outcome)
            );
            assert_eq!(reopened.store.prepared(&compensation).unwrap(), prepared);
        }
    }
}

#[test]
fn project_setup_undo_fails_closed_after_conversation_move_or_working_link_removal() {
    for change in ["move", "unlink"] {
        let (_root, mut app, note, _) = fixture();
        let (p, c) = project(&mut app, "knowledge-off-undo");
        let current = app.store.record(&p).unwrap().unwrap();
        let RecordData::Project(data) = current.data else {
            panic!()
        };
        app.project_command(
            "knowledge-off",
            ProjectCommand::Configure {
                project: p.clone(),
                version: current.version,
                title: data.title,
                instructions: data.instructions,
                use_brn_knowledge: false,
            },
        )
        .unwrap();
        app.link_writing_thread("working-link", &note.id, note.version, &c)
            .unwrap();
        let first = app
            .set_writing_setup(
                "first",
                None,
                &configuration(&note.id, Some(&c), "Before", &[]),
            )
            .unwrap();
        let setup = config_record(&first);
        let edit = app
            .set_writing_setup(
                "edit",
                Some(setup.version),
                &configuration(&note.id, Some(&c), "After", &[]),
            )
            .unwrap();
        let before = config_record(&edit);
        if change == "move" {
            let (other, _) = project(&mut app, "other");
            app.project_command(
                "move",
                ProjectCommand::MoveChat {
                    chat: c.clone(),
                    project: Some(other),
                },
            )
            .unwrap();
        } else {
            let link = app
                .records()
                .unwrap()
                .into_iter()
                .find(|r| matches!(&r.data, RecordData::Link(l) if l.from == c && l.to == note.id))
                .unwrap();
            app.owner_change(
                "unlink",
                &ChangeRequest {
                    reason: "Remove actual conversation access".into(),
                    writes: vec![Put {
                        id: link.id,
                        expected_version: Some(link.version),
                        archived: true,
                        data: link.data,
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        }
        if change == "unlink" {
            assert!(app.writing_setup(&note.id, Some(&c)).is_err());
            assert!(
                !app.store
                    .records_for_chat(Some(&c))
                    .unwrap()
                    .iter()
                    .any(|r| r.id == before.id)
            );
        }
        let outcome = app.undo_with_key("must-not-revive", edit.operation.as_str());
        assert!(
            !matches!(outcome, Ok((_, ApplyOutcome::Applied(_)))),
            "{change}"
        );
        assert_eq!(app.store.record(&before.id).unwrap().unwrap(), before);
        assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
    }
}

#[test]
fn capture_after_undo_reuses_default_setup_identity_at_its_actual_new_version() {
    let (_root, mut app, note, thread) = fixture();
    let first = snapshot(&mut app, "first", &note, &thread.id);
    let original = app
        .history()
        .unwrap()
        .into_iter()
        .find(|r| r.writes.iter().any(|w| w.after.id == first.request_message))
        .unwrap();
    assert!(matches!(
        app.undo(original.operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let second = snapshot(&mut app, "second", &note, &thread.id);
    let before = first.setup.unwrap();
    let after = second.setup.as_ref().unwrap();
    assert_eq!(after.input.record, before.input.record);
    assert_eq!(after.input.version, before.input.version + 2);
    assert_eq!(
        app.store
            .record(&after.input.record)
            .unwrap()
            .unwrap()
            .version,
        after.input.version
    );
    assert_eq!(
        app.writing_request(&second.request_message).unwrap(),
        second
    );
    assert_eq!(snapshot(&mut app, "second", &note, &thread.id), second);
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
}

#[test]
fn delayed_real_capture_prepare_rejects_changed_note_or_existing_setup_before_replay() {
    for existing_setup in [false, true] {
        let (root, mut app, note, thread) = fixture();
        if existing_setup {
            app.set_writing_setup(
                "existing",
                None,
                &configuration(&note.id, None, "Before", &[]),
            )
            .unwrap();
        }
        let backup = root.path().join("before-capture.sqlite3");
        app.store.backup(&backup).unwrap();
        let captured = snapshot(&mut app, "shape", &note, &thread.id);
        let receipt = app
            .history()
            .unwrap()
            .into_iter()
            .find(|r| {
                r.writes
                    .iter()
                    .any(|w| w.after.id == captured.request_message)
            })
            .unwrap();
        let mut request = app.store.prepared(&receipt.operation).unwrap().request;
        drop(app);
        let restored = root.path().join("delayed-capture");
        drop(Store::restore(&backup, &restored).unwrap());
        let mut app = Workspace::open(&restored).unwrap();
        let binding = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(
                    "request",
                    (&note.id, note.version, &thread.id, Vec::<String>::new())
                ))
                .unwrap()
            )
        );
        let op = app
            .store
            .allocate_bound_operation("writing-request:pending", &binding)
            .unwrap();
        let mut pending = captured.clone();
        pending.request_message = op.creation_id(0);
        if !existing_setup {
            pending.setup.as_mut().unwrap().input.record = op.creation_id(1);
        }
        for write in &mut request.writes {
            match &mut write.data {
                RecordData::Message(m) => {
                    write.id = pending.request_message.clone();
                    m.text = pending.message_text().unwrap();
                }
                RecordData::WritingSetup(_) => {
                    write.id = op.creation_id(1);
                }
                _ => panic!(),
            }
        }
        app.store
            .prepare_owner(&op, &request, &HostAuthority::owner("delayed host capture"))
            .unwrap();
        let prepared = app.store.prepared(&op).unwrap();
        if existing_setup {
            let current = pending.setup.as_ref().unwrap();
            app.set_writing_setup(
                "changed",
                Some(current.input.version),
                &configuration(&note.id, None, "After", &[]),
            )
            .unwrap();
        } else {
            let mut data = note.data.clone();
            if let RecordData::Note(n) = &mut data {
                n.markdown = "Changed after prepare".into();
            }
            app.owner_change(
                "changed-note",
                &ChangeRequest {
                    reason: "Saved Note changed after capture prepare".into(),
                    writes: vec![Put {
                        id: note.id.clone(),
                        expected_version: Some(note.version),
                        archived: false,
                        data,
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        }
        assert!(
            app.snapshot_writing_request("pending", &note.id, note.version, &thread.id, &[])
                .is_err()
        );
        assert!(app.store.receipt(&op).unwrap().is_none());
        assert!(
            app.store
                .record(&pending.request_message)
                .unwrap()
                .is_none()
        );
        assert_eq!(app.store.prepared(&op).unwrap(), prepared);
    }
}

#[test]
fn live_canonical_setup_undo_does_not_need_its_archived_conversation() {
    for project_scope in [false, true] {
        let (_root, mut app, note, ordinary) = fixture();
        let chat = if project_scope {
            let (p, c) = project(&mut app, "shared-working-note");
            link(&mut app, &note, &p, &c);
            c
        } else {
            ordinary.id
        };
        let first = app
            .set_writing_setup(
                "before",
                None,
                &configuration(&note.id, Some(&chat), "Before", &[]),
            )
            .unwrap();
        let saved = config_record(&first);
        let edit = app
            .set_writing_setup(
                "after",
                Some(saved.version),
                &configuration(&note.id, Some(&chat), "After", &[]),
            )
            .unwrap();
        let current = app.store.record(&chat).unwrap().unwrap();
        app.owner_change(
            "archive",
            &ChangeRequest {
                reason: "Archive only the conversation".into(),
                writes: vec![Put {
                    id: chat.clone(),
                    expected_version: Some(current.version),
                    archived: true,
                    data: current.data,
                }],
                inputs: vec![],
            },
        )
        .unwrap();
        let (_, outcome) = app
            .undo_with_key("restore-setup", edit.operation.as_str())
            .unwrap();
        assert!(matches!(outcome, ApplyOutcome::Applied(_)));
        let restored = app.store.record(&saved.id).unwrap().unwrap();
        assert!(!restored.archived);
        assert_eq!(restored.data, saved.data);
        assert_eq!(
            app.undo_with_key("restore-setup", edit.operation.as_str())
                .unwrap()
                .1,
            outcome
        );
        assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
        assert!(app.store.record(&chat).unwrap().unwrap().archived);
    }
}
