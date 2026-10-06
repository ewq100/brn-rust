//! Synthetic operational recovery qualification, before workflow producers.
use super::*;
use crate::app::AppConfig;
use brn_store::files::VaultIdentity;
use brn_store::work::{
    actions::{ActionData, ActionState},
    proposals::{
        ActionChange, CommentRequest, CommentTarget, ProposalDraft, ProposalState, ReviewComment,
    },
};
use std::{fs, path::PathBuf};

struct Fixture {
    _base: tempfile::TempDir,
    data: PathBuf,
    credentials: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        fs::create_dir(&data).unwrap();
        let credentials = base.path().join("credentials");
        Self {
            _base: base,
            data,
            credentials,
        }
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

    fn seed(&self, prepared: bool, uncertain: bool) -> ApplyJournal {
        let (mut store, _) = WorkStore::open(&self.data).unwrap();
        let record = store
            .create_proposal(&draft(ActionChange::Create {
                id: Uuid::new_v4(),
                data: data("\u{feff}Tähtaeg 🦀\r\n", ActionState::Open),
            }))
            .unwrap();
        let record = store
            .add_proposal_comment(&CommentRequest {
                expected: record.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Retained review evidence 日本語\r\n".into(),
                    target: CommentTarget::Proposal,
                },
            })
            .unwrap();
        let mut journal = store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: record.stamp(),
            })
            .unwrap();
        if prepared {
            journal = store
                .record_proposal_prepared(journal.request.operation_id, &[])
                .unwrap();
        }
        if uncertain {
            store
                .finish_proposal_apply(journal.request.operation_id, ApplyOutcome::Uncertain, None)
                .unwrap();
            journal = store
                .proposal_apply(journal.request.operation_id)
                .unwrap()
                .unwrap();
        }
        journal
    }

    fn assert_vaultless(&self, app: &App) {
        assert!(app.vault_root().is_none());
        assert!(app.editor.files.is_none());
        assert!(app.work_store().setting("vault.root").unwrap().is_none());
        assert!(
            app.work_store()
                .setting("vault.editor_identity")
                .unwrap()
                .is_none()
        );
        assert_eq!(fs::read_dir(&self.credentials).unwrap().count(), 0);
    }
}

fn data(title: &str, state: ActionState) -> ActionData {
    ActionData {
        title: title.into(),
        description: "Exact source-free operational work.\r\n".into(),
        state,
        owner: None,
        related_person: None,
        related_project: None,
        sources: Vec::new(),
        thread: None,
        due_on: None,
        follow_up_on: None,
        dependencies: Vec::new(),
        parent: None,
        follows_up: None,
        priority: None,
    }
}

fn draft(change: ActionChange) -> ProposalDraft {
    ProposalDraft {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: None,
        title: "Exact Action review".into(),
        changes: Vec::new(),
        sources: Vec::new(),
        action_changes: vec![change],
    }
}

fn admitted(app: &mut App, change: ActionChange) -> ApplyJournal {
    let review = app.store.create_proposal(&draft(change)).unwrap();
    app.store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        })
        .unwrap()
}

fn put_action(dir: &Path, record: &brn_store::work::actions::ActionRecord) {
    use sha2::{Digest, Sha256};
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

#[test]
fn admitted_vaultless_action_execution_uses_deliberate_whole_completion() {
    let f = Fixture::new();
    let mut app = f.app();
    let journal = admitted(
        &mut app,
        ActionChange::Create {
            id: Uuid::new_v4(),
            data: data("Exact live λ\r\n", ActionState::Waiting),
        },
    );
    let receipt = app.run_admitted_proposal(journal.clone()).unwrap();
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    assert_eq!(
        app.action(journal.action_records[0].origin.id).unwrap(),
        journal.action_records[0]
    );
    assert_eq!(app.approve_proposal(&journal.request).unwrap(), receipt);
    f.assert_vaultless(&app);
}

#[test]
fn action_cas_refusal_cannot_publish_a_terminal_applied_mirror() {
    use brn_store::work::actions::{ActionOrigin, ActionRecord};
    let f = Fixture::new();
    let mut app = f.app();
    let journal = admitted(
        &mut app,
        ActionChange::Create {
            id: Uuid::new_v4(),
            data: data("Reviewed creation", ActionState::Open),
        },
    );
    let journal = app
        .store
        .record_proposal_prepared(journal.request.operation_id, &[])
        .unwrap();
    app.application_records()
        .unwrap()
        .write(&journal, None)
        .unwrap();
    let competing = data("Competing approved creation", ActionState::Open);
    let competing = ActionRecord {
        origin: ActionOrigin {
            id: journal.action_records[0].origin.id,
            proposal: crate::proposals::ProposalStamp {
                id: Uuid::new_v4(),
                version: 1,
            },
            data: competing.clone(),
            created_at_ms: journal.started_at_ms,
        },
        version: 1,
        data: competing,
        updated_at_ms: journal.started_at_ms,
        waiting_since_ms: None,
        completed_at_ms: None,
    };
    put_action(&f.data, &competing);
    assert!(
        app.complete_proposal(&journal, ApplyOutcome::Applied, Some(Vec::new()), false)
            .is_err()
    );
    let mirrored = app
        .application_records()
        .unwrap()
        .read(journal.request.operation_id)
        .unwrap()
        .unwrap();
    assert!(
        mirrored
            .journal
            .receipt
            .as_ref()
            .is_none_or(|r| r.outcome != ApplyOutcome::Applied),
        "Refused Action CAS must not leave Applied recovery authority"
    );
    assert_eq!(
        app.store.action(competing.origin.id).unwrap(),
        Some(competing)
    );
}

#[test]
fn workflow_action_creation_replay_and_exact_replace_are_vaultless() {
    use crate::proposals::{DraftRequest, ProposalEdit};
    let f = Fixture::new();
    let mut app = f.app();
    let input = DraftRequest {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Review exact Action".into(),
        changes: Vec::new(),
        sources: Vec::new(),
        action_changes: vec![ActionChange::Create {
            id: Uuid::new_v4(),
            data: data("\u{feff}Task 🦀\r\n", ActionState::Open),
        }],
    };
    let review = app.create_proposal(&input).unwrap();
    assert!(
        app.store
            .action(input.action_changes[0].id())
            .unwrap()
            .is_none()
    );
    let mut edited = input.action_changes[0].data().clone();
    edited.description.push_str("Reviewed 日本語\r\n");
    let review = app
        .edit_proposal(&ProposalEdit {
            expected: review.stamp(),
            title: review.draft.title.clone(),
            texts: Vec::new(),
            action_data: vec![edited],
        })
        .unwrap();
    assert_eq!(app.create_proposal(&input).unwrap(), review);
    let mut other = input.clone();
    other.action_changes[0].data_mut().title.push('!');
    assert_eq!(
        app.create_proposal(&other).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    let receipt = app
        .approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        })
        .unwrap();
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    let before = app.action(input.action_changes[0].id()).unwrap();
    let mut after = before.data.clone();
    after.state = ActionState::Waiting;
    let replacement = DraftRequest {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        action_changes: vec![ActionChange::Replace {
            before: Box::new(before.clone()),
            data: after.clone(),
        }],
        ..input
    };
    let review = app.create_proposal(&replacement).unwrap();
    app.approve_proposal(&ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    })
    .unwrap();
    let changed = app.action(before.origin.id).unwrap();
    assert_eq!(changed.origin, before.origin);
    assert_eq!(changed.data, after);
    assert_eq!(changed.version, before.version + 1);
    assert!(changed.waiting_since_ms.is_some());
    f.assert_vaultless(&app);
    drop(app);
    let mut reopened = f.app();
    assert_eq!(reopened.action(changed.origin.id).unwrap(), changed);
    assert_eq!(
        reopened
            .approve_proposal(&ApprovalRequest {
                operation_id: receipt.operation_id,
                expected: crate::proposals::ProposalStamp {
                    id: receipt.proposal_id,
                    version: receipt.approved_version
                }
            })
            .unwrap(),
        receipt
    );
}

#[test]
fn unsettled_vaultless_action_intents_never_turn_empty_file_proofs_into_applied() {
    for (prepared, uncertain, mirrored) in [
        (false, false, false),
        (false, false, true),
        (true, false, true),
        (true, true, true),
    ] {
        let f = Fixture::new();
        let intent = f.seed(prepared, uncertain);
        if mirrored {
            ApplyRecoveryFiles::open(&f.data)
                .unwrap()
                .write(&intent, None)
                .unwrap();
        }
        let mut app = f.app();
        let receipt = app.reconcile_proposal(intent.request.operation_id).unwrap();
        assert_eq!(receipt.outcome, ApplyOutcome::NotApplied);
        assert_eq!(app.approve_proposal(&intent.request).unwrap(), receipt);
        assert!(
            app.work_store()
                .action(intent.action_records[0].origin.id)
                .unwrap()
                .is_none()
        );
        let review = app.proposal(intent.approved.draft.id).unwrap();
        assert_eq!(review.state, ProposalState::Draft);
        assert_eq!(review.comments, intent.approved.comments);
        let settled = app
            .proposal_apply(intent.request.operation_id)
            .unwrap()
            .unwrap();
        assert!(!settled.no_effects, "restart is not fresh admission");
        assert_eq!(settled.observations, Some(Vec::new()));
        assert!(!app.current_evidence_blocked().unwrap());
        f.assert_vaultless(&app);
        drop(app);
        let mut reopened = f.app();
        assert_eq!(
            reopened
                .reconcile_proposal(intent.request.operation_id)
                .unwrap(),
            receipt
        );
        assert_eq!(
            reopened
                .proposal_apply(intent.request.operation_id)
                .unwrap(),
            Some(settled)
        );
        f.assert_vaultless(&reopened);
    }
}

#[test]
fn durable_applied_vaultless_mirror_restores_whole_action_and_cleans_comments() {
    let f = Fixture::new();
    let intent = f.seed(true, false);
    let records = ApplyRecoveryFiles::open(&f.data).unwrap();
    let old = records.write(&intent, None).unwrap();
    let applied = completion(&intent, ApplyOutcome::Applied, Some(Vec::new()), false).unwrap();
    records.write(&applied, Some(&old)).unwrap();
    {
        let (store, _) = WorkStore::open(&f.data).unwrap();
        assert!(
            store
                .action(intent.action_records[0].origin.id)
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .proposal_apply(intent.request.operation_id)
                .unwrap()
                .unwrap()
                .receipt
                .is_none()
        );
    }
    let mut app = f.app();
    let expected = applied.action_records[0].clone();
    assert_eq!(
        app.work_store().action(expected.origin.id).unwrap(),
        Some(expected.clone())
    );
    let receipt = app.reconcile_proposal(intent.request.operation_id).unwrap();
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    let review = app.proposal(intent.approved.draft.id).unwrap();
    assert_eq!(review.state, ProposalState::Applied);
    assert!(review.comments.is_empty());
    assert!(
        records
            .read(intent.request.operation_id)
            .unwrap()
            .unwrap()
            .journal
            .approved
            .comments
            .is_empty()
    );
    assert!(!app.current_evidence_blocked().unwrap());
    f.assert_vaultless(&app);
    drop(app);
    let mut app = f.app();
    assert_eq!(app.approve_proposal(&intent.request).unwrap(), receipt);
    assert_eq!(
        app.work_store().action(expected.origin.id).unwrap(),
        Some(expected)
    );
    f.assert_vaultless(&app);
}

#[test]
fn fresh_database_restores_action_chain_without_binding_or_replaying_files() {
    let f = Fixture::new();
    let intent = f.seed(true, false);
    let backups: Vec<_> = fs::read_dir(f.data.join("backups"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1);
    let old_database = fs::read(&backups[0]).unwrap();
    let first = completion(&intent, ApplyOutcome::Applied, Some(Vec::new()), false).unwrap();
    let records = ApplyRecoveryFiles::open(&f.data).unwrap();
    records.write(&first, None).unwrap();
    let second;
    {
        let (mut store, _) = WorkStore::open(&f.data).unwrap();
        store
            .finish_proposal_apply(
                intent.request.operation_id,
                ApplyOutcome::Applied,
                Some(&[]),
            )
            .unwrap();
        let before = store
            .action(first.action_records[0].origin.id)
            .unwrap()
            .unwrap();
        let record = store
            .create_proposal(&draft(ActionChange::Replace {
                before: Box::new(before),
                data: data("Wait for reply λ\r\n", ActionState::Waiting),
            }))
            .unwrap();
        let pending = store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: record.stamp(),
            })
            .unwrap();
        let pending = store
            .record_proposal_prepared(pending.request.operation_id, &[])
            .unwrap();
        second = completion(&pending, ApplyOutcome::Applied, Some(Vec::new()), false).unwrap();
        records.write(&second, None).unwrap();
    }
    for restore_backup in [false, true] {
        let fresh = Fixture::new();
        let backup = fresh.data.join("backups/brn-0000000000001.sqlite");
        if restore_backup {
            fs::create_dir(fresh.data.join("backups")).unwrap();
            fs::write(&backup, &old_database).unwrap();
            fs::write(fresh.data.join("brn.sqlite"), b"synthetic physical damage").unwrap();
        }
        for id in records.ids().unwrap() {
            let filename = records.read(id).unwrap().unwrap().proof.relative;
            fs::copy(f.data.join(&filename), fresh.data.join(&filename)).unwrap();
        }
        let mut app = fresh.app();
        if restore_backup {
            assert_eq!(app.open_report().restored_from.as_ref(), Some(&backup));
            assert_eq!(fs::read(&backup).unwrap(), old_database);
            assert_eq!(
                fs::read(app.open_report().corrupt_moved_to.as_ref().unwrap()).unwrap(),
                b"synthetic physical damage"
            );
        }
        let expected = second.action_records[0].clone();
        assert_eq!(
            app.work_store().action(expected.origin.id).unwrap(),
            Some(expected.clone())
        );
        assert_eq!(expected.version, 2);
        assert_eq!(expected.origin, first.action_records[0].origin);
        assert_eq!(expected.waiting_since_ms, Some(second.started_at_ms));
        for journal in [&first, &second] {
            assert_eq!(
                app.approve_proposal(&journal.request).unwrap().outcome,
                ApplyOutcome::Applied
            );
            assert!(
                app.proposal(journal.approved.draft.id)
                    .unwrap()
                    .comments
                    .is_empty()
            );
        }
        fresh.assert_vaultless(&app);
        drop(app);
        let app = fresh.app();
        assert_eq!(
            app.work_store().action(expected.origin.id).unwrap(),
            Some(expected)
        );
        fresh.assert_vaultless(&app);
    }
}

#[test]
fn bound_unsettled_action_intent_does_not_apply_from_empty_file_proofs() {
    let f = Fixture::new();
    let vault = f._base.path().join("vault");
    fs::create_dir(&vault).unwrap();
    let exact = "\u{feff}Tähtaeg λ\r\n".as_bytes();
    fs::write(vault.join("a.md"), exact).unwrap();
    let binding;
    {
        let mut app = f.app();
        app.bind_vault(&vault).unwrap();
        app.open_editor("a.md").unwrap();
        binding = serde_json::from_str::<VaultRecord>(
            &app.work_store()
                .setting("vault.editor_identity")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    }
    let journal;
    {
        let (mut store, _) = WorkStore::open(&f.data).unwrap();
        let mut request = draft(ActionChange::Create {
            id: Uuid::new_v4(),
            data: data("Must remain a Draft", ActionState::Open),
        });
        request.vault = Some(binding);
        let record = store.create_proposal(&request).unwrap();
        let pending = store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: record.stamp(),
            })
            .unwrap();
        journal = store
            .record_proposal_prepared(pending.request.operation_id, &[])
            .unwrap();
    }
    ApplyRecoveryFiles::open(&f.data)
        .unwrap()
        .write(&journal, None)
        .unwrap();
    let mut app = f.app();
    assert_eq!(
        app.reconcile_proposal(journal.request.operation_id)
            .unwrap()
            .outcome,
        ApplyOutcome::NotApplied
    );
    assert!(
        app.work_store()
            .action(journal.action_records[0].origin.id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        app.proposal(journal.approved.draft.id).unwrap().state,
        ProposalState::Draft
    );
    assert_eq!(fs::read(vault.join("a.md")).unwrap(), exact);
    assert!(!app.current_evidence_blocked().unwrap());
}

#[test]
fn vaultless_action_mirrors_do_not_relax_bound_vault_refusal() {
    let f = Fixture::new();
    let action = f.seed(true, false);
    let backups: Vec<_> = fs::read_dir(f.data.join("backups"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1);
    let old_database = fs::read(&backups[0]).unwrap();
    let applied = completion(&action, ApplyOutcome::Applied, Some(Vec::new()), false).unwrap();
    let records = ApplyRecoveryFiles::open(&f.data).unwrap();
    records.write(&applied, None).unwrap();
    let bound_root = f._base.path().join("vault");
    let wrong_root = f._base.path().join("other-vault");
    fs::create_dir(&bound_root).unwrap();
    fs::create_dir(&wrong_root).unwrap();
    {
        let (mut store, _) = WorkStore::open(&f.data).unwrap();
        store
            .finish_proposal_apply(
                action.request.operation_id,
                ApplyOutcome::Applied,
                Some(&[]),
            )
            .unwrap();
        let parent = VaultIdentity {
            device: 42,
            inode: 123,
        };
        let note = store
            .create_proposal(&ProposalDraft {
                inbox_visual: None,
                inbox_knowledge: None,
                inbox_source: None,
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                vault: Some(VaultRecord {
                    id: Uuid::new_v4(),
                    root: bound_root,
                    identity: parent.clone(),
                }),
                title: "Bound file work".into(),
                changes: vec![NoteChange::Create {
                    path: "new.md".into(),
                    parent,
                    text: "\u{feff}Exact note λ\r\n".into(),
                }],
                sources: Vec::new(),
                action_changes: Vec::new(),
            })
            .unwrap();
        let pending = store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: note.stamp(),
            })
            .unwrap();
        records.write(&pending, None).unwrap();
    }
    // A qualified older operational database can coexist with later terminal
    // and interrupted ordinary receipts; admission itself remains exclusive.
    fs::write(f.data.join("brn.sqlite"), old_database).unwrap();
    let original = records
        .read(action.request.operation_id)
        .unwrap()
        .unwrap()
        .journal;
    let error = App::open(
        &f.data,
        AppConfig {
            vault_root: Some(wrong_root.clone()),
            credentials_dir: Some(f.credentials.clone()),
            model_dir: None,
        },
    )
    .err()
    .expect("another vault must refuse before any import");
    assert_eq!(error.kind, ErrorKind::ContextStale);
    let (store, _) = WorkStore::open(&f.data).unwrap();
    assert!(
        store
            .action(action.action_records[0].origin.id)
            .unwrap()
            .is_none()
    );
    assert!(store.setting("vault.root").unwrap().is_none());
    assert_eq!(
        records
            .read(action.request.operation_id)
            .unwrap()
            .unwrap()
            .journal,
        original
    );
    assert_eq!(fs::read_dir(wrong_root).unwrap().count(), 0);
    assert!(!f.credentials.exists());
}

fn action_input(change: ActionChange) -> crate::proposals::DraftRequest {
    crate::proposals::DraftRequest {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Exact operational proposal".into(),
        changes: Vec::new(),
        sources: Vec::new(),
        action_changes: vec![change],
    }
}
fn approve_input(app: &mut App, input: &crate::proposals::DraftRequest) -> ApplyReceipt {
    let review = app.create_proposal(input).unwrap();
    app.approve_proposal(&ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    })
    .unwrap()
}
fn bound(f: &Fixture) -> (PathBuf, App) {
    let vault = f._base.path().join("vault");
    fs::create_dir(&vault).unwrap();
    let app = App::open(
        &f.data,
        AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: Some(f.credentials.clone()),
            model_dir: None,
        },
    )
    .unwrap();
    (vault, app)
}
fn managed(id: Uuid, body: &str) -> String {
    format!("\u{feff}---\r\nbrn_id: {id}\r\n---\r\n{body}\r\n")
}

#[test]
fn source_only_actions_bind_the_vault_and_refuse_fresh_and_final_source_drift() {
    for during in [false, true] {
        let f = Fixture::new();
        let (vault, mut app) = bound(&f);
        let id = Uuid::new_v4();
        let source = vault.join("source.md");
        fs::write(&source, managed(id, "Original approved õ")).unwrap();
        let capture = app.proposal_source("source.md").unwrap().source;
        let mut candidate = data("Review sourced Action", ActionState::Open);
        candidate.sources = vec![id];
        let mut input = action_input(ActionChange::Create {
            id: Uuid::new_v4(),
            data: candidate,
        });
        input.sources = vec![capture];
        let review = app.create_proposal(&input).unwrap();
        assert!(review.draft.vault.is_some());
        if during {
            let source = source.clone();
            APPLY_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |step, _| {
                    if step == "verified" {
                        fs::write(&source, "External source change").unwrap();
                    }
                }))
            });
        } else {
            fs::write(&source, "External source change").unwrap();
        }
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        };
        let result = app.approve_proposal(&request);
        APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert!(result.is_err());
        assert!(
            app.store
                .action(input.action_changes[0].id())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            app.proposal(review.draft.id).unwrap().state,
            ProposalState::Draft
        );
        assert_eq!(fs::read(&source).unwrap(), b"External source change");
        if during {
            assert_eq!(
                app.proposal_apply(request.operation_id)
                    .unwrap()
                    .unwrap()
                    .receipt
                    .unwrap()
                    .outcome,
                ApplyOutcome::NotApplied
            );
        } else {
            assert!(app.proposal_apply(request.operation_id).unwrap().is_none());
        }
        assert!(!app.current_evidence_blocked().unwrap());
        assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    }
}

#[test]
fn mixed_action_cas_drift_never_claims_partial_success_and_restore_preserves_completion() {
    use brn_store::work::actions::ActionRecord;
    for stop in ["prepared", "synced", "verified"] {
        let f = Fixture::new();
        let (vault, mut app) = bound(&f);
        let note_id = Uuid::new_v4();
        let original = managed(note_id, "Original exact λ");
        fs::write(vault.join("note.md"), &original).unwrap();
        let source = app.proposal_source("note.md").unwrap().source;
        let id = Uuid::new_v4();
        approve_input(
            &mut app,
            &action_input(ActionChange::Create {
                id,
                data: data("Existing Action", ActionState::Open),
            }),
        );
        let initial = app.action(id).unwrap();
        let mut waiting = initial.data.clone();
        waiting.state = ActionState::Waiting;
        approve_input(
            &mut app,
            &action_input(ActionChange::Replace {
                before: Box::new(initial),
                data: waiting,
            }),
        );
        let before = app.action(id).unwrap();
        let mut after = before.data.clone();
        after.state = ActionState::Blocked;
        let mut input = action_input(ActionChange::Replace {
            before: Box::new(before.clone()),
            data: after,
        });
        input
            .changes
            .push(crate::proposals::DraftNoteChange::Replace {
                path: "note.md".into(),
                expected: source.fingerprint.clone(),
                text: managed(note_id, "Reviewed note after"),
            });
        input.sources.push(source);
        let review = app.create_proposal(&input).unwrap();
        let mut competing: ActionRecord = before.clone();
        competing.version += 1;
        competing.updated_at_ms += 1;
        competing.data.state = ActionState::Completed;
        competing.waiting_since_ms = None;
        competing.completed_at_ms = Some(competing.updated_at_ms);
        competing.validate().unwrap();
        let data_dir = f.data.clone();
        let newer = competing.clone();
        APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |step, member| {
                if step == stop && member == 0 {
                    put_action(&data_dir, &newer);
                }
            }))
        });
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        };
        let result = app.approve_proposal(&request);
        APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert!(result.is_err(), "{stop}");
        let journal = app.proposal_apply(request.operation_id).unwrap().unwrap();
        let expected = if stop == "prepared" {
            ApplyOutcome::NotApplied
        } else {
            ApplyOutcome::Uncertain
        };
        assert_eq!(
            journal.receipt.as_ref().unwrap().outcome,
            expected,
            "{stop}"
        );
        let mirror = app
            .application_records()
            .unwrap()
            .read(request.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(mirror.journal.receipt.as_ref().unwrap().outcome, expected);
        assert_eq!(app.store.action(id).unwrap(), Some(competing.clone()));
        if expected == ApplyOutcome::Uncertain {
            assert!(app.action(id).is_err());
            assert!(app.current_evidence_blocked().unwrap());
            let preview = app.preview_proposal_repair(request.operation_id).unwrap();
            let finish = RepairRequest {
                id: Uuid::new_v4(),
                operation_id: request.operation_id,
                expected: preview.expected,
                direction: RepairDirection::Finish,
            };
            assert!(app.repair_proposal(&finish).is_err());
            assert!(app.store.proposal_repair(finish.id).unwrap().is_none());
            let restore = RepairRequest {
                id: Uuid::new_v4(),
                direction: RepairDirection::Restore,
                ..finish
            };
            assert_eq!(
                app.repair_proposal(&restore).unwrap().outcome,
                Some(ApplyOutcome::NotApplied)
            );
        }
        assert_eq!(
            fs::read(vault.join("note.md")).unwrap(),
            original.as_bytes()
        );
        assert_eq!(app.action(id).unwrap(), competing);
        assert!(!app.current_evidence_blocked().unwrap());
        drop(app);
        let reopened = f.app();
        assert_eq!(
            reopened.action(id).unwrap().data.state,
            ActionState::Completed
        );
        assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    }
}

#[test]
#[ignore = "subprocess entry point for exact crash qualification"]
fn action_execution_crash_child() {
    let base = PathBuf::from(std::env::var("BRN_ACTION_EXECUTION_BASE").unwrap());
    let request: ApprovalRequest =
        serde_json::from_slice(&fs::read(base.join("request.json")).unwrap()).unwrap();
    let mut app = App::open(
        &base.join("data"),
        AppConfig {
            vault_root: None,
            credentials_dir: Some(base.join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    let step = std::env::var("BRN_ACTION_EXECUTION_STEP").unwrap();
    let member = std::env::var("BRN_ACTION_EXECUTION_MEMBER")
        .ok()
        .map(|value| value.parse().unwrap())
        .unwrap_or(0);
    APPLY_CHECKPOINT.with(|point| *point.borrow_mut() = Some((step, member)));
    let _ = app.approve_proposal(&request);
    panic!("Requested crash checkpoint was not reached");
}

#[test]
fn vaultless_action_crashes_require_terminal_authority_and_never_repeat_writes() {
    for step in [
        "intent",
        "mirror-intent",
        "prepared-db",
        "prepared",
        "verified",
        "completion",
        "receipt",
    ] {
        let f = Fixture::new();
        let mut app = f.app();
        let id = Uuid::new_v4();
        let input = action_input(ActionChange::Create {
            id,
            data: data("Crash-bound exact Action õ\r\n", ActionState::Waiting),
        });
        let review = app.create_proposal(&input).unwrap();
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        };
        fs::write(
            f._base.path().join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        drop(app);
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "proposal_apply::action_recovery_tests::action_execution_crash_child",
                "--ignored",
                "--test-threads=1",
            ])
            .env("BRN_ACTION_EXECUTION_BASE", f._base.path())
            .env("BRN_ACTION_EXECUTION_STEP", step)
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(86),
            "{step}: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        let mut reopened = f.app();
        // An intent can precede its ordinary mirror. Startup keeps that SQLite
        // intent fenced; explicit reconciliation classifies it without retry.
        let receipt = reopened.reconcile_proposal(request.operation_id).unwrap();
        let journal = reopened
            .proposal_apply(request.operation_id)
            .unwrap()
            .unwrap();
        let applied = matches!(step, "completion" | "receipt");
        let expected = if applied {
            ApplyOutcome::Applied
        } else {
            ApplyOutcome::NotApplied
        };
        assert_eq!(journal.receipt.as_ref(), Some(&receipt));
        assert_eq!(receipt.outcome, expected, "{step}");
        assert_eq!(reopened.approve_proposal(&request).unwrap(), receipt);
        assert_eq!(reopened.store.action(id).unwrap().is_some(), applied);
        assert_eq!(
            reopened.proposal(review.draft.id).unwrap().state,
            if applied {
                ProposalState::Applied
            } else {
                ProposalState::Draft
            }
        );
        f.assert_vaultless(&reopened);
    }
}

#[test]
fn mixed_action_crash_matrix_settles_only_whole_proofs_and_explicitly_restores_partial_files() {
    for (step, member) in [
        ("intent", 0),
        ("mirror-intent", 0),
        ("stage", 0),
        ("stage", 1),
        ("stage", 2),
        ("prepared-db", 0),
        ("prepared", 0),
        ("member", 0),
        ("synced", 0),
        ("member", 1),
        ("synced", 1),
        ("member", 2),
        ("synced", 2),
        ("verified", 0),
        ("completion", 0),
        ("receipt", 0),
    ] {
        let f = Fixture::new();
        let (vault, mut app) = bound(&f);
        let original = managed(Uuid::new_v4(), "Original exact before õ");
        let trash = managed(Uuid::new_v4(), "Retained original source 🦀");
        fs::write(vault.join("a.md"), &original).unwrap();
        fs::write(vault.join("trash.md"), &trash).unwrap();
        let before = app.proposal_source("a.md").unwrap().source;
        let trash_before = app.proposal_source("trash.md").unwrap().source;
        let note_id = brn_store::note_identity::read(&original).unwrap().unwrap();
        let after = managed(note_id, "Reviewed after 日本語");
        let created = managed(Uuid::new_v4(), "New approved knowledge");
        let id = Uuid::new_v4();
        let mut input = action_input(ActionChange::Create {
            id,
            data: data("Joined approved Action", ActionState::Waiting),
        });
        input.changes = vec![
            crate::proposals::DraftNoteChange::Replace {
                path: "a.md".into(),
                expected: before.fingerprint.clone(),
                text: after.clone(),
            },
            crate::proposals::DraftNoteChange::Create {
                path: "new.md".into(),
                text: created.clone(),
            },
            crate::proposals::DraftNoteChange::Trash {
                path: "trash.md".into(),
                expected: trash_before.fingerprint,
            },
        ];
        input.sources = vec![before];
        let review = app.create_proposal(&input).unwrap();
        let review = app
            .add_proposal_comment(&CommentRequest {
                expected: review.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Exact mixed review λ\r\n".into(),
                    target: CommentTarget::Proposal,
                },
            })
            .unwrap();
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        };
        fs::write(
            f._base.path().join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        drop(app);
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "proposal_apply::action_recovery_tests::action_execution_crash_child",
                "--ignored",
                "--test-threads=1",
            ])
            .env("BRN_ACTION_EXECUTION_BASE", f._base.path())
            .env("BRN_ACTION_EXECUTION_STEP", step)
            .env("BRN_ACTION_EXECUTION_MEMBER", member.to_string())
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(86),
            "{step}/{member}: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        let mut reopened = f.app();
        let receipt = reopened.reconcile_proposal(request.operation_id).unwrap();
        let outcome = if matches!(step, "verified" | "completion" | "receipt")
            || matches!(step, "member" | "synced") && member == 2
        {
            ApplyOutcome::Applied
        } else if matches!(step, "member" | "synced") {
            ApplyOutcome::Uncertain
        } else {
            ApplyOutcome::NotApplied
        };
        assert_eq!(receipt.outcome, outcome, "{step}/{member}");
        assert_eq!(reopened.approve_proposal(&request).unwrap(), receipt);
        assert_eq!(
            reopened.store.action(id).unwrap().is_some(),
            outcome == ApplyOutcome::Applied
        );
        if outcome == ApplyOutcome::Uncertain {
            assert!(reopened.action(id).is_err());
            let preview = reopened
                .preview_proposal_repair(request.operation_id)
                .unwrap();
            let repair = RepairRequest {
                id: Uuid::new_v4(),
                operation_id: request.operation_id,
                expected: preview.expected,
                direction: RepairDirection::Restore,
            };
            assert_eq!(
                reopened.repair_proposal(&repair).unwrap().outcome,
                Some(ApplyOutcome::NotApplied)
            );
        }
        if outcome == ApplyOutcome::Applied {
            assert_eq!(fs::read(vault.join("a.md")).unwrap(), after.as_bytes());
            assert_eq!(fs::read(vault.join("new.md")).unwrap(), created.as_bytes());
            assert!(!vault.join("trash.md").exists());
            let action = reopened.action(id).unwrap();
            assert_eq!(action.origin.proposal, review.stamp());
            assert_eq!(action.data, input.action_changes[0].data().clone());
            assert_eq!(action.waiting_since_ms, Some(action.origin.created_at_ms));
            assert!(
                reopened
                    .proposal(review.draft.id)
                    .unwrap()
                    .comments
                    .is_empty()
            );
        } else {
            assert_eq!(fs::read(vault.join("a.md")).unwrap(), original.as_bytes());
            assert_eq!(fs::read(vault.join("trash.md")).unwrap(), trash.as_bytes());
            assert!(!vault.join("new.md").exists());
            assert!(reopened.store.action(id).unwrap().is_none());
            assert_eq!(
                reopened.proposal(review.draft.id).unwrap().comments,
                review.comments
            );
        }
        assert!(!reopened.current_evidence_blocked().unwrap());
        assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    }
}
