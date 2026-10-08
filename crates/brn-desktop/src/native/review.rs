//! Full typed review presentation. AppWorker owns persistence and AI work.
use super::*;
use brn_workflow::{
    app_worker::AppEvent,
    proposals::{CommentTarget, NoteChange, ReviewComment},
};
use gpui_kit::{
    AnyElement, TestSupportExt,
    base::Disableable,
    component::{Selectable, WindowExt, input::Textarea},
};

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
#[path = "asset_review_tests.rs"]
mod asset_tests;

impl Desktop {
    pub(super) fn settle_comment_draft(
        &mut self,
        id: Uuid,
        event: &AppEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((operation, comment, text)) = &self.review_comment_pending else {
            return;
        };
        if id != *operation {
            return;
        }
        match event {
            AppEvent::Proposal(record) => {
                if record
                    .comments
                    .iter()
                    .any(|row| row.id == *comment && row.text == *text)
                    && self.review_comment.read(cx).value().as_ref() == text
                {
                    self.review_comment
                        .update(cx, |editor, cx| editor.set_value("", window, cx));
                    self.review_comment_draft = None;
                }
                self.review_comment_pending = None;
            }
            AppEvent::Failed(_) => self.review_comment_pending = None,
            _ => {}
        }
    }
    pub(super) fn sync_review_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_action_widgets(window, cx);
        let Some(review) = &self.ai.as_ref().unwrap().review else {
            return;
        };
        self.review_member = self
            .review_member
            .min(review.record.draft.changes.len().saturating_sub(1));
        let text = review
            .text(self.review_member)
            .unwrap_or_default()
            .to_owned();
        let title = review.title().to_owned();
        if self.review_editor.read(cx).value().as_ref() != text {
            self.review_editor
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
        }
        if self.review_title.read(cx).value().as_ref() != title {
            self.review_title
                .update(cx, |input, cx| input.set_value(title, window, cx));
        }
    }

    fn review_comment_dialog(
        &mut self,
        selected: bool,
        existing: Option<Uuid>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ai = self.ai.as_ref().unwrap();
        if !ai.review_can_mutate() || window.has_active_dialog(cx) {
            return;
        }
        let review = ai.review.as_ref().unwrap();
        let stamp = review.record.stamp();
        let draft = (stamp.id, existing);
        if self
            .review_comment_draft
            .is_some_and(|previous| previous != draft)
            && !self.review_comment.read(cx).value().is_empty()
        {
            self.ai.as_mut().unwrap().notice =
                "A different comment draft is retained. Copy or explicitly discard it first."
                    .into();
            cx.notify();
            return;
        }
        let target = if selected {
            let range = self.review_editor.read(cx).selected_range();
            let Some(target) = crate::review::selection_target(review, self.review_member, range)
            else {
                self.ai.as_mut().unwrap().notice =
                    "Select a nonempty exact range in the acknowledged proposal text first.".into();
                cx.notify();
                return;
            };
            target
        } else {
            existing
                .and_then(|id| {
                    review
                        .record
                        .comments
                        .iter()
                        .find(|comment| comment.id == id)
                })
                .map(|comment| comment.target.clone())
                .unwrap_or(CommentTarget::Proposal)
        };
        let text = if self.review_comment_draft == Some(draft) {
            self.review_comment.read(cx).value().to_string()
        } else {
            existing
                .and_then(|id| {
                    review
                        .record
                        .comments
                        .iter()
                        .find(|comment| comment.id == id)
                })
                .map(|comment| comment.text.clone())
                .unwrap_or_default()
        };
        self.review_comment_draft = Some(draft);
        self.review_comment
            .update(cx, |input, cx| input.set_value(text, window, cx));
        let input = self.review_comment.clone();
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let desktop = desktop.clone(); let target = target.clone(); let input_for_click = input.clone();
            let quote = match &target { CommentTarget::Text(anchor) => anchor.quote.clone(), CommentTarget::Unresolved(anchor) => format!("Unresolved previous selection: {}", anchor.quote), _ => "Whole proposal".into() };
            dialog.title(if existing.is_some() && selected { "Explicitly reattach comment" } else if existing.is_some() { "Edit temporary comment" } else { "Temporary review comment" }).w(px(600.))
                .child(div().flex().flex_col().gap_2()
                    .child(div().id("comment-captured-quote").max_h(px(160.)).overflow_y_scroll().child(quote))
                    .child(div().h(px(180.)).child(Editor::new(&input).h_full().aria_label("Full temporary comment text")))
                    .child(Button::new("submit-review-comment").label("Save comment")
                        .on_click(move |_, window, cx| {
                            let mut submitted = false;
                            let _ = desktop.update(cx, |this, cx| {
                                let text = input_for_click.read(cx).value().to_string();
                                let comment_id = existing.unwrap_or_else(Uuid::new_v4);
                                let ai = this.ai.as_mut().unwrap();
                                if ai.review.as_ref().is_some_and(|review| review.record.stamp() == stamp)
                                    && let Some(command) = ai.review_comment(ReviewComment { id: comment_id, text: text.clone(), target: target.clone() }, existing.is_some()) {
                                    this.review_comment_pending = Some((command.0, comment_id, text));
                                    this.simple_send(command, cx); submitted = true;
                                } else { ai.notice = "Review changed while the comment was open; recapture the selection. Comment text is retained.".into(); cx.notify(); }
                            });
                            if submitted { window.close_dialog(cx); }
                        })))
        });
    }

    fn discard_review_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = &self.ai.as_ref().unwrap().review else {
            return;
        };
        if review.pending() || window.has_active_dialog(cx) {
            return;
        }
        let record = review.record.clone();
        let observed = review.observed.clone();
        let title = review.title().to_owned();
        let texts = review.texts().to_vec();
        let actions = review.action_data().to_vec();
        let fields = review.action_fields().to_vec();
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let desktop = desktop.clone(); let record = record.clone(); let observed = observed.clone();
            let title = title.clone(); let texts = texts.clone(); let actions = actions.clone(); let fields = fields.clone();
            dialog.title("Discard retained local review text?").w(px(640.))
                .child("Copy local text first if needed. This adopts the acknowledged/current proposal and does not change vault Markdown.")
                .child(Button::new("confirm-discard-review").label("Confirm discard local review text")
                    .on_click(move |_, window, cx| {
                        let mut discarded = false;
                        let _ = desktop.update(cx, |this, cx| {
                            if let Some(review) = &mut this.ai.as_mut().unwrap().review
                                && review.record == record && review.observed == observed
                                && review.title() == title && review.texts() == texts.as_slice()
                                && review.action_data() == actions.as_slice() && review.action_fields() == fields.as_slice() {
                                discarded = review.discard_local();
                            }
                            if discarded { this.sync_review_widgets(window, cx); }
                            else { this.ai.as_mut().unwrap().notice = "Review changed while confirmation was open; local text retained.".into(); }
                            cx.notify();
                        });
                        if discarded { window.close_dialog(cx); }
                    }))
        });
    }

    pub(super) fn render_proposal_review(&mut self, cx: &mut Context<Self>) -> AnyElement {
        self.sync_comment_marks(cx);
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let leaving = self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || self.close_failed;
        use super::ui::{self, Tone};
        let mut decision: Option<gpui_kit::Div> = None;
        let mut body = div()
            .id("full-proposal-review")
            .test_support()
            .track_scroll(&self.review_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_2()
            .px(px(tokens::space::LG))
            .py(px(tokens::space::MD));
        if let Some(review) = &ai.review {
            let editable = ai.review_editable() && !leaving;
            let can_mutate = ai.review_can_mutate() && !leaving;
            let selected_text = review
                .record
                .draft
                .changes
                .get(self.review_member)
                .is_some_and(|change| change.text().is_some());
            let _ = editable;
            if let Some(error) = &review.error {
                body = body.child(ui::callout(Tone::Danger, error.clone(), p));
            }
            if let Some(observed) = &review.observed {
                body = body.child(ui::callout(Tone::Attention, format!("Current review is version {} / {:?}. Local text is retained; copy or explicitly discard it before continuing.", observed.version, observed.state), p));
            }
            let several = review.record.draft.changes.len() > 1;
            let mut members = ui::toolbar().gap(px(tokens::space::XS));
            for (index, change) in review.record.draft.changes.iter().enumerate() {
                let kind = match change {
                    NoteChange::Create { .. } => "Create",
                    NoteChange::Replace { .. } => "Replace",
                    NoteChange::Trash { .. } => "Trash",
                    NoteChange::CreateAsset { .. } => "Create asset",
                    NoteChange::ReplaceAsset { .. } => "Replace asset",
                    NoteChange::TrashAsset { .. } => "Trash asset",
                };
                members = members.child(
                    Button::new(format!("review-member-{index}"))
                        .label(format!("{kind} · {}", change.path()))
                        .small()
                        .when(index == self.review_member, |button| button.outline())
                        .when(index != self.review_member, |button| button.ghost())
                        .selected(index == self.review_member)
                        .disabled(leaving)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.review_member = index;
                            this.sync_review_widgets(window, cx);
                            cx.notify();
                        })),
                );
            }
            if several {
                body = body.child(members);
            }
            if let Some(change) = review.record.draft.changes.get(self.review_member) {
                match change {
                    // The Create badge already says this is a new note.
                    NoteChange::Create { .. } => {}
                    NoteChange::Replace { before_text, .. }
                    | NoteChange::Trash { before_text, .. } => {
                        body = body
                            .child(ui::section_label("Current text being replaced", p).px_0())
                            .child(
                                div()
                                    .id("review-before")
                                    .max_h(px(160.))
                                    .overflow_y_scroll()
                                    .child(before_text.clone()),
                            );
                    }
                    NoteChange::CreateAsset { .. }
                    | NoteChange::ReplaceAsset { .. }
                    | NoteChange::TrashAsset { .. } => {
                        body = body.child(super::approval::asset_body(
                            &format!("review-asset-{}", self.review_member),
                            change,
                        ));
                    }
                }
                if change.text().is_some() {
                    let editor_h = (self.viewport_h - 230.).max(320.);
                    body = body.child(
                        div()
                            .id("review-text-editor")
                            .test_support()
                            .h(px(editor_h))
                            .child(
                                Editor::new(&self.review_editor)
                                    .context_menu(|menu, _, _| {
                                        menu.menu(
                                            "Comment on selection…",
                                            Box::new(CommentOnSelection),
                                        )
                                        .separator()
                                        .menu("Copy", Box::new(gpui_kit::component::input::Copy))
                                        .menu(
                                            "Select All",
                                            Box::new(gpui_kit::component::input::SelectAll),
                                        )
                                    })
                                    .h_full()
                                    .font_family(tokens::UI_FONT)
                                    .text_size(px(tokens::text::READING + 1.0))
                                    .readonly(review.record.draft.inbox_source.is_some())
                                    .disabled(!editable)
                                    .aria_label("Full proposed Markdown member"),
                            ),
                    );
                } else if !change.is_asset() {
                    body =
                        body.child("Proposed: move this exact original note to recoverable Trash");
                }
            }
            for (index, change) in review.record.draft.action_changes.iter().enumerate() {
                body = body.child(self.action_editor_body(index, change, editable, cx));
            }
            if !review.record.draft.sources.is_empty() {
                body = body.child(ui::section_label("Captured source versions", p).px_0());
            }
            for source in &review.record.draft.sources {
                let hash: String = source
                    .fingerprint
                    .sha256
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                body = body.child(ui::meta(
                    format!("{} · captured SHA-256 {hash}", source.path),
                    p,
                ));
            }
            body = body
                .child(ui::section_label("Comments", p).px_0())
                .child(ui::hint(
                    "Select text and right-click to comment. Hover a highlight to read it.",
                    p,
                ));
            for comment in &review.record.comments {
                let id = comment.id;
                let label = match &comment.target {
                    CommentTarget::Proposal => "Whole proposal".to_owned(),
                    CommentTarget::Text(anchor) => format!(
                        "Selected text in member {}: {}",
                        anchor.change_index + 1,
                        anchor.quote
                    ),
                    CommentTarget::Unresolved(anchor) => format!(
                        "Unresolved previous selection in member {}: {}",
                        anchor.change_index + 1,
                        anchor.quote
                    ),
                };
                body = body
                    .child(ui::callout(
                        Tone::Attention,
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(ui::meta(label, p))
                            .child(comment.text.clone()),
                        p,
                    ))
                    .child(
                        ui::toolbar()
                            .child(
                                Button::new(format!("edit-comment-{id}"))
                                    .label("Edit comment…")
                                    .ghost()
                                    .small()
                                    .disabled(!can_mutate)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.review_comment_dialog(false, Some(id), window, cx)
                                    })),
                            )
                            .when(selected_text, |row| {
                                row.child(
                                    Button::new(format!("reattach-comment-{id}"))
                                        .label("Reattach to selected text…")
                                        .disabled(!can_mutate)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.review_comment_dialog(true, Some(id), window, cx)
                                        })),
                                )
                            })
                            .child(
                                Button::new(format!("delete-comment-{id}"))
                                    .label("Remove comment")
                                    .ghost()
                                    .small()
                                    .disabled(!can_mutate)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(command) =
                                            this.ai.as_mut().unwrap().remove_review_comment(id)
                                        {
                                            this.simple_send(command, cx);
                                        }
                                    })),
                            ),
                    );
            }
            body = body.child(
                ui::toolbar()
                    .child(
                        Button::new("review-comment-whole")
                            .label("Comment on proposal…")
                            .outline()
                            .small()
                            .disabled(!can_mutate)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.review_comment_dialog(false, None, window, cx)
                            })),
                    )
                    .when(selected_text, |row| {
                        row.child(
                            Button::new("review-comment-selection")
                                .label("Comment selected text…")
                                .outline()
                                .small()
                                .disabled(!can_mutate)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.review_comment_dialog(true, None, window, cx)
                                })),
                        )
                    })
                    .child(
                        Button::new("review-rewrite")
                            .label("Rewrite")
                            .outline()
                            .small()
                            .tooltip("Ask the selected model to rewrite the whole proposal using your comments. Knowledge stays unchanged.")
                            .disabled(leaving || !ai.can_rewrite())
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(command) = this.ai.as_mut().unwrap().start_rewrite() {
                                    this.simple_send(command, cx);
                                }
                            })),
                    ),
            );
            let auto_tools = review.dirty() || review.error.is_some() || review.observed.is_some();
            if !auto_tools {
                body = body.child(super::simple::tools_toggle(self.show_tools, cx));
            }
            if auto_tools || self.show_tools {
                body = body
                    .child(ui::section_label("Review state", p).px_0())
                    .child(ui::meta(
                        format!(
                            "Review version {} · {}",
                            review.record.version,
                            if review.pending() {
                                "Awaiting full review acknowledgement"
                            } else if review.dirty() {
                                "Local text is not acknowledged"
                            } else {
                                "Full review is recoverable"
                            }
                        ),
                        p,
                    ))
                    .child(
                        ui::toolbar()
                            .child(
                                Button::new("review-flush")
                                    .label("Flush / retry full review")
                                    .ghost()
                                    .small()
                                    .disabled(review.pending() || leaving)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(review) = &mut this.ai.as_mut().unwrap().review
                                        {
                                            review.retry();
                                        }
                                        if let Some(command) =
                                            this.ai.as_mut().unwrap().recover_review()
                                        {
                                            this.simple_send(command, cx);
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("review-observe")
                                    .label("Observe current review")
                                    .ghost()
                                    .small()
                                    .disabled(review.pending())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(command) =
                                            this.ai.as_mut().unwrap().refresh_review()
                                        {
                                            this.simple_send(command, cx);
                                        }
                                    })),
                            )
                            .child(
                                Button::new("review-copy-local")
                                    .label("Copy full local review")
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(review) = &this.ai.as_ref().unwrap().review
                                            && let Ok(text) = review.copy_local()
                                        {
                                            cx.write_to_clipboard(
                                                gpui_kit::ClipboardItem::new_string(text),
                                            );
                                        }
                                    })),
                            )
                            .child(
                                Button::new("review-discard-local")
                                    .label("Discard retained local text…")
                                    .ghost()
                                    .small()
                                    .disabled(
                                        review.pending()
                                            || (!review.dirty() && review.error.is_none()),
                                    )
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.discard_review_dialog(window, cx)
                                    })),
                            ),
                    );
            }
            decision = Some(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(ui::hint(
                        format!(
                            "Nothing in your vault changes until you approve version {}.",
                            review.record.version
                        ),
                        p,
                    ))
                    .child(
                ui::toolbar()
                    .justify_end()
                    .child(
                        Button::new("review-reject")
                            .label("Reject proposal")
                            .outline()
                            .small()
                            .disabled(!can_mutate)
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(command) = this.ai.as_mut().unwrap().reject_review() {
                                    this.simple_send(command, cx);
                                }
                            })),
                    )
                    .when(review.record.draft.group_id.is_some(), |row| {
                        row.child(
                        Button::new("review-approve-group")
                            .label("Review captured group approval…")
                            .outline()
                            .small()
                            .disabled(
                                !can_mutate
                                    || review.record.draft.group_id.is_none()
                                    || ai.application_busy()
                                    || ai.rewrite.is_some(),
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_approval_dialog(true, window, cx)
                            })),
                    )
                    })
                    .child(
                        Button::new("review-approve")
                            .label("Review exact approval…")
                            .primary()
                            .small()
                            .tooltip("Inspect exactly what will change, then approve this version")
                            .disabled(!can_mutate || ai.application_busy() || ai.rewrite.is_some())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_approval_dialog(false, window, cx)
                            })),
                    ),
                    ),
            );
            if let Some(active) = &ai.rewrite {
                body = body.child(format!(
                    "Rewrite · {} / {} / {} · {}",
                    crate::ai::provider_name(active.request.selection.provider),
                    active.request.selection.model,
                    active.request.effort.as_str(),
                    if active.stopping {
                        "Stopping; not finalized"
                    } else {
                        "Running; knowledge unchanged"
                    }
                ));
                if let Some(tool) = &active.tool {
                    body = body.child(format!("Read tool: {tool}"));
                }
                body = body.child(
                    Button::new("review-stop-rewrite")
                        .label("Stop Rewrite")
                        .on_click(cx.listener(|this, _, _, cx| this.simple_stop(cx))),
                );
            }
            if let Some(job) = &ai.last_rewrite
                && job.spec.expected.id == review.record.draft.id
            {
                body = body.child(ui::meta(format!("Last Rewrite: {:?}", job.status), p));
                if let Some(error) = &job.error_code {
                    body = body.child(ui::callout(
                        Tone::Danger,
                        format!("Safe failure category: {error}"),
                        p,
                    ));
                }
            }
            if ai.application_busy() {
                body = body.child("Waiting for application and current-review acknowledgement.");
            }
            for receipt in &ai.approval_receipts {
                body = body.child(format!(
                    "Operation {} · proposal {} · approved version {} · {:?}",
                    receipt.operation_id,
                    receipt.proposal_id,
                    receipt.approved_version,
                    receipt.outcome
                ));
            }
            if let Some(error) = &ai.approval_error {
                body = body.child(ui::callout(Tone::Danger, error.clone(), p));
            }
            if self.review_comment_draft.is_some()
                && !self.review_comment.read(cx).value().is_empty()
            {
                let draft = self.review_comment.read(cx).value().to_string();
                let copied = draft.clone();
                body = body
                    .child("Retained comment draft · not acknowledged · copy before closing")
                    .child(
                        div()
                            .id("retained-comment-draft")
                            .max_h(px(180.))
                            .overflow_y_scroll()
                            .child(draft),
                    )
                    .child(
                        Button::new("copy-retained-comment")
                            .label("Copy comment draft")
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                    copied.clone(),
                                ))
                            })),
                    )
                    .child(
                        Button::new("discard-retained-comment")
                            .label("Discard comment draft")
                            .disabled(self.review_comment_pending.is_some())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.review_comment
                                    .update(cx, |editor, cx| editor.set_value("", window, cx));
                                this.review_comment_draft = None;
                                cx.notify();
                            })),
                    );
            }
            if self.simple_transition.is_some() {
                body = body.child(
                    Button::new("cancel-review-leave")
                        .label("Cancel pending close / navigation")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.simple_transition = None;
                            this.ai.as_mut().unwrap().notice =
                                "Navigation cancelled; full local review retained.".into();
                            cx.notify();
                        })),
                );
            }
        } else {
            body = body.child(
                ai.review_error
                    .clone()
                    .unwrap_or("Opening full proposal review…".into()),
            );
        }
        let header = match &ai.review {
            Some(review) => {
                let (label, tone) = super::simple::proposal_state_badge(review.record.state);
                let editable = ai.review_editable() && !leaving;
                let identity = match review.record.draft.changes.get(self.review_member) {
                    Some(change) if review.record.draft.changes.len() == 1 => {
                        let kind = match change {
                            NoteChange::Create { .. } => "Create",
                            NoteChange::Replace { .. } => "Replace",
                            NoteChange::Trash { .. } => "Trash",
                            NoteChange::CreateAsset { .. } => "Create asset",
                            NoteChange::ReplaceAsset { .. } => "Replace asset",
                            NoteChange::TrashAsset { .. } => "Trash asset",
                        };
                        format!("{kind} · {} · v{}", change.path(), review.record.version)
                    }
                    _ => format!(
                        "{} changes · v{}",
                        review.record.draft.changes.len()
                            + review.record.draft.action_changes.len(),
                        review.record.version
                    ),
                };
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(tokens::space::SM))
                    .px(px(tokens::space::LG))
                    .py(px(tokens::space::SM))
                    .border_b_1()
                    .border_color(super::theme::color(p.line))
                    .child(ui::badge(label, tone, p))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(px(0.))
                            .child(
                                Textarea::new(&self.review_title)
                                    .appearance(false)
                                    .text_size(px(tokens::text::TITLE))
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .disabled(!editable)
                                    .aria_label("Full proposal title"),
                            )
                            .child(ui::meta(identity, p)),
                    )
                    .child(
                        ui::quiet("close-proposal-review", "Close")
                            .on_click(cx.listener(|this, _, _, cx| this.close_document(cx))),
                    )
            }
            None => ui::view_header("Proposal", None, None, p).child(
                ui::quiet("close-proposal-review", "Close")
                    .on_click(cx.listener(|this, _, _, cx| this.close_document(cx))),
            ),
        };
        let mut pane = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(super::theme::color(p.paper))
            // Right-click "Comment on selection…" in the proposed text arrives here.
            .on_action(cx.listener(|this, _: &CommentOnSelection, window, cx| {
                let leaving = this.simple_transition.is_some()
                    || this.closing.is_some()
                    || this.closed
                    || this.close_failed;
                if !leaving {
                    this.review_comment_dialog(true, None, window, cx);
                }
            }))
            .child(header)
            .child(body);
        if let Some(decision) = decision {
            pane = pane.child(
                decision
                    .px(px(tokens::space::LG))
                    .py(px(tokens::space::SM))
                    .border_t_1()
                    .border_color(super::theme::color(p.line))
                    .bg(super::theme::color(p.panel)),
            );
        }
        pane.into_any_element()
    }
}

impl Desktop {
    /// Shows text comments in the proposed-text editor: a tinted highlight on each
    /// exactly anchored range, and the comment text when the pointer hovers it.
    /// Anchors whose saved quote no longer matches the editor text are not shown
    /// in the text (never re-anchored by guessing); they stay in the comment list.
    fn sync_comment_marks(&mut self, cx: &mut Context<Self>) {
        use gpui_kit::base::input::{Diagnostic, DiagnosticSeverity, RopeExt, TextDecoration};
        use std::hash::{Hash, Hasher};
        let ai = self.ai.as_ref().unwrap();
        let text = self.review_editor.read(cx).value().to_string();
        let marks: Vec<(std::ops::Range<usize>, String)> = ai
            .review
            .as_ref()
            .map(|review| {
                review
                    .record
                    .comments
                    .iter()
                    .filter_map(|comment| match &comment.target {
                        CommentTarget::Text(anchor)
                            if anchor.change_index == self.review_member
                                && text.get(anchor.start..anchor.end)
                                    == Some(anchor.quote.as_str()) =>
                        {
                            Some((anchor.start..anchor.end, comment.text.clone()))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let p = self.palette();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (&text, &marks, p.amber).hash(&mut hasher);
        let signature = hasher.finish();
        if self.comment_marks == Some(signature) {
            return;
        }
        self.comment_marks = Some(signature);
        let tint = super::theme::color(p.amber).opacity(0.30);
        let decorations: Vec<TextDecoration> = marks
            .iter()
            .map(|(range, _)| {
                TextDecoration::new(
                    range.clone(),
                    gpui_kit::HighlightStyle {
                        background_color: Some(tint),
                        ..Default::default()
                    },
                )
            })
            .collect();
        let existing = self.comment_decorations.clone();
        let created = self.review_editor.update(cx, |editor, cx| {
            let rope = editor.text().clone();
            if let Some(set) = editor.diagnostics_mut() {
                set.reset(&rope);
                for (range, comment) in &marks {
                    set.push(
                        Diagnostic::new(
                            rope.offset_to_position(range.start)
                                ..rope.offset_to_position(range.end),
                            format!("**Comment:** {comment}"),
                        )
                        .with_severity(DiagnosticSeverity::Hint),
                    );
                }
            }
            cx.notify();
            existing
                .is_none()
                .then(|| editor.create_decorations_collection(decorations.clone(), cx))
        });
        // The collection updates the editor itself, so set it outside that update.
        if let Some(collection) = &self.comment_decorations {
            collection.set(decorations, cx);
        }
        if created.is_some() {
            self.comment_decorations = created;
        }
    }
}
