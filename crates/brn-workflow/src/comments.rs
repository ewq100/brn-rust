use crate::{Result, Workspace, error, error::WorkflowError};
use brn_store::{
    CommentAnchorSnapshot, CommentCapture, CommentCreated, CommentStatusChange,
    CommentStatusChanged, DraftComments, DraftWriteWithComments,
};
use uuid::Uuid;

impl Workspace {
    pub fn draft_comments(&self, draft_id: Uuid) -> Result<DraftComments> {
        self.store.draft_comments(draft_id).map_err(error)
    }

    pub fn comment_anchor_snapshots(&self, draft_id: Uuid) -> Result<Vec<CommentAnchorSnapshot>> {
        self.store.comment_anchor_snapshots(draft_id).map_err(error)
    }

    pub fn create_draft_comment(&mut self, request: CommentCapture) -> Result<CommentCreated> {
        self.store
            .create_draft_comment(request)
            .map_err(WorkflowError::from)
    }

    pub fn write_draft_with_comments(
        &mut self,
        request: DraftWriteWithComments,
    ) -> Result<DraftComments> {
        self.store.write_draft_with_comments(request).map_err(error)
    }

    pub fn set_comment_status(
        &mut self,
        request: CommentStatusChange,
    ) -> Result<CommentStatusChanged> {
        self.store
            .set_comment_status(request)
            .map_err(crate::WorkflowError::from)
    }
}
