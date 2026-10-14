//! Writing interactions stay with the document; chat remains optional.
use super::shell::pill;
use super::*;
use gpui_kit::component::button::ButtonVariants;
impl ThreadsUi {
    pub(super) fn writing_candidate_note(&self, candidate: &Prepared) -> Option<String> {
        candidate.request.inputs.iter().find_map(|input| {
            self.workspace
                .writing_request(&input.record)
                .ok()
                .map(|snapshot| snapshot.note.id)
        })
    }
    /// Match persisted host intent, never prompt prose or arbitrary conversation content.
    pub(super) fn latest_writing_instruction(&self, thread: &str) -> Option<String> {
        self.workspace
            .messages(thread)
            .ok()?
            .into_iter()
            .rev()
            .find_map(|record| match &record.data {
                RecordData::Message(message) if message.role == "owner" => Some(None),
                RecordData::Message(message) if message.role == "writing-request" => {
                    Some(Some(record.id.clone()))
                }
                _ => None,
            })
            .flatten()
    }
    pub(super) fn writing_worker_for(&self, note: &str) -> bool {
        self.worker
            .as_ref()
            .and_then(|worker| worker.run_id.as_ref())
            .and_then(|id| self.workspace.store.record(id).ok().flatten())
            .and_then(|record| match record.data {
                RecordData::Run(run) => run.instruction,
                _ => None,
            })
            .and_then(|id| self.workspace.writing_request(&id).ok())
            .is_some_and(|snapshot| snapshot.note.id == note)
    }
    pub(super) fn writing_activity(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(note) = self.open_note_id() else {
            return div().into_any_element();
        };
        if !self.writing_worker_for(note) {
            return div().into_any_element();
        }
        div()
            .id("writing-progress")
            .flex()
            .flex_col()
            .gap_1()
            .px_3()
            .py_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .text_size(UI_TEXT)
                            .child(if self.progress.is_empty() {
                                "Rewriting…".to_owned()
                            } else {
                                self.progress.clone()
                            }),
                    )
                    .child(
                        pill("stop-writing", "Stop")
                            .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                    ),
            )
            .test_support()
            .into_any_element()
    }
    pub(super) fn writing_proposals(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(note) = self.open_note_id() else {
            return div().into_any_element();
        };
        let mut proposals = div()
            .id("writing-proposals")
            .flex()
            .flex_col()
            .gap_1()
            .flex_shrink_0()
            .max_h(px(110.))
            .overflow_y_scroll();
        for candidate in self
            .candidates
            .iter()
            .filter(|candidate| self.writing_candidate_note(candidate).as_deref() == Some(note))
        {
            let summary = candidate
                .request
                .inputs
                .iter()
                .find_map(|input| self.workspace.writing_request(&input.record).ok())
                .map(|snapshot| {
                    if snapshot.general_feedback.trim().is_empty() {
                        format!("Rewrite ready · {} comments", snapshot.comments.len())
                    } else {
                        format!(
                            "Rewrite ready · {}",
                            snapshot
                                .general_feedback
                                .chars()
                                .take(38)
                                .collect::<String>()
                        )
                    }
                })
                .unwrap_or_else(|| "Rewrite ready to review".into());
            let show = candidate.clone();
            let dismiss = candidate.clone();
            let op = candidate.operation.as_str();
            proposals =
                proposals.child(
                    div()
                        .id(format!("writing-proposal-{op}"))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_2()
                        .child(div().flex_1().text_size(UI_TEXT).child(summary))
                        .child(pill(format!("writing-dismiss-{op}"), "Dismiss").on_click(
                            cx.listener(move |this, _, window, cx| {
                                this.card_dismiss(dismiss.clone(), window, cx)
                            }),
                        ))
                        .child(
                            button(format!("writing-review-{op}"))
                                .label("Review rewrite")
                                .compact()
                                .primary()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.open_review(show.clone(), window, cx)
                                })),
                        )
                        .test_support(),
                );
        }
        proposals.test_support().into_any_element()
    }
    pub(super) fn latest_note_writing_request(&self, note: &str) -> Option<Record> {
        let thread = self.writing_threads.get(note)?;
        self.workspace.messages(thread).ok()?.into_iter().rev().find(|record| {
            matches!(&record.data, RecordData::Message(message) if message.role == "writing-request") && self.workspace.writing_request(&record.id).is_ok_and(|snapshot| snapshot.note.id == note)
        })
    }
    pub(super) fn latest_rewrite_applied(&self, note: &str) -> bool {
        self.applied_rewrite.get(note).is_some_and(|(request, _)| {
            self.latest_note_writing_request(note)
                .is_some_and(|record| record.id == *request)
        })
    }
    pub(super) fn writing_saved_status(&self) -> Option<String> {
        let note = self.open_note_id()?;
        if self.writing_worker_for(note)
            || self
                .candidates
                .iter()
                .any(|candidate| self.writing_candidate_note(candidate).as_deref() == Some(note))
            || self.latest_rewrite_applied(note)
        {
            return None;
        }
        let thread = self.writing_threads.get(note)?;
        let request = self.latest_writing_instruction(thread)?;
        let snapshot = self.workspace.writing_request(&request).ok()?;
        if snapshot.note.id != note {
            return None;
        }
        let run = latest_run(&self.records, &self.history, thread).and_then(|record| match &record
            .data
        {
            RecordData::Run(run) if run.instruction.as_deref() == Some(request.as_str()) => {
                Some(run)
            }
            _ => None,
        });
        Some(match run {
            None => "Rewrite inputs saved · ready to retry".into(),
            Some(run) => match run.state {
                RunState::Working => "Saved rewrite activity unconfirmed · retry when ready".into(),
                RunState::Completed => "Rewrite finished · no pending proposal".into(),
                _ => format!("Rewrite {:?} · saved feedback is ready to retry", run.state)
                    .to_lowercase(),
            },
        })
    }
    /// A response is writing-owned only when committed with its exact bound terminal run.
    pub(super) fn writing_result_note(&self) -> Option<Record> {
        let note = self.open_note_id()?;
        if self.writing_worker_for(note)
            || self
                .candidates
                .iter()
                .any(|candidate| self.writing_candidate_note(candidate).as_deref() == Some(note))
            || self.latest_rewrite_applied(note)
        {
            return None;
        }
        let thread = self.writing_threads.get(note)?;
        let request = self.latest_writing_instruction(thread)?;
        if self.workspace.writing_request(&request).ok()?.note.id != note {
            return None;
        }
        let record = latest_run(&self.records, &self.history, thread)?;
        let RecordData::Run(run) = &record.data else {
            return None;
        };
        if run.instruction.as_deref() != Some(request.as_str()) || run.state == RunState::Working {
            return None;
        }
        self.history.iter().rev().filter(|receipt| receipt.writes.iter().any(|write| write.after.id == record.id && matches!(&write.after.data, RecordData::Run(saved) if saved.state == run.state && saved.instruction == run.instruction)))
            .find_map(|receipt| receipt.writes.iter().find_map(|write| {
                if write.before.is_none() && matches!(&write.after.data, RecordData::Message(message) if message.thread == *thread && matches!(message.role.as_str(), "assistant" | "assistant-partial")) {
                    self.records.iter().find(|record| record.id == write.after.id && !record.archived).cloned()
                } else { None }
            }))
    }
    pub(super) fn writing_result(&self) -> gpui_kit::AnyElement {
        let Some(record) = self.writing_result_note() else {
            return div().into_any_element();
        };
        let RecordData::Message(message) = record.data else {
            return div().into_any_element();
        };
        div()
            .id("writing-result")
            .flex()
            .flex_col()
            .gap_1()
            .flex_shrink_0()
            .max_h(px(120.))
            .overflow_y_scroll()
            .px_3()
            .py_2()
            .child(
                div()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(pal().muted))
                    .child("BRN · rewrite response"),
            )
            .child(self.safe_message_for_thread(
                format!("writing-response-{}", record.id),
                message.text,
                Some(&message.thread),
            ))
            .test_support()
            .into_any_element()
    }
    pub(super) fn overall_feedback(&self) -> gpui_kit::AnyElement {
        div()
            .id("overall-feedback")
            .flex()
            .flex_col()
            .gap_1()
            .px_3()
            .py_2()
            .child(
                div()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(pal().muted))
                    .child("Overall feedback"),
            )
            .child(
                div().h(px(64.)).child(
                    prose_editor(&self.general_feedback)
                        .h_full()
                        .aria_label("Overall feedback"),
                ),
            )
            .test_support()
            .into_any_element()
    }
}
