//! Shared Markdown observation and durable editing-buffer recovery, without publication.
mod files;
#[cfg(target_os = "macos")]
mod macos;

use brn_store::notes::VaultRecord;
pub use brn_store::notes::{
    ArtifactCleanup, FileOutcome, NoteAvailability, NoteBufferReceipt, NoteComparison,
    NoteErrorCode, NoteFailure, NoteReceipt, NoteRecordedResult, NoteRecovery, NoteResolution,
    NoteResult, NoteStamp, NoteSubmission, NoteView, SavePhase,
};
use files::{MacFiles, NoteNoticeSink, note_unsupported};
use std::{
    path::{Component, Path},
    sync::Arc,
};
use uuid::Uuid;

#[derive(Default)]
pub(super) struct NoteState {
    vault: Option<(VaultRecord, MacFiles)>,
    notices: NoteNoticeSink,
}

impl crate::Workspace {
    pub fn open_note(&mut self, op: Uuid, vault: &Path, relative: &Path) -> NoteResult<NoteView> {
        let result = (|| {
            if let Some(id) = self.store.note_enrollment_replay(op, vault, relative)? {
                return self.note(id);
            }
            validate_note_path(relative)?;
            let registered = self.store.registered_vault()?;
            let selected = select_vault(
                vault,
                registered
                    .as_ref()
                    .or_else(|| self.notes.vault.as_ref().map(|(owned, _)| owned)),
            )?;
            let mut registered_notes = Vec::new();
            for recovery in self.store.note_recoveries()? {
                let record = self.store.note_record(recovery.note_id)?;
                if record.relative_path == relative {
                    self.store.enroll_note_at(
                        op,
                        vault,
                        &selected,
                        relative,
                        record.baseline,
                        &recovery.baseline,
                    )?;
                    return self.note(record.id);
                }
                registered_notes.push(record);
            }
            self.open_note_vault(&selected)?;
            let files = &self.notes.vault.as_ref().unwrap().1;
            let observed = files.observe(relative)?;
            let same_file = |file: &brn_store::notes::FileFingerprint| {
                file.device == observed.fingerprint.device
                    && file.inode == observed.fingerprint.inode
            };
            for record in registered_notes {
                if record.vault_id == selected.id
                    && (same_file(&record.baseline)
                        || record.observed.as_ref().is_some_and(|(_, file)| same_file(file))
                        // An external replacement may not have a durable observation yet.
                        || files.observe(&record.relative_path)
                            .is_ok_and(|current| same_file(&current.fingerprint)))
                {
                    let mut error = note_failure(
                        NoteErrorCode::Conflict,
                        format!(
                            "file is already registered as {}; use that registered path",
                            record.relative_path.display()
                        ),
                    );
                    error.note_id = Some(record.id);
                    return Err(error);
                }
            }
            let recovered = self.store.enroll_note_at(
                op,
                vault,
                &selected,
                relative,
                observed.fingerprint,
                &observed.text,
            )?;
            self.note(recovered.note_id)
        })();
        result.map_err(|mut error: NoteFailure| {
            error.operation_id.get_or_insert(op);
            error
        })
    }

    /// Saved bytes come only from this call's filesystem observation, never recovery.
    pub fn note(&mut self, id: Uuid) -> NoteResult<NoteView> {
        let result = (|| {
            let record = self.store.note_record(id)?;
            let recovery = self.store.note_recovery(id)?.ok_or_else(|| {
                note_failure(NoteErrorCode::Storage, "registered note lacks recovery")
            })?;
            let mut view = NoteView {
                id,
                vault_id: record.vault_id,
                relative_path: record.relative_path,
                stamp: recovery.stamp,
                current_file_state: None,
                saved: None,
                buffer: recovery.working,
                availability: NoteAvailability::Available,
                availability_message: None,
                search_approval: record.search_approval,
            };
            let observation = self.acquire_note_vault(id).and_then(|()| {
                self.notes
                    .vault
                    .as_ref()
                    .unwrap()
                    .1
                    .observe(&view.relative_path)
            });
            match observation {
                Ok(observed) => {
                    view.current_file_state = Some(
                        self.store
                            .record_note_observation(id, &observed.fingerprint)?,
                    );
                    view.search_approval = self.store.note_record(id)?.search_approval;
                    if !recovery.pending_operations.is_empty() {
                        view.availability = NoteAvailability::Uncertain;
                        view.availability_message =
                            Some("note has unresolved recovery operations".into());
                    } else if observed.fingerprint != record.baseline {
                        view.availability = NoteAvailability::Conflict;
                        view.availability_message = Some("saved file differs from the editing baseline; explicit reconciliation is required".into());
                    }
                    view.saved = Some(observed.text);
                }
                Err(error) => {
                    view.availability = match error.code {
                        NoteErrorCode::Missing => NoteAvailability::Missing,
                        NoteErrorCode::Unsupported => NoteAvailability::Unsupported,
                        NoteErrorCode::Conflict | NoteErrorCode::StateChanged => {
                            NoteAvailability::Conflict
                        }
                        NoteErrorCode::SaveUncertain => NoteAvailability::Uncertain,
                        NoteErrorCode::VaultBusy => NoteAvailability::OwnedElsewhere,
                        NoteErrorCode::VaultUnavailable | NoteErrorCode::Io => {
                            NoteAvailability::Unavailable
                        }
                        NoteErrorCode::OperationConflict
                        | NoteErrorCode::WorkspaceBusy
                        | NoteErrorCode::Storage => return Err(error),
                    };
                    view.availability_message = Some(error.message);
                }
            }
            Ok(view)
        })();
        result.map_err(|mut error: NoteFailure| {
            error.note_id.get_or_insert(id);
            error
        })
    }

    pub fn save_note_buffer(&mut self, request: NoteSubmission) -> NoteResult<NoteBufferReceipt> {
        self.store.save_note_buffer(&request).map_err(|mut error| {
            error.operation_id.get_or_insert(request.operation_id);
            error.note_id.get_or_insert(request.note_id);
            error
        })
    }

    /// Metadata-only inspection remains available without the vault or its ownership.
    pub fn note_recoveries(&self) -> NoteResult<Vec<NoteRecovery>> {
        self.store.note_recoveries()
    }

    fn acquire_note_vault(&mut self, note_id: Uuid) -> NoteResult<()> {
        let vault = self.store.note_vault(note_id)?;
        self.open_note_vault(&vault)
    }

    fn open_note_vault(&mut self, vault: &VaultRecord) -> NoteResult<()> {
        if let Some((owned, files)) = &self.notes.vault {
            if owned.id != vault.id || owned.identity != vault.identity {
                return Err(note_failure(
                    NoteErrorCode::VaultUnavailable,
                    "workspace is bound to another vault; use a separate data directory",
                ));
            }
            return files.validate_root();
        }
        let files = MacFiles::open(vault, &self.path, Arc::clone(&self.notes.notices))?;
        self.notes.vault = Some((vault.clone(), files));
        Ok(())
    }
}

fn note_failure(code: NoteErrorCode, message: impl Into<String>) -> NoteFailure {
    NoteFailure {
        code,
        message: message.into(),
        operation_id: None,
        note_id: None,
        phase: None,
        filesystem_outcome: FileOutcome::NotApplied,
        recovery_available: false,
    }
}

fn validate_note_path(path: &Path) -> NoteResult<()> {
    let spelling = path
        .to_str()
        .ok_or_else(|| note_unsupported("note path is not UTF-8"))?;
    if spelling
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || path.extension().and_then(|ext| ext.to_str()) != Some("md")
    {
        return Err(note_unsupported(
            "note must be a contained relative .md path",
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn select_vault(root: &Path, registered: Option<&VaultRecord>) -> NoteResult<VaultRecord> {
    use brn_store::notes::VaultIdentity;
    use std::os::unix::fs::MetadataExt;

    if let Some(registered) = registered
        && root == registered.root
    {
        return Ok(registered.clone());
    }
    let canonical = std::fs::canonicalize(root)
        .map_err(|error| note_failure(NoteErrorCode::VaultUnavailable, error.to_string()))?;
    let metadata = std::fs::metadata(&canonical)
        .map_err(|error| note_failure(NoteErrorCode::VaultUnavailable, error.to_string()))?;
    if !metadata.is_dir() {
        return Err(note_failure(
            NoteErrorCode::VaultUnavailable,
            "vault root is not a directory",
        ));
    }
    let identity = VaultIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    };
    if let Some(registered) = registered {
        if identity != registered.identity {
            return Err(note_failure(
                NoteErrorCode::VaultUnavailable,
                "workspace is bound to another vault; use a separate data directory",
            ));
        }
        return Ok(registered.clone());
    }
    Ok(VaultRecord {
        id: Uuid::new_v4(),
        root: canonical,
        identity,
    })
}

#[cfg(not(target_os = "macos"))]
fn select_vault(_: &Path, _: Option<&VaultRecord>) -> NoteResult<VaultRecord> {
    Err(note_unsupported("Markdown vault editing requires macOS"))
}
