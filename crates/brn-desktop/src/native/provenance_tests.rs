use super::*;
use brn_workflow::{
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    knowledge::{CitationOutcome, NoteProvenance, VaultCitation},
    library::KnowledgeScope,
    proposals::{DraftNoteChange, DraftRequest},
};
use sha2::{Digest, Sha256};
use std::{fs, time::Instant};

const SOURCE: &str = "\u{feff}---\r\nbrn_id: 676451a3-dddb-4e3d-a7c8-c01a52d2356f\r\nbrn_kind: source\r\n---\r\nOriginal 日本語 õäöü 🧭\r\nlast λ\r";

struct Fixture {
    base: tempfile::TempDir,
    saved: String,
}
impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        fs::create_dir(base.path().join("data")).unwrap();
        fs::create_dir(base.path().join("vault")).unwrap();
        let citation = VaultCitation {
            note_id: Uuid::parse_str("676451a3-dddb-4e3d-a7c8-c01a52d2356f").unwrap(),
            sha256: Sha256::digest(SOURCE.as_bytes()).into(),
            start_byte: 0,
            end_byte: SOURCE.len(),
            quote: SOURCE.into(),
        };
        let json = serde_json::to_string(&[citation]).unwrap();
        let saved = format!("---\r\nbrn_provenance: {json}\r\n---\r\nSaved knowledge õäöü\r\n");
        fs::write(base.path().join("vault/source.md"), SOURCE).unwrap();
        fs::write(base.path().join("vault/current.md"), &saved).unwrap();
        Self { base, saved }
    }
    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: Some(self.base.path().join("vault")),
            credentials_dir: Some(self.base.path().join("credentials")),
            model_dir: None,
        }
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(self.base.path().join("data"), self.config()).unwrap();
        loop {
            match worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1
            {
                AppEvent::Ready { .. } => return worker,
                AppEvent::Failed(error) => panic!("startup: {}", error.message),
                _ => {}
            }
        }
    }
    fn unchanged(&self) {
        assert_eq!(
            fs::read(self.base.path().join("vault/source.md")).unwrap(),
            SOURCE.as_bytes()
        );
        assert_eq!(
            fs::read(self.base.path().join("vault/current.md")).unwrap(),
            self.saved.as_bytes()
        );
    }
}
fn receive(worker: &AppWorker, expected: Uuid) -> AppEvent {
    loop {
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if id == expected {
            if let AppEvent::Failed(error) = &event {
                panic!("command: {}", error.message);
            }
            return event;
        }
    }
}
fn reply(worker: &AppWorker, command: (Uuid, AppCommand)) -> (Uuid, AppEvent) {
    let (id, command) = command;
    worker.submit(id, command).unwrap();
    (id, receive(worker, id))
}
struct QuoteProbe(Entity<EditorState>);
impl Render for QuoteProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .child(simple::provenance_quote_widget(&self.0)),
            )
            .child(simple::provenance_copy_button(0, SOURCE))
    }
}

#[gpui_kit::test]
fn actual_source_quote_widget_refuses_typing_and_copy_button_preserves_exact_bytes(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::{EntityInputHandler, VisualTestContext, test::TestWindowExt};
    cx.update(gpui_kit::component::init);
    let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let editor = cx.new(|cx| EditorState::new(window, cx).default_value(SOURCE));
        *capture.borrow_mut() = Some(editor.clone());
        let probe = cx.new(|_| QuoteProbe(editor));
        Root::new(probe, window, cx)
    });
    let editor = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        editor.update(cx, |editor, cx| {
            editor.focus(window, cx);
            assert_eq!(editor.value().as_ref(), SOURCE);
            let end = SOURCE.encode_utf16().count();
            editor.replace_text_in_range(Some(end..end), "blocked mutation", window, cx);
            assert_eq!(editor.value().as_ref(), SOURCE);
        });
        window.click("copy-source-quote-0", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(SOURCE)
        );
        editor.update(cx, |editor, cx| editor.focus(window, cx));
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
        window.dispatch_action(Box::new(gpui_kit::component::input::Copy), cx);
    });
    visual.run_until_parked();
    assert_eq!(
        visual
            .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .as_deref(),
        Some(SOURCE)
    );
}

#[gpui_kit::test]
fn saved_sources_inspection_preserves_unsaved_editor_and_its_navigation_recovery_guard(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let (_, event) = reply(
        &worker,
        (Uuid::new_v4(), AppCommand::OpenEditor("current.md".into())),
    );
    let AppEvent::Editor(view) = event else {
        panic!("editor");
    };
    worker.shutdown().unwrap();
    cx.update(gpui_kit::component::init);
    let window = cx.add_window(|_, _| gpui_kit::Empty);
    let desktop = cx
        .update_window(window.into(), |_, window, cx| {
            cx.new(|cx| {
                Desktop::new(
                    fixture.base.path().join("data"),
                    fixture.config(),
                    (LayoutState::default(), Loaded::Missing),
                    window,
                    cx,
                )
            })
        })
        .unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            let (id, _) = ai.open_editor("current.md".into());
            ai.apply(id, AppEvent::Editor(view));
            ai.editor
                .as_mut()
                .unwrap()
                .edit("Unsaved original typing λ\r\n".into(), Instant::now())
                .unwrap();
            let acknowledged = ai.editor.as_ref().unwrap().view.record.stamp;
            desktop.note_editor.update(cx, |editor, cx| {
                editor.set_value("Unsaved original typing λ\r\n", window, cx)
            });
            desktop.simple_note_path = Some("current.md".into());
            desktop.open_doc = Some(DocRef::SavedNote);
            desktop.inspect_saved_sources(cx);
            let id = *desktop
                .ai
                .as_ref()
                .unwrap()
                .pending
                .iter()
                .find(|(_, pending)| matches!(pending, crate::ai::Pending::Provenance { .. }))
                .unwrap()
                .0;
            let event = receive(desktop.app_worker.as_ref().unwrap(), id);
            desktop.ai.as_mut().unwrap().apply(id, event);
            desktop.sync_provenance_widgets(window, cx);
            let ai = desktop.ai.as_ref().unwrap();
            assert_eq!(
                ai.provenance.as_ref().unwrap().citations[0].outcome,
                CitationOutcome::Matched
            );
            assert_eq!(
                ai.provenance.as_ref().unwrap().citations[0].citation.quote,
                SOURCE
            );
            assert_eq!(
                ai.editor.as_ref().unwrap().text,
                "Unsaved original typing λ\r\n"
            );
            assert_eq!(ai.editor.as_ref().unwrap().view.record.stamp, acknowledged);
            assert!(ai.editor.as_ref().unwrap().needs_recovery());
            assert_eq!(
                desktop.note_editor.read(cx).value().as_ref(),
                "Unsaved original typing λ\r\n"
            );
            assert_eq!(
                desktop.provenance_quotes[0].read(cx).value().as_ref(),
                SOURCE
            );
            let quote_id = desktop.provenance_quotes[0].entity_id();
            desktop.sync_provenance_widgets(window, cx);
            assert_eq!(desktop.provenance_quotes[0].entity_id(), quote_id);
            assert_eq!(desktop.open_doc, Some(DocRef::SavedNote));
            assert_eq!(
                desktop.ai.as_ref().unwrap().knowledge_scope,
                KnowledgeScope::Current
            );
            desktop.simple_leave(simple::EditorTransition::Activity, cx);
            assert!(desktop.simple_transition.is_some());
            assert_eq!(desktop.open_doc, Some(DocRef::SavedNote));
            assert!(desktop.provenance_open);
            assert_eq!(desktop.provenance_quotes[0].entity_id(), quote_id);
            let command = desktop.ai.as_mut().unwrap().recover_editor().unwrap();
            let (id, event) = reply(desktop.app_worker.as_ref().unwrap(), command);
            desktop.ai.as_mut().unwrap().apply(id, event);
            desktop.simple_progress_transition(cx);
            assert_eq!(desktop.open_doc, Some(DocRef::Activity));
            assert!(!desktop.provenance_open);
            assert!(desktop.provenance_quotes.is_empty());
            assert!(desktop.ai.as_ref().unwrap().provenance.is_none());
            desktop.app_worker.as_mut().unwrap().shutdown().unwrap();
        });
    })
    .unwrap();
    fixture.unchanged();
}

#[gpui_kit::test]
fn closing_sources_invalidates_inspection_without_changing_exact_document_or_scope(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let (_, event) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::ScopedNote {
                scope: KnowledgeScope::All,
                path: "current.md".into(),
            },
        ),
    );
    let AppEvent::Note(note) = event else {
        panic!("note");
    };
    let (_, event) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::NoteProvenance("current.md".into()),
        ),
    );
    let AppEvent::NoteProvenance(provenance) = event else {
        panic!("provenance");
    };
    worker.shutdown().unwrap();
    cx.update(gpui_kit::component::init);
    let window = cx.add_window(|_, _| gpui_kit::Empty);
    let desktop = cx
        .update_window(window.into(), |_, window, cx| {
            cx.new(|cx| {
                Desktop::new(
                    fixture.base.path().join("data"),
                    fixture.config(),
                    (LayoutState::default(), Loaded::Missing),
                    window,
                    cx,
                )
            })
        })
        .unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            let (id, _) = ai.open_evidence("current.md".into(), KnowledgeScope::All);
            ai.apply(id, AppEvent::Note(note));
            ai.provenance = Some(*provenance);
            desktop.open_doc = Some(DocRef::Evidence);
            desktop.provenance_open = true;
            desktop.sync_provenance_widgets(window, cx);
            assert_eq!(
                desktop.provenance_quotes[0].read(cx).value().as_ref(),
                SOURCE
            );
            let pending = desktop.ai.as_mut().unwrap().inspect_provenance().unwrap();
            assert!(desktop.ai.as_ref().unwrap().provenance_loading());
            desktop.sync_provenance_widgets(window, cx);
            assert_eq!(
                desktop.provenance_quotes[0].read(cx).value().as_ref(),
                SOURCE
            );
            desktop.clear_saved_sources();
            assert!(!desktop.provenance_open);
            assert!(!desktop.ai.as_ref().unwrap().provenance_loading());
            assert!(desktop.provenance_quotes.is_empty());
            let stale = NoteProvenance {
                inbox_source: None,
                path: "current.md".into(),
                citations: vec![],
            };
            desktop
                .ai
                .as_mut()
                .unwrap()
                .apply(pending.0, AppEvent::NoteProvenance(Box::new(stale)));
            assert!(desktop.ai.as_ref().unwrap().provenance.is_none());
            assert_eq!(desktop.open_doc, Some(DocRef::Evidence));
            let evidence = desktop.ai.as_ref().unwrap().evidence.as_ref().unwrap();
            assert_eq!(evidence.scope, KnowledgeScope::All);
            assert_eq!(evidence.note.as_ref().unwrap().text, fixture.saved);
            desktop.app_worker.as_mut().unwrap().shutdown().unwrap();
        });
    })
    .unwrap();
    fixture.unchanged();
}

#[gpui_kit::test]
fn accepted_review_draft_and_hide_navigation_clear_the_source_panel(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let (_, event) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::ScopedNote {
                scope: KnowledgeScope::All,
                path: "current.md".into(),
            },
        ),
    );
    let AppEvent::Note(note) = event else {
        panic!("note");
    };
    let (_, event) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::NoteProvenance("current.md".into()),
        ),
    );
    let AppEvent::NoteProvenance(provenance) = event else {
        panic!("provenance");
    };
    let (_, event) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(DraftRequest {
                inbox_knowledge: None,
                inbox_source: None,
                action_changes: Vec::new(),
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Synthetic review".into(),
                changes: vec![DraftNoteChange::Create {
                    path: "future.md".into(),
                    text: "Future reviewed text\r\n".into(),
                }],
                sources: vec![],
            }),
        ),
    );
    let AppEvent::Proposal(record) = event else {
        panic!("review");
    };
    worker.shutdown().unwrap();
    cx.update(gpui_kit::component::init);
    let window = cx.add_window(|_, _| gpui_kit::Empty);
    let desktop = cx
        .update_window(window.into(), |_, window, cx| {
            cx.new(|cx| {
                Desktop::new(
                    fixture.base.path().join("data"),
                    fixture.config(),
                    (LayoutState::default(), Loaded::Missing),
                    window,
                    cx,
                )
            })
        })
        .unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        desktop.update(cx, |desktop, cx| {
            for (transition, expected) in [
                (
                    simple::EditorTransition::Review(record.draft.id),
                    Some(DocRef::Proposal(record.draft.id)),
                ),
                (simple::EditorTransition::Draft(None), Some(DocRef::Draft)),
                (simple::EditorTransition::Hide, None),
            ] {
                let ai = desktop.ai.as_mut().unwrap();
                ai.ready = true;
                ai.vault_bound = true;
                ai.review = None;
                ai.draft = None;
                let (id, _) = ai.open_evidence("current.md".into(), KnowledgeScope::All);
                ai.apply(id, AppEvent::Note(note.clone()));
                ai.provenance = Some(*provenance.clone());
                desktop.open_doc = Some(DocRef::Evidence);
                desktop.provenance_open = true;
                desktop.sync_provenance_widgets(window, cx);
                assert!(!desktop.provenance_quotes.is_empty());
                desktop.simple_leave(transition, cx);
                assert_eq!(desktop.open_doc, expected);
                assert!(!desktop.provenance_open);
                assert!(desktop.provenance_quotes.is_empty());
                assert!(desktop.ai.as_ref().unwrap().provenance.is_none());
            }
            desktop.app_worker.as_mut().unwrap().shutdown().unwrap();
        });
    })
    .unwrap();
    fixture.unchanged();
    assert!(!fixture.base.path().join("vault/future.md").exists());
}

#[gpui_kit::test]
fn rendered_saved_sources_panel_scrolls_to_later_exact_quote_copy(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::{
        InputEvent as _, ScrollDelta, ScrollWheelEvent, VisualTestContext, test::TestWindowExt,
    };
    let mut fixture = Fixture::new();
    let full = VaultCitation {
        note_id: Uuid::parse_str("676451a3-dddb-4e3d-a7c8-c01a52d2356f").unwrap(),
        sha256: Sha256::digest(SOURCE.as_bytes()).into(),
        start_byte: 0,
        end_byte: SOURCE.len(),
        quote: SOURCE.into(),
    };
    let mut citations: Vec<_> = (3..14)
        .map(|start| VaultCitation {
            start_byte: start,
            end_byte: start + 1,
            quote: SOURCE[start..start + 1].into(),
            ..full.clone()
        })
        .collect();
    citations.push(full);
    fixture.saved = format!(
        "---\r\nbrn_provenance: {}\r\n---\r\nSaved knowledge õäöü\r\n",
        serde_json::to_string(&citations).unwrap()
    );
    fs::write(fixture.base.path().join("vault/current.md"), &fixture.saved).unwrap();
    let mut worker = fixture.worker();
    let (_, event) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::ScopedNote {
                scope: KnowledgeScope::All,
                path: "current.md".into(),
            },
        ),
    );
    let AppEvent::Note(note) = event else {
        panic!("note");
    };
    let (_, event) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::NoteProvenance("current.md".into()),
        ),
    );
    let AppEvent::NoteProvenance(provenance) = event else {
        panic!("provenance");
    };
    assert_eq!(provenance.citations.len(), 12);
    assert!(
        provenance
            .citations
            .iter()
            .all(|citation| citation.outcome == CitationOutcome::Matched)
    );
    worker.shutdown().unwrap();
    cx.update(gpui_kit::component::init);
    let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = captured.clone();
    let data = fixture.base.path().join("data");
    let config = fixture.config();
    let window = cx.add_window(move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut desktop = Desktop::new(
                data,
                config,
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            let (id, _) = ai.open_evidence("current.md".into(), KnowledgeScope::All);
            ai.apply(id, AppEvent::Note(note));
            ai.provenance = Some(*provenance);
            desktop.open_doc = Some(DocRef::Evidence);
            desktop.centre_tab = CentreTab::Document;
            desktop.provenance_open = true;
            desktop.sync_provenance_widgets(window, cx);
            desktop
        });
        *capture.borrow_mut() = Some(desktop.clone());
        Root::new(desktop, window, cx)
    });
    let desktop = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let panel = window.find("saved-sources-scroll");
        assert!(panel.visible());
        let before = desktop.read(cx).provenance_scroll.offset();
        let position = panel.bounds().origin + point(px(3.), px(3.));
        window.dispatch_event(
            ScrollWheelEvent {
                position,
                delta: ScrollDelta::Pixels(point(px(0.), px(-100_000.))),
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.render_frame(cx);
        let after = desktop.read(cx).provenance_scroll.offset();
        assert_ne!(
            after, before,
            "native wheel input must move the bounded saved-sources panel"
        );
        assert!(after.y < before.y);
        assert!(
            window.find("copy-source-quote-11").visible(),
            "last saved citation must be reachable after scrolling"
        );
        window.click("copy-source-quote-11", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(SOURCE)
        );
        desktop.update(cx, |desktop, _| {
            desktop.app_worker.as_mut().unwrap().shutdown().unwrap()
        });
    });
    fixture.unchanged();
}
