use brn_workflow::worker::{Draft, DraftStamp};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submitted {
    pub op: Uuid,
    pub id: Uuid,
    pub expected: DraftStamp,
    pub generation: u64,
    pub text: String,
}

/// Tracks acknowledged bytes independently of the live editor buffer.
pub struct DraftEditor {
    acknowledged: Draft,
    generation: u64,
    text: String,
    pending: Option<Submitted>,
}
impl DraftEditor {
    pub fn new(draft: Draft) -> Self {
        Self {
            generation: draft.stamp.generation,
            text: draft.text.clone(),
            acknowledged: draft,
            pending: None,
        }
    }
    pub fn id(&self) -> Uuid {
        self.acknowledged.id
    }
    pub fn title(&self) -> &str {
        &self.acknowledged.title
    }
    pub fn stamp(&self) -> DraftStamp {
        self.acknowledged.stamp
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn dirty(&self) -> bool {
        self.generation != self.acknowledged.stamp.generation || self.text != self.acknowledged.text
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn can_replace(&self) -> bool {
        !self.dirty() && !self.pending()
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
    pub fn edit(&mut self, text: String) {
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
        };
        self.pending = Some(submitted.clone());
        Some(submitted)
    }
    pub fn acknowledge(&mut self, op: Uuid, saved: Draft) -> bool {
        let Some(submitted) = &self.pending else {
            return false;
        };
        if submitted.op != op
            || submitted.id != saved.id
            || submitted.generation != saved.stamp.generation
            || submitted.text != saved.text
        {
            return false;
        }
        self.acknowledged = saved;
        self.pending = None;
        true
    }
    pub fn fail(&mut self, op: Uuid) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.op == op)
        {
            self.pending = None;
            true
        } else {
            false
        }
    }
    pub fn discard(&mut self) -> bool {
        if self.pending() {
            return false;
        }
        self.generation = self.acknowledged.stamp.generation;
        self.text = self.acknowledged.text.clone();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
