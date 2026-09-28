//! Durable comments, immutable checkpoint mappings, and frozen operation receipts.
use super::*;
use crate::anchors::{AnchorProjection, OriginalAnchor, RecoveryReference};
use crate::drafts::{checked_generation, checked_text, read_draft, read_revision};
use rusqlite::Transaction;
use std::ops::Range;

pub const MAX_COMMENT_BODY_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentStatus {
    Open,
    Resolved,
}
impl CommentStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
        }
    }
    fn parse(s: &str) -> Result<Self> {
        match s {
            "open" => Ok(Self::Open),
            "resolved" => Ok(Self::Resolved),
            _ => Err(invalid("invalid comment status")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftComment {
    pub id: Uuid,
    pub draft_id: Uuid,
    pub original_revision_id: Uuid,
    pub original_sha256: [u8; 32],
    pub original_start: usize,
    pub original_end: usize,
    pub original_quote: String,
    pub body: String,
    pub status: CommentStatus,
    pub status_version: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftCommentView {
    pub comment: DraftComment,
    pub anchor: AnchorState,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftComments {
    pub draft: Draft,
    pub comments: Vec<DraftCommentView>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentAnchorSnapshot {
    pub revision: DraftRevision,
    pub anchors: Vec<(Uuid, AnchorState)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentCapture {
    pub op: Uuid,
    pub draft_id: Uuid,
    pub expected: DraftStamp,
    pub generation: u64,
    pub text: String,
    pub edits: EditTrace,
    pub range: Range<usize>,
    pub quote: String,
    pub body: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentCreated {
    pub op: Uuid,
    pub submitted_generation: u64,
    pub saved: DraftComments,
    pub comment_id: Uuid,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentStatusChange {
    pub op: Uuid,
    pub draft_id: Uuid,
    pub comment_id: Uuid,
    pub expected_status_version: u64,
    pub status: CommentStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentStatusChanged {
    pub op: Uuid,
    pub comment: DraftComment,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftWriteWithComments {
    pub op: Uuid,
    pub draft_id: Uuid,
    pub expected: DraftStamp,
    pub generation: u64,
    pub text: String,
    pub edits: EditTrace,
    pub checkpoint: bool,
}

fn checked_usize(v: i64) -> Result<usize> {
    usize::try_from(v).map_err(|_| invalid("invalid stored byte offset"))
}
fn sql_usize(v: usize) -> Result<i64> {
    i64::try_from(v).map_err(|_| invalid("byte offset exceeds SQLite range"))
}
fn checked_hash(bytes: Vec<u8>) -> Result<[u8; 32]> {
    bytes
        .try_into()
        .map_err(|_| invalid("invalid stored SHA-256 length"))
}
fn checked_body(body: &str) -> Result<()> {
    if body.len() > MAX_COMMENT_BODY_BYTES || body.trim().is_empty() {
        return Err(invalid("comment body must contain text within 64 KiB"));
    }
    Ok(())
}
fn checked_quote(text: &str, range: &Range<usize>, quote: &str) -> Result<()> {
    if quote.is_empty()
        || range.start >= range.end
        || range.end > text.len()
        || !text.is_char_boundary(range.start)
        || !text.is_char_boundary(range.end)
        || text.get(range.clone()) != Some(quote)
    {
        return Err(invalid("comment selection does not match exact quote"));
    }
    Ok(())
}
fn reason_str(reason: AmbiguityReason) -> &'static str {
    match reason {
        AmbiguityReason::Touched => "touched",
        AmbiguityReason::Duplicate => "duplicate",
        AmbiguityReason::MissingUnsupported => "missing_unsupported",
        AmbiguityReason::BoundaryAmbiguity => "boundary_ambiguity",
        AmbiguityReason::ConflictingSnapshot => "conflicting_snapshot",
        AmbiguityReason::HistoryLimit => "history_limit",
    }
}
fn parse_reason(s: &str) -> Result<AmbiguityReason> {
    match s {
        "touched" => Ok(AmbiguityReason::Touched),
        "duplicate" => Ok(AmbiguityReason::Duplicate),
        "missing_unsupported" => Ok(AmbiguityReason::MissingUnsupported),
        "boundary_ambiguity" => Ok(AmbiguityReason::BoundaryAmbiguity),
        "conflicting_snapshot" => Ok(AmbiguityReason::ConflictingSnapshot),
        "history_limit" => Ok(AmbiguityReason::HistoryLimit),
        _ => Err(invalid("invalid anchor ambiguity reason")),
    }
}
type SqlState = (&'static str, Option<&'static str>, Option<i64>, Option<i64>);
type SnapshotEntry = (String, String, Option<String>, Option<i64>, Option<i64>);
fn state_sql(state: &AnchorState) -> Result<SqlState> {
    Ok(match state {
        AnchorState::Anchored { start, end } if start < end => (
            "anchored",
            None,
            Some(sql_usize(*start)?),
            Some(sql_usize(*end)?),
        ),
        AnchorState::Anchored { .. } => return Err(invalid("invalid anchored range")),
        AnchorState::Deleted => ("deleted", None, None, None),
        AnchorState::Ambiguous { reason } => ("ambiguous", Some(reason_str(*reason)), None, None),
    })
}
fn parse_state(
    tag: String,
    reason: Option<String>,
    start: Option<i64>,
    end: Option<i64>,
) -> Result<AnchorState> {
    match (tag.as_str(), reason, start, end) {
        ("anchored", None, Some(start), Some(end)) => {
            let (start, end) = (checked_usize(start)?, checked_usize(end)?);
            if start >= end {
                return Err(invalid("invalid stored anchor range"));
            }
            Ok(AnchorState::Anchored { start, end })
        }
        ("deleted", None, None, None) => Ok(AnchorState::Deleted),
        ("ambiguous", Some(reason), None, None) => Ok(AnchorState::Ambiguous {
            reason: parse_reason(&reason)?,
        }),
        _ => Err(invalid("invalid stored anchor location")),
    }
}
fn validate_state(text: &str, quote: &str, state: &AnchorState) -> Result<()> {
    if let AnchorState::Anchored { start, end } = state {
        checked_quote(text, &(*start..*end), quote)?;
    }
    Ok(())
}
fn read_comment(conn: &Connection, id: Uuid) -> Result<DraftComment> {
    type Row = (
        String,
        String,
        Vec<u8>,
        i64,
        i64,
        String,
        String,
        String,
        i64,
    );
    let row: Row = conn.query_row("SELECT draft_id,original_revision_id,original_sha256,original_start,original_end,original_quote,body,status,status_version FROM draft_comments WHERE id=?1", [id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?)))?;
    let comment = DraftComment {
        id,
        draft_id: parse_id(row.0)?,
        original_revision_id: parse_id(row.1)?,
        original_sha256: checked_hash(row.2)?,
        original_start: checked_usize(row.3)?,
        original_end: checked_usize(row.4)?,
        original_quote: row.5,
        body: row.6,
        status: CommentStatus::parse(&row.7)?,
        status_version: u64::try_from(row.8)
            .map_err(|_| invalid("invalid stored status version"))?,
    };
    checked_body(&comment.body)?;
    let revision = read_revision(conn, comment.original_revision_id)?
        .ok_or_else(|| invalid("comment original revision missing"))?;
    if revision.draft_id != comment.draft_id
        || revision.kind != RevisionKind::Checkpoint
        || revision.sha256 != comment.original_sha256
    {
        return Err(invalid("comment original revision mismatch"));
    }
    checked_quote(
        &revision.text,
        &(comment.original_start..comment.original_end),
        &comment.original_quote,
    )?;
    Ok(comment)
}
fn read_current_anchor(
    conn: &Connection,
    comment: &DraftComment,
    draft: &Draft,
) -> Result<AnchorState> {
    type Row = (
        String,
        i64,
        Vec<u8>,
        String,
        Option<String>,
        Option<i64>,
        Option<i64>,
    );
    let row: Row = conn.query_row("SELECT target_base_revision_id,target_generation,target_sha256,location,reason,start,end FROM draft_comment_anchors WHERE comment_id=?1", [comment.id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?)))?;
    if parse_id(row.0)? != draft.stamp.base_revision
        || u64::try_from(row.1).map_err(|_| invalid("invalid anchor generation"))?
            != draft.stamp.generation
        || checked_hash(row.2)? != draft.sha256
    {
        return Err(invalid("current anchor target stamp mismatch"));
    }
    let state = parse_state(row.3, row.4, row.5, row.6)?;
    validate_state(&draft.text, &comment.original_quote, &state)?;
    Ok(state)
}
fn draft_comments_tx(conn: &Connection, id: Uuid) -> Result<DraftComments> {
    let draft = read_draft(conn, id)?.ok_or_else(|| invalid("draft does not exist"))?;
    let mut stmt =
        conn.prepare("SELECT id FROM draft_comments WHERE draft_id=?1 ORDER BY rowid")?;
    let ids: Vec<String> = stmt
        .query_map([id.to_string()], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    let mut comments = Vec::with_capacity(ids.len());
    for id in ids {
        let comment = read_comment(conn, parse_id(id)?)?;
        let anchor = read_current_anchor(conn, &comment, &draft)?;
        comments.push(DraftCommentView { comment, anchor });
    }
    Ok(DraftComments { draft, comments })
}
fn snapshot_rows(conn: &Connection, draft_id: Uuid) -> Result<Vec<CommentAnchorSnapshot>> {
    let mut stmt = conn.prepare("SELECT DISTINCT a.revision_id FROM draft_revision_comment_anchors a JOIN draft_comments c ON c.id=a.comment_id WHERE c.draft_id=?1 ORDER BY a.rowid")?;
    let ids: Vec<String> = stmt
        .query_map([draft_id.to_string()], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    let mut result = Vec::new();
    for id in ids {
        let revision = read_revision(conn, parse_id(id)?)?
            .ok_or_else(|| invalid("anchor checkpoint missing"))?;
        if revision.kind != RevisionKind::Checkpoint || revision.draft_id != draft_id {
            return Err(invalid("invalid anchor checkpoint revision"));
        }
        let mut rows = conn.prepare("SELECT comment_id,location,reason,start,end FROM draft_revision_comment_anchors WHERE revision_id=?1 ORDER BY rowid")?;
        let entries: Vec<SnapshotEntry> = rows
            .query_map([revision.id.to_string()], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .collect::<std::result::Result<_, _>>()?;
        let mut anchors = Vec::with_capacity(entries.len());
        for (id, tag, reason, start, end) in entries {
            let comment_id = parse_id(id)?;
            let comment = read_comment(conn, comment_id)?;
            if comment.draft_id != draft_id {
                return Err(invalid("checkpoint comment belongs to another draft"));
            }
            let state = parse_state(tag, reason, start, end)?;
            validate_state(&revision.text, &comment.original_quote, &state)?;
            anchors.push((comment_id, state));
        }
        result.push(CommentAnchorSnapshot { revision, anchors });
    }
    Ok(result)
}
fn projections(conn: &Connection, current: &DraftComments) -> Result<Vec<AnchorProjection>> {
    let snapshots = snapshot_rows(conn, current.draft.id)?;
    current
        .comments
        .iter()
        .map(|view| {
            let comment = &view.comment;
            let original_revision = read_revision(conn, comment.original_revision_id)?
                .ok_or_else(|| invalid("comment original missing"))?;
            let original = OriginalAnchor {
                text: original_revision.text,
                sha256: original_revision.sha256,
                range: comment.original_start..comment.original_end,
                quote: comment.original_quote.clone(),
            };
            let checkpoints = snapshots
                .iter()
                .filter_map(|snapshot| {
                    snapshot
                        .anchors
                        .iter()
                        .find(|(id, _)| *id == comment.id)
                        .map(|(_, state)| RecoveryReference {
                            text: snapshot.revision.text.clone(),
                            sha256: snapshot.revision.sha256,
                            state: state.clone(),
                        })
                })
                .collect();
            Ok(AnchorProjection {
                state: view.anchor.clone(),
                original,
                checkpoints,
            })
        })
        .collect()
}
fn put_current_anchor(
    tx: &Transaction<'_>,
    comment_id: Uuid,
    draft: &Draft,
    state: &AnchorState,
) -> Result<()> {
    let (tag, reason, start, end) = state_sql(state)?;
    tx.execute("INSERT INTO draft_comment_anchors(comment_id,target_base_revision_id,target_generation,target_sha256,location,reason,start,end) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(comment_id) DO UPDATE SET target_base_revision_id=excluded.target_base_revision_id,target_generation=excluded.target_generation,target_sha256=excluded.target_sha256,location=excluded.location,reason=excluded.reason,start=excluded.start,end=excluded.end", params![comment_id.to_string(),draft.stamp.base_revision.to_string(),checked_generation(draft.stamp.generation)?,draft.sha256.as_slice(),tag,reason,start,end])?;
    Ok(())
}
fn put_checkpoint_anchor(
    tx: &Transaction<'_>,
    comment_id: Uuid,
    revision_id: Uuid,
    state: &AnchorState,
) -> Result<()> {
    let (tag, reason, start, end) = state_sql(state)?;
    tx.execute("INSERT INTO draft_revision_comment_anchors(comment_id,revision_id,location,reason,start,end) VALUES(?1,?2,?3,?4,?5,?6)", params![comment_id.to_string(),revision_id.to_string(),tag,reason,start,end])?;
    Ok(())
}
fn endpoint_trace(before: &str, after: &str) -> EditTrace {
    EditTrace::Steps(derive_edit(before, after).into_iter().collect())
}

/// Shared transactional draft writer; every path updates current mappings and checkpoint snapshots.
pub(super) fn apply_draft_write(
    tx: &Transaction<'_>,
    id: Uuid,
    expected: DraftStamp,
    generation: u64,
    text: &str,
    checkpoint: bool,
    edits: Option<&EditTrace>,
) -> Result<Draft> {
    let digest = checked_text(text)?;
    let generation_sql = checked_generation(generation)?;
    let expected_sql = checked_generation(expected.generation)?;
    let current = draft_comments_tx(tx, id)?;
    if current.draft.stamp != expected {
        return Err(invalid("draft changed before write"));
    }
    if checkpoint {
        if generation < expected.generation
            || generation == expected.generation && text != current.draft.text
        {
            return Err(invalid(
                "checkpoint changed text without advancing generation",
            ));
        }
    } else if generation <= expected.generation {
        return Err(invalid("save generation must advance"));
    }
    let fallback;
    let trace = match edits {
        Some(trace) => trace,
        None => {
            fallback = endpoint_trace(&current.draft.text, text);
            &fallback
        }
    };
    let mapped = replay_trace(
        &current.draft.text,
        trace,
        text,
        &projections(tx, &current)?,
    )?;
    let base = if checkpoint {
        Uuid::new_v4()
    } else {
        expected.base_revision
    };
    if checkpoint {
        tx.execute("INSERT INTO draft_revisions(id,draft_id,parent_id,kind,text,sha256) VALUES(?1,?2,?3,'checkpoint',?4,?5)", params![base.to_string(),id.to_string(),expected.base_revision.to_string(),text,digest.as_slice()])?;
    }
    let changed = tx.execute("UPDATE drafts SET base_revision_id=?2,generation=?3,text=?4,sha256=?5 WHERE id=?1 AND base_revision_id=?6 AND generation=?7", params![id.to_string(),base.to_string(),generation_sql,text,digest.as_slice(),expected.base_revision.to_string(),expected_sql])?;
    if changed != 1 {
        return Err(invalid("draft changed before write"));
    }
    let draft = Draft {
        id,
        title: current.draft.title,
        stamp: DraftStamp {
            base_revision: base,
            generation,
        },
        text: text.into(),
        sha256: digest,
    };
    for (view, state) in current.comments.iter().zip(mapped.iter()) {
        validate_state(text, &view.comment.original_quote, state)?;
        put_current_anchor(tx, view.comment.id, &draft, state)?;
        if checkpoint {
            put_checkpoint_anchor(tx, view.comment.id, base, state)?;
        }
    }
    Ok(draft)
}

fn receipt_digest(op: Uuid, kind: &str, bytes: &[u8]) -> [u8; 32] {
    hash(&encode_args(&[
        b"brn.comment.receipt.v1",
        op.as_bytes(),
        kind.as_bytes(),
        bytes,
    ]))
}
fn receipt<T: serde::de::DeserializeOwned>(
    tx: &Transaction<'_>,
    op: Uuid,
    kind: &str,
) -> Result<T> {
    let (actual, bytes, stored_digest): (String, Vec<u8>, Vec<u8>) = tx
        .query_row(
            "SELECT result_kind,result_json,result_sha256 FROM comment_results WHERE operation_id=?1",
            [op.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| invalid("comment operation has no result"))?;
    if actual != kind {
        return Err(invalid("comment result kind mismatch"));
    }
    if stored_digest.as_slice() != receipt_digest(op, kind, &bytes) {
        return Err(invalid("comment receipt integrity mismatch"));
    }
    serde_json::from_slice(&bytes).map_err(|_| invalid("invalid stored comment receipt"))
}
fn finish<T: Serialize>(tx: &Transaction<'_>, op: Uuid, kind: &str, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|_| invalid("cannot encode comment receipt"))?;
    let digest = receipt_digest(op, kind, &bytes);
    tx.execute(
        "INSERT INTO comment_results(operation_id,result_kind,result_json,result_sha256) VALUES(?1,?2,?3,?4)",
        params![op.to_string(), kind, bytes, digest.as_slice()],
    )?;
    tx.execute(
        "UPDATE operations SET status='completed' WHERE id=?1",
        [op.to_string()],
    )?;
    Ok(())
}
fn request_bytes<T: Serialize>(request: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(request).map_err(|_| invalid("cannot encode comment request"))
}
fn same_immutable(a: &DraftComment, b: &DraftComment) -> bool {
    a.id == b.id
        && a.draft_id == b.draft_id
        && a.original_revision_id == b.original_revision_id
        && a.original_sha256 == b.original_sha256
        && a.original_start == b.original_start
        && a.original_end == b.original_end
        && a.original_quote == b.original_quote
        && a.body == b.body
}
fn validate_saved_draft(
    conn: &Connection,
    saved: &DraftComments,
    draft_id: Uuid,
    checkpoint: bool,
) -> Result<()> {
    if saved.draft.id != draft_id || checked_text(&saved.draft.text)? != saved.draft.sha256 {
        return Err(invalid("invalid saved draft receipt"));
    }
    let current = read_draft(conn, draft_id)?.ok_or_else(|| invalid("saved draft missing"))?;
    if saved.draft.title != current.title {
        return Err(invalid("saved draft title mismatch"));
    }
    checked_generation(saved.draft.stamp.generation)?;
    let revision = read_revision(conn, saved.draft.stamp.base_revision)?
        .ok_or_else(|| invalid("saved receipt base missing"))?;
    if revision.draft_id != draft_id || revision.kind != RevisionKind::Checkpoint {
        return Err(invalid("invalid saved receipt base"));
    }
    if checkpoint && (revision.text != saved.draft.text || revision.sha256 != saved.draft.sha256) {
        return Err(invalid("saved checkpoint content mismatch"));
    }
    let mut seen = std::collections::BTreeMap::new();
    for view in &saved.comments {
        let actual = read_comment(conn, view.comment.id)?;
        if !same_immutable(&view.comment, &actual)
            || view.comment.draft_id != draft_id
            || view.comment.status_version > actual.status_version
        {
            return Err(invalid("invalid saved comment receipt"));
        }
        validate_state(
            &saved.draft.text,
            &view.comment.original_quote,
            &view.anchor,
        )?;
        if seen.insert(view.comment.id, view.anchor.clone()).is_some() {
            return Err(invalid("duplicate comment in saved receipt"));
        }
    }
    if checkpoint {
        let snapshots = snapshot_rows(conn, draft_id)?;
        let actual: std::collections::BTreeMap<_, _> = snapshots
            .iter()
            .find(|s| s.revision.id == saved.draft.stamp.base_revision)
            .map(|s| s.anchors.iter().cloned().collect())
            .unwrap_or_default();
        if actual != seen {
            return Err(invalid("saved checkpoint mappings mismatch"));
        }
    }
    Ok(())
}

impl Store {
    pub fn draft_comments(&self, draft_id: Uuid) -> Result<DraftComments> {
        draft_comments_tx(&self.conn, draft_id)
    }
    pub fn comment_anchor_snapshots(&self, draft_id: Uuid) -> Result<Vec<CommentAnchorSnapshot>> {
        if read_draft(&self.conn, draft_id)?.is_none() {
            return Err(invalid("draft does not exist"));
        }
        snapshot_rows(&self.conn, draft_id)
    }
    pub fn create_draft_comment(&mut self, request: CommentCapture) -> Result<CommentCreated> {
        checked_body(&request.body)?;
        checked_quote(&request.text, &request.range, &request.quote)?;
        checked_text(&request.text)?;
        checked_generation(request.generation)?;
        let args = request_bytes(&request)?;
        let tx = self.conn.transaction()?;
        if workflow::bind_operation(&tx, request.op, "comment.capture", &args)?
            == BeginOperation::Existing
        {
            let saved: CommentCreated = receipt(&tx, request.op, "capture")?;
            if saved.op != request.op
                || saved.submitted_generation != request.generation
                || saved.saved.draft.id != request.draft_id
                || saved.saved.draft.text != request.text
                || saved.saved.draft.stamp.generation != request.generation
                || saved.saved.draft.stamp.base_revision == request.expected.base_revision
                || !saved.saved.comments.iter().any(|v| {
                    v.comment.id == saved.comment_id
                        && v.comment.original_revision_id == saved.saved.draft.stamp.base_revision
                        && v.comment.original_start == request.range.start
                        && v.comment.original_end == request.range.end
                        && v.comment.original_quote == request.quote
                        && v.comment.body == request.body
                        && v.anchor
                            == (AnchorState::Anchored {
                                start: request.range.start,
                                end: request.range.end,
                            })
                })
            {
                return Err(invalid("capture receipt does not match request"));
            }
            let checkpoint = read_revision(&tx, saved.saved.draft.stamp.base_revision)?
                .ok_or_else(|| invalid("capture checkpoint missing"))?;
            if checkpoint.parent_id != Some(request.expected.base_revision) {
                return Err(invalid("capture checkpoint parent mismatch"));
            }
            validate_saved_draft(&tx, &saved.saved, request.draft_id, true)?;
            return Ok(saved);
        }
        let saved_draft = apply_draft_write(
            &tx,
            request.draft_id,
            request.expected,
            request.generation,
            &request.text,
            true,
            Some(&request.edits),
        )?;
        let id = Uuid::new_v4();
        tx.execute("INSERT INTO draft_comments(id,draft_id,original_revision_id,original_sha256,original_start,original_end,original_quote,body,status,status_version) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'open',0)", params![id.to_string(),request.draft_id.to_string(),saved_draft.stamp.base_revision.to_string(),saved_draft.sha256.as_slice(),sql_usize(request.range.start)?,sql_usize(request.range.end)?,&request.quote,&request.body])?;
        let state = AnchorState::Anchored {
            start: request.range.start,
            end: request.range.end,
        };
        put_current_anchor(&tx, id, &saved_draft, &state)?;
        put_checkpoint_anchor(&tx, id, saved_draft.stamp.base_revision, &state)?;
        let saved = draft_comments_tx(&tx, request.draft_id)?;
        let result = CommentCreated {
            op: request.op,
            submitted_generation: request.generation,
            saved,
            comment_id: id,
        };
        finish(&tx, request.op, "capture", &result)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn write_draft_with_comments(
        &mut self,
        request: DraftWriteWithComments,
    ) -> Result<DraftComments> {
        checked_text(&request.text)?;
        checked_generation(request.generation)?;
        let args = request_bytes(&request)?;
        let tx = self.conn.transaction()?;
        if workflow::bind_operation(&tx, request.op, "draft.write.comments", &args)?
            == BeginOperation::Existing
        {
            let saved: DraftComments = receipt(&tx, request.op, "write")?;
            if saved.draft.id != request.draft_id
                || saved.draft.text != request.text
                || saved.draft.stamp.generation != request.generation
                || (!request.checkpoint
                    && saved.draft.stamp.base_revision != request.expected.base_revision)
            {
                return Err(invalid("saved traced write does not match request"));
            }
            if request.checkpoint {
                let revision = read_revision(&tx, saved.draft.stamp.base_revision)?
                    .ok_or_else(|| invalid("traced checkpoint missing"))?;
                if revision.parent_id != Some(request.expected.base_revision) {
                    return Err(invalid("traced checkpoint parent mismatch"));
                }
            }
            validate_saved_draft(&tx, &saved, request.draft_id, request.checkpoint)?;
            return Ok(saved);
        }
        apply_draft_write(
            &tx,
            request.draft_id,
            request.expected,
            request.generation,
            &request.text,
            request.checkpoint,
            Some(&request.edits),
        )?;
        let saved = draft_comments_tx(&tx, request.draft_id)?;
        finish(&tx, request.op, "write", &saved)?;
        tx.commit()?;
        Ok(saved)
    }
    pub fn set_comment_status(
        &mut self,
        request: CommentStatusChange,
    ) -> Result<CommentStatusChanged> {
        checked_generation(request.expected_status_version)?;
        let args = request_bytes(&request)?;
        let tx = self.conn.transaction()?;
        if workflow::bind_operation(&tx, request.op, "comment.status", &args)?
            == BeginOperation::Existing
        {
            let saved: CommentStatusChanged = receipt(&tx, request.op, "status")?;
            let actual = read_comment(&tx, request.comment_id)?;
            let max_result_version = request
                .expected_status_version
                .checked_add(1)
                .ok_or_else(|| invalid("status version overflow"))?;
            if saved.op != request.op
                || saved.comment.id != request.comment_id
                || saved.comment.draft_id != request.draft_id
                || saved.comment.status != request.status
                || !same_immutable(&saved.comment, &actual)
                || saved.comment.status_version > actual.status_version
                || saved.comment.status_version < request.expected_status_version
                || saved.comment.status_version > max_result_version
            {
                return Err(invalid("status receipt does not match request"));
            }
            return Ok(saved);
        }
        let mut comment = read_comment(&tx, request.comment_id)?;
        if comment.draft_id != request.draft_id
            || comment.status_version != request.expected_status_version
        {
            return Err(invalid("comment status changed before write"));
        }
        if comment.status != request.status {
            let version = comment
                .status_version
                .checked_add(1)
                .ok_or_else(|| invalid("comment status version overflow"))?;
            tx.execute("UPDATE draft_comments SET status=?2,status_version=?3 WHERE id=?1 AND status_version=?4", params![comment.id.to_string(),request.status.as_str(),checked_generation(version)?,checked_generation(comment.status_version)?])?;
            comment.status = request.status;
            comment.status_version = version;
        }
        let result = CommentStatusChanged {
            op: request.op,
            comment,
        };
        finish(&tx, request.op, "status", &result)?;
        tx.commit()?;
        Ok(result)
    }
}
