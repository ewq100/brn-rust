use super::path::{EvidencePath, VaultPath};
use crate::MAX_NOTE_BYTES;
use brn_store::files::FileFingerprint;
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

/// Reads a note's exact bytes, refusing symlinked path parts and a file swapped
/// before opening (see the accepted parent-folder race limit below).
pub fn read_note(root: &Path, path: &VaultPath) -> Result<NoteText, ReadError> {
    read_path(root, path.as_str()).map(|(note, _)| note)
}

/// Reads explicit saved evidence, including archives, with the same exact-byte
/// and supported-file checks as current-note reading.
pub fn read_evidence(root: &Path, path: &EvidencePath) -> Result<NoteText, ReadError> {
    read_path(root, path.as_str()).map(|(note, _)| note)
}

pub(crate) fn read_evidence_source(
    root: &Path,
    path: &EvidencePath,
) -> Result<crate::proposals::ProposalSource, ReadError> {
    let (note, fingerprint) = read_path(root, path.as_str())?;
    Ok(crate::proposals::ProposalSource {
        source: crate::proposals::SourceVersion {
            path: path.as_str().into(),
            fingerprint,
        },
        text: note.text,
    })
}

fn read_path(root: &Path, path: &str) -> Result<(NoteText, FileFingerprint), ReadError> {
    let mut current = root.to_path_buf();
    let mut checked = None;
    for part in path.split('/') {
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
    if checked.len() > MAX_NOTE_BYTES as u64 {
        return Err(ReadError::TooLarge);
    }
    let file = File::open(&current)?;
    let opened = file.metadata()?;
    // The dev/inode check refuses a final file swapped between checks and open.
    // A parent folder swapped for a symlink during the read can still lead outside
    // the vault; this needs write access inside the vault and is an accepted limit.
    if !opened.is_file() || (opened.dev(), opened.ino()) != (checked.dev(), checked.ino()) {
        return Err(ReadError::NotAFile);
    }
    if opened.len() > MAX_NOTE_BYTES as u64 {
        return Err(ReadError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(opened.len().min(MAX_NOTE_BYTES as u64 + 1) as usize);
    file.take(MAX_NOTE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_NOTE_BYTES {
        return Err(ReadError::TooLarge);
    }
    let sha256 = Sha256::digest(&bytes).into();
    let text = String::from_utf8(bytes).map_err(|_| ReadError::NotUtf8)?;
    let fingerprint = FileFingerprint {
        device: opened.dev(),
        inode: opened.ino(),
        len: text.len() as u64,
        sha256,
    };
    Ok((NoteText { text, sha256 }, fingerprint))
}
