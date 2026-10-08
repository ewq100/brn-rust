//! Exercise dialogs through the same root composition used by the shipping app.
use super::*;
use gpui_kit::{TestSupportExt, VisualTestContext, component::WindowExt, test::TestWindowExt};

fn fixture_window(
    cx: &mut gpui_kit::TestAppContext,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<Root>,
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
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    let window = cx.open_window(size(px(1100.), px(800.)), move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut desktop = Desktop::new(
                data,
                config,
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            desktop.app_worker.take().unwrap().shutdown().unwrap();
            desktop.ai.as_mut().unwrap().ready = true;
            desktop
        });
        *saved.borrow_mut() = Some(desktop.clone());
        desktop_root(desktop, window, cx)
    });
    let desktop = capture.borrow().clone().unwrap();
    (fixture, window, desktop)
}

#[gpui_kit::test]
fn settings_is_visible_and_rebuilds_from_current_account_state(cx: &mut gpui_kit::TestAppContext) {
    let (fixture, window, desktop) = fixture_window(cx);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| window.click("settings-footer", cx));
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.has_active_dialog(cx), "Settings was not admitted");
        assert!(
            window
                .try_find("settings-body")
                .is_some_and(|body| body.visible()),
            "Settings opened but its dialog content was not rendered"
        );
        assert!(window.find("settings-body").visible());
        assert!(window.find("appearance-system").visible());
        window.click("settings-tab-ai", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.find("reasoning-effort-low").visible());
        assert!(window.try_find("model-0-0").is_none());
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.provider = Some(brn_workflow::Provider::Chatgpt);
            ai.accounts[0].models = vec![brn_workflow::ModelOption {
                id: "gpt-6-luna".into(),
                live_qualified: false,
            }];
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(
            window.try_find("model-0-0").is_some(),
            "Open settings kept old account state"
        );
        window.close_dialog(cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("settings-body").is_none());
        window.click("settings-footer", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.find("settings-body").visible());
    });
    assert_eq!(
        std::fs::read_dir(fixture.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
}

#[gpui_kit::test]
fn modal_blocks_background_and_exposes_its_content_until_closed(cx: &mut gpui_kit::TestAppContext) {
    let (fixture, window, desktop) = fixture_window(cx);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.open_dialog(cx, |dialog, _, _| {
            dialog
                .title("Synthetic exact review")
                .overlay_closable(false)
                .child(
                    div()
                        .id("synthetic-review-modal")
                        .test_support()
                        .child("Knowledge unchanged until exact approval"),
                )
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.find("synthetic-review-modal").visible());
        assert!(desktop.read(cx).open_doc.is_none());
        window.click("open-queue", cx);
        assert!(desktop.read(cx).open_doc.is_none());
        assert!(window.try_find("settings-body").is_none());
        window.close_dialog(cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("synthetic-review-modal").is_none());
        window.click("open-queue", cx);
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::Queue));
        window.click("settings-footer", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.find("settings-body").visible());
    });
    assert_eq!(
        std::fs::read_dir(fixture.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
}
