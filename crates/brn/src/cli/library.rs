//! Simple authority dispatch and the owned, correlated application event loop.
use super::{
    ai::{self, AiCommand},
    error::{classify_workflow, CliError},
    CliFailure, Command, Invocation, Output,
};
use brn_workflow::{
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    chat_worker::{AccountCommand, AccountEvent, AccountReply, AskRequest, ChatEvent},
    library::SearchMode,
    ErrorKind, Provider, Selection, WorkTurn, WorkTurnStatus, WorkspaceMode,
};
use serde_json::{json, Value};
use std::{
    io::Write as _,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
use uuid::Uuid;

fn typed(kind: ErrorKind, message: &str) -> CliFailure {
    CliError::Typed(kind, message.into()).into()
}

pub fn validate_workspace(i: &Invocation) -> Result<(), CliFailure> {
    if brn_workflow::workspace_mode(&i.data_dir).map_err(classify_workflow)?
        == WorkspaceMode::Legacy
    {
        return Err(typed(
            ErrorKind::WorkspaceModeConflict,
            "legacy workspace markers require a separate current data directory",
        ));
    }
    Ok(())
}

fn config(i: &Invocation) -> AppConfig {
    AppConfig {
        vault_root: i.vault.clone(),
        credentials_dir: i.credentials_dir.clone(),
        // This option is an install destination, not a missing model to load.
        model_dir: if matches!(i.command, Command::ModelDownload { .. }) {
            None
        } else {
            i.model_dir.clone()
        },
    }
}

#[derive(Clone, Copy)]
enum Job {
    Local,
    Ask(Uuid),
    Account(Uuid),
    Download(Uuid),
}

trait EventLane {
    fn submit(&self, id: Uuid, command: AppCommand) -> brn_workflow::Result<()>;
    fn try_event(&self) -> Option<(Uuid, AppEvent)>;
    fn recv_event_timeout(
        &self,
        duration: Duration,
    ) -> Result<(Uuid, AppEvent), std::sync::mpsc::RecvTimeoutError>;
    fn shutdown(&mut self) -> brn_workflow::Result<()>;
}

impl EventLane for AppWorker {
    fn submit(&self, id: Uuid, command: AppCommand) -> brn_workflow::Result<()> {
        self.submit(id, command)
    }
    fn try_event(&self) -> Option<(Uuid, AppEvent)> {
        self.try_event()
    }
    fn recv_event_timeout(
        &self,
        duration: Duration,
    ) -> Result<(Uuid, AppEvent), std::sync::mpsc::RecvTimeoutError> {
        self.recv_event_timeout(duration)
    }
    fn shutdown(&mut self) -> brn_workflow::Result<()> {
        self.shutdown()
    }
}

struct Lane<W = AppWorker> {
    worker: W,
    deadline: Instant,
    stopped: Option<bool>,
    joined: bool,
    shutdown_error: Option<brn_workflow::WorkflowError>,
    observe_cancel: bool,
}

impl Lane<AppWorker> {
    fn start(i: &Invocation, seconds: u64) -> Result<Self, CliFailure> {
        Self::start_observing(i, seconds, true)
    }

    fn start_observing(
        i: &Invocation,
        seconds: u64,
        observe_cancel: bool,
    ) -> Result<Self, CliFailure> {
        let mut lane = Self {
            worker: AppWorker::start(i.data_dir.clone(), config(i)).map_err(classify_workflow)?,
            deadline: Instant::now() + Duration::from_secs(seconds),
            stopped: None,
            joined: false,
            shutdown_error: None,
            observe_cancel,
        };
        let ready = (|| loop {
            match lane.next(Job::Local)? {
                (_, AppEvent::Ready { .. }) => break Ok(()),
                (_, AppEvent::Restored { backup }) => {
                    let _ = writeln!(std::io::stderr(), "restored work from {}", backup.display());
                }
                (_, AppEvent::Failed(error)) => break Err(classify_workflow(error).into()),
                _ => {}
            }
        })();
        if let Err(error) = ready {
            return lane.finish(Err(error));
        }
        Ok(lane)
    }
}

impl<W: EventLane> Lane<W> {
    fn next(&mut self, job: Job) -> Result<(Uuid, AppEvent), CliFailure> {
        self.next_observing(job, || crate::CANCEL.load(Ordering::SeqCst))
    }

    fn next_observing(
        &mut self,
        job: Job,
        signal: impl Fn() -> bool,
    ) -> Result<(Uuid, AppEvent), CliFailure> {
        loop {
            let queued = self.worker.try_event();
            // Confirmed terminal success wins; progress cannot starve cancellation.
            if queued
                .as_ref()
                .is_some_and(|(_, event)| confirmed_success(event))
            {
                return Ok(queued.expect("checked queued success"));
            }
            if self.stopped.is_none() {
                let signal = self.observe_cancel && signal();
                if signal || Instant::now() >= self.deadline {
                    self.stopped = Some(!signal);
                    let control = match job {
                        Job::Ask(id) => Some(AppCommand::CancelTurn(id)),
                        Job::Account(id) => Some(AppCommand::CancelAccount(id)),
                        Job::Download(id) => Some(AppCommand::CancelModelDownload(id)),
                        Job::Local => None,
                    };
                    if let Some(control) = control {
                        let _ = self.worker.submit(Uuid::new_v4(), control);
                    }
                    // Cancel acknowledgement can precede admission and be false.
                    // Shutdown fences queued admission, joins, and publishes final endings.
                    self.shutdown_error = self.worker.shutdown().err();
                    self.joined = true;
                }
            }
            if let Some(event) = queued {
                return Ok(event);
            }
            if let Some(event) = self.worker.try_event() {
                return Ok(event);
            }
            if self.joined {
                if let Some(error) = self.shutdown_error.clone() {
                    return Err(classify_workflow(error).into());
                }
                return Err(self.stop_error().into());
            }
            match self.worker.recv_event_timeout(Duration::from_millis(20)) {
                Ok(event) => return Ok(event),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(CliError::Workflow(
                        "application event lane closed without a terminal reply".into(),
                    )
                    .into())
                }
            }
        }
    }

    fn stop_error(&self) -> CliError {
        if self.stopped == Some(true) {
            CliError::Timeout("deadline reached; local work was stopped and joined; upstream cancellation is unconfirmed".into())
        } else {
            CliError::Interrupted("interrupted; local work was stopped and joined; upstream cancellation is unconfirmed".into())
        }
    }

    fn command_error(&self, error: brn_workflow::WorkflowError) -> CliError {
        if error.kind == ErrorKind::Cancelled && self.stopped.is_some() {
            self.stop_error()
        } else {
            classify_workflow(error)
        }
    }

    fn query(&mut self, command: AppCommand) -> Result<AppEvent, CliFailure> {
        self.query_with_id(Uuid::new_v4(), command)
    }

    fn query_with_id(&mut self, id: Uuid, command: AppCommand) -> Result<AppEvent, CliFailure> {
        self.worker
            .submit(id, command)
            .map_err(|error| self.command_error(error))?;
        loop {
            let (event_id, event) = self.next(Job::Local)?;
            if event_id != id {
                continue;
            }
            if let AppEvent::Failed(error) = event {
                return Err(self.command_error(error).into());
            }
            return Ok(event);
        }
    }

    fn finish<T>(&mut self, result: Result<T, CliFailure>) -> Result<T, CliFailure> {
        if !self.joined {
            self.shutdown_error = self.worker.shutdown().err();
            self.joined = true;
        }
        if let Some(error) = self.shutdown_error.take() {
            let mut failure = result
                .err()
                .unwrap_or_else(|| classify_workflow(error.clone()).into());
            let context = failure.context.get_or_insert_with(|| json!({}));
            context["shutdown_error"] =
                json!({"code": classify_workflow(error.clone()).code(), "message": error.message});
            // Finalization failure takes precedence over timeout/interruption.
            failure.error = classify_workflow(error);
            return Err(failure);
        }
        result
    }
}

fn confirmed_success(event: &AppEvent) -> bool {
    match event {
        AppEvent::Chat(ChatEvent::Finished { turn, .. }) => {
            turn.status == WorkTurnStatus::Completed
        }
        AppEvent::Account(AccountEvent::Finished { reply, .. }) => matches!(
            reply,
            AccountReply::Status(_) | AccountReply::Disconnected | AccountReply::Models(_)
        ),
        AppEvent::ModelInstalled => true,
        AppEvent::EditorRecovered(_)
        | AppEvent::EditorSaved(_)
        | AppEvent::ProposalApplied(_)
        | AppEvent::ProposalGroupApplied(_) => true,
        _ => false,
    }
}

fn output(data: Value) -> Output {
    Output {
        text: format!(
            "{}\n",
            serde_json::to_string_pretty(&data).expect("safe DTO")
        ),
        data,
    }
}

pub fn run(i: &Invocation) -> Result<Output, CliFailure> {
    if let Command::Activity(request) = &i.command {
        request.validate().map_err(classify_workflow)?;
    }
    let editor = if let Command::Editor(command) = &i.command {
        Some(super::editor::prepare(command)?)
    } else {
        None
    };
    let proposal = if let Command::Proposals(command) = &i.command {
        Some(super::proposals::prepare(command)?)
    } else {
        None
    };
    if matches!(i.command, Command::ModelDownload { .. })
        && !brn_workflow::native_retrieval_compiled()
    {
        return Err(typed(
            ErrorKind::SemanticUnavailableInBuild,
            "model download is unsupported in this build",
        ));
    }
    let timeout = match &i.command {
        Command::Ask {
            timeout_seconds, ..
        }
        | Command::ModelDownload { timeout_seconds } => *timeout_seconds,
        Command::Ai(AiCommand::Connect(_, seconds) | AiCommand::Models(_, seconds)) => *seconds,
        _ => 300,
    };
    let ask_id = if let Command::Ask { operation, .. } = i.command {
        Some(operation.unwrap_or_else(Uuid::new_v4))
    } else {
        None
    };
    let result = (|| {
        let mut lane = Lane::start(i, timeout)?;
        let result = execute(i, &mut lane, ask_id, editor, proposal);
        lane.finish(result)
    })();
    result.map_err(|mut failure: CliFailure| {
        if let (Some(op), Command::Ask { session, .. }) = (ask_id, &i.command) {
            if failure.context.is_none() {
                failure.context = Some(context(op, *session, None, None));
            }
        }
        failure
    })
}

fn execute(
    i: &Invocation,
    lane: &mut Lane,
    ask_id: Option<Uuid>,
    editor: Option<(Uuid, AppCommand)>,
    proposal: Option<(Uuid, AppCommand)>,
) -> Result<Output, CliFailure> {
    match &i.command {
        Command::Activity(request) => {
            let AppEvent::Activity(page) = lane.query(AppCommand::Activity(request.clone()))?
            else {
                return Err(unexpected());
            };
            Ok(super::activity::output(page))
        }
        Command::Proposals(_) => {
            let (id, command) = proposal.expect("proposal input prepared before startup");
            let data = match lane.query_with_id(id, command)? {
                AppEvent::Proposal(record) => json!(record),
                AppEvent::Proposals(records) => json!(records),
                AppEvent::ProposalApplied(receipt) => json!(receipt),
                AppEvent::ProposalGroupApplied(result) => json!(result),
                AppEvent::ProposalApplies(journals) => json!(journals),
                _ => return Err(unexpected()),
            };
            Ok(output(data))
        }
        Command::Editor(_) => {
            let (id, command) = editor.expect("editor input prepared before startup");
            let data = match lane.query_with_id(id, command)? {
                AppEvent::Editor(view) => json!(view),
                AppEvent::EditorRecovered(record) => json!(record),
                AppEvent::EditorSaved(receipt) => json!(receipt),
                AppEvent::Editors(records) => json!(records),
                _ => return Err(unexpected()),
            };
            Ok(output(data))
        }
        Command::Status => {
            let AppEvent::Status(status) = lane.query(AppCommand::Status)? else {
                return Err(unexpected());
            };
            Ok(output(
                json!({"version": env!("CARGO_PKG_VERSION"), "data_dir": i.data_dir, "mode": "simple",
                "vault_root": status.vault_root, "model_installed": status.model_installed,
                "model_download": status.model_download,
                "capabilities": {"native_retrieval": brn_workflow::native_retrieval_compiled()}}),
            ))
        }
        Command::NotesList { folder, cursor } => {
            let AppEvent::Notes(page) = lane.query(AppCommand::Notes {
                folder: folder.clone(),
                cursor: cursor.clone(),
            })?
            else {
                return Err(unexpected());
            };
            Ok(output(json!(page)))
        }
        Command::NotePath(path) => {
            let AppEvent::Note(note) = lane.query(AppCommand::Note(path.clone()))? else {
                return Err(unexpected());
            };
            Ok(Output {
                text: note.text.clone(),
                data: json!({"path": path, "text": note.text}),
            })
        }
        Command::Search {
            query,
            profile,
            limit,
        } => {
            let mode = profile.unwrap_or(SearchMode::Hybrid);
            let AppEvent::Search(results) = lane.query(AppCommand::Search {
                query: query.clone(),
                mode,
                limit: limit.unwrap_or(10),
            })?
            else {
                return Err(unexpected());
            };
            Ok(output(
                json!({"query": query, "hits": results.hits.iter().map(|h| json!({
                "path": h.path, "start_byte": h.start_byte, "end_byte": h.end_byte,
                "quote": h.quote, "score": h.score,
            })).collect::<Vec<_>>(), "keyword_only": results.keyword_only}),
            ))
        }
        Command::ConversationsList => {
            let AppEvent::Conversations(conversations) = lane.query(AppCommand::Conversations)?
            else {
                return Err(unexpected());
            };
            Ok(output(json!({"conversations": conversations})))
        }
        Command::ConversationsShow { session } => {
            let AppEvent::Turns(turns) = lane.query(AppCommand::Turns(*session))? else {
                return Err(unexpected());
            };
            Ok(output(
                json!({"session_id": session, "historical": true, "turns": turns.iter().map(turn_json).collect::<Vec<_>>()}),
            ))
        }
        Command::Ask {
            question, session, ..
        } => ask(
            lane,
            question,
            *session,
            ask_id.expect("ask operation allocated before startup"),
        ),
        Command::Ai(command) => account(i, lane, command),
        Command::ModelDownload { .. } => download(i, lane),
    }
}

fn unexpected() -> CliFailure {
    CliError::Workflow("unexpected terminal application reply".into()).into()
}

fn turn_json(turn: &WorkTurn) -> Value {
    json!({"operation_id": turn.id, "session_id": turn.conversation_id, "provider": turn.provider,
        "model": turn.model, "question": turn.question, "answer": turn.answer,
        "status": turn.status, "error_code": turn.error_code})
}

fn context(
    op: Uuid,
    session: Option<Uuid>,
    turn: Option<&WorkTurn>,
    partial: Option<&str>,
) -> Value {
    json!({"operation_id": op, "session_id": turn.map(|t| t.conversation_id).or(session),
        "recorded_status": turn.map(|t| t.status), "partial": partial,
        "receipt": turn.map(turn_json), "saved": turn.is_some(),
        "provider_outcome": if turn.is_some_and(|t| t.status == WorkTurnStatus::Completed) { "completed" } else { "unknown" }})
}

fn ask<W: EventLane>(
    lane: &mut Lane<W>,
    question: &str,
    session: Option<Uuid>,
    op: Uuid,
) -> Result<Output, CliFailure> {
    let result = (|| {
        // Replay uses its frozen recorded payload without a current-selection query.
        let AppEvent::Turn(recorded) = lane.query(AppCommand::Turn(op))? else {
            return Err(unexpected());
        };
        let selection = if let Some(turn) = &recorded {
            Selection {
                provider: ai::provider(&turn.provider)?,
                model: turn.model.clone(),
            }
        } else {
            let AppEvent::Selection(current) = lane.query(AppCommand::Selection)? else {
                return Err(unexpected());
            };
            current.ok_or_else(|| {
                typed(
                    ErrorKind::SelectionRequired,
                    "select an explicit provider and model with ai select before a new ask",
                )
            })?
        };
        lane.worker
            .submit(
                op,
                AppCommand::Ask(AskRequest {
                    id: op,
                    conversation: session,
                    question: question.into(),
                    selection,
                    generation: 0,
                }),
            )
            .map_err(|error| lane.command_error(error))?;
        wait_ask(lane, op, session)
    })();
    result.map_err(|mut failure: CliFailure| {
        if failure.context.is_none() {
            failure.context = Some(context(op, session, None, None));
        }
        failure
    })
}

fn wait_ask<W: EventLane>(
    lane: &mut Lane<W>,
    op: Uuid,
    session: Option<Uuid>,
) -> Result<Output, CliFailure> {
    let mut partial = String::new();
    loop {
        let (id, event) = lane.next(Job::Ask(op)).map_err(|mut failure| {
            if failure.context.is_none() {
                failure.context = Some(context(op, session, None, Some(&partial)));
            }
            failure
        })?;
        if id != op {
            continue;
        }
        match event {
            AppEvent::Failed(error) => return Err(lane.command_error(error).into()),
            AppEvent::Chat(event) if event.id() == op && event.generation() == 0 => match event {
                ChatEvent::Text { text, .. } => {
                    partial.push_str(&text);
                    let _ = write!(std::io::stderr(), "{text}");
                }
                ChatEvent::ToolStarted { name, .. } => {
                    let _ = writeln!(std::io::stderr(), "\ntool: {name}");
                }
                ChatEvent::Finished { turn, .. } | ChatEvent::AlreadyRunning { turn, .. } => {
                    if turn.status == WorkTurnStatus::Completed {
                        return Ok(Output {
                            text: format!("{}\n", turn.answer),
                            data: turn_json(&turn),
                        });
                    }
                    let error = if matches!(
                        turn.status,
                        WorkTurnStatus::Failed | WorkTurnStatus::Running
                    ) {
                        recorded_error(&turn)
                    } else if lane.stopped.is_some() {
                        lane.stop_error()
                    } else if turn.status == WorkTurnStatus::Interrupted {
                        CliError::Interrupted(
                            "recorded turn was interrupted; no automatic retry".into(),
                        )
                    } else {
                        recorded_error(&turn)
                    };
                    return Err(CliFailure {
                        error,
                        context: Some(context(op, session, Some(&turn), Some(&turn.answer))),
                    });
                }
                ChatEvent::Rejected { error, .. } => return Err(lane.command_error(error).into()),
                ChatEvent::PersistenceFailed { partial, error, .. } => {
                    return Err(CliFailure {
                        error: classify_workflow(error),
                        context: Some(context(op, session, None, Some(&partial))),
                    });
                }
            },
            _ => {}
        }
    }
}

fn recorded_error(turn: &WorkTurn) -> CliError {
    if turn.status == WorkTurnStatus::Running {
        return CliError::OperationConflict("operation is already running; do not resubmit".into());
    }
    classify_workflow(brn_workflow::WorkflowError::recorded_ai_failure(
        turn.error_code.as_deref(),
    ))
}

fn status_selection(result: Result<AppEvent, CliFailure>) -> Result<Value, CliFailure> {
    match result {
        Ok(AppEvent::Selection(selection)) => {
            Ok(json!({"selection": selection, "selection_error": null}))
        }
        Err(failure) if matches!(failure.error, CliError::Typed(ErrorKind::ModelRefused, _)) => {
            Ok(json!({"selection": null, "selection_error": {
                "code": failure.error.code(), "message": failure.error.message()
            }}))
        }
        Err(failure) => Err(failure),
        Ok(_) => Err(unexpected()),
    }
}

fn account(i: &Invocation, lane: &mut Lane, action: &AiCommand) -> Result<Output, CliFailure> {
    if let AiCommand::Select(selection) = action {
        let AppEvent::SelectionSaved = lane.query(AppCommand::Select(selection.clone()))? else {
            return Err(unexpected());
        };
        return Ok(output(json!({"selection": selection})));
    }
    if matches!(action, AiCommand::Status) {
        let mut statuses = Vec::new();
        for provider in [Provider::Chatgpt, Provider::Copilot] {
            statuses.push(account_operation(
                lane,
                AccountCommand::Status(provider),
                false,
            )?);
        }
        let mut data = status_selection(lane.query(AppCommand::Selection))?;
        data["accounts"] = json!(statuses);
        return Ok(output(data));
    }
    let (command, connect) = match action {
        AiCommand::Connect(provider, _) => (AccountCommand::Connect(*provider), true),
        AiCommand::Disconnect(provider) => (AccountCommand::Disconnect(*provider), false),
        AiCommand::Models(provider, _) => (AccountCommand::Models(*provider), false),
        _ => unreachable!(),
    };
    let provider = match action {
        AiCommand::Connect(p, _) | AiCommand::Disconnect(p) | AiCommand::Models(p, _) => *p,
        _ => unreachable!(),
    };
    let result = account_operation(lane, command, connect);
    if connect && result.is_err() {
        // Login may have persisted credentials before name lookup failed.
        // If cancellation joined the lane, reopen only for a local status query.
        let status = if lane.joined {
            match Lane::start_observing(i, 300, false) {
                Ok(mut local) => {
                    let result =
                        account_operation(&mut local, AccountCommand::Status(provider), false);
                    local.finish(result)
                }
                Err(error) => Err(error),
            }
        } else {
            lane.observe_cancel = false;
            lane.deadline = Instant::now() + Duration::from_secs(300);
            account_operation(lane, AccountCommand::Status(provider), false)
        };
        return result
            .map_err(|mut failure| {
                let context = failure.context.get_or_insert_with(|| json!({}));
                match status {
                    Ok(status) => context["account_status"] = status,
                    Err(error) => {
                        context["account_status_error"] =
                            json!({"code": error.error.code(), "message": error.error.message()})
                    }
                }
                failure
            })
            .map(output);
    }
    result.map(output)
}

fn account_operation<W: EventLane>(
    lane: &mut Lane<W>,
    command: AccountCommand,
    login: bool,
) -> Result<Value, CliFailure> {
    let op = Uuid::new_v4();
    lane.worker
        .submit(op, AppCommand::Account { id: op, command })
        .map_err(classify_workflow)?;
    wait_account(lane, op, login)
}

fn wait_account<W: EventLane>(
    lane: &mut Lane<W>,
    op: Uuid,
    login: bool,
) -> Result<Value, CliFailure> {
    let mut surface = LoginSurface(false);
    let result = (|| loop {
        let (id, event) = lane.next(Job::Account(op))?;
        if id != op {
            continue;
        }
        match event {
            AppEvent::Account(AccountEvent::Login { id, prompt })
                if login && id == op && lane.stopped.is_none() =>
            {
                surface.show(prompt);
            }
            AppEvent::Account(AccountEvent::Finished {
                id,
                provider,
                reply,
            }) if id == op => {
                break match reply {
                    AccountReply::Status(status) => Ok(json!(status)),
                    AccountReply::Disconnected => {
                        Ok(json!({"provider": provider, "connected": false, "name": null}))
                    }
                    AccountReply::Models(models) => {
                        Ok(json!({"provider": provider, "models": models}))
                    }
                    AccountReply::Cancelled => Err(if lane.stopped.is_some() {
                        lane.stop_error().into()
                    } else {
                        CliError::Interrupted("account operation cancelled locally".into()).into()
                    }),
                    AccountReply::Failed(error) => Err(classify_workflow(error.into()).into()),
                    AccountReply::Rejected(error) => Err(lane.command_error(error).into()),
                };
            }
            AppEvent::Failed(error) => break Err(lane.command_error(error).into()),
            _ => {}
        }
    })();
    result.map_err(|mut failure: CliFailure| {
        failure.context = Some(json!({"operation_id": op, "provider_outcome": "unknown"}));
        failure
    })
}

struct LoginSurface(bool);
impl LoginSurface {
    fn show(&mut self, prompt: brn_workflow::LoginPrompt) {
        self.clear();
        let _ = writeln!(
            std::io::stderr(),
            "Login: {}\nCode: {}",
            prompt.verification_uri,
            prompt.user_code
        );
        self.0 = true;
    }
    fn clear(&mut self) {
        if self.0 {
            // Only the dedicated terminal surface is cleared; redirected stderr
            // receives the explicit transient prompt, never an envelope/log DTO.
            if unsafe { libc::isatty(libc::STDERR_FILENO) } == 1 {
                let _ = write!(std::io::stderr(), "\x1b[2F\x1b[J");
            }
            let _ = writeln!(std::io::stderr(), "Login prompt closed.");
            self.0 = false;
        }
    }
}
impl Drop for LoginSurface {
    fn drop(&mut self) {
        self.clear();
    }
}

fn download(i: &Invocation, lane: &mut Lane) -> Result<Output, CliFailure> {
    let op = Uuid::new_v4();
    let target = i
        .model_dir
        .clone()
        .unwrap_or_else(|| i.data_dir.join("models/minilm"));
    lane.worker
        .submit(
            op,
            AppCommand::DownloadModel {
                consent: true,
                target: target.clone(),
            },
        )
        .map_err(classify_workflow)?;
    wait_download(lane, op, target)
}

fn wait_download<W: EventLane>(
    lane: &mut Lane<W>,
    op: Uuid,
    target: std::path::PathBuf,
) -> Result<Output, CliFailure> {
    let mut downloaded = None;
    loop {
        let (id, event) = lane.next(Job::Download(op)).map_err(|mut failure| {
            if failure.context.is_none() {
                failure.context =
                    Some(json!({"operation_id": op, "installed": false, "download": downloaded}));
            }
            failure
        })?;
        if id != op {
            continue;
        }
        match event {
            AppEvent::ModelDownload { received, total } => {
                let _ = writeln!(std::io::stderr(), "model: {received}/{total} bytes");
            }
            AppEvent::ModelDownloaded(report) => downloaded = Some(report),
            AppEvent::ModelInstalled => {
                return Ok(output(
                    json!({"operation_id": op, "installed": true, "directory": target, "download": downloaded}),
                ))
            }
            AppEvent::Failed(error) => {
                return Err(CliFailure {
                    error: if error.kind == ErrorKind::Cancelled && lane.stopped.is_some() {
                        lane.stop_error()
                    } else {
                        classify_workflow(error)
                    },
                    context: Some(
                        json!({"operation_id": op, "installed": false, "download": downloaded}),
                    ),
                })
            }
            AppEvent::ModelDownloadDeclined => return Err(unexpected()),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_workflow::WorkflowError;
    use std::{
        cell::{Cell, RefCell},
        collections::VecDeque,
    };

    #[test]
    fn status_selection_recovers_only_typed_model_refused() {
        let diagnostic =
            status_selection(Err(typed(ErrorKind::ModelRefused, "safe diagnostic"))).unwrap();
        assert!(diagnostic["selection"].is_null());
        assert_eq!(diagnostic["selection_error"]["code"], "AI_MODEL_REFUSED");
        assert_eq!(diagnostic["selection_error"]["message"], "safe diagnostic");
        for kind in [
            ErrorKind::AiStorage,
            ErrorKind::UnsafeCredentials,
            ErrorKind::Other,
            ErrorKind::ModelInvalid,
            ErrorKind::Cancelled,
        ] {
            let failure: CliFailure = classify_workflow(WorkflowError {
                kind,
                message: "selected model was not discovered for Copilot".into(),
            })
            .into();
            let expected = failure.error.clone();
            assert_eq!(status_selection(Err(failure)).unwrap_err().error, expected);
        }
        assert!(
            status_selection(Ok(AppEvent::Selection(None))).unwrap()["selection_error"].is_null()
        );
        assert!(status_selection(Ok(AppEvent::SelectionSaved)).is_err());
    }

    struct Projection {
        events: RefCell<VecDeque<(Uuid, AppEvent)>>,
        ending: Option<(Uuid, AppEvent)>,
        cancelled: Cell<bool>,
        joined: bool,
        failure: Option<WorkflowError>,
        query_replies: bool,
        recorded: Option<WorkTurn>,
        selection_queries: Cell<usize>,
        selected: Option<Selection>,
        ask_request: RefCell<Option<AskRequest>>,
    }
    impl EventLane for Projection {
        fn submit(&self, id: Uuid, command: AppCommand) -> brn_workflow::Result<()> {
            if self.query_replies {
                match &command {
                    AppCommand::Turn(_) => self
                        .events
                        .borrow_mut()
                        .push_back((id, AppEvent::Turn(self.recorded.clone()))),
                    AppCommand::Selection => {
                        self.selection_queries.set(self.selection_queries.get() + 1);
                        self.events
                            .borrow_mut()
                            .push_back((id, AppEvent::Selection(self.selected.clone())));
                    }
                    AppCommand::Ask(request) => {
                        self.ask_request.replace(Some(request.clone()));
                        if self.joined {
                            return Err(cancelled());
                        }
                        if let Some(turn) = &self.recorded {
                            self.events.borrow_mut().push_back((
                                id,
                                AppEvent::Chat(ChatEvent::Finished {
                                    id,
                                    generation: request.generation,
                                    turn: turn.clone(),
                                }),
                            ));
                        }
                    }
                    _ => {}
                }
            }
            if matches!(
                command,
                AppCommand::CancelTurn(_)
                    | AppCommand::CancelAccount(_)
                    | AppCommand::CancelModelDownload(_)
            ) {
                self.cancelled.set(true);
            }
            Ok(())
        }
        fn try_event(&self) -> Option<(Uuid, AppEvent)> {
            self.events.borrow_mut().pop_front()
        }
        fn recv_event_timeout(
            &self,
            _: Duration,
        ) -> Result<(Uuid, AppEvent), std::sync::mpsc::RecvTimeoutError> {
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        }
        fn shutdown(&mut self) -> brn_workflow::Result<()> {
            self.joined = true;
            if let Some(event) = self.ending.take() {
                self.events.borrow_mut().push_back(event);
            }
            self.failure.clone().map_or(Ok(()), Err)
        }
    }
    fn lane(op: Uuid, ending: AppEvent) -> Lane<Projection> {
        Lane {
            worker: Projection {
                events: RefCell::new(VecDeque::new()),
                ending: Some((op, ending)),
                cancelled: Cell::new(false),
                joined: false,
                failure: None,
                query_replies: false,
                recorded: None,
                selection_queries: Cell::new(0),
                selected: Some(Selection {
                    provider: Provider::Chatgpt,
                    model: "gpt-5.5".into(),
                }),
                ask_request: RefCell::new(None),
            },
            deadline: Instant::now() - Duration::from_secs(1),
            stopped: None,
            joined: false,
            shutdown_error: None,
            observe_cancel: false,
        }
    }
    fn cancelled() -> WorkflowError {
        WorkflowError {
            kind: ErrorKind::Cancelled,
            message: "operation cancelled".into(),
        }
    }
    fn turn(op: Uuid, status: WorkTurnStatus) -> WorkTurn {
        WorkTurn {
            id: op,
            conversation_id: Uuid::new_v4(),
            question: "q".into(),
            answer: "partial".into(),
            provider: "chatgpt".into(),
            model: "gpt-5.5".into(),
            status,
            error_code: None,
        }
    }

    #[test]
    fn actual_ask_submit_after_join_projects_deadline_and_sigint_and_skips_replay_selection() {
        for deadline in [true, false] {
            let op = Uuid::new_v4();
            let mut lane = lane(op, AppEvent::SelectionSaved);
            lane.worker.ending = None;
            lane.worker.query_replies = true;
            lane.worker.recorded = Some(turn(op, WorkTurnStatus::Completed));
            if !deadline {
                lane.stopped = Some(false);
                lane.joined = true;
                lane.worker.joined = true;
            }
            let failure = ask(&mut lane, "q", None, op).err().unwrap();
            assert_eq!(failure.error.exit_code(), if deadline { 124 } else { 130 });
            assert_eq!(lane.worker.selection_queries.get(), 0);
            assert!(lane.worker.ask_request.borrow().is_some());
            assert!(lane.worker.joined);
        }
    }
    #[test]
    fn actual_new_ask_requires_selection_but_frozen_completed_replay_never_queries_it() {
        let op = Uuid::new_v4();
        let mut lane = lane(op, AppEvent::SelectionSaved);
        lane.worker.ending = None;
        lane.worker.query_replies = true;
        lane.worker.selected = None;
        lane.deadline = Instant::now() + Duration::from_secs(300);
        let failure = ask(&mut lane, "q", None, op).err().unwrap();
        assert_eq!(failure.error.code(), "AI_SELECTION_REQUIRED");
        assert_eq!(lane.worker.selection_queries.get(), 1);
        assert!(lane.worker.ask_request.borrow().is_none());

        let recorded = turn(op, WorkTurnStatus::Completed);
        lane.worker.recorded = Some(recorded.clone());
        let result = ask(&mut lane, "q", None, op).unwrap_or_else(|_| panic!("frozen replay"));
        assert_eq!(result.data["status"], "completed");
        assert_eq!(lane.worker.selection_queries.get(), 1);
        let request = lane.worker.ask_request.borrow();
        let request = request.as_ref().unwrap();
        assert_eq!(request.selection.model, recorded.model);
        assert_eq!(request.selection.provider, Provider::Chatgpt);
    }

    #[test]
    fn deadline_before_ask_admission_keeps_intent_and_joins_rejected_request() {
        let op = Uuid::new_v4();
        let mut lane = lane(
            op,
            AppEvent::Chat(ChatEvent::Rejected {
                id: op,
                generation: 0,
                error: cancelled(),
            }),
        );
        let failure = wait_ask(&mut lane, op, None).err().unwrap();
        assert_eq!(failure.error.exit_code(), 124);
        assert!(lane.worker.cancelled.get() && lane.worker.joined);
    }

    #[test]
    fn deadline_before_account_admission_is_timeout_not_interrupted() {
        let op = Uuid::new_v4();
        let mut lane = lane(
            op,
            AppEvent::Account(AccountEvent::Finished {
                id: op,
                provider: Provider::Chatgpt,
                reply: AccountReply::Rejected(cancelled()),
            }),
        );
        let failure = wait_account(&mut lane, op, true).err().unwrap();
        assert_eq!(failure.error.exit_code(), 124);
        assert!(lane.worker.cancelled.get() && lane.worker.joined);
    }

    #[test]
    fn completed_after_cancel_is_success_but_persistence_failure_is_never_saved() {
        let op = Uuid::new_v4();
        let mut lane = lane(
            op,
            AppEvent::Chat(ChatEvent::Finished {
                id: op,
                generation: 0,
                turn: turn(op, WorkTurnStatus::Completed),
            }),
        );
        let result =
            wait_ask(&mut lane, op, None).unwrap_or_else(|_| panic!("completed receipt wins"));
        assert_eq!(result.data["status"], "completed");
        assert!(lane.worker.joined);
        let error = WorkflowError {
            kind: ErrorKind::AiStorage,
            message: "could not persist turn".into(),
        };
        let mut lane = self::lane(
            op,
            AppEvent::Chat(ChatEvent::PersistenceFailed {
                id: op,
                generation: 0,
                partial: "unsaved".into(),
                error: error.clone(),
            }),
        );
        lane.worker.failure = Some(error);
        let result = wait_ask(&mut lane, op, None);
        let failed = lane.finish(result).err().unwrap();
        assert_eq!(failed.error.code(), "AI_STORAGE_ERROR");
        let context = failed.context.unwrap();
        assert_eq!(context["saved"], false);
        assert!(context["recorded_status"].is_null());
        assert_eq!(context["partial"], "unsaved");
        assert_eq!(context["provider_outcome"], "unknown");
    }

    #[test]
    fn downloaded_is_not_installed_and_correlated_failure_terminates() {
        let op = Uuid::new_v4();
        let mut lane = lane(op, AppEvent::Failed(cancelled()));
        lane.worker.events.borrow_mut().push_back((
            op,
            AppEvent::ModelDownloaded(brn_workflow::models::ModelInstallReport {
                directory: "/synthetic/model".into(),
                downloaded_bytes: 7,
            }),
        ));
        let failure = wait_download(&mut lane, op, "/synthetic/model".into())
            .err()
            .unwrap();
        assert_eq!(failure.error.exit_code(), 124);
        assert_eq!(failure.context.unwrap()["installed"], false);
        assert!(lane.worker.cancelled.get() && lane.worker.joined);
    }

    #[test]
    fn signal_before_admission_stops_and_joins_exact_turn_and_keeps_partial_receipt() {
        let op = Uuid::new_v4();
        let mut lane = lane(
            op,
            AppEvent::Chat(ChatEvent::Finished {
                id: op,
                generation: 0,
                turn: turn(op, WorkTurnStatus::Interrupted),
            }),
        );
        lane.observe_cancel = true;
        lane.deadline = Instant::now() + Duration::from_secs(300);
        let event = lane
            .next_observing(Job::Ask(op), || true)
            .unwrap_or_else(|_| panic!("joined terminal"));
        lane.worker.events.borrow_mut().push_back(event);
        let failure = wait_ask(&mut lane, op, None).err().unwrap();
        assert_eq!(failure.error.exit_code(), 130);
        assert_eq!(failure.context.unwrap()["partial"], "partial");
        assert!(lane.worker.cancelled.get() && lane.worker.joined);
    }

    #[test]
    fn queued_progress_cannot_starve_expired_deadline_or_joined_cancellation() {
        let op = Uuid::new_v4();
        let mut lane = lane(
            op,
            AppEvent::Chat(ChatEvent::Finished {
                id: op,
                generation: 0,
                turn: turn(op, WorkTurnStatus::Interrupted),
            }),
        );
        lane.worker.events.borrow_mut().extend([
            (
                op,
                AppEvent::Chat(ChatEvent::Text {
                    id: op,
                    generation: 0,
                    text: String::new(),
                }),
            ),
            (
                op,
                AppEvent::Chat(ChatEvent::Finished {
                    id: op,
                    generation: 0,
                    turn: turn(op, WorkTurnStatus::Interrupted),
                }),
            ),
        ]);
        let failure = wait_ask(&mut lane, op, None).err().unwrap();
        assert_eq!(failure.error.exit_code(), 124);
        assert!(lane.worker.cancelled.get() && lane.worker.joined);
    }

    #[test]
    fn joined_stop_does_not_relabel_a_recorded_typed_storage_failure() {
        let op = Uuid::new_v4();
        let mut failed = turn(op, WorkTurnStatus::Failed);
        failed.error_code = Some("storage".into());
        let mut lane = lane(
            op,
            AppEvent::Chat(ChatEvent::Finished {
                id: op,
                generation: 0,
                turn: failed,
            }),
        );
        let failure = wait_ask(&mut lane, op, None).err().unwrap();
        assert_eq!(failure.error.code(), "AI_STORAGE_ERROR");
        assert_eq!(failure.error.exit_code(), 1);
        assert_eq!(failure.context.unwrap()["recorded_status"], "failed");
    }
}
