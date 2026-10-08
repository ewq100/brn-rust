//! Real worker callbacks and exact private-evidence structural revisions.
use super::*;
use crate::proposals::{CreateRenameRequest, KnowledgePredecessorRequest, NoteChange};

#[test]
fn repeated_knowledge_callback_returns_current_renamed_path_without_duplicate_or_inference() {
    let f = Fixture::new();
    let input = Arc::new(Mutex::new(None::<KnowledgeProposalArgs>));
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release = Arc::new(Mutex::new(release_rx));
    let hook_input = input.clone();
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
        let args = hook_input.lock().unwrap().clone().unwrap();
        let ready = ready_tx.clone();
        let release = release.clone();
        Box::pin(async move {
            let results = tokio::task::spawn_blocking(move || {
                let first = proposals.propose_knowledge(args.clone()).unwrap();
                ready.send(first.clone()).unwrap();
                release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
                let second = proposals.propose_knowledge(args).unwrap();
                vec![first, second]
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&results).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut w = f.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let args = knowledge(&source);
    *input.lock().unwrap() = Some(args.clone());
    let request = semantic(&source);
    w.submit(
        request.id,
        AppCommand::AnalyzeInboxActions(Box::new(request.clone())),
    )
    .unwrap();
    let first: Value = ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let stamp: crate::proposals::ProposalStamp =
        serde_json::from_value(first["stamp"].clone()).unwrap();
    let AppEvent::Proposal(before) = reply(&w, AppCommand::Proposal(stamp.id)) else {
        panic!("first review")
    };
    let AppEvent::Proposal(renamed) = reply(
        &w,
        AppCommand::RenameProposalCreate(CreateRenameRequest {
            expected: stamp,
            change_index: 0,
            path: "reviewed-color.md".into(),
        }),
    ) else {
        panic!("rename between callbacks")
    };
    crate::proposals::validate_create_rename_transition(&before, &renamed, 0, "reviewed-color.md")
        .unwrap();
    release_tx.send(()).unwrap();
    let turn = finish(&w, &request).unwrap();
    let replies: Vec<Value> = serde_json::from_str(&turn.answer).unwrap();
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0], first);
    assert_eq!(replies[1]["stamp"], json!(renamed.stamp()));
    assert_eq!(replies[1]["path"], "reviewed-color.md");
    assert_eq!(replies[1]["note_id"], first["note_id"]);
    assert_eq!(analysis(&w, request.id).proposals, vec![renamed.clone()]);
    assert_eq!(
        renamed.draft.changes[0].text(),
        before.draft.changes[0].text()
    );
    retained_original(&w, &source);
    w.shutdown().unwrap();
    std::fs::remove_file(f.base.path().join("vault/source.md")).unwrap();
    let mut app = crate::app::App::open(&f.base.path().join("data"), f.config()).unwrap();
    let job = app.store.inbox_action(request.id).unwrap().unwrap();
    let original = app
        .prepare_inbox_knowledge(&job, &args, turn.conversation_id)
        .unwrap();
    assert_eq!(app.create_proposal(&original).unwrap(), renamed);
    assert!(!f.base.path().join("vault/color.md").exists());
    assert!(!f.base.path().join("vault/reviewed-color.md").exists());
    no_credentials(&f);
}

#[test]
fn private_knowledge_rename_preserves_pending_source_and_protected_history_then_applies_after_restart()
 {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let (preview,source)=super::intake_tests::intake_at(&w,
        b"From: owner@example.test\r\nSubject: Rename evidence\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nBlue was selected.\r\n",
        "rename.eml","private-source.md");
    let (_, previous) = super::link_tests::target(&w, "previous.md", false);
    let AppEvent::InboxIntakeBinding(intake) = reply(
        &w,
        AppCommand::InboxIntakeBinding {
            source_proposal_id: source.draft.id,
        },
    ) else {
        panic!("intake binding")
    };
    let args = KnowledgeProposalArgs {
        supersedes: None,
        title: "Owner selection".into(),
        path: "selection.md".into(),
        source_paths: vec!["previous.md".into()],
        text: "# Selection\n\nBlue was selected.\n".into(),
        quotes: vec![KnowledgeQuoteArgs {
            source_id: Some(preview.extraction.as_ref().unwrap().sources[0].id.clone()),
            quote: "Blue was selected.".into(),
            occurrence: None,
        }],
    };
    *script.lock().unwrap() = vec![Step::Knowledge(args.clone())];
    let request = InboxActionRequest {
        budget: None,
        visual_asset: None,
        purpose: InboxAnalysisPurpose::KnowledgeAndActions,
        id: Uuid::new_v4(),
        conversation: None,
        source: None,
        intake: Some(*intake),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6-luna".into(),
        },
        effort: crate::ReasoningEffort::Medium,
        generation: 902,
    };
    let turn = analyze(&w, &request).unwrap();
    let original = analysis(&w, request.id).proposals.remove(0);
    let AppEvent::Proposal(attached) = reply(
        &w,
        AppCommand::AttachInboxKnowledgePredecessor(KnowledgePredecessorRequest {
            expected: original.stamp(),
            predecessor_path: "previous.md".into(),
        }),
    ) else {
        panic!("attachment")
    };
    let rename_event = reply(
        &w,
        AppCommand::RenameProposalCreate(CreateRenameRequest {
            expected: attached.stamp(),
            change_index: 0,
            path: "owner-selection.md".into(),
        }),
    );
    let renamed = match rename_event {
        AppEvent::Proposal(record) => record,
        AppEvent::Failed(error) => panic!("rename pending-intake Knowledge: {}", error.message),
        _ => panic!("unexpected rename pending-intake response"),
    };
    crate::proposals::validate_create_rename_transition(
        &attached,
        &renamed,
        0,
        "owner-selection.md",
    )
    .unwrap();
    assert_eq!(renamed.draft.changes[1], attached.draft.changes[1]);
    assert_eq!(
        renamed.draft.inbox_knowledge,
        attached.draft.inbox_knowledge
    );
    assert_eq!(renamed.draft.sources, attached.draft.sources);
    assert!(matches!(
        reply(
            &w,
            AppCommand::RenameProposalCreate(CreateRenameRequest {
                expected: source.stamp(),
                change_index: 0,
                path: "renamed-source.md".into()
            })
        ),
        AppEvent::Failed(_)
    ));
    assert!(matches!(
        reply(
            &w,
            AppCommand::ApproveProposal(ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: renamed.stamp()
            })
        ),
        AppEvent::Failed(_)
    ));
    assert_eq!(approved(&w, &source).outcome, ApplyOutcome::Applied);
    w.shutdown().unwrap();
    let mut app = crate::app::App::open(&f.base.path().join("data"), f.config()).unwrap();
    let job = app.store.inbox_action(request.id).unwrap().unwrap();
    let creation = app
        .prepare_inbox_knowledge(&job, &args, turn.conversation_id)
        .unwrap();
    assert_eq!(app.create_proposal(&creation).unwrap(), renamed);
    let receipt = app
        .approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: renamed.stamp(),
        })
        .unwrap();
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    assert!(!f.base.path().join("vault/selection.md").exists());
    assert_eq!(
        std::fs::read(f.base.path().join("vault/owner-selection.md")).unwrap(),
        renamed.draft.changes[0].text().unwrap().as_bytes()
    );
    assert_eq!(
        std::fs::read(f.base.path().join("vault/previous.md")).unwrap(),
        brn_store::note_metadata::to_history(&previous.text)
            .unwrap()
            .as_bytes()
    );
    let source_text = std::fs::read(f.base.path().join("vault/private-source.md")).unwrap();
    drop(app);
    let mut app = crate::app::App::open(&f.base.path().join("data"), f.config()).unwrap();
    let replay = app.create_proposal(&creation).unwrap();
    assert_eq!(replay.state, ProposalState::Applied);
    assert_eq!(replay.draft, renamed.draft);
    assert_eq!(
        std::fs::read(f.base.path().join("vault/private-source.md")).unwrap(),
        source_text
    );
    assert!(matches!(
        replay.draft.changes.as_slice(),
        [NoteChange::Create { .. }, NoteChange::Replace { .. }]
    ));
    no_credentials(&f);
}
