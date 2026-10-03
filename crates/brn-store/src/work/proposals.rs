//! Typed proposal review work. This module never performs vault I/O.
use super::{MAX_NOTE_BYTES, WorkStore, now_ms};
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
const MAX_STORED_BYTES: usize = MAX_PROPOSAL_BYTES * 6 + 256 * 1024;

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
}

impl NoteChange {
    pub fn path(&self) -> &str {
        match self {
            Self::Create { path, .. } | Self::Replace { path, .. } | Self::Trash { path, .. } => {
                path
            }
        }
    }
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Create { text, .. } | Self::Replace { text, .. } => Some(text),
            Self::Trash { .. } => None,
        }
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
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub vault: VaultRecord,
    pub title: String,
    pub changes: Vec<NoteChange>,
    pub sources: Vec<SourceVersion>,
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

/// Full replacement text for every change: Trash requires None, the others Some.
/// Bound destinations, source versions and before-text cannot be changed here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalEdit {
    pub expected: ProposalStamp,
    pub title: String,
    pub texts: Vec<Option<String>>,
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
struct StoredProposal {
    creation_sha256: [u8; 32],
    record: ProposalRecord,
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

fn nonnil(id: Uuid) -> Result<()> {
    if id.is_nil() {
        return Err(invalid("proposal review UUID must not be nil"));
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<()> {
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
    nonnil(draft.id)?;
    for id in [draft.group_id, draft.session_id].into_iter().flatten() {
        nonnil(id)?;
    }
    nonnil(draft.vault.id)?;
    let root = draft
        .vault
        .root
        .to_str()
        .ok_or_else(|| invalid("proposal vault root must be UTF-8"))?;
    if !draft.vault.root.is_absolute() || root.contains('\0') {
        return Err(invalid("proposal vault root must be an absolute path"));
    }
    validate_title(&draft.title)?;
    if draft.changes.is_empty()
        || draft.changes.len() > MAX_PROPOSAL_CHANGES
        || draft.sources.len() > MAX_PROPOSAL_CHANGES
    {
        return Err(invalid(
            "proposal needs 1 to 64 changes and at most 64 source bindings",
        ));
    }
    let mut total = 0;
    add_bytes(&mut total, root.len())?;
    add_bytes(&mut total, draft.title.len())?;
    let mut paths = HashSet::new();
    let mut identities = HashSet::new();
    for change in &draft.changes {
        validate_path(change.path())?;
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

fn validate_record(record: &ProposalRecord) -> Result<()> {
    let mut total = validate_draft(&record.draft)?;
    if record.version == 0
        || record.created_at_ms > record.updated_at_ms
        || record.comments.len() > MAX_PROPOSAL_COMMENTS
        || record.version == 1
            && (record.state != ProposalState::Draft || !record.comments.is_empty())
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
    Ok(())
}

fn read_proposal(conn: &Connection, id: Uuid) -> Result<Option<StoredProposal>> {
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

fn write_proposal(conn: &Connection, stored: &StoredProposal) -> Result<()> {
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

fn draft_at(conn: &Connection, expected: ProposalStamp) -> Result<StoredProposal> {
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

fn advance(record: &mut ProposalRecord) -> Result<()> {
    record.version = record
        .version
        .checked_add(1)
        .ok_or_else(|| invalid("proposal review version overflow"))?;
    record.updated_at_ms = now_ms().max(record.updated_at_ms);
    Ok(())
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
        let bytes = encode(&stored)?;
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
        let mut stored = draft_at(&tx, edit.expected)?;
        validate_title(&edit.title)?;
        if edit.texts.len() != stored.record.draft.changes.len() {
            return Err(invalid("proposal edit must supply every change's text"));
        }
        let mut changed = stored.record.draft.title != edit.title;
        for (index, (change, text)) in stored
            .record
            .draft
            .changes
            .iter_mut()
            .zip(&edit.texts)
            .enumerate()
        {
            match (change, text) {
                (
                    NoteChange::Create { text: old, .. } | NoteChange::Replace { text: old, .. },
                    Some(text),
                ) => {
                    if old != text {
                        changed = true;
                        for comment in &mut stored.record.comments {
                            if let CommentTarget::Text(anchor) = &comment.target
                                && anchor.change_index == index
                            {
                                comment.target = CommentTarget::Unresolved(anchor.clone());
                            }
                        }
                        *old = text.clone();
                    }
                }
                (NoteChange::Trash { .. }, None) => {}
                _ => {
                    return Err(invalid(
                        "proposal edit text does not match its typed change",
                    ));
                }
            }
        }
        if !changed {
            return Ok(stored.record);
        }
        stored.record.draft.title = edit.title.clone();
        advance(&mut stored.record)?;
        write_proposal(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.record)
    }

    /// The same exact-version operation as user editing; no AI work occurs here.
    pub fn rewrite_proposal(&mut self, edit: &ProposalEdit) -> Result<ProposalRecord> {
        self.edit_proposal(edit)
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
