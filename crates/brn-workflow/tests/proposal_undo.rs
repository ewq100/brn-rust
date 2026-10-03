#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind,
    activity::ActivityRequest,
    app::{App, AppConfig},
    editor::EditRequest,
    proposal_apply::{ApplyOutcome, ApprovalRequest, UndoRequest},
    proposals::*,
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
    process::Command,
};
use uuid::Uuid;

struct Fixture {
    _dir: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let vault = dir.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(vault.join("replace.md"), "\u{feff}Original λ\r\n").unwrap();
        fs::write(vault.join("trash.md"), "Original Trash 🦀\r\n").unwrap();
        Self {
            _dir: dir,
            data,
            vault,
        }
    }
    fn app(&self) -> App {
        App::open(
            &self.data,
            AppConfig {
                vault_root: Some(self.vault.clone()),
                credentials_dir: None,
                model_dir: None,
            },
        )
        .unwrap()
    }
    fn apply(&self, app: &mut App) -> ApprovalRequest {
        let editor_path = if app.work_store().editor("REPLACE.md").unwrap().is_some() {
            "REPLACE.md"
        } else {
            "replace.md"
        };
        let before = app.open_editor(editor_path).unwrap().record.baseline;
        let trash = app.open_editor("trash.md").unwrap().record.baseline;
        let draft = app
            .create_proposal(&DraftRequest {
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Changes".into(),
                changes: vec![
                    DraftNoteChange::Replace {
                        path: "replace.md".into(),
                        expected: before,
                        text: "New 日本語\r\n".into(),
                    },
                    DraftNoteChange::Create {
                        path: "new.md".into(),
                        text: "Created λ\r\n".into(),
                    },
                    DraftNoteChange::Trash {
                        path: "trash.md".into(),
                        expected: trash,
                    },
                ],
                sources: vec![],
            })
            .unwrap();
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: draft.stamp(),
        };
        assert_eq!(
            app.approve_proposal(&request).unwrap().outcome,
            ApplyOutcome::Applied
        );
        request
    }
}
fn undo(source: &ApprovalRequest) -> UndoRequest {
    UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: source.operation_id,
        trash_member: None,
    }
}

#[test]
fn inverse_restores_exact_originals_and_attributes_and_preserves_editor_stamp() {
    let f = Fixture::new();
    let path = f.vault.join("replace.md");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    assert!(
        Command::new("/usr/bin/xattr")
            .args(["-w", "org.brn.undo.synthetic", "retained"])
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("/bin/chmod")
            .args(["+a", "everyone allow read"])
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    let inode = fs::metadata(&path).unwrap().ino();
    let trash_inode = fs::metadata(f.vault.join("trash.md")).unwrap().ino();
    let mut app = f.app();
    let original = app.open_editor("replace.md").unwrap().record;
    let source = f.apply(&mut app);
    let request = undo(&source);
    let preview = app.preview_proposal_undo(&request).unwrap();
    assert_eq!(preview.draft.changes.len(), 3);
    assert_eq!(app.work_store().proposal_applies().unwrap().len(), 1);
    assert_eq!(fs::read_to_string(&path).unwrap(), "New 日本語\r\n");
    let receipt = app.undo_proposal(&request).unwrap();
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    assert_eq!(
        fs::read(&path).unwrap(),
        "\u{feff}Original λ\r\n".as_bytes()
    );
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(
        Command::new("/usr/bin/xattr")
            .args(["-p", "org.brn.undo.synthetic"])
            .arg(&path)
            .output()
            .unwrap()
            .stdout,
        b"retained\n"
    );
    assert!(
        String::from_utf8_lossy(
            &Command::new("/bin/ls")
                .arg("-le")
                .arg(&path)
                .output()
                .unwrap()
                .stdout
        )
        .contains("everyone allow read")
    );
    assert_eq!(
        fs::metadata(f.vault.join("trash.md")).unwrap().ino(),
        trash_inode
    );
    assert_eq!(
        fs::read(f.vault.join("trash.md")).unwrap(),
        "Original Trash 🦀\r\n".as_bytes()
    );
    assert!(!f.vault.join("new.md").exists());
    let restored = app.open_editor("replace.md").unwrap();
    assert_eq!(restored.record, original);
    assert!(!restored.conflict);
    let later = app
        .recover_editor(&EditRequest {
            path: "replace.md".into(),
            expected: original.stamp,
            generation: original.stamp.generation + 1,
            text: "Queued typing 🦀".into(),
        })
        .unwrap();
    assert_eq!(later.baseline, original.baseline);
    assert_eq!(app.undo_proposal(&request).unwrap(), receipt);
    drop(app);
    let mut app = f.app();
    assert_eq!(app.open_editor("replace.md").unwrap().record, later);
    assert_eq!(app.undo_proposal(&request).unwrap(), receipt);
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    fs::write(&path, "Later owner bytes").unwrap();
    assert_eq!(app.undo_proposal(&request).unwrap(), receipt);
    assert_eq!(fs::read_to_string(&path).unwrap(), "Later owner bytes");
    let entry = app
        .activity(&ActivityRequest::default())
        .unwrap()
        .entries
        .into_iter()
        .find(|entry| entry.operation_id == request.operation_id)
        .unwrap();
    assert_eq!(entry.undo.unwrap().operation_id, source.operation_id);
}

#[test]
fn scoped_trash_restore_preserves_later_other_notes_and_binds_scope_replay() {
    let f = Fixture::new();
    let mut app = f.app();
    let source = f.apply(&mut app);
    fs::write(f.vault.join("replace.md"), "Later replace").unwrap();
    fs::write(f.vault.join("new.md"), "Later create").unwrap();
    assert_eq!(
        app.undo_proposal(&undo(&source)).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    let request = UndoRequest {
        trash_member: Some(2),
        ..undo(&source)
    };
    let preview = app.preview_proposal_undo(&request).unwrap();
    assert_eq!(preview.draft.changes.len(), 1);
    assert_eq!(preview.draft.changes[0].path(), "trash.md");
    let receipt = app.undo_proposal(&request).unwrap();
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    assert_eq!(
        fs::read_to_string(f.vault.join("replace.md")).unwrap(),
        "Later replace"
    );
    assert_eq!(
        fs::read_to_string(f.vault.join("new.md")).unwrap(),
        "Later create"
    );
    assert_eq!(
        fs::read_to_string(f.vault.join("trash.md")).unwrap(),
        "Original Trash 🦀\r\n"
    );
    assert_eq!(
        app.undo_proposal(&UndoRequest {
            trash_member: None,
            ..request.clone()
        })
        .unwrap_err()
        .kind,
        ErrorKind::OperationConflict
    );
    assert_eq!(app.undo_proposal(&request).unwrap(), receipt);
    let entry = app
        .activity(&ActivityRequest::default())
        .unwrap()
        .entries
        .into_iter()
        .find(|entry| entry.operation_id == request.operation_id)
        .unwrap();
    assert_eq!(entry.undo.unwrap().trash_member, Some(2));
}

#[test]
fn altered_originals_targets_and_dirty_aliased_recovery_refuse_without_admission() {
    for scenario in [
        "original",
        "destination",
        "dirty-alias",
        "same-bytes-new-inode",
        "parent",
    ] {
        let f = Fixture::new();
        let mut app = f.app();
        if scenario == "dirty-alias" {
            app.open_editor("REPLACE.md").unwrap();
        }
        let source = f.apply(&mut app);
        let journal = app
            .work_store()
            .proposal_apply(source.operation_id)
            .unwrap()
            .unwrap();
        match scenario {
            "original" => fs::write(
                f.vault.join(&journal.members[0].staging),
                "Altered original",
            )
            .unwrap(),
            "destination" => fs::write(f.vault.join("trash.md"), "Unexpected occupant").unwrap(),
            "dirty-alias" => {
                let original = app.open_editor("REPLACE.md").unwrap().record;
                app.recover_editor(&EditRequest {
                    path: "REPLACE.md".into(),
                    expected: original.stamp,
                    generation: original.stamp.generation + 1,
                    text: "Later typing".into(),
                })
                .unwrap();
            }
            "same-bytes-new-inode" => {
                fs::rename(
                    f.vault.join(&journal.members[0].staging),
                    f.vault.join("kept-original"),
                )
                .unwrap();
                fs::write(
                    f.vault.join(&journal.members[0].staging),
                    "\u{feff}Original λ\r\n",
                )
                .unwrap();
            }
            "parent" => {
                fs::rename(&f.vault, f._dir.path().join("retained-vault")).unwrap();
                fs::create_dir(&f.vault).unwrap();
            }
            _ => unreachable!(),
        }
        let request = undo(&source);
        assert!(app.undo_proposal(&request).is_err(), "{scenario}");
        assert!(
            app.work_store()
                .proposal_apply(request.operation_id)
                .unwrap()
                .is_none(),
            "{scenario}"
        );
        assert!(app.proposal(source.expected.id).is_ok());
    }
}

#[test]
fn undo_of_undo_and_fresh_database_recovery_do_not_repeat_old_effects() {
    let f = Fixture::new();
    let mut app = f.app();
    let source = f.apply(&mut app);
    let request = undo(&source);
    let receipt = app.undo_proposal(&request).unwrap();
    let again = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: request.operation_id,
        trash_member: None,
    };
    assert_eq!(
        app.undo_proposal(&again).unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(
        fs::read_to_string(f.vault.join("replace.md")).unwrap(),
        "New 日本語\r\n"
    );
    drop(app);
    fs::write(f.vault.join("replace.md"), "Later bytes").unwrap();
    fs::remove_file(f.data.join("brn.sqlite")).unwrap();
    fs::remove_dir_all(f.data.join("backups")).unwrap();
    let mut app = f.app();
    assert_eq!(app.undo_proposal(&request).unwrap(), receipt);
    assert_eq!(
        fs::read_to_string(f.vault.join("replace.md")).unwrap(),
        "Later bytes"
    );
    assert_eq!(
        app.activity(&ActivityRequest::default())
            .unwrap()
            .entries
            .len(),
        3
    );
}
