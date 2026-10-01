//! Durable editing recovery and filesystem-write journals. No filesystem I/O occurs here.
use super::*;
use rusqlite::Transaction;
use serde::de::DeserializeOwned;
use std::path::Component;

pub const MAX_NOTE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteStamp {
    pub file_state: Uuid,
    pub generation: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteSubmission {
    pub operation_id: Uuid,
    pub note_id: Uuid,
    pub expected: NoteStamp,
    pub generation: u64,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteBufferReceipt {
    pub operation_id: Uuid,
    pub note_id: Uuid,
    pub stamp: NoteStamp,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteAvailability {
    Available,
    Missing,
    Conflict,
    Unsupported,
    Uncertain,
    Unavailable,
    OwnedElsewhere,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteView {
    pub id: Uuid,
    pub vault_id: Uuid,
    pub relative_path: PathBuf,
    pub stamp: NoteStamp,
    pub current_file_state: Option<Uuid>,
    pub saved: Option<String>,
    pub buffer: String,
    pub availability: NoteAvailability,
    pub availability_message: Option<String>,
    pub search_approval: Approval,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavePhase {
    Intent,
    Prepared,
    Exchanged,
    Verified,
    Complete,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactCleanup {
    Pending,
    Retired,
    RetainedUnexpected,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOutcome {
    NotApplied,
    Applied,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteResolution {
    Unresolved,
    NotApplied,
    Applied,
    AcceptedCurrent,
}
impl NoteResolution {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unresolved => "unresolved",
            Self::NotApplied => "not_applied",
            Self::Applied => "applied",
            Self::AcceptedCurrent => "accepted_current",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteReceipt {
    pub operation_id: Uuid,
    pub source_note_id: Uuid,
    pub note_id: Uuid,
    pub submitted_generation: u64,
    pub stamp: NoteStamp,
    pub filesystem_outcome: FileOutcome,
    pub recovery_available: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteFailure {
    pub code: NoteErrorCode,
    pub message: String,
    pub operation_id: Option<Uuid>,
    pub note_id: Option<Uuid>,
    pub phase: Option<SavePhase>,
    pub filesystem_outcome: FileOutcome,
    pub recovery_available: bool,
}
impl std::fmt::Display for NoteFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for NoteFailure {}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteRecordedResult {
    Receipt(NoteReceipt),
    Failure(NoteFailure),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteErrorCode {
    StateChanged,
    Conflict,
    Missing,
    Unsupported,
    SaveUncertain,
    Io,
    OperationConflict,
    WorkspaceBusy,
    VaultBusy,
    VaultUnavailable,
    Storage,
}
pub type NoteResult<T> = std::result::Result<T, NoteFailure>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteSearchReceipt {
    pub operation_id: Uuid,
    pub note_id: Uuid,
    pub file_state: Uuid,
    pub source_id: Uuid,
    pub version_id: Uuid,
}

/// Caller-visible identity, checked before filesystem access on replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteSearchRequest {
    Approve {
        note_id: Uuid,
        file_state: Uuid,
    },
    Import {
        note_id: Uuid,
        path: PathBuf,
        approval: Approval,
    },
    SetApproval {
        note_id: Uuid,
        source_id: Uuid,
        version_id: Uuid,
        approval: Approval,
    },
}
impl NoteSearchRequest {
    pub fn note_id(&self) -> Uuid {
        match self {
            Self::Approve { note_id, .. }
            | Self::Import { note_id, .. }
            | Self::SetApproval { note_id, .. } => *note_id,
        }
    }
    pub fn approval(&self) -> Approval {
        match self {
            Self::Approve { .. } => Approval::Approved,
            Self::Import { approval, .. } | Self::SetApproval { approval, .. } => *approval,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteSearchSnapshot {
    pub receipt: NoteSearchReceipt,
    pub fingerprint: FileFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFingerprint {
    pub device: u64,
    pub inode: u64,
    pub len: u64,
    pub sha256: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultIdentity {
    pub device: u64,
    pub inode: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultRecord {
    pub id: Uuid,
    pub root: PathBuf,
    pub identity: VaultIdentity,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteRecord {
    pub id: Uuid,
    pub vault_id: Uuid,
    pub relative_path: PathBuf,
    pub baseline: FileFingerprint,
    pub stamp: NoteStamp,
    pub observed: Option<(Uuid, FileFingerprint)>,
    pub search_approval: Approval,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteRecovery {
    pub note_id: Uuid,
    pub stamp: NoteStamp,
    pub baseline: String,
    pub working: String,
    pub pending_operations: Vec<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteComparison {
    pub note_id: Uuid,
    pub stamp: NoteStamp,
    pub baseline: String,
    pub working: String,
    pub observed: Option<String>,
    pub observed_file_state: Option<Uuid>,
    pub availability: NoteAvailability,
}

/// Caller-visible inputs only: observations are revalidated, not bound into replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteDecision {
    Reload {
        note_id: Uuid,
        expected: NoteStamp,
        discard: bool,
    },
    Relink {
        note_id: Uuid,
        expected: NoteStamp,
        relative: PathBuf,
        confirm_identity: bool,
    },
    AcceptCurrent {
        save_operation_id: Uuid,
        observed_file_state: Uuid,
    },
}

impl NoteDecision {
    fn operation_kind(&self) -> &'static str {
        match self {
            Self::Reload { .. } => "note.reload",
            Self::Relink { .. } => "note.relink",
            Self::AcceptCurrent { .. } => "note.accept-current",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedFile {
    pub relative: PathBuf,
    pub fingerprint: FileFingerprint,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactIdentity {
    pub device: u64,
    pub inode: u64,
    pub len: u64,
    pub kind: ArtifactKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactKind {
    Regular,
    Directory,
    Symlink,
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedArtifact {
    pub relative: PathBuf,
    pub identity: ArtifactIdentity,
    pub sha256: Option<[u8; 32]>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestinationPrecondition {
    Existing {
        fingerprint: FileFingerprint,
        baseline_text: String,
    },
    Absent {
        parent: VaultIdentity,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteWriteKind {
    Replace,
    Copy,
}
impl NoteWriteKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Copy => "copy",
        }
    }
    fn operation_kind(self) -> &'static str {
        match self {
            Self::Replace => "note.save",
            Self::Copy => "note.copy",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteSaveIntent {
    pub request: NoteSubmission,
    pub target_note_id: Uuid,
    pub destination: PathBuf,
    pub staging_relative: PathBuf,
    pub kind: NoteWriteKind,
    pub expected_destination: DestinationPrecondition,
    pub staged: Option<PreparedFile>,
    pub displaced: Option<RetainedArtifact>,
    pub phase: SavePhase,
    pub resolution: NoteResolution,
    pub acknowledged_by: Option<Uuid>,
    pub cleanup: ArtifactCleanup,
    pub prior_result: Option<NoteRecordedResult>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteVerification {
    Replace {
        installed: FileFingerprint,
        displaced: RetainedArtifact,
        displaced_bytes: Vec<u8>,
    },
    Copy {
        installed: FileFingerprint,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteReconciliation {
    pub resolution: NoteResolution,
    pub observed_destination: Option<FileFingerprint>,
    pub verification: Option<NoteVerification>,
    pub result: NoteRecordedResult,
}

pub(super) const V6: &str = "
ALTER TABLE chat_turns ADD COLUMN evidence_currentness TEXT NOT NULL DEFAULT 'unqualified' CHECK(evidence_currentness IN ('unqualified','current_at_completion','stale_at_completion'));
CREATE TABLE note_vaults (id TEXT PRIMARY KEY, singleton INTEGER NOT NULL UNIQUE CHECK(singleton=1), record_json BLOB NOT NULL, record_sha256 BLOB NOT NULL CHECK(length(record_sha256)=32));
CREATE TABLE notes (id TEXT PRIMARY KEY, vault_id TEXT NOT NULL REFERENCES note_vaults(id), relative TEXT NOT NULL, file_state TEXT NOT NULL, fingerprint_json BLOB NOT NULL, fingerprint_sha256 BLOB NOT NULL CHECK(length(fingerprint_sha256)=32), observed_file_state TEXT, observed_fingerprint_json BLOB, observed_fingerprint_sha256 BLOB CHECK(observed_fingerprint_sha256 IS NULL OR length(observed_fingerprint_sha256)=32), approval TEXT NOT NULL DEFAULT 'draft' CHECK(approval IN ('approved','draft','withdrawn')), CHECK((observed_file_state IS NULL AND observed_fingerprint_json IS NULL AND observed_fingerprint_sha256 IS NULL) OR (observed_file_state IS NOT NULL AND observed_fingerprint_json IS NOT NULL AND observed_fingerprint_sha256 IS NOT NULL)), UNIQUE(vault_id,relative));
CREATE TABLE note_buffers (note_id TEXT PRIMARY KEY REFERENCES notes(id), file_state TEXT NOT NULL, generation INTEGER NOT NULL CHECK(generation>=0), baseline BLOB NOT NULL, baseline_sha256 BLOB NOT NULL CHECK(length(baseline_sha256)=32), working BLOB NOT NULL, working_sha256 BLOB NOT NULL CHECK(length(working_sha256)=32));
CREATE TABLE note_results (operation_id TEXT PRIMARY KEY REFERENCES operations(id), result_json BLOB NOT NULL, result_sha256 BLOB NOT NULL CHECK(length(result_sha256)=32));
CREATE TABLE note_write_inputs (operation_id TEXT PRIMARY KEY REFERENCES operations(id), note_id TEXT NOT NULL REFERENCES notes(id), request_json BLOB NOT NULL, request_sha256 BLOB NOT NULL CHECK(length(request_sha256)=32));
CREATE TABLE note_save_intents (operation_id TEXT PRIMARY KEY REFERENCES operations(id), source_note_id TEXT NOT NULL REFERENCES notes(id), target_note_id TEXT NOT NULL, vault_id TEXT NOT NULL REFERENCES note_vaults(id), destination TEXT NOT NULL, write_kind TEXT NOT NULL CHECK(write_kind IN ('replace','copy')), phase TEXT NOT NULL CHECK(phase IN ('intent','prepared','exchanged','verified','complete')), resolution TEXT NOT NULL CHECK(resolution IN ('unresolved','not_applied','applied','accepted_current')), cleanup TEXT NOT NULL CHECK(cleanup IN ('pending','retired','retained_unexpected')), intent_json BLOB NOT NULL, intent_sha256 BLOB NOT NULL CHECK(length(intent_sha256)=32), verification_json BLOB, verification_sha256 BLOB CHECK(verification_sha256 IS NULL OR length(verification_sha256)=32));
CREATE UNIQUE INDEX one_unresolved_original_save ON note_save_intents(source_note_id) WHERE write_kind = 'replace' AND resolution = 'unresolved';
CREATE UNIQUE INDEX one_reserved_copy_destination ON note_save_intents(vault_id,destination) WHERE write_kind = 'copy' AND resolution IN ('unresolved','applied','accepted_current');
CREATE TABLE note_recovery_pairs (note_id TEXT PRIMARY KEY REFERENCES notes(id), operation_id TEXT NOT NULL REFERENCES operations(id), baseline BLOB NOT NULL, baseline_sha256 BLOB NOT NULL CHECK(length(baseline_sha256)=32), submitted BLOB NOT NULL, submitted_sha256 BLOB NOT NULL CHECK(length(submitted_sha256)=32));
CREATE TABLE note_shadowed_sources (note_id TEXT NOT NULL REFERENCES notes(id), source_id TEXT NOT NULL REFERENCES sources(id), PRIMARY KEY(note_id,source_id));
CREATE TABLE note_receipts (operation_id TEXT PRIMARY KEY REFERENCES operations(id), note_id TEXT NOT NULL, result_json BLOB NOT NULL, result_sha256 BLOB NOT NULL CHECK(length(result_sha256)=32));
CREATE TABLE note_write_destinations (operation_id TEXT PRIMARY KEY REFERENCES operations(id), destination_json BLOB NOT NULL, destination_sha256 BLOB NOT NULL CHECK(length(destination_sha256)=32));
CREATE TABLE note_decision_recoveries (operation_id TEXT PRIMARY KEY REFERENCES operations(id), recovery_json BLOB NOT NULL, recovery_sha256 BLOB NOT NULL CHECK(length(recovery_sha256)=32));
ALTER TABLE notes ADD COLUMN content_epoch INTEGER NOT NULL DEFAULT 0 CHECK(typeof(content_epoch)='integer' AND content_epoch>=0);
ALTER TABLE notes ADD COLUMN approval_epoch INTEGER NOT NULL DEFAULT 0 CHECK(typeof(approval_epoch)='integer' AND approval_epoch>=0);
CREATE TABLE note_search_snapshots (note_id TEXT PRIMARY KEY REFERENCES notes(id), source_id TEXT NOT NULL UNIQUE REFERENCES sources(id), snapshot_json BLOB NOT NULL, snapshot_sha256 BLOB NOT NULL CHECK(length(snapshot_sha256)=32));
CREATE TABLE note_search_results (operation_id TEXT PRIMARY KEY REFERENCES operations(id), result_json BLOB NOT NULL, result_sha256 BLOB NOT NULL CHECK(length(result_sha256)=32));
";

fn failure(code: NoteErrorCode, message: impl Into<String>) -> NoteFailure {
    NoteFailure {
        code,
        message: message.into(),
        operation_id: None,
        note_id: None,
        phase: None,
        filesystem_outcome: FileOutcome::NotApplied,
        recovery_available: false,
    }
}
impl From<Error> for NoteFailure {
    fn from(error: Error) -> Self {
        let code = match error {
            Error::OperationConflict(_) => NoteErrorCode::OperationConflict,
            Error::WorkspaceBusy(_) => NoteErrorCode::WorkspaceBusy,
            _ => NoteErrorCode::Storage,
        };
        failure(code, error.to_string())
    }
}
impl From<rusqlite::Error> for NoteFailure {
    fn from(error: rusqlite::Error) -> Self {
        Error::Sql(error).into()
    }
}
fn json<T: Serialize>(value: &T) -> NoteResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| failure(NoteErrorCode::Storage, e.to_string()))
}
fn decode<T: DeserializeOwned>(bytes: Vec<u8>, digest: Vec<u8>) -> NoteResult<T> {
    if digest != hash(&bytes) {
        return Err(failure(
            NoteErrorCode::Storage,
            "stored note payload hash mismatch",
        ));
    }
    serde_json::from_slice(&bytes).map_err(|e| failure(NoteErrorCode::Storage, e.to_string()))
}
fn checked_text(bytes: Vec<u8>, digest: Vec<u8>) -> NoteResult<String> {
    if bytes.len() > MAX_NOTE_BYTES || digest != hash(&bytes) {
        return Err(failure(
            NoteErrorCode::Storage,
            "invalid stored note bytes or hash",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| failure(NoteErrorCode::Storage, "invalid stored note UTF-8"))
}
fn generation(value: u64) -> NoteResult<i64> {
    i64::try_from(value)
        .map_err(|_| failure(NoteErrorCode::StateChanged, "note generation overflow"))
}
fn valid_text(text: &str) -> NoteResult<()> {
    if text.len() > MAX_NOTE_BYTES {
        return Err(failure(NoteErrorCode::Unsupported, "note exceeds 1 MiB"));
    }
    Ok(())
}
fn relative(path: &Path) -> NoteResult<&str> {
    if path.as_os_str().is_empty() || !path.components().all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(failure(
            NoteErrorCode::Unsupported,
            "note path must be confined and relative",
        ));
    }
    path.to_str()
        .ok_or_else(|| failure(NoteErrorCode::Unsupported, "note path is not UTF-8"))
}
fn destination(path: &Path) -> NoteResult<&str> {
    let path_str = relative(path)?;
    if path.extension().and_then(|s| s.to_str()) != Some("md") {
        return Err(failure(
            NoteErrorCode::Unsupported,
            "note destination must end in .md",
        ));
    }
    Ok(path_str)
}
fn matches_text(file: &FileFingerprint, text: &str) -> bool {
    file.len == text.len() as u64 && file.sha256 == hash(text.as_bytes())
}
fn bound(conn: &Connection, op: Uuid, kind: &str, payload: &[u8]) -> NoteResult<bool> {
    let row: Option<(String, Vec<u8>)> = conn
        .query_row(
            "SELECT kind,payload_hash FROM operations WHERE id=?1",
            [op.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match row {
        Some((old_kind, old_hash)) if old_kind == kind && old_hash == hash(payload) => Ok(true),
        Some(_) => Err(failure(
            NoteErrorCode::OperationConflict,
            "operation ID conflicts with note payload",
        )),
        None => Ok(false),
    }
}
fn write_payload(
    request: &NoteSubmission,
    path: &Path,
    kind: NoteWriteKind,
) -> NoteResult<Vec<u8>> {
    json(&(request, path, kind))
}
fn stored_result<T: DeserializeOwned>(
    conn: &Connection,
    table: &str,
    op: Uuid,
) -> NoteResult<Option<T>> {
    let row: Option<(Vec<u8>, Vec<u8>)> = conn
        .query_row(
            &format!("SELECT result_json,result_sha256 FROM {table} WHERE operation_id=?1"),
            [op.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.map(|(b, h)| decode(b, h)).transpose()
}
fn insert_result<T: Serialize>(
    tx: &Transaction<'_>,
    table: &str,
    op: Uuid,
    note: Uuid,
    value: &T,
) -> NoteResult<()> {
    let bytes = json(value)?;
    if table == "note_receipts" {
        tx.execute("INSERT INTO note_receipts(operation_id,note_id,result_json,result_sha256) VALUES(?1,?2,?3,?4)",
            params![op.to_string(),note.to_string(),bytes,hash(&bytes).as_slice()])?;
    } else {
        tx.execute(
            "INSERT INTO note_results(operation_id,result_json,result_sha256) VALUES(?1,?2,?3)",
            params![op.to_string(), bytes, hash(&bytes).as_slice()],
        )?;
    }
    Ok(())
}
fn recovery(conn: &Connection, id: Uuid) -> NoteResult<Option<NoteRecovery>> {
    type Row = (String, i64, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);
    let row: Option<Row> = conn.query_row(
        "SELECT file_state,generation,baseline,baseline_sha256,working,working_sha256 FROM note_buffers WHERE note_id=?1",
        [id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
    ).optional()?;
    let Some((file_state, stored_generation, baseline, bh, working, wh)) = row else {
        return Ok(None);
    };
    let mut stmt = conn.prepare("SELECT operation_id FROM note_save_intents WHERE source_note_id=?1 AND resolution IN ('unresolved','accepted_current') ORDER BY rowid")?;
    let ids = stmt.query_map([id.to_string()], |r| r.get::<_, String>(0))?;
    let pending_operations = ids
        .map(|row| Ok(parse_id(row?)?))
        .collect::<NoteResult<Vec<_>>>()?;
    Ok(Some(NoteRecovery {
        note_id: id,
        stamp: NoteStamp {
            file_state: parse_id(file_state)?,
            generation: u64::try_from(stored_generation)
                .map_err(|_| failure(NoteErrorCode::Storage, "invalid stored generation"))?,
        },
        baseline: checked_text(baseline, bh)?,
        working: checked_text(working, wh)?,
        pending_operations,
    }))
}
fn note_record(conn: &Connection, id: Uuid) -> NoteResult<NoteRecord> {
    type Row = (
        String,
        String,
        String,
        Vec<u8>,
        Vec<u8>,
        Option<String>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        String,
    );
    let row: Option<Row> = conn.query_row(
        "SELECT vault_id,relative,file_state,fingerprint_json,fingerprint_sha256,observed_file_state,observed_fingerprint_json,observed_fingerprint_sha256,approval FROM notes WHERE id=?1",
        [id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?)),
    ).optional()?;
    let (
        vault,
        path,
        state,
        bytes,
        digest,
        observed_state,
        observed_bytes,
        observed_digest,
        approval,
    ) = row.ok_or_else(|| failure(NoteErrorCode::Missing, "note does not exist"))?;
    let recovered = recovery(conn, id)?
        .ok_or_else(|| failure(NoteErrorCode::Storage, "registered note lacks recovery"))?;
    let baseline: FileFingerprint = decode(bytes, digest)?;
    let relative_path = PathBuf::from(path);
    if destination(&relative_path).is_err()
        || parse_id(state)? != recovered.stamp.file_state
        || !matches_text(&baseline, &recovered.baseline)
    {
        return Err(failure(
            NoteErrorCode::Storage,
            "invalid registered note baseline or path",
        ));
    }
    let observed = match (observed_state, observed_bytes, observed_digest) {
        (None, None, None) => None,
        (Some(state), Some(bytes), Some(digest)) => {
            Some((parse_id(state)?, decode(bytes, digest)?))
        }
        _ => {
            return Err(failure(
                NoteErrorCode::Storage,
                "incomplete note observation",
            ));
        }
    };
    Ok(NoteRecord {
        id,
        vault_id: parse_id(vault)?,
        relative_path,
        baseline,
        stamp: recovered.stamp,
        observed,
        search_approval: match approval.as_str() {
            "draft" => Approval::Draft,
            "approved" => Approval::Approved,
            "withdrawn" => Approval::Withdrawn,
            _ => return Err(failure(NoteErrorCode::Storage, "invalid note approval")),
        },
    })
}
fn checked_submission(conn: &Connection, request: &NoteSubmission) -> NoteResult<NoteRecovery> {
    valid_text(&request.text)?;
    generation(request.generation)?;
    generation(request.expected.generation)?;
    let current = recovery(conn, request.note_id)?
        .ok_or_else(|| failure(NoteErrorCode::Missing, "note does not exist"))?;
    if current.stamp != request.expected
        || request.generation < current.stamp.generation
        || request.generation == current.stamp.generation && request.text != current.working
    {
        return Err(failure(
            NoteErrorCode::StateChanged,
            "note buffer stamp or generation changed",
        ));
    }
    Ok(current)
}
fn accept_submission(tx: &Transaction<'_>, request: &NoteSubmission) -> NoteResult<NoteRecovery> {
    let current = checked_submission(tx, request)?;
    tx.execute(
        "UPDATE note_buffers SET generation=?2,working=?3,working_sha256=?4 WHERE note_id=?1",
        params![
            request.note_id.to_string(),
            generation(request.generation)?,
            request.text.as_bytes(),
            hash(request.text.as_bytes()).as_slice()
        ],
    )?;
    Ok(current)
}
fn insert_buffer(
    tx: &Transaction<'_>,
    note: Uuid,
    stamp: NoteStamp,
    baseline: &str,
    working: &str,
) -> NoteResult<()> {
    tx.execute("INSERT INTO note_buffers(note_id,file_state,generation,baseline,baseline_sha256,working,working_sha256) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![note.to_string(),stamp.file_state.to_string(),generation(stamp.generation)?,baseline.as_bytes(),hash(baseline.as_bytes()).as_slice(),working.as_bytes(),hash(working.as_bytes()).as_slice()])?;
    Ok(())
}
fn retain_input(tx: &Transaction<'_>, request: &NoteSubmission, path: &Path) -> NoteResult<()> {
    let bytes = json(request)?;
    tx.execute("INSERT INTO note_write_inputs(operation_id,note_id,request_json,request_sha256) VALUES(?1,?2,?3,?4)",
        params![request.operation_id.to_string(),request.note_id.to_string(),bytes,hash(&bytes).as_slice()])?;
    let bytes = json(&path)?;
    tx.execute("INSERT INTO note_write_destinations(operation_id,destination_json,destination_sha256) VALUES(?1,?2,?3)",
        params![request.operation_id.to_string(),bytes,hash(&bytes).as_slice()])?;
    Ok(())
}
fn phase_str(phase: SavePhase) -> &'static str {
    match phase {
        SavePhase::Intent => "intent",
        SavePhase::Prepared => "prepared",
        SavePhase::Exchanged => "exchanged",
        SavePhase::Verified => "verified",
        SavePhase::Complete => "complete",
    }
}
fn cleanup_str(cleanup: ArtifactCleanup) -> &'static str {
    match cleanup {
        ArtifactCleanup::Pending => "pending",
        ArtifactCleanup::Retired => "retired",
        ArtifactCleanup::RetainedUnexpected => "retained_unexpected",
    }
}
fn intent(conn: &Connection, op: Uuid) -> NoteResult<Option<NoteSaveIntent>> {
    type Row = (
        Vec<u8>,
        Vec<u8>,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    );
    let row: Option<Row> = conn.query_row(
        "SELECT intent_json,intent_sha256,source_note_id,target_note_id,destination,write_kind,phase,resolution,cleanup FROM note_save_intents WHERE operation_id=?1",
        [op.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?)),
    ).optional()?;
    let Some((bytes, digest, source, target, dest, kind, phase, resolution, cleanup)) = row else {
        return Ok(None);
    };
    let mut value: NoteSaveIntent = decode(bytes, digest)?;
    if value.request.operation_id != op
        || value.request.note_id.to_string() != source
        || value.target_note_id.to_string() != target
        || relative(&value.destination)? != dest
        || value.kind.as_str() != kind
        || phase_str(value.phase) != phase
        || value.resolution.as_str() != resolution
        || cleanup_str(value.cleanup) != cleanup
        || !bound(
            conn,
            op,
            value.kind.operation_kind(),
            &write_payload(&value.request, &value.destination, value.kind)?,
        )?
    {
        return Err(failure(
            NoteErrorCode::Storage,
            "note intent does not match bound operation",
        ));
    }
    value.prior_result = stored_result(conn, "note_receipts", op)?;
    Ok(Some(value))
}
fn require_intent(conn: &Connection, op: Uuid) -> NoteResult<NoteSaveIntent> {
    intent(conn, op)?
        .ok_or_else(|| failure(NoteErrorCode::Missing, "note save intent does not exist"))
}
fn update_intent(tx: &Transaction<'_>, value: &NoteSaveIntent) -> NoteResult<()> {
    let mut persisted = value.clone();
    persisted.prior_result = None;
    let bytes = json(&persisted)?;
    tx.execute("UPDATE note_save_intents SET phase=?2,resolution=?3,cleanup=?4,intent_json=?5,intent_sha256=?6 WHERE operation_id=?1",
        params![value.request.operation_id.to_string(),phase_str(value.phase),value.resolution.as_str(),cleanup_str(value.cleanup),bytes,hash(&bytes).as_slice()])?;
    Ok(())
}
fn failure_context(
    mut error: NoteFailure,
    op: Uuid,
    value: Option<&NoteSaveIntent>,
) -> NoteFailure {
    error.operation_id = Some(op);
    if let Some(value) = value {
        error.note_id = Some(value.request.note_id);
        error.phase = Some(value.phase);
        error.filesystem_outcome = match value.phase {
            SavePhase::Intent | SavePhase::Prepared => FileOutcome::NotApplied,
            SavePhase::Exchanged => FileOutcome::Unknown,
            SavePhase::Verified | SavePhase::Complete => match value.resolution {
                NoteResolution::NotApplied => FileOutcome::NotApplied,
                _ => FileOutcome::Applied,
            },
        };
    }
    error
}

fn original_save_blocker(conn: &Connection, id: Uuid) -> NoteResult<Option<NoteFailure>> {
    let blocker: Option<String> = conn.query_row(
        "SELECT operation_id FROM note_save_intents WHERE source_note_id=?1 AND write_kind='replace' AND resolution='unresolved'",
        [id.to_string()], |r| r.get(0),
    ).optional()?;
    let Some(blocker) = blocker else {
        return Ok(None);
    };
    let value = require_intent(conn, parse_id(blocker)?)?;
    let code = if matches!(value.prior_result, Some(NoteRecordedResult::Failure(error)) if error.code == NoteErrorCode::Conflict && error.filesystem_outcome == FileOutcome::NotApplied)
    {
        NoteErrorCode::Conflict
    } else {
        NoteErrorCode::SaveUncertain
    };
    Ok(Some(failure(
        code,
        format!(
            "original-path write {} remains unresolved; compare then explicitly accept-current",
            value.request.operation_id
        ),
    )))
}

impl Store {
    /// Exact registered-path provenance only; immutable legacy identities are not reassigned.
    pub fn reconcile_note_sources(&mut self) -> NoteResult<()> {
        let Some(vault) = self.registered_vault()? else {
            return Ok(());
        };
        let paths: Vec<_> = self
            .note_recoveries()?
            .into_iter()
            .map(|r| {
                let n = self.note_record(r.note_id)?;
                Ok((n.id, vault.root.join(n.relative_path)))
            })
            .collect::<NoteResult<_>>()?;
        let tx = self.conn.transaction()?;
        for (id, path) in paths {
            let origin = path
                .to_str()
                .ok_or_else(|| failure(NoteErrorCode::Storage, "registered path is not UTF-8"))?;
            tx.execute("INSERT OR IGNORE INTO note_shadowed_sources(note_id,source_id) SELECT ?1,id FROM sources WHERE origin=?2", params![id.to_string(), origin])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// (note, source, shadowed). Association ambiguity is never guessed.
    pub fn note_source_associations(&self) -> NoteResult<Vec<(Uuid, Uuid, bool)>> {
        let mut stmt = self.conn.prepare("SELECT note_id,source_id,1 FROM note_shadowed_sources UNION ALL SELECT note_id,source_id,0 FROM note_search_snapshots")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })?;
        rows.map(|r| {
            let (note, source, shadowed) = r?;
            Ok((parse_id(note)?, parse_id(source)?, shadowed))
        })
        .collect()
    }

    pub fn note_search_snapshot(&self, id: Uuid) -> NoteResult<Option<NoteSearchSnapshot>> {
        self.conn
            .query_row(
                "SELECT source_id,snapshot_json,snapshot_sha256 FROM note_search_snapshots WHERE note_id=?1",
                [id.to_string()],
                |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .map(|(source, bytes, digest)| {
                let snapshot: NoteSearchSnapshot = decode(bytes, digest)?;
                if snapshot.receipt.note_id != id || snapshot.receipt.source_id != parse_id(source)? {
                    return Err(failure(NoteErrorCode::Storage,"managed snapshot identity mismatch"));
                }
                let version = self.checked_search_revision(&snapshot.receipt)?;
                if snapshot.fingerprint.len != version.bytes.len() as u64 || snapshot.fingerprint.sha256 != version.sha256 {
                    return Err(failure(NoteErrorCode::Storage,"managed snapshot fingerprint mismatch"));
                }
                Ok(snapshot)
            })
            .transpose()
    }

    fn checked_search_revision(&self, receipt: &NoteSearchReceipt) -> NoteResult<Version> {
        let version = self
            .version(receipt.version_id)?
            .ok_or_else(|| failure(NoteErrorCode::Storage, "managed search revision missing"))?;
        let origin: Option<String> = self.conn.query_row(
            "SELECT origin FROM sources WHERE id=?1",
            [receipt.source_id.to_string()],
            |r| r.get(0),
        )?;
        if version.source_id != receipt.source_id
            || origin.as_deref() != Some(format!("brn-note:{}", receipt.note_id).as_str())
        {
            return Err(failure(
                NoteErrorCode::Storage,
                "managed search revision identity mismatch",
            ));
        }
        Ok(version)
    }

    pub fn note_evidence_epochs(&self, id: Uuid) -> NoteResult<(u64, u64)> {
        let (content, approval): (i64, i64) = self.conn.query_row(
            "SELECT content_epoch,approval_epoch FROM notes WHERE id=?1",
            [id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok((
            u64::try_from(content)
                .map_err(|_| failure(NoteErrorCode::Storage, "invalid content epoch"))?,
            u64::try_from(approval)
                .map_err(|_| failure(NoteErrorCode::Storage, "invalid approval epoch"))?,
        ))
    }

    pub fn note_search_replay(
        &self,
        op: Uuid,
        request: &NoteSearchRequest,
    ) -> NoteResult<Option<(NoteSearchReceipt, bool)>> {
        let args = json(request)?;
        if !bound(&self.conn, op, "note.search", &args)? {
            return Ok(None);
        }
        let (bytes, digest) = self.conn.query_row(
            "SELECT result_json,result_sha256 FROM note_search_results WHERE operation_id=?1",
            [op.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let result: (NoteSearchReceipt, bool) = decode(bytes, digest)?;
        let receipt = &result.0;
        let precondition_matches = match request {
            NoteSearchRequest::Approve { file_state, .. } => *file_state == receipt.file_state,
            NoteSearchRequest::SetApproval {
                source_id,
                version_id,
                ..
            } => *source_id == receipt.source_id && *version_id == receipt.version_id,
            NoteSearchRequest::Import { .. } => true,
        };
        if receipt.operation_id != op
            || receipt.note_id != request.note_id()
            || !precondition_matches
        {
            return Err(failure(
                NoteErrorCode::Storage,
                "managed search receipt identity mismatch",
            ));
        }
        self.checked_search_revision(receipt)?;
        Ok(Some(result))
    }

    /// Freezes only the reconciled observation; buffer recovery is never evidence.
    pub fn freeze_note_search_snapshot(
        &mut self,
        op: Uuid,
        request: &NoteSearchRequest,
        state: Uuid,
        file: &FileFingerprint,
        text: &str,
    ) -> NoteResult<(NoteSearchReceipt, bool)> {
        if let Some(result) = self.note_search_replay(op, request)? {
            return Ok(result);
        }
        let id = request.note_id();
        if text.len() > MAX_NOTE_BYTES
            || hash(text.as_bytes()) != file.sha256
            || text.len() as u64 != file.len
        {
            return Err(failure(
                NoteErrorCode::Storage,
                "snapshot bytes differ from observation",
            ));
        }
        let args = json(request)?;
        let previous_snapshot = self.note_search_snapshot(id)?;
        let tx = self.conn.transaction()?;
        let record = note_record(&tx, id)?;
        if original_save_blocker(&tx, id)?.is_some()
            || record.baseline != *file
            || record.observed.as_ref() != Some(&(state, file.clone()))
        {
            return Err(failure(
                NoteErrorCode::StateChanged,
                "note must be reconciled before search approval",
            ));
        }
        if let NoteSearchRequest::Approve { file_state, .. } = request
            && *file_state != state
        {
            return Err(failure(
                NoteErrorCode::StateChanged,
                "saved file changed before approval",
            ));
        }
        workflow::bind_operation(&tx, op, "note.search", &args)?;
        let origin = format!("brn-note:{id}");
        let prior: Option<(String, Option<String>)> = tx
            .query_row(
                "SELECT id,current_version_id FROM sources WHERE origin=?1",
                [&origin],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (source, prior_version) = match prior {
            Some((s, v)) => (parse_id(s)?, v.map(parse_id).transpose()?),
            None => {
                let source = Uuid::new_v4();
                tx.execute(
                    "INSERT INTO sources(id,title,origin,approval) VALUES(?1,?2,?3,'draft')",
                    params![
                        source.to_string(),
                        record.relative_path.to_string_lossy(),
                        origin
                    ],
                )?;
                (source, None)
            }
        };
        if let NoteSearchRequest::SetApproval {
            source_id,
            version_id,
            ..
        } = request
            && (*source_id != source || Some(*version_id) != prior_version)
        {
            return Err(failure(
                NoteErrorCode::StateChanged,
                "managed snapshot version changed before approval",
            ));
        }
        let unchanged = if let Some(version) = prior_version {
            let bytes: Vec<u8> = tx.query_row(
                "SELECT bytes FROM versions WHERE id=?1",
                [version.to_string()],
                |r| r.get(0),
            )?;
            bytes == text.as_bytes()
                && previous_snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.fingerprint == *file && snapshot.receipt.file_state == state
                })
        } else {
            false
        };
        let version = if unchanged {
            prior_version.unwrap()
        } else {
            let version = Uuid::new_v4();
            tx.execute(
                "INSERT INTO versions(id,source_id,parent_id,bytes,sha256) VALUES(?1,?2,?3,?4,?5)",
                params![
                    version.to_string(),
                    source.to_string(),
                    prior_version.map(|v| v.to_string()),
                    text.as_bytes(),
                    file.sha256.as_slice()
                ],
            )?;
            version
        };
        let approval = request.approval().as_str();
        tx.execute(
            "UPDATE sources SET current_version_id=?2,approval=?3,title=?4 WHERE id=?1",
            params![
                source.to_string(),
                version.to_string(),
                approval,
                record.relative_path.to_string_lossy()
            ],
        )?;
        tx.execute("UPDATE notes SET approval_epoch=approval_epoch+CASE WHEN approval<>?2 THEN 1 ELSE 0 END,approval=?2 WHERE id=?1",params![id.to_string(),approval])?;
        let receipt = NoteSearchReceipt {
            operation_id: op,
            note_id: id,
            file_state: state,
            source_id: source,
            version_id: version,
        };
        let snapshot = json(&NoteSearchSnapshot {
            receipt: receipt.clone(),
            fingerprint: file.clone(),
        })?;
        tx.execute("INSERT INTO note_search_snapshots(note_id,source_id,snapshot_json,snapshot_sha256) VALUES(?1,?2,?3,?4) ON CONFLICT(note_id) DO UPDATE SET snapshot_json=excluded.snapshot_json,snapshot_sha256=excluded.snapshot_sha256",params![id.to_string(),source.to_string(),snapshot,hash(&snapshot).as_slice()])?;
        let result = (receipt, !unchanged);
        let bytes = json(&result)?;
        tx.execute("INSERT INTO note_search_results(operation_id,result_json,result_sha256) VALUES(?1,?2,?3)",params![op.to_string(),bytes,hash(&bytes).as_slice()])?;
        tx.execute(
            "UPDATE operations SET status='completed' WHERE id=?1",
            [op.to_string()],
        )?;
        tx.commit()?;
        Ok(result)
    }

    pub fn note_original_save_blocker(&self, id: Uuid) -> NoteResult<Option<NoteFailure>> {
        original_save_blocker(&self.conn, id)
    }
    /// Compact destination binding survives payload pruning and later relinks.
    pub fn note_write_destination(&self, op: Uuid) -> NoteResult<Option<PathBuf>> {
        let row: Option<(Vec<u8>, Vec<u8>)> = self.conn.query_row(
            "SELECT destination_json,destination_sha256 FROM note_write_destinations WHERE operation_id=?1",
            [op.to_string()], |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional()?;
        row.map(|(bytes, digest)| decode(bytes, digest)).transpose()
    }

    pub fn note_decision_replay(
        &self,
        op: Uuid,
        decision: &NoteDecision,
    ) -> NoteResult<Option<Uuid>> {
        if !bound(&self.conn, op, decision.operation_kind(), &json(decision)?)? {
            return Ok(None);
        }
        stored_result(&self.conn, "note_results", op)
    }

    /// Protected snapshot retained by an uncertain-outcome acknowledgement.
    pub fn note_decision_recovery(&self, op: Uuid) -> NoteResult<Option<NoteRecovery>> {
        let row: Option<(Vec<u8>, Vec<u8>)> = self.conn.query_row(
            "SELECT recovery_json,recovery_sha256 FROM note_decision_recoveries WHERE operation_id=?1",
            [op.to_string()], |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional()?;
        row.map(|(bytes, digest)| decode(bytes, digest)).transpose()
    }

    pub fn record_note_decision(
        &mut self,
        op: Uuid,
        decision: &NoteDecision,
        file: &FileFingerprint,
        text: &str,
    ) -> NoteResult<NoteRecovery> {
        let tx = self.conn.transaction()?;
        if workflow::bind_operation(&tx, op, decision.operation_kind(), &json(decision)?)?
            == BeginOperation::Existing
        {
            let id: Uuid = stored_result(&tx, "note_results", op)?
                .ok_or_else(|| failure(NoteErrorCode::Storage, "decision has no result"))?;
            return recovery(&tx, id)?
                .ok_or_else(|| failure(NoteErrorCode::Storage, "decision note lacks recovery"));
        }
        valid_text(text)?;
        if !matches_text(file, text) {
            return Err(failure(
                NoteErrorCode::StateChanged,
                "decision observation bytes differ",
            ));
        }
        let (id, expected, path, discard, mut accepted) = match decision {
            NoteDecision::Reload {
                note_id,
                expected,
                discard,
            } => (*note_id, Some(*expected), None, *discard, None),
            NoteDecision::Relink {
                note_id,
                expected,
                relative,
                confirm_identity,
            } => {
                if !confirm_identity {
                    return Err(failure(
                        NoteErrorCode::Conflict,
                        "relink requires explicit identity confirmation",
                    ));
                }
                destination(relative)?;
                (*note_id, Some(*expected), Some(relative), false, None)
            }
            NoteDecision::AcceptCurrent {
                save_operation_id,
                observed_file_state,
            } => {
                let value = require_intent(&tx, *save_operation_id)?;
                let status: String = tx.query_row(
                    "SELECT status FROM operations WHERE id=?1",
                    [save_operation_id.to_string()],
                    |r| r.get(0),
                )?;
                if matches!(status.as_str(), "pending" | "running") {
                    return Err(failure(
                        NoteErrorCode::WorkspaceBusy,
                        "save job is still active",
                    ));
                }
                if value.kind != NoteWriteKind::Replace
                    || value.resolution != NoteResolution::Unresolved
                    || !matches!(value.prior_result, Some(NoteRecordedResult::Failure(_)))
                {
                    return Err(failure(
                        NoteErrorCode::StateChanged,
                        "accept-current requires an unresolved original save outcome",
                    ));
                }
                let record = note_record(&tx, value.request.note_id)?;
                if record.observed.as_ref() != Some(&(*observed_file_state, file.clone())) {
                    return Err(failure(
                        NoteErrorCode::StateChanged,
                        "reviewed disk observation changed; compare again",
                    ));
                }
                (value.request.note_id, None, None, false, Some(value))
            }
        };
        let record = note_record(&tx, id)?;
        let before = recovery(&tx, id)?
            .ok_or_else(|| failure(NoteErrorCode::Storage, "note lacks recovery"))?;
        if expected.is_some_and(|expected| expected != before.stamp) {
            return Err(failure(
                NoteErrorCode::StateChanged,
                "note buffer stamp changed",
            ));
        }
        // A metadata decision cannot race a live writer, even if its phase looks early.
        let active: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM note_save_intents i JOIN operations o ON o.id=i.operation_id WHERE i.source_note_id=?1 AND o.status IN ('pending','running'))",
            [id.to_string()], |r| r.get(0),
        )?;
        if active {
            return Err(failure(
                NoteErrorCode::WorkspaceBusy,
                "save job is still active",
            ));
        }
        if matches!(decision, NoteDecision::Reload { .. }) {
            if record.baseline.device != file.device || record.baseline.inode != file.inode {
                return Err(failure(
                    NoteErrorCode::Conflict,
                    "file identity changed; use confirmed relink",
                ));
            }
            if !discard
                && (before.working != before.baseline || !before.pending_operations.is_empty())
            {
                return Err(failure(
                    NoteErrorCode::Conflict,
                    "reload requires explicit discard confirmation",
                ));
            }
        }
        if let Some(path) = path {
            let reserved: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM notes WHERE vault_id=?1 AND relative=?2 AND id!=?3 UNION ALL SELECT 1 FROM note_save_intents WHERE vault_id=?1 AND destination=?2 AND write_kind='copy' AND resolution IN ('unresolved','applied','accepted_current'))",
                params![record.vault_id.to_string(), destination(path)?, id.to_string()], |r| r.get(0),
            )?;
            if reserved {
                return Err(failure(
                    NoteErrorCode::Conflict,
                    "relink path is registered or reserved",
                ));
            }
            tx.execute(
                "UPDATE notes SET relative=?2 WHERE id=?1",
                params![id.to_string(), destination(path)?],
            )?;
        }
        if accepted.is_some() {
            let bytes = json(&before)?;
            tx.execute("INSERT INTO note_decision_recoveries(operation_id,recovery_json,recovery_sha256) VALUES(?1,?2,?3)",
                params![op.to_string(),bytes,hash(&bytes).as_slice()])?;
        }
        // Even an unchanged-disk discard must invalidate queued pre-decision edits.
        let state = Uuid::new_v4();
        let bytes = json(file)?;
        tx.execute("UPDATE notes SET file_state=?2,fingerprint_json=?3,fingerprint_sha256=?4,observed_file_state=?2,observed_fingerprint_json=?3,observed_fingerprint_sha256=?4,content_epoch=content_epoch+1,approval_epoch=approval_epoch+1,approval='draft' WHERE id=?1",
            params![id.to_string(),state.to_string(),bytes,hash(&bytes).as_slice()])?;
        let working = if matches!(decision, NoteDecision::Reload { .. }) {
            text
        } else {
            &before.working
        };
        tx.execute("UPDATE note_buffers SET file_state=?2,baseline=?3,baseline_sha256=?4,working=?5,working_sha256=?6 WHERE note_id=?1",
            params![id.to_string(),state.to_string(),text.as_bytes(),hash(text.as_bytes()).as_slice(),working.as_bytes(),hash(working.as_bytes()).as_slice()])?;
        if let Some(value) = &mut accepted {
            value.resolution = NoteResolution::AcceptedCurrent;
            value.acknowledged_by = Some(op);
            update_intent(&tx, value)?;
        }
        insert_result(&tx, "note_results", op, id, &id)?;
        tx.execute(
            "UPDATE operations SET status='completed' WHERE id=?1",
            [op.to_string()],
        )?;
        let result = recovery(&tx, id)?.unwrap();
        tx.commit()?;
        Ok(result)
    }

    pub fn accept_note_disk_state(
        &mut self,
        ack_op: Uuid,
        save_op: Uuid,
        token: Uuid,
        file: &FileFingerprint,
        text: &str,
    ) -> NoteResult<NoteRecovery> {
        self.record_note_decision(
            ack_op,
            &NoteDecision::AcceptCurrent {
                save_operation_id: save_op,
                observed_file_state: token,
            },
            file,
            text,
        )
    }

    pub fn enroll_note(
        &mut self,
        op: Uuid,
        vault: &VaultRecord,
        path: &Path,
        file: FileFingerprint,
        text: &str,
    ) -> NoteResult<NoteRecovery> {
        self.enroll_note_at(op, &vault.root, vault, path, file, text)
    }

    /// Uses the caller's exact root spelling for replay, independently of the canonical registry.
    pub fn enroll_note_at(
        &mut self,
        op: Uuid,
        root: &Path,
        vault: &VaultRecord,
        path: &Path,
        file: FileFingerprint,
        text: &str,
    ) -> NoteResult<NoteRecovery> {
        let payload = json(&(root, path))?;
        let tx = self.conn.transaction()?;
        if workflow::bind_operation(&tx, op, "note.enroll", &payload)? == BeginOperation::Existing {
            return stored_result(&tx, "note_results", op)?
                .ok_or_else(|| failure(NoteErrorCode::Storage, "enrollment has no result"));
        }
        let path_str = destination(path)?;
        valid_text(text)?;
        if !matches_text(&file, text) {
            return Err(failure(
                NoteErrorCode::StateChanged,
                "enrollment fingerprint does not match exact bytes",
            ));
        }
        if !vault.root.is_absolute()
            || self
                .db_path
                .parent()
                .is_some_and(|data| data.starts_with(&vault.root))
        {
            return Err(failure(
                NoteErrorCode::Unsupported,
                "vault must be absolute and outside the data directory",
            ));
        }
        let old: Option<(Vec<u8>, Vec<u8>)> = tx
            .query_row(
                "SELECT record_json,record_sha256 FROM note_vaults",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let vault_id = if let Some((bytes, digest)) = old {
            let old: VaultRecord = decode(bytes, digest)?;
            if old.identity != vault.identity {
                return Err(failure(
                    NoteErrorCode::VaultUnavailable,
                    "workspace is bound to another vault; use a separate data directory",
                ));
            }
            old.id
        } else {
            let bytes = json(vault)?;
            tx.execute("INSERT INTO note_vaults(id,singleton,record_json,record_sha256) VALUES(?1,1,?2,?3)",
                params![vault.id.to_string(),bytes,hash(&bytes).as_slice()])?;
            vault.id
        };
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM notes WHERE vault_id=?1 AND relative=?2",
                params![vault_id.to_string(), path_str],
                |r| r.get(0),
            )
            .optional()?;
        let reserved: Option<String> = tx
            .query_row(
                "SELECT target_note_id FROM note_save_intents WHERE vault_id=?1 AND destination=?2 AND write_kind='copy' AND resolution IN ('unresolved','applied','accepted_current')",
                params![vault_id.to_string(), path_str],
                |r| r.get(0),
            )
            .optional()?;
        if reserved
            .as_ref()
            .is_some_and(|target| existing.as_ref() != Some(target))
        {
            return Err(failure(
                NoteErrorCode::Conflict,
                "note path is reserved by a copy intent",
            ));
        }
        let recovered = if let Some(existing) = existing {
            recovery(&tx, parse_id(existing)?)?
                .ok_or_else(|| failure(NoteErrorCode::Storage, "registered note lacks buffer"))?
        } else {
            let note_id = Uuid::new_v4();
            let stamp = NoteStamp {
                file_state: Uuid::new_v4(),
                generation: 0,
            };
            let bytes = json(&file)?;
            tx.execute("INSERT INTO notes(id,vault_id,relative,file_state,fingerprint_json,fingerprint_sha256) VALUES(?1,?2,?3,?4,?5,?6)",
                params![note_id.to_string(),vault_id.to_string(),path_str,stamp.file_state.to_string(),bytes,hash(&bytes).as_slice()])?;
            insert_buffer(&tx, note_id, stamp, text, text)?;
            // Path provenance, never matching bytes, identifies shadowed legacy imports.
            let origin = vault.root.join(path).to_string_lossy().into_owned();
            tx.execute("INSERT INTO note_shadowed_sources(note_id,source_id) SELECT ?1,id FROM sources WHERE origin=?2",
                params![note_id.to_string(),origin])?;
            NoteRecovery {
                note_id,
                stamp,
                baseline: text.into(),
                working: text.into(),
                pending_operations: Vec::new(),
            }
        };
        insert_result(&tx, "note_results", op, recovered.note_id, &recovered)?;
        tx.execute(
            "UPDATE operations SET status='completed' WHERE id=?1",
            [op.to_string()],
        )?;
        tx.commit()?;
        Ok(recovered)
    }

    pub fn save_note_buffer(&mut self, request: &NoteSubmission) -> NoteResult<NoteBufferReceipt> {
        let tx = self.conn.transaction()?;
        if workflow::bind_operation(&tx, request.operation_id, "note.buffer", &json(request)?)?
            == BeginOperation::Existing
        {
            return stored_result(&tx, "note_results", request.operation_id)?
                .ok_or_else(|| failure(NoteErrorCode::Storage, "buffer operation has no receipt"));
        }
        accept_submission(&tx, request)?;
        let result = NoteBufferReceipt {
            operation_id: request.operation_id,
            note_id: request.note_id,
            stamp: NoteStamp {
                file_state: request.expected.file_state,
                generation: request.generation,
            },
        };
        insert_result(
            &tx,
            "note_results",
            request.operation_id,
            request.note_id,
            &result,
        )?;
        tx.execute(
            "UPDATE operations SET status='completed' WHERE id=?1",
            [request.operation_id.to_string()],
        )?;
        tx.commit()?;
        Ok(result)
    }

    pub fn note_recovery(&self, id: Uuid) -> NoteResult<Option<NoteRecovery>> {
        recovery(&self.conn, id)
    }

    pub fn note_enrollment_replay(
        &self,
        op: Uuid,
        root: &Path,
        relative: &Path,
    ) -> NoteResult<Option<Uuid>> {
        if !bound(&self.conn, op, "note.enroll", &json(&(root, relative))?)? {
            return Ok(None);
        }
        let result: NoteRecovery = stored_result(&self.conn, "note_results", op)?
            .ok_or_else(|| failure(NoteErrorCode::Storage, "enrollment has no result"))?;
        Ok(Some(result.note_id))
    }

    pub fn registered_vault(&self) -> NoteResult<Option<VaultRecord>> {
        let row: Option<(String, Vec<u8>, Vec<u8>)> = self
            .conn
            .query_row(
                "SELECT id,record_json,record_sha256 FROM note_vaults",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        row.map(|(id, bytes, digest)| {
            let record: VaultRecord = decode(bytes, digest)?;
            if record.id != parse_id(id)? || !record.root.is_absolute() {
                return Err(failure(NoteErrorCode::Storage, "invalid registered vault"));
            }
            Ok(record)
        })
        .transpose()
    }

    pub fn note_record(&self, id: Uuid) -> NoteResult<NoteRecord> {
        note_record(&self.conn, id)
    }

    /// Includes clean notes as well as every protected or unresolved recovery.
    pub fn note_recoveries(&self) -> NoteResult<Vec<NoteRecovery>> {
        let mut stmt = self.conn.prepare("SELECT id FROM notes ORDER BY rowid")?;
        let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
        ids.map(|row| {
            recovery(&self.conn, parse_id(row?)?)?
                .ok_or_else(|| failure(NoteErrorCode::Storage, "registered note lacks recovery"))
        })
        .collect()
    }

    pub fn record_note_observation(
        &mut self,
        id: Uuid,
        file: &FileFingerprint,
    ) -> NoteResult<Uuid> {
        let tx = self.conn.transaction()?;
        let record = note_record(&tx, id)?;
        let token = if let Some((token, old)) = &record.observed
            && old == file
        {
            *token
        } else if &record.baseline == file {
            record.stamp.file_state
        } else {
            Uuid::new_v4()
        };
        let bytes = json(file)?;
        let changed = record
            .observed
            .as_ref()
            .map(|(_, old)| old)
            .unwrap_or(&record.baseline)
            != file;
        tx.execute(
            "UPDATE notes SET observed_file_state=?2,observed_fingerprint_json=?3,observed_fingerprint_sha256=?4,content_epoch=content_epoch+CASE WHEN ?5 THEN 1 ELSE 0 END,approval_epoch=approval_epoch+CASE WHEN ?5 AND approval='approved' THEN 1 ELSE 0 END,approval=CASE WHEN ?5 THEN 'draft' ELSE approval END WHERE id=?1",
            params![id.to_string(),token.to_string(),bytes,hash(&bytes).as_slice(),changed],
        )?;
        tx.commit()?;
        Ok(token)
    }

    pub fn note_vault(&self, id: Uuid) -> NoteResult<VaultRecord> {
        let note = self.note_record(id)?;
        let vault = self
            .registered_vault()?
            .ok_or_else(|| failure(NoteErrorCode::Storage, "registered note has no vault"))?;
        if vault.id != note.vault_id {
            return Err(failure(
                NoteErrorCode::Storage,
                "note vault identity does not match registry",
            ));
        }
        Ok(vault)
    }

    pub fn note_write_result(
        &self,
        request: &NoteSubmission,
        path: &Path,
        kind: NoteWriteKind,
    ) -> NoteResult<Option<NoteRecordedResult>> {
        if !bound(
            &self.conn,
            request.operation_id,
            kind.operation_kind(),
            &write_payload(request, path, kind)?,
        )? {
            return Ok(None);
        }
        stored_result(&self.conn, "note_receipts", request.operation_id)
    }

    /// Metadata-only validation; acceptance is rechecked inside each write transaction.
    pub fn validate_note_submission(&self, request: &NoteSubmission) -> NoteResult<()> {
        checked_submission(&self.conn, request).map(|_| ())
    }

    /// Reads a compact original-save result even after its full intent is pruned.
    /// ID-only reconciliation does not submit or bind a new write payload.
    pub fn note_save_result(&self, op: Uuid) -> NoteResult<Option<NoteRecordedResult>> {
        let kind: Option<String> = self
            .conn
            .query_row(
                "SELECT kind FROM operations WHERE id=?1",
                [op.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        match kind.as_deref() {
            None => Ok(None),
            Some("note.save" | "note.copy") => stored_result(&self.conn, "note_receipts", op),
            Some(_) => Err(failure(
                NoteErrorCode::OperationConflict,
                "operation is not a note save/copy",
            )),
        }
    }

    pub fn begin_note_save(
        &mut self,
        request: &NoteSubmission,
        path: &Path,
        kind: NoteWriteKind,
        expected: &DestinationPrecondition,
    ) -> NoteResult<NoteSaveIntent> {
        let payload = write_payload(request, path, kind)?;
        let tx = self.conn.transaction()?;
        if workflow::bind_operation(&tx, request.operation_id, kind.operation_kind(), &payload)?
            == BeginOperation::Existing
        {
            let value = require_intent(&tx, request.operation_id)?;
            if &value.expected_destination != expected {
                return Err(failure(
                    NoteErrorCode::OperationConflict,
                    "destination precondition conflicts with bound intent",
                ));
            }
            return Ok(value);
        }
        let dest = destination(path)?;
        let (vault_id, original, file_json, file_hash): (String, String, Vec<u8>, Vec<u8>) = tx
            .query_row(
                "SELECT vault_id,relative,fingerprint_json,fingerprint_sha256 FROM notes WHERE id=?1",
                [request.note_id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?
            .ok_or_else(|| failure(NoteErrorCode::Missing, "note does not exist"))?;
        if kind == NoteWriteKind::Replace
            && let Some(error) = original_save_blocker(&tx, request.note_id)?
        {
            return Err(error);
        }
        let before = accept_submission(&tx, request)?;
        match (kind, expected) {
            (
                NoteWriteKind::Replace,
                DestinationPrecondition::Existing {
                    fingerprint,
                    baseline_text,
                },
            ) => {
                let registered: FileFingerprint = decode(file_json, file_hash)?;
                if dest != original
                    || baseline_text != &before.baseline
                    || fingerprint != &registered
                    || !matches_text(fingerprint, baseline_text)
                {
                    return Err(failure(
                        NoteErrorCode::StateChanged,
                        "original destination precondition changed",
                    ));
                }
            }
            (NoteWriteKind::Copy, DestinationPrecondition::Absent { .. }) => {
                let reserved: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM notes WHERE vault_id=?1 AND relative=?2 UNION ALL SELECT 1 FROM note_save_intents WHERE vault_id=?1 AND destination=?2 AND write_kind='copy' AND resolution IN ('unresolved','applied','accepted_current'))",
                    params![vault_id,dest],|r|r.get(0))?;
                if dest == original || reserved {
                    return Err(failure(
                        NoteErrorCode::Conflict,
                        "copy destination is registered or reserved",
                    ));
                }
            }
            _ => {
                return Err(failure(
                    NoteErrorCode::Unsupported,
                    "write kind and destination precondition disagree",
                ));
            }
        }
        let staging_relative = path
            .parent()
            .unwrap_or(Path::new(""))
            .join(format!(".brn-{}.stage", request.operation_id));
        let value = NoteSaveIntent {
            request: request.clone(),
            target_note_id: if kind == NoteWriteKind::Copy {
                Uuid::new_v4()
            } else {
                request.note_id
            },
            destination: path.into(),
            staging_relative,
            kind,
            expected_destination: expected.clone(),
            staged: None,
            displaced: None,
            phase: SavePhase::Intent,
            resolution: NoteResolution::Unresolved,
            acknowledged_by: None,
            cleanup: ArtifactCleanup::Pending,
            prior_result: None,
        };
        let bytes = json(&value)?;
        tx.execute("INSERT INTO note_save_intents(operation_id,source_note_id,target_note_id,vault_id,destination,write_kind,phase,resolution,cleanup,intent_json,intent_sha256) VALUES(?1,?2,?3,?4,?5,?6,'intent','unresolved','pending',?7,?8)",
            params![request.operation_id.to_string(),request.note_id.to_string(),value.target_note_id.to_string(),vault_id,dest,kind.as_str(),bytes,hash(&bytes).as_slice()])?;
        retain_input(&tx, request, path)?;
        tx.commit()?;
        Ok(value)
    }

    pub fn record_note_write_failure(
        &mut self,
        request: &NoteSubmission,
        path: &Path,
        kind: NoteWriteKind,
        error: &NoteFailure,
    ) -> NoteResult<NoteFailure> {
        let result = (|| {
            let tx = self.conn.transaction()?;
            let state = workflow::bind_operation(
                &tx,
                request.operation_id,
                kind.operation_kind(),
                &write_payload(request, path, kind)?,
            )?;
            if let Some(prior) =
                stored_result::<NoteRecordedResult>(&tx, "note_receipts", request.operation_id)?
            {
                return match prior {
                    NoteRecordedResult::Failure(old) => Ok(old),
                    _ => Err(failure(
                        NoteErrorCode::OperationConflict,
                        "write already has a successful receipt",
                    )),
                };
            }
            destination(path)?;
            if state == BeginOperation::New {
                accept_submission(&tx, request)?;
                retain_input(&tx, request, path)?;
            } else if intent(&tx, request.operation_id)?.is_none() {
                return Err(failure(
                    NoteErrorCode::Storage,
                    "bound write has no protected input",
                ));
            }
            let mut acknowledged = error.clone();
            acknowledged.operation_id = Some(request.operation_id);
            acknowledged.note_id = Some(request.note_id);
            acknowledged.recovery_available = true;
            if let Some(mut value) = intent(&tx, request.operation_id)? {
                if acknowledged.phase != Some(value.phase) {
                    return Err(failure(
                        NoteErrorCode::StateChanged,
                        "failure phase disagrees with intent",
                    ));
                }
                if value.phase == SavePhase::Complete {
                    return Err(failure(NoteErrorCode::StateChanged, "save is complete"));
                }
                if value.displaced.is_some() && acknowledged.code == NoteErrorCode::Conflict {
                    value.cleanup = ArtifactCleanup::RetainedUnexpected;
                }
                update_intent(&tx, &value)?;
                // A failure receipt is not execution proof: keep the dedicated resolution unresolved.
            }
            insert_result(
                &tx,
                "note_receipts",
                request.operation_id,
                request.note_id,
                &NoteRecordedResult::Failure(acknowledged.clone()),
            )?;
            tx.execute(
                "UPDATE operations SET status='failed' WHERE id=?1",
                [request.operation_id.to_string()],
            )?;
            tx.commit()?;
            Ok(acknowledged)
        })();
        result.map_err(|mut storage| {
            storage.code = NoteErrorCode::Storage;
            storage.message = format!(
                "could not record note refusal ({:?}: {}): {}",
                error.code, error.message, storage.message
            );
            storage.operation_id = Some(request.operation_id);
            storage.note_id = Some(request.note_id);
            storage.phase = error.phase;
            storage.filesystem_outcome = error.filesystem_outcome;
            storage.recovery_available = false;
            storage
        })
    }

    pub fn note_save_intent(&self, op: Uuid) -> NoteResult<Option<NoteSaveIntent>> {
        intent(&self.conn, op)
    }

    pub fn note_save_intents(&self) -> NoteResult<Vec<NoteSaveIntent>> {
        let mut stmt = self
            .conn
            .prepare("SELECT operation_id FROM note_save_intents ORDER BY rowid")?;
        let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
        ids.map(|row| require_intent(&self.conn, parse_id(row?)?))
            .collect()
    }

    fn mutate_note_intent(
        &mut self,
        op: Uuid,
        edit: impl FnOnce(&Transaction<'_>, &mut NoteSaveIntent) -> NoteResult<()>,
    ) -> NoteResult<()> {
        let context = intent(&self.conn, op)?;
        let result = (|| {
            let tx = self.conn.transaction()?;
            let mut value = require_intent(&tx, op)?;
            if value.resolution != NoteResolution::Unresolved || value.prior_result.is_some() {
                return Err(failure(
                    NoteErrorCode::StateChanged,
                    "note write is not active",
                ));
            }
            let status: String = tx.query_row(
                "SELECT status FROM operations WHERE id=?1",
                [op.to_string()],
                |r| r.get(0),
            )?;
            if !matches!(status.as_str(), "pending" | "running") {
                return Err(failure(
                    NoteErrorCode::SaveUncertain,
                    "interrupted writes require reconciliation",
                ));
            }
            edit(&tx, &mut value)?;
            update_intent(&tx, &value)?;
            tx.commit()?;
            Ok(())
        })();
        result.map_err(|e| failure_context(e, op, context.as_ref()))
    }

    pub fn record_note_prepared(&mut self, op: Uuid, prepared: &PreparedFile) -> NoteResult<()> {
        self.mutate_note_intent(op, |_, value| {
            if value.staged.as_ref() == Some(prepared) && value.phase == SavePhase::Prepared {
                return Ok(());
            }
            if value.phase != SavePhase::Intent
                || prepared.relative != value.staging_relative
                || !matches_text(&prepared.fingerprint, &value.request.text)
            {
                return Err(failure(
                    NoteErrorCode::StateChanged,
                    "prepared file does not match intent",
                ));
            }
            value.staged = Some(prepared.clone());
            value.phase = SavePhase::Prepared;
            Ok(())
        })
    }

    pub fn mark_note_exchanged(&mut self, op: Uuid) -> NoteResult<()> {
        self.mutate_note_intent(op, |_, value| {
            if value.phase == SavePhase::Exchanged {
                return Ok(());
            }
            if value.phase != SavePhase::Prepared {
                return Err(failure(
                    NoteErrorCode::StateChanged,
                    "exchange requires prepared identity",
                ));
            }
            value.phase = SavePhase::Exchanged;
            Ok(())
        })
    }

    pub fn record_note_displaced(
        &mut self,
        op: Uuid,
        artifact: &RetainedArtifact,
    ) -> NoteResult<()> {
        self.mutate_note_intent(op, |_, value| {
            if value.phase != SavePhase::Exchanged
                || value.kind != NoteWriteKind::Replace
                || artifact.relative != value.staging_relative
            {
                return Err(failure(
                    NoteErrorCode::StateChanged,
                    "displaced artifact does not match exchanged intent",
                ));
            }
            if value.displaced.as_ref().is_some_and(|old| old != artifact) {
                return Err(failure(
                    NoteErrorCode::Conflict,
                    "displaced identity changed",
                ));
            }
            value.displaced = Some(artifact.clone());
            if !artifact_is_expected(value, artifact) {
                value.cleanup = ArtifactCleanup::RetainedUnexpected;
            }
            Ok(())
        })
    }

    pub fn record_note_verification(
        &mut self,
        op: Uuid,
        verification: &NoteVerification,
    ) -> NoteResult<()> {
        self.mutate_note_intent(op, |tx, value| {
            if value.phase == SavePhase::Verified {
                if load_verification(tx, op)?.as_ref() == Some(verification) {
                    return Ok(());
                }
                return Err(failure(
                    NoteErrorCode::OperationConflict,
                    "verification conflicts",
                ));
            }
            if value.phase != SavePhase::Exchanged {
                return Err(failure(
                    NoteErrorCode::StateChanged,
                    "verification requires exchanged phase",
                ));
            }
            verify(value, verification)?;
            persist_verification(tx, op, verification)?;
            if let NoteVerification::Replace { displaced, .. } = verification {
                value.displaced = Some(displaced.clone());
            }
            value.phase = SavePhase::Verified;
            Ok(())
        })
    }

    pub fn finish_note_save(&mut self, op: Uuid, receipt: &NoteReceipt) -> NoteResult<()> {
        let context = intent(&self.conn, op)?;
        let result = (|| {
            let tx = self.conn.transaction()?;
            let mut value = require_intent(&tx, op)?;
            if let Some(prior) = &value.prior_result {
                return if prior == &NoteRecordedResult::Receipt(receipt.clone()) {
                    Ok(())
                } else {
                    Err(failure(
                        NoteErrorCode::OperationConflict,
                        "terminal receipt conflicts",
                    ))
                };
            }
            let status: String = tx.query_row(
                "SELECT status FROM operations WHERE id=?1",
                [op.to_string()],
                |r| r.get(0),
            )?;
            if !matches!(status.as_str(), "pending" | "running") {
                return Err(failure(
                    NoteErrorCode::SaveUncertain,
                    "interrupted saves require reconciliation",
                ));
            }
            let verification = load_verification(&tx, op)?;
            let (resolution, observed) = match receipt.filesystem_outcome {
                FileOutcome::Applied if value.phase == SavePhase::Verified => (
                    NoteResolution::Applied,
                    verification.as_ref().map(installed).cloned(),
                ),
                FileOutcome::NotApplied
                    if value.phase == SavePhase::Intent && value.kind == NoteWriteKind::Replace =>
                {
                    match &value.expected_destination {
                        DestinationPrecondition::Existing {
                            fingerprint,
                            baseline_text,
                        } if baseline_text == &value.request.text => {
                            (NoteResolution::NotApplied, Some(fingerprint.clone()))
                        }
                        _ => {
                            return Err(failure(
                                NoteErrorCode::StateChanged,
                                "not-applied completion must be unchanged",
                            ));
                        }
                    }
                }
                _ => {
                    return Err(failure(
                        NoteErrorCode::StateChanged,
                        "receipt requires verified filesystem outcome",
                    ));
                }
            };
            if resolution == NoteResolution::NotApplied && value.cleanup == ArtifactCleanup::Pending
            {
                // The active unchanged-save path never creates an artifact. Interrupted
                // intent reconciliation cannot make this inference from missing metadata.
                value.cleanup = ArtifactCleanup::Retired;
            }
            commit_reconciliation(
                &tx,
                value,
                &NoteReconciliation {
                    resolution,
                    observed_destination: observed,
                    verification,
                    result: NoteRecordedResult::Receipt(receipt.clone()),
                },
            )?;
            tx.commit()?;
            Ok(())
        })();
        result.map_err(|e| failure_context(e, op, context.as_ref()))
    }

    pub fn reconcile_note_operation(
        &mut self,
        op: Uuid,
        record: &NoteReconciliation,
    ) -> NoteResult<NoteRecordedResult> {
        let context = intent(&self.conn, op)?;
        let result = (|| {
            let tx = self.conn.transaction()?;
            let value = require_intent(&tx, op)?;
            if value.resolution != NoteResolution::Unresolved {
                return if value.prior_result.as_ref() == Some(&record.result)
                    && value.resolution == record.resolution
                {
                    Ok(record.result.clone())
                } else {
                    Err(failure(
                        NoteErrorCode::OperationConflict,
                        "reconciliation conflicts with terminal result",
                    ))
                };
            }
            let result = commit_reconciliation(&tx, value, record)?;
            tx.commit()?;
            Ok(result)
        })();
        result.map_err(|e| failure_context(e, op, context.as_ref()))
    }

    /// Returns exact retirement proof only after recovery and terminal outcome checks.
    /// The workflow must still observe and verify the occupant before unlinking it.
    pub fn note_cleanup_candidate(&self, op: Uuid) -> NoteResult<Option<RetainedArtifact>> {
        let value = require_intent(&self.conn, op)?;
        if value.kind != NoteWriteKind::Replace
            || value.cleanup != ArtifactCleanup::Pending
            || !matches!(
                value.resolution,
                NoteResolution::Applied | NoteResolution::NotApplied
            )
            || value.prior_result.as_ref().is_none_or(result_is_uncertain)
        {
            return Ok(None);
        }
        let current = recovery(&self.conn, value.request.note_id)?
            .ok_or_else(|| failure(NoteErrorCode::Storage, "cleanup requires durable recovery"))?;
        // Loading the intent/recovery verifies the hashes of both protected snapshots.
        if current.stamp.generation < value.request.generation {
            return Err(failure(
                NoteErrorCode::Storage,
                "cleanup recovery generation is stale",
            ));
        }
        let candidate = match value.resolution {
            NoteResolution::Applied => {
                let verification = load_verification(&self.conn, op)?.ok_or_else(|| {
                    failure(
                        NoteErrorCode::SaveUncertain,
                        "cleanup lacks displaced recovery proof",
                    )
                })?;
                verify(&value, &verification)?;
                let NoteVerification::Replace { displaced, .. } = verification else {
                    return Ok(None);
                };
                displaced
            }
            NoteResolution::NotApplied => {
                let Some(prepared) = value.staged else {
                    return Ok(None);
                };
                RetainedArtifact {
                    relative: prepared.relative,
                    identity: ArtifactIdentity {
                        device: prepared.fingerprint.device,
                        inode: prepared.fingerprint.inode,
                        len: prepared.fingerprint.len,
                        kind: ArtifactKind::Regular,
                    },
                    sha256: Some(prepared.fingerprint.sha256),
                }
            }
            _ => return Ok(None),
        };
        for other in self.note_save_intents()? {
            if other.request.operation_id == op || other.cleanup == ArtifactCleanup::Retired {
                continue;
            }
            let referenced = other.staging_relative == candidate.relative
                || other.staged.as_ref().is_some_and(|file| {
                    file.fingerprint.device == candidate.identity.device
                        && file.fingerprint.inode == candidate.identity.inode
                })
                || other.displaced.as_ref().is_some_and(|file| {
                    file.identity.device == candidate.identity.device
                        && file.identity.inode == candidate.identity.inode
                });
            if referenced {
                return Ok(None);
            }
        }
        Ok(Some(candidate))
    }

    /// Records the workflow's verified artifact bookkeeping; never performs disk cleanup.
    pub fn record_note_cleanup(&mut self, op: Uuid, cleanup: ArtifactCleanup) -> NoteResult<()> {
        let context = intent(&self.conn, op)?;
        let result = (|| {
            let tx = self.conn.transaction()?;
            let mut value = require_intent(&tx, op)?;
            if value.cleanup == cleanup {
                return Ok(());
            }
            if value.cleanup != ArtifactCleanup::Pending || cleanup == ArtifactCleanup::Pending {
                return Err(failure(
                    NoteErrorCode::StateChanged,
                    "note cleanup cannot reverse or change its terminal disposition",
                ));
            }
            if cleanup == ArtifactCleanup::Retired {
                let terminal = value.prior_result.as_ref().ok_or_else(|| {
                    failure(
                        NoteErrorCode::StateChanged,
                        "retirement requires a terminal result",
                    )
                })?;
                if !matches!(
                    value.resolution,
                    NoteResolution::Applied | NoteResolution::NotApplied
                ) || result_is_uncertain(terminal)
                {
                    return Err(failure(
                        NoteErrorCode::SaveUncertain,
                        "unresolved, accepted-current or uncertain artifacts cannot be retired",
                    ));
                }
                let status: String = tx.query_row(
                    "SELECT status FROM operations WHERE id=?1",
                    [op.to_string()],
                    |r| r.get(0),
                )?;
                if !matches!(status.as_str(), "completed" | "failed" | "interrupted") {
                    return Err(failure(
                        NoteErrorCode::StateChanged,
                        "retirement requires a terminal operation",
                    ));
                }
            }
            value.cleanup = cleanup;
            update_intent(&tx, &value)?;
            tx.commit()?;
            Ok(())
        })();
        result.map_err(|e| failure_context(e, op, context.as_ref()))
    }

    /// Retires only obsolete proven-success payloads. Receipts and the latest pair survive.
    pub fn prune_completed_note_payloads(&mut self, note: Uuid) -> NoteResult<()> {
        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare("SELECT operation_id FROM note_save_intents WHERE source_note_id=?1 AND phase='complete' AND resolution IN ('applied','not_applied') AND cleanup='retired' AND operation_id NOT IN (SELECT operation_id FROM note_recovery_pairs)")?;
        let rows = stmt.query_map([note.to_string()], |r| r.get::<_, String>(0))?;
        let ids = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);
        for id in ids {
            let op = parse_id(id)?;
            let value = require_intent(&tx, op)?;
            let Some(NoteRecordedResult::Receipt(receipt)) = &value.prior_result else {
                continue;
            };
            if receipt.filesystem_outcome == FileOutcome::Unknown
                || protected_dependency(&tx, op, receipt.stamp.file_state)?
            {
                continue;
            }
            tx.execute(
                "DELETE FROM note_save_intents WHERE operation_id=?1",
                [op.to_string()],
            )?;
            tx.execute(
                "DELETE FROM note_write_inputs WHERE operation_id=?1",
                [op.to_string()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

fn result_is_uncertain(result: &NoteRecordedResult) -> bool {
    match result {
        NoteRecordedResult::Receipt(receipt) => receipt.filesystem_outcome == FileOutcome::Unknown,
        NoteRecordedResult::Failure(error) => {
            error.filesystem_outcome == FileOutcome::Unknown
                || error.code == NoteErrorCode::SaveUncertain
        }
    }
}

fn protected_dependency(conn: &Connection, op: Uuid, file_state: Uuid) -> NoteResult<bool> {
    let mut stmt = conn.prepare(
        "SELECT operation_id FROM note_save_intents WHERE resolution IN ('unresolved','accepted_current')",
    )?;
    let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
    for row in ids {
        let other = require_intent(conn, parse_id(row?)?)?;
        if other.acknowledged_by == Some(op)
            || other.request.expected.file_state == file_state
                && recovery(conn, other.request.note_id)?
                    .is_none_or(|current| current.stamp.file_state != file_state)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn artifact_is_expected(value: &NoteSaveIntent, artifact: &RetainedArtifact) -> bool {
    match &value.expected_destination {
        DestinationPrecondition::Existing { fingerprint, .. } => {
            artifact.relative == value.staging_relative
                && artifact.identity.kind == ArtifactKind::Regular
                && artifact.identity.device == fingerprint.device
                && artifact.identity.inode == fingerprint.inode
                && artifact.identity.len == fingerprint.len
                && artifact.sha256 == Some(fingerprint.sha256)
        }
        _ => false,
    }
}
fn installed(verification: &NoteVerification) -> &FileFingerprint {
    match verification {
        NoteVerification::Replace { installed, .. } | NoteVerification::Copy { installed } => {
            installed
        }
    }
}
fn verify(value: &NoteSaveIntent, verification: &NoteVerification) -> NoteResult<()> {
    let staged = value
        .staged
        .as_ref()
        .ok_or_else(|| failure(NoteErrorCode::SaveUncertain, "prepared identity is missing"))?;
    if installed(verification) != &staged.fingerprint
        || !matches_text(installed(verification), &value.request.text)
    {
        return Err(failure(
            NoteErrorCode::SaveUncertain,
            "installed identity does not match prepared bytes",
        ));
    }
    match (value.kind, verification) {
        (
            NoteWriteKind::Replace,
            NoteVerification::Replace {
                displaced,
                displaced_bytes,
                ..
            },
        ) => {
            let DestinationPrecondition::Existing { baseline_text, .. } =
                &value.expected_destination
            else {
                return Err(failure(
                    NoteErrorCode::Storage,
                    "invalid original precondition",
                ));
            };
            if !artifact_is_expected(value, displaced)
                || displaced_bytes != baseline_text.as_bytes()
                || value.displaced.as_ref().is_some_and(|old| old != displaced)
            {
                return Err(failure(
                    NoteErrorCode::Conflict,
                    "displaced object differs from original",
                ));
            }
        }
        (NoteWriteKind::Copy, NoteVerification::Copy { .. }) => {}
        _ => {
            return Err(failure(
                NoteErrorCode::StateChanged,
                "verification kind does not match write",
            ));
        }
    }
    Ok(())
}
fn persist_verification(
    tx: &Transaction<'_>,
    op: Uuid,
    verification: &NoteVerification,
) -> NoteResult<()> {
    let bytes = json(verification)?;
    tx.execute("UPDATE note_save_intents SET verification_json=?2,verification_sha256=?3 WHERE operation_id=?1",
        params![op.to_string(),bytes,hash(&bytes).as_slice()])?;
    Ok(())
}
fn load_verification(conn: &Connection, op: Uuid) -> NoteResult<Option<NoteVerification>> {
    let (bytes, digest): (Option<Vec<u8>>, Option<Vec<u8>>) = conn.query_row(
        "SELECT verification_json,verification_sha256 FROM note_save_intents WHERE operation_id=?1",
        [op.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    match (bytes, digest) {
        (Some(b), Some(h)) => decode(b, h).map(Some),
        (None, None) => Ok(None),
        _ => Err(failure(
            NoteErrorCode::Storage,
            "verification hash is missing",
        )),
    }
}
fn commit_reconciliation(
    tx: &Transaction<'_>,
    mut value: NoteSaveIntent,
    record: &NoteReconciliation,
) -> NoteResult<NoteRecordedResult> {
    let op = value.request.operation_id;
    if record.resolution == NoteResolution::AcceptedCurrent {
        return Err(failure(
            NoteErrorCode::StateChanged,
            "accepted-current requires a separate acknowledgement",
        ));
    }
    if record.resolution == NoteResolution::NotApplied
        && !matches!(value.phase, SavePhase::Intent | SavePhase::Prepared)
    {
        return Err(failure(
            NoteErrorCode::SaveUncertain,
            "original or absent destination cannot disprove a recorded exchange",
        ));
    }
    match record.resolution {
        NoteResolution::Applied => {
            let verification = record.verification.as_ref().ok_or_else(|| {
                failure(
                    NoteErrorCode::SaveUncertain,
                    "applied reconciliation requires verification",
                )
            })?;
            verify(&value, verification)?;
            if record.observed_destination.as_ref() != Some(installed(verification)) {
                return Err(failure(
                    NoteErrorCode::SaveUncertain,
                    "observed destination lacks installed identity",
                ));
            }
            if let Some(old) = load_verification(tx, op)?
                && old != *verification
            {
                return Err(failure(
                    NoteErrorCode::Conflict,
                    "persisted verification differs",
                ));
            }
            persist_verification(tx, op, verification)?;
        }
        NoteResolution::NotApplied => match &value.expected_destination {
            DestinationPrecondition::Existing { fingerprint, .. }
                if record.observed_destination.as_ref() == Some(fingerprint)
                    && record.verification.is_none() => {}
            DestinationPrecondition::Absent { .. }
                if record.observed_destination.is_none() && record.verification.is_none() => {}
            _ => {
                return Err(failure(
                    NoteErrorCode::SaveUncertain,
                    "not-applied reconciliation lacks original/absence proof",
                ));
            }
        },
        NoteResolution::Unresolved => {
            if !matches!(&record.result,NoteRecordedResult::Failure(error) if error.filesystem_outcome == FileOutcome::Unknown || error.code == NoteErrorCode::Conflict || error.code == NoteErrorCode::SaveUncertain)
            {
                return Err(failure(
                    NoteErrorCode::StateChanged,
                    "unresolved reconciliation requires uncertain failure",
                ));
            }
        }
        NoteResolution::AcceptedCurrent => unreachable!(),
    }
    if let Some(prior) = &value.prior_result {
        // A recorded refusal is immutable even when later execution proof becomes available.
        if prior != &record.result {
            return Err(failure(
                NoteErrorCode::OperationConflict,
                "reconciliation cannot replace an existing result",
            ));
        }
    }
    match &record.result {
        NoteRecordedResult::Receipt(receipt) => {
            let expected_outcome = match record.resolution {
                NoteResolution::Applied => FileOutcome::Applied,
                NoteResolution::NotApplied => FileOutcome::NotApplied,
                _ => {
                    return Err(failure(
                        NoteErrorCode::StateChanged,
                        "unresolved operation cannot return success",
                    ));
                }
            };
            if receipt.operation_id != op
                || receipt.source_note_id != value.request.note_id
                || receipt.note_id != value.target_note_id
                || receipt.submitted_generation != value.request.generation
                || receipt.stamp.generation != value.request.generation
                || receipt.filesystem_outcome != expected_outcome
                || !receipt.recovery_available
            {
                return Err(failure(
                    NoteErrorCode::StateChanged,
                    "receipt does not acknowledge bound submission",
                ));
            }
            if record.resolution == NoteResolution::NotApplied {
                if receipt.stamp.file_state != value.request.expected.file_state
                    || value.kind == NoteWriteKind::Copy
                {
                    return Err(failure(
                        NoteErrorCode::StateChanged,
                        "not-applied receipt must retain original identity",
                    ));
                }
            } else {
                if receipt.stamp.file_state == value.request.expected.file_state {
                    return Err(failure(
                        NoteErrorCode::StateChanged,
                        "applied receipt requires a new file state",
                    ));
                }
                apply_registry(
                    tx,
                    &value,
                    receipt.stamp,
                    record.observed_destination.as_ref().unwrap(),
                )?;
            }
        }
        NoteRecordedResult::Failure(error) => {
            if error.operation_id != Some(op)
                || error.note_id != Some(value.request.note_id)
                || !error.recovery_available
            {
                return Err(failure(
                    NoteErrorCode::StateChanged,
                    "failure does not acknowledge protected input",
                ));
            }
            if value.prior_result.is_none()
                && (error.phase != Some(value.phase)
                    || matches!(record.resolution, NoteResolution::NotApplied)
                        && error.filesystem_outcome != FileOutcome::NotApplied
                    || matches!(record.resolution, NoteResolution::Applied)
                        && error.filesystem_outcome != FileOutcome::Applied)
            {
                return Err(failure(
                    NoteErrorCode::StateChanged,
                    "new failure result disagrees with reconciliation proof",
                ));
            }
            if record.resolution == NoteResolution::Applied {
                let stamp = NoteStamp {
                    file_state: Uuid::new_v4(),
                    generation: value.request.generation,
                };
                apply_registry(
                    tx,
                    &value,
                    stamp,
                    record.observed_destination.as_ref().unwrap(),
                )?;
            }
        }
    }
    if record.resolution == NoteResolution::Applied {
        let baseline = match &value.expected_destination {
            DestinationPrecondition::Existing { baseline_text, .. } => baseline_text.as_str(),
            DestinationPrecondition::Absent { .. } => "",
        };
        tx.execute("INSERT INTO note_recovery_pairs(note_id,operation_id,baseline,baseline_sha256,submitted,submitted_sha256) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(note_id) DO UPDATE SET operation_id=excluded.operation_id,baseline=excluded.baseline,baseline_sha256=excluded.baseline_sha256,submitted=excluded.submitted,submitted_sha256=excluded.submitted_sha256",
            params![value.target_note_id.to_string(),op.to_string(),baseline.as_bytes(),hash(baseline.as_bytes()).as_slice(),value.request.text.as_bytes(),hash(value.request.text.as_bytes()).as_slice()])?;
    }
    if value.prior_result.is_none() {
        insert_result(
            tx,
            "note_receipts",
            op,
            value.target_note_id,
            &record.result,
        )?;
    }
    value.resolution = record.resolution;
    if record.resolution != NoteResolution::Unresolved {
        value.phase = SavePhase::Complete;
    }
    update_intent(tx, &value)?;
    let status = match &record.result {
        NoteRecordedResult::Receipt(_) => "completed",
        NoteRecordedResult::Failure(_) => "failed",
    };
    tx.execute(
        "UPDATE operations SET status=?2 WHERE id=?1",
        params![op.to_string(), status],
    )?;
    Ok(record.result.clone())
}

fn apply_registry(
    tx: &Transaction<'_>,
    value: &NoteSaveIntent,
    stamp: NoteStamp,
    file: &FileFingerprint,
) -> NoteResult<()> {
    let bytes = json(file)?;
    if value.kind == NoteWriteKind::Copy {
        let vault: String = tx.query_row(
            "SELECT vault_id FROM notes WHERE id=?1",
            [value.request.note_id.to_string()],
            |r| r.get(0),
        )?;
        tx.execute("INSERT INTO notes(id,vault_id,relative,file_state,fingerprint_json,fingerprint_sha256,observed_file_state,observed_fingerprint_json,observed_fingerprint_sha256) VALUES(?1,?2,?3,?4,?5,?6,?4,?5,?6)",
            params![value.target_note_id.to_string(),vault,relative(&value.destination)?,stamp.file_state.to_string(),bytes,hash(&bytes).as_slice()])?;
        insert_buffer(
            tx,
            value.target_note_id,
            stamp,
            &value.request.text,
            &value.request.text,
        )?;
    } else {
        let current = recovery(tx, value.request.note_id)?
            .ok_or_else(|| failure(NoteErrorCode::Storage, "note lost its buffer"))?;
        if current.stamp.file_state != value.request.expected.file_state
            || current.stamp.generation < value.request.generation
        {
            return Err(failure(
                NoteErrorCode::StateChanged,
                "note baseline changed during save",
            ));
        }
        tx.execute("UPDATE notes SET file_state=?2,fingerprint_json=?3,fingerprint_sha256=?5,observed_file_state=?2,observed_fingerprint_json=?3,observed_fingerprint_sha256=?5,content_epoch=content_epoch+CASE WHEN ?4 THEN 1 ELSE 0 END,approval_epoch=approval_epoch+CASE WHEN ?4 AND approval='approved' THEN 1 ELSE 0 END,approval=CASE WHEN ?4 THEN 'draft' ELSE approval END WHERE id=?1",
            params![value.target_note_id.to_string(),stamp.file_state.to_string(),bytes, value.request.text != current.baseline,hash(&bytes).as_slice()])?;
        tx.execute(
            "UPDATE note_buffers SET file_state=?2,baseline=?3,baseline_sha256=?4 WHERE note_id=?1",
            params![
                value.target_note_id.to_string(),
                stamp.file_state.to_string(),
                value.request.text.as_bytes(),
                hash(value.request.text.as_bytes()).as_slice()
            ],
        )?;
    }
    Ok(())
}
