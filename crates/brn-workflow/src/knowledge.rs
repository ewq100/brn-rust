//! Managed knowledge metadata preparation over exact saved Markdown.
//! These queries neither admit proposals nor change vault content.
use crate::{ErrorKind, Result, WorkflowError, app::App, proposals::*, vault::VaultPath};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use brn_store::note_identity;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityRequest {
    pub path: String,
    pub note_id: Uuid,
    pub proposal_id: Uuid,
    pub title: String,
}

impl IdentityRequest {
    pub fn validate(&self) -> Result<()> {
        VaultPath::parse(&self.path).map_err(|error| rejected(error.to_string()))?;
        if self.note_id.is_nil()
            || self.proposal_id.is_nil()
            || self.title.trim().is_empty()
            || self.title.len() > 512
        {
            return Err(rejected(
                "Identity preparation needs nonnil note/proposal UUIDs and a title of 1 to 512 bytes.",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteIdentityInfo {
    pub path: String,
    pub note_id: Option<Uuid>,
    pub sha256: [u8; 32],
}

fn rejected(message: impl Into<String>) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, message)
}

impl App {
    /// Inspects the saved current note. Missing metadata remains explicitly absent.
    pub fn note_identity(&mut self, path: &str) -> Result<NoteIdentityInfo> {
        let source = self.proposal_source(path)?;
        Ok(NoteIdentityInfo {
            path: path.to_owned(),
            note_id: note_identity::read(&source.text)
                .map_err(|error| rejected(error.to_string()))?,
            sha256: source.source.fingerprint.sha256,
        })
    }

    /// Returns complete ordinary review input. The supplied IDs are bound only
    /// when the caller explicitly creates that proposal, with source CAS.
    pub fn prepare_note_identity(&mut self, request: &IdentityRequest) -> Result<DraftRequest> {
        request.validate()?;
        let source = self.proposal_source(&request.path)?;
        if note_identity::read(&source.text)
            .map_err(|error| rejected(error.to_string()))?
            .is_some()
        {
            return Err(rejected(
                "The note already has a managed identity. Inspect it instead of assigning another.",
            ));
        }
        let text = note_identity::assign(&source.text, request.note_id)
            .map_err(|error| rejected(error.to_string()))?;
        let draft = DraftRequest {
            id: request.proposal_id,
            group_id: None,
            session_id: None,
            title: request.title.clone(),
            changes: vec![DraftNoteChange::Replace {
                path: request.path.clone(),
                expected: source.source.fingerprint.clone(),
                text,
            }],
            sources: vec![source.source],
        };
        draft.validate()?;
        Ok(draft)
    }
}
