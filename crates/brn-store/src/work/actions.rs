//! Checked operational Action records. Creation and mutation belong to exact
//! proposal application and identified completion; clients use WorkStore APIs.
use super::{
    WorkStore,
    proposals::{ActionChange, ProposalStamp},
};
use crate::{Error, Result, hash, invalid};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

// Domain checks belong to the typed validator: SQLite quick_check treats CHECK
// failures as corruption, which could otherwise restore older semantic work.
pub(super) const V10: &str = "
CREATE TABLE actions (
 id TEXT PRIMARY KEY,
 version INTEGER NOT NULL,
 state TEXT NOT NULL,
 created_at_ms INTEGER NOT NULL,
 creation_sha256 BLOB NOT NULL,
 record_json BLOB NOT NULL,
 record_sha256 BLOB NOT NULL
);
CREATE INDEX actions_page ON actions(created_at_ms DESC,id DESC);";

const MAX_SHORT_BYTES: usize = 512;
const MAX_DESCRIPTION_BYTES: usize = 64 * 1024;
const MAX_UUIDS: usize = 64;
// The immutable origin and current data each retain bounded text. JSON escaping
// can expand text sixfold; UUID arrays, field names and numeric metadata have a
// separate allowance. Check this before loading a stored JSON blob.
pub(super) const MAX_STORED_BYTES: usize =
    2 * (MAX_DESCRIPTION_BYTES + 2 * MAX_SHORT_BYTES) * 6 + 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionState {
    Open,
    Waiting,
    Blocked,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionData {
    pub title: String,
    pub description: String,
    pub state: ActionState,
    pub owner: Option<String>,
    pub related_person: Option<Uuid>,
    pub related_project: Option<Uuid>,
    pub sources: Vec<Uuid>,
    pub thread: Option<Uuid>,
    pub due_on: Option<String>,
    pub follow_up_on: Option<String>,
    pub dependencies: Vec<Uuid>,
    pub parent: Option<Uuid>,
    pub follows_up: Option<Uuid>,
    pub priority: Option<ActionPriority>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionOrigin {
    pub id: Uuid,
    pub proposal: ProposalStamp,
    pub data: ActionData,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionStamp {
    pub id: Uuid,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionRecord {
    pub origin: ActionOrigin,
    pub version: u64,
    pub data: ActionData,
    pub updated_at_ms: u64,
    pub waiting_since_ms: Option<u64>,
    pub completed_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCursor {
    pub created_at_ms: u64,
    pub id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionListRequest {
    pub state: Option<ActionState>,
    pub limit: usize,
    pub before: Option<ActionCursor>,
}

impl Default for ActionListRequest {
    fn default() -> Self {
        Self {
            state: None,
            limit: 25,
            before: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionPage {
    pub entries: Vec<ActionRecord>,
    pub next_before: Option<ActionCursor>,
}

fn nonnil(id: Uuid) -> Result<()> {
    if id.is_nil() {
        return Err(invalid("Action UUID must not be nil"));
    }
    Ok(())
}

fn signed(value: u64) -> Result<()> {
    if value > i64::MAX as u64 {
        return Err(invalid(
            "Action timestamp or revision exceeds the SQLite integer range",
        ));
    }
    Ok(())
}

fn date(text: &str) -> Result<()> {
    let bytes = text.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(i, b)| i != 4 && i != 7 && !b.is_ascii_digit())
    {
        return Err(invalid("Action date must be canonical YYYY-MM-DD"));
    }
    let number = |part: &[u8]| part.iter().fold(0_u32, |n, b| n * 10 + u32::from(b - b'0'));
    let year = number(&bytes[..4]);
    let month = number(&bytes[5..7]);
    let day = number(&bytes[8..]);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > days {
        return Err(invalid("Action date is not a valid civil date"));
    }
    Ok(())
}

fn ids(values: &[Uuid]) -> Result<()> {
    if values.len() > MAX_UUIDS {
        return Err(invalid("Action UUID collection exceeds 64 entries"));
    }
    let mut seen = HashSet::new();
    for id in values {
        nonnil(*id)?;
        if !seen.insert(*id) {
            return Err(invalid("Action UUID collection contains duplicates"));
        }
    }
    Ok(())
}

impl ActionData {
    /// Pure shape validation; approval checks relation existence and dependencies.
    pub fn validate(&self, action_id: Uuid) -> Result<()> {
        nonnil(action_id)?;
        if self.title.trim().is_empty() || self.title.len() > MAX_SHORT_BYTES {
            return Err(invalid("Action title must contain 1 to 512 UTF-8 bytes"));
        }
        if self.description.len() > MAX_DESCRIPTION_BYTES {
            return Err(invalid("Action description exceeds 64 KiB"));
        }
        if self
            .owner
            .as_ref()
            .is_some_and(|owner| owner.trim().is_empty() || owner.len() > MAX_SHORT_BYTES)
        {
            return Err(invalid(
                "Action owner must contain 1 to 512 UTF-8 bytes when present",
            ));
        }
        for id in [
            self.related_person,
            self.related_project,
            self.thread,
            self.parent,
            self.follows_up,
        ]
        .into_iter()
        .flatten()
        {
            nonnil(id)?;
        }
        ids(&self.sources)?;
        ids(&self.dependencies)?;
        if self.dependencies.contains(&action_id)
            || self.parent == Some(action_id)
            || self.follows_up == Some(action_id)
        {
            return Err(invalid(
                "Action dependency, parent or follow-up cannot refer to itself",
            ));
        }
        for value in [&self.due_on, &self.follow_up_on].into_iter().flatten() {
            date(value)?;
        }
        Ok(())
    }
}

impl ActionOrigin {
    pub fn validate(&self) -> Result<()> {
        nonnil(self.id)?;
        nonnil(self.proposal.id)?;
        if self.proposal.version == 0 || self.data.state == ActionState::Completed {
            return Err(invalid(
                "Action origin needs a positive creating proposal revision and an unfinished state",
            ));
        }
        signed(self.proposal.version)?;
        signed(self.created_at_ms)?;
        self.data.validate(self.id)
    }
}

impl ActionRecord {
    pub fn stamp(&self) -> ActionStamp {
        ActionStamp {
            id: self.origin.id,
            version: self.version,
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.origin.validate()?;
        self.data.validate(self.origin.id)?;
        signed(self.version)?;
        signed(self.updated_at_ms)?;
        if self.version == 0 || self.updated_at_ms < self.origin.created_at_ms {
            return Err(invalid("Action revision or timestamp order is invalid"));
        }
        if self.version == 1
            && (self.data != self.origin.data || self.updated_at_ms != self.origin.created_at_ms)
        {
            return Err(invalid(
                "Action initial revision differs from its immutable origin",
            ));
        }
        let in_range = |time: u64| self.origin.created_at_ms <= time && time <= self.updated_at_ms;
        match (self.data.state, self.waiting_since_ms) {
            (ActionState::Waiting, Some(time)) if in_range(time) => {
                if self.version == 1 && time != self.origin.created_at_ms {
                    return Err(invalid(
                        "Initial Waiting action must start waiting at creation",
                    ));
                }
            }
            (ActionState::Waiting, _) | (_, Some(_)) => {
                return Err(invalid(
                    "Action waiting time does not match its state or timestamp range",
                ));
            }
            _ => {}
        }
        match (self.data.state, self.completed_at_ms) {
            (ActionState::Completed, Some(time)) if in_range(time) => {}
            (ActionState::Completed, _) | (_, Some(_)) => {
                return Err(invalid(
                    "Action completion time does not match its state or timestamp range",
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

impl ActionCursor {
    pub fn validate(&self) -> Result<()> {
        nonnil(self.id)?;
        signed(self.created_at_ms)
    }
}

impl ActionListRequest {
    pub fn validate(&self) -> Result<()> {
        if !(1..=200).contains(&self.limit) {
            return Err(invalid("Action page limit must be between 1 and 200"));
        }
        if let Some(cursor) = &self.before {
            cursor.validate()?;
        }
        Ok(())
    }
}

fn state_name(state: ActionState) -> &'static str {
    match state {
        ActionState::Open => "open",
        ActionState::Waiting => "waiting",
        ActionState::Blocked => "blocked",
        ActionState::Completed => "completed",
    }
}

// Only whitespace outside the owned single-quoted SQL literals is ignored.
// Literal bytes remain significant; this is not a general SQL parser.
fn normalized_sql(sql: &str) -> String {
    let mut quoted = false;
    sql.trim()
        .trim_end_matches(';')
        .chars()
        .filter(|ch| {
            if *ch == '\'' {
                quoted = !quoted;
            }
            quoted || !ch.is_ascii_whitespace()
        })
        .collect()
}

pub(super) fn check_schema(conn: &Connection) -> Result<()> {
    for ((name, kind), expected) in [("actions", "table"), ("actions_page", "index")]
        .into_iter()
        .zip(V10.split(';').filter(|sql| !sql.trim().is_empty()))
    {
        let actual: Option<(String, String, String)> = conn
            .query_row(
                "SELECT type,tbl_name,sql FROM sqlite_schema WHERE name=?1",
                [name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if !actual.is_some_and(|(actual_kind, table, sql)| {
            actual_kind == kind
                && table == "actions"
                && normalized_sql(&sql) == normalized_sql(expected)
        }) {
            return Err(invalid(
                "Action schema or page index differs from its owned shape",
            ));
        }
    }
    let objects: Vec<(String, String)> = conn
        .prepare("SELECT type,name FROM sqlite_schema WHERE tbl_name='actions' ORDER BY name")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if objects
        != [
            ("table".into(), "actions".into()),
            ("index".into(), "actions_page".into()),
            ("index".into(), "sqlite_autoindex_actions_1".into()),
        ]
    {
        return Err(invalid(
            "Action schema contains missing or unexpected owned objects",
        ));
    }
    Ok(())
}

struct ActionRow {
    version: i64,
    state: String,
    created: i64,
    creation: Vec<u8>,
    bytes: Option<Vec<u8>>,
    digest: Vec<u8>,
}

pub(super) fn read(conn: &Connection, id: Uuid) -> Result<Option<ActionRecord>> {
    nonnil(id)?;
    let row: Option<ActionRow> = conn.query_row(
        "SELECT version,state,created_at_ms,creation_sha256,CASE WHEN length(record_json)<=?2 THEN record_json END,record_sha256 FROM actions WHERE id=?1",
        params![id.to_string(), MAX_STORED_BYTES as i64],
        |row| Ok(ActionRow {version:row.get(0)?,state:row.get(1)?,created:row.get(2)?,creation:row.get(3)?,bytes:row.get(4)?,digest:row.get(5)?}),
    ).optional()?;
    row.map(|row| {
        let bytes = row
            .bytes
            .ok_or_else(|| invalid("Stored Action exceeds its encoded size limit"))?;
        if row.digest.as_slice() != hash(&bytes) {
            return Err(invalid("Stored Action failed its record hash check"));
        }
        let record: ActionRecord =
            serde_json::from_slice(&bytes).map_err(|_| invalid("Invalid stored Action record"))?;
        record.validate()?;
        let origin = serde_json::to_vec(&record.origin)
            .map_err(|_| invalid("Could not encode Action origin"))?;
        if record.origin.id != id
            || i64::try_from(record.version).ok() != Some(row.version)
            || state_name(record.data.state) != row.state
            || i64::try_from(record.origin.created_at_ms).ok() != Some(row.created)
            || row.creation.as_slice() != hash(&origin)
        {
            return Err(invalid(
                "Stored Action differs from its immutable origin or indexed row binding",
            ));
        }
        Ok(record)
    })
    .transpose()
}

/// Called only within existing proposal transactions; there is no standalone
/// Action mutation API. Full records, rather than revisions alone, bind CAS.
pub(super) fn check_changes(tx: &Transaction<'_>, changes: &[ActionChange]) -> Result<()> {
    if changes.is_empty() {
        return Ok(());
    }
    check_schema(tx)?;
    for change in changes {
        let current = read(tx, change.id())?;
        let matches = match change {
            ActionChange::Create { .. } => current.is_none(),
            ActionChange::Replace { before, .. } => current.as_ref() == Some(before.as_ref()),
        };
        if !matches {
            return Err(Error::StateChanged(
                "Action differs from its exact approval baseline".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn write(tx: &Transaction<'_>, record: &ActionRecord) -> Result<()> {
    record.validate()?;
    let bytes =
        serde_json::to_vec(record).map_err(|_| invalid("Could not encode Action record"))?;
    if bytes.len() > MAX_STORED_BYTES {
        return Err(invalid("Stored Action exceeds its encoded size limit"));
    }
    let origin = serde_json::to_vec(&record.origin)
        .map_err(|_| invalid("Could not encode Action origin"))?;
    tx.execute(
        "INSERT INTO actions(id,version,state,created_at_ms,creation_sha256,record_json,record_sha256) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET version=excluded.version,state=excluded.state,record_json=excluded.record_json,record_sha256=excluded.record_sha256",
        params![record.origin.id.to_string(),record.version as i64,state_name(record.data.state),record.origin.created_at_ms as i64,hash(&origin).as_slice(),bytes,hash(&bytes).as_slice()],
    )?;
    Ok(())
}

pub(super) fn apply(
    tx: &Transaction<'_>,
    changes: &[ActionChange],
    after: &[ActionRecord],
) -> Result<()> {
    check_changes(tx, changes)?;
    for record in after {
        write(tx, record)?;
    }
    Ok(())
}

/// Recovery preserves newer real work. Incompatible immutable origins or
/// equal-version forks refuse the whole enclosing proposal import transaction.
pub(super) fn restore<'a>(
    tx: &Transaction<'_>,
    records: impl Iterator<Item = &'a ActionRecord>,
) -> Result<()> {
    let mut records = records.peekable();
    if records.peek().is_none() {
        return Ok(());
    }
    check_schema(tx)?;
    for record in records {
        record.validate()?;
        if let Some(current) = read(tx, record.origin.id)? {
            if current.origin != record.origin
                || current.version == record.version && current != *record
            {
                return Err(Error::OperationConflict(
                    "Action recovery conflicts with its retained lineage".into(),
                ));
            }
            if current.version >= record.version {
                continue;
            }
            if current.data.state == ActionState::Completed {
                return Err(Error::OperationConflict(
                    "Action recovery cannot supersede completed work".into(),
                ));
            }
        }
        write(tx, record)?;
    }
    Ok(())
}

/// Semantic corruption refuses before reconciliation/backup; physical SQLite
/// corruption keeps WorkStore's existing restoration behavior.
pub(super) fn check_all(conn: &Connection) -> Result<()> {
    check_schema(conn)?;
    let mut statement = conn.prepare("SELECT id FROM actions ORDER BY id")?;
    for raw in statement.query_map([], |row| row.get::<_, String>(0))? {
        let raw = raw?;
        let id = crate::parse_id(raw.clone())?;
        if raw != id.to_string() || read(conn, id)?.is_none() {
            return Err(invalid("Invalid stored Action row identity"));
        }
    }
    Ok(())
}

impl WorkStore {
    /// Retained operational data; this does not create or change a real Action.
    pub fn action(&self, id: Uuid) -> Result<Option<ActionRecord>> {
        nonnil(id)?;
        let tx = self.conn.unchecked_transaction()?;
        check_schema(&tx)?;
        let record = read(&tx, id)?;
        tx.commit()?;
        Ok(record)
    }

    /// One snapshot binds validation, filtering, immutable ordering and the page.
    pub fn action_list(&self, request: &ActionListRequest) -> Result<ActionPage> {
        request.validate()?;
        let tx = self.conn.unchecked_transaction()?;
        check_all(&tx)?;
        let ids = {
            let mut statement = tx.prepare(
                "SELECT id FROM actions WHERE (?1 IS NULL OR state=?1) AND (?2 IS NULL OR created_at_ms<?2 OR (created_at_ms=?2 AND id<?3)) ORDER BY created_at_ms DESC,id DESC LIMIT ?4",
            )?;
            statement
                .query_map(
                    params![
                        request.state.map(state_name),
                        request.before.map(|c| c.created_at_ms as i64),
                        request.before.map(|c| c.id.to_string()),
                        (request.limit + 1) as i64
                    ],
                    |row| row.get::<_, String>(0),
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut entries = ids
            .into_iter()
            .map(|id| {
                read(&tx, crate::parse_id(id)?)?.ok_or_else(|| invalid("Listed Action disappeared"))
            })
            .collect::<Result<Vec<_>>>()?;
        let more = entries.len() > request.limit;
        entries.truncate(request.limit);
        let next_before = more.then(|| {
            let last = entries.last().expect("positive Action page limit");
            ActionCursor {
                created_at_ms: last.origin.created_at_ms,
                id: last.origin.id,
            }
        });
        tx.commit()?;
        Ok(ActionPage {
            entries,
            next_before,
        })
    }
}
