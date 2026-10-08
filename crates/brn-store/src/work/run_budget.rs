//! Immutable operational run limits, separate from canonical work/evidence records.
use super::{WorkStore, chat::ChatStore};
use crate::{Error, Result, invalid};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) const V17: &str = "
CREATE TABLE ai_run_budgets (
 run_id TEXT PRIMARY KEY CHECK(length(CAST(run_id AS BLOB))=36),
 max_tool_rounds INTEGER NOT NULL CHECK(typeof(max_tool_rounds)='integer' AND max_tool_rounds BETWEEN 1 AND 32),
 timeout_seconds INTEGER NOT NULL CHECK(typeof(timeout_seconds)='integer' AND timeout_seconds BETWEEN 1 AND 3600)
);";

/// Frozen investigation limits; validation never admits work or changes history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct WorkBudget {
    /// Admitted tool-containing model responses, including parallel calls once (1..32).
    pub max_tool_rounds: u16,
    /// Owned investigation deadline in seconds (1..3600); workflow enforces draining.
    pub timeout_seconds: u32,
}

impl Default for WorkBudget {
    fn default() -> Self {
        Self {
            max_tool_rounds: 8,
            timeout_seconds: 300,
        }
    }
}

impl WorkBudget {
    /// Refuse out-of-policy values before persistence or provider work.
    pub fn validate(&self) -> Result<()> {
        if !(1..=32).contains(&self.max_tool_rounds) || !(1..=3600).contains(&self.timeout_seconds)
        {
            return Err(invalid("invalid AI work budget"));
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for WorkBudget {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            max_tool_rounds: u16,
            timeout_seconds: u32,
        }
        let fields = Fields::deserialize(deserializer)?;
        let budget = Self {
            max_tool_rounds: fields.max_tool_rounds,
            timeout_seconds: fields.timeout_seconds,
        };
        budget.validate().map_err(serde::de::Error::custom)?;
        Ok(budget)
    }
}

fn check_id(id: Uuid) -> Result<()> {
    if id.is_nil() {
        return Err(invalid("AI run budget needs a nonnil UUID"));
    }
    Ok(())
}

fn normalized(sql: &str) -> String {
    sql.trim()
        .trim_end_matches(';')
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect()
}

fn check_schema(conn: &Connection) -> Result<()> {
    let schema: Option<(String, String, String)> = conn
        .query_row(
            "SELECT type,tbl_name,sql FROM sqlite_schema WHERE name='ai_run_budgets'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    if !schema.is_some_and(|(kind, table, sql)| {
        kind == "table" && table == "ai_run_budgets" && normalized(&sql) == normalized(V17)
    }) {
        return Err(invalid(
            "AI run budget schema differs from the supported schema",
        ));
    }
    let extra: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE tbl_name='ai_run_budgets' AND name NOT IN ('ai_run_budgets','sqlite_autoindex_ai_run_budgets_1')", [], |row| row.get(0),
    )?;
    if extra != 0 {
        return Err(invalid("AI run budget has unexpected schema objects"));
    }
    Ok(())
}

pub(super) fn read(conn: &Connection, id: Uuid) -> Result<Option<WorkBudget>> {
    check_id(id)?;
    check_schema(conn)?;
    let row: Option<(i64, i64)> = conn
        .query_row(
            "SELECT max_tool_rounds,timeout_seconds FROM ai_run_budgets WHERE run_id=?1",
            [id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    row.map(|(rounds, seconds)| {
        let budget = WorkBudget {
            max_tool_rounds: u16::try_from(rounds)
                .map_err(|_| invalid("invalid stored AI tool-round limit"))?,
            timeout_seconds: u32::try_from(seconds)
                .map_err(|_| invalid("invalid stored AI time limit"))?,
        };
        budget.validate()?;
        Ok(budget)
    })
    .transpose()
}

pub(super) fn check_all(conn: &Connection) -> Result<()> {
    check_schema(conn)?;
    let ids = conn.prepare("SELECT CASE WHEN length(CAST(run_id AS BLOB))=36 THEN run_id END FROM ai_run_budgets ORDER BY rowid")?
        .query_map([], |row| row.get::<_, Option<String>>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for id in ids {
        let id = id.ok_or_else(|| invalid("invalid stored AI run UUID"))?;
        let uuid = Uuid::parse_str(&id).map_err(|_| invalid("invalid stored AI run UUID"))?;
        if uuid.to_string() != id || read(conn, uuid)?.is_none() {
            return Err(invalid("invalid stored AI run budget"));
        }
    }
    Ok(())
}

/// Resolve without recording a choice or changing historical records.
pub(super) fn resolve(
    conn: &Connection,
    id: Uuid,
    requested: Option<WorkBudget>,
) -> Result<Option<WorkBudget>> {
    check_id(id)?;
    if let Some(budget) = requested {
        budget.validate()?;
    }
    if let Some(recorded) = read(conn, id)? {
        if requested.is_some_and(|budget| budget != recorded) {
            return Err(conflict());
        }
        return Ok(Some(recorded));
    }
    let existing: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM messages WHERE turn_id=?1) OR EXISTS(SELECT 1 FROM inbox_actions WHERE id=?1) OR EXISTS(SELECT 1 FROM proposal_rewrites WHERE id=?1)",
        [id.to_string()], |row| row.get(0),
    )?;
    if existing || super::inbox_original_operations::has_archived_analysis(conn, id)? {
        if requested.is_some() {
            return Err(conflict());
        }
        return Ok(None);
    }
    Ok(Some(requested.unwrap_or_default()))
}

fn conflict() -> Error {
    Error::OperationConflict("AI run UUID reused with a different work budget".into())
}

pub(super) fn insert(conn: &Connection, id: Uuid, budget: WorkBudget) -> Result<()> {
    check_id(id)?;
    budget.validate()?;
    conn.execute(
        "INSERT INTO ai_run_budgets(run_id,max_tool_rounds,timeout_seconds) VALUES (?1,?2,?3)",
        params![
            id.to_string(),
            budget.max_tool_rounds,
            budget.timeout_seconds
        ],
    )?;
    Ok(())
}

/// Budget-only retained history is a UUID tombstone, never a fresh admission.
pub(super) fn refuse_retained(conn: &Connection, id: Uuid) -> Result<()> {
    if read(conn, id)?.is_some() {
        return Err(Error::StateChanged(
            "historical AI run needs a fresh UUID".into(),
        ));
    }
    Ok(())
}

impl WorkStore {
    /// Recorded immutable limits; historical absence remains unknown.
    pub fn run_budget(&self, id: Uuid) -> Result<Option<WorkBudget>> {
        read(&self.conn, id)
    }
    /// Resolve fresh defaults or recorded limits without writing; changed replay conflicts.
    pub fn resolve_run_budget(
        &self,
        id: Uuid,
        requested: Option<WorkBudget>,
    ) -> Result<Option<WorkBudget>> {
        resolve(&self.conn, id, requested)
    }
}
impl ChatStore {
    /// Recorded immutable limits; historical absence remains unknown.
    pub fn run_budget(&self, id: Uuid) -> Result<Option<WorkBudget>> {
        read(&self.conn, id)
    }
    /// Resolve fresh defaults or recorded limits without writing; changed replay conflicts.
    pub fn resolve_run_budget(
        &self,
        id: Uuid,
        requested: Option<WorkBudget>,
    ) -> Result<Option<WorkBudget>> {
        resolve(&self.conn, id, requested)
    }
}
