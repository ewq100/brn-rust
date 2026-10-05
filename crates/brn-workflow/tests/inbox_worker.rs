//! Actual headless owner, exact originals, availability and backup reconciliation.
#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind,
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    inbox::{
        CaptureInboxRequest, InboxAvailability, InboxItem, InboxKind, InboxListRequest,
        InboxOriginal, InboxRead,
    },
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, symlink},
    path::PathBuf,
    time::Duration,
};
use uuid::Uuid;
struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    credentials: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        fs::create_dir(&data).unwrap();
        Self {
            data,
            credentials: owner.path().join("credentials"),
            _owner: owner,
        }
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(
            self.data.clone(),
            AppConfig {
                vault_root: None,
                credentials_dir: Some(self.credentials.clone()),
                model_dir: None,
            },
        )
        .unwrap();
        loop {
            match worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1
            {
                AppEvent::Ready {
                    vault_bound: false,
                    model_installed: false,
                } => return worker,
                AppEvent::Restored { .. } => {}
                AppEvent::Failed(e) => panic!("startup: {e}"),
                _ => panic!("unexpected startup"),
            }
        }
    }
    fn request(&self, text: &str) -> CaptureInboxRequest {
        CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: InboxKind::Email,
            title: "Exact email õ 日本語".into(),
            original_name: Some("../../label only.eml".into()),
            text: text.into(),
        }
    }
}
fn query(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = match &command {
        AppCommand::CaptureInbox(r) => r.id,
        _ => Uuid::new_v4(),
    };
    worker.submit(id, command).unwrap();
    let (reply, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(reply, id);
    event
}
fn capture(worker: &AppWorker, r: &CaptureInboxRequest) -> InboxItem {
    match query(worker, AppCommand::CaptureInbox(r.clone())) {
        AppEvent::InboxCaptured(item) => *item,
        AppEvent::Failed(e) => panic!("capture: {e}"),
        _ => panic!("capture reply"),
    }
}
fn read(worker: &AppWorker, id: Uuid) -> InboxRead {
    match query(worker, AppCommand::InboxItem(id)) {
        AppEvent::InboxItem(item) => *item,
        AppEvent::Failed(e) => panic!("read: {e}"),
        _ => panic!("read reply"),
    }
}
#[test]
fn exact_unicode_bom_crlf_empty_distinct_replay_fifo_and_restart_without_knowledge_or_credentials()
{
    let f = Fixture::new();
    let mut worker = f.worker();
    assert!(!f.data.join("inbox").exists());
    let text = "\u{feff}From: Zoe\r\n\r\nÕun λ 日本語\r\n\t\u{0001}";
    let r = f.request(text);
    let first = capture(&worker, &r);
    assert_eq!(
        read(&worker, r.id),
        InboxRead {
            item: first.clone(),
            original: InboxOriginal::Available { text: text.into() }
        }
    );
    assert_eq!(capture(&worker, &r), first);
    let mut changed = r.clone();
    changed.kind = InboxKind::Teams;
    assert!(
        matches!(query(&worker,AppCommand::CaptureInbox(changed)),AppEvent::Failed(e) if e.kind==ErrorKind::OperationConflict)
    );
    let mut equal = r.clone();
    equal.id = Uuid::new_v4();
    let second = capture(&worker, &equal);
    assert_ne!(
        first.capture.copy.file_inode,
        second.capture.copy.file_inode
    );
    let empty = f.request("");
    let third = capture(&worker, &empty);
    assert!(
        matches!(read(&worker,empty.id).original,InboxOriginal::Available{text} if text.is_empty())
    );
    let AppEvent::InboxItems(page) = query(
        &worker,
        AppCommand::InboxItems(InboxListRequest {
            limit: 1,
            after: None,
        }),
    ) else {
        panic!("page")
    };
    assert_eq!(page.total_count, 3);
    assert_eq!(page.entries.len(), 1);
    assert!(page.issues.is_empty());
    assert!(!page.issues_truncated);
    assert_eq!(page.entries[0].availability, InboxAvailability::Available);
    let first_id = page.entries[0].item.capture.id;
    let AppEvent::InboxItems(next) = query(
        &worker,
        AppCommand::InboxItems(InboxListRequest {
            limit: 100,
            after: page.next_after,
        }),
    ) else {
        panic!("next page")
    };
    assert_eq!(next.entries.len(), 2);
    assert!(next.entries.iter().all(|e| e.item.capture.id != first_id));
    assert!(next.next_after.is_none());
    assert_eq!(
        fs::metadata(f.data.join("inbox")).unwrap().mode() & 0o777,
        0o700
    );
    for item in [&first, &second, &third] {
        let path = item.capture.copy.directory.join(item.capture.copy_name());
        let m = fs::metadata(&path).unwrap();
        assert_eq!(m.mode() & 0o777, 0o600);
        assert_eq!(m.nlink(), 1);
        assert_eq!(m.ino(), item.capture.copy.file_inode);
        assert_eq!(m.dev(), item.capture.copy.file_device);
    }
    assert!(!f.data.join("index.sqlite").exists());
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    worker.shutdown().unwrap();
    let mut worker = f.worker();
    assert_eq!(capture(&worker, &r), first);
    assert_eq!(capture(&worker, &empty), third);
    assert!(
        matches!(read(&worker,r.id).original,InboxOriginal::Available{text:actual} if actual==text)
    );
    worker.shutdown().unwrap();
}
#[test]
fn changed_missing_equal_byte_replacement_and_file_aliases_never_replace_the_retained_receipt() {
    let f = Fixture::new();
    let mut worker = f.worker();
    let r = f.request("exact original\r\n");
    let first = capture(&worker, &r);
    let path = first.capture.copy.directory.join(first.capture.copy_name());
    fs::write(&path, "changed").unwrap();
    assert!(matches!(
        read(&worker, r.id).original,
        InboxOriginal::Changed { .. }
    ));
    assert_eq!(capture(&worker, &r), first);
    let retained = path.with_extension("retained");
    fs::rename(&path, &retained).unwrap();
    assert_eq!(read(&worker, r.id).original, InboxOriginal::Missing);
    assert_eq!(capture(&worker, &r), first);
    fs::write(&path, &r.text).unwrap();
    assert_ne!(
        fs::metadata(&path).unwrap().ino(),
        first.capture.copy.file_inode
    );
    assert!(matches!(
        read(&worker, r.id).original,
        InboxOriginal::Unavailable { .. } | InboxOriginal::Changed { .. }
    ));
    fs::remove_file(&path).unwrap();
    symlink(&retained, &path).unwrap();
    assert!(matches!(
        read(&worker, r.id).original,
        InboxOriginal::Unavailable { .. }
    ));
    fs::remove_file(&path).unwrap();
    fs::hard_link(&retained, &path).unwrap();
    assert!(matches!(
        read(&worker, r.id).original,
        InboxOriginal::Unavailable { .. }
    ));
    assert_eq!(capture(&worker, &r), first);
    assert_eq!(fs::read(&retained).unwrap(), b"changed");
    worker.shutdown().unwrap();
    let mut worker = f.worker();
    assert_eq!(read(&worker, r.id).item, first);
    assert!(matches!(
        read(&worker, r.id).original,
        InboxOriginal::Unavailable { .. }
    ));
    let AppEvent::InboxItems(page) =
        query(&worker, AppCommand::InboxItems(InboxListRequest::default()))
    else {
        panic!("inventory")
    };
    assert!(!page.issues.is_empty());
    worker.shutdown().unwrap();
}
#[test]
fn replaced_root_is_not_adopted_and_replay_does_not_touch_it() {
    let f = Fixture::new();
    let mut worker = f.worker();
    let r = f.request("kept exact");
    let first = capture(&worker, &r);
    let root = f.data.join("inbox");
    let moved = f.data.join("retained-inbox");
    fs::rename(&root, &moved).unwrap();
    fs::create_dir(&root).unwrap();
    let marker = root.join("unrelated");
    fs::write(&marker, b"do not touch").unwrap();
    assert_eq!(capture(&worker, &r), first);
    assert!(matches!(
        read(&worker, r.id).original,
        InboxOriginal::Unavailable { .. }
    ));
    assert!(
        matches!(query(&worker,AppCommand::CaptureInbox(f.request("fresh"))),AppEvent::Failed(e) if e.kind==ErrorKind::InboxUnavailable)
    );
    worker.shutdown().unwrap();
    let mut worker = f.worker();
    assert_eq!(capture(&worker, &r), first);
    let AppEvent::InboxItems(page) =
        query(&worker, AppCommand::InboxItems(InboxListRequest::default()))
    else {
        panic!("inventory")
    };
    assert_eq!(page.total_count, 1);
    assert!(!page.issues.is_empty());
    assert_eq!(fs::read(marker).unwrap(), b"do not touch");
    assert_eq!(
        fs::read(moved.join(first.capture.copy_name())).unwrap(),
        r.text.as_bytes()
    );
    worker.shutdown().unwrap();
}
#[test]
fn actual_physical_database_restore_reimports_the_qualified_mirror_with_exact_time() {
    let f = Fixture::new();
    let mut worker = f.worker();
    let r = f.request("\u{feff}Retained after old backup\r\nÕun");
    let first = capture(&worker, &r);
    let copy = first.capture.copy.directory.join(first.capture.copy_name());
    let receipt = first
        .capture
        .copy
        .directory
        .join(format!(".brn-inbox-{}.receipt", r.id));
    let mirror = fs::read(&receipt).unwrap();
    worker.shutdown().unwrap();
    fs::write(f.data.join("brn.sqlite"), b"synthetic physical corruption").unwrap();
    let mut worker = f.worker();
    assert_eq!(read(&worker, r.id).item, first);
    assert_eq!(capture(&worker, &r), first);
    assert_eq!(fs::read(copy).unwrap(), r.text.as_bytes());
    assert_eq!(fs::read(receipt).unwrap(), mirror);
    worker.shutdown().unwrap();
}
#[test]
fn invalid_or_mismatched_envelope_refuses_before_creating_a_copy_namespace() {
    let f = Fixture::new();
    let mut worker = f.worker();
    let mut r = f.request("text");
    r.title = " \t".into();
    assert!(matches!(
        query(&worker, AppCommand::CaptureInbox(r)),
        AppEvent::Failed(_)
    ));
    let r = f.request("text");
    let outer = Uuid::new_v4();
    worker.submit(outer, AppCommand::CaptureInbox(r)).unwrap();
    let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(id, outer);
    assert!(matches!(event,AppEvent::Failed(e) if e.kind==ErrorKind::OperationConflict));
    assert!(!f.data.join("inbox").exists());
    worker.shutdown().unwrap();
}

#[test]
fn inbox_uncertainty_never_promotes_intake_or_fences_unrelated_current_knowledge() {
    use brn_workflow::library::KnowledgeScope;
    let f = Fixture::new();
    let vault = f._owner.path().join("vault");
    fs::create_dir(&vault).unwrap();
    fs::write(vault.join("approved.md"), "Approved current sentinel λ\r\n").unwrap();
    let mut worker = f.worker();
    assert!(matches!(
        query(&worker, AppCommand::BindVault(vault.clone())),
        AppEvent::VaultBound
    ));
    let r = f.request("Raw unapproved secret sentinel");
    let item = capture(&worker, &r);
    let assert_current = |worker: &AppWorker| {
        let AppEvent::Notes(page) = query(
            worker,
            AppCommand::ScopedNotes {
                scope: KnowledgeScope::All,
                folder: None,
                cursor: None,
            },
        ) else {
            panic!("scope inventory")
        };
        assert_eq!(page.notes.len(), 1);
        assert_eq!(page.notes[0].path, "approved.md");
        let AppEvent::Note(note) = query(
            worker,
            AppCommand::ScopedNote {
                scope: KnowledgeScope::Current,
                path: "approved.md".into(),
            },
        ) else {
            panic!("current note")
        };
        assert_eq!(note.text, "Approved current sentinel λ\r\n");
    };
    assert_current(&worker);
    let root = f.data.join("inbox");
    fs::rename(&root, f.data.join("retained-inbox")).unwrap();
    fs::create_dir(&root).unwrap();
    assert!(matches!(
        read(&worker, r.id).original,
        InboxOriginal::Unavailable { .. }
    ));
    assert_current(&worker);
    assert_eq!(capture(&worker, &r), item);
    worker.shutdown().unwrap();
    let worker = AppWorker::start(
        f.data.clone(),
        AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: Some(f.credentials.clone()),
            model_dir: None,
        },
    )
    .unwrap();
    let mut worker = worker;
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(10))
            .unwrap()
            .1,
        AppEvent::Ready {
            vault_bound: true,
            ..
        }
    ));
    assert_current(&worker);
    assert_eq!(
        fs::read(vault.join("approved.md")).unwrap(),
        b"Approved current sentinel \xce\xbb\r\n"
    );
    worker.shutdown().unwrap();
}
