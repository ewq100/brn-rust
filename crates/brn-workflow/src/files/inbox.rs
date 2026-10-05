//! Private ordinary originals and immutable capture mirrors. No vault authority.
use crate::{ErrorKind, Result, WorkflowError};
use brn_store::work::inbox::{InboxCopy, InboxItem};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InboxRoot {
    pub path: PathBuf,
    pub data_device: u64,
    pub data_inode: u64,
    pub device: u64,
    pub inode: u64,
}
fn unavailable(message: impl Into<String>) -> WorkflowError {
    WorkflowError::typed(ErrorKind::InboxUnavailable, message)
}
#[cfg(target_os = "macos")]
fn stage(id: Uuid) -> String {
    format!(".brn-inbox-{id}.stage")
}
#[cfg(target_os = "macos")]
fn receipt(id: Uuid) -> String {
    format!(".brn-inbox-{id}.receipt")
}
#[cfg(target_os = "macos")]
fn original(id: Uuid) -> String {
    format!("{id}.txt")
}
pub(crate) fn receipt_id(name: &str) -> Option<Uuid> {
    let raw = name.strip_prefix(".brn-inbox-")?.strip_suffix(".receipt")?;
    let id = Uuid::parse_str(raw).ok()?;
    (!id.is_nil() && id.to_string() == raw).then_some(id)
}

#[cfg(test)]
thread_local! {
    pub(crate) static FAULT: std::cell::Cell<Option<(&'static str, bool)>> = const { std::cell::Cell::new(None) };
}
pub(crate) fn checkpoint(_step: &str) -> Result<()> {
    #[cfg(test)]
    if let Some((step, crash)) = FAULT.with(|f| f.get())
        && step == _step
    {
        if crash {
            std::process::exit(91);
        }
        return Err(WorkflowError::typed(
            ErrorKind::InboxUncertain,
            "injected Inbox checkpoint failure",
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use crate::files::{
        cstring, full_sync, open_at, open_directory, read_file, rename_flags, sync_directory,
    };
    use sha2::{Digest, Sha256};
    use std::{
        ffi::{CStr, OsStr},
        fs::{File, Metadata},
        io::{Read, Write},
        os::{
            fd::{AsRawFd, IntoRawFd},
            unix::fs::MetadataExt,
        },
        path::Component,
    };
    const MAX_MIRROR_BYTES: usize = 64 * 1024;
    const MAX_ENTRIES: usize = 16_384;

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Envelope {
        format: u8,
        sha256: [u8; 32],
        item: InboxItem,
    }
    fn digest(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }
    fn io(error: std::io::Error) -> WorkflowError {
        unavailable(error.to_string())
    }
    fn file_error(error: crate::files::FileFailure) -> WorkflowError {
        unavailable(error.message)
    }
    fn uid() -> u32 {
        // SAFETY: geteuid has no preconditions.
        unsafe { libc::geteuid() }
    }
    fn checked_directory(path: &Path) -> Result<File> {
        if !path.is_absolute() {
            return Err(unavailable("Inbox data path must be absolute"));
        }
        let mut dir = open_directory(Path::new("/")).map_err(file_error)?;
        for part in path.components().skip(1) {
            let Component::Normal(name) = part else {
                return Err(unavailable("invalid Inbox data path"));
            };
            dir = open_at(&dir, name, libc::O_DIRECTORY, 0).map_err(file_error)?;
        }
        let m = dir.metadata().map_err(io)?;
        if !m.is_dir() || m.uid() != uid() {
            return Err(unavailable(
                "Inbox data directory must be owned by the current user",
            ));
        }
        Ok(dir)
    }
    fn private_directory(m: &Metadata) -> Result<()> {
        if !m.is_dir() || m.uid() != uid() || m.mode() & 0o777 != 0o700 {
            return Err(unavailable("Inbox needs an owned private directory"));
        }
        Ok(())
    }
    fn private_file(m: &Metadata, max: usize) -> Result<()> {
        if !m.is_file()
            || m.uid() != uid()
            || m.nlink() != 1
            || m.mode() & 0o777 != 0o600
            || m.len() > max as u64
        {
            return Err(unavailable(
                "Inbox needs a bounded owned private single-link regular file",
            ));
        }
        Ok(())
    }
    fn same(before: &Metadata, after: &Metadata) -> bool {
        (
            before.dev(),
            before.ino(),
            before.len(),
            before.mtime(),
            before.mtime_nsec(),
            before.ctime(),
            before.ctime_nsec(),
        ) == (
            after.dev(),
            after.ino(),
            after.len(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec(),
        )
    }
    fn encode(item: &InboxItem) -> Result<Vec<u8>> {
        item.validate()?;
        let encoded =
            serde_json::to_vec(item).map_err(|_| unavailable("could not encode Inbox capture"))?;
        let bytes = serde_json::to_vec(&Envelope {
            format: 1,
            sha256: digest(&encoded),
            item: item.clone(),
        })
        .map_err(|_| unavailable("could not encode Inbox mirror"))?;
        if bytes.len() > MAX_MIRROR_BYTES {
            return Err(unavailable("Inbox mirror exceeds its bound"));
        }
        Ok(bytes)
    }
    pub(crate) struct InboxFiles {
        root: InboxRoot,
        data: File,
        directory: File,
    }
    impl InboxFiles {
        /// Opening never adopts an occupied unbound namespace for new capture.
        /// Read-only recovery may inspect it, then bind only a fully proved mirror.
        pub(crate) fn open(
            data: &Path,
            bound: Option<&InboxRoot>,
            create: bool,
        ) -> Result<Option<Self>> {
            let parent = checked_directory(data)?;
            let pm = parent.metadata().map_err(io)?;
            let name = OsStr::new("inbox");
            let directory = match open_at(&parent, name, libc::O_DIRECTORY, 0) {
                Ok(dir) if create && bound.is_none() => {
                    drop(dir);
                    return Err(unavailable(
                        "unbound Inbox namespace is occupied; retained artifacts need inspection",
                    ));
                }
                Ok(dir) => dir,
                Err(error)
                    if error.code == crate::files::FileErrorCode::Missing
                        && bound.is_none()
                        && !create =>
                {
                    return Ok(None);
                }
                Err(error)
                    if error.code == crate::files::FileErrorCode::Missing
                        && bound.is_none()
                        && create =>
                {
                    let name = cstring(Path::new("inbox")).map_err(file_error)?;
                    // SAFETY: parent is retained and the fixed name is NUL-terminated.
                    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
                        return Err(io(std::io::Error::last_os_error()));
                    }
                    sync_directory(&parent).map_err(file_error)?;
                    open_at(&parent, OsStr::new("inbox"), libc::O_DIRECTORY, 0)
                        .map_err(file_error)?
                }
                Err(error) => return Err(file_error(error)),
            };
            let m = directory.metadata().map_err(io)?;
            private_directory(&m)?;
            if m.dev() != pm.dev() {
                return Err(unavailable("Inbox must share its owned data volume"));
            }
            let root = InboxRoot {
                path: data.join("inbox"),
                data_device: pm.dev(),
                data_inode: pm.ino(),
                device: m.dev(),
                inode: m.ino(),
            };
            if bound.is_some_and(|b| *b != root) {
                return Err(unavailable("Inbox directory identity changed"));
            }
            let result = Self {
                root,
                data: parent,
                directory,
            };
            result.validate()?;
            Ok(Some(result))
        }
        pub(crate) fn root(&self) -> &InboxRoot {
            &self.root
        }
        pub(crate) fn validate(&self) -> Result<()> {
            let parent = checked_directory(
                self.root
                    .path
                    .parent()
                    .ok_or_else(|| unavailable("invalid Inbox root"))?,
            )?;
            let pm = parent.metadata().map_err(io)?;
            let retained = self.data.metadata().map_err(io)?;
            if (pm.dev(), pm.ino()) != (self.root.data_device, self.root.data_inode)
                || (pm.dev(), pm.ino()) != (retained.dev(), retained.ino())
            {
                return Err(unavailable("Inbox data parent identity changed"));
            }
            let current =
                open_at(&parent, OsStr::new("inbox"), libc::O_DIRECTORY, 0).map_err(file_error)?;
            let m = current.metadata().map_err(io)?;
            private_directory(&m)?;
            let held = self.directory.metadata().map_err(io)?;
            private_directory(&held)?;
            if (m.dev(), m.ino()) != (self.root.device, self.root.inode)
                || (m.dev(), m.ino()) != (held.dev(), held.ino())
            {
                return Err(unavailable("Inbox directory identity changed"));
            }
            Ok(())
        }
        fn absent(&self, name: &str) -> Result<bool> {
            match open_at(&self.directory, OsStr::new(name), libc::O_RDONLY, 0) {
                Ok(_) => Ok(false),
                Err(e) if e.code == crate::files::FileErrorCode::Missing => Ok(true),
                Err(e) => Err(file_error(e)),
            }
        }
        pub(crate) fn prepare(&self, id: Uuid, text: &str) -> Result<InboxCopy> {
            if id.is_nil() || text.len() > crate::MAX_NOTE_BYTES {
                return Err(unavailable("invalid original-copy request"));
            }
            self.validate()?;
            for name in [original(id), stage(id), receipt(id)] {
                if !self.absent(&name)? {
                    return Err(unavailable(
                        "Inbox capture target or recovery artifact is occupied",
                    ));
                }
            }
            let mut file = open_at(
                &self.directory,
                OsStr::new(&stage(id)),
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
                0o600,
            )
            .map_err(file_error)?;
            file.write_all(text.as_bytes()).map_err(io)?;
            full_sync(&file).map_err(file_error)?;
            sync_directory(&self.directory).map_err(file_error)?;
            private_file(&file.metadata().map_err(io)?, crate::MAX_NOTE_BYTES)?;
            let written = read_file(&file).map_err(file_error)?;
            let observed = self.observe_name(&stage(id))?;
            if observed.fingerprint != written.fingerprint || observed.text != text {
                return Err(unavailable(
                    "Inbox staged original differs from explicit input",
                ));
            }
            self.validate()?;
            checkpoint("original_durable")?;
            let f = observed.fingerprint;
            Ok(InboxCopy {
                directory: self.root.path.clone(),
                directory_device: self.root.device,
                directory_inode: self.root.inode,
                file_device: f.device,
                file_inode: f.inode,
                byte_len: f.len,
                sha256: f.sha256,
            })
        }
        fn validate_item(&self, item: &InboxItem) -> Result<()> {
            item.validate()?;
            let c = &item.capture.copy;
            if c.directory != self.root.path
                || (c.directory_device, c.directory_inode) != (self.root.device, self.root.inode)
            {
                return Err(unavailable("Inbox capture belongs to another directory"));
            }
            Ok(())
        }
        fn observe_name(&self, name: &str) -> Result<crate::files::FileObservation> {
            self.validate()?;
            let file = open_at(&self.directory, OsStr::new(name), libc::O_RDONLY, 0)
                .map_err(file_error)?;
            private_file(&file.metadata().map_err(io)?, crate::MAX_NOTE_BYTES)?;
            let result = read_file(&file).map_err(file_error)?;
            private_file(&file.metadata().map_err(io)?, crate::MAX_NOTE_BYTES)?;
            let current = open_at(&self.directory, OsStr::new(name), libc::O_RDONLY, 0)
                .map_err(file_error)?;
            let m = current.metadata().map_err(io)?;
            private_file(&m, crate::MAX_NOTE_BYTES)?;
            if (m.dev(), m.ino()) != (result.fingerprint.device, result.fingerprint.inode) {
                return Err(unavailable("Inbox filename changed during observation"));
            }
            self.validate()?;
            Ok(result)
        }
        fn exact(&self, item: &InboxItem, name: &str) -> Result<String> {
            self.validate_item(item)?;
            let observed = self.observe_name(name)?;
            let c = &item.capture.copy;
            let f = observed.fingerprint;
            if (f.device, f.inode, f.len, f.sha256)
                != (c.file_device, c.file_inode, c.byte_len, c.sha256)
            {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "Inbox original identity or bytes changed",
                ));
            }
            Ok(observed.text)
        }
        pub(crate) fn read(&self, item: &InboxItem) -> Result<Option<String>> {
            self.validate_item(item)?;
            self.validate()?;
            let name = original(item.capture.id);
            if self.absent(&name)? {
                return Ok(None);
            }
            self.exact(item, &name).map(Some)
        }
        /// Publish immutable proof before any installation. Unknown effects are
        /// preserved; equal bytes never substitute for the captured file identity.
        pub(crate) fn publish(&self, item: &InboxItem) -> Result<()> {
            self.validate_item(item)?;
            self.exact(item, &stage(item.capture.id))?;
            if !self.absent(&original(item.capture.id))? {
                return Err(unavailable("Inbox original destination is occupied"));
            }
            let bytes = encode(item)?;
            let temp = format!(".brn-inbox-mirror-{}.stage", Uuid::new_v4());
            let mut file = open_at(
                &self.directory,
                OsStr::new(&temp),
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
                0o600,
            )
            .map_err(file_error)?;
            file.write_all(&bytes).map_err(io)?;
            full_sync(&file).map_err(file_error)?;
            sync_directory(&self.directory).map_err(file_error)?;
            self.validate()?;
            let named = open_at(&self.directory, OsStr::new(&temp), libc::O_RDONLY, 0)
                .map_err(file_error)?;
            let held = file.metadata().map_err(io)?;
            private_file(&held, MAX_MIRROR_BYTES)?;
            if !same(&held, &named.metadata().map_err(io)?) {
                return Err(unavailable("Inbox mirror staging filename changed"));
            }
            rename_flags(
                &self.directory,
                &cstring(Path::new(&temp)).map_err(file_error)?,
                &cstring(Path::new(&receipt(item.capture.id))).map_err(file_error)?,
                libc::RENAME_EXCL,
            )
            .map_err(file_error)?;
            full_sync(&file).map_err(file_error)?;
            sync_directory(&self.directory).map_err(file_error)?;
            self.validate()?;
            let retained = self.mirror(item.capture.id)?;
            if retained != *item {
                return Err(unavailable("Inbox mirror differs after publication"));
            }
            checkpoint("mirror_durable")?;
            self.recover(item)?;
            Ok(())
        }
        pub(crate) fn mirror(&self, id: Uuid) -> Result<InboxItem> {
            self.validate()?;
            let name = receipt(id);
            let mut file = open_at(&self.directory, OsStr::new(&name), libc::O_RDONLY, 0)
                .map_err(file_error)?;
            let before = file.metadata().map_err(io)?;
            private_file(&before, MAX_MIRROR_BYTES)?;
            let mut bytes = Vec::new();
            Read::by_ref(&mut file)
                .take((MAX_MIRROR_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(io)?;
            let after = file.metadata().map_err(io)?;
            private_file(&after, MAX_MIRROR_BYTES)?;
            if bytes.len() > MAX_MIRROR_BYTES
                || !same(&before, &after)
                || bytes.len() as u64 != after.len()
            {
                return Err(unavailable(
                    "Inbox mirror changed during bounded observation",
                ));
            }
            let envelope: Envelope = serde_json::from_slice(&bytes)
                .map_err(|_| unavailable("invalid Inbox recovery mirror"))?;
            if envelope.format != 1
                || envelope.item.capture.id != id
                || encode(&envelope.item)? != bytes
            {
                return Err(unavailable(
                    "Inbox mirror format, hash or immutable binding differs",
                ));
            }
            let current = open_at(&self.directory, OsStr::new(&name), libc::O_RDONLY, 0)
                .map_err(file_error)?;
            if !same(&after, &current.metadata().map_err(io)?) {
                return Err(unavailable(
                    "Inbox mirror filename changed during observation",
                ));
            }
            self.validate_item(&envelope.item)?;
            self.validate()?;
            Ok(envelope.item)
        }
        fn flush_original(&self, item: &InboxItem) -> Result<()> {
            let name = original(item.capture.id);
            self.exact(item, &name)?;
            let file = open_at(&self.directory, OsStr::new(&name), libc::O_RDONLY, 0)
                .map_err(file_error)?;
            let m = file.metadata().map_err(io)?;
            if (m.dev(), m.ino()) != (item.capture.copy.file_device, item.capture.copy.file_inode) {
                return Err(unavailable(
                    "Inbox original changed before durability confirmation",
                ));
            }
            full_sync(&file).map_err(file_error)?;
            sync_directory(&self.directory).map_err(file_error)?;
            self.exact(item, &name)?;
            self.validate()
        }
        pub(crate) fn recover(&self, item: &InboxItem) -> Result<()> {
            self.validate_item(item)?;
            if self.mirror(item.capture.id)? != *item {
                return Err(unavailable("Inbox recovery mirror differs from its item"));
            }
            let target = original(item.capture.id);
            let prepared = stage(item.capture.id);
            if !self.absent(&target)? {
                if !self.absent(&prepared)? {
                    return Err(unavailable(
                        "Inbox recovery has occupied original and stage endpoints",
                    ));
                }
                self.flush_original(item)?;
                return self.confirm_mirror(item);
            }
            self.exact(item, &prepared)?;
            self.validate()?;
            rename_flags(
                &self.directory,
                &cstring(Path::new(&prepared)).map_err(file_error)?,
                &cstring(Path::new(&target)).map_err(file_error)?,
                libc::RENAME_EXCL,
            )
            .map_err(file_error)?;
            self.flush_original(item)?;
            checkpoint("original_installed")?;
            self.confirm_mirror(item)
        }
        fn confirm_mirror(&self, item: &InboxItem) -> Result<()> {
            if self.mirror(item.capture.id)? != *item {
                return Err(unavailable("Inbox mirror changed before acknowledgement"));
            }
            let file = open_at(
                &self.directory,
                OsStr::new(&receipt(item.capture.id)),
                libc::O_RDONLY,
                0,
            )
            .map_err(file_error)?;
            private_file(&file.metadata().map_err(io)?, MAX_MIRROR_BYTES)?;
            full_sync(&file).map_err(file_error)?;
            sync_directory(&self.directory).map_err(file_error)?;
            if self.mirror(item.capture.id)? != *item {
                return Err(unavailable(
                    "Inbox mirror changed during durability confirmation",
                ));
            }
            self.validate()
        }
        /// Descriptor-relative bounded inventory. Unknown entries remain intact.
        pub(crate) fn names(&self) -> Result<Vec<String>> {
            self.validate()?;
            let duplicate = open_at(&self.directory, OsStr::new("."), libc::O_DIRECTORY, 0)
                .map_err(file_error)?
                .into_raw_fd();
            // SAFETY: duplicate is uniquely owned; fdopendir consumes it on success.
            let directory = unsafe { libc::fdopendir(duplicate) };
            if directory.is_null() {
                // SAFETY: fdopendir failed and left ownership with this caller.
                unsafe { libc::close(duplicate) };
                return Err(io(std::io::Error::last_os_error()));
            }
            let result = (|| {
                let mut names = Vec::new();
                loop {
                    // SAFETY: macOS errno storage is thread-local; DIR remains live.
                    unsafe { *libc::__error() = 0 };
                    let entry = unsafe { libc::readdir(directory) };
                    if entry.is_null() {
                        if unsafe { *libc::__error() } != 0 {
                            return Err(io(std::io::Error::last_os_error()));
                        }
                        break;
                    }
                    // SAFETY: readdir returned a live NUL-terminated name until next read.
                    let raw = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
                    if raw == b"." || raw == b".." {
                        continue;
                    }
                    let name = std::str::from_utf8(raw)
                        .map_err(|_| unavailable("Inbox contains an unknown non-UTF8 entry"))?;
                    if names.len() == MAX_ENTRIES {
                        return Err(unavailable(
                            "Inbox inventory exceeds 16384 entries; originals remain retained",
                        ));
                    }
                    names.push(name.to_owned());
                }
                names.sort();
                self.validate()?;
                Ok(names)
            })();
            // SAFETY: this is the sole DIR owner, closed once after enumeration.
            unsafe { libc::closedir(directory) };
            result
        }
    }
}
#[cfg(target_os = "macos")]
pub(crate) use platform::InboxFiles;

#[cfg(not(target_os = "macos"))]
pub(crate) struct InboxFiles;
#[cfg(not(target_os = "macos"))]
impl InboxFiles {
    pub(crate) fn open(_: &Path, _: Option<&InboxRoot>, create: bool) -> Result<Option<Self>> {
        if create {
            Err(unavailable("Inbox copy durability requires macOS"))
        } else {
            Ok(None)
        }
    }
    pub(crate) fn root(&self) -> &InboxRoot {
        unreachable!("unsupported Inbox files cannot open")
    }
    pub(crate) fn prepare(&self, _: Uuid, _: &str) -> Result<InboxCopy> {
        Err(unavailable("Inbox copy durability requires macOS"))
    }
    pub(crate) fn publish(&self, _: &InboxItem) -> Result<()> {
        Err(unavailable("Inbox copy durability requires macOS"))
    }
    pub(crate) fn mirror(&self, _: Uuid) -> Result<InboxItem> {
        Err(unavailable("Inbox copy durability requires macOS"))
    }
    pub(crate) fn recover(&self, _: &InboxItem) -> Result<()> {
        Err(unavailable("Inbox copy durability requires macOS"))
    }
    pub(crate) fn read(&self, _: &InboxItem) -> Result<Option<String>> {
        Err(unavailable("Inbox copy reads require macOS"))
    }
    pub(crate) fn names(&self) -> Result<Vec<String>> {
        Err(unavailable("Inbox copy reads require macOS"))
    }
}
