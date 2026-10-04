//! Native input/correlation over the existing read-only link workflow.
use super::{AiState, Pending};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    knowledge::{IdentityOutcome, LinkRequest, NoteLinks},
    proposals::{DraftNoteChange, DraftRequest, ProposalSource},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct LinkCapture {
    pub form: Uuid,
    pub generation: u64,
    pub binding_generation: u64,
    pub document_generation: u64,
    pub input_generation: u64,
    pub target_path: String,
    pub source: ProposalSource,
}
#[derive(Default)]
pub struct LinkPreparation {
    pub form: Option<Uuid>,
    pub target_path: String,
    pub label: String,
    pub generation: u64,
    pub target: Option<NoteLinks>,
    pub operation: Option<Uuid>,
    pub error: Option<String>,
}
impl AiState {
    pub fn edit_link_input(&mut self, target_path: String, label: String) {
        let Some(form) = self.draft.as_ref() else {
            return;
        };
        let input = &mut self.link_preparation;
        if input.form == Some(form.id) && input.target_path == target_path && input.label == label {
            return;
        }
        let Some(generation) = input.generation.checked_add(1) else {
            input.error = Some("Link input generation exhausted; copy retained input.".into());
            return;
        };
        if input.form != Some(form.id) || input.target_path != target_path {
            input.target = None;
        }
        input.form = Some(form.id);
        input.target_path = target_path;
        input.label = label;
        input.generation = generation;
        input.operation = None;
        input.error = None;
    }
    pub fn link_preparation_available(&self) -> bool {
        self.ready
            && self.vault_bound
            && !self.application_busy()
            && self.active.is_none()
            && self.rewrite.is_none()
            && self.draft.as_ref().is_some_and(|form| {
                form.kind == crate::draft::DraftKind::Replace
                    && form.prepared_request().is_none()
                    && !form.pending
                    && form.submitted.is_none()
                    && form.result.is_none()
                    && form.source_operation.is_none()
                    && form
                        .source
                        .as_ref()
                        .is_some_and(|source| source.source.path == form.path)
            })
    }
    fn link_capture(&self) -> Option<LinkCapture> {
        if !self.link_preparation_available() {
            return None;
        }
        let form = self.draft.as_ref()?;
        let input = &self.link_preparation;
        if input.form != Some(form.id) {
            return None;
        }
        Some(LinkCapture {
            form: form.id,
            generation: form.generation,
            binding_generation: form.binding_generation,
            document_generation: self.note_generation,
            input_generation: input.generation,
            target_path: input.target_path.clone(),
            source: form.source.clone()?,
        })
    }
    fn link_capture_matches(&self, capture: &LinkCapture, operation: Uuid) -> bool {
        self.link_preparation.operation == Some(operation)
            && self.link_preparation.form == Some(capture.form)
            && self.link_preparation.generation == capture.input_generation
            && self.link_preparation.target_path == capture.target_path
            && self.note_generation == capture.document_generation
            && self.draft.as_ref().is_some_and(|form| {
                form.id == capture.form
                    && form.generation == capture.generation
                    && form.binding_generation == capture.binding_generation
                    && form.source.as_ref() == Some(&capture.source)
                    && form.source_operation.is_none()
                    && form.submitted.is_none()
                    && form.prepared_request().is_none()
            })
    }
    pub fn inspect_link_target(&mut self) -> Option<(Uuid, AppCommand)> {
        if self.link_preparation.operation.is_some() {
            return None;
        }
        let capture = self.link_capture()?;
        let path = capture.target_path.clone();
        self.link_preparation.target = None;
        self.link_preparation.error = None;
        let command = self.command(Pending::LinkTarget(capture), AppCommand::NoteLinks(path));
        self.link_preparation.operation = Some(command.0);
        Some(command)
    }
    pub fn prepare_link_draft(&mut self) -> Option<(Uuid, AppCommand)> {
        if self.link_preparation.operation.is_some() {
            return None;
        }
        let capture = self.link_capture()?;
        let form = self.draft.as_ref()?;
        if !form.text.is_empty() && form.text != capture.source.text {
            self.link_preparation.error = Some("Link preparation uses saved consumer text. Copy and explicitly clear the authored proposed body, or start a separate form; all current input is retained.".into());
            return None;
        }
        let target = self.link_preparation.target.as_ref()?;
        if target.source.path != capture.target_path
            || target.source_outcome != Some(IdentityOutcome::Unique)
        {
            return None;
        }
        let request = LinkRequest {
            path: form.path.clone(),
            target_note_id: target.source.note_id?,
            expected_target_sha256: target.source.sha256,
            proposal_id: Uuid::new_v4(),
            title: form.title.clone(),
            label: self.link_preparation.label.clone(),
        };
        if let Err(error) = request.validate() {
            self.link_preparation.error = Some(error.message);
            return None;
        }
        let command = self.command(
            Pending::LinkPrepare {
                capture,
                request: Box::new(request.clone()),
            },
            AppCommand::PrepareNoteLink(request),
        );
        self.link_preparation.operation = Some(command.0);
        self.link_preparation.error = None;
        Some(command)
    }
    /// Consume only this dedicated operation. Wrong event/proof cannot settle it.
    pub(super) fn received_link_preparation(&mut self, id: Uuid, event: &AppEvent) -> bool {
        let Some(pending @ (Pending::LinkTarget(_) | Pending::LinkPrepare { .. })) =
            self.pending.get(&id).cloned()
        else {
            return false;
        };
        let capture = match &pending {
            Pending::LinkTarget(capture) | Pending::LinkPrepare { capture, .. } => capture,
            _ => unreachable!(),
        };
        if !self.link_capture_matches(capture, id) {
            self.pending.remove(&id);
            if self.link_preparation.operation == Some(id) {
                self.link_preparation.operation = None;
            }
            return true;
        }
        match (&pending, event) {
            (Pending::LinkTarget(_), AppEvent::NoteLinks(links))
                if links.source.path == capture.target_path =>
            {
                self.link_preparation.target = Some((**links).clone());
                if links.source_outcome == Some(IdentityOutcome::Unique)
                    && links.source.note_id.is_some_and(|id| !id.is_nil())
                {
                    self.link_preparation.error = None;
                } else {
                    self.link_preparation.error = Some(format!(
                        "Target identity is {:?}; select uniquely identified saved evidence. No identity was guessed.",
                        links.source_outcome
                    ));
                }
            }
            (Pending::LinkPrepare { request, .. }, AppEvent::NoteLinkDraft(prepared)) => {
                if prepared.id != request.proposal_id || prepared.title != request.title {
                    return true;
                }
                if !prepared_matches(prepared, request, capture) {
                    self.link_preparation.error = Some("Prepared consumer or target proof changed. Full input is retained; reload the exact consumer and inspect the target before retrying.".into());
                    self.link_preparation.operation = None;
                    self.pending.remove(&id);
                    return true;
                }
                let mut prepared = (**prepared).clone();
                prepared.session_id = self.draft.as_ref().and_then(|form| form.session_id);
                match crate::draft::DraftForm::from_prepared(prepared) {
                    Ok(mut form) => {
                        form.source = Some(capture.source.clone());
                        self.draft = Some(form);
                        self.link_preparation = LinkPreparation::default();
                        self.notice = "Exact link input prepared with both saved source bindings. Create review work explicitly; Markdown is unchanged.".into();
                    }
                    Err(error) => self.link_preparation.error = Some(error.message),
                }
            }
            (_, AppEvent::Failed(error)) => {
                self.link_preparation.error = Some(error.message.clone())
            }
            _ => return true,
        }
        self.pending.remove(&id);
        if self.link_preparation.operation == Some(id) {
            self.link_preparation.operation = None;
        }
        true
    }
}
fn prepared_matches(prepared: &DraftRequest, request: &LinkRequest, capture: &LinkCapture) -> bool {
    prepared.id == request.proposal_id
        && prepared.title == request.title
        && prepared.group_id.is_none()
        && prepared.session_id.is_none()
        && prepared.sources.len() == 2
        && prepared.sources.contains(&capture.source.source)
        && prepared.sources.iter().any(|source| {
            source.path != request.path
                && source.fingerprint.sha256 == request.expected_target_sha256
        })
        && matches!(prepared.changes.as_slice(), [DraftNoteChange::Replace { path, expected, text }]
            if path == &request.path && expected == &capture.source.source.fingerprint
                && text.as_bytes().starts_with(capture.source.text.as_bytes()))
}
