//! User-work database for the simple notes app (`brn.sqlite`). It is checked
//! on every open, restored from the newest backup when corrupt, and backed
//! up after every successful open. Notes themselves live in the vault.
mod backup;
pub mod chat;
pub mod editor;
mod edits;
pub mod proposal_apply;
mod proposal_repair;
pub mod proposal_rewrite;
mod proposal_undo;
pub mod proposals;

use crate::{Result, acquire_owner_lock, check_regular_single_link, invalid};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub use chat::{WorkConversation, WorkTurn, WorkTurnStatus};
pub use editor::{
    EditRequest, EditStamp, EditorRecord, SaveIntent, SaveOutcome, SaveReceipt, SaveRequest,
};
pub use edits::UnsavedEdit;

/// Largest note, unsaved edit or proposal text, in bytes.
pub const MAX_NOTE_BYTES: usize = 1024 * 1024;
const APPLICATION_ID: i64 = 0x4252_4e32; // BRN2
const DB_NAME: &str = "brn.sqlite";
/// Each entry upgrades the schema by one version; `user_version` is the number applied.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    CREATE TABLE unsaved_edits (
        path TEXT PRIMARY KEY,
        base_sha256 BLOB NOT NULL CHECK(length(base_sha256) = 32),
        text TEXT NOT NULL,
        updated_at_ms INTEGER NOT NULL
    );",
    "CREATE TABLE conversations (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        created_at_ms INTEGER NOT NULL
    );
    CREATE TABLE messages (
        turn_id TEXT NOT NULL,
        conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
        sequence INTEGER NOT NULL,
        role TEXT NOT NULL CHECK(role IN ('user','assistant')),
        text TEXT NOT NULL,
        provider TEXT NOT NULL CHECK(provider IN ('chatgpt','copilot')),
        model TEXT NOT NULL,
        status TEXT NOT NULL CHECK(status IN ('running','completed','interrupted','failed')),
        error_code TEXT,
        PRIMARY KEY(turn_id, role),
        UNIQUE(conversation_id, sequence, role)
    );
    CREATE INDEX messages_conversation ON messages(conversation_id, sequence);",
    editor::V3,
    proposals::V4,
    proposal_apply::V5,
    proposal_rewrite::V6,
    chat::V7,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenReport {
    /// Where a corrupt `brn.sqlite` was moved, if one was found.
    pub corrupt_moved_to: Option<PathBuf>,
    /// The backup that replaced it, or `None` if a fresh database was created.
    pub restored_from: Option<PathBuf>,
    /// The backup made by this open.
    pub backup: PathBuf,
}

pub struct WorkStore {
    conn: Connection,
    dir: PathBuf,
    _owner_lock: Arc<File>,
}

/// What an existing database file turned out to be.
enum Checked {
    /// A BRN work database with a supported schema.
    Brn(Connection),
    /// A healthy but empty SQLite file, for example left by an interrupted first start.
    Empty(Connection),
    /// Damaged, or not SQLite at all.
    Corrupt,
    /// A database BRN must not touch (another application's, or a newer BRN's).
    Foreign(&'static str),
}

enum HeaderCheck {
    Continue,
    Corrupt,
    Foreign(&'static str),
}

impl WorkStore {
    pub fn open(data_dir: &Path) -> Result<(Self, OpenReport)> {
        if !data_dir.is_dir() {
            return Err(invalid("data directory must already exist"));
        }
        let lock = lock_dir(data_dir)?;
        crate::workspace_mode::refuse_legacy(data_dir)?;
        let db = data_dir.join(DB_NAME);
        let existing = if db.exists() {
            check_regular_single_link(&db)?;
            Some(check(&db)?)
        } else {
            None
        };
        let mut corrupt_moved_to = None;
        let mut restored_from = None;
        let mut conn = match existing {
            Some(Checked::Brn(conn)) => conn,
            Some(Checked::Empty(conn)) if backup::list(data_dir)?.is_empty() => conn,
            Some(Checked::Foreign(reason)) => return Err(invalid(reason)),
            // Missing, corrupt or empty with backups: restore, or start fresh.
            unusable => {
                drop(unusable);
                corrupt_moved_to = backup::move_aside(&db)?;
                let (conn, from) = backup::restore_newest(data_dir, &db)?;
                restored_from = from;
                conn
            }
        };
        configure(&conn)?;
        migrate(&mut conn)?;
        chat::reconcile(&mut conn)?;
        proposal_rewrite::reconcile(&mut conn)?;
        let backup = backup::create(data_dir, &conn)?;
        backup::prune(data_dir)?;
        Ok((
            Self {
                conn,
                dir: data_dir.to_path_buf(),
                _owner_lock: Arc::new(lock),
            },
            OpenReport {
                corrupt_moved_to,
                restored_from,
                backup,
            },
        ))
    }

    pub fn data_dir(&self) -> &Path {
        &self.dir
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn remove_setting(&mut self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    }
}

fn lock_dir(dir: &Path) -> Result<File> {
    let path = dir.join("brn.owner.lock");
    if path.exists() {
        check_regular_single_link(&path)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)?;
    acquire_owner_lock(|| file.try_lock(), Instant::now() + Duration::from_secs(1))?;
    Ok(file)
}

fn is_corruption(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::NotADatabase | rusqlite::ErrorCode::DatabaseCorrupt)
    )
}

fn header_identity(db: &Path) -> Result<HeaderCheck> {
    let mut header = Vec::with_capacity(100);
    File::open(db)?.take(100).read_to_end(&mut header)?;
    if header.is_empty() {
        return Ok(HeaderCheck::Continue);
    }
    if header.len() < 100 || &header[..16] != b"SQLite format 3\0" {
        return Ok(HeaderCheck::Corrupt);
    }
    let version = i32::from_be_bytes(header[60..64].try_into().unwrap()) as i64;
    let application = i32::from_be_bytes(header[68..72].try_into().unwrap()) as i64;
    if application != 0 && application != APPLICATION_ID {
        return Ok(HeaderCheck::Foreign(
            "brn.sqlite belongs to another application",
        ));
    }
    if application == APPLICATION_ID && version > MIGRATIONS.len() as i64 {
        return Ok(HeaderCheck::Foreign(
            "brn.sqlite has an unsupported (newer?) BRN schema",
        ));
    }
    Ok(HeaderCheck::Continue)
}

/// Checks header identity before opening an existing database with SQLite.
fn check(db: &Path) -> Result<Checked> {
    match header_identity(db)? {
        HeaderCheck::Continue => {}
        HeaderCheck::Corrupt => return Ok(Checked::Corrupt),
        HeaderCheck::Foreign(reason) => return Ok(Checked::Foreign(reason)),
    }
    // An unbranded foreign WAL database may still be checkpointed when refused.
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = match Connection::open_with_flags(db, flags) {
        Ok(conn) => conn,
        Err(e) if is_corruption(&e) => return Ok(Checked::Corrupt),
        Err(e) => return Err(e.into()),
    };
    match conn.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0)) {
        Ok(result) if result == "ok" => {}
        Ok(_) => return Ok(Checked::Corrupt),
        Err(e) if is_corruption(&e) => return Ok(Checked::Corrupt),
        Err(e) => return Err(e.into()),
    }
    let application: i64 = conn.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if application == APPLICATION_ID {
        if (1..=MIGRATIONS.len() as i64).contains(&version) {
            return Ok(Checked::Brn(conn));
        }
        return Ok(Checked::Foreign(
            "brn.sqlite has an unsupported (newer?) BRN schema",
        ));
    }
    let objects: i64 = conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
    if application == 0 && version == 0 && objects == 0 {
        return Ok(Checked::Empty(conn));
    }
    Ok(Checked::Foreign(
        "brn.sqlite belongs to another application",
    ))
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "trusted_schema", "OFF")?;
    let mode: String = conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(invalid("brn.sqlite could not enable WAL mode"));
    }
    conn.pragma_update(None, "synchronous", "FULL")?;
    #[cfg(target_os = "macos")]
    conn.pragma_update(None, "fullfsync", "ON")?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let tx = conn.transaction()?;
    for sql in &MIGRATIONS[version as usize..] {
        tx.execute_batch(sql)?;
    }
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", MIGRATIONS.len() as i64)?;
    tx.commit()?;
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
