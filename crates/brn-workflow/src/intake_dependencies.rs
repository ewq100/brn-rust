//! Source prerequisites on the existing proposal/application path.
use crate::{ErrorKind, Result, WorkflowError, app::App, editor::file_error};
use brn_store::work::{
    inbox_actions::InboxIntakeBinding,
    proposal_apply::ApplyOutcome,
    proposals::{NoteChange, ProposalDraft, ProposalState},
};
use std::path::Path;

pub(crate) fn dependency(draft: &ProposalDraft) -> Option<&InboxIntakeBinding> {
    draft.intake.as_deref().or_else(|| {
        draft
            .inbox_knowledge
            .as_ref()
            .and_then(|b| b.intake.as_deref())
    })
}
fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}
impl App {
    /// Pending evidence can prepare review; only the exact Applied Source can admit effects.
    pub(crate) fn validate_intake_dependency(
        &self,
        binding: Option<&InboxIntakeBinding>,
        applied: bool,
    ) -> Result<()> {
        let Some(binding) = binding else {
            return Ok(());
        };
        binding.validate()?;
        let snapshot = self
            .store
            .intake_snapshot(binding.snapshot_id)?
            .ok_or_else(|| stale("private extraction is missing"))?;
        if snapshot.digest()? != binding.snapshot_sha256 {
            return Err(stale("private extraction changed"));
        }
        let proposal = self
            .store
            .proposal(binding.source_proposal.id)?
            .ok_or_else(|| stale("Source prerequisite is missing"))?;
        let source_binding = proposal
            .draft
            .inbox_source
            .as_ref()
            .ok_or_else(|| stale("prerequisite has no Source extraction"))?;
        let extraction = source_binding
            .extraction
            .as_ref()
            .ok_or_else(|| stale("prerequisite has no snapshot receipt"))?;
        let Some(NoteChange::Create { path, text, .. }) = proposal.draft.changes.first() else {
            return Err(stale("Source prerequisite Create is missing"));
        };
        if extraction.snapshot_id != binding.snapshot_id
            || extraction.snapshot_sha256 != binding.snapshot_sha256
            || path != &binding.source_path
            || source_binding.note_id != binding.source_note_id
            || brn_intake::digest(text.as_bytes()) != binding.source_text_sha256
        {
            return Err(stale(
                "Source prerequisite differs from investigated version",
            ));
        }
        if !applied
            && proposal.state == ProposalState::Draft
            && proposal.stamp() == binding.source_proposal
        {
            return self.validate_inbox_source(Some(source_binding));
        }
        if proposal.state != ProposalState::Applied {
            return Err(stale(
                "Apply the selected exact Source prerequisite before this knowledge or Action",
            ));
        }
        let mut exact_receipt = false;
        for id in self.store.proposal_apply_ids()? {
            let journal = self
                .store
                .proposal_apply(id)?
                .ok_or_else(|| stale("Source approval journal missing"))?;
            if journal.approved.stamp() == binding.source_proposal
                && journal.approved.draft == proposal.draft
                && journal
                    .receipt
                    .as_ref()
                    .is_some_and(|r| r.outcome == ApplyOutcome::Applied)
            {
                exact_receipt = true;
            }
        }
        if !exact_receipt {
            return Err(stale("Source prerequisite has no exact Applied receipt"));
        }
        self.validate_inbox_source(Some(source_binding))?;
        let files = self
            .editor
            .files
            .as_ref()
            .ok_or_else(|| stale("Source files unavailable"))?;
        for asset in &extraction.assets {
            let asset_path = extraction.asset_path(path, &asset.name)?;
            let observed = files
                .observe_asset(Path::new(&asset_path))
                .map_err(file_error)?;
            if observed.bytes.len() as u64 != asset.byte_len
                || brn_intake::digest(&observed.bytes) != asset.sha256
            {
                return Err(stale(
                    "Applied Source asset differs from investigated bytes",
                ));
            }
        }
        let saved = self
            .editor
            .files
            .as_ref()
            .ok_or_else(|| stale("Source files unavailable"))?
            .observe(Path::new(path))
            .map_err(file_error)?;
        if saved.text != *text
            || brn_store::note_identity::read(&saved.text)? != Some(binding.source_note_id)
        {
            return Err(stale("Applied Source differs from investigated bytes"));
        }
        Ok(())
    }
}
