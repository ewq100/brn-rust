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
        // Only a stop gives an orphan the notice. Archive ends Runs too, but
        // as a deliberate owner stop it adds none (owner decision 2); its
        // ended Runs' free lease files are still removed below.
        let orphans = if matches!(command, ChatCommand::Pause { run: Some(_), .. }) {
            self.lease_free_working_runs()?
        } else {
            BTreeSet::new()
        };
        let outcome = self.store.chat_command_with_orphans(
            &op,
            &command,
            &HostAuthority::owner("owner chat action"),
            &orphans,
        )?;
        self.remove_ended_leases(&outcome.receipt);
        Ok(outcome)
    }
    /// Non-archived Working Runs that no process holds a lease for. A stop
    /// treats them as orphans: notice, Review items and lease removal. Without
    /// a usable lease folder liveness is unknown, so every Run counts as live.
    /// A free lease stays free: only a new Run ID ever takes one.
    pub(crate) fn lease_free_working_runs(&self) -> AppResult<BTreeSet<String>> {
        let Ok(derived) = self.store.derived_directory() else {
            return Ok(BTreeSet::new());
        };
        Ok(self
            .records()?
            .into_iter()
            .filter(|r| {
                !r.archived
                    && matches!(&r.data, RecordData::Run(run) if run.state == RunState::Working)
                    && !brn_ai::run_lease::run_is_live(&derived, &r.id)
            })
            .map(|r| r.id)
            .collect())
    }
    /// Removes the leftover lease file of each Run that `receipt` ended. A
    /// lease a process still holds is never removed, so a live Run keeps its
    /// file until its own worker drops it. Best effort: errors keep the file.
    pub(crate) fn remove_ended_leases(&self, receipt: &Receipt) {
        let ended: Vec<&str> = receipt
            .writes
            .iter()
            .filter(|w| {
                matches!(&w.before, Some(Record { data: RecordData::Run(run), .. }) if run.state == RunState::Working)
                    && matches!(&w.after.data, RecordData::Run(run) if run.state != RunState::Working)
            })
            .map(|w| w.after.id.as_str())
            .collect();
        if ended.is_empty() {
            return;
        }
        let Ok(derived) = self.store.derived_directory() else {
            return;
        };
        for run in ended {
            let _ = brn_ai::run_lease::remove_free(&derived, run);
        }
    }
    /// [`Self::remove_ended_leases`] for an applied outcome.
    pub(crate) fn remove_leases_ended_by(&self, outcome: ApplyOutcome) -> ApplyOutcome {
        if let ApplyOutcome::Applied(receipt) = &outcome {
            self.remove_ended_leases(receipt);
        }
        outcome
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
        self.recover_orphaned_runs()?;
        Ok(self.chat_command(key, ChatCommand::Resume)?.receipt)
    }
    /// Interrupts Working Runs that no process holds a lease for, such as work
    /// left by a crash, kill or quit. Each gets the standard Interrupted notice
    /// so its chat offers Continue, a Review item for each candidate it left,
    /// and its leftover lease file is removed. Live Runs and Runs that
    /// finished meanwhile are left alone. Returns the interrupted Run IDs.
    pub fn recover_orphaned_runs(&mut self) -> AppResult<Vec<String>> {
        let records = self.records()?;
        let working: Vec<_> = records
            .iter()
            .filter(|r| {
                !r.archived
                    && matches!(&r.data, RecordData::Run(run) if run.state == RunState::Working)
            })
            .collect();
        if working.is_empty() {
            return Ok(vec![]);
        }
        // Without a usable lease folder liveness is unknown, so nothing is interrupted.
        let Ok(derived) = self.store.derived_directory() else {
            return Ok(vec![]);
        };
        let mut recovered = Vec::new();
        for record in working {
            if brn_ai::run_lease::run_is_live(&derived, &record.id) {
                continue;
            }
            let RecordData::Run(mut run) = record.data.clone() else {
                unreachable!()
            };
            run.state = RunState::Interrupted;
            run.fence = run
                .fence
                .checked_add(1)
                .ok_or_else(|| AppError::Invalid("run fence exhausted".into()))?;
            let thread = records.iter().find(|r| r.id == run.thread && !r.archived);
            let ended = [(record.id.clone(), run.clone())];
            let mut writes = vec![Put {
                id: record.id.clone(),
                expected_version: Some(record.version),
                archived: false,
                data: RecordData::Run(run),
            }];
            // The notice, plus a Review for each candidate the Run left.
            brn_threads_core::run_attention::ended_run_attention(
                &mut writes,
                &ended,
                true,
                |id| Ok(records.iter().find(|r| r.id == id).cloned()),
                |operations| self.store.active_candidates(operations),
            )?;
            // Same saved state, same key: a retry replays instead of repeating.
            let key = format!(
                "orphan-recovery:{}:{}:{}",
                record.id,
                record.version,
                thread.map_or(0, |t| t.version)
            );
            match self.owner_change(
                &key,
                &ChangeRequest {
                    reason: "Interrupt work whose process ended".into(),
                    writes,
                    inputs: vec![],
                },
            )? {
                ApplyOutcome::Applied(_) => {
                    // Its lease was free and the Run has ended: drop the leftover file.
                    let _ = brn_ai::run_lease::remove_free(&derived, &record.id);
                    recovered.push(record.id.clone());
                }
                // The Run finished or the chat changed meanwhile; the next check reads again.
                ApplyOutcome::Stale { .. } => {}
                other => {
                    return Err(AppError::Invalid(format!(
                        "Interrupted work could not be recovered: {other:?}"
                    )));
                }
            }
        }
        Ok(recovered)
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
        self.recover_orphaned_runs()?;
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
