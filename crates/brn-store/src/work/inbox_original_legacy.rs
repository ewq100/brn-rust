//! Exact owner commands and retained historical certificates for private originals.
//! Workflow qualifies files; these records grant no model or filesystem authority.
use super::{inbox::InboxItem, proposals::SourceVersion};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSourceProof {
    pub source: SourceVersion,
    pub text: String,
}
impl InboxSourceProof {
    pub fn validate(&self) -> Result<()> {
        proposals::validate_path(&self.source.path)?;
        if self.text.len() > super::MAX_NOTE_BYTES
            || self.source.fingerprint.len != self.text.len() as u64
            || self.source.fingerprint.sha256 != hash(self.text.as_bytes())
        {
            return Err(invalid(
                "saved proof needs complete bounded exact text/fingerprint",
            ));
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.validate()?;
        digest(self)
    }
}

pub const MAX_ORIGINAL_OPERATION_BYTES: usize = 64 * 1024 * 1024;
pub const ORIGINAL_OPERATION_PREFIX: &str = "inbox.original-operation.v1.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxRemovalAttestation {
    pub version: u8,
    pub copy_disposable: bool,
    pub meaningful_content_preserved: bool,
    pub consequences_reviewed: bool,
    pub conflicts_acknowledged: bool,
    pub exact_copy_removal_intended: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveInboxOriginalRequest {
    pub operation_id: Uuid,
    pub item_id: Uuid,
    pub preview_digest: [u8; 32],
    /// Explicit causal predecessor after a prior recoverable restoration.
    pub previous_restore: Option<Uuid>,
    pub attestation: InboxRemovalAttestation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxApprovedSource {
    pub operation_id: Uuid,
    pub proposal_id: Uuid,
    pub note_id: Uuid,
    pub saved: InboxSourceProof,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSavedConsequence {
    pub operation_id: Uuid,
    pub member_index: usize,
    pub saved: InboxSourceProof,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum InboxQualifiedOriginal {
    Available { text: String },
}
/// Same canonical fields as the complete preview, narrowed to an available
/// original and no blockers. The digest must equal the owner's captured preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxQualifiedRemovalEvidence {
    pub snapshot: InboxRemovalSnapshot,
    pub original: InboxQualifiedOriginal,
    pub sources: Vec<InboxApprovedSource>,
    pub saved_consequences: Vec<InboxSavedConsequence>,
    pub blockers: [(); 0],
    pub needs_owner_attestation: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxOriginalNamespace {
    pub data_device: u64,
    pub data_inode: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxOriginalRemovalRecord {
    pub request: RemoveInboxOriginalRequest,
    pub evidence: InboxQualifiedRemovalEvidence,
    pub namespace: InboxOriginalNamespace,
    pub prepared_at_ms: u64,
    pub removed_at_ms: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreInboxOriginalRequest {
    pub operation_id: Uuid,
    pub removal_operation_id: Uuid,
    pub removal_digest: [u8; 32],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxOriginalRestoreRecord {
    pub request: RestoreInboxOriginalRequest,
    pub original: InboxItem,
    pub namespace: InboxOriginalNamespace,
    pub prepared_at_ms: u64,
    pub restored_at_ms: Option<u64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxOriginalOperationKind {
    Remove,
    Restore,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxOriginalOperationSummary {
    pub kind: InboxOriginalOperationKind,
    pub operation_id: Uuid,
    pub item_id: Uuid,
    pub parent: Option<Uuid>,
    pub prepared_at_ms: u64,
    pub settled_at_ms: Option<u64>,
    pub record_sha256: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivedInboxAnalysis {
    pub removal_operation_id: Uuid,
    pub analysis: super::inbox_review::InboxAnalysisReview,
}

use super::{
    inbox_review, inbox_source,
    proposal_apply::{ApplyJournal, ApplyOutcome},
    proposals::{self, NoteChange},
};
use crate::{Error, Result, hash, invalid};
use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
};

const CORE_BYTES: usize = MAX_ORIGINAL_OPERATION_BYTES - 1024;

struct Bounded(Vec<u8>, usize);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > self.1)
        {
            return Err(io::Error::other(
                "complete original operation exceeds its bound",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    encode_bounded(value, CORE_BYTES)
}
pub(crate) fn encode_bounded<T: Serialize>(value: &T, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Bounded(Vec::new(), limit);
    serde_json::to_writer(&mut bytes, value)
        .map_err(|_| invalid("complete original operation exceeds its encoded bound"))?;
    Ok(bytes.0)
}
pub(crate) fn digest<T: Serialize>(value: &T) -> Result<[u8; 32]> {
    Ok(hash(&encode(value)?))
}
fn conflict() -> Error {
    Error::OperationConflict(
        "original operation UUID or causal certificate has another exact body".into(),
    )
}
fn nonnil(id: Uuid) -> Result<()> {
    proposals::nonnil(id)
}
fn time(prepared: u64, settled: Option<u64>) -> Result<()> {
    if prepared > i64::MAX as u64 || settled.is_some_and(|t| t < prepared || t > i64::MAX as u64) {
        return Err(invalid(
            "original operation timestamps are not monotonic bounded observations",
        ));
    }
    Ok(())
}
impl InboxRemovalAttestation {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1
            || !self.copy_disposable
            || !self.meaningful_content_preserved
            || !self.consequences_reviewed
            || !self.conflicts_acknowledged
            || !self.exact_copy_removal_intended
        {
            return Err(invalid(
                "original removal requires all five explicit owner attestations at version 1",
            ));
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.validate()?;
        digest(self)
    }
}
impl RemoveInboxOriginalRequest {
    pub fn validate(&self) -> Result<()> {
        nonnil(self.operation_id)?;
        nonnil(self.item_id)?;
        if let Some(id) = self.previous_restore {
            nonnil(id)?;
            if id == self.operation_id {
                return Err(conflict());
            }
        }
        self.attestation.validate()
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.validate()?;
        digest(self)
    }
}
impl RestoreInboxOriginalRequest {
    pub fn validate(&self) -> Result<()> {
        nonnil(self.operation_id)?;
        nonnil(self.removal_operation_id)?;
        if self.operation_id == self.removal_operation_id {
            return Err(conflict());
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.validate()?;
        digest(self)
    }
}
impl InboxOriginalNamespace {
    pub fn validate(&self) -> Result<()> {
        if self.data_inode == 0 {
            return Err(invalid(
                "original operation needs a qualified data-directory namespace",
            ));
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.validate()?;
        digest(self)
    }
}

fn unique<T>(values: &[T], id: impl Fn(&T) -> Uuid) -> Result<()> {
    let mut ids = HashSet::new();
    for value in values {
        let id = id(value);
        nonnil(id)?;
        if !ids.insert(id) {
            return Err(invalid("duplicate original certificate identity"));
        }
    }
    Ok(())
}
fn validate_analysis(
    analysis: &inbox_review::InboxAnalysisReview,
    original: &InboxItem,
) -> Result<()> {
    analysis.job.validate()?;
    let provenance = inbox_source::read_provenance(&analysis.job.capture.source_text)?
        .ok_or_else(|| invalid("archived analysis lost original provenance"))?;
    let c = &original.capture;
    if provenance.item_id != c.id
        || provenance.kind != c.kind
        || provenance.title != c.title
        || provenance.original_name != c.original_name
        || provenance.received_at_ms != original.received_at_ms
        || provenance.original_byte_len != c.copy.byte_len
        || provenance.original_sha256 != c.copy.sha256
    {
        return Err(invalid("archived analysis differs from the exact original"));
    }
    if let Some(turn) = &analysis.turn {
        let job = &analysis.job;
        let capture = &job.capture;
        super::chat::validate_selection(&turn.provider, &turn.model)?;
        super::chat::validate_error(turn.error_code.as_deref())?;
        nonnil(turn.conversation_id)?;
        if turn.id != capture.id
            || turn.question != job.question
            || turn.provider != capture.provider
            || turn.model != capture.model
            || turn.effort.as_deref() != Some(capture.effort.as_str())
            || capture
                .conversation
                .is_some_and(|id| id != turn.conversation_id)
            || turn.started_at_ms.is_some_and(|t| t > i64::MAX as u64)
            || turn.finished_at_ms.is_some_and(|t| t > i64::MAX as u64)
            || turn
                .started_at_ms
                .zip(turn.finished_at_ms)
                .is_some_and(|(a, b)| b < a)
            || turn.status == super::chat::WorkTurnStatus::Running && turn.finished_at_ms.is_some()
        {
            return Err(invalid(
                "archived analysis differs from its exact historical turn",
            ));
        }
    }
    Ok(())
}
fn journals(snapshot: &InboxRemovalSnapshot) -> Result<HashMap<Uuid, &ApplyJournal>> {
    let mut journals = HashMap::new();
    unique(&snapshot.review.approvals, |a| a.request.operation_id)?;
    unique(&snapshot.lineage, |a| a.request.operation_id)?;
    for journal in snapshot.review.approvals.iter().chain(&snapshot.lineage) {
        journal.validate()?;
        if let Some(known) = journals.insert(journal.request.operation_id, journal)
            && known != journal
        {
            return Err(invalid("certificate approval views disagree"));
        }
    }
    Ok(journals)
}
fn validate_snapshot(snapshot: &InboxRemovalSnapshot) -> Result<()> {
    let review = &snapshot.review;
    review.original.validate()?;
    unique(&review.processing, |b| b.request.id)?;
    for batch in &review.processing {
        batch.validate()?;
        if !batch
            .request
            .items
            .iter()
            .any(|item| item == &review.original)
            || batch
                .request
                .items
                .iter()
                .any(|i| i.capture.id == review.original.capture.id && i != &review.original)
        {
            return Err(invalid(
                "certificate processing differs from its original capture",
            ));
        }
    }
    unique(&review.analyses, |a| a.job.capture.id)?;
    for analysis in &review.analyses {
        validate_analysis(analysis, &review.original)?;
    }
    unique(&review.proposals, |p| p.record.draft.id)?;
    unique(&snapshot.related_reviews, |p| p.record.draft.id)?;
    let mut original_source_ids = HashSet::new();
    for p in review.proposals.iter().chain(&snapshot.related_reviews) {
        if let Some(binding) = &p.record.draft.inbox_source
            && binding.original.capture.id == review.original.capture.id
        {
            original_source_ids.insert(binding.note_id);
        }
    }
    for analysis in &review.analyses {
        original_source_ids.insert(analysis.job.capture.note_id()?);
    }
    let mut proposals_by_id = HashMap::new();
    for p in review.proposals.iter().chain(&snapshot.related_reviews) {
        proposals::validate_record(&p.record)?;
        if p.record.version == 1 && p.creation_sha256 != digest(&p.record.draft)? {
            return Err(invalid("certificate initial review creation hash differs"));
        }
        if proposals_by_id.insert(p.record.draft.id, p).is_some() {
            return Err(invalid("certificate repeats a review"));
        }
        if let Some(binding) = &p.record.draft.inbox_source
            && binding.original.capture.id == review.original.capture.id
            && binding.original != review.original
        {
            return Err(invalid("certificate Source has another original"));
        }
        if let Some(binding) = &p.record.draft.inbox_knowledge {
            let job = review
                .analyses
                .iter()
                .find(|a| a.job.capture.id == binding.analysis_id);
            // Action-family closure may retain unrelated original reviews.
            // Their bounded pure certificate is valid without importing their jobs.
            let Some(job) = job else {
                if original_source_ids.contains(&binding.citations[0].note_id) {
                    return Err(invalid("original-bound knowledge lost its exact analysis"));
                }
                continue;
            };
            binding.validate_capture(&job.job)?;
        }
    }
    for journal in journals(snapshot)?.values() {
        if journal.receipt.as_ref().map(|r| r.outcome) == Some(ApplyOutcome::Applied) {
            for expected in &journal.action_records {
                if !snapshot.actions.iter().any(|retained| {
                    retained.origin == expected.origin
                        && retained.version >= expected.version
                        && (retained.version != expected.version || retained == expected)
                }) {
                    return Err(invalid("certificate lost Applied Action authority"));
                }
            }
        }
        let p = proposals_by_id
            .get(&journal.approved.draft.id)
            .ok_or_else(|| invalid("certificate approval has no retained review"))?;
        if p.creation_sha256 != journal.creation_sha256 {
            return Err(invalid("certificate approval creation binding differs"));
        }
    }
    unique(&snapshot.actions, |a| a.origin.id)?;
    for action in &snapshot.actions {
        action.validate()?;
    }
    unique(&snapshot.completions, |c| c.request.operation_id)?;
    for completion in &snapshot.completions {
        completion.validate()?;
        if !snapshot.actions.iter().any(|a| {
            a.origin == completion.after.origin
                && a.data.state == super::actions::ActionState::Completed
                && a.version >= completion.after.version
                && (a.version != completion.after.version || a == &completion.after)
        }) {
            return Err(invalid(
                "certificate completion has no exact current Action",
            ));
        }
    }
    unique(&snapshot.rewrites, |r| r.spec.id)?;
    for action in &snapshot.actions {
        if action.data.state == super::actions::ActionState::Completed
            && !snapshot
                .completions
                .iter()
                .any(|c| c.after.origin.id == action.origin.id)
        {
            return Err(invalid("certificate Completed Action lost its completion"));
        }
    }
    for rewrite in &snapshot.rewrites {
        rewrite.validate()?;
    }
    unique(&review.findings, |f| f.draft.request.id)?;
    for finding in &review.findings {
        super::findings::validate_record(finding)?;
        if let super::findings::FindingOrigin::InboxConflict { analysis_id, .. } =
            finding.draft.request.origin
        {
            if let Some(analysis) = review
                .analyses
                .iter()
                .find(|a| a.job.capture.id == analysis_id)
            {
                super::findings::validate_inbox_certificate_capture(
                    &finding.draft,
                    &analysis.job.capture,
                )?;
            } else if finding.draft.evidence[0]
                .note_id
                .is_some_and(|id| original_source_ids.contains(&id))
            {
                return Err(invalid("original-bound conflict lost its exact analysis"));
            }
        }
    }
    for summary in &snapshot.original_operations {
        summary.validate()?;
    }
    if ordered_history(snapshot.original_operations.clone())? != snapshot.original_operations {
        return Err(invalid(
            "certificate predecessor history is not in causal order",
        ));
    }
    snapshot.digest()?;
    Ok(())
}
fn saved_member<'a>(
    journal: &'a ApplyJournal,
    index: usize,
    saved: &InboxSourceProof,
) -> Result<&'a NoteChange> {
    saved.validate()?;
    if journal.undo.is_some()
        || journal.receipt.as_ref().map(|r| r.outcome) != Some(ApplyOutcome::Applied)
    {
        return Err(invalid(
            "saved certificate member needs a terminal non-Undo Applied approval",
        ));
    }
    let change = journal
        .approved
        .draft
        .changes
        .get(index)
        .ok_or_else(|| invalid("saved member index is invalid"))?;
    let observed = journal
        .observations
        .as_ref()
        .and_then(|p| p.get(index))
        .and_then(|p| p.destination.as_ref());
    if change.path() != saved.source.path
        || change.text() != Some(saved.text.as_str())
        || observed != Some(&saved.source.fingerprint)
    {
        return Err(invalid(
            "saved certificate member differs from exact approved path/text/fingerprint",
        ));
    }
    Ok(change)
}
impl InboxQualifiedRemovalEvidence {
    pub fn validate(&self) -> Result<()> {
        if self.snapshot.review.original.capture.kind == super::inbox::InboxKind::Binary {
            return Err(invalid("Binary Inbox original removal is not supported"));
        }
        validate_snapshot(&self.snapshot)?;
        let InboxQualifiedOriginal::Available { text } = &self.original;
        let original = &self.snapshot.review.original;
        if !self.needs_owner_attestation
            || self.sources.is_empty()
            || text.len() as u64 != original.capture.copy.byte_len
            || hash(text.as_bytes()) != original.capture.copy.sha256
        {
            return Err(invalid(
                "qualified removal needs exact complete available original and approved Sources",
            ));
        }
        let journals = journals(&self.snapshot)?;
        let mut note_ids = HashSet::new();
        let mut member_ids = HashSet::new();
        for source in &self.sources {
            nonnil(source.note_id)?;
            let journal = journals
                .get(&source.operation_id)
                .ok_or_else(|| invalid("approved Source has no exact journal"))?;
            let binding = journal
                .approved
                .draft
                .inbox_source
                .as_ref()
                .ok_or_else(|| invalid("approved Source lacks original binding"))?;
            if journal.approved.draft.id != source.proposal_id
                || binding.original != *original
                || binding.note_id != source.note_id
                || !note_ids.insert(source.note_id)
                || !member_ids.insert((source.operation_id, 0))
            {
                return Err(invalid(
                    "approved Source certificate identity differs or repeats",
                ));
            }
            let batch = self
                .snapshot
                .review
                .processing
                .iter()
                .find(|b| b.request.id == binding.batch_id)
                .ok_or_else(|| invalid("approved Source lost its exact conversion attempt"))?;
            if batch.request.items.get(binding.index) != Some(original)
                || batch.entries.get(binding.index).map(|e| &e.outcome)
                    != Some(&super::inbox_processing::InboxProcessOutcome::Converted {
                        format: binding.format,
                        byte_len: binding.byte_len,
                        sha256: binding.sha256,
                    })
            {
                return Err(invalid(
                    "approved Source differs from its exact successful conversion",
                ));
            }
            saved_member(journal, 0, &source.saved)?;
            binding.validate_markdown(&source.saved.text)?;
        }
        for saved in &self.saved_consequences {
            let journal = journals
                .get(&saved.operation_id)
                .ok_or_else(|| invalid("saved consequence has no exact journal"))?;
            if journal
                .approved
                .draft
                .inbox_source
                .as_ref()
                .is_some_and(|b| b.original.capture.id == original.capture.id)
            {
                return Err(invalid(
                    "exact original Source must remain in the approved Sources proof list",
                ));
            }
            saved_member(journal, saved.member_index, &saved.saved)?;
            if !member_ids.insert((saved.operation_id, saved.member_index)) {
                return Err(invalid("saved consequence repeats a member"));
            }
            if let Some(id) = crate::note_identity::read(&saved.saved.text)?
                && !note_ids.insert(id)
            {
                return Err(invalid("saved certificate has duplicate note identities"));
            }
        }
        for journal in journals.values() {
            if journal.undo.is_none()
                && journal.receipt.as_ref().map(|r| r.outcome) == Some(ApplyOutcome::Applied)
            {
                for (index, change) in journal.approved.draft.changes.iter().enumerate() {
                    if change.text().is_none()
                        || !member_ids.contains(&(journal.request.operation_id, index))
                    {
                        return Err(invalid(
                            "qualified certificate omits an Applied note member or retains Trash",
                        ));
                    }
                }
            }
        }
        encode(self)?;
        Ok(())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.validate()?;
        digest(self)
    }
}
impl InboxOriginalRemovalRecord {
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        self.evidence.validate()?;
        self.namespace.validate()?;
        time(self.prepared_at_ms, self.removed_at_ms)?;
        if self.request.item_id != self.evidence.snapshot.review.original.capture.id
            || self.request.preview_digest != digest(&self.evidence)?
        {
            return Err(invalid(
                "removal record differs from exact owner preview identity",
            ));
        }
        let history = &self.evidence.snapshot.original_operations;
        if history.last().map(|s| s.operation_id) != self.request.previous_restore
            || history.last().is_some_and(|s| {
                s.kind != InboxOriginalOperationKind::Restore || s.settled_at_ms.is_none()
            })
            || history.iter().any(|s| {
                s.item_id != self.request.item_id || s.operation_id == self.request.operation_id
            })
        {
            return Err(invalid(
                "removal certificate does not retain its exact prior settled Restore chain",
            ));
        }
        encode(self)?;
        Ok(())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.validate()?;
        digest(self)
    }
}
impl InboxOriginalRestoreRecord {
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        self.original.validate()?;
        if self.original.capture.kind == super::inbox::InboxKind::Binary {
            return Err(invalid("Binary Inbox original restore is not supported"));
        }
        self.namespace.validate()?;
        time(self.prepared_at_ms, self.restored_at_ms)?;
        encode(self)?;
        Ok(())
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.validate()?;
        digest(self)
    }
}
impl InboxOriginalOperationSummary {
    pub fn validate(&self) -> Result<()> {
        nonnil(self.operation_id)?;
        nonnil(self.item_id)?;
        if let Some(parent) = self.parent {
            nonnil(parent)?;
            if parent == self.operation_id {
                return Err(conflict());
            }
        }
        if self.kind == InboxOriginalOperationKind::Restore && self.parent.is_none() {
            return Err(invalid("Restore summary has no causal removal"));
        }
        time(self.prepared_at_ms, self.settled_at_ms)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "record",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Operation {
    Remove(Box<InboxOriginalRemovalRecord>),
    Restore(Box<InboxOriginalRestoreRecord>),
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Envelope {
    pub(crate) operation: Operation,
    pub(crate) sha256: [u8; 32],
}
impl Operation {
    pub(crate) fn digest(&self) -> Result<[u8; 32]> {
        Ok(hash(&encode_bounded(self, MAX_ORIGINAL_OPERATION_BYTES)?))
    }
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Remove(r) => r.validate(),
            Self::Restore(r) => r.validate(),
        }
    }
    pub(crate) fn summary(&self) -> Result<InboxOriginalOperationSummary> {
        Ok(match self {
            Self::Remove(r) => InboxOriginalOperationSummary {
                kind: InboxOriginalOperationKind::Remove,
                operation_id: r.request.operation_id,
                item_id: r.request.item_id,
                parent: r.request.previous_restore,
                prepared_at_ms: r.prepared_at_ms,
                settled_at_ms: r.removed_at_ms,
                record_sha256: digest(r)?,
            },
            Self::Restore(r) => InboxOriginalOperationSummary {
                kind: InboxOriginalOperationKind::Restore,
                operation_id: r.request.operation_id,
                item_id: r.original.capture.id,
                parent: Some(r.request.removal_operation_id),
                prepared_at_ms: r.prepared_at_ms,
                settled_at_ms: r.restored_at_ms,
                record_sha256: digest(r)?,
            },
        })
    }
    pub fn original(&self) -> &InboxItem {
        match self {
            Self::Remove(r) => &r.evidence.snapshot.review.original,
            Self::Restore(r) => &r.original,
        }
    }
    pub fn namespace(&self) -> &InboxOriginalNamespace {
        match self {
            Self::Remove(r) => &r.namespace,
            Self::Restore(r) => &r.namespace,
        }
    }
}
pub(crate) fn ordered_history(
    values: Vec<InboxOriginalOperationSummary>,
) -> Result<Vec<InboxOriginalOperationSummary>> {
    if values.is_empty() {
        return Ok(Vec::new());
    }
    let mut by_id = HashMap::new();
    let mut children = HashMap::new();
    let mut root = None;
    for value in values {
        value.validate()?;
        if let Some(parent) = value.parent {
            if children.insert(parent, value.operation_id).is_some() {
                return Err(invalid("original-operation history forks"));
            }
        } else if root.replace(value.operation_id).is_some() {
            return Err(invalid("original-operation history has multiple roots"));
        }
        if by_id.insert(value.operation_id, value).is_some() {
            return Err(invalid("original-operation history repeats an identity"));
        }
    }
    let mut next = root.ok_or_else(|| invalid("original-operation history has no acyclic root"))?;
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(next) {
            return Err(invalid("original-operation history is cyclic"));
        }
        let entry = by_id
            .get(&next)
            .ok_or_else(|| invalid("original-operation history has an unknown parent"))?;
        if let Some(previous) = result.last() {
            let previous: &InboxOriginalOperationSummary = previous;
            if previous.kind == entry.kind
                || previous.settled_at_ms.is_none()
                || previous.item_id != entry.item_id
                || entry.prepared_at_ms < previous.settled_at_ms.unwrap()
            {
                return Err(invalid(
                    "original-operation causal parent is not exact settled opposite-kind authority",
                ));
            }
        } else if entry.kind != InboxOriginalOperationKind::Remove {
            return Err(invalid("original-operation root must be Remove"));
        }
        result.push(entry.clone());
        match children.get(&next) {
            Some(child) => next = *child,
            None => break,
        }
    }
    if result.len() != by_id.len() {
        return Err(invalid(
            "original-operation history has detached or unknown parents",
        ));
    }
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxRemovalSnapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub original_operations: Vec<InboxOriginalOperationSummary>,
    pub review: super::inbox_review::InboxReviewManifest,
    pub related_reviews: Vec<super::inbox_review::InboxProposalReview>,
    pub actions: Vec<super::actions::ActionRecord>,
    pub completions: Vec<super::action_completion::ActionCompletion>,
    pub rewrites: Vec<super::proposal_rewrite::RewriteJob>,
    pub lineage: Vec<ApplyJournal>,
}
impl InboxRemovalSnapshot {
    pub fn digest(&self) -> Result<[u8; 32]> {
        inbox_review::digest(self)
    }
}
