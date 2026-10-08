//! Guided presentation over the existing capture, extraction and proposal commands.
use super::*;
use brn_workflow::{
    ReasoningEffort, Selection, inbox_processing::IntakeSnapshot, proposals::ProposalRecord,
};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct GuidedInbox {
    pub selected: Option<Uuid>,
    pub selected_item: Option<InboxItem>,
    saved_choices: BTreeMap<Uuid, Uuid>,
    pub snapshots: Vec<IntakeSnapshot>,
    pub snapshot_id: Option<Uuid>,
    pub error: Option<String>,
    pub(super) automatic_capture: Option<Uuid>,
    automatic_batch: Option<(Uuid, Uuid)>,
    pub(super) source_intent: Option<SourceIntent>,
    investigation: Option<(Uuid, SourceIntent)>,
}

#[derive(Clone)]
pub struct SourceIntent {
    item: Uuid,
    proposal: Uuid,
    view: InboxViewCapture,
    investigate: bool,
    selection: Option<Selection>,
    effort: Option<ReasoningEffort>,
    generation: u64,
}

#[derive(Clone)]
pub enum GuidedPending {
    Snapshots {
        view: InboxViewCapture,
        item: InboxItem,
    },
    Create {
        request: DraftRequest,
        intent: SourceIntent,
    },
}

impl AiState {
    /// The import gesture admits local extraction, never provider work or approval.
    pub fn import_binary_inbox(
        &mut self,
        request: CaptureBinaryInboxRequest,
    ) -> Option<(Uuid, AppCommand)> {
        let command = self.capture_binary_inbox(request)?;
        self.inbox_queue.guided.automatic_capture = Some(command.0);
        Some(command)
    }
    pub fn import_text_inbox(
        &mut self,
        request: CaptureInboxRequest,
    ) -> Option<(Uuid, AppCommand)> {
        let command = self.capture_inbox(request)?;
        self.inbox_queue.guided.automatic_capture = Some(command.0);
        Some(command)
    }
    pub fn select_guided_inbox(&mut self, item_id: Uuid) -> Option<(Uuid, AppCommand)> {
        let item = self.known_inbox_item(item_id).cloned();
        let command = self.select_inbox(item_id)?;
        let guided = &mut self.inbox_queue.guided;
        guided.selected = Some(item_id);
        guided.selected_item = item;
        guided.automatic_capture = None;
        guided.automatic_batch = None;
        guided.snapshots.clear();
        guided.snapshot_id = None;
        guided.source_intent = None;
        guided.investigation = None;
        guided.error = None;
        self.inbox_queue.source_error = None;
        Some(command)
    }
    pub(super) fn invalidate_guided_inbox(&mut self) {
        let guided = &mut self.inbox_queue.guided;
        guided.automatic_capture = None;
        guided.automatic_batch = None;
        guided.source_intent = None;
        guided.investigation = None;
    }
    pub fn guided_inbox_loading(&self) -> bool {
        self.pending.values().any(|pending| matches!(pending,
            Pending::InboxGuided(pending) if matches!(pending.as_ref(), GuidedPending::Snapshots {view, ..} if self.inbox_view_current(view))))
    }
    pub fn guided_source_busy(&self) -> bool {
        self.inbox_queue.guided.source_intent.is_some() || self.inbox_queue.guided.investigation.is_some()
            || self.pending.values().any(|pending| matches!(pending, Pending::InboxGuided(pending)
                if matches!(pending.as_ref(), GuidedPending::Create {intent, ..} if self.inbox_view_current(&intent.view))))
    }
    pub(super) fn guided_original_ready(&mut self) {
        let Some(read) = self
            .inbox_queue
            .selected
            .as_ref()
            .filter(|read| Some(read.item.capture.id) == self.inbox_queue.guided.selected)
        else {
            return;
        };
        let item = read.item.clone();
        self.inbox_queue.guided.selected_item = Some(item.clone());
        let command = self.command(
            Pending::InboxGuided(Box::new(GuidedPending::Snapshots {
                view: self.inbox_view_capture(),
                item: item.clone(),
            })),
            AppCommand::InboxRetainedExtractions(item.capture.id),
        );
        self.inbox_queue.followups.push(command);
    }
    pub fn read_guided_original(&mut self) -> Option<(Uuid, AppCommand)> {
        let read = self.inbox_queue.selected.as_ref()?;
        if self.processing_pending()
            || Some(read.item.capture.id) != self.inbox_queue.guided.selected
        {
            return None;
        }
        let item = read.item.clone();
        let command = self.process_inbox_items(vec![item.clone()])?;
        self.inbox_queue.guided.automatic_batch = Some((command.0, item.capture.id));
        Some(command)
    }
    pub(super) fn guided_conversion_ready(&mut self, batch: Uuid) {
        let Some((expected, item)) = self.inbox_queue.guided.automatic_batch else {
            return;
        };
        if expected != batch
            || self.inbox_queue.guided.selected != Some(item)
            || !self.inbox_queue.visible
        {
            return;
        }
        self.inbox_queue.guided.automatic_batch = None;
        if let Some(command) = self.preview_inbox_candidate(0) {
            self.inbox_queue.followups.push(command);
        }
    }
    /// A saved version is historical evidence. It is never a refreshed approval proof.
    pub fn select_saved_inbox_extraction(&mut self, id: Uuid) -> bool {
        if !self.present_saved_inbox_extraction(id) {
            return false;
        }
        if let Some(item) = self.inbox_queue.guided.selected {
            self.inbox_queue.guided.saved_choices.insert(item, id);
        }
        true
    }
    fn present_saved_inbox_extraction(&mut self, id: Uuid) -> bool {
        if !self.ready || !self.inbox_queue.visible || self.guided_source_busy() {
            return false;
        }
        let Some(snapshot) = self
            .inbox_queue
            .guided
            .snapshots
            .iter()
            .find(|snapshot| {
                snapshot.id == id
                    && Some(snapshot.original.capture.id) == self.inbox_queue.guided.selected
            })
            .cloned()
        else {
            return false;
        };
        self.inbox_queue.preview_generation = self.inbox_queue.preview_generation.wrapping_add(1);
        self.inbox_queue.source_generation = self.inbox_queue.source_generation.wrapping_add(1);
        self.inbox_queue.guided.snapshot_id = Some(id);
        self.inbox_queue.preview = Some(InboxConversionPreview {
            request: InboxCandidateRequest {
                batch_id: snapshot.batch_id,
                index: snapshot.index,
            },
            original: snapshot.original,
            format: InboxConversionFormat::MaintainedExtractionV1,
            markdown: snapshot.extraction.markdown.clone(),
            needs_semantic_review: true,
            visual: None,
            extraction: Some(snapshot.extraction),
        });
        self.inbox_queue.guided.error = None;
        true
    }
    /// Explicit user request: retain one Source draft and optionally investigate it.
    pub fn begin_guided_source(
        &mut self,
        request: InboxSourceRequest,
        investigate: bool,
    ) -> Option<(Uuid, AppCommand)> {
        if self.guided_source_busy() || self.application_busy() || (investigate && !self.can_ask())
        {
            return None;
        }
        let preview = self.inbox_queue.preview.as_ref()?;
        let item = preview.original.capture.id;
        if investigate && preview.extraction.is_none() {
            self.inbox_queue.guided.error = Some("Save this text as a Source first, then use saved Source investigation in advanced tools.".into());
            return None;
        }
        if self.inbox_queue.guided.selected != Some(item) {
            return None;
        }
        let command = self.prepare_inbox_source(request.clone())?;
        self.inbox_queue.guided.source_intent = Some(SourceIntent {
            item,
            proposal: request.proposal_id,
            view: self.inbox_view_capture(),
            investigate,
            selection: self.selection.clone(),
            effort: self.effort,
            generation: self.generation,
        });
        Some(command)
    }
    pub(super) fn guided_source_ready(&mut self) {
        let Some(intent) = self.inbox_queue.guided.source_intent.take() else {
            return;
        };
        if !self.inbox_view_current(&intent.view)
            || self.inbox_queue.guided.selected != Some(intent.item)
        {
            return;
        }
        let Some(request) = self
            .inbox_queue
            .prepared
            .as_ref()
            .filter(|request| request.id == intent.proposal)
            .cloned()
        else {
            return;
        };
        let command = self.command(
            Pending::InboxGuided(Box::new(GuidedPending::Create {
                request: request.clone(),
                intent,
            })),
            AppCommand::CreateProposal(request),
        );
        self.inbox_queue.followups.push(command);
    }
    pub(crate) fn received_guided_inbox(
        &mut self,
        id: Uuid,
        event: &AppEvent,
    ) -> Option<Vec<(Uuid, AppCommand)>> {
        let Some(Pending::InboxGuided(pending)) = self.pending.get(&id).cloned() else {
            return matches!(event, AppEvent::InboxRetainedExtractions { .. }).then(Vec::new);
        };
        let mut commands = Vec::new();
        match (pending.as_ref(), event) {
            (
                GuidedPending::Snapshots { view, item },
                AppEvent::InboxRetainedExtractions { item_id, snapshots },
            ) => {
                if !self.inbox_view_current(view)
                    || self.inbox_queue.guided.selected != Some(item.capture.id)
                {
                    self.pending.remove(&id);
                    return Some(commands);
                }
                if *item_id != item.capture.id
                    || snapshots.len() > 64
                    || snapshots
                        .iter()
                        .any(|snapshot| snapshot.original != *item || snapshot.validate().is_err())
                {
                    return Some(commands);
                }
                self.inbox_queue.guided.snapshots = snapshots.clone();
                self.pending.remove(&id);
                if let Some(chosen) = self
                    .inbox_queue
                    .guided
                    .saved_choices
                    .get(&item.capture.id)
                    .copied()
                    && snapshots.iter().any(|snapshot| snapshot.id == chosen)
                {
                    self.present_saved_inbox_extraction(chosen);
                } else if snapshots.len() == 1 {
                    self.present_saved_inbox_extraction(snapshots[0].id);
                } else if snapshots.is_empty()
                    && self.inbox_queue.guided.automatic_capture.take() == Some(item.capture.id)
                    && let Some(command) = self.read_guided_original()
                {
                    commands.push(command);
                }
                return Some(commands);
            }
            (GuidedPending::Create { request, intent }, AppEvent::Proposal(record))
                if crate::draft::creation_matches(request, record) =>
            {
                self.proposals
                    .retain(|known| known.draft.id != record.draft.id);
                self.proposals.push(record.clone());
                if self.inbox_view_current(&intent.view)
                    && self.inbox_queue.guided.selected == Some(intent.item)
                {
                    self.notice =
                        "Source draft retained. Review its complete changes before approval."
                            .into();
                    if intent.investigate
                        && self.selection == intent.selection
                        && self.effort == intent.effort
                        && self.generation == intent.generation
                        && let Some(command) = self.inspect_private_intake(record.draft.id)
                    {
                        self.inbox_queue.guided.investigation = Some((command.0, intent.clone()));
                        commands.push(command);
                    }
                }
            }
            (pending, AppEvent::Failed(error)) => {
                let view = match pending {
                    GuidedPending::Snapshots { view, .. } => view,
                    GuidedPending::Create { intent, .. } => &intent.view,
                };
                if self.inbox_view_current(view) {
                    self.inbox_queue.guided.error = Some(error.message.clone());
                    self.inbox_queue.guided.source_intent = None;
                    self.inbox_queue.guided.investigation = None;
                }
            }
            _ => return Some(commands),
        }
        self.pending.remove(&id);
        Some(commands)
    }
    pub(crate) fn guided_analysis_ready(
        &mut self,
        id: Uuid,
        event: &AppEvent,
    ) -> Vec<(Uuid, AppCommand)> {
        let Some((operation, intent)) = self.inbox_queue.guided.investigation.as_ref() else {
            return Vec::new();
        };
        if *operation != id || self.pending.contains_key(&id) {
            return Vec::new();
        }
        let intent = intent.clone();
        self.inbox_queue.guided.investigation = None;
        if self.inbox_view_current(&intent.view)
            && self.inbox_queue.guided.selected == Some(intent.item)
            && let AppEvent::Failed(error) = event
        {
            self.inbox_queue.guided.error = Some(error.message.clone());
            return Vec::new();
        }
        let binding_matches = matches!(event, AppEvent::InboxIntakeBinding(_))
            && self
                .inbox_analysis
                .intake
                .as_ref()
                .is_some_and(|intake| intake.source_proposal.id == intent.proposal);
        if binding_matches
            && self.inbox_view_current(&intent.view)
            && self.inbox_queue.guided.selected == Some(intent.item)
            && self.selection == intent.selection
            && self.effort == intent.effort
            && self.generation == intent.generation
            && let Some(command) = self.analyze_inbox_source()
        {
            return vec![command];
        }
        Vec::new()
    }
    /// Related cards are joined by the exact Source proposal, never a filename or title.
    pub fn guided_related_proposals(&self) -> Vec<&ProposalRecord> {
        let Some(item) = self.inbox_queue.guided.selected else {
            return Vec::new();
        };
        let sources: Vec<_> = self
            .proposals
            .iter()
            .filter(|record| {
                record
                    .draft
                    .inbox_source
                    .as_ref()
                    .is_some_and(|binding| binding.original.capture.id == item)
            })
            .collect();
        self.proposals
            .iter()
            .filter(|record| {
                sources
                    .iter()
                    .any(|source| source.draft.id == record.draft.id)
                    || crate::approval::intake_dependency(record).is_some_and(|binding| {
                        sources.iter().any(|source| {
                            binding.source_proposal.id == source.draft.id
                                && source
                                    .draft
                                    .inbox_source
                                    .as_ref()
                                    .and_then(|b| b.extraction.as_ref())
                                    .is_some_and(|e| {
                                        e.snapshot_id == binding.snapshot_id
                                            && e.snapshot_sha256 == binding.snapshot_sha256
                                    })
                        })
                    })
            })
            .collect()
    }
    pub fn guided_source_record(&self) -> Option<&ProposalRecord> {
        let item = self.inbox_queue.guided.selected?;
        let preview = self.inbox_queue.preview.as_ref()?;
        let mut matching = self.proposals.iter().filter(|record| {
            record.draft.inbox_source.as_ref().is_some_and(|binding| {
                binding.original.capture.id == item
                    && binding.batch_id == preview.request.batch_id
                    && binding.index == preview.request.index
            })
        });
        let first = matching.next()?;
        matching.next().is_none().then_some(first)
    }
    /// One explicit click may load the exact binding and submit one investigation.
    pub fn begin_guided_investigation(&mut self, source_id: Uuid) -> Option<(Uuid, AppCommand)> {
        if self.guided_source_busy() || !self.can_ask() {
            return None;
        }
        let source = self
            .guided_source_record()
            .filter(|source| source.draft.id == source_id)?;
        let binding = source.draft.inbox_source.as_ref()?;
        let item = binding.original.capture.id;
        if source.state != brn_workflow::proposals::ProposalState::Draft {
            self.inbox_queue.guided.error = Some("This Source is already settled. Investigation of its retained email and pictures needs a pending Source draft.".into());
            return None;
        }
        let intent = SourceIntent {
            item,
            proposal: source_id,
            view: self.inbox_view_capture(),
            investigate: true,
            selection: self.selection.clone(),
            effort: self.effort,
            generation: self.generation,
        };
        let command = self.inspect_private_intake(source_id)?;
        self.inbox_queue.guided.investigation = Some((command.0, intent));
        Some(command)
    }
}
