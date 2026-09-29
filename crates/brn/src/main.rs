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

fn run(args: &[String]) -> ExitCode {
    match cli::parse(args) {
        Ok(Outcome::Help) => {
            println!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Ok(Outcome::Version) => {
            println!("{}", cli::version_line());
            ExitCode::SUCCESS
        }
        Ok(Outcome::Run(invocation)) => {
            let command = command_name(&invocation.command);
            cli::finish(invocation.json, command, cli::execute(&invocation))
        }
        Err(failure) => {
            cli::report_error(failure.json, failure.command, &failure.error);
            ExitCode::from(failure.error.exit_code())
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
