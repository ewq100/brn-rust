//! Private synthetic qualification of mixed file/Action explicit repair.
use super::*;
use crate::{
    app::AppConfig,
    proposals::{DraftNoteChange, DraftRequest},
};
use brn_store::work::{
    actions::{ActionData, ActionOrigin, ActionRecord, ActionState},
    proposals::{ActionChange, CommentRequest, CommentTarget, ReviewComment},
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};

struct Fixture {
    _base: tempfile::TempDir,
    vault: PathBuf,
    data: PathBuf,
    credentials: PathBuf,
    original: String,
    trashed: String,
    after: String,
    created: String,
    before_action: ActionRecord,
    created_action: Uuid,
    approval: ApprovalRequest,
}

fn data(title: &str) -> ActionData {
    ActionData {
        title: title.into(),
        description: "Exact operational work 日本語\r\n".into(),
        state: ActionState::Waiting,
        owner: Some("Õie".into()),
        related_person: None,
        related_project: None,
        sources: Vec::new(),
        thread: None,
        due_on: Some("2028-02-29".into()),
        follow_up_on: None,
        dependencies: Vec::new(),
        parent: None,
        follows_up: None,
        priority: None,
    }
}

fn input(action_changes: Vec<ActionChange>) -> DraftRequest {
    DraftRequest {
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Exact mixed repair review".into(),
        changes: Vec::new(),
        sources: Vec::new(),
        action_changes,
    }
}

fn managed(id: Uuid, body: &str) -> String {
    format!("\u{feff}---\r\nbrn_id: {id}\r\n---\r\n{body}\r\n")
}

impl Fixture {
    fn partial() -> (Self, App) {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let vault = base.path().join("vault");
        let data_dir = base.path().join("data");
        let credentials = base.path().join("credentials");
        fs::create_dir(&vault).unwrap();
        fs::create_dir(&data_dir).unwrap();
        let mut app = App::open(
            &data_dir,
            AppConfig {
                vault_root: Some(vault.clone()),
                credentials_dir: Some(credentials.clone()),
                model_dir: None,
            },
        )
        .unwrap();
        let existing = Uuid::new_v4();
        let review = app
            .create_proposal(&input(vec![ActionChange::Create {
                id: existing,
                data: data("Existing exact Action λ\r\n"),
            }]))
            .unwrap();
        app.approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        })
        .unwrap();
        let before_action = app.action(existing).unwrap();
        let note_id = Uuid::new_v4();
        let original = managed(note_id, "Original before õ");
        let trashed = managed(Uuid::new_v4(), "Original source 🦀");
        let after = managed(note_id, "Reviewed after 日本語");
        let created = managed(Uuid::new_v4(), "New approved note");
        fs::write(vault.join("a.md"), &original).unwrap();
        fs::write(vault.join("trash.md"), &trashed).unwrap();
        let source = app.proposal_source("a.md").unwrap().source;
        let trash = app.proposal_source("trash.md").unwrap().source;
        let created_action = Uuid::new_v4();
        let mut replacement = before_action.data.clone();
        replacement
            .description
            .push_str("Reviewed replacement õ\r\n");
        let mut request = input(vec![
            ActionChange::Create {
                id: created_action,
                data: data("New exact approved Action 🦀\r\n"),
            },
            ActionChange::Replace {
                before: Box::new(before_action.clone()),
                data: replacement,
            },
        ]);
        request.changes = vec![
            DraftNoteChange::Replace {
                path: "a.md".into(),
                expected: source.fingerprint.clone(),
                text: after.clone(),
            },
            DraftNoteChange::Create {
                path: "new.md".into(),
                text: created.clone(),
            },
            DraftNoteChange::Trash {
                path: "trash.md".into(),
                expected: trash.fingerprint,
            },
        ];
        request.sources = vec![source];
        let review = app.create_proposal(&request).unwrap();
        let review = app
            .add_proposal_comment(&CommentRequest {
                expected: review.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Retain this exact review until whole Finish λ\r\n".into(),
                    target: CommentTarget::Proposal,
                },
            })
            .unwrap();
        let approval = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        };
        fs::write(
            base.path().join("approval.json"),
            serde_json::to_vec(&approval).unwrap(),
        )
        .unwrap();
        drop(app);
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "proposal_apply::action_repair_tests::mixed_repair_crash_child",
                "--ignored",
                "--test-threads=1",
            ])
            .env("BRN_MIXED_REPAIR_BASE", base.path())
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(86),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        let fixture = Self {
            _base: base,
            vault,
            data: data_dir,
            credentials,
            original,
            trashed,
            after,
            created,
            before_action,
            created_action,
            approval,
        };
        let mut app = fixture.app();
        let receipt = app
            .reconcile_proposal(fixture.approval.operation_id)
            .unwrap();
        assert_eq!(receipt.outcome, ApplyOutcome::Uncertain);
        assert!(app.store.action(created_action).unwrap().is_none());
        assert_eq!(
            app.store.action(existing).unwrap(),
            Some(fixture.before_action.clone())
        );
        assert_eq!(
            fs::read(fixture.vault.join("a.md")).unwrap(),
            fixture.after.as_bytes()
        );
        assert!(!fixture.vault.join("new.md").exists());
        assert_eq!(
            fs::read(fixture.vault.join("trash.md")).unwrap(),
            fixture.trashed.as_bytes()
        );
        assert!(app.current_evidence_blocked().unwrap());
        assert!(app.action(existing).is_err());
        (fixture, app)
    }

    fn app(&self) -> App {
        App::open(
            &self.data,
            AppConfig {
                vault_root: None,
                credentials_dir: Some(self.credentials.clone()),
                model_dir: None,
            },
        )
        .unwrap()
    }

    fn repair(&self, app: &mut App, direction: RepairDirection) -> RepairRequest {
        let preview = app
            .preview_proposal_repair(self.approval.operation_id)
            .unwrap();
        RepairRequest {
            id: Uuid::new_v4(),
            operation_id: self.approval.operation_id,
            expected: preview.expected,
            direction,
        }
    }

    fn assert_originals(&self) {
        assert_eq!(
            fs::read(self.vault.join("a.md")).unwrap(),
            self.original.as_bytes()
        );
        assert_eq!(
            fs::read(self.vault.join("trash.md")).unwrap(),
            self.trashed.as_bytes()
        );
        assert!(!self.vault.join("new.md").exists());
    }
}

// A valid checked synthetic competing write, never a production mutation API.
fn put_action(dir: &Path, record: &ActionRecord) {
    record.validate().unwrap();
    let bytes = serde_json::to_vec(record).unwrap();
    let origin = serde_json::to_vec(&record.origin).unwrap();
    let state = serde_json::to_value(record.data.state).unwrap();
    rusqlite::Connection::open(dir.join("brn.sqlite"))
        .unwrap()
        .execute(
            "INSERT OR REPLACE INTO actions(id,version,state,created_at_ms,creation_sha256,record_json,record_sha256) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![record.origin.id.to_string(),record.version as i64,state.as_str().unwrap(),record.origin.created_at_ms as i64,Sha256::digest(&origin).as_slice(),bytes,Sha256::digest(&bytes).as_slice()],
        )
        .unwrap();
}

fn completed(mut record: ActionRecord) -> ActionRecord {
    record.version += 1;
    record.updated_at_ms += 1;
    record.data.state = ActionState::Completed;
    record
        .data
        .description
        .push_str("Actual later completion\r\n");
    record.waiting_since_ms = None;
    record.completed_at_ms = Some(record.updated_at_ms);
    record.validate().unwrap();
    record
}

#[test]
#[ignore = "subprocess entry point for genuine mixed repair interruption"]
fn mixed_repair_crash_child() {
    let base = PathBuf::from(std::env::var("BRN_MIXED_REPAIR_BASE").unwrap());
    let approval = serde_json::from_slice(&fs::read(base.join("approval.json")).unwrap()).unwrap();
    let mut app = App::open(
        &base.join("data"),
        AppConfig {
            vault_root: None,
            credentials_dir: Some(base.join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    APPLY_CHECKPOINT.with(|point| *point.borrow_mut() = Some(("synced".into(), 0)));
    let _ = app.approve_proposal(&approval);
    panic!("Requested partial installation checkpoint was not reached");
}

#[test]
fn mixed_explicit_finish_commits_exact_actions_and_whole_receipt_then_replays_without_effects() {
    let (f, mut app) = Fixture::partial();
    let preview = app
        .preview_proposal_repair(f.approval.operation_id)
        .unwrap();
    assert_eq!(
        preview.phases,
        vec![
            ApplyMemberPhase::Applied,
            ApplyMemberPhase::Before,
            ApplyMemberPhase::Before
        ]
    );
    let journal = app
        .proposal_apply(f.approval.operation_id)
        .unwrap()
        .unwrap();
    let review = app.proposal(f.approval.expected.id).unwrap();
    assert!(!review.comments.is_empty());
    let finish = f.repair(&mut app, RepairDirection::Finish);
    assert_eq!(app.proposal(f.approval.expected.id).unwrap(), review);
    assert!(app.store.proposal_repair(finish.id).unwrap().is_none());
    let receipt = app.repair_proposal(&finish).unwrap();
    assert_eq!(receipt.outcome, Some(ApplyOutcome::Applied));
    let settled = app
        .proposal_apply(f.approval.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        settled.receipt.as_ref().unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(settled.action_records, journal.action_records);
    assert_eq!(
        app.store.proposal_repair(finish.id).unwrap(),
        Some(receipt.clone())
    );
    for expected in &journal.action_records {
        assert_eq!(app.action(expected.origin.id).unwrap(), *expected);
    }
    assert_eq!(
        journal.action_records[0].origin.proposal,
        f.approval.expected
    );
    assert_eq!(journal.action_records[1].origin, f.before_action.origin);
    assert_eq!(
        journal.action_records[1].version,
        f.before_action.version + 1
    );
    assert_eq!(
        journal.action_records[1].waiting_since_ms,
        f.before_action.waiting_since_ms
    );
    assert_eq!(fs::read(f.vault.join("a.md")).unwrap(), f.after.as_bytes());
    assert_eq!(
        fs::read(f.vault.join("new.md")).unwrap(),
        f.created.as_bytes()
    );
    assert!(!f.vault.join("trash.md").exists());
    assert!(
        app.proposal(f.approval.expected.id)
            .unwrap()
            .comments
            .is_empty()
    );
    assert!(!app.current_evidence_blocked().unwrap());
    let mirrored = app
        .application_records()
        .unwrap()
        .read(f.approval.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(mirrored.journal, settled);
    let newer: Vec<_> = journal.action_records.into_iter().map(completed).collect();
    for record in &newer {
        put_action(&f.data, record);
    }
    let later = managed(
        brn_store::note_identity::read(&f.after).unwrap().unwrap(),
        "Later owner bytes",
    );
    fs::write(f.vault.join("a.md"), &later).unwrap();
    assert_eq!(app.repair_proposal(&finish).unwrap(), receipt);
    assert_eq!(
        app.approve_proposal(&f.approval).unwrap(),
        settled.receipt.clone().unwrap()
    );
    drop(app);
    let mut reopened = f.app();
    assert_eq!(reopened.repair_proposal(&finish).unwrap(), receipt);
    assert_eq!(
        reopened.approve_proposal(&f.approval).unwrap(),
        settled.receipt.unwrap()
    );
    for record in newer {
        assert_eq!(reopened.action(record.origin.id).unwrap(), record);
    }
    assert_eq!(fs::read(f.vault.join("a.md")).unwrap(), later.as_bytes());
    assert_eq!(
        fs::read(f.vault.join("new.md")).unwrap(),
        f.created.as_bytes()
    );
    assert!(!f.vault.join("trash.md").exists());
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
}

#[test]
fn admitted_mixed_finish_refuses_competing_actions_at_mirror_and_final_verification() {
    // These are distinct absent-Create and full-Replace CAS boundaries, each
    // before repair effects and after all file effects have been verified.
    for stop in ["repair-mirror", "repair-verified"] {
        for compete_with_create in [false, true] {
            let (f, mut app) = Fixture::partial();
            let journal = app
                .proposal_apply(f.approval.operation_id)
                .unwrap()
                .unwrap();
            let competing = if compete_with_create {
                let initial = data("Separate approved Action at same UUID");
                completed(ActionRecord {
                    origin: ActionOrigin {
                        id: f.created_action,
                        proposal: crate::proposals::ProposalStamp {
                            id: Uuid::new_v4(),
                            version: 1,
                        },
                        data: initial.clone(),
                        created_at_ms: journal.started_at_ms,
                    },
                    version: 1,
                    data: initial,
                    updated_at_ms: journal.started_at_ms,
                    waiting_since_ms: Some(journal.started_at_ms),
                    completed_at_ms: None,
                })
            } else {
                completed(f.before_action.clone())
            };
            let finish = f.repair(&mut app, RepairDirection::Finish);
            let dir = f.data.clone();
            let injected = competing.clone();
            APPLY_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |step, member| {
                    if step == stop && member == 0 {
                        put_action(&dir, &injected);
                    }
                }));
            });
            let result = app.repair_proposal(&finish);
            APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
            assert!(result.is_err(), "{stop}, Create={compete_with_create}");
            assert_eq!(
                app.store.action(competing.origin.id).unwrap(),
                Some(competing.clone())
            );
            assert_eq!(
                app.store
                    .proposal_repair(finish.id)
                    .unwrap()
                    .unwrap()
                    .outcome,
                Some(ApplyOutcome::Uncertain)
            );
            let unresolved = app
                .proposal_apply(f.approval.operation_id)
                .unwrap()
                .unwrap();
            assert_eq!(
                unresolved.receipt.as_ref().unwrap().outcome,
                ApplyOutcome::Uncertain
            );
            let mirror = app
                .application_records()
                .unwrap()
                .read(f.approval.operation_id)
                .unwrap()
                .unwrap();
            assert_eq!(
                mirror.journal.receipt.as_ref().unwrap().outcome,
                ApplyOutcome::Uncertain,
                "Refused Finish must never leave terminal Applied authority"
            );
            assert_eq!(
                mirror
                    .journal
                    .repair
                    .as_ref()
                    .unwrap()
                    .attempts
                    .last()
                    .unwrap()
                    .outcome,
                Some(ApplyOutcome::Uncertain)
            );
            assert!(app.current_evidence_blocked().unwrap());
            assert!(app.action(competing.origin.id).is_err());
            assert!(
                !app.proposal(f.approval.expected.id)
                    .unwrap()
                    .comments
                    .is_empty()
            );
            let restore = f.repair(&mut app, RepairDirection::Restore);
            assert_eq!(
                app.repair_proposal(&restore).unwrap().outcome,
                Some(ApplyOutcome::NotApplied)
            );
            f.assert_originals();
            assert_eq!(app.action(competing.origin.id).unwrap(), competing);
            if compete_with_create {
                assert_eq!(
                    app.action(f.before_action.origin.id).unwrap(),
                    f.before_action
                );
            } else {
                assert!(app.store.action(f.created_action).unwrap().is_none());
            }
            assert!(!app.current_evidence_blocked().unwrap());
            drop(app);
            let reopened = f.app();
            assert_eq!(reopened.action(competing.origin.id).unwrap(), competing);
            f.assert_originals();
            assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
        }
    }
}
