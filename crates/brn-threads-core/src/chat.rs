//! Host-only composer recovery. Latest text is recovery state, not keystroke history.
use crate::store::{apply_on, prepare_on, receipt_on, record_on};
use crate::*;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize)]
pub enum ChatCommand {
    Bootstrap,
    Enqueue {
        thread: String,
        text: String,
    },
    Edit {
        item: String,
        text: String,
    },
    Remove {
        item: String,
    },
    Move {
        item: String,
        before: Option<String>,
    },
    Pause {
        run: Option<String>,
        partial: Option<String>,
    },
    Resume,
    Recover,
    Dispatch,
    Bind {
        item: String,
        message: String,
        run: String,
        dispatch: String,
    },
    Finish {
        item: String,
        message: String,
        run: String,
        fence: u64,
        success: bool,
    },
    Archive {
        thread: String,
    },
    ArchiveProgress {
        thread: String,
        run: String,
        partial: String,
    },
    Unarchive {
        thread: String,
    },
    Quiet(bool),
}
pub struct ChatCommandOutcome {
    pub receipt: Receipt,
    pub dispatched: Option<QueuedMessage>,
}
fn instruction(text: &str) -> Result<()> {
    if text.trim().is_empty() || text.len() > 128 * 1024 {
        return Err(Error::Invalid("message needs 1–131072 bytes".into()));
    }
    Ok(())
}
pub(crate) fn partial_text(text: &str) -> Result<()> {
    if text.len() > 50 * 1024 * 1024 {
        return Err(Error::Invalid(
            "partial answer exceeds the supported answer bound".into(),
        ));
    }
    Ok(())
}
fn active_chat(tx: &rusqlite::Transaction<'_>, id: &str) -> Result<Record> {
    let r = record_on(tx, id)?.ok_or_else(|| Error::Missing(id.into()))?;
    if crate::project_for_chat(&crate::projects::rows_on(tx)?, id).is_some_and(|r| r.archived) {
        return Err(Error::Invalid(
            "Restore this project before sending work".into(),
        ));
    }
    if r.archived || !matches!(&r.data, RecordData::Thread(t) if t.state == ThreadState::Open) {
        return Err(Error::Invalid(
            "Unarchive this chat before sending queued work".into(),
        ));
    }
    Ok(r)
}
pub(crate) fn control_on(conn: &rusqlite::Connection) -> Result<Option<Record>> {
    let body: Option<String> = conn
        .query_row(
            "SELECT body FROM records WHERE json_type(body,'$.data.ChatControl') IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .optional()?;
    body.map(|b| serde_json::from_str(&b).map_err(Error::from))
        .transpose()
}

impl Store {
    pub fn chat_control(&self) -> Result<Option<Record>> {
        control_on(&self.conn)
    }

    /// Queue control and any posted Message/run fence share the checked transaction.
    pub fn chat_command(
        &mut self,
        op: &OperationId,
        command: &ChatCommand,
        owner: &HostAuthority,
    ) -> Result<ChatCommandOutcome> {
        match self.chat_command_body(op, command, owner, false)? {
            Converged::Changed(outcome) => Ok(outcome),
            Converged::Unchanged(_) => Err(Error::Invalid("no-op detection is off".into())),
        }
    }
    /// `chat_command`, except that an archive or unarchive whose state already
    /// holds returns `Unchanged` without committing anything.
    pub fn chat_command_converging(
        &mut self,
        op: &OperationId,
        command: &ChatCommand,
        owner: &HostAuthority,
    ) -> Result<Converged<ChatCommandOutcome>> {
        self.chat_command_body(op, command, owner, true)
    }
    fn chat_command_body(
        &mut self,
        op: &OperationId,
        command: &ChatCommand,
        owner: &HostAuthority,
        converge: bool,
    ) -> Result<Converged<ChatCommandOutcome>> {
        if !matches!(owner.scope, crate::types::Scope::Owner) {
            return Err(Error::Invalid(
                "chat controls require trusted owner authority".into(),
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(command)?));
        let prior: Option<String> = tx.query_row(
            "SELECT input_hash FROM operations WHERE id=?1",
            [op.as_str()],
            |r| r.get(0),
        )?;
        if prior.as_deref() != Some(&binding) {
            return Err(Error::ImmutableRequest);
        }
        if let Some(receipt) = receipt_on(&tx, op)? {
            let dispatched = if matches!(command, ChatCommand::Dispatch) {
                receipt.writes.iter().find_map(|w| match &w.after.data {
                    RecordData::ChatControl(c) => c.pending.clone(),
                    _ => None,
                })
            } else {
                None
            };
            tx.commit()?;
            return Ok(Converged::Changed(ChatCommandOutcome {
                receipt,
                dispatched,
            }));
        }
        let previous = control_on(&tx)?;
        let id = previous
            .as_ref()
            .map(|r| r.id.clone())
            .unwrap_or_else(|| op.creation_id(0));
        let version = previous.as_ref().map(|r| r.version);
        let mut control = previous
            .and_then(|r| match r.data {
                RecordData::ChatControl(c) => Some(c),
                _ => None,
            })
            .unwrap_or_default();
        let mut writes = vec![];
        let mut dispatched = None;
        match command {
            ChatCommand::Bootstrap => {}
            ChatCommand::Enqueue { thread, text } => {
                instruction(text)?;
                active_chat(&tx, thread)?;
                control.queue.push(QueuedMessage {
                    id: op.creation_id(1),
                    thread: thread.clone(),
                    text: text.clone(),
                    message: None,
                    run: None,
                    dispatch: None,
                });
            }
            ChatCommand::Edit { item, text } => {
                instruction(text)?;
                if let Some(q) = control.queue.iter_mut().find(|q| q.id == *item) {
                    q.text = text.clone();
                } else if let Some(q) = control.pending.as_mut().filter(|q| q.id == *item) {
                    if let Some(run) = &q.run
                        && matches!(record_on(&tx, run)?, Some(Record { data: RecordData::Run(r), .. }) if r.state == RunState::Working)
                    {
                        return Err(Error::Invalid(
                            "Stop the active item before editing it".into(),
                        ));
                    }
                    q.text = text.clone();
                    q.message = None;
                    q.run = None;
                    q.dispatch = None;
                } else {
                    return Err(Error::Missing(item.clone()));
                }
            }
            ChatCommand::Remove { item } => {
                if let Some(index) = control.queue.iter().position(|q| q.id == *item) {
                    control.queue.remove(index);
                } else if let Some(q) = control.pending.as_ref().filter(|q| q.id == *item) {
                    if let Some(run) = &q.run
                        && matches!(record_on(&tx, run)?, Some(Record { data: RecordData::Run(r), .. }) if r.state == RunState::Working)
                    {
                        return Err(Error::Invalid(
                            "Stop before removing the active item".into(),
                        ));
                    }
                    control.pending = None;
                } else {
                    return Err(Error::Missing(item.clone()));
                }
            }
            ChatCommand::Move { item, before } => {
                if before.as_ref() == Some(item) {
                    return Err(Error::Invalid("cannot move an item before itself".into()));
                }
                let index = control
                    .queue
                    .iter()
                    .position(|q| q.id == *item)
                    .ok_or_else(|| Error::Missing(item.clone()))?;
                let q = control.queue.remove(index);
                let target = match before {
                    Some(id) => control
                        .queue
                        .iter()
                        .position(|q| q.id == *id)
                        .ok_or_else(|| Error::Missing(id.clone()))?,
                    None => control.queue.len(),
                };
                control.queue.insert(target, q);
            }
            ChatCommand::Pause { run, partial } => {
                control.paused = true;
                if let Some(id) = run {
                    let record = record_on(&tx, id)?.ok_or_else(|| Error::Missing(id.clone()))?;
                    let RecordData::Run(mut r) = record.data else {
                        return Err(Error::Invalid("exact Run required".into()));
                    };
                    let was_working = r.state == RunState::Working;
                    if was_working {
                        r.state = RunState::Interrupted;
                        r.fence = r
                            .fence
                            .checked_add(1)
                            .ok_or_else(|| Error::Invalid("run fence exhausted".into()))?;
                        writes.push(Put {
                            id: id.clone(),
                            expected_version: Some(record.version),
                            archived: false,
                            data: RecordData::Run(r.clone()),
                        });
                    }
                    if let Some(text) = partial
                        .as_ref()
                        .filter(|t| was_working && !t.trim().is_empty())
                    {
                        partial_text(text)?;
                        writes.push(Put {
                            id: op.creation_id(2),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Message(Message {
                                thread: r.thread,
                                role: "assistant-partial".into(),
                                text: text.clone(),
                            }),
                        });
                    }
                } else if partial.is_some() {
                    return Err(Error::Invalid("partial progress requires its Run".into()));
                }
            }
            ChatCommand::Resume => {
                let working: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM records WHERE json_extract(body,'$.archived')=0 AND json_extract(body,'$.data.Run.state')='Working')", [], |r| r.get(0))?;
                if working {
                    return Err(Error::Invalid(
                        "Finish or stop current work before resuming the queue".into(),
                    ));
                }
                control.paused = false;
                if let Some(q) = &mut control.pending {
                    q.dispatch = None;
                    q.run = None;
                }
            }
            ChatCommand::Recover => control.paused = true,
            ChatCommand::Quiet(quiet) => control.quiet = *quiet,
            ChatCommand::Dispatch => {
                if control.paused {
                    return Err(Error::Invalid("queue paused; Resume when ready".into()));
                }
                let working: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM records WHERE json_extract(body,'$.archived')=0 AND json_extract(body,'$.data.Run.state')='Working')", [], |r| r.get(0))?;
                if working {
                    return Err(Error::Invalid(
                        "work is still running; queued messages are saved".into(),
                    ));
                }
                if control
                    .pending
                    .as_ref()
                    .is_some_and(|q| q.dispatch.is_some())
                {
                    return Err(Error::Invalid("queued instruction is claimed; explicit Resume is required after interruption".into()));
                }
                let mut q = if let Some(q) = control.pending.take() {
                    q
                } else if !control.queue.is_empty() {
                    control.queue.remove(0)
                } else {
                    return Err(Error::Invalid("queue is empty".into()));
                };
                active_chat(&tx, &q.thread)?;
                if let Some(message) = &q.message {
                    if !matches!(record_on(&tx, message)?, Some(Record { archived: false, data: RecordData::Message(m), .. }) if m.thread == q.thread && m.role == "owner" && m.text == q.text)
                    {
                        return Err(Error::Invalid("saved queued instruction changed".into()));
                    }
                } else {
                    let message = op.creation_id(1);
                    writes.push(Put {
                        id: message.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Message(Message {
                            thread: q.thread.clone(),
                            role: "owner".into(),
                            text: q.text.clone(),
                        }),
                    });
                    q.message = Some(message);
                }
                q.run = None;
                q.dispatch = Some(op.as_str().into());
                control.pending = Some(q.clone());
                dispatched = Some(q);
            }
            ChatCommand::Bind {
                item,
                message,
                run,
                dispatch,
            } => {
                let q = control
                    .pending
                    .as_mut()
                    .filter(|q| {
                        q.id == *item
                            && q.message.as_ref() == Some(message)
                            && q.dispatch.as_ref() == Some(dispatch)
                    })
                    .ok_or_else(|| {
                        Error::Invalid("pending instruction changed before run binding".into())
                    })?;
                if !matches!(record_on(&tx, run)?, Some(Record { archived: false, data: RecordData::Run(r), .. }) if r.thread == q.thread && r.instruction.as_ref() == Some(message) && r.queue_claim.as_ref() == Some(dispatch) && r.state == RunState::Working)
                {
                    return Err(Error::Invalid(
                        "queued Run does not match its exact instruction".into(),
                    ));
                }
                if let Some(old) = &q.run
                    && old != run
                    && matches!(record_on(&tx, old)?, Some(Record { data: RecordData::Run(r), .. }) if r.state == RunState::Working)
                {
                    return Err(Error::Invalid(
                        "pending item still has an active Run".into(),
                    ));
                }
                q.run = Some(run.clone());
            }
            ChatCommand::Finish {
                item,
                message,
                run,
                fence,
                success,
            } => {
                let q = control
                    .pending
                    .as_ref()
                    .filter(|q| {
                        q.id == *item
                            && q.message.as_ref() == Some(message)
                            && q.run.as_ref() == Some(run)
                    })
                    .ok_or_else(|| {
                        Error::Invalid(
                            "terminal event belongs to an older queued invocation".into(),
                        )
                    })?;
                if !matches!(record_on(&tx, run)?, Some(Record { data: RecordData::Run(r), .. }) if r.thread == q.thread && r.instruction.as_ref() == Some(message) && r.fence == *fence && if *success { r.state == RunState::Completed } else { matches!(r.state, RunState::Failed | RunState::Interrupted | RunState::Cancelled) })
                {
                    return Err(Error::Invalid(
                        "terminal Run/fence does not confirm the queue outcome".into(),
                    ));
                }
                if *success {
                    control.pending = None;
                } else {
                    control.paused = true;
                }
            }
            ChatCommand::Archive { thread }
            | ChatCommand::ArchiveProgress { thread, .. }
            | ChatCommand::Unarchive { thread } => {
                if crate::project_for_chat(&crate::projects::project_rows_on(&tx)?, thread)
                    .is_some_and(|p| p.archived)
                {
                    return Err(Error::Invalid(
                        "Restore this project before changing its chats".into(),
                    ));
                }
                let record =
                    record_on(&tx, thread)?.ok_or_else(|| Error::Missing(thread.clone()))?;
                let RecordData::Thread(mut t) = record.data else {
                    return Err(Error::Invalid("conversation required".into()));
                };
                if let ChatCommand::ArchiveProgress { run, .. } = command
                    && !matches!(record_on(&tx,run)?,Some(Record{data:RecordData::Run(r),..}) if r.thread==*thread)
                {
                    return Err(Error::Invalid(
                        "archive progress requires that chat's exact Run".into(),
                    ));
                }
                let archived = matches!(
                    command,
                    ChatCommand::Archive { .. } | ChatCommand::ArchiveProgress { .. }
                );
                t.state = ThreadState::Open;
                writes.push(Put {
                    id: thread.clone(),
                    expected_version: Some(record.version),
                    archived,
                    data: RecordData::Thread(t),
                });
                if archived {
                    control.paused = true;
                    let mut query = tx.prepare(
                        "SELECT body FROM records WHERE json_extract(body,'$.data.Run.thread')=?1",
                    )?;
                    let rows = query
                        .query_map([thread], |r| r.get::<_, String>(0))?
                        .collect::<std::result::Result<Vec<_>, _>>()?;
                    for body in rows {
                        let r: Record = serde_json::from_str(&body)?;
                        if let RecordData::Run(mut run) = r.data
                            && run.state == RunState::Working
                            && !r.archived
                        {
                            if let ChatCommand::ArchiveProgress {
                                run: exact,
                                partial,
                                ..
                            } = command
                                && r.id == *exact
                                && !partial.trim().is_empty()
                            {
                                partial_text(partial)?;
                                writes.push(Put {
                                    id: op.creation_id(2),
                                    expected_version: None,
                                    archived: false,
                                    data: RecordData::Message(Message {
                                        thread: thread.clone(),
                                        role: "assistant-partial".into(),
                                        text: partial.clone(),
                                    }),
                                });
                            }
                            run.state = RunState::Interrupted;
                            run.fence = run
                                .fence
                                .checked_add(1)
                                .ok_or_else(|| Error::Invalid("run fence exhausted".into()))?;
                            writes.push(Put {
                                id: r.id,
                                expected_version: Some(r.version),
                                archived: false,
                                data: RecordData::Run(run),
                            });
                        }
                    }
                }
            }
        }
        if control.queue.len() + usize::from(control.pending.is_some()) > 128
            || control
                .queue
                .iter()
                .chain(control.pending.iter())
                .map(|q| q.text.len())
                .sum::<usize>()
                > 4 * 1024 * 1024
        {
            return Err(Error::Invalid(
                "queue is full; your composer text is preserved".into(),
            ));
        }
        writes.push(Put {
            id,
            expected_version: version,
            archived: false,
            data: RecordData::ChatControl(control),
        });
        if let (true, ChatCommand::Archive { thread } | ChatCommand::Unarchive { thread }) =
            (converge, command)
            && let Some(unchanged) = crate::converge::detect_unchanged(&tx, thread, &writes, false)?
        {
            return Ok(Converged::Unchanged(unchanged));
        }
        prepare_on(
            &tx,
            op,
            &ChangeRequest {
                reason: format!("Chat control: {}", command.label()),
                writes,
                inputs: vec![],
            },
            true,
        )?;
        let ApplyOutcome::Applied(receipt) = apply_on(&tx, op, owner, None)? else {
            return Err(Error::Invalid("chat control group was not applied".into()));
        };
        tx.commit()?;
        Ok(Converged::Changed(ChatCommandOutcome {
            receipt,
            dispatched,
        }))
    }
}
impl ChatCommand {
    fn label(&self) -> &'static str {
        match self {
            Self::Bootstrap => "initialize",
            Self::Enqueue { .. } => "queue message",
            Self::Edit { .. } => "edit queued message",
            Self::Remove { .. } => "remove queued message",
            Self::Move { .. } => "reorder queue",
            Self::Pause { .. } => "stop and pause",
            Self::Resume => "resume queue",
            Self::Recover => "recover paused queue",
            Self::Dispatch => "dispatch queued instruction",
            Self::Bind { .. } => "bind queued run",
            Self::Finish { .. } => "record queued outcome",
            Self::Archive { .. } | Self::ArchiveProgress { .. } => "archive chat",
            Self::Unarchive { .. } => "unarchive chat",
            Self::Quiet(_) => "change display detail",
        }
    }
}

impl Store {
    pub fn chat_drafts(&self) -> Result<Vec<ChatDraft>> {
        let mut query = self
            .conn
            .prepare("SELECT destination,generation,text FROM chat_drafts ORDER BY destination")?;
        Ok(query
            .query_map([], |r| {
                Ok(ChatDraft {
                    destination: r.get(0)?,
                    generation: crate::read_u64(r, 1)?,
                    text: r.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn save_chat_draft(
        &mut self,
        destination: &str,
        generation: u64,
        text: &str,
    ) -> Result<ChatDraft> {
        if destination.is_empty() || destination.len() > 1024 || text.len() > 128 * 1024 {
            return Err(Error::Invalid(
                "bounded destination and composer text required".into(),
            ));
        }
        let generation = crate::integer(generation)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<(u64, String)> = tx
            .query_row(
                "SELECT generation,text FROM chat_drafts WHERE destination=?1",
                [destination],
                |r| Ok((crate::read_u64(r, 0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((old, previous)) = prior
            && (generation < crate::integer(old)?
                || (generation == crate::integer(old)? && text != previous))
        {
            return Err(Error::Invalid(
                "composer recovery generation is stale".into(),
            ));
        }
        tx.execute("INSERT INTO chat_drafts(destination,generation,text) VALUES(?1,?2,?3) ON CONFLICT(destination) DO UPDATE SET generation=excluded.generation,text=excluded.text", params![destination,generation,text])?;
        tx.commit()?;
        Ok(ChatDraft {
            destination: destination.into(),
            generation: generation as u64,
            text: text.into(),
        })
    }
}
