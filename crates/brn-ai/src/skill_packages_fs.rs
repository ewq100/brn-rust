//! Descriptor-anchored operations for the instruction-package folder only.
//! No path is reopened through an attacker-replaceable ancestor.
use std::{
    ffi::{CStr, CString, OsStr},
    fs::File,
    io,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path},
};
fn c(name: &OsStr) -> io::Result<CString> {
    CString::new(name.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in package path"))
}
fn result(fd: i32) -> io::Result<File> {
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: the successful open/dup transfers one owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}
pub(super) struct Dir(pub File);
impl Dir {
    pub fn absolute(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "skill folders must be absolute",
            ));
        }
        let mut dir = Self(File::open("/")?);
        for component in path.components() {
            match component {
                Component::RootDir => {}
                Component::Normal(part) => {
                    if part
                        .to_str()
                        .is_none_or(|s| s.chars().any(char::is_control))
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "invalid package path",
                        ));
                    }
                    dir = dir.child_os(part)?;
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "nonliteral package path",
                    ));
                }
            }
        }
        Ok(dir)
    }
    fn child_os(&self, name: &OsStr) -> io::Result<Self> {
        let name = c(name)?;
        // SAFETY: openat is anchored; NOFOLLOW refuses links, DIRECTORY/FIFO
        // checks do not require opening an arbitrary stream.
        result(unsafe {
            libc::openat(
                self.0.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY
                    | libc::O_DIRECTORY
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
            )
        })
        .map(Self)
    }
    pub fn mkdir_new(&self, name: &str) -> io::Result<()> {
        let name = c(OsStr::new(name))?;
        if unsafe { libc::mkdirat(self.0.as_raw_fd(), name.as_ptr(), 0o700) } == 0 {
            self.0.sync_all()
        } else {
            Err(io::Error::last_os_error())
        }
    }
    pub fn child(&self, name: &str) -> io::Result<Self> {
        self.child_os(OsStr::new(name))
    }
    pub fn file(&self, name: &str, flags: i32) -> io::Result<File> {
        let name = c(OsStr::new(name))?;
        // SAFETY: all callers pass one host-validated component.
        let file = result(unsafe {
            libc::openat(
                self.0.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                0o600,
            )
        })?;
        let meta = file.metadata()?;
        if !meta.is_file() || meta.nlink() != 1 {
            return Err(io::Error::other(
                "package files must be regular and not hard-linked",
            ));
        }
        Ok(file)
    }
    pub fn names(&self, limit: usize) -> io::Result<Vec<String>> {
        // Opening '.' gives an independent directory offset (dup would share it).
        let copy = self.child(".")?;
        use std::os::fd::IntoRawFd;
        // SAFETY: fdopendir takes ownership on success; close on failure.
        let fd = copy.0.into_raw_fd();
        let ptr = unsafe { libc::fdopendir(fd) };
        if ptr.is_null() {
            let e = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(e);
        }
        struct Entries(*mut libc::DIR);
        impl Drop for Entries {
            fn drop(&mut self) {
                unsafe {
                    libc::closedir(self.0);
                }
            }
        }
        let entries = Entries(ptr);
        let mut names = Vec::new();
        loop {
            // SAFETY: valid directory stream, pointer used before next read.
            #[cfg(target_os = "macos")]
            let errno = unsafe { libc::__error() };
            #[cfg(target_os = "linux")]
            let errno = unsafe { libc::__errno_location() };
            unsafe {
                *errno = 0;
            }
            let entry = unsafe { libc::readdir(entries.0) };
            if entry.is_null() {
                let error = unsafe { *errno };
                if error != 0 {
                    return Err(io::Error::from_raw_os_error(error));
                }
                break;
            }
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }
                .to_str()
                .map_err(|_| io::Error::other("package paths must be UTF-8"))?;
            if name == "." || name == ".." {
                continue;
            }
            if names.len() == limit {
                return Err(io::Error::other("package directory entry limit"));
            }
            names.push(name.to_owned());
        }
        names.sort();
        Ok(names)
    }
    pub fn unlink(&self, name: &str) -> io::Result<()> {
        let name = c(OsStr::new(name))?;
        if unsafe { libc::unlinkat(self.0.as_raw_fd(), name.as_ptr(), 0) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
    pub fn rename_exclusive(&self, from: &str, to: &str) -> io::Result<()> {
        let from = c(OsStr::new(from))?;
        let to = c(OsStr::new(to))?;
        #[cfg(target_os = "macos")]
        let n = unsafe {
            libc::renameatx_np(
                self.0.as_raw_fd(),
                from.as_ptr(),
                self.0.as_raw_fd(),
                to.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        #[cfg(target_os = "linux")]
        let n = unsafe {
            libc::renameat2(
                self.0.as_raw_fd(),
                from.as_ptr(),
                self.0.as_raw_fd(),
                to.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let n = {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "atomic package rename unavailable",
            ));
        };
        if n == 0 {
            self.0.sync_all()
        } else {
            Err(io::Error::last_os_error())
        }
    }
}
