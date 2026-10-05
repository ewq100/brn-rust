//! Owned chat/account lane. New turns are admitted only by AppWorker after local preflight.
use crate::proposal_rewrite::{self, RewriteEvent, RewriteRequest};
use crate::{ErrorKind, Result, WorkflowError, app::App};
use brn_ai::{
    AccountStatus, AiAnswer, AiError, AiErrorKind, AiEvent, AiTerminal, Auth, HistoryPair,
    LoginPrompt, ModelOption, Provider, ReadTools, ReasoningEffort, Selection,
};
use brn_store::work::{WorkTurn, WorkTurnStatus, chat::ChatStore};
use brn_store::work::{proposal_rewrite::RewriteOutcome, proposals::ProposalRecord};
use futures::FutureExt;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};
use tokio::{
    sync::{mpsc as async_mpsc, oneshot},
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskRequest {
    pub id: Uuid,
    pub conversation: Option<Uuid>,
    pub question: String,
    pub selection: Selection,
    pub effort: Option<ReasoningEffort>,
    pub generation: u64,
}

#[derive(Clone, Debug)]
pub enum ChatEvent {
    Text {
        id: Uuid,
        generation: u64,
        text: String,
    },
    ToolStarted {
        id: Uuid,
        generation: u64,
        name: String,
    },
    Finished {
        id: Uuid,
        generation: u64,
        turn: WorkTurn,
    },
    AlreadyRunning {
        id: Uuid,
        generation: u64,
        turn: WorkTurn,
    },
    Rejected {
        id: Uuid,
        generation: u64,
        error: WorkflowError,
    },
    PersistenceFailed {
        id: Uuid,
        generation: u64,
        partial: String,
        error: WorkflowError,
    },
}

impl ChatEvent {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Text { id, .. }
            | Self::ToolStarted { id, .. }
            | Self::Finished { id, .. }
            | Self::AlreadyRunning { id, .. }
            | Self::Rejected { id, .. }
            | Self::PersistenceFailed { id, .. } => *id,
        }
    }
    pub fn generation(&self) -> u64 {
        match self {
            Self::Text { generation, .. }
            | Self::ToolStarted { generation, .. }
            | Self::Finished { generation, .. }
            | Self::AlreadyRunning { generation, .. }
            | Self::Rejected { generation, .. }
            | Self::PersistenceFailed { generation, .. } => *generation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountCommand {
    Connect(Provider),
    Disconnect(Provider),
    Status(Provider),
    Models(Provider),
}
impl AccountCommand {
    fn provider(&self) -> Provider {
        match self {
            Self::Connect(p) | Self::Disconnect(p) | Self::Status(p) | Self::Models(p) => *p,
        }
    }
}

#[derive(Clone, Debug)]
pub enum AccountReply {
    Status(AccountStatus),
    Disconnected,
    Models(Vec<ModelOption>),
    Cancelled,
    Failed(AiError),
    Rejected(WorkflowError),
}

// Login codes are transient: deliberately neither Debug nor serializable.
pub enum AccountEvent {
    Login {
        id: Uuid,
        prompt: LoginPrompt,
    },
    Finished {
        id: Uuid,
        provider: Provider,
        reply: AccountReply,
    },
}

pub(crate) enum Output {
    Chat(ChatEvent),
    Rewrite(RewriteEvent),
    Account(AccountEvent),
    Idle,
}
pub(crate) type Emit = Arc<dyn Fn(Output) + Send + Sync>;

enum Command {
    Tools(Option<Arc<dyn ReadTools>>, mpsc::Sender<Result<()>>),
    Ask(
        AskRequest,
        Option<Box<crate::inbox_actions::InboxActionJob>>,
    ),
    Rewrite(RewriteRequest, CancellationToken),
    Account(Uuid, AccountCommand),
    Prompt(Uuid, LoginPrompt),
    Shutdown,
}

#[derive(Clone)]
pub(crate) struct ChatHandle {
    tx: async_mpsc::UnboundedSender<Command>,
    active: Arc<Mutex<Option<(Uuid, CancellationToken)>>>,
    accounts: Arc<Mutex<HashMap<Uuid, CancellationToken>>>,
    stopping: Arc<AtomicBool>,
    admission: Arc<Mutex<()>>,
}
impl ChatHandle {
    pub(crate) fn ask(&self, request: AskRequest) -> Result<()> {
        self.send(Command::Ask(request, None))
    }
    pub(crate) fn ask_inbox(
        &self,
        request: AskRequest,
        job: crate::inbox_actions::InboxActionJob,
    ) -> Result<()> {
        if request.id != job.capture.id
            || request.conversation != job.capture.conversation
            || request.question != job.question
            || provider_name(request.selection.provider) != job.capture.provider
            || request.selection.model != job.capture.model
            || request.effort.map(ReasoningEffort::as_str) != Some(job.capture.effort.as_str())
        {
            return Err(conflict());
        }
        self.send(Command::Ask(request, Some(Box::new(job))))
    }
    pub(crate) fn rewrite(&self, request: RewriteRequest, cancel: CancellationToken) -> Result<()> {
        self.send(Command::Rewrite(request, cancel))
    }
    pub(crate) fn account(&self, id: Uuid, command: AccountCommand) -> Result<()> {
        self.send(Command::Account(id, command))
    }
    fn send(&self, command: Command) -> Result<()> {
        let _admission = self.admission.lock().expect("owned admission fence");
        if self.stopping.load(Ordering::Acquire) {
            return Err(WorkflowError::cancelled());
        }
        self.tx.send(command).map_err(|_| closed())
    }
    pub(crate) fn cancel(&self, id: Uuid) -> bool {
        let active = self.active.lock().expect("owned cancellation registry");
        if let Some((active_id, cancel)) = &*active
            && *active_id == id
        {
            cancel.cancel();
            return true;
        }
        false
    }
    pub(crate) fn cancel_account(&self, id: Uuid) -> bool {
        let accounts = self.accounts.lock().expect("owned cancellation registry");
        if let Some(cancel) = accounts.get(&id) {
            cancel.cancel();
            true
        } else {
            false
        }
    }
    pub(crate) fn cancel_all(&self) {
        if let Some((_, cancel)) = &*self.active.lock().expect("owned cancellation registry") {
            cancel.cancel();
        }
        for cancel in self
            .accounts
            .lock()
            .expect("owned cancellation registry")
            .values()
        {
            cancel.cancel();
        }
    }
    pub(crate) fn stop_admission(&self) {
        let _admission = self.admission.lock().expect("owned admission fence");
        self.stopping.store(true, Ordering::Release);
        self.cancel_all();
    }
    pub(crate) fn set_tools(&self, tools: Option<Arc<dyn ReadTools>>) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.send(Command::Tools(tools, tx))?;
        rx.recv().map_err(|_| closed())?
    }
}

pub(crate) struct ChatWorker {
    pub(crate) handle: ChatHandle,
    join: Option<JoinHandle<Result<()>>>,
}
impl ChatWorker {
    pub(crate) fn start(
        app: &App,
        emit: Emit,
        hooks: Hooks,
        proposals: crate::app_worker::ActionProposals,
    ) -> Result<Self> {
        let store = app.work_store().chat_connection()?;
        let auth = app.auth();
        let (tx, rx) = async_mpsc::unbounded_channel();
        let handle = ChatHandle {
            tx,
            active: Arc::new(Mutex::new(None)),
            accounts: Arc::new(Mutex::new(HashMap::new())),
            stopping: Arc::new(AtomicBool::new(false)),
            admission: Arc::new(Mutex::new(())),
        };
        let control = handle.clone();
        let join = thread::Builder::new()
            .name("brn-chat".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| WorkflowError::msg("could not start chat runtime"))?;
                let (result, store) =
                    runtime.block_on(run(store, auth, rx, control, emit, hooks, proposals));
                // Runtime shutdown also joins blocking reads; the attachment outlives it.
                drop(runtime);
                drop(store);
                result
            })
            .map_err(|_| WorkflowError::msg("could not start chat lane"))?;
        Ok(Self {
            handle,
            join: Some(join),
        })
    }
    pub(crate) fn shutdown(&mut self) -> Result<()> {
        {
            let _admission = self.handle.admission.lock().expect("owned admission fence");
            self.handle.stopping.store(true, Ordering::Release);
            self.handle.cancel_all();
            let _ = self.handle.tx.send(Command::Shutdown);
        }
        if let Some(join) = self.join.take() {
            join.join()
                .map_err(|_| WorkflowError::msg("chat lane failed while draining"))?
        } else {
            Ok(())
        }
    }
}
impl Drop for ChatWorker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn closed() -> WorkflowError {
    WorkflowError::msg("chat lane is closed")
}
pub(crate) fn conflict() -> WorkflowError {
    WorkflowError::typed(
        ErrorKind::OperationConflict,
        "request UUID conflicts with an earlier request",
    )
}
pub(crate) fn provider_name(provider: Provider) -> &'static str {
    match provider {
        Provider::Chatgpt => "chatgpt",
        Provider::Copilot => "copilot",
    }
}
pub(crate) fn check_replay(request: &AskRequest, turn: &WorkTurn) -> Result<()> {
    if turn.question != request.question
        || turn.provider != provider_name(request.selection.provider)
        || turn.model != request.selection.model
        || turn.effort.as_deref() != request.effort.map(ReasoningEffort::as_str)
        || request
            .conversation
            .is_some_and(|id| id != turn.conversation_id)
    {
        Err(conflict())
    } else {
        Ok(())
    }
}
pub(crate) fn replay(request: &AskRequest, turn: WorkTurn) -> ChatEvent {
    if turn.status == WorkTurnStatus::Running {
        ChatEvent::AlreadyRunning {
            id: request.id,
            generation: request.generation,
            turn,
        }
    } else {
        ChatEvent::Finished {
            id: request.id,
            generation: request.generation,
            turn,
        }
    }
}
pub(crate) fn rejected(request: &AskRequest, error: WorkflowError) -> ChatEvent {
    ChatEvent::Rejected {
        id: request.id,
        generation: request.generation,
        error,
    }
}

#[derive(Clone, Default)]
pub(crate) struct Hooks {
    #[cfg(test)]
    pub(crate) answer: Option<super::simple_worker_tests::AnswerHook>,
    #[cfg(test)]
    pub(crate) proposal_answer: Option<super::simple_worker_tests::ProposalAnswerHook>,
    #[cfg(test)]
    pub(crate) account: Option<super::simple_worker_tests::AccountHook>,
    #[cfg(test)]
    pub(crate) rewrite: Option<proposal_rewrite::RewriteHook>,
}

struct Active {
    provider: Provider,
    cancel: CancellationToken,
}
enum RequestJob {
    Ask(AskRequest),
    Rewrite(RewriteRequest),
}
struct AccountJob {
    command: AccountCommand,
    cancel: CancellationToken,
}
enum JobResult {
    Turn(AskRequest, AiAnswer),
    Rewrite(RewriteRequest, RewriteOutcome),
    Account(Uuid, Provider, AccountReply),
    Disconnect(Uuid, Provider, brn_ai::AiResult<()>),
}

async fn run(
    mut store: ChatStore,
    auth: Arc<Auth>,
    mut rx: async_mpsc::UnboundedReceiver<Command>,
    control: ChatHandle,
    emit: Emit,
    hooks: Hooks,
    proposals: crate::app_worker::ActionProposals,
) -> (Result<()>, ChatStore) {
    let mut tools: Option<Arc<dyn ReadTools>> = None;
    let mut active: Option<Active> = None;
    let mut accounts = HashMap::<Uuid, AccountJob>::new();
    let mut account_ledger = HashMap::<Uuid, AccountCommand>::new();
    let mut turn_ledger = HashMap::<Uuid, AskRequest>::new();
    let mut disconnects = HashMap::<&'static str, (Uuid, Provider, bool)>::new();
    let mut jobs = JoinSet::new();
    let mut task_ids = HashMap::<tokio::task::Id, (Uuid, Option<RequestJob>, Provider)>::new();
    let mut closing = false;
    let mut final_error = None;
    loop {
        // Fenced deletion starts only after the target's owned clients/jobs have drained.
        for (id, provider, started) in disconnects.values_mut() {
            if !*started
                && active.as_ref().is_none_or(|a| a.provider != *provider)
                && accounts
                    .values()
                    .all(|job| job.command.provider() != *provider)
            {
                *started = true;
                let (id, provider, auth) = (*id, *provider, auth.clone());
                let task = jobs.spawn(async move {
                    JobResult::Disconnect(id, provider, auth.disconnect(provider).await)
                });
                task_ids.insert(task.id(), (id, None, provider));
            }
        }
        if closing && jobs.is_empty() && disconnects.is_empty() {
            break;
        }
        tokio::select! {
            command = rx.recv(), if !closing => {
                match command {
                    Some(Command::Shutdown) | None => {
                        closing = true;
                        control.cancel_all();
                    }
                    Some(Command::Tools(replacement, reply)) => {
                        let result = if active.is_some() {
                            Err(WorkflowError::typed(ErrorKind::ToolsBusy, "an AI turn still owns read tools"))
                        } else {
                            tools = replacement;
                            Ok(())
                        };
                        let _ = reply.send(result);
                    }
                    Some(Command::Prompt(id, prompt)) => {
                        if accounts.get(&id).is_some_and(|job| !job.cancel.is_cancelled()) {
                            emit(Output::Account(AccountEvent::Login { id, prompt }));
                        }
                    }
                    Some(Command::Ask(request, inbox)) => {
                        let validation = (|| -> Result<Option<WorkTurn>> {
                            if let Some(previous) = turn_ledger.get(&request.id) {
                                let mut previous = previous.clone();
                                if inbox.is_some() { previous.generation = request.generation; }
                                if previous != request { return Err(conflict()); }
                            }
                            if let Some(turn) = store.turn(request.id)? {
                                check_replay(&request, &turn)?;
                                return Ok(Some(turn));
                            }
                            if request.effort.is_none() {
                                return Err(WorkflowError::typed(ErrorKind::SelectionRequired,
                                    "choose an explicit reasoning effort before asking AI"));
                            }
                            if control.stopping.load(Ordering::Acquire) { return Err(WorkflowError::cancelled()); }
                            if disconnects.contains_key(provider_name(request.selection.provider)) {
                                return Err(WorkflowError::cancelled());
                            }
                            if active.is_some() {
                                return Err(WorkflowError::typed(ErrorKind::ToolsBusy, "another AI turn is active"));
                            }
                            if tools.is_none() {
                                return Err(WorkflowError::typed(ErrorKind::VaultNotBound, "choose a vault before asking AI"));
                            }
                            Ok(None)
                        })();
                        match validation {
                            Err(error) => emit(Output::Chat(rejected(&request, error))),
                            Ok(Some(turn)) => emit(Output::Chat(replay(&request, turn))),
                            Ok(None) => {
                                // Snapshot on this lane after the previous turn's commit, not during
                                // application preflight while that turn could still be finishing.
                                let history = match request.conversation {
                                    Some(conversation) => match store.turns(conversation) {
                                        Ok(turns) => crate::app::model_history(&turns),
                                        Err(error) => {
                                            emit(Output::Chat(rejected(&request, error.into())));
                                            continue;
                                        }
                                    },
                                    None => Vec::new(),
                                };
                                let admission = match inbox.as_deref() {
                                    Some(job) => store.begin_inbox_action_turn(job),
                                    None => store.begin_turn_with_effort(
                                        request.id, request.conversation, &request.question,
                                        provider_name(request.selection.provider), &request.selection.model,
                                        request.effort.map(ReasoningEffort::as_str),
                                    ),
                                };
                                match admission {
                                    Err(error) => emit(Output::Chat(rejected(&request, error.into()))),
                                    Ok(turn) => {
                                        let cancel = CancellationToken::new();
                                        *control.active.lock().expect("owned cancellation registry") = Some((request.id, cancel.clone()));
                                        active = Some(Active { provider: request.selection.provider, cancel: cancel.clone() });
                                        turn_ledger.insert(request.id, request.clone());
                                        let task = jobs.spawn(run_turn(
                                            auth.clone(), request.clone(), history,
                                            tools.as_ref().expect("preflight tools").clone(),
                                            match inbox {
                                                Some(job) => proposals.bind_inbox(&request, &turn, cancel.clone(), Some(job)),
                                                None => proposals.bind(&request, &turn, cancel.clone()),
                                            },
                                            cancel, emit.clone(), hooks.clone(),
                                        ));
                                        task_ids.insert(task.id(), (request.id, Some(RequestJob::Ask(request.clone())), request.selection.provider));
                                    }
                                }
                            }
                        }
                    }
                    Some(Command::Rewrite(request, cancel)) => {
                        let validation = (|| -> Result<Option<brn_store::work::proposal_rewrite::RewriteJob>> {
                            if let Some(job) = store.proposal_rewrite(request.id)? {
                                request.check_replay(&job)?;
                                return Ok(Some(job));
                            }
                            if cancel.is_cancelled() || control.stopping.load(Ordering::Acquire)
                                || disconnects.contains_key(provider_name(request.selection.provider))
                            { return Err(WorkflowError::cancelled()); }
                            if active.is_some() {
                                return Err(WorkflowError::typed(ErrorKind::ToolsBusy, "another AI request is active"));
                            }
                            if tools.is_none() {
                                return Err(WorkflowError::typed(ErrorKind::VaultNotBound, "choose a vault before Rewrite"));
                            }
                            Ok(None)
                        })();
                        match validation {
                            Err(error) => emit(Output::Rewrite(RewriteEvent::rejected(&request, error))),
                            Ok(Some(job)) => emit(Output::Rewrite(RewriteEvent::replay(&request, job))),
                            Ok(None) => match store.begin_proposal_rewrite(&request.spec()) {
                                Err(error) => emit(Output::Rewrite(RewriteEvent::rejected(&request, error.into()))),
                                Ok((job, capture)) => {
                                    let Some(capture) = capture else {
                                        emit(Output::Rewrite(RewriteEvent::replay(&request, job)));
                                        continue;
                                    };
                                    *control.active.lock().expect("owned cancellation registry") = Some((request.id, cancel.clone()));
                                    active = Some(Active { provider: request.selection.provider, cancel: cancel.clone() });
                                    emit(Output::Rewrite(RewriteEvent::started(&request, job)));
                                    let task = jobs.spawn(run_rewrite(
                                        auth.clone(), request.clone(), capture,
                                        tools.as_ref().expect("preflight tools").clone(),
                                        cancel, emit.clone(), hooks.clone(),
                                    ));
                                    task_ids.insert(task.id(), (request.id, Some(RequestJob::Rewrite(request.clone())), request.selection.provider));
                                }
                            }
                        }
                    }
                    Some(Command::Account(id, command)) => {
                        let provider = command.provider();
                        if account_ledger.contains_key(&id) {
                            emit(Output::Account(AccountEvent::Finished { id, provider, reply: AccountReply::Rejected(conflict()) }));
                            continue;
                        }
                        account_ledger.insert(id, command.clone());
                        if control.stopping.load(Ordering::Acquire) || disconnects.contains_key(provider_name(provider)) {
                            emit(Output::Account(AccountEvent::Finished { id, provider, reply: AccountReply::Cancelled }));
                            continue;
                        }
                        if matches!(command, AccountCommand::Disconnect(_)) {
                            if let Some(turn) = &active
                                && turn.provider == provider
                            { turn.cancel.cancel(); }
                            for job in accounts.values().filter(|j| j.command.provider() == provider) {
                                job.cancel.cancel();
                            }
                            disconnects.insert(provider_name(provider), (id, provider, false));
                        } else {
                            let cancel = CancellationToken::new();
                            control.accounts.lock().expect("owned cancellation registry").insert(id, cancel.clone());
                            accounts.insert(id, AccountJob { command: command.clone(), cancel: cancel.clone() });
                            let task = jobs.spawn(run_account(
                                auth.clone(), id, command, cancel, control.tx.clone(), hooks.clone(),
                            ));
                            task_ids.insert(task.id(), (id, None, provider));
                        }
                    }
                }
            }
            result = jobs.join_next_with_id(), if !jobs.is_empty() => {
                let Some(result) = result else { continue };
                let job = match result {
                    Ok((task_id, job)) => { task_ids.remove(&task_id); job }
                    Err(error) => {
                        let Some((id, request, provider)) = task_ids.remove(&error.id()) else {
                            final_error.get_or_insert_with(|| WorkflowError::msg("owned chat job failed"));
                            continue;
                        };
                        if let Some(request) = request {
                            match request {
                                RequestJob::Ask(request) => JobResult::Turn(request, AiAnswer { text: String::new(), terminal: AiTerminal::Failed(AiError::new(AiErrorKind::Other)) }),
                                RequestJob::Rewrite(request) => JobResult::Rewrite(request, RewriteOutcome::Failed("other".into())),
                            }
                        } else if disconnects.get(provider_name(provider)).is_some_and(|(job, _, _)| *job == id) {
                            JobResult::Disconnect(id, provider, Err(AiError::new(AiErrorKind::Other)))
                        } else {
                            JobResult::Account(id, provider, AccountReply::Failed(AiError::new(AiErrorKind::Other)))
                        }
                    }
                };
                match job {
                    JobResult::Rewrite(request, outcome) => {
                        match store.finish_proposal_rewrite(request.id, &outcome) {
                            Ok(job) => emit(Output::Rewrite(RewriteEvent::Finished { id: request.id, generation: request.generation, job })),
                            Err(_) => {
                                let error = WorkflowError::typed(ErrorKind::AiStorage, "could not confirm durable Rewrite settlement");
                                final_error.get_or_insert(error.clone());
                                emit(Output::Rewrite(RewriteEvent::PersistenceFailed { id: request.id, generation: request.generation, error }));
                            }
                        }
                        active = None;
                        *control.active.lock().expect("owned cancellation registry") = None;
                        emit(Output::Idle);
                    }
                    JobResult::Turn(request, answer) => {
                        let (status, code) = terminal(&answer.terminal);
                        match store.finish_turn(request.id, status, &answer.text, code) {
                            Ok(turn) => emit(Output::Chat(ChatEvent::Finished { id: request.id, generation: request.generation, turn })),
                            Err(_) => {
                                let error = WorkflowError::typed(ErrorKind::AiStorage, "could not save AI turn; partial text is only in memory");
                                final_error.get_or_insert(error.clone());
                                emit(Output::Chat(ChatEvent::PersistenceFailed {
                                    id: request.id, generation: request.generation, partial: answer.text, error,
                                }));
                            }
                        }
                        active = None;
                        *control.active.lock().expect("owned cancellation registry") = None;
                        emit(Output::Idle);
                    }
                    JobResult::Account(id, provider, reply) => {
                        accounts.remove(&id);
                        control.accounts.lock().expect("owned cancellation registry").remove(&id);
                        if let AccountReply::Failed(error) = &reply
                            && matches!(error.kind, AiErrorKind::Storage | AiErrorKind::UnsafeCredentials)
                        { final_error.get_or_insert_with(|| error.clone().into()); }
                        emit(Output::Account(AccountEvent::Finished { id, provider, reply }));
                    }
                    JobResult::Disconnect(id, provider, result) => {
                        disconnects.remove(provider_name(provider));
                        if let Err(error) = &result
                            && matches!(error.kind, AiErrorKind::Storage | AiErrorKind::UnsafeCredentials)
                        { final_error.get_or_insert_with(|| error.clone().into()); }
                        emit(Output::Account(AccountEvent::Finished { id, provider, reply: match result {
                            Ok(()) => AccountReply::Disconnected,
                            Err(error) => AccountReply::Failed(error),
                        }}));
                    }
                }
            }
        }
    }
    drop(tools);
    (final_error.map_or(Ok(()), Err), store)
}

fn terminal(terminal: &AiTerminal) -> (WorkTurnStatus, Option<&'static str>) {
    match terminal {
        AiTerminal::Completed => (WorkTurnStatus::Completed, None),
        AiTerminal::Interrupted => (WorkTurnStatus::Interrupted, None),
        AiTerminal::Failed(error) => (
            WorkTurnStatus::Failed,
            Some(match error.kind {
                AiErrorKind::ReconnectNeeded => "reconnect_needed",
                AiErrorKind::CodeExpired => "code_expired",
                AiErrorKind::RateLimited => "rate_limited",
                AiErrorKind::Network => "network",
                AiErrorKind::ModelRefused => "model_refused",
                AiErrorKind::InvalidToolUse => "invalid_tool_use",
                AiErrorKind::ToolLimitReached => "tool_limit_reached",
                AiErrorKind::UnsafeCredentials => "unsafe_credentials",
                AiErrorKind::ToolRejected => "tool_rejected",
                AiErrorKind::IndexStale => "index_stale",
                AiErrorKind::Storage => "storage",
                AiErrorKind::Other => "other",
            }),
        ),
    }
}

// This lease ends only after the agent AND every queued/running blocking read release it.
struct DrainedTools {
    tools: Arc<dyn ReadTools>,
    _drained: Arc<DrainSignal>,
}
struct DrainSignal(Option<oneshot::Sender<()>>);
impl Drop for DrainSignal {
    fn drop(&mut self) {
        if let Some(drained) = self.0.take() {
            let _ = drained.send(());
        }
    }
}
struct DrainedProposals {
    proposals: Arc<dyn brn_ai::ProposalTools>,
    _drained: Arc<DrainSignal>,
}
impl brn_ai::ProposalTools for DrainedProposals {
    fn knowledge_enabled(&self) -> bool {
        self.proposals.knowledge_enabled()
    }
    fn propose_knowledge(
        &self,
        args: brn_ai::KnowledgeProposalArgs,
    ) -> brn_ai::AiResult<serde_json::Value> {
        self.proposals.propose_knowledge(args)
    }
    fn propose_actions(
        &self,
        args: brn_ai::ActionProposalArgs,
    ) -> brn_ai::AiResult<serde_json::Value> {
        self.proposals.propose_actions(args)
    }
}
impl ReadTools for DrainedTools {
    fn read_action(&self, id: &str) -> brn_ai::AiResult<serde_json::Value> {
        self.tools.read_action(id)
    }
    fn list_actions(
        &self,
        state: Option<&str>,
        limit: usize,
        cursor: Option<&str>,
    ) -> brn_ai::AiResult<serde_json::Value> {
        self.tools.list_actions(state, limit, cursor)
    }
    fn search_notes(&self, query: &str, limit: usize) -> brn_ai::AiResult<brn_ai::ToolSearch> {
        self.tools.search_notes(query, limit)
    }
    fn read_note(&self, path: &str) -> brn_ai::AiResult<brn_ai::ToolNote> {
        self.tools.read_note(path)
    }
    fn list_notes(
        &self,
        folder: Option<&str>,
        cursor: Option<&str>,
    ) -> brn_ai::AiResult<brn_ai::NotePage> {
        self.tools.list_notes(folder, cursor)
    }

    fn search_notes_scoped(
        &self,
        query: &str,
        limit: usize,
        scope: brn_ai::ReadScope,
    ) -> brn_ai::AiResult<brn_ai::ToolSearch> {
        self.tools.search_notes_scoped(query, limit, scope)
    }
    fn read_note_scoped(
        &self,
        path: &str,
        scope: brn_ai::ReadScope,
    ) -> brn_ai::AiResult<brn_ai::ToolNote> {
        self.tools.read_note_scoped(path, scope)
    }
    fn list_notes_scoped(
        &self,
        folder: Option<&str>,
        cursor: Option<&str>,
        scope: brn_ai::ReadScope,
    ) -> brn_ai::AiResult<brn_ai::NotePage> {
        self.tools.list_notes_scoped(folder, cursor, scope)
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_turn(
    auth: Arc<Auth>,
    request: AskRequest,
    history: Vec<HistoryPair>,
    tools: Arc<dyn ReadTools>,
    proposals: Arc<dyn brn_ai::ProposalTools>,
    cancel: CancellationToken,
    emit: Emit,
    hooks: Hooks,
) -> JobResult {
    let (drained, wait) = oneshot::channel();
    let lease = Arc::new(DrainSignal(Some(drained)));
    let tools: Arc<dyn ReadTools> = Arc::new(DrainedTools {
        tools,
        _drained: lease.clone(),
    });
    let proposals: Arc<dyn brn_ai::ProposalTools> = Arc::new(DrainedProposals {
        proposals,
        _drained: lease,
    });
    let request_events = request.clone();
    let partial = Arc::new(Mutex::new(String::new()));
    let partial_events = partial.clone();
    let events: Arc<dyn Fn(AiEvent) + Send + Sync> = Arc::new(move |event| {
        let (id, generation) = (request_events.id, request_events.generation);
        emit(Output::Chat(match event {
            AiEvent::Text(text) => {
                partial_events
                    .lock()
                    .expect("turn-owned partial text")
                    .push_str(&text);
                ChatEvent::Text {
                    id,
                    generation,
                    text,
                }
            }
            AiEvent::ToolStarted { name } => ChatEvent::ToolStarted {
                id,
                generation,
                name,
            },
        }));
    });
    #[cfg(test)]
    let fake = hooks.answer;
    #[cfg(test)]
    let proposal_fake = hooks.proposal_answer;
    #[cfg(not(test))]
    let _ = hooks;
    let operation = async {
        #[cfg(test)]
        {
            if let Some(fake) = proposal_fake {
                fake(
                    request.clone(),
                    history,
                    tools,
                    proposals,
                    cancel.clone(),
                    events,
                )
                .await
            } else if let Some(fake) = fake {
                drop(proposals);
                fake(request.clone(), history, tools, cancel.clone(), events).await
            } else {
                real_answer(
                    auth,
                    &request,
                    history,
                    tools,
                    proposals,
                    cancel.clone(),
                    events,
                )
                .await
            }
        }
        #[cfg(not(test))]
        {
            real_answer(
                auth,
                &request,
                history,
                tools,
                proposals,
                cancel.clone(),
                events,
            )
            .await
        }
    };
    let mut answer = std::panic::AssertUnwindSafe(operation)
        .catch_unwind()
        .await
        .unwrap_or_else(|_| AiAnswer {
            text: partial.lock().expect("turn-owned partial text").clone(),
            terminal: AiTerminal::Failed(AiError::new(AiErrorKind::Other)),
        });
    let _ = wait.await;
    if cancel.is_cancelled()
        && matches!(
            answer.terminal,
            AiTerminal::Failed(AiError {
                kind: AiErrorKind::Other,
                ..
            })
        )
    {
        answer.terminal = AiTerminal::Interrupted;
    }
    JobResult::Turn(request, answer)
}
#[allow(clippy::too_many_arguments)]
async fn real_answer(
    auth: Arc<Auth>,
    request: &AskRequest,
    history: Vec<HistoryPair>,
    tools: Arc<dyn ReadTools>,
    proposals: Arc<dyn brn_ai::ProposalTools>,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer {
    let Some(effort) = request.effort else {
        return AiAnswer {
            text: String::new(),
            terminal: AiTerminal::Failed(AiError::new(AiErrorKind::ModelRefused)),
        };
    };
    match auth.client(&request.selection, cancel.clone()).await {
        Ok(client) => {
            brn_ai::answer_with_proposals(
                client,
                &request.question,
                &history,
                effort,
                tools,
                proposals,
                cancel,
                emit,
            )
            .await
        }
        Err(error) => AiAnswer {
            text: String::new(),
            terminal: AiTerminal::Failed(error),
        },
    }
}

async fn run_rewrite(
    auth: Arc<Auth>,
    request: RewriteRequest,
    capture: ProposalRecord,
    tools: Arc<dyn ReadTools>,
    cancel: CancellationToken,
    emit: Emit,
    hooks: Hooks,
) -> JobResult {
    let (drained, wait) = oneshot::channel();
    let tools: Arc<dyn ReadTools> = Arc::new(DrainedTools {
        tools,
        _drained: Arc::new(DrainSignal(Some(drained))),
    });
    let (id, generation) = (request.id, request.generation);
    let events: Arc<dyn Fn(AiEvent) + Send + Sync> = Arc::new(move |event| {
        if let AiEvent::ToolStarted { name } = event {
            emit(Output::Rewrite(RewriteEvent::ToolStarted {
                id,
                generation,
                name,
            }));
        }
    });
    #[cfg(test)]
    let fake = hooks.rewrite;
    #[cfg(not(test))]
    let _ = hooks;
    let operation = async {
        let prompt = match proposal_rewrite::prompt(&capture) {
            Ok(prompt) => prompt,
            Err(_) => return RewriteOutcome::Failed("tool_rejected".into()),
        };
        #[cfg(test)]
        let answer = if let Some(fake) = fake {
            fake(request.clone(), prompt, tools, cancel.clone(), events).await
        } else {
            real_rewrite(auth, &request, &prompt, tools, cancel.clone(), events).await
        };
        #[cfg(not(test))]
        let answer = real_rewrite(auth, &request, &prompt, tools, cancel.clone(), events).await;
        match answer.terminal {
            AiTerminal::Completed => match proposal_rewrite::decode(&request, &answer.text)
                .and_then(|edit| {
                    brn_store::work::proposal_rewrite::validate_result(&capture, &edit)?;
                    Ok(edit)
                }) {
                Ok(edit) => RewriteOutcome::Completed(edit),
                Err(_) => RewriteOutcome::Failed("tool_rejected".into()),
            },
            AiTerminal::Interrupted => RewriteOutcome::Interrupted,
            AiTerminal::Failed(error)
                if cancel.is_cancelled() && error.kind == AiErrorKind::Other =>
            {
                RewriteOutcome::Interrupted
            }
            AiTerminal::Failed(error) => {
                let (_, code) = terminal(&AiTerminal::Failed(error));
                RewriteOutcome::Failed(code.expect("failed terminal has safe code").into())
            }
        }
    };
    let mut outcome = std::panic::AssertUnwindSafe(operation)
        .catch_unwind()
        .await
        .unwrap_or_else(|_| RewriteOutcome::Failed("other".into()));
    let _ = wait.await;
    if cancel.is_cancelled() && matches!(&outcome, RewriteOutcome::Failed(code) if code == "other")
    {
        outcome = RewriteOutcome::Interrupted;
    }
    JobResult::Rewrite(request, outcome)
}

async fn real_rewrite(
    auth: Arc<Auth>,
    request: &RewriteRequest,
    prompt: &str,
    tools: Arc<dyn ReadTools>,
    cancel: CancellationToken,
    emit: Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer {
    match auth.client(&request.selection, cancel.clone()).await {
        Ok(client) => brn_ai::rewrite(client, prompt, request.effort, tools, cancel, emit).await,
        Err(error) => AiAnswer {
            text: String::new(),
            terminal: AiTerminal::Failed(error),
        },
    }
}
async fn run_account(
    auth: Arc<Auth>,
    id: Uuid,
    command: AccountCommand,
    cancel: CancellationToken,
    tx: async_mpsc::UnboundedSender<Command>,
    hooks: Hooks,
) -> JobResult {
    let provider = command.provider();
    let login: Arc<dyn Fn(LoginPrompt) + Send + Sync> = Arc::new(move |prompt| {
        let _ = tx.send(Command::Prompt(id, prompt));
    });
    #[cfg(test)]
    let fake = hooks.account;
    #[cfg(not(test))]
    let _ = hooks;
    #[cfg(test)]
    let reply = if let Some(fake) = fake {
        fake(command, cancel.clone(), login).await
    } else {
        real_account(auth, command, cancel.clone(), login).await
    };
    #[cfg(not(test))]
    let reply = real_account(auth, command, cancel.clone(), login).await;
    let reply = if cancel.is_cancelled()
        && matches!(
            reply,
            AccountReply::Failed(AiError {
                kind: AiErrorKind::Other,
                ..
            })
        ) {
        AccountReply::Cancelled
    } else {
        reply
    };
    JobResult::Account(id, provider, reply)
}
async fn real_account(
    auth: Arc<Auth>,
    command: AccountCommand,
    cancel: CancellationToken,
    login: Arc<dyn Fn(LoginPrompt) + Send + Sync>,
) -> AccountReply {
    match command {
        AccountCommand::Connect(provider) => auth
            .connect(provider, login, cancel)
            .await
            .map(AccountReply::Status),
        AccountCommand::Status(provider) => auth.status(provider).await.map(AccountReply::Status),
        AccountCommand::Models(provider) => auth
            .models(provider, cancel)
            .await
            .map(AccountReply::Models),
        AccountCommand::Disconnect(_) => unreachable!("disconnect is fenced in the dispatch loop"),
    }
    .unwrap_or_else(AccountReply::Failed)
}
