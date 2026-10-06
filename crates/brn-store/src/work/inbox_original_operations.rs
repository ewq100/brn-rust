//! Checked versioned Remove/Restore evidence. These imports restore authority,
//! never inspect a vault, grant fresh admission, or initiate filesystem effects.
pub use super::inbox_original_legacy as legacy;
use super::{
    WorkStore, inbox, inbox_actions, inbox_source::InboxSourcePreservation,
    proposal_apply::ApplyJournal,
};
use crate::{Error, Result, hash, invalid};
pub use legacy::{
    InboxOriginalNamespace, InboxOriginalOperationKind, InboxOriginalOperationSummary,
    InboxQualifiedOriginal, InboxSourceProof,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;

pub const MAX_ORIGINAL_OPERATION_BYTES: usize = legacy::MAX_ORIGINAL_OPERATION_BYTES;
/// Three worst-case JSON-escaped 1 MiB texts and bounded journal metadata.
pub const MAX_PRESERVATION_EVIDENCE_BYTES: usize = 18 * super::MAX_NOTE_BYTES + 256 * 1024;
pub const MAX_NEW_OPERATION_BYTES: usize = MAX_PRESERVATION_EVIDENCE_BYTES + 4096;
pub const MAX_ORIGINAL_OPERATIONS: usize = 16_384;
pub const MAX_SELECTED_ORIGINAL_OPERATIONS: usize = 128;
pub const ORIGINAL_OPERATION_PREFIX: &str = legacy::ORIGINAL_OPERATION_PREFIX;
pub(super) const V15: &str = "CREATE TABLE inbox_original_operations (
 id TEXT PRIMARY KEY,
 format INTEGER NOT NULL,
 kind TEXT NOT NULL,
 item_id TEXT NOT NULL,
 parent_id TEXT,
 parent_sha256 BLOB,
 prepared_at_ms INTEGER NOT NULL,
 settled_at_ms INTEGER,
 original_sha256 BLOB NOT NULL,
 namespace_sha256 BLOB NOT NULL,
 record_json BLOB NOT NULL,
 body_sha256 BLOB NOT NULL,
 record_sha256 BLOB NOT NULL
);
CREATE INDEX inbox_original_operations_item ON inbox_original_operations(item_id,id);";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxRemovalConfirmation {
    pub version: u8,
    pub exact_copy_removal_intended: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxOriginalParent {
    pub operation_id: Uuid,
    pub record_sha256: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveInboxOriginalRequest {
    pub operation_id: Uuid,
    pub item_id: Uuid,
    pub preview_digest: [u8; 32],
    pub previous_restore: Option<InboxOriginalParent>,
    pub confirmation: InboxRemovalConfirmation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxApprovedSource {
    pub approval: ApplyJournal,
    pub saved: InboxSourceProof,
}
/// Field order and representation exactly match the qualified Workflow preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxQualifiedRemovalEvidence {
    pub item: inbox::InboxItem,
    pub original: InboxQualifiedOriginal,
    pub source: Option<InboxApprovedSource>,
    pub blockers: [(); 0],
    pub needs_owner_confirmation: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxOriginalRemovalRecord {
    pub request: RemoveInboxOriginalRequest,
    pub evidence: InboxQualifiedRemovalEvidence,
    pub namespace: InboxOriginalNamespace,
    pub prepared_at_ms: u64,
    pub removed_at_ms: Option<u64>,
}
pub use legacy::RestoreInboxOriginalRequest;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxOriginalRestoreRecord {
    pub request: RestoreInboxOriginalRequest,
    pub original: inbox::InboxItem,
    pub namespace: InboxOriginalNamespace,
    pub prepared_at_ms: u64,
    pub restored_at_ms: Option<u64>,
}
/// Versions are explicit. Legacy variants retain their original typed records;
/// ordinary format1 mirrors must encode `legacy::Operation`, not this wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "record",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum InboxOriginalOperation {
    Remove(Box<InboxOriginalRemovalRecord>),
    Restore(Box<InboxOriginalRestoreRecord>),
    LegacyRemove(Box<legacy::InboxOriginalRemovalRecord>),
    LegacyRestore(Box<legacy::InboxOriginalRestoreRecord>),
}
#[derive(Debug, Clone)]
pub struct InboxOriginalInventory {
    /// Item UUID ascending, then each item's exact causal order. No wall-clock sort.
    pub history: Vec<InboxOriginalOperationSummary>,
    /// Caller order. Missing selections refuse; variant determines the exact kind.
    /// At most 128 bodies and 64 MiB aggregate canonical encoded bytes.
    pub selected: Vec<InboxOriginalOperation>,
    /// Full SQL/legacy bodies parsed in this pass; never a semantic authority.
    pub parsed_bodies: usize,
    archived_analyses: HashSet<Uuid>,
}
fn encode<T: Serialize>(v: &T, limit: usize) -> Result<Vec<u8>> {
    legacy::encode_bounded(v, limit)
}
fn digest<T: Serialize>(v: &T) -> Result<[u8; 32]> {
    Ok(hash(&encode(v, MAX_NEW_OPERATION_BYTES)?))
}
fn conflict() -> Error {
    Error::OperationConflict(
        "original operation UUID or causal certificate has another exact body".into(),
    )
}
fn nonnil(id: Uuid) -> Result<()> {
    super::proposals::nonnil(id)
}
fn time(prepared: u64, settled: Option<u64>) -> Result<()> {
    if prepared > i64::MAX as u64 || settled.is_some_and(|t| t < prepared || t > i64::MAX as u64) {
        return Err(invalid(
            "original operation timestamps are not monotonic bounded observations",
        ));
    }
    Ok(())
}
impl InboxRemovalConfirmation {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 || !self.exact_copy_removal_intended {
            return Err(invalid(
                "original removal requires explicit exact-copy confirmation version 1",
            ));
        }
        Ok(())
    }
}
impl RemoveInboxOriginalRequest {
    pub fn validate(&self) -> Result<()> {
        nonnil(self.operation_id)?;
        nonnil(self.item_id)?;
        self.confirmation.validate()?;
        if let Some(p) = &self.previous_restore {
            nonnil(p.operation_id)?;
            if p.operation_id == self.operation_id {
                return Err(conflict());
            }
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.validate()?;
        digest(self)
    }
}
impl InboxQualifiedRemovalEvidence {
    fn checked_bytes(&self) -> Result<Vec<u8>> {
        let InboxQualifiedOriginal::Available { text } = &self.original;
        let source = self.source.as_ref().ok_or_else(|| {
            invalid("original removal requires one exact approved Source witness")
        })?;
        if !self.needs_owner_confirmation {
            return Err(invalid(
                "qualified evidence must still require owner confirmation",
            ));
        }
        InboxSourcePreservation {
            original: &self.item,
            original_text: text,
            approval: &source.approval,
            saved: &source.saved.source,
            saved_text: &source.saved.text,
        }
        .validate()?;
        encode(self, MAX_PRESERVATION_EVIDENCE_BYTES)
    }
    pub fn validate(&self) -> Result<()> {
        self.checked_bytes().map(|_| ())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        Ok(hash(&self.checked_bytes()?))
    }
}
impl InboxOriginalRemovalRecord {
    fn checked_digest(&self) -> Result<[u8; 32]> {
        self.request.validate()?;
        let preview = self.evidence.digest()?;
        self.namespace.validate()?;
        time(self.prepared_at_ms, self.removed_at_ms)?;
        if self.request.item_id != self.evidence.item.capture.id
            || self.request.preview_digest != preview
            || self.prepared_at_ms < self.evidence.item.received_at_ms
        {
            return Err(invalid(
                "removal differs from exact owner preview or original time",
            ));
        }
        digest(self)
    }
    pub fn validate(&self) -> Result<()> {
        self.checked_digest().map(|_| ())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.checked_digest()
    }
}
impl InboxOriginalRestoreRecord {
    fn checked_digest(&self) -> Result<[u8; 32]> {
        self.request.validate()?;
        self.original.validate()?;
        self.namespace.validate()?;
        time(self.prepared_at_ms, self.restored_at_ms)?;
        if self.prepared_at_ms < self.original.received_at_ms {
            return Err(invalid("Restore predates its original capture"));
        }
        digest(self)
    }
    pub fn validate(&self) -> Result<()> {
        self.checked_digest().map(|_| ())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.checked_digest()
    }
}
impl InboxOriginalOperation {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Remove(r) => r.validate(),
            Self::Restore(r) => r.validate(),
            Self::LegacyRemove(r) => r.validate(),
            Self::LegacyRestore(r) => r.validate(),
        }
    }
    pub fn format(&self) -> u8 {
        match self {
            Self::Remove(_) | Self::Restore(_) => 2,
            _ => 1,
        }
    }
    pub fn original(&self) -> &inbox::InboxItem {
        match self {
            Self::Remove(r) => &r.evidence.item,
            Self::Restore(r) => &r.original,
            Self::LegacyRemove(r) => &r.evidence.snapshot.review.original,
            Self::LegacyRestore(r) => &r.original,
        }
    }
    pub fn namespace(&self) -> &InboxOriginalNamespace {
        match self {
            Self::Remove(r) => &r.namespace,
            Self::Restore(r) => &r.namespace,
            Self::LegacyRemove(r) => &r.namespace,
            Self::LegacyRestore(r) => &r.namespace,
        }
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        match self {
            Self::Remove(r) => r.digest(),
            Self::Restore(r) => r.digest(),
            Self::LegacyRemove(r) => r.digest(),
            Self::LegacyRestore(r) => r.digest(),
        }
    }
    pub fn summary(&self) -> Result<InboxOriginalOperationSummary> {
        let record_sha256 = self.digest()?;
        let (id, parent, kind, prepared, settled) = match self {
            Self::Remove(r) => (
                r.request.operation_id,
                r.request.previous_restore.as_ref().map(|p| p.operation_id),
                InboxOriginalOperationKind::Remove,
                r.prepared_at_ms,
                r.removed_at_ms,
            ),
            Self::Restore(r) => (
                r.request.operation_id,
                Some(r.request.removal_operation_id),
                InboxOriginalOperationKind::Restore,
                r.prepared_at_ms,
                r.restored_at_ms,
            ),
            Self::LegacyRemove(r) => (
                r.request.operation_id,
                r.request.previous_restore,
                InboxOriginalOperationKind::Remove,
                r.prepared_at_ms,
                r.removed_at_ms,
            ),
            Self::LegacyRestore(r) => (
                r.request.operation_id,
                Some(r.request.removal_operation_id),
                InboxOriginalOperationKind::Restore,
                r.prepared_at_ms,
                r.restored_at_ms,
            ),
        };
        Ok(InboxOriginalOperationSummary {
            kind,
            operation_id: id,
            item_id: self.original().capture.id,
            parent,
            prepared_at_ms: prepared,
            settled_at_ms: settled,
            record_sha256,
        })
    }
    fn parent_digest(&self) -> Option<[u8; 32]> {
        match self {
            Self::Remove(r) => r.request.previous_restore.as_ref().map(|p| p.record_sha256),
            Self::Restore(r) => Some(r.request.removal_digest),
            Self::LegacyRemove(r) => r
                .evidence
                .snapshot
                .original_operations
                .last()
                .map(|p| p.record_sha256),
            Self::LegacyRestore(r) => Some(r.request.removal_digest),
        }
    }
    fn without_terminal(&self) -> Self {
        let mut r = self.clone();
        match &mut r {
            Self::Remove(r) => r.removed_at_ms = None,
            Self::Restore(r) => r.restored_at_ms = None,
            Self::LegacyRemove(r) => r.removed_at_ms = None,
            Self::LegacyRestore(r) => r.restored_at_ms = None,
        }
        r
    }
}
fn from_legacy(value: legacy::Operation) -> InboxOriginalOperation {
    match value {
        legacy::Operation::Remove(r) => InboxOriginalOperation::LegacyRemove(r),
        legacy::Operation::Restore(r) => InboxOriginalOperation::LegacyRestore(r),
    }
}
#[derive(Serialize)]
#[serde(tag = "kind", content = "record", rename_all = "snake_case")]
enum LegacyRef<'a> {
    Remove(&'a legacy::InboxOriginalRemovalRecord),
    Restore(&'a legacy::InboxOriginalRestoreRecord),
}
fn legacy_ref(op: &InboxOriginalOperation) -> Option<LegacyRef<'_>> {
    match op {
        InboxOriginalOperation::LegacyRemove(r) => Some(LegacyRef::Remove(r)),
        InboxOriginalOperation::LegacyRestore(r) => Some(LegacyRef::Restore(r)),
        _ => None,
    }
}
fn canonical_body(op: &InboxOriginalOperation) -> Result<Vec<u8>> {
    if let Some(op) = legacy_ref(op) {
        encode(&op, MAX_ORIGINAL_OPERATION_BYTES)
    } else {
        encode(op, MAX_NEW_OPERATION_BYTES)
    }
}
pub(super) fn guard_setting(key: &str) -> Result<()> {
    if key.starts_with(ORIGINAL_OPERATION_PREFIX) {
        return Err(invalid(
            "original-operation settings are owned typed legacy records",
        ));
    }
    Ok(())
}
fn normalized(sql: &str) -> String {
    sql.trim()
        .trim_end_matches(';')
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect()
}
fn check_schema(conn: &Connection) -> Result<()> {
    for ((name, kind), expected) in [
        ("inbox_original_operations", "table"),
        ("inbox_original_operations_item", "index"),
    ]
    .into_iter()
    .zip(V15.split(';').filter(|s| !s.trim().is_empty()))
    {
        let row: Option<(String, String, String)> = conn
            .query_row(
                "SELECT type,tbl_name,sql FROM sqlite_schema WHERE name=?1",
                [name],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if !row.is_some_and(|(k, t, s)| {
            k == kind && t == "inbox_original_operations" && normalized(&s) == normalized(expected)
        }) {
            return Err(invalid(
                "original-operation schema/index differs from its owned shape",
            ));
        }
    }
    let objects:Vec<(String,String)>=conn.prepare("SELECT type,name FROM sqlite_schema WHERE tbl_name='inbox_original_operations' ORDER BY name")?.query_map([],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
    if objects
        != [
            ("table".into(), "inbox_original_operations".into()),
            ("index".into(), "inbox_original_operations_item".into()),
            (
                "index".into(),
                "sqlite_autoindex_inbox_original_operations_1".into(),
            ),
        ]
    {
        return Err(invalid(
            "original-operation schema has unexpected owned objects",
        ));
    }
    check_legacy_schema(conn)
}
fn check_legacy_schema(conn: &Connection) -> Result<()> {
    let actual: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='settings' AND type='table'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let extras:i64=conn.query_row("SELECT count(*) FROM sqlite_schema WHERE tbl_name='settings' AND name NOT IN ('settings','sqlite_autoindex_settings_1')",[],|r|r.get(0))?;
    if actual.is_none_or(|s| {
        normalized(&s) != normalized(super::MIGRATIONS[0].split(';').next().unwrap())
    }) || extras != 0
    {
        return Err(invalid(
            "legacy operation settings differ from their owned shape",
        ));
    }
    Ok(())
}
struct Row {
    id: String,
    format: i64,
    kind: String,
    item: String,
    parent: Option<String>,
    parent_sha: Option<Vec<u8>>,
    prepared: i64,
    settled: Option<i64>,
    original: Vec<u8>,
    namespace: Vec<u8>,
    body: Option<Vec<u8>>,
    body_sha: Vec<u8>,
    record_sha: Vec<u8>,
}
fn check_row(row: Row) -> Result<Checked> {
    let body = row
        .body
        .ok_or_else(|| invalid("original operation exceeds its complete encoded bound"))?;
    if hash(&body).as_slice() != row.body_sha {
        return Err(invalid("original operation body hash differs"));
    }
    let op: InboxOriginalOperation =
        serde_json::from_slice(&body).map_err(|_| invalid("malformed original-operation body"))?;
    if op.format() != 2 || encode(&op, MAX_NEW_OPERATION_BYTES)? != body {
        return Err(invalid(
            "original-operation body has another format or noncanonical bytes",
        ));
    }
    let checked = Checked::new(op)?;
    let s = &checked.summary;
    let op = &checked.op;
    if row.id != s.operation_id.to_string()
        || row.format != 2
        || row.kind != kind(s.kind)
        || row.item != s.item_id.to_string()
        || row.parent != s.parent.map(|p| p.to_string())
        || row.parent_sha != op.parent_digest().map(|p| p.to_vec())
        || row.prepared != s.prepared_at_ms as i64
        || row.settled != s.settled_at_ms.map(|t| t as i64)
        || row.record_sha != s.record_sha256
        || row.original != digest(op.original())?
        || row.namespace != digest(op.namespace())?
    {
        return Err(invalid(
            "original-operation indexed metadata differs from its full checked body",
        ));
    }
    Ok(checked)
}
fn kind(kind: InboxOriginalOperationKind) -> &'static str {
    match kind {
        InboxOriginalOperationKind::Remove => "remove",
        InboxOriginalOperationKind::Restore => "restore",
    }
}
struct Checked {
    op: InboxOriginalOperation,
    summary: InboxOriginalOperationSummary,
    original: [u8; 32],
    namespace: [u8; 32],
    parent_sha: Option<[u8; 32]>,
}
impl Checked {
    fn new(op: InboxOriginalOperation) -> Result<Self> {
        let summary = op.summary()?;
        let original = digest(op.original())?;
        let namespace = digest(op.namespace())?;
        let parent_sha = op.parent_digest();
        Ok(Self {
            op,
            summary,
            original,
            namespace,
            parent_sha,
        })
    }
}
struct Meta {
    summary: InboxOriginalOperationSummary,
    original: [u8; 32],
    namespace: [u8; 32],
    parent_sha: Option<[u8; 32]>,
    legacy_history: Option<(usize, [u8; 32])>,
}
struct Pass {
    metas: BTreeMap<Uuid, Meta>,
    selected: HashMap<Uuid, InboxOriginalOperation>,
    bytes: usize,
    parsed: usize,
    captures: HashMap<Uuid, [u8; 32]>,
    turns: HashMap<Uuid, [u8; 32]>,
}
impl Pass {
    fn new() -> Self {
        Self {
            metas: BTreeMap::new(),
            selected: HashMap::new(),
            bytes: 0,
            parsed: 0,
            captures: HashMap::new(),
            turns: HashMap::new(),
        }
    }
    fn accept(
        &mut self,
        conn: &Connection,
        checked: Checked,
        wanted: &HashSet<Uuid>,
    ) -> Result<()> {
        self.parsed += 1;
        if self.parsed > MAX_ORIGINAL_OPERATIONS {
            return Err(invalid(
                "original-operation inventory exceeds its row bound",
            ));
        }
        let Checked {
            op,
            summary: s,
            original,
            namespace,
            parent_sha,
        } = checked;
        if inbox::read(conn, s.item_id)?.as_ref() != Some(op.original()) {
            return Err(invalid("original operation lost its exact catalog capture"));
        }
        let legacy_history = if let InboxOriginalOperation::LegacyRemove(r) = &op {
            for a in &r.evidence.snapshot.review.analyses {
                let id = a.job.capture.id;
                let sha = legacy::digest(&a.job)?;
                if self.captures.insert(id, sha).is_some_and(|h| h != sha)
                    || inbox_actions::reserved(conn, id)?.as_ref() != Some(&a.job)
                {
                    return Err(invalid(
                        "legacy archived capture differs from genuine retained reservation",
                    ));
                }
                if let Some(t) = &a.turn {
                    let sha = legacy::digest(t)?;
                    if self.turns.insert(id, sha).is_some_and(|h| h != sha) {
                        return Err(invalid("legacy same-ID historical turns disagree"));
                    }
                }
            }
            Some((
                r.evidence.snapshot.original_operations.len(),
                legacy::digest(&r.evidence.snapshot.original_operations)?,
            ))
        } else {
            None
        };
        let meta = Meta {
            summary: s.clone(),
            original,
            namespace,
            parent_sha,
            legacy_history,
        };
        if self.metas.insert(s.operation_id, meta).is_some() {
            return Err(invalid(
                "original operation identity appears in more than one version/store",
            ));
        }
        if wanted.contains(&s.operation_id) {
            self.bytes = self
                .bytes
                .checked_add(canonical_body(&op)?.len())
                .ok_or_else(|| invalid("selected operation bound overflow"))?;
            if self.bytes > MAX_ORIGINAL_OPERATION_BYTES {
                return Err(invalid(
                    "selected complete original-operation bodies exceed aggregate bound",
                ));
            }
            self.selected.insert(s.operation_id, op);
        }
        Ok(())
    }
    fn finish(self, selected: &[Uuid], required: bool) -> Result<InboxOriginalInventory> {
        let mut groups: BTreeMap<Uuid, Vec<InboxOriginalOperationSummary>> = BTreeMap::new();
        for m in self.metas.values() {
            groups
                .entry(m.summary.item_id)
                .or_default()
                .push(m.summary.clone());
        }
        let mut history = Vec::new();
        use sha2::{Digest, Sha256};
        for values in groups.into_values() {
            let values = legacy::ordered_history(values)?;
            let mut prefix = Sha256::new();
            prefix.update(b"[");
            for (i, s) in values.iter().enumerate() {
                let m = &self.metas[&s.operation_id];
                if let Some(parent) = s.parent {
                    let p = &self.metas[&parent];
                    if m.original != p.original
                        || m.namespace != p.namespace
                        || m.parent_sha != Some(p.summary.record_sha256)
                    {
                        return Err(invalid(
                            "original-operation causal identity/namespace/parent digest differs",
                        ));
                    }
                }
                if let Some((len, sha)) = m.legacy_history {
                    let mut expected = prefix.clone();
                    expected.update(b"]");
                    if len != i || <[u8; 32]>::from(expected.finalize()) != sha {
                        return Err(invalid(
                            "legacy removal differs from its complete historical causal prefix",
                        ));
                    }
                }
                if i != 0 {
                    prefix.update(b",");
                }
                prefix.update(encode(s, MAX_ORIGINAL_OPERATION_BYTES)?);
            }
            history.extend(values);
        }
        let mut bodies = self.selected;
        let mut output = Vec::new();
        for id in selected {
            if let Some(op) = bodies.remove(id) {
                output.push(op);
            } else if required {
                return Err(Error::NotFound(
                    "selected original operation is absent".into(),
                ));
            }
        }
        Ok(InboxOriginalInventory {
            history,
            selected: output,
            parsed_bodies: self.parsed,
            archived_analyses: self.captures.into_keys().collect(),
        })
    }
}
fn select_check(selected: &[Uuid], public: bool) -> Result<HashSet<Uuid>> {
    if public && selected.len() > MAX_SELECTED_ORIGINAL_OPERATIONS {
        return Err(invalid("too many selected original operations"));
    }
    let mut ids = HashSet::new();
    for id in selected {
        nonnil(*id)?;
        if !ids.insert(*id) {
            return Err(invalid("duplicate selected original-operation UUID"));
        }
    }
    Ok(ids)
}
fn inventory(
    conn: &Connection,
    selected: &[Uuid],
    required: bool,
    table: bool,
) -> Result<InboxOriginalInventory> {
    if table {
        check_schema(conn)?;
    } else {
        check_legacy_schema(conn)?;
    }
    let wanted = select_check(selected, false)?;
    let mut pass = Pass::new();
    if table {
        let mut statement=conn.prepare("SELECT id,format,kind,item_id,parent_id,parent_sha256,prepared_at_ms,settled_at_ms,original_sha256,namespace_sha256,CASE WHEN length(record_json)<=?1 THEN record_json END,body_sha256,record_sha256 FROM inbox_original_operations ORDER BY id")?;
        for row in statement.query_map([MAX_NEW_OPERATION_BYTES as i64], |r| {
            Ok(Row {
                id: r.get(0)?,
                format: r.get(1)?,
                kind: r.get(2)?,
                item: r.get(3)?,
                parent: r.get(4)?,
                parent_sha: r.get(5)?,
                prepared: r.get(6)?,
                settled: r.get(7)?,
                original: r.get(8)?,
                namespace: r.get(9)?,
                body: r.get(10)?,
                body_sha: r.get(11)?,
                record_sha: r.get(12)?,
            })
        })? {
            pass.accept(conn, check_row(row?)?, &wanted)?;
        }
    }
    let mut statement=conn.prepare("SELECT key,CASE WHEN length(CAST(value AS BLOB))<=?2 THEN value END FROM settings WHERE substr(key,1,?1)=?3 ORDER BY key")?;
    for row in statement.query_map(
        params![
            ORIGINAL_OPERATION_PREFIX.len() as i64,
            MAX_ORIGINAL_OPERATION_BYTES as i64,
            ORIGINAL_OPERATION_PREFIX
        ],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
    )? {
        let (key, bytes) = row?;
        let bytes =
            bytes.ok_or_else(|| invalid("legacy original operation exceeds its complete bound"))?;
        let env: legacy::Envelope = serde_json::from_str(&bytes)
            .map_err(|_| invalid("malformed legacy original-operation envelope"))?;
        let id = env.operation.summary()?.operation_id;
        if key != format!("{ORIGINAL_OPERATION_PREFIX}{id}")
            || env.sha256 != env.operation.digest()?
            || encode(&env, MAX_ORIGINAL_OPERATION_BYTES)? != bytes.as_bytes()
        {
            return Err(invalid(
                "legacy original operation failed exact canonical/hash/key checks",
            ));
        }
        pass.accept(conn, Checked::new(from_legacy(env.operation))?, &wanted)?;
    }
    pass.finish(selected, required)
}
/// Historical legacy reservations are not live Sessions. Check the whole
/// family once and retain only analysis IDs, without restoring archived turns.
pub(super) fn has_archived_analysis(conn: &Connection, id: Uuid) -> Result<bool> {
    nonnil(id)?;
    Ok(inventory(conn, &[], false, true)?
        .archived_analyses
        .contains(&id))
}
pub(super) fn check_all(conn: &Connection) -> Result<()> {
    inventory(conn, &[], false, true).map(|_| ())
}
pub(super) fn check_legacy(conn: &Connection) -> Result<()> {
    inventory(conn, &[], false, false).map(|_| ())
}
fn write(conn: &Connection, checked: &Checked) -> Result<()> {
    let op = &checked.op;
    let s = &checked.summary;
    if let Some(operation) = legacy_ref(op) {
        #[derive(Serialize)]
        struct Envelope<'a> {
            operation: LegacyRef<'a>,
            sha256: [u8; 32],
        }
        let sha256 = hash(&encode(&operation, MAX_ORIGINAL_OPERATION_BYTES)?);
        let envelope = Envelope { operation, sha256 };
        let bytes = String::from_utf8(encode(&envelope, MAX_ORIGINAL_OPERATION_BYTES)?)
            .map_err(|_| invalid("legacy operation is not UTF8"))?;
        conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![format!("{ORIGINAL_OPERATION_PREFIX}{}",s.operation_id),bytes])?;
    } else {
        let bytes = encode(op, MAX_NEW_OPERATION_BYTES)?;
        conn.execute("INSERT INTO inbox_original_operations(id,format,kind,item_id,parent_id,parent_sha256,prepared_at_ms,settled_at_ms,original_sha256,namespace_sha256,record_json,body_sha256,record_sha256) VALUES(?1,2,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12) ON CONFLICT(id) DO UPDATE SET format=excluded.format,kind=excluded.kind,item_id=excluded.item_id,parent_id=excluded.parent_id,parent_sha256=excluded.parent_sha256,prepared_at_ms=excluded.prepared_at_ms,settled_at_ms=excluded.settled_at_ms,original_sha256=excluded.original_sha256,namespace_sha256=excluded.namespace_sha256,record_json=excluded.record_json,body_sha256=excluded.body_sha256,record_sha256=excluded.record_sha256",params![s.operation_id.to_string(),kind(s.kind),s.item_id.to_string(),s.parent.map(|p|p.to_string()),checked.parent_sha.map(|p|p.to_vec()),s.prepared_at_ms as i64,s.settled_at_ms.map(|t|t as i64),checked.original.as_slice(),checked.namespace.as_slice(),bytes,hash(&bytes).as_slice(),s.record_sha256.as_slice()])?;
    }
    Ok(())
}
impl WorkStore {
    pub fn inbox_original_operations(&self, selected: &[Uuid]) -> Result<InboxOriginalInventory> {
        select_check(selected, true)?;
        let tx = self.conn.unchecked_transaction()?;
        let result = inventory(&tx, selected, true, true)?;
        tx.commit()?;
        Ok(result)
    }
    /// Stream already checked records in causal inventory order. The complete
    /// semantic inventory is validated before the first callback. Keyed reads
    /// use the same read transaction, bind its exact summaries/digests, and drop
    /// each body after the callback. At most two full-body reads per record.
    /// The consumer owns any effects; a consumer error remains its own type.
    pub fn visit_inbox_original_operations<E: From<Error>>(
        &self,
        mut visitor: impl FnMut(InboxOriginalOperation) -> std::result::Result<(), E>,
    ) -> std::result::Result<InboxOriginalInventory, E> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(Error::from)
            .map_err(E::from)?;
        let mut result = inventory(&tx, &[], false, true).map_err(E::from)?;
        for summary in &result.history {
            let checked = read_one(&tx, summary.operation_id)
                .map_err(E::from)?
                .ok_or_else(|| {
                    E::from(invalid(
                        "checked original operation disappeared during visit",
                    ))
                })?;
            if checked.summary != *summary {
                return Err(E::from(invalid(
                    "visited original operation differs from its checked inventory",
                )));
            }
            result.parsed_bodies += 1;
            visitor(checked.op)?;
        }
        tx.commit().map_err(Error::from).map_err(E::from)?;
        Ok(result)
    }
    /// Exact recovery import convenience wrapper over the streaming transaction.
    pub fn restore_inbox_original_operations(
        &mut self,
        records: &[InboxOriginalOperation],
        selected: &[Uuid],
    ) -> Result<InboxOriginalInventory> {
        self.restore_inbox_original_operation_records(records.iter().cloned().map(Ok), selected)
    }
    /// Domain-specific streaming recovery. Each complete bounded body is dropped
    /// after its exact import. No aggregate input cap or full family accumulation;
    /// initial/final inventory passes and at most one existing-row read per input.
    /// All writes and genuine legacy reservations roll back on any iterator/error.
    pub fn restore_inbox_original_operation_records(
        &mut self,
        records: impl IntoIterator<Item = Result<InboxOriginalOperation>>,
        selected: &[Uuid],
    ) -> Result<InboxOriginalInventory> {
        select_check(selected, true)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut parsed_bodies = inventory(&tx, &[], false, true)?.parsed_bodies;
        let mut ids = HashSet::new();
        for record in records {
            let r = record?;
            let checked = Checked::new(r)?;
            let id = checked.summary.operation_id;
            if !ids.insert(id) || ids.len() > MAX_ORIGINAL_OPERATIONS {
                return Err(invalid("duplicate or excessive original-operation imports"));
            }
            if let Some(old) = read_one(&tx, id)? {
                parsed_bodies += 1;
                if canonical_body(&old.op.without_terminal())?
                    != canonical_body(&checked.op.without_terminal())?
                {
                    return Err(conflict());
                }
                let a = old.summary.settled_at_ms;
                let b = checked.summary.settled_at_ms;
                if a.zip(b).is_some_and(|(a, b)| a != b) {
                    return Err(conflict());
                }
                if a.is_some() || canonical_body(&old.op)? == canonical_body(&checked.op)? {
                    continue;
                }
            }
            let merged = &checked.op;
            inbox::restore_item(&tx, merged.original())?;
            if let InboxOriginalOperation::LegacyRemove(r) = &merged {
                for a in &r.evidence.snapshot.review.analyses {
                    inbox_actions::restore_capture(&tx, &a.job)?;
                }
            }
            write(&tx, &checked)?;
        }
        let mut result = inventory(&tx, selected, true, true)?;
        result.parsed_bodies += parsed_bodies;
        tx.commit()?;
        Ok(result)
    }
}
fn read_one(conn: &Connection, id: Uuid) -> Result<Option<Checked>> {
    let row=conn.query_row("SELECT id,format,kind,item_id,parent_id,parent_sha256,prepared_at_ms,settled_at_ms,original_sha256,namespace_sha256,CASE WHEN length(record_json)<=?2 THEN record_json END,body_sha256,record_sha256 FROM inbox_original_operations WHERE id=?1",params![id.to_string(),MAX_NEW_OPERATION_BYTES as i64],|r|Ok(Row{id:r.get(0)?,format:r.get(1)?,kind:r.get(2)?,item:r.get(3)?,parent:r.get(4)?,parent_sha:r.get(5)?,prepared:r.get(6)?,settled:r.get(7)?,original:r.get(8)?,namespace:r.get(9)?,body:r.get(10)?,body_sha:r.get(11)?,record_sha:r.get(12)?})).optional()?;
    if let Some(row) = row {
        return Ok(Some(check_row(row)?));
    }
    let value:Option<Option<String>>=conn.query_row("SELECT CASE WHEN length(CAST(value AS BLOB))<=?2 THEN value END FROM settings WHERE key=?1",params![format!("{ORIGINAL_OPERATION_PREFIX}{id}"),MAX_ORIGINAL_OPERATION_BYTES as i64],|r|r.get(0)).optional()?;
    value
        .map(|v| {
            let bytes = v.ok_or_else(|| invalid("legacy original operation exceeds its bound"))?;
            let env: legacy::Envelope = serde_json::from_str(&bytes)
                .map_err(|_| invalid("malformed legacy original operation"))?;
            if env.operation.summary()?.operation_id != id
                || env.sha256 != env.operation.digest()?
                || encode(&env, MAX_ORIGINAL_OPERATION_BYTES)? != bytes.as_bytes()
            {
                return Err(invalid(
                    "legacy original operation failed canonical/identity checks",
                ));
            }
            Checked::new(from_legacy(env.operation))
        })
        .transpose()
}
