use crate::ConversionError;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

pub(crate) struct TransientDir {
    pub path: PathBuf,
}
impl TransientDir {
    pub fn create(root: &Path) -> Result<Self, ConversionError> {
        if !root.is_absolute() {
            return Err(ConversionError::Invalid);
        }
        fs::create_dir_all(root).map_err(|_| ConversionError::Io)?;
        if !fs::symlink_metadata(root)
            .map_err(|_| ConversionError::Io)?
            .is_dir()
        {
            return Err(ConversionError::Invalid);
        }
        cleanup_expired(root)?;
        let path = root.join(format!("threads-intake-{}", uuid::Uuid::new_v4()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&path)
                .map_err(|_| ConversionError::Io)?;
        }
        #[cfg(not(unix))]
        {
            return Err(ConversionError::Unavailable);
        }
        Ok(Self {
            path: fs::canonicalize(&path).map_err(|_| ConversionError::Io)?,
        })
    }
    pub fn check_budget(&self, limit: usize) -> Result<(), ConversionError> {
        let mut bytes = 0u64;
        for (index, entry) in fs::read_dir(&self.path)
            .map_err(|_| ConversionError::Io)?
            .enumerate()
        {
            if index > brn_intake::MAX_ASSETS + 8 {
                return Err(ConversionError::Budget);
            }
            let metadata =
                match fs::symlink_metadata(entry.map_err(|_| ConversionError::Io)?.path()) {
                    Ok(metadata) => metadata,
                    // Renderers atomically rename their owned cache files. An enumerated
                    // name may vanish before stat; absent entries consume no scratch bytes.
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(_) => return Err(ConversionError::Io),
                };
            if !metadata.is_file() {
                return Err(ConversionError::Invalid);
            }
            bytes = bytes
                .checked_add(metadata.len())
                .ok_or(ConversionError::Budget)?;
            if bytes > limit as u64 {
                return Err(ConversionError::Budget);
            }
        }
        Ok(())
    }
    pub fn cleanup(&self) -> Result<(), ConversionError> {
        fs::remove_dir_all(&self.path).map_err(|_| ConversionError::Io)
    }
}
impl Drop for TransientDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn cleanup_expired(root: &Path) -> Result<usize, ConversionError> {
    if !root.exists() {
        return Ok(0);
    }
    if !root.is_absolute()
        || !fs::symlink_metadata(root)
            .map_err(|_| ConversionError::Io)?
            .is_dir()
    {
        return Err(ConversionError::Invalid);
    }
    let mut removed = 0;
    for entry in fs::read_dir(root).map_err(|_| ConversionError::Io)? {
        let entry = entry.map_err(|_| ConversionError::Io)?;
        let name = entry.file_name();
        let Some(suffix) = name
            .to_str()
            .and_then(|s| s.strip_prefix("threads-intake-"))
        else {
            continue;
        };
        if uuid::Uuid::parse_str(suffix).is_err() {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path()).map_err(|_| ConversionError::Io)?;
        if !metadata.is_dir() {
            continue;
        }
        if SystemTime::now()
            .duration_since(metadata.modified().map_err(|_| ConversionError::Io)?)
            .unwrap_or(Duration::ZERO)
            >= Duration::from_secs(24 * 60 * 60)
        {
            fs::remove_dir_all(entry.path()).map_err(|_| ConversionError::Io)?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn missing_scratch_root_and_excess_entries_still_fail() {
        let root = tempfile::tempdir().unwrap();
        let temp = TransientDir::create(root.path()).unwrap();
        for index in 0..brn_intake::MAX_ASSETS + 10 {
            fs::write(temp.path.join(format!("entry-{index}")), []).unwrap();
        }
        assert_eq!(temp.check_budget(4096), Err(ConversionError::Budget));
        temp.cleanup().unwrap();
        assert_eq!(temp.check_budget(4096), Err(ConversionError::Io));
    }

    #[cfg(unix)]
    #[test]
    fn existing_symlinks_and_non_regular_entries_still_fail() {
        let root = tempfile::tempdir().unwrap();
        let temp = TransientDir::create(root.path()).unwrap();
        let directory = temp.path.join("not-a-file");
        fs::create_dir(&directory).unwrap();
        assert_eq!(temp.check_budget(4096), Err(ConversionError::Invalid));
        fs::remove_dir(directory).unwrap();
        let target = root.path().join("outside-owned-run");
        fs::write(&target, [0u8; 16]).unwrap();
        std::os::unix::fs::symlink(&target, temp.path.join("link")).unwrap();
        assert_eq!(temp.check_budget(4096), Err(ConversionError::Invalid));
        temp.cleanup().unwrap();
        assert_eq!(fs::read(target).unwrap(), [0u8; 16]);
    }

    #[test]
    fn concurrent_cache_renames_do_not_fail_scratch_budget_monitoring() {
        let root = tempfile::tempdir().unwrap();
        let temp = TransientDir::create(root.path()).unwrap();
        fs::write(temp.path.join("stable-output"), [0u8; 128]).unwrap();
        let from = temp.path.join("cache-new");
        let to = temp.path.join("cache-ready");
        fs::write(&from, [0u8; 32]).unwrap();
        let running = Arc::new(AtomicBool::new(true));
        let barrier = Arc::new(Barrier::new(2));
        let writer_running = running.clone();
        let writer_barrier = barrier.clone();
        let writer = std::thread::spawn(move || {
            writer_barrier.wait();
            while writer_running.load(Ordering::Acquire) {
                fs::rename(&from, &to).unwrap();
                fs::rename(&to, &from).unwrap();
            }
        });
        barrier.wait();
        let result = (0..5_000).try_for_each(|_| temp.check_budget(4096));
        running.store(false, Ordering::Release);
        writer.join().unwrap();
        assert_eq!(result, Ok(()));
        assert_eq!(temp.check_budget(159), Err(ConversionError::Budget));
        temp.cleanup().unwrap();
    }
}
