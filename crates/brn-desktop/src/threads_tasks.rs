//! Inbox → Tasks: the follow-up list, task rows and the checked detail
//! editor. Task state is independent of chat state.
use super::components::AccessibleDropdownMenu;
use super::shell::{item, pill};
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Input;

use super::tiles::page;
use super::tiles::{PAGE_WIDTH, column};

fn group(id: impl Into<gpui_kit::ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
    div().id(id).flex().flex_col().gap(px(2.)).min_w_0()
}

impl ThreadsUi {
    pub(super) fn action_details_dialog(
        &mut self,
        record: Record,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let RecordData::Action(action) = &record.data else {
            return;
        };
        let fields = [
            (
                "Owner",
                "Person or team; blank if unknown",
                action.owner.clone(),
            ),
            (
                "Waiting on",
                "Person, team or needed response",
                action.waiting_on.clone(),
            ),
            (
                "Follow up",
                "YYYY-MM-DD; separate from due date",
                action.follow_up.clone(),
            ),
        ]
        .map(|(label, placeholder, value)| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
            input.update(cx, |input, cx| {
                input.set_value(value.unwrap_or_default(), window, cx)
            });
            (label, input)
        });
        let error = cx.new(|_| String::new());
        let target = cx.entity().downgrade();
        let key = format!("action-details:{}", uuid::Uuid::new_v4());
        let save: components::DialogAction = {
            let fields = fields.clone();
            let error = error.clone();
            std::rc::Rc::new(move |window, cx| {
                let RecordData::Action(mut action) = record.data.clone() else {
                    return;
                };
                let values = fields.each_ref().map(|(_, field)| {
                    let text = field.read(cx).value().to_string();
                    if text.trim().is_empty() {
                        None
                    } else {
                        Some(text)
                    }
                });
                [action.owner, action.waiting_on, action.follow_up] = values;
                let request = ChangeRequest {
                    reason: "Owner edited Action follow-up details".into(),
                    writes: vec![Put {
                        id: record.id.clone(),
                        expected_version: Some(record.version),
                        archived: record.archived,
                        data: RecordData::Action(action),
                    }],
                    inputs: vec![],
                };
                let mut result = Err("This window is unavailable.".to_owned());
                let _ = target.update(cx, |this, cx| {
                    result = match this.workspace.owner_change(&key, &request) {
                        Ok(outcome) => {
                            this.report_outcome(&outcome);
                            if matches!(outcome, ApplyOutcome::Applied(_)) { Ok(()) }
                            else { Err("This task changed or needs review. Your entries are kept here; close and reopen its current details before retrying.".into()) }
                        }
                        Err(error) => Err(error.to_string()),
                    };
                    this.refresh();
                    cx.notify();
                });
                match result {
                    Ok(()) => window.close_dialog(cx),
                    Err(message) => error.update(cx, |error, cx| {
                        *error = message;
                        cx.notify();
                    }),
                }
            })
        };
        let first = fields[0].1.clone();
        self.open_dialog(window, cx, move |dialog, window, cx| {
            let mut form = components::form_keys(div().flex().flex_col().gap_3(), save.clone());
            for (index, (label, input)) in fields.iter().enumerate() {
                form = form.child(div().flex().flex_col().gap_1()
                    .child(*label)
                    .child(Input::new(input).id(format!("action-detail-{index}")).aria_label(*label)));
            }
            dialog.title("Task follow-up details").width(px(520.)).on_ok(|_, _, _| false)
                .child(components::dialog_body("action-details-body", window)
                    .child(form)
                    .child(metadata("Blank means unspecified. Saving these details does not complete the task or schedule a reminder."))
                    .child(components::field_error("action-details-error", error.read(cx).clone()))
                    .test_support())
                .footer(components::form_keys(components::dialog_footer(), save.clone())
                    .child(button("action-details-cancel").label("Cancel").outline()
                        .on_click(|_, window, cx| window.close_dialog(cx)))
                    .child({ let save = save.clone();
                        button("action-details-save").label("Save").primary()
                            .on_click(move |_, window, cx| save(window, cx))
                    }))
        });
        first.update(cx, |input, cx| input.focus(window, cx));
    }

    /// A task row: its description and facts, the one main action for its
    /// state (outline) and a row ⋯ with the other state changes. `show_state`
    /// names the state where no section does.
    pub(super) fn action_row(
        &self,
        record: &Record,
        show_state: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let RecordData::Action(action) = &record.data else {
            return div().into_any_element();
        };
        let p = pal();
        let id = record.id.clone();
        // The task as it would be after a step; Confirm done records how.
        let next = move |step: lists::TaskStep| {
            let mut next = action.clone();
            next.state = step.state();
            if step == lists::TaskStep::ConfirmDone {
                next.evidence = Some("Owner confirmed completion in BRN.".into());
            }
            RecordData::Action(next)
        };
        let facts: Vec<String> = [
            show_state.then(|| format!("{:?}", action.state)),
            action
                .due
                .as_ref()
                .map(|due| format!("Due {}", lists::human_time(due))),
            action.owner.as_ref().map(|owner| format!("Owner: {owner}")),
            action
                .waiting_on
                .as_ref()
                .map(|waiting| format!("Waiting on: {waiting}")),
            action
                .follow_up
                .as_ref()
                .map(|date| format!("Follow up: {date}")),
            action.evidence.clone(),
        ]
        .into_iter()
        .flatten()
        .collect();
        let (main, steps) = lists::task_steps(&action.state);
        let mut controls = div().flex().flex_shrink_0().items_center().gap_1();
        if let Some(step) = main {
            let (record, data) = (record.clone(), next(step));
            let control = match step {
                lists::TaskStep::Accept => format!("open-action-{id}"),
                _ => format!("done-action-{id}"),
            };
            // The row's one main action is quiet: rows never hold a filled
            // primary.
            controls = controls.child(
                button(control)
                    .label(step.label())
                    .accessibility_label(format!("{}: {}", step.label(), action.description))
                    .outline()
                    .compact()
                    .h(components::tokens::COMPACT_CONTROL)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.change(step.reason(), record.clone(), data.clone(), cx)
                    })),
            );
        }
        {
            let this = cx.entity().downgrade();
            let details = record.clone();
            let items: Vec<_> = steps
                .into_iter()
                .map(|step| (step, record.clone(), next(step)))
                .collect();
            controls = controls.child(
                components::overflow_menu(
                    format!("task-menu-{id}"),
                    format!("Task actions: {}", action.description),
                )
                .accessible_dropdown_menu(move |mut menu, _, _| {
                    let details = details.clone();
                    menu = menu.item(item(
                        &this,
                        "Edit details",
                        None,
                        false,
                        false,
                        move |view, window, cx| {
                            view.action_details_dialog(details.clone(), window, cx);
                        },
                    ));
                    for (step, record, data) in items.clone() {
                        if step == lists::TaskStep::Cancel {
                            menu = menu.separator();
                        }
                        menu = menu.item(item(
                            &this,
                            step.label(),
                            None,
                            false,
                            false,
                            move |view, _, cx| {
                                view.change(step.reason(), record.clone(), data.clone(), cx)
                            },
                        ));
                    }
                    menu
                }),
            );
        }
        div()
            .id(format!("action-tile-{id}"))
            .flex()
            .flex_shrink_0()
            .items_start()
            .gap_2()
            .px_2()
            .py_2()
            .min_w_0()
            .border_b_1()
            .border_color(rgb(p.hairline))
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .id(format!("action-description-{id}"))
                            .role(gpui_kit::Role::Label)
                            .aria_label(action.description.clone())
                            .aria_value(action.description.clone())
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .text_color(rgb(p.ink))
                            .child(action.description.clone())
                            .test_support(),
                    )
                    .when(!facts.is_empty(), |text| {
                        let facts = facts.join(" · ");
                        text.child(
                            metadata(facts.clone())
                                .id(format!("action-facts-{id}"))
                                .role(gpui_kit::Role::Label)
                                .aria_label(facts.clone())
                                .aria_value(facts)
                                .test_support(),
                        )
                    }),
            )
            .child(controls)
            .test_support()
            .into_any_element()
    }
    /// A section's session-only disclosure: "Show N" or "Hide".
    pub(super) fn disclosure(
        &self,
        key: &'static str,
        count: usize,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let open = self.open_sections.contains(key);
        pill(
            format!("{key}-toggle"),
            if open {
                "Hide".to_owned()
            } else {
                format!("Show {count}")
            },
        )
        .accessibility_label(if open {
            "Hide".to_owned()
        } else {
            format!("Show {count}")
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            if !this.open_sections.remove(key) {
                this.open_sections.insert(key);
            }
            cx.notify();
        }))
        .into_any_element()
    }
    pub(super) fn actions_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut view = column();
        if !self
            .records
            .iter()
            .any(|r| !r.archived && matches!(r.data, RecordData::Action(_)))
        {
            view = view.child(components::empty_state(
                "actions-empty",
                IconName::ListTodo,
                "No tasks yet",
                "Tasks keep their own progress, including after a chat is archived.",
                None,
            ));
        }
        let today = lists::follow_up_today();
        view = view.child(metadata(format!(
            "Follow-ups as of {today}. Dates use your computer's local day (UTC if unavailable); due dates stay separate."
        )));
        for bucket in lists::FollowUpGroup::ALL {
            let (key, label, collapsible) = bucket.presentation();
            let records = lists::follow_up_records(&self.records, bucket, today);
            if records.is_empty() {
                continue;
            }
            // Done and Cancelled start collapsed, with their counts.
            let shown = collapsible.is_none_or(|key| self.open_sections.contains(key));
            let mut group = group(format!("actions-group-{key}"))
                .role(gpui_kit::Role::Group)
                .aria_label(format!("{label} · {}", records.len()))
                .child(components::section_header(
                    label,
                    Some(records.len()),
                    collapsible.map(|toggle| self.disclosure(toggle, records.len(), cx)),
                ));
            if shown {
                for record in records {
                    group = group.child(self.action_row(
                        record,
                        matches!(
                            bucket,
                            lists::FollowUpGroup::Overdue
                                | lists::FollowUpGroup::Today
                                | lists::FollowUpGroup::Upcoming
                                | lists::FollowUpGroup::CheckDate
                        ),
                        cx,
                    ));
                }
            }
            view = view.child(group.test_support());
        }
        page("actions-content", PAGE_WIDTH, view)
            .test_support()
            .into_any_element()
    }
}

/// The task whose detail panel is open, by record ID.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TaskDetail {
    pub(super) id: String,
}

impl ThreadsUi {
    /// Open Tasks with one task's detail panel.
    pub(super) fn open_task(&mut self, id: &str, _window: &mut Window, cx: &mut Context<Self>) {
        self.inbox_page.task = Some(TaskDetail { id: id.to_owned() });
        cx.notify();
    }
}
