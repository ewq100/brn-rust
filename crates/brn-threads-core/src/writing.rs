//! Scoped checked setup and immutable host-owned writing requests.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Host-owned Message role that durably binds proposal-only writing work.
pub const WRITING_REQUEST_ROLE: &str = "writing-request";
/// Complete encoded context bound; overflow must never truncate evidence.
pub const MAX_WRITING_REQUEST_BYTES: usize = 128 * 1024;
/// Purpose of a reference link; no reference content is copied into setup or requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WritingPurpose {
    Requirements,
    Evidence,
    Style,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WritingReference {
    pub note: String,
    pub purpose: WritingPurpose,
}
/// One small owner configuration record per working Note and canonical scope.
/// `chat` uses existing scope identities: None, a Project ID, or a private Thread ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WritingSetup {
    pub note: String,
    pub chat: Option<String>,
    pub brief: Option<String>,
    pub references: Vec<WritingReference>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WritingReferenceVersion {
    pub note: String,
    pub version: u64,
    pub purpose: WritingPurpose,
}
/// V2 metadata only. Setup content is captured; reference content is never captured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WritingSnapshotSetup {
    pub chat: Option<String>,
    pub input: RevisionInput,
    pub brief: Option<String>,
    pub references: Vec<WritingReferenceVersion>,
}
/// Resolve a host-selected scope to its existing Project or Thread identity.
/// Passing None explicitly selects general BRN. A private Thread never expands
/// its saved setup into a project when the Thread subsequently moves.
pub fn writing_setup_scope(rows: &[Record], chat: Option<&str>) -> Result<Option<String>> {
    let Some(chat) = chat else {
        return Ok(None);
    };
    if let Some(project) = crate::context::project_scope(rows, Some(chat)) {
        if !project.archived {
            return Ok(Some(project.id.clone()));
        }
    } else if rows.iter().any(|r| {
        r.id == chat
            && !r.archived
            && matches!(&r.data, RecordData::Thread(t) if t.state == ThreadState::Open)
    }) {
        return Ok(Some(chat.into()));
    }
    Err(Error::Invalid(
        "Writing scope is missing or inactive".into(),
    ))
}
/// The scope of an existing conversation, as established by host-owned state.
pub fn writing_request_scope(rows: &[Record], chat: &str) -> Result<Option<String>> {
    if crate::context::project_scope(rows, Some(chat)).is_some()
        || crate::context::chat_private(rows, chat)
    {
        writing_setup_scope(rows, Some(chat))
    } else {
        writing_setup_scope(rows, Some(chat))?;
        Ok(None)
    }
}
impl WritingSetup {
    pub(crate) fn validate(&self) -> Result<()> {
        let mut ids = BTreeSet::new();
        if self.note.is_empty()
            || self.chat.as_ref().is_some_and(String::is_empty)
            || self
                .references
                .iter()
                .any(|r| r.note.is_empty() || r.note == self.note || !ids.insert(&r.note))
            || serde_json::to_vec(self)?.len() > MAX_WRITING_REQUEST_BYTES
        {
            return Err(Error::Invalid("Writing setup requires unique reference Notes and at most 128 KiB; nothing was truncated".into()));
        }
        Ok(())
    }
}
/// Canonical saved-record snapshots for one explicitly selected note rewrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WritingSnapshot {
    pub request_message: String,
    pub thread: String,
    pub note: Record,
    pub comments: Vec<Record>,
    pub messages: Vec<Record>,
    /// Owner feedback about the whole note, bound by the immutable request.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub general_feedback: String,
    /// Absent preserves the exact legacy v1 serialization and input binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup: Option<WritingSnapshotSetup>,
}
#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u8,
    snapshot: WritingSnapshot,
}
/// Scoped request payloads are host configuration evidence, not ordinary chat
/// history. Dedicated writing capture is their only model-facing entry point.
pub fn scoped_writing_request(record: &Record) -> bool {
    matches!(&record.data, RecordData::Message(m) if m.role == WRITING_REQUEST_ROLE
        && serde_json::from_str::<serde_json::Value>(&m.text).ok().is_some_and(|v| v["version"] == 2))
}
impl WritingSnapshot {
    fn validate(&self) -> Result<()> {
        let RecordData::Note(note) = &self.note.data else {
            return Err(Error::Invalid("Writing context requires a note".into()));
        };
        if self.note.archived
            || note.superseded_by.is_some()
            || self.note.version == 0
            || self.thread.is_empty()
            || self.request_message.is_empty()
        {
            return Err(Error::Invalid(
                "Writing context has an inactive note or missing identity".into(),
            ));
        }
        let mut ids = BTreeSet::from([self.note.id.as_str(), self.request_message.as_str()]);
        for record in &self.comments {
            let RecordData::Comment(comment) = &record.data else {
                return Err(Error::Invalid("Writing feedback requires comments".into()));
            };
            if record.archived
                || record.version == 0
                || !ids.insert(&record.id)
                || comment.note != self.note.id
                || exact_writing_range(comment, self.note.version, &note.markdown).is_none()
            {
                return Err(Error::Invalid(
                    "Writing feedback needs an exact current saved passage".into(),
                ));
            }
        }
        for record in &self.messages {
            if record.archived
                || record.version == 0
                || !ids.insert(&record.id)
                || !matches!(&record.data, RecordData::Message(message) if message.thread == self.thread && matches!(message.role.as_str(), "owner" | "assistant"))
            {
                return Err(Error::Invalid(
                    "Writing chat requires unique posted messages in the selected thread".into(),
                ));
            }
        }
        if let Some(setup) = &self.setup {
            let mut references = BTreeSet::new();
            if setup.input.record.is_empty()
                || setup.input.expectation != InputExpectation::Current
                || setup.input.version == 0
                || !ids.insert(&setup.input.record)
                || setup.chat.as_ref().is_some_and(String::is_empty)
                || setup.references.iter().any(|r| {
                    r.version == 0
                        || r.note.is_empty()
                        || r.note == self.note.id
                        || !references.insert(&r.note)
                })
            {
                return Err(Error::Invalid("Invalid writing setup capture".into()));
            }
        }
        Ok(())
    }
    /// Encode the complete validated versioned payload, refusing oversized context.
    pub fn message_text(&self) -> Result<String> {
        self.validate()?;
        let text = serde_json::to_string(&Envelope {
            version: if self.setup.is_some() { 2 } else { 1 },
            snapshot: self.clone(),
        })?;
        if text.len() > MAX_WRITING_REQUEST_BYTES {
            return Err(Error::Invalid("Writing context exceeds 128 KiB. Select fewer comments or a shorter linked conversation before rewriting; nothing was truncated".into()));
        }
        Ok(text)
    }
    /// Decode one original immutable host request, checking identities, types and exact anchors.
    /// Callers also check captured records against canonical saved revisions.
    pub fn from_message(record: &Record) -> Result<Self> {
        let RecordData::Message(message) = &record.data else {
            return Err(Error::Invalid(
                "Writing request is not a saved message".into(),
            ));
        };
        if record.archived
            || record.version != 1
            || message.role != WRITING_REQUEST_ROLE
            || message.text.len() > MAX_WRITING_REQUEST_BYTES
        {
            return Err(Error::Invalid(
                "Writing request is inactive, modified, or too large".into(),
            ));
        }
        let envelope: Envelope = serde_json::from_str(&message.text)?;
        if !matches!(
            (envelope.version, envelope.snapshot.setup.is_some()),
            (1, false) | (2, true)
        ) || envelope.snapshot.request_message != record.id
            || envelope.snapshot.thread != message.thread
        {
            return Err(Error::Invalid(
                "Writing request identity does not match saved context".into(),
            ));
        }
        envelope.snapshot.validate()?;
        Ok(envelope.snapshot)
    }
    /// Verify v2 configuration and reference identities against canonical revisions.
    /// This does not assert current access; hosts recheck it before model exposure.
    pub fn validate_saved_setup(&self, store: &Store) -> Result<()> {
        let Some(capture) = &self.setup else {
            return Ok(());
        };
        let saved = store
            .revision(&capture.input.record, capture.input.version)?
            .ok_or_else(|| Error::Invalid("Writing setup revision is missing".into()))?;
        let RecordData::WritingSetup(setup) = saved.data else {
            return Err(Error::Invalid("Invalid writing setup revision".into()));
        };
        if saved.archived
            || setup.note != self.note.id
            || setup.chat != capture.chat
            || setup.brief != capture.brief
        {
            return Err(Error::Invalid(
                "Writing setup does not match saved evidence".into(),
            ));
        }
        let mut position = 0;
        for reference in &capture.references {
            let Some(offset) = setup.references[position..]
                .iter()
                .position(|r| r.note == reference.note && r.purpose == reference.purpose)
            else {
                return Err(Error::Invalid(
                    "Writing reference does not match saved setup".into(),
                ));
            };
            position += offset + 1;
            if !matches!(
                store.revision(&reference.note, reference.version)?,
                Some(Record {
                    archived: false,
                    data: RecordData::Note(_),
                    ..
                })
            ) {
                return Err(Error::Invalid(
                    "Writing reference revision is missing".into(),
                ));
            }
        }
        Ok(())
    }
    /// Bind every captured revision and the durable review marker as Current inputs.
    pub fn current_inputs(&self) -> Vec<RevisionInput> {
        std::iter::once(&self.note)
            .chain(self.comments.iter())
            .chain(self.messages.iter())
            .map(|record| RevisionInput {
                record: record.id.clone(),
                version: record.version,
                expectation: InputExpectation::Current,
            })
            .chain(std::iter::once(RevisionInput {
                record: self.request_message.clone(),
                version: 1,
                expectation: InputExpectation::Current,
            }))
            .chain(self.setup.iter().map(|setup| setup.input.clone()))
            .collect()
    }
    /// Prepare one whole-note result while preserving metadata and exact captured preconditions.
    pub fn change_request(&self, markdown: &str) -> Result<ChangeRequest> {
        self.validate()?;
        let RecordData::Note(mut note) = self.note.data.clone() else {
            unreachable!()
        };
        note.markdown = markdown.into();
        Ok(ChangeRequest {
            reason: "Whole-note writing proposal for owner review".into(),
            writes: vec![Put {
                id: self.note.id.clone(),
                expected_version: Some(self.note.version),
                archived: false,
                data: RecordData::Note(note),
            }],
            inputs: self.current_inputs(),
        })
    }
}
/// Exact byte mapping only; no quote search or inferred relocation.
pub fn exact_writing_range(
    comment: &Comment,
    version: u64,
    markdown: &str,
) -> Option<(usize, usize)> {
    if comment.unresolved {
        return None;
    }
    let (mapped_version, range) = match (comment.mapped_version, comment.mapped_range) {
        (Some(version), Some(range)) => (version, range),
        (None, None) => (comment.base_version, comment.range?),
        _ => return None,
    };
    (mapped_version == version
        && range.0 < range.1
        && markdown.get(range.0..range.1) == Some(comment.quote.as_str()))
    .then_some(range)
}

pub(crate) fn validate_setup_writes(
    writes: &[Put],
    before: &[Record],
    after: &[Record],
) -> Result<()> {
    for write in writes {
        let RecordData::WritingSetup(setup) = &write.data else {
            continue;
        };
        setup.validate()?;
        let old = before
            .iter()
            .find(|r| r.id == write.id)
            .and_then(|r| match &r.data {
                RecordData::WritingSetup(s) => Some(s),
                _ => None,
            });
        if old.is_some_and(|old| old.note != setup.note || old.chat != setup.chat) {
            return Err(Error::Invalid(
                "Writing setup context is immutable; explicitly create a separate setup".into(),
            ));
        }
        if write.archived {
            continue;
        }
        if writing_setup_scope(after, setup.chat.as_deref())? != setup.chat {
            return Err(Error::Invalid(
                "Writing setup scope must use its canonical Project identity".into(),
            ));
        }
        let usable = |id: &str| {
            after.iter().any(|r| {
                r.id == id
                    && !r.archived
                    && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none())
                    && crate::context::visible(r, after, setup.chat.as_deref())
            })
        };
        if !usable(&setup.note) {
            return Err(Error::Invalid(
                "Working Note is unavailable in this writing scope".into(),
            ));
        }
        for reference in &setup.references {
            // Existing unavailable links remain editable/removable and host-visible;
            // they are never copied into a new request. New links need current access.
            if !usable(&reference.note)
                && !old.is_some_and(|old| old.references.iter().any(|r| r.note == reference.note))
            {
                return Err(Error::Invalid(
                    "Reference Note is unavailable in this writing scope".into(),
                ));
            }
        }
    }
    Ok(())
}
