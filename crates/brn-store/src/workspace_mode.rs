use crate::{Error, Result};
use std::path::Path;

fn present(path: &Path) -> Result<bool> {
    match path.symlink_metadata() {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn refuse_markers(dir: &Path, database: &str) -> Result<()> {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        if present(&dir.join(format!("{database}{suffix}")))? {
            return Err(conflict());
        }
    }
    Ok(())
}

fn conflict() -> Error {
    Error::WorkspaceModeConflict(
        "data directory belongs to a different workspace mode; use a new data folder".into(),
    )
}

// Both callers hold brn.owner.lock before checking and keep it through SQLite open.
pub(crate) fn refuse_legacy(dir: &Path) -> Result<()> {
    refuse_markers(dir, "brn.sqlite3")
}

pub(crate) fn refuse_simple(dir: &Path) -> Result<()> {
    refuse_markers(dir, "brn.sqlite")?;
    let backups = dir.join("backups");
    if present(&backups)? {
        for entry in std::fs::read_dir(backups)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let name = ["-wal", "-shm", "-journal"]
                .iter()
                .find_map(|suffix| name.strip_suffix(suffix))
                .unwrap_or(name);
            if name
                .strip_prefix("brn-")
                .and_then(|name| name.strip_suffix(".sqlite"))
                .is_some_and(|stamp| !stamp.is_empty() && stamp.bytes().all(|b| b.is_ascii_digit()))
            {
                return Err(conflict());
            }
        }
    }
    Ok(())
}
