//! Disposable search index over the vault's notes (`index.sqlite`): note
//! metadata, passages, FTS5 keyword search and local embeddings. It can be
//! deleted at any time and rebuilt from the vault.
mod embeddings;
mod schema;
mod search;

use crate::{Error, Result, chunk};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, params};
use sha2::{Digest, Sha256};
use std::path::Path;

pub use embeddings::{Embedder, EmbeddingProgress};
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

pub trait NoteSearch {
    fn keyword(&self, query: &str, limit: usize) -> Result<Vec<NoteHit>>;
    fn semantic_for_model(
        &self,
        vector: &[f32],
        model_identity: &str,
        limit: usize,
    ) -> Result<Vec<NoteHit>>;
}

/// A query-only connection. Opening a reader never creates or rebuilds the index.
pub struct NoteIndexReader {
    conn: Connection,
}

impl NoteIndexReader {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.pragma_update(None, "query_only", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        schema::validate_reader(&conn)?;
        Ok(Self { conn })
    }

    pub fn notes(&self) -> Result<Vec<IndexedNote>> {
        notes(&self.conn)
    }
}

impl NoteSearch for NoteIndexReader {
    fn keyword(&self, query: &str, limit: usize) -> Result<Vec<NoteHit>> {
        search::keyword(&self.conn, query, limit)
    }
    fn semantic_for_model(
        &self,
        vector: &[f32],
        model_identity: &str,
        limit: usize,
    ) -> Result<Vec<NoteHit>> {
        embeddings::semantic_for_model(&self.conn, vector, Some(model_identity), limit)
    }
}

impl NoteSearch for NoteIndex {
    fn keyword(&self, query: &str, limit: usize) -> Result<Vec<NoteHit>> {
        search::keyword(&self.conn, query, limit)
    }
    fn semantic_for_model(
        &self,
        vector: &[f32],
        model_identity: &str,
        limit: usize,
    ) -> Result<Vec<NoteHit>> {
        embeddings::semantic_for_model(&self.conn, vector, Some(model_identity), limit)
    }
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
    /// Opens the index at `path`, checking its file header before SQLite opens it.
    /// Missing or zero-byte files are created empty. A BRNI-branded index is
    /// rebuilt if corrupt, outdated or missing required schema objects.
    /// An unbranded SQLite file is rebuilt only if healthy with no schema objects;
    /// damaged or nonempty unbranded databases are refused. Short/non-SQLite files
    /// and other nonzero application IDs are refused untouched, without SQLite.
    /// Other SQLite and I/O errors propagate. The second value is `true` when
    /// the index was created or rebuilt.
    pub fn open(path: &Path) -> Result<(Self, bool)> {
        let (conn, created) = schema::open(path)?;
        Ok((Self { conn }, created))
    }

    /// All indexed notes, ordered by path.
    pub fn notes(&self) -> Result<Vec<IndexedNote>> {
        notes(&self.conn)
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

fn notes(conn: &Connection) -> Result<Vec<IndexedNote>> {
    let mut statement = conn.prepare(&format!("SELECT {NOTE_COLUMNS} FROM notes ORDER BY path"))?;
    let rows = statement.query_map([], note_row)?;
    rows.map(|row| to_note(row?)).collect()
}

#[cfg(test)]
mod reader_tests {
    use super::*;

    #[test]
    fn reader_connection_refuses_sql_writes_even_while_writer_is_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.sqlite");
        let (writer, _) = NoteIndex::open(&path).unwrap();
        let reader = NoteIndexReader::open(&path).unwrap();
        let error = reader.conn.execute("DELETE FROM notes", []).unwrap_err();
        assert_eq!(
            error.sqlite_error_code(),
            Some(rusqlite::ErrorCode::ReadOnly)
        );
        assert_eq!(
            reader
                .conn
                .query_row("PRAGMA query_only", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(reader);
        drop(writer);
    }

    #[test]
    fn semantic_model_metadata_and_vectors_share_one_snapshot_during_switches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.sqlite");
        let (mut writer, _) = NoteIndex::open(&path).unwrap();
        writer
            .upsert_note(
                &IndexedNote {
                    path: "a.md".into(),
                    title: "A".into(),
                    size: 5,
                    modified_ns: 0,
                    sha256: Sha256::digest(b"apple").into(),
                },
                "apple",
            )
            .unwrap();
        writer.use_embedding_model("model-a", 2).unwrap();
        writer
            .conn
            .execute(
                "INSERT INTO embeddings SELECT id, ?1 FROM passages",
                [vec![1.0_f32, 0.0]
                    .into_iter()
                    .flat_map(f32::to_le_bytes)
                    .collect::<Vec<_>>()],
            )
            .unwrap();
        let reader = NoteIndexReader::open(&path).unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let ready = barrier.clone();
        let reads = std::thread::spawn(move || {
            ready.wait();
            for _ in 0..300 {
                match reader.semantic_for_model(&[1.0, 0.0], "model-a", 10) {
                    Ok(hits) => {
                        assert_eq!(hits.len(), 1);
                        assert_eq!(hits[0].score, 1.0);
                    }
                    Err(Error::ModelMismatch) => {}
                    other => panic!("unexpected snapshot result: {other:?}"),
                }
            }
        });
        barrier.wait();
        for i in 0..300 {
            let (identity, vector) = if i % 2 == 0 {
                ("model-b|2", [0.0_f32, 1.0])
            } else {
                ("model-a|2", [1.0_f32, 0.0])
            };
            let tx = writer.conn.transaction().unwrap();
            tx.execute(
                "UPDATE meta SET value=?1 WHERE key='embedding_model'",
                [identity],
            )
            .unwrap();
            tx.execute(
                "UPDATE embeddings SET vector=?1",
                [vector
                    .into_iter()
                    .flat_map(f32::to_le_bytes)
                    .collect::<Vec<_>>()],
            )
            .unwrap();
            tx.commit().unwrap();
        }
        reads.join().unwrap();
    }
}
