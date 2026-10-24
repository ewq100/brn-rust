use crate::{
    DATA_MARKER, DATABASE_FILENAME, Error, Result, Store,
    stage::{self, StageDir, StageKind},
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
        Self::prepare_restore(backup, fresh_directory)?.publish()
    }

    /// Validate and copy a backup into an unpublished private stage. Dropping
    /// the returned preview cancels it; only explicit publication exposes the
    /// restored workspace. The source is never reopened at confirmation time.
    pub fn prepare_restore(
        backup: impl AsRef<Path>,
        fresh_directory: impl AsRef<Path>,
    ) -> Result<PreparedRestore> {
        Ok(PreparedRestore(StagedRestore::create(
            backup.as_ref(),
            fresh_directory.as_ref(),
        )?))
    }
}

/// A validated restore snapshot awaiting host-owner confirmation. Not
/// serializable or cloneable: a preview is consumed once, or discarded on Drop.
pub struct PreparedRestore(StagedRestore);
impl PreparedRestore {
    pub fn destination(&self) -> &Path {
        &self.0.destination
    }

    pub fn publish(self) -> Result<Store> {
        self.0.publish()
    }
}

/// Own only the private stage, which holds the workspace in `directory`
/// (`<stage>/workspace`). Publication renames that folder, so dropping the
/// stage never removes a complete published workspace when flush or reopen
/// fails, and the stage marker is never published. A killed process may leave
/// the stage; the next restore into the same parent removes it.
struct StagedRestore {
    // Held for its Drop, which removes the stage.
    _stage: StageDir,
    directory: PathBuf,
    destination: PathBuf,
}
impl StagedRestore {
    fn create(backup: &Path, destination: &Path) -> Result<Self> {
        let source =
            Connection::open_with_flags(backup, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        match validate_schema(
            &source,
            &[3, 4, 5, 6, 7, 8, 9, 10, crate::store::SCHEMA_VERSION],
        ) {
            Ok(()) => {}
            Err(Error::Sqlite(rusqlite::Error::SqliteFailure(code, _)))
                if code.code == rusqlite::ErrorCode::NotADatabase =>
            {
                return Err(Error::UnsafeDirectory(
                    "The restore source is not a BRN backup; choose a BRN backup file".into(),
                ));
            }
            Err(error) => return Err(error),
        }
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
        // Remove what killed restores left in this parent before staging ours.
        stage::sweep_abandoned(parent, StageKind::Restore, destination);
        let stage = StageDir::create(parent, StageKind::Restore)?;
        let directory = stage.path().join("workspace");
        let builder = fs::DirBuilder::new();
        #[cfg(unix)]
        let mut builder = builder;
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        // On failure, dropping the stage removes it.
        builder.create(&directory)?;
        Ok(Self {
            _stage: stage,
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
        // The read-only, validated original backup is the pre-upgrade snapshot.
        // Only this unpublished copy changes; the source remains untouched.
        // One transaction: the missing schema steps; then every edit session
        // becomes owner-held (tokens for the original cannot write the copy);
        // then the copy is an owner workspace, whatever the source's purpose.
        let from: i64 = source.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let tx = store
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        crate::upgrade::upgrade_steps(&tx, from)?;
        tx.execute_batch(
            "DELETE FROM edit_session_bindings;
             INSERT OR REPLACE INTO workspace_profile(id,purpose,changed_at) VALUES(1,'owner',NULL);",
        )?;
        tx.commit()?;
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
    #[cfg(unix)]
    use crate::process_test::OwnedChild;
    #[cfg(unix)]
    use crate::stage::tree;
    use crate::stage::{STAGE_MARKER, stages};
    #[cfg(unix)]
    use std::{process::Stdio, thread, time::Instant};

    /// The staged workspace and its stage wrapper are both gone.
    fn assert_stage_gone(staging: &Path) {
        assert!(!staging.exists());
        let wrapper = staging.parent().unwrap();
        assert!(!wrapper.exists(), "{} must be removed", wrapper.display());
    }

    #[test]
    fn staging_never_exposes_ready_destination() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let source = Store::open(root.path().join("source")).unwrap();
        let backup = root.path().join("backup.sqlite3");
        source.backup(&backup).unwrap();
        let destination = root.path().join("restored");
        let stage = StagedRestore::create(&backup, &destination).unwrap();
        let staging = stage.directory.clone();
        let wrapper = staging.parent().unwrap();
        assert_eq!(wrapper.parent(), destination.parent());
        assert!(wrapper.join(STAGE_MARKER).is_file());
        assert_eq!(fs::read_dir(&staging).unwrap().count(), 2);
        validate_database(&staging.join(DATABASE_FILENAME)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Windows locks are mandatory, so only Unix can read a held marker.
            assert_eq!(
                fs::read(wrapper.join(STAGE_MARKER)).unwrap(),
                StageKind::Restore.marker().as_bytes()
            );
            for folder in [&staging, wrapper] {
                assert_eq!(
                    fs::metadata(folder).unwrap().permissions().mode() & 0o777,
                    0o700
                );
            }
        }
        assert!(
            !destination.exists(),
            "restore must publish only a complete staged directory"
        );
        stage.publish().unwrap();
        assert_stage_gone(&staging);
        // The published workspace is the staged one, without the stage marker.
        assert!(!destination.join(STAGE_MARKER).exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let source = Store::open(root.path().join("source")).unwrap();
        let backup = root.path().join("backup.sqlite3");
        source.backup(&backup).unwrap();
        (root, backup)
    }

    #[test]
    fn public_restore_preview_drop_discards_copy_and_retry_can_publish() {
        let (root, backup) = fixture();
        let before = fs::read(&backup).unwrap();
        let destination = root.path().join("restored");
        let preview = Store::prepare_restore(&backup, &destination).unwrap();
        assert_eq!(preview.destination(), destination);
        assert!(!destination.exists());
        let staged = preview.0.directory.clone();
        drop(preview);
        assert_stage_gone(&staged);
        assert!(!destination.exists());
        Store::prepare_restore(&backup, &destination)
            .unwrap()
            .publish()
            .unwrap();
        assert!(Store::open(&destination).is_ok());
        assert_eq!(fs::read(&backup).unwrap(), before);
    }

    #[test]
    fn public_restore_preview_binds_copy_not_later_source_bytes() {
        let (root, backup) = fixture();
        let destination = root.path().join("restored");
        let preview = Store::prepare_restore(&backup, &destination).unwrap();
        fs::write(&backup, b"replaced after preview").unwrap();
        let restored = preview.publish().unwrap();
        assert!(restored.records().is_ok());
        drop(restored);
        assert!(Store::open(&destination).is_ok());
        assert_eq!(fs::read(&backup).unwrap(), b"replaced after preview");
    }

    #[test]
    fn public_restore_preview_rechecks_destination_at_confirmation() {
        let (root, backup) = fixture();
        let destination = root.path().join("restored");
        let preview = Store::prepare_restore(&backup, &destination).unwrap();
        let staged = preview.0.directory.clone();
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("human-work"), b"preserve").unwrap();
        assert!(preview.publish().is_err());
        assert_eq!(
            fs::read(destination.join("human-work")).unwrap(),
            b"preserve"
        );
        assert_stage_gone(&staged);
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
            assert_stage_gone(&staging);
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
        assert_stage_gone(&staging);
        assert!(!destination.exists());
        // Opening succeeds, but the SQLite backup call fails on corrupt input.
        let corrupt = root.path().join("corrupt.sqlite3");
        fs::write(&corrupt, b"not a database").unwrap();
        let source = Connection::open(&corrupt).unwrap();
        let stage = StagedRestore::prepare(&destination).unwrap();
        let staging = stage.directory.clone();
        assert!(matches!(stage.copy_from(&source), Err(Error::Sqlite(_))));
        assert_stage_gone(&staging);
        assert!(!destination.exists());
        assert_eq!(fs::read(&corrupt).unwrap(), b"not a database");
        assert_eq!(
            stages(root.path(), StageKind::Restore),
            Vec::<PathBuf>::new()
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_killed_restores_stage_is_removed_by_the_next_restore_into_that_parent() {
        const CHILD_ROOT: &str = "BRN_TEST_RESTORE_PROCESS_STAGE_ROOT";
        if let Some(root) = std::env::var_os(CHILD_ROOT) {
            let root = PathBuf::from(root);
            let backup = root.join("backup.sqlite3");
            let destination = root.join("child-process-destination");
            let stage = StagedRestore::create(&backup, &destination).unwrap();
            let ready = root.join("restore-process-stage-ready");
            let ready_temporary = root.join("restore-process-stage-ready.tmp");
            fs::write(
                &ready_temporary,
                stage._stage.path().as_os_str().as_encoded_bytes(),
            )
            .unwrap();
            fs::rename(ready_temporary, ready).unwrap();
            let started = Instant::now();
            while started.elapsed() < Duration::from_secs(30) {
                assert!(stage.directory.is_dir());
                thread::sleep(Duration::from_millis(20));
            }
            panic!("restore stage child exceeded its 30-second fail-safe");
        }

        let (root, backup) = fixture();
        let backup_bytes = fs::read(&backup).unwrap();
        let earlier = root.path().join("earlier");
        drop(Store::restore(&backup, &earlier).unwrap());
        let earlier_before = tree(&earlier);

        let ready = root.path().join("restore-process-stage-ready");
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .arg("--exact")
            .arg("restore::tests::a_killed_restores_stage_is_removed_by_the_next_restore_into_that_parent")
            .env(CHILD_ROOT, root.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = OwnedChild::spawn(&mut command).unwrap();
        let ready_deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() && Instant::now() < ready_deadline {
            if let Some(status) = child.try_wait() {
                panic!("restore stage child exited before readiness: {status}");
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(ready.is_file(), "restore stage child did not become ready");
        use std::os::unix::ffi::OsStringExt;
        let reported = PathBuf::from(std::ffi::OsString::from_vec(fs::read(&ready).unwrap()));
        let canonical_root = root.path().canonicalize().unwrap();
        assert_eq!(reported.parent(), Some(canonical_root.as_path()));
        assert!(
            reported
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(StageKind::Restore.prefix())
        );
        assert!(stages(&canonical_root, StageKind::Restore).contains(&reported));
        let child_tree = tree(&reported);
        assert!(reported.join("workspace").join(DATABASE_FILENAME).is_file());

        let live_probe = canonical_root.join("live-process-probe");
        drop(Store::restore(&backup, &live_probe).unwrap());
        assert!(reported.is_dir(), "a live process stage must remain");
        assert_eq!(
            tree(&reported),
            child_tree,
            "a live process stage is untouched"
        );
        assert!(!live_probe.join(STAGE_MARKER).exists());
        let live_probe_tree = tree(&live_probe);

        // Prepare the original same-process full-copy witness while the child
        // still owns its lock, so no later prepare can sweep the child stage.
        let StagedRestore {
            _stage: full,
            directory,
            ..
        } = StagedRestore::create(&backup, &root.path().join("killed")).unwrap();
        assert!(directory.join(DATABASE_FILENAME).is_file());
        let full = full.abandon();
        assert_eq!(tree(&reported), child_tree);

        let status = child.kill_and_reap();
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            status.signal(),
            Some(9),
            "the owned child must end by SIGKILL"
        );
        assert_eq!(
            tree(&reported),
            child_tree,
            "SIGKILL leaves the staged copy"
        );
        assert!(!root.path().join("child-process-destination").exists());

        // Killed after publication, before the wrapper was removed.
        let wrapper = StageDir::create(root.path(), StageKind::Restore)
            .unwrap()
            .abandon();
        let unrelated = root.path().join(".brn-restore-stage-not-owned");
        fs::create_dir(&unrelated).unwrap();
        fs::write(unrelated.join("keep"), b"unrelated").unwrap();
        let destination = root.path().join("restored");
        assert!(reported.is_dir(), "the killed stage awaits public retry");
        let store = Store::restore(&backup, &destination).unwrap();
        assert!(store.records().unwrap().is_empty());
        assert!(!reported.exists(), "the killed process stage must be swept");
        assert!(!full.exists(), "the full leftover copy must be removed");
        assert!(!wrapper.exists(), "the leftover wrapper must be removed");
        assert_eq!(
            stages(root.path(), StageKind::Restore),
            vec![unrelated.clone()]
        );
        assert_eq!(fs::read(unrelated.join("keep")).unwrap(), b"unrelated");
        assert_eq!(tree(&live_probe), live_probe_tree);
        assert_eq!(tree(&earlier), earlier_before);
        assert_eq!(fs::read(&backup).unwrap(), backup_bytes);
        assert!(!root.path().join("killed").exists());
        assert!(!destination.join(STAGE_MARKER).exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_live_restore_stage_survives() {
        let (root, backup) = fixture();
        let live = StageDir::create(root.path(), StageKind::Restore).unwrap();
        fs::create_dir(live.path().join("workspace")).unwrap();
        fs::write(live.path().join("workspace").join("partial"), b"copying").unwrap();
        let before = tree(live.path());
        Store::restore(&backup, root.path().join("restored")).unwrap();
        assert!(live.path().is_dir(), "a held stage is never removed");
        assert_eq!(tree(live.path()), before, "a held stage is never touched");
        let path = live.path().to_owned();
        drop(live);
        assert!(!path.exists());
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
        assert_stage_gone(&staging);
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
            assert_stage_gone(&staging);
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
            assert!(matches!(
                Store::restore(source, &destination),
                Err(Error::UnsafeDirectory(_))
            ));
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
        assert_stage_gone(&staging);
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
