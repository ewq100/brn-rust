//! Descriptor-bound, read-only snapshots for the explicit inbox scan.
use crate::{AppError, AppResult};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path},
};

pub const MAX_INBOX_ENTRIES: usize = 256;
pub const MAX_INBOX_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_INBOX_SCAN_BYTES: u64 = 64 * 1024 * 1024;

pub(super) struct Folder {
    file: File,
    path: std::path::PathBuf,
    pub identity: String,
}

#[cfg(unix)]
fn open_at(parent: &File, name: &std::ffi::OsStr, directory: bool) -> AppResult<File> {
    use std::os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::OsStrExt,
    };
    let name = std::ffi::CString::new(name.as_bytes())
        .map_err(|_| AppError::Invalid("Invalid inbox filename".into()))?;
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // The parent descriptor stays open; the returned descriptor has unique ownership.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(unix)]
fn identity(meta: &fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!("{}:{}", meta.dev(), meta.ino())
}
#[cfg(unix)]
fn stamp(meta: &fs::Metadata) -> (String, u64, i64, i64, i64, i64) {
    use std::os::unix::fs::MetadataExt;
    (
        identity(meta),
        meta.len(),
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec(),
    )
}

impl Folder {
    #[cfg(unix)]
    pub fn open(path: &Path, workspace: &Path) -> AppResult<Self> {
        if !path.is_absolute() || path.to_str().is_none_or(|s| s.len() > 4096) {
            return Err(AppError::Invalid(
                "Choose an absolute inbox folder with a supported name".into(),
            ));
        }
        let mut file = File::open("/")?;
        for component in path.components() {
            match component {
                Component::RootDir => {}
                Component::Normal(name) => file = open_at(&file, name, true)?,
                _ => {
                    return Err(AppError::Invalid(
                        "Choose the real inbox folder without aliases".into(),
                    ));
                }
            }
        }
        let real = path.canonicalize()?;
        let workspace = workspace.canonicalize()?;
        if real != path || real.starts_with(&workspace) || workspace.starts_with(&real) {
            return Err(AppError::Invalid(
                "Choose an inbox separate from BRN's data folder".into(),
            ));
        }
        let identity = identity(&file.metadata()?);
        let folder = Self {
            file,
            path: real,
            identity,
        };
        folder.check_location()?;
        Ok(folder)
    }
    #[cfg(unix)]
    pub fn check_location(&self) -> AppResult<()> {
        // Rewalk every component without following links, including ancestors.
        let mut file = File::open("/")?;
        for component in self.path.components() {
            if let Component::Normal(name) = component {
                file = open_at(&file, name, true)?;
            }
        }
        if identity(&file.metadata()?) != self.identity {
            return Err(AppError::Invalid(
                "The inbox folder changed; choose it again".into(),
            ));
        }
        Ok(())
    }
    pub fn names(&self) -> AppResult<Vec<String>> {
        self.check_location()?;
        let directory = rustix::fs::Dir::read_from(&self.file).map_err(std::io::Error::from)?;
        let names = read_names(directory)?;
        self.check_location()?;
        Ok(names)
    }
    #[cfg(unix)]
    pub fn read(
        &self,
        name: &str,
        max_bytes: u64,
        consumed: &mut u64,
    ) -> AppResult<(Vec<u8>, [u8; 32])> {
        let path = Path::new(name);
        if path.components().count() != 1
            || !matches!(path.components().next(), Some(Component::Normal(_)))
        {
            return Err(AppError::Invalid("Invalid inbox filename".into()));
        }
        self.check_location()?;
        let file = open_at(&self.file, path.as_os_str(), false)?;
        let before = file.metadata()?;
        if !before.is_file() || before.len() == 0 || before.len() > MAX_INBOX_FILE_BYTES {
            return Err(AppError::Invalid(
                "Only nonempty ordinary files up to 16 MiB can be imported".into(),
            ));
        }
        if before.len() > max_bytes {
            return Err(AppError::Invalid(
                "Source-byte scan limit reached; use a smaller folder to progress".into(),
            ));
        }
        let mut bytes = Vec::new();
        let read_result = (&file)
            .take(max_bytes.min(MAX_INBOX_FILE_BYTES))
            .read_to_end(&mut bytes);
        *consumed += bytes.len() as u64;
        read_result?;
        if bytes.len() as u64 != before.len() || stamp(&before) != stamp(&file.metadata()?) {
            return Err(AppError::Invalid(
                "File changed while being read; scan again after saving finishes".into(),
            ));
        }
        let current = open_at(&self.file, path.as_os_str(), false)?;
        if stamp(&before) != stamp(&current.metadata()?) {
            return Err(AppError::Invalid(
                "File was replaced while being read; scan again".into(),
            ));
        }
        self.check_location()?;
        let hash = Sha256::digest(&bytes).into();
        Ok((bytes, hash))
    }
}

fn read_names(directory: rustix::fs::Dir) -> AppResult<Vec<String>> {
    let mut names = Vec::new();
    for entry in directory {
        let entry = entry.map_err(std::io::Error::from)?;
        let name = entry.file_name().to_str().map_err(|_| {
            AppError::Invalid("Inbox contains a filename that cannot be displayed".into())
        })?;
        if matches!(name, "." | "..") {
            continue;
        }
        if names.len() == MAX_INBOX_ENTRIES {
            return Err(AppError::Invalid(format!(
                "Inbox has more than {MAX_INBOX_ENTRIES} entries; split it into smaller folders"
            )));
        }
        names.push(name.to_owned());
    }
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directory_listing_uses_pinned_descriptor_during_path_swap() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("data");
        let path = root.path().join("inbox");
        let outside = root.path().join("outside");
        for p in [&data, &path, &outside] {
            fs::create_dir(p).unwrap();
        }
        fs::write(path.join("authorized.txt"), "Inside").unwrap();
        fs::write(outside.join("private-name.txt"), "Outside").unwrap();
        let folder = Folder::open(&path, &data).unwrap();
        let directory = rustix::fs::Dir::read_from(&folder.file).unwrap();
        fs::rename(&path, root.path().join("held")).unwrap();
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        assert_eq!(read_names(directory).unwrap(), vec!["authorized.txt"]);
        assert!(folder.names().is_err());
        fs::remove_file(&path).unwrap();
        fs::rename(root.path().join("held"), &path).unwrap();
        assert_eq!(folder.names().unwrap(), vec!["authorized.txt"]);
        assert!(
            folder
                .read("../outside/private-name.txt", MAX_INBOX_FILE_BYTES, &mut 0)
                .is_err()
        );
        assert_eq!(
            folder
                .read("authorized.txt", MAX_INBOX_FILE_BYTES, &mut 0)
                .unwrap()
                .0,
            b"Inside"
        );
    }
}
