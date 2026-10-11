//! Disposable search index over the vault's notes (`index.sqlite`): note
//! metadata, passages, FTS5 keyword search and local embeddings. It can be
//! deleted at any time and rebuilt from the vault.
mod edges;
mod embeddings;
mod schema;
mod search;

use crate::{Error, Result, chunk};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

pub use edges::{EdgeEndpoint, EdgeEvidence, EdgeOrigin, EdgePage, EvidenceEndpoint, NoteEdge};
pub use embeddings::{Embedder, EmbeddingProgress};
pub use search::{NoteHit, check_query, fuse_hits};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeScope {
    #[default]
    Current,
    Source,
    History,
    All,
}

impl KnowledgeScope {
    pub fn includes(self, source: bool, history: bool) -> bool {
        match self {
            Self::Current => !source && !history,
            Self::Source => source,
            Self::History => history,
            Self::All => true,
        }
    }

    pub(super) fn predicate(self) -> &'static str {
        match self {
            Self::Current => "n.metadata_issue IS NULL AND n.source = 0 AND n.history = 0",
            Self::Source => "n.metadata_issue IS NULL AND n.source = 1",
            Self::History => "n.metadata_issue IS NULL AND n.history = 1",
            Self::All => "n.metadata_issue IS NULL",
        }
    }
}

/// Caller-classified disposable metadata. UUID duplication is reported by the
/// workflow's fresh saved-evidence inventory, never resolved by this index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoteMetadata {
    pub note_id: Option<Uuid>,
    pub source: bool,
    pub history: bool,
    pub issue: Option<String>,
}

impl NoteMetadata {
    fn validate(&self) -> Result<()> {
        if self.note_id.is_some_and(|id| id.is_nil()) {
            return Err(Error::Invalid("note identity is nil"));
        }
        Ok(())
    }
}

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
    fn keyword_scoped(
        &self,
        query: &str,
        limit: usize,
        scope: KnowledgeScope,
    ) -> Result<Vec<NoteHit>> {
        if scope != KnowledgeScope::Current {
            return Err(Error::Invalid("search scope is unsupported"));
        }
        self.keyword(query, limit)
    }
    fn semantic_for_model(
        &self,
        vector: &[f32],
        model_identity: &str,
        limit: usize,
    ) -> Result<Vec<NoteHit>>;
    fn semantic_for_model_scoped(
        &self,
        vector: &[f32],
        model_identity: &str,
        limit: usize,
        scope: KnowledgeScope,
    ) -> Result<Vec<NoteHit>> {
        if scope != KnowledgeScope::Current {
            return Err(Error::Invalid("search scope is unsupported"));
        }
        self.semantic_for_model(vector, model_identity, limit)
    }
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
        self.notes_scoped(KnowledgeScope::Current)
    }

    pub fn notes_scoped(&self, scope: KnowledgeScope) -> Result<Vec<IndexedNote>> {
        notes(&self.conn, Some(scope))
    }

    pub fn note_metadata(&self, path: &str) -> Result<Option<NoteMetadata>> {
        note_metadata(&self.conn, path)
    }

    pub fn keyword_scoped(
        &self,
        query: &str,
        limit: usize,
        scope: KnowledgeScope,
    ) -> Result<Vec<NoteHit>> {
        search::keyword(&self.conn, query, limit, scope)
    }

    pub fn semantic_for_model_scoped(
        &self,
        vector: &[f32],
        model_identity: &str,
        limit: usize,
        scope: KnowledgeScope,
    ) -> Result<Vec<NoteHit>> {
        embeddings::semantic_for_model(&self.conn, vector, Some(model_identity), limit, scope)
    }
}

impl NoteSearch for NoteIndexReader {
    fn keyword(&self, query: &str, limit: usize) -> Result<Vec<NoteHit>> {
        search::keyword(&self.conn, query, limit, KnowledgeScope::Current)
    }
    fn keyword_scoped(
        &self,
        query: &str,
        limit: usize,
        scope: KnowledgeScope,
    ) -> Result<Vec<NoteHit>> {
        search::keyword(&self.conn, query, limit, scope)
    }
    fn semantic_for_model(
        &self,
        vector: &[f32],
        model_identity: &str,
        limit: usize,
    ) -> Result<Vec<NoteHit>> {
        embeddings::semantic_for_model(
            &self.conn,
            vector,
            Some(model_identity),
            limit,
            KnowledgeScope::Current,
        )
    }
    fn semantic_for_model_scoped(
        &self,
        vector: &[f32],
        model_identity: &str,
        limit: usize,
        scope: KnowledgeScope,
    ) -> Result<Vec<NoteHit>> {
        embeddings::semantic_for_model(&self.conn, vector, Some(model_identity), limit, scope)
    }
}

impl NoteSearch for NoteIndex {
    fn keyword(&self, query: &str, limit: usize) -> Result<Vec<NoteHit>> {
        search::keyword(&self.conn, query, limit, KnowledgeScope::Current)
    }
    fn keyword_scoped(
        &self,
        query: &str,
        limit: usize,
        scope: KnowledgeScope,
    ) -> Result<Vec<NoteHit>> {
        search::keyword(&self.conn, query, limit, scope)
    }
    fn semantic_for_model(
        &self,
        vector: &[f32],
        model_identity: &str,
        limit: usize,
    ) -> Result<Vec<NoteHit>> {
        embeddings::semantic_for_model(
            &self.conn,
            vector,
            Some(model_identity),
            limit,
            KnowledgeScope::Current,
        )
    }
    fn semantic_for_model_scoped(
        &self,
        vector: &[f32],
        model_identity: &str,
        limit: usize,
        scope: KnowledgeScope,
    ) -> Result<Vec<NoteHit>> {
        embeddings::semantic_for_model(&self.conn, vector, Some(model_identity), limit, scope)
    }
}

const NOTE_COLUMNS: &str = "path, title, size, modified_ns, sha256";
const METADATA_COLUMNS: &str = "note_id, source, history, metadata_issue";
type NoteRow = (String, String, i64, i64, Vec<u8>);
type MetadataRow = (Option<String>, i64, i64, Option<String>);

pub(super) fn metadata_row(row: &Row<'_>) -> rusqlite::Result<MetadataRow> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

pub(super) fn to_metadata((id, source, history, issue): MetadataRow) -> Result<NoteMetadata> {
    if !matches!(source, 0 | 1) || !matches!(history, 0 | 1) {
        return Err(Error::Corrupt("index note classification"));
    }
    let note_id = id
        .map(|id| {
            let parsed = Uuid::parse_str(&id).map_err(|_| Error::Corrupt("index note identity"))?;
            if parsed.is_nil() || parsed.to_string() != id {
                return Err(Error::Corrupt("index note identity"));
            }
            Ok(parsed)
        })
        .transpose()?;
    Ok(NoteMetadata {
        note_id,
        source: source == 1,
        history: history == 1,
        issue,
    })
}

fn note_metadata(conn: &Connection, path: &str) -> Result<Option<NoteMetadata>> {
    conn.query_row(
        &format!("SELECT {METADATA_COLUMNS} FROM notes WHERE path = ?1"),
        [path],
        metadata_row,
    )
    .optional()?
    .map(to_metadata)
    .transpose()
}

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

    /// An entirely rebuildable process-local index; no vault or user data discovery.
    pub fn in_memory() -> Result<Self> {
        Ok(Self {
            conn: schema::memory()?,
        })
    }

    /// Eligible current knowledge, ordered by path.
    pub fn notes(&self) -> Result<Vec<IndexedNote>> {
        self.notes_scoped(KnowledgeScope::Current)
    }

    pub fn notes_scoped(&self, scope: KnowledgeScope) -> Result<Vec<IndexedNote>> {
        notes(&self.conn, Some(scope))
    }

    /// Every derived row, including classification issues, for refresh/removal.
    pub fn all_notes(&self) -> Result<Vec<IndexedNote>> {
        notes(&self.conn, None)
    }

    pub fn note_metadata(&self, path: &str) -> Result<Option<NoteMetadata>> {
        note_metadata(&self.conn, path)
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
        self.upsert_note_with_metadata(note, text, &NoteMetadata::default())
    }

    pub fn upsert_note_with_metadata(
        &mut self,
        note: &IndexedNote,
        text: &str,
        metadata: &NoteMetadata,
    ) -> Result<()> {
        metadata.validate()?;
        let digest: [u8; 32] = Sha256::digest(text.as_bytes()).into();
        if text.len() as u64 != note.size || digest != note.sha256 {
            return Err(Error::Invalid("note text does not match its size and hash"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous = note_metadata(&tx, &note.path)?.and_then(|metadata| metadata.note_id);
        edges::invalidate(&tx, &note.path, &[previous, metadata.note_id])?;
        tx.execute("DELETE FROM notes WHERE path = ?1", [&note.path])?;
        tx.execute(
            &format!("INSERT INTO notes({NOTE_COLUMNS}, {METADATA_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"),
            params![
                note.path,
                note.title,
                note.size as i64,
                note.modified_ns,
                &note.sha256[..],
                metadata.note_id.map(|id| id.to_string()),
                metadata.source,
                metadata.history,
                metadata.issue,
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

    /// Classification-only changes retain exact passages and cached vectors.
    pub fn update_note_metadata(&mut self, path: &str, metadata: &NoteMetadata) -> Result<()> {
        metadata.validate()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous = note_metadata(&tx, path)?;
        if previous.is_none() || previous.as_ref() == Some(metadata) {
            return Ok(());
        }
        edges::invalidate(
            &tx,
            path,
            &[
                previous.and_then(|metadata| metadata.note_id),
                metadata.note_id,
            ],
        )?;
        tx.execute("UPDATE notes SET note_id = ?2, source = ?3, history = ?4, metadata_issue = ?5 WHERE path = ?1",
            params![path, metadata.note_id.map(|id| id.to_string()), metadata.source, metadata.history, metadata.issue])?;
        tx.commit()?;
        Ok(())
    }

    /// Records a new size and modification time for a note whose content is unchanged.
    pub fn update_metadata(&mut self, path: &str, size: u64, modified_ns: i64) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let old_size = tx
            .query_row("SELECT size FROM notes WHERE path=?1", [path], |row| {
                row.get::<_, i64>(0)
            })
            .optional()?;
        if old_size.is_some_and(|old| old != size as i64) {
            edges::invalidate(&tx, path, &[])?;
        }
        tx.execute(
            "UPDATE notes SET size = ?2, modified_ns = ?3 WHERE path = ?1",
            params![path, size as i64, modified_ns],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Removes the note with its passages and embeddings.
    pub fn remove_note(&mut self, path: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM notes WHERE path = ?1", [path])?;
        Ok(())
    }
}

fn notes(conn: &Connection, scope: Option<KnowledgeScope>) -> Result<Vec<IndexedNote>> {
    let predicate = scope
        .map(|scope| format!("WHERE {}", scope.predicate()))
        .unwrap_or_default();
    let mut statement = conn.prepare(&format!(
        "SELECT {NOTE_COLUMNS} FROM notes n {predicate} ORDER BY path"
    ))?;
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
