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
fn recovery_summaries_exclude_history_and_identified_snapshots_preserve_partial_effects() {
    use brn_store::{WorkStore, files::FileFingerprint};
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::MetadataExt;
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
    let create = |path: &str, text: &str| {
        let AppEvent::Proposal(record) = request(
            &worker,
            AppCommand::CreateProposal(DraftRequest {
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: format!("Exact {path}"),
                changes: vec![DraftNoteChange::Create {
                    path: path.into(),
                    text: text.into(),
                }],
                sources: vec![],
            }),
        ) else {
            panic!("draft");
        };
        record
    };
    let historical = create("historical.md", APPROVED);
    let historical_request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: historical.stamp(),
    };
    assert!(
        matches!(request(&worker, AppCommand::ApproveProposal(historical_request.clone())), AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    let AppEvent::Proposal(mixed) = request(
        &worker,
        AppCommand::CreateProposal(DraftRequest {
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Partial full snapshot λ".into(),
            changes: vec![
                DraftNoteChange::Create {
                    path: "one.md".into(),
                    text: APPROVED.repeat(4096),
                },
                DraftNoteChange::Create {
                    path: "two.md".into(),
                    text: LATER.into(),
                },
            ],
            sources: vec![],
        }),
    ) else {
        panic!("mixed draft");
    };
    worker.shutdown().unwrap();
    let mixed_request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: mixed.stamp(),
    };
    let (mut store, _) = WorkStore::open(&fixture.data).unwrap();
    let intent = store.begin_proposal_apply(&mixed_request).unwrap();
    let mut prepared = vec![];
    for (member, change) in intent.members.iter().zip(&intent.approved.draft.changes) {
        let stage = fixture.vault.join(&member.staging);
        let bytes = change.text().unwrap().as_bytes();
        fs::write(&stage, bytes).unwrap();
        let metadata = fs::metadata(&stage).unwrap();
        prepared.push(FileFingerprint {
            device: metadata.dev(),
            inode: metadata.ino(),
            len: metadata.len(),
            sha256: Sha256::digest(bytes).into(),
        });
    }
    let journal = store
        .record_proposal_prepared(mixed_request.operation_id, &prepared)
        .unwrap();
    fs::rename(
        fixture.vault.join(&journal.members[0].staging),
        fixture.vault.join("one.md"),
    )
    .unwrap();
    drop(store);
    let mut worker = fixture.worker();
    assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
    let AppEvent::ProposalRecovery(summaries) = request(&worker, AppCommand::ProposalRecovery)
    else {
        panic!("recovery summaries");
    };
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].request, mixed_request);
    assert_eq!(summaries[0].title, mixed.draft.title);
    assert_eq!(summaries[0].outcome, None);
    let encoded = serde_json::to_string(&summaries).unwrap();
    assert!(encoded.len() < 1024 && !encoded.contains(APPROVED));
    assert!(
        matches!(request(&worker, AppCommand::ReconcileProposal(mixed_request.operation_id)), AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Uncertain)
    );
    let AppEvent::ProposalRecovery(summaries) = request(&worker, AppCommand::ProposalRecovery)
    else {
        panic!("classified recovery summary");
    };
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].request, mixed_request);
    assert_eq!(summaries[0].outcome, Some(ApplyOutcome::Uncertain));
    assert!(
        matches!(request(&worker, AppCommand::ProposalApply(historical_request.operation_id)), AppEvent::ProposalApply(Some(snapshot)) if snapshot.request == historical_request && snapshot.approved.draft.changes[0].text() == Some(APPROVED))
    );
    let AppEvent::ProposalApply(Some(snapshot)) = request(
        &worker,
        AppCommand::ProposalApply(mixed_request.operation_id),
    ) else {
        panic!("identified full snapshot");
    };
    assert_eq!(snapshot.approved.draft, mixed.draft);
    assert_eq!(
        snapshot.receipt.as_ref().unwrap().outcome,
        ApplyOutcome::Uncertain
    );
    assert!(matches!(
        request(&worker, AppCommand::ProposalApply(Uuid::new_v4())),
        AppEvent::ProposalApply(None)
    ));
    assert!(
        matches!(request(&worker, AppCommand::ProposalApply(Uuid::nil())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert_eq!(
        fs::read(fixture.vault.join("historical.md")).unwrap(),
        APPROVED.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault.join("one.md")).unwrap(),
        APPROVED.repeat(4096).as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault.join(&journal.members[1].staging)).unwrap(),
        LATER.as_bytes()
    );
    assert!(!fixture.vault.join("two.md").exists());
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
