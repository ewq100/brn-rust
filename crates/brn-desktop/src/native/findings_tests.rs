use super::*;
use brn_workflow::findings::*;
use brn_workflow::proposals::SourceVersion;
use gpui_kit::{EntityInputHandler, TestSupportExt, VisualTestContext, test::TestWindowExt};
use std::fs;

const EXACT: &str = "\u{feff}Original 日本語 õ\r\nsecond λ\r";
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
}
fn record() -> FindingRecord {
    let quote = EXACT.repeat(200);
    let proof = SourceVersion {
        path: "current.md".into(),
        fingerprint: brn_workflow::editor::FileFingerprint {
            device: 741,
            inode: 852,
            len: (quote.len() * 2) as u64,
            sha256: [0x17; 32],
        },
    };
    let draft = FindingDraft {
        request: CaptureFindingRequest {
            id: Uuid::new_v4(),
            origin: FindingOrigin::UnresolvedLink {
                path: "current.md".into(),
                source_sha256: proof.fingerprint.sha256,
                destination: "missing.md".into(),
                start_byte: 0,
            },
        },
        vault: serde_json::from_value(serde_json::json!({"id":Uuid::new_v4(),"root":"/synthetic/vault","identity":{"device":741,"inode":800}})).unwrap(),
        title: "Full retained title 日本語\r\n".repeat(8),
        summary: "Original finding summary λ\r\n".repeat(80),
        evidence: (0..2)
            .map(|index| FindingEvidence {
                source: proof.clone(),
                note_id: None,
                quote: Some(FindingQuote {
                    start_byte: quote.len() * index,
                    end_byte: quote.len() * (index + 1),
                    quote: quote.clone(),
                }),
            })
            .collect(),
    };
    draft.validate().unwrap();
    FindingRecord {
        draft,
        version: 1,
        state: FindingState::Open,
        created_at_ms: 1234,
        updated_at_ms: 1234,
    }
}
struct FindingsProbe(Entity<Desktop>);
impl Render for FindingsProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .id("findings-test-pane")
                .size_full()
                .flex()
                .flex_col()
                .child(desktop.render_findings(cx))
                .test_support()
        })
    }
}
fn open_queue(
    cx: &mut gpui_kit::TestAppContext,
    fixture: &Fixture,
    selected: FindingRecord,
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
                ai.vault_bound = false;
                let _ = ai.open_findings();
                ai.pending.clear();
                ai.finding_queue.page = Some(FindingPage {
                    entries: (0..25)
                        .map(|i| {
                            let mut r = selected.clone();
                            if i > 0 {
                                r.draft.request.id = Uuid::new_v4();
                            }
                            r
                        })
                        .collect(),
                    next_before: Some(Uuid::new_v4()),
                    open_count: 26,
                });
                let selection = ai.select_finding(selected.draft.request.id).unwrap();
                assert!(
                    ai.apply(
                        selection.0,
                        brn_workflow::app_worker::AppEvent::Finding(Box::new(selected))
                    )
                    .is_empty()
                );
                desktop.open_doc = Some(DocRef::Findings);
                desktop.centre_tab = CentreTab::Document;
                desktop.sync_finding_widgets(window, cx);
                desktop
            });
            *saved.borrow_mut() = Some(desktop.clone());
            let probe = cx.new(|_| FindingsProbe(desktop));
            Root::new(probe, window, cx)
        },
    );
    (window, capture.borrow().clone().unwrap())
}
fn scroll_to(visual: &mut VisualTestContext, id: &'static str) {
    use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollDelta, ScrollWheelEvent};
    visual.update(|window, cx| {
        window.render_frame(cx);
        let pane = window.find("findings-test-pane");
        let target = window.find(id);
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
                delta: ScrollDelta::Pixels(point(px(0.), dy)),
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
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
fn rendered_queue_preserves_complete_readonly_proof_and_exact_copy_at_minimum_size(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let retained = record();
    let expected = findings::proof_text(&retained, None);
    let (window, desktop) = open_queue(cx, &fixture, retained.clone());
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    let entity = visual.update(|_, cx| desktop.read(cx).findings.proof.entity_id());
    scroll_to(&mut visual, "copy-full-finding-evidence");
    visual.update(|window, cx| {
        let proof = desktop.read(cx).findings.proof.clone();
        assert_eq!(proof.read(cx).value().as_ref(), expected);
        proof.update(cx, |editor, cx| {
            editor.focus(window, cx);
            editor.replace_text_in_range(Some(0..0), "blocked", window, cx);
            assert_eq!(editor.value().as_ref(), expected);
        });
        window.click("copy-full-finding-evidence", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(expected.as_str())
        );
        assert!(expected.contains("741") && expected.contains("852"));
        assert!(expected.contains(&serde_json::to_string(&retained.draft.title).unwrap()));
        assert!(
            expected.contains(
                &serde_json::to_string(&retained.draft.evidence[1].quote.as_ref().unwrap().quote)
                    .unwrap()
            )
        );
    });
    scroll_to(&mut visual, "next-finding-quote");
    visual.update(|window, cx| {
        window.click("next-finding-quote", cx);
        assert_eq!(desktop.read(cx).findings.quote_index, 1);
    });
    scroll_to(&mut visual, "copy-exact-finding-quote");
    visual.update(|window, cx| {
        window.click("copy-exact-finding-quote", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(
                retained.draft.evidence[1]
                    .quote
                    .as_ref()
                    .unwrap()
                    .quote
                    .as_str()
            )
        );
        desktop.update(cx, |desktop, cx| {
            let mut observed = retained.draft.evidence[0].source.clone();
            observed.fingerprint.inode += 1;
            desktop.ai.as_mut().unwrap().finding_queue.inspection = Some(FindingInspection {
                record: retained.clone(),
                evidence: vec![
                    FindingEvidenceObservation {
                        index: 0,
                        outcome: FindingEvidenceOutcome::Changed,
                        observed: Some(observed),
                        reason: None,
                    },
                    FindingEvidenceObservation {
                        index: 1,
                        outcome: FindingEvidenceOutcome::Unavailable,
                        observed: None,
                        reason: Some("Missing saved source".into()),
                    },
                ],
            });
            desktop.sync_finding_widgets(window, cx);
            assert_eq!(desktop.findings.proof.entity_id(), entity);
            assert!(desktop.findings.proof.read(cx).value().contains("changed"));
            assert!(
                desktop
                    .findings
                    .proof
                    .read(cx)
                    .value()
                    .contains("unavailable")
            );
            assert_eq!(
                desktop.ai.as_ref().unwrap().finding_queue.selected.as_ref(),
                Some(&retained)
            );
        });
    });
    assert_eq!(
        fs::read(fixture.0.path().join("vault/current.md")).unwrap(),
        EXACT.as_bytes()
    );
}

#[gpui_kit::test]
fn queue_closure_uses_full_selected_record_and_navigation_retains_unsubmitted_draft(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let retained = record();
    let (window, desktop) = open_queue(cx, &fixture, retained.clone());
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    scroll_to(&mut visual, "resolve-finding");
    visual.update(|window, cx| {
        window.click("resolve-finding", cx);
        // This widget fixture has no worker: submission failure must retain
        // the exact operation rather than claim a completed closure.
        let retry = desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .finding_close_retry_request()
            .cloned()
            .unwrap();
        assert_eq!(retry.expected, retained.stamp());
        assert_eq!(retry.state, FindingState::Resolved);
        assert_eq!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .finding_queue
                .selected
                .as_ref(),
            Some(&retained)
        );
        desktop.update(cx, |desktop, _| {
            assert!(desktop.ai.as_mut().unwrap().retry_finding_close().is_some());
        });
        let count = desktop.read(cx).ai.as_ref().unwrap().pending.len();
        desktop.update(cx, |desktop, cx| {
            desktop.close_finding(FindingState::Dismissed, cx);
        });
        assert_eq!(desktop.read(cx).ai.as_ref().unwrap().pending.len(), count);
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.pending.clear();
            let mut draft = crate::draft::DraftForm::new(None).unwrap();
            draft.edit(
                "Unacknowledged title 日本語".into(),
                "new.md".into(),
                "Full unsubmitted body õ\r\n".into(),
                crate::draft::DraftKind::Create,
            );
            ai.draft = Some(draft);
            desktop.open_doc = Some(DocRef::Draft);
            desktop.simple_leave(simple::EditorTransition::Findings, cx);
            assert_eq!(desktop.open_doc, Some(DocRef::Draft));
            assert_eq!(
                desktop.ai.as_ref().unwrap().draft.as_ref().unwrap().text,
                "Full unsubmitted body õ\r\n"
            );
        });
    });
    assert_eq!(
        fs::read(fixture.0.path().join("vault/current.md")).unwrap(),
        EXACT.as_bytes()
    );
}

#[gpui_kit::test]
fn refused_draft_navigation_keeps_findings_visible_and_usable(cx: &mut gpui_kit::TestAppContext) {
    let fixture = Fixture::new();
    let (window, desktop) = open_queue(cx, &fixture, record());
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|_, cx| {
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.vault_bound = true;
            ai.active = Some(crate::ai::ActiveTurn {
                request: brn_workflow::chat_worker::AskRequest {
                    id: Uuid::new_v4(),
                    conversation: None,
                    question: "Synthetic active turn; never submitted".into(),
                    selection: brn_workflow::Selection {
                        provider: brn_workflow::Provider::Chatgpt,
                        model: "synthetic-model".into(),
                    },
                    effort: Some(brn_workflow::ReasoningEffort::Low),
                    generation: 1,
                },
                partial: String::new(),
                tool: None,
                stopping: false,
            });
            assert!(!ai.application_busy());
            desktop.simple_leave(simple::EditorTransition::Draft(None), cx);
            assert_eq!(desktop.open_doc, Some(DocRef::Findings));
            let ai = desktop.ai.as_mut().unwrap();
            assert!(
                ai.finding_queue.visible,
                "refused navigation must retain the visible queue"
            );
            assert!(
                ai.refresh_findings(Some(FindingState::Open), None)
                    .is_some()
            );
        });
    });
}

#[gpui_kit::test]
fn exact_failed_requests_remain_visible_copyable_and_retryable_without_selection_or_vault(
    cx: &mut gpui_kit::TestAppContext,
) {
    use brn_workflow::app_worker::{AppCommand, AppEvent};
    let fixture = Fixture::new();
    let retained = record();
    let capture = retained.draft.request.clone();
    let closure = CloseFindingRequest {
        expected: retained.stamp(),
        state: FindingState::Dismissed,
    };
    let (window, desktop) = open_queue(cx, &fixture, retained);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            let failed = || {
                AppEvent::Failed(brn_workflow::WorkflowError {
                    kind: brn_workflow::ErrorKind::Cancelled,
                    message: "Synthetic lost acknowledgement".into(),
                })
            };
            let operation = ai.command(
                crate::ai::Pending::FindingCapture(capture.clone()),
                AppCommand::CaptureFinding(capture.clone()),
            );
            assert!(ai.apply(operation.0, failed()).is_empty());
            let operation = ai.close_selected_finding(FindingState::Dismissed).unwrap();
            assert!(ai.apply(operation.0, failed()).is_empty());
            ai.close_findings();
            let _ = ai.open_findings();
            ai.pending.clear();
            ai.finding_queue.page = None;
            assert!(ai.finding_queue.selected.is_none());
            assert!(!ai.vault_bound);
            desktop.sync_finding_widgets(window, cx);
            let displayed = desktop.findings.proof.read(cx).value();
            assert!(displayed.contains(&serde_json::to_string_pretty(&capture).unwrap()));
            assert!(displayed.contains(&serde_json::to_string_pretty(&closure).unwrap()));
        });
    });
    scroll_to(&mut visual, "copy-full-finding-evidence");
    visual.update(|window, cx| {
        window.click("copy-full-finding-evidence", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(desktop.read(cx).findings.proof.read(cx).value().as_ref())
        );
    });
    scroll_to(&mut visual, "retry-finding-capture");
    visual.update(|window, cx| {
        window.click("retry-finding-capture", cx);
        assert_eq!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .finding_capture_retry_request(),
            Some(&capture)
        );
    });
    scroll_to(&mut visual, "retry-finding-close");
    visual.update(|window, cx| {
        window.click("retry-finding-close", cx);
        assert_eq!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .finding_close_retry_request(),
            Some(&closure)
        );
    });
    assert_eq!(
        fs::read(fixture.0.path().join("vault/current.md")).unwrap(),
        EXACT.as_bytes()
    );
}
