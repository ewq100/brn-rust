//! Adopted format upgrades require a verified external snapshot before mutation.
use crate::store::{has_symlink_ancestor, validate_backup_directory, validate_schema};
use crate::*;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior};
use std::path::Path;

pub(crate) const CHAT_SCHEMA: &str = "
DROP INDEX IF EXISTS source_locator;
CREATE UNIQUE INDEX source_locator ON records(
    COALESCE(json_extract(body,'$.data.Source.chat'),''),
    json_extract(body,'$.data.Source.locator'))
    WHERE json_type(body,'$.data.Source') IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS one_chat_control ON records((1))
    WHERE json_type(body,'$.data.ChatControl') IS NOT NULL;
CREATE TABLE IF NOT EXISTS chat_drafts(
    destination TEXT PRIMARY KEY, generation INTEGER NOT NULL, text TEXT NOT NULL);
";

pub(crate) const PROJECT_SCHEMA: &str = "CREATE INDEX IF NOT EXISTS project_records ON records(id) WHERE json_type(body,'$.data.Project') IS NOT NULL;
CREATE INDEX IF NOT EXISTS context_link_from ON records(json_extract(body,'$.data.Link.from')) WHERE json_type(body,'$.data.Link') IS NOT NULL;
CREATE INDEX IF NOT EXISTS context_link_to ON records(json_extract(body,'$.data.Link.to')) WHERE json_type(body,'$.data.Link') IS NOT NULL;
CREATE INDEX IF NOT EXISTS context_asset_source ON records(json_extract(body,'$.data.Asset.source')) WHERE json_type(body,'$.data.Asset') IS NOT NULL;
CREATE INDEX IF NOT EXISTS context_import_source ON records(json_extract(body,'$.data.Note.import.source')) WHERE json_type(body,'$.data.Note') IS NOT NULL;
";

/// Schema 6: caller-bound edit sessions (hashes only), the workspace
/// profile (purpose, default `owner`) and the actor beside receipts.
pub(crate) const CALLER_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS edit_session_bindings(session TEXT PRIMARY KEY REFERENCES edit_sessions(id), token_hash TEXT NOT NULL UNIQUE);
CREATE TABLE IF NOT EXISTS workspace_profile(id INTEGER PRIMARY KEY CHECK(id=1), purpose TEXT NOT NULL CHECK(purpose IN ('owner','test')), changed_at TEXT);
INSERT OR IGNORE INTO workspace_profile(id,purpose) VALUES(1,'owner');
CREATE TABLE IF NOT EXISTS receipt_actors(operation TEXT PRIMARY KEY REFERENCES receipts(operation), actor TEXT NOT NULL);
";

/// A setup keeps its identity through archive/Undo; absence and general scope
/// have one normalized SQL key. Nonempty scope IDs are checked by the writer.
pub(crate) const WRITING_SCHEMA: &str = "CREATE UNIQUE INDEX IF NOT EXISTS one_writing_setup ON records(json_extract(body,'$.data.WritingSetup.note'), COALESCE(json_extract(body,'$.data.WritingSetup.chat'),'')) WHERE json_type(body,'$.data.WritingSetup') IS NOT NULL;";

/// Applies the schema steps a version-`from` database is missing, ending at
/// the current schema. The caller owns the transaction.
pub(crate) fn upgrade_steps(tx: &Connection, from: i64) -> Result<()> {
    if from < 5 {
        tx.execute_batch(CHAT_SCHEMA)?;
        tx.execute_batch(PROJECT_SCHEMA)?;
    }
    if from < 6 {
        tx.execute_batch(CALLER_SCHEMA)?;
    }
    if from < 7 {
        tx.execute_batch(WRITING_SCHEMA)?;
    }
    tx.pragma_update(None, "user_version", crate::store::SCHEMA_VERSION)?;
    Ok(())
}

impl Store {
    pub fn upgrade(directory: &Path, backup_directory: &Path) -> Result<()> {
        if !directory.is_absolute()
            || has_symlink_ancestor(directory)?
            || std::fs::read_to_string(directory.join(DATA_MARKER))? != "BRN Threads SQLite v1\n"
        {
            return Err(Error::UnsafeDirectory(
                "validated explicit Threads directory required".into(),
            ));
        }
        let database = directory.join(DATABASE_FILENAME);
        let conn = Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version == crate::store::SCHEMA_VERSION {
            return validate_schema(&conn, &[crate::store::SCHEMA_VERSION]);
        }
        upgrade_connection(&conn, &database, Some(backup_directory))
    }
}

pub(crate) fn upgrade_connection(
    conn: &Connection,
    database: &Path,
    chosen: Option<&Path>,
) -> Result<()> {
    validate_schema(conn, &[3, 4, 5, 6, 7, 8, 9])?;
    let from: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let workspace = database
        .parent()
        .ok_or_else(|| Error::UnsafeDirectory("workspace parent missing".into()))?;
    // A reserved writer blocks competing v3 committers throughout the snapshot.
    // Backup uses a separate read connection, avoiding SQLite's same-writer lock.
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    let busy: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM records WHERE json_extract(body,'$.archived')=0 AND json_extract(body,'$.data.Run.state')='Working') OR EXISTS(SELECT 1 FROM edit_sessions WHERE closed=0)", [], |r| r.get(0))?;
    if busy {
        return Err(Error::UnsafeDirectory(
            "Stop work and save/discard edits in the previous app before upgrading this workspace"
                .into(),
        ));
    }
    let remembered: Option<String> = tx.query_row(
        "SELECT json_extract(body,'$.data.Settings.backup_directory') FROM records WHERE json_type(body,'$.data.Settings') IS NOT NULL AND json_extract(body,'$.archived')=0 LIMIT 1", [], |r| r.get(0)
    ).optional()?.flatten();
    let folder = chosen.or_else(|| remembered.as_deref().map(Path::new)).ok_or_else(|| Error::UnsafeDirectory(
        if from >= 5 {
            format!("Schema {from} needs an external pre-upgrade backup folder. Run: brn --data-dir {} upgrade --backup-directory ABSOLUTE_FOLDER", workspace.display())
        } else {
            format!("Schema 3/4 needs an external pre-upgrade backup folder. Run: brn --data-dir {} upgrade --backup-directory ABSOLUTE_FOLDER", workspace.display())
        }
    ))?;
    let folder = validate_backup_directory(workspace, folder)?;
    let destination = folder.join(format!(
        "brn-before-schema-{}-{}.sqlite3",
        crate::store::SCHEMA_VERSION,
        uuid::Uuid::new_v4()
    ));
    let snapshot = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    crate::backup::StagedBackup::create(&snapshot, &destination)?.publish()?;
    upgrade_steps(&tx, from)?;
    tx.commit()?;
    validate_schema(conn, &[crate::store::SCHEMA_VERSION])
}
