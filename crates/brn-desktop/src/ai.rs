//! Presentation and correlation only. Authority and network work belong to AppWorker.
#![cfg_attr(not(feature = "native-ui"), allow(dead_code))]
use brn_workflow::{
    AccountStatus, LoginPrompt, ModelOption, NoteEntry, Provider, Selection, WorkConversation,
    WorkTurn, WorkTurnStatus,
    app_worker::{AppCommand, AppEvent},
    chat_worker::{AccountCommand, AccountEvent, AccountReply, AskRequest, ChatEvent},
    editor::{
        EditRequest, EditStamp, EditorRecord, EditorView, ReloadRequest, SaveOutcome, SaveReceipt,
        SaveRequest,
    },
    library::{RefreshReport, SearchResults},
    models::ModelDownloadPrompt,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};
use uuid::Uuid;

pub struct ActiveTurn {
    pub request: AskRequest,
    pub partial: String,
    pub tool: Option<String>,
    pub stopping: bool,
}
pub struct LoginDialog {
    pub operation: Uuid,
    pub prompt: Option<LoginPrompt>,
}
#[derive(Default)]
pub struct AccountRow {
    pub status: Option<AccountStatus>,
    pub error: Option<String>,
    pub models: Vec<ModelOption>,
}
#[derive(Clone)]
pub enum Pending {
    Status,
    Selection,
    Select,
    Account(AccountCommand),
    Bind,
    Refresh,
    Notes { append: bool },
    Editor { generation: u64, preserve: bool },
    EditorRecovery,
    EditorSave,
    EditorReconcile,
    EditorReload,
    Editors,
    Search { generation: u64 },
    Conversations,
    Turns { generation: u64 },
    Prompt,
    Download,
}
#[derive(Default)]
pub struct AiState {
    pub ready: bool,
    pub startup_failed: bool,
    pub vault_bound: bool,
    pub vault_root: Option<PathBuf>,
    pub model_installed: bool,
    pub selection: Option<Selection>,
    pub selection_error: Option<String>,
    pub provider: Option<Provider>,
    pub generation: u64,
    pub conversation: Option<Uuid>,
    pub conversations: Vec<WorkConversation>,
    pub turns: Vec<WorkTurn>,
    pub active: Option<ActiveTurn>,
    pub unsaved: Option<WorkTurn>,
    pub login: Option<LoginDialog>,
    pub cancelled_login: Option<Uuid>,
    pub accounts: [AccountRow; 2],
    pub pending: HashMap<Uuid, Pending>,
    pub notes: Vec<NoteEntry>,
    pub next_cursor: Option<String>,
    pub note_error: Option<String>,
    pub note_generation: u64,
    pub editor: Option<SimpleEditor>,
    pub editors: Vec<EditorRecord>,
    pub search: Option<SearchResults>,
    pub search_generation: u64,
    pub refresh: Option<RefreshReport>,
    pub indexing: Option<(Uuid, usize, usize)>,
    pub model_prompt: Option<ModelDownloadPrompt>,
    pub download: Option<Uuid>,
    pub download_stopping: bool,
    pub download_progress: Option<(u64, u64)>,
    pub model_state: String,
    pub notice: String,
    pub restored: Option<PathBuf>,
}

struct EditorMutation {
    id: Uuid,
    edit: EditRequest,
    destination: Option<String>,
    reload_text: Option<String>,
}

/// Live typing is distinct from the worker's acknowledged rolling buffer.
pub struct SimpleEditor {
    pub view: EditorView,
    pub text: String,
    generation: u64,
    acknowledged: EditStamp,
    pending: Option<EditorMutation>,
    pub error: Option<String>,
    recovery_failed: bool,
    last_edit: Option<Instant>,
}
impl SimpleEditor {
    fn new(view: EditorView) -> Self {
        Self {
            text: view.record.text.clone(),
            generation: view.record.stamp.generation,
            acknowledged: view.record.stamp,
            view,
            pending: None,
            error: None,
            recovery_failed: false,
            last_edit: None,
        }
    }
    pub fn edit(&mut self, text: String, now: Instant) -> Result<(), &'static str> {
        if self.replacing() {
            return Err("Waiting for confirmed reload acknowledgement");
        }
        if text == self.text {
            return Ok(());
        }
        if text.len() > 1024 * 1024 {
            return Err("Note exceeds the 1 MiB UTF-8 byte limit");
        }
        self.generation = self
            .generation
            .checked_add(1)
            .filter(|generation| *generation <= i64::MAX as u64)
            .ok_or("Note generation exhausted")?;
        self.text = text;
        self.last_edit = Some(now);
        Ok(())
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn replacing(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| pending.reload_text.is_some())
    }
    pub fn needs_recovery(&self) -> bool {
        self.generation != self.acknowledged.generation || self.text != self.view.record.text
    }
    pub fn can_leave(&self) -> bool {
        !self.pending() && !self.needs_recovery()
    }
    pub fn dirty(&self) -> bool {
        self.view.saved.as_deref() != Some(&self.text)
    }
    pub fn can_save(&self) -> bool {
        !self.pending() && !self.view.conflict && self.view.pending.is_empty()
    }
    pub fn reload_request(&self) -> Option<ReloadRequest> {
        if !self.can_leave() || !self.view.pending.is_empty() {
            return None;
        }
        Some(ReloadRequest {
            path: self.view.record.path.clone(),
            expected: self.acknowledged,
            observed: self.view.observed.clone()?,
            discard: self.dirty(),
        })
    }
    pub fn wants_recovery(&self, now: Instant, leaving: bool) -> bool {
        self.needs_recovery()
            && !self.pending()
            && !self.recovery_failed
            && (leaving
                || self.last_edit.is_some_and(|last| {
                    now.saturating_duration_since(last) >= Duration::from_millis(500)
                }))
    }
    pub fn retry_recovery(&mut self) {
        self.recovery_failed = false;
        self.last_edit = Some(Instant::now() - Duration::from_millis(500));
    }
    fn begin(&mut self, id: Uuid, destination: Option<String>) -> Option<EditRequest> {
        if self.pending() {
            return None;
        }
        let edit = EditRequest {
            path: self.view.record.path.clone(),
            expected: self.acknowledged,
            generation: self.generation,
            text: self.text.clone(),
        };
        self.pending = Some(EditorMutation {
            id,
            edit: edit.clone(),
            destination,
            reload_text: None,
        });
        self.error = None;
        Some(edit)
    }
    fn recovered(&mut self, id: Uuid, record: EditorRecord) {
        let Some(pending) = self.pending.as_ref().filter(|pending| pending.id == id) else {
            return;
        };
        if record.path != pending.edit.path
            || record.stamp.baseline != pending.edit.expected.baseline
            || record.stamp.generation != pending.edit.generation
            || record.text != pending.edit.text
        {
            self.failed(
                id,
                "Recovery acknowledgement did not match submitted text".into(),
                true,
            );
            return;
        }
        self.acknowledged = record.stamp;
        self.view.record = record;
        self.pending = None;
        self.recovery_failed = false;
    }
    fn reloaded(&mut self, id: Uuid, record: EditorRecord) {
        let Some(pending) = self.pending.as_ref().filter(|pending| pending.id == id) else {
            return;
        };
        if record.path != pending.edit.path
            || record.stamp.generation < pending.edit.generation
            || pending.reload_text.as_deref() != Some(record.text.as_str())
        {
            self.failed(
                id,
                "Reload acknowledgement did not match reviewed disk text".into(),
                false,
            );
            return;
        }
        self.text = record.text.clone();
        self.generation = record.stamp.generation;
        self.acknowledged = record.stamp;
        self.view.record = record;
        self.view.saved = Some(self.text.clone());
        self.view.conflict = false;
        self.pending = None;
        self.error = None;
        self.recovery_failed = false;
    }
    fn saved(&mut self, id: Uuid, receipt: &SaveReceipt) {
        let Some(pending) = self.pending.as_ref().filter(|pending| pending.id == id) else {
            return;
        };
        if receipt.operation_id != id
            || receipt.path != pending.edit.path
            || receipt.destination != pending.destination
            || receipt.submitted_generation != pending.edit.generation
            || receipt.stamp.generation != pending.edit.generation
            || pending.destination.is_some()
                && receipt.stamp.baseline != pending.edit.expected.baseline
        {
            self.failed(
                id,
                "Save acknowledgement did not match submitted text".into(),
                true,
            );
            return;
        }
        // The receipt only acknowledges its submitted snapshot, never subsequent typing.
        if pending.destination.is_none() && receipt.outcome == SaveOutcome::Applied {
            self.view.saved = Some(pending.edit.text.clone());
        }
        if receipt.outcome == SaveOutcome::Uncertain {
            if !self.view.pending.contains(&id) {
                self.view.pending.push(id);
            }
            self.error = Some(
                "Save outcome uncertain; reconcile before saving again. Recovery text is retained."
                    .into(),
            );
        }
        self.acknowledged = receipt.stamp;
        self.view.record.stamp = receipt.stamp;
        self.view.record.text = pending.edit.text.clone();
        self.pending = None;
    }
    fn observe(&mut self, view: EditorView) {
        if view.record.path != self.view.record.path {
            return;
        }
        // A queued observation can precede a later acknowledgement.
        if view.record.stamp.baseline != self.acknowledged.baseline
            || view.record.stamp.generation < self.acknowledged.generation
        {
            return;
        }
        // Fresh disk inspection is not an acknowledgement of another submitted
        // snapshot. Only identical live bytes prove the rolling buffer is current.
        if !self.pending()
            && view.record.text == self.text
            && view.record.stamp.generation >= self.generation
        {
            self.acknowledged = view.record.stamp;
            self.generation = view.record.stamp.generation;
        }
        self.view = view;
    }
    fn reconciled(&mut self, receipt: &SaveReceipt) {
        if receipt.path != self.view.record.path
            || !self.view.pending.contains(&receipt.operation_id)
        {
            return;
        }
        // Reconciliation can resolve an older original while a newer rolling
        // buffer is already acknowledged. Adopt its proven baseline, never lower
        // the buffer generation or replace the user's live text.
        if receipt.destination.is_none() && receipt.outcome == SaveOutcome::Applied {
            self.acknowledged.baseline = receipt.stamp.baseline;
            self.view.record.stamp.baseline = receipt.stamp.baseline;
        }
    }
    fn failed(&mut self, id: Uuid, message: String, recovery: bool) {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.id == id)
        {
            self.pending = None;
            self.error = Some(message);
            self.recovery_failed |= recovery;
        }
    }
    pub fn status(&self) -> &'static str {
        if self.pending() {
            "Waiting for local acknowledgement"
        } else if !self.view.pending.is_empty() {
            "Save outcome uncertain"
        } else if self.view.conflict {
            "Disk conflict or vault unavailable · buffer retained"
        } else if self.recovery_failed {
            "Recovery failed · text retained in memory"
        } else if self.needs_recovery() {
            "Unacknowledged edits"
        } else if self.dirty() {
            "Recovered in BRN · Markdown unsaved"
        } else {
            "Saved Markdown"
        }
    }
}
pub fn slot(provider: Provider) -> usize {
    match provider {
        Provider::Chatgpt => 0,
        Provider::Copilot => 1,
    }
}
pub fn provider_name(provider: Provider) -> &'static str {
    match provider {
        Provider::Chatgpt => "ChatGPT",
        Provider::Copilot => "Copilot",
    }
}
pub fn turn_label(turn: &WorkTurn) -> &'static str {
    match turn.status {
        WorkTurnStatus::Running => "Running (restart recovery pending)",
        WorkTurnStatus::Completed => "Completed",
        WorkTurnStatus::Interrupted => "Stopped",
        WorkTurnStatus::Failed => "Failed",
    }
}
fn unfinalized_turn(request: &AskRequest, partial: String) -> WorkTurn {
    WorkTurn {
        id: request.id,
        conversation_id: request.conversation.unwrap_or(Uuid::nil()),
        question: request.question.clone(),
        answer: partial,
        provider: match request.selection.provider {
            Provider::Chatgpt => "chatgpt",
            Provider::Copilot => "copilot",
        }
        .into(),
        model: request.selection.model.clone(),
        status: WorkTurnStatus::Failed,
        error_code: None,
    }
}
impl AiState {
    pub fn display_active(&self) -> Option<&ActiveTurn> {
        self.active
            .as_ref()
            .filter(|active| match active.request.conversation {
                Some(conversation) => self.conversation == Some(conversation),
                None => self.conversation.is_none() && active.request.generation == self.generation,
            })
    }
    pub fn display_turns(&self) -> impl Iterator<Item = &WorkTurn> {
        let active = self.display_active().map(|active| active.request.id);
        self.turns
            .iter()
            .filter(move |turn| turn.status != WorkTurnStatus::Running || active != Some(turn.id))
    }
    fn upsert_turn(&mut self, turn: WorkTurn) {
        if let Some(row) = self.turns.iter_mut().find(|row| row.id == turn.id) {
            *row = turn;
        } else {
            self.turns.push(turn);
        }
    }
    pub fn composer_changed(&mut self) {
        self.search_generation = self.search_generation.wrapping_add(1);
        self.search = None;
    }
    pub fn command(&mut self, pending: Pending, command: AppCommand) -> (Uuid, AppCommand) {
        let id = Uuid::new_v4();
        self.pending.insert(id, pending);
        (id, command)
    }
    pub fn open_editor(&mut self, path: String) -> (Uuid, AppCommand) {
        self.note_generation = self.note_generation.wrapping_add(1);
        self.editor = None;
        self.note_error = None;
        self.command(
            Pending::Editor {
                generation: self.note_generation,
                preserve: false,
            },
            AppCommand::OpenEditor(path),
        )
    }
    pub fn refresh_editor(&mut self) -> Option<(Uuid, AppCommand)> {
        let path = self.editor.as_ref()?.view.record.path.clone();
        Some(self.command(
            Pending::Editor {
                generation: self.note_generation,
                preserve: true,
            },
            AppCommand::OpenEditor(path),
        ))
    }
    pub fn recover_editor(&mut self) -> Option<(Uuid, AppCommand)> {
        let id = Uuid::new_v4();
        let edit = self.editor.as_mut()?.begin(id, None)?;
        self.pending.insert(id, Pending::EditorRecovery);
        Some((id, AppCommand::RecoverEditor(edit)))
    }
    pub fn save_editor(&mut self, destination: Option<String>) -> Option<(Uuid, AppCommand)> {
        if !self.ready {
            return None;
        }
        let editor = self.editor.as_mut()?;
        if destination.is_none() && !editor.can_save() {
            return None;
        }
        let id = Uuid::new_v4();
        let edit = editor.begin(id, destination.clone())?;
        self.pending.insert(id, Pending::EditorSave);
        Some((
            id,
            AppCommand::SaveEditor(SaveRequest {
                operation_id: id,
                edit,
                destination,
            }),
        ))
    }
    pub fn reload_editor(&mut self, request: ReloadRequest) -> Option<(Uuid, AppCommand)> {
        let editor = self.editor.as_mut()?;
        if !editor.can_leave()
            || editor.view.record.path != request.path
            || editor.acknowledged != request.expected
            || editor.view.observed.as_ref() != Some(&request.observed)
        {
            return None;
        }
        let reviewed = editor.view.saved.clone()?;
        let id = Uuid::new_v4();
        editor.begin(id, None)?;
        editor.pending.as_mut()?.reload_text = Some(reviewed);
        self.pending.insert(id, Pending::EditorReload);
        Some((id, AppCommand::ReloadEditor(request)))
    }
    pub fn can_ask(&self) -> bool {
        self.ready
            && self.vault_bound
            && self.selection.is_some()
            && self.selection_error.is_none()
            && self.active.is_none()
            && self.unsaved.is_none()
            && !self.pending.values().any(|p| matches!(p, Pending::Select))
    }
    pub fn ask(&mut self, question: String) -> Option<AskRequest> {
        if !self.can_ask() || question.trim().is_empty() {
            return None;
        }
        let request = AskRequest {
            id: Uuid::new_v4(),
            conversation: self.conversation,
            question,
            selection: self.selection.clone()?,
            generation: self.generation,
        };
        self.active = Some(ActiveTurn {
            request: request.clone(),
            partial: String::new(),
            tool: None,
            stopping: false,
        });
        self.notice = "Answer requested; provisional until local finalization.".into();
        Some(request)
    }
    pub fn stop(&mut self) -> Option<Uuid> {
        let active = self.active.as_mut()?;
        active.stopping = true;
        Some(active.request.id)
    }
    pub fn stop_controls(&self) -> Vec<(Uuid, AppCommand)> {
        let mut commands = Vec::new();
        if let Some(active) = &self.active
            && active.stopping
        {
            commands.push((Uuid::new_v4(), AppCommand::CancelTurn(active.request.id)));
        }
        if let Some(id) = self.cancelled_login {
            commands.push((Uuid::new_v4(), AppCommand::CancelAccount(id)));
        }
        if self.download_stopping
            && let Some(id) = self.download
        {
            commands.push((Uuid::new_v4(), AppCommand::CancelModelDownload(id)));
        }
        commands
    }
    pub fn account_busy(&self, provider: Provider) -> bool {
        self.pending.values().any(
            |p| matches!(p, Pending::Account(command) if account_provider(command) == provider),
        )
    }
    pub fn account(&mut self, command: AccountCommand) -> Option<(Uuid, AppCommand)> {
        let provider = account_provider(&command);
        if !self.ready || self.account_busy(provider) {
            return None;
        }
        let id = Uuid::new_v4();
        if matches!(command, AccountCommand::Connect(_)) {
            if self.login.is_some() {
                return None;
            }
            self.login = Some(LoginDialog {
                operation: id,
                prompt: None,
            });
            self.accounts[slot(provider)].error = None;
        }
        self.pending.insert(id, Pending::Account(command.clone()));
        Some((id, AppCommand::Account { id, command }))
    }
    pub fn dismiss_login(&mut self) -> Option<Uuid> {
        let id = self.login.take()?.operation;
        self.cancelled_login = Some(id);
        Some(id)
    }
    pub fn navigate(&mut self, conversation: Option<Uuid>) -> Option<(Uuid, AppCommand)> {
        if !self.ready {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.conversation = conversation;
        self.turns.clear();
        conversation.map(|id| {
            self.command(
                Pending::Turns {
                    generation: self.generation,
                },
                AppCommand::Turns(id),
            )
        })
    }
    pub fn apply(&mut self, id: Uuid, event: AppEvent) -> Vec<(Uuid, AppCommand)> {
        let mut commands = Vec::new();
        if let AppEvent::Chat(event) = event {
            if let Some(active) = &self.active
                && event.id() == active.request.id
                && id == event.id()
                && event.generation() == active.request.generation
            {
                let display = self.display_active().is_some();
                match event {
                    ChatEvent::Text { text, .. } => {
                        self.active.as_mut().unwrap().partial.push_str(&text);
                        return self.stop_controls();
                    }
                    ChatEvent::ToolStarted { name, .. } => {
                        self.active.as_mut().unwrap().tool = Some(name);
                        return self.stop_controls();
                    }
                    ChatEvent::Finished { turn, .. } => {
                        if display {
                            self.conversation = Some(turn.conversation_id);
                            self.upsert_turn(turn);
                        }
                        self.notice = "Turn finalized locally. Stop does not prove upstream cancellation or no billing.".into();
                        commands
                            .push(self.command(Pending::Conversations, AppCommand::Conversations));
                    }
                    ChatEvent::AlreadyRunning { turn, .. } => {
                        self.notice =
                            "This turn is already recorded Running; it was not resubmitted.".into();
                        if display {
                            self.upsert_turn(turn);
                        }
                    }
                    ChatEvent::Rejected { error, .. } => {
                        self.notice = error.message;
                        if !active.partial.is_empty() {
                            self.unsaved =
                                Some(unfinalized_turn(&active.request, active.partial.clone()));
                            self.notice.push_str(
                                " Partial retained; local finalization not acknowledged.",
                            );
                        }
                    }
                    ChatEvent::PersistenceFailed { partial, error, .. } => {
                        self.unsaved = Some(unfinalized_turn(&active.request, partial));
                        self.notice = format!(
                            "Local finalization failed; partial answer not saved. {}",
                            error.message
                        );
                    }
                }
                self.active = None;
            }
            return commands;
        }
        if let AppEvent::Account(event) = event {
            match event {
                AccountEvent::Login {
                    id: operation,
                    prompt,
                } => {
                    if operation == id
                        && let Some(login) = &mut self.login
                        && login.operation == id
                    {
                        login.prompt = Some(prompt);
                    }
                }
                AccountEvent::Finished {
                    id: operation,
                    provider,
                    reply,
                } => {
                    if id != operation {
                        return commands;
                    }
                    let Some(Pending::Account(command)) = self.pending.remove(&id) else {
                        return commands;
                    };
                    if account_provider(&command) != provider {
                        return commands;
                    }
                    if self.login.as_ref().is_some_and(|l| l.operation == id) {
                        self.login = None;
                    }
                    if self.cancelled_login == Some(id) {
                        self.cancelled_login = None;
                    }
                    let row = &mut self.accounts[slot(provider)];
                    match reply {
                        AccountReply::Status(status) => row.status = Some(status),
                        AccountReply::Disconnected => {
                            row.status = Some(AccountStatus {
                                provider,
                                connected: false,
                                name: None,
                            });
                            row.error = None;
                        }
                        AccountReply::Models(models) => row.models = models,
                        AccountReply::Cancelled => {
                            row.error =
                                Some("Connection cancelled. Retry only by explicit Connect.".into())
                        }
                        AccountReply::Failed(error) => row.error = Some(error.to_string()),
                        AccountReply::Rejected(error) => row.error = Some(error.message),
                    }
                    // Cache persistence can precede a failed/cancelled display-name lookup.
                    if matches!(command, AccountCommand::Connect(_))
                        && let Some(command) = self.account(AccountCommand::Status(provider))
                    {
                        commands.push(command);
                    }
                }
            }
            return commands;
        }
        let pending = self.pending.get(&id).cloned();
        match event {
            AppEvent::Ready {
                vault_bound,
                model_installed,
            } => {
                self.ready = true;
                self.vault_bound = vault_bound;
                self.model_installed = model_installed;
                self.model_state = if model_installed {
                    "Installed"
                } else {
                    "Keyword-only; no local model installed"
                }
                .into();
                for (pending, command) in [
                    (Pending::Status, AppCommand::Status),
                    (Pending::Selection, AppCommand::Selection),
                    (Pending::Conversations, AppCommand::Conversations),
                    (Pending::Prompt, AppCommand::ModelPrompt),
                    (Pending::Editors, AppCommand::Editors),
                ] {
                    commands.push(self.command(pending, command));
                }
                for provider in [Provider::Chatgpt, Provider::Copilot] {
                    if let Some(c) = self.account(AccountCommand::Status(provider)) {
                        commands.push(c);
                    }
                }
                if vault_bound {
                    commands.push(self.command(Pending::Refresh, AppCommand::Refresh));
                }
                self.notice = if vault_bound {
                    "Workspace ready."
                } else {
                    "Choose a vault to read notes. Accounts and history remain available."
                }
                .into();
            }
            AppEvent::Restored { backup } => self.restored = Some(backup),
            AppEvent::Status(status) => {
                self.vault_root = status.vault_root;
                self.model_installed = status.model_installed;
            }
            AppEvent::Selection(selection) => {
                self.provider = selection.as_ref().map(|s| s.provider);
                self.selection = selection;
                self.selection_error = None;
            }
            AppEvent::SelectionSaved => {
                commands.push(self.command(Pending::Selection, AppCommand::Selection))
            }
            AppEvent::VaultBound => {
                self.vault_bound = true;
                commands.push(self.command(Pending::Status, AppCommand::Status));
                commands.push(self.command(Pending::Refresh, AppCommand::Refresh));
            }
            AppEvent::Refreshed(report) => {
                self.refresh = Some(report);
                commands.push(self.command(
                    Pending::Notes { append: false },
                    AppCommand::Notes {
                        folder: None,
                        cursor: None,
                    },
                ));
            }
            AppEvent::Notes(page) => {
                if matches!(pending, Some(Pending::Notes { append: true })) {
                    self.notes.extend(page.notes);
                } else {
                    self.notes = page.notes;
                }
                self.next_cursor = page.next_cursor;
            }
            AppEvent::Note(_) | AppEvent::Proposal(_) | AppEvent::Proposals(_) => {}
            AppEvent::Editor(view) => {
                if let Some(Pending::Editor {
                    generation,
                    preserve,
                }) = pending
                    && generation == self.note_generation
                {
                    if preserve {
                        if let Some(editor) = &mut self.editor {
                            editor.observe(view);
                        }
                    } else {
                        self.editor = Some(SimpleEditor::new(view));
                    }
                }
            }
            AppEvent::EditorRecovered(record) => {
                if matches!(
                    pending,
                    Some(Pending::EditorRecovery | Pending::EditorReload)
                ) {
                    if let Some(editor) = &mut self.editor {
                        if matches!(pending, Some(Pending::EditorReload)) {
                            editor.reloaded(id, record);
                        } else {
                            editor.recovered(id, record);
                        }
                    }
                    if matches!(pending, Some(Pending::EditorReload))
                        && let Some(command) = self.refresh_editor()
                    {
                        commands.push(command);
                    }
                    commands.push(self.command(Pending::Editors, AppCommand::Editors));
                }
            }
            AppEvent::EditorSaved(receipt) => {
                if matches!(
                    pending,
                    Some(Pending::EditorSave | Pending::EditorReconcile)
                ) {
                    if let Some(editor) = &mut self.editor {
                        if matches!(pending, Some(Pending::EditorReconcile)) {
                            editor.reconciled(&receipt);
                        } else {
                            editor.saved(id, &receipt);
                        }
                    }
                    if let Some(command) = self.refresh_editor() {
                        commands.push(command);
                    }
                    commands.push(self.command(Pending::Editors, AppCommand::Editors));
                }
            }
            AppEvent::Editors(editors) => self.editors = editors,
            AppEvent::Search(results) => {
                if matches!(pending, Some(Pending::Search { generation }) if generation == self.search_generation)
                {
                    self.search = Some(results);
                }
            }
            AppEvent::Conversations(conversations) => self.conversations = conversations,
            AppEvent::Turns(turns) => {
                if matches!(pending, Some(Pending::Turns { generation }) if generation == self.generation)
                {
                    // A queued snapshot may predate Finished. Keep known durable
                    // endings only for this display; navigation drops this overlay.
                    let terminal: Vec<_> = self
                        .turns
                        .iter()
                        .filter(|turn| turn.status != WorkTurnStatus::Running)
                        .cloned()
                        .collect();
                    self.turns = turns;
                    for turn in terminal {
                        self.upsert_turn(turn);
                    }
                }
            }
            AppEvent::Turn(_) | AppEvent::EditRecovered => {}
            AppEvent::Indexing { embedded, total } => {
                self.indexing = Some((id, embedded, total));
                return commands;
            }
            AppEvent::ModelPrompt(prompt) => self.model_prompt = prompt,
            AppEvent::ModelDownload { received, total } => {
                if self.download == Some(id) {
                    self.download_progress = Some((received, total));
                }
                return commands;
            }
            AppEvent::ModelDownloaded(_) => {
                if self.download == Some(id) {
                    self.model_state = "Downloaded; awaiting activation (not installed)".into();
                }
                return commands;
            }
            AppEvent::ModelInstalled => {
                if self.download == Some(id) {
                    self.download = None;
                    self.download_stopping = false;
                    self.download_progress = None;
                    self.model_installed = true;
                    self.model_state = "Installed; indexing separately".into();
                    self.model_prompt = None;
                }
            }
            AppEvent::ModelDownloadDeclined => {
                if self.download == Some(id) {
                    self.download = None;
                    self.download_stopping = false;
                    self.download_progress = None;
                    self.model_prompt = None;
                    self.model_state = if self.model_installed {
                        "Download declined; the previously installed model is retained."
                    } else {
                        "Download declined; keyword-only. Later Download requires fresh approval."
                    }
                    .into();
                }
            }
            AppEvent::Failed(error) => {
                let mut retained_partial = false;
                if matches!(pending, Some(Pending::Editor { generation, .. }) if generation == self.note_generation)
                {
                    self.note_error = Some(error.message.clone());
                }
                if matches!(
                    pending,
                    Some(
                        Pending::EditorRecovery
                            | Pending::EditorSave
                            | Pending::EditorReconcile
                            | Pending::EditorReload
                    )
                ) {
                    if let Some(editor) = &mut self.editor {
                        editor.failed(
                            id,
                            error.message.clone(),
                            matches!(pending, Some(Pending::EditorRecovery)),
                        );
                    }
                    if !matches!(pending, Some(Pending::EditorRecovery))
                        && let Some(command) = self.refresh_editor()
                    {
                        commands.push(command);
                    }
                }
                if self
                    .active
                    .as_ref()
                    .is_some_and(|active| active.request.id == id)
                {
                    let active = self.active.as_ref().unwrap();
                    if !active.partial.is_empty() {
                        self.unsaved =
                            Some(unfinalized_turn(&active.request, active.partial.clone()));
                        retained_partial = true;
                    }
                    self.active = None;
                }
                if let Some(Pending::Account(command)) = &pending {
                    let provider = account_provider(command);
                    self.accounts[slot(provider)].error = Some(error.message.clone());
                    if self
                        .login
                        .as_ref()
                        .is_some_and(|login| login.operation == id)
                    {
                        self.login = None;
                    }
                    if self.cancelled_login == Some(id) {
                        self.cancelled_login = None;
                    }
                    if matches!(command, AccountCommand::Connect(_)) {
                        self.pending.remove(&id);
                        if let Some(command) = self.account(AccountCommand::Status(provider)) {
                            commands.push(command);
                        }
                    }
                }
                if matches!(pending, Some(Pending::Selection | Pending::Select)) {
                    self.selection_error = Some(error.message.clone());
                }
                if self.download == Some(id) {
                    self.download = None;
                    self.download_stopping = false;
                    self.download_progress = None;
                    self.model_state = format!("Installation failed: {}", error.message);
                    self.model_prompt = None;
                }
                if self.indexing.is_some_and(|(job, _, _)| job == id) {
                    self.indexing = None;
                }
                if !self.ready {
                    self.startup_failed = true;
                }
                self.notice = error.message;
                if retained_partial {
                    self.notice
                        .push_str(" Partial retained; local finalization not acknowledged.");
                }
            }
            AppEvent::TurnCancelRequested { .. }
            | AppEvent::AccountCancelRequested { .. }
            | AppEvent::ModelCancelRequested { .. } => return commands,
            AppEvent::Chat(_) | AppEvent::Account(_) => unreachable!(),
        }
        self.pending.remove(&id);
        commands
    }
}
fn account_provider(command: &AccountCommand) -> Provider {
    match command {
        AccountCommand::Connect(p)
        | AccountCommand::Disconnect(p)
        | AccountCommand::Status(p)
        | AccountCommand::Models(p) => *p,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_workflow::{AiError, AiErrorKind, WorkTurnStatus};

    fn selection() -> Selection {
        Selection {
            provider: Provider::Copilot,
            model: "explicit-model".into(),
        }
    }
    fn ready() -> AiState {
        let mut state = AiState::default();
        state.apply(
            Uuid::new_v4(),
            AppEvent::Ready {
                vault_bound: true,
                model_installed: false,
            },
        );
        state.pending.clear();
        state.selection = Some(selection());
        state
    }
    fn editor_view(path: &str, text: &str) -> EditorView {
        let mut view = EditorView {
            record: EditorRecord {
                path: path.into(),
                stamp: EditStamp {
                    baseline: Uuid::new_v4(),
                    generation: 0,
                },
                baseline: serde_json::from_value(serde_json::json!({
                    "device": 1, "inode": 2, "len": text.len(), "sha256": vec![0; 32]
                }))
                .unwrap(),
                baseline_text: text.into(),
                text: text.into(),
            },
            saved: Some(text.into()),
            conflict: false,
            pending: Vec::new(),
            observed: None,
        };
        view.observed = Some(view.record.baseline.clone());
        view
    }
    fn editing(text: &str) -> AiState {
        let mut state = ready();
        let (id, _) = state.open_editor("plan.md".into());
        state.apply(id, AppEvent::Editor(editor_view("plan.md", text)));
        state
    }
    #[test]
    fn recovery_snapshot_preserves_later_typing_and_close_waits_for_latest_ack() {
        let exact = "\u{feff}---\r\ntitle: e\u{301}\n---\r\n正文 🧭\r";
        let mut state = editing(exact);
        let now = Instant::now();
        let editor = state.editor.as_mut().unwrap();
        editor.edit(format!("{exact}first"), now).unwrap();
        assert!(!editor.wants_recovery(now, false));
        assert!(editor.wants_recovery(now, true));
        let (id, AppCommand::RecoverEditor(request)) = state.recover_editor().unwrap() else {
            panic!("recovery");
        };
        state
            .editor
            .as_mut()
            .unwrap()
            .edit(format!("{exact}latest"), now)
            .unwrap();
        let mut record = state.editor.as_ref().unwrap().view.record.clone();
        record.text = request.text.clone();
        record.stamp.generation = request.generation;
        state.apply(Uuid::new_v4(), AppEvent::EditorRecovered(record.clone()));
        assert!(state.editor.as_ref().unwrap().pending());
        state.apply(id, AppEvent::EditorRecovered(record));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.text, format!("{exact}latest"));
        assert!(editor.dirty());
        assert!(!editor.can_leave());
        let (id, AppCommand::RecoverEditor(latest)) = state.recover_editor().unwrap() else {
            panic!("recovery");
        };
        assert_eq!(latest.generation, 2);
        assert_eq!(latest.expected.generation, 1);
        let mut record = state.editor.as_ref().unwrap().view.record.clone();
        record.text = latest.text;
        record.stamp.generation = latest.generation;
        state.apply(id, AppEvent::EditorRecovered(record));
        assert!(state.editor.as_ref().unwrap().can_leave());
    }
    #[test]
    fn save_acknowledges_only_submitted_generation_then_recovers_newer_typing() {
        let mut state = editing("base");
        let now = Instant::now();
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("submitted".into(), now)
            .unwrap();
        let (id, AppCommand::SaveEditor(request)) = state.save_editor(None).unwrap() else {
            panic!("save");
        };
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("newer typing".into(), now)
            .unwrap();
        let new_baseline = Uuid::new_v4();
        state.apply(
            id,
            AppEvent::EditorSaved(SaveReceipt {
                operation_id: id,
                path: request.edit.path,
                destination: None,
                submitted_generation: request.edit.generation,
                stamp: EditStamp {
                    baseline: new_baseline,
                    generation: 1,
                },
                outcome: SaveOutcome::Applied,
            }),
        );
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.text, "newer typing");
        assert_eq!(editor.view.saved.as_deref(), Some("submitted"));
        assert!(!editor.can_leave());
        let (_, AppCommand::RecoverEditor(next)) = state.recover_editor().unwrap() else {
            panic!("recovery");
        };
        assert_eq!(next.expected.baseline, new_baseline);
        assert_eq!(next.expected.generation, 1);
        assert_eq!(next.generation, 2);
    }
    #[test]
    fn stale_same_generation_observation_cannot_undo_applied_save_baseline() {
        let mut state = editing("base");
        let now = Instant::now();
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("submitted".into(), now)
            .unwrap();
        let (save, AppCommand::SaveEditor(request)) = state.save_editor(None).unwrap() else {
            panic!("save");
        };
        let mut stale = state.editor.as_ref().unwrap().view.clone();
        stale.record.stamp.generation = request.edit.generation;
        stale.record.text = request.edit.text.clone();
        let observation = state.refresh_editor().unwrap().0;
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("newer".into(), now)
            .unwrap();
        let baseline = Uuid::new_v4();
        state.apply(
            save,
            AppEvent::EditorSaved(SaveReceipt {
                operation_id: save,
                path: request.edit.path,
                destination: None,
                submitted_generation: request.edit.generation,
                stamp: EditStamp {
                    baseline,
                    generation: request.edit.generation,
                },
                outcome: SaveOutcome::Applied,
            }),
        );
        state.apply(observation, AppEvent::Editor(stale));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.acknowledged.baseline, baseline);
        assert_eq!(editor.text, "newer");
        assert_eq!(editor.view.saved.as_deref(), Some("submitted"));
        let (_, AppCommand::RecoverEditor(next)) = state.recover_editor().unwrap() else {
            panic!("recovery");
        };
        assert_eq!(next.expected.baseline, baseline);
        assert_eq!(next.generation, 2);
    }
    #[test]
    fn observation_during_recovery_does_not_acknowledge_a_different_snapshot() {
        let mut state = editing("base");
        let now = Instant::now();
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("submitted".into(), now)
            .unwrap();
        state.recover_editor().unwrap();
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("latest".into(), now)
            .unwrap();
        let observation = state.refresh_editor().unwrap().0;
        let mut view = state.editor.as_ref().unwrap().view.clone();
        view.record.stamp.generation = 2;
        view.record.text = "different snapshot".into();
        state.apply(observation, AppEvent::Editor(view));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.acknowledged.generation, 0);
        assert_eq!(editor.generation, 2);
        assert_eq!(editor.text, "latest");
        assert!(!editor.can_leave());
    }
    #[test]
    fn reconciled_original_adopts_proven_baseline_without_lowering_recovered_generation() {
        let mut state = editing("base");
        let operation = Uuid::new_v4();
        let editor = state.editor.as_mut().unwrap();
        editor.text = "newer recovered buffer".into();
        editor.generation = 2;
        editor.acknowledged.generation = 2;
        editor.view.record.text = editor.text.clone();
        editor.view.record.stamp.generation = 2;
        editor.view.pending.push(operation);
        let (id, _) = state.command(
            Pending::EditorReconcile,
            AppCommand::ReconcileEditor(operation),
        );
        let baseline = Uuid::new_v4();
        let followups = state.apply(
            id,
            AppEvent::EditorSaved(SaveReceipt {
                operation_id: operation,
                path: "plan.md".into(),
                destination: None,
                submitted_generation: 1,
                stamp: EditStamp {
                    baseline,
                    generation: 1,
                },
                outcome: SaveOutcome::Applied,
            }),
        );
        let observation = followups
            .iter()
            .find_map(|(id, command)| matches!(command, AppCommand::OpenEditor(_)).then_some(*id))
            .unwrap();
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.acknowledged.generation, 2);
        assert_eq!(editor.acknowledged.baseline, baseline);
        let mut view = editor.view.clone();
        view.pending.clear();
        view.saved = Some("older saved snapshot".into());
        state.apply(observation, AppEvent::Editor(view));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.text, "newer recovered buffer");
        assert_eq!(editor.generation, 2);
        assert!(editor.can_leave());
        assert!(editor.dirty());
    }
    #[test]
    fn recovery_failure_blocks_leaving_and_does_not_loop_without_explicit_retry() {
        let mut state = editing("base");
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("latest".into(), Instant::now())
            .unwrap();
        let (id, _) = state.recover_editor().unwrap();
        state.apply(
            id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("disk full")),
        );
        let editor = state.editor.as_mut().unwrap();
        assert_eq!(editor.text, "latest");
        assert!(!editor.can_leave());
        assert!(!editor.wants_recovery(Instant::now() + Duration::from_secs(1), true));
        editor.retry_recovery();
        assert!(editor.wants_recovery(Instant::now(), true));
    }
    #[test]
    fn copy_keeps_original_dirty_and_cannot_resolve_uncertain_original() {
        let mut state = editing("base");
        let original = Uuid::new_v4();
        let editor = state.editor.as_mut().unwrap();
        editor.view.pending.push(original);
        editor.view.conflict = true;
        editor.edit("rescue".into(), Instant::now()).unwrap();
        assert!(state.save_editor(None).is_none());
        let (id, AppCommand::SaveEditor(request)) =
            state.save_editor(Some("rescue.md".into())).unwrap()
        else {
            panic!("copy");
        };
        state.apply(
            id,
            AppEvent::EditorSaved(SaveReceipt {
                operation_id: id,
                path: request.edit.path,
                destination: request.destination,
                submitted_generation: request.edit.generation,
                stamp: EditStamp {
                    generation: request.edit.generation,
                    ..request.edit.expected
                },
                outcome: SaveOutcome::Applied,
            }),
        );
        let editor = state.editor.as_ref().unwrap();
        assert!(editor.dirty());
        assert_eq!(editor.view.saved.as_deref(), Some("base"));
        assert_eq!(editor.view.pending, vec![original]);
        assert_eq!(editor.status(), "Save outcome uncertain");
    }
    #[test]
    fn oversized_utf8_edits_retain_prior_bytes_and_generation() {
        let mut state = editing("");
        let editor = state.editor.as_mut().unwrap();
        let exact = "é".repeat(512 * 1024);
        editor.edit(exact.clone(), Instant::now()).unwrap();
        assert!(editor.edit(format!("{exact}x"), Instant::now()).is_err());
        assert_eq!(editor.text, exact);
        assert_eq!(editor.generation, 1);
    }
    #[test]
    fn confirmed_reload_binds_reviewed_disk_and_replaces_only_after_matching_ack() {
        let mut state = editing("baseline");
        let editor = state.editor.as_mut().unwrap();
        editor.view.saved = Some("external disk".into());
        editor.view.conflict = true;
        let request = editor.reload_request().unwrap();
        let (id, _) = state.reload_editor(request).unwrap();
        assert!(
            state
                .editor
                .as_mut()
                .unwrap()
                .edit("later typing".into(), Instant::now())
                .is_err()
        );
        let mut record = state.editor.as_ref().unwrap().view.record.clone();
        record.stamp.baseline = Uuid::new_v4();
        record.stamp.generation = 1;
        record.text = "external disk".into();
        record.baseline_text = record.text.clone();
        state.apply(Uuid::new_v4(), AppEvent::EditorRecovered(record.clone()));
        assert_eq!(state.editor.as_ref().unwrap().text, "baseline");
        state.apply(id, AppEvent::EditorRecovered(record));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.text, "external disk");
        assert!(!editor.dirty());
        assert!(editor.can_leave());
        assert!(!editor.view.conflict);
    }
    fn ending(request: &AskRequest, status: WorkTurnStatus) -> WorkTurn {
        WorkTurn {
            id: request.id,
            conversation_id: Uuid::new_v4(),
            question: request.question.clone(),
            answer: "partial λ".into(),
            provider: "copilot".into(),
            model: request.selection.model.clone(),
            status,
            error_code: None,
        }
    }
    #[test]
    fn followup_same_history_click_keeps_owned_stream_and_replaces_running() {
        let mut state = ready();
        let conversation = Uuid::new_v4();
        state.navigate(Some(conversation));
        let request = state.ask("followup".into()).unwrap();
        let mut running = ending(&request, WorkTurnStatus::Running);
        running.conversation_id = conversation;
        state.turns.push(running.clone());
        let (history, _) = state.navigate(Some(conversation)).unwrap();
        state.apply(history, AppEvent::Turns(vec![running.clone()]));
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "full partial".into(),
            }),
        );
        assert_eq!(state.active.as_ref().unwrap().partial, "full partial");
        assert!(state.display_active().is_some());
        assert_eq!(state.display_turns().count(), 0);
        running.status = WorkTurnStatus::Completed;
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Finished {
                id: request.id,
                generation: request.generation,
                turn: running.clone(),
            }),
        );
        assert!(state.active.is_none());
        assert_eq!(state.turns.len(), 1);
        assert_eq!(state.turns[0].status, WorkTurnStatus::Completed);
    }
    #[test]
    fn followup_away_back_and_late_running_snapshot_never_regress_terminal() {
        for snapshot_first in [true, false] {
            let mut state = ready();
            let conversation = Uuid::new_v4();
            state.navigate(Some(conversation));
            let request = state.ask("followup".into()).unwrap();
            state.navigate(Some(Uuid::new_v4()));
            assert!(state.display_active().is_none());
            for (id, generation) in [
                (Uuid::new_v4(), request.generation),
                (request.id, request.generation.wrapping_add(1)),
            ] {
                state.apply(
                    id,
                    AppEvent::Chat(ChatEvent::Text {
                        id: request.id,
                        generation,
                        text: "uncorrelated".into(),
                    }),
                );
            }
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Text {
                    id: request.id,
                    generation: request.generation,
                    text: "away ".into(),
                }),
            );
            let (history, _) = state.navigate(Some(conversation)).unwrap();
            assert!(state.display_active().is_some());
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Text {
                    id: request.id,
                    generation: request.generation,
                    text: "back".into(),
                }),
            );
            assert_eq!(state.active.as_ref().unwrap().partial, "away back");
            let mut running = ending(&request, WorkTurnStatus::Running);
            running.conversation_id = conversation;
            if snapshot_first {
                state.apply(history, AppEvent::Turns(vec![running.clone()]));
                assert_eq!(state.display_turns().count(), 0);
            }
            let mut terminal = running.clone();
            terminal.status = WorkTurnStatus::Completed;
            terminal.answer = "away back".into();
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Finished {
                    id: request.id,
                    generation: request.generation,
                    turn: terminal.clone(),
                }),
            );
            if !snapshot_first {
                state.apply(history, AppEvent::Turns(vec![running]));
            }
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Finished {
                    id: request.id,
                    generation: request.generation,
                    turn: terminal.clone(),
                }),
            );
            assert!(state.active.is_none());
            assert_eq!(state.turns.len(), 1);
            assert_eq!(state.turns[0].status, WorkTurnStatus::Completed);
            assert_eq!(state.turns[0].answer, terminal.answer);
            assert_eq!(state.display_turns().count(), 1);
        }
    }
    #[test]
    fn late_running_snapshot_preserves_known_terminal_without_duplicate() {
        let mut state = ready();
        let conversation = Uuid::new_v4();
        state.navigate(Some(conversation));
        let request = state.ask("followup".into()).unwrap();
        state.navigate(Some(Uuid::new_v4()));
        let (history, _) = state.navigate(Some(conversation)).unwrap();
        let mut running = ending(&request, WorkTurnStatus::Running);
        running.conversation_id = conversation;
        let mut terminal = running.clone();
        terminal.status = WorkTurnStatus::Completed;
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Finished {
                id: request.id,
                generation: request.generation,
                turn: terminal,
            }),
        );
        assert_eq!(state.turns[0].status, WorkTurnStatus::Completed);
        state.apply(history, AppEvent::Turns(vec![running]));
        assert_eq!(state.turns.len(), 1);
        assert_eq!(state.turns[0].status, WorkTurnStatus::Completed);
    }
    #[test]
    fn followup_finished_in_other_or_new_blank_display_clears_global_active_only() {
        for destination in [Some(Uuid::new_v4()), None] {
            let mut state = ready();
            let conversation = Uuid::new_v4();
            state.navigate(Some(conversation));
            let request = state.ask("followup".into()).unwrap();
            state.navigate(destination);
            let mut terminal = ending(&request, WorkTurnStatus::Completed);
            terminal.conversation_id = conversation;
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Finished {
                    id: request.id,
                    generation: request.generation,
                    turn: terminal.clone(),
                }),
            );
            assert!(state.active.is_none());
            assert!(state.turns.is_empty());
            assert_eq!(state.conversation, destination);
            let (history, _) = state.navigate(Some(conversation)).unwrap();
            state.apply(history, AppEvent::Turns(vec![terminal]));
            assert_eq!(state.display_turns().count(), 1);
        }
    }
    #[test]
    fn new_first_chat_blank_navigation_retains_unsaved_without_adopting_conversation() {
        let mut state = ready();
        let request = state.ask("first".into()).unwrap();
        state.navigate(None);
        assert!(state.display_active().is_none());
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "retained".into(),
            }),
        );
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::PersistenceFailed {
                id: request.id,
                generation: request.generation,
                partial: "retained".into(),
                error: brn_workflow::WorkflowError::msg("unsaved"),
            }),
        );
        assert!(state.active.is_none());
        assert!(state.conversation.is_none());
        assert!(state.turns.is_empty());
        assert_eq!(state.unsaved.as_ref().unwrap().answer, "retained");
        assert!(!state.can_ask());
    }
    #[test]
    fn composer_change_keeps_stream_and_followup_conversation() {
        let mut state = ready();
        let request = state.ask("first question".into()).unwrap();
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "partial".into(),
            }),
        );
        state.composer_changed();
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: " λ complete".into(),
            }),
        );
        assert_eq!(state.active.as_ref().unwrap().partial, "partial λ complete");
        assert_eq!(
            state.generation, request.generation,
            "composer must not hide the active answer"
        );
        let mut turn = ending(&request, WorkTurnStatus::Completed);
        turn.answer = "partial λ complete".into();
        let conversation = turn.conversation_id;
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Finished {
                id: request.id,
                generation: request.generation,
                turn,
            }),
        );
        assert!(state.active.is_none());
        assert_eq!(state.turns[0].answer, "partial λ complete");
        let followup = state.ask("followup".into()).unwrap();
        assert_eq!(followup.conversation, Some(conversation));
        assert_eq!(followup.selection, request.selection);
    }
    #[test]
    fn explicit_frozen_selection_and_single_active_ask() {
        let mut state = ready();
        let request = state.ask("question".into()).unwrap();
        state.selection = Some(Selection {
            provider: Provider::Chatgpt,
            model: "other".into(),
        });
        assert_eq!(
            state.active.as_ref().unwrap().request.selection,
            selection()
        );
        assert!(state.ask("second".into()).is_none());
        assert_eq!(request.selection, selection());
    }
    #[test]
    fn stop_before_admission_retries_on_first_progress_and_never_claims_saved() {
        let mut state = ready();
        let request = state.ask("q".into()).unwrap();
        assert_eq!(state.stop(), Some(request.id));
        state.apply(
            Uuid::new_v4(),
            AppEvent::TurnCancelRequested {
                turn: request.id,
                accepted: false,
            },
        );
        let commands = state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "partial".into(),
            }),
        );
        assert!(commands.iter().any(
            |(_, command)| matches!(command, AppCommand::CancelTurn(id) if *id == request.id)
        ));
        assert!(state.active.is_some());
    }
    #[test]
    fn stopped_and_failed_keep_partial_and_stored_identity() {
        for (status, label) in [
            (WorkTurnStatus::Interrupted, "Stopped"),
            (WorkTurnStatus::Failed, "Failed"),
        ] {
            let mut state = ready();
            let request = state.ask("q".into()).unwrap();
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Finished {
                    id: request.id,
                    generation: request.generation,
                    turn: ending(&request, status),
                }),
            );
            assert!(state.active.is_none());
            assert_eq!(state.turns[0].answer, "partial λ");
            assert_eq!(state.turns[0].model, "explicit-model");
            assert_eq!(turn_label(&state.turns[0]), label);
        }
    }
    #[test]
    fn stale_display_does_not_strand_global_active_and_wrong_uuid_is_ignored() {
        let mut state = ready();
        let request = state.ask("q".into()).unwrap();
        let selected = Uuid::new_v4();
        let (history, _) = state.navigate(Some(selected)).unwrap();
        state.apply(
            Uuid::new_v4(),
            AppEvent::Chat(ChatEvent::Text {
                id: Uuid::new_v4(),
                generation: request.generation,
                text: "wrong".into(),
            }),
        );
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "stale".into(),
            }),
        );
        assert_eq!(state.active.as_ref().unwrap().partial, "stale");
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Finished {
                id: request.id,
                generation: request.generation,
                turn: ending(&request, WorkTurnStatus::Completed),
            }),
        );
        assert!(state.active.is_none());
        assert!(state.turns.is_empty());
        assert_eq!(state.conversation, Some(selected));
        state.apply(history, AppEvent::Turns(vec![]));
        assert_eq!(
            state.ask("selected followup".into()).unwrap().conversation,
            Some(selected)
        );
    }
    #[test]
    fn persistence_failure_retains_unsaved_partial() {
        let mut state = ready();
        let request = state.ask("q".into()).unwrap();
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::PersistenceFailed {
                id: request.id,
                generation: request.generation,
                partial: "unsaved".into(),
                error: brn_workflow::WorkflowError::msg("safe error"),
            }),
        );
        assert!(state.active.is_none());
        assert_eq!(state.unsaved.as_ref().unwrap().answer, "unsaved");
        assert!(state.notice.contains("not saved"));
        assert!(!state.can_ask());
        assert!(
            state
                .account(AccountCommand::Status(Provider::Copilot))
                .is_some()
        );
    }
    #[test]
    fn login_cancel_exact_operation_and_failed_connect_queries_actual_status() {
        let mut state = ready();
        let (id, _) = state
            .account(AccountCommand::Connect(Provider::Copilot))
            .unwrap();
        state.apply(
            id,
            AppEvent::Account(AccountEvent::Login {
                id,
                prompt: brn_workflow::LoginPrompt {
                    verification_uri: "https://example.invalid".into(),
                    user_code: "synthetic".into(),
                },
            }),
        );
        assert!(state.login.is_some());
        assert_eq!(state.dismiss_login(), Some(id));
        assert!(state.login.is_none());
        let commands = state.apply(
            id,
            AppEvent::Account(AccountEvent::Finished {
                id,
                provider: Provider::Copilot,
                reply: AccountReply::Failed(AiError::new(AiErrorKind::CodeExpired)),
            }),
        );
        assert!(state.login.is_none());
        assert!(commands.iter().any(|(_, c)| matches!(
            c,
            AppCommand::Account {
                command: AccountCommand::Status(Provider::Copilot),
                ..
            }
        )));
        assert!(
            state.accounts[1]
                .error
                .as_ref()
                .unwrap()
                .contains("expired")
        );
    }
    #[test]
    fn navigation_correlates_history_and_keeps_diagnostic_access_without_selection() {
        let mut state = ready();
        state.selection = None;
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let (old, _) = state.navigate(Some(first)).unwrap();
        let (current, _) = state.navigate(Some(second)).unwrap();
        let request = AskRequest {
            id: Uuid::new_v4(),
            conversation: None,
            question: "q".into(),
            selection: selection(),
            generation: 0,
        };
        state.apply(
            old,
            AppEvent::Turns(vec![ending(&request, WorkTurnStatus::Completed)]),
        );
        assert!(state.turns.is_empty());
        state.apply(
            current,
            AppEvent::Turns(vec![ending(&request, WorkTurnStatus::Completed)]),
        );
        assert_eq!(state.turns.len(), 1);
        assert!(!state.can_ask());
        assert!(
            state
                .account(AccountCommand::Status(Provider::Chatgpt))
                .is_some()
        );
    }
    #[test]
    fn immediate_submission_failure_clears_active_and_login_and_refreshes_cache_status() {
        let mut state = ready();
        let request = state.ask("q".into()).unwrap();
        state.apply(
            request.id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("lane failed")),
        );
        assert!(state.active.is_none());
        let (id, _) = state
            .account(AccountCommand::Connect(Provider::Copilot))
            .unwrap();
        let commands = state.apply(
            id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("lane failed")),
        );
        assert!(state.login.is_none());
        assert!(!state.pending.contains_key(&id));
        assert!(commands.iter().any(|(_, command)| matches!(
            command,
            AppCommand::Account {
                command: AccountCommand::Status(Provider::Copilot),
                ..
            }
        )));
    }
    #[test]
    fn startup_never_admits_login_discovery_download_or_default_selection() {
        let mut state = AiState::default();
        let commands = state.apply(
            Uuid::new_v4(),
            AppEvent::Ready {
                vault_bound: false,
                model_installed: false,
            },
        );
        assert!(state.selection.is_none());
        assert!(
            commands
                .iter()
                .any(|(_, command)| matches!(command, AppCommand::Editors))
        );
        assert!(commands.iter().all(|(_, command)| matches!(
            command,
            AppCommand::Status
                | AppCommand::Selection
                | AppCommand::Conversations
                | AppCommand::ModelPrompt
                | AppCommand::Editors
                | AppCommand::Account {
                    command: AccountCommand::Status(_),
                    ..
                }
        )));
        assert!(!state.can_ask());
    }
    #[test]
    fn recovery_buffer_remains_accessible_when_the_vault_is_unavailable() {
        let mut state = ready();
        state.vault_bound = false;
        let mut view = editor_view("missing.md", "exact recovery\r\n");
        view.saved = None;
        view.observed = None;
        view.conflict = true;
        state.apply(Uuid::new_v4(), AppEvent::Editors(vec![view.record.clone()]));
        assert_eq!(state.editors[0].path, "missing.md");
        let (id, _) = state.open_editor("missing.md".into());
        state.apply(id, AppEvent::Editor(view));
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.text, "exact recovery\r\n");
        assert!(!editor.can_save());
        assert!(editor.can_leave());
    }
    #[test]
    fn correlated_lane_failure_after_delta_keeps_unfinalized_partial_in_memory() {
        let mut state = ready();
        let request = state.ask("q".into()).unwrap();
        state.apply(
            request.id,
            AppEvent::Chat(ChatEvent::Text {
                id: request.id,
                generation: request.generation,
                text: "partial".into(),
            }),
        );
        state.apply(
            request.id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("lane failed")),
        );
        assert!(state.active.is_none());
        assert_eq!(state.unsaved.as_ref().unwrap().answer, "partial");
        assert!(state.notice.contains("not acknowledged"));
    }
    #[test]
    fn failed_connect_can_be_connected_with_unavailable_name_without_automatic_retry() {
        let mut state = ready();
        let (id, _) = state
            .account(AccountCommand::Connect(Provider::Copilot))
            .unwrap();
        let commands = state.apply(
            id,
            AppEvent::Account(AccountEvent::Finished {
                id,
                provider: Provider::Copilot,
                reply: AccountReply::Cancelled,
            }),
        );
        let (status, _) = commands.into_iter().next().unwrap();
        state.apply(
            status,
            AppEvent::Account(AccountEvent::Finished {
                id: status,
                provider: Provider::Copilot,
                reply: AccountReply::Status(AccountStatus {
                    provider: Provider::Copilot,
                    connected: true,
                    name: None,
                }),
            }),
        );
        assert!(state.accounts[1].status.as_ref().unwrap().connected);
        assert!(state.accounts[1].status.as_ref().unwrap().name.is_none());
        assert!(state.login.is_none());
        assert!(!state.account_busy(Provider::Copilot));
    }
    #[test]
    fn every_chat_terminal_unblocks_except_wrong_generation() {
        for choice in 0..2 {
            let mut state = ready();
            let request = state.ask("q".into()).unwrap();
            state.apply(
                request.id,
                AppEvent::Chat(ChatEvent::Rejected {
                    id: request.id,
                    generation: request.generation + 1,
                    error: brn_workflow::WorkflowError::msg("wrong generation"),
                }),
            );
            assert!(state.active.is_some());
            let event = if choice == 0 {
                ChatEvent::Rejected {
                    id: request.id,
                    generation: request.generation,
                    error: brn_workflow::WorkflowError::msg("refused"),
                }
            } else {
                ChatEvent::AlreadyRunning {
                    id: request.id,
                    generation: request.generation,
                    turn: ending(&request, WorkTurnStatus::Running),
                }
            };
            state.apply(request.id, AppEvent::Chat(event));
            assert!(state.active.is_none());
        }
    }
    #[test]
    fn downloaded_is_not_installed_and_failure_ends_correlated_progress() {
        let mut state = ready();
        let (id, _) = state.command(
            Pending::Download,
            AppCommand::DownloadModel {
                consent: true,
                target: PathBuf::from("/synthetic/model"),
            },
        );
        state.download = Some(id);
        state.apply(
            id,
            AppEvent::ModelDownload {
                received: 7,
                total: 10,
            },
        );
        state.apply(
            id,
            AppEvent::ModelDownloaded(brn_workflow::models::ModelInstallReport {
                directory: PathBuf::from("/synthetic/model"),
                downloaded_bytes: 10,
            }),
        );
        assert!(!state.model_installed);
        assert_eq!(state.download, Some(id));
        state.apply(
            id,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("activation failed")),
        );
        assert!(state.download.is_none());
        assert!(state.download_progress.is_none());
        assert!(!state.model_installed);
        assert!(state.model_state.contains("failed"));
        let (decline, _) = state.command(
            Pending::Download,
            AppCommand::DownloadModel {
                consent: false,
                target: PathBuf::from("/synthetic/model"),
            },
        );
        state.download = Some(decline);
        state.apply(decline, AppEvent::ModelDownloadDeclined);
        assert!(state.download.is_none());
        assert!(state.model_prompt.is_none());
    }
    #[test]
    fn stale_selection_error_keeps_ready_history_and_accounts_available() {
        let mut state = ready();
        let (id, _) = state.command(Pending::Selection, AppCommand::Selection);
        state.apply(
            id,
            AppEvent::Failed(brn_workflow::WorkflowError {
                kind: brn_workflow::ErrorKind::ModelRefused,
                message: "saved selection unavailable".into(),
            }),
        );
        assert!(state.ready);
        assert!(!state.can_ask());
        assert!(state.navigate(Some(Uuid::new_v4())).is_some());
        assert!(
            state
                .account(AccountCommand::Status(Provider::Copilot))
                .is_some()
        );
    }
    #[test]
    fn stale_model_decline_does_not_cancel_newer_explicit_installation() {
        let mut state = ready();
        let active = Uuid::new_v4();
        state.download = Some(active);
        state.apply(Uuid::new_v4(), AppEvent::ModelDownloadDeclined);
        assert_eq!(state.download, Some(active));
    }
    #[test]
    fn declining_later_download_does_not_claim_installed_model_is_keyword_only() {
        let mut state = ready();
        state.model_installed = true;
        let id = Uuid::new_v4();
        state.download = Some(id);
        state.apply(id, AppEvent::ModelDownloadDeclined);
        assert!(state.model_installed);
        assert!(!state.model_state.contains("keyword-only"));
    }
    #[test]
    fn cancel_download_intent_survives_control_before_admission_and_ends_only_on_terminal() {
        let mut state = ready();
        let active = Uuid::new_v4();
        state.download = Some(active);
        state.download_stopping = true;
        state.apply(
            Uuid::new_v4(),
            AppEvent::ModelCancelRequested {
                operation: active,
                accepted: false,
            },
        );
        assert!(
            state
                .stop_controls()
                .iter()
                .any(|(_, c)| matches!(c, AppCommand::CancelModelDownload(id) if *id == active))
        );
        state.apply(
            active,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("cancelled")),
        );
        assert!(!state.download_stopping);
        assert!(state.stop_controls().is_empty());
    }
    #[test]
    fn note_failure_is_terminal_and_stale_read_does_not_replace_new_document() {
        let mut state = ready();
        let (old, _) = state.open_editor("old.md".into());
        let (current, _) = state.open_editor("current.md".into());
        state.apply(old, AppEvent::Editor(editor_view("old.md", "old")));
        assert!(state.editor.is_none());
        state.apply(
            current,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("note unavailable")),
        );
        assert!(!state.pending.contains_key(&current));
        assert_eq!(state.note_error.as_deref(), Some("note unavailable"));
    }
    #[test]
    fn search_refresh_unreadable_and_index_failure_are_projected_without_invented_model() {
        let mut state = ready();
        let (id, _) = state.command(
            Pending::Search {
                generation: state.search_generation,
            },
            AppCommand::Search {
                query: "q".into(),
                mode: brn_workflow::library::SearchMode::Hybrid,
                limit: 10,
            },
        );
        state.apply(
            id,
            AppEvent::Search(SearchResults {
                hits: vec![],
                keyword_only: true,
            }),
        );
        assert!(state.search.as_ref().unwrap().keyword_only);
        let (refresh, _) = state.command(Pending::Refresh, AppCommand::Refresh);
        state.apply(
            refresh,
            AppEvent::Refreshed(RefreshReport {
                unreadable: vec![brn_workflow::library::Unreadable {
                    path: "bad.md".into(),
                    reason: "invalid UTF-8",
                }],
                ..Default::default()
            }),
        );
        assert_eq!(state.refresh.as_ref().unwrap().unreadable[0].path, "bad.md");
        state.apply(
            refresh,
            AppEvent::Indexing {
                embedded: 16,
                total: 35,
            },
        );
        assert_eq!(state.indexing, Some((refresh, 16, 35)));
        state.apply(
            refresh,
            AppEvent::Failed(brn_workflow::WorkflowError::msg("index failed")),
        );
        assert!(state.indexing.is_none());
        assert!(!state.model_installed);
    }
}
