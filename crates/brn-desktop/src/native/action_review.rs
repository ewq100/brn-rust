//! Field-based Action review and complete captured records; no domain resolution.
use super::*;
use crate::review::action_fields::{ActionFields, LABELS};
use brn_workflow::{
    actions::{ActionData, ActionPriority, ActionRecord, ActionState},
    proposals::ActionChange,
};
use gpui_kit::{
    Div, TestSupportExt,
    base::Disableable,
    component::{Selectable, WindowExt, input::Textarea},
};

#[derive(Default)]
pub(super) struct ActionEditors {
    proposal: Option<Uuid>,
    pub fields: Vec<Vec<Entity<TextareaState>>>,
    subscriptions: Vec<Subscription>,
}

pub(super) fn data_body(scope: &str, label: &str, data: &ActionData) -> Div {
    let fields = ActionFields::from(data);
    let mut body = div().flex().flex_col().gap_1().child(label.to_owned());
    for (field, (label, value)) in LABELS.iter().zip(fields.values).enumerate() {
        body = body.child(
            div()
                .id(format!("{scope}-field-{field}"))
                .test_support()
                .aria_label(format!("{label}: {value}"))
                .flex()
                .flex_col()
                .child((*label).to_owned())
                .child(value),
        );
    }
    body.child(
        div()
            .id(format!("{scope}-state"))
            .test_support()
            .aria_label(format!("State: {:?}", data.state))
            .child(format!("State: {:?}", data.state)),
    )
    .child(
        div()
            .id(format!("{scope}-priority"))
            .test_support()
            .aria_label(format!("Priority: {:?}", data.priority))
            .child(format!("Priority: {:?}", data.priority)),
    )
}

pub(super) fn before_body(index: usize, record: &ActionRecord) -> Div {
    let metadata = [
        format!(
            "Full captured before Action {} · version {} · updated at {} ms",
            record.origin.id, record.version, record.updated_at_ms
        ),
        format!(
            "Waiting since: {:?} ms · completed at: {:?} ms",
            record.waiting_since_ms, record.completed_at_ms
        ),
        format!(
            "Immutable origin: {} · creating proposal {} version {} · created at {} ms",
            record.origin.id,
            record.origin.proposal.id,
            record.origin.proposal.version,
            record.origin.created_at_ms
        ),
    ];
    let mut body = div().flex().flex_col().gap_1();
    for (field, value) in metadata.into_iter().enumerate() {
        body = body.child(
            div()
                .id(format!("action-{index}-metadata-{field}"))
                .test_support()
                .aria_label(value.clone())
                .child(value),
        );
    }
    body.child(data_body(
        &format!("action-{index}-origin"),
        "Full origin data",
        &record.origin.data,
    ))
    .child(data_body(
        &format!("action-{index}-before"),
        "Full before data",
        &record.data,
    ))
}

pub(super) fn member_body(index: usize, change: &ActionChange) -> Div {
    let kind = match change {
        ActionChange::Create { .. } => "Create",
        ActionChange::Replace { .. } => "Replace",
    };
    let mut body = div().flex().flex_col().gap_2().child(
        div()
            .id(format!("captured-action-{index}"))
            .test_support()
            .aria_label(format!("{kind} UUID {}", change.id()))
            .child(format!(
                "Action member {} · {kind} · UUID {}",
                index + 1,
                change.id()
            )),
    );
    body = match change {
        ActionChange::Create { .. } => body.child("Before: no Action with this UUID"),
        ActionChange::Replace { before, .. } => body.child(before_body(index, before)),
    };
    body.child(data_body(
        &format!("action-{index}-proposed"),
        "Full proposed Action data",
        change.data(),
    ))
}

impl Desktop {
    pub(super) fn action_review_editable(&self) -> bool {
        self.ai.as_ref().unwrap().review_editable()
            && self.simple_transition.is_none()
            && self.closing.is_none()
            && !self.closed
            && !self.close_failed
    }

    pub(super) fn sync_action_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = &self.ai.as_ref().unwrap().review else {
            return;
        };
        let id = review.record.draft.id;
        let forms = review.action_fields().to_vec();
        if self.action_editors.proposal != Some(id)
            || self.action_editors.fields.len() != forms.len()
        {
            let mut pane = ActionEditors {
                proposal: Some(id),
                ..Default::default()
            };
            for (index, form) in forms.iter().enumerate() {
                let mut widgets = Vec::new();
                for (field, value) in form.values.iter().enumerate() {
                    let editor = cx.new(|cx| {
                        TextareaState::new(window, cx)
                            .auto_grow(1, 8)
                            .default_value(value.clone())
                    });
                    pane.subscriptions.push(cx.subscribe_in(
                        &editor,
                        window,
                        move |this, editor, event: &InputEvent, window, cx| {
                            if !matches!(event, InputEvent::Change) {
                                return;
                            }
                            let permitted =
                                this.action_review_editable() && !window.has_active_dialog(cx);
                            let mut changed = false;
                            if permitted
                                && let Some(review) = this
                                    .ai
                                    .as_mut()
                                    .unwrap()
                                    .review
                                    .as_mut()
                                    .filter(|review| review.record.draft.id == id)
                                && let Some(form) = review.action_fields().get(index)
                            {
                                let mut form = form.clone();
                                form.values[field] = editor.read(cx).value().to_string();
                                match review.edit_action_fields(index, form, Instant::now()) {
                                    Ok(()) => changed = true,
                                    Err(error) => this.ai.as_mut().unwrap().notice = error.into(),
                                }
                            }
                            if !changed {
                                this.sync_action_widgets(window, cx);
                            }
                            cx.notify();
                        },
                    ));
                    widgets.push(editor);
                }
                pane.fields.push(widgets);
            }
            self.action_editors = pane;
        } else {
            for (widgets, form) in self.action_editors.fields.iter().zip(forms) {
                for (widget, value) in widgets.iter().zip(form.values) {
                    if widget.read(cx).value().as_ref() != value {
                        widget.update(cx, |input, cx| input.set_value(value, window, cx));
                    }
                }
            }
        }
    }

    pub(super) fn action_editor_body(
        &self,
        index: usize,
        change: &ActionChange,
        editable: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut local = change.clone();
        *local.data_mut() = self
            .ai
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .unwrap()
            .action_data()[index]
            .clone();
        let mut body = member_body(index, &local);
        let Some(widgets) = self.action_editors.fields.get(index) else {
            return body.child("Loading Action fields…");
        };
        let form = &self
            .ai
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .unwrap()
            .action_fields()[index];
        let proposal_id = self
            .ai
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .unwrap()
            .record
            .draft
            .id;
        let action_id = change.id();
        body = body.child("Edit proposed Action fields · empty optional fields remain unset");
        for (field, widget) in widgets.iter().enumerate() {
            body = body.child(
                div()
                    .id(format!("action-field-{index}-{field}"))
                    .test_support()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(LABELS[field])
                    .child(
                        Textarea::new(widget)
                            .disabled(!editable)
                            .aria_label(format!("Action {} {}", index + 1, LABELS[field])),
                    ),
            );
        }
        for state in [
            ActionState::Open,
            ActionState::Waiting,
            ActionState::Blocked,
        ] {
            body = body.child(
                Button::new(format!("action-state-{index}-{state:?}"))
                    .label(format!("{state:?}"))
                    .selected(form.state == state)
                    .disabled(!editable)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if !this.action_review_editable() || window.has_active_dialog(cx) {
                            return;
                        }
                        if let Some(review) =
                            this.ai.as_mut().unwrap().review.as_mut().filter(|review| {
                                review.record.draft.id == proposal_id
                                    && review
                                        .record
                                        .draft
                                        .action_changes
                                        .get(index)
                                        .is_some_and(|change| change.id() == action_id)
                            })
                            && let Some(form) = review.action_fields().get(index)
                        {
                            let mut form = form.clone();
                            form.state = state;
                            if let Err(error) =
                                review.edit_action_fields(index, form, Instant::now())
                            {
                                this.ai.as_mut().unwrap().notice = error.into();
                            }
                        }
                        cx.notify();
                    })),
            );
        }
        body = body.child("Completion requires an identified Complete command.");
        for priority in [
            None,
            Some(ActionPriority::Low),
            Some(ActionPriority::Normal),
            Some(ActionPriority::High),
        ] {
            let label = priority.map_or_else(|| "Unset".into(), |priority| format!("{priority:?}"));
            body = body.child(
                Button::new(format!("action-priority-{index}-{label}"))
                    .label(format!("Priority: {label}"))
                    .selected(form.priority == priority)
                    .disabled(!editable)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if !this.action_review_editable() || window.has_active_dialog(cx) {
                            return;
                        }
                        if let Some(review) =
                            this.ai.as_mut().unwrap().review.as_mut().filter(|review| {
                                review.record.draft.id == proposal_id
                                    && review
                                        .record
                                        .draft
                                        .action_changes
                                        .get(index)
                                        .is_some_and(|change| change.id() == action_id)
                            })
                            && let Some(form) = review.action_fields().get(index)
                        {
                            let mut form = form.clone();
                            form.priority = priority;
                            if let Err(error) =
                                review.edit_action_fields(index, form, Instant::now())
                            {
                                this.ai.as_mut().unwrap().notice = error.into();
                            }
                        }
                        cx.notify();
                    })),
            );
        }
        body
    }
}
