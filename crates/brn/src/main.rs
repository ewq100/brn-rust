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
        cli::Command::Ai(command) => command.name(),
        cli::Command::ModelDownload { .. } => "models.download",
        cli::Command::NotesList { .. } => "notes.list",
        cli::Command::NotePath(_) => "notes.show",
        cli::Command::Notes(command) => command.name(),
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
        cli::Command::DraftsCreate { .. } => "drafts.create",
        cli::Command::DraftsCheckpoint { .. } => "drafts.checkpoint",
        cli::Command::DraftsSave { .. } => "drafts.save",
        cli::Command::DraftsShow { .. } => "drafts.show",
        cli::Command::CommentsList { .. } => "comments.list",
        cli::Command::CommentsAdd { .. } => "comments.add",
        cli::Command::CommentsResolve { .. } => "comments.resolve",
        cli::Command::CommentsReopen { .. } => "comments.reopen",
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

    /// Holds the shared cancellation lock for the whole test, installs the
    /// required initial CANCEL value and restores the prior one on drop, so a
    /// panicking test cannot poison the process-global flag for the others.
    struct CancelTestGuard {
        previous: bool,
        _lock: MutexGuard<'static, ()>,
    }

    impl CancelTestGuard {
        fn with(value: bool) -> Self {
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
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn note_save_durable_receipt_survives_signal_after_completion() {
        let _cancel = CancelTestGuard::with(false);
        let data = tempfile::Builder::new()
            .tempdir_in(std::env::current_dir().unwrap())
            .unwrap();
        let vault = tempfile::Builder::new()
            .tempdir_in(std::env::current_dir().unwrap())
            .unwrap();
        let path = vault.path().join("plan.md");
        std::fs::write(&path, "base").unwrap();
        let mut workspace =
            brn_workflow::Workspace::open(data.path(), brn_workflow::Config::default()).unwrap();
        let view = workspace
            .open_note(
                uuid::Uuid::new_v4(),
                vault.path(),
                std::path::Path::new("plan.md"),
            )
            .unwrap();
        drop(workspace);
        let file = data.path().join("text.txt");
        std::fs::write(&file, "saved\r\n").unwrap();
        let operation = uuid::Uuid::new_v4();
        let invocation = cli::Invocation {
            json: true,
            data_dir: data.path().to_owned(),
            model_dir: None,
            vault: None,
            credentials_dir: None,
            legacy: false,
            command: cli::Command::Notes(cli::notes::NoteCommand::Write {
                kind: cli::notes::WriteKind::Save,
                note: view.id,
                expected: view.stamp,
                generation: 1,
                text_file: file,
                operation: Some(operation),
            }),
        };
        let result = cli::execute(&invocation).unwrap_or_else(|_| panic!("note save completes"));
        on_sigint(libc::SIGINT);
        let mut bytes = Vec::new();
        let (exit, diagnostic) = cli::out::finish_ok_to(true, "notes.save", result, &mut bytes);
        assert_eq!(exit, ExitCode::SUCCESS);
        assert!(diagnostic.is_none());
        let envelope: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(envelope["data"]["filesystem_outcome"], "Applied");
        assert_eq!(envelope["data"]["operation_id"], operation.to_string());
        assert_eq!(std::fs::read(&path).unwrap(), b"saved\r\n");
        let mut workspace =
            brn_workflow::Workspace::open(data.path(), brn_workflow::Config::default()).unwrap();
        assert_eq!(
            workspace
                .reconcile_note_save(operation)
                .unwrap()
                .filesystem_outcome,
            brn_workflow::notes::FileOutcome::Applied
        );
    }

    /// A signal that has arrived by the end of input preparation must
    /// prevent the workspace from being opened at all: drafts create
    /// validates the title, reads the file and only then checks cancellation
    /// BEFORE acquiring the workspace — so the data directory stays free of
    /// initialization artifacts and no mutation is attempted. Deterministic:
    /// CANCEL is set before execute() runs, no sleeps; the guard serializes
    /// with the other CANCEL tests and restores the prior flag on drop.
    #[test]
    fn cancel_after_input_preparation_prevents_workspace_and_mutation() {
        let _cancel = CancelTestGuard::with(true);
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        std::fs::create_dir(&data).unwrap();
        // The input fixture lives outside the data directory.
        let fixtures = tempfile::tempdir().unwrap();
        let file = fixtures.path().join("note.txt");
        std::fs::write(&file, "body\n").unwrap();

        let args: Vec<String> = [
            "drafts",
            "create",
            "--title",
            "t",
            "--text-file",
            file.to_str().unwrap(),
            "--data-dir",
            data.to_str().unwrap(),
            "--json",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let invocation = match cli::parse(&args) {
            Ok(cli::Outcome::Run(invocation)) => invocation,
            _ => panic!("drafts create parses into a runnable invocation"),
        };
        let failure = match cli::execute(&invocation) {
            Ok(_) => panic!("cancelled command must not run"),
            Err(failure) => failure,
        };
        assert!(
            matches!(failure.error, crate::cli::error::CliError::Interrupted(_)),
            "expected INTERRUPTED, got {:?}",
            failure.error
        );
        let entries: Vec<_> = std::fs::read_dir(&data).unwrap().collect();
        assert!(entries.is_empty(), "workspace was initialized: {entries:?}");
    }

    #[test]
    fn cancel_before_checkpoint_workspace_access_prevents_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let input = tempfile::tempdir().unwrap();
        let file = input.path().join("checkpoint.txt");
        std::fs::write(&file, "next\n").unwrap();

        let mut workspace =
            brn_workflow::Workspace::open(root, brn_workflow::Config::default()).unwrap();
        let draft = workspace
            .create_draft(uuid::Uuid::new_v4(), "checkpoint", "base\n")
            .unwrap();
        drop(workspace);

        let _cancel = CancelTestGuard::with(true);
        let args: Vec<String> = [
            "drafts",
            "checkpoint",
            &draft.id.to_string(),
            "--base-revision",
            &draft.stamp.base_revision.to_string(),
            "--expected-generation",
            "0",
            "--generation",
            "1",
            "--text-file",
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
            _ => panic!("checkpoint parses into a runnable invocation"),
        };
        let failure = match cli::execute(&invocation) {
            Ok(_) => panic!("cancelled checkpoint must not run"),
            Err(failure) => failure,
        };
        assert!(
            matches!(failure.error, crate::cli::error::CliError::Interrupted(_)),
            "expected INTERRUPTED, got {:?}",
            failure.error
        );

        let workspace =
            brn_workflow::Workspace::open(root, brn_workflow::Config::default()).unwrap();
        let unchanged = workspace.draft(draft.id).unwrap().unwrap();
        assert_eq!(unchanged, draft);
        assert_eq!(workspace.draft_revisions(draft.id).unwrap().len(), 1);
    }

    /// A signal that arrives after a mutation committed must not rewrite the
    /// durable result: finish keeps the success and the persisted state
    /// carries exactly the returned ids. CANCEL is process-global, so the
    /// guard installs the required clear flag and restores the prior value
    /// on drop, panic-safe.
    #[test]
    fn cancelled_after_completed_mutation_preserves_result() {
        let _cancel = CancelTestGuard::with(false);
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("note.md");
        std::fs::write(
            &file,
            "The synthetic Aurora mission launches on Tuesday.\r\n",
        )
        .unwrap();

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

        let mut workspace = brn_workflow::Workspace::open(root, brn_workflow::Config::default())
            .expect("workspace reopens");
        let sources = workspace.sources().unwrap();
        assert_eq!(sources.len(), 1, "exactly the committed import");
        assert_eq!(sources[0].source_id, source_id);
        assert_eq!(sources[0].version_id, version_id);
    }

    /// A stdout that cannot take the envelope must not rewrite the durable
    /// result: the finish core exits 1 with a completed-but-undelivered
    /// diagnostic carrying the real operation id, and the committed import
    /// is still on disk with exactly the returned ids.
    #[test]
    fn failed_delivery_after_completed_import_exits_1_and_keeps_result() {
        // CANCEL is process-global: the guard installs the required clear
        // flag and restores it on drop, serializing with the other CANCEL
        // tests.
        let _cancel = CancelTestGuard::with(false);
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("note.md");
        std::fs::write(
            &file,
            "The synthetic Aurora mission launches on Tuesday.\r\n",
        )
        .unwrap();

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
            Err(_) => panic!("import runs on a fresh workspace"),
        };
        let source_id = output.data["source_id"]
            .as_str()
            .expect("source id")
            .to_string();
        let version_id = output.data["version_id"]
            .as_str()
            .expect("version id")
            .to_string();
        let operation_id = output.data["operation_id"]
            .as_str()
            .expect("operation id")
            .to_string();

        struct Dead;
        impl std::io::Write for Dead {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "sink unavailable",
                ))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let (code, diagnostic) =
            cli::out::finish_ok_to(invocation.json, "import", output, &mut Dead);
        assert_eq!(
            code,
            ExitCode::from(1),
            "undelivered completed result exits 1"
        );
        let diagnostic = diagnostic.expect("diagnostic accompanies the undelivered result");
        assert!(diagnostic.contains("OUTPUT_DELIVERY_ERROR"), "{diagnostic}");
        assert!(diagnostic.contains("completed"), "{diagnostic}");
        assert!(diagnostic.contains(&operation_id), "{diagnostic}");
        assert!(
            !diagnostic.to_lowercase().contains("failed"),
            "{diagnostic}"
        );

        let mut workspace = brn_workflow::Workspace::open(root, brn_workflow::Config::default())
            .expect("workspace reopens");
        let sources = workspace.sources().unwrap();
        assert_eq!(sources.len(), 1, "exactly the committed import");
        assert_eq!(sources[0].source_id.to_string(), source_id);
        assert_eq!(sources[0].version_id.to_string(), version_id);
    }
}
