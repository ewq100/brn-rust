#![cfg(target_os = "macos")]
use brn_workflow::{
    app::{App, AppConfig},
    editor::EditRequest,
    proposal_apply::{ApplyOutcome, ApprovalRequest, GroupApprovalRequest},
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
        fs::write(vault.join("replace.md"), "Original λ\r\n").unwrap();
        fs::write(vault.join("trash.md"), "Retain original 🦀\r\n").unwrap();
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
    fn draft(&self, app: &mut App) -> ProposalRecord {
        let before = app.open_editor("replace.md").unwrap().record.baseline;
        let trash = app.open_editor("trash.md").unwrap().record.baseline;
        app.create_proposal(&DraftRequest {
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Whole exact proposal".into(),
            changes: vec![
                DraftNoteChange::Replace {
                    path: "replace.md".into(),
                    expected: before.clone(),
                    text: "\u{feff}New λ🦀\r\n".into(),
                },
                DraftNoteChange::Create {
                    path: "new.md".into(),
                    text: "Created\r\n".into(),
                },
                DraftNoteChange::Trash {
                    path: "trash.md".into(),
                    expected: trash,
                },
            ],
            sources: vec![SourceVersion {
                path: "replace.md".into(),
                fingerprint: before,
            }],
        })
        .unwrap()
    }
}
fn approval(proposal: &ProposalRecord) -> ApprovalRequest {
    ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: proposal.stamp(),
    }
}

#[test]
fn whole_application_retains_originals_comments_clear_and_replay_never_repeats() {
    let f = Fixture::new();
    let mut app = f.app();
    let draft = f.draft(&mut app);
    let draft = app
        .add_proposal_comment(&CommentRequest {
            expected: draft.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Temporary".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap();
    let request = approval(&draft);
    let result = app.approve_proposal(&request).unwrap();
    assert_eq!(result.outcome, ApplyOutcome::Applied);
    assert_eq!(
        fs::read(f.vault.join("replace.md")).unwrap(),
        "\u{feff}New λ🦀\r\n".as_bytes()
    );
    assert_eq!(fs::read(f.vault.join("new.md")).unwrap(), b"Created\r\n");
    assert!(!f.vault.join("trash.md").exists());
    let journal = app
        .work_store()
        .proposal_apply(request.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        fs::read(f.vault.join(&journal.members[0].staging)).unwrap(),
        "Original λ\r\n".as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join(&journal.members[2].staging)).unwrap(),
        "Retain original 🦀\r\n".as_bytes()
    );
    assert!(app.proposal(draft.draft.id).unwrap().comments.is_empty());
    fs::write(f.vault.join("replace.md"), "Subsequent owner bytes").unwrap();
    assert_eq!(app.approve_proposal(&request).unwrap(), result);
    drop(app);
    let mut app = f.app();
    assert_eq!(app.approve_proposal(&request).unwrap(), result);
    assert_eq!(
        fs::read_to_string(f.vault.join("replace.md")).unwrap(),
        "Subsequent owner bytes"
    );
}

#[test]
fn stale_approval_and_dirty_editor_fail_without_effects_or_lost_work() {
    let f = Fixture::new();
    let mut app = f.app();
    let draft = f.draft(&mut app);
    let view = app.open_editor("replace.md").unwrap();
    let recovery = app
        .recover_editor(&EditRequest {
            path: "replace.md".into(),
            expected: view.record.stamp,
            generation: view.record.stamp.generation + 1,
            text: "Later typing".into(),
        })
        .unwrap();
    assert!(app.approve_proposal(&approval(&draft)).is_err());
    assert_eq!(app.open_editor("replace.md").unwrap().record, recovery);
    assert_eq!(
        fs::read_to_string(f.vault.join("replace.md")).unwrap(),
        "Original λ\r\n"
    );
    assert!(!f.vault.join("new.md").exists());
    assert!(app.work_store().proposal_applies().unwrap().is_empty());
}

#[test]
fn earlier_healthy_database_recovers_applied_receipt_without_repeating_files() {
    let f = Fixture::new();
    let mut app = f.app();
    let draft = f.draft(&mut app);
    let request = approval(&draft);
    drop(app);
    let earlier = fs::read(f.data.join("brn.sqlite")).unwrap();
    let mut app = f.app();
    let applied = app.approve_proposal(&request).unwrap();
    drop(app);
    fs::write(f.vault.join("replace.md"), "Later owner text").unwrap();
    fs::write(f.data.join("brn.sqlite"), earlier).unwrap();
    let mut app = f.app();
    assert_eq!(app.approve_proposal(&request).unwrap(), applied);
    assert_eq!(
        app.proposal(draft.draft.id).unwrap().state,
        ProposalState::Applied
    );
    assert_eq!(
        fs::read_to_string(f.vault.join("replace.md")).unwrap(),
        "Later owner text"
    );
}

#[test]
fn captured_group_stops_on_refusal_and_never_approves_later_arrivals() {
    let f = Fixture::new();
    let mut app = f.app();
    let group = Uuid::new_v4();
    let mut create = |path: &str| {
        app.create_proposal(&DraftRequest {
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: Some(group),
            session_id: None,
            title: format!("Create {path}"),
            changes: vec![DraftNoteChange::Create {
                path: path.into(),
                text: format!("Approved {path}"),
            }],
            sources: vec![],
        })
        .unwrap()
    };
    let one = create("one.md");
    let two = create("two.md");
    let request = GroupApprovalRequest {
        group_id: group,
        approvals: vec![approval(&one), approval(&two)],
    };
    let late = create("late.md");
    fs::write(f.vault.join("two.md"), "Unexpected occupant").unwrap();
    let result = app.approve_proposal_group(&request).unwrap();
    assert_eq!(result.receipts.len(), 1);
    assert_eq!(result.receipts[0].outcome, ApplyOutcome::Applied);
    assert_eq!(
        result.stopped.unwrap().operation_id,
        request.approvals[1].operation_id
    );
    assert_eq!(
        fs::read_to_string(f.vault.join("two.md")).unwrap(),
        "Unexpected occupant"
    );
    assert!(!f.vault.join("late.md").exists());
    assert_eq!(
        app.proposal(late.draft.id).unwrap().state,
        ProposalState::Draft
    );
    fs::remove_file(f.vault.join("two.md")).unwrap();
    let result = app.approve_proposal_group(&request).unwrap();
    assert_eq!(result.receipts.len(), 2);
    assert!(result.stopped.is_none());
    assert_eq!(
        fs::read_to_string(f.vault.join("two.md")).unwrap(),
        "Approved two.md"
    );
    assert!(!f.vault.join("late.md").exists());
}

#[test]
fn missing_database_and_backups_restore_original_bound_review_from_retained_completion() {
    let f = Fixture::new();
    let mut app = f.app();
    let draft = f.draft(&mut app);
    let request = approval(&draft);
    let result = app.approve_proposal(&request).unwrap();
    drop(app);
    // Delete only this fixture's operational database/backups, retaining its
    // ordinary recovery receipts and synthetic vault.
    fs::remove_file(f.data.join("brn.sqlite")).unwrap();
    fs::remove_dir_all(f.data.join("backups")).unwrap();
    let mut app = f.app();
    assert!(app.open_report().restored_from.is_none());
    assert_eq!(
        app.proposal(draft.draft.id).unwrap().state,
        ProposalState::Applied
    );
    assert_eq!(app.approve_proposal(&request).unwrap(), result);
    assert_eq!(app.note("new.md").unwrap().text, "Created\r\n");
}

#[test]
fn dirty_editor_opened_under_case_or_unicode_alias_refuses_approval() {
    for (target, alias) in [("replace.md", "REPLACE.md"), ("é.md", "e\u{301}.md")] {
        let f = Fixture::new();
        if target != "replace.md" {
            fs::rename(f.vault.join("replace.md"), f.vault.join(target)).unwrap();
        }
        let mut app = f.app();
        let record = app.open_editor(alias).unwrap().record;
        let dirty = app
            .recover_editor(&EditRequest {
                path: alias.into(),
                expected: record.stamp,
                generation: record.stamp.generation + 1,
                text: "Later aliased editor typing".into(),
            })
            .unwrap();
        let proposal = app
            .create_proposal(&DraftRequest {
                inbox_visual: None,
                inbox_knowledge: None,
                inbox_source: None,
                action_changes: Vec::new(),
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Alias conflict".into(),
                changes: vec![DraftNoteChange::Replace {
                    path: target.into(),
                    expected: record.baseline,
                    text: "Proposed".into(),
                }],
                sources: vec![],
            })
            .unwrap();
        assert!(
            app.approve_proposal(&approval(&proposal)).is_err(),
            "{target}/{alias}"
        );
        assert_eq!(app.open_editor(alias).unwrap().record, dirty);
        assert_eq!(
            fs::read_to_string(f.vault.join(target)).unwrap(),
            "Original λ\r\n"
        );
        assert!(app.work_store().proposal_applies().unwrap().is_empty());
    }
}
