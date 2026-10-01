#![cfg(target_os = "macos")]

use brn_workflow::{
    Config, SearchApproval, Workspace,
    notes::{NoteAvailability, NoteErrorCode, NoteSubmission, NoteView},
};
use std::{fs, path::Path};
use tempfile::{TempDir, tempdir};
use uuid::Uuid;

struct Fixture {
    data: TempDir,
    vault: TempDir,
    workspace: Workspace,
    opened: NoteView,
}

impl Fixture {
    fn new(text: &str) -> Self {
        let data = tempdir().unwrap();
        let vault = tempdir().unwrap();
        fs::write(vault.path().join("plan.md"), text).unwrap();
        let mut workspace = Workspace::open(data.path(), Config::default()).unwrap();
        let opened = workspace
            .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
            .unwrap();
        Self {
            data,
            vault,
            workspace,
            opened,
        }
    }

    fn submission(&self, text: &str) -> NoteSubmission {
        NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: self.opened.id,
            expected: self.opened.stamp,
            generation: 1,
            text: text.into(),
        }
    }
}

#[test]
fn acknowledged_buffer_survives_reopen_without_writing_markdown() {
    let mut f = Fixture::new("# base\r\n");
    let receipt = f
        .workspace
        .save_note_buffer(f.submission("# draft\r\n"))
        .unwrap();
    assert_eq!(receipt.stamp.generation, 1);
    assert_eq!(f.workspace.note(f.opened.id).unwrap().buffer, "# draft\r\n");
    assert_eq!(
        fs::read(f.vault.path().join("plan.md")).unwrap(),
        b"# base\r\n"
    );
    let data = f.data.path().to_owned();
    drop(f.workspace);
    let mut reopened = Workspace::open(&data, Config::default()).unwrap();
    let view = reopened.note(f.opened.id).unwrap();
    assert_eq!(view.buffer, "# draft\r\n");
    assert_eq!(view.saved.as_deref(), Some("# base\r\n"));
    assert_eq!(view.stamp, receipt.stamp);
    assert_eq!(view.availability, NoteAvailability::Available);
    let opened = reopened
        .open_note(Uuid::new_v4(), f.vault.path(), Path::new("plan.md"))
        .unwrap();
    assert_eq!(opened.id, view.id);
    assert_eq!(opened.stamp, view.stamp);
    assert_eq!(opened.buffer, view.buffer);
}

#[test]
fn empty_and_exact_utf8_notes_are_valid_without_approval() {
    for text in ["", "\u{feff}---\r\nname: café\r\n---\r\n"] {
        let f = Fixture::new(text);
        assert_eq!(f.opened.saved.as_deref(), Some(text));
        assert_eq!(f.opened.buffer, text);
        assert_eq!(f.opened.search_approval, SearchApproval::Draft);
        assert_eq!(f.opened.current_file_state, Some(f.opened.stamp.file_state));
    }
}

#[test]
fn changed_disk_has_a_separate_stable_observation_and_protected_baseline() {
    let mut f = Fixture::new("base");
    let receipt = f.workspace.save_note_buffer(f.submission("draft")).unwrap();
    fs::write(f.vault.path().join("plan.md"), "external").unwrap();
    let view = f.workspace.note(f.opened.id).unwrap();
    assert_eq!(view.saved.as_deref(), Some("external"));
    assert_eq!(view.buffer, "draft");
    assert_eq!(view.stamp, receipt.stamp);
    assert_eq!(view.availability, NoteAvailability::Conflict);
    assert_ne!(view.current_file_state, Some(view.stamp.file_state));
    assert_eq!(
        f.workspace.note(view.id).unwrap().current_file_state,
        view.current_file_state
    );
    let recoveries = f.workspace.note_recoveries().unwrap();
    assert_eq!(recoveries[0].baseline, "base");
    assert_eq!(recoveries[0].working, "draft");
    let opened = f
        .workspace
        .open_note(Uuid::new_v4(), f.vault.path(), Path::new("plan.md"))
        .unwrap();
    assert_eq!(opened.id, view.id);
    assert_eq!(opened.stamp, receipt.stamp);
    assert_eq!(opened.buffer, "draft");
}

#[test]
fn replacement_with_identical_bytes_is_not_silently_adopted() {
    let mut f = Fixture::new("base");
    fs::write(f.vault.path().join("replacement"), "base").unwrap();
    fs::rename(
        f.vault.path().join("replacement"),
        f.vault.path().join("plan.md"),
    )
    .unwrap();
    let view = f.workspace.note(f.opened.id).unwrap();
    assert_eq!(view.availability, NoteAvailability::Conflict);
    assert_eq!(view.id, f.opened.id);
    assert_ne!(view.current_file_state, Some(view.stamp.file_state));
}

#[test]
fn clean_observation_survives_restart_without_rebasing_the_buffer() {
    let f = Fixture::new("base");
    let mut w = f.workspace;
    fs::write(f.vault.path().join("plan.md"), "external").unwrap();
    let observed = w.note(f.opened.id).unwrap();
    assert_eq!(observed.buffer, "base");
    assert_eq!(observed.stamp, f.opened.stamp);
    assert_eq!(observed.availability, NoteAvailability::Conflict);
    drop(w);
    let mut w = Workspace::open(f.data.path(), Config::default()).unwrap();
    let reopened = w.note(f.opened.id).unwrap();
    assert_eq!(reopened.current_file_state, observed.current_file_state);
    assert_eq!(reopened.saved.as_deref(), Some("external"));
    assert_eq!(reopened.stamp, f.opened.stamp);
    fs::write(f.vault.path().join("plan.md"), "base").unwrap();
    assert_eq!(
        w.note(f.opened.id).unwrap().current_file_state,
        Some(f.opened.stamp.file_state)
    );
}

#[test]
fn an_owned_root_replacement_is_validated_before_observing_a_stored_path() {
    let mut f = Fixture::new("base");
    fs::rename(f.vault.path(), f.data.path().join("old-root")).unwrap();
    fs::create_dir(f.vault.path()).unwrap();
    fs::write(f.vault.path().join("plan.md"), "impostor").unwrap();
    let view = f.workspace.note(f.opened.id).unwrap();
    assert_eq!(view.availability, NoteAvailability::Unavailable);
    assert_eq!(view.saved, None);
    assert_eq!(view.current_file_state, None);
    assert_eq!(view.buffer, "base");
    assert!(view.availability_message.is_some());
}

#[test]
fn buffer_generations_are_checked_and_replay_precedes_current_state() {
    let mut f = Fixture::new("base");
    let request = f.submission("draft");
    let receipt = f.workspace.save_note_buffer(request.clone()).unwrap();
    let mut later = request.clone();
    later.operation_id = Uuid::new_v4();
    later.expected = receipt.stamp;
    later.generation = 2;
    later.text = "later".into();
    let latest = f.workspace.save_note_buffer(later).unwrap();
    fs::remove_file(f.vault.path().join("plan.md")).unwrap();
    assert_eq!(
        f.workspace.save_note_buffer(request.clone()).unwrap(),
        receipt
    );
    let mut conflicting = request.clone();
    conflicting.text = "wrong".into();
    let error = f.workspace.save_note_buffer(conflicting).unwrap_err();
    assert_eq!(error.code, NoteErrorCode::OperationConflict);
    assert_eq!(error.operation_id, Some(request.operation_id));
    assert_eq!(error.note_id, Some(request.note_id));
    for (expected, generation, text) in [
        (request.expected, 3, "stale stamp"),
        (latest.stamp, 1, "lower"),
        (latest.stamp, 2, "different equal"),
    ] {
        let error = f
            .workspace
            .save_note_buffer(NoteSubmission {
                operation_id: Uuid::new_v4(),
                note_id: request.note_id,
                expected,
                generation,
                text: text.into(),
            })
            .unwrap_err();
        assert_eq!(error.code, NoteErrorCode::StateChanged);
    }
    assert_eq!(f.workspace.note(f.opened.id).unwrap().buffer, "later");
}

#[test]
fn deleted_note_keeps_recovery_and_accepts_new_buffers() {
    let mut f = Fixture::new("base");
    fs::remove_file(f.vault.path().join("plan.md")).unwrap();
    f.workspace.save_note_buffer(f.submission("draft")).unwrap();
    let view = f.workspace.note(f.opened.id).unwrap();
    assert_eq!(view.availability, NoteAvailability::Missing);
    assert_eq!(view.saved, None);
    assert_eq!(view.current_file_state, None);
    assert_eq!(view.buffer, "draft");
    assert_eq!(f.workspace.note_recoveries().unwrap()[0].baseline, "base");
    let reopened = f
        .workspace
        .open_note(Uuid::new_v4(), f.vault.path(), Path::new("plan.md"))
        .unwrap();
    assert_eq!(reopened.id, f.opened.id);
    assert_eq!(reopened.availability, NoteAvailability::Missing);
}

#[test]
fn unsupported_existing_notes_surface_explicit_state() {
    for bytes in [vec![0xff], vec![b'x'; 1024 * 1024 + 1]] {
        let mut f = Fixture::new("base");
        fs::write(f.vault.path().join("plan.md"), bytes).unwrap();
        let view = f.workspace.note(f.opened.id).unwrap();
        assert_eq!(view.availability, NoteAvailability::Unsupported);
        assert_eq!(view.saved, None);
        assert_eq!(view.buffer, "base");
        assert!(view.availability_message.is_some());
    }
}

#[test]
fn enrollment_rejects_unsupported_and_missing_files() {
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    fs::write(vault.path().join("bad.md"), [0xff]).unwrap();
    fs::write(vault.path().join("plain.txt"), "plain").unwrap();
    fs::create_dir(vault.path().join("directory.md")).unwrap();
    fs::write(vault.path().join("linked.md"), "linked").unwrap();
    fs::hard_link(vault.path().join("linked.md"), vault.path().join("hard.md")).unwrap();
    std::os::unix::fs::symlink("plain.txt", vault.path().join("symlink.md")).unwrap();
    for path in [
        "bad.md",
        "plain.txt",
        "directory.md",
        "hard.md",
        "symlink.md",
        "../escape.md",
        "./bad.md",
    ] {
        assert_eq!(
            w.open_note(Uuid::new_v4(), vault.path(), Path::new(path))
                .unwrap_err()
                .code,
            NoteErrorCode::Unsupported,
            "{path}"
        );
    }
    assert_eq!(
        w.open_note(Uuid::new_v4(), vault.path(), Path::new("absent.md"))
            .unwrap_err()
            .code,
        NoteErrorCode::Missing
    );
}

#[test]
fn missing_or_replaced_root_does_not_block_database_or_recovery() {
    for replace in [false, true] {
        let mut f = Fixture::new("base");
        let request = f.submission("draft");
        let receipt = f.workspace.save_note_buffer(request.clone()).unwrap();
        let root = f.vault.path().to_owned();
        let moved = f.data.path().join("moved-vault");
        fs::rename(&root, &moved).unwrap();
        if replace {
            fs::create_dir(&root).unwrap();
            fs::write(root.join("plan.md"), "impostor").unwrap();
        }
        let data = f.data.path().to_owned();
        drop(f.workspace);
        let mut w = Workspace::open(&data, Config::default()).unwrap();
        assert_eq!(w.note_recoveries().unwrap()[0].working, "draft");
        assert!(w.sources().unwrap().is_empty());
        assert!(w.sessions().unwrap().is_empty());
        let view = w.note(f.opened.id).unwrap();
        assert_eq!(view.availability, NoteAvailability::Unavailable);
        assert_eq!(view.saved, None);
        assert!(view.availability_message.is_some());
        assert_eq!(w.save_note_buffer(request).unwrap(), receipt);
        w.save_note_buffer(NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: view.id,
            expected: receipt.stamp,
            generation: 2,
            text: "offline edit".into(),
        })
        .unwrap();
    }
}

#[test]
fn another_owner_blocks_filesystem_only_and_ownership_lasts_until_drop() {
    let f = Fixture::new("base");
    let other_data = tempdir().unwrap();
    let mut other = Workspace::open(other_data.path(), Config::default()).unwrap();
    assert_eq!(
        other
            .open_note(Uuid::new_v4(), f.vault.path(), Path::new("plan.md"))
            .unwrap_err()
            .code,
        NoteErrorCode::VaultBusy
    );
    let data = f.data.path().to_owned();
    let root = f.vault.path().to_owned();
    let id = f.opened.id;
    drop(f.workspace);
    other
        .open_note(Uuid::new_v4(), &root, Path::new("plan.md"))
        .unwrap();
    let mut w = Workspace::open(&data, Config::default()).unwrap();
    let view = w.note(id).unwrap();
    assert_eq!(view.availability, NoteAvailability::OwnedElsewhere);
    assert_eq!(view.saved, None);
    assert_eq!(view.current_file_state, None);
    assert!(view.availability_message.is_some());
    assert_eq!(w.note_recoveries().unwrap()[0].baseline, "base");
    w.save_note_buffer(NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: id,
        expected: view.stamp,
        generation: 1,
        text: "offline".into(),
    })
    .unwrap();
    drop(other);
    assert_eq!(
        w.note(id).unwrap().availability,
        NoteAvailability::Available
    );
}

#[test]
fn different_vault_is_rejected_even_after_restart() {
    let f = Fixture::new("base");
    let different = tempdir().unwrap();
    fs::write(different.path().join("plan.md"), "other").unwrap();
    let data = f.data.path().to_owned();
    let mut w = f.workspace;
    assert_eq!(
        w.open_note(Uuid::new_v4(), different.path(), Path::new("plan.md"))
            .unwrap_err()
            .code,
        NoteErrorCode::VaultUnavailable
    );
    drop(w);
    let mut w = Workspace::open(&data, Config::default()).unwrap();
    assert_eq!(
        w.open_note(Uuid::new_v4(), different.path(), Path::new("plan.md"))
            .unwrap_err()
            .code,
        NoteErrorCode::VaultUnavailable
    );
}

#[test]
fn open_operation_replay_does_not_require_root_or_replace_current_buffer() {
    let data = tempdir().unwrap();
    let vault = tempdir().unwrap();
    fs::write(vault.path().join("plan.md"), "base").unwrap();
    let op = Uuid::new_v4();
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let opened = w.open_note(op, vault.path(), Path::new("plan.md")).unwrap();
    w.save_note_buffer(NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: opened.id,
        expected: opened.stamp,
        generation: 1,
        text: "draft".into(),
    })
    .unwrap();
    let root = vault.path().to_owned();
    fs::rename(&root, data.path().join("moved-vault")).unwrap();
    drop(w);
    let mut w = Workspace::open(data.path(), Config::default()).unwrap();
    let replayed = w.open_note(op, &root, Path::new("plan.md")).unwrap();
    assert_eq!(replayed.id, opened.id);
    assert_eq!(replayed.buffer, "draft");
    assert_eq!(replayed.saved, None);
    assert_eq!(replayed.availability, NoteAvailability::Unavailable);
    assert_eq!(
        w.open_note(op, &root, Path::new("other.md"))
            .unwrap_err()
            .code,
        NoteErrorCode::OperationConflict
    );
}

#[test]
fn alias_of_selected_vault_keeps_identity_and_data_inside_vault_is_rejected() {
    let mut f = Fixture::new("base");
    let alias = f.data.path().join("vault-alias");
    std::os::unix::fs::symlink(f.vault.path(), &alias).unwrap();
    assert_eq!(
        f.workspace
            .open_note(Uuid::new_v4(), &alias, Path::new("plan.md"))
            .unwrap()
            .id,
        f.opened.id
    );
    let nested_data = f.vault.path().join("app-data");
    fs::create_dir(&nested_data).unwrap();
    let mut nested = Workspace::open(&nested_data, Config::default()).unwrap();
    assert_eq!(
        nested
            .open_note(Uuid::new_v4(), f.vault.path(), Path::new("plan.md"))
            .unwrap_err()
            .code,
        NoteErrorCode::Unsupported
    );
}
