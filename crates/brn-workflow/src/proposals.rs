//! Full typed review drafts over WorkStore. These operations never apply notes.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    editor::file_error,
    vault::{EvidencePath, VaultPath},
};
use brn_store::files::{FileFingerprint, VaultRecord};
pub use brn_store::work::proposals::{
    ActionChange, CommentRequest, CommentTarget, MAX_ASSET_BYTES, MAX_ASSET_PROPOSAL_BYTES,
    MAX_COMMENT_BYTES, MAX_PROPOSAL_BYTES, MAX_PROPOSAL_CHANGES, MAX_PROPOSAL_COMMENTS, NoteChange,
    ProposalDraft, ProposalEdit, ProposalRecord, ProposalStamp, ProposalState, ReviewComment,
    SourceVersion, TextAnchor,
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
    CreateAsset {
        path: String,
        #[serde(with = "brn_store::work::proposals::asset_payload")]
        bytes: Vec<u8>,
    },
    ReplaceAsset {
        path: String,
        expected: FileFingerprint,
        #[serde(with = "brn_store::work::proposals::asset_payload")]
        bytes: Vec<u8>,
    },
    TrashAsset {
        path: String,
        expected: FileFingerprint,
    },
}

impl DraftNoteChange {
    fn path(&self) -> &str {
        match self {
            Self::Create { path, .. }
            | Self::Replace { path, .. }
            | Self::Trash { path, .. }
            | Self::CreateAsset { path, .. }
            | Self::ReplaceAsset { path, .. }
            | Self::TrashAsset { path, .. } => path,
        }
    }

    fn is_asset(&self) -> bool {
        matches!(
            self,
            Self::CreateAsset { .. } | Self::ReplaceAsset { .. } | Self::TrashAsset { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox_visual: Option<Box<brn_store::work::inbox_visual::InboxVisualAnnotationBinding>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox_knowledge: Option<Box<brn_store::work::inbox_actions::InboxKnowledgeBinding>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox_source: Option<Box<brn_store::work::inbox_source::InboxSourceBinding>>,
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

/// Fresh complete byte proof for an ordinary asset, without exposing its payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalAsset {
    pub path: String,
    pub fingerprint: FileFingerprint,
}

impl ProposalAsset {
    pub fn validate(&self) -> Result<()> {
        validate_asset_path(&self.path)?;
        if self.fingerprint.len > MAX_ASSET_BYTES as u64
            || self.fingerprint.device == 0
            || self.fingerprint.inode == 0
        {
            return Err(invalid("invalid complete asset proof"));
        }
        Ok(())
    }
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

/// Pure current asset destination validation; this grants no filesystem proof.
pub fn validate_asset_path(path: &str) -> Result<()> {
    brn_store::work::proposals::validate_asset_path(path)?;
    VaultPath::validate_folder(path)
        .map_err(|_| invalid("asset needs a contained current vault path"))
}

impl DraftRequest {
    /// Syntactic preparation before either frontend admits operational work.
    pub fn validate(&self) -> Result<()> {
        if let Some(binding) = &self.inbox_knowledge {
            let text = match (binding.supersedes.as_ref(), self.changes.as_slice()) {
                (None, [DraftNoteChange::Create { text, .. }]) => text,
                (
                    Some(bound),
                    [
                        DraftNoteChange::Create { text, .. },
                        DraftNoteChange::Replace { path, expected, .. },
                    ],
                ) if path == &bound.source.path && expected == &bound.source.fingerprint => text,
                _ => {
                    return Err(invalid(
                        "Inbox knowledge needs its exact Create or Create/History pair",
                    ));
                }
            };
            if self.inbox_source.is_some()
                || self.inbox_visual.is_some()
                || !self.action_changes.is_empty()
                || self.group_id != Some(binding.analysis_id)
            {
                return Err(invalid(
                    "Inbox knowledge needs its exact analysis/Source binding",
                ));
            }
            binding.validate_sources(&self.sources)?;
            binding.validate_text(text)?;
        }
        if let Some(binding) = &self.inbox_source {
            let (path, text, asset) = match self.changes.as_slice() {
                [DraftNoteChange::Create { path, text }] if binding.visual.is_none() => {
                    (path, text, None)
                }
                [
                    DraftNoteChange::Create { path, text },
                    DraftNoteChange::CreateAsset {
                        path: asset_path,
                        bytes,
                    },
                ] if binding.visual.is_some() => (path, text, Some((asset_path, bytes))),
                _ => {
                    return Err(invalid(
                        "Inbox Source needs its exact Create and optional bound PNG Create",
                    ));
                }
            };
            if self.inbox_visual.is_some()
                || !self.sources.is_empty()
                || !self.action_changes.is_empty()
            {
                return Err(invalid(
                    "Inbox source conversion is separate from semantic consequences",
                ));
            }
            binding.validate_markdown(text)?;
            if let Some((asset_path, bytes)) = asset {
                binding.validate_asset(path, asset_path, bytes)?;
            }
        }
        if let Some(binding) = &self.inbox_visual {
            if self.inbox_source.is_some()
                || self.inbox_knowledge.is_some()
                || !self.action_changes.is_empty()
                || self.group_id != Some(binding.analysis_id)
                || self.sources.as_slice() != std::slice::from_ref(&binding.source)
            {
                return Err(invalid(
                    "Visual annotation needs its exact analysis and Source",
                ));
            }
            let [
                DraftNoteChange::Replace {
                    path,
                    expected,
                    text,
                },
            ] = self.changes.as_slice()
            else {
                return Err(invalid("Visual annotation needs one Source Replace"));
            };
            binding.validate_replace(path, expected, &binding.source_text, text)?;
        }
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
        let mut asset_bytes = 0usize;
        let mut paths = std::collections::HashSet::new();
        for change in &self.changes {
            let path = change.path();
            if change.is_asset() {
                validate_asset_path(path)?;
            } else {
                path_check(path)?;
            }
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
                DraftNoteChange::CreateAsset { bytes, .. }
                | DraftNoteChange::ReplaceAsset { bytes, .. } => {
                    if bytes.len() > MAX_ASSET_BYTES {
                        return Err(invalid("proposed asset exceeds 16 MiB"));
                    }
                    asset_bytes = asset_bytes.saturating_add(bytes.len());
                }
                DraftNoteChange::TrashAsset { .. } => {}
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
            if let DraftNoteChange::ReplaceAsset { expected, .. }
            | DraftNoteChange::TrashAsset { expected, .. } = change
            {
                if expected.len > MAX_ASSET_BYTES as u64 {
                    return Err(invalid("expected asset exceeds 16 MiB"));
                }
                asset_bytes = asset_bytes.saturating_add(expected.len as usize);
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
        if asset_bytes > MAX_ASSET_PROPOSAL_BYTES {
            return Err(invalid("proposal asset payloads exceed 32 MiB"));
        }
        Ok(())
    }
}

impl App {
    /// Read-only proof capture for a current ordinary asset. This grants no
    /// source/retrieval identity, conversion, approval or filesystem authority.
    pub fn proposal_asset(&mut self, path: &str) -> Result<ProposalAsset> {
        validate_asset_path(path)?;
        self.require_current_evidence()?;
        let observed = self
            .editor_files()?
            .observe_asset(Path::new(path))
            .map_err(file_error)?;
        let asset = ProposalAsset {
            path: path.into(),
            fingerprint: observed.fingerprint,
        };
        asset.validate()?;
        Ok(asset)
    }

    /// Captures full saved source bytes without creating editor or review work.
    pub fn proposal_source(&mut self, path: &str) -> Result<ProposalSource> {
        path_check(path)?;
        self.proposal_evidence_source(path)
    }

    /// Read-only full source capture, including archived evidence. This does not
    /// expand destination authority or create an editor/review record.
    pub fn proposal_evidence_source(&mut self, path: &str) -> Result<ProposalSource> {
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
                    (
                        DraftNoteChange::CreateAsset { path, bytes },
                        NoteChange::CreateAsset {
                            path: old,
                            bytes: out,
                            ..
                        },
                    ) if path == old && bytes == out => {}
                    (
                        DraftNoteChange::ReplaceAsset {
                            path,
                            expected,
                            bytes,
                        },
                        NoteChange::ReplaceAsset {
                            path: old,
                            before,
                            bytes: out,
                            ..
                        },
                    ) if path == old && expected == before && bytes == out => {}
                    (
                        DraftNoteChange::TrashAsset { path, expected },
                        NoteChange::TrashAsset {
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
            draft.inbox_source = request.inbox_source.clone();
            draft.inbox_knowledge = request.inbox_knowledge.clone();
            draft.inbox_visual = request.inbox_visual.clone();
            return Ok(self.store.create_proposal(&draft)?);
        }
        self.require_current_evidence()?;
        self.validate_inbox_source(request.inbox_source.as_deref())?;
        self.validate_inbox_knowledge(request.inbox_knowledge.as_deref())?;
        self.validate_inbox_visual(request.inbox_visual.as_deref())?;
        if let Some(binding) = &request.inbox_source {
            let preview =
                self.inbox_candidate(&crate::inbox_processing::InboxCandidateRequest {
                    batch_id: binding.batch_id,
                    index: binding.index,
                })?;
            if preview.original != binding.original
                || preview.format != binding.format
                || preview.markdown.len() as u64 != binding.byte_len
                || <[u8; 32]>::from(Sha256::digest(preview.markdown.as_bytes())) != binding.sha256
            {
                return Err(invalid(
                    "Inbox source differs from its completed conversion receipt",
                ));
            }
            self.check_inbox_source_identity(binding.note_id)?;
        }
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
            let files = files.ok_or_else(|| invalid("file changes require a vault"))?;
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
                            DraftNoteChange::CreateAsset { path: name, bytes } => {
                                files.validate_copy_destination(path).map_err(file_error)?;
                                if files.artifact(path).map_err(file_error)?.is_some() {
                                    return Err(WorkflowError::typed(ErrorKind::ContextStale, "proposed asset destination is occupied"));
                                }
                                Ok(NoteChange::CreateAsset { path: name.clone(), parent, bytes: bytes.clone() })
                            }
                            DraftNoteChange::ReplaceAsset { path: name, expected, bytes } => {
                                let before = files.observe_asset_uncoordinated(path).map_err(file_error)?;
                                if before.fingerprint != *expected {
                                    return Err(WorkflowError::typed(ErrorKind::ContextStale, "proposed asset replacement baseline changed"));
                                }
                                Ok(NoteChange::ReplaceAsset { path: name.clone(), parent, before: before.fingerprint, before_bytes: before.bytes, bytes: bytes.clone() })
                            }
                            DraftNoteChange::TrashAsset { path: name, expected } => {
                                let before = files.observe_asset_uncoordinated(path).map_err(file_error)?;
                                if before.fingerprint != *expected {
                                    return Err(WorkflowError::typed(ErrorKind::ContextStale, "proposed asset Trash baseline changed"));
                                }
                                Ok(NoteChange::TrashAsset { path: name.clone(), parent, before: before.fingerprint, before_bytes: before.bytes })
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
            inbox_knowledge: request.inbox_knowledge.clone(),
            inbox_visual: request.inbox_visual.clone(),
            inbox_source: request.inbox_source.clone(),
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
