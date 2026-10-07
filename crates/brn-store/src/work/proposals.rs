//! Typed proposal review work. This module never performs vault I/O.
use super::{
    MAX_NOTE_BYTES, WorkStore,
    actions::{ActionData, ActionRecord, ActionState},
    now_ms,
};
use crate::files::{FileFingerprint, VaultIdentity, VaultRecord};
use crate::{Error, Result, hash, invalid};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Component, Path},
};
use uuid::Uuid;

pub const MAX_PROPOSAL_CHANGES: usize = 64;
pub const MAX_PROPOSAL_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ASSET_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_ASSET_PROPOSAL_BYTES: usize = 32 * 1024 * 1024;
pub mod asset_payload;
pub const MAX_PROPOSAL_COMMENTS: usize = 64;
pub const MAX_COMMENT_BYTES: usize = 16 * 1024;

pub(super) const V4: &str = "
CREATE TABLE proposals (
    id TEXT PRIMARY KEY,
    group_id TEXT,
    creation_sha256 BLOB NOT NULL CHECK(length(creation_sha256)=32),
    record_json BLOB NOT NULL,
    record_sha256 BLOB NOT NULL CHECK(length(record_sha256)=32)
);
CREATE INDEX proposals_group ON proposals(group_id);";

// JSON can expand each UTF-8 byte to six bytes. Non-text DTO overhead is bounded
// by the change/source/comment counts; this limit is checked before loading it.
pub(super) const MAX_STORED_BYTES: usize = MAX_PROPOSAL_BYTES * 6 + 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NoteChange {
    Create {
        path: String,
        parent: VaultIdentity,
        text: String,
    },
    Replace {
        path: String,
        parent: VaultIdentity,
        before: FileFingerprint,
        before_text: String,
        text: String,
    },
    Trash {
        path: String,
        parent: VaultIdentity,
        before: FileFingerprint,
        before_text: String,
    },
    CreateAsset {
        path: String,
        parent: VaultIdentity,
        #[serde(with = "asset_payload")]
        bytes: Vec<u8>,
    },
    ReplaceAsset {
        path: String,
        parent: VaultIdentity,
        before: FileFingerprint,
        #[serde(with = "asset_payload")]
        before_bytes: Vec<u8>,
        #[serde(with = "asset_payload")]
        bytes: Vec<u8>,
    },
    TrashAsset {
        path: String,
        parent: VaultIdentity,
        before: FileFingerprint,
        #[serde(with = "asset_payload")]
        before_bytes: Vec<u8>,
    },
}

impl NoteChange {
    pub fn path(&self) -> &str {
        match self {
            Self::Create { path, .. }
            | Self::Replace { path, .. }
            | Self::Trash { path, .. }
            | Self::CreateAsset { path, .. }
            | Self::ReplaceAsset { path, .. }
            | Self::TrashAsset { path, .. } => path,
        }
    }
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Create { text, .. } | Self::Replace { text, .. } => Some(text),
            Self::Trash { .. }
            | Self::CreateAsset { .. }
            | Self::ReplaceAsset { .. }
            | Self::TrashAsset { .. } => None,
        }
    }
    /// Complete candidate bytes; Trash has no candidate.
    pub fn candidate_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Create { text, .. } | Self::Replace { text, .. } => Some(text.as_bytes()),
            Self::CreateAsset { bytes, .. } | Self::ReplaceAsset { bytes, .. } => Some(bytes),
            Self::Trash { .. } | Self::TrashAsset { .. } => None,
        }
    }
    pub fn before_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Replace { before_text, .. } | Self::Trash { before_text, .. } => {
                Some(before_text.as_bytes())
            }
            Self::ReplaceAsset { before_bytes, .. } | Self::TrashAsset { before_bytes, .. } => {
                Some(before_bytes)
            }
            Self::Create { .. } | Self::CreateAsset { .. } => None,
        }
    }
    pub fn before(&self) -> Option<&FileFingerprint> {
        match self {
            Self::Replace { before, .. }
            | Self::Trash { before, .. }
            | Self::ReplaceAsset { before, .. }
            | Self::TrashAsset { before, .. } => Some(before),
            Self::Create { .. } | Self::CreateAsset { .. } => None,
        }
    }
    pub fn parent(&self) -> &VaultIdentity {
        match self {
            Self::Create { parent, .. }
            | Self::Replace { parent, .. }
            | Self::Trash { parent, .. }
            | Self::CreateAsset { parent, .. }
            | Self::ReplaceAsset { parent, .. }
            | Self::TrashAsset { parent, .. } => parent,
        }
    }
    pub fn is_asset(&self) -> bool {
        matches!(
            self,
            Self::CreateAsset { .. } | Self::ReplaceAsset { .. } | Self::TrashAsset { .. }
        )
    }
    pub fn byte_limit(&self) -> usize {
        if self.is_asset() {
            MAX_ASSET_BYTES
        } else {
            MAX_NOTE_BYTES
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionChange {
    Create {
        id: Uuid,
        data: ActionData,
    },
    Replace {
        before: Box<ActionRecord>,
        data: ActionData,
    },
}

impl ActionChange {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Create { id, .. } => *id,
            Self::Replace { before, .. } => before.origin.id,
        }
    }

    pub fn data(&self) -> &ActionData {
        match self {
            Self::Create { data, .. } | Self::Replace { data, .. } => data,
        }
    }

    pub fn data_mut(&mut self) -> &mut ActionData {
        match self {
            Self::Create { data, .. } | Self::Replace { data, .. } => data,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if let Self::Replace { before, .. } = self {
            before.validate()?;
            if before.data.state == ActionState::Completed {
                return Err(invalid("completed Action cannot be replaced"));
            }
        }
        self.data().validate(self.id())?;
        if self.data().state == ActionState::Completed {
            return Err(invalid(
                "Action completion requires an identified Complete command",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceVersion {
    pub path: String,
    pub fingerprint: FileFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalDraft {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intake: Option<Box<super::inbox_actions::InboxIntakeBinding>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox_visual: Option<Box<super::inbox_visual::InboxVisualAnnotationBinding>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox_knowledge: Option<Box<super::inbox_actions::InboxKnowledgeBinding>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox_source: Option<Box<super::inbox_source::InboxSourceBinding>>,
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub vault: Option<VaultRecord>,
    pub title: String,
    pub changes: Vec<NoteChange>,
    pub sources: Vec<SourceVersion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_changes: Vec<ActionChange>,
}

impl ProposalDraft {
    pub fn inbox_analysis_id(&self) -> Option<Uuid> {
        self.intake.as_ref().and(self.group_id).or_else(|| {
            self.inbox_knowledge
                .as_ref()
                .map(|b| b.analysis_id)
                .or_else(|| self.inbox_visual.as_ref().map(|b| b.analysis_id))
        })
    }
    pub fn validate_inbox_analysis_capture(
        &self,
        job: &super::inbox_actions::InboxActionJob,
    ) -> Result<()> {
        if let Some(intake) = &self.intake {
            if self.group_id != Some(job.capture.id) {
                return Err(invalid("private Action analysis differs"));
            }
            return intake.validate_capture(job);
        }
        match (&self.inbox_knowledge, &self.inbox_visual) {
            (Some(binding), None) => binding.validate_capture(job),
            (None, Some(binding)) => binding.validate_capture(job),
            _ => Err(invalid("approval needs one exact Inbox analysis binding")),
        }
    }
    /// Preserve the literal existing knowledge-binding hash; the companion
    /// remains the same recovery family and carries no discriminator wrapper.
    pub fn inbox_analysis_binding_hash(&self) -> Result<[u8; 32]> {
        if let Some(intake) = &self.intake {
            return Ok(hash(&encode(intake)?));
        }
        let bytes = match (&self.inbox_knowledge, &self.inbox_visual) {
            (Some(binding), None) => encode(binding)?,
            (None, Some(binding)) => encode(binding)?,
            _ => return Err(invalid("approval needs one exact Inbox analysis binding")),
        };
        Ok(hash(&bytes))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalStamp {
    pub id: Uuid,
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    Draft,
    Rejected,
    Applying,
    Uncertain,
    Applied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextAnchor {
    pub change_index: usize,
    pub start: usize,
    pub end: usize,
    pub quote: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "anchor",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CommentTarget {
    Proposal,
    Text(TextAnchor),
    /// Retains the old range as review evidence; never searches for a new range.
    Unresolved(TextAnchor),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewComment {
    pub id: Uuid,
    pub text: String,
    pub target: CommentTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalRecord {
    pub draft: ProposalDraft,
    pub version: u64,
    pub state: ProposalState,
    pub comments: Vec<ReviewComment>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl ProposalRecord {
    pub fn stamp(&self) -> ProposalStamp {
        ProposalStamp {
            id: self.draft.id,
            version: self.version,
        }
    }
}

/// Full replacement text for Markdown Create/Replace; Trash/assets require None.
/// Bound destinations, source versions and before-text cannot be changed here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalEdit {
    pub expected: ProposalStamp,
    pub title: String,
    pub texts: Vec<Option<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_data: Vec<ActionData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommentRequest {
    pub expected: ProposalStamp,
    pub comment: ReviewComment,
}

/// The original creation binding is included inside the hash-checked envelope,
/// as well as in its indexed row. Editing never changes this binding.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredProposal {
    pub(super) creation_sha256: [u8; 32],
    pub(super) record: ProposalRecord,
}

struct ProposalRow {
    group: Option<String>,
    creation: Vec<u8>,
    bytes: Option<Vec<u8>>,
    digest: Vec<u8>,
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| invalid("could not encode proposal review work"))
}

pub(super) fn nonnil(id: Uuid) -> Result<()> {
    if id.is_nil() {
        return Err(invalid("proposal review UUID must not be nil"));
    }
    Ok(())
}

pub(super) fn validate_path(path: &str) -> Result<()> {
    // Same contained Markdown rules as editor work, with hidden/internal paths
    // excluded from proposal targets and source bindings.
    if path.is_empty()
        || path.contains(['\\', '\0'])
        || path
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.'))
        || Path::new(path)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || !Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        return Err(invalid(
            "proposal needs a visible contained relative Markdown path",
        ));
    }
    Ok(())
}

/// Visible, contained relative ordinary-asset path; Markdown remains separate.
pub fn validate_asset_path(path: &str) -> Result<()> {
    if path.is_empty()
        || path.contains(['\\', '\0'])
        || path
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.'))
        || Path::new(path)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        return Err(invalid(
            "asset needs a visible contained relative non-Markdown path",
        ));
    }
    Ok(())
}
fn add_asset_bytes(total: &mut usize, bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_ASSET_BYTES {
        return Err(invalid("asset payload exceeds 16 MiB"));
    }
    *total = total
        .checked_add(bytes.len())
        .ok_or_else(|| invalid("asset proposal size overflow"))?;
    if *total > MAX_ASSET_PROPOSAL_BYTES {
        return Err(invalid("asset proposal payloads exceed 32 MiB"));
    }
    Ok(())
}
fn validate_asset_before(
    before: &FileFingerprint,
    bytes: &[u8],
    total: &mut usize,
    identities: &mut HashSet<(u64, u64)>,
) -> Result<()> {
    add_asset_bytes(total, bytes)?;
    if before.len != bytes.len() as u64 || before.sha256 != hash(bytes) {
        return Err(invalid(
            "asset before fingerprint does not match exact bytes",
        ));
    }
    if !identities.insert((before.device, before.inode)) {
        return Err(invalid(
            "proposal contains repeated existing file identities",
        ));
    }
    Ok(())
}

fn add_bytes(total: &mut usize, bytes: usize) -> Result<()> {
    *total = total
        .checked_add(bytes)
        .ok_or_else(|| invalid("proposal review size overflow"))?;
    if *total > MAX_PROPOSAL_BYTES {
        return Err(invalid(
            "proposal review work exceeds the 8 MiB aggregate limit",
        ));
    }
    Ok(())
}

fn validate_title(title: &str) -> Result<()> {
    if title.is_empty() || title.len() > 512 {
        return Err(invalid("proposal title must contain 1 to 512 bytes"));
    }
    Ok(())
}

fn validate_text(text: &str, total: &mut usize) -> Result<()> {
    if text.len() > MAX_NOTE_BYTES {
        return Err(invalid("proposal note text exceeds the 1 MiB note limit"));
    }
    add_bytes(total, text.len())
}

fn validate_draft(draft: &ProposalDraft) -> Result<usize> {
    if let Some(intake) = &draft.intake {
        intake.validate()?;
        if draft.group_id.is_none()
            || draft.inbox_source.is_some()
            || draft.inbox_knowledge.is_some()
            || draft.inbox_visual.is_some()
            || !draft.changes.is_empty()
            || draft.action_changes.is_empty()
        {
            return Err(invalid(
                "private Action needs one captured analysis dependency",
            ));
        }
    }
    if let Some(binding) = &draft.inbox_knowledge {
        let text = match (binding.supersedes.as_ref(), draft.changes.as_slice()) {
            (None, [NoteChange::Create { text, .. }]) => text,
            (
                Some(_),
                [
                    NoteChange::Create { text, .. },
                    NoteChange::Replace {
                        path,
                        before,
                        before_text,
                        text: history,
                        ..
                    },
                ],
            ) => {
                binding.validate_history(path, before, before_text, history)?;
                text
            }
            _ => {
                return Err(invalid(
                    "Inbox knowledge requires one Create or an exact Create/History Replace pair",
                ));
            }
        };
        if draft.inbox_source.is_some()
            || draft.inbox_visual.is_some()
            || !draft.action_changes.is_empty()
            || draft.group_id != Some(binding.analysis_id)
        {
            return Err(invalid(
                "Inbox knowledge needs its exact analysis/Source binding",
            ));
        }
        binding.validate_sources(&draft.sources)?;
        binding.validate_text(text)?;
    }
    if let Some(binding) = &draft.inbox_source {
        if draft.inbox_visual.is_some()
            || !draft.sources.is_empty()
            || !draft.action_changes.is_empty()
        {
            return Err(invalid(
                "Inbox source conversion is separate from semantic consequences",
            ));
        }
        binding.validate_members(&draft.changes)?;
    }
    if let Some(binding) = &draft.inbox_visual {
        if draft.inbox_source.is_some()
            || draft.inbox_knowledge.is_some()
            || !draft.action_changes.is_empty()
            || draft.group_id != Some(binding.analysis_id)
            || draft.sources.as_slice() != std::slice::from_ref(&binding.source)
        {
            return Err(invalid(
                "Visual annotation needs only its exact analysis and Source",
            ));
        }
        let [
            NoteChange::Replace {
                path,
                before,
                before_text,
                text,
                ..
            },
        ] = draft.changes.as_slice()
        else {
            return Err(invalid("Visual annotation needs one exact Source Replace"));
        };
        binding.validate_replace(path, before, before_text, text)?;
    }
    nonnil(draft.id)?;
    for id in [draft.group_id, draft.session_id].into_iter().flatten() {
        nonnil(id)?;
    }
    let root = match &draft.vault {
        Some(vault) => {
            nonnil(vault.id)?;
            let root = vault
                .root
                .to_str()
                .ok_or_else(|| invalid("proposal vault root must be UTF-8"))?;
            if !vault.root.is_absolute() || root.contains('\0') {
                return Err(invalid("proposal vault root must be an absolute path"));
            }
            root
        }
        None if draft.changes.is_empty() && draft.sources.is_empty() => "",
        None => {
            return Err(invalid(
                "Markdown changes and source proofs require a vault",
            ));
        }
    };
    validate_title(&draft.title)?;
    if !draft
        .changes
        .len()
        .checked_add(draft.action_changes.len())
        .is_some_and(|count| (1..=MAX_PROPOSAL_CHANGES).contains(&count))
        || draft.sources.len() > MAX_PROPOSAL_CHANGES
    {
        return Err(invalid(
            "proposal needs 1 to 64 changes and at most 64 source bindings",
        ));
    }
    let mut total = 0;
    let mut asset_total = 0;
    if let Some(binding) = &draft.inbox_knowledge {
        add_bytes(&mut total, encode(binding)?.len())?;
    }
    if let Some(binding) = &draft.inbox_source {
        add_bytes(
            &mut total,
            serde_json::to_vec(binding)
                .map_err(|_| invalid("could not encode Inbox source binding"))?
                .len(),
        )?;
    }
    if let Some(binding) = &draft.inbox_visual {
        add_bytes(&mut total, encode(binding)?.len())?;
    }
    add_bytes(&mut total, root.len())?;
    add_bytes(&mut total, draft.title.len())?;
    let mut paths = HashSet::new();
    let mut identities = HashSet::new();
    for change in &draft.changes {
        if change.is_asset() {
            validate_asset_path(change.path())?;
        } else {
            validate_path(change.path())?;
        }
        add_bytes(&mut total, change.path().len())?;
        if !paths.insert(change.path().to_ascii_lowercase()) {
            return Err(invalid("proposal contains duplicate target paths"));
        }
        match change {
            NoteChange::Create { text, .. } => validate_text(text, &mut total)?,
            NoteChange::Replace {
                before,
                before_text,
                text,
                ..
            } => {
                validate_before(before, before_text, &mut total, &mut identities)?;
                validate_text(text, &mut total)?;
            }
            NoteChange::Trash {
                before,
                before_text,
                ..
            } => {
                validate_before(before, before_text, &mut total, &mut identities)?;
            }
            NoteChange::CreateAsset { bytes, .. } => add_asset_bytes(&mut asset_total, bytes)?,
            NoteChange::ReplaceAsset {
                before,
                before_bytes,
                bytes,
                ..
            } => {
                validate_asset_before(before, before_bytes, &mut asset_total, &mut identities)?;
                add_asset_bytes(&mut asset_total, bytes)?;
            }
            NoteChange::TrashAsset {
                before,
                before_bytes,
                ..
            } => {
                validate_asset_before(before, before_bytes, &mut asset_total, &mut identities)?;
            }
        }
    }
    for source in &draft.sources {
        validate_path(&source.path)?;
        add_bytes(&mut total, source.path.len())?;
        if source.fingerprint.len > MAX_NOTE_BYTES as u64 {
            return Err(invalid(
                "proposal source fingerprint exceeds the 1 MiB note limit",
            ));
        }
    }
    let mut action_ids = HashSet::new();
    for change in &draft.action_changes {
        change.validate()?;
        if !action_ids.insert(change.id()) {
            return Err(invalid("proposal contains duplicate Action UUIDs"));
        }
        add_bytes(&mut total, encode(change)?.len())?;
    }
    Ok(total)
}

fn validate_before(
    before: &FileFingerprint,
    text: &str,
    total: &mut usize,
    identities: &mut HashSet<(u64, u64)>,
) -> Result<()> {
    validate_text(text, total)?;
    if before.len != text.len() as u64 || before.sha256 != hash(text.as_bytes()) {
        return Err(invalid(
            "proposal before fingerprint does not match exact bytes",
        ));
    }
    if !identities.insert((before.device, before.inode)) {
        return Err(invalid(
            "proposal contains repeated existing file identities",
        ));
    }
    Ok(())
}

fn validate_anchor(anchor: &TextAnchor, draft: &ProposalDraft, exact: bool) -> Result<()> {
    let text = draft
        .changes
        .get(anchor.change_index)
        .and_then(NoteChange::text)
        .ok_or_else(|| invalid("proposal comment must target an editable change"))?;
    if anchor.quote.is_empty()
        || anchor.quote.len() > MAX_COMMENT_BYTES
        || anchor.start >= anchor.end
        || anchor.end > MAX_NOTE_BYTES
        || anchor.end - anchor.start != anchor.quote.len()
    {
        return Err(invalid(
            "proposal comment has an invalid UTF-8 range or quote",
        ));
    }
    if exact && text.get(anchor.start..anchor.end) != Some(anchor.quote.as_str()) {
        return Err(invalid("proposal comment range must match its exact quote"));
    }
    Ok(())
}

fn validate_comment(
    comment: &ReviewComment,
    draft: &ProposalDraft,
    submitted: bool,
    total: &mut usize,
) -> Result<()> {
    nonnil(comment.id)?;
    if comment.text.is_empty() || comment.text.len() > MAX_COMMENT_BYTES {
        return Err(invalid("proposal comment must contain 1 to 16 KiB of text"));
    }
    add_bytes(total, comment.text.len())?;
    match &comment.target {
        CommentTarget::Proposal => Ok(()),
        CommentTarget::Text(anchor) => {
            validate_anchor(anchor, draft, true)?;
            add_bytes(total, anchor.quote.len())
        }
        CommentTarget::Unresolved(anchor) => {
            if submitted {
                return Err(invalid(
                    "uncertain comment anchors are produced only by proposal edits",
                ));
            }
            validate_anchor(anchor, draft, false)?;
            add_bytes(total, anchor.quote.len())
        }
    }
}

pub(super) fn validate_record(record: &ProposalRecord) -> Result<()> {
    let mut total = validate_draft(&record.draft)?;
    if record.version == 0
        || record.created_at_ms > record.updated_at_ms
        || record.comments.len() > MAX_PROPOSAL_COMMENTS
        || record.version == 1
            && (record.state != ProposalState::Draft || !record.comments.is_empty())
        || matches!(
            record.state,
            ProposalState::Uncertain | ProposalState::Applied
        ) && record.version < 3
        || record.state == ProposalState::Applied && !record.comments.is_empty()
    {
        return Err(invalid(
            "stored proposal has invalid review state, version or timestamps",
        ));
    }
    let mut ids = HashSet::new();
    for comment in &record.comments {
        if !ids.insert(comment.id) {
            return Err(invalid("stored proposal has duplicate comment UUIDs"));
        }
        validate_comment(comment, &record.draft, false, &mut total)?;
    }
    super::proposal_apply::reserve_asset_review(record)?;
    Ok(())
}

pub(super) fn read_proposal(conn: &Connection, id: Uuid) -> Result<Option<StoredProposal>> {
    nonnil(id)?;
    let row: Option<ProposalRow> = conn.query_row(
        "SELECT group_id,creation_sha256,CASE WHEN length(record_json)<=?2 THEN record_json END,record_sha256 FROM proposals WHERE id=?1",
        params![id.to_string(), MAX_STORED_BYTES as i64],
        |row| Ok(ProposalRow { group: row.get(0)?, creation: row.get(1)?, bytes: row.get(2)?, digest: row.get(3)? }),
    ).optional()?;
    row.map(|row| {
        let bytes = row
            .bytes
            .ok_or_else(|| invalid("stored proposal exceeds its encoded size limit"))?;
        if row.digest.as_slice() != hash(&bytes) {
            return Err(invalid("stored proposal record failed its hash check"));
        }
        let stored: StoredProposal = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("invalid stored proposal record"))?;
        if stored.record.draft.id != id
            || stored.record.draft.group_id.map(|id| id.to_string()) != row.group
            || stored.creation_sha256.as_slice() != row.creation
        {
            return Err(invalid(
                "stored proposal differs from its bound row identity",
            ));
        }
        validate_record(&stored.record)?;
        if let Some(binding) = &stored.record.draft.inbox_knowledge {
            super::inbox_actions::check_knowledge_binding(conn, binding)?;
        }
        if let Some(binding) = &stored.record.draft.inbox_visual {
            super::inbox_visual::check_binding(conn, binding)?;
        }
        if stored.record.version == 1
            && stored.creation_sha256 != hash(&encode(&stored.record.draft)?)
        {
            return Err(invalid(
                "stored initial proposal differs from its creation binding",
            ));
        }
        Ok(stored)
    })
    .transpose()
}

pub(super) fn check_all(conn: &Connection) -> Result<()> {
    let ids = conn
        .prepare("SELECT id FROM proposals ORDER BY rowid")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for id in ids {
        let uuid = Uuid::parse_str(&id).map_err(|_| invalid("invalid stored proposal UUID"))?;
        if uuid.to_string() != id || read_proposal(conn, uuid)?.is_none() {
            return Err(invalid("invalid stored proposal row"));
        }
    }
    Ok(())
}

pub(super) fn write_proposal(conn: &Connection, stored: &StoredProposal) -> Result<()> {
    validate_record(&stored.record)?;
    let bytes = encode(stored)?;
    if bytes.len() > MAX_STORED_BYTES {
        return Err(invalid("proposal exceeds its encoded size limit"));
    }
    conn.execute(
        "UPDATE proposals SET record_json=?2,record_sha256=?3 WHERE id=?1",
        params![
            stored.record.draft.id.to_string(),
            bytes,
            hash(&bytes).as_slice()
        ],
    )?;
    Ok(())
}

pub(super) fn draft_at(conn: &Connection, expected: ProposalStamp) -> Result<StoredProposal> {
    nonnil(expected.id)?;
    let stored = read_proposal(conn, expected.id)?
        .ok_or_else(|| Error::NotFound("proposal does not exist".into()))?;
    if stored.record.stamp() != expected || stored.record.state != ProposalState::Draft {
        return Err(Error::StateChanged(
            "proposal review version or state changed".into(),
        ));
    }
    Ok(stored)
}

pub(super) fn advance(record: &mut ProposalRecord) -> Result<()> {
    record.version = record
        .version
        .checked_add(1)
        .ok_or_else(|| invalid("proposal review version overflow"))?;
    record.updated_at_ms = now_ms().max(record.updated_at_ms);
    Ok(())
}

/// Bounds a submitted full edit before cloning the current review.
pub(super) fn validate_edit(edit: &ProposalEdit) -> Result<()> {
    nonnil(edit.expected.id)?;
    if edit.expected.version == 0
        || !edit
            .texts
            .len()
            .checked_add(edit.action_data.len())
            .is_some_and(|count| count <= MAX_PROPOSAL_CHANGES)
    {
        return Err(invalid("invalid proposal edit stamp or member count"));
    }
    validate_title(&edit.title)?;
    let mut total = edit.title.len();
    for text in edit.texts.iter().flatten() {
        validate_text(text, &mut total)?;
    }
    for data in &edit.action_data {
        add_bytes(&mut total, encode(data)?.len())?;
    }
    Ok(())
}

/// Shared pure preparation for imported edits and owned Rewrite validation.
pub(super) fn edited_review(
    record: &ProposalRecord,
    edit: &ProposalEdit,
) -> Result<(ProposalRecord, bool)> {
    validate_record(record)?;
    if record.stamp() != edit.expected || record.state != ProposalState::Draft {
        return Err(Error::StateChanged(
            "proposal review version or state changed".into(),
        ));
    }
    if edit.texts.len() != record.draft.changes.len()
        || edit.action_data.len() != record.draft.action_changes.len()
    {
        return Err(invalid(
            "proposal edit must supply every typed member's data",
        ));
    }
    for (change, data) in record.draft.action_changes.iter().zip(&edit.action_data) {
        data.validate(change.id())?;
        if data.state == ActionState::Completed {
            return Err(invalid(
                "Action completion requires an identified Complete command",
            ));
        }
    }
    validate_edit(edit)?;
    let mut record = record.clone();
    let mut changed = record.draft.title != edit.title;
    for (index, (change, text)) in record.draft.changes.iter_mut().zip(&edit.texts).enumerate() {
        match (change, text) {
            (
                NoteChange::Create { text: old, .. } | NoteChange::Replace { text: old, .. },
                Some(text),
            ) => {
                crate::note_identity::protect(old, text)?;
                if old != text {
                    changed = true;
                    for comment in &mut record.comments {
                        if let CommentTarget::Text(anchor) = &comment.target
                            && anchor.change_index == index
                        {
                            comment.target = CommentTarget::Unresolved(anchor.clone());
                        }
                    }
                    *old = text.clone();
                }
            }
            (
                NoteChange::Trash { .. }
                | NoteChange::CreateAsset { .. }
                | NoteChange::ReplaceAsset { .. }
                | NoteChange::TrashAsset { .. },
                None,
            ) => {}
            _ => {
                return Err(invalid(
                    "proposal edit text does not match its typed change",
                ));
            }
        }
    }
    for (change, data) in record
        .draft
        .action_changes
        .iter_mut()
        .zip(&edit.action_data)
    {
        if change.data() != data {
            changed = true;
            *change.data_mut() = data.clone();
        }
    }
    record.draft.title = edit.title.clone();
    if changed {
        record
            .version
            .checked_add(1)
            .ok_or_else(|| invalid("proposal review version overflow"))?;
    }
    validate_record(&record)?;
    Ok((record, changed))
}

/// The caller owns the transaction, allowing Rewrite's outcome and edit to commit together.
pub(super) fn edit_in_transaction(
    conn: &Connection,
    edit: &ProposalEdit,
) -> Result<ProposalRecord> {
    let mut stored = draft_at(conn, edit.expected)?;
    let (record, changed) = edited_review(&stored.record, edit)?;
    stored.record = record;
    if changed {
        advance(&mut stored.record)?;
        write_proposal(conn, &stored)?;
    }
    Ok(stored.record)
}

impl WorkStore {
    pub fn create_proposal(&mut self, draft: &ProposalDraft) -> Result<ProposalRecord> {
        nonnil(draft.id)?;
        let tx = self.conn.transaction()?;
        if let Some(stored) = read_proposal(&tx, draft.id)? {
            if stored.creation_sha256 != hash(&encode(draft)?) {
                return Err(Error::OperationConflict(
                    "proposal UUID has another initial draft".into(),
                ));
            }
            return Ok(stored.record);
        }
        validate_draft(draft)?;
        if let Some(binding) = &draft.inbox_knowledge {
            super::inbox_actions::check_knowledge_binding(&tx, binding)?;
        }
        if let Some(binding) = &draft.inbox_visual {
            super::inbox_visual::check_binding(&tx, binding)?;
        }
        let now = now_ms();
        let stored = StoredProposal {
            creation_sha256: hash(&encode(draft)?),
            record: ProposalRecord {
                draft: draft.clone(),
                version: 1,
                state: ProposalState::Draft,
                comments: Vec::new(),
                created_at_ms: now,
                updated_at_ms: now,
            },
        };
        validate_record(&stored.record)?;
        let bytes = encode(&stored)?;
        if bytes.len() > MAX_STORED_BYTES {
            return Err(invalid("proposal exceeds its encoded size limit"));
        }
        tx.execute(
            "INSERT INTO proposals(id,group_id,creation_sha256,record_json,record_sha256) VALUES(?1,?2,?3,?4,?5)",
            params![draft.id.to_string(), draft.group_id.map(|id| id.to_string()), stored.creation_sha256.as_slice(), bytes, hash(&bytes).as_slice()],
        )?;
        tx.commit()?;
        Ok(stored.record)
    }

    pub fn proposal(&self, id: Uuid) -> Result<Option<ProposalRecord>> {
        Ok(read_proposal(&self.conn, id)?.map(|stored| stored.record))
    }

    pub fn proposals(&self, group: Option<Uuid>) -> Result<Vec<ProposalRecord>> {
        if let Some(id) = group {
            nonnil(id)?;
        }
        let mut statement = self
            .conn
            .prepare("SELECT id FROM proposals WHERE ?1 IS NULL OR group_id=?1 ORDER BY rowid")?;
        statement
            .query_map([group.map(|id| id.to_string())], |row| {
                row.get::<_, String>(0)
            })?
            .map(|id| {
                read_proposal(&self.conn, crate::parse_id(id?)?)?
                    .map(|stored| stored.record)
                    .ok_or_else(|| invalid("listed proposal disappeared"))
            })
            .collect()
    }

    pub fn edit_proposal(&mut self, edit: &ProposalEdit) -> Result<ProposalRecord> {
        let tx = self.conn.transaction()?;
        let record = edit_in_transaction(&tx, edit)?;
        tx.commit()?;
        Ok(record)
    }

    /// An exact-version Rewrite result, preserving managed metadata before editing.
    /// No AI work occurs here; ordinary owner edits use `edit_proposal`.
    pub fn rewrite_proposal(&mut self, edit: &ProposalEdit) -> Result<ProposalRecord> {
        let tx = self.conn.transaction()?;
        let stored = draft_at(&tx, edit.expected)?;
        super::proposal_rewrite::validate_rewrite_result(&stored.record, edit)?;
        let record = edit_in_transaction(&tx, edit)?;
        tx.commit()?;
        Ok(record)
    }

    pub fn add_proposal_comment(&mut self, request: &CommentRequest) -> Result<ProposalRecord> {
        let tx = self.conn.transaction()?;
        let mut stored = draft_at(&tx, request.expected)?;
        if stored
            .record
            .comments
            .iter()
            .any(|comment| comment.id == request.comment.id)
        {
            return Err(Error::OperationConflict(
                "proposal comment UUID already exists".into(),
            ));
        }
        validate_comment(&request.comment, &stored.record.draft, true, &mut 0)?;
        stored.record.comments.push(request.comment.clone());
        advance(&mut stored.record)?;
        write_proposal(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.record)
    }

    pub fn update_proposal_comment(&mut self, request: &CommentRequest) -> Result<ProposalRecord> {
        let tx = self.conn.transaction()?;
        let mut stored = draft_at(&tx, request.expected)?;
        validate_comment(&request.comment, &stored.record.draft, true, &mut 0)?;
        let comment = stored
            .record
            .comments
            .iter_mut()
            .find(|comment| comment.id == request.comment.id)
            .ok_or_else(|| Error::NotFound("proposal comment does not exist".into()))?;
        if *comment == request.comment {
            return Ok(stored.record);
        }
        *comment = request.comment.clone();
        advance(&mut stored.record)?;
        write_proposal(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.record)
    }

    pub fn remove_proposal_comment(
        &mut self,
        expected: ProposalStamp,
        id: Uuid,
    ) -> Result<ProposalRecord> {
        nonnil(id)?;
        let tx = self.conn.transaction()?;
        let mut stored = draft_at(&tx, expected)?;
        let index = stored
            .record
            .comments
            .iter()
            .position(|comment| comment.id == id)
            .ok_or_else(|| Error::NotFound("proposal comment does not exist".into()))?;
        stored.record.comments.remove(index);
        advance(&mut stored.record)?;
        write_proposal(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.record)
    }

    pub fn reject_proposal(&mut self, expected: ProposalStamp) -> Result<ProposalRecord> {
        let tx = self.conn.transaction()?;
        let mut stored = draft_at(&tx, expected)?;
        stored.record.state = ProposalState::Rejected;
        advance(&mut stored.record)?;
        write_proposal(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.record)
    }
}
