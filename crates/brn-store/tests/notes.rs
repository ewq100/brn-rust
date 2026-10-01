use brn_store::{OperationStatus, Store, notes::*};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

#[test]
fn unconsumed_copy_stage_resolves_failure_releases_reservation_and_preserves_replay() {
    let (data, _vault, mut store, opened) = fixture();
    let vault = store.note_vault(opened.note_id).unwrap();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("copy.md"),
            NoteWriteKind::Copy,
            &DestinationPrecondition::Absent {
                parent: vault.identity.clone(),
            },
        )
        .unwrap();
    let stage = PreparedFile {
        relative: intent.staging_relative.clone(),
        fingerprint: fingerprint(&request.text, 4),
    };
    store
        .record_note_prepared(request.operation_id, &stage)
        .unwrap();
    let error = store
        .record_note_write_failure(
            &request,
            Path::new("copy.md"),
            NoteWriteKind::Copy,
            &NoteFailure {
                code: NoteErrorCode::SaveUncertain,
                message: "transient observation failure".into(),
                operation_id: Some(request.operation_id),
                note_id: Some(request.note_id),
                phase: Some(SavePhase::Prepared),
                filesystem_outcome: FileOutcome::Unknown,
                recovery_available: true,
            },
        )
        .unwrap();
    let result = NoteRecordedResult::Failure(error.clone());
    drop(store);
    let (mut store, _) = Store::open(data.path()).unwrap();
    assert_eq!(
        store
            .reconcile_note_copy_not_installed(request.operation_id, &stage, &result)
            .unwrap(),
        result
    );
    assert_eq!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap()
            .resolution,
        NoteResolution::NotApplied
    );
    assert_eq!(
        store.note_save_result(request.operation_id).unwrap(),
        Some(result.clone())
    );
    assert_eq!(
        store
            .note_recovery(request.note_id)
            .unwrap()
            .unwrap()
            .working,
        request.text
    );
    let occupant = fingerprint("external", 5);
    store
        .enroll_note(
            Uuid::new_v4(),
            &vault,
            Path::new("copy.md"),
            occupant,
            "external",
        )
        .unwrap();
    assert_eq!(
        store
            .reconcile_note_copy_not_installed(request.operation_id, &stage, &result)
            .unwrap(),
        result
    );
    let mut changed = error;
    changed.message = "not the original failure".into();
    assert_eq!(
        store
            .reconcile_note_copy_not_installed(
                request.operation_id,
                &stage,
                &NoteRecordedResult::Failure(changed)
            )
            .unwrap_err()
            .code,
        NoteErrorCode::OperationConflict
    );
}

#[test]
fn copy_not_installed_requires_exact_recorded_stage_and_unexchanged_copy_phase() {
    for invalid in ["unprepared", "fingerprint", "path", "exchanged", "replace"] {
        let (_data, _vault, mut store, opened) = fixture();
        let request = submission(&opened);
        let (kind, destination, expected) = if invalid == "replace" {
            (
                NoteWriteKind::Replace,
                Path::new("plan.md"),
                precondition(&opened),
            )
        } else {
            (
                NoteWriteKind::Copy,
                Path::new("copy.md"),
                DestinationPrecondition::Absent {
                    parent: store.note_vault(opened.note_id).unwrap().identity,
                },
            )
        };
        let intent = store
            .begin_note_save(&request, destination, kind, &expected)
            .unwrap();
        let mut stage = PreparedFile {
            relative: intent.staging_relative.clone(),
            fingerprint: fingerprint(&request.text, 4),
        };
        if invalid != "unprepared" {
            store
                .record_note_prepared(request.operation_id, &stage)
                .unwrap();
        }
        match invalid {
            "fingerprint" => stage.fingerprint.inode += 1,
            "path" => stage.relative = "other.stage".into(),
            "exchanged" => store.mark_note_exchanged(request.operation_id).unwrap(),
            _ => (),
        }
        let before = store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap();
        let result = NoteRecordedResult::Failure(NoteFailure {
            code: NoteErrorCode::Conflict,
            message: "collision".into(),
            operation_id: Some(request.operation_id),
            note_id: Some(request.note_id),
            phase: Some(before.phase),
            filesystem_outcome: FileOutcome::NotApplied,
            recovery_available: true,
        });
        assert_eq!(
            store
                .reconcile_note_copy_not_installed(request.operation_id, &stage, &result)
                .unwrap_err()
                .code,
            NoteErrorCode::SaveUncertain,
            "{invalid}"
        );
        assert_eq!(
            store
                .note_save_intent(request.operation_id)
                .unwrap()
                .unwrap(),
            before
        );
        assert!(
            store
                .note_save_result(request.operation_id)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn search_snapshot_receipts_bind_state_preserve_originals_and_track_permission_epochs() {
    use brn_store::Approval;
    let (_data, _vault, mut store, opened) = fixture();
    let record = store.note_record(opened.note_id).unwrap();
    let recovery = store.note_recovery(opened.note_id).unwrap().unwrap();
    let origin = store
        .note_vault(opened.note_id)
        .unwrap()
        .root
        .join("plan.md");
    let imported = store
        .import_text(
            Uuid::new_v4(),
            origin.to_str().unwrap(),
            "plan.md",
            recovery.baseline.as_bytes(),
            Approval::Approved,
        )
        .unwrap();
    store.reconcile_note_sources().unwrap();
    assert_eq!(
        store.note_source_associations().unwrap(),
        vec![(opened.note_id, imported.source_id, true)]
    );
    let state = store
        .record_note_observation(opened.note_id, &record.baseline)
        .unwrap();
    let request = NoteSearchRequest::Approve {
        note_id: opened.note_id,
        file_state: state,
    };
    let op = Uuid::new_v4();
    let (receipt, changed) = store
        .freeze_note_search_snapshot(op, &request, state, &record.baseline, &recovery.baseline)
        .unwrap();
    assert!(changed);
    assert_ne!(receipt.source_id, imported.source_id);
    assert_eq!(
        store.version(imported.version_id).unwrap().unwrap().bytes,
        recovery.baseline.as_bytes()
    );
    let epochs = store.note_evidence_epochs(opened.note_id).unwrap();
    assert_eq!(epochs, (0, 1));
    store
        .record_note_observation(opened.note_id, &record.baseline)
        .unwrap();
    assert_eq!(store.note_evidence_epochs(opened.note_id).unwrap(), epochs);
    let changed_file = fingerprint("changed", 3);
    store
        .record_note_observation(opened.note_id, &changed_file)
        .unwrap();
    assert_eq!(store.note_evidence_epochs(opened.note_id).unwrap(), (1, 2));
    store
        .record_note_observation(opened.note_id, &changed_file)
        .unwrap();
    assert_eq!(store.note_evidence_epochs(opened.note_id).unwrap(), (1, 2));
    assert_eq!(
        store.note_search_replay(op, &request).unwrap(),
        Some((receipt.clone(), changed))
    );
    assert_eq!(
        store
            .freeze_note_search_snapshot(op, &request, state, &record.baseline, &recovery.baseline)
            .unwrap(),
        (receipt, true)
    );
    assert_eq!(
        store.note_record(opened.note_id).unwrap().search_approval,
        Approval::Draft
    );
    assert_eq!(
        store
            .note_search_replay(
                op,
                &NoteSearchRequest::Approve {
                    note_id: opened.note_id,
                    file_state: Uuid::new_v4()
                }
            )
            .unwrap_err()
            .code,
        NoteErrorCode::OperationConflict
    );
}

#[test]
fn observation_epoch_overflow_is_rejected_atomically() {
    let (data, _vault, mut store, opened) = fixture();
    let conn = rusqlite::Connection::open(data.path().join("brn.sqlite3")).unwrap();
    conn.execute(
        "UPDATE notes SET content_epoch=9223372036854775807 WHERE id=?1",
        [opened.note_id.to_string()],
    )
    .unwrap();
    let before = store.note_record(opened.note_id).unwrap();
    assert_eq!(
        store
            .record_note_observation(opened.note_id, &fingerprint("changed", 3))
            .unwrap_err()
            .code,
        NoteErrorCode::Storage
    );
    assert_eq!(store.note_record(opened.note_id).unwrap(), before);
}

#[test]
fn search_snapshot_and_receipt_identity_mismatches_fail_visibly() {
    for snapshot in [false, true] {
        let (data, _vault, mut store, opened) = fixture();
        let file = store.note_record(opened.note_id).unwrap().baseline;
        let state = store
            .record_note_observation(opened.note_id, &file)
            .unwrap();
        let request = NoteSearchRequest::Approve {
            note_id: opened.note_id,
            file_state: state,
        };
        let op = Uuid::new_v4();
        let (mut receipt, changed) = store
            .freeze_note_search_snapshot(op, &request, state, &file, &opened.baseline)
            .unwrap();
        let conn = rusqlite::Connection::open(data.path().join("brn.sqlite3")).unwrap();
        if snapshot {
            receipt.note_id = Uuid::new_v4();
            let bytes = serde_json::to_vec(&NoteSearchSnapshot {
                receipt,
                fingerprint: file,
            })
            .unwrap();
            conn.execute(
                "UPDATE note_search_snapshots SET snapshot_json=?1,snapshot_sha256=?2",
                rusqlite::params![bytes, Sha256::digest(&bytes).as_slice()],
            )
            .unwrap();
            assert_eq!(
                store.note_search_snapshot(opened.note_id).unwrap_err().code,
                NoteErrorCode::Storage
            );
        } else {
            receipt.operation_id = Uuid::new_v4();
            let bytes = serde_json::to_vec(&(receipt, changed)).unwrap();
            conn.execute(
                "UPDATE note_search_results SET result_json=?1,result_sha256=?2",
                rusqlite::params![bytes, Sha256::digest(&bytes).as_slice()],
            )
            .unwrap();
            assert_eq!(
                store.note_search_replay(op, &request).unwrap_err().code,
                NoteErrorCode::Storage
            );
        }
    }
}

#[test]
fn reload_and_relink_bind_caller_decisions_and_only_discard_confirmed_work() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let dirty = store.save_note_buffer(&request).unwrap();
    let op = Uuid::new_v4();
    let stale = NoteDecision::Reload {
        note_id: opened.note_id,
        expected: opened.stamp,
        discard: true,
    };
    let observed = fingerprint("external\r\n", 3);
    assert_eq!(
        store
            .record_note_decision(op, &stale, &observed, "external\r\n")
            .unwrap_err()
            .code,
        NoteErrorCode::StateChanged
    );
    let unconfirmed = NoteDecision::Reload {
        note_id: opened.note_id,
        expected: dirty.stamp,
        discard: false,
    };
    assert_eq!(
        store
            .record_note_decision(op, &unconfirmed, &observed, "external\r\n")
            .unwrap_err()
            .code,
        NoteErrorCode::Conflict
    );
    let confirmed = NoteDecision::Reload {
        note_id: opened.note_id,
        expected: dirty.stamp,
        discard: true,
    };
    assert_eq!(
        store
            .record_note_decision(
                op,
                &confirmed,
                &fingerprint("external\r\n", 8),
                "external\r\n"
            )
            .unwrap_err()
            .code,
        NoteErrorCode::Conflict
    );
    let refreshed = store
        .record_note_decision(op, &confirmed, &observed, "external\r\n")
        .unwrap();
    assert_eq!(refreshed.working, "external\r\n");
    assert_eq!(refreshed.stamp.generation, dirty.stamp.generation);
    assert!(store.note_decision_recovery(op).unwrap().is_none());
    assert_eq!(
        store.note_decision_replay(op, &confirmed).unwrap(),
        Some(opened.note_id)
    );
    assert_eq!(
        store
            .note_decision_replay(op, &unconfirmed)
            .unwrap_err()
            .code,
        NoteErrorCode::OperationConflict
    );
    let relink = NoteDecision::Relink {
        note_id: opened.note_id,
        expected: refreshed.stamp,
        relative: "moved.md".into(),
        confirm_identity: true,
    };
    let result = store
        .record_note_decision(
            Uuid::new_v4(),
            &relink,
            &fingerprint("new identity", 8),
            "new identity",
        )
        .unwrap();
    assert_eq!(result.working, "external\r\n");
    assert_eq!(result.baseline, "new identity");
    assert_eq!(
        store.note_record(opened.note_id).unwrap().relative_path,
        Path::new("moved.md")
    );
}

#[test]
fn accept_current_checks_active_job_and_reviewed_observation_preserving_original_result() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    let observed = fingerprint("external", 9);
    let token = store
        .record_note_observation(opened.note_id, &observed)
        .unwrap();
    let ack = Uuid::new_v4();
    assert_eq!(
        store
            .accept_note_disk_state(ack, request.operation_id, token, &observed, "external")
            .unwrap_err()
            .code,
        NoteErrorCode::WorkspaceBusy
    );
    let original = store
        .record_note_write_failure(
            &request,
            &intent.destination,
            intent.kind,
            &NoteFailure {
                code: NoteErrorCode::SaveUncertain,
                message: "unknown".into(),
                operation_id: None,
                note_id: None,
                phase: Some(intent.phase),
                filesystem_outcome: FileOutcome::Unknown,
                recovery_available: false,
            },
        )
        .unwrap();
    assert_eq!(
        store
            .accept_note_disk_state(
                ack,
                request.operation_id,
                token,
                &fingerprint("later", 9),
                "later"
            )
            .unwrap_err()
            .code,
        NoteErrorCode::StateChanged
    );
    let current = store
        .accept_note_disk_state(ack, request.operation_id, token, &observed, "external")
        .unwrap();
    assert_eq!(current.baseline, "external");
    assert_eq!(current.working, request.text);
    assert_eq!(current.stamp.generation, request.generation);
    assert_eq!(
        store
            .accept_note_disk_state(ack, request.operation_id, token, &observed, "external")
            .unwrap(),
        current
    );
    let accepted = store
        .note_save_intent(request.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(accepted.resolution, NoteResolution::AcceptedCurrent);
    assert_eq!(accepted.acknowledged_by, Some(ack));
    assert_eq!(
        accepted.prior_result,
        Some(NoteRecordedResult::Failure(original.clone()))
    );
    assert_eq!(accepted.expected_destination, intent.expected_destination);
    let retained = store.note_decision_recovery(ack).unwrap().unwrap();
    assert_eq!(retained.baseline, opened.baseline);
    assert_eq!(retained.working, request.text);
    assert!(
        store
            .record_note_cleanup(request.operation_id, ArtifactCleanup::Retired)
            .is_err()
    );
    assert_eq!(
        store
            .note_write_result(&request, Path::new("plan.md"), NoteWriteKind::Replace)
            .unwrap(),
        Some(NoteRecordedResult::Failure(original))
    );
    store
        .begin_note_save(
            &NoteSubmission {
                operation_id: Uuid::new_v4(),
                expected: current.stamp,
                generation: 2,
                text: "next".into(),
                ..request
            },
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &DestinationPrecondition::Existing {
                fingerprint: observed,
                baseline_text: "external".into(),
            },
        )
        .unwrap();
}

#[test]
fn recorded_pre_exchange_failure_resolves_not_applied_without_changing_replay() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    store
        .record_note_cleanup(request.operation_id, ArtifactCleanup::RetainedUnexpected)
        .unwrap();
    let failure = store
        .record_note_write_failure(
            &request,
            &intent.destination,
            NoteWriteKind::Replace,
            &NoteFailure {
                code: NoteErrorCode::Io,
                message: "pre-exchange failure".into(),
                operation_id: None,
                note_id: None,
                phase: Some(SavePhase::Intent),
                filesystem_outcome: FileOutcome::NotApplied,
                recovery_available: false,
            },
        )
        .unwrap();
    let mut record = NoteReconciliation {
        resolution: NoteResolution::NotApplied,
        observed_destination: None,
        verification: None,
        result: NoteRecordedResult::Failure(failure.clone()),
    };
    assert!(
        store
            .reconcile_note_operation(request.operation_id, &record)
            .is_err()
    );
    assert_eq!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap()
            .resolution,
        NoteResolution::Unresolved
    );
    record.observed_destination = Some(fingerprint(&opened.baseline, 3));
    let mut changed = record.clone();
    if let NoteRecordedResult::Failure(error) = &mut changed.result {
        error.message = "different failure".into();
    }
    assert_eq!(
        store
            .reconcile_note_operation(request.operation_id, &changed)
            .unwrap_err()
            .code,
        NoteErrorCode::OperationConflict
    );
    assert_eq!(
        store
            .reconcile_note_operation(request.operation_id, &record)
            .unwrap(),
        record.result
    );
    let resolved = store
        .note_save_intent(request.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(resolved.resolution, NoteResolution::NotApplied);
    assert_eq!(resolved.cleanup, ArtifactCleanup::RetainedUnexpected);
    assert_eq!(
        store
            .note_write_result(&request, &intent.destination, NoteWriteKind::Replace)
            .unwrap(),
        Some(NoteRecordedResult::Failure(failure))
    );
    let recovered = store.note_recovery(opened.note_id).unwrap().unwrap();
    assert_eq!(recovered.working, request.text);
    assert!(recovered.pending_operations.is_empty());
    store
        .begin_note_save(
            &NoteSubmission {
                operation_id: Uuid::new_v4(),
                expected: recovered.stamp,
                generation: 2,
                text: "next".into(),
                ..request
            },
            &intent.destination,
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
}

fn fingerprint(text: &str, inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: text.len() as u64,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}

#[test]
fn cleanup_candidate_requires_terminal_proof_and_preserved_recovery() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    assert_eq!(
        store.note_cleanup_candidate(request.operation_id).unwrap(),
        None
    );
    let prepared = PreparedFile {
        relative: intent.staging_relative.clone(),
        fingerprint: fingerprint(&request.text, 8),
    };
    store
        .record_note_prepared(request.operation_id, &prepared)
        .unwrap();
    let receipt = NoteReceipt {
        operation_id: request.operation_id,
        source_note_id: request.note_id,
        note_id: request.note_id,
        submitted_generation: request.generation,
        stamp: NoteStamp {
            file_state: request.expected.file_state,
            generation: request.generation,
        },
        filesystem_outcome: FileOutcome::NotApplied,
        recovery_available: true,
    };
    store
        .reconcile_note_operation(
            request.operation_id,
            &NoteReconciliation {
                resolution: NoteResolution::NotApplied,
                observed_destination: Some(fingerprint(&opened.baseline, 3)),
                verification: None,
                result: NoteRecordedResult::Receipt(receipt),
            },
        )
        .unwrap();
    let candidate = store
        .note_cleanup_candidate(request.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(candidate.relative, prepared.relative);
    assert_eq!(candidate.identity.inode, 8);
    assert_eq!(candidate.sha256, Some(prepared.fingerprint.sha256));
    store
        .record_note_cleanup(request.operation_id, ArtifactCleanup::RetainedUnexpected)
        .unwrap();
    assert_eq!(
        store.note_cleanup_candidate(request.operation_id).unwrap(),
        None
    );
}
#[test]
fn durable_observations_are_separate_from_the_protected_baseline() {
    let (data, _vault, mut store, opened) = fixture();
    let record = store.note_record(opened.note_id).unwrap();
    assert_eq!(record.relative_path, Path::new("plan.md"));
    assert_eq!(record.baseline, fingerprint(&opened.baseline, 3));
    assert_eq!(record.stamp, opened.stamp);
    assert_eq!(store.note_recoveries().unwrap(), vec![opened.clone()]);
    assert_eq!(
        store.registered_vault().unwrap(),
        Some(store.note_vault(opened.note_id).unwrap())
    );
    let original = store
        .record_note_observation(opened.note_id, &record.baseline)
        .unwrap();
    assert_eq!(original, opened.stamp.file_state);
    let external = fingerprint("external", 4);
    let token = store
        .record_note_observation(opened.note_id, &external)
        .unwrap();
    assert_ne!(token, original);
    assert_eq!(
        store
            .record_note_observation(opened.note_id, &external)
            .unwrap(),
        token
    );
    let request = submission(&opened);
    store.save_note_buffer(&request).unwrap();
    drop(store);
    let (mut store, _) = Store::open(data.path()).unwrap();
    let record = store.note_record(opened.note_id).unwrap();
    assert_eq!(record.baseline, fingerprint(&opened.baseline, 3));
    assert_eq!(record.stamp.file_state, original);
    assert_eq!(record.stamp.generation, 1);
    assert_eq!(record.observed, Some((token, external.clone())));
    assert_eq!(
        store
            .record_note_observation(opened.note_id, &external)
            .unwrap(),
        token
    );
    assert_eq!(
        store
            .note_recovery(opened.note_id)
            .unwrap()
            .unwrap()
            .baseline,
        opened.baseline
    );
    assert_eq!(
        store
            .record_note_observation(opened.note_id, &record.baseline)
            .unwrap(),
        original
    );
    store
        .begin_note_save(
            &NoteSubmission {
                operation_id: Uuid::new_v4(),
                expected: record.stamp,
                ..request
            },
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
}

#[test]
fn submission_preflight_is_read_only_and_uses_transaction_generation_rules() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    store.validate_note_submission(&request).unwrap();
    assert_eq!(
        store.note_recovery(opened.note_id).unwrap().unwrap(),
        opened
    );
    let buffer = store.save_note_buffer(&request).unwrap();
    assert_eq!(
        store.validate_note_submission(&request).unwrap_err().code,
        NoteErrorCode::StateChanged
    );
    let equal = NoteSubmission {
        operation_id: Uuid::new_v4(),
        expected: buffer.stamp,
        ..request
    };
    store.validate_note_submission(&equal).unwrap();
    assert_eq!(
        store
            .validate_note_submission(&NoteSubmission {
                text: "changed".into(),
                ..equal
            })
            .unwrap_err()
            .code,
        NoteErrorCode::StateChanged
    );
}

#[test]
fn observation_payload_corruption_is_a_typed_storage_failure() {
    let (data, _vault, mut store, opened) = fixture();
    store
        .record_note_observation(opened.note_id, &fingerprint("external", 4))
        .unwrap();
    let conn = rusqlite::Connection::open(data.path().join("brn.sqlite3")).unwrap();
    conn.execute(
        "UPDATE notes SET observed_fingerprint_json=?1",
        [b"{}".as_slice()],
    )
    .unwrap();
    assert_eq!(
        store.note_record(opened.note_id).unwrap_err().code,
        NoteErrorCode::Storage
    );
}

#[test]
fn only_a_changed_observation_invalidates_version_bound_approval() {
    let (data, _vault, mut store, opened) = fixture();
    let external = fingerprint("external", 4);
    store
        .record_note_observation(opened.note_id, &external)
        .unwrap();
    let conn = rusqlite::Connection::open(data.path().join("brn.sqlite3")).unwrap();
    conn.execute("UPDATE notes SET approval='approved'", [])
        .unwrap();
    store
        .record_note_observation(opened.note_id, &external)
        .unwrap();
    assert_eq!(
        store.note_record(opened.note_id).unwrap().search_approval,
        brn_store::Approval::Approved
    );
    store
        .record_note_observation(opened.note_id, &fingerprint(&opened.baseline, 3))
        .unwrap();
    assert_eq!(
        store.note_record(opened.note_id).unwrap().search_approval,
        brn_store::Approval::Draft
    );
}

#[test]
fn enrollment_replays_caller_paths_before_validating_new_observations() {
    let (_data, _vault, mut store, opened) = fixture();
    let vault = store.note_vault(opened.note_id).unwrap();
    let op = Uuid::new_v4();
    assert_eq!(
        store
            .note_enrollment_replay(op, &vault.root, Path::new("plan.md"))
            .unwrap(),
        None
    );
    let enrolled = store
        .enroll_note(
            op,
            &vault,
            Path::new("plan.md"),
            fingerprint("external", 9),
            "external",
        )
        .unwrap();
    assert_eq!(enrolled.note_id, opened.note_id);
    assert_eq!(
        store
            .note_enrollment_replay(op, &vault.root, Path::new("plan.md"))
            .unwrap(),
        Some(opened.note_id)
    );
    assert_eq!(
        store
            .enroll_note(
                op,
                &vault,
                Path::new("plan.md"),
                fingerprint("different", 10),
                "different"
            )
            .unwrap(),
        enrolled
    );
    for (root, path) in [
        (vault.root.clone(), Path::new("other.md")),
        (vault.root.join("other"), Path::new("plan.md")),
    ] {
        assert_eq!(
            store
                .note_enrollment_replay(op, &root, path)
                .unwrap_err()
                .code,
            NoteErrorCode::OperationConflict
        );
    }
}

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, Store, NoteRecovery) {
    let data = tempfile::tempdir_in(".").unwrap();
    let vault = tempfile::tempdir_in(".").unwrap();
    let (mut store, _) = Store::open(data.path()).unwrap();
    let text = "\u{feff}# plan\r\n";
    let opened = store
        .enroll_note(
            Uuid::new_v4(),
            &VaultRecord {
                id: Uuid::new_v4(),
                root: vault.path().canonicalize().unwrap(),
                identity: VaultIdentity {
                    device: 1,
                    inode: 2,
                },
            },
            Path::new("plan.md"),
            fingerprint(text, 3),
            text,
        )
        .unwrap();
    (data, vault, store, opened)
}

fn submission(opened: &NoteRecovery) -> NoteSubmission {
    NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.note_id,
        expected: opened.stamp,
        generation: 1,
        text: "updated\r\n".into(),
    }
}

fn precondition(opened: &NoteRecovery) -> DestinationPrecondition {
    DestinationPrecondition::Existing {
        fingerprint: fingerprint(&opened.baseline, 3),
        baseline_text: opened.baseline.clone(),
    }
}

#[test]
fn exact_buffer_replay_survives_reopen() {
    let (data, _vault, mut store, opened) = fixture();
    assert_eq!(opened.baseline, "\u{feff}# plan\r\n");
    let request = submission(&opened);
    let first = store.save_note_buffer(&request).unwrap();
    assert_eq!(store.save_note_buffer(&request).unwrap(), first);
    drop(store);
    let (store, _) = Store::open(data.path()).unwrap();
    assert_eq!(
        store
            .note_recovery(opened.note_id)
            .unwrap()
            .unwrap()
            .working,
        "updated\r\n"
    );
}

#[test]
fn changed_payload_and_wrong_stamps_never_overwrite_working_text() {
    let (_data, _vault, mut store, opened) = fixture();
    let mut request = submission(&opened);
    let first = store.save_note_buffer(&request).unwrap();
    request.text = "different".into();
    assert_eq!(
        store.save_note_buffer(&request).unwrap_err().code,
        NoteErrorCode::OperationConflict
    );
    request.operation_id = Uuid::new_v4();
    assert_eq!(
        store.save_note_buffer(&request).unwrap_err().code,
        NoteErrorCode::StateChanged
    );
    request.expected = first.stamp;
    assert_eq!(
        store.save_note_buffer(&request).unwrap_err().code,
        NoteErrorCode::StateChanged
    );
    request.generation = 0;
    assert_eq!(
        store.save_note_buffer(&request).unwrap_err().code,
        NoteErrorCode::StateChanged
    );
    request.generation = u64::MAX;
    assert_eq!(
        store.save_note_buffer(&request).unwrap_err().code,
        NoteErrorCode::StateChanged
    );
    assert_eq!(
        store
            .note_recovery(opened.note_id)
            .unwrap()
            .unwrap()
            .working,
        "updated\r\n"
    );
}

#[test]
fn buffers_coalesce_and_equal_identical_generation_is_valid() {
    let (_data, _vault, mut store, opened) = fixture();
    let mut request = submission(&opened);
    let first = store.save_note_buffer(&request).unwrap();
    request.operation_id = Uuid::new_v4();
    request.expected = first.stamp;
    assert_eq!(store.save_note_buffer(&request).unwrap().stamp, first.stamp);
    request.operation_id = Uuid::new_v4();
    request.generation = 8;
    request.text = "".into();
    let last = store.save_note_buffer(&request).unwrap();
    let recovered = store.note_recovery(opened.note_id).unwrap().unwrap();
    assert_eq!(recovered.stamp, last.stamp);
    assert_eq!(recovered.working, "");
    assert_eq!(recovered.baseline, opened.baseline);
}

#[test]
fn unresolved_original_guard_is_atomic_and_copy_is_independent() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    assert_eq!(intent.phase, SavePhase::Intent);
    assert_eq!(intent.resolution, NoteResolution::Unresolved);
    assert_ne!(
        intent.staging_relative.extension().and_then(|x| x.to_str()),
        Some("md")
    );
    assert_eq!(
        store
            .begin_note_save(
                &request,
                Path::new("plan.md"),
                NoteWriteKind::Replace,
                &precondition(&opened)
            )
            .unwrap(),
        intent
    );
    let recovered = store.note_recovery(opened.note_id).unwrap().unwrap();
    let mut second = submission(&recovered);
    second.generation = 2;
    second.text = "blocked".into();
    assert_eq!(
        store
            .begin_note_save(
                &second,
                Path::new("plan.md"),
                NoteWriteKind::Replace,
                &precondition(&opened)
            )
            .unwrap_err()
            .code,
        NoteErrorCode::SaveUncertain
    );
    assert!(store.operation(second.operation_id).unwrap().is_none());
    assert_eq!(
        store
            .note_recovery(opened.note_id)
            .unwrap()
            .unwrap()
            .working,
        "updated\r\n"
    );
    let copy = store
        .begin_note_save(
            &second,
            Path::new("copy.md"),
            NoteWriteKind::Copy,
            &DestinationPrecondition::Absent {
                parent: VaultIdentity {
                    device: 1,
                    inode: 2,
                },
            },
        )
        .unwrap();
    assert_ne!(copy.target_note_id, opened.note_id);
    assert_eq!(store.note_save_intents().unwrap().len(), 2);
    assert!(
        store
            .finish_operation(request.operation_id, OperationStatus::Completed, b"fake")
            .is_err()
    );
}

#[test]
fn refusal_is_bound_and_replays_before_state_checks_without_an_intent() {
    let (data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let failure = NoteFailure {
        code: NoteErrorCode::VaultUnavailable,
        message: "root missing".into(),
        operation_id: Some(request.operation_id),
        note_id: Some(request.note_id),
        phase: None,
        filesystem_outcome: FileOutcome::NotApplied,
        recovery_available: false,
    };
    let ack = store
        .record_note_write_failure(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &failure,
        )
        .unwrap();
    assert!(ack.recovery_available);
    assert!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .is_none()
    );
    drop(store);
    let (store, _) = Store::open(data.path()).unwrap();
    assert_eq!(
        store
            .note_write_result(&request, Path::new("plan.md"), NoteWriteKind::Replace)
            .unwrap(),
        Some(NoteRecordedResult::Failure(ack))
    );
    let mut changed = request.clone();
    changed.text = "different".into();
    assert_eq!(
        store
            .note_write_result(&changed, Path::new("plan.md"), NoteWriteKind::Replace)
            .unwrap_err()
            .code,
        NoteErrorCode::OperationConflict
    );
}

#[test]
fn interrupted_save_reconciles_but_generic_interrupted_cannot_complete() {
    let (data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    let generic = Uuid::new_v4();
    store
        .begin_operation(generic, "external", b"input")
        .unwrap();
    drop(store);
    let (mut store, report) = Store::open(data.path()).unwrap();
    assert_eq!(report.interrupted_operations, 2);
    assert!(
        store
            .finish_operation(generic, OperationStatus::Completed, b"done")
            .is_err()
    );
    let receipt = NoteReceipt {
        operation_id: request.operation_id,
        source_note_id: opened.note_id,
        note_id: opened.note_id,
        submitted_generation: 1,
        stamp: NoteStamp {
            file_state: opened.stamp.file_state,
            generation: 1,
        },
        filesystem_outcome: FileOutcome::NotApplied,
        recovery_available: true,
    };
    let record = NoteReconciliation {
        resolution: NoteResolution::NotApplied,
        observed_destination: Some(fingerprint(&opened.baseline, 3)),
        verification: None,
        result: NoteRecordedResult::Receipt(receipt),
    };
    assert_eq!(
        store
            .reconcile_note_operation(request.operation_id, &record)
            .unwrap(),
        record.result
    );
    assert_eq!(
        store
            .reconcile_note_operation(request.operation_id, &record)
            .unwrap(),
        record.result
    );
    assert_eq!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap()
            .resolution,
        NoteResolution::NotApplied
    );
}

#[test]
fn turn_currentness_is_atomic_and_replay_cannot_change_it() {
    use brn_store::EvidenceCurrentness;
    let data = tempfile::tempdir_in(".").unwrap();
    let (mut store, _) = Store::open(data.path()).unwrap();
    let session = store
        .create_session(Uuid::new_v4(), "test", "store", None, Some("thread"), b"{}")
        .unwrap();
    let op = Uuid::new_v4();
    store
        .prepare_turn(op, session, "question", "keyword", "[]")
        .unwrap();
    store
        .complete_turn_with_currentness(
            op,
            OperationStatus::Completed,
            "answer",
            None,
            EvidenceCurrentness::StaleAtCompletion,
        )
        .unwrap();
    assert_eq!(
        store.turns(session).unwrap()[0].evidence_currentness,
        EvidenceCurrentness::StaleAtCompletion
    );
    assert!(
        store
            .complete_turn_with_currentness(
                op,
                OperationStatus::Completed,
                "answer",
                None,
                EvidenceCurrentness::CurrentAtCompletion
            )
            .is_err()
    );
    drop(store);
    let (store, _) = Store::open(data.path()).unwrap();
    assert_eq!(
        store.turns(session).unwrap()[0].evidence_currentness,
        EvidenceCurrentness::StaleAtCompletion
    );
}

fn prepare(store: &mut Store, intent: &NoteSaveIntent) -> NoteVerification {
    let prepared = PreparedFile {
        relative: intent.staging_relative.clone(),
        fingerprint: fingerprint(&intent.request.text, 3 + intent.request.generation),
    };
    store
        .record_note_prepared(intent.request.operation_id, &prepared)
        .unwrap();
    store
        .record_note_prepared(intent.request.operation_id, &prepared)
        .unwrap();
    store
        .mark_note_exchanged(intent.request.operation_id)
        .unwrap();
    match &intent.expected_destination {
        DestinationPrecondition::Existing {
            fingerprint,
            baseline_text,
        } => {
            let displaced = RetainedArtifact {
                relative: intent.staging_relative.clone(),
                identity: ArtifactIdentity {
                    device: fingerprint.device,
                    inode: fingerprint.inode,
                    len: fingerprint.len,
                    kind: ArtifactKind::Regular,
                },
                sha256: Some(fingerprint.sha256),
            };
            store
                .record_note_displaced(intent.request.operation_id, &displaced)
                .unwrap();
            NoteVerification::Replace {
                installed: prepared.fingerprint,
                displaced,
                displaced_bytes: baseline_text.as_bytes().to_vec(),
            }
        }
        DestinationPrecondition::Absent { .. } => NoteVerification::Copy {
            installed: prepared.fingerprint,
        },
    }
}

fn receipt(intent: &NoteSaveIntent) -> NoteReceipt {
    NoteReceipt {
        operation_id: intent.request.operation_id,
        source_note_id: intent.request.note_id,
        note_id: intent.target_note_id,
        submitted_generation: intent.request.generation,
        stamp: NoteStamp {
            file_state: Uuid::new_v4(),
            generation: intent.request.generation,
        },
        filesystem_outcome: FileOutcome::Applied,
        recovery_available: true,
    }
}

#[test]
fn verified_completion_rebases_but_never_overwrites_later_edits() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    let verification = prepare(&mut store, &intent);
    store
        .record_note_verification(request.operation_id, &verification)
        .unwrap();
    let mut later = submission(&store.note_recovery(opened.note_id).unwrap().unwrap());
    later.generation = 9;
    later.text = "later 🌍\r\n".into();
    store.save_note_buffer(&later).unwrap();
    let receipt = receipt(&intent);
    store
        .finish_note_save(request.operation_id, &receipt)
        .unwrap();
    store
        .finish_note_save(request.operation_id, &receipt)
        .unwrap();
    let recovered = store.note_recovery(opened.note_id).unwrap().unwrap();
    assert_eq!(recovered.working, "later 🌍\r\n");
    assert_eq!(recovered.baseline, "updated\r\n");
    assert_eq!(recovered.stamp.generation, 9);
    assert_eq!(recovered.stamp.file_state, receipt.stamp.file_state);
    assert_eq!(
        store
            .note_write_result(&request, Path::new("plan.md"), NoteWriteKind::Replace)
            .unwrap(),
        Some(NoteRecordedResult::Receipt(receipt))
    );
}

#[test]
fn applied_copy_uses_reserved_identity_and_does_not_acknowledge_source() {
    let (data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let conn = rusqlite::Connection::open(store.database_path()).unwrap();
    conn.execute("UPDATE notes SET approval='approved'", [])
        .unwrap();
    let intent = store
        .begin_note_save(
            &request,
            Path::new("copy.md"),
            NoteWriteKind::Copy,
            &DestinationPrecondition::Absent {
                parent: VaultIdentity {
                    device: 1,
                    inode: 2,
                },
            },
        )
        .unwrap();
    let verification = prepare(&mut store, &intent);
    let receipt = receipt(&intent);
    drop(store);
    let (mut store, _) = Store::open(data.path()).unwrap();
    let record = NoteReconciliation {
        resolution: NoteResolution::Applied,
        observed_destination: Some(fingerprint(&request.text, 4)),
        verification: Some(verification),
        result: NoteRecordedResult::Receipt(receipt.clone()),
    };
    store
        .reconcile_note_operation(request.operation_id, &record)
        .unwrap();
    assert_eq!(
        store
            .note_recovery(intent.target_note_id)
            .unwrap()
            .unwrap()
            .baseline,
        request.text
    );
    let source = store.note_recovery(opened.note_id).unwrap().unwrap();
    assert_eq!(source.baseline, opened.baseline);
    assert_eq!(source.stamp.file_state, opened.stamp.file_state);
    assert_ne!(receipt.note_id, receipt.source_note_id);
    assert_eq!(
        store
            .note_record(intent.target_note_id)
            .unwrap()
            .search_approval,
        brn_store::Approval::Draft
    );
    assert_eq!(
        store.note_record(opened.note_id).unwrap().search_approval,
        brn_store::Approval::Approved
    );
}

#[test]
fn wrong_verification_and_accepted_current_never_resolve_an_intent() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    let mut verification = prepare(&mut store, &intent);
    if let NoteVerification::Replace {
        displaced_bytes, ..
    } = &mut verification
    {
        *displaced_bytes = vec![0xff];
    }
    let failure = store
        .record_note_verification(request.operation_id, &verification)
        .unwrap_err();
    assert_eq!(failure.code, NoteErrorCode::Conflict);
    assert_eq!(failure.phase, Some(SavePhase::Exchanged));
    assert_eq!(failure.filesystem_outcome, FileOutcome::Unknown);
    let record = NoteReconciliation {
        resolution: NoteResolution::AcceptedCurrent,
        observed_destination: None,
        verification: None,
        result: NoteRecordedResult::Receipt(receipt(&intent)),
    };
    assert!(
        store
            .reconcile_note_operation(request.operation_id, &record)
            .is_err()
    );
    assert_eq!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap()
            .resolution,
        NoteResolution::Unresolved
    );
}

#[test]
fn corrupt_receipt_hash_and_buffer_bytes_fail_visibly() {
    for flavor in ["receipt", "buffer", "utf8", "schema"] {
        let (data, _vault, mut store, opened) = fixture();
        let request = submission(&opened);
        let failure = NoteFailure {
            code: NoteErrorCode::Conflict,
            message: "changed".into(),
            operation_id: Some(request.operation_id),
            note_id: Some(request.note_id),
            phase: None,
            filesystem_outcome: FileOutcome::NotApplied,
            recovery_available: false,
        };
        store
            .record_note_write_failure(
                &request,
                Path::new("plan.md"),
                NoteWriteKind::Replace,
                &failure,
            )
            .unwrap();
        let db = store.database_path().to_owned();
        drop(store);
        let conn = rusqlite::Connection::open(&db).unwrap();
        match flavor {
            "receipt" => {
                conn.execute("UPDATE note_receipts SET result_sha256=zeroblob(32)", [])
                    .unwrap();
            }
            "buffer" => {
                conn.execute("UPDATE note_buffers SET working=x'ff'", [])
                    .unwrap();
            }
            "utf8" => {
                conn.execute(
                    "UPDATE note_buffers SET working=x'ff',working_sha256=?1",
                    [Sha256::digest([0xff]).as_slice()],
                )
                .unwrap();
            }
            "schema" => {
                conn.execute_batch("DROP INDEX one_unresolved_original_save;")
                    .unwrap();
            }
            _ => unreachable!(),
        }
        drop(conn);
        if flavor == "schema" {
            assert!(Store::open(data.path()).is_err());
        } else {
            let (store, _) = Store::open(data.path()).unwrap();
            let error = if flavor == "receipt" {
                store
                    .note_write_result(&request, Path::new("plan.md"), NoteWriteKind::Replace)
                    .unwrap_err()
            } else {
                store.note_recovery(opened.note_id).unwrap_err()
            };
            assert_eq!(error.code, NoteErrorCode::Storage);
        }
    }
}

#[test]
fn oversized_submission_and_invalid_stamp_refusal_do_not_acknowledge_recovery() {
    let (_data, _vault, mut store, opened) = fixture();
    let mut request = submission(&opened);
    request.text = "🌍".repeat(262_145);
    assert_eq!(
        store.save_note_buffer(&request).unwrap_err().code,
        NoteErrorCode::Unsupported
    );
    request.text = "unsafe".into();
    request.expected.file_state = Uuid::new_v4();
    let failure = NoteFailure {
        code: NoteErrorCode::Conflict,
        message: "changed".into(),
        operation_id: Some(request.operation_id),
        note_id: Some(request.note_id),
        phase: None,
        filesystem_outcome: FileOutcome::NotApplied,
        recovery_available: false,
    };
    let error = store
        .record_note_write_failure(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &failure,
        )
        .unwrap_err();
    assert_eq!(error.code, NoteErrorCode::Storage);
    assert!(error.message.contains(&failure.message));
    assert_eq!(error.phase, failure.phase);
    assert_eq!(error.filesystem_outcome, failure.filesystem_outcome);
    assert!(!error.recovery_available);
    assert!(store.operation(request.operation_id).unwrap().is_none());
    assert_eq!(
        store
            .note_recovery(opened.note_id)
            .unwrap()
            .unwrap()
            .working,
        opened.working
    );
}

#[test]
fn completed_noop_payloads_retire_but_receipts_buffer_and_unresolved_survive() {
    let (_data, _vault, mut store, opened) = fixture();
    let mut completed = Vec::new();
    for _ in 0..2 {
        let request = NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: opened.note_id,
            expected: opened.stamp,
            generation: 0,
            text: opened.working.clone(),
        };
        store
            .begin_note_save(
                &request,
                Path::new("plan.md"),
                NoteWriteKind::Replace,
                &precondition(&opened),
            )
            .unwrap();
        let receipt = NoteReceipt {
            operation_id: request.operation_id,
            source_note_id: request.note_id,
            note_id: request.note_id,
            submitted_generation: 0,
            stamp: opened.stamp,
            filesystem_outcome: FileOutcome::NotApplied,
            recovery_available: true,
        };
        store
            .finish_note_save(request.operation_id, &receipt)
            .unwrap();
        completed.push(request);
    }
    let pending = submission(&opened);
    store
        .begin_note_save(
            &pending,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    store.prune_completed_note_payloads(opened.note_id).unwrap();
    assert!(
        store
            .note_save_result(completed[0].operation_id)
            .unwrap()
            .is_some()
    );
    assert!(store.note_save_result(Uuid::new_v4()).unwrap().is_none());
    assert_eq!(store.note_save_result(pending.operation_id).unwrap(), None,);
    assert!(
        store
            .note_save_intent(completed[0].operation_id)
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .note_write_result(&completed[0], Path::new("plan.md"), NoteWriteKind::Replace)
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .note_save_intent(completed[1].operation_id)
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .note_save_intent(pending.operation_id)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        store
            .note_recovery(opened.note_id)
            .unwrap()
            .unwrap()
            .working,
        pending.text
    );
}

#[test]
fn unknown_failure_receipt_keeps_original_write_blocked_after_restart_and_pruning() {
    let (data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    prepare(&mut store, &intent);
    let failure = NoteFailure {
        code: NoteErrorCode::SaveUncertain,
        message: "durability unknown".into(),
        operation_id: Some(request.operation_id),
        note_id: Some(request.note_id),
        phase: Some(SavePhase::Exchanged),
        filesystem_outcome: FileOutcome::Unknown,
        recovery_available: false,
    };
    let acknowledged = store
        .record_note_write_failure(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &failure,
        )
        .unwrap();
    drop(store);
    let (mut store, _) = Store::open(data.path()).unwrap();
    store.prune_completed_note_payloads(opened.note_id).unwrap();
    let value = store
        .note_save_intent(request.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(value.resolution, NoteResolution::Unresolved);
    assert_eq!(
        value.prior_result,
        Some(NoteRecordedResult::Failure(acknowledged))
    );
    let next = submission(&store.note_recovery(opened.note_id).unwrap().unwrap());
    assert_eq!(
        store
            .begin_note_save(
                &next,
                Path::new("plan.md"),
                NoteWriteKind::Replace,
                &precondition(&opened)
            )
            .unwrap_err()
            .code,
        NoteErrorCode::SaveUncertain
    );
}

#[test]
fn forged_failure_outcome_cannot_resolve_without_matching_proof() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    let error = NoteFailure {
        code: NoteErrorCode::SaveUncertain,
        message: "unknown".into(),
        operation_id: Some(request.operation_id),
        note_id: Some(request.note_id),
        phase: Some(SavePhase::Intent),
        filesystem_outcome: FileOutcome::Unknown,
        recovery_available: true,
    };
    let record = NoteReconciliation {
        resolution: NoteResolution::NotApplied,
        observed_destination: Some(fingerprint(&opened.baseline, 3)),
        verification: None,
        result: NoteRecordedResult::Failure(error),
    };
    assert!(
        store
            .reconcile_note_operation(request.operation_id, &record)
            .is_err()
    );
    assert_eq!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap()
            .resolution,
        NoteResolution::Unresolved
    );
}

#[test]
fn enrollment_rejects_data_directory_inside_vault_and_different_vault_identity() {
    let vault = tempfile::tempdir_in(".").unwrap();
    let data = tempfile::tempdir_in(vault.path()).unwrap();
    let (mut store, _) = Store::open(data.path()).unwrap();
    let record = VaultRecord {
        id: Uuid::new_v4(),
        root: vault.path().canonicalize().unwrap(),
        identity: VaultIdentity {
            device: 1,
            inode: 2,
        },
    };
    assert_eq!(
        store
            .enroll_note(
                Uuid::new_v4(),
                &record,
                Path::new("plan.md"),
                fingerprint("", 3),
                ""
            )
            .unwrap_err()
            .code,
        NoteErrorCode::Unsupported
    );

    let (_data, _vault, mut store, opened) = fixture();
    let mut record = store.note_vault(opened.note_id).unwrap();
    record.identity.inode = 99;
    assert_eq!(
        store
            .enroll_note(
                Uuid::new_v4(),
                &record,
                Path::new("other.md"),
                fingerprint("", 3),
                ""
            )
            .unwrap_err()
            .code,
        NoteErrorCode::VaultUnavailable
    );
    assert_eq!(
        store.note_recovery(opened.note_id).unwrap().unwrap(),
        opened
    );
}

#[test]
fn historical_failure_reconciliation_preserves_result_and_rebases_known_applied_save() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    let verification = prepare(&mut store, &intent);
    let error = NoteFailure {
        code: NoteErrorCode::SaveUncertain,
        message: "commit interrupted".into(),
        operation_id: Some(request.operation_id),
        note_id: Some(request.note_id),
        phase: Some(SavePhase::Exchanged),
        filesystem_outcome: FileOutcome::Unknown,
        recovery_available: false,
    };
    let ack = store
        .record_note_write_failure(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &error,
        )
        .unwrap();
    let record = NoteReconciliation {
        resolution: NoteResolution::Applied,
        observed_destination: Some(fingerprint(&request.text, 4)),
        verification: Some(verification),
        result: NoteRecordedResult::Failure(ack.clone()),
    };
    assert_eq!(
        store
            .reconcile_note_operation(request.operation_id, &record)
            .unwrap(),
        NoteRecordedResult::Failure(ack.clone())
    );
    assert_eq!(
        store
            .note_write_result(&request, Path::new("plan.md"), NoteWriteKind::Replace)
            .unwrap(),
        Some(NoteRecordedResult::Failure(ack))
    );
    let recovered = store.note_recovery(opened.note_id).unwrap().unwrap();
    assert_eq!(recovered.baseline, request.text);
    assert_ne!(recovered.stamp.file_state, opened.stamp.file_state);
    assert_eq!(recovered.stamp.generation, 1);
    assert_eq!(
        recovery_pair(&store, opened.note_id),
        Some((
            request.operation_id,
            opened.baseline.as_bytes().to_vec(),
            request.text.as_bytes().to_vec(),
        ))
    );
}

fn recovery_pair(store: &Store, note: Uuid) -> Option<(Uuid, Vec<u8>, Vec<u8>)> {
    use rusqlite::OptionalExtension;
    let conn = rusqlite::Connection::open(store.database_path()).unwrap();
    conn.query_row(
        "SELECT operation_id,baseline,submitted FROM note_recovery_pairs WHERE note_id=?1",
        [note.to_string()],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, Vec<u8>>(2)?,
            ))
        },
    )
    .optional()
    .unwrap()
    .map(|(op, baseline, submitted)| (Uuid::parse_str(&op).unwrap(), baseline, submitted))
}

#[test]
fn enrollment_cannot_adopt_a_reserved_copy_after_startup_interruption() {
    let (data, _vault, mut store, opened) = fixture();
    let vault = store.note_vault(opened.note_id).unwrap();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("copy.md"),
            NoteWriteKind::Copy,
            &DestinationPrecondition::Absent {
                parent: vault.identity.clone(),
            },
        )
        .unwrap();
    let verification = prepare(&mut store, &intent);
    drop(store);
    let (mut store, report) = Store::open(data.path()).unwrap();
    assert_eq!(report.interrupted_operations, 1);
    let enrollment = Uuid::new_v4();
    let error = store
        .enroll_note(
            enrollment,
            &vault,
            Path::new("copy.md"),
            fingerprint(&request.text, 4),
            &request.text,
        )
        .unwrap_err();
    assert_eq!(error.code, NoteErrorCode::Conflict);
    assert!(store.operation(enrollment).unwrap().is_none());
    assert!(
        store
            .note_recovery(intent.target_note_id)
            .unwrap()
            .is_none()
    );
    let receipt = receipt(&intent);
    store
        .reconcile_note_operation(
            request.operation_id,
            &NoteReconciliation {
                resolution: NoteResolution::Applied,
                observed_destination: Some(fingerprint(&request.text, 4)),
                verification: Some(verification),
                result: NoteRecordedResult::Receipt(receipt),
            },
        )
        .unwrap();
    assert_eq!(
        store
            .note_recovery(intent.target_note_id)
            .unwrap()
            .unwrap()
            .working,
        request.text
    );
    let reopened = store
        .enroll_note(
            Uuid::new_v4(),
            &vault,
            Path::new("copy.md"),
            fingerprint(&request.text, 4),
            &request.text,
        )
        .unwrap();
    assert_eq!(reopened.note_id, intent.target_note_id);
}

#[test]
fn failed_refusal_recording_is_storage_with_original_context_and_no_mutation() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    prepare(&mut store, &intent);
    let before = store
        .note_save_intent(request.operation_id)
        .unwrap()
        .unwrap();
    let failure = NoteFailure {
        code: NoteErrorCode::Conflict,
        message: "late conflict after exchange".into(),
        operation_id: Some(request.operation_id),
        note_id: Some(opened.note_id),
        phase: Some(SavePhase::Exchanged),
        filesystem_outcome: FileOutcome::Unknown,
        recovery_available: false,
    };
    let conn = rusqlite::Connection::open(store.database_path()).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_note_receipt BEFORE INSERT ON note_receipts BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END;").unwrap();
    let error = store
        .record_note_write_failure(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &failure,
        )
        .unwrap_err();
    assert_eq!(error.code, NoteErrorCode::Storage);
    assert!(error.message.contains(&failure.message));
    assert!(error.message.contains("injected receipt failure"));
    assert_eq!(error.phase, Some(SavePhase::Exchanged));
    assert_eq!(error.filesystem_outcome, FileOutcome::Unknown);
    assert!(!error.recovery_available);
    assert_eq!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap(),
        before
    );
    assert!(
        store
            .note_write_result(&request, Path::new("plan.md"), NoteWriteKind::Replace)
            .unwrap()
            .is_none()
    );
    conn.execute_batch("DROP TRIGGER reject_note_receipt;")
        .unwrap();
}

#[test]
fn refused_payload_recording_reports_storage_for_operation_conflicts_and_unsupported_paths() {
    for conflict in [false, true] {
        let (_data, _vault, mut store, opened) = fixture();
        let request = submission(&opened);
        if conflict {
            store
                .create_source(request.operation_id, "unrelated operation")
                .unwrap();
        }
        let before = store.operation(request.operation_id).unwrap();
        let refusal = NoteFailure {
            code: NoteErrorCode::VaultUnavailable,
            message: "the selected vault is unavailable".into(),
            operation_id: Some(request.operation_id),
            note_id: Some(opened.note_id),
            phase: None,
            filesystem_outcome: FileOutcome::NotApplied,
            recovery_available: false,
        };
        let path = if conflict {
            Path::new("plan.md")
        } else {
            Path::new("../plan.md")
        };
        let error = store
            .record_note_write_failure(&request, path, NoteWriteKind::Replace, &refusal)
            .unwrap_err();
        assert_eq!(error.code, NoteErrorCode::Storage);
        assert!(error.message.contains(&refusal.message));
        assert_eq!(error.phase, refusal.phase);
        assert_eq!(error.filesystem_outcome, refusal.filesystem_outcome);
        assert!(!error.recovery_available);
        assert_eq!(store.operation(request.operation_id).unwrap(), before);
        assert_eq!(
            store.note_recovery(opened.note_id).unwrap().unwrap(),
            opened
        );
    }
}

#[test]
fn noop_and_reconciled_not_applied_preserve_the_last_applied_recovery_pair() {
    let (data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    let verification = prepare(&mut store, &intent);
    store
        .record_note_verification(request.operation_id, &verification)
        .unwrap();
    let saved_receipt = receipt(&intent);
    store
        .finish_note_save(request.operation_id, &saved_receipt)
        .unwrap();
    store
        .record_note_cleanup(request.operation_id, ArtifactCleanup::Retired)
        .unwrap();
    let expected_pair = Some((
        request.operation_id,
        b"\xef\xbb\xbf# plan\r\n".to_vec(),
        b"updated\r\n".to_vec(),
    ));
    assert_eq!(recovery_pair(&store, opened.note_id), expected_pair);
    let recovered = store.note_recovery(opened.note_id).unwrap().unwrap();
    let expected = DestinationPrecondition::Existing {
        fingerprint: fingerprint(&recovered.baseline, 4),
        baseline_text: recovered.baseline.clone(),
    };
    let mut noop = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.note_id,
        expected: recovered.stamp,
        generation: 1,
        text: recovered.working.clone(),
    };
    store
        .begin_note_save(
            &noop,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &expected,
        )
        .unwrap();
    store
        .finish_note_save(
            noop.operation_id,
            &NoteReceipt {
                operation_id: noop.operation_id,
                source_note_id: noop.note_id,
                note_id: noop.note_id,
                submitted_generation: 1,
                stamp: recovered.stamp,
                filesystem_outcome: FileOutcome::NotApplied,
                recovery_available: true,
            },
        )
        .unwrap();
    assert_eq!(
        recovery_pair(&store, opened.note_id),
        expected_pair,
        "no-op cannot replace the real pre-save bytes"
    );
    store.prune_completed_note_payloads(opened.note_id).unwrap();
    assert!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .is_some()
    );

    noop.operation_id = Uuid::new_v4();
    noop.generation = 2;
    noop.text = "never installed\r\n".into();
    store
        .begin_note_save(
            &noop,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &expected,
        )
        .unwrap();
    drop(store);
    let (mut store, _) = Store::open(data.path()).unwrap();
    store
        .reconcile_note_operation(
            noop.operation_id,
            &NoteReconciliation {
                resolution: NoteResolution::NotApplied,
                observed_destination: Some(fingerprint("updated\r\n", 4)),
                verification: None,
                result: NoteRecordedResult::Receipt(NoteReceipt {
                    operation_id: noop.operation_id,
                    source_note_id: noop.note_id,
                    note_id: noop.note_id,
                    submitted_generation: 2,
                    stamp: NoteStamp {
                        file_state: recovered.stamp.file_state,
                        generation: 2,
                    },
                    filesystem_outcome: FileOutcome::NotApplied,
                    recovery_available: true,
                }),
            },
        )
        .unwrap();
    assert_eq!(
        recovery_pair(&store, opened.note_id),
        expected_pair,
        "unsaved input cannot become the successful-save pair"
    );
    store
        .record_note_cleanup(noop.operation_id, ArtifactCleanup::Retired)
        .unwrap();
    store.prune_completed_note_payloads(opened.note_id).unwrap();
    assert!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        store
            .note_recovery(opened.note_id)
            .unwrap()
            .unwrap()
            .working,
        "never installed\r\n"
    );
}

#[test]
fn applied_historical_failure_advances_the_successful_save_pair() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let first = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    let verification = prepare(&mut store, &first);
    store
        .record_note_verification(request.operation_id, &verification)
        .unwrap();
    store
        .finish_note_save(request.operation_id, &receipt(&first))
        .unwrap();
    let recovered = store.note_recovery(opened.note_id).unwrap().unwrap();
    let second_request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.note_id,
        expected: recovered.stamp,
        generation: 2,
        text: "applied after uncertainty\r\n".into(),
    };
    let second = store
        .begin_note_save(
            &second_request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &DestinationPrecondition::Existing {
                fingerprint: fingerprint(&recovered.baseline, 4),
                baseline_text: recovered.baseline,
            },
        )
        .unwrap();
    let verification = prepare(&mut store, &second);
    let failure = NoteFailure {
        code: NoteErrorCode::SaveUncertain,
        message: "completion initially uncertain".into(),
        operation_id: Some(second_request.operation_id),
        note_id: Some(opened.note_id),
        phase: Some(SavePhase::Exchanged),
        filesystem_outcome: FileOutcome::Unknown,
        recovery_available: false,
    };
    let failure = store
        .record_note_write_failure(
            &second_request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &failure,
        )
        .unwrap();
    let record = NoteReconciliation {
        resolution: NoteResolution::Applied,
        observed_destination: Some(fingerprint(&second_request.text, 5)),
        verification: Some(verification),
        result: NoteRecordedResult::Failure(failure.clone()),
    };
    store
        .reconcile_note_operation(second_request.operation_id, &record)
        .unwrap();
    assert_eq!(
        recovery_pair(&store, opened.note_id),
        Some((
            second_request.operation_id,
            b"updated\r\n".to_vec(),
            b"applied after uncertainty\r\n".to_vec(),
        ))
    );
    assert_eq!(
        store
            .note_write_result(
                &second_request,
                Path::new("plan.md"),
                NoteWriteKind::Replace
            )
            .unwrap(),
        Some(NoteRecordedResult::Failure(failure))
    );
}

#[test]
fn cleanup_retirement_requires_resolved_known_terminal_result() {
    for scenario in [
        "unresolved",
        "accepted_current",
        "unknown",
        "uncertain",
        "no_result",
    ] {
        let (_data, _vault, mut store, opened) = fixture();
        let request = submission(&opened);
        let intent = store
            .begin_note_save(
                &request,
                Path::new("plan.md"),
                NoteWriteKind::Replace,
                &precondition(&opened),
            )
            .unwrap();
        if scenario != "unresolved" {
            let verification = prepare(&mut store, &intent);
            store
                .record_note_verification(request.operation_id, &verification)
                .unwrap();
            store
                .finish_note_save(request.operation_id, &receipt(&intent))
                .unwrap();
            let conn = rusqlite::Connection::open(store.database_path()).unwrap();
            if scenario == "accepted_current" {
                let mut value = store
                    .note_save_intent(request.operation_id)
                    .unwrap()
                    .unwrap();
                value.resolution = NoteResolution::AcceptedCurrent;
                value.acknowledged_by = Some(Uuid::new_v4());
                value.prior_result = None;
                let bytes = serde_json::to_vec(&value).unwrap();
                conn.execute("UPDATE note_save_intents SET resolution='accepted_current',intent_json=?2,intent_sha256=?3 WHERE operation_id=?1",
                    rusqlite::params![request.operation_id.to_string(), bytes, Sha256::digest(&bytes).as_slice()]).unwrap();
            } else if scenario == "no_result" {
                conn.execute(
                    "DELETE FROM note_receipts WHERE operation_id=?1",
                    [request.operation_id.to_string()],
                )
                .unwrap();
            } else {
                let result = NoteRecordedResult::Failure(NoteFailure {
                    code: if scenario == "uncertain" {
                        NoteErrorCode::SaveUncertain
                    } else {
                        NoteErrorCode::Io
                    },
                    message: "preserved original failure".into(),
                    operation_id: Some(request.operation_id),
                    note_id: Some(request.note_id),
                    phase: Some(SavePhase::Exchanged),
                    filesystem_outcome: if scenario == "unknown" {
                        FileOutcome::Unknown
                    } else {
                        FileOutcome::Applied
                    },
                    recovery_available: true,
                });
                let bytes = serde_json::to_vec(&result).unwrap();
                conn.execute("UPDATE note_receipts SET result_json=?2,result_sha256=?3 WHERE operation_id=?1",
                    rusqlite::params![request.operation_id.to_string(), bytes, Sha256::digest(&bytes).as_slice()]).unwrap();
            }
        }
        let before = store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap();
        let error = store
            .record_note_cleanup(request.operation_id, ArtifactCleanup::Retired)
            .unwrap_err();
        assert!(matches!(
            error.code,
            NoteErrorCode::StateChanged | NoteErrorCode::SaveUncertain
        ));
        assert_eq!(error.operation_id, Some(request.operation_id));
        assert_eq!(
            store
                .note_save_intent(request.operation_id)
                .unwrap()
                .unwrap(),
            before,
            "{scenario}"
        );
        store
            .record_note_cleanup(request.operation_id, ArtifactCleanup::RetainedUnexpected)
            .unwrap();
        store
            .record_note_cleanup(request.operation_id, ArtifactCleanup::RetainedUnexpected)
            .unwrap();
        let retained = store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(retained.cleanup, ArtifactCleanup::RetainedUnexpected);
        assert!(
            store
                .record_note_cleanup(request.operation_id, ArtifactCleanup::Pending)
                .is_err()
        );
        assert!(
            store
                .record_note_cleanup(request.operation_id, ArtifactCleanup::Retired)
                .is_err()
        );
        assert_eq!(
            store
                .note_save_intent(request.operation_id)
                .unwrap()
                .unwrap(),
            retained
        );
    }
}

#[test]
fn applied_cleanup_is_monotonic_and_superseded_payloads_retire_without_receipts() {
    let (data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let first = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    store
        .record_note_cleanup(request.operation_id, ArtifactCleanup::Pending)
        .unwrap();
    let verification = prepare(&mut store, &first);
    store
        .record_note_verification(request.operation_id, &verification)
        .unwrap();
    let first_receipt = receipt(&first);
    store
        .finish_note_save(request.operation_id, &first_receipt)
        .unwrap();
    store
        .record_note_cleanup(request.operation_id, ArtifactCleanup::Retired)
        .unwrap();
    store
        .record_note_cleanup(request.operation_id, ArtifactCleanup::Retired)
        .unwrap();
    assert!(
        store
            .record_note_cleanup(request.operation_id, ArtifactCleanup::Pending)
            .is_err()
    );
    assert!(
        store
            .record_note_cleanup(request.operation_id, ArtifactCleanup::RetainedUnexpected)
            .is_err()
    );
    store.prune_completed_note_payloads(opened.note_id).unwrap();
    assert!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .is_some(),
        "latest pair remains protected"
    );

    let recovered = store.note_recovery(opened.note_id).unwrap().unwrap();
    let second_request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.note_id,
        expected: recovered.stamp,
        generation: 2,
        text: "newest\r\n".into(),
    };
    let second = store
        .begin_note_save(
            &second_request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &DestinationPrecondition::Existing {
                fingerprint: fingerprint(&recovered.baseline, 4),
                baseline_text: recovered.baseline.clone(),
            },
        )
        .unwrap();
    let second_verification = prepare(&mut store, &second);
    store
        .record_note_verification(second_request.operation_id, &second_verification)
        .unwrap();
    let second_receipt = receipt(&second);
    store
        .finish_note_save(second_request.operation_id, &second_receipt)
        .unwrap();
    store
        .record_note_cleanup(second_request.operation_id, ArtifactCleanup::Retired)
        .unwrap();
    store.prune_completed_note_payloads(opened.note_id).unwrap();
    assert!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .note_save_intent(second_request.operation_id)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        store
            .note_write_result(&request, Path::new("plan.md"), NoteWriteKind::Replace)
            .unwrap(),
        Some(NoteRecordedResult::Receipt(first_receipt.clone()))
    );
    let conn = rusqlite::Connection::open(store.database_path()).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM note_write_inputs WHERE operation_id=?1",
            [request.operation_id.to_string()],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row(
            "SELECT operation_id FROM note_recovery_pairs WHERE note_id=?1",
            [opened.note_id.to_string()],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        second_request.operation_id.to_string()
    );
    drop(conn);
    drop(store);
    let (store, _) = Store::open(data.path()).unwrap();
    assert_eq!(
        store
            .note_write_result(&request, Path::new("plan.md"), NoteWriteKind::Replace)
            .unwrap(),
        Some(NoteRecordedResult::Receipt(first_receipt))
    );
    let recovered = store.note_recovery(opened.note_id).unwrap().unwrap();
    assert_eq!(recovered.working, "newest\r\n");
    assert_eq!(recovered.baseline, "newest\r\n");
    assert_eq!(recovered.stamp, second_receipt.stamp);
}

#[test]
fn proven_not_applied_cleanup_can_retire_after_preparing_an_artifact() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let intent = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    store
        .record_note_prepared(
            request.operation_id,
            &PreparedFile {
                relative: intent.staging_relative,
                fingerprint: fingerprint(&request.text, 4),
            },
        )
        .unwrap();
    let result = NoteRecordedResult::Receipt(NoteReceipt {
        operation_id: request.operation_id,
        source_note_id: opened.note_id,
        note_id: opened.note_id,
        submitted_generation: 1,
        stamp: NoteStamp {
            file_state: opened.stamp.file_state,
            generation: 1,
        },
        filesystem_outcome: FileOutcome::NotApplied,
        recovery_available: true,
    });
    store
        .reconcile_note_operation(
            request.operation_id,
            &NoteReconciliation {
                resolution: NoteResolution::NotApplied,
                observed_destination: Some(fingerprint(&opened.baseline, 3)),
                verification: None,
                result,
            },
        )
        .unwrap();
    assert_eq!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap()
            .cleanup,
        ArtifactCleanup::Pending
    );
    store
        .record_note_cleanup(request.operation_id, ArtifactCleanup::Retired)
        .unwrap();
    assert_eq!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap()
            .cleanup,
        ArtifactCleanup::Retired
    );
}

#[test]
fn pruning_keeps_retired_payloads_referenced_by_unresolved_copy() {
    let (_data, _vault, mut store, opened) = fixture();
    let request = submission(&opened);
    let first = store
        .begin_note_save(
            &request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &precondition(&opened),
        )
        .unwrap();
    let verification = prepare(&mut store, &first);
    store
        .record_note_verification(request.operation_id, &verification)
        .unwrap();
    store
        .finish_note_save(request.operation_id, &receipt(&first))
        .unwrap();
    store
        .record_note_cleanup(request.operation_id, ArtifactCleanup::Retired)
        .unwrap();
    let recovered = store.note_recovery(opened.note_id).unwrap().unwrap();
    let copy_request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.note_id,
        expected: recovered.stamp,
        generation: recovered.stamp.generation,
        text: recovered.working.clone(),
    };
    store
        .begin_note_save(
            &copy_request,
            Path::new("copy.md"),
            NoteWriteKind::Copy,
            &DestinationPrecondition::Absent {
                parent: VaultIdentity {
                    device: 1,
                    inode: 2,
                },
            },
        )
        .unwrap();
    let second_request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.note_id,
        expected: recovered.stamp,
        generation: 2,
        text: "newest\r\n".into(),
    };
    let second = store
        .begin_note_save(
            &second_request,
            Path::new("plan.md"),
            NoteWriteKind::Replace,
            &DestinationPrecondition::Existing {
                fingerprint: fingerprint(&recovered.baseline, 4),
                baseline_text: recovered.baseline.clone(),
            },
        )
        .unwrap();
    let verification = prepare(&mut store, &second);
    store
        .record_note_verification(second_request.operation_id, &verification)
        .unwrap();
    store
        .finish_note_save(second_request.operation_id, &receipt(&second))
        .unwrap();
    store.prune_completed_note_payloads(opened.note_id).unwrap();
    assert!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .is_some(),
        "unresolved copy protects its starting file state"
    );

    let result = NoteRecordedResult::Failure(NoteFailure {
        code: NoteErrorCode::Io,
        message: "copy refused before staging".into(),
        operation_id: Some(copy_request.operation_id),
        note_id: Some(opened.note_id),
        phase: Some(SavePhase::Intent),
        filesystem_outcome: FileOutcome::NotApplied,
        recovery_available: true,
    });
    store
        .reconcile_note_operation(
            copy_request.operation_id,
            &NoteReconciliation {
                resolution: NoteResolution::NotApplied,
                observed_destination: None,
                verification: None,
                result,
            },
        )
        .unwrap();
    store.prune_completed_note_payloads(opened.note_id).unwrap();
    assert!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .is_none()
    );
}

#[test]
fn reconciliation_never_overwrites_unexpected_or_uncertain_cleanup_disposition() {
    for uncertain in [false, true] {
        let (_data, _vault, mut store, opened) = fixture();
        let request = submission(&opened);
        store
            .begin_note_save(
                &request,
                Path::new("plan.md"),
                NoteWriteKind::Replace,
                &precondition(&opened),
            )
            .unwrap();
        let result = if uncertain {
            let error = NoteFailure {
                code: NoteErrorCode::SaveUncertain,
                message: "unproven artifact".into(),
                operation_id: Some(request.operation_id),
                note_id: Some(request.note_id),
                phase: Some(SavePhase::Intent),
                filesystem_outcome: FileOutcome::Unknown,
                recovery_available: false,
            };
            NoteRecordedResult::Failure(
                store
                    .record_note_write_failure(
                        &request,
                        Path::new("plan.md"),
                        NoteWriteKind::Replace,
                        &error,
                    )
                    .unwrap(),
            )
        } else {
            store
                .record_note_cleanup(request.operation_id, ArtifactCleanup::RetainedUnexpected)
                .unwrap();
            NoteRecordedResult::Receipt(NoteReceipt {
                operation_id: request.operation_id,
                source_note_id: request.note_id,
                note_id: request.note_id,
                submitted_generation: 1,
                stamp: NoteStamp {
                    file_state: opened.stamp.file_state,
                    generation: 1,
                },
                filesystem_outcome: FileOutcome::NotApplied,
                recovery_available: true,
            })
        };
        store
            .reconcile_note_operation(
                request.operation_id,
                &NoteReconciliation {
                    resolution: NoteResolution::NotApplied,
                    observed_destination: Some(fingerprint(&opened.baseline, 3)),
                    verification: None,
                    result,
                },
            )
            .unwrap();
        assert_eq!(
            store
                .note_save_intent(request.operation_id)
                .unwrap()
                .unwrap()
                .cleanup,
            if uncertain {
                ArtifactCleanup::Pending
            } else {
                ArtifactCleanup::RetainedUnexpected
            }
        );
        assert!(
            store
                .record_note_cleanup(request.operation_id, ArtifactCleanup::Retired)
                .is_err()
        );
    }
}

#[test]
fn original_or_absence_observation_cannot_undo_a_recorded_exchange() {
    for kind in [NoteWriteKind::Replace, NoteWriteKind::Copy] {
        let (_data, _vault, mut store, opened) = fixture();
        let request = submission(&opened);
        let path = if kind == NoteWriteKind::Replace {
            Path::new("plan.md")
        } else {
            Path::new("copy.md")
        };
        let expected = if kind == NoteWriteKind::Replace {
            precondition(&opened)
        } else {
            DestinationPrecondition::Absent {
                parent: VaultIdentity {
                    device: 1,
                    inode: 2,
                },
            }
        };
        let intent = store
            .begin_note_save(&request, path, kind, &expected)
            .unwrap();
        prepare(&mut store, &intent);
        let result = NoteRecordedResult::Failure(NoteFailure {
            code: NoteErrorCode::Io,
            message: "observed original or absence".into(),
            operation_id: Some(request.operation_id),
            note_id: Some(opened.note_id),
            phase: Some(SavePhase::Exchanged),
            filesystem_outcome: FileOutcome::NotApplied,
            recovery_available: true,
        });
        let record = NoteReconciliation {
            resolution: NoteResolution::NotApplied,
            observed_destination: if kind == NoteWriteKind::Replace {
                Some(fingerprint(&opened.baseline, 3))
            } else {
                None
            },
            verification: None,
            result,
        };
        assert_eq!(
            store
                .reconcile_note_operation(request.operation_id, &record)
                .unwrap_err()
                .code,
            NoteErrorCode::SaveUncertain
        );
        assert_eq!(
            store
                .note_save_intent(request.operation_id)
                .unwrap()
                .unwrap()
                .resolution,
            NoteResolution::Unresolved
        );
    }
}
