//! Real headless history controls and composer preservation; never launch the GUI.
use super::*;
use crate::ai::session_state_tests::{ready, result, summary, turn};
use brn_workflow::{
    app_worker::AppEvent,
    conversations::{ConversationFilter, ConversationState},
};
use gpui_kit::{TestSupportExt, VisualTestContext, test::TestWindowExt};

struct SessionProbe(Entity<Desktop>);
impl Render for SessionProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .id("session-test-pane")
                .size_full()
                .flex()
                .child(
                    div()
                        .w(px(320.))
                        .h_full()
                        .child(desktop.render_simple_history(cx)),
                )
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .child(desktop.render_simple_chat(cx)),
                )
                .test_support()
        })
    }
}
fn window(
    cx: &mut gpui_kit::TestAppContext,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<SessionProbe>,
    Entity<Desktop>,
    Uuid,
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
    let selected = Uuid::new_v4();
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let captured = saved.clone();
    let handle = cx.open_window(size(px(1200.), px(1000.)), move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut desktop = Desktop::new(
                data,
                config,
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            desktop.app_worker.take().unwrap().shutdown().unwrap();
            desktop.ai = Some(ready(selected));
            let turn = turn(selected);
            desktop.ai.as_mut().unwrap().turns.push(turn);
            desktop.query.update(cx, |input, cx| {
                input.set_value("Unsent composer õ\r\n", window, cx)
            });
            desktop
        });
        *captured.borrow_mut() = Some(desktop.clone());
        SessionProbe(desktop)
    });
    (fixture, handle, saved.borrow().clone().unwrap(), selected)
}

#[gpui_kit::test]
fn session_lifecycle_native_archived_history_is_readable_ask_is_disabled_and_restore_failure_is_visible(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop, id) = window(cx);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.update(|_, cx| {
        desktop.update(cx, |this, cx| {
            let ai = this.ai.as_mut().unwrap();
            assert_eq!(ai.session_history.filter, ConversationFilter::Active);
            ai.session_history
                .lifecycle
                .insert(id, summary(id, 2, ConversationState::Archived).lifecycle);
            ai.session_history.summaries.clear();
            cx.notify();
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window.find("selected-session-lifecycle").label(),
            Some("Archived — restore to continue")
        );
        assert_eq!(
            window.find("archived-chat-banner").label(),
            Some("Archived — restore to continue")
        );
        assert!(window.try_find("restore-session").is_some());
        window.click("ask", cx);
        let ai = desktop.read(cx).ai.as_ref().unwrap();
        assert!(ai.active.is_none());
        assert_eq!(ai.turns[0].answer, "Retained partial answer õ\r\n");
        window.click("restore-session", cx);
        let this = desktop.read(cx);
        let ai = this.ai.as_ref().unwrap();
        assert!(!ai.session_change_pending());
        assert_eq!(ai.conversation, Some(id));
        assert_eq!(ai.selected_session_lifecycle().unwrap().stamp.version, 2);
        assert_eq!(
            ai.session_history.error.as_deref(),
            Some("Application lane is closing")
        );
        assert_eq!(
            this.query.read(cx).value().as_ref(),
            "Unsent composer õ\r\n"
        );
        assert!(ai.active.is_none());
        assert!(ai.rewrite.is_none());
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window.find("session-lifecycle-error").label(),
            Some("Application lane is closing")
        );
        desktop.update(cx, |this, cx| {
            let ai = this.ai.as_mut().unwrap();
            let (
                operation,
                brn_workflow::app_worker::AppCommand::SetConversationLifecycle(request),
            ) = ai
                .set_selected_session_lifecycle(ConversationState::Active)
                .unwrap()
            else {
                panic!("Restore")
            };
            assert_eq!(request.expected.version, 2);
            ai.apply(
                operation,
                AppEvent::ConversationLifecycleChanged(result(&request)),
            );
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("archived-chat-banner").is_none());
        assert!(window.try_find("archive-session").is_some());
        let this = desktop.read(cx);
        let ai = this.ai.as_ref().unwrap();
        assert!(ai.can_ask());
        assert_eq!(ai.conversation, Some(id));
        assert_eq!(ai.turns.len(), 1);
        assert_eq!(
            this.query.read(cx).value().as_ref(),
            "Unsent composer õ\r\n"
        );
        assert!(ai.active.is_none());
    });
}

#[gpui_kit::test]
fn session_lifecycle_native_pending_controls_freeze_navigation_and_ack_preserves_composer_and_review_widgets(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop, id) = window(cx);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    let other = Uuid::new_v4();
    let mut captured = None;
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| {
            let ai = this.ai.as_mut().unwrap();
            ai.session_history
                .summaries
                .push(summary(other, 1, ConversationState::Active));
            let (record, _) = crate::review::predecessor_tests::fixture();
            ai.review = Some(crate::review::ProposalReview::new(record));
            ai.review
                .as_mut()
                .unwrap()
                .edit_title("Owner review title õ".into(), Instant::now())
                .unwrap();
            this.sync_review_widgets(window, cx);
            this.review_comment.update(cx, |input, cx| {
                input.set_value("Unsent comment õ", window, cx)
            });
            let ai = this.ai.as_mut().unwrap();
            captured = ai.set_selected_session_lifecycle(ConversationState::Archived);
            this.centre_tab = CentreTab::Document;
            cx.notify();
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        for target in [
            "new-session".to_owned(),
            "sessions-archived".into(),
            format!("conversation-{other}"),
            "archive-session".into(),
        ] {
            window.click(target, cx);
        }
        let this = desktop.read(cx);
        let ai = this.ai.as_ref().unwrap();
        assert_eq!(ai.conversation, Some(id));
        assert_eq!(ai.session_history.filter, ConversationFilter::Active);
        assert_eq!(this.centre_tab, CentreTab::Document);
        assert!(ai.session_change_pending());
        assert_eq!(ai.turns.len(), 1);
        assert!(!ai.can_ask());
        assert_eq!(
            this.query.read(cx).value().as_ref(),
            "Unsent composer õ\r\n"
        );
        assert_eq!(
            this.review_title.read(cx).value().as_ref(),
            "Owner review title õ"
        );
        assert_eq!(
            this.review_comment.read(cx).value().as_ref(),
            "Unsent comment õ"
        );
        desktop.update(cx, |this, cx| {
            let (
                operation,
                brn_workflow::app_worker::AppCommand::SetConversationLifecycle(request),
            ) = captured.take().unwrap()
            else {
                panic!("Archive")
            };
            this.ai.as_mut().unwrap().apply(
                operation,
                AppEvent::ConversationLifecycleChanged(result(&request)),
            );
            this.sync_review_widgets(window, cx);
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let this = desktop.read(cx);
        let ai = this.ai.as_ref().unwrap();
        assert!(!ai.session_change_pending());
        assert_eq!(ai.conversation, Some(id));
        assert_eq!(ai.session_history.filter, ConversationFilter::Active);
        assert_eq!(ai.turns.len(), 1);
        assert_eq!(
            this.query.read(cx).value().as_ref(),
            "Unsent composer õ\r\n"
        );
        assert_eq!(
            this.review_title.read(cx).value().as_ref(),
            "Owner review title õ"
        );
        assert_eq!(
            this.review_comment.read(cx).value().as_ref(),
            "Unsent comment õ"
        );
        assert!(ai.active.is_none());
        assert!(ai.rewrite.is_none());
        assert_eq!(
            window.find("selected-session-lifecycle").label(),
            Some("Archived — restore to continue")
        );
    });
}

#[gpui_kit::test]
fn session_lifecycle_native_filters_select_archived_history_without_replacing_the_composer(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop, original) = window(cx);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    let archived = Uuid::new_v4();
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        window.click("sessions-archived", cx);
        desktop.update(cx, |this, cx| {
            let ai = this.ai.as_mut().unwrap();
            assert_eq!(ai.session_history.filter, ConversationFilter::Archived);
            assert_eq!(ai.conversation, Some(original));
            assert_eq!(ai.turns.len(), 1);
            let query = ai.refresh_session_summaries();
            ai.apply(
                query.0,
                AppEvent::ConversationSummaries {
                    filter: ConversationFilter::Archived,
                    summaries: vec![summary(archived, 2, ConversationState::Archived)],
                },
            );
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        window.click(format!("conversation-{archived}"), cx);
        let this = desktop.read(cx);
        let ai = this.ai.as_ref().unwrap();
        assert_eq!(ai.conversation, Some(archived));
        assert!(!ai.can_ask());
        assert_eq!(
            this.query.read(cx).value().as_ref(),
            "Unsent composer õ\r\n"
        );
        assert!(ai.active.is_none());
        window.click("sessions-active", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let this = desktop.read(cx);
        let ai = this.ai.as_ref().unwrap();
        assert_eq!(ai.session_history.filter, ConversationFilter::Active);
        assert_eq!(ai.conversation, Some(archived));
        assert_eq!(
            window.find("selected-session-lifecycle").label(),
            Some("Archived — restore to continue")
        );
        assert_eq!(
            this.query.read(cx).value().as_ref(),
            "Unsent composer õ\r\n"
        );
        assert!(ai.active.is_none());
    });
}
