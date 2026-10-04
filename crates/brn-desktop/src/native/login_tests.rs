//! Synthetic device prompts exercised through the shipping dialog root.
use super::*;
use brn_workflow::{
    LoginPrompt, Provider,
    app_worker::AppEvent,
    chat_worker::{AccountCommand, AccountEvent},
};
use gpui_kit::{VisualTestContext, component::WindowExt, test::TestWindowExt};

const URI: &str = "https://example.invalid/device?flow=synthetic";
const CODE: &str = "SYNTHETIC-λ-1234";

fn prompt() -> LoginPrompt {
    LoginPrompt {
        verification_uri: URI.into(),
        user_code: CODE.into(),
    }
}
fn fixture(
    cx: &mut gpui_kit::TestAppContext,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<Root>,
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
        let id = desktop.update(cx, |desktop, _| {
            let (id, _) = desktop
                .ai
                .as_mut()
                .unwrap()
                .account(AccountCommand::Connect(Provider::Chatgpt))
                .unwrap();
            id
        });
        *saved.borrow_mut() = Some((desktop.clone(), id));
        desktop_root(desktop, window, cx)
    });
    let (desktop, id) = capture.borrow().clone().unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        desktop.update(cx, |desktop, cx| desktop.open_login(id, window, cx));
    })
    .unwrap();
    (fixture, window, desktop, id)
}
fn deliver(desktop: &Entity<Desktop>, id: Uuid, cx: &mut App) {
    desktop.update(cx, |desktop, cx| {
        desktop.ai.as_mut().unwrap().apply(
            id,
            AppEvent::Account(AccountEvent::Login {
                id,
                prompt: prompt(),
            }),
        );
        cx.notify();
    });
}
fn assert_no_credentials(fixture: &tempfile::TempDir) {
    assert_eq!(
        std::fs::read_dir(fixture.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
}

#[gpui_kit::test]
fn login_dialog_opens_exact_url_only_on_click_and_code_is_readonly_selectable_copyable(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (fixture, window, desktop, id) = fixture(cx);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|_, cx| deliver(&desktop, id, cx));
    visual.run_until_parked();
    assert_eq!(visual.opened_url(), None);
    assert!(visual.update(|_, cx| cx.read_from_clipboard()).is_none());
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("login-code").value(), Some(CODE));
        window.click("login-code", cx);
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
        window.dispatch_action(Box::new(gpui_kit::component::input::Copy), cx);
    });
    visual.run_until_parked();
    assert_eq!(
        visual
            .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .as_deref(),
        Some(CODE)
    );
    visual.update(|window, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
        window.render_frame(cx);
        window.dispatch_action(Box::new(gpui_kit::component::input::Copy), cx);
    });
    visual.run_until_parked();
    assert_eq!(
        visual
            .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .as_deref(),
        Some(CODE)
    );
    visual.update(|window, cx| {
        window.input("blocked mutation", cx);
        window.press("backspace", cx);
        window.render_frame(cx);
        assert_eq!(window.find("login-code").value(), Some(CODE));
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
        window.click("copy-login-code", cx);
    });
    assert_eq!(
        visual
            .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .as_deref(),
        Some(CODE)
    );
    visual.update(|window, cx| window.click("login-verification-url", cx));
    assert_eq!(visual.opened_url().as_deref(), Some(URI));
    visual.update(|_, cx| {
        assert_eq!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .login
                .as_ref()
                .unwrap()
                .prompt
                .as_ref()
                .unwrap()
                .user_code,
            CODE
        );
    });
    assert_no_credentials(&fixture);
}

#[gpui_kit::test]
fn login_controls_reject_a_replaced_prompt_between_pointer_down_and_up(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::{InputEvent as _, MouseButton, MouseDownEvent, MouseUpEvent};
    for control in ["login-verification-url", "copy-login-code"] {
        let (fixture, window, desktop, id) = fixture(cx);
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        visual.run_until_parked();
        visual.update(|_, cx| deliver(&desktop, id, cx));
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            let position = window.find(control).bounds().center();
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
            window.dispatch_event(
                MouseDownEvent {
                    position,
                    button: MouseButton::Left,
                    click_count: 1,
                    modifiers: Default::default(),
                    first_mouse: false,
                }
                .to_platform_input(),
                cx,
            );
            // Keep the painted callback, while the same operation receives a new prompt.
            desktop.update(cx, |desktop, _| {
                desktop.ai.as_mut().unwrap().login.as_mut().unwrap().prompt = Some(LoginPrompt {
                    verification_uri: "https://example.invalid/replacement".into(),
                    user_code: "SYNTHETIC-REPLACEMENT".into(),
                });
            });
            window.dispatch_event(
                MouseUpEvent {
                    position,
                    button: MouseButton::Left,
                    click_count: 1,
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
        });
        assert_eq!(visual.opened_url(), None);
        assert_eq!(
            visual
                .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
                .as_deref(),
            Some("sentinel")
        );
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert_eq!(
                window.find("login-code").value(),
                Some("SYNTHETIC-REPLACEMENT")
            );
        });
        assert_no_credentials(&fixture);
    }
}

#[gpui_kit::test]
fn waiting_login_hides_controls_and_rejects_other_operation_prompts(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (fixture, window, desktop, id) = fixture(cx);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.has_active_dialog(cx));
        assert!(window.try_find("login-code").is_none());
        deliver(&desktop, Uuid::new_v4(), cx);
        window.render_frame(cx);
        assert!(window.try_find("login-verification-url").is_none());
        assert!(window.try_find("copy-login-code").is_none());
        assert!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .login
                .as_ref()
                .unwrap()
                .prompt
                .is_none()
        );
        deliver(&desktop, id, cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("login-code").value(), Some(CODE));
        // A mismatched current operation must remove the old dialog's controls.
        desktop.update(cx, |desktop, cx| {
            desktop
                .ai
                .as_mut()
                .unwrap()
                .login
                .as_mut()
                .unwrap()
                .operation = Uuid::new_v4();
            cx.notify();
        });
        window.render_frame(cx);
        assert!(window.try_find("login-code").is_none());
        assert!(window.try_find("login-verification-url").is_none());
        assert!(window.try_find("copy-login-code").is_none());
    });
    assert_eq!(visual.opened_url(), None);
    assert!(visual.update(|_, cx| cx.read_from_clipboard()).is_none());
    assert_no_credentials(&fixture);
}

#[gpui_kit::test]
fn cancel_and_dialog_close_clear_only_current_login_and_late_prompt_cannot_restore_it(
    cx: &mut gpui_kit::TestAppContext,
) {
    for cancel_button in [true, false] {
        let (fixture, window, desktop, id) = fixture(cx);
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        visual.run_until_parked();
        visual.update(|_, cx| deliver(&desktop, id, cx));
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.find("login-code").visible());
            if cancel_button {
                window.click("cancel-login", cx);
            } else {
                window.press("escape", cx);
            }
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert!(!window.has_active_dialog(cx));
            let ai = desktop.read(cx).ai.as_ref().unwrap();
            assert!(ai.login.is_none());
            assert_eq!(ai.cancelled_login, Some(id));
            assert_eq!(desktop.read(cx).login_dialog, None);
            deliver(&desktop, id, cx);
            window.render_frame(cx);
            assert!(desktop.read(cx).ai.as_ref().unwrap().login.is_none());
            assert!(window.try_find("login-code").is_none());
        });
        assert_eq!(visual.opened_url(), None);
        assert!(visual.update(|_, cx| cx.read_from_clipboard()).is_none());
        assert_no_credentials(&fixture);
    }
}
