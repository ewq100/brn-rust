//! Bounded, read-only inspection pages for hosts (the CLI): receipts,
//! unapplied candidates, records, runs, one operation, and pages of the
//! app's notes, chats, needs-you, projects, open edit sessions and one chat's
//! messages. SELECT only with bound parameters; no transaction, no write, and
//! nothing here is used by the desktop.
use crate::*;
use rusqlite::{OptionalExtension, params};

/// Where an operation stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    /// A receipt exists.
    Applied,
    /// Prepared, not retired, no receipt.
    Prepared,
    /// Retired before it was applied.
    Retired,
    /// Allocated under a host key, never prepared.
    Allocated,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationInfo {
    pub id: OperationId,
    pub state: OperationState,
    pub undo_of: Option<OperationId>,
    pub replacement_of: Option<OperationId>,
    pub prepared: Option<Prepared>,
    pub receipt: Option<Receipt>,
}
/// One unapplied candidate and its position for the next page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRow {
    pub rowid: i64,
    pub prepared: Prepared,
    pub undo_of: Option<OperationId>,
    pub replacement_of: Option<OperationId>,
}
/// The `RecordData` variant names accepted as a record kind filter.
pub const RECORD_KINDS: [&str; 12] = [
    "Note",
    "Thread",
    "Action",
    "Source",
    "Asset",
    "Link",
    "Comment",
    "Message",
    "Run",
    "Settings",
    "ChatControl",
    "Project",
];
/// `hash, retired, undo_of, replacement_of` of one operations row.
type OperationRow = (Option<String>, i64, Option<String>, Option<String>);
const MAX_PAGE: usize = 100;
fn page_limit(limit: usize) -> Result<i64> {
    if !(1..=MAX_PAGE).contains(&limit) {
        return Err(Error::Invalid("page limit must be 1–100".into()));
    }
    Ok(limit as i64 + 1)
}
/// Keeps `limit` rows and reports whether more followed.
fn split<T>(mut rows: Vec<T>, limit: usize) -> (Vec<T>, bool) {
    let more = rows.len() > limit;
    rows.truncate(limit);
    (rows, more)
}
fn decode<T: serde::de::DeserializeOwned>(body: &str) -> Result<T> {
    Ok(serde_json::from_str(body)?)
}
impl Store {
    fn require_record(&self, id: &str) -> Result<()> {
        let found: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM records WHERE id=?1)",
            [id],
            |r| r.get(0),
        )?;
        if found {
            Ok(())
        } else {
            Err(Error::Missing(id.into()))
        }
    }
    /// Receipts after `after` (a receipt rowid) in commit order, optionally
    /// only those that wrote `record`.
    pub fn receipt_page(
        &self,
        after: i64,
        limit: usize,
        record: Option<&str>,
    ) -> Result<(Vec<(i64, Receipt)>, bool)> {
        let fetch = page_limit(limit)?;
        let rows: Vec<(i64, String)> = match record {
            Some(id) => {
                self.require_record(id)?;
                let mut stmt = self.conn.prepare(
                    "SELECT rowid,body FROM receipts WHERE rowid>?1 AND operation IN (SELECT operation FROM revisions WHERE id=?2) ORDER BY rowid LIMIT ?3",
                )?;
                stmt.query_map(params![after, id, fetch], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .collect::<std::result::Result<_, _>>()?
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT rowid,body FROM receipts WHERE rowid>?1 ORDER BY rowid LIMIT ?2",
                )?;
                stmt.query_map(params![after, fetch], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .collect::<std::result::Result<_, _>>()?
            }
        };
        let rows = rows
            .into_iter()
            .map(|(rowid, body)| Ok((rowid, decode(&body)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(split(rows, limit))
    }
    /// The number of receipts, optionally only those that wrote `record`.
    pub fn receipt_count(&self, record: Option<&str>) -> Result<u64> {
        let count: i64 = match record {
            Some(id) => {
                self.require_record(id)?;
                self.conn.query_row(
                    "SELECT COUNT(*) FROM receipts WHERE operation IN (SELECT operation FROM revisions WHERE id=?1)",
                    [id],
                    |r| r.get(0),
                )?
            }
            None => self
                .conn
                .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))?,
        };
        Ok(count as u64)
    }
    /// Unapplied candidates after `after` (an operations rowid), with the same
    /// filter as `candidates`.
    pub fn candidate_page(&self, after: i64, limit: usize) -> Result<(Vec<CandidateRow>, bool)> {
        let fetch = page_limit(limit)?;
        let mut stmt = self.conn.prepare(
            "SELECT rowid,id,undo_of,replacement_of FROM operations WHERE hash IS NOT NULL AND retired=0 AND NOT EXISTS(SELECT 1 FROM receipts WHERE operation=operations.id) AND rowid>?1 ORDER BY rowid LIMIT ?2",
        )?;
        let rows: Vec<(i64, String, Option<String>, Option<String>)> = stmt
            .query_map(params![after, fetch], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })?
            .collect::<std::result::Result<_, _>>()?;
        let rows = rows
            .into_iter()
            .map(|(rowid, id, undo_of, replacement_of)| {
                Ok(CandidateRow {
                    rowid,
                    prepared: self.prepared(&OperationId(id))?,
                    undo_of: undo_of.map(OperationId),
                    replacement_of: replacement_of.map(OperationId),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(split(rows, limit))
    }
    /// Records with an ID after `after`, in ID order, optionally of one kind.
    pub fn record_page(
        &self,
        kind: Option<&str>,
        after: &str,
        limit: usize,
    ) -> Result<(Vec<Record>, bool)> {
        let fetch = page_limit(limit)?;
        let rows: Vec<String> = match kind {
            Some(kind) => {
                if !RECORD_KINDS.contains(&kind) {
                    return Err(Error::Invalid("unknown record kind".into()));
                }
                let path = format!("$.data.{kind}");
                let mut stmt = self.conn.prepare(
                    "SELECT body FROM records WHERE json_type(body,?1) IS NOT NULL AND id>?2 ORDER BY id LIMIT ?3",
                )?;
                stmt.query_map(params![path, after, fetch], |r| r.get(0))?
                    .collect::<std::result::Result<_, _>>()?
            }
            None => {
                let mut stmt = self
                    .conn
                    .prepare("SELECT body FROM records WHERE id>?1 ORDER BY id LIMIT ?2")?;
                stmt.query_map(params![after, fetch], |r| r.get(0))?
                    .collect::<std::result::Result<_, _>>()?
            }
        };
        let rows = rows
            .iter()
            .map(|body| decode(body))
            .collect::<Result<Vec<Record>>>()?;
        Ok(split(rows, limit))
    }
    /// Run records with an ID after `after`, optionally in one chat or state.
    pub fn run_page(
        &self,
        thread: Option<&str>,
        state: Option<RunState>,
        after: &str,
        limit: usize,
    ) -> Result<(Vec<Record>, bool)> {
        let fetch = page_limit(limit)?;
        let state = state
            .map(|state| serde_json::to_value(state).map(|v| v.as_str().map(str::to_owned)))
            .transpose()?
            .flatten();
        let mut stmt = self.conn.prepare(
            "SELECT body FROM records WHERE json_type(body,'$.data.Run') IS NOT NULL AND (?1 IS NULL OR json_extract(body,'$.data.Run.thread')=?1) AND (?2 IS NULL OR json_extract(body,'$.data.Run.state')=?2) AND id>?3 ORDER BY id LIMIT ?4",
        )?;
        let rows: Vec<String> = stmt
            .query_map(params![thread, state, after, fetch], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let rows = rows
            .iter()
            .map(|body| decode(body))
            .collect::<Result<Vec<Record>>>()?;
        Ok(split(rows, limit))
    }
    /// Decoded record bodies from `sql` (which selects one `body` column and
    /// fetches `limit + 1` rows), split into a page and whether more followed.
    fn body_page(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
        limit: usize,
    ) -> Result<(Vec<Record>, bool)> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows: Vec<String> = stmt
            .query_map(params, |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let rows = rows
            .iter()
            .map(|body| decode(body))
            .collect::<Result<Vec<Record>>>()?;
        Ok(split(rows, limit))
    }
    /// Current general notes with an ID after `after`, in ID order: the
    /// app's `notes()` filter (not archived, not superseded, no chat).
    pub fn note_page(&self, after: &str, limit: usize) -> Result<(Vec<Record>, bool)> {
        let fetch = page_limit(limit)?;
        self.body_page(
            "SELECT body FROM records WHERE json_type(body,'$.data.Note') IS NOT NULL AND json_extract(body,'$.archived')=0 AND json_extract(body,'$.data.Note.superseded_by') IS NULL AND json_extract(body,'$.data.Note.chat') IS NULL AND id>?1 ORDER BY id LIMIT ?2",
            params![after, fetch],
            limit,
        )
    }
    /// Chats that are not archived, after `after` in ID order: the app's `threads()` filter.
    pub fn thread_page(&self, after: &str, limit: usize) -> Result<(Vec<Record>, bool)> {
        let fetch = page_limit(limit)?;
        self.body_page(
            "SELECT body FROM records WHERE json_type(body,'$.data.Thread') IS NOT NULL AND json_extract(body,'$.archived')=0 AND id>?1 ORDER BY id LIMIT ?2",
            params![after, fetch],
            limit,
        )
    }
    /// Chats with attention, archived ones included, after `after` in ID
    /// order: the app's `needs_you()` filter.
    pub fn needs_you_page(&self, after: &str, limit: usize) -> Result<(Vec<Record>, bool)> {
        let fetch = page_limit(limit)?;
        self.body_page(
            "SELECT body FROM records WHERE json_type(body,'$.data.Thread') IS NOT NULL AND json_array_length(body,'$.data.Thread.attention')>0 AND id>?1 ORDER BY id LIMIT ?2",
            params![after, fetch],
            limit,
        )
    }
    /// Projects after `after` in ID order, trashed ones only with
    /// `include_trash`: the app's `projects()` filter.
    pub fn project_page(
        &self,
        include_trash: bool,
        after: &str,
        limit: usize,
    ) -> Result<(Vec<Record>, bool)> {
        let fetch = page_limit(limit)?;
        self.body_page(
            "SELECT body FROM records WHERE json_type(body,'$.data.Project') IS NOT NULL AND (?1 OR json_extract(body,'$.archived')=0) AND id>?2 ORDER BY id LIMIT ?3",
            params![include_trash, after, fetch],
            limit,
        )
    }
    /// Open edit sessions with an ID after `after`, in ID order, as
    /// `recovery_buffers()` returns them.
    pub fn recovery_page(&self, after: &str, limit: usize) -> Result<(Vec<EditSession>, bool)> {
        let fetch = page_limit(limit)?;
        let mut stmt = self.conn.prepare(
            "SELECT id FROM edit_sessions WHERE closed=0 AND id>?1 ORDER BY id LIMIT ?2",
        )?;
        let ids: Vec<String> = stmt
            .query_map(params![after, fetch], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let sessions = ids
            .iter()
            .map(|id| self.edit_session(id))
            .collect::<Result<Vec<_>>>()?;
        Ok(split(sessions, limit))
    }
    /// The non-archived Message records of `thread`, in creation order, after
    /// position `after`: the app's `messages()` filter and order. A message's
    /// position is the rowid of its first revision; revisions and receipts are
    /// inserted in write order inside one transaction, so this is the app's
    /// (receipt, write) order, and an edit keeps the original position.
    pub fn message_page(
        &self,
        thread: &str,
        after: i64,
        limit: usize,
    ) -> Result<(Vec<(i64, Record)>, bool)> {
        let fetch = page_limit(limit)?;
        let mut stmt = self.conn.prepare(
            "SELECT pos,body FROM (SELECT (SELECT MIN(v.rowid) FROM revisions v WHERE v.id=r.id) AS pos, r.body AS body FROM records r WHERE json_type(r.body,'$.data.Message') IS NOT NULL AND json_extract(r.body,'$.data.Message.thread')=?1 AND json_extract(r.body,'$.archived')=0) WHERE pos>?2 ORDER BY pos LIMIT ?3",
        )?;
        let rows: Vec<(i64, String)> = stmt
            .query_map(params![thread, after, fetch], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<std::result::Result<_, _>>()?;
        let rows = rows
            .into_iter()
            .map(|(pos, body)| Ok((pos, decode(&body)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(split(rows, limit))
    }
    /// The operation allocated under exactly this host key; never allocates.
    pub fn operation_by_host_key(&self, key: &str) -> Result<Option<OperationId>> {
        Ok(self
            .conn
            .query_row("SELECT id FROM operations WHERE host_key=?1", [key], |r| {
                r.get::<_, String>(0)
            })
            .optional()?
            .map(OperationId))
    }
    /// Where one operation stands, with its request and receipt when present.
    pub fn operation_info(&self, id: &str) -> Result<OperationInfo> {
        let row: Option<OperationRow> = self
            .conn
            .query_row(
                "SELECT hash,retired,undo_of,replacement_of FROM operations WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let (hash, retired, undo_of, replacement_of) =
            row.ok_or_else(|| Error::Missing(id.into()))?;
        let operation = OperationId(id.into());
        let receipt = self.receipt(&operation)?;
        let prepared = match hash {
            Some(_) => Some(self.prepared(&operation)?),
            None => None,
        };
        let state = if receipt.is_some() {
            OperationState::Applied
        } else if retired != 0 {
            OperationState::Retired
        } else if prepared.is_some() {
            OperationState::Prepared
        } else {
            OperationState::Allocated
        };
        Ok(OperationInfo {
            id: operation,
            state,
            undo_of: undo_of.map(OperationId),
            replacement_of: replacement_of.map(OperationId),
            prepared,
            receipt,
        })
    }
}
