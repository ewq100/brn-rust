//! brn CLI entry point: SIGINT wiring, argument parsing, dispatch and exit codes.
mod cli;

use cli::Outcome;
use std::{
    process::ExitCode,
    sync::atomic::{AtomicBool, Ordering},
};

/// Set by the SIGINT handler; later-phase commands pass this as workflow
/// cancellation. The unsafe surface is exactly the one `libc::signal`
/// registration below; the handler only stores to a lock-free atomic.
pub static CANCEL: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigint(_signal: libc::c_int) {
    CANCEL.store(true, Ordering::SeqCst);
}

fn main() -> ExitCode {
    // SIGPIPE keeps Rust's default disposition (ignored → pipe writes return
    // EPIPE errors instead of killing the process): a broken provider stdin
    // pipe or a closed stdout must surface as a structured error or quiet
    // success, never terminate brn. The SIGINT handler only sets a flag.
    // SAFETY: replaces the default SIGINT disposition with a handler that only
    // sets an atomic flag (async-signal-safe). It never returns, so normal
    // teardown is unaffected. Restoring the prior handler is not attempted.
    unsafe {
        libc::signal(
            libc::SIGINT,
            on_sigint as extern "C" fn(libc::c_int) as libc::sighandler_t,
        );
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    run(&args)
}

/// Dispatch policy around cancellation: a SIGINT requested before a command
/// starts prevents it (checked below, before `execute` — no operation is
/// attempted). Once a command returns, its result is durably committed: the
/// completed result and its IDs stand, even if a signal arrived during the
/// run. Ask classifies its own cancellation.
fn run(args: &[String]) -> ExitCode {
    use std::io::Write as _;
    match cli::parse(args) {
        Ok(Outcome::Help) => {
            let _ = writeln!(std::io::stdout(), "{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Ok(Outcome::Version) => {
            let _ = writeln!(std::io::stdout(), "{}", cli::version_line());
            ExitCode::SUCCESS
        }
        Ok(Outcome::Run(invocation)) => {
            let command = command_name(&invocation.command);
            let is_ask = matches!(invocation.command, cli::Command::Ask { .. });
            if !is_ask && crate::CANCEL.load(Ordering::SeqCst) {
                return cli::finish(
                    invocation.json,
                    command,
                    Err(cli::error::CliError::Interrupted(
                        "interrupted before the command started; no operation was attempted".into(),
                    )
                    .into()),
                );
            }
            let result = cli::execute(&invocation);
            cli::finish(invocation.json, command, result)
        }
        Err(failure) => {
            let code = failure.error.exit_code();
            // Parse failures carry no context.
            cli::report_error(failure.json, failure.command, &failure.error.into());
            ExitCode::from(code)
        }
    }
}

/// The dotted envelope name is unique per parsed command, so a single match on
/// the variant is sufficient.
fn command_name(command: &cli::Command) -> &'static str {
    match command {
        cli::Command::Status => "status",
        cli::Command::Import { .. } => "import",
        cli::Command::DocumentsList => "documents.list",
        cli::Command::DocumentsShow { .. } => "documents.show",
        cli::Command::DocumentsSetApproval { .. } => "documents.set-search-approval",
        cli::Command::IndexBuild => "index.build",
        cli::Command::Search { .. } => "search",
        cli::Command::Ask { .. } => "ask",
        cli::Command::ConversationsList => "conversations.list",
        cli::Command::ConversationsShow { .. } => "conversations.show",
        cli::Command::DraftsList => "drafts.list",
        cli::Command::DraftsShow { .. } => "drafts.show",
        cli::Command::CommentsList { .. } => "comments.list",
        cli::Command::RevisionsList { .. } => "revisions.list",
        cli::Command::RevisionsShow { .. } => "revisions.show",
        cli::Command::RevisionsDiff { .. } => "revisions.diff",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Output;
    use std::sync::atomic::Ordering;

    /// One sequential test: CANCEL is process-global, so all cases share a
    /// single thread and the prior flag value is restored at the end.
    #[test]
    fn cancel_before_dispatch_prevents_and_after_completion_preserves() {
        let was = crate::CANCEL.load(Ordering::SeqCst);
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // (a) CANCEL set before a non-ask command: refused before any work,
        // exit 130, and the data directory is never even initialized.
        crate::CANCEL.store(true, Ordering::SeqCst);
        let args: Vec<String> = ["status", "--data-dir", root.to_str().unwrap(), "--json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(run(&args), ExitCode::from(130), "pre-dispatch refusal");
        assert!(
            !root.join("brn.sqlite3").exists(),
            "no operation was attempted"
        );

        // (b) A signal that arrives after completion must not convert a
        // finished result into INTERRUPTED: finish keeps it a success.
        let code = cli::finish(
            false,
            "status",
            Ok(Output {
                text: String::new(),
                data: serde_json::json!(null),
            }),
        );
        assert_eq!(code, ExitCode::SUCCESS, "completed result stands");

        crate::CANCEL.store(was, Ordering::SeqCst);
    }
}
