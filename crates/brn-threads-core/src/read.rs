use crate::*;
use serde::Serialize;

pub const MAX_NOTE_READ_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoteHeading {
    pub start: usize,
    pub level: usize,
    pub title: String,
}

/// Markdown ATX headings outside fenced blocks. Offsets always refer to stored bytes.
pub fn note_headings(markdown: &str) -> Vec<NoteHeading> {
    let mut headings = vec![];
    let mut start = 0;
    let mut fence: Option<(u8, usize)> = None;
    for line in markdown.split_inclusive('\n') {
        let trimmed = line.trim_start_matches(' ');
        let indent = line.len() - trimmed.len();
        if indent <= 3 {
            let marker = trimmed.as_bytes().first().copied();
            let count = marker.map_or(0, |marker| {
                trimmed.bytes().take_while(|c| *c == marker).count()
            });
            if matches!(marker, Some(b'`' | b'~')) && count >= 3 {
                match fence {
                    Some((old, width))
                        if marker == Some(old)
                            && count >= width
                            && trimmed[count..].trim().is_empty() =>
                    {
                        fence = None
                    }
                    None => fence = Some((marker.unwrap(), count)),
                    _ => {}
                }
            } else if fence.is_none()
                && marker == Some(b'#')
                && (1..=6).contains(&count)
                && trimmed
                    .as_bytes()
                    .get(count)
                    .is_some_and(u8::is_ascii_whitespace)
            {
                headings.push(NoteHeading {
                    start,
                    level: count,
                    title: trimmed[count..].trim().chars().take(160).collect(),
                });
            }
        }
        start += line.len();
    }
    headings
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoteSlice {
    pub id: String,
    pub version: u64,
    pub title: String,
    pub start: usize,
    pub end: usize,
    pub total_bytes: usize,
    pub markdown: String,
    /// Resume with the same version and `section=false`; the range remains bounded.
    pub next_start: Option<usize>,
    pub section_end: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome")]
pub enum NoteReadOutcome {
    Read {
        note: NoteSlice,
    },
    Stale {
        id: String,
        expected_version: u64,
        current_version: u64,
    },
}

impl Store {
    /// Read a bounded current revision, refusing to combine slices of different versions.
    /// `section` selects the heading beginning exactly at `start`, including subsections.
    pub fn read_note_range(
        &self,
        id: &str,
        version: u64,
        start: usize,
        max_bytes: usize,
        section: bool,
    ) -> Result<NoteReadOutcome> {
        if version == 0 || max_bytes == 0 || max_bytes > MAX_NOTE_READ_BYTES {
            return Err(Error::Invalid(
                "note reads require a version and 1..=131072 bytes".into(),
            ));
        }
        let record = self.record(id)?.ok_or_else(|| Error::Missing(id.into()))?;
        if record.archived
            || matches!(&record.data, RecordData::Note(note) if note.superseded_by.is_some())
        {
            return Err(Error::Missing(id.into()));
        }
        let RecordData::Note(note) = record.data else {
            return Err(Error::Invalid("not a note".into()));
        };
        if record.version != version {
            return Ok(NoteReadOutcome::Stale {
                id: id.into(),
                expected_version: version,
                current_version: record.version,
            });
        }
        if !note.markdown.is_char_boundary(start) {
            return Err(Error::Invalid(
                "note read start must be a valid UTF-8 byte offset".into(),
            ));
        }
        let section_end = if section {
            let headings = note_headings(&note.markdown);
            let heading = headings
                .iter()
                .find(|heading| heading.start == start)
                .ok_or_else(|| {
                    Error::Invalid("section start must identify an advertised heading".into())
                })?;
            Some(
                headings
                    .iter()
                    .find(|next| next.start > start && next.level <= heading.level)
                    .map_or(note.markdown.len(), |next| next.start),
            )
        } else {
            None
        };
        let limit = section_end.unwrap_or(note.markdown.len());
        let mut end = start.saturating_add(max_bytes).min(limit);
        while end > start && !note.markdown.is_char_boundary(end) {
            end -= 1;
        }
        // A too-small budget cannot return an empty slice with a non-advancing cursor.
        if end == start && start < limit {
            return Err(Error::Invalid(
                "byte budget is smaller than the next UTF-8 character".into(),
            ));
        }
        Ok(NoteReadOutcome::Read {
            note: NoteSlice {
                id: id.into(),
                version,
                title: note.title,
                start,
                end,
                total_bytes: note.markdown.len(),
                markdown: note.markdown[start..end].into(),
                next_start: (end < limit).then_some(end),
                section_end,
            },
        })
    }
}
