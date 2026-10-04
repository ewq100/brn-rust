//! Retained operational Actions through the shared application boundary.
//! Current reads require durable-change reconciliation, but no vault or provider.
use crate::{ErrorKind, Result, WorkflowError, app::App};
pub use brn_store::work::actions::{
    ActionCursor, ActionData, ActionListRequest, ActionOrigin, ActionPage, ActionPriority,
    ActionRecord, ActionStamp, ActionState,
};
use uuid::Uuid;

impl App {
    /// Return the exact current baseline, including its immutable approved origin.
    pub fn action(&self, id: Uuid) -> Result<ActionRecord> {
        if id.is_nil() {
            return Err(WorkflowError::typed(
                ErrorKind::ToolRejected,
                "Action UUID must not be nil",
            ));
        }
        self.require_current_evidence()?;
        self.store
            .action(id)?
            .ok_or_else(|| WorkflowError::typed(ErrorKind::NotFound, "Action does not exist"))
    }

    /// Page by immutable creation time and UUID; edits do not reorder retained work.
    pub fn actions(&self, request: &ActionListRequest) -> Result<ActionPage> {
        request
            .validate()
            .map_err(|error| WorkflowError::typed(ErrorKind::ToolRejected, error.to_string()))?;
        self.require_current_evidence()?;
        Ok(self.store.action_list(request)?)
    }
}
