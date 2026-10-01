use super::*;

impl Desktop {
    pub(super) fn render_centre(
        &mut self,
        resolved: &ResolvedLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = self.palette();
        let centre = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .h_full();
        match resolved.mode {
            CentreMode::ChatOnly => centre.child(self.render_chat(cx)),
            CentreMode::Split => centre.child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.))
                    .child(
                        div()
                            .w(px(resolved.doc_w))
                            .flex_shrink_0()
                            .h_full()
                            .child(self.render_document(cx)),
                    )
                    .child(self.render_divider(crate::layout::Divider::Document, window, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .h_full()
                            .child(self.render_chat(cx)),
                    ),
            ),
            CentreMode::Tabs => {
                let tab = self.centre_tab;
                let bar = div()
                    .flex()
                    .flex_shrink_0()
                    .gap_1()
                    .p_1()
                    .border_b_1()
                    .border_color(color(p.line))
                    .bg(color(p.panel))
                    .child(
                        Button::new("tab-document")
                            .label("Document")
                            .compact()
                            .selected(tab == CentreTab::Document)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.centre_tab = CentreTab::Document;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("tab-chat")
                            .label("Chat")
                            .compact()
                            .selected(tab == CentreTab::Chat)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.centre_tab = CentreTab::Chat;
                                cx.notify();
                            })),
                    );
                let pane = match tab {
                    CentreTab::Document => self.render_document(cx).into_any_element(),
                    CentreTab::Chat => self.render_chat(cx).into_any_element(),
                };
                centre
                    .child(bar)
                    .child(div().flex_1().min_h(px(0.)).child(pane))
            }
        }
    }

    pub(super) fn render_document(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let (label, body) = match self.open_doc {
            Some(DocRef::Draft) => (
                format!(
                    "Drafts / {}",
                    self.draft_state
                        .as_ref()
                        .map_or("opening…".to_string(), |state| state.title().to_string())
                ),
                self.render_draft_document(cx),
            ),
            Some(DocRef::Source(id)) => (
                format!(
                    "Sources / {}",
                    self.sources
                        .iter()
                        .find(|source| source.source_id == id)
                        .map_or("unavailable".to_string(), |source| compact_title(
                            &source.title
                        ))
                ),
                self.render_source_reader(id, cx),
            ),
            Some(DocRef::Note(_)) => (
                format!(
                    "Notes / {}",
                    self.note_state.as_ref().map_or("opening…".into(), |state| {
                        state.view().relative_path.display().to_string()
                    })
                ),
                self.render_note_document(cx),
            ),
            None => (String::new(), div().into_any_element()),
        };
        let scroll = match self.open_doc {
            Some(DocRef::Source(_)) => self.source_scroll.clone(),
            Some(DocRef::Note(_)) => self.note_scroll.clone(),
            _ => self.draft_scroll.clone(),
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(color(p.paper))
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .border_b_1()
                    .border_color(color(p.line))
                    .text_color(color(p.muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(label),
                    )
                    .child(
                        Button::new("close-document")
                            .label("Close")
                            .compact()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_document(cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("document-body")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.))
                    .gap_3()
                    .p_3()
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .vertical_scrollbar(&scroll)
                    .child(body),
            )
    }

    // Interim note document integration; UI slice 3 will redesign these controls.
    fn render_note_document(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let mut body = div().flex().flex_col().min_w(px(0.)).gap_3()
            .child("Explicit Save/Cmd-S writes Markdown. Recovery in BRN is not publication or search approval.");
        if let Some(failure) = &self.last_note_failure {
            body = body.child(format!("Recorded note failure: operation {:?}, note {:?}, {:?}, phase {:?}, filesystem {:?}, recovery confirmed: {}. {}",
                        failure.operation_id, failure.note_id, failure.code, failure.phase,
                        failure.filesystem_outcome, failure.recovery_available, failure.message))
                        .child(Button::new("dismiss-note-error").label("Dismiss error display (does not resolve operation)")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.last_note_failure = None;
                                cx.notify();
                            })));
        }
        if let Some(state) = &self.note_state {
            let id = state.id();
            body = body
                .child(format!(
                    "{} - {}",
                    state.view().relative_path.display(),
                    state.display_state()
                ))
                .child(format!(
                    "Availability: {:?}; last observed search approval: {:?}",
                    state.view().availability,
                    state.view().search_approval
                ));
            if let Some(message) = &state.view().availability_message {
                body = body.child(message.clone());
            }
            if let Some(failure) = state.failure() {
                body = body.child(format!(
                    "Retained operation {:?}: {:?}, phase {:?}, effect {:?}; {}",
                    failure.operation_id,
                    failure.code,
                    failure.phase,
                    failure.filesystem_outcome,
                    failure.message
                ));
            }
            if let Some(failure) = state.copy_failure() {
                body = body.child(format!(
                    "Separate copy outcome {:?}: {:?}; {}",
                    failure.operation_id, failure.filesystem_outcome, failure.message
                ));
            }
            if state.needs_recovery() || state.pending() || self.note_schedule.closing() {
                body = body.child("Recovery/operation pending. The 500 ms timer schedules work; only an acknowledged commit protects the latest typing. Busy provider work may delay it.");
            }
            body = body
                        .child(div().key_context("MarkdownNote").child(
                            Editor::new(&self.note_editor).h(px(300.)).flex_shrink_0()
                                .disabled(self.pending_note_open.is_some() || state.discard_pending())
                                .aria_label("Markdown note editor")))
                        .child(div().flex().flex_wrap().gap_2()
                            .child(Button::new("save-note").label("Save to Markdown (Cmd-S)")
                                .disabled(self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| this.queue_note_control(NoteControl::Save, cx))))
                            .child(Button::new("retry-note-recovery").label("Flush / retry recovery")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.note_schedule.retry();
                                    cx.notify();
                                })))
                            .child(Button::new("cancel-note-close").label("Cancel pending close/action")
                                .disabled(!self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.note_schedule.take_close();
                                    this.note_control = None;
                                    this.message = "Close/action cancelled; note work remains accessible.".into();
                                    cx.notify();
                                })))
                            .child(Button::new("compare-note").label("Compare baseline / local / disk")
                                .disabled(self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| this.queue_note_control(NoteControl::Compare, cx))))
                            .child(Button::new("reload-note").label("Confirm reload: discard local edits")
                                .disabled(!self.phase.can_submit() || state.pending() || self.pending_note_job.is_some())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.note_schedule.take_close();
                                    this.note_control = None;
                                    this.run_note_control(NoteControl::Reload, cx);
                                })))
                            .child(Button::new("discard-unrecovered-note").label("Confirm discard ONLY unrecovered typing")
                                .disabled(state.pending() || self.pending_note_job.is_some())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if let Some(state) = &mut this.note_state {
                                        let result = state.begin_discard(Uuid::new_v4())
                                            .and_then(|request| state.discard_unrecovered(request.operation_id));
                                        match result {
                                            Ok(()) => {
                                                let text = state.input_text();
                                                this.note_editor.update(cx, |editor, cx| editor.set_value(text, window, cx));
                                                this.message = "Unrecovered typing explicitly discarded; acknowledged recovery and any uncertain operation remain retained.".into();
                                            }
                                            Err(error) => this.message = error.message,
                                        }
                                    }
                                    cx.notify();
                                })))
                            .child(Button::new("inspect-note-recovery").label("Inspect retained recovery")
                                .disabled(!self.phase.can_submit())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.submit(Action::NoteRecovery { id }, "Inspect note recovery", cx);
                                }))))
                        .child(Input::new(&self.note_path))
                        .child(div().flex().flex_wrap().gap_2()
                            .child(Button::new("copy-note").label("Save separate copy to chosen path")
                                .disabled(self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let path = PathBuf::from(this.note_path.read(cx).value().to_string());
                                    this.queue_note_control(NoteControl::Copy(path), cx);
                                })))
                            .child(Button::new("relink-note").label("Confirm selected path identity; retain edits")
                                .disabled(self.note_schedule.closing())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let path = PathBuf::from(this.note_path.read(cx).value().to_string());
                                    this.queue_note_control(NoteControl::Relink(path), cx);
                                }))));
            if let Some(token) = state.view().current_file_state {
                body = body.child(
                    Button::new("approve-note")
                        .label("Explicitly approve saved snapshot for search (not Save)")
                        .disabled(
                            self.note_schedule.closing()
                                || state.view().availability
                                    != brn_workflow::notes::NoteAvailability::Available,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.queue_note_control(NoteControl::Approve { file_state: token }, cx);
                        })),
                );
            }
            if let Some(recovery) = self
                .note_recoveries
                .iter()
                .find(|recovery| recovery.note_id == id)
            {
                for op in &recovery.pending_operations {
                    let op = *op;
                    body = body.child(
                        Button::new(format!("reconcile-note-{op}"))
                            .label(format!(
                                "Inspect/reconcile operation {op} (no filesystem replay)"
                            ))
                            .disabled(!self.phase.can_submit() || state.pending())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.original_note_operation = Some(op);
                                if let Some(job) = this.submit(
                                    Action::ReconcileNoteSave { op },
                                    "Reconcile note save",
                                    cx,
                                ) {
                                    this.pending_note_job = Some((job, op));
                                }
                            })),
                    );
                }
            }
        }
        if let Some(comparison) = &self.note_comparison {
            let local = self
                .note_state
                .as_ref()
                .map_or(comparison.working.as_str(), NoteEditor::text);
            body = body.child(format!("Starting snapshot:\n{}\n\nLocal editor:\n{}\n\nObserved disk:\n{}\n\nUnexpected displacement, if any, remains protected in the operation recovery; comparison does not delete it.",
                        comparison.baseline, local, comparison.observed.as_deref().unwrap_or("[deleted or unavailable]")));
            if let (Some(save_op), Some(file_state)) =
                (self.original_note_operation, comparison.observed_file_state)
            {
                body = body.child(Button::new("accept-note-disk")
                            .label("Accept reviewed current disk state; KEEP recovery and original outcome")
                            .disabled(self.note_schedule.closing())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.queue_note_control(NoteControl::Accept { save_op, file_state }, cx);
                            })));
            }
        }
        if let Some(copy) = &self.note_copy {
            body = body.child(format!("Verified separate copy: {} ({}) - {:?}. Original editor retained; copy has no inherited search approval.",
                        copy.relative_path.display(), copy.id, copy.availability));
        }
        body.into_any_element()
    }

    fn render_draft_document(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let mut panel = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .min_w(px(0.))
            .gap_3();
        if self.draft_state.is_none() {
            return panel.child("Opening draft…").into_any_element();
        }
        if let Some(state) = &self.draft_state {
            let status = if state.pending() {
                "Saving snapshot; edits remain editable"
            } else if state.dirty() {
                "Unsaved changes"
            } else {
                "Saved"
            };
            let draft_id = state.id();
            panel = panel
                .child(format!(
                    "Working copy: {} · {} · {} bytes · generation {}",
                    state.title(),
                    status,
                    state.text().len(),
                    state.generation()
                ))
                .child(
                    div()
                        .id("draft-editor-anchor")
                        .anchor_scroll(Some(self.draft_editor_anchor.clone()))
                        .child(
                            Editor::new(&self.draft_editor)
                                .h(px(250.))
                                .flex_shrink_0()
                                .aria_label("Markdown working copy"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            Button::new("save-working-copy")
                                .label("Save working copy")
                                .disabled(
                                    !self.phase.can_submit()
                                        || !state.dirty()
                                        || state.text().len()
                                            > brn_workflow::worker::MAX_DRAFT_BYTES,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.save_draft(false, cx))),
                        )
                        .child(
                            Button::new("save-checkpoint")
                                .label("Save checkpoint")
                                .disabled(
                                    !self.phase.can_submit()
                                        || state.text().len()
                                            > brn_workflow::worker::MAX_DRAFT_BYTES,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.save_draft(true, cx))),
                        )
                        .child(
                            Button::new("discard-draft-edits")
                                .label("Discard edits")
                                .disabled(!self.phase.can_submit() || !state.dirty())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.discard_draft(window, cx)
                                })),
                        ),
                )
                .child(format!("Base checkpoint: {}", state.stamp().base_revision))
                .child(format!("Revision history: {}", self.revisions.len()));
            let mut comment_panel = div()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .gap_2()
                .child("Comments")
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            Button::new("capture-comment-selection")
                                .label("Capture selection")
                                .disabled(state.pending())
                                .on_click(cx.listener(|this, _, _, cx| this.capture_comment(cx))),
                        )
                        .child(
                            Button::new("refresh-comments")
                                .label("Refresh comments")
                                .disabled(!self.phase.can_submit())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.submit(
                                        Action::RefreshDraftComments { id: draft_id },
                                        "Refresh comments",
                                        cx,
                                    );
                                })),
                        ),
                );
            if let Some(quote) = state.capture_quote() {
                comment_panel = comment_panel.child("Captured exact quote:").child(
                    div()
                        .id("captured-quote-scroll")
                        .h(px(90.))
                        .min_w(px(0.))
                        .flex_shrink_0()
                        .border_1()
                        .overflow_y_scroll()
                        .child(
                            div()
                                .w_full()
                                .font_family("Menlo")
                                .flex()
                                .flex_col()
                                .children(
                                    quote.split('\n').map(|line| div().child(line.to_owned())),
                                ),
                        ),
                );
            } else {
                comment_panel =
                    comment_panel.child("Select a passage in the editor, then capture it.");
            }
            comment_panel = comment_panel
                .child(
                    Editor::new(&self.comment_input)
                        .h(px(95.))
                        .flex_shrink_0()
                        .aria_label("Comment body"),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            Button::new("add-draft-comment")
                                .label("Save checkpoint + add comment")
                                .disabled(
                                    !self.phase.can_submit()
                                        || !state.can_add_comment(state.composer()),
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.add_comment(cx))),
                        )
                        .child(
                            Button::new("discard-draft-comment")
                                .label("Discard comment")
                                .disabled(state.composer().is_empty() || state.pending())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.discard_comment(window, cx)
                                })),
                        ),
                );
            let preview = state.preview_states();
            if let Err(error) = &preview {
                comment_panel = comment_panel.child(format!("Preview unavailable: {error}"));
            }
            if state.comments().is_empty() {
                comment_panel = comment_panel.child("No comments yet.");
            }
            for (index, view) in state.comments().iter().enumerate() {
                let id = view.comment.id;
                let anchor = preview.as_ref().ok().and_then(|states| states.get(index));
                let location = match anchor {
                    Some(AnchorState::Anchored { start, end }) => {
                        format!("Anchored at bytes {start}..{end}")
                    }
                    Some(AnchorState::Deleted) => "Deleted passage".into(),
                    Some(AnchorState::Ambiguous { reason }) => {
                        format!("Location ambiguous: {reason:?}")
                    }
                    None => "Location unavailable".into(),
                };
                let toggle = if view.comment.status == CommentStatus::Open {
                    "Resolve"
                } else {
                    "Reopen"
                };
                let mut card = div()
                    .min_w(px(0.))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .border_1()
                    .child(format!(
                        "Comment {}… · {:?} · {location}",
                        &id.to_string()[..8],
                        view.comment.status
                    ))
                    .child("Original quote:")
                    .child(
                        div()
                            .id(format!("quote-scroll-{id}"))
                            .h(px(128.))
                            .min_w(px(0.))
                            .flex_shrink_0()
                            .border_1()
                            .overflow_y_scroll()
                            .child(
                                div()
                                    .w_full()
                                    .font_family("Menlo")
                                    .flex()
                                    .flex_col()
                                    .children(
                                        view.comment
                                            .original_quote
                                            .split('\n')
                                            .map(|line| div().child(line.to_owned())),
                                    ),
                            ),
                    )
                    .child("Comment body:")
                    .child(
                        div()
                            .id(format!("comment-body-scroll-{id}"))
                            .h(px(105.))
                            .min_w(px(0.))
                            .flex_shrink_0()
                            .overflow_y_scroll()
                            .child(
                                div().w_full().flex().flex_col().children(
                                    view.comment
                                        .body
                                        .split('\n')
                                        .map(|line| div().child(line.to_owned())),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                Button::new(format!("show-passage-{id}"))
                                    .label("Show passage")
                                    .disabled(!matches!(anchor, Some(AnchorState::Anchored { .. })))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.show_comment_passage(id, window, cx)
                                    })),
                            )
                            .child(
                                Button::new(format!("show-original-{id}"))
                                    .label("Show original revision")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.show_comment_original(id, cx)
                                    })),
                            )
                            .child(
                                Button::new(format!("toggle-comment-{id}"))
                                    .label(toggle)
                                    .disabled(!self.phase.can_submit())
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.change_comment_status(id, cx)
                                    })),
                            ),
                    );
                if let Some(original) = &self.comment_original
                    && original.id == view.comment.original_revision_id
                {
                    card = card
                        .child(format!("Original checkpoint {} · read-only", original.id))
                        .child(
                            div()
                                .id(format!("comment-original-scroll-{id}"))
                                .h(px(170.))
                                .min_w(px(0.))
                                .flex_shrink_0()
                                .border_1()
                                .overflow_y_scroll()
                                .child(
                                    div()
                                        .w_full()
                                        .font_family("Menlo")
                                        .flex()
                                        .flex_col()
                                        .children(
                                            original
                                                .text
                                                .split('\n')
                                                .map(|line| div().child(line.to_owned())),
                                        ),
                                ),
                        );
                }
                comment_panel = comment_panel.child(card);
            }
            panel = panel.child(comment_panel);
            for revision in &self.revisions {
                let id = revision.id;
                let label = format!(
                    "{:?} · {}…{}",
                    revision.kind,
                    &id.to_string()[..8],
                    revision.origin_turn.map_or(String::new(), |turn| format!(
                        " · answer {}…",
                        &turn.to_string()[..8]
                    ))
                );
                panel = panel.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            Button::new(format!("review-{id}"))
                                .label(format!("View {label}"))
                                .disabled(!self.phase.can_submit())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.submit(
                                        Action::OpenDraftRevision {
                                            draft: draft_id,
                                            revision: id,
                                        },
                                        "Review revision",
                                        cx,
                                    );
                                })),
                        )
                        .child(
                            Button::new(format!("before-{id}"))
                                .label("Use as before")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.compare_before = Some(id);
                                    this.diff = None;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new(format!("after-{id}"))
                                .label("Use as after")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.compare_after = Some(id);
                                    this.diff = None;
                                    cx.notify();
                                })),
                        ),
                );
            }
            if let Some(revision) = &self.review {
                panel = panel
                    .child(format!(
                        "Read-only {:?} {} · parent {} · origin answer {}",
                        revision.kind,
                        revision.id,
                        revision
                            .parent_id
                            .map_or("none".into(), |id| id.to_string()),
                        revision
                            .origin_turn
                            .map_or("none".into(), |id| id.to_string())
                    ))
                    .child(
                        div()
                            .id("revision-content")
                            .h(px(170.))
                            .flex_shrink_0()
                            .min_w(px(0.))
                            .overflow_y_scroll()
                            .border_1()
                            .child(
                                div()
                                    .w_full()
                                    .font_family("Menlo")
                                    .child(revision.text.clone()),
                            ),
                    );
            }
            panel = panel
                .child(format!(
                    "Compare: before {} · after {}",
                    self.compare_before
                        .map_or("none".into(), |id| id.to_string()),
                    self.compare_after
                        .map_or("none".into(), |id| id.to_string())
                ))
                .child(
                    Button::new("compare-revisions")
                        .label("Compare revisions")
                        .disabled(
                            !self.phase.can_submit()
                                || self.compare_before.is_none()
                                || self.compare_after.is_none(),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let (Some(before), Some(after)) =
                                (this.compare_before, this.compare_after)
                            {
                                this.submit(
                                    Action::CompareDraftRevisions {
                                        draft: draft_id,
                                        before,
                                        after,
                                    },
                                    "Compare revisions",
                                    cx,
                                );
                            }
                        })),
                );
            if let Some(diff) = &self.diff {
                panel = panel.child(
                    div()
                        .id("revision-diff")
                        .h(px(220.))
                        .flex_shrink_0()
                        .min_w(px(0.))
                        .overflow_y_scroll()
                        .border_1()
                        .child(div().w_full().font_family("Menlo").child(diff.clone())),
                );
            }
        }
        panel.into_any_element()
    }

    fn render_source_reader(&mut self, id: Uuid, cx: &mut Context<Self>) -> AnyElement {
        let p = self.palette();
        let Some(source) = self.sources.iter().find(|source| source.source_id == id) else {
            return div()
                .child("This source is no longer listed. Refresh the vault or close this document.")
                .into_any_element();
        };
        let source_id = source.source_id;
        let version = source.version_id;
        div()
            .flex()
            .flex_col()
            .gap_3()
            .min_w(px(0.))
            .child(
                div()
                    .font_family(tokens::READING_FONT)
                    .text_size(px(24.))
                    .child(source.title.clone()),
            )
            .child(div().text_color(color(p.muted)).child(format!(
                "Source · read only · Current at last observation · {} · {} bytes",
                approval_tag(source.approval),
                source.bytes.len()
            )))
            .child(div().text_color(color(p.muted)).child(format!(
                "Revision: {}",
                spaced_identifier(&version.to_string())
            )))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new(format!("approve-{source_id}"))
                            .label("Approve")
                            .disabled(!self.phase.can_submit())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.submit(
                                    Action::SetApproval {
                                        source: source_id,
                                        version,
                                        approval: Approval::Approved,
                                    },
                                    "Approve",
                                    cx,
                                );
                            })),
                    )
                    .child(
                        Button::new(format!("withdraw-{source_id}"))
                            .label("Withdraw")
                            .disabled(!self.phase.can_submit())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.submit(
                                    Action::SetApproval {
                                        source: source_id,
                                        version,
                                        approval: Approval::Withdrawn,
                                    },
                                    "Withdraw",
                                    cx,
                                );
                            })),
                    ),
            )
            .child(
                div()
                    .font_family(tokens::READING_FONT)
                    .text_size(px(15.))
                    .line_height(relative(1.6))
                    .child(String::from_utf8_lossy(&source.bytes).into_owned()),
            )
            .into_any_element()
    }

    pub(super) fn render_chat(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let mut body = div()
            .id("chat-transcript")
            .track_scroll(&self.chat_scroll)
            .vertical_scrollbar(&self.chat_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .gap_3()
            .p_3()
            .overflow_y_scroll();
        if self.history.is_empty() && self.search.is_none() && self.streamed_text.is_empty() {
            body = body.child(
                div()
                    .text_color(color(p.muted))
                    .child("Ask about approved sources. Answers, citations and saved conversations appear here."),
            );
        }
        for (i, turn) in self.history.iter().enumerate() {
            body = body.child(
                Button::new(format!("turn-{i}"))
                    .label(format!(
                        "{} · {} · {:?}",
                        compact_title(&turn.question),
                        turn.profile,
                        turn.status
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_turn = Some(i);
                        this.selected_saved_evidence = None;
                        cx.notify();
                    })),
            );
        }
        if let Some(turn) = self.selected_turn.and_then(|i| self.history.get(i)) {
            body = body.child(format!(
                "Question: {}\nAnswer: {}\nStatus: {:?}\nProvider turn: {}",
                turn.question,
                turn.answer.as_deref().unwrap_or("No answer saved"),
                turn.status,
                spaced_identifier(turn.provider_turn_id.as_deref().unwrap_or("none"))
            ));
            if let Ok(evidence) = serde_json::from_str::<Vec<Evidence>>(&turn.evidence_json) {
                for (i, hit) in evidence.iter().enumerate() {
                    let saved = hit.clone();
                    body = body.child(
                        Button::new(format!("saved-evidence-{i}"))
                            .label(saved_evidence_button_label(i, hit))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected_saved_evidence = Some(saved.clone());
                                cx.notify();
                            })),
                    );
                }
            }
            if turn.status == brn_workflow::worker::OperationStatus::Completed
                && turn.answer.is_some()
                && let Some(state) = &self.draft_state
            {
                let turn_id = turn.operation_id;
                let parent = self
                    .review
                    .as_ref()
                    .map(|r| r.id)
                    .unwrap_or(state.stamp().base_revision);
                body = body
                    .child(format!(
                        "Candidate target: {} · parent revision {} · answer {}",
                        state.title(),
                        parent,
                        turn_id
                    ))
                    .child(
                        Button::new("save-candidate")
                            .label("Save as candidate")
                            .disabled(!self.phase.can_submit())
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.save_candidate(turn_id, cx)),
                            ),
                    );
            }
        }
        if let Some(hit) = &self.selected_saved_evidence {
            body = body.child(format!("Saved evidence snapshot (may be historical) · source {} · revision {} · bytes {}..{}\n{}", hit.source_id, hit.version_id, hit.start_byte, hit.end_byte, hit.quote));
        }
        if let Some(search) = &self.search {
            body = body.child(format!("Search passages for: {}", search.query));
            for (i, hit) in search.evidence.iter().enumerate() {
                let evidence = hit.clone();
                body = body.child(
                    Button::new(format!("passage-{i}"))
                        .label(passage_button_label(i, hit, &self.sources))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selected_evidence = Some(evidence.clone());
                            cx.notify();
                        })),
                );
            }
        }
        if let Some(hit) = &self.selected_evidence {
            body = body.child(format!(
                "Selected passage · source {} · revision {} · bytes {}..{}\n{}",
                hit.source_id, hit.version_id, hit.start_byte, hit.end_byte, hit.quote
            ));
        }
        if self.phase.shows_live_answer() && !self.streamed_text.is_empty() {
            let heading = if matches!(self.phase, Phase::Cancelling { .. }) {
                "Partial answer while cancellation finishes (not saved)"
            } else {
                "Answer in progress (not saved)"
            };
            body = body.child(format!("{heading}:\n{}", self.streamed_text));
        }
        let composer = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_2()
            .p_2()
            .border_t_1()
            .border_color(color(p.line))
            .bg(color(p.panel))
            .child(
                Editor::new(&self.query)
                    .h(px(90.))
                    .flex_shrink_0()
                    .aria_label("Question for approved sources"),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("profile-keyword")
                            .label("Keyword")
                            .compact()
                            .selected(matches!(self.profile, Profile::Keyword))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.choose_profile(Profile::Keyword, cx)
                            })),
                    )
                    .child(
                        Button::new("profile-semantic")
                            .label("Semantic")
                            .compact()
                            .selected(matches!(self.profile, Profile::Semantic))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.choose_profile(Profile::Semantic, cx)
                            })),
                    )
                    .child(
                        Button::new("profile-hybrid")
                            .label("Hybrid")
                            .compact()
                            .selected(matches!(self.profile, Profile::Hybrid))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.choose_profile(Profile::Hybrid, cx)
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("search")
                            .label("Search")
                            .disabled(!self.phase.can_submit())
                            .on_click(cx.listener(|this, _, _, cx| this.search(cx))),
                    )
                    .child(
                        Button::new("ask")
                            .label("Ask from sources")
                            .disabled(!self.phase.can_submit())
                            .on_click(cx.listener(|this, _, _, cx| this.ask(cx))),
                    ),
            );
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(body)
            .child(composer)
    }
}
