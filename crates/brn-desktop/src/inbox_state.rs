//! Inbox presentation and exact worker response correlation only.
use super::{AiState, Pending};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    inbox::{
        CaptureBinaryInboxRequest, CaptureInboxRequest, InboxInventory, InboxItem,
        InboxListRequest, InboxRead,
    },
    inbox_processing::{
        InboxCandidateRequest, InboxConversionFormat, InboxConversionPreview, InboxProcessBatch,
        InboxProcessOutcome, InboxSourceBinding, InboxSourceRequest, ProcessInboxRequest,
    },
    proposals::DraftRequest,
};
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq)]
pub struct InboxViewCapture {
    view: u64,
    page: u64,
    selection: u64,
}
#[derive(Clone)]
pub struct ConversionCapture {
    request: InboxCandidateRequest,
    original: InboxItem,
    format: InboxConversionFormat,
    byte_len: u64,
    sha256: [u8; 32],
    visual: Option<brn_workflow::inbox_processing::InboxVisualPreview>,
    extraction: Option<brn_intake::Extraction>,
}
#[derive(Clone)]
pub enum InboxPending {
    Page {
        view: InboxViewCapture,
        request: InboxListRequest,
    },
    Read {
        view: InboxViewCapture,
        id: Uuid,
        expected: Option<InboxItem>,
    },
    Capture(CaptureInboxRequest),
    CaptureBinary(CaptureBinaryInboxRequest),
    Process(ProcessInboxRequest),
    Cancel(ProcessInboxRequest),
    Preview {
        view: InboxViewCapture,
        generation: u64,
        conversion: ConversionCapture,
        batch: Box<InboxProcessBatch>,
    },
    Source {
        view: InboxViewCapture,
        generation: u64,
        request: InboxSourceRequest,
        conversion: ConversionCapture,
    },
}
#[derive(Default)]
pub struct InboxQueue {
    pub visible: bool,
    pub page: Option<InboxInventory>,
    pub selected: Option<InboxRead>,
    pub batch: Option<InboxProcessBatch>,
    pub preview: Option<InboxConversionPreview>,
    pub error: Option<String>,
    pub capture_result: Option<InboxItem>,
    pub capture_error: Option<String>,
    pub source_error: Option<String>,
    /// Full prepared input remains available when another unfinished form is open.
    pub prepared: Option<DraftRequest>,
    view: u64,
    page_generation: u64,
    selection: u64,
    selected_id: Option<Uuid>,
    preview_generation: u64,
    source_generation: u64,
    last_capture: Option<CaptureInboxRequest>,
    last_binary_capture: Option<CaptureBinaryInboxRequest>,
    last_process: Option<ProcessInboxRequest>,
    process_failed: bool,
}

impl AiState {
    pub fn open_inbox(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready {
            return None;
        }
        self.inbox_queue.visible = true;
        self.invalidate_inbox_copy();
        self.open_analysis_view();
        self.inbox_queue.view = self.inbox_queue.view.wrapping_add(1);
        self.refresh_inbox()
    }
    pub fn close_inbox(&mut self) {
        self.inbox_queue.visible = false;
        self.invalidate_inbox_copy();
        self.close_analysis_view();
        self.inbox_queue.view = self.inbox_queue.view.wrapping_add(1);
    }
    pub fn refresh_inbox(&mut self) -> Option<(Uuid, AppCommand)> {
        self.inbox_page_command(None)
    }
    pub fn next_inbox_page(&mut self) -> Option<(Uuid, AppCommand)> {
        let after = self.inbox_queue.page.as_ref()?.next_after?;
        self.inbox_page_command(Some(after))
    }
    fn inbox_page_command(&mut self, after: Option<Uuid>) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.inbox_queue.visible {
            return None;
        }
        self.invalidate_inbox_copy();
        let queue = &mut self.inbox_queue;
        queue.page_generation = queue.page_generation.wrapping_add(1);
        queue.selection = queue.selection.wrapping_add(1);
        queue.selected_id = None;
        queue.selected = None;
        queue.preview = None;
        queue.error = None;
        let request = InboxListRequest { limit: 25, after };
        let view = self.inbox_view_capture();
        Some(self.command(
            Pending::Inbox(Box::new(InboxPending::Page {
                view,
                request: request.clone(),
            })),
            AppCommand::InboxItems(request),
        ))
    }
    pub fn inbox_loading(&self) -> bool {
        self.pending.values().any(|pending| match pending {
            Pending::Inbox(pending) => match pending.as_ref() {
                InboxPending::Page { view, .. } => self.inbox_page_current(view),
                InboxPending::Read { view, .. } => self.inbox_view_current(view),
                _ => false,
            },
            _ => false,
        })
    }
    pub fn select_inbox(&mut self, id: Uuid) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.inbox_queue.visible || id.is_nil() {
            self.inbox_queue.error = Some("Choose an identified Inbox item.".into());
            return None;
        }
        let expected = self.known_inbox_item(id).cloned();
        self.invalidate_inbox_copy();
        let queue = &mut self.inbox_queue;
        queue.selection = queue.selection.wrapping_add(1);
        queue.selected_id = Some(id);
        queue.selected = None;
        queue.preview = None;
        queue.error = None;
        let view = self.inbox_view_capture();
        Some(self.command(
            Pending::Inbox(Box::new(InboxPending::Read { view, id, expected })),
            AppCommand::InboxItem(id),
        ))
    }
    pub fn capture_pending(&self) -> bool {
        self.pending.values().any(|pending| {
            matches!(pending,
            Pending::Inbox(pending) if matches!(pending.as_ref(), InboxPending::Capture(_) | InboxPending::CaptureBinary(_)))
        })
    }
    pub fn capture_inbox(&mut self, request: CaptureInboxRequest) -> Option<(Uuid, AppCommand)> {
        if let Err(error) = request.validate() {
            self.inbox_queue.capture_error = Some(error.message);
            return None;
        }
        if !self.ready || self.capture_pending() {
            self.inbox_queue.capture_error =
                Some("Wait for the admitted Inbox capture to settle.".into());
            return None;
        }
        self.inbox_queue.last_capture = Some(request.clone());
        self.inbox_queue.last_binary_capture = None;
        self.inbox_queue.capture_error = None;
        let id = request.id;
        if self.pending.contains_key(&id) {
            self.inbox_queue.capture_error = Some("This operation UUID is already pending.".into());
            return None;
        }
        self.pending.insert(
            id,
            Pending::Inbox(Box::new(InboxPending::Capture(request.clone()))),
        );
        Some((id, AppCommand::CaptureInbox(request)))
    }
    pub fn capture_binary_inbox(
        &mut self,
        request: CaptureBinaryInboxRequest,
    ) -> Option<(Uuid, AppCommand)> {
        if let Err(error) = request.validate() {
            self.inbox_queue.capture_error = Some(error.message);
            return None;
        }
        if !self.ready || self.capture_pending() || self.pending.contains_key(&request.id) {
            self.inbox_queue.capture_error =
                Some("Wait for the admitted Inbox capture to settle.".into());
            return None;
        }
        self.inbox_queue.last_binary_capture = Some(request.clone());
        self.inbox_queue.last_capture = None;
        self.inbox_queue.capture_error = None;
        let id = request.id;
        self.pending.insert(
            id,
            Pending::Inbox(Box::new(InboxPending::CaptureBinary(request.clone()))),
        );
        Some((id, AppCommand::CaptureBinaryInbox(request)))
    }
    pub fn retry_capture(&mut self) -> Option<(Uuid, AppCommand)> {
        self.inbox_queue.capture_error.as_ref()?;
        if let Some(request) = self.inbox_queue.last_binary_capture.clone() {
            self.capture_binary_inbox(request)
        } else {
            self.capture_inbox(self.inbox_queue.last_capture.clone()?)
        }
    }
    pub fn processing_pending(&self) -> bool {
        self.pending.values().any(|pending| {
            matches!(pending,
            Pending::Inbox(pending) if matches!(pending.as_ref(), InboxPending::Process(_) | InboxPending::Cancel(_)))
        })
    }
    pub fn process_inbox_items(&mut self, items: Vec<InboxItem>) -> Option<(Uuid, AppCommand)> {
        self.submit_inbox_process(ProcessInboxRequest {
            limits: None,

            id: Uuid::new_v4(),
            items,
        })
    }
    fn submit_inbox_process(&mut self, request: ProcessInboxRequest) -> Option<(Uuid, AppCommand)> {
        if let Err(error) = request.validate() {
            self.inbox_queue.error = Some(error.to_string());
            return None;
        }
        if !self.ready || self.processing_pending() || self.pending.contains_key(&request.id) {
            self.inbox_queue.error = Some("Wait for the admitted Inbox batch to settle.".into());
            return None;
        }
        self.inbox_queue.last_process = Some(request.clone());
        self.inbox_queue.process_failed = false;
        self.inbox_queue.error = None;
        self.inbox_queue.preview = None;
        self.inbox_queue.preview_generation = self.inbox_queue.preview_generation.wrapping_add(1);
        self.inbox_queue.source_generation = self.inbox_queue.source_generation.wrapping_add(1);
        let id = request.id;
        self.pending.insert(
            id,
            Pending::Inbox(Box::new(InboxPending::Process(request.clone()))),
        );
        Some((id, AppCommand::ProcessInbox(request)))
    }
    pub fn retry_process(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.inbox_queue.process_failed {
            return None;
        }
        self.submit_inbox_process(self.inbox_queue.last_process.clone()?)
    }
    pub fn can_retry_process(&self) -> bool {
        self.ready
            && self.inbox_queue.process_failed
            && self.inbox_queue.last_process.is_some()
            && !self.processing_pending()
    }
    pub fn cancel_inbox_batch(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready
            || !self.processing_pending()
            || self.pending.values().any(|pending| {
                matches!(pending,
            Pending::Inbox(pending) if matches!(pending.as_ref(), InboxPending::Cancel(_)))
            })
        {
            return None;
        }
        let request = self.inbox_queue.last_process.clone()?;
        Some(self.command(
            Pending::Inbox(Box::new(InboxPending::Cancel(request.clone()))),
            AppCommand::CancelInboxProcessing(request.id),
        ))
    }
    pub fn preview_inbox_candidate(&mut self, index: usize) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.inbox_queue.visible {
            return None;
        }
        let Some(conversion) = self.inbox_conversion(index) else {
            self.inbox_queue.error =
                Some("Choose a completed conversion in the retained batch.".into());
            return None;
        };
        self.inbox_queue.preview_generation = self.inbox_queue.preview_generation.wrapping_add(1);
        self.inbox_queue.source_generation = self.inbox_queue.source_generation.wrapping_add(1);
        self.inbox_queue.preview = None;
        self.inbox_queue.error = None;
        let request = conversion.request.clone();
        let pending = InboxPending::Preview {
            view: self.inbox_view_capture(),
            generation: self.inbox_queue.preview_generation,
            conversion,
            batch: Box::new(self.inbox_queue.batch.as_ref()?.clone()),
        };
        Some(self.command(
            Pending::Inbox(Box::new(pending)),
            AppCommand::InboxCandidate(request),
        ))
    }
    pub fn source_pending(&self) -> bool {
        self.pending.values().any(|pending| match pending {
            Pending::Inbox(pending) => matches!(pending.as_ref(), InboxPending::Source { view, generation, .. }
                if self.inbox_view_current(view) && *generation == self.inbox_queue.source_generation),
            _ => false,
        })
    }
    pub fn prepare_inbox_source(
        &mut self,
        request: InboxSourceRequest,
    ) -> Option<(Uuid, AppCommand)> {
        if let Err(error) = request.validate() {
            self.inbox_queue.source_error = Some(error.message);
            return None;
        }
        if !self.ready || !self.vault_bound || !self.inbox_queue.visible || self.source_pending() {
            self.inbox_queue.source_error =
                Some("Open Inbox with a bound vault before preparing a source.".into());
            return None;
        }
        let Some(conversion) = self
            .inbox_conversion(request.candidate.index)
            .filter(|conversion| conversion.request == request.candidate)
        else {
            self.inbox_queue.source_error =
                Some("Source preparation needs the exact completed batch entry.".into());
            return None;
        };
        self.inbox_queue.source_generation = self.inbox_queue.source_generation.wrapping_add(1);
        self.inbox_queue.source_error = None;
        let pending = InboxPending::Source {
            view: self.inbox_view_capture(),
            generation: self.inbox_queue.source_generation,
            request: request.clone(),
            conversion,
        };
        Some(self.command(
            Pending::Inbox(Box::new(pending)),
            AppCommand::PrepareInboxSource(request),
        ))
    }
    /// Explicit opening may replace settled presentation, never unfinished input.
    pub fn open_inbox_source_draft(&mut self) -> bool {
        if !self.ready
            || !self.vault_bound
            || !self.review_can_leave()
            || self.application_busy()
            || self.active.is_some()
            || self.rewrite.is_some()
            || self
                .editor
                .as_ref()
                .is_some_and(|editor| !editor.can_leave())
        {
            self.inbox_queue.source_error =
                Some("Finish or retain the open input before opening the prepared source.".into());
            return false;
        }
        let Some(request) = self.inbox_queue.prepared.clone() else {
            return false;
        };
        let draft = match crate::draft::DraftForm::from_inbox_source(request) {
            Ok(draft) => draft,
            Err(error) => {
                self.inbox_queue.source_error = Some(error.message);
                return false;
            }
        };
        self.draft = Some(draft);
        self.inbox_queue.prepared = None;
        self.link_preparation = Default::default();
        self.inbox_queue.source_error = None;
        true
    }
    pub(super) fn inbox_view_capture(&self) -> InboxViewCapture {
        let queue = &self.inbox_queue;
        InboxViewCapture {
            view: queue.view,
            page: queue.page_generation,
            selection: queue.selection,
        }
    }
    pub(super) fn inbox_view_current(&self, capture: &InboxViewCapture) -> bool {
        let queue = &self.inbox_queue;
        self.inbox_page_current(capture) && queue.selection == capture.selection
    }
    fn inbox_page_current(&self, capture: &InboxViewCapture) -> bool {
        let queue = &self.inbox_queue;
        queue.visible && queue.view == capture.view && queue.page_generation == capture.page
    }
    fn known_inbox_item(&self, id: Uuid) -> Option<&InboxItem> {
        self.inbox_queue
            .selected
            .iter()
            .map(|read| &read.item)
            .chain(self.inbox_queue.capture_result.iter())
            .chain(
                self.inbox_queue
                    .page
                    .iter()
                    .flat_map(|page| page.entries.iter().map(|entry| &entry.item)),
            )
            .find(|item| item.capture.id == id)
    }
    fn inbox_conversion(&self, index: usize) -> Option<ConversionCapture> {
        let batch = self.inbox_queue.batch.as_ref()?;
        batch.validate().ok()?;
        let InboxProcessOutcome::Converted {
            format,
            byte_len,
            sha256,
        } = batch.entries.get(index)?.outcome
        else {
            return None;
        };
        Some(ConversionCapture {
            extraction: self
                .inbox_queue
                .preview
                .as_ref()
                .filter(|preview| {
                    preview.request.batch_id == batch.request.id && preview.request.index == index
                })
                .and_then(|preview| preview.extraction.clone()),
            visual: self
                .inbox_queue
                .preview
                .as_ref()
                .filter(|preview| {
                    preview.request.batch_id == batch.request.id && preview.request.index == index
                })
                .and_then(|preview| preview.visual.clone()),
            request: InboxCandidateRequest {
                batch_id: batch.request.id,
                index,
            },
            original: batch.request.items[index].clone(),
            format,
            byte_len,
            sha256,
        })
    }
    pub(super) fn apply_inbox_event(&mut self, id: Uuid, event: &AppEvent) -> bool {
        let Some(Pending::Inbox(pending)) = self.pending.get(&id).cloned() else {
            return matches!(
                event,
                AppEvent::InboxCaptured(_)
                    | AppEvent::InboxItems(_)
                    | AppEvent::InboxItem(_)
                    | AppEvent::InboxProcessing(_)
                    | AppEvent::InboxCandidate(_)
                    | AppEvent::InboxSourceDraft(_)
            );
        };
        let current = match pending.as_ref() {
            InboxPending::Page { view, .. } => self.inbox_page_current(view),
            InboxPending::Read { view, id, .. } => {
                self.inbox_view_current(view) && self.inbox_queue.selected_id == Some(*id)
            }
            InboxPending::Preview {
                view, generation, ..
            } => {
                self.inbox_view_current(view) && *generation == self.inbox_queue.preview_generation
            }
            InboxPending::Source {
                view, generation, ..
            } => self.inbox_view_current(view) && *generation == self.inbox_queue.source_generation,
            _ => true,
        };
        if !current {
            self.pending.remove(&id);
            return true;
        }
        let mut terminal = true;
        match (pending.as_ref(), event) {
            (InboxPending::Page { request, .. }, AppEvent::InboxItems(page))
                if page_valid(page, request)
                    && page.entries.iter().all(|entry| {
                        self.known_inbox_item(entry.item.capture.id)
                            .is_none_or(|known| known == &entry.item)
                    }) =>
            {
                self.inbox_queue.page = Some((**page).clone());
                self.inbox_queue.error = None;
            }
            (
                InboxPending::Read {
                    id: item_id,
                    expected,
                    ..
                },
                AppEvent::InboxItem(read),
            ) if read.item.capture.id == *item_id
                && read_valid(read)
                && expected
                    .as_ref()
                    .is_none_or(|expected| expected == &read.item) =>
            {
                self.inbox_queue.selected = Some((**read).clone());
                self.inbox_queue.error = None;
            }
            (InboxPending::Capture(request), AppEvent::InboxCaptured(item))
                if request.id == id && request.validate_receipt(item).is_ok() =>
            {
                self.inbox_queue.capture_result = Some((**item).clone());
                self.inbox_queue.capture_error = None;
                self.notice = "Exact original retained in Inbox. Prepare and review its source before approval.".into();
            }
            (
                InboxPending::Process(request) | InboxPending::Cancel(request),
                AppEvent::InboxProcessing(batch),
            ) if batch.request == *request
                && batch.validate().is_ok()
                && self.inbox_queue.batch.as_ref().is_none_or(|previous| {
                    previous.request.id != request.id || batch_advances(previous, batch)
                }) =>
            {
                let complete = batch.pending_count() == 0;
                self.inbox_queue.batch = Some((**batch).clone());
                self.inbox_queue.process_failed = false;
                self.inbox_queue.error = None;
                terminal = matches!(pending.as_ref(), InboxPending::Cancel(_)) || complete;
                if complete
                    && matches!(self.pending.get(&request.id), Some(Pending::Inbox(pending)) if matches!(pending.as_ref(), InboxPending::Process(bound) if bound == request))
                {
                    self.pending.remove(&request.id);
                }
            }
            (
                InboxPending::Preview {
                    conversion, batch, ..
                },
                AppEvent::InboxCandidate(preview),
            ) if preview.request == conversion.request
                && preview.validate_receipt(batch).is_ok() =>
            {
                self.inbox_queue.preview = Some((**preview).clone());
                self.inbox_queue.error = None;
            }
            (
                InboxPending::Source {
                    request,
                    conversion,
                    ..
                },
                AppEvent::InboxSourceDraft(draft),
            ) if request.validate_draft(draft).is_ok()
                && conversion.visual.as_ref().is_none_or(|visual| draft.changes.iter().any(|change| matches!(change, brn_workflow::proposals::DraftNoteChange::CreateAsset { bytes, .. } if bytes == &visual.bytes)))
                && conversion.extraction.as_ref().is_none_or(|extraction| extraction.assets.iter().all(|asset| draft.changes.iter().any(|change| matches!(change, brn_workflow::proposals::DraftNoteChange::CreateAsset { bytes, .. } if bytes == &asset.bytes))))
                && draft
                    .inbox_source
                    .as_deref()
                    .is_some_and(|binding| conversion.binding_matches(binding)) =>
            {
                self.inbox_queue.prepared = Some((**draft).clone());
                self.inbox_queue.source_error = None;
            }
            (InboxPending::CaptureBinary(request), AppEvent::InboxCaptured(item))
                if request.id == id && request.validate_receipt(item).is_ok() => {
                self.inbox_queue.capture_result = Some((**item).clone());
                self.inbox_queue.capture_error = None;
                self.notice = "Exact EML/DOCX file retained in Inbox. Inspect extraction and originals before approval.".into();
            }
            (InboxPending::Capture(_) | InboxPending::CaptureBinary(_), AppEvent::Failed(error)) => {
                self.inbox_queue.capture_error = Some(error.message.clone())
            }
            (InboxPending::Source { .. }, AppEvent::Failed(error)) => {
                self.inbox_queue.source_error = Some(error.message.clone())
            }
            (InboxPending::Process(_), AppEvent::Failed(error)) => {
                self.inbox_queue.error = Some(error.message.clone());
                self.inbox_queue.process_failed = true;
            }
            (_, AppEvent::Failed(error)) => self.inbox_queue.error = Some(error.message.clone()),
            _ => return true,
        }
        if terminal {
            self.pending.remove(&id);
        }
        true
    }
}

impl ConversionCapture {
    fn binding_matches(&self, binding: &InboxSourceBinding) -> bool {
        let (byte_len, sha256) = if let Some(extraction) = &self.extraction {
            let Ok(materialized) = extraction.materialize_for_source(&binding.note_id.to_string())
            else {
                return false;
            };
            (
                materialized.markdown.len() as u64,
                brn_intake::digest(materialized.markdown.as_bytes()),
            )
        } else {
            (self.byte_len, self.sha256)
        };
        binding.batch_id == self.request.batch_id
            && binding.index == self.request.index
            && binding.original == self.original
            && binding.format == self.format
            && binding.byte_len == byte_len
            && binding.sha256 == sha256
            && match (&self.extraction, &binding.extraction) {
                (None, None) => true,
                (Some(extraction), Some(receipt)) => {
                    receipt.assets.len() == extraction.assets.len()
                        && extraction.assets.iter().all(|asset| {
                            brn_intake::asset_file_name_for_source(
                                asset,
                                &binding.note_id.to_string(),
                            )
                            .is_ok_and(|name| {
                                receipt.assets.iter().any(|stored| {
                                    stored.name == name
                                        && stored.sha256 == asset.sha256
                                        && stored.byte_len == asset.bytes.len() as u64
                                })
                            })
                        })
                }
                _ => false,
            }
            && self
                .visual
                .as_ref()
                .is_none_or(|visual| binding.visual.as_ref() == Some(&visual.proof))
    }
}
fn read_valid(read: &InboxRead) -> bool {
    read.validate_receipt().is_ok()
}
fn page_valid(page: &InboxInventory, request: &InboxListRequest) -> bool {
    let mut ids = std::collections::HashSet::new();
    request.validate().is_ok()
        && page.entries.len() <= request.limit
        && page.total_count >= page.entries.len()
        && page.issues.len() <= 100
        && page.entries.iter().all(|entry| {
            entry.item.validate().is_ok()
                && ids.insert(entry.item.capture.id)
                && Some(entry.item.capture.id) != request.after
        })
        && page.entries.windows(2).all(|pair| {
            (pair[0].item.received_at_ms, pair[0].item.capture.id)
                < (pair[1].item.received_at_ms, pair[1].item.capture.id)
        })
        && page.next_after.is_none_or(|cursor| {
            page.entries
                .last()
                .is_some_and(|entry| entry.item.capture.id == cursor)
        })
        && page
            .issues
            .iter()
            .all(|issue| issue.item_id.is_none_or(|id| !id.is_nil()) && issue.message.len() <= 512)
}
fn batch_advances(previous: &InboxProcessBatch, next: &InboxProcessBatch) -> bool {
    previous.request == next.request
        && previous.queued_at_ms == next.queued_at_ms
        && previous
            .entries
            .iter()
            .zip(&next.entries)
            .all(|(before, after)| {
                if !before.outcome.pending() {
                    before == after
                } else {
                    before
                        .started_at_ms
                        .is_none_or(|started| Some(started) == after.started_at_ms)
                        && (!matches!(before.outcome, InboxProcessOutcome::Running)
                            || !matches!(after.outcome, InboxProcessOutcome::Queued))
                }
            })
}
