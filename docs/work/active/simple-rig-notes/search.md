# Step 3: Search

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Search the vault's notes by keyword (SQLite FTS5/BM25), by meaning (local embeddings) and both (reciprocal-rank fusion), from a disposable `index.sqlite` kept in step with the vault; remove LanceDB.

**Architecture:** `brn_retrieval::note_index::NoteIndex` owns `index.sqlite` (notes, passages, FTS5, embedding blobs) and its queries. `brn_retrieval::native::LocalEmbedder` (feature `native`) runs the local MiniLM model. `brn_workflow::library::Library` joins the vault module from step 2 with the index: refresh, background embedding, search with a visible keyword-only fallback. The old `Index` keeps keyword search only until step 6 deletes it. CLI commands move to step 4, together with the switch to the new data folder.

**Tech Stack:** Rust `1.98.1`, rusqlite `=0.40.2` (bundled, FTS5), fastembed `=7.1.0` (feature `native`), sha2, tempfile.

**Spec:** [Simple Rig-based notes app](../../../superpowers/specs/2026-10-02-simple-rig-notes-design.md), sections 3 and 8; [roadmap](plan.md); [step 2 plan](store.md).

## Global Constraints

- Inherit the [roadmap](plan.md) constraints.
- `index.sqlite` is disposable: deleted and rebuilt from the vault if corrupt, missing or from another version. It never holds user work.
- Passages: the existing chunker (at most 1600 bytes, preferring whitespace breaks); each keeps note path, note SHA-256, byte range and exact text.
- Keyword: FTS5 with `unicode61 remove_diacritics 2` and BM25; query words are quoted and joined with OR; user input never becomes SQL or FTS syntax.
- Semantic: local all-MiniLM-L6-v2, mean pooling, 384 dimensions, through fastembed `=7.1.0`, loaded from a local folder; never downloads. Normalised vectors stored as little-endian `f32` blobs; exhaustive dot product in Rust. A changed model identity deletes all embeddings.
- Hybrid: top 50 from each, reciprocal-rank fusion `k = 60`, ties broken by passage ID.
- Without a model, every search runs keyword-only and says so (`keyword_only`).
- Refresh skips files whose size and mtime are unchanged, compares SHA-256 for the rest, re-chunks only changed notes, drops removed ones, and lists unreadable `.md` files (non-UTF-8 content or name, larger than 1 MiB).
- Queries: non-empty, at most 512 bytes; limit 1–50.
- Known pre-existing failures, not caused by this step: `crates/brn-workflow/tests/note_evidence.rs` `managed_missing_import_fails_as_stale_instead_of_uncategorized_io` fails on `main@d739364` (old code, removed in step 6); `crates/brn/tests/cli_ask.rs` `deadline_before_submission_timeout_context` can fail under full-workspace load and passes alone.

## File map

| Path | Responsibility |
| --- | --- |
| `crates/brn-retrieval/src/chunk.rs` | Passage boundaries, shared by the old `Index` and `NoteIndex` |
| `crates/brn-retrieval/src/note_index/mod.rs` | `NoteIndex`, `IndexedNote`: notes and passages |
| `crates/brn-retrieval/src/note_index/schema.rs` | Create, check and rebuild `index.sqlite` |
| `crates/brn-retrieval/src/note_index/search.rs` | `NoteHit`, query checks, FTS5 keyword search, fusion |
| `crates/brn-retrieval/src/note_index/embeddings.rs` | `Embedder` trait, embedding storage, semantic search |
| `crates/brn-retrieval/src/native.rs` | `LocalEmbedder` (replaces the LanceDB adapter) |
| `crates/brn-workflow/src/library.rs` | `Library`: refresh, background embedding, search |
| `crates/brn-retrieval/tests/{note_index,note_search,note_embeddings,local_embedder}.rs`, `crates/brn-workflow/tests/library.rs` | Tests |

All code below was compiled and its tests run in a scratch worktree of `main@d739364` while planning (27 retrieval tests, 2 local-embedder tests with the model test skipped, 6 library tests; Clippy clean with and without `native`).

---

### Task 1: Passage chunker and `index.sqlite`

**Files:**
- Create: `crates/brn-retrieval/src/chunk.rs`, `crates/brn-retrieval/src/note_index/mod.rs`, `crates/brn-retrieval/src/note_index/schema.rs`, `crates/brn-retrieval/tests/note_index.rs`
- Modify: `crates/brn-retrieval/Cargo.toml` (add rusqlite), `crates/brn-retrieval/src/lib.rs` (patch below), `Cargo.lock`

**Interfaces:**
- Produces:

```rust
pub struct IndexedNote { pub path: String, pub title: String, pub size: u64, pub modified_ns: i64, pub sha256: [u8; 32] }
impl NoteIndex {
    pub fn open(path: &Path) -> brn_retrieval::Result<(NoteIndex, bool)>; // bool: (re)created
    pub fn notes(&self) -> Result<Vec<IndexedNote>>;                     // ordered by path
    pub fn note(&self, path: &str) -> Result<Option<IndexedNote>>;
    pub fn upsert_note(&mut self, note: &IndexedNote, text: &str) -> Result<()>;
    pub fn update_metadata(&mut self, path: &str, size: u64, modified_ns: i64) -> Result<()>;
    pub fn remove_note(&mut self, path: &str) -> Result<()>;
}
// brn_retrieval::Error gains: Sql(rusqlite::Error)
```

- [ ] **Step 1: Write the failing tests** in `crates/brn-retrieval/tests/note_index.rs`:

```rust
use brn_retrieval::note_index::{IndexedNote, NoteIndex};
use sha2::{Digest, Sha256};

fn note(path: &str, text: &str) -> IndexedNote {
    IndexedNote {
        path: path.into(),
        title: path.into(),
        size: text.len() as u64,
        modified_ns: 1,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}

fn passages(index_path: &std::path::Path, path: &str) -> Vec<(i64, i64, String)> {
    let conn = rusqlite::Connection::open(index_path).unwrap();
    let mut statement = conn
        .prepare(
            "SELECT start_byte, end_byte, text FROM passages WHERE path = ?1 ORDER BY start_byte",
        )
        .unwrap();
    statement
        .query_map([path], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn new_index_is_created_then_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let (mut index, created) = NoteIndex::open(&path).unwrap();
    assert!(created);
    index.upsert_note(&note("a.md", "alpha"), "alpha").unwrap();
    drop(index);
    let (index, created) = NoteIndex::open(&path).unwrap();
    assert!(!created);
    assert_eq!(index.notes().unwrap(), vec![note("a.md", "alpha")]);
}

#[test]
fn damaged_or_foreign_index_is_rebuilt_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    std::fs::write(&path, vec![0x42u8; 8192]).unwrap();
    let (index, created) = NoteIndex::open(&path).unwrap();
    assert!(created);
    assert!(index.notes().unwrap().is_empty());
    drop(index);

    let other = dir.path().join("other.sqlite");
    let raw = rusqlite::Connection::open(&other).unwrap();
    raw.execute_batch("CREATE TABLE x (y INTEGER); PRAGMA user_version = 99;")
        .unwrap();
    drop(raw);
    let (index, created) = NoteIndex::open(&other).unwrap();
    assert!(created);
    assert!(index.notes().unwrap().is_empty());
}

#[test]
fn upsert_splits_into_passages_and_replaces_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let (mut index, _) = NoteIndex::open(&path).unwrap();
    let long = "word ".repeat(500);
    index.upsert_note(&note("a.md", &long), &long).unwrap();
    let parts = passages(&path, "a.md");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].0, 0);
    assert_eq!(parts[0].1, parts[1].0);
    assert_eq!(parts[1].1 as usize, long.len());
    let joined: String = parts.iter().map(|p| p.2.as_str()).collect();
    assert_eq!(joined, long);

    index
        .upsert_note(&note("a.md", "short é\r\n"), "short é\r\n")
        .unwrap();
    assert_eq!(
        passages(&path, "a.md"),
        vec![(0, 10, "short é\r\n".to_string())]
    );
    assert_eq!(index.notes().unwrap().len(), 1);
}

#[test]
fn upsert_rejects_text_that_does_not_match_size_or_hash() {
    let dir = tempfile::tempdir().unwrap();
    let (mut index, _) = NoteIndex::open(&dir.path().join("index.sqlite")).unwrap();
    assert!(matches!(
        index.upsert_note(&note("a.md", "alpha"), "alphx"),
        Err(brn_retrieval::Error::Invalid(_))
    ));
    assert!(index.notes().unwrap().is_empty());
}

#[test]
fn metadata_update_and_removal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let (mut index, _) = NoteIndex::open(&path).unwrap();
    index.upsert_note(&note("a.md", "alpha"), "alpha").unwrap();
    index.update_metadata("a.md", 5, 99).unwrap();
    assert_eq!(index.note("a.md").unwrap().unwrap().modified_ns, 99);
    index.remove_note("a.md").unwrap();
    assert_eq!(index.note("a.md").unwrap(), None);
    assert!(passages(&path, "a.md").is_empty());
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p brn-retrieval --test note_index`
Expected: compile error, unresolved import `brn_retrieval::note_index`.

- [ ] **Step 3: Add rusqlite** to `crates/brn-retrieval/Cargo.toml` under `[dependencies]`, after `thiserror = "2"`:

```toml
rusqlite = { version = "=0.40.2", default-features = false, features = ["bundled"] }
```

- [ ] **Step 4: Create `crates/brn-retrieval/src/chunk.rs`**

```rust
//! Splits note text into passages of at most 1600 bytes, preferring to break after whitespace.
const MAX_PASSAGE: usize = 1600;

/// Half-open byte ranges that cover `text` in order, each on UTF-8 boundaries.
pub(crate) fn passages(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + MAX_PASSAGE).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if end < text.len() {
            let slice = &text[start..end];
            if let Some((offset, c)) = slice
                .char_indices()
                .rev()
                .find(|(i, c)| *i >= MAX_PASSAGE / 2 && c.is_whitespace())
            {
                end = start + offset + c.len_utf8();
            }
        }
        ranges.push((start, end));
        start = end;
    }
    ranges
}
```

- [ ] **Step 5: Create `crates/brn-retrieval/src/note_index/schema.rs`**

```rust
use crate::Result;
use rusqlite::Connection;
use std::{
    ffi::OsString,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::Duration,
};

const APPLICATION_ID: i64 = 0x4252_4e49; // BRNI
const VERSION: i64 = 1;
const SCHEMA: &str = "
CREATE TABLE notes (
    path TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    size INTEGER NOT NULL,
    modified_ns INTEGER NOT NULL,
    sha256 BLOB NOT NULL CHECK(length(sha256) = 32)
);
CREATE TABLE passages (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL REFERENCES notes(path) ON DELETE CASCADE,
    start_byte INTEGER NOT NULL,
    end_byte INTEGER NOT NULL,
    text TEXT NOT NULL
);
CREATE INDEX passages_path ON passages(path);
CREATE VIRTUAL TABLE passages_fts USING fts5(
    text, content = 'passages', content_rowid = 'id',
    tokenize = 'unicode61 remove_diacritics 2'
);
CREATE TRIGGER passages_insert AFTER INSERT ON passages BEGIN
    INSERT INTO passages_fts(rowid, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER passages_delete AFTER DELETE ON passages BEGIN
    INSERT INTO passages_fts(passages_fts, rowid, text) VALUES ('delete', old.id, old.text);
END;
CREATE TABLE embeddings (
    passage_id INTEGER PRIMARY KEY REFERENCES passages(id) ON DELETE CASCADE,
    vector BLOB NOT NULL
);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
";

pub(super) fn open(path: &Path) -> Result<(Connection, bool)> {
    if path.exists()
        && let Some(conn) = existing(path)
    {
        return Ok((conn, false));
    }
    remove(path)?;
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    let tx = conn.transaction()?;
    tx.execute_batch(SCHEMA)?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", VERSION)?;
    tx.commit()?;
    Ok((conn, true))
}

/// The existing index if it is healthy and current; `None` means rebuild it.
fn existing(path: &Path) -> Option<Connection> {
    let conn = Connection::open(path).ok()?;
    let healthy: String = conn
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .ok()?;
    let application: i64 = conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .ok()?;
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .ok()?;
    if healthy != "ok" || application != APPLICATION_ID || version != VERSION {
        return None;
    }
    configure(&conn).ok()?;
    Some(conn)
}

fn remove(path: &Path) -> Result<()> {
    for suffix in ["", "-wal", "-shm"] {
        let mut name: OsString = path.as_os_str().to_owned();
        name.push(suffix);
        match std::fs::remove_file(PathBuf::from(name)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get::<_, String>(0))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}
```

- [ ] **Step 6: Create `crates/brn-retrieval/src/note_index/mod.rs`**

```rust
//! Disposable search index over the vault's notes (`index.sqlite`): note
//! metadata, passages, FTS5 keyword search and local embeddings. It can be
//! deleted at any time and rebuilt from the vault.
mod schema;

use crate::{Error, Result, chunk};
use rusqlite::{Connection, OptionalExtension, Row, params};
use sha2::{Digest, Sha256};
use std::path::Path;

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
```

- [ ] **Step 7: Update `crates/brn-retrieval/src/lib.rs`.** Save this patch to a git-ignored file (for example `.superpowers/task1.patch`) and run `git apply .superpowers/task1.patch`. It adds the `Sql` error, makes the old chunker use `chunk::passages` (same boundaries and passage IDs) and declares the new modules.

```diff
--- a/crates/brn-retrieval/src/lib.rs
+++ b/crates/brn-retrieval/src/lib.rs
@@ -28,6 +28,8 @@
     Format(#[from] serde_json::Error),
     #[error("native retrieval: {0}")]
     Native(String),
+    #[error("index database: {0}")]
+    Sql(#[from] rusqlite::Error),
 }
 pub type Result<T> = std::result::Result<T, Error>;
 
@@ -80,7 +82,6 @@
     db_files: BTreeMap<String, String>,
 }
 const CHUNKER: &str = "utf8-1600-v1";
-const MAX_CHUNK: usize = 1600;
 const MAX_DOCUMENT: usize = 1_048_576;
 const FORMAT: u32 = 1;
 
@@ -113,27 +114,7 @@
     let mut chunks = Vec::new();
     for (doc_index, doc) in docs.iter().enumerate() {
         check_cancel(cancel)?;
-        let mut start = 0;
-        while start < doc.text.len() {
-            check_cancel(cancel)?;
-            let hard = (start + MAX_CHUNK).min(doc.text.len());
-            let mut end = hard;
-            while !doc.text.is_char_boundary(end) {
-                end -= 1;
-            }
-            if end == start {
-                return Err(Error::Invalid("invalid UTF-8 chunk boundary"));
-            }
-            if end < doc.text.len() {
-                let slice = &doc.text[start..end];
-                if let Some((offset, _)) = slice
-                    .char_indices()
-                    .rev()
-                    .find(|(i, c)| *i >= MAX_CHUNK / 2 && c.is_whitespace())
-                {
-                    end = start + offset + slice[offset..].chars().next().unwrap().len_utf8();
-                }
-            }
+        for (start, end) in chunk::passages(&doc.text) {
             chunks.push(Chunk {
                 doc: doc_index,
                 start,
@@ -143,7 +124,6 @@
                     doc.source_id, doc.version_id
                 ),
             });
-            start = end;
         }
     }
     Ok(chunks)
@@ -516,3 +496,6 @@
 }
 #[cfg(feature = "native")]
 mod native;
+
+mod chunk;
+pub mod note_index;
```

- [ ] **Step 8: Run the tests**

Run: `cargo test -p brn-retrieval --test note_index` (without `--locked` this first time: the new dependency updates `Cargo.lock`)
Expected: 5 passed. Then `cargo test -p brn-retrieval --locked`: all pass, including the old `keyword` tests (unchanged passages).

- [ ] **Step 9: Clippy and commit**

Run: `cargo fmt --all && cargo clippy -p brn-retrieval --all-targets --locked -- -D warnings`

```bash
git add Cargo.lock crates/brn-retrieval/Cargo.toml crates/brn-retrieval/src/lib.rs crates/brn-retrieval/src/chunk.rs crates/brn-retrieval/src/note_index crates/brn-retrieval/tests/note_index.rs
git commit -m "feat(retrieval): add disposable index.sqlite for vault notes

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 2: Keyword search and fusion

**Files:**
- Create: `crates/brn-retrieval/src/note_index/search.rs`, `crates/brn-retrieval/tests/note_search.rs`
- Modify: `crates/brn-retrieval/src/note_index/mod.rs`

**Interfaces:**
- Consumes: Task 1 `NoteIndex` (its private `conn` field is visible to child modules), `IndexedNote`.
- Produces:

```rust
pub struct NoteHit { pub passage_id: i64, pub path: String, pub note_sha256: [u8; 32], pub start_byte: usize, pub end_byte: usize, pub quote: String, pub score: f32 }
pub fn check_query(query: &str, limit: usize) -> brn_retrieval::Result<()>;
pub fn fuse_hits(lists: &[&[NoteHit]], limit: usize) -> Vec<NoteHit>;
impl NoteIndex { pub fn keyword(&self, query: &str, limit: usize) -> Result<Vec<NoteHit>>; }
// pub(super), used by Task 3: HIT_COLUMNS, HitRow, hit_row(), to_hit()
```

- [ ] **Step 1: Write the failing tests** in `crates/brn-retrieval/tests/note_search.rs`:

```rust
use brn_retrieval::note_index::{IndexedNote, NoteHit, NoteIndex, fuse_hits};
use sha2::{Digest, Sha256};

fn note(path: &str, text: &str) -> IndexedNote {
    IndexedNote {
        path: path.into(),
        title: path.into(),
        size: text.len() as u64,
        modified_ns: 1,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}

fn index_with(notes: &[(&str, &str)]) -> (tempfile::TempDir, NoteIndex) {
    let dir = tempfile::tempdir().unwrap();
    let (mut index, _) = NoteIndex::open(&dir.path().join("index.sqlite")).unwrap();
    for (path, text) in notes {
        index.upsert_note(&note(path, text), text).unwrap();
    }
    (dir, index)
}

fn paths(hits: &[NoteHit]) -> Vec<&str> {
    hits.iter().map(|h| h.path.as_str()).collect()
}

#[test]
fn matches_whole_words_not_substrings() {
    let (_dir, index) = index_with(&[
        ("token.md", "red blue"),
        ("substring.md", "reddish blueprint"),
    ]);
    assert_eq!(paths(&index.keyword("red", 10).unwrap()), vec!["token.md"]);
}

#[test]
fn ignores_case_and_diacritics() {
    let (_dir, index) = index_with(&[("cafe.md", "Café résumé")]);
    assert_eq!(
        paths(&index.keyword("CAFE resume", 10).unwrap()),
        vec!["cafe.md"]
    );
}

#[test]
fn passages_matching_more_words_rank_first() {
    let (_dir, index) = index_with(&[("one.md", "alpha"), ("both.md", "alpha beta")]);
    assert_eq!(
        paths(&index.keyword("alpha beta", 10).unwrap()),
        vec!["both.md", "one.md"]
    );
}

#[test]
fn query_syntax_is_treated_as_words() {
    let (_dir, index) = index_with(&[("a.md", "alpha"), ("b.md", "beta")]);
    let hits = index.keyword("alpha\" OR NOT (beta* NEAR", 10).unwrap();
    assert_eq!(paths(&hits), vec!["a.md", "b.md"]);
}

#[test]
fn hits_carry_exact_provenance() {
    let text = "intro\n\nThe launch is in October.";
    let (_dir, index) = index_with(&[("plan.md", text)]);
    let hit = &index.keyword("october", 10).unwrap()[0];
    assert_eq!(&text[hit.start_byte..hit.end_byte], hit.quote);
    assert_eq!(
        hit.note_sha256,
        <[u8; 32]>::from(Sha256::digest(text.as_bytes()))
    );
    assert!(hit.score > 0.0);
}

#[test]
fn reindexed_and_removed_text_is_not_found() {
    let (_dir, mut index) = index_with(&[("a.md", "old words")]);
    index
        .upsert_note(&note("a.md", "new words"), "new words")
        .unwrap();
    assert!(index.keyword("old", 10).unwrap().is_empty());
    assert_eq!(paths(&index.keyword("new", 10).unwrap()), vec!["a.md"]);
    index.remove_note("a.md").unwrap();
    assert!(index.keyword("new", 10).unwrap().is_empty());
}

#[test]
fn rejects_bad_queries_and_limits() {
    let (_dir, index) = index_with(&[("a.md", "alpha")]);
    for (query, limit) in [("", 10), ("   ", 10), ("alpha", 0), ("alpha", 51)] {
        assert!(
            matches!(
                index.keyword(query, limit),
                Err(brn_retrieval::Error::Invalid(_))
            ),
            "{query:?} {limit}"
        );
    }
    assert!(matches!(
        index.keyword(&"a".repeat(513), 10),
        Err(brn_retrieval::Error::Invalid(_))
    ));
    assert!(matches!(
        index.keyword("!!! ???", 10),
        Err(brn_retrieval::Error::Invalid(_))
    ));
    assert_eq!(index.keyword("alpha", 50).unwrap().len(), 1);
}

#[test]
fn fusion_rewards_agreement_and_breaks_ties_by_passage() {
    let hit = |id: i64| NoteHit {
        passage_id: id,
        path: format!("{id}.md"),
        note_sha256: [0; 32],
        start_byte: 0,
        end_byte: 1,
        quote: "x".into(),
        score: 0.0,
    };
    let keyword = [hit(1), hit(2), hit(3)];
    let semantic = [hit(3), hit(4)];
    let fused = fuse_hits(&[&keyword, &semantic], 10);
    let ids: Vec<i64> = fused.iter().map(|h| h.passage_id).collect();
    assert_eq!(ids, vec![3, 1, 2, 4]);
    assert!((fused[0].score - (1.0 / 63.0 + 1.0 / 61.0)).abs() < 1e-6);
    assert_eq!(fuse_hits(&[&keyword, &semantic], 2).len(), 2);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p brn-retrieval --locked --test note_search`
Expected: compile error, `NoteHit` / `fuse_hits` not found.

- [ ] **Step 3: Create `crates/brn-retrieval/src/note_index/search.rs`**

```rust
use super::NoteIndex;
use crate::{Error, Result};
use rusqlite::{Row, params};

/// One matching passage, with its note's path and content hash at indexing time.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteHit {
    pub passage_id: i64,
    pub path: String,
    pub note_sha256: [u8; 32],
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
    /// Higher is better; comparable only within one result list.
    pub score: f32,
}

pub(super) const HIT_COLUMNS: &str = "p.id, p.path, n.sha256, p.start_byte, p.end_byte, p.text";
pub(super) type HitRow = (i64, String, Vec<u8>, i64, i64, String);

pub(super) fn hit_row(row: &Row<'_>) -> rusqlite::Result<HitRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

pub(super) fn to_hit(
    (passage_id, path, sha, start, end, quote): HitRow,
    score: f32,
) -> Result<NoteHit> {
    let note_sha256 = sha
        .try_into()
        .map_err(|_| Error::Corrupt("index note hash"))?;
    Ok(NoteHit {
        passage_id,
        path,
        note_sha256,
        start_byte: start.max(0) as usize,
        end_byte: end.max(0) as usize,
        quote,
        score,
    })
}

/// Rejects empty or over-long (> 512 bytes) queries and limits outside 1..=50.
pub fn check_query(query: &str, limit: usize) -> Result<()> {
    if query.trim().is_empty() || query.len() > 512 || !(1..=50).contains(&limit) {
        return Err(Error::Invalid("query length or limit"));
    }
    Ok(())
}

/// An FTS5 query matching any of the query's words. Each word is quoted, so
/// user text never becomes FTS5 syntax.
fn fts_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.to_lowercase()))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" OR "))
}

impl NoteIndex {
    /// Passages containing any query word, best BM25 score first.
    pub fn keyword(&self, query: &str, limit: usize) -> Result<Vec<NoteHit>> {
        check_query(query, limit)?;
        let fts = fts_query(query).ok_or(Error::Invalid("query has no search terms"))?;
        let mut statement = self.conn.prepare(&format!(
            "SELECT {HIT_COLUMNS}, bm25(passages_fts) AS rank FROM passages_fts \
             JOIN passages p ON p.id = passages_fts.rowid \
             JOIN notes n ON n.path = p.path \
             WHERE passages_fts MATCH ?1 ORDER BY rank, p.id LIMIT ?2"
        ))?;
        let rows = statement.query_map(params![fts, limit as i64], |row| {
            Ok((hit_row(row)?, row.get::<_, f64>(6)?))
        })?;
        rows.map(|row| {
            let (hit, rank) = row?;
            to_hit(hit, -rank as f32)
        })
        .collect()
    }
}

/// Reciprocal-rank fusion (k = 60) of ranked lists; ties are broken by passage id.
pub fn fuse_hits(lists: &[&[NoteHit]], limit: usize) -> Vec<NoteHit> {
    let mut merged: Vec<NoteHit> = Vec::new();
    for list in lists {
        for (rank, hit) in list.iter().enumerate() {
            let contribution = 1.0 / (60.0 + rank as f32 + 1.0);
            match merged.iter_mut().find(|m| m.passage_id == hit.passage_id) {
                Some(existing) => existing.score += contribution,
                None => {
                    let mut fused = hit.clone();
                    fused.score = contribution;
                    merged.push(fused);
                }
            }
        }
    }
    merged.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.passage_id.cmp(&b.passage_id))
    });
    merged.truncate(limit);
    merged
}
```

- [ ] **Step 4: Declare and export it** in `note_index/mod.rs`: add `mod search;` after `mod schema;`, and after the `use std::path::Path;` line add a blank line and:

```rust
pub use search::{NoteHit, check_query, fuse_hits};
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p brn-retrieval --locked --test note_search`
Expected: 8 passed. (Fusion order is `[3, 1, 2, 4]`: passages 2 and 4 both score 1/62, so the lower ID comes first.)

- [ ] **Step 6: Clippy and commit**

Run: `cargo fmt --all && cargo clippy -p brn-retrieval --all-targets --locked -- -D warnings`

```bash
git add crates/brn-retrieval/src/note_index crates/brn-retrieval/tests/note_search.rs
git commit -m "feat(retrieval): add FTS5 keyword search over notes

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 3: Embeddings and semantic search

**Files:**
- Create: `crates/brn-retrieval/src/note_index/embeddings.rs`, `crates/brn-retrieval/tests/note_embeddings.rs`
- Modify: `crates/brn-retrieval/src/note_index/mod.rs`

**Interfaces:**
- Consumes: Task 1 `NoteIndex`; Task 2 `NoteHit`, `HIT_COLUMNS`, `hit_row`, `to_hit`, `fuse_hits`.
- Produces:

```rust
pub trait Embedder { fn identity(&self) -> &str; fn dimension(&self) -> usize; fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>>; }
pub struct EmbeddingProgress { pub embedded: usize, pub total: usize }
impl NoteIndex {
    pub fn use_embedding_model(&mut self, identity: &str, dimension: usize) -> Result<()>;
    pub fn embedding_progress(&self) -> Result<EmbeddingProgress>;
    pub fn embed_pending(&mut self, embedder: &mut dyn Embedder, batch: usize) -> Result<EmbeddingProgress>;
    pub fn semantic(&self, query_vector: &[f32], limit: usize) -> Result<Vec<NoteHit>>;
}
```

- [ ] **Step 1: Write the failing tests** in `crates/brn-retrieval/tests/note_embeddings.rs`:

```rust
use brn_retrieval::note_index::{Embedder, EmbeddingProgress, IndexedNote, NoteIndex, fuse_hits};
use sha2::{Digest, Sha256};

/// Three topics: fruit, vehicles, everything else.
struct TopicEmbedder {
    calls: usize,
}

impl Embedder for TopicEmbedder {
    fn identity(&self) -> &str {
        "test-topics-v1"
    }
    fn dimension(&self) -> usize {
        3
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        self.calls += 1;
        Ok(texts
            .iter()
            .map(|text| {
                let mut v = vec![0.0, 0.0, 0.1];
                for word in text.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
                    match word {
                        "apple" | "apples" | "banana" | "bananas" | "fruit" => v[0] += 1.0,
                        "train" | "trains" | "car" | "cars" | "vehicle" => v[1] += 1.0,
                        _ => {}
                    }
                }
                v
            })
            .collect())
    }
}

struct BrokenEmbedder(Vec<f32>);

impl Embedder for BrokenEmbedder {
    fn identity(&self) -> &str {
        "broken"
    }
    fn dimension(&self) -> usize {
        3
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| self.0.clone()).collect())
    }
}

fn note(path: &str, text: &str) -> IndexedNote {
    IndexedNote {
        path: path.into(),
        title: path.into(),
        size: text.len() as u64,
        modified_ns: 1,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}

fn index_with(notes: &[(&str, &str)]) -> (tempfile::TempDir, NoteIndex) {
    let dir = tempfile::tempdir().unwrap();
    let (mut index, _) = NoteIndex::open(&dir.path().join("index.sqlite")).unwrap();
    for (path, text) in notes {
        index.upsert_note(&note(path, text), text).unwrap();
    }
    (dir, index)
}

const NOTES: [(&str, &str); 3] = [
    ("fruit.md", "Apples and bananas in the kitchen."),
    ("travel.md", "Trains and cars for the trip."),
    ("other.md", "Meeting notes."),
];

#[test]
fn embeds_pending_passages_in_batches() {
    let (_dir, mut index) = index_with(&NOTES);
    let mut embedder = TopicEmbedder { calls: 0 };
    assert_eq!(
        index.embed_pending(&mut embedder, 2).unwrap(),
        EmbeddingProgress {
            embedded: 2,
            total: 3
        }
    );
    assert_eq!(
        index.embed_pending(&mut embedder, 2).unwrap(),
        EmbeddingProgress {
            embedded: 3,
            total: 3
        }
    );
    assert_eq!(
        index.embed_pending(&mut embedder, 2).unwrap(),
        EmbeddingProgress {
            embedded: 3,
            total: 3
        }
    );
    assert_eq!(embedder.calls, 2);
}

#[test]
fn semantic_search_finds_by_meaning() {
    let (_dir, mut index) = index_with(&NOTES);
    let mut embedder = TopicEmbedder { calls: 0 };
    index.embed_pending(&mut embedder, 10).unwrap();
    let query = embedder.embed(&["fruit"]).unwrap().remove(0);
    let hits = index.semantic(&query, 2).unwrap();
    assert_eq!(hits[0].path, "fruit.md");
    assert_eq!(hits.len(), 2);
    assert!(index.keyword("fruit", 10).unwrap().is_empty());
}

#[test]
fn hybrid_fusion_combines_keyword_and_semantic() {
    let (_dir, mut index) = index_with(&NOTES);
    let mut embedder = TopicEmbedder { calls: 0 };
    index.embed_pending(&mut embedder, 10).unwrap();
    let keyword = index.keyword("meeting", 50).unwrap();
    let semantic = index
        .semantic(&embedder.embed(&["fruit"]).unwrap().remove(0), 50)
        .unwrap();
    let fused = fuse_hits(&[&keyword, &semantic], 10);
    let top: Vec<&str> = fused.iter().take(2).map(|h| h.path.as_str()).collect();
    assert!(
        top.contains(&"other.md") && top.contains(&"fruit.md"),
        "{top:?}"
    );
}

#[test]
fn model_change_and_reindexing_drop_embeddings() {
    let (_dir, mut index) = index_with(&NOTES);
    let mut embedder = TopicEmbedder { calls: 0 };
    index.embed_pending(&mut embedder, 10).unwrap();
    let text = "Apples only.";
    index.upsert_note(&note("fruit.md", text), text).unwrap();
    assert_eq!(
        index.embedding_progress().unwrap(),
        EmbeddingProgress {
            embedded: 2,
            total: 3
        }
    );
    index.use_embedding_model("test-topics-v2", 3).unwrap();
    assert_eq!(
        index.embedding_progress().unwrap(),
        EmbeddingProgress {
            embedded: 0,
            total: 3
        }
    );
}

#[test]
fn rejects_bad_vectors_and_queries() {
    let (_dir, mut index) = index_with(&NOTES);
    for bad in [
        vec![f32::NAN, 0.0, 0.0],
        vec![0.0, 0.0, 0.0],
        vec![1.0, 2.0],
    ] {
        assert!(matches!(
            index.embed_pending(&mut BrokenEmbedder(bad), 10),
            Err(brn_retrieval::Error::Invalid(_))
        ));
    }
    assert_eq!(index.embedding_progress().unwrap().embedded, 0);
    assert!(index.semantic(&[1.0, 0.0], 5).is_err());
    assert!(index.semantic(&[1.0, 0.0, 0.0], 0).is_err());
}

#[test]
fn semantic_is_empty_before_any_model() {
    let (_dir, index) = index_with(&NOTES);
    assert!(index.semantic(&[1.0, 0.0, 0.0], 5).unwrap().is_empty());
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p brn-retrieval --locked --test note_embeddings`
Expected: compile error, `Embedder` not found.

- [ ] **Step 3: Create `crates/brn-retrieval/src/note_index/embeddings.rs`**

```rust
use super::search::{HIT_COLUMNS, hit_row, to_hit};
use super::{NoteHit, NoteIndex};
use crate::{Error, Result};
use rusqlite::{OptionalExtension, params};

/// Turns text into vectors. Implemented by the local model and by test fakes.
pub trait Embedder {
    /// Stable identity of the model and its settings; a change rebuilds all embeddings.
    fn identity(&self) -> &str;
    fn dimension(&self) -> usize;
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddingProgress {
    pub embedded: usize,
    pub total: usize,
}

fn unit(vector: &[f32]) -> Result<Vec<f32>> {
    if vector.iter().any(|x| !x.is_finite()) {
        return Err(Error::Invalid("embedding is not finite"));
    }
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return Err(Error::Invalid("embedding is all zeros"));
    }
    Ok(vector.iter().map(|x| x / norm).collect())
}

fn to_blob(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn from_blob(bytes: &[u8], dimension: usize) -> Result<Vec<f32>> {
    if bytes.len() != dimension * 4 {
        return Err(Error::Corrupt("stored embedding size"));
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect())
}

impl NoteIndex {
    fn embedding_model(&self) -> Result<Option<(String, usize)>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'embedding_model'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        value
            .map(|value| {
                let (identity, dimension) = value
                    .rsplit_once('|')
                    .ok_or(Error::Corrupt("embedding model record"))?;
                let dimension = dimension
                    .parse()
                    .map_err(|_| Error::Corrupt("embedding model record"))?;
                Ok((identity.to_owned(), dimension))
            })
            .transpose()
    }

    /// Records the embedding model in use. A different identity or dimension
    /// deletes every stored embedding.
    pub fn use_embedding_model(&mut self, identity: &str, dimension: usize) -> Result<()> {
        if identity.is_empty() || identity.contains('|') || dimension == 0 {
            return Err(Error::Invalid("embedding model identity or dimension"));
        }
        if self
            .embedding_model()?
            .is_some_and(|(i, d)| i == identity && d == dimension)
        {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM embeddings", [])?;
        tx.execute(
            "INSERT INTO meta(key, value) VALUES ('embedding_model', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [format!("{identity}|{dimension}")],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn embedding_progress(&self) -> Result<EmbeddingProgress> {
        let total: i64 = self
            .conn
            .query_row("SELECT count(*) FROM passages", [], |r| r.get(0))?;
        let embedded: i64 = self
            .conn
            .query_row("SELECT count(*) FROM embeddings", [], |r| r.get(0))?;
        Ok(EmbeddingProgress {
            embedded: embedded as usize,
            total: total as usize,
        })
    }

    /// Embeds up to `batch` passages that have no embedding yet.
    pub fn embed_pending(
        &mut self,
        embedder: &mut dyn Embedder,
        batch: usize,
    ) -> Result<EmbeddingProgress> {
        let dimension = embedder.dimension();
        self.use_embedding_model(embedder.identity(), dimension)?;
        let pending: Vec<(i64, String)> = {
            let mut statement = self.conn.prepare(
                "SELECT p.id, p.text FROM passages p
                 LEFT JOIN embeddings e ON e.passage_id = p.id
                 WHERE e.passage_id IS NULL ORDER BY p.id LIMIT ?1",
            )?;
            let rows =
                statement.query_map([batch.max(1) as i64], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        if !pending.is_empty() {
            let texts: Vec<&str> = pending.iter().map(|(_, text)| text.as_str()).collect();
            let vectors = embedder.embed(&texts)?;
            if vectors.len() != pending.len() || vectors.iter().any(|v| v.len() != dimension) {
                return Err(Error::Invalid(
                    "embedder returned the wrong count or dimension",
                ));
            }
            let tx = self.conn.transaction()?;
            {
                let mut insert = tx.prepare(
                    "INSERT OR REPLACE INTO embeddings(passage_id, vector) VALUES (?1, ?2)",
                )?;
                for ((id, _), vector) in pending.iter().zip(&vectors) {
                    insert.execute(params![id, to_blob(&unit(vector)?)])?;
                }
            }
            tx.commit()?;
        }
        self.embedding_progress()
    }

    /// Embedded passages closest to `query_vector` by cosine similarity, best first.
    /// Returns nothing before any embedding model has been recorded.
    pub fn semantic(&self, query_vector: &[f32], limit: usize) -> Result<Vec<NoteHit>> {
        if !(1..=50).contains(&limit) {
            return Err(Error::Invalid("query length or limit"));
        }
        let Some((_, dimension)) = self.embedding_model()? else {
            return Ok(Vec::new());
        };
        if query_vector.len() != dimension {
            return Err(Error::Invalid("query embedding dimension"));
        }
        let query = unit(query_vector)?;
        let mut scored: Vec<(f32, i64)> = Vec::new();
        {
            let mut statement = self
                .conn
                .prepare("SELECT passage_id, vector FROM embeddings")?;
            let mut rows = statement.query([])?;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                let vector = from_blob(&bytes, dimension)?;
                let score = vector.iter().zip(&query).map(|(a, b)| a * b).sum::<f32>();
                scored.push((score, id));
            }
        }
        scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        scored.truncate(limit);
        let mut statement = self.conn.prepare(&format!(
            "SELECT {HIT_COLUMNS} FROM passages p JOIN notes n ON n.path = p.path WHERE p.id = ?1"
        ))?;
        scored
            .into_iter()
            .map(|(score, id)| to_hit(statement.query_row([id], hit_row)?, score))
            .collect()
    }
}
```

- [ ] **Step 4: Declare and export it** in `note_index/mod.rs`: add `mod embeddings;` before `mod schema;`, and `pub use embeddings::{Embedder, EmbeddingProgress};` directly before the `pub use search::...` line. The top of the file then reads:

```rust
//! Disposable search index over the vault's notes (`index.sqlite`): note
//! metadata, passages, FTS5 keyword search and local embeddings. It can be
//! deleted at any time and rebuilt from the vault.
mod embeddings;
mod schema;
mod search;

use crate::{Error, Result, chunk};
use rusqlite::{Connection, OptionalExtension, Row, params};
use sha2::{Digest, Sha256};
use std::path::Path;

pub use embeddings::{Embedder, EmbeddingProgress};
pub use search::{NoteHit, check_query, fuse_hits};
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p brn-retrieval --locked`
Expected: all pass (`note_embeddings`: 6).

- [ ] **Step 6: Clippy and commit**

Run: `cargo fmt --all && cargo clippy -p brn-retrieval --all-targets --locked -- -D warnings`

```bash
git add crates/brn-retrieval/src/note_index crates/brn-retrieval/tests/note_embeddings.rs
git commit -m "feat(retrieval): store local embeddings and search by meaning

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 4: Local embedder; remove LanceDB

**Files:**
- Replace: `crates/brn-retrieval/src/native.rs`
- Create: `crates/brn-retrieval/tests/local_embedder.rs`
- Delete: `crates/brn-retrieval/tests/native_smoke.rs`
- Modify: `crates/brn-retrieval/Cargo.toml`, `crates/brn-retrieval/src/lib.rs` (patch below), `crates/brn-retrieval/README.md`, `Cargo.lock`

**Interfaces:**
- Consumes: Task 3 `Embedder`.
- Produces: `brn_retrieval::native::LocalEmbedder` (feature `native`) with `pub fn open(dir: &Path) -> Result<LocalEmbedder>`; identity `fastembed-7.1.0/all-MiniLM-L6-v2/mean/384/<first 16 hex digits of the model.onnx SHA-256>`. The old `Index` returns `Error::Unavailable("semantic search moved to the note index")` for semantic or hybrid search and for `build` with a model folder.

- [ ] **Step 1: Write the tests** in `crates/brn-retrieval/tests/local_embedder.rs`. The model test skips without `BRN_NATIVE_MODEL_DIR`; a skip is not a pass.

```rust
#![cfg(feature = "native")]
use brn_retrieval::{native::LocalEmbedder, note_index::Embedder};

#[test]
fn missing_model_folder_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        LocalEmbedder::open(dir.path()),
        Err(brn_retrieval::Error::Unavailable(_))
    ));
}

/// Needs the five model files in `BRN_NATIVE_MODEL_DIR`; skipped (not passed) otherwise.
#[test]
fn local_model_embeds_by_meaning() {
    let Some(dir) = std::env::var_os("BRN_NATIVE_MODEL_DIR") else {
        eprintln!("skipped: set BRN_NATIVE_MODEL_DIR to a local all-MiniLM-L6-v2 folder");
        return;
    };
    let mut embedder = LocalEmbedder::open(std::path::Path::new(&dir)).unwrap();
    assert_eq!(embedder.dimension(), 384);
    assert!(
        embedder
            .identity()
            .starts_with("fastembed-7.1.0/all-MiniLM-L6-v2/mean/384/")
    );
    let v = embedder
        .embed(&[
            "The release ships in October.",
            "Launch date is next month.",
            "A café menu.",
        ])
        .unwrap();
    let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    assert!(dot(&v[0], &v[1]) > dot(&v[0], &v[2]));
}
```

- [ ] **Step 2: Replace `crates/brn-retrieval/src/native.rs`**

```rust
//! The local embedding model: all-MiniLM-L6-v2 with mean pooling (384
//! dimensions) through FastEmbed, loaded from a folder holding its five files.
use crate::{Error, Result, note_index::Embedder};
use fastembed::{
    InitOptionsUserDefined, Pooling, TextEmbedding, TokenizerFiles, UserDefinedEmbeddingModel,
};
use sha2::{Digest, Sha256};
use std::path::Path;

const DIMENSION: usize = 384;

pub struct LocalEmbedder {
    model: TextEmbedding,
    identity: String,
}

impl LocalEmbedder {
    /// Loads the model from `dir`, which must contain `model.onnx`,
    /// `tokenizer.json`, `config.json`, `special_tokens_map.json` and
    /// `tokenizer_config.json` as regular files. Never downloads anything.
    pub fn open(dir: &Path) -> Result<Self> {
        let load = |name: &str| -> Result<Vec<u8>> {
            let path = dir.join(name);
            match path.symlink_metadata() {
                Ok(meta) if meta.is_file() => Ok(std::fs::read(path)?),
                _ => Err(Error::Unavailable("embedding model files are missing")),
            }
        };
        let onnx = load("model.onnx")?;
        let files = TokenizerFiles {
            tokenizer_file: load("tokenizer.json")?,
            config_file: load("config.json")?,
            special_tokens_map_file: load("special_tokens_map.json")?,
            tokenizer_config_file: load("tokenizer_config.json")?,
        };
        let digest = hex::encode(Sha256::digest(&onnx));
        let identity = format!(
            "fastembed-7.1.0/all-MiniLM-L6-v2/mean/{DIMENSION}/{}",
            &digest[..16]
        );
        let user = UserDefinedEmbeddingModel::new(onnx, files).with_pooling(Pooling::Mean);
        let model = TextEmbedding::try_new_from_user_defined(
            user,
            InitOptionsUserDefined::default().with_intra_threads(4),
        )
        .map_err(|e| Error::Native(e.to_string()))?;
        Ok(Self { model, identity })
    }
}

impl Embedder for LocalEmbedder {
    fn identity(&self) -> &str {
        &self.identity
    }
    fn dimension(&self) -> usize {
        DIMENSION
    }
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.model
            .embed(texts, None)
            .map_err(|e| Error::Native(e.to_string()))
    }
}
```

- [ ] **Step 3: Update `crates/brn-retrieval/Cargo.toml`.** The `[features]` and `[dependencies]` sections become:

```toml
[features]
default = []
native = ["dep:fastembed"]

[dependencies]
sha2 = "0.10"
hex = "0.4"
uuid = { version = "1", features = ["v4"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
rusqlite = { version = "=0.40.2", default-features = false, features = ["bundled"] }
fastembed = { version = "=7.1.0", default-features = false, features = ["ort-download-binaries-rustls-tls"], optional = true }
```

`lancedb`, `arrow-array`, `arrow-schema`, `tokio` and `futures` are removed; fastembed's `hf-hub-rustls-tls` feature is dropped because this code never downloads a model.

- [ ] **Step 4: Remove the old LanceDB paths from `crates/brn-retrieval/src/lib.rs`.** Save this patch to a git-ignored file (for example `.superpowers/task4.patch`) and run `git apply .superpowers/task4.patch`:

```diff
--- a/crates/brn-retrieval/src/lib.rs
+++ b/crates/brn-retrieval/src/lib.rs
@@ -84,6 +84,7 @@
 const CHUNKER: &str = "utf8-1600-v1";
 const MAX_DOCUMENT: usize = 1_048_576;
 const FORMAT: u32 = 1;
+const SEMANTIC_MOVED: &str = "semantic search moved to the note index";
 
 pub fn hash(bytes: &[u8]) -> String {
     hex::encode(Sha256::digest(bytes))
@@ -138,41 +139,6 @@
     File::open(path)?.sync_all()?;
     Ok(())
 }
-#[cfg(feature = "native")]
-fn inventory(root: &Path) -> Result<BTreeMap<String, String>> {
-    if !root.is_dir() {
-        return Err(Error::Corrupt("missing native directory"));
-    }
-    fn visit(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> Result<()> {
-        for entry in fs::read_dir(dir)? {
-            let entry = entry?;
-            let kind = entry.file_type()?;
-            if kind.is_symlink() {
-                return Err(Error::Corrupt("symlink in native files"));
-            }
-            if kind.is_dir() {
-                visit(root, &entry.path(), out)?;
-            } else if kind.is_file() {
-                let name = entry
-                    .path()
-                    .strip_prefix(root)
-                    .map_err(|_| Error::Corrupt("invalid native file path"))?
-                    .to_string_lossy()
-                    .into_owned();
-                out.insert(name, hash(&fs::read(entry.path())?));
-            } else {
-                return Err(Error::Corrupt("invalid native file type"));
-            }
-        }
-        Ok(())
-    }
-    let mut result = BTreeMap::new();
-    visit(root, root, &mut result)?;
-    if result.is_empty() {
-        return Err(Error::Corrupt("empty native directory"));
-    }
-    Ok(result)
-}
 fn validate_chunks(docs: &[Document], chunks: &[Chunk]) -> Result<()> {
     let expected = chunk_documents(docs, &AtomicBool::new(false))?;
     if expected != chunks {
@@ -186,14 +152,11 @@
     docs: Vec<Document>,
     chunks: Vec<Chunk>,
     manifest: Manifest,
-    #[cfg(feature = "native")]
-    native: Option<native::NativeState>,
 }
 impl Index {
     /// Builds in a new directory only; `COMPLETE` is written and synced last.
-    /// Each document is limited to 1 MiB. Native embedding runs in batches of at
-    /// most 16 passages; model initialization and an individual ORT batch cannot
-    /// be interrupted, so callers should run builds on a supervised worker.
+    /// Each document is limited to 1 MiB. Semantic search moved to
+    /// [`note_index`], so `model_dir` must be `None`.
     pub fn build(
         path: &Path,
         documents: &[Document],
@@ -203,9 +166,8 @@
     ) -> Result<Self> {
         validate_documents(documents)?;
         check_cancel(cancel)?;
-        #[cfg(not(feature = "native"))]
         if model_dir.is_some() {
-            return Err(Error::Unavailable("native feature disabled"));
+            return Err(Error::Unavailable(SEMANTIC_MOVED));
         }
         fs::create_dir(path)?;
         on_progress("chunking");
@@ -214,28 +176,6 @@
         let chunks_bytes = serde_json::to_vec(&chunks)?;
         write_sync(&path.join("documents.json"), &docs_bytes)?;
         write_sync(&path.join("chunks.json"), &chunks_bytes)?;
-        #[cfg(feature = "native")]
-        let (model_identity, model_files, db_files) = if let Some(model_dir) = model_dir {
-            check_cancel(cancel)?;
-            on_progress("embedding");
-            let (identity, model_files, db_files) = native::build(
-                path,
-                &chunks,
-                documents,
-                model_dir,
-                cancel,
-                &mut on_progress,
-            )?;
-            (Some(identity), model_files, db_files)
-        } else {
-            (None, BTreeMap::new(), BTreeMap::new())
-        };
-        #[cfg(not(feature = "native"))]
-        let (model_identity, model_files, db_files): (
-            Option<String>,
-            BTreeMap<String, String>,
-            BTreeMap<String, String>,
-        ) = (None, BTreeMap::new(), BTreeMap::new());
         check_cancel(cancel)?;
         let manifest = Manifest {
             format: FORMAT,
@@ -244,10 +184,10 @@
             fingerprint: hash(&docs_bytes),
             documents_hash: hash(&docs_bytes),
             chunks_hash: hash(&chunks_bytes),
-            native: model_identity.is_some(),
-            model_identity,
-            model_files,
-            db_files,
+            native: false,
+            model_identity: None,
+            model_files: BTreeMap::new(),
+            db_files: BTreeMap::new(),
         };
         let manifest_bytes = serde_json::to_vec(&manifest)?;
         write_sync(&path.join("manifest.json"), &manifest_bytes)?;
@@ -298,15 +238,11 @@
         {
             return Err(Error::Corrupt("unexpected native manifest"));
         }
-        #[cfg(feature = "native")]
-        let native = None;
         Ok(Self {
             path: path.to_path_buf(),
             docs,
             chunks,
             manifest,
-            #[cfg(feature = "native")]
-            native,
         })
     }
     pub fn generation(&self) -> &str {
@@ -369,59 +305,9 @@
         hits.truncate(limit);
         hits
     }
-    fn semantic(&mut self, query: &str, limit: usize) -> Result<Vec<Evidence>> {
-        #[cfg(feature = "native")]
-        {
-            if !self.manifest.native {
-                return Err(Error::Unavailable("semantic resources absent"));
-            }
-            if self.native.is_none() {
-                self.open_native()?;
-            }
-            let ranks = self
-                .native
-                .as_mut()
-                .expect("opened above")
-                .search(query, limit)?;
-            ranks
-                .into_iter()
-                .map(|(id, score)| {
-                    let chunk = self
-                        .chunks
-                        .get(id)
-                        .ok_or(Error::Corrupt("native passage id"))?;
-                    Ok(self.hit(chunk, score, "cosine_similarity"))
-                })
-                .collect()
-        }
-        #[cfg(not(feature = "native"))]
-        {
-            let _ = (query, limit);
-            Err(Error::Unavailable("native feature disabled"))
-        }
+    fn semantic(&mut self, _query: &str, _limit: usize) -> Result<Vec<Evidence>> {
+        Err(Error::Unavailable(SEMANTIC_MOVED))
     }
-    #[cfg(feature = "native")]
-    fn open_native(&mut self) -> Result<()> {
-        if self.manifest.model_identity.as_deref() != Some(native_identity()) {
-            return Err(Error::Unavailable("incompatible semantic model"));
-        }
-        let model = self.path.join("model");
-        let database = self.path.join("lancedb");
-        if !model.is_dir() || !database.is_dir() {
-            return Err(Error::Unavailable("semantic resources absent"));
-        }
-        if inventory(&model)? != self.manifest.model_files
-            || inventory(&database)? != self.manifest.db_files
-        {
-            return Err(Error::Corrupt("native resource mismatch"));
-        }
-        self.native = Some(native::NativeState::open(
-            &self.path,
-            &self.docs,
-            &self.chunks,
-        )?);
-        Ok(())
-    }
     fn hit(&self, chunk: &Chunk, score: f32, kind: &str) -> Evidence {
         let doc = &self.docs[chunk.doc];
         Evidence {
@@ -466,10 +352,6 @@
         .map(str::to_lowercase)
         .collect()
 }
-#[cfg(feature = "native")]
-fn native_identity() -> &'static str {
-    "fastembed-7.1.0/all-MiniLM-L6-v2-onnx/mean/384"
-}
 fn fuse(a: &[Evidence], b: &[Evidence], limit: usize) -> Vec<Evidence> {
     let mut merged: Vec<Evidence> = Vec::new();
     for list in [a, b] {
@@ -495,7 +377,7 @@
     merged
 }
 #[cfg(feature = "native")]
-mod native;
+pub mod native;
 
 mod chunk;
 pub mod note_index;
```

Then delete the old test: `git rm crates/brn-retrieval/tests/native_smoke.rs`.

- [ ] **Step 5: Build, test and lint both feature sets**

Run (the first build without `--locked`, to shrink `Cargo.lock`):

```sh
cargo build -p brn-retrieval
cargo test -p brn-retrieval --locked
cargo test -p brn-retrieval --locked --features native
cargo clippy -p brn-retrieval --all-targets --locked -- -D warnings
cargo clippy -p brn-retrieval --all-targets --locked --features native -- -D warnings
cargo check -p brn-workflow -p brn --locked --features brn-workflow/native-retrieval
cargo check -p brn-desktop --locked --features native-ui,native-retrieval
grep -c 'name = "lancedb"' Cargo.lock
```

Expected: all pass; `local_embedder` 2 passed (the model test prints `skipped` unless `BRN_NATIVE_MODEL_DIR` is set); `grep` prints `0`. The `native` build downloads ONNX Runtime binaries once (fastembed's existing build behaviour, not a model download). Record whether the `brn-desktop` native-retrieval check now succeeds without `protoc` (before, LanceDB needed it).

- [ ] **Step 6: Update `crates/brn-retrieval/README.md`.** Replace its title, first paragraph, "Interfaces and source" and "Dependencies and features" sections with:

```markdown
# brn-retrieval

Search over the vault's notes: a disposable `index.sqlite` with FTS5 keyword search, local embeddings and reciprocal-rank fusion ([`note_index`](src/note_index/mod.rs)), plus the local embedding model ([`native`](src/native.rs), feature `native`). The older generation-based `Index` in [lib.rs](src/lib.rs) keeps keyword search only until the cleanup step removes it.

## Dependencies and features

No workspace dependencies. Default features are empty. `native` enables FastEmbed/ONNX for the local embedding model. Consumed by `brn-workflow`.
```

In its "Verification" section, replace the sentence starting "Native tests require inspection" with: "The local model test runs only when `BRN_NATIVE_MODEL_DIR` points at the model files; a skip is not model verification. Never treat an index as authoritative storage."

- [ ] **Step 7: Commit**

```bash
git add Cargo.lock crates/brn-retrieval
git commit -m "refactor(retrieval): replace LanceDB with the local embedder

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 5: Library in the workflow

**Files:**
- Create: `crates/brn-workflow/src/library.rs`, `crates/brn-workflow/tests/library.rs`
- Modify: `crates/brn-workflow/src/lib.rs` (add `pub mod library;` after `pub mod error;`)

**Interfaces:**
- Consumes: step 2 `brn_workflow::vault::{scan, read_note, ReadError, SkipReason, VaultPath}`; Tasks 1–4 `NoteIndex`, `check_query`, `fuse_hits`, `Embedder`, `EmbeddingProgress`, `IndexedNote`, `NoteHit`, `LocalEmbedder`.
- Produces (for step 4's CLI and AI tools):

```rust
impl Library {
    pub fn open(vault_root: &Path, index_path: &Path, embedder: Option<Box<dyn Embedder + Send>>) -> LibraryResult<Library>;
    pub fn refresh(&mut self) -> LibraryResult<RefreshReport>;
    pub fn embed_pending(&mut self, batch: usize) -> LibraryResult<Option<EmbeddingProgress>>;
    pub fn search(&mut self, query: &str, mode: SearchMode, limit: usize) -> LibraryResult<SearchResults>;
    pub fn notes(&self) -> LibraryResult<Vec<IndexedNote>>;
}
pub struct RefreshReport { pub added: usize, pub updated: usize, pub removed: usize, pub unchanged: usize, pub unreadable: Vec<Unreadable> }
pub struct Unreadable { pub path: String, pub reason: &'static str }
pub enum SearchMode { Keyword, Semantic, Hybrid }
pub struct SearchResults { pub hits: Vec<NoteHit>, pub keyword_only: bool }
pub enum LibraryError { Io(std::io::Error), Index(brn_retrieval::Error) }
// re-exported: Embedder, EmbeddingProgress, IndexedNote, NoteHit; LocalEmbedder with feature native-retrieval
```

- [ ] **Step 1: Write the failing tests** in `crates/brn-workflow/tests/library.rs`:

```rust
use brn_workflow::library::{
    Embedder, EmbeddingProgress, Library, RefreshReport, SearchMode, Unreadable,
};
use std::{
    path::Path,
    time::{Duration, SystemTime},
};

/// Three topics: fruit, vehicles, everything else.
struct TopicEmbedder;

impl Embedder for TopicEmbedder {
    fn identity(&self) -> &str {
        "test-topics-v1"
    }
    fn dimension(&self) -> usize {
        3
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|text| {
                let mut v = vec![0.0, 0.0, 0.1];
                for word in text.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
                    match word {
                        "apple" | "apples" | "banana" | "bananas" | "fruit" => v[0] += 1.0,
                        "train" | "trains" | "car" | "cars" | "vehicle" => v[1] += 1.0,
                        _ => {}
                    }
                }
                v
            })
            .collect())
    }
}

fn write(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn bump_mtime(root: &Path, relative: &str) {
    let file = std::fs::File::options()
        .write(true)
        .open(root.join(relative))
        .unwrap();
    file.set_modified(SystemTime::now() + Duration::from_secs(60))
        .unwrap();
}

struct Setup {
    vault: tempfile::TempDir,
    data: tempfile::TempDir,
}

impl Setup {
    fn new() -> Self {
        Self {
            vault: tempfile::tempdir().unwrap(),
            data: tempfile::tempdir().unwrap(),
        }
    }
    fn open(&self, embedder: Option<Box<dyn Embedder + Send>>) -> Library {
        Library::open(
            self.vault.path(),
            &self.data.path().join("index.sqlite"),
            embedder,
        )
        .unwrap()
    }
}

#[test]
fn refresh_tracks_added_changed_unchanged_and_removed_notes() {
    let s = Setup::new();
    let root = s.vault.path();
    write(root, "a.md", b"# Alpha plan\nfirst");
    write(root, "sub/b.md", b"second");
    write(root, "archive/old.md", b"archived");
    write(root, ".hidden/h.md", b"hidden");
    let mut library = s.open(None);
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            added: 2,
            ..Default::default()
        }
    );
    let notes = library.notes().unwrap();
    assert_eq!(
        notes
            .iter()
            .map(|n| (n.path.as_str(), n.title.as_str()))
            .collect::<Vec<_>>(),
        vec![("a.md", "Alpha plan"), ("sub/b.md", "b")]
    );

    write(root, "a.md", b"# Alpha plan\nfirst, edited");
    bump_mtime(root, "a.md");
    std::fs::remove_file(root.join("sub/b.md")).unwrap();
    write(root, "c.md", b"third");
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            added: 1,
            updated: 1,
            removed: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            unchanged: 2,
            ..Default::default()
        }
    );
}

#[test]
fn touched_but_identical_note_is_not_reindexed() {
    let s = Setup::new();
    write(s.vault.path(), "a.md", b"same");
    let mut library = s.open(None);
    library.refresh().unwrap();
    bump_mtime(s.vault.path(), "a.md");
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            unchanged: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            unchanged: 1,
            ..Default::default()
        }
    );
}

#[test]
fn unreadable_notes_are_listed_and_dropped_from_the_index() {
    let s = Setup::new();
    let root = s.vault.path();
    write(root, "bad.md", b"fine at first");
    write(root, "big.md", &vec![b'x'; 1024 * 1024 + 1]);
    let mut library = s.open(None);
    library.refresh().unwrap();
    write(root, "bad.md", &[0xff, 0xfe]);
    bump_mtime(root, "bad.md");
    let report = library.refresh().unwrap();
    assert_eq!(report.removed, 1);
    assert_eq!(
        report.unreadable,
        vec![
            Unreadable {
                path: "bad.md".into(),
                reason: "not valid UTF-8"
            },
            Unreadable {
                path: "big.md".into(),
                reason: "larger than 1 MiB"
            },
        ]
    );
    assert!(library.notes().unwrap().is_empty());
}

#[test]
fn search_without_a_model_is_keyword_only_and_says_so() {
    let s = Setup::new();
    write(s.vault.path(), "fruit.md", b"Apples and bananas.");
    let mut library = s.open(None);
    library.refresh().unwrap();
    assert_eq!(library.embed_pending(10).unwrap(), None);
    let keyword = library.search("apples", SearchMode::Keyword, 10).unwrap();
    assert!(!keyword.keyword_only);
    assert_eq!(keyword.hits[0].path, "fruit.md");
    let hybrid = library.search("apples", SearchMode::Hybrid, 10).unwrap();
    assert!(hybrid.keyword_only);
    assert_eq!(hybrid.hits[0].path, "fruit.md");
}

#[test]
fn semantic_and_hybrid_search_use_embedded_passages() {
    let s = Setup::new();
    write(s.vault.path(), "fruit.md", b"Apples and bananas.");
    write(s.vault.path(), "travel.md", b"Trains and cars.");
    let mut library = s.open(Some(Box::new(TopicEmbedder)));
    library.refresh().unwrap();
    assert!(
        library
            .search("fruit", SearchMode::Semantic, 5)
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(
        library.embed_pending(10).unwrap(),
        Some(EmbeddingProgress {
            embedded: 2,
            total: 2
        })
    );
    let semantic = library.search("fruit", SearchMode::Semantic, 1).unwrap();
    assert!(!semantic.keyword_only);
    assert_eq!(semantic.hits[0].path, "fruit.md");
    let hybrid = library.search("trains", SearchMode::Hybrid, 2).unwrap();
    assert_eq!(hybrid.hits[0].path, "travel.md");
}

#[test]
fn deleted_index_is_rebuilt_from_the_vault() {
    let s = Setup::new();
    write(s.vault.path(), "a.md", b"alpha");
    let index = s.data.path().join("index.sqlite");
    {
        let mut library = s.open(None);
        library.refresh().unwrap();
    }
    std::fs::remove_file(&index).unwrap();
    let mut library = s.open(None);
    assert_eq!(
        library.refresh().unwrap(),
        RefreshReport {
            added: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        library
            .search("alpha", SearchMode::Keyword, 5)
            .unwrap()
            .hits
            .len(),
        1
    );
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p brn-workflow --locked --test library`
Expected: compile error, unresolved module `library`.

- [ ] **Step 3: Create `crates/brn-workflow/src/library.rs`**

```rust
//! The vault's notes as a searchable library: keeps `index.sqlite` in step
//! with the vault and answers keyword, semantic and hybrid searches.
use crate::vault::{self, ReadError, SkipReason, VaultPath};
use brn_retrieval::note_index::{NoteIndex, check_query, fuse_hits};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

#[cfg(feature = "native-retrieval")]
pub use brn_retrieval::native::LocalEmbedder;
pub use brn_retrieval::note_index::{Embedder, EmbeddingProgress, IndexedNote, NoteHit};

/// How many results each side contributes before hybrid fusion.
const FUSION_DEPTH: usize = 50;

#[derive(Debug)]
pub enum LibraryError {
    Io(std::io::Error),
    Index(brn_retrieval::Error),
}

impl std::fmt::Display for LibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "could not read the vault: {e}"),
            Self::Index(e) => write!(f, "search index: {e}"),
        }
    }
}

impl std::error::Error for LibraryError {}

impl From<std::io::Error> for LibraryError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<brn_retrieval::Error> for LibraryError {
    fn from(e: brn_retrieval::Error) -> Self {
        Self::Index(e)
    }
}

pub type LibraryResult<T> = Result<T, LibraryError>;

/// A `.md` file in the vault that could not be indexed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RefreshReport {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub unchanged: usize,
    /// Sorted by path.
    pub unreadable: Vec<Unreadable>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Keyword,
    Semantic,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResults {
    pub hits: Vec<NoteHit>,
    /// Semantic search was requested but no embedding model is installed, so
    /// only keyword search ran. Callers must tell the user.
    pub keyword_only: bool,
}

pub struct Library {
    root: PathBuf,
    index: NoteIndex,
    embedder: Option<Box<dyn Embedder + Send>>,
}

impl Library {
    /// Opens the index at `index_path` (rebuilding it if damaged) for the
    /// vault at `vault_root`. Without an embedder, searches are keyword-only.
    pub fn open(
        vault_root: &Path,
        index_path: &Path,
        embedder: Option<Box<dyn Embedder + Send>>,
    ) -> LibraryResult<Self> {
        let (mut index, _) = NoteIndex::open(index_path)?;
        if let Some(embedder) = &embedder {
            index.use_embedding_model(embedder.identity(), embedder.dimension())?;
        }
        Ok(Self {
            root: vault_root.to_path_buf(),
            index,
            embedder,
        })
    }

    /// Brings the index in step with the vault. Files whose size and
    /// modification time are unchanged are not read again.
    pub fn refresh(&mut self) -> LibraryResult<RefreshReport> {
        let scan = vault::scan(&self.root)?;
        let mut report = RefreshReport::default();
        for skipped in &scan.skipped {
            report.unreadable.push(Unreadable {
                path: skipped.path.clone(),
                reason: match skipped.reason {
                    SkipReason::TooLarge => "larger than 1 MiB",
                    SkipReason::InvalidName => "file name is not valid UTF-8",
                },
            });
        }
        let indexed: HashMap<String, IndexedNote> = self
            .index
            .notes()?
            .into_iter()
            .map(|note| (note.path.clone(), note))
            .collect();
        let mut seen = HashSet::new();
        for file in &scan.notes {
            let path = file.path.as_str();
            seen.insert(path.to_owned());
            let old = indexed.get(path);
            if old.is_some_and(|old| old.size == file.size && old.modified_ns == file.modified_ns) {
                report.unchanged += 1;
                continue;
            }
            let note = match vault::read_note(&self.root, &file.path) {
                Ok(note) => note,
                Err(ReadError::Io(e)) => return Err(e.into()),
                Err(error) => {
                    if let Some(reason) = unreadable_reason(&error) {
                        report.unreadable.push(Unreadable {
                            path: path.to_owned(),
                            reason,
                        });
                    }
                    if old.is_some() {
                        self.index.remove_note(path)?;
                        report.removed += 1;
                    }
                    continue;
                }
            };
            let size = note.text.len() as u64;
            if old.is_some_and(|old| old.sha256 == note.sha256) {
                self.index.update_metadata(path, size, file.modified_ns)?;
                report.unchanged += 1;
                continue;
            }
            let record = IndexedNote {
                path: path.to_owned(),
                title: title(&note.text, &file.path),
                size,
                modified_ns: file.modified_ns,
                sha256: note.sha256,
            };
            self.index.upsert_note(&record, &note.text)?;
            if old.is_some() {
                report.updated += 1;
            } else {
                report.added += 1;
            }
        }
        for path in indexed.keys().filter(|path| !seen.contains(*path)) {
            self.index.remove_note(path)?;
            report.removed += 1;
        }
        report.unreadable.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(report)
    }

    /// Embeds up to `batch` passages that have none yet. Call repeatedly in
    /// the background until `embedded == total`. `None` without an embedder.
    pub fn embed_pending(&mut self, batch: usize) -> LibraryResult<Option<EmbeddingProgress>> {
        match self.embedder.as_deref_mut() {
            Some(embedder) => Ok(Some(self.index.embed_pending(embedder, batch)?)),
            None => Ok(None),
        }
    }

    /// Searches the indexed notes. Semantic and hybrid searches fall back to
    /// keyword search, flagged by `keyword_only`, when no model is installed;
    /// they cover only passages embedded so far.
    pub fn search(
        &mut self,
        query: &str,
        mode: SearchMode,
        limit: usize,
    ) -> LibraryResult<SearchResults> {
        check_query(query, limit)?;
        let embedder = match (mode, self.embedder.as_deref_mut()) {
            (SearchMode::Keyword, _) | (_, None) => {
                return Ok(SearchResults {
                    hits: self.index.keyword(query, limit)?,
                    keyword_only: mode != SearchMode::Keyword,
                });
            }
            (_, Some(embedder)) => embedder,
        };
        let vector = embedder
            .embed(&[query])?
            .pop()
            .ok_or(brn_retrieval::Error::Invalid("embedder returned no vector"))?;
        let hits = if mode == SearchMode::Semantic {
            self.index.semantic(&vector, limit)?
        } else {
            let keyword = self.index.keyword(query, FUSION_DEPTH)?;
            let semantic = self.index.semantic(&vector, FUSION_DEPTH)?;
            fuse_hits(&[&keyword, &semantic], limit)
        };
        Ok(SearchResults {
            hits,
            keyword_only: false,
        })
    }

    /// All indexed notes, ordered by path.
    pub fn notes(&self) -> LibraryResult<Vec<IndexedNote>> {
        Ok(self.index.notes()?)
    }
}

fn unreadable_reason(error: &ReadError) -> Option<&'static str> {
    match error {
        ReadError::NotUtf8 => Some("not valid UTF-8"),
        ReadError::TooLarge => Some("larger than 1 MiB"),
        ReadError::Missing | ReadError::NotAFile | ReadError::Io(_) => None,
    }
}

/// The first level-1 Markdown heading in the first 50 lines, or the file name without `.md`.
fn title(text: &str, path: &VaultPath) -> String {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.lines()
        .take(50)
        .find_map(|line| line.strip_prefix("# "))
        .map(str::trim)
        .filter(|heading| !heading.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            let name = path.as_str().rsplit('/').next().unwrap_or(path.as_str());
            name[..name.len() - 3].to_owned()
        })
}
```

- [ ] **Step 4: Add `pub mod library;`** to `crates/brn-workflow/src/lib.rs` directly after `pub mod error;`.

- [ ] **Step 5: Run tests and lint**

```sh
cargo test -p brn-workflow --locked --test library
cargo clippy -p brn-workflow --all-targets --locked -- -D warnings
cargo clippy -p brn-workflow --all-targets --locked --features native-retrieval -- -D warnings
```

Expected: 6 passed; no warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/brn-workflow/src/lib.rs crates/brn-workflow/src/library.rs crates/brn-workflow/tests/library.rs
git commit -m "feat(workflow): keep a searchable library of vault notes

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 6: Documentation and evidence

**Files:**
- Modify: `docs/work/active/simple-rig-notes/plan.md`, `docs/work/active/simple-rig-notes/evidence.md`, `docs/development/verification.md`

- [ ] **Step 1: Update the roadmap table** in `plan.md`:
  - Step 3 row, "Plan" cell: `[Search](search.md)`; "Delivers" cell: `` `index.sqlite`: notes, passages, FTS5, local embeddings, fusion; non-UTF-8 notes listed as unreadable; workflow `Library`; LanceDB removed. ``
  - Step 4 row, "Delivers" cell: `` `brn-ai`, Connect/Disconnect/select, read tools, saved conversations; CLI moves to the new data folder with `notes list`, `search`, `ask`; model download with consent; `brn-provider` removed. ``
  - Below the table, change "Plans for steps 3–6 are written when their inputs exist" to "Plans for steps 4–6 are written when their inputs exist".

- [ ] **Step 2: Update `docs/development/verification.md`.** In the "Select checks by change" table, rename the "Keyword retrieval" row to "Search index or keyword retrieval" (keep its checks), and replace the "Native retrieval" row's checks with: `` `cargo test -p brn-retrieval --features native --locked` (set `BRN_NATIVE_MODEL_DIR` to run the local model test; otherwise it is skipped); `cargo check -p brn-desktop --features native-ui,native-retrieval --locked` ``.

- [ ] **Step 3: Record evidence.** Append "## Step 3: search" to `evidence.md` (at most 20 lines): date, commits, `rustc --version`, and the actual results of:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p brn-retrieval -p brn-workflow --locked --no-fail-fast
cargo test -p brn-retrieval --locked --features native
cargo check -p brn-desktop --locked --features native-ui,native-retrieval
git diff --check
```

Note the pre-existing `note_evidence` failure (if it still fails) as not caused by this step, whether the local model test ran or was skipped, and whether native retrieval now builds without `protoc`.

- [ ] **Step 4: Commit**

```bash
git add docs/work/active/simple-rig-notes/plan.md docs/work/active/simple-rig-notes/evidence.md docs/development/verification.md
git commit -m "docs: record search step

Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```
