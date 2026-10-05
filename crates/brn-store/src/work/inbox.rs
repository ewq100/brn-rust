//! Operational catalog of retained original copies. Payload bytes belong to
//! ordinary intake files; only workflow may observe, install or remove them.
use super::{MAX_NOTE_BYTES, WorkStore, now_ms};
use crate::{Error, Result, hash, invalid};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::{Component, PathBuf};
use uuid::Uuid;

pub(super) const V12: &str = "
CREATE TABLE inbox_items (
 id TEXT PRIMARY KEY,
 received_at_ms INTEGER NOT NULL CHECK(received_at_ms>=0),
 capture_sha256 BLOB NOT NULL CHECK(length(capture_sha256)=32),
 record_json BLOB NOT NULL,
 record_sha256 BLOB NOT NULL CHECK(length(record_sha256)=32)
);
CREATE INDEX inbox_items_page ON inbox_items(received_at_ms,id);";
const MAX_LABEL_BYTES: usize = 512;
const MAX_DIRECTORY_BYTES: usize = 4096;
// Escaped labels/path plus fixed UUID, numeric and digest fields, never content.
const MAX_RECORD_BYTES: usize = 40 * 1024;

/// Deliberate text copies, not inferred MIME types or live account imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxKind {
    Text,
    Markdown,
    Email,
    Teams,
}

/// Exact proof of one original copy in a workflow-owned ordinary directory.
/// Its filename is derived from the item UUID; labels never become paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxCopy {
    pub directory: PathBuf,
    pub directory_device: u64,
    pub directory_inode: u64,
    pub file_device: u64,
    pub file_inode: u64,
    pub byte_len: u64,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxCapture {
    pub id: Uuid,
    pub kind: InboxKind,
    pub title: String,
    pub original_name: Option<String>,
    pub copy: InboxCopy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxItem {
    pub capture: InboxCapture,
    pub received_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxListRequest {
    pub limit: usize,
    pub after: Option<Uuid>,
}
impl Default for InboxListRequest {
    fn default() -> Self {
        Self {
            limit: 25,
            after: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxPage {
    pub entries: Vec<InboxItem>,
    pub next_after: Option<Uuid>,
    pub total_count: usize,
}

fn nonnil(id: Uuid) -> Result<()> {
    if id.is_nil() {
        return Err(invalid("Inbox item UUID must not be nil"));
    }
    Ok(())
}
fn label(text: &str) -> Result<()> {
    if text.trim().is_empty() || text.len() > MAX_LABEL_BYTES || text.chars().any(char::is_control)
    {
        return Err(invalid(
            "Inbox label must be visible and at most 512 UTF-8 bytes",
        ));
    }
    Ok(())
}
impl InboxCapture {
    /// Symbolic bounds only. Workflow must independently qualify durable copy
    /// identity/content before fresh admission; this API grants no file access.
    pub fn validate(&self) -> Result<()> {
        nonnil(self.id)?;
        label(&self.title)?;
        if let Some(name) = &self.original_name {
            label(name)?;
        }
        let path = self
            .copy
            .directory
            .to_str()
            .ok_or_else(|| invalid("Inbox directory must be UTF-8"))?;
        if !self.copy.directory.is_absolute()
            || path.len() > MAX_DIRECTORY_BYTES
            || path.chars().any(char::is_control)
            || path
                .split('/')
                .skip(1)
                .any(|part| part.is_empty() || part == "." || part == "..")
            || self
                .copy
                .directory
                .components()
                .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
            || self.copy.directory_inode == 0
            || self.copy.file_inode == 0
            || self.copy.file_device != self.copy.directory_device
            || self.copy.file_inode == self.copy.directory_inode
            || self.copy.byte_len > MAX_NOTE_BYTES as u64
            || self.copy.byte_len == 0 && self.copy.sha256 != hash(&[])
        {
            return Err(invalid(
                "Inbox needs a bounded absolute same-volume original-copy proof",
            ));
        }
        Ok(())
    }
    /// This fixed name is independent of user labels and the interpreted kind.
    pub fn copy_name(&self) -> String {
        format!("{}.txt", self.id)
    }
}
impl InboxListRequest {
    pub fn validate(&self) -> Result<()> {
        if !(1..=100).contains(&self.limit) {
            return Err(invalid("Inbox page limit must be 1 to 100"));
        }
        if let Some(id) = self.after {
            nonnil(id)?;
        }
        Ok(())
    }
}
fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| invalid("could not encode Inbox work"))
}
fn normalized_sql(sql: &str) -> String {
    sql.trim()
        .trim_end_matches(';')
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect()
}
fn check_schema(conn: &Connection) -> Result<()> {
    for ((name, kind), expected) in [("inbox_items", "table"), ("inbox_items_page", "index")]
        .into_iter()
        .zip(V12.split(';').filter(|sql| !sql.trim().is_empty()))
    {
        let actual: Option<(String, String, String)> = conn
            .query_row(
                "SELECT type,tbl_name,sql FROM sqlite_schema WHERE name=?1",
                [name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if !actual.is_some_and(|(k, table, sql)| {
            k == kind && table == "inbox_items" && normalized_sql(&sql) == normalized_sql(expected)
        }) {
            return Err(invalid(
                "Inbox schema or page index differs from its owned shape",
            ));
        }
    }
    let objects: Vec<(String, String)> = conn
        .prepare("SELECT type,name FROM sqlite_schema WHERE tbl_name='inbox_items' ORDER BY name")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if objects
        != [
            ("table".into(), "inbox_items".into()),
            ("index".into(), "inbox_items_page".into()),
            ("index".into(), "sqlite_autoindex_inbox_items_1".into()),
        ]
    {
        return Err(invalid(
            "Inbox schema contains unexpected or missing owned objects",
        ));
    }
    Ok(())
}
struct Row {
    received: i64,
    capture: Vec<u8>,
    bytes: Option<Vec<u8>>,
    digest: Vec<u8>,
}
fn read(conn: &Connection, id: Uuid) -> Result<Option<InboxItem>> {
    nonnil(id)?;
    let row = conn.query_row(
        "SELECT received_at_ms,capture_sha256,CASE WHEN length(record_json)<=?2 THEN record_json END,record_sha256 FROM inbox_items WHERE id=?1",
        params![id.to_string(), MAX_RECORD_BYTES as i64],
        |r| Ok(Row { received:r.get(0)?,capture:r.get(1)?,bytes:r.get(2)?,digest:r.get(3)? }),
    ).optional()?;
    row.map(|row| {
        let bytes = row
            .bytes
            .ok_or_else(|| invalid("stored Inbox record exceeds encoded size limit"))?;
        if row.digest.as_slice() != hash(&bytes) {
            return Err(invalid("stored Inbox record failed its hash check"));
        }
        let item: InboxItem =
            serde_json::from_slice(&bytes).map_err(|_| invalid("invalid stored Inbox record"))?;
        item.capture.validate()?;
        if item.capture.id != id
            || item.received_at_ms > i64::MAX as u64
            || i64::try_from(item.received_at_ms).ok() != Some(row.received)
            || row.capture.as_slice() != hash(&encode(&item.capture)?)
            || encode(&item)? != bytes
        {
            return Err(invalid(
                "stored Inbox work differs from its complete immutable capture or row binding",
            ));
        }
        Ok(item)
    })
    .transpose()
}
/// Semantic refusal precedes physical-corruption recovery and startup backup.
pub(super) fn check_all(conn: &Connection) -> Result<()> {
    check_schema(conn)?;
    for raw in conn
        .prepare("SELECT id FROM inbox_items ORDER BY id")?
        .query_map([], |r| r.get::<_, String>(0))?
    {
        let raw = raw?;
        let id = crate::parse_id(raw.clone())?;
        if raw != id.to_string() || read(conn, id)?.is_none() {
            return Err(invalid("invalid Inbox row identity"));
        }
    }
    Ok(())
}
impl WorkStore {
    /// Catalog an already-retained workflow-qualified original. No payload is
    /// stored and no filesystem write or deletion occurs. Exact UUID replay is
    /// immutable; an explicitly distinct UUID represents a separate copy.
    pub fn capture_inbox(&mut self, capture: &InboxCapture) -> Result<InboxItem> {
        capture.validate()?;
        let tx = self.conn.transaction()?;
        check_schema(&tx)?;
        if let Some(item) = read(&tx, capture.id)? {
            if item.capture != *capture {
                return Err(Error::OperationConflict(
                    "Inbox UUID has another original capture".into(),
                ));
            }
            return Ok(item);
        }
        let item = InboxItem {
            capture: capture.clone(),
            received_at_ms: now_ms(),
        };
        if item.received_at_ms > i64::MAX as u64 {
            return Err(invalid("Inbox received time is out of range"));
        }
        let bytes = encode(&item)?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(invalid("Inbox record exceeds encoded size limit"));
        }
        tx.execute("INSERT INTO inbox_items(id,received_at_ms,capture_sha256,record_json,record_sha256) VALUES(?1,?2,?3,?4,?5)", params![capture.id.to_string(),item.received_at_ms as i64,hash(&encode(capture)?).as_slice(),bytes,hash(&bytes).as_slice()])?;
        tx.commit()?;
        Ok(item)
    }
    /// Retained operational proof; callers still need fresh original validation.
    pub fn inbox_item(&self, id: Uuid) -> Result<Option<InboxItem>> {
        let tx = self.conn.unchecked_transaction()?;
        check_schema(&tx)?;
        let item = read(&tx, id)?;
        tx.commit()?;
        Ok(item)
    }
    /// Checked chronological FIFO page, UUID tie-break and count from one snapshot.
    pub fn inbox_items(&self, request: &InboxListRequest) -> Result<InboxPage> {
        request.validate()?;
        let tx = self.conn.unchecked_transaction()?;
        check_all(&tx)?;
        let cursor = request
            .after
            .map(|id| {
                read(&tx, id)?.ok_or_else(|| Error::NotFound("Inbox cursor does not exist".into()))
            })
            .transpose()?;
        let ids: Vec<String> = tx.prepare("SELECT id FROM inbox_items WHERE (?1 IS NULL OR received_at_ms>?1 OR (received_at_ms=?1 AND id>?2)) ORDER BY received_at_ms,id LIMIT ?3")?
            .query_map(params![cursor.map(|r| r.received_at_ms as i64),request.after.map(|id| id.to_string()),(request.limit+1) as i64], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut entries = ids
            .into_iter()
            .map(|raw| {
                read(&tx, crate::parse_id(raw)?)?
                    .ok_or_else(|| invalid("listed Inbox work disappeared"))
            })
            .collect::<Result<Vec<_>>>()?;
        let more = entries.len() > request.limit;
        entries.truncate(request.limit);
        let next_after = more.then(|| entries.last().expect("positive page limit").capture.id);
        let count: i64 = tx.query_row("SELECT count(*) FROM inbox_items", [], |r| r.get(0))?;
        let total_count =
            usize::try_from(count).map_err(|_| invalid("Inbox count is out of range"))?;
        tx.commit()?;
        Ok(InboxPage {
            entries,
            next_after,
            total_count,
        })
    }
}
