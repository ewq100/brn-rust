//! Small presentation rules; canonical state and authority remain in the shared core.
use brn_threads_app::{
    ApplyOutcome, Comment, Receipt, Record, RecordData, RunState, SaveOutcome, ThreadState,
    WorkspaceSettings,
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
    // Compare receipt order, never timestamps or later Run updates.
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
pub(super) fn continuation_notice(state: &RunState) -> &'static str {
    if *state == RunState::Working {
        "This run may have been interrupted or may still be running elsewhere. Continue starts fresh with the saved conversation and current notes, and supersedes the previous run."
    } else {
        "Continue starts fresh with the saved conversation and current notes."
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
pub(super) fn outcome_message(outcome: &ApplyOutcome) -> String {
    match outcome {
        ApplyOutcome::Applied(receipt) => format!("Saved. {}{}", receipt.reason, if receipt.needs_refresh.is_empty() { String::new() } else { format!(" {}", refresh_hint(receipt.needs_refresh.len())) }),
        ApplyOutcome::Deferred { .. } => "Waiting for an open edit. Save or discard that edit, then retry this change.".into(),
        ApplyOutcome::Stale { .. } => "The current content changed. This proposal was preserved; compare it with the latest content before preparing a new change.".into(),
        ApplyOutcome::NeedsReview { .. } => "This change needs your review. The current notes are unchanged.".into(),
        ApplyOutcome::Superseded { .. } => "This run stopped because newer work took its place.".into(),
    }
}
pub(super) fn save_message(outcome: &SaveOutcome) -> &'static str {
    match outcome {
        SaveOutcome::Saved(_) => "Saved. Your edit is in History.",
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
        assert!(continuation_notice(&RunState::Working).contains("running elsewhere"));
        assert!(continuation_notice(&RunState::Working).contains("supersedes"));
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
