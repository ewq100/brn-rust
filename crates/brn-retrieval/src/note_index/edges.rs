//! Rebuildable saved-note relationships. These records confer no authority.
use super::{KnowledgeScope, NoteIndex, NoteIndexReader};
use crate::{Error, Result};
use rusqlite::{Connection, Row, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeOrigin {
    ExplicitLink,
    InferredProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeEndpoint {
    pub path: String,
    pub note_id: Uuid,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceEndpoint {
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeEvidence {
    pub endpoint: EvidenceEndpoint,
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteEdge {
    pub source: EdgeEndpoint,
    pub target: EdgeEndpoint,
    pub origin: EdgeOrigin,
    pub evidence: Vec<EdgeEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgePage {
    pub scope: KnowledgeScope,
    pub offset: usize,
    pub total: usize,
    pub edges: Vec<NoteEdge>,
}

const MAX_NOTE_BYTES: usize = 1024 * 1024;
const MAX_PROOFS: usize = 8192;
const MAX_QUOTE_BYTES: usize = 4 * 1024 * 1024;

pub(super) const SCHEMA: &str = "
CREATE TABLE edges (
    source_path TEXT NOT NULL REFERENCES notes(path) ON DELETE CASCADE,
    target_path TEXT NOT NULL REFERENCES notes(path) ON DELETE CASCADE,
    origin TEXT NOT NULL CHECK(origin IN ('explicit_link','inferred_provenance')),
    payload TEXT NOT NULL,
    PRIMARY KEY(source_path,target_path,origin)
);";

impl EdgeOrigin {
    fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitLink => "explicit_link",
            Self::InferredProvenance => "inferred_provenance",
        }
    }
}

struct CachedNote {
    id: Uuid,
    sha256: [u8; 32],
    text: String,
}
struct Validator<'a> {
    conn: &'a Connection,
    notes: BTreeMap<String, CachedNote>,
}
impl<'a> Validator<'a> {
    fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            notes: BTreeMap::new(),
        }
    }
    fn endpoint(&mut self, endpoint: &EdgeEndpoint) -> Result<()> {
        if endpoint.note_id.is_nil() || endpoint.path.is_empty() {
            return Err(Error::Invalid(
                "edge endpoint needs a nonnil identity and path",
            ));
        }
        if !self.notes.contains_key(&endpoint.path) {
            let cached = cached_note(self.conn, &endpoint.path)?;
            self.notes.insert(endpoint.path.clone(), cached);
        }
        let cached = &self.notes[&endpoint.path];
        if cached.id != endpoint.note_id || cached.sha256 != endpoint.sha256 {
            return Err(Error::Invalid(
                "edge endpoint identity or hash differs from cached note",
            ));
        }
        Ok(())
    }
    fn edge(&mut self, edge: &NoteEdge) -> Result<()> {
        if edge.source.note_id == edge.target.note_id || edge.source.path == edge.target.path {
            return Err(Error::Invalid(
                "edge endpoints must have distinct identities and paths",
            ));
        }
        self.endpoint(&edge.source)?;
        self.endpoint(&edge.target)?;
        if edge.evidence.is_empty() || edge.evidence.len() > MAX_PROOFS {
            return Err(Error::Invalid(
                "edge needs between one and 8192 exact proofs",
            ));
        }
        let expected = match edge.origin {
            EdgeOrigin::ExplicitLink => EvidenceEndpoint::Source,
            EdgeOrigin::InferredProvenance => EvidenceEndpoint::Target,
        };
        let path = match expected {
            EvidenceEndpoint::Source => &edge.source.path,
            EvidenceEndpoint::Target => &edge.target.path,
        };
        let text = &self.notes[path].text;
        let mut bytes = 0usize;
        let mut proofs = HashSet::new();
        for proof in &edge.evidence {
            bytes = bytes
                .checked_add(proof.quote.len())
                .filter(|sum| *sum <= MAX_QUOTE_BYTES)
                .ok_or(Error::Invalid("edge quotes exceed the 4 MiB limit"))?;
            if proof.endpoint != expected
                || proof.start_byte >= proof.end_byte
                || proof.end_byte > MAX_NOTE_BYTES
                || proof.end_byte - proof.start_byte != proof.quote.len()
                || text.get(proof.start_byte..proof.end_byte) != Some(proof.quote.as_str())
                || !proofs.insert((proof.start_byte, proof.end_byte, proof.quote.as_str()))
            {
                return Err(Error::Invalid(
                    "edge proof is duplicate or differs from exact endpoint bytes",
                ));
            }
        }
        Ok(())
    }
}

fn cached_note(conn: &Connection, path: &str) -> Result<CachedNote> {
    let metadata =
        super::note_metadata(conn, path)?.ok_or(Error::Invalid("edge endpoint is absent"))?;
    let id = metadata
        .note_id
        .filter(|_| metadata.issue.is_none())
        .ok_or(Error::Invalid("edge endpoint is unmanaged or ineligible"))?;
    let aliases: i64 = conn.query_row(
        "SELECT count(*) FROM notes WHERE note_id=?1",
        [id.to_string()],
        |row| row.get(0),
    )?;
    if aliases != 1 {
        return Err(Error::Invalid(
            "edge endpoint identity is not unique in cached notes",
        ));
    }
    let (size, hash): (i64, Vec<u8>) = conn.query_row(
        "SELECT size,sha256 FROM notes WHERE path=?1",
        [path],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let size = usize::try_from(size)
        .ok()
        .filter(|size| *size <= MAX_NOTE_BYTES)
        .ok_or(Error::Corrupt("edge endpoint note size"))?;
    let sha256: [u8; 32] = hash
        .try_into()
        .map_err(|_| Error::Corrupt("edge endpoint note hash"))?;
    let mut text = String::with_capacity(size);
    let mut statement = conn.prepare(
        "SELECT start_byte,end_byte,text FROM passages WHERE path=?1 ORDER BY start_byte,id",
    )?;
    let mut rows = statement.query([path])?;
    while let Some(row) = rows.next()? {
        let start: i64 = row.get(0)?;
        let end: i64 = row.get(1)?;
        let passage: String = row.get(2)?;
        if usize::try_from(start).ok() != Some(text.len())
            || end <= start
            || usize::try_from(end).ok() != text.len().checked_add(passage.len())
            || usize::try_from(end).ok().is_none_or(|end| end > size)
        {
            return Err(Error::Corrupt("edge endpoint passage coverage"));
        }
        text.push_str(&passage);
    }
    if text.len() != size || <[u8; 32]>::from(Sha256::digest(text.as_bytes())) != sha256 {
        return Err(Error::Corrupt("edge endpoint full saved-byte proof"));
    }
    Ok(CachedNote { id, sha256, text })
}

fn encoded_limit(source: &str, target: &str) -> Result<usize> {
    source
        .len()
        .checked_add(target.len())
        .and_then(|paths| paths.checked_mul(6))
        .and_then(|paths| paths.checked_add(MAX_QUOTE_BYTES * 6 + MAX_PROOFS * 256 + 2048))
        .ok_or(Error::Corrupt("encoded edge size"))
}
fn decode(row: &Row<'_>) -> Result<NoteEdge> {
    let source: String = row.get(0)?;
    let target: String = row.get(1)?;
    let origin: String = row.get(2)?;
    let bytes: i64 = row.get(3)?;
    if usize::try_from(bytes)
        .ok()
        .is_none_or(|bytes| bytes > encoded_limit(&source, &target).unwrap_or(0))
    {
        return Err(Error::Corrupt("encoded edge exceeds bounded proofs"));
    }
    let payload: String = row.get(4)?;
    let edge: NoteEdge =
        serde_json::from_str(&payload).map_err(|_| Error::Corrupt("stored edge payload"))?;
    if edge.source.path != source || edge.target.path != target || edge.origin.as_str() != origin {
        return Err(Error::Corrupt("stored edge row and payload disagree"));
    }
    Ok(edge)
}
fn stored_error(error: Error) -> Error {
    match error {
        Error::Invalid(_) | Error::Format(_) => Error::Corrupt("stored edge endpoint or proof"),
        Error::Sql(error)
            if !matches!(
                error.sqlite_error_code(),
                Some(
                    rusqlite::ErrorCode::DatabaseBusy
                        | rusqlite::ErrorCode::DatabaseLocked
                        | rusqlite::ErrorCode::SystemIoFailure
                        | rusqlite::ErrorCode::CannotOpen
                )
            ) =>
        {
            Error::Corrupt("stored edge schema or value")
        }
        other => other,
    }
}
const ROW_COLUMNS: &str =
    "e.source_path,e.target_path,e.origin,length(CAST(e.payload AS BLOB)),e.payload";

pub(super) fn validate_rows(conn: &Connection) -> Result<()> {
    let result = (|| {
        let mut validator = Validator::new(conn);
        let mut pairs = BTreeSet::new();
        let mut statement = conn.prepare(&format!("SELECT {ROW_COLUMNS} FROM edges e"))?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let edge = decode(row)?;
            if !pairs.insert((
                edge.source.path.clone(),
                edge.target.path.clone(),
                edge.origin,
            )) {
                return Err(Error::Corrupt("duplicate stored edge"));
            }
            validator.edge(&edge)?;
        }
        Ok(())
    })();
    result.map_err(stored_error)
}

pub(super) fn valid_schema(conn: &Connection) -> Result<bool> {
    let expected = [
        ("source_path", "TEXT", 1, 1),
        ("target_path", "TEXT", 1, 2),
        ("origin", "TEXT", 1, 3),
        ("payload", "TEXT", 1, 0),
    ];
    let actual = conn
        .prepare("PRAGMA table_info(edges)")?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if actual.len() != expected.len()
        || actual
            .iter()
            .zip(expected)
            .any(|((name, kind, nullable, pk), expected)| {
                (name.as_str(), kind.as_str(), *nullable, *pk) != expected
            })
    {
        return Ok(false);
    }
    let foreign = conn
        .prepare("PRAGMA foreign_key_list(edges)")?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .collect::<rusqlite::Result<BTreeSet<_>>>()?;
    Ok(foreign
        == BTreeSet::from([
            (
                "notes".into(),
                "source_path".into(),
                "path".into(),
                "CASCADE".into(),
            ),
            (
                "notes".into(),
                "target_path".into(),
                "path".into(),
                "CASCADE".into(),
            ),
        ]))
}

impl NoteIndex {
    /// Atomically replace only disposable relationships; no knowledge is approved.
    pub fn replace_edges(&mut self, edges: &[NoteEdge]) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut validator = Validator::new(&tx);
        let mut pairs = BTreeSet::new();
        for edge in edges {
            validator.edge(edge)?;
            if !pairs.insert((&edge.source.path, &edge.target.path, edge.origin)) {
                return Err(Error::Invalid("duplicate endpoint pair and edge origin"));
            }
        }
        tx.execute("DELETE FROM edges", [])?;
        for edge in edges {
            tx.execute(
                "INSERT INTO edges(source_path,target_path,origin,payload) VALUES(?1,?2,?3,?4)",
                params![
                    edge.source.path,
                    edge.target.path,
                    edge.origin.as_str(),
                    serde_json::to_string(edge)?
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn edges(&self, scope: KnowledgeScope, offset: usize, limit: usize) -> Result<EdgePage> {
        page(&self.conn, scope, offset, limit)
    }
}
impl NoteIndexReader {
    pub fn edges(&self, scope: KnowledgeScope, offset: usize, limit: usize) -> Result<EdgePage> {
        page(&self.conn, scope, offset, limit)
    }
}
fn page(conn: &Connection, scope: KnowledgeScope, offset: usize, limit: usize) -> Result<EdgePage> {
    if !(1..=200).contains(&limit) {
        return Err(Error::Invalid(
            "edge page limit must be between one and 200",
        ));
    }
    let tx = conn.unchecked_transaction()?;
    if !valid_schema(&tx)? {
        return Err(Error::Corrupt("stored edge schema"));
    }
    validate_rows(&tx)?;
    let joins = format!(
        "FROM edges e JOIN notes s ON s.path=e.source_path JOIN notes t ON t.path=e.target_path WHERE {} AND {}",
        scope.predicate().replace("n.", "s."),
        scope.predicate().replace("n.", "t.")
    );
    let count: i64 = tx.query_row(&format!("SELECT count(*) {joins}"), [], |row| row.get(0))?;
    let total = usize::try_from(count).map_err(|_| Error::Corrupt("edge page total"))?;
    let mut edges = Vec::new();
    if offset < total {
        let offset_sql = i64::try_from(offset).map_err(|_| Error::Invalid("edge page offset"))?;
        let mut statement = tx.prepare(&format!("SELECT {ROW_COLUMNS} {joins} ORDER BY e.source_path,e.target_path,e.origin LIMIT ?1 OFFSET ?2"))?;
        let mut rows = statement.query(params![limit as i64, offset_sql])?;
        while let Some(row) = rows.next()? {
            edges.push(decode(row).map_err(stored_error)?);
        }
    }
    tx.commit()?;
    Ok(EdgePage {
        scope,
        offset,
        total,
        edges,
    })
}

pub(super) fn invalidate(conn: &Connection, path: &str, ids: &[Option<Uuid>]) -> Result<()> {
    conn.execute(
        "DELETE FROM edges WHERE source_path=?1 OR target_path=?1",
        [path],
    )?;
    for id in ids.iter().flatten() {
        conn.execute("DELETE FROM edges WHERE source_path IN (SELECT path FROM notes WHERE note_id=?1) OR target_path IN (SELECT path FROM notes WHERE note_id=?1)",[id.to_string()])?;
    }
    Ok(())
}
