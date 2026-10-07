//! Durable citations are ordinary reviewed Markdown, not session-owned records.
use super::{IdentityInventory, IdentityIssue, IdentityOutcome, NoteIdentityInfo, rejected};
use crate::{
    ErrorKind, MAX_NOTE_BYTES, Result, WorkflowError,
    app::App,
    proposals::{DraftNoteChange, DraftRequest, NoteChange, ProposalDraft, SourceVersion},
    vault::{self, EvidencePath, VaultPath},
};
use brn_store::{note_identity, note_provenance};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use brn_store::note_provenance::VaultCitation;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationRequest {
    pub note_id: Uuid,
    pub expected_sha256: [u8; 32],
    pub start_byte: usize,
    pub end_byte: usize,
}
impl CitationRequest {
    pub fn validate(&self) -> Result<()> {
        if self.note_id.is_nil()
            || self.start_byte >= self.end_byte
            || self.end_byte > MAX_NOTE_BYTES
            || self.end_byte - self.start_byte > note_provenance::MAX_QUOTE_BYTES
        {
            return Err(rejected(
                "Citation capture needs a nonnil UUID and a bounded nonempty saved byte range.",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationCapture {
    pub citation: VaultCitation,
    pub source: SourceVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceRequest {
    pub path: String,
    pub proposal_id: Uuid,
    pub title: String,
    /// Additive: existing citations remain exact, including stale evidence.
    pub citations: Vec<VaultCitation>,
}
impl ProvenanceRequest {
    pub fn validate(&self) -> Result<()> {
        VaultPath::parse(&self.path).map_err(|error| rejected(error.to_string()))?;
        if self.proposal_id.is_nil()
            || self.title.trim().is_empty()
            || self.title.len() > 512
            || self.citations.is_empty()
        {
            return Err(rejected(
                "Provenance preparation needs a nonnil proposal UUID, title and citations.",
            ));
        }
        note_provenance::validate(&self.citations).map_err(|error| rejected(error.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationOutcome {
    Matched,
    Changed,
    Absent,
    Ambiguous,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedCitation {
    pub citation: VaultCitation,
    pub outcome: CitationOutcome,
    pub matches: Vec<NoteIdentityInfo>,
    pub issues: Vec<IdentityIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteProvenance {
    pub path: String,
    pub citations: Vec<ResolvedCitation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox_source: Option<brn_store::work::inbox_source::InboxSourceProvenance>,
}

fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}

impl App {
    /// Fresh identity lookup plus coordinated full saved source observation.
    /// The returned path/fingerprint is proposal proof, not durable identity.
    pub fn capture_citation(&mut self, request: &CitationRequest) -> Result<CitationCapture> {
        request.validate()?;
        let inventory = self.identity_inventory()?;
        self.capture_from_inventory(request, &inventory)
    }

    fn capture_from_inventory(
        &mut self,
        request: &CitationRequest,
        inventory: &IdentityInventory,
    ) -> Result<CitationCapture> {
        let resolution = inventory.resolution(request.note_id);
        if resolution.outcome != IdentityOutcome::Unique {
            return Err(rejected(
                "Citation source identity is absent, ambiguous or incompletely inspected.",
            ));
        }
        let matched = &resolution.matches[0];
        if matched.sha256 != request.expected_sha256 {
            return Err(stale("Citation source content changed."));
        }
        let observed = self.proposal_evidence_source(&matched.path)?;
        if observed.source.fingerprint.sha256 != request.expected_sha256
            || note_identity::read(&observed.text).map_err(|error| rejected(error.to_string()))?
                != Some(request.note_id)
        {
            return Err(stale("Citation source changed during capture."));
        }
        let quote = observed
            .text
            .get(request.start_byte..request.end_byte)
            .ok_or_else(|| {
                rejected("Citation range is outside saved text or splits a UTF-8 character.")
            })?;
        let citation = VaultCitation {
            note_id: request.note_id,
            sha256: request.expected_sha256,
            start_byte: request.start_byte,
            end_byte: request.end_byte,
            quote: quote.into(),
        };
        citation
            .validate()
            .map_err(|error| rejected(error.to_string()))?;
        Ok(CitationCapture {
            citation,
            source: observed.source,
        })
    }

    /// Inspection never substitutes a later version or a guessed path. Saved
    /// quotes stay available even when their sources changed or disappeared.
    pub fn note_provenance(&self, path: &str) -> Result<NoteProvenance> {
        let note = self.evidence_note(path)?;
        let citations =
            note_provenance::read(&note.text).map_err(|error| rejected(error.to_string()))?;
        let inventory = if citations.is_empty() {
            None
        } else {
            Some(self.identity_inventory()?)
        };
        let root = self.require_vault()?;
        let mut resolved = Vec::with_capacity(citations.len());
        for citation in citations {
            let resolution = inventory
                .as_ref()
                .expect("citations inspected")
                .resolution(citation.note_id);
            let mut issues = resolution.issues;
            let outcome = match resolution.outcome {
                IdentityOutcome::Absent => CitationOutcome::Absent,
                IdentityOutcome::Ambiguous => CitationOutcome::Ambiguous,
                IdentityOutcome::Incomplete => CitationOutcome::Incomplete,
                IdentityOutcome::Unique => {
                    let matched = &resolution.matches[0];
                    let observed = EvidencePath::parse(&matched.path)
                        .map_err(|error| error.to_string())
                        .and_then(|path| {
                            vault::read_evidence(root, &path).map_err(|error| error.to_string())
                        });
                    match observed {
                        Ok(note)
                            if note.sha256 == citation.sha256
                                && note_identity::read(&note.text).ok().flatten()
                                    == Some(citation.note_id)
                                && note.text.get(citation.start_byte..citation.end_byte)
                                    == Some(citation.quote.as_str()) =>
                        {
                            CitationOutcome::Matched
                        }
                        Ok(_) => CitationOutcome::Changed,
                        Err(reason) => {
                            issues.push(IdentityIssue {
                                path: matched.path.clone(),
                                reason,
                            });
                            CitationOutcome::Incomplete
                        }
                    }
                }
            };
            resolved.push(ResolvedCitation {
                citation,
                outcome,
                matches: resolution.matches,
                issues,
            });
        }
        Ok(NoteProvenance {
            inbox_source: brn_store::work::inbox_source::read_provenance(&note.text)?,
            path: path.into(),
            citations: resolved,
        })
    }

    /// Produces complete additive review input, without admitting review work.
    pub fn prepare_note_provenance(&mut self, request: &ProvenanceRequest) -> Result<DraftRequest> {
        request.validate()?;
        let target = self.proposal_source(&request.path)?;
        let mut citations =
            note_provenance::read(&target.text).map_err(|error| rejected(error.to_string()))?;
        let inventory = self.identity_inventory()?;
        let mut sources = vec![target.source.clone()];
        let mut added = false;
        for citation in &request.citations {
            if citations.contains(citation) {
                continue;
            }
            let capture = self.capture_from_inventory(
                &CitationRequest {
                    note_id: citation.note_id,
                    expected_sha256: citation.sha256,
                    start_byte: citation.start_byte,
                    end_byte: citation.end_byte,
                },
                &inventory,
            )?;
            if capture.citation != *citation {
                return Err(stale(
                    "Citation quote does not match the exact saved source.",
                ));
            }
            if capture.source.fingerprint.device == target.source.fingerprint.device
                && capture.source.fingerprint.inode == target.source.fingerprint.inode
            {
                return Err(rejected("Capture provenance from a separate source note."));
            }
            if !sources
                .iter()
                .any(|source| source.path == capture.source.path)
            {
                sources.push(capture.source);
            }
            citations.push(citation.clone());
            added = true;
        }
        if !added {
            return Err(rejected("All requested citations are already present."));
        }
        let text = note_provenance::write(&target.text, &citations)
            .map_err(|error| rejected(error.to_string()))?;
        let draft = DraftRequest {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: Vec::new(),
            id: request.proposal_id,
            group_id: None,
            session_id: None,
            title: request.title.clone(),
            changes: vec![DraftNoteChange::Replace {
                path: request.path.clone(),
                expected: target.source.fingerprint,
                text,
            }],
            sources,
        };
        draft.validate()?;
        Ok(draft)
    }

    /// Run before fresh ordinary approval. Unchanged prior citations do not need
    /// their historical sources to remain current, and exact Undo bypasses this.
    pub(crate) fn validate_proposal_provenance(&mut self, draft: &ProposalDraft) -> Result<()> {
        let mut added = Vec::new();
        for change in &draft.changes {
            let Some(text) = change.text() else {
                continue;
            };
            let before = match change {
                NoteChange::Replace { before_text, .. } => before_text.as_str(),
                _ => "",
            };
            if text == before {
                continue;
            }
            let inbox = brn_store::work::inbox_source::read_provenance(text)?;
            let previous_inbox =
                brn_store::work::inbox_source::read_provenance(before).unwrap_or_default();
            if inbox.is_some()
                && inbox != previous_inbox
                && !draft
                    .inbox_source
                    .as_ref()
                    .is_some_and(|binding| Some(binding.provenance()) == inbox)
            {
                return Err(rejected(
                    "New Inbox provenance requires its exact original conversion binding.",
                ));
            }
            let citations =
                note_provenance::read(text).map_err(|error| rejected(error.to_string()))?;
            // Explicit repair may replace invalid old managed metadata. It never
            // supplies an assumed trusted citation from the malformed baseline.
            let previous = note_provenance::read(before).unwrap_or_default();
            for citation in citations {
                if !previous.contains(&citation) && !added.contains(&citation) {
                    added.push(citation);
                }
            }
        }
        if added.is_empty() {
            return Ok(());
        }
        let inventory = self.identity_inventory()?;
        for citation in added {
            let capture = self.capture_from_inventory(
                &CitationRequest {
                    note_id: citation.note_id,
                    expected_sha256: citation.sha256,
                    start_byte: citation.start_byte,
                    end_byte: citation.end_byte,
                },
                &inventory,
            )?;
            if capture.citation != citation {
                return Err(stale(
                    "New provenance quote does not match saved source evidence.",
                ));
            }
            let private = crate::intake_dependencies::dependency(draft).is_some_and(|binding| {
                binding.source_note_id == citation.note_id
                    && binding.source_text_sha256 == citation.sha256
                    && binding.source_path == capture.source.path
            });
            if !draft.sources.contains(&capture.source) && !private {
                return Err(stale(
                    "New provenance needs its exact captured source binding.",
                ));
            }
        }
        Ok(())
    }
}
