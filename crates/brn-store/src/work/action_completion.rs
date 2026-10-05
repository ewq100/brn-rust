//! Exact identified completion and checked operational recovery evidence.
use super::{
    WorkStore,
    actions::{self, ActionRecord, ActionState},
};
use crate::{Error, Result, hash, invalid};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) const V11: &str = "
CREATE TABLE action_completions (
 operation_id TEXT PRIMARY KEY,
 action_id TEXT NOT NULL UNIQUE,
 request_sha256 BLOB NOT NULL,
 completion_json BLOB NOT NULL,
 completion_sha256 BLOB NOT NULL
);";
const MAX_COMPLETION_BYTES: usize = 2 * actions::MAX_STORED_BYTES + 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteActionRequest {
    pub operation_id: Uuid,
    pub before: Box<ActionRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCompletion {
    pub request: CompleteActionRequest,
    pub after: ActionRecord,
}
impl CompleteActionRequest {
    pub fn validate(&self) -> Result<()> {
        if self.operation_id.is_nil() {
            return Err(invalid("Action completion operation UUID must not be nil"));
        }
        self.before.validate()?;
        if self.before.data.state == ActionState::Completed {
            return Err(invalid("Completed Action cannot be completed again"));
        }
        Ok(())
    }
}
impl ActionCompletion {
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        self.after.validate()?;
        let expected = completed(&self.request, self.after.updated_at_ms)?;
        if self.after != expected {
            return Err(invalid(
                "Action completion differs from its exact before record",
            ));
        }
        Ok(())
    }
}
fn completed(request: &CompleteActionRequest, at_ms: u64) -> Result<ActionRecord> {
    request.validate()?;
    if at_ms > i64::MAX as u64 {
        return Err(invalid(
            "Action completion timestamp exceeds SQLite integer range",
        ));
    }
    let mut after = *request.before.clone();
    after.version = after
        .version
        .checked_add(1)
        .filter(|version| *version <= i64::MAX as u64)
        .ok_or_else(|| invalid("Action completion revision exceeds SQLite integer range"))?;
    after.data.state = ActionState::Completed;
    after.updated_at_ms = at_ms.max(after.updated_at_ms);
    after.waiting_since_ms = None;
    after.completed_at_ms = Some(after.updated_at_ms);
    after.validate()?;
    Ok(after)
}
fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| invalid("Could not encode Action completion"))?;
    if bytes.len() > MAX_COMPLETION_BYTES {
        return Err(invalid("Action completion exceeds its encoded size limit"));
    }
    Ok(bytes)
}
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
fn check_schema(conn: &Connection) -> Result<()> {
    let sql: Option<String> = conn.query_row("SELECT sql FROM sqlite_schema WHERE type='table' AND name='action_completions' AND tbl_name='action_completions'", [], |row| row.get(0)).optional()?;
    if !sql.is_some_and(|sql| normalized_sql(&sql) == normalized_sql(V11)) {
        return Err(invalid(
            "Action completion schema differs from its owned shape",
        ));
    }
    let objects: Vec<(String, String)> = conn
        .prepare(
            "SELECT type,name FROM sqlite_schema WHERE tbl_name='action_completions' ORDER BY name",
        )?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if objects
        != [
            ("table".into(), "action_completions".into()),
            (
                "index".into(),
                "sqlite_autoindex_action_completions_1".into(),
            ),
            (
                "index".into(),
                "sqlite_autoindex_action_completions_2".into(),
            ),
        ]
    {
        return Err(invalid(
            "Action completion schema contains missing or unexpected owned objects",
        ));
    }
    Ok(())
}
fn canonical_id(raw: &str) -> Result<Uuid> {
    let id = Uuid::parse_str(raw).map_err(|_| invalid("Invalid Action completion UUID"))?;
    if id.is_nil() || id.to_string() != raw {
        return Err(invalid(
            "Action completion UUID must be canonical and nonnil",
        ));
    }
    Ok(id)
}
struct CompletionRow {
    action_id: String,
    request_hash: Vec<u8>,
    bytes: Option<Vec<u8>>,
    completion_hash: Vec<u8>,
}
fn read(conn: &Connection, operation_id: Uuid) -> Result<Option<ActionCompletion>> {
    if operation_id.is_nil() {
        return Err(invalid("Action completion UUID must not be nil"));
    }
    let row: Option<CompletionRow> = conn.query_row(
        "SELECT action_id,request_sha256,CASE WHEN length(completion_json)<=?2 THEN completion_json END,completion_sha256 FROM action_completions WHERE operation_id=?1",
        params![operation_id.to_string(), MAX_COMPLETION_BYTES as i64],
        |row| Ok(CompletionRow { action_id: row.get(0)?, request_hash: row.get(1)?, bytes: row.get(2)?, completion_hash: row.get(3)? }),
    ).optional()?;
    row.map(|row| {
        let action_id = canonical_id(&row.action_id)?;
        let bytes = row
            .bytes
            .ok_or_else(|| invalid("Stored Action completion exceeds its encoded size limit"))?;
        if row.completion_hash.as_slice() != hash(&bytes) {
            return Err(invalid("Stored Action completion failed its hash check"));
        }
        let completion: ActionCompletion = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("Invalid stored Action completion"))?;
        completion.validate()?;
        if completion.request.operation_id != operation_id
            || completion.request.before.origin.id != action_id
            || row.request_hash.as_slice() != hash(&encode(&completion.request)?)
        {
            return Err(invalid(
                "Stored Action completion differs from its indexed request binding",
            ));
        }
        Ok(completion)
    })
    .transpose()
}
fn existing_for_action(conn: &Connection, action_id: Uuid) -> Result<Option<ActionCompletion>> {
    let operation: Option<String> = conn
        .query_row(
            "SELECT operation_id FROM action_completions WHERE action_id=?1",
            [action_id.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    operation
        .map(|operation| {
            read(conn, canonical_id(&operation)?)?
                .ok_or_else(|| invalid("Indexed Action completion disappeared"))
        })
        .transpose()
}
fn bound_replay(
    conn: &Connection,
    request: &CompleteActionRequest,
) -> Result<Option<ActionCompletion>> {
    let completion = read(conn, request.operation_id)?;
    if completion
        .as_ref()
        .is_some_and(|completion| completion.request != *request)
    {
        return Err(Error::OperationConflict(
            "Action completion operation has another exact request".into(),
        ));
    }
    request.validate()?;
    Ok(completion)
}
fn require_retained_completion(conn: &Connection, completion: &ActionCompletion) -> Result<()> {
    let current = actions::read(conn, completion.after.origin.id)?
        .ok_or_else(|| invalid("Stored Action completion has no retained Action"))?;
    if current.origin != completion.after.origin
        || current.data.state != ActionState::Completed
        || current.version < completion.after.version
        || current.version == completion.after.version && current != completion.after
    {
        return Err(invalid(
            "Stored Action completion conflicts with its retained completed Action",
        ));
    }
    Ok(())
}
pub(super) fn check_all(conn: &Connection) -> Result<()> {
    check_schema(conn)?;
    let mut statement =
        conn.prepare("SELECT operation_id FROM action_completions ORDER BY operation_id")?;
    for operation in statement.query_map([], |row| row.get::<_, String>(0))? {
        let completion = read(conn, canonical_id(&operation?)?)?
            .ok_or_else(|| invalid("Stored Action completion disappeared"))?;
        require_retained_completion(conn, &completion)?;
    }
    Ok(())
}
fn insert(tx: &Transaction<'_>, completion: &ActionCompletion, bytes: Vec<u8>) -> Result<()> {
    tx.execute("INSERT INTO action_completions(operation_id,action_id,request_sha256,completion_json,completion_sha256) VALUES(?1,?2,?3,?4,?5)", params![completion.request.operation_id.to_string(), completion.after.origin.id.to_string(), hash(&encode(&completion.request)?).as_slice(), &bytes, hash(&bytes).as_slice()])?;
    Ok(())
}
impl WorkStore {
    /// Replay binds the whole exact command, without publishing recovery evidence again.
    pub fn action_completion_for(
        &self,
        request: &CompleteActionRequest,
    ) -> Result<Option<ActionCompletion>> {
        let tx = self.conn.unchecked_transaction()?;
        check_schema(&tx)?;
        let completion = bound_replay(&tx, request)?;
        tx.commit()?;
        Ok(completion)
    }
    /// Publish only the checked exact recovery evidence, under SQLite write exclusion.
    pub fn complete_action_with(
        &mut self,
        request: &CompleteActionRequest,
        at_ms: u64,
        publish: impl FnOnce(&ActionCompletion) -> Result<()>,
    ) -> Result<ActionCompletion> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_schema(&tx)?;
        if let Some(completion) = bound_replay(&tx, request)? {
            tx.commit()?;
            return Ok(completion);
        }
        if existing_for_action(&tx, request.before.origin.id)?.is_some() {
            return Err(Error::OperationConflict(
                "Action already has an identified completion".into(),
            ));
        }
        actions::check_schema(&tx)?;
        let current = actions::read(&tx, request.before.origin.id)?
            .ok_or_else(|| Error::NotFound("Action does not exist".into()))?;
        if current != *request.before {
            return Err(Error::StateChanged(
                "Action differs from its exact completion baseline".into(),
            ));
        }
        let completion = ActionCompletion {
            request: request.clone(),
            after: completed(request, at_ms)?,
        };
        completion.validate()?;
        let bytes = encode(&completion)?;
        publish(&completion)?;
        actions::write(&tx, &completion.after)?;
        insert(&tx, &completion, bytes)?;
        tx.commit()?;
        Ok(completion)
    }
    /// Import exact completion evidence into older/fresh work without replacing newer completion.
    pub fn restore_action_completion(
        &mut self,
        completion: &ActionCompletion,
    ) -> Result<ActionCompletion> {
        completion.validate()?;
        let bytes = encode(completion)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_schema(&tx)?;
        actions::check_schema(&tx)?;
        let existing = read(&tx, completion.request.operation_id)?;
        if existing
            .as_ref()
            .is_some_and(|existing| existing != completion)
        {
            return Err(Error::OperationConflict(
                "Action completion recovery has another exact snapshot".into(),
            ));
        }
        if let Some(other) = existing_for_action(&tx, completion.after.origin.id)?
            && other != *completion
        {
            return Err(Error::OperationConflict(
                "Action already has another identified completion".into(),
            ));
        }
        if let Some(current) = actions::read(&tx, completion.after.origin.id)? {
            let before = completion.request.before.as_ref();
            let after = &completion.after;
            if current.origin != before.origin
                || current.version == before.version && current != *before
                || current.version == after.version && current != *after
                || current.version > after.version && current.data.state != ActionState::Completed
                || current.version < after.version && current.data.state == ActionState::Completed
            {
                return Err(Error::OperationConflict(
                    "Action completion recovery conflicts with retained lineage".into(),
                ));
            }
        }
        actions::restore(
            &tx,
            [completion.request.before.as_ref(), &completion.after].into_iter(),
        )?;
        if existing.is_none() {
            insert(&tx, completion, bytes)?;
        }
        require_retained_completion(&tx, completion)?;
        tx.commit()?;
        Ok(completion.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::{
        actions::ActionData,
        proposal_apply::{ApplyOutcome, ApprovalRequest},
        proposals::{ActionChange, ProposalDraft},
    };
    use std::cell::Cell;

    #[test]
    fn sqlite_full_after_publication_rolls_back_action_and_receipt_together() {
        let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let id = Uuid::new_v4();
        let data = ActionData {
            title: "Approved exact completion".into(),
            description: "λ".repeat(32 * 1024),
            state: ActionState::Waiting,
            owner: None,
            related_person: None,
            related_project: None,
            sources: Vec::new(),
            thread: None,
            due_on: None,
            follow_up_on: None,
            dependencies: Vec::new(),
            parent: None,
            follows_up: None,
            priority: None,
        };
        let review = store
            .create_proposal(&ProposalDraft {
                inbox_knowledge: None,
                inbox_source: None,
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                vault: None,
                title: "Approved exact fixture".into(),
                changes: Vec::new(),
                sources: Vec::new(),
                action_changes: vec![ActionChange::Create { id, data }],
            })
            .unwrap();
        let operation_id = Uuid::new_v4();
        store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id,
                expected: review.stamp(),
            })
            .unwrap();
        store.record_proposal_prepared(operation_id, &[]).unwrap();
        store
            .finish_proposal_apply(operation_id, ApplyOutcome::Applied, Some(&[]))
            .unwrap();
        let before = store.action(id).unwrap().unwrap();
        let request = CompleteActionRequest {
            operation_id: Uuid::new_v4(),
            before: Box::new(before.clone()),
        };
        let pages: i64 = store
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .unwrap();
        let maximum: i64 = store
            .conn
            .query_row(&format!("PRAGMA max_page_count={pages}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(maximum, pages);
        let published = Cell::new(false);
        let error = store
            .complete_action_with(&request, 0, |candidate| {
                candidate.validate()?;
                published.set(true);
                Ok(())
            })
            .unwrap_err();
        assert!(
            published.get(),
            "SQL failure must follow the exact evidence publication"
        );
        assert!(
            matches!(error, Error::Sql(ref error) if error.sqlite_error_code() == Some(rusqlite::ErrorCode::DiskFull))
        );
        assert_eq!(store.action(id).unwrap(), Some(before));
        assert!(store.action_completion_for(&request).unwrap().is_none());
        assert_eq!(
            store
                .conn
                .query_row("SELECT count(*) FROM action_completions", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
