//! Actual guarded raw queries, full-shell range widgets and exact read-only copies.
use super::*;
use brn_workflow::{
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    library::{RefreshReport, Unreadable},
};
use gpui_kit::{EntityInputHandler, VisualTestContext, test::TestWindowExt};

fn reply(worker: &AppWorker, command: (Uuid, AppCommand)) -> (Uuid, AppEvent) {
    worker.submit(command.0, command.1).unwrap();
    loop {
        let event = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if event.0 == command.0 {
            return event;
        }
    }
}
#[gpui_kit::test]
fn shipping_shell_damaged_raw_ranges_are_exact_readonly_and_copyable(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = fixture.path().join("data");
    let vault = fixture.path().join("vault");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(&vault).unwrap();
    let mut text = "\u{feff}---\r\nbrn_id: invalid\r\n---\r\n".to_string();
    text.push_str(&"x".repeat(949000 - text.len()));
    text.push_str("Appendix õ 日本語 🦀\r\nExact final tail\r\n");
    std::fs::write(vault.join("damaged.md"), &text).unwrap();
    let mut worker = AppWorker::start(
        data,
        AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: Some(fixture.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    loop {
        let (_, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        match event {
            AppEvent::Ready { .. } => break,
            AppEvent::Failed(error) => panic!("{}", error.message),
            _ => {}
        }
    }
    for width in [1100., 480.] {
        let (_window_fixture, handle, desktop) = dashboard_tests::window(cx, true);
        let mut visual = VisualTestContext::from_window(handle.into(), cx);
        visual.simulate_resize(size(px(width), px(800.)));
        let command = visual.update(|_, cx| {
            desktop.update(cx, |this, _| {
                this.open_doc = Some(DocRef::Evidence);
                this.centre_tab = CentreTab::Document;
                this.ai
                    .as_mut()
                    .unwrap()
                    .open_raw_evidence("damaged.md".into())
            })
        });
        let (id, event) = reply(&worker, command);
        visual.update(|window, cx| {
            desktop.update(cx, |this, cx| {
                this.ai.as_mut().unwrap().apply(id, event);
                this.sync_raw_evidence_widgets(window, cx);
                cx.notify();
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            let editor = desktop.read(cx).raw_evidence.editor.clone();
            editor.update(cx, |editor, cx| {
                editor.focus(window, cx);
                editor.replace_text_in_range(Some(0..0), "forbidden mutation", window, cx);
                assert_eq!(editor.value().as_ref(), &text[..50000]);
            });
        });
        let command = visual.update(|_, cx| {
            desktop.update(cx, |this, _| {
                this.ai
                    .as_mut()
                    .unwrap()
                    .raw_range("949000", &text.len().to_string())
                    .unwrap()
            })
        });
        let (id, event) = reply(&worker, command);
        visual.update(|window, cx| {
            desktop.update(cx, |this, cx| {
                this.ai.as_mut().unwrap().apply(id, event);
                this.sync_raw_evidence_widgets(window, cx);
                cx.notify();
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollDelta, ScrollWheelEvent};
            window.render_frame(cx);
            let position = window.find("raw-evidence-body").bounds().origin + point(px(3.), px(3.));
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
                    delta: ScrollDelta::Pixels(point(px(0.), px(-10000.))),
                    ..Default::default()
                }
                .to_platform_input(),
                cx,
            );
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            let control = window.find("copy-raw-range").bounds();
            assert!(
                control.origin.y >= px(0.) && control.bottom() <= window.viewport_size().height
            );
            window.click("copy-raw-range", cx);
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().unwrap(),
                &text[949000..]
            );
            window.click("copy-raw-details", cx);
            let copied: serde_json::Value =
                serde_json::from_str(&cx.read_from_clipboard().unwrap().text().unwrap()).unwrap();
            assert_eq!(copied["start_byte"], 949000);
            assert_eq!(copied["end_byte"], text.len());
            assert_eq!(copied["text"], &text[949000..]);
            assert!(copied["facts"].is_null());
            let this = desktop.read(cx);
            assert!(this.ai.as_ref().unwrap().editor.is_none());
            assert!(this.ai.as_ref().unwrap().evidence.is_none());
        });
    }
    let (_, event) = reply(&worker, (Uuid::new_v4(), AppCommand::Editors));
    assert!(matches!(event, AppEvent::Editors(editors) if editors.is_empty()));
    assert_eq!(
        std::fs::read(vault.join("damaged.md")).unwrap(),
        text.as_bytes()
    );
    worker.shutdown().unwrap();
}

#[gpui_kit::test]
fn unreadable_inventory_raw_navigation_preserves_unsubmitted_action_input(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = dashboard_tests::action_window(cx, true);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| {
            this.layout.history_collapsed = true;
            let ai = this.ai.as_mut().unwrap();
            ai.vault_bound = true;
            ai.refresh = Some(RefreshReport {
                unreadable: vec![Unreadable {
                    path: "damaged.md".into(),
                    reason: "invalid identity",
                }],
                ..Default::default()
            });
            assert!(ai.begin_action_draft(None));
            this.open_doc = Some(DocRef::Draft);
            this.centre_tab = CentreTab::Document;
            this.sync_draft_widgets(window, cx);
            cx.notify();
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let input = desktop.read(cx).initial_action.fields[0].clone();
        input.update(cx, |input, cx| {
            input.replace_text_in_range(Some(0..0), "Retained owner input õ\r\n", window, cx)
        });
        window.click("open-raw-evidence-damaged.md", cx);
        let this = desktop.read(cx);
        assert_eq!(this.open_doc, Some(DocRef::Draft));
        assert!(this.ai.as_ref().unwrap().draft.is_some());
        assert!(this.ai.as_ref().unwrap().raw_evidence.is_none());
    });
}
