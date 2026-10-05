//! Source proposal authority, exact edits and ordinary recovery.
use super::*;
use crate::{
    app::{App, AppConfig},
    inbox::CaptureInboxRequest,
    proposal_apply::{ApplyOutcome, ApprovalRequest},
    proposals::{DraftNoteChange, ProposalEdit},
};
use std::{fs, path::PathBuf};

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        Self {
            _owner: owner,
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
    fn prepare(
        &self,
        app: &mut App,
        kind: InboxKind,
        text: &str,
    ) -> crate::proposals::DraftRequest {
        let item = app
            .capture_inbox(&CaptureInboxRequest {
                id: Uuid::new_v4(),
                kind,
                title: "Source õ".into(),
                original_name: Some("original.eml".into()),
                text: text.into(),
            })
            .unwrap();
        let batch_id = Uuid::new_v4();
        app.process_inbox(&ProcessInboxRequest {
            id: batch_id,
            items: vec![item],
        })
        .unwrap();
        app.advance_inbox_processing(batch_id, &AtomicBool::new(false))
            .unwrap();
        let request = InboxSourceRequest {
            candidate: InboxCandidateRequest { batch_id, index: 0 },
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "source.md".into(),
            title: "Review exact source".into(),
        };
        let draft = app.prepare_inbox_source(&request).unwrap();
        request.validate_draft(&draft).unwrap();
        draft
    }
}
fn text(draft: &crate::proposals::DraftRequest) -> &str {
    let DraftNoteChange::Create { text, .. } = &draft.changes[0] else {
        panic!("create")
    };
    text
}
fn original_path(draft: &crate::proposals::DraftRequest) -> PathBuf {
    let capture = &draft.inbox_source.as_ref().unwrap().original.capture;
    capture.copy.directory.join(capture.copy_name())
}

#[test]
fn source_requires_approval_and_preserves_imported_frontmatter_as_exact_body() {
    let f = Fixture::new();
    let mut app = f.app();
    let imported = "\u{feff}---\r\nbrn_id: invalid\r\nbrn_kind: knowledge\r\nbrn_provenance: []\r\n---\r\n# õ 日本語\0";
    let draft = f.prepare(&mut app, InboxKind::Markdown, imported);
    assert!(text(&draft).ends_with(imported));
    assert!(!f.vault.join("source.md").exists());
    let proposal = app.create_proposal(&draft).unwrap();
    let mut unknown_batch = draft.clone();
    unknown_batch.id = Uuid::new_v4();
    unknown_batch.inbox_source.as_mut().unwrap().batch_id = Uuid::new_v4();
    assert!(app.create_proposal(&unknown_batch).is_err());
    assert!(!f.vault.join("source.md").exists());
    assert!(app.proposals(None).unwrap().iter().any(|p| p == &proposal));
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: proposal.stamp(),
    };
    assert_eq!(
        app.approve_proposal(&approval).unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        text(&draft).as_bytes()
    );
    assert_eq!(
        fs::read(original_path(&draft)).unwrap(),
        imported.as_bytes()
    );
    assert!(
        brn_store::note_metadata::classify(text(&draft))
            .unwrap()
            .source
    );
    assert_eq!(
        brn_store::note_identity::read(text(&draft)).unwrap(),
        Some(draft.inbox_source.as_ref().unwrap().note_id)
    );
    let provenance = app.note_provenance("source.md").unwrap();
    assert!(provenance.citations.is_empty());
    assert_eq!(
        provenance.inbox_source,
        Some(draft.inbox_source.as_ref().unwrap().provenance())
    );
    assert!(
        !serde_json::to_string(&provenance)
            .unwrap()
            .contains(f.data.to_str().unwrap())
    );
}

#[test]
fn source_copy_does_not_assert_imported_uuid_links_as_new_current_relationships() {
    let f = Fixture::new();
    let mut app = f.app();
    let imported = format!("[An old source link](brn://note/{})\n", Uuid::new_v4());
    let draft = f.prepare(&mut app, InboxKind::Markdown, &imported);
    let proposal = app.create_proposal(&draft).unwrap();
    assert_eq!(
        app.approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: proposal.stamp()
        })
        .unwrap()
        .outcome,
        ApplyOutcome::Applied
    );
    assert!(
        fs::read_to_string(f.vault.join("source.md"))
            .unwrap()
            .ends_with(&imported)
    );
}

#[test]
fn missing_changed_forged_conversion_and_duplicate_identity_refuse_before_effects() {
    for missing in [false, true] {
        let f = Fixture::new();
        let mut app = f.app();
        let draft = f.prepare(&mut app, InboxKind::Email, "exact email\r\nõ");
        let mut forged = draft.clone();
        let binding = forged.inbox_source.as_mut().unwrap();
        let forged_body = "```text\nforged completely different email\n```\n";
        binding.byte_len = forged_body.len() as u64;
        binding.sha256 = digest(forged_body.as_bytes());
        let fake = binding.markdown(forged_body).unwrap();
        let DraftNoteChange::Create { text, .. } = &mut forged.changes[0] else {
            unreachable!()
        };
        *text = fake;
        assert!(app.create_proposal(&forged).is_err());
        let proposal = app.create_proposal(&draft).unwrap();
        if missing {
            fs::remove_file(original_path(&draft)).unwrap();
        } else {
            fs::write(original_path(&draft), "changed").unwrap();
        }
        assert_eq!(
            app.approve_proposal(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: proposal.stamp()
            })
            .unwrap_err()
            .kind,
            ErrorKind::ContextStale
        );
        assert!(
            app.create_proposal(&draft).is_ok(),
            "creation replay remains exact history"
        );
        assert!(!f.vault.join("source.md").exists());
    }
    let f = Fixture::new();
    let mut app = f.app();
    let draft = f.prepare(&mut app, InboxKind::Text, "copy");
    let proposal = app.create_proposal(&draft).unwrap();
    fs::write(
        f.vault.join("duplicate.md"),
        format!(
            "---\nbrn_id: {}\n---\n",
            draft.inbox_source.as_ref().unwrap().note_id
        ),
    )
    .unwrap();
    assert_eq!(
        app.approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: proposal.stamp()
        })
        .unwrap_err()
        .kind,
        ErrorKind::ContextStale
    );
    assert!(!f.vault.join("source.md").exists());
}

#[test]
fn review_can_change_title_but_cannot_reinterpret_source_or_drop_binding() {
    let f = Fixture::new();
    let mut app = f.app();
    let draft = f.prepare(&mut app, InboxKind::Teams, "~~~\nexact 👋\r\n");
    let proposal = app.create_proposal(&draft).unwrap();
    let mut edit = ProposalEdit {
        expected: proposal.stamp(),
        title: "Reviewed source".into(),
        texts: vec![Some(text(&draft).to_owned())],
        action_data: Vec::new(),
    };
    let renamed = app.edit_proposal(&edit).unwrap();
    assert_eq!(renamed.draft.inbox_source, proposal.draft.inbox_source);
    edit.expected = renamed.stamp();
    edit.texts = vec![Some(text(&draft).replace("exact", "interpreted"))];
    assert!(app.edit_proposal(&edit).is_err());
    assert!(app.rewrite_proposal(&edit).is_err());
    assert_eq!(app.proposal(draft.id).unwrap(), renamed);
    let mut detached = draft.clone();
    detached.inbox_source = None;
    assert!(
        app.create_proposal(&detached).is_err(),
        "UUID replay cannot remove original proof"
    );
    detached.id = Uuid::new_v4();
    let detached = app.create_proposal(&detached).unwrap();
    assert!(
        app.approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: detached.stamp()
        })
        .is_err(),
        "invented Inbox provenance cannot be approved"
    );
}

#[test]
fn completed_source_history_recovers_without_processing_rows_database_or_original() {
    let f = Fixture::new();
    let mut app = f.app();
    let draft = f.prepare(&mut app, InboxKind::Email, "retained email\r\n");
    let proposal = app.create_proposal(&draft).unwrap();
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: proposal.stamp(),
    };
    let receipt = app.approve_proposal(&approval).unwrap();
    drop(app);
    fs::remove_file(original_path(&draft)).unwrap();
    fs::remove_file(f.data.join("brn.sqlite")).unwrap();
    fs::remove_dir_all(f.data.join("backups")).unwrap();
    let mut app = f.app();
    assert_eq!(app.approve_proposal(&approval).unwrap(), receipt);
    assert_eq!(
        app.create_proposal(&draft).unwrap().draft.inbox_source,
        draft.inbox_source
    );
    assert!(
        app.inbox_processing(draft.inbox_source.as_ref().unwrap().batch_id)
            .is_err()
    );
    assert_eq!(
        app.note_provenance("source.md").unwrap().inbox_source,
        Some(draft.inbox_source.as_ref().unwrap().provenance())
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        text(&draft).as_bytes()
    );
}

#[test]
fn recovery_cleanup_preserves_a_valid_temporary_with_another_original_binding() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let mut app = f.app();
    let draft = f.prepare(&mut app, InboxKind::Text, "Exact source");
    let proposal = app.create_proposal(&draft).unwrap();
    let proposal = app
        .edit_proposal(&ProposalEdit {
            expected: proposal.stamp(),
            title: "Reviewed exact source".into(),
            texts: vec![Some(text(&draft).into())],
            action_data: Vec::new(),
        })
        .unwrap();
    assert_eq!(proposal.version, 2);
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: proposal.stamp(),
    };
    app.approve_proposal(&approval).unwrap();
    let journal = app
        .work_store()
        .proposal_apply(approval.operation_id)
        .unwrap()
        .unwrap();
    let mut foreign = journal.clone();
    foreign
        .approved
        .draft
        .inbox_source
        .as_mut()
        .unwrap()
        .original
        .capture
        .copy
        .directory_inode += 100;
    foreign.validate().unwrap();
    let foreign_dir = f.data.join("synthetic-foreign");
    fs::create_dir(&foreign_dir).unwrap();
    fs::set_permissions(&foreign_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let foreign_files = crate::files::recovery::ApplyRecoveryFiles::open(&foreign_dir).unwrap();
    let snapshot = foreign_files.write(&foreign, None).unwrap();
    let bytes = fs::read(foreign_dir.join(snapshot.proof.relative)).unwrap();
    let temporary = f
        .data
        .join(format!(".brn-apply-temp-{}.stage", Uuid::new_v4()));
    fs::write(&temporary, &bytes).unwrap();
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600)).unwrap();
    let files = crate::files::recovery::ApplyRecoveryFiles::open(&f.data).unwrap();
    files.retire_review_temporaries(&journal).unwrap();
    assert_eq!(
        fs::read(&temporary).unwrap(),
        bytes,
        "a differing immutable Inbox proof is never covered cleanup"
    );
}

#[test]
fn source_wrapper_refuses_oversize_without_truncating_or_writing() {
    let f = Fixture::new();
    let mut app = f.app();
    let item = app
        .capture_inbox(&CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: InboxKind::Markdown,
            title: "Large exact copy".into(),
            original_name: None,
            text: "x".repeat(crate::MAX_NOTE_BYTES),
        })
        .unwrap();
    let batch_id = Uuid::new_v4();
    app.process_inbox(&ProcessInboxRequest {
        id: batch_id,
        items: vec![item.clone()],
    })
    .unwrap();
    app.advance_inbox_processing(batch_id, &AtomicBool::new(false))
        .unwrap();
    assert!(
        app.prepare_inbox_source(&InboxSourceRequest {
            candidate: InboxCandidateRequest { batch_id, index: 0 },
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "source.md".into(),
            title: "Full source".into()
        })
        .is_err()
    );
    assert!(app.proposals(None).unwrap().is_empty());
    assert!(!f.vault.join("source.md").exists());
    assert_eq!(
        fs::read(item.capture.copy.directory.join(item.capture.copy_name()))
            .unwrap()
            .len(),
        crate::MAX_NOTE_BYTES
    );
}

#[test]
fn mid_application_original_loss_refuses_effects_or_keeps_unknown_work_fenced() {
    for (phase, expected) in [
        ("prepared", ApplyOutcome::NotApplied),
        ("member", ApplyOutcome::Uncertain),
    ] {
        let f = Fixture::new();
        let mut app = f.app();
        let draft = f.prepare(&mut app, InboxKind::Text, "source proof stays exact");
        let proposal = app.create_proposal(&draft).unwrap();
        let approval = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: proposal.stamp(),
        };
        let original = original_path(&draft);
        crate::proposal_apply::APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |step, _| {
                if step == phase {
                    fs::remove_file(&original).unwrap();
                }
            }))
        });
        let result = app.approve_proposal(&approval);
        crate::proposal_apply::APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert!(result.is_err());
        let journal = app
            .work_store()
            .proposal_apply(approval.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(journal.receipt.unwrap().outcome, expected);
        assert_eq!(f.vault.join("source.md").exists(), phase == "member");
        drop(app);
        let mut app = f.app();
        assert_eq!(
            app.reconcile_proposal(approval.operation_id)
                .unwrap()
                .outcome,
            expected
        );
        if phase == "member" {
            assert_eq!(
                app.note("source.md").unwrap_err().kind,
                ErrorKind::SaveUncertain
            );
            let preview = app.preview_proposal_repair(approval.operation_id).unwrap();
            let repair = crate::proposal_apply::RepairRequest {
                id: Uuid::new_v4(),
                operation_id: approval.operation_id,
                expected: preview.expected,
                direction: crate::proposal_apply::RepairDirection::Finish,
            };
            assert!(
                app.repair_proposal(&repair).is_err(),
                "Finish cannot waive missing original proof"
            );
            let mut restore = repair;
            restore.id = Uuid::new_v4();
            restore.direction = crate::proposal_apply::RepairDirection::Restore;
            app.repair_proposal(&restore).unwrap();
            assert!(!f.vault.join("source.md").exists());
        }
    }
}
