//! Dashboard presentation and exact command correlation; no domain queries.
use super::{AiState, Pending};
use brn_workflow::{
    WorkflowError,
    action_completion::{
        ActionCompletion, CompleteActionRequest, PrepareSentCompletionRequest,
        SentCompletionPreview,
    },
    actions::{ActionRecord, ActionState},
    app_worker::{AppCommand, AppEvent},
    dashboard::{DashboardEntry, DashboardFilter, DashboardPage, DashboardRequest},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkedActionRole {
    Dependency,
    Parent,
    FollowsUp,
}
impl LinkedActionRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dependency => "Dependency",
            Self::Parent => "Parent",
            Self::FollowsUp => "Follows up",
        }
    }
    fn contains(self, source: &ActionRecord, target: Uuid) -> bool {
        match self {
            Self::Dependency => source.data.dependencies.contains(&target),
            Self::Parent => source.data.parent == Some(target),
            Self::FollowsUp => source.data.follows_up == Some(target),
        }
    }
}
#[derive(Clone)]
pub struct LinkedActionCapture {
    pub source: Box<ActionRecord>,
    pub target: Uuid,
    pub role: LinkedActionRole,
    view: u64,
    page: u64,
    selection: u64,
    generation: u64,
    session_navigation: u64,
    session_generation: u64,
}
pub struct LinkedActionDetail {
    pub capture: LinkedActionCapture,
    pub record: Option<ActionRecord>,
    pub error: Option<WorkflowError>,
    pub intent: Option<Uuid>,
}

#[derive(Clone)]
pub struct DashboardQuery {
    view: u64,
    page: u64,
    request: DashboardRequest,
}
#[derive(Clone)]
pub struct SentPreparationCapture {
    view: u64,
    page: u64,
    selection: u64,
    generation: u64,
    request: PrepareSentCompletionRequest,
}
#[derive(Clone)]
pub struct PreparedSentCompletion {
    capture: SentPreparationCapture,
    pub preview: SentCompletionPreview,
}
#[derive(Clone)]
pub struct CompletionCapture {
    view: u64,
    page: u64,
    selection: u64,
    request: CompleteActionRequest,
    sent: Option<PreparedSentCompletion>,
}
impl CompletionCapture {
    pub fn request(&self) -> &CompleteActionRequest {
        &self.request
    }
}
pub struct CompletionAttempt {
    pub request: CompleteActionRequest,
    pub error: Option<WorkflowError>,
    pub receipt: Option<ActionCompletion>,
}
#[derive(Default)]
pub struct DashboardView {
    pub visible: bool,
    pub filter: DashboardFilter,
    pub page: Option<DashboardPage>,
    pub selected: Option<DashboardEntry>,
    pub error: Option<String>,
    pub linked: Option<LinkedActionDetail>,
    linked_generation: u64,
    /// Transient input/outcomes retained across view navigation. Durable recovery
    /// remains in the application, not in this presentation list.
    pub attempts: Vec<CompletionAttempt>,
    pub sent_source_path: String,
    pub sent_preview: Option<PreparedSentCompletion>,
    pub sent_error: Option<String>,
    sent_generation: u64,
    view: u64,
    page_generation: u64,
    selection: u64,
}
impl AiState {
    pub fn clear_linked_action(&mut self) {
        self.dashboard.linked_generation = self.dashboard.linked_generation.wrapping_add(1);
        self.dashboard.linked = None;
    }
    pub fn linked_action_capture_current(&self, capture: &LinkedActionCapture) -> bool {
        self.ready
            && self.dashboard.visible
            && !self.dashboard_loading()
            && !self.application_busy()
            && self.active.is_none()
            && self.rewrite.is_none()
            && !self.session_change_pending()
            && self.dashboard.view == capture.view
            && self.dashboard.page_generation == capture.page
            && self.dashboard.selection == capture.selection
            && self.dashboard.linked_generation == capture.generation
            && self.generation == capture.session_navigation
            && self.session_history.list_generation == capture.session_generation
            && self
                .dashboard
                .selected
                .as_ref()
                .is_some_and(|entry| entry.action == *capture.source)
            && !capture.target.is_nil()
            && capture.source.validate().is_ok()
            && capture.role.contains(&capture.source, capture.target)
    }
    pub fn capture_linked_action(
        &self,
        role: LinkedActionRole,
        target: Uuid,
    ) -> Option<LinkedActionCapture> {
        let capture = LinkedActionCapture {
            source: Box::new(self.dashboard.selected.as_ref()?.action.clone()),
            target,
            role,
            view: self.dashboard.view,
            page: self.dashboard.page_generation,
            selection: self.dashboard.selection,
            generation: self.dashboard.linked_generation,
            session_navigation: self.generation,
            session_generation: self.session_history.list_generation,
        };
        self.linked_action_capture_current(&capture)
            .then_some(capture)
    }
    pub fn linked_action_loading(&self) -> bool {
        self.dashboard.linked.as_ref().is_some_and(|detail| {
            self.linked_action_capture_current(&detail.capture)
                && detail.intent.is_some_and(|id| {
                    matches!(self.pending.get(&id), Some(Pending::LinkedAction(_)))
                })
        })
    }
    pub fn inspect_linked_action(
        &mut self,
        capture: &LinkedActionCapture,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.linked_action_capture_current(capture) || self.linked_action_loading() {
            return None;
        }
        self.clear_linked_action();
        let mut capture = capture.clone();
        capture.generation = self.dashboard.linked_generation;
        let command = self.command(
            Pending::LinkedAction(Box::new(capture.clone())),
            AppCommand::Action(capture.target),
        );
        self.dashboard.linked = Some(LinkedActionDetail {
            capture,
            record: None,
            error: None,
            intent: Some(command.0),
        });
        Some(command)
    }
    pub fn retry_linked_action(&mut self) -> Option<(Uuid, AppCommand)> {
        let detail = self.dashboard.linked.as_ref()?;
        if detail.error.is_none() || detail.intent.is_some() {
            return None;
        }
        self.inspect_linked_action(&detail.capture.clone())
    }
    /// Consume typed linked reads before generic event settlement. Malformed or
    /// unrelated replies never settle this intent or another owner operation.
    pub(super) fn apply_linked_action_event(&mut self, id: Uuid, event: &AppEvent) -> bool {
        if self
            .dashboard
            .linked
            .as_ref()
            .is_some_and(|detail| !self.linked_action_capture_current(&detail.capture))
        {
            self.clear_linked_action();
        }
        let Some(Pending::LinkedAction(capture)) = self.pending.get(&id).cloned() else {
            return matches!(event, AppEvent::Action(_));
        };
        if !self.linked_action_capture_current(&capture)
            || self
                .dashboard
                .linked
                .as_ref()
                .is_none_or(|detail| detail.intent != Some(id))
        {
            self.pending.remove(&id);
            return true;
        }
        let detail = self.dashboard.linked.as_mut().unwrap();
        match event {
            AppEvent::Action(record)
                if record.origin.id == capture.target && record.validate().is_ok() =>
            {
                detail.record = Some((**record).clone());
                detail.error = None;
            }
            AppEvent::Failed(error) => detail.error = Some(error.clone()),
            _ => return true,
        }
        detail.intent = None;
        self.pending.remove(&id);
        true
    }
    pub fn open_dashboard(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready {
            return None;
        }
        self.dashboard.visible = true;
        self.dashboard.view = self.dashboard.view.wrapping_add(1);
        self.refresh_dashboard(self.dashboard.filter, false)
    }
    pub fn close_dashboard(&mut self) {
        self.clear_linked_action();
        self.dashboard.visible = false;
        self.dashboard.view = self.dashboard.view.wrapping_add(1);
        self.dashboard.page = None;
        self.dashboard.selected = None;
        self.dashboard.error = None;
        self.dashboard.selection = self.dashboard.selection.wrapping_add(1);
    }
    pub fn dashboard_loading(&self) -> bool {
        self.pending.values().any(|pending| {
            matches!(pending,
            Pending::Dashboard(query) if self.dashboard_query_current(query))
        })
    }
    pub fn refresh_dashboard(
        &mut self,
        filter: DashboardFilter,
        older: bool,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.dashboard.visible {
            return None;
        }
        let request = if older {
            if filter != self.dashboard.filter {
                return None;
            }
            let page = self.dashboard.page.as_ref()?;
            DashboardRequest {
                filter,
                as_of: Some(page.as_of.clone()),
                before: Some(page.next_before?),
                ..Default::default()
            }
        } else {
            DashboardRequest {
                filter,
                ..Default::default()
            }
        };
        request.validate().ok()?;
        self.clear_linked_action();
        let view = &mut self.dashboard;
        view.filter = filter;
        view.page_generation = view.page_generation.wrapping_add(1);
        view.selection = view.selection.wrapping_add(1);
        view.page = None;
        view.selected = None;
        view.error = None;
        let capture = DashboardQuery {
            view: view.view,
            page: view.page_generation,
            request: request.clone(),
        };
        Some(self.command(
            Pending::Dashboard(capture),
            AppCommand::ActionDashboard(request),
        ))
    }
    pub fn select_dashboard_action(&mut self, id: Uuid) -> bool {
        if !self.ready || !self.dashboard.visible || self.dashboard_loading() {
            return false;
        }
        let Some(entry) = self
            .dashboard
            .page
            .as_ref()
            .and_then(|page| {
                page.entries
                    .iter()
                    .find(|entry| entry.action.origin.id == id)
            })
            .cloned()
        else {
            return false;
        };
        self.dashboard.selection = self.dashboard.selection.wrapping_add(1);
        self.clear_linked_action();
        self.dashboard.selected = Some(entry);
        true
    }
    pub fn action_completion_available(&self) -> bool {
        self.ready
            && self.dashboard.visible
            && !self.dashboard_loading()
            && !self.application_busy()
            && self.active.is_none()
            && self.rewrite.is_none()
            && self.dashboard.selected.as_ref().is_some_and(|entry| {
                entry.action.data.state != ActionState::Completed && entry.action.validate().is_ok()
            })
    }
    pub fn capture_action_completion(&self) -> Option<CompletionCapture> {
        if !self.action_completion_available() {
            return None;
        }
        let before = &self.dashboard.selected.as_ref()?.action;
        let request = CompleteActionRequest {
            operation_id: Uuid::new_v4(),
            before: Box::new(before.clone()),
            sent_source: None,
        };
        request.validate().ok()?;
        Some(CompletionCapture {
            view: self.dashboard.view,
            page: self.dashboard.page_generation,
            selection: self.dashboard.selection,
            request,
            sent: None,
        })
    }
    pub fn action_completion_capture_current(&self, capture: &CompletionCapture) -> bool {
        self.ready
            && self.dashboard.visible
            && !self.dashboard_loading()
            && !self.application_busy()
            && self.active.is_none()
            && self.rewrite.is_none()
            && self.dashboard.view == capture.view
            && self.dashboard.page_generation == capture.page
            && self.dashboard.selection == capture.selection
            && self
                .dashboard
                .selected
                .as_ref()
                .is_some_and(|entry| entry.action == *capture.request.before)
            && capture.request.validate().is_ok()
            && match &capture.sent {
                Some(sent) => {
                    self.sent_capture_current(&sent.capture)
                        && sent.preview.validate_for(&sent.capture.request).is_ok()
                        && sent.preview.request == capture.request
                }
                None => capture.request.sent_source.is_none(),
            }
            && !self
                .dashboard
                .attempts
                .iter()
                .any(|attempt| attempt.request.operation_id == capture.request.operation_id)
    }
    pub fn confirm_action_completion(
        &mut self,
        capture: &CompletionCapture,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.action_completion_capture_current(capture) {
            return None;
        }
        self.dashboard.attempts.push(CompletionAttempt {
            request: capture.request.clone(),
            error: None,
            receipt: None,
        });
        self.submit_completion(capture.clone())
    }
    pub fn retry_action_completion(&mut self, operation: Uuid) -> Option<(Uuid, AppCommand)> {
        if !self.ready || self.application_busy() || self.active.is_some() || self.rewrite.is_some()
        {
            return None;
        }
        let attempt = self
            .dashboard
            .attempts
            .iter()
            .find(|attempt| attempt.request.operation_id == operation)?;
        if attempt.receipt.is_some() || attempt.error.is_none() {
            return None;
        }
        let request = attempt.request.clone();
        self.submit_completion(CompletionCapture {
            view: self.dashboard.view,
            page: self.dashboard.page_generation,
            selection: self.dashboard.selection,
            request,
            sent: None,
        })
    }
    fn submit_completion(&mut self, capture: CompletionCapture) -> Option<(Uuid, AppCommand)> {
        let operation = capture.request.operation_id;
        if self.pending.contains_key(&operation) {
            return None;
        }
        let attempt = self
            .dashboard
            .attempts
            .iter_mut()
            .find(|attempt| attempt.request == capture.request)?;
        attempt.error = None;
        let command = AppCommand::CompleteAction(capture.request.clone());
        self.clear_linked_action();
        self.clear_profile_context();
        self.pending
            .insert(operation, Pending::ActionComplete(Box::new(capture)));
        Some((operation, command))
    }
    pub fn edit_sent_source_path(&mut self, path: String) {
        if self.dashboard.sent_source_path != path {
            self.dashboard.sent_source_path = path;
            self.dashboard.sent_generation = self.dashboard.sent_generation.wrapping_add(1);
            self.dashboard.sent_error = None;
        }
    }
    pub fn sent_completion_loading(&self) -> bool {
        self.pending.values().any(|pending| {
            matches!(pending,
            Pending::PrepareSentCompletion(capture) if self.sent_capture_current(capture))
        })
    }
    fn sent_capture_current(&self, capture: &SentPreparationCapture) -> bool {
        self.dashboard.visible
            && self.dashboard.view == capture.view
            && self.dashboard.page_generation == capture.page
            && self.dashboard.selection == capture.selection
            && self.dashboard.sent_generation == capture.generation
            && self.dashboard.sent_source_path == capture.request.source_path
            && self
                .dashboard
                .selected
                .as_ref()
                .is_some_and(|entry| entry.action == *capture.request.before)
    }
    pub fn prepare_sent_action_completion(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.action_completion_available() || self.sent_completion_loading() {
            return None;
        }
        let request = PrepareSentCompletionRequest {
            before: Box::new(self.dashboard.selected.as_ref()?.action.clone()),
            source_path: self.dashboard.sent_source_path.clone(),
        };
        if let Err(error) = request.validate() {
            self.dashboard.sent_error = Some(error.to_string());
            return None;
        }
        self.dashboard.sent_generation = self.dashboard.sent_generation.wrapping_add(1);
        self.dashboard.sent_error = None;
        let capture = SentPreparationCapture {
            view: self.dashboard.view,
            page: self.dashboard.page_generation,
            selection: self.dashboard.selection,
            generation: self.dashboard.sent_generation,
            request: request.clone(),
        };
        Some(self.command(
            Pending::PrepareSentCompletion(Box::new(capture)),
            AppCommand::PrepareSentActionCompletion(request),
        ))
    }
    pub fn sent_action_completion_available(&self) -> bool {
        self.action_completion_available()
            && !self.sent_completion_loading()
            && self.dashboard.sent_preview.as_ref().is_some_and(|sent| {
                self.sent_capture_current(&sent.capture)
                    && sent.preview.validate_for(&sent.capture.request).is_ok()
            })
    }
    pub fn capture_sent_action_completion(&self) -> Option<CompletionCapture> {
        if !self.sent_action_completion_available() {
            return None;
        }
        let sent = self.dashboard.sent_preview.as_ref()?;
        Some(CompletionCapture {
            view: self.dashboard.view,
            page: self.dashboard.page_generation,
            selection: self.dashboard.selection,
            request: sent.preview.request.clone(),
            sent: Some(sent.clone()),
        })
    }
    fn dashboard_query_current(&self, query: &DashboardQuery) -> bool {
        self.dashboard.visible
            && self.dashboard.view == query.view
            && self.dashboard.page_generation == query.page
            && self.dashboard.filter == query.request.filter
    }
    pub(super) fn received_dashboard(
        &mut self,
        id: Uuid,
        event: &AppEvent,
    ) -> Option<Vec<(Uuid, AppCommand)>> {
        let pending = self.pending.get(&id).cloned();
        match pending {
            Some(Pending::PrepareSentCompletion(capture)) => {
                if !self.sent_capture_current(&capture) {
                    self.pending.remove(&id);
                    return Some(Vec::new());
                }
                match event {
                    AppEvent::SentActionCompletionPrepared(preview)
                        if preview.validate_for(&capture.request).is_ok() =>
                    {
                        self.dashboard.sent_preview = Some(PreparedSentCompletion {
                            capture: *capture,
                            preview: (**preview).clone(),
                        });
                        self.dashboard.sent_error = None;
                    }
                    AppEvent::Failed(error) => {
                        self.dashboard.sent_error = Some(format!(
                            "Source preparation failed: {}. Input retained; read again explicitly.",
                            error.message
                        ))
                    }
                    _ => return Some(Vec::new()),
                }
                self.pending.remove(&id);
                Some(Vec::new())
            }
            Some(Pending::Dashboard(query)) => {
                if !self.dashboard_query_current(&query) {
                    self.pending.remove(&id);
                    return Some(Vec::new());
                }
                match event {
                    AppEvent::ActionDashboard(page)
                        if query.request.validate_page(page).is_ok() =>
                    {
                        self.dashboard.page = Some((**page).clone());
                        self.dashboard.error = None;
                    }
                    AppEvent::Failed(error) => self.dashboard.error = Some(error.message.clone()),
                    _ => return Some(Vec::new()),
                }
                self.pending.remove(&id);
                Some(Vec::new())
            }
            Some(Pending::ActionComplete(capture)) => {
                let index = self
                    .dashboard
                    .attempts
                    .iter()
                    .position(|attempt| attempt.request == capture.request)?;
                let mut commands = Vec::new();
                match event {
                    AppEvent::ActionCompleted(receipt)
                        if receipt.request == capture.request
                            && receipt.request.operation_id == id
                            && receipt.validate().is_ok() =>
                    {
                        self.dashboard.attempts[index].receipt = Some((**receipt).clone());
                        self.dashboard.attempts[index].error = None;
                        self.pending.remove(&id);
                        self.notice = format!(
                            "Action {} completed; exact operation {} acknowledged.",
                            receipt.after.origin.id, id
                        );
                        // A fresh observation replaces the whole page/counts. Never
                        // install this historical result into another selection.
                        if self.dashboard.visible
                            && let Some(refresh) =
                                self.refresh_dashboard(self.dashboard.filter, false)
                        {
                            commands.push(refresh);
                        }
                    }
                    AppEvent::Failed(error) => {
                        self.dashboard.attempts[index].error = Some(error.clone());
                        self.notice = format!(
                            "{}Completion {} not confirmed: {}. Inspect the current Action and retained attempt; exact request retained for retry.",
                            if capture.request.sent_source.is_some() {
                                "Source already retained. "
                            } else {
                                ""
                            },
                            id,
                            error.message
                        );
                        self.pending.remove(&id);
                    }
                    _ => {} // Misbound/incomplete body cannot acknowledge admission.
                }
                Some(commands)
            }
            None if self
                .dashboard
                .attempts
                .iter()
                .any(|attempt| attempt.request.operation_id == id) =>
            {
                Some(Vec::new())
            }
            _ => None,
        }
    }
}
