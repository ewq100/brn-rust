use super::*;
use brn_workflow::{NotePage, library::KnowledgeScope, vault::NoteText};

fn page(path: &str, next: Option<&str>) -> AppEvent {
    AppEvent::Notes(NotePage {
        notes: vec![NoteEntry {
            path: path.into(),
            title: path.into(),
        }],
        next_cursor: next.map(str::to_owned),
    })
}

fn note(text: &str) -> AppEvent {
    AppEvent::Note(NoteText {
        text: text.into(),
        sha256: [7; 32],
    })
}

#[test]
fn scope_switch_lists_saved_scope_without_changing_open_editor_or_ask() {
    let mut state = ready();
    let (open, _) = state.open_editor("current.md".into());
    state.apply(
        open,
        AppEvent::Editor(editor_view("current.md", "current λ")),
    );
    state
        .editor
        .as_mut()
        .unwrap()
        .edit("local λ".into(), Instant::now())
        .unwrap();
    let generation = state.note_generation;
    assert_eq!(state.knowledge_scope, KnowledgeScope::Current);
    let (_, command) = state.select_scope(KnowledgeScope::Source).unwrap();
    assert!(matches!(
        command,
        AppCommand::ScopedNotes {
            scope: KnowledgeScope::Source,
            cursor: None,
            ..
        }
    ));
    assert_eq!(state.note_generation, generation);
    assert_eq!(state.editor.as_ref().unwrap().text, "local λ");
    assert!(state.ask("current question".into()).is_some());
}

#[test]
fn scoped_list_rejects_old_scope_reset_duplicate_append_and_stale_errors() {
    let mut state = ready();
    let (old, _) = state.refresh_notes().unwrap();
    let (source, _) = state.select_scope(KnowledgeScope::Source).unwrap();
    state.apply(old, page("current.md", Some("current.md")));
    assert!(state.notes.is_empty());
    state.apply(source, page("source.md", Some("source.md")));
    let (append, command) = state.more_notes().unwrap();
    assert!(
        matches!(command, AppCommand::ScopedNotes { scope: KnowledgeScope::Source, cursor: Some(cursor), .. } if cursor == "source.md")
    );
    assert!(state.more_notes().is_none());
    let (reset, _) = state.refresh_notes().unwrap();
    state.apply(append, page("old-page.md", None));
    state.apply(reset, page("new-source.md", Some("new-source.md")));
    assert_eq!(state.notes[0].path, "new-source.md");
    assert_eq!(state.next_cursor.as_deref(), Some("new-source.md"));
    let (stale, _) = state.more_notes().unwrap();
    let (history, _) = state.select_scope(KnowledgeScope::History).unwrap();
    state.apply(history, page("archive/history.md", None));
    let notice = state.notice.clone();
    state.apply(
        stale,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("old failure")),
    );
    assert_eq!(state.notice, notice);
    assert!(state.notes_error.is_none());
    assert_eq!(state.notes[0].path, "archive/history.md");
}

#[test]
fn search_submission_and_scope_switch_discard_stale_results_and_errors() {
    let mut state = ready();
    let (old, _) = state.search_notes("old".into()).unwrap();
    let (new, _) = state.search_notes("new".into()).unwrap();
    state.apply(
        old,
        AppEvent::Search(SearchResults {
            hits: vec![],
            keyword_only: false,
        }),
    );
    assert!(state.search.is_none());
    state.apply(
        new,
        AppEvent::Search(SearchResults {
            hits: vec![],
            keyword_only: true,
        }),
    );
    assert_eq!(state.search_scope, Some(KnowledgeScope::Current));
    let (stale, _) = state.search_notes("q".into()).unwrap();
    state.select_scope(KnowledgeScope::All).unwrap();
    let notice = state.notice.clone();
    state.apply(
        stale,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("old search failure")),
    );
    assert_eq!(state.notice, notice);
    assert!(state.search.is_none());
    let (_, command) = state.search_notes("q".into()).unwrap();
    assert!(matches!(
        command,
        AppCommand::ScopedSearch {
            scope: KnowledgeScope::All,
            ..
        }
    ));
}

#[test]
fn evidence_freezes_requested_scope_and_exact_text_and_never_admits_mutations() {
    let mut state = ready();
    let exact = "\u{feff}---\r\nbrn_kind: source\r\n---\r\nOriginal 日本語 λ\nlast\r";
    let (source, command) = state.open_evidence("source.md".into(), KnowledgeScope::Source);
    assert!(
        matches!(command, AppCommand::ScopedNote { scope: KnowledgeScope::Source, path } if path == "source.md")
    );
    state.select_scope(KnowledgeScope::History).unwrap();
    state.apply(source, note(exact));
    let evidence = state.evidence.as_ref().unwrap();
    assert_eq!(evidence.scope, KnowledgeScope::Source);
    assert_eq!(evidence.path, "source.md");
    assert_eq!(
        evidence.note.as_ref().unwrap().text.as_bytes(),
        exact.as_bytes()
    );
    assert!(state.editor.is_none());
    assert!(state.save_editor(None).is_none());
    assert!(state.recover_editor().is_none());
    let (stale, _) = state.open_evidence("archive/old.md".into(), KnowledgeScope::History);
    let (stale_fail, _) = state.open_evidence("archive/failing.md".into(), KnowledgeScope::History);
    let (current, _) = state.open_editor("current.md".into());
    state.apply(stale, note("late history"));
    state.apply(
        current,
        AppEvent::Editor(editor_view("current.md", "new current")),
    );
    assert!(state.evidence.is_none());
    let notice = state.notice.clone();
    state.apply(
        stale_fail,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("late failure")),
    );
    assert_eq!(state.notice, notice);
    assert!(state.note_error.is_none());
    assert_eq!(state.editor.as_ref().unwrap().text, "new current");
}

#[test]
fn refresh_and_application_list_use_the_selected_scope() {
    let mut state = ready();
    state.select_scope(KnowledgeScope::History).unwrap();
    let (refresh, _) = state.command(Pending::Refresh, AppCommand::Refresh);
    let commands = state.apply(refresh, AppEvent::Refreshed(RefreshReport::default()));
    assert!(commands.iter().any(|(_, command)| matches!(
        command,
        AppCommand::ScopedNotes {
            scope: KnowledgeScope::History,
            cursor: None,
            ..
        }
    )));
    let commands = state.refresh_after_application(&[], state.review_generation);
    assert!(commands.iter().any(|(_, command)| matches!(
        command,
        AppCommand::ScopedNotes {
            scope: KnowledgeScope::History,
            cursor: None,
            ..
        }
    )));
    assert!(
        !commands
            .iter()
            .any(|(_, command)| matches!(command, AppCommand::Notes { .. }))
    );
}

#[cfg(target_os = "macos")]
#[test]
fn real_worker_scoped_browsing_preserves_originals_without_editor_registration() {
    use brn_workflow::{app::AppConfig, app_worker::AppWorker};
    use std::fs;
    let fixture = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let vault = fixture.path().join("vault");
    fs::create_dir(&vault).unwrap();
    fs::create_dir(fixture.path().join("data")).unwrap();
    fs::create_dir(vault.join("archive")).unwrap();
    let originals = [
        ("current.md", "# Current\r\nneedle current λ\r\n"),
        (
            "source.md",
            "\u{feff}---\r\nbrn_kind: source\r\n---\r\nneedle original 日本語 λ\nlast\r",
        ),
        (
            "archive/history.md",
            "# Historical\r\nneedle history 🧭\r\n",
        ),
    ];
    for (path, text) in originals {
        fs::write(vault.join(path), text).unwrap();
    }
    for _ in 0..2 {
        let mut worker = AppWorker::start(
            fixture.path().join("data"),
            AppConfig {
                vault_root: Some(vault.clone()),
                credentials_dir: Some(fixture.path().join("credentials")),
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
                AppEvent::Ready { .. } => break,
                AppEvent::Failed(error) => panic!("startup: {}", error.message),
                _ => {}
            }
        }
        let reply = |command: (Uuid, AppCommand)| {
            let (id, command) = command;
            worker.submit(id, command).unwrap();
            loop {
                let (actual, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
                if actual == id {
                    if let AppEvent::Failed(error) = &event {
                        panic!("command: {}", error.message);
                    }
                    break (id, event);
                }
            }
        };
        let mut state = ready();
        for (scope, expected) in [
            (KnowledgeScope::Current, vec!["current.md"]),
            (KnowledgeScope::Source, vec!["source.md"]),
            (KnowledgeScope::History, vec!["archive/history.md"]),
            (
                KnowledgeScope::All,
                vec!["archive/history.md", "current.md", "source.md"],
            ),
        ] {
            let list = if scope == KnowledgeScope::Current {
                state.refresh_notes().unwrap()
            } else {
                state.select_scope(scope).unwrap()
            };
            let (id, event) = reply(list);
            state.apply(id, event);
            assert_eq!(
                state
                    .notes
                    .iter()
                    .map(|note| note.path.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
            let (id, event) = reply(state.search_notes("needle".into()).unwrap());
            state.apply(id, event);
            let mut paths = state
                .search
                .as_ref()
                .unwrap()
                .hits
                .iter()
                .map(|hit| hit.path.as_str())
                .collect::<Vec<_>>();
            paths.sort();
            assert_eq!(paths, expected);
            assert_eq!(state.search_scope, Some(scope));
            if scope != KnowledgeScope::Current {
                let path = expected[0];
                let (id, event) = reply(state.open_evidence(path.into(), scope));
                state.apply(id, event);
                let exact = originals
                    .iter()
                    .find(|(original, _)| *original == path)
                    .unwrap()
                    .1;
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
                    exact.as_bytes()
                );
                assert!(state.editor.is_none());
                assert!(state.save_editor(None).is_none());
            }
        }
        let (_, event) = reply((Uuid::new_v4(), AppCommand::Editors));
        assert!(matches!(event, AppEvent::Editors(editors) if editors.is_empty()));
        for (path, text) in originals {
            assert_eq!(fs::read(vault.join(path)).unwrap(), text.as_bytes());
        }
        worker.shutdown().unwrap();
    }
}
