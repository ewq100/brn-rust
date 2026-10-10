//! Initial Action presentation over the retained ordinary proposal form.
use super::*;
use crate::review::action_fields::LABELS;
use brn_workflow::actions::{ActionPriority, ActionState};
use gpui_kit::{
    AnyElement, TestSupportExt,
    base::Disableable,
    component::{Selectable, WindowExt, input::Textarea},
};

#[derive(Default)]
pub(super) struct ActionInputs {
    form: Option<Uuid>,
    pub fields: Vec<Entity<TextareaState>>,
    subscriptions: Vec<Subscription>,
}

fn raw_input(form: &crate::draft::DraftForm) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "proposal_id": form.id, "action": form.action,
        "source_path_input": form.path,
        "submitted_request": form.submitted.as_ref().map(|submitted| &submitted.request),
    }))
    .expect("retained Action input JSON")
}

impl Desktop {
    fn initial_action_editable(&self) -> bool {
        self.simple_transition.is_none()
            && self.closing.is_none()
            && !self.closed
            && !self.close_failed
            && self
                .ai
                .as_ref()
                .unwrap()
                .draft
                .as_ref()
                .is_some_and(|form| {
                    form.action.is_some()
                        && (!self.ai.as_ref().unwrap().application_busy() || form.pending)
                })
    }
    pub(super) fn sync_initial_action_widgets(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let form = self.ai.as_ref().unwrap().draft.as_ref().unwrap();
        let action = form.action.as_ref().unwrap();
        let id = form.id;
        let sources = action
            .sources
            .iter()
            .map(|source| &source.source)
            .collect::<Vec<_>>();
        let proofs = serde_json::to_string_pretty(&serde_json::json!({
            "before": action.before,
            "sources": sources,
        }))
        .expect("captured source bindings JSON");
        if self.draft_link_proofs.read(cx).value().as_ref() != proofs {
            self.draft_link_proofs
                .update(cx, |editor, cx| editor.set_value(proofs, window, cx));
        }
        if self.initial_action.form == Some(id) && self.draft_widget_id == Some(id) {
            return;
        }
        let fields = action.fields.clone();
        let path = form.path.clone();
        let action_id = action.id;
        let mut pane = ActionInputs {
            form: Some(id),
            ..Default::default()
        };
        for (index, value) in fields.values.into_iter().enumerate() {
            let input = cx.new(|cx| {
                TextareaState::new(window, cx)
                    .auto_grow(1, 8)
                    .default_value(value)
            });
            pane.subscriptions.push(cx.on_focus(
                &input.read(cx).focus_handle(cx),
                window,
                move |this, _, cx| {
                    if this.initial_action.form == Some(id) {
                        this.draft_scroll.scroll_to_top_of_item(index + 3);
                        cx.notify();
                    }
                },
            ));
            pane.subscriptions.push(cx.subscribe_in(
                &input,
                window,
                move |this, input, event: &InputEvent, window, cx| {
                    if !matches!(event, InputEvent::Change)
                        || !this.initial_action_editable()
                        || window.has_active_dialog(cx)
                    {
                        return;
                    }
                    if let Some(form) = this
                        .ai
                        .as_mut()
                        .unwrap()
                        .draft
                        .as_mut()
                        .filter(|form| form.id == id)
                        && let Some(action) =
                            form.action.as_ref().filter(|action| action.id == action_id)
                    {
                        let mut fields = action.fields.clone();
                        fields.values[index] = input.read(cx).value().to_string();
                        form.edit_action_fields(fields);
                        cx.notify();
                    }
                },
            ));
            pane.fields.push(input);
        }
        pane.subscriptions.push(cx.on_focus(
            &self.draft_path.read(cx).focus_handle(cx),
            window,
            move |this, _, cx| {
                if this.initial_action.form == Some(id) {
                    // Three headings, twelve fields, state/priority and two
                    // source explanations precede this separate textarea.
                    this.draft_scroll.scroll_to_top_of_item(19);
                    cx.notify();
                }
            },
        ));
        self.initial_action = pane;
        self.draft_path
            .update(cx, |input, cx| input.set_value(path, window, cx));
        self.draft_widget_id = Some(id);
        self.draft_scroll.set_offset(point(px(0.), px(0.)));
    }
    pub(super) fn capture_initial_action_widgets(&mut self, cx: &App) {
        if let Some(form) = self.ai.as_mut().unwrap().draft.as_mut()
            && self.initial_action.form == Some(form.id)
            && self.draft_widget_id == Some(form.id)
            && let Some(action) = &form.action
        {
            let mut fields = action.fields.clone();
            for (field, input) in fields.values.iter_mut().zip(&self.initial_action.fields) {
                *field = input.read(cx).value().to_string();
            }
            form.edit_action_fields(fields);
            form.edit_action_source_path(self.draft_path.read(cx).value().to_string());
        }
    }
    pub(super) fn render_initial_action(&self, cx: &mut Context<Self>) -> AnyElement {
        let ai = self.ai.as_ref().unwrap();
        let form = ai.draft.as_ref().unwrap();
        let action = form.action.as_ref().unwrap();
        let editable = self.initial_action_editable();
        let blocked = self.draft_command_blocked();
        let mut body = div().id("initial-action-form").test_support()
            .track_scroll(&self.draft_scroll).overflow_y_scroll().vertical_scrollbar(&self.draft_scroll)
            .flex().flex_col().flex_1().min_h(px(0.)).gap_2().p_3()
            .child(if action.before.is_some() { "Edit existing Action proposal" } else { "New Action proposal" })
            .child(if action.before.is_some() { "Create retains an exact replacement review. The existing Action changes only after approval. The captured baseline is retained; competing changes require a fresh explicit proposal." } else { "Create retains review work. The Action exists only after exact proposal approval. Completed work stays completed; a follow-up is a new related Action." })
            .child(format!("Proposal {} · Action {} · input generation {}", form.id, action.id, form.generation));
        if self.initial_action.form != Some(form.id) || self.draft_widget_id != Some(form.id) {
            return body
                .child("Loading retained Action fields…")
                .into_any_element();
        }
        for (index, (label, input)) in LABELS.iter().zip(&self.initial_action.fields).enumerate() {
            body = body.child(
                div()
                    .id(format!("initial-action-field-{index}"))
                    .test_support()
                    .flex()
                    .flex_col()
                    .child((*label).to_owned())
                    .child(Textarea::new(input).disabled(!editable).aria_label(*label)),
            );
        }
        let mut states = div().flex().flex_wrap().gap_1().child("State");
        for state in [
            ActionState::Open,
            ActionState::Waiting,
            ActionState::Blocked,
        ] {
            states = states.child(
                Button::new(format!("initial-action-state-{state:?}"))
                    .label(format!("{state:?}"))
                    .selected(action.fields.state == state)
                    .disabled(!editable)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if !this.initial_action_editable() || window.has_active_dialog(cx) {
                            return;
                        }
                        this.capture_initial_action_widgets(cx);
                        let form = this.ai.as_mut().unwrap().draft.as_mut().unwrap();
                        let mut fields = form.action.as_ref().unwrap().fields.clone();
                        fields.state = state;
                        form.edit_action_fields(fields);
                        cx.notify();
                    })),
            );
        }
        let mut priorities = div().flex().flex_wrap().gap_1().child("Priority");
        for (label, priority) in [
            ("Unset", None),
            ("Low", Some(ActionPriority::Low)),
            ("Normal", Some(ActionPriority::Normal)),
            ("High", Some(ActionPriority::High)),
        ] {
            priorities = priorities.child(
                Button::new(format!("initial-action-priority-{label}"))
                    .label(label)
                    .selected(action.fields.priority == priority)
                    .disabled(!editable)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if !this.initial_action_editable() || window.has_active_dialog(cx) {
                            return;
                        }
                        this.capture_initial_action_widgets(cx);
                        let form = this.ai.as_mut().unwrap().draft.as_mut().unwrap();
                        let mut fields = form.action.as_ref().unwrap().fields.clone();
                        fields.priority = priority;
                        form.edit_action_fields(fields);
                        cx.notify();
                    })),
            );
        }
        body = body.child(states).child(priorities)
            .child("Optional saved source proofs")
            .child("Enter a saved Markdown path and capture it explicitly. Recapturing the same path replaces only that proof; other captures remain bound. Identity and current-source checks run in the shared application.")
            .child(div().id("initial-action-source-path").test_support()
                .child(Textarea::new(&self.draft_path).disabled(!editable).aria_label("Saved source path for Action proposal")))
            .child(Button::new("capture-initial-action-source").label("Capture full saved source")
                .disabled(blocked || !ai.vault_bound)
                .on_click(cx.listener(|this, _, window, cx| {
                    if this.draft_command_blocked() || window.has_active_dialog(cx) { return; }
                    this.capture_initial_action_widgets(cx);
                    if let Some(command) = this.ai.as_mut().unwrap().draft_source() { this.simple_send(command, cx); }
                })))
            .child("Complete captured Action baseline and explicit source bindings")
            .child(div().h(px(200.)).flex_shrink_0().child(Editor::new(&self.draft_link_proofs).h_full().readonly(true).aria_label("Complete captured Action baseline and source bindings")));
        if form.source_operation.is_some() {
            body =
                body.child("Capturing source… Create and leaving wait for the current response.");
        }
        if let Some(error) = &form.source_error {
            body = body.child(error.clone());
        }
        for (index, capture) in action.sources.iter().enumerate() {
            let text = capture.text.clone();
            body = body.child(
                Button::new(format!("copy-action-source-{index}"))
                    .label(format!("Copy full captured source {}", capture.source.path))
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text.clone()))
                    }),
            );
        }
        if let Some(error) = &form.error {
            body = body.child(error.clone());
        }
        if let Some((generation, record)) = &form.result {
            let proposal = record.draft.id;
            body = body
                .child(format!(
                    "Proposal {proposal} returned at review {} · submitted generation {generation}",
                    record.version
                ))
                .child(
                    Button::new("open-created-current-proposal")
                        .label("Open current proposal")
                        .disabled(!form.can_leave() || !editable)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if !window.has_active_dialog(cx) {
                                this.simple_leave(simple::EditorTransition::Review(proposal), cx);
                            }
                        })),
                );
        }
        let controls = div()
            .id("initial-action-controls")
            .test_support()
            .flex()
            .flex_wrap()
            .flex_shrink_0()
            .gap_2()
            .p_3()
            .border_t_1()
            .child(
                Button::new("create-initial-review-draft")
                    .label("Create review draft")
                    .disabled(blocked || form.result.is_some() || form.source_operation.is_some())
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.draft_command_blocked() || window.has_active_dialog(cx) {
                            return;
                        }
                        this.capture_initial_action_widgets(cx);
                        if let Some(command) = this.ai.as_mut().unwrap().create_draft() {
                            this.simple_send(command, cx);
                        }
                        cx.notify();
                    })),
            )
            .child(
                Button::new("copy-initial-action-input")
                    .label("Copy all raw Action input / proofs")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.capture_initial_action_widgets(cx);
                        let text = raw_input(this.ai.as_ref().unwrap().draft.as_ref().unwrap());
                        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
                    })),
            )
            .child(
                Button::new("discard-initial-full-form")
                    .label("Discard local form…")
                    .disabled(!editable || form.pending)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.discard_draft_dialog(false, window, cx)
                    })),
            );
        if form.submitted.is_some() {
            body = body.child(
                Button::new("separate-initial-proposal")
                    .label("Start separate proposal from retained input")
                    .disabled(!editable || form.pending)
                    .on_click(cx.listener(|this, _, window, cx| {
                        if !this.initial_action_editable() || window.has_active_dialog(cx) {
                            return;
                        }
                        this.capture_initial_action_widgets(cx);
                        if this.ai.as_mut().unwrap().separate_draft() {
                            this.simple_transition = None;
                            this.sync_draft_widgets(window, cx);
                        }
                        cx.notify();
                    })),
            );
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h(px(0.))
            .child(body)
            .child(controls)
            .into_any_element()
    }
}
