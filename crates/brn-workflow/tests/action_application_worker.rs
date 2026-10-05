//! Producer and approval qualification through a real joined AppWorker.
//! Ordinary approval receipts currently require the macOS file adapter.
#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind,
    actions::{ActionData, ActionListRequest, ActionPriority, ActionRecord, ActionState},
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    proposal_apply::{ApplyOutcome, ApplyReceipt, ApprovalRequest},
    proposals::{
        ActionChange, CommentRequest, CommentTarget, DraftRequest, ProposalEdit, ProposalRecord,
        ReviewComment,
    },
};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    credentials: PathBuf,
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
            data,
            credentials: owner.path().join("credentials"),
            vault,
            _owner: owner,
        }
    }

    fn worker(&self, bound: bool) -> AppWorker {
        let worker = AppWorker::start(
            self.data.clone(),
            AppConfig {
                vault_root: bound.then(|| self.vault.clone()),
                credentials_dir: Some(self.credentials.clone()),
                model_dir: None,
            },
        )
        .unwrap();
        assert!(
            matches!(worker.recv_event_timeout(Duration::from_secs(10)).unwrap().1,
            AppEvent::Ready { vault_bound, model_installed: false } if vault_bound == bound)
        );
        worker
    }

    fn quiet(&self, worker: &AppWorker) {
        assert!(matches!(
            request(worker, AppCommand::Selection),
            AppEvent::Selection(None)
        ));
        assert!(
            matches!(request(worker, AppCommand::Conversations), AppEvent::Conversations(items) if items.is_empty())
        );
        assert_eq!(fs::read_dir(&self.credentials).unwrap().count(), 0);
    }
}

fn request(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    let (actual, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(
        actual, id,
        "results must correlate with the admitted command"
    );
    event
}

fn proposal(event: AppEvent) -> ProposalRecord {
    match event {
        AppEvent::Proposal(record) => record,
        AppEvent::Failed(error) => panic!("proposal failed: {error}"),
        _ => panic!("expected full proposal record"),
    }
}

fn action(worker: &AppWorker, id: Uuid) -> ActionRecord {
    match request(worker, AppCommand::Action(id)) {
        AppEvent::Action(record) => *record,
        AppEvent::Failed(error) => panic!("Action read failed: {error}"),
        _ => panic!("expected full Action record"),
    }
}

fn apply(worker: &AppWorker, approval: ApprovalRequest) -> ApplyReceipt {
    match request(worker, AppCommand::ApproveProposal(approval)) {
        AppEvent::ProposalApplied(receipt) => receipt,
        AppEvent::Failed(error) => panic!("approval failed: {error}"),
        _ => panic!("expected full application receipt"),
    }
}

fn approval(record: &ProposalRecord) -> ApprovalRequest {
    ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    }
}

fn data(title: &str) -> ActionData {
    ActionData {
        title: title.into(),
        description: "\u{feff}Full \"quoted\" 日本語 õ\r\n\t\u{0001}\u{001b}".into(),
        state: ActionState::Waiting,
        owner: Some("  Zoë\t\u{0000}  ".into()),
        related_person: None,
        related_project: None,
        sources: vec![],
        thread: None,
        due_on: Some("2028-02-29".into()),
        follow_up_on: Some("2028-03-01".into()),
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: Some(ActionPriority::High),
    }
}

fn draft(changes: Vec<ActionChange>) -> DraftRequest {
    DraftRequest {
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Exact Action review λ".into(),
        changes: vec![],
        sources: vec![],
        action_changes: changes,
    }
}

#[test]
fn vaultless_full_review_and_joined_approval_survive_restart_without_reapplication() {
    let f = Fixture::new();
    let mut worker = f.worker(false);
    let root = Uuid::new_v4();
    let child = Uuid::new_v4();
    let mut full = data("  Exact λ\r\n\u{0007}  ");
    full.dependencies = vec![root];
    full.parent = Some(root);
    full.follows_up = Some(root);
    let input = draft(vec![
        ActionChange::Create {
            id: root,
            data: data("Root"),
        },
        ActionChange::Create {
            id: child,
            data: full.clone(),
        },
    ]);
    let created = proposal(request(&worker, AppCommand::CreateProposal(input.clone())));
    assert!(created.draft.vault.is_none());
    assert_eq!(created.draft.action_changes, input.action_changes);
    assert!(
        matches!(request(&worker, AppCommand::Actions(ActionListRequest::default())), AppEvent::Actions(page) if page.entries.is_empty())
    );
    let commented = proposal(request(
        &worker,
        AppCommand::AddProposalComment(CommentRequest {
            expected: created.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Whole Action review 日本語\r\n".into(),
                target: CommentTarget::Proposal,
            },
        }),
    ));
    full.description = "\u{feff}Reviewed exact bytes λ\r\n\t\u{001b}".into();
    let edit = ProposalEdit {
        expected: commented.stamp(),
        title: "Reviewed full Action λ".into(),
        texts: vec![],
        action_data: vec![data("Root"), full.clone()],
    };
    let reviewed = proposal(request(&worker, AppCommand::EditProposal(edit.clone())));
    assert_eq!(reviewed.version, 3);
    assert_eq!(reviewed.comments, commented.comments);
    assert_eq!(
        proposal(request(&worker, AppCommand::CreateProposal(input))),
        reviewed
    );
    assert!(
        matches!(request(&worker, AppCommand::EditProposal(edit)), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    assert!(
        matches!(request(&worker, AppCommand::ApproveProposal(approval(&commented))), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    assert!(
        matches!(request(&worker, AppCommand::ProposalApplies), AppEvent::ProposalApplies(items) if items.is_empty())
    );

    let approved = approval(&reviewed);
    let command = Uuid::new_v4();
    worker
        .submit(command, AppCommand::ApproveProposal(approved.clone()))
        .unwrap();
    worker.shutdown().unwrap(); // Admitted approval must settle before join.
    let (actual, event) = worker.try_event().expect("joined approval acknowledgement");
    assert_eq!(actual, command);
    let AppEvent::ProposalApplied(receipt) = event else {
        panic!("joined Action approval");
    };
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    assert_eq!(receipt.approved_version, reviewed.version);
    assert!(worker.try_event().is_none());
    drop(worker);

    let mut worker = f.worker(false);
    let before = action(&worker, child);
    assert_eq!(before.data, full);
    assert_eq!(before.origin.data, full);
    assert_eq!(before.origin.proposal, reviewed.stamp());
    assert_eq!(before.waiting_since_ms, Some(before.origin.created_at_ms));
    assert_eq!(before.version, 1);
    assert_eq!(apply(&worker, approved.clone()), receipt);
    assert!(
        matches!(request(&worker, AppCommand::ReconcileProposal(approved.operation_id)), AppEvent::ProposalApplied(reconciled) if reconciled == receipt)
    );
    let applied = proposal(request(&worker, AppCommand::Proposal(reviewed.draft.id)));
    assert!(applied.comments.is_empty());
    assert_eq!(applied.draft, reviewed.draft);
    assert!(
        matches!(request(&worker, AppCommand::ProposalApply(approved.operation_id)), AppEvent::ProposalApply(Some(journal)) if journal.approved.draft == reviewed.draft && journal.action_records.iter().any(|record| record == &before) && journal.receipt.as_ref() == Some(&receipt))
    );

    let mut after = full.clone();
    after.title = "Replacement õ\r\n".into();
    after.priority = Some(ActionPriority::Low);
    let replacing = proposal(request(
        &worker,
        AppCommand::CreateProposal(draft(vec![ActionChange::Replace {
            before: Box::new(before.clone()),
            data: after.clone(),
        }])),
    ));
    assert_eq!(action(&worker, child), before);
    let replacement = approval(&replacing);
    let replacement_receipt = apply(&worker, replacement.clone());
    let current = action(&worker, child);
    assert_eq!(current.version, 2);
    assert_eq!(current.data, after);
    assert_eq!(current.origin, before.origin);
    assert_eq!(current.waiting_since_ms, before.waiting_since_ms);
    assert_eq!(apply(&worker, replacement), replacement_receipt);
    assert_eq!(apply(&worker, approved), receipt);
    assert_eq!(action(&worker, child), current);
    f.quiet(&worker);
    worker.shutdown().unwrap();
    assert!(!f.data.join("index.sqlite").exists());
    assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 0);
}

#[test]
fn mixed_note_action_receipt_and_source_failure_keep_full_operational_and_vault_state() {
    use brn_workflow::proposals::DraftNoteChange;
    let f = Fixture::new();
    let source_id = Uuid::new_v4();
    let text = format!(
        "\u{feff}---\r\nbrn_id: {source_id}\r\nbrn_kind: source\r\n---\r\nExact original õ\r\n"
    );
    fs::write(f.vault.join("source.md"), &text).unwrap();
    let mut worker = f.worker(true);
    let AppEvent::ProposalSource(source) =
        request(&worker, AppCommand::ProposalSource("source.md".into()))
    else {
        panic!("full coordinated source");
    };
    assert_eq!(source.text, text);
    let id = Uuid::new_v4();
    let mut full = data("Full source-linked Action λ");
    full.related_person = Some(source_id);
    full.related_project = Some(source_id);
    full.thread = Some(source_id);
    full.sources = vec![source_id];
    let note = "\u{feff}Approved mixed 日本語\r\n";
    let mut input = draft(vec![ActionChange::Create {
        id,
        data: full.clone(),
    }]);
    input.changes = vec![DraftNoteChange::Create {
        path: "created.md".into(),
        text: note.into(),
    }];
    input.sources = vec![source.source.clone()];
    let reviewed = proposal(request(&worker, AppCommand::CreateProposal(input)));
    assert!(reviewed.draft.vault.is_some());
    assert!(
        matches!(request(&worker, AppCommand::Action(id)), AppEvent::Failed(error) if error.kind == ErrorKind::NotFound)
    );
    assert!(!f.vault.join("created.md").exists());
    let approved = approval(&reviewed);
    let receipt = apply(&worker, approved.clone());
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    assert_eq!(receipt.proposal_id, reviewed.draft.id);
    let before = action(&worker, id);
    assert_eq!(before.data, full);
    assert_eq!(before.origin.data, full);
    assert_eq!(before.origin.proposal, reviewed.stamp());
    assert_eq!(
        fs::read(f.vault.join("created.md")).unwrap(),
        note.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        text.as_bytes()
    );
    worker.shutdown().unwrap();
    drop(worker);

    let mut worker = f.worker(true);
    assert_eq!(action(&worker, id), before);
    assert_eq!(apply(&worker, approved.clone()), receipt);
    assert!(
        matches!(request(&worker, AppCommand::ProposalApply(approved.operation_id)), AppEvent::ProposalApply(Some(journal)) if journal.approved.draft == reviewed.draft && journal.action_records == vec![before.clone()] && journal.receipt.as_ref() == Some(&receipt))
    );
    let sibling = Uuid::new_v4();
    let mut changed = full;
    changed.description = "Retain complete proposal-only λ\r\n".into();
    let mut blocked_input = draft(vec![
        ActionChange::Replace {
            before: Box::new(before.clone()),
            data: changed,
        },
        ActionChange::Create {
            id: sibling,
            data: data("Refused sibling"),
        },
    ]);
    blocked_input.changes = vec![DraftNoteChange::Create {
        path: "blocked.md".into(),
        text: "Must remain absent".into(),
    }];
    blocked_input.sources = vec![source.source];
    let blocked = proposal(request(&worker, AppCommand::CreateProposal(blocked_input)));
    let AppEvent::ProposalApplies(journals) = request(&worker, AppCommand::ProposalApplies) else {
        panic!("journal list");
    };
    fs::write(f.vault.join("source.md"), "External source edit 🦀\r\n").unwrap();
    assert!(
        matches!(request(&worker, AppCommand::ApproveProposal(approval(&blocked))), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    assert_eq!(action(&worker, id), before);
    assert!(
        matches!(request(&worker, AppCommand::Action(sibling)), AppEvent::Failed(error) if error.kind == ErrorKind::NotFound)
    );
    assert_eq!(
        proposal(request(&worker, AppCommand::Proposal(blocked.draft.id))),
        blocked
    );
    assert!(
        matches!(request(&worker, AppCommand::ProposalApplies), AppEvent::ProposalApplies(current) if current == journals)
    );
    assert!(!f.vault.join("blocked.md").exists());
    assert_eq!(
        fs::read(f.vault.join("created.md")).unwrap(),
        note.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        "External source edit 🦀\r\n".as_bytes()
    );
    f.quiet(&worker);
    worker.shutdown().unwrap();
}
