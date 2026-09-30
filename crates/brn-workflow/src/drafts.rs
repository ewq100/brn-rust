use crate::{Result, Workspace, error, error::WorkflowError};
use brn_store::{Draft, DraftRevision, DraftStamp};
use uuid::Uuid;

impl Workspace {
    pub fn create_draft(&mut self, op: Uuid, title: &str, text: &str) -> Result<Draft> {
        // Preserve the store's typed categories (notably OperationConflict for
        // operation replay/reuse) instead of flattening them to `Other`.
        self.store
            .create_draft(op, title, text)
            .map_err(WorkflowError::from)
    }
    pub fn drafts(&self) -> Result<Vec<Draft>> {
        self.store.drafts().map_err(error)
    }
    pub fn draft(&self, id: Uuid) -> Result<Option<Draft>> {
        self.store.draft(id).map_err(error)
    }
    pub fn save_draft(
        &mut self,
        op: Uuid,
        id: Uuid,
        expected: DraftStamp,
        generation: u64,
        text: &str,
    ) -> Result<Draft> {
        self.store
            .save_draft(op, id, expected, generation, text)
            .map_err(error)
    }
    pub fn checkpoint_draft(
        &mut self,
        op: Uuid,
        id: Uuid,
        expected: DraftStamp,
        generation: u64,
        text: &str,
    ) -> Result<Draft> {
        self.store
            .checkpoint_draft(op, id, expected, generation, text)
            .map_err(error)
    }
    pub fn draft_revisions(&self, id: Uuid) -> Result<Vec<DraftRevision>> {
        self.store.draft_revisions(id).map_err(error)
    }
    pub fn draft_revision(&self, id: Uuid) -> Result<Option<DraftRevision>> {
        self.store.draft_revision(id).map_err(error)
    }
    pub fn candidate_from_turn(
        &mut self,
        op: Uuid,
        id: Uuid,
        parent: Uuid,
        turn: Uuid,
    ) -> Result<DraftRevision> {
        self.store
            .candidate_from_turn(op, id, parent, turn)
            .map_err(error)
    }
    pub fn compare_draft_revisions(
        &self,
        draft: Uuid,
        before: Uuid,
        after: Uuid,
    ) -> Result<String> {
        let first = self
            .draft_revision(before)?
            .ok_or("before revision does not exist")?;
        let second = self
            .draft_revision(after)?
            .ok_or("after revision does not exist")?;
        if first.draft_id != draft || second.draft_id != draft {
            return Err("revision does not belong to selected draft".into());
        }
        if first.text == second.text {
            return Ok(format!(
                "No changes between revisions {before} and {after}."
            ));
        }
        let diff = similar::TextDiff::from_lines(&first.text, &second.text);
        Ok(diff
            .unified_diff()
            .header(&before.to_string(), &after.to_string())
            .to_string())
    }
}
