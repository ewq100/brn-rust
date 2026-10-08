#![cfg(target_os = "macos")]

use brn_workflow::{
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    editor::{EditRequest, EditorRecord, EditorView},
    proposal_apply::{
        ApplyJournal, ApplyOutcome, ApplyReceipt, ApprovalRequest, UndoPreview, UndoRequest,
    },
    proposals::{DraftNoteChange, DraftRequest, NoteChange, ProposalRecord},
};
use std::{fs, os::unix::fs::MetadataExt, path::PathBuf, time::Duration};
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

    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(
            self.data.clone(),
            AppConfig {
                vault_root: Some(self.vault.clone()),
                credentials_dir: Some(self.credentials.clone()),
                model_dir: None,
            },
        )
        .unwrap();
        assert!(matches!(
            next(&worker).1,
            AppEvent::Ready {
                vault_bound: true,
                model_installed: false,
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
}

fn next(worker: &AppWorker) -> (Uuid, AppEvent) {
    worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
}

fn reply(worker: &AppWorker, id: Uuid, command: AppCommand) -> AppEvent {
    worker.submit(id, command).unwrap();
    let (actual, event) = next(worker);
    assert_eq!(
        actual, id,
        "every worker reply must retain its request UUID"
    );
    if let AppEvent::Failed(error) = &event {
        panic!("worker command {id} failed: {error}");
    }
    event
}

fn request_event(worker: &AppWorker, command: AppCommand) -> AppEvent {
    reply(worker, Uuid::new_v4(), command)
}

fn open(worker: &AppWorker, path: &str) -> EditorView {
    let AppEvent::Editor(view) = request_event(worker, AppCommand::OpenEditor(path.into())) else {
        panic!("expected the editor view");
    };
    view
}

fn journals(worker: &AppWorker) -> Vec<ApplyJournal> {
    let AppEvent::ProposalApplies(journals) = request_event(worker, AppCommand::ProposalApplies)
    else {
        panic!("expected the whole-proposal application journals");
    };
    journals
}

fn proposals(worker: &AppWorker) -> Vec<ProposalRecord> {
    let AppEvent::Proposals(proposals) = request_event(worker, AppCommand::Proposals(None)) else {
        panic!("expected the proposal review records");
    };
    proposals
}

fn applied_fixture(worker: &AppWorker) -> (ApprovalRequest, EditorRecord, EditorRecord) {
    let before = open(worker, "replace.md").record;
    let trash = open(worker, "trash.md").record;
    let AppEvent::Proposal(draft) = request_event(
        worker,
        AppCommand::CreateProposal(DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Exact synthetic Replace, Create and Trash".into(),
            changes: vec![
                DraftNoteChange::Replace {
                    path: before.path.clone(),
                    expected: before.baseline.clone(),
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
            sources: Vec::new(),
        }),
    ) else {
        panic!("expected the full source proposal");
    };
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: draft.stamp(),
    };
    let AppEvent::ProposalApplied(receipt) = reply(
        worker,
        approval.operation_id,
        AppCommand::ApproveProposal(approval.clone()),
    ) else {
        panic!("expected the source approval receipt");
    };
    assert_eq!(receipt.operation_id, approval.operation_id);
    assert_eq!(receipt.proposal_id, approval.expected.id);
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    (approval, before, trash)
}

fn undo(source: &ApprovalRequest) -> UndoRequest {
    UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: source.operation_id,
        trash_member: None,
    }
}

fn preview(worker: &AppWorker, request: &UndoRequest) -> UndoPreview {
    // The presentation request and future durable operation have distinct IDs.
    let id = Uuid::new_v4();
    assert_ne!(id, request.operation_id);
    let AppEvent::ProposalUndoPreview(preview) =
        reply(worker, id, AppCommand::PreviewProposalUndo(request.clone()))
    else {
        panic!("expected the exact read-only Undo preview");
    };
    preview
}

#[test]
fn full_undo_preview_is_read_only_and_correlates_each_reply() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let (source, before, trash) = applied_fixture(&worker);
    let request = undo(&source);
    let files = fixture.vault_snapshot();
    let existing_journals = journals(&worker);
    let existing_proposals = proposals(&worker);
    assert_eq!(existing_journals.len(), 1);
    assert_eq!(existing_proposals.len(), 1);

    let reviewed = preview(&worker, &request);
    assert_eq!(reviewed.draft.id, request.operation_id);
    assert_eq!(reviewed.binding.operation_id, source.operation_id);
    assert!(reviewed.binding.trash_member.is_none());
    assert_eq!(reviewed.draft.changes.len(), 3);
    assert!(reviewed.draft.sources.is_empty());
    assert!(matches!(
        &reviewed.draft.changes[0],
        NoteChange::Replace { path, before_text, text, .. }
            if path == "replace.md" && before_text == APPROVED && text == ORIGINAL
    ));
    assert!(matches!(
        &reviewed.draft.changes[1],
        NoteChange::Trash { path, before_text, .. }
            if path == "created.md" && before_text == CREATED
    ));
    assert!(matches!(
        &reviewed.draft.changes[2],
        NoteChange::Create { path, text, .. } if path == "trash.md" && text == TRASH
    ));
    let originals = &reviewed.binding.originals;
    assert_eq!(originals.len(), 3);
    assert_eq!(originals[0].as_ref().unwrap().fingerprint, before.baseline);
    assert_eq!(
        originals[0].as_ref().unwrap().member_id,
        existing_journals[0].members[0].id
    );
    assert!(originals[1].is_none());
    assert_eq!(originals[2].as_ref().unwrap().fingerprint, trash.baseline);
    assert_eq!(
        originals[2].as_ref().unwrap().member_id,
        existing_journals[0].members[2].id
    );
    assert_eq!(preview(&worker, &request), reviewed);
    assert_eq!(fixture.vault_snapshot(), files);
    assert_eq!(journals(&worker), existing_journals);
    assert_eq!(proposals(&worker), existing_proposals);
    assert_eq!(open(&worker, "replace.md").record, before);
    assert_eq!(open(&worker, "trash.md").record, trash);
    worker.shutdown().unwrap();
    let (notification, AppEvent::BackupStatus(backup)) = worker.try_event().unwrap() else {
        panic!("joined shutdown reports its separate internal checkpoint");
    };
    assert!(notification.is_nil());
    assert!(backup.latest_path.is_file() && backup.last_error.is_none());
    assert!(worker.try_event().is_none());
}

#[test]
fn admitted_undo_and_later_old_stamp_recovery_drain_before_restart_and_replay() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let (source, before, trash) = applied_fixture(&worker);
    let request = undo(&source);
    let reviewed = preview(&worker, &request);
    let recovery_id = Uuid::new_v4();
    worker
        .submit(
            request.operation_id,
            AppCommand::UndoProposal(request.clone()),
        )
        .unwrap();
    // The owner typed against the old displayed baseline before the Undo reply.
    worker
        .submit(
            recovery_id,
            AppCommand::RecoverEditor(EditRequest {
                path: before.path.clone(),
                expected: before.stamp,
                generation: before.stamp.generation + 1,
                text: LATER.into(),
            }),
        )
        .unwrap();
    worker.shutdown().unwrap();

    let mut undone: Option<ApplyReceipt> = None;
    let mut recovered: Option<EditorRecord> = None;
    let mut checkpoint_seen = false;
    while let Some((id, event)) = worker.try_event() {
        match event {
            AppEvent::ProposalApplied(receipt) if id == request.operation_id => {
                assert!(undone.replace(receipt).is_none());
            }
            AppEvent::EditorRecovered(record) if id == recovery_id => {
                assert!(recovered.replace(record).is_none());
            }
            AppEvent::BackupStatus(status) if id.is_nil() => {
                assert!(
                    !checkpoint_seen,
                    "duplicate shutdown checkpoint notification"
                );
                assert!(status.latest_path.is_file() && status.last_error.is_none());
                checkpoint_seen = true;
            }
            AppEvent::Failed(error) => panic!("admitted command {id} failed: {error}"),
            _ => panic!("unexpected drained reply for {id}"),
        }
    }
    assert!(
        checkpoint_seen,
        "joined shutdown must report its checkpoint"
    );
    let undone = undone.expect("admitted Undo must finish before shutdown returns");
    assert_eq!(undone.operation_id, request.operation_id);
    assert_eq!(undone.proposal_id, reviewed.draft.id);
    assert_eq!(undone.outcome, ApplyOutcome::Applied);
    let recovered = recovered.expect("queued old-stamp recovery must also drain");
    assert_eq!(recovered.path, before.path);
    assert_eq!(recovered.stamp.baseline, before.stamp.baseline);
    assert_eq!(recovered.stamp.generation, before.stamp.generation + 1);
    assert_eq!(recovered.baseline, before.baseline);
    assert_eq!(recovered.baseline_text, ORIGINAL);
    assert_eq!(recovered.text, LATER);
    assert_eq!(
        fs::read(fixture.vault.join("replace.md")).unwrap(),
        ORIGINAL.as_bytes()
    );
    assert_eq!(
        fs::metadata(fixture.vault.join("replace.md"))
            .unwrap()
            .ino(),
        before.baseline.inode
    );
    assert_eq!(
        fs::read(fixture.vault.join("trash.md")).unwrap(),
        TRASH.as_bytes()
    );
    assert_eq!(
        fs::metadata(fixture.vault.join("trash.md")).unwrap().ino(),
        trash.baseline.inode
    );
    assert!(!fixture.vault.join("created.md").exists());
    drop(worker);

    let mut worker = fixture.worker();
    let restored = open(&worker, "replace.md");
    assert_eq!(restored.record, recovered);
    assert_eq!(restored.saved.as_deref(), Some(ORIGINAL));
    assert_eq!(restored.observed.as_ref(), Some(&before.baseline));
    assert!(
        !restored.conflict,
        "restored identity naturally matches the retained baseline"
    );
    assert_eq!(open(&worker, "trash.md").record, trash);
    let history = journals(&worker);
    assert_eq!(history.len(), 2);
    let completed = history
        .iter()
        .find(|journal| journal.request.operation_id == request.operation_id)
        .unwrap();
    assert_eq!(completed.approved.draft, reviewed.draft);
    assert_eq!(completed.undo.as_ref(), Some(&reviewed.binding));
    assert_eq!(completed.receipt.as_ref(), Some(&undone));
    assert_eq!(
        fs::read(fixture.vault.join(&completed.members[0].staging)).unwrap(),
        APPROVED.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault.join(&completed.members[1].staging)).unwrap(),
        CREATED.as_bytes()
    );

    // Historical replay cannot repeat namespace work, even after later owner edits.
    fs::write(
        fixture.vault.join("replace.md"),
        "Later owner bytes õ🦀\r\n",
    )
    .unwrap();
    let files = fixture.vault_snapshot();
    let AppEvent::ProposalApplied(replayed) =
        request_event(&worker, AppCommand::UndoProposal(request.clone()))
    else {
        panic!("expected the historical Undo receipt");
    };
    assert_eq!(replayed, undone);
    assert_eq!(fixture.vault_snapshot(), files);
    let current = open(&worker, "replace.md");
    assert_eq!(current.record, recovered);
    assert!(current.conflict);
    worker.shutdown().unwrap();
}
