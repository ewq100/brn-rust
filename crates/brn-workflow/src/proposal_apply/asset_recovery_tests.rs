//! Actual opaque bytes through the existing application, repair and Undo family.
use super::*;
use crate::{app::AppConfig, proposals::*};
use brn_ai::ReadTools;
use std::{fs, path::PathBuf, process::Command};

struct Fixture {
    base: tempfile::TempDir,
    original: Vec<u8>,
    after: Vec<u8>,
    trash: Vec<u8>,
}

fn open(base: &Path, data: &Path) -> App {
    App::open(
        data,
        AppConfig {
            vault_root: Some(base.join("vault")),
            credentials_dir: Some(base.join("credentials")),
            model_dir: None,
        },
    )
    .unwrap()
}

impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        fs::create_dir(base.path().join("vault")).unwrap();
        fs::create_dir(base.path().join("vault/assets")).unwrap();
        fs::create_dir(base.path().join("data")).unwrap();
        let original = b"\0\xffOriginal opaque asset\r\n".to_vec();
        let after = b"\xff\0Reviewed opaque replacement\r\n".to_vec();
        let trash = b"\xfe\0Retained opaque Trash\r\n".to_vec();
        fs::write(base.path().join("vault/assets/a.bin"), &original).unwrap();
        fs::write(base.path().join("vault/assets/trash.bin"), &trash).unwrap();
        fs::write(base.path().join("vault/source.md"), "Saved source λ\r\n").unwrap();
        Self {
            base,
            original,
            after,
            trash,
        }
    }
    fn app(&self) -> App {
        open(self.base.path(), &self.base.path().join("data"))
    }
    fn draft(&self, app: &mut App) -> DraftRequest {
        let before = app.proposal_asset("assets/a.bin").unwrap().fingerprint;
        let trash = app.proposal_asset("assets/trash.bin").unwrap().fingerprint;
        DraftRequest {
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: vec![],
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Exact mixed opaque review".into(),
            changes: vec![
                DraftNoteChange::ReplaceAsset {
                    path: "assets/a.bin".into(),
                    expected: before,
                    bytes: self.after.clone(),
                },
                DraftNoteChange::CreateAsset {
                    path: "assets/new.bin".into(),
                    bytes: vec![],
                },
                DraftNoteChange::TrashAsset {
                    path: "assets/trash.bin".into(),
                    expected: trash,
                },
                DraftNoteChange::Create {
                    path: "new.md".into(),
                    text: "Exact approved Markdown 🦀\r\n".into(),
                },
            ],
            sources: vec![app.proposal_source("source.md").unwrap().source],
        }
    }
    fn prepare(&self) -> ApprovalRequest {
        let mut app = self.app();
        let request = self.draft(&mut app);
        let review = app.create_proposal(&request).unwrap();
        let review = app
            .add_proposal_comment(&CommentRequest {
                expected: review.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Full opaque review retained until approval".into(),
                    target: CommentTarget::Proposal,
                },
            })
            .unwrap();
        let approval = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        };
        fs::write(
            self.base.path().join("request.json"),
            serde_json::to_vec(&approval).unwrap(),
        )
        .unwrap();
        approval
    }
    fn crash(&self, phase: &str, member: usize, mode: &str) {
        let _guard = crate::SUBPROCESS_FIXTURES.lock().unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "proposal_apply::asset_recovery_tests::asset_crash_child",
                "--ignored",
                "--test-threads=1",
            ])
            .env("BRN_ASSET_TEST_BASE", self.base.path())
            .env("BRN_ASSET_TEST_PHASE", phase)
            .env("BRN_ASSET_TEST_MEMBER", member.to_string())
            .env("BRN_ASSET_TEST_MODE", mode)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(86),
            "{mode} {phase}/{member}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fn assert_applied(&self) {
        assert_eq!(
            fs::read(self.base.path().join("vault/assets/a.bin")).unwrap(),
            self.after
        );
        assert_eq!(
            fs::read(self.base.path().join("vault/assets/new.bin")).unwrap(),
            b""
        );
        assert!(!self.base.path().join("vault/assets/trash.bin").exists());
        assert_eq!(
            fs::read(self.base.path().join("vault/new.md")).unwrap(),
            "Exact approved Markdown 🦀\r\n".as_bytes()
        );
    }
    fn assert_originals(&self) {
        assert_eq!(
            fs::read(self.base.path().join("vault/assets/a.bin")).unwrap(),
            self.original
        );
        assert_eq!(
            fs::read(self.base.path().join("vault/assets/trash.bin")).unwrap(),
            self.trash
        );
        assert!(!self.base.path().join("vault/assets/new.bin").exists());
        assert!(!self.base.path().join("vault/new.md").exists());
    }
    fn copy_records(&self, label: &str) -> PathBuf {
        let data = self.base.path().join(label);
        fs::create_dir(&data).unwrap();
        for entry in fs::read_dir(self.base.path().join("data")).unwrap() {
            let entry = entry.unwrap();
            if entry
                .file_name()
                .to_str()
                .unwrap()
                .starts_with(".brn-apply-")
            {
                fs::copy(entry.path(), data.join(entry.file_name())).unwrap();
            }
        }
        data
    }
}

#[test]
#[ignore = "private subprocess entry exercised by actual asset crash matrices"]
fn asset_crash_child() {
    let base = PathBuf::from(std::env::var_os("BRN_ASSET_TEST_BASE").unwrap());
    let phase = std::env::var("BRN_ASSET_TEST_PHASE").unwrap();
    let member = std::env::var("BRN_ASSET_TEST_MEMBER")
        .unwrap()
        .parse()
        .unwrap();
    let mut app = open(&base, &base.join("data"));
    APPLY_CHECKPOINT.with(|stop| *stop.borrow_mut() = Some((phase, member)));
    match std::env::var("BRN_ASSET_TEST_MODE").unwrap().as_str() {
        "apply" => {
            let request =
                serde_json::from_slice(&fs::read(base.join("request.json")).unwrap()).unwrap();
            app.approve_proposal(&request).unwrap();
        }
        "undo" => {
            let request =
                serde_json::from_slice(&fs::read(base.join("undo.json")).unwrap()).unwrap();
            app.undo_proposal(&request).unwrap();
        }
        "repair" => {
            let request =
                serde_json::from_slice(&fs::read(base.join("repair.json")).unwrap()).unwrap();
            app.repair_proposal(&request).unwrap();
        }
        _ => panic!("unknown private fixture mode"),
    }
    panic!("selected asset crash checkpoint not reached");
}

#[test]
fn assets_apply_replay_and_whole_or_scoped_undo_preserve_exact_bytes() {
    for scope in [None, Some(2)] {
        let f = Fixture::new();
        let approval = f.prepare();
        let mut app = f.app();
        let result = app.approve_proposal(&approval).unwrap();
        assert_eq!(result.outcome, ApplyOutcome::Applied);
        f.assert_applied();
        assert!(
            app.proposal(approval.expected.id)
                .unwrap()
                .comments
                .is_empty()
        );
        assert!(app.note("assets/a.bin").is_err());
        assert!(app.tools().unwrap().read_note("assets/a.bin").is_err());
        assert_eq!(app.approve_proposal(&approval).unwrap(), result);
        let undo = UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: approval.operation_id,
            trash_member: scope,
        };
        let result = app.undo_proposal(&undo).unwrap();
        assert_eq!(result.outcome, ApplyOutcome::Applied);
        assert_eq!(app.undo_proposal(&undo).unwrap(), result);
        if scope.is_none() {
            f.assert_originals();
            let redo = UndoRequest {
                operation_id: Uuid::new_v4(),
                target_operation_id: undo.operation_id,
                trash_member: None,
            };
            assert_eq!(
                app.undo_proposal(&redo).unwrap().outcome,
                ApplyOutcome::Applied
            );
            f.assert_applied();
        } else {
            assert_eq!(
                fs::read(f.base.path().join("vault/assets/trash.bin")).unwrap(),
                f.trash
            );
            assert_eq!(
                fs::read(f.base.path().join("vault/assets/a.bin")).unwrap(),
                f.after
            );
            assert!(f.base.path().join("vault/new.md").exists());
        }
        drop(app);
        let mut reopened = f.app();
        assert_eq!(reopened.undo_proposal(&undo).unwrap(), result);
        assert!(reopened.tools().is_ok());
    }
}

#[test]
fn maximum_asset_replacement_recovers_from_fresh_sql_and_undo_without_text_decoding() {
    let f = Fixture::new();
    let original = vec![0xff; MAX_ASSET_BYTES];
    let after = vec![0xfe; MAX_ASSET_BYTES];
    fs::write(f.base.path().join("vault/assets/a.bin"), &original).unwrap();
    let mut app = f.app();
    let before = app.proposal_asset("assets/a.bin").unwrap().fingerprint;
    let review = app
        .create_proposal(&DraftRequest {
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: vec![],
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Full two-body 16 MiB asset".into(),
            changes: vec![DraftNoteChange::ReplaceAsset {
                path: "assets/a.bin".into(),
                expected: before,
                bytes: after.clone(),
            }],
            sources: vec![],
        })
        .unwrap();
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    assert_eq!(
        app.approve_proposal(&approval).unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(
        fs::read(f.base.path().join("vault/assets/a.bin")).unwrap(),
        after
    );
    drop(app);
    let data = f.copy_records("fresh-max-asset");
    let mut app = open(f.base.path(), &data);
    let recovered = app.proposal_apply(approval.operation_id).unwrap().unwrap();
    assert_eq!(recovered.approved.draft, review.draft);
    assert_eq!(recovered.receipt.unwrap().outcome, ApplyOutcome::Applied);
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: approval.operation_id,
        trash_member: None,
    };
    assert_eq!(
        app.undo_proposal(&undo).unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(
        fs::read(f.base.path().join("vault/assets/a.bin")).unwrap(),
        original
    );
    drop(app);
    let mut app = open(f.base.path(), &data);
    assert_eq!(
        app.undo_proposal(&undo).unwrap().outcome,
        ApplyOutcome::Applied
    );
}

#[test]
fn mixed_asset_comment_cleanup_restores_fresh_sql_without_reviving_old_annotations() {
    let f = Fixture::new();
    let approval = f.prepare();
    let mut app = f.app();
    let result = app.approve_proposal(&approval).unwrap();
    assert_eq!(result.outcome, ApplyOutcome::Applied);
    drop(app);
    let data = f.copy_records("fresh-mixed-asset");
    let mut app = open(f.base.path(), &data);
    let review = app.proposal(approval.expected.id).unwrap();
    assert_eq!(review.state, ProposalState::Applied);
    assert!(review.comments.is_empty());
    assert_eq!(app.approve_proposal(&approval).unwrap(), result);
    f.assert_applied();
    drop(app);
    assert!(open(f.base.path(), &data).tools().is_ok());
}

#[test]
fn asset_crashes_settle_complete_proofs_and_repair_partial_mixed_effects() {
    for (phase, member, expected) in [
        ("intent", 0, ApplyOutcome::NotApplied),
        ("mirror-intent", 0, ApplyOutcome::NotApplied),
        ("stage", 0, ApplyOutcome::NotApplied),
        ("stage", 3, ApplyOutcome::NotApplied),
        ("prepared-db", 0, ApplyOutcome::NotApplied),
        ("prepared", 0, ApplyOutcome::NotApplied),
        ("member", 0, ApplyOutcome::Uncertain),
        ("member", 1, ApplyOutcome::Uncertain),
        ("member", 2, ApplyOutcome::Uncertain),
        ("member", 3, ApplyOutcome::Applied),
        ("synced", 3, ApplyOutcome::Applied),
        ("verified", 0, ApplyOutcome::Applied),
        ("completion", 0, ApplyOutcome::Applied),
        ("receipt", 0, ApplyOutcome::Applied),
    ] {
        let f = Fixture::new();
        let approval = f.prepare();
        f.crash(phase, member, "apply");
        let mut app = f.app();
        let result = app.reconcile_proposal(approval.operation_id).unwrap();
        assert_eq!(result.outcome, expected, "{phase}/{member}");
        assert_eq!(app.approve_proposal(&approval).unwrap(), result);
        if expected == ApplyOutcome::Uncertain {
            assert_eq!(
                app.proposal_asset("assets/a.bin").unwrap_err().kind,
                ErrorKind::SaveUncertain
            );
            assert!(
                !app.proposal(approval.expected.id)
                    .unwrap()
                    .comments
                    .is_empty()
            );
            let preview = app.preview_proposal_repair(approval.operation_id).unwrap();
            let direction = if member == 1 {
                RepairDirection::Restore
            } else {
                RepairDirection::Finish
            };
            let repair = RepairRequest {
                id: Uuid::new_v4(),
                operation_id: approval.operation_id,
                expected: preview.expected,
                direction,
            };
            let repaired = app.repair_proposal(&repair).unwrap();
            assert_eq!(app.repair_proposal(&repair).unwrap(), repaired);
            if direction == RepairDirection::Restore {
                f.assert_originals();
            } else {
                f.assert_applied();
            }
        } else if expected == ApplyOutcome::Applied {
            f.assert_applied();
        } else {
            f.assert_originals();
        }
        assert!(app.tools().is_ok());
    }
}

#[test]
fn asset_undo_and_repair_interruption_reconcile_without_repeating_namespace_writes() {
    for (phase, member, expected) in [
        ("prepared", 0, ApplyOutcome::NotApplied),
        ("member", 0, ApplyOutcome::Uncertain),
        ("member", 1, ApplyOutcome::Uncertain),
        ("member", 2, ApplyOutcome::Uncertain),
        ("member", 3, ApplyOutcome::Applied),
        ("completion", 0, ApplyOutcome::Applied),
    ] {
        let f = Fixture::new();
        let approval = f.prepare();
        let mut app = f.app();
        app.approve_proposal(&approval).unwrap();
        let undo = UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: approval.operation_id,
            trash_member: None,
        };
        fs::write(
            f.base.path().join("undo.json"),
            serde_json::to_vec(&undo).unwrap(),
        )
        .unwrap();
        drop(app);
        f.crash(phase, member, "undo");
        let mut app = f.app();
        assert_eq!(
            app.reconcile_proposal(undo.operation_id).unwrap().outcome,
            expected,
            "{phase}/{member}"
        );
        if expected == ApplyOutcome::Uncertain {
            let preview = app.preview_proposal_repair(undo.operation_id).unwrap();
            let repair = RepairRequest {
                id: Uuid::new_v4(),
                operation_id: undo.operation_id,
                expected: preview.expected,
                direction: RepairDirection::Finish,
            };
            assert_eq!(
                app.repair_proposal(&repair).unwrap().outcome,
                Some(ApplyOutcome::Applied)
            );
            f.assert_originals();
        }
    }
    for direction in [RepairDirection::Finish, RepairDirection::Restore] {
        for (phase, member) in [
            ("repair-intent", 0),
            ("repair-mirror", 0),
            (
                "repair-member",
                if direction == RepairDirection::Finish {
                    2
                } else {
                    0
                },
            ),
            ("repair-synced", 0),
            ("repair-verified", 0),
        ] {
            let f = Fixture::new();
            let approval = f.prepare();
            f.crash("member", 1, "apply");
            let mut app = f.app();
            assert_eq!(
                app.reconcile_proposal(approval.operation_id)
                    .unwrap()
                    .outcome,
                ApplyOutcome::Uncertain
            );
            let preview = app.preview_proposal_repair(approval.operation_id).unwrap();
            let repair = RepairRequest {
                id: Uuid::new_v4(),
                operation_id: approval.operation_id,
                expected: preview.expected,
                direction,
            };
            fs::write(
                f.base.path().join("repair.json"),
                serde_json::to_vec(&repair).unwrap(),
            )
            .unwrap();
            drop(app);
            f.crash(phase, member, "repair");
            let mut app = f.app();
            let settled = app.reconcile_proposal(approval.operation_id).unwrap();
            if settled.outcome == ApplyOutcome::Uncertain {
                let preview = app.preview_proposal_repair(approval.operation_id).unwrap();
                let finish = RepairRequest {
                    id: Uuid::new_v4(),
                    operation_id: approval.operation_id,
                    expected: preview.expected,
                    direction,
                };
                app.repair_proposal(&finish).unwrap();
            }
            if direction == RepairDirection::Finish {
                f.assert_applied();
            } else {
                f.assert_originals();
            }
        }
    }
}

#[test]
fn stale_asset_identity_and_external_occupants_refuse_without_effects_or_retargeting() {
    for change in 0..3 {
        let f = Fixture::new();
        let approval = f.prepare();
        let path = f.base.path().join("vault/assets/a.bin");
        let mut app = f.app();
        match change {
            0 => fs::write(&path, b"Changed exact bytes").unwrap(),
            1 => {
                fs::rename(&path, f.base.path().join("vault/assets/owner-old.bin")).unwrap();
                fs::write(&path, &f.original).unwrap();
            }
            _ => fs::write(
                f.base.path().join("vault/assets/new.bin"),
                b"External occupant",
            )
            .unwrap(),
        }
        assert!(app.approve_proposal(&approval).is_err());
        assert!(!f.base.path().join("vault/new.md").exists());
        assert_eq!(
            fs::read(f.base.path().join("vault/assets/trash.bin")).unwrap(),
            f.trash
        );
        assert_eq!(
            app.proposal(approval.expected.id).unwrap().state,
            ProposalState::Draft
        );
        assert!(
            !app.proposal(approval.expected.id)
                .unwrap()
                .comments
                .is_empty()
        );
    }
}

#[test]
fn asset_creation_replay_preserves_original_proofs_and_later_review_after_file_loss() {
    let f = Fixture::new();
    let mut app = f.app();
    let input = f.draft(&mut app);
    let original = app.create_proposal(&input).unwrap();
    let edited = app
        .edit_proposal(&ProposalEdit {
            expected: original.stamp(),
            title: "Newer owner review".into(),
            texts: vec![
                None,
                None,
                None,
                Some("Newer complete Markdown λ\r\n".into()),
            ],
            action_data: vec![],
        })
        .unwrap();
    fs::remove_file(f.base.path().join("vault/assets/a.bin")).unwrap();
    fs::remove_file(f.base.path().join("vault/assets/trash.bin")).unwrap();
    assert_eq!(app.create_proposal(&input).unwrap(), edited);
    assert_eq!(edited.draft.changes[..3], original.draft.changes[..3]);
    let mut changed = input;
    let DraftNoteChange::ReplaceAsset { bytes, .. } = &mut changed.changes[0] else {
        panic!("asset")
    };
    bytes.push(0);
    assert_eq!(
        app.create_proposal(&changed).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    assert_eq!(app.proposal(original.draft.id).unwrap(), edited);
    assert!(!f.base.path().join("vault/assets/new.bin").exists());
}

#[test]
fn mixed_asset_and_action_approval_preserves_existing_undo_refusal_without_effects() {
    use brn_store::work::actions::{ActionData, ActionState};
    let f = Fixture::new();
    let mut app = f.app();
    let mut input = f.draft(&mut app);
    let id = Uuid::new_v4();
    let data = ActionData {
        title: "Exact asset-related Action".into(),
        description: "Owner-reviewed meaning λ\r\n".into(),
        state: ActionState::Waiting,
        owner: Some("Õie".into()),
        related_person: None,
        related_project: None,
        sources: vec![],
        thread: None,
        due_on: Some("2028-02-29".into()),
        follow_up_on: None,
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: None,
    };
    input.action_changes = vec![ActionChange::Create {
        id,
        data: data.clone(),
    }];
    let review = app.create_proposal(&input).unwrap();
    assert!(app.store.action(id).unwrap().is_none());
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    assert_eq!(
        app.approve_proposal(&approval).unwrap().outcome,
        ApplyOutcome::Applied
    );
    f.assert_applied();
    let created = app.action(id).unwrap();
    assert_eq!(created.data, data);
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: approval.operation_id,
        trash_member: None,
    };
    assert_eq!(
        app.preview_proposal_undo(&undo).unwrap_err().message,
        "Action-bearing Undo is not yet supported"
    );
    assert_eq!(
        app.undo_proposal(&undo).unwrap_err().message,
        "Action-bearing Undo is not yet supported"
    );
    assert!(app.proposal_apply(undo.operation_id).unwrap().is_none());
    f.assert_applied();
    assert_eq!(app.action(id).unwrap(), created);
    assert_eq!(
        app.approve_proposal(&approval).unwrap().outcome,
        ApplyOutcome::Applied
    );
    drop(app);
    let mut app = f.app();
    assert_eq!(app.action(id).unwrap(), created);
    assert!(app.undo_proposal(&undo).is_err());
    f.assert_applied();
}

#[test]
fn older_sql_restores_terminal_asset_approval_and_cleared_comments_without_reapplying() {
    let f = Fixture::new();
    let approval = f.prepare();
    let database = f.base.path().join("data/brn.sqlite");
    let earlier = fs::read(&database).unwrap();
    let mut app = f.app();
    let receipt = app.approve_proposal(&approval).unwrap();
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    let proof = app.proposal_asset("assets/a.bin").unwrap();
    drop(app);
    fs::write(&database, earlier).unwrap();
    let mut app = f.app();
    assert_eq!(app.approve_proposal(&approval).unwrap(), receipt);
    assert!(
        app.proposal(approval.expected.id)
            .unwrap()
            .comments
            .is_empty()
    );
    assert_eq!(app.proposal_asset("assets/a.bin").unwrap(), proof);
    f.assert_applied();
}
