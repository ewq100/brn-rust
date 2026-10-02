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
