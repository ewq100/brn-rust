//! Fresh added UUID links are bound to saved evidence or exact same-draft bytes.
use super::{extract, rejected};
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    editor::file_error,
    knowledge::note_identity,
    library::{legacy_body_start, saved_metadata},
    proposals::{NoteChange, ProposalDraft},
    vault::EvidencePath,
};
use std::{collections::BTreeSet, path::Path};
use uuid::Uuid;

fn ids(text: &str) -> Result<BTreeSet<Uuid>> {
    // Unsupported legacy metadata remains editable when its old relationships
    // are unchanged. Exact complete header framing can locate the body without
    // interpreting opaque YAML or granting metadata strings link authority.
    // An unsupported/incomplete delimiter never supplies a guessed boundary.
    let body = match note_identity::body_start(text) {
        Ok(body) => body,
        Err(error) => legacy_body_start(text).ok_or_else(|| rejected(error.to_string()))?,
    };
    extract::stable_ids(text, body)
}

struct Target<'a> {
    path: &'a str,
    id: Uuid,
    /// Saved candidates need a fresh full source capture; same-draft candidates
    /// are already bound by the exact reviewed payload and destination proofs.
    saved_hash: Option<[u8; 32]>,
    reviewed_text: Option<&'a str>,
}

impl App {
    /// Called only on fresh ordinary approval. Historical links, exact Undo and
    /// completed operation replay retain their existing authority.
    pub(crate) fn validate_proposal_links(&mut self, draft: &ProposalDraft) -> Result<()> {
        if draft.inbox_source.is_some() {
            // The checked single source Create retains the exact imported body.
            // Its historical links are source evidence, not newly asserted BRN
            // knowledge relationships. Separate semantic proposals still need
            // the ordinary saved/same-draft target proof below.
            return Ok(());
        }
        let mut added = BTreeSet::new();
        for change in &draft.changes {
            let Some(text) = change.text() else { continue };
            let before = match change {
                NoteChange::Replace { before_text, .. } => before_text.as_str(),
                _ => "",
            };
            if text == before {
                continue;
            }
            // Comparison preserves prior Markdown targets, never a guessed
            // consumer identity. An unparseable baseline supplies no old links.
            let previous = ids(before).unwrap_or_default();
            added.extend(ids(text)?.difference(&previous).copied());
        }
        self.validate_proposal_note_targets(draft, &added)
    }

    /// Fresh explicit managed UUID references share the strict after-draft
    /// inventory and captured-source proof used by added Markdown links.
    pub(crate) fn validate_proposal_note_targets(
        &mut self,
        draft: &ProposalDraft,
        added: &BTreeSet<Uuid>,
    ) -> Result<()> {
        if added.is_empty() {
            return Ok(());
        }
        let inventory = self.identity_inventory()?;
        let files = self.editor_files()?;
        let changed = |path: &str| -> Result<bool> {
            // Folded names only veto conflicts; they are not positive alias
            // proof on a case-sensitive volume. A removed inventory row must
            // match the complete actual before-file proof of Replace/Trash.
            if EvidencePath::parse(path).is_err() {
                return Ok(false);
            }
            for change in &draft.changes {
                let before = match change {
                    NoteChange::Replace { before, .. } | NoteChange::Trash { before, .. } => before,
                    NoteChange::Create { .. }
                    | NoteChange::CreateAsset { .. }
                    | NoteChange::ReplaceAsset { .. }
                    | NoteChange::TrashAsset { .. } => continue,
                };
                if files
                    .reserved_copy_path_matches(Path::new(path), Path::new(change.path()))
                    .map_err(file_error)?
                    && files
                        .observe(Path::new(path))
                        .map_err(file_error)?
                        .fingerprint
                        == *before
                {
                    return Ok(true);
                }
            }
            Ok(false)
        };
        for issue in &inventory.issues {
            if !changed(&issue.path)? {
                return Err(rejected(
                    "New stable links need complete identity inspection.",
                ));
            }
        }
        let mut targets = Vec::new();
        for note in &inventory.notes {
            if !changed(&note.path)?
                && let Some(id) = note.note_id
            {
                targets.push(Target {
                    path: &note.path,
                    id,
                    saved_hash: Some(note.sha256),
                    reviewed_text: None,
                });
            }
        }
        for change in &draft.changes {
            if let Some(text) = change.text()
                && let Some(id) =
                    note_identity::read(text).map_err(|error| rejected(error.to_string()))?
            {
                targets.push(Target {
                    path: change.path(),
                    id,
                    saved_hash: None,
                    reviewed_text: Some(text),
                });
            }
        }
        for &id in added {
            let mut matches = targets.iter().filter(|target| target.id == id);
            let Some(target) = matches.next() else {
                return Err(rejected(
                    "New stable link target is absent or removed by this draft.",
                ));
            };
            if matches.next().is_some() {
                return Err(rejected("New stable link target identity is ambiguous."));
            }
            if let Some(text) = target.reviewed_text {
                let metadata = saved_metadata(text, target.path);
                if metadata.issue.is_some() || metadata.note_id != Some(id) {
                    return Err(rejected(
                        "Reviewed link target needs valid managed metadata.",
                    ));
                }
            } else {
                let capture = self.proposal_evidence_source(target.path)?;
                let metadata = saved_metadata(&capture.text, target.path);
                if metadata.issue.is_some() || metadata.note_id != Some(id) {
                    return Err(rejected("Saved link target needs valid managed metadata."));
                }
                if Some(capture.source.fingerprint.sha256) != target.saved_hash
                    || !draft.sources.contains(&capture.source)
                {
                    return Err(WorkflowError::typed(
                        ErrorKind::ContextStale,
                        "New stable link needs its exact captured target source binding.",
                    ));
                }
            }
        }
        Ok(())
    }
}
