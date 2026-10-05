//! Exact knowledge preparation and approval guards over the existing proposal.
use super::*;
use crate::{
    editor::file_error,
    knowledge::IdentityOutcome,
    proposal_apply::ApplyJournal,
    proposals::{DraftNoteChange, DraftRequest},
    vault::VaultPath,
};
use brn_ai::KnowledgeProposalArgs;
use brn_store::{note_identity, note_metadata, note_provenance};
use std::path::Path;

fn rejected(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, message)
}
impl App {
    /// Retained Source citations plus explicit saved context. Original creation
    /// replay reuses its ordered proofs before observing any current files.
    pub(crate) fn prepare_inbox_knowledge(
        &mut self,
        job: &InboxActionJob,
        args: &KnowledgeProposalArgs,
        session: Uuid,
    ) -> Result<DraftRequest> {
        args.validate()
            .map_err(|_| rejected("invalid knowledge protocol input"))?;
        if job.capture.purpose != InboxAnalysisPurpose::KnowledgeAndActions {
            return Err(rejected("this analysis did not admit knowledge proposals"));
        }
        let id =
            Uuid::parse_str(&args.id).map_err(|_| rejected("knowledge proposal needs a UUID"))?;
        let note_id =
            Uuid::parse_str(&args.note_id).map_err(|_| rejected("knowledge note needs a UUID"))?;
        VaultPath::parse(&args.path)
            .map_err(|_| rejected("knowledge destination must be Current"))?;
        if note_metadata::classify(&args.text)? != note_metadata::NoteClassification::default()
            || brn_store::work::inbox_source::read_provenance(&args.text)?.is_some()
            || !note_provenance::read(&args.text)?.is_empty()
        {
            return Err(rejected(
                "knowledge candidate cannot invent Source/provenance or History authority",
            ));
        }
        let source_id = job.capture.note_id()?;
        let mut citations = Vec::new();
        for range in &args.quotes {
            let quote = job
                .capture
                .source_text
                .get(range.start_byte..range.end_byte)
                .ok_or_else(|| rejected("Source quote is outside full text or splits UTF-8"))?;
            citations.push(note_provenance::VaultCitation {
                note_id: source_id,
                sha256: job.capture.source.fingerprint.sha256,
                start_byte: range.start_byte,
                end_byte: range.end_byte,
                quote: quote.into(),
            });
        }
        note_provenance::validate(&citations)?;
        let text =
            note_provenance::write(&note_identity::assign(&args.text, note_id)?, &citations)?;
        let sources = match self.store.proposal(id)? {
            Some(existing) => {
                if existing.draft.sources.first() != Some(&job.capture.source)
                    || !args.source_paths.iter().map(String::as_str).eq(existing
                        .draft
                        .sources
                        .iter()
                        .skip(1)
                        .map(|s| s.path.as_str()))
                {
                    return Err(rejected("knowledge creation target paths changed"));
                }
                existing.draft.sources
            }
            None => {
                let mut sources = vec![job.capture.source.clone()];
                for path in &args.source_paths {
                    sources.push(self.proposal_evidence_source(path)?.source);
                }
                sources
            }
        };
        let request = DraftRequest {
            inbox_knowledge: Some(Box::new(InboxKnowledgeBinding {
                analysis_id: job.capture.id,
                note_id,
                source: job.capture.source.clone(),
                citations,
            })),
            inbox_source: None,
            id,
            group_id: Some(job.capture.id),
            session_id: Some(session),
            title: args.title.clone(),
            changes: vec![DraftNoteChange::Create {
                path: args.path.clone(),
                text,
            }],
            sources,
            action_changes: vec![],
        };
        request.validate()?;
        Ok(request)
    }

    /// A bound knowledge Create remains tied to the full immutable analysis,
    /// even after review edits. It never authorizes replacement or deletion.
    pub(crate) fn validate_inbox_knowledge(
        &mut self,
        binding: Option<&InboxKnowledgeBinding>,
    ) -> Result<()> {
        let Some(binding) = binding else {
            return Ok(());
        };
        let job = self.inbox_knowledge_capture(binding)?;
        self.validate_inbox_action_source(&job.capture)?;
        if self.resolve_note_identity(binding.note_id)?.outcome != IdentityOutcome::Absent {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "knowledge identity already exists or cannot be completely inspected",
            ));
        }
        Ok(())
    }

    fn inbox_knowledge_capture(&self, binding: &InboxKnowledgeBinding) -> Result<InboxActionJob> {
        binding.validate()?;
        let job = self
            .store
            .inbox_action(binding.analysis_id)?
            .ok_or_else(|| rejected("knowledge analysis capture is unavailable"))?;
        let source_id = job.capture.note_id()?;
        if job.capture.purpose != InboxAnalysisPurpose::KnowledgeAndActions
            || job.capture.source != binding.source
            || binding.citations.iter().any(|c| {
                c.note_id != source_id
                    || job.capture.source_text.get(c.start_byte..c.end_byte)
                        != Some(c.quote.as_str())
            })
        {
            return Err(rejected(
                "knowledge proposal differs from exact captured Source evidence",
            ));
        }
        Ok(job)
    }

    /// Rechecks complete Source/identity authority during apply and Finish. The
    /// only existing candidate allowed is this operation's exact prepared object.
    pub(crate) fn check_inbox_knowledge_apply(&self, journal: &ApplyJournal) -> Result<()> {
        let Some(binding) = journal.approved.draft.inbox_knowledge.as_deref() else {
            return Ok(());
        };
        let job = self.inbox_knowledge_capture(binding)?;
        let files = self.editor.files.as_ref().ok_or_else(|| {
            WorkflowError::typed(ErrorKind::ContextStale, "knowledge files are unavailable")
        })?;
        let source = files
            .observe(Path::new(&binding.source.path))
            .map_err(file_error)?;
        if source.fingerprint != binding.source.fingerprint
            || source.text != job.capture.source_text
        {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "selected Inbox Source changed during knowledge application",
            ));
        }
        let inventory = self.inspect_identity_inventory()?;
        let source_identity = inventory.resolution(job.capture.note_id()?);
        if source_identity.outcome != IdentityOutcome::Unique
            || source_identity.matches[0].path != binding.source.path
            || source_identity.matches[0].sha256 != binding.source.fingerprint.sha256
        {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "selected Inbox Source identity is ambiguous or incompletely inspected",
            ));
        }
        let draft = &journal.approved.draft;
        let text = draft.changes[0]
            .text()
            .ok_or_else(|| rejected("knowledge Create text is unavailable"))?;
        for id in crate::knowledge::stable_link_ids(text)? {
            if id == binding.note_id {
                // The sole Create's identity remains subject to the exact
                // own-prepared proof check below, including after installation.
                continue;
            }
            let target = inventory.resolution(id);
            if target.outcome != IdentityOutcome::Unique {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "knowledge link target identity changed or is incompletely inspected",
                ));
            }
            let target = &target.matches[0];
            let captured = draft.sources.iter().find(|source| {
                source.path == target.path && source.fingerprint.sha256 == target.sha256
            });
            let observed = files.observe(Path::new(&target.path)).map_err(file_error)?;
            if captured.is_none_or(|source| source.fingerprint != observed.fingerprint)
                || crate::library::saved_metadata(&observed.text, &target.path)
                    .issue
                    .is_some()
            {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "knowledge link target no longer matches exact reviewed saved evidence",
                ));
            }
        }
        let candidate = inventory.resolution(binding.note_id);
        if candidate.outcome == IdentityOutcome::Absent {
            return Ok(());
        }
        let path = journal.approved.draft.changes[0].path();
        let own_prepared = journal.prepared.as_ref().and_then(|proofs| proofs.first());
        let own_installed = match (candidate.outcome, own_prepared) {
            (IdentityOutcome::Unique, Some(proof))
                if candidate.matches[0].path == path
                    && candidate.matches[0].sha256 == proof.sha256 =>
            {
                files
                    .observe(Path::new(path))
                    .map_err(file_error)?
                    .fingerprint
                    == *proof
            }
            _ => false,
        };
        if !own_installed {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "knowledge identity is occupied, ambiguous or no longer this prepared note",
            ));
        }
        Ok(())
    }
}
