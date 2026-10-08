//! Exact captured approval and read-only application history presentation.
use super::*;
use brn_workflow::{
    proposal_apply::{ApplyJournal, ApplyOutcome, RepairDirection},
    proposals::{CommentTarget, NoteChange, ProposalDraft, ProposalRecord},
};
use gpui_kit::{
    AnyElement, Div, TestSupportExt,
    base::Disableable,
    component::{WindowExt, checkbox::Checkbox},
};
use sha2::{Digest, Sha256};

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

fn asset_line(scope: &str, field: &str, label: String) -> AnyElement {
    div()
        .id(format!("{scope}-{field}"))
        .test_support()
        .aria_label(label.clone())
        .child(label)
        .into_any_element()
}

/// Exact held-payload proof, shared by review and captured approval/Undo/repair.
/// Assets never become Markdown editor input or a text selection surface.
pub(super) fn asset_body(scope: &str, change: &NoteChange) -> AnyElement {
    let kind = match change {
        NoteChange::CreateAsset { .. } => "Create asset",
        NoteChange::ReplaceAsset { .. } => "Replace asset",
        NoteChange::TrashAsset { .. } => "Move asset to Trash",
        _ => unreachable!("asset presentation requires a typed asset member"),
    };
    let mut body = div()
        .id(format!("{scope}-proof"))
        .test_support()
        .flex()
        .flex_col()
        .gap_1()
        .child(asset_line(scope, "kind", kind.into()))
        .child(asset_line(
            scope,
            "destination",
            format!("Destination: {}", change.path()),
        ))
        .child(asset_line(
            scope,
            "parent",
            format!(
                "Captured parent · device {} · inode {}",
                change.parent().device,
                change.parent().inode
            ),
        ));
    body = if let Some(before) = change.before() {
        body.child(asset_line(
            scope,
            "before",
            format!(
                "Captured before · device {} · inode {} · {} bytes · SHA-256 {}",
                before.device,
                before.inode,
                before.len,
                sha256(&before.sha256)
            ),
        ))
    } else {
        body.child(asset_line(
            scope,
            "before",
            "Before: no asset at the captured destination".into(),
        ))
    };
    body = if let Some(bytes) = change.candidate_bytes() {
        let hash: [u8; 32] = Sha256::digest(bytes).into();
        body.child(asset_line(
            scope,
            "candidate",
            format!(
                "Exact proposed asset · {} bytes · SHA-256 {}",
                bytes.len(),
                sha256(&hash)
            ),
        ))
    } else {
        body.child(asset_line(
            scope,
            "candidate",
            "Proposed: move the exact captured asset to recoverable Trash".into(),
        ))
    };
    body.child("Asset bytes stay fixed in this review and are not interpreted as text.")
        .into_any_element()
}

/// Render the actual draft supplied by approval, Undo or repair without inventing a review record.
fn draft_body(draft: &ProposalDraft) -> Div {
    let mut body = div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .child(full_text("Full title", &draft.title));
    for (index, change) in draft.changes.iter().enumerate() {
        let kind = match change {
            NoteChange::Create { .. } => "Create",
            NoteChange::Replace { .. } => "Replace",
            NoteChange::Trash { .. } => "Move to Trash",
            NoteChange::CreateAsset { .. } => "Create asset",
            NoteChange::ReplaceAsset { .. } => "Replace asset",
            NoteChange::TrashAsset { .. } => "Move asset to Trash",
        };
        let mut member = div().flex().flex_col().gap_2().child(format!(
            "Member {} · {kind} · {}",
            index + 1,
            change.path()
        ));
        if change.is_asset() {
            body = body.child(member.child(asset_body(&format!("captured-asset-{index}"), change)));
            continue;
        }
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
            NoteChange::CreateAsset { .. }
            | NoteChange::ReplaceAsset { .. }
            | NoteChange::TrashAsset { .. } => unreachable!("assets use their proof presentation"),
        }
        member = match change.text() {
            Some(text) => member.child(full_text("Full proposed text", text)),
            None => member.child("Proposed: move the captured original to Trash"),
        };
        body = body.child(member);
    }
    for (index, change) in draft.action_changes.iter().enumerate() {
        body = body.child(super::action_review::member_body(index, change));
    }
    body = body.child("Captured source versions");
    if draft.sources.is_empty() {
        body = body.child("No source versions attached");
    }
    for source in &draft.sources {
        body = body.child(format!(
            "{} · {} bytes · SHA-256 {}",
            source.path,
            source.fingerprint.len,
            sha256(&source.fingerprint.sha256)
        ));
    }
    body
}

/// The same complete frozen inverse appears in the native Undo confirmation.
pub(super) fn undo_body(capture: &crate::approval::UndoCapture) -> AnyElement {
    let mut body = div()
        .id("exact-undo-capture")
        .test_support()
        .flex()
        .flex_col()
        .gap_2();
    if !capture.preview().draft.action_changes.is_empty() {
        let meaning = "Restore previous Action details as a new revision, preserving origin and history. Changed or Completed Actions refuse confirmation.";
        body = body.child(
            div()
                .id("action-compensation-meaning")
                .test_support()
                .aria_label(meaning)
                .child(meaning),
        );
    }
    body.child(draft_body(&capture.preview().draft))
        .into_any_element()
}

fn review_comments(record: &ProposalRecord) -> Div {
    let mut body = div()
        .flex()
        .flex_col()
        .gap_2()
        .child("Temporary review comments");
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

/// Every captured member and temporary comment is rendered in full.
fn snapshot(record: &ProposalRecord) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(format!(
            "Proposal {} · review version {} · {:?}",
            record.draft.id, record.version, record.state
        ))
        .child(draft_body(&record.draft))
        .child(review_comments(record))
}

fn direction_name(direction: RepairDirection) -> &'static str {
    match direction {
        RepairDirection::Finish => "Finish",
        RepairDirection::Restore => "Restore",
    }
}

fn repair_outcome(outcome: Option<ApplyOutcome>) -> &'static str {
    match outcome {
        Some(ApplyOutcome::Applied) => "Applied",
        Some(ApplyOutcome::NotApplied) => "Not applied",
        Some(ApplyOutcome::Uncertain) => "Uncertain",
        None => "Outcome unconfirmed",
    }
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

    pub(super) fn operation_native_blocked(&self, cx: &App) -> bool {
        self.approval_native_blocked(cx)
            || self.ai.as_ref().is_none_or(|ai| {
                !ai.ready || !ai.review_can_leave() || ai.active.is_some() || ai.rewrite.is_some()
            })
    }

    pub(super) fn file_operation_native_blocked(&self, cx: &App) -> bool {
        self.operation_native_blocked(cx) || self.ai.as_ref().is_none_or(|ai| !ai.vault_bound)
    }

    fn open_undo_dialog(
        &mut self,
        operation: Uuid,
        trash_member: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.operation_native_blocked(cx) || window.has_active_dialog(cx) {
            return;
        }
        let Some(command) = self
            .ai
            .as_mut()
            .unwrap()
            .preview_undo(operation, trash_member)
        else {
            return;
        };
        let generation = self.ai.as_ref().unwrap().operation_generation;
        self.simple_send(command, cx);
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let mut body = div().flex().flex_col().gap_2()
                .child(format!("Source operation {operation}"))
                .child(trash_member.map(|index| format!("Restore original Trash member {}", index + 1)).unwrap_or_else(|| "Undo the complete recorded operation".into()))
                .child("This historical inverse does not establish current eligibility. Confirmation may refuse changed Actions, destinations or retained originals.");
            if let Some(current) = desktop.upgrade() {
                let this = current.read(cx);
                let ai = this.ai.as_ref().unwrap();
                if generation != ai.operation_generation {
                    body = body.child("A different operation preview was requested; close and preview this operation again.");
                } else if let Some(error) = &ai.operation_error {
                    body = body.child(error.clone());
                } else if let Some(capture) = ai.undo_preview.as_ref().filter(|capture| {
                    capture.request().target_operation_id == operation
                        && capture.request().trash_member == trash_member
                }) {
                    let request = capture.request();
                    body = body.child(format!("Captured Undo operation {}", request.operation_id))
                        .child(undo_body(capture));
                    for (index, original) in capture.preview().binding.originals.iter().enumerate() {
                        if let Some(original) = original {
                            body = body.child(format!("Retained original for inverse member {} · member {} · {} bytes · SHA-256 {}", index + 1, original.member_id, original.fingerprint.len, sha256(&original.fingerprint.sha256)));
                        }
                    }
                    let desktop = desktop.clone();
                    let capture = capture.clone();
                    body = body.child(Button::new("confirm-captured-undo")
                        .label(if trash_member.is_some() { "Confirm captured Trash restore" } else if !capture.preview().draft.action_changes.is_empty() { "Confirm previous Action details as new revision" } else { "Confirm captured Undo" })
                        .disabled(this.operation_native_blocked(cx) || (!ai.vault_bound && !capture.preview().draft.changes.is_empty()))
                        .on_click(move |_, window, cx| {
                            let mut admitted = false;
                            let _ = desktop.update(cx, |this, cx| {
                                if this.operation_native_blocked(cx) || this.ai.as_ref().unwrap().operation_generation != generation {
                                    this.ai.as_mut().unwrap().notice = "Undo was not admitted. Settle current work and recapture this operation if needed.".into();
                                    cx.notify();
                                    return;
                                }
                                let Some(command) = this.ai.as_mut().unwrap().confirm_undo(&capture) else {
                                    this.ai.as_mut().unwrap().notice = "Undo was not admitted. The captured preview changed; close and preview the identified operation again.".into();
                                    cx.notify();
                                    return;
                                };
                                let outer = command.0;
                                this.simple_send(command, cx);
                                admitted = this.ai.as_ref().unwrap().pending.contains_key(&outer);
                            });
                            if admitted { window.close_dialog(cx); }
                        }));
                } else {
                    body = body.child("Loading the identified complete Undo preview…");
                }
                body = body.child(ai.notice.clone());
            }
            dialog.title("Review exact Undo / Trash restore").w(px(840.)).child(body)
        });
    }

    fn open_repair_dialog(
        &mut self,
        operation: Uuid,
        direction: RepairDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.file_operation_native_blocked(cx) || window.has_active_dialog(cx) {
            return;
        }
        let Some(preview) = self
            .ai
            .as_mut()
            .unwrap()
            .preview_repair(operation, direction)
        else {
            return;
        };
        let generation = self.ai.as_ref().unwrap().operation_generation;
        let Some(snapshot) = self.ai.as_mut().unwrap().inspect_apply(operation) else {
            self.ai.as_mut().unwrap().notice =
                "Repair preview needs its identified recorded approval before confirmation.".into();
            cx.notify();
            return;
        };
        let snapshot_generation = self.ai.as_ref().unwrap().snapshot_generation;
        self.simple_send(snapshot, cx);
        self.simple_send(preview, cx);
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let name = direction_name(direction);
            let mut body = div().flex().flex_col().gap_2()
                .child(format!("Original operation {operation} · explicit direction {name}"))
                .child(match direction {
                    RepairDirection::Finish => "Finish completes the captured approved changes from the observed member phases.",
                    RepairDirection::Restore => "Restore returns the captured members to their before states from the observed member phases.",
                })
                .child("Confirmation is bound to this repair attempt and observation hash. Changed proofs may refuse it; no automatic retry follows.");
            if let Some(current) = desktop.upgrade() {
                let this = current.read(cx);
                let ai = this.ai.as_ref().unwrap();
                if generation != ai.operation_generation || snapshot_generation != ai.snapshot_generation {
                    body = body.child("A different preview or snapshot was requested; close and review this repair direction again.");
                } else if let Some(error) = &ai.operation_error {
                    body = body.child(error.clone());
                } else if let Some(capture) = ai.repair_preview.as_ref().filter(|capture| {
                    capture.request().operation_id == operation && capture.request().direction == direction
                }) {
                    let request = capture.request();
                    body = body.child(format!("Repair attempt {} · expected observation SHA-256 {}", request.id, sha256(&request.expected)));
                    for (index, phase) in capture.preview().phases.iter().enumerate() {
                        body = body.child(format!("Observed original member {} · {:?}", index + 1, phase));
                    }
                    body = body.child(draft_body(&capture.preview().approved));
                    let journal = ai.application_snapshot.as_ref().filter(|journal| {
                        journal.request.operation_id == operation
                            && journal.approved.draft == capture.preview().approved
                    });
                    if let Some(journal) = journal {
                        body = body.child(review_comments(&journal.approved));
                    } else if let Some(error) = &ai.snapshot_error {
                        body = body.child(error.clone());
                    } else {
                        body = body.child("Waiting for the matching recorded approval and its full temporary comments…");
                    }
                    let desktop = desktop.clone();
                    let capture = capture.clone();
                    body = body.child(Button::new("confirm-captured-repair")
                        .label(format!("Confirm captured {name} repair"))
                        .disabled(this.file_operation_native_blocked(cx) || journal.is_none())
                        .on_click(move |_, window, cx| {
                            let mut admitted = false;
                            let _ = desktop.update(cx, |this, cx| {
                                let ai = this.ai.as_ref().unwrap();
                                let current = generation == ai.operation_generation
                                    && snapshot_generation == ai.snapshot_generation
                                    && ai.application_snapshot.as_ref().is_some_and(|journal| journal.request.operation_id == operation && journal.approved.draft == capture.preview().approved);
                                if this.file_operation_native_blocked(cx) || !current {
                                    this.ai.as_mut().unwrap().notice = "Repair was not admitted. Settle current work and review the matching recorded operation again.".into();
                                    cx.notify();
                                    return;
                                }
                                let Some(command) = this.ai.as_mut().unwrap().confirm_repair(&capture) else {
                                    this.ai.as_mut().unwrap().notice = "Repair was not admitted. The captured direction or observation changed; close and review it again.".into();
                                    cx.notify();
                                    return;
                                };
                                let outer = command.0;
                                this.simple_send(command, cx);
                                admitted = this.ai.as_ref().unwrap().pending.contains_key(&outer);
                            });
                            if admitted { window.close_dialog(cx); }
                        }));
                } else {
                    body = body.child("Loading the identified full repair observation…");
                }
                body = body.child(ai.notice.clone());
            }
            dialog.title(format!("Review explicit {} repair", direction_name(direction))).w(px(840.)).child(body)
        });
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
        let selected = std::rc::Rc::new(std::cell::RefCell::new(
            capture
                .records()
                .iter()
                .map(|r| r.draft.id)
                .collect::<std::collections::HashSet<_>>(),
        ));
        window.open_dialog(cx, move |dialog, _, cx| {
            let current = desktop.upgrade();
            let disabled = current
                .as_ref()
                .is_none_or(|desktop| desktop.read(cx).approval_native_blocked(cx));
            let mut content = div().id("exact-approval-capture").test_support().flex().flex_col().gap_3();
            if let Some(group_id) = capture.group_id() {
                content = content.child(format!(
                    "Approve these {} captured proposals in group {group_id}? Each proposal is independent. Application may stop after an earlier proposal; later proposals remain unapplied. New arrivals are not included.",
                    capture.records().len()
                ));
            } else {
                content = content.child("Approve this exact full proposal? This applies the captured Markdown and Action changes. Temporary comments are deleted only after successful application.");
            }
            for (index, (record, request)) in capture.records().iter().zip(capture.requests()).enumerate() {
                if capture.group_id().is_some() {
                    let proposal_id = record.draft.id;
                    let selected = selected.clone();
                    let checked = selected.borrow().contains(&proposal_id);
                    content = content.child(Checkbox::new(format!("approve-selected-{index}"))
                        .label(format!("Include this exact proposal {}", record.draft.title)).checked(checked)
                        .on_change(move |checked, _, _| { if *checked { selected.borrow_mut().insert(proposal_id); } else { selected.borrow_mut().remove(&proposal_id); } }));
                }
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
            let selected = selected.clone();
            // The toolkit scrolls the entire dialog body, including every snapshot.
            dialog
                .title("Confirm exact approval")
                .w(px(840.))
                .child(content)
                .child(
                    Button::new("confirm-exact-proposal-approval")
                        .label("Approve selected captured changes")
                        .disabled(disabled)
                        .on_click(move |_, window, cx| {
                            let mut admitted = false;
                            let _ = desktop.update(cx, |this, cx| {
                                if this.approval_native_blocked(cx) {
                                    this.ai.as_mut().unwrap().notice = "Approval was not admitted. Retained comment text or current work must be settled first.".into();
                                    cx.notify();
                                    return;
                                }
                                let selected_capture = if capture.group_id().is_some() { capture.select(&selected.borrow()) } else { Some(capture.clone()) };
                                let Some(selected_capture) = selected_capture else {
                                    this.ai.as_mut().unwrap().notice = "Select at least one exact proposal and include every pending Source prerequisite for selected knowledge or Actions.".into(); cx.notify(); return;
                                };
                                let Some(command) = this.ai.as_mut().unwrap().confirm_approval(&selected_capture) else {
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
            if let Some(current) = desktop.upgrade() {
                let this = current.read(cx);
                let ai = this.ai.as_ref().unwrap();
                if generation != ai.snapshot_generation {
                    body = body.child("A different snapshot was requested; close and inspect this operation again.");
                } else if let Some(error) = &ai.snapshot_error { body = body.child(error.clone()); }
                else if let Some(journal) = ai.application_snapshot.as_ref().filter(|journal| journal.request.operation_id == operation) {
                    body = body.child(format!("{} · admitted at {} ms since Unix epoch", outcome(journal), journal.started_at_ms)).child(snapshot(&journal.approved));
                    if let Some(repair) = &journal.repair {
                        for attempt in &repair.attempts {
                            body = body.child(format!("Recorded repair attempt {} · {} · expected observation SHA-256 {} · {}", attempt.request.id, direction_name(attempt.request.direction), sha256(&attempt.request.expected), repair_outcome(attempt.outcome)));
                        }
                    }
                    if journal.receipt.as_ref().is_some_and(|receipt| receipt.outcome == ApplyOutcome::Applied) {
                        // Enumerate the complete original snapshot before selecting Trash members.
                        for (index, change) in journal.approved.draft.changes.iter().enumerate() {
                            if matches!(change, NoteChange::Trash { .. } | NoteChange::TrashAsset { .. }) {
                                let desktop = desktop.clone();
                                body = body.child(Button::new(format!("preview-trash-restore-{operation}-{index}"))
                                    .label(format!("Review original Trash member {} restore…", index + 1))
                                    .disabled(this.file_operation_native_blocked(cx))
                                    .on_click(move |_, window, cx| {
                                        let _ = desktop.update(cx, |this, cx| {
                                            if !this.file_operation_native_blocked(cx) {
                                                window.close_dialog(cx);
                                                this.open_undo_dialog(operation, Some(index), window, cx);
                                            }
                                        });
                                    }));
                            }
                        }
                    }
                } else { body = body.child("Loading the identified full snapshot…"); }
            }
            dialog.title("Recorded approval snapshot · read only").w(px(840.)).child(body)
        });
    }

    pub(super) fn render_activity(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let blocked = self.approval_native_blocked(cx);
        let operation_blocked = self.operation_native_blocked(cx);
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
                row = row.child(
                    Button::new(format!("preview-full-undo-{operation}"))
                        .label("Review full Undo…")
                        .disabled(operation_blocked)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_undo_dialog(operation, None, window, cx);
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
        body = body.child("Latest application results");
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
        if let Some(error) = &ai.operation_error {
            body = body.child(error.clone());
        }
        if let Some(request) = &ai.last_undo_request {
            let operation = request.operation_id;
            body = body
                .child(format!(
                    "Requested Undo operation {operation} · source operation {}{}",
                    request.target_operation_id,
                    request
                        .trash_member
                        .map(|index| format!(" · original Trash member {}", index + 1))
                        .unwrap_or_default()
                ))
                .child(
                    Button::new(format!("inspect-requested-undo-{operation}"))
                        .label("Inspect recorded Undo outcome…")
                        .disabled(blocked)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_application_snapshot(operation, window, cx)
                        })),
                );
        }
        if let Some(receipt) = &ai.repair_receipt {
            body = body.child(format!(
                "Repair attempt {} · original operation {} · {} · {}",
                receipt.id,
                receipt.operation_id,
                direction_name(receipt.direction),
                repair_outcome(receipt.outcome)
            ));
        }
        if let Some(request) = &ai.last_repair_request {
            let operation = request.operation_id;
            body = body.child(format!("Requested repair attempt {} · original operation {operation} · {} · expected observation SHA-256 {}", request.id, direction_name(request.direction), sha256(&request.expected)))
                .child(Button::new(format!("inspect-requested-repair-{}", request.id)).label("Inspect recorded repair operation…").disabled(blocked)
                    .on_click(cx.listener(move |this, _, window, cx| this.open_application_snapshot(operation, window, cx))));
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
                    )
                    .child(
                        Button::new(format!("preview-finish-repair-{operation}"))
                            .label("Review Finish repair…")
                            .disabled(operation_blocked || !ai.vault_bound)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_repair_dialog(operation, RepairDirection::Finish, window, cx);
                            })),
                    )
                    .child(
                        Button::new(format!("preview-restore-repair-{operation}"))
                            .label("Review Restore repair…")
                            .disabled(operation_blocked || !ai.vault_bound)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_repair_dialog(operation, RepairDirection::Restore, window, cx);
                            })),
                    ),
            );
        }
        body.into_any_element()
    }
}
