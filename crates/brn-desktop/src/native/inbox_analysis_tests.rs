use super::*;
use brn_workflow::{
    Provider, ReasoningEffort, Selection,
    app_worker::{AppCommand, AppEvent},
    chat_worker::ChatEvent,
    inbox_actions::{InboxActionAnalysis, InboxActionCapture, InboxActionJob},
    proposals::ProposalSource,
};
use gpui_kit::{EntityInputHandler, TestSupportExt, VisualTestContext, test::TestWindowExt};

struct AnalysisProbe(Entity<Desktop>);
impl Render for AnalysisProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            let pane = if desktop.open_doc == Some(DocRef::Evidence) {
                desktop.render_evidence_document(cx)
            } else {
                desktop.render_inbox(cx)
            };
            div()
                .id("analysis-test-pane")
                .size_full()
                .flex()
                .flex_col()
                .child(pane)
                .test_support()
        })
    }
}

#[gpui_kit::test]
fn saved_inbox_provenance_entry_uses_guarded_navigation_without_starting_inference(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, source) = crate::ai::inbox_analysis_state_tests::source_fixture();
    let mut worker = brn_workflow::app_worker::AppWorker::start(
        owner.path().join("data"),
        brn_workflow::app::AppConfig {
            vault_root: Some(owner.path().join("vault")),
            credentials_dir: Some(owner.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(10))
            .unwrap()
            .1,
        AppEvent::Ready { .. }
    ));
    let operation = Uuid::new_v4();
    worker
        .submit(
            operation,
            AppCommand::NoteProvenance(source.source.path.clone()),
        )
        .unwrap();
    let provenance = loop {
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if id == operation {
            let AppEvent::NoteProvenance(provenance) = event else {
                panic!("saved Source provenance");
            };
            break provenance;
        }
    };
    assert!(provenance.inbox_source.is_some());
    worker.shutdown().unwrap();
    let (window, desktop) = open(cx, &owner, source.clone());
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.update(|window, cx| {
        desktop.update(cx, |d, cx| {
            assert!(d.ai.as_mut().unwrap().begin_draft(None));
            d.open_doc = Some(DocRef::Draft);
            d.sync_draft_widgets(window, cx);
            d.draft_title.update(cx, |input, cx| {
                input.set_value("Unsubmitted retained title õ", window, cx)
            });
            d.simple_leave(
                simple::EditorTransition::AnalyzeInboxSource(source.source.path.clone()),
                cx,
            );
            assert_eq!(d.open_doc, Some(DocRef::Draft));
            assert!(d.simple_transition.is_none());
            assert_eq!(
                d.ai.as_ref().unwrap().draft.as_ref().unwrap().title,
                "Unsubmitted retained title õ"
            );
            assert!(d.ai.as_mut().unwrap().discard_draft());
            let ai = d.ai.as_mut().unwrap();
            ai.close_inbox();
            ai.evidence = Some(crate::ai::EvidenceDocument {
                path: source.source.path.clone(),
                scope: brn_workflow::library::KnowledgeScope::Source,
                note: Some(brn_workflow::vault::NoteText {
                    text: source.text.clone(),
                    sha256: source.source.fingerprint.sha256,
                }),
            });
            ai.provenance = Some(*provenance);
            d.open_doc = Some(DocRef::Evidence);
            d.provenance_open = true;
            d.evidence_editor.update(cx, |editor, cx| {
                editor.set_value(source.text.clone(), window, cx)
            });
            d.sync_provenance_widgets(window, cx);
            cx.notify();
        })
    });
    visual.run_until_parked();
    reach(&mut visual, "analyze-saved-inbox-source");
    visual.update(|window, cx| {
        window.click("analyze-saved-inbox-source", cx);
        desktop.update(cx, |d, cx| {
            assert_eq!(d.open_doc, Some(DocRef::Inbox));
            assert!(d.ai.as_ref().unwrap().inbox_analysis.request.is_none());
            assert_eq!(
                d.ai.as_ref().unwrap().inbox_analysis.source_path.as_deref(),
                Some(source.source.path.as_str())
            );
            d.sync_inbox_widgets(window, cx);
            assert_eq!(
                d.inbox.analysis_source_path.read(cx).value().as_ref(),
                source.source.path
            );
        });
    });
    assert_eq!(
        std::fs::read(owner.path().join("vault/source.md")).unwrap(),
        source.text.as_bytes()
    );
    assert_eq!(
        std::fs::read_dir(owner.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
}
fn open(
    cx: &mut gpui_kit::TestAppContext,
    owner: &tempfile::TempDir,
    source: ProposalSource,
) -> (gpui_kit::WindowHandle<Root>, Entity<Desktop>) {
    cx.update(gpui_kit::component::init);
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    let data = owner.path().join("data");
    let config = brn_workflow::app::AppConfig {
        vault_root: Some(owner.path().join("vault")),
        credentials_dir: Some(owner.path().join("credentials")),
        model_dir: None,
    };
    let window = cx.open_window(
        size(px(layout::WINDOW_MIN), px(layout::WINDOW_MIN)),
        move |window, cx| {
            let desktop = cx.new(|cx| {
                let mut d = Desktop::new(
                    data,
                    config,
                    (LayoutState::default(), Loaded::Missing),
                    window,
                    cx,
                );
                d.app_worker.take().unwrap().shutdown().unwrap();
                let ai = d.ai.as_mut().unwrap();
                ai.ready = true;
                ai.vault_bound = true;
                ai.pending.clear();
                ai.open_inbox();
                ai.pending.clear();
                let (id, _) = ai
                    .inspect_inbox_analysis_source(source.source.path.clone())
                    .unwrap();
                ai.apply(id, AppEvent::ProposalSource(Box::new(source.clone())));
                d.inbox.analysis_path_target = Some(source.source.path.clone());
                d.open_doc = Some(DocRef::Inbox);
                d.centre_tab = CentreTab::Document;
                d.sync_inbox_widgets(window, cx);
                d
            });
            *saved.borrow_mut() = Some(desktop.clone());
            let probe = cx.new(|_| AnalysisProbe(desktop));
            Root::new(probe, window, cx)
        },
    );
    (window, capture.borrow().clone().unwrap())
}
fn reach(visual: &mut VisualTestContext, id: &'static str) {
    use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollDelta, ScrollWheelEvent};
    visual.update(|window, cx| {
        window.render_frame(cx);
        let pane = window.find("analysis-test-pane");
        let target = window.find(id);
        let position = pane.bounds().origin + point(px(3.), px(3.));
        let dy = pane.bounds().origin.y + px(20.) - target.bounds().origin.y;
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
                delta: ScrollDelta::Pixels(point(px(0.), dy)),
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let target = window.find(id);
        assert!(
            target.visible()
                && target.bounds().center().y >= px(0.)
                && target.bounds().center().y < window.viewport_size().height,
            "{id} must be reachable"
        );
    });
}

#[gpui_kit::test]
fn complete_saved_source_widget_copy_and_explicit_start_preserve_proof_and_later_path(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, source) = crate::ai::inbox_analysis_state_tests::source_fixture();
    let (window, desktop) = open(cx, &owner, source.clone());
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    reach(&mut visual, "inbox-analysis-copy-source");
    visual.update(|window, cx| {
        window.click("inbox-analysis-copy-source", cx);
        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some(source.text.clone())
        );
        desktop.update(cx, |d, cx| {
            let editor = d.inbox.analysis_source.clone();
            editor.update(cx, |editor, cx| {
                editor.replace_text_in_range(Some(0..0), "forbidden edit", window, cx);
                assert_eq!(editor.value().as_ref(), source.text);
            });
            assert!(d.ai.as_ref().unwrap().inbox_analysis.request.is_none());
            let ai = d.ai.as_mut().unwrap();
            ai.selection = Some(Selection {
                provider: Provider::Chatgpt,
                model: "synthetic-luna".into(),
            });
            ai.effort = Some(ReasoningEffort::Low);
            d.inbox
                .analysis_source_path
                .update(cx, |input, cx| input.set_value("other.md", window, cx));
            d.sync_inbox_widgets(window, cx);
            assert_eq!(
                d.inbox.analysis_source_path.read(cx).value().as_ref(),
                "other.md"
            );
            cx.notify();
        });
    });
    visual.run_until_parked();
    reach(&mut visual, "inbox-analysis-start");
    visual.update(|window, cx| {
        window.click("inbox-analysis-start", cx);
        assert!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .inbox_analysis
                .request
                .is_none()
        );
        desktop.update(cx, |d, cx| {
            d.inbox.analysis_source_path.update(cx, |input, cx| {
                input.set_value(source.source.path.clone(), window, cx)
            });
            cx.notify();
        });
    });
    visual.run_until_parked();
    reach(&mut visual, "inbox-analysis-start");
    visual.update(|window, cx| {
        window.click("inbox-analysis-start", cx);
        let ai = desktop.read(cx).ai.as_ref().unwrap();
        let request = ai.inbox_analysis.request.as_ref().unwrap();
        assert_eq!(*request.source, source);
        assert_eq!(request.selection.model, "synthetic-luna");
        assert_eq!(request.effort, ReasoningEffort::Low);
        assert_eq!(
            request.purpose,
            brn_workflow::inbox_actions::InboxAnalysisPurpose::KnowledgeAndActions
        );
        // The deliberately closed worker rejects submission; no inference is run.
        assert!(ai.active.is_none());
        assert_eq!(
            std::fs::read(owner.path().join("vault/source.md")).unwrap(),
            source.text.as_bytes()
        );
        assert_eq!(
            std::fs::read_dir(owner.path().join("credentials"))
                .unwrap()
                .count(),
            0
        );
    });
}

#[gpui_kit::test]
fn retained_source_and_unsaved_partial_are_distinct_copyable_and_readonly(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, source) = crate::ai::inbox_analysis_state_tests::source_fixture();
    let (window, desktop) = open(cx, &owner, source.clone());
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    let exact = "\u{feff}Provisional õ 日本語\r\n🦀";
    visual.update(|window, cx| {
        desktop.update(cx, |d, cx| {
            let ai = d.ai.as_mut().unwrap();
            ai.selection = Some(Selection {
                provider: Provider::Chatgpt,
                model: "synthetic-luna".into(),
            });
            ai.effort = Some(ReasoningEffort::Medium);
            let (id, command) = ai.analyze_inbox_source().unwrap();
            let AppCommand::AnalyzeInboxActions(request) = command else {
                panic!("typed analysis");
            };
            ai.apply(
                id,
                AppEvent::Chat(ChatEvent::Text {
                    id,
                    generation: request.generation,
                    text: exact.into(),
                }),
            );
            let record = InboxActionAnalysis {
                job: InboxActionJob {
                    capture: InboxActionCapture {
                        purpose: request.purpose,
                        id,
                        conversation: request.conversation,
                        source: source.source.clone(),
                        source_text: source.text.clone(),
                        provider: "chatgpt".into(),
                        model: "synthetic-luna".into(),
                        effort: "medium".into(),
                    },
                    question: "Synthetic workflow-owned prompt".into(),
                    created_at_ms: 1,
                },
                turn: None,
                proposals: vec![],
                needs_semantic_review: true,
            };
            let (lookup, _) = ai.inspect_inbox_analysis(id).unwrap();
            ai.apply(
                lookup,
                AppEvent::InboxActionAnalysis(Box::new(record.clone())),
            );
            ai.apply(
                id,
                AppEvent::Chat(ChatEvent::PersistenceFailed {
                    id,
                    generation: request.generation,
                    partial: exact.into(),
                    error: brn_workflow::WorkflowError::msg("synthetic finalization refusal"),
                }),
            );
            // Restore the retained record after the automatic fresh lookup was prepared.
            let lookup = ai
                .pending
                .iter()
                .find_map(|(id, pending)| {
                    matches!(pending, crate::ai::Pending::InboxAnalysis(_)).then_some(*id)
                })
                .unwrap();
            ai.apply(lookup, AppEvent::InboxActionAnalysis(Box::new(record)));
            let (inspect, _) = ai
                .inspect_inbox_analysis_source("missing.md".into())
                .unwrap();
            ai.apply(
                inspect,
                AppEvent::Failed(brn_workflow::WorkflowError::msg("new Source unavailable")),
            );
            assert!(ai.inbox_analysis.source.is_none());
            assert!(!ai.can_analyze_inbox_source());
            d.sync_inbox_widgets(window, cx);
            cx.notify();
        })
    });
    visual.run_until_parked();
    reach(&mut visual, "inbox-analysis-copy-retained-source");
    visual.update(|window, cx| {
        window.click("inbox-analysis-copy-retained-source", cx);
        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some(source.text.clone())
        );
        desktop.update(cx, |d, cx| {
            d.inbox.analysis_retained_source.update(cx, |editor, cx| {
                editor.replace_text_in_range(Some(0..0), "forbidden", window, cx);
                assert_eq!(editor.value().as_ref(), source.text);
            })
        });
    });
    reach(&mut visual, "inbox-analysis-copy-answer");
    visual.update(|window, cx| {
        window.click("inbox-analysis-copy-answer", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(exact)
        );
        desktop.update(cx, |d, cx| {
            d.inbox.analysis_answer.update(cx, |editor, cx| {
                editor.replace_text_in_range(Some(0..0), "forbidden", window, cx);
                assert_eq!(editor.value().as_ref(), exact);
            })
        });
    });
    assert_eq!(
        std::fs::read(owner.path().join("vault/source.md")).unwrap(),
        source.text.as_bytes()
    );
}
