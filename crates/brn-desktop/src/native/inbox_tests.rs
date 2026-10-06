use super::*;
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    inbox::{
        CaptureInboxRequest, InboxAvailability, InboxEntry, InboxInventory, InboxItem, InboxKind,
        InboxOriginal, InboxRead,
    },
    inbox_processing::{InboxConversionFormat, InboxConversionPreview, InboxProcessBatch},
};
use gpui_kit::{EntityInputHandler, TestSupportExt, VisualTestContext, test::TestWindowExt};
use sha2::{Digest, Sha256};
use std::fs;

const EXACT: &str = "\u{feff}# Original 日本語 õ\r\nsecond λ 🦀\r";
const LATER: &str = "\u{feff}Later local text Ελληνικά\r\nλ\r";
struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        fs::create_dir(owner.path().join("data")).unwrap();
        fs::create_dir(owner.path().join("vault")).unwrap();
        fs::write(owner.path().join("vault/current.md"), EXACT).unwrap();
        Self(owner)
    }
    fn config(&self) -> brn_workflow::app::AppConfig {
        brn_workflow::app::AppConfig {
            vault_root: Some(self.0.path().join("vault")),
            credentials_dir: Some(self.0.path().join("credentials")),
            model_dir: None,
        }
    }
    fn unchanged(&self) {
        assert_eq!(
            fs::read(self.0.path().join("vault/current.md")).unwrap(),
            EXACT.as_bytes()
        );
        assert_eq!(
            fs::read_dir(self.0.path().join("vault")).unwrap().count(),
            1
        );
        assert_eq!(
            fs::read_dir(self.0.path().join("credentials"))
                .unwrap()
                .count(),
            0
        );
    }
}
fn item(request: &CaptureInboxRequest, received: u64) -> InboxItem {
    let sha256: [u8; 32] = Sha256::digest(request.text.as_bytes()).into();
    serde_json::from_value(serde_json::json!({
        "capture": {
            "id": request.id, "kind": request.kind, "title": request.title,
            "original_name": request.original_name,
            "copy": {"directory": "/synthetic/intake", "directory_device": 11,
                "directory_inode": 21, "file_device": 11, "file_inode": 22,
                "byte_len": request.text.len(), "sha256": sha256}
        }, "received_at_ms": received
    }))
    .unwrap()
}
fn originals(count: usize) -> Vec<InboxItem> {
    (0..count)
        .map(|index| {
            item(
                &CaptureInboxRequest {
                    id: Uuid::new_v4(),
                    kind: InboxKind::Markdown,
                    title: format!("Original {index} 日本語 λ"),
                    original_name: Some("Synthetic.md".into()),
                    text: EXACT.into(),
                },
                100 + index as u64,
            )
        })
        .collect()
}
fn inventory(items: &[InboxItem]) -> InboxInventory {
    InboxInventory {
        entries: items
            .iter()
            .cloned()
            .map(|item| InboxEntry {
                item,
                availability: InboxAvailability::Available,
            })
            .collect(),
        next_after: None,
        total_count: items.len(),
        issues: vec![],
        issues_truncated: false,
    }
}
struct InboxProbe(Entity<Desktop>);
impl Render for InboxProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .id("inbox-test-pane")
                .size_full()
                .flex()
                .flex_col()
                .child(desktop.render_inbox(cx))
                .test_support()
        })
    }
}
fn open_pane(
    cx: &mut gpui_kit::TestAppContext,
    fixture: &Fixture,
    items: Vec<InboxItem>,
) -> (gpui_kit::WindowHandle<Root>, Entity<Desktop>) {
    cx.update(gpui_kit::component::init);
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    let data = fixture.0.path().join("data");
    let config = fixture.config();
    let window = cx.open_window(
        size(px(layout::WINDOW_MIN), px(layout::WINDOW_MIN)),
        move |window, cx| {
            let desktop = cx.new(|cx| {
                let mut desktop = Desktop::new(
                    data,
                    config,
                    (LayoutState::default(), Loaded::Missing),
                    window,
                    cx,
                );
                desktop.app_worker.take().unwrap().shutdown().unwrap();
                let ai = desktop.ai.as_mut().unwrap();
                ai.ready = true;
                ai.vault_bound = true;
                let (operation, _) = ai.open_inbox().unwrap();
                assert!(
                    ai.apply(operation, AppEvent::InboxItems(Box::new(inventory(&items))))
                        .is_empty()
                );
                desktop.open_doc = Some(DocRef::Inbox);
                desktop.centre_tab = CentreTab::Document;
                desktop.sync_inbox_widgets(window, cx);
                desktop
            });
            *saved.borrow_mut() = Some(desktop.clone());
            let probe = cx.new(|_| InboxProbe(desktop));
            Root::new(probe, window, cx)
        },
    );
    (window, capture.borrow().clone().unwrap())
}
fn scroll_to(visual: &mut VisualTestContext, id: &'static str) {
    use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollDelta, ScrollWheelEvent};
    visual.update(|window, cx| {
        window.render_frame(cx);
        let pane = window.find("inbox-test-pane");
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
            "{id} must be reachable at 480×480: {:?}",
            target.bounds()
        );
    });
}

#[gpui_kit::test]
fn native_inbox_capture_retry_ack_and_navigation_preserve_later_exact_typing(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let (window, desktop) = open_pane(cx, &fixture, originals(1));
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        let pane = &desktop.read(cx).inbox;
        let (title, body) = (pane.title.clone(), pane.body.clone());
        title.update(cx, |input, cx| {
            input.replace_text_in_range(Some(0..0), "Capture 日本語 λ", window, cx)
        });
        body.update(cx, |input, cx| {
            input.replace_text_in_range(Some(0..0), EXACT, window, cx)
        });
    });
    scroll_to(&mut visual, "inbox-kind-Email");
    visual.update(|window, cx| window.click("inbox-kind-Email", cx));
    scroll_to(&mut visual, "capture-inbox-original");
    // The absent test worker reports a failure after typed admission.
    visual.update(|window, cx| window.click("capture-inbox-original", cx));
    visual.run_until_parked();
    let submitted = visual.update(|_, cx| {
        desktop.update(cx, |desktop, _| {
            let ai = desktop.ai.as_mut().unwrap();
            assert!(ai.inbox_queue.capture_error.is_some());
            let (operation, AppCommand::CaptureInbox(request)) = ai.retry_capture().unwrap() else {
                panic!("typed exact retry")
            };
            assert_eq!(operation, request.id);
            assert_eq!(request.kind, InboxKind::Email);
            assert_eq!(request.title, "Capture 日本語 λ");
            assert_eq!(request.text.as_bytes(), EXACT.as_bytes());
            request
        })
    });
    visual.update(|window, cx| {
        let pane = &desktop.read(cx).inbox;
        let (title, body) = (pane.title.clone(), pane.body.clone());
        title.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "Later title 日本語", window, cx)
        });
        body.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), LATER, window, cx)
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.ai.as_mut().unwrap().apply(
                submitted.id,
                AppEvent::InboxCaptured(Box::new(item(&submitted, 200))),
            );
            desktop.sync_inbox_widgets(window, cx);
            desktop.open_doc = None;
            desktop.ai.as_mut().unwrap().close_inbox();
            desktop.open_doc = Some(DocRef::Inbox);
            let _ = desktop.ai.as_mut().unwrap().open_inbox();
            desktop.sync_inbox_widgets(window, cx);
            assert_eq!(
                desktop.inbox.title.read(cx).value().as_ref(),
                "Later title 日本語"
            );
            assert_eq!(
                desktop.inbox.body.read(cx).value().as_bytes(),
                LATER.as_bytes()
            );
            assert_eq!(desktop.inbox.kind, InboxKind::Email);
            assert_eq!(
                desktop
                    .ai
                    .as_ref()
                    .unwrap()
                    .inbox_queue
                    .capture_result
                    .as_ref()
                    .unwrap()
                    .capture
                    .id,
                submitted.id
            );
        })
    });
    fixture.unchanged();
}

#[gpui_kit::test]
fn native_inbox_original_and_preview_are_readonly_and_copy_complete_exact_bytes(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let items = originals(1);
    let original = items[0].clone();
    let (window, desktop) = open_pane(cx, &fixture, items);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.update(|window, cx| desktop.update(cx, |desktop, cx| {
        let ai = desktop.ai.as_mut().unwrap();
        let (read, _) = ai.select_inbox(original.capture.id).unwrap();
        ai.apply(read, AppEvent::InboxItem(Box::new(InboxRead { item: original.clone(), original: InboxOriginal::Available { text: EXACT.into() } })));
        let (process, AppCommand::ProcessInbox(request)) = ai.process_inbox_items(vec![original.clone()]).unwrap() else { panic!("typed process") };
        let batch: InboxProcessBatch = serde_json::from_value(serde_json::json!({
            "request": request, "queued_at_ms": 100,
            "entries": [{"outcome": {"state": "converted", "format": "verbatim_markdown_v1", "byte_len": original.capture.copy.byte_len, "sha256": original.capture.copy.sha256}, "started_at_ms": 100, "finished_at_ms": 101}]
        })).unwrap();
        batch.validate().unwrap();
        ai.apply(process, AppEvent::InboxProcessing(Box::new(batch)));
        let (operation, AppCommand::InboxCandidate(request)) = ai.preview_inbox_candidate(0).unwrap() else { panic!("typed preview") };
        ai.apply(operation, AppEvent::InboxCandidate(Box::new(InboxConversionPreview {
            request, original: original.clone(), format: InboxConversionFormat::VerbatimMarkdownV1,
            markdown: EXACT.into(), needs_semantic_review: true,
        })));
        desktop.sync_inbox_widgets(window, cx);
        cx.notify();
    }));
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let pane = &desktop.read(cx).inbox;
        for field in [pane.original.clone(), pane.preview.clone()] {
            field.update(cx, |editor, cx| {
                editor.replace_text_in_range(Some(0..0), "blocked", window, cx);
                assert_eq!(editor.value().as_bytes(), EXACT.as_bytes());
            });
        }
    });
    for button in ["copy-inbox-original", "copy-inbox-preview"] {
        scroll_to(&mut visual, button);
        visual.update(|window, cx| {
            window.click(button, cx);
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some(EXACT)
            );
        });
    }
    for unavailable in [
        InboxOriginal::RemovedRetained {
            operation_id: Uuid::new_v4(),
        },
        InboxOriginal::Missing,
        InboxOriginal::Changed {
            reason: "synthetic changed bytes".into(),
        },
        InboxOriginal::Unavailable {
            reason: "synthetic blocked original".into(),
        },
    ] {
        visual.update(|window, cx| {
            desktop.update(cx, |desktop, cx| {
                let ai = desktop.ai.as_mut().unwrap();
                let (read, _) = ai.select_inbox(original.capture.id).unwrap();
                ai.apply(
                    read,
                    AppEvent::InboxItem(Box::new(InboxRead {
                        item: original.clone(),
                        original: unavailable.clone(),
                    })),
                );
                desktop.sync_inbox_widgets(window, cx);
                assert!(desktop.inbox.original.read(cx).value().is_empty());
                cx.notify();
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("copy-inbox-original").is_none());
        });
    }
    fixture.unchanged();
}

#[gpui_kit::test]
fn native_inbox_checked_batch_is_bounded_and_keeps_exact_snapshots_after_page_change(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let items = originals(9);
    let (window, desktop) = open_pane(cx, &fixture, items.clone());
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    for button in [
        "inbox-check-0",
        "inbox-check-1",
        "inbox-check-2",
        "inbox-check-3",
        "inbox-check-4",
        "inbox-check-5",
        "inbox-check-6",
        "inbox-check-7",
        "inbox-check-8",
    ] {
        scroll_to(&mut visual, button);
        visual.update(|window, cx| window.click(button, cx));
        visual.run_until_parked();
    }
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            assert_eq!(desktop.inbox.checked, items[..8]);
            let replacement = originals(1);
            desktop.ai.as_mut().unwrap().inbox_queue.page = Some(inventory(&replacement));
            desktop.sync_inbox_widgets(window, cx);
            // A stale rendered checkbox cannot introduce an item from the old page.
            desktop.check_inbox_item(items[8].clone(), true, cx);
            assert_eq!(desktop.inbox.checked, items[..8]);
            desktop.process_checked_inbox(cx);
            // No worker is attached; the shared state retains the exact failed request.
            let (retry_id, AppCommand::ProcessInbox(request)) =
                desktop.ai.as_mut().unwrap().retry_process().unwrap()
            else {
                panic!("typed exact process retry")
            };
            assert_eq!(request.items, items[..8]);
            assert!(!request.items.iter().any(|item| item == &replacement[0]));
            assert_eq!(desktop.inbox.checked, items[..8]);
            let ai = desktop.ai.as_mut().unwrap();
            ai.apply(
                retry_id,
                AppEvent::Failed(brn_workflow::WorkflowError::msg("synthetic admission loss")),
            );
            let (refresh_id, _) = ai.refresh_inbox().unwrap();
            ai.apply(
                refresh_id,
                AppEvent::InboxItems(Box::new(inventory(&replacement))),
            );
            assert!(ai.inbox_queue.error.is_none());
            assert!(ai.can_retry_process());
            cx.notify();
        })
    });
    visual.run_until_parked();
    scroll_to(&mut visual, "retry-inbox-processing");
    visual.update(|window, cx| {
        window.click("retry-inbox-processing", cx);
        let ai = desktop.read(cx).ai.as_ref().unwrap();
        assert!(
            ai.inbox_queue.error.is_some(),
            "enabled retry must attempt its exact command despite a successful inventory refresh"
        );
    });
    fixture.unchanged();
}

#[gpui_kit::test]
fn native_binary_original_has_proof_view_and_batch_admission_without_text_copy(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let mut binary = originals(1).remove(0);
    binary.capture.kind = InboxKind::Binary;
    binary.capture.title = "Retained binary".into();
    let (window, desktop) = open_pane(cx, &fixture, vec![binary.clone()]);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    scroll_to(&mut visual, "inbox-check-0");
    visual.update(|window, cx| {
        window.click("inbox-check-0", cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            assert_eq!(desktop.inbox.checked, vec![binary.clone()]);
            desktop.check_inbox_item(binary.clone(), true, cx);
            assert_eq!(desktop.inbox.checked, vec![binary.clone()]);
            let ai = desktop.ai.as_mut().unwrap();
            let (read, _) = ai.select_inbox(binary.capture.id).unwrap();
            ai.apply(
                read,
                AppEvent::InboxItem(Box::new(InboxRead {
                    item: binary.clone(),
                    original: InboxOriginal::AvailableBinary {
                        byte_len: binary.capture.copy.byte_len,
                        sha256: binary.capture.copy.sha256,
                    },
                })),
            );
            let selected = ai.inbox_queue.selected.as_ref().unwrap();
            assert_eq!(selected.item, binary);
            assert!(matches!(
                selected.original,
                InboxOriginal::AvailableBinary { .. }
            ));
            desktop.sync_inbox_widgets(window, cx);
            assert!(desktop.inbox.original.read(cx).value().is_empty());
            cx.notify();
        });
    });
    visual.run_until_parked();
    scroll_to(&mut visual, "inbox-binary-original");
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("inbox-binary-original").is_some());
        assert!(window.try_find("copy-inbox-original").is_none());
    });
    fixture.unchanged();
}
