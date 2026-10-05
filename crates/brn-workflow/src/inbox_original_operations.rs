//! Exact owner attestation and recoverable private-copy effects. No AI authority.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    files::inbox::{
        InboxFiles, InboxRoot, OriginalOperationFile, checkpoint, original_operation_name,
    },
    inbox::{InboxOriginal, restore_inbox_captures},
    inbox_removal::InboxRemovalPreview,
};
pub use brn_store::work::inbox_original_operations::{
    ArchivedInboxAnalysis, InboxOriginalOperationSummary, InboxOriginalRemovalRecord,
    InboxOriginalRestoreRecord, InboxRemovalAttestation, RemoveInboxOriginalRequest,
    RestoreInboxOriginalRequest,
};
use brn_store::{
    WorkStore,
    work::inbox_original_operations::{
        InboxOriginalOperationKind, InboxQualifiedOriginal, InboxQualifiedRemovalEvidence,
    },
};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

fn uncertain(error: impl std::fmt::Display) -> WorkflowError {
    WorkflowError::typed(
        ErrorKind::InboxUncertain,
        format!(
            "original operation may be interrupted; inspect retained records and endpoints: {error}"
        ),
    )
}
fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}
fn now_ms(prepared: u64) -> u64 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64)
        .max(prepared)
}
fn parent(record: &OriginalOperationFile) -> Option<Uuid> {
    match record {
        OriginalOperationFile::Remove(r) => r.request.previous_restore,
        OriginalOperationFile::Restore(r) => Some(r.request.removal_operation_id),
    }
}
fn intent(mut record: OriginalOperationFile) -> OriginalOperationFile {
    match &mut record {
        OriginalOperationFile::Remove(r) => r.removed_at_ms = None,
        OriginalOperationFile::Restore(r) => r.restored_at_ms = None,
    }
    record
}
fn operation_digest(record: &OriginalOperationFile) -> Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    Ok(Sha256::digest(
        serde_json::to_vec(record).map_err(|_| stale("could not encode original operation"))?,
    )
    .into())
}
fn persist(store: &mut WorkStore, record: &OriginalOperationFile) -> Result<()> {
    match record {
        OriginalOperationFile::Remove(r) => {
            store.restore_inbox_original_removal(r)?;
        }
        OriginalOperationFile::Restore(r) => {
            store.restore_inbox_original_restore(r)?;
        }
    }
    Ok(())
}
/// Called before approval recovery. It reserves only certified catalog/analysis
/// captures; complete historical chat/review evidence stays in the certificate.
pub(crate) fn bootstrap(store: &mut WorkStore) -> Result<()> {
    let bound = store
        .setting("inbox.root")?
        .map(|raw| {
            let bound = serde_json::from_str::<InboxRoot>(&raw)
                .map_err(|_| stale("malformed Inbox root"))?;
            if serde_json::to_string(&bound).ok().as_ref() != Some(&raw) {
                return Err(stale("Inbox root canonical binding differs"));
            }
            Ok(bound)
        })
        .transpose()?;
    let Some(files) = InboxFiles::open(store.data_dir(), bound.as_ref(), false)? else {
        return Ok(());
    };
    restore_records(store, &files, &files.names()?)?;
    Ok(())
}
pub(crate) fn restore_records(
    store: &mut WorkStore,
    files: &InboxFiles,
    names: &[String],
) -> Result<HashSet<String>> {
    struct RecoveryFile {
        name: String,
        parent: Option<Uuid>,
        intent_digest: [u8; 32],
        receipt_digest: Option<[u8; 32]>,
    }
    // Inventory retains small identities only. Load one bounded full certificate
    // at a time instead of accumulating the bodies of all historical operations.
    let mut records: BTreeMap<Uuid, RecoveryFile> = BTreeMap::new();
    let mut known = HashSet::new();
    for name in names.iter().filter(|name| original_operation_name(name)) {
        let record = files.operation(name)?;
        let intent_digest = operation_digest(&intent(record.clone()))?;
        let receipt_digest = record
            .settled()
            .then(|| operation_digest(&record))
            .transpose()?;
        if let Some(prior) = records.get(&record.id()) {
            if prior.intent_digest != intent_digest {
                return Err(stale(
                    "Inbox intent and receipt have different immutable bodies",
                ));
            }
            if prior.receipt_digest.is_some()
                && receipt_digest.is_some()
                && prior.receipt_digest != receipt_digest
            {
                return Err(stale("Inbox receipt timestamp fork"));
            }
        }
        if !records
            .get(&record.id())
            .is_some_and(|r| r.receipt_digest.is_some())
        {
            records.insert(
                record.id(),
                RecoveryFile {
                    name: name.clone(),
                    parent: parent(&record),
                    intent_digest,
                    receipt_digest,
                },
            );
        }
        known.insert(name.clone());
    }
    // Restore in explicit causal order. Wall-clock observations never choose a
    // winner, and missing parents or forks cannot silently become fresh work.
    let mut restored: HashSet<_> = store
        .inbox_original_operation_ids()?
        .into_iter()
        .filter(|id| !records.contains_key(id))
        .collect();
    while !records.is_empty() {
        let ready = records
            .iter()
            .find(|(_, r)| r.parent.is_none_or(|p| restored.contains(&p)))
            .map(|(id, _)| *id)
            .ok_or_else(|| stale("Inbox operation recovery has a missing or cyclic parent"))?;
        let selected = records.remove(&ready).expect("selected record");
        let record = files.operation(&selected.name)?;
        if parent(&record) != selected.parent
            || operation_digest(&intent(record.clone()))? != selected.intent_digest
            || record
                .settled()
                .then(|| operation_digest(&record))
                .transpose()?
                != selected.receipt_digest
        {
            return Err(stale("Inbox operation mirror changed after inventory"));
        }
        persist(store, &record)?;
        restored.insert(ready);
    }
    let ids = store.inbox_original_operation_ids()?;
    let mut items = HashSet::new();
    for id in ids {
        let summary = store
            .inbox_original_operation_summary(id)?
            .ok_or_else(|| stale("listed Inbox operation disappeared"))?;
        items.insert(summary.item_id);
        let record = match summary.kind {
            InboxOriginalOperationKind::Remove => OriginalOperationFile::Remove(Box::new(
                store
                    .inbox_original_removal(id)?
                    .ok_or_else(|| stale("listed removal disappeared"))?,
            )),
            InboxOriginalOperationKind::Restore => OriginalOperationFile::Restore(Box::new(
                store
                    .inbox_original_restore(id)?
                    .ok_or_else(|| stale("listed restoration disappeared"))?,
            )),
        };
        // Checked SQLite/backup records may replace missing ordinary mirrors,
        // never damaged or conflicting ones. This performs no namespace effect.
        let prepared = intent(record.clone());
        files.publish_operation(&prepared)?;
        known.insert(prepared.name());
        if record.settled() {
            files.publish_operation(&record)?;
            known.insert(record.name());
        }
    }
    for item in items {
        let head = store
            .inbox_original_operation_head(item)?
            .ok_or_else(|| stale("original operation family has no head"))?;
        if head.settled_at_ms.is_some() {
            continue;
        }
        let mut record = match head.kind {
            InboxOriginalOperationKind::Remove => OriginalOperationFile::Remove(Box::new(
                store
                    .inbox_original_removal(head.operation_id)?
                    .ok_or_else(|| stale("removal intent disappeared"))?,
            )),
            InboxOriginalOperationKind::Restore => OriginalOperationFile::Restore(Box::new(
                store
                    .inbox_original_restore(head.operation_id)?
                    .ok_or_else(|| stale("restore intent disappeared"))?,
            )),
        };
        // An intent with untouched endpoints stays pending. Never initiate its
        // effect during startup or infer intent from a missing capture alone.
        if files.operation_effect_observed(&record)? {
            match &mut record {
                OriginalOperationFile::Remove(r) => {
                    r.removed_at_ms = Some(now_ms(r.prepared_at_ms))
                }
                OriginalOperationFile::Restore(r) => {
                    r.restored_at_ms = Some(now_ms(r.prepared_at_ms))
                }
            }
            files.publish_operation(&record)?;
            persist(store, &record)?;
            known.insert(record.name());
        }
    }
    Ok(known)
}

impl App {
    pub fn inbox_original_removal(&self, id: Uuid) -> Result<Option<InboxOriginalRemovalRecord>> {
        if id.is_nil() {
            return Err(stale(
                "original removal lookup requires a nonnil operation UUID",
            ));
        }
        Ok(self.store.inbox_original_removal(id)?)
    }
    pub fn inbox_original_restore(&self, id: Uuid) -> Result<Option<InboxOriginalRestoreRecord>> {
        if id.is_nil() {
            return Err(stale(
                "original restoration lookup requires a nonnil operation UUID",
            ));
        }
        Ok(self.store.inbox_original_restore(id)?)
    }
    pub fn archived_inbox_analysis(&self, id: Uuid) -> Result<Option<ArchivedInboxAnalysis>> {
        if id.is_nil() {
            return Err(stale("analysis lookup requires a nonnil UUID"));
        }
        Ok(self.store.archived_inbox_analysis(id)?)
    }
    pub fn inbox_original_operations(
        &self,
        item: Uuid,
    ) -> Result<Vec<InboxOriginalOperationSummary>> {
        if item.is_nil() {
            return Err(stale("original history requires a nonnil capture UUID"));
        }
        Ok(self.store.inbox_original_operation_summaries(item)?)
    }
    fn qualify_original_removal(
        &mut self,
        request: &RemoveInboxOriginalRequest,
        pending: Option<Uuid>,
    ) -> Result<InboxQualifiedRemovalEvidence> {
        let InboxRemovalPreview { mut evidence, .. } =
            self.preview_inbox_removal(request.item_id)?;
        if let Some(owned) = pending {
            evidence
                .snapshot
                .original_operations
                .retain(|s| s.operation_id != owned);
            evidence.blockers.retain(|b| !matches!(b, crate::inbox_removal::InboxRemovalBlocker::OriginalOperationUnsettled { operation_id } if *operation_id == owned));
        }
        if evidence.digest()? != request.preview_digest || !evidence.blockers.is_empty() {
            return Err(stale(
                "complete original-removal preview changed or contains blockers; review a fresh preview",
            ));
        }
        let InboxOriginal::Available { text } = evidence.original else {
            return Err(stale("exact original is unavailable"));
        };
        let qualified = InboxQualifiedRemovalEvidence {
            snapshot: evidence.snapshot,
            original: InboxQualifiedOriginal::Available { text },
            sources: evidence.sources,
            saved_consequences: evidence.saved_consequences,
            blockers: [],
            needs_owner_attestation: true,
        };
        if qualified.digest()? != request.preview_digest {
            return Err(stale(
                "qualified original certificate differs from owner preview",
            ));
        }
        Ok(qualified)
    }
    fn check_original_saved_files(
        &mut self,
        evidence: &InboxQualifiedRemovalEvidence,
    ) -> Result<()> {
        let files = self.editor_files()?;
        for saved in evidence
            .sources
            .iter()
            .map(|s| &s.saved)
            .chain(evidence.saved_consequences.iter().map(|s| &s.saved))
        {
            files
                .coordinate(std::path::Path::new(&saved.source.path), || {
                    let current =
                        files.observe_uncoordinated(std::path::Path::new(&saved.source.path))?;
                    Ok(
                        if current.fingerprint == saved.source.fingerprint
                            && current.text == saved.text
                        {
                            Ok(())
                        } else {
                            Err(stale("saved outcome changed before original removal"))
                        },
                    )
                })
                .map_err(crate::editor::file_error)??;
        }
        files.validate_root().map_err(crate::editor::file_error)
    }
    pub fn remove_inbox_original(
        &mut self,
        request: &RemoveInboxOriginalRequest,
    ) -> Result<InboxOriginalRemovalRecord> {
        request.validate()?;
        // Immutable replay is checked before any fresh vault/filesystem eligibility.
        if let Some(record) = self.store.inbox_original_removal(request.operation_id)? {
            if record.request != *request {
                return Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "original removal UUID has another exact owner request",
                ));
            }
            if record.removed_at_ms.is_some() {
                return Ok(record);
            }
        }
        self.inbox = restore_inbox_captures(&mut self.store)?;
        if let Some(record) = self.store.inbox_original_removal(request.operation_id)? {
            if record.removed_at_ms.is_some() {
                return Ok(record);
            }
        }
        let pending = self
            .store
            .inbox_original_removal(request.operation_id)?
            .map(|r| r.request.operation_id);
        let evidence = self.qualify_original_removal(request, pending)?;
        let namespace = self
            .inbox
            .files
            .as_ref()
            .ok_or_else(|| stale("owned Inbox files are unavailable"))?
            .original_namespace();
        let mut record = self
            .store
            .prepare_inbox_original_removal(request, &evidence, &namespace)?;
        checkpoint("original_operation_store_intent").map_err(uncertain)?;
        let operation = OriginalOperationFile::Remove(Box::new(record.clone()));
        self.inbox
            .files
            .as_ref()
            .expect("qualified files")
            .publish_operation(&operation)
            .map_err(uncertain)?;
        // Intent publication may involve substantial bounded evidence I/O.
        // Repeat admission immediately before the effect, excluding only this
        // command's own intent. Later work or changed proofs require new review.
        let fresh = self.qualify_original_removal(request, Some(request.operation_id))?;
        self.check_original_saved_files(&fresh)?;
        let files = self.inbox.files.as_ref().expect("qualified files");
        files.move_original(&operation).map_err(uncertain)?;
        record.removed_at_ms = Some(now_ms(record.prepared_at_ms));
        files
            .publish_operation(&OriginalOperationFile::Remove(Box::new(record.clone())))
            .map_err(uncertain)?;
        let record = self
            .store
            .settle_inbox_original_removal(&record)
            .map_err(uncertain)?;
        checkpoint("original_operation_store_receipt").map_err(uncertain)?;
        self.inbox = restore_inbox_captures(&mut self.store)?;
        Ok(record)
    }
    pub fn restore_inbox_original(
        &mut self,
        request: &RestoreInboxOriginalRequest,
    ) -> Result<InboxOriginalRestoreRecord> {
        request.validate()?;
        if let Some(record) = self.store.inbox_original_restore(request.operation_id)? {
            if record.request != *request {
                return Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "original restoration UUID has another exact request",
                ));
            }
            if record.restored_at_ms.is_some() {
                return Ok(record);
            }
        }
        self.inbox = restore_inbox_captures(&mut self.store)?;
        if let Some(record) = self.store.inbox_original_restore(request.operation_id)? {
            if record.restored_at_ms.is_some() {
                return Ok(record);
            }
        }
        let files = self
            .inbox
            .files
            .as_ref()
            .ok_or_else(|| stale("owned Inbox files are unavailable"))?;
        let removal = self
            .store
            .inbox_original_removal(request.removal_operation_id)?
            .ok_or_else(|| stale("original removal is unknown"))?;
        files.retained_original(&removal)?;
        if files.original_occupied(&removal.evidence.snapshot.review.original)? {
            return Err(stale(
                "original restoration destination is occupied; it remains untouched",
            ));
        }
        let mut record = self.store.prepare_inbox_original_restore(request)?;
        checkpoint("original_operation_store_intent").map_err(uncertain)?;
        let operation = OriginalOperationFile::Restore(Box::new(record.clone()));
        files.publish_operation(&operation).map_err(uncertain)?;
        files.move_original(&operation).map_err(uncertain)?;
        record.restored_at_ms = Some(now_ms(record.prepared_at_ms));
        files
            .publish_operation(&OriginalOperationFile::Restore(Box::new(record.clone())))
            .map_err(uncertain)?;
        let record = self
            .store
            .settle_inbox_original_restore(&record)
            .map_err(uncertain)?;
        checkpoint("original_operation_store_receipt").map_err(uncertain)?;
        self.inbox = restore_inbox_captures(&mut self.store)?;
        Ok(record)
    }
}

#[cfg(all(test, target_os = "macos"))]
#[path = "inbox_original_operations_tests.rs"]
mod tests;
