//! Retained Inbox presentation over typed AppWorker commands.
use super::*;
use brn_workflow::{
    inbox::{CaptureInboxRequest, InboxAvailability, InboxItem, InboxKind, InboxOriginal},
    inbox_processing::{
        InboxConversionPreview, InboxProcessOutcome, InboxSourceRequest, MAX_PROCESS_BATCH,
    },
};
use gpui_kit::{
    AnyElement, TestSupportExt,
    base::Disableable,
    component::{Selectable, checkbox::Checkbox, input::Textarea},
};

pub(super) struct InboxPane {
    pub(super) title: Entity<TextareaState>,
    pub(super) body: Entity<EditorState>,
    pub(super) kind: InboxKind,
    pub(super) original: Entity<EditorState>,
    pub(super) preview: Entity<EditorState>,
    pub(super) source_title: Entity<TextareaState>,
    pub(super) source_path: Entity<TextareaState>,
    pub(super) analysis_source_path: Entity<TextareaState>,
    pub(super) analysis_id: Entity<TextareaState>,
    pub(super) analysis_source: Entity<EditorState>,
    pub(super) analysis_retained_source: Entity<EditorState>,
    pub(super) analysis_answer: Entity<EditorState>,
    /// Explicit guarded entry intention, consumed once without replacing later typing.
    pub(super) analysis_path_target: Option<String>,
    pub(super) checked: Vec<InboxItem>,
    pub(super) scroll: ScrollHandle,
    selection_error: Option<String>,
    preview_snapshot: Option<InboxConversionPreview>,
}
impl InboxPane {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Desktop>) -> Self {
        Self {
            title: cx.new(|cx| {
                TextareaState::new(window, cx)
                    .placeholder("Title for this exact original copy")
                    .auto_grow(1, 3)
            }),
            body: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            kind: InboxKind::Text,
            original: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            preview: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            source_title: cx.new(|cx| {
                TextareaState::new(window, cx)
                    .placeholder("Source proposal title")
                    .auto_grow(1, 3)
            }),
            source_path: cx.new(|cx| {
                TextareaState::new(window, cx)
                    .placeholder("New Source note vault-relative .md path")
                    .auto_grow(1, 3)
            }),
            analysis_source_path: cx.new(|cx| {
                TextareaState::new(window, cx)
                    .placeholder("Saved approved Source vault-relative path")
                    .auto_grow(1, 3)
            }),
            analysis_id: cx.new(|cx| {
                TextareaState::new(window, cx)
                    .placeholder("Retained analysis UUID")
                    .auto_grow(1, 3)
            }),
            analysis_source: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            analysis_retained_source: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            analysis_answer: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            analysis_path_target: None,
            checked: Vec::new(),
            scroll: ScrollHandle::new(),
            selection_error: None,
            preview_snapshot: None,
        }
    }
    fn capture_request(&self, cx: &App) -> CaptureInboxRequest {
        CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: self.kind,
            title: self.title.read(cx).value().to_string(),
            original_name: None,
            text: self.body.read(cx).value().to_string(),
        }
    }
}

fn kind_name(kind: InboxKind) -> &'static str {
    match kind {
        InboxKind::Text => "Text",
        InboxKind::Markdown => "Markdown",
        InboxKind::Email => "Email copy",
        InboxKind::Teams => "Teams copy",
    }
}
fn availability_name(availability: &InboxAvailability) -> &'static str {
    match availability {
        InboxAvailability::Available => "Original available",
        InboxAvailability::Missing => "Original missing",
        InboxAvailability::Changed => "Original changed",
        InboxAvailability::Unavailable => "Original unavailable",
    }
}
fn outcome_text(outcome: &InboxProcessOutcome) -> String {
    match outcome {
        InboxProcessOutcome::Queued => "Queued".into(),
        InboxProcessOutcome::Running => "Converting".into(),
        InboxProcessOutcome::Converted {
            format, byte_len, ..
        } => {
            format!("Converted · {format:?} · {byte_len} bytes")
        }
        InboxProcessOutcome::Failed { code } => format!("Failed · {code}"),
        InboxProcessOutcome::Cancelled => "Cancelled".into(),
        InboxProcessOutcome::Interrupted => "Interrupted".into(),
    }
}
fn readonly(editor: &Entity<EditorState>, label: &'static str) -> Editor {
    Editor::new(editor)
        .h_full()
        .readonly(true)
        .aria_label(label)
}
fn copy(id: &'static str, label: &'static str, text: String) -> Button {
    Button::new(id).label(label).on_click(move |_, _, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text.clone()));
    })
}

impl Desktop {
    /// Response synchronization never replaces retained capture or source typing.
    pub(super) fn sync_inbox_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let queue = &self.ai.as_ref().unwrap().inbox_queue;
        let original = queue
            .selected
            .as_ref()
            .and_then(|read| match &read.original {
                InboxOriginal::Available { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap_or_default();
        if self.inbox.original.read(cx).value().as_ref() != original {
            self.inbox
                .original
                .update(cx, |editor, cx| editor.set_value(original, window, cx));
        }
        let preview = queue
            .preview
            .as_ref()
            .map_or("", |preview| preview.markdown.as_str());
        if self.inbox.preview.read(cx).value().as_ref() != preview {
            self.inbox
                .preview
                .update(cx, |editor, cx| editor.set_value(preview, window, cx));
        }
        self.inbox.preview_snapshot = queue.preview.clone();
        self.sync_inbox_analysis_widgets(window, cx);
    }
    pub(super) fn inbox_blocked(&self) -> bool {
        !self.ai.as_ref().unwrap().ready
            || self.open_doc != Some(DocRef::Inbox)
            || self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || self.close_failed
    }
    fn capture_inbox_input(&mut self, cx: &mut Context<Self>) {
        if self.inbox_blocked() {
            return;
        }
        let request = self.inbox.capture_request(cx);
        if let Some(command) = self.ai.as_mut().unwrap().capture_inbox(request) {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn check_inbox_item(
        &mut self,
        item: InboxItem,
        checked: bool,
        cx: &mut Context<Self>,
    ) {
        if self.inbox_blocked() || self.ai.as_ref().unwrap().processing_pending() {
            return;
        }
        if !checked {
            self.inbox
                .checked
                .retain(|bound| bound.capture.id != item.capture.id);
            self.inbox.selection_error = None;
        } else if !self
            .ai
            .as_ref()
            .unwrap()
            .inbox_queue
            .page
            .as_ref()
            .is_some_and(|page| page.entries.iter().any(|entry| entry.item == item))
        {
            self.inbox.selection_error =
                Some("The displayed page changed. Select an item from the current page.".into());
        } else if self
            .inbox
            .checked
            .iter()
            .any(|bound| bound.capture.id == item.capture.id)
        {
            self.inbox.selection_error = Some("This original snapshot is already retained in the selection. Remove it explicitly before selecting another snapshot.".into());
        } else if self.inbox.checked.len() == MAX_PROCESS_BATCH {
            self.inbox.selection_error = Some(format!(
                "Choose at most {MAX_PROCESS_BATCH} exact originals for one batch."
            ));
        } else {
            self.inbox.checked.push(item);
            self.inbox.selection_error = None;
        }
        cx.notify();
    }
    pub(super) fn process_checked_inbox(&mut self, cx: &mut Context<Self>) {
        if self.inbox_blocked() {
            return;
        }
        // These are the checked complete snapshots, never a lookup by a later page ID.
        let items = self.inbox.checked.clone();
        if let Some(command) = self.ai.as_mut().unwrap().process_inbox_items(items) {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    fn prepare_inbox_source_input(&mut self, cx: &mut Context<Self>) {
        if self.inbox_blocked() {
            return;
        }
        let ai = self.ai.as_mut().unwrap();
        let Some(preview) = self
            .inbox
            .preview_snapshot
            .as_ref()
            .filter(|preview| ai.inbox_queue.preview.as_ref() == Some(*preview))
        else {
            ai.inbox_queue.source_error = Some("Inspect the current complete converted preview before preparing its Source proposal.".into());
            cx.notify();
            return;
        };
        let request = InboxSourceRequest {
            candidate: preview.request.clone(),
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: self.inbox.source_path.read(cx).value().to_string(),
            title: self.inbox.source_title.read(cx).value().to_string(),
        };
        if let Some(command) = ai.prepare_inbox_source(request) {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn render_inbox(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let ai = self.ai.as_ref().unwrap();
        let queue = &ai.inbox_queue;
        let blocked = self.inbox_blocked();
        let mut content = div().id("inbox-content").track_scroll(&self.inbox.scroll)
            .flex().flex_col().flex_1().min_h(px(0.)).overflow_y_scroll().p_3().gap_2()
            .child("Inbox")
            .child(self.render_inbox_analysis(cx))
            .child("Keep an exact UTF-8 text, Markdown, email or Teams copy. Originals stay retained. Conversion previews and Source proposals require review; knowledge changes only after exact approval.")
            .child(Textarea::new(&self.inbox.title).disabled(blocked).aria_label("Retained exact Inbox capture title"));
        let mut kinds = div().flex().flex_wrap().gap_1();
        for kind in [
            InboxKind::Text,
            InboxKind::Markdown,
            InboxKind::Email,
            InboxKind::Teams,
        ] {
            kinds = kinds.child(
                Button::new(format!("inbox-kind-{kind:?}"))
                    .label(kind_name(kind))
                    .selected(self.inbox.kind == kind)
                    .disabled(blocked)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.inbox_blocked() {
                            this.inbox.kind = kind;
                            cx.notify();
                        }
                    })),
            );
        }
        content = content
            .child(kinds)
            .child(
                div().h(px(240.)).flex_shrink_0().child(
                    Editor::new(&self.inbox.body)
                        .h_full()
                        .disabled(blocked)
                        .aria_label("Retained exact Inbox capture text"),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(
                        Button::new("capture-inbox-original")
                            .label("Capture exact original")
                            .disabled(blocked || ai.capture_pending())
                            .on_click(cx.listener(|this, _, _, cx| this.capture_inbox_input(cx))),
                    )
                    .child(
                        Button::new("retry-inbox-capture")
                            .label("Retry exact submitted capture")
                            .disabled(
                                blocked || ai.capture_pending() || queue.capture_error.is_none(),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.inbox_blocked() {
                                    return;
                                }
                                if let Some(command) = this.ai.as_mut().unwrap().retry_capture() {
                                    this.simple_send(command, cx);
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("copy-inbox-capture-text")
                            .label("Copy full capture text")
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                    this.inbox.body.read(cx).value().to_string(),
                                ));
                            })),
                    ),
            );
        if ai.capture_pending() {
            content = content
                .child("Retaining the exact submitted capture. Later typing stays in this form.");
        }
        if let Some(item) = &queue.capture_result {
            let id = item.capture.id;
            content = content
                .child(format!(
                    "Captured {} · {} · {} bytes",
                    item.capture.title, id, item.capture.copy.byte_len
                ))
                .child(
                    Button::new("inspect-captured-inbox-original")
                        .label("Inspect captured original")
                        .disabled(blocked || ai.inbox_loading())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if this.inbox_blocked() {
                                return;
                            }
                            if let Some(command) = this.ai.as_mut().unwrap().select_inbox(id) {
                                this.simple_send(command, cx);
                            }
                            cx.notify();
                        })),
                );
        }
        if let Some(error) = &queue.capture_error {
            content = content.child(error.clone());
        }
        content = content.child("FIFO originals").child(
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .child(
                    Button::new("refresh-inbox-inventory")
                        .label("Refresh first page")
                        .disabled(blocked || ai.inbox_loading())
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.inbox_blocked() {
                                return;
                            }
                            if let Some(command) = this.ai.as_mut().unwrap().refresh_inbox() {
                                this.simple_send(command, cx);
                            }
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("next-inbox-inventory")
                        .label("Next page")
                        .disabled(
                            blocked
                                || ai.inbox_loading()
                                || queue
                                    .page
                                    .as_ref()
                                    .is_none_or(|page| page.next_after.is_none()),
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.inbox_blocked() {
                                return;
                            }
                            if let Some(command) = this.ai.as_mut().unwrap().next_inbox_page() {
                                this.simple_send(command, cx);
                            }
                            cx.notify();
                        })),
                ),
        );
        if ai.inbox_loading() {
            content = content.child("Loading current Inbox inventory or original…");
        }
        if let Some(page) = &queue.page {
            content = content.child(format!(
                "{} retained originals · {} on this page",
                page.total_count,
                page.entries.len()
            ));
            for (index, entry) in page.entries.iter().enumerate() {
                let item = entry.item.clone();
                let inspect_id = item.capture.id;
                let checked = self.inbox.checked.iter().any(|bound| bound == &item);
                content = content.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(format!(
                            "{} · {} · received {} · {}",
                            item.capture.title,
                            kind_name(item.capture.kind),
                            item.received_at_ms,
                            availability_name(&entry.availability)
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_1()
                                .child(
                                    Checkbox::new(format!("inbox-check-{index}"))
                                        .label("Include exact original in batch")
                                        .checked(checked)
                                        .disabled(
                                            blocked
                                                || ai.processing_pending()
                                                || (!checked
                                                    && self.inbox.checked.len()
                                                        >= MAX_PROCESS_BATCH),
                                        )
                                        .on_change(cx.listener(move |this, checked, _, cx| {
                                            this.check_inbox_item(item.clone(), *checked, cx)
                                        })),
                                )
                                .child(
                                    Button::new(format!("inspect-inbox-original-{index}"))
                                        .label("Inspect full original")
                                        .disabled(blocked || ai.inbox_loading())
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if this.inbox_blocked() {
                                                return;
                                            }
                                            if let Some(command) =
                                                this.ai.as_mut().unwrap().select_inbox(inspect_id)
                                            {
                                                this.simple_send(command, cx);
                                            }
                                            cx.notify();
                                        })),
                                ),
                        ),
                );
            }
            for issue in &page.issues {
                content = content.child(format!(
                    "Inbox issue {:?}: {}",
                    issue.item_id, issue.message
                ));
            }
            if page.issues_truncated {
                content = content
                    .child("Additional Inbox issues were reported beyond this page's issue limit.");
            }
        }
        content = content.child(format!(
            "{} exact original snapshots checked · choose 1–{MAX_PROCESS_BATCH}",
            self.inbox.checked.len()
        ));
        for (index, item) in self.inbox.checked.iter().enumerate() {
            let id = item.capture.id;
            content = content.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(format!(
                        "Checked: {} · {} · received {}",
                        item.capture.title, id, item.received_at_ms
                    ))
                    .child(
                        Button::new(format!("remove-inbox-checked-{index}"))
                            .label("Remove from batch selection")
                            .disabled(blocked || ai.processing_pending())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.inbox_blocked()
                                    || this.ai.as_ref().unwrap().processing_pending()
                                {
                                    return;
                                }
                                this.inbox.checked.retain(|item| item.capture.id != id);
                                this.inbox.selection_error = None;
                                cx.notify();
                            })),
                    ),
            );
        }
        if let Some(error) = &self.inbox.selection_error {
            content = content.child(error.clone());
        }
        content = content.child(
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .child(
                    Button::new("process-checked-inbox")
                        .label("Process checked originals")
                        .disabled(
                            blocked || ai.processing_pending() || self.inbox.checked.is_empty(),
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.process_checked_inbox(cx))),
                )
                .child(
                    Button::new("retry-inbox-processing")
                        .label("Retry exact submitted batch")
                        .disabled(blocked || !ai.can_retry_process())
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.inbox_blocked() {
                                return;
                            }
                            if let Some(command) = this.ai.as_mut().unwrap().retry_process() {
                                this.simple_send(command, cx);
                            }
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("cancel-inbox-processing")
                        .label("Cancel remaining conversions")
                        .disabled(blocked || !ai.processing_pending())
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.inbox_blocked() {
                                return;
                            }
                            if let Some(command) = this.ai.as_mut().unwrap().cancel_inbox_batch() {
                                this.simple_send(command, cx);
                            }
                            cx.notify();
                        })),
                ),
        );
        if let Some(read) = &queue.selected {
            content = content.child(format!(
                "Inspected original: {} · {}",
                read.item.capture.title, read.item.capture.id
            ));
            match &read.original {
                InboxOriginal::Available { text } => {
                    content = content
                        .child(div().h(px(240.)).flex_shrink_0().child(readonly(
                            &self.inbox.original,
                            "Complete exact retained Inbox original",
                        )))
                        .child(copy(
                            "copy-inbox-original",
                            "Copy full exact original",
                            text.clone(),
                        ));
                }
                InboxOriginal::Missing => {
                    content = content.child("The workflow reports this original is missing.")
                }
                InboxOriginal::Changed { reason } => {
                    content = content.child(format!(
                        "The workflow reports this original changed: {reason}"
                    ))
                }
                InboxOriginal::Unavailable { reason } => {
                    content = content.child(format!(
                        "The workflow reports this original is unavailable: {reason}"
                    ))
                }
            }
        }
        if let Some(batch) = &queue.batch {
            content = content.child(format!(
                "Batch {} · {} remaining conversions",
                batch.request.id,
                batch.pending_count()
            ));
            for (index, (item, entry)) in batch.request.items.iter().zip(&batch.entries).enumerate()
            {
                let batch_id = batch.request.id;
                content = content
                    .child(format!(
                        "{} · {} · {}",
                        index + 1,
                        item.capture.title,
                        outcome_text(&entry.outcome)
                    ))
                    .child(
                        Button::new(format!("preview-inbox-conversion-{index}"))
                            .label("Inspect full converted preview")
                            .disabled(
                                blocked
                                    || !matches!(
                                        entry.outcome,
                                        InboxProcessOutcome::Converted { .. }
                                    ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.inbox_blocked()
                                    || this
                                        .ai
                                        .as_ref()
                                        .unwrap()
                                        .inbox_queue
                                        .batch
                                        .as_ref()
                                        .is_none_or(|batch| batch.request.id != batch_id)
                                {
                                    return;
                                }
                                if let Some(command) =
                                    this.ai.as_mut().unwrap().preview_inbox_candidate(index)
                                {
                                    this.simple_send(command, cx);
                                }
                                cx.notify();
                            })),
                    );
            }
        }
        if let Some(error) = &queue.error {
            content = content.child(error.clone());
        }
        if let Some(preview) = &queue.preview {
            content = content
                .child(format!(
                    "Converted preview: {} · {:?} · batch {} entry {}",
                    preview.original.capture.title,
                    preview.format,
                    preview.request.batch_id,
                    preview.request.index + 1
                ))
                .child(div().h(px(300.)).flex_shrink_0().child(readonly(
                    &self.inbox.preview,
                    "Complete exact converted Inbox preview",
                )))
                .child(copy(
                    "copy-inbox-preview",
                    "Copy full converted preview",
                    preview.markdown.clone(),
                ));
            if preview.needs_semantic_review {
                content = content.child("The conversion still needs semantic review. Attachments and visual completeness are not established by this text copy.");
            }
        }
        content = content
            .child("Prepare a Source proposal from the inspected conversion")
            .child(
                Textarea::new(&self.inbox.source_title)
                    .disabled(blocked)
                    .aria_label("Retained exact Inbox Source proposal title"),
            )
            .child(
                Textarea::new(&self.inbox.source_path)
                    .disabled(blocked)
                    .aria_label("Retained Inbox Source destination path"),
            )
            .child(
                Button::new("prepare-inbox-source")
                    .label("Prepare Source review input")
                    .disabled(
                        blocked
                            || !ai.vault_bound
                            || ai.source_pending()
                            || queue.preview.is_none(),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.prepare_inbox_source_input(cx))),
            );
        if ai.source_pending() {
            content = content.child("Preparing the exact Source review input. Later title and path typing stays retained here.");
        }
        if let Some(prepared) = &queue.prepared {
            content = content
                .child(format!(
                    "Retained prepared Source proposal {} · {}",
                    prepared.id, prepared.title
                ))
                .child(
                    Button::new("open-inbox-source-form")
                        .label("Open retained Source proposal form")
                        .disabled(blocked)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if !this.inbox_blocked() {
                                this.simple_leave(simple::EditorTransition::InboxSourceDraft, cx);
                            }
                        })),
                );
        }
        if let Some(error) = &queue.source_error {
            content = content.child(error.clone());
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(content.test_support())
            .into_any_element()
    }
}
