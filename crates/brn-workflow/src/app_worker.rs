//! The frontend owns this handle, never App, SQLite, a model or a runtime.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::{App, AppConfig},
    chat_worker::{
        self, AccountCommand, AccountEvent, AccountReply, AskRequest, ChatEvent, ChatHandle,
        ChatWorker,
    },
    library::{RefreshReport, SearchMode, SearchResults},
    models::{ModelDownloadPrompt, ModelInstallReport},
    vault::{NoteText, VaultPath},
};
use brn_ai::{ModelOption, NotePage, Provider, ReasoningEffort, Selection};
use brn_store::work::{WorkConversation, WorkTurn};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use uuid::Uuid;

pub enum AppCommand {
    #[cfg(test)]
    TestPause {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    },
    BindVault(PathBuf),
    Status,
    Refresh,
    Selection,
    Select(Selection),
    Effort,
    SelectEffort(ReasoningEffort),
    Notes {
        folder: Option<String>,
        cursor: Option<String>,
    },
    Note(String),
    OpenEditor(String),
    ReloadEditor(crate::editor::ReloadRequest),
    RecoverEditor(crate::editor::EditRequest),
    SaveEditor(crate::editor::SaveRequest),
    Editors,
    ReconcileEditor(Uuid),
    ProposalSource(String),
    NoteIdentity(String),
    IdentityInventory,
    ResolveNoteIdentity(Uuid),
    EvidenceNote(String),
    PrepareNoteIdentity(crate::knowledge::IdentityRequest),
    CreateProposal(crate::proposals::DraftRequest),
    Proposal(Uuid),
    Proposals(Option<Uuid>),
    EditProposal(crate::proposals::ProposalEdit),
    RewriteProposal(crate::proposals::ProposalEdit),
    StartProposalRewrite(crate::proposal_rewrite::RewriteRequest),
    ProposalRewrite(Uuid),
    AddProposalComment(crate::proposals::CommentRequest),
    UpdateProposalComment(crate::proposals::CommentRequest),
    RemoveProposalComment {
        expected: crate::proposals::ProposalStamp,
        comment: Uuid,
    },
    RejectProposal(crate::proposals::ProposalStamp),
    ApproveProposal(crate::proposal_apply::ApprovalRequest),
    ReconcileProposal(Uuid),
    PreviewProposalUndo(crate::proposal_apply::UndoRequest),
    UndoProposal(crate::proposal_apply::UndoRequest),
    PreviewProposalRepair(Uuid),
    RepairProposal(crate::proposal_apply::RepairRequest),
    ApproveProposalGroup(crate::proposal_apply::GroupApprovalRequest),
    ProposalApplies,
    ProposalRecovery,
    ProposalApply(Uuid),
    Activity(crate::activity::ActivityRequest),
    RecoverEdit {
        path: String,
        base_sha256: [u8; 32],
        text: String,
    },
    Search {
        query: String,
        mode: SearchMode,
        limit: usize,
    },
    Conversations,
    Turns(Uuid),
    Turn(Uuid),
    Ask(AskRequest),
    CancelTurn(Uuid),
    Account {
        id: Uuid,
        command: AccountCommand,
    },
    CancelAccount(Uuid),
    ModelPrompt,
    DownloadModel {
        consent: bool,
        target: PathBuf,
    },
    CancelModelDownload(Uuid),
}

// No Debug or serialization: Account can carry a transient device code.
pub enum AppEvent {
    Ready {
        vault_bound: bool,
        model_installed: bool,
    },
    Restored {
        backup: PathBuf,
    },
    VaultBound,
    Status(AppStatus),
    Selection(Option<Selection>),
    SelectionSaved,
    Effort(Option<ReasoningEffort>),
    EffortSaved,
    Refreshed(RefreshReport),
    Notes(NotePage),
    Note(NoteText),
    EditRecovered,
    Editor(crate::editor::EditorView),
    EditorRecovered(crate::editor::EditorRecord),
    EditorSaved(crate::editor::SaveReceipt),
    Editors(Vec<crate::editor::EditorRecord>),
    ProposalSource(Box<crate::proposals::ProposalSource>),
    NoteIdentity(crate::knowledge::NoteIdentityInfo),
    IdentityInventory(Box<crate::knowledge::IdentityInventory>),
    NoteIdentityResolved(Box<crate::knowledge::IdentityResolution>),
    EvidenceNote(NoteText),
    NoteIdentityDraft(Box<crate::proposals::DraftRequest>),
    Proposal(crate::proposals::ProposalRecord),
    ProposalRewrite(Option<crate::proposal_rewrite::RewriteJob>),
    Rewrite(crate::proposal_rewrite::RewriteEvent),
    Proposals(Vec<crate::proposals::ProposalRecord>),
    ProposalApplied(crate::proposal_apply::ApplyReceipt),
    ProposalUndoPreview(crate::proposal_apply::UndoPreview),
    ProposalRepairPreview(crate::proposal_apply::RepairPreview),
    ProposalRepaired(crate::proposal_apply::RepairReceipt),
    ProposalGroupApplied(crate::proposal_apply::GroupApprovalResult),
    ProposalApplies(Vec<crate::proposal_apply::ApplyJournal>),
    ProposalRecovery(Vec<crate::proposal_apply::ApplySummary>),
    ProposalApply(Option<Box<crate::proposal_apply::ApplyJournal>>),
    Activity(crate::activity::ActivityPage),
    Search(SearchResults),
    Conversations(Vec<WorkConversation>),
    Turns(Vec<WorkTurn>),
    Turn(Option<WorkTurn>),
    Indexing {
        embedded: usize,
        total: usize,
    },
    Chat(ChatEvent),
    Account(AccountEvent),
    TurnCancelRequested {
        turn: Uuid,
        accepted: bool,
    },
    AccountCancelRequested {
        operation: Uuid,
        accepted: bool,
    },
    ModelCancelRequested {
        operation: Uuid,
        accepted: bool,
    },
    ModelPrompt(Option<ModelDownloadPrompt>),
    ModelDownload {
        received: u64,
        total: u64,
    },
    ModelDownloaded(ModelInstallReport),
    ModelInstalled,
    ModelDownloadDeclined,
    Failed(WorkflowError),
}

#[derive(Clone, Debug)]
pub struct AppStatus {
    pub vault_root: Option<PathBuf>,
    pub model_installed: bool,
    pub model_download: Option<Uuid>,
}

enum Message {
    Command(Uuid, AppCommand),
    Models(Uuid, Provider, Vec<ModelOption>),
    ModelProgress(Uuid, u64, u64),
    ModelDone(Uuid),
    ChatIdle,
    #[cfg(test)]
    IdleBarrier(mpsc::Sender<()>),
    Shutdown,
}

#[derive(Default)]
struct Controls {
    chat: Option<ChatHandle>,
    queued_rewrites: HashMap<Uuid, tokio_util::sync::CancellationToken>,
    model: Option<(Uuid, Arc<AtomicBool>)>,
}

/// A Send handle with separate immediate controls and correlated owned-lane events.
pub struct AppWorker {
    tx: mpsc::Sender<Message>,
    events: mpsc::Receiver<(Uuid, AppEvent)>,
    emit: mpsc::Sender<(Uuid, AppEvent)>,
    controls: Arc<Mutex<Controls>>,
    stopping: Arc<AtomicBool>,
    admission: Mutex<()>,
    join: Option<JoinHandle<Result<()>>>,
    shutdown_result: Option<Result<()>>,
}

#[derive(Clone, Default)]
struct Hooks {
    chat: chat_worker::Hooks,
    #[cfg(test)]
    install: Option<super::simple_worker_tests::InstallHook>,
    #[cfg(test)]
    load: Option<super::simple_worker_tests::LoadHook>,
}

impl AppWorker {
    /// Only spawns an owned thread. Opening, scanning and model loading happen there.
    pub fn start(data_dir: PathBuf, config: AppConfig) -> Result<Self> {
        Self::start_owned(data_dir, config, Hooks::default())
    }

    fn start_owned(data_dir: PathBuf, config: AppConfig, hooks: Hooks) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let (emit, events) = mpsc::channel();
        let controls = Arc::new(Mutex::new(Controls::default()));
        let stopping = Arc::new(AtomicBool::new(false));
        let (messages, output, control, closing) =
            (tx.clone(), emit.clone(), controls.clone(), stopping.clone());
        let startup = Uuid::new_v4();
        let join = thread::Builder::new()
            .name("brn-app".into())
            .spawn(move || {
                let result = app_lane(
                    data_dir,
                    config,
                    rx,
                    messages,
                    output.clone(),
                    control,
                    closing,
                    startup,
                    hooks,
                );
                if let Err(error) = &result {
                    let _ = output.send((startup, AppEvent::Failed(error.clone())));
                }
                result
            })
            .map_err(|_| WorkflowError::msg("could not start application lane"))?;
        Ok(Self {
            tx,
            events,
            emit,
            controls,
            stopping,
            join: Some(join),
            shutdown_result: None,
            admission: Mutex::new(()),
        })
    }

    pub fn submit(&self, id: Uuid, command: AppCommand) -> Result<()> {
        let _admission = self.admission.lock().expect("owned admission fence");
        if self.stopping.load(Ordering::Acquire) {
            return Err(WorkflowError::cancelled());
        }
        if matches!(&command, AppCommand::Ask(request) if request.id != id)
            || matches!(&command, AppCommand::StartProposalRewrite(request) if request.id != id)
            || matches!(&command, AppCommand::Account { id: operation, .. } if *operation != id)
            || matches!(&command, AppCommand::SaveEditor(request) if request.operation_id != id)
        {
            return Err(chat_worker::conflict());
        }
        // These commands never wait behind a scan, model load, stream or device login.
        match command {
            AppCommand::CancelTurn(turn) => {
                let controls = self.controls.lock().expect("owned controls");
                let queued = controls.queued_rewrites.get(&turn).is_some_and(|cancel| {
                    cancel.cancel();
                    true
                });
                let accepted =
                    controls.chat.as_ref().is_some_and(|chat| chat.cancel(turn)) || queued;
                self.output(id, AppEvent::TurnCancelRequested { turn, accepted })
            }
            AppCommand::StartProposalRewrite(request) => {
                self.controls
                    .lock()
                    .expect("owned controls")
                    .queued_rewrites
                    .entry(id)
                    .or_default();
                self.tx
                    .send(Message::Command(
                        id,
                        AppCommand::StartProposalRewrite(request),
                    ))
                    .map_err(|_| closed())
            }
            AppCommand::CancelAccount(operation) => {
                let accepted = self
                    .controls
                    .lock()
                    .expect("owned controls")
                    .chat
                    .as_ref()
                    .is_some_and(|chat| chat.cancel_account(operation));
                self.output(
                    id,
                    AppEvent::AccountCancelRequested {
                        operation,
                        accepted,
                    },
                )
            }
            AppCommand::CancelModelDownload(operation) => {
                let accepted = self
                    .controls
                    .lock()
                    .expect("owned controls")
                    .model
                    .as_ref()
                    .is_some_and(|(active, cancel)| {
                        if *active == operation {
                            cancel.store(true, Ordering::Release);
                            true
                        } else {
                            false
                        }
                    });
                self.output(
                    id,
                    AppEvent::ModelCancelRequested {
                        operation,
                        accepted,
                    },
                )?;
                if accepted {
                    self.tx.send(Message::ChatIdle).map_err(|_| closed())?;
                }
                Ok(())
            }
            AppCommand::Account { id, command } => {
                let controls = self.controls.lock().expect("owned controls");
                if let Some(chat) = &controls.chat {
                    chat.account(id, command)
                } else {
                    self.tx
                        .send(Message::Command(id, AppCommand::Account { id, command }))
                        .map_err(|_| closed())
                }
            }
            command => self
                .tx
                .send(Message::Command(id, command))
                .map_err(|_| closed()),
        }
    }
    fn output(&self, id: Uuid, event: AppEvent) -> Result<()> {
        self.emit.send((id, event)).map_err(|_| closed())
    }
    pub fn try_event(&self) -> Option<(Uuid, AppEvent)> {
        self.events.try_recv().ok()
    }
    pub fn recv_event_timeout(
        &self,
        timeout: Duration,
    ) -> std::result::Result<(Uuid, AppEvent), mpsc::RecvTimeoutError> {
        self.events.recv_timeout(timeout)
    }
    pub fn shutdown(&mut self) -> Result<()> {
        {
            let _admission = self.admission.lock().expect("owned admission fence");
            self.stopping.store(true, Ordering::Release);
            let controls = self.controls.lock().expect("owned controls");
            if let Some(chat) = &controls.chat {
                chat.stop_admission();
            }
            if let Some((_, cancel)) = &controls.model {
                cancel.store(true, Ordering::Release);
            }
            let _ = self.tx.send(Message::Shutdown);
        }
        if let Some(join) = self.join.take() {
            let result = join
                .join()
                .map_err(|_| WorkflowError::msg("application lane failed while draining"))
                .and_then(|r| r);
            self.shutdown_result = Some(result.clone());
            result
        } else {
            self.shutdown_result.clone().unwrap_or(Ok(()))
        }
    }
}
impl Drop for AppWorker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
fn closed() -> WorkflowError {
    WorkflowError::msg("application lane is closed")
}

fn cancelled_command(command: AppCommand) -> AppEvent {
    match command {
        AppCommand::StartProposalRewrite(request) => AppEvent::Rewrite(
            crate::proposal_rewrite::RewriteEvent::rejected(&request, WorkflowError::cancelled()),
        ),
        AppCommand::Ask(request) => {
            AppEvent::Chat(chat_worker::rejected(&request, WorkflowError::cancelled()))
        }
        AppCommand::Account { id, command } => {
            let provider = match command {
                AccountCommand::Connect(provider)
                | AccountCommand::Disconnect(provider)
                | AccountCommand::Status(provider)
                | AccountCommand::Models(provider) => provider,
            };
            AppEvent::Account(AccountEvent::Finished {
                id,
                provider,
                reply: AccountReply::Cancelled,
            })
        }
        _ => AppEvent::Failed(WorkflowError::cancelled()),
    }
}

struct InstallJob {
    id: Uuid,
    cancel: Arc<AtomicBool>,
    join: Option<JoinHandle<Result<ModelInstallReport>>>,
    downloaded: Option<ModelInstallReport>,
}

fn indexing_job(app: &App, id: Uuid) -> Result<Option<Uuid>> {
    if !app.model_installed() {
        return Ok(None);
    }
    match app.tools() {
        Ok(_) => Ok(Some(id)),
        Err(error)
            if matches!(
                error.kind,
                ErrorKind::VaultNotBound | ErrorKind::VaultUnavailable | ErrorKind::SaveUncertain
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

impl Drop for InstallJob {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn app_lane(
    data: PathBuf,
    config: AppConfig,
    rx: mpsc::Receiver<Message>,
    tx: mpsc::Sender<Message>,
    emit: mpsc::Sender<(Uuid, AppEvent)>,
    controls: Arc<Mutex<Controls>>,
    stopping: Arc<AtomicBool>,
    startup: Uuid,
    hooks: Hooks,
) -> Result<()> {
    #[cfg(test)]
    let mut app = App::open_with_model_loader(&data, config, |data, selected| {
        if let (Some(load), Some(directory)) = (&hooks.load, selected) {
            load(directory, &AtomicBool::new(false)).map(Some)
        } else {
            crate::app::load_model(data, selected)
        }
    })?;
    #[cfg(not(test))]
    let mut app = App::open(&data, config)?;
    if let Some(backup) = &app.open_report().restored_from {
        let _ = emit.send((
            startup,
            AppEvent::Restored {
                backup: backup.clone(),
            },
        ));
    }
    let (output, messages, rewrite_controls) = (emit.clone(), tx.clone(), controls.clone());
    let chat_emit: chat_worker::Emit = Arc::new(move |event| match event {
        chat_worker::Output::Chat(event) => {
            let _ = output.send((event.id(), AppEvent::Chat(event)));
        }
        chat_worker::Output::Rewrite(event) => {
            if !matches!(
                &event,
                crate::proposal_rewrite::RewriteEvent::ToolStarted { .. }
            ) {
                rewrite_controls
                    .lock()
                    .expect("owned controls")
                    .queued_rewrites
                    .remove(&event.id());
            }
            let _ = output.send((event.id(), AppEvent::Rewrite(event)));
        }
        chat_worker::Output::Account(AccountEvent::Finished {
            id,
            provider,
            reply: AccountReply::Models(models),
        }) => {
            let _ = messages.send(Message::Models(id, provider, models));
        }
        chat_worker::Output::Account(event) => {
            let id = match &event {
                AccountEvent::Login { id, .. } | AccountEvent::Finished { id, .. } => *id,
            };
            let _ = output.send((id, AppEvent::Account(event)));
        }
        chat_worker::Output::Idle => {
            let _ = messages.send(Message::ChatIdle);
        }
    });
    let mut chat = ChatWorker::start(&app, chat_emit, hooks.chat.clone())?;
    if app.vault_root().is_some() {
        match app.guarded_tools() {
            Ok(tools) => chat.handle.set_tools(Some(tools))?,
            Err(error) if error.kind == ErrorKind::VaultUnavailable => {}
            Err(error) => return Err(error),
        }
    }
    controls.lock().expect("owned controls").chat = Some(chat.handle.clone());
    let _ = emit.send((
        startup,
        AppEvent::Ready {
            vault_bound: app.vault_root().is_some(),
            model_installed: app.model_installed(),
        },
    ));
    let mut indexing = indexing_job(&app, startup)?;
    let mut model: Option<InstallJob> = None;
    let mut model_ledger = HashMap::<Uuid, (bool, PathBuf)>::new();
    let mut ask_ledger = HashMap::<Uuid, AskRequest>::new();
    let mut final_error: Option<WorkflowError> = None;
    #[cfg(test)]
    let mut idle_barriers: Vec<mpsc::Sender<()>> = Vec::new();
    loop {
        // One bounded batch only when commands are not queued.
        let message = match rx.try_recv() {
            Ok(message) => message,
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {
                if let Some(id) = indexing.take() {
                    if indexing_job(&app, id)?.is_none() {
                        continue;
                    }
                    match app.embed_pending(16) {
                        Ok(Some(progress)) => {
                            let _ = emit.send((
                                id,
                                AppEvent::Indexing {
                                    embedded: progress.embedded,
                                    total: progress.total,
                                },
                            ));
                            if progress.embedded < progress.total {
                                indexing = indexing_job(&app, id)?;
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            let _ = emit.send((id, AppEvent::Failed(error)));
                        }
                    }
                    continue;
                }
                #[cfg(test)]
                for barrier in idle_barriers.drain(..) {
                    let _ = barrier.send(());
                }
                match rx.recv() {
                    Ok(message) => message,
                    Err(_) => break,
                }
            }
        };
        if matches!(message, Message::Shutdown) {
            break;
        }
        match message {
            Message::Models(id, provider, models) => match app.record_models(provider, &models) {
                Ok(()) => {
                    let _ = emit.send((
                        id,
                        AppEvent::Account(AccountEvent::Finished {
                            id,
                            provider,
                            reply: AccountReply::Models(models),
                        }),
                    ));
                }
                Err(error) => {
                    let _ = emit.send((
                        id,
                        AppEvent::Account(AccountEvent::Finished {
                            id,
                            provider,
                            reply: AccountReply::Rejected(error),
                        }),
                    ));
                }
            },
            Message::ModelProgress(id, received, total) => {
                if model.as_ref().is_some_and(|job| job.id == id) {
                    let _ = emit.send((id, AppEvent::ModelDownload { received, total }));
                }
            }
            Message::ModelDone(id) => {
                if let Some(job) = model.as_mut().filter(|job| job.id == id) {
                    let result = job
                        .join
                        .take()
                        .expect("admitted installer")
                        .join()
                        .map_err(|_| WorkflowError::msg("model installation job failed"))
                        .and_then(|result| result);
                    match result {
                        Ok(report) => {
                            let _ = emit.send((id, AppEvent::ModelDownloaded(report.clone())));
                            job.downloaded = Some(report);
                        }
                        Err(error) => {
                            let _ = emit.send((id, AppEvent::Failed(error)));
                            model = None;
                            controls.lock().expect("owned controls").model = None;
                        }
                    }
                }
            }
            Message::ChatIdle => {}
            #[cfg(test)]
            Message::IdleBarrier(barrier) => idle_barriers.push(barrier),
            Message::Command(id, command) => {
                if stopping.load(Ordering::Acquire) && !critical_mutation_command(&command) {
                    let _ = emit.send((id, cancelled_command(command)));
                    continue;
                }
                let result = dispatch(
                    &mut app,
                    &chat.handle,
                    id,
                    command,
                    &mut model,
                    &mut model_ledger,
                    &mut ask_ledger,
                    &tx,
                    &emit,
                    &controls,
                    &hooks,
                    &mut indexing,
                );
                if let Err(error) = result {
                    let _ = emit.send((id, AppEvent::Failed(error)));
                }
            }
            Message::Shutdown => unreachable!(),
        }
        if let Some(job) = model.as_ref()
            && let Some(report) = &job.downloaded
        {
            let id = job.id;
            if job.cancel.load(Ordering::Acquire) {
                let _ = emit.send((id, AppEvent::Failed(WorkflowError::cancelled())));
                model = None;
                controls.lock().expect("owned controls").model = None;
                continue;
            }
            // The chat lane stays active until all tool leases have drained.
            match chat.handle.set_tools(None) {
                Err(error) if error.kind == ErrorKind::ToolsBusy => continue,
                Err(error) => {
                    let _ = emit.send((id, AppEvent::Failed(error)));
                }
                Ok(()) => {
                    #[cfg(test)]
                    let result = if let Some(load) = &hooks.load {
                        app.activate_model_with(&report.directory, &job.cancel, |_, directory| {
                            load(directory, &job.cancel).map(Some)
                        })
                    } else {
                        app.activate_model_cancellable(&report.directory, &job.cancel)
                    };
                    #[cfg(not(test))]
                    let result = app.activate_model_cancellable(&report.directory, &job.cancel);
                    let mut activation = result;
                    match app.guarded_tools() {
                        Ok(tools) => {
                            if let Err(error) = chat.handle.set_tools(Some(tools)) {
                                activation = Err(error);
                            }
                        }
                        Err(error)
                            if error.kind == ErrorKind::VaultNotBound
                                || error.kind == ErrorKind::VaultUnavailable => {}
                        Err(error) => {
                            activation = Err(error);
                        }
                    }
                    match activation {
                        Ok(()) => {
                            let _ = emit.send((id, AppEvent::ModelInstalled));
                            indexing = indexing_job(&app, id)?;
                        }
                        Err(error) => {
                            let _ = emit.send((id, AppEvent::Failed(error)));
                        }
                    }
                }
            }
            model = None;
            controls.lock().expect("owned controls").model = None;
        }
    }
    chat.handle.cancel_all();
    if let Some(mut job) = model.take() {
        job.cancel.store(true, Ordering::Release);
        let result = if let Some(join) = job.join.take() {
            join.join()
                .map_err(|_| WorkflowError::msg("model installation job failed while draining"))
                .and_then(|r| r)
        } else {
            Err(WorkflowError::cancelled())
        };
        let error = match result {
            Ok(_) => WorkflowError::cancelled(),
            Err(error) => error,
        };
        if error.kind != ErrorKind::Cancelled {
            final_error.get_or_insert(error.clone());
        }
        let _ = emit.send((job.id, AppEvent::Failed(error)));
    }
    if let Err(error) = chat.shutdown() {
        final_error.get_or_insert(error);
    }
    // Discovery replies queued during shutdown must still get a terminal correlated result.
    while let Ok(message) = rx.try_recv() {
        if let Message::Models(id, provider, models) = message {
            match app.record_models(provider, &models) {
                Ok(()) => {
                    let _ = emit.send((
                        id,
                        AppEvent::Account(AccountEvent::Finished {
                            id,
                            provider,
                            reply: AccountReply::Models(models),
                        }),
                    ));
                }
                Err(error) => {
                    let _ = emit.send((
                        id,
                        AppEvent::Account(AccountEvent::Finished {
                            id,
                            provider,
                            reply: AccountReply::Rejected(error.clone()),
                        }),
                    ));
                    final_error.get_or_insert(error);
                }
            }
        }
    }
    controls.lock().expect("owned controls").chat = None;
    controls
        .lock()
        .expect("owned controls")
        .queued_rewrites
        .clear();
    controls.lock().expect("owned controls").model = None;
    drop(chat);
    drop(app);
    final_error.map_or(Ok(()), Err)
}

#[allow(clippy::too_many_arguments)]
fn dispatch(
    app: &mut App,
    chat: &ChatHandle,
    id: Uuid,
    command: AppCommand,
    model: &mut Option<InstallJob>,
    model_ledger: &mut HashMap<Uuid, (bool, PathBuf)>,
    ask_ledger: &mut HashMap<Uuid, AskRequest>,
    tx: &mpsc::Sender<Message>,
    emit: &mpsc::Sender<(Uuid, AppEvent)>,
    controls: &Arc<Mutex<Controls>>,
    hooks: &Hooks,
    indexing: &mut Option<Uuid>,
) -> Result<()> {
    let event = match command {
        #[cfg(test)]
        AppCommand::TestPause { entered, release } => {
            entered.send(()).expect("test observer");
            release.recv().expect("test release");
            AppEvent::EditRecovered
        }
        AppCommand::Status => AppEvent::Status(AppStatus {
            vault_root: app.vault_root().map(PathBuf::from),
            model_installed: app.model_installed(),
            model_download: model.as_ref().map(|job| job.id),
        }),
        AppCommand::BindVault(root) => {
            app.bind_vault(&root)?;
            chat.set_tools(Some(app.guarded_tools()?))?;
            *indexing = indexing_job(app, id)?;
            AppEvent::VaultBound
        }
        AppCommand::Refresh => {
            if let Err(error) = app.tools() {
                if error.kind != ErrorKind::VaultUnavailable {
                    return Err(error);
                }
                let root = app.vault_root().ok_or(error)?.to_owned();
                app.bind_vault(&root)?;
                chat.set_tools(Some(app.guarded_tools()?))?;
            }
            let report = app.refresh()?;
            *indexing = indexing_job(app, id)?;
            AppEvent::Refreshed(report)
        }
        AppCommand::Selection => AppEvent::Selection(app.selection()?),
        AppCommand::Select(selection) => {
            app.select(selection)?;
            AppEvent::SelectionSaved
        }
        AppCommand::Effort => AppEvent::Effort(app.effort()?),
        AppCommand::SelectEffort(effort) => {
            app.select_effort(effort)?;
            AppEvent::EffortSaved
        }
        AppCommand::Notes { folder, cursor } => {
            AppEvent::Notes(app.notes(folder.as_deref(), cursor.as_deref())?)
        }
        AppCommand::Note(path) => AppEvent::Note(app.note(&path)?),
        AppCommand::ProposalSource(path) => {
            AppEvent::ProposalSource(Box::new(app.proposal_source(&path)?))
        }
        AppCommand::NoteIdentity(path) => AppEvent::NoteIdentity(app.note_identity(&path)?),
        AppCommand::IdentityInventory => {
            AppEvent::IdentityInventory(Box::new(app.identity_inventory()?))
        }
        AppCommand::ResolveNoteIdentity(note_id) => {
            AppEvent::NoteIdentityResolved(Box::new(app.resolve_note_identity(note_id)?))
        }
        AppCommand::EvidenceNote(path) => AppEvent::EvidenceNote(app.evidence_note(&path)?),
        AppCommand::PrepareNoteIdentity(request) => {
            AppEvent::NoteIdentityDraft(Box::new(app.prepare_note_identity(&request)?))
        }
        AppCommand::CreateProposal(request) => AppEvent::Proposal(app.create_proposal(&request)?),
        AppCommand::Proposal(proposal) => AppEvent::Proposal(app.proposal(proposal)?),
        AppCommand::Proposals(group) => AppEvent::Proposals(app.proposals(group)?),
        AppCommand::EditProposal(edit) => AppEvent::Proposal(app.edit_proposal(&edit)?),
        AppCommand::RewriteProposal(edit) => AppEvent::Proposal(app.rewrite_proposal(&edit)?),
        AppCommand::ProposalRewrite(operation) => {
            AppEvent::ProposalRewrite(app.work_store().proposal_rewrite(operation)?)
        }
        AppCommand::StartProposalRewrite(request) => {
            let result = (|| {
                request.validate()?;
                // Historical replay precedes filesystem, discovery and account access.
                if let Some(job) = app.work_store().proposal_rewrite(request.id)? {
                    request.check_replay(&job)?;
                    return Ok(Some(crate::proposal_rewrite::RewriteEvent::replay(
                        &request, job,
                    )));
                }
                let cancel = controls
                    .lock()
                    .expect("owned controls")
                    .queued_rewrites
                    .get(&request.id)
                    .cloned()
                    .unwrap_or_default();
                if cancel.is_cancelled() {
                    return Err(WorkflowError::cancelled());
                }
                let record = app.proposal(request.expected.id)?;
                if record.stamp() != request.expected
                    || record.state != crate::proposals::ProposalState::Draft
                {
                    return Err(WorkflowError::typed(
                        ErrorKind::ContextStale,
                        "proposal review changed before Rewrite",
                    ));
                }
                app.refresh()?;
                let vault: brn_store::files::VaultRecord = serde_json::from_str(
                    &app.work_store()
                        .setting("vault.editor_identity")?
                        .ok_or_else(|| {
                            WorkflowError::typed(
                                ErrorKind::VaultNotBound,
                                "choose a vault before Rewrite",
                            )
                        })?,
                )
                .map_err(|_| WorkflowError::msg("invalid saved vault identity"))?;
                if vault != record.draft.vault {
                    return Err(WorkflowError::typed(
                        ErrorKind::ContextStale,
                        "proposal belongs to a different vault binding",
                    ));
                }
                app.validate_selection(&request.selection)?;
                drop(app.tools()?);
                chat.rewrite(request.clone(), cancel)?;
                Ok(None)
            })();
            match result {
                Ok(Some(event)) => {
                    controls
                        .lock()
                        .expect("owned controls")
                        .queued_rewrites
                        .remove(&id);
                    let _ = emit.send((id, AppEvent::Rewrite(event)));
                }
                Ok(None) => {}
                Err(error) => {
                    controls
                        .lock()
                        .expect("owned controls")
                        .queued_rewrites
                        .remove(&id);
                    let _ = emit.send((
                        id,
                        AppEvent::Rewrite(crate::proposal_rewrite::RewriteEvent::rejected(
                            &request, error,
                        )),
                    ));
                }
            }
            return Ok(());
        }
        AppCommand::AddProposalComment(comment) => {
            AppEvent::Proposal(app.add_proposal_comment(&comment)?)
        }
        AppCommand::UpdateProposalComment(comment) => {
            AppEvent::Proposal(app.update_proposal_comment(&comment)?)
        }
        AppCommand::RemoveProposalComment { expected, comment } => {
            AppEvent::Proposal(app.remove_proposal_comment(expected, comment)?)
        }
        AppCommand::RejectProposal(stamp) => AppEvent::Proposal(app.reject_proposal(stamp)?),
        AppCommand::ApproveProposal(request) => {
            AppEvent::ProposalApplied(app.approve_proposal(&request)?)
        }
        AppCommand::ReconcileProposal(id) => AppEvent::ProposalApplied(app.reconcile_proposal(id)?),
        AppCommand::PreviewProposalUndo(request) => {
            AppEvent::ProposalUndoPreview(app.preview_proposal_undo(&request)?)
        }
        AppCommand::UndoProposal(request) => {
            AppEvent::ProposalApplied(app.undo_proposal(&request)?)
        }
        AppCommand::PreviewProposalRepair(id) => {
            AppEvent::ProposalRepairPreview(app.preview_proposal_repair(id)?)
        }
        AppCommand::RepairProposal(request) => {
            AppEvent::ProposalRepaired(app.repair_proposal(&request)?)
        }
        AppCommand::ApproveProposalGroup(request) => {
            AppEvent::ProposalGroupApplied(app.approve_proposal_group(&request)?)
        }
        AppCommand::ProposalApplies => {
            AppEvent::ProposalApplies(app.work_store().proposal_applies()?)
        }
        AppCommand::ProposalRecovery => {
            AppEvent::ProposalRecovery(app.proposal_recovery_operations()?)
        }
        AppCommand::ProposalApply(operation) => {
            AppEvent::ProposalApply(app.proposal_apply(operation)?.map(Box::new))
        }
        AppCommand::Activity(request) => AppEvent::Activity(app.activity(&request)?),
        AppCommand::ReloadEditor(request) => {
            AppEvent::EditorRecovered(app.reload_editor(&request)?)
        }
        AppCommand::OpenEditor(path) => AppEvent::Editor(app.open_editor(&path)?),
        AppCommand::RecoverEditor(request) => {
            AppEvent::EditorRecovered(app.recover_editor(&request)?)
        }
        AppCommand::SaveEditor(request) => AppEvent::EditorSaved(app.save_editor(&request)?),
        AppCommand::Editors => AppEvent::Editors(app.work_store().editors()?),
        AppCommand::ReconcileEditor(operation) => {
            AppEvent::EditorSaved(app.reconcile_editor(operation)?)
        }
        AppCommand::RecoverEdit {
            path,
            base_sha256,
            text,
        } => {
            VaultPath::parse(&path).map_err(|_| {
                WorkflowError::typed(ErrorKind::ToolRejected, "invalid recovery note path")
            })?;
            app.work_store_mut()
                .put_unsaved_edit(&path, base_sha256, &text)?;
            AppEvent::EditRecovered
        }
        AppCommand::Search { query, mode, limit } => {
            AppEvent::Search(app.search(&query, mode, limit)?)
        }
        AppCommand::Conversations => AppEvent::Conversations(app.conversations()?),
        AppCommand::Turns(conversation) => AppEvent::Turns(app.turns(conversation)?),
        AppCommand::Turn(turn) => AppEvent::Turn(app.work_store().turn(turn)?),
        AppCommand::Ask(request) => {
            let result = (|| {
                if let Some(previous) = ask_ledger.get(&id)
                    && previous != &request
                {
                    return Err(chat_worker::conflict());
                }
                // History-only replay is before refresh, selection and credential access.
                if let Some(turn) = app.work_store().turn(id)? {
                    chat_worker::check_replay(&request, &turn)?;
                    ask_ledger.insert(id, request.clone());
                    return Ok(Some(chat_worker::replay(&request, turn)));
                }
                if request.effort.is_none() {
                    return Err(WorkflowError::typed(
                        ErrorKind::SelectionRequired,
                        "choose an explicit reasoning effort before asking AI",
                    ));
                }
                if let Some(conversation) = request.conversation {
                    app.turns(conversation)?;
                }
                app.refresh()?;
                app.validate_selection(&request.selection)?;
                // Tools were installed at binding/startup. This lease verifies current availability.
                drop(app.tools()?);
                ask_ledger.insert(id, request.clone());
                chat.ask(request.clone())?;
                Ok(None)
            })();
            match result {
                Ok(Some(event)) => {
                    let _ = emit.send((id, AppEvent::Chat(event)));
                }
                Ok(None) => {}
                Err(error) => {
                    let _ = emit.send((id, AppEvent::Chat(chat_worker::rejected(&request, error))));
                }
            }
            return Ok(());
        }
        AppCommand::Account { id, command } => {
            chat.account(id, command)?;
            return Ok(());
        }
        AppCommand::ModelPrompt => AppEvent::ModelPrompt(app.model_download_prompt()?),
        AppCommand::DownloadModel { consent, target } => {
            if model_ledger.contains_key(&id) {
                return Err(chat_worker::conflict());
            }
            if model.is_some() {
                return Err(WorkflowError::typed(
                    ErrorKind::ToolsBusy,
                    "a model installation is already pending",
                ));
            }
            model_ledger.insert(id, (consent, target.clone()));
            #[cfg(test)]
            let request = if consent && hooks.install.is_some() {
                app.work_store_mut()
                    .set_setting("model.download_decision", "approved")?;
                Some(crate::models::ModelInstallRequest::test_request(
                    target.clone(),
                ))
            } else {
                app.prepare_model_download(consent, &target)?
            };
            #[cfg(not(test))]
            let request = app.prepare_model_download(consent, &target)?;
            let Some(request) = request else {
                let _ = emit.send((id, AppEvent::ModelDownloadDeclined));
                return Ok(());
            };
            let cancel = Arc::new(AtomicBool::new(false));
            let (cancel_job, messages) = (cancel.clone(), tx.clone());
            #[cfg(test)]
            let fake = hooks.install.clone();
            #[cfg(not(test))]
            let _ = hooks;
            let join = thread::Builder::new()
                .name("brn-model-install".into())
                .spawn(move || {
                    let progress_tx = messages.clone();
                    let progress = move |received, total| {
                        let _ = progress_tx.send(Message::ModelProgress(id, received, total));
                    };
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        #[cfg(test)]
                        let result = if let Some(fake) = fake {
                            fake(request, &cancel_job, &progress)
                        } else {
                            request.install(&cancel_job, progress)
                        };
                        #[cfg(not(test))]
                        let result = request.install(&cancel_job, progress);
                        result
                    }))
                    .map_err(|_| WorkflowError::msg("model installation job failed"))
                    .and_then(|r| r);
                    let _ = messages.send(Message::ModelDone(id));
                    result
                })
                .map_err(|_| WorkflowError::msg("could not start model installation job"))?;
            *model = Some(InstallJob {
                id,
                cancel: cancel.clone(),
                join: Some(join),
                downloaded: None,
            });
            controls.lock().expect("owned controls").model = Some((id, cancel));
            return Ok(());
        }
        AppCommand::CancelTurn(_)
        | AppCommand::CancelAccount(_)
        | AppCommand::CancelModelDownload(_) => unreachable!("immediate handle controls"),
    };
    let _ = emit.send((id, event));
    Ok(())
}

#[cfg(test)]
pub(crate) fn start_test(
    data: PathBuf,
    config: AppConfig,
    chat: chat_worker::Hooks,
    install: Option<super::simple_worker_tests::InstallHook>,
    load: Option<super::simple_worker_tests::LoadHook>,
) -> Result<AppWorker> {
    AppWorker::start_owned(
        data,
        config,
        Hooks {
            chat,
            install,
            load,
        },
    )
}

#[cfg(test)]
pub(crate) fn wait_test_idle(worker: &AppWorker) {
    let (tx, rx) = mpsc::channel();
    worker.tx.send(Message::IdleBarrier(tx)).unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap();
}

fn critical_mutation_command(command: &AppCommand) -> bool {
    matches!(
        command,
        AppCommand::ReloadEditor(_)
            | AppCommand::RecoverEditor(_)
            | AppCommand::SaveEditor(_)
            | AppCommand::ReconcileEditor(_)
            | AppCommand::RecoverEdit { .. }
            | AppCommand::CreateProposal(_)
            | AppCommand::EditProposal(_)
            | AppCommand::RewriteProposal(_)
            | AppCommand::AddProposalComment(_)
            | AppCommand::UpdateProposalComment(_)
            | AppCommand::RemoveProposalComment { .. }
            | AppCommand::RejectProposal(_)
            | AppCommand::UndoProposal(_)
            | AppCommand::RepairProposal(_)
            | AppCommand::ApproveProposal(_)
            | AppCommand::ReconcileProposal(_)
            | AppCommand::ApproveProposalGroup(_)
    )
}

#[cfg(all(test, target_os = "macos"))]
mod editor_shutdown_tests {
    use super::*;
    use crate::editor::{EditRequest, SaveRequest};

    #[test]
    fn stop_withdraws_rewrite_queued_behind_application_work_before_admission() {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        let vault = base.path().join("vault");
        std::fs::create_dir(&data).unwrap();
        std::fs::create_dir(&vault).unwrap();
        let config = AppConfig {
            vault_root: Some(vault),
            credentials_dir: None,
            model_dir: None,
        };
        let mut worker = AppWorker::start(data.clone(), config).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let (entered_tx, entered) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        let pause = Uuid::new_v4();
        worker
            .submit(
                pause,
                AppCommand::TestPause {
                    entered: entered_tx,
                    release: release_rx,
                },
            )
            .unwrap();
        entered.recv_timeout(Duration::from_secs(10)).unwrap();
        let operation = Uuid::new_v4();
        worker
            .submit(
                operation,
                AppCommand::StartProposalRewrite(crate::proposal_rewrite::RewriteRequest {
                    id: operation,
                    // The proposal need not exist: Stop precedes proposal/vault preflight.
                    expected: crate::proposals::ProposalStamp {
                        id: Uuid::new_v4(),
                        version: 1,
                    },
                    selection: Selection {
                        provider: Provider::Chatgpt,
                        model: "gpt-5.5".into(),
                    },
                    effort: crate::proposal_rewrite::ReasoningEffort::High,
                    generation: 27,
                }),
            )
            .unwrap();
        let stop = Uuid::new_v4();
        worker
            .submit(stop, AppCommand::CancelTurn(operation))
            .unwrap();
        assert!(
            matches!(worker.recv_event_timeout(Duration::from_secs(10)).unwrap(),
            (id, AppEvent::TurnCancelRequested { accepted: true, .. }) if id == stop)
        );
        release.send(()).unwrap();
        loop {
            let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
            if id == operation {
                assert!(
                    matches!(event, AppEvent::Rewrite(crate::proposal_rewrite::RewriteEvent::Rejected { generation: 27, error, .. }) if error.kind == ErrorKind::Cancelled)
                );
                break;
            }
        }
        worker.shutdown().unwrap();
        assert!(worker.controls.lock().unwrap().queued_rewrites.is_empty());
        let (store, _) = brn_store::WorkStore::open(&data).unwrap();
        assert!(store.proposal_rewrite(operation).unwrap().is_none());
    }

    #[test]
    fn shutdown_drains_admitted_proposal_review_work_without_applying_notes() {
        use crate::proposals::{
            CommentRequest, CommentTarget, DraftNoteChange, DraftRequest, ProposalEdit,
            ProposalStamp, ProposalState, ReviewComment,
        };
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        let vault = base.path().join("vault");
        std::fs::create_dir(&data).unwrap();
        std::fs::create_dir(&vault).unwrap();
        let config = || AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: None,
            model_dir: None,
        };
        let mut worker = AppWorker::start(data.clone(), config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let (entered_tx, entered) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        worker
            .submit(
                Uuid::new_v4(),
                AppCommand::TestPause {
                    entered: entered_tx,
                    release: release_rx,
                },
            )
            .unwrap();
        entered.recv_timeout(Duration::from_secs(10)).unwrap();
        let id = Uuid::new_v4();
        let comment_id = Uuid::new_v4();
        let operations = [
            AppCommand::CreateProposal(DraftRequest {
                id,
                group_id: None,
                session_id: None,
                title: "Initial".into(),
                changes: vec![DraftNoteChange::Create {
                    path: "new.md".into(),
                    text: "Initial text".into(),
                }],
                sources: vec![],
            }),
            AppCommand::EditProposal(ProposalEdit {
                expected: ProposalStamp { id, version: 1 },
                title: "Edited".into(),
                texts: vec![Some("My later work".into())],
            }),
            AppCommand::AddProposalComment(CommentRequest {
                expected: ProposalStamp { id, version: 2 },
                comment: ReviewComment {
                    id: comment_id,
                    text: "Review before approval".into(),
                    target: CommentTarget::Proposal,
                },
            }),
        ];
        let ids: Vec<_> = operations
            .into_iter()
            .map(|command| {
                let op = Uuid::new_v4();
                worker.submit(op, command).unwrap();
                op
            })
            .collect();
        let rewrite = Uuid::new_v4();
        worker
            .submit(
                rewrite,
                AppCommand::StartProposalRewrite(crate::proposal_rewrite::RewriteRequest {
                    id: rewrite,
                    expected: ProposalStamp { id, version: 3 },
                    selection: Selection {
                        provider: Provider::Chatgpt,
                        model: "gpt-5.5".into(),
                    },
                    effort: crate::proposal_rewrite::ReasoningEffort::High,
                    generation: 9,
                }),
            )
            .unwrap();
        let stopping = worker.stopping.clone();
        let join = std::thread::spawn(move || {
            worker.shutdown().unwrap();
            worker
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !stopping.load(Ordering::Acquire) {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        release.send(()).unwrap();
        let worker = join.join().unwrap();
        let mut acknowledged = std::collections::HashSet::new();
        let mut rewrite_cancelled = false;
        while let Some((op, event)) = worker.try_event() {
            rewrite_cancelled |= op == rewrite
                && matches!(&event,
                AppEvent::Rewrite(crate::proposal_rewrite::RewriteEvent::Rejected { error, .. })
                if error.kind == ErrorKind::Cancelled);
            if ids.contains(&op) && matches!(event, AppEvent::Proposal(_)) {
                acknowledged.insert(op);
            }
        }
        assert_eq!(acknowledged.len(), ids.len());
        assert!(rewrite_cancelled);
        drop(worker);
        let app = App::open(&data, config()).unwrap();
        assert!(
            app.work_store()
                .proposal_rewrite(rewrite)
                .unwrap()
                .is_none()
        );
        let record = app.proposal(id).unwrap();
        assert_eq!(record.version, 3);
        assert_eq!(record.state, ProposalState::Draft);
        assert_eq!(record.draft.changes[0].text(), Some("My later work"));
        assert_eq!(record.comments[0].id, comment_id);
        assert_eq!(std::fs::read_dir(vault).unwrap().count(), 0);
    }

    #[test]
    fn unfinished_save_restarts_worker_and_keeps_recovery_commands_available() {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        std::fs::create_dir(base.path().join("data")).unwrap();
        std::fs::create_dir(base.path().join("vault")).unwrap();
        std::fs::write(base.path().join("vault/a.md"), "baseline").unwrap();
        let config = || AppConfig {
            vault_root: Some(base.path().join("vault")),
            credentials_dir: Some(base.path().join("credentials")),
            model_dir: None,
        };
        let mut app = App::open(&base.path().join("data"), config()).unwrap();
        let record = app.open_editor("a.md").unwrap().record;
        let operation = Uuid::new_v4();
        let request = SaveRequest {
            operation_id: operation,
            edit: EditRequest {
                path: "a.md".into(),
                expected: record.stamp,
                generation: 1,
                text: "unfinished".into(),
            },
            destination: None,
        };
        app.work_store_mut()
            .begin_editor_save(
                &request,
                &std::path::Path::new("a.md").with_file_name(format!(".brn-{operation}.stage")),
            )
            .unwrap();
        drop(app);
        let mut worker = AppWorker::start(base.path().join("data"), config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let query = |command| {
            let id = Uuid::new_v4();
            worker.submit(id, command).unwrap();
            loop {
                let (actual, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
                if actual == id {
                    break event;
                }
            }
        };
        let AppEvent::Editors(records) = query(AppCommand::Editors) else {
            panic!("recovery list");
        };
        assert_eq!(records[0].text, "unfinished");
        let AppEvent::Editor(view) = query(AppCommand::OpenEditor("a.md".into())) else {
            panic!("editor recovery");
        };
        assert_eq!(view.pending, vec![operation]);
        assert!(
            matches!(query(AppCommand::Search { query: "baseline".into(), mode: SearchMode::Keyword, limit: 10 }), AppEvent::Failed(error) if error.kind == ErrorKind::SaveUncertain)
        );
        assert!(
            matches!(query(AppCommand::ReconcileEditor(operation)), AppEvent::EditorSaved(receipt) if receipt.outcome == crate::editor::SaveOutcome::NotApplied)
        );
        assert!(matches!(
            query(AppCommand::Search {
                query: "baseline".into(),
                mode: SearchMode::Keyword,
                limit: 10
            }),
            AppEvent::Search(_)
        ));
        worker.shutdown().unwrap();
        assert_eq!(
            std::fs::read(base.path().join("vault/a.md")).unwrap(),
            b"baseline"
        );
    }

    #[test]
    fn shutdown_drains_already_admitted_editor_mutations() {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        std::fs::create_dir(base.path().join("data")).unwrap();
        std::fs::create_dir(base.path().join("vault")).unwrap();
        std::fs::write(base.path().join("vault/a.md"), "baseline").unwrap();
        let config = || AppConfig {
            vault_root: Some(base.path().join("vault")),
            credentials_dir: Some(base.path().join("credentials")),
            model_dir: None,
        };
        let mut worker = AppWorker::start(base.path().join("data"), config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let open = Uuid::new_v4();
        worker
            .submit(open, AppCommand::OpenEditor("a.md".into()))
            .unwrap();
        let record = loop {
            let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
            if id == open {
                let AppEvent::Editor(view) = event else {
                    panic!("editor open");
                };
                break view.record;
            }
        };
        let (entered_tx, entered) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        worker
            .submit(
                Uuid::new_v4(),
                AppCommand::TestPause {
                    entered: entered_tx,
                    release: release_rx,
                },
            )
            .unwrap();
        entered.recv_timeout(Duration::from_secs(10)).unwrap();
        let edit = EditRequest {
            path: "a.md".into(),
            expected: record.stamp,
            generation: 1,
            text: "admitted buffer".into(),
        };
        let recovery = Uuid::new_v4();
        worker
            .submit(recovery, AppCommand::RecoverEditor(edit.clone()))
            .unwrap();
        let save = Uuid::new_v4();
        worker
            .submit(
                save,
                AppCommand::SaveEditor(SaveRequest {
                    operation_id: save,
                    edit: EditRequest {
                        generation: 2,
                        text: "admitted save".into(),
                        ..edit
                    },
                    destination: None,
                }),
            )
            .unwrap();
        let stopping = worker.stopping.clone();
        let join = std::thread::spawn(move || {
            worker.shutdown().unwrap();
            worker
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !stopping.load(Ordering::Acquire) {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        release.send(()).unwrap();
        let worker = join.join().unwrap();
        let mut recovered = false;
        let mut saved = false;
        while let Some((id, event)) = worker.try_event() {
            recovered |= id == recovery && matches!(event, AppEvent::EditorRecovered(_));
            saved |= id == save && matches!(event, AppEvent::EditorSaved(_));
        }
        assert!(recovered && saved);
        assert_eq!(
            std::fs::read(base.path().join("vault/a.md")).unwrap(),
            b"admitted save"
        );
        drop(worker);
        let app = App::open(&base.path().join("data"), config()).unwrap();
        assert_eq!(
            app.work_store().editor("a.md").unwrap().unwrap().text,
            "admitted save"
        );
    }
}
