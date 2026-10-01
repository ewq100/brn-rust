#![cfg(target_os = "macos")]

use brn_workflow::{
    Config, Workspace,
    notes::{FileOutcome, NoteAvailability, NoteErrorCode, NoteSubmission},
};
use std::{fs, path::Path};
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn invalid_generations_are_state_changed_even_when_the_vault_is_unavailable() {
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    fs::write(vault.path().join("plan.md"), b"base").unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let opened = w
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let buffer = w
        .save_note_buffer(NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: opened.id,
            expected: opened.stamp,
            generation: 1,
            text: "recovered".into(),
        })
        .unwrap();
    drop(w);
    fs::remove_dir_all(vault.path()).unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    for (expected, generation, text) in [
        (opened.stamp, 2, "stale"),
        (buffer.stamp, 0, "lower"),
        (buffer.stamp, 1, "different"),
    ] {
        let error = w
            .save_note(NoteSubmission {
                operation_id: Uuid::new_v4(),
                note_id: opened.id,
                expected,
                generation,
                text: text.into(),
            })
            .unwrap_err();
        assert_eq!(error.code, NoteErrorCode::StateChanged);
        assert!(!error.recovery_available);
    }
    assert_eq!(w.note_recoveries().unwrap()[0].working, "recovered");
}

#[test]
fn save_rejects_conflicting_operation_reuse_before_filesystem_checks() {
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    fs::write(vault.path().join("plan.md"), b"base").unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let opened = w
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.id,
        expected: opened.stamp,
        generation: 1,
        text: "mine".into(),
    };
    let receipt = w.save_note(request.clone()).unwrap();
    drop(w);
    fs::remove_dir_all(vault.path()).unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    assert_eq!(w.save_note(request.clone()).unwrap(), receipt);
    assert_eq!(
        w.save_note(NoteSubmission {
            text: "different".into(),
            ..request
        })
        .unwrap_err()
        .code,
        NoteErrorCode::OperationConflict
    );
}

#[test]
fn an_unexpected_staging_occupant_is_never_deleted() {
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let destination = vault.path().join("plan.md");
    fs::write(&destination, b"base").unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let opened = w
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.id,
        expected: opened.stamp,
        generation: 1,
        text: "mine".into(),
    };
    let stage = vault
        .path()
        .join(format!(".brn-{}.stage", request.operation_id));
    fs::write(&stage, "external artifact").unwrap();
    let error = w.save_note(request.clone()).unwrap_err();
    assert_eq!(error.filesystem_outcome, FileOutcome::NotApplied);
    assert!(error.recovery_available);
    assert_eq!(fs::read(&stage).unwrap(), b"external artifact");
    assert_eq!(fs::read(&destination).unwrap(), b"base");
    drop(w);
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    assert_eq!(w.save_note(request.clone()).unwrap_err(), error);
    let recovered = w.note_recoveries().unwrap().remove(0);
    assert!(recovered.pending_operations.is_empty());
    assert_eq!(
        w.note(request.note_id).unwrap().availability,
        NoteAvailability::Available
    );
    let receipt = w
        .save_note(NoteSubmission {
            operation_id: Uuid::new_v4(),
            expected: recovered.stamp,
            generation: 2,
            text: "second save".into(),
            ..request.clone()
        })
        .unwrap();
    assert_eq!(receipt.filesystem_outcome, FileOutcome::Applied);
    assert_eq!(w.save_note(request).unwrap_err(), error);
    assert_eq!(fs::read(&destination).unwrap(), b"second save");
    assert_eq!(fs::read(&stage).unwrap(), b"external artifact");
}

#[test]
fn original_save_preserves_mode_and_exact_bom_line_endings() {
    use std::os::unix::fs::PermissionsExt;
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let path = vault.path().join("plan.md");
    fs::write(&path, b"base").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let opened = w
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let text = "\u{feff}---\r\nname: café\r\n---\r\n";
    w.save_note(NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.id,
        expected: opened.stamp,
        generation: 1,
        text: text.into(),
    })
    .unwrap();
    assert_eq!(fs::read(&path).unwrap(), text.as_bytes());
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[test]
fn root_refusal_is_durable_and_id_only_recovery_remains_available() {
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    fs::write(vault.path().join("plan.md"), b"base").unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let opened = w
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.id,
        expected: opened.stamp,
        generation: 1,
        text: "mine".into(),
    };
    drop(w);
    fs::remove_dir_all(vault.path()).unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let error = w.save_note(request.clone()).unwrap_err();
    assert_eq!(error.code, NoteErrorCode::VaultUnavailable);
    assert_eq!(error.filesystem_outcome, FileOutcome::NotApplied);
    assert!(error.recovery_available);
    assert_eq!(w.note_recoveries().unwrap()[0].working, "mine");
    drop(w);
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    assert_eq!(w.save_note(request).unwrap_err(), error);
}

#[test]
fn same_file_save_replays_after_later_edits_and_restart() {
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let path = vault.path().join("plan.md");
    fs::write(&path, b"base").unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let opened = w
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.id,
        expected: opened.stamp,
        generation: 1,
        text: "mine".into(),
    };
    let receipt = w.save_note(request.clone()).unwrap();
    assert_eq!(receipt.filesystem_outcome, FileOutcome::Applied);
    assert_eq!(fs::read(&path).unwrap(), b"mine");
    let view = w.note(opened.id).unwrap();
    assert_eq!(view.current_file_state, Some(view.stamp.file_state));
    w.save_note_buffer(NoteSubmission {
        operation_id: Uuid::new_v4(),
        expected: view.stamp,
        generation: 2,
        text: "later".into(),
        ..request.clone()
    })
    .unwrap();
    fs::write(&path, "external").unwrap();
    assert_eq!(w.save_note(request.clone()).unwrap(), receipt);
    drop(w);
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    assert_eq!(w.save_note(request).unwrap(), receipt);
    assert_eq!(
        w.reconcile_note_save(receipt.operation_id).unwrap(),
        receipt
    );
    assert_eq!(fs::read(&path).unwrap(), b"external");
    assert_eq!(w.note(opened.id).unwrap().buffer, "later");
}

#[test]
fn stale_baseline_refusal_retains_submission_and_replays_without_vault() {
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let path = vault.path().join("plan.md");
    fs::write(&path, b"base").unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let opened = w
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.id,
        expected: opened.stamp,
        generation: 1,
        text: "mine".into(),
    };
    fs::write(&path, b"external").unwrap();
    let failure = w.save_note(request.clone()).unwrap_err();
    assert_eq!(failure.code, NoteErrorCode::Conflict);
    assert!(failure.recovery_available);
    assert_eq!(fs::read(&path).unwrap(), b"external");
    assert_eq!(w.note_recoveries().unwrap()[0].working, "mine");
    drop(w);
    fs::remove_dir_all(vault.path()).unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    assert_eq!(w.save_note(request).unwrap_err(), failure);
}

#[test]
fn recovered_equal_generation_saves_and_noop_preserves_metadata() {
    use std::os::unix::fs::MetadataExt;
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let path = vault.path().join("plan.md");
    fs::write(&path, b"base").unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let opened = w
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let buffer = w
        .save_note_buffer(NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: opened.id,
            expected: opened.stamp,
            generation: 1,
            text: "mine".into(),
        })
        .unwrap();
    drop(w);
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let receipt = w
        .save_note(NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: opened.id,
            expected: buffer.stamp,
            generation: 1,
            text: "mine".into(),
        })
        .unwrap();
    let before = fs::metadata(&path).unwrap();
    let noop = w
        .save_note(NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: opened.id,
            expected: receipt.stamp,
            generation: 1,
            text: "mine".into(),
        })
        .unwrap();
    let after = fs::metadata(&path).unwrap();
    assert_eq!(noop.filesystem_outcome, FileOutcome::NotApplied);
    assert_eq!(w.reconcile_note_save(noop.operation_id).unwrap(), noop);
    assert_eq!(
        (
            before.ino(),
            before.mtime(),
            before.mtime_nsec(),
            before.mode()
        ),
        (after.ino(), after.mtime(), after.mtime_nsec(), after.mode())
    );
}
