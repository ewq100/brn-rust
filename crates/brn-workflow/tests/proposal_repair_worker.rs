#![cfg(target_os = "macos")]

use brn_store::{WorkStore, files::FileFingerprint};
use brn_workflow::{
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    editor::{EditRequest, EditorRecord, EditorView},
    proposal_apply::{
        ApplyJournal, ApplyMemberPhase, ApplyOutcome, ApprovalRequest, RepairDirection,
        RepairPreview, RepairReceipt, RepairRequest,
    },
    proposals::{
        CommentRequest, CommentTarget, DraftNoteChange, DraftRequest, ProposalRecord,
        ProposalState, ReviewComment, SourceVersion,
    },
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

const ORIGINAL: &str = "\u{feff}Original õ🦀\r\n正文\nlast\r";
const APPROVED: &str = "\u{feff}Approved 日本語 λ\r\n";
const TRASH: &str = "\u{feff}Retained Trash õ🦀\r\n";
const CREATED: &str = "Created evidence λ\r\n";
const LATER: &str = "\u{feff}Queued unfinished typing 日本語🦀\r\n";

struct Fixture {
    _base: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
}

struct Mixed {
    journal: ApplyJournal,
    editor: EditorRecord,
    trash: EditorRecord,
}

impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        let vault = base.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(vault.join("replace.md"), ORIGINAL).unwrap();
        fs::write(vault.join("trash.md"), TRASH).unwrap();
        let credentials = base.path().join("credentials");
        Self {
            _base: base,
            data,
            vault,
            credentials,
        }
    }

    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: Some(self.vault.clone()),
            credentials_dir: Some(self.credentials.clone()),
            model_dir: None,
        }
    }

    fn mixed(&self) -> Mixed {
        let mut app = App::open(&self.data, self.config()).unwrap();
        let editor = app.open_editor("replace.md").unwrap().record;
        let trash = app.open_editor("trash.md").unwrap().record;
        let draft = app
            .create_proposal(&DraftRequest {
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Exact synthetic mixed Replace, Create and Trash".into(),
                changes: vec![
                    DraftNoteChange::Replace {
                        path: editor.path.clone(),
                        expected: editor.baseline.clone(),
                        text: APPROVED.into(),
                    },
                    DraftNoteChange::Create {
                        path: "created.md".into(),
                        text: CREATED.into(),
                    },
                    DraftNoteChange::Trash {
                        path: trash.path.clone(),
                        expected: trash.baseline.clone(),
                    },
                ],
                sources: vec![
                    SourceVersion {
                        path: editor.path.clone(),
                        fingerprint: editor.baseline.clone(),
                    },
                    SourceVersion {
                        path: trash.path.clone(),
                        fingerprint: trash.baseline.clone(),
                    },
                ],
            })
            .unwrap();
        let reviewed = app
            .add_proposal_comment(&CommentRequest {
                expected: draft.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Temporary synthetic review comment".into(),
                    target: CommentTarget::Proposal,
                },
            })
            .unwrap();
        let approval = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: reviewed.stamp(),
        };
        drop(app);

        // Public storage admission plus controlled disposable-file moves model
        // interruption after Replace and Trash, with Create still prepared.
        // No private ordinary-recovery encoding is manufactured by this fixture.
        let (mut store, _) = WorkStore::open(&self.data).unwrap();
        let intent = store.begin_proposal_apply(&approval).unwrap();
        let replace_stage = self.vault.join(&intent.members[0].staging);
        let create_stage = self.vault.join(&intent.members[1].staging);
        fs::write(&replace_stage, APPROVED).unwrap();
        fs::write(&create_stage, CREATED).unwrap();
        let prepared = vec![
            fingerprint(&replace_stage),
            fingerprint(&create_stage),
            trash.baseline.clone(),
        ];
        let journal = store
            .record_proposal_prepared(approval.operation_id, &prepared)
            .unwrap();
        let retained = self
            .vault
            .join(format!(".fixture-original-{}", Uuid::new_v4()));
        fs::rename(self.vault.join("replace.md"), &retained).unwrap();
        fs::rename(&replace_stage, self.vault.join("replace.md")).unwrap();
        fs::rename(retained, &replace_stage).unwrap();
        fs::rename(
            self.vault.join("trash.md"),
            self.vault.join(&journal.members[2].staging),
        )
        .unwrap();
        drop(store);
        assert_eq!(fingerprint(&replace_stage), editor.baseline);
        assert_eq!(fingerprint(&self.vault.join("replace.md")), prepared[0]);
        assert_eq!(
            fingerprint(&self.vault.join(&journal.members[2].staging)),
            trash.baseline
        );
        Mixed {
            journal,
            editor,
            trash,
        }
    }

    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(self.data.clone(), self.config()).unwrap();
        assert!(matches!(
            next(&worker).1,
            AppEvent::Ready {
                vault_bound: true,
                model_installed: false
            }
        ));
        worker
    }

    fn vault_snapshot(&self) -> Vec<(String, Vec<u8>, u64)> {
        let mut files = fs::read_dir(&self.vault)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                assert!(entry.file_type().unwrap().is_file());
                (
                    entry.file_name().into_string().unwrap(),
                    fs::read(entry.path()).unwrap(),
                    entry.metadata().unwrap().ino(),
                )
            })
            .collect::<Vec<_>>();
        files.sort();
        files
    }

    fn assert_endpoint(&self, mixed: &Mixed, direction: RepairDirection) {
        let prepared = mixed.journal.prepared.as_ref().unwrap();
        let pairs = match direction {
            RepairDirection::Finish => [
                (Some(&prepared[0]), Some(&mixed.editor.baseline)),
                (Some(&prepared[1]), None),
                (None, Some(&mixed.trash.baseline)),
            ],
            RepairDirection::Restore => [
                (Some(&mixed.editor.baseline), Some(&prepared[0])),
                (None, Some(&prepared[1])),
                (Some(&mixed.trash.baseline), None),
            ],
        };
        for ((change, member), (destination, staging)) in mixed
            .journal
            .approved
            .draft
            .changes
            .iter()
            .zip(&mixed.journal.members)
            .zip(pairs)
        {
            assert_proof(&self.vault.join(change.path()), destination);
            assert_proof(&self.vault.join(&member.staging), staging);
        }
    }
}

fn fingerprint(path: &Path) -> FileFingerprint {
    let bytes = fs::read(path).unwrap();
    let metadata = fs::metadata(path).unwrap();
    FileFingerprint {
        device: metadata.dev(),
        inode: metadata.ino(),
        len: bytes.len() as u64,
        sha256: Sha256::digest(bytes).into(),
    }
}

fn assert_proof(path: &Path, expected: Option<&FileFingerprint>) {
    match expected {
        Some(expected) => assert_eq!(&fingerprint(path), expected, "{}", path.display()),
        None => assert!(
            !path.try_exists().unwrap(),
            "{} must remain absent",
            path.display()
        ),
    }
}

fn next(worker: &AppWorker) -> (Uuid, AppEvent) {
    worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
}

fn reply(worker: &AppWorker, id: Uuid, command: AppCommand) -> AppEvent {
    worker.submit(id, command).unwrap();
    let (actual, event) = next(worker);
    assert_eq!(
        actual, id,
        "worker replies must retain their presentation request UUID"
    );
    if let AppEvent::Failed(error) = &event {
        panic!("worker command {id} failed: {error}");
    }
    event
}

fn request_event(worker: &AppWorker, command: AppCommand) -> AppEvent {
    reply(worker, Uuid::new_v4(), command)
}

fn preview(worker: &AppWorker, operation: Uuid) -> RepairPreview {
    let id = Uuid::new_v4();
    assert_ne!(id, operation);
    let AppEvent::ProposalRepairPreview(preview) =
        reply(worker, id, AppCommand::PreviewProposalRepair(operation))
    else {
        panic!("expected the full exact repair preview");
    };
    preview
}

fn journals(worker: &AppWorker) -> Vec<ApplyJournal> {
    let AppEvent::ProposalApplies(journals) = request_event(worker, AppCommand::ProposalApplies)
    else {
        panic!("expected the application journals");
    };
    journals
}

fn proposals(worker: &AppWorker) -> Vec<ProposalRecord> {
    let AppEvent::Proposals(proposals) = request_event(worker, AppCommand::Proposals(None)) else {
        panic!("expected the proposal review records");
    };
    proposals
}

fn open(worker: &AppWorker) -> EditorView {
    let AppEvent::Editor(view) = request_event(worker, AppCommand::OpenEditor("replace.md".into()))
    else {
        panic!("expected the retained editor view");
    };
    view
}

fn repair(preview: &RepairPreview, direction: RepairDirection) -> RepairRequest {
    RepairRequest {
        id: Uuid::new_v4(),
        operation_id: preview.operation_id,
        expected: preview.expected,
        direction,
    }
}

fn outcome(direction: RepairDirection) -> ApplyOutcome {
    match direction {
        RepairDirection::Finish => ApplyOutcome::Applied,
        RepairDirection::Restore => ApplyOutcome::NotApplied,
    }
}

#[test]
fn full_repair_preview_is_read_only_and_finish_restore_receipts_are_correlated() {
    for direction in [RepairDirection::Finish, RepairDirection::Restore] {
        let fixture = Fixture::new();
        let mixed = fixture.mixed();
        let mut worker = fixture.worker();
        let files = fixture.vault_snapshot();
        let existing_journals = journals(&worker);
        let existing_proposals = proposals(&worker);
        assert_eq!(existing_journals.len(), 1);
        assert_eq!(existing_proposals.len(), 1);
        let reviewed = preview(&worker, mixed.journal.request.operation_id);
        assert_eq!(reviewed.operation_id, mixed.journal.request.operation_id);
        assert_eq!(reviewed.approved, mixed.journal.approved.draft);
        assert_eq!(
            reviewed.phases,
            vec![
                ApplyMemberPhase::Applied,
                ApplyMemberPhase::Before,
                ApplyMemberPhase::Applied
            ]
        );
        assert_eq!(preview(&worker, reviewed.operation_id), reviewed);
        assert_eq!(fixture.vault_snapshot(), files);
        assert_eq!(journals(&worker), existing_journals);
        assert_eq!(proposals(&worker), existing_proposals);
        assert_eq!(open(&worker).record, mixed.editor);

        let request = repair(&reviewed, direction);
        let AppEvent::ProposalRepaired(receipt) = reply(
            &worker,
            request.id,
            AppCommand::RepairProposal(request.clone()),
        ) else {
            panic!("expected the typed repair receipt");
        };
        assert_eq!(
            receipt,
            RepairReceipt {
                id: request.id,
                operation_id: request.operation_id,
                direction,
                outcome: Some(outcome(direction))
            }
        );
        fixture.assert_endpoint(&mixed, direction);
        let records = proposals(&worker);
        assert_eq!(records[0].draft, mixed.journal.approved.draft);
        assert_eq!(
            records[0].state,
            match direction {
                RepairDirection::Finish => ProposalState::Applied,
                RepairDirection::Restore => ProposalState::Draft,
            }
        );
        assert_eq!(
            records[0].comments.is_empty(),
            direction == RepairDirection::Finish
        );
        worker.shutdown().unwrap();
        assert!(worker.try_event().is_none());
    }
}

#[test]
fn admitted_repair_and_queued_old_stamp_typing_drain_restart_and_replay_without_effects() {
    for direction in [RepairDirection::Finish, RepairDirection::Restore] {
        let fixture = Fixture::new();
        let mixed = fixture.mixed();
        let mut worker = fixture.worker();
        let reviewed = preview(&worker, mixed.journal.request.operation_id);
        let request = repair(&reviewed, direction);
        let recovery_id = Uuid::new_v4();
        worker
            .submit(request.id, AppCommand::RepairProposal(request.clone()))
            .unwrap();
        // Typing was captured against the old displayed baseline before any
        // repair acknowledgement. Both admitted mutations must survive close.
        worker
            .submit(
                recovery_id,
                AppCommand::RecoverEditor(EditRequest {
                    path: mixed.editor.path.clone(),
                    expected: mixed.editor.stamp,
                    generation: mixed.editor.stamp.generation + 1,
                    text: LATER.into(),
                }),
            )
            .unwrap();
        worker.shutdown().unwrap();
        let mut repaired = None;
        let mut recovered: Option<EditorRecord> = None;
        while let Some((id, event)) = worker.try_event() {
            match event {
                AppEvent::ProposalRepaired(receipt) if id == request.id => {
                    assert!(repaired.replace(receipt).is_none());
                }
                AppEvent::EditorRecovered(record) if id == recovery_id => {
                    assert!(recovered.replace(record).is_none());
                }
                AppEvent::Failed(error) => panic!("admitted command {id} failed: {error}"),
                _ => panic!("unexpected drained reply for {id}"),
            }
        }
        let repaired = repaired.expect("admitted repair must drain before shutdown returns");
        assert_eq!(repaired.id, request.id);
        assert_eq!(repaired.operation_id, request.operation_id);
        assert_eq!(repaired.direction, direction);
        assert_eq!(repaired.outcome, Some(outcome(direction)));
        let recovered = recovered.expect("queued old-stamp editor recovery must also drain");
        assert_eq!(recovered.path, mixed.editor.path);
        assert_eq!(recovered.stamp.baseline, mixed.editor.stamp.baseline);
        assert_eq!(
            recovered.stamp.generation,
            mixed.editor.stamp.generation + 1
        );
        assert_eq!(recovered.baseline, mixed.editor.baseline);
        assert_eq!(recovered.baseline_text, ORIGINAL);
        assert_eq!(recovered.text, LATER);
        fixture.assert_endpoint(&mixed, direction);
        drop(worker);

        let mut worker = fixture.worker();
        let view = open(&worker);
        assert_eq!(view.record, recovered);
        assert_eq!(
            view.saved.as_deref(),
            Some(match direction {
                RepairDirection::Finish => APPROVED,
                RepairDirection::Restore => ORIGINAL,
            })
        );
        assert_eq!(view.conflict, direction == RepairDirection::Finish);
        assert!(view.pending.is_empty());
        let history = journals(&worker);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].approved.draft, mixed.journal.approved.draft);
        assert_eq!(
            history[0].receipt.as_ref().unwrap().outcome,
            outcome(direction)
        );
        let attempts = &history[0].repair.as_ref().unwrap().attempts;
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].request, request);
        assert_eq!(attempts[0].outcome, Some(outcome(direction)));

        fs::write(
            fixture.vault.join("replace.md"),
            "Later owner bytes õ🦀\r\n",
        )
        .unwrap();
        let files = fixture.vault_snapshot();
        let AppEvent::ProposalRepaired(replayed) =
            request_event(&worker, AppCommand::RepairProposal(request.clone()))
        else {
            panic!("expected the historical repair receipt");
        };
        assert_eq!(replayed, repaired);
        assert_eq!(fixture.vault_snapshot(), files);
        let current = open(&worker);
        assert_eq!(current.record, recovered);
        assert!(current.conflict);
        worker.shutdown().unwrap();
    }
}
