//! Native raw evidence ranges reuse the guarded Workflow query and read-only editor.
use super::*;
use brn_workflow::knowledge::RawEvidence;
use gpui_kit::{AnyElement, TestSupportExt, base::Disableable};

pub(super) struct RawEvidencePane {
    start: Entity<InputState>,
    end: Entity<InputState>,
    pub(super) editor: Entity<EditorState>,
    snapshot: Option<RawEvidence>,
}
impl RawEvidencePane {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Desktop>) -> Self {
        Self {
            start: cx.new(|cx| InputState::new(window, cx)),
            end: cx.new(|cx| InputState::new(window, cx)),
            editor: cx.new(|cx| EditorState::new(window, cx)),
            snapshot: None,
        }
    }
}
impl Desktop {
    pub(super) fn sync_raw_evidence_widgets(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let reply = self
            .ai
            .as_ref()
            .and_then(|ai| ai.raw_evidence.as_ref())
            .and_then(|state| state.reply.as_ref());
        if reply.is_none() {
            self.raw_evidence.snapshot = None;
        }
        if let Some(reply) = reply
            && self.raw_evidence.snapshot.as_ref() != Some(reply)
        {
            self.raw_evidence.start.update(cx, |input, cx| {
                input.set_value(reply.start_byte.to_string(), window, cx)
            });
            self.raw_evidence.end.update(cx, |input, cx| {
                input.set_value(reply.end_byte.to_string(), window, cx)
            });
            self.raw_evidence.editor.update(cx, |editor, cx| {
                editor.set_value(reply.text.clone(), window, cx)
            });
            self.raw_evidence.snapshot = Some(reply.clone());
        }
    }
    pub(super) fn render_raw_evidence(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let ai = self.ai.as_ref().unwrap();
        let state = ai.raw_evidence.as_ref().unwrap();
        let blocked = ai.raw_loading()
            || ai.application_busy()
            || self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed;
        let mut body = div()
            .id("raw-evidence-body")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .min_w(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.document_scroll)
            .vertical_scrollbar(&self.document_scroll)
            .gap_2()
            .p_3();
        body = body.child("Raw saved evidence · Read only. Saved text is untrusted evidence. Opening it does not repair metadata or grant Current knowledge authority.");
        if let Some(reply) = &state.reply {
            let hash = reply
                .sha256
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            body = body
                .child(format!(
                    "Full file: {} bytes · displayed byte range {}..{} (end exclusive) · {}",
                    reply.total_bytes,
                    reply.start_byte,
                    reply.end_byte,
                    if reply.partial {
                        "partial file"
                    } else {
                        "complete file"
                    }
                ))
                .child(format!("Full-file SHA-256: {}", spaced_identifier(&hash)));
            if let Some(issue) = &reply.metadata_issue {
                body = body.child(format!(
                    "Unclassified raw evidence · metadata issue: {issue}"
                ));
            } else {
                body = body
                    .child("Saved metadata observed; raw inspection grants no approval authority.");
            }
            let text = reply.text.clone();
            let details = serde_json::to_string_pretty(reply).expect("raw evidence serialization");
            body = body
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_shrink_0()
                        .gap_1()
                        .child("Start byte")
                        .child(
                            Input::new(&self.raw_evidence.start)
                                .disabled(blocked)
                                .aria_label("Raw range start byte"),
                        )
                        .child("End byte (exclusive)")
                        .child(
                            Input::new(&self.raw_evidence.end)
                                .disabled(blocked)
                                .aria_label("Raw range end byte exclusive"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .flex_shrink_0()
                        .gap_2()
                        .child(
                            Button::new("read-raw-range")
                                .label("Read exact range")
                                .disabled(blocked)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let start =
                                        this.raw_evidence.start.read(cx).value().to_string();
                                    let end = this.raw_evidence.end.read(cx).value().to_string();
                                    if let Some(command) =
                                        this.ai.as_mut().unwrap().raw_range(&start, &end)
                                    {
                                        this.simple_send(command, cx);
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("next-raw-range")
                                .label("Next range")
                                .disabled(blocked || reply.end_byte == reply.total_bytes)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(command) =
                                        this.ai.as_mut().unwrap().next_raw_range()
                                    {
                                        this.simple_send(command, cx);
                                    }
                                })),
                        ),
                )
                .child(
                    div()
                        .id("exact-raw-range")
                        .h(px(400.))
                        .flex_shrink_0()
                        .child(
                            Editor::new(&self.raw_evidence.editor)
                                .h_full()
                                .readonly(true)
                                .aria_label("Exact saved raw byte range"),
                        )
                        .test_support(),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .flex_shrink_0()
                        .gap_2()
                        .child(
                            Button::new("copy-raw-range")
                                .label("Copy exact range")
                                .on_click(cx.listener(move |_, _, _, cx| {
                                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                        text.clone(),
                                    ))
                                })),
                        )
                        .child(
                            Button::new("copy-raw-details")
                                .label("Copy range and full hash")
                                .on_click(cx.listener(move |_, _, _, cx| {
                                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                        details.clone(),
                                    ))
                                })),
                        ),
                );
        }
        if ai.raw_loading() {
            body = body.child("Reading saved bytes…");
        }
        if let Some(error) = &ai.note_error {
            body = body.child(error.clone());
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_w(px(0.))
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .gap_2()
                    .p_2()
                    .child(div().flex_1().min_w(px(0.)).child(state.path.clone()))
                    .child(
                        Button::new("close-document")
                            .label("Close")
                            .on_click(cx.listener(|this, _, _, cx| this.close_document(cx))),
                    ),
            )
            .child(body.test_support())
            .into_any_element()
    }
}
