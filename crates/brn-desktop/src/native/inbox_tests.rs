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
struct InboxProbe(Entity<Desktop>, bool);
impl Render for InboxProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .id("inbox-test-pane")
                .size_full()
                .flex()
                .flex_col()
                .child(if self.1 {
                    desktop.render_inbox_advanced(cx)
                } else {
                    desktop.render_inbox(cx)
                })
                .test_support()
        })
    }
}
fn open_pane(
    cx: &mut gpui_kit::TestAppContext,
    fixture: &Fixture,
    items: Vec<InboxItem>,
) -> (gpui_kit::WindowHandle<Root>, Entity<Desktop>) {
    open_pane_mode(cx, fixture, items, true)
}
fn open_pane_mode(
    cx: &mut gpui_kit::TestAppContext,
    fixture: &Fixture,
    items: Vec<InboxItem>,
    advanced: bool,
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
            let probe = cx.new(|_| InboxProbe(desktop, advanced));
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
            extraction: None,
            visual: None,
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

#[gpui_kit::test]
fn native_plural_extraction_shows_repeated_images_sources_and_visible_gaps(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let original = originals(1).remove(0);
    let image = include_bytes!(
        "../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/water-use.png"
    )
    .to_vec();
    let sha256 = brn_intake::digest(&image);
    let (width, height) = brn_intake::validate_png_image(&image).unwrap();
    let asset = brn_intake::ImageAsset {
        id: format!("asset-{}", brn_intake::hex(&sha256)),
        sha256,
        width,
        height,
        media_type: "image/png".into(),
        bytes: image,
    };
    let link = format!("![image]({})", brn_intake::asset_file_name(&asset).unwrap());
    let markdown = format!("First actual picture\n{link}\nRepeated actual picture\n{link}\n");
    let first = markdown.find(&link).unwrap();
    let second = markdown.rfind(&link).unwrap();
    let extraction = brn_intake::Extraction {
        limits: Default::default(),
        consumed: None,
        schema: 1,
        converter: brn_intake::CONVERTER.into(),
        original_sha256: original.capture.copy.sha256,
        markdown: markdown.clone(),
        sources: vec![
            brn_intake::SourceNode {
                id: "source-0".into(),
                parent: None,
                name: "synthetic retained original".into(),
                media_type: "text/plain".into(),
                locator: "original".into(),
                status: "partial".into(),
                bytes: EXACT.as_bytes().to_vec(),
                text: markdown.clone(),
            },
            brn_intake::SourceNode {
                id: "source-1".into(),
                parent: Some("source-0".into()),
                name: "forecast.xlsx".into(),
                media_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                    .into(),
                locator: "mime/part/2".into(),
                status: "unprocessed".into(),
                bytes: b"opaque synthetic XLSX".to_vec(),
                text: String::new(),
            },
        ],
        assets: vec![asset.clone()],
        occurrences: vec![
            brn_intake::ImageOccurrence {
                id: "occurrence-0".into(),
                source_id: "source-0".into(),
                asset_id: asset.id.clone(),
                locator: "document/image/1".into(),
                alt: Some("120 to 72 litres/day".into()),
                start: first,
                end: first + link.len(),
            },
            brn_intake::ImageOccurrence {
                id: "occurrence-1".into(),
                source_id: "source-0".into(),
                asset_id: asset.id.clone(),
                locator: "document/image/2".into(),
                alt: Some("Repeated operational evidence".into()),
                start: second,
                end: second + link.len(),
            },
        ],
        gaps: vec![
            "Unsupported XLSX remains retained and unprocessed.".into(),
            "Native chart content unavailable; inspect original.".into(),
        ],
    };
    extraction.validate().unwrap();
    let (window, desktop) = open_pane(cx, &fixture, vec![original.clone()]);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.ai.as_mut().unwrap().inbox_queue.preview = Some(InboxConversionPreview {
                extraction: Some(extraction.clone()),
                visual: None,
                request: brn_workflow::inbox_processing::InboxCandidateRequest {
                    batch_id: Uuid::new_v4(),
                    index: 0,
                },
                original: original.clone(),
                format: InboxConversionFormat::MaintainedExtractionV1,
                markdown: markdown.clone(),
                needs_semantic_review: true,
            });
            desktop.sync_inbox_widgets(window, cx);
            cx.notify();
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        for id in [
            "import-intake-file",
            "intake-extraction-summary",
            "intake-source-0",
            "intake-source-1",
            "intake-image-0",
            "intake-image-1",
            "intake-gap-0",
            "intake-gap-1",
        ] {
            assert!(
                window.try_find(id).is_some(),
                "missing plural inspection widget {id}"
            );
        }
        assert!(
            window.try_find("inbox-preview-png").is_none(),
            "New extraction is a collection, not the historical singleton."
        );
        assert_eq!(
            desktop.read(cx).inbox.preview.read(cx).value().as_ref(),
            markdown
        );
    });
    fixture.unchanged();
}

#[gpui_kit::test]
fn native_retained_analysis_reopens_plural_images_without_a_queue_preview(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, snapshot, record) =
        crate::ai::inbox_analysis_state_tests::retained_analysis_fixture();
    let fixture = Fixture(owner);
    assert!(!snapshot.original.capture.copy.directory.exists());
    let (window, desktop) = open_pane(cx, &fixture, vec![]);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.inbox.body.update(cx, |editor, cx| editor.set_value(LATER, window, cx));
            let ai = desktop.ai.as_mut().unwrap();
            assert!(ai.inbox_queue.preview.is_none());
            let (id, _) = ai.inspect_inbox_analysis(record.job.capture.id).unwrap();
            ai.apply(id, AppEvent::InboxActionAnalysis(Box::new(record.clone())));
            let (id, command) = ai.inspect_retained_extraction().unwrap();
            assert!(matches!(command, AppCommand::InboxExtraction(requested) if requested == snapshot.id));
            ai.apply(id, AppEvent::InboxExtraction(Box::new(snapshot.clone())));
            desktop.sync_inbox_widgets(window, cx);
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        for id in [
            "inbox-inspect-retained-extraction",
            "retained-intake-extraction",
            "intake-extraction-summary",
            "intake-source-0",
            "intake-image-0",
            "intake-image-1",
            "intake-gap-0",
        ] {
            assert!(
                window.try_find(id).is_some(),
                "missing reopened retained extraction widget {id}"
            );
        }
        let desktop = desktop.read(cx);
        assert!(desktop.ai.as_ref().unwrap().inbox_queue.preview.is_none());
        assert_eq!(
            desktop.inbox.retained_extraction.read(cx).value().as_ref(),
            snapshot.extraction.markdown
        );
        assert_eq!(desktop.inbox.body.read(cx).value().as_ref(), LATER);
        assert_eq!(
            desktop.inbox.intake_images.len(),
            1,
            "two repeated occurrences share one checked image allocation"
        );
    });
}

#[gpui_kit::test]
fn guided_sidebar_shows_twenty_sixth_import_and_preserves_per_item_source_input(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let all = originals(26);
    let latest = all[25].clone();
    let first = all[0].clone();
    let (window, desktop) = open_pane_mode(cx, &fixture, all[..25].to_vec(), false);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.inbox_queue.capture_result = Some(latest.clone());
            ai.inbox_queue.guided.selected = Some(latest.capture.id);
            ai.inbox_queue.guided.selected_item = Some(latest.clone());
            ai.inbox_queue.selected = Some(InboxRead {
                item: latest.clone(),
                original: InboxOriginal::Available { text: EXACT.into() },
            });
            desktop.sync_inbox_widgets(window, cx);
            desktop.inbox.source_path.update(cx, |input, cx| {
                input.set_value("owner-chosen.md", window, cx)
            });
            cx.notify();
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let latest_id = format!("guided-item-{}", latest.capture.id);
        assert!(
            window
                .find(gpui_kit::SharedString::from(latest_id))
                .visible(),
            "import must appear immediately, including beyond the FIFO page"
        );
        assert!(window.try_find("inbox-guided-content").is_some());
        assert!(window.try_find("guided-import-file").is_some());
        assert!(
            window.try_find("intake-extraction-summary").is_none(),
            "technical evidence is disclosed on demand"
        );
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.inbox_queue.guided.selected = Some(first.capture.id);
            ai.inbox_queue.selected = Some(InboxRead {
                item: first.clone(),
                original: InboxOriginal::Available { text: EXACT.into() },
            });
            desktop.sync_inbox_widgets(window, cx);
            let ai = desktop.ai.as_mut().unwrap();
            ai.inbox_queue.guided.selected = Some(latest.capture.id);
            ai.inbox_queue.selected = Some(InboxRead {
                item: latest.clone(),
                original: InboxOriginal::Available { text: EXACT.into() },
            });
            desktop.sync_inbox_widgets(window, cx);
            assert_eq!(
                desktop.inbox.source_path.read(cx).value().as_ref(),
                "owner-chosen.md"
            );
            desktop.inbox.guided_advanced = true;
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("guided-back-from-tools").is_some());
        assert!(
            window.try_find("inbox-guided-content").is_none(),
            "advanced tools must not duplicate the guided controls"
        );
    });
    fixture.unchanged();
}

#[test]
fn guided_original_inspection_targets_selected_attachment_without_parent_fallback() {
    let (_owner, mut snapshot, _) =
        crate::ai::inbox_analysis_state_tests::retained_analysis_fixture();
    let root = snapshot.extraction.sources[0].id.clone();
    let mut attachment = snapshot.extraction.sources[0].clone();
    attachment.id = "selected-docx".into();
    attachment.parent = Some(root.clone());
    attachment.name = "harbor.docx".into();
    attachment.media_type =
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document".into();
    snapshot.extraction.sources.push(attachment.clone());
    assert_eq!(
        inbox_guided::inspection_source(&snapshot.extraction, None)
            .unwrap()
            .id,
        root
    );
    assert_eq!(
        inbox_guided::inspection_source(&snapshot.extraction, Some(&attachment.id)),
        Some(&attachment)
    );
    assert!(
        inbox_guided::inspection_source(&snapshot.extraction, Some("stale-attachment")).is_none()
    );
}

#[test]
fn guided_slide_inspection_reaches_only_its_exact_retained_presentation_parent() {
    let (_owner, mut snapshot, _) =
        crate::ai::inbox_analysis_state_tests::retained_analysis_fixture();
    let root = snapshot.extraction.sources[0].clone();
    let mut deck = root.clone();
    deck.id = "presentation-parent".into();
    deck.parent = Some(root.id.clone());
    deck.media_type =
        "application/vnd.openxmlformats-officedocument.presentationml.presentation".into();
    let mut slide = deck.clone();
    slide.id = "slide-part".into();
    slide.parent = Some(deck.id.clone());
    slide.media_type =
        "application/vnd.openxmlformats-officedocument.presentationml.slide+xml".into();
    let mut shape = slide.clone();
    shape.id = "shape-part".into();
    shape.parent = Some(slide.id.clone());
    shape.media_type = "application/x-brn-pptx-extracted-shape".into();
    shape.bytes.clear();
    snapshot
        .extraction
        .sources
        .extend([deck.clone(), slide.clone(), shape.clone()]);
    assert_eq!(
        inbox_guided::inspection_source(&snapshot.extraction, Some(&shape.id)),
        Some(&deck)
    );
    assert_eq!(
        inbox_guided::inspection_source(&snapshot.extraction, Some(&slide.id)),
        Some(&deck)
    );
    assert_eq!(
        inbox_guided::inspection_source(&snapshot.extraction, Some(&deck.id)),
        Some(&deck)
    );
    // Broken/cyclic part ancestry must not fall back to the email or another deck.
    let last = snapshot.extraction.sources.len() - 1;
    snapshot.extraction.sources[last].parent = Some("missing-parent".into());
    assert!(inbox_guided::inspection_source(&snapshot.extraction, Some(&shape.id)).is_none());
    snapshot.extraction.sources[last].parent = Some(shape.id.clone());
    assert!(inbox_guided::inspection_source(&snapshot.extraction, Some(&shape.id)).is_none());
}
