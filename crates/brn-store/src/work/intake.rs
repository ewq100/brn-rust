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

/// Discovery refuses larger inventories instead of truncating saved history.
pub const MAX_DISCOVERY_SCAN_ROWS: usize = 4096;
pub const MAX_DISCOVERY_SCAN_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_DISCOVERY_VERSIONS: usize = 64;
pub const MAX_DISCOVERY_RESULT_BYTES: usize = 64 * 1024 * 1024;

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
    batch_id: Option<String>,
    index: i64,
    bytes: Option<Vec<u8>>,
    digest: Option<Vec<u8>>,
}

pub(super) fn read(conn: &Connection, id: Uuid) -> Result<Option<IntakeSnapshot>> {
    nonnil(id)?;
    let row = conn
        .query_row(
            "SELECT CASE WHEN typeof(batch_id)='text' AND length(CAST(batch_id AS BLOB))=36 THEN batch_id END,item_index,CASE WHEN typeof(record_json)='blob' AND length(record_json)<=?2 THEN record_json END,CASE WHEN typeof(record_sha256)='blob' AND length(record_sha256)=32 THEN record_sha256 END FROM intake_snapshots WHERE id=?1",
            params![id.to_string(), MAX_RECORD_BYTES as i64],
            |r| Ok(Row { batch_id: r.get(0)?, index: r.get(1)?, bytes: r.get(2)?, digest: r.get(3)? }),
        )
        .optional()?;
    row.map(|row| {
        let batch_id = row
            .batch_id
            .ok_or_else(|| invalid("invalid intake snapshot batch row identity"))?;
        let bytes = row
            .bytes
            .ok_or_else(|| invalid("stored intake snapshot exceeds its encoded size limit"))?;
        if row.digest.as_deref() != Some(hash(&bytes).as_slice()) {
            return Err(invalid("stored intake snapshot failed its hash check"));
        }
        let snapshot: IntakeSnapshot = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("malformed stored intake snapshot"))?;
        snapshot.validate()?;
        if snapshot.id != id
            || snapshot.batch_id.to_string() != batch_id
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

fn discovery_inventory(
    conn: &Connection,
    max_rows: usize,
    max_bytes: usize,
) -> Result<Vec<(Uuid, usize)>> {
    let mut statement = conn.prepare(
        "SELECT CASE WHEN typeof(id)='text' AND length(CAST(id AS BLOB))=36 THEN id END,typeof(record_json),length(CAST(record_json AS BLOB)) FROM intake_snapshots ORDER BY id LIMIT ?1",
    )?;
    let mut rows = statement.query([(max_rows + 1) as i64])?;
    let mut inventory = Vec::new();
    let mut scanned_bytes = 0usize;
    while let Some(row) = rows.next()? {
        // Check the row limit before even materializing the extra row's metadata.
        if inventory.len() >= max_rows {
            return Err(invalid(
                "retained extraction discovery scan row quota exceeded",
            ));
        }
        let raw: Option<String> = row.get(0)?;
        let raw = raw.ok_or_else(|| invalid("invalid intake snapshot row identity"))?;
        let id = crate::parse_id(raw.clone())?;
        if raw != id.to_string() || id.is_nil() {
            return Err(invalid("invalid intake snapshot row identity"));
        }
        let kind: String = row.get(1)?;
        let encoded_len = usize::try_from(row.get::<_, i64>(2)?)
            .map_err(|_| invalid("invalid intake snapshot encoded length"))?;
        if kind != "blob" || encoded_len > MAX_RECORD_BYTES {
            return Err(invalid(
                "stored intake snapshot exceeds its encoded size limit or type",
            ));
        }
        scanned_bytes = scanned_bytes
            .checked_add(encoded_len)
            .ok_or_else(|| invalid("retained extraction discovery scan byte quota exceeded"))?;
        if scanned_bytes > max_bytes {
            return Err(invalid(
                "retained extraction discovery scan byte quota exceeded",
            ));
        }
        inventory.push((id, encoded_len));
    }
    Ok(inventory)
}

fn next_result_bytes(versions: usize, total: usize, encoded_len: usize) -> Result<usize> {
    let total = total
        .checked_add(encoded_len)
        .ok_or_else(|| invalid("retained extraction result byte quota exceeded"))?;
    if versions >= MAX_DISCOVERY_VERSIONS || total > MAX_DISCOVERY_RESULT_BYTES {
        return Err(invalid("retained extraction result quota exceeded"));
    }
    Ok(total)
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
    /// Discover every checked saved version for one exact retained catalog item.
    /// Equal bytes from other capture UUIDs remain distinct. Ordering is by
    /// batch UUID, slot, then snapshot UUID; it conveys no current-version or
    /// approval preference. Missing catalog proof refuses this query; historical
    /// UUID reads remain available independently through `intake_snapshot`.
    ///
    /// Refuses inventories above 4096 rows or 256 MiB encoded bytes, and results
    /// above 64 versions or 64 MiB encoded bytes. Metadata is bounded before any
    /// snapshot JSON decoding, and every scanned record must pass integrity.
    pub fn intake_snapshots_for_item(&self, item_id: Uuid) -> Result<Vec<IntakeSnapshot>> {
        if item_id.is_nil() {
            return Err(invalid("retained Inbox item UUID must not be nil"));
        }
        let tx = self.conn.unchecked_transaction()?;
        check_schema(&tx)?;
        let original = super::inbox::read(&tx, item_id)?
            .ok_or_else(|| Error::NotFound("retained Inbox item is unavailable".into()))?;
        let inventory =
            discovery_inventory(&tx, MAX_DISCOVERY_SCAN_ROWS, MAX_DISCOVERY_SCAN_BYTES)?;
        let mut snapshots = Vec::new();
        let mut result_bytes = 0usize;
        for (id, encoded_len) in inventory {
            let snapshot = read(&tx, id)?
                .ok_or_else(|| invalid("missing intake snapshot during discovery"))?;
            if snapshot.original.capture.id != item_id {
                continue;
            }
            if snapshot.original != original {
                return Err(invalid(
                    "saved extraction differs from its exact retained Inbox item",
                ));
            }
            result_bytes = next_result_bytes(snapshots.len(), result_bytes, encoded_len)?;
            snapshots.push(snapshot);
        }
        snapshots.sort_by_key(|snapshot| (snapshot.batch_id, snapshot.index, snapshot.id));
        tx.commit()?;
        Ok(snapshots)
    }

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

#[cfg(test)]
mod discovery_tests {
    use super::*;

    #[test]
    fn metadata_byte_quota_precedes_record_decode_without_allocating_large_records() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(V16).unwrap();
        for value in 1..=2 {
            let id = Uuid::from_u128(value).to_string();
            conn.execute(
                "INSERT INTO intake_snapshots VALUES(?1,?1,0,?2,?3)",
                params![id, b"malformed".as_slice(), [0u8; 32].as_slice()],
            )
            .unwrap();
        }
        assert_eq!(discovery_inventory(&conn, 2, 18).unwrap().len(), 2);
        assert!(
            matches!(discovery_inventory(&conn, 2, 17), Err(Error::Invalid(reason)) if reason.contains("scan byte quota"))
        );
        assert!(
            matches!(discovery_inventory(&conn, 1, 18), Err(Error::Invalid(reason)) if reason.contains("scan row quota"))
        );
    }

    #[test]
    fn result_byte_and_version_limits_refuse_before_retaining_an_extra_version() {
        assert_eq!(
            next_result_bytes(63, MAX_DISCOVERY_RESULT_BYTES - 1, 1).unwrap(),
            MAX_DISCOVERY_RESULT_BYTES
        );
        assert!(next_result_bytes(64, 0, 1).is_err());
        assert!(next_result_bytes(0, MAX_DISCOVERY_RESULT_BYTES, 1).is_err());
        assert!(next_result_bytes(0, usize::MAX, 1).is_err());
    }
}
