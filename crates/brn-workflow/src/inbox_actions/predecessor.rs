//! Explicit owner restructuring over the existing exact supersession format.
use super::*;
use crate::{
    editor::file_error,
    proposals::{KnowledgePredecessorRequest, NoteChange, ProposalState},
};
use brn_store::{note_identity, note_metadata, work::inbox_actions::InboxSupersedesBinding};
use std::path::Path;

fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}

impl App {
    /// Captures a Current predecessor without altering any saved note or original
    /// creation payload. Only the returned revised review can later be approved.
    pub fn attach_inbox_knowledge_predecessor(
        &mut self,
        request: &KnowledgePredecessorRequest,
    ) -> Result<ProposalRecord> {
        request.validate()?;
        let record = self.proposal(request.expected.id)?;
        let Some(binding) = record.draft.inbox_knowledge.as_ref() else {
            return Err(stale(
                "predecessor attachment needs an Inbox Knowledge draft",
            ));
        };
        let [NoteChange::Create { path, text, .. }] = record.draft.changes.as_slice() else {
            return Err(stale(
                "predecessor attachment needs one supplemental Create",
            ));
        };
        if record.stamp() != request.expected
            || record.state != ProposalState::Draft
            || binding.supersedes.is_some()
        {
            return Err(stale(
                "predecessor attachment review version or shape changed",
            ));
        }
        self.require_current_evidence()?;
        self.editor_files()?;
        self.validate_intake_dependency(binding.intake.as_deref(), false)?;
        self.validate_inbox_knowledge(Some(binding))?;
        let saved = self.proposal_source(&request.predecessor_path)?;
        let predecessor_id = note_identity::read(&saved.text)?
            .ok_or_else(|| stale("predecessor needs a saved managed identity"))?;
        if predecessor_id == binding.note_id
            || binding
                .citations
                .iter()
                .any(|citation| citation.note_id == predecessor_id)
            || note_metadata::classify(&saved.text)? != note_metadata::NoteClassification::default()
            || brn_store::work::inbox_source::read_provenance(&saved.text)?.is_some()
        {
            return Err(stale("predecessor must be separate Current knowledge"));
        }
        let bound = InboxSupersedesBinding {
            note_id: predecessor_id,
            source: saved.source.clone(),
        };
        let mut candidate = record.draft.clone();
        candidate
            .inbox_knowledge
            .as_mut()
            .expect("checked Knowledge")
            .supersedes = Some(bound.clone());
        let successor =
            format!("{text}\n\nPrevious version: [History](brn://note/{predecessor_id})\n");
        super::validate_supersession_link(&successor, &bound)?;
        let NoteChange::Create { text, .. } = &mut candidate.changes[0] else {
            unreachable!()
        };
        *text = successor;
        let files = self.editor_files()?;
        if files
            .reserved_copy_path_matches(Path::new(path), Path::new(&request.predecessor_path))
            .map_err(file_error)?
        {
            return Err(stale("predecessor aliases the successor destination"));
        }
        for source in &candidate.sources {
            if files
                .observe(Path::new(&source.path))
                .map_err(file_error)?
                .fingerprint
                != source.fingerprint
            {
                return Err(stale("captured Knowledge context changed"));
            }
        }
        let predecessor_path = Path::new(&request.predecessor_path);
        let history = files
            .coordinate(predecessor_path, || {
                let before = files.observe_uncoordinated(predecessor_path)?;
                Ok((files.parent_identity(predecessor_path)?, before))
            })
            .map_err(file_error)?;
        if history.1.fingerprint != saved.source.fingerprint || history.1.text != saved.text {
            return Err(stale("selected predecessor changed during capture"));
        }
        let history = NoteChange::Replace {
            path: request.predecessor_path.clone(),
            parent: history.0,
            before: saved.source.fingerprint.clone(),
            before_text: saved.text.clone(),
            text: note_metadata::to_history(&saved.text)?,
        };
        // Promotion may change order but never refresh an existing proof.
        if let Some(captured) = candidate
            .sources
            .iter()
            .find(|source| source.path == saved.source.path)
            && captured != &saved.source
        {
            return Err(stale("retained predecessor proof changed"));
        }
        candidate
            .sources
            .retain(|source| source.path != saved.source.path);
        candidate
            .sources
            .insert(usize::from(binding.source.is_some()), saved.source);
        candidate.changes.push(history.clone());
        self.validate_inbox_knowledge(candidate.inbox_knowledge.as_deref())?;
        self.preflight_proposal_targets(&candidate, None)?;
        self.validate_proposal_links(&candidate)?;
        Ok(self
            .store
            .attach_inbox_knowledge_predecessor(request.expected, &history)?)
    }
}
