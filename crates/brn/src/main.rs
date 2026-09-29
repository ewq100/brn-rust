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
    // SAFETY: replaces the default SIGINT disposition with `on_sigint`, which
    // only stores to a lock-free atomic (async-signal-safe) and never returns.
    // Normal teardown is unaffected; the prior handler is not restored.
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
    use std::sync::{Mutex, MutexGuard};

    /// Serializes every test that flips the process-global CANCEL flag: the
    /// parallel test harness would otherwise interleave the mutations.
    static CANCEL_TESTS: Mutex<()> = Mutex::new(());

    fn cancel_lock() -> MutexGuard<'static, ()> {
        CANCEL_TESTS.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// One sequential test: CANCEL is process-global, so all cases share a
    /// single thread and the prior flag value is restored at the end.
    #[test]
    fn cancel_before_dispatch_prevents_and_after_completion_preserves() {
        let _guard = cancel_lock();
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

    /// A signal that arrives after a mutation committed must not rewrite the
    /// durable result: finish keeps the success and the persisted state
    /// carries exactly the returned ids. CANCEL is process-global, so this
    /// runs inside the same sequential test and restores the prior value.
    #[test]
    fn cancelled_after_completed_mutation_preserves_result() {
        let _guard = cancel_lock();
        let was = crate::CANCEL.load(Ordering::SeqCst);
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("note.md");
        std::fs::write(
            &file,
            "The synthetic Aurora mission launches on Tuesday.\r\n",
        )
        .unwrap();

        crate::CANCEL.store(false, Ordering::SeqCst);
        let args: Vec<String> = [
            "import",
            file.to_str().unwrap(),
            "--data-dir",
            root.to_str().unwrap(),
            "--json",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let invocation = match cli::parse(&args) {
            Ok(cli::Outcome::Run(invocation)) => invocation,
            _ => panic!("import parses into a runnable invocation"),
        };
        let output = match cli::execute(&invocation) {
            Ok(output) => output,
            Err(_) => panic!("import runs while CANCEL is clear"),
        };
        let source_id: uuid::Uuid = output.data["source_id"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .expect("source id in envelope");
        let version_id: uuid::Uuid = output.data["version_id"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .expect("version id in envelope");

        crate::CANCEL.store(true, Ordering::SeqCst);
        let code = cli::finish(invocation.json, "import", Ok(output));
        assert_eq!(code, ExitCode::SUCCESS, "completed result stands");

        let workspace = brn_workflow::Workspace::open(root, brn_workflow::Config::default())
            .expect("workspace reopens");
        let sources = workspace.sources().unwrap();
        assert_eq!(sources.len(), 1, "exactly the committed import");
        assert_eq!(sources[0].source_id, source_id);
        assert_eq!(sources[0].version_id, version_id);

        crate::CANCEL.store(was, Ordering::SeqCst);
    }
}
