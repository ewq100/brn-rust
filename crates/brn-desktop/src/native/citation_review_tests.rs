use super::*;
use crate::ai::{citation_review_state::NeedsReviewMode, citation_review_state_tests as fixture};
use brn_workflow::{app_worker::AppEvent, knowledge::*};
use gpui_kit::{EntityInputHandler, TestSupportExt, VisualTestContext, test::TestWindowExt};

struct Probe(Entity<Desktop>);
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .id("citation-test-pane")
                .size_full()
                .flex()
                .flex_col()
                .child(desktop.render_findings(cx))
                .test_support()
        })
    }
}
fn open(
    cx: &mut gpui_kit::TestAppContext,
    page: CitationReviewPage,
    detail: Option<CitationReviewDetail>,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<Root>,
    Entity<Desktop>,
) {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    let vault = owner.path().join("vault");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(&vault).unwrap();
    std::fs::write(vault.join("owner.md"), fixture::EXACT).unwrap();
    let config = brn_workflow::app::AppConfig {
        vault_root: Some(vault),
        credentials_dir: Some(owner.path().join("credentials")),
        model_dir: None,
    };
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    cx.update(gpui_kit::component::init);
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
                *ai = fixture::ready();
                let (id, _) = ai.open_citation_review().unwrap();
                ai.apply(id, AppEvent::CitationReview(Box::new(page)));
                if let Some(detail) = detail {
                    let row = ai
                        .citation_review_row_capture(&fixture::entry(&detail))
                        .unwrap();
                    let (id, _) = ai.select_citation_review(row).unwrap();
                    ai.apply(id, AppEvent::CitationReviewDetail(Box::new(detail)));
                }
                d.open_doc = Some(DocRef::Findings);
                d.centre_tab = CentreTab::Document;
                d.sync_citation_review_widgets(window, cx);
                d
            });
            *saved.borrow_mut() = Some(desktop.clone());
            Root::new(cx.new(|_| Probe(desktop)), window, cx)
        },
    );
    (owner, window, capture.borrow().clone().unwrap())
}
fn scroll_to(visual: &mut VisualTestContext, id: &'static str) {
    use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollDelta, ScrollWheelEvent};
    visual.update(|window, cx| {
        window.render_frame(cx);
        let pane = window.find("citation-test-pane");
        let target = window.find(id);
        let position = pane.bounds().origin + point(px(3.), px(3.));
        let dy = pane.bounds().origin.y + px(40.) - target.bounds().origin.y;
        window.dispatch_event(
            MouseMoveEvent {
                position,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            ScrollWheelEvent {
                position,
                delta: ScrollDelta::Pixels(point(px(0.), dy)),
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.render_frame(cx);
        assert!(window.find(id).visible(), "{id} must be reachable");
    });
}
#[gpui_kit::test]
fn exact_consumer_complete_proof_and_original_quotes_are_readonly_and_copyable(
    cx: &mut gpui_kit::TestAppContext,
) {
    let detail = fixture::detail();
    let proof = serde_json::to_string_pretty(&detail.provenance).unwrap();
    let (owner, window, desktop) = open(
        cx,
        fixture::page(vec![fixture::entry(&detail)]),
        Some(detail.clone()),
    );
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        for id in [
            "resolve-finding",
            "dismiss-finding",
            "inspect-finding-evidence",
            "retry-finding-capture",
            "analyze-saved-inbox-source",
        ] {
            assert!(
                window.try_find(id).is_none(),
                "derived citation review must not expose {id}"
            );
        }
        assert!(
            window
                .find("citation-review-row-0")
                .label()
                .unwrap()
                .contains("Incomplete")
        );
    });
    scroll_to(&mut visual, "copy-citation-consumer");
    visual.update(|window, cx| {
        let pane = &desktop.read(cx).citation_review;
        let consumer = pane.consumer.clone();
        let evidence = pane.proof.clone();
        let quote = pane.quotes[0].clone();
        for (editor, expected) in [
            (consumer, detail.text.as_str()),
            (evidence, proof.as_str()),
            (quote, fixture::EXACT),
        ] {
            editor.update(cx, |editor, cx| {
                assert_eq!(editor.value().as_ref(), expected);
                editor.replace_text_in_range(Some(0..0), "blocked", window, cx);
                assert_eq!(editor.value().as_ref(), expected);
            });
        }
        window.click("copy-citation-consumer", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(detail.text.as_str())
        );
    });
    scroll_to(&mut visual, "copy-citation-proof");
    visual.update(|window, cx| {
        window.click("copy-citation-proof", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(proof.as_str())
        );
    });
    scroll_to(&mut visual, "copy-source-quote-0");
    visual.update(|window, cx| {
        window.click("copy-source-quote-0", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(fixture::EXACT)
        );
    });
    assert_eq!(
        std::fs::read(owner.path().join("vault/owner.md")).unwrap(),
        fixture::EXACT.as_bytes()
    );
}
#[gpui_kit::test]
fn sparse_pages_load_more_and_modes_refresh_without_owner_buffer_changes(
    cx: &mut gpui_kit::TestAppContext,
) {
    let mut sparse = fixture::page(vec![]);
    sparse.inspected_count = 25;
    sparse.next_cursor = Some(fixture::cursor());
    sparse.coverage.incomplete = true;
    sparse.coverage.diagnostic_count = 33;
    sparse.coverage.diagnostics_truncated = true;
    sparse.coverage.diagnostics = (0..32)
        .map(|i| IdentityIssue {
            path: format!("bad-{i:02}.md"),
            reason: "Synthetic unavailable identity".into(),
        })
        .collect();
    let (_owner, window, desktop) = open(cx, sparse, None);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert_ne!(window.find("more-citation-review").disabled(), Some(true));
        let coverage = &desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .citation_review
            .page
            .as_ref()
            .unwrap()
            .coverage;
        assert_eq!(coverage.diagnostic_count, 33);
        assert_eq!(coverage.diagnostics.len(), 32);
        assert!(coverage.diagnostics_truncated);
    });
    scroll_to(&mut visual, "more-citation-review");
    visual.update(|window, cx| {
        desktop.update(cx, |d, _| {
            fixture::owner_editor(d.ai.as_mut().unwrap());
            d.message = "Composer õ\r\n".into();
        });
        window.click("more-citation-review", cx);
        let d = desktop.read(cx);
        let ai = d.ai.as_ref().unwrap();
        assert!(
            ai.citation_review.page.is_none(),
            "empty sparse page still admitted Load more"
        );
        assert!(
            ai.citation_review.error.is_some(),
            "disconnected fixture reports submission failure"
        );
        assert_eq!(ai.editor.as_ref().unwrap().text, "Owner unsaved note õ\r\n");
        assert_eq!(d.message, "Composer õ\r\n");
    });
    scroll_to(&mut visual, "needs-review-findings");
    visual.update(|window, cx| {
        window.click("needs-review-findings", cx);
        assert_eq!(
            desktop.read(cx).ai.as_ref().unwrap().needs_review_mode,
            NeedsReviewMode::Findings
        );
        window.render_frame(cx);
        window.click("needs-review-citations", cx);
        window.render_frame(cx);
        assert_eq!(
            desktop.read(cx).ai.as_ref().unwrap().needs_review_mode,
            NeedsReviewMode::CitationEvidence
        );
        window.click("refresh-citation-review", cx);
        let ai = desktop.read(cx).ai.as_ref().unwrap();
        assert!(ai.active.is_none());
        assert!(ai.proposals.is_empty());
    });
}
#[gpui_kit::test]
fn guarded_needs_review_navigation_preserves_editor_composer_comment_and_pending_destination(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_owner, window, desktop) = open(cx, fixture::page(vec![]), None);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |d, cx| {
            fixture::owner_editor(d.ai.as_mut().unwrap());
            d.open_doc = Some(DocRef::SavedNote);
            d.message = "Retained composer".into();
            d.simple_leave(simple::EditorTransition::Findings, cx);
            assert_eq!(d.open_doc, Some(DocRef::SavedNote));
            assert!(matches!(
                d.simple_transition,
                Some(simple::EditorTransition::Findings)
            ));
            assert_eq!(
                d.ai.as_ref().unwrap().editor.as_ref().unwrap().text,
                "Owner unsaved note õ\r\n"
            );
            d.review_comment_draft = Some((Uuid::new_v4(), None));
            d.review_comment.update(cx, |editor, cx| {
                editor.set_value("Retained comment õ\r\n", window, cx)
            });
            d.simple_progress_transition(cx);
            assert_eq!(
                d.review_comment.read(cx).value().as_ref(),
                "Retained comment õ\r\n"
            );
            assert!(matches!(
                d.simple_transition,
                Some(simple::EditorTransition::Findings)
            ));
            // A mode callback cannot overtake the pending guarded transition.
            d.open_doc = Some(DocRef::Findings);
            let mode = d.ai.as_ref().unwrap().needs_review_mode;
            d.switch_needs_review_mode(NeedsReviewMode::Findings, cx);
            assert_eq!(d.ai.as_ref().unwrap().needs_review_mode, mode);
            assert_eq!(d.message, "Retained composer");
        })
    });
}
