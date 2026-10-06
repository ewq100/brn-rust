//! Explicit repair of an exact observed, already-approved operation.
use super::*;
use brn_store::work::proposal_apply::{
    ApplyMemberPhase, RepairDirection, RepairPreview, RepairReceipt, RepairRequest,
};

pub fn validate_repair_request(request: &RepairRequest) -> Result<()> {
    if request.id.is_nil() || request.operation_id.is_nil() || request.id == request.operation_id {
        return Err(WorkflowError::typed(
            ErrorKind::ToolRejected,
            "repair requires distinct non-nil attempt and operation UUIDs",
        ));
    }
    Ok(())
}

pub(super) fn check_phase_sources(
    files: &MacFiles,
    journal: &ApplyJournal,
    phases: &[ApplyMemberPhase],
) -> Result<()> {
    for source in &journal.approved.draft.sources {
        let own = journal
            .approved
            .draft
            .changes
            .iter()
            .enumerate()
            .find(|(_, change)| original(change) == Some(&source.fingerprint));
        let expected = match own {
            Some((i, NoteChange::Trash { .. } | NoteChange::TrashAsset { .. }))
                if phases[i] == ApplyMemberPhase::Applied =>
            {
                None
            }
            Some((i, _)) if phases[i] == ApplyMemberPhase::Applied => {
                journal.prepared.as_ref().and_then(|proofs| proofs.get(i))
            }
            _ => Some(&source.fingerprint),
        };
        if observed(files, Path::new(&source.path))?.as_ref() != expected {
            return Err(stale("reviewed proposal source changed"));
        }
    }
    Ok(())
}

impl App {
    /// Exact current file phases and the complete approved proposal, without effects.
    pub fn preview_proposal_repair(&mut self, operation: Uuid) -> Result<RepairPreview> {
        if operation.is_nil() {
            return Err(WorkflowError::typed(
                ErrorKind::ToolRejected,
                "repair operation UUID must be non-nil",
            ));
        }
        let journal = self.store.proposal_apply(operation)?.ok_or_else(|| {
            WorkflowError::typed(ErrorKind::NotFound, "repair operation does not exist")
        })?;
        if !unsettled(&journal) {
            return Err(stale("repair requires an unresolved operation"));
        }
        self.editor_files()?;
        self.check_repair_vault(&journal)?;
        Ok(journal.repair_preview(&self.observe_proposal(&journal)?)?)
    }

    /// A new explicit human attempt. Repeating its UUID never repeats namespace writes.
    pub fn repair_proposal(&mut self, request: &RepairRequest) -> Result<RepairReceipt> {
        validate_repair_request(request)?;
        if self.store.proposal_repair(request.id)?.is_some() {
            // Store verifies the whole immutable request before returning history.
            self.store.begin_proposal_repair(request, &[])?;
            return Ok(self
                .store
                .proposal_repair(request.id)?
                .expect("checked repair"));
        }
        let journal = self
            .store
            .proposal_apply(request.operation_id)?
            .ok_or_else(|| {
                WorkflowError::typed(ErrorKind::NotFound, "repair operation does not exist")
            })?;
        if !unsettled(&journal) {
            return Err(stale("repair requires an unresolved operation"));
        }
        self.editor_files()?;
        self.check_repair_vault(&journal)?;
        let observations = self.observe_proposal(&journal)?;
        let preview = journal.repair_preview(&observations)?;
        if preview.expected != request.expected {
            return Err(stale("repair files or history changed after preview"));
        }
        if self.store.editor_saves()?.iter().any(|save| {
            save.receipt
                .as_ref()
                .is_none_or(|receipt| receipt.outcome == crate::editor::SaveOutcome::Uncertain)
        }) {
            return Err(WorkflowError::typed(
                ErrorKind::SaveUncertain,
                "reconcile interrupted manual Save before repairing a proposal",
            ));
        }
        self.check_repair_editors(&journal)?;
        if request.direction == RepairDirection::Finish {
            self.check_approved_actions(&journal)?;
            check_phase_sources(
                self.editor.files.as_ref().expect("opened files"),
                &journal,
                &preview.phases,
            )?;
        }
        self.application_records()?;
        self.set_current_tool_barrier(true);
        let journal = self.store.begin_proposal_repair(request, &observations)?;
        checkpoint("repair-intent", 0);
        let result = self
            .execute_proposal_repair(&journal, request.direction, preview.phases)
            .and_then(|proofs| {
                if request.direction == RepairDirection::Finish {
                    self.check_applied_eligibility(&journal)?;
                }
                Ok(proofs)
            });
        match result {
            Ok(proofs) => {
                let outcome = match request.direction {
                    RepairDirection::Finish => ApplyOutcome::Applied,
                    RepairDirection::Restore => ApplyOutcome::NotApplied,
                };
                self.complete_proposal(&journal, outcome, Some(proofs), false)?;
                Ok(self
                    .store
                    .proposal_repair(request.id)?
                    .expect("admitted repair"))
            }
            Err(error) => {
                self.interrupt_repair(&journal)?;
                Err(error)
            }
        }
    }

    fn check_repair_vault(&self, journal: &ApplyJournal) -> Result<()> {
        let bound: VaultRecord = serde_json::from_str(
            &self
                .store
                .setting("vault.editor_identity")?
                .ok_or_else(|| stale("vault identity is missing"))?,
        )
        .map_err(|_| stale("vault identity is invalid"))?;
        if journal.approved.draft.vault.as_ref() != Some(&bound) {
            return Err(stale("repair vault identity changed"));
        }
        Ok(())
    }

    fn check_repair_editors(&self, journal: &ApplyJournal) -> Result<()> {
        let files = self.editor.files.as_ref().expect("opened files");
        let prepared = journal
            .prepared
            .as_ref()
            .expect("checked complete preparation");
        let editors = self.store.editors()?;
        for (change, new) in journal.approved.draft.changes.iter().zip(prepared) {
            for editor in &editors {
                let identity_matches = |proof: &FileFingerprint| {
                    proof.device == editor.baseline.device && proof.inode == editor.baseline.inode
                };
                let matches = identity_matches(new)
                    || original(change).is_some_and(identity_matches)
                    || files
                        .reserved_copy_path_matches(
                            Path::new(change.path()),
                            Path::new(&editor.path),
                        )
                        .map_err(file_error)?;
                let known = editor.baseline == *new || original(change) == Some(&editor.baseline);
                if matches && (editor.text != editor.baseline_text || !known) {
                    return Err(stale(
                        "repair conflicts with retained editor work; preserve or explicitly reload it first",
                    ));
                }
            }
        }
        Ok(())
    }

    fn execute_proposal_repair(
        &self,
        journal: &ApplyJournal,
        direction: RepairDirection,
        mut phases: Vec<ApplyMemberPhase>,
    ) -> Result<Vec<ApplyMemberProof>> {
        let records = self.apply_records.as_ref().expect("opened recovery files");
        super::retain_inbox_capture(records, &self.store, journal)?;
        let previous = records
            .read(journal.request.operation_id)
            .map_err(file_error)?;
        records
            .write(journal, previous.as_ref())
            .map_err(file_error)?;
        checkpoint("repair-mirror", 0);
        if direction == RepairDirection::Finish {
            self.check_approved_actions(journal)?;
        }
        let files = self.editor.files.as_ref().expect("opened files");
        let desired = match direction {
            RepairDirection::Finish => ApplyMemberPhase::Applied,
            RepairDirection::Restore => ApplyMemberPhase::Before,
        };
        let prepared = journal
            .prepared
            .as_ref()
            .expect("checked complete preparation");
        for (i, (change, member)) in journal
            .approved
            .draft
            .changes
            .iter()
            .zip(&journal.members)
            .enumerate()
        {
            if direction == RepairDirection::Finish {
                self.check_approved_actions(journal)?;
                check_phase_sources(files, journal, &phases)?;
            }
            let destination = Path::new(change.path());
            files
                .coordinate(destination, || {
                    Ok((|| -> Result<()> {
                        if files.parent_identity(destination).map_err(file_error)?
                            != *parent(change)
                        {
                            return Err(stale("repair member parent changed"));
                        }
                        let proof = ApplyMemberProof {
                            destination: observed_member(files, destination, change)?,
                            staging: observed_member(files, &member.staging, change)?,
                        };
                        let mut current = journal
                            .repair
                            .as_ref()
                            .expect("admitted repair")
                            .observations
                            .clone();
                        current[i] = proof;
                        if journal.repair_preview(&current)?.phases[i] != phases[i] {
                            return Err(stale("repair member changed after capture"));
                        }
                        if phases[i] != desired {
                            let staged = PreparedFile {
                                relative: member.staging.clone(),
                                fingerprint: prepared[i].clone(),
                            };
                            match (direction, change) {
                                (
                                    RepairDirection::Finish,
                                    NoteChange::Create { .. } | NoteChange::CreateAsset { .. },
                                ) => install_member(files, &staged, destination, change),
                                (
                                    RepairDirection::Finish,
                                    NoteChange::Replace { .. } | NoteChange::ReplaceAsset { .. },
                                ) => exchange_member(files, &staged, destination, change),
                                (
                                    RepairDirection::Finish,
                                    NoteChange::Trash { before, .. }
                                    | NoteChange::TrashAsset { before, .. },
                                ) => install_member(
                                    files,
                                    &PreparedFile {
                                        relative: destination.to_owned(),
                                        fingerprint: before.clone(),
                                    },
                                    &member.staging,
                                    change,
                                ),
                                (
                                    RepairDirection::Restore,
                                    NoteChange::Create { .. } | NoteChange::CreateAsset { .. },
                                ) => install_member(
                                    files,
                                    &PreparedFile {
                                        relative: destination.to_owned(),
                                        fingerprint: prepared[i].clone(),
                                    },
                                    &member.staging,
                                    change,
                                ),
                                (
                                    RepairDirection::Restore,
                                    NoteChange::Replace { before, .. }
                                    | NoteChange::ReplaceAsset { before, .. },
                                ) => exchange_member(
                                    files,
                                    &PreparedFile {
                                        relative: member.staging.clone(),
                                        fingerprint: before.clone(),
                                    },
                                    destination,
                                    change,
                                ),
                                (
                                    RepairDirection::Restore,
                                    NoteChange::Trash { before, .. }
                                    | NoteChange::TrashAsset { before, .. },
                                ) => install_member(
                                    files,
                                    &PreparedFile {
                                        relative: member.staging.clone(),
                                        fingerprint: before.clone(),
                                    },
                                    destination,
                                    change,
                                ),
                            }
                            .map_err(file_error)?;
                            checkpoint("repair-member", i);
                            phases[i] = desired;
                        }
                        // Flush skipped members too: endpoint proof alone is not durability.
                        for path in [destination, member.staging.as_path()] {
                            if observed_member(files, path, change)?.is_some() {
                                files.flush_artifact(path).map_err(file_error)?;
                            }
                        }
                        checkpoint("repair-synced", i);
                        let proof = ApplyMemberProof {
                            destination: observed_member(files, destination, change)?,
                            staging: observed_member(files, &member.staging, change)?,
                        };
                        current[i] = proof;
                        if journal.repair_preview(&current)?.phases[i] != desired {
                            return Err(stale("repair member changed during installation"));
                        }
                        Ok(())
                    })())
                })
                .map_err(file_error)??;
        }
        if direction == RepairDirection::Finish {
            check_phase_sources(files, journal, &phases)?;
        }
        let observations = self.observe_proposal(journal)?;
        if journal
            .repair_preview(&observations)?
            .phases
            .iter()
            .any(|phase| *phase != desired)
        {
            return Err(stale("repair endpoint changed before settlement"));
        }
        if direction == RepairDirection::Finish {
            check_phase_sources(files, journal, &phases)?;
        }
        checkpoint("repair-verified", 0);
        Ok(observations)
    }

    pub(super) fn interrupt_repair(&mut self, journal: &ApplyJournal) -> Result<()> {
        if journal.receipt.is_none() {
            self.complete_proposal(
                journal,
                ApplyOutcome::Uncertain,
                self.observe_proposal(journal).ok(),
                false,
            )?;
            return Ok(());
        }
        let mut candidate = journal.clone();
        let attempt = candidate
            .repair
            .as_mut()
            .expect("admitted repair")
            .attempts
            .last_mut()
            .expect("repair attempt");
        attempt.outcome = Some(ApplyOutcome::Uncertain);
        let id = attempt.request.id;
        candidate.validate()?;
        self.application_records()?;
        let records = self.apply_records.as_ref().expect("opened recovery files");
        super::retain_inbox_capture(records, &self.store, &candidate)?;
        let previous = records
            .read(journal.request.operation_id)
            .map_err(file_error)?;
        records
            .write(&candidate, previous.as_ref())
            .map_err(file_error)?;
        self.store.interrupt_proposal_repair(id)?;
        self.synchronize_current_barrier()?;
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::super::tests::Fixture;
    use super::*;
    use crate::editor::EditRequest;
    use std::{fs, process::Command};

    fn arrange(mask: usize) -> (Fixture, App, ApprovalRequest) {
        let fixture = Fixture::new();
        let approval = fixture.prepare();
        fixture.crash("member", 0);
        let mut app = fixture.app();
        let journal = app
            .store
            .proposal_apply(approval.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            journal.receipt.as_ref().unwrap().outcome,
            ApplyOutcome::Uncertain
        );
        let phases = journal
            .repair_preview(&app.observe_proposal(&journal).unwrap())
            .unwrap()
            .phases;
        let files = app.editor_files().unwrap();
        for (i, ((change, member), new)) in journal
            .approved
            .draft
            .changes
            .iter()
            .zip(&journal.members)
            .zip(journal.prepared.as_ref().unwrap())
            .enumerate()
        {
            let applied = mask & (1 << i) != 0;
            if applied == (phases[i] == ApplyMemberPhase::Applied) {
                continue;
            }
            let destination = Path::new(change.path());
            let staged = PreparedFile {
                relative: member.staging.clone(),
                fingerprint: new.clone(),
            };
            match (applied, change) {
                (true, NoteChange::Create { .. }) => files.install_exclusive(&staged, destination),
                (true, NoteChange::Replace { .. }) => files.exchange(&staged, destination),
                (true, NoteChange::Trash { before, .. }) => files.install_exclusive(
                    &PreparedFile {
                        relative: destination.to_owned(),
                        fingerprint: before.clone(),
                    },
                    &member.staging,
                ),
                (false, NoteChange::Create { .. }) => files.install_exclusive(
                    &PreparedFile {
                        relative: destination.to_owned(),
                        fingerprint: new.clone(),
                    },
                    &member.staging,
                ),
                (false, NoteChange::Replace { before, .. }) => files.exchange(
                    &PreparedFile {
                        relative: member.staging.clone(),
                        fingerprint: before.clone(),
                    },
                    destination,
                ),
                (false, NoteChange::Trash { before, .. }) => files.install_exclusive(
                    &PreparedFile {
                        relative: member.staging.clone(),
                        fingerprint: before.clone(),
                    },
                    destination,
                ),
                (
                    _,
                    NoteChange::CreateAsset { .. }
                    | NoteChange::ReplaceAsset { .. }
                    | NoteChange::TrashAsset { .. },
                ) => unreachable!("Markdown-only phase fixture"),
            }
            .unwrap();
            for path in [destination, member.staging.as_path()] {
                if observed(files, path).unwrap().is_some() {
                    files.flush_artifact(path).unwrap();
                }
            }
        }
        (fixture, app, approval)
    }

    fn request(app: &mut App, operation: Uuid, direction: RepairDirection) -> RepairRequest {
        let preview = app.preview_proposal_repair(operation).unwrap();
        RepairRequest {
            id: Uuid::new_v4(),
            operation_id: operation,
            expected: preview.expected,
            direction,
        }
    }

    #[test]
    fn all_phase_subsets_finish_or_restore_exact_files_without_rebasing_editors() {
        for mask in 0..8 {
            for direction in [RepairDirection::Finish, RepairDirection::Restore] {
                let (fixture, mut app, approval) = arrange(mask);
                let before = app.store.editor("a.md").unwrap().unwrap();
                let journal = app
                    .store
                    .proposal_apply(approval.operation_id)
                    .unwrap()
                    .unwrap();
                let review = app.proposal(approval.expected.id).unwrap();
                let request = request(&mut app, approval.operation_id, direction);
                assert_eq!(app.proposal(approval.expected.id).unwrap(), review);
                let receipt = app.repair_proposal(&request).unwrap();
                let outcome = if direction == RepairDirection::Finish {
                    ApplyOutcome::Applied
                } else {
                    ApplyOutcome::NotApplied
                };
                assert_eq!(receipt.outcome, Some(outcome));
                assert_eq!(app.store.editor("a.md").unwrap().unwrap(), before);
                let settled = app
                    .store
                    .proposal_apply(approval.operation_id)
                    .unwrap()
                    .unwrap();
                assert_eq!(settled.receipt.as_ref().unwrap().outcome, outcome);
                let observations = app.observe_proposal(&settled).unwrap();
                let prepared = journal.prepared.as_ref().unwrap();
                for (i, (change, proof)) in journal
                    .approved
                    .draft
                    .changes
                    .iter()
                    .zip(&observations)
                    .enumerate()
                {
                    let expected = match (direction, change) {
                        (RepairDirection::Finish, NoteChange::Trash { .. })
                        | (RepairDirection::Restore, NoteChange::Create { .. }) => None,
                        (RepairDirection::Finish, _) => Some(&prepared[i]),
                        (RepairDirection::Restore, _) => original(change),
                    };
                    assert_eq!(proof.destination.as_ref(), expected);
                    if let Some(expected) = expected {
                        let bytes =
                            fs::read(fixture.base.join("vault").join(change.path())).unwrap();
                        assert_eq!(bytes.len() as u64, expected.len);
                        let text = if direction == RepairDirection::Finish {
                            change.text().unwrap()
                        } else {
                            match change {
                                NoteChange::Replace { before_text, .. }
                                | NoteChange::Trash { before_text, .. } => before_text,
                                _ => unreachable!(),
                            }
                        };
                        assert_eq!(bytes, text.as_bytes());
                    }
                }
                assert_eq!(
                    app.proposal(approval.expected.id)
                        .unwrap()
                        .comments
                        .is_empty(),
                    direction == RepairDirection::Finish
                );
                assert!(app.note("source.md").is_ok());
                // Historical replay precedes both fresh disk and editor checks.
                fs::write(fixture.base.join("vault/a.md"), "Later owner bytes").unwrap();
                assert_eq!(app.repair_proposal(&request).unwrap(), receipt);
                assert_eq!(
                    fs::read(fixture.base.join("vault/a.md")).unwrap(),
                    b"Later owner bytes"
                );
                let mut wrong = request.clone();
                wrong.direction = if direction == RepairDirection::Finish {
                    RepairDirection::Restore
                } else {
                    RepairDirection::Finish
                };
                assert_eq!(
                    app.repair_proposal(&wrong).unwrap_err().kind,
                    ErrorKind::OperationConflict
                );
                drop(app);
                let mut reopened = fixture.app();
                assert_eq!(reopened.repair_proposal(&request).unwrap(), receipt);
                assert_eq!(
                    fs::read(fixture.base.join("vault/a.md")).unwrap(),
                    b"Later owner bytes"
                );
            }
        }
    }

    #[test]
    fn stale_source_unknown_member_and_dirty_alias_refuse_before_admission() {
        let (fixture, mut app, approval) = arrange(4);
        let finish = request(&mut app, approval.operation_id, RepairDirection::Finish);
        let journal = app
            .store
            .proposal_apply(approval.operation_id)
            .unwrap()
            .unwrap();
        let proofs = app.observe_proposal(&journal).unwrap();
        fs::write(
            fixture.base.join("vault/source.md"),
            "Changed independent source",
        )
        .unwrap();
        assert_eq!(
            app.repair_proposal(&finish).unwrap_err().kind,
            ErrorKind::ContextStale
        );
        assert!(app.store.proposal_repair(finish.id).unwrap().is_none());
        assert_eq!(app.observe_proposal(&journal).unwrap(), proofs);
        let restore = request(&mut app, approval.operation_id, RepairDirection::Restore);
        app.repair_proposal(&restore).unwrap();
        assert_eq!(
            fs::read(fixture.base.join("vault/source.md")).unwrap(),
            b"Changed independent source"
        );

        let (fixture, mut app, approval) = arrange(1);
        let restore = request(&mut app, approval.operation_id, RepairDirection::Restore);
        fs::write(fixture.base.join("vault/new.md"), "Unknown occupant").unwrap();
        assert!(app.repair_proposal(&restore).is_err());
        assert!(app.store.proposal_repair(restore.id).unwrap().is_none());
        assert_eq!(
            app.note("source.md").unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        assert_eq!(
            fs::read(fixture.base.join("vault/a.md")).unwrap(),
            "\u{feff}Approved λ\r\n".as_bytes()
        );

        let (_fixture, mut app, approval) = arrange(1);
        let finish = request(&mut app, approval.operation_id, RepairDirection::Finish);
        let editor = app.open_editor("A.md").unwrap().record;
        app.recover_editor(&EditRequest {
            path: "A.md".into(),
            expected: editor.stamp,
            generation: editor.stamp.generation + 1,
            text: "Unsaved later typing".into(),
        })
        .unwrap();
        assert_eq!(
            app.repair_proposal(&finish).unwrap_err().kind,
            ErrorKind::ContextStale
        );
        assert!(app.store.proposal_repair(finish.id).unwrap().is_none());
        assert_eq!(
            app.store.editor("A.md").unwrap().unwrap().text,
            "Unsaved later typing"
        );
    }

    fn crash_repair(fixture: &Fixture, request: &RepairRequest, phase: &str, member: usize) {
        fs::write(
            fixture.base.join("repair-request.json"),
            serde_json::to_vec(request).unwrap(),
        )
        .unwrap();
        let _process_fixtures = crate::SUBPROCESS_FIXTURES.lock().unwrap();
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "proposal_apply::tests::crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("BRN_APPLY_TEST_BASE", &fixture.base)
            .env("BRN_APPLY_TEST_REPAIR", "1")
            .env("BRN_APPLY_TEST_PHASE", phase)
            .env("BRN_APPLY_TEST_MEMBER", member.to_string())
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(86),
            "{phase}/{member}: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    fn interrupted_repairs_reconcile_proofs_and_replay_without_implicit_effects() {
        for direction in [RepairDirection::Finish, RepairDirection::Restore] {
            for (phase, member) in [
                ("repair-intent", 0),
                ("repair-mirror", 0),
                ("repair-member", 0),
                ("repair-synced", 0),
                ("repair-member", 2),
                ("repair-synced", 2),
                ("repair-verified", 0),
                ("completion", 0),
                ("receipt", 0),
            ] {
                let mask = if direction == RepairDirection::Finish {
                    2
                } else {
                    5
                };
                let (fixture, mut app, approval) = arrange(mask);
                let repair = request(&mut app, approval.operation_id, direction);
                drop(app);
                crash_repair(&fixture, &repair, phase, member);
                let files_before: Vec<_> = fs::read_dir(fixture.base.join("vault"))
                    .unwrap()
                    .map(|entry| {
                        let entry = entry.unwrap();
                        (
                            entry.file_name(),
                            fs::read(entry.path()).unwrap(),
                            std::os::unix::fs::MetadataExt::ino(&entry.metadata().unwrap()),
                        )
                    })
                    .collect();
                let mut app = fixture.app();
                let receipt = app.repair_proposal(&repair).unwrap();
                let files_after: Vec<_> = fs::read_dir(fixture.base.join("vault"))
                    .unwrap()
                    .map(|entry| {
                        let entry = entry.unwrap();
                        (
                            entry.file_name(),
                            fs::read(entry.path()).unwrap(),
                            std::os::unix::fs::MetadataExt::ino(&entry.metadata().unwrap()),
                        )
                    })
                    .collect();
                let mut before = files_before;
                let mut after = files_after;
                before.sort();
                after.sort();
                assert_eq!(before, after, "{direction:?} {phase}/{member}");
                if matches!(phase, "repair-intent" | "repair-mirror")
                    || member == 0 && matches!(phase, "repair-member" | "repair-synced")
                {
                    assert!(
                        receipt
                            .outcome
                            .is_none_or(|outcome| outcome == ApplyOutcome::Uncertain)
                    );
                    assert_eq!(
                        app.note("source.md").unwrap_err().kind,
                        ErrorKind::SaveUncertain
                    );
                    let next = request(&mut app, approval.operation_id, direction);
                    let finished = app.repair_proposal(&next).unwrap();
                    assert!(matches!(
                        finished.outcome,
                        Some(ApplyOutcome::Applied | ApplyOutcome::NotApplied)
                    ));
                } else {
                    assert_eq!(
                        receipt.outcome,
                        Some(if direction == RepairDirection::Finish {
                            ApplyOutcome::Applied
                        } else {
                            ApplyOutcome::NotApplied
                        })
                    );
                }
            }
        }
    }

    #[test]
    fn mirror_failures_keep_first_uncertainty_and_recover_only_proven_endpoints() {
        for direction in [RepairDirection::Finish, RepairDirection::Restore] {
            for phase in ["repair-intent", "repair-verified"] {
                let mut failures = vec![
                    "file_sync",
                    "prepare_directory_sync",
                    "directory_sync",
                    "postproof",
                ];
                if phase == "repair-verified" && direction == RepairDirection::Finish {
                    failures.push("cleanup_directory_sync");
                }
                for failure in failures {
                    let (fixture, mut app, approval) = arrange(1);
                    let repair = request(&mut app, approval.operation_id, direction);
                    let first = app
                        .store
                        .proposal_apply(approval.operation_id)
                        .unwrap()
                        .unwrap();
                    APPLY_HOOK.with(|hook| {
                        *hook.borrow_mut() = Some(Box::new(move |step, _| {
                            if step == phase {
                                crate::files::recovery::FAILURE
                                    .with(|selected| selected.set(Some(failure)));
                            }
                        }))
                    });
                    let result = app.repair_proposal(&repair);
                    APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
                    crate::files::recovery::FAILURE.with(|selected| selected.set(None));
                    assert!(result.is_err(), "{direction:?} {phase}/{failure}");
                    let pending = app
                        .store
                        .proposal_apply(approval.operation_id)
                        .unwrap()
                        .unwrap();
                    assert_eq!(pending.receipt, first.receipt);
                    assert_eq!(pending.observations, first.observations);
                    assert_eq!(
                        app.note("source.md").unwrap_err().kind,
                        ErrorKind::SaveUncertain
                    );
                    let files = app.observe_proposal(&pending).unwrap();
                    drop(app);
                    let mut app = fixture.app();
                    app.editor_files().unwrap();
                    let restored = app
                        .store
                        .proposal_apply(approval.operation_id)
                        .unwrap()
                        .unwrap();
                    assert_eq!(app.observe_proposal(&restored).unwrap(), files);
                    let receipt = app.repair_proposal(&repair).unwrap();
                    if phase == "repair-intent" {
                        assert_eq!(receipt.outcome, Some(ApplyOutcome::Uncertain));
                        assert_eq!(
                            app.note("source.md").unwrap_err().kind,
                            ErrorKind::SaveUncertain
                        );
                        let next = request(&mut app, approval.operation_id, direction);
                        assert!(matches!(
                            app.repair_proposal(&next).unwrap().outcome,
                            Some(ApplyOutcome::Applied | ApplyOutcome::NotApplied)
                        ));
                    } else {
                        assert_eq!(
                            receipt.outcome,
                            Some(if direction == RepairDirection::Finish {
                                ApplyOutcome::Applied
                            } else {
                                ApplyOutcome::NotApplied
                            })
                        );
                        assert!(app.note("source.md").is_ok());
                    }
                }
            }
        }
    }

    #[test]
    fn sqlite_receipt_failure_recovers_both_directions_without_overwriting_later_bytes() {
        for direction in [RepairDirection::Finish, RepairDirection::Restore] {
            let (fixture, mut app, approval) = arrange(1);
            let repair = request(&mut app, approval.operation_id, direction);
            let database = fixture.base.join("data/brn.sqlite");
            APPLY_HOOK.with(|hook| *hook.borrow_mut() = Some(Box::new(move |step, _| {
                if step == "completion" {
                    rusqlite::Connection::open(&database).unwrap().execute_batch("CREATE TRIGGER fail_repair_receipt BEFORE UPDATE ON proposal_applies WHEN NEW.outcome IN ('applied','not_applied') BEGIN SELECT RAISE(ABORT, 'synthetic repair receipt failure'); END;").unwrap();
                }
            })));
            let result = app.repair_proposal(&repair);
            APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
            assert!(result.is_err());
            assert_eq!(
                app.store
                    .proposal_apply(approval.operation_id)
                    .unwrap()
                    .unwrap()
                    .receipt
                    .unwrap()
                    .outcome,
                ApplyOutcome::Uncertain
            );
            assert_eq!(
                app.note("source.md").unwrap_err().kind,
                ErrorKind::SaveUncertain
            );
            rusqlite::Connection::open(fixture.base.join("data/brn.sqlite"))
                .unwrap()
                .execute_batch("DROP TRIGGER fail_repair_receipt;")
                .unwrap();
            fs::write(fixture.base.join("vault/a.md"), "Later owner bytes 🦀").unwrap();
            app.reconcile_proposal(approval.operation_id).unwrap();
            let receipt = app.repair_proposal(&repair).unwrap();
            assert_eq!(
                receipt.outcome,
                Some(if direction == RepairDirection::Finish {
                    ApplyOutcome::Applied
                } else {
                    ApplyOutcome::NotApplied
                })
            );
            assert_eq!(
                fs::read(fixture.base.join("vault/a.md")).unwrap(),
                "Later owner bytes 🦀".as_bytes()
            );
            assert_eq!(
                app.proposal(approval.expected.id)
                    .unwrap()
                    .comments
                    .is_empty(),
                direction == RepairDirection::Finish
            );
            assert!(app.note("source.md").is_ok());
        }
    }

    #[test]
    fn ordinary_repair_receipts_restore_older_or_missing_sqlite_without_namespace_retry() {
        for missing in [false, true] {
            for direction in [RepairDirection::Finish, RepairDirection::Restore] {
                let (fixture, mut app, approval) = arrange(1);
                let earlier = fs::read(fixture.base.join("data/brn.sqlite")).unwrap();
                let repair = request(&mut app, approval.operation_id, direction);
                let receipt = app.repair_proposal(&repair).unwrap();
                drop(app);
                fs::write(fixture.base.join("vault/a.md"), "Later bytes after repair").unwrap();
                let database = fixture.base.join("data/brn.sqlite");
                if missing {
                    fs::remove_file(&database).unwrap();
                    fs::remove_dir_all(fixture.base.join("data/backups")).unwrap();
                } else {
                    fs::write(&database, &earlier).unwrap();
                }
                let mut app = fixture.app();
                assert_eq!(app.repair_proposal(&repair).unwrap(), receipt);
                assert_eq!(
                    fs::read(fixture.base.join("vault/a.md")).unwrap(),
                    b"Later bytes after repair"
                );
                assert_eq!(
                    app.proposal(approval.expected.id)
                        .unwrap()
                        .comments
                        .is_empty(),
                    direction == RepairDirection::Finish
                );
                assert!(app.note("source.md").is_ok());
            }
        }
    }

    #[test]
    fn unknown_before_stage_after_admission_cannot_discharge_the_repair_fence() {
        let (fixture, mut app, approval) = arrange(0);
        let repair = request(&mut app, approval.operation_id, RepairDirection::Restore);
        let journal = app
            .store
            .proposal_apply(approval.operation_id)
            .unwrap()
            .unwrap();
        let stage = fixture.base.join("vault").join(&journal.members[1].staging);
        APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |step, _| {
                if step == "repair-intent" {
                    fs::remove_file(&stage).unwrap();
                }
            }))
        });
        let result = app.repair_proposal(&repair);
        APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert!(result.is_err());
        assert_eq!(
            app.reconcile_proposal(approval.operation_id)
                .unwrap()
                .outcome,
            ApplyOutcome::Uncertain
        );
        assert_eq!(
            app.note("source.md").unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        assert_eq!(
            app.repair_proposal(&repair).unwrap().outcome,
            Some(ApplyOutcome::Uncertain)
        );
        drop(app);
        let mut app = fixture.app();
        assert_eq!(
            app.note("source.md").unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        assert!(app.preview_proposal_repair(approval.operation_id).is_err());
    }
}
