use super::path::VaultPath;
use crate::MAX_IMPORT_BYTES;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{ErrorKind, Read},
    os::unix::fs::MetadataExt,
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteText {
    pub text: String,
    pub sha256: [u8; 32],
}

#[derive(Debug)]
pub enum ReadError {
    Missing,
    /// A symlink, a folder or another non-regular file.
    NotAFile,
    TooLarge,
    NotUtf8,
    Io(std::io::Error),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => f.write_str("note does not exist"),
            Self::NotAFile => f.write_str("note is not a regular file"),
            Self::TooLarge => f.write_str("note is larger than 1 MiB"),
            Self::NotUtf8 => f.write_str("note is not valid UTF-8"),
            Self::Io(e) => write!(f, "could not read note: {e}"),
        }
    }
}

impl std::error::Error for ReadError {}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == ErrorKind::NotFound {
            Self::Missing
        } else {
            Self::Io(e)
        }
    }
}

/// Reads a note's exact bytes without following symlinks anywhere below `root`.
pub fn read_note(root: &Path, path: &VaultPath) -> Result<NoteText, ReadError> {
    let mut current = root.to_path_buf();
    let mut checked = None;
    for part in path.as_str().split('/') {
        current.push(part);
        let meta = std::fs::symlink_metadata(&current)?;
        if meta.file_type().is_symlink() {
            return Err(ReadError::NotAFile);
        }
        checked = Some(meta);
    }
    let Some(checked) = checked else {
        return Err(ReadError::Missing);
    };
    if !checked.is_file() {
        return Err(ReadError::NotAFile);
    }
    if checked.len() > MAX_IMPORT_BYTES as u64 {
        return Err(ReadError::TooLarge);
    }
    let file = File::open(&current)?;
    let opened = file.metadata()?;
    // If a folder or the file was swapped for a symlink after the checks
    // above, `open` reached a different file: refuse it.
    if !opened.is_file() || (opened.dev(), opened.ino()) != (checked.dev(), checked.ino()) {
        return Err(ReadError::NotAFile);
    }
    let mut bytes = Vec::with_capacity(opened.len() as usize);
    file.take(MAX_IMPORT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(ReadError::TooLarge);
    }
    let sha256 = Sha256::digest(&bytes).into();
    let text = String::from_utf8(bytes).map_err(|_| ReadError::NotUtf8)?;
    Ok(NoteText { text, sha256 })
}
