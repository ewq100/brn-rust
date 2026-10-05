//! Operational reads remain shared, exact and fenced without a vault/provider.
use brn_store::{
    WorkStore,
    work::{
        proposal_apply::{ApplyOutcome, ApprovalRequest},
        proposals::{ActionChange, ProposalDraft},
    },
};
use brn_workflow::{
    ErrorKind,
    actions::{ActionCursor, ActionData, ActionListRequest, ActionRecord, ActionState},
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

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
    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: None,
            credentials_dir: Some(self.credentials.clone()),
            model_dir: None,
        }
    }
    fn app(&self) -> App {
        App::open(&self.data, self.config()).unwrap()
    }
    fn no_provider(&self, app: &App) {
        assert!(app.vault_root().is_none());
        assert!(app.selection().unwrap().is_none());
        assert!(app.work_store().conversations().unwrap().is_empty());
        assert_eq!(fs::read_dir(&self.credentials).unwrap().count(), 0);
    }
    fn seed(&self, state: ActionState) -> ActionRecord {
        let (mut store, _) = WorkStore::open(&self.data).unwrap();
        let id = Uuid::new_v4();
        apply(
            &mut store,
            ActionChange::Create {
                id,
                data: data(state),
            },
        );
        store.action(id).unwrap().unwrap()
    }
}
fn data(state: ActionState) -> ActionData {
    ActionData {
        title: "\u{feff}Tähtaeg 日本語 🦀\r\n\u{1b}[31m".into(),
        description: "Keep exact \"quotes\", tabs\t and CRLF.\r\n".into(),
        state,
        owner: Some("Evõ\tKessler".into()),
        related_person: None,
        related_project: None,
        sources: vec![Uuid::new_v4()],
        thread: None,
        due_on: Some("2028-02-29".into()),
        follow_up_on: Some("2028-03-01".into()),
        dependencies: Vec::new(),
        parent: None,
        follows_up: None,
        priority: None,
    }
}
fn draft(change: ActionChange) -> ProposalDraft {
    ProposalDraft {
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: None,
        title: "Synthetic exact operational review".into(),
        changes: Vec::new(),
        sources: Vec::new(),
        action_changes: vec![change],
    }
}
fn pending(store: &mut WorkStore) -> ApprovalRequest {
    let record = store
        .create_proposal(&draft(ActionChange::Create {
            id: Uuid::new_v4(),
            data: data(ActionState::Open),
        }))
        .unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    request
}
fn apply(store: &mut WorkStore, change: ActionChange) {
    let record = store.create_proposal(&draft(change)).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    store
        .record_proposal_prepared(request.operation_id, &[])
        .unwrap();
    store
        .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&[]))
        .unwrap();
}
fn next(worker: &AppWorker) -> (Uuid, AppEvent) {
    worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
}

#[test]
fn current_reads_retain_exact_updated_baseline_and_approved_origin_across_restart() {
    let f = Fixture::new();
    let before = f.seed(ActionState::Open);
    let id = before.origin.id;
    let mut app = f.app();
    assert_eq!(app.action(id).unwrap(), before);
    let mut changed = before.data.clone();
    changed.state = ActionState::Waiting;
    changed.description.push_str("New follow-up text\r\n");
    apply(
        app.work_store_mut(),
        ActionChange::Replace {
            before: Box::new(before.clone()),
            data: changed.clone(),
        },
    );
    let after = app.action(id).unwrap();
    assert_eq!(after.origin, before.origin);
    assert_eq!(after.version, before.version + 1);
    assert_eq!(after.data, changed);
    assert!(after.waiting_since_ms.is_some());
    assert!(after.completed_at_ms.is_none());
    assert_eq!(
        app.actions(&ActionListRequest::default()).unwrap().entries,
        vec![after.clone()]
    );
    f.no_provider(&app);
    drop(app);
    let app = f.app();
    assert_eq!(app.action(id).unwrap(), after);
    f.no_provider(&app);
}

#[test]
fn pagination_uses_immutable_creation_keys_and_filters_before_limiting() {
    let f = Fixture::new();
    let mut records = vec![
        f.seed(ActionState::Open),
        f.seed(ActionState::Waiting),
        f.seed(ActionState::Open),
    ];
    let mut app = f.app();
    let before = records[0].clone();
    let mut changed = before.data.clone();
    changed.state = ActionState::Blocked;
    apply(
        app.work_store_mut(),
        ActionChange::Replace {
            before: Box::new(before.clone()),
            data: changed,
        },
    );
    records[0] = app.action(before.origin.id).unwrap();
    records
        .sort_by_key(|record| std::cmp::Reverse((record.origin.created_at_ms, record.origin.id)));
    assert_eq!(
        app.actions(&ActionListRequest::default()).unwrap().entries,
        records
    );
    let mut seen = Vec::new();
    let mut request = ActionListRequest {
        limit: 1,
        ..Default::default()
    };
    loop {
        let page = app.actions(&request).unwrap();
        assert_eq!(page.entries.len(), 1);
        if let Some(cursor) = page.next_before {
            assert_eq!(
                cursor,
                ActionCursor {
                    created_at_ms: page.entries[0].origin.created_at_ms,
                    id: page.entries[0].origin.id
                }
            );
        }
        seen.extend(page.entries);
        match page.next_before {
            Some(cursor) => request.before = Some(cursor),
            None => break,
        }
    }
    assert_eq!(seen, records);
    for state in [
        ActionState::Open,
        ActionState::Waiting,
        ActionState::Blocked,
        ActionState::Completed,
    ] {
        let page = app
            .actions(&ActionListRequest {
                state: Some(state),
                limit: 200,
                before: None,
            })
            .unwrap();
        assert_eq!(
            page.entries,
            records
                .iter()
                .filter(|record| record.data.state == state)
                .cloned()
                .collect::<Vec<_>>()
        );
        assert!(page.next_before.is_none());
    }
    f.no_provider(&app);
}

#[test]
fn real_worker_correlates_exact_reads_and_typed_failures_without_knowledge_setup() {
    let f = Fixture::new();
    let expected = f.seed(ActionState::Waiting);
    for _ in 0..2 {
        let mut worker = AppWorker::start(f.data.clone(), f.config()).unwrap();
        assert!(matches!(
            next(&worker).1,
            AppEvent::Ready {
                vault_bound: false,
                ..
            }
        ));
        let request = Uuid::new_v4();
        worker
            .submit(request, AppCommand::Actions(ActionListRequest::default()))
            .unwrap();
        assert!(
            matches!(next(&worker), (id, AppEvent::Actions(page)) if id == request && page.entries == vec![expected.clone()] && page.next_before.is_none())
        );
        let request = Uuid::new_v4();
        worker
            .submit(request, AppCommand::Action(expected.origin.id))
            .unwrap();
        assert!(
            matches!(next(&worker), (id, AppEvent::Action(record)) if id == request && *record == expected)
        );
        let request = Uuid::new_v4();
        worker
            .submit(request, AppCommand::Action(Uuid::new_v4()))
            .unwrap();
        assert!(
            matches!(next(&worker), (id, AppEvent::Failed(error)) if id == request && error.kind == ErrorKind::NotFound)
        );
        let request = Uuid::new_v4();
        worker
            .submit(request, AppCommand::Action(Uuid::nil()))
            .unwrap();
        assert!(
            matches!(next(&worker), (id, AppEvent::Failed(error)) if id == request && error.kind == ErrorKind::ToolRejected)
        );
        worker.shutdown().unwrap();
    }
    f.no_provider(&f.app());
}

#[test]
fn pending_and_uncertain_proposals_fence_current_actions_until_exact_reconciliation() {
    for uncertain in [false, true] {
        let f = Fixture::new();
        let expected = f.seed(ActionState::Open);
        let mut app = f.app();
        let request = pending(app.work_store_mut());
        if uncertain {
            app.work_store_mut()
                .finish_proposal_apply(request.operation_id, ApplyOutcome::Uncertain, None)
                .unwrap();
        }
        for id in [expected.origin.id, Uuid::new_v4()] {
            assert_eq!(app.action(id).unwrap_err().kind, ErrorKind::SaveUncertain);
        }
        assert_eq!(
            app.actions(&ActionListRequest::default()).unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        assert_eq!(
            app.action(Uuid::nil()).unwrap_err().kind,
            ErrorKind::ToolRejected
        );
        for invalid in [
            ActionListRequest {
                limit: 0,
                ..Default::default()
            },
            ActionListRequest {
                limit: 201,
                ..Default::default()
            },
            ActionListRequest {
                before: Some(ActionCursor {
                    created_at_ms: 0,
                    id: Uuid::nil(),
                }),
                ..Default::default()
            },
            ActionListRequest {
                before: Some(ActionCursor {
                    created_at_ms: u64::MAX,
                    id: Uuid::new_v4(),
                }),
                ..Default::default()
            },
        ] {
            assert_eq!(
                app.actions(&invalid).unwrap_err().kind,
                ErrorKind::ToolRejected
            );
        }
        assert_eq!(
            app.work_store().action(expected.origin.id).unwrap(),
            Some(expected.clone())
        );
        drop(app);
        let mut worker = AppWorker::start(f.data.clone(), f.config()).unwrap();
        assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
        for command in [
            AppCommand::Action(expected.origin.id),
            AppCommand::Actions(ActionListRequest::default()),
        ] {
            let id = Uuid::new_v4();
            worker.submit(id, command).unwrap();
            assert!(
                matches!(next(&worker), (actual, AppEvent::Failed(error)) if actual == id && error.kind == ErrorKind::SaveUncertain)
            );
        }
        let id = Uuid::new_v4();
        worker
            .submit(id, AppCommand::ReconcileProposal(request.operation_id))
            .unwrap();
        #[cfg(target_os = "macos")]
        {
            assert!(
                matches!(next(&worker), (actual, AppEvent::ProposalApplied(receipt)) if actual == id && receipt.outcome == ApplyOutcome::NotApplied)
            );
            let id = Uuid::new_v4();
            worker
                .submit(id, AppCommand::Action(expected.origin.id))
                .unwrap();
            assert!(
                matches!(next(&worker), (actual, AppEvent::Action(record)) if actual == id && *record == expected)
            );
        }
        #[cfg(not(target_os = "macos"))]
        {
            // The shared read fence applies everywhere; ordinary recovery writes
            // retain the existing macOS filesystem-coordination requirement.
            assert!(
                matches!(next(&worker), (actual, AppEvent::Failed(error)) if actual == id && error.kind == ErrorKind::ToolRejected)
            );
            let id = Uuid::new_v4();
            worker
                .submit(id, AppCommand::Action(expected.origin.id))
                .unwrap();
            assert!(
                matches!(next(&worker), (actual, AppEvent::Failed(error)) if actual == id && error.kind == ErrorKind::SaveUncertain)
            );
        }
        worker.shutdown().unwrap();
        f.no_provider(&f.app());
    }
}

#[cfg(target_os = "macos")]
#[test]
fn interrupted_manual_save_also_fences_actions_without_mutating_records_or_note_bytes() {
    use brn_workflow::editor::{EditRequest, SaveOutcome, SaveRequest};
    for uncertain in [false, true] {
        let f = Fixture::new();
        let expected = f.seed(ActionState::Open);
        let vault = f._base.path().join("vault");
        fs::create_dir(&vault).unwrap();
        let original = b"\xef\xbb\xbfSaved \r\n";
        fs::write(vault.join("note.md"), original).unwrap();
        let mut config = f.config();
        config.vault_root = Some(vault.clone());
        let mut app = App::open(&f.data, config).unwrap();
        let record = app.open_editor("note.md").unwrap().record;
        let save = SaveRequest {
            operation_id: Uuid::new_v4(),
            destination: None,
            edit: EditRequest {
                path: "note.md".into(),
                expected: record.stamp,
                generation: record.stamp.generation + 1,
                text: "Unsaved exact 🦀\r\n".into(),
            },
        };
        let staging = PathBuf::from(format!(".brn-{}.stage", save.operation_id));
        app.work_store_mut()
            .begin_editor_save(&save, &staging)
            .unwrap();
        if uncertain {
            app.work_store_mut()
                .finish_editor_save(save.operation_id, SaveOutcome::Uncertain, None)
                .unwrap();
        }
        assert_eq!(
            app.action(expected.origin.id).unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        assert_eq!(
            app.actions(&ActionListRequest::default()).unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        let receipt = app.reconcile_editor(save.operation_id).unwrap();
        assert_eq!(receipt.outcome, SaveOutcome::NotApplied);
        assert_eq!(app.action(expected.origin.id).unwrap(), expected);
        assert_eq!(fs::read(vault.join("note.md")).unwrap(), original);
        assert_eq!(
            app.work_store().editor("note.md").unwrap().unwrap().text,
            save.edit.text
        );
        assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    }
}
