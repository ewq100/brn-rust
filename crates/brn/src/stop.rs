//! Why a command stopped early: SIGINT, SIGTERM (machine mode) or the
//! `--timeout` deadline. Every reason also raises `CANCEL`, so each existing
//! observation point serves all three. A watchdog raises the deadline and,
//! when no envelope appears 3 s after SIGTERM or the deadline, writes one
//! line itself and exits: the command may have committed by then.
use crate::contract::{self, CliError, Code, Status};
use crate::spec::Class;
use crate::CANCEL;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering::SeqCst};
use std::time::{Duration, Instant};

/// 0 none (SIGINT is inferred from `CANCEL`), 2 SIGTERM, 3 deadline.
static STOP: AtomicU8 = AtomicU8::new(0);
/// Claimed by whoever writes the single result: the command or the watchdog.
static EMITTED: AtomicBool = AtomicBool::new(false);
const SIGTERM: u8 = 2;
const DEADLINE: u8 = 3;
/// How long the watchdog waits for a safe point after SIGTERM or the deadline.
const GRACE: Duration = Duration::from_secs(3);

/// The first reason wins; a SIGINT already seen keeps its reason.
fn raise(reason: u8) {
    if !CANCEL.load(SeqCst) {
        let _ = STOP.compare_exchange(0, reason, SeqCst, SeqCst);
    }
    CANCEL.store(true, SeqCst);
}
extern "C" fn on_sigterm(_: libc::c_int) {
    raise(SIGTERM);
}

/// `sigint`, `sigterm`, `deadline`, or `None` while nothing asked to stop.
pub(crate) fn reason() -> Option<&'static str> {
    match STOP.load(SeqCst) {
        SIGTERM => Some("sigterm"),
        DEADLINE => Some("deadline"),
        _ if CANCEL.load(SeqCst) => Some("sigint"),
        _ => None,
    }
}

/// The observation points added for SIGTERM and the deadline (SIGINT keeps its points).
pub(crate) fn check() -> Result<(), CliError> {
    if matches!(STOP.load(SeqCst), SIGTERM | DEADLINE) {
        return Err(CliError::interrupted());
    }
    Ok(())
}

/// Machine mode only: the first SIGTERM is observed; the handler resets, so a
/// second SIGTERM kills the process.
pub(crate) fn install_sigterm() {
    // SAFETY: the handler only touches lock-free atomics; the struct is zeroed
    // and then filled before the call.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = on_sigterm as *const () as libc::sighandler_t;
        action.sa_flags = libc::SA_RESETHAND | libc::SA_RESTART;
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(libc::SIGTERM, &action, std::ptr::null_mut());
    }
}

/// True for the one writer of the result; the command calls this before printing.
pub(crate) fn claim_output() -> bool {
    EMITTED
        .compare_exchange(false, true, SeqCst, SeqCst)
        .is_ok()
}

/// What the watchdog needs to write the hard-stop line.
pub(crate) struct Watch {
    pub(crate) machine: bool,
    pub(crate) deadline: Option<u64>,
    pub(crate) command: String,
    pub(crate) key: Option<String>,
    pub(crate) retry: Class,
}

/// Default-mode text of a soft deadline stop.
pub(crate) fn soft_text(seconds: u64) -> String {
    format!("Stopped at the --timeout deadline ({seconds} s); nothing was committed")
}

/// Restamps an `interrupted` error by the stop reason: SIGTERM is
/// `terminated`, the deadline is `timeout` with `{deadline_seconds, hard}`.
pub(crate) fn restamp(error: CliError, deadline: Option<u64>) -> CliError {
    if error.code != Code::Interrupted {
        return error;
    }
    match reason() {
        Some("sigterm") => CliError::new(Code::Terminated, "Stopped by SIGTERM before committing"),
        Some("deadline") => {
            let seconds = deadline.unwrap_or_default();
            let mut stopped = CliError::new(
                Code::Timeout,
                "The --timeout deadline passed before the command committed",
            )
            .with("deadline_seconds", seconds)
            .with("hard", false);
            stopped.default_text = Some(soft_text(seconds));
            stopped
        }
        _ => error,
    }
}

fn hard_stop(watch: &Watch) {
    if !claim_output() {
        return;
    }
    let deadline = reason() == Some("deadline");
    if !watch.machine {
        let seconds = watch.deadline.unwrap_or_default();
        contract::emit_err(&format!(
            "Stopped at the --timeout deadline ({seconds} s); state unknown\n"
        ));
        // SAFETY: ends the process without unwinding; nothing else writes now.
        unsafe { libc::_exit(1) };
    }
    let message = "Stopped; the command may have committed. Read state as the retry class describes before retrying.";
    let (status, error) = if deadline {
        let error = CliError::new(Code::Timeout, message)
            .with("deadline_seconds", watch.deadline.unwrap_or_default())
            .with("hard", true);
        (Status::Failed, error)
    } else {
        (
            Status::Terminated,
            CliError::new(Code::Terminated, message).with("hard", true),
        )
    };
    let envelope = contract::Envelope {
        command: Some(&watch.command),
        status,
        outcome: None,
        operation: None,
        key: watch.key.as_deref(),
        retry: watch.retry,
        changed: vec![],
        conflict: None,
        result: Value::Null,
        error: Some(error),
        page: None,
        stop: reason(),
    };
    contract::emit_line(&envelope.to_value().to_string());
    // SAFETY: ends the process without unwinding after the one line is written.
    unsafe { libc::_exit(i32::from(status.exit())) };
}

/// Starts the watchdog: it raises the deadline and hard-stops after the grace.
pub(crate) fn start(watch: Watch) {
    std::thread::spawn(move || {
        let started = Instant::now();
        let mut raised = false;
        let mut stopped_at: Option<Instant> = None;
        loop {
            std::thread::sleep(Duration::from_millis(25));
            if let Some(seconds) = watch.deadline {
                if !raised && started.elapsed() >= Duration::from_secs(seconds) {
                    raised = true;
                    raise(DEADLINE);
                }
            }
            if matches!(STOP.load(SeqCst), SIGTERM | DEADLINE) {
                let at = *stopped_at.get_or_insert_with(Instant::now);
                if at.elapsed() >= GRACE {
                    hard_stop(&watch);
                    return;
                }
            }
        }
    });
}
