use std::path::{Path, PathBuf};

/// A vault-relative path to a note: `/`-separated, ends in `.md`, never hidden,
/// never under the top-level `archive/` folder and never escaping the vault.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultPath(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultPathError {
    Empty,
    Absolute,
    InvalidComponent,
    Traversal,
    Hidden,
    Archived,
    NotMarkdown,
}

impl std::fmt::Display for VaultPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Empty => "note path is empty",
            Self::Absolute => "note path must be relative to the vault",
            Self::InvalidComponent => "note path has an empty or invalid part",
            Self::Traversal => "note path must stay inside the vault",
            Self::Hidden => "hidden files and folders are not notes",
            Self::Archived => "notes in archive/ are not available",
            Self::NotMarkdown => "notes must be .md files",
        })
    }
}

impl std::error::Error for VaultPathError {}

pub(crate) fn is_markdown_name(name: &str) -> bool {
    name.len() > 3
        && name
            .get(name.len() - 3..)
            .is_some_and(|ext| ext.eq_ignore_ascii_case(".md"))
}

impl VaultPath {
    pub fn parse(raw: &str) -> Result<Self, VaultPathError> {
        Self::validate_components(raw)?;
        let name = raw.rsplit('/').next().ok_or(VaultPathError::Empty)?;
        if !is_markdown_name(name) {
            return Err(VaultPathError::NotMarkdown);
        }
        Ok(Self(raw.to_owned()))
    }

    /// A component-only folder filter, without the Markdown suffix requirement.
    pub fn validate_folder(raw: &str) -> Result<(), VaultPathError> {
        Self::validate_components(raw)?;
        if raw.eq_ignore_ascii_case("archive") {
            return Err(VaultPathError::Archived);
        }
        Ok(())
    }

    fn validate_components(raw: &str) -> Result<(), VaultPathError> {
        if raw.is_empty() {
            return Err(VaultPathError::Empty);
        }
        if raw.starts_with('/') {
            return Err(VaultPathError::Absolute);
        }
        if raw.contains('\\') || raw.contains('\0') {
            return Err(VaultPathError::InvalidComponent);
        }
        let parts: Vec<&str> = raw.split('/').collect();
        for part in &parts {
            match *part {
                "" | "." => return Err(VaultPathError::InvalidComponent),
                ".." => return Err(VaultPathError::Traversal),
                p if p.starts_with('.') => return Err(VaultPathError::Hidden),
                _ => {}
            }
        }
        if parts.len() > 1 && parts[0].eq_ignore_ascii_case("archive") {
            return Err(VaultPathError::Archived);
        }
        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_fs_path(&self, root: &Path) -> PathBuf {
        let mut path = root.to_path_buf();
        for part in self.0.split('/') {
            path.push(part);
        }
        path
    }
}

impl std::fmt::Display for VaultPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
