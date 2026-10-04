//! Retained initial proposal input; source capture and creation go through AppWorker.
use super::*;
use crate::draft::DraftKind;
use gpui_kit::{
    AnyElement,
    base::Disableable,
    component::{Selectable, WindowExt, input::Textarea},
};

pub(super) fn path_state(window: &mut Window, cx: &mut Context<TextareaState>) -> TextareaState {
    TextareaState::new(window, cx)
        .placeholder("Vault-relative .md path")
        .auto_grow(1, 3)
}

pub(super) fn body_state(window: &mut Window, cx: &mut Context<EditorState>) -> EditorState {
    EditorState::new(window, cx)
        .language("markdown")
        .default_value("")
}

impl Desktop {
    /// Bind each new form once. Creation/source ACKs never replace later widget input.
    pub(super) fn sync_draft_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &self.ai.as_ref().unwrap().draft else {
            self.draft_widget_id = None;
            return;
        };
        if self.draft_widget_id == Some(form.id) {
            return;
        }
        let (id, title, path, text) = (
            form.id,
            form.title.clone(),
            form.path.clone(),
            form.text.clone(),
        );
        self.draft_title
            .update(cx, |input, cx| input.set_value(title, window, cx));
        self.draft_path
            .update(cx, |input, cx| input.set_value(path, window, cx));
        self.draft_editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        self.draft_widget_id = Some(id);
        self.draft_scroll.set_offset(point(px(0.), px(0.)));
    }

    pub(super) fn capture_draft_widgets(&mut self, cx: &App) {
        if let Some(form) = &mut self.ai.as_mut().unwrap().draft
            && self.draft_widget_id == Some(form.id)
        {
            form.edit(
                self.draft_title.read(cx).value().to_string(),
                self.draft_path.read(cx).value().to_string(),
                self.draft_editor.read(cx).value().to_string(),
                form.kind,
            );
        }
    }

    fn draft_command_blocked(&self) -> bool {
        let ai = self.ai.as_ref().unwrap();
        !ai.ready
            || !ai.vault_bound
            || ai.application_busy()
            || ai.active.is_some()
            || ai.rewrite.is_some()
            || self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || self.close_failed
    }

    fn discard_draft_dialog(
        &mut self,
        body_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.closing.is_some()
            || self.closed
            || self.close_failed
            || window.has_active_dialog(cx)
        {
            return;
        }
        self.capture_draft_widgets(cx);
        let Some(form) = self
            .ai
            .as_ref()
            .unwrap()
            .draft
            .as_ref()
            .filter(|form| !form.pending)
        else {
            return;
        };
        if body_only && (form.kind != DraftKind::Trash || form.text.is_empty()) {
            return;
        }
        let captured = (
            form.id,
            form.generation,
            form.title.clone(),
            form.path.clone(),
            form.text.clone(),
            form.kind,
        );
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let desktop = desktop.clone();
            let captured = captured.clone();
            dialog.title(if body_only { "Discard retained proposed body?" } else { "Discard retained initial form?" }).w(px(640.))
                .child("Copy the full text first if needed. This discards local input and does not change vault Markdown or an already-created proposal.")
                .child(Button::new("confirm-discard-initial-form").label(if body_only { "Confirm discard retained body" } else { "Confirm discard local form" })
                    .on_click(move |_, window, cx| {
                        let mut discarded = false;
                        let _ = desktop.update(cx, |this, cx| {
                            this.capture_draft_widgets(cx);
                            let unchanged = this.ai.as_ref().unwrap().draft.as_ref().is_some_and(|form| {
                                !form.pending && (form.id, form.generation, &form.title, &form.path, &form.text, form.kind)
                                    == (captured.0, captured.1, &captured.2, &captured.3, &captured.4, captured.5)
                            });
                            if unchanged && this.closing.is_none() && !this.closed && !this.close_failed {
                                if body_only {
                                    let form = this.ai.as_mut().unwrap().draft.as_mut().unwrap();
                                    form.edit(form.title.clone(), form.path.clone(), String::new(), form.kind);
                                    this.draft_editor.update(cx, |editor, cx| editor.set_value("", window, cx));
                                    discarded = true;
                                } else {
                                    discarded = this.ai.as_mut().unwrap().discard_draft();
                                    if discarded { this.draft_widget_id = None; }
                                }
                            }
                            if !discarded {
                                this.ai.as_mut().unwrap().notice = "The form changed while confirmation was open; full input is retained.".into();
                            }
                            cx.notify();
                        });
                        if discarded { window.close_dialog(cx); }
                    }))
        });
    }

    pub(super) fn render_draft(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let ai = self.ai.as_ref().unwrap();
        let mut body = div().id("initial-full-proposal-form")
            .track_scroll(&self.draft_scroll).flex().flex_col().flex_1()
            .min_h(px(0.)).overflow_y_scroll().gap_2().p_3()
            .child("New full note proposal")
            .child("Create saves review work. Vault Markdown changes only after exact proposal approval. Unsubmitted or later input stays in this form until explicitly discarded.");
        let Some(form) = &ai.draft else {
            return body.child("No retained initial form. Choose New proposal or Review as new note on a completed answer.").into_any_element();
        };
        if self.draft_widget_id != Some(form.id) {
            return body
                .child("Loading the retained full form…")
                .into_any_element();
        }
        let leaving = self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || self.close_failed;
        // Own pending creation still permits local typing; its exact submitted generation is frozen.
        let editable = !leaving && (!ai.application_busy() || form.pending);
        let command_blocked = self.draft_command_blocked();
        body = body
            .child(format!(
                "Proposal {} · input generation {}{}",
                form.id,
                form.generation,
                if form.pending {
                    " · creation awaiting acknowledgement"
                } else {
                    ""
                }
            ))
            .child(
                Textarea::new(&self.draft_title)
                    .disabled(!editable)
                    .aria_label("Exact full proposal title"),
            )
            .child(
                Textarea::new(&self.draft_path)
                    .disabled(!editable)
                    .aria_label("Exact scalar vault-relative Markdown path"),
            );
        let mut kinds = div().flex().flex_wrap().gap_1();
        for (kind, label) in [
            (DraftKind::Create, "Create"),
            (DraftKind::Replace, "Replace"),
            (DraftKind::Trash, "Trash"),
        ] {
            kinds = kinds.child(
                Button::new(format!("initial-kind-{kind:?}"))
                    .label(label)
                    .selected(form.kind == kind)
                    .disabled(!editable)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.capture_draft_widgets(cx);
                        if let Some(form) = &mut this.ai.as_mut().unwrap().draft {
                            form.edit(
                                form.title.clone(),
                                form.path.clone(),
                                form.text.clone(),
                                kind,
                            );
                        }
                        cx.notify();
                    })),
            );
        }
        body = body
            .child(kinds)
            .child(format!(
                "Full retained proposed body · {} UTF-8 bytes",
                form.text.len()
            ))
            .child(
                div().h(px(320.)).child(
                    Editor::new(&self.draft_editor)
                        .h_full()
                        .disabled(!editable)
                        .aria_label("Exact full initial proposal body"),
                ),
            );
        if form.kind != DraftKind::Create {
            body = body.child(
                Button::new("load-initial-proposal-source")
                    .label("Load exact existing note…")
                    .disabled(command_blocked)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.draft_command_blocked() {
                            return;
                        }
                        this.capture_draft_widgets(cx);
                        if let Some(command) = this.ai.as_mut().unwrap().draft_source() {
                            this.simple_send(command, cx);
                        }
                    })),
            );
            if form.source_operation.is_some() {
                body = body.child("Loading the identified full note and trusted fingerprint…");
            }
            if let Some(error) = &form.source_error {
                body = body.child(error.clone());
            }
            if let Some(source) = &form.source {
                let hash: String = source
                    .source
                    .fingerprint
                    .sha256
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                body = body
                    .child(format!(
                        "Captured source {} · {} bytes · SHA-256 {hash}",
                        source.source.path, source.source.fingerprint.len
                    ))
                    .child(
                        div()
                            .id("initial-captured-source")
                            .max_h(px(280.))
                            .overflow_y_scroll()
                            .child(source.text.clone()),
                    );
                if form.kind == DraftKind::Replace {
                    body = body.child(
                        Button::new("use-initial-captured-text")
                            .label("Use captured text as proposed body")
                            .disabled(!editable)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.capture_draft_widgets(cx);
                                if let Some(form) = &mut this.ai.as_mut().unwrap().draft
                                    && form.kind == DraftKind::Replace
                                    && let Some(source) = &form.source
                                {
                                    let text = source.text.clone();
                                    form.edit(
                                        form.title.clone(),
                                        form.path.clone(),
                                        text.clone(),
                                        form.kind,
                                    );
                                    this.draft_editor.update(cx, |editor, cx| {
                                        editor.set_value(text, window, cx)
                                    });
                                }
                                cx.notify();
                            })),
                    );
                }
            }
        }
        if form.kind == DraftKind::Trash {
            body = body.child("Trash proposes moving the captured original. Any retained proposed body must be copied or explicitly discarded first.");
            if !form.text.is_empty() {
                body = body.child(
                    Button::new("discard-initial-trash-body")
                        .label("Discard retained proposed body…")
                        .disabled(
                            form.pending
                                || self.closing.is_some()
                                || self.closed
                                || self.close_failed,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.discard_draft_dialog(true, window, cx)
                        })),
                );
            }
        }
        if let Some(error) = &form.error {
            body = body.child(error.clone());
        }
        if let Some((generation, result)) = &form.result {
            let proposal = result.draft.id;
            body = body.child(format!("Worker returned proposal {proposal} · review {} · submitted generation {generation}", result.version))
                .child(Button::new("open-created-current-proposal").label("Open current proposal").disabled(!form.can_leave() || leaving)
                    .on_click(cx.listener(move |this, _, _, cx| this.simple_leave(simple::EditorTransition::Review(proposal), cx))));
        }
        body = body.child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(
                    Button::new("create-initial-review-draft")
                        .label("Create review draft")
                        .disabled(
                            command_blocked
                                || form.result.is_some()
                                || form.source_operation.is_some(),
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.draft_command_blocked() {
                                return;
                            }
                            this.capture_draft_widgets(cx);
                            if let Some(command) = this.ai.as_mut().unwrap().create_draft() {
                                this.simple_send(command, cx);
                            }
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("copy-initial-full-body")
                        .label("Copy full body")
                        .on_click(cx.listener(|this, _, _, cx| {
                            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                this.draft_editor.read(cx).value().to_string(),
                            ))
                        })),
                )
                .child(
                    Button::new("copy-initial-full-title")
                        .label("Copy full title")
                        .on_click(cx.listener(|this, _, _, cx| {
                            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                this.draft_title.read(cx).value().to_string(),
                            ))
                        })),
                )
                .child(
                    Button::new("copy-initial-full-path")
                        .label("Copy full path")
                        .on_click(cx.listener(|this, _, _, cx| {
                            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                this.draft_path.read(cx).value().to_string(),
                            ))
                        })),
                )
                .child(
                    Button::new("discard-initial-full-form")
                        .label("Discard local form…")
                        .disabled(
                            form.pending
                                || self.closing.is_some()
                                || self.closed
                                || self.close_failed,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.discard_draft_dialog(false, window, cx)
                        })),
                ),
        );
        if form.submitted.is_some() || form.result.is_some() {
            body = body.child(
                Button::new("separate-initial-proposal")
                    .label("Start separate proposal from retained input")
                    .disabled(
                        form.pending || self.closing.is_some() || self.closed || self.close_failed,
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.capture_draft_widgets(cx);
                        if this.ai.as_mut().unwrap().separate_draft() {
                            this.simple_transition = None;
                            this.sync_draft_widgets(window, cx);
                        }
                        cx.notify();
                    })),
            );
        }
        if self.simple_transition.is_some() && self.closing.is_none() {
            body = body.child(
                Button::new("keep-initial-form-open")
                    .label("Keep form open")
                    .disabled(self.closed || self.close_failed)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.simple_transition = None;
                        cx.notify();
                    })),
            );
        }
        body.into_any_element()
    }
}

#[cfg(all(test, feature = "native-test-support"))]
mod tests {
    use super::*;
    use gpui_kit::{AppContext, EntityInputHandler};

    #[gpui_kit::test]
    fn actual_initial_widgets_preserve_full_title_path_body_and_oversized_input(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        const TITLE: &str = "\u{feff}日本語 🧭\r\nTitle λ\r";
        const PATH: &str = "café/資料\r\n🧭.md";
        const TEXT: &str = "\u{feff}# 正文\r\nΕλληνικά 🦀\n";
        cx.update(gpui_kit::component::init);
        let window = cx.add_window(|_, _| gpui_kit::Empty);
        cx.update_window(window.into(), |_, window, cx| {
            let title =
                cx.new(|cx| super::super::review_title_state(window, cx).default_value(TITLE));
            let path = cx.new(|cx| path_state(window, cx).default_value(PATH));
            let body = cx.new(|cx| body_state(window, cx).default_value(TEXT));
            assert_eq!(title.read(cx).value().as_ref(), TITLE);
            assert_eq!(path.read(cx).value().as_ref(), PATH);
            assert_eq!(body.read(cx).value().as_ref(), TEXT);
            title.update(cx, |title, cx| {
                title.replace_text_in_range(
                    Some(TITLE.encode_utf16().count()..TITLE.encode_utf16().count()),
                    "追加\r\n",
                    window,
                    cx,
                )
            });
            path.update(cx, |path, cx| {
                path.replace_text_in_range(Some(0..0), "新\n", window, cx)
            });
            body.update(cx, |body, cx| {
                body.replace_text_in_range(
                    Some(TEXT.encode_utf16().count()..TEXT.encode_utf16().count()),
                    "café λ\r\n",
                    window,
                    cx,
                )
            });
            let mut form = crate::draft::DraftForm::new(None).unwrap();
            form.edit(
                title.read(cx).value().to_string(),
                path.read(cx).value().to_string(),
                body.read(cx).value().to_string(),
                DraftKind::Create,
            );
            assert_eq!(form.title, format!("{TITLE}追加\r\n"));
            assert_eq!(form.path, format!("新\n{PATH}"));
            assert_eq!(form.text, format!("{TEXT}café λ\r\n"));
            assert_eq!(
                form.request().unwrap().changes[0],
                brn_workflow::proposals::DraftNoteChange::Create {
                    path: form.path.clone(),
                    text: form.text.clone()
                }
            );
            let oversized = "λ".repeat(brn_workflow::MAX_NOTE_BYTES / 2 + 1);
            body.update(cx, |body, cx| body.set_value(oversized.clone(), window, cx));
            form.edit(
                form.title.clone(),
                form.path.clone(),
                body.read(cx).value().to_string(),
                form.kind,
            );
            assert_eq!(form.text.as_bytes(), oversized.as_bytes());
            assert_eq!(form.text.len(), brn_workflow::MAX_NOTE_BYTES + 2);
            assert!(form.request().is_err());
            assert!(!form.can_leave());
        })
        .unwrap();
    }
}
