use crate::{
    DATA_MARKER, DATABASE_FILENAME, Error, Result, Store,
    store::{has_symlink_ancestor, validate_database, validate_schema},
};
use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

impl Store {
    /// Restore a validated backup into a fresh directory, publishing only the
    /// complete workspace. An interrupted copy leaves no ready destination.
    pub fn restore(backup: impl AsRef<Path>, fresh_directory: impl AsRef<Path>) -> Result<Self> {
        StagedRestore::create(backup.as_ref(), fresh_directory.as_ref())?.publish()
    }
}

/// Own only the private stage. After rename, this path is absent, so cleanup
/// never removes a complete published workspace when flush or reopen fails.
struct StagedRestore {
    directory: PathBuf,
    destination: PathBuf,
}
impl StagedRestore {
    fn create(backup: &Path, destination: &Path) -> Result<Self> {
        let source =
            Connection::open_with_flags(backup, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        validate_schema(&source, &[3, 4, 5])?;
        Self::prepare(destination)?.copy_from(&source)
    }

    fn prepare(destination: &Path) -> Result<Self> {
        if !destination.is_absolute() || destination.file_name().is_none() {
            return Err(Error::UnsafeDirectory(
                "an absolute fresh restore directory is required".into(),
            ));
        }
        if has_symlink_ancestor(destination)? {
            return Err(Error::UnsafeDirectory(
                "symlink restore directory or ancestor".into(),
            ));
        }
        require_fresh_destination(destination)?;
        let parent = destination.parent().expect("absolute restore parent");
        // Preserve the existing API's nested nonexistent directory support.
        fs::create_dir_all(parent)?;
        if has_symlink_ancestor(parent)? {
            return Err(Error::UnsafeDirectory("symlink restore ancestor".into()));
        }
        let directory = parent.join(format!(".brn-restore-stage-{}", uuid::Uuid::new_v4()));
        let builder = fs::DirBuilder::new();
        #[cfg(unix)]
        let mut builder = builder;
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory)?;
        Ok(Self {
            directory,
            destination: destination.to_owned(),
        })
    }

    fn copy_from(self, source: &Connection) -> Result<Self> {
        let mut store = Store::open(&self.directory)?;
        rusqlite::backup::Backup::new(source, &mut store.conn)?.run_to_completion(
            64,
            Duration::from_millis(1),
            None,
        )?;
        if source.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))?
            < crate::store::SCHEMA_VERSION
        {
            // The read-only, validated original backup is the pre-upgrade snapshot.
            // Only this unpublished copy changes; the source remains untouched.
            let tx = store
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute_batch(crate::upgrade::CHAT_SCHEMA)?;
            tx.execute_batch(crate::upgrade::PROJECT_SCHEMA)?;
            tx.pragma_update(None, "user_version", crate::store::SCHEMA_VERSION)?;
            tx.commit()?;
        }
        // Online Backup inherits WAL mode. Close a standalone DELETE database
        // before validation/flush and before moving the entire directory.
        store.conn.pragma_update(None, "journal_mode", "DELETE")?;
        store.conn.close().map_err(|(_, error)| error)?;
        validate_database(&self.directory.join(DATABASE_FILENAME))?;
        for name in [DATABASE_FILENAME, DATA_MARKER] {
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(self.directory.join(name))?
                .sync_all()?;
        }
        #[cfg(unix)]
        fs::File::open(&self.directory)?.sync_all()?;
        Ok(self)
    }

    fn publish(self) -> Result<Store> {
        self.publish_then(|destination| {
            #[cfg(unix)]
            fs::File::open(destination.parent().expect("absolute restore parent"))?.sync_all()?;
            Store::open(destination)
        })
    }

    fn publish_then(self, after_publish: impl FnOnce(&Path) -> Result<Store>) -> Result<Store> {
        if has_symlink_ancestor(&self.destination)? {
            return Err(Error::UnsafeDirectory(
                "symlink restore directory or ancestor".into(),
            ));
        }
        require_fresh_destination(&self.destination)?;
        self.rename_directory()?;
        after_publish(&self.destination)
    }

    fn rename_directory(&self) -> Result<()> {
        // Unix replaces an empty destination directory atomically and refuses
        // an occupied one. Try the same rename first on every platform.
        match fs::rename(&self.directory, &self.destination) {
            Ok(()) => Ok(()),
            Err(error) => {
                #[cfg(windows)]
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::PermissionDenied
                ) && self.directory.is_dir()
                    && self.destination.is_dir()
                {
                    // Some Windows filesystems refuse replacing an empty
                    // directory. remove_dir refuses a newly occupied one too.
                    require_fresh_destination(&self.destination)?;
                    fs::remove_dir(&self.destination)?;
                    return fs::rename(&self.directory, &self.destination).map_err(|error| {
                        // Preserve the caller's initially empty directory on
                        // failed fallback without replacing a competing entry.
                        let _ = fs::create_dir(&self.destination);
                        error.into()
                    });
                }
                Err(error.into())
            }
        }
    }
}
impl Drop for StagedRestore {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn require_fresh_destination(destination: &Path) -> Result<()> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) => {
            if !metadata.is_dir()
                || metadata.file_type().is_symlink()
                || fs::read_dir(destination)?.next().is_some()
            {
                return Err(Error::UnsafeDirectory(
                    "restore requires an empty fresh directory".into(),
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staging_never_exposes_ready_destination() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let source = Store::open(root.path().join("source")).unwrap();
        let backup = root.path().join("backup.sqlite3");
        source.backup(&backup).unwrap();
        let destination = root.path().join("restored");
        let stage = StagedRestore::create(&backup, &destination).unwrap();
        let staging = stage.directory.clone();
        assert_eq!(staging.parent(), destination.parent());
        assert_eq!(fs::read_dir(&staging).unwrap().count(), 2);
        validate_database(&staging.join(DATABASE_FILENAME)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&staging).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        assert!(
            !destination.exists(),
            "restore must publish only a complete staged directory"
        );
        stage.publish().unwrap();
        assert!(!staging.exists());
    }

    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let source = Store::open(root.path().join("source")).unwrap();
        let backup = root.path().join("backup.sqlite3");
        source.backup(&backup).unwrap();
        (root, backup)
    }

    #[test]
    fn empty_destination_and_nested_missing_parents_restore_and_preserve_source_bytes() {
        let (root, backup) = fixture();
        let bytes = fs::read(&backup).unwrap();
        for destination in [
            root.path().join("empty"),
            root.path().join("nested/missing/restored"),
        ] {
            if destination.file_name().unwrap() == "empty" {
                fs::create_dir(&destination).unwrap();
            }
            let stage = StagedRestore::create(&backup, &destination).unwrap();
            assert!(!destination.join(DATA_MARKER).exists());
            let store = stage.publish().unwrap();
            assert!(store.records().unwrap().is_empty());
            drop(store);
            Store::open(&destination).unwrap();
            assert_eq!(fs::read(&backup).unwrap(), bytes);
        }
    }

    #[test]
    fn collision_and_rename_refuse_occupied_destination_and_clean_only_owned_stage() {
        let (root, backup) = fixture();
        let unrelated = root.path().join(".brn-restore-stage-not-owned");
        fs::create_dir(&unrelated).unwrap();
        fs::write(unrelated.join("keep"), b"unrelated").unwrap();
        for after_preflight in [false, true] {
            let destination = root.path().join(format!("occupied-{after_preflight}"));
            fs::create_dir(&destination).unwrap();
            let stage = StagedRestore::create(&backup, &destination).unwrap();
            let staging = stage.directory.clone();
            // Exercise actual production rename after the freshness check too.
            require_fresh_destination(&destination).unwrap();
            fs::write(destination.join("keep"), b"new occupant").unwrap();
            if after_preflight {
                assert!(stage.rename_directory().is_err());
                drop(stage);
            } else {
                assert!(stage.publish().is_err());
            }
            assert_eq!(fs::read(destination.join("keep")).unwrap(), b"new occupant");
            assert!(!staging.exists());
        }
        assert_eq!(fs::read(unrelated.join("keep")).unwrap(), b"unrelated");
    }

    #[test]
    fn abandoned_stage_and_sqlite_copy_failure_clean_up_without_publication() {
        let (root, backup) = fixture();
        let destination = root.path().join("restore");
        let stage = StagedRestore::create(&backup, &destination).unwrap();
        let staging = stage.directory.clone();
        drop(stage);
        assert!(!staging.exists());
        assert!(!destination.exists());
        // Opening succeeds, but the SQLite backup call fails on corrupt input.
        let corrupt = root.path().join("corrupt.sqlite3");
        fs::write(&corrupt, b"not a database").unwrap();
        let source = Connection::open(&corrupt).unwrap();
        let stage = StagedRestore::prepare(&destination).unwrap();
        let staging = stage.directory.clone();
        assert!(matches!(stage.copy_from(&source), Err(Error::Sqlite(_))));
        assert!(!staging.exists());
        assert!(!destination.exists());
        assert_eq!(fs::read(&corrupt).unwrap(), b"not a database");
    }

    #[test]
    fn publication_failure_cleans_owned_stage_and_preserves_empty_destination() {
        let (root, backup) = fixture();
        let destination = root.path().join("empty");
        fs::create_dir(&destination).unwrap();
        let stage = StagedRestore::create(&backup, &destination).unwrap();
        let staging = stage.directory.clone();
        // Deterministic filesystem failure independent of privilege or timing.
        fs::remove_dir_all(&staging).unwrap();
        assert!(stage.publish().is_err());
        assert!(destination.is_dir());
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
        assert!(!staging.exists());
    }

    #[test]
    fn post_publication_flush_or_reopen_failure_preserves_complete_destination() {
        let (root, backup) = fixture();
        for reopen in [false, true] {
            let destination = root.path().join(format!("restored-{reopen}"));
            let stage = StagedRestore::create(&backup, &destination).unwrap();
            let staging = stage.directory.clone();
            let result = stage.publish_then(|_| {
                if reopen {
                    Err(Error::Sqlite(rusqlite::Error::InvalidPath(
                        destination.clone(),
                    )))
                } else {
                    Err(std::io::Error::other("injected parent flush failure").into())
                }
            });
            assert!(result.is_err());
            assert!(!staging.exists());
            validate_database(&destination.join(DATABASE_FILENAME)).unwrap();
            assert!(destination.join(DATA_MARKER).is_file());
            Store::open(&destination).unwrap();
        }
    }

    #[test]
    fn invalid_corrupt_incompatible_sources_leave_destination_and_source_unchanged() {
        let (root, backup) = fixture();
        let corrupt = root.path().join("corrupt.sqlite3");
        fs::write(&corrupt, b"corrupt source bytes").unwrap();
        let incompatible = root.path().join("incompatible.sqlite3");
        fs::copy(&backup, &incompatible).unwrap();
        let conn = Connection::open(&incompatible).unwrap();
        conn.pragma_update(None, "user_version", 9000).unwrap();
        conn.close().unwrap();
        let foreign = root.path().join("foreign.sqlite3");
        let conn = Connection::open(&foreign).unwrap();
        conn.execute_batch("CREATE TABLE unrelated(id INTEGER)")
            .unwrap();
        conn.close().unwrap();
        let empty = root.path().join("zero.sqlite3");
        fs::write(&empty, []).unwrap();
        for source in [&corrupt, &incompatible, &foreign, &empty] {
            let bytes = fs::read(source).unwrap();
            let destination = root.path().join("missing/parents/destination");
            assert!(Store::restore(source, &destination).is_err());
            assert!(!root.path().join("missing").exists());
            assert_eq!(fs::read(source).unwrap(), bytes);
        }
        assert!(
            Store::restore(
                root.path().join("absent.sqlite3"),
                root.path().join("restore")
            )
            .is_err()
        );
        assert!(!root.path().join("absent.sqlite3").exists());
        assert!(!fs::read_dir(root.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".brn-restore-stage-")
        }));
    }

    #[test]
    fn relative_source_works_absolute_destination_is_required_without_cwd_change() {
        let cwd = std::env::current_dir().unwrap();
        let root = tempfile::tempdir_in(&cwd).unwrap();
        let source = Store::open(root.path().join("source")).unwrap();
        let backup = root.path().join("backup.sqlite3");
        source.backup(&backup).unwrap();
        let relative = backup.strip_prefix(&cwd).unwrap();
        Store::restore(relative, root.path().join("restore")).unwrap();
        let relative_destination = root.path().strip_prefix(&cwd).unwrap().join("refused");
        assert!(matches!(
            Store::restore(relative, &relative_destination),
            Err(Error::UnsafeDirectory(_))
        ));
        assert!(!root.path().join("refused").exists());
        assert_eq!(std::env::current_dir().unwrap(), cwd);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_destinations_and_ancestors_are_refused_before_and_after_staging() {
        use std::os::unix::fs::symlink;
        let (root, backup) = fixture();
        let outside = root.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep"), b"outside content").unwrap();
        let alias = root.path().join("alias");
        symlink(&outside, &alias).unwrap();
        assert!(Store::restore(&backup, &alias).is_err());
        assert!(Store::restore(&backup, alias.join("nested/restored")).is_err());
        let dangling = root.path().join("dangling");
        symlink(root.path().join("absent"), &dangling).unwrap();
        assert!(Store::restore(&backup, &dangling).is_err());
        let destination = root.path().join("restored");
        let stage = StagedRestore::create(&backup, &destination).unwrap();
        let staging = stage.directory.clone();
        symlink(&outside, &destination).unwrap();
        assert!(stage.publish().is_err());
        assert!(!staging.exists());
        assert!(
            fs::symlink_metadata(&destination)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(outside.join("keep")).unwrap(), b"outside content");
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
        assert!(!root.path().join("absent").exists());
    }
}
