//! Reversible session organization; canonical chat and captured outcomes stay unchanged.
use super::{WorkConversation, WorkStore, chat, proposal_rewrite, proposals};
use crate::{Error, Result, hash, invalid, parse_id};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub(super) const V18: &str = "
CREATE TABLE conversation_lifecycle (
 conversation_id TEXT PRIMARY KEY REFERENCES conversations(id),
 version INTEGER NOT NULL CHECK(typeof(version)='integer' AND version>0),
 state TEXT NOT NULL CHECK(state IN ('active','archived'))
);
CREATE TABLE conversation_lifecycle_operations (
 operation_id TEXT PRIMARY KEY,
 conversation_id TEXT NOT NULL REFERENCES conversations(id),
 after_version INTEGER NOT NULL CHECK(typeof(after_version)='integer' AND after_version>1),
 receipt_json BLOB NOT NULL CHECK(length(receipt_json)<=4096),
 receipt_sha256 BLOB NOT NULL CHECK(length(receipt_sha256)=32),
 UNIQUE(conversation_id,after_version)
);
INSERT INTO conversation_lifecycle(conversation_id,version,state)
 SELECT id,1,'active' FROM conversations;";
const MAX_RECEIPT_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationState {
    Active,
    Archived,
}
impl ConversationState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationFilter {
    Active,
    Archived,
    All,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationStamp {
    pub id: Uuid,
    pub version: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationLifecycle {
    pub stamp: ConversationStamp,
    pub state: ConversationState,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationSummary {
    pub conversation: WorkConversation,
    pub lifecycle: ConversationLifecycle,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationLifecycleRequest {
    pub operation_id: Uuid,
    pub expected: ConversationStamp,
    pub target: ConversationState,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationLifecycleReceipt {
    pub request: ConversationLifecycleRequest,
    pub before: ConversationLifecycle,
    pub after: ConversationLifecycle,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationLifecycleResult {
    pub receipt: ConversationLifecycleReceipt,
    pub current: ConversationLifecycle,
}
impl ConversationStamp {
    pub fn validate(&self) -> Result<()> {
        if self.id.is_nil() || self.version == 0 || self.version > i64::MAX as u64 {
            return Err(invalid(
                "invalid conversation lifecycle identity or version",
            ));
        }
        Ok(())
    }
}
impl ConversationLifecycle {
    pub fn validate(&self) -> Result<()> {
        self.stamp.validate()
    }
}
impl ConversationLifecycleRequest {
    pub fn validate(&self) -> Result<()> {
        self.expected.validate()?;
        if self.operation_id.is_nil() || self.expected.version >= i64::MAX as u64 {
            return Err(invalid(
                "invalid conversation lifecycle operation or version increment",
            ));
        }
        Ok(())
    }
}
impl ConversationLifecycleReceipt {
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        self.before.validate()?;
        self.after.validate()?;
        if self.before.stamp != self.request.expected
            || self.after.stamp.id != self.before.stamp.id
            || self.after.stamp.version != self.before.stamp.version + 1
            || self.after.state != self.request.target
            || self.after.state == self.before.state
        {
            return Err(invalid("invalid conversation lifecycle transition receipt"));
        }
        Ok(())
    }
}
impl ConversationLifecycleResult {
    pub fn validate(&self) -> Result<()> {
        self.receipt.validate()?;
        self.current.validate()?;
        let after = self.receipt.after;
        if self.current.stamp.id != after.stamp.id
            || self.current.stamp.version < after.stamp.version
        {
            return Err(invalid(
                "invalid current conversation lifecycle acknowledgement",
            ));
        }
        let same = (self.current.stamp.version - after.stamp.version).is_multiple_of(2);
        if (self.current.state == after.state) != same {
            return Err(invalid(
                "current conversation lifecycle contradicts transition history",
            ));
        }
        Ok(())
    }
}
fn normalized(sql: &str) -> String {
    sql.trim()
        .trim_end_matches(';')
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect()
}
fn check_schema(conn: &Connection) -> Result<()> {
    for (name, expected) in [
        "conversation_lifecycle",
        "conversation_lifecycle_operations",
    ]
    .into_iter()
    .zip(V18.split(';'))
    {
        let actual: Option<(String, String, String)> = conn
            .query_row(
                "SELECT type,tbl_name,sql FROM sqlite_schema WHERE name=?1",
                [name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if !actual.is_some_and(|(kind, table, sql)| {
            kind == "table" && table == name && normalized(&sql) == normalized(expected)
        }) {
            return Err(invalid(
                "conversation lifecycle schema differs from the supported schema",
            ));
        }
        let extras: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE tbl_name=?1 AND name!=?1 AND NOT (type='index' AND sql IS NULL AND name IN ('sqlite_autoindex_conversation_lifecycle_1','sqlite_autoindex_conversation_lifecycle_operations_1','sqlite_autoindex_conversation_lifecycle_operations_2'))",
            [name], |row| row.get(0),
        )?;
        if extras != 0 {
            return Err(invalid(
                "conversation lifecycle has unexpected schema objects",
            ));
        }
    }
    Ok(())
}
fn read(conn: &Connection, id: Uuid) -> Result<ConversationLifecycle> {
    if id.is_nil() {
        return Err(invalid("conversation needs a nonnil UUID"));
    }
    let row: Option<(i64, String)> = conn
        .query_row(
            "SELECT version,state FROM conversation_lifecycle WHERE conversation_id=?1",
            [id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (version, state) = row.ok_or_else(|| Error::NotFound("conversation not found".into()))?;
    let state = match state.as_str() {
        "active" => ConversationState::Active,
        "archived" => ConversationState::Archived,
        _ => return Err(invalid("invalid stored conversation lifecycle state")),
    };
    let lifecycle = ConversationLifecycle {
        stamp: ConversationStamp {
            id,
            version: u64::try_from(version)
                .map_err(|_| invalid("invalid stored conversation lifecycle version"))?,
        },
        state,
    };
    lifecycle.validate()?;
    Ok(lifecycle)
}
fn receipt(conn: &Connection, operation: Uuid) -> Result<Option<ConversationLifecycleReceipt>> {
    type Row = (String, i64, Option<Vec<u8>>, Vec<u8>);
    let row: Option<Row> = conn.query_row(
        "SELECT conversation_id,after_version,CASE WHEN length(receipt_json)<=?2 THEN receipt_json END,receipt_sha256 FROM conversation_lifecycle_operations WHERE operation_id=?1",
        params![operation.to_string(), MAX_RECEIPT_BYTES as i64], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
    ).optional()?;
    row.map(|(conversation, version, bytes, digest)| {
        let bytes = bytes.ok_or_else(|| invalid("stored lifecycle receipt exceeds its limit"))?;
        if digest.as_slice() != hash(&bytes) {
            return Err(invalid("stored lifecycle receipt failed its hash check"));
        }
        let receipt: ConversationLifecycleReceipt = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("invalid stored conversation lifecycle receipt"))?;
        receipt.validate()?;
        if receipt.request.operation_id != operation
            || receipt.after.stamp.id.to_string() != conversation
            || receipt.after.stamp.version != version as u64
        {
            return Err(invalid("lifecycle receipt differs from its row identity"));
        }
        Ok(receipt)
    })
    .transpose()
}
pub(super) fn check_all(conn: &Connection) -> Result<()> {
    check_schema(conn)?;
    let ids = conn
        .prepare("SELECT id FROM conversations ORDER BY id")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut chains = HashMap::new();
    for id in &ids {
        let uuid = parse_id(id.clone())?;
        if uuid.is_nil() || uuid.to_string() != *id {
            return Err(invalid("invalid stored conversation UUID"));
        }
        read(conn, uuid)?;
        chains.insert(
            uuid,
            ConversationLifecycle {
                stamp: ConversationStamp {
                    id: uuid,
                    version: 1,
                },
                state: ConversationState::Active,
            },
        );
    }
    let count: i64 = conn.query_row("SELECT count(*) FROM conversation_lifecycle", [], |row| {
        row.get(0)
    })?;
    if count != ids.len() as i64 {
        return Err(invalid(
            "conversation lifecycle row coverage differs from conversations",
        ));
    }
    let operations = conn.prepare("SELECT operation_id FROM conversation_lifecycle_operations ORDER BY conversation_id,after_version")?.query_map([],|row|row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for operation in operations {
        let uuid = parse_id(operation.clone())?;
        if uuid.is_nil() || uuid.to_string() != operation {
            return Err(invalid("invalid stored lifecycle operation UUID"));
        }
        let receipt = receipt(conn, uuid)?.ok_or_else(|| invalid("missing lifecycle receipt"))?;
        let previous = chains
            .get_mut(&receipt.before.stamp.id)
            .ok_or_else(|| invalid("lifecycle receipt references an absent conversation"))?;
        if *previous != receipt.before {
            return Err(invalid(
                "conversation lifecycle receipt chain is incomplete",
            ));
        }
        *previous = receipt.after;
    }
    for (id, expected) in chains {
        if read(conn, id)? != expected {
            return Err(invalid(
                "conversation lifecycle metadata differs from its receipt chain",
            ));
        }
    }
    Ok(())
}
pub(super) fn require_active(conn: &Connection, id: Uuid) -> Result<()> {
    check_all(conn)?;
    if read(conn, id)?.state != ConversationState::Active {
        return Err(Error::StateChanged(
            "Restore this archived session before continuing.".into(),
        ));
    }
    Ok(())
}
/// Legacy proposal session identifiers may precede persisted conversations.
pub(super) fn require_active_if_present(conn: &Connection, id: Option<Uuid>) -> Result<()> {
    if let Some(id) = id {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM conversations WHERE id=?1)",
            [id.to_string()],
            |row| row.get(0),
        )?;
        if exists {
            require_active(conn, id)?;
        }
    }
    Ok(())
}
pub(super) fn insert_active(conn: &Connection, id: Uuid) -> Result<()> {
    conn.execute(
        "INSERT INTO conversation_lifecycle(conversation_id,version,state) VALUES(?1,1,'active')",
        [id.to_string()],
    )?;
    Ok(())
}
fn busy(conn: &Connection, id: Uuid) -> Result<bool> {
    let running: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM messages WHERE conversation_id=?1 AND status='running')",
        [id.to_string()],
        |row| row.get(0),
    )?;
    if running {
        return Ok(true);
    }
    let ids = conn
        .prepare("SELECT id FROM proposal_rewrites")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for job in ids {
        let job = proposal_rewrite::read_job(conn, parse_id(job)?)?
            .ok_or_else(|| invalid("missing Rewrite job"))?;
        if job.status == proposal_rewrite::RewriteStatus::Running {
            let proposal = proposals::read_proposal(conn, job.spec.expected.id)?
                .ok_or_else(|| invalid("running Rewrite proposal is missing"))?;
            if proposal.record.draft.session_id == Some(id) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn replay(
    conn: &Connection,
    request: &ConversationLifecycleRequest,
) -> Result<Option<ConversationLifecycleResult>> {
    request.validate()?;
    check_all(conn)?;
    receipt(conn, request.operation_id)?
        .map(|receipt| {
            if receipt.request != *request {
                return Err(Error::OperationConflict(
                    "conversation lifecycle UUID has a different request".into(),
                ));
            }
            let result = ConversationLifecycleResult {
                current: read(conn, request.expected.id)?,
                receipt,
            };
            result.validate()?;
            Ok(result)
        })
        .transpose()
}
fn set(
    conn: &mut Connection,
    request: &ConversationLifecycleRequest,
) -> Result<ConversationLifecycleResult> {
    request.validate()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(result) = replay(&tx, request)? {
        tx.commit()?;
        return Ok(result);
    }
    let before = read(&tx, request.expected.id)?;
    if before.stamp != request.expected {
        return Err(Error::StateChanged(
            "conversation lifecycle version changed".into(),
        ));
    }
    if before.state == request.target {
        return Err(Error::StateChanged(
            "conversation already has the requested state".into(),
        ));
    }
    if busy(&tx, request.expected.id)? {
        return Err(Error::WorkspaceBusy(
            "conversation has running AI work".into(),
        ));
    }
    let after = ConversationLifecycle {
        stamp: ConversationStamp {
            id: before.stamp.id,
            version: before.stamp.version + 1,
        },
        state: request.target,
    };
    let receipt = ConversationLifecycleReceipt {
        request: request.clone(),
        before,
        after,
    };
    receipt.validate()?;
    let bytes =
        serde_json::to_vec(&receipt).map_err(|_| invalid("could not encode lifecycle receipt"))?;
    if bytes.len() > MAX_RECEIPT_BYTES {
        return Err(invalid("lifecycle receipt exceeds its limit"));
    }
    tx.execute(
        "UPDATE conversation_lifecycle SET version=?2,state=?3 WHERE conversation_id=?1",
        params![
            after.stamp.id.to_string(),
            after.stamp.version as i64,
            after.state.as_str()
        ],
    )?;
    tx.execute("INSERT INTO conversation_lifecycle_operations(operation_id,conversation_id,after_version,receipt_json,receipt_sha256) VALUES(?1,?2,?3,?4,?5)",params![request.operation_id.to_string(),after.stamp.id.to_string(),after.stamp.version as i64,bytes,hash(&bytes).as_slice()])?;
    tx.commit()?;
    Ok(ConversationLifecycleResult {
        receipt,
        current: after,
    })
}
fn summaries(conn: &Connection, filter: ConversationFilter) -> Result<Vec<ConversationSummary>> {
    check_all(conn)?;
    chat::conversations(conn)?
        .into_iter()
        .filter_map(|conversation| {
            let lifecycle = match read(conn, conversation.id) {
                Ok(value) => value,
                Err(error) => return Some(Err(error)),
            };
            if !matches!(
                (filter, lifecycle.state),
                (ConversationFilter::All, _)
                    | (ConversationFilter::Active, ConversationState::Active)
                    | (ConversationFilter::Archived, ConversationState::Archived)
            ) {
                return None;
            }
            Some(Ok(ConversationSummary {
                conversation,
                lifecycle,
            }))
        })
        .collect()
}
macro_rules! methods {
    () => {
        pub fn conversation_lifecycle(&self, id: Uuid) -> Result<ConversationLifecycle> {
            let tx = self.conn.unchecked_transaction()?;
            check_all(&tx)?;
            let value = read(&tx, id)?;
            tx.commit()?;
            Ok(value)
        }
        pub fn conversation_summaries(
            &self,
            filter: ConversationFilter,
        ) -> Result<Vec<ConversationSummary>> {
            let tx = self.conn.unchecked_transaction()?;
            let values = summaries(&tx, filter)?;
            tx.commit()?;
            Ok(values)
        }
        /// Exact read-only replay, suitable before a workflow live-job busy gate.
        pub fn conversation_lifecycle_replay(
            &self,
            request: &ConversationLifecycleRequest,
        ) -> Result<Option<ConversationLifecycleResult>> {
            let tx = self.conn.unchecked_transaction()?;
            let value = replay(&tx, request)?;
            tx.commit()?;
            Ok(value)
        }
        pub fn set_conversation_lifecycle(
            &mut self,
            request: &ConversationLifecycleRequest,
        ) -> Result<ConversationLifecycleResult> {
            set(&mut self.conn, request)
        }
    };
}
impl WorkStore {
    methods!();
}
impl chat::ChatStore {
    methods!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_database_rolls_back_transition_metadata_if_receipt_cannot_be_inserted() {
        let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        let turn = store
            .begin_turn(Uuid::new_v4(), None, "original", "chatgpt", "model")
            .unwrap();
        let turn = store
            .finish_turn(
                turn.id,
                super::super::WorkTurnStatus::Completed,
                "kept",
                None,
            )
            .unwrap();
        let pages: i64 = store
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .unwrap();
        store
            .conn
            .pragma_update(None, "max_page_count", pages)
            .unwrap();
        let mut failed = false;
        for _ in 0..128 {
            let before = store.conversation_lifecycle(turn.conversation_id).unwrap();
            let target = if before.state == ConversationState::Active {
                ConversationState::Archived
            } else {
                ConversationState::Active
            };
            let request = ConversationLifecycleRequest {
                operation_id: Uuid::new_v4(),
                expected: before.stamp,
                target,
            };
            if store.set_conversation_lifecycle(&request).is_err() {
                assert_eq!(
                    store.conversation_lifecycle(turn.conversation_id).unwrap(),
                    before
                );
                assert!(
                    store
                        .conversation_lifecycle_replay(&request)
                        .unwrap()
                        .is_none()
                );
                failed = true;
                break;
            }
        }
        assert!(
            failed,
            "bounded lifecycle receipt growth must hit the page ceiling"
        );
        assert_eq!(
            serde_json::to_vec(&store.turn(turn.id).unwrap().unwrap()).unwrap(),
            serde_json::to_vec(&turn).unwrap()
        );
    }
}
