//! Complete retained review evidence for one Inbox original. No filesystem I/O,
//! semantic completeness decision, approval or removal authority.
use super::{
    WorkStore,
    chat::{self, WorkTurn},
    findings::{self, FindingOrigin, FindingRecord},
    inbox::{self, InboxItem},
    inbox_actions::{self, InboxActionJob},
    inbox_processing::{self, InboxProcessBatch},
    inbox_source,
    proposal_apply::{self, ApplyJournal},
    proposals::{self, ProposalRecord},
};
use crate::{Result, hash, invalid};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    io::{self, Write},
};
use uuid::Uuid;

/// Reject oversized complete evidence instead of silently limiting membership.
pub const MAX_INBOX_REVIEW_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxAnalysisReview {
    pub job: InboxActionJob,
    /// Absent after explicit Session deletion or before admission. Never infer
    /// Completed from absence and never reconstruct a deleted Session here.
    pub turn: Option<WorkTurn>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxProposalReview {
    /// Preserve original creation identity even after edits/rejection.
    pub creation_sha256: [u8; 32],
    pub record: ProposalRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxReviewManifest {
    pub original: InboxItem,
    pub processing: Vec<InboxProcessBatch>,
    pub analyses: Vec<InboxAnalysisReview>,
    pub proposals: Vec<InboxProposalReview>,
    pub approvals: Vec<ApplyJournal>,
    pub findings: Vec<FindingRecord>,
}
impl InboxReviewManifest {
    /// Canonical whole-record identity; versions, membership, order and terminal
    /// outcomes all participate. This digest is evidence, never authorization.
    pub fn digest(&self) -> Result<[u8; 32]> {
        let mut writer = BoundedBytes(Vec::new());
        serde_json::to_writer(&mut writer, self)
            .map_err(|_| invalid("complete Inbox review exceeds its encoded bound"))?;
        Ok(hash(&writer.0))
    }
}
struct BoundedBytes(Vec<u8>);
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|len| len > MAX_INBOX_REVIEW_BYTES)
        {
            return Err(io::Error::other("complete Inbox review exceeds its bound"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn ids(conn: &Connection, sql: &str) -> Result<Vec<Uuid>> {
    conn.prepare(sql)?
        .query_map([], |r| r.get::<_, String>(0))?
        .map(|value| crate::parse_id(value?))
        .collect()
}
fn append<T: Serialize>(budget: &mut usize, value: &T) -> Result<()> {
    // Each underlying reader has its own record bound. Reserve the whole encoded
    // member before retaining it; many individually valid records cannot clip.
    let mut writer = BoundedBytes(Vec::new());
    serde_json::to_writer(&mut writer, value)
        .map_err(|_| invalid("Inbox review member exceeds its encoded bound"))?;
    *budget = budget
        .checked_add(writer.0.len() + 1)
        .filter(|len| *len <= MAX_INBOX_REVIEW_BYTES)
        .ok_or_else(|| invalid("complete Inbox review exceeds its encoded bound"))?;
    Ok(())
}
fn related(
    draft: &proposals::ProposalDraft,
    item_id: Uuid,
    groups: &HashSet<Uuid>,
    source_ids: &HashSet<Uuid>,
    source_paths: &HashSet<String>,
) -> bool {
    let linked_data =
        |data: &super::actions::ActionData| data.sources.iter().any(|id| source_ids.contains(id));
    draft
        .inbox_source
        .as_ref()
        .is_some_and(|binding| binding.original.capture.id == item_id)
        || draft.group_id.is_some_and(|group| groups.contains(&group))
        || draft
            .sources
            .iter()
            .any(|source| source_paths.contains(&source.path))
        || draft.action_changes.iter().any(|change| {
            linked_data(change.data())
                || match change {
                    proposals::ActionChange::Replace { before, .. } => {
                        linked_data(&before.data) || linked_data(&before.origin.data)
                    }
                    proposals::ActionChange::Create { .. } => false,
                }
        })
}
impl WorkStore {
    /// One consistent checked Store snapshot. Includes all processing attempts,
    /// analyses (even before admission/after Session deletion), Source proposals,
    /// grouped consequences and manual work linked to known Source paths/UUIDs.
    /// Closed/rejected work remains visible. No state confers removal readiness.
    pub fn inbox_review_manifest(&self, item_id: Uuid) -> Result<InboxReviewManifest> {
        if item_id.is_nil() {
            return Err(invalid("Inbox review needs a nonnil original UUID"));
        }
        let tx = self.conn.unchecked_transaction()?;
        let original = inbox::read(&tx, item_id)?
            .ok_or_else(|| crate::Error::NotFound("Inbox original does not exist".into()))?;
        let mut manifest = InboxReviewManifest {
            original,
            processing: Vec::new(),
            analyses: Vec::new(),
            proposals: Vec::new(),
            approvals: Vec::new(),
            findings: Vec::new(),
        };
        let mut budget = 1024;
        append(&mut budget, &manifest.original)?;
        for id in ids(
            &tx,
            "SELECT id FROM inbox_processing ORDER BY queued_at_ms,id",
        )? {
            let batch = inbox_processing::read(&tx, id)?
                .ok_or_else(|| invalid("listed Inbox processing disappeared"))?;
            if batch
                .request
                .items
                .iter()
                .any(|item| item.capture.id == item_id)
            {
                if batch
                    .request
                    .items
                    .iter()
                    .any(|item| item.capture.id == item_id && *item != manifest.original)
                {
                    return Err(invalid(
                        "Inbox processing differs from the original review capture",
                    ));
                }
                append(&mut budget, &batch)?;
                manifest.processing.push(batch);
            }
        }
        let mut groups = HashSet::new();
        let mut source_ids = HashSet::new();
        let mut source_paths = HashSet::new();
        for id in ids(
            &tx,
            "SELECT id FROM inbox_actions ORDER BY created_at_ms,id",
        )? {
            let job = inbox_actions::read(&tx, id)?
                .ok_or_else(|| invalid("listed Inbox analysis disappeared"))?;
            let provenance = inbox_source::read_provenance(&job.capture.source_text)?
                .ok_or_else(|| invalid("Inbox analysis lost original provenance"))?;
            if provenance.item_id == item_id {
                let capture = &manifest.original.capture;
                let expected = inbox_source::InboxSourceProvenance {
                    item_id,
                    kind: capture.kind,
                    title: capture.title.clone(),
                    original_name: capture.original_name.clone(),
                    received_at_ms: manifest.original.received_at_ms,
                    original_byte_len: capture.copy.byte_len,
                    original_sha256: capture.copy.sha256,
                    format: provenance.format,
                };
                if provenance != expected {
                    return Err(invalid(
                        "Inbox analysis provenance differs from the original review capture",
                    ));
                }
                groups.insert(id);
                source_ids.insert(job.capture.note_id()?);
                source_paths.insert(job.capture.source.path.clone());
                let analysis = InboxAnalysisReview {
                    job,
                    turn: chat::read_turn(&tx, id)?.map(|(turn, _)| turn),
                };
                append(&mut budget, &analysis)?;
                manifest.analyses.push(analysis);
            }
        }
        // First discover every Source wrapper before selecting manual linked work.
        let proposal_ids = ids(&tx, "SELECT id FROM proposals ORDER BY id")?;
        for id in &proposal_ids {
            let proposal = proposals::read_proposal(&tx, *id)?
                .ok_or_else(|| invalid("listed Inbox review proposal disappeared"))?;
            if let Some(binding) = &proposal.record.draft.inbox_source
                && binding.original.capture.id == item_id
            {
                if binding.original != manifest.original {
                    return Err(invalid(
                        "Source proposal differs from the original review capture",
                    ));
                }
                source_ids.insert(binding.note_id);
                for change in &proposal.record.draft.changes {
                    source_paths.insert(change.path().to_owned());
                }
            }
        }
        // A settled attempt retains its exact approved draft even if a later
        // edit/rejection removes every mutable Source link. Discover historical
        // membership before selecting current reviews, then retain all attempts.
        let approval_ids = ids(
            &tx,
            "SELECT operation_id FROM proposal_applies ORDER BY operation_id",
        )?;
        let mut historical_proposals = HashSet::new();
        for id in &approval_ids {
            let journal = proposal_apply::read_journal(&tx, *id)?
                .ok_or_else(|| invalid("listed Inbox review approval disappeared"))?;
            if related(
                &journal.approved.draft,
                item_id,
                &groups,
                &source_ids,
                &source_paths,
            ) {
                historical_proposals.insert(journal.approved.draft.id);
            }
        }
        let mut retained_proposals = HashSet::new();
        for id in proposal_ids {
            let stored = proposals::read_proposal(&tx, id)?
                .ok_or_else(|| invalid("listed Inbox review proposal disappeared"))?;
            let draft = &stored.record.draft;
            let linked = related(draft, item_id, &groups, &source_ids, &source_paths)
                || historical_proposals.contains(&id);
            if linked {
                let proposal = InboxProposalReview {
                    creation_sha256: stored.creation_sha256,
                    record: stored.record,
                };
                append(&mut budget, &proposal)?;
                manifest.proposals.push(proposal);
                retained_proposals.insert(id);
            }
        }
        for id in approval_ids {
            let journal = proposal_apply::read_journal(&tx, id)?
                .ok_or_else(|| invalid("listed Inbox review approval disappeared"))?;
            if retained_proposals.contains(&journal.approved.draft.id) {
                append(&mut budget, &journal)?;
                manifest.approvals.push(journal);
            }
        }
        for id in ids(&tx, "SELECT id FROM findings ORDER BY created_at_ms,id")? {
            let finding = findings::read(&tx, id)?
                .ok_or_else(|| invalid("listed Inbox review finding disappeared"))?;
            let linked = matches!(&finding.draft.request.origin, FindingOrigin::InboxConflict { analysis_id, .. } if groups.contains(analysis_id))
                || finding.draft.evidence.iter().any(|proof| {
                    proof.note_id.is_some_and(|id| source_ids.contains(&id))
                        || source_paths.contains(&proof.source.path)
                });
            if linked {
                append(&mut budget, &finding)?;
                manifest.findings.push(finding);
            }
        }
        manifest.digest()?;
        tx.commit()?;
        Ok(manifest)
    }
}
