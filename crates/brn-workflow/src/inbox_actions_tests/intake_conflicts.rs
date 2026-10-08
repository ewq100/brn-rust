//! Actual MIME/worker captures over an exact Applied private Source prerequisite.
use super::*;
use crate::findings::{
    CaptureFindingRequest, CloseFindingRequest, FindingEvidenceOutcome, FindingOrigin,
    FindingQuote, FindingRecord, FindingState, NoteConflictRequest,
};
use crate::library::KnowledgeScope;
use brn_ai::{ConflictArgs, ConflictQuote};

const SOURCE_QUOTE: &str = "Decision: extend the pilot to Pier B on 15 October 2026. Owner: Mira.";
fn start_checked(fixture: &Fixture, hooks: Hooks) -> (AppWorker, bool) {
    let worker = crate::app_worker::start_test(
        fixture.base.path().join("data"),
        fixture.config(),
        hooks,
        None,
        None,
    )
    .unwrap();
    let mut restored = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let initial = worker
            .recv_event_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        match initial.1 {
            AppEvent::Restored { .. } => restored = true,
            AppEvent::Ready { .. } => return (worker, restored),
            AppEvent::Failed(error) => panic!("startup failure: {error:?}"),
            other => panic!("unexpected startup event: {}", event_name(&other)),
        }
    }
}
fn args() -> ConflictArgs {
    ConflictArgs { title: "Tentative retained-source disagreement".into(), summary: "Inspect the opposing exact evidence before an owner resolution; Source approval alone is not truth.".into(), source_quote: ConflictQuote { quote: SOURCE_QUOTE.into(), occurrence: None }, other_path: "context.md".into(), other_quote: ConflictQuote { quote: "Known context.".into(), occurrence: None } }
}
fn selected(text: &str, quote: &str) -> FindingQuote {
    let start_byte = text.find(quote).unwrap();
    FindingQuote {
        start_byte,
        end_byte: start_byte + quote.len(),
        quote: quote.into(),
    }
}

#[test]
fn applied_intake_worker_conflict_exact_replay_loss_undo_restart_and_backup_keep_capture_unchanged()
{
    let fixture = Fixture::new();
    let (created, ready) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let resume = Arc::new(Mutex::new(resume));
    let calls = Arc::new(AtomicUsize::new(0));
    let called = calls.clone();
    let hook: ProposalAnswerHook = Arc::new(move |ask, _, _, proposals, _, _| {
        called.fetch_add(1, Ordering::SeqCst);
        let created = created.clone();
        let resume = resume.clone();
        Box::pin(async move {
            let out = tokio::task::spawn_blocking(move || {
                let evidence: Value =
                    serde_json::from_str(ask.question.rsplit_once("\n\n").unwrap().1).unwrap();
                assert_eq!(evidence["source_approval"], "applied");
                assert!(
                    ask.question
                        .contains("When source_approval is applied, report_conflict")
                );
                assert!(ask.question.contains(
                    "When source_approval is pending, explain conflicts in the answer only"
                ));
                created
                    .send(proposals.report_conflict(args()).unwrap())
                    .unwrap();
                resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
                let replay = tool_reply(proposals.report_conflict(args()));
                let mut changed = args();
                changed.title.push_str(" after Source loss");
                vec![replay, tool_reply(proposals.report_conflict(changed))]
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&out).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let (mut worker, restored) = start_checked(
        &fixture,
        Hooks {
            proposal_answer: Some(hook),
            ..Hooks::default()
        },
    );
    assert!(!restored);
    let (preview, source) = approve_source(&worker);
    let (_, other) = link_tests::target(&worker, "context.md", false);
    let request = request(binding(&worker, &source));
    assert!(request.source.is_none());
    let path = fixture.base.path().join("vault/harbor-source.md");
    let source_bytes = std::fs::read(&path).unwrap();
    let original_path = preview
        .original
        .capture
        .copy
        .directory
        .join(preview.original.capture.copy_name());
    let original_bytes = std::fs::read(&original_path).unwrap();
    worker
        .submit(
            request.id,
            AppCommand::AnalyzeInboxActions(Box::new(request.clone())),
        )
        .unwrap();
    let record: FindingRecord =
        serde_json::from_value(ready.recv_timeout(Duration::from_secs(10)).unwrap()).unwrap();
    let initial = analysis(&worker, request.id);
    assert_eq!(initial.findings, vec![record.clone()]);
    assert!(initial.proposals.is_empty());
    assert!(initial.job.capture.source.is_none());
    assert_eq!(initial.job.capture.intake, request.intake);
    let canonical_job = serde_json::to_vec(&initial.job).unwrap();
    let expected_source = record.draft.evidence[0].source.clone();
    assert_eq!(
        record.draft.evidence[0].note_id,
        Some(request.intake.as_ref().unwrap().source_note_id)
    );
    assert_eq!(record.draft.evidence[1].source, other.source);
    for (proof, text, wording) in [
        (
            &record.draft.evidence[0],
            initial.job.capture.source_text.as_str(),
            SOURCE_QUOTE,
        ),
        (
            &record.draft.evidence[1],
            other.text.as_str(),
            "Known context.",
        ),
    ] {
        let quote = proof.quote.as_ref().unwrap();
        assert_eq!(quote.quote, wording);
        assert_eq!(text.get(quote.start_byte..quote.end_byte), Some(wording));
    }
    let AppEvent::NoteConflicts(page) = reply(
        &worker,
        AppCommand::NoteConflicts(Box::new(NoteConflictRequest {
            path: "harbor-source.md".into(),
            scope: KnowledgeScope::Source,
            limit: 10,
            cursor: None,
        })),
    ) else {
        panic!("Source conflict page")
    };
    assert_eq!(page.open_count, 1);
    assert!(
        page.entries[0]
            .evidence
            .iter()
            .all(|e| e.outcome == FindingEvidenceOutcome::Unchanged)
    );
    let AppEvent::Finding(closed) = reply(
        &worker,
        AppCommand::CloseFinding(CloseFindingRequest {
            expected: record.stamp(),
            state: FindingState::Resolved,
        }),
    ) else {
        panic!("explicit Finding closure")
    };
    let held = fixture.base.path().join("held-source.md");
    std::fs::rename(&path, &held).unwrap();
    release.send(()).unwrap();
    let turn = finish(&worker, &request).unwrap();
    let out = results(&turn);
    assert_eq!(out[0]["ok"], json!(closed));
    assert!(out[1].get("error").is_some());
    assert_eq!(
        analysis(&worker, request.id).findings,
        vec![(*closed).clone()]
    );
    std::fs::rename(&held, &path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), source_bytes);
    no_actions(&worker);
    let AppEvent::FindingInspection(inspected) =
        reply(&worker, AppCommand::InspectFinding(record.draft.request.id))
    else {
        panic!("historical inspection")
    };
    assert_eq!(inspected.record, *closed);
    let source_operation = {
        let AppEvent::ProposalApplies(applies) = reply(&worker, AppCommand::ProposalApplies) else {
            panic!("Source approval receipt")
        };
        applies
            .into_iter()
            .find(|entry| entry.request.expected.id == source.draft.id)
            .unwrap()
            .request
            .operation_id
    };
    let undo = crate::proposal_apply::UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: source_operation,
        trash_member: None,
    };
    let AppEvent::ProposalApplied(undone) =
        reply_at(&worker, undo.operation_id, AppCommand::UndoProposal(undo))
    else {
        panic!("explicit Source Undo")
    };
    assert_eq!(undone.outcome, ApplyOutcome::Applied);
    assert!(!path.exists());
    let AppEvent::FindingInspection(inspected) =
        reply(&worker, AppCommand::InspectFinding(record.draft.request.id))
    else {
        panic!("inspection after Undo")
    };
    assert_eq!(inspected.record, *closed);
    assert_eq!(
        inspected.evidence[0].outcome,
        FindingEvidenceOutcome::Unavailable
    );
    assert_eq!(
        inspected.evidence[1].outcome,
        FindingEvidenceOutcome::Unchanged
    );
    assert_eq!(std::fs::read(&original_path).unwrap(), original_bytes);
    assert_eq!(
        serde_json::to_vec(&analysis(&worker, request.id).job).unwrap(),
        canonical_job
    );
    worker.shutdown().unwrap();
    let panic_hook = || Hooks {
        proposal_answer: Some(Arc::new(|_, _, _, _, _, _| {
            panic!("history must not infer")
        })),
        ..Hooks::default()
    };
    let (mut worker, restored) = start_checked(&fixture, panic_hook());
    assert!(!restored);
    assert_eq!(json!(analyze(&worker, &request).unwrap()), json!(turn));
    assert_eq!(
        serde_json::to_vec(&analysis(&worker, request.id).job).unwrap(),
        canonical_job
    );
    let AppEvent::Finding(saved) = reply(&worker, AppCommand::Finding(record.draft.request.id))
    else {
        panic!("restart Finding")
    };
    assert_eq!(*saved, *closed);
    worker.shutdown().unwrap();
    std::fs::write(
        fixture.base.path().join("data/brn.sqlite"),
        b"synthetic physical damage",
    )
    .unwrap();
    let (mut worker, restored) = start_checked(&fixture, panic_hook());
    assert!(restored);
    assert_eq!(json!(analyze(&worker, &request).unwrap()), json!(turn));
    assert_eq!(
        serde_json::to_vec(&analysis(&worker, request.id).job).unwrap(),
        canonical_job
    );
    let AppEvent::Finding(saved) = reply(&worker, AppCommand::Finding(record.draft.request.id))
    else {
        panic!("backup Finding")
    };
    assert_eq!(*saved, *closed);
    assert_eq!(saved.draft.evidence[0].source, expected_source);
    let AppEvent::InboxCandidate(retained) =
        reply(&worker, AppCommand::InboxCandidate(preview.request.clone()))
    else {
        panic!("retained snapshot")
    };
    assert_eq!(*retained, preview);
    assert_eq!(std::fs::read(&original_path).unwrap(), original_bytes);
    assert!(!path.exists());
    no_actions(&worker);
    no_credentials(&fixture);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    worker.shutdown().unwrap();
}

#[test]
fn applied_intake_fresh_finding_refuses_pending_reinstalled_original_asset_and_identity_changes() {
    for damage in [
        "pending", "inode", "source", "original", "asset", "identity",
    ] {
        let fixture = Fixture::new();
        let mut worker = fixture.start(Hooks::default());
        let (preview, source) = if damage == "pending" {
            intake(
                &worker,
                include_bytes!(
                    "../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml"
                ),
                "plural.eml",
            )
        } else {
            approve_source(&worker)
        };
        let (_, other) = link_tests::target(&worker, "context.md", false);
        let request = request(binding(&worker, &source));
        worker.shutdown().unwrap();
        let mut app = App::open(&fixture.base.path().join("data"), fixture.config()).unwrap();
        let (ask, capture) = app.prepare_inbox_action_request(&request).unwrap();
        if damage == "pending" {
            assert!(
                ask.question.contains(
                    "When source_approval is pending, explain conflicts in the answer only"
                )
            );
            let evidence: Value =
                serde_json::from_str(ask.question.rsplit_once("\n\n").unwrap().1).unwrap();
            assert_eq!(evidence["source_approval"], "pending");
        }
        let job = app
            .work_store_mut()
            .reserve_inbox_action(&capture, &ask.question)
            .unwrap();
        let canonical = serde_json::to_vec(&job).unwrap();
        let source_path = fixture.base.path().join("vault/harbor-source.md");
        match damage {
            "pending" => {}
            "inode" => {
                let text = std::fs::read(&source_path).unwrap();
                std::fs::rename(&source_path, fixture.base.path().join("held-source.md")).unwrap();
                std::fs::write(&source_path, text).unwrap();
            }
            "source" => {
                let mut text = std::fs::read(&source_path).unwrap();
                text.extend_from_slice(b"\nChanged Source\n");
                std::fs::write(&source_path, text).unwrap();
            }
            "original" => std::fs::write(
                preview
                    .original
                    .capture
                    .copy
                    .directory
                    .join(preview.original.capture.copy_name()),
                b"changed original",
            )
            .unwrap(),
            "asset" => std::fs::write(
                fixture
                    .base
                    .path()
                    .join("vault")
                    .join(source.draft.changes[1].path()),
                b"changed image",
            )
            .unwrap(),
            "identity" => {
                std::fs::copy(&source_path, fixture.base.path().join("vault/alias.md")).unwrap();
            }
            _ => unreachable!(),
        }
        let command = CaptureFindingRequest {
            id: Uuid::new_v4(),
            origin: FindingOrigin::InboxConflict {
                analysis_id: request.id,
                title: "Refuse changed retained evidence".into(),
                summary: "Synthetic admission witness".into(),
                source_quote: selected(&capture.source_text, SOURCE_QUOTE),
                other_path: "context.md".into(),
                other_quote: selected(&other.text, "Known context."),
            },
        };
        assert!(app.capture_finding(&command).is_err(), "{damage}");
        assert!(app.work_store().finding(command.id).unwrap().is_none());
        assert!(
            app.work_store()
                .inbox_conflicts(request.id)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            serde_json::to_vec(&app.work_store().inbox_action(request.id).unwrap().unwrap())
                .unwrap(),
            canonical
        );
        assert!(app.work_store().turn(request.id).unwrap().is_none());
        assert!(
            app.work_store()
                .proposals(Some(request.id))
                .unwrap()
                .is_empty()
        );
        no_credentials(&fixture);
    }
}
