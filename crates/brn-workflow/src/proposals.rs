//! Full typed review drafts over WorkStore. These operations never apply notes.
use crate::{ErrorKind, Result, WorkflowError, app::App, editor::file_error, vault::VaultPath};
use brn_store::files::{FileFingerprint, VaultRecord};
pub use brn_store::work::proposals::{
    CommentRequest, CommentTarget, MAX_COMMENT_BYTES, MAX_PROPOSAL_BYTES, MAX_PROPOSAL_CHANGES,
    MAX_PROPOSAL_COMMENTS, NoteChange, ProposalDraft, ProposalEdit, ProposalRecord, ProposalStamp,
    ProposalState, ReviewComment, SourceVersion, TextAnchor,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DraftNoteChange {
    Create {
        path: String,
        text: String,
    },
    Replace {
        path: String,
        expected: FileFingerprint,
        text: String,
    },
    Trash {
        path: String,
        expected: FileFingerprint,
    },
}

impl DraftNoteChange {
    fn path(&self) -> &str {
        match self {
            Self::Create { path, .. } | Self::Replace { path, .. } | Self::Trash { path, .. } => {
                path
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRequest {
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub title: String,
    pub changes: Vec<DraftNoteChange>,
    pub sources: Vec<SourceVersion>,
}

/// Pure validation of a complete review edit before a frontend queues it.
pub fn validate_review_edit(record: &ProposalRecord, edit: &ProposalEdit) -> Result<()> {
    Ok(brn_store::work::proposal_rewrite::validate_result(
        record, edit,
    )?)
}

fn invalid(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, message)
}

fn path_check(path: &str) -> Result<()> {
    VaultPath::parse(path).map_err(|_| invalid("proposal needs a contained Markdown path"))?;
    if path.split('/').any(|part| part.starts_with('.')) {
        return Err(invalid(
            "proposal targets and sources must not use hidden/internal paths",
        ));
    }
    Ok(())
}

impl DraftRequest {
    /// Syntactic preparation before either frontend admits operational work.
    pub fn validate(&self) -> Result<()> {
        if self.id.is_nil()
            || self.group_id.is_some_and(|id| id.is_nil())
            || self.session_id.is_some_and(|id| id.is_nil())
            || self.title.trim().is_empty()
            || self.title.len() > 512
            || self.changes.is_empty()
            || self.changes.len() > MAX_PROPOSAL_CHANGES
            || self.sources.len() > MAX_PROPOSAL_CHANGES
        {
            return Err(invalid("invalid proposal identity, title or member count"));
        }
        let mut bytes = self.title.len();
        let mut paths = std::collections::HashSet::new();
        for change in &self.changes {
            let path = change.path();
            path_check(path)?;
            if !paths.insert(path.to_ascii_lowercase()) {
                return Err(invalid("proposal contains duplicate destinations"));
            }
            bytes = bytes.saturating_add(path.len());
            match change {
                DraftNoteChange::Create { text, .. } | DraftNoteChange::Replace { text, .. } => {
                    if text.len() > crate::MAX_NOTE_BYTES {
                        return Err(invalid("proposed note exceeds 1 MiB"));
                    }
                    bytes = bytes.saturating_add(text.len());
                }
                DraftNoteChange::Trash { .. } => {}
            }
            match change {
                DraftNoteChange::Replace { expected, .. }
                | DraftNoteChange::Trash { expected, .. }
                    if expected.len > crate::MAX_NOTE_BYTES as u64 =>
                {
                    return Err(invalid("expected note exceeds 1 MiB"));
                }
                _ => {}
            }
        }
        let mut sources = std::collections::HashSet::new();
        for source in &self.sources {
            path_check(&source.path)?;
            if !sources.insert(source.path.to_ascii_lowercase())
                || source.fingerprint.len > crate::MAX_NOTE_BYTES as u64
            {
                return Err(invalid("invalid or duplicate proposal source"));
            }
            bytes = bytes.saturating_add(source.path.len());
        }
        if bytes > MAX_PROPOSAL_BYTES {
            return Err(invalid("proposal review work exceeds 8 MiB"));
        }
        Ok(())
    }
}

impl App {
    pub fn create_proposal(&mut self, request: &DraftRequest) -> Result<ProposalRecord> {
        request.validate()?;
        // Creation replay binds the original input before any fresh-vault check.
        // Edits retain these immutable destinations/before/source/parent bindings.
        if let Some(existing) = self.store.proposal(request.id)? {
            let conflict = || {
                WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "proposal UUID has another creation payload",
                )
            };
            if request.changes.len() != existing.draft.changes.len() {
                return Err(conflict());
            }
            let mut draft = existing.draft;
            for (input, bound) in request.changes.iter().zip(&mut draft.changes) {
                match (input, bound) {
                    (
                        DraftNoteChange::Create { path, text },
                        NoteChange::Create {
                            path: old,
                            text: out,
                            ..
                        },
                    ) if path == old => *out = text.clone(),
                    (
                        DraftNoteChange::Replace {
                            path,
                            expected,
                            text,
                        },
                        NoteChange::Replace {
                            path: old,
                            before,
                            text: out,
                            ..
                        },
                    ) if path == old && expected == before => *out = text.clone(),
                    (
                        DraftNoteChange::Trash { path, expected },
                        NoteChange::Trash {
                            path: old, before, ..
                        },
                    ) if path == old && expected == before => {}
                    _ => return Err(conflict()),
                }
            }
            draft.group_id = request.group_id;
            draft.session_id = request.session_id;
            draft.title = request.title.clone();
            draft.sources = request.sources.clone();
            return Ok(self.store.create_proposal(&draft)?);
        }
        self.require_current_evidence()?;
        if let Some(session) = request.session_id
            && !self
                .store
                .conversations()?
                .iter()
                .any(|item| item.id == session)
        {
            return Err(WorkflowError::typed(
                ErrorKind::NotFound,
                "proposal session does not exist",
            ));
        }
        self.editor_files()?;
        let vault: VaultRecord = serde_json::from_str(
            &self
                .store
                .setting("vault.editor_identity")?
                .ok_or_else(|| WorkflowError::msg("vault identity is unavailable"))?,
        )
        .map_err(|_| WorkflowError::msg("invalid saved vault identity"))?;
        let files = self.editor_files()?;
        let mut changes = Vec::with_capacity(request.changes.len());
        for (index, input) in request.changes.iter().enumerate() {
            for previous in &request.changes[..index] {
                if files
                    .reserved_copy_path_matches(Path::new(input.path()), Path::new(previous.path()))
                    .map_err(file_error)?
                {
                    return Err(invalid("proposal destinations alias the same namespace"));
                }
            }
            let path = Path::new(input.path());
            let change = files
                .coordinate(path, || {
                    Ok((|| -> Result<NoteChange> {
                        let parent = files.parent_identity(path).map_err(file_error)?;
                        match input {
                            DraftNoteChange::Create { path: name, text } => {
                                files.validate_copy_destination(path).map_err(file_error)?;
                                if files.artifact(path).map_err(file_error)?.is_some() {
                                    return Err(WorkflowError::typed(
                                        ErrorKind::ContextStale,
                                        "proposed creation destination is occupied",
                                    ));
                                }
                                Ok(NoteChange::Create {
                                    path: name.clone(),
                                    parent,
                                    text: text.clone(),
                                })
                            }
                            DraftNoteChange::Replace {
                                path: name,
                                expected,
                                text,
                            } => {
                                let before =
                                    files.observe_uncoordinated(path).map_err(file_error)?;
                                if before.fingerprint != *expected {
                                    return Err(WorkflowError::typed(
                                        ErrorKind::ContextStale,
                                        "proposed replacement baseline changed",
                                    ));
                                }
                                Ok(NoteChange::Replace {
                                    path: name.clone(),
                                    parent,
                                    before: before.fingerprint,
                                    before_text: before.text,
                                    text: text.clone(),
                                })
                            }
                            DraftNoteChange::Trash {
                                path: name,
                                expected,
                            } => {
                                let before =
                                    files.observe_uncoordinated(path).map_err(file_error)?;
                                if before.fingerprint != *expected {
                                    return Err(WorkflowError::typed(
                                        ErrorKind::ContextStale,
                                        "proposed Trash baseline changed",
                                    ));
                                }
                                Ok(NoteChange::Trash {
                                    path: name.clone(),
                                    parent,
                                    before: before.fingerprint,
                                    before_text: before.text,
                                })
                            }
                        }
                    })())
                })
                .map_err(file_error)??;
            changes.push(change);
        }
        for source in &request.sources {
            let before = files.observe(Path::new(&source.path)).map_err(file_error)?;
            if before.fingerprint != source.fingerprint {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "proposal source version changed",
                ));
            }
        }
        Ok(self.store.create_proposal(&ProposalDraft {
            id: request.id,
            group_id: request.group_id,
            session_id: request.session_id,
            vault,
            title: request.title.clone(),
            changes,
            sources: request.sources.clone(),
        })?)
    }

    pub fn proposal(&self, id: Uuid) -> Result<ProposalRecord> {
        self.store
            .proposal(id)?
            .ok_or_else(|| WorkflowError::typed(ErrorKind::NotFound, "proposal does not exist"))
    }
    pub fn proposals(&self, group: Option<Uuid>) -> Result<Vec<ProposalRecord>> {
        Ok(self.store.proposals(group)?)
    }
    pub fn edit_proposal(&mut self, edit: &ProposalEdit) -> Result<ProposalRecord> {
        Ok(self.store.edit_proposal(edit)?)
    }
    pub fn rewrite_proposal(&mut self, result: &ProposalEdit) -> Result<ProposalRecord> {
        Ok(self.store.rewrite_proposal(result)?)
    }
    pub fn add_proposal_comment(&mut self, request: &CommentRequest) -> Result<ProposalRecord> {
        Ok(self.store.add_proposal_comment(request)?)
    }
    pub fn update_proposal_comment(&mut self, request: &CommentRequest) -> Result<ProposalRecord> {
        Ok(self.store.update_proposal_comment(request)?)
    }
    pub fn remove_proposal_comment(
        &mut self,
        stamp: ProposalStamp,
        comment: Uuid,
    ) -> Result<ProposalRecord> {
        Ok(self.store.remove_proposal_comment(stamp, comment)?)
    }
    pub fn reject_proposal(&mut self, stamp: ProposalStamp) -> Result<ProposalRecord> {
        Ok(self.store.reject_proposal(stamp)?)
    }
}
