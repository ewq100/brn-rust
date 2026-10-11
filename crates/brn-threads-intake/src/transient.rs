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
            let metadata = fs::symlink_metadata(entry.map_err(|_| ConversionError::Io)?.path())
                .map_err(|_| ConversionError::Io)?;
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
