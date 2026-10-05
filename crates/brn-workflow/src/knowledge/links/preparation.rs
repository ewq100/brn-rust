//! Additive durable links use the existing exact proposal boundary.
use super::{NoteLinkOutcome, prepare_text, rejected};
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    knowledge::{IdentityOutcome, note_identity},
    library::saved_metadata,
    proposals::{DraftNoteChange, DraftRequest},
    vault::{NoteText, VaultPath},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkRequest {
    pub path: String,
    pub target_note_id: Uuid,
    pub expected_target_sha256: [u8; 32],
    pub proposal_id: Uuid,
    pub title: String,
    pub label: String,
}

impl LinkRequest {
    pub fn validate(&self) -> Result<()> {
        VaultPath::parse(&self.path).map_err(|error| rejected(error.to_string()))?;
        if self.target_note_id.is_nil()
            || self.proposal_id.is_nil()
            || self.title.trim().is_empty()
            || self.title.len() > 512
            || self.label.trim().is_empty()
            || self.label.len() > 512
            || self.label.chars().any(char::is_control)
        {
            return Err(rejected(
                "Link preparation needs nonnil UUIDs, a title and a single-line label of 1 to 512 bytes.",
            ));
        }
        Ok(())
    }
}

impl App {
    /// Full ordinary review input, with no proposal admission or vault write.
    pub fn prepare_note_link(&mut self, request: &LinkRequest) -> Result<DraftRequest> {
        request.validate()?;
        let source = self.proposal_source(&request.path)?;
        let metadata = saved_metadata(&source.text, &request.path);
        let Some(source_id) = metadata
            .note_id
            .filter(|_| !metadata.source && !metadata.history && metadata.issue.is_none())
        else {
            return Err(rejected(
                "Link preparation needs an identified current knowledge note with valid metadata.",
            ));
        };
        let inventory = self.identity_inventory()?;
        let source_resolution = inventory.resolution(source_id);
        if source_resolution.outcome != IdentityOutcome::Unique
            || source_resolution.matches[0].path != request.path
            || source_resolution.matches[0].sha256 != source.source.fingerprint.sha256
        {
            return Err(rejected(
                "Link source identity is ambiguous, incomplete or changed.",
            ));
        }
        if source_id == request.target_note_id {
            return Err(rejected("Prepare a relationship to a separate note."));
        }
        let resolution = inventory.resolution(request.target_note_id);
        if resolution.outcome != IdentityOutcome::Unique {
            return Err(rejected(
                "Link target identity is absent, ambiguous or incompletely inspected.",
            ));
        }
        let target = &resolution.matches[0];
        if target.sha256 != request.expected_target_sha256 {
            return Err(stale("Selected link target content changed."));
        }
        let captured = self.proposal_evidence_source(&target.path)?;
        let metadata = saved_metadata(&captured.text, &target.path);
        if metadata.issue.is_some() || metadata.note_id != Some(request.target_note_id) {
            return Err(rejected("Link target needs valid managed metadata."));
        }
        if captured.source.fingerprint.sha256 != request.expected_target_sha256 {
            return Err(stale("Link target changed during preparation."));
        }
        let links = self.links_from_saved(
            &request.path,
            &NoteText {
                text: source.text.clone(),
                sha256: source.source.fingerprint.sha256,
            },
            &inventory,
        )?;
        if links.links.iter().any(|link| {
            link.outcome == NoteLinkOutcome::Resolved
                && link
                    .matches
                    .iter()
                    .any(|note| note.note_id == Some(request.target_note_id))
        }) {
            return Err(rejected(
                "The note already has a resolved link to this target.",
            ));
        }
        let body =
            note_identity::body_start(&source.text).map_err(|error| rejected(error.to_string()))?;
        let text =
            prepare_text::append(&source.text, body, &request.label, request.target_note_id)?;
        let draft = DraftRequest {
            inbox_knowledge: None,
            inbox_source: None,
            action_changes: Vec::new(),
            id: request.proposal_id,
            group_id: None,
            session_id: None,
            title: request.title.clone(),
            changes: vec![DraftNoteChange::Replace {
                path: request.path.clone(),
                expected: source.source.fingerprint.clone(),
                text,
            }],
            sources: vec![source.source, captured.source],
        };
        draft.validate()?;
        Ok(draft)
    }
}

fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}
