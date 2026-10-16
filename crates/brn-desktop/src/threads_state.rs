//! Small presentation rules; canonical state and authority remain in the shared core.
use brn_threads_app::{
    ApplyOutcome, ChatControl, Comment, Prepared, Receipt, Record, RecordData, Run, RunState,
    SaveOutcome, ThreadState, WorkspaceSettings,
};
use std::{collections::BTreeMap, ops::Range};

pub(super) fn delegation_status(settings: &WorkspaceSettings) -> &'static str {
    if !settings.delegation_chosen {
        "Choose how BRN maintains your working knowledge. Reading and editing remain available."
    } else if !settings.maintenance {
        "Maintenance is paused. Your saved choice will apply when you resume."
    } else if settings.review_first {
        "Review changes first is selected. Proposed changes wait for your approval."
    } else {
        "BRN may maintain ordinary notes within your delegated scope."
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub(super) enum ComposerDestination {
    Home,
    Thread(String),
    Note(String, Option<String>),
}

/// Session drafts are keyed by the same destination used when posting.
#[derive(Default)]
pub(super) struct ComposerDrafts(BTreeMap<ComposerDestination, String>);
impl ComposerDrafts {
    pub(super) fn preserve(&mut self, destination: ComposerDestination, text: String) {
        if text.is_empty() {
            self.0.remove(&destination);
        } else {
            self.0.insert(destination, text);
        }
    }
    pub(super) fn text(&self, destination: &ComposerDestination) -> &str {
        self.0.get(destination).map(String::as_str).unwrap_or("")
    }
    pub(super) fn posted(&mut self, destination: &ComposerDestination) {
        self.0.remove(destination);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WorkerKind {
    Run,
    Import,
    Export,
    Backup,
    // Only the native UI starts GitHub Copilot sign-in.
    #[cfg_attr(not(feature = "native-ui"), allow(dead_code))]
    Connect,
}
impl WorkerKind {
    pub(super) fn can_cancel(self) -> bool {
        matches!(self, Self::Run | Self::Import | Self::Connect)
    }
    pub(super) fn status(self, progress: &str) -> String {
        match self {
            Self::Run | Self::Import | Self::Connect => progress.into(),
            Self::Export => "Finishing Markdown export… Please wait for completion.".into(),
            Self::Backup => "Finishing full backup… Please wait for completion.".into(),
        }
    }
    pub(super) fn busy_notice(self) -> &'static str {
        match self {
            Self::Run => "Finish or stop the current run before starting other work.",
            Self::Import => "Finish or stop the current import before starting other work.",
            Self::Export => "Wait for the Markdown export to finish before starting other work.",
            Self::Backup => "Wait for the full backup to finish before starting other work.",
            Self::Connect => "Finish or stop GitHub Copilot sign-in before starting other work.",
        }
    }
}

/// Creation receipts, rather than IDs or a late progress write, identify the
/// latest invocation. Opening the UI never changes its persisted status.
pub(super) fn latest_run<'a>(
    records: &'a [Record],
    history: &[Receipt],
    thread: &str,
) -> Option<&'a Record> {
    history
        .iter()
        .rev()
        .flat_map(|receipt| receipt.writes.iter())
        .filter(|write| {
            write.before.is_none()
                && matches!(&write.after.data,RecordData::Run(run) if run.thread==thread)
        })
        .find_map(|write| {
            records
                .iter()
                .find(|record| record.id == write.after.id && !record.archived)
        })
}
pub(super) fn can_continue(state: &RunState, thread: &ThreadState, has_worker: bool) -> bool {
    !has_worker
        && *thread == ThreadState::Open
        && matches!(
            state,
            RunState::Working | RunState::Interrupted | RunState::Failed | RunState::Cancelled
        )
}
/// A saved owner message newer than the latest invocation is durable work even
/// if the previous run completed, or a replacement never reached preparation.
pub(super) fn can_continue_thread(
    records: &[Record],
    history: &[Receipt],
    thread: &str,
    state: &ThreadState,
    has_worker: bool,
) -> bool {
    if has_worker || *state != ThreadState::Open {
        return false;
    }
    let latest = latest_run(records, history, thread);
    if latest.is_some_and(|record| matches!(&record.data, RecordData::Run(run) if can_continue(&run.state, state, false))) {
        return true;
    }
    owner_message_after_latest_run(records, history, thread)
}
/// A saved owner message was created after the chat's latest run (or the
/// chat has an owner message and no run yet). Compares receipt order, never
/// timestamps or later Run updates.
pub(super) fn owner_message_after_latest_run(
    records: &[Record],
    history: &[Receipt],
    thread: &str,
) -> bool {
    let latest = latest_run(records, history, thread);
    for receipt in history.iter().rev() {
        if latest.is_some_and(|run| {
            receipt
                .writes
                .iter()
                .any(|write| write.before.is_none() && write.after.id == run.id)
        }) {
            return false;
        }
        if receipt.writes.iter().any(|write| {
            write.before.is_none()
                && matches!(&write.after.data, RecordData::Message(message) if message.thread == thread && message.role == "owner")
                && records.iter().any(|record| record.id == write.after.id && !record.archived)
        }) {
            return true;
        }
    }
    false
}

/// What Continue does for a chat. `continue_run` dispatches on it and the
/// chat's recovery card is gated and labelled by it, so they cannot disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ContinuePath {
    /// This chat holds the queue's pending item: Resume queue.
    ResumeQueue,
    /// The newest request after the last owner message is a rewrite; the
    /// Note's Retry saved rewrite owns it, never the chat.
    RetryRewrite(String),
    /// A fresh reply from the saved chat and current notes.
    FreshRun,
}
/// The pending item's chat first, then the newest writing request after the
/// last owner message. `messages` are the chat's messages in creation order.
pub(super) fn continue_path(
    queue: &ChatControl,
    messages: &[Record],
    thread: &str,
) -> ContinuePath {
    if queue.pending.as_ref().is_some_and(|q| q.thread == thread) {
        return ContinuePath::ResumeQueue;
    }
    for record in messages.iter().rev() {
        if let RecordData::Message(message) = &record.data {
            if message.role == "owner" {
                break;
            }
            if message.role == "writing-request" {
                return ContinuePath::RetryRewrite(record.id.clone());
            }
        }
    }
    ContinuePath::FreshRun
}

/// A recovery line's tone, ordered so the card takes the strongest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Severity {
    Neutral,
    Warning,
    Error,
}
/// The chat's recovery card, filled from recorded state; BRN never supplies
/// a cause of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Recovery {
    /// Line 1: the latest run's state, or "Not answered yet".
    pub(super) word: &'static str,
    pub(super) word_severity: Severity,
    /// The run's recorded reason, verbatim and shown whole: the chat's
    /// notice for a failed or interrupted run, otherwise its progress.
    pub(super) reason: Option<String>,
    /// The run's progress beside a notice, when it says something else.
    pub(super) detail: Option<String>,
    /// Line 2: what the button does.
    pub(super) next: String,
    pub(super) next_severity: Severity,
    /// Waiting messages that stay paused when the button does not resume.
    pub(super) waiting: Option<String>,
    /// The button: "Resume queue" or "Continue".
    pub(super) action: &'static str,
    /// False in an archived chat, where the service refuses a run.
    pub(super) available: bool,
}
impl Recovery {
    pub(super) fn severity(&self) -> Severity {
        self.word_severity.max(self.next_severity)
    }
    /// An archived chat keeps line 1 and the recorded reason, but the
    /// service refuses a run there, so Unarchive comes first and the button
    /// is unavailable.
    pub(super) fn archived(self) -> Self {
        Recovery {
            next: "Unarchive to continue.".into(),
            next_severity: Severity::Neutral,
            available: false,
            ..self
        }
    }
    /// A failed or interrupted run's reason is the chat's recorded notice
    /// for it, the text Needs you and the status line show (U10). The run's
    /// progress, the agent's own summary, stays as a secondary line when it
    /// says something else. Without a notice the card is unchanged.
    pub(super) fn with_notice(self, notice: Option<String>) -> Self {
        let Some(notice) = notice else {
            return self;
        };
        let detail = self
            .reason
            .filter(|progress| progress.trim() != notice.trim());
        Recovery {
            reason: Some(notice),
            detail,
            ..self
        }
    }
    /// Every line, as announced.
    pub(super) fn label(&self) -> String {
        [
            Some(self.word.to_owned()),
            self.reason.clone(),
            self.detail.clone(),
            Some(self.next.clone()),
            self.waiting.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
    }
}
fn messages_count(count: usize) -> String {
    format!(
        "{count} waiting message{}",
        if count == 1 { "" } else { "s" }
    )
}
/// Line 1 uses the latest run's state only when no owner message postdates
/// it (`owner_after`); line 2 follows the path and queue state.
/// `pending_working` is `resume_chat_queue`'s own check (the pending item's
/// run is saved as Working): Resume queue then stops that reply first.
pub(super) fn recovery(
    path: &ContinuePath,
    pending_working: bool,
    latest: Option<&Run>,
    owner_after: bool,
    queue: &ChatControl,
    thread: &str,
) -> Recovery {
    const NOT_ANSWERED: &str = "Not answered yet";
    let (word, word_severity) = match latest.filter(|_| !owner_after).map(|run| &run.state) {
        Some(RunState::Failed) => ("Failed", Severity::Error),
        Some(RunState::Interrupted) => ("Interrupted", Severity::Warning),
        Some(RunState::Cancelled) => ("Stopped", Severity::Warning),
        Some(RunState::Working) => ("May still be running", Severity::Warning),
        Some(RunState::Completed) | None => (NOT_ANSWERED, Severity::Neutral),
    };
    let reason = latest
        .filter(|run| word != NOT_ANSWERED && !run.progress.trim().is_empty())
        .map(|run| run.progress.clone());
    let still_running = latest.is_some_and(|run| run.state == RunState::Working);
    let (next, next_severity, action) = match path {
        ContinuePath::ResumeQueue => {
            let others = queue.queue.iter().filter(|q| q.thread != thread).count();
            let saved = if queue.paused {
                "Your message is saved and the queue is paused."
            } else {
                "Your message is saved."
            };
            let waiting = 1 + queue.queue.len();
            let order = if waiting == 1 { "" } else { " in order" };
            let including = match others {
                0 => String::new(),
                1 => ", including 1 from another chat".into(),
                others => format!(", including {others} from other chats"),
            };
            let (stops, severity) = if pending_working {
                (
                    "stops the reply that may still be running, then ",
                    Severity::Warning,
                )
            } else {
                ("", Severity::Neutral)
            };
            (
                format!(
                    "{saved} Resume queue {stops}runs {}{order}{including}.",
                    messages_count(waiting)
                ),
                severity,
                "Resume queue",
            )
        }
        _ if still_running => (
            format!(
                "{} may still be running elsewhere. Continue starts a fresh reply and replaces it.",
                if word == NOT_ANSWERED {
                    "An earlier reply"
                } else {
                    "This reply"
                }
            ),
            Severity::Warning,
            "Continue",
        ),
        _ => (
            "Your message is saved. Continue starts a fresh reply with the saved chat and current notes."
                .into(),
            Severity::Neutral,
            "Continue",
        ),
    };
    let paused = queue.queue.len();
    let waiting = (queue.paused && paused > 0 && *path != ContinuePath::ResumeQueue).then(|| {
        format!(
            "{} {} paused until you resume the queue.",
            messages_count(paused),
            if paused == 1 { "stays" } else { "stay" }
        )
    });
    Recovery {
        word,
        word_severity,
        reason,
        detail: None,
        next,
        next_severity,
        waiting,
        action,
        available: true,
    }
}

/// The notes a prepared proposal writes, and whether any is protected now or
/// would be after it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ReviewSummary {
    pub(super) note_titles: Vec<String>,
    pub(super) protected: bool,
    pub(super) other_writes: usize,
}
pub(super) fn review_summary(candidate: &Prepared, records: &[Record]) -> ReviewSummary {
    let mut summary = ReviewSummary::default();
    for write in &candidate.request.writes {
        let RecordData::Note(proposed) = &write.data else {
            summary.other_writes += 1;
            continue;
        };
        let current = records.iter().find(|record| record.id == write.id);
        summary.protected |= proposed.protected
            || current.is_some_and(
                |record| matches!(&record.data, RecordData::Note(note) if note.protected),
            );
        summary
            .note_titles
            .push(if proposed.title.trim().is_empty() {
                "Untitled note".into()
            } else {
                proposed.title.clone()
            });
    }
    summary
}
/// "Review changes to A", "… to A and B", "… to A, B and N more", or for a
/// proposal that writes no note "Review N changes · {reason}".
pub(super) fn review_label(summary: &ReviewSummary, reason: &str) -> String {
    match summary.note_titles.as_slice() {
        [] => format!(
            "Review {} change{} · {reason}",
            summary.other_writes,
            if summary.other_writes == 1 { "" } else { "s" }
        ),
        [a] => format!("Review changes to {a}"),
        [a, b] => format!("Review changes to {a} and {b}"),
        [a, b, rest @ ..] => format!("Review changes to {a}, {b} and {} more", rest.len()),
    }
}
/// Names where to follow up after a change that left derived work behind.
pub(super) fn refresh_hint(count: usize) -> String {
    format!(
        "{count} related item{} may need refresh. Open Changes to review {}.",
        if count == 1 { "" } else { "s" },
        if count == 1 { "it" } else { "them" }
    )
}
/// Whether a change stopped work that was running: it moved a Working run
/// to another state (Settings fence and cancel running replies).
fn stopped_running_work(receipt: &Receipt) -> bool {
    receipt.writes.iter().any(|write| {
        matches!(&write.before, Some(Record { data: RecordData::Run(before), .. }) if before.state == RunState::Working)
            && matches!(&write.after.data, RecordData::Run(after) if after.state != RunState::Working)
    })
}
/// An applied change in the owner's terms. The recorded reason is service
/// wording for Changes, never notice copy: "Saved." or "Undone.", whether
/// running work stopped, then the refresh follow-up, each its own sentence.
pub(super) fn applied_message(receipt: &Receipt) -> String {
    let mut text = if receipt.undo_of.is_some() {
        "Undone.".to_owned()
    } else {
        "Saved.".to_owned()
    };
    if stopped_running_work(receipt) {
        text.push_str(" Running work was stopped.");
    }
    if !receipt.needs_refresh.is_empty() {
        text.push(' ');
        text.push_str(&refresh_hint(receipt.needs_refresh.len()));
    }
    text
}
pub(super) fn outcome_message(outcome: &ApplyOutcome) -> String {
    match outcome {
        ApplyOutcome::Applied(receipt) => applied_message(receipt),
        ApplyOutcome::Deferred { .. } => "Waiting for an open edit. Save or discard that edit, then retry this change.".into(),
        ApplyOutcome::Stale { .. } => "The current content changed. This proposal was preserved; compare it with the latest content before preparing a new change.".into(),
        ApplyOutcome::NeedsReview { .. } => "This change needs your review. The current notes are unchanged.".into(),
        ApplyOutcome::Superseded { .. } => "This run stopped because newer work took its place.".into(),
    }
}
pub(super) fn save_message(outcome: &SaveOutcome) -> &'static str {
    match outcome {
        SaveOutcome::Saved(_) => "Saved. Your edit is in Changes.",
        SaveOutcome::Stale(_) => {
            "The note changed while you were editing. Your text is preserved. Compare the current note or save your text as a new note."
        }
        SaveOutcome::GenerationMismatch(_) => {
            "A newer edit is preserved. Refresh the recovery buffer before saving."
        }
        SaveOutcome::Closed(_) => {
            "This edit is already closed. Your visible text is retained until you choose another note."
        }
        SaveOutcome::Deferred { .. } => {
            "Another edit is open for this note. Your text is preserved; finish the other edit before retrying Save."
        }
    }
}
/// Why a comment has no place in the shown version, worded for what is
/// true: the service unresolved it (its passage was edited away), or it is
/// still tied to the earlier version it was written on and nothing
/// re-anchored it. `status` is the card's line; `reason` follows
/// "unavailable:" in labels and tooltips.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Unlocated {
    pub(super) status: String,
    pub(super) reason: String,
}
pub(super) fn unlocated_reason(comment: &Comment, version: u64) -> Unlocated {
    let tied = comment.mapped_version.unwrap_or(comment.base_version);
    let next = "Exclude it, or select the passage and comment again.";
    if !comment.unresolved && tied != version {
        Unlocated {
            status: format!("Tied to v{tied}, not re-anchored to v{version}. {next}"),
            reason: format!("the comment is tied to v{tied}, not v{version}"),
        }
    } else {
        Unlocated {
            status: format!("Passage not found in v{version}. {next}"),
            reason: format!("the passage is not in v{version}"),
        }
    }
}
/// Only an exact supported mapping supplies a current location. Repeated quotes
/// and changed revisions never trigger a guessed relocation.
pub(super) fn comment_location(
    comment: &Comment,
    version: u64,
    text: &str,
) -> Option<Range<usize>> {
    brn_threads_app::exact_writing_range(comment, version, text).map(|(start, end)| start..end)
}
pub(super) fn linked(records: &[Record], id: &str) -> Vec<String> {
    let mut found = Vec::new();
    for record in records.iter().filter(|r| !r.archived) {
        if let RecordData::Link(link) = &record.data {
            let target = if link.from == id {
                Some(&link.to)
            } else if link.to == id {
                Some(&link.from)
            } else {
                None
            };
            if let Some(target) = target
                && !found.contains(target)
            {
                found.push(target.clone());
            }
        }
    }
    found
}
/// Navigation only, never a second source-of-truth representation of Markdown.
pub(super) fn sections(text: &str) -> Vec<(String, Range<usize>)> {
    let mut offset = 0;
    let mut fence: Option<(char, usize)> = None;
    let mut out = Vec::new();
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let marker = trimmed.chars().next().unwrap_or(' ');
        let run = trimmed.chars().take_while(|c| *c == marker).count();
        if (marker == '`' || marker == '~') && run >= 3 {
            if fence.is_some_and(|(c, n)| c == marker && run >= n) {
                fence = None;
            } else if fence.is_none() {
                fence = Some((marker, run));
            }
        } else if fence.is_none() {
            let hashes = line.chars().take_while(|c| *c == '#').count();
            if (1..=6).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ') {
                let title = line[hashes + 1..]
                    .trim()
                    .trim_end_matches('#')
                    .trim()
                    .to_owned();
                if !title.is_empty() {
                    out.push((title, offset + hashes + 1..offset + line.trim_end().len()));
                }
            }
        }
        offset += line.len();
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_display_does_not_infer_consent_from_legacy_maintenance_flags() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let workspace = brn_threads_app::Workspace::open(root.path()).unwrap();
        let mut settings = workspace.settings().unwrap();
        settings.maintenance = true;
        settings.review_first = false;
        assert!(delegation_status(&settings).starts_with("Choose how"));
        settings.review_first = true;
        assert!(delegation_status(&settings).starts_with("Choose how"));
        settings.delegation_chosen = true;
        assert!(delegation_status(&settings).starts_with("Review changes first"));
        settings.review_first = false;
        assert!(delegation_status(&settings).contains("delegated scope"));
        settings.maintenance = false;
        assert!(delegation_status(&settings).contains("paused"));
    }
    #[test]
    fn posting_clears_only_the_submitted_destination_draft() {
        let mut drafts = ComposerDrafts::default();
        let a = ComposerDestination::Thread("A".into());
        let b = ComposerDestination::Thread("B".into());
        drafts.preserve(ComposerDestination::Home, "Home õun".into());
        drafts.preserve(a.clone(), "A draft".into());
        drafts.preserve(b.clone(), "B draft".into());
        drafts.posted(&a);
        assert_eq!(drafts.text(&a), "");
        assert_eq!(drafts.text(&b), "B draft");
        assert_eq!(drafts.text(&ComposerDestination::Home), "Home õun");
        drafts.preserve(b.clone(), "".into());
        assert_eq!(drafts.text(&b), "");
    }
    #[test]
    fn document_drafts_are_isolated_from_home_and_other_documents() {
        let mut drafts = ComposerDrafts::default();
        let a = ComposerDestination::Note("A".into(), None);
        let b = ComposerDestination::Note("B".into(), None);
        drafts.preserve(ComposerDestination::Home, "Home".into());
        drafts.preserve(a.clone(), "Document A õun".into());
        drafts.preserve(b.clone(), "Document B".into());
        drafts.posted(&a);
        assert_eq!(drafts.text(&a), "");
        assert_eq!(drafts.text(&b), "Document B");
        assert_eq!(drafts.text(&ComposerDestination::Home), "Home");
    }
    #[test]
    fn unresolved_or_partial_mapping_never_enables_current_passage() {
        let mut comment = Comment {
            note: "n".into(),
            base_version: 1,
            quote: "õun".into(),
            range: Some((0, 4)),
            mapped_version: None,
            mapped_range: None,
            body: "Feedback".into(),
            unresolved: false,
        };
        assert_eq!(comment_location(&comment, 1, "õun"), Some(0..4));
        comment.unresolved = true;
        assert_eq!(comment_location(&comment, 1, "õun"), None);
        comment.unresolved = false;
        comment.mapped_version = Some(1);
        assert_eq!(comment_location(&comment, 1, "õun"), None);
        comment.mapped_version = None;
        comment.mapped_range = Some((0, 4));
        assert_eq!(comment_location(&comment, 1, "õun"), None);
    }
    #[test]
    fn only_cancellable_workers_offer_stop_and_portable_outputs_explain_finishing() {
        for kind in [WorkerKind::Run, WorkerKind::Import] {
            assert!(kind.can_cancel());
            assert_eq!(kind.status("Actual progress"), "Actual progress");
            assert!(kind.busy_notice().contains("stop"));
        }
        for kind in [WorkerKind::Export, WorkerKind::Backup] {
            assert!(!kind.can_cancel());
            assert!(kind.status("Actual progress").starts_with("Finishing"));
            assert!(kind.busy_notice().starts_with("Wait for"));
            assert!(!kind.busy_notice().contains("stop"));
        }
    }
    #[test]
    fn saved_working_run_can_continue_only_without_a_local_worker_and_in_an_open_thread() {
        assert!(can_continue(&RunState::Working, &ThreadState::Open, false));
        assert!(!can_continue(&RunState::Working, &ThreadState::Open, true));
        assert!(!can_continue(
            &RunState::Working,
            &ThreadState::Resolved,
            false
        ));
        assert!(!can_continue(
            &RunState::Completed,
            &ThreadState::Open,
            false
        ));
        assert!(can_continue(
            &RunState::Interrupted,
            &ThreadState::Open,
            false
        ));
        // The saved Working run's card says it may still run elsewhere and
        // that Continue replaces it.
        let working = run(RunState::Working, "");
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&working),
            false,
            &ChatControl::default(),
            "chat",
        );
        assert!(card.next.contains("running elsewhere"));
        assert!(card.next.contains("replaces it"));
    }

    fn run(state: RunState, progress: &str) -> Run {
        Run {
            instruction: None,
            queue_claim: None,
            thread: "chat".into(),
            provider: "offline-fixture".into(),
            model: "synthetic-model".into(),
            guide_identity: "synthetic".into(),
            budget: 1,
            effort: "medium".into(),
            loaded_skills: vec![],
            fence: 0,
            state,
            progress: progress.into(),
            operations: vec![],
            sources: vec![],
        }
    }
    fn message(id: &str, thread: &str, role: &str) -> Record {
        Record {
            id: id.into(),
            version: 1,
            archived: false,
            data: RecordData::Message(brn_threads_app::Message {
                thread: thread.into(),
                role: role.into(),
                text: format!("{role} text"),
            }),
        }
    }
    fn queued(id: &str, thread: &str) -> brn_threads_app::QueuedMessage {
        brn_threads_app::QueuedMessage {
            id: id.into(),
            thread: thread.into(),
            text: "Waiting text".into(),
            message: None,
            run: None,
            dispatch: None,
        }
    }

    #[test]
    fn continue_path_checks_the_pending_chat_then_the_newest_request() {
        let rewrite = vec![
            message("m1", "chat", "owner"),
            message("w1", "chat", "writing-request"),
            message("a1", "chat", "assistant"),
        ];
        let idle = ChatControl::default();
        assert_eq!(
            continue_path(&idle, &rewrite, "chat"),
            ContinuePath::RetryRewrite("w1".into())
        );
        // An owner message after the request makes it an ordinary reply.
        let mut answered = rewrite.clone();
        answered.push(message("m2", "chat", "owner"));
        assert_eq!(
            continue_path(&idle, &answered, "chat"),
            ContinuePath::FreshRun
        );
        assert_eq!(continue_path(&idle, &[], "chat"), ContinuePath::FreshRun);
        // The pending item's chat wins over a newer rewrite request.
        let pending = ChatControl {
            pending: Some(queued("q1", "chat")),
            ..ChatControl::default()
        };
        assert_eq!(
            continue_path(&pending, &rewrite, "chat"),
            ContinuePath::ResumeQueue
        );
        assert_eq!(
            continue_path(&pending, &rewrite, "other"),
            ContinuePath::RetryRewrite("w1".into())
        );
    }

    /// U10: the chat's recorded notice is the card's reason; the run's
    /// progress becomes a secondary line only when it says something else.
    #[test]
    fn a_recorded_notice_is_the_recovery_reason_and_progress_is_detail() {
        let notice = "Run failed: The provider account needs reconnecting. Continue when the provider is ready.";
        let idle = ChatControl::default();
        let failed = run(RunState::Failed, "Read the delivery note");
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&failed),
            false,
            &idle,
            "chat",
        )
        .with_notice(Some(notice.into()));
        assert_eq!(card.reason.as_deref(), Some(notice));
        assert_eq!(card.detail.as_deref(), Some("Read the delivery note"));
        assert_eq!(
            card.label(),
            format!("Failed · {notice} · Read the delivery note · {}", card.next)
        );
        // Progress that repeats the notice, or none, adds no line.
        let same = run(
            RunState::Interrupted,
            "Run interrupted. Continue starts fresh.",
        );
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&same),
            false,
            &idle,
            "chat",
        )
        .with_notice(Some("Run interrupted. Continue starts fresh.".into()));
        assert_eq!(card.detail, None);
        let empty = run(RunState::Interrupted, "");
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&empty),
            false,
            &idle,
            "chat",
        )
        .with_notice(Some("Run interrupted. Continue starts fresh.".into()));
        assert_eq!(
            (card.reason.as_deref(), card.detail),
            (Some("Run interrupted. Continue starts fresh."), None)
        );
        // Without a notice (the owner's own Stop records none) the progress
        // stays the reason.
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&failed),
            false,
            &idle,
            "chat",
        )
        .with_notice(None);
        assert_eq!(
            (card.reason.as_deref(), card.detail),
            (Some("Read the delivery note"), None)
        );
    }

    #[test]
    fn recovery_cards_state_the_recorded_reason_and_what_the_button_does() {
        let reason = "Synthetic recorded reason that BRN shows whole.";
        let idle = ChatControl::default();
        let failed = run(RunState::Failed, reason);
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&failed),
            false,
            &idle,
            "chat",
        );
        assert_eq!(card.word, "Failed");
        assert_eq!(card.severity(), Severity::Error);
        assert_eq!(card.reason.as_deref(), Some(reason));
        assert_eq!(
            card.next,
            "Your message is saved. Continue starts a fresh reply with the saved chat and current notes."
        );
        assert_eq!(card.waiting, None);
        assert_eq!(card.action, "Continue");
        assert_eq!(card.label(), format!("Failed · {reason} · {}", card.next));
        for (state, word, severity) in [
            (RunState::Interrupted, "Interrupted", Severity::Warning),
            (RunState::Cancelled, "Stopped", Severity::Warning),
            (RunState::Working, "May still be running", Severity::Warning),
        ] {
            let saved = run(state, "");
            let card = recovery(
                &ContinuePath::FreshRun,
                false,
                Some(&saved),
                false,
                &idle,
                "chat",
            );
            assert_eq!((card.word, card.word_severity), (word, severity));
            assert_eq!(card.reason, None, "an empty reason is omitted");
        }
        // A newer owner message: Not answered yet, neutral, no reason.
        let completed = run(RunState::Completed, "Done");
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&completed),
            true,
            &idle,
            "chat",
        );
        assert_eq!(card.word, "Not answered yet");
        assert_eq!(card.severity(), Severity::Neutral);
        assert_eq!(card.reason, None);
        assert!(
            card.next
                .starts_with("Your message is saved. Continue starts")
        );
        // With an older run still Working the card warns.
        let working = run(RunState::Working, "Started elsewhere");
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&working),
            true,
            &idle,
            "chat",
        );
        assert_eq!(card.word, "Not answered yet");
        assert_eq!(card.reason, None);
        assert_eq!(
            card.next,
            "An earlier reply may still be running elsewhere. Continue starts a fresh reply and replaces it."
        );
        assert_eq!(card.severity(), Severity::Warning);
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&working),
            false,
            &idle,
            "chat",
        );
        assert_eq!(card.reason.as_deref(), Some("Started elsewhere"));
        assert!(
            card.next
                .starts_with("This reply may still be running elsewhere.")
        );
        assert_eq!(card.severity(), Severity::Warning);
        // No run yet.
        let card = recovery(&ContinuePath::FreshRun, false, None, true, &idle, "chat");
        assert_eq!(card.word, "Not answered yet");
    }

    #[test]
    fn recovery_cards_count_the_queue() {
        let failed = run(RunState::Failed, "Synthetic failure");
        // Shot 29: Failed, no pending item, paused, one queued message.
        let paused = ChatControl {
            queue: vec![queued("q1", "chat")],
            paused: true,
            ..ChatControl::default()
        };
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&failed),
            false,
            &paused,
            "chat",
        );
        assert_eq!(card.action, "Continue");
        assert_eq!(
            card.waiting.as_deref(),
            Some("1 waiting message stays paused until you resume the queue.")
        );
        let two = ChatControl {
            queue: vec![queued("q1", "chat"), queued("q2", "other")],
            ..paused.clone()
        };
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&failed),
            false,
            &two,
            "chat",
        );
        assert_eq!(
            card.waiting.as_deref(),
            Some("2 waiting messages stay paused until you resume the queue.")
        );
        // Not paused: nothing stays paused.
        let running = ChatControl {
            paused: false,
            ..two.clone()
        };
        let card = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&failed),
            false,
            &running,
            "chat",
        );
        assert_eq!(card.waiting, None);
        // Resume queue counts the pending item and names other chats.
        let pending = ChatControl {
            pending: Some(queued("p", "chat")),
            ..two.clone()
        };
        let working = run(RunState::Working, "");
        // The pending item's run is saved as Working: Resume queue stops it
        // first, and line 2 says so in the warning tone.
        let card = recovery(
            &ContinuePath::ResumeQueue,
            true,
            Some(&working),
            false,
            &pending,
            "chat",
        );
        assert_eq!(card.action, "Resume queue");
        assert_eq!(card.word, "May still be running");
        assert_eq!(
            card.next,
            "Your message is saved and the queue is paused. Resume queue stops the reply that may still be running, then runs 3 waiting messages in order, including 1 from another chat."
        );
        assert_eq!(card.next_severity, Severity::Warning);
        assert_eq!(card.waiting, None, "Resume queue runs them");
        // Without a Working pending run nothing is stopped.
        let card = recovery(
            &ContinuePath::ResumeQueue,
            false,
            Some(&working),
            false,
            &pending,
            "chat",
        );
        assert_eq!(
            card.next,
            "Your message is saved and the queue is paused. Resume queue runs 3 waiting messages in order, including 1 from another chat."
        );
        assert_eq!(card.next_severity, Severity::Neutral);
        let alone = ChatControl {
            pending: Some(queued("p", "chat")),
            ..ChatControl::default()
        };
        let card = recovery(
            &ContinuePath::ResumeQueue,
            false,
            Some(&failed),
            false,
            &alone,
            "chat",
        );
        assert_eq!(
            card.next,
            "Your message is saved. Resume queue runs 1 waiting message."
        );
        let card = recovery(
            &ContinuePath::ResumeQueue,
            true,
            Some(&working),
            false,
            &alone,
            "chat",
        );
        assert_eq!(
            card.next,
            "Your message is saved. Resume queue stops the reply that may still be running, then runs 1 waiting message."
        );
        let others = ChatControl {
            queue: vec![queued("q1", "a"), queued("q2", "b"), queued("q3", "chat")],
            ..alone
        };
        let card = recovery(
            &ContinuePath::ResumeQueue,
            false,
            Some(&failed),
            false,
            &others,
            "chat",
        );
        assert!(
            card.next
                .ends_with("runs 4 waiting messages in order, including 2 from other chats.")
        );
    }

    #[test]
    fn an_archived_chat_keeps_the_recorded_reason_and_waits_for_unarchive() {
        let reason = "Synthetic recorded reason kept while archived.";
        let failed = run(RunState::Failed, reason);
        let paused = ChatControl {
            queue: vec![queued("q1", "other")],
            paused: true,
            ..ChatControl::default()
        };
        let open = recovery(
            &ContinuePath::FreshRun,
            false,
            Some(&failed),
            false,
            &paused,
            "chat",
        );
        assert!(open.available);
        let card = open.clone().archived();
        // Line 1 and the reason stay; line 2 and the button follow Unarchive.
        assert_eq!(
            (card.word, card.reason.as_deref()),
            ("Failed", Some(reason))
        );
        assert_eq!(card.next, "Unarchive to continue.");
        assert_eq!(card.next_severity, Severity::Neutral);
        assert_eq!(card.severity(), Severity::Error);
        assert_eq!(card.waiting, open.waiting);
        assert!(!card.available);
        assert_eq!(
            card.label(),
            format!(
                "Failed · {reason} · Unarchive to continue. · 1 waiting message stays paused until you resume the queue."
            )
        );
        // A Resume queue card waits for Unarchive too, without the stop
        // clause's warning.
        let pending = ChatControl {
            pending: Some(queued("p", "chat")),
            ..paused
        };
        let working = run(RunState::Working, "");
        let card = recovery(
            &ContinuePath::ResumeQueue,
            true,
            Some(&working),
            true,
            &pending,
            "chat",
        )
        .archived();
        assert_eq!(card.word, "Not answered yet");
        assert_eq!(card.next, "Unarchive to continue.");
        assert_eq!(card.severity(), Severity::Neutral);
        assert!(!card.available);
    }

    #[test]
    fn review_labels_name_every_target_note_and_mark_protection() {
        let note = |id: &str, title: &str, protected: bool| Record {
            id: id.into(),
            version: 1,
            archived: false,
            data: RecordData::Note(brn_threads_app::Note {
                protected,
                ..brn_threads_app::Note::working(title, "text")
            }),
        };
        let put = |record: &Record| brn_threads_app::Put {
            id: record.id.clone(),
            expected_version: Some(record.version),
            archived: false,
            data: record.data.clone(),
        };
        let candidate = |writes: Vec<brn_threads_app::Put>| Prepared {
            operation: serde_json::from_str("\"synthetic-op\"").unwrap(),
            request_hash: String::new(),
            request: brn_threads_app::ChangeRequest {
                reason: "Review prepared changes".into(),
                writes,
                inputs: vec![],
            },
        };
        let decision = note("d", "Delivery decision: 22 November", true);
        let survey = note("s", "Survey", false);
        // Protection comes from the current record even if the proposal
        // drops it.
        let unprotected = note("d", "Delivery decision: 22 November", false);
        let records = vec![decision.clone(), survey.clone()];
        let summary = review_summary(&candidate(vec![put(&unprotected), put(&survey)]), &records);
        assert_eq!(
            summary,
            ReviewSummary {
                note_titles: vec!["Delivery decision: 22 November".into(), "Survey".into()],
                protected: true,
                other_writes: 0,
            }
        );
        assert_eq!(
            review_label(&summary, "Review prepared changes"),
            "Review changes to Delivery decision: 22 November and Survey"
        );
        // A proposal that protects a new note is protected too.
        let fresh = note("n", "New decision", true);
        assert!(review_summary(&candidate(vec![put(&fresh)]), &[]).protected);
        let ordinary = review_summary(&candidate(vec![put(&survey)]), &records);
        assert!(!ordinary.protected);
        assert_eq!(review_label(&ordinary, "x"), "Review changes to Survey");
        let many = ReviewSummary {
            note_titles: vec!["A".into(), "B".into(), "C".into(), "D".into()],
            ..ReviewSummary::default()
        };
        assert_eq!(
            review_label(&many, "x"),
            "Review changes to A, B and 2 more"
        );
        let tasks = ReviewSummary {
            other_writes: 2,
            ..ReviewSummary::default()
        };
        assert_eq!(
            review_label(&tasks, "Review prepared changes"),
            "Review 2 changes · Review prepared changes"
        );
        let task = ReviewSummary {
            other_writes: 1,
            ..ReviewSummary::default()
        };
        assert_eq!(
            review_label(&task, "Close task"),
            "Review 1 change · Close task"
        );
    }
    #[test]
    fn linked_work_is_bidirectional_unique_and_excludes_archived_connections() {
        use brn_threads_app::Link;
        let record = |id: &str, from: &str, to: &str, archived| Record {
            id: id.into(),
            version: 1,
            archived,
            data: RecordData::Link(Link {
                from: from.into(),
                to: to.into(),
                relation: "context".into(),
            }),
        };
        let records = vec![
            record("l1", "thread", "note", false),
            record("l2", "note", "thread", false),
            record("l3", "thread", "old", true),
        ];
        assert_eq!(linked(&records, "thread"), vec!["note"]);
        assert_eq!(linked(&records, "note"), vec!["thread"]);
    }
    #[test]
    fn repeated_quote_anchors_only_the_selected_occurrence_and_never_relocates() {
        let comment = Comment {
            note: "n".into(),
            base_version: 1,
            quote: "tere".into(),
            range: Some((5, 9)),
            mapped_version: None,
            mapped_range: None,
            body: "second".into(),
            unresolved: false,
        };
        assert_eq!(comment_location(&comment, 1, "tere tere"), Some(5..9));
        assert_eq!(comment_location(&comment, 2, "tere tere"), None);
        assert_eq!(comment_location(&comment, 1, "tere ära"), None);
    }
    #[test]
    fn mapped_comment_requires_exact_revision_and_utf8_quote() {
        let comment = Comment {
            note: "n".into(),
            base_version: 1,
            quote: "õun".into(),
            range: Some((0, 4)),
            mapped_version: Some(2),
            mapped_range: Some((2, 6)),
            body: "fruit".into(),
            unresolved: false,
        };
        assert_eq!(comment_location(&comment, 2, "x õun"), Some(2..6));
        assert_eq!(comment_location(&comment, 3, "x õun"), None);
    }
    #[test]
    fn headings_in_fences_do_not_become_sections() {
        assert_eq!(
            sections("# Üks\n```\n# fake\n```\n## Kaks\n")
                .iter()
                .map(|(s, _)| s.as_str())
                .collect::<Vec<_>>(),
            vec!["Üks", "Kaks"]
        );
    }
    #[test]
    fn stale_and_deferred_changes_have_distinct_non_success_copy() {
        assert!(
            outcome_message(&ApplyOutcome::Stale {
                records: vec!["n".into()]
            })
            .contains("preserved")
        );
        assert!(
            outcome_message(&ApplyOutcome::Deferred {
                guarded: vec!["n".into()]
            })
            .contains("Waiting")
        );
        assert!(
            save_message(&SaveOutcome::Deferred {
                guarded: vec!["n".into()]
            })
            .contains("preserved")
        );
    }
    #[test]
    fn saved_notices_name_changes_and_the_refresh_follow_up() {
        let receipt = |needs_refresh: Vec<String>| Receipt {
            operation: serde_json::from_str("\"synthetic-op\"").unwrap(),
            request_hash: String::new(),
            reason: "Owner edit.".into(),
            writes: vec![],
            authority: String::new(),
            committed_at: String::new(),
            undo_of: None,
            needs_refresh,
        };
        // The recorded reason ("Owner edit.") stays in Changes; the notice
        // is the owner's terms, one sentence each.
        assert_eq!(
            outcome_message(&ApplyOutcome::Applied(receipt(vec![
                "a".into(),
                "b".into()
            ]))),
            "Saved. 2 related items may need refresh. Open Changes to review them."
        );
        assert_eq!(
            outcome_message(&ApplyOutcome::Applied(receipt(vec![]))),
            "Saved."
        );
        let mut undo = receipt(vec!["a".into()]);
        undo.undo_of = Some(serde_json::from_str("\"synthetic-original\"").unwrap());
        assert_eq!(
            outcome_message(&ApplyOutcome::Applied(undo)),
            "Undone. 1 related item may need refresh. Open Changes to review it."
        );
        // Settings that stopped a running reply say so; nothing else does.
        let run = |state: RunState| Record {
            id: "run".into(),
            version: 1,
            archived: false,
            data: RecordData::Run(Run {
                instruction: None,
                queue_claim: None,
                thread: "chat".into(),
                provider: "offline-fixture".into(),
                model: "model".into(),
                guide_identity: "guide".into(),
                budget: 1,
                effort: "medium".into(),
                loaded_skills: vec![],
                fence: 1,
                state,
                progress: String::new(),
                operations: vec![],
                sources: vec![],
            }),
        };
        let mut configured = receipt(vec![]);
        configured.reason = "Owner configured workspace and stopped prior invocations".into();
        assert_eq!(
            outcome_message(&ApplyOutcome::Applied(configured.clone())),
            "Saved."
        );
        configured.writes = vec![brn_threads_app::Written {
            before: Some(run(RunState::Working)),
            after: run(RunState::Cancelled),
        }];
        assert_eq!(
            outcome_message(&ApplyOutcome::Applied(configured)),
            "Saved. Running work was stopped."
        );
        assert_eq!(
            save_message(&SaveOutcome::Saved(receipt(vec![]))),
            "Saved. Your edit is in Changes."
        );
    }
    #[test]
    fn unlocated_comments_say_what_is_true() {
        let comment = |base_version: u64, mapped: Option<u64>, unresolved: bool| Comment {
            note: "note".into(),
            base_version,
            quote: "quick".into(),
            range: Some((4, 9)),
            mapped_version: mapped,
            mapped_range: mapped.map(|_| (4, 9)),
            body: "Sharper".into(),
            unresolved,
        };
        // Written on v1 and never re-anchored (Apply then Undo made v3).
        assert_eq!(
            unlocated_reason(&comment(1, None, false), 3),
            Unlocated {
                status: "Tied to v1, not re-anchored to v3. Exclude it, or select the passage and comment again.".into(),
                reason: "the comment is tied to v1, not v3".into(),
            }
        );
        assert_eq!(
            unlocated_reason(&comment(1, Some(2), false), 3).reason,
            "the comment is tied to v2, not v3"
        );
        // The service resolved its edit and the passage is gone.
        assert_eq!(
            unlocated_reason(&comment(1, Some(2), true), 2),
            Unlocated {
                status:
                    "Passage not found in v2. Exclude it, or select the passage and comment again."
                        .into(),
                reason: "the passage is not in v2".into(),
            }
        );
    }
    #[test]
    fn refresh_follow_ups_name_changes() {
        assert_eq!(
            refresh_hint(1),
            "1 related item may need refresh. Open Changes to review it."
        );
        assert_eq!(
            refresh_hint(3),
            "3 related items may need refresh. Open Changes to review them."
        );
    }
}

#[cfg(test)]
mod integrity_tests {
    use brn_threads_app::*;

    fn change_note(workspace: &mut Workspace, record: &Record, markdown: &str) -> ApplyOutcome {
        let RecordData::Note(mut note) = record.data.clone() else {
            unreachable!()
        };
        note.markdown = markdown.into();
        workspace
            .owner_change(
                &format!("test:{}", uuid::Uuid::new_v4()),
                &ChangeRequest {
                    reason: "Current note update".into(),
                    writes: vec![Put {
                        id: record.id.clone(),
                        expected_version: Some(record.version),
                        archived: false,
                        data: RecordData::Note(note),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap()
    }
    #[test]
    fn reading_stays_clean_first_edit_defers_changes_and_save_closes_guard() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let record = workspace.new_note("Working", "base").unwrap();
        workspace.store.note(&record.id).unwrap();
        assert!(workspace.store.recovery_buffers().unwrap().is_empty());
        let session = workspace
            .store
            .begin_edit(&record.id, record.version)
            .unwrap();
        let BufferOutcome::Updated(session) = workspace
            .store
            .update_buffer(&session.id, 1, "owner text")
            .unwrap()
        else {
            unreachable!()
        };
        assert!(matches!(
            change_note(&mut workspace, &record, "other change"),
            ApplyOutcome::Deferred { .. }
        ));
        assert!(matches!(
            workspace
                .store
                .save(&session.id, session.generation)
                .unwrap(),
            SaveOutcome::Saved(_)
        ));
        assert!(workspace.store.recovery_buffers().unwrap().is_empty());
        let current = workspace.store.note(&record.id).unwrap().unwrap();
        let RecordData::Note(note) = current.data else {
            unreachable!()
        };
        assert_eq!(note.markdown, "owner text");
    }
    #[test]
    fn stale_save_preserves_full_typed_buffer_across_restart() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let record = workspace.new_note("Working", "base").unwrap();
        assert!(matches!(
            change_note(&mut workspace, &record, "new current"),
            ApplyOutcome::Applied(_)
        ));
        let session = workspace
            .store
            .begin_edit(&record.id, record.version)
            .unwrap();
        workspace
            .store
            .update_buffer(&session.id, 1, "typed õun\n| A | B |\n")
            .unwrap();
        let SaveOutcome::Stale(stale) = workspace.store.save(&session.id, 1).unwrap() else {
            panic!("must preserve stale edit")
        };
        assert_eq!(stale.markdown, "typed õun\n| A | B |\n");
        drop(workspace);
        let workspace = Workspace::open(directory.path()).unwrap();
        assert_eq!(workspace.store.recovery_buffers().unwrap()[0], stale);
        let RecordData::Note(note) = workspace.store.note(&record.id).unwrap().unwrap().data else {
            unreachable!()
        };
        assert_eq!(note.markdown, "new current");
    }
    #[test]
    fn delayed_generation_cannot_discard_or_replace_a_newer_edit() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let record = workspace.new_note("Working", "base").unwrap();
        let session = workspace
            .store
            .begin_edit(&record.id, record.version)
            .unwrap();
        workspace
            .store
            .update_buffer(&session.id, 1, "first")
            .unwrap();
        workspace
            .store
            .update_buffer(&session.id, 2, "newer")
            .unwrap();
        assert!(matches!(
            workspace.store.save(&session.id, 1).unwrap(),
            SaveOutcome::GenerationMismatch(_)
        ));
        assert!(matches!(
            workspace.store.discard(&session.id, 1).unwrap(),
            BufferOutcome::Ignored(_)
        ));
        assert!(matches!(
            workspace
                .store
                .update_buffer(&session.id, 1, "late")
                .unwrap(),
            BufferOutcome::Ignored(_)
        ));
        assert_eq!(
            workspace.store.edit_session(&session.id).unwrap().markdown,
            "newer"
        );
    }
    #[test]
    fn newest_run_uses_creation_order_without_cancelling_saved_working_state() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let thread = workspace.new_thread("Recover work").unwrap();
        let settings = workspace.settings().unwrap();
        let mut newest = String::new();
        for (key, state) in [
            ("old-run", RunState::Failed),
            ("new-run", RunState::Working),
        ] {
            let op = workspace.store.allocate_operation(key).unwrap();
            newest = op.creation_id(0);
            workspace
                .owner_change(
                    key,
                    &ChangeRequest {
                        reason: "Host invocation fixture".into(),
                        writes: vec![Put {
                            id: newest.clone(),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Run(Run {
                                instruction: None,
                                queue_claim: None,
                                thread: thread.id.clone(),
                                provider: settings.provider.clone(),
                                model: settings.model.clone(),
                                guide_identity: "test".into(),
                                budget: 1,
                                effort: settings.effort.clone(),
                                loaded_skills: vec![],
                                fence: 0,
                                state,
                                progress: "Saved progress".into(),
                                operations: vec![],
                                sources: vec![],
                            }),
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
        }
        let records = workspace.records().unwrap();
        let history = workspace.history().unwrap();
        let latest = super::latest_run(&records, &history, &thread.id).unwrap();
        assert_eq!(latest.id, newest);
        let RecordData::Run(run) = &latest.data else {
            unreachable!()
        };
        assert_eq!(run.state, RunState::Working);
        assert!(super::can_continue(&run.state, &ThreadState::Open, false));
        let mut completed = run.clone();
        completed.state = RunState::Completed;
        completed.fence += 1;
        workspace
            .owner_change(
                "complete-newest",
                &ChangeRequest {
                    reason: "Synthetic terminal status".into(),
                    writes: vec![Put {
                        id: latest.id.clone(),
                        expected_version: Some(latest.version),
                        archived: false,
                        data: RecordData::Run(completed.clone()),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        assert!(!super::can_continue_thread(
            &workspace.records().unwrap(),
            &workspace.history().unwrap(),
            &thread.id,
            &ThreadState::Open,
            false
        ));
        let instruction = workspace
            .post_message(&thread.id, "Unprocessed new instruction")
            .unwrap();
        let current_run = workspace.store.record(&latest.id).unwrap().unwrap();
        completed.progress = "Late update of an older invocation".into();
        workspace
            .owner_change(
                "late-old-run",
                &ChangeRequest {
                    reason: "Late old progress".into(),
                    writes: vec![Put {
                        id: latest.id.clone(),
                        expected_version: Some(current_run.version),
                        archived: false,
                        data: RecordData::Run(completed.clone()),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        assert!(super::can_continue_thread(
            &workspace.records().unwrap(),
            &workspace.history().unwrap(),
            &thread.id,
            &ThreadState::Open,
            false
        ));
        let next = workspace
            .store
            .allocate_operation("later-completed-run")
            .unwrap();
        workspace
            .owner_change(
                "later-completed-run",
                &ChangeRequest {
                    reason: "Later invocation consumed saved instruction".into(),
                    writes: vec![Put {
                        id: next.creation_id(0),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Run(completed),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        let instruction = instruction.writes.into_iter().next().unwrap().after;
        let mut edited = match instruction.data {
            RecordData::Message(message) => message,
            _ => panic!("owner message"),
        };
        edited.text = "Later edit is not another instruction creation".into();
        workspace
            .owner_change(
                "late-message-edit",
                &ChangeRequest {
                    reason: "Edit existing message".into(),
                    writes: vec![Put {
                        id: instruction.id,
                        expected_version: Some(instruction.version),
                        archived: false,
                        data: RecordData::Message(edited),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        assert!(!super::can_continue_thread(
            &workspace.records().unwrap(),
            &workspace.history().unwrap(),
            &thread.id,
            &ThreadState::Open,
            false
        ));
    }
    #[test]
    fn unsent_owner_message_remains_continuable_without_a_run_after_reopen() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let thread = workspace.new_thread("Saved instruction").unwrap();
        assert!(!super::can_continue_thread(
            &workspace.records().unwrap(),
            &workspace.history().unwrap(),
            &thread.id,
            &ThreadState::Open,
            false
        ));
        workspace
            .post_message(&thread.id, "Continue this exact instruction")
            .unwrap();
        drop(workspace);
        let workspace = Workspace::open(directory.path()).unwrap();
        let records = workspace.records().unwrap();
        let history = workspace.history().unwrap();
        assert!(super::can_continue_thread(
            &records,
            &history,
            &thread.id,
            &ThreadState::Open,
            false
        ));
        assert!(!super::can_continue_thread(
            &records,
            &history,
            &thread.id,
            &ThreadState::Resolved,
            false
        ));
        assert!(!super::can_continue_thread(
            &records,
            &history,
            &thread.id,
            &ThreadState::Open,
            true
        ));
    }
    #[test]
    fn saved_feedback_mapping_survives_restart_and_retains_removed_passage() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let record = workspace.new_note("Repeated", "tere tere").unwrap();
        let thread = workspace.new_thread("Writing").unwrap().id;
        workspace
            .link_writing_thread("passage-link", &record.id, record.version, &thread)
            .unwrap();
        let saved = workspace
            .add_writing_comment(
                "passage-fixture",
                &brn_threads_app::WritingCommentInput {
                    note: record.id.clone(),
                    version: record.version,
                    thread: thread.clone(),
                    range: (5, 9),
                    quote: "tere".into(),
                    body: "Second occurrence".into(),
                },
            )
            .unwrap();
        let comment_id = saved.id.clone();
        let RecordData::Comment(comment) = saved.data else {
            unreachable!()
        };
        assert_eq!(
            super::comment_location(&comment, record.version, "tere tere"),
            Some(5..9)
        );
        let session = workspace
            .store
            .begin_edit(&record.id, record.version)
            .unwrap();
        workspace
            .store
            .update_buffer(&session.id, 1, "Õun: tere tere")
            .unwrap();
        assert!(matches!(
            workspace.store.save(&session.id, 1).unwrap(),
            SaveOutcome::Saved(_)
        ));
        drop(workspace);
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let current = workspace.store.note(&record.id).unwrap().unwrap();
        let RecordData::Comment(mapped) =
            workspace.store.record(&comment_id).unwrap().unwrap().data
        else {
            unreachable!()
        };
        assert_eq!(
            super::comment_location(&mapped, current.version, "Õun: tere tere"),
            Some(11..15)
        );
        let session = workspace
            .store
            .begin_edit(&record.id, current.version)
            .unwrap();
        workspace
            .store
            .update_buffer(&session.id, 1, "Õun: tere kadunud")
            .unwrap();
        assert!(matches!(
            workspace.store.save(&session.id, 1).unwrap(),
            SaveOutcome::Saved(_)
        ));
        let current = workspace.store.note(&record.id).unwrap().unwrap();
        let RecordData::Comment(mapped) =
            workspace.store.record(&comment_id).unwrap().unwrap().data
        else {
            unreachable!()
        };
        assert!(mapped.unresolved);
        assert_eq!(mapped.quote, "tere");
        assert_eq!(
            super::comment_location(&mapped, current.version, "Õun: tere kadunud"),
            None
        );
        assert!(workspace.messages(&thread).unwrap().is_empty());
    }
    #[test]
    fn resolving_a_discussion_keeps_the_linked_waiting_obligation() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let thread = workspace.new_thread("Delivery").unwrap();
        let operation = workspace
            .store
            .allocate_operation("waiting-fixture")
            .unwrap();
        let action = operation.creation_id(0);
        let request = ChangeRequest {
            reason: "Waiting obligation".into(),
            writes: vec![
                Put {
                    id: action.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Action(Action {
                        kept_outside_project: false,
                        chat: None,
                        description: "Deliver the update".into(),
                        state: ActionState::Waiting,
                        internal: false,
                        evidence: None,
                        due: None,
                    }),
                },
                Put {
                    id: operation.creation_id(1),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Link(Link {
                        from: thread.id.clone(),
                        to: action.clone(),
                        relation: "tracks".into(),
                    }),
                },
            ],
            inputs: vec![],
        };
        workspace.owner_change("waiting-fixture", &request).unwrap();
        workspace.resolve_thread(&thread.id).unwrap();
        let RecordData::Action(action) = workspace.store.record(&action).unwrap().unwrap().data
        else {
            unreachable!()
        };
        assert_eq!(action.state, ActionState::Waiting);
        assert_eq!(action.evidence, None);
    }
}
