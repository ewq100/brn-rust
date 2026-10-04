use super::*;
use brn_workflow::knowledge::{
    EdgeEndpoint, EdgeEvidence, EdgeOrigin, EvidenceEndpoint, IdentityOutcome, LinkEvidence,
    NoteEdge, NoteIdentityInfo, NoteLinkOutcome, NoteLinks, RelationshipPage, ResolvedNoteLink,
};

const QUOTE: &str = "[Õ 原文 🦀](brn://note/22222222-2222-4222-8222-222222222222)\r\n";
fn current(state: &mut AiState, path: &str) {
    let (id, _) = state.open_editor(path.into());
    state.apply(id, AppEvent::Editor(editor_view(path, "Saved current\r\n")));
}
fn links(path: &str, outcome: NoteLinkOutcome) -> AppEvent {
    AppEvent::NoteLinks(Box::new(NoteLinks {
        source: NoteIdentityInfo {
            path: path.into(),
            note_id: Some(Uuid::new_v4()),
            sha256: [5; 32],
        },
        source_outcome: Some(IdentityOutcome::Unique),
        links: vec![ResolvedNoteLink {
            destination: "brn://note/22222222-2222-4222-8222-222222222222".into(),
            evidence: vec![LinkEvidence {
                start_byte: 0,
                end_byte: QUOTE.len(),
                quote: QUOTE.into(),
            }],
            target_path: Some("archive/source.md".into()),
            outcome,
            matches: vec![],
            issues: vec![],
        }],
        issues: vec![],
    }))
}
fn page(scope: KnowledgeScope, offset: usize, total: usize, count: usize) -> AppEvent {
    AppEvent::Relationships(Box::new(RelationshipPage {
        scope,
        offset,
        total,
        issues: vec![],
        duplicates: vec![],
        edges: (0..count)
            .map(|index| NoteEdge {
                source: EdgeEndpoint {
                    path: format!("source-{index}.md"),
                    note_id: Uuid::new_v4(),
                    sha256: [7; 32],
                },
                target: EdgeEndpoint {
                    path: format!("target-{index}.md"),
                    note_id: Uuid::new_v4(),
                    sha256: [8; 32],
                },
                origin: EdgeOrigin::ExplicitLink,
                evidence: vec![EdgeEvidence {
                    endpoint: EvidenceEndpoint::Source,
                    start_byte: 0,
                    end_byte: QUOTE.len(),
                    quote: QUOTE.into(),
                }],
            })
            .collect(),
    }))
}

#[test]
fn saved_links_leave_unsaved_typing_and_review_intact_and_keep_full_quote() {
    let mut state = ready();
    current(&mut state, "current.md");
    state
        .editor
        .as_mut()
        .unwrap()
        .edit("Unacknowledged õ\r\n".into(), Instant::now())
        .unwrap();
    let document = state.note_generation;
    let review = state.review_generation;
    let (id, command) = state.inspect_links().unwrap();
    assert!(matches!(command, AppCommand::NoteLinks(path) if path == "current.md"));
    assert!(state.links_loading());
    assert!(state.inspect_links().is_none());
    state.apply(id, links("current.md", NoteLinkOutcome::Resolved));
    assert_eq!(
        state.links.as_ref().unwrap().links[0].evidence[0]
            .quote
            .as_bytes(),
        QUOTE.as_bytes()
    );
    assert!(!state.links_loading());
    assert_eq!(state.editor.as_ref().unwrap().text, "Unacknowledged õ\r\n");
    assert_eq!(state.note_generation, document);
    assert_eq!(state.review_generation, review);
}

#[test]
fn navigation_close_wrong_path_and_stale_errors_never_replace_link_inspection() {
    let mut state = ready();
    current(&mut state, "first.md");
    let (old, _) = state.inspect_links().unwrap();
    current(&mut state, "second.md");
    let (new, _) = state.inspect_links().unwrap();
    state.apply(old, links("first.md", NoteLinkOutcome::Resolved));
    assert!(state.links.is_none());
    state.apply(new, links("wrong.md", NoteLinkOutcome::Resolved));
    assert!(state.links.is_none());
    let (old, _) = state.inspect_links().unwrap();
    state.clear_links();
    let (new, _) = state.inspect_links().unwrap();
    state.apply(new, links("second.md", NoteLinkOutcome::Ambiguous));
    let preserved = state.links.clone();
    let notice = state.notice.clone();
    state.apply(
        old,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("old read")),
    );
    assert_eq!(state.links, preserved);
    assert!(state.links_error.is_none());
    assert_eq!(state.notice, notice);
    let (old, _) = state.inspect_links().unwrap();
    state.clear_links();
    state.apply(old, links("second.md", NoteLinkOutcome::Resolved));
    assert!(state.links.is_none());
    assert!(!state.links_loading());
}

#[test]
fn evidence_links_preserve_read_only_text_and_refusal_outcome() {
    let mut state = ready();
    let (id, _) = state.open_evidence("archive/source.md".into(), KnowledgeScope::All);
    state.apply(
        id,
        AppEvent::Note(brn_workflow::vault::NoteText {
            text: QUOTE.into(),
            sha256: [5; 32],
        }),
    );
    let (id, _) = state.inspect_links().unwrap();
    state.apply(id, links("archive/source.md", NoteLinkOutcome::Incomplete));
    assert_eq!(
        state.links.as_ref().unwrap().links[0].outcome,
        NoteLinkOutcome::Incomplete
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
    assert!(state.editor.is_none());
    let (id, _) = state.inspect_links().unwrap();
    state.apply(
        id,
        AppEvent::Failed(brn_workflow::WorkflowError {
            kind: brn_workflow::ErrorKind::SaveUncertain,
            message: "uncertain work".into(),
        }),
    );
    assert_eq!(state.links_error.as_deref(), Some("uncertain work"));
    assert!(!state.links_loading());
}

#[test]
fn relationship_pages_replace_one_snapshot_with_checked_next_and_previous_offsets() {
    let mut state = ready();
    let (id, command) = state.refresh_relationships(0).unwrap();
    assert!(
        matches!(command, AppCommand::Relationships(request) if request.scope == KnowledgeScope::Current && request.offset == 0 && request.limit == 25)
    );
    assert!(state.relationships_loading());
    assert!(state.refresh_relationships(0).is_none());
    state.apply(id, page(KnowledgeScope::Current, 0, 27, 25));
    assert_eq!(state.relationship_next_offset(), Some(25));
    assert_eq!(state.relationship_previous_offset(), None);
    let (id, _) = state.refresh_relationships(25).unwrap();
    assert!(state.relationships.is_none());
    state.apply(id, page(KnowledgeScope::Current, 25, 27, 2));
    assert_eq!(state.relationships.as_ref().unwrap().edges.len(), 2);
    assert_eq!(state.relationship_next_offset(), None);
    assert_eq!(state.relationship_previous_offset(), Some(0));
    let (id, _) = state.refresh_relationships(usize::MAX).unwrap();
    state.apply(id, page(KnowledgeScope::Current, usize::MAX, 27, 0));
    assert_eq!(state.relationship_next_offset(), None);
    assert_eq!(state.relationship_previous_offset(), Some(usize::MAX - 25));
}

#[test]
fn scope_close_and_wrong_page_metadata_discard_stale_relationship_success_and_errors() {
    let mut state = ready();
    let (old, _) = state.refresh_relationships(0).unwrap();
    state.select_scope(KnowledgeScope::All).unwrap();
    let (new, _) = state.refresh_relationships(0).unwrap();
    state.apply(old, page(KnowledgeScope::Current, 0, 1, 1));
    assert!(state.relationships.is_none());
    state.apply(new, page(KnowledgeScope::Current, 0, 1, 1));
    assert!(state.relationships.is_none());
    let (old, _) = state.refresh_relationships(0).unwrap();
    state.clear_relationships();
    let (new, _) = state.refresh_relationships(25).unwrap();
    state.apply(new, page(KnowledgeScope::All, 25, 25, 0));
    let preserved = state.relationships.clone();
    let notice = state.notice.clone();
    state.apply(
        old,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("old page failure")),
    );
    assert_eq!(state.relationships, preserved);
    assert!(state.relationships_error.is_none());
    assert_eq!(state.notice, notice);
    let (id, _) = state.refresh_relationships(0).unwrap();
    state.apply(id, page(KnowledgeScope::All, 1, 1, 0));
    assert!(state.relationships.is_none());
    let (id, _) = state.refresh_relationships(0).unwrap();
    state.apply(id, page(KnowledgeScope::All, 0, 26, 26));
    assert!(state.relationships.is_none());
    let (id, _) = state.refresh_relationships(0).unwrap();
    state.apply(
        id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("fresh page failure")),
    );
    assert_eq!(
        state.relationships_error.as_deref(),
        Some("fresh page failure")
    );
    let (id, _) = state.refresh_relationships(0).unwrap();
    state.clear_relationships();
    state.apply(id, page(KnowledgeScope::All, 0, 1, 1));
    assert!(state.relationships.is_none());
}

#[test]
fn applied_original_save_and_reload_invalidate_link_and_relationship_inspections() {
    for reload in [false, true] {
        let mut state = ready();
        current(&mut state, "current.md");
        let (late_links, _) = state.inspect_links().unwrap();
        let (late_page, _) = state.refresh_relationships(0).unwrap();
        if reload {
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
        } else {
            state
                .editor
                .as_mut()
                .unwrap()
                .edit("Explicit saved replacement".into(), Instant::now())
                .unwrap();
            let (id, AppCommand::SaveEditor(request)) = state.save_editor(None).unwrap() else {
                panic!("Save")
            };
            state.apply(
                id,
                AppEvent::EditorSaved(SaveReceipt {
                    operation_id: id,
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
        }
        state.apply(late_links, links("current.md", NoteLinkOutcome::Resolved));
        state.apply(late_page, page(KnowledgeScope::Current, 0, 1, 1));
        assert!(state.links.is_none());
        assert!(state.relationships.is_none());
        assert!(!state.links_loading());
        assert!(!state.relationships_loading());
    }
}

#[test]
fn save_copy_invalidates_saved_link_and_relationship_observations() {
    let mut state = ready();
    current(&mut state, "current.md");
    let (link_id, _) = state.inspect_links().unwrap();
    state.apply(link_id, links("current.md", NoteLinkOutcome::Resolved));
    let (late_page, _) = state.refresh_relationships(0).unwrap();
    let (id, AppCommand::SaveEditor(request)) = state.save_editor(Some("copy.md".into())).unwrap()
    else {
        panic!("Save Copy")
    };
    state.apply(
        id,
        AppEvent::EditorSaved(SaveReceipt {
            operation_id: id,
            path: request.edit.path,
            destination: Some("copy.md".into()),
            submitted_generation: request.edit.generation,
            stamp: request.edit.expected,
            outcome: SaveOutcome::Applied,
        }),
    );
    state.apply(late_page, page(KnowledgeScope::Current, 0, 1, 1));
    assert!(state.links.is_none());
    assert!(state.relationships.is_none());
    assert_eq!(
        state.editor.as_ref().unwrap().view.record.path,
        "current.md"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn actual_worker_save_copy_changes_source_identity_without_changing_original_bytes() {
    use brn_workflow::{app::AppConfig, app_worker::AppWorker};
    use std::fs;
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    let vault = owner.path().join("vault");
    fs::create_dir(&data).unwrap();
    fs::create_dir(&vault).unwrap();
    let text = "---\r\nbrn_id: 11111111-1111-4111-8111-111111111111\r\n---\r\nSaved õ\r\n";
    fs::write(vault.join("current.md"), text).unwrap();
    let mut worker = AppWorker::start(
        data,
        AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: Some(owner.path().join("task.credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(20))
            .unwrap()
            .1,
        AppEvent::Ready { .. }
    ));
    let mut state = ready();
    let drive = |state: &mut AiState, pair: (Uuid, AppCommand)| {
        worker.submit(pair.0, pair.1).unwrap();
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
        assert_eq!(id, pair.0);
        state.apply(id, event);
    };
    let open = state.open_editor("current.md".into());
    drive(&mut state, open);
    let inspect = state.inspect_links().unwrap();
    drive(&mut state, inspect);
    assert_eq!(
        state.links.as_ref().unwrap().source_outcome,
        Some(IdentityOutcome::Unique)
    );
    let copy = state.save_editor(Some("copy.md".into())).unwrap();
    drive(&mut state, copy);
    let id = Uuid::new_v4();
    worker
        .submit(id, AppCommand::NoteLinks("current.md".into()))
        .unwrap();
    let (actual, AppEvent::NoteLinks(fresh)) =
        worker.recv_event_timeout(Duration::from_secs(20)).unwrap()
    else {
        panic!("fresh links")
    };
    assert_eq!(actual, id);
    assert_eq!(fresh.source_outcome, Some(IdentityOutcome::Ambiguous));
    worker.shutdown().unwrap();
    assert_eq!(fs::read(vault.join("current.md")).unwrap(), text.as_bytes());
    assert_eq!(fs::read(vault.join("copy.md")).unwrap(), text.as_bytes());
    assert_eq!(state.editor.as_ref().unwrap().text, text);
    assert!(
        state.links.is_none(),
        "known duplicate introduction must invalidate displayed Unique source"
    );
}

#[test]
fn known_application_effects_and_refresh_discard_older_relationship_pages() {
    for application in [false, true] {
        let mut state = ready();
        current(&mut state, "current.md");
        state
            .editor
            .as_mut()
            .unwrap()
            .edit("Unsaved retained õ".into(), Instant::now())
            .unwrap();
        let (late_link, _) = state.inspect_links().unwrap();
        let (late_page, _) = state.refresh_relationships(0).unwrap();
        if application {
            state.refresh_after_application(&[], state.review_generation);
        } else {
            let (id, _) = state.command(Pending::Refresh, AppCommand::Refresh);
            state.apply(id, AppEvent::Refreshed(RefreshReport::default()));
        }
        state.apply(late_page, page(KnowledgeScope::Current, 0, 1, 1));
        assert!(state.relationships.is_none());
        state.apply(late_link, links("current.md", NoteLinkOutcome::Resolved));
        assert_eq!(state.links.is_none(), application);
        assert_eq!(state.editor.as_ref().unwrap().text, "Unsaved retained õ");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn actual_worker_observes_saved_links_and_scoped_edges_without_vault_or_proposal_changes() {
    use brn_workflow::{app::AppConfig, app_worker::AppWorker};
    use std::fs;
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    let vault = owner.path().join("vault");
    let credentials = owner.path().join("task.credentials");
    fs::create_dir(&data).unwrap();
    fs::create_dir(&vault).unwrap();
    fs::create_dir(vault.join("archive")).unwrap();
    let current = "\u{feff}---\r\nbrn_id: 11111111-1111-4111-8111-111111111111\r\n...\r\n[exact](brn://note/22222222-2222-4222-8222-222222222222)\r\n";
    let source = "---\nbrn_id: 22222222-2222-4222-8222-222222222222\nbrn_kind: source\n---\nOriginal õ 原文 🦀\r\n";
    fs::write(vault.join("current.md"), current).unwrap();
    fs::write(vault.join("archive/source.md"), source).unwrap();
    let mut worker = AppWorker::start(
        data,
        AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: Some(credentials.clone()),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(20))
            .unwrap()
            .1,
        AppEvent::Ready {
            vault_bound: true,
            ..
        }
    ));
    let mut state = ready();
    let drive = |state: &mut AiState, pair: (Uuid, AppCommand)| {
        worker.submit(pair.0, pair.1).unwrap();
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
        assert_eq!(id, pair.0);
        state.apply(id, event);
    };
    let open = state.open_editor("current.md".into());
    drive(&mut state, open);
    state
        .editor
        .as_mut()
        .unwrap()
        .edit("Unsaved retained õ\r\n".into(), Instant::now())
        .unwrap();
    let inspect = state.inspect_links().unwrap();
    drive(&mut state, inspect);
    let result = state.links.as_ref().unwrap();
    assert_eq!(result.links[0].outcome, NoteLinkOutcome::Resolved);
    let proof = &result.links[0].evidence[0];
    assert_eq!(
        current.get(proof.start_byte..proof.end_byte),
        Some(proof.quote.as_str())
    );
    let inspect = state.refresh_relationships(0).unwrap();
    drive(&mut state, inspect);
    assert_eq!(state.relationships.as_ref().unwrap().total, 0);
    state.select_scope(KnowledgeScope::All).unwrap();
    let inspect = state.refresh_relationships(0).unwrap();
    drive(&mut state, inspect);
    let page = state.relationships.as_ref().unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.edges[0].target.path, "archive/source.md");
    assert_eq!(
        state.editor.as_ref().unwrap().text,
        "Unsaved retained õ\r\n"
    );
    let id = Uuid::new_v4();
    worker.submit(id, AppCommand::Proposals(None)).unwrap();
    let (actual, event) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
    assert_eq!(actual, id);
    assert!(matches!(event, AppEvent::Proposals(records) if records.is_empty()));
    worker.shutdown().unwrap();
    assert_eq!(
        fs::read(vault.join("current.md")).unwrap(),
        current.as_bytes()
    );
    assert_eq!(
        fs::read(vault.join("archive/source.md")).unwrap(),
        source.as_bytes()
    );
    assert_eq!(fs::read_dir(credentials).unwrap().count(), 0);
}
