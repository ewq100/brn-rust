//! Shipping-root widgets with genuine synthetic preservation evidence; no provider.
use super::*;
use brn_workflow::{
    app_worker::{AppCommand, AppEvent, AppWorker},
    inbox::{
        CaptureInboxRequest, InboxAvailability, InboxEntry, InboxInventory, InboxKind,
        InboxOriginal, InboxRead,
    },
    inbox_original_operations::{
        InboxOriginalOperation, InboxRemovalConfirmation, RemoveInboxOriginalRequest,
    },
    inbox_processing::{InboxCandidateRequest, InboxSourceRequest, ProcessInboxRequest},
    inbox_removal::InboxRemovalPreview,
    proposal_apply::ApprovalRequest,
};
use gpui_kit::{EntityInputHandler, VisualTestContext, component::WindowExt, test::TestWindowExt};

const EXACT: &str = "\u{feff}Exact original 日本語 õ\r\nsecond λ 🦀\r";
const LATER: &str = "Later unsubmitted capture Ελληνικά\r\n";
struct Fixture {
    owner: tempfile::TempDir,
    read: InboxRead,
    preview: InboxRemovalPreview,
    source: String,
    removal: Option<InboxOriginalOperation>,
}
fn response(worker: &AppWorker, id: Uuid, command: AppCommand) -> AppEvent {
    worker.submit(id, command).unwrap();
    loop {
        let (got, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if got == id {
            return match event {
                AppEvent::Failed(error) => panic!("synthetic fixture refused: {error}"),
                event => event,
            };
        }
    }
}
impl Fixture {
    fn config(&self) -> brn_workflow::app::AppConfig {
        brn_workflow::app::AppConfig {
            vault_root: Some(self.owner.path().join("vault")),
            credentials_dir: Some(self.owner.path().join("credentials")),
            model_dir: None,
        }
    }
    fn new(removed: bool) -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        std::fs::create_dir(owner.path().join("data")).unwrap();
        std::fs::create_dir(owner.path().join("vault")).unwrap();
        std::fs::write(owner.path().join("vault/untouched.md"), EXACT).unwrap();
        let config = brn_workflow::app::AppConfig {
            vault_root: Some(owner.path().join("vault")),
            credentials_dir: Some(owner.path().join("credentials")),
            model_dir: None,
        };
        let mut worker = AppWorker::start(owner.path().join("data"), config).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let request = CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: InboxKind::Text,
            title: "Synthetic copy 日本語".into(),
            original_name: Some("copy.eml".into()),
            text: EXACT.into(),
        };
        let AppEvent::InboxCaptured(item) =
            response(&worker, request.id, AppCommand::CaptureInbox(request))
        else {
            panic!("capture")
        };
        let processing = ProcessInboxRequest {
            limits: None,

            id: Uuid::new_v4(),
            items: vec![(*item).clone()],
        };
        let batch_id = processing.id;
        let AppEvent::InboxProcessing(mut batch) =
            response(&worker, batch_id, AppCommand::ProcessInbox(processing))
        else {
            panic!("process")
        };
        while batch.pending_count() != 0 {
            let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
            if id == batch_id {
                match event {
                    AppEvent::InboxProcessing(next) => batch = next,
                    AppEvent::Failed(error) => panic!("process: {error}"),
                    _ => {}
                }
            }
        }
        let source_request = InboxSourceRequest {
            candidate: InboxCandidateRequest { batch_id, index: 0 },
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "source.md".into(),
            title: "Saved exact Source".into(),
        };
        let AppEvent::InboxSourceDraft(draft) = response(
            &worker,
            Uuid::new_v4(),
            AppCommand::PrepareInboxSource(source_request),
        ) else {
            panic!("source")
        };
        let AppEvent::Proposal(proposal) =
            response(&worker, draft.id, AppCommand::CreateProposal(*draft))
        else {
            panic!("proposal")
        };
        let approval = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: proposal.stamp(),
        };
        let _ = response(
            &worker,
            approval.operation_id,
            AppCommand::ApproveProposal(approval),
        );
        let AppEvent::InboxRemovalPreview(preview) = response(
            &worker,
            Uuid::new_v4(),
            AppCommand::PreviewInboxRemoval(item.capture.id),
        ) else {
            panic!("preservation preview")
        };
        let source = preview.evidence.source.as_ref().unwrap().saved.text.clone();
        let removal = if removed {
            let request = RemoveInboxOriginalRequest {
                operation_id: Uuid::new_v4(),
                item_id: item.capture.id,
                preview_digest: preview.digest,
                previous_restore: None,
                confirmation: InboxRemovalConfirmation {
                    version: 1,
                    exact_copy_removal_intended: true,
                },
            };
            let AppEvent::InboxOriginalRemoved(record) = response(
                &worker,
                request.operation_id,
                AppCommand::RemoveInboxOriginal(request),
            ) else {
                panic!("remove")
            };
            Some(InboxOriginalOperation::Remove(record))
        } else {
            None
        };
        let AppEvent::InboxItem(read) = response(
            &worker,
            Uuid::new_v4(),
            AppCommand::InboxItem(item.capture.id),
        ) else {
            panic!("original")
        };
        worker.shutdown().unwrap();
        Self {
            owner,
            read: *read,
            preview: *preview,
            source,
            removal,
        }
    }
    fn unchanged(&self) {
        assert_eq!(
            std::fs::read(self.owner.path().join("vault/source.md")).unwrap(),
            self.source.as_bytes()
        );
        assert_eq!(
            std::fs::read(self.owner.path().join("vault/untouched.md")).unwrap(),
            EXACT.as_bytes()
        );
        assert_eq!(
            std::fs::read_dir(self.owner.path().join("credentials"))
                .unwrap()
                .count(),
            0
        );
    }
}
fn open(
    cx: &mut gpui_kit::TestAppContext,
    fixture: &Fixture,
) -> (gpui_kit::WindowHandle<Root>, Entity<Desktop>) {
    cx.update(gpui_kit::component::init);
    let config = fixture.config();
    let data = fixture.owner.path().join("data");
    let read = fixture.read.clone();
    let preview = fixture.preview.clone();
    let removal = fixture.removal.clone();
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = saved.clone();
    let window = cx.open_window(size(px(480.), px(480.)), move |window, cx| {
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
            ai.pending.clear();
            let (id, _) = ai.open_inbox().unwrap();
            ai.apply(
                id,
                AppEvent::InboxItems(Box::new(InboxInventory {
                    entries: vec![InboxEntry {
                        item: read.item.clone(),
                        availability: if removal.is_some() {
                            InboxAvailability::RemovedRetained
                        } else {
                            InboxAvailability::Available
                        },
                    }],
                    total_count: 1,
                    next_after: None,
                    issues: vec![],
                    issues_truncated: false,
                })),
            );
            let (id, _) = ai.select_inbox(read.item.capture.id).unwrap();
            ai.apply(id, AppEvent::InboxItem(Box::new(read.clone())));
            let mut commands = std::collections::VecDeque::from(ai.inspect_inbox_copy());
            while let Some((id, command)) = commands.pop_front() {
                let event = match command {
                    AppCommand::PreviewInboxRemoval(_) => {
                        AppEvent::InboxRemovalPreview(Box::new(preview.clone()))
                    }
                    AppCommand::InboxOriginalOperations(item_id) => {
                        AppEvent::InboxOriginalOperations {
                            item_id,
                            operations: removal
                                .as_ref()
                                .map(|record| vec![record.summary().unwrap()])
                                .unwrap_or_default(),
                        }
                    }
                    AppCommand::InboxOriginalRemoval(operation_id) => {
                        AppEvent::InboxOriginalRemoval {
                            operation_id,
                            record: removal.clone().map(Box::new),
                        }
                    }
                    _ => panic!("copy inspection command"),
                };
                commands.extend(ai.apply(id, event));
            }
            // Exercise the shipping secondary original-management tools explicitly.
            desktop.inbox.guided_manage = true;
            desktop.open_doc = Some(DocRef::Inbox);
            desktop.centre_tab = CentreTab::Document;
            desktop.sync_inbox_widgets(window, cx);
            desktop
        });
        *capture.borrow_mut() = Some(desktop.clone());
        desktop_root(desktop, window, cx)
    });
    let desktop = saved.borrow().clone().unwrap();
    (window, desktop)
}
fn clipboard(cx: &gpui_kit::App) -> String {
    cx.read_from_clipboard()
        .and_then(|item| item.text())
        .unwrap()
}
fn scroll_to(visual: &mut VisualTestContext, target: &'static str) {
    use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollDelta, ScrollWheelEvent};
    visual.update(|window, cx| {
        window.render_frame(cx);
        let pane = window.find("guided-inbox-focus");
        let target = window.find(target);
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
        let target = window.find(target);
        assert!(
            target.visible()
                && target.bounds().center().y >= px(0.)
                && target.bounds().center().y < window.viewport_size().height
        );
    });
}
#[gpui_kit::test]
fn copy_cleanup_source_and_complete_confirmation_are_readonly_cancel_preserves_input(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new(false);
    let (window, desktop) = open(cx, &fixture);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        let (body, source) = {
            let pane = &desktop.read(cx).inbox;
            (pane.body.clone(), pane.copy_source.clone())
        };
        body.update(cx, |editor, cx| editor.set_value(LATER, window, cx));
        source.update(cx, |editor, cx| {
            editor.replace_text_in_range(Some(0..0), "forged", window, cx);
            assert_eq!(editor.value().as_bytes(), fixture.source.as_bytes());
        });
    });
    scroll_to(&mut visual, "copy-inbox-preserved-source");
    visual.update(|window, cx| {
        window.click("copy-inbox-preserved-source", cx);
        assert_eq!(clipboard(cx), fixture.source);
    });
    scroll_to(&mut visual, "review-inbox-copy-removal");
    visual.update(|window, cx| window.click("review-inbox-copy-removal", cx));
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.has_active_dialog(cx));
        assert!(window.find("confirm-inbox-copy").visible());
        let pending = desktop.read(cx).ai.as_ref().unwrap().pending.len();
        window.click("inspect-inbox-copy", cx);
        assert!(window.has_active_dialog(cx));
        assert_eq!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .inbox_copy
                .preview
                .as_ref()
                .unwrap()
                .digest,
            fixture.preview.digest,
            "modal blocks the background inspection/refresh"
        );
        window.click("copy-inbox-copy-confirmation", cx);
        let captured: serde_json::Value = serde_json::from_str(&clipboard(cx)).unwrap();
        assert_eq!(
            captured["preservation_preview"],
            serde_json::to_value(&fixture.preview).unwrap()
        );
        assert_eq!(
            captured["request"]["remove"]["confirmation"]["exact_copy_removal_intended"],
            true
        );
        assert_eq!(
            desktop.read(cx).ai.as_ref().unwrap().pending.len(),
            pending,
            "opening/copy never submits a mutation"
        );
        window.click("cancel-inbox-copy", cx);
        assert!(!window.has_active_dialog(cx));
        assert_eq!(
            desktop.read(cx).inbox.body.read(cx).value().as_bytes(),
            LATER.as_bytes()
        );
    });
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.open_inbox_copy_confirmation(true, window, cx);
            let _ = desktop
                .ai
                .as_mut()
                .unwrap()
                .select_inbox(fixture.read.item.capture.id);
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.click("confirm-inbox-copy", cx);
        assert!(
            window.has_active_dialog(cx),
            "stale capture is not admitted"
        );
        assert!(!desktop.read(cx).ai.as_ref().unwrap().inbox_copy_pending());
        window.click("cancel-inbox-copy", cx);
    });
    fixture.unchanged();
}
#[gpui_kit::test]
fn copy_cleanup_confirm_failure_and_retry_keep_exact_request_and_restore_proof(
    cx: &mut gpui_kit::TestAppContext,
) {
    for removed in [false, true] {
        let fixture = Fixture::new(removed);
        let (window, desktop) = open(cx, &fixture);
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        visual.run_until_parked();
        let button = if removed {
            "review-inbox-copy-restore"
        } else {
            "review-inbox-copy-removal"
        };
        scroll_to(&mut visual, button);
        visual.update(|window, cx| window.click(button, cx));
        visual.run_until_parked();
        let expected = visual.update(|window, cx| {
            window.render_frame(cx);
            window.click("copy-inbox-copy-confirmation", cx);
            let captured: serde_json::Value = serde_json::from_str(&clipboard(cx)).unwrap();
            assert_eq!(
                captured["removal"],
                serde_json::to_value(&fixture.removal).unwrap()
            );
            window.click("confirm-inbox-copy", cx);
            assert!(!window.has_active_dialog(cx));
            let ai = desktop.read(cx).ai.as_ref().unwrap();
            assert!(
                ai.inbox_copy.error.is_some(),
                "detached worker cannot establish success"
            );
            assert!(ai.can_retry_inbox_copy());
            captured["request"].clone()
        });
        visual.run_until_parked();
        scroll_to(&mut visual, "retry-inbox-copy");
        visual.update(|window, cx| {
            window.click("retry-inbox-copy", cx);
            assert!(
                desktop
                    .read(cx)
                    .ai
                    .as_ref()
                    .unwrap()
                    .inbox_copy
                    .error
                    .is_some()
            );
        });
        visual.update(|_, cx| {
            desktop.update(cx, |desktop, _| {
                let (id, command) = desktop.ai.as_mut().unwrap().retry_inbox_copy().unwrap();
                let actual = match command {
                    AppCommand::RemoveInboxOriginal(request) => {
                        assert_eq!(id, request.operation_id);
                        serde_json::json!({"remove":request})
                    }
                    AppCommand::RestoreInboxOriginal(request) => {
                        assert_eq!(id, request.operation_id);
                        serde_json::json!({"restore":request})
                    }
                    _ => panic!("exact copy retry"),
                };
                assert_eq!(actual, expected);
            })
        });
        assert_eq!(
            matches!(
                &fixture.read.original,
                InboxOriginal::RemovedRetained { .. }
            ),
            removed
        );
        fixture.unchanged();
    }
}
