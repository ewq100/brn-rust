//! Ask-bound proposal callbacks to the application owner; no separate Store writer.
use super::*;
use crate::proposals::{ActionChange, DraftRequest, ProposalStamp, ProposalState, SourceVersion};
use brn_ai::{
    ActionCandidate, ActionCandidateData, ActionCandidatePriority, ActionCandidateState,
    ActionProposalArgs, ActionRef, AiError, AiErrorKind, AiResult, ProposalTools,
};
use brn_store::work::WorkTurnStatus;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) struct ActionProposals {
    tx: mpsc::Sender<Message>,
    admission: Arc<Mutex<()>>,
    stopping: Arc<AtomicBool>,
}
impl ActionProposals {
    pub(super) fn new(
        tx: mpsc::Sender<Message>,
        admission: Arc<Mutex<()>>,
        stopping: Arc<AtomicBool>,
    ) -> Self {
        Self {
            tx,
            admission,
            stopping,
        }
    }
    pub(crate) fn bind(
        &self,
        request: &AskRequest,
        turn: &WorkTurn,
        cancel: CancellationToken,
    ) -> Arc<dyn ProposalTools> {
        self.bind_inbox(request, turn, cancel, None)
    }
    pub(crate) fn bind_inbox(
        &self,
        request: &AskRequest,
        turn: &WorkTurn,
        cancel: CancellationToken,
        inbox: Option<Box<crate::inbox_actions::InboxActionJob>>,
    ) -> Arc<dyn ProposalTools> {
        Arc::new(BoundProposal {
            owner: self.clone(),
            request: request.clone(),
            turn: turn.clone(),
            cancel,
            inbox,
        })
    }
}
struct BoundProposal {
    owner: ActionProposals,
    request: AskRequest,
    turn: WorkTurn,
    cancel: CancellationToken,
    inbox: Option<Box<crate::inbox_actions::InboxActionJob>>,
}
pub(super) struct ActionProposal {
    args: ActionProposalArgs,
    request: AskRequest,
    turn: WorkTurn,
    reply: mpsc::Sender<AiResult<Value>>,
    inbox: Option<Box<crate::inbox_actions::InboxActionJob>>,
}
impl ProposalTools for BoundProposal {
    fn knowledge_enabled(&self) -> bool {
        self.inbox.as_ref().is_some_and(|job| {
            job.capture.purpose == crate::inbox_actions::InboxAnalysisPurpose::KnowledgeAndActions
        })
    }
    fn report_conflict(&self, args: brn_ai::ConflictArgs) -> AiResult<Value> {
        args.validate()?;
        if !self.knowledge_enabled() {
            return Err(rejected());
        }
        let (reply, rx) = mpsc::channel();
        {
            let _admission = self
                .owner
                .admission
                .lock()
                .map_err(|_| AiError::new(AiErrorKind::Other))?;
            if self.owner.stopping.load(Ordering::Acquire) || self.cancel.is_cancelled() {
                return Err(rejected());
            }
            self.owner
                .tx
                .send(Message::ConflictReport(Box::new(
                    super::conflict_proposals::ConflictReport {
                        args,
                        request: self.request.clone(),
                        turn: self.turn.clone(),
                        reply,
                        inbox: self
                            .inbox
                            .as_ref()
                            .expect("qualified conflict capability")
                            .clone(),
                    },
                )))
                .map_err(|_| AiError::new(AiErrorKind::Other))?;
        }
        rx.recv().map_err(|_| AiError::new(AiErrorKind::Other))?
    }
    fn propose_knowledge(&self, args: brn_ai::KnowledgeProposalArgs) -> AiResult<Value> {
        args.validate()?;
        if !self.knowledge_enabled() {
            return Err(rejected());
        }
        let (reply, rx) = mpsc::channel();
        {
            let _admission = self
                .owner
                .admission
                .lock()
                .map_err(|_| AiError::new(AiErrorKind::Other))?;
            if self.owner.stopping.load(Ordering::Acquire) || self.cancel.is_cancelled() {
                return Err(rejected());
            }
            self.owner
                .tx
                .send(Message::KnowledgeProposal(Box::new(
                    super::knowledge_proposals::KnowledgeProposal {
                        args,
                        request: self.request.clone(),
                        turn: self.turn.clone(),
                        reply,
                        inbox: self
                            .inbox
                            .as_ref()
                            .expect("qualified knowledge capability")
                            .clone(),
                    },
                )))
                .map_err(|_| AiError::new(AiErrorKind::Other))?;
        }
        rx.recv().map_err(|_| AiError::new(AiErrorKind::Other))?
    }

    fn propose_actions(&self, args: ActionProposalArgs) -> AiResult<Value> {
        self.enqueue(args)?
            .recv()
            .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}
impl BoundProposal {
    fn enqueue(&self, args: ActionProposalArgs) -> AiResult<mpsc::Receiver<AiResult<Value>>> {
        args.validate()?;
        let (reply, rx) = mpsc::channel();
        {
            let _admission = self
                .owner
                .admission
                .lock()
                .map_err(|_| AiError::new(AiErrorKind::Other))?;
            if self.owner.stopping.load(Ordering::Acquire) || self.cancel.is_cancelled() {
                return Err(AiError::new(AiErrorKind::ToolRejected));
            }
            self.owner
                .tx
                .send(Message::ActionProposal(Box::new(ActionProposal {
                    args,
                    request: self.request.clone(),
                    turn: self.turn.clone(),
                    reply,
                    inbox: self.inbox.clone(),
                })))
                .map_err(|_| AiError::new(AiErrorKind::Other))?;
        }
        Ok(rx)
    }
}
impl ActionProposal {
    pub(super) fn refuse(self, kind: AiErrorKind) {
        let _ = self.reply.send(Err(AiError::new(kind)));
    }
    pub(super) fn settle(self, app: &mut App) {
        // Admission already checked cancellation under the Stop fence. An admitted
        // mutation must settle even when Stop/Shutdown has since closed admission.
        let result = self.run(app);
        let _ = self.reply.send(result);
    }
    fn run(&self, app: &mut App) -> AiResult<Value> {
        self.args.validate()?;
        let actual = active_turn(app, &self.request, &self.turn)?;
        let id = candidate_id(b"brn-action-proposal-v1", actual.id, &self.args)?;
        let group_id = if let Some(job) = &self.inbox {
            if job.capture.id != self.request.id
                || job.question != self.request.question
                || app
                    .work_store()
                    .inbox_action(self.request.id)
                    .map_err(|e| safe(e.into()))?
                    .as_ref()
                    != Some(job.as_ref())
                || self.args.action_changes.len() != 1
            {
                return Err(rejected());
            }
            Some(job.capture.id)
        } else {
            None
        };

        // Replay resolves from retained full baselines and ordered proofs before
        // observing mutable files/Actions. The Store then checks creation input
        // and returns the newer review, never resetting it to candidate data.
        let existing = match app.proposal(id) {
            Ok(record) => Some(record),
            Err(e) if e.kind == ErrorKind::NotFound => None,
            Err(e) => return Err(safe(e)),
        };
        let paths = source_paths(&self.args, self.inbox.as_deref())?;
        let sources = if let Some(record) = &existing {
            if record.draft.group_id != group_id
                || record.draft.session_id != Some(actual.conversation_id)
                || !paths.iter().map(String::as_str).eq(record
                    .draft
                    .sources
                    .iter()
                    .map(|s| s.path.as_str()))
            {
                return Err(rejected());
            }
            record.draft.sources.clone()
        } else {
            if let Some(job) = &self.inbox {
                if inbox_consequence_count(app, job)?
                    >= crate::inbox_actions::MAX_INBOX_ACTION_PROPOSALS
                {
                    return Err(rejected());
                }
                app.validate_inbox_action_source(&job.capture)
                    .map_err(safe)?;
            }
            paths
                .iter()
                .map(|path| {
                    app.proposal_evidence_source(path)
                        .map(|s| s.source)
                        .map_err(safe)
                })
                .collect::<AiResult<Vec<_>>>()?
        };
        if let Some(job) = &self.inbox
            && sources.first() != Some(&job.capture.source)
        {
            return Err(rejected());
        }
        let member_ids = self
            .args
            .action_changes
            .iter()
            .enumerate()
            .map(|(index, change)| match change {
                ActionCandidate::Create { .. } => {
                    candidate_id(b"brn-action-member-v1", id, &(index + 1))
                }
                ActionCandidate::Replace { target, .. } => non_nil_uuid(&target.id),
            })
            .collect::<AiResult<Vec<_>>>()?;
        let source_id = self
            .inbox
            .as_ref()
            .map(|job| job.capture.note_id().map_err(|e| safe(e.into())))
            .transpose()?;
        let action_changes = self
            .args
            .action_changes
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let (data, before) = match candidate {
                    ActionCandidate::Create { data } => (data, None),
                    ActionCandidate::Replace { target, data } => {
                        let before = if let Some(record) = &existing {
                            match record.draft.action_changes.get(index) {
                                Some(ActionChange::Replace { before, .. }) => {
                                    before.as_ref().clone()
                                }
                                _ => return Err(rejected()),
                            }
                        } else {
                            app.action(member_ids[index]).map_err(safe)?
                        };
                        if super::action_tools::checked_reference(&before)? != *target {
                            return Err(rejected());
                        }
                        (data, Some(before))
                    }
                };
                let mut data = action_data(data, &member_ids, member_ids[index])?;
                // Validate duplicate source UUIDs before injecting the selected proof.
                // Keep exactly one mandatory Source first, then caller order.
                if let Some(source) = source_id {
                    data.sources.retain(|id| *id != source);
                    data.sources.insert(0, source);
                }
                let change = match before {
                    Some(before) => ActionChange::Replace {
                        before: Box::new(before),
                        data,
                    },
                    None => ActionChange::Create {
                        id: member_ids[index],
                        data,
                    },
                };
                change.validate().map_err(|_| rejected())?;
                Ok(change)
            })
            .collect::<AiResult<Vec<_>>>()?;
        let request = DraftRequest {
            inbox_knowledge: None,
            inbox_source: None,
            id,
            group_id,
            session_id: Some(actual.conversation_id),
            title: self.args.title.clone(),
            changes: vec![],
            sources,
            action_changes,
        };
        request.validate().map_err(safe)?;
        // Bound the complete receipt BEFORE creating durable review work. Maximal
        // stamp/state encodings cover all replay states and future review versions.
        let mut receipt = Receipt {
            group_id,
            stamp: ProposalStamp {
                id,
                version: u64::MAX,
            },
            state: ProposalState::Uncertain,
            session_id: request.session_id,
            action_ids: request
                .action_changes
                .iter()
                .map(ActionChange::id)
                .collect(),
            sources: request.sources.clone(),
        };
        if serde_json::to_vec(&receipt).map_err(|_| rejected())?.len() > brn_ai::READ_ACTION_BYTES {
            return Err(rejected());
        }
        let record = app.create_proposal(&request).map_err(safe)?;
        receipt.stamp = record.stamp();
        receipt.state = record.state;
        serde_json::to_value(receipt).map_err(|_| AiError::new(AiErrorKind::Storage))
    }
}

#[derive(Serialize)]
struct Receipt {
    #[serde(skip_serializing_if = "Option::is_none")]
    group_id: Option<Uuid>,
    stamp: ProposalStamp,
    state: ProposalState,
    session_id: Option<Uuid>,
    action_ids: Vec<Uuid>,
    sources: Vec<SourceVersion>,
}
// Exact ordered typed intent is scoped to this owned turn. Domains separate
// proposal and member UUIDs; UUID bits describe Rust-minted version8 identities.
fn candidate_id<T: Serialize>(domain: &[u8], owner: Uuid, value: &T) -> AiResult<Uuid> {
    let encoded = serde_json::to_vec(value).map_err(|_| rejected())?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(owner.as_bytes());
    digest.update(encoded);
    let hash = digest.finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(Uuid::from_bytes(bytes))
}
fn non_nil_uuid(text: &str) -> AiResult<Uuid> {
    let id = Uuid::parse_str(text).map_err(|_| rejected())?;
    if id.is_nil() {
        return Err(rejected());
    }
    Ok(id)
}
fn source_paths(
    args: &ActionProposalArgs,
    inbox: Option<&crate::inbox_actions::InboxActionJob>,
) -> AiResult<Vec<String>> {
    let mut paths = Vec::with_capacity(args.source_paths.len() + usize::from(inbox.is_some()));
    let mut seen = std::collections::HashSet::new();
    if let Some(job) = inbox {
        paths.push(job.capture.source.path.clone());
        seen.insert(job.capture.source.path.to_ascii_lowercase());
    }
    for path in &args.source_paths {
        if !seen.insert(path.to_ascii_lowercase()) {
            return Err(rejected());
        }
        paths.push(path.clone());
    }
    if paths.len() > crate::proposals::MAX_PROPOSAL_CHANGES {
        return Err(rejected());
    }
    Ok(paths)
}
fn action_ref(reference: &ActionRef, members: &[Uuid]) -> AiResult<Uuid> {
    match reference {
        ActionRef::Existing { id } => non_nil_uuid(id),
        ActionRef::Member { index } => index
            .checked_sub(1)
            .and_then(|index| members.get(index))
            .copied()
            .ok_or_else(rejected),
    }
}
fn action_data(
    input: &ActionCandidateData,
    members: &[Uuid],
    id: Uuid,
) -> AiResult<crate::actions::ActionData> {
    use crate::actions::{ActionData, ActionPriority, ActionState};
    let data = ActionData {
        title: input.title.clone(),
        description: input.description.clone(),
        state: match input.state {
            ActionCandidateState::Open => ActionState::Open,
            ActionCandidateState::Waiting => ActionState::Waiting,
            ActionCandidateState::Blocked => ActionState::Blocked,
        },
        owner: input.owner.clone(),
        related_person: input
            .related_person
            .as_deref()
            .map(non_nil_uuid)
            .transpose()?,
        related_project: input
            .related_project
            .as_deref()
            .map(non_nil_uuid)
            .transpose()?,
        sources: input
            .sources
            .iter()
            .map(|id| non_nil_uuid(id))
            .collect::<AiResult<Vec<_>>>()?,
        thread: input.thread.as_deref().map(non_nil_uuid).transpose()?,
        due_on: input.due_on.clone(),
        follow_up_on: input.follow_up_on.clone(),
        dependencies: input
            .dependencies
            .iter()
            .map(|reference| action_ref(reference, members))
            .collect::<AiResult<Vec<_>>>()?,
        parent: input
            .parent
            .as_ref()
            .map(|reference| action_ref(reference, members))
            .transpose()?,
        follows_up: input
            .follows_up
            .as_ref()
            .map(|reference| action_ref(reference, members))
            .transpose()?,
        priority: input.priority.map(|priority| match priority {
            ActionCandidatePriority::Low => ActionPriority::Low,
            ActionCandidatePriority::Normal => ActionPriority::Normal,
            ActionCandidatePriority::High => ActionPriority::High,
        }),
    };
    data.validate(id).map_err(|_| rejected())?;
    Ok(data)
}
pub(super) fn rejected() -> AiError {
    AiError::new(AiErrorKind::ToolRejected)
}
pub(super) fn safe(error: WorkflowError) -> AiError {
    AiError::new(match error.kind {
        ErrorKind::QuoteNotFound => AiErrorKind::QuoteNotFound,
        ErrorKind::QuoteAmbiguous => AiErrorKind::QuoteAmbiguous,
        ErrorKind::QuoteOccurrenceInvalid => AiErrorKind::QuoteOccurrenceInvalid,
        ErrorKind::ToolRejected | ErrorKind::OperationConflict | ErrorKind::NotFound => {
            AiErrorKind::ToolRejected
        }
        ErrorKind::SaveUncertain
        | ErrorKind::ContextStale
        | ErrorKind::IndexStale
        | ErrorKind::AiIndexStale => AiErrorKind::IndexStale,
        _ => AiErrorKind::Storage,
    })
}

pub(super) fn inbox_consequence_count(
    app: &App,
    job: &crate::inbox_actions::InboxActionJob,
) -> AiResult<usize> {
    let proposals = app.proposals(Some(job.capture.id)).map_err(safe)?.len();
    let findings =
        if job.capture.purpose == crate::inbox_actions::InboxAnalysisPurpose::KnowledgeAndActions {
            app.work_store()
                .inbox_conflicts(job.capture.id)
                .map_err(|e| safe(e.into()))?
                .len()
        } else {
            0
        };
    Ok(proposals + findings)
}

pub(super) fn active_turn(app: &App, request: &AskRequest, turn: &WorkTurn) -> AiResult<WorkTurn> {
    let actual = app
        .work_store()
        .turn(request.id)
        .map_err(|e| safe(e.into()))?
        .ok_or_else(rejected)?;
    if turn.id != request.id
        || actual.id != turn.id
        || actual.conversation_id != turn.conversation_id
        || actual.started_at_ms != turn.started_at_ms
        || actual.status != WorkTurnStatus::Running
        || turn.status != WorkTurnStatus::Running
    {
        return Err(rejected());
    }
    chat_worker::check_replay(request, turn).map_err(safe)?;
    chat_worker::check_replay(request, &actual).map_err(safe)?;

    Ok(actual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{actions::ActionListRequest, chat_worker::Hooks};
    use serde_json::json;
    use std::time::{Duration, Instant};
    fn args() -> ActionProposalArgs {
        serde_json::from_value(json!({"title":"Whole review õ\r\n","source_paths":[],"action_changes":[{
            "kind":"create","data":{"title":"Exact õ","description":"\u{feff}Original 🦀\r\n","state":"open","owner":null,"related_person":null,"related_project":null,"sources":[],"thread":null,"due_on":null,"follow_up_on":null,"dependencies":[],"parent":null,"follows_up":null,"priority":null}
        }]})).unwrap()
    }
    fn event(worker: &AppWorker) -> (Uuid, AppEvent) {
        worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
    }
    struct Active {
        base: tempfile::TempDir,
        worker: AppWorker,
        bound: BoundProposal,
        lease: Arc<dyn ProposalTools>,
        canceled: mpsc::Receiver<()>,
    }
    fn active() -> Active {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        std::fs::create_dir(base.path().join("data")).unwrap();
        std::fs::create_dir(base.path().join("vault")).unwrap();
        std::fs::write(base.path().join("vault/a.md"), b"current").unwrap();
        let config = config(base.path());
        let (sent, received) = mpsc::channel();
        let (noticed, canceled) = mpsc::channel();
        let hook: crate::simple_worker_tests::ProposalAnswerHook =
            Arc::new(move |_, _, reads, proposals, cancel, _| {
                drop(reads);
                sent.send((proposals, cancel.clone())).unwrap();
                let noticed = noticed.clone();
                Box::pin(async move {
                    cancel.cancelled().await;
                    noticed.send(()).unwrap();
                    brn_ai::AiAnswer {
                        text: "Stopped with retained proposal lease".into(),
                        terminal: brn_ai::AiTerminal::Interrupted,
                    }
                })
            });
        let worker = start_test(
            base.path().join("data"),
            config,
            Hooks {
                proposal_answer: Some(hook),
                ..Hooks::default()
            },
            None,
            None,
        )
        .unwrap();
        assert!(matches!(event(&worker).1, AppEvent::Ready { .. }));
        let request = AskRequest {
            id: Uuid::new_v4(),
            conversation: None,
            question: "Synthetic proposal".into(),
            selection: Selection {
                provider: Provider::Chatgpt,
                model: "gpt-6-luna".into(),
            },
            effort: Some(ReasoningEffort::Low),
            generation: 7,
        };
        worker
            .submit(request.id, AppCommand::Ask(request.clone()))
            .unwrap();
        let (lease, cancel) = received.recv_timeout(Duration::from_secs(10)).unwrap();
        let query = Uuid::new_v4();
        worker.submit(query, AppCommand::Turn(request.id)).unwrap();
        let turn = loop {
            let (id, e) = event(&worker);
            if id == query {
                break match e {
                    AppEvent::Turn(Some(t)) => t,
                    _ => panic!("wrong turn"),
                };
            }
        };
        let bound = BoundProposal {
            owner: ActionProposals::new(
                worker.tx.clone(),
                worker.admission.clone(),
                worker.stopping.clone(),
            ),
            request,
            turn,
            cancel,
            inbox: None,
        };
        Active {
            base,
            worker,
            bound,
            lease,
            canceled,
        }
    }
    fn pause(worker: &AppWorker) -> mpsc::Sender<()> {
        let (entered, wait) = mpsc::channel();
        let (release, held) = mpsc::channel();
        worker
            .submit(
                Uuid::new_v4(),
                AppCommand::TestPause {
                    entered,
                    release: held,
                },
            )
            .unwrap();
        wait.recv_timeout(Duration::from_secs(10)).unwrap();
        release
    }
    fn terminal(worker: &AppWorker, id: Uuid) -> WorkTurn {
        loop {
            let (_, e) = event(worker);
            if let AppEvent::Chat(ChatEvent::Finished {
                id: actual, turn, ..
            }) = e
                && actual == id
            {
                return turn;
            }
        }
    }
    fn config(base: &std::path::Path) -> AppConfig {
        AppConfig {
            vault_root: Some(base.join("vault")),
            credentials_dir: Some(base.join("credentials")),
            model_dir: None,
        }
    }
    fn persisted(base: &std::path::Path, id: Uuid, exists: bool) {
        let app = App::open(&base.join("data"), config(base)).unwrap();
        assert_eq!(app.proposal(id).is_ok(), exists);
        assert!(
            app.actions(&ActionListRequest::default())
                .unwrap()
                .entries
                .is_empty()
        );
        assert_eq!(std::fs::read(base.join("vault/a.md")).unwrap(), b"current");
        assert_eq!(
            std::fs::read_dir(base.join("credentials")).unwrap().count(),
            0
        );
    }
    #[test]
    fn proposal_admitted_before_stop_settles_and_only_proposal_lease_delays_terminal() {
        let mut a = active();
        let input = args();
        let id = candidate_id(b"brn-action-proposal-v1", a.bound.turn.id, &input).unwrap();
        let release = pause(&a.worker);
        // enqueue returns only after sending under the actual admission fence.
        let receipt = a.bound.enqueue(input.clone()).unwrap();
        let stop = Uuid::new_v4();
        a.worker
            .submit(stop, AppCommand::CancelTurn(a.bound.request.id))
            .unwrap();
        a.canceled.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(
            a.lease.propose_actions(args()).unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
        let status = Uuid::new_v4();
        a.worker.submit(status, AppCommand::Status).unwrap();
        release.send(()).unwrap();
        let value = receipt
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
        assert_eq!(value["session_id"], json!(a.bound.turn.conversation_id));
        let mut saw_stop = false;
        let mut saw_status = false;
        while !(saw_stop && saw_status) {
            let (actual, e) = event(&a.worker);
            match e {
                AppEvent::TurnCancelRequested { accepted, .. } => {
                    assert_eq!(actual, stop);
                    assert!(accepted);
                    saw_stop = true;
                }
                AppEvent::Status(_) => {
                    assert_eq!(actual, status);
                    saw_status = true;
                }
                AppEvent::Chat(ChatEvent::Finished { .. }) => {
                    panic!("proposal lease must delay terminal")
                }
                _ => {}
            }
        }
        let query = Uuid::new_v4();
        a.worker
            .submit(query, AppCommand::Turn(a.bound.request.id))
            .unwrap();
        loop {
            let (actual, e) = event(&a.worker);
            if actual == query {
                assert!(matches!(e,AppEvent::Turn(Some(t))if t.status==WorkTurnStatus::Running));
                break;
            }
        }
        drop(a.lease);
        assert_eq!(
            terminal(&a.worker, a.bound.request.id).status,
            WorkTurnStatus::Interrupted
        );
        a.worker.shutdown().unwrap();
        persisted(a.base.path(), id, true);
    }
    #[test]
    fn proposal_admitted_before_shutdown_settles_before_final_lease_and_restart() {
        let a = active();
        let input = args();
        let id = candidate_id(b"brn-action-proposal-v1", a.bound.turn.id, &input).unwrap();
        let release = pause(&a.worker);
        let receipt = a.bound.enqueue(input).unwrap();
        let closing = a.worker.stopping.clone();
        let (done, finished) = mpsc::channel();
        let join = thread::spawn(move || {
            let mut w = a.worker;
            let r = w.shutdown();
            done.send((w, r)).unwrap();
        });
        let deadline = Instant::now() + Duration::from_secs(10);
        while !closing.load(Ordering::Acquire) {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        assert_eq!(
            a.lease.propose_actions(args()).unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
        release.send(()).unwrap();
        assert!(
            receipt
                .recv_timeout(Duration::from_secs(10))
                .unwrap()
                .is_ok()
        );
        assert!(matches!(
            finished.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        drop(a.lease);
        let (w, r) = finished.recv_timeout(Duration::from_secs(10)).unwrap();
        r.unwrap();
        join.join().unwrap();
        assert_eq!(
            std::iter::from_fn(|| w.try_event())
                .filter(|(_, e)| matches!(e, AppEvent::Chat(ChatEvent::Finished { .. })))
                .count(),
            1
        );
        persisted(a.base.path(), id, true);
    }
    #[test]
    fn fatal_lane_refuses_queued_proposal_before_joining_retained_lease_and_preserves_prior_review()
    {
        let mut a = active();
        let first = args();
        let first_id = candidate_id(b"brn-action-proposal-v1", a.bound.turn.id, &first).unwrap();
        assert!(a.lease.propose_actions(first).is_ok());
        let release = pause(&a.worker);
        {
            let _fence = a.worker.admission.lock().unwrap();
            a.worker.tx.send(Message::FailLane).unwrap();
        }
        let mut failed = args();
        failed.title.push_str(" failure");
        let failed_id = candidate_id(b"brn-action-proposal-v1", a.bound.turn.id, &failed).unwrap();
        let receipt = a.bound.enqueue(failed).unwrap();
        let status = Uuid::new_v4();
        a.worker.submit(status, AppCommand::Status).unwrap();
        release.send(()).unwrap();
        let value = receipt.recv_timeout(Duration::from_secs(1));
        // Release the synthetic lease before asserting, even when witnessing RED.
        drop(a.lease);
        let failure = a.worker.shutdown().unwrap_err();
        assert_eq!(failure.kind, ErrorKind::Other);
        assert!(
            matches!(
                value,
                Ok(Err(AiError {
                    kind: AiErrorKind::Storage,
                    ..
                }))
            ),
            "{value:?}"
        );
        assert_eq!(
            a.bound.propose_actions(args()).unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
        let events = std::iter::from_fn(|| a.worker.try_event()).collect::<Vec<_>>();
        assert!(events.iter().any(|(id, e)| *id == status
            && matches!(e,AppEvent::Failed(error)if error.kind==ErrorKind::Cancelled)));
        assert_eq!(
            events
                .iter()
                .filter(|(_, e)| matches!(e, AppEvent::Chat(ChatEvent::Finished { .. })))
                .count(),
            1
        );
        persisted(a.base.path(), first_id, true);
        persisted(a.base.path(), failed_id, false);
    }
    #[test]
    fn mismatched_captured_turn_identity_selection_effort_and_terminal_status_cannot_create_reviews()
     {
        let mut a = active();
        let input = args();
        let id = candidate_id(b"brn-action-proposal-v1", a.bound.turn.id, &input).unwrap();
        let mut variants = vec![];
        let mut turn = a.bound.turn.clone();
        turn.id = Uuid::new_v4();
        variants.push(turn);
        let mut turn = a.bound.turn.clone();
        turn.conversation_id = Uuid::new_v4();
        variants.push(turn);
        let mut turn = a.bound.turn.clone();
        turn.provider = "copilot".into();
        variants.push(turn);
        let mut turn = a.bound.turn.clone();
        turn.model = "different".into();
        variants.push(turn);
        let mut turn = a.bound.turn.clone();
        turn.effort = Some("high".into());
        variants.push(turn);
        let mut turn = a.bound.turn.clone();
        turn.started_at_ms = Some(0);
        variants.push(turn);
        let mut turn = a.bound.turn.clone();
        turn.status = WorkTurnStatus::Completed;
        variants.push(turn);
        for turn in variants {
            let bad = BoundProposal {
                owner: a.bound.owner.clone(),
                request: a.bound.request.clone(),
                turn,
                cancel: a.bound.cancel.clone(),
                inbox: None,
            };
            assert!(bad.propose_actions(input.clone()).is_err());
        }
        a.worker
            .submit(Uuid::new_v4(), AppCommand::CancelTurn(a.bound.request.id))
            .unwrap();
        drop(a.lease);
        terminal(&a.worker, a.bound.request.id);
        a.worker.shutdown().unwrap();
        persisted(a.base.path(), id, false);
    }
    #[test]
    fn maximal_fixed_receipt_is_small_and_never_contains_candidate_or_comment_bodies() {
        // A conservative superset even permits maximally escaped path bytes and
        // u64 values that the evidence adapter would not produce.
        let mut receipt = Receipt {
            stamp: ProposalStamp {
                id: Uuid::new_v4(),
                version: u64::MAX,
            },
            state: ProposalState::Uncertain,
            session_id: Some(Uuid::new_v4()),
            group_id: Some(Uuid::new_v4()),
            action_ids: vec![Uuid::new_v4(); 20],
            sources: vec![
                SourceVersion {
                    path: "\u{1}".repeat(512),
                    fingerprint: brn_store::files::FileFingerprint {
                        device: u64::MAX,
                        inode: u64::MAX,
                        len: u64::MAX,
                        sha256: [255; 32]
                    },
                };
                64
            ],
        };
        let bytes = serde_json::to_vec(&receipt).unwrap();
        assert!(bytes.len() < brn_ai::READ_ACTION_BYTES);
        let json = serde_json::from_slice::<Value>(&bytes).unwrap();
        assert_eq!(json.as_object().unwrap().len(), 6);
        assert!(json.get("action_changes").is_none());
        assert!(json.get("comments").is_none());
        receipt.group_id = None;
        let ordinary = serde_json::to_value(&receipt).unwrap();
        assert_eq!(ordinary.as_object().unwrap().len(), 5);
        assert!(ordinary.get("group_id").is_none());
    }
}

#[cfg(test)]
mod quote_refusal_tests {
    use super::*;
    #[test]
    fn quote_refusals_keep_only_allowlisted_categories_and_fixed_messages() {
        for (workflow, kind) in [
            (ErrorKind::QuoteNotFound, AiErrorKind::QuoteNotFound),
            (ErrorKind::QuoteAmbiguous, AiErrorKind::QuoteAmbiguous),
            (
                ErrorKind::QuoteOccurrenceInvalid,
                AiErrorKind::QuoteOccurrenceInvalid,
            ),
        ] {
            let error = safe(WorkflowError::typed(
                workflow,
                "SYNTHETIC_PRIVATE_DIAGNOSTIC",
            ));
            assert_eq!(error.kind, kind);
            assert!(!format!("{error:?} {error}").contains("SYNTHETIC_PRIVATE_DIAGNOSTIC"));
            assert!(
                !serde_json::to_string(&error)
                    .unwrap()
                    .contains("SYNTHETIC_PRIVATE_DIAGNOSTIC")
            );
        }
    }
}
