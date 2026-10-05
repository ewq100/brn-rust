//! Consistent complete operational evidence for original-copy qualification.
//! Reading this snapshot never attests semantic completeness or removes bytes.
use super::{
    WorkStore,
    action_completion::{self, ActionCompletion},
    actions::{self, ActionRecord},
    inbox_review::{self, InboxProposalReview, InboxReviewManifest},
    proposal_apply::{self, ApplyJournal},
    proposal_rewrite::{self, RewriteJob},
    proposals,
};
use crate::{Result, invalid};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxRemovalSnapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub original_operations: Vec<super::inbox_original_operations::InboxOriginalOperationSummary>,
    pub review: InboxReviewManifest,
    /// Current reviews reached through Action families or Undo lineage beyond
    /// the original direct-Source review. Preserve immutable creation proofs.
    pub related_reviews: Vec<InboxProposalReview>,
    pub actions: Vec<ActionRecord>,
    pub completions: Vec<ActionCompletion>,
    pub rewrites: Vec<RewriteJob>,
    /// Related approvals plus every transitively related Undo operation. A
    /// successful Undo is a consequence even when its proposal has no sources.
    pub lineage: Vec<ApplyJournal>,
}
impl InboxRemovalSnapshot {
    pub fn digest(&self) -> Result<[u8; 32]> {
        inbox_review::digest(self)
    }
}
impl WorkStore {
    pub fn inbox_removal_snapshot(&self, item_id: Uuid) -> Result<InboxRemovalSnapshot> {
        let tx = self.conn.unchecked_transaction()?;
        let snapshot = read(&tx, item_id)?;
        tx.commit()?;
        Ok(snapshot)
    }
}
pub(super) fn read(conn: &rusqlite::Connection, item_id: Uuid) -> Result<InboxRemovalSnapshot> {
    let review = inbox_review::read(conn, item_id)?;
    let proposal_ids: HashSet<_> = review.proposals.iter().map(|p| p.record.draft.id).collect();
    let source_ids: HashSet<_> = review
        .proposals
        .iter()
        .filter_map(|p| p.record.draft.inbox_source.as_ref().map(|b| b.note_id))
        .chain(
            review
                .approvals
                .iter()
                .filter_map(|a| a.approved.draft.inbox_source.as_ref().map(|b| b.note_id)),
        )
        .chain(
            review
                .analyses
                .iter()
                .map(|a| a.job.capture.note_id())
                .collect::<Result<Vec<_>>>()?,
        )
        .collect();
    // A small identity graph closes complete atomic proposal families over
    // current/historical Action members and Undo lineage. Read checked rows
    // one at a time; retain only UUID edges during discovery, not unrelated
    // full bodies. This also avoids repeatedly rereading long Undo chains.
    let mut graph = HashMap::new();
    let mut members: HashSet<Node> = proposal_ids
        .iter()
        .map(|id| Node::Proposal(*id))
        .chain(
            review
                .approvals
                .iter()
                .map(|a| Node::Operation(a.request.operation_id)),
        )
        .collect();
    let all_proposals = inbox_review::ids(conn, "SELECT id FROM proposals ORDER BY id")?;
    for id in &all_proposals {
        let p = proposals::read_proposal(conn, *id)?
            .ok_or_else(|| invalid("listed proposal disappeared"))?;
        for change in &p.record.draft.action_changes {
            edge(&mut graph, Node::Proposal(*id), Node::Action(change.id()));
        }
    }
    let approval_ids = inbox_review::ids(
        conn,
        "SELECT operation_id FROM proposal_applies ORDER BY operation_id",
    )?;
    for id in &approval_ids {
        let a = proposal_apply::read_journal(conn, *id)?
            .ok_or_else(|| invalid("listed approval disappeared"))?;
        edge(
            &mut graph,
            Node::Operation(*id),
            Node::Proposal(a.approved.draft.id),
        );
        if let Some(undo) = &a.undo {
            edge(
                &mut graph,
                Node::Operation(*id),
                Node::Operation(undo.operation_id),
            );
        }
        for change in &a.approved.draft.action_changes {
            edge(&mut graph, Node::Operation(*id), Node::Action(change.id()));
        }
    }
    let all_actions = inbox_review::ids(conn, "SELECT id FROM actions ORDER BY created_at_ms,id")?;
    for id in &all_actions {
        let a = actions::read(conn, *id)?.ok_or_else(|| invalid("listed Action disappeared"))?;
        edge(
            &mut graph,
            Node::Action(*id),
            Node::Proposal(a.origin.proposal.id),
        );
        if a.data
            .sources
            .iter()
            .chain(&a.origin.data.sources)
            .any(|id| source_ids.contains(id))
        {
            members.insert(Node::Action(*id));
        }
    }
    let mut pending: Vec<_> = members.iter().copied().collect();
    while let Some(node) = pending.pop() {
        if let Some(neighbors) = graph.get(&node) {
            for neighbor in neighbors {
                if members.insert(*neighbor) {
                    pending.push(*neighbor);
                }
            }
        }
    }
    let present_proposals: HashSet<_> = all_proposals.iter().copied().collect();
    if members
        .iter()
        .any(|node| matches!(node,Node::Proposal(id) if !present_proposals.contains(id)))
    {
        return Err(invalid("related proposal authority is missing"));
    }
    let mut budget = 1024;
    inbox_review::append(&mut budget, &review)?;
    let mut related_reviews = Vec::new();
    for id in &all_proposals {
        if members.contains(&Node::Proposal(*id)) && !proposal_ids.contains(id) {
            let p = proposals::read_proposal(conn, *id)?
                .ok_or_else(|| invalid("related proposal disappeared"))?;
            let p = InboxProposalReview {
                creation_sha256: p.creation_sha256,
                record: p.record,
            };
            inbox_review::append(&mut budget, &p)?;
            related_reviews.push(p);
        }
    }
    let mut lineage = Vec::new();
    for id in approval_ids {
        if members.contains(&Node::Operation(id)) {
            let a = proposal_apply::read_journal(conn, id)?
                .ok_or_else(|| invalid("related approval disappeared"))?;
            inbox_review::append(&mut budget, &a)?;
            lineage.push(a);
        }
    }
    let mut actions = Vec::new();
    for id in all_actions {
        if members.contains(&Node::Action(id)) {
            let a =
                actions::read(conn, id)?.ok_or_else(|| invalid("related Action disappeared"))?;
            inbox_review::append(&mut budget, &a)?;
            actions.push(a);
        }
    }
    let current: HashMap<_, _> = actions.iter().map(|a| (a.origin.id, a)).collect();
    // Enumeration alone cannot prove that established durable authority still
    // exists. Require each known Applied Action baseline or its exact-origin
    // later revision, including when the Action row disappeared after open.
    for a in &lineage {
        if a.receipt.as_ref().map(|r| r.outcome) == Some(proposal_apply::ApplyOutcome::Applied) {
            for expected in &a.action_records {
                let retained = current
                    .get(&expected.origin.id)
                    .ok_or_else(|| invalid("known Applied Action authority is missing"))?;
                if retained.origin != expected.origin
                    || retained.version < expected.version
                    || retained.version == expected.version && **retained != *expected
                {
                    return Err(invalid(
                        "known Applied Action authority differs from its exact origin/revision",
                    ));
                }
            }
        }
    }
    let mut completions = Vec::new();
    let mut completed_ids = HashSet::new();
    for id in inbox_review::ids(
        conn,
        "SELECT operation_id FROM action_completions ORDER BY operation_id",
    )? {
        let c = action_completion::read(conn, id)?
            .ok_or_else(|| invalid("listed Action completion disappeared"))?;
        if members.contains(&Node::Action(c.after.origin.id)) {
            action_completion::require_retained_completion(conn, &c)?;
            inbox_review::append(&mut budget, &c)?;
            completed_ids.insert(c.after.origin.id);
            completions.push(c);
        }
    }
    for a in &actions {
        if a.data.state == actions::ActionState::Completed && !completed_ids.contains(&a.origin.id)
        {
            return Err(invalid(
                "Completed Action authority has no retained exact completion",
            ));
        }
    }
    let mut rewrites = Vec::new();
    for id in inbox_review::ids(conn, "SELECT id FROM proposal_rewrites ORDER BY id")? {
        let job = proposal_rewrite::read_job(conn, id)?
            .ok_or_else(|| invalid("listed Rewrite disappeared"))?;
        if members.contains(&Node::Proposal(job.spec.expected.id)) {
            inbox_review::append(&mut budget, &job)?;
            rewrites.push(job);
        }
    }
    let original_operations = super::inbox_original_operations::history(conn, item_id)?;
    inbox_review::append(&mut budget, &original_operations)?;
    let snapshot = InboxRemovalSnapshot {
        original_operations,
        review,
        related_reviews,
        actions,
        completions,
        rewrites,
        lineage,
    };
    snapshot.digest()?;
    Ok(snapshot)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Node {
    Proposal(Uuid),
    Action(Uuid),
    Operation(Uuid),
}
fn edge(graph: &mut HashMap<Node, Vec<Node>>, a: Node, b: Node) {
    graph.entry(a).or_default().push(b);
    graph.entry(b).or_default().push(a);
}
