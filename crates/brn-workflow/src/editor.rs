//! Manual Markdown edits. WorkStore journals intent; the vault owns saved bytes.
use crate::files::{FileErrorCode, FileFailure};
use crate::{ErrorKind, Result, WorkflowError, app::App, files::MacFiles, vault::VaultPath};
pub use brn_store::files::FileFingerprint;
use brn_store::files::VaultRecord;
pub use brn_store::work::editor::{
    EditRequest, EditStamp, EditorRecord, SaveIntent, SaveOutcome, SaveReceipt, SaveRequest,
};
use serde::{Deserialize, Serialize};
use std::{cell::Cell, path::Path, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorView {
    pub record: EditorRecord,
    /// Fresh disk bytes, never a stored recovery buffer.
    pub saved: Option<String>,
    pub observed: Option<FileFingerprint>,
    pub conflict: bool,
    pub pending: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReloadRequest {
    pub path: String,
    pub expected: EditStamp,
    pub observed: FileFingerprint,
    pub discard: bool,
}

#[derive(Default)]
pub(crate) struct EditorState {
    pub(crate) files: Option<MacFiles>,
}

pub(crate) fn file_error(error: FileFailure) -> WorkflowError {
    let kind = match error.code {
        #[cfg(target_os = "macos")]
        FileErrorCode::Conflict | FileErrorCode::Missing => ErrorKind::ContextStale,
        #[cfg(target_os = "macos")]
        FileErrorCode::VaultUnavailable => ErrorKind::VaultUnavailable,
        #[cfg(target_os = "macos")]
        FileErrorCode::VaultBusy => ErrorKind::WorkspaceBusy,
        #[cfg(target_os = "macos")]
        FileErrorCode::SaveUncertain => ErrorKind::SaveUncertain,
        FileErrorCode::Unsupported => ErrorKind::ToolRejected,
        #[cfg(target_os = "macos")]
        FileErrorCode::Io => ErrorKind::Other,
    };
    WorkflowError::typed(kind, error.message)
}

fn validate_path(path: &str) -> Result<()> {
    VaultPath::parse(path)
        .map(|_| ())
        .map_err(|e| WorkflowError::typed(ErrorKind::ToolRejected, e.to_string()))
}

impl App {
    pub(crate) fn proposals_have_unresolved(&self) -> Result<bool> {
        use brn_store::work::proposal_apply::ApplyOutcome;
        let mut blocked = false;
        for id in self.store.proposal_apply_ids()? {
            let journal = self.store.proposal_apply(id)?.ok_or_else(|| {
                WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "listed approval journal disappeared",
                )
            })?;
            // Check every indexed row, including settled history, without
            // retaining all full review bodies during startup/current reads.
            blocked |= journal
                .receipt
                .as_ref()
                .is_none_or(|receipt| receipt.outcome == ApplyOutcome::Uncertain);
        }
        Ok(blocked)
    }

    pub(crate) fn current_evidence_blocked(&self) -> Result<bool> {
        Ok(self.proposals_have_unresolved()? || self.store.editor_saves()?.iter().any(unresolved))
    }

    pub(crate) fn require_current_evidence(&self) -> Result<()> {
        if self.current_evidence_blocked()? {
            self.set_current_tool_barrier(true);
            return Err(WorkflowError::typed(
                ErrorKind::SaveUncertain,
                "reconcile interrupted durable changes before reading current knowledge",
            ));
        }
        Ok(())
    }

    pub(crate) fn synchronize_current_barrier(&self) -> Result<()> {
        // Pending work makes tools() refuse, so access the already-owned Arc directly.
        self.set_current_tool_barrier(self.current_evidence_blocked()?);
        Ok(())
    }
    pub(crate) fn editor_files(&mut self) -> Result<&MacFiles> {
        if self.editor.files.is_none() {
            let root = self.root.as_deref().ok_or_else(|| {
                WorkflowError::typed(ErrorKind::VaultNotBound, "choose a vault before editing")
            })?;
            let record: VaultRecord = match self.store.setting("vault.editor_identity")? {
                Some(json) => serde_json::from_str(&json)
                    .map_err(|_| WorkflowError::msg("invalid saved vault identity"))?,
                None => {
                    #[cfg(target_os = "macos")]
                    {
                        use std::os::unix::fs::MetadataExt;
                        let meta = root.symlink_metadata().map_err(|_| {
                            WorkflowError::typed(ErrorKind::VaultUnavailable, "vault unavailable")
                        })?;
                        if !meta.is_dir() || meta.file_type().is_symlink() {
                            return Err(WorkflowError::typed(
                                ErrorKind::VaultUnavailable,
                                "vault root changed",
                            ));
                        }
                        VaultRecord {
                            id: Uuid::new_v4(),
                            root: root.to_owned(),
                            identity: brn_store::files::VaultIdentity {
                                device: meta.dev(),
                                inode: meta.ino(),
                            },
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    return Err(WorkflowError::typed(
                        ErrorKind::ToolRejected,
                        "Markdown Save requires macOS",
                    ));
                }
            };
            if record.root != root {
                return Err(WorkflowError::typed(
                    ErrorKind::VaultUnavailable,
                    "editor is bound to another vault",
                ));
            }
            let files = MacFiles::open(&record, self.store.data_dir(), Arc::default())
                .map_err(file_error)?;
            self.store.set_setting(
                "vault.editor_identity",
                &serde_json::to_string(&record)
                    .map_err(|_| WorkflowError::msg("could not encode vault identity"))?,
            )?;
            self.editor.files = Some(files);
        }
        let files = self.editor.files.as_ref().expect("acquired editor vault");
        files.validate_root().map_err(file_error)?;
        Ok(files)
    }

    pub fn open_editor(&mut self, path: &str) -> Result<EditorView> {
        validate_path(path)?;
        // Recovery stays readable even if the vault is missing, replaced or owned elsewhere.
        let existing = self.store.editor(path)?;
        let intents = self.store.editor_saves()?;
        let observed = self
            .editor_files()
            .and_then(|files| files.observe(Path::new(path)).map_err(file_error));
        let reservation = match self.editor.files.as_ref() {
            Some(files) => copy_reservation(
                &intents,
                files,
                Path::new(path),
                observed.as_ref().ok().map(|value| &value.fingerprint),
            ),
            None => Ok(intents
                .iter()
                .find(|intent| {
                    unresolved(intent) && intent.request.destination.as_deref() == Some(path)
                })
                .map(|intent| intent.request.operation_id)),
        };
        let reservation_failed = reservation.is_err();
        let reserved = match reservation {
            Ok(reserved) => reserved,
            Err(error) if existing.is_none() => return Err(error),
            Err(_) => intents
                .iter()
                .find(|intent| {
                    unresolved(intent) && intent.request.destination.as_deref() == Some(path)
                })
                .map(|intent| intent.request.operation_id),
        };
        if existing.is_none() && reserved.is_some() {
            return Err(WorkflowError::typed(
                ErrorKind::SaveUncertain,
                "copy destination is reserved; reconcile that operation before opening it",
            ));
        }
        let record = match existing {
            Some(record) => record,
            None => {
                let observed = observed.as_ref().map_err(Clone::clone)?;
                // A path alias must not create a second editable identity for one file.
                if self
                    .store
                    .editors()?
                    .iter()
                    .any(|record| same_identity(&record.baseline, &observed.fingerprint))
                {
                    return Err(WorkflowError::typed(
                        ErrorKind::ContextStale,
                        "file is already registered at another path",
                    ));
                }
                self.store
                    .open_editor(path, &observed.fingerprint, &observed.text)?
            }
        };
        let pending = intents
            .into_iter()
            .filter(|intent| intent.request.edit.path == path && unresolved(intent))
            .map(|intent| intent.request.operation_id)
            .chain(reserved)
            .collect::<Vec<_>>();
        let conflict = reservation_failed
            || self.proposals_have_unresolved()?
            || !pending.is_empty()
            || observed.as_ref().is_err()
            || observed
                .as_ref()
                .is_ok_and(|value| value.fingerprint != record.baseline);
        Ok(EditorView {
            record,
            observed: observed
                .as_ref()
                .ok()
                .map(|value| value.fingerprint.clone()),
            saved: observed.ok().map(|value| value.text),
            conflict,
            pending,
        })
    }

    pub fn reload_editor(&mut self, request: &ReloadRequest) -> Result<EditorRecord> {
        validate_path(&request.path)?;
        self.require_proposals_reconciled()?;
        let observed = self
            .editor_files()?
            .observe(Path::new(&request.path))
            .map_err(file_error)?;
        if observed.fingerprint != request.observed {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "reviewed disk state changed before reload",
            ));
        }
        Ok(self.store.reload_editor(
            &request.path,
            request.expected,
            &observed.fingerprint,
            &observed.text,
            request.discard,
        )?)
    }

    pub fn recover_editor(&mut self, request: &EditRequest) -> Result<EditorRecord> {
        validate_path(&request.path)?;
        Ok(self.store.recover_editor(request)?)
    }

    pub fn save_editor(&mut self, request: &SaveRequest) -> Result<SaveReceipt> {
        validate_path(&request.edit.path)?;
        if let Some(path) = &request.destination {
            validate_path(path)?;
        }
        let destination = request.destination.as_deref().unwrap_or(&request.edit.path);
        let staging =
            Path::new(destination).with_file_name(format!(".brn-{}.stage", request.operation_id));
        // Bound replay is checked before current filesystem state. It never writes again.
        if let Some((receipt, no_op)) = self.store.editor_save_replay(request, &staging)? {
            if receipt.outcome == SaveOutcome::Applied || no_op {
                return Ok(receipt);
            }
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "recorded save did not apply; use a fresh operation after resolving the conflict",
            ));
        }
        if let Some(previous) = self.store.editor_save(request.operation_id)? {
            if previous.request != *request || previous.staging != staging {
                return Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "save UUID has another payload",
                ));
            }
            return receipt_or_uncertain(&previous);
        }
        self.require_proposals_reconciled()?;
        // Fence retained tool leases before the journal becomes visible.
        self.set_current_tool_barrier(true);
        let intent = match self.store.begin_editor_save(request, &staging) {
            Ok(intent) => intent,
            Err(error) => {
                self.synchronize_current_barrier()?;
                return Err(error.into());
            }
        };
        self.synchronize_current_barrier()?;
        checkpoint("intent");
        let attempted = Cell::new(false);
        let result = self.execute_editor_save(&intent, &attempted);
        match result {
            Ok(installed) => {
                let outcome = if installed.is_some() {
                    SaveOutcome::Applied
                } else {
                    SaveOutcome::NotApplied
                };
                let receipt = self.store.finish_editor_save(
                    request.operation_id,
                    outcome,
                    installed.as_ref(),
                )?;
                self.synchronize_current_barrier()?;
                checkpoint("receipt");
                // Index failures cannot turn a durable Markdown receipt into a write failure.
                let _ = self.retire_editor_artifacts();
                let _ = self.refresh();
                Ok(receipt)
            }
            Err(error) => {
                let outcome = if attempted.get() {
                    SaveOutcome::Uncertain
                } else {
                    SaveOutcome::NotApplied
                };
                self.store
                    .finish_editor_save(request.operation_id, outcome, None)?;
                self.synchronize_current_barrier()?;
                let _ = self.retire_editor_artifacts();
                if attempted.get() {
                    Err(WorkflowError::typed(
                        ErrorKind::SaveUncertain,
                        format!(
                            "save {} outcome uncertain; recover/reconcile before another original Save: {}",
                            request.operation_id, error.message
                        ),
                    ))
                } else {
                    Err(error)
                }
            }
        }
    }

    fn require_proposals_reconciled(&self) -> Result<()> {
        if self.proposals_have_unresolved()? {
            self.set_current_tool_barrier(true);
            return Err(WorkflowError::typed(
                ErrorKind::SaveUncertain,
                "reconcile the interrupted proposal before Save, Save Copy or reload",
            ));
        }
        Ok(())
    }

    fn execute_editor_save(
        &mut self,
        intent: &SaveIntent,
        attempted: &Cell<bool>,
    ) -> Result<Option<FileFingerprint>> {
        self.editor_files()?;
        let files = self.editor.files.as_ref().expect("acquired editor vault");
        let destination = Path::new(
            intent
                .request
                .destination
                .as_deref()
                .unwrap_or(&intent.request.edit.path),
        );
        let original = Path::new(&intent.request.edit.path);
        let store = &mut self.store;
        files
            .coordinate(destination, || {
                // Workflow/store errors are relayed through the adapter's callback, preserving kind outside.
                Ok((|| -> Result<Option<FileFingerprint>> {
                    let copy = intent.request.destination.is_some();
                    if copy {
                        files
                            .validate_copy_destination(destination)
                            .map_err(file_error)?;
                        if files
                            .aliases_original(destination, original)
                            .map_err(file_error)?
                            || files.artifact(destination).map_err(file_error)?.is_some()
                        {
                            return Err(WorkflowError::typed(
                                ErrorKind::ContextStale,
                                "copy destination is occupied or aliases the original",
                            ));
                        }
                        for other in store.editor_saves()? {
                            if other.request.operation_id != intent.request.operation_id
                                && other.request.destination.is_some()
                                && unresolved(&other)
                                && files
                                    .reserved_copy_path_matches(
                                        destination,
                                        Path::new(
                                            other.request.destination.as_ref().expect("copy"),
                                        ),
                                    )
                                    .map_err(file_error)?
                            {
                                return Err(WorkflowError::typed(
                                    ErrorKind::ContextStale,
                                    "copy destination has unresolved work",
                                ));
                            }
                        }
                    } else {
                        let observed = files
                            .observe_uncoordinated(destination)
                            .map_err(file_error)?;
                        if copy_reservation(
                            &store.editor_saves()?,
                            files,
                            destination,
                            Some(&observed.fingerprint),
                        )?
                        .is_some()
                        {
                            return Err(WorkflowError::typed(
                                ErrorKind::SaveUncertain,
                                "destination belongs to unresolved copy work",
                            ));
                        }
                        if observed.fingerprint != intent.baseline
                            || observed.text != intent.baseline_text
                        {
                            return Err(WorkflowError::typed(
                                ErrorKind::ContextStale,
                                "Markdown changed externally; local edits retained",
                            ));
                        }
                        if intent.request.edit.text == intent.baseline_text {
                            store.mark_editor_save_noop(intent.request.operation_id)?;
                            return Ok(None);
                        }
                    }
                    if files
                        .artifact(&intent.staging)
                        .map_err(file_error)?
                        .is_some()
                    {
                        return Err(WorkflowError::typed(
                            ErrorKind::ContextStale,
                            "save staging path is occupied",
                        ));
                    }
                    let parent = files.parent_identity(destination).map_err(file_error)?;
                    store.bind_editor_save_parent(intent.request.operation_id, &parent)?;
                    let prepared = if copy {
                        files.prepare_copy(
                            intent.request.operation_id,
                            &intent.staging,
                            destination,
                            intent.request.edit.text.as_bytes(),
                        )
                    } else {
                        files.prepare_replace(
                            intent.request.operation_id,
                            &intent.staging,
                            destination,
                            intent.request.edit.text.as_bytes(),
                        )
                    }
                    .map_err(file_error)?;
                    checkpoint("stage");
                    store.prepare_editor_save(intent.request.operation_id, &prepared)?;
                    checkpoint("prepared");
                    if files.parent_identity(destination).map_err(file_error)? != parent {
                        return Err(WorkflowError::typed(
                            ErrorKind::ContextStale,
                            "save parent changed",
                        ));
                    }
                    if !copy
                        && files
                            .observe_uncoordinated(destination)
                            .map_err(file_error)?
                            .fingerprint
                            != intent.baseline
                    {
                        return Err(WorkflowError::typed(
                            ErrorKind::ContextStale,
                            "Markdown changed before installation",
                        ));
                    }
                    attempted.set(true);
                    if copy {
                        files.install_exclusive(&prepared, destination)
                    } else {
                        files.exchange(&prepared, destination)
                    }
                    .map_err(file_error)?;
                    checkpoint("exchange");
                    files.flush_artifact(destination).map_err(file_error)?;
                    if !copy {
                        files.flush_artifact(&intent.staging).map_err(file_error)?;
                    }
                    checkpoint("synced");
                    let installed = files
                        .observe_uncoordinated(destination)
                        .map_err(file_error)?;
                    if installed.fingerprint != prepared.fingerprint
                        || installed.text != intent.request.edit.text
                        || files.parent_identity(destination).map_err(file_error)? != parent
                    {
                        return Err(WorkflowError::typed(
                            ErrorKind::SaveUncertain,
                            "installed file proof changed",
                        ));
                    }
                    if copy {
                        if files
                            .artifact(&intent.staging)
                            .map_err(file_error)?
                            .is_some()
                        {
                            return Err(WorkflowError::typed(
                                ErrorKind::SaveUncertain,
                                "copy staging was not consumed",
                            ));
                        }
                    } else {
                        let displaced = files
                            .observe_uncoordinated(&intent.staging)
                            .map_err(file_error)?;
                        if displaced.fingerprint != intent.baseline
                            || displaced.text != intent.baseline_text
                        {
                            return Err(WorkflowError::typed(
                                ErrorKind::SaveUncertain,
                                "unexpected displaced file retained",
                            ));
                        }
                    }
                    checkpoint("verified");
                    Ok(Some(installed.fingerprint))
                })())
            })
            .map_err(file_error)?
    }

    // Keep the latest Applied original pair. Retire only older, identity-proven
    // operation artifacts; unexpected occupants and unresolved work never expire.
    fn retire_editor_artifacts(&mut self) -> Result<()> {
        if self.editor.files.is_none() {
            return Ok(());
        }
        let intents = self.store.editor_saves()?;
        let mut latest = std::collections::HashMap::new();
        for intent in &intents {
            if intent.request.destination.is_none()
                && intent
                    .receipt
                    .as_ref()
                    .is_some_and(|receipt| receipt.outcome == SaveOutcome::Applied)
            {
                latest.insert(
                    intent.request.edit.path.clone(),
                    intent.request.operation_id,
                );
            }
        }
        let files = self.editor.files.as_ref().expect("acquired editor vault");
        for intent in intents {
            let Some(receipt) = &intent.receipt else {
                continue;
            };
            if receipt.outcome == SaveOutcome::Uncertain
                || latest.get(&intent.request.edit.path) == Some(&intent.request.operation_id)
            {
                continue;
            }
            if intent.parent.as_ref().is_some_and(|parent| {
                files.parent_identity(&intent.staging).as_ref().ok() != Some(parent)
            }) {
                continue;
            }
            let artifact = match files.artifact(&intent.staging) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if let Some(artifact) = artifact {
                let expected = match receipt.outcome {
                    SaveOutcome::Applied if intent.request.destination.is_none() => {
                        Some(&intent.baseline)
                    }
                    SaveOutcome::NotApplied => intent
                        .prepared
                        .as_ref()
                        .map(|prepared| &prepared.fingerprint),
                    _ => None,
                };
                let Some(expected) = expected else {
                    continue;
                };
                if artifact.identity.kind != brn_store::files::ArtifactKind::Regular
                    || artifact.identity.device != expected.device
                    || artifact.identity.inode != expected.inode
                    || artifact.identity.len != expected.len
                    || artifact.sha256 != Some(expected.sha256)
                {
                    continue;
                }
                if files.remove_artifact(&artifact).is_err() {
                    continue;
                }
            }
            self.store
                .compact_editor_save(intent.request.operation_id)?;
        }
        Ok(())
    }

    /// Inspect proof and finish a journal. Never redo a filesystem write.
    pub fn reconcile_editor(&mut self, operation: Uuid) -> Result<SaveReceipt> {
        let intent = match self.store.editor_save(operation)? {
            Some(intent) => intent,
            None => {
                return self.store.editor_save_receipt(operation)?.ok_or_else(|| {
                    WorkflowError::typed(ErrorKind::NotFound, "save operation not found")
                });
            }
        };
        if !unresolved(&intent) {
            return Ok(intent.receipt.expect("settled save"));
        }
        self.editor_files()?;
        let files = self.editor.files.as_ref().expect("acquired editor vault");
        let destination = Path::new(
            intent
                .request
                .destination
                .as_deref()
                .unwrap_or(&intent.request.edit.path),
        );
        let (outcome, installed) = files
            .coordinate(destination, || {
                if intent.parent.as_ref().is_some_and(|parent| {
                    files.parent_identity(destination).as_ref().ok() != Some(parent)
                }) {
                    return Ok((SaveOutcome::Uncertain, None));
                }
                let disk = files.observe_uncoordinated(destination).ok();
                let stage = files.observe_uncoordinated(&intent.staging).ok();
                let artifact = files.artifact(&intent.staging)?;
                let copy = intent.request.destination.is_some();
                if let (Some(prepared), Some(disk)) = (&intent.prepared, disk.as_ref())
                    && intent.parent.is_some()
                    && disk.fingerprint == prepared.fingerprint
                    && disk.text == intent.request.edit.text
                    && if copy {
                        artifact.is_none()
                    } else {
                        stage.as_ref().is_some_and(|stage| {
                            stage.fingerprint == intent.baseline
                                && stage.text == intent.baseline_text
                        })
                    }
                {
                    files.flush_artifact(destination)?;
                    if !copy {
                        files.flush_artifact(&intent.staging)?;
                    }
                    if files.observe_uncoordinated(destination)?.fingerprint != disk.fingerprint
                        || (!copy
                            && files.observe_uncoordinated(&intent.staging)?.fingerprint
                                != intent.baseline)
                    {
                        return Ok((SaveOutcome::Uncertain, None));
                    }
                    return Ok((SaveOutcome::Applied, Some(disk.fingerprint.clone())));
                }
                let unchanged = if copy {
                    disk.as_ref().is_none_or(|disk| {
                        intent.prepared.as_ref().is_none_or(|prepared| {
                            !same_identity(&disk.fingerprint, &prepared.fingerprint)
                        })
                    })
                } else {
                    disk.as_ref()
                        .is_some_and(|disk| disk.fingerprint == intent.baseline)
                };
                let unconsumed = match (&intent.prepared, stage.as_ref(), artifact.as_ref()) {
                    (None, None, None) => true,
                    (Some(prepared), Some(stage), Some(_)) => {
                        stage.fingerprint == prepared.fingerprint
                    }
                    _ => false,
                };
                Ok((
                    if unchanged && unconsumed {
                        SaveOutcome::NotApplied
                    } else {
                        SaveOutcome::Uncertain
                    },
                    None,
                ))
            })
            .map_err(file_error)?;
        let receipt = self
            .store
            .finish_editor_save(operation, outcome, installed.as_ref())?;
        self.synchronize_current_barrier()?;
        let _ = self.retire_editor_artifacts();
        let _ = self.refresh();
        Ok(receipt)
    }
}

fn same_identity(left: &FileFingerprint, right: &FileFingerprint) -> bool {
    left.device == right.device && left.inode == right.inode
}
fn unresolved(intent: &SaveIntent) -> bool {
    intent
        .receipt
        .as_ref()
        .is_none_or(|receipt| receipt.outcome == SaveOutcome::Uncertain)
}

fn copy_reservation(
    intents: &[SaveIntent],
    files: &MacFiles,
    path: &Path,
    observed: Option<&FileFingerprint>,
) -> Result<Option<Uuid>> {
    for intent in intents {
        if !unresolved(intent) {
            continue;
        }
        let Some(destination) = &intent.request.destination else {
            continue;
        };
        if files
            .reserved_copy_path_matches(path, Path::new(destination))
            .map_err(file_error)?
            || observed.is_some_and(|file| {
                intent
                    .prepared
                    .as_ref()
                    .is_some_and(|prepared| same_identity(file, &prepared.fingerprint))
            })
        {
            return Ok(Some(intent.request.operation_id));
        }
    }
    Ok(None)
}
fn receipt_or_uncertain(intent: &SaveIntent) -> Result<SaveReceipt> {
    match &intent.receipt {
        Some(receipt) if receipt.outcome == SaveOutcome::Applied => Ok(receipt.clone()),
        Some(receipt)
            if receipt.outcome == SaveOutcome::NotApplied
                && intent.request.destination.is_none()
                && intent.no_op =>
        {
            Ok(receipt.clone())
        }
        Some(receipt) if receipt.outcome == SaveOutcome::NotApplied => Err(WorkflowError::typed(
            ErrorKind::ContextStale,
            "recorded save did not apply; use a fresh operation after resolving the conflict",
        )),
        _ => Err(WorkflowError::typed(
            ErrorKind::SaveUncertain,
            format!(
                "save {} needs reconciliation; replay cannot write",
                intent.request.operation_id
            ),
        )),
    }
}

fn checkpoint(_phase: &str) {
    #[cfg(test)]
    #[cfg(test)]
    CHECKPOINT_HOOK.with(|hook| {
        if let Some(hook) = hook.borrow().as_ref() {
            hook(_phase);
        }
    });
    #[cfg(test)]
    if CRASH_PHASE.with(|phase| phase.borrow().as_deref() == Some(_phase)) {
        std::process::exit(71);
    }
}

#[cfg(test)]
type CheckpointHook = Box<dyn Fn(&str)>;

#[cfg(test)]
thread_local! {
    static CRASH_PHASE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    static CHECKPOINT_HOOK: std::cell::RefCell<Option<CheckpointHook>> = const { std::cell::RefCell::new(None) };
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::app::AppConfig;
    use std::{fs, path::PathBuf, process::Command};

    struct Fixture {
        base: tempfile::TempDir,
    }
    impl Fixture {
        fn new() -> Self {
            let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            fs::create_dir(base.path().join("data")).unwrap();
            fs::create_dir(base.path().join("vault")).unwrap();
            fs::write(
                base.path().join("vault/a.md"),
                "\u{feff}---\r\ntype: topic\r\n---\r\nTere õhtust\n",
            )
            .unwrap();
            Self { base }
        }
        fn app(&self) -> App {
            open(self.base.path())
        }
        fn note(&self) -> PathBuf {
            self.base.path().join("vault/a.md")
        }
    }
    fn open(base: &Path) -> App {
        App::open(
            &base.join("data"),
            AppConfig {
                vault_root: Some(base.join("vault")),
                credentials_dir: Some(base.join("credentials")),
                model_dir: None,
            },
        )
        .unwrap()
    }
    fn request(record: &EditorRecord, text: &str) -> SaveRequest {
        SaveRequest {
            operation_id: Uuid::new_v4(),
            edit: EditRequest {
                path: record.path.clone(),
                expected: record.stamp,
                generation: record.stamp.generation + 1,
                text: text.into(),
            },
            destination: None,
        }
    }

    #[test]
    fn save_exact_bytes_replay_and_restart_recovery() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        let initial = app.open_editor("a.md").unwrap();
        assert_eq!(
            initial.record.text.as_bytes(),
            fs::read(fixture.note()).unwrap()
        );
        let text = "\u{feff}---\r\nunrelated: väärtus\r\n---\r\nÕun\n\r\n";
        let request = request(&initial.record, text);
        let receipt = app.save_editor(&request).unwrap();
        assert_eq!(receipt.outcome, SaveOutcome::Applied);
        assert_eq!(fs::read(fixture.note()).unwrap(), text.as_bytes());
        fs::write(fixture.note(), "later outside edit").unwrap();
        assert_eq!(app.save_editor(&request).unwrap(), receipt);
        assert_eq!(fs::read(fixture.note()).unwrap(), b"later outside edit");
        let mut changed = request.clone();
        changed.edit.text.push('!');
        assert_eq!(
            app.save_editor(&changed).unwrap_err().kind,
            ErrorKind::OperationConflict
        );
        let record = app.open_editor("a.md").unwrap().record;
        let edit = EditRequest {
            path: "a.md".into(),
            expected: record.stamp,
            generation: record.stamp.generation + 1,
            text: "unfinished õ".into(),
        };
        app.recover_editor(&edit).unwrap();
        drop(app);
        let mut app = fixture.app();
        let view = app.open_editor("a.md").unwrap();
        assert_eq!(view.record.text, "unfinished õ");
        assert_eq!(view.saved.as_deref(), Some("later outside edit"));
        assert!(view.conflict);
    }

    #[test]
    fn conflicts_missing_files_and_exclusive_copies_preserve_every_occupant() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        let view = app.open_editor("a.md").unwrap();
        let mut save = request(&view.record, "my edits");
        fs::write(fixture.note(), "external").unwrap();
        assert_eq!(
            app.save_editor(&save).unwrap_err().kind,
            ErrorKind::ContextStale
        );
        assert_eq!(fs::read(fixture.note()).unwrap(), b"external");
        assert_eq!(
            app.work_store().editor("a.md").unwrap().unwrap().text,
            "my edits"
        );
        // Even equal bytes at a replaced inode do not establish editing identity.
        fs::rename(fixture.note(), fixture.note().with_extension("old")).unwrap();
        fs::write(fixture.note(), &view.record.baseline_text).unwrap();
        save.operation_id = Uuid::new_v4();
        assert_eq!(
            app.save_editor(&save).unwrap_err().kind,
            ErrorKind::ContextStale
        );
        fs::remove_file(fixture.note()).unwrap();
        save.operation_id = Uuid::new_v4();
        assert_eq!(
            app.save_editor(&save).unwrap_err().kind,
            ErrorKind::ContextStale
        );
        assert!(!fixture.note().exists());
        save.operation_id = Uuid::new_v4();
        save.destination = Some("rescue.md".into());
        assert_eq!(
            app.save_editor(&save).unwrap().outcome,
            SaveOutcome::Applied
        );
        assert_eq!(
            fs::read(fixture.base.path().join("vault/rescue.md")).unwrap(),
            b"my edits"
        );
        save.operation_id = Uuid::new_v4();
        assert_eq!(
            app.save_editor(&save).unwrap_err().kind,
            ErrorKind::ContextStale
        );
        assert_eq!(
            fs::read(fixture.base.path().join("vault/rescue.md")).unwrap(),
            b"my edits"
        );
        save.operation_id = Uuid::new_v4();
        save.destination = Some("RESCUE.md".into());
        // Alias veto works on case-insensitive target volumes (the adapter qualifies names).
        if fixture.base.path().join("vault/RESCUE.md").exists() {
            assert!(app.save_editor(&save).is_err());
        }
    }

    #[test]
    fn empty_noop_link_root_and_ownership_guards() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        let initial = app.open_editor("a.md").unwrap();
        let save = request(&initial.record, "");
        let applied = app.save_editor(&save).unwrap();
        assert_eq!(fs::read(fixture.note()).unwrap(), b"");
        let mut noop = save.clone();
        noop.operation_id = Uuid::new_v4();
        noop.edit.expected = applied.stamp;
        let no_op_receipt = app.save_editor(&noop).unwrap();
        assert_eq!(no_op_receipt.outcome, SaveOutcome::NotApplied);
        assert_eq!(app.save_editor(&noop).unwrap(), no_op_receipt);
        assert_eq!(
            app.work_store().editor_previous("a.md").unwrap().unwrap(),
            (initial.record.text, String::new())
        );
        drop(app);
        let mut app = fixture.app();
        fs::write(fixture.note(), "outside edit after no-op").unwrap();
        assert_eq!(app.save_editor(&noop).unwrap(), no_op_receipt);
        assert_eq!(
            fs::read(fixture.note()).unwrap(),
            b"outside edit after no-op"
        );
        fs::hard_link(fixture.note(), fixture.base.path().join("vault/hard.md")).unwrap();
        assert!(app.open_editor("hard.md").is_err());
        std::os::unix::fs::symlink(fixture.note(), fixture.base.path().join("vault/link.md"))
            .unwrap();
        assert!(app.open_editor("link.md").is_err());
        let other_data = fixture.base.path().join("other-data");
        fs::create_dir(&other_data).unwrap();
        let mut other = App::open(
            &other_data,
            AppConfig {
                vault_root: Some(fixture.base.path().join("vault")),
                credentials_dir: Some(fixture.base.path().join("other-credentials")),
                model_dir: None,
            },
        )
        .unwrap();
        assert_eq!(
            other.open_editor("a.md").unwrap_err().kind,
            ErrorKind::WorkspaceBusy
        );
        drop(other);
        fs::rename(
            fixture.base.path().join("vault"),
            fixture.base.path().join("moved-vault"),
        )
        .unwrap();
        fs::create_dir(fixture.base.path().join("vault")).unwrap();
        fs::write(fixture.note(), "replacement root").unwrap();
        let mut save = request(&app.open_editor("a.md").unwrap().record, "don't overwrite");
        save.operation_id = Uuid::new_v4();
        assert_eq!(
            app.save_editor(&save).unwrap_err().kind,
            ErrorKind::VaultUnavailable
        );
        assert_eq!(fs::read(fixture.note()).unwrap(), b"replacement root");
    }

    #[test]
    fn process_interruption_reconciles_without_replaying_filesystem_writes() {
        for copy in [false, true] {
            for phase in [
                "intent", "stage", "prepared", "exchange", "synced", "verified", "receipt",
            ] {
                let fixture = Fixture::new();
                let status = Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", "editor::tests::crash_child", "--ignored"])
                    .env("BRN_EDITOR_TEST_ROOT", fixture.base.path())
                    .env("BRN_EDITOR_TEST_PHASE", phase)
                    .env("BRN_EDITOR_TEST_COPY", if copy { "yes" } else { "no" })
                    .status()
                    .unwrap();
                assert_eq!(status.code(), Some(71), "child phase {phase}");
                let mut app = fixture.app();
                let intent = app.work_store().editor_saves().unwrap().pop().unwrap();
                assert_eq!(app.open_editor("a.md").unwrap().record.text, "saved õ\r\n");
                if phase != "receipt" {
                    assert_eq!(
                        app.save_editor(&intent.request).unwrap_err().kind,
                        ErrorKind::SaveUncertain
                    );
                }
                let receipt = app.reconcile_editor(intent.request.operation_id).unwrap();
                let expected = match phase {
                    "intent" | "prepared" => SaveOutcome::NotApplied,
                    "stage" => SaveOutcome::Uncertain,
                    _ => SaveOutcome::Applied,
                };
                assert_eq!(receipt.outcome, expected, "copy={copy}, phase={phase}");
                if expected == SaveOutcome::Applied {
                    let destination = fixture.base.path().join("vault").join(if copy {
                        "copy.md"
                    } else {
                        "a.md"
                    });
                    assert_eq!(fs::read(&destination).unwrap(), b"saved \xc3\xb5\r\n");
                    fs::write(&destination, "new outside edit").unwrap();
                    assert_eq!(
                        app.save_editor(&intent.request).unwrap().outcome,
                        SaveOutcome::Applied
                    );
                    assert_eq!(fs::read(destination).unwrap(), b"new outside edit");
                }
            }
        }
    }

    #[test]
    fn unexpected_displacement_is_retained_and_blocks_current_search_until_reconciled() {
        use brn_ai::ReadTools;
        let fixture = Fixture::new();
        let mut app = fixture.app();
        let record = app.open_editor("a.md").unwrap().record;
        let save = request(&record, "submitted");
        let stage = fixture
            .base
            .path()
            .join("vault")
            .join(format!(".brn-{}.stage", save.operation_id));
        let stage_in_hook = stage.clone();
        let tools = app.tools().unwrap();
        CHECKPOINT_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |phase| {
                if phase == "exchange" {
                    fs::write(&stage_in_hook, "unexpected external bytes").unwrap();
                }
            }))
        });
        let result = app.save_editor(&save);
        CHECKPOINT_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert_eq!(result.unwrap_err().kind, ErrorKind::SaveUncertain);
        assert_eq!(fs::read(&stage).unwrap(), b"unexpected external bytes");
        assert_eq!(fs::read(fixture.note()).unwrap(), b"submitted");
        assert!(tools.read_note("a.md").is_err());
        assert_eq!(
            app.search("submitted", crate::library::SearchMode::Keyword, 10)
                .unwrap_err()
                .kind,
            ErrorKind::SaveUncertain
        );
        assert_eq!(
            app.reconcile_editor(save.operation_id).unwrap().outcome,
            SaveOutcome::Uncertain
        );
        let next = request(&app.open_editor("a.md").unwrap().record, "next");
        assert_eq!(
            app.save_editor(&next).unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        // A rescue copy remains an explicit independent operation.
        let mut rescue = save;
        rescue.operation_id = Uuid::new_v4();
        rescue.destination = Some("rescue.md".into());
        assert_eq!(
            app.save_editor(&rescue).unwrap().outcome,
            SaveOutcome::Applied
        );
        assert_eq!(fs::read(stage).unwrap(), b"unexpected external bytes");
    }

    #[test]
    fn obsolete_artifacts_retire_to_compact_replay_while_latest_pair_remains() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        let initial = app.open_editor("a.md").unwrap().record;
        let first = request(&initial, "one");
        let first_receipt = app.save_editor(&first).unwrap();
        let first_stage = fixture
            .base
            .path()
            .join("vault")
            .join(format!(".brn-{}.stage", first.operation_id));
        assert!(first_stage.exists());
        let second = request(&app.open_editor("a.md").unwrap().record, "two");
        app.save_editor(&second).unwrap();
        assert!(!first_stage.exists());
        assert_eq!(app.work_store().editor_saves().unwrap().len(), 1);
        assert_eq!(
            app.work_store().editor_previous("a.md").unwrap().unwrap(),
            ("one".into(), "two".into())
        );
        assert_eq!(app.save_editor(&first).unwrap(), first_receipt);
        assert_eq!(
            app.reconcile_editor(first.operation_id).unwrap(),
            first_receipt
        );
        assert_eq!(fs::read(fixture.note()).unwrap(), b"two");
        let mut changed = first.clone();
        changed.edit.text.push('!');
        assert_eq!(
            app.save_editor(&changed).unwrap_err().kind,
            ErrorKind::OperationConflict
        );
        let second_stage = fixture
            .base
            .path()
            .join("vault")
            .join(format!(".brn-{}.stage", second.operation_id));
        fs::write(&second_stage, "unexpected artifact").unwrap();
        let third = request(&app.open_editor("a.md").unwrap().record, "three");
        app.save_editor(&third).unwrap();
        assert_eq!(fs::read(second_stage).unwrap(), b"unexpected artifact");
        assert!(
            app.work_store()
                .editor_save(second.operation_id)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn moved_prepared_copy_in_replacement_parent_does_not_prove_application() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.base.path().join("vault/folder")).unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "editor::tests::crash_child", "--ignored"])
            .env("BRN_EDITOR_TEST_ROOT", fixture.base.path())
            .env("BRN_EDITOR_TEST_PHASE", "exchange")
            .env("BRN_EDITOR_TEST_COPY", "yes")
            .env("BRN_EDITOR_TEST_DEST", "folder/copy.md")
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(71));
        fs::rename(
            fixture.base.path().join("vault/folder"),
            fixture.base.path().join("vault/old-folder"),
        )
        .unwrap();
        fs::create_dir(fixture.base.path().join("vault/folder")).unwrap();
        fs::rename(
            fixture.base.path().join("vault/old-folder/copy.md"),
            fixture.base.path().join("vault/folder/copy.md"),
        )
        .unwrap();
        let mut app = fixture.app();
        let intent = app.work_store().editor_saves().unwrap().pop().unwrap();
        assert_eq!(
            app.reconcile_editor(intent.request.operation_id)
                .unwrap()
                .outcome,
            SaveOutcome::Uncertain
        );
        assert_eq!(
            fs::read(fixture.base.path().join("vault/folder/copy.md")).unwrap(),
            b"saved \xc3\xb5\r\n"
        );
    }

    #[test]
    fn required_staging_failures_retain_buffer_and_do_not_overwrite_original() {
        for step in ["write", "attributes", "file_sync", "directory_sync"] {
            let fixture = Fixture::new();
            let mut app = fixture.app();
            let record = app.open_editor("a.md").unwrap().record;
            let save = request(&record, "recoverable");
            crate::files::PREPARE_FAILURE.with(|selected| selected.set(Some(step)));
            let result = app.save_editor(&save);
            crate::files::PREPARE_FAILURE.with(|selected| selected.set(None));
            assert!(result.is_err(), "step {step}");
            assert_eq!(
                fs::read(fixture.note()).unwrap(),
                record.baseline_text.as_bytes()
            );
            assert_eq!(
                app.work_store().editor("a.md").unwrap().unwrap().text,
                "recoverable"
            );
        }
    }

    #[test]
    fn refused_equal_text_save_stays_refused_on_bound_replay_after_restart() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        let record = app.open_editor("a.md").unwrap().record;
        fs::write(fixture.note(), "external").unwrap();
        let save = SaveRequest {
            operation_id: Uuid::new_v4(),
            edit: EditRequest {
                path: record.path,
                expected: record.stamp,
                generation: record.stamp.generation,
                text: record.text,
            },
            destination: None,
        };
        assert_eq!(
            app.save_editor(&save).unwrap_err().kind,
            ErrorKind::ContextStale
        );
        assert_eq!(
            app.save_editor(&save).unwrap_err().kind,
            ErrorKind::ContextStale
        );
        drop(app);
        let mut app = fixture.app();
        assert_eq!(
            app.save_editor(&save).unwrap_err().kind,
            ErrorKind::ContextStale
        );
        assert_eq!(fs::read(fixture.note()).unwrap(), b"external");
    }

    #[test]
    fn unavailable_cached_vault_with_pending_copy_keeps_original_buffer_visible() {
        let fixture = Fixture::new();
        let mut app = fixture.app();
        let record = app.open_editor("a.md").unwrap().record;
        let mut pending = request(&record, "unfinished copy");
        pending.destination = Some("folder/a.md".into());
        app.work_store_mut()
            .begin_editor_save(
                &pending,
                &Path::new("folder/a.md")
                    .with_file_name(format!(".brn-{}.stage", pending.operation_id)),
            )
            .unwrap();
        fs::rename(
            fixture.base.path().join("vault"),
            fixture.base.path().join("old-vault"),
        )
        .unwrap();
        fs::create_dir(fixture.base.path().join("vault")).unwrap();
        fs::write(fixture.note(), "new root").unwrap();
        let view = app.open_editor("a.md").unwrap();
        assert_eq!(view.record.text, "unfinished copy");
        assert!(view.conflict);
        assert!(view.saved.is_none());
        assert!(view.pending.contains(&pending.operation_id));
    }

    #[test]
    fn interrupted_copy_destination_alias_and_moved_identity_cannot_be_enrolled() {
        let fixture = Fixture::new();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "editor::tests::crash_child", "--ignored"])
            .env("BRN_EDITOR_TEST_ROOT", fixture.base.path())
            .env("BRN_EDITOR_TEST_PHASE", "exchange")
            .env("BRN_EDITOR_TEST_COPY", "yes")
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(71));
        let mut app = fixture.app();
        let operation = app.work_store().editor_saves().unwrap()[0]
            .request
            .operation_id;
        assert_eq!(
            app.open_editor("copy.md").unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        assert_eq!(
            app.open_editor("COPY.md").unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        fs::rename(
            fixture.base.path().join("vault/copy.md"),
            fixture.base.path().join("vault/moved.md"),
        )
        .unwrap();
        assert_eq!(
            app.open_editor("moved.md").unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        fs::rename(
            fixture.base.path().join("vault/moved.md"),
            fixture.base.path().join("vault/copy.md"),
        )
        .unwrap();
        assert_eq!(
            app.reconcile_editor(operation).unwrap().outcome,
            SaveOutcome::Applied
        );
        assert!(!app.open_editor("copy.md").unwrap().conflict);
    }

    #[test]
    fn retained_tool_search_cannot_return_across_uncertain_or_completed_save() {
        use crate::library::{Embedder, SharedEmbedder};
        use brn_ai::ReadTools;
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            mpsc,
        };
        struct BlockingModel {
            block: Arc<AtomicBool>,
            entered: mpsc::Sender<()>,
            release: mpsc::Receiver<()>,
        }
        impl Embedder for BlockingModel {
            fn identity(&self) -> &str {
                "synthetic-evidence-fence"
            }
            fn dimension(&self) -> usize {
                2
            }
            fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
                if self.block.load(Ordering::Acquire) {
                    self.entered.send(()).unwrap();
                    self.release
                        .recv_timeout(std::time::Duration::from_secs(10))
                        .unwrap();
                }
                Ok(texts.iter().map(|_| vec![1.0, 0.5]).collect())
            }
        }
        for uncertain in [false, true] {
            let fixture = Fixture::new();
            fs::write(fixture.base.path().join("vault/b.md"), "unrelated needle").unwrap();
            let block = Arc::new(AtomicBool::new(false));
            let (entered_tx, entered) = mpsc::channel();
            let (release, release_rx) = mpsc::channel();
            let model = SharedEmbedder::new(Box::new(BlockingModel {
                block: block.clone(),
                entered: entered_tx,
                release: release_rx,
            }));
            let mut app = App::open_with_model_loader(
                &fixture.base.path().join("data"),
                AppConfig {
                    vault_root: Some(fixture.base.path().join("vault")),
                    credentials_dir: Some(fixture.base.path().join("credentials")),
                    model_dir: None,
                },
                |_, _| Ok(Some(model)),
            )
            .unwrap();
            app.embed_pending(16).unwrap();
            let save = request(&app.open_editor("a.md").unwrap().record, "changed");
            let tools = app.tools().unwrap();
            block.store(true, Ordering::Release);
            let join = std::thread::spawn(move || tools.search_notes("needle", 10));
            entered
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            if uncertain {
                let stage = fixture
                    .base
                    .path()
                    .join("vault")
                    .join(format!(".brn-{}.stage", save.operation_id));
                CHECKPOINT_HOOK.with(|hook| {
                    *hook.borrow_mut() = Some(Box::new(move |phase| {
                        if phase == "exchange" {
                            fs::write(&stage, "unexpected").unwrap();
                        }
                    }))
                });
            }
            let saved = app.save_editor(&save);
            CHECKPOINT_HOOK.with(|hook| *hook.borrow_mut() = None);
            assert_eq!(saved.is_err(), uncertain);
            release.send(()).unwrap();
            assert_eq!(
                join.join().unwrap().unwrap_err().kind,
                brn_ai::AiErrorKind::IndexStale
            );
            block.store(false, Ordering::Release);
        }
    }

    #[test]
    #[ignore = "private subprocess crash entry point"]
    fn crash_child() {
        let base =
            PathBuf::from(std::env::var_os("BRN_EDITOR_TEST_ROOT").expect("private fixture"));
        let phase = std::env::var("BRN_EDITOR_TEST_PHASE").unwrap();
        let mut app = open(&base);
        let record = app.open_editor("a.md").unwrap().record;
        let mut save = request(&record, "saved õ\r\n");
        if std::env::var("BRN_EDITOR_TEST_COPY").unwrap() == "yes" {
            save.destination =
                Some(std::env::var("BRN_EDITOR_TEST_DEST").unwrap_or_else(|_| "copy.md".into()));
        }
        CRASH_PHASE.with(|selected| *selected.borrow_mut() = Some(phase));
        app.save_editor(&save).unwrap();
        panic!("crash checkpoint was not reached");
    }
}
