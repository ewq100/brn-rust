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
    validate_schema(conn, &[3])?;
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
        format!("Schema 3 needs an external pre-upgrade backup folder. Run: brn --data-dir {} upgrade --backup-directory ABSOLUTE_FOLDER", workspace.display())
    ))?;
    let folder = validate_backup_directory(workspace, folder)?;
    let destination = folder.join(format!(
        "brn-before-chat-schema-4-{}.sqlite3",
        uuid::Uuid::new_v4()
    ));
    let snapshot = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    crate::backup::StagedBackup::create(&snapshot, &destination)?.publish()?;
    tx.execute_batch(CHAT_SCHEMA)?;
    tx.pragma_update(None, "user_version", crate::store::SCHEMA_VERSION)?;
    tx.commit()?;
    validate_schema(conn, &[crate::store::SCHEMA_VERSION])
}
