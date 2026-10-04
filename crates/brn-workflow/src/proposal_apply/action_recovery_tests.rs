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
