//! Small presentation rules; canonical state and authority remain in the shared core.
use brn_threads_app::{
    ApplyOutcome, Comment, Receipt, Record, RecordData, RunState, SaveOutcome, ThreadState,
};
use std::ops::Range;

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
pub(super) fn continuation_notice(state: &RunState) -> &'static str {
    if *state == RunState::Working {
        "This run may have been interrupted or may still be running elsewhere. Continue starts fresh with the saved conversation and current notes, and supersedes the previous run."
    } else {
        "Continue starts fresh with the saved conversation and current notes."
    }
}
pub(super) fn passage_request(
    operation: &brn_threads_app::OperationId,
    note: &str,
    version: u64,
    range: Range<usize>,
    quote: &str,
    body: &str,
) -> (String, brn_threads_app::ChangeRequest) {
    use brn_threads_app::{ChangeRequest, Link, Message, Put, RevisionInput, Thread, ThreadState};
    let thread = operation.creation_id(0);
    let comment = operation.creation_id(1);
    let request = ChangeRequest {
        reason: "Owner commented on a passage".into(),
        writes: vec![
            Put {
                id: thread.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Thread(Thread {
                    title: body.chars().take(70).collect(),
                    state: ThreadState::Open,
                    attention: vec![],
                }),
            },
            Put {
                id: comment.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Comment(Comment {
                    note: note.into(),
                    base_version: version,
                    quote: quote.into(),
                    range: Some((range.start, range.end)),
                    mapped_version: None,
                    mapped_range: None,
                    body: body.into(),
                    unresolved: true,
                }),
            },
            Put {
                id: operation.creation_id(2),
                expected_version: None,
                archived: false,
                data: RecordData::Link(Link {
                    from: thread.clone(),
                    to: comment,
                    relation: "passage comment".into(),
                }),
            },
            Put {
                id: operation.creation_id(3),
                expected_version: None,
                archived: false,
                data: RecordData::Link(Link {
                    from: thread.clone(),
                    to: note.into(),
                    relation: "discusses".into(),
                }),
            },
            Put {
                id: operation.creation_id(4),
                expected_version: None,
                archived: false,
                data: RecordData::Message(Message {
                    thread: thread.clone(),
                    role: "owner".into(),
                    text: body.into(),
                }),
            },
        ],
        inputs: vec![RevisionInput {
            record: note.into(),
            version,
        }],
    };
    (thread, request)
}
pub(super) fn outcome_message(outcome: &ApplyOutcome) -> String {
    match outcome {
        ApplyOutcome::Applied(receipt) => format!("Saved. {}", receipt.reason),
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
    let (mapped_version, range) = match (comment.mapped_version, comment.mapped_range) {
        (Some(version), Some(range)) => (version, range),
        _ => (comment.base_version, comment.range?),
    };
    if mapped_version != version {
        return None;
    }
    let range = range.0..range.1;
    (range.start < range.end && text.get(range.clone()) == Some(comment.quote.as_str()))
        .then_some(range)
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
            unresolved: true,
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
            unresolved: true,
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
    }
    #[test]
    fn passage_comment_creates_one_thread_and_context_without_revising_the_note() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let record = workspace.new_note("Repeated", "tere tere").unwrap();
        let op = workspace
            .store
            .allocate_operation("passage-fixture")
            .unwrap();
        let (thread, request) = super::passage_request(
            &op,
            &record.id,
            record.version,
            5..9,
            "tere",
            "Second occurrence",
        );
        assert!(matches!(
            workspace.owner_change("passage-fixture", &request).unwrap(),
            ApplyOutcome::Applied(_)
        ));
        assert_eq!(
            workspace.store.note(&record.id).unwrap().unwrap().version,
            record.version
        );
        assert_eq!(workspace.messages(&thread).unwrap().len(), 1);
        let RecordData::Comment(comment) = workspace
            .store
            .record(&op.creation_id(1))
            .unwrap()
            .unwrap()
            .data
        else {
            unreachable!()
        };
        assert_eq!(
            super::comment_location(&comment, record.version, "tere tere"),
            Some(5..9)
        );
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
