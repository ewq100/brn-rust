//! Inbox → Tasks: the follow-up list, task rows and the checked detail
//! editor. Task state is independent of chat state.
use super::components::AccessibleDropdownMenu;
use super::shell::{item, pill};
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Input;

use super::inbox_page::{lead, page_frame};
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
            ("Due", "When it is due, as agreed", action.due.clone()),
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
                if let Err(message) = check_follow_up(values[2].as_deref()) {
                    error.update(cx, |error, cx| {
                        *error = message;
                        cx.notify();
                    });
                    return;
                }
                [
                    action.owner,
                    action.waiting_on,
                    action.follow_up,
                    action.due,
                ] = values;
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
                    .child(metadata("Blank means unspecified. Follow up and Due are separate dates. Saving these details does not complete the task or schedule a reminder."))
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
        self.task_row(record, show_state, false, cx)
    }
    /// A task row; in Tasks (`opens`) its text opens the detail panel.
    fn task_row(
        &self,
        record: &Record,
        show_state: bool,
        opens: bool,
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
            show_state.then(|| state_word(&action.state).to_owned()),
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
        let selected = opens
            && self
                .inbox_page
                .task
                .as_ref()
                .is_some_and(|task| task.id == record.id);
        let text = div()
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
            });
        let text = if opens {
            let open = record.id.clone();
            button(format!("task-open-{id}"))
                .ghost()
                .flex_1()
                .min_w_0()
                .h_auto()
                .px_1()
                .py_1()
                .rounded(px(theme::metrics::RADIUS_CONTROL))
                .accessibility_label(format!("Open task: {}", action.description))
                .child(text)
                .on_click(cx.listener(move |this, _, window, cx| this.open_task(&open, window, cx)))
                .into_any_element()
        } else {
            text.into_any_element()
        };
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
            .rounded(px(theme::metrics::RADIUS_CONTROL))
            .when(selected, |row| row.bg(rgb(p.selection)))
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .child(text)
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
        let compact = self.inbox_compact();
        let mut view = column().child(lead(
            "tasks-lead",
            "Tasks you and BRN are tracking. Each keeps its own state, even after its chat ends.",
        ));
        if !self
            .records
            .iter()
            .any(|r| !r.archived && matches!(r.data, RecordData::Action(_)))
        {
            view = view.child(components::empty_state(
                "actions-empty",
                IconName::ListTodo,
                "No tasks yet",
                "Ask BRN in a chat to create a task, or choose Capture outcomes after a chat.",
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
                    group = group.child(self.task_row(
                        record,
                        matches!(
                            bucket,
                            lists::FollowUpGroup::Overdue
                                | lists::FollowUpGroup::Today
                                | lists::FollowUpGroup::Upcoming
                                | lists::FollowUpGroup::CheckDate
                        ),
                        true,
                        cx,
                    ));
                }
            }
            view = view.child(group.test_support());
        }
        let Some(task) = self.inbox_page.task.as_ref() else {
            return page_frame("actions-content", PAGE_WIDTH, compact, view)
                .test_support()
                .into_any_element();
        };
        let detail = self.task_detail(&task.id, cx);
        // Wide: list and detail side by side. Narrow: the detail alone, with
        // a way back to every task.
        if self.arrangement.primary >= DETAIL_BESIDE {
            div()
                .id("actions-content")
                .flex()
                .size_full()
                .min_h_0()
                .child(
                    div().flex_1().min_w_0().min_h_0().child(
                        page_frame("actions-list", PAGE_WIDTH, compact, view).test_support(),
                    ),
                )
                .child(
                    div()
                        .w(px(DETAIL_WIDTH))
                        .flex_shrink_0()
                        .min_h_0()
                        .border_l_1()
                        .border_color(rgb(pal().hairline))
                        .child(page("task-detail-scroll", DETAIL_WIDTH, detail)),
                )
                .test_support()
                .into_any_element()
        } else {
            page_frame("actions-content", PAGE_WIDTH, compact, detail)
                .test_support()
                .into_any_element()
        }
    }

    /// The task detail panel: every fact the task records, its state
    /// changes with explicit completion, and the checked details editor.
    fn task_detail(&self, id: &str, cx: &mut Context<Self>) -> gpui_kit::Div {
        let p = pal();
        let close = button("task-detail-close")
            .label("All tasks")
            .icon(IconName::ChevronLeft)
            .ghost()
            .compact()
            .h(components::tokens::COMPACT_CONTROL)
            .on_click(cx.listener(|this, _, _, cx| {
                this.inbox_page.task = None;
                cx.notify();
            }));
        let mut panel = column()
            .id("task-detail")
            .gap_3()
            .pt(components::tokens::SPACE_2)
            .child(div().flex().child(close));
        let Some(record) = self
            .records
            .iter()
            .find(|record| record.id == id && !record.archived)
            .cloned()
        else {
            return div().child(
                panel
                    .child(components::empty_state(
                        "task-detail-missing",
                        IconName::ListTodo,
                        "This task is no longer available",
                        "It was archived or removed. Recent changes shows what happened.",
                        None,
                    ))
                    .test_support(),
            );
        };
        let RecordData::Action(action) = &record.data else {
            return div().child(panel.test_support());
        };
        let (tone, icon) = state_tone(&action.state);
        panel = panel.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .id("task-detail-title")
                        .text_size(px(theme::metrics::TEXT_SECTION))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .text_color(rgb(p.ink))
                        .aria_label(action.description.clone())
                        .child(action.description.clone())
                        .test_support(),
                )
                .child(div().flex().child(components::status_chip(
                    "task-detail-state",
                    Some(tone),
                    Some(icon),
                    state_word(&action.state),
                ))),
        );
        let unset = |value: &Option<String>, empty: &str| {
            value
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map_or_else(|| empty.to_owned(), str::to_owned)
        };
        let mut facts = div()
            .id("task-detail-facts")
            .flex()
            .flex_col()
            .role(gpui_kit::Role::Group)
            .aria_label("Task details");
        for (key, label, value) in [
            ("owner", "Owner", unset(&action.owner, "Not reported")),
            ("waiting", "Waiting on", unset(&action.waiting_on, "Nobody")),
            (
                "follow-up",
                "Follow up",
                unset(&action.follow_up, "No date"),
            ),
            ("due", "Due", unset(&action.due, "No due date")),
        ]
        .into_iter()
        .chain(
            action
                .evidence
                .clone()
                .map(|evidence| ("evidence", "Completed", evidence)),
        ) {
            facts = facts.child(detail_fact(key, label, value));
        }
        panel = panel.child(facts.test_support());
        let (main, steps) = lists::task_steps(&action.state);
        let next = |step: lists::TaskStep| {
            let mut next = action.clone();
            next.state = step.state();
            if step == lists::TaskStep::ConfirmDone {
                next.evidence = Some("Owner confirmed completion in BRN.".into());
            }
            RecordData::Action(next)
        };
        let mut buttons = div().flex().flex_wrap().gap_2();
        if let Some(step) = main {
            let (record, data) = (record.clone(), next(step));
            buttons = buttons.child(
                button("task-detail-main")
                    .label(step.label())
                    .primary()
                    .accessibility_label(format!("{}: {}", step.label(), action.description))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.change(step.reason(), record.clone(), data.clone(), cx)
                    })),
            );
        }
        {
            let details = record.clone();
            buttons = buttons.child(
                button("task-detail-edit")
                    .label("Edit details…")
                    .outline()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.action_details_dialog(details.clone(), window, cx)
                    })),
            );
        }
        for step in steps {
            let (record, data) = (record.clone(), next(step));
            buttons = buttons.child(
                button(format!("task-detail-{}", step_key(step)))
                    .label(step.label())
                    .ghost()
                    .when(step == lists::TaskStep::Cancel, |b| b.danger())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.change(step.reason(), record.clone(), data.clone(), cx)
                    })),
            );
        }
        panel = panel.child(buttons);
        if let Some(chat) = action.chat.clone() {
            panel = panel.child(
                button("task-detail-chat")
                    .label("Open its chat")
                    .icon(IconName::MessageSquare)
                    .ghost()
                    .compact()
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open_thread(&chat, window, cx)),
                    ),
            );
        }
        let panel = panel.child(metadata(
            "A task keeps its own state. Archiving or continuing a chat does not change it.",
        ));
        div().child(panel.test_support())
    }
}

/// Wide enough to show the detail panel beside the list.
const DETAIL_BESIDE: f32 = 960.;
const DETAIL_WIDTH: f32 = 340.;

/// A task state in owner words.
pub(super) fn state_word(state: &ActionState) -> &'static str {
    match state {
        ActionState::Suggested => "Suggested",
        ActionState::Open => "Open",
        ActionState::Waiting => "Waiting",
        ActionState::Done => "Done",
        ActionState::Cancelled => "Cancelled",
    }
}

fn state_tone(state: &ActionState) -> (Tone, IconName) {
    match state {
        ActionState::Suggested => (Tone::Info, IconName::Sparkles),
        ActionState::Open => (Tone::Info, IconName::Circle),
        ActionState::Waiting => (Tone::Warning, IconName::Clock),
        ActionState::Done => (Tone::Success, IconName::CircleCheck),
        ActionState::Cancelled => (Tone::Info, IconName::CircleX),
    }
}

fn step_key(step: lists::TaskStep) -> &'static str {
    match step {
        lists::TaskStep::Accept => "accept",
        lists::TaskStep::ConfirmDone => "done",
        lists::TaskStep::Waiting => "waiting",
        lists::TaskStep::MarkOpen => "open",
        lists::TaskStep::Cancel => "cancel",
    }
}

/// A label above its value, one fact per line.
fn detail_fact(key: &str, label: &str, value: String) -> gpui_kit::AnyElement {
    let p = pal();
    div()
        .id(gpui_kit::SharedString::from(format!("task-detail-{key}")))
        .flex()
        .flex_col()
        .gap(px(2.))
        .py_2()
        .border_b_1()
        .border_color(rgb(p.hairline))
        .aria_label(format!("{label}: {value}"))
        .child(
            div()
                .text_size(px(theme::metrics::TEXT_LABEL))
                .text_color(rgb(p.muted))
                .child(label.to_owned()),
        )
        .child(
            div()
                .text_size(px(theme::metrics::TEXT_BODY))
                .text_color(rgb(p.ink))
                .child(value),
        )
        .test_support()
        .into_any_element()
}

/// Follow up is a calendar date or blank; anything else keeps the form
/// open with the owner's entry.
pub(super) fn check_follow_up(value: Option<&str>) -> Result<(), String> {
    let Some(value) = value.map(str::trim) else {
        return Ok(());
    };
    match chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        Ok(date) if date.format("%Y-%m-%d").to_string() == value => Ok(()),
        _ => Err(format!(
            "Follow up needs a calendar date written as YYYY-MM-DD, for example 2026-11-20. \"{value}\" isn't one. Nothing was saved."
        )),
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
