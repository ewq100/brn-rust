//! Pure capture and preview logic. GPUI entities live in `native` only.
use brn_workflow::{
    AmbiguityReason, AnchorState, CommentAnchorSnapshot, DraftComments, EditTrace, MAX_EDIT_STEPS,
    MAX_TRACE_REPLACEMENT_BYTES, TextEdit, apply_edit, derive_edit, map_anchor,
};
#[cfg(test)]
use brn_workflow::{AnchorProjection, OriginalAnchor, RecoveryReference, replay_trace};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pub draft_id: Uuid,
    pub generation: u64,
    pub text: String,
    pub range: Range<usize>,
    pub quote: String,
}

impl Capture {
    pub fn new(
        draft_id: Uuid,
        generation: u64,
        text: &str,
        range: Range<usize>,
    ) -> Result<Self, String> {
        let quote = text
            .get(range.clone())
            .ok_or("Select a valid passage first.")?;
        if quote.is_empty() {
            return Err("Select a nonempty passage first.".into());
        }
        Ok(Self {
            draft_id,
            generation,
            text: text.into(),
            range,
            quote: quote.into(),
        })
    }

    pub fn matches(&self, draft_id: Uuid, generation: u64, text: &str) -> bool {
        self.draft_id == draft_id && self.generation == generation && self.text == text
    }
}

struct CommentReference {
    original: usize,
    range: Range<usize>,
    quote: String,
    checkpoints: Vec<(usize, AnchorState)>,
}

/// Each immutable revision has one shared text buffer, regardless of how many
/// comments refer to it. Hashes and ranges are checked once on acceptance.
pub struct RecoveryCache {
    revisions: Vec<Arc<str>>,
    comments: Vec<CommentReference>,
}

fn valid_range(text: &str, range: &Range<usize>, quote: &str) -> bool {
    !quote.is_empty() && range.start < range.end && text.get(range.clone()) == Some(quote)
}

impl RecoveryCache {
    pub fn new(saved: &DraftComments, snapshots: &[CommentAnchorSnapshot]) -> Result<Self, String> {
        if !saved.comments.is_empty()
            && (saved.draft.text.len() > brn_workflow::worker::MAX_DRAFT_BYTES
                || Sha256::digest(saved.draft.text.as_bytes()).as_slice() != saved.draft.sha256)
        {
            return Err("Current draft text or hash is invalid for comment preview.".into());
        }
        let mut revisions = Vec::with_capacity(snapshots.len());
        let mut indices = HashMap::with_capacity(snapshots.len());
        for snapshot in snapshots {
            let revision = &snapshot.revision;
            if revision.draft_id != saved.draft.id
                || revision.kind != brn_workflow::worker::RevisionKind::Checkpoint
                || revision.text.len() > brn_workflow::worker::MAX_DRAFT_BYTES
                || Sha256::digest(revision.text.as_bytes()).as_slice() != revision.sha256
                || indices.insert(revision.id, revisions.len()).is_some()
            {
                return Err("Invalid shared comment checkpoint.".into());
            }
            revisions.push(Arc::<str>::from(revision.text.as_str()));
        }
        let mut comments = Vec::with_capacity(saved.comments.len());
        for view in &saved.comments {
            let comment = &view.comment;
            if comment.draft_id != saved.draft.id {
                return Err("Comment belongs to another draft.".into());
            }
            let original = *indices
                .get(&comment.original_revision_id)
                .ok_or("Original comment revision missing from snapshots.")?;
            let original_revision = &snapshots[original].revision;
            let range = comment.original_start..comment.original_end;
            if original_revision.sha256 != comment.original_sha256
                || !valid_range(&revisions[original], &range, &comment.original_quote)
            {
                return Err("Original comment range or hash does not match.".into());
            }
            if let AnchorState::Anchored { start, end } = view.anchor
                && !valid_range(&saved.draft.text, &(start..end), &comment.original_quote)
            {
                return Err("Current comment range does not match.".into());
            }
            let mut checkpoints = Vec::new();
            for (index, snapshot) in snapshots.iter().enumerate() {
                if let Some((_, state)) = snapshot.anchors.iter().find(|(id, _)| *id == comment.id)
                {
                    if let AnchorState::Anchored { start, end } = state
                        && !valid_range(&revisions[index], &(*start..*end), &comment.original_quote)
                    {
                        return Err("Checkpoint comment range does not match.".into());
                    }
                    checkpoints.push((index, state.clone()));
                }
            }
            if !checkpoints.iter().any(|(index, state)| {
                *index == original
                    && *state
                        == (AnchorState::Anchored {
                            start: range.start,
                            end: range.end,
                        })
            }) {
                return Err("Original checkpoint anchor missing.".into());
            }
            comments.push(CommentReference {
                original,
                range,
                quote: comment.original_quote.clone(),
                checkpoints,
            });
        }
        Ok(Self {
            revisions,
            comments,
        })
    }

    fn recover(&self, text: &str, reference: &CommentReference, state: AnchorState) -> AnchorState {
        if text == &*self.revisions[reference.original] {
            return AnchorState::Anchored {
                start: reference.range.start,
                end: reference.range.end,
            };
        }
        let mut candidate = None;
        for (index, stored) in &reference.checkpoints {
            if text == &*self.revisions[*index]
                && let AnchorState::Anchored { .. } = stored
            {
                if let Some(previous) = &candidate
                    && previous != stored
                {
                    return AnchorState::Ambiguous {
                        reason: AmbiguityReason::ConflictingSnapshot,
                    };
                }
                candidate = Some(stored.clone());
            }
        }
        candidate.unwrap_or(state)
    }

    pub fn advance(
        &self,
        before: &str,
        after: &str,
        states: &[AnchorState],
        edit: Option<&TextEdit>,
    ) -> Result<Vec<AnchorState>, String> {
        if before.len() > brn_workflow::worker::MAX_DRAFT_BYTES
            || after.len() > brn_workflow::worker::MAX_DRAFT_BYTES
        {
            return Err("Draft exceeds 1 MiB during comment preview.".into());
        }
        if states.len() != self.comments.len() {
            return Err("Comment preview count changed.".into());
        }
        if let Some(edit) = edit
            && (apply_edit(before, edit).map_err(|e| e.to_string())? != after
                || derive_edit(before, after).as_ref() != Some(edit))
        {
            return Err("Editor change is not canonical.".into());
        }
        self.comments
            .iter()
            .zip(states)
            .map(|(reference, state)| {
                let mapped = match edit {
                    Some(edit) => map_anchor(before, after, state, &reference.quote, edit)
                        .map_err(|e| e.to_string())?,
                    None => AnchorState::Ambiguous {
                        reason: AmbiguityReason::HistoryLimit,
                    },
                };
                Ok(self.recover(after, reference, mapped))
            })
            .collect()
    }

    pub fn replay(
        &self,
        initial: &str,
        trace: &EditTrace,
        final_text: &str,
        initial_states: &[AnchorState],
    ) -> Result<Vec<AnchorState>, String> {
        if initial.len() > brn_workflow::worker::MAX_DRAFT_BYTES
            || final_text.len() > brn_workflow::worker::MAX_DRAFT_BYTES
        {
            return Err("Draft exceeds 1 MiB during comment preview.".into());
        }
        if initial_states.len() != self.comments.len() {
            return Err("Comment preview count changed.".into());
        }
        match trace {
            EditTrace::HistoryLost => self.advance(initial, final_text, initial_states, None),
            EditTrace::Steps(steps) => {
                if steps.len() > MAX_EDIT_STEPS
                    || steps.iter().map(|e| e.replacement.len()).sum::<usize>()
                        > MAX_TRACE_REPLACEMENT_BYTES
                {
                    return Err("Comment edit trace exceeds its limit.".into());
                }
                let mut text = initial.to_owned();
                let mut states = initial_states.to_vec();
                for edit in steps {
                    let next = apply_edit(&text, edit).map_err(|e| e.to_string())?;
                    states = self.advance(&text, &next, &states, Some(edit))?;
                    text = next;
                }
                if text != final_text {
                    return Err("Comment edit trace does not reach the current text.".into());
                }
                Ok(self
                    .comments
                    .iter()
                    .zip(states)
                    .map(|(reference, state)| self.recover(final_text, reference, state))
                    .collect())
            }
        }
    }
}

#[cfg(test)]
pub fn projections(
    saved: &DraftComments,
    snapshots: &[CommentAnchorSnapshot],
) -> Result<Vec<AnchorProjection>, String> {
    saved
        .comments
        .iter()
        .map(|view| {
            let comment = &view.comment;
            let original = snapshots
                .iter()
                .find(|s| s.revision.id == comment.original_revision_id)
                .ok_or_else(|| "Original comment revision missing from snapshots.".to_string())?;
            if original.revision.draft_id != saved.draft.id
                || original.revision.sha256 != comment.original_sha256
            {
                return Err("Original comment revision does not match its draft.".into());
            }
            let checkpoints = snapshots
                .iter()
                .filter_map(|snapshot| {
                    snapshot
                        .anchors
                        .iter()
                        .find(|(id, _)| *id == comment.id)
                        .map(|(_, state)| RecoveryReference {
                            text: snapshot.revision.text.clone(),
                            sha256: snapshot.revision.sha256,
                            state: state.clone(),
                        })
                })
                .collect();
            Ok(AnchorProjection {
                state: view.anchor.clone(),
                original: OriginalAnchor {
                    text: original.revision.text.clone(),
                    sha256: original.revision.sha256,
                    range: comment.original_start..comment.original_end,
                    quote: comment.original_quote.clone(),
                },
                checkpoints,
            })
        })
        .collect()
}

#[cfg(test)]
pub fn preview(
    saved: &DraftComments,
    snapshots: &[CommentAnchorSnapshot],
    text: &str,
    trace: &EditTrace,
) -> Result<Vec<AnchorState>, String> {
    replay_trace(
        &saved.draft.text,
        trace,
        text,
        &projections(saved, snapshots)?,
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_workflow::worker::{Draft, DraftRevision, DraftStamp, RevisionKind};
    use brn_workflow::{CommentStatus, DraftComment, DraftCommentView};

    #[test]
    fn shared_revision_buffer_is_reused_across_comments_and_edits() {
        let draft_id = Uuid::new_v4();
        let revision_id = Uuid::new_v4();
        let digest: [u8; 32] = Sha256::digest(b"one").into();
        let anchor = AnchorState::Anchored { start: 0, end: 3 };
        let mut views = Vec::new();
        for _ in 0..2 {
            views.push(DraftCommentView {
                comment: DraftComment {
                    id: Uuid::new_v4(),
                    draft_id,
                    original_revision_id: revision_id,
                    original_sha256: digest,
                    original_start: 0,
                    original_end: 3,
                    original_quote: "one".into(),
                    body: "note".into(),
                    status: CommentStatus::Open,
                    status_version: 0,
                },
                anchor: anchor.clone(),
            });
        }
        let saved = DraftComments {
            draft: Draft {
                id: draft_id,
                title: "test".into(),
                stamp: DraftStamp {
                    base_revision: revision_id,
                    generation: 0,
                },
                text: "one".into(),
                sha256: digest,
            },
            comments: views.clone(),
        };
        let snapshots = vec![CommentAnchorSnapshot {
            revision: DraftRevision {
                id: revision_id,
                draft_id,
                parent_id: None,
                kind: RevisionKind::Checkpoint,
                text: "one".into(),
                sha256: digest,
                origin_turn: None,
            },
            anchors: views
                .iter()
                .map(|view| (view.comment.id, anchor.clone()))
                .collect(),
        }];
        let cache = RecoveryCache::new(&saved, &snapshots).unwrap();
        assert_eq!(cache.revisions.len(), 1);
        assert_eq!(cache.comments.len(), 2);
        assert!(
            cache
                .comments
                .iter()
                .all(|comment| comment.original == 0 && comment.checkpoints.len() == 1)
        );
        let shared = Arc::as_ptr(&cache.revisions[0]);
        let first = derive_edit("one", "Xone").unwrap();
        let second = derive_edit("Xone", "Yone").unwrap();
        let states = cache
            .advance(
                "one",
                "Xone",
                &[anchor.clone(), anchor.clone()],
                Some(&first),
            )
            .unwrap();
        let states = cache
            .advance("Xone", "Yone", &states, Some(&second))
            .unwrap();
        assert_eq!(Arc::as_ptr(&cache.revisions[0]), shared);
        assert_eq!(Arc::strong_count(&cache.revisions[0]), 1);
        assert_eq!(
            states,
            preview(
                &saved,
                &snapshots,
                "Yone",
                &EditTrace::Steps(vec![first, second])
            )
            .unwrap()
        );
    }
}
