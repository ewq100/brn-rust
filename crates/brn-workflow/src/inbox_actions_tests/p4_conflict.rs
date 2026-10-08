//! Paired retained EML evidence, recommendation review and exact approved effects.
//! Scripted semantic outputs prove workflow mechanics, not model judgment quality.
use super::*;
use crate::{
    findings::{CloseFindingRequest, FindingEvidenceOutcome, FindingState, NoteConflictRequest},
    library::KnowledgeScope,
    proposal_apply::GroupApprovalRequest,
    proposals::{NoteChange, ProposalState},
};
use brn_ai::{ConflictArgs, ConflictQuote};
use brn_store::note_metadata;
use std::sync::atomic::{AtomicUsize, Ordering};

const APPROVED_EMAIL: &[u8] = include_bytes!("fixtures/north-quay/approved-release.eml");
const CARRIER_EMAIL: &[u8] = include_bytes!("fixtures/north-quay/carrier-change.eml");
const TARGET: &str = "The approved target is to ship 100 sensor units on 19 October 2026.";
const AUTHORITY: &str = "Only release coordinator Kaia may authorize a change to that commitment.";
const CANCELLED: &str =
    "Our 19 October 2026 collection slot for the 100 sensor units is cancelled.";
const OFFER: &str = "The earliest collection slot we can offer is 22 October 2026 at 13:00 +03:00.";
const VISIBILITY: &str = "Kaia is copied for visibility; we have not received her authorization to change the release commitment.";
const ESTONIAN: &str = "See teade kirjeldab veovõimalust ega ole luba muuta väljalaske tähtaega.";
const UNKNOWN: &str = "We have not reserved this later slot and cannot confirm another carrier's availability or price.";
const RECOMMENDATION: &str = "Prefer retaining the 19 October release commitment while Operations asks Kaia for a decision by 16 October 2026 at 16:00 +03:00. The carrier controls its collection availability, but Kaia alone authorizes a release change; copied visibility is not authorization. The 22 October 13:00 +03:00 offer is unreserved. Alternatives: ask an alternate carrier whether 19 October is feasible, with price and availability still unknown, or seek Kaia's explicit authorization for the later slot. Do not book or promise either alternative before approval.";

fn quote(text: &str) -> KnowledgeQuoteArgs {
    KnowledgeQuoteArgs {
        source_id: None,
        quote: text.into(),
        occurrence: None,
    }
}

fn saved_email(
    worker: &AppWorker,
    bytes: &[u8],
    name: &str,
    path: &str,
) -> (
    crate::inbox_processing::InboxConversionPreview,
    SourceFixture,
) {
    let (preview, record) = intake_tests::intake_at(worker, bytes, name, path);
    assert_eq!(preview.extraction.as_ref().unwrap().sources[0].bytes, bytes);
    assert_eq!(approved(worker, &record).outcome, ApplyOutcome::Applied);
    let AppEvent::ProposalSource(source) =
        reply(worker, AppCommand::ProposalEvidenceSource(path.into()))
    else {
        panic!("saved EML Source proof")
    };
    source.validate().unwrap();
    assert!(note_metadata::classify(&source.text).unwrap().source);
    let note_id = note_identity::read(&source.text).unwrap().unwrap();
    let fixture = SourceFixture {
        source: *source,
        note_id,
        original: preview.original.clone(),
        raw: String::from_utf8(bytes.to_vec()).unwrap(),
    };
    (preview, fixture)
}

fn retained_email(
    worker: &AppWorker,
    preview: &crate::inbox_processing::InboxConversionPreview,
    source: &SourceFixture,
    bytes: &[u8],
) {
    let AppEvent::InboxItem(original) =
        reply(worker, AppCommand::InboxItem(source.original.capture.id))
    else {
        panic!("retained original")
    };
    original.validate_receipt().unwrap();
    assert_eq!(original.item, source.original);
    assert!(
        matches!(original.original, InboxOriginal::AvailableBinary { byte_len, sha256 } if byte_len == bytes.len() as u64 && sha256 == source.original.capture.copy.sha256)
    );
    let AppEvent::InboxCandidate(retained) =
        reply(worker, AppCommand::InboxCandidate(preview.request.clone()))
    else {
        panic!("retained extraction")
    };
    assert_eq!(*retained, *preview);
    assert_eq!(
        retained.extraction.as_ref().unwrap().sources[0].bytes,
        bytes
    );
}

fn paths(worker: &AppWorker, scope: KnowledgeScope) -> Vec<String> {
    let AppEvent::Notes(page) = reply(
        worker,
        AppCommand::ScopedNotes {
            scope,
            folder: None,
            cursor: None,
        },
    ) else {
        panic!("scoped notes")
    };
    page.notes.into_iter().map(|note| note.path).collect()
}

fn follow_up(title: &str, description: &str, predecessor: Uuid) -> ActionProposalArgs {
    let mut input = args();
    input.title = title.into();
    input.source_paths = vec!["north-quay-approved.md".into()];
    let data = candidate_data_mut(&mut input);
    data.title = title.into();
    data.description = description.into();
    data.state = ActionCandidateState::Open;
    data.owner = Some("Operations".into());
    data.sources = vec![predecessor.to_string()];
    data.due_on = Some("2026-10-16".into());
    data.follow_up_on = None;
    input
}

#[test]
fn p4_saved_eml_conflict_recommendation_exact_review_order_and_restart_replay() {
    let f = Fixture::new();
    let script = Arc::new(Mutex::new(vec![]));
    let calls = Arc::new(AtomicUsize::new(0));
    let invoked = calls.clone();
    let scripted_answer = scripted(script.clone());
    let hook: ProposalAnswerHook = Arc::new(move |ask, history, tools, proposals, cancel, emit| {
        let call = invoked.fetch_add(1, Ordering::SeqCst);
        let scripted_answer = scripted_answer.clone();
        Box::pin(async move {
            if call == 1 {
                let evidence_tools = tools.clone();
                tokio::task::spawn_blocking(move || {
                    let current = evidence_tools.read_note("north-quay-approved.md").unwrap();
                    assert!(!current.facts.source && !current.facts.history);
                    assert!(current.text.contains(TARGET) && current.text.contains(AUTHORITY));
                    assert!(evidence_tools.read_note("carrier-source.md").is_err());
                    let source = evidence_tools
                        .read_note_scoped("carrier-source.md", ReadScope::Source)
                        .unwrap();
                    assert!(source.facts.source && !source.facts.history);
                    assert!(source.text.contains(ESTONIAN));
                })
                .await
                .unwrap();
            }
            let answer = scripted_answer(ask, history, tools, proposals, cancel, emit).await;
            let receipts: Value = serde_json::from_str(&answer.text).unwrap();
            AiAnswer { text: json!({"recommendation": if call == 1 { RECOMMENDATION } else { "Preserve the explicitly approved release commitment and its authorization rule." }, "receipts": receipts}).to_string(), terminal: answer.terminal }
        })
    });
    let mut worker = f.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let (approved_preview, old_source) = saved_email(
        &worker,
        APPROVED_EMAIL,
        "approved-release.eml",
        "approved-source.md",
    );
    *script.lock().unwrap() = vec![Step::Knowledge(KnowledgeProposalArgs {
        supersedes: None,
        title: "Approved North Quay shipment commitment".into(),
        path: "north-quay-approved.md".into(),
        text: format!("# North Quay shipment\n\n{TARGET}\n{AUTHORITY}\n"),
        quotes: vec![quote(TARGET), quote(AUTHORITY)],
        source_paths: vec![],
    })];
    let seed_request = semantic(&old_source);
    let seed_turn = analyze(&worker, &seed_request).unwrap();
    let seed_answer: Value = serde_json::from_str(&seed_turn.answer).unwrap();
    assert!(
        seed_answer["receipts"][0].get("ok").is_some(),
        "{seed_answer}"
    );
    let predecessor = analysis(&worker, seed_request.id).proposals.remove(0);
    assert_eq!(
        approved(&worker, &predecessor).outcome,
        ApplyOutcome::Applied
    );
    let predecessor_id = predecessor.draft.inbox_knowledge.as_ref().unwrap().note_id;
    let predecessor_bytes =
        std::fs::read(f.base.path().join("vault/north-quay-approved.md")).unwrap();
    let predecessor_text = String::from_utf8(predecessor_bytes.clone()).unwrap();
    let (carrier_preview, carrier_source) = saved_email(
        &worker,
        CARRIER_EMAIL,
        "carrier-change.eml",
        "carrier-source.md",
    );
    let source_bytes = ["approved-source.md", "carrier-source.md"].map(|path| {
        (
            path,
            std::fs::read(f.base.path().join("vault").join(path)).unwrap(),
        )
    });
    let successor = KnowledgeProposalArgs {
        supersedes: Some("north-quay-approved.md".into()),
        title: "North Quay commitment with carrier uncertainty".into(),
        path: "north-quay-current.md".into(),
        text: format!(
            "# North Quay shipment\n\n{TARGET}\n{AUTHORITY}\n\nCarrier evidence: {CANCELLED}\n{OFFER}\n{VISIBILITY}\n{ESTONIAN}\n{UNKNOWN}\n\nRecommendation: {RECOMMENDATION}\n"
        ),
        quotes: [CANCELLED, OFFER, VISIBILITY, ESTONIAN, UNKNOWN]
            .into_iter()
            .map(quote)
            .collect(),
        source_paths: vec![],
    };
    *script.lock().unwrap() = vec![
        Step::Conflict(ConflictArgs {
            title: "Approved shipment target versus cancelled carrier slot".into(),
            summary: "The approved release commitment remains 19 October, but this carrier cancelled its collection slot. The later offer is not release authorization; alternatives and their availability need a decision.".into(),
            source_quote: ConflictQuote { quote: CANCELLED.into(), occurrence: None },
            other_path: "north-quay-approved.md".into(),
            other_quote: ConflictQuote { quote: TARGET.into(), occurrence: None },
        }),
        Step::Action(follow_up("Ask Kaia to decide the North Quay response", RECOMMENDATION, predecessor_id)),
        Step::Action(follow_up("Check an alternate carrier before promising 19 October", "Alternative: ask for actual availability and price; neither is confirmed. Do not book a collection.", predecessor_id)),
        Step::Knowledge(successor),
    ];
    let request = semantic(&carrier_source);
    let turn = analyze(&worker, &request).unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Completed);
    let answer: Value = serde_json::from_str(&turn.answer).unwrap();
    assert_eq!(answer["recommendation"], RECOMMENDATION);
    assert!(
        answer["receipts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r.get("ok").is_some()),
        "{answer}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let investigation = analysis(&worker, request.id);
    assert!(investigation.needs_semantic_review);
    assert_eq!(investigation.proposals.len(), 3);
    assert_eq!(investigation.findings.len(), 1);
    let finding = &investigation.findings[0];
    assert_eq!(finding.state, FindingState::Open);
    assert_eq!(finding.draft.evidence.len(), 2);
    for (evidence, expected_id, text, expected_quote) in [
        (
            &finding.draft.evidence[0],
            carrier_source.note_id,
            carrier_source.source.text.as_str(),
            CANCELLED,
        ),
        (
            &finding.draft.evidence[1],
            predecessor_id,
            predecessor_text.as_str(),
            TARGET,
        ),
    ] {
        assert_eq!(evidence.note_id, Some(expected_id));
        let quote = evidence.quote.as_ref().unwrap();
        assert_eq!(quote.quote, expected_quote);
        assert_eq!(
            text.get(quote.start_byte..quote.end_byte),
            Some(expected_quote)
        );
    }
    assert_eq!(
        finding.draft.evidence[0].source,
        carrier_source.source.source
    );
    assert_eq!(
        finding.draft.evidence[1].source.path,
        "north-quay-approved.md"
    );
    let preferred = investigation
        .proposals
        .iter()
        .find(|p| p.draft.inbox_knowledge.is_some())
        .unwrap();
    let action = investigation
        .proposals
        .iter()
        .find(|p| p.draft.title.starts_with("Ask Kaia"))
        .unwrap();
    let alternative = investigation
        .proposals
        .iter()
        .find(|p| p.draft.title.starts_with("Check an alternate"))
        .unwrap();
    assert!(
        action
            .draft
            .sources
            .iter()
            .any(|s| s.path == "north-quay-approved.md")
    );
    assert_eq!(preferred.draft.changes.len(), 2);
    let history = note_metadata::to_history(&predecessor_text).unwrap();
    assert!(
        matches!(&preferred.draft.changes[1], NoteChange::Replace { before_text, text, .. } if before_text == &predecessor_text && text == &history)
    );
    no_actions(&worker);
    assert!(!f.base.path().join("vault/north-quay-current.md").exists());
    assert_eq!(
        std::fs::read(f.base.path().join("vault/north-quay-approved.md")).unwrap(),
        predecessor_bytes
    );
    let AppEvent::Proposal(rejected) =
        reply(&worker, AppCommand::RejectProposal(alternative.stamp()))
    else {
        panic!("reject alternative")
    };
    assert_eq!(rejected.state, ProposalState::Rejected);
    let edited_text = preferred.draft.changes[0].text().unwrap().replace(
        "Recommendation:",
        "Owner-reviewed recommendation (seek confirmation before booking):",
    );
    let AppEvent::Proposal(edited) = reply(
        &worker,
        AppCommand::EditProposal(ProposalEdit {
            expected: preferred.stamp(),
            title: preferred.draft.title.clone(),
            texts: vec![Some(edited_text), Some(history.clone())],
            action_data: vec![],
        }),
    ) else {
        panic!("owner edit preferred consequence")
    };
    assert!(matches!(
        reply(
            &worker,
            AppCommand::EditProposal(ProposalEdit {
                expected: edited.stamp(),
                title: edited.draft.title.clone(),
                texts: vec![
                    Some(edited.draft.changes[0].text().unwrap().into()),
                    Some(history.clone() + "Altered history")
                ],
                action_data: vec![],
            })
        ),
        AppEvent::Failed(_)
    ));
    let stale = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: preferred.stamp(),
    };
    assert!(matches!(
        reply_at(
            &worker,
            stale.operation_id,
            AppCommand::ApproveProposal(stale)
        ),
        AppEvent::Failed(_)
    ));
    no_actions(&worker);
    assert!(!f.base.path().join("vault/north-quay-current.md").exists());
    // Both consequences captured the predecessor. Apply the Action before its
    // proof becomes History; group order is explicit rather than inferred.
    let group = GroupApprovalRequest {
        group_id: request.id,
        approvals: [action, &edited]
            .into_iter()
            .map(|p| ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: p.stamp(),
            })
            .collect(),
    };
    let AppEvent::ProposalGroupApplied(applied) =
        reply(&worker, AppCommand::ApproveProposalGroup(group.clone()))
    else {
        panic!("ordered exact group")
    };
    assert!(applied.stopped.is_none(), "{applied:?}");
    assert_eq!(applied.receipts.len(), 2);
    for (receipt, approval) in applied.receipts.iter().zip(&group.approvals) {
        assert_eq!(receipt.operation_id, approval.operation_id);
        assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    }
    let current_id = edited.draft.inbox_knowledge.as_ref().unwrap().note_id;
    let current_bytes = std::fs::read(f.base.path().join("vault/north-quay-current.md")).unwrap();
    assert!(String::from_utf8_lossy(&current_bytes).contains("Owner-reviewed recommendation"));
    assert_eq!(
        std::fs::read(f.base.path().join("vault/north-quay-approved.md")).unwrap(),
        history.as_bytes()
    );
    assert!(paths(&worker, KnowledgeScope::Current).contains(&"north-quay-current.md".into()));
    assert!(!paths(&worker, KnowledgeScope::Current).contains(&"north-quay-approved.md".into()));
    assert!(paths(&worker, KnowledgeScope::History).contains(&"north-quay-approved.md".into()));
    let edges = link_tests::relationships(&worker, KnowledgeScope::All).edges;
    assert!(
        edges
            .iter()
            .any(|e| e.source.note_id == current_id && e.target.note_id == predecessor_id)
    );
    let AppEvent::NoteLinks(links) = reply(
        &worker,
        AppCommand::NoteLinks("north-quay-current.md".into()),
    ) else {
        panic!("successor History link")
    };
    assert!(links.links.iter().any(|link| link.destination
        == format!("brn://note/{predecessor_id}")
        && link.target_path.as_deref() == Some("north-quay-approved.md")
        && link.outcome == crate::knowledge::NoteLinkOutcome::Resolved));
    let AppEvent::Actions(actions) =
        reply(&worker, AppCommand::Actions(ActionListRequest::default()))
    else {
        panic!("approved Actions")
    };
    assert_eq!(actions.entries.len(), 1);
    let action_id = action.draft.action_changes[0].id();
    let AppEvent::Action(retained_action) = reply(&worker, AppCommand::Action(action_id)) else {
        panic!("approved follow-up")
    };
    assert!(retained_action.data.sources.contains(&predecessor_id));
    assert_eq!(retained_action.data.owner.as_deref(), Some("Operations"));
    assert_eq!(retained_action.data.due_on.as_deref(), Some("2026-10-16"));
    assert!(retained_action.data.description.contains("16:00 +03:00"));
    assert_eq!(
        analysis(&worker, request.id).findings,
        vec![finding.clone()]
    );
    for (preview, source, bytes) in [
        (&approved_preview, &old_source, APPROVED_EMAIL),
        (&carrier_preview, &carrier_source, CARRIER_EMAIL),
    ] {
        retained_email(&worker, preview, source, bytes);
    }
    no_credentials(&f);
    worker.shutdown().unwrap();
    std::fs::remove_file(f.base.path().join("data/index.sqlite")).unwrap();
    let mut restarted = f.start(Hooks {
        proposal_answer: Some(Arc::new(|_, _, _, _, _, _| {
            panic!("saved investigation replay must not infer")
        })),
        ..Hooks::default()
    });
    let replayed = analyze(&restarted, &request).unwrap();
    assert_eq!(json!(replayed), json!(turn));
    let saved = analysis(&restarted, request.id);
    assert_eq!(saved.job, investigation.job);
    assert_eq!(saved.findings, vec![finding.clone()]);
    assert_eq!(saved.proposals.len(), 3);
    assert_eq!(
        saved
            .proposals
            .iter()
            .find(|p| p.draft.id == rejected.draft.id)
            .unwrap()
            .state,
        ProposalState::Rejected
    );
    let AppEvent::ProposalGroupApplied(replayed_group) =
        reply(&restarted, AppCommand::ApproveProposalGroup(group))
    else {
        panic!("effect replay")
    };
    assert_eq!(replayed_group, applied);
    assert_eq!(
        link_tests::relationships(&restarted, KnowledgeScope::All).edges,
        edges
    );
    let AppEvent::NoteLinks(replayed_links) = reply(
        &restarted,
        AppCommand::NoteLinks("north-quay-current.md".into()),
    ) else {
        panic!("rebuilt successor History link")
    };
    assert_eq!(replayed_links, links);
    assert!(paths(&restarted, KnowledgeScope::Current).contains(&"north-quay-current.md".into()));
    assert!(paths(&restarted, KnowledgeScope::History).contains(&"north-quay-approved.md".into()));
    let AppEvent::Action(replayed_action) = reply(&restarted, AppCommand::Action(action_id)) else {
        panic!("retained Action")
    };
    assert_eq!(replayed_action, retained_action);
    let AppEvent::Actions(actions) = reply(
        &restarted,
        AppCommand::Actions(ActionListRequest::default()),
    ) else {
        panic!("replayed Actions")
    };
    assert_eq!(actions.entries.len(), 1);
    assert_eq!(
        std::fs::read(f.base.path().join("vault/north-quay-current.md")).unwrap(),
        current_bytes
    );
    assert_eq!(
        std::fs::read(f.base.path().join("vault/north-quay-approved.md")).unwrap(),
        history.as_bytes()
    );
    for (path, bytes) in source_bytes {
        assert_eq!(
            std::fs::read(f.base.path().join("vault").join(path)).unwrap(),
            bytes
        );
    }
    for (preview, source, bytes) in [
        (&approved_preview, &old_source, APPROVED_EMAIL),
        (&carrier_preview, &carrier_source, CARRIER_EMAIL),
    ] {
        retained_email(&restarted, preview, source, bytes);
    }
    // Applying knowledge and an Action never closes a tentative Finding.
    // History lookup retains exact old quotes while showing the changed proof.
    let AppEvent::NoteConflicts(conflicts) = reply(
        &restarted,
        AppCommand::NoteConflicts(Box::new(NoteConflictRequest {
            path: "north-quay-approved.md".into(),
            scope: KnowledgeScope::History,
            limit: 10,
            cursor: None,
        })),
    ) else {
        panic!("History conflict inspection")
    };
    assert_eq!(conflicts.open_count, 1);
    assert_eq!(conflicts.entries[0].record, *finding);
    assert_eq!(
        conflicts.entries[0].evidence[0].outcome,
        FindingEvidenceOutcome::Unchanged
    );
    assert_eq!(
        conflicts.entries[0].evidence[1].outcome,
        FindingEvidenceOutcome::Changed
    );
    let AppEvent::Finding(closed) = reply(
        &restarted,
        AppCommand::CloseFinding(CloseFindingRequest {
            expected: finding.stamp(),
            state: FindingState::Resolved,
        }),
    ) else {
        panic!("explicit owner Finding closure")
    };
    assert_eq!(closed.draft, finding.draft);
    assert_eq!(closed.state, FindingState::Resolved);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    no_credentials(&f);
    restarted.shutdown().unwrap();
}

#[test]
fn p4_completed_historical_investigation_replays_its_question_without_new_guidance_or_inference() {
    let fixture = Fixture::new();
    let mut worker = fixture.start(Hooks::default());
    let source = capture_source(&worker, "Retained synthetic shipment evidence.\r\n");
    let request = semantic(&source);
    worker.shutdown().unwrap();
    let mut app =
        crate::app::App::open(&fixture.base.path().join("data"), fixture.config()).unwrap();
    let (prepared, capture) = app.prepare_inbox_action_request(&request).unwrap();
    let old_question = "Historical synthetic investigation: describe opposing facts without preferring a resolution.";
    assert_ne!(prepared.question, old_question);
    let job = app
        .work_store_mut()
        .reserve_inbox_action(&capture, old_question)
        .unwrap();
    app.work_store_mut().begin_inbox_action_turn(&job).unwrap();
    let completed = app
        .work_store_mut()
        .finish_turn(
            request.id,
            WorkTurnStatus::Completed,
            "Historical synthetic answer retained exactly.\r\n",
            None,
        )
        .unwrap();
    let canonical_job = serde_json::to_vec(&job).unwrap();
    let canonical_capture = serde_json::to_vec(&capture).unwrap();
    drop(app);
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(Arc::new(|_, _, _, _, _, _| {
            panic!("historical completed investigation must not infer")
        })),
        ..Hooks::default()
    });
    assert_eq!(json!(analyze(&worker, &request).unwrap()), json!(completed));
    let saved = analysis(&worker, request.id);
    assert_eq!(saved.job.question, old_question);
    assert_eq!(serde_json::to_vec(&saved.job).unwrap(), canonical_job);
    assert_eq!(
        serde_json::to_vec(&saved.job.capture).unwrap(),
        canonical_capture
    );
    assert_eq!(json!(saved.turn.unwrap()), json!(completed));
    assert!(saved.proposals.is_empty() && saved.findings.is_empty());
    assert_eq!(saved.budget, None);
    no_actions(&worker);
    retained_original(&worker, &source);
    no_credentials(&fixture);
    worker.shutdown().unwrap();
}
