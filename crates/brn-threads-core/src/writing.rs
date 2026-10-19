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
    #[serde(default, skip_serializing_if = "WritingReadRequirements::is_default")]
    pub requirements: WritingReadRequirements,
    pub chat: Option<String>,
    pub input: RevisionInput,
    pub brief: Option<String>,
    pub references: Vec<WritingReferenceVersion>,
}
/// Owner-selected freshness requirements, captured only in v2 requests.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WritingReadRequirements {
    pub exhaustive: bool,
    pub must_be_current: Vec<String>,
}
impl WritingReadRequirements {
    pub fn is_default(&self) -> bool {
        !self.exhaustive && self.must_be_current.is_empty()
    }
}
pub const WRITING_CONTEXT_ROLE: &str = "writing-context";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WritingContextUse {
    pub note: String,
    pub chat: Option<String>,
    pub thread: String,
}
/// Host-created provenance travels with the immutable candidate and its receipt.
pub const WRITING_PUBLIC_REASON: &str = "Whole-note writing proposal for owner review";
pub const WRITING_EVIDENCE_ROLE: &str = "writing-evidence";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WritingRead {
    pub id: String,
    pub version: u64,
    pub covered_retained_bytes: usize,
    pub total_retained_bytes: usize,
    pub complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WritingQuotation {
    pub id: String,
    pub version: u64,
    pub start: usize,
    pub end: usize,
    pub quote: String,
}
/// Claims are model reports, never an assertion that requirements were verified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WritingRequirementsClaim {
    pub claim: String,
    pub quotations: Vec<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WritingCandidateEvidence {
    pub version: u8,
    pub request_message: String,
    pub thread: String,
    pub chat: Option<String>,
    pub reads: Vec<WritingRead>,
    pub quotations: Vec<WritingQuotation>,
    pub requirements_reported: Vec<WritingRequirementsClaim>,
    /// Untrusted model suggestions, never comment-resolution authority.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proposed_addressed_comment_ids: Vec<String>,
    #[serde(default)]
    pub explanation: String,
}
/// Resolve existing host scope for reads, including archived/legacy Resolved
/// conversations. Setup never creates privacy or changes that scope.
pub fn writing_setup_scope(rows: &[Record], chat: Option<&str>) -> Result<Option<String>> {
    let Some(chat) = chat else {
        return Ok(None);
    };
    if let Some(project) = crate::context::project_scope(rows, Some(chat)) {
        return Ok(Some(project.id.clone()));
    }
    if rows
        .iter()
        .any(|r| r.id == chat && matches!(r.data, RecordData::Thread(_)))
    {
        return Ok(crate::context::chat_private(rows, chat).then(|| chat.into()));
    }
    Err(Error::Invalid("Writing scope is missing".into()))
}
/// Scope of the selected existing conversation; ordinary chats remain general.
pub fn writing_request_scope(rows: &[Record], chat: &str) -> Result<Option<String>> {
    writing_setup_scope(rows, Some(chat))
}
fn require_active_writing_scope(rows: &[Record], chat: Option<&str>) -> Result<()> {
    if let Some(chat) = chat {
        let active = rows.iter().any(|r| {
            r.id == chat
                && !r.archived
                && match &r.data {
                    RecordData::Thread(t) => t.state == ThreadState::Open,
                    RecordData::Project(_) => true,
                    _ => false,
                }
        }) && !crate::context::project_scope(rows, Some(chat))
            .is_some_and(|p| p.archived);
        if !active {
            return Err(Error::Invalid("Writing scope is inactive".into()));
        }
    }
    Ok(())
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
    matches!(&record.data, RecordData::Message(m) if matches!(m.role.as_str(), WRITING_EVIDENCE_ROLE | WRITING_CONTEXT_ROLE) || m.role == WRITING_REQUEST_ROLE
        && serde_json::from_str::<serde_json::Value>(&m.text).ok().is_some_and(|v| v["version"] == 2))
}
impl WritingSnapshot {
    /// Only the immutable captured subset is eligible for a model suggestion.
    pub fn validate_proposed_addressed_comment_ids(&self, ids: &[String]) -> Result<()> {
        let mut seen = BTreeSet::new();
        if ids.len() > 128
            || ids.iter().any(|id| {
                !seen.insert(id) || !self.comments.iter().any(|comment| comment.id == *id)
            })
        {
            return Err(Error::Invalid("Propose at most 128 unique IDs from the saved request's captured comments, or omit the suggestions".into()));
        }
        Ok(())
    }

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
                || setup
                    .requirements
                    .must_be_current
                    .iter()
                    .any(|id| !setup.references.iter().any(|r| r.note == *id))
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
    request: &ChangeRequest,
    before: &[Record],
    after: &[Record],
    host_chat: Option<&str>,
) -> Result<()> {
    if !request
        .writes
        .iter()
        .any(|w| matches!(w.data, RecordData::WritingSetup(_)))
    {
        return Ok(());
    }
    // The host binds the actual conversation, not an arbitrary linked chat.
    // Persisted checked Thread inputs also retain that context over prepare
    // replay and Undo; multiple inputs require an explicit host binding.
    let chats: Vec<_> = request
        .inputs
        .iter()
        .filter(|i| {
            matches!(
                i.expectation,
                InputExpectation::Current | InputExpectation::ChatContext
            )
        })
        .filter_map(|i| {
            before
                .iter()
                .find(|r| r.id == i.record && matches!(r.data, RecordData::Thread(_)))
                .map(|r| r.id.as_str())
        })
        .collect();
    let chat = host_chat.or_else(|| (chats.len() == 1).then(|| chats[0]));
    for write in &request.writes {
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
        require_active_writing_scope(after, setup.chat.as_deref())?;
        require_active_writing_scope(after, chat)?;
        if chat.is_some() && writing_setup_scope(after, chat)? != setup.chat {
            return Err(Error::Invalid(
                "Writing setup and bound conversation scopes disagree".into(),
            ));
        }
        let usable = |id: &str, context: Option<&str>| {
            after.iter().any(|r| {
                r.id == id
                    && !r.archived
                    && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none())
                    && crate::context::visible(r, after, context)
            })
        };
        if !usable(&setup.note, chat.or(setup.chat.as_deref())) {
            return Err(Error::Invalid(
                "Working Note is unavailable in this writing scope".into(),
            ));
        }
        for reference in &setup.references {
            // Existing unavailable links remain editable/removable and host-visible;
            // they are never copied into a new request. New links need current access.
            if !usable(&reference.note, setup.chat.as_deref())
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

/// Validate the host representation against the exact request/revisions before
/// it becomes immutable. Actual read/quote admission remains the runtime's job.
pub(crate) fn validate_candidate_evidence(
    request: &ChangeRequest,
    mut revision: impl FnMut(&str, u64) -> Result<Option<Record>>,
) -> Result<()> {
    let mut count = 0;
    for write in &request.writes {
        let RecordData::Message(message) = &write.data else {
            continue;
        };
        if message.role != WRITING_EVIDENCE_ROLE || write.archived {
            continue;
        }
        count += 1;
        let evidence: WritingCandidateEvidence = serde_json::from_str(&message.text)?;
        if request.reason != WRITING_PUBLIC_REASON {
            return Err(Error::Invalid(
                "Private writing explanation belongs only in scoped evidence".into(),
            ));
        }
        let invalid = || {
            Error::Invalid("Writing evidence must match the exact saved request, read revisions and quotations".into())
        };
        if count > 1
            || evidence.version != 1
            || evidence.thread != message.thread
            || message.text.len() > MAX_WRITING_REQUEST_BYTES
        {
            return Err(invalid());
        }
        let marker = request
            .inputs
            .iter()
            .find(|i| {
                i.record == evidence.request_message && i.expectation == InputExpectation::Current
            })
            .ok_or_else(invalid)?;
        let saved = revision(&marker.record, marker.version)?.ok_or_else(invalid)?;
        let snapshot = WritingSnapshot::from_message(&saved)?;
        let setup = snapshot.setup.as_ref().ok_or_else(invalid)?;
        snapshot
            .validate_proposed_addressed_comment_ids(&evidence.proposed_addressed_comment_ids)?;
        if snapshot.thread != evidence.thread
            || setup.chat != evidence.chat
            || !snapshot
                .current_inputs()
                .iter()
                .all(|i| request.inputs.contains(i))
            || !request.writes.iter().any(|w| {
                w.id == snapshot.note.id
                    && w.expected_version == Some(snapshot.note.version)
                    && matches!(w.data, RecordData::Note(_))
            })
        {
            return Err(invalid());
        }
        let mut keys = BTreeSet::new();
        for read in &evidence.reads {
            if !keys.insert((&read.id, read.version))
                || read.covered_retained_bytes > read.total_retained_bytes
                || read.complete != (read.covered_retained_bytes == read.total_retained_bytes)
            {
                return Err(invalid());
            }
            let Some(Record {
                archived: false,
                data: RecordData::Note(note),
                ..
            }) = revision(&read.id, read.version)?
            else {
                return Err(invalid());
            };
            if note.markdown.len() != read.total_retained_bytes
                || !request
                    .inputs
                    .iter()
                    .any(|i| i.record == read.id && i.version == read.version)
            {
                return Err(invalid());
            }
        }
        for quote in &evidence.quotations {
            let Some(Record {
                archived: false,
                data: RecordData::Note(note),
                ..
            }) = revision(&quote.id, quote.version)?
            else {
                return Err(invalid());
            };
            if quote.start >= quote.end
                || note.markdown.get(quote.start..quote.end) != Some(quote.quote.as_str())
                || !evidence
                    .reads
                    .iter()
                    .any(|r| r.id == quote.id && r.version == quote.version)
            {
                return Err(invalid());
            }
        }
        if evidence.requirements_reported.iter().any(|claim| {
            claim
                .quotations
                .iter()
                .any(|i| *i >= evidence.quotations.len())
        }) {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Owner replacement may drop only suggestions the model made for the old text.
/// The original immutable Message remains untouched.
pub(crate) fn evidence_without_suggestions(data: &RecordData) -> Option<RecordData> {
    let RecordData::Message(message) = data else {
        return None;
    };
    if message.role != WRITING_EVIDENCE_ROLE {
        return None;
    }
    let mut evidence: WritingCandidateEvidence = serde_json::from_str(&message.text).ok()?;
    if evidence.proposed_addressed_comment_ids.is_empty() {
        return None;
    }
    evidence.proposed_addressed_comment_ids.clear();
    let mut cleared = message.clone();
    cleared.text = serde_json::to_string(&evidence).ok()?;
    Some(RecordData::Message(cleared))
}
