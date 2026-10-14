//! Liveness hint for a Working Run. The process running it holds an OS lock
//! on `derived/run-leases/<run>.lock` for the Run's lifetime. The OS releases
//! the lock however that process ends, including SIGKILL, so a Working Run
//! whose lock is free is orphaned. SQLite stays the only Run state; a lease
//! file is derived and may be deleted.
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

const DIRECTORY: &str = "run-leases";

/// Held while a Run executes. Dropping it removes the file, then unlocks.
pub struct RunLease {
    file: File,
    path: PathBuf,
}

impl RunLease {
    /// Takes the lease for `run` under the core-validated workspace `derived`
    /// directory. Fails if another handle already holds it.
    pub fn acquire(derived: &Path, run: &str) -> io::Result<Self> {
        let path = lease_path(derived, run, true)?;
        let file = open(&path, true)?;
        match file.try_lock() {
            Ok(()) => Ok(Self { file, path }),
            Err(TryLockError::WouldBlock) => Err(io::Error::new(
                ErrorKind::WouldBlock,
                "this run is already leased",
            )),
            Err(TryLockError::Error(error)) => Err(error),
        }
    }
}

impl Drop for RunLease {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = self.file.unlock();
    }
}

/// Whether a process still holds `run`'s lease. A missing lease is not live.
/// Any doubt counts as live, so a running Run is never interrupted by mistake.
pub fn run_is_live(derived: &Path, run: &str) -> bool {
    let path = match lease_path(derived, run, false) {
        Ok(path) => path,
        Err(error) => return error.kind() != ErrorKind::NotFound,
    };
    let file = match open(&path, false) {
        Ok(file) => file,
        Err(error) => return error.kind() != ErrorKind::NotFound,
    };
    match file.try_lock() {
        Ok(()) => {
            let _ = file.unlock();
            false
        }
        Err(_) => true,
    }
}

/// `derived/run-leases/<run>.lock`, refusing symlinked or foreign paths. Only
/// host Run IDs are accepted; `:` maps to `_` in the file name.
fn lease_path(derived: &Path, run: &str, create: bool) -> io::Result<PathBuf> {
    if run.is_empty()
        || run.len() > 128
        || !run
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'_' | b'-'))
    {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "run leases need a host Run ID",
        ));
    }
    let directory = derived.join(DIRECTORY);
    if create {
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    for folder in [derived, directory.as_path()] {
        let kind = fs::symlink_metadata(folder)?.file_type();
        if kind.is_symlink() || !kind.is_dir() {
            return Err(io::Error::other("run leases must be in real directories"));
        }
    }
    let path = directory.join(format!("{}.lock", run.replace(':', "_")));
    match fs::symlink_metadata(&path) {
        Ok(meta) if !meta.file_type().is_file() => {
            Err(io::Error::other("a run lease must be a regular file"))
        }
        Ok(_) => Ok(path),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(path),
        Err(error) => Err(error),
    }
}

fn open(path: &Path, create: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if create {
        options.create(true).truncate(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn derived() -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let derived = root.path().join("derived");
        fs::create_dir(&derived).unwrap();
        (root, derived)
    }

    #[test]
    fn a_held_lease_is_live_and_a_dropped_or_abandoned_one_is_not() {
        let (_root, derived) = derived();
        let run = "0b6f6c1e-2f4a-4c55-9b1e-8f2c9a0d1e3f:0";
        assert!(!run_is_live(&derived, run), "no lease yet");
        let lease = RunLease::acquire(&derived, run).unwrap();
        assert!(run_is_live(&derived, run));
        assert!(RunLease::acquire(&derived, run).is_err(), "one holder");
        drop(lease);
        assert!(!run_is_live(&derived, run));
        assert!(
            !derived
                .join(DIRECTORY)
                .join(format!("{}.lock", run.replace(':', "_")))
                .exists()
        );
        // A process that died leaves its file behind, but not its lock.
        fs::write(derived.join(DIRECTORY).join("abandoned_0.lock"), b"").unwrap();
        assert!(!run_is_live(&derived, "abandoned:0"));
    }

    #[cfg(unix)]
    #[test]
    fn foreign_ids_and_symlinked_paths_are_refused_and_count_as_live() {
        let (root, derived) = derived();
        for bad in ["", "../escape", "a/b", &"x".repeat(129)] {
            assert!(RunLease::acquire(&derived, bad).is_err(), "{bad}");
            assert!(run_is_live(&derived, bad), "{bad}");
        }
        let elsewhere = root.path().join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        std::os::unix::fs::symlink(&elsewhere, derived.join(DIRECTORY)).unwrap();
        assert!(RunLease::acquire(&derived, "run:0").is_err());
        assert!(run_is_live(&derived, "run:0"));
        assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 0);
    }
}
