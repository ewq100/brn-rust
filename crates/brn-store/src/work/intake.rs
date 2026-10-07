//! Immutable, self-contained private extraction evidence. Complete manifests
//! and opaque source/image bytes share one hashed SQLite record, so a committed
//! snapshot and its backups cannot refer to missing external blobs.
use super::{WorkStore, inbox::InboxItem, inbox_processing::MAX_PROCESS_BATCH};
use crate::{Error, Result, hash, invalid};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) const V16: &str = "
CREATE TABLE intake_snapshots (
 id TEXT PRIMARY KEY,
 batch_id TEXT NOT NULL,
 item_index INTEGER NOT NULL CHECK(item_index>=0 AND item_index<8),
 record_json BLOB NOT NULL,
 record_sha256 BLOB NOT NULL CHECK(length(record_sha256)=32),
 UNIQUE(batch_id,item_index)
);";
// The helper's complete serialized extraction limit plus bounded capture/path,
// UUID and index metadata. Opaque bytes are already included in helper output.
const MAX_RECORD_BYTES: usize = brn_intake::MAX_OUTPUT_BYTES + 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeSnapshot {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub index: usize,
    pub original: InboxItem,
    pub extraction: brn_intake::Extraction,
}

impl IntakeSnapshot {
    /// Pure immutable-evidence validation. It does not inspect current files or
    /// require the operational queue/catalog to survive for historical reading.
    pub fn validate(&self) -> Result<()> {
        validate_slot(self.batch_id, self.index)?;
        nonnil(self.id)?;
        self.original.validate()?;
        self.extraction
            .validate()
            .map_err(|reason| Error::Invalid(format!("invalid intake extraction: {reason}")))?;
        let mut roots = self
            .extraction
            .sources
            .iter()
            .filter(|node| node.parent.is_none());
        let root = roots
            .next()
            .ok_or_else(|| invalid("intake extraction has no retained original root"))?;
        if roots.next().is_some()
            || self.extraction.original_sha256 != self.original.capture.copy.sha256
            || root.bytes.len() as u64 != self.original.capture.copy.byte_len
            || hash(&root.bytes) != self.original.capture.copy.sha256
        {
            return Err(invalid(
                "intake extraction differs from its exact original bytes",
            ));
        }
        if encode(self)?.len() > MAX_RECORD_BYTES {
            return Err(invalid("intake snapshot exceeds its encoded size limit"));
        }
        Ok(())
    }

    /// Hash of the complete canonical manifest, including every retained byte.
    pub fn digest(&self) -> Result<[u8; 32]> {
        self.validate()?;
        Ok(hash(&encode(self)?))
    }
}

fn nonnil(id: Uuid) -> Result<()> {
    if id.is_nil() {
        return Err(invalid("intake snapshot and batch UUIDs must not be nil"));
    }
    Ok(())
}

fn validate_slot(batch_id: Uuid, index: usize) -> Result<()> {
    nonnil(batch_id)?;
    if index >= MAX_PROCESS_BATCH {
        return Err(invalid("intake snapshot batch index is out of range"));
    }
    Ok(())
}

fn encode(snapshot: &IntakeSnapshot) -> Result<Vec<u8>> {
    serde_json::to_vec(snapshot).map_err(|_| invalid("could not encode intake snapshot"))
}

fn normalized(sql: &str) -> String {
    sql.trim()
        .trim_end_matches(';')
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect()
}

fn check_schema(conn: &Connection) -> Result<()> {
    let actual: Option<(String, String, String)> = conn
        .query_row(
            "SELECT type,tbl_name,sql FROM sqlite_schema WHERE name='intake_snapshots'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if !actual.is_some_and(|(kind, table, sql)| {
        kind == "table" && table == "intake_snapshots" && normalized(&sql) == normalized(V16)
    }) {
        return Err(invalid(
            "intake snapshot schema differs from its owned shape",
        ));
    }
    let objects: Vec<(String, String)> = conn
        .prepare(
            "SELECT type,name FROM sqlite_schema WHERE tbl_name='intake_snapshots' ORDER BY name",
        )?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if objects
        != [
            ("table".into(), "intake_snapshots".into()),
            ("index".into(), "sqlite_autoindex_intake_snapshots_1".into()),
            ("index".into(), "sqlite_autoindex_intake_snapshots_2".into()),
        ]
    {
        return Err(invalid(
            "intake snapshots have unexpected or missing schema objects",
        ));
    }
    Ok(())
}

struct Row {
    batch_id: String,
    index: i64,
    bytes: Option<Vec<u8>>,
    digest: Vec<u8>,
}

pub(super) fn read(conn: &Connection, id: Uuid) -> Result<Option<IntakeSnapshot>> {
    nonnil(id)?;
    let row = conn
        .query_row(
            "SELECT batch_id,item_index,CASE WHEN length(record_json)<=?2 THEN record_json END,record_sha256 FROM intake_snapshots WHERE id=?1",
            params![id.to_string(), MAX_RECORD_BYTES as i64],
            |r| Ok(Row { batch_id: r.get(0)?, index: r.get(1)?, bytes: r.get(2)?, digest: r.get(3)? }),
        )
        .optional()?;
    row.map(|row| {
        let bytes = row
            .bytes
            .ok_or_else(|| invalid("stored intake snapshot exceeds its encoded size limit"))?;
        if row.digest.as_slice() != hash(&bytes) {
            return Err(invalid("stored intake snapshot failed its hash check"));
        }
        let snapshot: IntakeSnapshot = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("malformed stored intake snapshot"))?;
        snapshot.validate()?;
        if snapshot.id != id
            || snapshot.batch_id.to_string() != row.batch_id
            || i64::try_from(snapshot.index).ok() != Some(row.index)
            || encode(&snapshot)? != bytes
        {
            return Err(invalid(
                "stored intake snapshot differs from its canonical record or row binding",
            ));
        }
        Ok(snapshot)
    })
    .transpose()
}

fn read_for(conn: &Connection, batch_id: Uuid, index: usize) -> Result<Option<IntakeSnapshot>> {
    validate_slot(batch_id, index)?;
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM intake_snapshots WHERE batch_id=?1 AND item_index=?2",
            params![batch_id.to_string(), index as i64],
            |r| r.get(0),
        )
        .optional()?;
    id.map(|raw| {
        let id = crate::parse_id(raw.clone())?;
        if raw != id.to_string() {
            return Err(invalid("invalid intake snapshot row identity"));
        }
        let snapshot = read(conn, id)?.ok_or_else(|| invalid("missing intake snapshot row"))?;
        if snapshot.batch_id != batch_id || snapshot.index != index {
            return Err(invalid("intake snapshot slot differs from its record"));
        }
        Ok(snapshot)
    })
    .transpose()
}

/// Domain refusal precedes physical-corruption recovery and startup backup.
pub(super) fn check_all(conn: &Connection) -> Result<()> {
    check_schema(conn)?;
    for raw in conn
        .prepare("SELECT id FROM intake_snapshots ORDER BY id")?
        .query_map([], |r| r.get::<_, String>(0))?
    {
        let raw = raw?;
        let id = crate::parse_id(raw.clone())?;
        if raw != id.to_string() || read(conn, id)?.is_none() {
            return Err(invalid("invalid intake snapshot row identity"));
        }
    }
    Ok(())
}

impl WorkStore {
    /// Atomically retain complete extraction evidence for an exact queued item.
    /// UUID and batch-slot replay are immutable. A distinct batch may retain a
    /// later extraction; existing evidence is never replaced or re-converted.
    pub fn save_intake_snapshot(&mut self, snapshot: &IntakeSnapshot) -> Result<IntakeSnapshot> {
        self.retain_intake_snapshot(snapshot, true)
    }

    /// Restore an already-qualified complete private mirror. This restores no
    /// queue state and never updates an existing UUID or batch-slot binding.
    pub fn restore_intake_snapshot(&mut self, snapshot: &IntakeSnapshot) -> Result<IntakeSnapshot> {
        self.retain_intake_snapshot(snapshot, false)
    }

    fn retain_intake_snapshot(
        &mut self,
        snapshot: &IntakeSnapshot,
        require_batch: bool,
    ) -> Result<IntakeSnapshot> {
        snapshot.validate()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_schema(&tx)?;
        let existing_by_id = read(&tx, snapshot.id)?;
        let existing_by_slot = read_for(&tx, snapshot.batch_id, snapshot.index)?;
        if let Some(existing) = existing_by_id.or(existing_by_slot) {
            if existing != *snapshot {
                return Err(Error::OperationConflict(
                    "intake snapshot UUID or batch slot has another extraction".into(),
                ));
            }
            tx.commit()?;
            return Ok(existing);
        }
        if require_batch {
            let batch =
                super::inbox_processing::read(&tx, snapshot.batch_id)?.ok_or_else(|| {
                    Error::NotFound("intake snapshot processing batch does not exist".into())
                })?;
            if batch.request.items.get(snapshot.index) != Some(&snapshot.original) {
                return Err(Error::OperationConflict(
                    "intake snapshot needs the exact processing item".into(),
                ));
            }
        }
        let bytes = encode(snapshot)?;
        tx.execute(
            "INSERT INTO intake_snapshots(id,batch_id,item_index,record_json,record_sha256) VALUES(?1,?2,?3,?4,?5)",
            params![snapshot.id.to_string(), snapshot.batch_id.to_string(), snapshot.index as i64, bytes, hash(&bytes).as_slice()],
        )?;
        tx.commit()?;
        Ok(snapshot.clone())
    }

    /// Read checked historical evidence without touching current original files.
    pub fn intake_snapshot(&self, id: Uuid) -> Result<Option<IntakeSnapshot>> {
        let tx = self.conn.unchecked_transaction()?;
        check_schema(&tx)?;
        let snapshot = read(&tx, id)?;
        tx.commit()?;
        Ok(snapshot)
    }

    pub fn intake_snapshot_for(
        &self,
        batch_id: Uuid,
        index: usize,
    ) -> Result<Option<IntakeSnapshot>> {
        let tx = self.conn.unchecked_transaction()?;
        check_schema(&tx)?;
        let snapshot = read_for(&tx, batch_id, index)?;
        tx.commit()?;
        Ok(snapshot)
    }
}
