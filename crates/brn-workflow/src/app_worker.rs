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
use brn_ai::{ModelOption, NotePage, Provider, Selection};
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
    BindVault(PathBuf),
    Status,
    Refresh,
    Selection,
    Select(Selection),
    Notes {
        folder: Option<String>,
        cursor: Option<String>,
    },
    Note(String),
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
    Refreshed(RefreshReport),
    Notes(NotePage),
    Note(NoteText),
    EditRecovered,
    Search(SearchResults),
    Conversations(Vec<WorkConversation>),
    Turns(Vec<WorkTurn>),
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
            || matches!(&command, AppCommand::Account { id: operation, .. } if *operation != id)
        {
            return Err(chat_worker::conflict());
        }
        // These commands never wait behind a scan, model load, stream or device login.
        match command {
            AppCommand::CancelTurn(turn) => {
                let accepted = self
                    .controls
                    .lock()
                    .expect("owned controls")
                    .chat
                    .as_ref()
                    .is_some_and(|chat| chat.cancel(turn));
                self.output(id, AppEvent::TurnCancelRequested { turn, accepted })
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
                ErrorKind::VaultNotBound | ErrorKind::VaultUnavailable
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
    let (output, messages) = (emit.clone(), tx.clone());
    let chat_emit: chat_worker::Emit = Arc::new(move |event| match event {
        chat_worker::Output::Chat(event) => {
            let _ = output.send((event.id(), AppEvent::Chat(event)));
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
        match app.tools() {
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
                if stopping.load(Ordering::Acquire) {
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
                    match app.tools() {
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
        AppCommand::Status => AppEvent::Status(AppStatus {
            vault_root: app.vault_root().map(PathBuf::from),
            model_installed: app.model_installed(),
            model_download: model.as_ref().map(|job| job.id),
        }),
        AppCommand::BindVault(root) => {
            app.bind_vault(&root)?;
            chat.set_tools(Some(app.tools()?))?;
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
                chat.set_tools(Some(app.tools()?))?;
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
        AppCommand::Notes { folder, cursor } => {
            AppEvent::Notes(app.notes(folder.as_deref(), cursor.as_deref())?)
        }
        AppCommand::Note(path) => AppEvent::Note(app.note(&path)?),
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
