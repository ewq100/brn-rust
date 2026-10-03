//! Presentation and correlation only. Authority and network work belong to AppWorker.
#![cfg_attr(not(feature = "native-ui"), allow(dead_code))]
use brn_workflow::{
    AccountStatus, LoginPrompt, ModelOption, NoteEntry, Provider, Selection, WorkConversation,
    WorkTurn, WorkTurnStatus,
    app_worker::{AppCommand, AppEvent},
    chat_worker::{AccountCommand, AccountEvent, AccountReply, AskRequest, ChatEvent},
    library::{RefreshReport, SearchResults},
    models::ModelDownloadPrompt,
    vault::NoteText,
};
use std::{collections::HashMap, path::PathBuf};
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
    Note { generation: u64 },
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
    pub note: Option<NoteText>,
    pub note_error: Option<String>,
    pub note_generation: u64,
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
            AppEvent::Note(note) => {
                if matches!(pending, Some(Pending::Note { generation }) if generation == self.note_generation)
                {
                    self.note = Some(note);
                }
            }
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
                if matches!(pending, Some(Pending::Note { generation }) if generation == self.note_generation)
                {
                    self.note_error = Some(error.message.clone());
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
        assert!(commands.iter().all(|(_, command)| matches!(
            command,
            AppCommand::Status
                | AppCommand::Selection
                | AppCommand::Conversations
                | AppCommand::ModelPrompt
                | AppCommand::Account {
                    command: AccountCommand::Status(_),
                    ..
                }
        )));
        assert!(!state.can_ask());
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
        state.note_generation = 3;
        let (old, _) = state.command(
            Pending::Note { generation: 2 },
            AppCommand::Note("old.md".into()),
        );
        let (current, _) = state.command(
            Pending::Note { generation: 3 },
            AppCommand::Note("current.md".into()),
        );
        state.apply(
            old,
            AppEvent::Note(NoteText {
                text: "old".into(),
                sha256: [0; 32],
            }),
        );
        assert!(state.note.is_none());
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
