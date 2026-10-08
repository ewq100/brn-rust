//! Right-click "Comment on selection…" in proposal text; no graphical/provider calls.
use super::*;
use crate::review::ProposalReview;
use gpui_kit::{VisualTestContext, component::WindowExt, test::TestWindowExt};

#[gpui_kit::test]
fn comment_on_selection_action_opens_the_selected_text_comment(cx: &mut gpui_kit::TestAppContext) {
    let fixture = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = fixture.path().join("data");
    std::fs::create_dir(&data).unwrap();
    cx.update(|cx| {
        gpui_kit::component::init(cx);
        cx.set_reduce_motion(true);
    });
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = saved.clone();
    let record = crate::review::tests::fixture();
    let id = record.draft.id;
    let handle = cx.open_window(size(px(1100.), px(800.)), move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut this = Desktop::new(
                data,
                brn_workflow::app::AppConfig {
                    vault_root: None,
                    credentials_dir: None,
                    model_dir: None,
                },
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            this.app_worker.take().unwrap().shutdown().unwrap();
            let ai = this.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            ai.review = Some(ProposalReview::new(record));
            this.open_doc = Some(DocRef::Proposal(id));
            this.centre_tab = CentreTab::Document;
            this.review_member = 0;
            this.sync_review_widgets(window, cx);
            this
        });
        *capture.borrow_mut() = Some(desktop.clone());
        desktop_root(desktop, window, cx)
    });
    let desktop = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.find("review-text-editor").visible());
        let editor = desktop.read(cx).review_editor.clone();
        editor.update(cx, |editor, cx| editor.focus(window, cx));
        window.render_frame(cx);
        editor.update(cx, |editor, cx| editor.select_all(window, cx));
        assert!(!editor.read(cx).selected_range().is_empty());
        assert!(!window.has_active_dialog(cx));
        window.dispatch_action(Box::new(CommentOnSelection), cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(
            window.has_active_dialog(cx),
            "the action opens the comment dialog"
        );
        assert_eq!(desktop.read(cx).review_comment_draft, Some((id, None)));
    });
}

#[gpui_kit::test]
fn anchored_comments_are_highlighted_and_hoverable_in_the_proposal_text(
    cx: &mut gpui_kit::TestAppContext,
) {
    use brn_workflow::proposals::{CommentTarget, ReviewComment, TextAnchor};
    let fixture = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = fixture.path().join("data");
    std::fs::create_dir(&data).unwrap();
    cx.update(|cx| {
        gpui_kit::component::init(cx);
        cx.set_reduce_motion(true);
    });
    let mut record = crate::review::tests::fixture();
    let text = record.draft.changes[0].text().unwrap().to_owned();
    let quote = "repeat λ";
    let start = text.find(quote).unwrap();
    record.comments.push(ReviewComment {
        id: uuid::Uuid::new_v4(),
        text: "Say this once.".into(),
        target: CommentTarget::Text(TextAnchor {
            change_index: 0,
            start,
            end: start + quote.len(),
            quote: quote.into(),
        }),
    });
    let id = record.draft.id;
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = saved.clone();
    let handle = cx.open_window(size(px(1100.), px(800.)), move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut this = Desktop::new(
                data,
                brn_workflow::app::AppConfig {
                    vault_root: None,
                    credentials_dir: None,
                    model_dir: None,
                },
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            this.app_worker.take().unwrap().shutdown().unwrap();
            let ai = this.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            ai.review = Some(ProposalReview::new(record));
            this.open_doc = Some(DocRef::Proposal(id));
            this.centre_tab = CentreTab::Document;
            this.review_member = 0;
            this.sync_review_widgets(window, cx);
            this
        });
        *capture.borrow_mut() = Some(desktop.clone());
        desktop_root(desktop, window, cx)
    });
    let desktop = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let editor = desktop.read(cx).review_editor.read(cx);
        let set = editor
            .diagnostics()
            .expect("proposal editor keeps comment marks");
        let entry = set
            .for_offset(start + 1)
            .expect("hovering the highlight finds the comment");
        assert_eq!(entry.range, start..start + quote.len());
        assert!(entry.diagnostic.message.contains("Say this once."));
        assert!(
            set.for_offset(0).is_none(),
            "text outside comments has no mark"
        );
        assert!(desktop.read(cx).comment_decorations.is_some());
    });
}
