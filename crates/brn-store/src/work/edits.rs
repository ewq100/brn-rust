use super::{MAX_NOTE_BYTES, WorkStore, now_ms};
use crate::{Result, invalid};
use rusqlite::{OptionalExtension, Row, params};

/// Unsaved editor text for one note, kept so it can be offered back after a crash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsavedEdit {
    pub path: String,
    /// SHA-256 of the saved note the edit started from.
    pub base_sha256: [u8; 32],
    pub text: String,
    pub updated_at_ms: u64,
}

fn read(row: &Row<'_>) -> rusqlite::Result<(String, Vec<u8>, String, i64)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn convert((path, base, text, updated): (String, Vec<u8>, String, i64)) -> Result<UnsavedEdit> {
    let base_sha256 = base
        .try_into()
        .map_err(|_| invalid("stored unsaved edit has an invalid hash"))?;
    Ok(UnsavedEdit {
        path,
        base_sha256,
        text,
        updated_at_ms: updated.max(0) as u64,
    })
}

impl WorkStore {
    pub fn put_unsaved_edit(
        &mut self,
        path: &str,
        base_sha256: [u8; 32],
        text: &str,
    ) -> Result<()> {
        if path.is_empty() {
            return Err(invalid("unsaved edit needs a note path"));
        }
        if text.len() > MAX_NOTE_BYTES {
            return Err(invalid("unsaved edit exceeds the 1 MiB note limit"));
        }
        self.conn.execute(
            "INSERT INTO unsaved_edits(path, base_sha256, text, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET base_sha256 = excluded.base_sha256,
                 text = excluded.text, updated_at_ms = excluded.updated_at_ms",
            params![path, &base_sha256[..], text, now_ms() as i64],
        )?;
        Ok(())
    }

    pub fn unsaved_edit(&self, path: &str) -> Result<Option<UnsavedEdit>> {
        self.conn
            .query_row(
                "SELECT path, base_sha256, text, updated_at_ms FROM unsaved_edits WHERE path = ?1",
                [path],
                read,
            )
            .optional()?
            .map(convert)
            .transpose()
    }

    pub fn unsaved_edits(&self) -> Result<Vec<UnsavedEdit>> {
        let mut statement = self.conn.prepare(
            "SELECT path, base_sha256, text, updated_at_ms FROM unsaved_edits ORDER BY path",
        )?;
        let rows = statement.query_map([], read)?;
        rows.map(|row| convert(row?)).collect()
    }

    pub fn clear_unsaved_edit(&mut self, path: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM unsaved_edits WHERE path = ?1", [path])?;
        Ok(())
    }
}
