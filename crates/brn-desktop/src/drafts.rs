use crate::comments::{self, Capture};
use brn_workflow::worker::{Draft, DraftStamp};
use brn_workflow::{
    AnchorState, CommentAnchorSnapshot, CommentCapture, CommentStatusChanged, DraftCommentView,
    DraftComments, EditTrace, MAX_EDIT_STEPS, MAX_TRACE_REPLACEMENT_BYTES, TextEdit, derive_edit,
};
use std::ops::Range;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submitted {
    pub op: Uuid,
    pub id: Uuid,
    pub expected: DraftStamp,
    pub generation: u64,
    pub text: String,
    pub edits: EditTrace,
    pub capture: bool,
    pub composer_generation: u64,
    pub composer: String,
}

/// Tracks acknowledged bytes independently of the live editor buffer.
pub struct DraftEditor {
    acknowledged: DraftComments,
    snapshots: Vec<CommentAnchorSnapshot>,
    generation: u64,
    text: String,
    trace: EditTrace,
    preview: Result<Vec<AnchorState>, String>,
    pending: Option<Submitted>,
    capture: Option<Capture>,
    composer: String,
    composer_generation: u64,
}
impl DraftEditor {
    pub fn new(draft: Draft) -> Self {
        Self::from_comments(
            DraftComments {
                draft,
                comments: vec![],
            },
            vec![],
        )
    }
    pub fn from_comments(
        acknowledged: DraftComments,
        snapshots: Vec<CommentAnchorSnapshot>,
    ) -> Self {
        let preview = Ok(acknowledged
            .comments
            .iter()
            .map(|view| view.anchor.clone())
            .collect());
        Self {
            generation: acknowledged.draft.stamp.generation,
            text: acknowledged.draft.text.clone(),
            acknowledged,
            snapshots,
            trace: EditTrace::Steps(vec![]),
            preview,
            pending: None,
            capture: None,
            composer: String::new(),
            composer_generation: 0,
        }
    }
    pub fn id(&self) -> Uuid {
        self.acknowledged.draft.id
    }
    pub fn title(&self) -> &str {
        &self.acknowledged.draft.title
    }
    pub fn stamp(&self) -> DraftStamp {
        self.acknowledged.draft.stamp
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    #[cfg(test)]
    pub fn trace(&self) -> &EditTrace {
        &self.trace
    }
    pub fn comments(&self) -> &[DraftCommentView] {
        &self.acknowledged.comments
    }
    pub fn original_revision(
        &self,
        comment_id: Uuid,
    ) -> Option<brn_workflow::worker::DraftRevision> {
        let comment = self
            .acknowledged
            .comments
            .iter()
            .find(|v| v.comment.id == comment_id)?;
        self.snapshots
            .iter()
            .find(|s| s.revision.id == comment.comment.original_revision_id)
            .map(|s| s.revision.clone())
    }
    pub fn capture_quote(&self) -> Option<&str> {
        self.capture.as_ref().map(|c| c.quote.as_str())
    }
    pub fn capture(&mut self, range: Range<usize>) -> Result<(), String> {
        self.capture = Some(Capture::new(self.id(), self.generation, &self.text, range)?);
        Ok(())
    }
    pub fn can_add_comment(&self, body: &str) -> bool {
        self.pending.is_none()
            && !body.trim().is_empty()
            && body.len() <= 64 * 1024
            && self.text.len() <= brn_workflow::worker::MAX_DRAFT_BYTES
            && self
                .capture
                .as_ref()
                .is_some_and(|c| c.matches(self.id(), self.generation, &self.text))
    }
    pub fn composer(&self) -> &str {
        &self.composer
    }
    pub fn set_composer(&mut self, body: String) {
        if self.composer != body {
            self.composer = body;
            self.composer_generation = self.composer_generation.saturating_add(1);
        }
    }
    pub fn discard_composer(&mut self) {
        self.set_composer(String::new());
        self.capture = None;
    }
    pub fn preview_states(&self) -> Result<Vec<AnchorState>, String> {
        self.preview.clone()
    }
    pub fn dirty(&self) -> bool {
        self.generation != self.acknowledged.draft.stamp.generation
            || self.text != self.acknowledged.draft.text
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn can_replace(&self) -> bool {
        !self.dirty() && !self.pending() && self.composer.trim().is_empty()
    }
    pub fn can_close(&self) -> bool {
        self.can_replace()
    }
    pub fn replace(&mut self, draft: Draft) -> bool {
        if !self.can_replace() {
            return false;
        }
        *self = Self::new(draft);
        true
    }
    pub fn replace_comments(
        &mut self,
        saved: DraftComments,
        snapshots: Vec<CommentAnchorSnapshot>,
    ) -> bool {
        if !self.can_replace() {
            return false;
        }
        *self = Self::from_comments(saved, snapshots);
        true
    }
    pub fn refresh_comments(
        &mut self,
        saved: DraftComments,
        snapshots: Vec<CommentAnchorSnapshot>,
    ) -> bool {
        let old = &self.acknowledged.draft;
        if saved.draft.id != old.id
            || saved.draft.stamp != old.stamp
            || saved.draft.sha256 != old.sha256
            || saved.draft.text != old.text
        {
            return false;
        }
        if self.acknowledged.comments.iter().any(|current| {
            saved
                .comments
                .iter()
                .find(|incoming| incoming.comment.id == current.comment.id)
                .is_none_or(|incoming| {
                    incoming.comment.status_version < current.comment.status_version
                })
        }) {
            return false;
        }
        self.acknowledged = saved;
        self.snapshots = snapshots;
        let trace = match &self.pending {
            Some(p) => merge_traces(&p.edits, &self.trace),
            None => self.trace.clone(),
        };
        self.preview = comments::preview(&self.acknowledged, &self.snapshots, &self.text, &trace);
        true
    }
    pub fn edit(&mut self, text: String) {
        if self.text == text {
            return;
        }
        let before = self.text.clone();
        let next = if self.text.len() > brn_workflow::worker::MAX_DRAFT_BYTES
            || text.len() > brn_workflow::worker::MAX_DRAFT_BYTES
        {
            None
        } else {
            derive_edit(&self.text, &text)
        };
        match (&mut self.trace, next.clone()) {
            (EditTrace::Steps(steps), Some(edit)) => {
                let bytes: usize = steps.iter().map(|step| step.replacement.len()).sum();
                if steps.len() >= MAX_EDIT_STEPS
                    || bytes.saturating_add(edit.replacement.len()) > MAX_TRACE_REPLACEMENT_BYTES
                {
                    self.trace = EditTrace::HistoryLost;
                } else {
                    steps.push(edit);
                }
            }
            _ => self.trace = EditTrace::HistoryLost,
        }
        let step = if matches!(self.trace, EditTrace::HistoryLost) {
            None
        } else {
            next.as_ref()
        };
        self.preview = match &self.preview {
            Ok(states) => comments::advance(
                &self.acknowledged,
                &self.snapshots,
                &before,
                &text,
                states,
                step,
            ),
            Err(_) => {
                let trace = match &self.pending {
                    Some(p) => merge_traces(&p.edits, &self.trace),
                    None => self.trace.clone(),
                };
                comments::preview(&self.acknowledged, &self.snapshots, &text, &trace)
            }
        };
        self.generation = self
            .generation
            .checked_add(1)
            .expect("editor generation exhausted");
        self.text = text;
    }
    pub fn begin_save(&mut self, op: Uuid) -> Option<Submitted> {
        if self.pending() {
            return None;
        }
        let submitted = Submitted {
            op,
            id: self.id(),
            expected: self.stamp(),
            generation: self.generation,
            text: self.text.clone(),
            edits: std::mem::replace(&mut self.trace, EditTrace::Steps(vec![])),
            capture: false,
            composer_generation: self.composer_generation,
            composer: self.composer.clone(),
        };
        self.pending = Some(submitted.clone());
        Some(submitted)
    }
    pub fn begin_comment(&mut self, op: Uuid) -> Option<(Submitted, CommentCapture)> {
        if !self.can_add_comment(&self.composer) {
            return None;
        }
        let capture = self.capture.clone()?;
        let mut submitted = self.begin_save(op)?;
        submitted.capture = true;
        self.pending = Some(submitted.clone());
        let request = CommentCapture {
            op,
            draft_id: submitted.id,
            expected: submitted.expected,
            generation: submitted.generation,
            text: submitted.text.clone(),
            edits: submitted.edits.clone(),
            range: capture.range,
            quote: capture.quote,
            body: submitted.composer.clone(),
        };
        Some((submitted, request))
    }
    pub fn acknowledge(&mut self, op: Uuid, saved: Draft) -> bool {
        self.acknowledge_comments(
            op,
            DraftComments {
                draft: saved,
                comments: vec![],
            },
            vec![],
        )
    }
    pub fn acknowledge_comments(
        &mut self,
        op: Uuid,
        saved: DraftComments,
        snapshots: Vec<CommentAnchorSnapshot>,
    ) -> bool {
        let Some(submitted) = &self.pending else {
            return false;
        };
        if submitted.op != op
            || submitted.id != saved.draft.id
            || submitted.generation != saved.draft.stamp.generation
            || submitted.text != saved.draft.text
        {
            return false;
        }
        self.acknowledged = saved;
        self.snapshots = snapshots;
        self.preview =
            comments::preview(&self.acknowledged, &self.snapshots, &self.text, &self.trace);
        if submitted.capture
            && self.composer_generation == submitted.composer_generation
            && self.composer == submitted.composer
        {
            self.composer.clear();
            self.capture = None;
        }
        self.pending = None;
        true
    }
    pub fn apply_status(&mut self, result: &CommentStatusChanged) -> bool {
        let Some(view) = self
            .acknowledged
            .comments
            .iter_mut()
            .find(|v| v.comment.id == result.comment.id)
        else {
            return false;
        };
        if view.comment.draft_id != result.comment.draft_id
            || view.comment.id != result.comment.id
            || result.comment.status_version < view.comment.status_version
        {
            return false;
        }
        view.comment.status = result.comment.status;
        view.comment.status_version = result.comment.status_version;
        true
    }
    pub fn fail(&mut self, op: Uuid) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.op == op)
        {
            let submitted = self.pending.take().unwrap();
            self.trace = merge_traces(&submitted.edits, &self.trace);
            true
        } else {
            false
        }
    }
    pub fn discard(&mut self) -> bool {
        if self.pending() {
            return false;
        }
        self.generation = self.acknowledged.draft.stamp.generation;
        self.text = self.acknowledged.draft.text.clone();
        self.trace = EditTrace::Steps(vec![]);
        self.preview = Ok(self
            .acknowledged
            .comments
            .iter()
            .map(|v| v.anchor.clone())
            .collect());
        self.capture = None;
        true
    }
}

fn merge_traces(prefix: &EditTrace, suffix: &EditTrace) -> EditTrace {
    match (prefix, suffix) {
        (EditTrace::Steps(a), EditTrace::Steps(b)) => {
            if a.len().saturating_add(b.len()) > MAX_EDIT_STEPS
                || a.iter()
                    .chain(b)
                    .map(|step: &TextEdit| step.replacement.len())
                    .sum::<usize>()
                    > MAX_TRACE_REPLACEMENT_BYTES
            {
                EditTrace::HistoryLost
            } else {
                EditTrace::Steps(a.iter().chain(b).cloned().collect())
            }
        }
        _ => EditTrace::HistoryLost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_workflow::worker::RevisionKind;
    use brn_workflow::{CommentStatus, DraftComment};

    fn draft() -> Draft {
        Draft {
            id: Uuid::new_v4(),
            title: "title".into(),
            stamp: DraftStamp {
                base_revision: Uuid::new_v4(),
                generation: 0,
            },
            text: "one".into(),
            sha256: [0; 32],
        }
    }
    fn saved_comment(
        text: &str,
        start: usize,
        end: usize,
    ) -> (DraftComments, Vec<CommentAnchorSnapshot>) {
        let mut d = draft();
        d.text = text.into();
        let id = Uuid::new_v4();
        let original = Uuid::new_v4();
        let digest: [u8; 32] = [
            0x76, 0x92, 0xc3, 0xad, 0x35, 0x40, 0xbb, 0x80, 0x3c, 0x02, 0x0b, 0x3a, 0xee, 0x66,
            0xcd, 0x88, 0x87, 0x12, 0x32, 0x34, 0xea, 0x0c, 0x6e, 0x71, 0x43, 0xc0, 0xad, 0xd7,
            0x3f, 0xf4, 0x31, 0xed,
        ];
        let anchor = AnchorState::Anchored { start, end };
        let comment = DraftComment {
            id,
            draft_id: d.id,
            original_revision_id: original,
            original_sha256: digest,
            original_start: start,
            original_end: end,
            original_quote: text[start..end].into(),
            body: "note".into(),
            status: CommentStatus::Open,
            status_version: 0,
        };
        let snapshot = CommentAnchorSnapshot {
            revision: brn_workflow::worker::DraftRevision {
                id: original,
                draft_id: d.id,
                parent_id: None,
                kind: RevisionKind::Checkpoint,
                text: text.into(),
                sha256: digest,
                origin_turn: None,
            },
            anchors: vec![(id, anchor.clone())],
        };
        (
            DraftComments {
                draft: d,
                comments: vec![DraftCommentView { comment, anchor }],
            },
            vec![snapshot],
        )
    }
    #[test]
    fn save_ack_preserves_later_edits() {
        let d = draft();
        let mut state = DraftEditor::new(d.clone());
        state.edit("two".into());
        let pending = state.begin_save(Uuid::new_v4()).unwrap();
        state.edit("three".into());
        let mut saved = d.clone();
        saved.text = "two".into();
        saved.stamp.generation = 1;
        assert!(state.acknowledge(pending.op, saved.clone()));
        assert_eq!(state.text(), "three");
        assert!(state.dirty());
        assert_eq!(
            state.begin_save(Uuid::new_v4()).unwrap().expected,
            saved.stamp
        );
    }
    #[test]
    fn checkpoint_ack_advances_base_without_replacing_newer_text() {
        let d = draft();
        let mut state = DraftEditor::new(d.clone());
        state.edit("two".into());
        let pending = state.begin_save(Uuid::new_v4()).unwrap();
        state.edit("three".into());
        let mut checked = d;
        checked.text = "two".into();
        checked.stamp.generation = 1;
        checked.stamp.base_revision = Uuid::new_v4();
        assert!(state.acknowledge(pending.op, checked.clone()));
        assert_eq!(state.text(), "three");
        assert_eq!(
            state.begin_save(Uuid::new_v4()).unwrap().expected,
            checked.stamp
        );
    }
    #[test]
    fn unrelated_draft_result_is_ignored() {
        let d = draft();
        let mut state = DraftEditor::new(d.clone());
        state.edit("two".into());
        let pending = state.begin_save(Uuid::new_v4()).unwrap();
        let mut other = draft();
        other.text = "two".into();
        other.stamp.generation = 1;
        assert!(!state.acknowledge(pending.op, other));
        assert_eq!(state.text(), "two");
        assert!(state.pending());
    }
    #[test]
    fn save_failure_keeps_dirty_text() {
        let d = draft();
        let mut state = DraftEditor::new(d);
        state.edit("two".into());
        let pending = state.begin_save(Uuid::new_v4()).unwrap();
        state.fail(pending.op);
        assert_eq!(state.text(), "two");
        assert!(state.dirty());
        assert!(!state.pending());
    }
    #[test]
    fn undo_advances_generation() {
        let d = draft();
        let mut state = DraftEditor::new(d);
        state.edit("two".into());
        state.edit("one".into());
        assert_eq!(state.generation(), 2);
        assert!(state.dirty());
    }
    #[test]
    fn dirty_switch_close_and_pending_save_guards() {
        let d = draft();
        let mut state = DraftEditor::new(d);
        assert!(state.can_replace());
        assert!(state.can_close());
        state.edit("two".into());
        assert!(!state.can_replace());
        assert!(!state.can_close());
        let pending = state.begin_save(Uuid::new_v4()).unwrap();
        assert!(!state.can_close());
        state.fail(pending.op);
        state.discard();
        assert!(state.can_close());
        assert_eq!(state.text(), "one");
    }
    #[test]
    fn open_result_does_not_replace_edits_made_while_worker_was_busy() {
        let d = draft();
        let mut state = DraftEditor::new(d);
        let incoming = draft();
        state.edit("typed during open".into());
        assert!(!state.replace(incoming));
        assert_eq!(state.text(), "typed during open");
        assert!(state.dirty());
    }
    #[test]
    fn capture_stays_stale_after_edit_and_undo() {
        let mut d = draft();
        d.text = "a 🦀 b".into();
        let mut state = DraftEditor::new(d);
        state.capture(2..6).unwrap();
        assert!(state.can_add_comment("body"));
        state.edit("a 🦀 b!".into());
        state.edit("a 🦀 b".into());
        assert!(!state.can_add_comment("body"));
        assert_eq!(state.capture_quote(), Some("🦀"));
    }
    #[test]
    fn pending_save_preserves_suffix_and_failed_save_preserves_full_trace() {
        let mut state = DraftEditor::new(draft());
        state.edit("two".into());
        let submitted = state.begin_save(Uuid::new_v4()).unwrap();
        state.edit("three".into());
        assert!(state.fail(submitted.op));
        assert_eq!(state.text(), "three");
        assert!(matches!(state.trace(), brn_workflow::EditTrace::Steps(steps) if steps.len()==2));
    }
    #[test]
    fn unsaved_composer_blocks_close_until_explicit_discard() {
        let mut state = DraftEditor::new(draft());
        state.set_composer("  comment  ".into());
        assert!(!state.can_close());
        assert!(!state.can_replace());
        state.discard_composer();
        assert!(state.can_close());
    }
    #[test]
    fn comment_ack_after_typing_preserves_newer_body_and_text() {
        let mut state = DraftEditor::new(draft());
        state.capture(0..3).unwrap();
        state.set_composer("first".into());
        let (submitted, _) = state.begin_comment(Uuid::new_v4()).unwrap();
        state.edit("one more".into());
        state.set_composer("second".into());
        let mut saved = draft();
        saved.id = submitted.id;
        saved.stamp = submitted.expected;
        saved.text = submitted.text;
        assert!(state.acknowledge_comments(
            submitted.op,
            DraftComments {
                draft: saved,
                comments: vec![]
            },
            vec![]
        ));
        assert_eq!(state.text(), "one more");
        assert_eq!(state.composer(), "second");
        assert!(state.dirty());
    }
    #[test]
    fn stale_refresh_cannot_replace_acknowledged_projection() {
        let d = draft();
        let mut state = DraftEditor::new(d.clone());
        let mut stale = d;
        stale.stamp.generation += 1;
        assert!(!state.refresh_comments(
            DraftComments {
                draft: stale,
                comments: vec![]
            },
            vec![]
        ));
        assert_eq!(state.stamp().generation, 0);
    }
    #[test]
    fn trace_overflow_remains_history_lost_after_shrinking() {
        let mut state = DraftEditor::new(draft());
        state.edit("x".repeat(brn_workflow::worker::MAX_DRAFT_BYTES + 1));
        state.edit("short".into());
        assert!(matches!(state.trace(), EditTrace::HistoryLost));
    }
    #[test]
    fn preview_maps_before_insert_and_recovers_exact_original() {
        let (saved, snapshots) = saved_comment("one", 0, 3);
        let mut state = DraftEditor::from_comments(saved, snapshots);
        state.edit("Xone".into());
        assert_eq!(
            state.preview_states().unwrap(),
            vec![AnchorState::Anchored { start: 1, end: 4 }]
        );
        state.edit("X".into());
        assert_eq!(state.preview_states().unwrap(), vec![AnchorState::Deleted]);
        state.edit("one".into());
        assert_eq!(
            state.preview_states().unwrap(),
            vec![AnchorState::Anchored { start: 0, end: 3 }]
        );
    }
    #[test]
    fn lifecycle_ack_during_dirty_edit_preserves_preview() {
        let (saved, snapshots) = saved_comment("one", 0, 3);
        let mut state = DraftEditor::from_comments(saved.clone(), snapshots.clone());
        state.edit("Xone".into());
        let mut changed = saved;
        changed.comments[0].comment.status = CommentStatus::Resolved;
        changed.comments[0].comment.status_version = 1;
        assert!(state.refresh_comments(changed, snapshots));
        assert_eq!(state.text(), "Xone");
        assert_eq!(
            state.preview_states().unwrap(),
            vec![AnchorState::Anchored { start: 1, end: 4 }]
        );
        assert_eq!(state.comments()[0].comment.status, CommentStatus::Resolved);
    }
    #[test]
    fn older_list_cannot_regress_resolved_status_at_same_stamp() {
        let (old, snapshots) = saved_comment("one", 0, 3);
        let mut newer = old.clone();
        newer.comments[0].comment.status = CommentStatus::Resolved;
        newer.comments[0].comment.status_version = 1;
        let mut state = DraftEditor::from_comments(newer, snapshots.clone());
        assert!(!state.refresh_comments(old, snapshots));
        assert_eq!(state.comments()[0].comment.status, CommentStatus::Resolved);
    }
    #[test]
    fn new_comment_ack_replays_suffix_into_its_preview() {
        let (mut saved, snapshots) = saved_comment("one", 0, 3);
        let mut state = DraftEditor::new(saved.draft.clone());
        state.capture(0..3).unwrap();
        state.set_composer("note".into());
        let (submitted, _) = state.begin_comment(Uuid::new_v4()).unwrap();
        state.edit("Xone".into());
        saved.draft.stamp.base_revision = snapshots[0].revision.id;
        assert!(state.acknowledge_comments(submitted.op, saved, snapshots));
        assert_eq!(
            state.preview_states().unwrap(),
            vec![AnchorState::Anchored { start: 1, end: 4 }]
        );
        assert_eq!(state.text(), "Xone");
    }
    #[test]
    fn failed_comment_submission_keeps_body_capture_and_trace() {
        let mut state = DraftEditor::new(draft());
        state.edit("one!".into());
        state.capture(0..3).unwrap();
        state.set_composer("note".into());
        let (submitted, _) = state.begin_comment(Uuid::new_v4()).unwrap();
        state.edit("one!!".into());
        assert!(state.fail(submitted.op));
        assert_eq!(state.composer(), "note");
        assert_eq!(state.capture_quote(), Some("one"));
        assert!(matches!(state.trace(), EditTrace::Steps(steps) if steps.len() == 2));
    }
    #[test]
    fn incremental_preview_matches_full_trace_replay() {
        let (saved, snapshots) = saved_comment("one", 0, 3);
        let mut state = DraftEditor::from_comments(saved.clone(), snapshots.clone());
        for text in ["Xone", "Xone one", "X", "one", "one!"] {
            state.edit(text.into());
            assert_eq!(
                state.preview_states(),
                comments::preview(&saved, &snapshots, text, state.trace())
            );
        }
    }
}
