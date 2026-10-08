//! Real AppWorker/provider callbacks over approved synthetic Sources.
use super::*;
#[path = "intake.rs"]
mod intake_tests;
use crate::inbox_actions::InboxAnalysisPurpose;
use brn_ai::{KnowledgeProposalArgs, KnowledgeQuoteArgs};
use brn_store::{note_identity, note_provenance};

#[derive(Clone)]
enum Step {
    Knowledge(KnowledgeProposalArgs),
    Action(ActionProposalArgs),
    Conflict(brn_ai::ConflictArgs),
}
fn semantic(source: &SourceFixture) -> InboxActionRequest {
    let mut r = request(source);
    r.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    r
}
fn knowledge(_source: &SourceFixture) -> KnowledgeProposalArgs {
    KnowledgeProposalArgs {
        supersedes: None,
        title: "Reviewed color knowledge".into(),
        path: "color.md".into(),
        source_paths: vec![],
        text: "# Color decision\r\n\r\nThe team chose Blue õ 🦀.\r\n".into(),
        quotes: vec![KnowledgeQuoteArgs {
            source_id: None,
            quote: "Blue õ 🦀".into(),
            occurrence: None,
        }],
    }
}

#[path = "conflicts.rs"]
mod conflict_tests;
#[path = "knowledge_links.rs"]
mod link_tests;
#[path = "supersession_replay.rs"]
mod supersession_replay_tests;
#[path = "supersession.rs"]
mod supersession_tests;
fn scripted(script: Arc<Mutex<Vec<Step>>>) -> ProposalAnswerHook {
    Arc::new(move |ask, _, _, proposals, _, _| {
        let steps = std::mem::take(&mut *script.lock().unwrap());
        Box::pin(async move {
            let results = tokio::task::spawn_blocking(move || {
                assert_eq!(
                    proposals.knowledge_enabled(),
                    ask.question.contains("propose_knowledge")
                );
                steps
                    .into_iter()
                    .map(|s| {
                        tool_reply(match s {
                            Step::Knowledge(k) => proposals.propose_knowledge(k),
                            Step::Action(a) => proposals.propose_actions(a),
                            Step::Conflict(c) => proposals.report_conflict(c),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&results).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    })
}
fn approved(
    worker: &AppWorker,
    record: &crate::proposals::ProposalRecord,
) -> crate::proposal_apply::ApplyReceipt {
    let r = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    let AppEvent::ProposalApplied(receipt) =
        reply_at(worker, r.operation_id, AppCommand::ApproveProposal(r))
    else {
        panic!("approval response");
    };
    receipt
}
fn results(turn: &WorkTurn) -> Vec<Value> {
    serde_json::from_str(&turn.answer).unwrap()
}

#[test]
fn inbox_knowledge_mixed_reviews_preserve_exact_quotes_identity_current_and_separate_approval() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Original exact: Blue õ 🦀\r\nCall Anna.\r\n");
    let input = knowledge(&source);
    let action = args();
    *script.lock().unwrap() = vec![Step::Knowledge(input.clone()), Step::Action(action.clone())];
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Completed);
    assert!(results(&turn).iter().all(|x| x.get("ok").is_some()));
    let all = analysis(&w, r.id);
    assert_eq!(all.proposals.len(), 2);
    assert!(all.needs_semantic_review);
    let record = all
        .proposals
        .iter()
        .find(|p| p.draft.inbox_knowledge.is_some())
        .unwrap();
    assert_eq!(record.draft.group_id, Some(r.id));
    assert_eq!(record.draft.session_id, Some(turn.conversation_id));
    let binding = record.draft.inbox_knowledge.as_ref().unwrap();
    assert_eq!(binding.source, Some(source.source.source.clone()));
    let note_id = binding.note_id;
    assert_ne!(note_id, record.draft.id);
    assert_ne!(note_id, source.note_id);
    assert_eq!(note_id.get_version_num(), 8);
    assert_eq!(record.draft.id.get_version_num(), 8);
    let citation = &binding.citations[0];
    assert_eq!(citation.note_id, source.note_id);
    assert_eq!(citation.quote, "Blue õ 🦀");
    assert_eq!(citation.sha256, source.source.source.fingerprint.sha256);
    let text = record.draft.changes[0].text().unwrap();
    assert!(text.ends_with(&input.text));
    assert_eq!(note_identity::read(text).unwrap(), Some(note_id));
    assert!(!f.base.path().join("vault/color.md").exists());
    no_actions(&w);
    let mut invalid = text.to_string();
    invalid = note_provenance::write(&invalid, &[]).unwrap();
    let edit = ProposalEdit {
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: vec![Some(invalid)],
        action_data: vec![],
    };
    assert!(matches!(
        reply(&w, AppCommand::EditProposal(edit)),
        AppEvent::Failed(_)
    ));
    let knowledge_receipt = approved(&w, record);
    assert_eq!(knowledge_receipt.outcome, ApplyOutcome::Applied);
    no_actions(&w);
    let AppEvent::NoteProvenance(provenance) =
        reply(&w, AppCommand::NoteProvenance(input.path.clone()))
    else {
        panic!("provenance");
    };
    assert_eq!(
        provenance.citations[0].outcome,
        crate::knowledge::CitationOutcome::Matched
    );
    let AppEvent::Notes(page) = reply(
        &w,
        AppCommand::ScopedNotes {
            scope: crate::library::KnowledgeScope::Current,
            folder: None,
            cursor: None,
        },
    ) else {
        panic!("Current list");
    };
    assert!(page.notes.iter().any(|x| x.path == input.path));
    assert!(!page.notes.iter().any(|x| x.path == "source.md"));
    let action_record = all
        .proposals
        .iter()
        .find(|p| !p.draft.action_changes.is_empty())
        .unwrap();
    assert_eq!(approved(&w, action_record).outcome, ApplyOutcome::Applied);
    let undo = crate::proposal_apply::UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: knowledge_receipt.operation_id,
        trash_member: None,
    };
    let AppEvent::ProposalApplied(undone) =
        reply_at(&w, undo.operation_id, AppCommand::UndoProposal(undo))
    else {
        panic!("undo apply");
    };
    assert_eq!(undone.outcome, ApplyOutcome::Applied);
    assert!(!f.base.path().join("vault/color.md").exists());
    assert_eq!(
        std::fs::read(f.base.path().join("vault/source.md")).unwrap(),
        source.source.text.as_bytes()
    );
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn inbox_knowledge_invalid_candidates_and_action_only_scope_never_admit_knowledge() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let base = knowledge(&source);
    let mut invalid = vec![];
    let mut k = base.clone();
    k.text = note_identity::assign(&k.text, source.note_id).unwrap();
    invalid.push(k);
    let mut k = base.clone();
    k.path = "archive/color.md".into();
    invalid.push(k);
    let mut k = base.clone();
    k.path = "a.md".into();
    invalid.push(k);
    for text in [
        "---\nbrn_state: history\n---\nold",
        "---\nbrn_kind: source\n---\nSource",
        "---\nbrn_id: bad\n---\nWrong",
        "---\nbrn_id: 5b344a65-e247-4b2c-9941-c4b52c405bdb\n---\nForged managed identity",
    ] {
        let mut k = base.clone();
        k.text = text.into();
        invalid.push(k);
    }
    let mut k = base.clone();
    k.quotes[0].quote = "Blue õ 🦀 with fabricated wording".into();
    invalid.push(k);
    let mut k = base.clone();
    k.quotes[0].occurrence = Some(2);
    invalid.push(k);
    let mut k = base.clone();
    k.quotes.push(k.quotes[0].clone());
    invalid.push(k);
    let mut k = base.clone();
    k.quotes.clear();
    invalid.push(k);
    *script.lock().unwrap() = invalid.into_iter().map(Step::Knowledge).collect();
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    assert!(results(&turn).iter().all(|x| x.get("error").is_some()));
    assert!(analysis(&w, r.id).proposals.is_empty());
    *script.lock().unwrap() = vec![Step::Knowledge(base.clone())];
    let action_only = request(&source);
    let turn = analyze(&w, &action_only).unwrap();
    assert_eq!(results(&turn)[0]["error"], json!(AiErrorKind::ToolRejected));
    assert!(analysis(&w, action_only.id).proposals.is_empty());
    no_actions(&w);
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn inbox_knowledge_creation_replay_preserves_newer_review_after_source_loss_and_restart() {
    let f = Fixture::new();
    let input = Arc::new(Mutex::new(None::<KnowledgeProposalArgs>));
    let script = input.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let called = calls.clone();
    let (created, ready) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let resume = Arc::new(Mutex::new(resume));
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
        called.fetch_add(1, Ordering::SeqCst);
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
                let mut changed = k.clone();
                changed.text.push_str("changed");
                let changed = tool_reply(proposals.propose_knowledge(changed));
                let mut fresh = k;
                fresh.path = "fresh.md".into();
                vec![
                    same,
                    changed,
                    tool_reply(proposals.propose_knowledge(fresh)),
                ]
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
    let k = knowledge(&source);
    *input.lock().unwrap() = Some(k.clone());
    let mut r = semantic(&source);
    w.submit(r.id, AppCommand::AnalyzeInboxActions(Box::new(r.clone())))
        .unwrap();
    let first = ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let id = Uuid::parse_str(first["stamp"]["id"].as_str().unwrap()).unwrap();
    let AppEvent::Proposal(review) = reply(&w, AppCommand::Proposal(id)) else {
        panic!("review");
    };
    let changed = review.draft.changes[0].text().unwrap().to_string() + "Owner edit preserved\r\n";
    let edit = ProposalEdit {
        expected: review.stamp(),
        title: "Owner's newer review".into(),
        texts: vec![Some(changed.clone())],
        action_data: vec![],
    };
    let AppEvent::Proposal(edited) = reply(&w, AppCommand::EditProposal(edit)) else {
        panic!("edit");
    };
    std::fs::remove_file(f.base.path().join("vault/source.md")).unwrap();
    release.send(()).unwrap();
    let turn = finish(&w, &r).unwrap();
    let out = results(&turn);
    assert_eq!(out[0]["ok"]["stamp"], json!(edited.stamp()));
    assert_ne!(first["stamp"], out[0]["ok"]["stamp"]);
    assert!(out[1].get("error").is_some());
    assert!(out[2].get("error").is_some());
    assert_eq!(
        analysis(&w, r.id).proposals[0].draft.changes[0].text(),
        Some(changed.as_str())
    );
    let capture = analysis(&w, r.id).job;
    w.shutdown().unwrap();
    let mut w = f.start(Hooks::default());
    r.generation += 1;
    assert_eq!(json!(analyze(&w, &r).unwrap()), json!(turn));
    let got = analysis(&w, r.id);
    assert_eq!(got.job, capture);
    assert_eq!(got.proposals[0].stamp(), edited.stamp());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    r.purpose = InboxAnalysisPurpose::Actions;
    assert_eq!(
        analyze(&w, &r).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn inbox_knowledge_fresh_approval_refuses_changed_source_or_new_identity_collision() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let k = knowledge(&source);
    *script.lock().unwrap() = vec![Step::Knowledge(k.clone())];
    let r = semantic(&source);
    analyze(&w, &r).unwrap();
    let record = analysis(&w, r.id).proposals[0].clone();
    for identity_collision in [true, false] {
        if identity_collision {
            std::fs::write(
                f.base.path().join("vault/collision.md"),
                note_identity::assign(
                    "Other",
                    record.draft.inbox_knowledge.as_ref().unwrap().note_id,
                )
                .unwrap(),
            )
            .unwrap();
        } else {
            std::fs::remove_file(f.base.path().join("vault/collision.md")).unwrap();
            std::fs::write(
                f.base.path().join("vault/source.md"),
                source.source.text.clone() + "changed",
            )
            .unwrap();
        }
        let a = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        };
        assert!(matches!(
            reply_at(&w, a.operation_id, AppCommand::ApproveProposal(a)),
            AppEvent::Failed(_)
        ));
        assert!(!f.base.path().join("vault/color.md").exists());
        assert_eq!(analysis(&w, r.id).proposals[0].stamp(), record.stamp());
    }
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn inbox_knowledge_and_actions_share_twenty_draft_cap_and_creation_replay() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let first = knowledge(&source);
    let mut steps = vec![Step::Knowledge(first.clone())];
    for i in 1..20 {
        if i % 2 == 0 {
            let mut k = knowledge(&source);
            k.path = format!("color-{i}.md");
            steps.push(Step::Knowledge(k));
        } else {
            let mut action = args();
            action.title = format!("Review Inbox consequence {i}");
            steps.push(Step::Action(action));
        }
    }
    let mut extra = knowledge(&source);
    extra.path = "twenty-first.md".into();
    steps.push(Step::Knowledge(extra));
    steps.push(Step::Knowledge(first));
    *script.lock().unwrap() = steps;
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    let out = results(&turn);
    assert!(out[..20].iter().all(|x| x.get("ok").is_some()));
    assert_eq!(out[20]["error"], json!(AiErrorKind::ToolRejected));
    assert_eq!(out[21], out[0]);
    assert_eq!(analysis(&w, r.id).proposals.len(), 20);
    assert!(!f.base.path().join("vault/color.md").exists());
    no_actions(&w);
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn inbox_knowledge_cancelled_turn_waits_for_lease_refuses_new_drafts_and_restarts_interrupted() {
    let f = Fixture::new();
    let (sent, ready) = mpsc::channel();
    let (noticed, cancelled) = mpsc::channel();
    let hook: ProposalAnswerHook = Arc::new(move |_, _, reads, proposals, cancel, _| {
        drop(reads);
        sent.send(proposals).unwrap();
        let noticed = noticed.clone();
        Box::pin(async move {
            cancel.cancelled().await;
            noticed.send(()).unwrap();
            AiAnswer {
                text: "Stopped; semantic work is incomplete".into(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let mut w = f.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\n");
    let k = knowledge(&source);
    let r = semantic(&source);
    w.submit(r.id, AppCommand::AnalyzeInboxActions(Box::new(r.clone())))
        .unwrap();
    let lease = ready.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(lease.knowledge_enabled());
    assert!(matches!(
        reply(&w, AppCommand::CancelTurn(r.id)),
        AppEvent::TurnCancelRequested { accepted: true, .. }
    ));
    cancelled.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(
        lease.propose_knowledge(k).unwrap_err().kind,
        AiErrorKind::ToolRejected
    );
    assert!(
        matches!(reply(&w,AppCommand::Turn(r.id)),AppEvent::Turn(Some(t)) if t.status==WorkTurnStatus::Running)
    );
    drop(lease);
    let turn = finish(&w, &r).unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Interrupted);
    assert!(analysis(&w, r.id).proposals.is_empty());
    w.shutdown().unwrap();
    let mut w = f.start(Hooks::default());
    assert_eq!(json!(analyze(&w, &r).unwrap()), json!(turn));
    assert!(analysis(&w, r.id).needs_semantic_review);
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn knowledge_quote_selection_refuses_metadata_ambiguity_and_invalid_occurrences_before_retention() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "\u{feff}Blue õ 🦀\r\nBlue õ 🦀\r\n");
    let base = knowledge(&source);
    let mut metadata = base.clone();
    metadata.quotes[0].quote = "brn_kind: source".into();
    let mut zero = base.clone();
    zero.quotes[0].occurrence = Some(0);
    let mut outside = base.clone();
    outside.quotes[0].occurrence = Some(3);
    let mut selected = base.clone();
    selected.quotes[0].occurrence = Some(2);
    *script.lock().unwrap() = vec![metadata, base, zero, outside, selected.clone()]
        .into_iter()
        .map(Step::Knowledge)
        .collect();
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    let out = results(&turn);
    for (index, kind) in [
        AiErrorKind::QuoteNotFound,
        AiErrorKind::QuoteAmbiguous,
        AiErrorKind::QuoteOccurrenceInvalid,
        AiErrorKind::QuoteOccurrenceInvalid,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(out[index]["error"], json!(kind));
    }
    assert!(out[4].get("ok").is_some(), "{}", turn.answer);
    let all = analysis(&w, r.id);
    assert_eq!(all.proposals.len(), 1);
    let citation = &all.proposals[0]
        .draft
        .inbox_knowledge
        .as_ref()
        .unwrap()
        .citations[0];
    assert_eq!(
        citation.start_byte,
        source.source.text.rfind("Blue õ 🦀").unwrap()
    );
    assert_eq!(
        source
            .source
            .text
            .get(citation.start_byte..citation.end_byte),
        Some(citation.quote.as_str())
    );
    no_actions(&w);
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}

#[test]
fn knowledge_exact_intent_replays_and_changed_intent_or_analysis_creates_distinct_review() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let mut w = f.start(Hooks {
        proposal_answer: Some(scripted(script.clone())),
        ..Hooks::default()
    });
    let source = capture_source(&w, "Blue õ 🦀\r\nBlue õ 🦀\r\n");
    let targets = [
        link_tests::target(&w, "person.md", false),
        link_tests::target(&w, "project.md", false),
    ];
    let previous = link_tests::target(&w, "previous.md", false);
    let alternate = link_tests::target(&w, "alternate.md", false);
    let mut base = knowledge(&source);
    base.quotes[0].occurrence = Some(1);
    base.source_paths = targets
        .iter()
        .map(|(_, saved)| saved.source.path.clone())
        .collect();
    let mut steps = vec![Step::Knowledge(base.clone()), Step::Knowledge(base.clone())];
    for mode in 0..8 {
        let mut changed = base.clone();
        match mode {
            0 => changed.title.push(' '),
            1 => changed.text = changed.text.replace("\r\n", "\n"),
            2 => changed.path = "another.md".into(),
            3 => changed.quotes[0].quote = "Blue".into(),
            4 => changed.quotes[0].occurrence = Some(2),
            5 => changed.supersedes = Some(previous.1.source.path.clone()),
            6 => changed.supersedes = Some(alternate.1.source.path.clone()),
            _ => changed.source_paths.swap(0, 1),
        }
        steps.push(Step::Knowledge(changed));
    }
    *script.lock().unwrap() = steps;
    let r = semantic(&source);
    let turn = analyze(&w, &r).unwrap();
    let out = results(&turn);
    assert!(out.iter().all(|v| v.get("ok").is_some()), "{}", turn.answer);
    assert_eq!(out[0], out[1]);
    let records = analysis(&w, r.id).proposals;
    assert_eq!(records.len(), 9);
    let mut ids = std::collections::HashSet::new();
    ids.insert(source.note_id);
    for record in &records {
        let binding = record.draft.inbox_knowledge.as_ref().unwrap();
        assert!(ids.insert(record.draft.id));
        assert!(ids.insert(binding.note_id));
        assert_eq!(binding.analysis_id, r.id);
        assert_eq!(record.draft.sources[0], source.source.source);
    }
    *script.lock().unwrap() = vec![Step::Knowledge(base)];
    let other = semantic(&source);
    let turn = analyze(&w, &other).unwrap();
    assert!(results(&turn)[0].get("ok").is_some(), "{}", turn.answer);
    let record = analysis(&w, other.id).proposals.remove(0);
    assert!(ids.insert(record.draft.id));
    assert!(ids.insert(record.draft.inbox_knowledge.as_ref().unwrap().note_id));
    assert!(!f.base.path().join("vault/color.md").exists());
    no_actions(&w);
    retained_original(&w, &source);
    no_credentials(&f);
    w.shutdown().unwrap();
}
