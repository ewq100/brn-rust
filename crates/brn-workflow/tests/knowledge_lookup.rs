#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind, MAX_NOTE_BYTES,
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    knowledge::{IdentityInventory, IdentityOutcome, IdentityResolution, note_identity},
    proposal_apply::ApprovalRequest,
    proposals::{DraftNoteChange, DraftRequest},
};
use std::{fs, os::unix::fs::symlink, path::PathBuf, time::Duration};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        let credentials = owner.path().join("credentials");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        Self {
            _owner: owner,
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
    fn write(&self, path: &str, text: &[u8]) {
        let path = self.vault.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn managed(&self, path: &str, id: Uuid) -> String {
        let text = note_identity::assign(
            "\u{feff}# Täpne allikas 日本語\r\nOriginal wording 🦀\r\n",
            id,
        )
        .unwrap();
        self.write(path, text.as_bytes());
        text
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(self.data.clone(), self.config()).unwrap();
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
}
fn reply(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    let (actual, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(actual, id);
    event
}
fn inventory(worker: &AppWorker) -> IdentityInventory {
    match reply(worker, AppCommand::IdentityInventory) {
        AppEvent::IdentityInventory(report) => *report,
        AppEvent::Failed(error) => panic!("inventory failed: {error}"),
        _ => panic!("expected inventory"),
    }
}
fn resolve(worker: &AppWorker, id: Uuid) -> IdentityResolution {
    match reply(worker, AppCommand::ResolveNoteIdentity(id)) {
        AppEvent::NoteIdentityResolved(report) => *report,
        AppEvent::Failed(error) => panic!("resolution failed: {error}"),
        _ => panic!("expected identity resolution"),
    }
}

#[test]
fn archives_participate_in_duplicate_resolution_but_keep_current_read_and_write_rules() {
    let fixture = Fixture::new();
    let id = Uuid::new_v4();
    let current = fixture.managed("current.md", id);
    let archived = fixture.managed("Archive/old.md", id);
    fixture.write("unmanaged.md", b"# unmanaged\n");
    let mut worker = fixture.worker();
    let report = inventory(&worker);
    assert_eq!(report.notes.len(), 3);
    assert!(report.issues.is_empty());
    assert_eq!(report.duplicates.len(), 1);
    assert_eq!(report.duplicates[0].note_id, id);
    assert_eq!(report.duplicates[0].paths, ["Archive/old.md", "current.md"]);
    let resolution = resolve(&worker, id);
    assert_eq!(resolution.outcome, IdentityOutcome::Ambiguous);
    assert_eq!(resolution.matches.len(), 2);
    assert!(resolution.issues.is_empty());
    assert!(
        matches!(reply(&worker, AppCommand::Note("Archive/old.md".into())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert!(
        matches!(reply(&worker, AppCommand::ProposalSource("Archive/old.md".into())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert!(
        matches!(reply(&worker, AppCommand::OpenEditor("Archive/old.md".into())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert!(
        matches!(reply(&worker, AppCommand::EvidenceNote("Archive/old.md".into())), AppEvent::EvidenceNote(note) if note.text == archived)
    );
    assert!(
        matches!(reply(&worker, AppCommand::Notes{folder:None,cursor:None}), AppEvent::Notes(page) if page.notes.iter().all(|n| !n.path.starts_with("Archive/")))
    );
    assert!(
        matches!(reply(&worker, AppCommand::Proposals(None)), AppEvent::Proposals(notes) if notes.is_empty())
    );
    assert!(
        matches!(reply(&worker, AppCommand::Editors), AppEvent::Editors(notes) if notes.is_empty())
    );
    worker.shutdown().unwrap();
    fs::remove_file(fixture.data.join("index.sqlite")).unwrap();
    let mut worker = fixture.worker();
    assert_eq!(inventory(&worker), report);
    assert_eq!(resolve(&worker, id), resolution);
    worker.shutdown().unwrap();
    assert_eq!(
        fs::read(fixture.vault.join("current.md")).unwrap(),
        current.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault.join("Archive/old.md")).unwrap(),
        archived.as_bytes()
    );
}

#[test]
fn same_size_retained_mtime_new_duplicates_moves_removals_and_absence_are_fresh() {
    let fixture = Fixture::new();
    let id = Uuid::new_v4();
    let second = Uuid::new_v4();
    fixture.managed("a.md", id);
    let original = fixture.managed("b.md", second);
    let mut worker = fixture.worker();
    assert_eq!(resolve(&worker, id).outcome, IdentityOutcome::Unique);
    let times = fs::FileTimes::new().set_modified(
        fs::metadata(fixture.vault.join("b.md"))
            .unwrap()
            .modified()
            .unwrap(),
    );
    let changed = original.replace(&second.to_string(), &id.to_string());
    assert_eq!(changed.len(), original.len());
    fixture.write("b.md", changed.as_bytes());
    fs::File::options()
        .write(true)
        .open(fixture.vault.join("b.md"))
        .unwrap()
        .set_times(times)
        .unwrap();
    assert_eq!(resolve(&worker, id).outcome, IdentityOutcome::Ambiguous);
    assert_eq!(resolve(&worker, second).outcome, IdentityOutcome::Absent);
    fs::create_dir(fixture.vault.join("archive")).unwrap();
    fs::rename(
        fixture.vault.join("b.md"),
        fixture.vault.join("archive/moved.md"),
    )
    .unwrap();
    fs::remove_file(fixture.vault.join("a.md")).unwrap();
    let unique = resolve(&worker, id);
    assert_eq!(unique.outcome, IdentityOutcome::Unique);
    assert_eq!(unique.matches[0].path, "archive/moved.md");
    assert_eq!(
        fs::read(fixture.vault.join("archive/moved.md")).unwrap(),
        changed.as_bytes()
    );
    assert!(
        matches!(reply(&worker, AppCommand::ResolveNoteIdentity(Uuid::nil())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    worker.shutdown().unwrap();
    let mut worker = fixture.worker();
    assert_eq!(resolve(&worker, id), unique);
    worker.shutdown().unwrap();
}

#[test]
fn incomplete_inspection_cannot_certify_unique_or_absent_ids_and_exact_bad_source_is_readable() {
    let fixture = Fixture::new();
    let id = Uuid::new_v4();
    fixture.managed("a.md", id);
    fixture.write(
        "archive/bad.md",
        b"---\nbrn_id: not-a-uuid\n---\noriginal\n",
    );
    fixture.write("archive/binary.md", &[0xff, 0xfe, 0xfd]);
    fixture.write(
        "archive/unsupported\\name.md",
        b"---\nbrn_id: unknown\n---\n",
    );
    fixture.write("large.md", &vec![b'x'; MAX_NOTE_BYTES + 1]);
    let mut worker = fixture.worker();
    let report = inventory(&worker);
    assert_eq!(report.notes.len(), 1);
    assert_eq!(
        report
            .issues
            .iter()
            .map(|issue| issue.path.as_str())
            .collect::<Vec<_>>(),
        [
            "archive/bad.md",
            "archive/binary.md",
            "archive/unsupported\\name.md",
            "large.md"
        ]
    );
    for wanted in [id, Uuid::new_v4()] {
        let result = resolve(&worker, wanted);
        assert_eq!(result.outcome, IdentityOutcome::Incomplete);
        assert_eq!(result.issues, report.issues);
    }
    assert!(
        matches!(reply(&worker, AppCommand::EvidenceNote("archive/bad.md".into())), AppEvent::EvidenceNote(note) if note.text == "---\nbrn_id: not-a-uuid\n---\noriginal\n")
    );
    assert!(
        matches!(reply(&worker, AppCommand::EvidenceNote("archive/binary.md".into())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert!(
        matches!(reply(&worker, AppCommand::EvidenceNote("large.md".into())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    worker.shutdown().unwrap();
}

#[test]
fn hidden_and_symlink_entries_are_excluded_without_following_them() {
    let fixture = Fixture::new();
    let id = Uuid::new_v4();
    fixture.managed("a.md", id);
    fixture.managed(".private/hidden.md", id);
    symlink(fixture.vault.join("a.md"), fixture.vault.join("link.md")).unwrap();
    symlink(
        fixture.vault.join(".private"),
        fixture.vault.join("linked-folder"),
    )
    .unwrap();
    let mut worker = fixture.worker();
    let report = inventory(&worker);
    assert_eq!(report.notes.len(), 1);
    assert!(report.duplicates.is_empty() && report.issues.is_empty());
    assert_eq!(resolve(&worker, id).outcome, IdentityOutcome::Unique);
    for path in [
        ".private/hidden.md",
        "link.md",
        "linked-folder/hidden.md",
        "../a.md",
    ] {
        assert!(
            matches!(reply(&worker, AppCommand::EvidenceNote(path.into())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
        );
    }
    worker.shutdown().unwrap();
}

#[test]
fn unresolved_application_fences_inventory_resolution_and_explicit_archived_reads() {
    let fixture = Fixture::new();
    let id = Uuid::new_v4();
    let current = fixture.managed("a.md", id);
    let original = fixture.managed("archive/original.md", Uuid::new_v4());
    let mut app = App::open(&fixture.data, fixture.config()).unwrap();
    let source = app.proposal_source("a.md").unwrap();
    let record = app
        .create_proposal(&DraftRequest {
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Pending known proposal".into(),
            changes: vec![DraftNoteChange::Create {
                path: "new.md".into(),
                text: "# new\n".into(),
            }],
            sources: vec![source.source],
        })
        .unwrap();
    app.work_store_mut()
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        })
        .unwrap();
    assert_eq!(
        app.identity_inventory().unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        app.resolve_note_identity(id).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        app.evidence_note("archive/original.md").unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        fs::read(fixture.vault.join("a.md")).unwrap(),
        current.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault.join("archive/original.md")).unwrap(),
        original.as_bytes()
    );
    assert!(!fixture.vault.join("new.md").exists());
}
