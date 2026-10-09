use super::*;
use brn_workflow::{
    actions::ActionState,
    app_worker::{AppCommand, AppEvent},
    dashboard::DashboardFilter,
    proposals::{DraftRequest, ProposalRecord},
};
use gpui_kit::{EntityInputHandler, VisualTestContext, component::WindowExt, test::TestWindowExt};

fn scroll(visual: &mut VisualTestContext, pane: &str, id: &str) {
    use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollDelta, ScrollWheelEvent};
    visual.update(|window, cx| {
        window.render_frame(cx);
        let origin = window.find(pane.to_owned()).bounds().origin;
        let target = window.find(id.to_owned()).bounds();
        let position = origin + point(px(3.), px(3.));
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
                delta: ScrollDelta::Pixels(point(px(0.), origin.y + px(40.) - target.origin.y)),
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let target = window.find(id.to_owned());
        assert!(
            target.visible()
                && target.bounds().center().y >= px(0.)
                && target.bounds().center().y < window.viewport_size().height,
            "{id}: {:?}",
            target.bounds()
        );
    });
}
fn begin(desktop: &Entity<Desktop>, window: &mut Window, cx: &mut App) {
    desktop.update(cx, |this, cx| {
        assert!(this.ai.as_mut().unwrap().begin_action_draft(None));
        this.open_doc = Some(DocRef::Draft);
        this.sync_draft_widgets(window, cx);
        cx.notify();
    });
}
fn type_field(
    desktop: &Entity<Desktop>,
    index: usize,
    value: &str,
    window: &mut Window,
    cx: &mut App,
) {
    let input = desktop.read(cx).initial_action.fields[index].clone();
    let len = input.read(cx).value().encode_utf16().count();
    input.update(cx, |input, cx| {
        input.replace_text_in_range(Some(0..len), value, window, cx)
    });
}
fn record(request: &DraftRequest) -> ProposalRecord {
    let mut record = crate::review::action_tests::fixture();
    record.draft.id = request.id;
    record.draft.group_id = request.group_id;
    record.draft.session_id = request.session_id;
    record.draft.title = request.title.clone();
    record.draft.changes.clear();
    record.draft.sources = request.sources.clone();
    record.draft.vault = None;
    record.draft.action_changes = request.action_changes.clone();
    record
}

#[gpui_kit::test]
fn full_action_fields_invalid_typing_and_exact_copy_are_reachable_at_480(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = dashboard_tests::action_window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.update(|window, cx| begin(&desktop, window, cx));
    visual.run_until_parked();
    let mut values = crate::review::action_fields::ActionFields::from(
        crate::review::action_tests::fixture().draft.action_changes[0].data(),
    )
    .values;
    values[3] = "incomplete-UUID λ\r\n".into();
    visual.update(|window, cx| {
        for (index, value) in values.iter().enumerate() {
            type_field(&desktop, index, value, window, cx);
        }
    });
    visual.run_until_parked();
    for index in 0..12 {
        scroll(
            &mut visual,
            "initial-action-form",
            &format!("initial-action-field-{index}"),
        );
    }
    for id in [
        "initial-action-state-Waiting",
        "initial-action-priority-High",
        "capture-initial-action-source",
        "copy-initial-action-input",
        "create-initial-review-draft",
        "discard-initial-full-form",
    ] {
        scroll(&mut visual, "initial-action-form", id);
    }
    scroll(
        &mut visual,
        "initial-action-form",
        "copy-initial-action-input",
    );
    visual.update(|window, cx| {
        window.click("copy-initial-action-input", cx);
        let raw: serde_json::Value =
            serde_json::from_str(&cx.read_from_clipboard().unwrap().text().unwrap()).unwrap();
        assert_eq!(
            raw["action"]["fields"]["values"],
            serde_json::to_value(&values).unwrap()
        );
        desktop.update(cx, |this, cx| {
            this.simple_leave(simple::EditorTransition::Dashboard, cx)
        });
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::Draft));
        assert!(desktop.read(cx).simple_transition.is_none());
    });
    scroll(
        &mut visual,
        "initial-action-form",
        "create-initial-review-draft",
    );
    visual.update(|window, cx| {
        window.click("create-initial-review-draft", cx);
        let this = desktop.read(cx);
        let form = this.ai.as_ref().unwrap().draft.as_ref().unwrap();
        assert!(form.submitted.is_none());
        assert!(form.error.is_some());
        assert_eq!(form.action.as_ref().unwrap().fields.values, values);
    });
}

#[gpui_kit::test]
fn pending_ack_keeps_later_widgets_and_explicit_separation_uses_new_ids(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = dashboard_tests::action_window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.update(|window, cx| begin(&desktop, window, cx));
    visual.run_until_parked();
    let mut capture = None;
    visual.update(|window, cx| {
        type_field(&desktop, 0, "\u{feff}Exact Action õ\r\n", window, cx);
        desktop.update(cx, |this, cx| {
            this.capture_draft_widgets(cx);
            let (id, AppCommand::CreateProposal(request)) =
                this.ai.as_mut().unwrap().create_draft().unwrap()
            else {
                panic!("create")
            };
            capture = Some((id, request));
        });
        type_field(&desktop, 1, "Later local 日本語\r\n", window, cx);
    });
    let (outer, request) = capture.unwrap();
    let result = record(&request);
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| {
            let mut wrong = result.clone();
            wrong.draft.action_changes.clear();
            this.ai
                .as_mut()
                .unwrap()
                .apply(outer, AppEvent::Proposal(wrong));
            assert!(this.ai.as_ref().unwrap().draft.as_ref().unwrap().pending);
            this.ai
                .as_mut()
                .unwrap()
                .apply(outer, AppEvent::Proposal(result));
            this.sync_draft_widgets(window, cx);
            assert_eq!(
                this.initial_action.fields[1].read(cx).value().as_ref(),
                "Later local 日本語\r\n"
            );
            assert!(
                !this
                    .ai
                    .as_ref()
                    .unwrap()
                    .draft
                    .as_ref()
                    .unwrap()
                    .can_leave()
            );
        });
    });
    visual.run_until_parked();
    scroll(
        &mut visual,
        "initial-action-form",
        "copy-initial-action-input",
    );
    visual.update(|window, cx| {
        window.click("copy-initial-action-input", cx);
        let raw: serde_json::Value =
            serde_json::from_str(&cx.read_from_clipboard().unwrap().text().unwrap()).unwrap();
        assert_eq!(
            raw["submitted_request"],
            serde_json::to_value(&request).unwrap()
        );
        assert_eq!(
            raw["action"]["fields"]["values"][1],
            "Later local 日本語\r\n"
        );
    });
    scroll(
        &mut visual,
        "initial-action-form",
        "separate-initial-proposal",
    );
    visual.update(|window, cx| {
        window.click("separate-initial-proposal", cx);
        let this = desktop.read(cx);
        let form = this.ai.as_ref().unwrap().draft.as_ref().unwrap();
        assert_ne!(form.id, request.id);
        assert_ne!(
            form.action.as_ref().unwrap().id,
            request.action_changes[0].id()
        );
        assert_eq!(
            form.action.as_ref().unwrap().fields.values[1],
            "Later local 日本語\r\n"
        );
        assert!(form.submitted.is_none());
        assert!(!form.can_leave());
    });
}

#[gpui_kit::test]
fn completed_dashboard_selection_seeds_new_open_related_work(cx: &mut gpui_kit::TestAppContext) {
    let (_fixture, handle, desktop) = dashboard_tests::action_window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    let mut completed = crate::ai::dashboard_state_tests::record(200);
    completed.data.state = ActionState::Completed;
    completed.completed_at_ms = Some(completed.updated_at_ms);
    completed.validate().unwrap();
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| {
            let ai = this.ai.as_mut().unwrap();
            ai.open_dashboard();
            let command = ai
                .refresh_dashboard(DashboardFilter::Completed, false)
                .unwrap();
            let mut page = crate::ai::dashboard_state_tests::page(completed.clone());
            page.counts.open = 0;
            page.counts.completed = 1;
            page.entries[0].dependency_blocked = false;
            ai.apply(command.0, AppEvent::ActionDashboard(Box::new(page)));
            assert!(ai.select_dashboard_action(completed.origin.id));
            this.open_doc = Some(DocRef::Dashboard);
            this.sync_dashboard_widgets(window, cx);
            cx.notify();
        });
    });
    visual.run_until_parked();
    scroll(&mut visual, "dashboard-scroll", "new-related-action");
    visual.update(|window, cx| {
        window.click("new-related-action", cx);
        desktop.update(cx, |this, cx| this.sync_draft_widgets(window, cx));
        let this = desktop.read(cx);
        assert_eq!(this.open_doc, Some(DocRef::Draft));
        let form = this.ai.as_ref().unwrap().draft.as_ref().unwrap();
        let action = form.action.as_ref().unwrap();
        assert_ne!(action.id, completed.origin.id);
        assert_eq!(action.fields.state, ActionState::Open);
        assert_eq!(action.fields.values[11], completed.origin.id.to_string());
        assert_eq!(
            this.initial_action.fields[11].read(cx).value().as_ref(),
            completed.origin.id.to_string()
        );
        assert!(!form.can_leave());
        assert!(form.submitted.is_none());
        assert!(!this.ai.as_ref().unwrap().dashboard.visible);
    });
}

fn load_edit_action(
    desktop: &Entity<Desktop>,
    before: &brn_workflow::actions::ActionRecord,
    window: &mut Window,
    cx: &mut App,
) {
    desktop.update(cx, |this, cx| {
        let ai = this.ai.as_mut().unwrap();
        let command = ai.open_dashboard().unwrap();
        ai.apply(
            command.0,
            AppEvent::ActionDashboard(Box::new(crate::ai::dashboard_state_tests::page(
                before.clone(),
            ))),
        );
        assert!(ai.select_dashboard_action(before.origin.id));
        this.open_doc = Some(DocRef::Dashboard);
        this.sync_dashboard_widgets(window, cx);
        cx.notify();
    });
}

#[gpui_kit::test]
fn dashboard_edit_prefills_exact_baseline_and_owner_fields_at_480(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = dashboard_tests::action_window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    let before = crate::ai::dashboard_state_tests::record(200);
    visual.update(|window, cx| load_edit_action(&desktop, &before, window, cx));
    visual.run_until_parked();
    scroll(&mut visual, "dashboard-scroll", "edit-selected-action");
    visual.update(|window, cx| {
        window.click("edit-selected-action", cx);
        desktop.update(cx, |this, cx| this.sync_draft_widgets(window, cx));
        let this = desktop.read(cx);
        assert_eq!(this.open_doc, Some(DocRef::Draft));
        let action = this
            .ai
            .as_ref()
            .unwrap()
            .draft
            .as_ref()
            .unwrap()
            .action
            .as_ref()
            .unwrap();
        assert_eq!(action.id, before.origin.id);
        assert_eq!(action.before.as_deref(), Some(&before));
        assert_eq!(action.fields.data().unwrap(), before.data);
        assert!(action.sources.is_empty());
        let observed: serde_json::Value =
            serde_json::from_str(this.draft_link_proofs.read(cx).value().as_ref()).unwrap();
        assert_eq!(observed["before"], serde_json::to_value(&before).unwrap());
    });
    visual.run_until_parked();
    for index in 0..12 {
        scroll(
            &mut visual,
            "initial-action-form",
            &format!("initial-action-field-{index}"),
        );
    }
    scroll(
        &mut visual,
        "initial-action-form",
        "initial-action-state-Blocked",
    );
    visual.update(|window, cx| {
        type_field(
            &desktop,
            1,
            "\u{feff}Edited õ 日本語\r\nexact λ",
            window,
            cx,
        );
        window.click("initial-action-state-Blocked", cx);
    });
    scroll(
        &mut visual,
        "initial-action-form",
        "copy-initial-action-input",
    );
    visual.update(|window, cx| {
        window.click("copy-initial-action-input", cx);
        let copied: serde_json::Value =
            serde_json::from_str(&cx.read_from_clipboard().unwrap().text().unwrap()).unwrap();
        assert_eq!(
            copied["action"]["before"],
            serde_json::to_value(&before).unwrap()
        );
        assert_eq!(
            copied["action"]["fields"]["values"][1],
            "\u{feff}Edited õ 日本語\r\nexact λ"
        );
    });
    scroll(
        &mut visual,
        "initial-action-form",
        "create-initial-review-draft",
    );
    visual.update(|window, cx| {
        window.click("create-initial-review-draft", cx);
        let this = desktop.read(cx);
        let request = &this.ai.as_ref().unwrap().draft.as_ref().unwrap().submitted.as_ref().unwrap().request;
        assert!(matches!(&request.action_changes[0], brn_workflow::proposals::ActionChange::Replace { before: captured, data } if captured.as_ref() == &before && data.state == ActionState::Blocked && data.description == "\u{feff}Edited õ 日本語\r\nexact λ"));
        assert_eq!(this.ai.as_ref().unwrap().dashboard.attempts.len(), 0);
    });
}

#[gpui_kit::test]
fn painted_edit_callback_rejects_changed_record_and_same_id_reselection(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::{InputEvent as _, MouseButton, MouseDownEvent, MouseUpEvent};
    let (_fixture, handle, desktop) = dashboard_tests::action_window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    let before = crate::ai::dashboard_state_tests::record(200);
    for case in 0..3 {
        visual.update(|window, cx| load_edit_action(&desktop, &before, window, cx));
        visual.run_until_parked();
        scroll(&mut visual, "dashboard-scroll", "edit-selected-action");
        visual.update(|window, cx| {
            window.render_frame(cx);
            let position = window.find("edit-selected-action").bounds().center();
            // Keep the painted callback, then change its authoritative selection
            // without a new render. Dispatch the real pointer events against it.
            desktop.update(cx, |this, _| {
                let ai = this.ai.as_mut().unwrap();
                if case == 0 {
                    let selected = &mut ai.dashboard.selected.as_mut().unwrap().action;
                    selected.version += 1;
                    selected.updated_at_ms += 1;
                    selected.data.description.push_str(" newer");
                } else if case == 1 {
                    assert!(ai.select_dashboard_action(before.origin.id));
                }
            });
            window.dispatch_event(
                MouseDownEvent {
                    button: MouseButton::Left,
                    position,
                    modifiers: Default::default(),
                    click_count: 1,
                    first_mouse: false,
                }
                .to_platform_input(),
                cx,
            );
            window.dispatch_event(
                MouseUpEvent {
                    button: MouseButton::Left,
                    position,
                    modifiers: Default::default(),
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            assert_eq!(
                desktop.read(cx).open_doc,
                Some(if case == 2 {
                    DocRef::Draft
                } else {
                    DocRef::Dashboard
                })
            );
            assert_eq!(
                desktop.read(cx).ai.as_ref().unwrap().draft.is_some(),
                case == 2,
                "fresh events must exercise the same painted callback"
            );
        });
    }
}

#[gpui_kit::test]
fn queued_edit_keeps_original_baseline_through_retained_comment_guard(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = dashboard_tests::action_window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    let before = crate::ai::dashboard_state_tests::record(200);
    visual.update(|window, cx| load_edit_action(&desktop, &before, window, cx));
    visual.run_until_parked();
    scroll(&mut visual, "dashboard-scroll", "edit-selected-action");
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| {
            this.review_comment_draft = Some((Uuid::new_v4(), None));
            this.review_comment.update(cx, |editor, cx| editor.set_value("Retained comment λ", window, cx));
        });
        window.render_frame(cx);
        window.click("edit-selected-action", cx);
        assert!(matches!(desktop.read(cx).simple_transition.as_ref(), Some(simple::EditorTransition::ActionReplace(captured)) if captured.as_ref() == &before));
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::Dashboard));
        desktop.update(cx, |this, cx| {
            let selected = &mut this.ai.as_mut().unwrap().dashboard.selected.as_mut().unwrap().action;
            selected.version += 1;
            selected.updated_at_ms += 1;
            selected.data.description.push_str(" changed while queued");
            this.review_comment.update(cx, |editor, cx| editor.set_value("", window, cx));
            this.simple_progress_transition(cx);
            this.sync_draft_widgets(window, cx);
        });
        let this = desktop.read(cx);
        assert_eq!(this.open_doc, Some(DocRef::Draft));
        assert_eq!(this.ai.as_ref().unwrap().draft.as_ref().unwrap().action.as_ref().unwrap().before.as_deref(), Some(&before));
    });
}

#[gpui_kit::test]
fn captured_discard_refuses_later_action_input_then_explicit_current_discard_works(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = dashboard_tests::action_window(cx, true);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.update(|window, cx| {
        begin(&desktop, window, cx);
        type_field(&desktop, 0, "Captured local Action", window, cx);
        desktop.update(cx, |this, cx| this.discard_draft_dialog(false, window, cx));
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        // Forced later widget input stands in for a queued edit arriving after capture.
        let input = desktop.read(cx).initial_action.fields[1].clone();
        input.update(cx, |input, cx| {
            input.set_value("Later exact λ\r\n", window, cx)
        });
        window.render_frame(cx);
        window.click("confirm-discard-initial-form", cx);
        let this = desktop.read(cx);
        let form = this.ai.as_ref().unwrap().draft.as_ref().unwrap();
        assert_eq!(
            form.action.as_ref().unwrap().fields.values[1],
            "Later exact λ\r\n"
        );
        assert!(this.ai.as_ref().unwrap().notice.contains("changed"));
        assert!(window.has_active_dialog(cx));
        window.close_dialog(cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |this, cx| this.discard_draft_dialog(false, window, cx))
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        window.click("confirm-discard-initial-form", cx);
        assert!(
            desktop.read(cx).ai.as_ref().unwrap().draft.is_none(),
            "current discard did not settle: dialog={}, notice={}",
            window.has_active_dialog(cx),
            desktop.read(cx).ai.as_ref().unwrap().notice
        );
        assert!(!window.has_active_dialog(cx));
    });
}
