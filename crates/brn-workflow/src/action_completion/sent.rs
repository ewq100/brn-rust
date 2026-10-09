//! Explicit owner attestation of an already-retained actual sent Text Source.
use super::{CompleteActionRequest, SentSourceBinding};
use crate::{
    ErrorKind, Result, WorkflowError,
    actions::{ActionRecord, ActionState},
    app::App,
    knowledge::IdentityOutcome,
    library::saved_metadata,
    proposals::ProposalSource,
    vault::EvidencePath,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn rejected(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, message)
}
fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareSentCompletionRequest {
    pub before: Box<ActionRecord>,
    pub source_path: String,
}
impl PrepareSentCompletionRequest {
    /// Pure client preflight; this selects no Source authority or completion.
    pub fn validate(&self) -> Result<()> {
        self.before
            .validate()
            .map_err(|error| rejected(&error.to_string()))?;
        if self.before.data.state == ActionState::Completed
            || self.before.data.thread.is_none()
            || !(1..=512).contains(&self.source_path.len())
        {
            return Err(rejected(
                "sent completion needs an unfinished Action with an explicit thread and bounded Source path",
            ));
        }
        EvidencePath::parse(&self.source_path)
            .map_err(|_| rejected("sent Source needs a contained visible Markdown path"))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SentCompletionPreview {
    pub request: CompleteActionRequest,
    pub source: ProposalSource,
    pub title: String,
}
impl SentCompletionPreview {
    /// Validate a complete host response without observing files or granting effects.
    pub fn validate_for(&self, expected: &PrepareSentCompletionRequest) -> Result<()> {
        expected.validate()?;
        self.request.validate()?;
        self.source.validate()?;
        let binding = self
            .request
            .sent_source
            .as_ref()
            .ok_or_else(|| rejected("sent preview has no exact Source binding"))?;
        let metadata = saved_metadata(&self.source.text, &self.source.source.path);
        if self.request.before != expected.before
            || binding.source.path != expected.source_path
            || binding.source != self.source.source
            || self.title.trim().is_empty()
            || self.title.len() > 512
            || metadata.issue.is_some()
            || !metadata.source
            || metadata.note_id != Some(binding.note_id)
            || brn_store::work::inbox_source::read_provenance(&self.source.text).is_err()
        {
            return Err(rejected(
                "sent preview differs from its exact Action and saved Source selection",
            ));
        }
        Ok(())
    }
}

impl App {
    /// Capture an exact review; Source capture/approval happened separately already.
    pub fn prepare_sent_action_completion(
        &mut self,
        request: &PrepareSentCompletionRequest,
    ) -> Result<SentCompletionPreview> {
        request.validate()?;
        self.require_current_evidence()?;
        if self.action(request.before.origin.id)? != *request.before {
            return Err(stale("Action changed before sent-evidence review"));
        }
        let observed = self.proposal_evidence_source(&request.source_path)?;
        let binding = self.store.sent_source_binding(&observed.source)?;
        let source = self.validate_sent_action_source(&binding)?;
        if source != observed {
            return Err(stale("sent Source changed during review preparation"));
        }
        let title = self
            .store
            .proposal(binding.source_approval.expected.id)?
            .ok_or_else(|| stale("sent Source proposal is unavailable"))?
            .draft
            .title;
        let preview = SentCompletionPreview {
            request: CompleteActionRequest {
                operation_id: Uuid::new_v4(),
                before: request.before.clone(),
                sent_source: Some(binding),
            },
            source,
            title,
        };
        preview.validate_for(request)?;
        self.require_current_evidence()?;
        Ok(preview)
    }

    /// Fresh admission checks the exact saved version. Historical replay uses its receipt.
    pub(super) fn validate_sent_action_source(
        &mut self,
        binding: &SentSourceBinding,
    ) -> Result<ProposalSource> {
        binding.validate()?;
        self.require_current_evidence()?;
        self.store.validate_sent_source_binding(binding)?;
        let record = self
            .store
            .proposal(binding.source_approval.expected.id)?
            .ok_or_else(|| stale("sent Source approval is unavailable"))?;
        self.validate_inbox_source(record.draft.inbox_source.as_deref())?;
        let source = self.proposal_evidence_source(&binding.source.path)?;
        if source.source != binding.source {
            return Err(stale("sent Source changed from the displayed full version"));
        }
        let resolution = self.resolve_note_identity(binding.note_id)?;
        if resolution.outcome != IdentityOutcome::Unique
            || resolution.matches.len() != 1
            || resolution.matches[0].path != binding.source.path
        {
            return Err(stale(
                "sent Source identity is missing, ambiguous or incompletely inspected",
            ));
        }
        // Reuse the complete physical proof after inventory observation, not size/mtime.
        let confirmed = self.proposal_evidence_source(&binding.source.path)?;
        if confirmed != source {
            return Err(stale("sent Source changed during completion validation"));
        }
        self.require_current_evidence()?;
        Ok(confirmed)
    }
}
