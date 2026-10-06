//! One exact saved PNG occurrence, transported transiently and annotated only
//! through the existing proposal family. Original DOCX bytes are never needed
//! to replay an already completed analysis or approval.
use super::*;
use crate::{
    editor::file_error,
    proposals::{DraftNoteChange, DraftRequest},
};
use brn_store::work::{
    WorkTurnStatus, inbox_visual::InboxVisualAnnotationBinding, proposal_apply::ApplyJournal,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{path::Path, sync::atomic::AtomicBool};

fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}
fn rejected(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, message)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Interpretation {
    description: String,
    uncertainty: String,
}

impl App {
    /// Fresh complete image qualification after terminal replay has been ruled
    /// out. No payload is persisted in the analysis question or job.
    pub(crate) fn inbox_visual_image(
        &self,
        capture: &InboxActionCapture,
    ) -> Result<Option<brn_ai::VisualImage>> {
        capture.validate()?;
        if capture.purpose != InboxAnalysisPurpose::VisualInterpretation {
            return Ok(None);
        }
        let asset = capture
            .visual_asset
            .as_ref()
            .expect("validated visual asset");
        let files = self
            .editor
            .files
            .as_ref()
            .ok_or_else(|| stale("visual files are unavailable"))?;
        let observed = files
            .observe_asset(Path::new(&asset.path))
            .map_err(file_error)?;
        if observed.fingerprint != asset.fingerprint {
            return Err(stale("selected visual asset changed"));
        }
        let provenance = brn_store::work::inbox_source::read_provenance(&capture.source_text)?
            .ok_or_else(|| rejected("visual Source has no provenance"))?;
        let visual = provenance
            .visual
            .as_ref()
            .ok_or_else(|| rejected("visual Source has no occurrence"))?;
        let facts = brn_store::work::inbox_source::validate_png_image(
            &observed.bytes,
            &AtomicBool::new(false),
        )
        .map_err(|_| stale("selected visual PNG is incomplete or unsupported"))?;
        if facts.width != visual.width || facts.height != visual.height {
            return Err(stale("selected visual PNG dimensions changed"));
        }
        brn_ai::VisualImage::png(observed.bytes)
            .map(Some)
            .map_err(|_| rejected("visual transport exceeds its bound"))
    }

    /// Provider-free preparation from a completed owned turn. Clients can then
    /// create this review draft with the existing CreateProposal command.
    pub fn prepare_inbox_visual_annotation(&mut self, analysis_id: Uuid) -> Result<DraftRequest> {
        let job = self
            .store
            .inbox_action(analysis_id)?
            .ok_or_else(|| rejected("visual analysis capture is unavailable"))?;
        if job.capture.purpose != InboxAnalysisPurpose::VisualInterpretation {
            return Err(rejected("this analysis does not interpret a visual"));
        }
        let turn = self
            .store
            .turn(analysis_id)?
            .ok_or_else(|| rejected("visual analysis has no completed turn"))?;
        if turn.status != WorkTurnStatus::Completed || turn.answer.len() > 16 * 1024 {
            return Err(rejected(
                "only a complete bounded interpretation can become a proposal",
            ));
        }
        let interpretation: Interpretation = serde_json::from_str(&turn.answer).map_err(|_| {
            rejected("visual interpretation needs exact description and uncertainty JSON")
        })?;
        let binding = InboxVisualAnnotationBinding {
            analysis_id,
            note_id: job.capture.note_id()?,
            source: job.capture.source.clone(),
            source_text: job.capture.source_text.clone(),
            asset: job
                .capture
                .visual_asset
                .clone()
                .expect("validated visual capture"),
            description: interpretation.description,
            uncertainty: interpretation.uncertainty,
        };
        binding.validate_capture(&job)?;
        let text = binding.candidate_text()?;
        let input =
            serde_json::to_vec(&binding).map_err(|_| rejected("could not bind visual intent"))?;
        let mut hash = Sha256::new();
        hash.update(b"brn/inbox-visual-annotation-proposal/v1\0");
        hash.update(&input);
        let digest = hash.finalize();
        let mut bytes: [u8; 16] = digest[..16].try_into().expect("SHA-256 prefix");
        bytes[6] = (bytes[6] & 0x0f) | 0x80;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let request = DraftRequest {
            inbox_visual: Some(Box::new(binding.clone())),
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::from_bytes(bytes),
            group_id: Some(analysis_id),
            session_id: Some(turn.conversation_id),
            title: "Review tentative visual interpretation".into(),
            changes: vec![DraftNoteChange::Replace {
                path: binding.source.path.clone(),
                expected: binding.source.fingerprint.clone(),
                text,
            }],
            sources: vec![binding.source.clone()],
            action_changes: vec![],
        };
        request.validate()?;
        // Reconstructed original creation remains reviewable after later edits
        // or loss. Creation replay checks the complete retained initial draft.
        match self.proposal(request.id) {
            Ok(_) => return Ok(request),
            Err(error) if error.kind == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        self.validate_inbox_visual(Some(&binding))?;
        Ok(request)
    }

    pub(crate) fn validate_inbox_visual(
        &mut self,
        binding: Option<&InboxVisualAnnotationBinding>,
    ) -> Result<()> {
        let Some(binding) = binding else {
            return Ok(());
        };
        let job = self
            .store
            .inbox_action(binding.analysis_id)?
            .ok_or_else(|| rejected("visual analysis capture is unavailable"))?;
        binding.validate_capture(&job)?;
        self.validate_inbox_action_source(&job.capture)?;
        self.inbox_visual_image(&job.capture)?;
        Ok(())
    }

    /// During application, accept only the captured before or this approval's
    /// exact prepared Source object. The separately bound PNG must stay fresh.
    pub(crate) fn check_inbox_visual_apply(&self, journal: &ApplyJournal) -> Result<()> {
        let Some(binding) = journal.approved.draft.inbox_visual.as_deref() else {
            return Ok(());
        };
        let job = self
            .store
            .inbox_action(binding.analysis_id)?
            .ok_or_else(|| rejected("visual analysis capture is unavailable"))?;
        binding.validate_capture(&job)?;
        let files = self
            .editor
            .files
            .as_ref()
            .ok_or_else(|| stale("visual files are unavailable"))?;
        let observed = files
            .observe(Path::new(&binding.source.path))
            .map_err(file_error)?;
        let prepared = journal.prepared.as_ref().and_then(|proofs| proofs.first());
        if observed.fingerprint == binding.source.fingerprint {
            if observed.text != binding.source_text {
                return Err(stale("visual Source bytes changed"));
            }
        } else if prepared != Some(&observed.fingerprint)
            || journal.approved.draft.changes[0].text() != Some(observed.text.as_str())
        {
            return Err(stale(
                "visual Source is not its exact before or prepared annotation",
            ));
        }
        let inventory = self.inspect_identity_inventory()?;
        let identity = inventory.resolution(binding.note_id);
        if identity.outcome != crate::knowledge::IdentityOutcome::Unique
            || identity.matches[0].path != binding.source.path
            || identity.matches[0].sha256 != observed.fingerprint.sha256
        {
            return Err(stale(
                "visual Source identity is ambiguous or incompletely inspected",
            ));
        }
        self.inbox_visual_image(&job.capture)?;
        Ok(())
    }
}
