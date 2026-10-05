//! Client correlation over actual approved synthetic Source proofs; no inference.
use super::*;
use brn_workflow::{
    WorkflowError,
    app::AppConfig,
    app_worker::AppWorker,
    inbox::{CaptureInboxRequest, InboxItem, InboxKind, InboxOriginal},
    inbox_actions::{
        InboxActionAnalysis, InboxActionCapture, InboxActionJob, InboxAnalysisPurpose,
    },
    inbox_processing::{
        InboxCandidateRequest, InboxProcessOutcome, InboxSourceRequest, ProcessInboxRequest,
    },
    proposals::{DraftNoteChange, DraftRequest, ProposalSource},
};
use std::{fs, os::unix::fs::PermissionsExt};

struct Fixture {
    base: tempfile::TempDir,
    source: ProposalSource,
    original: InboxItem,
    raw: String,
}
fn reply(worker: &AppWorker, command: (Uuid, AppCommand)) -> AppEvent {
    worker.submit(command.0, command.1).unwrap();
    loop {
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if id == command.0 {
            return event;
        }
    }
}
impl Fixture {
    fn new() -> (Self, AppWorker) {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        fs::set_permissions(base.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(base.path().join("data")).unwrap();
        fs::create_dir(base.path().join("vault")).unwrap();
        fs::write(
            base.path().join("vault/untouched.md"),
            "\u{feff}Untouched õ\r\n",
        )
        .unwrap();
        let worker = AppWorker::start(
            base.path().join("data"),
            AppConfig {
                vault_root: Some(base.path().join("vault")),
                credentials_dir: Some(base.path().join("credentials")),
                model_dir: None,
            },
        )
        .unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let raw = "Exact email evidence: Blue õ 🦀\r\nFollow up with Anna.\r\n".to_owned();
        let request = CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: InboxKind::Email,
            title: "Synthetic source to analyze".into(),
            original_name: Some("synthetic-email.txt".into()),
            text: raw.clone(),
        };
        let AppEvent::InboxCaptured(original) = reply(
            &worker,
            (request.id, AppCommand::CaptureInbox(request.clone())),
        ) else {
            panic!("capture");
        };
        request.validate_receipt(&original).unwrap();
        let process = ProcessInboxRequest {
            id: Uuid::new_v4(),
            items: vec![(*original).clone()],
        };
        worker
            .submit(process.id, AppCommand::ProcessInbox(process.clone()))
            .unwrap();
        loop {
            let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
            if id != process.id {
                continue;
            }
            match event {
                AppEvent::InboxProcessing(batch) if batch.pending_count() == 0 => {
                    assert!(matches!(
                        batch.entries[0].outcome,
                        InboxProcessOutcome::Converted { .. }
                    ));
                    break;
                }
                AppEvent::InboxProcessing(_) => {}
                AppEvent::Failed(error) => panic!("conversion: {error:?}"),
                _ => panic!("conversion response"),
            }
        }
        let source_request = InboxSourceRequest {
            candidate: InboxCandidateRequest {
                batch_id: process.id,
                index: 0,
            },
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "source.md".into(),
            title: "Exactly approve preserved Source".into(),
        };
        let AppEvent::InboxSourceDraft(draft) = reply(
            &worker,
            (
                Uuid::new_v4(),
                AppCommand::PrepareInboxSource(source_request.clone()),
            ),
        ) else {
            panic!("prepare");
        };
        source_request.validate_draft(&draft).unwrap();
        let AppEvent::Proposal(record) = reply(
            &worker,
            (Uuid::new_v4(), AppCommand::CreateProposal(*draft)),
        ) else {
            panic!("Create");
        };
        let approval = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        };
        assert!(matches!(
            reply(&worker, (approval.operation_id, AppCommand::ApproveProposal(approval))),
            AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied
        ));
        let AppEvent::ProposalSource(source) = reply(
            &worker,
            (
                Uuid::new_v4(),
                AppCommand::ProposalEvidenceSource("source.md".into()),
            ),
        ) else {
            panic!("full saved evidence");
        };
        source.validate().unwrap();
        (
            Self {
                base,
                source: *source,
                original: *original,
                raw,
            },
            worker,
        )
    }
    fn preserved(&self, worker: &AppWorker) {
        let AppEvent::InboxItem(read) = reply(
            worker,
            (
                Uuid::new_v4(),
                AppCommand::InboxItem(self.original.capture.id),
            ),
        ) else {
            panic!("retained original");
        };
        assert_eq!(read.item, self.original);
        assert_eq!(
            read.original,
            InboxOriginal::Available {
                text: self.raw.clone()
            }
        );
        assert_eq!(
            fs::read(self.base.path().join("vault/source.md")).unwrap(),
            self.source.text.as_bytes()
        );
        assert_eq!(
            fs::read(self.base.path().join("vault/untouched.md")).unwrap(),
            "\u{feff}Untouched õ\r\n".as_bytes()
        );
        assert_eq!(
            fs::read_dir(self.base.path().join("vault"))
                .unwrap()
                .count(),
            2
        );
        assert_eq!(
            fs::read_dir(self.base.path().join("credentials"))
                .unwrap()
                .count(),
            0
        );
    }
}

/// Narrow shared native-widget fixture, not a production testing interface.
#[cfg(all(target_os = "macos", feature = "native-test-support"))]
pub(crate) fn source_fixture() -> (tempfile::TempDir, ProposalSource) {
    let (f, mut worker) = Fixture::new();
    f.preserved(&worker);
    worker.shutdown().unwrap();
    (f.base, f.source)
}

fn state() -> AiState {
    let mut state = AiState {
        ready: true,
        vault_bound: true,
        selection: Some(Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6-luna".into(),
        }),
        effort: Some(ReasoningEffort::High),
        generation: 11,
        conversation: Some(Uuid::new_v4()),
        ..Default::default()
    };
    state.open_inbox().unwrap();
    state
}
fn load(state: &mut AiState, source: &ProposalSource) {
    let (id, command) = state
        .inspect_inbox_analysis_source(source.source.path.clone())
        .unwrap();
    assert!(
        matches!(command, AppCommand::ProposalEvidenceSource(ref path) if path == &source.source.path)
    );
    assert!(state.inbox_analysis_source_loading());
    assert!(!state.can_analyze_inbox_source());
    state.apply(id, AppEvent::ProposalSource(Box::new(source.clone())));
    assert!(!state.inbox_analysis_source_loading());
}
fn submit(state: &mut AiState) -> InboxActionRequest {
    let (id, command) = state.analyze_inbox_source().unwrap();
    let AppCommand::AnalyzeInboxActions(request) = command else {
        panic!("analysis command");
    };
    assert_eq!(id, request.id);
    request.validate().unwrap();
    *request
}
fn symbolic_analysis(request: &InboxActionRequest) -> InboxActionAnalysis {
    let job = InboxActionJob {
        capture: InboxActionCapture {
            purpose: request.purpose,
            id: request.id,
            conversation: request.conversation,
            source: request.source.source.clone(),
            source_text: request.source.text.clone(),
            provider: match request.selection.provider {
                Provider::Chatgpt => "chatgpt",
                Provider::Copilot => "copilot",
            }
            .into(),
            model: request.selection.model.clone(),
            effort: request.effort.as_str().into(),
        },
        question: "Synthetic retained workflow question; client never generates it".into(),
        created_at_ms: 1,
    };
    job.validate().unwrap();
    let turn = WorkTurn {
        id: request.id,
        conversation_id: request.conversation.unwrap_or_else(Uuid::new_v4),
        question: job.question.clone(),
        answer: "Independent review drafts only".into(),
        provider: job.capture.provider.clone(),
        model: job.capture.model.clone(),
        effort: Some(job.capture.effort.clone()),
        status: WorkTurnStatus::Completed,
        error_code: None,
        started_at_ms: Some(1),
        finished_at_ms: Some(2),
    };
    InboxActionAnalysis {
        job,
        turn: Some(turn),
        proposals: vec![],
        needs_semantic_review: true,
    }
}
fn group_review(worker: &AppWorker, request: &InboxActionRequest) -> ProposalRecord {
    let AppEvent::Proposal(record) = reply(
        worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(DraftRequest {
                inbox_knowledge: None,
                inbox_source: None,
                id: Uuid::new_v4(),
                group_id: Some(request.id),
                session_id: None,
                title: "Synthetic separately reviewable candidate".into(),
                changes: vec![DraftNoteChange::Create {
                    path: "candidate.md".into(),
                    text: "# Independent candidate\r\n".into(),
                }],
                sources: vec![request.source.source.clone()],
                action_changes: vec![],
            }),
        ),
    ) else {
        panic!("separate review");
    };
    record
}

#[test]
fn actual_approved_source_submission_freezes_selection_effort_generation_and_session() {
    let (f, mut worker) = Fixture::new();
    let mut state = state();
    let command = state
        .inspect_inbox_analysis_source("source.md".into())
        .unwrap();
    let id = command.0;
    let proof = reply(&worker, command);
    state.apply(id, proof);
    assert_eq!(state.inbox_analysis.source.as_ref(), Some(&f.source));
    let conversation = state.conversation;
    let selection = state.selection.clone().unwrap();
    let generation = state.generation;
    let request = submit(&mut state);
    assert_eq!(request.purpose, InboxAnalysisPurpose::KnowledgeAndActions);
    assert_eq!(*request.source, f.source);
    assert_eq!(request.conversation, conversation);
    assert_eq!(request.selection, selection);
    assert_eq!(request.effort, ReasoningEffort::High);
    assert_eq!(request.generation, generation);
    state.selection = Some(Selection {
        provider: Provider::Copilot,
        model: "gpt-6-sol".into(),
    });
    state.effort = Some(ReasoningEffort::Low);
    state.navigate(Some(Uuid::new_v4()));
    assert_eq!(
        state.active.as_ref().unwrap().request.inbox(),
        Some(&request)
    );
    assert_eq!(state.inbox_analysis.request.as_ref(), Some(&request));
    assert!(!state.can_analyze_inbox_source());
    assert!(state.analyze_inbox_source().is_none());
    assert!(state.ask("second concurrent job".into()).is_none());
    f.preserved(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn source_reads_require_exact_operation_path_hash_and_current_view_generation() {
    let (f, mut worker) = Fixture::new();
    let mut state = state();
    let old = state
        .inspect_inbox_analysis_source("source.md".into())
        .unwrap();
    let current = state
        .inspect_inbox_analysis_source("source.md".into())
        .unwrap();
    state.apply(old.0, AppEvent::ProposalSource(Box::new(f.source.clone())));
    assert!(state.inbox_analysis.source.is_none());
    assert!(!state.pending.contains_key(&old.0));
    state.apply(
        Uuid::new_v4(),
        AppEvent::ProposalSource(Box::new(f.source.clone())),
    );
    assert!(state.inbox_analysis.source.is_none());
    for mode in 0..3 {
        let mut wrong = f.source.clone();
        match mode {
            0 => wrong.source.path = "other.md".into(),
            1 => wrong.source.fingerprint.sha256[0] ^= 1,
            _ => wrong.text.push_str("uncaptured bytes"),
        }
        state.apply(current.0, AppEvent::ProposalSource(Box::new(wrong)));
        assert!(state.inbox_analysis.source.is_none());
        assert!(state.pending.contains_key(&current.0));
    }
    state.close_inbox();
    state.open_inbox().unwrap();
    state.apply(
        current.0,
        AppEvent::Failed(WorkflowError::msg("closed-view failure")),
    );
    assert!(state.inbox_analysis.source_error.is_none());
    assert!(!state.pending.contains_key(&current.0));
    let pending = state
        .inspect_inbox_analysis_source("source.md".into())
        .unwrap();
    state.apply(
        pending.0,
        AppEvent::Failed(WorkflowError::msg("current read refused")),
    );
    assert_eq!(
        state.inbox_analysis.source_error.as_deref(),
        Some("current read refused")
    );
    assert!(!state.can_analyze_inbox_source());
    load(&mut state, &f.source);
    assert!(state.inbox_analysis.source_error.is_none());
    assert!(state.can_analyze_inbox_source());
    f.preserved(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn analysis_submission_obeys_selection_effort_application_rewrite_and_unsaved_gates() {
    let (f, mut worker) = Fixture::new();
    for mode in 0..14 {
        let mut state = state();
        load(&mut state, &f.source);
        let reference = submit(&mut state);
        state.active = None;
        match mode {
            0 => state.ready = false,
            1 => state.vault_bound = false,
            2 => state.selection = None,
            3 => state.selection_error = Some("invalid selection".into()),
            4 => state.effort = None,
            5 => state.effort_error = Some("invalid effort".into()),
            6 => {
                state.pending.insert(Uuid::new_v4(), Pending::Select);
            }
            7 => {
                state.pending.insert(Uuid::new_v4(), Pending::Effort);
            }
            8 => {
                state.pending.insert(Uuid::new_v4(), Pending::SelectEffort);
            }
            9 => {
                state.pending.insert(
                    Uuid::new_v4(),
                    Pending::AppliedReview {
                        id: Uuid::new_v4(),
                        generation: 1,
                    },
                );
            }
            10 => {
                state.ask("ordinary Ask already active".into()).unwrap();
            }
            11 => {
                state.rewrite = Some(ActiveRewrite {
                    request: RewriteRequest {
                        id: Uuid::new_v4(),
                        expected: brn_workflow::proposals::ProposalStamp {
                            id: Uuid::new_v4(),
                            version: 1,
                        },
                        selection: reference.selection.clone(),
                        effort: reference.effort,
                        generation: 1,
                    },
                    job: None,
                    tool: None,
                    stopping: false,
                })
            }
            12 => state.unsaved = symbolic_analysis(&reference).turn,
            _ => state.close_inbox(),
        }
        assert!(!state.can_analyze_inbox_source(), "mode {mode}");
        assert!(state.analyze_inbox_source().is_none(), "mode {mode}");
    }
    f.preserved(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn global_stop_and_terminal_analysis_ignore_forged_receipts_and_survive_navigation() {
    let (f, mut worker) = Fixture::new();
    let mut state = state();
    load(&mut state, &f.source);
    let request = submit(&mut state);
    let exact = symbolic_analysis(&request).turn.unwrap();
    for mode in 0..8 {
        let mut wrong = exact.clone();
        match mode {
            0 => wrong.id = Uuid::new_v4(),
            1 => wrong.provider = "copilot".into(),
            2 => wrong.model = "different-model".into(),
            3 => wrong.effort = Some("low".into()),
            4 => wrong.conversation_id = Uuid::new_v4(),
            5 => wrong.status = WorkTurnStatus::Running,
            6 => {
                state.apply(
                    Uuid::new_v4(),
                    AppEvent::Chat(ChatEvent::Finished {
                        id: request.id,
                        generation: request.generation,
                        turn: wrong,
                    }),
                );
                assert!(state.active.is_some());
                continue;
            }
            _ => {
                state.apply(
                    request.id,
                    AppEvent::Chat(ChatEvent::Finished {
                        id: request.id,
                        generation: request.generation + 1,
                        turn: wrong,
                    }),
                );
                assert!(state.active.is_some());
                continue;
            }
        }
        assert!(
            state
                .apply(
                    request.id,
                    AppEvent::Chat(ChatEvent::Finished {
                        id: request.id,
                        generation: request.generation,
                        turn: wrong,
                    })
                )
                .is_empty()
        );
        assert!(state.active.is_some(), "receipt mode {mode}");
    }
    assert!(
        state
            .apply(
                request.id,
                AppEvent::Chat(ChatEvent::AlreadyRunning {
                    id: request.id,
                    generation: request.generation,
                    turn: exact.clone(),
                }),
            )
            .is_empty(),
        "AlreadyRunning cannot acknowledge a terminal receipt"
    );
    assert!(state.active.is_some());
    assert_eq!(state.stop(), Some(request.id));
    assert!(
        state
            .stop_controls()
            .iter()
            .any(|(_, command)| matches!(command, AppCommand::CancelTurn(id) if *id == request.id))
    );
    state.navigate(Some(Uuid::new_v4()));
    assert!(state.display_active().is_none());
    let commands = state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::Text {
            id: request.id,
            generation: request.generation,
            text: "Partial exact õ".into(),
        }),
    );
    assert_eq!(state.active.as_ref().unwrap().partial, "Partial exact õ");
    assert!(
        commands
            .iter()
            .any(|(_, command)| matches!(command, AppCommand::CancelTurn(id) if *id == request.id))
    );
    let commands = state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::Finished {
            id: request.id,
            generation: request.generation,
            turn: exact,
        }),
    );
    assert!(state.active.is_none());
    assert!(
        state.turns.is_empty(),
        "old conversation cannot replace current navigation"
    );
    assert!(
        commands
            .iter()
            .any(|(_, command)| matches!(command, AppCommand::Proposals(None)))
    );
    assert!(commands.iter().any(
        |(_, command)| matches!(command, AppCommand::InboxActionAnalysis(id) if *id == request.id)
    ));
    assert!(state.inbox_analysis_loading());
    f.preserved(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn analysis_failure_retains_partial_without_finalization_and_refreshes_review_lookup() {
    let (f, mut worker) = Fixture::new();
    for persistence_failure in [false, true] {
        let mut state = state();
        load(&mut state, &f.source);
        let request = submit(&mut state);
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "Partial õ".into(),
            }),
        );
        state.navigate(None);
        let event = if persistence_failure {
            ChatEvent::PersistenceFailed {
                id: request.id,
                generation: request.generation,
                partial: "Unfinalized full partial õ".into(),
                error: WorkflowError::msg("synthetic storage failure"),
            }
        } else {
            ChatEvent::Rejected {
                id: request.id,
                generation: request.generation,
                error: WorkflowError::msg("synthetic admission rejection"),
            }
        };
        let commands = state.apply(request.id, AppEvent::Chat(event));
        assert!(state.active.is_none());
        let partial = state.unsaved.as_ref().unwrap();
        assert_eq!(partial.id, request.id);
        assert_eq!(partial.model, request.selection.model);
        assert_eq!(partial.effort.as_deref(), Some("high"));
        assert!(partial.answer.contains("Partial") || partial.answer.contains("partial"));
        assert!(state.turns.is_empty());
        assert!(!state.can_analyze_inbox_source());
        assert!(commands.iter().any(|(_, command)| matches!(command, AppCommand::InboxActionAnalysis(id) if *id == request.id)));
    }
    f.preserved(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn retained_analysis_lookup_checks_full_capture_turn_group_and_view_without_live_model() {
    let (f, mut worker) = Fixture::new();
    let mut state = state();
    load(&mut state, &f.source);
    let request = submit(&mut state);
    let mut exact = symbolic_analysis(&request);
    exact.proposals = vec![group_review(&worker, &request)];
    state.active = None;
    let query = state.inspect_inbox_analysis(request.id).unwrap();
    for mode in 0..14 {
        let mut wrong = exact.clone();
        match mode {
            0 => wrong.job.capture.id = Uuid::new_v4(),
            1 => wrong.job.capture.source.fingerprint.sha256[0] ^= 1,
            2 => wrong.job.capture.source.path = "other.md".into(),
            3 => wrong.job.capture.purpose = InboxAnalysisPurpose::Actions,
            4 => wrong.turn.as_mut().unwrap().id = Uuid::new_v4(),
            5 => wrong.turn.as_mut().unwrap().model = "different-model".into(),
            6 => wrong.turn.as_mut().unwrap().effort = Some("low".into()),
            7 => wrong.proposals[0].draft.group_id = Some(Uuid::new_v4()),
            8 => wrong.proposals.push(wrong.proposals[0].clone()),
            9 => wrong.needs_semantic_review = false,
            10 => wrong.turn.as_mut().unwrap().conversation_id = Uuid::new_v4(),
            11 => wrong
                .turn
                .as_mut()
                .unwrap()
                .question
                .push_str(" forked capture"),
            12 => wrong.proposals[0].draft.id = Uuid::nil(),
            _ => {
                wrong.proposals = (0..=brn_workflow::inbox_actions::MAX_INBOX_ACTION_PROPOSALS)
                    .map(|_| {
                        let mut record = exact.proposals[0].clone();
                        record.draft.id = Uuid::new_v4();
                        record
                    })
                    .collect();
            }
        }
        state.apply(query.0, AppEvent::InboxActionAnalysis(Box::new(wrong)));
        assert!(state.pending.contains_key(&query.0), "mode {mode}");
        assert!(state.inbox_analysis.record.is_none(), "mode {mode}");
    }
    state.apply(
        query.0,
        AppEvent::InboxActionAnalysis(Box::new(exact.clone())),
    );
    assert!(!state.pending.contains_key(&query.0));
    assert_eq!(state.inbox_analysis.record.as_ref().unwrap().job, exact.job);
    let old = state.inspect_inbox_analysis(request.id).unwrap();
    let current = state.inspect_inbox_analysis(request.id).unwrap();
    state.apply(
        old.0,
        AppEvent::InboxActionAnalysis(Box::new(exact.clone())),
    );
    assert!(state.inbox_analysis.record.is_none());
    assert!(!state.pending.contains_key(&old.0));
    state.close_inbox();
    state.open_inbox().unwrap();
    state.apply(
        current.0,
        AppEvent::Failed(WorkflowError::msg("old lookup failure")),
    );
    assert!(state.inbox_analysis.error.is_none());
    assert!(!state.pending.contains_key(&current.0));
    state.selection = None;
    state.effort = None;
    state.vault_bound = false;
    state.inbox_analysis.request = None;
    assert!(!state.can_ask());
    let retained = state.inspect_inbox_analysis(request.id).unwrap();
    assert!(matches!(retained.1, AppCommand::InboxActionAnalysis(id) if id == request.id));
    state.apply(retained.0, AppEvent::InboxActionAnalysis(Box::new(exact)));
    assert!(
        state.inbox_analysis.record.is_some(),
        "historical lookup needs no model/provider selection"
    );
    let failed = state.inspect_inbox_analysis(Uuid::new_v4()).unwrap();
    state.apply(
        failed.0,
        AppEvent::Failed(WorkflowError::msg("analysis unavailable")),
    );
    assert_eq!(
        state.inbox_analysis.error.as_deref(),
        Some("analysis unavailable")
    );
    assert!(!state.inbox_analysis_loading());
    assert!(state.inspect_inbox_analysis(Uuid::nil()).is_none());
    f.preserved(&worker);
    worker.shutdown().unwrap();
}
