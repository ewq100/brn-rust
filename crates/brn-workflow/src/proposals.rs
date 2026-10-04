//! Full typed review drafts over WorkStore. These operations never apply notes.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    editor::file_error,
    vault::{EvidencePath, VaultPath},
};
use brn_store::files::{FileFingerprint, VaultRecord};
pub use brn_store::work::proposals::{
    ActionChange, CommentRequest, CommentTarget, MAX_COMMENT_BYTES, MAX_PROPOSAL_BYTES,
    MAX_PROPOSAL_CHANGES, MAX_PROPOSAL_COMMENTS, NoteChange, ProposalDraft, ProposalEdit,
    ProposalRecord, ProposalStamp, ProposalState, ReviewComment, SourceVersion, TextAnchor,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_changes: Vec<ActionChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalSource {
    pub source: SourceVersion,
    pub text: String,
}

impl ProposalSource {
    /// Check a complete returned proof without observing files or changing authority.
    pub fn validate(&self) -> Result<()> {
        EvidencePath::parse(&self.source.path)
            .map_err(|_| invalid("invalid captured proposal evidence path"))?;
        if self.text.len() > crate::MAX_NOTE_BYTES
            || self.source.fingerprint.len != self.text.len() as u64
            || self.source.fingerprint.sha256
                != <[u8; 32]>::from(Sha256::digest(self.text.as_bytes()))
        {
            return Err(invalid(
                "captured source bytes do not match their complete fingerprint",
            ));
        }
        Ok(())
    }
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
            || !self
                .changes
                .len()
                .checked_add(self.action_changes.len())
                .is_some_and(|count| (1..=MAX_PROPOSAL_CHANGES).contains(&count))
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
            EvidencePath::parse(&source.path).map_err(|_| {
                invalid("proposal source needs a contained visible Markdown evidence path")
            })?;
            if !sources.insert(source.path.to_ascii_lowercase())
                || source.fingerprint.len > crate::MAX_NOTE_BYTES as u64
            {
                return Err(invalid("invalid or duplicate proposal source"));
            }
            bytes = bytes.saturating_add(source.path.len());
        }
        let mut action_ids = std::collections::HashSet::new();
        for change in &self.action_changes {
            change
                .validate()
                .map_err(|error| invalid(&error.to_string()))?;
            if !action_ids.insert(change.id()) {
                return Err(invalid("proposal contains duplicate Action UUIDs"));
            }
            bytes = bytes.saturating_add(
                serde_json::to_vec(change)
                    .map_err(|_| invalid("could not encode Action review work"))?
                    .len(),
            );
        }
        if bytes > MAX_PROPOSAL_BYTES {
            return Err(invalid("proposal review work exceeds 8 MiB"));
        }
        Ok(())
    }
}

impl App {
    /// Captures full saved source bytes without creating editor or review work.
    pub fn proposal_source(&mut self, path: &str) -> Result<ProposalSource> {
        path_check(path)?;
        self.proposal_evidence_source(path)
    }

    /// Read-only full source capture, including archived evidence. This does not
    /// expand destination authority or create an editor/review record.
    pub(crate) fn proposal_evidence_source(&mut self, path: &str) -> Result<ProposalSource> {
        EvidencePath::parse(path)
            .map_err(|_| invalid("source needs a contained visible Markdown evidence path"))?;
        self.require_current_evidence()?;
        let observed = self
            .editor_files()?
            .observe(Path::new(path))
            .map_err(file_error)?;
        Ok(ProposalSource {
            source: SourceVersion {
                path: path.to_owned(),
                fingerprint: observed.fingerprint,
            },
            text: observed.text,
        })
    }

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
            if request.changes.len() != existing.draft.changes.len()
                || request.action_changes.len() != existing.draft.action_changes.len()
            {
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
            for (input, bound) in request.action_changes.iter().zip(&mut draft.action_changes) {
                match (input, bound) {
                    (
                        ActionChange::Create { id, data },
                        ActionChange::Create { id: old, data: out },
                    ) if id == old => *out = data.clone(),
                    (
                        ActionChange::Replace { before, data },
                        ActionChange::Replace {
                            before: old,
                            data: out,
                        },
                    ) if before == old => *out = data.clone(),
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
        let vault = if !request.changes.is_empty() || !request.sources.is_empty() {
            self.editor_files()?;
            Some(
                serde_json::from_str::<VaultRecord>(
                    &self
                        .store
                        .setting("vault.editor_identity")?
                        .ok_or_else(|| WorkflowError::msg("vault identity is unavailable"))?,
                )
                .map_err(|_| WorkflowError::msg("invalid saved vault identity"))?,
            )
        } else {
            None
        };
        let files = if vault.is_some() {
            Some(self.editor_files()?)
        } else {
            None
        };
        let mut changes = Vec::with_capacity(request.changes.len());
        for (index, input) in request.changes.iter().enumerate() {
            let files = files.ok_or_else(|| invalid("Markdown changes require a vault"))?;
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
                                // A known logical identity survives ordinary replacement.
                                // Malformed old metadata may be explicitly repaired through
                                // complete reviewed bytes; it never supplies a guessed ID.
                                if let Ok(Some(existing_id)) =
                                    brn_store::note_identity::read(&before.text)
                                {
                                    let proposed_id = brn_store::note_identity::read(text)
                                        .map_err(|error| invalid(&error.to_string()))?;
                                    if proposed_id != Some(existing_id) {
                                        return Err(invalid("proposed replacement must preserve the note's managed identity"));
                                    }
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
            let files = files.ok_or_else(|| invalid("source evidence requires a vault"))?;
            let before = files.observe(Path::new(&source.path)).map_err(file_error)?;
            if before.fingerprint != source.fingerprint {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "proposal source version changed",
                ));
            }
        }
        let draft = ProposalDraft {
            action_changes: request.action_changes.clone(),
            id: request.id,
            group_id: request.group_id,
            session_id: request.session_id,
            vault,
            title: request.title.clone(),
            changes,
            sources: request.sources.clone(),
        };
        self.store
            .validate_proposal_action_changes(&draft.action_changes)?;
        self.validate_action_references(&draft)?;
        Ok(self.store.create_proposal(&draft)?)
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
