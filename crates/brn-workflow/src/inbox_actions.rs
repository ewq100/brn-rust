//! One captured Inbox Source analyzed on the existing owned chat lane.
//! Action review work is separate from approval and semantic completeness.
use crate::proposals::SourceVersion;
use crate::{
    ErrorKind, ReasoningEffort, Result, Selection, WorkTurn, WorkflowError,
    app::App,
    chat_worker::AskRequest,
    proposals::{ProposalRecord, ProposalSource},
};
pub use brn_store::work::inbox_actions::{
    InboxActionCapture, InboxActionJob, InboxAnalysisPurpose, InboxIntakeBinding,
    InboxKnowledgeBinding,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod knowledge;
mod visual;
pub(crate) use knowledge::validate_supersession_link;
pub use visual::InboxVisualEvidence;

pub const MAX_INBOX_ACTION_PROPOSALS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxActionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_asset: Option<SourceVersion>,
    #[serde(default)]
    pub purpose: InboxAnalysisPurpose,
    pub id: Uuid,
    pub conversation: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Box<ProposalSource>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intake: Option<InboxIntakeBinding>,
    pub selection: Selection,
    pub effort: ReasoningEffort,
    pub generation: u64,
}
impl InboxActionRequest {
    pub fn validate(&self) -> Result<()> {
        match (&self.source, &self.intake) {
            (Some(source), None) => {
                source.validate()?;
                self.capture().validate()?;
            }
            (None, Some(intake)) => {
                intake.validate()?;
                if self.id.is_nil()
                    || self.conversation.is_some_and(|id| id.is_nil())
                    || self.visual_asset.is_some()
                    || self.purpose == InboxAnalysisPurpose::VisualInterpretation
                {
                    return Err(WorkflowError::typed(
                        ErrorKind::ToolRejected,
                        "invalid private intake analysis request",
                    ));
                }
            }
            _ => {
                return Err(WorkflowError::typed(
                    ErrorKind::ToolRejected,
                    "analysis needs exactly one saved Source or private intake binding",
                ));
            }
        }
        self.selection.validate()?;
        Ok(())
    }
    pub(crate) fn capture(&self) -> InboxActionCapture {
        InboxActionCapture {
            visual_asset: self.visual_asset.clone(),
            purpose: self.purpose,
            id: self.id,
            conversation: self.conversation,
            source: self.source.as_ref().map(|source| source.source.clone()),
            intake: self.intake.clone(),
            source_text: self
                .source
                .as_ref()
                .map_or_else(String::new, |source| source.text.clone()),
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
        let mut capture = request.capture();
        if let Some(intake) = &capture.intake {
            if let Some(job) = self.store.inbox_action(request.id)? {
                capture.source_text = job.capture.source_text;
            } else {
                let source = self
                    .store
                    .proposal(intake.source_proposal.id)?
                    .ok_or_else(|| {
                        WorkflowError::typed(
                            ErrorKind::NotFound,
                            "planned Source proposal is unavailable",
                        )
                    })?;
                capture.source_text = source
                    .draft
                    .changes
                    .first()
                    .and_then(|change| change.text())
                    .ok_or_else(|| {
                        WorkflowError::typed(
                            ErrorKind::ToolRejected,
                            "planned Source text is unavailable",
                        )
                    })?
                    .to_owned();
            }
        }
        capture.validate()?;
        let question = match self.store.inbox_action(request.id)? {
            Some(job) if job.capture == capture => job.question,
            Some(_) => return Err(conflict()),
            None => {
                if let Some(intake) = &capture.intake {
                    let snapshot =
                        self.store
                            .intake_snapshot(intake.snapshot_id)?
                            .ok_or_else(|| {
                                WorkflowError::typed(
                                    ErrorKind::ContextStale,
                                    "private extraction is unavailable",
                                )
                            })?;
                    let source =
                        self.store
                            .proposal(intake.source_proposal.id)?
                            .ok_or_else(|| {
                                WorkflowError::typed(
                                    ErrorKind::ContextStale,
                                    "bound Source is unavailable",
                                )
                            })?;
                    crate::ai_behavior::TaskInput::Intake(
                        &capture,
                        &snapshot,
                        source.state == crate::proposals::ProposalState::Applied,
                    )
                    .prompt()?
                } else {
                    crate::ai_behavior::TaskInput::Inbox(&capture).prompt()?
                }
            }
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
        if let Some(intake) = &capture.intake {
            // A retained approved Source can be investigated immediately after
            // restart, before any editor or proposal-preparation view opened.
            self.editor_files()?;
            self.validate_intake_dependency(Some(intake), false)?;
            let snapshot = self
                .store
                .intake_snapshot(intake.snapshot_id)?
                .ok_or_else(|| {
                    WorkflowError::typed(
                        ErrorKind::ContextStale,
                        "private extraction is unavailable",
                    )
                })?;
            intake.validate_snapshot(&snapshot)?;
            let source = self
                .store
                .proposal(intake.source_proposal.id)?
                .ok_or_else(|| {
                    WorkflowError::typed(ErrorKind::ContextStale, "planned Source is unavailable")
                })?;
            self.validate_inbox_source(source.draft.inbox_source.as_deref())?;
            return Ok(());
        }
        let source = capture.source.as_ref().ok_or_else(|| {
            WorkflowError::typed(ErrorKind::ToolRejected, "saved Source proof is unavailable")
        })?;
        let fresh = self.proposal_evidence_source(&source.path)?;
        if fresh.source != *source || fresh.text != capture.source_text {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "selected Inbox Source changed",
            ));
        }
        let resolution = self.resolve_note_identity(capture.note_id()?)?;
        if resolution.outcome != crate::knowledge::IdentityOutcome::Unique
            || resolution.matches.len() != 1
            || resolution.matches[0].path != source.path
        {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "selected Inbox Source identity is ambiguous or incompletely inspected",
            ));
        }
        Ok(())
    }

    /// Mint a retained collection binding from a Draft or exact Applied Source.
    /// Callers may explicitly select a smaller image/occurrence collection later.
    pub fn intake_analysis_binding(&self, source_proposal_id: Uuid) -> Result<InboxIntakeBinding> {
        let source = self.store.proposal(source_proposal_id)?.ok_or_else(|| {
            WorkflowError::typed(ErrorKind::NotFound, "Source proposal is unavailable")
        })?;
        let source_stamp = match source.state {
            crate::proposals::ProposalState::Draft => source.stamp(),
            crate::proposals::ProposalState::Applied => {
                let mut approved_stamp = None;
                for id in self.store.proposal_apply_ids()? {
                    let journal = self.store.proposal_apply(id)?.ok_or_else(|| {
                        WorkflowError::typed(
                            ErrorKind::ContextStale,
                            "listed Source approval is unavailable",
                        )
                    })?;
                    if journal.approved.draft == source.draft
                        && journal.receipt.as_ref().is_some_and(|receipt| {
                            receipt.outcome == crate::proposal_apply::ApplyOutcome::Applied
                                && receipt.stamp == source.stamp()
                        })
                        && approved_stamp.replace(journal.approved.stamp()).is_some()
                    {
                        return Err(WorkflowError::typed(
                            ErrorKind::ContextStale,
                            "Source approval receipt is ambiguous",
                        ));
                    }
                }
                approved_stamp.ok_or_else(|| {
                    WorkflowError::typed(
                        ErrorKind::ContextStale,
                        "Source has no exact current Applied receipt",
                    )
                })?
            }
            _ => {
                return Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "retained investigation needs a Draft or exact Applied Source",
                ));
            }
        };
        let extraction = source
            .draft
            .inbox_source
            .as_deref()
            .and_then(|binding| binding.extraction.as_ref())
            .ok_or_else(|| {
                WorkflowError::typed(ErrorKind::ToolRejected, "Source has no retained extraction")
            })?;
        let snapshot = self
            .store
            .intake_snapshot(extraction.snapshot_id)?
            .ok_or_else(|| {
                WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "retained extraction is unavailable",
                )
            })?;
        let change = source.draft.changes.first().ok_or_else(|| {
            WorkflowError::typed(
                ErrorKind::ToolRejected,
                "planned Source member is unavailable",
            )
        })?;
        let text = change.text().ok_or_else(|| {
            WorkflowError::typed(
                ErrorKind::ToolRejected,
                "planned Source text is unavailable",
            )
        })?;
        let assets = snapshot
            .extraction
            .assets
            .iter()
            .filter(|asset| asset.media_type == "image/png")
            .map(|asset| asset.id.clone())
            .collect::<Vec<_>>();
        let binding = InboxIntakeBinding {
            snapshot_id: snapshot.id,
            snapshot_sha256: snapshot.digest()?,
            source_proposal: source_stamp,
            source_path: change.path().to_owned(),
            source_note_id: brn_store::note_identity::read(text)?.ok_or_else(|| {
                WorkflowError::typed(
                    ErrorKind::ToolRejected,
                    "planned Source identity is unavailable",
                )
            })?,
            source_text_sha256: brn_intake::digest(text.as_bytes()),
            occurrences: snapshot
                .extraction
                .occurrences
                .iter()
                .filter(|occurrence| assets.contains(&occurrence.asset_id))
                .map(|occurrence| occurrence.id.clone())
                .collect(),
            assets,
        };
        binding.validate_text(text)?;
        binding.validate_snapshot(&snapshot)?;
        Ok(binding)
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
