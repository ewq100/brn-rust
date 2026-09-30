//! Output-delivery policy for stdout: a consumer that closed its pipe is
//! never a domain failure (quiet success), while any other stdout write or
//! flush error after a completed operation exits 1 with a stderr diagnostic
//! that identifies `OUTPUT_DELIVERY_ERROR` and states the result exists but
//! was not delivered. Error reporting keeps the original exit code whatever
//! happens to the report write. The SIGPIPE disposition is never changed.

use super::{envelope_err, envelope_ok, CliFailure, Output};
use std::io::{self, Write};
use std::process::ExitCode;

/// Outcome of delivering one stdout payload.
#[derive(Debug)]
pub(crate) enum Delivery {
    Delivered,
    ClosedPipe,
    Failed(io::Error),
}

/// Write `payload` (newline-terminated when a line is expected) and flush.
/// A broken pipe from either step is `ClosedPipe`; any other error is
/// `Failed`.
pub(crate) fn deliver(out: &mut impl Write, payload: &str) -> Delivery {
    match out.write_all(payload.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => Delivery::Delivered,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Delivery::ClosedPipe,
        Err(e) => Delivery::Failed(e),
    }
}

/// Pure diagnostic for an undeliverable result: names the delivery failure,
/// states that the operation completed (its result exists) but could not be
/// delivered, and carries the operation id when known.
pub(crate) fn delivery_failure_diag(
    what: &str,
    operation_id: Option<&str>,
    e: &io::Error,
) -> String {
    let id = operation_id
        .map(|id| format!(" (operation_id: {id})"))
        .unwrap_or_default();
    format!(
        "OUTPUT_DELIVERY_ERROR: the {what} operation completed, but its result could not be delivered to stdout: {e}{id}"
    )
}

/// Injectable Ok-path core of `finish`: deliver the success envelope (`--json`)
/// or the human text to `out`. Returns the process exit code plus, when the
/// result could not be delivered (anything but a closed pipe), the stderr
/// diagnostic to emit; nothing further is written to stdout.
pub(crate) fn finish_ok_to(
    json: bool,
    command: &str,
    output: Output,
    out: &mut impl Write,
) -> (ExitCode, Option<String>) {
    let operation_id = output.data["operation_id"].as_str().map(str::to_string);
    let payload = if json {
        format!("{}\n", envelope_ok(command, output.data))
    } else {
        output.text
    };
    match deliver(out, &payload) {
        Delivery::Delivered | Delivery::ClosedPipe => (ExitCode::SUCCESS, None),
        Delivery::Failed(e) => (
            ExitCode::from(1),
            Some(delivery_failure_diag(command, operation_id.as_deref(), &e)),
        ),
    }
}

/// Injectable core of `report_error`: deliver the failure report to `out`
/// (the stdout envelope in `--json` mode, the stderr line otherwise). The
/// error's own exit code is returned unchanged whatever happens to the write;
/// a non-pipe failure of the json envelope yields a best-effort stderr
/// diagnostic, while a closed pipe stays quiet.
pub(crate) fn report_error_to(
    json: bool,
    command: Option<&str>,
    failure: &CliFailure,
    out: &mut impl Write,
) -> (ExitCode, Option<String>) {
    let code = ExitCode::from(failure.error.exit_code());
    if !json {
        let _ = writeln!(out, "error: {}", failure.error.message());
        let _ = out.flush();
        return (code, None);
    }
    let payload = format!(
        "{}\n",
        envelope_err(command, &failure.error, failure.context.as_ref())
    );
    match deliver(out, &payload) {
        Delivery::Delivered | Delivery::ClosedPipe => (code, None),
        Delivery::Failed(e) => (
            code,
            Some(format!(
                "OUTPUT_DELIVERY_ERROR: could not deliver the {} error report to stdout: {e}",
                command.unwrap_or("invoked command")
            )),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::error::CliError;
    use std::collections::VecDeque;

    /// Writer driven by a script: each `write()` pops one scripted result (a
    /// short count loops `write_all` back for the rest); when the script runs
    /// dry writes succeed. `flush_err` fails the first flush.
    struct Scripted {
        script: VecDeque<io::Result<usize>>,
        flush_err: Option<io::Error>,
        written: usize,
    }

    impl Scripted {
        fn ok() -> Self {
            Self {
                script: VecDeque::new(),
                flush_err: None,
                written: 0,
            }
        }
        fn writes(script: Vec<io::Result<usize>>) -> Self {
            Self {
                script: script.into(),
                flush_err: None,
                written: 0,
            }
        }
        fn flush_err(e: io::Error) -> Self {
            Self {
                script: VecDeque::new(),
                flush_err: Some(e),
                written: 0,
            }
        }
    }

    impl Write for Scripted {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            match self.script.pop_front() {
                Some(Ok(n)) => {
                    let n = n.min(buf.len());
                    self.written += n;
                    Ok(n)
                }
                Some(Err(e)) => Err(e),
                None => {
                    self.written += buf.len();
                    Ok(buf.len())
                }
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            match self.flush_err.take() {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
    }

    fn dead() -> io::Error {
        io::Error::new(io::ErrorKind::PermissionDenied, "sink unavailable")
    }

    fn output(data: serde_json::Value) -> Output {
        Output {
            text: String::new(),
            data,
        }
    }

    #[test]
    fn clean_write_and_flush_delivers() {
        let mut w = Scripted::ok();
        assert!(matches!(deliver(&mut w, "payload\n"), Delivery::Delivered));
        assert_eq!(w.written, 8);
    }

    #[test]
    fn broken_pipe_on_write_is_closed_pipe() {
        let mut w = Scripted::writes(vec![Err(io::Error::from(io::ErrorKind::BrokenPipe))]);
        assert!(matches!(deliver(&mut w, "x"), Delivery::ClosedPipe));
    }

    #[test]
    fn broken_pipe_on_flush_is_closed_pipe() {
        let mut w = Scripted::flush_err(io::Error::from(io::ErrorKind::BrokenPipe));
        assert!(matches!(deliver(&mut w, "x"), Delivery::ClosedPipe));
    }

    #[test]
    fn denied_write_fails() {
        let mut w = Scripted::writes(vec![Err(dead())]);
        match deliver(&mut w, "x") {
            Delivery::Failed(e) => assert_eq!(e.kind(), io::ErrorKind::PermissionDenied),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn short_write_then_error_fails() {
        let mut w = Scripted::writes(vec![Ok(1), Err(dead())]);
        match deliver(&mut w, "the rest is lost") {
            Delivery::Failed(e) => assert_eq!(e.kind(), io::ErrorKind::PermissionDenied),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn flush_error_after_good_writes_fails() {
        let mut w = Scripted::flush_err(dead());
        match deliver(&mut w, "buffered in the consumer") {
            Delivery::Failed(e) => assert_eq!(e.kind(), io::ErrorKind::PermissionDenied),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn diag_names_delivery_error_and_completed_operation() {
        let d = delivery_failure_diag("import", Some("0f-1"), &dead());
        assert!(d.contains("OUTPUT_DELIVERY_ERROR"), "{d}");
        assert!(d.contains("import"), "{d}");
        assert!(d.contains("completed"), "{d}");
        assert!(d.contains("0f-1"), "{d}");
        assert!(!d.to_lowercase().contains("failed"), "{d}");
        assert!(!d.to_lowercase().contains("retry"), "{d}");
    }

    #[test]
    fn diag_omits_absent_operation_id() {
        let d = delivery_failure_diag("help", None, &dead());
        assert!(d.contains("OUTPUT_DELIVERY_ERROR"), "{d}");
        assert!(d.contains("completed"), "{d}");
        assert!(!d.contains("operation_id"), "{d}");
    }

    #[test]
    fn finish_core_delivers_success_quietly() {
        let mut w = Scripted::ok();
        let (code, diag) = finish_ok_to(
            true,
            "status",
            output(serde_json::json!({"operation_id": "op-1"})),
            &mut w,
        );
        assert_eq!(code, ExitCode::SUCCESS);
        assert_eq!(diag, None);
    }

    #[test]
    fn finish_core_broken_pipe_stays_success() {
        let mut w = Scripted::writes(vec![Err(io::Error::from(io::ErrorKind::BrokenPipe))]);
        let (code, diag) = finish_ok_to(false, "status", output(serde_json::json!(null)), &mut w);
        assert_eq!(code, ExitCode::SUCCESS);
        assert_eq!(diag, None);
    }

    #[test]
    fn finish_core_failed_delivery_exits_one_with_diag() {
        let mut w = Scripted::writes(vec![Err(dead())]);
        let (code, diag) = finish_ok_to(
            true,
            "import",
            output(serde_json::json!({"operation_id": "op-9"})),
            &mut w,
        );
        assert_eq!(code, ExitCode::from(1));
        let diag = diag.expect("diagnostic accompanies the undelivered result");
        assert!(diag.contains("OUTPUT_DELIVERY_ERROR"), "{diag}");
        assert!(diag.contains("completed"), "{diag}");
        assert!(diag.contains("op-9"), "{diag}");
    }

    #[test]
    fn error_exit_code_survives_report_write_failure() {
        let failure = CliFailure::from(CliError::Workflow("boom".into()));
        let mut w = Scripted::writes(vec![Err(dead())]);
        let (code, diag) = report_error_to(true, Some("import"), &failure, &mut w);
        assert_eq!(code, ExitCode::from(1), "original exit code kept");
        let diag = diag.expect("non-pipe envelope failure yields a diagnostic");
        assert!(diag.contains("OUTPUT_DELIVERY_ERROR"), "{diag}");

        let mut pipe = Scripted::writes(vec![Err(io::Error::from(io::ErrorKind::BrokenPipe))]);
        let (code, diag) = report_error_to(true, Some("import"), &failure, &mut pipe);
        assert_eq!(code, ExitCode::from(1));
        assert_eq!(diag, None, "broken pipe on the error envelope stays quiet");

        let usage = CliFailure::from(CliError::Usage("bad".into()));
        let mut w = Scripted::writes(vec![Err(dead())]);
        let (code, diag) = report_error_to(true, Some("status"), &usage, &mut w);
        assert_eq!(code, ExitCode::from(2), "usage exit code kept");
        assert!(diag.is_some());
    }

    #[test]
    fn text_error_report_stays_best_effort() {
        let failure = CliFailure::from(CliError::Workflow("boom".into()));
        let mut w = Scripted::writes(vec![Err(dead())]);
        let (code, diag) = report_error_to(false, Some("status"), &failure, &mut w);
        assert_eq!(code, ExitCode::from(1));
        assert_eq!(diag, None);
    }
}
