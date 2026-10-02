//! Disposable search index over the vault's notes (`index.sqlite`): note
//! metadata, passages, FTS5 keyword search and local embeddings. It can be
//! deleted at any time and rebuilt from the vault.
mod schema;
mod search;

use crate::{Error, Result, chunk};
use rusqlite::{Connection, OptionalExtension, Row, params};
use sha2::{Digest, Sha256};
use std::path::Path;

pub use search::{NoteHit, check_query, fuse_hits};

/// A note as last seen in the vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedNote {
    /// Vault-relative path with `/` separators.
    pub path: String,
    pub title: String,
    pub size: u64,
    pub modified_ns: i64,
    pub sha256: [u8; 32],
}

pub struct NoteIndex {
    conn: Connection,
}

const NOTE_COLUMNS: &str = "path, title, size, modified_ns, sha256";
type NoteRow = (String, String, i64, i64, Vec<u8>);

fn note_row(row: &Row<'_>) -> rusqlite::Result<NoteRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn to_note((path, title, size, modified_ns, sha): NoteRow) -> Result<IndexedNote> {
    let sha256 = sha
        .try_into()
        .map_err(|_| Error::Corrupt("index note hash"))?;
    Ok(IndexedNote {
        path,
        title,
        size: size.max(0) as u64,
        modified_ns,
        sha256,
    })
}

impl NoteIndex {
    /// Opens the index at `path`. A missing, damaged or outdated index is
    /// deleted and created empty; the second value is `true` when that happened.
    pub fn open(path: &Path) -> Result<(Self, bool)> {
        let (conn, created) = schema::open(path)?;
        Ok((Self { conn }, created))
    }

    /// All indexed notes, ordered by path.
    pub fn notes(&self) -> Result<Vec<IndexedNote>> {
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {NOTE_COLUMNS} FROM notes ORDER BY path"))?;
        let rows = statement.query_map([], note_row)?;
        rows.map(|row| to_note(row?)).collect()
    }

    pub fn note(&self, path: &str) -> Result<Option<IndexedNote>> {
        self.conn
            .query_row(
                &format!("SELECT {NOTE_COLUMNS} FROM notes WHERE path = ?1"),
                [path],
                note_row,
            )
            .optional()?
            .map(to_note)
            .transpose()
    }

    /// Replaces the note's record and passages (dropping their embeddings).
    /// `text` must match `note.size` and `note.sha256`.
    pub fn upsert_note(&mut self, note: &IndexedNote, text: &str) -> Result<()> {
        let digest: [u8; 32] = Sha256::digest(text.as_bytes()).into();
        if text.len() as u64 != note.size || digest != note.sha256 {
            return Err(Error::Invalid("note text does not match its size and hash"));
        }
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM notes WHERE path = ?1", [&note.path])?;
        tx.execute(
            &format!("INSERT INTO notes({NOTE_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5)"),
            params![
                note.path,
                note.title,
                note.size as i64,
                note.modified_ns,
                &note.sha256[..]
            ],
        )?;
        {
            let mut insert = tx.prepare(
                "INSERT INTO passages(path, start_byte, end_byte, text) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for (start, end) in chunk::passages(text) {
                insert.execute(params![
                    note.path,
                    start as i64,
                    end as i64,
                    &text[start..end]
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Records a new size and modification time for a note whose content is unchanged.
    pub fn update_metadata(&mut self, path: &str, size: u64, modified_ns: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE notes SET size = ?2, modified_ns = ?3 WHERE path = ?1",
            params![path, size as i64, modified_ns],
        )?;
        Ok(())
    }

    /// Removes the note with its passages and embeddings.
    pub fn remove_note(&mut self, path: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM notes WHERE path = ?1", [path])?;
        Ok(())
    }
}
