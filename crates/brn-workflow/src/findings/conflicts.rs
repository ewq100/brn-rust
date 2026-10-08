//! Exact tentative conflict capture and fresh, query-bound read observations.
use super::*;
use crate::{knowledge::IdentityOutcome, library::KnowledgeScope, proposals::SourceVersion};

fn default_limit() -> usize {
    10
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteConflictCursor {
    pub vault_id: Uuid,
    pub path: String,
    pub note_id: Uuid,
    pub scope: KnowledgeScope,
    pub source: SourceVersion,
    pub before: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteConflictRequest {
    pub path: String,
    #[serde(default)]
    pub scope: KnowledgeScope,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub cursor: Option<NoteConflictCursor>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteConflictPage {
    pub path: String,
    pub note_id: Uuid,
    pub scope: KnowledgeScope,
    pub source: SourceVersion,
    pub entries: Vec<FindingInspection>,
    pub next_cursor: Option<NoteConflictCursor>,
    pub open_count: usize,
    /// Known retained count bound to this page's exact managed identity/source proof.
    pub facts: brn_ai::NoteFacts,
}

impl App {
    pub(super) fn capture_conflict_evidence(
        &mut self,
        analysis_id: Uuid,
        source_quote: &FindingQuote,
        other_path: &str,
        other_quote: &FindingQuote,
    ) -> Result<Vec<FindingEvidence>> {
        let (source, other) = self.conflict_sources(analysis_id, other_path)?;
        self.conflict_evidence(&source, &other, source_quote, other_quote)
    }

    fn conflict_sources(
        &mut self,
        analysis_id: Uuid,
        other_path: &str,
    ) -> Result<(ProposalSource, ProposalSource)> {
        let job = self
            .store
            .inbox_action(analysis_id)?
            .ok_or_else(|| rejected("Conflict needs a retained Inbox analysis"))?;
        if job.capture.purpose != crate::inbox_actions::InboxAnalysisPurpose::KnowledgeAndActions {
            return Err(rejected("This Inbox analysis does not admit conflicts"));
        }
        self.editor_files()?;
        self.validate_intake_dependency(job.capture.intake.as_ref(), true)?;
        let selected = self.store.inbox_conflict_source(analysis_id)?;
        let source = self.proposal_evidence_source(&selected.path)?;
        if source.source != selected || source.text != job.capture.source_text {
            return Err(stale(
                "Selected Inbox Source changed before conflict capture",
            ));
        }
        let other = self.proposal_evidence_source(other_path)?;
        Ok((source, other))
    }

    fn conflict_evidence(
        &mut self,
        source: &ProposalSource,
        other: &ProposalSource,
        source_quote: &FindingQuote,
        other_quote: &FindingQuote,
    ) -> Result<Vec<FindingEvidence>> {
        let inventory = self.identity_inventory()?;
        let mut evidence = Vec::with_capacity(2);
        for (index, (saved, quote)) in [(source, source_quote), (other, other_quote)]
            .into_iter()
            .enumerate()
        {
            let metadata = crate::library::saved_metadata(&saved.text, &saved.source.path);
            if metadata.issue.is_some() {
                return Err(rejected("Conflict side has malformed managed metadata"));
            }
            if metadata.history || index == 0 && !metadata.source {
                return Err(rejected(
                    "Conflict needs nonhistorical saved Source/Current evidence",
                ));
            }
            let id = saved_id(saved)?
                .ok_or_else(|| rejected("Conflict side needs a managed identity"))?;
            let identity = inventory.resolution(id);
            if identity.outcome != IdentityOutcome::Unique
                || identity.matches[0].path != saved.source.path
                || identity.matches[0].sha256 != saved.source.fingerprint.sha256
            {
                return Err(stale(
                    "Conflict identity is ambiguous, incomplete or changed",
                ));
            }
            let body = note_identity::body_start(&saved.text)?;
            if quote.start_byte < body
                || saved.text.get(quote.start_byte..quote.end_byte) != Some(quote.quote.as_str())
            {
                return Err(stale(
                    "Conflict quotation must match the exact saved body bytes",
                ));
            }
            evidence.push(FindingEvidence {
                source: saved.source.clone(),
                note_id: Some(id),
                quote: Some(quote.clone()),
            });
        }
        if evidence[0].source.path == evidence[1].source.path
            || evidence[0].note_id == evidence[1].note_id
        {
            return Err(rejected(
                "Conflict needs two distinct saved identities and paths",
            ));
        }
        Ok(evidence)
    }

    /// AI selects wording; Rust mints its ranges against one captured full proof.
    /// Retain original replay before observing current files, including closure.
    pub(crate) fn capture_selected_conflict(
        &mut self,
        analysis_id: Uuid,
        id: Uuid,
        args: &brn_ai::ConflictArgs,
    ) -> Result<FindingRecord> {
        let job = self
            .store
            .inbox_action(analysis_id)?
            .ok_or_else(|| rejected("Conflict needs a retained Inbox analysis"))?;
        let source_range = crate::quote_selection::resolve(
            &job.capture.source_text,
            &args.source_quote.quote,
            args.source_quote.occurrence,
        )?;
        let source_quote = FindingQuote {
            start_byte: source_range.start,
            end_byte: source_range.end,
            quote: args.source_quote.quote.clone(),
        };
        if let Some(record) = self.store.finding(id)? {
            let matches = matches!(&record.draft.request.origin, FindingOrigin::InboxConflict { analysis_id: stored_analysis, title, summary, source_quote: stored_source, other_path, other_quote }
                if *stored_analysis == analysis_id && title == &args.title && summary == &args.summary && stored_source == &source_quote && other_path == &args.other_path && other_quote.quote == args.other_quote.quote);
            return if matches {
                Ok(record)
            } else {
                Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "Conflict identity has another capture request",
                ))
            };
        }
        self.require_current_evidence()?;
        let (source, other) = self.conflict_sources(analysis_id, &args.other_path)?;
        let other_range = crate::quote_selection::resolve(
            &other.text,
            &args.other_quote.quote,
            args.other_quote.occurrence,
        )?;
        let other_quote = FindingQuote {
            start_byte: other_range.start,
            end_byte: other_range.end,
            quote: args.other_quote.quote.clone(),
        };
        let evidence = self.conflict_evidence(&source, &other, &source_quote, &other_quote)?;
        let request = CaptureFindingRequest {
            id,
            origin: FindingOrigin::InboxConflict {
                analysis_id,
                title: args.title.clone(),
                summary: args.summary.clone(),
                source_quote,
                other_path: args.other_path.clone(),
                other_quote,
            },
        };
        request.validate().map_err(|e| rejected(e.to_string()))?;
        let vault = self.finding_vault()?;
        self.retain_finding(FindingDraft {
            request,
            vault,
            title: args.title.clone(),
            summary: args.summary.clone(),
            evidence,
        })
    }

    /// Complete retained conflicts and separate fresh evidence. No winner is
    /// inferred from a closed finding, changed proof or an incomplete page.
    pub fn note_conflicts(&mut self, request: &NoteConflictRequest) -> Result<NoteConflictPage> {
        if !(1..=100).contains(&request.limit)
            || request.path.is_empty()
            || request.path.len() > 512
        {
            return Err(rejected("Conflict lookup needs a bounded path and page"));
        }
        self.require_current_evidence()?;
        let saved = self.proposal_evidence_source(&request.path)?;
        let metadata = crate::library::saved_metadata(&saved.text, &saved.source.path);
        if metadata.issue.is_some() {
            return Err(rejected(
                "Conflict lookup note has malformed managed metadata",
            ));
        }
        if !request.scope.includes(metadata.source, metadata.history) {
            return Err(rejected(
                "Conflict lookup note is outside the requested scope",
            ));
        }
        let note_id = saved_id(&saved)?
            .ok_or_else(|| rejected("Conflict lookup needs a managed saved note"))?;
        let inventory = self.identity_inventory()?;
        let resolution = inventory.resolution(note_id);
        if resolution.outcome != IdentityOutcome::Unique
            || resolution.matches[0].path != request.path
            || resolution.matches[0].sha256 != saved.source.fingerprint.sha256
        {
            return Err(stale(
                "Conflict lookup identity is ambiguous, incomplete or changed",
            ));
        }
        let vault = self.finding_vault()?;
        if let Some(cursor) = &request.cursor
            && (cursor.vault_id != vault.id
                || cursor.path != request.path
                || cursor.note_id != note_id
                || cursor.scope != request.scope
                || cursor.source != saved.source)
        {
            return Err(stale(
                "Conflict cursor belongs to a different query or saved version",
            ));
        }
        let page = self.store.note_conflicts(
            &vault,
            &request.path,
            note_id,
            &FindingListRequest {
                state: Some(FindingState::Open),
                limit: request.limit,
                before: request.cursor.as_ref().map(|c| c.before),
            },
        )?;
        let mut entries = Vec::with_capacity(page.entries.len());
        for record in page.entries {
            entries.push(self.inspect_finding(record.draft.request.id)?);
        }
        if self.proposal_evidence_source(&request.path)?.source != saved.source {
            return Err(stale("Conflict lookup note changed during inspection"));
        }
        let final_inventory = self.identity_inventory()?;
        let final_resolution = final_inventory.resolution(note_id);
        if final_resolution.outcome != IdentityOutcome::Unique
            || final_resolution.matches[0].path != request.path
            || final_resolution.matches[0].sha256 != saved.source.fingerprint.sha256
        {
            return Err(stale("Conflict lookup identity changed during inspection"));
        }
        self.require_current_evidence()?;
        let next_cursor = page.next_before.map(|before| NoteConflictCursor {
            vault_id: vault.id,
            path: request.path.clone(),
            note_id,
            scope: request.scope,
            source: saved.source.clone(),
            before,
        });
        let facts = brn_ai::NoteFacts {
            note_id: Some(note_id.to_string()),
            sha256: saved.source.fingerprint.sha256,
            source: metadata.source,
            history: metadata.history,
            conflicts: brn_ai::ConflictKnowledge::Known {
                open_count: page.open_count,
            },
        };
        let result = NoteConflictPage {
            path: request.path.clone(),
            note_id,
            scope: request.scope,
            source: saved.source,
            entries,
            next_cursor,
            open_count: page.open_count,
            facts,
        };
        if serde_json::to_vec(&result)
            .map_err(|_| rejected("Conflict page could not be encoded"))?
            .len()
            > brn_ai::READ_ACTION_BYTES
        {
            return Err(rejected(
                "Complete conflict page exceeds its output limit; request fewer entries",
            ));
        }
        Ok(result)
    }
}
