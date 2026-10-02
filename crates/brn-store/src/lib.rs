//! SQLite authority for local BRN records. A `Store` owns the data directory lock
//! and a single connection; writes require `&mut self` and a durable operation ID.
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use uuid::Uuid;
pub mod anchors;
mod comments;
mod drafts;
pub mod notes;
pub mod work;
mod workflow;
pub use anchors::{
    AmbiguityReason, AnchorProjection, AnchorState, EditTrace, OriginalAnchor, RecoveryReference,
    TextEdit, apply_edit, derive_edit, map_anchor, replay_trace,
};
pub use comments::{
    CommentAnchorSnapshot, CommentCapture, CommentCreated, CommentStatus, CommentStatusChange,
    CommentStatusChanged, DraftComment, DraftCommentView, DraftComments, DraftWriteWithComments,
    MAX_COMMENT_BODY_BYTES,
};
pub use drafts::{Draft, DraftRevision, DraftStamp, MAX_DRAFT_BYTES, RevisionKind};
pub use work::{MAX_NOTE_BYTES, OpenReport, UnsavedEdit, WorkStore};
pub use workflow::{Approval, ChatTurn, EvidenceCurrentness, ImportResult, SourceDocument};

const APPLICATION_ID: u32 = 0x4252_4e31; // BRN1
const SCHEMA_VERSION: u32 = 6;
use notes::V6;
const V1: &str = "CREATE TABLE sources (id TEXT PRIMARY KEY, title TEXT NOT NULL);\
CREATE TABLE versions (id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES sources(id), parent_id TEXT REFERENCES versions(id), bytes BLOB NOT NULL, sha256 BLOB NOT NULL);";
const V2: &str = "CREATE TABLE operations (id TEXT PRIMARY KEY, kind TEXT NOT NULL, payload_hash BLOB NOT NULL, status TEXT NOT NULL CHECK(status IN ('pending','running','completed','failed','interrupted')), terminal_data BLOB);\
CREATE TABLE sessions (id TEXT PRIMARY KEY, provider TEXT NOT NULL, provider_store TEXT NOT NULL, provider_account TEXT, thread_id TEXT, metadata BLOB NOT NULL);\
CREATE TABLE messages (id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES sessions(id), ordinal INTEGER NOT NULL, role TEXT NOT NULL, content BLOB NOT NULL, UNIQUE(session_id,ordinal));\
CREATE TABLE operation_results (operation_id TEXT NOT NULL REFERENCES operations(id), action TEXT NOT NULL, args_hash BLOB NOT NULL, entity_id TEXT NOT NULL, PRIMARY KEY(operation_id,action));";
const V3: &str = "ALTER TABLE sources ADD COLUMN origin TEXT;\
ALTER TABLE sources ADD COLUMN current_version_id TEXT REFERENCES versions(id);\
ALTER TABLE sources ADD COLUMN approval TEXT NOT NULL DEFAULT 'draft' CHECK(approval IN ('approved','draft','withdrawn'));\
CREATE UNIQUE INDEX source_origin_unique ON sources(origin);\
UPDATE sources SET current_version_id=(SELECT id FROM versions WHERE versions.source_id=sources.id ORDER BY rowid DESC LIMIT 1);\
CREATE TABLE imports (operation_id TEXT PRIMARY KEY REFERENCES operations(id), source_id TEXT NOT NULL REFERENCES sources(id), version_id TEXT NOT NULL REFERENCES versions(id), changed INTEGER NOT NULL CHECK(changed IN (0,1)));\
CREATE TABLE chat_turns (operation_id TEXT PRIMARY KEY REFERENCES operations(id), session_id TEXT NOT NULL REFERENCES sessions(id), question TEXT NOT NULL, profile TEXT NOT NULL, evidence_json TEXT NOT NULL, answer TEXT, provider_turn_id TEXT, usage_json TEXT);";
const V4: &str = "CREATE TABLE drafts (id TEXT PRIMARY KEY, title TEXT NOT NULL, base_revision_id TEXT NOT NULL, generation INTEGER NOT NULL CHECK(generation >= 0), text TEXT NOT NULL, sha256 BLOB NOT NULL);\
CREATE TABLE draft_revisions (id TEXT PRIMARY KEY, draft_id TEXT NOT NULL REFERENCES drafts(id), parent_id TEXT REFERENCES draft_revisions(id), kind TEXT NOT NULL CHECK(kind IN ('checkpoint','candidate')), text TEXT NOT NULL, sha256 BLOB NOT NULL, origin_turn TEXT REFERENCES chat_turns(operation_id));\
CREATE TABLE draft_results (operation_id TEXT PRIMARY KEY REFERENCES operations(id), result_kind TEXT NOT NULL CHECK(result_kind IN ('draft','revision')), result_json BLOB NOT NULL);";
const V5: &str = "CREATE TABLE draft_comments (id TEXT PRIMARY KEY, draft_id TEXT NOT NULL REFERENCES drafts(id), original_revision_id TEXT NOT NULL REFERENCES draft_revisions(id), original_sha256 BLOB NOT NULL, original_start INTEGER NOT NULL CHECK(original_start >= 0), original_end INTEGER NOT NULL CHECK(original_end > original_start), original_quote TEXT NOT NULL, body TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('open','resolved')), status_version INTEGER NOT NULL CHECK(status_version >= 0));\
CREATE TABLE draft_comment_anchors (comment_id TEXT PRIMARY KEY REFERENCES draft_comments(id), target_base_revision_id TEXT NOT NULL REFERENCES draft_revisions(id), target_generation INTEGER NOT NULL CHECK(target_generation >= 0), target_sha256 BLOB NOT NULL, location TEXT NOT NULL CHECK(location IN ('anchored','deleted','ambiguous')), reason TEXT, start INTEGER, end INTEGER, CHECK((location='anchored' AND reason IS NULL AND start IS NOT NULL AND end IS NOT NULL AND start >= 0 AND end > start) OR (location='deleted' AND reason IS NULL AND start IS NULL AND end IS NULL) OR (location='ambiguous' AND reason IS NOT NULL AND reason IN ('touched','duplicate','missing_unsupported','boundary_ambiguity','conflicting_snapshot','history_limit') AND start IS NULL AND end IS NULL)));\
CREATE TABLE draft_revision_comment_anchors (comment_id TEXT NOT NULL REFERENCES draft_comments(id), revision_id TEXT NOT NULL REFERENCES draft_revisions(id), location TEXT NOT NULL CHECK(location IN ('anchored','deleted','ambiguous')), reason TEXT, start INTEGER, end INTEGER, PRIMARY KEY(comment_id,revision_id), CHECK((location='anchored' AND reason IS NULL AND start IS NOT NULL AND end IS NOT NULL AND start >= 0 AND end > start) OR (location='deleted' AND reason IS NULL AND start IS NULL AND end IS NULL) OR (location='ambiguous' AND reason IS NOT NULL AND reason IN ('touched','duplicate','missing_unsupported','boundary_ambiguity','conflicting_snapshot','history_limit') AND start IS NULL AND end IS NULL)));\
CREATE TABLE comment_results (operation_id TEXT PRIMARY KEY REFERENCES operations(id), result_kind TEXT NOT NULL CHECK(result_kind IN ('capture','status','write')), result_json BLOB NOT NULL, result_sha256 BLOB NOT NULL CHECK(length(result_sha256)=32));";

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Sql(rusqlite::Error),
    Invalid(String),
    /// The data directory lock is held by another live process.
    WorkspaceBusy(String),
    /// A durable operation ID was reused with different kind or payload.
    OperationConflict(String),
    /// A requested durable record does not exist.
    NotFound(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O: {e}"),
            Self::Sql(e) => write!(f, "SQLite: {e}"),
            Self::Invalid(e)
            | Self::WorkspaceBusy(e)
            | Self::OperationConflict(e)
            | Self::NotFound(e) => f.write_str(e),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn parse_id(s: String) -> Result<Uuid> {
    Uuid::parse_str(&s).map_err(|_| invalid("invalid stored UUID"))
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    pub interrupted_operations: usize,
    pub migrated_from: Option<u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeginOperation {
    New,
    Existing,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Interrupted,
}
impl OperationStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
        }
    }
    fn parse(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "interrupted" => Ok(Self::Interrupted),
            _ => Err(invalid("invalid operation status")),
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct Operation {
    pub id: Uuid,
    pub kind: String,
    pub payload_hash: [u8; 32],
    pub status: OperationStatus,
    pub terminal_data: Option<Vec<u8>>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Version {
    pub id: Uuid,
    pub source_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub bytes: Vec<u8>,
    pub sha256: [u8; 32],
}
#[derive(Debug, PartialEq, Eq)]
pub struct Session {
    pub id: Uuid,
    pub provider: String,
    pub provider_store: String,
    pub provider_account: Option<String>,
    pub thread_id: Option<String>,
    pub metadata: Vec<u8>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub ordinal: i64,
    pub role: String,
    pub content: Vec<u8>,
}

pub struct Store {
    conn: Connection,
    _owner_lock: File,
    db_path: PathBuf,
}
/// Acquires the owner lock: `WouldBlock` is genuine contention (including
/// the transient fork+exec window) and is retried until `deadline`; any other
/// lock failure is an I/O error and returns immediately. Classification is
/// the enum variant, never the error text.
fn acquire_owner_lock(
    mut attempt: impl FnMut() -> std::result::Result<(), std::fs::TryLockError>,
    deadline: std::time::Instant,
) -> Result<()> {
    loop {
        match attempt() {
            Ok(()) => return Ok(()),
            Err(e @ std::fs::TryLockError::WouldBlock) => {
                if std::time::Instant::now() >= deadline {
                    return Err(Error::WorkspaceBusy(format!(
                        "data directory is already owned: {e}"
                    )));
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(std::fs::TryLockError::Error(e)) => return Err(Error::Io(e)),
        }
    }
}

impl Store {
    /// Opens an existing directory. Creates the database once if absent; never replaces an existing file.
    pub fn open(data_dir: impl AsRef<Path>) -> Result<(Self, RecoveryReport)> {
        let dir = data_dir.as_ref();
        if !dir.is_dir() {
            return Err(invalid("data directory must already exist"));
        }
        let lock_path = dir.join("brn.owner.lock");
        if lock_path.exists() {
            check_regular_single_link(&lock_path)?;
        }
        let owner = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)?;
        check_regular_single_link(&lock_path)?;
        // A concurrent fork+exec in this process (subprocess spawn) briefly
        // duplicates this open lock descriptor into the child until exec
        // closes it, so a lock released moments ago can transiently report
        // busy. Transient busy is WouldBlock and is retried within a short
        // bounded grace before reporting ownership; any non-contention lock
        // failure is an I/O error and surfaces immediately. Exclusivity
        // itself is never weakened.
        let lock_deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        acquire_owner_lock(|| owner.try_lock(), lock_deadline)?;
        let db_path = dir.join("brn.sqlite3");
        let exists = db_path.exists();
        let prior = if exists {
            check_regular_single_link(&db_path)?;
            Some(preflight(&db_path)?)
        } else {
            None
        };
        if !exists {
            for suffix in ["-journal", "-wal", "-shm"] {
                if dir.join(format!("brn.sqlite3{suffix}")).exists() {
                    return Err(invalid("orphan SQLite sidecar without database"));
                }
            }
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&db_path)?;
        }
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let mut conn = Connection::open_with_flags(&db_path, flags)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "trusted_schema", "OFF")?;
        let mut report = RecoveryReport::default();
        let recovered_version = if prior.is_some() {
            validate_integrity(&conn)?;
            let application: i64 = conn.query_row("PRAGMA application_id", [], |r| r.get(0))?;
            let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
            if application != i64::from(APPLICATION_ID)
                || !(1..=i64::from(SCHEMA_VERSION)).contains(&version)
            {
                return Err(invalid("unsupported recovered database header"));
            }
            validate_schema(&conn, version as u32)?;
            Some(version as u32)
        } else {
            None
        };
        configure_durability(&conn)?;
        match recovered_version {
            None => {
                let tx = conn.transaction()?;
                tx.execute_batch(V1)?;
                tx.execute_batch(V2)?;
                tx.execute_batch(V3)?;
                tx.execute_batch(V4)?;
                tx.execute_batch(V5)?;
                tx.execute_batch(V6)?;
                tx.pragma_update(None, "application_id", APPLICATION_ID)?;
                tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
                tx.commit()?;
            }
            Some(version) => {
                if version == 1 {
                    migrate_v1(&mut conn, || Ok(()))?;
                    report.migrated_from = Some(1);
                } else if version == 2 {
                    migrate_v2(&mut conn)?;
                    report.migrated_from = Some(2);
                } else if version == 3 {
                    migrate_v3(&mut conn, || Ok(()))?;
                    report.migrated_from = Some(3);
                } else if version == 4 {
                    migrate_v4(&mut conn, || Ok(()))?;
                    report.migrated_from = Some(4);
                } else if version == 5 {
                    migrate_v5(&mut conn, || Ok(()))?;
                    report.migrated_from = Some(5);
                }
            }
        }
        validate_integrity(&conn)?;
        validate_schema(&conn, SCHEMA_VERSION)?;
        // Interrupted work has no implicit replay. Reconciliation is an explicit caller choice.
        let tx = conn.transaction()?;
        report.interrupted_operations = tx.execute(
            "UPDATE operations SET status='interrupted' WHERE status IN ('pending','running')",
            [],
        )?;
        tx.commit()?;
        Ok((
            Self {
                conn,
                _owner_lock: owner,
                db_path,
            },
            report,
        ))
    }
    pub fn database_path(&self) -> &Path {
        &self.db_path
    }
    pub fn begin_operation(
        &mut self,
        id: Uuid,
        kind: &str,
        payload: &[u8],
    ) -> Result<BeginOperation> {
        if kind.is_empty() || is_local_kind(kind) {
            return Err(invalid("invalid external operation kind"));
        }
        let digest = hash(payload);
        let tx = self.conn.transaction()?;
        let current: Option<(String, Vec<u8>)> = tx
            .query_row(
                "SELECT kind,payload_hash FROM operations WHERE id=?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let result = match current {
            Some((old_kind, old_hash)) if old_kind == kind && old_hash == digest => {
                BeginOperation::Existing
            }
            Some(_) => {
                return Err(Error::OperationConflict(
                    "operation ID conflicts with existing kind or payload".into(),
                ));
            }
            None => {
                tx.execute("INSERT INTO operations(id,kind,payload_hash,status) VALUES(?1,?2,?3,'pending')",params![id.to_string(),kind,digest.as_slice()])?;
                BeginOperation::New
            }
        };
        tx.commit()?;
        Ok(result)
    }
    pub fn operation(&self, id: Uuid) -> Result<Option<Operation>> {
        type OperationRow = (String, Vec<u8>, String, Option<Vec<u8>>);
        let row: Option<OperationRow> = self
            .conn
            .query_row(
                "SELECT kind,payload_hash,status,terminal_data FROM operations WHERE id=?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        row.map(|(kind, h, status, data)| {
            Ok(Operation {
                id,
                kind,
                payload_hash: h
                    .try_into()
                    .map_err(|_| invalid("invalid stored operation hash"))?,
                status: OperationStatus::parse(&status)?,
                terminal_data: data,
            })
        })
        .transpose()
    }
    pub fn mark_running(&mut self, id: Uuid) -> Result<()> {
        self.transition(id, OperationStatus::Running, None)
    }
    pub fn finish_operation(
        &mut self,
        id: Uuid,
        status: OperationStatus,
        data: &[u8],
    ) -> Result<()> {
        if self
            .operation(id)?
            .is_some_and(|op| op.kind.starts_with("note."))
        {
            return Err(invalid("note operations require dedicated transitions"));
        }
        if !matches!(
            status,
            OperationStatus::Completed | OperationStatus::Failed | OperationStatus::Interrupted
        ) {
            return Err(invalid("finish requires terminal status"));
        }
        self.transition(id, status, Some(data))
    }
    fn transition(&mut self, id: Uuid, next: OperationStatus, data: Option<&[u8]>) -> Result<()> {
        let tx = self.conn.transaction()?;
        let old: Option<(String, String, Option<Vec<u8>>)> = tx
            .query_row(
                "SELECT kind,status,terminal_data FROM operations WHERE id=?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((kind, status, old_data)) = old else {
            return Err(invalid("operation does not exist"));
        };
        if kind == "chat.turn" {
            return Err(invalid("chat turns require workflow transitions"));
        }
        let old = OperationStatus::parse(&status)?;
        if old == next && (old == OperationStatus::Running || old_data.as_deref() == data) {
            return Ok(());
        }
        if !matches!(old, OperationStatus::Pending | OperationStatus::Running)
            || next == OperationStatus::Pending
            || old == OperationStatus::Pending
                && next == OperationStatus::Completed
                && data.is_none()
        {
            return Err(invalid("invalid operation state transition"));
        }
        tx.execute(
            "UPDATE operations SET status=?2,terminal_data=?3 WHERE id=?1",
            params![id.to_string(), next.as_str(), data],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn create_source(&mut self, op: Uuid, title: &str) -> Result<Uuid> {
        self.write_result(
            op,
            "source.create",
            &encode_args(&[title.as_bytes()]),
            |tx, id| {
                tx.execute(
                    "INSERT INTO sources(id,title) VALUES(?1,?2)",
                    params![id.to_string(), title],
                )?;
                Ok(())
            },
        )
    }
    pub fn source_title(&self, id: Uuid) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT title FROM sources WHERE id=?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn sources(&self) -> Result<Vec<(Uuid, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id,title FROM sources ORDER BY rowid")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.map(|row| {
            let (id, title) = row?;
            Ok((parse_id(id)?, title))
        })
        .collect()
    }
    pub fn versions(&self, source: Uuid) -> Result<Vec<Version>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM versions WHERE source_id=?1 ORDER BY rowid")?;
        let ids: Vec<String> = stmt
            .query_map([source.to_string()], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        ids.into_iter()
            .map(|id| {
                self.version(parse_id(id)?)?
                    .ok_or_else(|| invalid("version vanished during read"))
            })
            .collect()
    }
    pub fn interrupted_operations(&self) -> Result<Vec<Uuid>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM operations WHERE status='interrupted' ORDER BY rowid")?;
        let ids: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        ids.into_iter().map(parse_id).collect()
    }
    pub fn add_version(
        &mut self,
        op: Uuid,
        source: Uuid,
        parent: Option<Uuid>,
        bytes: &[u8],
    ) -> Result<Uuid> {
        if std::str::from_utf8(bytes).is_err() {
            return Err(invalid("version must be UTF-8"));
        }
        let parent_tag = [u8::from(parent.is_some())];
        let args = encode_args(&[
            source.as_bytes(),
            &parent_tag,
            parent.unwrap_or(Uuid::nil()).as_bytes(),
            bytes,
        ]);
        let digest = hash(bytes);
        self.write_result(op, "version.add", &args, |tx, id| {
            if let Some(parent) = parent {
                let parent_source: Option<String> = tx
                    .query_row(
                        "SELECT source_id FROM versions WHERE id=?1",
                        [parent.to_string()],
                        |r| r.get(0),
                    )
                    .optional()?;
                if parent_source.as_deref() != Some(&source.to_string()) {
                    return Err(invalid("parent version must belong to source"));
                }
            }
            tx.execute(
                "INSERT INTO versions(id,source_id,parent_id,bytes,sha256) VALUES(?1,?2,?3,?4,?5)",
                params![
                    id.to_string(),
                    source.to_string(),
                    parent.map(|p| p.to_string()),
                    bytes,
                    digest.as_slice()
                ],
            )?;
            Ok(())
        })
    }
    pub fn version(&self, id: Uuid) -> Result<Option<Version>> {
        type VersionRow = (String, Option<String>, Vec<u8>, Vec<u8>);
        let row: Option<VersionRow> = self
            .conn
            .query_row(
                "SELECT source_id,parent_id,bytes,sha256 FROM versions WHERE id=?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        row.map(|(source, parent, bytes, digest)| {
            if std::str::from_utf8(&bytes).is_err() || hash(&bytes).as_slice() != digest {
                return Err(invalid("version content hash mismatch or invalid UTF-8"));
            }
            Ok(Version {
                id,
                source_id: parse_id(source)?,
                parent_id: parent.map(parse_id).transpose()?,
                bytes,
                sha256: digest
                    .try_into()
                    .map_err(|_| invalid("invalid version hash"))?,
            })
        })
        .transpose()
    }
    pub fn create_session(
        &mut self,
        op: Uuid,
        provider: &str,
        provider_store: &str,
        provider_account: Option<&str>,
        thread_id: Option<&str>,
        metadata: &[u8],
    ) -> Result<Uuid> {
        if provider.is_empty() || provider_store.is_empty() {
            return Err(invalid("provider and store association required"));
        }
        let account_tag = [u8::from(provider_account.is_some())];
        let thread_tag = [u8::from(thread_id.is_some())];
        let args = encode_args(&[
            provider.as_bytes(),
            provider_store.as_bytes(),
            &account_tag,
            provider_account.unwrap_or("").as_bytes(),
            &thread_tag,
            thread_id.unwrap_or("").as_bytes(),
            metadata,
        ]);
        self.write_result(op,"session.create",&args,|tx,id| {tx.execute("INSERT INTO sessions(id,provider,provider_store,provider_account,thread_id,metadata) VALUES(?1,?2,?3,?4,?5,?6)",params![id.to_string(),provider,provider_store,provider_account,thread_id,metadata])?;Ok(())})
    }
    pub fn sessions(&self) -> Result<Vec<Session>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM sessions ORDER BY rowid")?;
        let ids: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        ids.into_iter()
            .map(|id| {
                self.session(parse_id(id)?)?
                    .ok_or_else(|| invalid("session vanished during read"))
            })
            .collect()
    }
    pub fn session(&self, id: Uuid) -> Result<Option<Session>> {
        Ok(self.conn.query_row("SELECT provider,provider_store,provider_account,thread_id,metadata FROM sessions WHERE id=?1",[id.to_string()],|r|Ok(Session{id,provider:r.get(0)?,provider_store:r.get(1)?,provider_account:r.get(2)?,thread_id:r.get(3)?,metadata:r.get(4)?})).optional()?)
    }
    pub fn add_message(
        &mut self,
        op: Uuid,
        session: Uuid,
        role: &str,
        content: &[u8],
    ) -> Result<Uuid> {
        if role.is_empty() || std::str::from_utf8(content).is_err() {
            return Err(invalid("message role and UTF-8 content required"));
        }
        let args = encode_args(&[session.as_bytes(), role.as_bytes(), content]);
        self.write_result(op,"message.add",&args,|tx,id| {tx.execute("INSERT INTO messages(id,session_id,ordinal,role,content) VALUES(?1,?2,(SELECT coalesce(max(ordinal)+1,0) FROM messages WHERE session_id=?2),?3,?4)",params![id.to_string(),session.to_string(),role,content])?;Ok(())})
    }
    pub fn messages(&self, session: Uuid) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,ordinal,role,content FROM messages WHERE session_id=?1 ORDER BY ordinal",
        )?;
        let rows = stmt.query_map([session.to_string()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Vec<u8>>(3)?,
            ))
        })?;
        rows.map(|row| {
            let (id, ordinal, role, content) = row?;
            Ok(Message {
                id: parse_id(id)?,
                session_id: session,
                ordinal,
                role,
                content,
            })
        })
        .collect()
    }
    fn write_result(
        &mut self,
        op: Uuid,
        action: &str,
        args: &[u8],
        insert: impl FnOnce(&rusqlite::Transaction<'_>, Uuid) -> Result<()>,
    ) -> Result<Uuid> {
        let digest = hash(args);
        let tx = self.conn.transaction()?;
        let existing: Option<(String, Vec<u8>, String)> = tx
            .query_row(
                "SELECT kind,payload_hash,status FROM operations WHERE id=?1",
                [op.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let status = match existing {
            None => {
                tx.execute("INSERT INTO operations(id,kind,payload_hash,status) VALUES(?1,?2,?3,'pending')",params![op.to_string(),action,digest.as_slice()])?;
                "pending".to_string()
            }
            Some((kind, payload_hash, status)) if kind == action && payload_hash == digest => {
                status
            }
            Some(_) => {
                return Err(Error::OperationConflict(
                    "operation ID conflicts with local mutation".into(),
                ));
            }
        };
        let old:Option<(Vec<u8>,String)>=tx.query_row("SELECT args_hash,entity_id FROM operation_results WHERE operation_id=?1 AND action=?2",params![op.to_string(),action],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        if let Some((old_hash, old_id)) = old {
            return if old_hash == digest {
                parse_id(old_id)
            } else {
                Err(invalid("operation write arguments conflict"))
            };
        }
        if !matches!(status.as_str(), "pending" | "running") {
            return Err(invalid("operation is not writable"));
        }
        let id = Uuid::new_v4();
        insert(&tx, id)?;
        tx.execute("INSERT INTO operation_results(operation_id,action,args_hash,entity_id) VALUES(?1,?2,?3,?4)",params![op.to_string(),action,digest.as_slice(),id.to_string()])?;
        tx.execute(
            "UPDATE operations SET status='completed' WHERE id=?1",
            [op.to_string()],
        )?;
        tx.commit()?;
        Ok(id)
    }
}
fn is_local_kind(kind: &str) -> bool {
    matches!(
        kind,
        "source.create"
            | "version.add"
            | "session.create"
            | "message.add"
            | "source.import"
            | "source.approval"
            | "session.attach"
            | "chat.turn"
            | "draft.create"
            | "draft.save"
            | "draft.checkpoint"
            | "draft.candidate"
            | "draft.write.comments"
            | "comment.capture"
            | "comment.status"
            | "note.enroll"
            | "note.buffer"
            | "note.save"
            | "note.copy"
    )
}
fn encode_args(items: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for item in items {
        out.extend_from_slice(&(item.len() as u64).to_be_bytes());
        out.extend_from_slice(item);
    }
    out
}
fn check_regular_single_link(path: &Path) -> Result<()> {
    let meta = path.symlink_metadata()?;
    if !meta.file_type().is_file() || meta.nlink() != 1 {
        return Err(invalid(
            "database or lock path is not an unaliased regular file",
        ));
    }
    Ok(())
}
fn preflight(path: &Path) -> Result<u32> {
    let mut f = File::open(path)?;
    let mut header = [0u8; 100];
    f.read_exact(&mut header)
        .map_err(|_| invalid("existing database has invalid header"))?;
    if &header[..16] != b"SQLite format 3\0" {
        return Err(invalid("existing database is not SQLite"));
    }
    let app = u32::from_be_bytes(header[68..72].try_into().unwrap());
    let version = u32::from_be_bytes(header[60..64].try_into().unwrap());
    if app != APPLICATION_ID {
        return Err(invalid("foreign database application ID"));
    }
    if version == 0 || version > SCHEMA_VERSION {
        return Err(invalid("unsupported database schema version"));
    }
    f.seek(SeekFrom::Start(0))?;
    Ok(version)
}
fn validate_integrity(conn: &Connection) -> Result<()> {
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(invalid("database integrity check failed"));
    }
    let fk_count: i64 =
        conn.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })?;
    if fk_count != 0 {
        return Err(invalid("database foreign key check failed"));
    }
    Ok(())
}
fn migrate_v1(conn: &mut Connection, after_ddl: impl FnOnce() -> Result<()>) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch(V2)?;
    tx.execute_batch(V3)?;
    tx.execute_batch(V4)?;
    tx.execute_batch(V5)?;
    tx.execute_batch(V6)?;
    after_ddl()?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}
fn migrate_v2(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch(V3)?;
    tx.execute_batch(V4)?;
    tx.execute_batch(V5)?;
    tx.execute_batch(V6)?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}
fn migrate_v3(conn: &mut Connection, after_ddl: impl FnOnce() -> Result<()>) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch(V4)?;
    tx.execute_batch(V5)?;
    tx.execute_batch(V6)?;
    after_ddl()?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}
fn migrate_v4(conn: &mut Connection, after_ddl: impl FnOnce() -> Result<()>) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch(V5)?;
    tx.execute_batch(V6)?;
    after_ddl()?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}
fn migrate_v5(conn: &mut Connection, after_ddl: impl FnOnce() -> Result<()>) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch(V6)?;
    after_ddl()?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}
fn schema_map(conn: &Connection) -> Result<BTreeMap<String, (String, String)>> {
    let mut stmt = conn
        .prepare("SELECT name,type,sql FROM sqlite_master WHERE substr(name,1,7) != 'sqlite_'")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    let mut map = BTreeMap::new();
    for row in rows {
        let (name, kind, sql) = row?;
        map.insert(name, (kind, sql));
    }
    Ok(map)
}
fn validate_schema(conn: &Connection, version: u32) -> Result<()> {
    let expected = Connection::open_in_memory()?;
    expected.execute_batch(V1)?;
    if version >= 2 {
        expected.execute_batch(V2)?;
    }
    if version >= 3 {
        expected.execute_batch(V3)?;
    }
    if version >= 4 {
        expected.execute_batch(V4)?;
    }
    if version >= 5 {
        expected.execute_batch(V5)?;
    }
    if version >= 6 {
        expected.execute_batch(V6)?;
    }
    if schema_map(conn)? != schema_map(&expected)? {
        return Err(invalid("unexpected database schema"));
    }
    Ok(())
}
fn configure_durability(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "DELETE")?;
    conn.pragma_update(None, "synchronous", "EXTRA")?;
    #[cfg(target_os = "macos")]
    conn.pragma_update(None, "fullfsync", "ON")?;
    verify_pragmas(conn)
}
fn verify_pragmas(conn: &Connection) -> Result<()> {
    for (name, want) in [
        ("foreign_keys", 1),
        ("trusted_schema", 0),
        ("synchronous", 3),
    ] {
        let got: i64 = conn.query_row(&format!("PRAGMA {name}"), [], |r| r.get(0))?;
        if got != want {
            return Err(invalid("required SQLite pragma unavailable"));
        }
    }
    let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
    if mode.to_lowercase() != "delete" {
        return Err(invalid("required SQLite journal mode unavailable"));
    }
    #[cfg(target_os = "macos")]
    {
        let full: i64 = conn.query_row("PRAGMA fullfsync", [], |r| r.get(0))?;
        if full != 1 {
            return Err(invalid("fullfsync unavailable"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_five_migration_is_atomic_and_defaults_existing_turn_currentness() {
        let dir = tempfile::tempdir_in(".").unwrap();
        let db = dir.path().join("brn.sqlite3");
        let mut conn = Connection::open(&db).unwrap();
        for ddl in [V1, V2, V3, V4, V5] {
            conn.execute_batch(ddl).unwrap();
        }
        conn.pragma_update(None, "application_id", APPLICATION_ID)
            .unwrap();
        conn.pragma_update(None, "user_version", 5).unwrap();
        let session = Uuid::new_v4();
        let op = Uuid::new_v4();
        conn.execute("INSERT INTO sessions(id,provider,provider_store,thread_id,metadata) VALUES(?1,'test','test','thread',x'7b7d')", [session.to_string()]).unwrap();
        conn.execute("INSERT INTO operations(id,kind,payload_hash,status) VALUES(?1,'chat.turn',zeroblob(32),'completed')", [op.to_string()]).unwrap();
        conn.execute("INSERT INTO chat_turns(operation_id,session_id,question,profile,evidence_json,answer) VALUES(?1,?2,'q','keyword','[]','a')", params![op.to_string(),session.to_string()]).unwrap();
        assert!(migrate_v5(&mut conn, || Err(invalid("injected V6 failure"))).is_err());
        validate_schema(&conn, 5).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            5
        );
        drop(conn);
        let (store, report) = Store::open(dir.path()).unwrap();
        assert_eq!(report.migrated_from, Some(5));
        let turn = &store.turns(session).unwrap()[0];
        assert_eq!(turn.answer.as_deref(), Some("a"));
        assert_eq!(turn.evidence_currentness, EvidenceCurrentness::Unqualified);
    }

    #[test]
    fn version_one_migrates_and_injected_failure_rolls_back_schema_and_version() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("brn.sqlite3");
        let mut conn = Connection::open(&db).unwrap();
        conn.execute_batch(V1).unwrap();
        conn.pragma_update(None, "application_id", APPLICATION_ID)
            .unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        let source = Uuid::new_v4();
        conn.execute(
            "INSERT INTO sources(id,title) VALUES(?1,'preserved')",
            [source.to_string()],
        )
        .unwrap();
        assert!(migrate_v1(&mut conn, || Err(invalid("injected migration failure"))).is_err());
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        validate_schema(&conn, 1).unwrap();
        drop(conn);
        let (store, report) = Store::open(dir.path()).unwrap();
        assert_eq!(report.migrated_from, Some(1));
        assert_eq!(
            store.source_title(source).unwrap().as_deref(),
            Some("preserved")
        );
    }
    #[test]
    fn version_two_migrates_without_losing_history_or_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("brn.sqlite3");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(V1).unwrap();
        conn.execute_batch(V2).unwrap();
        conn.pragma_update(None, "application_id", APPLICATION_ID)
            .unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
        let source = Uuid::new_v4();
        let version = Uuid::new_v4();
        let session = Uuid::new_v4();
        conn.execute(
            "INSERT INTO sources(id,title) VALUES(?1,'old')",
            [source.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO versions(id,source_id,parent_id,bytes,sha256) VALUES(?1,?2,NULL,?3,?4)",
            params![
                version.to_string(),
                source.to_string(),
                b"old",
                hash(b"old").as_slice()
            ],
        )
        .unwrap();
        conn.execute("INSERT INTO sessions(id,provider,provider_store,metadata) VALUES(?1,'codex','store-a',?2)",params![session.to_string(),b"{}"] ).unwrap();
        drop(conn);
        let (store, report) = Store::open(dir.path()).unwrap();
        assert_eq!(report.migrated_from, Some(2));
        assert_eq!(store.version(version).unwrap().unwrap().bytes, b"old");
        assert_eq!(store.source_title(source).unwrap().as_deref(), Some("old"));
        assert_eq!(
            store.session(session).unwrap().unwrap().provider_store,
            "store-a"
        );
    }
    #[test]
    fn version_three_migrates_atomically_and_preserves_rows() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("brn.sqlite3");
        let mut conn = Connection::open(&db).unwrap();
        conn.execute_batch(V1).unwrap();
        conn.execute_batch(V2).unwrap();
        conn.execute_batch(V3).unwrap();
        conn.pragma_update(None, "application_id", APPLICATION_ID)
            .unwrap();
        conn.pragma_update(None, "user_version", 3).unwrap();
        let source = Uuid::new_v4();
        let version = Uuid::new_v4();
        conn.execute(
            "INSERT INTO sources(id,title,origin,approval) VALUES(?1,'old','file-a','approved')",
            [source.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO versions(id,source_id,bytes,sha256) VALUES(?1,?2,?3,?4)",
            params![
                version.to_string(),
                source.to_string(),
                b"exact\r\n",
                hash(b"exact\r\n").as_slice()
            ],
        )
        .unwrap();
        conn.execute(
            "UPDATE sources SET current_version_id=?2 WHERE id=?1",
            params![source.to_string(), version.to_string()],
        )
        .unwrap();
        assert!(migrate_v3(&mut conn, || Err(invalid("injected migration failure"))).is_err());
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            3
        );
        validate_schema(&conn, 3).unwrap();
        drop(conn);
        let (store, report) = Store::open(dir.path()).unwrap();
        assert_eq!(report.migrated_from, Some(3));
        assert_eq!(store.version(version).unwrap().unwrap().bytes, b"exact\r\n");
        assert_eq!(store.documents().unwrap()[0].approval, Approval::Approved);
        assert!(store.drafts().unwrap().is_empty());
    }

    #[test]
    fn populated_version_four_migrates_atomically_and_retains_draft_result() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("brn.sqlite3");
        let mut conn = Connection::open(&db).unwrap();
        for ddl in [V1, V2, V3, V4] {
            conn.execute_batch(ddl).unwrap();
        }
        conn.pragma_update(None, "application_id", APPLICATION_ID)
            .unwrap();
        conn.pragma_update(None, "user_version", 4).unwrap();
        let draft = Uuid::new_v4();
        let root = Uuid::new_v4();
        let op = Uuid::new_v4();
        let text = "exact\r\n🙂";
        let digest = hash(text.as_bytes());
        conn.execute("INSERT INTO drafts(id,title,base_revision_id,generation,text,sha256) VALUES(?1,'old',?2,0,?3,?4)", params![draft.to_string(),root.to_string(),text,digest.as_slice()]).unwrap();
        conn.execute("INSERT INTO draft_revisions(id,draft_id,parent_id,kind,text,sha256) VALUES(?1,?2,NULL,'checkpoint',?3,?4)", params![root.to_string(),draft.to_string(),text,digest.as_slice()]).unwrap();
        conn.execute("INSERT INTO operations(id,kind,payload_hash,status) VALUES(?1,'draft.create',?2,'completed')", params![op.to_string(),hash(&encode_args(&[b"old",text.as_bytes()])).as_slice()]).unwrap();
        let old = Draft {
            id: draft,
            title: "old".into(),
            stamp: DraftStamp {
                base_revision: root,
                generation: 0,
            },
            text: text.into(),
            sha256: digest,
        };
        conn.execute(
            "INSERT INTO draft_results(operation_id,result_kind,result_json) VALUES(?1,'draft',?2)",
            params![op.to_string(), serde_json::to_vec(&old).unwrap()],
        )
        .unwrap();
        assert!(migrate_v4(&mut conn, || Err(invalid("injected after v5 DDL"))).is_err());
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            4
        );
        validate_schema(&conn, 4).unwrap();
        drop(conn);
        let (mut store, report) = Store::open(dir.path()).unwrap();
        assert_eq!(report.migrated_from, Some(4));
        assert_eq!(store.draft(draft).unwrap().unwrap(), old);
        assert_eq!(store.create_draft(op, "old", text).unwrap(), old);
        assert!(store.draft_comments(draft).unwrap().comments.is_empty());
    }

    #[test]
    fn owner_lock_succeeds_on_first_attempt() {
        use std::time::{Duration, Instant};
        let mut calls = 0;
        let result = acquire_owner_lock(
            || {
                calls += 1;
                Ok(())
            },
            Instant::now() + Duration::from_secs(1),
        );
        assert!(result.is_ok());
        assert_eq!(calls, 1);
    }

    #[test]
    fn owner_lock_retries_would_block_then_succeeds() {
        use std::fs::TryLockError;
        use std::time::{Duration, Instant};
        let mut calls = 0;
        let result = acquire_owner_lock(
            || {
                calls += 1;
                if calls < 4 {
                    Err(TryLockError::WouldBlock)
                } else {
                    Ok(())
                }
            },
            Instant::now() + Duration::from_secs(10),
        );
        assert!(result.is_ok());
        assert_eq!(calls, 4);
    }

    #[test]
    fn owner_lock_would_block_past_deadline_is_workspace_busy() {
        use std::fs::TryLockError;
        use std::time::Instant;
        let mut calls = 0;
        let err = acquire_owner_lock(
            || {
                calls += 1;
                Err(TryLockError::WouldBlock)
            },
            Instant::now(),
        )
        .unwrap_err();
        assert!(matches!(err, Error::WorkspaceBusy(ref msg) if msg.contains("already owned")));
        assert_eq!(calls, 1);
    }

    #[test]
    fn owner_lock_io_error_returns_immediately_without_retry() {
        use std::fs::TryLockError;
        use std::io::ErrorKind;
        use std::time::{Duration, Instant};
        let mut calls = 0;
        let err = acquire_owner_lock(
            || {
                calls += 1;
                Err(TryLockError::Error(std::io::Error::new(
                    ErrorKind::PermissionDenied,
                    "lock file not writable",
                )))
            },
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap_err();
        assert!(matches!(err, Error::Io(ref e) if e.kind() == ErrorKind::PermissionDenied));
        assert_eq!(calls, 1);
    }

    #[test]
    fn owner_lock_classifies_by_variant_not_error_text() {
        use std::fs::TryLockError;
        use std::io::ErrorKind;
        use std::time::{Duration, Instant};
        // An Error variant whose text claims contention must still be Io.
        let mut calls = 0;
        let err = acquire_owner_lock(
            || {
                calls += 1;
                Err(TryLockError::Error(std::io::Error::other(
                    "lock acquisition failed because the operation would block",
                )))
            },
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap_err();
        assert!(matches!(err, Error::Io(ref e) if e.kind() == ErrorKind::Other));
        assert_eq!(calls, 1);
        // WouldBlock carries no text and is always contention (covered by the
        // retry and deadline tests above).
    }
}
