//! Private stage folders for backup and restore. A stage is
//! `<parent>/<prefix><uuid>` holding a `.brn-stage` marker. The marker's text
//! says which kind of BRN stage it is, and the creating process holds an OS
//! lock on it for the stage's lifetime. The OS releases that lock however the
//! process ends, including SIGKILL, so a stage whose marker is free and
//! carries BRN's text was abandoned. The next backup or restore into the same
//! folder removes such stages and nothing else.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub(crate) const STAGE_MARKER: &str = ".brn-stage";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StageKind {
    Backup,
    Restore,
}

impl StageKind {
    pub(crate) fn prefix(self) -> &'static str {
        match self {
            Self::Backup => ".brn-backup-stage-",
            Self::Restore => ".brn-restore-stage-",
        }
    }

    pub(crate) fn marker(self) -> &'static str {
        match self {
            Self::Backup => "BRN backup stage v1\n",
            Self::Restore => "BRN restore stage v1\n",
        }
    }
}

/// An owned stage. Dropping it removes the folder, then unlocks.
pub(crate) struct StageDir {
    path: PathBuf,
    // Locked for the stage's lifetime; `None` only after a test's `abandon`.
    lock: Option<File>,
}

impl StageDir {
    /// Exclusively creates a fresh 0700 stage in `parent` with its locked
    /// marker. Any failure after the folder exists removes it again.
    pub(crate) fn create(parent: &Path, kind: StageKind) -> io::Result<Self> {
        let path = parent.join(format!("{}{}", kind.prefix(), uuid::Uuid::new_v4()));
        let builder = fs::DirBuilder::new();
        #[cfg(unix)]
        let mut builder = builder;
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path)?;
        match mark(&path, kind) {
            Ok(lock) => Ok(Self {
                path,
                lock: Some(lock),
            }),
            Err(error) => {
                let _ = fs::remove_dir_all(&path);
                Err(error)
            }
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Leaves the stage as a killed process does: folder kept, lock free.
    #[cfg(test)]
    pub(crate) fn abandon(mut self) -> PathBuf {
        drop(self.lock.take());
        self.path.clone()
    }
}

impl Drop for StageDir {
    fn drop(&mut self) {
        let Some(lock) = self.lock.take() else {
            return;
        };
        // Remove while still locked, so no sweep ever sees this stage free.
        let removed = fs::remove_dir_all(&self.path);
        let _ = lock.unlock();
        drop(lock);
        if removed.is_err() {
            // Some platforms refuse to delete a file that is still open.
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// The marker is written only while locked. A sweep that wins the lock
/// before this writes sees an empty marker and leaves the stage alone.
fn mark(stage: &Path, kind: StageKind) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(stage.join(STAGE_MARKER))?;
    file.lock()?;
    file.write_all(kind.marker().as_bytes())?;
    file.sync_all()?;
    Ok(file)
}

/// Best effort: removes the abandoned `kind` stages that are direct children
/// of `parent`, never `keep` (the requested destination). An entry that fails
/// any check, or any error, leaves it in place and never fails the caller.
/// Returns how many stages were removed.
#[cfg(unix)]
pub(crate) fn sweep_abandoned(parent: &Path, kind: StageKind, keep: &Path) -> usize {
    let Ok(entries) = fs::read_dir(parent) else {
        return 0;
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| remove_if_abandoned(entry, kind, keep).unwrap_or(false))
        .count()
}

/// Stage liveness relies on Unix file identity; elsewhere nothing is removed.
#[cfg(not(unix))]
pub(crate) fn sweep_abandoned(_parent: &Path, _kind: StageKind, _keep: &Path) -> usize {
    0
}

#[cfg(unix)]
fn remove_if_abandoned(entry: &fs::DirEntry, kind: StageKind, keep: &Path) -> io::Result<bool> {
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;
    // 1. Exactly BRN's name: the kind's prefix and a lowercase hyphenated UUID.
    let name = entry.file_name();
    let Some(suffix) = name
        .to_str()
        .and_then(|name| name.strip_prefix(kind.prefix()))
    else {
        return Ok(false);
    };
    match uuid::Uuid::parse_str(suffix) {
        Ok(id) if id.hyphenated().to_string() == suffix => {}
        _ => return Ok(false),
    }
    let stage = entry.path();
    // 2. Never the requested destination.
    if stage == keep {
        return Ok(false);
    }
    // 3. A real directory, never a symlink.
    if !entry.file_type()?.is_dir() {
        return Ok(false);
    }
    let folder = fs::symlink_metadata(&stage)?;
    if !folder.file_type().is_dir() {
        return Ok(false);
    }
    // 4. The marker is a regular file, not a symlink.
    let marker = stage.join(STAGE_MARKER);
    let listed = fs::symlink_metadata(&marker)?;
    if !listed.file_type().is_file() {
        return Ok(false);
    }
    // 5. The opened file is the one just checked, not a swapped-in symlink.
    let mut file = OpenOptions::new().read(true).write(true).open(&marker)?;
    let opened = file.metadata()?;
    if (opened.dev(), opened.ino()) != (listed.dev(), listed.ino()) {
        return Ok(false);
    }
    // 6. No process holds it. The lock is released when `file` drops.
    match file.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => return Ok(false),
        Err(fs::TryLockError::Error(error)) => return Err(error),
    }
    // 7. BRN's own marker for this kind, read while locked.
    let mut text = Vec::new();
    (&mut file)
        .take(kind.marker().len() as u64 + 1)
        .read_to_end(&mut text)?;
    if text != kind.marker().as_bytes() {
        return Ok(false);
    }
    // 8. Still the same real directory.
    let again = fs::symlink_metadata(&stage)?;
    if !again.file_type().is_dir() || (again.dev(), again.ino()) != (folder.dev(), folder.ino()) {
        return Ok(false);
    }
    // 9. Remove while locked. remove_dir_all never follows symlinks inside.
    fs::remove_dir_all(&stage)?;
    Ok(true)
}

/// The `kind`-prefixed entries directly in `parent`, sorted.
#[cfg(test)]
pub(crate) fn stages(parent: &Path, kind: StageKind) -> Vec<PathBuf> {
    let mut found: Vec<_> = fs::read_dir(parent)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(kind.prefix())
        })
        .map(|entry| entry.path())
        .collect();
    found.sort();
    found
}

/// Every entry under `root` without following symlinks: file bytes, symlink
/// targets, and folders, so tests can show a tree is unchanged.
#[cfg(all(test, unix))]
pub(crate) fn tree(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut entries = std::collections::BTreeMap::new();
    let mut pending = vec![root.to_owned()];
    while let Some(path) = pending.pop() {
        let kind = fs::symlink_metadata(&path).unwrap().file_type();
        let content = if kind.is_symlink() {
            let target = fs::read_link(&path).unwrap();
            format!("link:{}", target.display()).into_bytes()
        } else if kind.is_dir() {
            for entry in fs::read_dir(&path).unwrap() {
                pending.push(entry.unwrap().path());
            }
            b"dir".to_vec()
        } else {
            fs::read(&path).unwrap()
        };
        entries.insert(path, content);
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> tempfile::TempDir {
        tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
    }

    #[test]
    fn a_stage_is_private_marked_locked_and_removed_on_drop() {
        let root = root();
        for kind in [StageKind::Backup, StageKind::Restore] {
            let stage = StageDir::create(root.path(), kind).unwrap();
            let path = stage.path().to_owned();
            assert_eq!(path.parent(), Some(root.path()));
            let name = path.file_name().unwrap().to_str().unwrap();
            assert!(name.starts_with(kind.prefix()));
            let marker = path.join(STAGE_MARKER);
            // Windows locks are mandatory, so only Unix can read a held marker.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(fs::read(&marker).unwrap(), kind.marker().as_bytes());
                let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
                assert_eq!(mode(&path), 0o700);
                assert_eq!(mode(&marker), 0o600);
            }
            let other = File::open(&marker).unwrap();
            assert!(matches!(
                other.try_lock(),
                Err(fs::TryLockError::WouldBlock)
            ));
            drop(other);
            drop(stage);
            assert!(!path.exists());
        }
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn an_abandoned_stage_keeps_its_folder_and_frees_its_lock() {
        let root = root();
        let path = StageDir::create(root.path(), StageKind::Backup)
            .unwrap()
            .abandon();
        let marker = File::open(path.join(STAGE_MARKER)).unwrap();
        marker.try_lock().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_sweep_removes_only_abandoned_stages_of_its_kind_with_exact_names() {
        let root = root();
        let parent = root.path();
        let backup = StageDir::create(parent, StageKind::Backup)
            .unwrap()
            .abandon();
        let restore = StageDir::create(parent, StageKind::Restore)
            .unwrap()
            .abandon();
        let live = StageDir::create(parent, StageKind::Backup).unwrap();
        let id = uuid::Uuid::new_v4();
        let mut kept = vec![restore.clone(), live.path().to_owned()];
        // Not BRN's exact form: other UUID spellings, a suffix, no UUID.
        for name in [
            format!(".brn-backup-stage-{}", id.simple()),
            format!(
                ".brn-backup-stage-{}",
                id.hyphenated().to_string().to_uppercase()
            ),
            format!(".brn-backup-stage-{id}-copy"),
            ".brn-backup-stage-not-owned".to_owned(),
        ] {
            let path = parent.join(name);
            fs::create_dir(&path).unwrap();
            fs::write(path.join(STAGE_MARKER), StageKind::Backup.marker()).unwrap();
            kept.push(path);
        }
        // The requested destination is never removed, even if stage-shaped.
        let keep = parent.join(format!(".brn-backup-stage-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&keep).unwrap();
        fs::write(keep.join(STAGE_MARKER), StageKind::Backup.marker()).unwrap();
        kept.push(keep.clone());
        assert_eq!(sweep_abandoned(parent, StageKind::Backup, &keep), 1);
        assert!(!backup.exists());
        for path in &kept {
            assert!(path.join(STAGE_MARKER).is_file(), "{}", path.display());
        }
        assert_eq!(sweep_abandoned(parent, StageKind::Backup, &keep), 0);
        assert_eq!(sweep_abandoned(parent, StageKind::Restore, &keep), 1);
        assert!(!restore.exists());
        drop(live);
    }
}
