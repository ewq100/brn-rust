use super::{
    files::MacFiles,
    save::{self, SaveProgress},
    *,
};
use brn_store::{
    Store,
    notes::{
        DestinationPrecondition, NoteDecision, NoteSaveIntent, NoteVerification, NoteWriteKind,
    },
};

impl crate::Workspace {
    /// Observes disk without rebasing, merging, publishing or discarding recovery.
    pub fn compare_note(&mut self, id: Uuid) -> NoteResult<NoteComparison> {
        let view = self.note(id)?;
        let recovery = self
            .store
            .note_recovery(id)?
            .ok_or_else(|| note_failure(NoteErrorCode::Storage, "note lacks protected baseline"))?;
        Ok(NoteComparison {
            note_id: id,
            stamp: recovery.stamp,
            baseline: recovery.baseline,
            working: recovery.working,
            observed: view.saved,
            observed_file_state: view.current_file_state,
            availability: view.availability,
        })
    }

    pub fn reload_note(
        &mut self,
        op: Uuid,
        id: Uuid,
        expected: NoteStamp,
        discard: bool,
    ) -> NoteResult<NoteView> {
        let decision = NoteDecision::Reload {
            note_id: id,
            expected,
            discard,
        };
        self.apply_note_decision(op, id, decision, None)
    }

    /// Identity adoption is explicit. Relinking retains the local text/generation.
    pub fn relink_note(
        &mut self,
        op: Uuid,
        id: Uuid,
        expected: NoteStamp,
        relative: &Path,
        confirm_identity: bool,
    ) -> NoteResult<NoteView> {
        let decision = NoteDecision::Relink {
            note_id: id,
            expected,
            relative: relative.to_owned(),
            confirm_identity,
        };
        self.apply_note_decision(op, id, decision, Some(relative))
    }

    fn apply_note_decision(
        &mut self,
        op: Uuid,
        id: Uuid,
        decision: NoteDecision,
        relative: Option<&Path>,
    ) -> NoteResult<NoteView> {
        let result = (|| {
            if let Some(id) = self.store.note_decision_replay(op, &decision)? {
                return self.note(id);
            }
            let record = self.store.note_record(id)?;
            let (expected, confirmed) = match &decision {
                NoteDecision::Reload { expected, .. } => (*expected, true),
                NoteDecision::Relink {
                    expected,
                    confirm_identity,
                    ..
                } => (*expected, *confirm_identity),
                _ => unreachable!(),
            };
            if record.stamp != expected {
                return Err(note_failure(
                    NoteErrorCode::StateChanged,
                    "note buffer stamp changed",
                ));
            }
            if !confirmed {
                return Err(note_failure(
                    NoteErrorCode::Conflict,
                    "relink requires explicit identity confirmation",
                ));
            }
            let path = relative.unwrap_or(&record.relative_path);
            validate_note_path(path)?;
            self.acquire_note_vault(id)?;
            if relative.is_some() && self.registry_path_is_reserved_except(path, Some(id))? {
                return Err(note_copy_conflict(
                    "relink destination is registered or reserved",
                ));
            }
            let files = &self.notes.vault.as_ref().unwrap().1;
            files.coordinate(path, || {
                let observed = files.observe_uncoordinated(path)?;
                self.store.record_note_decision(
                    op,
                    &decision,
                    &observed.fingerprint,
                    &observed.text,
                )?;
                Ok(())
            })?;
            self.note(id)
        })();
        result.map_err(|mut error: NoteFailure| {
            error.operation_id.get_or_insert(op);
            error.note_id.get_or_insert(id);
            error
        })
    }

    /// Metadata-only resolution of a reviewed uncertain/late-conflict original save.
    /// The old result and artifacts remain protected; this is not a save receipt.
    pub fn accept_note_disk_state(
        &mut self,
        ack_op: Uuid,
        save_op: Uuid,
        observed_file_state: Uuid,
    ) -> NoteResult<NoteView> {
        let decision = NoteDecision::AcceptCurrent {
            save_operation_id: save_op,
            observed_file_state,
        };
        let result = (|| {
            if let Some(id) = self.store.note_decision_replay(ack_op, &decision)? {
                return self.note(id);
            }
            let intent = self.store.note_save_intent(save_op)?.ok_or_else(|| {
                note_failure(NoteErrorCode::Missing, "original save intent is absent")
            })?;
            self.acquire_note_vault(intent.request.note_id)?;
            let record = self.store.note_record(intent.request.note_id)?;
            let files = &self.notes.vault.as_ref().unwrap().1;
            files.coordinate(&record.relative_path, || {
                let observed = files.observe_uncoordinated(&record.relative_path)?;
                self.store.accept_note_disk_state(
                    ack_op,
                    save_op,
                    observed_file_state,
                    &observed.fingerprint,
                    &observed.text,
                )?;
                Ok(())
            })?;
            self.note(intent.request.note_id)
        })();
        result.map_err(|mut error: NoteFailure| {
            error.operation_id.get_or_insert(ack_op);
            error
        })
    }

    pub(super) fn registry_path_is_reserved(&self, relative: &Path) -> NoteResult<bool> {
        self.registry_path_is_reserved_except(relative, None)
    }

    fn registry_path_is_reserved_except(
        &self,
        relative: &Path,
        except: Option<Uuid>,
    ) -> NoteResult<bool> {
        let files = &self.notes.vault.as_ref().unwrap().1;
        for recovered in self.store.note_recoveries()? {
            if Some(recovered.note_id) == except {
                continue;
            }
            let record = self.store.note_record(recovered.note_id)?;
            if files.aliases_original(relative, &record.relative_path)? {
                return Ok(true);
            }
        }
        for intent in self.store.note_save_intents()? {
            if intent.kind == NoteWriteKind::Copy
                && matches!(
                    intent.resolution,
                    NoteResolution::Unresolved
                        | NoteResolution::Applied
                        | NoteResolution::AcceptedCurrent
                )
                && files.aliases_original(relative, &intent.destination)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Saves an independent unapproved note using exclusive atomic installation.
    /// An unresolved original write is neither resolved nor replayed by this copy.
    pub fn save_note_copy(
        &mut self,
        request: NoteSubmission,
        relative: &Path,
    ) -> NoteResult<NoteReceipt> {
        let result = (|| {
            if let Some(result) =
                self.store
                    .note_write_result(&request, relative, NoteWriteKind::Copy)?
            {
                return save::replay(result);
            }
            if self.store.note_save_intent(request.operation_id)?.is_some() {
                return self.reconcile_note_save(request.operation_id);
            }
            self.store.validate_note_submission(&request)?;
            validate_note_path(relative)?;
            let preflight = (|| {
                self.acquire_note_vault(request.note_id)?;
                let record = self.store.note_record(request.note_id)?;
                let files = &self.notes.vault.as_ref().unwrap().1;
                if files.artifact(relative)?.is_some() {
                    return Err(note_copy_conflict(
                        "copy destination is occupied; no overwrite is available",
                    ));
                }
                if files.aliases_original(relative, &record.relative_path)?
                    || self.registry_path_is_reserved(relative)?
                {
                    return Err(note_copy_conflict(
                        "copy needs a distinct unregistered path",
                    ));
                }
                files.parent_identity(relative)
            })();
            let parent = match preflight {
                Ok(parent) => parent,
                Err(error) => {
                    return Err(self.store.record_note_write_failure(
                        &request,
                        relative,
                        NoteWriteKind::Copy,
                        &error,
                    )?);
                }
            };
            save::checkpoint("before_intent");
            let intent = match self.store.begin_note_save(
                &request,
                relative,
                NoteWriteKind::Copy,
                &DestinationPrecondition::Absent { parent },
            ) {
                Ok(intent) => intent,
                Err(error) if error.code == NoteErrorCode::Conflict => {
                    return Err(self.store.record_note_write_failure(
                        &request,
                        relative,
                        NoteWriteKind::Copy,
                        &error,
                    )?);
                }
                Err(error) => return Err(error),
            };
            save::checkpoint("intent");
            let files = &self.notes.vault.as_ref().unwrap().1;
            let progress = SaveProgress::default();
            let result = files.coordinate(relative, || {
                execute_copy(&mut self.store, files, intent.clone(), &progress)
            });
            match result {
                Ok(receipt) => Ok(receipt),
                Err(error) => self.record_note_save_failure(intent, error, &progress),
            }
        })();
        result.map_err(|mut error: NoteFailure| {
            error.operation_id.get_or_insert(request.operation_id);
            error.note_id.get_or_insert(request.note_id);
            error
        })
    }
}

fn note_copy_conflict(message: &str) -> NoteFailure {
    note_failure(NoteErrorCode::Conflict, message)
}

fn execute_copy(
    store: &mut Store,
    files: &MacFiles,
    mut intent: NoteSaveIntent,
    progress: &SaveProgress,
) -> NoteResult<NoteReceipt> {
    let op = intent.request.operation_id;
    let DestinationPrecondition::Absent { parent } = &intent.expected_destination else {
        return Err(note_failure(
            NoteErrorCode::Unsupported,
            "copy requires an absent destination",
        ));
    };
    let parent = parent.clone();
    if files.parent_identity(&intent.destination)? != parent {
        return Err(note_copy_conflict("copy parent changed"));
    }
    if files.artifact(&intent.staging_relative)?.is_some() {
        return Err(note_copy_conflict("copy staging path is occupied"));
    }
    save::checkpoint("before_stage");
    progress.staging_attempted.set(true);
    let prepared = files.prepare_copy(
        op,
        &intent.staging_relative,
        &intent.destination,
        intent.request.text.as_bytes(),
    )?;
    save::checkpoint("stage");
    store.record_note_prepared(op, &prepared)?;
    intent = store
        .note_save_intent(op)?
        .ok_or_else(|| note_failure(NoteErrorCode::Storage, "copy intent disappeared"))?;
    save::checkpoint("prepared");
    if files.parent_identity(&intent.destination)? != parent {
        return Err(note_copy_conflict(
            "copy parent changed before installation",
        ));
    }
    save::checkpoint("prechecked");
    progress.exchange_attempted.set(true);
    files.install_exclusive(&prepared, &intent.destination)?;
    save::checkpoint("exchange_returned");
    store.mark_note_exchanged(op)?;
    intent = store
        .note_save_intent(op)?
        .ok_or_else(|| note_failure(NoteErrorCode::Storage, "installed copy intent disappeared"))?;
    files.flush_artifact(&intent.destination)?;
    save::checkpoint("synced");
    let observed = files.observe_uncoordinated(&intent.destination)?;
    if observed.fingerprint != prepared.fingerprint
        || observed.text != intent.request.text
        || files.artifact(&intent.staging_relative)?.is_some()
        || files.parent_identity(&intent.destination)? != parent
    {
        return Err(note_copy_conflict(
            "copy installed identity or consumed staging proof changed",
        ));
    }
    store.record_note_verification(
        op,
        &NoteVerification::Copy {
            installed: observed.fingerprint,
        },
    )?;
    save::checkpoint("verified");
    let receipt = save::receipt(&intent, FileOutcome::Applied);
    store.finish_note_save(op, &receipt)?;
    save::checkpoint("receipt");
    Ok(receipt)
}
