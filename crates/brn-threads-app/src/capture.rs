//! Explicit review-only capture of posted chat outcomes; no archive or dispatch here.
use crate::*;
// Bound traversal as well as the accepted snapshot. Existing workspace context
// loading still reads records; this is not a process-wide allocation limit.
impl Workspace {
    /// Read-only consent preview. No request or Run exists until explicit save.
    pub fn preview_capture_request(&self, thread: &str) -> AppResult<CaptureSnapshot> {
        let chat = self
            .store
            .record(thread)?
            .filter(|r| {
                !r.archived
                    && matches!(&r.data, RecordData::Thread(t) if t.state == ThreadState::Open)
            })
            .ok_or_else(|| AppError::Invalid("Choose an open chat to capture outcomes".into()))?;
        let history_limit = || {
            AppError::Invalid("Capture supports up to 1024 history records and 256 posted messages within 128 KiB; start a focused chat".into())
        };
        // Count even metadata hidden by scoped paging, so metadata-only history
        // cannot make the existing visibility filter traverse an unlimited chat.
        if !self.store.capture_history_within_limit(thread)? {
            return Err(history_limit());
        }
        let mut messages = Vec::new();
        let (mut after, mut visited, mut raw_bytes) = (0, 0usize, 0usize);
        loop {
            let (page, more) = self.store.message_page(thread, after, 64)?;
            for (position, record) in page {
                after = position;
                visited += 1;
                if visited > MAX_CAPTURE_HISTORY_RECORDS {
                    return Err(history_limit());
                }
                if let RecordData::Message(message) = &record.data
                    && matches!(message.role.as_str(), "owner" | "assistant")
                {
                    raw_bytes = raw_bytes
                        .checked_add(message.text.len())
                        .ok_or_else(history_limit)?;
                    if messages.len() >= 256 || raw_bytes > MAX_CAPTURE_BYTES {
                        return Err(history_limit());
                    }
                    messages.push(record);
                }
            }
            if !more {
                break;
            }
        }
        let mut context = vec![RevisionInput {
            record: thread.into(),
            version: chat.version,
            expectation: InputExpectation::ChatContext,
        }];
        let project = self.store.project_context_record(thread)?;
        if let Some(project) = &project {
            context.push(RevisionInput {
                record: project.id.clone(),
                version: project.version,
                expectation: InputExpectation::Current,
            });
        }
        let snapshot = CaptureSnapshot {
            version: 1,
            request_message: "capture-preview".into(),
            thread: thread.into(),
            messages,
            context,
            private_context: self.store.chat_has_private_context(thread)?,
            project: project.map(|r| r.id.clone()),
        };
        snapshot.message_text()?;
        Ok(snapshot)
    }
    /// Save exactly the displayed snapshot, fencing concurrent source/scope changes.
    pub fn save_capture_request(
        &mut self,
        key: &str,
        preview: &CaptureSnapshot,
    ) -> AppResult<CaptureSnapshot> {
        let operation = self
            .store
            .allocate_operation(&format!("capture-request:{key}"))?;
        let mut snapshot = preview.clone();
        snapshot.request_message = operation.creation_id(0);
        if let Some(receipt) = self.store.receipt(&operation)? {
            let record = receipt
                .writes
                .iter()
                .find(|w| w.after.id == snapshot.request_message)
                .ok_or_else(|| AppError::Invalid("Capture receipt has no request".into()))?;
            let saved = CaptureSnapshot::from_message(&record.after)?;
            if saved != snapshot {
                return Err(AppError::Invalid(
                    "Capture request key belongs to different saved context".into(),
                ));
            }
            return Ok(saved);
        }
        let mut messages_match = true;
        for observed in &snapshot.messages {
            if self.store.record(&observed.id)?.as_ref() != Some(observed) {
                messages_match = false;
                break;
            }
        }
        if !messages_match
            || self
                .store
                .project_context_record(&snapshot.thread)?
                .as_ref()
                .map(|r| r.id.as_str())
                != snapshot.project.as_deref()
            || self.store.chat_has_private_context(&snapshot.thread)? != snapshot.private_context
        {
            return Err(AppError::Invalid("Conversation or destination changed; reopen Capture outcomes to review its current scope".into()));
        }
        let text = snapshot.message_text()?;
        let mut inputs = snapshot.current_inputs();
        inputs.retain(|i| i.record != snapshot.request_message);
        // The consent snapshot itself uses an exact chat revision. Candidate
        // inputs retain ChatContext so ordinary review attention is not stale.
        for input in &mut inputs {
            if input.expectation == InputExpectation::ChatContext {
                input.expectation = InputExpectation::Current;
            }
        }
        let request = ChangeRequest {
            reason: "Capture useful chat outcomes for owner review".into(),
            inputs,
            writes: vec![Put {
                id: snapshot.request_message.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Message(Message {
                    thread: snapshot.thread.clone(),
                    role: CAPTURE_REQUEST_ROLE.into(),
                    text,
                }),
            }],
        };
        let owner = HostAuthority::owner("explicit Capture outcomes");
        self.store.prepare_owner(&operation, &request, &owner)?;
        match self.store.apply(&operation, &owner)? {
            ApplyOutcome::Applied(_) => Ok(snapshot),
            outcome => Err(AppError::Invalid(format!(
                "Capture was not saved; conversation or scope may have changed. Reopen Capture outcomes. {outcome:?}"
            ))),
        }
    }
    pub fn snapshot_capture_request(
        &mut self,
        key: &str,
        thread: &str,
    ) -> AppResult<CaptureSnapshot> {
        let preview = self.preview_capture_request(thread)?;
        self.save_capture_request(key, &preview)
    }
}
