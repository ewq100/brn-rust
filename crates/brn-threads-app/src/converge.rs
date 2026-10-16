//! Converging twins of the owner archive, unarchive and project commands
//! (owner decision B4): the same host-key prefix, command binding and
//! authority as `chat.rs` and `projects.rs`, with the core reporting
//! `Unchanged` when the requested state already holds.
use crate::*;

impl Workspace {
    fn chat_command_converged(
        &mut self,
        key: &str,
        command: ChatCommand,
    ) -> AppResult<Converged<Receipt>> {
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&command)?));
        let op = self
            .store
            .allocate_bound_operation(&format!("chat-control:{key}"), &binding)?;
        Ok(
            match self.store.chat_command_converging(
                &op,
                &command,
                &HostAuthority::owner("owner chat action"),
            )? {
                Converged::Changed(outcome) => {
                    // As `chat_command`: a Run the archive stopped leaves no free lease.
                    self.remove_ended_leases(&outcome.receipt);
                    Converged::Changed(outcome.receipt)
                }
                Converged::Unchanged(unchanged) => Converged::Unchanged(unchanged),
            },
        )
    }
    /// `archive_chat`, or `Unchanged` when the chat is already archived with its queue paused.
    pub fn archive_chat_converged(
        &mut self,
        key: &str,
        thread: &str,
    ) -> AppResult<Converged<Receipt>> {
        self.chat_command_converged(
            key,
            ChatCommand::Archive {
                thread: thread.into(),
            },
        )
    }
    /// `unarchive_chat`, or `Unchanged` when the chat is already open.
    pub fn unarchive_chat_converged(
        &mut self,
        key: &str,
        thread: &str,
    ) -> AppResult<Converged<Receipt>> {
        self.chat_command_converged(
            key,
            ChatCommand::Unarchive {
                thread: thread.into(),
            },
        )
    }
    /// `project_command`, or `Unchanged` for a chat move or document
    /// link/unlink whose state already holds.
    pub fn project_command_converged(
        &mut self,
        key: &str,
        command: ProjectCommand,
    ) -> AppResult<Converged<Receipt>> {
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&command)?));
        let op = self
            .store
            .allocate_bound_operation(&format!("project:{key}"), &binding)?;
        let outcome = self.store.project_command_converging(
            &op,
            &command,
            &HostAuthority::owner("owner project action"),
        )?;
        // As `project_command`: a Run the command stopped leaves no free lease.
        if let Converged::Changed(receipt) = &outcome {
            self.remove_ended_leases(receipt);
        }
        Ok(outcome)
    }
}
