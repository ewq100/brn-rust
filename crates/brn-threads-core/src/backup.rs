use crate::{Error, Result, Store, store::has_symlink_ancestor};
use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

impl Store {
    /// Publish a complete standalone snapshot at a fresh destination.
    pub fn backup(&self, destination: impl AsRef<Path>) -> Result<()> {
        let destination = destination.as_ref();
        let absolute = if destination.is_absolute() {
            destination.to_owned()
        } else {
            std::env::current_dir()?.join(destination)
        };
        let parent = absolute
            .parent()
            .ok_or_else(|| Error::Invalid("backup parent required".into()))?;
        self.validate_backup_directory(parent)?;
        StagedBackup::create(&self.conn, &absolute)?.publish()
    }
}

/// Own only the unique private directory, never the final destination. An
/// interrupted process may leave this recognizable stage, but not a partial
/// file under the requested backup name.
pub(crate) struct StagedBackup {
    directory: PathBuf,
    snapshot: PathBuf,
    destination: PathBuf,
}

impl StagedBackup {
    pub(crate) fn create(source: &Connection, destination: &Path) -> Result<Self> {
        // Resolve relative core paths without changing the process-wide cwd.
        let destination = if destination.is_absolute() {
            destination.to_owned()
        } else {
            std::env::current_dir()?.join(destination)
        };
        if destination.file_name().is_none() {
            return Err(Error::Invalid("backup filename required".into()));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| Error::Invalid("backup parent required".into()))?;
        if has_symlink_ancestor(parent)? {
            return Err(Error::UnsafeDirectory("symlink backup ancestor".into()));
        }
        // Refuse occupied paths cheaply, including dangling symlinks. The
        // exclusive publication below still handles a later competing writer.
        match fs::symlink_metadata(&destination) {
            Ok(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!(
                        "backup destination already exists: {}",
                        destination.display()
                    ),
                )
                .into());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let directory = parent.join(format!(".brn-backup-stage-{}", uuid::Uuid::new_v4()));
        let builder = fs::DirBuilder::new();
        #[cfg(unix)]
        let mut builder = builder;
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        // Exclusive creation establishes ownership before any SQLite work.
        builder.create(&directory)?;
        let stage = Self {
            snapshot: directory.join("snapshot.sqlite3"),
            directory,
            destination,
        };
        let mut target = Connection::open(&stage.snapshot)?;
        rusqlite::backup::Backup::new(source, &mut target)?.run_to_completion(
            64,
            Duration::from_millis(1),
            None,
        )?;
        // Online Backup copies the source WAL journal mode. Switch the target
        // back to a standalone database before closing all SQLite handles.
        target.pragma_update(None, "journal_mode", "DELETE")?;
        target.close().map_err(|(_, error)| error)?;
        let check = Connection::open_with_flags(
            &stage.snapshot,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        crate::store::validate_schema(&check, &[3, 4, crate::store::SCHEMA_VERSION])?;
        drop(check);
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&stage.snapshot)?
            .sync_all()?;
        Ok(stage)
    }

    pub(crate) fn publish(self) -> Result<()> {
        // hard_link creates the final directory entry exclusively, including
        // when another writer claims the name after staging has completed.
        fs::hard_link(&self.snapshot, &self.destination).map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!(
                    "cannot publish backup exclusively at {}: {error}",
                    self.destination.display()
                ),
            )
        })?;
        #[cfg(unix)]
        fs::File::open(self.destination.parent().expect("resolved backup parent"))?.sync_all()?;
        // Drop removes only staging, even if the post-publication flush fails.
        Ok(())
    }
}

impl Drop for StagedBackup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::validate_database;
    use crate::{ApplyOutcome, ChangeRequest, HostAuthority, Note, Put, Record, RecordData};

    fn root() -> tempfile::TempDir {
        tempfile::TempDir::new_in(fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap()
    }

    fn note(store: &mut Store, key: &str) -> Record {
        let operation = store.allocate_operation(key).unwrap();
        let request = ChangeRequest {
            reason: "synthetic backup witness".into(),
            inputs: vec![],
            writes: vec![Put {
                id: operation.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Note(Note::working(key, "snapshot content")),
            }],
        };
        let owner = HostAuthority::owner("synthetic backup owner");
        store.prepare_owner(&operation, &request, &owner).unwrap();
        let ApplyOutcome::Applied(receipt) = store.apply(&operation, &owner).unwrap() else {
            panic!("checked owner note must commit");
        };
        receipt.writes[0].after.clone()
    }

    #[test]
    fn stage_keeps_final_absent_until_complete_snapshot_is_published() {
        let root = root();
        let store = Store::open(root.path().join("source")).unwrap();
        let destination = root.path().join("backup.sqlite3");
        let stage = StagedBackup::create(&store.conn, &destination).unwrap();
        let staging = stage.directory.clone();
        validate_database(&stage.snapshot).unwrap();
        assert!(
            !destination.exists(),
            "the final backup name must stay absent while staged"
        );
        assert_eq!(stage.directory.parent(), destination.parent());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&staging).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        stage.publish().unwrap();
        assert!(!staging.exists());
        validate_database(&destination).unwrap();
        assert!(!destination.with_extension("sqlite3-wal").exists());
        assert!(!destination.with_extension("sqlite3-shm").exists());
    }

    #[test]
    fn occupied_destination_is_rejected_before_sqlite_copy() {
        let root = root();
        // Invalid SQLite identity makes an attempted snapshot fail validation.
        // An occupied destination must be refused before reaching that work.
        let source = Connection::open_in_memory().unwrap();
        source
            .execute_batch("CREATE TABLE foreign_data(id INTEGER)")
            .unwrap();
        let destination = root.path().join("backup.sqlite3");
        fs::write(&destination, b"existing backup bytes").unwrap();
        assert!(
            matches!(StagedBackup::create(&source, &destination), Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists)
        );
        assert_eq!(fs::read(&destination).unwrap(), b"existing backup bytes");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn publication_collision_preserves_new_destination_and_cleans_only_owned_stage() {
        let root = root();
        let store = Store::open(root.path().join("source")).unwrap();
        let destination = root.path().join("backup.sqlite3");
        let unrelated = root.path().join(".brn-backup-stage-not-owned");
        fs::create_dir(&unrelated).unwrap();
        fs::write(unrelated.join("keep"), b"unrelated").unwrap();
        let stage = StagedBackup::create(&store.conn, &destination).unwrap();
        let staging = stage.directory.clone();
        fs::write(&destination, b"claimed after staging").unwrap();
        assert!(stage.publish().is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"claimed after staging");
        assert!(!staging.exists());
        assert_eq!(fs::read(unrelated.join("keep")).unwrap(), b"unrelated");
    }

    #[cfg(unix)]
    #[test]
    fn parent_flush_failure_keeps_complete_published_snapshot_and_cleans_stage() {
        use std::os::unix::fs::PermissionsExt;
        let root = root();
        let store = Store::open(root.path().join("source")).unwrap();
        let parent = root.path().join("backups");
        fs::create_dir(&parent).unwrap();
        let destination = parent.join("backup.sqlite3");
        let stage = StagedBackup::create(&store.conn, &destination).unwrap();
        let staging = stage.directory.clone();
        // Write/execute permits publication and cleanup; missing read access
        // makes opening the parent for its flush fail for unprivileged users.
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o300)).unwrap();
        let flush_denied = fs::File::open(&parent).is_err();
        let result = stage.publish();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        if flush_denied {
            assert!(
                matches!(result, Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied)
            );
        } else {
            // Privileged Unix test runners can read mode-0300 directories.
            result.unwrap();
        }
        validate_database(&destination).unwrap();
        assert!(!staging.exists());
        assert_eq!(fs::read_dir(&parent).unwrap().count(), 1);
    }

    #[test]
    fn invalid_snapshot_and_abandoned_stage_clean_up_without_final_file() {
        let root = root();
        let foreign = Connection::open_in_memory().unwrap();
        foreign
            .execute_batch("CREATE TABLE foreign_data(id INTEGER)")
            .unwrap();
        let destination = root.path().join("backup.sqlite3");
        assert!(StagedBackup::create(&foreign, &destination).is_err());
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        let store = Store::open(root.path().join("source")).unwrap();
        let stage = StagedBackup::create(&store.conn, &destination).unwrap();
        let staging = stage.directory.clone();
        drop(stage);
        assert!(!staging.exists());
        assert!(!destination.exists());
    }

    #[test]
    fn wal_snapshot_remains_standalone_and_restores_original_recovery_after_later_writes() {
        let root = root();
        let mut store = Store::open(root.path().join("source")).unwrap();
        store
            .conn
            .pragma_update(None, "wal_autocheckpoint", 0)
            .unwrap();
        let first = note(&mut store, "before backup");
        let session = store.begin_edit(&first.id, first.version).unwrap();
        store
            .update_buffer(&session.id, 1, "recover snapshot draft")
            .unwrap();
        assert!(
            fs::metadata(store.directory().join("threads.sqlite3-wal"))
                .unwrap()
                .len()
                > 0
        );
        let records = store.records().unwrap();
        let buffers = store.recovery_buffers().unwrap();
        let destination = root.path().join("backup.sqlite3");
        let stage = StagedBackup::create(&store.conn, &destination).unwrap();
        assert_eq!(fs::read_dir(&stage.directory).unwrap().count(), 1);
        note(&mut store, "after staging");
        store
            .update_buffer(&session.id, 2, "later source draft")
            .unwrap();
        stage.publish().unwrap();
        let connection =
            Connection::open_with_flags(&destination, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
                .unwrap(),
            "delete"
        );
        connection.close().unwrap();
        let restored = Store::restore(&destination, root.path().join("restored")).unwrap();
        assert_eq!(restored.records().unwrap(), records);
        assert_eq!(restored.recovery_buffers().unwrap(), buffers);
        assert_eq!(restored.revision(&first.id, 1).unwrap(), Some(first));
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
        assert_eq!(store.records().unwrap().len(), records.len() + 1);
    }

    #[test]
    fn relative_core_destination_preserves_process_directory() {
        let cwd = std::env::current_dir().unwrap();
        let root = tempfile::TempDir::new_in(&cwd).unwrap();
        let store = Store::open(root.path().join("source")).unwrap();
        let destination = root
            .path()
            .strip_prefix(&cwd)
            .unwrap()
            .join("backup.sqlite3");
        store.backup(&destination).unwrap();
        assert_eq!(std::env::current_dir().unwrap(), cwd);
        validate_database(&root.path().join("backup.sqlite3")).unwrap();
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[test]
    fn missing_parent_and_directory_destination_preserve_source() {
        let root = root();
        let mut store = Store::open(root.path().join("source")).unwrap();
        let first = note(&mut store, "source remains intact");
        assert!(
            store
                .backup(root.path().join("missing/backup.sqlite3"))
                .is_err()
        );
        assert!(!root.path().join("missing").exists());
        let occupied = root.path().join("occupied");
        fs::create_dir(&occupied).unwrap();
        fs::write(occupied.join("keep"), b"directory content").unwrap();
        assert!(store.backup(&occupied).is_err());
        assert_eq!(
            fs::read(occupied.join("keep")).unwrap(),
            b"directory content"
        );
        assert_eq!(store.record(&first.id).unwrap(), Some(first));
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_ancestors_and_final_symlinks_are_refused_without_changes() {
        use std::os::unix::fs::symlink;
        let root = root();
        let mut store = Store::open(root.path().join("source")).unwrap();
        let first = note(&mut store, "unchanged source");
        let outside = root.path().join("outside");
        fs::create_dir(&outside).unwrap();
        let alias = root.path().join("alias");
        symlink(&outside, &alias).unwrap();
        assert!(matches!(
            store.backup(alias.join("backup.sqlite3")),
            Err(Error::UnsafeDirectory(_))
        ));
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
        let victim = outside.join("victim");
        fs::write(&victim, b"victim content").unwrap();
        let destination = root.path().join("backup.sqlite3");
        symlink(&victim, &destination).unwrap();
        assert!(store.backup(&destination).is_err());
        assert!(
            fs::symlink_metadata(&destination)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&victim).unwrap(), b"victim content");
        fs::remove_file(&destination).unwrap();
        symlink(outside.join("missing"), &destination).unwrap();
        assert!(store.backup(&destination).is_err());
        assert!(!outside.join("missing").exists());
        assert_eq!(store.record(&first.id).unwrap(), Some(first));
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 4);
    }
}
