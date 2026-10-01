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
struct SaveProgress {
    staging_attempted: Cell<bool>,
    exchange_attempted: Cell<bool>,
}

struct NoteSaveObservations {
    destination: Option<FileObservation>,
    staging: Option<FileObservation>,
    retained: Option<RetainedArtifact>,
}

enum RecoveryDecision {
    NotApplied(FileFingerprint),
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
            if let Some(result) = self.store.note_write_result(
                &request,
                &record.relative_path,
                NoteWriteKind::Replace,
            )? {
                return replay(result);
            }
            if self.store.note_save_intent(request.operation_id)?.is_some() {
                return self.reconcile_note_save(request.operation_id);
            }
            self.store.validate_note_submission(&request)?;
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
                Err(error) if error.code == NoteErrorCode::SaveUncertain => {
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

    fn record_note_save_failure(
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
        let mut error = context(error, &current, FileOutcome::Unknown);
        // Live progress proves whether creation/exchange was attempted; the journal
        // phase alone is never such proof after a crash or an uncertain exchange.
        if !progress.exchange_attempted.get() {
            let original = if !progress.staging_attempted.get() {
                files
                    .observe(&current.destination)
                    .ok()
                    .and_then(|observed| match &current.expected_destination {
                        DestinationPrecondition::Existing { fingerprint, .. }
                            if observed.fingerprint == *fingerprint =>
                        {
                            Some(observed.fingerprint)
                        }
                        _ => None,
                    })
            } else {
                self.observe_note_intent(&current)
                    .ok()
                    .and_then(|observations| classify_note_save(&current, &observations).ok())
                    .and_then(|decision| match decision {
                        RecoveryDecision::NotApplied(original) => Some(original),
                        _ => None,
                    })
            };
            if let Some(original) = original {
                return replay(
                    self.store
                        .reconcile_note_operation(
                            current.request.operation_id,
                            &NoteReconciliation {
                                resolution: NoteResolution::NotApplied,
                                observed_destination: Some(original),
                                verification: None,
                                result: NoteRecordedResult::Failure(known_failure),
                            },
                        )
                        .map_err(|storage| context(storage, &current, FileOutcome::Unknown))?,
                );
            }
            error.code = NoteErrorCode::SaveUncertain;
            error.message = format!(
                "pre-exchange refusal lacks not-applied proof: {}",
                error.message
            );
        }
        Err(self.store.record_note_write_failure(
            &current.request,
            &current.destination,
            NoteWriteKind::Replace,
            &error,
        )?)
    }

    /// Classifies interrupted writes from exact identity pairs; never writes or removes files.
    pub fn reconcile_note_save(&mut self, op: Uuid) -> NoteResult<NoteReceipt> {
        if let Some(result) = self.store.note_save_result(op)? {
            return replay(result);
        }
        let intent = self.load_note_intent(op)?;
        if let Some(result) = intent.prior_result.clone() {
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
        observe_intent(files, intent)
    }

    fn commit_note_reconciliation(
        &mut self,
        intent: NoteSaveIntent,
        decision: RecoveryDecision,
    ) -> NoteResult<NoteReceipt> {
        let record = reconciliation(&intent, decision);
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
    progress.staging_attempted.set(true);
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
        observe_intent_uncoordinated(files, intent)
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
        return Err(note_failure(
            NoteErrorCode::Unsupported,
            "copy reconciliation belongs to save-copy",
        ));
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
            (None, None, None) => return Ok(RecoveryDecision::NotApplied(original.clone())),
            (Some(prepared), Some(stage), Some(_)) if stage.fingerprint == prepared.fingerprint => {
                return Ok(RecoveryDecision::NotApplied(original.clone()));
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
            let NoteVerification::Replace { installed, .. } = &verification else {
                unreachable!()
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
            observed_destination: Some(original),
            verification: None,
            result: NoteRecordedResult::Receipt(receipt(intent, FileOutcome::NotApplied)),
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

fn receipt(intent: &NoteSaveIntent, outcome: FileOutcome) -> NoteReceipt {
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

fn replay(result: NoteRecordedResult) -> NoteResult<NoteReceipt> {
    match result {
        NoteRecordedResult::Receipt(receipt) => Ok(receipt),
        NoteRecordedResult::Failure(error) => Err(error),
    }
}

fn context(mut error: NoteFailure, intent: &NoteSaveIntent, outcome: FileOutcome) -> NoteFailure {
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
