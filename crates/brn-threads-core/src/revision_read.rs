//! Explicit bounded inspection of accepted immutable note revisions.
use crate::*;
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevisionSummary {
    pub version: u64,
    pub operation: String,
    pub reason: String,
    pub authority: String,
    pub committed_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevisionPage {
    pub id: String,
    pub revisions: Vec<RevisionSummary>,
    pub next_after: Option<u64>,
}
impl Store {
    /// At most 32 summaries strictly after the supplied version. No note bodies
    /// or unaccepted candidates are loaded into the caller's history context.
    pub fn note_revisions(&self, id: &str, after: u64) -> Result<RevisionPage> {
        let record = self.record(id)?.ok_or_else(|| Error::Missing(id.into()))?;
        if !matches!(record.data, RecordData::Note(_)) {
            return Err(Error::Invalid("history requires a note".into()));
        }
        let mut statement = self.conn.prepare(
            "SELECT r.version,r.operation,json_extract(c.body,'$.reason'),json_extract(c.body,'$.authority'),json_extract(c.body,'$.committed_at') FROM revisions r JOIN receipts c ON c.operation=r.operation WHERE r.id=?1 AND r.version>?2 ORDER BY r.version LIMIT 33",
        )?;
        let mut revisions = statement
            .query_map(params![id, crate::integer(after)?], |row| {
                Ok(RevisionSummary {
                    version: crate::read_u64(row, 0)?,
                    operation: row.get(1)?,
                    reason: row.get::<_, String>(2)?.chars().take(512).collect(),
                    authority: row.get::<_, String>(3)?.chars().take(512).collect(),
                    committed_at: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let more = revisions.len() > 32;
        revisions.truncate(32);
        let next_after = more.then(|| revisions.last().unwrap().version);
        Ok(RevisionPage {
            id: id.into(),
            revisions,
            next_after,
        })
    }
    /// An explicitly named accepted revision stays readable after later edits,
    /// archiving, or supersession. This is evidence, never Current.
    pub fn read_note_revision(
        &self,
        id: &str,
        version: u64,
        start: usize,
        max_bytes: usize,
    ) -> Result<NoteSlice> {
        if version == 0 || max_bytes == 0 || max_bytes > MAX_NOTE_READ_BYTES {
            return Err(Error::Invalid(
                "revision reads require a version and 1..=131072 bytes".into(),
            ));
        }
        let record = self
            .revision(id, version)?
            .ok_or_else(|| Error::Missing(format!("{id}@{version}")))?;
        let RecordData::Note(note) = record.data else {
            return Err(Error::Invalid("revision is not a note".into()));
        };
        if !note.markdown.is_char_boundary(start) {
            return Err(Error::Invalid(
                "revision start must be a valid UTF-8 byte offset".into(),
            ));
        }
        let mut end = start.saturating_add(max_bytes).min(note.markdown.len());
        while end > start && !note.markdown.is_char_boundary(end) {
            end -= 1;
        }
        if end == start && start < note.markdown.len() {
            return Err(Error::Invalid(
                "byte budget is smaller than the next UTF-8 character".into(),
            ));
        }
        Ok(NoteSlice {
            id: id.into(),
            version,
            title: note.title,
            start,
            end,
            total_bytes: note.markdown.len(),
            markdown: note.markdown[start..end].into(),
            next_start: (end < note.markdown.len()).then_some(end),
            section_end: None,
        })
    }
}
