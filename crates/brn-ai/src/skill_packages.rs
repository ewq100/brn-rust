//! Owner-installed, inactive local instruction configuration. Not canonical
//! knowledge, a derived cache, or a runtime authority/enablement mechanism.
use crate::threads_guides::{CATALOG, skill_metadata, valid_skill_name};
use brn_threads_core::Store;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, Read, Write},
    os::unix::fs::MetadataExt,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
#[path = "skill_packages_fs.rs"]
mod fs;
use fs::Dir;
const FILE_BYTES: usize = 64 * 1024;
const TOTAL_BYTES: usize = 256 * 1024;
const FILES: usize = 16;
// JSON escapes at most double permitted text; <=16 bounded ASCII paths and
// fixed envelope fit comfortably below the additional 32 KiB.
const ARTIFACT_BYTES: usize = 544 * 1024;
const PACKAGES: usize = 64;
const MARKER: &str = "BRN instruction packages v1\n";
pub const RETENTION_NOTICE: &str = "Installed skills are machine-local instruction configuration, excluded from current BRN SQLite backups and Markdown exports. Preserve original package folders for reinstall. Installation does not enable a skill.";
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PackageInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub origin: String,
    /// Installation scope only; project/runtime enablement belongs to D3b.
    pub scope: String,
    pub digest: String,
    pub file_count: usize,
    pub total_bytes: usize,
    pub enabled: bool,
    pub requirements: Vec<String>,
    pub included_in_backup: bool,
    pub included_in_export: bool,
    pub retention_notice: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Package {
    pub info: PackageInfo,
    pub files: BTreeMap<String, String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    format: u8,
    digest: String,
    files: BTreeMap<String, String>,
}
fn bad(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
/// Package-domain refusals, carried inside std I/O without confusing OS faults.
#[derive(Debug, PartialEq, Eq)]
pub enum PackageRefusal {
    DigestConflict,
    NotInstalled,
}
impl std::fmt::Display for PackageRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DigestConflict => "skill identity conflicts with installed bytes; inspect, remove with its exact digest, then install",
            Self::NotInstalled => "skill package not installed",
        })
    }
}
impl std::error::Error for PackageRefusal {}
fn conflict() -> io::Error {
    io::Error::new(io::ErrorKind::AlreadyExists, PackageRefusal::DigestConflict)
}

fn digest(files: &BTreeMap<String, String>) -> String {
    let mut h = Sha256::new();
    h.update(b"brn-instruction-package-v1\0");
    for (path, text) in files {
        h.update((path.len() as u64).to_be_bytes());
        h.update(path.as_bytes());
        h.update((text.len() as u64).to_be_bytes());
        h.update(text.as_bytes());
    }
    format!("{:x}", h.finalize())
}
fn component(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.as_bytes()[0].is_ascii_alphanumeric()
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        && name != "."
        && name != ".."
}
fn file_path(path: &str) -> bool {
    if path == "SKILL.md" {
        return true;
    }
    let parts = path.split('/').collect::<Vec<_>>();
    (2..=3).contains(&parts.len())
        && parts[0] == "references"
        && parts.iter().all(|s| component(s))
        && (path.ends_with(".md") || path.ends_with(".txt"))
}
fn validate(files: BTreeMap<String, String>, builtin: bool) -> io::Result<Package> {
    if files.is_empty()
        || files.len() > FILES
        || files.values().map(String::len).sum::<usize>() > TOTAL_BYTES
    {
        return Err(bad("package exceeds file/count/total-byte bounds"));
    }
    for (path, text) in &files {
        if !file_path(path)
            || text.len() > FILE_BYTES
            || text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            return Err(bad("unsupported package file/path or control-bearing text"));
        }
        // Supported references are literal package-root-relative Markdown links.
        // URLs can be quoted as plain evidence, but are never fetched.
        let mut rest = text.as_str();
        while let Some((_, tail)) = rest.split_once("](") {
            let (target, next) = tail
                .split_once(')')
                .ok_or_else(|| bad("unterminated reference link"))?;
            let path = target.split('#').next().unwrap_or_default();
            if !path.is_empty() && (!file_path(path) || !files.contains_key(path)) {
                return Err(bad(
                    "only existing confined package-relative references are supported; no remote/dependency fetch",
                ));
            }
            rest = next;
        }
    }
    let text = files
        .get("SKILL.md")
        .ok_or_else(|| bad("SKILL.md required"))?;
    let metadata = skill_metadata(text).map_err(bad)?;
    if !builtin && CATALOG.iter().any(|s| s.id == metadata.name) {
        return Err(bad("installed skill cannot shadow a built-in"));
    }
    let info = PackageInfo {
        id: if builtin {
            metadata.name.to_owned()
        } else {
            format!("installed:{}", metadata.name)
        },
        name: metadata.name.to_owned(),
        description: metadata.description.to_owned(),
        origin: if builtin {
            "builtin"
        } else {
            "owner_installed"
        }
        .into(),
        scope: if builtin {
            "packaged"
        } else {
            "workspace_installation"
        }
        .into(),
        digest: digest(&files),
        file_count: files.len(),
        total_bytes: files.values().map(String::len).sum(),
        enabled: builtin,
        requirements: Vec::new(),
        included_in_backup: false,
        included_in_export: false,
        retention_notice: RETENTION_NOTICE,
    };
    Ok(Package { info, files })
}
fn bounded(mut file: File, limit: usize) -> io::Result<String> {
    let before = file.metadata()?;
    if before.len() > limit as u64 || before.mode() & 0o111 != 0 {
        return Err(bad(
            "package text exceeds limit or has executable permissions",
        ));
    }
    let mut bytes = Vec::new();
    (&mut file).take(limit as u64 + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() > limit
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
    {
        return Err(bad("package changed during bounded read"));
    }
    String::from_utf8(bytes).map_err(|_| bad("package files must be UTF-8"))
}
fn source_files(
    dir: &Dir,
    prefix: &str,
    files: &mut BTreeMap<String, String>,
    entries: &mut usize,
) -> io::Result<()> {
    for name in dir.names(64)? {
        *entries += 1;
        if *entries > 64 || !component(&name) {
            return Err(bad("package entry count or name unsupported"));
        }
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if path == "references" || (prefix == "references" && !name.contains('.')) {
            source_files(&dir.child(&name)?, &path, files, entries)?;
        } else {
            if !file_path(&path) || files.len() == FILES {
                return Err(bad(
                    "unsupported file, executable/dependency or excessive package depth/count",
                ));
            }
            let text = bounded(dir.file(&name, libc::O_RDONLY)?, FILE_BYTES)?;
            if files.values().map(String::len).sum::<usize>() + text.len() > TOTAL_BYTES {
                return Err(bad("package total exceeds 256 KiB"));
            }
            files.insert(path, text);
        }
    }
    Ok(())
}
fn from_source(source: &Path) -> io::Result<Package> {
    let dir = Dir::absolute(source)?;
    let mut files = BTreeMap::new();
    source_files(&dir, "", &mut files, &mut 0)?;
    validate(files, false)
}
struct Catalog {
    dir: Dir,
    _lock: File,
}
impl Catalog {
    fn open(store: &Store, create: bool) -> io::Result<Option<Self>> {
        let root = Dir::absolute(store.directory())?;
        let mut created = false;
        let dir = match root.child("skills") {
            Ok(d) => d,
            Err(e) if e.kind() == io::ErrorKind::NotFound && !create => return Ok(None),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                match root.mkdir_new("skills") {
                    Ok(()) => created = true,
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(e) => return Err(e),
                }
                root.child("skills")?
            }
            Err(e) => return Err(e),
        };
        if dir.0.metadata()?.mode() & 0o022 != 0 {
            return Err(bad(
                "managed skills folder must not be writable by other users",
            ));
        }
        if created {
            match dir.file(
                ".brn-skill-packages",
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
            ) {
                Ok(mut f) => {
                    f.write_all(MARKER.as_bytes())?;
                    f.sync_all()?;
                    dir.0.sync_all()?;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
        let marker = match dir.file(".brn-skill-packages", libc::O_RDONLY) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(bad("foreign skills folder; marker required"));
            }
            other => other?,
        };
        if bounded(marker, 128)? != MARKER {
            return Err(bad("foreign skills folder; marker required"));
        }
        let lock = dir.file(".lock", libc::O_RDWR | libc::O_CREAT)?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "skill management is busy; retry",
                ));
            }
            Err(std::fs::TryLockError::Error(error)) => return Err(error),
        }
        Ok(Some(Self { dir, _lock: lock }))
    }
    fn read(&self, name: &str) -> io::Result<Option<Package>> {
        if !valid_skill_name(name) {
            return Err(bad("invalid skill name"));
        }
        let file = match self.dir.file(&format!("{name}.json"), libc::O_RDONLY) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let artifact: Artifact = serde_json::from_str(&bounded(file, ARTIFACT_BYTES)?)
            .map_err(|_| bad("invalid installed package artifact"))?;
        let package = validate(artifact.files, false)?;
        if artifact.format != 1
            || package.info.digest != artifact.digest
            || package.info.name != name
        {
            return Err(conflict());
        }
        Ok(Some(package))
    }
    fn installed(&self) -> io::Result<Vec<PackageInfo>> {
        let mut list = Vec::new();
        for name in self.dir.names(PACKAGES + 4)? {
            if name == ".brn-skill-packages"
                || name == ".lock"
                || name.starts_with(".stage-")
                || name.starts_with(".removed-")
            {
                continue;
            }
            let name = name
                .strip_suffix(".json")
                .ok_or_else(|| bad("foreign file in managed skills folder"))?;
            if list.len() == PACKAGES {
                return Err(bad("installed package count exceeds 64"));
            }
            list.push(
                self.read(name)?
                    .ok_or_else(|| bad("installed package disappeared"))?
                    .info,
            );
        }
        Ok(list)
    }
}
fn builtin(name: &str) -> io::Result<Option<Package>> {
    // Runtime catalogue/preamble remains unchanged; management uses frontmatter,
    // rather than adding another hardcoded description source.
    CATALOG
        .iter()
        .find(|s| s.id == name)
        .map(|s| validate(BTreeMap::from([("SKILL.md".into(), s.text.into())]), true))
        .transpose()
}
pub fn list(store: &Store) -> io::Result<Vec<PackageInfo>> {
    let mut list = CATALOG
        .iter()
        .map(|s| builtin(s.id).map(|p| p.expect("catalogue entry").info))
        .collect::<io::Result<Vec<_>>>()?;
    if let Some(catalog) = Catalog::open(store, false)? {
        list.extend(catalog.installed()?);
    }
    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}
pub fn inspect(store: &Store, name: &str) -> io::Result<Package> {
    if let Some(package) = builtin(name)? {
        return Ok(package);
    }
    Catalog::open(store, false)?
        .map(|c| c.read(name))
        .transpose()?
        .flatten()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, PackageRefusal::NotInstalled))
}
static NONCE: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
type RemoveHook = Box<dyn FnOnce(&Dir, &str)>;
#[cfg(test)]
thread_local! {
    // A scoped one-shot on this test thread only; absent from production builds.
    static BEFORE_REMOVE_RENAME: std::cell::RefCell<Option<RemoveHook>> = const { std::cell::RefCell::new(None) };
}
fn transient(prefix: &str) -> String {
    format!(
        ".{prefix}-{}-{}",
        std::process::id(),
        NONCE.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn install(store: &Store, source: &Path) -> io::Result<PackageInfo> {
    let package = from_source(source)?;
    let catalog = Catalog::open(store, true)?.ok_or_else(|| bad("skills folder unavailable"))?;
    if let Some(old) = catalog.read(&package.info.name)? {
        return if old.info.digest == package.info.digest {
            Ok(old.info)
        } else {
            Err(conflict())
        };
    }
    if catalog.installed()?.len() == PACKAGES {
        return Err(bad("installed package limit reached"));
    }
    let artifact = Artifact {
        format: 1,
        digest: package.info.digest.clone(),
        files: package.files,
    };
    let bytes = serde_json::to_vec(&artifact).map_err(|_| bad("cannot encode package"))?;
    if bytes.len() > ARTIFACT_BYTES {
        return Err(bad("encoded package exceeds artifact bound"));
    }
    let stage = transient("stage");
    let mut file = catalog
        .dir
        .file(&stage, libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL)?;
    let outcome = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        catalog
            .dir
            .rename_exclusive(&stage, &format!("{}.json", package.info.name))
    })();
    if outcome.is_err() {
        // Never clean a stage path replaced by an unrelated file.
        if let Ok(current) = catalog.dir.file(&stage, libc::O_RDONLY)
            && let (Ok(current), Ok(owned)) = (current.metadata(), file.metadata())
            && current.ino() == owned.ino()
            && current.dev() == owned.dev()
        {
            let _ = catalog.dir.unlink(&stage);
        }
    }
    outcome?;
    Ok(package.info)
}
pub fn remove(store: &Store, name: &str, expected_digest: &str) -> io::Result<bool> {
    if !valid_skill_name(name) || CATALOG.iter().any(|s| s.id == name) {
        return Err(bad("only installed skills can be removed"));
    }
    if expected_digest.len() != 64
        || !expected_digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(bad("exact SHA-256 digest required for removal"));
    }
    let Some(catalog) = Catalog::open(store, false)? else {
        return Ok(false);
    };
    let Some(old) = catalog.read(name)? else {
        return Ok(false);
    };
    if old.info.digest != expected_digest {
        return Err(conflict());
    }
    let artifact = format!("{name}.json");
    let removed = transient("removed");
    #[cfg(test)]
    BEFORE_REMOVE_RENAME.with(|slot| {
        let hook = slot.borrow_mut().take();
        if let Some(hook) = hook {
            hook(&catalog.dir, &artifact);
        }
    });
    catalog.dir.rename_exclusive(&artifact, &removed)?;
    // Recheck the moved bytes, so an uncooperative replacement is preserved,
    // never deleted. Exclusive rollback never overwrites a new owner file.
    let checked = (|| -> io::Result<()> {
        let stored: Artifact = serde_json::from_str(&bounded(
            catalog.dir.file(&removed, libc::O_RDONLY)?,
            ARTIFACT_BYTES,
        )?)
        .map_err(|_| bad("invalid removed artifact"))?;
        let p = validate(stored.files, false)?;
        if stored.format != 1
            || stored.digest != expected_digest
            || p.info.digest != expected_digest
            || p.info.name != name
        {
            return Err(conflict());
        }
        Ok(())
    })();
    if let Err(e) = checked {
        let _ = catalog.dir.rename_exclusive(&removed, &artifact);
        return Err(e);
    }
    catalog.dir.unlink(&removed)?;
    catalog.dir.0.sync_all()?;
    Ok(true)
}
#[cfg(test)]
#[path = "skill_packages_tests.rs"]
mod tests;
