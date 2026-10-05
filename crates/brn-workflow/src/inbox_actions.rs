//! One captured Inbox Source analyzed on the existing owned chat lane.
//! Action review work is separate from approval and semantic completeness.
use crate::{
    ErrorKind, ReasoningEffort, Result, Selection, WorkTurn, WorkflowError,
    app::App,
    chat_worker::AskRequest,
    proposals::{ProposalRecord, ProposalSource},
};
pub use brn_store::work::inbox_actions::{
    InboxActionCapture, InboxActionJob, InboxAnalysisPurpose, InboxKnowledgeBinding,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod knowledge;
pub(crate) use knowledge::validate_supersession_link;

pub const MAX_INBOX_ACTION_PROPOSALS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxActionRequest {
    #[serde(default)]
    pub purpose: InboxAnalysisPurpose,
    pub id: Uuid,
    pub conversation: Option<Uuid>,
    pub source: Box<ProposalSource>,
    pub selection: Selection,
    pub effort: ReasoningEffort,
    pub generation: u64,
}
impl InboxActionRequest {
    pub fn validate(&self) -> Result<()> {
        self.source.validate()?;
        self.selection.validate()?;
        self.capture().validate()?;
        Ok(())
    }
    pub(crate) fn capture(&self) -> InboxActionCapture {
        InboxActionCapture {
            purpose: self.purpose,
            id: self.id,
            conversation: self.conversation,
            source: self.source.source.clone(),
            source_text: self.source.text.clone(),
            provider: crate::chat_worker::provider_name(self.selection.provider).into(),
            model: self.selection.model.clone(),
            effort: self.effort.as_str().into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxActionAnalysis {
    pub job: InboxActionJob,
    pub turn: Option<WorkTurn>,
    pub proposals: Vec<ProposalRecord>,
    pub findings: Vec<crate::findings::FindingRecord>,
    /// Action analysis does not establish complete semantic ingestion/deletion.
    pub needs_semantic_review: bool,
}

fn conflict() -> WorkflowError {
    WorkflowError::typed(
        ErrorKind::OperationConflict,
        "Inbox analysis UUID reused with different captured input",
    )
}

impl App {
    /// Pure preparation/replay; new admission qualifies the saved Source afresh.
    pub(crate) fn prepare_inbox_action_request(
        &self,
        request: &InboxActionRequest,
    ) -> Result<(AskRequest, InboxActionCapture)> {
        request.validate()?;
        let capture = request.capture();
        let question = match self.store.inbox_action(request.id)? {
            Some(job) if job.capture == capture => job.question,
            Some(_) => return Err(conflict()),
            None => crate::ai_behavior::TaskInput::Inbox(&capture).prompt()?,
        };
        Ok((
            AskRequest {
                id: request.id,
                conversation: request.conversation,
                question,
                selection: request.selection.clone(),
                effort: Some(request.effort),
                generation: request.generation,
            },
            capture,
        ))
    }

    pub(crate) fn validate_inbox_action_source(
        &mut self,
        capture: &InboxActionCapture,
    ) -> Result<()> {
        capture.validate()?;
        let fresh = self.proposal_evidence_source(&capture.source.path)?;
        if fresh.source != capture.source || fresh.text != capture.source_text {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "selected Inbox Source changed",
            ));
        }
        let resolution = self.resolve_note_identity(capture.note_id()?)?;
        if resolution.outcome != crate::knowledge::IdentityOutcome::Unique
            || resolution.matches.len() != 1
            || resolution.matches[0].path != capture.source.path
        {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "selected Inbox Source identity is ambiguous or incompletely inspected",
            ));
        }
        Ok(())
    }

    pub fn inbox_action_analysis(&self, id: Uuid) -> Result<InboxActionAnalysis> {
        let job = self.store.inbox_action(id)?.ok_or_else(|| {
            WorkflowError::typed(ErrorKind::NotFound, "Inbox Action analysis does not exist")
        })?;
        Ok(InboxActionAnalysis {
            job,
            turn: self.store.turn(id)?,
            proposals: self.proposals(Some(id))?,
            findings: self.store.inbox_conflicts(id)?,
            needs_semantic_review: true,
        })
    }
}
