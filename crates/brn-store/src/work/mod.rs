//! User-work database for the simple notes app (`brn.sqlite`). It is checked
//! on every open, restored from the newest backup when corrupt, and backed
//! up after every successful open. Notes themselves live in the vault.
pub mod action_completion;
pub mod actions;
mod backup;
pub mod chat;
pub mod editor;
mod edits;
pub mod findings;
pub mod inbox;
pub mod inbox_actions;
pub mod inbox_original_legacy;
pub mod inbox_original_operations;
pub mod inbox_processing;
pub mod inbox_removal;
pub mod inbox_review;
pub mod inbox_source;
pub mod inbox_visual;
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
    chat::V8,
    findings::V9,
    actions::V10,
    action_completion::V11,
    inbox::V12,
    inbox_processing::V13,
    inbox_actions::V14,
    inbox_original_operations::V15,
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
    /// Readable BRN operational work failed semantic validation. Refuse a main
    /// database without replacing it; a restore candidate may be skipped.
    Invalid(crate::Error),
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
            Some(Checked::Invalid(error)) => return Err(error),
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
        actions::check_all(&conn)?;
        action_completion::check_all(&conn)?;
        findings::check_all(&conn)?;
        inbox::check_all(&conn)?;
        inbox_actions::check_all(&conn)?;
        inbox_original_operations::check_all(&conn)?;
        inbox_processing::reconcile(&mut conn)?;
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
        inbox_original_operations::guard_setting(key)?;
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn remove_setting(&mut self, key: &str) -> Result<()> {
        inbox_original_operations::guard_setting(key)?;
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
    let identity = (|| -> rusqlite::Result<(i64, i64)> {
        Ok((
            conn.query_row("PRAGMA application_id", [], |r| r.get(0))?,
            conn.query_row("PRAGMA user_version", [], |r| r.get(0))?,
        ))
    })();
    let (application, version) = match identity {
        Ok(identity) => identity,
        Err(e) if is_corruption(&e) => return Ok(Checked::Corrupt),
        Err(e) => return Err(e.into()),
    };
    if application == APPLICATION_ID && (12..=MIGRATIONS.len() as i64).contains(&version) {
        let result = if version >= 15 {
            inbox_original_operations::check_all(&conn)
        } else {
            inbox_original_operations::check_legacy(&conn)
        };
        match result {
            Err(crate::Error::Sql(e)) if is_corruption(&e) => return Ok(Checked::Corrupt),
            Err(error) => return Ok(Checked::Invalid(error)),
            Ok(()) => {}
        }
    }
    if application == APPLICATION_ID && (14..=MIGRATIONS.len() as i64).contains(&version) {
        // A readable reserved analysis or bound turn cannot be replaced by
        // an older backup, even when its own table remains physically healthy.
        match inbox_actions::check_all(&conn) {
            Err(crate::Error::Sql(e)) if is_corruption(&e) => return Ok(Checked::Corrupt),
            Err(error) => return Ok(Checked::Invalid(error)),
            Ok(()) => {}
        }
    }
    if application == APPLICATION_ID && (13..=MIGRATIONS.len() as i64).contains(&version) {
        // Readable queue damage must not silently discard pending/review work.
        match inbox_processing::check_all(&conn) {
            Err(crate::Error::Sql(e)) if is_corruption(&e) => return Ok(Checked::Corrupt),
            Err(error) => return Ok(Checked::Invalid(error)),
            Ok(()) => {}
        }
    }
    if application == APPLICATION_ID && (12..=MIGRATIONS.len() as i64).contains(&version) {
        // Readable Inbox damage must not silently restore older operational
        // inventory or be copied into a new startup backup.
        match inbox::check_all(&conn) {
            Err(crate::Error::Sql(e)) if is_corruption(&e) => return Ok(Checked::Corrupt),
            Err(error) => return Ok(Checked::Invalid(error)),
            Ok(()) => {}
        }
    }
    if application == APPLICATION_ID && (11..=MIGRATIONS.len() as i64).contains(&version) {
        // Readable completion evidence must never be discarded by restoring an
        // older backup. Validate its owned shape, full bindings and current
        // Completed record before SQLite classifies domain damage as corruption.
        match action_completion::check_all(&conn) {
            Err(crate::Error::Sql(e)) if is_corruption(&e) => return Ok(Checked::Corrupt),
            Err(error) => return Ok(Checked::Invalid(error)),
            Ok(()) => {}
        }
    }
    if application == APPLICATION_ID && (10..=MIGRATIONS.len() as i64).contains(&version) {
        // Readable Action schema/record damage is semantic refusal. Checking
        // the owned shape and full rows first prevents quick_check from
        // replacing main work and keeps malformed restore candidates unusable.
        match actions::check_all(&conn) {
            Err(crate::Error::Sql(e)) if is_corruption(&e) => return Ok(Checked::Corrupt),
            Err(error) => return Ok(Checked::Invalid(error)),
            Ok(()) => {}
        }
    }
    if application == APPLICATION_ID && (9..=MIGRATIONS.len() as i64).contains(&version) {
        // SQLite quick_check also reports domain CHECK failures. Validate
        // readable Findings first so malformed state/hash bindings cannot
        // silently restore older operational work. Retain the V9 schema.
        match findings::check_all(&conn) {
            Err(crate::Error::Sql(e)) if is_corruption(&e) => return Ok(Checked::Corrupt),
            Err(error) => return Ok(Checked::Invalid(error)),
            Ok(()) => {}
        }
    }
    match conn.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0)) {
        Ok(result) if result == "ok" => {}
        Ok(_) => return Ok(Checked::Corrupt),
        Err(e) if is_corruption(&e) => return Ok(Checked::Corrupt),
        Err(e) => return Err(e.into()),
    }
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
