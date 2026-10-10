//! Historical Action identities through the actual 480px Activity widgets.
use super::*;
use brn_workflow::activity::{
    ActivityActionChange, ActivityActionChangeKind, ActivityEntry, ActivityPage, ActivityUndo,
};
use gpui_kit::{VisualTestContext, test::TestWindowExt};

struct ActivityProbe(Entity<Desktop>);
impl Render for ActivityProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .id("activity-test-pane")
                .size_full()
                .flex()
                .flex_col()
                .child(desktop.render_activity(cx))
                .test_support()
        })
    }
}
fn page() -> ActivityPage {
    ActivityPage {
        entries: (0..2)
            .map(|index| ActivityEntry {
                operation_id: Uuid::from_u128(1000 + index),
                proposal_id: Uuid::from_u128(2000 + index),
                group_id: None,
                session_id: None,
                title: format!("Historical approval {index} õ 日本語"),
                approved_at_ms: 1,
                approved_at_utc: None,
                summary: "Created 32 actions; updated 32 actions.".into(),
                changes: vec![],
                undo: (index == 1).then_some(ActivityUndo {
                    operation_id: Uuid::from_u128(999),
                    trash_member: None,
                }),
                action_changes: (0..64)
                    .map(|member| ActivityActionChange {
                        kind: if member % 2 == 0 {
                            ActivityActionChangeKind::Created
                        } else {
                            ActivityActionChangeKind::Replaced
                        },
                        action_id: Uuid::from_u128(index * 100 + member + 1),
                        title: format!(
                            "{index}/{member} full historical õ 日本語\r\n{}TAIL λ",
                            "long approved title ".repeat(20)
                        ),
                    })
                    .collect(),
            })
            .collect(),
        next_before: None,
    }
}
fn scroll_to(window: &mut Window, id: String, cx: &mut App) {
    use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollWheelEvent};
    window.render_frame(cx);
    let pane = window.find("approved-activity-and-recovery");
    let target = window.find(id.clone());
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
            delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), dy)),
            ..Default::default()
        }
        .to_platform_input(),
        cx,
    );
    window.render_frame(cx);
    let target = window.find(id.clone());
    assert!(
        target.visible()
            && target.bounds().center().y >= px(0.)
            && target.bounds().center().y < window.viewport_size().height,
        "{id} must be reachable at 480px: {:?}",
        target.bounds()
    );
}
#[gpui_kit::test]
fn activity_action_inventory_all_members_full_titles_and_recorded_approval_control_at_480(
    cx: &mut gpui_kit::TestAppContext,
) {
    cx.update(|cx| {
        gpui_kit::component::init(cx);
        cx.set_reduce_motion(true);
    });
    let fixture = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = fixture.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let captured = saved.clone();
    let expected = page();
    let initial = expected.clone();
    let handle = cx.open_window(size(px(480.), px(480.)), move |window, cx| {
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
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.pending.clear();
            ai.activity = Some(initial);
            desktop.open_doc = Some(DocRef::Activity);
            desktop.centre_tab = CentreTab::Document;
            desktop.query.update(cx, |editor, cx| {
                editor.set_value("retained composer õ\r\nλ", window, cx)
            });
            desktop
        });
        *captured.borrow_mut() = Some(desktop.clone());
        let probe = cx.new(|_| ActivityProbe(desktop));
        Root::new(probe, window, cx)
    });
    let desktop = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        for entry in &expected.entries {
            for (index, change) in entry.action_changes.iter().enumerate() {
                let member = window.find(format!("activity-action-{}-{index}", entry.operation_id));
                let kind = if change.kind == ActivityActionChangeKind::Created {
                    "Created"
                } else {
                    "Replaced"
                };
                assert_eq!(
                    member.label(),
                    Some(
                        format!(
                            "{kind} Action {} · historical approved title: {}",
                            change.action_id, change.title
                        )
                        .as_str()
                    )
                );
                assert!(
                    member.bounds().origin.x >= px(0.) && member.bounds().right() <= px(480.),
                    "full member must wrap within viewport: {:?}",
                    member.bounds()
                );
                assert!(
                    member.bounds().size.height > px(60.),
                    "complete long title must wrap"
                );
            }
        }
    });
    for entry in &expected.entries {
        for index in [0, 31, 63] {
            visual.update(|window, cx| {
                scroll_to(
                    window,
                    format!("activity-action-{}-{index}", entry.operation_id),
                    cx,
                )
            });
        }
        let control = format!("inspect-approved-{}", entry.operation_id);
        visual.update(|window, cx| scroll_to(window, control.clone(), cx));
        visual.update(|window, cx| {
            window.click(control.clone(), cx);
            window.render_frame(cx);
            let ai = desktop.read(cx).ai.as_ref().unwrap();
            // The absent worker returns the ordinary explicit submission error;
            // reaching this exact snapshot dialog proves the actual control ran.
            assert!(ai.snapshot_generation > 0);
            assert!(ai.snapshot_error.is_some());
            assert_eq!(ai.activity.as_ref(), Some(&expected));
            assert!(ai.application_snapshot.is_none());
            assert_eq!(
                desktop.read(cx).query.read(cx).value().as_ref(),
                "retained composer õ\r\nλ"
            );
            window.close_dialog(cx);
        });
        visual.run_until_parked();
    }
}

#[gpui_kit::test]
fn shipping_shell_activity_oldest_approval_scrolls_into_view(cx: &mut gpui_kit::TestAppContext) {
    for width in [1100., 480.] {
        let (_fixture, handle, desktop) = dashboard_tests::window(cx, true);
        let mut visual = VisualTestContext::from_window(handle.into(), cx);
        visual.simulate_resize(size(px(width), px(800.)));
        let expected = page();
        let control = format!("inspect-approved-{}", expected.entries[1].operation_id);
        visual.update(|_, cx| {
            desktop.update(cx, |this, cx| {
                this.ai.as_mut().unwrap().activity = Some(expected.clone());
                this.open_doc = Some(DocRef::Activity);
                this.centre_tab = CentreTab::Document;
                cx.notify();
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| scroll_to(window, control.clone(), cx));
        visual.update(|window, _| {
            let pane = window.find("approved-activity-and-recovery").bounds();
            let button = window.find(control.clone()).bounds();
            assert!(
                button.origin.y >= pane.origin.y && button.bottom() <= pane.bottom(),
                "approval outside pane: {button:?} / {pane:?}"
            );
        });
    }
}
