//! Exact owner commands and retained historical certificates for private originals.
//! Workflow qualifies files; these records grant no model or filesystem authority.
use super::{inbox::InboxItem, inbox_removal::InboxRemovalSnapshot, proposals::SourceVersion};
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
    WorkStore, inbox, inbox_actions, inbox_review, inbox_source, now_ms,
    proposal_apply::{ApplyJournal, ApplyOutcome},
    proposals::{self, NoteChange},
};
use crate::{Error, Result, hash, invalid};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
};

const MAX_ROWS: usize = 16_384;
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
fn encode_bounded<T: Serialize>(value: &T, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Bounded(Vec::new(), limit);
    serde_json::to_writer(&mut bytes, value)
        .map_err(|_| invalid("complete original operation exceeds its encoded bound"))?;
    Ok(bytes.0)
}
fn digest<T: Serialize>(value: &T) -> Result<[u8; 32]> {
    Ok(hash(&encode(value)?))
}
fn equal<T: Serialize>(a: &T, b: &T) -> Result<bool> {
    Ok(encode(a)? == encode(b)?)
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
            if job.job.capture.source != binding.source
                || job.job.capture.note_id()? != binding.citations[0].note_id
            {
                return Err(invalid(
                    "certificate knowledge differs from its analysis capture",
                ));
            }
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
enum Operation {
    Remove(Box<InboxOriginalRemovalRecord>),
    Restore(Box<InboxOriginalRestoreRecord>),
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    operation: Operation,
    sha256: [u8; 32],
}
impl Operation {
    fn digest(&self) -> Result<[u8; 32]> {
        Ok(hash(&encode_bounded(self, MAX_ORIGINAL_OPERATION_BYTES)?))
    }
    fn validate(&self) -> Result<()> {
        match self {
            Self::Remove(r) => r.validate(),
            Self::Restore(r) => r.validate(),
        }
    }
    fn summary(&self) -> Result<InboxOriginalOperationSummary> {
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
    fn original(&self) -> &InboxItem {
        match self {
            Self::Remove(r) => &r.evidence.snapshot.review.original,
            Self::Restore(r) => &r.original,
        }
    }
    fn namespace(&self) -> &InboxOriginalNamespace {
        match self {
            Self::Remove(r) => &r.namespace,
            Self::Restore(r) => &r.namespace,
        }
    }
}
fn key(id: Uuid) -> String {
    format!("{ORIGINAL_OPERATION_PREFIX}{id}")
}
pub(super) fn guard_setting(key: &str) -> Result<()> {
    if key.starts_with(ORIGINAL_OPERATION_PREFIX) {
        return Err(invalid(
            "original-operation settings are owned typed records",
        ));
    }
    Ok(())
}
fn ids(conn: &Connection) -> Result<Vec<Uuid>> {
    let mut ids = Vec::new();
    for raw in conn
        .prepare("SELECT key FROM settings WHERE substr(key,1,?1)=?2 ORDER BY key")?
        .query_map(
            params![
                ORIGINAL_OPERATION_PREFIX.len() as i64,
                ORIGINAL_OPERATION_PREFIX
            ],
            |r| r.get::<_, String>(0),
        )?
    {
        let raw = raw?;
        let suffix = raw
            .strip_prefix(ORIGINAL_OPERATION_PREFIX)
            .ok_or_else(|| invalid("invalid operation key"))?;
        let id = Uuid::parse_str(suffix)
            .map_err(|_| invalid("invalid owned original-operation UUID"))?;
        nonnil(id)?;
        if raw != key(id) || ids.len() >= MAX_ROWS {
            return Err(invalid(
                "owned original-operation identities or count are invalid",
            ));
        }
        ids.push(id);
    }
    Ok(ids)
}
fn read(conn: &Connection, id: Uuid) -> Result<Option<Operation>> {
    nonnil(id)?;
    let value: Option<Option<String>> = conn.query_row(
        "SELECT CASE WHEN length(CAST(value AS BLOB))<=?2 THEN value END FROM settings WHERE key=?1",
        params![key(id),MAX_ORIGINAL_OPERATION_BYTES as i64], |r| r.get(0)).optional()?;
    let Some(value) = value else {
        return Ok(None);
    };
    let value =
        value.ok_or_else(|| invalid("owned original operation exceeds its complete bound"))?;
    let envelope: Envelope = serde_json::from_str(&value)
        .map_err(|_| invalid("malformed owned original-operation envelope"))?;
    envelope.operation.validate()?;
    if envelope.sha256 != envelope.operation.digest()?
        || envelope.operation.summary()?.operation_id != id
        || encode_bounded(&envelope, MAX_ORIGINAL_OPERATION_BYTES)? != value.as_bytes()
    {
        return Err(invalid(
            "owned original operation failed canonical full-record bindings",
        ));
    }
    Ok(Some(envelope.operation))
}
fn write(conn: &Connection, operation: Operation) -> Result<()> {
    operation.validate()?;
    let id = operation.summary()?.operation_id;
    let envelope = Envelope {
        sha256: operation.digest()?,
        operation,
    };
    let bytes = String::from_utf8(encode_bounded(&envelope, MAX_ORIGINAL_OPERATION_BYTES)?)
        .map_err(|_| invalid("invalid operation encoding"))?;
    conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key(id),bytes])?;
    Ok(())
}
fn ordered_history(
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

fn check_settings(conn: &Connection) -> Result<()> {
    let schema: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='settings' AND type='table'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let normalized = |sql: &str| {
        sql.trim()
            .trim_end_matches(';')
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .collect::<String>()
    };
    let expected = super::MIGRATIONS[0]
        .split(';')
        .next()
        .expect("settings migration");
    let extras: i64 = conn.query_row("SELECT count(*) FROM sqlite_schema WHERE tbl_name='settings' AND name NOT IN ('settings','sqlite_autoindex_settings_1')", [], |r| r.get(0))?;
    if schema.is_none_or(|sql| normalized(&sql) != normalized(expected)) || extras != 0 {
        return Err(invalid(
            "original-operation settings schema differs from its owned shape",
        ));
    }
    Ok(())
}
#[derive(Clone)]
struct Binding {
    original: [u8; 32],
    namespace: [u8; 32],
    removal_digest: Option<[u8; 32]>,
}
fn index(conn: &Connection) -> Result<HashMap<Uuid, Vec<InboxOriginalOperationSummary>>> {
    check_settings(conn)?;
    let mut groups: HashMap<Uuid, Vec<InboxOriginalOperationSummary>> = HashMap::new();
    let mut bindings = HashMap::new();
    let mut captures = HashMap::new();
    let mut turns = HashMap::new();
    for id in ids(conn)? {
        // Never retain all full bodies: each bounded owned row is checked/dropped.
        let operation =
            read(conn, id)?.ok_or_else(|| invalid("listed original operation disappeared"))?;
        let summary = operation.summary()?;
        let original = operation.original();
        if inbox::read(conn, original.capture.id)?.as_ref() != Some(original) {
            return Err(invalid(
                "original-operation certificate lost its exact catalog capture",
            ));
        }
        if let Operation::Remove(record) = &operation {
            for analysis in &record.evidence.snapshot.review.analyses {
                let id = analysis.job.capture.id;
                let sha = digest(&analysis.job)?;
                if captures.insert(id, sha).is_some_and(|known| known != sha)
                    || inbox_actions::reserved(conn, id)?.as_ref() != Some(&analysis.job)
                {
                    return Err(invalid(
                        "archived analysis captures disagree or lost their exact current reservation",
                    ));
                }
                if let Some(turn) = &analysis.turn {
                    let sha = digest(turn)?;
                    if turns.insert(id, sha).is_some_and(|known| known != sha) {
                        return Err(invalid(
                            "archived same-ID turns disagree across certificates",
                        ));
                    }
                }
            }
        }
        bindings.insert(
            id,
            Binding {
                original: digest(original)?,
                namespace: digest(operation.namespace())?,
                removal_digest: match &operation {
                    Operation::Restore(r) => Some(r.request.removal_digest),
                    _ => None,
                },
            },
        );
        groups.entry(summary.item_id).or_default().push(summary);
    }
    for values in groups.values_mut() {
        *values = ordered_history(std::mem::take(values))?;
        for pair in values.windows(2) {
            let [parent, child] = pair else {
                unreachable!()
            };
            let a = &bindings[&parent.operation_id];
            let b = &bindings[&child.operation_id];
            if a.original != b.original
                || a.namespace != b.namespace
                || b.removal_digest
                    .is_some_and(|digest| digest != parent.record_sha256)
            {
                return Err(invalid(
                    "causal original-operation certificates disagree on exact origin/namespace/removal digest",
                ));
            }
        }
        // Each Remove retained the complete previously observed history, including
        // exact digests/terminal identities. Summary membership changes invalidate it.
        for (i, summary) in values.iter().enumerate() {
            if summary.kind == InboxOriginalOperationKind::Remove {
                let Some(Operation::Remove(record)) = read(conn, summary.operation_id)? else {
                    return Err(invalid("removal kind changed"));
                };
                if record.evidence.snapshot.original_operations != values[..i] {
                    return Err(invalid(
                        "removal certificate differs from its complete causal predecessor history",
                    ));
                }
            }
        }
    }
    Ok(groups)
}
pub(super) fn check_all(conn: &Connection) -> Result<()> {
    index(conn).map(|_| ())
}
pub(super) fn history(conn: &Connection, item: Uuid) -> Result<Vec<InboxOriginalOperationSummary>> {
    nonnil(item)?;
    Ok(index(conn)?.remove(&item).unwrap_or_default())
}
pub(super) fn archived(
    conn: &Connection,
    analysis_id: Uuid,
) -> Result<Option<ArchivedInboxAnalysis>> {
    nonnil(analysis_id)?;
    check_all(conn)?;
    if super::chat::read_turn(conn, analysis_id)?.is_some() {
        return Ok(None);
    }
    let mut result: Option<ArchivedInboxAnalysis> = None;
    for id in ids(conn)? {
        if let Some(Operation::Remove(record)) = read(conn, id)? {
            for analysis in record.evidence.snapshot.review.analyses {
                if analysis.job.capture.id == analysis_id {
                    if inbox_actions::reserved(conn, analysis_id)?.as_ref() != Some(&analysis.job) {
                        return Err(Error::StateChanged(
                            "historical analysis capture differs from current reservation".into(),
                        ));
                    }
                    if result.as_ref().is_none_or(|known| {
                        known.analysis.turn.is_none() && analysis.turn.is_some()
                    }) {
                        result = Some(ArchivedInboxAnalysis {
                            removal_operation_id: id,
                            analysis,
                        });
                    }
                }
            }
        }
    }
    Ok(result)
}
fn bootstrap(conn: &Connection, record: &InboxOriginalRemovalRecord) -> Result<()> {
    inbox::restore_item(conn, &record.evidence.snapshot.review.original)?;
    for analysis in &record.evidence.snapshot.review.analyses {
        inbox_actions::restore_capture(conn, &analysis.job)?;
    }
    Ok(())
}
fn immutable_remove(
    a: &InboxOriginalRemovalRecord,
    b: &InboxOriginalRemovalRecord,
) -> Result<bool> {
    let mut a = a.clone();
    let mut b = b.clone();
    a.removed_at_ms = None;
    b.removed_at_ms = None;
    equal(&a, &b)
}
fn immutable_restore(
    a: &InboxOriginalRestoreRecord,
    b: &InboxOriginalRestoreRecord,
) -> Result<bool> {
    let mut a = a.clone();
    let mut b = b.clone();
    a.restored_at_ms = None;
    b.restored_at_ms = None;
    equal(&a, &b)
}
fn merge_remove(
    current: &InboxOriginalRemovalRecord,
    incoming: &InboxOriginalRemovalRecord,
) -> Result<InboxOriginalRemovalRecord> {
    if !immutable_remove(current, incoming)?
        || current
            .removed_at_ms
            .zip(incoming.removed_at_ms)
            .is_some_and(|(a, b)| a != b)
    {
        return Err(conflict());
    }
    let mut result = current.clone();
    result.removed_at_ms = current.removed_at_ms.or(incoming.removed_at_ms);
    Ok(result)
}
fn merge_restore(
    current: &InboxOriginalRestoreRecord,
    incoming: &InboxOriginalRestoreRecord,
) -> Result<InboxOriginalRestoreRecord> {
    if !immutable_restore(current, incoming)?
        || current
            .restored_at_ms
            .zip(incoming.restored_at_ms)
            .is_some_and(|(a, b)| a != b)
    {
        return Err(conflict());
    }
    let mut result = current.clone();
    result.restored_at_ms = current.restored_at_ms.or(incoming.restored_at_ms);
    Ok(result)
}
impl WorkStore {
    pub fn inbox_original_operation_summary(
        &self,
        id: Uuid,
    ) -> Result<Option<InboxOriginalOperationSummary>> {
        nonnil(id)?;
        let tx = self.conn.unchecked_transaction()?;
        check_all(&tx)?;
        let result = read(&tx, id)?.map(|r| r.summary()).transpose()?;
        tx.commit()?;
        Ok(result)
    }
    pub fn inbox_original_removal(&self, id: Uuid) -> Result<Option<InboxOriginalRemovalRecord>> {
        nonnil(id)?;
        let tx = self.conn.unchecked_transaction()?;
        check_all(&tx)?;
        let result = match read(&tx, id)? {
            Some(Operation::Remove(r)) => Some(*r),
            Some(_) => return Err(conflict()),
            None => None,
        };
        tx.commit()?;
        Ok(result)
    }
    pub fn inbox_original_restore(&self, id: Uuid) -> Result<Option<InboxOriginalRestoreRecord>> {
        nonnil(id)?;
        let tx = self.conn.unchecked_transaction()?;
        check_all(&tx)?;
        let result = match read(&tx, id)? {
            Some(Operation::Restore(r)) => Some(*r),
            Some(_) => return Err(conflict()),
            None => None,
        };
        tx.commit()?;
        Ok(result)
    }
    pub fn inbox_original_operation_summaries(
        &self,
        item: Uuid,
    ) -> Result<Vec<InboxOriginalOperationSummary>> {
        let tx = self.conn.unchecked_transaction()?;
        let result = history(&tx, item)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn inbox_original_operation_head(
        &self,
        item: Uuid,
    ) -> Result<Option<InboxOriginalOperationSummary>> {
        Ok(self.inbox_original_operation_summaries(item)?.pop())
    }
    pub fn inbox_original_operation_ids(&self) -> Result<Vec<Uuid>> {
        let tx = self.conn.unchecked_transaction()?;
        check_all(&tx)?;
        let result = ids(&tx)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn archived_inbox_analysis(&self, id: Uuid) -> Result<Option<ArchivedInboxAnalysis>> {
        let tx = self.conn.unchecked_transaction()?;
        let result = archived(&tx, id)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn prepare_inbox_original_removal(
        &mut self,
        request: &RemoveInboxOriginalRequest,
        evidence: &InboxQualifiedRemovalEvidence,
        namespace: &InboxOriginalNamespace,
    ) -> Result<InboxOriginalRemovalRecord> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_all(&tx)?;
        if let Some(existing) = read(&tx, request.operation_id)? {
            if let Operation::Remove(record) = existing
                && record.request == *request
            {
                tx.commit()?;
                return Ok(*record);
            }
            return Err(conflict());
        }
        request.validate()?;
        evidence.validate()?;
        namespace.validate()?;
        if request.item_id != evidence.snapshot.review.original.capture.id
            || request.preview_digest != evidence.digest()?
            || !equal(
                &super::inbox_removal::read(&tx, request.item_id)?,
                &evidence.snapshot,
            )?
        {
            return Err(Error::StateChanged(
                "complete original-removal preview changed".into(),
            ));
        }
        let previous = history(&tx, request.item_id)?.pop();
        if previous.as_ref().map(|p| p.operation_id) != request.previous_restore
            || previous.as_ref().is_some_and(|p| {
                p.kind != InboxOriginalOperationKind::Restore || p.settled_at_ms.is_none()
            })
        {
            return Err(Error::StateChanged(
                "original removal needs the exact settled Restore head or first root".into(),
            ));
        }
        let record = InboxOriginalRemovalRecord {
            request: request.clone(),
            evidence: evidence.clone(),
            namespace: namespace.clone(),
            prepared_at_ms: now_ms()
                .max(evidence.snapshot.review.original.received_at_ms)
                .max(previous.and_then(|p| p.settled_at_ms).unwrap_or(0)),
            removed_at_ms: None,
        };
        write(&tx, Operation::Remove(Box::new(record.clone())))?;
        check_all(&tx)?;
        tx.commit()?;
        Ok(record)
    }
    pub fn settle_inbox_original_removal(
        &mut self,
        record: &InboxOriginalRemovalRecord,
    ) -> Result<InboxOriginalRemovalRecord> {
        record.validate()?;
        if record.removed_at_ms.is_none() {
            return Err(invalid(
                "settlement needs an exact Removed terminal timestamp",
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_all(&tx)?;
        let Some(Operation::Remove(existing)) = read(&tx, record.request.operation_id)? else {
            return Err(conflict());
        };
        let merged = merge_remove(&existing, record)?;
        write(&tx, Operation::Remove(Box::new(merged.clone())))?;
        check_all(&tx)?;
        tx.commit()?;
        Ok(merged)
    }
    pub fn restore_inbox_original_removal(
        &mut self,
        record: &InboxOriginalRemovalRecord,
    ) -> Result<InboxOriginalRemovalRecord> {
        record.validate()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_all(&tx)?;
        let merged = match read(&tx, record.request.operation_id)? {
            Some(Operation::Remove(current)) => merge_remove(&current, record)?,
            Some(_) => return Err(conflict()),
            None => record.clone(),
        };
        bootstrap(&tx, &merged)?;
        write(&tx, Operation::Remove(Box::new(merged.clone())))?;
        check_all(&tx)?;
        tx.commit()?;
        Ok(merged)
    }
    pub fn prepare_inbox_original_restore(
        &mut self,
        request: &RestoreInboxOriginalRequest,
    ) -> Result<InboxOriginalRestoreRecord> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_all(&tx)?;
        if let Some(existing) = read(&tx, request.operation_id)? {
            if let Operation::Restore(record) = existing
                && record.request == *request
            {
                tx.commit()?;
                return Ok(*record);
            }
            return Err(conflict());
        }
        request.validate()?;
        let Some(Operation::Remove(removal)) = read(&tx, request.removal_operation_id)? else {
            return Err(conflict());
        };
        if removal.removed_at_ms.is_none() || request.removal_digest != removal.digest()? {
            return Err(Error::StateChanged(
                "Restore needs the exact settled Remove digest".into(),
            ));
        }
        let head = history(&tx, removal.request.item_id)?.pop();
        if head.as_ref().map(|h| h.operation_id) != Some(request.removal_operation_id) {
            return Err(Error::StateChanged(
                "Restore source is not the current causal Remove head".into(),
            ));
        }
        let record = InboxOriginalRestoreRecord {
            request: request.clone(),
            original: removal.evidence.snapshot.review.original,
            namespace: removal.namespace,
            prepared_at_ms: now_ms().max(removal.removed_at_ms.unwrap()),
            restored_at_ms: None,
        };
        write(&tx, Operation::Restore(Box::new(record.clone())))?;
        check_all(&tx)?;
        tx.commit()?;
        Ok(record)
    }
    pub fn settle_inbox_original_restore(
        &mut self,
        record: &InboxOriginalRestoreRecord,
    ) -> Result<InboxOriginalRestoreRecord> {
        record.validate()?;
        if record.restored_at_ms.is_none() {
            return Err(invalid(
                "settlement needs an exact Restored terminal timestamp",
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_all(&tx)?;
        let Some(Operation::Restore(existing)) = read(&tx, record.request.operation_id)? else {
            return Err(conflict());
        };
        let merged = merge_restore(&existing, record)?;
        write(&tx, Operation::Restore(Box::new(merged.clone())))?;
        check_all(&tx)?;
        tx.commit()?;
        Ok(merged)
    }
    pub fn restore_inbox_original_restore(
        &mut self,
        record: &InboxOriginalRestoreRecord,
    ) -> Result<InboxOriginalRestoreRecord> {
        record.validate()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_all(&tx)?;
        let merged = match read(&tx, record.request.operation_id)? {
            Some(Operation::Restore(current)) => merge_restore(&current, record)?,
            Some(_) => return Err(conflict()),
            None => record.clone(),
        };
        write(&tx, Operation::Restore(Box::new(merged.clone())))?;
        check_all(&tx)?;
        tx.commit()?;
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_encoded_bound_reserves_envelope_and_never_clips() {
        let exact = "x".repeat(CORE_BYTES - 2);
        let encoded = encode(&exact).unwrap();
        assert_eq!(encoded.len(), CORE_BYTES);
        assert_eq!(serde_json::from_slice::<String>(&encoded).unwrap(), exact);
        assert!(encode(&(exact + "x")).is_err());
        let complete = "x".repeat(MAX_ORIGINAL_OPERATION_BYTES - 2);
        assert_eq!(
            encode_bounded(&complete, MAX_ORIGINAL_OPERATION_BYTES)
                .unwrap()
                .len(),
            MAX_ORIGINAL_OPERATION_BYTES
        );
        assert!(encode_bounded(&(complete + "x"), MAX_ORIGINAL_OPERATION_BYTES).is_err());
        // Encoding, not character count, bounds escaped/unicode evidence.
        assert!(encode(&"\n".repeat(CORE_BYTES / 2)).is_err());
    }
    #[test]
    fn causal_history_is_not_sorted_by_timestamp_and_refuses_disconnected_cycles() {
        let item = Uuid::new_v4();
        let root = InboxOriginalOperationSummary {
            kind: InboxOriginalOperationKind::Remove,
            operation_id: Uuid::new_v4(),
            item_id: item,
            parent: None,
            prepared_at_ms: 10,
            settled_at_ms: Some(10),
            record_sha256: [1; 32],
        };
        let child = InboxOriginalOperationSummary {
            kind: InboxOriginalOperationKind::Restore,
            operation_id: Uuid::new_v4(),
            parent: Some(root.operation_id),
            record_sha256: [2; 32],
            ..root.clone()
        };
        assert_eq!(
            ordered_history(vec![child.clone(), root.clone()]).unwrap(),
            vec![root.clone(), child.clone()]
        );
        let mut fork = child.clone();
        fork.operation_id = Uuid::new_v4();
        assert!(ordered_history(vec![root.clone(), child.clone(), fork]).is_err());
        let mut unknown = child.clone();
        unknown.parent = Some(Uuid::new_v4());
        assert!(ordered_history(vec![root.clone(), unknown]).is_err());
        let mut cycle_root = root.clone();
        cycle_root.parent = Some(child.operation_id);
        assert!(ordered_history(vec![cycle_root, child.clone()]).is_err());
        let pending = InboxOriginalOperationSummary {
            settled_at_ms: None,
            ..root.clone()
        };
        assert!(ordered_history(vec![pending, child]).is_err());
    }
}
