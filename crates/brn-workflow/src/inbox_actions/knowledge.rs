//! Exact knowledge preparation and approval guards over the existing proposal.
use super::*;
use crate::{
    editor::file_error,
    knowledge::IdentityOutcome,
    proposal_apply::ApplyJournal,
    proposals::{DraftNoteChange, DraftRequest, NoteChange, ProposalSource},
    vault::VaultPath,
};
use brn_ai::KnowledgeProposalArgs;
use brn_store::work::inbox_actions::InboxSupersedesBinding;
use brn_store::{note_identity, note_metadata, note_provenance};
use sha2::{Digest, Sha256};
use std::path::Path;

fn rejected(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, message)
}
pub(crate) fn validate_supersession_link(text: &str, bound: &InboxSupersedesBinding) -> Result<()> {
    let footer = format!(
        "\n\nPrevious version: [History](brn://note/{})\n",
        bound.note_id
    );
    let Some(prefix) = text.strip_suffix(&footer) else {
        return Err(rejected("knowledge predecessor footer is unavailable"));
    };
    let start = prefix.len() + "\n\nPrevious version: ".len();
    let end = text.len() - 1;
    if !crate::knowledge::stable_link_at(text, bound.note_id, start, end)? {
        return Err(rejected(
            "knowledge predecessor footer must be a readable Markdown link",
        ));
    }
    Ok(())
}

// Hash exact semantic input before adding Rust metadata or observing mutable files.
// Distinct domains separate proposal/note identities within one owned analysis.
fn knowledge_ids(analysis: Uuid, args: &KnowledgeProposalArgs) -> Result<(Uuid, Uuid)> {
    let input =
        serde_json::to_vec(args).map_err(|_| rejected("knowledge intent cannot be encoded"))?;
    let mint = |domain: &[u8]| {
        let mut hash = Sha256::new();
        hash.update(domain);
        hash.update(analysis.as_bytes());
        hash.update(&input);
        let digest = hash.finalize();
        let mut bytes: [u8; 16] = digest[..16].try_into().expect("SHA-256 prefix");
        bytes[6] = (bytes[6] & 0x0f) | 0x80;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Uuid::from_bytes(bytes)
    };
    Ok((
        mint(b"brn/inbox-knowledge-proposal/v1\0"),
        mint(b"brn/inbox-knowledge-note/v1\0"),
    ))
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
        let (id, note_id) = knowledge_ids(job.capture.id, args)?;
        VaultPath::parse(&args.path)
            .map_err(|_| rejected("knowledge destination must be Current"))?;
        if note_identity::read(&args.text)?.is_some()
            || note_metadata::classify(&args.text)? != note_metadata::NoteClassification::default()
            || brn_store::work::inbox_source::read_provenance(&args.text)?.is_some()
            || !note_provenance::read(&args.text)?.is_empty()
        {
            return Err(rejected(
                "knowledge candidate cannot invent identity, Source/provenance or History authority",
            ));
        }
        let source_id = job.capture.note_id()?;
        let mut citations = Vec::new();
        let mut intake_citations = Vec::new();
        let private_snapshot = job
            .capture
            .intake
            .as_ref()
            .map(|intake| {
                self.store
                    .intake_snapshot(intake.snapshot_id)?
                    .ok_or_else(|| rejected("private extraction is unavailable"))
            })
            .transpose()?;
        let body = note_identity::body_start(&job.capture.source_text)?;
        for selection in &args.quotes {
            let range = crate::quote_selection::resolve(
                &job.capture.source_text,
                &selection.quote,
                selection.occurrence,
            )?;
            if let Some(snapshot) = &private_snapshot {
                intake_citations.push(
                    brn_store::work::inbox_actions::IntakeCitation::resolve_for_source(
                        snapshot,
                        job.capture.note_id()?,
                        range.start.checked_sub(body).ok_or_else(|| {
                            rejected("private quote cannot cite wrapper metadata")
                        })?,
                        range.end.checked_sub(body).ok_or_else(|| {
                            rejected("private quote cannot cite wrapper metadata")
                        })?,
                        selection.source_id.as_deref(),
                    )
                    .map_err(|error| rejected(&error.to_string()))?,
                );
            } else if selection.source_id.is_some() {
                return Err(rejected(
                    "source_id selects a private extraction node; saved Source quotations use body occurrences",
                ));
            }
            citations.push(note_provenance::VaultCitation {
                note_id: source_id,
                sha256: job.capture.source_sha256(),
                start_byte: range.start,
                end_byte: range.end,
                quote: selection.quote.clone(),
            });
        }
        note_provenance::validate(&citations)?;
        let existing = self.store.proposal(id)?;
        let predecessor = match (&args.supersedes, &existing) {
            (Some(path), Some(record)) => {
                let bound = record
                    .draft
                    .inbox_knowledge
                    .as_ref()
                    .and_then(|binding| binding.supersedes.as_ref())
                    .filter(|bound| &bound.source.path == path)
                    .ok_or_else(|| rejected("knowledge creation predecessor changed"))?;
                let Some(NoteChange::Replace { before_text, .. }) = record.draft.changes.get(1)
                else {
                    return Err(rejected("knowledge predecessor baseline is unavailable"));
                };
                Some(ProposalSource {
                    source: bound.source.clone(),
                    text: before_text.clone(),
                })
            }
            (Some(path), None) => Some(self.proposal_source(path)?),
            (None, Some(record))
                if record
                    .draft
                    .inbox_knowledge
                    .as_ref()
                    .is_some_and(|binding| binding.supersedes.is_some()) =>
            {
                return Err(rejected("knowledge creation predecessor changed"));
            }
            (None, _) => None,
        };
        let supersedes = predecessor
            .as_ref()
            .map(|saved| -> Result<_> {
                let old_id = note_identity::read(&saved.text)?
                    .ok_or_else(|| rejected("predecessor needs a saved managed identity"))?;
                if old_id == note_id
                    || old_id == source_id
                    || note_metadata::classify(&saved.text)?
                        != note_metadata::NoteClassification::default()
                    || brn_store::work::inbox_source::read_provenance(&saved.text)?.is_some()
                {
                    return Err(rejected("predecessor must be separate Current knowledge"));
                }
                Ok(InboxSupersedesBinding {
                    note_id: old_id,
                    source: saved.source.clone(),
                })
            })
            .transpose()?;
        let mut text =
            note_provenance::write(&note_identity::assign(&args.text, note_id)?, &citations)?;
        if let Some(bound) = &supersedes {
            text.push_str(&format!(
                "\n\nPrevious version: [History](brn://note/{})\n",
                bound.note_id
            ));
            validate_supersession_link(&text, bound)?;
        }
        let sources = match existing {
            Some(existing) => {
                if job
                    .capture
                    .source
                    .as_ref()
                    .is_some_and(|source| existing.draft.sources.first() != Some(source))
                    || supersedes
                        .as_ref()
                        .is_some_and(|bound| existing.draft.sources.get(1) != Some(&bound.source))
                    || !args.source_paths.iter().map(String::as_str).eq(existing
                        .draft
                        .sources
                        .iter()
                        .skip(
                            usize::from(job.capture.source.is_some())
                                + usize::from(supersedes.is_some()),
                        )
                        .map(|s| s.path.as_str()))
                {
                    return Err(rejected("knowledge creation target paths changed"));
                }
                existing.draft.sources
            }
            None => {
                let mut sources = job.capture.source.iter().cloned().collect::<Vec<_>>();
                if let Some(bound) = &supersedes {
                    sources.push(bound.source.clone());
                }
                for path in &args.source_paths {
                    sources.push(self.proposal_evidence_source(path)?.source);
                }
                sources
            }
        };
        let mut changes = vec![DraftNoteChange::Create {
            path: args.path.clone(),
            text,
        }];
        if let Some(saved) = predecessor {
            changes.push(DraftNoteChange::Replace {
                path: saved.source.path,
                expected: saved.source.fingerprint,
                text: note_metadata::to_history(&saved.text)?,
            });
        }
        let request = DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: Some(Box::new(InboxKnowledgeBinding {
                supersedes,
                analysis_id: job.capture.id,
                note_id,
                source: job.capture.source.clone(),
                intake: job.capture.intake.clone().map(Box::new),
                intake_citations,
                citations,
            })),
            inbox_source: None,
            id,
            group_id: Some(job.capture.id),
            session_id: Some(session),
            title: args.title.clone(),
            changes,
            sources,
            action_changes: vec![],
        };
        request.validate()?;
        Ok(request)
    }

    /// A bound consequence retains the full Source and exact Current predecessor.
    /// Review cannot expand this into other replacements or deletion.
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
        if let Some(bound) = &binding.supersedes {
            let saved = self.proposal_source(&bound.source.path)?;
            let identity = self.resolve_note_identity(bound.note_id)?;
            if saved.source != bound.source
                || note_identity::read(&saved.text)? != Some(bound.note_id)
                || identity.outcome != IdentityOutcome::Unique
                || identity.matches[0].path != bound.source.path
            {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "knowledge predecessor changed or is ambiguous",
                ));
            }
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
            || job.capture.intake.as_ref() != binding.intake.as_deref()
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
        self.validate_intake_dependency(binding.intake.as_deref(), true)?;
        let files = self.editor.files.as_ref().ok_or_else(|| {
            WorkflowError::typed(ErrorKind::ContextStale, "knowledge files are unavailable")
        })?;
        let source = files
            .observe(Path::new(job.capture.source_path()))
            .map_err(file_error)?;
        if binding
            .source
            .as_ref()
            .is_some_and(|bound| source.fingerprint != bound.fingerprint)
            || source.fingerprint.sha256 != job.capture.source_sha256()
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
            || source_identity.matches[0].path != job.capture.source_path()
            || source_identity.matches[0].sha256 != job.capture.source_sha256()
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
        let link_ids = crate::knowledge::stable_link_ids(text)?;
        if let Some(bound) = &binding.supersedes {
            validate_supersession_link(text, bound)?;
            let target = inventory.resolution(bound.note_id);
            if target.outcome != IdentityOutcome::Unique
                || target.matches[0].path != bound.source.path
            {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "knowledge predecessor identity or relationship changed",
                ));
            }
            let observed = files
                .observe(Path::new(&bound.source.path))
                .map_err(file_error)?;
            let own_history = journal.prepared.as_ref().and_then(|proofs| proofs.get(1));
            if observed.fingerprint != bound.source.fingerprint
                && own_history != Some(&observed.fingerprint)
            {
                return Err(WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "knowledge predecessor is no longer its exact before or prepared History object",
                ));
            }
        }
        for id in link_ids {
            if id == binding.note_id
                || binding
                    .intake
                    .as_ref()
                    .is_some_and(|intake| intake.source_note_id == id)
                || binding
                    .supersedes
                    .as_ref()
                    .is_some_and(|bound| bound.note_id == id)
            {
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

#[cfg(test)]
mod tests {
    use super::*;
    use brn_ai::KnowledgeQuoteArgs;

    #[test]
    fn identities_bind_exact_ordered_semantic_intent_and_analysis_with_distinct_domains() {
        let analysis = Uuid::from_u128(1);
        let args = KnowledgeProposalArgs {
            supersedes: Some("previous.md".into()),
            title: "Interpretation õ".into(),
            path: "current.md".into(),
            text: "\u{feff}# Candidate 🦀\r\nExact bytes\r\n".into(),
            quotes: vec![
                KnowledgeQuoteArgs {
                    source_id: None,
                    quote: "First õ".into(),
                    occurrence: Some(1),
                },
                KnowledgeQuoteArgs {
                    source_id: None,
                    quote: "Second 🦀".into(),
                    occurrence: None,
                },
            ],
            source_paths: vec!["person.md".into(), "project.md".into()],
        };
        let original = knowledge_ids(analysis, &args).unwrap();
        assert_eq!(knowledge_ids(analysis, &args).unwrap(), original);
        assert_ne!(original.0, original.1);
        for id in [original.0, original.1] {
            assert_eq!(id.get_version_num(), 8);
            assert_eq!(id.get_variant(), uuid::Variant::RFC4122);
        }
        let mut identities = vec![original];
        for mode in 0..11 {
            let mut changed = args.clone();
            match mode {
                0 => changed.title.push(' '),
                1 => changed.text = changed.text.replace("\r\n", "\n"),
                2 => changed.path = "another.md".into(),
                3 => changed.quotes[0].quote.push(' '),
                4 => changed.quotes[0].occurrence = Some(2),
                5 => changed.quotes[0].occurrence = None,
                6 => changed.supersedes = Some("other.md".into()),
                7 => changed.supersedes = None,
                8 => changed.source_paths.swap(0, 1),
                9 => changed.quotes.swap(0, 1),
                _ => changed.quotes[0].source_id = Some("source-docx".into()),
            }
            let new = knowledge_ids(analysis, &changed).unwrap();
            assert!(
                identities
                    .iter()
                    .all(|old| old.0 != new.0 && old.1 != new.1),
                "mode {mode}"
            );
            identities.push(new);
        }
        let other_analysis = knowledge_ids(Uuid::from_u128(2), &args).unwrap();
        assert!(
            identities
                .iter()
                .all(|old| old.0 != other_analysis.0 && old.1 != other_analysis.1)
        );
    }
}
