use super::*;
use brn_workflow::{
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    knowledge::{
        CitationOutcome, EdgeEndpoint, EdgeEvidence, EdgeOrigin, EvidenceEndpoint, IdentityOutcome,
        LinkEvidence, NoteEdge, NoteIdentityInfo, NoteLinkOutcome, NoteLinks, NoteProvenance,
        RelationshipPage, ResolvedCitation, ResolvedNoteLink, VaultCitation,
    },
    library::KnowledgeScope,
};
use std::{fs, time::Instant};

const EXACT: &str = "\u{feff}Exact 日本語 õäöü\r\nsecond λ\r";
struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        fs::create_dir(base.path().join("data")).unwrap();
        fs::create_dir(base.path().join("vault")).unwrap();
        fs::write(base.path().join("vault/current.md"), "Current saved\r\n").unwrap();
        fs::write(
            base.path().join("vault/source.md"),
            "---\r\nbrn_kind: source\r\n---\r\nOriginal\r\n",
        )
        .unwrap();
        Self(base)
    }
    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: Some(self.0.path().join("vault")),
            credentials_dir: Some(self.0.path().join("credentials")),
            model_dir: None,
        }
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(self.0.path().join("data"), self.config()).unwrap();
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
}
fn reply(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    loop {
        let (actual, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if actual == id {
            return event;
        }
    }
}
fn links(path: &str) -> NoteLinks {
    NoteLinks {
        source: NoteIdentityInfo {
            path: path.into(),
            note_id: Some(Uuid::new_v4()),
            sha256: [1; 32],
        },
        source_outcome: Some(IdentityOutcome::Unique),
        links: vec![ResolvedNoteLink {
            destination: "brn://note/00000000-0000-4000-8000-000000000001".into(),
            evidence: vec![
                LinkEvidence {
                    start_byte: 0,
                    end_byte: EXACT.len(),
                    quote: EXACT.into(),
                },
                LinkEvidence {
                    start_byte: 40,
                    end_byte: 40 + "Used definition λ\r\n".len(),
                    quote: "Used definition λ\r\n".into(),
                },
            ],
            target_path: Some("archive/source.md".into()),
            outcome: NoteLinkOutcome::Ambiguous,
            matches: vec![NoteIdentityInfo {
                path: "archive/source.md".into(),
                note_id: Some(Uuid::new_v4()),
                sha256: [2; 32],
            }],
            issues: vec![],
        }],
        issues: vec![],
    }
}
fn page() -> RelationshipPage {
    let source = EdgeEndpoint {
        path: "current.md".into(),
        note_id: Uuid::new_v4(),
        sha256: [1; 32],
    };
    RelationshipPage {
        scope: KnowledgeScope::All,
        offset: 0,
        total: 25,
        edges: (0..25)
            .map(|i| NoteEdge {
                source: source.clone(),
                target: EdgeEndpoint {
                    path: format!("target-{i}.md"),
                    note_id: Uuid::new_v4(),
                    sha256: [2; 32],
                },
                origin: if i == 24 {
                    EdgeOrigin::InferredProvenance
                } else {
                    EdgeOrigin::ExplicitLink
                },
                evidence: vec![EdgeEvidence {
                    endpoint: if i == 24 {
                        EvidenceEndpoint::Target
                    } else {
                        EvidenceEndpoint::Source
                    },
                    start_byte: 0,
                    end_byte: EXACT.len(),
                    quote: EXACT.into(),
                }],
            })
            .collect(),
        issues: vec![],
        duplicates: vec![],
    }
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
                    .child(relationships::quote_widget(&self.0)),
            )
            .child(relationships::copy_button("copy-inspected-proof", EXACT))
    }
}
#[gpui_kit::test]
fn actual_inspection_quote_refuses_typing_and_copy_preserves_exact_bytes(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::{EntityInputHandler, VisualTestContext, test::TestWindowExt};
    cx.update(gpui_kit::component::init);
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    let window = cx.add_window(move |window, cx| {
        let editor = cx.new(|cx| EditorState::new(window, cx).default_value(EXACT));
        *saved.borrow_mut() = Some(editor.clone());
        let probe = cx.new(|_| QuoteProbe(editor));
        Root::new(probe, window, cx)
    });
    let editor = capture.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        editor.update(cx, |editor, cx| {
            editor.focus(window, cx);
            let end = EXACT.encode_utf16().count();
            editor.replace_text_in_range(Some(end..end), "blocked", window, cx);
            assert_eq!(editor.value().as_ref(), EXACT);
        });
        window.click("copy-inspected-proof", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(EXACT)
        );
    });
}

#[gpui_kit::test]
fn saved_link_selection_uses_one_persistent_quote_without_touching_unsaved_editor(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let AppEvent::Editor(view) = reply(&worker, AppCommand::OpenEditor("current.md".into())) else {
        panic!("editor")
    };
    worker.shutdown().unwrap();
    cx.update(gpui_kit::component::init);
    let window = cx.add_window(|_, _| gpui_kit::Empty);
    cx.update_window(window.into(), |_, window, cx| {
        let desktop = cx.new(|cx| {
            Desktop::new(
                fixture.0.path().join("data"),
                fixture.config(),
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            )
        });
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            let (id, _) = ai.open_editor("current.md".into());
            ai.apply(id, AppEvent::Editor(view));
            ai.editor
                .as_mut()
                .unwrap()
                .edit("Unsaved λ\r\n".into(), Instant::now())
                .unwrap();
            desktop.open_doc = Some(DocRef::SavedNote);
            desktop.provenance_open = true;
            desktop.inspect_saved_links(cx);
            assert!(desktop.provenance_open);
            assert!(desktop.saved_links.open);
            desktop.ai.as_mut().unwrap().links = Some(links("current.md"));
            let entity = desktop.saved_links.quote.entity_id();
            desktop.sync_relationship_widgets(window, cx);
            assert_eq!(desktop.saved_links.quote.read(cx).value().as_ref(), EXACT);
            assert_eq!(
                desktop.ai.as_ref().unwrap().editor.as_ref().unwrap().text,
                "Unsaved λ\r\n"
            );
            desktop.close_saved_links(cx);
            desktop.sync_relationship_widgets(window, cx);
            assert_eq!(desktop.saved_links.quote.entity_id(), entity);
            assert_eq!(desktop.saved_links.quote.read(cx).value().as_ref(), "");
            assert!(
                !desktop
                    .ai
                    .as_ref()
                    .unwrap()
                    .editor
                    .as_ref()
                    .unwrap()
                    .can_leave()
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn selected_relationship_proof_is_persistent_and_invalidation_clears_exact_widget(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    cx.update(gpui_kit::component::init);
    let window = cx.add_window(|_, _| gpui_kit::Empty);
    cx.update_window(window.into(), |_, window, cx| {
        let desktop = cx.new(|cx| {
            Desktop::new(
                fixture.0.path().join("data"),
                fixture.config(),
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            )
        });
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            ai.knowledge_scope = KnowledgeScope::All;
            ai.relationships = Some(page());
            desktop.relationships.open = true;
            let entity = desktop.relationships.quote.entity_id();
            desktop.sync_relationship_widgets(window, cx);
            assert_eq!(desktop.relationships.quote.read(cx).value().as_ref(), EXACT);
            desktop.ai.as_mut().unwrap().clear_relationships();
            desktop.sync_relationship_widgets(window, cx);
            assert_eq!(desktop.relationships.quote.entity_id(), entity);
            assert_eq!(desktop.relationships.quote.read(cx).value().as_ref(), "");
        });
    })
    .unwrap();
}

struct PaneProbe {
    desktop: Entity<Desktop>,
    links: bool,
}
impl Render for PaneProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.desktop.update(cx, |desktop, cx| {
            let pane = if self.links {
                desktop.render_saved_links(cx)
            } else {
                desktop.render_relationship_page(cx)
            };
            div().size_full().children(pane)
        })
    }
}
fn scroll_pane(visual: &mut gpui_kit::VisualTestContext, id: &'static str) {
    use gpui_kit::{ScrollDelta, test::TestWindowExt};
    visual.update(|window, cx| {
        window.render_frame(cx);
        let panel = window.find(id);
        assert!(panel.visible());
        let position = panel.bounds().origin + point(px(3.), px(3.));
        wheel_at(
            window,
            cx,
            position,
            ScrollDelta::Pixels(point(px(0.), px(-100_000.))),
        );
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
    });
}
fn wheel_at(
    window: &mut Window,
    cx: &mut App,
    position: gpui_kit::Point<gpui_kit::Pixels>,
    delta: gpui_kit::ScrollDelta,
) {
    use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollWheelEvent, test::TestWindowExt};
    window.dispatch_event(
        MouseMoveEvent {
            position,
            ..Default::default()
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
    window.dispatch_event(
        ScrollWheelEvent {
            position,
            delta,
            ..Default::default()
        }
        .to_platform_input(),
        cx,
    );
}
fn scroll_to_target(
    visual: &mut gpui_kit::VisualTestContext,
    pane: &'static str,
    target: &'static str,
) {
    use gpui_kit::{ScrollDelta, test::TestWindowExt};
    visual.update(|window, cx| {
        window.render_frame(cx);
        let panel = window.find(pane);
        let target = window.find(target);
        wheel_at(
            window,
            cx,
            panel.bounds().origin + point(px(3.), px(3.)),
            ScrollDelta::Pixels(point(
                px(0.),
                panel.bounds().origin.y + px(15.) - target.bounds().origin.y,
            )),
        );
    });
    visual.run_until_parked();
    visual.update(|window, cx| window.render_frame(cx));
}

#[gpui_kit::test]
fn rendered_saved_links_scroll_switches_definition_and_preserves_independent_sources_state(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::{EntityInputHandler, VisualTestContext, test::TestWindowExt};
    let fixture = Fixture::new();
    cx.update(gpui_kit::component::init);
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    let data = fixture.0.path().join("data");
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
            ai.links = Some(links("source.md"));
            desktop.saved_links.open = true;
            desktop.sync_relationship_widgets(window, cx);
            desktop
        });
        *saved.borrow_mut() = Some(desktop.clone());
        let probe = cx.new(|_| PaneProbe {
            desktop,
            links: true,
        });
        Root::new(probe, window, cx)
    });
    let desktop = capture.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    let entity = visual.update(|_, cx| desktop.read(cx).saved_links.quote.entity_id());
    let before = visual.update(|_, cx| desktop.read(cx).saved_links.scroll.offset());
    scroll_to_target(&mut visual, "saved-links-scroll", "next-link-proof");
    visual.update(|window, cx| {
        assert!(desktop.read(cx).saved_links.scroll.offset().y < before.y);
        window.click("next-link-proof", cx);
        window.render_frame(cx);
        let quote = desktop.read(cx).saved_links.quote.clone();
        assert_eq!(quote.entity_id(), entity);
        assert_eq!(quote.read(cx).value().as_ref(), "Used definition λ\r\n");
        quote.update(cx, |editor, cx| {
            editor.focus(window, cx);
            editor.replace_text_in_range(Some(0..0), "blocked", window, cx);
            assert_eq!(editor.value().as_ref(), "Used definition λ\r\n");
        });
    });
    scroll_pane(&mut visual, "saved-links-scroll");
    visual.update(|window, cx| {
        window.click("copy-saved-link-proof", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("Used definition λ\r\n")
        );
        desktop.update(cx, |desktop, cx| {
            desktop.clear_saved_sources();
            assert!(desktop.ai.as_ref().unwrap().links.is_some());
            assert!(desktop.saved_links.open);
            desktop.close_saved_links(cx);
            desktop.sync_relationship_widgets(window, cx);
            assert!(!desktop.saved_links.open);
            assert!(desktop.ai.as_ref().unwrap().links.is_none());
            assert_eq!(desktop.saved_links.quote.read(cx).value().as_ref(), "");
        });
    });
}

#[gpui_kit::test]
fn rendered_relationship_selection_reaches_last_edge_and_page_controls_use_state_offsets(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::{VisualTestContext, test::TestWindowExt};
    const LAST: &str = "\u{feff}Last exact 日本語\r\nλ\r";
    let fixture = Fixture::new();
    cx.update(gpui_kit::component::init);
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    let data = fixture.0.path().join("data");
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
            ai.knowledge_scope = KnowledgeScope::All;
            let mut page = page();
            page.total = 50;
            page.edges[24].evidence[0].quote = LAST.into();
            page.edges[24].evidence[0].end_byte = LAST.len();
            ai.relationships = Some(page);
            desktop.relationships.open = true;
            desktop.sync_relationship_widgets(window, cx);
            desktop
        });
        *saved.borrow_mut() = Some(desktop.clone());
        let probe = cx.new(|_| PaneProbe {
            desktop,
            links: false,
        });
        Root::new(probe, window, cx)
    });
    let desktop = capture.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    let entity = visual.update(|_, cx| desktop.read(cx).relationships.quote.entity_id());
    for _ in 0..24 {
        visual.update(|window, cx| {
            window.render_frame(cx);
            window.click("next-relationship-edge", cx);
        });
        visual.run_until_parked();
    }
    let before = visual.update(|_, cx| desktop.read(cx).relationships.scroll.offset());
    scroll_pane(&mut visual, "relationships-scroll");
    visual.update(|window, cx| {
        let quote = desktop.read(cx).relationships.quote.clone();
        assert_eq!(quote.entity_id(), entity);
        assert_eq!(quote.read(cx).value().as_ref(), LAST);
        assert!(desktop.read(cx).relationships.scroll.offset().y < before.y);
        window.click("copy-relationship-proof", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(LAST)
        );
        window.click("relationship-page-next", cx);
        assert!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .pending
                .values()
                .any(|pending| matches!(
                    pending,
                    crate::ai::Pending::Relationships {
                        scope: KnowledgeScope::All,
                        offset: 25,
                        limit: 25,
                        ..
                    }
                ))
        );
        assert!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .relationships
                .is_none()
        );
        desktop.update(cx, |desktop, cx| {
            desktop.sync_relationship_widgets(window, cx)
        });
        assert_eq!(quote.read(cx).value().as_ref(), "");
        window.render_frame(cx);
        window.click("close-relationships", cx);
        assert!(!desktop.read(cx).relationships.open);
    });
}

fn full_document_at_minimum_height(cx: &mut gpui_kit::TestAppContext, read_only: bool) {
    use gpui_kit::{ScrollDelta, VisualTestContext, test::TestWindowExt};
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let event = reply(
        &worker,
        if read_only {
            AppCommand::ScopedNote {
                scope: KnowledgeScope::All,
                path: "source.md".into(),
            }
        } else {
            AppCommand::OpenEditor("current.md".into())
        },
    );
    worker.shutdown().unwrap();
    cx.update(gpui_kit::component::init);
    let data = fixture.0.path().join("data");
    let config = fixture.config();
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    let window = cx.open_window(
        size(px(layout::WINDOW_MIN), px(layout::WINDOW_MIN)),
        move |window, cx| {
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
                let path = if read_only { "source.md" } else { "current.md" };
                let (id, _) = if read_only {
                    ai.open_evidence(path.into(), KnowledgeScope::All)
                } else {
                    ai.open_editor(path.into())
                };
                ai.apply(id, event);
                if !read_only {
                    ai.editor
                        .as_mut()
                        .unwrap()
                        .edit("Unsaved λ\r\n".into(), Instant::now())
                        .unwrap();
                }
                ai.links = Some(links(path));
                ai.provenance = Some(NoteProvenance {
                    inbox_source: None,
                    path: path.into(),
                    citations: vec![ResolvedCitation {
                        citation: VaultCitation {
                            note_id: Uuid::new_v4(),
                            sha256: [7; 32],
                            start_byte: 0,
                            end_byte: EXACT.len(),
                            quote: EXACT.into(),
                        },
                        outcome: CitationOutcome::Matched,
                        matches: vec![],
                        issues: vec![],
                    }],
                });
                desktop.simple_note_path = Some(path.into());
                desktop.open_doc = Some(if read_only {
                    DocRef::Evidence
                } else {
                    DocRef::SavedNote
                });
                desktop.centre_tab = CentreTab::Document;
                desktop.provenance_open = true;
                desktop.saved_links.open = true;
                if read_only {
                    let text = desktop
                        .ai
                        .as_ref()
                        .unwrap()
                        .evidence
                        .as_ref()
                        .unwrap()
                        .note
                        .as_ref()
                        .unwrap()
                        .text
                        .clone();
                    desktop
                        .evidence_editor
                        .update(cx, |editor, cx| editor.set_value(text, window, cx));
                } else {
                    desktop.note_editor.update(cx, |editor, cx| {
                        editor.set_value("Unsaved λ\r\n", window, cx)
                    });
                }
                desktop.sync_provenance_widgets(window, cx);
                desktop.sync_relationship_widgets(window, cx);
                desktop
            });
            *saved.borrow_mut() = Some(desktop.clone());
            Root::new(desktop, window, cx)
        },
    );
    let desktop = capture.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    let body_id = if read_only {
        "evidence-note-body"
    } else {
        "saved-note-body"
    };
    visual.update(|window, cx| {
        window.render_frame(cx);
        let body = window.find(body_id);
        let position = body.bounds().origin + point(px(3.), px(3.));
        wheel_at(window, cx, position, ScrollDelta::Pixels(point(px(0.), px(-100_000.))));
        window.render_frame(cx);
        let control = window.find("close-saved-links");
        assert!(control.visible() && control.bounds().center().y < window.viewport_size().height,
            "Links Close must be reachable in the full {read_only:?} document at the supported minimum height: {:?}; body {:?}; offset {:?}", control.bounds(), body.bounds(), desktop.read(cx).document_scroll.offset());
    });
    scroll_pane(&mut visual, "saved-links-scroll");
    visual.update(|window, cx| {
        window.click("copy-saved-link-proof", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(EXACT)
        );
    });
    scroll_to_target(&mut visual, body_id, "close-saved-sources");
    visual.update(|window, cx| {
        let control = window.find("close-saved-sources");
        assert!(
            control.visible(),
            "Sources Close after document wheel {:?}, body {:?}, document offset {:?}",
            control.bounds(),
            window.find(body_id).bounds(),
            desktop.read(cx).document_scroll.offset()
        );
    });
    let outer_offset = visual.update(|_, cx| desktop.read(cx).document_scroll.offset());
    scroll_pane(&mut visual, "saved-sources-scroll");
    visual.update(|window, cx| {
        assert_eq!(desktop.read(cx).document_scroll.offset(), outer_offset);
        let control = window.find("copy-source-quote-0");
        assert!(
            control.visible(),
            "Source Copy {:?}, panel {:?}, body {:?}, document offset {:?}, source offset {:?}",
            control.bounds(),
            window.find("saved-sources-scroll").bounds(),
            window.find(body_id).bounds(),
            desktop.read(cx).document_scroll.offset(),
            desktop.read(cx).provenance_scroll.offset()
        );
        window.click("copy-source-quote-0", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(EXACT)
        );
        window.click("close-saved-sources", cx);
        assert!(!desktop.read(cx).provenance_open);
        assert!(desktop.read(cx).saved_links.open);
        assert!(desktop.read(cx).ai.as_ref().unwrap().links.is_some());
    });
    scroll_pane(&mut visual, body_id);
    visual.update(|window, cx| {
        window.click("close-saved-links", cx);
        assert!(!desktop.read(cx).saved_links.open);
        if read_only {
            assert_eq!(
                desktop.read(cx).evidence_editor.read(cx).value().as_ref(),
                "---\r\nbrn_kind: source\r\n---\r\nOriginal\r\n"
            );
        } else {
            let ai = desktop.read(cx).ai.as_ref().unwrap();
            assert_eq!(ai.editor.as_ref().unwrap().text, "Unsaved λ\r\n");
            assert!(!ai.editor.as_ref().unwrap().can_leave());
            assert_eq!(
                desktop.read(cx).note_editor.read(cx).value().as_ref(),
                "Unsaved λ\r\n"
            );
        }
    });
    assert_eq!(
        fs::read(fixture.0.path().join("vault/current.md")).unwrap(),
        b"Current saved\r\n"
    );
    assert_eq!(
        fs::read(fixture.0.path().join("vault/source.md")).unwrap(),
        b"---\r\nbrn_kind: source\r\n---\r\nOriginal\r\n"
    );
}

#[gpui_kit::test]
fn full_saved_document_inspection_controls_are_reachable_at_minimum_height(
    cx: &mut gpui_kit::TestAppContext,
) {
    full_document_at_minimum_height(cx, false);
}

#[gpui_kit::test]
fn full_evidence_document_inspection_controls_are_reachable_at_minimum_height(
    cx: &mut gpui_kit::TestAppContext,
) {
    full_document_at_minimum_height(cx, true);
}
