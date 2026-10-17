use crate::{
    Error, Result, Store,
    stage::{self, StageDir, StageKind},
    store::has_symlink_ancestor,
};
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

/// Own only the unique private stage, never the final destination. Dropping
/// it removes the stage on every error path. A killed process may leave the
/// stage, but not a partial file under the requested backup name; the next
/// backup into the same folder removes that abandoned stage.
pub(crate) struct StagedBackup {
    // Held for its Drop, which removes the stage.
    _stage: StageDir,
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
        // Remove what killed backups left in this folder before staging ours.
        stage::sweep_abandoned(parent, StageKind::Backup, &destination);
        // Exclusive creation establishes ownership before any SQLite work.
        let stage = StageDir::create(parent, StageKind::Backup)?;
        let stage = Self {
            snapshot: stage.path().join("snapshot.sqlite3"),
            _stage: stage,
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
        crate::store::validate_schema(&check, &[3, 4, 5, 6, 7, crate::store::SCHEMA_VERSION])?;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use crate::stage::tree;
    use crate::stage::{STAGE_MARKER, stages};
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
        let staging = stage._stage.path().to_owned();
        validate_database(&stage.snapshot).unwrap();
        assert!(
            !destination.exists(),
            "the final backup name must stay absent while staged"
        );
        assert_eq!(staging.parent(), destination.parent());
        assert!(staging.join(STAGE_MARKER).is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Windows locks are mandatory, so only Unix can read a held marker.
            assert_eq!(
                fs::read(staging.join(STAGE_MARKER)).unwrap(),
                StageKind::Backup.marker().as_bytes()
            );
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
        let staging = stage._stage.path().to_owned();
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
        let staging = stage._stage.path().to_owned();
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
        // Validation fails after the stage and its snapshot were created.
        assert!(StagedBackup::create(&foreign, &destination).is_err());
        assert!(!destination.exists());
        assert_eq!(
            stages(root.path(), StageKind::Backup),
            Vec::<PathBuf>::new()
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        let store = Store::open(root.path().join("source")).unwrap();
        let stage = StagedBackup::create(&store.conn, &destination).unwrap();
        let staging = stage._stage.path().to_owned();
        drop(stage);
        assert!(!staging.exists());
        assert!(!destination.exists());
        assert_eq!(
            stages(root.path(), StageKind::Backup),
            Vec::<PathBuf>::new()
        );
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
        // The snapshot and the stage marker.
        assert_eq!(fs::read_dir(stage._stage.path()).unwrap().count(), 2);
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

    /// A source store and an empty backup folder outside it.
    #[cfg(unix)]
    fn backup_folder() -> (tempfile::TempDir, Store, PathBuf) {
        let root = root();
        let store = Store::open(root.path().join("source")).unwrap();
        let folder = root.path().join("backups");
        fs::create_dir(&folder).unwrap();
        (root, store, folder)
    }

    #[cfg(unix)]
    #[test]
    fn a_killed_backups_stage_is_removed_by_the_next_backup_to_that_folder() {
        let (_root, store, folder) = backup_folder();
        let empty = StageDir::create(&folder, StageKind::Backup)
            .unwrap()
            .abandon();
        // Killed after its snapshot was complete but before publication.
        let StagedBackup { _stage: full, .. } =
            StagedBackup::create(&store.conn, &folder.join("killed.sqlite3")).unwrap();
        let full = full.abandon();
        assert!(full.join("snapshot.sqlite3").is_file());
        let unrelated = folder.join(".brn-backup-stage-not-owned");
        fs::create_dir(&unrelated).unwrap();
        fs::write(unrelated.join("keep"), b"unrelated").unwrap();
        let destination = folder.join("backup.sqlite3");
        store.backup(&destination).unwrap();
        assert!(!empty.exists(), "leftover stage must be removed");
        assert!(!full.exists(), "leftover stage must be removed");
        validate_database(&destination).unwrap();
        assert!(!folder.join("killed.sqlite3").exists());
        assert_eq!(fs::read(unrelated.join("keep")).unwrap(), b"unrelated");
        assert_eq!(stages(&folder, StageKind::Backup), vec![unrelated]);
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 2);
    }

    /// Child half of the SIGKILL test: holds a staged backup until killed.
    #[cfg(unix)]
    #[test]
    #[ignore = "runs only as the child of the SIGKILL stage test"]
    fn stage_holder_child() {
        let Some(folder) = std::env::var_os("BRN_STAGE_CHILD_PARENT") else {
            return;
        };
        let folder = PathBuf::from(folder);
        let store = Store::open(folder.join("child-source")).unwrap();
        let _stage = StagedBackup::create(&store.conn, &folder.join("never.sqlite3")).unwrap();
        println!("BRN_STAGE_READY");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        std::thread::sleep(Duration::from_secs(60));
    }

    #[cfg(unix)]
    #[test]
    fn a_sigkilled_backup_process_leaves_a_stage_the_next_backup_removes() {
        use std::io::BufRead;
        let (_root, store, folder) = backup_folder();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "backup::tests::stage_holder_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("BRN_STAGE_CHILD_PARENT", &folder)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut lines = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
        loop {
            let line = lines
                .next()
                .expect("the child must report its stage before exiting")
                .unwrap();
            if line.contains("BRN_STAGE_READY") {
                break;
            }
        }
        child.kill().unwrap(); // SIGKILL
        child.wait().unwrap();
        assert!(!folder.join("never.sqlite3").exists());
        let left = stages(&folder, StageKind::Backup);
        assert_eq!(left.len(), 1, "the killed process leaves its stage");
        let marker = fs::File::open(left[0].join(STAGE_MARKER)).unwrap();
        marker.try_lock().expect("the OS released the killed lock");
        drop(marker);
        let destination = folder.join("backup.sqlite3");
        store.backup(&destination).unwrap();
        assert_eq!(stages(&folder, StageKind::Backup), Vec::<PathBuf>::new());
        validate_database(&destination).unwrap();
        assert!(
            folder
                .join("child-source")
                .join(crate::DATA_MARKER)
                .is_file()
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_live_stage_survives_another_backup() {
        let (_root, store, folder) = backup_folder();
        let live = StageDir::create(&folder, StageKind::Backup).unwrap();
        fs::write(live.path().join("snapshot.sqlite3"), b"in progress").unwrap();
        let before = tree(live.path());
        store.backup(folder.join("backup.sqlite3")).unwrap();
        assert!(live.path().is_dir(), "a held stage is never removed");
        assert_eq!(tree(live.path()), before, "a held stage is never touched");
        let path = live.path().to_owned();
        drop(live);
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn stage_named_entries_that_are_not_brn_stages_are_never_followed_or_removed() {
        use std::os::unix::fs::symlink;
        let (root, store, folder) = backup_folder();
        let named = || folder.join(format!(".brn-backup-stage-{}", uuid::Uuid::new_v4()));
        // Outside the folder: a free, valid-looking backup stage marker.
        let outside = root.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join(STAGE_MARKER), StageKind::Backup.marker()).unwrap();
        fs::write(outside.join("keep"), b"outside content").unwrap();
        symlink(&outside, named()).unwrap();
        let no_marker = named();
        fs::create_dir(&no_marker).unwrap();
        fs::write(no_marker.join("keep"), b"no marker").unwrap();
        let restore_marker = named();
        fs::create_dir(&restore_marker).unwrap();
        fs::write(
            restore_marker.join(STAGE_MARKER),
            StageKind::Restore.marker(),
        )
        .unwrap();
        let linked_marker = named();
        fs::create_dir(&linked_marker).unwrap();
        symlink(outside.join(STAGE_MARKER), linked_marker.join(STAGE_MARKER)).unwrap();
        fs::write(named(), b"a file with a stage name").unwrap();
        let not_owned = folder.join(".brn-backup-stage-not-owned");
        fs::create_dir(&not_owned).unwrap();
        fs::write(not_owned.join(STAGE_MARKER), StageKind::Backup.marker()).unwrap();
        let (folder_before, outside_before) = (tree(&folder), tree(&outside));
        let destination = folder.join("backup.sqlite3");
        store.backup(&destination).unwrap();
        let mut folder_after = tree(&folder);
        assert!(folder_after.remove(&destination).is_some());
        assert_eq!(folder_after, folder_before);
        assert_eq!(tree(&outside), outside_before);
    }

    #[cfg(unix)]
    #[test]
    fn a_backup_sweeps_only_its_own_parent() {
        let root = root();
        let store = Store::open(root.path().join("source")).unwrap();
        let area = root.path().join("area");
        let folder = area.join("backups");
        let sibling = area.join("other");
        let nested = folder.join("nested");
        for path in [&area, &folder, &sibling, &nested] {
            fs::create_dir_all(path).unwrap();
        }
        let elsewhere: Vec<_> = [&area, &sibling, &nested]
            .into_iter()
            .map(|parent| {
                StageDir::create(parent, StageKind::Backup)
                    .unwrap()
                    .abandon()
            })
            .collect();
        store.backup(folder.join("backup.sqlite3")).unwrap();
        for stage in &elsewhere {
            assert!(
                stage.join(STAGE_MARKER).is_file(),
                "{} is outside the backup's folder",
                stage.display()
            );
        }
    }
}
