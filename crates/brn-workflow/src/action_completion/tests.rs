use super::*;
use crate::{
    actions::{ActionData, ActionListRequest, ActionState},
    app::AppConfig,
    proposal_apply::ApprovalRequest,
    proposals::{ActionChange, DraftRequest},
};
use brn_ai::ReadTools;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    base: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let base = owner.path().to_owned();
        fs::create_dir(base.join("data")).unwrap();
        fs::create_dir(base.join("vault")).unwrap();
        fs::write(base.join("vault/a.md"), "\u{feff}Current needle õ\r\n").unwrap();
        Self {
            _owner: owner,
            base,
        }
    }
    fn app(&self) -> App {
        open(&self.base)
    }
    fn prepare(&self) -> CompleteActionRequest {
        let mut app = self.app();
        let id = Uuid::new_v4();
        let review = app
            .create_proposal(&DraftRequest {
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "User review λ".into(),
                changes: vec![],
                sources: vec![],
                action_changes: vec![ActionChange::Create {
                    id,
                    data: ActionData {
                        title: "Approved Waiting action".into(),
                        description: "\u{feff}日本語\r\n\t".into(),
                        state: ActionState::Waiting,
                        owner: Some("Zoë".into()),
                        related_person: None,
                        related_project: None,
                        sources: vec![],
                        thread: None,
                        due_on: None,
                        follow_up_on: None,
                        dependencies: vec![],
                        parent: None,
                        follows_up: None,
                        priority: None,
                    },
                }],
            })
            .unwrap();
        app.approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        })
        .unwrap();
        let request = CompleteActionRequest {
            operation_id: Uuid::new_v4(),
            before: Box::new(app.action(id).unwrap()),
        };
        fs::write(
            self.base.join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        request
    }
    fn receipt_path(&self, id: Uuid) -> PathBuf {
        self.base
            .join("data")
            .join(format!(".brn-complete-{id}.receipt"))
    }
}
fn open(base: &Path) -> App {
    App::open(
        &base.join("data"),
        AppConfig {
            vault_root: Some(base.join("vault")),
            credentials_dir: Some(base.join("credentials")),
            model_dir: None,
        },
    )
    .unwrap()
}

#[test]
fn publication_failures_and_sql_rollback_fence_current_until_exact_retry() {
    for (phase, uncertain) in [
        ("write", false),
        ("file_sync", false),
        ("prepare_directory_sync", false),
        ("rename", true),
        ("directory_sync", true),
        ("postproof", true),
        ("published", true),
    ] {
        let f = Fixture::new();
        let request = f.prepare();
        let mut app = f.app();
        let tools = app.tools().unwrap();
        if phase == "published" {
            FAILURE.with(|fault| fault.set(Some(phase)));
        } else {
            crate::files::recovery::FAILURE.with(|fault| fault.set(Some(phase)));
        }
        let failed = app.complete_action(&request).unwrap_err();
        FAILURE.with(|fault| fault.set(None));
        crate::files::recovery::FAILURE.with(|fault| fault.set(None));
        assert_eq!(
            failed.kind,
            if uncertain {
                ErrorKind::SaveUncertain
            } else {
                ErrorKind::Other
            },
            "{phase}"
        );
        assert_eq!(
            app.store.action(request.before.origin.id).unwrap(),
            Some(*request.before.clone())
        );
        assert_eq!(app.store.action_completion_for(&request).unwrap(), None);
        if uncertain {
            assert!(app.current_evidence_blocked().unwrap(), "{phase}");
            assert_eq!(
                app.action(request.before.origin.id).unwrap_err().kind,
                ErrorKind::SaveUncertain
            );
            assert_eq!(
                app.actions(&ActionListRequest::default()).unwrap_err().kind,
                ErrorKind::SaveUncertain
            );
            assert_eq!(app.note("a.md").unwrap_err().kind, ErrorKind::SaveUncertain);
            assert!(
                tools.search_notes("needle", 10).is_err(),
                "held tools need fence: {phase}"
            );
        } else {
            assert!(!app.current_evidence_blocked().unwrap());
            assert_eq!(
                app.action(request.before.origin.id).unwrap(),
                *request.before
            );
        }
        let completion = app.complete_action(&request).unwrap();
        assert_eq!(
            app.action(request.before.origin.id).unwrap(),
            completion.after
        );
        assert_eq!(app.complete_action(&request).unwrap(), completion);
        assert!(!app.current_evidence_blocked().unwrap());
        assert!(!tools.search_notes("needle", 10).unwrap().hits.is_empty());
        drop(tools);
        drop(app);
        assert_eq!(f.app().complete_action(&request).unwrap(), completion);
        assert_eq!(
            fs::read(f.base.join("vault/a.md")).unwrap(),
            "\u{feff}Current needle õ\r\n".as_bytes()
        );
    }
}

#[test]
fn exact_recovery_payload_conflict_and_retry_sync_failure_preserve_fence() {
    let f = Fixture::new();
    let request = f.prepare();
    let mut app = f.app();
    FAILURE.with(|fault| fault.set(Some("published")));
    assert_eq!(
        app.complete_action(&request).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    FAILURE.with(|fault| fault.set(None));
    let original = fs::read(f.receipt_path(request.operation_id)).unwrap();
    let mut changed = request.clone();
    changed.before.version += 1;
    changed.before.data.description.push('!');
    assert_eq!(
        app.complete_action(&changed).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    crate::files::recovery::FAILURE.with(|fault| fault.set(Some("file_sync")));
    assert!(app.complete_action(&request).is_err());
    crate::files::recovery::FAILURE.with(|fault| fault.set(None));
    assert!(app.current_evidence_blocked().unwrap());
    assert_eq!(
        app.store.action(request.before.origin.id).unwrap(),
        Some(*request.before.clone())
    );
    assert_eq!(
        fs::read(f.receipt_path(request.operation_id)).unwrap(),
        original
    );
    let completion = app.complete_action(&request).unwrap();
    assert_eq!(
        app.action(request.before.origin.id).unwrap(),
        completion.after
    );
    assert_eq!(
        fs::read(f.receipt_path(request.operation_id)).unwrap(),
        original
    );
}

#[test]
fn fresh_completion_respects_other_durable_fences_and_terminal_replay_preserves_them() {
    let f = Fixture::new();
    let request = f.prepare();
    let mut app = f.app();
    let editor = app.open_editor("a.md").unwrap();
    let proposal = app
        .create_proposal(&DraftRequest {
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Pending exact knowledge".into(),
            changes: vec![crate::proposals::DraftNoteChange::Replace {
                path: "a.md".into(),
                expected: editor.record.baseline.clone(),
                text: "Changed".into(),
            }],
            sources: vec![],
            action_changes: vec![],
        })
        .unwrap();
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: proposal.stamp(),
    };
    app.store.begin_proposal_apply(&approval).unwrap();
    assert_eq!(
        app.complete_action(&request).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert!(!f.receipt_path(request.operation_id).exists());
    app.store
        .finish_proposal_apply(
            approval.operation_id,
            crate::proposal_apply::ApplyOutcome::NotApplied,
            Some(&[crate::proposal_apply::ApplyMemberProof {
                destination: Some(editor.record.baseline.clone()),
                staging: None,
            }]),
        )
        .unwrap();
    let completion = app.complete_action(&request).unwrap();
    // An independent pending approval does not revoke a checked terminal receipt.
    let second = app
        .create_proposal(&DraftRequest {
            id: Uuid::new_v4(),
            ..proposal_input(&editor)
        })
        .unwrap();
    app.store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: second.stamp(),
        })
        .unwrap();
    assert_eq!(app.complete_action(&request).unwrap(), completion);
    assert!(app.current_evidence_blocked().unwrap());
    assert_eq!(
        app.action(request.before.origin.id).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
}
fn proposal_input(editor: &crate::editor::EditorView) -> DraftRequest {
    DraftRequest {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Independent pending knowledge".into(),
        changes: vec![crate::proposals::DraftNoteChange::Replace {
            path: "a.md".into(),
            expected: editor.record.baseline.clone(),
            text: "Other".into(),
        }],
        sources: vec![],
        action_changes: vec![],
    }
}

#[test]
fn malformed_completion_evidence_refuses_startup_before_auth_or_model_loading() {
    let f = Fixture::new();
    let request = f.prepare();
    let mut app = f.app();
    let completion = app.complete_action(&request).unwrap();
    let proposals = app.store.proposals(None).unwrap();
    drop(app);
    fs::write(
        f.receipt_path(request.operation_id),
        b"foreign malformed completion evidence",
    )
    .unwrap();
    fs::remove_dir(f.base.join("credentials")).unwrap();
    let loaded = std::cell::Cell::new(false);
    let failure = App::open_with_model_loader(
        &f.base.join("data"),
        AppConfig {
            vault_root: None,
            credentials_dir: Some(f.base.join("credentials")),
            model_dir: None,
        },
        |_, _| {
            loaded.set(true);
            Ok(None)
        },
    )
    .err()
    .expect("malformed authority refuses");
    assert_eq!(failure.kind, ErrorKind::ToolRejected);
    assert!(!loaded.get());
    assert!(!f.base.join("credentials").exists());
    // SQLite may rewrite already-approved recovery rows during startup. Compare
    // checked operational records, rather than its physical page representation.
    let (store, report) = WorkStore::open(&f.base.join("data")).unwrap();
    assert!(report.corrupt_moved_to.is_none());
    assert!(report.restored_from.is_none());
    assert_eq!(
        store.action(request.before.origin.id).unwrap(),
        Some(completion.after.clone())
    );
    assert_eq!(
        store.action_completion_for(&request).unwrap(),
        Some(completion)
    );
    assert_eq!(store.proposals(None).unwrap(), proposals);
    assert_eq!(
        fs::read(f.receipt_path(request.operation_id)).unwrap(),
        b"foreign malformed completion evidence"
    );
}

#[test]
fn interrupted_completion_restores_from_ordinary_evidence_with_current_older_or_fresh_sqlite() {
    for phase in ["published", "settled"] {
        for database in ["current", "older", "backup", "fresh"] {
            let f = Fixture::new();
            let request = f.prepare();
            fs::copy(f.base.join("data/brn.sqlite"), f.base.join("before.sqlite")).unwrap();
            let _process_fixtures = crate::SUBPROCESS_FIXTURES.lock().unwrap();
            let result = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "action_completion::tests::crash_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("BRN_COMPLETE_TEST_BASE", &f.base)
                .env("BRN_COMPLETE_TEST_PHASE", phase)
                .output()
                .unwrap();
            assert_eq!(
                result.status.code(),
                Some(73),
                "{phase}/{database}: {} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            drop(_process_fixtures);
            let data = f.base.join("data");
            if database != "current" {
                for file in ["brn.sqlite", "brn.sqlite-wal", "brn.sqlite-shm"] {
                    let path = data.join(file);
                    if path.exists() {
                        fs::remove_file(path).unwrap();
                    }
                }
                if database == "older" {
                    fs::copy(f.base.join("before.sqlite"), data.join("brn.sqlite")).unwrap();
                }
                if database == "fresh" {
                    fs::remove_dir_all(data.join("backups")).unwrap();
                }
                if database == "backup" {
                    fs::copy(
                        f.base.join("before.sqlite"),
                        data.join("backups/brn-9999999999999.sqlite"),
                    )
                    .unwrap();
                }
            }
            let mut app = f.app();
            assert_eq!(
                app.open_report().restored_from.is_some(),
                database == "backup"
            );
            let receipt = app.store.action_completion_for(&request).unwrap().unwrap();
            receipt.validate().unwrap();
            assert_eq!(app.action(request.before.origin.id).unwrap(), receipt.after);
            assert_eq!(app.complete_action(&request).unwrap(), receipt);
            assert!(!app.current_evidence_blocked().unwrap());
            assert!(app.store.conversations().unwrap().is_empty());
            drop(app);
            assert_eq!(fs::read_dir(f.base.join("credentials")).unwrap().count(), 0);
        }
    }
}
#[test]
#[ignore = "private subprocess entry exercised by completion crash recovery matrix"]
fn crash_child() {
    let base = PathBuf::from(std::env::var_os("BRN_COMPLETE_TEST_BASE").unwrap());
    let phase = std::env::var("BRN_COMPLETE_TEST_PHASE").unwrap();
    let request = serde_json::from_slice(&fs::read(base.join("request.json")).unwrap()).unwrap();
    let mut app = open(&base);
    CRASH_PHASE.with(|selected| *selected.borrow_mut() = Some(phase));
    app.complete_action(&request).unwrap();
    panic!("selected completion checkpoint not reached");
}
