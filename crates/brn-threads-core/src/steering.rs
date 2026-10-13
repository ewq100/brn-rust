use crate::store::{apply_on, prepare_on, receipt_on, record_on};
use crate::*;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};

impl Store {
    /// Save a new owner instruction and fence the exact invocation atomically.
    /// Receipt replay precedes mutable destination and run checks.
    pub fn stop_and_post(
        &mut self,
        operation: &OperationId,
        run_id: &str,
        thread: &str,
        text: &str,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        self.stop_and_post_with_partial(operation, run_id, thread, text, None, authority)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn stop_and_post_with_partial(
        &mut self,
        operation: &OperationId,
        run_id: &str,
        thread: &str,
        text: &str,
        partial: Option<&str>,
        authority: &HostAuthority,
    ) -> Result<ApplyOutcome> {
        if !matches!(authority.scope, crate::types::Scope::Owner) {
            return Err(Error::Invalid(
                "stop and send requires owner authority".into(),
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Match the application allocation binding: include the command kind
        // and all exact input bytes, independent of current record versions.
        let binding = format!(
            "{:x}",
            Sha256::digest(if let Some(partial) = partial {
                serde_json::to_vec(&("stop-and-post-partial", run_id, thread, text, partial))?
            } else {
                serde_json::to_vec(&("stop-and-post", run_id, thread, text))?
            })
        );
        let (hash, prior): (Option<String>, Option<String>) = tx
            .query_row(
                "SELECT hash,input_hash FROM operations WHERE id=?1",
                [operation.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| Error::Missing(operation.as_str().into()))?;
        if prior.as_ref().is_some_and(|prior| prior != &binding)
            || (hash.is_some() && prior.is_none())
        {
            return Err(Error::ImmutableRequest);
        }
        tx.execute(
            "UPDATE operations SET input_hash=?2 WHERE id=?1 AND input_hash IS NULL",
            params![operation.as_str(), binding],
        )?;
        if receipt_on(&tx, operation)?.is_some() {
            let outcome = apply_on(&tx, operation, authority, None)?;
            tx.commit()?;
            return Ok(outcome);
        }
        if text.trim().is_empty() || text.len() > 128 * 1024 {
            return Err(Error::Invalid(
                "instruction must contain 1–131072 bytes".into(),
            ));
        }
        if !matches!(
            record_on(&tx, thread)?,
            Some(Record {
                archived: false,
                data: RecordData::Thread(Thread {
                    state: ThreadState::Open,
                    ..
                }),
                ..
            })
        ) {
            return Err(Error::Invalid(
                "destination must be an open active thread".into(),
            ));
        }
        let record = record_on(&tx, run_id)?.ok_or_else(|| Error::Missing(run_id.into()))?;
        let RecordData::Run(mut run) = record.data else {
            return Err(Error::Invalid("exact invocation is not a Run".into()));
        };
        if record.archived || run.thread != thread {
            return Err(Error::Invalid(
                "exact invocation does not belong to this thread".into(),
            ));
        }
        let other_working: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM records WHERE id<>?1 AND json_extract(body,'$.archived')=0 AND json_extract(body,'$.data.Run.thread')=?2 AND json_extract(body,'$.data.Run.state')='Working')",
            params![run_id, thread],
            |row| row.get(0),
        )?;
        if other_working {
            return Err(Error::Invalid(
                "another invocation is working in this thread".into(),
            ));
        }
        let mut writes = vec![Put {
            id: operation.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: thread.into(),
                role: "owner".into(),
                text: text.into(),
            }),
        }];
        if run.state == RunState::Working {
            if let Some(partial) = partial.filter(|p| !p.trim().is_empty()) {
                crate::chat::partial_text(partial)?;
                writes.push(Put {
                    id: operation.creation_id(1),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: thread.into(),
                        role: "assistant-partial".into(),
                        text: partial.into(),
                    }),
                });
            }

            run.state = RunState::Interrupted;
            run.fence = run
                .fence
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("run fence exhausted".into()))?;
            writes.push(Put {
                id: run_id.into(),
                expected_version: Some(record.version),
                archived: false,
                data: RecordData::Run(run),
            });
        }
        if let Some(record) = crate::chat::control_on(&tx)?
            && let RecordData::ChatControl(mut c) = record.data
            && let Some(q) = c
                .pending
                .as_mut()
                .filter(|q| q.thread == thread && q.run.as_deref() == Some(run_id))
        {
            q.text = text.into();
            q.message = Some(operation.creation_id(0));
            q.run = None;
            q.dispatch = Some(operation.as_str().into());
            writes.push(Put {
                id: record.id,
                expected_version: Some(record.version),
                archived: false,
                data: RecordData::ChatControl(c),
            });
        }
        prepare_on(
            &tx,
            operation,
            &ChangeRequest {
                reason: "Owner stopped and sent a new instruction".into(),
                writes,
                inputs: vec![],
            },
            true,
        )?;
        let outcome = apply_on(&tx, operation, authority, None)?;
        tx.commit()?;
        Ok(outcome)
    }
}
