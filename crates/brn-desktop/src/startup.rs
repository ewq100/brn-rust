//! Startup selection resumes the existing backup-first upgrade; Cancel does nothing.
use brn_threads_app::{Store, Workspace};
use std::path::{Path, PathBuf};

pub(crate) const BACKUP_PROMPT: &str =
    "Choose a backup folder. BRN saves a copy of your workspace there before upgrading.";

pub(crate) fn with_backup_folder(
    data: &Path,
    folder: Option<PathBuf>,
) -> Result<Option<Workspace>, String> {
    let Some(folder) = folder else {
        return Ok(None);
    };
    // Store::upgrade validates the chosen external directory under the writer
    // lock, publishes the staged snapshot, then upgrades in one transaction.
    Store::upgrade(data, &folder).map_err(|e| e.to_string())?;
    let mut workspace = Workspace::open(data).map_err(|e| e.to_string())?;
    workspace
        .choose_backup_directory(&folder)
        .map_err(|e| e.to_string())?;
    Ok(Some(workspace))
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_threads_app::{AppError, DATABASE_FILENAME, Error};
    use rusqlite::Connection;
    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("synthetic-workspace");
        drop(Workspace::open_without_default_credentials(&data).unwrap());
        Connection::open(data.join(DATABASE_FILENAME))
            .unwrap()
            .pragma_update(None, "user_version", 12)
            .unwrap();
        assert!(matches!(
            Workspace::open(&data),
            Err(AppError::Core(Error::UpgradeNeedsBackupFolder {
                from: 12,
                ..
            }))
        ));
        (root, data)
    }
    fn version(path: &Path) -> i64 {
        Connection::open(path)
            .unwrap()
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap()
    }
    #[test]
    fn chosen_folder_upgrades_and_remembers_it_after_publishing_old_schema_backup() {
        assert!(BACKUP_PROMPT.contains("before upgrading"));
        let (root, data) = fixture();
        let folder = root.path().join("backups");
        std::fs::create_dir(&folder).unwrap();
        let workspace = with_backup_folder(&data, Some(folder.clone()))
            .unwrap()
            .unwrap();
        assert_eq!(version(&data.join(DATABASE_FILENAME)), 13);
        assert_eq!(
            workspace.settings().unwrap().backup_directory.as_deref(),
            folder.to_str()
        );
        let backups: Vec<_> = std::fs::read_dir(&folder)
            .unwrap()
            .map(|p| p.unwrap().path())
            .collect();
        assert_eq!(backups.len(), 1);
        assert!(
            backups[0]
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("brn-before-schema-13-")
        );
        assert_eq!(version(&backups[0]), 12);
        assert!(Workspace::open(&data).is_ok());
    }
    #[test]
    fn cancelling_keeps_old_store_bytes_and_schema_unchanged() {
        let (_root, data) = fixture();
        let db = data.join(DATABASE_FILENAME);
        let bytes = std::fs::read(&db).unwrap();
        assert!(with_backup_folder(&data, None).unwrap().is_none());
        assert_eq!(std::fs::read(&db).unwrap(), bytes);
        assert_eq!(version(&db), 12);
    }
    #[test]
    fn invalid_backup_destination_keeps_old_store_unchanged() {
        let (_root, data) = fixture();
        let db = data.join(DATABASE_FILENAME);
        let bytes = std::fs::read(&db).unwrap();
        assert!(with_backup_folder(&data, Some(data.clone())).is_err());
        assert_eq!(std::fs::read(&db).unwrap(), bytes);
        assert_eq!(version(&db), 12);
    }
}
