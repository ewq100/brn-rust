//! Read-only snapshots for derived data such as the search index. They never
//! upgrade or write the canonical database, and check visibility over only
//! the records a candidate references instead of the whole workspace.
use crate::store::{has_symlink_ancestor, validate_connection};
use crate::*;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Duration,
};

/// Where a current note may appear in search. `General` notes are visible in
/// every context; `Chat` notes only in that conversation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SearchEligibility {
    General,
    Chat(String),
    Hidden,
}

/// Derived from [`context::visible`] itself, so it cannot drift from core
/// rules. Archived and superseded notes are hidden, as in `records_for_chat`.
/// `rows` must contain the record's references (see [`Snapshot::with_references`]).
pub fn search_eligibility(record: &Record, rows: &[Record]) -> SearchEligibility {
    let RecordData::Note(note) = &record.data else {
        return SearchEligibility::Hidden;
    };
    if record.archived || note.superseded_by.is_some() {
        return SearchEligibility::Hidden;
    }
    if crate::context::visible(record, rows, None) {
        return SearchEligibility::General;
    }
    // Only the note's own chat or its import source's chat can admit it.
    let source_chat = note.import.as_ref().and_then(|import| {
        rows.iter()
            .find(|r| r.id == import.source)
            .and_then(|r| match &r.data {
                RecordData::Source(source) => source.chat.clone(),
                _ => None,
            })
    });
    [note.chat.clone(), source_chat]
        .into_iter()
        .flatten()
        .find(|chat| crate::context::visible(record, rows, Some(chat)))
        .map_or(SearchEligibility::Hidden, SearchEligibility::Chat)
}

pub struct StoreReader {
    conn: Connection,
}

impl Store {
    /// Opens a validated, read-only view of an existing current-schema
    /// workspace. Older schemas are refused rather than upgraded.
    pub fn open_reader(directory: impl AsRef<Path>) -> Result<StoreReader> {
        let directory = directory.as_ref();
        if !directory.is_absolute()
            || has_symlink_ancestor(directory)?
            || std::fs::read_to_string(directory.join(DATA_MARKER))? != "BRN Threads SQLite v1\n"
        {
            return Err(Error::UnsafeDirectory(
                "validated explicit Threads directory required".into(),
            ));
        }
        let conn = Connection::open_with_flags(
            directory.join(DATABASE_FILENAME),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.busy_timeout(Duration::from_secs(5))?;
        validate_connection(&conn)?;
        Ok(StoreReader { conn })
    }
}

impl StoreReader {
    /// One consistent read snapshot; it ends when dropped.
    pub fn snapshot(&mut self) -> Result<Snapshot<'_>> {
        Ok(Snapshot {
            tx: self.conn.transaction()?,
        })
    }
}

pub struct Snapshot<'a> {
    tx: Transaction<'a>,
}

fn record_from(body: String) -> Result<Record> {
    Ok(serde_json::from_str(&body)?)
}

impl Snapshot<'_> {
    /// Rises with every applied record write (all writes add a revision in
    /// the same transaction); 0 for a workspace without writes.
    pub fn watermark(&self) -> Result<i64> {
        Ok(self
            .tx
            .query_row("SELECT coalesce(max(rowid), 0) FROM revisions", [], |r| {
                r.get(0)
            })?)
    }
    /// The workspace's first operation, distinguishing workspaces whose
    /// watermarks happen to match.
    pub fn workspace_identity(&self) -> Result<Option<String>> {
        Ok(self
            .tx
            .query_row(
                "SELECT id FROM operations ORDER BY rowid LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn records(&self) -> Result<Vec<Record>> {
        let mut stmt = self.tx.prepare("SELECT body FROM records ORDER BY id")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|row| record_from(row?)).collect()
    }
    /// Current records written after `watermark`, in id order.
    pub fn changed_since(&self, watermark: i64) -> Result<Vec<Record>> {
        let mut stmt = self.tx.prepare(
            "SELECT body FROM records WHERE id IN \
             (SELECT id FROM revisions WHERE rowid > ?1) ORDER BY id",
        )?;
        let rows = stmt.query_map([watermark], |r| r.get::<_, String>(0))?;
        rows.map(|row| record_from(row?)).collect()
    }
    pub fn record(&self, id: &str) -> Result<Option<Record>> {
        self.tx
            .query_row("SELECT body FROM records WHERE id=?1", params![id], |r| {
                r.get::<_, String>(0)
            })
            .optional()?
            .map(record_from)
            .transpose()
    }
    /// The records named by `ids` plus everything core visibility follows
    /// from them: import sources, supersession targets, asset and comment
    /// notes, and link ends. Missing ids are skipped.
    pub fn with_references(&self, ids: &[String]) -> Result<Vec<Record>> {
        let mut found = BTreeMap::new();
        let mut pending: Vec<String> = ids.to_vec();
        while let Some(id) = pending.pop() {
            if found.contains_key(&id) {
                continue;
            }
            let Some(record) = self.record(&id)? else {
                continue;
            };
            match &record.data {
                RecordData::Note(note) => {
                    pending.extend(note.import.as_ref().map(|i| i.source.clone()));
                    pending.extend(note.superseded_by.clone());
                }
                RecordData::Asset(asset) => {
                    pending.push(asset.note.clone());
                    pending.extend(asset.source.clone());
                }
                RecordData::Comment(comment) => pending.push(comment.note.clone()),
                RecordData::Link(link) => {
                    pending.push(link.from.clone());
                    pending.push(link.to.clone());
                }
                _ => {}
            }
            found.insert(id, record);
        }
        Ok(found.into_values().collect())
    }
    /// Of `ids`, the current, non-archived, non-superseded notes visible in
    /// `chat` (`None` = general), matching `records_for_chat` for notes.
    pub fn searchable_notes(&self, ids: &[String], chat: Option<&str>) -> Result<BTreeSet<String>> {
        let rows = self.with_references(ids)?;
        Ok(ids
            .iter()
            .filter_map(|id| rows.iter().find(|r| &r.id == id))
            .filter(|r| match search_eligibility(r, &rows) {
                SearchEligibility::General => true,
                SearchEligibility::Chat(owner) => Some(owner.as_str()) == chat,
                SearchEligibility::Hidden => false,
            })
            .map(|r| r.id.clone())
            .collect())
    }
}
