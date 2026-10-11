//! Host-owned, exact writing context. Stored inside existing Message records.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Host-owned Message role that durably binds proposal-only writing work.
pub const WRITING_REQUEST_ROLE: &str = "writing-request";
/// Complete encoded context bound; overflow must never truncate evidence.
pub const MAX_WRITING_REQUEST_BYTES: usize = 128 * 1024;
/// Canonical saved-record snapshots for one explicitly selected note rewrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WritingSnapshot {
    pub request_message: String,
    pub thread: String,
    pub note: Record,
    pub comments: Vec<Record>,
    pub messages: Vec<Record>,
}
#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u8,
    snapshot: WritingSnapshot,
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
        Ok(())
    }
    /// Encode the complete validated versioned payload, refusing oversized context.
    pub fn message_text(&self) -> Result<String> {
        self.validate()?;
        let text = serde_json::to_string(&Envelope {
            version: 1,
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
        if envelope.version != 1
            || envelope.snapshot.request_message != record.id
            || envelope.snapshot.thread != message.thread
        {
            return Err(Error::Invalid(
                "Writing request identity does not match saved context".into(),
            ));
        }
        envelope.snapshot.validate()?;
        Ok(envelope.snapshot)
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
