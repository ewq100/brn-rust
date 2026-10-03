//! Whole-proposal approval journals and exact file proofs. No filesystem I/O.
use super::{
    MAX_NOTE_BYTES, WorkStore, now_ms,
    proposals::{self, NoteChange, ProposalRecord, ProposalStamp, ProposalState},
};
use crate::{Error, Result, files::FileFingerprint, hash, invalid};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub(super) const V5: &str = "
CREATE TABLE proposal_applies (
    operation_id TEXT PRIMARY KEY,
    proposal_id TEXT NOT NULL REFERENCES proposals(id),
    outcome TEXT CHECK(outcome IN ('applied','not_applied','uncertain')),
    request_sha256 BLOB NOT NULL CHECK(length(request_sha256)=32),
    journal_json BLOB NOT NULL,
    journal_sha256 BLOB NOT NULL CHECK(length(journal_sha256)=32)
);
CREATE UNIQUE INDEX one_unresolved_proposal_apply ON proposal_applies((1))
    WHERE outcome IS NULL OR outcome='uncertain';";

const MAX_METADATA_BYTES: usize = 256 * 1024;
const MAX_JOURNAL_BYTES: usize = proposals::MAX_STORED_BYTES + MAX_METADATA_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalRequest {
    pub operation_id: Uuid,
    pub expected: ProposalStamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyOutcome {
    Applied,
    NotApplied,
    Uncertain,
}

impl ApplyOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::NotApplied => "not_applied",
            Self::Uncertain => "uncertain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyMember {
    pub id: Uuid,
    pub staging: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyMemberProof {
    pub destination: Option<FileFingerprint>,
    pub staging: Option<FileFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyReceipt {
    pub operation_id: Uuid,
    pub proposal_id: Uuid,
    pub approved_version: u64,
    pub stamp: ProposalStamp,
    pub outcome: ApplyOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyJournal {
    pub request: ApprovalRequest,
    pub approved: ProposalRecord,
    pub creation_sha256: [u8; 32],
    pub members: Vec<ApplyMember>,
    pub prepared: Option<Vec<FileFingerprint>>,
    pub receipt: Option<ApplyReceipt>,
    pub observations: Option<Vec<ApplyMemberProof>>,
    /// Certified by the workflow only before any namespace-effect attempt.
    #[serde(default, skip_serializing_if = "is_false")]
    pub no_effects: bool,
    pub started_at_ms: u64,
}

fn is_false(value: &bool) -> bool {
    !value
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| invalid("could not encode proposal approval journal"))
}

fn stages_for(id: Uuid, change: &NoteChange) -> PathBuf {
    Path::new(change.path()).with_file_name(format!(".brn-{id}.stage"))
}

fn next_version(version: u64, steps: u64) -> Result<u64> {
    version
        .checked_add(steps)
        .ok_or_else(|| invalid("proposal approval version overflow"))
}

fn bounded_fingerprint(proof: &FileFingerprint) -> Result<()> {
    if proof.len > MAX_NOTE_BYTES as u64 {
        return Err(invalid(
            "proposal approval proof exceeds the 1 MiB note limit",
        ));
    }
    Ok(())
}

impl ApplyJournal {
    /// Validates the bounded domain and all journal bindings without observing
    /// files or SQLite. An ordinary-file mirror must pass the same checks.
    pub fn validate(&self) -> Result<()> {
        proposals::nonnil(self.request.operation_id)?;
        proposals::validate_record(&self.approved)?;
        if self.no_effects
            && self.receipt.as_ref().map(|receipt| receipt.outcome)
                != Some(ApplyOutcome::NotApplied)
        {
            return Err(invalid(
                "no-effect certification requires a NotApplied receipt",
            ));
        }
        if self.approved.state != ProposalState::Draft
            || self.request.expected != self.approved.stamp()
            || self.started_at_ms < self.approved.updated_at_ms
            || self.members.len() != self.approved.draft.changes.len()
        {
            return Err(invalid(
                "approval journal differs from its exact Draft review snapshot",
            ));
        }
        // Reserve enough room for admission, uncertainty and reconciliation.
        next_version(self.approved.version, 3)?;
        if self.approved.version == 1
            && self.creation_sha256 != hash(&encode(&self.approved.draft)?)
        {
            return Err(invalid(
                "approval journal differs from its initial creation binding",
            ));
        }
        let mut member_ids = HashSet::new();
        for (member, change) in self.members.iter().zip(&self.approved.draft.changes) {
            proposals::nonnil(member.id)?;
            if !member_ids.insert(member.id) || member.staging != stages_for(member.id, change) {
                return Err(invalid(
                    "approval journal has duplicate members or an unbound sibling staging path",
                ));
            }
        }
        if let Some(prepared) = &self.prepared {
            self.validate_prepared(prepared)?;
        }
        if let Some(observations) = &self.observations {
            if observations.len() != self.members.len() {
                return Err(invalid(
                    "approval journal requires a complete observation set",
                ));
            }
            for proof in observations {
                for fingerprint in [&proof.destination, &proof.staging].into_iter().flatten() {
                    bounded_fingerprint(fingerprint)?;
                }
            }
        }
        if let Some(receipt) = &self.receipt {
            if receipt.outcome == ApplyOutcome::Applied && !self.approved.comments.is_empty() {
                return Err(invalid(
                    "applied approval journal must not retain temporary review comments",
                ));
            }
            if receipt.operation_id != self.request.operation_id
                || receipt.proposal_id != self.approved.draft.id
                || receipt.approved_version != self.approved.version
                || receipt.stamp.id != self.approved.draft.id
            {
                return Err(invalid("approval receipt differs from its bound request"));
            }
            let first_terminal = next_version(self.approved.version, 2)?;
            let reconciled = next_version(self.approved.version, 3)?;
            if self.no_effects && receipt.stamp.version != first_terminal {
                return Err(invalid(
                    "no-effect certification cannot follow interrupted application work",
                ));
            }
            if receipt.stamp.version != first_terminal
                && (receipt.outcome == ApplyOutcome::Uncertain
                    || receipt.stamp.version != reconciled)
            {
                return Err(invalid("approval receipt has an invalid review version"));
            }
            self.validate_outcome(receipt.outcome)?;
        } else if self.observations.is_some() {
            return Err(invalid(
                "approval observations require a whole-proposal receipt",
            ));
        }
        // Paths occur both in the approved review and in member metadata. Bound
        // the latter separately, rather than letting it duplicate unbounded text.
        let metadata = encode(&(
            &self.request,
            self.creation_sha256,
            &self.members,
            &self.prepared,
            &self.receipt,
            &self.observations,
            self.no_effects,
            self.started_at_ms,
        ))?;
        if metadata.len() > MAX_METADATA_BYTES - 1024 {
            return Err(invalid(
                "approval journal metadata exceeds its bounded size limit",
            ));
        }
        Ok(())
    }

    fn validate_prepared(&self, prepared: &[FileFingerprint]) -> Result<()> {
        if prepared.len() != self.members.len() {
            return Err(invalid(
                "proposal approval requires all prepared member fingerprints",
            ));
        }
        let mut old = HashSet::new();
        for change in &self.approved.draft.changes {
            if let NoteChange::Replace { before, .. } | NoteChange::Trash { before, .. } = change {
                old.insert((before.device, before.inode));
            }
        }
        for source in &self.approved.draft.sources {
            old.insert((source.fingerprint.device, source.fingerprint.inode));
        }
        let mut new = HashSet::new();
        for (change, proof) in self.approved.draft.changes.iter().zip(prepared) {
            bounded_fingerprint(proof)?;
            match change {
                NoteChange::Create { text, .. } | NoteChange::Replace { text, .. } => {
                    if proof.len != text.len() as u64
                        || proof.sha256 != hash(text.as_bytes())
                        || old.contains(&(proof.device, proof.inode))
                        || !new.insert((proof.device, proof.inode))
                    {
                        return Err(invalid(
                            "prepared proposal member differs from its bytes or independent new identity",
                        ));
                    }
                }
                NoteChange::Trash { before, .. } if proof == before => {}
                NoteChange::Trash { .. } => {
                    return Err(invalid("prepared Trash proof must be its exact original"));
                }
            }
        }
        Ok(())
    }

    fn validate_outcome(&self, outcome: ApplyOutcome) -> Result<()> {
        if outcome == ApplyOutcome::Uncertain
            || self.no_effects && outcome == ApplyOutcome::NotApplied
        {
            return Ok(());
        }
        let observations = self.observations.as_deref().ok_or_else(|| {
            invalid("settled approval requires complete destination observations")
        })?;
        match outcome {
            ApplyOutcome::Applied => {
                let prepared = self
                    .prepared
                    .as_deref()
                    .ok_or_else(|| invalid("applied proposal requires all prepared proofs"))?;
                for ((change, new), observed) in self
                    .approved
                    .draft
                    .changes
                    .iter()
                    .zip(prepared)
                    .zip(observations)
                {
                    let exact = match change {
                        NoteChange::Create { .. } => {
                            observed.destination.as_ref() == Some(new) && observed.staging.is_none()
                        }
                        NoteChange::Replace { before, .. } => {
                            observed.destination.as_ref() == Some(new)
                                && observed.staging.as_ref() == Some(before)
                        }
                        NoteChange::Trash { before, .. } => {
                            observed.destination.is_none()
                                && observed.staging.as_ref() == Some(before)
                        }
                    };
                    if !exact {
                        return Err(invalid(
                            "applied proposal lacks exact installed and retained-original proofs",
                        ));
                    }
                }
            }
            ApplyOutcome::NotApplied => {
                for (change, observed) in self.approved.draft.changes.iter().zip(observations) {
                    let exact = match change {
                        NoteChange::Create { .. } => observed.destination.is_none(),
                        NoteChange::Replace { before, .. } | NoteChange::Trash { before, .. } => {
                            observed.destination.as_ref() == Some(before)
                        }
                    };
                    if !exact {
                        return Err(invalid(
                            "not-applied proposal requires every unchanged destination baseline",
                        ));
                    }
                }
            }
            ApplyOutcome::Uncertain => unreachable!(),
        }
        Ok(())
    }
}

struct JournalRow {
    proposal_id: String,
    outcome: Option<String>,
    request_hash: Vec<u8>,
    bytes: Option<Vec<u8>>,
    digest: Vec<u8>,
}

fn read_journal(conn: &Connection, id: Uuid) -> Result<Option<ApplyJournal>> {
    proposals::nonnil(id)?;
    let row: Option<JournalRow> = conn.query_row(
        "SELECT proposal_id,outcome,request_sha256,CASE WHEN length(journal_json)<=?2 THEN journal_json END,journal_sha256 FROM proposal_applies WHERE operation_id=?1",
        params![id.to_string(), MAX_JOURNAL_BYTES as i64],
        |row| Ok(JournalRow { proposal_id: row.get(0)?, outcome: row.get(1)?, request_hash: row.get(2)?, bytes: row.get(3)?, digest: row.get(4)? }),
    ).optional()?;
    row.map(|row| {
        let bytes = row
            .bytes
            .ok_or_else(|| invalid("stored approval journal exceeds its encoded size limit"))?;
        if row.digest.as_slice() != hash(&bytes) {
            return Err(invalid("stored approval journal failed its hash check"));
        }
        let journal: ApplyJournal = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("invalid stored approval journal"))?;
        if journal.request.operation_id != id
            || journal.approved.draft.id.to_string() != row.proposal_id
            || journal
                .receipt
                .as_ref()
                .map(|receipt| receipt.outcome.as_str().to_owned())
                != row.outcome
            || row.request_hash.as_slice() != hash(&encode(&journal.request)?)
        {
            return Err(invalid(
                "stored approval journal differs from its indexed bindings",
            ));
        }
        journal.validate()?;
        let proposal = proposals::read_proposal(conn, journal.approved.draft.id)?
            .ok_or_else(|| invalid("stored approval journal has no bound proposal"))?;
        if proposal.creation_sha256 != journal.creation_sha256 {
            return Err(invalid(
                "stored approval journal differs from its original creation binding",
            ));
        }
        if proposal.record.state == ProposalState::Applied && !journal.approved.comments.is_empty()
        {
            return Err(invalid(
                "approved proposal journals must not retain temporary review comments",
            ));
        }
        Ok(journal)
    })
    .transpose()
}

fn write_journal(conn: &Connection, journal: &ApplyJournal) -> Result<()> {
    journal.validate()?;
    let bytes = encode(journal)?;
    if bytes.len() > MAX_JOURNAL_BYTES {
        return Err(invalid("approval journal exceeds its encoded size limit"));
    }
    conn.execute(
        "UPDATE proposal_applies SET outcome=?2,journal_json=?3,journal_sha256=?4 WHERE operation_id=?1",
        params![journal.request.operation_id.to_string(), journal.receipt.as_ref().map(|receipt| receipt.outcome.as_str()), bytes, hash(&bytes).as_slice()],
    )?;
    Ok(())
}

fn purge_prior_comments(
    conn: &Connection,
    proposal_id: Uuid,
    current_id: Uuid,
    through_version: Option<u64>,
) -> Result<()> {
    let mut statement = conn.prepare(
        "SELECT operation_id FROM proposal_applies WHERE proposal_id=?1 AND operation_id!=?2 ORDER BY rowid",
    )?;
    let ids = statement
        .query_map(
            params![proposal_id.to_string(), current_id.to_string()],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);
    for id in ids {
        let mut journal = read_journal(conn, crate::parse_id(id)?)?
            .ok_or_else(|| invalid("prior approval journal disappeared during comment cleanup"))?;
        if through_version.is_none_or(|version| journal.approved.version <= version)
            && !journal.approved.comments.is_empty()
        {
            // Annotation cleanup is the only allowed alteration of an older
            // approval snapshot. Its request, proofs and receipt remain exact.
            journal.approved.comments.clear();
            write_journal(conn, &journal)?;
        }
    }
    Ok(())
}

fn current_unresolved(
    conn: &Connection,
    journal: &ApplyJournal,
) -> Result<proposals::StoredProposal> {
    let stored = proposals::read_proposal(conn, journal.approved.draft.id)?
        .ok_or_else(|| Error::NotFound("approval proposal is absent".into()))?;
    let (state, version) = match &journal.receipt {
        None => (
            ProposalState::Applying,
            next_version(journal.approved.version, 1)?,
        ),
        Some(receipt) if receipt.outcome == ApplyOutcome::Uncertain => (
            ProposalState::Uncertain,
            next_version(journal.approved.version, 2)?,
        ),
        _ => {
            return Err(Error::StateChanged(
                "approval journal is already settled".into(),
            ));
        }
    };
    if stored.creation_sha256 != journal.creation_sha256
        || stored.record.draft != journal.approved.draft
        || stored.record.comments != journal.approved.comments
        || stored.record.created_at_ms != journal.approved.created_at_ms
        || stored.record.updated_at_ms < journal.approved.updated_at_ms
        || stored.record.state != state
        || stored.record.version != version
    {
        return Err(Error::StateChanged(
            "unresolved approval no longer matches its immutable review work".into(),
        ));
    }
    Ok(stored)
}

fn settled(journal: &ApplyJournal) -> bool {
    journal
        .receipt
        .as_ref()
        .is_some_and(|receipt| receipt.outcome != ApplyOutcome::Uncertain)
}

fn applied_receipt(journal: &ApplyJournal) -> bool {
    journal
        .receipt
        .as_ref()
        .is_some_and(|receipt| receipt.outcome == ApplyOutcome::Applied)
}

fn same_approval(left: &ApplyJournal, right: &ApplyJournal) -> bool {
    let mut left_review = left.approved.clone();
    let mut right_review = right.approved.clone();
    left_review.comments.clear();
    right_review.comments.clear();
    left.request == right.request
        && left.creation_sha256 == right.creation_sha256
        && left_review == right_review
        && left.started_at_ms == right.started_at_ms
        && left.members == right.members
}

fn merge_journal(
    existing: &ApplyJournal,
    incoming: &ApplyJournal,
    live_applied: bool,
) -> Result<ApplyJournal> {
    if !same_approval(existing, incoming) {
        return Err(Error::OperationConflict(
            "recovery journal has incompatible approval bindings".into(),
        ));
    }
    if let (Some(old), Some(new)) = (&existing.prepared, &incoming.prepared)
        && old != new
    {
        return Err(Error::OperationConflict(
            "recovery journal has incompatible prepared proofs".into(),
        ));
    }
    if settled(existing) {
        if settled(incoming)
            && (existing.receipt != incoming.receipt
                || existing.observations != incoming.observations
                || existing.no_effects != incoming.no_effects
                || existing.prepared.is_none() && incoming.prepared.is_some())
        {
            return Err(Error::OperationConflict(
                "recovery journal conflicts with its settled receipt".into(),
            ));
        }
        if !existing.approved.comments.is_empty()
            && !incoming.approved.comments.is_empty()
            && existing.approved.comments != incoming.approved.comments
        {
            return Err(Error::OperationConflict(
                "recovery journal has incompatible review comments".into(),
            ));
        }
        // A terminal receipt wins over earlier snapshots and annotation cleanup
        // cannot reintroduce already deleted temporary comments.
        return Ok(existing.clone());
    }
    let mut next = if existing.receipt.is_some() && incoming.receipt.is_none() {
        existing.clone()
    } else {
        if existing.receipt.is_some() && incoming.no_effects {
            return Err(Error::StateChanged(
                "no-effect recovery cannot discharge interrupted application work".into(),
            ));
        }
        if existing.receipt.is_some()
            && incoming
                .receipt
                .as_ref()
                .is_some_and(|receipt| receipt.outcome == ApplyOutcome::Uncertain)
            && (existing.receipt != incoming.receipt
                || existing.observations != incoming.observations)
        {
            return Err(Error::OperationConflict(
                "recovery journal has incompatible uncertain observations".into(),
            ));
        }
        incoming.clone()
    };
    if existing.approved.comments != incoming.approved.comments
        && !live_applied
        && !applied_receipt(incoming)
    {
        if incoming
            .receipt
            .as_ref()
            .is_some_and(|receipt| receipt.outcome == ApplyOutcome::NotApplied)
            && incoming.approved.comments.is_empty()
        {
            // A mirror may have had its old annotations cleaned by a later
            // approval. Retain SQLite's review work until that approval imports.
            next.approved.comments = existing.approved.comments.clone();
        } else {
            return Err(Error::OperationConflict(
                "recovery journal has incompatible review comments".into(),
            ));
        }
    }
    if live_applied {
        next.approved.comments.clear();
    }
    if next.prepared.is_none() {
        next.prepared = existing.prepared.clone();
    }
    if next.prepared.is_none() && existing.receipt.is_some() && incoming.receipt.is_none() {
        next.prepared = incoming.prepared.clone();
    }
    next.validate()?;
    Ok(next)
}

fn immutable_review_bindings(record: &ProposalRecord) -> super::proposals::ProposalDraft {
    let mut draft = record.draft.clone();
    draft.title.clear();
    for change in &mut draft.changes {
        match change {
            NoteChange::Create { text, .. } | NoteChange::Replace { text, .. } => text.clear(),
            NoteChange::Trash { .. } => {}
        }
    }
    draft
}

fn target_record(journal: &ApplyJournal) -> Result<ProposalRecord> {
    let mut record = journal.approved.clone();
    (record.state, record.version) = match &journal.receipt {
        None => (ProposalState::Applying, next_version(record.version, 1)?),
        Some(receipt) => (
            match receipt.outcome {
                ApplyOutcome::Applied => ProposalState::Applied,
                ApplyOutcome::NotApplied => ProposalState::Draft,
                ApplyOutcome::Uncertain => ProposalState::Uncertain,
            },
            receipt.stamp.version,
        ),
    };
    if record.state == ProposalState::Applied {
        record.comments.clear();
    }
    record.updated_at_ms = now_ms()
        .max(journal.started_at_ms)
        .max(record.updated_at_ms);
    proposals::validate_record(&record)?;
    Ok(record)
}

fn restored_review(
    current: Option<&proposals::StoredProposal>,
    journal: &ApplyJournal,
) -> Result<Option<ProposalRecord>> {
    let target = target_record(journal)?;
    let Some(current) = current else {
        return Ok(Some(target));
    };
    if current.creation_sha256 != journal.creation_sha256
        || immutable_review_bindings(&current.record)
            != immutable_review_bindings(&journal.approved)
        || current.record.created_at_ms != journal.approved.created_at_ms
    {
        return Err(Error::OperationConflict(
            "recovery snapshot has incompatible original review bindings".into(),
        ));
    }
    if current.record.version > target.version {
        if !settled(journal) {
            return Err(Error::StateChanged(
                "recovery cannot replace newer review work with an unresolved snapshot".into(),
            ));
        }
        return Ok(None);
    }
    if current.record.state == ProposalState::Applied {
        if settled(journal) {
            return Ok(None);
        }
        return Err(Error::StateChanged(
            "recovery cannot replace an Applied review with unresolved work".into(),
        ));
    }
    if current.record.version == target.version {
        if current.record.draft != target.draft
            || current.record.state != target.state
            || current.record.comments != target.comments
        {
            return Err(Error::StateChanged(
                "recovery conflicts with the current review at the same version".into(),
            ));
        }
        return Ok(None);
    }
    if current.record.version <= journal.approved.version {
        if current.record.version == journal.approved.version
            && (current.record.draft != journal.approved.draft
                || !applied_receipt(journal)
                    && current.record.comments != journal.approved.comments)
        {
            return Err(Error::StateChanged(
                "recovery conflicts with the exact approved review version".into(),
            ));
        }
    } else {
        let coherent_state = current.record.version == next_version(journal.approved.version, 1)?
            && current.record.state == ProposalState::Applying
            || current.record.version == next_version(journal.approved.version, 2)?
                && current.record.state == ProposalState::Uncertain;
        if !coherent_state
            || current.record.draft != journal.approved.draft
            || !applied_receipt(journal) && current.record.comments != journal.approved.comments
        {
            return Err(Error::StateChanged(
                "recovery conflicts with admitted review work".into(),
            ));
        }
    }
    if journal.no_effects && current.record.state == ProposalState::Uncertain {
        return Err(Error::StateChanged(
            "no-effect recovery cannot discharge interrupted review work".into(),
        ));
    }
    let mut target = target;
    target.updated_at_ms = target.updated_at_ms.max(current.record.updated_at_ms);
    Ok(Some(target))
}

fn insert_review(conn: &Connection, stored: &proposals::StoredProposal) -> Result<()> {
    proposals::validate_record(&stored.record)?;
    let bytes = encode(stored)?;
    if bytes.len() > proposals::MAX_STORED_BYTES {
        return Err(invalid("restored review exceeds its encoded size limit"));
    }
    conn.execute(
        "INSERT INTO proposals(id,group_id,creation_sha256,record_json,record_sha256) VALUES(?1,?2,?3,?4,?5)",
        params![stored.record.draft.id.to_string(), stored.record.draft.group_id.map(|id| id.to_string()), stored.creation_sha256.as_slice(), bytes, hash(&bytes).as_slice()],
    )?;
    Ok(())
}

fn insert_journal(conn: &Connection, journal: &ApplyJournal) -> Result<()> {
    journal.validate()?;
    let bytes = encode(journal)?;
    if bytes.len() > MAX_JOURNAL_BYTES {
        return Err(invalid("recovery journal exceeds its encoded size limit"));
    }
    conn.execute(
        "INSERT INTO proposal_applies(operation_id,proposal_id,outcome,request_sha256,journal_json,journal_sha256) VALUES(?1,?2,?3,?4,?5,?6)",
        params![journal.request.operation_id.to_string(), journal.approved.draft.id.to_string(), journal.receipt.as_ref().map(|receipt| receipt.outcome.as_str()), hash(&encode(&journal.request)?).as_slice(), bytes, hash(&bytes).as_slice()],
    )?;
    Ok(())
}

impl WorkStore {
    /// Imports checked ordinary recovery evidence without granting permission to
    /// install files. Newer operational receipts or live review work win.
    pub fn restore_proposal_apply(&mut self, snapshot: &ApplyJournal) -> Result<ApplyJournal> {
        snapshot.validate()?;
        let tx = self.conn.transaction()?;
        let existing = read_journal(&tx, snapshot.request.operation_id)?;
        let current = proposals::read_proposal(&tx, snapshot.approved.draft.id)?;
        let live_applied = current
            .as_ref()
            .is_some_and(|stored| stored.record.state == ProposalState::Applied);
        let mut incoming = snapshot.clone();
        if live_applied || existing.as_ref().is_some_and(applied_receipt) {
            incoming.approved.comments.clear();
        }
        let effective = if let Some(existing) = &existing {
            merge_journal(existing, &incoming, live_applied)?
        } else {
            incoming
        };
        effective.validate()?;
        let historical_terminal = effective.receipt.as_ref().is_some_and(|receipt| {
            settled(&effective)
                && current
                    .as_ref()
                    .is_some_and(|stored| stored.record.version > receipt.stamp.version)
        });
        if let Some(existing) = &existing
            && !settled(existing)
            && *existing != effective
            && !historical_terminal
        {
            // A historical terminal journal can catch up after a newer live
            // review was restored. Its immutable merge and lineage still pass
            // the checks above/below; the newer review is never replaced.
            current_unresolved(&tx, existing)?;
        }
        let next_review = restored_review(current.as_ref(), &effective)?;
        if !settled(&effective) {
            let blocked: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM proposal_applies WHERE operation_id!=?1 AND (outcome IS NULL OR outcome='uncertain'))",
                [effective.request.operation_id.to_string()], |row| row.get(0),
            )?;
            if blocked {
                return Err(Error::StateChanged(
                    "recovery conflicts with another unresolved application".into(),
                ));
            }
        }
        if current.is_none() {
            insert_review(
                &tx,
                &proposals::StoredProposal {
                    creation_sha256: effective.creation_sha256,
                    record: next_review
                        .clone()
                        .ok_or_else(|| invalid("missing restored review"))?,
                },
            )?;
        }
        if existing.as_ref() != Some(&effective) {
            if existing.is_some() {
                write_journal(&tx, &effective)?;
            } else {
                insert_journal(&tx, &effective)?;
            }
        }
        if applied_receipt(&effective) {
            purge_prior_comments(
                &tx,
                effective.approved.draft.id,
                effective.request.operation_id,
                Some(effective.approved.version),
            )?;
        }
        if current.is_some()
            && let Some(record) = next_review
        {
            proposals::write_proposal(
                &tx,
                &proposals::StoredProposal {
                    creation_sha256: effective.creation_sha256,
                    record,
                },
            )?;
        }
        tx.commit()?;
        Ok(effective)
    }

    pub fn begin_proposal_apply(&mut self, request: &ApprovalRequest) -> Result<ApplyJournal> {
        proposals::nonnil(request.operation_id)?;
        let tx = self.conn.transaction()?;
        if let Some(journal) = read_journal(&tx, request.operation_id)? {
            if journal.request != *request {
                return Err(Error::OperationConflict(
                    "approval operation UUID has another request".into(),
                ));
            }
            return Ok(journal);
        }
        let mut stored = proposals::draft_at(&tx, request.expected)?;
        let unresolved: Option<String> = tx.query_row(
            "SELECT operation_id FROM proposal_applies WHERE outcome IS NULL OR outcome='uncertain'", [], |row| row.get(0),
        ).optional()?;
        if let Some(id) = unresolved {
            read_journal(&tx, crate::parse_id(id)?)?;
            return Err(Error::StateChanged(
                "an unresolved proposal application requires reconciliation".into(),
            ));
        }
        let journal = ApplyJournal {
            request: request.clone(),
            approved: stored.record.clone(),
            creation_sha256: stored.creation_sha256,
            members: stored
                .record
                .draft
                .changes
                .iter()
                .map(|change| {
                    let id = Uuid::new_v4();
                    ApplyMember {
                        id,
                        staging: stages_for(id, change),
                    }
                })
                .collect(),
            prepared: None,
            receipt: None,
            observations: None,
            no_effects: false,
            started_at_ms: now_ms().max(stored.record.updated_at_ms),
        };
        journal.validate()?;
        let bytes = encode(&journal)?;
        if bytes.len() > MAX_JOURNAL_BYTES {
            return Err(invalid("approval journal exceeds its encoded size limit"));
        }
        stored.record.state = ProposalState::Applying;
        proposals::advance(&mut stored.record)?;
        proposals::write_proposal(&tx, &stored)?;
        tx.execute(
            "INSERT INTO proposal_applies(operation_id,proposal_id,request_sha256,journal_json,journal_sha256) VALUES(?1,?2,?3,?4,?5)",
            params![request.operation_id.to_string(), request.expected.id.to_string(), hash(&encode(request)?).as_slice(), bytes, hash(&bytes).as_slice()],
        )?;
        tx.commit()?;
        Ok(journal)
    }

    pub fn proposal_apply(&self, id: Uuid) -> Result<Option<ApplyJournal>> {
        read_journal(&self.conn, id)
    }

    pub fn proposal_applies(&self) -> Result<Vec<ApplyJournal>> {
        let mut statement = self
            .conn
            .prepare("SELECT operation_id FROM proposal_applies ORDER BY rowid")?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .map(|id| {
                read_journal(&self.conn, crate::parse_id(id?)?)?
                    .ok_or_else(|| invalid("listed approval journal disappeared"))
            })
            .collect()
    }

    pub fn record_proposal_prepared(
        &mut self,
        id: Uuid,
        prepared: &[FileFingerprint],
    ) -> Result<ApplyJournal> {
        let tx = self.conn.transaction()?;
        let mut journal = read_journal(&tx, id)?
            .ok_or_else(|| Error::NotFound("approval journal is absent".into()))?;
        if let Some(existing) = &journal.prepared {
            if existing == prepared {
                return Ok(journal);
            }
            return Err(Error::OperationConflict(
                "approval journal already has another prepared set".into(),
            ));
        }
        current_unresolved(&tx, &journal)?;
        journal.prepared = Some(prepared.to_vec());
        write_journal(&tx, &journal)?;
        tx.commit()?;
        Ok(journal)
    }

    pub fn finish_proposal_apply(
        &mut self,
        id: Uuid,
        outcome: ApplyOutcome,
        observations: Option<&[ApplyMemberProof]>,
    ) -> Result<ApplyReceipt> {
        self.settle_proposal_apply(id, outcome, observations, false)
    }

    /// The workflow certifies this branch only while it knows that no namespace
    /// effect was attempted. Unknown interrupted work uses strict completion.
    pub fn refuse_proposal_before_effects(
        &mut self,
        id: Uuid,
        observations: Option<&[ApplyMemberProof]>,
    ) -> Result<ApplyReceipt> {
        self.settle_proposal_apply(id, ApplyOutcome::NotApplied, observations, true)
    }

    fn settle_proposal_apply(
        &mut self,
        id: Uuid,
        outcome: ApplyOutcome,
        observations: Option<&[ApplyMemberProof]>,
        no_effects: bool,
    ) -> Result<ApplyReceipt> {
        let tx = self.conn.transaction()?;
        let mut journal = read_journal(&tx, id)?
            .ok_or_else(|| Error::NotFound("approval journal is absent".into()))?;
        if let Some(receipt) = &journal.receipt
            && receipt.outcome != ApplyOutcome::Uncertain
        {
            if receipt.outcome == outcome
                && journal.observations.as_deref() == observations
                && journal.no_effects == no_effects
            {
                return Ok(receipt.clone());
            }
            return Err(Error::OperationConflict(
                "settled approval differs from its receipt or observations".into(),
            ));
        }
        if no_effects && journal.receipt.is_some() {
            return Err(Error::StateChanged(
                "no-effect certification cannot discharge interrupted application work".into(),
            ));
        }
        let mut stored = current_unresolved(&tx, &journal)?;
        if let Some(receipt) = &journal.receipt
            && outcome == ApplyOutcome::Uncertain
        {
            if journal.observations.as_deref() == observations {
                return Ok(receipt.clone());
            }
            return Err(Error::OperationConflict(
                "uncertain approval already records another observation set".into(),
            ));
        }
        stored.record.state = match outcome {
            ApplyOutcome::Applied => ProposalState::Applied,
            ApplyOutcome::NotApplied => ProposalState::Draft,
            ApplyOutcome::Uncertain => ProposalState::Uncertain,
        };
        if outcome == ApplyOutcome::Applied {
            stored.record.comments.clear();
        }
        proposals::advance(&mut stored.record)?;
        let receipt = ApplyReceipt {
            operation_id: id,
            proposal_id: journal.approved.draft.id,
            approved_version: journal.approved.version,
            stamp: stored.record.stamp(),
            outcome,
        };
        journal.receipt = Some(receipt.clone());
        journal.observations = observations.map(<[ApplyMemberProof]>::to_vec);
        journal.no_effects = no_effects;
        if outcome == ApplyOutcome::Applied {
            journal.approved.comments.clear();
        }
        write_journal(&tx, &journal)?;
        if outcome == ApplyOutcome::Applied {
            // Validate and clean older rows while the live record still holds
            // its unresolved state; then commit the Applied record atomically.
            purge_prior_comments(&tx, stored.record.draft.id, id, None)?;
        }
        proposals::write_proposal(&tx, &stored)?;
        tx.commit()?;
        Ok(receipt)
    }
}
