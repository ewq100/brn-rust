use super::*;
use brn_workflow::knowledge::{CitationOutcome, NoteProvenance, ResolvedCitation, VaultCitation};

const QUOTE: &str = "\u{feff}Original õäöü\r\n原文 🦀\n";
fn provenance(path: &str, outcome: CitationOutcome) -> AppEvent {
    AppEvent::NoteProvenance(Box::new(NoteProvenance {
        path: path.into(),
        citations: vec![ResolvedCitation {
            citation: VaultCitation {
                note_id: Uuid::new_v4(),
                sha256: [17; 32],
                start_byte: 0,
                end_byte: QUOTE.len(),
                quote: QUOTE.into(),
            },
            outcome,
            matches: vec![],
            issues: vec![],
        }],
    }))
}
fn current(state: &mut AiState, path: &str) {
    let (id, _) = state.open_editor(path.into());
    state.apply(id, AppEvent::Editor(editor_view(path, "Saved current\r\n")));
}

#[test]
fn saved_inspection_preserves_unsaved_typing_scope_and_review_state() {
    let mut state = ready();
    current(&mut state, "current.md");
    state
        .editor
        .as_mut()
        .unwrap()
        .edit("Unacknowledged typing õ\r\n".into(), Instant::now())
        .unwrap();
    let generation = state.note_generation;
    let review_generation = state.review_generation;
    let (id, command) = state.inspect_provenance().unwrap();
    assert!(matches!(command, AppCommand::NoteProvenance(path) if path == "current.md"));
    assert!(state.provenance_loading());
    assert!(state.inspect_provenance().is_none());
    state.apply(id, provenance("current.md", CitationOutcome::Matched));
    assert!(!state.provenance_loading());
    assert_eq!(
        state.provenance.as_ref().unwrap().citations[0]
            .citation
            .quote
            .as_bytes(),
        QUOTE.as_bytes()
    );
    assert_eq!(
        state.editor.as_ref().unwrap().text,
        "Unacknowledged typing õ\r\n"
    );
    assert_eq!(state.note_generation, generation);
    assert_eq!(state.review_generation, review_generation);
    assert_eq!(state.knowledge_scope, KnowledgeScope::Current);
}

#[test]
fn new_document_or_inspection_discards_old_success_wrong_path_and_stale_errors() {
    let mut state = ready();
    current(&mut state, "first.md");
    let (old, _) = state.inspect_provenance().unwrap();
    current(&mut state, "second.md");
    let (new, _) = state.inspect_provenance().unwrap();
    state.apply(old, provenance("first.md", CitationOutcome::Matched));
    assert!(state.provenance.is_none());
    state.apply(new, provenance("wrong.md", CitationOutcome::Matched));
    assert!(state.provenance.is_none());
    let (old, _) = state.inspect_provenance().unwrap();
    state.clear_provenance();
    let (new, _) = state.inspect_provenance().unwrap();
    state.apply(new, provenance("second.md", CitationOutcome::Changed));
    let preserved = state.provenance.clone();
    let notice = state.notice.clone();
    state.apply(
        old,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("old inspection failure")),
    );
    assert_eq!(state.provenance, preserved);
    assert!(state.provenance_error.is_none());
    assert_eq!(state.notice, notice);
    let (old, _) = state.inspect_provenance().unwrap();
    state.clear_provenance();
    let (new, _) = state.inspect_provenance().unwrap();
    state.apply(old, provenance("second.md", CitationOutcome::Absent));
    assert!(state.provenance.is_none());
    state.apply(
        new,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("fresh inspection failure")),
    );
    assert_eq!(
        state.provenance_error.as_deref(),
        Some("fresh inspection failure")
    );
    assert!(!state.provenance_loading());
}

#[test]
fn evidence_scope_inspection_and_closed_panel_preserve_exact_sources_and_outcomes() {
    let mut state = ready();
    let (open, _) = state.open_evidence("archive/original.md".into(), KnowledgeScope::All);
    state.apply(
        open,
        AppEvent::Note(brn_workflow::vault::NoteText {
            text: QUOTE.into(),
            sha256: [7; 32],
        }),
    );
    for outcome in [
        CitationOutcome::Matched,
        CitationOutcome::Changed,
        CitationOutcome::Absent,
        CitationOutcome::Ambiguous,
        CitationOutcome::Incomplete,
    ] {
        let (id, _) = state.inspect_provenance().unwrap();
        state.apply(id, provenance("archive/original.md", outcome));
        assert_eq!(
            state.provenance.as_ref().unwrap().citations[0].outcome,
            outcome
        );
        assert_eq!(
            state
                .evidence
                .as_ref()
                .unwrap()
                .note
                .as_ref()
                .unwrap()
                .text
                .as_bytes(),
            QUOTE.as_bytes()
        );
        assert_eq!(state.evidence.as_ref().unwrap().scope, KnowledgeScope::All);
    }
    let (late, _) = state.inspect_provenance().unwrap();
    state.clear_provenance();
    state.apply(
        late,
        provenance("archive/original.md", CitationOutcome::Matched),
    );
    assert!(state.provenance.is_none());
    assert!(!state.provenance_loading());
    state.evidence = None;
    assert!(state.inspect_provenance().is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn real_worker_inspects_saved_citations_while_unsaved_typing_remains_recoverable() {
    use brn_workflow::{app::AppConfig, app_worker::AppWorker};
    use std::fs;
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    let vault = owner.path().join("vault");
    fs::create_dir(&data).unwrap();
    fs::create_dir(&vault).unwrap();
    let source_id = Uuid::new_v4();
    let source = format!(
        "{}{QUOTE}",
        brn_workflow::knowledge::note_identity::assign("# Source\n", source_id).unwrap()
    );
    fs::write(vault.join("source.md"), &source).unwrap();
    let hash = brn_workflow::vault::read_evidence(
        &vault,
        &brn_workflow::vault::EvidencePath::parse("source.md").unwrap(),
    )
    .unwrap()
    .sha256;
    let start_byte = source.find(QUOTE).unwrap();
    let citation = VaultCitation {
        note_id: source_id,
        sha256: hash,
        start_byte,
        end_byte: start_byte + QUOTE.len(),
        quote: QUOTE.into(),
    };
    let saved = format!(
        "---\r\nbrn_provenance: {}\r\n---\r\nCurrent saved\r\n",
        serde_json::to_string(&vec![citation]).unwrap()
    );
    fs::write(vault.join("current.md"), &saved).unwrap();
    let mut worker = AppWorker::start(
        data.clone(),
        AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: Some(owner.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    let mut state = AiState::default();
    let (id, event) = worker.recv_event_timeout(Duration::from_secs(15)).unwrap();
    state.apply(id, event);
    let send = |command: (Uuid, AppCommand)| {
        let (id, command) = command;
        worker.submit(id, command).unwrap();
        let (reply, event) = worker.recv_event_timeout(Duration::from_secs(15)).unwrap();
        assert_eq!(id, reply);
        (id, event)
    };
    let (id, event) = send(state.open_editor("current.md".into()));
    state.apply(id, event);
    state
        .editor
        .as_mut()
        .unwrap()
        .edit("Unsaved replacement õ\r\n".into(), Instant::now())
        .unwrap();
    let (id, event) = send(state.inspect_provenance().unwrap());
    state.apply(id, event);
    assert_eq!(
        state.provenance.as_ref().unwrap().citations[0].outcome,
        CitationOutcome::Matched
    );
    assert_eq!(
        state.provenance.as_ref().unwrap().citations[0]
            .citation
            .quote
            .as_bytes(),
        QUOTE.as_bytes()
    );
    assert_eq!(
        state.editor.as_ref().unwrap().text,
        "Unsaved replacement õ\r\n"
    );
    let (id, event) = send(state.recover_editor().unwrap());
    state.apply(id, event);
    assert_eq!(
        fs::read(vault.join("current.md")).unwrap(),
        saved.as_bytes()
    );
    assert_eq!(
        fs::read(vault.join("source.md")).unwrap(),
        source.as_bytes()
    );
    worker.shutdown().unwrap();
    let mut worker = AppWorker::start(
        data,
        AppConfig {
            vault_root: Some(vault),
            credentials_dir: Some(owner.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    worker.recv_event_timeout(Duration::from_secs(15)).unwrap();
    let id = Uuid::new_v4();
    worker
        .submit(id, AppCommand::OpenEditor("current.md".into()))
        .unwrap();
    let (_, event) = worker.recv_event_timeout(Duration::from_secs(15)).unwrap();
    assert!(
        matches!(event,AppEvent::Editor(view) if view.record.text=="Unsaved replacement õ\r\n")
    );
    worker.shutdown().unwrap();
}

#[test]
fn applied_original_save_invalidates_completed_and_pending_saved_inspections() {
    let mut state = ready();
    current(&mut state, "current.md");
    let (id, _) = state.inspect_provenance().unwrap();
    state.apply(id, provenance("current.md", CitationOutcome::Matched));
    let (late, _) = state.inspect_provenance().unwrap();
    state
        .editor
        .as_mut()
        .unwrap()
        .edit("Explicit saved replacement".into(), Instant::now())
        .unwrap();
    let (save, AppCommand::SaveEditor(request)) = state.save_editor(None).unwrap() else {
        panic!()
    };
    state.apply(
        save,
        AppEvent::EditorSaved(SaveReceipt {
            operation_id: save,
            path: request.edit.path,
            destination: None,
            submitted_generation: request.edit.generation,
            stamp: EditStamp {
                baseline: Uuid::new_v4(),
                generation: 1,
            },
            outcome: SaveOutcome::Applied,
        }),
    );
    state.apply(late, provenance("current.md", CitationOutcome::Matched));
    assert!(state.provenance.is_none());
    assert!(!state.provenance_loading());
    assert!(state.inspect_provenance().is_some());
    assert_eq!(
        state.editor.as_ref().unwrap().text,
        "Explicit saved replacement"
    );
}

#[test]
fn confirmed_reload_invalidates_an_old_same_path_inspection() {
    let mut state = ready();
    current(&mut state, "current.md");
    let (late, _) = state.inspect_provenance().unwrap();
    let mut record = state.editor.as_ref().unwrap().view.record.clone();
    let request = ReloadRequest {
        path: record.path.clone(),
        expected: record.stamp,
        observed: state
            .editor
            .as_ref()
            .unwrap()
            .view
            .observed
            .clone()
            .unwrap(),
        discard: false,
    };
    let (id, _) = state.reload_editor(request).unwrap();
    record.stamp.baseline = Uuid::new_v4();
    state.apply(id, AppEvent::EditorRecovered(record));
    state.apply(late, provenance("current.md", CitationOutcome::Matched));
    assert!(state.provenance.is_none());
    assert!(!state.provenance_loading());
    assert!(state.inspect_provenance().is_some());
}
