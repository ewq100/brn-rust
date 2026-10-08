//! Session lifecycle presentation only; checked transitions belong to workflow.
use super::{AiState, Pending};
use brn_workflow::{
    ErrorKind,
    app_worker::{AppCommand, AppEvent},
    conversations::{
        ConversationFilter, ConversationLifecycle, ConversationLifecycleRequest,
        ConversationLifecycleResult, ConversationState, ConversationSummary,
    },
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct SessionHistory {
    pub filter: ConversationFilter,
    pub summaries: Vec<ConversationSummary>,
    pub lifecycle: HashMap<Uuid, ConversationLifecycle>,
    pub error: Option<String>,
    pub list_generation: u64,
    pub pending: Option<LifecycleCapture>,
    missing_legacy: HashSet<Uuid>,
}
impl Default for SessionHistory {
    fn default() -> Self {
        Self {
            filter: ConversationFilter::Active,
            summaries: Vec::new(),
            lifecycle: HashMap::new(),
            error: None,
            list_generation: 0,
            pending: None,
            missing_legacy: HashSet::new(),
        }
    }
}

#[derive(Clone)]
pub struct LifecycleCapture {
    pub request: ConversationLifecycleRequest,
    before: ConversationLifecycle,
    conversation: Uuid,
    navigation: u64,
    list_generation: u64,
    filter: ConversationFilter,
}

#[derive(Clone)]
pub enum SessionPending {
    Summaries {
        filter: ConversationFilter,
        generation: u64,
    },
    Selected {
        id: Uuid,
        navigation: u64,
    },
    Review {
        id: Uuid,
        proposal: Uuid,
        generation: u64,
    },
    Change(LifecycleCapture),
}

fn valid(lifecycle: &ConversationLifecycle) -> bool {
    !lifecycle.stamp.id.is_nil()
        && lifecycle.stamp.version > 0
        && lifecycle.stamp.version <= i64::MAX as u64
}
fn included(filter: ConversationFilter, state: ConversationState) -> bool {
    matches!(filter, ConversationFilter::All)
        || matches!(
            (filter, state),
            (ConversationFilter::Active, ConversationState::Active)
                | (ConversationFilter::Archived, ConversationState::Archived)
        )
}

impl AiState {
    pub fn session_change_pending(&self) -> bool {
        self.session_history.pending.is_some()
    }
    pub fn selected_session_lifecycle(&self) -> Option<&ConversationLifecycle> {
        self.session_history.lifecycle.get(&self.conversation?)
    }
    pub fn session_allows_new_work(&self) -> bool {
        !self.session_change_pending()
            && self.conversation.is_none_or(|id| {
                self.session_history
                    .lifecycle
                    .get(&id)
                    .is_some_and(|value| value.state == ConversationState::Active)
            })
    }
    pub(super) fn review_session_allows_new_work(&self) -> bool {
        self.review.as_ref().is_none_or(|review| {
            review.record.draft.session_id.is_none_or(|id| {
                self.session_history
                    .lifecycle
                    .get(&id)
                    .is_some_and(|value| value.state == ConversationState::Active)
                    || self.session_history.missing_legacy.contains(&id)
            })
        })
    }
    pub fn refresh_session_summaries(&mut self) -> (Uuid, AppCommand) {
        self.session_history.list_generation = self.session_history.list_generation.wrapping_add(1);
        self.command(
            Pending::Session(SessionPending::Summaries {
                filter: self.session_history.filter,
                generation: self.session_history.list_generation,
            }),
            AppCommand::ConversationSummaries(self.session_history.filter),
        )
    }
    pub fn select_session_filter(
        &mut self,
        filter: ConversationFilter,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.ready || self.session_change_pending() {
            return None;
        }
        self.session_history.filter = filter;
        self.session_history.error = None;
        self.session_history.summaries.clear();
        Some(self.refresh_session_summaries())
    }
    pub fn can_change_session_lifecycle(&self) -> bool {
        self.ready
            && !self.session_change_pending()
            && !self.application_busy()
            && self.active.is_none()
            && self.rewrite.is_none()
            && self.selected_session_lifecycle().is_some()
    }
    pub fn set_selected_session_lifecycle(
        &mut self,
        target: ConversationState,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.can_change_session_lifecycle() {
            return None;
        }
        let before = *self.selected_session_lifecycle()?;
        if before.state == target || !valid(&before) || before.stamp.version == i64::MAX as u64 {
            return None;
        }
        self.session_history.list_generation = self.session_history.list_generation.wrapping_add(1);
        let request = ConversationLifecycleRequest {
            operation_id: Uuid::new_v4(),
            expected: before.stamp,
            target,
        };
        let capture = LifecycleCapture {
            request: request.clone(),
            before,
            conversation: self.conversation?,
            navigation: self.generation,
            list_generation: self.session_history.list_generation,
            filter: self.session_history.filter,
        };
        self.session_history.pending = Some(capture.clone());
        self.session_history.error = None;
        self.pending.insert(
            request.operation_id,
            Pending::Session(SessionPending::Change(capture)),
        );
        Some((
            request.operation_id,
            AppCommand::SetConversationLifecycle(request),
        ))
    }
    pub(super) fn request_selected_lifecycle(&mut self) -> Option<(Uuid, AppCommand)> {
        let id = self.conversation?;
        Some(self.command(
            Pending::Session(SessionPending::Selected {
                id,
                navigation: self.generation,
            }),
            AppCommand::ConversationLifecycle(id),
        ))
    }
    pub(super) fn request_review_lifecycle(&mut self) -> Option<(Uuid, AppCommand)> {
        let review = self.review.as_ref()?;
        let id = review.record.draft.session_id?;
        let proposal = review.record.draft.id;
        self.session_history.missing_legacy.remove(&id);
        Some(self.command(
            Pending::Session(SessionPending::Review {
                id,
                proposal,
                generation: self.review_generation,
            }),
            AppCommand::ConversationLifecycle(id),
        ))
    }
    fn observe_lifecycle(&mut self, lifecycle: &ConversationLifecycle) -> bool {
        if !valid(lifecycle) {
            return false;
        }
        let id = lifecycle.stamp.id;
        if let Some(known) = self.session_history.lifecycle.get(&id) {
            if known.stamp.version > lifecycle.stamp.version {
                return false;
            }
            if known.stamp.version == lifecycle.stamp.version && known != lifecycle {
                return false;
            }
        }
        self.session_history.missing_legacy.remove(&id);
        self.session_history.lifecycle.insert(id, *lifecycle);
        true
    }
    fn session_capture_current(&self, capture: &LifecycleCapture) -> bool {
        self.conversation == Some(capture.conversation)
            && self.generation == capture.navigation
            && self.session_history.list_generation == capture.list_generation
            && self.session_history.filter == capture.filter
            && self
                .session_history
                .pending
                .as_ref()
                .is_some_and(|pending| {
                    pending.request == capture.request
                        && pending.navigation == capture.navigation
                        && pending.list_generation == capture.list_generation
                })
    }
    fn valid_session_result(
        capture: &LifecycleCapture,
        result: &ConversationLifecycleResult,
    ) -> bool {
        let receipt = &result.receipt;
        receipt.request == capture.request
            && receipt.before == capture.before
            && valid(&receipt.after)
            && receipt.after.stamp.id == capture.conversation
            && receipt.after.stamp.version == capture.before.stamp.version + 1
            && receipt.after.state == capture.request.target
            && receipt.after.state != receipt.before.state
            && valid(&result.current)
            && result.current.stamp.id == capture.conversation
            && result.current.stamp.version >= receipt.after.stamp.version
            && (result.current.stamp.version - receipt.after.stamp.version).is_multiple_of(2)
                == (result.current.state == receipt.after.state)
    }
    /// Consume only session replies, preserving every editor/review/chat buffer.
    pub(super) fn apply_session_event(
        &mut self,
        operation: Uuid,
        event: &AppEvent,
    ) -> Option<Vec<(Uuid, AppCommand)>> {
        let pending = match self.pending.get(&operation) {
            Some(Pending::Session(pending)) => pending.clone(),
            _ => {
                return matches!(
                    event,
                    AppEvent::ConversationSummaries { .. }
                        | AppEvent::ConversationLifecycle(_)
                        | AppEvent::ConversationLifecycleChanged(_)
                )
                .then(Vec::new);
            }
        };
        let mut commands = Vec::new();
        let mut accepted = true;
        match (&pending, event) {
            (
                SessionPending::Summaries { filter, generation },
                AppEvent::ConversationSummaries {
                    filter: actual,
                    summaries,
                },
            ) => {
                if filter == actual
                    && *filter == self.session_history.filter
                    && *generation == self.session_history.list_generation
                    && !self.session_change_pending()
                {
                    let mut seen = HashSet::new();
                    let checked = summaries.iter().all(|summary| {
                        summary.conversation.id == summary.lifecycle.stamp.id
                            && valid(&summary.lifecycle)
                            && included(*filter, summary.lifecycle.state)
                            && seen.insert(summary.conversation.id)
                    });
                    if checked {
                        let mut visible = Vec::new();
                        for summary in summaries {
                            // A pre-transition snapshot cannot restore older lifecycle metadata.
                            if self.observe_lifecycle(&summary.lifecycle) {
                                visible.push(summary.clone());
                            }
                        }
                        self.session_history.summaries = visible;
                        self.session_history.error = None;
                    } else {
                        self.session_history.error = Some(
                            "Session list reply did not match its exact lifecycle projection."
                                .into(),
                        );
                    }
                }
            }
            (
                SessionPending::Selected { id, navigation },
                AppEvent::ConversationLifecycle(lifecycle),
            ) => {
                if self.conversation == Some(*id)
                    && self.generation == *navigation
                    && lifecycle.stamp.id == *id
                {
                    self.observe_lifecycle(lifecycle);
                }
            }
            (
                SessionPending::Review {
                    id,
                    proposal,
                    generation,
                },
                AppEvent::ConversationLifecycle(lifecycle),
            ) => {
                if self.review_generation == *generation
                    && self.review.as_ref().is_some_and(|review| {
                        review.record.draft.id == *proposal
                            && review.record.draft.session_id == Some(*id)
                    })
                    && lifecycle.stamp.id == *id
                {
                    self.observe_lifecycle(lifecycle);
                }
            }
            (SessionPending::Change(capture), AppEvent::ConversationLifecycleChanged(result)) => {
                let owns_pending = self
                    .session_history
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.request == capture.request);
                let current = self.session_capture_current(capture);
                if owns_pending {
                    self.session_history.pending = None;
                }
                if current {
                    if operation == capture.request.operation_id
                        && Self::valid_session_result(capture, result)
                        && self.observe_lifecycle(&result.current)
                    {
                        self.session_history.error = None;
                        // Selection, filter and every owner buffer stay exactly where they were.
                        self.session_history.summaries.retain(|summary| {
                            summary.conversation.id != capture.conversation
                                || included(capture.filter, result.current.state)
                        });
                        for summary in &mut self.session_history.summaries {
                            if summary.conversation.id == capture.conversation {
                                summary.lifecycle = result.current;
                            }
                        }
                        commands.push(self.refresh_session_summaries());
                    } else {
                        self.session_history.error = Some("Archive/Restore reply did not match the captured operation and exact session version. Refresh history before retrying.".into());
                    }
                }
            }
            (_, AppEvent::Failed(error)) => {
                let current = match &pending {
                    SessionPending::Summaries { filter, generation } => {
                        *filter == self.session_history.filter
                            && *generation == self.session_history.list_generation
                    }
                    SessionPending::Selected { id, navigation } => {
                        self.conversation == Some(*id) && self.generation == *navigation
                    }
                    SessionPending::Review {
                        id,
                        proposal,
                        generation,
                    } => {
                        let current = self.review_generation == *generation
                            && self.review.as_ref().is_some_and(|review| {
                                review.record.draft.id == *proposal
                                    && review.record.draft.session_id == Some(*id)
                            });
                        if current
                            && error.kind == ErrorKind::NotFound
                            && !self.session_history.lifecycle.contains_key(id)
                        {
                            self.session_history.missing_legacy.insert(*id);
                        }
                        current && error.kind != ErrorKind::NotFound
                    }
                    SessionPending::Change(capture) => {
                        let current = self.session_capture_current(capture);
                        if self
                            .session_history
                            .pending
                            .as_ref()
                            .is_some_and(|pending| pending.request == capture.request)
                        {
                            self.session_history.pending = None;
                        }
                        current
                    }
                };
                if current {
                    if matches!(pending, SessionPending::Change(_)) {
                        self.session_history.pending = None;
                    }
                    self.session_history.error = Some(error.message.clone());
                }
            }
            _ => accepted = false,
        }
        if accepted {
            self.pending.remove(&operation);
        }
        Some(commands)
    }
}
