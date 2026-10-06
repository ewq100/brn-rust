//! Explicit owner confirmation and recoverable private-copy effects. No AI authority.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    files::inbox::{
        InboxFiles, InboxRoot, OriginalOperationFile, checkpoint, original_operation_name,
    },
    inbox::restore_inbox_captures,
};
pub use brn_store::work::inbox_original_operations::legacy::ArchivedInboxAnalysis;
pub use brn_store::work::inbox_original_operations::{
    InboxOriginalOperation, InboxOriginalOperationSummary, InboxOriginalParent,
    InboxOriginalRemovalRecord, InboxOriginalRestoreRecord, InboxRemovalConfirmation,
    RemoveInboxOriginalRequest, RestoreInboxOriginalRequest,
};
use brn_store::{
    WorkStore,
    work::inbox_original_operations::{
        InboxOriginalNamespace, InboxOriginalOperationKind, InboxQualifiedRemovalEvidence,
    },
};
use std::collections::{BTreeMap, HashMap, HashSet};
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
fn now_ms(minimum: u64) -> u64 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as u64)
        .max(minimum)
}
fn intent(mut record: InboxOriginalOperation) -> InboxOriginalOperation {
    match &mut record {
        InboxOriginalOperation::Remove(r) => r.removed_at_ms = None,
        InboxOriginalOperation::Restore(r) => r.restored_at_ms = None,
        InboxOriginalOperation::LegacyRemove(r) => r.removed_at_ms = None,
        InboxOriginalOperation::LegacyRestore(r) => r.restored_at_ms = None,
    }
    record
}
fn settle(record: &mut OriginalOperationFile) {
    match record {
        OriginalOperationFile::Remove(r) => r.removed_at_ms = Some(now_ms(r.prepared_at_ms)),
        OriginalOperationFile::Restore(r) => r.restored_at_ms = Some(now_ms(r.prepared_at_ms)),
        OriginalOperationFile::LegacyRemove(r) => r.removed_at_ms = Some(now_ms(r.prepared_at_ms)),
        OriginalOperationFile::LegacyRestore(r) => {
            r.restored_at_ms = Some(now_ms(r.prepared_at_ms))
        }
    }
}
#[derive(PartialEq, Eq)]
struct MirrorIdentity {
    format: u8,
    intent_digest: [u8; 32],
    receipt_digest: Option<[u8; 32]>,
}
fn mirror_identity(record: InboxOriginalOperation) -> Result<MirrorIdentity> {
    let receipt_digest = record
        .summary()?
        .settled_at_ms
        .map(|_| record.digest())
        .transpose()?;
    Ok(MirrorIdentity {
        format: record.format(),
        intent_digest: intent(record).digest()?,
        receipt_digest,
    })
}
struct SelectedMirror {
    name: String,
    identity: MirrorIdentity,
}
/// Reopen each checked mirror once inside one atomic Store import. Retain only
/// compact identities during discovery, never all historical certificate bodies.
fn import_mirrors(
    store: &mut WorkStore,
    files: &InboxFiles,
    names: &[String],
) -> Result<brn_store::work::inbox_original_operations::InboxOriginalInventory> {
    let mut selected: BTreeMap<Uuid, SelectedMirror> = BTreeMap::new();
    for name in names.iter().filter(|n| original_operation_name(n)) {
        let record: InboxOriginalOperation = files.operation(name)?.into();
        let id = record.summary()?.operation_id;
        let identity = mirror_identity(record)?;
        if let Some(prior) = selected.get(&id) {
            if prior.identity.format != identity.format
                || prior.identity.intent_digest != identity.intent_digest
                || prior
                    .identity
                    .receipt_digest
                    .zip(identity.receipt_digest)
                    .is_some_and(|(a, b)| a != b)
            {
                return Err(stale(
                    "Inbox intent and receipt have different exact bodies",
                ));
            }
            if prior.identity.receipt_digest.is_some() {
                continue;
            }
        }
        selected.insert(
            id,
            SelectedMirror {
                name: name.clone(),
                identity,
            },
        );
    }
    let mut file_error = None;
    let records = selected.values().map(|selected| {
        let read = (|| -> Result<InboxOriginalOperation> {
            let record: InboxOriginalOperation = files.operation(&selected.name)?.into();
            if mirror_identity(record.clone())? != selected.identity {
                return Err(stale("Inbox operation mirror changed after inventory"));
            }
            Ok(record)
        })();
        read.map_err(|error| {
            file_error = Some(error);
            brn_store::Error::Invalid("Inbox operation mirror changed during recovery".into())
        })
    });
    let imported = store.restore_inbox_original_operation_records(records, &[]);
    if let Some(error) = file_error {
        return Err(error);
    }
    Ok(imported?)
}
fn checked_root(store: &WorkStore) -> Result<Option<InboxRoot>> {
    store
        .setting("inbox.root")?
        .map(|raw| {
            let root: InboxRoot =
                serde_json::from_str(&raw).map_err(|_| stale("malformed Inbox root"))?;
            if serde_json::to_string(&root).ok().as_ref() != Some(&raw) {
                return Err(stale("Inbox root canonical binding differs"));
            }
            Ok(root)
        })
        .transpose()
}
/// Before approval recovery: restore exact catalog identities and certified
/// legacy jobs. Never fabricate chat, fresh admission or filesystem effects.
pub(crate) fn bootstrap(store: &mut WorkStore) -> Result<()> {
    let has_records = !store.inbox_original_operations(&[])?.history.is_empty();
    let bound = match checked_root(store) {
        Ok(bound) => bound,
        Err(error) if has_records => return Err(error),
        Err(_) => return Ok(()), // ordinary Inbox issues are reported by intake
    };
    let files = match InboxFiles::open(store.data_dir(), bound.as_ref(), false) {
        Ok(Some(files)) => files,
        Ok(None) if !has_records => return Ok(()),
        Err(_) if !has_records => return Ok(()),
        Ok(None) => {
            return Err(stale(
                "recorded original operations have no Inbox namespace",
            ));
        }
        Err(error) => return Err(error),
    };
    let names = files.names()?;
    if has_records || names.iter().any(|n| original_operation_name(n)) {
        import_mirrors(store, &files, &names)?;
    }
    Ok(())
}
pub(crate) struct RemovedOriginal {
    pub operation_id: Uuid,
    pub namespace: InboxOriginalNamespace,
}
pub(crate) struct RecoveredOriginals {
    pub known: HashSet<String>,
    pub heads: HashMap<Uuid, InboxOriginalOperationSummary>,
    pub removed: HashMap<Uuid, RemovedOriginal>,
}
fn heads(
    history: &[InboxOriginalOperationSummary],
) -> HashMap<Uuid, InboxOriginalOperationSummary> {
    // Store has checked and ordered each family causally, independent of clocks.
    history.iter().map(|s| (s.item_id, s.clone())).collect()
}
pub(crate) fn restore_records(
    store: &mut WorkStore,
    files: &InboxFiles,
    names: &[String],
) -> Result<RecoveredOriginals> {
    let imported = import_mirrors(store, files, names)?;
    let mut recovered = RecoveredOriginals {
        known: names
            .iter()
            .filter(|n| original_operation_name(n))
            .cloned()
            .collect(),
        heads: heads(&imported.history),
        removed: HashMap::new(),
    };
    let mut observed_receipts = Vec::new();
    store.visit_inbox_original_operations::<WorkflowError>(|record| {
        let summary = record.summary()?;
        let is_head = recovered.heads.get(&summary.item_id) == Some(&summary);
        let mut operation = OriginalOperationFile::from(record);
        let prepared =
            OriginalOperationFile::from(intent(InboxOriginalOperation::from(operation.clone())));
        files.publish_operation(&prepared)?;
        recovered.known.insert(prepared.name());
        if operation.settled() {
            files.publish_operation(&operation)?;
            recovered.known.insert(operation.name());
        } else if is_head && files.operation_effect_observed(&operation)? {
            // Untouched endpoints stay pending. Startup only acknowledges a
            // previously completed, exact exclusive move; it never starts one.
            settle(&mut operation);
            files.publish_operation(&operation)?;
            observed_receipts.push(operation.name());
            recovered.known.insert(operation.name());
        }
        if is_head {
            recovered.known.insert(operation.item().capture.copy_name());
            if operation.settled() && summary.kind == InboxOriginalOperationKind::Remove {
                recovered
                    .known
                    .insert(format!(".brn-inbox-removed-{}.original", operation.id()));
                recovered.removed.insert(
                    summary.item_id,
                    RemovedOriginal {
                        operation_id: operation.id(),
                        namespace: operation.namespace().clone(),
                    },
                );
            } else if summary.kind == InboxOriginalOperationKind::Restore
                && !operation.settled()
                && let Some(parent) = summary.parent
            {
                recovered
                    .known
                    .insert(format!(".brn-inbox-removed-{parent}.original"));
            }
        }
        Ok(())
    })?;
    // Store mutation follows the visitor's read transaction. Each observed body
    // is rechecked and imported atomically, with no per-item family scans.
    if !observed_receipts.is_empty() {
        recovered.heads = heads(&import_mirrors(store, files, &observed_receipts)?.history);
    }
    Ok(recovered)
}
fn selected(store: &WorkStore, id: Uuid) -> Result<Option<InboxOriginalOperation>> {
    if id.is_nil() {
        return Err(stale("original operation lookup requires a nonnil UUID"));
    }
    match store.inbox_original_operations(&[id]) {
        Ok(mut inventory) => Ok(inventory.selected.pop()),
        Err(brn_store::Error::NotFound(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}
fn persist(store: &mut WorkStore, record: InboxOriginalOperation) -> Result<()> {
    store.restore_inbox_original_operation_records(std::iter::once(Ok(record)), &[])?;
    Ok(())
}
impl App {
    pub fn inbox_original_removal(&self, id: Uuid) -> Result<Option<InboxOriginalOperation>> {
        let record = selected(&self.store, id)?;
        Ok(record.filter(|r| {
            matches!(
                r,
                InboxOriginalOperation::Remove(_) | InboxOriginalOperation::LegacyRemove(_)
            )
        }))
    }
    pub fn inbox_original_restore(&self, id: Uuid) -> Result<Option<InboxOriginalOperation>> {
        let record = selected(&self.store, id)?;
        Ok(record.filter(|r| {
            matches!(
                r,
                InboxOriginalOperation::Restore(_) | InboxOriginalOperation::LegacyRestore(_)
            )
        }))
    }
    pub fn inbox_original_operations(
        &self,
        item: Uuid,
    ) -> Result<Vec<InboxOriginalOperationSummary>> {
        if item.is_nil() {
            return Err(stale("original history requires a nonnil capture UUID"));
        }
        Ok(self
            .store
            .inbox_original_operations(&[])?
            .history
            .into_iter()
            .filter(|s| s.item_id == item)
            .collect())
    }
    pub fn archived_inbox_analysis(&self, id: Uuid) -> Result<Option<ArchivedInboxAnalysis>> {
        if id.is_nil() {
            return Err(stale("analysis lookup requires a nonnil UUID"));
        }
        if self.store.turn(id)?.is_some() {
            return Ok(None);
        }
        let mut archived: Option<ArchivedInboxAnalysis> = None;
        self.store
            .visit_inbox_original_operations::<WorkflowError>(|record| {
                if let InboxOriginalOperation::LegacyRemove(record) = record {
                    for analysis in &record.evidence.snapshot.review.analyses {
                        if analysis.job.capture.id == id
                            && archived.as_ref().is_none_or(|old| {
                                (old.analysis.turn.is_none() && analysis.turn.is_some())
                                    || (old.analysis.turn.is_some() == analysis.turn.is_some()
                                        && record.request.operation_id < old.removal_operation_id)
                            })
                        {
                            archived = Some(ArchivedInboxAnalysis {
                                removal_operation_id: record.request.operation_id,
                                analysis: analysis.clone(),
                            });
                        }
                    }
                }
                Ok(())
            })?;
        Ok(archived)
    }
    fn qualify_original_removal(
        &mut self,
        request: &RemoveInboxOriginalRequest,
    ) -> Result<InboxQualifiedRemovalEvidence> {
        let preview = self.preview_inbox_removal(request.item_id)?;
        if preview.digest != request.preview_digest || !preview.evidence.blockers.is_empty() {
            return Err(stale(
                "original-removal preview changed or contains blockers; review a fresh preview",
            ));
        }
        // These checked DTOs share the exact canonical qualified-preview shape.
        let evidence: InboxQualifiedRemovalEvidence = serde_json::from_slice(
            &serde_json::to_vec(&preview.evidence)
                .map_err(|_| stale("could not encode exact removal preview"))?,
        )
        .map_err(|_| stale("qualified removal preview has another complete shape"))?;
        if evidence.digest()? != request.preview_digest {
            return Err(stale("qualified certificate differs from owner preview"));
        }
        Ok(evidence)
    }
    fn original_head(
        &self,
        item: Uuid,
        own_pending: Uuid,
    ) -> Result<Option<InboxOriginalOperationSummary>> {
        let history = self.inbox_original_operations(item)?;
        let mut iter = history.into_iter().rev();
        let Some(head) = iter.next() else {
            return Ok(None);
        };
        if head.operation_id == own_pending && head.settled_at_ms.is_none() {
            return Ok(iter.next());
        }
        if head.settled_at_ms.is_none() {
            return Err(uncertain("another original operation is unsettled"));
        }
        Ok(Some(head))
    }
    pub fn remove_inbox_original(
        &mut self,
        request: &RemoveInboxOriginalRequest,
    ) -> Result<InboxOriginalRemovalRecord> {
        request.validate()?;
        let replay = |record: InboxOriginalOperation| -> Result<InboxOriginalRemovalRecord> {
            match record {
                InboxOriginalOperation::Remove(r) if r.request == *request => Ok(*r),
                _ => Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "original removal UUID has another exact request or format",
                )),
            }
        };
        if let Some(record) = selected(&self.store, request.operation_id)? {
            let record = replay(record)?;
            if record.removed_at_ms.is_some() {
                return Ok(record);
            }
        }
        self.inbox = restore_inbox_captures(&mut self.store)?;
        let pending = selected(&self.store, request.operation_id)?
            .map(replay)
            .transpose()?;
        if let Some(record) = &pending
            && record.removed_at_ms.is_some()
        {
            return Ok(record.clone());
        }
        let parent = self.original_head(request.item_id, request.operation_id)?;
        let minimum = match (&parent, &request.previous_restore) {
            (None, None) => 0,
            (Some(head), Some(expected))
                if head.kind == InboxOriginalOperationKind::Restore
                    && head.operation_id == expected.operation_id
                    && head.record_sha256 == expected.record_sha256 =>
            {
                head.settled_at_ms
                    .ok_or_else(|| stale("prior restoration is unsettled"))?
            }
            _ => {
                return Err(stale(
                    "removal does not bind the current exact restoration head",
                ));
            }
        };
        let evidence = self.qualify_original_removal(request)?;
        let namespace = self
            .inbox
            .files
            .as_ref()
            .ok_or_else(|| stale("owned Inbox files are unavailable"))?
            .original_namespace();
        let mut record = match pending {
            Some(record) => {
                if record.evidence.digest()? != evidence.digest()? || record.namespace != namespace
                {
                    return Err(stale(
                        "pending removal differs from freshly qualified original and Source",
                    ));
                }
                record
            }
            None => InboxOriginalRemovalRecord {
                request: request.clone(),
                prepared_at_ms: now_ms(minimum.max(evidence.item.received_at_ms)),
                evidence,
                namespace,
                removed_at_ms: None,
            },
        };
        persist(
            &mut self.store,
            InboxOriginalOperation::Remove(Box::new(record.clone())),
        )?;
        checkpoint("original_operation_store_intent").map_err(uncertain)?;
        let operation = OriginalOperationFile::Remove(Box::new(record.clone()));
        self.inbox
            .files
            .as_ref()
            .expect("qualified files")
            .publish_operation(&operation)
            .map_err(uncertain)?;
        // Repeat the complete selected Source proof after bounded intent I/O.
        self.qualify_original_removal(request)?;
        let files = self.inbox.files.as_ref().expect("qualified files");
        files.move_original(&operation).map_err(uncertain)?;
        record.removed_at_ms = Some(now_ms(record.prepared_at_ms));
        files
            .publish_operation(&OriginalOperationFile::Remove(Box::new(record.clone())))
            .map_err(uncertain)?;
        persist(
            &mut self.store,
            InboxOriginalOperation::Remove(Box::new(record.clone())),
        )
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
        let replay = |record: InboxOriginalOperation| -> Result<InboxOriginalRestoreRecord> {
            match record {
                InboxOriginalOperation::Restore(r) if r.request == *request => Ok(*r),
                _ => Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "original restoration UUID has another exact request or format",
                )),
            }
        };
        if let Some(record) = selected(&self.store, request.operation_id)? {
            let record = replay(record)?;
            if record.restored_at_ms.is_some() {
                return Ok(record);
            }
        }
        self.inbox = restore_inbox_captures(&mut self.store)?;
        let pending = selected(&self.store, request.operation_id)?
            .map(replay)
            .transpose()?;
        if let Some(record) = &pending
            && record.restored_at_ms.is_some()
        {
            return Ok(record.clone());
        }
        let removal = self
            .inbox_original_removal(request.removal_operation_id)?
            .ok_or_else(|| stale("original removal is unknown"))?;
        let summary = removal.summary()?;
        let head = self.original_head(summary.item_id, request.operation_id)?;
        if head.as_ref() != Some(&summary)
            || summary.settled_at_ms.is_none()
            || summary.record_sha256 != request.removal_digest
        {
            return Err(stale(
                "restoration does not bind the current settled removal head",
            ));
        }
        let files = self
            .inbox
            .files
            .as_ref()
            .ok_or_else(|| stale("owned Inbox files are unavailable"))?;
        if files.original_namespace() != *removal.namespace() {
            return Err(stale("original namespace changed"));
        }
        files.retained_copy(
            removal.original(),
            summary.operation_id,
            removal.namespace(),
        )?;
        if files.original_occupied(removal.original())? {
            return Err(stale(
                "original restoration destination is occupied; it remains untouched",
            ));
        }
        let mut record = pending.unwrap_or_else(|| InboxOriginalRestoreRecord {
            request: request.clone(),
            original: removal.original().clone(),
            namespace: removal.namespace().clone(),
            prepared_at_ms: now_ms(summary.settled_at_ms.unwrap_or_default()),
            restored_at_ms: None,
        });
        if record.original != *removal.original() || record.namespace != *removal.namespace() {
            return Err(stale(
                "pending restoration differs from exact removal endpoints",
            ));
        }
        persist(
            &mut self.store,
            InboxOriginalOperation::Restore(Box::new(record.clone())),
        )?;
        checkpoint("original_operation_store_intent").map_err(uncertain)?;
        let files = self.inbox.files.as_ref().expect("qualified files");
        let operation = OriginalOperationFile::Restore(Box::new(record.clone()));
        files.publish_operation(&operation).map_err(uncertain)?;
        files.move_original(&operation).map_err(uncertain)?;
        record.restored_at_ms = Some(now_ms(record.prepared_at_ms));
        files
            .publish_operation(&OriginalOperationFile::Restore(Box::new(record.clone())))
            .map_err(uncertain)?;
        persist(
            &mut self.store,
            InboxOriginalOperation::Restore(Box::new(record.clone())),
        )
        .map_err(uncertain)?;
        checkpoint("original_operation_store_receipt").map_err(uncertain)?;
        self.inbox = restore_inbox_captures(&mut self.store)?;
        Ok(record)
    }
}
