//! Shared application workflow for the desktop and agent-facing CLI.
pub mod action_completion;
pub mod actions;
pub mod activity;
mod ai_behavior;
pub mod ai_tools;
pub mod app;
pub mod app_worker;
pub mod chat_worker;
pub mod dashboard;
pub mod editor;
pub mod error;
mod files;
pub mod findings;
pub mod inbox;
pub mod inbox_actions;
pub mod inbox_original_operations;
pub mod inbox_processing;
pub mod inbox_removal;
pub mod knowledge;
pub mod library;
pub mod models;
#[cfg(all(test, feature = "native-retrieval"))]
mod models_tests;
pub mod proposal_apply;
pub mod proposal_rewrite;
pub mod proposals;
#[cfg(test)]
mod simple_worker_tests;
pub mod vault;

pub use brn_ai::{
    AccountStatus, AiError, AiErrorKind, Auth, HistoryPair, LoginPrompt, ModelOption, NoteEntry,
    NotePage, Passage, Provider, ReadTools, ReasoningEffort, Selection, ToolNote, ToolSearch,
};
pub use brn_store::work::{MAX_NOTE_BYTES, WorkConversation, WorkTurn, WorkTurnStatus};
pub use brn_store::workspace_mode::WorkspaceMode;
pub use error::{ErrorKind, WorkflowError};
pub type Result<T> = std::result::Result<T, WorkflowError>;

// A fork can briefly inherit another test's CLOEXEC ownership descriptors.
// Serialize subprocess launches against the ownership release/reacquire proof.
#[cfg(all(test, target_os = "macos"))]
pub(crate) static SUBPROCESS_FIXTURES: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Classify authority markers without opening, inspecting or migrating old data.
pub fn workspace_mode(data: &std::path::Path) -> Result<WorkspaceMode> {
    Ok(brn_store::workspace_mode::classify(data)?)
}
pub fn native_retrieval_compiled() -> bool {
    cfg!(feature = "native-retrieval")
}
