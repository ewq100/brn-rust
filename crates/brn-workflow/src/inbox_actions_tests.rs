//! Actual owned-worker Inbox Action analysis with synthetic model hooks only.
use super::*;
use crate::{
    actions::{ActionData, ActionListRequest, ActionPriority, ActionState},
    inbox::{CaptureInboxRequest, InboxItem, InboxKind, InboxOriginal},
    inbox_actions::{InboxActionAnalysis, InboxActionRequest},
    inbox_processing::{
        InboxCandidateRequest, InboxProcessOutcome, InboxSourceRequest, ProcessInboxRequest,
    },
    proposal_apply::{ApplyOutcome, ApprovalRequest},
    proposals::{ActionChange, ProposalEdit, ProposalSource, ProposalState},
};
use brn_ai::{
    ActionCandidate, ActionCandidateData, ActionCandidatePriority, ActionCandidateState,
    ActionProposalArgs, ReadScope,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicUsize;
use std::{panic::Location, time::Instant};

#[path = "inbox_actions_tests/budgets.rs"]
mod budgets;

// Match only enum variants: payloads can contain private evidence or credentials.
macro_rules! variant_name {
    ($value:expr, $kind:ident, [$($unit:ident),*], [$($tuple:ident),*], [$($fields:ident),*]) => {
        match $value {
            $($kind::$unit => stringify!($unit),)*
            $($kind::$tuple(..) => stringify!($tuple),)*
            $($kind::$fields { .. } => stringify!($fields),)*
        }
    };
}

fn command_name(command: &AppCommand) -> &'static str {
    variant_name!(
        command,
        AppCommand,
        [
            Status,
            BackupStatus,
            CheckpointBackup,
            Refresh,
            Selection,
            Effort,
            Editors,
            IdentityInventory,
            ProposalApplies,
            ProposalRecovery,
            Conversations,
            ModelPrompt
        ],
        [
            BindVault,
            Select,
            SelectEffort,
            Note,
            OpenEditor,
            ReloadEditor,
            RecoverEditor,
            SaveEditor,
            ReconcileEditor,
            ProposalSource,
            ProposalEvidenceSource,
            ProposalAsset,
            NoteIdentity,
            ResolveNoteIdentity,
            EvidenceNote,
            PrepareNoteIdentity,
            NoteProvenance,
            NoteLinks,
            PrepareNoteLink,
            Relationships,
            CaptureFinding,
            Actions,
            ActionDashboard,
            Action,
            CompleteAction,
            CaptureInbox,
            CaptureBinaryInbox,
            InboxItems,
            InboxItem,
            InboxReview,
            PreviewInboxRemoval,
            RemoveInboxOriginal,
            RestoreInboxOriginal,
            InboxOriginalRemoval,
            InboxOriginalRestore,
            InboxOriginalOperations,
            ArchivedInboxAnalysis,
            ProcessInbox,
            InboxProcessing,
            InboxCandidate,
            InboxExtraction,
            InboxRetainedExtractions,
            PrepareInboxSource,
            PrepareInboxVisualAnnotation,
            InboxVisualEvidence,
            CancelInboxProcessing,
            Findings,
            NoteConflicts,
            Finding,
            CloseFinding,
            InspectFinding,
            CaptureCitation,
            PrepareNoteProvenance,
            CreateProposal,
            Proposal,
            Proposals,
            EditProposal,
            AttachInboxKnowledgePredecessor,
            RewriteProposal,
            StartProposalRewrite,
            ProposalRewrite,
            AddProposalComment,
            UpdateProposalComment,
            RejectProposal,
            ApproveProposal,
            ReconcileProposal,
            PreviewProposalUndo,
            UndoProposal,
            PreviewProposalRepair,
            RepairProposal,
            ApproveProposalGroup,
            ProposalApply,
            Activity,
            ConversationSummaries,
            ConversationLifecycle,
            SetConversationLifecycle,
            Turns,
            Turn,
            RunBudget,
            Ask,
            AnalyzeInboxActions,
            InboxActionAnalysis,
            CancelTurn,
            CancelAccount,
            CancelModelDownload
        ],
        [
            TestPause,
            Notes,
            ScopedNotes,
            ScopedNote,
            RemoveProposalComment,
            RecoverEdit,
            Search,
            ScopedSearch,
            InboxIntakeBinding,
            Account,
            DownloadModel
        ]
    )
}

fn event_name(event: &AppEvent) -> &'static str {
    variant_name!(
        event,
        AppEvent,
        [
            VaultBound,
            SelectionSaved,
            EffortSaved,
            EditRecovered,
            ModelInstalled,
            ModelDownloadDeclined
        ],
        [
            Status,
            BackupStatus,
            Selection,
            Effort,
            Refreshed,
            Notes,
            Note,
            Editor,
            EditorRecovered,
            EditorSaved,
            Editors,
            ProposalSource,
            ProposalAsset,
            NoteIdentity,
            IdentityInventory,
            NoteIdentityResolved,
            EvidenceNote,
            NoteIdentityDraft,
            NoteProvenance,
            NoteLinks,
            NoteLinkDraft,
            Relationships,
            Finding,
            Action,
            Actions,
            ActionDashboard,
            ActionCompleted,
            InboxCaptured,
            InboxItems,
            InboxItem,
            InboxReview,
            InboxRemovalPreview,
            InboxOriginalRemoved,
            InboxOriginalRestored,
            InboxProcessing,
            InboxCandidate,
            InboxExtraction,
            InboxSourceDraft,
            InboxVisualDraft,
            InboxVisualEvidence,
            InboxIntakeBinding,
            InboxActionAnalysis,
            Findings,
            NoteConflicts,
            FindingInspection,
            CitationCaptured,
            NoteProvenanceDraft,
            Proposal,
            ProposalRewrite,
            Rewrite,
            Proposals,
            ProposalApplied,
            ProposalUndoPreview,
            ProposalRepairPreview,
            ProposalRepaired,
            ProposalGroupApplied,
            ProposalApplies,
            ProposalRecovery,
            ProposalApply,
            Activity,
            Search,
            Conversations,
            ConversationLifecycle,
            ConversationLifecycleChanged,
            Turns,
            Turn,
            Chat,
            Account,
            ModelPrompt,
            ModelDownloaded,
            Failed
        ],
        [
            ConversationSummaries,
            Ready,
            Restored,
            InboxOriginalRemoval,
            InboxOriginalRestore,
            InboxOriginalOperations,
            ArchivedInboxAnalysis,
            InboxRetainedExtractions,
            Indexing,
            TurnCancelRequested,
            AccountCancelRequested,
            ModelCancelRequested,
            ModelDownload,
            RunBudget
        ]
    )
}

struct OperationWait {
    operation: Uuid,
    command: &'static str,
    caller: &'static Location<'static>,
    started: Instant,
    ceiling: Duration,
    last_event: Option<(&'static str, Uuid)>,
}
impl OperationWait {
    #[track_caller]
    fn new(operation: Uuid, command: &'static str) -> Self {
        Self {
            operation,
            command,
            caller: Location::caller(),
            started: Instant::now(),
            ceiling: Duration::from_secs(10),
            last_event: None,
        }
    }

    fn event(&mut self, worker: &AppWorker) -> (Uuid, AppEvent) {
        let remaining = self.ceiling.saturating_sub(self.started.elapsed());
        let received = if remaining.is_zero() {
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        } else {
            worker.recv_event_timeout(remaining)
        };
        let (id, event) = received.unwrap_or_else(|error| {
            panic!(
                "worker wait failed: command={} operation={} caller={} elapsed={:?} ceiling={:?} last_event={:?} receive={error:?}",
                self.command, self.operation, self.caller, self.started.elapsed(), self.ceiling, self.last_event,
            )
        });
        self.last_event = Some((event_name(&event), id));
        (id, event)
    }
}

struct SourceFixture {
    source: ProposalSource,
    note_id: Uuid,
    original: InboxItem,
    raw: String,
}

#[track_caller]
fn reply_at(worker: &AppWorker, operation: Uuid, command: AppCommand) -> AppEvent {
    let mut wait = OperationWait::new(operation, command_name(&command));
    if let AppCommand::ApproveProposalGroup(request) = &command
        && request.validate().is_ok()
    {
        // A captured group executes its atomic approvals sequentially. Give
        // each member the ordinary test allowance, retaining one absolute
        // group deadline that unrelated events cannot extend.
        wait.ceiling *= request.approvals.len() as u32;
    }
    worker.submit(operation, command).unwrap();
    loop {
        let (id, value) = wait.event(worker);
        if id == operation {
            return value;
        }
    }
}

#[track_caller]
fn reply(worker: &AppWorker, command: AppCommand) -> AppEvent {
    reply_at(worker, Uuid::new_v4(), command)
}

#[track_caller]
fn capture_source(worker: &AppWorker, raw: &str) -> SourceFixture {
    let capture = CaptureInboxRequest {
        id: Uuid::new_v4(),
        kind: InboxKind::Markdown,
        title: "Synthetic Inbox evidence õ".into(),
        original_name: Some("deliberate-copy.md".into()),
        text: raw.into(),
    };
    let AppEvent::InboxCaptured(original) = reply_at(
        worker,
        capture.id,
        AppCommand::CaptureInbox(capture.clone()),
    ) else {
        panic!("capture response");
    };
    let process = ProcessInboxRequest {
        limits: None,

        id: Uuid::new_v4(),
        items: vec![(*original).clone()],
    };
    let mut wait = OperationWait::new(process.id, "ProcessInbox");
    worker
        .submit(process.id, AppCommand::ProcessInbox(process.clone()))
        .unwrap();
    loop {
        let (id, value) = wait.event(worker);
        if id != process.id {
            continue;
        }
        match value {
            AppEvent::InboxProcessing(batch) if batch.pending_count() == 0 => {
                assert!(matches!(
                    batch.entries[0].outcome,
                    InboxProcessOutcome::Converted { .. }
                ));
                break;
            }
            AppEvent::InboxProcessing(_) => {}
            AppEvent::Failed(error) => panic!("processing: {error:?}"),
            _ => panic!("processing response"),
        }
    }
    let request = InboxSourceRequest {
        candidate: InboxCandidateRequest {
            batch_id: process.id,
            index: 0,
        },
        proposal_id: Uuid::new_v4(),
        note_id: Uuid::new_v4(),
        path: "source.md".into(),
        title: "Exact Source to analyze".into(),
    };
    let AppEvent::InboxSourceDraft(draft) =
        reply(worker, AppCommand::PrepareInboxSource(request.clone()))
    else {
        panic!("Source preparation response");
    };
    request.validate_draft(&draft).unwrap();
    let AppEvent::Proposal(review) = reply(worker, AppCommand::CreateProposal(*draft)) else {
        panic!("Source review response");
    };
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    assert!(
        matches!(reply_at(worker, approval.operation_id, AppCommand::ApproveProposal(approval)),
            AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    let AppEvent::ProposalSource(source) = reply(worker, AppCommand::ProposalSource(request.path))
    else {
        panic!("complete Source proof response");
    };
    source.validate().unwrap();
    assert!(source.text.ends_with(raw));
    assert_eq!(
        brn_store::note_identity::read(&source.text).unwrap(),
        Some(request.note_id)
    );
    SourceFixture {
        source: *source,
        note_id: request.note_id,
        original: *original,
        raw: raw.into(),
    }
}

fn request(source: &SourceFixture) -> InboxActionRequest {
    InboxActionRequest {
        budget: None,
        intake: None,
        visual_asset: None,
        purpose: Default::default(),
        id: Uuid::new_v4(),
        conversation: None,
        source: Some(Box::new(source.source.clone())),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6-luna".into(),
        },
        effort: crate::ReasoningEffort::High,
        generation: 713,
    }
}

fn data(note_id: Uuid) -> ActionData {
    ActionData {
        title: "  Call Anna õ  ".into(),
        description: "\u{feff}Interpretation kept separate 🦀\r\n".into(),
        state: ActionState::Waiting,
        owner: Some("Anna Õ".into()),
        related_person: None,
        related_project: None,
        sources: vec![note_id],
        thread: None,
        due_on: Some("2028-02-29".into()),
        follow_up_on: Some("2028-03-01".into()),
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: Some(ActionPriority::High),
    }
}

fn candidate_data() -> ActionCandidateData {
    ActionCandidateData {
        title: "  Call Anna õ  ".into(),
        description: "\u{feff}Interpretation kept separate 🦀\r\n".into(),
        state: ActionCandidateState::Waiting,
        owner: Some("Anna Õ".into()),
        related_person: None,
        related_project: None,
        sources: vec![],
        thread: None,
        due_on: Some("2028-02-29".into()),
        follow_up_on: Some("2028-03-01".into()),
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: Some(ActionCandidatePriority::High),
    }
}

fn args() -> ActionProposalArgs {
    ActionProposalArgs {
        title: "Review one Inbox consequence".into(),
        source_paths: vec![],
        action_changes: vec![ActionCandidate::Create {
            data: candidate_data(),
        }],
    }
}

fn candidate_data_mut(input: &mut ActionProposalArgs) -> &mut ActionCandidateData {
    let ActionCandidate::Create { data } = &mut input.action_changes[0] else {
        panic!("Create fixture");
    };
    data
}

fn tool_reply(result: brn_ai::AiResult<Value>) -> Value {
    match result {
        Ok(value) => json!({"ok": value}),
        Err(error) => json!({"error": error.kind}),
    }
}

#[track_caller]
fn finish(
    worker: &AppWorker,
    request: &InboxActionRequest,
) -> std::result::Result<WorkTurn, WorkflowError> {
    let mut wait = OperationWait::new(request.id, "AnalyzeInboxActions");
    loop {
        let (outer, value) = wait.event(worker);
        if outer != request.id {
            continue;
        }
        match value {
            AppEvent::Chat(chat) => {
                assert_eq!(chat.id(), request.id);
                assert_eq!(chat.generation(), request.generation);
                match chat {
                    ChatEvent::Finished { turn, .. } => return Ok(turn),
                    ChatEvent::Rejected { error, .. } => return Err(error),
                    ChatEvent::PersistenceFailed { error, .. } => {
                        panic!("analysis persistence: {error:?}")
                    }
                    _ => {}
                }
            }
            AppEvent::Failed(error) => panic!("analysis response: {error:?}"),
            _ => panic!("unexpected analysis event"),
        }
    }
}

#[track_caller]
fn analyze(
    worker: &AppWorker,
    request: &InboxActionRequest,
) -> std::result::Result<WorkTurn, WorkflowError> {
    worker
        .submit(
            request.id,
            AppCommand::AnalyzeInboxActions(Box::new(request.clone())),
        )
        .unwrap();
    finish(worker, request)
}

fn analysis(worker: &AppWorker, id: Uuid) -> InboxActionAnalysis {
    let AppEvent::InboxActionAnalysis(value) = reply(worker, AppCommand::InboxActionAnalysis(id))
    else {
        panic!("analysis inspection response");
    };
    *value
}

fn no_actions(worker: &AppWorker) {
    assert!(
        matches!(reply(worker, AppCommand::Actions(ActionListRequest::default())),
            AppEvent::Actions(page) if page.entries.is_empty())
    );
}

fn retained_original(worker: &AppWorker, source: &SourceFixture) {
    let AppEvent::InboxItem(item) =
        reply(worker, AppCommand::InboxItem(source.original.capture.id))
    else {
        panic!("original inspection response");
    };
    assert_eq!(item.item, source.original);
    assert_eq!(
        item.original,
        InboxOriginal::Available {
            text: source.raw.clone()
        }
    );
}

fn no_credentials(fixture: &Fixture) {
    assert_eq!(
        std::fs::read_dir(fixture.base.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn inbox_action_worker_preserves_scope_group_complete_proof_and_exact_approval() {
    let fixture = Fixture::new();
    let steps = Arc::new(Mutex::new(Vec::<ActionProposalArgs>::new()));
    let script = steps.clone();
    let expected = Arc::new(Mutex::new(None::<ProposalSource>));
    let observed = expected.clone();
    let hook: ProposalAnswerHook = Arc::new(move |ask, _, reads, proposals, _, _| {
        let inputs = std::mem::take(&mut *script.lock().unwrap());
        let source = observed.lock().unwrap().clone().unwrap();
        Box::pin(async move {
            let results = tokio::task::spawn_blocking(move || {
                let evidence: Value =
                    serde_json::from_str(ask.question.rsplit_once("\n\n").unwrap().1).unwrap();
                assert_eq!(evidence["source_text"], json!(source.text));
                assert_eq!(reads.read_note("a.md").unwrap().text, "current");
                assert!(reads.read_note(&source.source.path).is_err());
                let note = reads
                    .read_note_scoped(&source.source.path, ReadScope::Source)
                    .unwrap();
                assert_eq!(note.text, source.text);
                assert!(!note.truncated);
                assert!(
                    reads.list_actions(None, 20, None).unwrap()["entries"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                );
                inputs
                    .into_iter()
                    .map(|input| tool_reply(proposals.propose_actions(input)))
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&results).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(
        &worker,
        "\u{feff}Original exact õ 🦀\r\nCall Anna; confirm date.\r\n",
    );
    *expected.lock().unwrap() = Some(source.source.clone());
    let mut valid = args();
    valid.source_paths.push("a.md".into());
    let mut repeated_selected = args();
    repeated_selected.source_paths.push("source.md".into());
    let mut repeated_additional = args();
    repeated_additional.source_paths = vec!["a.md".into(), "a.md".into()];
    let mut duplicate_source_uuid = args();
    candidate_data_mut(&mut duplicate_source_uuid).sources =
        vec![source.note_id.to_string(), source.note_id.to_string()];
    let mut multiple = args();
    multiple.action_changes.push(ActionCandidate::Create {
        data: candidate_data(),
    });
    let mut with_source_uuid = valid.clone();
    candidate_data_mut(&mut with_source_uuid).sources = vec![source.note_id.to_string()];
    *steps.lock().unwrap() = vec![
        repeated_selected,
        repeated_additional,
        duplicate_source_uuid,
        multiple,
        valid,
        with_source_uuid,
    ];
    let request = request(&source);
    let turn = analyze(&worker, &request).unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Completed);
    let results: Vec<Value> = serde_json::from_str(&turn.answer).unwrap();
    for refusal in &results[..4] {
        assert_eq!(refusal["error"], json!(AiErrorKind::ToolRejected));
    }
    let inspected = analysis(&worker, request.id);
    assert!(inspected.needs_semantic_review);
    assert_eq!(json!(inspected.turn), json!(Some(&turn)));
    assert_eq!(
        inspected.job.capture.source,
        Some(source.source.source.clone())
    );
    assert_eq!(inspected.job.capture.source_text, source.source.text);
    assert_eq!(inspected.job.capture.effort, "high");
    assert_eq!(inspected.proposals.len(), 2);
    assert!(results[4..].iter().all(|result| result.get("ok").is_some()));
    let proposal_id = Uuid::parse_str(results[4]["ok"]["stamp"]["id"].as_str().unwrap()).unwrap();
    let action_id = Uuid::parse_str(results[4]["ok"]["action_ids"][0].as_str().unwrap()).unwrap();
    let review = inspected
        .proposals
        .iter()
        .find(|review| review.draft.id == proposal_id)
        .unwrap();
    assert_eq!(review.state, ProposalState::Draft);
    assert_eq!(proposal_id.get_version_num(), 8);
    assert_eq!(action_id.get_version_num(), 8);
    assert_ne!(proposal_id, action_id);
    assert_ne!(source.note_id, action_id);
    for result in &results[4..] {
        let id = Uuid::parse_str(result["ok"]["stamp"]["id"].as_str().unwrap()).unwrap();
        let record = inspected
            .proposals
            .iter()
            .find(|record| record.draft.id == id)
            .unwrap();
        assert_eq!(
            record.draft.action_changes[0].data().sources,
            vec![source.note_id]
        );
        assert_eq!(record.draft.sources[0], source.source.source);
        assert_eq!(
            result["ok"]["action_ids"],
            json!([record.draft.action_changes[0].id()])
        );
    }
    assert_ne!(
        results[4]["ok"]["stamp"]["id"],
        results[5]["ok"]["stamp"]["id"]
    );
    assert_ne!(
        results[4]["ok"]["action_ids"],
        results[5]["ok"]["action_ids"]
    );
    assert_eq!(review.draft.group_id, Some(request.id));
    assert_eq!(review.draft.session_id, Some(turn.conversation_id));
    assert_eq!(review.draft.sources[0], source.source.source);
    assert_eq!(review.draft.sources[1].path, "a.md");
    assert_eq!(
        review.draft.action_changes,
        vec![ActionChange::Create {
            id: action_id,
            data: data(source.note_id),
        }]
    );
    assert!(review.draft.changes.is_empty());
    assert!(review.draft.inbox_source.is_none());
    assert_eq!(results[4]["ok"]["group_id"], json!(request.id));
    assert_eq!(results[4]["ok"]["sources"], json!(review.draft.sources));
    no_actions(&worker);
    let stale = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: crate::proposals::ProposalStamp {
            id: proposal_id,
            version: review.version + 1,
        },
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
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    assert!(
        matches!(reply_at(&worker, approval.operation_id, AppCommand::ApproveProposal(approval)),
        AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    let AppEvent::Action(action) = reply(&worker, AppCommand::Action(action_id)) else {
        panic!("approved real Action response");
    };
    assert_eq!(action.data, data(source.note_id));
    assert_eq!(action.origin.data, data(source.note_id));
    assert_eq!(action.origin.proposal, review.stamp());
    assert_eq!(
        std::fs::read(fixture.base.path().join("vault/source.md")).unwrap(),
        source.source.text.as_bytes()
    );
    assert_eq!(
        std::fs::read(fixture.base.path().join("vault/a.md")).unwrap(),
        b"current"
    );
    retained_original(&worker, &source);
    no_credentials(&fixture);
    worker.shutdown().unwrap();
}

#[test]
fn inbox_action_injected_source_counts_toward_complete_proof_limit() {
    let fixture = Fixture::new();
    let steps = Arc::new(Mutex::new(Vec::<ActionProposalArgs>::new()));
    let script = steps.clone();
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
        let inputs = std::mem::take(&mut *script.lock().unwrap());
        Box::pin(async move {
            let out = tokio::task::spawn_blocking(move || {
                inputs
                    .into_iter()
                    .map(|input| tool_reply(proposals.propose_actions(input)))
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&out).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(&worker, "Exact selected Source plus bounded evidence\r\n");
    let paths = (0..crate::proposals::MAX_PROPOSAL_CHANGES)
        .map(|index| {
            let path = format!("evidence-{index}.md");
            std::fs::write(
                fixture.base.path().join("vault").join(&path),
                format!("Evidence {index} õ\r\n"),
            )
            .unwrap();
            path
        })
        .collect::<Vec<_>>();
    let mut overflow = args();
    overflow.source_paths = paths.clone();
    let mut maximum = overflow.clone();
    maximum.source_paths.pop();
    *steps.lock().unwrap() = vec![overflow, maximum];
    let request = request(&source);
    let turn = analyze(&worker, &request).unwrap();
    let out: Vec<Value> = serde_json::from_str(&turn.answer).unwrap();
    assert_eq!(out[0]["error"], json!(AiErrorKind::ToolRejected));
    assert!(out[1].get("ok").is_some(), "{}", turn.answer);
    let records = analysis(&worker, request.id).proposals;
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(
        record.draft.sources.len(),
        crate::proposals::MAX_PROPOSAL_CHANGES
    );
    assert_eq!(record.draft.sources[0], source.source.source);
    assert_eq!(
        record.draft.sources[1..]
            .iter()
            .map(|proof| &proof.path)
            .collect::<Vec<_>>(),
        paths[..paths.len() - 1].iter().collect::<Vec<_>>()
    );
    assert_eq!(
        record.draft.action_changes[0].data().sources,
        vec![source.note_id]
    );
    assert_eq!(out[1]["ok"]["stamp"], json!(record.stamp()));
    assert_eq!(
        out[1]["ok"]["action_ids"],
        json!([record.draft.action_changes[0].id()])
    );
    no_actions(&worker);
    retained_original(&worker, &source);
    no_credentials(&fixture);
    worker.shutdown().unwrap();
}

#[test]
fn inbox_action_exact_restart_replay_survives_source_loss_and_rejects_changed_capture() {
    let fixture = Fixture::new();
    let input = Arc::new(Mutex::new(None::<ActionProposalArgs>));
    let scripted = input.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let called = calls.clone();
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
        called.fetch_add(1, Ordering::SeqCst);
        let input = scripted.lock().unwrap().clone().unwrap();
        Box::pin(async move {
            let receipt =
                tokio::task::spawn_blocking(move || proposals.propose_actions(input).unwrap())
                    .await
                    .unwrap();
            AiAnswer {
                text: serde_json::to_string(&receipt).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let hooks = Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    };
    let mut worker = fixture.start(hooks.clone());
    let source = capture_source(&worker, "Retain this exact evidence õ\r\n");
    *input.lock().unwrap() = Some(args());
    let request = request(&source);
    let turn = analyze(&worker, &request).unwrap();
    let before = analysis(&worker, request.id);
    assert!(before.needs_semantic_review);
    assert_eq!(before.proposals.len(), 1);
    assert_eq!(before.proposals[0].draft.group_id, Some(request.id));
    let mut current_presentation = request.clone();
    current_presentation.generation += 97;
    assert_eq!(
        json!(analyze(&worker, &current_presentation).unwrap()),
        json!(turn)
    );
    assert_eq!(json!(analysis(&worker, request.id)), json!(before));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    worker.shutdown().unwrap();
    std::fs::remove_file(fixture.base.path().join("vault/source.md")).unwrap();
    let mut worker = fixture.start(hooks);
    let mut replay = request.clone();
    replay.generation += 1;
    assert_eq!(json!(analyze(&worker, &replay).unwrap()), json!(turn));
    assert_eq!(json!(analysis(&worker, request.id)), json!(before));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let mut changed_source = replay.clone();
    changed_source
        .source
        .as_mut()
        .unwrap()
        .text
        .push_str("Different capture\r\n");
    changed_source
        .source
        .as_mut()
        .unwrap()
        .source
        .fingerprint
        .len = changed_source.source.as_mut().unwrap().text.len() as u64;
    changed_source
        .source
        .as_mut()
        .unwrap()
        .source
        .fingerprint
        .sha256 = Sha256::digest(changed_source.source.as_mut().unwrap().text.as_bytes()).into();
    let mut changed_selection = replay.clone();
    changed_selection.selection.model = "gpt-5.5".into();
    let mut changed_effort = replay.clone();
    changed_effort.effort = crate::ReasoningEffort::Medium;
    for changed in [changed_source, changed_selection, changed_effort] {
        assert_eq!(
            analyze(&worker, &changed).unwrap_err().kind,
            ErrorKind::OperationConflict
        );
    }
    let reserved = AskRequest {
        budget: None,
        id: replay.id,
        conversation: replay.conversation,
        question: before.job.question.clone(),
        selection: replay.selection.clone(),
        effort: Some(replay.effort),
        generation: replay.generation,
    };
    worker
        .submit(reserved.id, AppCommand::Ask(reserved))
        .unwrap();
    assert_eq!(
        finish(&worker, &replay).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(json!(analysis(&worker, request.id)), json!(before));
    no_actions(&worker);
    retained_original(&worker, &source);
    no_credentials(&fixture);
    worker.shutdown().unwrap();
}

#[test]
fn inbox_action_fresh_consequence_refuses_source_changed_during_the_turn() {
    let fixture = Fixture::new();
    let input = Arc::new(Mutex::new(None::<ActionProposalArgs>));
    let scripted = input.clone();
    let path = fixture.base.path().join("vault/source.md");
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
        let input = scripted.lock().unwrap().clone().unwrap();
        let path = path.clone();
        Box::pin(async move {
            let result = tokio::task::spawn_blocking(move || {
                let mut changed = std::fs::read_to_string(&path).unwrap();
                changed.push_str("Changed after admission õ\r\n");
                std::fs::write(path, changed).unwrap();
                tool_reply(proposals.propose_actions(input))
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&result).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(&worker, "Original selected evidence\r\n");
    *input.lock().unwrap() = Some(args());
    let request = request(&source);
    let turn = analyze(&worker, &request).unwrap();
    let result: Value = serde_json::from_str(&turn.answer).unwrap();
    assert_eq!(result["error"], json!(AiErrorKind::IndexStale));
    let inspected = analysis(&worker, request.id);
    assert_eq!(
        inspected.job.capture.source,
        Some(source.source.source.clone())
    );
    assert!(inspected.proposals.is_empty());
    assert!(inspected.needs_semantic_review);
    no_actions(&worker);
    retained_original(&worker, &source);
    no_credentials(&fixture);
    worker.shutdown().unwrap();
}

#[test]
fn inbox_action_creation_replay_keeps_newer_review_after_source_loss_on_the_bound_turn() {
    let fixture = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let called = calls.clone();
    let input = Arc::new(Mutex::new(None::<ActionProposalArgs>));
    let scripted = input.clone();
    let (created, ready) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let resume = Arc::new(Mutex::new(resume));
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
        called.fetch_add(1, Ordering::SeqCst);
        let input = scripted.lock().unwrap().clone().unwrap();
        let created = created.clone();
        let resume = resume.clone();
        Box::pin(async move {
            let results = tokio::task::spawn_blocking(move || {
                let first = proposals.propose_actions(input.clone()).unwrap();
                created.send(first).unwrap();
                resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
                let replay = tool_reply(proposals.propose_actions(input.clone()));
                let mut fresh = input.clone();
                candidate_data_mut(&mut fresh)
                    .description
                    .push_str("Different consequence");
                let refused = tool_reply(proposals.propose_actions(fresh));
                let mut changed = input;
                changed.title.push_str(" changed");
                vec![
                    replay,
                    refused,
                    tool_reply(proposals.propose_actions(changed)),
                ]
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&results).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(&worker, "One proposed consequence\r\n");
    *input.lock().unwrap() = Some(args());
    let request = request(&source);
    worker
        .submit(
            request.id,
            AppCommand::AnalyzeInboxActions(Box::new(request.clone())),
        )
        .unwrap();
    let first = ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let proposal_id = Uuid::parse_str(first["stamp"]["id"].as_str().unwrap()).unwrap();
    let captured = analysis(&worker, request.id).job;
    let mut current_presentation = request.clone();
    current_presentation.generation += 17;
    let mut replay_wait = OperationWait::new(request.id, "AnalyzeInboxActions");
    worker
        .submit(
            request.id,
            AppCommand::AnalyzeInboxActions(Box::new(current_presentation.clone())),
        )
        .unwrap();
    loop {
        let (outer, value) = replay_wait.event(&worker);
        if outer != request.id {
            continue;
        }
        let AppEvent::Chat(ChatEvent::AlreadyRunning {
            id,
            generation,
            turn,
        }) = value
        else {
            panic!("running replay must acknowledge the retained turn");
        };
        assert_eq!(id, request.id);
        assert_eq!(generation, current_presentation.generation);
        assert_eq!(turn.status, WorkTurnStatus::Running);
        break;
    }
    assert_eq!(analysis(&worker, request.id).job, captured);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let AppEvent::Proposal(review) = reply(&worker, AppCommand::Proposal(proposal_id)) else {
        panic!("retained review");
    };
    assert_eq!(first["stamp"], json!(review.stamp()));
    let mut newer = data(source.note_id);
    newer.description = "Later exact manual review õ\r\n".into();
    let AppEvent::Proposal(edited) = reply(
        &worker,
        AppCommand::EditProposal(ProposalEdit {
            expected: review.stamp(),
            title: "Later manual title".into(),
            texts: vec![],
            action_data: vec![newer],
        }),
    ) else {
        panic!("manual review edit");
    };
    assert_eq!(edited.version, review.version + 1);
    std::fs::remove_file(fixture.base.path().join("vault/source.md")).unwrap();
    release.send(()).unwrap();
    let turn = finish(&worker, &request).unwrap();
    let results: Vec<Value> = serde_json::from_str(&turn.answer).unwrap();
    assert_eq!(results[0]["ok"]["stamp"], json!(edited.stamp()));
    assert!(results[1].get("error").is_some());
    assert_eq!(results[2]["error"], json!(AiErrorKind::IndexStale));
    let inspected = analysis(&worker, request.id);
    assert_eq!(inspected.proposals, vec![edited]);
    assert_eq!(inspected.job, captured);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(inspected.needs_semantic_review);
    no_actions(&worker);
    retained_original(&worker, &source);
    no_credentials(&fixture);
    worker.shutdown().unwrap();
}

#[test]
fn inbox_action_twenty_separate_proposals_refuse_twenty_first_and_allow_exact_replay_at_cap() {
    let fixture = Fixture::new();
    let steps = Arc::new(Mutex::new(Vec::<ActionProposalArgs>::new()));
    let scripted = steps.clone();
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
        let inputs = std::mem::take(&mut *scripted.lock().unwrap());
        Box::pin(async move {
            let results = tokio::task::spawn_blocking(move || {
                inputs
                    .into_iter()
                    .map(|input| tool_reply(proposals.propose_actions(input)))
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&results).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(&worker, "Bounded list of potential follow-ups\r\n");
    let mut inputs = (0..21)
        .map(|index| {
            let mut input = args();
            input.title = format!("Review Inbox consequence {index}");
            input
        })
        .collect::<Vec<_>>();
    inputs.push(inputs[0].clone());
    *steps.lock().unwrap() = inputs;
    let request = request(&source);
    let turn = analyze(&worker, &request).unwrap();
    let results: Vec<Value> = serde_json::from_str(&turn.answer).unwrap();
    assert_eq!(results.len(), 22);
    assert!(
        results[..20]
            .iter()
            .all(|result| result.get("ok").is_some())
    );
    assert_eq!(results[20]["error"], json!(AiErrorKind::ToolRejected));
    assert_eq!(results[21], results[0]);
    let inspected = analysis(&worker, request.id);
    assert_eq!(inspected.proposals.len(), 20);
    assert!(
        inspected
            .proposals
            .iter()
            .all(|review| review.state == ProposalState::Draft
                && review.draft.group_id == Some(request.id)
                && review.draft.action_changes.len() == 1
                && review.draft.sources == vec![source.source.source.clone()])
    );
    let mut ids = std::collections::HashSet::new();
    ids.insert(source.note_id);
    for result in &results[..20] {
        let proposal_id = Uuid::parse_str(result["ok"]["stamp"]["id"].as_str().unwrap()).unwrap();
        let action_id = Uuid::parse_str(result["ok"]["action_ids"][0].as_str().unwrap()).unwrap();
        assert!(ids.insert(proposal_id));
        assert!(ids.insert(action_id));
        let record = inspected
            .proposals
            .iter()
            .find(|record| record.draft.id == proposal_id)
            .unwrap();
        assert_eq!(record.draft.action_changes[0].id(), action_id);
    }
    let mut same_intent = args();
    same_intent.title = "Review Inbox consequence 0".into();
    *steps.lock().unwrap() = vec![same_intent];
    let other = self::request(&source);
    let other_turn = analyze(&worker, &other).unwrap();
    let other_result: Vec<Value> = serde_json::from_str(&other_turn.answer).unwrap();
    assert!(other_result[0].get("ok").is_some(), "{}", other_turn.answer);
    let other_record = analysis(&worker, other.id).proposals.remove(0);
    assert!(ids.insert(other_record.draft.id));
    assert!(ids.insert(other_record.draft.action_changes[0].id()));
    assert_eq!(other_record.draft.group_id, Some(other.id));
    assert_eq!(other_result[0]["ok"]["stamp"], json!(other_record.stamp()));
    assert!(inspected.needs_semantic_review);
    no_actions(&worker);
    retained_original(&worker, &source);
    no_credentials(&fixture);
    worker.shutdown().unwrap();
}

#[test]
fn inbox_action_no_action_failed_and_cancelled_turns_keep_original_and_pending_semantic_review() {
    for outcome in ["no_action", "failed", "cancelled"] {
        let stop = outcome == "cancelled";
        let fixture = Fixture::new();
        let (started, ready) = mpsc::channel();
        let hook: AnswerHook = Arc::new(move |_, _, tools, cancel, emit| {
            let started = started.clone();
            Box::pin(async move {
                let partial = "No supported Action. Identity and date remain unresolved õ\r\n";
                emit(AiEvent::Text(partial.into()));
                started.send(()).unwrap();
                if stop {
                    cancel.cancelled().await;
                }
                drop(tools);
                AiAnswer {
                    text: partial.into(),
                    terminal: if outcome == "no_action" {
                        AiTerminal::Completed
                    } else {
                        AiTerminal::Failed(AiError::new(AiErrorKind::Other))
                    },
                }
            })
        });
        let mut worker = fixture.start(Hooks {
            answer: Some(hook),
            ..Hooks::default()
        });
        let source = capture_source(&worker, "Original requiring further interpretation\r\n");
        let request = request(&source);
        worker
            .submit(
                request.id,
                AppCommand::AnalyzeInboxActions(Box::new(request.clone())),
            )
            .unwrap();
        ready.recv_timeout(Duration::from_secs(10)).unwrap();
        if stop {
            worker
                .submit(Uuid::new_v4(), AppCommand::CancelTurn(request.id))
                .unwrap();
        }
        let turn = finish(&worker, &request).unwrap();
        assert_eq!(
            turn.status,
            if stop {
                WorkTurnStatus::Interrupted
            } else if outcome == "no_action" {
                WorkTurnStatus::Completed
            } else {
                WorkTurnStatus::Failed
            }
        );
        assert_eq!(
            turn.answer,
            "No supported Action. Identity and date remain unresolved õ\r\n"
        );
        let inspected = analysis(&worker, request.id);
        assert_eq!(json!(inspected.turn), json!(Some(&turn)));
        assert_eq!(
            inspected.job.capture.source,
            Some(source.source.source.clone())
        );
        assert!(inspected.proposals.is_empty());
        assert!(inspected.needs_semantic_review);
        no_actions(&worker);
        retained_original(&worker, &source);
        assert_eq!(
            std::fs::read(fixture.base.path().join("vault/source.md")).unwrap(),
            source.source.text.as_bytes()
        );
        no_credentials(&fixture);
        worker.shutdown().unwrap();
    }
}

#[test]
fn inbox_action_oversized_malformed_forged_and_ambiguous_sources_refuse_before_model_or_job() {
    for case in ["oversized", "malformed", "forged", "ambiguous"] {
        let fixture = Fixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let called = calls.clone();
        let hook: AnswerHook = Arc::new(move |_, _, _, _, _| {
            called.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                AiAnswer {
                    text: "Unexpected model admission".into(),
                    terminal: AiTerminal::Completed,
                }
            })
        });
        let mut worker = fixture.start(Hooks {
            answer: Some(hook),
            ..Hooks::default()
        });
        let raw = if case == "oversized" {
            "x".repeat(brn_ai::READ_NOTE_BYTES - 1)
        } else {
            "Source admission gate\r\n".into()
        };
        let mut source = capture_source(&worker, &raw);
        match case {
            "oversized" => {
                assert!(source.raw.len() < brn_ai::READ_NOTE_BYTES);
                assert!(source.source.text.len() > brn_ai::READ_NOTE_BYTES);
            }
            "malformed" => {
                let malformed = source
                    .source
                    .text
                    .replace(&format!("brn_id: {}", source.note_id), "brn_id: not-a-uuid");
                assert_ne!(malformed, source.source.text);
                std::fs::write(fixture.base.path().join("vault/source.md"), malformed).unwrap();
                let AppEvent::ProposalSource(proof) =
                    reply(&worker, AppCommand::ProposalSource("source.md".into()))
                else {
                    panic!("complete malformed proof");
                };
                source.source = *proof;
                source.source.validate().unwrap();
            }
            "ambiguous" => {
                std::fs::write(
                    fixture.base.path().join("vault/duplicate.md"),
                    &source.source.text,
                )
                .unwrap();
            }
            "forged" => {
                source.source.source.fingerprint.inode += 1;
                source.source.validate().unwrap();
            }
            _ => unreachable!(),
        }
        let request = request(&source);
        let error = analyze(&worker, &request).unwrap_err();
        if matches!(case, "ambiguous" | "forged") {
            assert_eq!(error.kind, ErrorKind::ContextStale);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0, "{case}");
        assert!(
            matches!(reply(&worker, AppCommand::InboxActionAnalysis(request.id)),
            AppEvent::Failed(error) if error.kind == ErrorKind::NotFound)
        );
        assert!(matches!(
            reply(&worker, AppCommand::Turn(request.id)),
            AppEvent::Turn(None)
        ));
        no_actions(&worker);
        retained_original(&worker, &source);
        no_credentials(&fixture);
        worker.shutdown().unwrap();
    }
}

#[test]
fn inbox_action_explicit_historical_source_stays_evidence_and_requires_semantic_review() {
    let fixture = Fixture::new();
    let expected = Arc::new(Mutex::new(None::<ProposalSource>));
    let observed = expected.clone();
    let hook: AnswerHook = Arc::new(move |ask, _, reads, _, _| {
        let source = observed.lock().unwrap().clone().unwrap();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                assert!(ask.question.contains("\"historical_source\":true"));
                assert!(
                    ask.question
                        .contains("Historical source does not establish current truth")
                );
                assert_eq!(reads.read_note("a.md").unwrap().text, "current");
                assert!(reads.read_note(&source.source.path).is_err());
                assert_eq!(
                    reads
                        .read_note_scoped(&source.source.path, ReadScope::History)
                        .unwrap()
                        .text,
                    source.text
                );
                assert_eq!(
                    reads
                        .read_note_scoped(&source.source.path, ReadScope::Source)
                        .unwrap()
                        .text,
                    source.text
                );
            })
            .await
            .unwrap();
            AiAnswer {
                text: "No Action supported by this historical evidence alone.".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        answer: Some(hook),
        ..Hooks::default()
    });
    let mut source = capture_source(&worker, "Historical copied wording õ\r\n");
    let original_proof = source.source.clone();
    std::fs::create_dir(fixture.base.path().join("vault/archive")).unwrap();
    std::fs::rename(
        fixture.base.path().join("vault/source.md"),
        fixture.base.path().join("vault/archive/source.md"),
    )
    .unwrap();
    let AppEvent::ProposalSource(proof) = reply(
        &worker,
        AppCommand::ProposalEvidenceSource("archive/source.md".into()),
    ) else {
        panic!("complete historical proof");
    };
    source.source = *proof;
    assert_eq!(source.source.text, original_proof.text);
    assert_eq!(
        source.source.source.fingerprint,
        original_proof.source.fingerprint
    );
    *expected.lock().unwrap() = Some(source.source.clone());
    let request = request(&source);
    assert_eq!(
        analyze(&worker, &request).unwrap().status,
        WorkTurnStatus::Completed
    );
    let inspected = analysis(&worker, request.id);
    assert!(inspected.needs_semantic_review);
    assert!(inspected.proposals.is_empty());
    no_actions(&worker);
    retained_original(&worker, &source);
    no_credentials(&fixture);
    worker.shutdown().unwrap();
}

#[path = "inbox_actions_tests/knowledge.rs"]
mod knowledge_tests;

#[path = "inbox_actions_tests/visual.rs"]
mod visual_tests;
