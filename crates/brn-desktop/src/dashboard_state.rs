//! Dashboard presentation and exact command correlation; no domain queries.
use super::{AiState, Pending};
use brn_workflow::{
    WorkflowError,
    action_completion::{ActionCompletion, CompleteActionRequest},
    actions::ActionState,
    app_worker::{AppCommand, AppEvent},
    dashboard::{DashboardEntry, DashboardFilter, DashboardPage, DashboardRequest},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct DashboardQuery {
    view: u64,
    page: u64,
    request: DashboardRequest,
}
#[derive(Clone)]
pub struct CompletionCapture {
    view: u64,
    page: u64,
    selection: u64,
    request: CompleteActionRequest,
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
    /// Transient input/outcomes retained across view navigation. Durable recovery
    /// remains in the application, not in this presentation list.
    pub attempts: Vec<CompletionAttempt>,
    view: u64,
    page_generation: u64,
    selection: u64,
}
impl AiState {
    pub fn open_dashboard(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready {
            return None;
        }
        self.dashboard.visible = true;
        self.dashboard.view = self.dashboard.view.wrapping_add(1);
        self.refresh_dashboard(self.dashboard.filter, false)
    }
    pub fn close_dashboard(&mut self) {
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
        };
        request.validate().ok()?;
        Some(CompletionCapture {
            view: self.dashboard.view,
            page: self.dashboard.page_generation,
            selection: self.dashboard.selection,
            request,
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
        self.pending
            .insert(operation, Pending::ActionComplete(Box::new(capture)));
        Some((operation, command))
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
                            "Completion {}: {}. Exact request retained for retry.",
                            id, error.message
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
