use super::{Checked, check, now_ms};
use crate::{Result, invalid};
use rusqlite::Connection;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

const KEEP: usize = 5;

fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// The number in a backup file name `brn-<number>.sqlite`.
fn stamp(path: &Path) -> Option<u64> {
    path.file_name()?
        .to_str()?
        .strip_prefix("brn-")?
        .strip_suffix(".sqlite")?
        .parse()
        .ok()
}

/// Backups, oldest first.
pub(super) fn list(data_dir: &Path) -> Result<Vec<PathBuf>> {
    let dir = backups_dir(data_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut backups = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(number) = stamp(&path)
            && entry.file_type()?.is_file()
        {
            backups.push((number, path));
        }
    }
    backups.sort();
    Ok(backups.into_iter().map(|(_, path)| path).collect())
}

/// Writes a consistent copy with SQLite's online backup API. The number is the
/// current time in milliseconds, but always above every existing backup, so the
/// new copy is the newest even if the clock went backwards.
pub(super) fn create(data_dir: &Path, conn: &Connection) -> Result<PathBuf> {
    std::fs::create_dir_all(backups_dir(data_dir))?;
    let after_last = list(data_dir)?
        .last()
        .and_then(|path| stamp(path))
        .map_or(0, |last| last + 1);
    let path = backups_dir(data_dir).join(format!("brn-{:013}.sqlite", now_ms().max(after_last)));
    conn.backup("main", &path, None)?;
    Ok(path)
}

pub(super) fn prune(data_dir: &Path) -> Result<()> {
    let backups = list(data_dir)?;
    if backups.len() > KEEP {
        for old in &backups[..backups.len() - KEEP] {
            std::fs::remove_file(old)?;
        }
    }
    Ok(())
}

/// Renames a database and its WAL/SHM files out of the way. Returns the new
/// name of the database file, or `None` if only leftover WAL/SHM files existed.
pub(super) fn move_aside(db: &Path) -> Result<Option<PathBuf>> {
    let parent = db
        .parent()
        .ok_or_else(|| invalid("database path has no parent folder"))?;
    let mut ms = now_ms();
    let target = loop {
        let candidate = parent.join(format!("brn.sqlite.corrupt-{ms:013}"));
        if !candidate.exists() {
            break candidate;
        }
        ms += 1;
    };
    let moved = if db.exists() {
        std::fs::rename(db, &target)?;
        Some(target.clone())
    } else {
        None
    };
    for suffix in ["-wal", "-shm"] {
        let side = with_suffix(db, suffix);
        if side.exists() {
            std::fs::rename(&side, with_suffix(&target, suffix))?;
        }
    }
    Ok(moved)
}

fn remove_with_sidecars(db: &Path) -> Result<()> {
    for path in [
        db.to_path_buf(),
        with_suffix(db, "-wal"),
        with_suffix(db, "-shm"),
    ] {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// Copies the newest usable BRN backup to `db`, skipping damaged, empty or
/// foreign files. Falls back to a fresh, empty database.
pub(super) fn restore_newest(data_dir: &Path, db: &Path) -> Result<(Connection, Option<PathBuf>)> {
    for backup in list(data_dir)?.into_iter().rev() {
        std::fs::copy(&backup, db)?;
        if let Checked::Brn(conn) = check(db)? {
            return Ok((conn, Some(backup)));
        }
        remove_with_sidecars(db)?;
    }
    Ok((Connection::open(db)?, None))
}
