//! Full target proofs and ordinary approval/replay over actual owned workers.
use super::*;
use crate::{
    knowledge::{EdgeOrigin, NoteLinkOutcome, RelationshipRequest},
    library::KnowledgeScope,
    proposals::{DraftNoteChange, DraftRequest},
};

pub(super) fn target(worker: &AppWorker, path: &str, history: bool) -> (Uuid, ProposalSource) {
    let id = Uuid::new_v4();
    let request = DraftRequest {
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        action_changes: vec![],
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Explicit synthetic context".into(),
        changes: vec![DraftNoteChange::Create {
            path: path.into(),
            text: format!(
                "\u{feff}---\r\nbrn_id: {id}\r\nbrn_kind: knowledge\r\nbrn_state: {}\r\n---\r\n# Context õ\r\nKnown context.\r\n",
                if history { "history" } else { "current" }
            ),
        }],
        sources: vec![],
    };
    let AppEvent::Proposal(review) = reply(worker, AppCommand::CreateProposal(request)) else {
        panic!("target review")
    };
    assert_eq!(approved(worker, &review).outcome, ApplyOutcome::Applied);
    let AppEvent::ProposalSource(proof) =
        reply(worker, AppCommand::ProposalEvidenceSource(path.into()))
    else {
        panic!("target full proof")
    };
    proof.validate().unwrap();
    (id, *proof)
}

fn linked(source: &SourceFixture, targets: &[(Uuid, ProposalSource)]) -> KnowledgeProposalArgs {
    let mut k = knowledge(source);
    k.source_paths = targets.iter().map(|(_, p)| p.source.path.clone()).collect();
    k.text
        .push_str(&format!("\r\n[Source](brn://note/{})\r\n", source.note_id));
    for (id, _) in targets {
        k.text.push_str(&format!("[Context](brn://note/{id})\r\n"));
    }
    k
}

pub(super) fn relationships(
    worker: &AppWorker,
    scope: KnowledgeScope,
) -> crate::knowledge::RelationshipPage {
    let AppEvent::Relationships(page) = reply(
        worker,
        AppCommand::Relationships(RelationshipRequest {
            scope,
            offset: 0,
            limit: 200,
        }),
    ) else {
        panic!("relationships")
    };
    *page
}

#[test]
fn inbox_knowledge_links_bind_complete_named_targets_and_rebuild_with_explicit_history() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let targets = vec![
        target(&w, "project.md", false),
        target(&w, "previous.md", true),
    ];
    let k = linked(&source, &targets);
    *script.lock().unwrap() = vec![Step::Knowledge(k.clone())];
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    assert!(results(&turn)[0].get("ok").is_some(), "{}", turn.answer);
    let all = analysis(&w, r.id);
    assert!(all.needs_semantic_review);
    let record = &all.proposals[0];
    let expected = std::iter::once(source.source.source.clone())
        .chain(targets.iter().map(|(_, p)| p.source.clone()))
        .collect::<Vec<_>>();
    assert_eq!(record.draft.sources, expected);
    assert!(!f.base.path().join("vault/color.md").exists());
    assert_eq!(approved(&w, record).outcome, ApplyOutcome::Applied);
    let AppEvent::NoteLinks(links) = reply(&w, AppCommand::NoteLinks("color.md".into())) else {
        panic!("saved links")
    };
    assert_eq!(links.links.len(), 3);
    assert!(
        links
            .links
            .iter()
            .all(|l| l.outcome == NoteLinkOutcome::Resolved)
    );
    let id = record.draft.inbox_knowledge.as_ref().unwrap().note_id;
    let all_edges = relationships(&w, KnowledgeScope::All);
    assert_eq!(
        all_edges
            .edges
            .iter()
            .filter(|e| e.source.note_id == id && e.origin == EdgeOrigin::ExplicitLink)
            .count(),
        3
    );
    let current = relationships(&w, KnowledgeScope::Current);
    assert_eq!(
        current
            .edges
            .iter()
            .filter(|e| e.source.note_id == id && e.origin == EdgeOrigin::ExplicitLink)
            .count(),
        1
    );
    assert!(
        current
            .edges
            .iter()
            .all(|e| e.target.note_id != targets[1].0 && e.target.note_id != source.note_id)
    );
    retained_original(&w, &source);
    w.shutdown().unwrap();
    std::fs::remove_file(f.base.path().join("data/index.sqlite")).unwrap();
    let mut w = f.start(Hooks::default());
    assert_eq!(analysis(&w, r.id).proposals[0].draft.sources, expected);
    assert_eq!(
        relationships(&w, KnowledgeScope::All).edges,
        all_edges.edges
    );
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn inbox_knowledge_link_approval_refuses_changed_ambiguous_and_uncaptured_targets() {
    for mode in 0..4 {
        let f = Fixture::new();
        let script = Arc::new(Mutex::new(vec![]));
        let mut w = f.start(Hooks {
            proposal_answer: Some(scripted(script.clone())),
            ..Hooks::default()
        });
        let source = capture_source(&w, "Blue õ 🦀\r\n");
        let targets = vec![target(&w, "project.md", false)];
        let mut k = linked(&source, &targets);
        if mode == 3 {
            k.source_paths.clear();
        }
        *script.lock().unwrap() = vec![Step::Knowledge(k)];
        let r = semantic(&source);
        let turn = analyze(&w, &r).unwrap();
        assert!(results(&turn)[0].get("ok").is_some());
        let record = analysis(&w, r.id).proposals.remove(0);
        let path = f.base.path().join("vault/project.md");
        match mode {
            0 => std::fs::write(&path, targets[0].1.text.clone() + "Changed\r\n").unwrap(),
            1 => std::fs::remove_file(&path).unwrap(),
            2 => std::fs::write(f.base.path().join("vault/alias.md"), &targets[0].1.text).unwrap(),
            _ => {}
        }
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        };
        assert!(
            matches!(
                reply_at(
                    &w,
                    request.operation_id,
                    AppCommand::ApproveProposal(request)
                ),
                AppEvent::Failed(_)
            ),
            "mode{mode}"
        );
        assert!(!f.base.path().join("vault/color.md").exists());
        assert_eq!(analysis(&w, r.id).proposals[0].state, ProposalState::Draft);
        retained_original(&w, &source);
        no_actions(&w);
        no_credentials(&f);
        w.shutdown().unwrap();
    }
}

#[test]
fn inbox_knowledge_links_refuse_pending_targets_and_duplicate_or_escaping_paths() {
    for mode in 0..3 {
        let f = Fixture::new();
        let script = Arc::new(Mutex::new(vec![]));
        let mut w = f.start(Hooks {
            proposal_answer: Some(scripted(script.clone())),
            ..Hooks::default()
        });
        let source = capture_source(&w, "Blue õ 🦀\r\n");
        let pending_id = Uuid::new_v4();
        let pending = DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: vec![],
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Still unapproved".into(),
            changes: vec![DraftNoteChange::Create {
                path: "pending.md".into(),
                text: note_identity::assign("Pending context", pending_id).unwrap(),
            }],
            sources: vec![],
        };
        let AppEvent::Proposal(pending) = reply(&w, AppCommand::CreateProposal(pending)) else {
            panic!("pending target")
        };
        let mut k = knowledge(&source);
        k.text
            .push_str(&format!("\r\n[Context](brn://note/{pending_id})\r\n"));
        k.source_paths = vec![match mode {
            0 => "pending.md".into(),
            1 => source.source.source.path.clone(),
            _ => "../escape.md".into(),
        }];
        *script.lock().unwrap() = vec![Step::Knowledge(k)];
        let r = semantic(&source);
        assert!(results(&analyze(&w, &r).unwrap())[0].get("error").is_some());
        assert!(analysis(&w, r.id).proposals.is_empty());
        let AppEvent::Proposal(retained) = reply(&w, AppCommand::Proposal(pending.draft.id)) else {
            panic!("retained pending")
        };
        assert_eq!(retained.stamp(), pending.stamp());
        assert!(!f.base.path().join("vault/pending.md").exists());
        assert!(!f.base.path().join("vault/color.md").exists());
        retained_original(&w, &source);
        no_credentials(&f);
        w.shutdown().unwrap();
    }
}

#[test]
fn inbox_knowledge_link_creation_replay_preserves_original_target_order_without_files() {
    let f = Fixture::new();
    let input = Arc::new(Mutex::new(None::<KnowledgeProposalArgs>));
    let script = input.clone();
    let (created, ready) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let resume = Arc::new(Mutex::new(resume));
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
        let k = script.lock().unwrap().clone().unwrap();
        let created = created.clone();
        let resume = resume.clone();
        Box::pin(async move {
            let out = tokio::task::spawn_blocking(move || {
                created
                    .send(proposals.propose_knowledge(k.clone()).unwrap())
                    .unwrap();
                resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
                let same = tool_reply(proposals.propose_knowledge(k.clone()));
                let mut changed = k;
                changed.source_paths.reverse();
                vec![same, tool_reply(proposals.propose_knowledge(changed))]
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&out).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut w = f.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let targets = vec![
        target(&w, "project.md", false),
        target(&w, "person.md", false),
    ];
    let k = linked(&source, &targets);
    *input.lock().unwrap() = Some(k.clone());
    let r = semantic(&source);
    w.submit(r.id, AppCommand::AnalyzeInboxActions(Box::new(r.clone())))
        .unwrap();
    let receipt = ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let id = Uuid::parse_str(receipt["stamp"]["id"].as_str().unwrap()).unwrap();
    let AppEvent::Proposal(record) = reply(&w, AppCommand::Proposal(id)) else {
        panic!("review")
    };
    let changed = record.draft.changes[0].text().unwrap().to_owned() + "Owner edit\r\n";
    let edit = ProposalEdit {
        expected: record.stamp(),
        title: "Newer owner review".into(),
        texts: vec![Some(changed.clone())],
        action_data: vec![],
    };
    let AppEvent::Proposal(edited) = reply(&w, AppCommand::EditProposal(edit)) else {
        panic!("edit")
    };
    for (_, p) in &targets {
        std::fs::remove_file(f.base.path().join("vault").join(&p.source.path)).unwrap();
    }
    std::fs::remove_file(f.base.path().join("vault/source.md")).unwrap();
    release.send(()).unwrap();
    let turn = finish(&w, &r).unwrap();
    let out = results(&turn);
    assert_eq!(out[0]["ok"]["stamp"], json!(edited.stamp()));
    assert!(out[1].get("error").is_some());
    let got = analysis(&w, r.id).proposals.remove(0);
    assert_eq!(got.draft.sources, record.draft.sources);
    assert_eq!(got.draft.changes[0].text(), Some(changed.as_str()));
    w.shutdown().unwrap();
    let mut w = f.start(Hooks::default());
    assert_eq!(
        analysis(&w, r.id).proposals[0].draft.sources,
        record.draft.sources
    );
    assert_eq!(json!(analyze(&w, &r).unwrap()), json!(turn));
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}
