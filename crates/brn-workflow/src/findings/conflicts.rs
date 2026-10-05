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
}

impl App {
    pub(super) fn capture_conflict_evidence(
        &mut self,
        analysis_id: Uuid,
        source_quote: &FindingQuote,
        other_path: &str,
        other_quote: &FindingQuote,
    ) -> Result<Vec<FindingEvidence>> {
        let job = self
            .store
            .inbox_action(analysis_id)?
            .ok_or_else(|| rejected("Conflict needs a retained Inbox analysis"))?;
        if job.capture.purpose != crate::inbox_actions::InboxAnalysisPurpose::KnowledgeAndActions {
            return Err(rejected("This Inbox analysis does not admit conflicts"));
        }
        let source = self.proposal_evidence_source(&job.capture.source.path)?;
        if source.source != job.capture.source || source.text != job.capture.source_text {
            return Err(stale(
                "Selected Inbox Source changed before conflict capture",
            ));
        }
        let other = self.proposal_evidence_source(other_path)?;
        let inventory = self.identity_inventory()?;
        let mut evidence = Vec::with_capacity(2);
        for (index, (saved, quote)) in [(&source, source_quote), (&other, other_quote)]
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
        let result = NoteConflictPage {
            path: request.path.clone(),
            note_id,
            scope: request.scope,
            source: saved.source,
            entries,
            next_cursor,
            open_count: page.open_count,
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
