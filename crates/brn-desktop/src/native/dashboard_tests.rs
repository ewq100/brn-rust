//! Native controls, immutable confirmation and guarded navigation.
use super::*;
use gpui_kit::{
    EntityInputHandler, TestSupportExt, VisualTestContext, component::WindowExt,
    test::TestWindowExt,
};

struct DashboardProbe(Entity<Desktop>, bool);
impl Render for DashboardProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .id("dashboard-test-pane")
                .size_full()
                .flex()
                .flex_col()
                .child(if desktop.open_doc == Some(DocRef::Dashboard) {
                    desktop.render_dashboard(cx)
                } else if self.1 && desktop.open_doc == Some(DocRef::Draft) {
                    desktop.render_draft(cx)
                } else {
                    desktop.render_simple_history(cx)
                })
                .test_support()
        })
    }
}
pub(super) fn window(
    cx: &mut gpui_kit::TestAppContext,
    shipping: bool,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<Root>,
    Entity<Desktop>,
) {
    window_with_draft(cx, shipping, false)
}
pub(super) fn action_window(
    cx: &mut gpui_kit::TestAppContext,
    shipping: bool,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<Root>,
    Entity<Desktop>,
) {
    window_with_draft(cx, shipping, true)
}
fn window_with_draft(
    cx: &mut gpui_kit::TestAppContext,
    shipping: bool,
    show_draft: bool,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<Root>,
    Entity<Desktop>,
) {
    cx.update(|cx| {
        gpui_kit::component::init(cx);
        // Mouse down/up must target settled dialog geometry in widget tests.
        cx.set_reduce_motion(true);
    });
    let fixture = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = fixture.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let captured = saved.clone();
    let viewport = if shipping {
        size(px(1100.), px(800.))
    } else {
        size(px(480.), px(480.))
    };
    let handle = cx.open_window(viewport, move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut desktop = Desktop::new(
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
            desktop.app_worker.take().unwrap().shutdown().unwrap();
            desktop.ai.as_mut().unwrap().ready = true;
            desktop
        });
        *captured.borrow_mut() = Some(desktop.clone());
        if shipping {
            desktop_root(desktop, window, cx)
        } else {
            let probe = cx.new(|_| DashboardProbe(desktop, show_draft));
            Root::new(probe, window, cx)
        }
    });
    let desktop = saved.borrow().clone().unwrap();
    (fixture, handle, desktop)
}
#[gpui_kit::test]
fn dashboard_navigation_is_visible_and_preserves_unresolved_initial_input(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.find("open-dashboard").visible());
        desktop.update(cx, |this, cx| {
            this.ai.as_mut().unwrap().vault_bound = true;
            assert!(this.ai.as_mut().unwrap().begin_draft(None));
            this.open_doc = Some(DocRef::Draft);
            this.sync_draft_widgets(window, cx);
            this.draft_title.update(cx, |editor, cx| {
                editor.set_value("Unacknowledged input λ", window, cx)
            });
        });
        window.click("open-dashboard", cx);
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::Draft));
        assert!(desktop.read(cx).ai.as_ref().unwrap().draft.is_some());
        assert!(desktop.read(cx).simple_transition.is_none());
        assert!(!desktop.read(cx).ai.as_ref().unwrap().dashboard.visible);
    });
}

fn load_dashboard(desktop: &Entity<Desktop>, window: &mut Window, cx: &mut App) {
    desktop.update(cx, |this, cx| {
        let ai = this.ai.as_mut().unwrap();
        let query = ai.open_dashboard().unwrap();
        ai.apply(
            query.0,
            brn_workflow::app_worker::AppEvent::ActionDashboard(Box::new(
                crate::ai::dashboard_state_tests::page(crate::ai::dashboard_state_tests::record(
                    200,
                )),
            )),
        );
        assert!(ai.select_dashboard_action(Uuid::from_u128(200)));
        this.open_doc = Some(DocRef::Dashboard);
        this.sync_dashboard_widgets(window, cx);
        cx.notify();
    });
}
fn scroll_dashboard(window: &mut Window, id: &str, cx: &mut App) {
    use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollWheelEvent};
    window.render_frame(cx);
    let pane = window.find("dashboard-scroll");
    let target = window.find(id.to_owned());
    let position = pane.bounds().origin + point(px(3.), px(3.));
    let dy = pane.bounds().origin.y + px(40.) - target.bounds().origin.y;
    // Scroll the outer view, not the read-only editor under its centre.
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
            delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), dy)),
            ..Default::default()
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
    let target = window.find(id.to_owned());
    assert!(
        target.visible()
            && target.bounds().center().y >= px(0.)
            && target.bounds().center().y < window.viewport_size().height,
        "{id} must be reachable: {:?}",
        target.bounds()
    );
}
fn scroll_to(visual: &mut VisualTestContext, id: &str) {
    visual.update(|window, cx| scroll_dashboard(window, id, cx));
}
#[gpui_kit::test]
fn dashboard_full_readonly_proof_copy_counts_and_filters_are_reachable_at_minimum_size(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| load_dashboard(&desktop, window, cx));
    visual.run_until_parked();
    for index in 0..8 {
        scroll_to(&mut visual, &format!("dashboard-filter-{index}"));
    }
    scroll_to(&mut visual, "dashboard-counts");
    visual.update(|window, _| {
        assert_eq!(window.find("dashboard-counts").label(), Some("As of 2028-02-29 · Open 1 · Waiting 0 · Blocked 0 · Completed 0 · Overdue 0 · Follow-up 0"));
    });
    scroll_to(&mut visual, "copy-dashboard-proof");
    visual.update(|window, cx| {
        let editor = desktop.read(cx).dashboard.proof.clone();
        let expected = dashboard::proof_text(desktop.read(cx).ai.as_ref().unwrap(), None);
        assert_eq!(editor.read(cx).value().as_ref(), expected);
        editor.update(cx, |editor, cx| {
            editor.focus(window, cx);
            editor.replace_text_in_range(Some(0..0), "forbidden edit", window, cx);
            assert_eq!(editor.value().as_ref(), expected);
        });
        window.click("copy-dashboard-proof", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(expected.as_str())
        );
        let entry = desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .dashboard
            .selected
            .as_ref()
            .unwrap();
        assert!(expected.contains(&serde_json::to_string(&entry.action.data.description).unwrap()));
        let shown: brn_workflow::dashboard::DashboardEntry = serde_json::from_str(
            expected
                .strip_prefix("Complete Action and dependency observation:\n")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(&shown, entry);
        assert!(
            expected.contains("dependencies")
                && expected.contains("proposal")
                && expected.contains("waiting_since_ms")
        );
    });
    scroll_to(&mut visual, "complete-selected-action");
}
#[gpui_kit::test]
fn completion_modal_freezes_every_before_field_and_refuses_changed_selection(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, true);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| load_dashboard(&desktop, window, cx));
    let before = crate::ai::dashboard_state_tests::record(200);
    visual
        .update(|window, cx| desktop.update(cx, |this, cx| this.open_complete_action(window, cx)));
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.has_active_dialog(cx));
        let modal = window.within("exact-completion-capture");
        assert_eq!(
            modal.find("action-0-metadata-0").label(),
            Some(
                format!(
                    "Full captured before Action {} · version {} · updated at {} ms",
                    before.origin.id, before.version, before.updated_at_ms
                )
                .as_str()
            )
        );
        for (scope, data) in [
            ("action-0-origin", &before.origin.data),
            ("action-0-before", &before.data),
        ] {
            let fields = crate::review::action_fields::ActionFields::from(data);
            for (field, label) in crate::review::action_fields::LABELS.iter().enumerate() {
                assert_eq!(
                    modal.find(format!("{scope}-field-{field}")).label(),
                    Some(format!("{label}: {}", fields.values[field]).as_str())
                );
            }
        }
        desktop.update(cx, |this, cx| {
            this.ai
                .as_mut()
                .unwrap()
                .dashboard
                .selected
                .as_mut()
                .unwrap()
                .action
                .data
                .title = "New uncaptured title".into();
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window
                .within("exact-completion-capture")
                .find("action-0-before-field-0")
                .label(),
            Some(format!("Title: {}", before.data.title).as_str())
        );
        window.scroll(
            "dialog-0",
            gpui_kit::ScrollDelta::Pixels(point(px(0.), px(-10000.))),
            cx,
        );
        window.render_frame(cx);
        assert!(window.find("confirm-exact-action-completion").visible());
        window.click("confirm-exact-action-completion", cx);
        assert!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .dashboard
                .attempts
                .is_empty()
        );
        assert!(window.has_active_dialog(cx));
        window.close_dialog(cx);
    });
}

#[gpui_kit::test]
fn synchronous_submission_refusal_retains_copyable_exact_request_and_retry_after_navigation(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, true);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| load_dashboard(&desktop, window, cx));
    visual
        .update(|window, cx| desktop.update(cx, |this, cx| this.open_complete_action(window, cx)));
    visual.run_until_parked();
    let request = visual.update(|window, cx| {
        window.render_frame(cx);
        window.scroll(
            "dialog-0",
            gpui_kit::ScrollDelta::Pixels(point(px(0.), px(-10000.))),
            cx,
        );
        window.render_frame(cx);
        window.click("confirm-exact-action-completion", cx);
        let this = desktop.read(cx);
        let ai = this.ai.as_ref().unwrap();
        assert_eq!(ai.dashboard.attempts.len(), 1);
        let attempt = &ai.dashboard.attempts[0];
        assert_eq!(
            attempt.error.as_ref().unwrap().kind,
            brn_workflow::ErrorKind::Cancelled
        );
        assert!(attempt.receipt.is_none());
        assert!(!ai.pending.contains_key(&attempt.request.operation_id));
        let request = attempt.request.clone();
        window.close_dialog(cx);
        request
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        let copy = format!("copy-completion-request-{}", request.operation_id);
        scroll_dashboard(window, &copy, cx);
        window.click(copy, cx);
        let copied: brn_workflow::action_completion::CompleteActionRequest = serde_json::from_str(
            &cx.read_from_clipboard()
                .and_then(|item| item.text())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(copied, request);
        desktop.update(cx, |this, cx| {
            this.simple_leave(simple::EditorTransition::Hide, cx);
            assert!(!this.ai.as_ref().unwrap().dashboard.visible);
            assert_eq!(
                this.ai.as_ref().unwrap().dashboard.attempts[0].request,
                request
            );
        });
        load_dashboard(&desktop, window, cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        let retry = format!("retry-completion-{}", request.operation_id);
        scroll_dashboard(window, &retry, cx);
        window.click(retry, cx);
        let ai = desktop.read(cx).ai.as_ref().unwrap();
        assert_eq!(ai.dashboard.attempts.len(), 1);
        assert_eq!(ai.dashboard.attempts[0].request, request);
        assert_eq!(
            ai.dashboard.attempts[0].error.as_ref().unwrap().kind,
            brn_workflow::ErrorKind::Cancelled
        );
    });
}

#[gpui_kit::test]
fn new_action_navigation_is_available_without_a_vault_or_provider(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.find("new-action-form").visible());
        window.click("new-action-form", cx);
        let this = desktop.read(cx);
        assert_eq!(this.open_doc, Some(DocRef::Draft));
        let form = this.ai.as_ref().unwrap().draft.as_ref().unwrap();
        assert!(form.action.is_some());
        assert!(form.can_leave());
        assert!(form.submitted.is_none());
        assert!(!this.ai.as_ref().unwrap().vault_bound);
        assert!(this.ai.as_ref().unwrap().selection.is_none());
    });
}
