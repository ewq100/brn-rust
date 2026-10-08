#![cfg(target_os = "macos")]
use brn_workflow::{
    app::{App, AppConfig},
    editor::EditRequest,
    proposal_apply::{ApplyOutcome, ApprovalRequest, GroupApprovalRequest, UndoRequest},
    proposals::*,
};
use std::{fs, path::PathBuf};
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
        fs::create_dir(vault.join("folder")).unwrap();
        fs::write(vault.join("source.md"), "Retained λ source\r\n").unwrap();
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
    fn request(&self, app: &mut App) -> DraftRequest {
        DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: Some(Uuid::new_v4()),
            session_id: None,
            title: "Owner filename correction".into(),
            changes: vec![DraftNoteChange::Create {
                path: "folder/original.md".into(),
                text: brn_store::note_identity::assign(
                    "# Owner λ\r\n\r\nExact 🦀 bytes\r\n",
                    Uuid::new_v4(),
                )
                .unwrap(),
            }],
            sources: vec![app.proposal_source("source.md").unwrap().source],
            action_changes: vec![],
        }
    }
}
fn rename(record: &ProposalRecord, path: &str) -> CreateRenameRequest {
    CreateRenameRequest {
        expected: record.stamp(),
        change_index: 0,
        path: path.into(),
    }
}

#[test]
fn create_rename_preserves_owner_work_restarts_replays_original_and_undoes_exact_destination() {
    let f = Fixture::new();
    let mut app = f.app();
    let original = f.request(&mut app);
    let initial = app.create_proposal(&original).unwrap();
    let edited = app
        .edit_proposal(&ProposalEdit {
            expected: initial.stamp(),
            title: "Owner's saved title".into(),
            texts: vec![Some(
                initial.draft.changes[0].text().unwrap().to_owned()
                    + "\r\nOwner comment response\r\n",
            )],
            action_data: vec![],
        })
        .unwrap();
    let text = edited.draft.changes[0].text().unwrap();
    let start = text.find("Exact 🦀").unwrap();
    let commented = app
        .add_proposal_comment(&CommentRequest {
            expected: edited.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Keep exact owner bytes".into(),
                target: CommentTarget::Text(TextAnchor {
                    change_index: 0,
                    start,
                    end: start + "Exact 🦀".len(),
                    quote: "Exact 🦀".into(),
                }),
            },
        })
        .unwrap();
    let renamed = app
        .rename_proposal_create(&rename(&commented, "folder/corrected.md"))
        .unwrap();
    validate_create_rename_transition(&commented, &renamed, 0, "folder/corrected.md").unwrap();
    assert_eq!(renamed.comments, commented.comments);
    assert_eq!(
        renamed.draft.changes[0].text(),
        commented.draft.changes[0].text()
    );
    assert_eq!(
        app.rename_proposal_create(&rename(&renamed, "folder/corrected.md"))
            .unwrap(),
        renamed
    );
    assert!(
        app.rename_proposal_create(&rename(&commented, "folder/corrected.md"))
            .is_err()
    );
    assert!(
        app.rewrite_proposal(&ProposalEdit {
            expected: commented.stamp(),
            title: commented.draft.title.clone(),
            texts: vec![commented.draft.changes[0].text().map(str::to_owned)],
            action_data: vec![]
        })
        .is_err()
    );
    assert!(
        app.approve_proposal_group(&GroupApprovalRequest {
            group_id: original.group_id.unwrap(),
            approvals: vec![ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: commented.stamp()
            }]
        })
        .is_err()
    );
    let final_review = app
        .rename_proposal_create(&rename(&renamed, "folder/final.md"))
        .unwrap();
    assert_eq!(app.create_proposal(&original).unwrap(), final_review);
    assert_eq!(fs::read_dir(f.vault.join("folder")).unwrap().count(), 0);
    drop(app);
    let mut app = f.app();
    assert_eq!(app.create_proposal(&original).unwrap(), final_review);
    let mut other = original.clone();
    let DraftNoteChange::Create { path, .. } = &mut other.changes[0] else {
        unreachable!()
    };
    *path = "folder/final.md".into();
    assert!(
        app.create_proposal(&other).is_err(),
        "renamed path is not the original creation payload"
    );
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: final_review.stamp(),
    };
    let applied = app.approve_proposal(&approval).unwrap();
    assert_eq!(applied.outcome, ApplyOutcome::Applied);
    assert_eq!(
        fs::read_to_string(f.vault.join("folder/final.md")).unwrap(),
        final_review.draft.changes[0].text().unwrap()
    );
    assert!(!f.vault.join("folder/original.md").exists());
    assert!(!f.vault.join("folder/corrected.md").exists());
    fs::remove_file(f.vault.join("source.md")).unwrap();
    drop(app);
    let mut app = f.app();
    let replay = app.create_proposal(&original).unwrap();
    assert_eq!(replay.state, ProposalState::Applied);
    assert_eq!(replay.draft, final_review.draft);
    assert_eq!(app.approve_proposal(&approval).unwrap(), applied);
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: approval.operation_id,
        trash_member: None,
    };
    assert_eq!(
        app.undo_proposal(&undo).unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert!(!f.vault.join("folder/final.md").exists());
}

#[test]
fn create_rename_refusals_preserve_review_files_and_parent_authority() {
    for mode in 0..9 {
        let f = Fixture::new();
        let mut app = f.app();
        let input = f.request(&mut app);
        let record = app.create_proposal(&input).unwrap();
        let mut request = rename(&record, "folder/corrected.md");
        match mode {
            0 => request.path = "other.md".into(),
            1 => request.path = "folder/.hidden.md".into(),
            2 => request.path = "folder/../other.md".into(),
            3 => request.change_index = 1,
            4 => request.expected.version += 1,
            5 => fs::write(f.vault.join("folder/corrected.md"), "Owner occupant\n").unwrap(),
            6 => fs::write(f.vault.join("source.md"), "Changed λ source\r\n").unwrap(),
            7 => {
                fs::rename(f.vault.join("folder"), f.vault.join("old-folder")).unwrap();
                fs::create_dir(f.vault.join("folder")).unwrap();
            }
            8 => {
                fs::write(f.vault.join("folder/corrected.md"), "Editor baseline\n").unwrap();
                let view = app.open_editor("folder/corrected.md").unwrap();
                app.recover_editor(&EditRequest {
                    path: "folder/corrected.md".into(),
                    expected: view.record.stamp,
                    generation: 1,
                    text: "Owner unfinished buffer\n".into(),
                })
                .unwrap();
                fs::remove_file(f.vault.join("folder/corrected.md")).unwrap();
            }
            _ => unreachable!(),
        }
        let source = fs::read(f.vault.join("source.md")).unwrap();
        let occupied = fs::read(f.vault.join("folder/corrected.md")).ok();
        assert!(app.rename_proposal_create(&request).is_err(), "mode {mode}");
        assert_eq!(
            app.proposal(record.draft.id).unwrap(),
            record,
            "mode {mode}"
        );
        assert_eq!(fs::read(f.vault.join("source.md")).unwrap(), source);
        assert_eq!(fs::read(f.vault.join("folder/corrected.md")).ok(), occupied);
        assert!(!f.vault.join("folder/original.md").exists());
    }
}

#[test]
fn create_rename_rejects_folded_alias_with_other_member_and_preserves_original_proof() {
    let f = Fixture::new();
    let mut app = f.app();
    let mut input = f.request(&mut app);
    input.changes.push(DraftNoteChange::Create {
        path: "folder/é.md".into(),
        text: "Other exact member\n".into(),
    });
    let record = app.create_proposal(&input).unwrap();
    assert!(
        app.rename_proposal_create(&rename(&record, "folder/e\u{301}.md"))
            .is_err()
    );
    assert_eq!(app.proposal(record.draft.id).unwrap(), record);
    assert_eq!(fs::read_dir(f.vault.join("folder")).unwrap().count(), 0);
}
