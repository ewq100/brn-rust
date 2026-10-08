#![cfg(target_os = "macos")]

use brn_workflow::{
    ErrorKind, MAX_NOTE_BYTES,
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    proposal_apply::ApprovalRequest,
    proposals::{DraftNoteChange, DraftRequest, NoteChange, ProposalSource, SourceVersion},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{MetadataExt, symlink},
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

const PATH: &str = "正文-日本語.md";
const ORIGINAL: &str = "\u{feff}---\r\ncustom: õ🦀\r\n---\r\n正文 λ\nlast\r";
const LATER: &str = "\u{feff}Later Ελληνικά 日本語\r\nlast\r";

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
            next(&worker).1,
            AppEvent::Ready {
                vault_bound: true,
                model_installed: false
            }
        ));
        worker
    }
}

fn next(worker: &AppWorker) -> (Uuid, AppEvent) {
    worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
}

fn request(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    let (actual, event) = next(worker);
    assert_eq!(actual, id, "source queries retain their exact request UUID");
    event
}

fn source(worker: &AppWorker) -> ProposalSource {
    match request(worker, AppCommand::ProposalSource(PATH.into())) {
        AppEvent::ProposalSource(value) => *value,
        AppEvent::Failed(error) => panic!("source query failed: {error}"),
        _ => panic!("expected full source capture"),
    }
}

fn assert_no_review_work(worker: &AppWorker) {
    assert!(
        matches!(request(worker, AppCommand::Editors), AppEvent::Editors(records) if records.is_empty())
    );
    assert!(
        matches!(request(worker, AppCommand::Proposals(None)), AppEvent::Proposals(records) if records.is_empty())
    );
}

type VaultEntry = (PathBuf, &'static str, Vec<u8>, u64, u64);

fn snapshot(root: &Path) -> Vec<VaultEntry> {
    fn visit(root: &Path, path: &Path, entries: &mut Vec<VaultEntry>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let metadata = entry.path().symlink_metadata().unwrap();
            let kind = metadata.file_type();
            let (name, bytes) = if kind.is_symlink() {
                (
                    "symlink",
                    fs::read_link(entry.path())
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .as_bytes()
                        .to_vec(),
                )
            } else if kind.is_dir() {
                visit(root, &entry.path(), entries);
                ("directory", vec![])
            } else {
                assert!(kind.is_file());
                ("file", fs::read(entry.path()).unwrap())
            };
            entries.push((
                entry.path().strip_prefix(root).unwrap().to_owned(),
                name,
                bytes,
                metadata.dev(),
                metadata.ino(),
            ));
        }
    }
    let mut entries = vec![];
    visit(root, root, &mut entries);
    entries.sort();
    entries
}

fn draft(source: &SourceVersion, change: DraftNoteChange) -> DraftRequest {
    DraftRequest {
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        action_changes: Vec::new(),
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Complete synthetic proposal 日本語\r\n".into(),
        changes: vec![change],
        sources: vec![source.clone()],
    }
}

#[test]
fn source_returns_exact_full_bytes_and_identity_without_creating_review_or_editor_work() {
    // More than the ordinary read tool's 50,000-byte excerpt, still a valid note.
    let mut full = format!("{ORIGINAL}{}", "正文 日本語 λ\r\n".repeat(12_000));
    full.push_str(&"x".repeat(MAX_NOTE_BYTES - full.len()));
    assert_eq!(full.len(), MAX_NOTE_BYTES);
    let fixture = Fixture::new(&full);
    let mut worker = fixture.worker();
    assert_no_review_work(&worker);
    let original_files = snapshot(&fixture.vault);
    let captured = source(&worker);
    assert_eq!(captured.source.path, PATH);
    assert_eq!(captured.text.as_bytes(), full.as_bytes());
    let metadata = fs::metadata(fixture.vault.join(PATH)).unwrap();
    assert_eq!(captured.source.fingerprint.device, metadata.dev());
    assert_eq!(captured.source.fingerprint.inode, metadata.ino());
    assert_eq!(captured.source.fingerprint.len, full.len() as u64);
    assert_eq!(
        captured.source.fingerprint.sha256,
        <[u8; 32]>::from(Sha256::digest(full.as_bytes()))
    );
    assert_eq!(source(&worker), captured);
    assert_eq!(snapshot(&fixture.vault), original_files);
    assert_no_review_work(&worker);

    let encoded = serde_json::to_value(&captured).unwrap();
    assert_eq!(
        serde_json::from_value::<ProposalSource>(encoded.clone()).unwrap(),
        captured
    );
    let mut extra = encoded.clone();
    extra
        .as_object_mut()
        .unwrap()
        .insert("ignored".into(), true.into());
    assert!(serde_json::from_value::<ProposalSource>(extra).is_err());
    let mut extra = encoded;
    extra["source"]
        .as_object_mut()
        .unwrap()
        .insert("ignored".into(), true.into());
    assert!(serde_json::from_value::<ProposalSource>(extra).is_err());
    worker.shutdown().unwrap();
    let mut worker = fixture.worker();
    assert_eq!(source(&worker), captured);
    assert_eq!(snapshot(&fixture.vault), original_files);
    assert_no_review_work(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn fresh_reads_capture_changed_bytes_and_old_replace_trash_or_source_bindings_refuse() {
    let fixture = Fixture::new(ORIGINAL);
    let mut worker = fixture.worker();
    let old = source(&worker);
    fs::write(fixture.vault.join(PATH), LATER).unwrap();
    let fresh = source(&worker);
    assert_eq!(fresh.text.as_bytes(), LATER.as_bytes());
    assert_ne!(fresh.source.fingerprint, old.source.fingerprint);
    let changed = snapshot(&fixture.vault);
    for change in [
        DraftNoteChange::Replace {
            path: PATH.into(),
            expected: old.source.fingerprint.clone(),
            text: "Never install this".into(),
        },
        DraftNoteChange::Trash {
            path: PATH.into(),
            expected: old.source.fingerprint.clone(),
        },
        DraftNoteChange::Create {
            path: "new.md".into(),
            text: "Old source must not be adopted".into(),
        },
    ] {
        let AppEvent::Failed(error) = request(
            &worker,
            AppCommand::CreateProposal(draft(&old.source, change)),
        ) else {
            panic!("stale capture must refuse atomically");
        };
        assert_eq!(error.kind, ErrorKind::ContextStale);
        assert_eq!(snapshot(&fixture.vault), changed);
        assert_no_review_work(&worker);
    }

    // Same bytes in a new object are a different source/baseline identity.
    fs::write(fixture.vault.join("replacement.tmp"), LATER).unwrap();
    fs::rename(
        fixture.vault.join("replacement.tmp"),
        fixture.vault.join(PATH),
    )
    .unwrap();
    let replacement = source(&worker);
    assert_eq!(replacement.text, fresh.text);
    assert_eq!(
        replacement.source.fingerprint.sha256,
        fresh.source.fingerprint.sha256
    );
    assert_ne!(
        replacement.source.fingerprint.inode,
        fresh.source.fingerprint.inode
    );
    let AppEvent::Failed(error) = request(
        &worker,
        AppCommand::CreateProposal(draft(
            &fresh.source,
            DraftNoteChange::Replace {
                path: PATH.into(),
                expected: fresh.source.fingerprint.clone(),
                text: "Never adopt another identity".into(),
            },
        )),
    ) else {
        panic!("replacement identity must refuse");
    };
    assert_eq!(error.kind, ErrorKind::ContextStale);
    assert_no_review_work(&worker);

    let creation = draft(
        &replacement.source,
        DraftNoteChange::Replace {
            path: PATH.into(),
            expected: replacement.source.fingerprint.clone(),
            text: "\u{feff}Proposed only 日本語\r\n".into(),
        },
    );
    let current_files = snapshot(&fixture.vault);
    let AppEvent::Proposal(record) = request(&worker, AppCommand::CreateProposal(creation)) else {
        panic!("fresh capture creates review work");
    };
    assert!(
        matches!(&record.draft.changes[0], NoteChange::Replace {before, before_text, ..} if before == &replacement.source.fingerprint && before_text == LATER)
    );
    assert_eq!(record.draft.sources, vec![replacement.source]);
    assert_eq!(snapshot(&fixture.vault), current_files);
    assert!(
        matches!(request(&worker, AppCommand::Editors), AppEvent::Editors(records) if records.is_empty())
    );
    worker.shutdown().unwrap();
}

#[test]
fn invalid_hidden_and_unsupported_sources_refuse_without_mutating_files_or_review_work() {
    let fixture = Fixture::new(ORIGINAL);
    fs::write(fixture.vault.join("plain.txt"), "not Markdown").unwrap();
    fs::write(fixture.vault.join(".hidden.md"), "hidden").unwrap();
    fs::create_dir(fixture.vault.join(".internal")).unwrap();
    fs::write(fixture.vault.join(".internal/note.md"), "hidden parent").unwrap();
    fs::write(
        fixture.vault.join("oversized.md"),
        "x".repeat(MAX_NOTE_BYTES + 1),
    )
    .unwrap();
    fs::write(fixture.vault.join("invalid-utf8.md"), [0xff, 0xfe, 0xf0]).unwrap();
    fs::create_dir(fixture.vault.join("directory.md")).unwrap();
    symlink(PATH, fixture.vault.join("symlink.md")).unwrap();
    let mut worker = fixture.worker();
    let original_files = snapshot(&fixture.vault);
    for path in [
        "",
        "../outside.md",
        "/absolute.md",
        "nested/../../outside.md",
        "plain.txt",
        ".hidden.md",
        ".internal/note.md",
        ".brn-stage.md",
        "bad\0.md",
        "oversized.md",
        "invalid-utf8.md",
        "directory.md",
        "symlink.md",
    ] {
        let AppEvent::Failed(error) = request(&worker, AppCommand::ProposalSource(path.into()))
        else {
            panic!("unsupported source {path:?} must refuse");
        };
        assert_eq!(error.kind, ErrorKind::ToolRejected, "{path:?}: {error}");
        assert_eq!(snapshot(&fixture.vault), original_files);
        assert_no_review_work(&worker);
    }
    let AppEvent::Failed(error) = request(&worker, AppCommand::ProposalSource("missing.md".into()))
    else {
        panic!("missing source must refuse");
    };
    assert_eq!(error.kind, ErrorKind::ContextStale);
    assert_eq!(source(&worker).text, ORIGINAL);
    assert_eq!(snapshot(&fixture.vault), original_files);
    assert_no_review_work(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn unresolved_application_fences_source_capture_and_preserves_pending_review_and_files() {
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
    let captured = app.proposal_source(PATH).unwrap();
    let review = app
        .create_proposal(&draft(
            &captured.source,
            DraftNoteChange::Create {
                path: "pending.md".into(),
                text: "Unapproved review only 日本語\r\n".into(),
            },
        ))
        .unwrap();
    // Public operational admission only; no staging or installation occurs.
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    let pending = app
        .work_store_mut()
        .begin_proposal_apply(&approval)
        .unwrap();
    let live = app.proposal(review.draft.id).unwrap();
    let unchanged = snapshot(&fixture.vault);
    let error = app.proposal_source(PATH).unwrap_err();
    assert_eq!(error.kind, ErrorKind::SaveUncertain);
    assert_eq!(snapshot(&fixture.vault), unchanged);
    assert_eq!(app.proposal(review.draft.id).unwrap(), live);
    assert_eq!(
        app.work_store()
            .proposal_apply(approval.operation_id)
            .unwrap(),
        Some(pending)
    );
    assert!(app.work_store().editors().unwrap().is_empty());
}
