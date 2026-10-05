use super::WorkStore;
use crate::{Error, Result, invalid, parse_id};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

pub(super) const V7: &str = "
ALTER TABLE messages ADD COLUMN effort TEXT
CHECK(effort IS NULL OR effort IN ('low','medium','high'));";

pub(super) const V8: &str = "
ALTER TABLE conversations ADD COLUMN last_activity_at_ms INTEGER;
ALTER TABLE messages ADD COLUMN started_at_ms INTEGER;
ALTER TABLE messages ADD COLUMN finished_at_ms INTEGER;";

/// An attachment authorized by a checked/migrated owner, retaining its exact lock.
pub struct ChatStore {
    pub(super) conn: Connection,
    _owner_lock: Arc<File>,
}

impl ChatStore {
    pub fn turn(&self, id: Uuid) -> Result<Option<WorkTurn>> {
        Ok(read_turn(&self.conn, id)?.map(|(turn, _)| turn))
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
        self.begin_turn_with_effort(id, conversation, question, provider, model, None)
    }

    pub fn begin_turn_with_effort(
        &mut self,
        id: Uuid,
        conversation: Option<Uuid>,
        question: &str,
        provider: &str,
        model: &str,
        effort: Option<&str>,
    ) -> Result<WorkTurn> {
        begin_turn(
            &mut self.conn,
            id,
            conversation,
            question,
            provider,
            model,
            effort,
        )
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
    pub created_at_ms: u64,
    #[serde(default)]
    pub last_activity_at_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkTurn {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub question: String,
    pub answer: String,
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    pub status: WorkTurnStatus,
    pub error_code: Option<String>,
    #[serde(default)]
    pub started_at_ms: Option<u64>,
    #[serde(default)]
    pub finished_at_ms: Option<u64>,
}

pub(super) fn validate_selection(provider: &str, model: &str) -> Result<()> {
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

pub(super) fn validate_error(code: Option<&str>) -> Result<()> {
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

fn validate_effort(effort: Option<&str>) -> Result<()> {
    if effort.is_some_and(|effort| !matches!(effort, "low" | "medium" | "high")) {
        return Err(invalid("invalid chat reasoning effort"));
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
    effort: Option<String>,
    status: String,
    error: Option<String>,
    started: Option<i64>,
    finished: Option<i64>,
}

pub(super) fn read_turn(conn: &Connection, id: Uuid) -> Result<Option<(WorkTurn, i64)>> {
    let mut statement = conn.prepare(
        "SELECT conversation_id, sequence, role, text, provider, model, effort, status, error_code, started_at_ms, finished_at_ms
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
                effort: r.get(6)?,
                status: r.get(7)?,
                error: r.get(8)?,
                started: r.get(9)?,
                finished: r.get(10)?,
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
        || user.effort != assistant.effort
        || user.status != assistant.status
        || user.error != assistant.error
        || user.started != assistant.started
        || user.finished != assistant.finished
        || user.text.trim().is_empty()
    {
        return Err(invalid("invalid stored turn pair"));
    }
    validate_selection(&user.provider, &user.model)?;
    validate_selection(&assistant.provider, &assistant.model)?;
    validate_effort(user.effort.as_deref())?;
    validate_error(user.error.as_deref())?;
    let conversation_id = parse_id(user.conversation.clone())?;
    let conversation = require_conversation(conn, conversation_id)?;
    let started_at_ms = optional_timestamp(user.started)?;
    let finished_at_ms = optional_timestamp(user.finished)?;
    let status = WorkTurnStatus::parse(&user.status)?;
    if status == WorkTurnStatus::Running && finished_at_ms.is_some()
        || started_at_ms
            .zip(finished_at_ms)
            .is_some_and(|(start, finish)| finish < start)
        || [started_at_ms, finished_at_ms]
            .into_iter()
            .flatten()
            .any(|time| {
                time < conversation.created
                    || conversation.last_activity.is_some_and(|last| time > last)
            })
    {
        return Err(invalid("invalid stored chat timestamp ordering"));
    }
    let turn = WorkTurn {
        id,
        conversation_id,
        question: user.text.clone(),
        answer: assistant.text.clone(),
        provider: user.provider.clone(),
        model: user.model.clone(),
        effort: user.effort.clone(),
        status,
        error_code: user.error.clone(),
        started_at_ms,
        finished_at_ms,
    };
    super::inbox_actions::check_turn(conn, &turn)?;
    Ok(Some((turn, user.sequence)))
}

struct ConversationTimes {
    created: u64,
    last_activity: Option<u64>,
}

fn timestamp(value: i64) -> Result<u64> {
    u64::try_from(value).map_err(|_| invalid("stored chat timestamps must not be negative"))
}
fn optional_timestamp(value: Option<i64>) -> Result<Option<u64>> {
    value.map(timestamp).transpose()
}
fn sql_timestamp(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| invalid("chat timestamp exceeds the supported integer range"))
}
fn require_conversation(conn: &Connection, id: Uuid) -> Result<ConversationTimes> {
    let exists = conn
        .query_row(
            "SELECT created_at_ms,last_activity_at_ms FROM conversations WHERE id = ?1",
            [id.to_string()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .optional()?;
    let (created, last) = exists.ok_or_else(|| Error::NotFound("conversation not found".into()))?;
    let created = timestamp(created)?;
    let last_activity = optional_timestamp(last)?;
    if last_activity.is_some_and(|last| last < created) {
        return Err(invalid("stored session activity predates its creation"));
    }
    Ok(ConversationTimes {
        created,
        last_activity,
    })
}

fn clock_floor(conn: &Connection, id: Uuid, conversation: &ConversationTimes) -> Result<u64> {
    let mut floor = conversation.created;
    if let Some(last) = conversation.last_activity {
        return Ok(floor.max(last));
    }
    // Unknown activity remains unknown until a genuine new event. Known prior
    // turn times only constrain that event's clock; they are never backfilled.
    let mut statement =
        conn.prepare("SELECT started_at_ms,finished_at_ms FROM messages WHERE conversation_id=?1")?;
    let pairs = statement.query_map([id.to_string()], |row| {
        Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?))
    })?;
    for pair in pairs {
        let (start, finish) = pair?;
        for time in [optional_timestamp(start)?, optional_timestamp(finish)?]
            .into_iter()
            .flatten()
        {
            floor = floor.max(time);
        }
    }
    Ok(floor)
}
fn capture_time(floor: u64) -> Result<u64> {
    let wall = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let now = u64::try_from(wall)
        .map_err(|_| invalid("chat clock exceeds the supported integer range"))?;
    sql_timestamp(now)?;
    let captured = now.max(floor);
    sql_timestamp(captured)?;
    Ok(captured)
}

fn conflict() -> Error {
    Error::OperationConflict("chat turn UUID reused with a different payload".into())
}

// Reserve the writer before reading: a deferred WAL upgrade can bypass busy_timeout.
pub(super) fn begin_turn(
    conn: &mut Connection,
    id: Uuid,
    conversation: Option<Uuid>,
    question: &str,
    provider: &str,
    model: &str,
    effort: Option<&str>,
) -> Result<WorkTurn> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if super::inbox_actions::reserved(&tx, id)?.is_some() {
        return Err(conflict());
    }
    let turn = begin_in_transaction(&tx, id, conversation, question, provider, model, effort)?;
    tx.commit()?;
    Ok(turn)
}

pub(super) fn begin_inbox_action_turn(
    conn: &mut Connection,
    job: &super::inbox_actions::InboxActionJob,
) -> Result<WorkTurn> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if super::inbox_actions::reserved(&tx, job.capture.id)?.as_ref() != Some(job) {
        return Err(conflict());
    }
    if read_turn(&tx, job.capture.id)?.is_none()
        && super::inbox_original_operations::archived(&tx, job.capture.id)?.is_some()
    {
        return Err(Error::StateChanged(
            "historical Inbox analysis needs a fresh UUID; archived turns are not live Sessions"
                .into(),
        ));
    }
    let capture = &job.capture;
    let turn = begin_in_transaction(
        &tx,
        capture.id,
        capture.conversation,
        &job.question,
        &capture.provider,
        &capture.model,
        Some(&capture.effort),
    )?;
    tx.commit()?;
    Ok(turn)
}

fn begin_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    id: Uuid,
    conversation: Option<Uuid>,
    question: &str,
    provider: &str,
    model: &str,
    effort: Option<&str>,
) -> Result<WorkTurn> {
    if super::proposal_rewrite::read_job(tx, id)?.is_some() {
        return Err(conflict());
    }
    if let Some((turn, _)) = read_turn(tx, id)? {
        if turn.question != question
            || turn.provider != provider
            || turn.model != model
            || turn.effort.as_deref() != effort
            || conversation.is_some_and(|c| c != turn.conversation_id)
        {
            return Err(conflict());
        }
        return Ok(turn);
    }
    if question.trim().is_empty() {
        return Err(invalid("chat question must not be blank"));
    }
    validate_selection(provider, model)?;
    validate_effort(effort)?;
    let (conversation_id, started_at_ms) = if let Some(c) = conversation {
        let known = require_conversation(tx, c)?;
        (c, capture_time(clock_floor(tx, c, &known)?)?)
    } else {
        let c = Uuid::new_v4();
        let created = capture_time(0)?;
        tx.execute(
            "INSERT INTO conversations(id, title, created_at_ms,last_activity_at_ms) VALUES (?1, '', ?2,?2)",
            params![c.to_string(), sql_timestamp(created)?],
        )?;
        (c, created)
    };
    let sequence: i64 = tx.query_row(
        "SELECT COALESCE(MAX(sequence), 0) + 1 FROM messages WHERE conversation_id = ?1",
        [conversation_id.to_string()],
        |r| r.get(0),
    )?;
    tx.execute(
        "INSERT INTO messages(turn_id, conversation_id, sequence, role, text, provider, model, status, effort,started_at_ms)
         VALUES (?1, ?2, ?3, 'user', ?4, ?5, ?6, 'running', ?7,?8),
                (?1, ?2, ?3, 'assistant', '', ?5, ?6, 'running', ?7,?8)",
        params![id.to_string(), conversation_id.to_string(), sequence, question, provider, model, effort,sql_timestamp(started_at_ms)?],
    )?;
    tx.execute(
        "UPDATE conversations SET last_activity_at_ms=?2 WHERE id=?1",
        params![conversation_id.to_string(), sql_timestamp(started_at_ms)?],
    )?;
    let turn = read_turn(tx, id)?
        .ok_or_else(|| invalid("inserted chat turn is missing"))?
        .0;
    Ok(turn)
}

pub(super) fn finish_turn(
    conn: &mut Connection,
    id: Uuid,
    status: WorkTurnStatus,
    answer: &str,
    error_code: Option<&str>,
) -> Result<WorkTurn> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let turn = finish_in_transaction(&tx, id, status, answer, error_code, true)?;
    tx.commit()?;
    Ok(turn)
}

fn finish_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    id: Uuid,
    status: WorkTurnStatus,
    answer: &str,
    error_code: Option<&str>,
    capture_activity: bool,
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
    let finished_at_ms = if capture_activity {
        let known = require_conversation(tx, turn.conversation_id)?;
        let floor =
            clock_floor(tx, turn.conversation_id, &known)?.max(turn.started_at_ms.unwrap_or(0));
        Some(capture_time(floor)?)
    } else {
        None
    };
    tx.execute(
        "UPDATE messages SET status = ?2, error_code = ?3, finished_at_ms = ?5,
         text = CASE WHEN role = 'assistant' THEN ?4 ELSE text END WHERE turn_id = ?1",
        params![
            id.to_string(),
            status.as_str(),
            error_code,
            answer,
            finished_at_ms.map(sql_timestamp).transpose()?
        ],
    )?;
    if let Some(finished) = finished_at_ms {
        tx.execute(
            "UPDATE conversations SET last_activity_at_ms=?2 WHERE id=?1",
            params![turn.conversation_id.to_string(), sql_timestamp(finished)?],
        )?;
    }
    if sequence == 1 {
        tx.execute(
            "UPDATE conversations SET title = ?2 WHERE id = ?1",
            params![turn.conversation_id.to_string(), turn.question],
        )?;
    }
    turn.status = status;
    turn.answer = answer.into();
    turn.error_code = error_code.map(str::to_owned);
    turn.finished_at_ms = finished_at_ms;
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
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let conversations = tx
        .prepare("SELECT id FROM conversations")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for id in conversations {
        require_conversation(&tx, parse_id(id)?)?;
    }
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
                false,
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

impl WorkStore {
    pub fn chat_connection(&self) -> Result<ChatStore> {
        let db = self.dir.join(super::DB_NAME);
        crate::check_regular_single_link(&db)?;
        let conn = Connection::open_with_flags(
            db,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        super::configure(&conn)?;
        Ok(ChatStore {
            conn,
            _owner_lock: self._owner_lock.clone(),
        })
    }

    pub fn turn(&self, id: Uuid) -> Result<Option<WorkTurn>> {
        Ok(read_turn(&self.conn, id)?.map(|(turn, _)| turn))
    }

    pub fn conversations(&self) -> Result<Vec<WorkConversation>> {
        let tx = self.conn.unchecked_transaction()?;
        let rows = {
            let mut statement =
                tx.prepare("SELECT id, title FROM conversations ORDER BY created_at_ms, id")?;
            statement
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let conversations = rows
            .into_iter()
            .map(|(id, title)| {
                let id = parse_id(id)?;
                let timestamps = require_conversation(&tx, id)?;
                Ok(WorkConversation {
                    id,
                    title,
                    turns: turns(&tx, id)?.len(),
                    created_at_ms: timestamps.created,
                    last_activity_at_ms: timestamps.last_activity,
                })
            })
            .collect::<Result<_>>()?;
        tx.commit()?;
        Ok(conversations)
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
        self.begin_turn_with_effort(id, conversation, question, provider, model, None)
    }

    pub fn begin_turn_with_effort(
        &mut self,
        id: Uuid,
        conversation: Option<Uuid>,
        question: &str,
        provider: &str,
        model: &str,
        effort: Option<&str>,
    ) -> Result<WorkTurn> {
        begin_turn(
            &mut self.conn,
            id,
            conversation,
            question,
            provider,
            model,
            effort,
        )
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
