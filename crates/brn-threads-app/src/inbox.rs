//! Explicit, bounded full-note inbox imports through the existing checked intake service.
use crate::*;
use brn_threads_intake::{ConversionResult, DocumentKind};
use std::sync::atomic::AtomicBool;
#[cfg(unix)]
use std::sync::atomic::Ordering;

#[cfg(unix)]
const MAX_INBOX_RETAINED_BYTES: usize = 64 * 1024 * 1024;

pub const INBOX_DESTINATION_NOTICE: &str = "General knowledge (available across projects). Files become protected full notes; protection controls editing, not privacy. Original files stay untouched.";

pub struct InboxDocument {
    pub kind: DocumentKind,
    pub bytes: Vec<u8>,
    pub source_reference: String,
}
#[derive(Default, Debug, PartialEq, Eq)]
pub struct InboxScanSummary {
    pub imported: usize,
    pub failed: usize,
    pub pending: usize,
}
impl std::fmt::Display for InboxScanSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Inbox: {} imported, {} failed, {} pending",
            self.imported, self.failed, self.pending
        )
    }
}
#[cfg(unix)]
fn kind(name: &str) -> Option<DocumentKind> {
    Some(
        match Path::new(name)
            .extension()?
            .to_str()?
            .to_ascii_lowercase()
            .as_str()
        {
            "docx" => DocumentKind::Docx,
            "pdf" => DocumentKind::Pdf,
            "md" | "markdown" => DocumentKind::Markdown,
            "txt" => DocumentKind::Text,
            "eml" => DocumentKind::Eml,
            "html" | "htm" => DocumentKind::Html,
            "xlsx" => DocumentKind::Xlsx,
            "pptx" => DocumentKind::Pptx,
            _ => return None,
        },
    )
}
#[cfg(unix)]
fn owner() -> HostAuthority {
    HostAuthority::owner("explicit manual inbox")
}
#[cfg(unix)]
fn detail(error: impl std::fmt::Display) -> String {
    error.to_string().chars().take(256).collect()
}

impl Workspace {
    pub fn inbox(&self) -> AppResult<Option<InboxState>> {
        Ok(self.store.inbox_state()?)
    }

    #[cfg(unix)]
    fn inbox_lease(&self) -> AppResult<std::fs::File> {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let path = self.store.derived_directory()?.join("manual-inbox.lock");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(path)?;
        let meta = file.metadata()?;
        if !meta.is_file() || meta.nlink() != 1 {
            return Err(AppError::Invalid("Unsafe inbox lock".into()));
        }
        file.try_lock().map_err(|_| {
            AppError::Invalid(
                "An inbox scan is already active; stop it or wait before changing the folder"
                    .into(),
            )
        })?;
        // Keep this named lock file: unlinking it could give another process a second lock inode.
        Ok(file)
    }

    /// Called only after the owner explicitly chooses the displayed General destination.
    #[cfg(unix)]
    pub fn choose_inbox(
        &mut self,
        path: &Path,
        destination: InboxDestination,
    ) -> AppResult<InboxState> {
        let _lease = self.inbox_lease()?;
        let folder = crate::inbox_files::Folder::open(path, self.store.directory())?;
        let prior = self.inbox()?;
        if let Some(state) = &prior
            && state.folder == path.to_string_lossy()
            && state.folder_identity == folder.identity
            && state.destination == destination
        {
            return Ok(state.clone());
        }
        let state = InboxState {
            revision: 0,
            generation: uuid::Uuid::new_v4().to_string(),
            folder: path.to_string_lossy().into_owned(),
            folder_identity: folder.identity,
            destination,
            entries: vec![],
        };
        Ok(self
            .store
            .save_inbox_state(prior.as_ref().map(|s| s.revision), &state, &owner())?)
    }
    #[cfg(not(unix))]
    pub fn choose_inbox(
        &mut self,
        _path: &Path,
        _destination: InboxDestination,
    ) -> AppResult<InboxState> {
        Err(AppError::Unavailable(
            "Manual inbox requires the supported macOS or Linux host".into(),
        ))
    }

    #[cfg(unix)]
    fn save_inbox_progress(&mut self, state: &mut InboxState) -> AppResult<()> {
        *state = self
            .store
            .save_inbox_state(Some(state.revision), state, &owner())?;
        Ok(())
    }

    #[cfg(not(unix))]
    pub fn scan_inbox(
        &mut self,
        _cancel: &AtomicBool,
        _convert: impl FnMut(InboxDocument, &DocumentConversionSettings) -> AppResult<ConversionResult>,
    ) -> AppResult<InboxScanSummary> {
        Err(AppError::Unavailable(
            "Manual inbox requires the supported macOS or Linux host".into(),
        ))
    }

    /// The host supplies the existing conversion operation, never an alternate parser.
    /// No raw document bytes are persisted in the inbox ledger.
    #[cfg(unix)]
    pub fn scan_inbox(
        &mut self,
        cancel: &AtomicBool,
        convert: impl FnMut(InboxDocument, &DocumentConversionSettings) -> AppResult<ConversionResult>,
    ) -> AppResult<InboxScanSummary> {
        self.scan_inbox_limited(cancel, MAX_INBOX_RETAINED_BYTES, convert)
    }
    #[cfg(unix)]
    fn scan_inbox_limited(
        &mut self,
        cancel: &AtomicBool,
        max_retained: usize,
        mut convert: impl FnMut(
            InboxDocument,
            &DocumentConversionSettings,
        ) -> AppResult<ConversionResult>,
    ) -> AppResult<InboxScanSummary> {
        use crate::inbox_files::*;
        if max_retained == 0 || max_retained > MAX_INBOX_RETAINED_BYTES {
            return Err(AppError::Invalid("Unsupported inbox output budget".into()));
        }
        let _lease = self.inbox_lease()?;
        let mut state = self.inbox()?.ok_or_else(|| {
            AppError::Invalid("Choose an inbox folder and its General destination first".into())
        })?;
        let folder = Folder::open(Path::new(&state.folder), self.store.directory())?;
        if folder.identity != state.folder_identity {
            return Err(AppError::Invalid(
                "Inbox folder was replaced; choose it again".into(),
            ));
        }
        let settings = self.settings()?.document_conversion;
        let provenance = crate::intake::document_provenance(&settings)?;
        let names = folder.names()?;
        // The ledger describes this bounded directory snapshot. Historical imports
        // and their operations remain canonical knowledge, even after files disappear.
        state.entries = names
            .into_iter()
            .map(|name| InboxEntry {
                name,
                source_sha256: None,
                operation_key: None,
                status: InboxStatus::Pending,
                detail: "Waiting for this explicit scan".into(),
                note: None,
            })
            .collect();
        self.save_inbox_progress(&mut state)?;
        let mut used = 0u64;
        let mut retained = 0usize;
        let mut output_exhausted = false;
        for index in 0..state.entries.len() {
            if cancel.load(Ordering::Acquire) {
                break;
            }
            let name = state.entries[index].name.clone();
            let outcome = (|| -> AppResult<Option<String>> {
                let kind=kind(&name).ok_or_else(||AppError::Invalid("Unsupported file: use PDF, DOCX, PPTX, XLSX, EML, HTML, Markdown or text; folders and links are not followed".into()))?;
                if used >= MAX_INBOX_SCAN_BYTES {
                    return Ok(None);
                }
                let (bytes, hash) = folder.read(&name, MAX_INBOX_SCAN_BYTES - used, &mut used)?;
                let snapshot_len = bytes.len() as u64;
                let key = format!(
                    "inbox:{:x}",
                    Sha256::digest(serde_json::to_vec(&(
                        &state.folder,
                        &name,
                        &state.destination,
                        hash,
                        &provenance
                    ))?)
                );
                state.entries[index].source_sha256 = Some(hash);
                state.entries[index].operation_key = Some(key.clone());
                state.entries[index].detail =
                    "Import pending; safe to retry after interruption".into();
                self.save_inbox_progress(&mut state)?;
                if cancel.load(Ordering::Acquire) {
                    return Ok(None);
                }
                let source_reference = Path::new(&state.folder)
                    .join(&name)
                    .to_string_lossy()
                    .into_owned();
                let title = Path::new(&name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&name);
                if self.settings()?.document_conversion != settings {
                    return Err(AppError::Invalid(
                        "Conversion settings changed; scan again".into(),
                    ));
                }
                // Shared keyed import recovery validates source, scope and conversion
                // before replaying committed/prepared work; never repeat model conversion.
                if let Some(outcome) = self.full_import_replay_with_conversion(
                    &key,
                    title,
                    &source_reference,
                    &bytes,
                    None,
                    &settings,
                )? {
                    let receipt = applied_receipt(outcome)?;
                    return self
                        .full_import_note_in_scope(&receipt, &hash, None)
                        .map(|note| Some(note.id));
                }
                // The CLI replay helper intentionally returns None for prepared work
                // or observation-only receipts. Its check above already validated the
                // saved request; recover those through the same core apply/resolver.
                if let Some(op) = self
                    .store
                    .operation_by_host_key(&format!("full-import:{key}"))?
                    && self.store.prepared(&op).is_ok()
                {
                    let receipt = applied_receipt(self.store.apply_inbox_import(
                        &op,
                        &settings,
                        &state.generation,
                        &owner(),
                    )?)?;
                    return self
                        .full_import_note_in_scope(&receipt, &hash, None)
                        .map(|note| Some(note.id));
                }
                let converted = convert(
                    InboxDocument {
                        kind,
                        bytes,
                        source_reference: source_reference.clone(),
                    },
                    &settings,
                )?;
                if cancel.load(Ordering::Acquire) {
                    return Ok(None);
                }
                if converted.source_reference != source_reference || converted.source_sha256 != hash
                {
                    return Err(AppError::Invalid(
                        "Conversion did not preserve the inbox source identity".into(),
                    ));
                }
                let output_bytes = converted
                    .assets
                    .iter()
                    .try_fold(converted.markdown.len(), |sum, asset| {
                        sum.checked_add(asset.bytes.len())
                    })
                    .ok_or_else(|| AppError::Invalid("Converted output size overflow".into()))?;
                if output_bytes > max_retained {
                    return Err(AppError::Invalid(format!(
                        "This document exceeds the {max_retained} byte converted-output scan limit; use a smaller document"
                    )));
                }
                if retained
                    .checked_add(output_bytes)
                    .is_none_or(|sum| sum > max_retained)
                {
                    output_exhausted = true;
                    return Ok(None);
                }
                let (_, current_hash) = folder.read(&name, snapshot_len, &mut 0).map_err(|_|AppError::Invalid("File changed or became unavailable during conversion; scan again after saving finishes".into()))?;
                if current_hash != hash {
                    return Err(AppError::Invalid(
                        "File changed during conversion; scan again after saving finishes".into(),
                    ));
                }
                if self.settings()?.document_conversion != settings {
                    return Err(AppError::Invalid(
                        "Conversion settings changed; scan again".into(),
                    ));
                }
                let receipt = applied_receipt(self.import_inbox_full(
                    &key,
                    title,
                    &converted,
                    &settings,
                    &state.generation,
                )?)?;
                retained += output_bytes;
                self.full_import_note_in_scope(&receipt, &hash, None)
                    .map(|note| Some(note.id))
            })();
            let entry = &mut state.entries[index];
            match outcome {
                Ok(Some(note)) => {
                    entry.status = InboxStatus::Imported;
                    entry.note = Some(note);
                    entry.detail = "Protected full note in General knowledge".into();
                }
                Ok(None) => {
                    entry.detail = if cancel.load(Ordering::Acquire) {
                        "Scan stopped; choose Scan inbox to retry"
                    } else if output_exhausted {
                        "Converted-output budget reached; retry pending files. If a source-byte limit prevents progress, use a smaller folder."
                    } else {
                        "Source-byte scan limit reached; use a smaller folder to progress"
                    }
                    .into();
                }
                Err(_) if cancel.load(Ordering::Acquire) => {
                    entry.status = InboxStatus::Pending;
                    entry.detail = "Scan stopped; choose Scan inbox to retry".into();
                }
                Err(error) => {
                    entry.status = InboxStatus::Failed;
                    entry.detail = detail(error);
                }
            }
            self.save_inbox_progress(&mut state)?;
            if output_exhausted {
                break;
            }
        }
        Ok(InboxScanSummary {
            imported: state
                .entries
                .iter()
                .filter(|e| e.status == InboxStatus::Imported)
                .count(),
            failed: state
                .entries
                .iter()
                .filter(|e| e.status == InboxStatus::Failed)
                .count(),
            pending: state
                .entries
                .iter()
                .filter(|e| e.status == InboxStatus::Pending)
                .count(),
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn output_budget_counts_markdown_and_assets_replays_are_free_and_oversize_is_failed() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let folder = root.path().join("inbox");
        std::fs::create_dir(&folder).unwrap();
        for name in ["a.txt", "b.txt", "c.txt", "d.txt"] {
            std::fs::write(folder.join(name), name).unwrap();
        }
        let mut workspace =
            Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
        workspace
            .choose_inbox(&folder, InboxDestination::General)
            .unwrap();
        let mut calls = Vec::new();
        let mut conversion = |doc: InboxDocument, settings: &DocumentConversionSettings| {
            let name = Path::new(&doc.source_reference)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            calls.push(name.clone());
            let (size, assets) = match name.as_str() {
                "a.txt" => (40, vec![]),
                "b.txt" => (30, vec![]),
                // Synthetic converter payload: budget rejects its asset bytes
                // before the ordinary retained-asset validation/import boundary.
                "c.txt" => (
                    1,
                    vec![brn_threads_intake::RetainedAsset {
                        name: "synthetic.png".into(),
                        media_type: "image/png".into(),
                        sha256: Sha256::digest([0; 70]).into(),
                        width: 1,
                        height: 1,
                        bytes: vec![0; 70],
                    }],
                ),
                _ => (5, vec![]),
            };
            Ok(ConversionResult {
                intent: brn_threads_intake::ImportIntent::FullNote,
                markdown: "x".repeat(size),
                assets,
                source_sha256: Sha256::digest(&doc.bytes).into(),
                source_reference: doc.source_reference,
                converter: brn_threads_intake::document_conversion_label("xberg", settings.ai)
                    .unwrap(),
                sources: vec![],
                coverage: brn_threads_intake::Coverage {
                    status: brn_threads_intake::CoverageStatus::Unchecked,
                    checked_items: vec![],
                    gaps: vec![],
                },
            })
        };
        let stop = AtomicBool::new(false);
        let first = workspace
            .scan_inbox_limited(&stop, 64, &mut conversion)
            .unwrap();
        assert_eq!((first.imported, first.failed, first.pending), (1, 0, 3));
        let second = workspace
            .scan_inbox_limited(&stop, 64, &mut conversion)
            .unwrap();
        assert_eq!((second.imported, second.failed, second.pending), (3, 1, 0));
        assert_eq!(calls, vec!["a.txt", "b.txt", "b.txt", "c.txt", "d.txt"]);
        let entries = workspace.inbox().unwrap().unwrap().entries;
        let failed = entries.iter().find(|e| e.name == "c.txt").unwrap();
        assert_eq!(failed.status, InboxStatus::Failed);
        assert!(failed.detail.contains("converted-output scan limit"));
        assert_eq!(
            workspace
                .records()
                .unwrap()
                .iter()
                .filter(|r| !r.archived && matches!(r.data, RecordData::Note(_)))
                .count(),
            3
        );
    }
}
