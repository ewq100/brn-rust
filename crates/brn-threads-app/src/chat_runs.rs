//! Each chat's latest Run and whether its saved work can continue, read from
//! one records and history snapshot. Hosts rebuild it when they refresh that
//! snapshot, so status queries never rescan history.
use crate::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ChatRuns {
    chats: HashMap<String, ChatRun>,
}

#[derive(Debug, Clone, Default)]
struct ChatRun {
    /// The latest current Run and the history index of its creation receipt.
    latest: Option<(Record, usize)>,
    /// The history index of the newest receipt creating a current owner
    /// message in the chat.
    newest_owner: Option<usize>,
}

impl ChatRuns {
    /// Creation receipts, rather than IDs, timestamps or a later progress
    /// write, order Runs and owner messages. Archived records are skipped, so
    /// an archived newest Run falls back to the next current one.
    pub fn new(records: &[Record], history: &[Receipt]) -> Self {
        let current: HashMap<&str, &Record> = records
            .iter()
            .filter(|record| !record.archived)
            .map(|record| (record.id.as_str(), record))
            .collect();
        let mut chats: HashMap<String, ChatRun> = HashMap::new();
        for (index, receipt) in history.iter().enumerate().rev() {
            for write in receipt.writes.iter().filter(|w| w.before.is_none()) {
                let Some(record) = current.get(write.after.id.as_str()) else {
                    continue;
                };
                match &write.after.data {
                    RecordData::Run(run) => {
                        let chat = chats.entry(run.thread.clone()).or_default();
                        if chat.latest.is_none() {
                            chat.latest = Some(((*record).clone(), index));
                        }
                    }
                    RecordData::Message(message) if message.role == "owner" => {
                        chats
                            .entry(message.thread.clone())
                            .or_default()
                            .newest_owner
                            .get_or_insert(index);
                    }
                    _ => {}
                }
            }
        }
        Self { chats }
    }

    /// The chat's latest current Run. Opening a chat never changes its
    /// persisted status.
    pub fn latest(&self, chat: &str) -> Option<&Record> {
        self.chats
            .get(chat)?
            .latest
            .as_ref()
            .map(|(record, _)| record)
    }

    /// A saved owner message was created after the chat's latest Run (or the
    /// chat has an owner message and no Run yet). A message saved in the
    /// Run's own creation receipt is not after it.
    pub fn owner_message_after_latest(&self, chat: &str) -> bool {
        let Some(chat) = self.chats.get(chat) else {
            return false;
        };
        match (chat.newest_owner, &chat.latest) {
            (Some(owner), Some((_, run))) => owner > *run,
            (owner, None) => owner.is_some(),
            (None, Some(_)) => false,
        }
    }

    /// Continue applies to an open chat this window is not running when its
    /// latest Run did not complete, or a saved owner message is newer than
    /// that Run (durable work even if the Run completed or a replacement
    /// never reached preparation).
    pub fn can_continue(&self, chat: &str, state: &ThreadState, has_worker: bool) -> bool {
        if has_worker || *state != ThreadState::Open {
            return false;
        }
        self.latest(chat).is_some_and(|record| {
            matches!(&record.data, RecordData::Run(run) if matches!(
                run.state,
                RunState::Working | RunState::Interrupted | RunState::Failed | RunState::Cancelled
            ))
        }) || self.owner_message_after_latest(chat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_run(workspace: &mut Workspace, key: &str, thread: &str, state: RunState) -> String {
        let settings = workspace.settings().unwrap();
        let id = workspace
            .store
            .allocate_operation(key)
            .unwrap()
            .creation_id(0);
        workspace
            .owner_change(
                key,
                &ChangeRequest {
                    reason: "Host invocation fixture".into(),
                    writes: vec![Put {
                        id: id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Run(Run {
                            loaded_skill_details: vec![],
                            instruction: None,
                            queue_claim: None,
                            thread: thread.into(),
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
                            web_sources: Vec::new(),
                        }),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        id
    }
    fn rewrite(workspace: &mut Workspace, key: &str, id: &str, edit: impl FnOnce(&mut Record)) {
        let mut record = workspace.store.record(id).unwrap().unwrap();
        let version = record.version;
        edit(&mut record);
        workspace
            .owner_change(
                key,
                &ChangeRequest {
                    reason: "Synthetic update".into(),
                    writes: vec![Put {
                        id: record.id,
                        expected_version: Some(version),
                        archived: record.archived,
                        data: record.data,
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
    }
    fn runs(workspace: &Workspace) -> ChatRuns {
        ChatRuns::new(&workspace.records().unwrap(), &workspace.history().unwrap())
    }
    fn open(workspace: &Workspace, chat: &str) -> bool {
        runs(workspace).can_continue(chat, &ThreadState::Open, false)
    }
    fn set_state(record: &mut Record, state: RunState) {
        if let RecordData::Run(run) = &mut record.data {
            run.state = state;
            run.fence += 1;
        }
    }

    #[test]
    fn newest_run_uses_creation_order_and_later_updates_never_reorder() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let thread = workspace.new_thread("Recover work").unwrap().id;
        let old = record_run(&mut workspace, "old-run", &thread, RunState::Failed);
        let newest = record_run(&mut workspace, "new-run", &thread, RunState::Working);
        assert_eq!(runs(&workspace).latest(&thread).unwrap().id, newest);
        assert!(open(&workspace, &thread));
        assert!(!runs(&workspace).can_continue(&thread, &ThreadState::Open, true));
        assert!(!runs(&workspace).can_continue(&thread, &ThreadState::Resolved, false));
        // A late write to the older Run does not make it the latest.
        rewrite(&mut workspace, "late-old", &old, |r| {
            set_state(r, RunState::Interrupted)
        });
        assert_eq!(runs(&workspace).latest(&thread).unwrap().id, newest);
        rewrite(&mut workspace, "complete-newest", &newest, |r| {
            set_state(r, RunState::Completed)
        });
        assert!(!open(&workspace, &thread));

        let instruction = workspace
            .post_message(&thread, "Unprocessed new instruction")
            .unwrap()
            .writes
            .into_iter()
            .next()
            .unwrap()
            .after
            .id;
        rewrite(&mut workspace, "late-newest", &newest, |r| {
            if let RecordData::Run(run) = &mut r.data {
                run.progress = "Late update of an older invocation".into();
            }
        });
        assert!(runs(&workspace).owner_message_after_latest(&thread));
        assert!(open(&workspace, &thread));

        let later = record_run(&mut workspace, "later-run", &thread, RunState::Completed);
        // Editing the message is not another instruction creation.
        rewrite(&mut workspace, "late-message-edit", &instruction, |r| {
            if let RecordData::Message(message) = &mut r.data {
                message.text = "Later edit".into();
            }
        });
        assert!(!runs(&workspace).owner_message_after_latest(&thread));
        assert!(!open(&workspace, &thread));

        // An archived newest Run falls back to the previous current one.
        rewrite(&mut workspace, "archive-later", &later, |r| {
            r.archived = true;
            set_state(r, RunState::Completed);
        });
        assert_eq!(runs(&workspace).latest(&thread).unwrap().id, newest);
        assert!(runs(&workspace).owner_message_after_latest(&thread));
        assert!(runs(&workspace).latest("other-chat").is_none());
    }

    #[test]
    fn unsent_owner_message_remains_continuable_without_a_run_after_reopen() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let thread = workspace.new_thread("Saved instruction").unwrap().id;
        assert!(!open(&workspace, &thread));
        workspace
            .post_message(&thread, "Continue this exact instruction")
            .unwrap();
        drop(workspace);
        let workspace = Workspace::open(directory.path()).unwrap();
        let runs = runs(&workspace);
        assert!(runs.latest(&thread).is_none());
        assert!(runs.can_continue(&thread, &ThreadState::Open, false));
        assert!(!runs.can_continue(&thread, &ThreadState::Resolved, false));
        assert!(!runs.can_continue(&thread, &ThreadState::Open, true));
    }

    #[test]
    fn owner_message_in_the_runs_own_receipt_is_not_after_it() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut workspace = Workspace::open(directory.path()).unwrap();
        let thread = workspace.new_thread("Same receipt").unwrap().id;
        let run = record_run(&mut workspace, "run", &thread, RunState::Completed);
        let run = workspace.store.record(&run).unwrap().unwrap();
        let op = workspace.store.allocate_operation("together").unwrap();
        let writes = vec![
            Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Message(Message {
                    thread: thread.clone(),
                    role: "owner".into(),
                    text: "Synthetic".into(),
                }),
            },
            Put {
                id: op.creation_id(1),
                expected_version: None,
                archived: false,
                data: run.data,
            },
        ];
        workspace
            .owner_change(
                "together",
                &ChangeRequest {
                    reason: "Message and Run together".into(),
                    writes,
                    inputs: vec![],
                },
            )
            .unwrap();
        let runs = runs(&workspace);
        assert_eq!(runs.latest(&thread).unwrap().id, op.creation_id(1));
        assert!(!runs.owner_message_after_latest(&thread));
        assert!(!runs.can_continue(&thread, &ThreadState::Open, false));
    }
}
