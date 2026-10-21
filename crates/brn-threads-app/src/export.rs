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
    publish_with_directory_flush(store, destination, sync_directory)
}

fn publish_with_directory_flush(
    store: &Store,
    destination: &Path,
    mut flush_directory: impl FnMut(&Path) -> std::io::Result<()>,
) -> AppResult<ExportManifest> {
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
    let full_snapshot = store.records()?;
    // General-looking Notes may still be private through a retained Source.
    // Decide eligibility against the complete snapshot before omitting private
    // metadata. Keep only IDs so large retained assets are not cloned.
    let general_notes = full_snapshot
        .iter()
        .filter(|r| {
            !r.archived
                && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none() && n.chat.is_none())
                && record_in_chat(r, &full_snapshot, None)
        })
        .map(|r| r.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let records = full_snapshot
        .into_iter()
        .filter(|r| !matches!(&r.data,RecordData::Source(s) if s.chat.is_some()))
        .collect::<Vec<_>>();
    let notes: Vec<_> = records
        .iter()
        .filter(|r| general_notes.contains(&r.id))
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
        let body = crate::assets::rewrite(&note.markdown, |id| assets.get(id).cloned())?;
        let markdown = portable_markdown(record, note, &records, &body)?;
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
    flush_directory(&stage.path().join("assets"))?;
    flush_directory(stage.path())?;
    rename_exclusive(stage.path(), destination)?;
    flush_directory(parent)?;
    Ok(manifest)
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    fs::File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
fn portable_markdown(
    record: &Record,
    note: &Note,
    records: &[Record],
    body: &str,
) -> AppResult<String> {
    let mut source_ids = std::collections::BTreeSet::new();
    if let Some(import) = &note.import {
        source_ids.insert(import.source.as_str());
    }
    for row in records {
        if !row.archived
            && let RecordData::Link(link) = &row.data
            && link.from == record.id
        {
            source_ids.insert(link.to.as_str());
        }
    }
    let sources: Vec<_> = records
        .iter()
        .filter_map(|row| {
            if source_ids.contains(row.id.as_str())
                && let RecordData::Source(source) = &row.data
            {
                Some((row, source))
            } else {
                None
            }
        })
        .collect();
    // JSON double-quoted scalars are YAML-compatible. Newlines/delimiters remain
    // escaped data, and an existing front matter block stays untouched in body.
    let mut markdown = format!("---\ntitle: {}\n", serde_json::to_string(&note.title)?);
    if sources.is_empty() {
        markdown.push_str("sources: []\n");
    } else {
        markdown.push_str("sources:\n");
        for (row, source) in sources {
            markdown.push_str(&format!(
                "  - record: {}\n    locator: {}\n    label: {}\n",
                serde_json::to_string(&row.id)?,
                serde_json::to_string(&source.locator)?,
                serde_json::to_string(&source.label)?,
            ));
        }
    }
    markdown.push_str("---\n");
    markdown.push_str(body);
    Ok(markdown)
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

#[cfg(test)]
mod tests {
    use super::*;
    use brn_threads_core::Store;

    #[cfg(unix)]
    struct OwnedChild(std::process::Child);

    #[cfg(unix)]
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[cfg(unix)]
    #[test]
    fn sigkilled_staged_export_is_not_published_and_retry_publishes_complete_snapshot() {
        const CHILD_ROOT: &str = "BRN_EXPORT_INTERRUPTION_CHILD_ROOT";
        const TEST_NAME: &str = "export::tests::sigkilled_staged_export_is_not_published_and_retry_publishes_complete_snapshot";

        if let Some(root) = std::env::var_os(CHILD_ROOT) {
            let root = std::path::PathBuf::from(root);
            let workspace = Workspace::open(root.join("workspace")).unwrap();
            let destination = root.join("published").join("snapshot");
            let ready = root.join("child-ready");
            let _ = publish_with_directory_flush(&workspace.store, &destination, |path| {
                assert_eq!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("assets")
                );
                fs::write(&ready, b"ready")?;
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                }
            });
            panic!("child export unexpectedly passed the interruption seam");
        }

        use std::os::unix::process::ExitStatusExt;
        use std::{
            process::{Command, Stdio},
            thread,
            time::{Duration, Instant},
        };

        let root = tempfile::tempdir_in(fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
        let workspace_path = root.path().join("workspace");
        let destination_parent = root.path().join("published");
        fs::create_dir(&destination_parent).unwrap();
        let destination = destination_parent.join("snapshot");
        let title = "Synthetic export interruption";
        let body = "Expected exact body: café.\nSecond line.\n";
        let mut workspace = Workspace::open(&workspace_path).unwrap();
        let note = workspace.new_note(title, body).unwrap();
        let note_id = note.id.clone();
        let note_version = note.version;
        let before_records = workspace.store.records().unwrap();
        let before_history = workspace.history().unwrap();
        drop(workspace);

        let ready = root.path().join("child-ready");
        let mut child = OwnedChild(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
                .env(CHILD_ROOT, root.path())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while !ready.exists() {
            if let Some(status) = child.0.try_wait().unwrap() {
                panic!("export child exited before reaching the flush seam: {status}");
            }
            assert!(
                Instant::now() < deadline,
                "export child did not reach the flush seam within five seconds"
            );
            thread::sleep(Duration::from_millis(10));
        }

        child.0.kill().unwrap();
        let status = child.0.wait().unwrap();
        assert_eq!(status.signal(), Some(libc::SIGKILL));
        assert!(!destination.exists());

        let mut reopened = Workspace::open(&workspace_path).unwrap();
        assert_eq!(reopened.store.records().unwrap(), before_records);
        assert_eq!(reopened.history().unwrap(), before_history);
        let manifest = reopened.export_now(&destination).unwrap();

        assert_eq!(manifest.format, "BRN Threads Markdown snapshot 1");
        assert_eq!(manifest.files.len(), 1);
        let file = &manifest.files[0];
        assert_eq!(file.path, format!("{}.md", safe_name(&note_id)));
        assert_eq!(file.record, note_id);
        assert_eq!(file.version, note_version);
        let markdown_bytes = fs::read(destination.join(&file.path)).unwrap();
        assert_eq!(
            file.sha256,
            format!("{:x}", Sha256::digest(&markdown_bytes))
        );
        assert_eq!(
            String::from_utf8(markdown_bytes).unwrap(),
            format!("---\ntitle: {title:?}\nsources: []\n---\n{body}")
        );
        let on_disk_manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(destination.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(on_disk_manifest["format"], manifest.format);
        assert_eq!(on_disk_manifest["files"][0]["record"], file.record);
        assert_eq!(on_disk_manifest["files"][0]["version"], file.version);
        assert_eq!(on_disk_manifest["files"][0]["sha256"], file.sha256);
        assert_eq!(on_disk_manifest["files"][0]["path"], file.path);
        assert_eq!(reopened.store.records().unwrap(), before_records);
        assert_eq!(reopened.history().unwrap(), before_history);
    }

    #[test]
    fn directory_flush_failure_before_publication_cleans_stage_and_keeps_destination_absent() {
        let root =
            tempfile::tempdir_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
        let store = Store::open(root.path().join("source")).unwrap();
        let destination = root.path().join("snapshot");
        let mut calls = 0;

        let result = publish_with_directory_flush(&store, &destination, |_| {
            calls += 1;
            if calls == 2 {
                Err(std::io::Error::other(
                    "injected staged-directory flush failure",
                ))
            } else {
                Ok(())
            }
        });

        assert!(result.is_err());
        assert!(!destination.exists());
        assert!(std::fs::read_dir(root.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".brn-export-")
        }));
    }

    #[test]
    fn parent_flush_failure_reports_error_and_preserves_complete_published_snapshot() {
        let root =
            tempfile::tempdir_in(std::fs::canonicalize(std::env::temp_dir()).unwrap()).unwrap();
        let mut workspace = Workspace::open(root.path().join("source")).unwrap();
        workspace
            .new_note("Synthetic export", "Complete published body")
            .unwrap();
        let destination = root.path().join("snapshot");
        let mut calls = 0;

        let result = publish_with_directory_flush(&workspace.store, &destination, |_| {
            calls += 1;
            if calls == 3 {
                Err(std::io::Error::other(
                    "injected parent-directory flush failure",
                ))
            } else {
                Ok(())
            }
        });

        assert!(result.is_err());
        assert_eq!(calls, 3);
        assert!(destination.join("assets").is_dir());
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(destination.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["format"], "BRN Threads Markdown snapshot 1");
        let files = manifest["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        let markdown =
            std::fs::read_to_string(destination.join(files[0]["path"].as_str().unwrap())).unwrap();
        assert!(markdown.contains("Complete published body"));
        assert!(std::fs::read_dir(root.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".brn-export-")
        }));
    }
}
