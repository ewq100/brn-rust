//! Pure capture and preview logic. GPUI entities live in `native` only.
use brn_workflow::{
    AnchorProjection, AnchorState, CommentAnchorSnapshot, DraftComments, EditTrace, OriginalAnchor,
    RecoveryReference, TextEdit, replay_trace,
};
use std::ops::Range;
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

/// Advance a cached preview through exactly one observed editor change.
pub fn advance(
    saved: &DraftComments,
    snapshots: &[CommentAnchorSnapshot],
    before: &str,
    after: &str,
    states: &[AnchorState],
    edit: Option<&TextEdit>,
) -> Result<Vec<AnchorState>, String> {
    let mut projections = projections(saved, snapshots)?;
    if projections.len() != states.len() {
        return Err("Comment preview count changed.".into());
    }
    for (projection, state) in projections.iter_mut().zip(states) {
        projection.state = state.clone();
    }
    let trace = match edit {
        Some(edit) => EditTrace::Steps(vec![edit.clone()]),
        None => EditTrace::HistoryLost,
    };
    replay_trace(before, &trace, after, &projections).map_err(|error| error.to_string())
}
