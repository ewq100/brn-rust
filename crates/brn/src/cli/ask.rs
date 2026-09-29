//! `ask`, `conversations list/show`: grounded chat with the Codex provider,
//! deadline watching and SIGINT classification.
use crate::cli::{
    error::{classify_workflow, CliError},
    open_workspace, Command, Invocation, Output,
};
use brn_workflow::{worker::OperationStatus, ChatTurn, Workspace};
use serde_json::{json, Value};
use std::{
    io::Write as _,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};
use uuid::Uuid;

pub fn run(invocation: &Invocation) -> Result<Output, CliError> {
    match &invocation.command {
        Command::ConversationsList => conversations_list(invocation),
        Command::ConversationsShow { session } => conversations_show(invocation, *session),
        Command::Ask { .. } => ask(invocation),
        _ => unreachable!("ask module handles ask and conversations commands only"),
    }
}

/// Lowercase stable status string for history surfaces.
fn status_str(status: OperationStatus) -> &'static str {
    match status {
        OperationStatus::Pending => "pending",
        OperationStatus::Running => "running",
        OperationStatus::Completed => "completed",
        OperationStatus::Failed => "failed",
        OperationStatus::Interrupted => "interrupted",
    }
}

/// `evidence_json`/`usage_json` are JSON strings in the DTO; surface them as
/// real JSON values, falling back to the raw string if they do not parse.
fn parse_or_raw(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

/// Stable reshape of the workflow ChatTurn DTO.
fn turn_json(turn: &ChatTurn) -> Value {
    let usage = match turn.usage_json.as_deref() {
        Some(raw) => parse_or_raw(raw),
        None => Value::Null,
    };
    json!({
        "operation_id": turn.operation_id,
        "session_id": turn.session_id,
        "question": turn.question,
        "profile": turn.profile,
        "answer": turn.answer,
        "status": status_str(turn.status),
        "provider_turn_id": turn.provider_turn_id,
        "evidence": parse_or_raw(&turn.evidence_json),
        "usage": usage,
    })
}

fn require_session(workspace: &Workspace, session: Uuid) -> Result<(), CliError> {
    let sessions = workspace.sessions().map_err(classify_workflow)?;
    if sessions.iter().any(|s| s.id == session) {
        Ok(())
    } else {
        Err(CliError::NotFound(format!("session {session} not found")))
    }
}

fn conversations_list(invocation: &Invocation) -> Result<Output, CliError> {
    let workspace = open_workspace(invocation)?;
    let sessions = workspace.sessions().map_err(classify_workflow)?;
    let mut text = String::new();
    let mut list = Vec::with_capacity(sessions.len());
    for session in &sessions {
        text.push_str(&format!(
            "{} has_thread={} turns={}\n",
            session.id, session.has_thread, session.turns
        ));
        list.push(json!({
            "id": session.id,
            "has_thread": session.has_thread,
            "turns": session.turns,
        }));
    }
    Ok(Output {
        text,
        data: json!({ "sessions": list }),
    })
}

fn conversations_show(invocation: &Invocation, session: Uuid) -> Result<Output, CliError> {
    let workspace = open_workspace(invocation)?;
    require_session(&workspace, session)?;
    let turns = workspace.history(session).map_err(classify_workflow)?;
    let mut text = String::new();
    for turn in &turns {
        text.push_str(&format!(
            "operation {} session {} status {} profile {}\nquestion: {}\nanswer: {}\n\n",
            turn.operation_id,
            turn.session_id,
            status_str(turn.status),
            turn.profile,
            turn.question,
            turn.answer.as_deref().unwrap_or("(none)"),
        ));
    }
    Ok(Output {
        text,
        data: json!({
            "session_id": session,
            "turns": turns.iter().map(turn_json).collect::<Vec<_>>(),
        }),
    })
}

/// Deadline/SIGINT errors carry the identifiers actually known at that point.
fn timeout_error(op: Uuid, session: Option<Uuid>, seconds: u64) -> CliError {
    let session = session.map(|s| format!(" session {s}")).unwrap_or_default();
    CliError::Timeout(format!(
        "ask timed out after {seconds}s; operation {op}{session}"
    ))
}

fn interrupted_error(op: Uuid, session: Option<Uuid>) -> CliError {
    let session = session.map(|s| format!(" session {s}")).unwrap_or_default();
    CliError::Interrupted(format!("ask interrupted; operation {op}{session}"))
}

fn ask(invocation: &Invocation) -> Result<Output, CliError> {
    let Command::Ask {
        question,
        profile,
        session,
        operation,
        timeout_seconds,
    } = &invocation.command
    else {
        unreachable!("ask command only");
    };
    // Refuse before the workspace opens: a fresh operation UUID must never be
    // durably recorded for an invocation that cannot reach a provider. A
    // missing required option is a usage error, not an operational failure.
    if invocation.codex.is_none() {
        return Err(CliError::Usage(
            "ask requires --codex ABSOLUTE_EXECUTABLE".into(),
        ));
    }
    let mut workspace = open_workspace(invocation)?;
    if let Some(session) = session {
        require_session(&workspace, *session)?;
    }
    let op = operation.unwrap_or_else(Uuid::new_v4);
    let deadline = Duration::from_secs(*timeout_seconds);
    let timed_out = Arc::new(AtomicBool::new(false));
    // Watcher: sleeps in 50 ms slices until the deadline or ask completion.
    // On the deadline it sets both the local flag and the global CANCEL flag
    // (which routes the provider interrupt); it is joined before reporting.
    let (done, done_rx) = mpsc::channel::<()>();
    let watcher = {
        let timed_out = Arc::clone(&timed_out);
        std::thread::spawn(move || {
            let start = Instant::now();
            while start.elapsed() < deadline {
                if done_rx.recv_timeout(Duration::from_millis(50)).is_ok() {
                    return;
                }
            }
            timed_out.store(true, Ordering::SeqCst);
            crate::CANCEL.store(true, Ordering::SeqCst);
        })
    };
    // Deltas stream to stderr so stdout stays exactly one envelope object;
    // the write is best-effort (a closed stderr is not a lifecycle event).
    let result = workspace.ask(op, *session, question, *profile, &crate::CANCEL, |delta| {
        let _ = write!(std::io::stderr(), "{delta}");
    });
    let _ = done.send(());
    watcher.join().expect("deadline watcher thread");
    // Classification order: a durably completed turn stays a success even if
    // the deadline flag fired in the slice window after completion — the
    // outcome is accurate and a same-operation rerun would return it anyway.
    match result {
        Err(error) => {
            if timed_out.load(Ordering::SeqCst) {
                Err(timeout_error(op, *session, *timeout_seconds))
            } else if crate::CANCEL.load(Ordering::SeqCst) {
                Err(interrupted_error(op, *session))
            } else {
                Err(classify_workflow(error))
            }
        }
        Ok(turn) => {
            let session = turn.session_id;
            if turn.status == OperationStatus::Completed {
                return Ok(Output {
                    text: format!("{}\n", turn.answer.as_deref().unwrap_or_default()),
                    data: turn_json(&turn),
                });
            }
            if timed_out.load(Ordering::SeqCst) {
                return Err(timeout_error(op, Some(session), *timeout_seconds));
            }
            if crate::CANCEL.load(Ordering::SeqCst) {
                return Err(interrupted_error(op, Some(session)));
            }
            Err(CliError::Workflow(format!(
                "turn ended as {}; operation {op} session {session}",
                status_str(turn.status)
            )))
        }
    }
}
