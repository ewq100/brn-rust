#![cfg(target_os = "macos")]
use brn_ai::{AiErrorKind, ReadTools};
use brn_store::work::proposal_apply::{ApplyMemberProof, ApplyOutcome, ApprovalRequest};
use brn_workflow::{
    ErrorKind,
    app::{App, AppConfig},
    editor::{EditRequest, ReloadRequest, SaveOutcome, SaveRequest},
    library::SearchMode,
    proposals::{DraftNoteChange, DraftRequest, NoteChange, ProposalRecord},
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
        fs::write(vault.join("note.md"), "Saved λ\r\n").unwrap();
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
}
fn draft(app: &mut App) -> ProposalRecord {
    let before = app.open_editor("note.md").unwrap().record.baseline;
    app.create_proposal(&DraftRequest {
        inbox_source: None,
        action_changes: Vec::new(),
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Whole proposal".into(),
        changes: vec![DraftNoteChange::Replace {
            path: "note.md".into(),
            expected: before,
            text: "Proposed 🦀\r\n".into(),
        }],
        sources: vec![],
    })
    .unwrap()
}
fn request(proposal: &ProposalRecord) -> ApprovalRequest {
    ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: proposal.stamp(),
    }
}
fn unchanged(proposal: &ProposalRecord) -> Vec<ApplyMemberProof> {
    let NoteChange::Replace { before, .. } = &proposal.draft.changes[0] else {
        panic!()
    };
    vec![ApplyMemberProof {
        destination: Some(before.clone()),
        staging: None,
    }]
}

#[test]
fn pending_and_uncertain_proposals_block_all_current_reads_and_preserve_editor_recovery() {
    let f = Fixture::new();
    let mut app = f.app();
    let proposal = draft(&mut app);
    let approval = request(&proposal);
    let retained = app.tools().unwrap();
    assert_eq!(retained.read_note("note.md").unwrap().text, "Saved λ\r\n");
    app.work_store_mut()
        .begin_proposal_apply(&approval)
        .unwrap();
    assert_eq!(app.tools().err().unwrap().kind, ErrorKind::SaveUncertain);
    assert_eq!(
        retained.read_note("note.md").unwrap_err().kind,
        AiErrorKind::IndexStale
    );
    assert_eq!(
        app.note("note.md").unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        app.notes(None, None).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(app.refresh().unwrap_err().kind, ErrorKind::SaveUncertain);
    assert_eq!(
        app.embed_pending(1).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        app.search("Saved", SearchMode::Keyword, 5)
            .unwrap_err()
            .kind,
        ErrorKind::SaveUncertain
    );
    let current = app.open_editor("note.md").unwrap();
    assert!(current.conflict);
    let edit = EditRequest {
        path: "note.md".into(),
        expected: current.record.stamp,
        generation: current.record.stamp.generation + 1,
        text: "My unfinished work 🦀".into(),
    };
    app.recover_editor(&edit).unwrap();
    let original_save = SaveRequest {
        operation_id: Uuid::new_v4(),
        edit: edit.clone(),
        destination: None,
    };
    assert_eq!(
        app.save_editor(&original_save).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    let copy_save = SaveRequest {
        operation_id: Uuid::new_v4(),
        destination: Some("copy.md".into()),
        ..original_save
    };
    assert_eq!(
        app.save_editor(&copy_save).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        app.reload_editor(&ReloadRequest {
            path: "note.md".into(),
            expected: current.record.stamp,
            observed: current.observed.unwrap(),
            discard: true,
        })
        .unwrap_err()
        .kind,
        ErrorKind::SaveUncertain
    );
    assert!(app.work_store().editor_saves().unwrap().is_empty());
    assert!(!f.vault.join("copy.md").exists());
    app.work_store_mut()
        .finish_proposal_apply(approval.operation_id, ApplyOutcome::Uncertain, None)
        .unwrap();
    drop(retained);
    drop(app);
    let mut app = f.app();
    assert_eq!(app.tools().err().unwrap().kind, ErrorKind::SaveUncertain);
    assert_eq!(
        app.note("note.md").unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    let recovered = app.open_editor("note.md").unwrap();
    assert_eq!(recovered.record.text, "My unfinished work 🦀");
    assert_eq!(recovered.saved.as_deref(), Some("Saved λ\r\n"));
    assert!(recovered.conflict);
    // Settling an operation at storage level does not perform file effects.
    app.work_store_mut()
        .finish_proposal_apply(
            approval.operation_id,
            ApplyOutcome::NotApplied,
            Some(&unchanged(&proposal)),
        )
        .unwrap();
    drop(app);
    let mut app = f.app();
    assert_eq!(
        app.tools().unwrap().read_note("note.md").unwrap().text,
        "Saved λ\r\n"
    );
    assert_eq!(app.notes(None, None).unwrap().notes.len(), 1);
    assert_eq!(
        app.open_editor("note.md").unwrap().record.text,
        "My unfinished work 🦀"
    );
}

#[test]
fn editor_reconciliation_cannot_clear_an_unresolved_proposal_fence() {
    let f = Fixture::new();
    let mut app = f.app();
    let proposal = draft(&mut app);
    let current = app.open_editor("note.md").unwrap().record;
    let save = SaveRequest {
        operation_id: Uuid::new_v4(),
        destination: None,
        edit: EditRequest {
            path: "note.md".into(),
            expected: current.stamp,
            generation: current.stamp.generation + 1,
            text: "Unfinished editor".into(),
        },
    };
    let stage = PathBuf::from(format!(".brn-{}.stage", save.operation_id));
    let retained = app.tools().unwrap();
    // Simulate recovery journals coexisting after restore; no file write occurred.
    app.work_store_mut()
        .begin_editor_save(&save, &stage)
        .unwrap();
    let approval = request(&proposal);
    app.work_store_mut()
        .begin_proposal_apply(&approval)
        .unwrap();
    assert_eq!(app.tools().err().unwrap().kind, ErrorKind::SaveUncertain);
    let receipt = app.reconcile_editor(save.operation_id).unwrap();
    assert_eq!(receipt.outcome, SaveOutcome::NotApplied);
    assert_eq!(
        retained.read_note("note.md").unwrap_err().kind,
        AiErrorKind::IndexStale
    );
    assert_eq!(
        app.note("note.md").unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        app.work_store().editor("note.md").unwrap().unwrap().text,
        "Unfinished editor"
    );
    assert_eq!(
        fs::read_to_string(f.vault.join("note.md")).unwrap(),
        "Saved λ\r\n"
    );
}
