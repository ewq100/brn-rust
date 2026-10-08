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
    match cli::parse(args) {
        Ok(Outcome::Help) => deliver_stdout(&format!("{}\n", cli::HELP), "help"),
        Ok(Outcome::Version) => deliver_stdout(&format!("{}\n", cli::version_line()), "version"),
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

/// Deliver one static stdout payload (help, version) under the shared
/// output-delivery policy: closed pipe exits 0 quietly, any other write or
/// flush failure exits 1 with a best-effort stderr diagnostic.
fn deliver_stdout(payload: &str, what: &str) -> ExitCode {
    use std::io::Write as _;
    let mut stdout = std::io::stdout();
    match cli::out::deliver(&mut stdout, payload) {
        cli::out::Delivery::Delivered | cli::out::Delivery::ClosedPipe => ExitCode::SUCCESS,
        cli::out::Delivery::Failed(e) => {
            let diagnostic = cli::out::delivery_failure_diag(what, None, &e);
            let _ = writeln!(std::io::stderr(), "{diagnostic}");
            ExitCode::from(1)
        }
    }
}

/// The dotted envelope name is unique per parsed command, so a single match on
/// the variant is sufficient.
fn command_name(command: &cli::Command) -> &'static str {
    match command {
        cli::Command::Editor(command) => command.name(),
        cli::Command::Proposals(command) => command.name(),
        cli::Command::Identity(command) => command.name(),
        cli::Command::Evidence(command) => command.name(),
        cli::Command::Provenance(command) => command.name(),
        cli::Command::Links(command) => command.name(),
        cli::Command::Findings(command) => command.name(),
        cli::Command::Actions(command) => command.name(),
        cli::Command::Inbox(command) => command.name(),
        cli::Command::Relationships(_) => "relationships.list",
        cli::Command::Activity(_) => "activity.list",
        cli::Command::Ai(command) => command.name(),
        cli::Command::ModelDownload { .. } => "models.download",
        cli::Command::NotesList { .. } => "notes.list",
        cli::Command::NotePath { .. } => "notes.show",
        cli::Command::Status => "status",
        cli::Command::Search { .. } => "search",
        cli::Command::Ask { .. } => "ask",
        cli::Command::ConversationsList { .. } => "conversations.list",
        cli::Command::ConversationsSetLifecycle { target, .. } => match target {
            brn_workflow::conversations::ConversationState::Archived => "conversations.archive",
            brn_workflow::conversations::ConversationState::Active => "conversations.restore",
        },
        cli::Command::ConversationsShow { .. } => "conversations.show",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Output;
    use std::sync::atomic::Ordering;
    use std::sync::{Mutex, MutexGuard};

    /// Serializes tests that change CANCEL or execute commands that read it.
    /// Locking only writers lets a parallel command observe another test's signal.
    static CANCEL_TESTS: Mutex<()> = Mutex::new(());

    fn cancel_lock() -> MutexGuard<'static, ()> {
        CANCEL_TESTS.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Holds the shared cancellation lock for the whole test, installs the
    /// required initial CANCEL value and restores the prior one on drop, so a
    /// panicking test cannot poison the process-global flag for the others.
    pub(crate) struct CancelTestGuard {
        previous: bool,
        _lock: MutexGuard<'static, ()>,
    }

    impl CancelTestGuard {
        pub(crate) fn with(value: bool) -> Self {
            let _lock = cancel_lock();
            let previous = crate::CANCEL.swap(value, Ordering::SeqCst);
            Self { previous, _lock }
        }
    }

    impl Drop for CancelTestGuard {
        fn drop(&mut self) {
            crate::CANCEL.store(self.previous, Ordering::SeqCst);
        }
    }

    /// One sequential test: CANCEL is process-global, so the guard
    /// serializes with the other CANCEL tests and restores the prior flag
    /// on drop, panic-safe.
    #[test]
    fn cancel_before_dispatch_prevents_and_after_completion_preserves() {
        let _cancel = CancelTestGuard::with(true);
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
            !root.join("brn.sqlite").exists(),
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
    }

    #[test]
    fn cancel_after_editor_input_preparation_prevents_storage() {
        let _cancel = CancelTestGuard::with(true);
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.txt");
        std::fs::write(&input, "unfinished\r\n").unwrap();
        let data = dir.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let args = [
            "edit",
            "recover",
            "plan.md",
            "--baseline",
            "00000000-0000-0000-0000-000000000001",
            "--expected-generation",
            "0",
            "--generation",
            "1",
            "--file",
            input.to_str().unwrap(),
            "--data-dir",
            data.to_str().unwrap(),
            "--json",
        ]
        .map(str::to_owned);
        let cli::Outcome::Run(invocation) =
            cli::parse(&args).unwrap_or_else(|_| panic!("valid editor input"))
        else {
            panic!("run");
        };
        let failure = cli::execute(&invocation)
            .err()
            .expect("cancelled input refuses");
        assert_eq!(failure.error.code(), "INTERRUPTED");
        assert_eq!(std::fs::read_dir(data).unwrap().count(), 0);
    }
}
