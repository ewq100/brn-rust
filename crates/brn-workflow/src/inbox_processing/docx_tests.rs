//! Genuine Stored ZIP32 OOXML, observed through the owned Inbox file boundary.
use super::*;
use crate::{
    app::AppConfig,
    inbox::CaptureBinaryInboxRequest,
    proposal_apply::{ApplyOutcome, ApprovalRequest},
    proposals::{DraftNoteChange, DraftRequest},
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
};

const DOCX: &[u8] = include_bytes!("fixtures/basic-text.docx");
const BODY: &str = "First õ 日本語\n\nSecond preserved\n";
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
    fn queue(&self, app: &mut App, bytes: &[u8]) -> (crate::inbox::InboxItem, ProcessInboxRequest) {
        let capture = CaptureBinaryInboxRequest {
            id: Uuid::new_v4(),
            title: "Genuine synthetic DOCX õ".into(),
            // The parser must use actual package bytes rather than this label.
            original_name: Some("label.txt".into()),
            bytes: bytes.to_vec(),
        };
        let item = app.capture_binary_inbox(&capture).unwrap();
        capture.validate_receipt(&item).unwrap();
        let process = ProcessInboxRequest {
            id: Uuid::new_v4(),
            items: vec![item.clone()],
        };
        app.process_inbox(&process).unwrap();
        (item, process)
    }
    fn prepare(&self, app: &mut App) -> (crate::inbox::InboxItem, DraftRequest) {
        let (item, process) = self.queue(app, DOCX);
        let done = app
            .advance_inbox_processing(process.id, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(
            done.entries[0].outcome,
            InboxProcessOutcome::Converted {
                format: InboxConversionFormat::DocxTextV1,
                byte_len: BODY.len() as u64,
                sha256: digest(BODY.as_bytes())
            }
        );
        assert_eq!(app.process_inbox(&process).unwrap(), done);
        let candidate = InboxCandidateRequest {
            batch_id: process.id,
            index: 0,
        };
        let preview = app.inbox_candidate(&candidate).unwrap();
        preview.validate_receipt(&done).unwrap();
        assert_eq!(preview.markdown, BODY);
        assert!(preview.needs_semantic_review);
        let request = InboxSourceRequest {
            candidate,
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "source.md".into(),
            title: "Review complete DOCX Source".into(),
        };
        let draft = app.prepare_inbox_source(&request).unwrap();
        request.validate_draft(&draft).unwrap();
        (item, draft)
    }
}
fn text(draft: &DraftRequest) -> &str {
    let [DraftNoteChange::Create { text, .. }] = draft.changes.as_slice() else {
        panic!("Source Create")
    };
    text
}
fn original(item: &crate::inbox::InboxItem) -> PathBuf {
    item.capture.copy.directory.join(item.capture.copy_name())
}
fn private_write(path: &std::path::Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn docx_fresh_conversion_source_approval_preserves_complete_original() {
    let f = Fixture::new();
    let mut app = f.app();
    let (item, draft) = f.prepare(&mut app);
    let path = original(&item);
    let inode = path.metadata().unwrap().ino();
    assert!(text(&draft).ends_with(BODY));
    assert!(!f.vault.join("source.md").exists());
    let record = app.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    assert_eq!(
        app.approve_proposal(&request).unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        text(&draft).as_bytes()
    );
    assert_eq!(fs::read(&path).unwrap(), DOCX);
    assert_eq!(path.metadata().unwrap().ino(), inode);
    assert_eq!(
        app.note_provenance("source.md").unwrap().inbox_source,
        Some(draft.inbox_source.as_ref().unwrap().provenance())
    );
    assert!(app.preview_inbox_removal(item.capture.id).is_err());
    assert!(
        app.inbox_original_operations(item.capture.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn docx_candidate_and_unfinished_source_refuse_changed_missing_or_substituted_copy() {
    for damage in 0..5 {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, draft) = f.prepare(&mut app);
        let record = app.create_proposal(&draft).unwrap();
        let path = original(&item);
        match damage {
            0 => fs::remove_file(&path).unwrap(),
            1 => private_write(&path, b"changed exact original"),
            2 => {
                fs::rename(&path, path.with_extension("kept")).unwrap();
                private_write(&path, DOCX);
            }
            3 => fs::hard_link(&path, path.with_extension("alias")).unwrap(),
            _ => fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap(),
        }
        let binding = draft.inbox_source.as_ref().unwrap();
        assert_eq!(
            app.inbox_candidate(&InboxCandidateRequest {
                batch_id: binding.batch_id,
                index: binding.index
            })
            .unwrap_err()
            .kind,
            ErrorKind::ContextStale
        );
        assert_eq!(
            app.approve_proposal(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: record.stamp()
            })
            .unwrap_err()
            .kind,
            ErrorKind::ContextStale
        );
        assert!(
            app.create_proposal(&draft).is_ok(),
            "exact historical creation replay remains available"
        );
        assert!(!f.vault.join("source.md").exists());
    }
}

#[test]
fn docx_binary_byte_observation_refuses_midread_same_bytes_new_inode() {
    let f = Fixture::new();
    let mut app = f.app();
    let (item, draft) = f.prepare(&mut app);
    let path = original(&item);
    let before = path.metadata().unwrap().ino();
    crate::files::inbox::OBSERVE_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            fs::rename(&path, path.with_extension("kept")).unwrap();
            private_write(&path, DOCX);
        }));
    });
    let binding = draft.inbox_source.as_ref().unwrap();
    assert!(
        app.inbox_candidate(&InboxCandidateRequest {
            batch_id: binding.batch_id,
            index: 0
        })
        .is_err()
    );
    assert_ne!(original(&item).metadata().unwrap().ino(), before);
    assert!(app.proposals(None).unwrap().is_empty());
    assert!(!f.vault.join("source.md").exists());
}

#[test]
fn docx_forged_self_consistent_receipt_and_source_body_are_not_fresh_authority() {
    let f = Fixture::new();
    let mut app = f.app();
    let (item, process) = f.queue(&mut app, DOCX);
    app.work_store_mut()
        .start_inbox_processing(process.id, 0)
        .unwrap();
    let fake = "Invented output with consistent hash\n";
    app.work_store_mut()
        .finish_inbox_processing(
            process.id,
            0,
            InboxProcessOutcome::Converted {
                format: InboxConversionFormat::DocxTextV1,
                byte_len: fake.len() as u64,
                sha256: digest(fake.as_bytes()),
            },
        )
        .unwrap();
    let candidate = InboxCandidateRequest {
        batch_id: process.id,
        index: 0,
    };
    assert_eq!(
        app.inbox_candidate(&candidate).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    assert!(
        app.prepare_inbox_source(&InboxSourceRequest {
            candidate,
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "source.md".into(),
            title: "Forged Source".into()
        })
        .is_err()
    );
    let binding = InboxSourceBinding {
        batch_id: process.id,
        index: 0,
        original: item,
        format: InboxConversionFormat::DocxTextV1,
        byte_len: fake.len() as u64,
        sha256: digest(fake.as_bytes()),
        note_id: Uuid::new_v4(),
    };
    let draft = DraftRequest {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Consistent forged wrapper".into(),
        changes: vec![DraftNoteChange::Create {
            path: "source.md".into(),
            text: binding.markdown(fake).unwrap(),
        }],
        sources: vec![],
        action_changes: vec![],
        inbox_source: Some(Box::new(binding)),
        inbox_knowledge: None,
    };
    draft.validate().unwrap();
    assert!(app.create_proposal(&draft).is_err());
    assert!(app.proposals(None).unwrap().is_empty());
    assert!(!f.vault.join("source.md").exists());
}

#[test]
fn docx_unfinished_approval_rederives_without_disposable_processing_rows() {
    let f = Fixture::new();
    let mut app = f.app();
    let (_, draft) = f.prepare(&mut app);
    let record = app.create_proposal(&draft).unwrap();
    let raw = rusqlite::Connection::open(f.data.join("brn.sqlite")).unwrap();
    raw.execute("DELETE FROM inbox_processing", []).unwrap();
    raw.execute("DELETE FROM inbox_items", []).unwrap();
    drop(raw);
    assert_eq!(
        app.approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp()
        })
        .unwrap()
        .outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        text(&draft).as_bytes()
    );
}

#[test]
fn docx_terminal_source_recovers_fresh_sql_without_original_or_processing() {
    let f = Fixture::new();
    let mut app = f.app();
    let (item, draft) = f.prepare(&mut app);
    let record = app.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    let receipt = app.approve_proposal(&request).unwrap();
    drop(app);
    fs::remove_file(original(&item)).unwrap();
    fs::remove_file(f.data.join("brn.sqlite")).unwrap();
    fs::remove_dir_all(f.data.join("backups")).unwrap();
    let mut app = f.app();
    assert_eq!(app.approve_proposal(&request).unwrap(), receipt);
    assert_eq!(
        app.create_proposal(&draft).unwrap().draft.inbox_source,
        draft.inbox_source
    );
    assert!(
        app.inbox_processing(draft.inbox_source.as_ref().unwrap().batch_id)
            .is_err()
    );
    assert_eq!(
        app.inbox_item(item.capture.id).unwrap().original,
        InboxOriginal::Missing
    );
    assert!(
        !original(&item).exists(),
        "historical import never resurrects original bytes"
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        text(&draft).as_bytes()
    );
}

#[test]
fn docx_queue_cancel_interrupt_and_original_damage_have_durable_outcomes() {
    let f = Fixture::new();
    let mut app = f.app();
    let (_, cancelled) = f.queue(&mut app, DOCX);
    let done = app
        .advance_inbox_processing(cancelled.id, &AtomicBool::new(true))
        .unwrap();
    assert_eq!(done.entries[0].outcome, InboxProcessOutcome::Cancelled);
    assert_eq!(app.process_inbox(&cancelled).unwrap(), done);
    let (_, interrupted) = f.queue(&mut app, DOCX);
    app.work_store_mut()
        .start_inbox_processing(interrupted.id, 0)
        .unwrap();
    drop(app);
    let mut app = f.app();
    assert_eq!(
        app.inbox_processing(interrupted.id).unwrap().entries[0].outcome,
        InboxProcessOutcome::Interrupted
    );
    for missing in [false, true] {
        let (item, process) = f.queue(&mut app, DOCX);
        if missing {
            fs::remove_file(original(&item)).unwrap();
        } else {
            private_write(&original(&item), b"changed");
        }
        let done = app
            .advance_inbox_processing(process.id, &AtomicBool::new(false))
            .unwrap();
        assert!(
            matches!(&done.entries[0].outcome, InboxProcessOutcome::Failed { code } if code == if missing { "original_missing" } else { "original_changed" })
        );
        assert_eq!(app.process_inbox(&process).unwrap(), done);
    }
}

#[test]
fn docx_original_loss_during_apply_keeps_existing_eligibility_fences() {
    for (phase, expected) in [
        ("prepared", ApplyOutcome::NotApplied),
        ("member", ApplyOutcome::Uncertain),
    ] {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, draft) = f.prepare(&mut app);
        let record = app.create_proposal(&draft).unwrap();
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        };
        let path = original(&item);
        crate::proposal_apply::APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |step, _| {
                if step == phase {
                    fs::remove_file(&path).unwrap();
                }
            }));
        });
        let result = app.approve_proposal(&request);
        crate::proposal_apply::APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert!(result.is_err());
        let journal = app
            .work_store()
            .proposal_apply(request.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(journal.receipt.unwrap().outcome, expected);
        assert_eq!(f.vault.join("source.md").exists(), phase == "member");
        drop(app);
        let mut app = f.app();
        assert_eq!(
            app.reconcile_proposal(request.operation_id)
                .unwrap()
                .outcome,
            expected
        );
    }
}

#[test]
fn docx_malformed_actual_package_failure_is_durable_and_never_has_a_candidate() {
    let f = Fixture::new();
    let mut app = f.app();
    let mut damaged = DOCX.to_vec();
    let start = damaged
        .windows(b"First".len())
        .position(|w| w == b"First")
        .unwrap();
    damaged[start] = b'X'; // Exact capture observes this; ZIP CRC must still refuse it.
    let (item, process) = f.queue(&mut app, &damaged);
    let terminal = app
        .advance_inbox_processing(process.id, &AtomicBool::new(false))
        .unwrap();
    assert!(
        matches!(&terminal.entries[0].outcome, InboxProcessOutcome::Failed { code } if code == "docx_invalid")
    );
    assert!(
        app.inbox_candidate(&InboxCandidateRequest {
            batch_id: process.id,
            index: 0
        })
        .is_err()
    );
    assert_eq!(fs::read(original(&item)).unwrap(), damaged);
    drop(app);
    let mut app = f.app();
    assert_eq!(app.process_inbox(&process).unwrap(), terminal);
    assert!(app.proposals(None).unwrap().is_empty());
}
