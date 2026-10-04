#![cfg(target_os = "macos")]

use brn_workflow::{
    ErrorKind,
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    editor::{EditRequest, EditorRecord},
    proposal_apply::{ApplyOutcome, ApprovalRequest},
    proposals::{DraftNoteChange, DraftRequest},
};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

const ORIGINAL: &str = "\u{feff}Original λ\r\n";
const APPROVED: &str = "\u{feff}Approved 日本語🦀\r\n";
const LATER: &str = "\u{feff}Later unfinished typing õ🦀\r\n";

struct Fixture {
    _base: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        let vault = base.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(vault.join("note.md"), ORIGINAL).unwrap();
        let credentials = base.path().join("credentials");
        Self {
            _base: base,
            data,
            vault,
            credentials,
        }
    }

    fn worker(&self) -> AppWorker {
        AppWorker::start(
            self.data.clone(),
            AppConfig {
                vault_root: Some(self.vault.clone()),
                credentials_dir: Some(self.credentials.clone()),
                model_dir: None,
            },
        )
        .unwrap()
    }
}

fn next(worker: &AppWorker) -> (Uuid, AppEvent) {
    worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
}

fn request(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    let (actual, event) = next(worker);
    assert_eq!(actual, id, "worker must correlate this command's result");
    event
}

fn open(worker: &AppWorker) -> brn_workflow::editor::EditorView {
    match request(worker, AppCommand::OpenEditor("note.md".into())) {
        AppEvent::Editor(view) => view,
        AppEvent::Failed(error) => panic!("editor open failed: {error}"),
        _ => panic!("expected the editor view"),
    }
}

#[test]
fn admitted_approval_later_old_stamp_recovery_and_reconciliation_drain_before_restart() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    assert!(matches!(
        next(&worker).1,
        AppEvent::Ready {
            vault_bound: true,
            model_installed: false,
        }
    ));
    let before = open(&worker).record;
    let proposal_id = Uuid::new_v4();
    let draft = match request(
        &worker,
        AppCommand::CreateProposal(DraftRequest {
            id: proposal_id,
            group_id: None,
            session_id: None,
            title: "Approve this exact synthetic whole proposal".into(),
            changes: vec![
                DraftNoteChange::Replace {
                    path: "note.md".into(),
                    expected: before.baseline.clone(),
                    text: APPROVED.into(),
                },
                DraftNoteChange::Create {
                    path: "created.md".into(),
                    text: "Created evidence\r\n".into(),
                },
            ],
            sources: Vec::new(),
        }),
    ) {
        AppEvent::Proposal(record) => record,
        AppEvent::Failed(error) => panic!("proposal creation failed: {error}"),
        _ => panic!("expected the full proposal review record"),
    };
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: draft.stamp(),
    };
    let recovery = Uuid::new_v4();
    let reconciliation = Uuid::new_v4();
    worker
        .submit(
            approval.operation_id,
            AppCommand::ApproveProposal(approval.clone()),
        )
        .unwrap();
    // This snapshot was typed against the previously displayed baseline. It is
    // admitted after approval, before receiving any apply acknowledgement.
    worker
        .submit(
            recovery,
            AppCommand::RecoverEditor(EditRequest {
                path: before.path.clone(),
                expected: before.stamp,
                generation: before.stamp.generation + 1,
                text: LATER.into(),
            }),
        )
        .unwrap();
    worker
        .submit(
            reconciliation,
            AppCommand::ReconcileProposal(approval.operation_id),
        )
        .unwrap();
    // Do not wait for any of these results before closing admission and joining.
    worker.shutdown().unwrap();

    let mut applied = None;
    let mut recovered: Option<EditorRecord> = None;
    let mut reconciled = None;
    while let Some((id, event)) = worker.try_event() {
        match event {
            AppEvent::ProposalApplied(receipt) if id == approval.operation_id => {
                assert!(applied.replace(receipt).is_none());
            }
            AppEvent::EditorRecovered(record) if id == recovery => {
                assert!(recovered.replace(record).is_none());
            }
            AppEvent::ProposalApplied(receipt) if id == reconciliation => {
                assert!(reconciled.replace(receipt).is_none());
            }
            AppEvent::Failed(error) => panic!("admitted command {id} failed: {error}"),
            _ => panic!("unexpected drained event for {id}"),
        }
    }
    let applied = applied.expect("approval must drain and acknowledge its exact receipt");
    assert_eq!(applied.operation_id, approval.operation_id);
    assert_eq!(applied.proposal_id, proposal_id);
    assert_eq!(applied.approved_version, draft.version);
    assert_eq!(applied.outcome, ApplyOutcome::Applied);
    assert_eq!(reconciled, Some(applied.clone()));
    let recovered = recovered.expect("later recovery must drain and acknowledge its exact buffer");
    assert_eq!(recovered.stamp.baseline, before.stamp.baseline);
    assert_eq!(recovered.stamp.generation, before.stamp.generation + 1);
    assert_eq!(recovered.baseline, before.baseline);
    assert_eq!(recovered.baseline_text, ORIGINAL);
    assert_eq!(recovered.text, LATER);
    assert_eq!(
        fs::read(fixture.vault.join("note.md")).unwrap(),
        APPROVED.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault.join("created.md")).unwrap(),
        b"Created evidence\r\n"
    );
    drop(worker);

    let mut worker = fixture.worker();
    assert!(matches!(
        next(&worker).1,
        AppEvent::Ready {
            vault_bound: true,
            ..
        }
    ));
    let view = open(&worker);
    assert_eq!(view.record, recovered);
    assert_eq!(view.saved.as_deref(), Some(APPROVED));
    assert!(
        view.conflict,
        "the older local baseline must remain visibly stale"
    );
    assert!(
        view.pending.is_empty(),
        "a settled proposal is not an uncertain Save"
    );
    assert_ne!(view.observed.as_ref(), Some(&view.record.baseline));
    let AppEvent::ProposalApplies(journals) = request(&worker, AppCommand::ProposalApplies) else {
        panic!("expected the recoverable whole-proposal journal list");
    };
    assert_eq!(journals.len(), 1);
    assert_eq!(journals[0].request, approval);
    assert_eq!(journals[0].receipt.as_ref(), Some(&applied));
    assert_eq!(
        fs::read(fixture.vault.join(&journals[0].members[0].staging)).unwrap(),
        ORIGINAL.as_bytes()
    );
    let AppEvent::ProposalApplied(replayed) =
        request(&worker, AppCommand::ApproveProposal(approval))
    else {
        panic!("exact replay must return its receipt before later editor conflict gates");
    };
    assert_eq!(replayed, applied);
    assert_eq!(open(&worker).record, recovered);
    assert_eq!(
        fs::read(fixture.vault.join("note.md")).unwrap(),
        APPROVED.as_bytes()
    );
    worker.shutdown().unwrap();
}

#[test]
fn malformed_ordinary_receipt_fails_startup_before_ready_or_current_evidence() {
    for filename in [
        format!(".brn-apply-{}.receipt", Uuid::new_v4()),
        ".brn-apply-invalid.receipt".into(),
    ] {
        let fixture = Fixture::new();
        fs::write(fixture.data.join(&filename), b"synthetic malformed receipt").unwrap();
        let mut worker = fixture.worker();
        // Admission can race asynchronous opening. Regardless of admission, no
        // current note result may escape the failed startup recovery boundary.
        let _ = worker.submit(Uuid::new_v4(), AppCommand::Note("note.md".into()));
        let (_, event) = next(&worker);
        let AppEvent::Failed(error) = event else {
            panic!("malformed recovery must fail before emitting Ready or note evidence");
        };
        assert_eq!(error.kind, ErrorKind::ToolRejected);
        assert!(worker.shutdown().is_err());
        assert!(worker.try_event().is_none());
        assert_eq!(
            fs::read(fixture.vault.join("note.md")).unwrap(),
            ORIGINAL.as_bytes()
        );
        assert_eq!(
            fs::read(fixture.data.join(filename)).unwrap(),
            b"synthetic malformed receipt"
        );
        assert!(
            !fixture.data.join("index.sqlite").exists(),
            "failed recovery must precede vault scanning"
        );
    }
}
