//! The workspace search index (`derived/search.sqlite`): a rebuildable mirror
//! of current notes and links with exact and English-stemmed FTS5 lanes.
//! It is never authoritative; callers re-check core visibility on results.
use crate::markdown_chunk::{CHUNKER_VERSION, chunks};
use crate::note_index::search::{check_query, fts_query};
use crate::{Error, Result};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
    time::Duration,
};

pub const FILE_NAME: &str = "search.sqlite";
const LOCK_NAME: &str = "search.lock";
const APPLICATION_ID: i64 = 0x4252_4e53; // BRNS
const SCHEMA_VERSION: i64 = 1;
const DEFAULT_LANGUAGE: &str = "en";
const NOT_OURS: &str = "derived search path holds a file that is not a BRN search index";
/// Reciprocal-rank fusion constant.
const RRF_K: f64 = 60.0;

/// Language analyzers. FTS5 fixes one tokenizer per table, so a later
/// language (for example Estonian) would write pre-stemmed text; adding one
/// changes `analyzer_version` and therefore rebuilds the index.
pub struct Analyzer {
    pub language: &'static str,
    pub stem_tokenizer: &'static str,
    pub version: &'static str,
}

pub const ANALYZERS: &[Analyzer] = &[Analyzer {
    language: "en",
    stem_tokenizer: "porter unicode61 remove_diacritics 2",
    version: "en-porter-unicode61-1",
}];

pub fn analyzer(language: &str) -> Option<&'static Analyzer> {
    ANALYZERS.iter().find(|a| a.language == language)
}

fn analyzer_version() -> String {
    ANALYZERS
        .iter()
        .map(|a| a.version)
        .collect::<Vec<_>>()
        .join(",")
}

fn schema() -> String {
    let stem = analyzer(DEFAULT_LANGUAGE)
        .expect("default analyzer is registered")
        .stem_tokenizer;
    format!(
        "
CREATE TABLE docs (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('note', 'message')),
    version INTEGER NOT NULL,
    sha256 BLOB NOT NULL CHECK(length(sha256) = 32),
    title TEXT NOT NULL,
    visibility TEXT NOT NULL,
    protected INTEGER NOT NULL CHECK(protected IN (0, 1)),
    imported_from TEXT,
    language TEXT NOT NULL
);
CREATE INDEX docs_visibility ON docs(visibility);
CREATE INDEX docs_imported_from ON docs(imported_from) WHERE imported_from IS NOT NULL;
CREATE TABLE passages (
    id INTEGER PRIMARY KEY,
    doc TEXT NOT NULL REFERENCES docs(id),
    start_byte INTEGER NOT NULL,
    end_byte INTEGER NOT NULL,
    heading_path TEXT NOT NULL,
    text TEXT NOT NULL,
    indexed_text TEXT NOT NULL
);
CREATE INDEX passages_doc ON passages(doc);
CREATE VIRTUAL TABLE fts_exact USING fts5(
    indexed_text, content = 'passages', content_rowid = 'id',
    tokenize = 'unicode61 remove_diacritics 2'
);
CREATE VIRTUAL TABLE fts_stem USING fts5(
    indexed_text, content = 'passages', content_rowid = 'id',
    tokenize = '{stem}'
);
CREATE TRIGGER passages_exact_insert AFTER INSERT ON passages BEGIN
    INSERT INTO fts_exact(rowid, indexed_text) VALUES (new.id, new.indexed_text);
END;
CREATE TRIGGER passages_exact_delete AFTER DELETE ON passages BEGIN
    INSERT INTO fts_exact(fts_exact, rowid, indexed_text) VALUES ('delete', old.id, old.indexed_text);
END;
CREATE TRIGGER passages_stem_insert AFTER INSERT ON passages BEGIN
    INSERT INTO fts_stem(rowid, indexed_text) VALUES (new.id, new.indexed_text);
END;
CREATE TRIGGER passages_stem_delete AFTER DELETE ON passages BEGIN
    INSERT INTO fts_stem(fts_stem, rowid, indexed_text) VALUES ('delete', old.id, old.indexed_text);
END;
CREATE TRIGGER passages_no_update BEFORE UPDATE ON passages BEGIN
    SELECT RAISE(ABORT, 'passages are replaced, never updated');
END;
CREATE TABLE vectors (
    passage INTEGER NOT NULL REFERENCES passages(id),
    model TEXT NOT NULL,
    blob BLOB NOT NULL,
    PRIMARY KEY (passage, model)
);
CREATE TABLE links (
    id TEXT PRIMARY KEY,
    src TEXT NOT NULL,
    dst TEXT NOT NULL,
    relation TEXT NOT NULL
);
CREATE INDEX links_src ON links(src);
CREATE INDEX links_dst ON links(dst);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
"
    )
}

const OBJECTS: &[&str] = &[
    "docs",
    "docs_visibility",
    "docs_imported_from",
    "passages",
    "passages_doc",
    "fts_exact",
    "fts_stem",
    "passages_exact_insert",
    "passages_exact_delete",
    "passages_stem_insert",
    "passages_stem_delete",
    "passages_no_update",
    "vectors",
    "links",
    "links_src",
    "links_dst",
    "meta",
];

/// Settings whose change makes the stored index meaningless.
fn expected_meta() -> Vec<(&'static str, String)> {
    vec![
        ("chunker_version", CHUNKER_VERSION.into()),
        ("analyzer_version", analyzer_version()),
        ("default_language", DEFAULT_LANGUAGE.into()),
        ("embedding_model", "none".into()),
        ("embedding_dimension", "0".into()),
    ]
}

/// Where a document may appear, mirroring core search eligibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    General,
    Chat(String),
    Hidden,
}

impl Visibility {
    fn tag(&self) -> String {
        match self {
            Self::General => "general".into(),
            Self::Chat(id) => format!("chat:{id}"),
            Self::Hidden => "hidden".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DocInput {
    pub id: String,
    pub version: u64,
    pub title: String,
    pub markdown: String,
    pub visibility: Visibility,
    pub protected: bool,
    /// Import source record, so a source change can re-derive visibility.
    pub imported_from: Option<String>,
    pub language: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkInput {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation: String,
}

/// One synchronization step up to `watermark`, applied atomically.
#[derive(Debug, Clone, Default)]
pub struct Changes {
    /// The sync state this step was computed from. If another writer moved
    /// the index meanwhile, applying fails with [`Error::Stale`].
    pub base: Option<SyncState>,
    /// Replace the whole mirror, recording `workspace_identity`, before applying.
    pub full: bool,
    pub workspace_identity: Option<String>,
    pub upsert_docs: Vec<DocInput>,
    pub remove_docs: Vec<String>,
    pub upsert_links: Vec<LinkInput>,
    pub remove_links: Vec<String>,
    pub watermark: i64,
}

/// The canonical position this index mirrors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncState {
    pub workspace_identity: Option<String>,
    pub watermark: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    Existing,
    Created,
    /// Replaced because it was damaged, outdated or used other settings.
    Rebuilt,
}

/// One matching document with its best passage. Byte ranges point into the
/// note's saved Markdown.
#[derive(Debug, Clone, PartialEq)]
pub struct DocHit {
    pub doc: String,
    pub version: u64,
    pub passage: i64,
    pub start_byte: usize,
    pub end_byte: usize,
    pub heading_path: String,
    pub title: String,
    pub text: String,
    /// Matching passages of this document (the larger lane count).
    pub passages: usize,
    /// Fused reciprocal-rank score; comparable only within one result list.
    pub score: f64,
}

/// Facts used to label and balance results; see [`SearchIndex::facts`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DocFacts {
    pub protected: bool,
    /// Imported from a source record (original evidence).
    pub imported: bool,
    /// Links into this document from visible documents.
    pub inbound_links: usize,
    /// Of those, links using the evidence relation (for example `source`).
    pub evidence_links: usize,
}

pub struct SearchIndex {
    conn: Connection,
}

impl SearchIndex {
    /// Opens `derived/search.sqlite`, creating or rebuilding it in place under
    /// an advisory lock shared by every BRN process. A foreign file is refused.
    /// `derived` must be the core-validated workspace `derived/` directory.
    pub fn open(derived: &Path) -> Result<(Self, Opened)> {
        let path = derived.join(FILE_NAME);
        for candidate in [
            derived.to_path_buf(),
            derived.join(LOCK_NAME),
            path.clone(),
            sibling(&path, "-wal"),
            sibling(&path, "-shm"),
        ] {
            match fs::symlink_metadata(&candidate) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    return Err(Error::Invalid("derived search paths must not be symlinks"));
                }
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(derived.join(LOCK_NAME))?;
        lock.lock()?;
        let opened = open_locked(&path);
        drop(lock);
        let (conn, opened) = opened?;
        Ok((Self { conn }, opened))
    }

    /// `None` until a first synchronization has completed.
    pub fn sync_state(&self) -> Result<Option<SyncState>> {
        sync_state(&self.conn)
    }

    /// Document IDs whose visibility was derived from these import sources.
    pub fn docs_importing(&self, sources: &[String]) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM docs WHERE imported_from = ?1 ORDER BY id")?;
        let mut out = Vec::new();
        for source in sources {
            for id in stmt.query_map([source], |r| r.get::<_, String>(0))? {
                out.push(id?);
            }
        }
        Ok(out)
    }

    pub fn doc_count(&self) -> Result<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT count(*) FROM docs", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    /// Applies one synchronization step atomically. A document whose content
    /// or title changed is re-chunked whole; otherwise only its metadata moves.
    pub fn apply(&mut self, changes: &Changes) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if sync_state(&tx)? != changes.base {
            return Err(Error::Stale);
        }
        if changes.full {
            tx.execute_batch(
                "DELETE FROM vectors; DELETE FROM passages; DELETE FROM docs; DELETE FROM links;
                 DELETE FROM meta WHERE key = 'workspace_identity';",
            )?;
            if let Some(identity) = &changes.workspace_identity {
                tx.execute(
                    "INSERT INTO meta(key, value) VALUES ('workspace_identity', ?1)",
                    [identity],
                )?;
            }
        }
        for id in &changes.remove_docs {
            remove_doc(&tx, id)?;
        }
        for doc in &changes.upsert_docs {
            if analyzer(&doc.language).is_none() {
                return Err(Error::Invalid("document language has no analyzer"));
            }
            let sha: [u8; 32] = Sha256::digest(doc.markdown.as_bytes()).into();
            let existing: Option<(Vec<u8>, String)> = tx
                .query_row(
                    "SELECT sha256, title FROM docs WHERE id = ?1",
                    [&doc.id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let same_text = existing
                .as_ref()
                .is_some_and(|(old, title)| old.as_slice() == sha && *title == doc.title);
            if !same_text {
                remove_doc(&tx, &doc.id)?;
            }
            tx.execute(
                "INSERT INTO docs(id, kind, version, sha256, title, visibility, protected, imported_from, language)
                 VALUES (?1, 'note', ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET version = excluded.version,
                    visibility = excluded.visibility, protected = excluded.protected,
                    imported_from = excluded.imported_from, language = excluded.language",
                params![
                    doc.id,
                    i64::try_from(doc.version).map_err(|_| Error::Invalid("document version"))?,
                    sha.as_slice(),
                    doc.title,
                    doc.visibility.tag(),
                    doc.protected,
                    doc.imported_from,
                    doc.language,
                ],
            )?;
            if !same_text {
                insert_passages(&tx, doc)?;
            }
        }
        for id in &changes.remove_links {
            tx.execute("DELETE FROM links WHERE id = ?1", [id])?;
        }
        for link in &changes.upsert_links {
            tx.execute(
                "INSERT INTO links(id, src, dst, relation) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET src = excluded.src, dst = excluded.dst,
                    relation = excluded.relation",
                params![link.id, link.from, link.to, link.relation],
            )?;
        }
        tx.execute(
            "INSERT INTO meta(key, value) VALUES ('watermark', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [changes.watermark.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Exact, stemmed and all-words-stemmed keyword lanes over documents
    /// visible in `chat` (`None` = general only). Each lane ranks documents
    /// by their best passage and keeps the top `per_lane`; the lanes are
    /// fused by RRF.
    pub fn keyword(&self, query: &str, chat: Option<&str>, per_lane: usize) -> Result<Vec<DocHit>> {
        check_query(query, per_lane)?;
        let fts = fts_query(query).ok_or(Error::Invalid("query has no search terms"))?;
        let scope = scope(chat);
        // Passages holding every query word (stemmed) form an extra lane, so a
        // note matching the whole query through inflections is not outranked
        // by notes repeating one exact word. It is a subset of the stem lane.
        // The first lane matching a note supplies its passage.
        let mut lanes = Vec::new();
        if fts.contains(" OR ") {
            lanes.push(("fts_stem", fts.replace(" OR ", " AND ")));
        }
        lanes.extend([("fts_exact", fts.clone()), ("fts_stem", fts.clone())]);
        let mut fused: BTreeMap<String, DocHit> = BTreeMap::new();
        for (table, fts) in &lanes {
            for (rank, hit) in self
                .lane(table, fts, &scope, per_lane)?
                .into_iter()
                .enumerate()
            {
                let contribution = 1.0 / (RRF_K + rank as f64 + 1.0);
                let entry = fused.entry(hit.doc.clone()).or_insert_with(|| hit.clone());
                entry.passages = entry.passages.max(hit.passages);
                entry.score += contribution;
            }
        }
        let mut hits: Vec<DocHit> = fused.into_values().collect();
        // Ties are broken by content, not by record or passage IDs, so equal
        // workspaces rank equally.
        hits.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.start_byte.cmp(&b.start_byte))
                .then_with(|| a.doc.cmp(&b.doc))
        });
        // Lanes skip passage text so wide windows stay cheap; fetch it once here.
        let mut text = self
            .conn
            .prepare("SELECT text FROM passages WHERE id = ?1")?;
        for hit in &mut hits {
            hit.text = text.query_row([hit.passage], |r| r.get(0))?;
        }
        Ok(hits)
    }

    /// The exact number of documents visible in `chat` (`None` = general)
    /// that match `query` in either lane.
    pub fn count(&self, query: &str, chat: Option<&str>) -> Result<usize> {
        check_query(query, 1)?;
        let fts = fts_query(query).ok_or(Error::Invalid("query has no search terms"))?;
        let lane = |table: &str| {
            format!(
                "SELECT p.doc FROM {table} JOIN passages p ON p.id = {table}.rowid
                 JOIN docs d ON d.id = p.doc
                 WHERE {table} MATCH ?1 AND d.visibility IN ('general', ?2)"
            )
        };
        let n: i64 = self.conn.query_row(
            &format!(
                "SELECT count(*) FROM ({} UNION {})",
                lane("fts_exact"),
                lane("fts_stem")
            ),
            params![fts, scope(chat)],
            |r| r.get(0),
        )?;
        Ok(n as usize)
    }

    /// Ranking-independent facts about indexed documents, as seen from
    /// `chat`. Link counts include only links from documents visible there,
    /// so they reveal nothing about notes the context cannot see.
    pub fn facts(
        &self,
        ids: &[String],
        chat: Option<&str>,
        evidence_relation: &str,
    ) -> Result<BTreeMap<String, DocFacts>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.protected, d.imported_from IS NOT NULL,
                (SELECT count(*) FROM links l JOIN docs s ON s.id = l.src
                 WHERE l.dst = d.id AND s.visibility IN ('general', ?2)),
                (SELECT count(*) FROM links l JOIN docs s ON s.id = l.src
                 WHERE l.dst = d.id AND l.relation = ?3 AND s.visibility IN ('general', ?2))
             FROM docs d WHERE d.id = ?1",
        )?;
        let scope = scope(chat);
        let mut out = BTreeMap::new();
        for id in ids {
            if let Some(facts) = stmt
                .query_row(params![id, scope, evidence_relation], |r| {
                    Ok(DocFacts {
                        protected: r.get(0)?,
                        imported: r.get(1)?,
                        inbound_links: r.get::<_, i64>(2)? as usize,
                        evidence_links: r.get::<_, i64>(3)? as usize,
                    })
                })
                .optional()?
            {
                out.insert(id.clone(), facts);
            }
        }
        Ok(out)
    }

    /// The best-ranked passages, collapsed to each document's best one.
    /// Fetches a window of passages and widens it only while fewer than
    /// `per_lane` documents appeared and more passages exist, so a document
    /// with many matching passages cannot crowd others out. `passages`
    /// counts a document's matches within the fetched window.
    fn lane(&self, table: &str, fts: &str, scope: &str, per_lane: usize) -> Result<Vec<DocHit>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT p.id, p.doc, d.version, p.start_byte, p.end_byte, p.heading_path, d.title
             FROM {table} JOIN passages p ON p.id = {table}.rowid JOIN docs d ON d.id = p.doc
             WHERE {table} MATCH ?1 AND d.visibility IN ('general', ?2)
             ORDER BY bm25({table}), d.title, p.start_byte, p.id LIMIT ?3"
        ))?;
        let mut window = per_lane * 8;
        loop {
            let mut docs: Vec<DocHit> = Vec::new();
            let mut position: BTreeMap<String, usize> = BTreeMap::new();
            let mut fetched = 0;
            let rows = stmt.query_map(params![fts, scope, window as i64], |r| {
                Ok(DocHit {
                    passage: r.get(0)?,
                    doc: r.get(1)?,
                    version: r.get::<_, i64>(2)? as u64,
                    start_byte: r.get::<_, i64>(3)? as usize,
                    end_byte: r.get::<_, i64>(4)? as usize,
                    heading_path: r.get(5)?,
                    title: r.get(6)?,
                    text: String::new(),
                    passages: 1,
                    score: 0.0,
                })
            })?;
            for row in rows {
                let hit = row?;
                fetched += 1;
                match position.get(&hit.doc) {
                    Some(&i) => docs[i].passages += 1,
                    None => {
                        position.insert(hit.doc.clone(), docs.len());
                        docs.push(hit);
                    }
                }
            }
            if docs.len() >= per_lane || fetched < window {
                docs.truncate(per_lane);
                return Ok(docs);
            }
            window *= 4;
        }
    }

    /// Recorded links touching `id`, as `(link id, from, to, relation)`.
    pub fn links_of(&self, id: &str) -> Result<Vec<LinkInput>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, src, dst, relation FROM links WHERE src = ?1 OR dst = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([id], |r| {
            Ok(LinkInput {
                id: r.get(0)?,
                from: r.get(1)?,
                to: r.get(2)?,
                relation: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

fn scope(chat: Option<&str>) -> String {
    chat.map_or_else(|| "general".to_string(), |id| format!("chat:{id}"))
}

fn sync_state(conn: &Connection) -> Result<Option<SyncState>> {
    let read = |key: &str| -> Result<Option<String>> {
        Ok(conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    };
    let Some(watermark) = read("watermark")? else {
        return Ok(None);
    };
    Ok(Some(SyncState {
        workspace_identity: read("workspace_identity")?,
        watermark: watermark
            .parse()
            .map_err(|_| Error::Corrupt("search index watermark"))?,
    }))
}

fn remove_doc(tx: &rusqlite::Transaction<'_>, id: &str) -> Result<()> {
    tx.execute(
        "DELETE FROM vectors WHERE passage IN (SELECT id FROM passages WHERE doc = ?1)",
        [id],
    )?;
    tx.execute("DELETE FROM passages WHERE doc = ?1", [id])?;
    tx.execute("DELETE FROM docs WHERE id = ?1", [id])?;
    Ok(())
}

fn insert_passages(tx: &rusqlite::Transaction<'_>, doc: &DocInput) -> Result<()> {
    let mut stmt = tx.prepare(
        "INSERT INTO passages(doc, start_byte, end_byte, heading_path, text, indexed_text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for chunk in chunks(&doc.markdown) {
        let heading_path = chunk.heading_path.join(" › ");
        let text = &doc.markdown[chunk.start..chunk.end];
        let label = if heading_path.is_empty() {
            doc.title.clone()
        } else {
            format!("{} › {heading_path}", doc.title)
        };
        stmt.execute(params![
            doc.id,
            chunk.start as i64,
            chunk.end as i64,
            heading_path,
            text,
            format!("{label}\n\n{text}"),
        ])?;
    }
    if doc.markdown.is_empty() {
        stmt.execute(params![doc.id, 0, 0, "", "", doc.title])?;
    }
    Ok(())
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get::<_, String>(0))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

enum Found {
    Missing,
    Usable,
    Rebuild,
    Replace,
}

fn open_locked(path: &Path) -> Result<(Connection, Opened)> {
    let found = inspect(path)?;
    if matches!(found, Found::Replace) {
        for suffix in ["", "-wal", "-shm"] {
            match fs::remove_file(sibling(path, suffix)) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
    }
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    if matches!(found, Found::Usable) {
        return Ok((conn, Opened::Existing));
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    // Rebuilding in place keeps one file identity for concurrent readers.
    let objects: Vec<(String, String)> = {
        let mut stmt = tx.prepare(
            "SELECT type, name FROM sqlite_schema
             WHERE name NOT LIKE 'sqlite_%' AND type IN ('table', 'trigger', 'view')
             AND (type != 'table' OR sql NOT LIKE 'CREATE TABLE ''%')",
        )?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?
    };
    // Triggers first, then tables newest first, so no drop breaks a reference.
    let ordered = objects
        .iter()
        .filter(|(k, _)| k == "trigger")
        .chain(objects.iter().rev().filter(|(k, _)| k != "trigger"));
    for (kind, name) in ordered {
        tx.execute_batch(&format!(
            "DROP {} IF EXISTS \"{}\";",
            kind.to_uppercase(),
            name.replace('"', "\"\"")
        ))?;
    }
    tx.execute_batch(&schema())?;
    for (key, value) in expected_meta() {
        tx.execute(
            "INSERT INTO meta(key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
    }
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    // Persist the brand in the main file before releasing the lock, so another
    // process never inspects an unbranded header and refuses it as foreign.
    let (busy, frames, copied): (i64, i64, i64) =
        conn.query_row("PRAGMA wal_checkpoint(FULL)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
    if busy != 0 || frames != copied {
        return Err(Error::Corrupt("search index header checkpoint"));
    }
    let opened = match found {
        Found::Missing => Opened::Created,
        _ => Opened::Rebuilt,
    };
    Ok((conn, opened))
}

fn inspect(path: &Path) -> Result<Found> {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Found::Missing),
        Err(e) => return Err(e.into()),
    };
    if file.metadata()?.len() == 0 {
        return Ok(Found::Replace);
    }
    let mut header = [0; 100];
    match file.read_exact(&mut header) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => return Err(Error::Invalid(NOT_OURS)),
        Err(e) => return Err(e.into()),
    }
    if &header[..16] != b"SQLite format 3\0" {
        return Err(Error::Invalid(NOT_OURS));
    }
    let application = i64::from(i32::from_be_bytes([
        header[68], header[69], header[70], header[71],
    ]));
    if application != APPLICATION_ID && application != 0 {
        return Err(Error::Invalid(NOT_OURS));
    }
    let checked = (|| -> Result<Found> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        if application == 0 {
            // Only an interrupted creation of ours leaves an empty, unbranded database.
            let objects: i64 =
                conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
            return if objects == 0 {
                Ok(Found::Replace)
            } else {
                Err(Error::Invalid(NOT_OURS))
            };
        }
        let healthy: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if healthy != "ok" {
            return Ok(Found::Replace);
        }
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        let names: Vec<String> = {
            let mut stmt = conn.prepare("SELECT name FROM sqlite_schema")?;
            stmt.query_map([], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?
        };
        if version != SCHEMA_VERSION || OBJECTS.iter().any(|o| !names.iter().any(|n| n == o)) {
            return Ok(Found::Rebuild);
        }
        for (key, value) in expected_meta() {
            let stored: Option<String> = conn
                .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
                .optional()?;
            if stored.as_deref() != Some(value.as_str()) {
                return Ok(Found::Rebuild);
            }
        }
        Ok(Found::Usable)
    })();
    match checked {
        Err(Error::Sql(error))
            if application == APPLICATION_ID
                && matches!(
                    error.sqlite_error_code(),
                    Some(rusqlite::ErrorCode::NotADatabase | rusqlite::ErrorCode::DatabaseCorrupt)
                ) =>
        {
            Ok(Found::Replace)
        }
        other => other,
    }
}
