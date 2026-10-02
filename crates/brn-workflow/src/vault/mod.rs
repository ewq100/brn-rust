//! The vault folder: which files are notes, listing them and reading them.
mod path;
mod read;
mod scan;

pub use path::{VaultPath, VaultPathError};
pub use read::{NoteText, ReadError, read_note};
pub use scan::{NoteFile, Scan, SkipReason, Skipped, scan};
