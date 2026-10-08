//! Presentation correlation only. Workflow checks PNG and annotation authority.
use super::{AiState, Pending, inbox_analysis_state::AnalysisPending};
use brn_workflow::{
    WorkTurnStatus,
    app_worker::{AppCommand, AppEvent},
    inbox_actions::{InboxActionCapture, InboxAnalysisPurpose},
    proposals::{DraftRequest, NoteChange, ProposalSource},
};
use uuid::Uuid;

#[derive(Clone)]
pub enum VisualStep {
    Evidence(Box<ProposalSource>),
    Prepare(Box<InboxActionCapture>),
    Create(Box<DraftRequest>),
}
#[derive(Clone)]
pub struct VisualPending {
    view: u64,
    source_generation: u64,
    inspection_generation: u64,
    step: VisualStep,
}
impl AiState {
    pub(super) fn invalidate_visual_analysis(&mut self) {
        let view = &mut self.inbox_analysis;
        view.visual = None;
        view.visual_error = None;
        view.annotation = None;
        view.annotation_review = None;
    }
    pub fn visual_analysis_pending(&self) -> bool {
        self.pending.values().any(|pending| matches!(pending, Pending::InboxAnalysis(AnalysisPending::Visual(p)) if self.visual_pending_current(p)))
    }
    fn visual_pending_current(&self, p: &VisualPending) -> bool {
        let view = &self.inbox_analysis;
        view.visible
            && view.view == p.view
            && view.source_generation == p.source_generation
            && (matches!(p.step, VisualStep::Evidence(_))
                || view.inspection_generation == p.inspection_generation)
    }
    fn visual_command(&mut self, step: VisualStep, command: AppCommand) -> (Uuid, AppCommand) {
        let view = &self.inbox_analysis;
        self.command(
            Pending::InboxAnalysis(AnalysisPending::Visual(Box::new(VisualPending {
                view: view.view,
                source_generation: view.source_generation,
                inspection_generation: view.inspection_generation,
                step,
            }))),
            command,
        )
    }
    pub fn inspect_inbox_visual(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready
            || !self.vault_bound
            || !self.inbox_analysis.visible
            || self.application_busy()
            || self.inbox_analysis_source_loading()
            || self.visual_analysis_pending()
        {
            return None;
        }
        let source = self.inbox_analysis.source.clone()?;
        source.validate().ok()?;
        self.invalidate_visual_analysis();
        Some(self.visual_command(
            VisualStep::Evidence(Box::new(source.clone())),
            AppCommand::InboxVisualEvidence(source.source.path),
        ))
    }
    pub fn can_interpret_inbox_visual(&self) -> bool {
        self.can_analyze_inbox_source()
            && !self.visual_analysis_pending()
            && self.inbox_analysis.visual.as_ref().is_some_and(|visual| {
                visual.validate().is_ok()
                    && self.inbox_analysis.source.as_ref() == Some(visual.source.as_ref())
            })
    }
    pub fn interpret_inbox_visual(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.can_interpret_inbox_visual() {
            return None;
        }
        let asset = self.inbox_analysis.visual.as_ref()?.asset.clone();
        self.start_source_analysis(InboxAnalysisPurpose::VisualInterpretation, Some(asset))
    }
    pub fn can_prepare_visual_annotation(&self) -> bool {
        let view = &self.inbox_analysis;
        self.ready
            && self.vault_bound
            && view.visible
            && !self.application_busy()
            && self.active.is_none()
            && self.unsaved.is_none()
            && !self.visual_analysis_pending()
            && !self.inbox_analysis_loading()
            && view.record.as_ref().is_some_and(|record| {
                let capture = &record.job.capture;
                capture.purpose == InboxAnalysisPurpose::VisualInterpretation
                    && view.analysis_id == Some(capture.id)
                    && view.visual.as_ref().is_some_and(|visual| {
                        visual.validate().is_ok()
                            && capture.source.as_ref() == Some(&visual.source.source)
                            && capture.source_text == visual.source.text
                            && capture.visual_asset.as_ref() == Some(&visual.asset)
                    })
                    && record
                        .turn
                        .as_ref()
                        .is_some_and(|turn| turn.status == WorkTurnStatus::Completed)
            })
    }
    pub fn prepare_visual_annotation(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.can_prepare_visual_annotation() {
            return None;
        }
        let capture = self.inbox_analysis.record.as_ref()?.job.capture.clone();
        self.inbox_analysis.annotation = None;
        self.inbox_analysis.annotation_review = None;
        self.inbox_analysis.visual_error = None;
        Some(self.visual_command(
            VisualStep::Prepare(Box::new(capture.clone())),
            AppCommand::PrepareInboxVisualAnnotation(capture.id),
        ))
    }
    pub fn create_visual_annotation(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.can_prepare_visual_annotation() || self.inbox_analysis.annotation_review.is_some()
        {
            return None;
        }
        let draft = self.inbox_analysis.annotation.clone()?;
        Some(self.visual_command(
            VisualStep::Create(Box::new(draft.clone())),
            AppCommand::CreateProposal(draft),
        ))
    }
    pub(super) fn received_visual_analysis(
        &mut self,
        id: Uuid,
        pending: VisualPending,
        event: &AppEvent,
    ) {
        if !self.visual_pending_current(&pending) {
            self.pending.remove(&id);
            return;
        }
        let view = &mut self.inbox_analysis;
        let accepted = match (&pending.step, event) {
            (VisualStep::Evidence(source), AppEvent::InboxVisualEvidence(visual))
                if visual.validate().is_ok()
                    && visual.source == *source
                    && view.source.as_ref() == Some(source.as_ref()) =>
            {
                view.visual = Some((**visual).clone());
                view.visual_error = None;
                true
            }
            (VisualStep::Prepare(capture), AppEvent::InboxVisualDraft(draft))
                if view
                    .record
                    .as_ref()
                    .is_some_and(|record| record.job.capture == **capture)
                    && annotation_matches(
                        draft,
                        capture,
                        view.record.as_ref().and_then(|record| record.turn.as_ref()),
                    ) =>
            {
                view.annotation = Some((**draft).clone());
                view.visual_error = None;
                true
            }
            (VisualStep::Create(draft), AppEvent::Proposal(record))
                if crate::draft::creation_matches(draft, record)
                    && record.draft.inbox_visual == draft.inbox_visual
                    && (record.version > 1 || record.draft.title == draft.title)
                    && record.draft.changes.iter().zip(&draft.changes).all(
                        |(change, input)| match (change, input) {
                            (
                                NoteChange::Replace {
                                    before_text, text, ..
                                },
                                brn_workflow::proposals::DraftNoteChange::Replace {
                                    text: expected,
                                    ..
                                },
                            ) => draft.inbox_visual.as_ref().is_some_and(|binding| {
                                &binding.source_text == before_text
                                    && (record.version > 1 || text == expected)
                                    && binding
                                        .validate_replace(
                                            &binding.source.path,
                                            &binding.source.fingerprint,
                                            before_text,
                                            text,
                                        )
                                        .is_ok()
                            }),
                            _ => false,
                        },
                    ) =>
            {
                view.annotation_review = Some(record.draft.id);
                view.visual_error = None;
                true
            }
            (_, AppEvent::Failed(error)) => {
                view.visual_error = Some(error.message.clone());
                true
            }
            _ => false,
        };
        if accepted {
            self.pending.remove(&id);
        }
    }
}
fn annotation_matches(
    draft: &DraftRequest,
    capture: &InboxActionCapture,
    turn: Option<&brn_workflow::WorkTurn>,
) -> bool {
    let Some(binding) = &draft.inbox_visual else {
        return false;
    };
    let Some(turn) = turn.filter(|turn| turn.status == WorkTurnStatus::Completed) else {
        return false;
    };
    let Ok(answer) = serde_json::from_str::<serde_json::Value>(&turn.answer) else {
        return false;
    };
    draft.validate().is_ok()
        && matches!(draft.changes.as_slice(), [brn_workflow::proposals::DraftNoteChange::Replace { text, .. }]
            if binding.candidate_text().is_ok_and(|candidate| text == &candidate))
        && draft.group_id == Some(capture.id)
        && draft.session_id == Some(turn.conversation_id)
        && binding.analysis_id == capture.id
        && Some(&binding.source) == capture.source.as_ref()
        && binding.source_text == capture.source_text
        && capture.visual_asset.as_ref() == Some(&binding.asset)
        && answer.as_object().is_some_and(|answer| answer.len() == 2)
        && answer["description"].as_str() == Some(binding.description.as_str())
        && answer["uncertainty"].as_str() == Some(binding.uncertainty.as_str())
}
