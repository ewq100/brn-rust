use super::{
    files::{FileObservation, MacFiles},
    *,
};
use brn_store::{
    Store,
    notes::{
        DestinationPrecondition, FileFingerprint, NoteReconciliation, NoteSaveIntent,
        NoteVerification, NoteWriteKind, RetainedArtifact,
    },
};
use std::cell::Cell;

#[derive(Default)]
pub(super) struct SaveProgress {
    pub(super) exchange_attempted: Cell<bool>,
}

struct NoteSaveObservations {
    destination: Option<FileObservation>,
    staging: Option<FileObservation>,
    retained: Option<RetainedArtifact>,
}

enum RecoveryDecision {
    NotApplied(Option<FileFingerprint>),
    Applied(NoteVerification),
    MatchingContentUnproven,
    Conflict,
    Uncertain,
}

impl crate::Workspace {
    /// Explicitly publishes a generation-checked snapshot to its original Markdown path.
    /// Repeated operation IDs never repeat filesystem mutations.
    pub fn save_note(&mut self, request: NoteSubmission) -> NoteResult<NoteReceipt> {
        let result = (|| {
            let record = self.store.note_record(request.note_id)?;
            let destination = self
                .store
                .note_write_destination(request.operation_id)?
                .unwrap_or_else(|| record.relative_path.clone());
            if let Some(result) =
                self.store
                    .note_write_result(&request, &destination, NoteWriteKind::Replace)?
            {
                return replay(result);
            }
            if self.store.note_save_intent(request.operation_id)?.is_some() {
                return self.reconcile_note_save(request.operation_id);
            }
            self.store.validate_note_submission(&request)?;
            if let Some(error) = self.store.note_original_save_blocker(request.note_id)? {
                return Err(self.store.record_note_write_failure(
                    &request,
                    &record.relative_path,
                    NoteWriteKind::Replace,
                    &error,
                )?);
            }
            let preflight = self.acquire_note_vault(request.note_id).and_then(|()| {
                let observed = self
                    .notes
                    .vault
                    .as_ref()
                    .unwrap()
                    .1
                    .observe(&record.relative_path)?;
                if observed.fingerprint != record.baseline {
                    return Err(note_failure(
                        NoteErrorCode::Conflict,
                        "saved file differs from editing baseline",
                    ));
                }
                Ok(())
            });
            if let Err(error) = preflight {
                return Err(self.store.record_note_write_failure(
                    &request,
                    &record.relative_path,
                    NoteWriteKind::Replace,
                    &error,
                )?);
            }
            checkpoint("before_intent");
            let recovery = self
                .store
                .note_recovery(request.note_id)?
                .ok_or_else(|| note_failure(NoteErrorCode::Storage, "note has no recovery"))?;
            let intent = match self.store.begin_note_save(
                &request,
                &record.relative_path,
                NoteWriteKind::Replace,
                &DestinationPrecondition::Existing {
                    fingerprint: record.baseline,
                    baseline_text: recovery.baseline,
                },
            ) {
                Ok(intent) => intent,
                Err(error)
                    if matches!(
                        error.code,
                        NoteErrorCode::SaveUncertain | NoteErrorCode::Conflict
                    ) =>
                {
                    return Err(self.store.record_note_write_failure(
                        &request,
                        &record.relative_path,
                        NoteWriteKind::Replace,
                        &error,
                    )?);
                }
                Err(error) => return Err(error),
            };
            checkpoint("intent");
            let files = &self.notes.vault.as_ref().unwrap().1;
            let progress = SaveProgress::default();
            let result = files.coordinate(&intent.destination, || {
                execute_save(&mut self.store, files, intent.clone(), &progress)
            });
            match result {
                Ok(receipt) => {
                    // Cleanup is separate from the save outcome. Failure leaves Pending
                    // bookkeeping and the proven recovery intact for a later cleanup.
                    let _ = self.cleanup_note_artifacts(request.operation_id);
                    if let Ok(intents) = self.store.note_save_intents() {
                        for old in intents {
                            if old.request.note_id == request.note_id
                                && old.request.operation_id != request.operation_id
                                && matches!(
                                    old.resolution,
                                    NoteResolution::Applied | NoteResolution::NotApplied
                                )
                            {
                                let _ = self.cleanup_note_artifacts(old.request.operation_id);
                            }
                        }
                    }
                    checkpoint("before_prune");
                    let _ = self.store.prune_completed_note_payloads(request.note_id);
                    checkpoint("pruned");
                    Ok(receipt)
                }
                Err(error) => self.record_note_save_failure(intent, error, &progress),
            }
        })();
        result.map_err(|mut error: NoteFailure| {
            error.operation_id.get_or_insert(request.operation_id);
            error.note_id.get_or_insert(request.note_id);
            error
        })
    }

    pub(super) fn record_note_save_failure(
        &mut self,
        intent: NoteSaveIntent,
        error: NoteFailure,
        progress: &SaveProgress,
    ) -> NoteResult<NoteReceipt> {
        checkpoint("save_failed");
        let current = self
            .load_note_intent(intent.request.operation_id)
            .map_err(|storage| context(storage, &intent, FileOutcome::Unknown))?;
        if let Some(result) = current.prior_result.clone() {
            return replay(result);
        }
        let files = &self.notes.vault.as_ref().unwrap().1;
        if current.staged.is_none()
            && !matches!(files.artifact(&current.staging_relative), Ok(None))
        {
            self.store
                .record_note_cleanup(
                    current.request.operation_id,
                    ArtifactCleanup::RetainedUnexpected,
                )
                .map_err(|storage| context(storage, &current, FileOutcome::Unknown))?;
        }
        let known_failure = context(error.clone(), &current, FileOutcome::NotApplied);
        let error = context(error, &current, FileOutcome::Unknown);
        // Live progress proves whether exchange was attempted; the journal
        // phase alone is never such proof after a crash or an uncertain exchange.
        if !progress.exchange_attempted.get() {
            // Unjournaled staging affects cleanup, not live execution disproof.
            let destination_proof = match &current.expected_destination {
                DestinationPrecondition::Existing { fingerprint, .. } => files
                    .observe(&current.destination)
                    .ok()
                    .filter(|observed| observed.fingerprint == *fingerprint)
                    .map(|observed| Some(observed.fingerprint)),
                DestinationPrecondition::Absent { .. } => {
                    match files.artifact(&current.destination) {
                        Ok(None) => Some(None),
                        Ok(Some(actual))
                            if current.staged.as_ref().is_none_or(|prepared| {
                                actual.identity.device != prepared.fingerprint.device
                                    || actual.identity.inode != prepared.fingerprint.inode
                            }) =>
                        {
                            Some(None)
                        }
                        _ => None,
                    }
                }
            };
            if let Some(observed_destination) = destination_proof {
                return replay(
                    self.store
                        .reconcile_note_operation(
                            current.request.operation_id,
                            &NoteReconciliation {
                                resolution: NoteResolution::NotApplied,
                                observed_destination,
                                verification: None,
                                result: NoteRecordedResult::Failure(known_failure),
                            },
                        )
                        .map_err(|storage| context(storage, &current, FileOutcome::Unknown))?,
                );
            }
            if current.kind == NoteWriteKind::Replace {
                return Err(self.store.record_note_write_failure(
                    &current.request,
                    &current.destination,
                    current.kind,
                    &known_failure,
                )?);
            }
        }
        if let Some(prepared) = Self::unconsumed_copy_stage(files, &current) {
            return replay(
                self.store
                    .reconcile_note_copy_not_installed(
                        current.request.operation_id,
                        &prepared,
                        &NoteRecordedResult::Failure(known_failure),
                    )
                    .map_err(|storage| context(storage, &current, FileOutcome::Unknown))?,
            );
        }
        Err(self.store.record_note_write_failure(
            &current.request,
            &current.destination,
            current.kind,
            &error,
        )?)
    }

    /// Classifies interrupted writes from exact identity pairs; never writes or removes files.
    pub fn reconcile_note_save(&mut self, op: Uuid) -> NoteResult<NoteReceipt> {
        if let Some(result) = self.store.note_save_result(op)?
            && self.store.note_save_intent(op)?.is_none()
        {
            return replay(result);
        }
        let intent = self.load_note_intent(op)?;
        if let Some(result) = intent.prior_result.clone() {
            if intent.resolution == NoteResolution::Unresolved
                && intent.kind == NoteWriteKind::Copy
                && self.acquire_note_vault(intent.request.note_id).is_ok()
            {
                let files = &self.notes.vault.as_ref().unwrap().1;
                if let Some(prepared) = Self::unconsumed_copy_stage(files, &intent) {
                    self.store
                        .reconcile_note_copy_not_installed(op, &prepared, &result)
                        .map_err(|error| context(error, &intent, FileOutcome::Unknown))?;
                } else if intent.phase == SavePhase::Intent
                    && intent.staged.is_none()
                    && matches!(files.artifact(&intent.destination), Ok(None))
                    && matches!(files.artifact(&intent.staging_relative), Ok(None))
                {
                    self.store
                        .reconcile_note_operation(
                            op,
                            &NoteReconciliation {
                                resolution: NoteResolution::NotApplied,
                                observed_destination: None,
                                verification: None,
                                result: result.clone(),
                            },
                        )
                        .map_err(|error| context(error, &intent, FileOutcome::Unknown))?;
                }
            }
            return replay(result);
        }
        self.acquire_note_vault(intent.request.note_id)
            .map_err(|error| context(error, &intent, FileOutcome::Unknown))?;
        let observations = match self.observe_note_intent(&intent) {
            Ok(observations) => observations,
            Err(error) => {
                let error = context(error, &intent, FileOutcome::Unknown);
                self.store
                    .reconcile_note_operation(
                        op,
                        &NoteReconciliation {
                            resolution: NoteResolution::Unresolved,
                            observed_destination: None,
                            verification: None,
                            result: NoteRecordedResult::Failure(error.clone()),
                        },
                    )
                    .map_err(|storage| context(storage, &intent, FileOutcome::Unknown))?;
                return Err(error);
            }
        };
        let decision = classify_note_save(&intent, &observations)?;
        self.commit_note_reconciliation(intent, decision)
    }

    fn unconsumed_copy_stage(
        files: &MacFiles,
        intent: &NoteSaveIntent,
    ) -> Option<brn_store::notes::PreparedFile> {
        if intent.kind != NoteWriteKind::Copy || intent.phase != SavePhase::Prepared {
            return None;
        }
        let prepared = intent.staged.as_ref()?;
        let destination = files.artifact(&intent.destination).ok()?;
        if destination.is_some_and(|actual| {
            actual.identity.device == prepared.fingerprint.device
                && actual.identity.inode == prepared.fingerprint.inode
        }) {
            return None;
        }
        let stage = files.observe(&intent.staging_relative).ok()?;
        (stage.fingerprint == prepared.fingerprint).then(|| prepared.clone())
    }

    fn load_note_intent(&self, op: Uuid) -> NoteResult<NoteSaveIntent> {
        self.store.note_save_intent(op)?.ok_or_else(|| {
            let mut error =
                note_failure(NoteErrorCode::Missing, "save intent is absent or retired");
            error.operation_id = Some(op);
            error
        })
    }

    fn observe_note_intent(&self, intent: &NoteSaveIntent) -> NoteResult<NoteSaveObservations> {
        let files = &self.notes.vault.as_ref().unwrap().1;
        if let DestinationPrecondition::Absent { parent } = &intent.expected_destination
            && files.parent_identity(&intent.destination)? != *parent
        {
            return Err(note_failure(
                NoteErrorCode::Conflict,
                "copy destination parent changed",
            ));
        }
        observe_intent(files, intent)
    }

    fn commit_note_reconciliation(
        &mut self,
        intent: NoteSaveIntent,
        decision: RecoveryDecision,
    ) -> NoteResult<NoteReceipt> {
        let record = reconciliation(&intent, decision);
        if record.resolution == NoteResolution::NotApplied
            && intent.kind == NoteWriteKind::Copy
            && let Some(prepared) = &intent.staged
        {
            return replay(
                self.store
                    .reconcile_note_copy_not_installed(
                        intent.request.operation_id,
                        prepared,
                        &record.result,
                    )
                    .map_err(|error| context(error, &intent, FileOutcome::Unknown))?,
            );
        }
        replay(
            self.store
                .reconcile_note_operation(intent.request.operation_id, &record)
                .map_err(|error| context(error, &intent, FileOutcome::Unknown))?,
        )
    }

    pub(super) fn cleanup_note_artifacts(&mut self, op: Uuid) -> NoteResult<()> {
        let intent = self.load_note_intent(op)?;
        if intent.cleanup != ArtifactCleanup::Pending {
            return Ok(());
        }
        let Some(candidate) = self.store.note_cleanup_candidate(op)? else {
            if intent.resolution == NoteResolution::NotApplied
                && intent.staged.is_none()
                && matches!(&intent.prior_result, Some(NoteRecordedResult::Receipt(receipt)) if receipt.filesystem_outcome == FileOutcome::NotApplied)
            {
                self.acquire_note_vault(intent.request.note_id)?;
                let files = &self.notes.vault.as_ref().unwrap().1;
                self.store
                    .note_recovery(intent.request.note_id)?
                    .ok_or_else(|| {
                        note_failure(NoteErrorCode::Storage, "retirement requires recovery")
                    })?;
                let cleanup = if files.artifact(&intent.staging_relative)?.is_none() {
                    ArtifactCleanup::Retired
                } else {
                    ArtifactCleanup::RetainedUnexpected
                };
                self.store.record_note_cleanup(op, cleanup)?;
            }
            return Ok(());
        };
        self.acquire_note_vault(intent.request.note_id)?;
        let files = &self.notes.vault.as_ref().unwrap().1;
        checkpoint("cleanup_pending");
        match files.artifact(&candidate.relative)? {
            None => {
                // Absence after an interrupted unlink is metadata-only retirement.
                self.store
                    .record_note_cleanup(op, ArtifactCleanup::Retired)?;
            }
            Some(actual) if actual == candidate => {
                files.remove_artifact(&candidate)?;
                checkpoint("cleanup_unlinked");
                self.store
                    .record_note_cleanup(op, ArtifactCleanup::Retired)?;
            }
            Some(_) => {
                self.store
                    .record_note_cleanup(op, ArtifactCleanup::RetainedUnexpected)?;
            }
        }
        Ok(())
    }
}

fn execute_save(
    store: &mut Store,
    files: &MacFiles,
    mut intent: NoteSaveIntent,
    progress: &SaveProgress,
) -> NoteResult<NoteReceipt> {
    let op = intent.request.operation_id;
    let DestinationPrecondition::Existing {
        fingerprint,
        baseline_text,
    } = &intent.expected_destination
    else {
        return Err(note_failure(
            NoteErrorCode::Unsupported,
            "original save requires existing destination",
        ));
    };
    let fingerprint = fingerprint.clone();
    if baseline_text == &intent.request.text {
        if files
            .observe_uncoordinated(&intent.destination)?
            .fingerprint
            != fingerprint
        {
            return Err(note_failure(
                NoteErrorCode::Conflict,
                "unchanged save baseline changed",
            ));
        }
        let receipt = receipt(&intent, FileOutcome::NotApplied);
        store.finish_note_save(op, &receipt)?;
        checkpoint("receipt");
        return Ok(receipt);
    }
    if files.artifact(&intent.staging_relative)?.is_some() {
        return Err(note_failure(
            NoteErrorCode::Conflict,
            "staging path is already occupied",
        ));
    }
    checkpoint("before_stage");
    let prepared = files.prepare_replace(
        op,
        &intent.staging_relative,
        &intent.destination,
        intent.request.text.as_bytes(),
    )?;
    checkpoint("stage");
    store.record_note_prepared(op, &prepared)?;
    intent = store.note_save_intent(op)?.ok_or_else(|| {
        context(
            note_failure(NoteErrorCode::Storage, "prepared intent disappeared"),
            &intent,
            FileOutcome::NotApplied,
        )
    })?;
    checkpoint("prepared");
    let observed = files.observe_uncoordinated(&intent.destination)?;
    if observed.fingerprint != fingerprint {
        return Err(note_failure(
            NoteErrorCode::Conflict,
            "destination changed before exchange",
        ));
    }
    checkpoint("prechecked");
    // Even an exchange error cannot be treated as execution disproof.
    progress.exchange_attempted.set(true);
    files
        .exchange(&prepared, &intent.destination)
        .map_err(|error| context(error, &intent, FileOutcome::Unknown))?;
    checkpoint("exchange_returned");
    store
        .mark_note_exchanged(op)
        .map_err(|error| context(error, &intent, FileOutcome::Unknown))?;
    intent = store
        .note_save_intent(op)
        .map_err(|error| context(error, &intent, FileOutcome::Unknown))?
        .ok_or_else(|| {
            context(
                note_failure(NoteErrorCode::Storage, "exchanged intent disappeared"),
                &intent,
                FileOutcome::Unknown,
            )
        })?;
    if let Some(displaced) = files
        .artifact(&intent.staging_relative)
        .map_err(|error| context(error, &intent, FileOutcome::Unknown))?
    {
        store
            .record_note_displaced(op, &displaced)
            .map_err(|error| context(error, &intent, FileOutcome::Unknown))?;
    }
    files
        .flush_artifact(&intent.destination)
        .and_then(|()| files.flush_artifact(&intent.staging_relative))
        .map_err(|error| context(error, &intent, FileOutcome::Unknown))?;
    checkpoint("synced");
    let observations = observe_intent_uncoordinated(files, &intent)
        .map_err(|error| context(error, &intent, FileOutcome::Unknown))?;
    let decision = classify_note_save(&intent, &observations)?;
    let RecoveryDecision::Applied(verification) = decision else {
        return Err(context(
            note_failure(
                NoteErrorCode::Conflict,
                "installed/displaced proof differs after exchange",
            ),
            &intent,
            FileOutcome::Unknown,
        ));
    };
    store
        .record_note_verification(op, &verification)
        .map_err(|error| context(error, &intent, FileOutcome::Unknown))?;
    checkpoint("verified");
    let receipt = receipt(&intent, FileOutcome::Applied);
    store
        .finish_note_save(op, &receipt)
        .map_err(|error| context(error, &intent, FileOutcome::Unknown))?;
    checkpoint("receipt");
    Ok(receipt)
}

fn observe_intent(files: &MacFiles, intent: &NoteSaveIntent) -> NoteResult<NoteSaveObservations> {
    files.coordinate(&intent.destination, || {
        let observations = observe_intent_uncoordinated(files, intent)?;
        if intent.kind == NoteWriteKind::Copy
            && matches!(
                classify_note_save(intent, &observations)?,
                RecoveryDecision::Applied(_)
            )
        {
            // A process may have died immediately after exclusive installation,
            // before flushing. Identity proof does not itself establish durability.
            files.flush_artifact(&intent.destination)?;
            let DestinationPrecondition::Absent { parent } = &intent.expected_destination else {
                unreachable!()
            };
            if files.parent_identity(&intent.destination)? != *parent {
                return Err(note_failure(
                    NoteErrorCode::Conflict,
                    "copy destination parent changed",
                ));
            }
            return observe_intent_uncoordinated(files, intent);
        }
        Ok(observations)
    })
}

fn observe_intent_uncoordinated(
    files: &MacFiles,
    intent: &NoteSaveIntent,
) -> NoteResult<NoteSaveObservations> {
    fn optional(files: &MacFiles, relative: &Path) -> NoteResult<Option<FileObservation>> {
        match files.observe_uncoordinated(relative) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.code == NoteErrorCode::Missing => Ok(None),
            Err(error) => Err(error),
        }
    }
    let retained = files.artifact(&intent.staging_relative)?;
    Ok(NoteSaveObservations {
        destination: optional(files, &intent.destination)?,
        staging: optional(files, &intent.staging_relative)?,
        retained,
    })
}

fn classify_note_save(
    intent: &NoteSaveIntent,
    observations: &NoteSaveObservations,
) -> NoteResult<RecoveryDecision> {
    let DestinationPrecondition::Existing {
        fingerprint: original,
        baseline_text,
    } = &intent.expected_destination
    else {
        let destination = observations.destination.as_ref();
        if let (Some(prepared), Some(destination), None) =
            (&intent.staged, destination, &observations.retained)
            && destination.fingerprint == prepared.fingerprint
            && destination.text == intent.request.text
        {
            return Ok(RecoveryDecision::Applied(NoteVerification::Copy {
                installed: destination.fingerprint.clone(),
            }));
        }
        if intent.phase == SavePhase::Prepared
            && let (Some(prepared), Some(stage)) = (&intent.staged, &observations.staging)
            && stage.fingerprint == prepared.fingerprint
            && destination.is_none_or(|value| {
                value.fingerprint.device != prepared.fingerprint.device
                    || value.fingerprint.inode != prepared.fingerprint.inode
            })
        {
            return Ok(RecoveryDecision::NotApplied(None));
        }
        if destination.is_none() && matches!(intent.phase, SavePhase::Intent | SavePhase::Prepared)
        {
            return Ok(
                match (
                    &intent.staged,
                    &observations.staging,
                    &observations.retained,
                ) {
                    (None, None, None) => RecoveryDecision::NotApplied(None),
                    (Some(prepared), Some(stage), Some(_))
                        if stage.fingerprint == prepared.fingerprint =>
                    {
                        RecoveryDecision::NotApplied(None)
                    }
                    _ => RecoveryDecision::Uncertain,
                },
            );
        }
        return Ok(
            if destination.is_some_and(|value| value.text == intent.request.text) {
                RecoveryDecision::MatchingContentUnproven
            } else {
                RecoveryDecision::Conflict
            },
        );
    };
    let destination = observations.destination.as_ref();
    let staging = observations.staging.as_ref();
    if let (Some(prepared), Some(destination), Some(staging), Some(retained)) =
        (&intent.staged, destination, staging, &observations.retained)
        && destination.fingerprint == prepared.fingerprint
        && destination.text == intent.request.text
        && staging.fingerprint == *original
        && staging.text == *baseline_text
        && intent.displaced.as_ref().is_none_or(|old| old == retained)
    {
        return Ok(RecoveryDecision::Applied(NoteVerification::Replace {
            installed: destination.fingerprint.clone(),
            displaced: retained.clone(),
            displaced_bytes: staging.text.as_bytes().to_vec(),
        }));
    }
    if matches!(intent.phase, SavePhase::Intent | SavePhase::Prepared)
        && destination.is_some_and(|value| value.fingerprint == *original)
    {
        match (&intent.staged, staging, &observations.retained) {
            (None, None, None) => return Ok(RecoveryDecision::NotApplied(Some(original.clone()))),
            (Some(prepared), Some(stage), Some(_)) if stage.fingerprint == prepared.fingerprint => {
                return Ok(RecoveryDecision::NotApplied(Some(original.clone())));
            }
            // A synced but unjournaled stage remains unproven even with expected bytes.
            _ => return Ok(RecoveryDecision::Uncertain),
        }
    }
    if destination.is_some_and(|value| value.text == intent.request.text) {
        return Ok(RecoveryDecision::MatchingContentUnproven);
    }
    Ok(RecoveryDecision::Conflict)
}

fn reconciliation(intent: &NoteSaveIntent, decision: RecoveryDecision) -> NoteReconciliation {
    match decision {
        RecoveryDecision::Applied(verification) => {
            let installed = match &verification {
                NoteVerification::Replace { installed, .. }
                | NoteVerification::Copy { installed } => installed,
            };
            NoteReconciliation {
                resolution: NoteResolution::Applied,
                observed_destination: Some(installed.clone()),
                verification: Some(verification),
                result: NoteRecordedResult::Receipt(receipt(intent, FileOutcome::Applied)),
            }
        }
        RecoveryDecision::NotApplied(original) => NoteReconciliation {
            resolution: NoteResolution::NotApplied,
            observed_destination: original,
            verification: None,
            result: if intent.kind == NoteWriteKind::Copy {
                NoteRecordedResult::Failure(context(
                    note_failure(
                        NoteErrorCode::Conflict,
                        "copy was not installed; input remains protected",
                    ),
                    intent,
                    FileOutcome::NotApplied,
                ))
            } else {
                NoteRecordedResult::Receipt(receipt(intent, FileOutcome::NotApplied))
            },
        },
        decision => {
            let (code, message) = match decision {
                RecoveryDecision::MatchingContentUnproven => (
                    NoteErrorCode::SaveUncertain,
                    "matching content lacks execution identity proof",
                ),
                RecoveryDecision::Conflict => (
                    NoteErrorCode::Conflict,
                    "destination or artifact differs from save intent",
                ),
                _ => (
                    NoteErrorCode::SaveUncertain,
                    "save artifact lacks prepared execution proof",
                ),
            };
            NoteReconciliation {
                resolution: NoteResolution::Unresolved,
                observed_destination: None,
                verification: None,
                result: NoteRecordedResult::Failure(context(
                    note_failure(code, message),
                    intent,
                    FileOutcome::Unknown,
                )),
            }
        }
    }
}

pub(super) fn receipt(intent: &NoteSaveIntent, outcome: FileOutcome) -> NoteReceipt {
    NoteReceipt {
        operation_id: intent.request.operation_id,
        source_note_id: intent.request.note_id,
        note_id: intent.target_note_id,
        submitted_generation: intent.request.generation,
        stamp: NoteStamp {
            file_state: if outcome == FileOutcome::Applied {
                Uuid::new_v4()
            } else {
                intent.request.expected.file_state
            },
            generation: intent.request.generation,
        },
        filesystem_outcome: outcome,
        recovery_available: true,
    }
}

pub(super) fn replay(result: NoteRecordedResult) -> NoteResult<NoteReceipt> {
    match result {
        NoteRecordedResult::Receipt(receipt) => Ok(receipt),
        NoteRecordedResult::Failure(error) => Err(error),
    }
}

pub(super) fn context(
    mut error: NoteFailure,
    intent: &NoteSaveIntent,
    outcome: FileOutcome,
) -> NoteFailure {
    if outcome == FileOutcome::Unknown
        && matches!(
            error.code,
            NoteErrorCode::Missing | NoteErrorCode::Unsupported | NoteErrorCode::Io
        )
    {
        error.code = NoteErrorCode::SaveUncertain;
        error.message = format!("filesystem outcome is uncertain: {}", error.message);
    }
    error.operation_id = Some(intent.request.operation_id);
    error.note_id = Some(intent.request.note_id);
    error.phase = Some(intent.phase);
    error.filesystem_outcome = outcome;
    error.recovery_available = true;
    error
}

pub(super) fn checkpoint(phase: &str) {
    #[cfg(test)]
    TEST_HOOK.with(|hook| {
        if let Some(hook) = &*hook.borrow() {
            hook(phase);
        }
    });
    #[cfg(not(test))]
    let _ = phase;
}

#[cfg(test)]
type PhaseHook = Box<dyn Fn(&str)>;
#[cfg(test)]
thread_local! {
    pub(super) static TEST_HOOK: std::cell::RefCell<Option<PhaseHook>> = const { std::cell::RefCell::new(None) };
}
