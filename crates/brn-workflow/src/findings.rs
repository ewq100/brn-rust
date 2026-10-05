//! Tentative operational findings; exact proposals still own knowledge changes.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    knowledge::{NoteLinkOutcome, note_identity},
    proposals::ProposalSource,
};
use brn_store::files::VaultRecord;
pub use brn_store::work::findings::{
    CaptureFindingRequest, CloseFindingRequest, FindingDraft, FindingEvidence, FindingListRequest,
    FindingOrigin, FindingPage, FindingQuote, FindingRecord, FindingStamp, FindingState,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod conflicts;
pub use conflicts::{NoteConflictCursor, NoteConflictPage, NoteConflictRequest};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingEvidenceOutcome {
    Unchanged,
    Changed,
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingEvidenceObservation {
    pub index: usize,
    pub outcome: FindingEvidenceOutcome,
    pub observed: Option<crate::proposals::SourceVersion>,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingInspection {
    pub record: FindingRecord,
    pub evidence: Vec<FindingEvidenceObservation>,
}
impl App {
    /// Retains a tentative issue and exact saved proof. This neither creates a
    /// proposal nor changes knowledge. Immutable request replay is history-only.
    pub fn capture_finding(&mut self, request: &CaptureFindingRequest) -> Result<FindingRecord> {
        request
            .validate()
            .map_err(|error| rejected(error.to_string()))?;
        if let Some(record) = self.store.finding(request.id)? {
            return if record.draft.request == *request {
                Ok(record)
            } else {
                Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "finding UUID has another capture request",
                ))
            };
        }
        self.require_current_evidence()?;
        let (title, summary, evidence) = match &request.origin {
            FindingOrigin::IdentityAmbiguity { note_id } => {
                let inventory = self.identity_inventory()?;
                let matches = inventory
                    .notes
                    .iter()
                    .filter(|note| note.note_id == Some(*note_id))
                    .take(2)
                    .collect::<Vec<_>>();
                if matches.len() != 2 || matches[0].path == matches[1].path {
                    return Err(rejected(
                        "Identity finding needs at least two observed distinct saved paths with the same UUID.",
                    ));
                }
                let mut evidence = Vec::with_capacity(2);
                for observed in matches {
                    let saved = self.proposal_evidence_source(&observed.path)?;
                    if saved.source.fingerprint.sha256 != observed.sha256
                        || saved_id(&saved)? != Some(*note_id)
                    {
                        return Err(stale("Identity evidence changed during capture."));
                    }
                    // Capture a UTF-8 prefix, never cut a scalar or fabricate a
                    // quote for an empty saved note.
                    let mut end = saved.text.len().min(16 * 1024);
                    while !saved.text.is_char_boundary(end) {
                        end -= 1;
                    }
                    let quote = (end > 0).then(|| FindingQuote {
                        start_byte: 0,
                        end_byte: end,
                        quote: saved.text[..end].into(),
                    });
                    evidence.push(FindingEvidence {
                        source: saved.source,
                        note_id: Some(*note_id),
                        quote,
                    });
                }
                (
                    "Ambiguous note identity".into(),
                    format!(
                        "At least two distinct saved paths use note UUID {note_id}. Inspect the retained sources before proposing a correction."
                    ),
                    evidence,
                )
            }
            FindingOrigin::InboxConflict {
                analysis_id,
                title,
                summary,
                source_quote,
                other_path,
                other_quote,
            } => (
                title.clone(),
                summary.clone(),
                self.capture_conflict_evidence(
                    *analysis_id,
                    source_quote,
                    other_path,
                    other_quote,
                )?,
            ),
            FindingOrigin::UnresolvedLink {
                path,
                source_sha256,
                destination,
                start_byte,
            } => {
                let links = self.note_links(path)?;
                if links.source.sha256 != *source_sha256 {
                    return Err(stale("Saved link source changed before capture."));
                }
                let link = links
                    .links
                    .iter()
                    .find(|link| {
                        link.destination == *destination
                            && link
                                .evidence
                                .first()
                                .is_some_and(|proof| proof.start_byte == *start_byte)
                    })
                    .ok_or_else(|| {
                        stale("The exact saved link occurrence is no longer present.")
                    })?;
                if matches!(
                    link.outcome,
                    NoteLinkOutcome::Resolved
                        | NoteLinkOutcome::External
                        | NoteLinkOutcome::NonNote
                ) {
                    return Err(rejected(
                        "This saved link is not an unresolved note-link finding.",
                    ));
                }
                let saved = self.proposal_evidence_source(path)?;
                if saved.source.fingerprint.sha256 != *source_sha256 {
                    return Err(stale("Saved link source changed during capture."));
                }
                let note_id = saved_id(&saved)?;
                let mut evidence = Vec::with_capacity(link.evidence.len());
                for quote in &link.evidence {
                    if saved.text.get(quote.start_byte..quote.end_byte)
                        != Some(quote.quote.as_str())
                    {
                        return Err(stale(
                            "Saved link quote no longer matches the exact source bytes.",
                        ));
                    }
                    evidence.push(FindingEvidence {
                        source: saved.source.clone(),
                        note_id,
                        quote: Some(FindingQuote {
                            start_byte: quote.start_byte,
                            end_byte: quote.end_byte,
                            quote: quote.quote.clone(),
                        }),
                    });
                }
                (
                    "Unresolved saved note link".into(),
                    format!(
                        "The saved note link was observed as {:?}. Its original occurrence and used definition remain evidence; resolving this finding does not correct Markdown.",
                        link.outcome
                    ),
                    evidence,
                )
            }
        };
        let vault = self.finding_vault()?;
        let draft = FindingDraft {
            request: request.clone(),
            vault,
            title,
            summary,
            evidence,
        };
        self.retain_finding(draft)
    }
    pub(super) fn retain_finding(&mut self, draft: FindingDraft) -> Result<FindingRecord> {
        draft
            .validate()
            .map_err(|error| rejected(error.to_string()))?;
        // Multiple reference proofs can share one saved file. Reobserve each
        // distinct full proof, including identity, after all captures.
        let mut checked = std::collections::HashSet::new();
        for evidence in &draft.evidence {
            if checked.insert(&evidence.source.path)
                && self.proposal_evidence_source(&evidence.source.path)?.source != evidence.source
            {
                return Err(stale("Finding source changed before retention."));
            }
        }
        self.require_current_evidence()?;
        if matches!(draft.request.origin, FindingOrigin::InboxConflict { .. }) {
            // Bound the entire future receipt before retention, including the
            // widest supported replay stamp/timestamps and terminal state.
            let maximum = FindingRecord {
                draft: draft.clone(),
                version: u64::MAX,
                state: FindingState::Dismissed,
                created_at_ms: u64::MAX,
                updated_at_ms: u64::MAX,
            };
            if serde_json::to_vec(&maximum)
                .map_err(|_| rejected("Conflict receipt could not be encoded"))?
                .len()
                > brn_ai::READ_ACTION_BYTES
            {
                return Err(rejected(
                    "Complete conflict receipt exceeds its output limit",
                ));
            }
        }
        Ok(self.store.create_finding(&draft)?)
    }
    pub fn findings(&self, request: &FindingListRequest) -> Result<FindingPage> {
        Ok(self.store.findings(request)?)
    }
    pub fn finding(&self, id: Uuid) -> Result<FindingRecord> {
        self.store
            .finding(id)?
            .ok_or_else(|| WorkflowError::typed(ErrorKind::NotFound, "Finding does not exist"))
    }
    pub fn close_finding(&mut self, request: &CloseFindingRequest) -> Result<FindingRecord> {
        Ok(self.store.close_finding(request)?)
    }
    /// Reports fresh proof separately from immutable historical evidence. Even
    /// unchanged bytes do not certify that the original issue still persists.
    pub fn inspect_finding(&mut self, id: Uuid) -> Result<FindingInspection> {
        let record = self.finding(id)?;
        let availability = self
            .require_current_evidence()
            .and_then(|()| self.finding_vault())
            .and_then(|vault| {
                if vault == record.draft.vault {
                    Ok(())
                } else {
                    Err(stale(
                        "Finding belongs to another vault; its paths were not inspected.",
                    ))
                }
            });
        let mut evidence = Vec::with_capacity(record.draft.evidence.len());
        for (index, proof) in record.draft.evidence.iter().enumerate() {
            let observed = match &availability {
                Ok(()) => self.proposal_evidence_source(&proof.source.path),
                Err(error) => Err(error.clone()),
            };
            evidence.push(match observed {
                Ok(saved) => FindingEvidenceObservation {
                    index,
                    outcome: if saved.source == proof.source {
                        FindingEvidenceOutcome::Unchanged
                    } else {
                        FindingEvidenceOutcome::Changed
                    },
                    observed: Some(saved.source),
                    reason: None,
                },
                Err(error) => FindingEvidenceObservation {
                    index,
                    outcome: FindingEvidenceOutcome::Unavailable,
                    observed: None,
                    reason: Some(error.to_string()),
                },
            });
        }
        if matches!(
            record.draft.request.origin,
            FindingOrigin::InboxConflict { .. }
        ) && availability.is_ok()
        {
            // Equal file bytes alone do not certify a uniquely resolvable side.
            // Keep byte changes distinct, but never report an ambiguous or
            // incomplete managed identity as unchanged current evidence.
            let inventory = self.identity_inventory();
            for observation in &mut evidence {
                let proof = &record.draft.evidence[observation.index];
                let qualified = inventory.as_ref().is_ok_and(|inventory| {
                    let resolution =
                        inventory.resolution(proof.note_id.expect("checked conflict identity"));
                    resolution.outcome == crate::knowledge::IdentityOutcome::Unique
                        && resolution.matches[0].path == proof.source.path
                        && observation.observed.as_ref().is_some_and(|observed| {
                            resolution.matches[0].sha256 == observed.fingerprint.sha256
                        })
                });
                if !qualified {
                    observation.reason =
                        Some("Conflict side identity is unavailable, ambiguous or changed".into());
                    if observation.outcome == FindingEvidenceOutcome::Unchanged {
                        observation.outcome = FindingEvidenceOutcome::Unavailable;
                        observation.observed = None;
                    }
                }
            }
        }
        Ok(FindingInspection { record, evidence })
    }

    fn finding_vault(&mut self) -> Result<VaultRecord> {
        self.editor_files()?;
        serde_json::from_str(
            &self
                .store
                .setting("vault.editor_identity")?
                .ok_or_else(|| WorkflowError::msg("vault identity is unavailable"))?,
        )
        .map_err(|_| WorkflowError::msg("invalid saved vault identity"))
    }
}

fn saved_id(source: &ProposalSource) -> Result<Option<Uuid>> {
    note_identity::read(&source.text).map_err(|error| rejected(error.to_string()))
}
fn rejected(message: impl Into<String>) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, message)
}
fn stale(message: impl Into<String>) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}
