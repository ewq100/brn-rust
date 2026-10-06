pub(crate) mod inbox;
#[cfg(target_os = "macos")]
mod macos;
pub(crate) mod recovery;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileErrorCode {
    #[cfg(target_os = "macos")]
    Conflict,
    #[cfg(target_os = "macos")]
    Missing,
    Unsupported,
    #[cfg(target_os = "macos")]
    SaveUncertain,
    #[cfg(target_os = "macos")]
    Io,
    #[cfg(target_os = "macos")]
    VaultBusy,
    #[cfg(target_os = "macos")]
    VaultUnavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOutcome {
    NotApplied,
    #[cfg(target_os = "macos")]
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileFailure {
    pub code: FileErrorCode,
    pub message: String,
    pub filesystem_outcome: FileOutcome,
}
impl std::fmt::Display for FileFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for FileFailure {}
pub(crate) type FileResult<T> = std::result::Result<T, FileFailure>;

#[cfg(target_os = "macos")]
use brn_store::files::{ArtifactIdentity, ArtifactKind};
use brn_store::files::{FileFingerprint, PreparedFile, RetainedArtifact, VaultRecord};
#[cfg(target_os = "macos")]
use std::{collections::VecDeque, path::PathBuf};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[cfg(all(test, target_os = "macos"))]
thread_local! {
    pub(crate) static PREPARE_FAILURE: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}

#[cfg(all(test, target_os = "macos"))]
fn prepare_failure(step: &str) -> FileResult<()> {
    if PREPARE_FAILURE.with(|selected| selected.get() == Some(step)) {
        Err(failure(FileErrorCode::Io, "injected staging I/O failure"))
    } else {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteNoticeKind {
    Changed,
    Moved,
    Deleted,
    RescanRequired,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteFileNotice {
    pub vault_id: Uuid,
    pub relative_path: Option<PathBuf>,
    pub kind: NoteNoticeKind,
}

pub(crate) type NoteNoticeSink = Arc<Mutex<NoteNoticeQueue>>;

#[derive(Debug, Default)]
pub(crate) struct NoteNoticeQueue {
    #[cfg(target_os = "macos")]
    pending: VecDeque<NoteFileNotice>,
}

#[cfg(target_os = "macos")]
impl NoteNoticeQueue {
    const CAPACITY: usize = 256;

    pub(crate) fn push(&mut self, notice: NoteFileNotice) {
        if self.pending.iter().any(|old| {
            old.vault_id == notice.vault_id
                && (old.kind == NoteNoticeKind::RescanRequired || *old == notice)
        }) {
            return;
        }
        if notice.kind == NoteNoticeKind::RescanRequired || self.pending.len() == Self::CAPACITY {
            self.pending.clear();
            self.pending.push_back(NoteFileNotice {
                vault_id: notice.vault_id,
                relative_path: None,
                kind: NoteNoticeKind::RescanRequired,
            });
        } else {
            self.pending.push_back(notice);
        }
    }

    #[cfg(test)]
    pub(crate) fn drain(&mut self) -> Vec<NoteFileNotice> {
        self.pending.drain(..).collect()
    }
}

#[derive(Debug)]
pub(crate) struct FileObservation {
    pub fingerprint: FileFingerprint,
    pub text: String,
}

#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(crate) struct FileByteObservation {
    pub fingerprint: FileFingerprint,
    pub bytes: Vec<u8>,
}

fn failure(code: FileErrorCode, message: impl Into<String>) -> FileFailure {
    FileFailure {
        code,
        message: message.into(),
        filesystem_outcome: FileOutcome::NotApplied,
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn note_io_failure(error: std::io::Error) -> FileFailure {
    let code = match error.raw_os_error() {
        #[cfg(target_os = "macos")]
        Some(libc::ELOOP | libc::ENOTDIR | libc::ENOTSUP | libc::ENOSYS) => {
            FileErrorCode::Unsupported
        }
        _ if error.kind() == std::io::ErrorKind::NotFound => FileErrorCode::Missing,
        _ => FileErrorCode::Io,
    };
    failure(code, error.to_string())
}

pub(crate) fn note_unsupported(message: &str) -> FileFailure {
    failure(FileErrorCode::Unsupported, message)
}

#[cfg(target_os = "macos")]
pub(crate) fn note_utf8_failure(error: std::string::FromUtf8Error) -> FileFailure {
    note_unsupported(&format!("note is not UTF-8: {error}"))
}

#[cfg(target_os = "macos")]
use {
    self::macos::Coordination,
    brn_store::files::VaultIdentity,
    sha2::{Digest, Sha256},
    std::{
        ffi::{CString, OsStr},
        fs::{File, Metadata},
        io::{Read, Seek, Write},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{ffi::OsStrExt, fs::MetadataExt},
        },
        path::Component,
    },
};

#[cfg(target_os = "macos")]
pub(crate) struct MacFiles {
    // Drop the presenter before releasing the ownership descriptors.
    coordination: Coordination,
    registered_root: PathBuf,
    root: PathBuf,
    identity: VaultIdentity,
    locks: Vec<File>,
}

#[cfg(target_os = "macos")]
impl MacFiles {
    pub(crate) fn open(
        vault: &VaultRecord,
        data_dir: &Path,
        notices: NoteNoticeSink,
    ) -> FileResult<Self> {
        let root = std::fs::canonicalize(&vault.root)
            .map_err(|error| failure(FileErrorCode::VaultUnavailable, error.to_string()))?;
        let data = std::fs::canonicalize(data_dir).map_err(note_io_failure)?;
        if data.starts_with(&root) {
            return Err(note_unsupported(
                "application data directory is inside the vault",
            ));
        }
        let mut locks = Vec::new();
        let unavailable =
            |error: FileFailure| failure(FileErrorCode::VaultUnavailable, error.message);
        let mut directory = open_directory(Path::new("/")).map_err(unavailable)?;
        for component in root.components().skip(1) {
            lock_directory(&directory, false)?;
            locks.push(directory);
            let Component::Normal(name) = component else {
                return Err(note_unsupported("invalid canonical vault root"));
            };
            directory =
                open_at(locks.last().unwrap(), name, libc::O_DIRECTORY, 0).map_err(unavailable)?;
        }
        let metadata = directory.metadata().map_err(note_io_failure)?;
        if identity(&metadata) != vault.identity {
            return Err(failure(
                FileErrorCode::VaultUnavailable,
                "registered vault root changed",
            ));
        }
        qualify_volume(&directory)?;
        lock_directory(&directory, true)?;
        locks.push(directory);
        let coordination = Coordination::new(vault.id, &root, notices)?;
        let files = Self {
            coordination,
            registered_root: vault.root.clone(),
            root,
            identity: vault.identity.clone(),
            locks,
        };
        files.validate_root()?;
        Ok(files)
    }

    fn root_directory(&self) -> &File {
        self.locks.last().unwrap()
    }

    pub(crate) fn validate_root(&self) -> FileResult<()> {
        let changed = || {
            failure(
                FileErrorCode::VaultUnavailable,
                "registered vault root unavailable or changed",
            )
        };
        let canonical = std::fs::canonicalize(&self.registered_root).map_err(|error| {
            failure(
                FileErrorCode::VaultUnavailable,
                format!("registered vault root unavailable: {error}"),
            )
        })?;
        if canonical != self.root {
            return Err(changed());
        }
        let file = open_directory(&canonical).map_err(|error| {
            failure(
                FileErrorCode::VaultUnavailable,
                format!("registered vault root unavailable: {}", error.message),
            )
        })?;
        if identity(&file.metadata().map_err(note_io_failure)?) != self.identity {
            return Err(changed());
        }
        Ok(())
    }

    fn parent(&self, relative: &Path) -> FileResult<(File, CString)> {
        self.validate_root()?;
        validate_relative(relative)?;
        let mut directory = self.root_directory().try_clone().map_err(note_io_failure)?;
        let mut components = relative.components().peekable();
        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                return Err(note_unsupported(
                    "note path must be a contained relative path",
                ));
            };
            if components.peek().is_none() {
                return Ok((directory, cstring(Path::new(name))?));
            }
            directory = open_at(&directory, name, libc::O_DIRECTORY, 0)?;
            if directory.metadata().map_err(note_io_failure)?.dev() != self.identity.device {
                return Err(note_unsupported("note path crosses a filesystem boundary"));
            }
        }
        Err(note_unsupported("empty note path"))
    }

    fn validate_parent(&self, relative: &Path, parent: &File) -> FileResult<()> {
        let (current, _) = self.parent(relative)?;
        if identity(&current.metadata().map_err(note_io_failure)?)
            != identity(&parent.metadata().map_err(note_io_failure)?)
        {
            return Err(failure(FileErrorCode::Conflict, "note parent changed"));
        }
        Ok(())
    }

    pub(crate) fn observe(&self, relative: &Path) -> FileResult<FileObservation> {
        self.parent(relative)?;
        self.coordination.read(&self.root.join(relative), || {
            self.observe_uncoordinated(relative)
        })
    }

    pub(crate) fn observe_uncoordinated(&self, relative: &Path) -> FileResult<FileObservation> {
        let (parent, name) = self.parent(relative)?;
        let file = open_at(&parent, OsStr::from_bytes(name.as_bytes()), 0, 0)?;
        if file.metadata().map_err(note_io_failure)?.dev() != self.identity.device {
            return Err(note_unsupported("note crosses a filesystem boundary"));
        }
        let observation = read_file(&file)?;
        self.validate_parent(relative, &parent)?;
        let current = open_at(&parent, OsStr::from_bytes(name.as_bytes()), 0, 0)?;
        if identity(&current.metadata().map_err(note_io_failure)?)
            != (VaultIdentity {
                device: observation.fingerprint.device,
                inode: observation.fingerprint.inode,
            })
        {
            return Err(failure(
                FileErrorCode::Conflict,
                "note replaced during observation",
            ));
        }
        Ok(observation)
    }

    pub(crate) fn coordinate<T>(
        &self,
        relative: &Path,
        action: impl FnOnce() -> FileResult<T>,
    ) -> FileResult<T> {
        self.parent(relative)?;
        self.coordination.write(&self.root.join(relative), || {
            self.validate_root()?;
            action()
        })
    }

    pub(crate) fn prepare_replace(
        &self,
        op: Uuid,
        staging: &Path,
        destination: &Path,
        bytes: &[u8],
    ) -> FileResult<PreparedFile> {
        if bytes.len() > crate::MAX_NOTE_BYTES || std::str::from_utf8(bytes).is_err() {
            return Err(note_unsupported(
                "submitted note must be UTF-8 and at most 1 MiB",
            ));
        }
        let expected = format!(".brn-{op}.stage");
        if staging.file_name() != Some(OsStr::new(&expected))
            || staging.parent() != destination.parent()
            || staging == destination
        {
            return Err(note_unsupported(
                "staging must use the operation-owned non-Markdown path in the destination parent",
            ));
        }
        let (parent, destination_name) = self.parent(destination)?;
        validate_relative(staging)?;
        let original = open_at(
            &parent,
            OsStr::from_bytes(destination_name.as_bytes()),
            0,
            0,
        )?;
        validate_regular(&original.metadata().map_err(note_io_failure)?)?;
        let mut stage = open_at(
            &parent,
            staging.file_name().unwrap(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )?;
        #[cfg(test)]
        #[cfg(test)]
        prepare_failure("write")?;
        stage.write_all(bytes).map_err(note_io_failure)?;
        #[cfg(test)]
        prepare_failure("attributes")?;
        preserve_attributes(&original, &stage)?;
        #[cfg(test)]
        prepare_failure("file_sync")?;
        full_sync(&stage)?;
        #[cfg(test)]
        prepare_failure("directory_sync")?;
        sync_directory(&parent)?;
        self.validate_parent(destination, &parent)?;
        let observed = read_file(&stage)?;
        if observed.text.as_bytes() != bytes {
            return Err(failure(FileErrorCode::Conflict, "prepared bytes changed"));
        }
        let prepared = PreparedFile {
            relative: staging.to_owned(),
            fingerprint: observed.fingerprint,
        };
        if self.observe_uncoordinated(staging)?.fingerprint != prepared.fingerprint {
            return Err(failure(
                FileErrorCode::Conflict,
                "prepared path identity changed",
            ));
        }
        Ok(prepared)
    }

    pub(crate) fn parent_identity(
        &self,
        relative: &Path,
    ) -> FileResult<brn_store::files::VaultIdentity> {
        let (parent, _) = self.parent(relative)?;
        Ok(identity(&parent.metadata().map_err(note_io_failure)?))
    }

    pub(crate) fn validate_copy_destination(&self, relative: &Path) -> FileResult<()> {
        component_key(relative).map(|_| ())
    }

    /// A reserved namespace cannot be enrolled merely because its occupant changed.
    /// This guard does not apply destination-name restrictions to ordinary opens.
    pub(crate) fn reserved_copy_path_matches(
        &self,
        candidate: &Path,
        reserved: &Path,
    ) -> FileResult<bool> {
        validate_relative(candidate)?;
        validate_relative(reserved)?;
        if candidate == reserved {
            return Ok(true);
        }
        let (Ok(candidate_key), Ok(reserved_key)) =
            (component_key(candidate), component_key(reserved))
        else {
            return Ok(false);
        };
        if candidate_key.last() != reserved_key.last() {
            return Ok(false);
        }
        let candidate_parent = self.parent_identity(candidate)?;
        match self.parent_identity(reserved) {
            Ok(parent) => Ok(parent == candidate_parent),
            Err(error)
                if matches!(
                    error.code,
                    FileErrorCode::Missing | FileErrorCode::Unsupported
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    /// Equality here is a conservative veto, never proof of logical note identity.
    pub(crate) fn aliases_original(&self, candidate: &Path, original: &Path) -> FileResult<bool> {
        let candidate_key = component_key(candidate)?;
        let optional = |path: &Path| -> FileResult<Option<VaultIdentity>> {
            let result = (|| {
                let (parent, name) = self.parent(path)?;
                let file = open_at(&parent, OsStr::from_bytes(name.as_bytes()), 0, 0)?;
                let metadata = file.metadata().map_err(note_io_failure)?;
                validate_regular(&metadata)?;
                self.validate_parent(path, &parent)?;
                let current = open_at(&parent, OsStr::from_bytes(name.as_bytes()), 0, 0)?;
                if identity(&current.metadata().map_err(note_io_failure)?) != identity(&metadata) {
                    return Err(failure(
                        FileErrorCode::Conflict,
                        "file replaced during alias validation",
                    ));
                }
                Ok(identity(&metadata))
            })();
            match result {
                Ok(value) => Ok(Some(value)),
                Err(error)
                    if matches!(
                        error.code,
                        FileErrorCode::Missing | FileErrorCode::Unsupported
                    ) =>
                {
                    Ok(None)
                }
                Err(error) => Err(error),
            }
        };
        if let Some(left) = optional(candidate)?
            && let Some(right) = optional(original)?
        {
            return Ok(left == right);
        }
        let original_key = match component_key(original) {
            Ok(key) => key,
            // Registered names need not meet the stricter destination rules.
            // An unqualified basename can only alias within the same parent.
            // An unresolved original parent cannot disprove that possibility.
            Err(error) if error.code == FileErrorCode::Unsupported => {
                let candidate_parent = self.parent_identity(candidate)?;
                return Ok(match self.parent_identity(original) {
                    Ok(parent) => parent == candidate_parent,
                    Err(_) => true,
                });
            }
            Err(error) => return Err(error),
        };
        if candidate_key == original_key {
            return Ok(true);
        }
        let candidate_parent = self.parent_identity(candidate)?;
        match self.parent_identity(original) {
            Ok(parent)
                if parent == candidate_parent && candidate_key.last() == original_key.last() =>
            {
                return Ok(true);
            }
            Ok(_) => (),
            Err(error) if error.code == FileErrorCode::Missing => {
                // A vanished parent cannot be resolved; veto a potentially moved
                // equivalent basename rather than infer a distinct namespace.
                if candidate_key.last() == original_key.last() {
                    return Ok(true);
                }
            }
            Err(error) => return Err(error),
        }
        Ok(false)
    }

    pub(crate) fn prepare_copy(
        &self,
        op: Uuid,
        staging: &Path,
        destination: &Path,
        bytes: &[u8],
    ) -> FileResult<PreparedFile> {
        if bytes.len() > crate::MAX_NOTE_BYTES || std::str::from_utf8(bytes).is_err() {
            return Err(note_unsupported(
                "submitted note must be UTF-8 and at most 1 MiB",
            ));
        }
        let expected = format!(".brn-{op}.stage");
        if staging.file_name() != Some(OsStr::new(&expected))
            || staging.parent() != destination.parent()
            || staging == destination
        {
            return Err(note_unsupported(
                "copy staging must use its operation-owned sibling path",
            ));
        }
        let (parent, _) = self.parent(destination)?;
        let mut stage = open_at(
            &parent,
            staging.file_name().unwrap(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )?;
        #[cfg(test)]
        #[cfg(test)]
        prepare_failure("write")?;
        stage.write_all(bytes).map_err(note_io_failure)?;
        #[cfg(test)]
        prepare_failure("file_sync")?;
        full_sync(&stage)?;
        #[cfg(test)]
        prepare_failure("directory_sync")?;
        sync_directory(&parent)?;
        self.validate_parent(destination, &parent)?;
        let observed = read_file(&stage)?;
        if observed.text.as_bytes() != bytes
            || self.observe_uncoordinated(staging)?.fingerprint != observed.fingerprint
        {
            return Err(failure(FileErrorCode::Conflict, "prepared copy changed"));
        }
        Ok(PreparedFile {
            relative: staging.to_owned(),
            fingerprint: observed.fingerprint,
        })
    }

    pub(crate) fn install_exclusive(
        &self,
        prepared: &PreparedFile,
        destination: &Path,
    ) -> FileResult<()> {
        if prepared.relative.parent() != destination.parent() || prepared.relative == destination {
            return Err(note_unsupported(
                "exclusive installation requires distinct sibling paths",
            ));
        }
        if self.observe_uncoordinated(&prepared.relative)?.fingerprint != prepared.fingerprint {
            return Err(failure(FileErrorCode::Conflict, "prepared copy changed"));
        }
        let (parent, target) = self.parent(destination)?;
        let (_, stage) = self.parent(&prepared.relative)?;
        self.validate_parent(destination, &parent)?;
        rename_flags(&parent, &stage, &target, libc::RENAME_EXCL).map_err(|error| {
            if error.code == FileErrorCode::Io
                && self
                    .artifact(destination)
                    .is_ok_and(|value| value.is_some())
            {
                failure(FileErrorCode::Conflict, "copy destination is occupied")
            } else {
                error
            }
        })
    }

    pub(crate) fn exchange(&self, prepared: &PreparedFile, destination: &Path) -> FileResult<()> {
        if prepared.relative.parent() != destination.parent() || prepared.relative == destination {
            return Err(note_unsupported(
                "exchange requires distinct paths in one validated parent",
            ));
        }
        if self.observe_uncoordinated(&prepared.relative)?.fingerprint != prepared.fingerprint {
            return Err(failure(
                FileErrorCode::Conflict,
                "prepared artifact changed",
            ));
        }
        let (parent, target) = self.parent(destination)?;
        let original = open_at(&parent, OsStr::from_bytes(target.as_bytes()), 0, 0)?;
        validate_regular(&original.metadata().map_err(note_io_failure)?)?;
        let (_, stage) = self.parent(&prepared.relative)?;
        self.validate_parent(destination, &parent)?;
        rename_flags(&parent, &stage, &target, libc::RENAME_SWAP)
    }

    pub(crate) fn flush_artifact(&self, relative: &Path) -> FileResult<()> {
        let (parent, name) = self.parent(relative)?;
        let artifact = open_at(&parent, OsStr::from_bytes(name.as_bytes()), 0, 0)?;
        validate_regular(&artifact.metadata().map_err(note_io_failure)?)?;
        full_sync(&artifact)?;
        sync_directory(&parent)?;
        self.validate_parent(relative, &parent)
    }

    pub(crate) fn artifact(&self, relative: &Path) -> FileResult<Option<RetainedArtifact>> {
        let (parent, name) = self.parent(relative)?;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: parent/name are live; fstatat initializes stat on success and never follows links.
        if unsafe {
            libc::fstatat(
                parent.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            let error = note_io_failure(std::io::Error::last_os_error());
            return if error.code == FileErrorCode::Missing {
                Ok(None)
            } else {
                Err(error)
            };
        }
        // SAFETY: successful fstatat initialized the entire struct.
        let stat = unsafe { stat.assume_init() };
        let kind = match stat.st_mode & libc::S_IFMT {
            libc::S_IFREG => ArtifactKind::Regular,
            libc::S_IFDIR => ArtifactKind::Directory,
            libc::S_IFLNK => ArtifactKind::Symlink,
            _ => ArtifactKind::Other,
        };
        let sha256 = if kind == ArtifactKind::Regular {
            match self.observe_uncoordinated(relative) {
                Ok(value)
                    if value.fingerprint.device == stat.st_dev as u64
                        && value.fingerprint.inode == stat.st_ino
                        && value.fingerprint.len == stat.st_size as u64 =>
                {
                    Some(value.fingerprint.sha256)
                }
                Ok(_) => {
                    return Err(failure(
                        FileErrorCode::Conflict,
                        "artifact replaced during observation",
                    ));
                }
                Err(error) if error.code == FileErrorCode::Unsupported => None,
                Err(error) => return Err(error),
            }
        } else {
            None
        };
        self.validate_parent(relative, &parent)?;
        Ok(Some(RetainedArtifact {
            relative: relative.to_owned(),
            identity: ArtifactIdentity {
                device: stat.st_dev as u64,
                inode: stat.st_ino,
                len: stat.st_size as u64,
                kind,
            },
            sha256,
        }))
    }

    pub(crate) fn remove_artifact(&self, expected: &RetainedArtifact) -> FileResult<()> {
        let (parent, name) = self.parent(&expected.relative)?;
        if self.artifact(&expected.relative)?.as_ref() != Some(expected) {
            return Err(failure(FileErrorCode::Conflict, "cleanup occupant changed"));
        }
        self.validate_parent(&expected.relative, &parent)?;
        // SAFETY: the proven regular artifact is relative to a validated parent; no recursive removal.
        if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(note_io_failure(std::io::Error::last_os_error()));
        }
        sync_directory(&parent)
    }
}

#[cfg(target_os = "macos")]
fn validate_relative(path: &Path) -> FileResult<()> {
    // Path::components normalizes internal "."; reject it before normalization.
    if path.as_os_str().is_empty()
        || path
            .as_os_str()
            .as_bytes()
            .split(|byte| *byte == b'/')
            .any(|part| part.is_empty() || part == b"." || part == b"..")
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(note_unsupported(
            "note path must be a contained relative path",
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn cstring(path: &Path) -> FileResult<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| note_unsupported("NUL in filesystem path"))
}

#[cfg(target_os = "macos")]
fn open_directory(path: &Path) -> FileResult<File> {
    let name = cstring(path)?;
    // SAFETY: name is NUL-terminated; the returned descriptor is owned exactly once.
    let fd = unsafe {
        libc::open(
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    owned_fd(fd)
}

#[cfg(target_os = "macos")]
fn open_at(parent: &File, name: &OsStr, flags: i32, mode: libc::mode_t) -> FileResult<File> {
    let name = cstring(Path::new(name))?;
    // SAFETY: parent is live and name is NUL-terminated. O_NONBLOCK avoids blocking on FIFOs.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            mode as libc::c_int,
        )
    };
    owned_fd(fd)
}

#[cfg(target_os = "macos")]
fn owned_fd(fd: i32) -> FileResult<File> {
    if fd < 0 {
        return Err(note_io_failure(std::io::Error::last_os_error()));
    }
    // SAFETY: fd is a newly opened descriptor; this transfers its sole ownership.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(target_os = "macos")]
fn identity(metadata: &Metadata) -> VaultIdentity {
    VaultIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(target_os = "macos")]
fn lock_directory(directory: &File, exclusive: bool) -> FileResult<()> {
    let kind = if exclusive {
        libc::LOCK_EX
    } else {
        libc::LOCK_SH
    };
    // SAFETY: the descriptor remains live and retained for the adapter lifetime.
    if unsafe { libc::flock(directory.as_raw_fd(), kind | libc::LOCK_NB) } != 0 {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::WouldBlock {
            failure(
                FileErrorCode::VaultBusy,
                "vault or overlapping root is owned by another workspace",
            )
        } else {
            note_io_failure(error)
        });
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn validate_regular(metadata: &Metadata) -> FileResult<()> {
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(note_unsupported("note must be a single-link regular file"));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn read_file(file: &File) -> FileResult<FileObservation> {
    let observation = read_file_bytes(file, crate::MAX_NOTE_BYTES, "note exceeds 1 MiB")?;
    Ok(FileObservation {
        fingerprint: observation.fingerprint,
        text: String::from_utf8(observation.bytes).map_err(note_utf8_failure)?,
    })
}

#[cfg(target_os = "macos")]
fn read_file_bytes(
    mut file: &File,
    max: usize,
    limit_message: &str,
) -> FileResult<FileByteObservation> {
    let before = file.metadata().map_err(note_io_failure)?;
    validate_regular(&before)?;
    file.rewind().map_err(note_io_failure)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((max + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(note_io_failure)?;
    if bytes.len() > max {
        return Err(note_unsupported(limit_message));
    }
    let after = file.metadata().map_err(note_io_failure)?;
    validate_regular(&after)?;
    if identity(&before) != identity(&after)
        || before.len() != after.len()
        || after.len() != bytes.len() as u64
        || (
            before.mtime(),
            before.mtime_nsec(),
            before.ctime(),
            before.ctime_nsec(),
        ) != (
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec(),
        )
    {
        return Err(failure(
            FileErrorCode::Conflict,
            "note changed during bounded observation",
        ));
    }
    let fingerprint = FileFingerprint {
        device: after.dev(),
        inode: after.ino(),
        len: after.len(),
        sha256: Sha256::digest(&bytes).into(),
    };
    Ok(FileByteObservation { fingerprint, bytes })
}

#[cfg(target_os = "macos")]
fn qualify_volume(directory: &File) -> FileResult<()> {
    #[repr(C)]
    struct Capabilities {
        length: u32,
        volume: libc::vol_capabilities_attr_t,
    }
    // SAFETY: these plain C structs admit all-zero initialization.
    let mut attributes: libc::attrlist = unsafe { std::mem::zeroed() };
    attributes.bitmapcount = libc::ATTR_BIT_MAP_COUNT;
    attributes.volattr = libc::ATTR_VOL_CAPABILITIES;
    // SAFETY: this plain C output struct admits zero initialization.
    let mut capabilities: Capabilities = unsafe { std::mem::zeroed() };
    // SAFETY: both buffers are correctly sized, aligned and live for the call.
    if unsafe {
        libc::fgetattrlist(
            directory.as_raw_fd(),
            (&mut attributes as *mut libc::attrlist).cast(),
            (&mut capabilities as *mut Capabilities).cast(),
            std::mem::size_of::<Capabilities>(),
            0,
        )
    } != 0
    {
        return Err(note_io_failure(std::io::Error::last_os_error()));
    }
    let index = libc::VOL_CAPABILITIES_INTERFACES;
    let required = libc::VOL_CAP_INT_RENAME_SWAP
        | libc::VOL_CAP_INT_RENAME_EXCL
        | libc::VOL_CAP_INT_FLOCK
        | libc::VOL_CAP_INT_EXTENDED_SECURITY
        | libc::VOL_CAP_INT_EXTENDED_ATTR;
    if capabilities.length as usize != std::mem::size_of::<Capabilities>()
        || capabilities.volume.valid[index] & required != required
        || capabilities.volume.capabilities[index] & required != required
    {
        return Err(note_unsupported(
            "volume does not advertise required exchange, exclusive rename, locking and attributes",
        ));
    }
    // SAFETY: statfs is a plain C output struct and the descriptor is live.
    let mut volume: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: volume is correctly aligned and sized for fstatfs.
    if unsafe { libc::fstatfs(directory.as_raw_fd(), &mut volume) } != 0 {
        return Err(note_io_failure(std::io::Error::last_os_error()));
    }
    if volume.f_flags & libc::MNT_LOCAL as u32 == 0 {
        return Err(note_unsupported(
            "only local vault filesystems are supported",
        ));
    }
    // Only these local volume families have qualified canonical-name equivalence.
    // The veto folds more than the volume may do (including case-sensitive APFS).
    let filesystem = unsafe { std::ffi::CStr::from_ptr(volume.f_fstypename.as_ptr()) };
    let format = libc::VOL_CAPABILITIES_FORMAT;
    let name_capabilities = libc::VOL_CAP_FMT_CASE_SENSITIVE | libc::VOL_CAP_FMT_CASE_PRESERVING;
    if !matches!(filesystem.to_bytes(), b"apfs" | b"hfs")
        || capabilities.volume.valid[format] & name_capabilities != name_capabilities
    {
        return Err(note_unsupported(
            "volume name equivalence is not qualified for safe copy reservations",
        ));
    }
    sync_directory(directory)
}

#[cfg(target_os = "macos")]
fn component_key(path: &Path) -> FileResult<Vec<String>> {
    validate_relative(path)?;
    path.components()
        .map(|component| {
            let Component::Normal(name) = component else {
                unreachable!()
            };
            let name = name
                .to_str()
                .ok_or_else(|| note_unsupported("copy path is not UTF-8"))?;
            // HFS has ignorable scalars and different historical Unicode tables. Refuse
            // invisible/control names rather than claiming a key proves equivalence.
            if name.chars().any(|c| {
                c.is_control()
                    || matches!(c as u32,
            0x00ad | 0x034f | 0x061c | 0x115f..=0x1160 | 0x17b4..=0x17b5 |
            0x180b..=0x180f | 0x200b..=0x200f | 0x202a..=0x202e | 0x2060..=0x206f |
            0x3164 | 0xfe00..=0xfe0f | 0xfeff | 0xffa0 | 0xfff0..=0xfff8 | 0x1bca0..=0x1bca3 |
            0x1d173..=0x1d17a | 0xe0000..=0xe0fff)
            }) {
                return Err(note_unsupported("ambiguous invisible copy-path component"));
            }
            canonical_case_fold(name)
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn canonical_case_fold(value: &str) -> FileResult<String> {
    use std::ffi::c_void;
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithBytes(
            allocator: *const c_void,
            bytes: *const u8,
            len: isize,
            encoding: u32,
            external: u8,
        ) -> *const c_void;
        fn CFStringCreateMutableCopy(
            allocator: *const c_void,
            capacity: isize,
            string: *const c_void,
        ) -> *mut c_void;
        fn CFStringNormalize(string: *mut c_void, form: isize);
        fn CFStringFold(string: *mut c_void, flags: usize, locale: *const c_void);
        fn CFStringGetLength(string: *const c_void) -> isize;
        fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
        fn CFStringGetCString(
            string: *const c_void,
            buffer: *mut i8,
            size: isize,
            encoding: u32,
        ) -> u8;
        fn CFRelease(value: *const c_void);
    }
    const UTF8: u32 = 0x08000100;
    // SAFETY: immutable UTF-8 input is valid for creation; owned CF objects are
    // released on every path. Normalize(D) + full case fold + Normalize(D) is a
    // canonical Unicode component veto, not NSString's lowercase-only mapping.
    unsafe {
        let string = CFStringCreateWithBytes(
            std::ptr::null(),
            value.as_ptr(),
            value.len() as isize,
            UTF8,
            0,
        );
        if string.is_null() {
            return Err(note_unsupported("cannot qualify Unicode copy name"));
        }
        let folded = CFStringCreateMutableCopy(std::ptr::null(), 0, string);
        CFRelease(string);
        if folded.is_null() {
            return Err(note_unsupported("cannot qualify Unicode copy name"));
        }
        CFStringNormalize(folded, 0);
        CFStringFold(folded, 1, std::ptr::null());
        CFStringNormalize(folded, 0);
        let size = CFStringGetMaximumSizeForEncoding(CFStringGetLength(folded), UTF8) + 1;
        let mut bytes = vec![0u8; size as usize];
        let success = CFStringGetCString(folded, bytes.as_mut_ptr().cast(), size, UTF8);
        CFRelease(folded);
        if success == 0 {
            return Err(note_unsupported("cannot qualify Unicode copy name"));
        }
        bytes.truncate(
            bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len()),
        );
        String::from_utf8(bytes).map_err(note_utf8_failure)
    }
}

#[cfg(target_os = "macos")]
fn preserve_attributes(source: &File, destination: &File) -> FileResult<()> {
    let mode = source.metadata().map_err(note_io_failure)?.mode() & 0o7777;
    // SAFETY: descriptors are live; no callback state, no data or COPYFILE_STAT copying.
    if unsafe {
        libc::fcopyfile(
            source.as_raw_fd(),
            destination.as_raw_fd(),
            std::ptr::null_mut(),
            libc::COPYFILE_ACL | libc::COPYFILE_XATTR,
        )
    } != 0
    {
        return Err(note_io_failure(std::io::Error::last_os_error()));
    }
    // SAFETY: destination is an owned live descriptor; only mode bits are copied.
    if unsafe { libc::fchmod(destination.as_raw_fd(), mode as libc::mode_t) } != 0 {
        return Err(note_io_failure(std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn full_sync(file: &File) -> FileResult<()> {
    // SAFETY: file is a live descriptor. No ordinary fsync fallback is permitted.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_FULLFSYNC) } != 0 {
        return Err(note_io_failure(std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn sync_directory(directory: &File) -> FileResult<()> {
    // Directory durability deliberately uses fsync; Apple's File::sync_all uses F_FULLFSYNC.
    // SAFETY: directory is a live descriptor retained throughout the call.
    if unsafe { libc::fsync(directory.as_raw_fd()) } != 0 {
        return Err(note_io_failure(std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn rename_flags(directory: &File, from: &CString, to: &CString, flags: u32) -> FileResult<()> {
    // SAFETY: both NUL-terminated names are relative to the same validated live directory.
    if unsafe {
        libc::renameatx_np(
            directory.as_raw_fd(),
            from.as_ptr(),
            directory.as_raw_fd(),
            to.as_ptr(),
            flags,
        )
    } != 0
    {
        return Err(note_io_failure(std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub(crate) struct MacFiles;

#[cfg(not(target_os = "macos"))]
impl MacFiles {
    pub(crate) fn open(_: &VaultRecord, _: &Path, _: NoteNoticeSink) -> FileResult<Self> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn validate_root(&self) -> FileResult<()> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn aliases_original(&self, _: &Path, _: &Path) -> FileResult<bool> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn parent_identity(&self, _: &Path) -> FileResult<brn_store::files::VaultIdentity> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn validate_copy_destination(&self, _: &Path) -> FileResult<()> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn reserved_copy_path_matches(&self, _: &Path, _: &Path) -> FileResult<bool> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn prepare_copy(
        &self,
        _: Uuid,
        _: &Path,
        _: &Path,
        _: &[u8],
    ) -> FileResult<PreparedFile> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn install_exclusive(&self, _: &PreparedFile, _: &Path) -> FileResult<()> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn observe(&self, _: &Path) -> FileResult<FileObservation> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn coordinate<T>(
        &self,
        _: &Path,
        _: impl FnOnce() -> FileResult<T>,
    ) -> FileResult<T> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn prepare_replace(
        &self,
        _: Uuid,
        _: &Path,
        _: &Path,
        _: &[u8],
    ) -> FileResult<PreparedFile> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn exchange(&self, _: &PreparedFile, _: &Path) -> FileResult<()> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn flush_artifact(&self, _: &Path) -> FileResult<()> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }

    pub(crate) fn artifact(&self, _: &Path) -> FileResult<Option<RetainedArtifact>> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn remove_artifact(&self, _: &RetainedArtifact) -> FileResult<()> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
    pub(crate) fn observe_uncoordinated(&self, _: &Path) -> FileResult<FileObservation> {
        Err(note_unsupported(
            "managed notes require macOS filesystem coordination",
        ))
    }
}

#[cfg(all(test, not(target_os = "macos")))]
mod unsupported_tests {
    use super::*;
    use brn_store::files::VaultIdentity;

    #[test]
    fn unsupported_adapter_refuses_before_creating_vault_or_data() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let vault = owner.path().join("vault-must-not-exist");
        let data = owner.path().join("data-must-not-exist");
        let record = VaultRecord {
            id: Uuid::new_v4(),
            root: vault.clone(),
            identity: VaultIdentity {
                device: 0,
                inode: 0,
            },
        };
        let failure = MacFiles::open(&record, &data, Arc::default())
            .err()
            .expect("filesystem coordination is unavailable on this platform");
        assert_eq!(failure.code, FileErrorCode::Unsupported);
        assert_eq!(failure.filesystem_outcome, FileOutcome::NotApplied);
        assert!(!vault.exists());
        assert!(!data.exists());
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::SUBPROCESS_FIXTURES as PROCESS_FIXTURES;
    use brn_store::files::{VaultIdentity, VaultRecord};
    use std::{
        os::unix::fs::MetadataExt,
        path::Path,
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;
    fn directory() -> tempfile::TempDir {
        tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
    }

    #[test]
    fn binary_observation_core_preserves_text_utf8_limit_and_fingerprint_behavior() {
        let data = directory();
        let path = data.path().join("opaque.bin");
        let bytes = [0, 0xff, 0x80, b'a'];
        std::fs::write(&path, bytes).unwrap();
        let file = File::open(&path).unwrap();
        let observed = read_file_bytes(&file, 16 * 1024 * 1024, "binary bound").unwrap();
        assert_eq!(observed.bytes, bytes);
        assert_eq!(
            observed.fingerprint.sha256,
            <[u8; 32]>::from(Sha256::digest(bytes))
        );
        assert_eq!(observed.fingerprint.len, 4);
        let error = read_file(&file).unwrap_err();
        assert_eq!(error.code, FileErrorCode::Unsupported);
        assert!(error.message.starts_with("note is not UTF-8:"));
        std::fs::write(&path, vec![b'x'; crate::MAX_NOTE_BYTES + 1]).unwrap();
        let error = read_file(&file).unwrap_err();
        assert_eq!(error.code, FileErrorCode::Unsupported);
        assert_eq!(error.message, "note exceeds 1 MiB");
        assert_eq!(
            read_file_bytes(&file, 16 * 1024 * 1024, "binary bound")
                .unwrap()
                .bytes
                .len(),
            crate::MAX_NOTE_BYTES + 1
        );
        std::fs::write(&path, b"BOM\r\nbody\n").unwrap();
        let text = read_file(&file).unwrap();
        let raw = read_file_bytes(&file, crate::MAX_NOTE_BYTES, "note exceeds 1 MiB").unwrap();
        assert_eq!(text.text.as_bytes(), raw.bytes);
        assert_eq!(text.fingerprint, raw.fingerprint);
        std::fs::hard_link(&path, data.path().join("alias")).unwrap();
        assert!(read_file_bytes(&file, 16 * 1024 * 1024, "binary bound").is_err());
    }

    fn registered(root: &Path) -> VaultRecord {
        let metadata = std::fs::metadata(root).unwrap();
        VaultRecord {
            id: Uuid::new_v4(),
            root: root.to_owned(),
            identity: VaultIdentity {
                device: metadata.dev(),
                inode: metadata.ino(),
            },
        }
    }

    #[test]
    fn ambiguous_registered_names_fall_back_without_rejecting_known_distinct_identities() {
        let vault = directory();
        let data = directory();
        let record = registered(vault.path());
        let files = MacFiles::open(
            &record,
            data.path(),
            Arc::new(Mutex::new(NoteNoticeQueue::default())),
        )
        .unwrap();
        std::fs::write(vault.path().join("plain.md"), "plain").unwrap();
        std::fs::create_dir(vault.path().join("different")).unwrap();
        for original in ["❤️.md", "👩‍💻.md"] {
            std::fs::write(vault.path().join(original), "emoji").unwrap();
            assert!(
                !files
                    .aliases_original(Path::new("plain.md"), Path::new(original))
                    .unwrap()
            );
            assert!(
                !files
                    .aliases_original(Path::new("different/copy.md"), Path::new(original))
                    .unwrap()
            );
            assert!(
                files
                    .aliases_original(Path::new("absent.md"), Path::new(original))
                    .unwrap()
            );
            assert_eq!(
                files
                    .aliases_original(Path::new(original), Path::new("plain.md"))
                    .unwrap_err()
                    .code,
                FileErrorCode::Unsupported
            );
            std::fs::remove_file(vault.path().join(original)).unwrap();
            assert!(
                !files
                    .aliases_original(Path::new("different/copy.md"), Path::new(original))
                    .unwrap()
            );
            assert!(
                files
                    .aliases_original(Path::new("absent.md"), Path::new(original))
                    .unwrap()
            );
            assert!(
                files
                    .aliases_original(
                        Path::new("different/copy.md"),
                        &Path::new("missing").join(original)
                    )
                    .unwrap()
            );
        }
    }

    #[test]
    fn copy_reservations_match_paths_without_requiring_matching_file_identity() {
        let vault = directory();
        let data = directory();
        let record = registered(vault.path());
        let files = MacFiles::open(
            &record,
            data.path(),
            Arc::new(Mutex::new(NoteNoticeQueue::default())),
        )
        .unwrap();
        std::fs::create_dir(vault.path().join("same")).unwrap();
        std::fs::create_dir(vault.path().join("different")).unwrap();
        // No recorded prepared identity or successful content observation is needed.
        std::fs::write(vault.path().join("same/rescue.md"), [0xff]).unwrap();
        assert!(
            files
                .reserved_copy_path_matches(
                    Path::new("same/rescue.md"),
                    Path::new("same/rescue.md")
                )
                .unwrap()
        );
        assert!(
            files
                .reserved_copy_path_matches(
                    Path::new("same/RESCUE.md"),
                    Path::new("same/rescue.md")
                )
                .unwrap()
        );
        assert!(
            !files
                .reserved_copy_path_matches(
                    Path::new("different/rescue.md"),
                    Path::new("same/rescue.md")
                )
                .unwrap()
        );
        assert!(
            !files
                .reserved_copy_path_matches(Path::new("same/❤️.md"), Path::new("same/rescue.md"))
                .unwrap()
        );
    }

    #[test]
    fn exact_observation_and_exclusive_ownership() {
        let vault = directory();
        let data = directory();
        std::fs::write(vault.path().join("plan.md"), b"\xef\xbb\xbf# plan\r\n").unwrap();
        let record = registered(vault.path());
        let notices = Arc::new(Mutex::new(NoteNoticeQueue::default()));
        let files = MacFiles::open(&record, data.path(), notices.clone()).unwrap();
        assert_eq!(
            files.observe(Path::new("plan.md")).unwrap().text.as_bytes(),
            b"\xef\xbb\xbf# plan\r\n"
        );
        assert_eq!(
            files
                .coordinate(Path::new("plan.md"), || files.observe(Path::new("plan.md")))
                .unwrap()
                .text
                .as_bytes(),
            b"\xef\xbb\xbf# plan\r\n"
        );
        assert_eq!(
            MacFiles::open(&record, data.path(), notices)
                .err()
                .unwrap()
                .code,
            FileErrorCode::VaultBusy
        );
    }

    fn adapter(vault: &Path, data: &Path) -> MacFiles {
        MacFiles::open(
            &registered(vault),
            data,
            Arc::new(Mutex::new(NoteNoticeQueue::default())),
        )
        .unwrap()
    }

    #[test]
    fn bounded_inputs_and_unsafe_locations() {
        use std::os::unix::fs::symlink;
        let vault = directory();
        let data = directory();
        let files = adapter(vault.path(), data.path());
        for bytes in [
            vec![],
            vec![b'x'; crate::MAX_NOTE_BYTES],
            b"\xef\xbb\xbf\r\n".to_vec(),
        ] {
            std::fs::write(vault.path().join("plan.md"), &bytes).unwrap();
            assert_eq!(
                files.observe(Path::new("plan.md")).unwrap().text.as_bytes(),
                bytes
            );
        }
        for bytes in [vec![b'x'; crate::MAX_NOTE_BYTES + 1], vec![0xff]] {
            std::fs::write(vault.path().join("plan.md"), bytes).unwrap();
            assert_eq!(
                files.observe(Path::new("plan.md")).unwrap_err().code,
                FileErrorCode::Unsupported
            );
        }
        std::fs::write(vault.path().join("plan.md"), b"valid").unwrap();
        std::fs::hard_link(vault.path().join("plan.md"), vault.path().join("hard.md")).unwrap();
        assert_eq!(
            files.observe(Path::new("plan.md")).unwrap_err().code,
            FileErrorCode::Unsupported
        );
        symlink(data.path(), vault.path().join("escape")).unwrap();
        symlink("hard.md", vault.path().join("link.md")).unwrap();
        std::fs::create_dir(vault.path().join("directory.md")).unwrap();
        for path in [
            "../outside.md",
            "/outside.md",
            "escape/outside.md",
            "link.md",
            "directory.md",
            "./hard.md",
            "",
        ] {
            assert!(files.observe(Path::new(path)).is_err(), "{path}");
        }
        assert_eq!(
            files.observe(Path::new("missing.md")).unwrap_err().code,
            FileErrorCode::Missing
        );
    }

    #[test]
    fn root_identity_data_containment_and_namespace_changes() {
        let vault = directory();
        let data = directory();
        std::fs::create_dir(vault.path().join("data")).unwrap();
        assert_eq!(
            MacFiles::open(
                &registered(vault.path()),
                &vault.path().join("data"),
                Arc::default()
            )
            .err()
            .unwrap()
            .code,
            FileErrorCode::Unsupported
        );
        let mut record = registered(vault.path());
        record.identity.inode += 1;
        assert_eq!(
            MacFiles::open(&record, data.path(), Arc::default())
                .err()
                .unwrap()
                .code,
            FileErrorCode::VaultUnavailable
        );
        let outer = directory();
        let root = outer.path().join("vault");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("plan.md"), b"old").unwrap();
        let record = registered(&root);
        let files = adapter(&root, data.path());
        std::fs::rename(&root, outer.path().join("moved")).unwrap();
        std::fs::create_dir(&root).unwrap();
        assert_eq!(
            files.observe(Path::new("plan.md")).unwrap_err().code,
            FileErrorCode::VaultUnavailable
        );
        std::fs::remove_dir(&root).unwrap();
        std::fs::write(&root, b"not a vault directory").unwrap();
        assert_eq!(
            MacFiles::open(&record, data.path(), Arc::default())
                .err()
                .unwrap()
                .code,
            FileErrorCode::VaultUnavailable
        );
    }

    #[test]
    fn prepare_exchange_preserves_displaced_object_and_attributes() {
        let _process_fixtures = PROCESS_FIXTURES.lock().unwrap();
        use std::os::unix::fs::PermissionsExt;
        let vault = directory();
        let data = directory();
        let target = vault.path().join("plan.md");
        std::fs::write(&target, b"original").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();
        let status = std::process::Command::new("/usr/bin/xattr")
            .args(["-w", "org.brn.synthetic", "kept"])
            .arg(&target)
            .status()
            .unwrap();
        assert!(status.success());
        let status = std::process::Command::new("/bin/chmod")
            .args(["+a", "everyone allow read"])
            .arg(&target)
            .status()
            .unwrap();
        assert!(status.success());
        let original = std::fs::metadata(&target).unwrap();
        let files = adapter(vault.path(), data.path());
        let op = Uuid::new_v4();
        let stage = PathBuf::from(format!(".brn-{op}.stage"));
        let prepared = files
            .prepare_replace(op, &stage, Path::new("plan.md"), b"submitted")
            .unwrap();
        assert_eq!(prepared.relative, stage);
        assert_eq!(
            prepared.fingerprint.sha256,
            <[u8; 32]>::from(Sha256::digest(b"submitted"))
        );
        assert_eq!(
            files
                .prepare_replace(op, &stage, Path::new("plan.md"), b"submitted")
                .unwrap_err()
                .code,
            FileErrorCode::Io
        );
        files
            .coordinate(Path::new("plan.md"), || {
                files.exchange(&prepared, Path::new("plan.md"))
            })
            .unwrap();
        files.flush_artifact(Path::new("plan.md")).unwrap();
        files.flush_artifact(&stage).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"submitted");
        assert_eq!(
            std::fs::read(vault.path().join(&stage)).unwrap(),
            b"original"
        );
        assert_eq!(
            std::fs::metadata(vault.path().join(&stage)).unwrap().ino(),
            original.ino()
        );
        assert_eq!(
            std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
            0o640
        );
        let installed = std::fs::metadata(&target).unwrap();
        assert_ne!(
            (installed.mtime(), installed.mtime_nsec()),
            (original.mtime(), original.mtime_nsec())
        );
        assert_eq!(
            std::process::Command::new("/usr/bin/xattr")
                .args(["-p", "org.brn.synthetic"])
                .arg(&target)
                .output()
                .unwrap()
                .stdout,
            b"kept\n"
        );
        let acl = std::process::Command::new("/bin/ls")
            .arg("-le")
            .arg(&target)
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&acl.stdout).contains("everyone allow read"));
        std::process::Command::new("/bin/chmod")
            .arg("-N")
            .arg(&target)
            .status()
            .unwrap();
        std::process::Command::new("/bin/chmod")
            .arg("-N")
            .arg(vault.path().join(&stage))
            .status()
            .unwrap();
        println!("APFS: prepare F_FULLFSYNC, directory fsync, RENAME_SWAP, mode/ACL/xattrs passed");
    }

    #[test]
    fn missing_exchange_tampered_stage_and_failures_stop() {
        let vault = directory();
        let data = directory();
        std::fs::write(vault.path().join("plan.md"), b"old").unwrap();
        let files = adapter(vault.path(), data.path());
        let op = Uuid::new_v4();
        let stage = PathBuf::from(format!(".brn-{op}.stage"));
        let prepared = files
            .prepare_replace(op, &stage, Path::new("plan.md"), b"new")
            .unwrap();
        std::fs::remove_file(vault.path().join("plan.md")).unwrap();
        assert!(files.exchange(&prepared, Path::new("plan.md")).is_err());
        assert_eq!(std::fs::read(vault.path().join(&stage)).unwrap(), b"new");
        std::fs::write(vault.path().join("plan.md"), b"later").unwrap();
        std::fs::write(vault.path().join(&stage), b"tampered").unwrap();
        assert_eq!(
            files
                .exchange(&prepared, Path::new("plan.md"))
                .unwrap_err()
                .code,
            FileErrorCode::Conflict
        );
        assert_eq!(
            std::fs::read(vault.path().join("plan.md")).unwrap(),
            b"later"
        );
        assert!(files.flush_artifact(Path::new("missing")).is_err());
        assert!(
            files
                .prepare_replace(
                    Uuid::new_v4(),
                    Path::new("wrong.md"),
                    Path::new("plan.md"),
                    b"new"
                )
                .is_err()
        );
        let result: FileResult<()> = files.coordinate(Path::new("plan.md"), || {
            Err(note_unsupported("accessor stopped"))
        });
        assert_eq!(result.unwrap_err().message, "accessor stopped");
    }

    #[test]
    fn external_atomic_replacement_is_freshly_observed() {
        let vault = directory();
        let data = directory();
        std::fs::write(vault.path().join("plan.md"), b"old").unwrap();
        let files = adapter(vault.path(), data.path());
        let before = files.observe(Path::new("plan.md")).unwrap();
        std::fs::write(vault.path().join("external"), b"new").unwrap();
        std::fs::rename(vault.path().join("external"), vault.path().join("plan.md")).unwrap();
        let after = files.observe(Path::new("plan.md")).unwrap();
        assert_eq!(after.text, "new");
        assert_ne!(before.fingerprint.inode, after.fingerprint.inode);
    }

    #[test]
    fn subprocess_ownership_child() {
        let Some(root) = std::env::var_os("BRN_NOTE_CHILD_ROOT") else {
            return;
        };
        let data = directory();
        let result = MacFiles::open(&registered(Path::new(&root)), data.path(), Arc::default());
        assert_eq!(result.err().unwrap().code, FileErrorCode::VaultBusy);
        println!("independent process reports VaultBusy");
    }

    #[test]
    fn same_nested_and_aliased_roots_exclude_other_processes() {
        let _process_fixtures = PROCESS_FIXTURES.lock().unwrap();
        use std::os::unix::fs::symlink;
        let outer = directory();
        let data = directory();
        let root = outer.path().join("vault");
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let alias = outer.path().join("alias");
        symlink(&root, &alias).unwrap();
        let run = |path: &Path| {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "files::tests::subprocess_ownership_child",
                    "--nocapture",
                ])
                .env("BRN_NOTE_CHILD_ROOT", path)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stdout)
                    .contains("independent process reports VaultBusy")
            );
        };
        let files = adapter(&root, data.path());
        run(&root);
        run(&nested);
        run(&alias);
        drop(files);
        let files = adapter(&nested, data.path());
        run(&root);
        drop(files);
        let _reopened = adapter(&alias, data.path());
        println!(
            "APFS directory flock: same/nested/reverse-nested/aliased roots excluded across subprocesses"
        );
    }

    #[test]
    fn explicit_directory_fsync_and_failure_propagation() {
        let vault = directory();
        let parent = open_directory(vault.path()).unwrap();
        sync_directory(&parent).unwrap();

        let mut descriptors = [-1; 2];
        // SAFETY: the two-element output buffer is live and correctly sized.
        assert_eq!(unsafe { libc::pipe(descriptors.as_mut_ptr()) }, 0);
        // SAFETY: pipe returned two new descriptors; each is owned exactly once.
        let (reader, _writer) = unsafe {
            (
                File::from_raw_fd(descriptors[0]),
                File::from_raw_fd(descriptors[1]),
            )
        };
        // macOS versions differ: fsync on a pipe reports EINVAL or ENOTSUP.
        // Compare against this kernel's direct error, while still requiring failure.
        // SAFETY: reader owns a live pipe descriptor throughout both calls.
        assert_eq!(unsafe { libc::fsync(reader.as_raw_fd()) }, -1);
        let error = std::io::Error::last_os_error();
        assert!(matches!(
            error.raw_os_error(),
            Some(libc::EINVAL | libc::ENOTSUP)
        ));
        assert_eq!(sync_directory(&reader).unwrap_err(), note_io_failure(error));
        println!("APFS: explicit libc::fsync accepted read-only directory; pipe error propagated");
    }

    #[test]
    fn physical_exclusive_rename_and_required_failure() {
        let _process_fixtures = PROCESS_FIXTURES.lock().unwrap();
        let vault = directory();
        std::fs::write(vault.path().join("a"), b"one").unwrap();
        std::fs::write(vault.path().join("b"), b"two").unwrap();
        let directory = open_directory(vault.path()).unwrap();
        assert!(
            rename_flags(
                &directory,
                &cstring(Path::new("a")).unwrap(),
                &cstring(Path::new("b")).unwrap(),
                libc::RENAME_EXCL
            )
            .is_err()
        );
        assert_eq!(std::fs::read(vault.path().join("b")).unwrap(), b"two");
        rename_flags(
            &directory,
            &cstring(Path::new("a")).unwrap(),
            &cstring(Path::new("c")).unwrap(),
            libc::RENAME_EXCL,
        )
        .unwrap();
        assert_eq!(std::fs::read(vault.path().join("c")).unwrap(), b"one");
        let readonly = std::fs::File::open(vault.path().join("b")).unwrap();
        let readonly_destination = std::fs::File::open(vault.path().join("c")).unwrap();
        let destination = vault.path().join("c");
        assert!(
            std::process::Command::new("/usr/bin/chflags")
                .arg("uchg")
                .arg(&destination)
                .status()
                .unwrap()
                .success()
        );
        let attribute_result = preserve_attributes(&readonly, &readonly_destination);
        assert!(
            std::process::Command::new("/usr/bin/chflags")
                .arg("nouchg")
                .arg(&destination)
                .status()
                .unwrap()
                .success()
        );
        assert!(attribute_result.is_err());
        assert!(full_sync(&std::fs::File::open("/dev/null").unwrap()).is_err());
        println!(
            "APFS RENAME_EXCL: collision rejected, absent destination installed; unsupported full sync rejected"
        );
    }
}
