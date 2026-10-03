//! Legacy history only. All new AI submissions go through the simple AppWorker.
use super::{
    error::{classify_workflow, CliError},
    open_workspace, CliFailure, Command, Invocation, Output,
};
use brn_workflow::{worker::OperationStatus, ChatTurn};
use serde_json::{json, Value};

fn status_str(status: OperationStatus) -> &'static str {
    match status {
        OperationStatus::Pending => "pending",
        OperationStatus::Running => "running",
        OperationStatus::Completed => "completed",
        OperationStatus::Failed => "failed",
        OperationStatus::Interrupted => "interrupted",
    }
}

fn parse_or_raw(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.into()))
}

fn turn_json(turn: &ChatTurn) -> Value {
    json!({
        "operation_id": turn.operation_id, "session_id": turn.session_id,
        "question": turn.question, "profile": turn.profile, "answer": turn.answer,
        "status": status_str(turn.status), "provider_turn_id": turn.provider_turn_id,
        "evidence": parse_or_raw(&turn.evidence_json),
        "usage": turn.usage_json.as_deref().map(parse_or_raw),
        "evidence_currentness": turn.evidence_currentness,
    })
}

pub fn run(invocation: &Invocation) -> Result<Output, CliFailure> {
    let workspace = open_workspace(invocation)?;
    let sessions = workspace.sessions().map_err(classify_workflow)?;
    match invocation.command {
        Command::ConversationsList => Ok(Output {
            text: sessions
                .iter()
                .map(|s| format!("{} has_thread={} turns={}\n", s.id, s.has_thread, s.turns))
                .collect(),
            data: json!({ "sessions": sessions.iter().map(|s| json!({"id": s.id, "has_thread": s.has_thread, "turns": s.turns})).collect::<Vec<_>>() }),
        }),
        Command::ConversationsShow { session } => {
            if !sessions.iter().any(|s| s.id == session) {
                return Err(CliError::NotFound(format!("session {session} not found")).into());
            }
            let turns = workspace.history(session).map_err(classify_workflow)?;
            Ok(Output {
                text: turns.iter().map(|t| format!(
                    "historical operation {} session {} status {} profile {} evidence_currentness {:?}\nquestion: {}\nanswer: {}\n\n",
                    t.operation_id, t.session_id, status_str(t.status), t.profile, t.evidence_currentness,
                    t.question, t.answer.as_deref().unwrap_or("(none)")
                )).collect(),
                data: json!({"session_id": session, "historical": true, "turns": turns.iter().map(turn_json).collect::<Vec<_>>() }),
            })
        }
        _ => unreachable!("legacy history only"),
    }
}
