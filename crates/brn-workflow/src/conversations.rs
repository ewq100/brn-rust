//! Reversible session organization; captured outcomes retain their authority.
pub use brn_store::work::conversations::{
    ConversationFilter, ConversationLifecycle, ConversationLifecycleReceipt,
    ConversationLifecycleRequest, ConversationLifecycleResult, ConversationStamp,
    ConversationState, ConversationSummary,
};

pub(crate) fn store_error(error: brn_store::Error) -> crate::WorkflowError {
    match error {
        brn_store::Error::WorkspaceBusy(message) => {
            crate::WorkflowError::typed(crate::ErrorKind::ToolsBusy, message)
        }
        error => error.into(),
    }
}
