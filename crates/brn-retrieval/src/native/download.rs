//! Explicit, pinned asset installation. Loading a model never enters this path.
use crate::{Error, Result, check_cancel};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Component, Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

pub const REVISION: &str = "751bff37182d3f1213fa05d7196b954e230abad9";
const BASE_URL: &str = "https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/751bff37182d3f1213fa05d7196b954e230abad9/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ModelInstallError {
    #[error("model destination is occupied by unverified assets")]
    TargetOccupied,
    #[error("model asset has an unexpected byte size")]
    SizeMismatch,
    #[error("model asset digest does not match the pinned revision")]
    DigestMismatch,
    #[error("model asset request failed")]
    Network,
    #[error("model destination must be an absolute path without symlinks")]
    InvalidTarget,
    #[error("could not clean owned model staging files")]
    Cleanup,
    #[error("exclusive model installation is unavailable on this platform")]
    ExclusiveInstallUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInstallReport {
    pub directory: PathBuf,
    pub downloaded_bytes: u64,
}

#[derive(Clone, Copy)]
pub(super) enum DigestPin {
    Sha256([u8; 32]),
    GitBlobSha1([u8; 20]),
}

pub(super) struct Asset {
    pub source: &'static str,
    pub name: &'static str,
    pub bytes: u64,
    pub digest: DigestPin,
}

const fn hex<const N: usize>(value: &str) -> [u8; N] {
    const fn digit(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("invalid pinned digest"),
        }
    }
    assert!(value.len() == N * 2);
    let mut bytes = [0; N];
    let mut i = 0;
    while i < N {
        bytes[i] = digit(value.as_bytes()[2 * i]) * 16 + digit(value.as_bytes()[2 * i + 1]);
        i += 1;
    }
    bytes
}

pub(super) const ASSETS: [Asset; 5] = [
    Asset {
        source: "onnx/model.onnx",
        name: "model.onnx",
        bytes: 90_387_606,
        digest: DigestPin::Sha256(hex(
            "759c3cd2b7fe7e93933ad23c4c9181b7396442a2ed746ec7c1d46192c469c46e",
        )),
    },
    Asset {
        source: "tokenizer.json",
        name: "tokenizer.json",
        bytes: 711_661,
        digest: DigestPin::GitBlobSha1(hex("c17ed520ed8438736732a54957a69306b8822215")),
    },
    Asset {
        source: "config.json",
        name: "config.json",
        bytes: 650,
        digest: DigestPin::GitBlobSha1(hex("72147e4ff4426ebedbfa2146c4a0999def51a313")),
    },
    Asset {
        source: "special_tokens_map.json",
        name: "special_tokens_map.json",
        bytes: 125,
        digest: DigestPin::GitBlobSha1(hex("a8b3208c2884c4efb86e49300fdd3dc877220cdf")),
    },
    Asset {
        source: "tokenizer_config.json",
        name: "tokenizer_config.json",
        bytes: 366,
        digest: DigestPin::GitBlobSha1(hex("37fca74771bc76a8e01178ce3a6055a0995f8093")),
    },
];

pub fn download_model(
    target: &Path,
    cancel: &AtomicBool,
    progress: impl Fn(u64, u64),
) -> Result<ModelInstallReport> {
    check_cancel(cancel)?;
    // Client construction is inert. The fetch closure is called only for missing assets.
    let source = HttpSource::new(BASE_URL)?;
    install_with(target, cancel, progress, &ASSETS, |asset| {
        source.fetch(asset)
    })
}

pub(super) struct HttpSource {
    client: reqwest::blocking::Client,
    base: String,
}

impl HttpSource {
    pub(super) fn new(base: &str) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|_| ModelInstallError::Network)?;
        Ok(Self {
            client,
            base: base.to_owned(),
        })
    }

    pub(super) fn fetch(&self, asset: &Asset) -> Result<Box<dyn Read>> {
        let response = self
            .client
            .get(format!("{}{}", self.base, asset.source))
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|_| ModelInstallError::Network)?;
        if response
            .content_length()
            .is_some_and(|size| size != asset.bytes)
        {
            return Err(ModelInstallError::SizeMismatch.into());
        }
        Ok(Box::new(response))
    }
}

enum Hasher {
    Sha256(Sha256),
    GitBlobSha1(sha1::Sha1),
}

impl Hasher {
    fn new(asset: &Asset) -> Self {
        match asset.digest {
            DigestPin::Sha256(_) => Self::Sha256(Sha256::new()),
            DigestPin::GitBlobSha1(_) => {
                let mut hash = sha1::Sha1::new();
                hash.update(format!("blob {}\0", asset.bytes));
                Self::GitBlobSha1(hash)
            }
        }
    }
    fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Sha256(hash) => hash.update(bytes),
            Self::GitBlobSha1(hash) => hash.update(bytes),
        }
    }
    fn verify(self, pin: DigestPin) -> Result<()> {
        let valid = match (self, pin) {
            (Self::Sha256(hash), DigestPin::Sha256(expected)) => {
                <[u8; 32]>::from(hash.finalize()) == expected
            }
            (Self::GitBlobSha1(hash), DigestPin::GitBlobSha1(expected)) => {
                <[u8; 20]>::from(hash.finalize()) == expected
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(ModelInstallError::DigestMismatch.into())
        }
    }
}

fn validate_target(target: &Path) -> Result<()> {
    if !target.is_absolute()
        || target.file_name().is_none()
        || target
            .components()
            .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
    {
        return Err(ModelInstallError::InvalidTarget.into());
    }
    for ancestor in target.ancestors() {
        match ancestor.symlink_metadata() {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(ModelInstallError::InvalidTarget.into());
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn verified_existing(target: &Path, assets: &[Asset], cancel: &AtomicBool) -> Result<bool> {
    let meta = match target.symlink_metadata() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
        Ok(meta) => meta,
    };
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(ModelInstallError::TargetOccupied.into());
    }
    let verify = || -> Result<()> {
        let entries = std::fs::read_dir(target)?.collect::<std::io::Result<Vec<_>>>()?;
        if entries.len() != assets.len() {
            return Err(ModelInstallError::TargetOccupied.into());
        }
        for asset in assets {
            check_cancel(cancel)?;
            let path = target.join(asset.name);
            let meta = path.symlink_metadata()?;
            if !meta.is_file() || meta.nlink() != 1 || meta.len() != asset.bytes {
                return Err(ModelInstallError::TargetOccupied.into());
            }
            let mut file = File::open(path)?;
            let opened = file.metadata()?;
            if (opened.dev(), opened.ino()) != (meta.dev(), meta.ino()) {
                return Err(ModelInstallError::TargetOccupied.into());
            }
            let mut hash = Hasher::new(asset);
            let mut size = 0;
            let mut buffer = [0; 65_536];
            loop {
                check_cancel(cancel)?;
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                size += count as u64;
                if size > asset.bytes {
                    return Err(ModelInstallError::SizeMismatch.into());
                }
                hash.update(&buffer[..count]);
            }
            if size != asset.bytes {
                return Err(ModelInstallError::SizeMismatch.into());
            }
            hash.verify(asset.digest)?;
        }
        Ok(())
    };
    match verify() {
        Ok(()) => Ok(true),
        Err(Error::Cancelled) => Err(Error::Cancelled),
        Err(Error::ModelInstall(_)) => Err(ModelInstallError::TargetOccupied.into()),
        Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(ModelInstallError::TargetOccupied.into())
        }
        Err(error) => Err(error),
    }
}

struct Stage {
    path: PathBuf,
    identity: (u64, u64),
    files: Vec<(PathBuf, (u64, u64))>,
}

impl Stage {
    fn create(parent: &Path) -> Result<Self> {
        let path = parent.join(format!(".brn-model-{}", uuid::Uuid::new_v4()));
        std::fs::DirBuilder::new().mode(0o700).create(&path)?;
        let meta = path.symlink_metadata()?;
        Ok(Self {
            path,
            identity: (meta.dev(), meta.ino()),
            files: Vec::new(),
        })
    }

    fn create_file(&mut self, name: &str) -> Result<File> {
        let path = self.path.join(name);
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&path)?;
        let meta = file.metadata()?;
        self.files.push((path, (meta.dev(), meta.ino())));
        Ok(file)
    }

    fn cleanup(&mut self) -> Result<()> {
        let meta = self
            .path
            .symlink_metadata()
            .map_err(|_| ModelInstallError::Cleanup)?;
        if (meta.dev(), meta.ino()) != self.identity || !meta.is_dir() {
            return Err(ModelInstallError::Cleanup.into());
        }
        for (path, identity) in &self.files {
            let meta = path
                .symlink_metadata()
                .map_err(|_| ModelInstallError::Cleanup)?;
            if (meta.dev(), meta.ino()) != *identity || !meta.is_file() {
                return Err(ModelInstallError::Cleanup.into());
            }
            std::fs::remove_file(path).map_err(|_| ModelInstallError::Cleanup)?;
        }
        self.files.clear();
        std::fs::remove_dir(&self.path).map_err(|_| ModelInstallError::Cleanup)?;
        Ok(())
    }
}

pub(super) fn install_with(
    target: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
    assets: &[Asset],
    fetch: impl Fn(&Asset) -> Result<Box<dyn Read>>,
) -> Result<ModelInstallReport> {
    check_cancel(cancel)?;
    validate_target(target)?;
    if verified_existing(target, assets, cancel)? {
        return Ok(ModelInstallReport {
            directory: target.to_owned(),
            downloaded_bytes: 0,
        });
    }
    let parent = target.parent().ok_or(ModelInstallError::InvalidTarget)?;
    std::fs::create_dir_all(parent)?;
    validate_target(target)?;
    let total: u64 = assets.iter().map(|a| a.bytes).sum();
    let mut stage = Stage::create(parent)?;
    let result = (|| {
        let mut received = 0;
        for asset in assets {
            check_cancel(cancel)?;
            let mut input = fetch(asset)?;
            let mut output = stage.create_file(asset.name)?;
            let mut hash = Hasher::new(asset);
            let mut size = 0;
            let mut buffer = [0; 65_536];
            loop {
                check_cancel(cancel)?;
                // Read at most one byte beyond the exact pin; never buffer an unbounded response.
                let bound = buffer.len().min((asset.bytes - size + 1) as usize);
                let count = input
                    .read(&mut buffer[..bound])
                    .map_err(|_| ModelInstallError::Network)?;
                if count == 0 {
                    break;
                }
                size += count as u64;
                if size > asset.bytes {
                    return Err(ModelInstallError::SizeMismatch.into());
                }
                output.write_all(&buffer[..count])?;
                hash.update(&buffer[..count]);
                received += count as u64;
                progress(received, total);
            }
            check_cancel(cancel)?;
            if size != asset.bytes {
                return Err(ModelInstallError::SizeMismatch.into());
            }
            hash.verify(asset.digest)?;
            output.sync_all()?;
        }
        check_cancel(cancel)?;
        File::open(&stage.path)?.sync_all()?;
        install_exclusive(&stage.path, target)?;
        File::open(parent)?.sync_all()?;
        Ok(ModelInstallReport {
            directory: target.to_owned(),
            downloaded_bytes: received,
        })
    })();
    // Once installed, the directory is no longer staging and must never be removed.
    if result.is_err() && stage.path.try_exists()? {
        stage.cleanup()?;
    }
    result
}

#[cfg(target_os = "macos")]
fn install_exclusive(stage: &Path, target: &Path) -> Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let source =
        CString::new(stage.as_os_str().as_bytes()).map_err(|_| ModelInstallError::InvalidTarget)?;
    let target = CString::new(target.as_os_str().as_bytes())
        .map_err(|_| ModelInstallError::InvalidTarget)?;
    let status = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if status == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        Err(ModelInstallError::TargetOccupied.into())
    } else {
        Err(error.into())
    }
}

#[cfg(not(target_os = "macos"))]
fn install_exclusive(_: &Path, _: &Path) -> Result<()> {
    Err(ModelInstallError::ExclusiveInstallUnavailable.into())
}
