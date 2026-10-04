//! Actual field widgets and immutable modal captures; no graphical/provider calls.
use super::*;
use crate::review::{
    ProposalReview,
    action_fields::{ActionFields, LABELS},
    action_tests::reply,
};
use gpui_kit::{EntityInputHandler, VisualTestContext, component::WindowExt, test::TestWindowExt};

fn window(
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
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = saved.clone();
    let window = cx.open_window(size(px(1100.), px(800.)), move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut this = Desktop::new(
                data,
                config,
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            this.app_worker.take().unwrap().shutdown().unwrap();
            let record = crate::review::action_tests::fixture();
            let ai = this.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            ai.review = Some(ProposalReview::new(record.clone()));
            this.open_doc = Some(DocRef::Proposal(record.draft.id));
            this.centre_tab = CentreTab::Document;
            this.sync_review_widgets(window, cx);
            this
        });
        *capture.borrow_mut() = Some(desktop.clone());
        desktop_root(desktop, window, cx)
    });
    let desktop = saved.borrow().clone().unwrap();
    (fixture, window, desktop)
}

fn assert_data(window: &gpui_kit::Window, scope: &str, data: &brn_workflow::actions::ActionData) {
    let form = ActionFields::from(data);
    for (field, label) in LABELS.iter().enumerate() {
        assert_eq!(
            window.find(format!("{scope}-field-{field}")).label(),
            Some(format!("{label}: {}", form.values[field]).as_str())
        );
    }
    assert_eq!(
        window.find(format!("{scope}-state")).label(),
        Some(format!("State: {:?}", data.state).as_str())
    );
    assert_eq!(
        window.find(format!("{scope}-priority")).label(),
        Some(format!("Priority: {:?}", data.priority).as_str())
    );
}

#[gpui_kit::test]
fn native_fields_preserve_incomplete_input_late_typing_and_all_ordered_members(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let this = desktop.read(cx);
        let review = this.ai.as_ref().unwrap().review.as_ref().unwrap();
        for (index, form) in review.action_fields().iter().enumerate() {
            for (field, value) in form.values.iter().enumerate() {
                assert_eq!(
                    this.action_editors.fields[index][field]
                        .read(cx)
                        .value()
                        .as_ref(),
                    value
                );
                assert!(
                    window
                        .try_find(format!("action-field-{index}-{field}"))
                        .is_some()
                );
            }
            assert_data(
                window,
                &format!("action-{index}-proposed"),
                &review.action_data()[index],
            );
        }
        if let brn_workflow::proposals::ActionChange::Replace { before, .. } =
            &review.record.draft.action_changes[1]
        {
            assert_data(window, "action-1-before", &before.data);
            assert_data(window, "action-1-origin", &before.origin.data);
        }
        assert!(window.try_find("action-state-0-Completed").is_none());
        let input = this.action_editors.fields[0][3].clone();
        input.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "incomplete-", window, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| {
            let review = this.ai.as_mut().unwrap().review.as_mut().unwrap();
            assert_eq!(review.action_fields()[0].values[3], "incomplete-");
            assert!(!review.can_leave() && review.prepare_edit().is_none());
            this.sync_review_widgets(window, cx);
            assert_eq!(
                this.action_editors.fields[0][3].read(cx).value().as_ref(),
                "incomplete-"
            );
            let input = this.action_editors.fields[0][3].clone();
            input.update(cx, |input, cx| {
                let end = input.value().encode_utf16().count();
                input.replace_text_in_range(
                    Some(0..end),
                    "00000000-0000-0000-0000-000000000020",
                    window,
                    cx,
                );
            });
        })
    });
    visual.run_until_parked();
    let mut submitted = None;
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| {
            let review = this.ai.as_mut().unwrap().review.as_mut().unwrap();
            review.retry();
            let baseline = review.record.clone();
            let (id, edit) = review.prepare_edit().unwrap();
            submitted = Some((id, reply(&baseline, &edit)));
            let input = this.action_editors.fields[1][0].clone();
            input.update(cx, |input, cx| {
                let end = input.value().encode_utf16().count();
                input.replace_text_in_range(Some(end..end), " late 🦀\r\n", window, cx);
            });
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| {
            let review = this.ai.as_mut().unwrap().review.as_mut().unwrap();
            let (id, record) = submitted.take().unwrap();
            assert!(review.acknowledge_edit(id, record));
            assert!(review.action_data()[1].title.ends_with(" late 🦀\r\n"));
            assert!(!review.can_leave());
            this.sync_review_widgets(window, cx);
            assert!(
                this.action_editors.fields[1][0]
                    .read(cx)
                    .value()
                    .ends_with(" late 🦀\r\n")
            );
        })
    });
}

#[gpui_kit::test]
fn exact_approval_modal_renders_all_action_after_before_and_origin_fields(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    let record = visual.update(|_, cx| {
        desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .unwrap()
            .record
            .clone()
    });
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| this.open_approval_dialog(false, window, cx))
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.has_active_dialog(cx));
        desktop.update(cx, |this, cx| {
            let mut later = record.clone();
            later.version += 1;
            later.draft.action_changes[0].data_mut().title = "Later uncaptured title".into();
            this.ai.as_mut().unwrap().review = Some(ProposalReview::new(later));
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let modal = window.within("exact-approval-capture");
        for (index, change) in record.draft.action_changes.iter().enumerate() {
            let kind = if matches!(change, brn_workflow::proposals::ActionChange::Create { .. }) {
                "Create"
            } else {
                "Replace"
            };
            assert_eq!(
                modal.find(format!("captured-action-{index}")).label(),
                Some(format!("{kind} UUID {}", change.id()).as_str())
            );
            let mut captures = vec![(format!("action-{index}-proposed"), change.data())];
            if let brn_workflow::proposals::ActionChange::Replace { before, .. } = change {
                assert_eq!(modal.find(format!("action-{index}-metadata-0")).label(), Some(format!("Full captured before Action {} · version {} · updated at {} ms", before.origin.id, before.version, before.updated_at_ms).as_str()));
                assert_eq!(modal.find(format!("action-{index}-metadata-1")).label(), Some(format!("Waiting since: {:?} ms · completed at: {:?} ms", before.waiting_since_ms, before.completed_at_ms).as_str()));
                assert_eq!(modal.find(format!("action-{index}-metadata-2")).label(), Some(format!("Immutable origin: {} · creating proposal {} version {} · created at {} ms", before.origin.id, before.origin.proposal.id, before.origin.proposal.version, before.origin.created_at_ms).as_str()));
                captures.push((format!("action-{index}-before"), &before.data));
                captures.push((format!("action-{index}-origin"), &before.origin.data));
            }
            for (scope, data) in captures {
                let form = ActionFields::from(data);
                for (field, label) in LABELS.iter().enumerate() {
                    assert_eq!(
                        modal.find(format!("{scope}-field-{field}")).label(),
                        Some(format!("{label}: {}", form.values[field]).as_str())
                    );
                }
                assert_eq!(
                    modal.find(format!("{scope}-state")).label(),
                    Some(format!("State: {:?}", data.state).as_str())
                );
                assert_eq!(
                    modal.find(format!("{scope}-priority")).label(),
                    Some(format!("Priority: {:?}", data.priority).as_str())
                );
            }
        }
        window.close_dialog(cx);
    });
}

#[gpui_kit::test]
fn action_input_modal_and_quit_guards_preserve_retained_bytes(cx: &mut gpui_kit::TestAppContext) {
    let (_fixture, handle, desktop) = window(cx);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| {
            let ai = this.ai.as_mut().unwrap();
            ai.selection = Some(brn_workflow::Selection {
                provider: brn_workflow::Provider::Chatgpt,
                model: "gpt-6-luna".into(),
            });
            ai.effort = Some(brn_workflow::ReasoningEffort::Low);
            assert!(ai.can_rewrite());
            ai.notice = "rewrite-disabled-sentinel".into();
            cx.notify();
        });
        window.render_frame(cx);
        let panel = window.find("full-proposal-review");
        let target = window.find("review-rewrite");
        window.scroll(
            "full-proposal-review",
            gpui_kit::ScrollDelta::Pixels(point(
                px(0.),
                panel.bounds().origin.y + px(40.) - target.bounds().origin.y,
            )),
            cx,
        );
        window.render_frame(cx);
        window.click("review-rewrite", cx);
        assert_eq!(
            desktop.read(cx).ai.as_ref().unwrap().notice,
            "rewrite-disabled-sentinel"
        );
        window.open_dialog(cx, |dialog, _, _| {
            dialog.title("Synthetic modal guard").child("Modal capture")
        });
        let input = desktop.read(cx).action_editors.fields[0][0].clone();
        input.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "background input", window, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |this, _| {
            assert!(!this.ai.as_ref().unwrap().review.as_ref().unwrap().dirty());
        });
        window.close_dialog(cx);
        let input = desktop.read(cx).action_editors.fields[0][3].clone();
        input.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "partial-uuid", window, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|_, cx| {
        desktop.update(cx, |this, cx| {
            this.simple_leave(simple::EditorTransition::Close(CloseRoute::Quit), cx);
            assert!(this.closing.is_none() && !this.closed && this.simple_transition.is_some());
            let review = this.ai.as_ref().unwrap().review.as_ref().unwrap();
            assert_eq!(review.action_fields()[0].values[3], "partial-uuid");
            let copy: serde_json::Value =
                serde_json::from_str(&review.copy_local().unwrap()).unwrap();
            assert_eq!(copy["local_action_fields"][0]["values"][3], "partial-uuid");
            assert_eq!(copy["edit"]["action_data"].as_array().unwrap().len(), 2);
        })
    });
}
