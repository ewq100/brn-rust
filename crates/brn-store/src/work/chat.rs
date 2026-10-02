use super::{WorkStore, now_ms};
use crate::{Error, Result, invalid, parse_id};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkTurnStatus {
    Running,
    Completed,
    Interrupted,
    Failed,
}

impl WorkTurnStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "interrupted" => Ok(Self::Interrupted),
            "failed" => Ok(Self::Failed),
            _ => Err(invalid("invalid stored turn status")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkConversation {
    pub id: Uuid,
    pub title: String,
    pub turns: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkTurn {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub question: String,
    pub answer: String,
    pub provider: String,
    pub model: String,
    pub status: WorkTurnStatus,
    pub error_code: Option<String>,
}

fn validate_selection(provider: &str, model: &str) -> Result<()> {
    if !matches!(provider, "chatgpt" | "copilot") {
        return Err(invalid("invalid chat provider"));
    }
    if model.is_empty()
        || model.len() > 128
        || !model
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
    {
        return Err(invalid("invalid chat model identifier"));
    }
    Ok(())
}

fn validate_error(code: Option<&str>) -> Result<()> {
    if let Some(code) = code
        && !matches!(
            code,
            "reconnect_needed"
                | "code_expired"
                | "rate_limited"
                | "network"
                | "model_refused"
                | "invalid_tool_use"
                | "tool_limit_reached"
                | "unsafe_credentials"
                | "tool_rejected"
                | "index_stale"
                | "storage"
                | "other"
        )
    {
        return Err(invalid("invalid chat error category"));
    }
    Ok(())
}

struct Message {
    conversation: String,
    sequence: i64,
    role: String,
    text: String,
    provider: String,
    model: String,
    status: String,
    error: Option<String>,
}

fn read_turn(conn: &Connection, id: Uuid) -> Result<Option<(WorkTurn, i64)>> {
    let mut statement = conn.prepare(
        "SELECT conversation_id, sequence, role, text, provider, model, status, error_code
         FROM messages WHERE turn_id = ?1 ORDER BY role DESC",
    )?;
    let messages = statement
        .query_map([id.to_string()], |r| {
            Ok(Message {
                conversation: r.get(0)?,
                sequence: r.get(1)?,
                role: r.get(2)?,
                text: r.get(3)?,
                provider: r.get(4)?,
                model: r.get(5)?,
                status: r.get(6)?,
                error: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if messages.is_empty() {
        return Ok(None);
    }
    let [user, assistant] = messages.as_slice() else {
        return Err(invalid("stored turn must have one user/assistant pair"));
    };
    if user.role != "user"
        || assistant.role != "assistant"
        || user.conversation != assistant.conversation
        || user.sequence != assistant.sequence
        || user.sequence < 1
        || user.provider != assistant.provider
        || user.model != assistant.model
        || user.status != assistant.status
        || user.error != assistant.error
        || user.text.trim().is_empty()
    {
        return Err(invalid("invalid stored turn pair"));
    }
    validate_selection(&user.provider, &user.model)?;
    validate_selection(&assistant.provider, &assistant.model)?;
    validate_error(user.error.as_deref())?;
    let conversation_id = parse_id(user.conversation.clone())?;
    require_conversation(conn, conversation_id)?;
    Ok(Some((
        WorkTurn {
            id,
            conversation_id,
            question: user.text.clone(),
            answer: assistant.text.clone(),
            provider: user.provider.clone(),
            model: user.model.clone(),
            status: WorkTurnStatus::parse(&user.status)?,
            error_code: user.error.clone(),
        },
        user.sequence,
    )))
}

fn require_conversation(conn: &Connection, id: Uuid) -> Result<()> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM conversations WHERE id = ?1",
            [id.to_string()],
            |_| Ok(()),
        )
        .optional()?;
    exists.ok_or_else(|| Error::NotFound("conversation not found".into()))
}

fn conflict() -> Error {
    Error::OperationConflict("chat turn UUID reused with a different payload".into())
}

// Connection-based helpers are shared by the owner and the future attached chat lane.
pub(super) fn begin_turn(
    conn: &mut Connection,
    id: Uuid,
    conversation: Option<Uuid>,
    question: &str,
    provider: &str,
    model: &str,
) -> Result<WorkTurn> {
    let tx = conn.transaction()?;
    if let Some((turn, _)) = read_turn(&tx, id)? {
        if turn.question != question
            || turn.provider != provider
            || turn.model != model
            || conversation.is_some_and(|c| c != turn.conversation_id)
        {
            return Err(conflict());
        }
        tx.commit()?;
        return Ok(turn);
    }
    if question.trim().is_empty() {
        return Err(invalid("chat question must not be blank"));
    }
    validate_selection(provider, model)?;
    let conversation_id = if let Some(c) = conversation {
        require_conversation(&tx, c)?;
        c
    } else {
        let c = Uuid::new_v4();
        tx.execute(
            "INSERT INTO conversations(id, title, created_at_ms) VALUES (?1, '', ?2)",
            params![c.to_string(), now_ms() as i64],
        )?;
        c
    };
    let sequence: i64 = tx.query_row(
        "SELECT COALESCE(MAX(sequence), 0) + 1 FROM messages WHERE conversation_id = ?1",
        [conversation_id.to_string()],
        |r| r.get(0),
    )?;
    tx.execute(
        "INSERT INTO messages(turn_id, conversation_id, sequence, role, text, provider, model, status)
         VALUES (?1, ?2, ?3, 'user', ?4, ?5, ?6, 'running'),
                (?1, ?2, ?3, 'assistant', '', ?5, ?6, 'running')",
        params![id.to_string(), conversation_id.to_string(), sequence, question, provider, model],
    )?;
    let turn = read_turn(&tx, id)?
        .ok_or_else(|| invalid("inserted chat turn is missing"))?
        .0;
    tx.commit()?;
    Ok(turn)
}

pub(super) fn finish_turn(
    conn: &mut Connection,
    id: Uuid,
    status: WorkTurnStatus,
    answer: &str,
    error_code: Option<&str>,
) -> Result<WorkTurn> {
    let tx = conn.transaction()?;
    let turn = finish_in_transaction(&tx, id, status, answer, error_code)?;
    tx.commit()?;
    Ok(turn)
}

fn finish_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    id: Uuid,
    status: WorkTurnStatus,
    answer: &str,
    error_code: Option<&str>,
) -> Result<WorkTurn> {
    validate_error(error_code)?;
    if status == WorkTurnStatus::Running {
        return Err(invalid("finish_turn requires a terminal status"));
    }
    let (mut turn, sequence) =
        read_turn(tx, id)?.ok_or_else(|| Error::NotFound("chat turn not found".into()))?;
    if turn.status != WorkTurnStatus::Running {
        if turn.status != status
            || turn.answer != answer
            || turn.error_code.as_deref() != error_code
        {
            return Err(conflict());
        }
        return Ok(turn);
    }
    tx.execute(
        "UPDATE messages SET status = ?2, error_code = ?3,
         text = CASE WHEN role = 'assistant' THEN ?4 ELSE text END WHERE turn_id = ?1",
        params![id.to_string(), status.as_str(), error_code, answer],
    )?;
    if sequence == 1 {
        tx.execute(
            "UPDATE conversations SET title = ?2 WHERE id = ?1",
            params![turn.conversation_id.to_string(), turn.question],
        )?;
    }
    turn.status = status;
    turn.answer = answer.into();
    turn.error_code = error_code.map(str::to_owned);
    Ok(turn)
}

fn turns(conn: &Connection, conversation: Uuid) -> Result<Vec<WorkTurn>> {
    require_conversation(conn, conversation)?;
    let mut statement = conn.prepare(
        "SELECT DISTINCT turn_id, sequence FROM messages WHERE conversation_id = ?1 ORDER BY sequence",
    )?;
    let ids = statement
        .query_map([conversation.to_string()], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.into_iter()
        .map(|id| {
            Ok(read_turn(conn, parse_id(id)?)?
                .ok_or_else(|| invalid("stored chat turn is missing"))?
                .0)
        })
        .collect()
}

pub(super) fn reconcile(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    let ids = tx
        .prepare("SELECT DISTINCT turn_id FROM messages")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for id in ids {
        let (turn, _) =
            read_turn(&tx, parse_id(id)?)?.ok_or_else(|| invalid("stored chat turn is missing"))?;
        if turn.status == WorkTurnStatus::Running {
            finish_in_transaction(
                &tx,
                turn.id,
                WorkTurnStatus::Interrupted,
                &turn.answer,
                turn.error_code.as_deref(),
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

impl WorkStore {
    pub fn conversations(&self) -> Result<Vec<WorkConversation>> {
        let mut statement = self
            .conn
            .prepare("SELECT id, title FROM conversations ORDER BY created_at_ms, id")?;
        let rows = statement
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter()
            .map(|(id, title)| {
                let id = parse_id(id)?;
                Ok(WorkConversation {
                    id,
                    title,
                    turns: turns(&self.conn, id)?.len(),
                })
            })
            .collect()
    }

    pub fn turns(&self, conversation: Uuid) -> Result<Vec<WorkTurn>> {
        turns(&self.conn, conversation)
    }

    pub fn begin_turn(
        &mut self,
        id: Uuid,
        conversation: Option<Uuid>,
        question: &str,
        provider: &str,
        model: &str,
    ) -> Result<WorkTurn> {
        begin_turn(&mut self.conn, id, conversation, question, provider, model)
    }

    pub fn finish_turn(
        &mut self,
        id: Uuid,
        status: WorkTurnStatus,
        answer: &str,
        error_code: Option<&str>,
    ) -> Result<WorkTurn> {
        finish_turn(&mut self.conn, id, status, answer, error_code)
    }
}
