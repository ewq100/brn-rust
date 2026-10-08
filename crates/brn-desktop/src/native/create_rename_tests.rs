//! Real headless destination controls and protected full-review widgets.
use super::super::*;
use crate::review::{ProposalReview, create_rename_tests::renamed, predecessor_tests::fixture};
use brn_workflow::{app_worker::AppEvent, proposals::ProposalRecord};
use gpui_kit::{EntityInputHandler, TestSupportExt, VisualTestContext, test::TestWindowExt};

struct ReviewProbe(Entity<Desktop>);
impl Render for ReviewProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .id("create-rename-test-pane")
                .test_support()
                .size_full()
                .flex()
                .flex_col()
                .child(desktop.render_proposal_review(cx))
        })
    }
}
fn window(
    cx: &mut gpui_kit::TestAppContext,
    record: ProposalRecord,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<ReviewProbe>,
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
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = saved.clone();
    let handle = cx.open_window(size(px(1100.), px(1400.)), move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut desktop = Desktop::new(
                data,
                config,
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            desktop.app_worker.take().unwrap().shutdown().unwrap();
            desktop.open_doc = Some(DocRef::Proposal(record.draft.id));
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            ai.pending.clear();
            ai.review = Some(ProposalReview::new(record));
            desktop.sync_review_widgets(window, cx);
            desktop
        });
        *capture.borrow_mut() = Some(desktop.clone());
        ReviewProbe(desktop)
    });
    (fixture, handle, saved.borrow().clone().unwrap())
}

#[gpui_kit::test]
fn destination_controls_cover_each_create_and_preserve_input_on_failed_submission(
    cx: &mut gpui_kit::TestAppContext,
) {
    let mut before = crate::review::asset_tests::fixture();
    let mut extra = before.draft.changes[0].clone();
    if let brn_workflow::proposals::NoteChange::Create { path, .. } = &mut extra {
        *path = "second.md".into();
    }
    before.draft.changes.push(extra);
    let (_owner, handle, desktop) = window(cx, before.clone());
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("create-rename-controls-0").is_some());
        assert!(window.try_find("create-rename-controls-4").is_some());
        for index in 1..4 {
            assert!(
                window
                    .try_find(format!("create-rename-controls-{index}"))
                    .is_none()
            );
        }
        desktop.update(cx, |desktop, cx| {
            desktop.review_create_paths[0].update(cx, |input, cx| {
                input.set_value("owner õ.md", window, cx);
            });
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        // The real button goes through the ordinary application send boundary;
        // the deliberately closed synthetic worker yields a retained failure.
        window.click("rename-create-0", cx);
        let desktop = desktop.read(cx);
        let review = desktop.ai.as_ref().unwrap().review.as_ref().unwrap();
        assert_eq!(review.record, before);
        assert!(review.error.is_some() && !review.pending());
        assert_eq!(
            desktop.review_create_paths[0].read(cx).value().as_ref(),
            "owner õ.md"
        );
        assert_eq!(
            desktop.review_create_paths[4].read(cx).value().as_ref(),
            "second.md"
        );
    });
}

#[gpui_kit::test]
fn rename_pending_fences_real_typing_navigation_and_keeps_path_through_exact_ack(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (before, _) = fixture();
    let after = renamed(&before, 0, "owner successor.md");
    let (_owner, handle, desktop) = window(cx, before.clone());
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    let mut operation = None;
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.review_create_paths[0].update(cx, |input, cx| {
                input.set_value("owner successor.md", window, cx)
            });
            operation = Some(
                desktop
                    .ai
                    .as_mut()
                    .unwrap()
                    .rename_proposal_create(0, "owner successor.md".into())
                    .unwrap()
                    .0,
            );
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        let input = desktop.read(cx).review_title.clone();
        input.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "Buffered title", window, cx);
        });
        let input = desktop.read(cx).review_editor.clone();
        input.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "Buffered body", window, cx);
        });
        desktop.update(cx, |desktop, cx| {
            desktop.simple_leave(simple::EditorTransition::Dashboard, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            let review = desktop.ai.as_ref().unwrap().review.as_ref().unwrap();
            assert_eq!(review.title(), before.draft.title);
            assert_eq!(review.text(0), before.draft.changes[0].text());
            assert_eq!(desktop.open_doc, Some(DocRef::Proposal(before.draft.id)));
            desktop
                .ai
                .as_mut()
                .unwrap()
                .apply(Uuid::new_v4(), AppEvent::Proposal(after.clone()));
            assert_eq!(
                desktop.ai.as_ref().unwrap().review.as_ref().unwrap().record,
                before
            );
            let mut pending = operation.unwrap();
            for worker_failure in [false, true] {
                let event = if worker_failure {
                    AppEvent::Failed(brn_workflow::WorkflowError::msg("Synthetic worker failure"))
                } else {
                    let mut malformed = after.clone();
                    malformed.comments[0].text.push_str("forged");
                    AppEvent::Proposal(malformed)
                };
                desktop.ai.as_mut().unwrap().apply(pending, event);
                desktop.simple_progress_transition(cx);
                assert_eq!(desktop.open_doc, Some(DocRef::Proposal(before.draft.id)));
                assert_eq!(
                    desktop.ai.as_ref().unwrap().review.as_ref().unwrap().record,
                    before
                );
                assert!(!desktop.ai.as_ref().unwrap().review_can_leave());
                assert_eq!(
                    desktop.review_create_paths[0].read(cx).value().as_ref(),
                    "owner successor.md"
                );
                // Explicitly cancel queued navigation and retry retained input.
                desktop.simple_transition = None;
                assert!(
                    desktop
                        .ai
                        .as_mut()
                        .unwrap()
                        .review
                        .as_mut()
                        .unwrap()
                        .retry()
                );
                if !worker_failure {
                    pending = desktop
                        .ai
                        .as_mut()
                        .unwrap()
                        .rename_proposal_create(0, "owner successor.md".into())
                        .unwrap()
                        .0;
                    desktop.simple_leave(simple::EditorTransition::Dashboard, cx);
                }
            }
            let (id, _) = desktop
                .ai
                .as_mut()
                .unwrap()
                .rename_proposal_create(0, "owner successor.md".into())
                .unwrap();
            desktop
                .ai
                .as_mut()
                .unwrap()
                .apply(id, AppEvent::Proposal(after.clone()));
            desktop.sync_review_widgets(window, cx);
            assert_eq!(
                desktop.review_create_paths[0].read(cx).value().as_ref(),
                "owner successor.md"
            );
            assert_eq!(
                desktop.ai.as_ref().unwrap().review.as_ref().unwrap().record,
                after
            );
            cx.notify();
        });
    });
}

#[gpui_kit::test]
fn history_pair_has_only_successor_control_and_retains_readonly_history_after_rename(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_, before) = fixture();
    let after = renamed(&before, 0, "revised successor.md");
    let (_owner, handle, desktop) = window(cx, before.clone());
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("create-rename-controls-0").is_some());
        assert!(window.try_find("create-rename-controls-1").is_none());
        desktop.update(cx, |desktop, cx| {
            let (id, _) = desktop
                .ai
                .as_mut()
                .unwrap()
                .rename_proposal_create(0, "revised successor.md".into())
                .unwrap();
            desktop
                .ai
                .as_mut()
                .unwrap()
                .apply(id, AppEvent::Proposal(after.clone()));
            desktop.review_member = 1;
            desktop.sync_review_widgets(window, cx);
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        let editor = desktop.read(cx).review_editor.clone();
        editor.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "Do not replace History", window, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|_, cx| {
        let desktop = desktop.read(cx);
        assert_eq!(
            desktop.review_editor.read(cx).value().as_ref(),
            before.draft.changes[1].text().unwrap()
        );
        assert_eq!(
            desktop.ai.as_ref().unwrap().review.as_ref().unwrap().record,
            after
        );
    });
}

fn observe_after_failure_keeps_queued_navigation(
    cx: &mut gpui_kit::TestAppContext,
    malformed: bool,
    changed: bool,
) {
    let (before, _) = fixture();
    let (_owner, handle, desktop) = window(cx, before.clone());
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.review_create_paths[0].update(cx, |input, cx| {
                input.set_value("retained successor.md", window, cx);
            });
            let (id, _) = desktop
                .ai
                .as_mut()
                .unwrap()
                .rename_proposal_create(0, "retained successor.md".into())
                .unwrap();
            desktop.simple_leave(simple::EditorTransition::Dashboard, cx);
            let event = if malformed {
                let mut after = renamed(&before, 0, "retained successor.md");
                after.comments[0].text.push_str("forged");
                AppEvent::Proposal(after)
            } else {
                AppEvent::Failed(brn_workflow::WorkflowError::msg("Synthetic rename refusal"))
            };
            desktop.ai.as_mut().unwrap().apply(id, event);
            let mut current = before.clone();
            if changed {
                current.version += 1;
                current.updated_at_ms += 1;
                current.draft.title = "Observed newer review title".into();
            }
            // Correlate the same read command used by Observe current review.
            let (read, _) = desktop.ai.as_mut().unwrap().refresh_review().unwrap();
            desktop
                .ai
                .as_mut()
                .unwrap()
                .apply(read, AppEvent::Proposal(current));
            desktop.sync_review_widgets(window, cx);
            desktop.simple_progress_transition(cx);
            assert_eq!(desktop.open_doc, Some(DocRef::Proposal(before.draft.id)));
            assert!(matches!(
                desktop.simple_transition,
                Some(simple::EditorTransition::Dashboard)
            ));
            let review = desktop.ai.as_ref().unwrap().review.as_ref().unwrap();
            assert_eq!(review.record, before);
            assert!(!review.can_leave() && review.error.is_some());
            assert_eq!(
                desktop.review_title.read(cx).value().as_ref(),
                before.draft.title
            );
            assert_eq!(
                desktop.review_editor.read(cx).value().as_ref(),
                before.draft.changes[0].text().unwrap()
            );
            assert_eq!(
                desktop.review_create_paths[0].read(cx).value().as_ref(),
                "retained successor.md"
            );
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|_, cx| {
        desktop.update(cx, |desktop, cx| {
            // Only the owner's explicit resolution releases requested navigation.
            let review = desktop.ai.as_mut().unwrap().review.as_mut().unwrap();
            assert!(if changed {
                review.discard_local()
            } else {
                review.retry()
            });
            assert!(review.can_leave());
            desktop.simple_progress_transition(cx);
            assert_eq!(desktop.open_doc, Some(DocRef::Dashboard));
            assert!(desktop.simple_transition.is_none());
        });
    });
}

#[gpui_kit::test]
fn worker_refusal_then_same_observation_keeps_destination_until_explicit_retry(
    cx: &mut gpui_kit::TestAppContext,
) {
    observe_after_failure_keeps_queued_navigation(cx, false, false);
}

#[gpui_kit::test]
fn worker_refusal_then_changed_observation_keeps_destination_until_explicit_discard(
    cx: &mut gpui_kit::TestAppContext,
) {
    observe_after_failure_keeps_queued_navigation(cx, false, true);
}

#[gpui_kit::test]
fn malformed_ack_then_same_observation_keeps_destination_until_explicit_retry(
    cx: &mut gpui_kit::TestAppContext,
) {
    observe_after_failure_keeps_queued_navigation(cx, true, false);
}

#[gpui_kit::test]
fn malformed_ack_then_changed_observation_keeps_destination_until_explicit_discard(
    cx: &mut gpui_kit::TestAppContext,
) {
    observe_after_failure_keeps_queued_navigation(cx, true, true);
}
