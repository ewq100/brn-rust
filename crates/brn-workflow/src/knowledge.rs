//! Managed knowledge metadata preparation over exact saved Markdown.
//! These queries neither admit proposals nor change vault content.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    proposals::*,
    vault::{self, EvidencePath, NoteText, VaultPath},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

pub use brn_store::note_identity;
mod provenance;
pub use provenance::*;
mod links;
pub use links::*;
mod relationships;
pub use relationships::*;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityIssue {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DuplicateIdentity {
    pub note_id: Uuid,
    pub paths: Vec<String>,
}

/// Complete readable observations include unmanaged notes. Issues prevent a
/// resolution from treating one observed match as a proven unique identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityInventory {
    pub notes: Vec<NoteIdentityInfo>,
    pub issues: Vec<IdentityIssue>,
    pub duplicates: Vec<DuplicateIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityOutcome {
    Unique,
    Absent,
    Ambiguous,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityResolution {
    pub note_id: Uuid,
    pub outcome: IdentityOutcome,
    pub matches: Vec<NoteIdentityInfo>,
    pub issues: Vec<IdentityIssue>,
}

impl IdentityInventory {
    fn resolution(&self, note_id: Uuid) -> IdentityResolution {
        let matches = self
            .notes
            .iter()
            .filter(|note| note.note_id == Some(note_id))
            .cloned()
            .collect::<Vec<_>>();
        let outcome = if matches.len() > 1 {
            IdentityOutcome::Ambiguous
        } else if !self.issues.is_empty() {
            IdentityOutcome::Incomplete
        } else if matches.is_empty() {
            IdentityOutcome::Absent
        } else {
            IdentityOutcome::Unique
        };
        IdentityResolution {
            note_id,
            outcome,
            matches,
            issues: self.issues.clone(),
        }
    }
}

fn rejected(message: impl Into<String>) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, message)
}

impl App {
    /// Explicit source/history access preserves the original wording even when
    /// its metadata is incomplete. It does not open an editor or admit a change.
    pub fn evidence_note(&self, path: &str) -> Result<NoteText> {
        self.require_current_evidence()?;
        let root = self.require_vault()?;
        let path = EvidencePath::parse(path).map_err(|error| rejected(error.to_string()))?;
        vault::read_evidence(root, &path).map_err(|error| rejected(error.to_string()))
    }

    /// Reads every eligible saved Markdown file, including archives. Cached
    /// metadata cannot prove identity absence or detect newly introduced aliases.
    pub fn identity_inventory(&self) -> Result<IdentityInventory> {
        self.require_current_evidence()?;
        let root = self.require_vault()?;
        let scan = vault::scan_evidence(root).map_err(|error| {
            WorkflowError::msg(format!("could not inspect vault identities: {error}"))
        })?;
        let mut notes = Vec::new();
        let mut issues = scan
            .skipped
            .into_iter()
            .map(|skipped| IdentityIssue {
                path: skipped.path,
                reason: match skipped.reason {
                    vault::SkipReason::TooLarge => "note is larger than 1 MiB",
                    vault::SkipReason::InvalidName => {
                        "note path is not valid UTF-8 or is unsupported"
                    }
                    vault::SkipReason::UnreadableDirectory => "could not inspect folder",
                }
                .into(),
            })
            .collect::<Vec<_>>();
        let mut identities = BTreeMap::<Uuid, Vec<String>>::new();
        for file in scan.notes {
            let path = file.path.as_str();
            let observed = vault::read_evidence(root, &file.path)
                .map_err(|error| error.to_string())
                .and_then(|note| {
                    note_identity::read(&note.text)
                        .map(|note_id| NoteIdentityInfo {
                            path: path.into(),
                            note_id,
                            sha256: note.sha256,
                        })
                        .map_err(|error| error.to_string())
                });
            match observed {
                Ok(note) => {
                    if let Some(id) = note.note_id {
                        identities.entry(id).or_default().push(note.path.clone());
                    }
                    notes.push(note);
                }
                Err(reason) => issues.push(IdentityIssue {
                    path: path.into(),
                    reason,
                }),
            }
        }
        issues.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.reason.cmp(&right.reason))
        });
        let duplicates = identities
            .into_iter()
            .filter(|(_, paths)| paths.len() > 1)
            .map(|(note_id, paths)| DuplicateIdentity { note_id, paths })
            .collect();
        Ok(IdentityInventory {
            notes,
            issues,
            duplicates,
        })
    }

    /// Resolves only from a freshly observed evidence universe. Incomplete
    /// inspection cannot certify absence or uniqueness; ambiguity never guesses.
    pub fn resolve_note_identity(&self, note_id: Uuid) -> Result<IdentityResolution> {
        if note_id.is_nil() {
            return Err(rejected("Identity resolution needs a nonnil UUID."));
        }
        Ok(self.identity_inventory()?.resolution(note_id))
    }

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
