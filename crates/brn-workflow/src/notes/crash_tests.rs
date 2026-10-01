use super::*;

#[test]
fn proven_pre_exchange_refusals_do_not_block_later_original_saves() {
    for race in [
        "noop_refusal",
        "pre_stage_refusal",
        "prepared_refusal",
        "root_refusal",
    ] {
        let mut f = Fixture::new();
        if race == "noop_refusal" {
            f.request.text = "base".into();
            fs::write(
                f.root.path().join("request.json"),
                serde_json::to_vec(&f.request).unwrap(),
            )
            .unwrap();
        }
        fs::write(f.root.path().join("race"), race).unwrap();
        f.crash("refusal");
        let mut w = f.reopen();
        let error = w.save_note(f.request.clone()).unwrap_err();
        assert_eq!(
            error.filesystem_outcome,
            FileOutcome::NotApplied,
            "{race}: {error}"
        );
        let intent = w
            .store
            .note_save_intent(f.request.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(intent.resolution, NoteResolution::NotApplied, "{race}");
        let recovered = w.note_recoveries().unwrap().remove(0);
        assert_eq!(recovered.working, f.request.text);
        assert!(recovered.pending_operations.is_empty());
        assert_eq!(
            w.note(f.request.note_id).unwrap().availability,
            NoteAvailability::Available
        );
        w.save_note(NoteSubmission {
            operation_id: Uuid::new_v4(),
            expected: recovered.stamp,
            generation: 3,
            text: "second save".into(),
            ..f.request.clone()
        })
        .unwrap();
        assert_eq!(w.save_note(f.request.clone()).unwrap_err(), error);
        assert_eq!(
            w.reconcile_note_save(f.request.operation_id).unwrap_err(),
            error
        );
        assert_eq!(
            fs::read(f.root.path().join("vault/plan.md")).unwrap(),
            b"second save"
        );
    }
}

#[test]
fn pre_exchange_external_conflict_is_not_applied_and_stays_blocked() {
    for race in [
        "precheck_unrestored",
        "noop_unrestored",
        "precheck_stage_absent",
    ] {
        let mut f = Fixture::new();
        if race == "noop_unrestored" {
            f.request.text = "base".into();
            fs::write(
                f.root.path().join("request.json"),
                serde_json::to_vec(&f.request).unwrap(),
            )
            .unwrap();
        }
        fs::write(f.root.path().join("race"), race).unwrap();
        f.crash("refusal");
        let mut w = f.reopen();
        let error = w.save_note(f.request.clone()).unwrap_err();
        assert_eq!(error.code, NoteErrorCode::Conflict);
        assert_eq!(error.filesystem_outcome, FileOutcome::NotApplied);
        assert_eq!(
            w.store
                .note_save_intent(f.request.operation_id)
                .unwrap()
                .unwrap()
                .resolution,
            NoteResolution::Unresolved
        );
        assert_eq!(
            fs::read(f.root.path().join("vault/plan.md")).unwrap(),
            b"external"
        );
        let view = w.note(f.request.note_id).unwrap();
        assert_eq!(view.availability, NoteAvailability::Conflict);
        assert_eq!(view.saved.as_deref(), Some("external"));
        assert_eq!(view.buffer, f.request.text);
        assert_ne!(view.current_file_state, Some(view.stamp.file_state));
        assert_eq!(
            w.reconcile_note_save(f.request.operation_id).unwrap_err(),
            error
        );
        if race == "precheck_stage_absent" {
            assert!(!f.stage().exists());
        }
        fs::write(f.root.path().join("vault/plan.md"), "base").unwrap();
        let recovered = w.note_recoveries().unwrap().remove(0);
        let second = w
            .save_note(NoteSubmission {
                operation_id: Uuid::new_v4(),
                expected: recovered.stamp,
                generation: 3,
                text: "second".into(),
                ..f.request.clone()
            })
            .unwrap_err();
        assert_eq!(second.code, NoteErrorCode::SaveUncertain);
        assert_eq!(
            w.note(f.request.note_id).unwrap().availability,
            NoteAvailability::Conflict
        );
        assert_eq!(w.save_note(f.request.clone()).unwrap_err(), error);
        assert_eq!(
            fs::read(f.root.path().join("vault/plan.md")).unwrap(),
            b"base"
        );
    }
}

#[test]
fn terminal_no_stage_attempts_coalesce_and_later_saves_retry_pending_cleanup() {
    for phase in ["intent", "cleanup_pending"] {
        let f = Fixture::new();
        f.crash(phase);
        let mut w = f.reopen();
        let receipt = w.reconcile_note_save(f.request.operation_id).unwrap();
        let recovery = w.note_recoveries().unwrap().remove(0);
        w.save_note(NoteSubmission {
            operation_id: Uuid::new_v4(),
            expected: recovery.stamp,
            generation: 3,
            text: "new save".into(),
            ..f.request.clone()
        })
        .unwrap();
        assert!(
            w.store
                .note_save_intent(f.request.operation_id)
                .unwrap()
                .is_none(),
            "{phase}"
        );
        assert!(!f.stage().exists());
        assert_eq!(w.save_note(f.request.clone()).unwrap(), receipt);
        assert_eq!(
            fs::read(f.root.path().join("vault/plan.md")).unwrap(),
            b"new save"
        );
    }
}

#[test]
fn failure_after_exclusive_create_protects_unjournaled_artifact() {
    let f = Fixture::new();
    fs::write(f.root.path().join("race"), "stage_replace").unwrap();
    f.crash("refusal");
    let mut w = f.reopen();
    let error = w.save_note(f.request.clone()).unwrap_err();
    assert_eq!(error.filesystem_outcome, FileOutcome::Unknown);
    assert_eq!(error.code, NoteErrorCode::SaveUncertain);
    assert_eq!(
        w.note(f.request.note_id).unwrap().availability,
        NoteAvailability::Uncertain
    );
    assert_eq!(error.phase, Some(SavePhase::Intent));
    assert!(error.recovery_available);
    let intent = w
        .store
        .note_save_intent(f.request.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(intent.cleanup, ArtifactCleanup::RetainedUnexpected);
    assert!(intent.staged.is_none());
    assert_eq!(fs::read(f.stage().join("unexpected")).unwrap(), b"preserve");
    assert_eq!(
        fs::read(f.root.path().join("vault/plan.md")).unwrap(),
        b"base"
    );
}

#[test]
fn crashing_during_obsolete_payload_retirement_preserves_current_pair_and_replays() {
    for phase in ["before_prune", "pruned"] {
        let mut f = Fixture::new();
        let mut w = f.reopen();
        let earlier_request = NoteSubmission {
            text: "first".into(),
            ..f.request.clone()
        };
        let earlier = w.save_note(earlier_request.clone()).unwrap();
        f.request = NoteSubmission {
            operation_id: Uuid::new_v4(),
            expected: earlier.stamp,
            generation: 3,
            text: "mine".into(),
            ..f.request
        };
        fs::write(
            f.root.path().join("request.json"),
            serde_json::to_vec(&f.request).unwrap(),
        )
        .unwrap();
        drop(w);
        f.crash(phase);
        let mut w = f.reopen();
        let intent = w.store.note_save_intent(earlier.operation_id).unwrap();
        assert_eq!(intent.is_some(), phase == "before_prune");
        let receipt = w.reconcile_note_save(f.request.operation_id).unwrap();
        assert_eq!(receipt.filesystem_outcome, FileOutcome::Applied);
        assert_eq!(w.note_recoveries().unwrap()[0].baseline, "mine");
        assert_eq!(w.note_recoveries().unwrap()[0].working, "mine");
        // The full successful-save pair is still bound to the new operation.
        assert!(
            w.store
                .note_save_intent(f.request.operation_id)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            fs::read(f.root.path().join("vault/plan.md")).unwrap(),
            b"mine"
        );
        assert_eq!(w.save_note(earlier_request).unwrap(), earlier);
        assert_eq!(w.save_note(f.request.clone()).unwrap(), receipt);
    }
}
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};
use tempfile::{TempDir, tempdir};

fn metadata(path: &Path) -> Option<(u64, u64, i64, i64, u32, u64)> {
    use std::os::unix::fs::MetadataExt;
    fs::symlink_metadata(path).ok().map(|value| {
        (
            value.dev(),
            value.ino(),
            value.mtime(),
            value.mtime_nsec(),
            value.mode(),
            value.len(),
        )
    })
}

#[test]
fn synced_checkpoint_requires_exact_installed_and_displaced_proof() {
    for change in [
        "missing_destination",
        "different_destination",
        "different_stage",
        "directory",
        "symlink",
    ] {
        let f = Fixture::new();
        f.crash("synced");
        let destination = f.root.path().join("vault/plan.md");
        match change {
            "missing_destination" => fs::remove_file(&destination).unwrap(),
            "different_destination" => fs::write(&destination, "new external").unwrap(),
            "different_stage" => fs::write(f.stage(), "late displaced write").unwrap(),
            "directory" => {
                fs::remove_file(f.stage()).unwrap();
                fs::create_dir(f.stage()).unwrap();
                fs::write(f.stage().join("protected"), "external").unwrap();
            }
            "symlink" => {
                fs::remove_file(f.stage()).unwrap();
                std::os::unix::fs::symlink(&destination, f.stage()).unwrap();
            }
            _ => unreachable!(),
        }
        let identities = (metadata(&destination), metadata(&f.stage()));
        let destination_bytes = fs::read(&destination).ok();
        let mut w = f.reopen();
        let error = w.reconcile_note_save(f.request.operation_id).unwrap_err();
        assert!(
            matches!(
                error.code,
                NoteErrorCode::Conflict | NoteErrorCode::SaveUncertain
            ),
            "{change}: {error}"
        );
        assert_eq!(error.filesystem_outcome, FileOutcome::Unknown);
        w.cleanup_note_artifacts(f.request.operation_id).unwrap();
        assert_eq!((metadata(&destination), metadata(&f.stage())), identities);
        assert_eq!(fs::read(&destination).ok(), destination_bytes);
        if change == "directory" {
            assert_eq!(fs::read(f.stage().join("protected")).unwrap(), b"external");
        }
    }
}

#[test]
fn child_worker() {
    let Ok(root) = std::env::var("BRN_NOTE_TEST_FIXTURE") else {
        return;
    };
    let phase = std::env::var("BRN_NOTE_TEST_PHASE").unwrap();
    let root = Path::new(&root);
    let request: NoteSubmission =
        serde_json::from_slice(&fs::read(root.join("request.json")).unwrap()).unwrap();
    let race = fs::read_to_string(root.join("race")).ok();
    let destination = root.join("vault/plan.md");
    let staging = root.join(format!("vault/.brn-{}.stage", request.operation_id));
    let vault = root.join("vault");
    let away = root.join("vault-away");
    let target = phase.clone();
    let mut w = crate::Workspace::open(&root.join("data"), crate::Config::default()).unwrap();
    save::TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |current| {
            if current == "intent" {
                match race.as_deref() {
                    Some("noop_refusal" | "noop_unrestored") => {
                        fs::write(&destination, "external").unwrap()
                    }
                    Some("root_refusal") => fs::rename(&vault, &away).unwrap(),
                    _ => (),
                }
            }
            if current == "before_stage" && race.as_deref() == Some("pre_stage_refusal") {
                fs::rename(&destination, destination.with_extension("removed")).unwrap();
            }
            if current == "prepared"
                && matches!(
                    race.as_deref(),
                    Some("prepared_refusal" | "precheck_unrestored" | "precheck_stage_absent")
                )
            {
                fs::write(&destination, "external").unwrap();
                if race.as_deref() == Some("precheck_stage_absent") {
                    fs::remove_file(&staging).unwrap();
                }
            }
            if current == "save_failed" {
                match race.as_deref() {
                    Some("noop_refusal" | "prepared_refusal") => {
                        fs::write(&destination, "base").unwrap()
                    }
                    Some("pre_stage_refusal") => {
                        fs::rename(destination.with_extension("removed"), &destination).unwrap()
                    }
                    Some("root_refusal") => fs::rename(&away, &vault).unwrap(),
                    _ => (),
                }
            }
            if current == "stage_created" && race.as_deref() == Some("stage_replace") {
                fs::remove_file(&staging).unwrap();
                fs::create_dir(&staging).unwrap();
                fs::write(staging.join("unexpected"), "preserve").unwrap();
            }
            if current == "prechecked" {
                match race.as_deref() {
                    Some("content") => fs::write(&destination, b"external").unwrap(),
                    Some("delete") => fs::remove_file(&destination).unwrap(),
                    Some("replace") => {
                        fs::write(destination.with_extension("replacement"), b"external").unwrap();
                        fs::rename(destination.with_extension("replacement"), &destination)
                            .unwrap();
                    }
                    Some("non_utf8") => fs::write(&destination, [0xff, 0xfe]).unwrap(),
                    Some("oversized") => {
                        fs::write(&destination, vec![b'x'; crate::MAX_IMPORT_BYTES + 1]).unwrap()
                    }
                    _ => (),
                }
            }
            if current == phase {
                println!("ACK:{current}");
                std::io::stdout().flush().unwrap();
                loop {
                    std::thread::sleep(Duration::from_secs(60));
                }
            }
        }));
    });
    let result = w.save_note(request);
    if target == "refusal" {
        assert!(result.is_err());
        save::checkpoint("refusal");
    } else {
        result.unwrap();
    }
    panic!("checkpoint was not reached");
}

struct Fixture {
    root: TempDir,
    request: NoteSubmission,
}

impl Fixture {
    fn new() -> Self {
        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("vault")).unwrap();
        fs::create_dir(root.path().join("data")).unwrap();
        fs::write(root.path().join("vault/plan.md"), "base").unwrap();
        let mut w =
            crate::Workspace::open(&root.path().join("data"), crate::Config::default()).unwrap();
        let opened = w
            .open_note(
                Uuid::new_v4(),
                &root.path().join("vault"),
                Path::new("plan.md"),
            )
            .unwrap();
        let buffer = w
            .save_note_buffer(NoteSubmission {
                operation_id: Uuid::new_v4(),
                note_id: opened.id,
                expected: opened.stamp,
                generation: 1,
                text: "acknowledged".into(),
            })
            .unwrap();
        let request = NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: opened.id,
            expected: buffer.stamp,
            generation: 2,
            text: "mine".into(),
        };
        fs::write(
            root.path().join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        Self { root, request }
    }

    fn crash(&self, phase: &str) {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "notes::crash_tests::child_worker", "--nocapture"])
            .env("BRN_NOTE_TEST_FIXTURE", self.root.path())
            .env("BRN_NOTE_TEST_PHASE", phase)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, receive) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        let expected = format!("ACK:{phase}");
        let reached = loop {
            match receive.recv_timeout(Duration::from_secs(30)) {
                Ok(Ok(line)) if line.starts_with("ACK:") => break line == expected,
                Ok(Ok(_)) => (),
                _ => break false,
            }
        };
        let killed = child.kill();
        let reaped = child.wait();
        reader.join().unwrap();
        reaped.unwrap();
        assert!(
            reached,
            "child did not acknowledge exact checkpoint {expected}"
        );
        killed.unwrap();
    }

    fn reopen(&self) -> crate::Workspace {
        crate::Workspace::open(&self.root.path().join("data"), crate::Config::default()).unwrap()
    }
    fn stage(&self) -> std::path::PathBuf {
        self.root
            .path()
            .join(format!("vault/.brn-{}.stage", self.request.operation_id))
    }
}

#[test]
fn all_durable_checkpoint_rows_classify_without_filesystem_mutation() {
    for (phase, outcome) in [
        ("before_intent", None),
        ("intent", Some(FileOutcome::NotApplied)),
        ("stage", Some(FileOutcome::Unknown)),
        ("prepared", Some(FileOutcome::NotApplied)),
        ("exchange_returned", Some(FileOutcome::Applied)),
        ("synced", Some(FileOutcome::Applied)),
        ("verified", Some(FileOutcome::Applied)),
        ("receipt", Some(FileOutcome::Applied)),
        ("cleanup_pending", Some(FileOutcome::Applied)),
        ("cleanup_unlinked", Some(FileOutcome::Applied)),
        ("before_prune", Some(FileOutcome::Applied)),
        ("pruned", Some(FileOutcome::Applied)),
    ] {
        let f = Fixture::new();
        f.crash(phase);
        let before = fs::read(f.root.path().join("vault/plan.md")).unwrap();
        let stage = fs::read(f.stage()).ok();
        match phase {
            "before_intent" | "intent" => {
                assert_eq!(before, b"base");
                assert!(stage.is_none());
            }
            "stage" | "prepared" => {
                assert_eq!(before, b"base");
                assert_eq!(stage.as_deref(), Some(b"mine".as_slice()));
            }
            "cleanup_unlinked" | "before_prune" | "pruned" => {
                assert_eq!(before, b"mine");
                assert!(stage.is_none());
            }
            _ => {
                assert_eq!(before, b"mine");
                assert_eq!(stage.as_deref(), Some(b"base".as_slice()));
            }
        }
        let identities = (
            metadata(&f.root.path().join("vault/plan.md")),
            metadata(&f.stage()),
        );
        let mut w = f.reopen();
        if phase == "before_intent" {
            assert_eq!(w.note_recoveries().unwrap()[0].working, "acknowledged");
            assert!(
                w.store
                    .note_save_intent(f.request.operation_id)
                    .unwrap()
                    .is_none()
            );
        } else if outcome == Some(FileOutcome::Unknown) {
            let error = w.reconcile_note_save(f.request.operation_id).unwrap_err();
            assert_eq!(error.code, NoteErrorCode::SaveUncertain);
            assert_eq!(error.filesystem_outcome, FileOutcome::Unknown);
            assert!(error.recovery_available);
        } else {
            let receipt = w.reconcile_note_save(f.request.operation_id).unwrap();
            assert_eq!(Some(receipt.filesystem_outcome), outcome, "{phase}");
            assert_eq!(w.save_note(f.request.clone()).unwrap(), receipt);
        }
        assert_eq!(
            fs::read(f.root.path().join("vault/plan.md")).unwrap(),
            before,
            "{phase}"
        );
        assert_eq!(fs::read(f.stage()).ok(), stage, "{phase}");
        assert_eq!(
            (
                metadata(&f.root.path().join("vault/plan.md")),
                metadata(&f.stage())
            ),
            identities,
            "{phase}"
        );
    }
}
#[test]
fn late_content_deletion_replacement_and_invalid_displaced_bytes_never_rollback() {
    for race in ["content", "delete", "replace", "non_utf8", "oversized"] {
        let f = Fixture::new();
        let destination = f.root.path().join("vault/plan.md");
        fs::write(f.root.path().join("race"), race).unwrap();
        f.crash("refusal");
        let mut w = f.reopen();
        let error = w.save_note(f.request.clone()).unwrap_err();
        assert_eq!(error.filesystem_outcome, FileOutcome::Unknown, "{race}");
        assert!(
            matches!(
                error.code,
                NoteErrorCode::Conflict | NoteErrorCode::SaveUncertain
            ),
            "{race}: {error}"
        );
        assert!(error.recovery_available);
        if race == "delete" {
            assert!(!destination.exists());
            assert_eq!(fs::read(f.stage()).unwrap(), b"mine");
        } else {
            assert_eq!(fs::read(&destination).unwrap(), b"mine", "{race}");
            let displaced = fs::read(f.stage()).unwrap();
            let expected = match race {
                "non_utf8" => vec![0xff, 0xfe],
                "oversized" => vec![b'x'; crate::MAX_IMPORT_BYTES + 1],
                _ => b"external".to_vec(),
            };
            assert_eq!(displaced, expected, "{race}");
            let intent = w
                .store
                .note_save_intent(f.request.operation_id)
                .unwrap()
                .unwrap();
            assert!(intent.displaced.is_some());
            assert_eq!(intent.cleanup, ArtifactCleanup::RetainedUnexpected);
        }
        let before = fs::read(&destination).ok();
        assert_eq!(w.save_note(f.request.clone()).unwrap_err(), error);
        let second = NoteSubmission {
            operation_id: Uuid::new_v4(),
            expected: w.note_recoveries().unwrap()[0].stamp,
            generation: 3,
            text: "later".into(),
            ..f.request.clone()
        };
        assert!(w.save_note(second).is_err());
        w.store
            .prune_completed_note_payloads(f.request.note_id)
            .unwrap();
        assert!(
            w.store
                .note_save_intent(f.request.operation_id)
                .unwrap()
                .is_some()
        );
        assert_eq!(fs::read(&destination).ok(), before);
        drop(w);
        let mut w = f.reopen();
        assert_eq!(
            w.reconcile_note_save(f.request.operation_id).unwrap_err(),
            error
        );
        assert_eq!(fs::read(&destination).ok(), before);
    }
}

#[test]
fn equal_content_without_prepared_or_displaced_proof_is_uncertain() {
    for phase in ["intent", "prepared", "exchange_returned"] {
        let f = Fixture::new();
        f.crash(phase);
        let destination = f.root.path().join("vault/plan.md");
        if phase == "exchange_returned" {
            fs::remove_file(f.stage()).unwrap();
        } else {
            fs::write(destination.with_extension("replacement"), "mine").unwrap();
            fs::rename(destination.with_extension("replacement"), &destination).unwrap();
        }
        let before = fs::read(&destination).unwrap();
        let stage = fs::read(f.stage()).ok();
        let mut w = f.reopen();
        let error = w.reconcile_note_save(f.request.operation_id).unwrap_err();
        assert_eq!(error.code, NoteErrorCode::SaveUncertain, "{phase}");
        assert_eq!(error.filesystem_outcome, FileOutcome::Unknown);
        assert_eq!(fs::read(&destination).unwrap(), before);
        assert_eq!(fs::read(f.stage()).ok(), stage);
        w.cleanup_note_artifacts(f.request.operation_id).unwrap();
        assert_eq!(fs::read(f.stage()).ok(), stage);
    }
}

#[test]
fn cleanup_pending_retires_absence_but_preserves_unexpected_occupant() {
    for phase in ["cleanup_pending", "cleanup_unlinked"] {
        for unexpected in [false, true] {
            let f = Fixture::new();
            f.crash(phase);
            if unexpected {
                if f.stage().exists() {
                    fs::remove_file(f.stage()).unwrap();
                }
                fs::create_dir(f.stage()).unwrap();
                fs::write(f.stage().join("never-delete"), "external").unwrap();
            }
            let mut w = f.reopen();
            let receipt = w.reconcile_note_save(f.request.operation_id).unwrap();
            assert_eq!(receipt.filesystem_outcome, FileOutcome::Applied);
            w.cleanup_note_artifacts(f.request.operation_id).unwrap();
            let intent = w
                .store
                .note_save_intent(f.request.operation_id)
                .unwrap()
                .unwrap();
            assert_eq!(
                intent.cleanup,
                if unexpected {
                    ArtifactCleanup::RetainedUnexpected
                } else {
                    ArtifactCleanup::Retired
                }
            );
            if unexpected {
                assert_eq!(
                    fs::read(f.stage().join("never-delete")).unwrap(),
                    b"external"
                );
            } else {
                assert!(!f.stage().exists());
            }
            assert_eq!(
                fs::read(f.root.path().join("vault/plan.md")).unwrap(),
                b"mine"
            );
        }
    }
}

#[test]
fn later_typing_is_retained_and_rebased_by_interrupted_save_completion() {
    let f = Fixture::new();
    f.crash("verified");
    let mut w = f.reopen();
    let buffer = w
        .save_note_buffer(NoteSubmission {
            operation_id: Uuid::new_v4(),
            expected: w.note_recoveries().unwrap()[0].stamp,
            generation: 3,
            text: "typed later".into(),
            ..f.request.clone()
        })
        .unwrap();
    let receipt = w.reconcile_note_save(f.request.operation_id).unwrap();
    let view = w.note(f.request.note_id).unwrap();
    assert_eq!(view.buffer, "typed later");
    assert_eq!(view.stamp.generation, buffer.stamp.generation);
    assert_eq!(view.stamp.file_state, receipt.stamp.file_state);
    assert_eq!(view.current_file_state, Some(receipt.stamp.file_state));
    assert_eq!(view.saved.as_deref(), Some("mine"));
}

#[test]
fn second_original_intent_is_blocked_even_after_store_startup_interrupts_it() {
    let f = Fixture::new();
    f.crash("intent");
    let mut w = f.reopen();
    let error = w
        .save_note(NoteSubmission {
            operation_id: Uuid::new_v4(),
            expected: w.note_recoveries().unwrap()[0].stamp,
            generation: 3,
            text: "later".into(),
            ..f.request.clone()
        })
        .unwrap_err();
    assert_eq!(error.code, NoteErrorCode::SaveUncertain);
    assert!(error.recovery_available);
    assert_eq!(
        fs::read(f.root.path().join("vault/plan.md")).unwrap(),
        b"base"
    );
    assert_eq!(w.store.note_save_intents().unwrap().len(), 1);
}

#[test]
fn not_applied_payloads_coalesce_but_uncertain_payloads_and_replays_survive() {
    let f = Fixture::new();
    f.crash("prepared");
    let mut w = f.reopen();
    let receipt = w.reconcile_note_save(f.request.operation_id).unwrap();
    w.cleanup_note_artifacts(f.request.operation_id).unwrap();
    w.store
        .prune_completed_note_payloads(f.request.note_id)
        .unwrap();
    assert!(
        w.store
            .note_save_intent(f.request.operation_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(w.save_note(f.request.clone()).unwrap(), receipt);
    let recovery = w.note_recoveries().unwrap().remove(0);
    assert_eq!(recovery.working, "mine");
    assert_eq!(recovery.baseline, "base");
    let uncertain = NoteSubmission {
        operation_id: Uuid::new_v4(),
        expected: recovery.stamp,
        generation: 3,
        text: "unresolved".into(),
        ..f.request.clone()
    };
    let record = w.store.note_record(uncertain.note_id).unwrap();
    w.store
        .begin_note_save(
            &uncertain,
            &record.relative_path,
            brn_store::notes::NoteWriteKind::Replace,
            &brn_store::notes::DestinationPrecondition::Existing {
                fingerprint: record.baseline,
                baseline_text: recovery.baseline,
            },
        )
        .unwrap();
    fs::write(
        f.root
            .path()
            .join(format!("vault/.brn-{}.stage", uncertain.operation_id)),
        "unproven",
    )
    .unwrap();
    let error = w.reconcile_note_save(uncertain.operation_id).unwrap_err();
    assert_eq!(error.code, NoteErrorCode::SaveUncertain);
    w.cleanup_note_artifacts(uncertain.operation_id).unwrap();
    w.store
        .prune_completed_note_payloads(uncertain.note_id)
        .unwrap();
    assert!(
        w.store
            .note_save_intent(uncertain.operation_id)
            .unwrap()
            .is_some()
    );
    assert_eq!(w.note_recoveries().unwrap()[0].working, "unresolved");
    assert_eq!(w.save_note(uncertain).unwrap_err(), error);
}
