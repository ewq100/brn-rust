//! Tentative review work; this module never reads or changes vault files.
use super::{MAX_NOTE_BYTES, WorkStore, now_ms, proposals::SourceVersion};
use crate::{
    Error, Result,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    hash, invalid,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};
use uuid::Uuid;

pub(super) const V9: &str = "
CREATE TABLE findings (
 id TEXT PRIMARY KEY,
 state TEXT NOT NULL CHECK(state IN ('open','resolved','dismissed')),
 created_at_ms INTEGER NOT NULL,
 creation_sha256 BLOB NOT NULL CHECK(length(creation_sha256)=32),
 record_json BLOB NOT NULL,
 record_sha256 BLOB NOT NULL CHECK(length(record_sha256)=32)
);
CREATE INDEX findings_page ON findings(created_at_ms DESC,id DESC);";
const MAX_SUMMARY_BYTES: usize = 16 * 1024;
const MAX_QUOTE_BYTES: usize = 16 * 1024;
const MAX_EVIDENCE: usize = 64;
// Text JSON escaping can expand each retained byte sixfold; numeric proofs and
// the fixed record envelope have a separate bounded allowance.
const MAX_STORED_BYTES: usize = MAX_NOTE_BYTES * 6 + 128 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureFindingRequest {
    pub id: Uuid,
    pub origin: FindingOrigin,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FindingOrigin {
    IdentityAmbiguity {
        note_id: Uuid,
    },
    UnresolvedLink {
        path: String,
        source_sha256: [u8; 32],
        destination: String,
        start_byte: usize,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingQuote {
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingEvidence {
    #[serde(deserialize_with = "strict_source")]
    pub source: SourceVersion,
    pub note_id: Option<Uuid>,
    pub quote: Option<FindingQuote>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingDraft {
    pub request: CaptureFindingRequest,
    #[serde(deserialize_with = "strict_vault")]
    pub vault: VaultRecord,
    pub title: String,
    pub summary: String,
    pub evidence: Vec<FindingEvidence>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingState {
    Open,
    Resolved,
    Dismissed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingStamp {
    pub id: Uuid,
    pub version: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingRecord {
    pub draft: FindingDraft,
    pub version: u64,
    pub state: FindingState,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}
impl FindingRecord {
    /// Exact operational review version; this conveys no vault-write authority.
    pub fn stamp(&self) -> FindingStamp {
        FindingStamp {
            id: self.draft.request.id,
            version: self.version,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingListRequest {
    pub state: Option<FindingState>,
    pub limit: usize,
    pub before: Option<Uuid>,
}
impl Default for FindingListRequest {
    fn default() -> Self {
        Self {
            state: Some(FindingState::Open),
            limit: 25,
            before: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingPage {
    pub entries: Vec<FindingRecord>,
    pub next_before: Option<Uuid>,
    pub open_count: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloseFindingRequest {
    pub expected: FindingStamp,
    pub state: FindingState,
}

fn nonnil(id: Uuid) -> Result<()> {
    if id.is_nil() {
        return Err(invalid("finding UUID must not be nil"));
    }
    Ok(())
}
fn path(raw: &str) -> Result<()> {
    if raw.is_empty()
        || raw.contains('\\')
        || raw.chars().any(char::is_control)
        || raw
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.'))
        || Path::new(raw)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || !Path::new(raw)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        return Err(invalid(
            "finding source needs a scalar visible contained Markdown path",
        ));
    }
    Ok(())
}
fn add(total: &mut usize, bytes: usize) -> Result<()> {
    *total = total
        .checked_add(bytes)
        .filter(|n| *n <= MAX_NOTE_BYTES)
        .ok_or_else(|| invalid("finding retained work exceeds 1 MiB"))?;
    Ok(())
}
impl CaptureFindingRequest {
    /// Pure symbolic-intent bounds; the workflow separately captures saved proof.
    pub fn validate(&self) -> Result<()> {
        nonnil(self.id)?;
        match &self.origin {
            FindingOrigin::IdentityAmbiguity { note_id } => nonnil(*note_id)?,
            FindingOrigin::UnresolvedLink {
                path: source,
                destination,
                start_byte,
                ..
            } => {
                path(source)?;
                if source.len() > MAX_NOTE_BYTES
                    || destination.is_empty()
                    || destination.len() > MAX_NOTE_BYTES
                    || *start_byte >= MAX_NOTE_BYTES
                {
                    return Err(invalid(
                        "unresolved link needs a bounded saved occurrence and destination",
                    ));
                }
            }
        }
        Ok(())
    }
}
impl FindingDraft {
    /// Bounds immutable evidence. Original bytes and UTF-8 boundaries are checked
    /// by workflow capture, never guessed from a standalone quote.
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        nonnil(self.vault.id)?;
        let root = self
            .vault
            .root
            .to_str()
            .ok_or_else(|| invalid("finding vault root must be UTF-8"))?;
        if !self.vault.root.is_absolute() || root.contains('\0') {
            return Err(invalid("finding vault root must be an absolute path"));
        }
        if self.title.trim().is_empty()
            || self.title.len() > 512
            || self.summary.trim().is_empty()
            || self.summary.len() > MAX_SUMMARY_BYTES
            || self.evidence.is_empty()
            || self.evidence.len() > MAX_EVIDENCE
        {
            return Err(invalid(
                "finding needs a bounded title, summary and 1 to 64 evidence entries",
            ));
        }
        let mut total = 0;
        add(&mut total, root.len())?;
        add(&mut total, self.title.len())?;
        add(&mut total, self.summary.len())?;
        if let FindingOrigin::UnresolvedLink {
            path, destination, ..
        } = &self.request.origin
        {
            add(&mut total, path.len())?;
            add(&mut total, destination.len())?;
        }
        let mut paths = HashSet::new();
        for (index, evidence) in self.evidence.iter().enumerate() {
            path(&evidence.source.path)?;
            add(&mut total, evidence.source.path.len())?;
            if evidence.source.fingerprint.len > MAX_NOTE_BYTES as u64 {
                return Err(invalid("finding source exceeds 1 MiB"));
            }
            if let Some(id) = evidence.note_id {
                nonnil(id)?;
            }
            if self.evidence[..index].contains(evidence) {
                return Err(invalid("finding contains duplicate evidence proof"));
            }
            if let Some(quote) = &evidence.quote {
                if quote.quote.is_empty()
                    || quote.quote.len() > MAX_QUOTE_BYTES
                    || quote.start_byte >= quote.end_byte
                    || quote.end_byte as u128 > evidence.source.fingerprint.len as u128
                    || quote.end_byte - quote.start_byte != quote.quote.len()
                {
                    return Err(invalid("finding quote needs a bounded exact byte range"));
                }
                if self.evidence[..index].iter().any(|previous| {
                    previous.source.path == evidence.source.path
                        && previous.quote.as_ref().is_some_and(|previous| {
                            previous.start_byte == quote.start_byte
                                && previous.end_byte == quote.end_byte
                        })
                }) {
                    return Err(invalid("finding contains a repeated saved evidence range"));
                }
                add(&mut total, quote.quote.len())?;
            }
            match &self.request.origin {
                FindingOrigin::IdentityAmbiguity { note_id } => {
                    if evidence.note_id != Some(*note_id)
                        || !paths.insert(evidence.source.path.as_str())
                    {
                        return Err(invalid(
                            "identity ambiguity needs distinct paths with the observed UUID",
                        ));
                    }
                }
                FindingOrigin::UnresolvedLink {
                    path,
                    source_sha256,
                    start_byte,
                    ..
                } => {
                    let first = &self.evidence[0];
                    if evidence.source.path != *path
                        || evidence.source.fingerprint.sha256 != *source_sha256
                        || evidence.source != first.source
                        || evidence.note_id != first.note_id
                        || evidence.quote.is_none()
                        || index == 0
                            && evidence
                                .quote
                                .as_ref()
                                .is_none_or(|quote| quote.start_byte != *start_byte)
                    {
                        return Err(invalid(
                            "unresolved-link evidence must retain one full source proof and exact occurrence/definition quotes",
                        ));
                    }
                }
            }
        }
        if matches!(self.request.origin, FindingOrigin::IdentityAmbiguity { .. })
            && self.evidence.len() < 2
        {
            return Err(invalid("identity ambiguity needs at least two saved paths"));
        }
        Ok(())
    }
}
impl FindingListRequest {
    /// Validate bounded paging before operational storage is opened.
    pub fn validate(&self) -> Result<()> {
        if !(1..=100).contains(&self.limit) {
            return Err(invalid("finding page limit must be 1 to 100"));
        }
        if let Some(id) = self.before {
            nonnil(id)?;
        }
        Ok(())
    }
}
impl CloseFindingRequest {
    /// Only a version-one Open finding has a fresh terminal transition.
    pub fn validate(&self) -> Result<()> {
        nonnil(self.expected.id)?;
        if self.expected.version == 0 || self.state == FindingState::Open {
            return Err(invalid(
                "finding closure needs a review stamp and terminal state",
            ));
        }
        Ok(())
    }
}
fn state_name(state: FindingState) -> &'static str {
    match state {
        FindingState::Open => "open",
        FindingState::Resolved => "resolved",
        FindingState::Dismissed => "dismissed",
    }
}
fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| invalid("could not encode finding work"))
}
fn validate_record(record: &FindingRecord) -> Result<()> {
    record.draft.validate()?;
    if record.created_at_ms > record.updated_at_ms
        || record.updated_at_ms > i64::MAX as u64
        || !matches!(
            (record.version, record.state),
            (1, FindingState::Open) | (2, FindingState::Resolved | FindingState::Dismissed)
        )
        || record.version == 1 && record.updated_at_ms != record.created_at_ms
    {
        return Err(invalid(
            "stored finding has invalid state, version or timestamps",
        ));
    }
    Ok(())
}
struct FindingRow {
    state: String,
    created: i64,
    creation: Vec<u8>,
    bytes: Option<Vec<u8>>,
    digest: Vec<u8>,
}
fn read(conn: &Connection, id: Uuid) -> Result<Option<FindingRecord>> {
    nonnil(id)?;
    let row=conn.query_row("SELECT state,created_at_ms,creation_sha256,CASE WHEN length(record_json)<=?2 THEN record_json END,record_sha256 FROM findings WHERE id=?1",params![id.to_string(),MAX_STORED_BYTES as i64],|row| Ok(FindingRow{state:row.get(0)?,created:row.get(1)?,creation:row.get(2)?,bytes:row.get(3)?,digest:row.get(4)?})).optional()?;
    row.map(|row| {
        let bytes = row
            .bytes
            .ok_or_else(|| invalid("stored finding exceeds encoded size limit"))?;
        if row.digest.as_slice() != hash(&bytes) {
            return Err(invalid("stored finding failed its hash check"));
        }
        let record: FindingRecord =
            serde_json::from_slice(&bytes).map_err(|_| invalid("invalid stored finding record"))?;
        validate_record(&record)?;
        if record.draft.request.id != id
            || state_name(record.state) != row.state
            || i64::try_from(record.created_at_ms).ok() != Some(row.created)
            || row.creation.as_slice() != hash(&encode(&record.draft)?)
        {
            return Err(invalid(
                "stored finding differs from its immutable creation or indexed row binding",
            ));
        }
        Ok(record)
    })
    .transpose()
}
/// Refuse malformed semantic work before startup reconciliation or backup.
/// SQLite physical corruption retains the existing backup restoration path.
pub(super) fn check_all(conn: &Connection) -> Result<()> {
    let mut statement = conn.prepare("SELECT id FROM findings ORDER BY id")?;
    for id in statement.query_map([], |row| row.get::<_, String>(0))? {
        let raw = id?;
        let id = crate::parse_id(raw.clone())?;
        if raw != id.to_string() || read(conn, id)?.is_none() {
            return Err(invalid("invalid finding row identity"));
        }
    }
    Ok(())
}
fn write(conn: &Connection, record: &FindingRecord, insert: bool) -> Result<()> {
    validate_record(record)?;
    let bytes = encode(record)?;
    if bytes.len() > MAX_STORED_BYTES {
        return Err(invalid("finding exceeds encoded size limit"));
    }
    if insert {
        conn.execute("INSERT INTO findings(id,state,created_at_ms,creation_sha256,record_json,record_sha256) VALUES(?1,?2,?3,?4,?5,?6)",params![record.draft.request.id.to_string(),state_name(record.state),record.created_at_ms as i64,hash(&encode(&record.draft)?).as_slice(),bytes,hash(&bytes).as_slice()])?;
    } else {
        conn.execute(
            "UPDATE findings SET state=?2,record_json=?3,record_sha256=?4 WHERE id=?1",
            params![
                record.draft.request.id.to_string(),
                state_name(record.state),
                bytes,
                hash(&bytes).as_slice()
            ],
        )?;
    }
    Ok(())
}
impl WorkStore {
    /// Identical complete-draft replay returns retained current work, even closed.
    pub fn create_finding(&mut self, draft: &FindingDraft) -> Result<FindingRecord> {
        draft.validate()?;
        let tx = self.conn.transaction()?;
        if let Some(record) = read(&tx, draft.request.id)? {
            if hash(&encode(draft)?) != hash(&encode(&record.draft)?) {
                return Err(Error::OperationConflict(
                    "finding UUID has another initial draft".into(),
                ));
            }
            return Ok(record);
        }
        let now = now_ms();
        let record = FindingRecord {
            draft: draft.clone(),
            version: 1,
            state: FindingState::Open,
            created_at_ms: now,
            updated_at_ms: now,
        };
        write(&tx, &record, true)?;
        tx.commit()?;
        Ok(record)
    }
    /// Checked retained work, without a fresh vault observation or substitution.
    pub fn finding(&self, id: Uuid) -> Result<Option<FindingRecord>> {
        read(&self.conn, id)
    }
    /// One SQLite snapshot binds the page, cursor and operational open count.
    pub fn findings(&self, request: &FindingListRequest) -> Result<FindingPage> {
        request.validate()?;
        let tx = self.conn.unchecked_transaction()?;
        check_all(&tx)?;
        let cursor = request
            .before
            .map(|id| {
                read(&tx, id)?
                    .ok_or_else(|| Error::NotFound("finding cursor does not exist".into()))
            })
            .transpose()?;
        let created = cursor.as_ref().map(|r| r.created_at_ms as i64);
        let ids = {
            let mut statement=tx.prepare("SELECT id FROM findings WHERE (?1 IS NULL OR state=?1) AND (?2 IS NULL OR created_at_ms<?2 OR (created_at_ms=?2 AND id<?3)) ORDER BY created_at_ms DESC,id DESC LIMIT ?4")?;
            statement
                .query_map(
                    params![
                        request.state.map(state_name),
                        created,
                        request.before.map(|id| id.to_string()),
                        (request.limit + 1) as i64
                    ],
                    |row| row.get::<_, String>(0),
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut entries = ids
            .into_iter()
            .map(|id| {
                read(&tx, crate::parse_id(id)?)?
                    .ok_or_else(|| invalid("listed finding disappeared"))
            })
            .collect::<Result<Vec<_>>>()?;
        let more = entries.len() > request.limit;
        entries.truncate(request.limit);
        let next_before = more.then(|| {
            entries
                .last()
                .expect("positive page limit")
                .draft
                .request
                .id
        });
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM findings WHERE state='open'",
            [],
            |row| row.get(0),
        )?;
        let open_count =
            usize::try_from(count).map_err(|_| invalid("finding open count is out of range"))?;
        tx.commit()?;
        Ok(FindingPage {
            entries,
            next_before,
            open_count,
        })
    }
    /// One terminal transition; exact old-stamp/same-outcome replay is immutable.
    pub fn close_finding(&mut self, request: &CloseFindingRequest) -> Result<FindingRecord> {
        request.validate()?;
        let tx = self.conn.transaction()?;
        let mut record = read(&tx, request.expected.id)?
            .ok_or_else(|| Error::NotFound("finding does not exist".into()))?;
        if request.expected.version == 1 && record.version == 2 && record.state == request.state {
            return Ok(record);
        }
        if record.stamp() != request.expected || record.state != FindingState::Open {
            return Err(Error::StateChanged(
                "finding review version or state changed".into(),
            ));
        }
        record.version = 2;
        record.state = request.state;
        record.updated_at_ms = now_ms().max(record.updated_at_ms);
        write(&tx, &record, false)?;
        tx.commit()?;
        Ok(record)
    }
}
// Existing proof DTOs predate strict finding JSON. These local wire shapes make
// nested proof decoding strict without changing historical Save/proposal JSON.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictFingerprint {
    device: u64,
    inode: u64,
    len: u64,
    sha256: [u8; 32],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictSource {
    path: String,
    fingerprint: StrictFingerprint,
}
fn strict_source<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<SourceVersion, D::Error> {
    let source = StrictSource::deserialize(d)?;
    let f = source.fingerprint;
    Ok(SourceVersion {
        path: source.path,
        fingerprint: FileFingerprint {
            device: f.device,
            inode: f.inode,
            len: f.len,
            sha256: f.sha256,
        },
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictIdentity {
    device: u64,
    inode: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictVault {
    id: Uuid,
    root: PathBuf,
    identity: StrictIdentity,
}
fn strict_vault<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<VaultRecord, D::Error> {
    let vault = StrictVault::deserialize(d)?;
    Ok(VaultRecord {
        id: vault.id,
        root: vault.root,
        identity: VaultIdentity {
            device: vault.identity.device,
            inode: vault.identity.inode,
        },
    })
}
