//! Exact transient approval confirmation; AppWorker owns application and recovery.

use brn_workflow::{
    app_worker::AppCommand,
    proposal_apply::{
        ApplyOutcome, ApplyReceipt, ApprovalRequest, GroupApprovalRequest, GroupApprovalResult,
        RepairDirection, RepairPreview, RepairReceipt, RepairRequest, UndoPreview, UndoRequest,
        validate_approval_request, validate_undo_request,
    },
    proposals::{
        ActionChange, MAX_PROPOSAL_CHANGES, NoteChange, ProposalEdit, ProposalRecord,
        ProposalStamp, ProposalState, validate_review_edit,
    },
};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalCapture {
    records: Vec<ProposalRecord>,
    requests: Vec<ApprovalRequest>,
    group_id: Option<Uuid>,
}

impl ApprovalCapture {
    pub fn new(mut records: Vec<ProposalRecord>, group_id: Option<Uuid>) -> Option<Self> {
        if records.is_empty()
            || records.len() > MAX_PROPOSAL_CHANGES
            || group_id.is_none() && records.len() != 1
            || group_id.is_some_and(|id| id.is_nil())
        {
            return None;
        }
        let bound_vault = records
            .iter()
            .find_map(|record| record.draft.vault.as_ref());
        let mut proposal_ids = HashSet::new();
        for record in &records {
            if record.state != ProposalState::Draft
                || !proposal_ids.insert(record.draft.id)
                || group_id.is_some_and(|id| {
                    record.draft.group_id != Some(id)
                        && (!record
                            .draft
                            .inbox_source
                            .as_ref()
                            .is_some_and(|binding| binding.extraction.is_some())
                            || !records
                                .iter()
                                .filter_map(intake_dependency)
                                .any(|binding| binding.source_proposal == record.stamp()))
                })
                || record
                    .draft
                    .vault
                    .as_ref()
                    .is_some_and(|vault| Some(vault) != bound_vault)
            {
                return None;
            }
            let edit = ProposalEdit {
                action_data: crate::review::record_actions(record),
                expected: record.stamp(),
                title: record.draft.title.clone(),
                texts: record
                    .draft
                    .changes
                    .iter()
                    .map(|change| change.text().map(str::to_owned))
                    .collect(),
            };
            validate_review_edit(record, &edit).ok()?;
        }
        // Match workflow's stable Source-first execution before binding operations.
        records.sort_by_key(|record| record.draft.inbox_source.is_none());
        let mut operations = HashSet::new();
        let mut requests = Vec::with_capacity(records.len());
        for record in &records {
            let operation_id = loop {
                let id = Uuid::new_v4();
                if !id.is_nil() && operations.insert(id) {
                    break id;
                }
            };
            let request = ApprovalRequest {
                operation_id,
                expected: record.stamp(),
            };
            validate_approval_request(&request).ok()?;
            requests.push(request);
        }
        Some(Self {
            records,
            requests,
            group_id,
        })
    }

    pub fn move_earlier(&self, id: Uuid) -> Option<Self> {
        let index = self
            .records
            .iter()
            .position(|record| record.draft.id == id)?;
        self.swap_non_sources(index, index.checked_sub(1)?)
    }

    pub fn move_later(&self, id: Uuid) -> Option<Self> {
        let index = self
            .records
            .iter()
            .position(|record| record.draft.id == id)?;
        self.swap_non_sources(index, index.checked_add(1)?)
    }

    fn swap_non_sources(&self, index: usize, adjacent: usize) -> Option<Self> {
        if self.group_id.is_none()
            || self.records.get(index)?.draft.inbox_source.is_some()
            || self.records.get(adjacent)?.draft.inbox_source.is_some()
        {
            return None;
        }
        let mut ordered = self.clone();
        ordered.records.swap(index, adjacent);
        ordered.requests.swap(index, adjacent);
        Some(ordered)
    }

    pub fn select(&self, selected: &HashSet<Uuid>) -> Option<Self> {
        if self.group_id.is_none() || selected.is_empty() {
            return None;
        }
        let pairs: Vec<_> = self
            .records
            .iter()
            .zip(&self.requests)
            .filter(|(r, _)| selected.contains(&r.draft.id))
            .collect();
        if pairs.len() != selected.len() {
            return None;
        }
        for (record, _) in &pairs {
            if let Some(binding) = intake_dependency(record) {
                let source_selected = pairs
                    .iter()
                    .any(|(source, _)| source.stamp() == binding.source_proposal);
                let source_was_pending = self
                    .records
                    .iter()
                    .any(|source| source.stamp() == binding.source_proposal);
                if source_was_pending && !source_selected {
                    return None;
                }
            }
        }
        Some(Self {
            records: pairs.iter().map(|(r, _)| (*r).clone()).collect(),
            requests: pairs.iter().map(|(_, r)| (*r).clone()).collect(),
            group_id: self.group_id,
        })
    }
    pub fn records(&self) -> &[ProposalRecord] {
        &self.records
    }

    pub fn requests(&self) -> &[ApprovalRequest] {
        &self.requests
    }

    pub fn group_id(&self) -> Option<Uuid> {
        self.group_id
    }

    pub fn command(&self) -> AppCommand {
        match self.group_id {
            None => AppCommand::ApproveProposal(self.requests[0].clone()),
            Some(group_id) => AppCommand::ApproveProposalGroup(GroupApprovalRequest {
                group_id,
                approvals: self.requests.clone(),
            }),
        }
    }

    pub fn accepts_receipt(&self, index: usize, receipt: &ApplyReceipt) -> bool {
        self.requests
            .get(index)
            .is_some_and(|request| receipt_matches(request, receipt))
    }

    pub fn accepts_group(&self, result: &GroupApprovalResult) -> bool {
        if self.group_id.is_none()
            || result.receipts.len() > self.requests.len()
            || result
                .receipts
                .iter()
                .enumerate()
                .any(|(index, receipt)| !self.accepts_receipt(index, receipt))
        {
            return false;
        }
        let Some(stopped) = &result.stopped else {
            return result.receipts.len() == self.requests.len()
                && result
                    .receipts
                    .iter()
                    .all(|receipt| receipt.outcome == ApplyOutcome::Applied);
        };
        let Some(stop_index) = self
            .requests
            .iter()
            .position(|request| request.operation_id == stopped.operation_id)
        else {
            return false;
        };
        let Some((last, earlier)) = result.receipts.split_last() else {
            return stop_index == 0;
        };
        if earlier
            .iter()
            .any(|receipt| receipt.outcome != ApplyOutcome::Applied)
        {
            return false;
        }
        // A failure can follow a recorded receipt, including Applied, or occur
        // before the next member is admitted. No later member may have run.
        stop_index == result.receipts.len() - 1
            || last.outcome == ApplyOutcome::Applied && stop_index == result.receipts.len()
    }
}

/// One exact historical inverse confirmation. Workflow owns current eligibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoCapture {
    request: UndoRequest,
    preview: UndoPreview,
}

impl UndoCapture {
    pub fn new(request: UndoRequest, preview: UndoPreview) -> Option<Self> {
        validate_undo_request(&request).ok()?;
        let members = preview
            .draft
            .changes
            .len()
            .checked_add(preview.draft.action_changes.len())?;
        if preview.draft.id != request.operation_id
            || preview.binding.operation_id != request.target_operation_id
            || preview.binding.trash_member != request.trash_member
            || preview.draft.group_id.is_some()
            || members == 0
            || members > MAX_PROPOSAL_CHANGES
            || preview.binding.originals.len() != preview.draft.changes.len()
        {
            return None;
        }
        if !preview.draft.action_changes.is_empty()
            && (!preview.draft.changes.is_empty()
                || request.trash_member.is_some()
                || preview.draft.action_changes.iter().any(|change| {
                    !matches!(change, ActionChange::Replace { .. }) || change.validate().is_err()
                }))
        {
            return None;
        }
        if request.trash_member.is_some()
            && (preview.draft.changes.len() != 1
                || !matches!(
                    preview.draft.changes[0],
                    NoteChange::Create { .. } | NoteChange::CreateAsset { .. }
                )
                || preview.binding.originals[0].is_none())
        {
            return None;
        }
        Some(Self { request, preview })
    }

    pub fn request(&self) -> &UndoRequest {
        &self.request
    }

    pub fn preview(&self) -> &UndoPreview {
        &self.preview
    }

    pub fn command(&self) -> AppCommand {
        AppCommand::UndoProposal(self.request.clone())
    }

    pub fn accepts_receipt(&self, receipt: &ApplyReceipt) -> bool {
        // Undo admits its immutable inverse at version one; a replay preserves
        // that admission even after later review edits or reconciliation.
        receipt_matches(
            &ApprovalRequest {
                operation_id: self.request.operation_id,
                expected: ProposalStamp {
                    id: self.request.operation_id,
                    version: 1,
                },
            },
            receipt,
        )
    }
}

/// One explicit Finish or Restore attempt with the exact observed capture hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairCapture {
    request: RepairRequest,
    preview: RepairPreview,
}

impl RepairCapture {
    pub fn new(preview: RepairPreview, direction: RepairDirection) -> Option<Self> {
        if preview.operation_id.is_nil()
            || preview.approved.changes.is_empty()
            || preview.approved.changes.len() > MAX_PROPOSAL_CHANGES
            || preview.phases.len() != preview.approved.changes.len()
        {
            return None;
        }
        let id = loop {
            let id = Uuid::new_v4();
            if !id.is_nil() && id != preview.operation_id {
                break id;
            }
        };
        let request = RepairRequest {
            id,
            operation_id: preview.operation_id,
            expected: preview.expected,
            direction,
        };
        Some(Self { request, preview })
    }

    pub fn request(&self) -> &RepairRequest {
        &self.request
    }

    pub fn preview(&self) -> &RepairPreview {
        &self.preview
    }

    pub fn command(&self) -> AppCommand {
        AppCommand::RepairProposal(self.request.clone())
    }

    pub fn accepts_receipt(&self, receipt: &RepairReceipt) -> bool {
        receipt.id == self.request.id
            && receipt.operation_id == self.request.operation_id
            && receipt.direction == self.request.direction
    }
}

pub fn receipt_matches(request: &ApprovalRequest, receipt: &ApplyReceipt) -> bool {
    request
        .expected
        .version
        .checked_add(2)
        .is_some_and(|minimum| {
            receipt.operation_id == request.operation_id
                && receipt.proposal_id == request.expected.id
                && receipt.approved_version == request.expected.version
                && receipt.stamp.id == request.expected.id
                && receipt.stamp.version >= minimum
        })
}

#[cfg(test)]
#[path = "approval_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "approval_order_tests.rs"]
pub(crate) mod ordering_tests;

#[cfg(test)]
#[path = "approval_operation_tests.rs"]
mod operation_tests;

pub(crate) fn intake_dependency(
    record: &ProposalRecord,
) -> Option<&brn_workflow::inbox_actions::InboxIntakeBinding> {
    record.draft.intake.as_deref().or_else(|| {
        record
            .draft
            .inbox_knowledge
            .as_ref()
            .and_then(|b| b.intake.as_deref())
    })
}
