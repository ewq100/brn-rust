use crate::*;
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Clone, Debug, Serialize)]
pub struct ExportFile {
    pub path: String,
    pub record: String,
    pub version: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct ExportManifest {
    pub format: String,
    pub files: Vec<ExportFile>,
}
pub fn publish(store: &Store, destination: &Path) -> AppResult<ExportManifest> {
    if !destination.is_absolute() {
        return Err(AppError::Invalid(
            "Choose an absolute fresh export destination".into(),
        ));
    }
    if destination.try_exists()? {
        return Err(AppError::Invalid(
            "Export destination already exists; choose a new snapshot".into(),
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| AppError::Invalid("Invalid destination".into()))?;
    let stage = tempfile::Builder::new()
        .prefix(".brn-export-")
        .tempdir_in(parent)?;
    // A single SELECT holds a consistent committed view across all canonical rows.
    let records = store.records()?;
    let notes: Vec<_> = records
        .iter()
        .filter(|r| {
            !r.archived && matches!(&r.data,RecordData::Note(n) if n.superseded_by.is_none())
        })
        .collect();
    let mut manifest = ExportManifest {
        format: "BRN Threads Markdown snapshot 1".into(),
        files: vec![],
    };
    fs::create_dir(stage.path().join("assets"))?;
    let mut assets = BTreeMap::new();
    for record in &records {
        if !record.archived
            && let RecordData::Asset(asset) = &record.data
            && notes.iter().any(|n| n.id == asset.note)
        {
            let extension = match asset.media_type.as_str() {
                "image/png" => "png",
                "image/jpeg" => "jpg",
                "image/webp" => "webp",
                _ => "bin",
            };
            let name = format!("assets/{}.{extension}", safe_name(&record.id));
            fs::write(stage.path().join(&name), &asset.bytes)?;
            assets.insert(record.id.clone(), name.clone());
            manifest.files.push(ExportFile {
                path: name,
                record: record.id.clone(),
                version: record.version,
                sha256: format!("{:x}", Sha256::digest(&asset.bytes)),
            });
        }
    }
    for record in notes {
        let RecordData::Note(note) = &record.data else {
            continue;
        };
        let markdown = crate::assets::rewrite(&note.markdown, |id| assets.get(id).cloned())?;
        let name = format!("{}.md", safe_name(&record.id));
        fs::write(stage.path().join(&name), markdown.as_bytes())?;
        manifest.files.push(ExportFile {
            path: name,
            record: record.id.clone(),
            version: record.version,
            sha256: format!("{:x}", Sha256::digest(markdown.as_bytes())),
        });
    }
    for file in &manifest.files {
        let bytes = fs::read(stage.path().join(&file.path))?;
        if format!("{:x}", Sha256::digest(&bytes)) != file.sha256 {
            return Err(AppError::Invalid("Snapshot verification failed".into()));
        }
    }
    fs::write(
        stage.path().join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    // Flush completed files before the directory becomes visible at its final name.
    for file in &manifest.files {
        fs::File::open(stage.path().join(&file.path))?.sync_all()?;
    }
    fs::File::open(stage.path().join("manifest.json"))?.sync_all()?;
    rename_exclusive(stage.path(), destination)?;
    Ok(manifest)
}
fn safe_name(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn rename_exclusive(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let from = std::ffi::CString::new(source.as_os_str().as_bytes())
        .map_err(|_| std::io::ErrorKind::InvalidInput)?;
    let to = std::ffi::CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| std::io::ErrorKind::InvalidInput)?;
    #[cfg(target_os = "macos")]
    let result = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn rename_exclusive(source: &Path, destination: &Path) -> std::io::Result<()> {
    if destination.try_exists()? {
        return Err(std::io::ErrorKind::AlreadyExists.into());
    }
    // Windows directory rename refuses an existing target.
    fs::rename(source, destination)
}
