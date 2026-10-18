use crate::*;
use std::collections::BTreeSet;

const ADDRESSED_ROLE: &str = "writing-addressed";
/// Saved feedback and derived explicit-owner addressed state.
#[derive(Debug, Clone)]
pub struct WritingComment {
    pub record: Record,
    pub addressed: bool,
}
/// An exact passage from the saved note and its selected conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingCommentInput {
    pub note: String,
    pub version: u64,
    pub thread: String,
    pub range: (usize, usize),
    pub quote: String,
    pub body: String,
}
/// Owner view of a configured link. Unavailable references have no copied title/content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WritingReferenceState {
    pub reference: WritingReference,
    /// None means "no longer available here"; do not fetch its title or content.
    pub version: Option<u64>,
}
#[derive(Debug, Clone)]
pub struct WritingSetupView {
    pub record: Option<Record>,
    pub references: Vec<WritingReferenceState>,
}
#[derive(Debug, Serialize, Deserialize)]
struct Addressed {
    version: u8,
    request_message: String,
    applied_operation: String,
    comments: Vec<Record>,
}
/// Resolved without creating a conversation or changing the owner's selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveWritingContext {
    pub note: String,
    pub scope: Option<String>,
    pub conversation: Option<String>,
}
impl Workspace {
    pub fn active_writing_context(
        &self,
        note: &str,
        opened_from: Option<&str>,
    ) -> AppResult<ActiveWritingContext> {
        let rows = self.records()?;
        let receipts = self.store.receipts()?;
        self.active_writing_context_from(note, opened_from, &rows, &receipts)
    }
    pub(crate) fn active_writing_context_from(
        &self,
        note: &str,
        opened_from: Option<&str>,
        rows: &[Record],
        receipts: &[Receipt],
    ) -> AppResult<ActiveWritingContext> {
        let saved = rows
            .iter()
            .find(|r| {
                r.id == note
                    && !r.archived
                    && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none())
            })
            .ok_or_else(|| AppError::Invalid("Working Note is unavailable".into()))?;
        let RecordData::Note(data) = &saved.data else {
            unreachable!()
        };
        let opening = opened_from.or(data.chat.as_deref());
        let scope = writing_setup_scope(rows, opening)?;
        // Receipt order is durable; arbitrary links and current window chat are
        // not a selection. V2 requests record the scope at the time of use.
        let mut conversation = None;
        let current = rows
            .iter()
            .map(|r| (r.id.as_str(), r))
            .collect::<BTreeMap<_, _>>();
        let mut thread_scopes = BTreeMap::new();
        for receipt in receipts {
            for write in &receipt.writes {
                if write.before.is_some()
                    || write.after.archived
                    || !current
                        .get(write.after.id.as_str())
                        .is_some_and(|r| !r.archived)
                {
                    continue;
                }
                let used = match &write.after.data {
                    RecordData::Message(m) if m.role == WRITING_CONTEXT_ROLE => {
                        serde_json::from_str::<WritingContextUse>(&m.text).ok()
                    }
                    RecordData::Message(m) if m.role == WRITING_REQUEST_ROLE => {
                        WritingSnapshot::from_message(&write.after)
                            .ok()
                            .and_then(|s| {
                                s.setup.map(|setup| WritingContextUse {
                                    note: s.note.id,
                                    chat: setup.chat,
                                    thread: s.thread,
                                })
                            })
                    }
                    _ => None,
                };
                if let Some(used) = used
                    && used.note == note
                    && used.chat == scope
                {
                    // A moved conversation never follows the Note into another scope.
                    if thread_scopes
                        .entry(used.thread.clone())
                        .or_insert_with(|| writing_request_scope(rows, &used.thread).ok())
                        .as_ref()
                        == Some(&scope)
                        && current
                            .get(used.thread.as_str())
                            .is_some_and(|r| matches!(r.data, RecordData::Thread(_)))
                    {
                        conversation = Some(used.thread);
                    }
                }
            }
        }
        // CW1's actual selected-conversation exception applies to this working
        // Note only. References and evidence keep their canonical scope.
        if !record_in_chat(saved, rows, opening)
            && !conversation
                .as_deref()
                .is_some_and(|thread| record_in_chat(saved, rows, Some(thread)))
        {
            return Err(AppError::Invalid(
                "Working Note is unavailable in this writing context".into(),
            ));
        }
        Ok(ActiveWritingContext {
            note: note.into(),
            scope,
            conversation,
        })
    }
    fn writing_context_put(&self, id: String, note: &str, thread: &str) -> AppResult<Put> {
        let chat = writing_request_scope(&self.records()?, thread)?;
        Ok(Put {
            id,
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: thread.into(),
                role: WRITING_CONTEXT_ROLE.into(),
                text: serde_json::to_string(&WritingContextUse {
                    note: note.into(),
                    chat,
                    thread: thread.into(),
                })?,
            }),
        })
    }
    /// Explicit owner selection, distinct from inspection and programmatic refresh.
    pub fn select_writing_conversation(
        &mut self,
        key: &str,
        note: &str,
        thread: &str,
    ) -> AppResult<Option<Receipt>> {
        // Repeated explicit picks do not create operations, Messages or receipts.
        if self
            .active_writing_context(note, Some(thread))?
            .conversation
            .as_deref()
            == Some(thread)
        {
            return Ok(None);
        }
        let op = self.writing_operation("context", key, (note, thread))?;
        if let Some(receipt) = self.writing_replay(&op)? {
            return Ok(Some(receipt));
        }
        let saved = self.writing_note(note, None)?;
        let chat = self.writing_thread(thread)?;
        let link = self.writing_link(note, thread)?;
        if !self.store.record_visible_in_chat(note, Some(thread))? {
            return Err(AppError::Invalid(
                "Note is unavailable in selected writing conversation".into(),
            ));
        }
        self.writing_commit(
            &op,
            ChangeRequest {
                reason: "Owner explicitly selected a writing conversation".into(),
                writes: vec![self.writing_context_put(op.creation_id(0), note, thread)?],
                inputs: current_records([&saved, &chat, &link]),
            },
        )
        .map(Some)
    }
    /// First comment/rewrite creates a scoped conversation only when no explicit
    /// selection exists. Private scope is its existing host-owned conversation.
    pub fn ensure_writing_context(
        &mut self,
        key: &str,
        note: &str,
        opened_from: Option<&str>,
    ) -> AppResult<ActiveWritingContext> {
        let context = self.active_writing_context(note, opened_from)?;
        if let Some(thread) = &context.conversation {
            self.writing_thread(thread)?;
            self.writing_link(note, thread)?;
            return Ok(context);
        }
        self.new_writing_conversation(key, note, opened_from)
    }
    /// Explicit new conversation in the same canonical scope. A private scope
    /// already is a conversation and cannot silently become general/Project.
    pub fn new_writing_conversation(
        &mut self,
        key: &str,
        note: &str,
        opened_from: Option<&str>,
    ) -> AppResult<ActiveWritingContext> {
        let mut context = self.active_writing_context(note, opened_from)?;
        let rows = self.records()?;
        let scope = context
            .scope
            .as_deref()
            .and_then(|id| rows.iter().find(|r| r.id == id));
        let thread = match scope {
            Some(Record {
                id,
                data: RecordData::Thread(_),
                ..
            }) => {
                self.writing_thread(id)?;
                id.clone()
            }
            Some(Record {
                id,
                archived: false,
                data: RecordData::Project(_),
                ..
            }) => {
                let receipt = self.project_command(
                    &format!("writing-context:{key}"),
                    ProjectCommand::CreateChat {
                        project: id.clone(),
                        title: "Writing".into(),
                    },
                )?;
                receipt
                    .writes
                    .into_iter()
                    .find(|w| matches!(w.after.data, RecordData::Thread(_)))
                    .ok_or_else(|| {
                        AppError::Invalid("Writing conversation was not created".into())
                    })?
                    .after
                    .id
            }
            Some(_) => return Err(AppError::Invalid("Writing scope is inactive".into())),
            None if context.scope.is_none() => {
                first_record(
                    self.new_thread_with_key(&format!("writing-context:{key}"), "Writing")?,
                )?
                .id
            }
            None => return Err(AppError::Invalid("Writing scope is unavailable".into())),
        };
        let saved = self.writing_note(note, None)?;
        if self.writing_link(note, &thread).is_err() {
            self.link_writing_thread(&format!("context-link:{key}"), note, saved.version, &thread)?;
        } else {
            self.select_writing_conversation(&format!("context-use:{key}"), note, &thread)?;
        }
        context.conversation = Some(thread);
        Ok(context)
    }
}
impl Workspace {
    /// Read exactly this host-selected writing context, never another setup on a shared Note.
    pub fn writing_setup(&self, note: &str, chat: Option<&str>) -> AppResult<WritingSetupView> {
        self.writing_note(note, None)?;
        let rows = self.records()?;
        let scope = writing_setup_scope(&rows, chat)?;
        if !rows
            .iter()
            .any(|r| r.id == note && record_in_chat(r, &rows, chat))
        {
            return Err(AppError::Invalid(
                "Working Note is unavailable in this writing scope".into(),
            ));
        }
        let record = rows.iter().find(|r| !r.archived && matches!(&r.data, RecordData::WritingSetup(s) if s.note == note && s.chat == scope)).cloned();
        let references = record
            .as_ref()
            .map(|r| match &r.data {
                RecordData::WritingSetup(setup) => setup
                    .references
                    .iter()
                    .map(|reference| {
                        let version = rows.iter().find(|r| {
                            r.id == reference.note && !r.archived
                                && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none())
                                && record_in_chat(r, &rows, scope.as_deref())
                        }).map(|r| r.version);
                        WritingReferenceState { reference: reference.clone(), version }
                    })
                    .collect(),
                _ => unreachable!(),
            })
            .unwrap_or_default();
        Ok(WritingSetupView { record, references })
    }
    /// Owner-only versioned configuration. None means no active setup; an archived
    /// identity is reused after Undo, so each context always has one record.
    pub fn set_writing_setup(
        &mut self,
        key: &str,
        expected_version: Option<u64>,
        setup: &WritingSetup,
    ) -> AppResult<Receipt> {
        let operation = self.writing_operation("setup", key, (expected_version, setup))?;
        if let Some(receipt) = self.writing_replay(&operation)? {
            return Ok(receipt);
        }
        let rows = self.records()?;
        let scope = writing_setup_scope(&rows, setup.chat.as_deref())?;
        let conversation = setup.chat.as_deref().and_then(|id| {
            rows.iter()
                .find(|r| r.id == id && matches!(r.data, RecordData::Thread(_)))
        });
        if let Some(thread) = conversation {
            self.writing_thread(&thread.id)?;
        }
        let mut setup = setup.clone();
        setup.chat = scope.clone();
        let current = rows.iter().find(|r| matches!(&r.data, RecordData::WritingSetup(s) if s.note == setup.note && s.chat == scope));
        if current.filter(|r| !r.archived).map(|r| r.version) != expected_version {
            return Err(AppError::Invalid(
                "Writing setup changed; reload before saving".into(),
            ));
        }
        self.writing_commit(
            &operation,
            ChangeRequest {
                reason: "Owner updated scoped writing setup".into(),
                writes: vec![Put {
                    id: current
                        .map(|r| r.id.clone())
                        .unwrap_or_else(|| operation.creation_id(0)),
                    expected_version: current.map(|r| r.version),
                    archived: false,
                    data: RecordData::WritingSetup(setup.clone()),
                }],
                inputs: current_records(conversation),
            },
        )
    }
    fn writing_operation(
        &mut self,
        kind: &str,
        key: &str,
        input: impl Serialize,
    ) -> AppResult<OperationId> {
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&(kind, input))?));
        Ok(self
            .store
            .allocate_bound_operation(&format!("writing-{kind}:{key}"), &binding)?)
    }
    fn writing_replay(&mut self, operation: &OperationId) -> AppResult<Option<Receipt>> {
        if let Some(receipt) = self.store.receipt(operation)? {
            return Ok(Some(receipt));
        }
        match self.store.prepared(operation) {
            Ok(_) => Ok(Some(applied_receipt(self.store.apply(
                operation,
                &HostAuthority::owner("owner writing interaction"),
            )?)?)),
            Err(brn_threads_core::Error::Missing(_)) => Ok(None),
            Err(brn_threads_core::Error::Invalid(reason)) if reason == "operation not prepared" => {
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }
    fn writing_commit(
        &mut self,
        operation: &OperationId,
        request: ChangeRequest,
    ) -> AppResult<Receipt> {
        let owner = HostAuthority::owner("owner writing interaction");
        self.store.prepare_owner(operation, &request, &owner)?;
        applied_receipt(self.store.apply(operation, &owner)?)
    }
    fn writing_note(&self, id: &str, version: Option<u64>) -> AppResult<Record> {
        let record = self
            .store
            .record(id)?
            .ok_or_else(|| AppError::Invalid("Writing note is missing".into()))?;
        if record.archived
            || version.is_some_and(|version| version != record.version)
            || !matches!(&record.data, RecordData::Note(note) if note.superseded_by.is_none())
        {
            return Err(AppError::Invalid(
                "The saved note changed. Refresh the saved passage before continuing".into(),
            ));
        }
        Ok(record)
    }
    fn writing_thread(&self, id: &str) -> AppResult<Record> {
        let record = self
            .store
            .record(id)?
            .ok_or_else(|| AppError::Invalid("Writing conversation is missing".into()))?;
        if record.archived
            || matches!(&record.data, RecordData::Thread(t) if t.state == ThreadState::Resolved)
        {
            return Err(AppError::Invalid("Unarchive this conversation before continuing writing. Your feedback is preserved.".into()));
        }
        if record.archived
            || !matches!(&record.data, RecordData::Thread(thread) if thread.state == ThreadState::Open)
        {
            return Err(AppError::Invalid(
                "Select an open linked conversation".into(),
            ));
        }
        Ok(record)
    }
    fn writing_link(&self, note: &str, thread: &str) -> AppResult<Record> {
        self.records()?.into_iter().find(|record| !record.archived && matches!(&record.data, RecordData::Link(link) if link.from == thread && link.to == note || link.from == note && link.to == thread)).ok_or_else(|| AppError::Invalid("Select a conversation linked to this note".into()))
    }
    /// List note-linked conversations without copying note or chat content.
    pub fn writing_threads(&self, note: &str) -> AppResult<Vec<Record>> {
        self.writing_note(note, None)?;
        let records = self.records()?;
        let mut linked = BTreeSet::new();
        for record in &records {
            if !record.archived
                && let RecordData::Link(link) = &record.data
            {
                if link.from == note {
                    linked.insert(link.to.as_str());
                }
                if link.to == note {
                    linked.insert(link.from.as_str());
                }
            }
        }
        Ok(records
            .iter()
            .filter(|record| {
                !record.archived
                    && linked.contains(record.id.as_str())
                    && matches!(record.data, RecordData::Thread(_))
            })
            .cloned()
            .collect())
    }
    /// Durably link an open conversation to this exact saved note version.
    pub fn link_writing_thread(
        &mut self,
        key: &str,
        note: &str,
        version: u64,
        thread: &str,
    ) -> AppResult<Receipt> {
        let operation = self.writing_operation("link", key, (note, version, thread))?;
        if let Some(receipt) = self.writing_replay(&operation)? {
            return Ok(receipt);
        }
        let note_record = self.writing_note(note, Some(version))?;
        let thread_record = self.writing_thread(thread)?;
        self.writing_commit(
            &operation,
            ChangeRequest {
                reason: "Owner linked a writing conversation".into(),
                writes: vec![
                    Put {
                        id: operation.creation_id(0),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Link(Link {
                            from: thread.into(),
                            to: note.into(),
                            relation: "discusses".into(),
                        }),
                    },
                    self.writing_context_put(operation.creation_id(1), note, thread)?,
                ],
                inputs: current_records([&note_record, &thread_record]),
            },
        )
    }
    /// Save pending feedback at an exact UTF-8 saved passage; never start a run.
    pub fn add_writing_comment(
        &mut self,
        key: &str,
        input: &WritingCommentInput,
    ) -> AppResult<Record> {
        let operation = self.writing_operation("comment", key, input)?;
        if let Some(receipt) = self.writing_replay(&operation)? {
            return receipt_record(&receipt, &operation.creation_id(0));
        }
        let note = self.writing_note(&input.note, Some(input.version))?;
        let thread = self.writing_thread(&input.thread)?;
        let link = self.writing_link(&input.note, &input.thread)?;
        let comment = Comment {
            note: input.note.clone(),
            base_version: input.version,
            quote: input.quote.clone(),
            range: Some(input.range),
            mapped_version: None,
            mapped_range: None,
            body: input.body.clone(),
            unresolved: false,
        };
        let RecordData::Note(note_data) = &note.data else {
            unreachable!()
        };
        if input.body.trim().is_empty()
            || exact_writing_range(&comment, note.version, &note_data.markdown).is_none()
        {
            return Err(AppError::Invalid(
                "Select an exact nonempty saved passage and enter a comment".into(),
            ));
        }
        let receipt = self.writing_commit(
            &operation,
            ChangeRequest {
                reason: "Owner added pending passage feedback".into(),
                writes: vec![
                    Put {
                        id: operation.creation_id(0),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Comment(comment),
                    },
                    Put {
                        id: operation.creation_id(1),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Link(Link {
                            from: input.thread.clone(),
                            to: operation.creation_id(0),
                            relation: "passage comment".into(),
                        }),
                    },
                    self.writing_context_put(operation.creation_id(2), &input.note, &input.thread)?,
                ],
                inputs: current_records([&note, &thread, &link]),
            },
        )?;
        receipt_record(&receipt, &operation.creation_id(0))
    }
    /// Load an immutable request and verify every captured record against canonical revisions.
    pub fn writing_request(&self, message: &str) -> AppResult<WritingSnapshot> {
        let record = self
            .store
            .record(message)?
            .ok_or_else(|| AppError::Invalid("Saved writing request is missing".into()))?;
        self.writing_request_from_record(&record)
    }
    pub(crate) fn writing_request_from_record(
        &self,
        record: &Record,
    ) -> AppResult<WritingSnapshot> {
        let snapshot = WritingSnapshot::from_message(record)?;
        for captured in std::iter::once(&snapshot.note)
            .chain(snapshot.comments.iter())
            .chain(snapshot.messages.iter())
        {
            if self
                .store
                .revision(&captured.id, captured.version)?
                .as_ref()
                != Some(captured)
            {
                return Err(AppError::Invalid(
                    "Writing request does not match canonical saved evidence".into(),
                ));
            }
        }
        snapshot.validate_saved_setup(&self.store)?;
        Ok(snapshot)
    }
    /// Inspect a saved request only within the shared resolved displayed context.
    /// This also works before a candidate/evidence Message exists.
    pub fn writing_request_in_context(
        &self,
        message: &str,
        note: &str,
        displayed: Option<&str>,
    ) -> AppResult<WritingSnapshot> {
        let rows = self.records()?;
        let receipts = self.store.receipts()?;
        let context = self.active_writing_context_from(note, displayed, &rows, &receipts)?;
        if context.scope != writing_setup_scope(&rows, displayed)? {
            return Err(AppError::Invalid(
                "Saved writing request is unavailable in this displayed context".into(),
            ));
        }
        let record = rows
            .iter()
            .find(|r| r.id == message)
            .ok_or_else(|| AppError::Invalid("Saved writing request is missing".into()))?;
        self.writing_request_in_context_from(record, &context, &rows)
    }
    pub(crate) fn writing_request_in_context_from(
        &self,
        record: &Record,
        context: &ActiveWritingContext,
        rows: &[Record],
    ) -> AppResult<WritingSnapshot> {
        let snapshot = self.writing_request_from_record(record)?;
        if snapshot.note.id != context.note
            || snapshot.setup.is_some()
                && context.conversation.as_deref() != Some(snapshot.thread.as_str())
            || snapshot
                .setup
                .as_ref()
                .is_some_and(|setup| setup.chat != context.scope)
            || writing_request_scope(rows, &snapshot.thread)? != context.scope
            || record.archived
            || !record_in_chat(record, rows, Some(&snapshot.thread))
        {
            return Err(AppError::Invalid(
                "Saved writing request is unavailable in this displayed context".into(),
            ));
        }
        Ok(snapshot)
    }
    /// Persist exact saved note, selected pending feedback and all posted chat before generation.
    /// The complete JSON is bounded to 128 KiB; overflow fails without truncation.
    /// Reusing the same key and arguments returns the original context after later chat.
    pub fn snapshot_writing_request(
        &mut self,
        key: &str,
        note: &str,
        version: u64,
        thread: &str,
        comment_ids: &[String],
    ) -> AppResult<WritingSnapshot> {
        self.snapshot_writing_request_with_feedback(key, note, version, thread, comment_ids, "")
    }
    /// Persist whole-note owner feedback atomically with the exact rewrite inputs.
    /// Feedback is part of the bounded immutable request, never a posted chat message.
    pub fn snapshot_writing_request_with_feedback(
        &mut self,
        key: &str,
        note: &str,
        version: u64,
        thread: &str,
        comment_ids: &[String],
        feedback: &str,
    ) -> AppResult<WritingSnapshot> {
        self.snapshot_writing_request_with_requirements(
            key,
            note,
            version,
            thread,
            comment_ids,
            feedback,
            &WritingReadRequirements::default(),
        )
    }
    /// Freshness is explicit owner input, never inferred from model instructions.
    #[allow(clippy::too_many_arguments)]
    pub fn snapshot_writing_request_with_requirements(
        &mut self,
        key: &str,
        note: &str,
        version: u64,
        thread: &str,
        comment_ids: &[String],
        feedback: &str,
        requirements: &WritingReadRequirements,
    ) -> AppResult<WritingSnapshot> {
        let operation = if !requirements.is_default() {
            self.writing_operation(
                "request",
                key,
                (note, version, thread, comment_ids, feedback, requirements),
            )?
        } else if feedback.is_empty() {
            // Preserve intent bindings for existing empty-feedback callers and retries.
            self.writing_operation("request", key, (note, version, thread, comment_ids))?
        } else {
            self.writing_operation(
                "request",
                key,
                (note, version, thread, comment_ids, feedback),
            )?
        };
        if let Some(receipt) = self.writing_replay(&operation)? {
            return Ok(WritingSnapshot::from_message(&receipt_record(
                &receipt,
                &operation.creation_id(0),
            )?)?);
        }
        let note_record = self.writing_note(note, Some(version))?;
        let thread_record = self.writing_thread(thread)?;
        let link = self.writing_link(note, thread)?;
        let available = self.writing_comments(note)?;
        let mut chosen = BTreeSet::new();
        let comments = comment_ids
            .iter()
            .map(|id| {
                if !chosen.insert(id) {
                    return Err(AppError::Invalid(
                        "A comment was selected more than once".into(),
                    ));
                }
                available
                    .iter()
                    .find(|comment| comment.record.id == *id && !comment.addressed)
                    .map(|comment| comment.record.clone())
                    .ok_or_else(|| {
                        AppError::Invalid(
                            "Selected feedback is missing or already addressed".into(),
                        )
                    })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let messages = self.messages(thread)?.into_iter().filter(|record| matches!(&record.data,RecordData::Message(message) if matches!(message.role.as_str(),"owner" | "assistant"))).collect();
        let rows = self.records()?;
        let scope = writing_request_scope(&rows, thread)?;
        let view = self.writing_setup(note, Some(thread))?;
        let stored = rows.iter().find(
            |r| matches!(&r.data, RecordData::WritingSetup(s) if s.note == note && s.chat == scope),
        );
        let active = view.record.as_ref();
        let setup = active
            .and_then(|r| match &r.data {
                RecordData::WritingSetup(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or(WritingSetup {
                note: note.into(),
                chat: scope.clone(),
                brief: None,
                references: vec![],
            });
        let setup_id = stored
            .map(|r| r.id.clone())
            .unwrap_or_else(|| operation.creation_id(1));
        let setup_version = active
            .map(|r| r.version)
            .unwrap_or_else(|| stored.map(|r| r.version + 1).unwrap_or(1));
        let new_setup = active.is_none().then(|| Put {
            id: setup_id.clone(),
            expected_version: stored.map(|r| r.version),
            archived: false,
            data: RecordData::WritingSetup(setup.clone()),
        });
        let snapshot = WritingSnapshot {
            setup: Some(WritingSnapshotSetup {
                requirements: requirements.clone(),
                chat: scope,
                brief: setup.brief,
                input: RevisionInput {
                    record: setup_id.clone(),
                    version: setup_version,
                    expectation: InputExpectation::Current,
                },
                references: view
                    .references
                    .into_iter()
                    .filter_map(|r| {
                        r.version.map(|version| WritingReferenceVersion {
                            note: r.reference.note,
                            version,
                            purpose: r.reference.purpose,
                        })
                    })
                    .collect(),
            }),
            request_message: operation.creation_id(0),
            thread: thread.into(),
            note: note_record,
            comments,
            messages,
            general_feedback: feedback.into(),
        };
        let text = snapshot.message_text()?;
        let mut inputs = snapshot.current_inputs();
        inputs.retain(|input| {
            input.record != snapshot.request_message
                && !(new_setup.is_some() && input.record == setup_id)
        });
        inputs.extend(current_records([&thread_record, &link]));
        let mut writes = vec![Put {
            id: snapshot.request_message.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: thread.into(),
                role: WRITING_REQUEST_ROLE.into(),
                text,
            }),
        }];
        writes.extend(new_setup);
        // Bind exact reference revisions without claiming they must stay current.
        // CW2 adds only references actually read to candidates and rechecks all Apply visibility.
        inputs.extend(
            snapshot
                .setup
                .iter()
                .flat_map(|s| s.references.iter())
                .map(|r| RevisionInput {
                    record: r.note.clone(),
                    version: r.version,
                    expectation: InputExpectation::Historical,
                }),
        );
        if let Some(project) = project_scope(&rows, Some(thread)) {
            inputs.extend(current_records([project]));
        }
        let owner = HostAuthority::owner("owner writing interaction")
            .for_chat(thread, self.store.chat_has_private_context(thread)?);
        self.store.prepare_owner(
            &operation,
            &ChangeRequest {
                reason: "Owner saved exact writing rewrite context before generation".into(),
                writes,
                inputs,
            },
            &owner,
        )?;
        applied_receipt(self.store.apply(&operation, &owner)?)?;
        Ok(snapshot)
    }
    fn addressed_proof(&self, proof: &Addressed) -> AppResult<WritingSnapshot> {
        if proof.version != 1 || proof.comments.is_empty() {
            return Err(AppError::Invalid(
                "Addressed feedback proof is invalid".into(),
            ));
        }
        let snapshot = self.writing_request(&proof.request_message)?;
        let operation = self.store.operation(&proof.applied_operation)?;
        let receipt = self.store.receipt(&operation)?.ok_or_else(|| {
            AppError::Invalid("Apply the writing proposal before marking feedback addressed".into())
        })?;
        let prepared = self.store.prepared(&operation)?;
        if !prepared.request.inputs.contains(&RevisionInput {
            record: proof.request_message.clone(),
            version: 1,
            expectation: InputExpectation::Current,
        }) || !receipt.writes.iter().any(|write| {
            write.before.as_ref() == Some(&snapshot.note)
                && write.after.id == snapshot.note.id
                && matches!(write.after.data, RecordData::Note(_))
        }) {
            return Err(AppError::Invalid(
                "This receipt does not apply the captured writing proposal".into(),
            ));
        }
        let mut seen = BTreeSet::new();
        if proof
            .comments
            .iter()
            .any(|comment| !seen.insert(&comment.id) || !snapshot.comments.contains(comment))
        {
            return Err(AppError::Invalid(
                "Only feedback captured in this writing request can be addressed".into(),
            ));
        }
        if self
            .history()?
            .iter()
            .any(|receipt| receipt.undo_of.as_ref() == Some(&operation))
        {
            return Err(AppError::Invalid(
                "This writing change was undone; its feedback remains pending".into(),
            ));
        }
        Ok(snapshot)
    }
    /// List feedback; addressed state requires explicit confirmation of a matching, un-undone Apply.
    /// Mapping metadata changes preserve confirmation; changed feedback content remains pending.
    pub fn writing_comments(&self, note: &str) -> AppResult<Vec<WritingComment>> {
        self.writing_note(note, None)?;
        let records = self.records()?;
        let mut addressed = BTreeMap::<String, Vec<Record>>::new();
        for record in &records {
            if record.archived || record.version != 1 {
                continue;
            }
            if let RecordData::Message(message) = &record.data
                && message.role == ADDRESSED_ROLE
                && let Ok(proof) = serde_json::from_str::<Addressed>(&message.text)
                && let Ok(snapshot) = self.addressed_proof(&proof)
                && snapshot.note.id == note
                && message.thread == snapshot.thread
            {
                for comment in proof.comments {
                    addressed
                        .entry(comment.id.clone())
                        .or_default()
                        .push(comment);
                }
            }
        }
        Ok(records
            .into_iter()
            .filter(|record| {
                !record.archived
                    && matches!(&record.data,RecordData::Comment(comment) if comment.note == note)
            })
            .map(|record| WritingComment {
                addressed: addressed.get(&record.id).is_some_and(|captured| {
                    captured
                        .iter()
                        .any(|captured| same_feedback(captured, &record))
                }),
                record,
            })
            .collect())
    }
    /// Record explicit owner confirmation only for captured feedback after its actual Apply.
    /// Generation, Apply and dismissal themselves leave feedback pending.
    /// Mapping-only revisions are accepted, while feedback fields must still match the captured request.
    pub fn mark_writing_comments_addressed(
        &mut self,
        key: &str,
        request_message: &str,
        applied_operation: &str,
        comment_ids: &[String],
    ) -> AppResult<Receipt> {
        let operation = self.writing_operation(
            "addressed",
            key,
            (request_message, applied_operation, comment_ids),
        )?;
        if let Some(receipt) = self.writing_replay(&operation)? {
            return Ok(receipt);
        }
        let snapshot = self.writing_request(request_message)?;
        let mut seen = BTreeSet::new();
        let comments = comment_ids
            .iter()
            .map(|id| {
                if !seen.insert(id) {
                    return Err(AppError::Invalid("Feedback selected twice".into()));
                }
                snapshot
                    .comments
                    .iter()
                    .find(|comment| comment.id == *id)
                    .cloned()
                    .ok_or_else(|| {
                        AppError::Invalid("Only captured feedback can be addressed".into())
                    })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let proof = Addressed {
            version: 1,
            request_message: request_message.into(),
            applied_operation: applied_operation.into(),
            comments,
        };
        // Capture before the no-Undo proof so a concurrent Undo makes this input stale.
        let note = self.writing_note(&snapshot.note.id, None)?;
        self.addressed_proof(&proof)?;
        // Fence the current versions while preserving the original captured feedback proof.
        // Location mapping may advance a Comment without changing the owner's feedback.
        let current_comments = proof
            .comments
            .iter()
            .map(|captured| {
                let current = self
                    .store
                    .record(&captured.id)?
                    .ok_or_else(|| AppError::Invalid("Captured feedback is missing".into()))?;
                if !same_feedback(captured, &current) {
                    return Err(AppError::Invalid(
                        "Captured feedback changed after this writing request; it remains pending"
                            .into(),
                    ));
                }
                Ok(current)
            })
            .collect::<AppResult<Vec<_>>>()?;
        let mut inputs = current_records(current_comments.iter());
        inputs.extend(current_records([&note]));
        inputs.push(RevisionInput {
            record: request_message.into(),
            version: 1,
            expectation: InputExpectation::Current,
        });
        self.writing_commit(
            &operation,
            ChangeRequest {
                reason: "Owner explicitly marked captured writing feedback addressed after Apply"
                    .into(),
                writes: vec![Put {
                    id: operation.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: snapshot.thread,
                        role: ADDRESSED_ROLE.into(),
                        text: serde_json::to_string(&proof)?,
                    }),
                }],
                inputs,
            },
        )
    }
}
fn current_records<'a>(records: impl IntoIterator<Item = &'a Record>) -> Vec<RevisionInput> {
    records
        .into_iter()
        .map(|record| RevisionInput {
            record: record.id.clone(),
            version: record.version,
            expectation: InputExpectation::Current,
        })
        .collect()
}
fn receipt_record(receipt: &Receipt, id: &str) -> AppResult<Record> {
    receipt
        .writes
        .iter()
        .find(|write| write.after.id == id)
        .map(|write| write.after.clone())
        .ok_or_else(|| AppError::Invalid("Writing operation has no matching saved record".into()))
}

// Feedback identity is its saved original anchor and instruction. Current location
// mapping and uncertainty are presentation metadata, independent of addressed state.
fn same_feedback(captured: &Record, current: &Record) -> bool {
    if captured.id != current.id || current.archived {
        return false;
    }
    match (&captured.data, &current.data) {
        (RecordData::Comment(captured), RecordData::Comment(current)) => {
            captured.note == current.note
                && captured.base_version == current.base_version
                && captured.range == current.range
                && captured.quote == current.quote
                && captured.body == current.body
        }
        _ => false,
    }
}
