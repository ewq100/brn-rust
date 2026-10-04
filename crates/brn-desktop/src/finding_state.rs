//! Correlated tentative finding observations over the existing application lane.
use super::{AiState, Pending};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    findings::*,
    knowledge::{IdentityOutcome, NoteLinkOutcome},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct PageCapture {
    pub view: u64,
    pub page: u64,
}
#[derive(Clone)]
pub struct SelectionCapture {
    pub view: u64,
    pub page: u64,
    pub selection: u64,
    pub id: Uuid,
}
#[derive(Clone)]
struct CloseIntent {
    capture: SelectionCapture,
    record: FindingRecord,
    request: CloseFindingRequest,
}
pub struct FindingQueue {
    pub visible: bool,
    pub state: Option<FindingState>,
    pub page: Option<FindingPage>,
    pub selected: Option<FindingRecord>,
    pub inspection: Option<FindingInspection>,
    pub error: Option<String>,
    pub capture_result: Option<FindingRecord>,
    pub capture_error: Option<String>,
    pub close_result: Option<FindingRecord>,
    pub close_error: Option<String>,
    view: u64,
    page_generation: u64,
    selection: u64,
    inspection_generation: u64,
    selected_id: Option<Uuid>,
    before: Option<Uuid>,
    last_capture: Option<CaptureFindingRequest>,
    last_close: Option<CloseIntent>,
}
impl Default for FindingQueue {
    fn default() -> Self {
        Self {
            visible: false,
            state: Some(FindingState::Open),
            page: None,
            selected: None,
            inspection: None,
            error: None,
            capture_result: None,
            capture_error: None,
            close_result: None,
            close_error: None,
            view: 0,
            page_generation: 0,
            selection: 0,
            inspection_generation: 0,
            selected_id: None,
            before: None,
            last_capture: None,
            last_close: None,
        }
    }
}
impl AiState {
    pub fn retry_finding_capture(&mut self) -> Option<(Uuid, AppCommand)> {
        let request = self.finding_capture_retry_request()?.clone();
        self.finding_queue.capture_error = None;
        Some(self.command(
            Pending::FindingCapture(request.clone()),
            AppCommand::CaptureFinding(request),
        ))
    }
    pub fn retry_finding_close(&mut self) -> Option<(Uuid, AppCommand)> {
        self.finding_close_retry_request()?;
        let intent = self.finding_queue.last_close.clone()?;
        self.finding_queue.close_error = None;
        Some(self.command(
            Pending::FindingClose {
                capture: intent.capture,
                record: Box::new(intent.record),
                request: intent.request.clone(),
            },
            AppCommand::CloseFinding(intent.request),
        ))
    }
    pub fn finding_capture_retry_request(&self) -> Option<&CaptureFindingRequest> {
        if !self.ready
            || self.finding_queue.capture_error.is_none()
            || self.finding_capture_pending()
        {
            return None;
        }
        self.finding_queue.last_capture.as_ref()
    }
    pub fn finding_close_retry_request(&self) -> Option<&CloseFindingRequest> {
        if !self.ready || self.finding_queue.close_error.is_none() {
            return None;
        }
        let request = &self.finding_queue.last_close.as_ref()?.request;
        (!self.finding_closure_pending(request.expected.id)).then_some(request)
    }
    pub fn open_findings(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready {
            return None;
        }
        self.finding_queue.visible = true;
        self.finding_queue.view = self.finding_queue.view.wrapping_add(1);
        self.finding_queue.selection = self.finding_queue.selection.wrapping_add(1);
        self.finding_queue.selected_id = None;
        self.finding_queue.selected = None;
        self.finding_queue.inspection = None;
        self.refresh_findings(self.finding_queue.state, None)
    }
    pub fn close_findings(&mut self) {
        self.finding_queue.visible = false;
        self.finding_queue.view = self.finding_queue.view.wrapping_add(1);
    }
    pub fn refresh_findings(
        &mut self,
        state: Option<FindingState>,
        before: Option<Uuid>,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.finding_queue.visible || before.is_some_and(|id| id.is_nil()) {
            return None;
        }
        let queue = &mut self.finding_queue;
        if queue.state != state || queue.before != before {
            queue.selection = queue.selection.wrapping_add(1);
            queue.selected_id = None;
            queue.selected = None;
            queue.inspection = None;
            queue.page = None;
        }
        queue.state = state;
        queue.before = before;
        Some(self.finding_page_command())
    }
    pub fn select_finding(&mut self, id: Uuid) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.finding_queue.visible || id.is_nil() {
            return None;
        }
        let record = self.known_finding(id).cloned().map(Box::new);
        let queue = &mut self.finding_queue;
        queue.selection = queue.selection.wrapping_add(1);
        if queue.selected_id != Some(id) {
            queue.selected = None;
        }
        queue.selected_id = Some(id);
        queue.inspection = None;
        queue.error = None;
        let capture = self.finding_selection_capture(id);
        Some(self.command(
            Pending::Finding { capture, record },
            AppCommand::Finding(id),
        ))
    }
    pub fn inspect_selected_finding(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.finding_queue.visible || self.finding_loading() {
            return None;
        }
        let record = self.finding_queue.selected.clone()?;
        if self.finding_queue.selected_id != Some(record.draft.request.id) {
            return None;
        }
        self.finding_queue.inspection_generation =
            self.finding_queue.inspection_generation.wrapping_add(1);
        self.finding_queue.error = None;
        let capture = self.finding_selection_capture(record.draft.request.id);
        Some(self.command(
            Pending::FindingInspection {
                capture,
                inspection: self.finding_queue.inspection_generation,
                record: Box::new(record.clone()),
            },
            AppCommand::InspectFinding(record.draft.request.id),
        ))
    }
    pub fn close_selected_finding(&mut self, state: FindingState) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.finding_queue.visible || state == FindingState::Open {
            return None;
        }
        let record = self.finding_queue.selected.clone()?;
        let id = record.draft.request.id;
        if record.state != FindingState::Open
            || record.version != 1
            || self.finding_queue.selected_id != Some(id)
            || self.finding_closure_pending(id)
        {
            return None;
        }
        let request = CloseFindingRequest {
            expected: record.stamp(),
            state,
        };
        request.validate().ok()?;
        self.finding_queue.close_error = None;
        let capture = self.finding_selection_capture(id);
        self.finding_queue.last_close = Some(CloseIntent {
            capture: capture.clone(),
            record: record.clone(),
            request: request.clone(),
        });
        Some(self.command(
            Pending::FindingClose {
                capture,
                record: Box::new(record),
                request: request.clone(),
            },
            AppCommand::CloseFinding(request),
        ))
    }
    pub fn capture_link_finding(&mut self, index: usize) -> Option<(Uuid, AppCommand)> {
        let links = self.links.as_ref()?;
        if self.saved_document_path() != Some(links.source.path.as_str()) {
            return None;
        }
        let link = links.links.get(index)?;
        if matches!(
            link.outcome,
            NoteLinkOutcome::Resolved | NoteLinkOutcome::External | NoteLinkOutcome::NonNote
        ) {
            return None;
        }
        let start_byte = link.evidence.first()?.start_byte;
        let origin = FindingOrigin::UnresolvedLink {
            path: links.source.path.clone(),
            source_sha256: links.source.sha256,
            destination: link.destination.clone(),
            start_byte,
        };
        self.finding_capture_command(origin)
    }
    pub fn capture_identity_finding(&mut self) -> Option<(Uuid, AppCommand)> {
        let links = self.links.as_ref()?;
        if self.saved_document_path() != Some(links.source.path.as_str())
            || links.source_outcome != Some(IdentityOutcome::Ambiguous)
        {
            return None;
        }
        self.finding_capture_command(FindingOrigin::IdentityAmbiguity {
            note_id: links.source.note_id?,
        })
    }
    pub fn findings_loading(&self) -> bool {
        self.pending.values().any(|pending|matches!(pending,Pending::Findings {capture,request} if self.finding_page_matches(capture,request)))
    }
    pub fn finding_loading(&self) -> bool {
        self.pending.values().any(|pending| match pending {
            Pending::Finding { capture, .. } => self.finding_selection_matches(capture, true),
            Pending::FindingInspection {
                capture,
                inspection,
                ..
            } => {
                self.finding_selection_matches(capture, true)
                    && *inspection == self.finding_queue.inspection_generation
            }
            _ => false,
        })
    }
    pub fn finding_close_pending(&self) -> bool {
        self.pending
            .values()
            .any(|pending| matches!(pending, Pending::FindingClose { .. }))
    }
    pub fn finding_capture_pending(&self) -> bool {
        self.pending
            .values()
            .any(|pending| matches!(pending, Pending::FindingCapture(_)))
    }
    fn finding_closure_pending(&self, finding: Uuid) -> bool {
        self.pending.values().any(|pending| {
            matches!(pending,
            Pending::FindingClose { request, .. } if request.expected.id == finding)
        })
    }
    fn finding_capture_command(&mut self, origin: FindingOrigin) -> Option<(Uuid, AppCommand)> {
        if !self.ready
            || !self.vault_bound
            || self.application_busy()
            || self.finding_capture_pending()
        {
            return None;
        }
        let request = self
            .finding_queue
            .last_capture
            .as_ref()
            .filter(|request| request.origin == origin)
            .cloned()
            .unwrap_or_else(|| CaptureFindingRequest {
                id: Uuid::new_v4(),
                origin,
            });
        request.validate().ok()?;
        self.finding_queue.last_capture = Some(request.clone());
        self.finding_queue.capture_error = None;
        Some(self.command(
            Pending::FindingCapture(request.clone()),
            AppCommand::CaptureFinding(request),
        ))
    }
    fn finding_page_command(&mut self) -> (Uuid, AppCommand) {
        let queue = &mut self.finding_queue;
        queue.page_generation = queue.page_generation.wrapping_add(1);
        queue.error = None;
        let capture = PageCapture {
            view: queue.view,
            page: queue.page_generation,
        };
        let request = FindingListRequest {
            state: queue.state,
            limit: 25,
            before: queue.before,
        };
        self.command(
            Pending::Findings {
                capture,
                request: request.clone(),
            },
            AppCommand::Findings(request),
        )
    }
    fn finding_page_matches(&self, capture: &PageCapture, request: &FindingListRequest) -> bool {
        let queue = &self.finding_queue;
        queue.visible
            && queue.view == capture.view
            && queue.page_generation == capture.page
            && queue.state == request.state
            && queue.before == request.before
            && request.limit == 25
    }
    fn finding_selection_capture(&self, id: Uuid) -> SelectionCapture {
        let queue = &self.finding_queue;
        SelectionCapture {
            view: queue.view,
            page: queue.page_generation,
            selection: queue.selection,
            id,
        }
    }
    fn finding_selection_matches(&self, capture: &SelectionCapture, page: bool) -> bool {
        let queue = &self.finding_queue;
        queue.visible
            && queue.view == capture.view
            && (!page || queue.page_generation == capture.page)
            && queue.selection == capture.selection
            && queue.selected_id == Some(capture.id)
    }
    fn known_finding(&self, id: Uuid) -> Option<&FindingRecord> {
        let queue = &self.finding_queue;
        [
            queue.selected.as_ref(),
            queue.close_result.as_ref(),
            queue.capture_result.as_ref(),
        ]
        .into_iter()
        .flatten()
        .chain(queue.page.iter().flat_map(|page| &page.entries))
        .filter(|record| record.draft.request.id == id)
        .max_by_key(|record| (record.version, record.updated_at_ms))
    }
    fn refresh_visible_findings(&mut self) -> Vec<(Uuid, AppCommand)> {
        if self.ready && self.finding_queue.visible {
            vec![self.finding_page_command()]
        } else {
            Vec::new()
        }
    }
    pub(super) fn received_findings(
        &mut self,
        id: Uuid,
        event: &AppEvent,
    ) -> Option<Vec<(Uuid, AppCommand)>> {
        let pending = self.pending.get(&id)?.clone();
        let current = match &pending {
            Pending::Findings { capture, request } => self.finding_page_matches(capture, request),
            Pending::Finding { capture, .. } => self.finding_selection_matches(capture, true),
            Pending::FindingInspection {
                capture,
                inspection,
                ..
            } => {
                self.finding_selection_matches(capture, true)
                    && *inspection == self.finding_queue.inspection_generation
            }
            Pending::FindingCapture(_) | Pending::FindingClose { .. } => true,
            _ => return None,
        };
        if !current {
            self.pending.remove(&id);
            return Some(Vec::new());
        }
        let mut commands = Vec::new();
        match (&pending, event) {
            (Pending::Findings { request, .. }, AppEvent::Findings(page))
                if page_valid(page, request)
                    && page.entries.iter().all(|record| {
                        self.known_finding(record.draft.request.id)
                            .is_none_or(|previous| can_advance(previous, record))
                    }) =>
            {
                self.finding_queue.page = Some((**page).clone());
                self.finding_queue.error = None;
            }
            (
                Pending::Finding {
                    capture,
                    record: previous,
                },
                AppEvent::Finding(record),
            ) if record.draft.request.id == capture.id
                && record_valid(record)
                && previous
                    .as_ref()
                    .is_none_or(|previous| can_advance(previous, record)) =>
            {
                if self
                    .finding_queue
                    .selected
                    .as_ref()
                    .is_some_and(|previous| !can_advance(previous, record))
                {
                    return Some(Vec::new());
                }
                self.finding_queue.selected = Some((**record).clone());
                self.finding_queue.error = None;
            }
            (
                Pending::FindingInspection {
                    record: previous, ..
                },
                AppEvent::FindingInspection(inspection),
            ) if inspection_valid(inspection, previous) => {
                if self
                    .finding_queue
                    .selected
                    .as_ref()
                    .is_some_and(|current| !can_advance(current, &inspection.record))
                {
                    self.pending.remove(&id);
                    return Some(Vec::new());
                }
                self.finding_queue.selected = Some(inspection.record.clone());
                self.finding_queue.inspection = Some((**inspection).clone());
                self.finding_queue.error = None;
            }
            (Pending::FindingCapture(request), AppEvent::Finding(record))
                if record.draft.request == *request && record_valid(record) =>
            {
                self.finding_queue.capture_result = Some((**record).clone());
                self.finding_queue.capture_error = None;
                self.notice = format!(
                    "Finding {} retained as {:?}. Knowledge is unchanged.",
                    request.id, record.state
                );
                commands = self.refresh_visible_findings();
            }
            (
                Pending::FindingClose {
                    capture,
                    record: previous,
                    request,
                },
                AppEvent::Finding(record),
            ) if can_advance(previous, record)
                && record.version == 2
                && record.state == request.state =>
            {
                self.finding_queue.close_result = Some((**record).clone());
                self.finding_queue.close_error = None;
                if self.finding_selection_matches(capture, false)
                    && self.finding_queue.selected.as_ref() == Some(previous.as_ref())
                {
                    self.finding_queue.selected = Some((**record).clone());
                    self.finding_queue.inspection = None;
                }
                self.notice = format!(
                    "Finding {} {:?}; queue state changed. Knowledge is unchanged.",
                    request.expected.id, record.state
                );
                commands = self.refresh_visible_findings();
            }
            (Pending::FindingCapture(request), AppEvent::Failed(error)) => {
                self.finding_queue.last_capture = Some(request.clone());
                self.finding_queue.capture_error = Some(error.message.clone());
                self.notice = format!(
                    "Finding capture failed: {}. Retained input may be retried explicitly.",
                    error.message
                );
            }
            (
                Pending::FindingClose {
                    capture,
                    record,
                    request,
                },
                AppEvent::Failed(error),
            ) => {
                self.finding_queue.last_close = Some(CloseIntent {
                    capture: capture.clone(),
                    record: (**record).clone(),
                    request: request.clone(),
                });
                self.finding_queue.close_error = Some(error.message.clone());
                self.notice = format!(
                    "Finding closure failed: {}. Retained review may be retried explicitly.",
                    error.message
                );
            }
            (_, AppEvent::Failed(error)) => self.finding_queue.error = Some(error.message.clone()),
            _ => return Some(Vec::new()),
        }
        self.pending.remove(&id);
        Some(commands)
    }
}

fn record_valid(record: &FindingRecord) -> bool {
    record.draft.validate().is_ok()
        && record.created_at_ms <= record.updated_at_ms
        && record.updated_at_ms <= i64::MAX as u64
        && match (record.version, record.state) {
            (1, FindingState::Open) => record.created_at_ms == record.updated_at_ms,
            (2, FindingState::Resolved | FindingState::Dismissed) => true,
            _ => false,
        }
}
fn can_advance(previous: &FindingRecord, record: &FindingRecord) -> bool {
    record_valid(record)
        && previous.draft == record.draft
        && previous.created_at_ms == record.created_at_ms
        && record.updated_at_ms >= previous.updated_at_ms
        && if previous.version == 1 {
            record.version >= previous.version
        } else {
            previous == record
        }
}
fn page_valid(page: &FindingPage, request: &FindingListRequest) -> bool {
    let mut ids = std::collections::HashSet::new();
    page.entries.len() <= request.limit
        && page.entries.iter().all(|record| {
            record_valid(record)
                && request.state.is_none_or(|state| state == record.state)
                && Some(record.draft.request.id) != request.before
                && ids.insert(record.draft.request.id)
        })
        && page.open_count
            >= page
                .entries
                .iter()
                .filter(|record| record.state == FindingState::Open)
                .count()
        && page.next_before.is_none_or(|id| {
            page.entries.len() == request.limit
                && page
                    .entries
                    .last()
                    .is_some_and(|record| record.draft.request.id == id)
        })
}
fn inspection_valid(inspection: &FindingInspection, previous: &FindingRecord) -> bool {
    can_advance(previous, &inspection.record)
        && inspection.evidence.len() == inspection.record.draft.evidence.len()
        && inspection
            .evidence
            .iter()
            .enumerate()
            .all(|(index, observation)| {
                let retained = &inspection.record.draft.evidence[index].source;
                observation.index == index
                    && match observation.outcome {
                        FindingEvidenceOutcome::Unchanged => {
                            observation.observed.as_ref() == Some(retained)
                                && observation.reason.is_none()
                        }
                        FindingEvidenceOutcome::Changed => {
                            observation.observed.as_ref().is_some_and(|source| {
                                source.path == retained.path
                                    && source != retained
                                    && source.fingerprint.len <= brn_workflow::MAX_NOTE_BYTES as u64
                            }) && observation.reason.is_none()
                        }
                        FindingEvidenceOutcome::Unavailable => {
                            observation.observed.is_none() && observation.reason.is_some()
                        }
                    }
            })
}
