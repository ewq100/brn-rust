#![cfg(target_os = "macos")]
use brn_store::note_provenance;
use brn_workflow::{
    ErrorKind,
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    knowledge::{
        CitationCapture, CitationOutcome, CitationRequest, NoteProvenance, ProvenanceRequest,
        VaultCitation,
    },
    proposal_apply::{ApplyOutcome, ApprovalRequest, UndoRequest},
    proposals::{DraftNoteChange, DraftRequest, ProposalEdit, ProposalRecord},
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

const TARGET: &str = "current.md";
const SOURCE: &str = "archive/source.md";
const SOURCE_ID: &str = "99999999-1111-4222-8333-444444444444";
const ORIGINAL: &str = "\u{feff}---\r\nbrn_id: aaaaaaaa-1111-4222-8333-444444444444\r\ncustom: 'λ õ'\r\n---\r\n# Knowledge\r\nCurrent interpretation\nlast\r";
const QUOTE: &str = "Anna ütles: õäöü\r\n原文 🦀\n";

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
    source: String,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        assert!(!owner.path().starts_with(env!("CARGO_MANIFEST_DIR")));
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        let credentials = owner.path().join("credentials");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::create_dir(vault.join("archive")).unwrap();
        let source = format!(
            "\u{feff}---\r\nbrn_id: {SOURCE_ID}\r\nbrn_kind: source\r\ncustom: [opaque, values]\r\n---\r\n{QUOTE}end\r"
        );
        fs::write(vault.join(TARGET), ORIGINAL).unwrap();
        fs::write(vault.join(SOURCE), &source).unwrap();
        Self {
            _owner: owner,
            data,
            vault,
            credentials,
            source,
        }
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(self.data.clone(), self.config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(20))
                .unwrap()
                .1,
            AppEvent::Ready {
                vault_bound: true,
                model_installed: false
            }
        ));
        worker
    }
    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: Some(self.vault.clone()),
            credentials_dir: Some(self.credentials.clone()),
            model_dir: None,
        }
    }
    fn request(&self) -> CitationRequest {
        let start_byte = self.source.find(QUOTE).unwrap();
        CitationRequest {
            note_id: Uuid::parse_str(SOURCE_ID).unwrap(),
            expected_sha256: Sha256::digest(self.source.as_bytes()).into(),
            start_byte,
            end_byte: start_byte + QUOTE.len(),
        }
    }
}
fn reply(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    let (actual, event) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
    assert_eq!(actual, id);
    event
}
fn capture(worker: &AppWorker, fixture: &Fixture) -> CitationCapture {
    match reply(worker, AppCommand::CaptureCitation(fixture.request())) {
        AppEvent::CitationCaptured(value) => *value,
        AppEvent::Failed(error) => panic!("capture: {error}"),
        _ => panic!("capture response"),
    }
}
fn request(citation: VaultCitation) -> ProvenanceRequest {
    ProvenanceRequest {
        path: TARGET.into(),
        proposal_id: Uuid::new_v4(),
        title: "Retain exact source provenance õ\r\n".into(),
        citations: vec![citation],
    }
}
fn prepare(worker: &AppWorker, request: ProvenanceRequest) -> DraftRequest {
    match reply(worker, AppCommand::PrepareNoteProvenance(request)) {
        AppEvent::NoteProvenanceDraft(value) => *value,
        AppEvent::Failed(error) => panic!("prepare: {error}"),
        _ => panic!("draft response"),
    }
}
fn create(worker: &AppWorker, draft: DraftRequest) -> ProposalRecord {
    match reply(worker, AppCommand::CreateProposal(draft)) {
        AppEvent::Proposal(record) => record,
        AppEvent::Failed(error) => panic!("create: {error}"),
        _ => panic!("proposal response"),
    }
}
fn approve(worker: &AppWorker, record: &ProposalRecord) -> ApprovalRequest {
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    assert!(
        matches!(reply(worker, AppCommand::ApproveProposal(approval.clone())),
        AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    approval
}
fn inspect(worker: &AppWorker) -> NoteProvenance {
    match reply(worker, AppCommand::NoteProvenance(TARGET.into())) {
        AppEvent::NoteProvenance(value) => *value,
        AppEvent::Failed(error) => panic!("inspect: {error}"),
        _ => panic!("inspection response"),
    }
}

#[test]
fn archived_exact_provenance_requires_approval_and_survives_restart_index_loss_and_new_store() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let captured = capture(&worker, &fixture);
    assert_eq!(captured.citation.quote.as_bytes(), QUOTE.as_bytes());
    assert_eq!(captured.source.path, SOURCE);
    let input = request(captured.citation.clone());
    let draft = prepare(&worker, input.clone());
    assert_eq!(draft, prepare(&worker, input));
    assert_eq!(draft.sources.len(), 2);
    assert!(draft.sources.contains(&captured.source));
    let expected =
        note_provenance::write(ORIGINAL, std::slice::from_ref(&captured.citation)).unwrap();
    assert!(matches!(&draft.changes[0], DraftNoteChange::Replace {text, ..} if text == &expected));
    assert_eq!(
        fs::read(fixture.vault.join(TARGET)).unwrap(),
        ORIGINAL.as_bytes()
    );
    assert!(
        matches!(reply(&worker, AppCommand::Editors), AppEvent::Editors(records) if records.is_empty())
    );
    assert!(
        matches!(reply(&worker, AppCommand::Proposals(None)), AppEvent::Proposals(records) if records.is_empty())
    );
    let record = create(&worker, draft);
    assert_eq!(
        fs::read(fixture.vault.join(TARGET)).unwrap(),
        ORIGINAL.as_bytes()
    );
    let approval = approve(&worker, &record);
    assert_eq!(
        fs::read(fixture.vault.join(TARGET)).unwrap(),
        expected.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault.join(SOURCE)).unwrap(),
        fixture.source.as_bytes()
    );
    let shown = inspect(&worker);
    assert_eq!(shown.citations[0].citation, captured.citation);
    assert_eq!(shown.citations[0].outcome, CitationOutcome::Matched);
    worker.shutdown().unwrap();
    fs::remove_file(fixture.data.join("index.sqlite")).unwrap();
    let mut worker = fixture.worker();
    assert_eq!(inspect(&worker), shown);
    worker.shutdown().unwrap();
    // No prior operational state is needed to retain or inspect approved quotes.
    // Stage 13 will qualify actual session Delete separately.
    let fresh_data = fixture._owner.path().join("independent-data");
    fs::create_dir(&fresh_data).unwrap();
    let fresh_vault = fixture._owner.path().join("independent-vault");
    fs::create_dir(&fresh_vault).unwrap();
    fs::create_dir(fresh_vault.join("archive")).unwrap();
    fs::write(fresh_vault.join(TARGET), &expected).unwrap();
    fs::write(fresh_vault.join(SOURCE), &fixture.source).unwrap();
    let fresh = App::open(
        &fresh_data,
        AppConfig {
            vault_root: Some(fresh_vault),
            ..fixture.config()
        },
    )
    .unwrap();
    assert!(fresh.work_store().conversations().unwrap().is_empty());
    assert!(fresh.work_store().proposals(None).unwrap().is_empty());
    assert_eq!(fresh.note_provenance(TARGET).unwrap(), shown);
    drop(fresh);
    let mut worker = fixture.worker();
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: approval.operation_id,
        trash_member: None,
    };
    assert!(
        matches!(reply(&worker, AppCommand::UndoProposal(undo)), AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    assert_eq!(
        fs::read(fixture.vault.join(TARGET)).unwrap(),
        ORIGINAL.as_bytes()
    );
    assert!(inspect(&worker).citations.is_empty());
    worker.shutdown().unwrap();
}

#[test]
fn provenance_resolves_moves_and_reports_changed_absent_ambiguous_and_incomplete_without_guessing()
{
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let citation = capture(&worker, &fixture).citation;
    let record = create(&worker, prepare(&worker, request(citation.clone())));
    approve(&worker, &record);
    let moved = "archive/renamed-õ.md";
    fs::rename(fixture.vault.join(SOURCE), fixture.vault.join(moved)).unwrap();
    assert_eq!(
        inspect(&worker).citations[0].outcome,
        CitationOutcome::Matched
    );
    assert_eq!(inspect(&worker).citations[0].matches[0].path, moved);
    fs::write(fixture.vault.join("duplicate.md"), &fixture.source).unwrap();
    assert_eq!(
        inspect(&worker).citations[0].outcome,
        CitationOutcome::Ambiguous
    );
    fs::remove_file(fixture.vault.join("duplicate.md")).unwrap();
    fs::write(
        fixture.vault.join(moved),
        fixture.source.replace("Anna", "Mari"),
    )
    .unwrap();
    let changed = inspect(&worker);
    assert_eq!(changed.citations[0].outcome, CitationOutcome::Changed);
    assert_eq!(changed.citations[0].citation, citation);
    fs::remove_file(fixture.vault.join(moved)).unwrap();
    assert_eq!(
        inspect(&worker).citations[0].outcome,
        CitationOutcome::Absent
    );
    fs::write(
        fixture.vault.join("unreadable-identity.md"),
        "---\nbrn_id: broken\n---\nsource",
    )
    .unwrap();
    let incomplete = inspect(&worker);
    assert_eq!(incomplete.citations[0].outcome, CitationOutcome::Incomplete);
    assert!(!incomplete.citations[0].issues.is_empty());
    assert_eq!(
        incomplete.citations[0].citation.quote.as_bytes(),
        QUOTE.as_bytes()
    );
    worker.shutdown().unwrap();
}

#[test]
fn capture_refuses_invalid_utf8_ranges_unknown_duplicates_and_changed_hashes_without_admission() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let original = fixture.request();
    for invalid in [
        CitationRequest {
            note_id: Uuid::nil(),
            ..original.clone()
        },
        CitationRequest {
            note_id: Uuid::new_v4(),
            ..original.clone()
        },
        CitationRequest {
            start_byte: fixture.source.find('õ').unwrap() + 1,
            ..original.clone()
        },
        CitationRequest {
            end_byte: fixture.source.len() + 1,
            ..original.clone()
        },
        CitationRequest {
            start_byte: original.end_byte,
            ..original.clone()
        },
    ] {
        assert!(
            matches!(reply(&worker, AppCommand::CaptureCitation(invalid)), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
        );
    }
    let wrong_hash = CitationRequest {
        expected_sha256: [0; 32],
        ..original.clone()
    };
    assert!(
        matches!(reply(&worker, AppCommand::CaptureCitation(wrong_hash)), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    fs::write(fixture.vault.join("duplicate.md"), &fixture.source).unwrap();
    assert!(
        matches!(reply(&worker, AppCommand::CaptureCitation(original)), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert!(
        matches!(reply(&worker, AppCommand::Editors), AppEvent::Editors(records) if records.is_empty())
    );
    assert!(
        matches!(reply(&worker, AppCommand::Proposals(None)), AppEvent::Proposals(records) if records.is_empty())
    );
    assert_eq!(
        fs::read(fixture.vault.join(TARGET)).unwrap(),
        ORIGINAL.as_bytes()
    );
    worker.shutdown().unwrap();
}

#[test]
fn source_cas_and_fresh_uuid_check_refuse_stale_preparation_creation_and_approval() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let captured = capture(&worker, &fixture);
    let input = request(captured.citation);
    let draft = prepare(&worker, input.clone());
    fs::write(
        fixture.vault.join(SOURCE),
        fixture.source.replace("Anna", "Mari"),
    )
    .unwrap();
    assert!(
        matches!(reply(&worker, AppCommand::PrepareNoteProvenance(input.clone())), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    assert!(
        matches!(reply(&worker, AppCommand::CreateProposal(draft)), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    fs::write(fixture.vault.join(SOURCE), &fixture.source).unwrap();
    let record = create(&worker, prepare(&worker, input));
    fs::write(fixture.vault.join("duplicate.md"), &fixture.source).unwrap();
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    assert!(
        matches!(reply(&worker, AppCommand::ApproveProposal(approval.clone())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert!(matches!(
        reply(&worker, AppCommand::ProposalApply(approval.operation_id)),
        AppEvent::ProposalApply(None)
    ));
    fs::remove_file(fixture.vault.join("duplicate.md")).unwrap();
    assert!(
        matches!(reply(&worker, AppCommand::ApproveProposal(approval)), AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    worker.shutdown().unwrap();
}

#[test]
fn pending_authoritative_changes_fence_capture_preparation_and_provenance_reads() {
    let fixture = Fixture::new();
    let mut app = App::open(&fixture.data, fixture.config()).unwrap();
    let captured = app.capture_citation(&fixture.request()).unwrap();
    let input = request(captured.citation);
    let draft = app.prepare_note_provenance(&input).unwrap();
    let record = app.create_proposal(&draft).unwrap();
    app.work_store_mut()
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        })
        .unwrap();
    assert_eq!(
        app.note_provenance(TARGET).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        app.capture_citation(&fixture.request()).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        app.prepare_note_provenance(&input).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        fs::read(fixture.vault.join(TARGET)).unwrap(),
        ORIGINAL.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault.join(SOURCE)).unwrap(),
        fixture.source.as_bytes()
    );
}

#[test]
fn approval_refuses_malformed_managed_provenance_but_preserves_unrelated_legacy_layouts() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let invalid = create(
        &worker,
        DraftRequest {
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Invalid managed source".into(),
            changes: vec![DraftNoteChange::Create {
                path: "invalid.md".into(),
                text: "---\nbrn_provenance: not-json\n---\nbody".into(),
            }],
            sources: vec![],
        },
    );
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: invalid.stamp(),
    };
    assert!(
        matches!(reply(&worker, AppCommand::ApproveProposal(approval.clone())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert!(matches!(
        reply(&worker, AppCommand::ProposalApply(approval.operation_id)),
        AppEvent::ProposalApply(None)
    ));
    assert!(!fixture.vault.join("invalid.md").exists());
    let legacy = "---\n{custom: [opaque, values]}\n---\nlegacy words\n";
    fs::write(fixture.vault.join("legacy.md"), legacy).unwrap();
    let AppEvent::ProposalSource(source) =
        reply(&worker, AppCommand::ProposalSource("legacy.md".into()))
    else {
        panic!()
    };
    let text = format!("{legacy}Explicit body edit\n");
    let record = create(
        &worker,
        DraftRequest {
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Preserve ordinary legacy metadata".into(),
            changes: vec![DraftNoteChange::Replace {
                path: "legacy.md".into(),
                expected: source.source.fingerprint,
                text: text.clone(),
            }],
            sources: vec![],
        },
    );
    approve(&worker, &record);
    assert_eq!(
        fs::read(fixture.vault.join("legacy.md")).unwrap(),
        text.as_bytes()
    );
    worker.shutdown().unwrap();
}

#[test]
fn malformed_saved_provenance_is_an_index_issue_and_refuses_all_scoped_ai_reads() {
    use brn_ai::{AiErrorKind, ReadScope, ReadTools};
    use brn_workflow::library::{KnowledgeScope, SearchMode};
    let fixture = Fixture::new();
    let invalid = "---\nbrn_provenance: not-json\n---\nunique-malformed-evidence\n";
    fs::write(fixture.vault.join("invalid.md"), invalid).unwrap();
    let mut app = App::open(&fixture.data, fixture.config()).unwrap();
    let report = app.refresh().unwrap();
    assert!(
        report
            .unreadable
            .iter()
            .any(|issue| issue.path == "invalid.md" && issue.reason == "invalid managed metadata")
    );
    for scope in [
        KnowledgeScope::Current,
        KnowledgeScope::Source,
        KnowledgeScope::History,
        KnowledgeScope::All,
    ] {
        assert!(
            app.search_scoped("unique-malformed-evidence", SearchMode::Keyword, 10, scope)
                .unwrap()
                .hits
                .is_empty()
        );
        assert_eq!(
            app.note_scoped("invalid.md", scope).unwrap_err().kind,
            ErrorKind::ToolRejected
        );
    }
    let tools = app.tools().unwrap();
    for scope in [
        ReadScope::Current,
        ReadScope::Source,
        ReadScope::History,
        ReadScope::All,
    ] {
        assert!(
            tools
                .search_notes_scoped("unique-malformed-evidence", 10, scope)
                .unwrap()
                .hits
                .is_empty()
        );
        assert_eq!(
            tools
                .read_note_scoped("invalid.md", scope)
                .unwrap_err()
                .kind,
            AiErrorKind::ToolRejected
        );
    }
    assert_eq!(app.evidence_note("invalid.md").unwrap().text, invalid);
    assert_eq!(
        fs::read(fixture.vault.join("invalid.md")).unwrap(),
        invalid.as_bytes()
    );
}

#[test]
fn edited_and_rewritten_quotes_require_exact_sources_and_cannot_forge_provenance() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let capture = capture(&worker, &fixture);
    let draft = prepare(&worker, request(capture.citation.clone()));
    let mut record = create(&worker, draft);
    let forged = VaultCitation {
        quote: "x".repeat(capture.citation.quote.len()),
        ..capture.citation.clone()
    };
    let bad_text = note_provenance::write(ORIGINAL, &[forged]).unwrap();
    let edit = ProposalEdit {
        action_data: Vec::new(),
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: vec![Some(bad_text)],
    };
    let AppEvent::Proposal(edited) = reply(&worker, AppCommand::EditProposal(edit)) else {
        panic!()
    };
    record = edited;
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    assert!(
        matches!(reply(&worker, AppCommand::ApproveProposal(approval)), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    let good_text =
        note_provenance::write(ORIGINAL, std::slice::from_ref(&capture.citation)).unwrap();
    let rewrite = ProposalEdit {
        action_data: Vec::new(),
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: vec![Some(good_text.clone())],
    };
    // AI Rewrite cannot repair or otherwise change provenance metadata. The
    // owner can explicitly repair it, after which body-only Rewrite remains valid.
    assert!(matches!(
        reply(&worker, AppCommand::RewriteProposal(rewrite.clone())),
        AppEvent::Failed(_)
    ));
    assert!(matches!(
        reply(&worker, AppCommand::Proposal(record.draft.id)),
        AppEvent::Proposal(unchanged) if unchanged == record
    ));
    let AppEvent::Proposal(record) = reply(&worker, AppCommand::EditProposal(rewrite)) else {
        panic!("explicit owner provenance repair")
    };
    let body_rewrite = ProposalEdit {
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: vec![Some(good_text.clone() + "\nRewritten body õ 🦀")],
        action_data: Vec::new(),
    };
    let AppEvent::Proposal(record) = reply(&worker, AppCommand::RewriteProposal(body_rewrite))
    else {
        panic!("body-only Rewrite preserves repaired provenance")
    };
    approve(&worker, &record);
    let unbound = DraftRequest {
        inbox_knowledge: None,
        inbox_source: None,
        action_changes: Vec::new(),
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "New source proof required".into(),
        changes: vec![DraftNoteChange::Create {
            path: "new.md".into(),
            text: good_text,
        }],
        sources: vec![],
    };
    let unbound = create(&worker, unbound);
    assert!(
        matches!(reply(&worker, AppCommand::ApproveProposal(ApprovalRequest {operation_id: Uuid::new_v4(), expected: unbound.stamp()})), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    assert!(!fixture.vault.join("new.md").exists());
    worker.shutdown().unwrap();
}

#[test]
fn historical_provenance_survives_body_edits_and_exact_undo_after_source_change() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let citation = capture(&worker, &fixture).citation;
    let record = create(&worker, prepare(&worker, request(citation.clone())));
    let first = approve(&worker, &record);
    fs::write(
        fixture.vault.join(SOURCE),
        fixture.source.replace("Anna", "Mari"),
    )
    .unwrap();
    assert!(
        matches!(reply(&worker, AppCommand::ApproveProposal(first.clone())),
        AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    let AppEvent::ProposalSource(target) =
        reply(&worker, AppCommand::ProposalSource(TARGET.into()))
    else {
        panic!()
    };
    let edited = format!("{}\nExplicit reviewed body edit", target.text);
    let record = create(
        &worker,
        DraftRequest {
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: Vec::new(),
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "Keep prior provenance".into(),
            changes: vec![DraftNoteChange::Replace {
                path: TARGET.into(),
                expected: target.source.fingerprint.clone(),
                text: edited,
            }],
            sources: vec![target.source],
        },
    );
    let second = approve(&worker, &record);
    assert_eq!(inspect(&worker).citations[0].citation, citation);
    assert_eq!(
        inspect(&worker).citations[0].outcome,
        CitationOutcome::Changed
    );
    for target_operation_id in [second.operation_id, first.operation_id] {
        assert!(
            matches!(reply(&worker, AppCommand::UndoProposal(UndoRequest {operation_id: Uuid::new_v4(), target_operation_id, trash_member: None})), AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
        );
    }
    assert_eq!(
        fs::read(fixture.vault.join(TARGET)).unwrap(),
        ORIGINAL.as_bytes()
    );
    worker.shutdown().unwrap();
}

#[test]
fn preparation_is_additive_bounded_and_refuses_self_sources_and_archived_destinations() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let citation = capture(&worker, &fixture).citation;
    let record = create(&worker, prepare(&worker, request(citation.clone())));
    approve(&worker, &record);
    assert!(
        matches!(reply(&worker, AppCommand::PrepareNoteProvenance(request(citation.clone()))), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    let archived = ProvenanceRequest {
        path: SOURCE.into(),
        ..request(citation.clone())
    };
    assert!(
        matches!(reply(&worker, AppCommand::PrepareNoteProvenance(archived)), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    let dup = ProvenanceRequest {
        citations: vec![citation.clone(), citation],
        ..request(capture(&worker, &fixture).citation)
    };
    assert!(
        matches!(reply(&worker, AppCommand::PrepareNoteProvenance(dup)), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    let AppEvent::ProposalSource(target) =
        reply(&worker, AppCommand::ProposalSource(TARGET.into()))
    else {
        panic!()
    };
    let target_id = brn_store::note_identity::read(&target.text)
        .unwrap()
        .unwrap();
    let start_byte = target.text.find("Current interpretation").unwrap();
    let AppEvent::CitationCaptured(self_capture) = reply(
        &worker,
        AppCommand::CaptureCitation(CitationRequest {
            note_id: target_id,
            expected_sha256: target.source.fingerprint.sha256,
            start_byte,
            end_byte: start_byte + "Current interpretation".len(),
        }),
    ) else {
        panic!()
    };
    assert!(
        matches!(reply(&worker, AppCommand::PrepareNoteProvenance(request(self_capture.citation.clone()))), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    worker.shutdown().unwrap();
}
