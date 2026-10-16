//! Writing interactions stay with the document; chat remains optional.
use super::shell::pill;
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::{Icon, Sizable, Size};
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
            .and_then(|id| self.writing_requests.get(&id))
            .is_some_and(|snapshot| snapshot.note.id == note)
    }
    pub(super) fn writing_activity(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(note) = self.open_note_id() else {
            return div().into_any_element();
        };
        if !self.writing_worker_for(note) {
            return div().into_any_element();
        }
        // Shown whenever this note's rewrite runs, whatever the bar shows.
        div()
            .id("writing-progress")
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .border_t_1()
            .border_color(rgb(pal().hairline))
            .child(
                Icon::new(IconName::LoaderCircle)
                    .with_size(Size::Small)
                    .flex_shrink_0()
                    .text_color(rgb(pal().info)),
            )
            .child(div().flex_1().min_w_0().child(if self.progress.is_empty() {
                "Rewriting…".to_owned()
            } else {
                self.progress.clone()
            }))
            .child(
                button("stop-writing")
                    .label("Stop")
                    .icon(IconName::Square)
                    .compact()
                    .outline()
                    .h(components::tokens::COMPACT_CONTROL)
                    .tooltip("Stop the current work and pause the queue · ⌘.")
                    .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
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
            .flex_shrink_0()
            .max_h(px(110.))
            .overflow_y_scroll();
        let ready: Vec<&Prepared> = self
            .candidates
            .iter()
            .filter(|candidate| self.writing_candidate_note(candidate).as_deref() == Some(note))
            .filter(|candidate| !candidate.request.writes.iter().any(|w| matches!(&w.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE))
                || self.writing_reviews.contains_key(candidate.operation.as_str()))
            .collect();
        // Only the newest proposal is filled, and only while it is the
        // bar's first matching row; older ones are outline.
        let lead = self.writing_bar_row() == Some(WritingRow::ProposalReady);
        let newest = ready.len().saturating_sub(1);
        if !ready.is_empty() {
            proposals = proposals.border_t_1().border_color(rgb(pal().hairline));
        }
        for (index, candidate) in ready.into_iter().enumerate() {
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
                        .py_1()
                        .child(div().flex_1().min_w_0().text_size(UI_TEXT).child(summary))
                        .child(pill(format!("writing-dismiss-{op}"), "Dismiss").on_click(
                            cx.listener(move |this, _, window, cx| {
                                this.card_dismiss(dismiss.clone(), window, cx)
                            }),
                        ))
                        .child({
                            let review = button(format!("writing-review-{op}"))
                                .label("Review rewrite")
                                .compact()
                                .h(components::tokens::COMPACT_CONTROL)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.open_review(show.clone(), window, cx)
                                }));
                            if lead && index == newest {
                                review.primary()
                            } else {
                                review.outline()
                            }
                        })
                        .test_support(),
                );
        }
        proposals.test_support().into_any_element()
    }
    pub(super) fn latest_note_writing_request(&self, note: &str) -> Option<Record> {
        let thread = self.writing_threads.get(note)?;
        self.workspace.messages(thread).ok()?.into_iter().rev().find(|record| {
            matches!(&record.data, RecordData::Message(message) if message.role == "writing-request") && self.writing_requests.get(&record.id).is_some_and(|s| s.note.id == note)
        })
    }
    pub(super) fn latest_rewrite_applied(&self, note: &str) -> bool {
        self.applied_rewrite.get(note).is_some_and(|(request, _)| {
            self.latest_note_writing_request(note)
                .is_some_and(|record| record.id == *request)
        })
    }
    /// The last rewrite's recorded outcome, under the conditions today's
    /// saved-status line used: no live worker for this note, no proposal and
    /// no applied rewrite.
    pub(super) fn writing_run_outcome(&self) -> Option<RunOutcome> {
        let note = self.open_note_id()?;
        if self.writing_worker_for(note)
            || self
                .candidates
                .iter()
            .filter(|candidate| !candidate.request.writes.iter().any(|w| matches!(&w.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE))
                || self.writing_reviews.contains_key(candidate.operation.as_str()))
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
                Some((record.id.clone(), run))
            }
            _ => None,
        });
        // A failed or interrupted rewrite reads as its chat's recorded
        // notice (the Needs you text), as the status line and the chat's
        // recovery card do.
        let notice = |id: &str, run: &Run| {
            let RecordData::Thread(chat) = &self.records.iter().find(|r| r.id == *thread)?.data
            else {
                return None;
            };
            terminal_notice(run, id, &chat.attention).map(|(_, notice)| notice)
        };
        if let Some((id, run)) = &run
            && let Some(notice) = notice(id, run)
        {
            return Some(RunOutcome::Recorded {
                failed: run.state == RunState::Failed,
                notice,
                detail: run.progress.clone(),
            });
        }
        Some(match run {
            None => RunOutcome::InputsSaved,
            Some((id, run)) => match run.state {
                RunState::Working => RunOutcome::Elsewhere(id),
                RunState::Completed => RunOutcome::NoProposal,
                RunState::Failed => RunOutcome::Failed(run.progress.clone()),
                RunState::Interrupted => RunOutcome::Interrupted(run.progress.clone()),
                RunState::Cancelled => RunOutcome::Stopped(run.progress.clone()),
            },
        })
    }
    /// `writing-run-status`: the outcome's tone and the full recorded reason,
    /// wrapping, never truncated. Beside a recorded notice, the run's
    /// progress is a muted second line, `writing-run-detail` (U10).
    pub(super) fn writing_run_status(&self) -> gpui_kit::AnyElement {
        let Some(outcome) = self.writing_run_outcome() else {
            return div().into_any_element();
        };
        div()
            .flex_shrink_0()
            .px_3()
            .py_1()
            .border_t_1()
            .border_color(rgb(pal().hairline))
            .child(components::status_line(
                "writing-run-status",
                outcome.tone(),
                outcome.text(),
            ))
            .children(outcome.detail().map(|detail| {
                div()
                    .id("writing-run-detail")
                    .min_w_0()
                    .font_family(UI_FONT)
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(pal().muted))
                    .aria_label(detail.to_owned())
                    .child(detail.to_owned())
                    .test_support()
            }))
            .into_any_element()
    }
    /// The writing bar's first matching row for the open note.
    pub(super) fn writing_bar_row(&self) -> Option<WritingRow> {
        let open = self.note.as_ref()?;
        let note = open.record.id.as_str();
        let proposal = self
            .candidates
            .iter()
            .filter(|candidate| !candidate.request.writes.iter().any(|w| matches!(&w.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE))
                || self.writing_reviews.contains_key(candidate.operation.as_str()))
            .any(|candidate| self.writing_candidate_note(candidate).as_deref() == Some(note));
        let applied = self.latest_rewrite_applied(note)
            && self.applied_rewrite.get(note).is_some_and(|(request, _)| {
                self.workspace
                    .writing_request(request)
                    .is_ok_and(|snapshot| snapshot.note.id == note)
            });
        // A kept edit blocks the note (it holds no open edit then): the
        // banner's Recover edit owns the primary.
        if open.session.is_none() && self.recovery_blocked() {
            return Some(WritingRow::Recover);
        }
        Some(writing_row(
            open.session.is_some(),
            self.edit_is_stale(),
            proposal,
            applied,
            self.writing_run_outcome().as_ref(),
        ))
    }
    /// An open edit whose base version is behind the saved note.
    pub(super) fn edit_is_stale(&self) -> bool {
        self.note
            .as_ref()
            .and_then(|open| open.session.as_ref())
            .is_some_and(|session| {
                self.records
                    .iter()
                    .any(|r| r.id == session.note && r.version != session.base_version)
            })
    }
    /// A response is writing-owned only when committed with its exact bound terminal run.
    pub(super) fn writing_result_note(&self) -> Option<Record> {
        let note = self.open_note_id()?;
        if self.writing_worker_for(note)
            || self
                .candidates
                .iter()
                .filter(|candidate| !candidate.request.writes.iter().any(|w| matches!(&w.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE))
                    || self.writing_reviews.contains_key(candidate.operation.as_str()))
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
        // A capped scroller that also gives way in a short Note, so the
        // writing bar below stays whole; its label and a line stay in view.
        // It gives way before the text and an open comments section, down
        // to its label while the section is open.
        div()
            .id("writing-result")
            .flex()
            .flex_col()
            .gap_1()
            .flex_shrink(1.)
            .min_h(px(if self.comments_stacked() { 32. } else { 44. }))
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
    /// The writing bar's feedback field: one line when empty, up to four as
    /// the owner writes. Its placeholder names it; the label is announced.
    pub(super) fn overall_feedback(&self) -> gpui_kit::AnyElement {
        div()
            .id("overall-feedback")
            .pt_1()
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .child(
                Textarea::new(&self.general_feedback)
                    .small()
                    .aria_label("Overall feedback"),
            )
            .test_support()
            .into_any_element()
    }
}

/// How the last rewrite ended, when nothing newer owns the writing bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RunOutcome {
    /// Failed or Interrupted with the chat's recorded notice for this run;
    /// the run's progress is secondary detail.
    Recorded {
        failed: bool,
        notice: String,
        detail: String,
    },
    Failed(String),
    Interrupted(String),
    Stopped(String),
    /// Saved as Working with no live worker here; holds the run's ID.
    Elsewhere(String),
    /// Inputs saved, no run recorded.
    InputsSaved,
    /// Completed without a proposal.
    NoProposal,
}
impl RunOutcome {
    /// `None` is neutral.
    pub(super) fn tone(&self) -> Option<Tone> {
        match self {
            Self::Recorded { failed: true, .. } | Self::Failed(_) => Some(Tone::Error),
            Self::Recorded { .. } => Some(Tone::Warning),
            Self::Interrupted(_) | Self::Stopped(_) | Self::Elsewhere(_) => Some(Tone::Warning),
            Self::InputsSaved | Self::NoProposal => None,
        }
    }
    /// The state, then the run's recorded reason verbatim when it has one.
    pub(super) fn text(&self) -> String {
        let with = |state: &str, progress: &str| {
            if progress.trim().is_empty() {
                state.to_owned()
            } else {
                format!("{state} · {progress}")
            }
        };
        match self {
            // The recorded notice alone, as Needs you and the status line
            // read; the progress is its own line (`detail`).
            Self::Recorded { notice, .. } => notice.clone(),
            Self::Failed(progress) => with("Rewrite failed", progress),
            Self::Interrupted(progress) => with("Rewrite interrupted", progress),
            Self::Stopped(progress) => with("Rewrite stopped", progress),
            Self::Elsewhere(_) => {
                "This rewrite may still be running elsewhere. Retry starts a new attempt.".into()
            }
            Self::InputsSaved => "Rewrite inputs saved".into(),
            Self::NoProposal => "Rewrite finished · no proposal".into(),
        }
    }
    /// The run's progress beside a recorded notice, as the chat's recovery
    /// card shows it: a secondary line, only when it says something else.
    pub(super) fn detail(&self) -> Option<&str> {
        match self {
            Self::Recorded { notice, detail, .. }
                if !detail.trim().is_empty() && detail.trim() != notice.trim() =>
            {
                Some(detail.as_str())
            }
            _ => None,
        }
    }
}

/// The writing bar's first matching row (plan.md step 3), which owns the one
/// filled primary. Rows 5 and 6 come from the recorded run outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WritingRow {
    /// A kept edit blocks the note: Recover edit, in the banner.
    Recover,
    /// The note changed since the edit began: Save as new note, in the banner.
    DirtyStale,
    /// Save.
    Dirty,
    /// Review rewrite, newest proposal.
    ProposalReady,
    /// Mark comments addressed, when any are open.
    Applied,
    /// Retry saved rewrite.
    Retry,
    /// Rewrite with AI, after a run that left no proposal.
    Finished,
    /// Rewrite with AI.
    Idle,
}
pub(super) fn writing_row(
    dirty: bool,
    stale: bool,
    proposal: bool,
    applied: bool,
    outcome: Option<&RunOutcome>,
) -> WritingRow {
    match outcome {
        _ if dirty && stale => WritingRow::DirtyStale,
        _ if dirty => WritingRow::Dirty,
        _ if proposal => WritingRow::ProposalReady,
        _ if applied => WritingRow::Applied,
        Some(RunOutcome::NoProposal) => WritingRow::Finished,
        Some(_) => WritingRow::Retry,
        None => WritingRow::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_matching_writing_row_owns_the_primary() {
        let failed = RunOutcome::Failed("Provider unavailable".into());
        let done = RunOutcome::NoProposal;
        for (dirty, stale, proposal, applied, outcome, row) in [
            (
                true,
                true,
                true,
                true,
                Some(&failed),
                WritingRow::DirtyStale,
            ),
            (true, false, true, true, Some(&failed), WritingRow::Dirty),
            (false, false, true, true, None, WritingRow::ProposalReady),
            (false, false, false, true, None, WritingRow::Applied),
            (false, false, false, false, Some(&failed), WritingRow::Retry),
            (
                false,
                false,
                false,
                false,
                Some(&RunOutcome::Elsewhere("run".into())),
                WritingRow::Retry,
            ),
            (
                false,
                false,
                false,
                false,
                Some(&RunOutcome::InputsSaved),
                WritingRow::Retry,
            ),
            (
                false,
                false,
                false,
                false,
                Some(&done),
                WritingRow::Finished,
            ),
            (false, false, false, false, None, WritingRow::Idle),
        ] {
            assert_eq!(writing_row(dirty, stale, proposal, applied, outcome), row);
        }
    }

    /// U10: a recorded notice is the whole status line; progress is a
    /// separate secondary line only when it adds something.
    #[test]
    fn a_recorded_notice_leads_and_progress_is_secondary() {
        let notice = "Run failed: The provider account needs reconnecting. Continue when the provider is ready.";
        let failed = RunOutcome::Recorded {
            failed: true,
            notice: notice.into(),
            detail: "Read 2 notes".into(),
        };
        assert_eq!(failed.text(), notice);
        assert_eq!(failed.detail(), Some("Read 2 notes"));
        assert_eq!(failed.tone(), Some(Tone::Error));
        let interrupted = RunOutcome::Recorded {
            failed: false,
            notice: "Run interrupted. Continue starts fresh.".into(),
            detail: " ".into(),
        };
        assert_eq!(
            interrupted.text(),
            "Run interrupted. Continue starts fresh."
        );
        assert_eq!(interrupted.detail(), None);
        assert_eq!(interrupted.tone(), Some(Tone::Warning));
        let same = RunOutcome::Recorded {
            failed: true,
            notice: notice.into(),
            detail: notice.into(),
        };
        assert_eq!(same.text(), notice);
        assert_eq!(same.detail(), None);
        // Outcomes without a notice keep their one line.
        assert_eq!(RunOutcome::Failed("Read 2 notes".into()).detail(), None);
    }
    #[test]
    fn run_outcomes_keep_the_recorded_reason_and_their_tone() {
        let reason = "Run failed: Reconnect this account before continuing.";
        assert_eq!(
            RunOutcome::Failed(reason.into()).text(),
            format!("Rewrite failed · {reason}")
        );
        assert_eq!(RunOutcome::Failed(" ".into()).text(), "Rewrite failed");
        assert_eq!(
            RunOutcome::Stopped("Stopped by owner".into()).text(),
            "Rewrite stopped · Stopped by owner"
        );
        assert_eq!(RunOutcome::Failed(String::new()).tone(), Some(Tone::Error));
        assert_eq!(
            RunOutcome::Interrupted(String::new()).tone(),
            Some(Tone::Warning)
        );
        assert_eq!(
            RunOutcome::Elsewhere("run".into()).tone(),
            Some(Tone::Warning)
        );
        assert_eq!(RunOutcome::InputsSaved.tone(), None);
        assert_eq!(
            RunOutcome::NoProposal.text(),
            "Rewrite finished · no proposal"
        );
    }
}
