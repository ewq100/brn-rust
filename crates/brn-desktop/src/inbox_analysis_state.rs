//! Saved-Source analysis presentation. Workflow owns eligibility and execution.
use super::{ActiveRequest, ActiveTurn, AiState, Pending};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    findings::{FindingOrigin, FindingRecord},
    inbox_actions::{InboxActionAnalysis, InboxActionRequest, InboxAnalysisPurpose},
    proposals::ProposalSource,
};
use uuid::Uuid;

#[derive(Clone)]
pub enum AnalysisPending {
    Source {
        view: u64,
        generation: u64,
        path: String,
    },
    Inspection {
        view: u64,
        generation: u64,
        analysis: Uuid,
    },
}

#[derive(Default)]
pub struct InboxAnalysisView {
    pub source_path: Option<String>,
    pub source: Option<ProposalSource>,
    pub source_error: Option<String>,
    pub analysis_id: Option<Uuid>,
    pub record: Option<InboxActionAnalysis>,
    pub error: Option<String>,
    /// Last explicit submission is retained for reply correlation, never resubmitted.
    pub request: Option<InboxActionRequest>,
    visible: bool,
    view: u64,
    source_generation: u64,
    inspection_generation: u64,
}

impl AiState {
    /// Only a member of the currently inspected analysis may open Needs Review.
    pub fn inbox_analysis_finding(&self, analysis: Uuid, id: Uuid) -> Option<&FindingRecord> {
        if !self.ready
            || !self.inbox_analysis.visible
            || self.inbox_analysis.analysis_id != Some(analysis)
            || id.is_nil()
        {
            return None;
        }
        let record = self.inbox_analysis.record.as_ref()?;
        if record.job.capture.id != analysis {
            return None;
        }
        record.findings.iter().find(|finding| finding.draft.request.id == id
            && matches!(finding.draft.request.origin, FindingOrigin::InboxConflict { analysis_id, .. } if analysis_id == analysis))
    }

    pub fn inbox_analysis_path_for_turn(&self, id: Uuid) -> Option<&str> {
        if let Some(request) = self
            .inbox_analysis
            .request
            .as_ref()
            .filter(|request| request.id == id)
        {
            return Some(&request.source.source.path);
        }
        self.inbox_analysis
            .record
            .as_ref()
            .filter(|record| record.job.capture.id == id)
            .map(|record| record.job.capture.source.path.as_str())
    }
    pub(super) fn open_analysis_view(&mut self) {
        self.inbox_analysis.visible = true;
        self.inbox_analysis.view = self.inbox_analysis.view.wrapping_add(1);
    }
    pub(super) fn close_analysis_view(&mut self) {
        self.inbox_analysis.visible = false;
        self.inbox_analysis.view = self.inbox_analysis.view.wrapping_add(1);
    }
    pub fn inspect_inbox_analysis_source(&mut self, path: String) -> Option<(Uuid, AppCommand)> {
        if !self.ready
            || !self.vault_bound
            || !self.inbox_analysis.visible
            || self.application_busy()
        {
            return None;
        }
        let view = &mut self.inbox_analysis;
        view.source_generation = view.source_generation.wrapping_add(1);
        view.source_path = Some(path.clone());
        view.source = None;
        view.source_error = None;
        Some(self.command(
            Pending::InboxAnalysis(AnalysisPending::Source {
                view: self.inbox_analysis.view,
                generation: self.inbox_analysis.source_generation,
                path: path.clone(),
            }),
            AppCommand::ProposalEvidenceSource(path),
        ))
    }
    pub fn inbox_analysis_source_loading(&self) -> bool {
        self.pending.values().any(|pending| matches!(pending,
            Pending::InboxAnalysis(AnalysisPending::Source { view, generation, .. })
                if *view == self.inbox_analysis.view && *generation == self.inbox_analysis.source_generation))
    }
    pub fn can_analyze_inbox_source(&self) -> bool {
        self.inbox_analysis.visible
            && self.can_ask()
            && !self
                .pending
                .values()
                .any(|pending| matches!(pending, Pending::Selection))
            && !self.inbox_analysis_source_loading()
            && self.inbox_analysis.source.is_some()
            && self.inbox_analysis.source_error.is_none()
    }
    pub fn analyze_inbox_source(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.can_analyze_inbox_source() {
            return None;
        }
        let request = InboxActionRequest {
            purpose: InboxAnalysisPurpose::KnowledgeAndActions,
            id: Uuid::new_v4(),
            conversation: self.conversation,
            source: Box::new(self.inbox_analysis.source.clone()?),
            selection: self.selection.clone()?,
            effort: self.effort?,
            generation: self.generation,
        };
        if let Err(error) = request.validate() {
            self.inbox_analysis.source_error = Some(error.message);
            return None;
        }
        self.inbox_analysis.analysis_id = Some(request.id);
        self.inbox_analysis.inspection_generation =
            self.inbox_analysis.inspection_generation.wrapping_add(1);
        self.inbox_analysis.record = None;
        self.inbox_analysis.error = None;
        self.inbox_analysis.request = Some(request.clone());
        self.active = Some(ActiveTurn {
            request: ActiveRequest::Inbox(Box::new(request.clone())),
            partial: String::new(),
            tool: None,
            stopping: false,
        });
        self.notice = "Source analysis requested. Drafts require separate review and exact approval; originals stay retained.".into();
        Some((
            request.id,
            AppCommand::AnalyzeInboxActions(Box::new(request)),
        ))
    }
    pub fn inspect_inbox_analysis(&mut self, analysis: Uuid) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.inbox_analysis.visible || analysis.is_nil() {
            return None;
        }
        let view = &mut self.inbox_analysis;
        view.inspection_generation = view.inspection_generation.wrapping_add(1);
        view.analysis_id = Some(analysis);
        view.record = None;
        view.error = None;
        Some(self.command(
            Pending::InboxAnalysis(AnalysisPending::Inspection {
                view: self.inbox_analysis.view,
                generation: self.inbox_analysis.inspection_generation,
                analysis,
            }),
            AppCommand::InboxActionAnalysis(analysis),
        ))
    }
    pub fn inbox_analysis_loading(&self) -> bool {
        self.pending.values().any(|pending| matches!(pending,
            Pending::InboxAnalysis(AnalysisPending::Inspection { view, generation, .. })
                if *view == self.inbox_analysis.view && *generation == self.inbox_analysis.inspection_generation))
    }
    pub(super) fn analysis_settled(&mut self, analysis: Option<Uuid>) -> Vec<(Uuid, AppCommand)> {
        let Some(analysis) = analysis else {
            return Vec::new();
        };
        let mut commands = vec![self.command(Pending::Proposals, AppCommand::Proposals(None))];
        if self.inbox_analysis.analysis_id == Some(analysis)
            && let Some(command) = self.inspect_inbox_analysis(analysis)
        {
            commands.push(command);
        }
        commands
    }
    pub(super) fn received_inbox_analysis(&mut self, id: Uuid, event: &AppEvent) -> bool {
        let Some(Pending::InboxAnalysis(pending)) = self.pending.get(&id).cloned() else {
            return false;
        };
        let state = &mut self.inbox_analysis;
        match pending {
            AnalysisPending::Source {
                view,
                generation,
                path,
            } => {
                if !state.visible
                    || state.view != view
                    || state.source_generation != generation
                    || state.source_path.as_ref() != Some(&path)
                {
                    self.pending.remove(&id);
                    return true;
                }
                match event {
                    AppEvent::ProposalSource(source)
                        if source.source.path == path && source.validate().is_ok() =>
                    {
                        state.source = Some((**source).clone());
                        state.source_error = None;
                    }
                    AppEvent::Failed(error) => state.source_error = Some(error.message.clone()),
                    _ => return true,
                }
            }
            AnalysisPending::Inspection {
                view,
                generation,
                analysis,
            } => {
                if !state.visible
                    || state.view != view
                    || state.inspection_generation != generation
                    || state.analysis_id != Some(analysis)
                {
                    self.pending.remove(&id);
                    return true;
                }
                match event {
                    AppEvent::InboxActionAnalysis(record)
                        if analysis_matches(state.request.as_ref(), analysis, record) =>
                    {
                        state.record = Some((**record).clone());
                        state.error = None;
                    }
                    AppEvent::Failed(error) => state.error = Some(error.message.clone()),
                    _ => return true,
                }
            }
        }
        self.pending.remove(&id);
        true
    }
}

fn analysis_matches(
    request: Option<&InboxActionRequest>,
    id: Uuid,
    record: &InboxActionAnalysis,
) -> bool {
    let capture = &record.job.capture;
    if capture.id != id
        || record.job.validate().is_err()
        || !record.needs_semantic_review
        || record.turn.as_ref().is_some_and(|turn| {
            turn.id != id
                || capture
                    .conversation
                    .is_some_and(|id| turn.conversation_id != id)
                || turn.question != record.job.question
                || turn.provider != capture.provider
                || turn.model != capture.model
                || turn.effort.as_deref() != Some(capture.effort.as_str())
        })
        || record
            .proposals
            .iter()
            .any(|proposal| proposal.draft.id.is_nil() || proposal.draft.group_id != Some(id))
        || record.findings.iter().any(|finding| {
            finding.draft.request.id.is_nil()
                || !matches!(finding.draft.request.origin, FindingOrigin::InboxConflict { analysis_id, .. } if analysis_id == id)
        })
    {
        return false;
    }
    let mut ids = std::collections::HashSet::new();
    if record
        .proposals
        .iter()
        .any(|proposal| !ids.insert(proposal.draft.id))
    {
        return false;
    }
    let mut finding_ids = std::collections::HashSet::new();
    if record
        .findings
        .iter()
        .any(|finding| !finding_ids.insert(finding.draft.request.id))
    {
        return false;
    }
    let Some(request) = request.filter(|request| request.id == id) else {
        return true;
    };
    capture.purpose == request.purpose
        && capture.conversation == request.conversation
        && capture.source == request.source.source
        && capture.source_text == request.source.text
        && capture.provider == super::provider_key(request.selection.provider)
        && capture.model == request.selection.model
        && capture.effort == request.effort.as_str()
}
