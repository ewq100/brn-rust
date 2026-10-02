use super::path::{VaultPath, is_markdown_name};
use crate::MAX_IMPORT_BYTES;
use std::{fs::Metadata, path::Path, time::UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteFile {
    pub path: VaultPath,
    pub size: u64,
    /// Modification time in nanoseconds since the Unix epoch (0 if unavailable).
    pub modified_ns: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    TooLarge,
    InvalidName,
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

/// Lists every note in the vault. Symlinks, hidden entries and the top-level
/// `archive/` folder are skipped silently; unusable `.md` files are reported.
pub fn scan(root: &Path) -> std::io::Result<Scan> {
    let mut found = Scan::default();
    visit(root, "", &mut found)?;
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

fn visit(dir: &Path, prefix: &str, found: &mut Scan) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let os_name = entry.file_name();
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
        if name.starts_with('.') {
            continue;
        }
        let relative = join(prefix, name);
        if file_type.is_dir() {
            if prefix.is_empty() && name.eq_ignore_ascii_case("archive") {
                continue;
            }
            visit(&entry.path(), &relative, found)?;
        } else if file_type.is_file() && is_markdown_name(name) {
            let Ok(path) = VaultPath::parse(&relative) else {
                found.skipped.push(Skipped {
                    path: relative,
                    reason: SkipReason::InvalidName,
                });
                continue;
            };
            let meta = entry.metadata()?;
            if meta.len() > MAX_IMPORT_BYTES as u64 {
                found.skipped.push(Skipped {
                    path: relative,
                    reason: SkipReason::TooLarge,
                });
                continue;
            }
            found.notes.push(NoteFile {
                path,
                size: meta.len(),
                modified_ns: modified_ns(&meta),
            });
        }
    }
    Ok(())
}
