use crate::PdfTools;
use std::path::{Path, PathBuf};

impl PdfTools {
    /// Resolve Poppler once in the host, before conversion clears the child environment.
    /// Explicit per-tool overrides are authoritative; invalid overrides never fall back.
    /// Finder does not inherit shell PATH, so common Homebrew prefixes are also checked.
    pub fn discover() -> Option<Self> {
        let mut directories: Vec<_> = std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect())
            .unwrap_or_default();
        directories.extend([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ]);
        if let Some(home) = std::env::var_os("HOME") {
            directories.push(PathBuf::from(home).join(".local/bin"));
        }
        discover(
            std::env::var_os("BRN_PDFTOTEXT").map(PathBuf::from),
            std::env::var_os("BRN_PDFIMAGES").map(PathBuf::from),
            &directories,
        )
    }
}

fn discover(
    text: Option<PathBuf>,
    images: Option<PathBuf>,
    directories: &[PathBuf],
) -> Option<PdfTools> {
    fn resolve(explicit: Option<PathBuf>, name: &str, directories: &[PathBuf]) -> Option<PathBuf> {
        if let Some(path) = explicit {
            return executable(&path).then_some(path);
        }
        directories
            .iter()
            .map(|dir| dir.join(name))
            .find(|path| executable(path))
    }
    Some(PdfTools {
        pdftotext: resolve(text, "pdftotext", directories)?,
        pdfimages: resolve(images, "pdfimages", directories)?,
    })
}

pub(crate) fn executable(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(directory: &Path, name: &str) -> PathBuf {
        std::fs::create_dir_all(directory).unwrap();
        let path = directory.join(name);
        std::fs::write(&path, b"synthetic executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        path
    }

    #[test]
    fn individual_override_wins_and_other_tool_uses_path() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("path bin");
        tool(&directory, "pdftotext");
        let images = tool(&directory, "pdfimages");
        let text = tool(&root.path().join("configured bin"), "custom-text");
        let result = discover(Some(text.clone()), None, &[directory]).unwrap();
        assert_eq!(result.pdftotext, text);
        assert_eq!(result.pdfimages, images);
    }

    #[test]
    fn invalid_explicit_tool_never_silently_falls_back() {
        let root = tempfile::tempdir().unwrap();
        tool(root.path(), "pdftotext");
        tool(root.path(), "pdfimages");
        for invalid in [root.path().join("missing"), PathBuf::from("pdftotext")] {
            assert!(discover(Some(invalid), None, &[root.path().to_owned()]).is_none());
        }
    }

    #[test]
    fn limited_finder_path_uses_homebrew_fallback_in_order() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("finder/bin");
        let standard = root.path().join("standard/bin");
        let local = root.path().join("home/.local/bin");
        let text = tool(&local, "pdftotext");
        let images = tool(&local, "pdfimages");
        let result =
            discover(None, None, &[path.clone(), standard.clone(), local.clone()]).unwrap();
        assert_eq!(result.pdftotext, text);
        assert_eq!(result.pdfimages, images);
        let preferred = tool(&path, "pdftotext");
        let result = discover(None, None, &[path, standard, local]).unwrap();
        assert_eq!(result.pdftotext, preferred);
    }

    #[test]
    fn incomplete_pair_is_unavailable_and_relative_path_entries_are_ignored() {
        let root = tempfile::tempdir().unwrap();
        tool(root.path(), "pdftotext");
        assert!(discover(None, None, &[root.path().to_owned()]).is_none());
        assert!(!executable(Path::new("pdftotext")));
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_files_are_ignored() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("pdftotext"), b"not executable").unwrap();
        tool(root.path(), "pdfimages");
        assert!(discover(None, None, &[root.path().to_owned()]).is_none());
    }
}
