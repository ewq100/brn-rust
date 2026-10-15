//! Host attention for a Run that ends: the terminal notice (one per chat) and
//! a Review item for each candidate it prepared but never applied. The runtime's
//! terminal result and every checked change that ends a Working Run outside it
//! share these rules, inside that same change.
use crate::*;

/// The host's Blocker reason for a Run that stopped before finishing.
pub const INTERRUPTED_ATTENTION: &str = "Run interrupted. Continue starts fresh.";
/// Earlier notices, still recognised in saved chats so a newer result
/// replaces them. Every "Run failed: …" text matches by its prefix.
const LEGACY_NOTICES: [&str; 2] = [
    "Run interrupted; Continue starts fresh",
    "Run failed; check provider or continue from retained progress",
];
pub const FAILED_PREFIX: &str = "Run failed: ";
/// The Review reason for a prepared candidate that waits for the owner.
pub const REVIEW_ATTENTION: &str = "Review prepared changes";

/// Whether `reason` is a host terminal notice for an interrupted or failed Run.
pub fn is_terminal_notice(reason: &str) -> bool {
    reason == INTERRUPTED_ATTENTION
        || LEGACY_NOTICES.contains(&reason)
        || reason.starts_with(FAILED_PREFIX)
}

/// Keeps one terminal notice per chat. Drops host terminal Blockers for `run`
/// and for earlier Runs that `superseded` names, then adds `reason` for
/// `run`. Questions, reviews, conflicts and other Blockers stay.
pub fn replace_terminal_notice(
    attention: &mut Vec<Attention>,
    run: &str,
    reason: String,
    superseded: impl Fn(&str) -> bool,
) {
    attention.retain(|item| {
        item.kind != AttentionKind::Blocker
            || !is_terminal_notice(&item.reason)
            || !item
                .record
                .as_deref()
                .is_some_and(|id| id == run || superseded(id))
    });
    attention.push(Attention {
        kind: AttentionKind::Blocker,
        reason,
        record: Some(run.to_owned()),
    });
}

/// Adds the standard Review item for each candidate operation, in order,
/// unless an attention item already names that operation.
pub fn add_review_attention(attention: &mut Vec<Attention>, candidates: &[String]) {
    for operation in candidates {
        if !attention
            .iter()
            .any(|item| item.record.as_deref() == Some(operation.as_str()))
        {
            attention.push(Attention {
                kind: AttentionKind::Review,
                reason: REVIEW_ATTENTION.into(),
                record: Some(operation.clone()),
            });
        }
    }
}

/// Attention for Working Runs that `writes` ends outside the runtime's own
/// terminal result. Only each Run's `thread` and `operations` are read.
///
/// Per chat: a Review item for each still-active candidate the Run prepared
/// (`active`), then, with `notice`, the Interrupted notice that replaces older
/// terminal notices. A Thread write already in `writes` is edited in place;
/// otherwise a non-archived saved Thread (`record`) gets one write, added only
/// when its attention changes. Candidates are never applied or retired.
pub fn ended_run_attention(
    writes: &mut Vec<Put>,
    ended: &[(String, Run)],
    notice: bool,
    record: impl Fn(&str) -> Result<Option<Record>>,
    active: impl Fn(&[OperationId]) -> Result<Vec<String>>,
) -> Result<()> {
    let mut chats: Vec<&str> = Vec::new();
    for (_, run) in ended {
        if !chats.contains(&run.thread.as_str()) {
            chats.push(&run.thread);
        }
    }
    for chat in chats {
        let existing = writes
            .iter()
            .position(|w| w.id == chat && matches!(w.data, RecordData::Thread(_)));
        let (mut thread, version) = match existing {
            Some(index) => match &writes[index].data {
                RecordData::Thread(thread) => (thread.clone(), None),
                _ => unreachable!(),
            },
            None => match record(chat)? {
                Some(Record {
                    version,
                    archived: false,
                    data: RecordData::Thread(thread),
                    ..
                }) => (thread, Some(version)),
                _ => continue,
            },
        };
        let before = thread.attention.clone();
        for (id, run) in ended.iter().filter(|(_, run)| run.thread == chat) {
            add_review_attention(&mut thread.attention, &active(&run.operations)?);
            if notice {
                replace_terminal_notice(
                    &mut thread.attention,
                    id,
                    INTERRUPTED_ATTENTION.into(),
                    |other| {
                        ended.iter().any(|(ended, _)| ended == other)
                            || record(other).ok().flatten().is_some_and(|r| {
                                matches!(r.data, RecordData::Run(run) if run.thread == chat
                                    && matches!(run.state, RunState::Interrupted | RunState::Failed | RunState::Cancelled))
                            })
                    },
                );
            }
        }
        if thread.attention == before {
            continue;
        }
        match (existing, version) {
            (Some(index), _) => writes[index].data = RecordData::Thread(thread),
            (None, version) => writes.push(Put {
                id: chat.to_owned(),
                expected_version: version,
                archived: false,
                data: RecordData::Thread(thread),
            }),
        }
    }
    Ok(())
}

/// The operations among `operations` that [`Store::candidates`] lists:
/// prepared, not retired and without a receipt. Order is kept.
pub(crate) fn active_candidates_on(
    conn: &rusqlite::Connection,
    operations: &[OperationId],
) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT EXISTS(SELECT 1 FROM operations WHERE id=?1 AND hash IS NOT NULL AND retired=0 AND NOT EXISTS(SELECT 1 FROM receipts WHERE operation=operations.id))")?;
    let mut active: Vec<String> = Vec::new();
    for operation in operations {
        if !active.iter().any(|id| id == operation.as_str())
            && stmt.query_row([operation.as_str()], |row| row.get::<_, bool>(0))?
        {
            active.push(operation.as_str().to_owned());
        }
    }
    Ok(active)
}

impl Store {
    /// See [`active_candidates_on`].
    pub fn active_candidates(&self, operations: &[OperationId]) -> Result<Vec<String>> {
        active_candidates_on(&self.conn, operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(chat: &str, operations: &[&str]) -> Run {
        Run {
            instruction: None,
            queue_claim: None,
            thread: chat.into(),
            provider: "Codex".into(),
            model: "gpt-6.1-sol".into(),
            guide_identity: "fixture".into(),
            budget: 3,
            effort: "medium".into(),
            loaded_skills: vec![],
            fence: 2,
            state: RunState::Cancelled,
            progress: String::new(),
            operations: operations
                .iter()
                .map(|id| OperationId((*id).into()))
                .collect(),
            sources: vec![],
        }
    }
    fn chat(attention: Vec<Attention>, archived: bool) -> Record {
        Record {
            id: "chat".into(),
            version: 4,
            archived,
            data: RecordData::Thread(Thread {
                private_context: false,
                context_epoch: 0,
                title: "Chat".into(),
                state: ThreadState::Open,
                attention,
            }),
        }
    }
    fn blocker(run: &str) -> Attention {
        Attention {
            kind: AttentionKind::Blocker,
            reason: INTERRUPTED_ATTENTION.into(),
            record: Some(run.into()),
        }
    }
    fn review(operation: &str) -> Attention {
        Attention {
            kind: AttentionKind::Review,
            reason: REVIEW_ATTENTION.into(),
            record: Some(operation.into()),
        }
    }
    fn attention(write: &Put) -> &[Attention] {
        match &write.data {
            RecordData::Thread(thread) => &thread.attention,
            _ => panic!("Thread"),
        }
    }
    /// `saved` are the records `record` returns; every operation is active.
    fn ended(
        writes: &mut Vec<Put>,
        runs: &[(&str, Run)],
        notice: bool,
        saved: &[Record],
    ) -> Result<()> {
        let runs: Vec<_> = runs
            .iter()
            .map(|(id, run)| ((*id).to_owned(), run.clone()))
            .collect();
        ended_run_attention(
            writes,
            &runs,
            notice,
            |id| Ok(saved.iter().find(|r| r.id == id).cloned()),
            |ops| Ok(ops.iter().map(|op| op.as_str().to_owned()).collect()),
        )
    }

    #[test]
    fn review_is_skipped_when_any_item_already_names_the_operation() {
        let mut items = vec![Attention {
            kind: AttentionKind::Conflict,
            reason: "Prepared changes need a fresh base".into(),
            record: Some("op-1".into()),
        }];
        add_review_attention(&mut items, &["op-1".into(), "op-2".into(), "op-2".into()]);
        assert_eq!(items.len(), 2);
        assert_eq!(items[1], review("op-2"));
    }

    #[test]
    fn two_runs_ended_in_one_chat_leave_one_notice() {
        let saved = [chat(vec![], false)];
        let mut writes = vec![];
        ended(
            &mut writes,
            &[("run-a", run("chat", &[])), ("run-b", run("chat", &[]))],
            true,
            &saved,
        )
        .unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].expected_version, Some(4));
        assert!(!writes[0].archived);
        assert_eq!(attention(&writes[0]), [blocker("run-b")]);
    }

    #[test]
    fn a_notice_on_a_cancelled_run_is_replaced() {
        let mut old = run("chat", &[]);
        old.state = RunState::Cancelled;
        let saved = [
            chat(vec![blocker("run-old")], false),
            Record {
                id: "run-old".into(),
                version: 3,
                archived: false,
                data: RecordData::Run(old),
            },
        ];
        let mut writes = vec![];
        ended(&mut writes, &[("run-new", run("chat", &[]))], true, &saved).unwrap();
        assert_eq!(attention(&writes[0]), [blocker("run-new")]);
    }

    #[test]
    fn without_a_notice_only_the_review_is_added() {
        let saved = [chat(vec![], false)];
        let mut writes = vec![];
        ended(&mut writes, &[("run", run("chat", &["op"]))], false, &saved).unwrap();
        assert_eq!(attention(&writes[0]), [review("op")]);
        // With a notice, the Review comes first.
        let mut writes = vec![];
        ended(&mut writes, &[("run", run("chat", &["op"]))], true, &saved).unwrap();
        assert_eq!(attention(&writes[0]), [review("op"), blocker("run")]);
    }

    #[test]
    fn an_unchanged_or_archived_chat_gets_no_write() {
        let mut writes = vec![];
        ended(
            &mut writes,
            &[("run", run("chat", &[]))],
            false,
            &[chat(vec![], false)],
        )
        .unwrap();
        ended(
            &mut writes,
            &[("run", run("chat", &["op"]))],
            false,
            &[chat(vec![review("op")], false)],
        )
        .unwrap();
        ended(
            &mut writes,
            &[("run", run("chat", &["op"]))],
            true,
            &[chat(vec![], true)],
        )
        .unwrap();
        assert!(writes.is_empty());
    }

    #[test]
    fn an_existing_thread_write_is_edited_never_duplicated() {
        let RecordData::Thread(thread) = chat(vec![], false).data else {
            unreachable!()
        };
        let mut writes = vec![Put {
            id: "chat".into(),
            expected_version: Some(7),
            archived: true,
            data: RecordData::Thread(thread),
        }];
        ended(&mut writes, &[("run", run("chat", &["op"]))], true, &[]).unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].expected_version, Some(7));
        assert!(writes[0].archived);
        assert_eq!(attention(&writes[0]), [review("op"), blocker("run")]);
    }
}
