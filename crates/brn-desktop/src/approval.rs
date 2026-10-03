//! Exact transient approval confirmation; AppWorker owns application and recovery.

use brn_workflow::{
    app_worker::AppCommand,
    proposal_apply::{
        ApplyOutcome, ApplyReceipt, ApprovalRequest, GroupApprovalRequest, GroupApprovalResult,
        validate_approval_request,
    },
    proposals::{
        MAX_PROPOSAL_CHANGES, ProposalEdit, ProposalRecord, ProposalState, validate_review_edit,
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
    pub fn new(records: Vec<ProposalRecord>, group_id: Option<Uuid>) -> Option<Self> {
        if records.is_empty()
            || records.len() > MAX_PROPOSAL_CHANGES
            || group_id.is_none() && records.len() != 1
            || group_id.is_some_and(|id| id.is_nil())
        {
            return None;
        }
        let mut proposal_ids = HashSet::new();
        for record in &records {
            if record.state != ProposalState::Draft
                || !proposal_ids.insert(record.draft.id)
                || group_id.is_some_and(|id| record.draft.group_id != Some(id))
                || record.draft.vault != records[0].draft.vault
            {
                return None;
            }
            let edit = ProposalEdit {
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
