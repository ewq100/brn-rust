//! Host coordination around the runtime's single durable prepare/run seam.
use crate::*;
use brn_ai::{Provider, ReasoningEffort, Selection, threads::*};
use std::sync::Arc;

pub struct ThreadRunStart {
    pub thread: String,
    pub prompt: String,
    pub request_message: Option<String>,
    pub queue_claim: Option<String>,
    pub owner_request: Option<OwnerRequest>,
    pub sources: Vec<TransientSource>,
    pub max_tool_rounds: Option<u16>,
    pub trace: TraceLevel,
    /// A host's already obtained catalogue, never a model-supplied choice.
    pub offered_copilot_models: Option<Vec<String>>,
}
#[derive(Debug, thiserror::Error)]
pub enum RunSelectionError {
    #[error("Choose a provider and model in Settings before asking BRN.")]
    Provider,
    #[error("Choose a supported reasoning effort in Settings.")]
    Effort,
    #[error(transparent)]
    Model(brn_ai::AiError),
    #[error(
        "{model} is not offered by GitHub Copilot, so BRN has not started. Choose a model from the model menu: {offered}."
    )]
    CopilotCatalogue { model: String, offered: String },
}
#[derive(Debug, thiserror::Error)]
pub enum RunPreparationError {
    #[error(transparent)]
    Selection(RunSelectionError),
    #[error(transparent)]
    Preparation(brn_ai::AiError),
    #[error(transparent)]
    Settings(AppError),
}
impl From<RunPreparationError> for AppError {
    fn from(error: RunPreparationError) -> Self {
        match error {
            RunPreparationError::Settings(error) => error,
            RunPreparationError::Selection(error) => AppError::Invalid(error.to_string()),
            RunPreparationError::Preparation(error) => AppError::Invalid(error.to_string()),
        }
    }
}
pub struct PreparedWorkspaceRun {
    pub prepared: PreparedThreadRun,
    pub search_status: String,
}
/// Validate selection before allocating a Run or entering any writing workflow.
pub fn resolve_run_selection(
    settings: &WorkspaceSettings,
    offered: Option<&[String]>,
) -> std::result::Result<(Selection, ReasoningEffort), RunSelectionError> {
    let provider = match settings.provider.to_ascii_lowercase().as_str() {
        "codex" | "chatgpt" => Provider::Chatgpt,
        "copilot" => Provider::Copilot,
        _ => {
            return Err(RunSelectionError::Provider);
        }
    };
    if provider == Provider::Copilot
        && offered.is_some_and(|models| !models.is_empty() && !models.contains(&settings.model))
    {
        return Err(RunSelectionError::CopilotCatalogue {
            model: settings.model.clone(),
            offered: offered.unwrap_or_default().join(", "),
        });
    }
    let effort = match settings.effort.as_str() {
        "low" => ReasoningEffort::Low,
        "medium" => ReasoningEffort::Medium,
        "high" => ReasoningEffort::High,
        _ => {
            return Err(RunSelectionError::Effort);
        }
    };
    let selection = Selection {
        provider,
        model: settings.model.clone(),
    };
    selection.validate().map_err(RunSelectionError::Model)?;
    Ok((selection, effort))
}
impl Workspace {
    pub fn prepare_thread_run(
        &mut self,
        start: ThreadRunStart,
    ) -> std::result::Result<PreparedWorkspaceRun, RunPreparationError> {
        let settings = self.settings().map_err(RunPreparationError::Settings)?;
        let (selection, effort) =
            resolve_run_selection(&settings, start.offered_copilot_models.as_deref())
                .map_err(RunPreparationError::Selection)?;
        let (search, search_status) = match self.search_handle() {
            Ok(handle) => (
                Some(Arc::new(handle) as Arc<dyn RunNoteSearch>),
                "available".into(),
            ),
            Err(error) => (None, format!("unavailable: {error}")),
        };
        let request = ThreadRunRequest {
            request_message: start.request_message,
            queue_claim: start.queue_claim,
            owner_request: start.owner_request,
            data_dir: self.store.directory().to_owned(),
            thread: start.thread,
            prompt: start.prompt,
            selection,
            effort,
            max_tool_rounds: start
                .max_tool_rounds
                .unwrap_or_else(|| default_tool_rounds(!start.sources.is_empty())),
            auth_file: settings.auth_file.map(Into::into),
            credentials_dir: settings.credentials_dir.map(Into::into),
            sources: start.sources,
            search,
            trace: start.trace,
        };
        let prepared = prepare_run(request).map_err(RunPreparationError::Preparation)?;
        Ok(PreparedWorkspaceRun {
            prepared,
            search_status,
        })
    }
}
