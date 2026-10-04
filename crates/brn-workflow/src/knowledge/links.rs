//! Saved Markdown links carry exact evidence; inspection never changes knowledge.
use super::{IdentityInventory, IdentityIssue, IdentityOutcome, NoteIdentityInfo, rejected};
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    vault::{self, EvidencePath, NoteText},
};
use brn_store::note_identity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod extract;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkEvidence {
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
}

pub(super) struct RawMarkdownLink {
    pub destination: String,
    /// The occurrence, followed by its used definition for a reference link.
    pub evidence: Vec<LinkEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteLinkOutcome {
    Resolved,
    Absent,
    Unmanaged,
    Ambiguous,
    Incomplete,
    Changed,
    External,
    NonNote,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedNoteLink {
    pub destination: String,
    pub evidence: Vec<LinkEvidence>,
    pub target_path: Option<String>,
    pub outcome: NoteLinkOutcome,
    pub matches: Vec<NoteIdentityInfo>,
    pub issues: Vec<IdentityIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteLinks {
    pub source: NoteIdentityInfo,
    /// None means the saved source is unmanaged, rather than a guessed identity.
    pub source_outcome: Option<IdentityOutcome>,
    pub links: Vec<ResolvedNoteLink>,
    pub issues: Vec<IdentityIssue>,
}

impl App {
    /// A fresh saved-evidence observation, never a transactional vault snapshot.
    /// Derived consumers must retain the full source proof and identity outcome.
    pub fn note_links(&self, path: &str) -> Result<NoteLinks> {
        let note = self.evidence_note(path)?;
        let inventory = self.identity_inventory()?;
        self.links_from_saved(path, &note, &inventory)
    }

    pub(super) fn links_from_saved(
        &self,
        path: &str,
        note: &NoteText,
        inventory: &IdentityInventory,
    ) -> Result<NoteLinks> {
        let source = NoteIdentityInfo {
            path: path.to_owned(),
            note_id: note_identity::read(&note.text)
                .map_err(|error| rejected(error.to_string()))?,
            sha256: note.sha256,
        };
        let body =
            note_identity::body_start(&note.text).map_err(|error| rejected(error.to_string()))?;
        let raw = extract::extract(&note.text, body)?;
        if !inventory.notes.contains(&source) {
            return Err(changed_source());
        }
        let source_outcome = source.note_id.map(|id| inventory.resolution(id).outcome);
        let mut links = Vec::with_capacity(raw.len());
        for link in raw {
            let (target, immediate) = target(&source.path, &link.destination);
            let mut resolved = ResolvedNoteLink {
                destination: link.destination,
                evidence: link.evidence,
                target_path: None,
                outcome: immediate,
                matches: Vec::new(),
                issues: Vec::new(),
            };
            if let Some(target) = target {
                self.resolve_link(inventory, target, &mut resolved)?;
            }
            links.push(resolved);
        }
        // An external edit of the inspected source cannot certify old link text
        // as a current saved observation, even if size/mtime were retained.
        if self.evidence_note(path)?.sha256 != source.sha256 {
            return Err(changed_source());
        }
        Ok(NoteLinks {
            source,
            source_outcome,
            links,
            issues: inventory.issues.clone(),
        })
    }

    fn resolve_link(
        &self,
        inventory: &IdentityInventory,
        target: LinkTarget,
        link: &mut ResolvedNoteLink,
    ) -> Result<()> {
        let known = match target {
            LinkTarget::Path(path) => {
                link.target_path = Some(path.clone());
                let Some(known) = inventory.notes.iter().find(|note| note.path == path) else {
                    link.outcome = if inventory.issues.is_empty() {
                        NoteLinkOutcome::Absent
                    } else {
                        NoteLinkOutcome::Incomplete
                    };
                    return Ok(());
                };
                known.clone()
            }
            LinkTarget::Identity(id) => {
                let resolution = inventory.resolution(id);
                link.matches = resolution.matches;
                link.outcome = identity_outcome(resolution.outcome);
                if resolution.outcome != IdentityOutcome::Unique {
                    return Ok(());
                }
                let known = link.matches[0].clone();
                link.target_path = Some(known.path.clone());
                known
            }
        };
        if let Some(id) = known.note_id {
            let resolution = inventory.resolution(id);
            link.matches = resolution.matches;
            link.outcome = identity_outcome(resolution.outcome);
            if resolution.outcome != IdentityOutcome::Unique {
                return Ok(());
            }
        } else {
            link.matches = vec![known.clone()];
            link.outcome = NoteLinkOutcome::Unmanaged;
        }
        let path = EvidencePath::parse(&known.path).map_err(|error| rejected(error.to_string()))?;
        match vault::read_evidence(self.require_vault()?, &path) {
            Ok(note)
                if note.sha256 == known.sha256
                    && note_identity::read(&note.text).ok().flatten() == known.note_id => {}
            Ok(_) => link.outcome = NoteLinkOutcome::Changed,
            Err(error) => {
                link.outcome = NoteLinkOutcome::Incomplete;
                link.issues.push(IdentityIssue {
                    path: known.path,
                    reason: error.to_string(),
                });
            }
        }
        Ok(())
    }
}

fn changed_source() -> WorkflowError {
    WorkflowError::typed(
        ErrorKind::ContextStale,
        "Saved link source changed during inspection.",
    )
}

fn identity_outcome(outcome: IdentityOutcome) -> NoteLinkOutcome {
    match outcome {
        IdentityOutcome::Unique => NoteLinkOutcome::Resolved,
        IdentityOutcome::Absent => NoteLinkOutcome::Absent,
        IdentityOutcome::Ambiguous => NoteLinkOutcome::Ambiguous,
        IdentityOutcome::Incomplete => NoteLinkOutcome::Incomplete,
    }
}

enum LinkTarget {
    Path(String),
    Identity(Uuid),
}

fn target(source: &str, destination: &str) -> (Option<LinkTarget>, NoteLinkOutcome) {
    let invalid = || (None, NoteLinkOutcome::Unsupported);
    if let Some((scheme, rest)) = destination.split_once(':')
        && scheme.starts_with(|ch: char| ch.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
    {
        if !scheme.eq_ignore_ascii_case("brn") {
            return (None, NoteLinkOutcome::External);
        }
        let path = rest.split(['?', '#']).next().unwrap_or(rest);
        let Some(id) = path.strip_prefix("//note/") else {
            return invalid();
        };
        let Ok(uuid) = Uuid::parse_str(id) else {
            return invalid();
        };
        if id.len() != 36 || uuid.is_nil() || !uuid.to_string().eq_ignore_ascii_case(id) {
            return invalid();
        }
        return (Some(LinkTarget::Identity(uuid)), NoteLinkOutcome::Resolved);
    }
    if destination.starts_with("//") {
        return (None, NoteLinkOutcome::External);
    }
    let path = destination.split(['?', '#']).next().unwrap_or(destination);
    let Some(decoded) = decode_path(path) else {
        return invalid();
    };
    if decoded.is_empty() {
        return (
            Some(LinkTarget::Path(source.into())),
            NoteLinkOutcome::Resolved,
        );
    }
    if decoded.starts_with('/') || decoded.contains(['\\', '\0']) {
        return invalid();
    }
    let mut components: Vec<&str> = source.split('/').collect();
    components.pop();
    for part in decoded.split('/') {
        match part {
            "" => return invalid(),
            "." => {}
            ".." => {
                if components.pop().is_none() {
                    return invalid();
                }
            }
            part => components.push(part),
        }
    }
    let path = components.join("/");
    if EvidencePath::validate_folder(&path).is_err() {
        return invalid();
    }
    if EvidencePath::parse(&path).is_err() {
        return (None, NoteLinkOutcome::NonNote);
    }
    (Some(LinkTarget::Path(path)), NoteLinkOutcome::Resolved)
}

/// Decode once; plus is a literal filename byte. Encoded separators are refused
/// rather than reinterpreted as structure. Query/fragment splitting happened first.
fn decode_path(path: &str) -> Option<String> {
    let mut decoded = Vec::with_capacity(path.len());
    let mut bytes = path.bytes();
    while let Some(byte) = bytes.next() {
        let byte = if byte == b'%' {
            let digit = |byte: u8| (byte as char).to_digit(16).map(|digit| digit as u8);
            let byte = digit(bytes.next()?)?.checked_mul(16)? + digit(bytes.next()?)?;
            if matches!(byte, b'/' | b'\\' | 0) {
                return None;
            }
            byte
        } else {
            byte
        };
        decoded.push(byte);
    }
    String::from_utf8(decoded).ok()
}
