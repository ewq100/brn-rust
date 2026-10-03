//! Exact captured approval and read-only application history presentation.
use super::*;
use brn_workflow::{
    proposal_apply::{ApplyJournal, ApplyOutcome},
    proposals::{CommentTarget, NoteChange, ProposalRecord},
};
use gpui_kit::{AnyElement, Div, base::Disableable, component::WindowExt};

fn sha256(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn full_text(label: &str, text: &str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(format!("{label} · {} UTF-8 bytes", text.len()))
        .child(div().p_2().child(text.to_owned()))
}

/// Every captured member is rendered in full; this is not a preview or editor.
fn snapshot(record: &ProposalRecord) -> Div {
    let mut body = div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .child(format!(
            "Proposal {} · review version {} · {:?}",
            record.draft.id, record.version, record.state
        ))
        .child(full_text("Full title", &record.draft.title));
    for (index, change) in record.draft.changes.iter().enumerate() {
        let kind = match change {
            NoteChange::Create { .. } => "Create",
            NoteChange::Replace { .. } => "Replace",
            NoteChange::Trash { .. } => "Move to Trash",
        };
        let mut member = div().flex().flex_col().gap_2().child(format!(
            "Member {} · {kind} · {}",
            index + 1,
            change.path()
        ));
        match change {
            NoteChange::Create { .. } => {
                member = member.child("Before: no note at the captured destination");
            }
            NoteChange::Replace {
                before,
                before_text,
                ..
            }
            | NoteChange::Trash {
                before,
                before_text,
                ..
            } => {
                member = member
                    .child(format!(
                        "Captured before · {} bytes · SHA-256 {}",
                        before.len,
                        sha256(&before.sha256)
                    ))
                    .child(full_text("Full before text", before_text));
            }
        }
        member = match change.text() {
            Some(text) => member.child(full_text("Full proposed text", text)),
            None => member.child("Proposed: move the captured original to Trash"),
        };
        body = body.child(member);
    }
    body = body.child("Captured source versions");
    if record.draft.sources.is_empty() {
        body = body.child("No source versions attached");
    }
    for source in &record.draft.sources {
        body = body.child(format!(
            "{} · {} bytes · SHA-256 {}",
            source.path,
            source.fingerprint.len,
            sha256(&source.fingerprint.sha256)
        ));
    }
    body = body.child("Temporary review comments");
    if record.comments.is_empty() {
        body = body.child("No temporary comments");
    }
    for comment in &record.comments {
        let target = match &comment.target {
            CommentTarget::Proposal => "Whole proposal".to_owned(),
            CommentTarget::Text(anchor) => format!(
                "Member {} · exact UTF-8 byte range {}..{}",
                anchor.change_index + 1,
                anchor.start,
                anchor.end
            ),
            CommentTarget::Unresolved(anchor) => format!(
                "Unresolved previous selection · member {} · old UTF-8 byte range {}..{}",
                anchor.change_index + 1,
                anchor.start,
                anchor.end
            ),
        };
        let mut row = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(format!("Comment {} · {target}", comment.id));
        if let CommentTarget::Text(anchor) | CommentTarget::Unresolved(anchor) = &comment.target {
            row = row.child(full_text("Captured quote", &anchor.quote));
        }
        body = body.child(row.child(full_text("Full comment", &comment.text)));
    }
    body
}

fn outcome(journal: &ApplyJournal) -> &'static str {
    match journal.receipt.as_ref().map(|receipt| receipt.outcome) {
        Some(ApplyOutcome::Applied) => "Applied",
        Some(ApplyOutcome::NotApplied) => "Not applied",
        Some(ApplyOutcome::Uncertain) => "Uncertain",
        None => "Pending · outcome unconfirmed",
    }
}

impl Desktop {
    fn approval_native_blocked(&self, cx: &App) -> bool {
        self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || self.close_failed
            || !self.review_comment.read(cx).value().is_empty()
            || self.ai.as_ref().is_none_or(|ai| ai.application_busy())
    }

    pub(super) fn open_approval_dialog(
        &mut self,
        group: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) {
            return;
        }
        if self.approval_native_blocked(cx) {
            self.ai.as_mut().unwrap().notice =
                "Finish retained comment text and current work before approval.".into();
            cx.notify();
            return;
        }
        let Some(capture) = self.ai.as_mut().unwrap().capture_approval(group) else {
            cx.notify();
            return;
        };
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let current = desktop.upgrade();
            let disabled = current
                .as_ref()
                .is_none_or(|desktop| desktop.read(cx).approval_native_blocked(cx));
            let mut content = div().flex().flex_col().gap_3();
            if let Some(group_id) = capture.group_id() {
                content = content.child(format!(
                    "Approve these {} captured proposals in group {group_id}? Each proposal is independent. Application may stop after an earlier proposal; later proposals remain unapplied. New arrivals are not included.",
                    capture.records().len()
                ));
            } else {
                content = content.child("Approve this exact full proposal? This applies the captured changes to vault Markdown. Temporary comments are deleted only after successful application.");
            }
            for (record, request) in capture.records().iter().zip(capture.requests()) {
                content = content
                    .child(format!("Approval operation {}", request.operation_id))
                    .child(snapshot(record));
            }
            if let Some(desktop) = &current {
                let ai = desktop.read(cx).ai.as_ref().unwrap();
                if let Some(error) = &ai.approval_error {
                    content = content.child(error.clone());
                }
                content = content.child(ai.notice.clone());
            }
            let desktop = desktop.clone();
            let capture = capture.clone();
            // The toolkit scrolls the entire dialog body, including every snapshot.
            dialog
                .title("Confirm exact approval")
                .w(px(840.))
                .child(content)
                .child(
                    Button::new("confirm-exact-proposal-approval")
                        .label("Approve captured changes")
                        .disabled(disabled)
                        .on_click(move |_, window, cx| {
                            let mut admitted = false;
                            let _ = desktop.update(cx, |this, cx| {
                                if this.approval_native_blocked(cx) {
                                    this.ai.as_mut().unwrap().notice = "Approval was not admitted. Retained comment text or current work must be settled first.".into();
                                    cx.notify();
                                    return;
                                }
                                let Some(command) = this.ai.as_mut().unwrap().confirm_approval(&capture) else {
                                    this.ai.as_mut().unwrap().notice = "Approval was not admitted. The captured review changed or is unavailable. Close this confirmation and inspect the acknowledged review before capturing again.".into();
                                    cx.notify();
                                    return;
                                };
                                let operation = command.0;
                                this.simple_send(command, cx);
                                admitted = this.ai.as_ref().unwrap().pending.contains_key(&operation);
                            });
                            if admitted {
                                window.close_dialog(cx);
                            }
                        }),
                )
        });
    }

    fn open_application_snapshot(
        &mut self,
        operation: Uuid,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.approval_native_blocked(cx) || window.has_active_dialog(cx) {
            return;
        }
        let Some(command) = self.ai.as_mut().unwrap().inspect_apply(operation) else {
            return;
        };
        let generation = self.ai.as_ref().unwrap().snapshot_generation;
        self.simple_send(command, cx);
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let mut body = div().flex().flex_col().gap_2().child(format!("Operation {operation}"))
                .child("This is the recorded full approval snapshot, not evidence of the current vault contents.");
            if let Some(desktop) = desktop.upgrade() {
                let ai = desktop.read(cx).ai.as_ref().unwrap();
                if generation != ai.snapshot_generation {
                    body = body.child("A different snapshot was requested; close and inspect this operation again.");
                } else if let Some(error) = &ai.snapshot_error { body = body.child(error.clone()); }
                else if let Some(journal) = &ai.application_snapshot {
                    body = body.child(format!("{} · admitted at {} ms since Unix epoch", outcome(journal), journal.started_at_ms)).child(snapshot(&journal.approved));
                } else { body = body.child("Loading the identified full snapshot…"); }
            }
            dialog.title("Recorded approval snapshot · read only").w(px(840.)).child(body)
        });
    }

    pub(super) fn render_activity(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let blocked = self.approval_native_blocked(cx);
        let ai = self.ai.as_ref().unwrap();
        let p = self.palette();
        let mut body = div()
            .id("approved-activity-and-recovery")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_3()
            .p_3()
            .child("Approved changes")
            .child("Activity records completed approvals. Current vault files may have changed afterward.")
            .child(
                Button::new("refresh-approval-activity")
                    .label("Refresh activity and recovery")
                    .disabled(blocked)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.approval_native_blocked(cx) {
                            return;
                        }
                        let mut commands = vec![];
                        let ai = this.ai.as_mut().unwrap();
                        commands.extend(ai.refresh_activity());
                        commands.extend(ai.refresh_applies());
                        for command in commands {
                            this.simple_send(command, cx);
                        }
                    })),
            );
        if let Some(error) = &ai.activity_error {
            body = body.child(error.clone());
        }
        if let Some(page) = &ai.activity {
            if page.entries.is_empty() {
                body = body.child("No completed approvals recorded");
            }
            for entry in &page.entries {
                let mut row =
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_3()
                        .border_1()
                        .border_color(theme::color(p.line))
                        .child(entry.approved_at_utc.clone().unwrap_or_else(|| {
                            format!("{} ms since Unix epoch", entry.approved_at_ms)
                        }))
                        .child(full_text("Approved title", &entry.title))
                        .child(entry.summary.clone())
                        .child(format!(
                            "Operation {} · proposal {}",
                            entry.operation_id, entry.proposal_id
                        ));
                for change in &entry.changes {
                    row = row.child(format!("{:?} · {}", change.kind, change.path));
                }
                if let Some(undo) = &entry.undo {
                    row = row.child(format!(
                        "Undo of operation {}{}",
                        undo.operation_id,
                        undo.trash_member
                            .map(|index| format!(" · Trash member {}", index + 1))
                            .unwrap_or_default()
                    ));
                }
                let operation = entry.operation_id;
                row = row.child(
                    Button::new(format!("inspect-approved-{operation}"))
                        .label("Inspect full recorded approval…")
                        .disabled(blocked)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_application_snapshot(operation, window, cx)
                        })),
                );
                body = body.child(row);
            }
            body = body.child(
                Button::new("more-approval-activity")
                    .label("Load earlier activity")
                    .disabled(blocked || page.next_before.is_none())
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.approval_native_blocked(cx)
                            && let Some(command) = this.ai.as_mut().unwrap().more_activity()
                        {
                            this.simple_send(command, cx);
                        }
                    })),
            );
        } else {
            body = body.child("Activity has not loaded yet");
        }
        body = body.child("Latest approval results");
        if let Some(error) = &ai.approval_error {
            body = body.child(error.clone());
        }
        for receipt in &ai.approval_receipts {
            body = body.child(format!(
                "Operation {} · proposal {} · approved review {} · {:?}",
                receipt.operation_id,
                receipt.proposal_id,
                receipt.approved_version,
                receipt.outcome
            ));
        }
        for request in &ai.approval_requests {
            if !ai
                .approval_receipts
                .iter()
                .any(|receipt| receipt.operation_id == request.operation_id)
            {
                let operation = request.operation_id;
                body = body.child(format!("Requested operation {operation} · proposal {} · review {} · inspect its recorded outcome", request.expected.id, request.expected.version))
                    .child(Button::new(format!("inspect-requested-{operation}")).label("Inspect recorded outcome…").disabled(blocked)
                        .on_click(cx.listener(move |this, _, window, cx| this.open_application_snapshot(operation, window, cx))));
            }
        }
        body = body.child("Recovery requiring attention");
        if let Some(error) = &ai.applies_error {
            body = body.child(error.clone());
        }
        if ai.applies.is_empty() {
            body = body.child("No pending or uncertain application is loaded");
        }
        for summary in &ai.applies {
            let operation = summary.request.operation_id;
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .border_1()
                    .border_color(theme::color(p.amber))
                    .child(format!("Operation {operation} · {}", match summary.outcome { None => "Pending · outcome unconfirmed", Some(ApplyOutcome::Uncertain) => "Uncertain", Some(ApplyOutcome::Applied) => "Applied", Some(ApplyOutcome::NotApplied) => "Not applied" }))
                    .child(full_text("Captured title", &summary.title))
                    .child("Reconcile inspects this original operation. It does not submit a fresh approval or choose a repair direction.")
                    .child(Button::new(format!("inspect-recovery-{operation}")).label("Inspect full recorded approval…").disabled(blocked).on_click(cx.listener(move |this, _, window, cx| this.open_application_snapshot(operation, window, cx))))
                    .child(
                        Button::new(format!("reconcile-approval-{operation}"))
                            .label("Reconcile this operation")
                            .disabled(blocked)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.approval_native_blocked(cx)
                                    && let Some(command) = this.ai.as_mut().unwrap().reconcile_apply(operation)
                                {
                                    this.simple_send(command, cx);
                                }
                            })),
                    ),
            );
        }
        body.into_any_element()
    }
}
