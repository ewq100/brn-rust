//! Captured owner confirmation only; AppWorker owns preservation and copy effects.
use super::*;
use crate::ai::{InboxCopyConfirmation, InboxCopyRequest};
use gpui_kit::{AnyElement, TestSupportExt, base::Disableable, component::WindowExt};

fn complete_json(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|_| {
        "The complete copy proof could not be displayed. No command was admitted.".into()
    })
}
fn copy_text(id: &'static str, label: &'static str, text: String) -> Button {
    Button::new(id).label(label).on_click(move |_, _, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text.clone()));
    })
}
fn readonly(editor: &Entity<EditorState>, label: &'static str) -> Editor {
    Editor::new(editor)
        .h_full()
        .readonly(true)
        .aria_label(label)
}
fn confirmation_json(capture: &InboxCopyConfirmation) -> Result<String, String> {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    enum Request<'a> {
        Remove(&'a brn_workflow::inbox_original_operations::RemoveInboxOriginalRequest),
        Restore(&'a brn_workflow::inbox_original_operations::RestoreInboxOriginalRequest),
    }
    #[derive(serde::Serialize)]
    struct Proof<'a> {
        item: &'a brn_workflow::inbox::InboxItem,
        request: Request<'a>,
        preservation_preview: Option<&'a brn_workflow::inbox_removal::InboxRemovalPreview>,
        removal: Option<&'a brn_workflow::inbox_original_operations::InboxOriginalOperation>,
    }
    complete_json(&Proof {
        item: capture.item(),
        request: match capture.request() {
            InboxCopyRequest::Remove(request) => Request::Remove(request),
            InboxCopyRequest::Restore(request) => Request::Restore(request),
        },
        preservation_preview: capture.preview(),
        removal: capture.removal(),
    })
}
impl Desktop {
    pub(super) fn sync_inbox_copy_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self
            .ai
            .as_ref()
            .unwrap()
            .inbox_copy
            .preview
            .as_ref()
            .and_then(|preview| preview.evidence.source.as_ref())
            .map_or("", |source| source.saved.text.as_str());
        if self.inbox.copy_source.read(cx).value().as_ref() != text {
            self.inbox
                .copy_source
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
        }
    }
    pub(super) fn render_inbox_copy(&self, cx: &mut Context<Self>) -> AnyElement {
        let ai = self.ai.as_ref().unwrap();
        let state = &ai.inbox_copy;
        let blocked = self.inbox_blocked();
        let busy = ai.inbox_copy_loading() || ai.inbox_copy_pending();
        let mut panel = div().id("inbox-copy-panel").test_support().flex().flex_col().gap_2()
            .child("Recoverable original-copy cleanup")
            .child("An approved Source must still preserve this exact original. Cleanup requires your explicit confirmation and retains a recoverable copy. Inbox disposition and semantic reviews are separate.")
            .child(div().flex().flex_wrap().gap_1()
                .child(Button::new("inspect-inbox-copy").label("Inspect copy cleanup")
                    .disabled(blocked || busy || ai.inbox_queue.selected.is_none())
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.inbox_blocked() || window.has_active_dialog(cx) { return; }
                        let commands = this.ai.as_mut().unwrap().inspect_inbox_copy();
                        for command in commands { this.simple_send(command, cx); }
                        cx.notify();
                    })))
                .child(Button::new("review-inbox-copy-removal").label("Review recoverable removal")
                    .disabled(blocked || busy || !ai.can_remove_inbox_copy())
                    .on_click(cx.listener(|this, _, window, cx| this.open_inbox_copy_confirmation(true, window, cx))))
                .child(Button::new("review-inbox-copy-restore").label("Review exact copy restore")
                    .disabled(blocked || busy || !ai.can_restore_inbox_copy())
                    .on_click(cx.listener(|this, _, window, cx| this.open_inbox_copy_confirmation(false, window, cx))))
                .child(Button::new("retry-inbox-copy").label("Retry exact recorded request")
                    .disabled(blocked || !ai.can_retry_inbox_copy())
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.inbox_blocked() || window.has_active_dialog(cx) { return; }
                        if let Some(command) = this.ai.as_mut().unwrap().retry_inbox_copy() { this.simple_send(command, cx); }
                        cx.notify();
                    }))));
        if ai.inbox_copy_loading() {
            panel = panel.child("Reading fresh preservation evidence and copy history…");
        }
        if ai.inbox_copy_pending() {
            panel = panel.child("Copy operation admitted; outcome is unconfirmed until its exact recorded reply arrives.");
        }
        if let Some(error) = &state.error {
            panel = panel.child(error.clone());
        }
        if let Some(message) = &state.message {
            panel = panel.child(message.clone());
        }
        if let Some(preview) = &state.preview {
            panel = panel.child(format!(
                "Captured preservation digest: {:?}",
                preview.digest
            ));
            for blocker in &preview.evidence.blockers {
                panel = panel.child(format!("Cleanup blocked: {blocker:?}"));
            }
            if let Some(source) = &preview.evidence.source {
                panel = panel
                    .child(format!(
                        "Selected approved Source: {} · approval {}",
                        source.saved.source.path, source.approval.request.operation_id
                    ))
                    .child("Complete saved Source · read only")
                    .child(div().h(px(220.)).flex_shrink_0().child(readonly(
                        &self.inbox.copy_source,
                        "Complete saved Source preservation witness",
                    )))
                    .child(copy_text(
                        "copy-inbox-preserved-source",
                        "Copy complete saved Source",
                        source.saved.text.clone(),
                    ));
            }
            panel = panel.child(
                Button::new("inspect-inbox-copy-proof")
                    .label("Inspect complete preservation proof")
                    .disabled(blocked)
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.inbox_blocked() || window.has_active_dialog(cx) {
                            return;
                        }
                        let Some(preview) = this.ai.as_ref().unwrap().inbox_copy.preview.as_ref()
                        else {
                            return;
                        };
                        let proof = complete_json(preview);
                        this.open_inbox_copy_proof(proof, window, cx);
                    })),
            );
        }
        if let Some(history) = &state.history {
            panel = panel.child(format!(
                "{} recorded copy operations · causal order",
                history.len()
            ));
            for entry in history {
                let operation = entry.operation_id;
                panel = panel
                    .child(format!(
                        "{} · {:?} · {} · parent {:?} · record SHA256 {:?}",
                        entry.operation_id,
                        entry.kind,
                        if entry.settled_at_ms.is_some() {
                            "Recorded settled outcome"
                        } else {
                            "Unconfirmed outcome"
                        },
                        entry.parent,
                        entry.record_sha256
                    ))
                    .child(
                        Button::new(format!("inspect-inbox-copy-operation-{operation}"))
                            .label("Inspect full recorded operation")
                            .disabled(blocked || busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if this.inbox_blocked() || window.has_active_dialog(cx) {
                                    return;
                                }
                                if let Some(command) = this
                                    .ai
                                    .as_mut()
                                    .unwrap()
                                    .inspect_inbox_copy_operation(operation)
                                {
                                    this.simple_send(command, cx);
                                }
                                cx.notify();
                            })),
                    );
            }
        }
        if let Some(record) = state
            .operation
            .as_ref()
            .or(state.receipt.as_ref())
            .or(state.removal.as_ref())
        {
            match record.summary() {
                Ok(summary) => {
                    panel = panel
                        .child(format!(
                            "Recorded copy operation {} · {:?} · {}",
                            summary.operation_id,
                            summary.kind,
                            if summary.settled_at_ms.is_some() {
                                "Settled"
                            } else {
                                "Outcome unconfirmed; inspect and retry the exact request"
                            }
                        ))
                        .child(
                            Button::new("inspect-inbox-copy-record")
                                .label("Inspect complete versioned copy record")
                                .disabled(blocked)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if this.inbox_blocked() || window.has_active_dialog(cx) {
                                        return;
                                    }
                                    let state = &this.ai.as_ref().unwrap().inbox_copy;
                                    let Some(record) = state
                                        .operation
                                        .as_ref()
                                        .or(state.receipt.as_ref())
                                        .or(state.removal.as_ref())
                                    else {
                                        return;
                                    };
                                    let proof = complete_json(record);
                                    this.open_inbox_copy_proof(proof, window, cx);
                                })),
                        );
                }
                Err(_) => {
                    panel = panel.child("Recorded copy proof is invalid; no outcome is confirmed.")
                }
            }
        }
        panel.into_any_element()
    }
    fn open_inbox_copy_proof(
        &mut self,
        proof: Result<String, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let proof = match proof {
            Ok(proof) => proof,
            Err(error) => {
                self.ai.as_mut().unwrap().inbox_copy.error = Some(error);
                cx.notify();
                return;
            }
        };
        let editor = cx.new(|cx| EditorState::new(window, cx).default_value(proof.clone()));
        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .title("Complete copy proof · read only")
                .w(px(440.))
                .overlay_closable(false)
                .child(
                    div()
                        .id("inbox-copy-proof-dialog")
                        .test_support()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(copy_text(
                            "copy-inbox-copy-proof",
                            "Copy complete proof",
                            proof.clone(),
                        ))
                        .child(
                            Button::new("close-inbox-copy-proof")
                                .label("Close proof")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            div()
                                .h(px(220.))
                                .flex_shrink_0()
                                .child(readonly(&editor, "Complete versioned copy proof")),
                        ),
                )
        });
    }
    pub(super) fn open_inbox_copy_confirmation(
        &mut self,
        remove: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.inbox_blocked() || window.has_active_dialog(cx) {
            return;
        }
        let capture = if remove {
            self.ai.as_mut().unwrap().capture_inbox_copy_removal()
        } else {
            self.ai.as_mut().unwrap().capture_inbox_copy_restore()
        };
        let Some(capture) = capture else {
            cx.notify();
            return;
        };
        let proof = match confirmation_json(&capture) {
            Ok(proof) => proof,
            Err(error) => {
                self.ai.as_mut().unwrap().inbox_copy.error = Some(error);
                cx.notify();
                return;
            }
        };
        let editor = cx.new(|cx| EditorState::new(window, cx).default_value(proof.clone()));
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let current = desktop.upgrade();
            let disabled = current.as_ref().is_none_or(|desktop| {
                let desktop = desktop.read(cx);
                desktop.inbox_blocked() || desktop.ai.as_ref().unwrap().inbox_copy_pending()
            });
            let mut body = div().id("inbox-copy-confirmation").test_support().flex().flex_col().gap_2()
                .child(copy_text("copy-inbox-copy-confirmation", "Copy complete captured confirmation", proof.clone()))
                .child(format!("{} · {}", capture.item().capture.title, capture.item().capture.id))
                .child(if capture.is_remove() { "Confirm removing this exact Inbox original while retaining its recoverable copy. The approved Source stays saved. This does not mark the item processed or resolve semantic reviews." } else { "Confirm restoring this exact retained original to its vacant Inbox destination. An occupied destination will refuse; no bytes are overwritten." });
            if let Some(preview) = capture.preview() {
                if let brn_workflow::inbox::InboxOriginal::Available { text } = &preview.evidence.original {
                    body = body.child(copy_text("copy-inbox-captured-original", "Copy complete captured original", text.clone()));
                }
                if let Some(source) = &preview.evidence.source {
                    body = body.child(copy_text("copy-inbox-captured-source", "Copy complete captured Source", source.saved.text.clone()));
                }
            }
            if let Some(current) = current
                && let Some(error) = &current.read(cx).ai.as_ref().unwrap().inbox_copy.error {
                body = body.child(error.clone());
            }
            body = body.child(div().h(px(200.)).flex_shrink_0().child(readonly(&editor, "Complete captured copy confirmation and preservation proof")));
            let footer = div().flex().flex_col().gap_1()                .child(Button::new("confirm-inbox-copy").label(if capture.is_remove() { "Confirm recoverable removal" } else { "Confirm exact copy restore" })
                    .disabled(disabled).on_click({
                        let desktop = desktop.clone(); let capture = capture.clone();
                        move |_, window, cx| {
                            let mut admitted = false;
                            let _ = desktop.update(cx, |this, cx| {
                                if this.inbox_blocked() { return; }
                                let Some(command) = this.ai.as_mut().unwrap().confirm_inbox_copy(&capture) else { cx.notify(); return; };
                                admitted = true;
                                this.simple_send(command, cx);
                            });
                            if admitted { window.close_dialog(cx); }
                        }
                    }))
                .child(Button::new("cancel-inbox-copy").label("Cancel without changing copies").on_click(|_, window, cx| window.close_dialog(cx)))
;
            dialog.title(if capture.is_remove() { "Confirm recoverable copy removal" } else { "Confirm retained copy restoration" })
                .w(px(440.)).overlay_closable(false).footer(footer).child(body)
        });
    }
}
