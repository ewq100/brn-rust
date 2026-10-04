//! Exact reviewed approval and whole-proposal filesystem recovery.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    editor::file_error,
    files::{MacFiles, recovery::ApplyRecoveryFiles},
};
pub use brn_store::work::proposal_apply::{
    ApplyJournal, ApplyMemberPhase, ApplyMemberProof, ApplyOutcome, ApplyReceipt, ApprovalRequest,
    RepairDirection, RepairPreview, RepairReceipt, RepairRequest, UndoBinding, UndoOriginal,
    UndoPreview, UndoRequest,
};
mod repair;
use brn_store::work::proposals::{NoteChange, ProposalDraft, ProposalState};
use brn_store::{
    WorkStore,
    files::{FileFingerprint, PreparedFile, VaultRecord},
};
pub use repair::validate_repair_request;
use serde::{Deserialize, Serialize};
use std::{cell::Cell, path::Path};
use uuid::Uuid;

fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}
fn unsettled(journal: &ApplyJournal) -> bool {
    journal
        .receipt
        .as_ref()
        .is_none_or(|r| r.outcome == ApplyOutcome::Uncertain)
}

/// Explicit captured membership: newly arrived proposals are never included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupApprovalRequest {
    pub group_id: Uuid,
    pub approvals: Vec<ApprovalRequest>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupApprovalResult {
    pub receipts: Vec<ApplyReceipt>,
    /// The member that stopped the captured group, without admitting later members.
    pub stopped: Option<GroupApprovalStop>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupApprovalStop {
    pub operation_id: Uuid,
    pub message: String,
}

/// Small checked operational summary. Full historical bodies are loaded only
/// by an explicit identified lookup, independently of paged activity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplySummary {
    pub request: ApprovalRequest,
    pub title: String,
    pub outcome: Option<ApplyOutcome>,
    pub started_at_ms: u64,
}

impl App {
    pub fn proposal_recovery_operations(&self) -> Result<Vec<ApplySummary>> {
        let mut operations = Vec::new();
        for id in self.store.proposal_apply_ids()? {
            // Validate one bounded full journal at a time. Never retain all
            // historical snapshots just to enumerate unresolved operations.
            let journal = self
                .store
                .proposal_apply(id)?
                .ok_or_else(|| stale("listed approval journal disappeared"))?;
            if unsettled(&journal) {
                operations.push(ApplySummary {
                    request: journal.request,
                    title: journal.approved.draft.title,
                    outcome: journal.receipt.map(|receipt| receipt.outcome),
                    started_at_ms: journal.started_at_ms,
                });
            }
        }
        Ok(operations)
    }
    pub fn proposal_apply(&self, id: Uuid) -> Result<Option<ApplyJournal>> {
        if id.is_nil() {
            return Err(WorkflowError::typed(
                ErrorKind::ToolRejected,
                "approval lookup requires a non-nil operation UUID",
            ));
        }
        Ok(self.store.proposal_apply(id)?)
    }
}

pub fn validate_approval_request(request: &ApprovalRequest) -> Result<()> {
    if request.operation_id.is_nil()
        || request.expected.id.is_nil()
        || request.expected.version == 0
        || request.expected.version.checked_add(3).is_none()
    {
        return Err(WorkflowError::typed(
            ErrorKind::ToolRejected,
            "approval requires nonzero operation/proposal UUIDs and a bounded review version",
        ));
    }
    Ok(())
}
pub fn validate_undo_request(request: &UndoRequest) -> Result<()> {
    if request.operation_id.is_nil()
        || request.target_operation_id.is_nil()
        || request.operation_id == request.target_operation_id
        || request
            .trash_member
            .is_some_and(|index| index >= crate::proposals::MAX_PROPOSAL_CHANGES)
    {
        return Err(WorkflowError::typed(
            ErrorKind::ToolRejected,
            "Undo requires distinct non-nil operation UUIDs and a bounded Trash member",
        ));
    }
    Ok(())
}

impl GroupApprovalRequest {
    pub fn validate(&self) -> Result<()> {
        if self.group_id.is_nil()
            || self.approvals.is_empty()
            || self.approvals.len() > crate::proposals::MAX_PROPOSAL_CHANGES
        {
            return Err(WorkflowError::typed(
                ErrorKind::ToolRejected,
                "invalid captured proposal group",
            ));
        }
        let mut ids = std::collections::HashSet::new();
        let mut operations = std::collections::HashSet::new();
        for request in &self.approvals {
            validate_approval_request(request)?;
            if !ids.insert(request.expected.id) || !operations.insert(request.operation_id) {
                return Err(WorkflowError::typed(
                    ErrorKind::ToolRejected,
                    "captured proposal group has duplicate members or operations",
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn restore_application_records(
    store: &mut WorkStore,
    requested: Option<&Path>,
) -> Result<Option<ApplyRecoveryFiles>> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (store, requested);
        Ok(None)
    }
    #[cfg(target_os = "macos")]
    {
        let records = ApplyRecoveryFiles::open(store.data_dir()).map_err(file_error)?;
        // Keep only ordering/binding metadata between reads; one bounded full
        // journal is decoded at a time, even with a long receipt history.
        let mut order = Vec::new();
        let mut vault: Option<VaultRecord> = None;
        for id in records.ids().map_err(file_error)? {
            let snapshot = records
                .read(id)
                .map_err(file_error)?
                .ok_or_else(|| stale("approval recovery record disappeared"))?;
            let bound = &snapshot.journal.approved.draft.vault;
            if vault.as_ref().is_some_and(|old| old != bound) {
                return Err(stale("approval recovery records bind different vaults"));
            }
            vault = Some(bound.clone());
            order.push((
                unsettled(&snapshot.journal),
                std::cmp::Reverse(snapshot.journal.approved.version),
                snapshot.journal.started_at_ms,
                id,
            ));
        }
        if let Some(vault) = vault {
            let requested = requested.map(|p| p.canonicalize().unwrap_or_else(|_| p.to_owned()));
            let stored = store.setting("vault.root")?.map(std::path::PathBuf::from);
            if requested.as_ref().is_some_and(|p| *p != vault.root)
                || stored.as_ref().is_some_and(|p| *p != vault.root)
            {
                return Err(stale("approval recovery is bound to another vault"));
            }
            if let Some(encoded) = store.setting("vault.editor_identity")? {
                let original: VaultRecord = serde_json::from_str(&encoded)
                    .map_err(|_| stale("invalid saved vault identity"))?;
                if original != vault {
                    return Err(stale("approval recovery vault identity differs"));
                }
            }
            // An older/fresh operational database may predate the binding. The
            // ordinary receipt retains the same trusted root, never a new one.
            store.set_setting(
                "vault.root",
                vault
                    .root
                    .to_str()
                    .ok_or_else(|| stale("invalid vault path"))?,
            )?;
            store.set_setting(
                "vault.editor_identity",
                &serde_json::to_string(&vault)
                    .map_err(|_| stale("could not encode vault binding"))?,
            )?;
        }
        order.sort();
        // Import the newest terminal evidence first. Earlier refused snapshots
        // may have had annotations removed by that success; restoring them
        // first would conflict with an older database's commented Draft.
        for (_, _, _, id) in order {
            let snapshot = records
                .read(id)
                .map_err(file_error)?
                .ok_or_else(|| stale("approval recovery record disappeared"))?;
            if snapshot
                .journal
                .receipt
                .as_ref()
                .is_some_and(|r| r.outcome == ApplyOutcome::Applied)
            {
                clean_snapshot_comments(&records, &snapshot.journal)?;
            }
            records
                .write(&snapshot.journal, Some(&snapshot))
                .map_err(file_error)?;
            let effective = store.restore_proposal_apply(&snapshot.journal)?;
            if effective != snapshot.journal {
                if effective
                    .receipt
                    .as_ref()
                    .is_some_and(|r| r.outcome == ApplyOutcome::Applied)
                {
                    clean_snapshot_comments(&records, &effective)?;
                }
                records
                    .write(&effective, Some(&snapshot))
                    .map_err(file_error)?;
            }
        }
        Ok(Some(records))
    }
}

fn clean_snapshot_comments(records: &ApplyRecoveryFiles, approved: &ApplyJournal) -> Result<()> {
    for id in records.ids().map_err(file_error)? {
        if id == approved.request.operation_id {
            continue;
        }
        let Some(snapshot) = records.read(id).map_err(file_error)? else {
            return Err(stale("approval record disappeared during cleanup"));
        };
        if snapshot.journal.approved.draft.id == approved.approved.draft.id
            && snapshot.journal.approved.version <= approved.approved.version
            && !snapshot.journal.approved.comments.is_empty()
        {
            let mut cleaned = snapshot.journal.clone();
            cleaned.approved.comments.clear();
            records
                .write(&cleaned, Some(&snapshot))
                .map_err(file_error)?;
        }
    }
    records
        .retire_review_temporaries(approved)
        .map_err(file_error)?;
    Ok(())
}

fn original(change: &NoteChange) -> Option<&FileFingerprint> {
    match change {
        NoteChange::Create { .. } => None,
        NoteChange::Replace { before, .. } | NoteChange::Trash { before, .. } => Some(before),
    }
}
fn parent(change: &NoteChange) -> &brn_store::files::VaultIdentity {
    match change {
        NoteChange::Create { parent, .. }
        | NoteChange::Replace { parent, .. }
        | NoteChange::Trash { parent, .. } => parent,
    }
}
fn observed(files: &MacFiles, path: &Path) -> Result<Option<FileFingerprint>> {
    match files.artifact(path).map_err(file_error)? {
        None => Ok(None),
        Some(_) => Ok(Some(
            files
                .observe_uncoordinated(path)
                .map_err(file_error)?
                .fingerprint,
        )),
    }
}
fn check_targets(files: &MacFiles, draft: &ProposalDraft) -> Result<()> {
    for change in &draft.changes {
        let path = Path::new(change.path());
        files
            .coordinate(path, || {
                Ok((|| -> Result<()> {
                    if files.parent_identity(path).map_err(file_error)? != *parent(change) {
                        return Err(stale("reviewed proposal parent changed"));
                    }
                    if observed(files, path)?.as_ref() != original(change) {
                        return Err(stale("reviewed proposal destination changed"));
                    }
                    Ok(())
                })())
            })
            .map_err(file_error)??;
    }
    Ok(())
}
fn check_sources(files: &MacFiles, journal: &ApplyJournal, installed: usize) -> Result<()> {
    let phases: Vec<_> = (0..journal.members.len())
        .map(|index| {
            if index < installed {
                ApplyMemberPhase::Applied
            } else {
                ApplyMemberPhase::Before
            }
        })
        .collect();
    repair::check_phase_sources(files, journal, &phases)
}

impl App {
    fn application_records(&mut self) -> Result<&ApplyRecoveryFiles> {
        if self.apply_records.is_none() {
            self.apply_records =
                Some(ApplyRecoveryFiles::open(self.store.data_dir()).map_err(file_error)?);
        }
        Ok(self.apply_records.as_ref().expect("opened recovery files"))
    }

    pub(crate) fn reconcile_startup_proposals(&mut self) -> Result<()> {
        let Some(records) = &self.apply_records else {
            return Ok(());
        };
        let mut pending = Vec::new();
        for id in records.ids().map_err(file_error)? {
            if self
                .store
                .proposal_apply(id)?
                .as_ref()
                .is_some_and(unsettled)
            {
                pending.push(id);
            }
        }
        for id in pending {
            // Missing/replaced/unavailable vault proof remains fenced. Review
            // work is still readable; no namespace write is attempted here.
            let _ = self.reconcile_proposal(id);
        }
        Ok(())
    }

    pub fn approve_proposal(&mut self, request: &ApprovalRequest) -> Result<ApplyReceipt> {
        validate_approval_request(request)?;
        if let Some(journal) = self.store.proposal_apply(request.operation_id)? {
            if journal.request != *request {
                return Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "approval UUID has another review stamp",
                ));
            }
            return journal.receipt.ok_or_else(|| {
                WorkflowError::typed(
                    ErrorKind::SaveUncertain,
                    "approval is interrupted; reconcile it without repeating writes",
                )
            });
        }
        self.require_current_evidence()?;
        let review = self.proposal(request.expected.id)?;
        if review.stamp() != request.expected || review.state != ProposalState::Draft {
            return Err(stale("approval requires the exact current Draft"));
        }
        self.preflight_proposal(&review.draft, None)?;
        self.application_records()?;
        self.set_current_tool_barrier(true);
        let journal = match self.store.begin_proposal_apply(request) {
            Ok(journal) => journal,
            Err(error) => {
                self.synchronize_current_barrier()?;
                return Err(error.into());
            }
        };
        self.run_admitted_proposal(journal)
    }

    /// History-only exact inverse preview; it does not claim current eligibility.
    pub fn preview_proposal_undo(&self, request: &UndoRequest) -> Result<UndoPreview> {
        validate_undo_request(request)?;
        Ok(self.store.preview_proposal_undo(request)?)
    }

    /// Explicit human Undo/Trash restore. AI has no direct durable-change tool.
    pub fn undo_proposal(&mut self, request: &UndoRequest) -> Result<ApplyReceipt> {
        validate_undo_request(request)?;
        if let Some(journal) = self.store.proposal_apply(request.operation_id)? {
            if !journal.undo.as_ref().is_some_and(|binding| {
                binding.operation_id == request.target_operation_id
                    && binding.trash_member == request.trash_member
            }) {
                return Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "Undo UUID has another source or scope",
                ));
            }
            return journal.receipt.ok_or_else(|| {
                WorkflowError::typed(
                    ErrorKind::SaveUncertain,
                    "Undo is interrupted; reconcile it without repeating writes",
                )
            });
        }
        self.require_current_evidence()?;
        let preview = self.preview_proposal_undo(request)?;
        self.preflight_proposal(&preview.draft, Some(&preview.binding))?;
        self.application_records()?;
        self.set_current_tool_barrier(true);
        let journal = match self.store.begin_proposal_undo(request) {
            Ok(journal) => journal,
            Err(error) => {
                self.synchronize_current_barrier()?;
                return Err(error.into());
            }
        };
        self.run_admitted_proposal(journal)
    }

    fn preflight_proposal(
        &mut self,
        draft: &ProposalDraft,
        undo: Option<&UndoBinding>,
    ) -> Result<()> {
        self.editor_files()?;
        let bound: VaultRecord = serde_json::from_str(
            &self
                .store
                .setting("vault.editor_identity")?
                .ok_or_else(|| stale("vault identity is missing"))?,
        )
        .map_err(|_| stale("vault identity is invalid"))?;
        if bound != draft.vault {
            return Err(stale("reviewed vault identity changed"));
        }
        let editors = self.store.editors()?;
        for (index, change) in draft.changes.iter().enumerate() {
            let borrowed = undo.and_then(|binding| binding.originals[index].as_ref());
            for editor in &editors {
                let matches = original(change).is_some_and(|before| {
                    before.device == editor.baseline.device && before.inode == editor.baseline.inode
                }) || borrowed.is_some_and(|original| {
                    original.fingerprint.device == editor.baseline.device
                        && original.fingerprint.inode == editor.baseline.inode
                }) || self
                    .editor
                    .files
                    .as_ref()
                    .expect("opened files")
                    .reserved_copy_path_matches(Path::new(change.path()), Path::new(&editor.path))
                    .map_err(file_error)?;
                let restored_baseline = borrowed.is_some_and(|original| {
                    original.fingerprint == editor.baseline
                        && change.text() == Some(editor.baseline_text.as_str())
                });
                let baseline_matches = match original(change) {
                    Some(before) => *before == editor.baseline || restored_baseline,
                    None if undo.is_some() => restored_baseline,
                    None => true,
                };
                if matches && (editor.text != editor.baseline_text || !baseline_matches) {
                    return Err(stale(
                        "proposal conflicts with retained editor work; save or explicitly reload it first",
                    ));
                }
            }
        }
        check_targets(self.editor.files.as_ref().expect("opened files"), draft)?;
        for source in &draft.sources {
            if self
                .editor
                .files
                .as_ref()
                .expect("opened files")
                .observe(Path::new(&source.path))
                .map_err(file_error)?
                .fingerprint
                != source.fingerprint
            {
                return Err(stale("reviewed proposal source changed"));
            }
        }
        if let Some(binding) = undo {
            let files = self.editor.files.as_ref().expect("opened files");
            for (change, original) in draft.changes.iter().zip(&binding.originals) {
                if let Some(original) = original {
                    let destination = Path::new(change.path());
                    let staging =
                        destination.with_file_name(format!(".brn-{}.stage", original.member_id));
                    files
                        .coordinate(destination, || {
                            Ok((|| -> Result<()> {
                                if files.parent_identity(destination).map_err(file_error)?
                                    != *parent(change)
                                    || observed(files, &staging)?.as_ref()
                                        != Some(&original.fingerprint)
                                {
                                    return Err(stale("retained Undo original changed"));
                                }
                                Ok(())
                            })())
                        })
                        .map_err(file_error)??;
                }
            }
        }
        Ok(())
    }

    fn run_admitted_proposal(&mut self, mut journal: ApplyJournal) -> Result<ApplyReceipt> {
        checkpoint("intent", 0);
        let attempted = Cell::new(false);
        let result = self.execute_proposal(&mut journal, &attempted);
        match result {
            Ok(proofs) => {
                self.complete_proposal(&journal, ApplyOutcome::Applied, Some(proofs), false)
            }
            Err(error) => {
                let observations = self.observe_proposal(&journal).ok();
                let outcome = if attempted.get() {
                    ApplyOutcome::Uncertain
                } else {
                    ApplyOutcome::NotApplied
                };
                // A persistence failure must leave the intent fenced. Never
                // expose a Draft while its recovery record says unknown.
                let _receipt =
                    self.complete_proposal(&journal, outcome, observations, !attempted.get())?;
                Err(error)
            }
        }
    }

    fn execute_proposal(
        &mut self,
        journal: &mut ApplyJournal,
        attempted: &Cell<bool>,
    ) -> Result<Vec<ApplyMemberProof>> {
        let records = self.apply_records.as_ref().expect("opened recovery files");
        let snapshot = records.write(journal, None).map_err(file_error)?;
        checkpoint("mirror-intent", 0);
        let files = self.editor.files.as_ref().expect("opened files");
        check_targets(files, &journal.approved.draft)?;
        check_sources(files, journal, 0)?;
        let mut prepared = Vec::new();
        for (i, (change, member)) in journal
            .approved
            .draft
            .changes
            .iter()
            .zip(&journal.members)
            .enumerate()
        {
            let destination = Path::new(change.path());
            let proof = files
                .coordinate(destination, || {
                    Ok((|| -> Result<FileFingerprint> {
                        if files.parent_identity(destination).map_err(file_error)?
                            != *parent(change)
                            || observed(files, destination)?.as_ref() != original(change)
                        {
                            return Err(stale("proposal member changed before preparation"));
                        }
                        if let Some(original) = journal
                            .undo
                            .as_ref()
                            .and_then(|binding| binding.originals[i].as_ref())
                        {
                            if observed(files, &member.staging)?.as_ref()
                                != Some(&original.fingerprint)
                            {
                                return Err(stale(
                                    "retained Undo original changed before preparation",
                                ));
                            }
                            files.flush_artifact(&member.staging).map_err(file_error)?;
                            if observed(files, &member.staging)?.as_ref()
                                != Some(&original.fingerprint)
                            {
                                return Err(stale(
                                    "retained Undo original changed during preparation",
                                ));
                            }
                            return Ok(original.fingerprint.clone());
                        }
                        if files
                            .artifact(&member.staging)
                            .map_err(file_error)?
                            .is_some()
                        {
                            return Err(stale("proposal staging path is occupied"));
                        }
                        Ok(match change {
                            NoteChange::Create { text, .. } => {
                                files
                                    .prepare_copy(
                                        member.id,
                                        &member.staging,
                                        destination,
                                        text.as_bytes(),
                                    )
                                    .map_err(file_error)?
                                    .fingerprint
                            }
                            NoteChange::Replace { text, .. } => {
                                files
                                    .prepare_replace(
                                        member.id,
                                        &member.staging,
                                        destination,
                                        text.as_bytes(),
                                    )
                                    .map_err(file_error)?
                                    .fingerprint
                            }
                            NoteChange::Trash { before, .. } => before.clone(),
                        })
                    })())
                })
                .map_err(file_error)??;
            prepared.push(proof);
            checkpoint("stage", i);
        }
        *journal = self
            .store
            .record_proposal_prepared(journal.request.operation_id, &prepared)?;
        checkpoint("prepared-db", 0);
        records
            .write(journal, Some(&snapshot))
            .map_err(file_error)?;
        checkpoint("prepared", 0);
        // All preparation is durable before the first namespace effect.
        check_targets(files, &journal.approved.draft)?;
        for (i, ((change, member), proof)) in journal
            .approved
            .draft
            .changes
            .iter()
            .zip(&journal.members)
            .zip(&prepared)
            .enumerate()
        {
            check_sources(files, journal, i)?;
            let destination = Path::new(change.path());
            files
                .coordinate(destination, || {
                    Ok((|| -> Result<()> {
                        if files.parent_identity(destination).map_err(file_error)?
                            != *parent(change)
                            || observed(files, destination)?.as_ref() != original(change)
                        {
                            return Err(stale("proposal destination changed before installation"));
                        }
                        attempted.set(true);
                        let staged = PreparedFile {
                            relative: member.staging.clone(),
                            fingerprint: proof.clone(),
                        };
                        match change {
                            NoteChange::Create { .. } => {
                                files.install_exclusive(&staged, destination)
                            }
                            NoteChange::Replace { .. } => files.exchange(&staged, destination),
                            NoteChange::Trash { before, .. } => files.install_exclusive(
                                &PreparedFile {
                                    relative: destination.to_owned(),
                                    fingerprint: before.clone(),
                                },
                                &member.staging,
                            ),
                        }
                        .map_err(file_error)?;
                        checkpoint("member", i);
                        match change {
                            NoteChange::Create { .. } => files.flush_artifact(destination),
                            NoteChange::Replace { .. } => {
                                files.flush_artifact(destination).map_err(file_error)?;
                                files.flush_artifact(&member.staging)
                            }
                            NoteChange::Trash { .. } => files.flush_artifact(&member.staging),
                        }
                        .map_err(file_error)?;
                        checkpoint("synced", i);
                        Ok(())
                    })())
                })
                .map_err(file_error)??;
        }
        check_sources(files, journal, journal.members.len())?;
        let observations = self.observe_proposal(journal)?;
        check_sources(files, journal, journal.members.len())?;
        let candidate = completion(
            journal,
            ApplyOutcome::Applied,
            Some(observations.clone()),
            false,
        )?;
        candidate.validate()?;
        checkpoint("verified", 0);
        Ok(observations)
    }

    fn observe_proposal(&self, journal: &ApplyJournal) -> Result<Vec<ApplyMemberProof>> {
        let files = self
            .editor
            .files
            .as_ref()
            .ok_or_else(|| stale("vault files unavailable"))?;
        journal
            .approved
            .draft
            .changes
            .iter()
            .zip(&journal.members)
            .map(|(change, member)| {
                let destination = Path::new(change.path());
                files
                    .coordinate(destination, || {
                        Ok((|| -> Result<ApplyMemberProof> {
                            if files.parent_identity(destination).map_err(file_error)?
                                != *parent(change)
                            {
                                return Err(stale("proposal member parent changed"));
                            }
                            Ok(ApplyMemberProof {
                                destination: observed(files, destination)?,
                                staging: observed(files, &member.staging)?,
                            })
                        })())
                    })
                    .map_err(file_error)?
            })
            .collect()
    }

    fn complete_proposal(
        &mut self,
        journal: &ApplyJournal,
        outcome: ApplyOutcome,
        observations: Option<Vec<ApplyMemberProof>>,
        no_effects: bool,
    ) -> Result<ApplyReceipt> {
        let candidate = completion(journal, outcome, observations, no_effects)?;
        let records = self.application_records()?;
        let previous = records
            .read(journal.request.operation_id)
            .map_err(file_error)?;
        records
            .write(&candidate, previous.as_ref())
            .map_err(file_error)?;
        checkpoint("completion", 0);
        if outcome == ApplyOutcome::Applied {
            clean_snapshot_comments(records, &candidate)?;
        }
        let receipt = if no_effects {
            self.store.refuse_proposal_before_effects(
                journal.request.operation_id,
                candidate.observations.as_deref(),
            )?
        } else {
            self.store.finish_proposal_apply(
                journal.request.operation_id,
                outcome,
                candidate.observations.as_deref(),
            )?
        };
        self.synchronize_current_barrier()?;
        checkpoint("receipt", 0);
        let _ = self.refresh();
        Ok(receipt)
    }

    /// Classifies complete proofs; never repeats installation or guesses a path.
    pub fn reconcile_proposal(&mut self, id: Uuid) -> Result<ApplyReceipt> {
        let mut journal = self.store.proposal_apply(id)?.ok_or_else(|| {
            WorkflowError::typed(ErrorKind::NotFound, "approval operation does not exist")
        })?;
        self.application_records()?;
        if let Some(snapshot) = self
            .apply_records
            .as_ref()
            .expect("opened records")
            .read(id)
            .map_err(file_error)?
            && snapshot
                .journal
                .receipt
                .as_ref()
                .is_some_and(|r| r.outcome != ApplyOutcome::Uncertain)
        {
            self.apply_records
                .as_ref()
                .expect("opened records")
                .write(&snapshot.journal, Some(&snapshot))
                .map_err(file_error)?;
            if snapshot
                .journal
                .receipt
                .as_ref()
                .is_some_and(|r| r.outcome == ApplyOutcome::Applied)
            {
                clean_snapshot_comments(
                    self.apply_records.as_ref().expect("opened records"),
                    &snapshot.journal,
                )?;
            }
            journal = self.store.restore_proposal_apply(&snapshot.journal)?;
        }
        if !unsettled(&journal) {
            self.synchronize_current_barrier()?;
            return Ok(journal.receipt.expect("settled receipt"));
        }
        self.set_current_tool_barrier(true);
        self.editor_files()?;
        let observations = self.observe_proposal(&journal).ok();
        let mut outcome = ApplyOutcome::Uncertain;
        if let Some(proofs) = &observations {
            if completion(&journal, ApplyOutcome::Applied, Some(proofs.clone()), false).is_ok() {
                outcome = ApplyOutcome::Applied;
            } else if completion(
                &journal,
                ApplyOutcome::NotApplied,
                Some(proofs.clone()),
                false,
            )
            .is_ok()
            {
                outcome = ApplyOutcome::NotApplied;
            }
        }
        if outcome == ApplyOutcome::Applied {
            let files = self.editor.files.as_ref().expect("opened files");
            if check_sources(files, &journal, journal.members.len()).is_err() {
                outcome = ApplyOutcome::Uncertain;
            }
        }
        if outcome == ApplyOutcome::Applied
            || outcome == ApplyOutcome::NotApplied && journal.repair.is_some()
        {
            let files = self.editor.files.as_ref().expect("opened files");
            for (change, member) in journal.approved.draft.changes.iter().zip(&journal.members) {
                for path in [Path::new(change.path()), member.staging.as_path()] {
                    if observed(files, path)?.is_some() {
                        files.flush_artifact(path).map_err(file_error)?;
                    }
                }
            }
            if self.observe_proposal(&journal).ok() != observations
                || outcome == ApplyOutcome::Applied
                    && check_sources(files, &journal, journal.members.len()).is_err()
            {
                outcome = ApplyOutcome::Uncertain;
            }
        }
        // Repeated uncertainty is immutable; a later definitive inspection can
        // settle it, but cannot rewrite its first observations.
        if outcome == ApplyOutcome::Uncertain
            && let Some(receipt) = journal.receipt.clone()
        {
            if journal.repair.as_ref().is_some_and(|binding| {
                binding
                    .attempts
                    .last()
                    .is_some_and(|attempt| attempt.outcome.is_none())
            }) {
                self.interrupt_repair(&journal)?;
            }
            return Ok(receipt);
        }
        self.complete_proposal(&journal, outcome, observations, false)
    }

    pub fn approve_proposal_group(
        &mut self,
        request: &GroupApprovalRequest,
    ) -> Result<GroupApprovalResult> {
        request.validate()?;
        for approval in &request.approvals {
            if self.proposal(approval.expected.id)?.draft.group_id != Some(request.group_id) {
                return Err(stale(
                    "approval group contains duplicate or unbound members",
                ));
            }
        }
        let mut result = GroupApprovalResult {
            receipts: Vec::new(),
            stopped: None,
        };
        for approval in &request.approvals {
            match self.approve_proposal(approval) {
                Ok(receipt) => {
                    let applied = receipt.outcome == ApplyOutcome::Applied;
                    result.receipts.push(receipt);
                    if !applied {
                        result.stopped = Some(GroupApprovalStop { operation_id: approval.operation_id, message: "recorded approval did not apply; inspect its receipt before continuing".into() });
                        break;
                    }
                }
                Err(error) => {
                    if let Some(receipt) = self
                        .store
                        .proposal_apply(approval.operation_id)?
                        .and_then(|journal| journal.receipt)
                    {
                        result.receipts.push(receipt);
                    }
                    result.stopped = Some(GroupApprovalStop {
                        operation_id: approval.operation_id,
                        message: error.message,
                    });
                    break;
                }
            }
        }
        Ok(result)
    }
}

fn completion(
    journal: &ApplyJournal,
    outcome: ApplyOutcome,
    observations: Option<Vec<ApplyMemberProof>>,
    no_effects: bool,
) -> Result<ApplyJournal> {
    let mut candidate = journal.clone();
    let steps = if journal.receipt.is_some() { 3 } else { 2 };
    let version = journal
        .approved
        .version
        .checked_add(steps)
        .ok_or_else(|| stale("approval version overflow"))?;
    candidate.receipt = Some(ApplyReceipt {
        operation_id: journal.request.operation_id,
        proposal_id: journal.approved.draft.id,
        approved_version: journal.approved.version,
        stamp: crate::proposals::ProposalStamp {
            id: journal.approved.draft.id,
            version,
        },
        outcome,
    });
    candidate.observations = observations;
    candidate.no_effects = no_effects;
    if let Some(binding) = &mut candidate.repair {
        binding
            .attempts
            .last_mut()
            .expect("validated repair history")
            .outcome = Some(outcome);
    }
    if outcome == ApplyOutcome::Applied {
        candidate.approved.comments.clear();
    }
    candidate.validate()?;
    Ok(candidate)
}

#[cfg(test)]
type CheckpointHook = Box<dyn Fn(&str, usize)>;
#[cfg(test)]
thread_local! {
    static APPLY_CHECKPOINT: std::cell::RefCell<Option<(String, usize)>> = const { std::cell::RefCell::new(None) };
    static APPLY_HOOK: std::cell::RefCell<Option<CheckpointHook>> = const { std::cell::RefCell::new(None) };
}
fn checkpoint(step: &str, member: usize) {
    #[cfg(test)]
    APPLY_HOOK.with(|hook| {
        if let Some(hook) = hook.borrow().as_ref() {
            hook(step, member);
        }
    });
    #[cfg(test)]
    if APPLY_CHECKPOINT.with(|stop| {
        stop.borrow()
            .as_ref()
            .is_some_and(|(selected, index)| selected == step && *index == member)
    }) {
        std::process::exit(86);
    }
    let _ = (step, member);
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::{app::AppConfig, editor::EditRequest, proposals::*};
    use brn_ai::{AiErrorKind, ReadTools};
    use std::{fs, path::PathBuf, process::Command};

    pub(super) struct Fixture {
        _dir: tempfile::TempDir,
        pub(super) base: PathBuf,
    }
    impl Fixture {
        pub(super) fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let base = dir.path().to_owned();
            fs::create_dir(base.join("data")).unwrap();
            fs::create_dir(base.join("vault")).unwrap();
            fs::write(base.join("vault/a.md"), "Original λ\r\n").unwrap();
            fs::write(base.join("vault/trash.md"), "Original trash 🦀\r\n").unwrap();
            fs::write(base.join("vault/source.md"), "Bound source evidence").unwrap();
            Self { _dir: dir, base }
        }
        pub(super) fn app(&self) -> App {
            open(&self.base)
        }
        pub(super) fn prepare(&self) -> ApprovalRequest {
            let mut app = self.app();
            let before = app.open_editor("a.md").unwrap().record.baseline;
            let trash = app.open_editor("trash.md").unwrap().record.baseline;
            let source = app.open_editor("source.md").unwrap().record.baseline;
            let draft = app
                .create_proposal(&DraftRequest {
                    id: Uuid::new_v4(),
                    group_id: None,
                    session_id: None,
                    title: "Three-member approval".into(),
                    changes: vec![
                        DraftNoteChange::Replace {
                            path: "a.md".into(),
                            expected: before.clone(),
                            text: "\u{feff}Approved λ\r\n".into(),
                        },
                        DraftNoteChange::Create {
                            path: "new.md".into(),
                            text: "Approved new 🦀\r\n".into(),
                        },
                        DraftNoteChange::Trash {
                            path: "trash.md".into(),
                            expected: trash.clone(),
                        },
                    ],
                    sources: vec![
                        SourceVersion {
                            path: "A.md".into(),
                            fingerprint: before,
                        },
                        SourceVersion {
                            path: "trash.md".into(),
                            fingerprint: trash,
                        },
                        SourceVersion {
                            path: "source.md".into(),
                            fingerprint: source,
                        },
                    ],
                })
                .unwrap();
            let commented = app
                .add_proposal_comment(&CommentRequest {
                    expected: draft.stamp(),
                    comment: ReviewComment {
                        id: Uuid::new_v4(),
                        text: "Temporary review".into(),
                        target: CommentTarget::Proposal,
                    },
                })
                .unwrap();
            let request = ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: commented.stamp(),
            };
            fs::write(
                self.base.join("request.json"),
                serde_json::to_vec(&request).unwrap(),
            )
            .unwrap();
            request
        }
        fn prepare_undo(&self) -> UndoRequest {
            let approval = self.prepare();
            let mut app = self.app();
            app.approve_proposal(&approval).unwrap();
            let request = UndoRequest {
                operation_id: Uuid::new_v4(),
                target_operation_id: approval.operation_id,
                trash_member: None,
            };
            fs::write(
                self.base.join("undo-request.json"),
                serde_json::to_vec(&request).unwrap(),
            )
            .unwrap();
            request
        }
        fn crash_undo(&self, phase: &str, member: usize) {
            self.crash_mode(phase, member, true);
        }
        pub(super) fn crash(&self, phase: &str, member: usize) {
            self.crash_mode(phase, member, false);
        }
        fn crash_mode(&self, phase: &str, member: usize, undo: bool) {
            let result = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "proposal_apply::tests::crash_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("BRN_APPLY_TEST_BASE", &self.base)
                .env("BRN_APPLY_TEST_UNDO", if undo { "1" } else { "0" })
                .env("BRN_APPLY_TEST_PHASE", phase)
                .env("BRN_APPLY_TEST_MEMBER", member.to_string())
                .output()
                .unwrap();
            assert_eq!(
                result.status.code(),
                Some(86),
                "phase {phase}/{member}: {} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
    fn open(base: &Path) -> App {
        App::open(
            &base.join("data"),
            AppConfig {
                vault_root: Some(base.join("vault")),
                credentials_dir: None,
                model_dir: None,
            },
        )
        .unwrap()
    }
    #[test]
    #[ignore = "private subprocess entry exercised by crash recovery matrix"]
    fn crash_child() {
        let base = PathBuf::from(std::env::var_os("BRN_APPLY_TEST_BASE").unwrap());
        let phase = std::env::var("BRN_APPLY_TEST_PHASE").unwrap();
        let member = std::env::var("BRN_APPLY_TEST_MEMBER")
            .unwrap()
            .parse()
            .unwrap();
        let mut app = open(&base);
        APPLY_CHECKPOINT.with(|stop| *stop.borrow_mut() = Some((phase, member)));
        if std::env::var("BRN_APPLY_TEST_REPAIR").as_deref() == Ok("1") {
            let request =
                serde_json::from_slice(&fs::read(base.join("repair-request.json")).unwrap())
                    .unwrap();
            app.repair_proposal(&request).unwrap();
        } else if std::env::var("BRN_APPLY_TEST_UNDO").as_deref() == Ok("1") {
            let request =
                serde_json::from_slice(&fs::read(base.join("undo-request.json")).unwrap()).unwrap();
            app.undo_proposal(&request).unwrap();
        } else {
            let request =
                serde_json::from_slice(&fs::read(base.join("request.json")).unwrap()).unwrap();
            app.approve_proposal(&request).unwrap();
        }
        panic!("selected crash checkpoint not reached");
    }

    #[test]
    fn undo_crashes_at_each_boundary_settle_only_complete_proofs_without_retry() {
        for (phase, index, expected) in [
            ("intent", 0, ApplyOutcome::NotApplied),
            ("mirror-intent", 0, ApplyOutcome::NotApplied),
            ("stage", 0, ApplyOutcome::NotApplied),
            ("stage", 1, ApplyOutcome::NotApplied),
            ("stage", 2, ApplyOutcome::NotApplied),
            ("prepared-db", 0, ApplyOutcome::NotApplied),
            ("prepared", 0, ApplyOutcome::NotApplied),
            ("member", 0, ApplyOutcome::Uncertain),
            ("member", 1, ApplyOutcome::Uncertain),
            ("member", 2, ApplyOutcome::Applied),
            ("synced", 2, ApplyOutcome::Applied),
            ("verified", 0, ApplyOutcome::Applied),
            ("completion", 0, ApplyOutcome::Applied),
            ("receipt", 0, ApplyOutcome::Applied),
        ] {
            let f = Fixture::new();
            let request = f.prepare_undo();
            f.crash_undo(phase, index);
            let mut app = f.app();
            let result = app.reconcile_proposal(request.operation_id).unwrap();
            assert_eq!(result.outcome, expected, "{phase}/{index}");
            assert_eq!(app.undo_proposal(&request).unwrap(), result);
            let journal = app
                .work_store()
                .proposal_apply(request.operation_id)
                .unwrap()
                .unwrap();
            assert!(!journal.no_effects, "restart cannot certify no effect");
            assert!(journal.undo.is_some());
            if expected == ApplyOutcome::Uncertain {
                assert_eq!(app.note("a.md").unwrap_err().kind, ErrorKind::SaveUncertain);
                assert_eq!(
                    fs::read_to_string(f.base.join("vault/a.md")).unwrap(),
                    "Original λ\r\n"
                );
                assert!(!f.base.join("vault/trash.md").exists());
            } else {
                assert!(app.note("a.md").is_ok());
            }
        }
    }

    #[test]
    fn undo_mirror_failures_and_changed_borrowed_original_keep_exact_outcomes() {
        for (phase, expected) in [
            ("prepared-db", ApplyOutcome::NotApplied),
            ("verified", ApplyOutcome::Applied),
        ] {
            let f = Fixture::new();
            let request = f.prepare_undo();
            let mut app = f.app();
            let lease = app.tools().unwrap();
            APPLY_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |step, _| {
                    if step == "intent" {
                        assert_eq!(
                            lease.read_note("a.md").unwrap_err().kind,
                            AiErrorKind::IndexStale
                        );
                    }
                    if step == phase {
                        crate::files::recovery::FAILURE
                            .with(|failure| failure.set(Some("file_sync")));
                    }
                }))
            });
            let result = app.undo_proposal(&request);
            APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
            crate::files::recovery::FAILURE.with(|failure| failure.set(None));
            assert!(result.is_err(), "{phase}");
            assert_eq!(app.note("a.md").unwrap_err().kind, ErrorKind::SaveUncertain);
            drop(app);
            let mut app = f.app();
            assert_eq!(
                app.reconcile_proposal(request.operation_id)
                    .unwrap()
                    .outcome,
                expected
            );
        }
        let f = Fixture::new();
        let request = f.prepare_undo();
        let mut app = f.app();
        let source = app
            .work_store()
            .proposal_apply(request.target_operation_id)
            .unwrap()
            .unwrap();
        let retained = f.base.join("vault").join(&source.members[2].staging);
        APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |step, index| {
                if step == "synced" && index == 0 {
                    fs::write(&retained, "Changed retained original").unwrap();
                }
            }))
        });
        let result = app.undo_proposal(&request);
        APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert!(result.is_err());
        assert_eq!(
            app.work_store()
                .proposal_apply(request.operation_id)
                .unwrap()
                .unwrap()
                .receipt
                .unwrap()
                .outcome,
            ApplyOutcome::Uncertain
        );
        assert_eq!(
            app.reconcile_proposal(request.operation_id)
                .unwrap()
                .outcome,
            ApplyOutcome::Uncertain
        );
        assert!(!f.base.join("vault/trash.md").exists());
    }

    #[test]
    fn crashes_before_effects_and_at_each_member_classify_whole_proposal() {
        let cases = [
            ("intent", 0, ApplyOutcome::NotApplied),
            ("mirror-intent", 0, ApplyOutcome::NotApplied),
            ("stage", 0, ApplyOutcome::NotApplied),
            ("stage", 1, ApplyOutcome::NotApplied),
            ("prepared-db", 0, ApplyOutcome::NotApplied),
            ("prepared", 0, ApplyOutcome::NotApplied),
            ("member", 0, ApplyOutcome::Uncertain),
            ("member", 1, ApplyOutcome::Uncertain),
            ("member", 2, ApplyOutcome::Applied),
            ("synced", 2, ApplyOutcome::Applied),
            ("verified", 0, ApplyOutcome::Applied),
            ("completion", 0, ApplyOutcome::Applied),
            ("receipt", 0, ApplyOutcome::Applied),
        ];
        for (phase, index, expected) in cases {
            let f = Fixture::new();
            let request = f.prepare();
            f.crash(phase, index);
            let mut app = f.app();
            let result = app.reconcile_proposal(request.operation_id).unwrap();
            assert_eq!(result.outcome, expected, "{phase}/{index}");
            assert_eq!(
                app.approve_proposal(&request).unwrap(),
                result,
                "replay {phase}"
            );
            let journal = app
                .work_store()
                .proposal_apply(request.operation_id)
                .unwrap()
                .unwrap();
            assert!(
                !journal.no_effects,
                "restart never certifies no-effect admission"
            );
            if expected == ApplyOutcome::Uncertain {
                assert_eq!(app.note("a.md").unwrap_err().kind, ErrorKind::SaveUncertain);
                assert_eq!(
                    fs::read(f.base.join("vault/a.md")).unwrap(),
                    "\u{feff}Approved λ\r\n".as_bytes()
                );
                assert!(
                    !app.proposal(request.expected.id)
                        .unwrap()
                        .comments
                        .is_empty()
                );
            } else {
                assert!(app.note("a.md").is_ok());
                if expected == ApplyOutcome::Applied {
                    assert!(
                        app.proposal(request.expected.id)
                            .unwrap()
                            .comments
                            .is_empty()
                    );
                }
            }
        }
    }

    #[test]
    fn pre_effect_external_occupant_refuses_and_preserves_proofs_comments_and_old_tools() {
        let f = Fixture::new();
        let request = f.prepare();
        let mut app = f.app();
        let tools = app.tools().unwrap();
        let external = f.base.join("vault/new.md");
        let lease = tools.clone();
        APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |step, _| {
                if step == "intent" {
                    assert_eq!(
                        lease.read_note("a.md").unwrap_err().kind,
                        AiErrorKind::IndexStale
                    );
                }
                if step == "prepared" {
                    fs::write(&external, "External occupant").unwrap();
                }
            }))
        });
        let result = app.approve_proposal(&request);
        APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert_eq!(result.unwrap_err().kind, ErrorKind::ContextStale);
        let journal = app
            .work_store()
            .proposal_apply(request.operation_id)
            .unwrap()
            .unwrap();
        assert!(journal.no_effects);
        assert_eq!(journal.receipt.unwrap().outcome, ApplyOutcome::NotApplied);
        assert!(!journal.approved.comments.is_empty());
        assert_eq!(
            fs::read_to_string(f.base.join("vault/new.md")).unwrap(),
            "External occupant"
        );
        assert_eq!(
            fs::read_to_string(f.base.join("vault/a.md")).unwrap(),
            "Original λ\r\n"
        );
        assert!(app.tools().is_ok());
        drop(app);
        drop(tools);
        assert!(f.app().tools().is_ok());
    }

    #[test]
    fn changed_later_member_after_first_effect_keeps_whole_proposal_uncertain() {
        let f = Fixture::new();
        let request = f.prepare();
        let mut app = f.app();
        let external = f.base.join("vault/new.md");
        APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |step, index| {
                if step == "synced" && index == 0 {
                    fs::write(&external, "External occupant").unwrap();
                }
            }))
        });
        let result = app.approve_proposal(&request);
        APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert!(result.is_err());
        let journal = app
            .work_store()
            .proposal_apply(request.operation_id)
            .unwrap()
            .unwrap();
        assert!(!journal.no_effects);
        assert_eq!(journal.receipt.unwrap().outcome, ApplyOutcome::Uncertain);
        assert_eq!(app.note("a.md").unwrap_err().kind, ErrorKind::SaveUncertain);
        assert_eq!(
            app.reconcile_proposal(request.operation_id)
                .unwrap()
                .outcome,
            ApplyOutcome::Uncertain
        );
        assert_eq!(
            fs::read_to_string(f.base.join("vault/new.md")).unwrap(),
            "External occupant"
        );
    }

    #[test]
    fn later_old_stamp_typing_is_retained_after_apply_and_restart() {
        let f = Fixture::new();
        let request = f.prepare();
        let mut app = f.app();
        let before = app.open_editor("a.md").unwrap().record;
        app.approve_proposal(&request).unwrap();
        let later = app
            .recover_editor(&EditRequest {
                path: "a.md".into(),
                expected: before.stamp,
                generation: before.stamp.generation + 1,
                text: "Later typing 🦀".into(),
            })
            .unwrap();
        assert_eq!(later.baseline, before.baseline);
        let view = app.open_editor("a.md").unwrap();
        assert!(view.conflict);
        assert_eq!(view.record.text, "Later typing 🦀");
        assert_eq!(view.saved.as_deref(), Some("\u{feff}Approved λ\r\n"));
        drop(app);
        let mut app = f.app();
        assert_eq!(app.open_editor("a.md").unwrap().record, later);
    }

    #[test]
    fn sqlite_receipt_failure_restores_durable_completion_and_deletes_old_comments() {
        let f = Fixture::new();
        let request = f.prepare();
        let mut app = f.app();
        let database = f.base.join("data/brn.sqlite");
        APPLY_HOOK.with(|hook| *hook.borrow_mut() = Some(Box::new(move |step, _| {
            if step == "completion" {
                rusqlite::Connection::open(&database).unwrap().execute_batch("CREATE TRIGGER fail_receipt BEFORE UPDATE ON proposals WHEN json_extract(NEW.record_json, '$.record.state')='applied' BEGIN SELECT RAISE(ABORT, 'synthetic receipt failure'); END;").unwrap();
            }
        })));
        let result = app.approve_proposal(&request);
        APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert!(result.is_err());
        assert!(
            app.work_store()
                .proposal_apply(request.operation_id)
                .unwrap()
                .unwrap()
                .receipt
                .is_none()
        );
        assert_eq!(app.note("a.md").unwrap_err().kind, ErrorKind::SaveUncertain);
        rusqlite::Connection::open(f.base.join("data/brn.sqlite"))
            .unwrap()
            .execute_batch("DROP TRIGGER fail_receipt;")
            .unwrap();
        let result = app.reconcile_proposal(request.operation_id).unwrap();
        assert_eq!(result.outcome, ApplyOutcome::Applied);
        assert!(
            app.proposal(request.expected.id)
                .unwrap()
                .comments
                .is_empty()
        );
        drop(app);
        assert!(f.app().tools().is_ok());
    }

    #[test]
    fn required_preparation_failures_never_mutate_targets_and_leave_review_readable() {
        for step in ["write", "attributes", "file_sync", "directory_sync"] {
            let f = Fixture::new();
            let request = f.prepare();
            let mut app = f.app();
            crate::files::PREPARE_FAILURE.with(|failure| failure.set(Some(step)));
            let result = app.approve_proposal(&request);
            crate::files::PREPARE_FAILURE.with(|failure| failure.set(None));
            assert!(result.is_err(), "{step}");
            let journal = app
                .work_store()
                .proposal_apply(request.operation_id)
                .unwrap()
                .unwrap();
            assert_eq!(journal.receipt.unwrap().outcome, ApplyOutcome::NotApplied);
            assert!(journal.no_effects);
            assert_eq!(
                app.proposal(request.expected.id).unwrap().state,
                ProposalState::Draft
            );
            assert!(
                !app.proposal(request.expected.id)
                    .unwrap()
                    .comments
                    .is_empty()
            );
            assert_eq!(
                fs::read_to_string(f.base.join("vault/a.md")).unwrap(),
                "Original λ\r\n"
            );
            assert!(!f.base.join("vault/new.md").exists());
        }
    }

    #[test]
    fn interrupted_whole_file_proof_still_requires_sources_but_historical_completion_does_not() {
        for (phase, expected) in [
            ("member", ApplyOutcome::Uncertain),
            ("completion", ApplyOutcome::Applied),
        ] {
            let f = Fixture::new();
            let request = f.prepare();
            f.crash(phase, if phase == "member" { 2 } else { 0 });
            fs::write(f.base.join("vault/source.md"), "Changed source evidence").unwrap();
            let mut app = f.app();
            let receipt = app.reconcile_proposal(request.operation_id).unwrap();
            assert_eq!(receipt.outcome, expected, "{phase}");
            assert_eq!(
                fs::read_to_string(f.base.join("vault/a.md")).unwrap(),
                "\u{feff}Approved λ\r\n"
            );
            assert_eq!(app.note("a.md").is_ok(), expected == ApplyOutcome::Applied);
        }
    }

    #[test]
    fn recovery_record_sync_failures_keep_fence_until_fresh_proof_recovery() {
        for (phase, expected) in [
            ("prepared-db", ApplyOutcome::NotApplied),
            ("verified", ApplyOutcome::Applied),
        ] {
            for failure in [
                "file_sync",
                "prepare_directory_sync",
                "directory_sync",
                "postproof",
                "cleanup_directory_sync",
            ] {
                let f = Fixture::new();
                let request = f.prepare();
                let mut app = f.app();
                APPLY_HOOK.with(|hook| {
                    *hook.borrow_mut() = Some(Box::new(move |step, _| {
                        if step == phase {
                            crate::files::recovery::FAILURE
                                .with(|selected| selected.set(Some(failure)));
                        }
                    }))
                });
                let result = app.approve_proposal(&request);
                APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
                crate::files::recovery::FAILURE.with(|selected| selected.set(None));
                assert!(result.is_err(), "{phase}/{failure}");
                assert_eq!(
                    app.note("a.md").unwrap_err().kind,
                    ErrorKind::SaveUncertain,
                    "{phase}/{failure}"
                );
                drop(app);
                let mut app = f.app();
                assert_eq!(
                    app.reconcile_proposal(request.operation_id)
                        .unwrap()
                        .outcome,
                    expected,
                    "{phase}/{failure}"
                );
                assert!(app.note("a.md").is_ok());
            }
        }
    }

    #[test]
    fn interrupted_applied_mirror_cleanup_retires_comment_bearing_displacement_before_ready() {
        let f = Fixture::new();
        let request = f.prepare();
        let mut app = f.app();
        APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(|step, _| {
                if step == "verified" {
                    crate::files::recovery::FAILURE.with(|selected| selected.set(Some("cleanup")));
                }
            }))
        });
        let result = app.approve_proposal(&request);
        APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        crate::files::recovery::FAILURE.with(|selected| selected.set(None));
        assert!(result.is_err());
        assert!(
            app.work_store()
                .proposal_apply(request.operation_id)
                .unwrap()
                .unwrap()
                .receipt
                .is_none()
        );
        let temporary = || {
            fs::read_dir(f.base.join("data"))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|p| {
                    p.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with(".brn-apply-temp-")
                })
                .collect::<Vec<_>>()
        };
        let retained = temporary();
        assert_eq!(retained.len(), 1);
        let retained_body: serde_json::Value =
            serde_json::from_slice(&fs::read(&retained[0]).unwrap()).unwrap();
        assert!(
            !retained_body["journal"]["approved"]["comments"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        drop(app);
        let mut app = f.app();
        assert!(temporary().is_empty());
        assert_eq!(
            app.reconcile_proposal(request.operation_id)
                .unwrap()
                .outcome,
            ApplyOutcome::Applied
        );
        assert!(
            app.proposal(request.expected.id)
                .unwrap()
                .comments
                .is_empty()
        );
        assert!(app.tools().is_ok());
    }

    #[test]
    fn older_database_replays_refused_then_applied_history_without_reviving_annotations() {
        for old_pending in [false, true] {
            let f = Fixture::new();
            let first = f.prepare();
            if old_pending {
                let mut setup = f.app();
                setup.store.begin_proposal_apply(&first).unwrap();
            }
            let earlier = fs::read(f.base.join("data/brn.sqlite")).unwrap();
            let mut app = f.app();
            if old_pending {
                app.reconcile_proposal(first.operation_id).unwrap();
            }
            let occupant = f.base.join("vault/new.md");
            let copied = occupant.clone();
            APPLY_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |step, _| {
                    if step == "prepared" {
                        fs::write(&copied, "Unexpected occupant").unwrap();
                    }
                }))
            });
            if !old_pending {
                assert!(app.approve_proposal(&first).is_err());
            }
            APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
            let refused = app
                .work_store()
                .proposal_apply(first.operation_id)
                .unwrap()
                .unwrap()
                .receipt
                .unwrap();
            assert_eq!(refused.outcome, ApplyOutcome::NotApplied);
            if !old_pending {
                fs::remove_file(&occupant).unwrap();
            }
            let updated = app.proposal(first.expected.id).unwrap();
            let second = ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: updated.stamp(),
            };
            let applied = app.approve_proposal(&second).unwrap();
            assert_eq!(applied.outcome, ApplyOutcome::Applied);
            for journal in app.work_store().proposal_applies().unwrap() {
                assert!(journal.approved.comments.is_empty());
            }
            for id in app.apply_records.as_ref().unwrap().ids().unwrap() {
                assert!(
                    app.apply_records
                        .as_ref()
                        .unwrap()
                        .read(id)
                        .unwrap()
                        .unwrap()
                        .journal
                        .approved
                        .comments
                        .is_empty()
                );
            }
            drop(app);
            fs::write(f.base.join("data/brn.sqlite"), earlier).unwrap();
            let mut app = f.app();
            assert_eq!(app.approve_proposal(&first).unwrap(), refused);
            assert_eq!(app.approve_proposal(&second).unwrap(), applied);
            assert_eq!(
                app.proposal(first.expected.id).unwrap().state,
                ProposalState::Applied
            );
            assert!(app.proposal(first.expected.id).unwrap().comments.is_empty());
            assert!(app.tools().is_ok());
        }
    }
}
