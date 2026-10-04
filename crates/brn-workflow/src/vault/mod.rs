//! The vault folder: which files are notes, listing them and reading them.
mod path;
mod read;
mod scan;

pub use path::{EvidencePath, VaultPath, VaultPathError};
pub use read::{NoteText, ReadError, read_evidence, read_note};
pub use scan::{
    EvidenceFile, EvidenceScan, NoteFile, Scan, SkipReason, Skipped, scan, scan_evidence,
};
