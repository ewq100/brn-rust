use crate::{Error, Result};
use rusqlite::Connection;
use std::{
    ffi::OsString,
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
    time::Duration,
};

const APPLICATION_ID: i64 = 0x4252_4e49; // BRNI
const VERSION: i64 = 3;
const NOT_OURS: &str =
    "index path holds a file that is not a BRN index; remove it or choose another path";
const SCHEMA: &str = "
CREATE TABLE notes (
    path TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    size INTEGER NOT NULL,
    modified_ns INTEGER NOT NULL,
    sha256 BLOB NOT NULL CHECK(length(sha256) = 32),
    note_id TEXT,
    source INTEGER NOT NULL DEFAULT 0 CHECK(source IN (0, 1)),
    history INTEGER NOT NULL DEFAULT 0 CHECK(history IN (0, 1)),
    metadata_issue TEXT
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

pub(super) fn validate_reader(conn: &Connection) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    let application: i64 = tx.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let version: i64 = tx.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let required: i64 = tx.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE name IN (
            'notes', 'passages', 'passages_path', 'passages_fts',
            'passages_insert', 'passages_delete', 'embeddings', 'meta', 'edges')",
        [],
        |r| r.get(0),
    )?;
    if application != APPLICATION_ID || version != VERSION || required != 9 {
        return Err(Error::Corrupt("read-only note index identity or schema"));
    }
    let healthy: String = tx.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if healthy != "ok" {
        return Err(Error::Corrupt("read-only note index integrity"));
    }
    if !valid_notes(&tx)? {
        return Err(Error::Corrupt("read-only note metadata schema or rows"));
    }
    if !super::edges::valid_schema(&tx)? {
        return Err(Error::Corrupt("read-only edge schema"));
    }
    super::edges::validate_rows(&tx)?;
    tx.commit()?;
    Ok(())
}

pub(super) fn open(path: &Path) -> Result<(Connection, bool)> {
    if path.try_exists()?
        && let Found::Usable(conn) = existing(path)?
    {
        configure(&conn)?;
        return Ok((conn, false));
    }
    remove(path)?;
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    let tx = conn.transaction()?;
    tx.execute_batch(SCHEMA)?;
    tx.execute_batch(super::edges::SCHEMA)?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", VERSION)?;
    tx.commit()?;
    // Persist the brand before readers attach: an uncheckpointed WAL must not
    // leave our new main file looking like a foreign, unbranded database.
    let (busy, frames, copied): (i64, i64, i64) =
        conn.query_row("PRAGMA wal_checkpoint(FULL)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
    if busy != 0 || frames != copied {
        return Err(Error::Corrupt("new index header checkpoint"));
    }
    Ok((conn, true))
}

/// Disposable process-local index over current database revisions.
pub(super) fn memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    let tx = conn.transaction()?;
    tx.execute_batch(SCHEMA)?;
    tx.execute_batch(super::edges::SCHEMA)?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", VERSION)?;
    tx.commit()?;
    Ok(conn)
}

enum Found {
    Usable(Connection),
    Rebuild,
}

fn existing(path: &Path) -> Result<Found> {
    let mut file = std::fs::File::open(path)?;
    if file.metadata()?.len() == 0 {
        return Ok(Found::Rebuild);
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
    if application != 0 && application != APPLICATION_ID {
        return Err(Error::Invalid(NOT_OURS));
    }
    let found = (|| {
        let mut conn = Connection::open(path)?;
        let healthy: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if healthy != "ok" {
            return if application == APPLICATION_ID {
                Ok(Found::Rebuild)
            } else {
                Err(Error::Invalid(NOT_OURS))
            };
        }
        if application == APPLICATION_ID {
            let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
            let required: i64 = conn.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name IN (
                    'notes', 'passages', 'passages_path', 'passages_fts',
                    'passages_insert', 'passages_delete', 'embeddings', 'meta', 'edges'
                )",
                [],
                |r| r.get(0),
            )?;
            if valid_notes(&conn)? {
                let edges_present: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name='edges')",
                    [],
                    |row| row.get(0),
                )?;
                if version == 2 && required == 8 && !edges_present {
                    configure(&conn)?;
                    let tx = conn.transaction()?;
                    tx.execute_batch(super::edges::SCHEMA)?;
                    tx.pragma_update(None, "user_version", VERSION)?;
                    tx.commit()?;
                    return Ok(Found::Usable(conn));
                }
                if version == VERSION && required == 9 && super::edges::valid_schema(&conn)? {
                    match super::edges::validate_rows(&conn) {
                        Ok(()) => return Ok(Found::Usable(conn)),
                        Err(Error::Corrupt(_)) => return Ok(Found::Rebuild),
                        Err(error) => return Err(error),
                    }
                }
            }
            return Ok(Found::Rebuild);
        }
        let objects: i64 =
            conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
        if objects == 0 {
            return Ok(Found::Rebuild);
        }
        Err(Error::Invalid(NOT_OURS))
    })();
    match found {
        Err(Error::Sql(error))
            if matches!(
                error.sqlite_error_code(),
                Some(rusqlite::ErrorCode::NotADatabase | rusqlite::ErrorCode::DatabaseCorrupt)
            ) =>
        {
            if application == APPLICATION_ID {
                Ok(Found::Rebuild)
            } else {
                Err(Error::Invalid(NOT_OURS))
            }
        }
        other => other,
    }
}

fn valid_notes(conn: &Connection) -> Result<bool> {
    let expected = [
        ("path", "TEXT", 0, 1),
        ("title", "TEXT", 1, 0),
        ("size", "INTEGER", 1, 0),
        ("modified_ns", "INTEGER", 1, 0),
        ("sha256", "BLOB", 1, 0),
        ("note_id", "TEXT", 0, 0),
        ("source", "INTEGER", 1, 0),
        ("history", "INTEGER", 1, 0),
        ("metadata_issue", "TEXT", 0, 0),
    ];
    let mut columns = conn.prepare("PRAGMA table_info(notes)")?;
    let actual = columns
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
    let mut statement =
        conn.prepare("SELECT note_id, source, history, metadata_issue FROM notes")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        match super::metadata_row(row) {
            Ok(metadata) => {
                if super::to_metadata(metadata).is_err() {
                    return Ok(false);
                }
            }
            Err(
                rusqlite::Error::InvalidColumnType(..)
                | rusqlite::Error::FromSqlConversionFailure(..),
            ) => return Ok(false),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(true)
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
