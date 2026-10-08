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
#[path = "predecessor_tests.rs"]
mod predecessor_tests;

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
        let proposal = review.record.draft.id;
        if self.review_predecessor_proposal != Some(proposal) {
            self.review_predecessor
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.review_predecessor_proposal = Some(proposal);
        }
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
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let leaving = self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || self.close_failed;
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
            .p_3();
        if let Some(review) = &ai.review {
            let editable = ai.review_editable() && !leaving;
            let can_mutate = ai.review_can_mutate() && !leaving;
            let selected_text = review
                .record
                .draft
                .changes
                .get(self.review_member)
                .is_some_and(|change| change.text().is_some());
            body = body.child(format!("{:?} · review version {} · {}", review.record.state, review.record.version,
                if review.pending() { "Awaiting full review acknowledgement" } else if review.dirty() { "Local text is not acknowledged" } else { "Full review is recoverable" }))
                .child("Proposal edits and Rewrite do not change vault knowledge. Approval is a separate exact full-proposal operation.")
                .child(Textarea::new(&self.review_title).disabled(!editable).aria_label("Full proposal title"));
            if let Some(error) = &review.error {
                body = body.child(error.clone());
            }
            if let Some(observed) = &review.observed {
                body = body.child(format!("Current review is version {} / {:?}. Local text is retained; copy or explicitly discard it before continuing.", observed.version, observed.state));
            }
            if review.predecessor_eligible() {
                let can_attach = can_mutate
                    && review.can_attach_predecessor()
                    && ai.active.is_none()
                    && ai.rewrite.is_none()
                    && self.review_comment_pending.is_none()
                    && !(self.review_comment_draft.is_some()
                        && !self.review_comment.read(cx).value().is_empty());
                body = body.child(div().id("knowledge-predecessor-controls").test_support().flex().flex_col().gap_2()
                    .child("Attach one saved Current knowledge predecessor. Exact approval of this revised proposal creates the successor and makes the selected predecessor History. Your successor wording is preserved; attachment does not decide semantic replacement.")
                    .child(Textarea::new(&self.review_predecessor).disabled(!can_attach).aria_label("Current knowledge predecessor path"))
                    .child(Button::new("attach-knowledge-predecessor").label("Attach predecessor for full review")
                        .disabled(!can_attach)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.simple_transition.is_none() && this.closing.is_none()
                                && !this.closed && !this.close_failed
                                && this.review_comment_pending.is_none()
                                && !(this.review_comment_draft.is_some() && !this.review_comment.read(cx).value().is_empty())
                            {
                                let path = this.review_predecessor.read(cx).value().to_string();
                                if let Some(command) = this.ai.as_mut().unwrap().attach_knowledge_predecessor(path) {
                                    this.simple_send(command, cx);
                                }
                            }
                            cx.notify();
                        }))));
                if !review.can_attach_predecessor() {
                    body = body.child("Predecessor attachment requires a clean, acknowledged review. Flush or resolve retained local text first.");
                }
            }
            if review.predecessor_pending() {
                body = body.child("Attaching predecessor; typing and navigation wait for the exact revised review acknowledgement.");
            }
            for (index, change) in review.record.draft.changes.iter().enumerate() {
                let kind = match change {
                    NoteChange::Create { .. } => "Create",
                    NoteChange::Replace { .. } => "Replace",
                    NoteChange::Trash { .. } => "Trash",
                    NoteChange::CreateAsset { .. } => "Create asset",
                    NoteChange::ReplaceAsset { .. } => "Replace asset",
                    NoteChange::TrashAsset { .. } => "Trash asset",
                };
                body = body.child(
                    Button::new(format!("review-member-{index}"))
                        .label(format!("{kind} · {}", change.path()))
                        .selected(index == self.review_member)
                        .disabled(leaving || review.predecessor_pending())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.review_member = index;
                            this.sync_review_widgets(window, cx);
                            cx.notify();
                        })),
                );
            }
            if let Some(change) = review.record.draft.changes.get(self.review_member) {
                match change {
                    NoteChange::Create { .. } => {
                        body = body.child("Before: no note at the captured destination");
                    }
                    NoteChange::Replace { before_text, .. }
                    | NoteChange::Trash { before_text, .. } => {
                        body = body.child("Full captured before text").child(
                            div()
                                .id("review-before")
                                .max_h(px(220.))
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
                    if review.history_member(self.review_member) {
                        body = body.child("Generated predecessor History · read only. Exact approval changes this captured Current note to History.");
                    }
                    body = body.child("Full proposed text").child(
                        div()
                            .id("review-text-editor")
                            .test_support()
                            .h(px(320.))
                            .child(
                                Editor::new(&self.review_editor)
                                    .h_full()
                                    .readonly(review.member_readonly(self.review_member))
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
            body = body.child("Captured source versions");
            if review.record.draft.sources.is_empty() {
                body = body.child("No source files recorded for this proposal");
            }
            for source in &review.record.draft.sources {
                let hash: String = source
                    .fingerprint
                    .sha256
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                body = body.child(format!("{} · captured SHA-256 {hash}", source.path));
            }
            body = body.child("Temporary review comments");
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
                body = body.child(label).child(comment.text.clone()).child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .child(
                            Button::new(format!("edit-comment-{id}"))
                                .label("Edit comment…")
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
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("review-flush")
                            .label("Flush / retry full review")
                            .disabled(review.pending() || leaving)
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(review) = &mut this.ai.as_mut().unwrap().review {
                                    review.retry();
                                }
                                if let Some(command) = this.ai.as_mut().unwrap().recover_review() {
                                    this.simple_send(command, cx);
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("review-comment-whole")
                            .label("Comment on proposal…")
                            .disabled(!can_mutate)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.review_comment_dialog(false, None, window, cx)
                            })),
                    )
                    .when(selected_text, |row| {
                        row.child(
                            Button::new("review-comment-selection")
                                .label("Comment selected text…")
                                .disabled(!can_mutate)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.review_comment_dialog(true, None, window, cx)
                                })),
                        )
                    })
                    .child(
                        Button::new("review-rewrite")
                            .label("Rewrite")
                            .disabled(leaving || !ai.can_rewrite())
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(command) = this.ai.as_mut().unwrap().start_rewrite() {
                                    this.simple_send(command, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("review-approve")
                            .label("Review exact approval…")
                            .disabled(!can_mutate || ai.application_busy() || ai.rewrite.is_some())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_approval_dialog(false, window, cx)
                            })),
                    )
                    .child(
                        Button::new("review-approve-group")
                            .label("Review captured group approval…")
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
                    .child(
                        Button::new("review-reject")
                            .label("Reject proposal")
                            .disabled(!can_mutate)
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(command) = this.ai.as_mut().unwrap().reject_review() {
                                    this.simple_send(command, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("review-observe")
                            .label("Observe current review")
                            .disabled(review.pending())
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(command) = this.ai.as_mut().unwrap().refresh_review() {
                                    this.simple_send(command, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("review-copy-local")
                            .label("Copy full local review")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(review) = &this.ai.as_ref().unwrap().review
                                    && let Ok(text) = review.copy_local()
                                {
                                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                        text,
                                    ));
                                }
                            })),
                    )
                    .child(
                        Button::new("review-discard-local")
                            .label("Discard retained local text…")
                            .disabled(
                                review.pending() || (!review.dirty() && review.error.is_none()),
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.discard_review_dialog(window, cx)
                            })),
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
                body = body.child(format!("Last Rewrite: {:?}", job.status));
                if let Some(error) = &job.error_code {
                    body = body.child(format!("Safe failure category: {error}"));
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
                body = body.child(error.clone());
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
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(super::theme::color(p.paper))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .child("Full proposal review")
                    .child(
                        Button::new("close-proposal-review")
                            .label("Close")
                            .on_click(cx.listener(|this, _, _, cx| this.close_document(cx))),
                    ),
            )
            .child(body)
            .into_any_element()
    }
}
