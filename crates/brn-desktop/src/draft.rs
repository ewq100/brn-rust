//! Retained full initial form. Workflow owns source capture and typed creation.
use brn_workflow::{
    MAX_NOTE_BYTES, WorkTurn, WorkTurnStatus,
    proposals::{DraftNoteChange, DraftRequest, NoteChange, ProposalRecord, ProposalSource},
};
use uuid::Uuid;

#[path = "action_input.rs"]
mod action_input;
pub use action_input::InitialAction;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DraftKind {
    #[default]
    Create,
    Replace,
    Trash,
}

#[derive(Clone)]
pub struct SubmittedDraft {
    pub operation: Uuid,
    pub generation: u64,
    pub request: DraftRequest,
}

pub struct DraftForm {
    pub id: Uuid,
    pub title: String,
    pub path: String,
    pub text: String,
    pub kind: DraftKind,
    pub session_id: Option<Uuid>,
    pub generation: u64,
    pub binding_generation: u64,
    pub source: Option<ProposalSource>,
    pub source_operation: Option<Uuid>,
    pub source_error: Option<String>,
    pub submitted: Option<SubmittedDraft>,
    pub pending: bool,
    pub result: Option<(u64, ProposalRecord)>,
    pub error: Option<String>,
    pub action: Option<Box<InitialAction>>,
    prepared: Option<DraftRequest>,
}

#[cfg(test)]
#[path = "draft_prepared_tests.rs"]
mod prepared_tests;

#[cfg(test)]
#[path = "draft_source_tests.rs"]
mod source_tests;

#[cfg(test)]
#[path = "action_draft_tests.rs"]
mod action_draft_tests;

impl DraftForm {
    /// Retain the complete workflow-prepared Source creation for exact review.
    pub fn from_inbox_source(request: DraftRequest) -> brn_workflow::Result<Self> {
        request.validate()?;
        let [DraftNoteChange::Create { path, text }] = request.changes.as_slice() else {
            return Err(brn_workflow::WorkflowError::msg(
                "Prepared Inbox Source review needs exactly one new Source note.",
            ));
        };
        if request.inbox_source.is_none() {
            return Err(brn_workflow::WorkflowError::msg(
                "Prepared Inbox Source review needs its complete original and conversion binding.",
            ));
        }
        Ok(Self {
            id: request.id,
            title: request.title.clone(),
            path: path.clone(),
            text: text.clone(),
            kind: DraftKind::Create,
            session_id: request.session_id,
            generation: 1,
            binding_generation: 1,
            source: None,
            source_operation: None,
            source_error: None,
            submitted: None,
            pending: false,
            result: None,
            error: None,
            action: None,
            prepared: Some(request),
        })
    }

    /// Retain the complete read-only link preparation as ordinary review input.
    /// The consumer and target source versions stay bound; no source text is invented.
    pub fn from_prepared(request: DraftRequest) -> brn_workflow::Result<Self> {
        request.validate()?;
        let [
            DraftNoteChange::Replace {
                path,
                expected,
                text,
            },
        ] = request.changes.as_slice()
        else {
            return Err(brn_workflow::WorkflowError::msg(
                "Prepared link review needs exactly one existing-note replacement.",
            ));
        };
        if request.sources.len() != 2
            || !request.action_changes.is_empty()
            || !request
                .sources
                .iter()
                .any(|source| source.path == *path && source.fingerprint == *expected)
        {
            return Err(brn_workflow::WorkflowError::msg(
                "Prepared link review needs distinct consumer/target bindings and the consumer's exact full fingerprint.",
            ));
        }
        Ok(Self {
            id: request.id,
            title: request.title.clone(),
            path: path.clone(),
            text: text.clone(),
            kind: DraftKind::Replace,
            session_id: request.session_id,
            generation: 1,
            binding_generation: 1,
            source: None,
            source_operation: None,
            source_error: None,
            submitted: None,
            pending: false,
            result: None,
            error: None,
            action: None,
            prepared: Some(request),
        })
    }

    /// The validated preparation, including all immutable full source bindings.
    pub fn prepared_request(&self) -> Option<&DraftRequest> {
        self.prepared.as_ref()
    }

    pub fn is_prepared_source(&self) -> bool {
        self.prepared
            .as_ref()
            .is_some_and(|request| request.inbox_source.is_some())
    }

    /// Copy retained input to a new proposal UUID without losing prepared proofs.
    /// Invalid full typing is retained for correction; admission validates it later.
    pub fn separate(&self) -> Option<Self> {
        if self.pending {
            return None;
        }
        let mut next = Self::new(None)?;
        next.edit(
            self.title.clone(),
            self.path.clone(),
            self.text.clone(),
            self.kind,
        );
        next.session_id = self.session_id;
        next.source = self.source.clone();
        next.action = self.action.clone().map(|mut action| {
            action.id = Uuid::new_v4();
            action
        });
        next.prepared = self.prepared.clone().map(|mut request| {
            request.id = next.id;
            request
        });
        Some(next)
    }

    pub fn new(turn: Option<&WorkTurn>) -> Option<Self> {
        if turn.is_some_and(|turn| {
            turn.status != WorkTurnStatus::Completed
                || turn.id.is_nil()
                || turn.conversation_id.is_nil()
                || turn.answer.len() > MAX_NOTE_BYTES
        }) {
            return None;
        }
        Some(Self {
            id: Uuid::new_v4(),
            title: String::new(),
            path: String::new(),
            text: turn.map(|turn| turn.answer.clone()).unwrap_or_default(),
            kind: DraftKind::Create,
            session_id: turn.map(|turn| turn.conversation_id),
            generation: 1,
            binding_generation: 1,
            source: None,
            source_operation: None,
            source_error: None,
            submitted: None,
            pending: false,
            result: None,
            error: None,
            action: None,
            prepared: None,
        })
    }

    pub fn edit(&mut self, title: String, path: String, text: String, kind: DraftKind) {
        if self.action.is_some() {
            self.error = Some("Use the retained Action fields to edit this form.".into());
            return;
        }
        if self.is_prepared_source()
            && (self.path != path || self.kind != kind || self.text != text)
        {
            self.error = Some(
                "Prepared Inbox Source destination, kind and exact body stay fixed. Only the proposal title is editable."
                    .into(),
            );
            return;
        }
        if self.prepared.is_some() && (self.path != path || self.kind != kind) {
            self.error = Some("Prepared link destination and kind stay fixed. Copy retained input and prepare another link to change them.".into());
            return;
        }
        if (&self.title, &self.path, &self.text, self.kind) == (&title, &path, &text, kind) {
            return;
        }
        let Some(generation) = self.generation.checked_add(1) else {
            self.error = Some("Draft form generation exhausted; copy retained input.".into());
            return;
        };
        if self.path != path || self.kind != kind {
            let Some(binding) = self.binding_generation.checked_add(1) else {
                self.error =
                    Some("Draft binding generation exhausted; copy retained input.".into());
                return;
            };
            self.binding_generation = binding;
            self.source = None;
            self.source_operation = None;
            self.source_error = None;
        }
        self.generation = generation;
        self.title = title;
        self.path = path;
        self.text = text;
        self.kind = kind;
    }

    pub fn can_leave(&self) -> bool {
        !self.pending
            && self.source_operation.is_none()
            && (self
                .result
                .as_ref()
                .is_some_and(|(generation, _)| *generation == self.generation)
                || self.submitted.is_none()
                    && self.title.is_empty()
                    && self.path.is_empty()
                    && self.text.is_empty())
            && (self
                .result
                .as_ref()
                .is_some_and(|(generation, _)| *generation == self.generation)
                || self.action.as_ref().is_none_or(|action| action.pristine()))
    }

    pub fn request(&self) -> brn_workflow::Result<DraftRequest> {
        if let Some(action) = &self.action {
            return self.action_request(action);
        }
        if let Some(prepared) = &self.prepared {
            let mut request = prepared.clone();
            let (path, text, kind) = match &mut request.changes[0] {
                DraftNoteChange::Create { path, text } if prepared.inbox_source.is_some() => {
                    (path, text, DraftKind::Create)
                }
                DraftNoteChange::Replace { path, text, .. } => (path, text, DraftKind::Replace),
                _ => unreachable!("prepared forms retain one Source Create or link Replace"),
            };
            if self.id != request.id || self.kind != kind || self.path != *path {
                return Err(brn_workflow::WorkflowError::msg(
                    if prepared.inbox_source.is_some() {
                        "Prepared Inbox Source identity, destination and kind stay fixed. Copy retained input to use another proposal UUID."
                    } else {
                        "Prepared link destination and kind stay fixed. Copy retained input and prepare another link to change them."
                    },
                ));
            }
            request.title = self.title.clone();
            *text = self.text.clone();
            request.validate()?;
            return Ok(request);
        }
        if self.kind == DraftKind::Trash && !self.text.is_empty() {
            return Err(brn_workflow::WorkflowError::msg(
                "Trash has no replacement text. Copy and explicitly clear the retained note text before creating this proposal.",
            ));
        }
        let expected = || {
            self.source
                .as_ref()
                .filter(|capture| capture.source.path == self.path)
                .map(|capture| capture.source.fingerprint.clone())
                .ok_or_else(|| {
                    brn_workflow::WorkflowError::msg(
                        "Load the exact existing note before creating its proposal.",
                    )
                })
        };
        let change = match self.kind {
            DraftKind::Create => DraftNoteChange::Create {
                path: self.path.clone(),
                text: self.text.clone(),
            },
            DraftKind::Replace => DraftNoteChange::Replace {
                path: self.path.clone(),
                expected: expected()?,
                text: self.text.clone(),
            },
            DraftKind::Trash => DraftNoteChange::Trash {
                path: self.path.clone(),
                expected: expected()?,
            },
        };
        let request = DraftRequest {
            inbox_source: None,
            action_changes: Vec::new(),
            id: self.id,
            group_id: None,
            session_id: self.session_id,
            title: self.title.clone(),
            changes: vec![change],
            sources: if self.kind == DraftKind::Create {
                vec![]
            } else {
                vec![
                    self.source
                        .as_ref()
                        .expect("validated source")
                        .source
                        .clone(),
                ]
            },
        };
        request.validate()?;
        Ok(request)
    }

    pub fn prepare(&mut self) -> Option<SubmittedDraft> {
        if self.pending || self.result.is_some() || self.source_operation.is_some() {
            return None;
        }
        let request = match self.request() {
            Ok(request) => request,
            Err(error) => {
                self.error = Some(error.message);
                return None;
            }
        };
        if self
            .submitted
            .as_ref()
            .is_some_and(|old| old.request != request)
        {
            self.error = Some("Input changed after a submitted request. Copy it and explicitly start a separate proposal; the old UUID remains bound to its original request.".into());
            return None;
        }
        let submitted = SubmittedDraft {
            operation: Uuid::new_v4(),
            generation: self.generation,
            request,
        };
        self.submitted = Some(submitted.clone());
        self.pending = true;
        self.error = None;
        Some(submitted)
    }

    pub fn created(&mut self, operation: Uuid, record: ProposalRecord) -> bool {
        let Some(submitted) = self.submitted.as_ref().filter(|submitted| {
            submitted.operation == operation && creation_matches(&submitted.request, &record)
        }) else {
            return false;
        };
        self.pending = false;
        self.error = None;
        self.result = Some((submitted.generation, record));
        true
    }

    pub fn failed(&mut self, operation: Uuid, message: String) {
        if self
            .submitted
            .as_ref()
            .is_some_and(|submitted| submitted.operation == operation)
        {
            self.pending = false;
            self.error = Some(message);
        }
    }
}

/// Creation replay may return later edited/approved review text. Only immutable
/// creation bindings are compared; the worker checks its original payload hash.
pub fn creation_matches(request: &DraftRequest, record: &ProposalRecord) -> bool {
    record.draft.id == request.id
        && record.version > 0
        && record.draft.group_id == request.group_id
        && record.draft.session_id == request.session_id
        && record.draft.sources == request.sources
        && record.draft.inbox_source == request.inbox_source
        && record.draft.changes.len() == request.changes.len()
        && record.draft.action_changes.len() == request.action_changes.len()
        && record
            .draft
            .action_changes
            .iter()
            .zip(&request.action_changes)
            .all(|(bound, requested)| {
                use brn_workflow::proposals::ActionChange;
                match (bound, requested) {
                    (
                        ActionChange::Create { id, .. },
                        ActionChange::Create { id: expected, .. },
                    ) => id == expected,
                    (
                        ActionChange::Replace { before, .. },
                        ActionChange::Replace {
                            before: expected, ..
                        },
                    ) => before == expected,
                    _ => false,
                }
            })
        && record
            .draft
            .changes
            .iter()
            .zip(&request.changes)
            .all(|(bound, requested)| match (bound, requested) {
                (
                    NoteChange::Create { path, .. },
                    DraftNoteChange::Create { path: expected, .. },
                ) => path == expected,
                (
                    NoteChange::Replace { path, before, .. },
                    DraftNoteChange::Replace {
                        path: expected,
                        expected: proof,
                        ..
                    },
                )
                | (
                    NoteChange::Trash { path, before, .. },
                    DraftNoteChange::Trash {
                        path: expected,
                        expected: proof,
                    },
                ) => path == expected && before == proof,
                _ => false,
            })
}
