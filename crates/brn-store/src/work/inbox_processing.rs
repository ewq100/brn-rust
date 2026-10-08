//! Bounded owned Inbox conversion jobs. Originals stay in ordinary intake files;
//! receipts are operational review work, never approved source evidence.
use super::{MAX_NOTE_BYTES, WorkStore, inbox::InboxItem, now_ms};
use crate::{Error, Result, hash, invalid};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_PROCESS_BATCH: usize = 8;
pub const MAX_PENDING_ITEMS: usize = 16;
const MAX_RECORD_BYTES: usize = 512 * 1024;
pub(super) const V13: &str = "
CREATE TABLE inbox_processing (
 id TEXT PRIMARY KEY,
 queued_at_ms INTEGER NOT NULL CHECK(queued_at_ms>=0),
 pending INTEGER NOT NULL CHECK(pending IN (0,1)),
 record_json BLOB NOT NULL,
 record_sha256 BLOB NOT NULL CHECK(length(record_sha256)=32)
);
CREATE INDEX inbox_processing_queue ON inbox_processing(pending,queued_at_ms,id);";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessInboxRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<brn_intake::IntakeLimits>,
    pub id: Uuid,
    pub items: Vec<InboxItem>,
}
impl ProcessInboxRequest {
    pub fn validate(&self) -> Result<()> {
        if let Some(limits) = &self.limits {
            limits.validate().map_err(crate::Error::Invalid)?;
        }
        if self.id.is_nil() || !(1..=MAX_PROCESS_BATCH).contains(&self.items.len()) {
            return Err(invalid(
                "Inbox processing needs a UUID and 1 to 8 exact items",
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for item in &self.items {
            item.validate()?;
            if !ids.insert(item.capture.id) {
                return Err(invalid("Inbox processing contains a duplicate item"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxConversionFormat {
    VerbatimMarkdownV1,
    LiteralTextV1,
    MaintainedExtractionV1,
    DocxTextV1,
    DocxInlinePngV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum InboxProcessOutcome {
    Queued,
    Running,
    Converted {
        format: InboxConversionFormat,
        byte_len: u64,
        sha256: [u8; 32],
    },
    Failed {
        code: String,
    },
    Cancelled,
    Interrupted,
}
impl InboxProcessOutcome {
    pub fn pending(&self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxProcessEntry {
    pub outcome: InboxProcessOutcome,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxProcessBatch {
    pub request: ProcessInboxRequest,
    pub queued_at_ms: u64,
    pub entries: Vec<InboxProcessEntry>,
}
impl InboxProcessBatch {
    pub fn pending_count(&self) -> usize {
        self.entries.iter().filter(|e| e.outcome.pending()).count()
    }
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        if self.queued_at_ms > i64::MAX as u64 || self.entries.len() != self.request.items.len() {
            return Err(invalid("Inbox processing time or entries are invalid"));
        }
        for (entry, item) in self.entries.iter().zip(&self.request.items) {
            if entry
                .started_at_ms
                .is_some_and(|t| t < self.queued_at_ms || t > i64::MAX as u64)
                || entry.finished_at_ms.is_some_and(|t| {
                    t < entry.started_at_ms.unwrap_or(self.queued_at_ms) || t > i64::MAX as u64
                })
            {
                return Err(invalid("Inbox processing timestamps are invalid"));
            }
            let valid = match &entry.outcome {
                InboxProcessOutcome::Queued => {
                    entry.started_at_ms.is_none() && entry.finished_at_ms.is_none()
                }
                InboxProcessOutcome::Running => {
                    entry.started_at_ms.is_some() && entry.finished_at_ms.is_none()
                }
                InboxProcessOutcome::Converted {
                    format,
                    byte_len,
                    sha256,
                } => {
                    let exact_format = if *format == InboxConversionFormat::MaintainedExtractionV1 {
                        *byte_len != 0
                    } else {
                        match item.capture.kind {
                            super::inbox::InboxKind::Markdown => {
                                *format == InboxConversionFormat::VerbatimMarkdownV1
                                    && *byte_len == item.capture.copy.byte_len
                                    && *sha256 == item.capture.copy.sha256
                            }
                            super::inbox::InboxKind::Binary => {
                                matches!(
                                    format,
                                    InboxConversionFormat::DocxTextV1
                                        | InboxConversionFormat::DocxInlinePngV1
                                ) && (22..=super::inbox::MAX_INBOX_BINARY_BYTES as u64)
                                    .contains(&item.capture.copy.byte_len)
                                    && (*byte_len != 0 || *sha256 == hash(&[]))
                            }
                            _ => {
                                *format == InboxConversionFormat::LiteralTextV1
                                    && *byte_len >= item.capture.copy.byte_len + 12
                                    && (item.capture.copy.byte_len != 0
                                        || (*byte_len == 13
                                            && *sha256 == hash(b"```text\n\n```\n")))
                            }
                        }
                    };
                    exact_format
                        && *byte_len <= MAX_NOTE_BYTES as u64
                        && entry.started_at_ms.is_some()
                        && entry.finished_at_ms.is_some()
                }
                InboxProcessOutcome::Failed { code } => {
                    matches!(
                        code.as_str(),
                        "original_missing"
                            | "original_changed"
                            | "original_unavailable"
                            | "candidate_too_large"
                            | "binary_unsupported"
                            | "intake_failed"
                            | "intake_unavailable"
                            | "intake_timeout"
                            | "intake_quota"
                            | "intake_protocol"
                            | "intake_invalid"
                            | "docx_invalid"
                            | "docx_unsupported"
                            | "docx_limit"
                    ) && entry.started_at_ms.is_some()
                        && entry.finished_at_ms.is_some()
                }
                InboxProcessOutcome::Cancelled | InboxProcessOutcome::Interrupted => {
                    entry.finished_at_ms.is_some()
                }
            };
            if !valid {
                return Err(invalid("Inbox processing outcome is invalid"));
            }
        }
        if encode(self)?.len() > MAX_RECORD_BYTES {
            return Err(invalid("Inbox processing record exceeds its bound"));
        }
        Ok(())
    }
}
fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| invalid("could not encode Inbox processing"))
}
fn normalized(sql: &str) -> String {
    sql.trim()
        .trim_end_matches(';')
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect()
}
fn check_schema(conn: &Connection) -> Result<()> {
    for ((name, kind), sql) in [
        ("inbox_processing", "table"),
        ("inbox_processing_queue", "index"),
    ]
    .into_iter()
    .zip(V13.split(';').filter(|s| !s.trim().is_empty()))
    {
        let actual: Option<(String, String, String)> = conn
            .query_row(
                "SELECT type,tbl_name,sql FROM sqlite_schema WHERE name=?1",
                [name],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if !actual.is_some_and(|(k, table, s)| {
            k == kind && table == "inbox_processing" && normalized(&s) == normalized(sql)
        }) {
            return Err(invalid(
                "Inbox processing schema differs from its owned shape",
            ));
        }
    }
    let count: i64 = conn.query_row("SELECT count(*) FROM sqlite_schema WHERE tbl_name='inbox_processing' AND name NOT IN ('inbox_processing','inbox_processing_queue','sqlite_autoindex_inbox_processing_1')", [], |r| r.get(0))?;
    if count != 0 {
        return Err(invalid("Inbox processing has unexpected schema objects"));
    }
    Ok(())
}
pub(super) fn read(conn: &Connection, id: Uuid) -> Result<Option<InboxProcessBatch>> {
    let row: Option<(i64, i64, Vec<u8>, Vec<u8>)> = conn.query_row("SELECT queued_at_ms,pending,record_json,record_sha256 FROM inbox_processing WHERE id=?1 AND length(record_json)<=?2", params![id.to_string(), MAX_RECORD_BYTES as i64], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).optional()?;
    let Some((time, pending, bytes, sha)) = row else {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM inbox_processing WHERE id=?1)",
            [id.to_string()],
            |r| r.get(0),
        )?;
        return if exists {
            Err(invalid("Inbox processing record exceeds its bound"))
        } else {
            Ok(None)
        };
    };
    let batch: InboxProcessBatch =
        serde_json::from_slice(&bytes).map_err(|_| invalid("malformed Inbox processing record"))?;
    batch.validate()?;
    if batch.request.id != id
        || time < 0
        || time as u64 != batch.queued_at_ms
        || pending != i64::from(batch.pending_count() > 0)
        || sha != hash(&bytes)
        || encode(&batch)? != bytes
    {
        return Err(invalid("Inbox processing record bindings differ"));
    }
    for item in &batch.request.items {
        if super::inbox::read(conn, item.capture.id)?.as_ref() != Some(item) {
            return Err(invalid(
                "Inbox processing original snapshot differs from its catalog",
            ));
        }
    }
    Ok(Some(batch))
}
fn write(conn: &Connection, batch: &InboxProcessBatch) -> Result<()> {
    batch.validate()?;
    let bytes = encode(batch)?;
    conn.execute("INSERT INTO inbox_processing(id,queued_at_ms,pending,record_json,record_sha256) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET pending=excluded.pending,record_json=excluded.record_json,record_sha256=excluded.record_sha256", params![batch.request.id.to_string(), batch.queued_at_ms as i64, i64::from(batch.pending_count() > 0), bytes, hash(&bytes).as_slice()])?;
    Ok(())
}
pub(super) fn check_all(conn: &Connection) -> Result<()> {
    check_schema(conn)?;
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM inbox_processing")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for id in ids {
        let uuid = Uuid::parse_str(&id).map_err(|_| invalid("invalid Inbox processing UUID"))?;
        if uuid.to_string() != id || read(conn, uuid)?.is_none() {
            return Err(invalid("invalid Inbox processing row"));
        }
    }
    Ok(())
}
pub(super) fn reconcile(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    check_all(&tx)?;
    let ids: Vec<String> = tx
        .prepare("SELECT id FROM inbox_processing WHERE pending=1")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for id in ids {
        let mut batch =
            read(&tx, Uuid::parse_str(&id).expect("checked UUID"))?.expect("checked row");
        terminate(&mut batch, InboxProcessOutcome::Interrupted);
        write(&tx, &batch)?;
    }
    tx.commit()?;
    Ok(())
}
fn terminate(batch: &mut InboxProcessBatch, outcome: InboxProcessOutcome) {
    let time = now_ms().max(batch.queued_at_ms);
    for entry in &mut batch.entries {
        if entry.outcome.pending() {
            entry.outcome = outcome.clone();
            entry.finished_at_ms = Some(time.max(entry.started_at_ms.unwrap_or(0)));
        }
    }
}
impl WorkStore {
    pub fn process_inbox(&mut self, request: &ProcessInboxRequest) -> Result<InboxProcessBatch> {
        request.validate()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_all(&tx)?;
        if let Some(batch) = read(&tx, request.id)? {
            if batch.request != *request {
                return Err(Error::OperationConflict(
                    "Inbox processing UUID has another exact request".into(),
                ));
            }
            tx.commit()?;
            return Ok(batch);
        }
        for item in &request.items {
            if super::inbox::read(&tx, item.capture.id)?.as_ref() != Some(item) {
                return Err(Error::OperationConflict(
                    "Inbox processing needs the exact retained item snapshot".into(),
                ));
            }
        }
        let ids: Vec<String> = tx
            .prepare("SELECT id FROM inbox_processing WHERE pending=1")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut pending = 0;
        for id in ids {
            pending += read(&tx, Uuid::parse_str(&id).expect("checked UUID"))?
                .expect("checked row")
                .pending_count();
        }
        if pending + request.items.len() > MAX_PENDING_ITEMS {
            return Err(Error::OperationConflict(
                "Inbox processing queue is full (16 pending items)".into(),
            ));
        }
        let batch = InboxProcessBatch {
            request: request.clone(),
            queued_at_ms: now_ms(),
            entries: request
                .items
                .iter()
                .map(|_| InboxProcessEntry {
                    outcome: InboxProcessOutcome::Queued,
                    started_at_ms: None,
                    finished_at_ms: None,
                })
                .collect(),
        };
        write(&tx, &batch)?;
        tx.commit()?;
        Ok(batch)
    }
    pub fn inbox_processing(&self, id: Uuid) -> Result<Option<InboxProcessBatch>> {
        if id.is_nil() {
            return Err(invalid("Inbox processing UUID must not be nil"));
        }
        check_schema(&self.conn)?;
        read(&self.conn, id)
    }
    pub fn next_inbox_processing(&self) -> Result<Option<InboxProcessBatch>> {
        check_schema(&self.conn)?;
        let id: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM inbox_processing WHERE pending=1 ORDER BY queued_at_ms,id LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        id.map(|id| {
            read(
                &self.conn,
                Uuid::parse_str(&id).map_err(|_| invalid("invalid Inbox processing UUID"))?,
            )
            .and_then(|b| b.ok_or_else(|| invalid("missing queued Inbox processing")))
        })
        .transpose()
    }
    pub fn start_inbox_processing(&mut self, id: Uuid, index: usize) -> Result<InboxProcessBatch> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_schema(&tx)?;
        let mut batch = read(&tx, id)?
            .ok_or_else(|| Error::NotFound("Inbox processing does not exist".into()))?;
        let time = now_ms().max(batch.queued_at_ms);
        let entry = batch
            .entries
            .get_mut(index)
            .ok_or_else(|| invalid("invalid Inbox processing entry"))?;
        if entry.outcome != InboxProcessOutcome::Queued {
            return Err(Error::OperationConflict(
                "Inbox processing entry is not queued".into(),
            ));
        }
        entry.outcome = InboxProcessOutcome::Running;
        entry.started_at_ms = Some(time);
        write(&tx, &batch)?;
        tx.commit()?;
        Ok(batch)
    }
    pub fn finish_inbox_processing(
        &mut self,
        id: Uuid,
        index: usize,
        outcome: InboxProcessOutcome,
    ) -> Result<InboxProcessBatch> {
        if outcome.pending() || outcome == InboxProcessOutcome::Interrupted {
            return Err(invalid(
                "Inbox processing needs a terminal conversion outcome",
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_schema(&tx)?;
        let mut batch = read(&tx, id)?
            .ok_or_else(|| Error::NotFound("Inbox processing does not exist".into()))?;
        let entry = batch
            .entries
            .get_mut(index)
            .ok_or_else(|| invalid("invalid Inbox processing entry"))?;
        if entry.outcome != InboxProcessOutcome::Running {
            return Err(Error::OperationConflict(
                "Inbox processing entry is not running".into(),
            ));
        }
        entry.outcome = outcome;
        entry.finished_at_ms = Some(now_ms().max(entry.started_at_ms.expect("running entry")));
        write(&tx, &batch)?;
        tx.commit()?;
        Ok(batch)
    }
    pub fn cancel_inbox_processing(&mut self, id: Uuid) -> Result<InboxProcessBatch> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_schema(&tx)?;
        let mut batch = read(&tx, id)?
            .ok_or_else(|| Error::NotFound("Inbox processing does not exist".into()))?;
        terminate(&mut batch, InboxProcessOutcome::Cancelled);
        write(&tx, &batch)?;
        tx.commit()?;
        Ok(batch)
    }
}
