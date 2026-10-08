//! Explicit owner structural revision, exact original replay and fresh approval.
use super::link_tests::{relationships, target};
use super::*;
use crate::{
    app::App,
    library::KnowledgeScope,
    proposals::{
        CommentRequest, CommentTarget, KnowledgePredecessorRequest, ReviewComment, TextAnchor,
    },
};

#[test]
fn owner_predecessor_attachment_preserves_review_replays_original_and_applies_after_restart() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let (previous_id, previous) = target(&w, "previous.md", false);
    let (_, extra) = target(&w, "extra.md", false);
    let mut k = knowledge(&source);
    k.source_paths = vec!["extra.md".into(), "previous.md".into()];
    *script.lock().unwrap() = vec![Step::Knowledge(k.clone())];
    let request = semantic(&source);
    let turn = analyze(&w, &request).unwrap();
    let original = analysis(&w, request.id).proposals.remove(0);
    let owner_text = original.draft.changes[0]
        .text()
        .unwrap()
        .replace("The team chose", "Owner reviewed: the team chose");
    let AppEvent::Proposal(edited) = reply(
        &w,
        AppCommand::EditProposal(ProposalEdit {
            expected: original.stamp(),
            title: "Owner replacement decision".into(),
            texts: vec![Some(owner_text.clone())],
            action_data: vec![],
        }),
    ) else {
        panic!("owner edit")
    };
    let start = owner_text.find("Blue õ 🦀").unwrap();
    let AppEvent::Proposal(commented) = reply(
        &w,
        AppCommand::AddProposalComment(CommentRequest {
            expected: edited.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Keep this reviewed wording".into(),
                target: CommentTarget::Text(TextAnchor {
                    change_index: 0,
                    start,
                    end: start + "Blue õ 🦀".len(),
                    quote: "Blue õ 🦀".into(),
                }),
            },
        }),
    ) else {
        panic!("anchored comment")
    };
    let AppEvent::Proposal(attached) = reply(
        &w,
        AppCommand::AttachInboxKnowledgePredecessor(KnowledgePredecessorRequest {
            expected: commented.stamp(),
            predecessor_path: "previous.md".into(),
        }),
    ) else {
        panic!("explicit attachment")
    };
    crate::proposals::validate_knowledge_predecessor_transition(&commented, &attached).unwrap();
    assert_eq!(attached.version, commented.version + 1);
    assert_eq!(attached.comments, commented.comments);
    assert!(
        attached.draft.changes[0]
            .text()
            .unwrap()
            .starts_with(&owner_text)
    );
    assert_eq!(
        attached.draft.sources,
        vec![
            source.source.source.clone(),
            previous.source.clone(),
            extra.source.clone()
        ]
    );
    assert_eq!(
        attached
            .draft
            .inbox_knowledge
            .as_ref()
            .unwrap()
            .supersedes
            .as_ref()
            .unwrap()
            .note_id,
        previous_id
    );
    assert_eq!(
        std::fs::read(f.base.path().join("vault/previous.md")).unwrap(),
        previous.text.as_bytes()
    );
    assert!(!f.base.path().join("vault/color.md").exists());
    assert!(matches!(
        reply(
            &w,
            AppCommand::ApproveProposal(ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: commented.stamp()
            })
        ),
        AppEvent::Failed(_)
    ));
    w.shutdown().unwrap();

    let mut app = App::open(&f.base.path().join("data"), f.config()).unwrap();
    let job = app.store.inbox_action(request.id).unwrap().unwrap();
    let creation = app
        .prepare_inbox_knowledge(&job, &k, turn.conversation_id)
        .unwrap();
    assert_eq!(app.create_proposal(&creation).unwrap(), attached);
    let further = app
        .edit_proposal(&ProposalEdit {
            expected: attached.stamp(),
            title: attached.draft.title.clone(),
            texts: vec![
                Some(
                    attached.draft.changes[0]
                        .text()
                        .unwrap()
                        .replace("Owner reviewed:", "Owner confirmed:"),
                ),
                attached.draft.changes[1].text().map(str::to_owned),
            ],
            action_data: vec![],
        })
        .unwrap();
    assert_eq!(app.create_proposal(&creation).unwrap(), further);
    let mut changed_creation = creation.clone();
    changed_creation.title.push_str(" changed original");
    assert!(app.create_proposal(&changed_creation).is_err());
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: further.stamp(),
    };
    let applied = app.approve_proposal(&approval).unwrap();
    assert_eq!(applied.outcome, ApplyOutcome::Applied);
    assert_eq!(
        std::fs::read(f.base.path().join("vault/previous.md")).unwrap(),
        brn_store::note_metadata::to_history(&previous.text)
            .unwrap()
            .as_bytes()
    );
    drop(app);
    std::fs::remove_file(f.base.path().join("data/index.sqlite")).unwrap();
    let mut w = f.start(Hooks::default());
    let edges = relationships(&w, KnowledgeScope::All).edges;
    let successor_id = attached.draft.inbox_knowledge.as_ref().unwrap().note_id;
    assert!(
        edges
            .iter()
            .any(|edge| edge.source.note_id == successor_id && edge.target.note_id == previous_id)
    );
    let AppEvent::ProposalApplied(replayed) = reply_at(
        &w,
        approval.operation_id,
        AppCommand::ApproveProposal(approval.clone()),
    ) else {
        panic!("approval replay")
    };
    assert_eq!(replayed, applied);
    retained_original(&w, &source);
    w.shutdown().unwrap();
    std::fs::remove_file(f.base.path().join("vault/source.md")).unwrap();
    let mut app = App::open(&f.base.path().join("data"), f.config()).unwrap();
    let replay = app
        .prepare_inbox_knowledge(&job, &k, turn.conversation_id)
        .unwrap();
    let current = app.create_proposal(&replay).unwrap();
    assert_eq!(current.state, ProposalState::Applied);
    assert_eq!(current.draft, further.draft);
    assert_eq!(app.approve_proposal(&approval).unwrap(), applied);
    no_credentials(&f);
}

#[test]
fn owner_predecessor_attachment_refuses_stale_invalid_and_unfinished_context_without_revision() {
    for mode in 0..12 {
        let f = Fixture::new();
        let script = Arc::new(Mutex::new(vec![]));
        let mut w = f.start(Hooks {
            proposal_answer: Some(scripted(script.clone())),
            ..Hooks::default()
        });
        let source = capture_source(&w, "Blue õ 🦀\r\n");
        let (_, previous) = target(&w, "previous.md", mode == 0);
        let mut k = knowledge(&source);
        k.source_paths = vec!["previous.md".into()];
        *script.lock().unwrap() = vec![Step::Knowledge(k)];
        let request = semantic(&source);
        analyze(&w, &request).unwrap();
        let mut record = analysis(&w, request.id).proposals.remove(0);
        let mut path = "previous.md".to_owned();
        match mode {
            0 => {}
            1 => path = "source.md".into(),
            2 => {
                std::fs::write(f.base.path().join("vault/unmanaged.md"), "No identity\n").unwrap();
                path = "unmanaged.md".into();
            }
            3 => std::fs::write(
                f.base.path().join("vault/source.md"),
                source.source.text.clone() + "Later source\n",
            )
            .unwrap(),
            4 => std::fs::write(
                f.base.path().join("vault/previous.md"),
                previous.text.clone() + "Later context\n",
            )
            .unwrap(),
            5 => std::fs::write(f.base.path().join("vault/alias.md"), &previous.text).unwrap(),
            6 => std::fs::write(
                f.base.path().join("vault/color.md"),
                "Occupied owner destination\n",
            )
            .unwrap(),
            7 => {
                let AppEvent::Proposal(edited) = reply(
                    &w,
                    AppCommand::EditProposal(ProposalEdit {
                        expected: record.stamp(),
                        title: record.draft.title.clone(),
                        texts: vec![Some(
                            record.draft.changes[0].text().unwrap().to_owned() + "\n```text\n",
                        )],
                        action_data: vec![],
                    }),
                ) else {
                    panic!("hidden footer setup")
                };
                record = edited;
            }
            8 => {
                let AppEvent::Editor(view) =
                    reply(&w, AppCommand::OpenEditor("previous.md".into()))
                else {
                    panic!("open dirty predecessor")
                };
                assert!(matches!(
                    reply(
                        &w,
                        AppCommand::RecoverEditor(crate::editor::EditRequest {
                            path: "previous.md".into(),
                            expected: view.record.stamp,
                            generation: 1,
                            text: view.record.text + "Owner unfinished change\n"
                        })
                    ),
                    AppEvent::EditorRecovered(_)
                ));
            }
            9 => path = "../escape.md".into(),
            10 => path = "color.md".into(),
            _ => {}
        }
        let before = std::fs::read(f.base.path().join("vault/previous.md")).unwrap();
        let mut expected = record.stamp();
        if mode == 11 {
            expected.version += 1;
        }
        let result = reply(
            &w,
            AppCommand::AttachInboxKnowledgePredecessor(KnowledgePredecessorRequest {
                expected,
                predecessor_path: path,
            }),
        );
        assert!(matches!(result, AppEvent::Failed(_)), "mode {mode}");
        let AppEvent::Proposal(unchanged) = reply(&w, AppCommand::Proposal(record.draft.id)) else {
            panic!("retained review")
        };
        assert_eq!(unchanged, record, "mode {mode}");
        assert_eq!(
            std::fs::read(f.base.path().join("vault/previous.md")).unwrap(),
            before
        );
        no_credentials(&f);
        w.shutdown().unwrap();
    }
}

#[test]
fn owner_predecessor_attachment_retains_pending_private_intake_prerequisite_and_citations() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let (preview, source) = super::intake_tests::intake_at(&w,
        b"From: owner@example.test\r\nSubject: Synthetic selection\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nBlue was selected.\r\n",
        "selection.eml", "private-source.md");
    let (_, previous) = target(&w, "previous.md", false);
    let AppEvent::InboxIntakeBinding(intake) = reply(
        &w,
        AppCommand::InboxIntakeBinding {
            source_proposal_id: source.draft.id,
        },
    ) else {
        panic!("intake binding")
    };
    let extraction = preview.extraction.as_ref().unwrap();
    *script.lock().unwrap() = vec![Step::Knowledge(KnowledgeProposalArgs {
        supersedes: None,
        title: "Owner selection review".into(),
        path: "selection.md".into(),
        source_paths: vec!["previous.md".into()],
        text: "# Selection\n\nBlue was selected.\n".into(),
        quotes: vec![KnowledgeQuoteArgs {
            source_id: Some(extraction.sources[0].id.clone()),
            quote: "Blue was selected.".into(),
            occurrence: None,
        }],
    })];
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
        generation: 901,
    };
    let turn = analyze(&w, &request).unwrap();
    assert!(results(&turn)[0].get("ok").is_some(), "{}", turn.answer);
    let original = analysis(&w, request.id).proposals.remove(0);
    let AppEvent::Proposal(attached) = reply(
        &w,
        AppCommand::AttachInboxKnowledgePredecessor(KnowledgePredecessorRequest {
            expected: original.stamp(),
            predecessor_path: "previous.md".into(),
        }),
    ) else {
        panic!("private attachment")
    };
    let binding = attached.draft.inbox_knowledge.as_ref().unwrap();
    assert_eq!(
        binding.intake,
        original.draft.inbox_knowledge.as_ref().unwrap().intake
    );
    assert_eq!(
        binding.intake_citations,
        original
            .draft
            .inbox_knowledge
            .as_ref()
            .unwrap()
            .intake_citations
    );
    assert_eq!(attached.draft.sources, vec![previous.source]);
    assert!(!f.base.path().join("vault/private-source.md").exists());
    assert!(matches!(
        reply(
            &w,
            AppCommand::ApproveProposal(ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: attached.stamp()
            })
        ),
        AppEvent::Failed(_)
    ));
    assert_eq!(approved(&w, &source).outcome, ApplyOutcome::Applied);
    w.shutdown().unwrap();
    let mut w = f.start(Hooks::default());
    assert_eq!(approved(&w, &attached).outcome, ApplyOutcome::Applied);
    assert_eq!(
        std::fs::read(f.base.path().join("vault/previous.md")).unwrap(),
        brn_store::note_metadata::to_history(&previous.text)
            .unwrap()
            .as_bytes()
    );
    assert_eq!(json!(analyze(&w, &request).unwrap()), json!(turn));
    no_credentials(&f);
    w.shutdown().unwrap();
}
