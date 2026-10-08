//! Headless real widgets for explicit predecessor selection and protected History.
use super::super::*;
use crate::review::{ProposalReview, predecessor_tests::fixture};
use brn_workflow::{app_worker::AppEvent, proposals::ProposalRecord};
use gpui_kit::{EntityInputHandler, TestSupportExt, VisualTestContext, test::TestWindowExt};

struct ReviewProbe(Entity<Desktop>);
impl Render for ReviewProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .id("predecessor-test-pane")
                .test_support()
                .size_full()
                .flex()
                .flex_col()
                .child(desktop.render_proposal_review(cx))
        })
    }
}
fn window(
    cx: &mut gpui_kit::TestAppContext,
    record: ProposalRecord,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<ReviewProbe>,
    Entity<Desktop>,
) {
    let fixture = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = fixture.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let config = brn_workflow::app::AppConfig {
        vault_root: None,
        credentials_dir: Some(fixture.path().join("credentials")),
        model_dir: None,
    };
    cx.update(|cx| {
        gpui_kit::component::init(cx);
        cx.set_reduce_motion(true);
    });
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = saved.clone();
    let handle = cx.open_window(size(px(1100.), px(1400.)), move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut desktop = Desktop::new(
                data,
                config,
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            desktop.app_worker.take().unwrap().shutdown().unwrap();
            desktop.open_doc = Some(DocRef::Proposal(record.draft.id));
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            ai.pending.clear();
            ai.review = Some(ProposalReview::new(record));
            desktop.sync_review_widgets(window, cx);
            desktop
        });
        *capture.borrow_mut() = Some(desktop.clone());
        ReviewProbe(desktop)
    });
    (fixture, handle, saved.borrow().clone().unwrap())
}

#[gpui_kit::test]
fn predecessor_pending_fences_real_title_and_body_widgets_and_preserves_selected_path(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (before, after) = fixture();
    let (_owner, handle, desktop) = window(cx, before.clone());
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    let mut operation = None;
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("knowledge-predecessor-controls").is_some());
        desktop.update(cx, |desktop, cx| {
            desktop
                .review_predecessor
                .update(cx, |input, cx| input.set_value("previous.md", window, cx));
            // Capture the real state request; no provider or live worker submission.
            operation = Some(
                desktop
                    .ai
                    .as_mut()
                    .unwrap()
                    .attach_knowledge_predecessor("previous.md".into())
                    .unwrap()
                    .0,
            );
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        let input = desktop.read(cx).review_title.clone();
        input.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "Buffered conflicting typing", window, cx);
        });
        let input = desktop.read(cx).review_editor.clone();
        input.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "Buffered body typing", window, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_ref().unwrap();
            assert_eq!(ai.review.as_ref().unwrap().title(), before.draft.title);
            assert_eq!(
                ai.review.as_ref().unwrap().text(0),
                before.draft.changes[0].text()
            );
            assert_eq!(
                desktop.review_predecessor.read(cx).value().as_ref(),
                "previous.md"
            );
            desktop
                .ai
                .as_mut()
                .unwrap()
                .apply(operation.unwrap(), AppEvent::Proposal(after.clone()));
            desktop.sync_review_widgets(window, cx);
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("knowledge-predecessor-controls").is_none());
        assert_eq!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .review
                .as_ref()
                .unwrap()
                .record,
            after
        );
    });
}

#[gpui_kit::test]
fn generated_history_widget_is_readonly_while_successor_retains_owner_editing(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_, after) = fixture();
    let (_owner, handle, desktop) = window(cx, after.clone());
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.review_member = 1;
            desktop.sync_review_widgets(window, cx);
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let input = desktop.read(cx).review_editor.clone();
        input.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "Must not rewrite History", window, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            assert_eq!(
                desktop
                    .ai
                    .as_ref()
                    .unwrap()
                    .review
                    .as_ref()
                    .unwrap()
                    .text(1),
                after.draft.changes[1].text()
            );
            assert_eq!(
                desktop.review_editor.read(cx).value().as_ref(),
                after.draft.changes[1].text().unwrap()
            );
            desktop.review_member = 0;
            desktop.sync_review_widgets(window, cx);
            cx.notify();
        });
    });
    visual.run_until_parked();
    let refined = after.draft.changes[0]
        .text()
        .unwrap()
        .replace("Exact owner wording", "Refined owner wording");
    visual.update(|window, cx| {
        let input = desktop.read(cx).review_editor.clone();
        input.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), &refined, window, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|_, cx| {
        let review = desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .unwrap();
        assert_eq!(review.text(0), Some(refined.as_str()));
        assert_eq!(review.text(1), after.draft.changes[1].text());
    });
}
