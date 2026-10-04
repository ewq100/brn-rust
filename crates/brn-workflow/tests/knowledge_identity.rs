#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind, MAX_NOTE_BYTES,
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    knowledge::{IdentityRequest, note_identity},
    proposal_apply::{ApplyOutcome, ApprovalRequest, UndoRequest},
    proposals::{DraftNoteChange, DraftRequest, ProposalEdit},
};
use std::{fs, os::unix::fs::MetadataExt, path::PathBuf, time::Duration};
use uuid::Uuid;

const PATH: &str = "資料-õ.md";
const ORIGINAL: &str = "\u{feff}---\r\ncustom: λ\r\nquoted: 'source wording'\r\n---\r\n# 正文 🦀\r\nTäpsed allikad\nlast\r";

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
}
impl Fixture {
    fn new(text: &str) -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        assert!(!owner.path().starts_with(env!("CARGO_MANIFEST_DIR")));
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        let credentials = owner.path().join("credentials");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(vault.join(PATH), text).unwrap();
        Self {
            _owner: owner,
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
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready {
                vault_bound: true,
                model_installed: false
            }
        ));
        worker
    }
    fn request(&self) -> IdentityRequest {
        IdentityRequest {
            path: PATH.into(),
            note_id: Uuid::new_v4(),
            proposal_id: Uuid::new_v4(),
            title: "Assign reviewed note identity 日本語\r\n".into(),
        }
    }
}
fn reply(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    let (actual, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(actual, id);
    event
}
fn prepare(worker: &AppWorker, request: &IdentityRequest) -> DraftRequest {
    match reply(worker, AppCommand::PrepareNoteIdentity(request.clone())) {
        AppEvent::NoteIdentityDraft(draft) => *draft,
        AppEvent::Failed(error) => panic!("identity preparation: {error}"),
        _ => panic!("expected complete identity draft"),
    }
}
fn unchanged(worker: &AppWorker, fixture: &Fixture, expected: &str) {
    assert_eq!(
        fs::read(fixture.vault.join(PATH)).unwrap(),
        expected.as_bytes()
    );
    assert!(
        matches!(reply(worker, AppCommand::Editors), AppEvent::Editors(records) if records.is_empty())
    );
    assert!(
        matches!(reply(worker, AppCommand::Proposals(None)), AppEvent::Proposals(records) if records.is_empty())
    );
    assert_eq!(fs::read_dir(&fixture.vault).unwrap().count(), 1);
}

#[test]
fn exact_identity_assignment_is_reviewed_then_restored_by_undo_and_restart() {
    let fixture = Fixture::new(ORIGINAL);
    let mut worker = fixture.worker();
    let original_inode = fs::metadata(fixture.vault.join(PATH)).unwrap().ino();
    let request = fixture.request();
    assert!(
        matches!(reply(&worker, AppCommand::NoteIdentity(PATH.into())), AppEvent::NoteIdentity(info) if info.note_id.is_none())
    );
    let draft = prepare(&worker, &request);
    let expected = ORIGINAL.replacen(
        "---\r\n",
        &format!("---\r\nbrn_id: {}\r\n", request.note_id),
        1,
    );
    let DraftNoteChange::Replace {
        path,
        expected: proof,
        text,
    } = &draft.changes[0]
    else {
        panic!()
    };
    assert_eq!(path, PATH);
    assert_eq!(text.as_bytes(), expected.as_bytes());
    assert_eq!(draft.id, request.proposal_id);
    assert_eq!(draft.sources[0].fingerprint, *proof);
    assert_eq!(proof.inode, original_inode);
    assert_eq!(prepare(&worker, &request), draft);
    unchanged(&worker, &fixture, ORIGINAL);
    let AppEvent::Proposal(record) = reply(&worker, AppCommand::CreateProposal(draft.clone()))
    else {
        panic!()
    };
    assert_eq!(
        fs::read(fixture.vault.join(PATH)).unwrap(),
        ORIGINAL.as_bytes()
    );
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    assert!(
        matches!(reply(&worker, AppCommand::ApproveProposal(approval.clone())), AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    assert_eq!(
        fs::read(fixture.vault.join(PATH)).unwrap(),
        expected.as_bytes()
    );
    assert!(
        matches!(reply(&worker, AppCommand::NoteIdentity(PATH.into())), AppEvent::NoteIdentity(info) if info.note_id == Some(request.note_id))
    );
    worker.shutdown().unwrap();
    let mut worker = fixture.worker();
    assert!(
        matches!(reply(&worker, AppCommand::NoteIdentity(PATH.into())), AppEvent::NoteIdentity(info) if info.note_id == Some(request.note_id))
    );
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: approval.operation_id,
        trash_member: None,
    };
    assert!(
        matches!(reply(&worker, AppCommand::UndoProposal(undo.clone())), AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    assert_eq!(
        fs::read(fixture.vault.join(PATH)).unwrap(),
        ORIGINAL.as_bytes()
    );
    assert_eq!(
        fs::metadata(fixture.vault.join(PATH)).unwrap().ino(),
        original_inode
    );
    assert!(
        matches!(reply(&worker, AppCommand::NoteIdentity(PATH.into())), AppEvent::NoteIdentity(info) if info.note_id.is_none())
    );
    worker.shutdown().unwrap();
    let mut worker = fixture.worker();
    assert!(
        matches!(reply(&worker, AppCommand::UndoProposal(undo)), AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    assert_eq!(
        fs::read(fixture.vault.join(PATH)).unwrap(),
        ORIGINAL.as_bytes()
    );
    worker.shutdown().unwrap();
}

#[test]
fn stale_prepared_sources_and_malformed_or_oversized_identity_refuse_without_work() {
    let fixture = Fixture::new(ORIGINAL);
    let mut worker = fixture.worker();
    let draft = prepare(&worker, &fixture.request());
    fs::write(fixture.vault.join(PATH), "Later full source λ\r\n").unwrap();
    assert!(
        matches!(reply(&worker, AppCommand::CreateProposal(draft)), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    unchanged(&worker, &fixture, "Later full source λ\r\n");
    for bad in [
        "---\nbrn_id: invalid\n---\nbody",
        "---\nbrn_id: 00000000-0000-0000-0000-000000000000\n---\nbody",
        "---\nbrn_id: invalid\nbrn_id: invalid\n---\nbody",
        "---\nbrn_id: 9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d\n  extra scalar text\n---\nbody",
    ] {
        fs::write(fixture.vault.join(PATH), bad).unwrap();
        assert!(
            matches!(reply(&worker, AppCommand::NoteIdentity(PATH.into())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
        );
        assert!(
            matches!(reply(&worker, AppCommand::PrepareNoteIdentity(fixture.request())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
        );
        unchanged(&worker, &fixture, bad);
    }
    let large = "x".repeat(MAX_NOTE_BYTES);
    fs::write(fixture.vault.join(PATH), &large).unwrap();
    assert!(
        matches!(reply(&worker, AppCommand::PrepareNoteIdentity(fixture.request())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    unchanged(&worker, &fixture, &large);
    worker.shutdown().unwrap();
}

#[test]
fn fresh_replacement_and_full_edit_cannot_drop_or_swap_known_identity() {
    let fixture = Fixture::new(ORIGINAL);
    let mut worker = fixture.worker();
    let request = fixture.request();
    let managed = note_identity::assign(ORIGINAL, request.note_id).unwrap();
    fs::write(fixture.vault.join(PATH), &managed).unwrap();
    assert!(
        matches!(reply(&worker, AppCommand::PrepareNoteIdentity(request.clone())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    let AppEvent::ProposalSource(source) = reply(&worker, AppCommand::ProposalSource(PATH.into()))
    else {
        panic!()
    };
    for text in [
        ORIGINAL.into(),
        note_identity::assign(ORIGINAL, Uuid::new_v4()).unwrap(),
    ] {
        let draft = DraftRequest {
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Invalid identity replacement".into(),
            changes: vec![DraftNoteChange::Replace {
                path: PATH.into(),
                expected: source.source.fingerprint.clone(),
                text,
            }],
            sources: vec![source.source.clone()],
        };
        assert!(
            matches!(reply(&worker, AppCommand::CreateProposal(draft)), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
        );
        unchanged(&worker, &fixture, &managed);
    }
    let proposed = format!("{managed}\nApproved body improvement õ\n");
    let draft = DraftRequest {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Preserve identity".into(),
        changes: vec![DraftNoteChange::Replace {
            path: PATH.into(),
            expected: source.source.fingerprint.clone(),
            text: proposed.clone(),
        }],
        sources: vec![source.source.clone()],
    };
    let AppEvent::Proposal(record) = reply(&worker, AppCommand::CreateProposal(draft.clone()))
    else {
        panic!()
    };
    let invalid = ProposalEdit {
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: vec![Some(ORIGINAL.into())],
    };
    assert!(matches!(
        reply(&worker, AppCommand::EditProposal(invalid)),
        AppEvent::Failed(_)
    ));
    assert!(
        matches!(reply(&worker, AppCommand::Proposal(record.draft.id)), AppEvent::Proposal(saved) if saved == record)
    );
    assert!(
        matches!(reply(&worker, AppCommand::CreateProposal(draft)), AppEvent::Proposal(saved) if saved == record)
    );
    assert_eq!(
        fs::read(fixture.vault.join(PATH)).unwrap(),
        managed.as_bytes()
    );
    worker.shutdown().unwrap();
}

#[test]
fn identity_observation_reads_fresh_bytes_and_unresolved_work_fences_preparation() {
    let fixture = Fixture::new(ORIGINAL);
    let mut app = App::open(
        &fixture.data,
        AppConfig {
            vault_root: Some(fixture.vault.clone()),
            credentials_dir: Some(fixture.credentials.clone()),
            model_dir: None,
        },
    )
    .unwrap();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let managed = note_identity::assign(ORIGINAL, first).unwrap();
    fs::write(fixture.vault.join(PATH), &managed).unwrap();
    let times = fs::FileTimes::new().set_modified(
        fs::metadata(fixture.vault.join(PATH))
            .unwrap()
            .modified()
            .unwrap(),
    );
    app.refresh().unwrap();
    assert_eq!(app.note_identity(PATH).unwrap().note_id, Some(first));
    let changed = managed.replace(&first.to_string(), &second.to_string());
    fs::write(fixture.vault.join(PATH), &changed).unwrap();
    fs::File::options()
        .write(true)
        .open(fixture.vault.join(PATH))
        .unwrap()
        .set_times(times)
        .unwrap();
    assert_eq!(app.note_identity(PATH).unwrap().note_id, Some(second));
    let source = app.proposal_source(PATH).unwrap();
    let draft = DraftRequest {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Interrupted unrelated creation".into(),
        changes: vec![DraftNoteChange::Create {
            path: "new.md".into(),
            text: "New source".into(),
        }],
        sources: vec![source.source],
    };
    let record = app.create_proposal(&draft).unwrap();
    app.work_store_mut()
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        })
        .unwrap();
    assert_eq!(
        app.note_identity(PATH).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        app.prepare_note_identity(&fixture.request())
            .unwrap_err()
            .kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        fs::read(fixture.vault.join(PATH)).unwrap(),
        changed.as_bytes()
    );
    assert!(!fixture.vault.join("new.md").exists());
}

#[test]
fn unrelated_block_scalar_identity_words_and_delimiters_remain_opaque() {
    for original in [
        "---\nsummary: |\n  brn_id: literal text\n---\nbody λ\n",
        "---\nsummary: |\n  ---\n  retained original wording\n---\nbody λ\n",
    ] {
        let fixture = Fixture::new(original);
        let mut worker = fixture.worker();
        assert!(
            matches!(reply(&worker, AppCommand::NoteIdentity(PATH.into())), AppEvent::NoteIdentity(info) if info.note_id.is_none())
        );
        let request = fixture.request();
        let draft = prepare(&worker, &request);
        let DraftNoteChange::Replace { text, .. } = &draft.changes[0] else {
            panic!()
        };
        assert_eq!(
            text,
            &original.replacen("---\n", &format!("---\nbrn_id: {}\n", request.note_id), 1)
        );
        unchanged(&worker, &fixture, original);
        worker.shutdown().unwrap();
    }
}

#[test]
fn unsupported_root_flow_identity_is_reported_instead_of_assigning_a_second_id() {
    let original = format!(
        "---\n{{title: hi, brn_id: {}}}\n---\nbody λ\n",
        Uuid::new_v4()
    );
    let fixture = Fixture::new(&original);
    let mut worker = fixture.worker();
    assert!(
        matches!(reply(&worker, AppCommand::NoteIdentity(PATH.into())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert!(
        matches!(reply(&worker, AppCommand::PrepareNoteIdentity(fixture.request())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    unchanged(&worker, &fixture, &original);
    worker.shutdown().unwrap();
}

#[test]
fn ordinary_unmanaged_markdown_horizontal_rule_does_not_require_metadata() {
    let fixture = Fixture::new(ORIGINAL);
    let mut worker = fixture.worker();
    let text = "---\n# Work λ\nOriginal Markdown body\n";
    let draft = DraftRequest {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Ordinary Markdown".into(),
        changes: vec![DraftNoteChange::Create {
            path: "ordinary.md".into(),
            text: text.into(),
        }],
        sources: vec![],
    };
    let AppEvent::Proposal(record) = reply(&worker, AppCommand::CreateProposal(draft)) else {
        panic!("ordinary valid Markdown refused")
    };
    let edit = ProposalEdit {
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: vec![Some(format!("{text}Later wording\n"))],
    };
    assert!(matches!(
        reply(&worker, AppCommand::EditProposal(edit)),
        AppEvent::Proposal(_)
    ));
    assert!(!fixture.vault.join("ordinary.md").exists());
    worker.shutdown().unwrap();
}
