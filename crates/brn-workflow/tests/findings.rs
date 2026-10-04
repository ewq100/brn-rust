#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind,
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    findings::*,
};
use std::{fs, os::unix::fs::MetadataExt, path::PathBuf, time::Duration};
use uuid::Uuid;

const NOTE_ID: &str = "11111111-1111-4111-8111-111111111111";
const PATH: &str = "archive/õ-日本.md";
struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
    text: String,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        let credentials = owner.path().join("credentials");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::create_dir(vault.join("archive")).unwrap();
        let text = format!(
            "\u{feff}---\r\nbrn_id: {NOTE_ID}\r\nbrn_kind: source\r\n---\r\n# Õ 🦀\r\n[puudu][õ]\r\n\r\n[õ]: ../missing.md\r\n"
        );
        fs::write(vault.join(PATH), &text).unwrap();
        Self {
            _owner: owner,
            data,
            vault,
            credentials,
            text,
        }
    }
    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: Some(self.vault.clone()),
            credentials_dir: Some(self.credentials.clone()),
            model_dir: None,
        }
    }
    fn app(&self) -> App {
        App::open(&self.data, self.config()).unwrap()
    }
    fn link_request(&self, app: &App) -> CaptureFindingRequest {
        let links = app.note_links(PATH).unwrap();
        let link = &links.links[0];
        CaptureFindingRequest {
            id: Uuid::new_v4(),
            origin: FindingOrigin::UnresolvedLink {
                path: PATH.into(),
                source_sha256: links.source.sha256,
                destination: link.destination.clone(),
                start_byte: link.evidence[0].start_byte,
            },
        }
    }
    fn unchanged(&self, app: &App) {
        assert_eq!(
            fs::read(self.vault.join(PATH)).unwrap(),
            self.text.as_bytes()
        );
        assert!(app.work_store().proposals(None).unwrap().is_empty());
        assert!(app.work_store().editors().unwrap().is_empty());
        assert_eq!(fs::read_dir(&self.credentials).unwrap().count(), 0);
    }
}

#[test]
fn reference_link_capture_retains_complete_exact_saved_proofs_and_restart_lifecycle() {
    let f = Fixture::new();
    let mut app = f.app();
    let request = f.link_request(&app);
    let record = app.capture_finding(&request).unwrap();
    assert_eq!(record.version, 1);
    assert_eq!(record.state, FindingState::Open);
    assert_eq!(record.draft.request, request);
    assert_eq!(record.draft.evidence.len(), 2);
    for proof in &record.draft.evidence {
        assert_eq!(proof.source.path, PATH);
        assert_eq!(
            proof.source.fingerprint.inode,
            fs::metadata(f.vault.join(PATH)).unwrap().ino()
        );
        let quote = proof.quote.as_ref().unwrap();
        assert_eq!(
            f.text.get(quote.start_byte..quote.end_byte),
            Some(quote.quote.as_str())
        );
        assert_eq!(proof.note_id, Some(NOTE_ID.parse().unwrap()));
    }
    assert!(
        record.draft.evidence[1]
            .quote
            .as_ref()
            .unwrap()
            .quote
            .starts_with("[õ]:")
    );
    assert_eq!(app.capture_finding(&request).unwrap(), record);
    assert_eq!(
        app.findings(&FindingListRequest::default())
            .unwrap()
            .open_count,
        1
    );
    f.unchanged(&app);
    drop(app);
    let mut app = f.app();
    assert_eq!(app.finding(request.id).unwrap(), record);
    let close = CloseFindingRequest {
        expected: record.stamp(),
        state: FindingState::Resolved,
    };
    let closed = app.close_finding(&close).unwrap();
    assert_eq!(closed.version, 2);
    assert_eq!(app.close_finding(&close).unwrap(), closed);
    assert_eq!(app.capture_finding(&request).unwrap(), closed);
    assert_eq!(
        app.close_finding(&CloseFindingRequest {
            state: FindingState::Dismissed,
            ..close
        })
        .unwrap_err()
        .kind,
        ErrorKind::ContextStale
    );
    assert_eq!(
        app.findings(&FindingListRequest::default())
            .unwrap()
            .open_count,
        0
    );
    f.unchanged(&app);
}

#[test]
fn duplicate_identity_capture_requires_two_fresh_distinct_saved_paths() {
    let f = Fixture::new();
    let mut app = f.app();
    let request = CaptureFindingRequest {
        id: Uuid::new_v4(),
        origin: FindingOrigin::IdentityAmbiguity {
            note_id: NOTE_ID.parse().unwrap(),
        },
    };
    assert_eq!(
        app.capture_finding(&request).unwrap_err().kind,
        ErrorKind::ToolRejected
    );
    assert!(
        app.findings(&FindingListRequest::default())
            .unwrap()
            .entries
            .is_empty()
    );
    fs::copy(f.vault.join(PATH), f.vault.join("duplicate.md")).unwrap();
    let record = app.capture_finding(&request).unwrap();
    assert_eq!(record.draft.evidence.len(), 2);
    assert_ne!(
        record.draft.evidence[0].source.path,
        record.draft.evidence[1].source.path
    );
    for proof in &record.draft.evidence {
        assert_eq!(proof.note_id, Some(NOTE_ID.parse().unwrap()));
        let text = fs::read_to_string(f.vault.join(&proof.source.path)).unwrap();
        let quote = proof.quote.as_ref().unwrap();
        assert_eq!(
            text.get(quote.start_byte..quote.end_byte),
            Some(quote.quote.as_str())
        );
    }
    f.unchanged(&app);
}

#[test]
fn identity_excerpt_is_bounded_on_actual_utf8_boundaries_without_clipping_full_proof() {
    let mut f = Fixture::new();
    f.text.push_str(&"日本語🦀\r\n".repeat(3_000));
    fs::write(f.vault.join(PATH), &f.text).unwrap();
    fs::copy(f.vault.join(PATH), f.vault.join("duplicate.md")).unwrap();
    let mut app = f.app();
    let record = app
        .capture_finding(&CaptureFindingRequest {
            id: Uuid::new_v4(),
            origin: FindingOrigin::IdentityAmbiguity {
                note_id: NOTE_ID.parse().unwrap(),
            },
        })
        .unwrap();
    for proof in &record.draft.evidence {
        let quote = proof.quote.as_ref().unwrap();
        assert_eq!(quote.start_byte, 0);
        assert!((16 * 1024 - 4..=16 * 1024).contains(&quote.end_byte));
        assert_eq!(f.text.get(..quote.end_byte), Some(quote.quote.as_str()));
        assert_eq!(proof.source.fingerprint.len, f.text.len() as u64);
        assert!(proof.source.fingerprint.len > quote.end_byte as u64);
    }
    f.unchanged(&app);
}

#[test]
fn drift_inspection_preserves_original_quotes_and_full_inode_proof_without_reanchoring() {
    let f = Fixture::new();
    let mut app = f.app();
    let request = f.link_request(&app);
    let record = app.capture_finding(&request).unwrap();
    let inspect = app.inspect_finding(request.id).unwrap();
    assert!(
        inspect
            .evidence
            .iter()
            .all(|p| p.outcome == FindingEvidenceOutcome::Unchanged)
    );
    fs::write(f.vault.join("replacement.tmp"), &f.text).unwrap();
    fs::rename(f.vault.join("replacement.tmp"), f.vault.join(PATH)).unwrap();
    let inspect = app.inspect_finding(request.id).unwrap();
    assert_eq!(inspect.record, record);
    for proof in &inspect.evidence {
        assert_eq!(proof.outcome, FindingEvidenceOutcome::Changed);
        assert_eq!(
            proof.observed.as_ref().unwrap().fingerprint.sha256,
            record.draft.evidence[proof.index].source.fingerprint.sha256
        );
        assert_ne!(
            proof.observed.as_ref().unwrap().fingerprint.inode,
            record.draft.evidence[proof.index].source.fingerprint.inode
        );
    }
    fs::remove_file(f.vault.join(PATH)).unwrap();
    assert!(
        app.inspect_finding(request.id)
            .unwrap()
            .evidence
            .iter()
            .all(|p| p.outcome == FindingEvidenceOutcome::Unavailable
                && p.observed.is_none()
                && p.reason.is_some())
    );
    assert_eq!(app.finding(request.id).unwrap(), record);
    assert_eq!(app.capture_finding(&request).unwrap(), record);
}

#[test]
fn stale_or_non_issue_origins_cannot_create_work_and_payload_conflict_precedes_missing_vault() {
    let f = Fixture::new();
    let mut app = f.app();
    let request = f.link_request(&app);
    let mut invalid = request.clone();
    if let FindingOrigin::UnresolvedLink { start_byte, .. } = &mut invalid.origin {
        *start_byte += 1;
    }
    assert_eq!(
        app.capture_finding(&invalid).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    fs::write(
        f.vault.join("missing.md"),
        "---\nbrn_id: 22222222-2222-4222-8222-222222222222\n---\nPresent\n",
    )
    .unwrap();
    assert_eq!(
        app.capture_finding(&request).unwrap_err().kind,
        ErrorKind::ToolRejected
    );
    fs::remove_file(f.vault.join("missing.md")).unwrap();
    let record = app.capture_finding(&request).unwrap();
    drop(app);
    fs::rename(&f.vault, f.vault.with_extension("unavailable")).unwrap();
    let mut app = f.app();
    assert_eq!(app.capture_finding(&request).unwrap(), record);
    assert_eq!(
        app.capture_finding(&invalid).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    let inspect = app.inspect_finding(request.id).unwrap();
    assert_eq!(inspect.record, record);
    assert!(
        inspect
            .evidence
            .iter()
            .all(|p| p.outcome == FindingEvidenceOutcome::Unavailable)
    );
    let closed = app
        .close_finding(&CloseFindingRequest {
            expected: record.stamp(),
            state: FindingState::Dismissed,
        })
        .unwrap();
    assert_eq!(closed.state, FindingState::Dismissed);
    assert_eq!(
        app.finding(Uuid::new_v4()).unwrap_err().kind,
        ErrorKind::NotFound
    );
}

#[test]
fn worker_correlates_real_capture_and_drains_admitted_creation_and_closure() {
    let f = Fixture::new();
    let app = f.app();
    let request = f.link_request(&app);
    drop(app);
    let mut worker = AppWorker::start(f.data.clone(), f.config()).unwrap();
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(10))
            .unwrap()
            .1,
        AppEvent::Ready { .. }
    ));
    let capture_id = Uuid::new_v4();
    worker
        .submit(capture_id, AppCommand::CaptureFinding(request.clone()))
        .unwrap();
    worker.shutdown().unwrap();
    let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(id, capture_id);
    let AppEvent::Finding(record) = event else {
        panic!("capture must settle during shutdown")
    };
    drop(worker);
    let mut worker = AppWorker::start(f.data.clone(), f.config()).unwrap();
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(10))
            .unwrap()
            .1,
        AppEvent::Ready { .. }
    ));
    let close_id = Uuid::new_v4();
    worker
        .submit(
            close_id,
            AppCommand::CloseFinding(CloseFindingRequest {
                expected: record.stamp(),
                state: FindingState::Dismissed,
            }),
        )
        .unwrap();
    worker.shutdown().unwrap();
    let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(id, close_id);
    assert!(
        matches!(event, AppEvent::Finding(record) if record.version == 2 && record.state == FindingState::Dismissed)
    );
    drop(worker);
    let app = f.app();
    assert_eq!(
        app.finding(request.id).unwrap().state,
        FindingState::Dismissed
    );
    f.unchanged(&app);
}

#[test]
fn evidence_fence_blocks_new_capture_and_fresh_inspection_but_retains_history_and_closure() {
    use brn_workflow::{
        proposal_apply::ApprovalRequest,
        proposals::{DraftNoteChange, DraftRequest},
    };
    let f = Fixture::new();
    let mut app = f.app();
    let request = f.link_request(&app);
    let record = app.capture_finding(&request).unwrap();
    let saved = app.proposal_source("archive/õ-日本.md").unwrap_err();
    // Archive paths remain evidence-only, rather than replacement destinations.
    assert_eq!(saved.kind, ErrorKind::ToolRejected);
    fs::write(f.vault.join("current.md"), "Saved current\r\n").unwrap();
    let saved = app.proposal_source("current.md").unwrap();
    let proposal = app
        .create_proposal(&DraftRequest {
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Synthetic pending proof".into(),
            changes: vec![DraftNoteChange::Replace {
                path: "current.md".into(),
                expected: saved.source.fingerprint,
                text: "Proposed\n".into(),
            }],
            sources: vec![],
        })
        .unwrap();
    app.work_store_mut()
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: proposal.stamp(),
        })
        .unwrap();
    let fresh = CaptureFindingRequest {
        id: Uuid::new_v4(),
        ..request.clone()
    };
    assert_eq!(
        app.capture_finding(&fresh).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(app.capture_finding(&request).unwrap(), record);
    assert_eq!(
        app.findings(&FindingListRequest::default())
            .unwrap()
            .entries,
        vec![record.clone()]
    );
    let inspected = app.inspect_finding(request.id).unwrap();
    assert_eq!(inspected.record, record);
    assert!(
        inspected
            .evidence
            .iter()
            .all(|p| p.outcome == FindingEvidenceOutcome::Unavailable
                && p.reason.as_ref().unwrap().contains("reconcile"))
    );
    app.close_finding(&CloseFindingRequest {
        expected: record.stamp(),
        state: FindingState::Dismissed,
    })
    .unwrap();
    assert_eq!(fs::read(f.vault.join(PATH)).unwrap(), f.text.as_bytes());
    assert_eq!(
        fs::read(f.vault.join("current.md")).unwrap(),
        b"Saved current\r\n"
    );
}

#[test]
fn foreign_bound_vault_is_not_inspected_and_stale_saved_hash_cannot_admit_new_capture() {
    let f = Fixture::new();
    let mut app = f.app();
    let request = f.link_request(&app);
    let record = app.capture_finding(&request).unwrap();
    let other = Fixture::new();
    let mut other_app = other.app();
    let other_request = other.link_request(&other_app);
    let foreign = other_app.capture_finding(&other_request).unwrap();
    // A retained record can outlive a selection; inspection never reads an old
    // vault or treats the same relative path in this vault as its replacement.
    app.work_store_mut().create_finding(&foreign.draft).unwrap();
    let inspected = app.inspect_finding(other_request.id).unwrap();
    assert_eq!(inspected.record.draft, foreign.draft);
    assert!(
        inspected
            .evidence
            .iter()
            .all(|p| p.outcome == FindingEvidenceOutcome::Unavailable
                && p.observed.is_none()
                && p.reason.as_ref().unwrap().contains("another vault"))
    );
    fs::write(f.vault.join(PATH), format!("{}Later õ\r\n", f.text)).unwrap();
    let fresh = CaptureFindingRequest {
        id: Uuid::new_v4(),
        ..request.clone()
    };
    assert_eq!(
        app.capture_finding(&fresh).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    assert_eq!(app.capture_finding(&request).unwrap(), record);
    assert_eq!(app.inspect_finding(request.id).unwrap().record, record);
    assert_eq!(
        app.findings(&FindingListRequest::default())
            .unwrap()
            .open_count,
        2
    );
    assert!(app.work_store().proposals(None).unwrap().is_empty());
}
