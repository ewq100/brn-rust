//! Inbox copy presentation and correlation. Workflow owns qualification/effects.
use super::{AiState, Pending, inbox_state::InboxViewCapture};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    inbox::{InboxItem, InboxOriginal},
    inbox_original_operations::{
        InboxOriginalOperation, InboxOriginalOperationKind as Kind, InboxOriginalOperationSummary,
        InboxOriginalParent, InboxRemovalConfirmation, MAX_ORIGINAL_OPERATIONS,
        RemoveInboxOriginalRequest, RestoreInboxOriginalRequest,
    },
    inbox_removal::InboxRemovalPreview,
};
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq)]
pub enum InboxCopyRequest {
    Remove(RemoveInboxOriginalRequest),
    Restore(RestoreInboxOriginalRequest),
}
impl InboxCopyRequest {
    fn id(&self) -> Uuid {
        match self {
            Self::Remove(r) => r.operation_id,
            Self::Restore(r) => r.operation_id,
        }
    }
    fn command(&self) -> AppCommand {
        match self {
            Self::Remove(r) => AppCommand::RemoveInboxOriginal(r.clone()),
            Self::Restore(r) => AppCommand::RestoreInboxOriginal(r.clone()),
        }
    }
    fn matches(&self, record: &InboxOriginalOperation) -> bool {
        match (self, record) {
            (Self::Remove(r), InboxOriginalOperation::Remove(actual)) => r == &actual.request,
            (Self::Restore(r), InboxOriginalOperation::Restore(actual)) => r == &actual.request,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct CopyRead {
    view: InboxViewCapture,
    generation: u64,
    item: InboxItem,
}
#[derive(Clone)]
pub struct CopyEffect {
    view: InboxViewCapture,
    item: InboxItem,
    request: InboxCopyRequest,
}
#[derive(Clone)]
pub enum CopyPending {
    Preview(CopyRead),
    History(CopyRead),
    Operation {
        read: CopyRead,
        summary: InboxOriginalOperationSummary,
        generation: u64,
    },
    Effect(CopyEffect),
}
#[derive(Default)]
pub struct InboxCopyView {
    pub preview: Option<Box<InboxRemovalPreview>>,
    pub history: Option<Vec<InboxOriginalOperationSummary>>,
    pub removal: Option<Box<InboxOriginalOperation>>,
    pub operation: Option<Box<InboxOriginalOperation>>,
    pub receipt: Option<Box<InboxOriginalOperation>>,
    pub error: Option<String>,
    pub message: Option<String>,
    generation: u64,
    operation_generation: u64,
    last_effect: Option<CopyEffect>,
    failed: bool,
}

/// One immutable modal capture. Opening/cancelling it admits no effect.
#[derive(Clone)]
pub struct InboxCopyConfirmation {
    read: CopyRead,
    request: InboxCopyRequest,
    preview: Option<Box<InboxRemovalPreview>>,
    removal: Option<Box<InboxOriginalOperation>>,
}
impl InboxCopyConfirmation {
    pub fn item(&self) -> &InboxItem {
        &self.read.item
    }
    pub fn request(&self) -> &InboxCopyRequest {
        &self.request
    }
    pub fn preview(&self) -> Option<&InboxRemovalPreview> {
        self.preview.as_deref()
    }
    pub fn removal(&self) -> Option<&InboxOriginalOperation> {
        self.removal.as_deref()
    }
    pub fn is_remove(&self) -> bool {
        matches!(self.request, InboxCopyRequest::Remove(_))
    }
}

impl AiState {
    pub(super) fn invalidate_inbox_copy(&mut self) {
        let copy = &mut self.inbox_copy;
        copy.generation = copy.generation.wrapping_add(1);
        copy.preview = None;
        copy.history = None;
        copy.removal = None;
        copy.operation = None;
        copy.error = None;
        // Keep identified admitted outcomes/requests across navigation.
    }
    fn copy_read(&self) -> Option<CopyRead> {
        if !self.ready || !self.inbox_queue.visible {
            return None;
        }
        Some(CopyRead {
            view: self.inbox_view_capture(),
            generation: self.inbox_copy.generation,
            item: self.inbox_queue.selected.as_ref()?.item.clone(),
        })
    }
    fn copy_read_current(&self, read: &CopyRead) -> bool {
        self.ready
            && self.inbox_view_current(&read.view)
            && read.generation == self.inbox_copy.generation
            && self
                .inbox_queue
                .selected
                .as_ref()
                .is_some_and(|r| r.item == read.item)
    }
    pub fn inbox_copy_pending(&self) -> bool {
        self.pending.values().any(|p| {
            matches!(p,
            Pending::InboxCopy(p) if matches!(p.as_ref(), CopyPending::Effect(_)))
        })
    }
    pub fn inbox_copy_loading(&self) -> bool {
        self.pending.values().any(|p| match p {
            Pending::InboxCopy(p) => match p.as_ref() {
                CopyPending::Preview(r) | CopyPending::History(r) => self.copy_read_current(r),
                CopyPending::Operation {
                    read, generation, ..
                } => {
                    self.copy_read_current(read)
                        && *generation == self.inbox_copy.operation_generation
                }
                CopyPending::Effect(_) => false,
            },
            _ => false,
        })
    }
    pub fn inspect_inbox_copy(&mut self) -> Vec<(Uuid, AppCommand)> {
        if self.copy_read().is_none() || self.inbox_copy_pending() {
            return vec![];
        }
        self.invalidate_inbox_copy();
        let read = self.copy_read().expect("selected visible copy");
        let available = matches!(
            self.inbox_queue.selected.as_ref().unwrap().original,
            InboxOriginal::Available { .. }
        );
        let mut commands = vec![self.command(
            Pending::InboxCopy(Box::new(CopyPending::History(read.clone()))),
            AppCommand::InboxOriginalOperations(read.item.capture.id),
        )];
        if available {
            commands.push(self.command(
                Pending::InboxCopy(Box::new(CopyPending::Preview(read.clone()))),
                AppCommand::PreviewInboxRemoval(read.item.capture.id),
            ));
        }
        commands
    }
    pub fn inspect_inbox_copy_operation(&mut self, operation: Uuid) -> Option<(Uuid, AppCommand)> {
        if self.inbox_copy_pending() {
            return None;
        }
        let read = self.copy_read()?;
        let summary = self
            .inbox_copy
            .history
            .as_ref()?
            .iter()
            .find(|s| s.operation_id == operation)?
            .clone();
        self.inbox_copy.operation = None;
        self.inbox_copy.operation_generation = self.inbox_copy.operation_generation.wrapping_add(1);
        let generation = self.inbox_copy.operation_generation;
        let command = match summary.kind {
            Kind::Remove => AppCommand::InboxOriginalRemoval(operation),
            Kind::Restore => AppCommand::InboxOriginalRestore(operation),
        };
        Some(self.command(
            Pending::InboxCopy(Box::new(CopyPending::Operation {
                read,
                summary,
                generation,
            })),
            command,
        ))
    }
    fn copy_ready(&self) -> bool {
        self.copy_read().is_some()
            && !self.inbox_copy_loading()
            && !self.inbox_copy_pending()
            && self.inbox_copy.error.is_none()
            && self.inbox_copy.history.is_some()
    }
    pub fn can_remove_inbox_copy(&self) -> bool {
        if !self.copy_ready() {
            return false;
        }
        let selected = self.inbox_queue.selected.as_ref().unwrap();
        let Some(preview) = self.inbox_copy.preview.as_ref() else {
            return false;
        };
        matches!(&selected.original, InboxOriginal::Available { .. })
            && preview.evidence.item == selected.item
            && preview.evidence.original == selected.original
            && preview.evidence.blockers.is_empty()
            && preview.evidence.digest().ok() == Some(preview.digest)
            && self
                .inbox_copy
                .history
                .as_ref()
                .unwrap()
                .last()
                .is_none_or(|s| s.kind == Kind::Restore && s.settled_at_ms.is_some())
    }
    pub fn can_restore_inbox_copy(&self) -> bool {
        if !self.copy_ready() {
            return false;
        }
        let selected = self.inbox_queue.selected.as_ref().unwrap();
        let InboxOriginal::RemovedRetained { operation_id } = selected.original else {
            return false;
        };
        let Some(record) = self.inbox_copy.removal.as_ref() else {
            return false;
        };
        let Ok(summary) = record.summary() else {
            return false;
        };
        record.original() == &selected.item
            && summary.operation_id == operation_id
            && summary.kind == Kind::Remove
            && summary.settled_at_ms.is_some()
            && self.inbox_copy.history.as_ref().unwrap().last() == Some(&summary)
    }
    pub fn capture_inbox_copy_removal(&mut self) -> Option<InboxCopyConfirmation> {
        if !self.can_remove_inbox_copy() {
            return None;
        }
        let read = self.copy_read()?;
        let preview = self.inbox_copy.preview.clone()?;
        let previous_restore =
            self.inbox_copy
                .history
                .as_ref()?
                .last()
                .map(|s| InboxOriginalParent {
                    operation_id: s.operation_id,
                    record_sha256: s.record_sha256,
                });
        Some(InboxCopyConfirmation {
            request: InboxCopyRequest::Remove(RemoveInboxOriginalRequest {
                operation_id: Uuid::new_v4(),
                item_id: read.item.capture.id,
                preview_digest: preview.digest,
                previous_restore,
                confirmation: InboxRemovalConfirmation {
                    version: 1,
                    exact_copy_removal_intended: true,
                },
            }),
            read,
            preview: Some(preview),
            removal: None,
        })
    }
    pub fn capture_inbox_copy_restore(&mut self) -> Option<InboxCopyConfirmation> {
        if !self.can_restore_inbox_copy() {
            return None;
        }
        let removal = self.inbox_copy.removal.clone()?;
        let summary = removal.summary().ok()?;
        Some(InboxCopyConfirmation {
            read: self.copy_read()?,
            request: InboxCopyRequest::Restore(RestoreInboxOriginalRequest {
                operation_id: Uuid::new_v4(),
                removal_operation_id: summary.operation_id,
                removal_digest: summary.record_sha256,
            }),
            preview: None,
            removal: Some(removal),
        })
    }
    pub fn confirm_inbox_copy(
        &mut self,
        capture: &InboxCopyConfirmation,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.copy_read_current(&capture.read)
            || self.pending.contains_key(&capture.request.id())
        {
            return None;
        }
        let valid =
            match &capture.request {
                InboxCopyRequest::Remove(request) => {
                    self.can_remove_inbox_copy()
                        && request.item_id == capture.read.item.capture.id
                        && request.validate().is_ok()
                        && capture.preview.as_ref().is_some_and(|p| {
                            p.evidence.digest().ok() == Some(request.preview_digest)
                                && self
                                    .inbox_copy
                                    .preview
                                    .as_ref()
                                    .is_some_and(|current| current.digest == p.digest)
                        })
                        && request.previous_restore
                            == self.inbox_copy.history.as_ref()?.last().map(|s| {
                                InboxOriginalParent {
                                    operation_id: s.operation_id,
                                    record_sha256: s.record_sha256,
                                }
                            })
                }
                InboxCopyRequest::Restore(request) => {
                    self.can_restore_inbox_copy()
                        && request.validate().is_ok()
                        && capture.removal.as_ref().is_some_and(|r| {
                            r.summary().is_ok_and(|s| {
                                s.operation_id == request.removal_operation_id
                                    && s.record_sha256 == request.removal_digest
                            }) && self.inbox_copy.removal.as_ref().is_some_and(|current| {
                                current.digest().ok() == Some(request.removal_digest)
                            })
                        })
                }
            };
        if !valid {
            return None;
        }
        self.submit_copy_effect(CopyEffect {
            view: capture.read.view.clone(),
            item: capture.read.item.clone(),
            request: capture.request.clone(),
        })
    }
    fn submit_copy_effect(&mut self, effect: CopyEffect) -> Option<(Uuid, AppCommand)> {
        if !self.ready
            || self.inbox_copy_pending()
            || self.pending.contains_key(&effect.request.id())
        {
            return None;
        }
        let id = effect.request.id();
        let command = effect.request.command();
        self.inbox_copy.last_effect = Some(effect.clone());
        self.inbox_copy.failed = false;
        self.inbox_copy.error = None;
        self.inbox_copy.message = Some(format!(
            "Original-copy operation {id} admitted; outcome unconfirmed."
        ));
        self.pending.insert(
            id,
            Pending::InboxCopy(Box::new(CopyPending::Effect(effect))),
        );
        Some((id, command))
    }
    pub fn can_retry_inbox_copy(&self) -> bool {
        self.ready
            && !self.inbox_copy_pending()
            && ((self.inbox_copy.failed && self.inbox_copy.last_effect.is_some())
                || self.recorded_pending_effect().is_some())
    }
    pub fn retry_inbox_copy(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.can_retry_inbox_copy() {
            return None;
        }
        let effect = if self.inbox_copy.failed {
            self.inbox_copy.last_effect.clone()?
        } else {
            self.recorded_pending_effect()?
        };
        self.submit_copy_effect(effect)
    }
    fn recorded_pending_effect(&self) -> Option<CopyEffect> {
        if self.inbox_copy_loading() || self.inbox_copy.error.is_some() {
            return None;
        }
        let read = self.copy_read()?;
        let record = self.inbox_copy.operation.as_ref()?;
        let summary = record.summary().ok()?;
        if summary.settled_at_ms.is_some()
            || record.original() != &read.item
            || self.inbox_copy.history.as_ref()?.last() != Some(&summary)
        {
            return None;
        }
        let request = match record.as_ref() {
            InboxOriginalOperation::Remove(r) => InboxCopyRequest::Remove(r.request.clone()),
            InboxOriginalOperation::Restore(r) => InboxCopyRequest::Restore(r.request.clone()),
            _ => return None, // Legacy intents keep their established reader/CLI path.
        };
        if !request.matches(record) {
            return None;
        }
        Some(CopyEffect {
            view: read.view,
            item: read.item,
            request,
        })
    }
    pub(super) fn received_inbox_copy(
        &mut self,
        id: Uuid,
        event: &AppEvent,
    ) -> Option<Vec<(Uuid, AppCommand)>> {
        let Some(Pending::InboxCopy(pending)) = self.pending.get(&id).cloned() else {
            return matches!(
                event,
                AppEvent::InboxRemovalPreview(_)
                    | AppEvent::InboxOriginalRemoved(_)
                    | AppEvent::InboxOriginalRestored(_)
                    | AppEvent::InboxOriginalRemoval { .. }
                    | AppEvent::InboxOriginalRestore { .. }
                    | AppEvent::InboxOriginalOperations { .. }
                    | AppEvent::ArchivedInboxAnalysis { .. }
            )
            .then(Vec::new);
        };
        let read = match pending.as_ref() {
            CopyPending::Preview(r)
            | CopyPending::History(r)
            | CopyPending::Operation { read: r, .. } => Some(r),
            CopyPending::Effect(_) => None,
        };
        if read.is_some_and(|r| !self.copy_read_current(r))
            || matches!(pending.as_ref(), CopyPending::Operation { generation, .. }
                if *generation != self.inbox_copy.operation_generation)
        {
            self.pending.remove(&id);
            return Some(vec![]);
        }
        let mut commands = vec![];
        match (pending.as_ref(), event) {
            (CopyPending::Preview(read), AppEvent::InboxRemovalPreview(preview))
                if preview.evidence.item == read.item
                    && preview.evidence.digest().ok() == Some(preview.digest)
                    && self
                        .inbox_queue
                        .selected
                        .as_ref()
                        .is_some_and(|r| r.original == preview.evidence.original) =>
            {
                self.inbox_copy.preview = Some(preview.clone());
            }
            (
                CopyPending::History(read),
                AppEvent::InboxOriginalOperations {
                    item_id,
                    operations,
                },
            ) if *item_id == read.item.capture.id && history_valid(operations, *item_id) => {
                self.inbox_copy.history = Some(operations.clone());
                if let Some(head) = operations.last()
                    && let Some(command) = self.inspect_inbox_copy_operation(head.operation_id)
                {
                    commands.push(command);
                }
            }
            (
                CopyPending::Operation { read, summary, .. },
                AppEvent::InboxOriginalRemoval {
                    operation_id,
                    record,
                }
                | AppEvent::InboxOriginalRestore {
                    operation_id,
                    record,
                },
            ) if summary.operation_id == *operation_id
                && event_kind_matches(summary.kind, event)
                && record.as_ref().is_some_and(|r| {
                    r.original() == &read.item && r.summary().ok().as_ref() == Some(summary)
                }) =>
            {
                self.inbox_copy.operation = record.clone();
                if summary.kind == Kind::Remove
                    && self.inbox_queue.selected.as_ref().is_some_and(|selected| {
                        matches!(selected.original, InboxOriginal::RemovedRetained { operation_id }
                            if operation_id == summary.operation_id)
                    })
                {
                    self.inbox_copy.removal = record.clone();
                }
            }
            (
                CopyPending::Operation { summary, .. },
                AppEvent::InboxOriginalRemoval {
                    operation_id,
                    record: None,
                }
                | AppEvent::InboxOriginalRestore {
                    operation_id,
                    record: None,
                },
            ) if summary.operation_id == *operation_id
                && event_kind_matches(summary.kind, event) =>
            {
                self.inbox_copy.error =
                    Some("The identified retained original-copy record is unavailable.".into());
            }
            (CopyPending::Effect(effect), AppEvent::InboxOriginalRemoved(record))
                if matches!(&effect.request, InboxCopyRequest::Remove(r) if r == &record.request)
                    && record.removed_at_ms.is_some()
                    && record.validate().is_ok()
                    && record.evidence.item == effect.item =>
            {
                self.inbox_copy.receipt =
                    Some(Box::new(InboxOriginalOperation::Remove(record.clone())));
                self.copy_effect_settled(effect, "removed; exact copy retained", &mut commands);
            }
            (CopyPending::Effect(effect), AppEvent::InboxOriginalRestored(record))
                if matches!(&effect.request, InboxCopyRequest::Restore(r) if r == &record.request)
                    && record.restored_at_ms.is_some()
                    && record.validate().is_ok()
                    && record.original == effect.item =>
            {
                self.inbox_copy.receipt =
                    Some(Box::new(InboxOriginalOperation::Restore(record.clone())));
                self.copy_effect_settled(effect, "restored", &mut commands);
            }
            (CopyPending::Effect(_), AppEvent::Failed(error)) => {
                self.inbox_copy.error = Some(error.message.clone());
                self.inbox_copy.failed = true;
                self.inbox_copy.message = Some(format!(
                    "Original-copy operation {id} failed or is unconfirmed. Inspect its recorded outcome; retry preserves the exact request."
                ));
            }
            (_, AppEvent::Failed(error)) => self.inbox_copy.error = Some(error.message.clone()),
            _ => return Some(vec![]),
        }
        self.pending.remove(&id);
        Some(commands)
    }
    fn copy_effect_settled(
        &mut self,
        effect: &CopyEffect,
        status: &str,
        commands: &mut Vec<(Uuid, AppCommand)>,
    ) {
        self.inbox_copy.failed = false;
        self.inbox_copy.error = None;
        let message = format!(
            "Original-copy operation {} {status}. Inbox disposition and semantic review remain separate.",
            effect.request.id()
        );
        self.inbox_copy.message = Some(message.clone());
        self.notice = message;
        if self.inbox_view_current(&effect.view)
            && self
                .inbox_queue
                .selected
                .as_ref()
                .is_some_and(|r| r.item == effect.item)
            && let Some(command) = self.select_inbox(effect.item.capture.id)
        {
            commands.push(command);
        }
    }
}

fn event_kind_matches(kind: Kind, event: &AppEvent) -> bool {
    matches!(
        (kind, event),
        (Kind::Remove, AppEvent::InboxOriginalRemoval { .. })
            | (Kind::Restore, AppEvent::InboxOriginalRestore { .. })
    )
}

fn history_valid(history: &[InboxOriginalOperationSummary], item: Uuid) -> bool {
    let mut previous: Option<&InboxOriginalOperationSummary> = None;
    let mut ids = std::collections::HashSet::new();
    history.len() <= MAX_ORIGINAL_OPERATIONS
        && history.iter().all(|s| {
            if s.validate().is_err() || s.item_id != item || !ids.insert(s.operation_id) {
                return false;
            }
            let valid = match previous {
                None => s.kind == Kind::Remove && s.parent.is_none(),
                Some(p) => {
                    s.parent == Some(p.operation_id)
                        && s.kind != p.kind
                        && p.settled_at_ms.is_some_and(|t| t <= s.prepared_at_ms)
                }
            };
            previous = Some(s);
            valid
        })
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    include!("inbox_copy_state_tests.rs");
}
