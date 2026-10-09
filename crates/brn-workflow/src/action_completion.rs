//! Direct explicit completion of an exact identified Action, with durable recovery.
use crate::files::recovery::{ApplyRecoveryFiles, CompletionRecoverySnapshot};
use crate::{ErrorKind, Result, WorkflowError, app::App, editor::file_error};
use brn_store::WorkStore;
pub use brn_store::work::action_completion::{
    ActionCompletion, CompleteActionRequest, SentSourceBinding,
};
mod sent;
pub use sent::{PrepareSentCompletionRequest, SentCompletionPreview};

/// Approval evidence is imported first. Completion needs neither a vault nor a provider.
pub(crate) fn restore_action_completions(
    store: &mut WorkStore,
    records: Option<&ApplyRecoveryFiles>,
) -> Result<()> {
    let Some(records) = records else {
        return Ok(());
    };
    for id in records.completion_ids().map_err(file_error)? {
        let snapshot = records
            .read_completion(id)
            .map_err(file_error)?
            .ok_or_else(|| {
                WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "completion recovery record disappeared",
                )
            })?;
        // Re-sync a possible former post-install failure before granting authority.
        let synced: CompletionRecoverySnapshot = records
            .publish_completion(&snapshot.completion)
            .map_err(file_error)?;
        if synced != snapshot {
            return Err(WorkflowError::typed(
                ErrorKind::ContextStale,
                "completion recovery proof changed during reconciliation",
            ));
        }
        store.restore_action_completion(&synced.completion)?;
    }
    Ok(())
}

impl App {
    /// Complete only the retained unfinished record explicitly selected by a user.
    /// This command never infers real-world completion or grants an AI write tool.
    pub fn complete_action(&mut self, request: &CompleteActionRequest) -> Result<ActionCompletion> {
        request
            .validate()
            .map_err(|error| WorkflowError::typed(ErrorKind::ToolRejected, error.to_string()))?;
        // Terminal replay is bound to original bytes, before fresh eligibility.
        if let Some(completion) = self.store.action_completion_for(request)? {
            return Ok(completion);
        }
        self.application_records()?;
        let records = self.apply_records.as_ref().expect("opened recovery files");
        let retained = records
            .read_completion(request.operation_id)
            .map_err(file_error)?;
        if retained
            .as_ref()
            .is_some_and(|snapshot| snapshot.completion.request != *request)
        {
            return Err(WorkflowError::typed(
                ErrorKind::OperationConflict,
                "completion operation UUID has another exact request",
            ));
        }
        if retained.is_some() || self.completion_uncertain {
            // Validate the whole family before clearing its fence; temporary files
            // never grant completion authority. Absence can be retried as fresh work.
            self.completion_uncertain = true;
            self.set_current_tool_barrier(true);
            restore_action_completions(&mut self.store, self.apply_records.as_ref())?;
            self.completion_uncertain = false;
            self.synchronize_current_barrier()?;
            if let Some(completion) = self.store.action_completion_for(request)? {
                return Ok(completion);
            }
        }
        self.require_current_evidence()?;
        if let Some(binding) = &request.sent_source {
            self.validate_sent_action_source(binding)?;
        }
        let records = self.apply_records.as_ref().expect("opened recovery files");
        let mut publication_error = None;
        let mut published = false;
        let at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| WorkflowError::msg("system clock precedes Unix epoch"))?
            .as_millis()
            .try_into()
            .map_err(|_| WorkflowError::msg("system clock exceeds completion range"))?;
        let result = self
            .store
            .complete_action_with(request, at_ms, |completion| {
                let publication: std::result::Result<CompletionRecoverySnapshot, _> =
                    records.publish_completion(completion);
                match publication {
                    Ok(_) => {
                        published = true;
                        checkpoint("published")
                    }
                    Err(error) => {
                        publication_error = Some(error);
                        Err(brn_store::Error::Io(std::io::Error::other(
                            "completion evidence publication failed",
                        )))
                    }
                }
            });
        match result {
            Ok(completion) => {
                checkpoint("settled")?;
                Ok(completion)
            }
            Err(error) => {
                #[cfg(target_os = "macos")]
                let uncertain_publication = publication_error.as_ref().is_some_and(|failure| {
                    failure.filesystem_outcome == crate::files::FileOutcome::Unknown
                });
                #[cfg(not(target_os = "macos"))]
                let uncertain_publication = false;
                if published || uncertain_publication {
                    self.completion_uncertain = true;
                    self.set_current_tool_barrier(true);
                    return Err(WorkflowError::typed(
                        ErrorKind::SaveUncertain,
                        "Action completion needs exact retry or restart reconciliation",
                    ));
                }
                Err(publication_error
                    .map(file_error)
                    .unwrap_or_else(|| error.into()))
            }
        }
    }
}

fn checkpoint(_phase: &str) -> brn_store::Result<()> {
    #[cfg(all(test, target_os = "macos"))]
    {
        if CRASH_PHASE.with(|phase| phase.borrow().as_deref() == Some(_phase)) {
            std::process::exit(73);
        }
        if FAILURE.with(|phase| phase.get() == Some(_phase)) {
            return Err(brn_store::Error::Io(std::io::Error::other(
                "injected completion settlement failure",
            )));
        }
    }
    Ok(())
}
#[cfg(all(test, target_os = "macos"))]
thread_local! {
    static CRASH_PHASE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    static FAILURE: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}
#[cfg(all(test, target_os = "macos"))]
mod tests;
