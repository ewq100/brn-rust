use super::path::{EvidencePath, VaultPath, is_markdown_name};
use crate::MAX_NOTE_BYTES;
use std::{fs::Metadata, os::unix::ffi::OsStrExt, path::Path, time::UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteFile {
    pub path: VaultPath,
    pub size: u64,
    /// Modification time in nanoseconds since the Unix epoch (0 if unavailable).
    pub modified_ns: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceFile {
    pub path: EvidencePath,
    pub size: u64,
    /// Modification time in nanoseconds since the Unix epoch (0 if unavailable).
    pub modified_ns: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    TooLarge,
    InvalidName,
    UnreadableDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub path: String,
    pub reason: SkipReason,
}

#[derive(Debug, Default)]
pub struct Scan {
    pub notes: Vec<NoteFile>,
    pub skipped: Vec<Skipped>,
}

#[derive(Debug, Default)]
pub struct EvidenceScan {
    pub notes: Vec<EvidenceFile>,
    pub skipped: Vec<Skipped>,
}

/// Lists every note in the vault. Symlinks, hidden entries and the top-level
/// `archive/` folder are skipped silently; unusable `.md` files are reported.
pub fn scan(root: &Path) -> std::io::Result<Scan> {
    let found = scan_with_archives(root, false)?;
    let notes = found
        .notes
        .into_iter()
        .map(|note| {
            // The shared walk excludes top-level archives before descending. Convert
            // to the narrower current path rather than expanding its authority.
            Ok(NoteFile {
                path: VaultPath::parse(note.path.as_str()).map_err(std::io::Error::other)?,
                size: note.size,
                modified_ns: note.modified_ns,
            })
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    Ok(Scan {
        notes,
        skipped: found.skipped,
    })
}

/// Lists visible saved Markdown evidence, including top-level archives, with
/// the same hidden/symlink/name/size exclusions as current scanning.
pub fn scan_evidence(root: &Path) -> std::io::Result<EvidenceScan> {
    scan_with_archives(root, true)
}

fn scan_with_archives(root: &Path, include_archive: bool) -> std::io::Result<EvidenceScan> {
    let mut found = EvidenceScan::default();
    visit(root, "", include_archive, &mut found)?;
    found.notes.sort_by(|a, b| a.path.cmp(&b.path));
    found.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}/{name}")
    }
}

fn modified_ns(meta: &Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

fn visit(
    dir: &Path,
    prefix: &str,
    include_archive: bool,
    found: &mut EvidenceScan,
) -> std::io::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) if include_archive && !prefix.is_empty() => {
            // An uninspected evidence subtree is incomplete, never an empty
            // directory. Current knowledge elsewhere can still be retrieved.
            found.skipped.push(Skipped {
                path: prefix.into(),
                reason: SkipReason::UnreadableDirectory,
            });
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let os_name = entry.file_name();
        if os_name.as_bytes().first() == Some(&b'.') {
            continue;
        }
        let Some(name) = os_name.to_str() else {
            let lossy = os_name.to_string_lossy();
            if file_type.is_file() && is_markdown_name(&lossy) {
                found.skipped.push(Skipped {
                    path: join(prefix, &lossy),
                    reason: SkipReason::InvalidName,
                });
            }
            continue;
        };
        let relative = join(prefix, name);
        if file_type.is_dir() {
            if !include_archive && prefix.is_empty() && name.eq_ignore_ascii_case("archive") {
                continue;
            }
            visit(&entry.path(), &relative, include_archive, found)?;
        } else if file_type.is_file() && is_markdown_name(name) {
            let Ok(path) = EvidencePath::parse(&relative) else {
                found.skipped.push(Skipped {
                    path: relative,
                    reason: SkipReason::InvalidName,
                });
                continue;
            };
            let meta = entry.metadata()?;
            if meta.len() > MAX_NOTE_BYTES as u64 {
                found.skipped.push(Skipped {
                    path: relative,
                    reason: SkipReason::TooLarge,
                });
                continue;
            }
            found.notes.push(EvidenceFile {
                path,
                size: meta.len(),
                modified_ns: modified_ns(&meta),
            });
        }
    }
    Ok(())
}
