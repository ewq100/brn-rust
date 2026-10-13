//! Durable host chat controls over the checked core writer.
use crate::*;

impl Workspace {
    pub(crate) fn bootstrap_chat_control(&mut self) -> AppResult<()> {
        if self.store.chat_control()?.is_none() {
            self.chat_command("bootstrap", ChatCommand::Bootstrap)?;
        }
        Ok(())
    }
    pub fn chat_control(&self) -> AppResult<Record> {
        self.store
            .chat_control()?
            .ok_or_else(|| AppError::Invalid("chat controls missing".into()))
    }
    fn chat_command(&mut self, key: &str, command: ChatCommand) -> AppResult<ChatCommandOutcome> {
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&command)?));
        let op = self
            .store
            .allocate_bound_operation(&format!("chat-control:{key}"), &binding)?;
        Ok(self
            .store
            .chat_command(&op, &command, &HostAuthority::owner("owner chat action"))?)
    }
    pub fn queue_message(&mut self, key: &str, thread: &str, text: &str) -> AppResult<Receipt> {
        Ok(self
            .chat_command(
                key,
                ChatCommand::Enqueue {
                    thread: thread.into(),
                    text: text.into(),
                },
            )?
            .receipt)
    }
    pub fn edit_queued(&mut self, key: &str, item: &str, text: &str) -> AppResult<Receipt> {
        Ok(self
            .chat_command(
                key,
                ChatCommand::Edit {
                    item: item.into(),
                    text: text.into(),
                },
            )?
            .receipt)
    }
    pub fn remove_queued(&mut self, key: &str, item: &str) -> AppResult<Receipt> {
        Ok(self
            .chat_command(key, ChatCommand::Remove { item: item.into() })?
            .receipt)
    }
    pub fn move_queued(
        &mut self,
        key: &str,
        item: &str,
        before: Option<&str>,
    ) -> AppResult<Receipt> {
        Ok(self
            .chat_command(
                key,
                ChatCommand::Move {
                    item: item.into(),
                    before: before.map(str::to_owned),
                },
            )?
            .receipt)
    }
    pub fn pause_queue(&mut self, key: &str) -> AppResult<Receipt> {
        Ok(self
            .chat_command(
                key,
                ChatCommand::Pause {
                    run: None,
                    partial: None,
                },
            )?
            .receipt)
    }
    pub fn stop_chat_run(
        &mut self,
        key: &str,
        run: &str,
        partial: Option<&str>,
    ) -> AppResult<Receipt> {
        Ok(self
            .chat_command(
                key,
                ChatCommand::Pause {
                    run: Some(run.into()),
                    partial: partial.map(str::to_owned),
                },
            )?
            .receipt)
    }
    pub fn resume_queue(&mut self, key: &str) -> AppResult<Receipt> {
        Ok(self.chat_command(key, ChatCommand::Resume)?.receipt)
    }
    pub fn recover_queue(&mut self, key: &str) -> AppResult<Receipt> {
        Ok(self.chat_command(key, ChatCommand::Recover)?.receipt)
    }
    pub fn choose_quiet(&mut self, key: &str, quiet: bool) -> AppResult<Receipt> {
        Ok(self.chat_command(key, ChatCommand::Quiet(quiet))?.receipt)
    }
    pub fn dispatch_queued(&mut self, key: &str) -> AppResult<Option<QueuedMessage>> {
        let command = ChatCommand::Dispatch;
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&command)?));
        let operation = self
            .store
            .allocate_bound_operation(&format!("chat-control:{key}"), &binding)?;
        if let Some(receipt) = self.store.receipt(&operation)? {
            return Ok(receipt.writes.iter().find_map(|w| match &w.after.data {
                RecordData::ChatControl(c) => c.pending.clone(),
                _ => None,
            }));
        }
        let record = self.chat_control()?;
        let RecordData::ChatControl(c) = record.data else {
            unreachable!()
        };
        if c.paused || c.pending.is_none() && c.queue.is_empty() {
            return Ok(None);
        }
        Ok(self.chat_command(key, ChatCommand::Dispatch)?.dispatched)
    }
    pub fn bind_queued_run(
        &mut self,
        key: &str,
        item: &str,
        message: &str,
        run: &str,
    ) -> AppResult<Receipt> {
        let dispatch = self
            .store
            .record(run)?
            .and_then(|r| match r.data {
                RecordData::Run(r) => r.queue_claim,
                _ => None,
            })
            .ok_or_else(|| AppError::Invalid("queued run claim missing".into()))?;
        Ok(self
            .chat_command(
                key,
                ChatCommand::Bind {
                    item: item.into(),
                    message: message.into(),
                    run: run.into(),
                    dispatch,
                },
            )?
            .receipt)
    }
    pub fn finish_queued(
        &mut self,
        key: &str,
        item: &str,
        message: &str,
        run: &str,
        fence: u64,
        success: bool,
    ) -> AppResult<Receipt> {
        Ok(self
            .chat_command(
                key,
                ChatCommand::Finish {
                    item: item.into(),
                    message: message.into(),
                    run: run.into(),
                    fence,
                    success,
                },
            )?
            .receipt)
    }
    pub fn archive_chat(&mut self, key: &str, thread: &str) -> AppResult<Receipt> {
        Ok(self
            .chat_command(
                key,
                ChatCommand::Archive {
                    thread: thread.into(),
                },
            )?
            .receipt)
    }
    pub fn archive_chat_progress(
        &mut self,
        key: &str,
        thread: &str,
        run: &str,
        partial: &str,
    ) -> AppResult<Receipt> {
        Ok(self
            .chat_command(
                key,
                ChatCommand::ArchiveProgress {
                    thread: thread.into(),
                    run: run.into(),
                    partial: partial.into(),
                },
            )?
            .receipt)
    }
    pub fn unarchive_chat(&mut self, key: &str, thread: &str) -> AppResult<Receipt> {
        Ok(self
            .chat_command(
                key,
                ChatCommand::Unarchive {
                    thread: thread.into(),
                },
            )?
            .receipt)
    }
}
