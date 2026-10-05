//! Immutable Inbox Action-analysis admission. The existing chat turn owns the
//! execution lifecycle; saved Source bytes/proofs are retained as operational
//! evidence, never modified or promoted to knowledge here.
use super::{WorkStore, chat, now_ms, proposals::SourceVersion};
use crate::{Error, Result, files::FileFingerprint, hash, invalid};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxAnalysisPurpose {
    #[default]
    Actions,
    KnowledgeAndActions,
}
impl InboxAnalysisPurpose {
    pub fn is_actions(&self) -> bool {
        *self == Self::Actions
    }
}

pub const MAX_INBOX_ACTION_SOURCE_BYTES: usize = 50_000;
const MAX_QUESTION_BYTES: usize = 512 * 1024;
const MAX_RECORD_BYTES: usize = 1024 * 1024;
pub(super) const V14: &str = "
CREATE TABLE inbox_actions (
 id TEXT PRIMARY KEY,
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0),
 record_json BLOB NOT NULL,
 record_sha256 BLOB NOT NULL CHECK(length(record_sha256)=32)
);";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxActionCapture {
    /// Default omissions keep previously stored V14 canonical bytes unchanged.
    #[serde(default, skip_serializing_if = "InboxAnalysisPurpose::is_actions")]
    pub purpose: InboxAnalysisPurpose,
    pub id: Uuid,
    pub conversation: Option<Uuid>,
    pub source: SourceVersion,
    pub source_text: String,
    pub provider: String,
    pub model: String,
    pub effort: String,
}

impl InboxActionCapture {
    pub fn validate(&self) -> Result<()> {
        if self.id.is_nil() || self.conversation.is_some_and(|id| id.is_nil()) {
            return Err(invalid("Inbox Action analysis needs nonnil identities"));
        }
        super::proposals::validate_path(&self.source.path)?;
        if self.source_text.len() > MAX_INBOX_ACTION_SOURCE_BYTES
            || self.source.fingerprint.len != self.source_text.len() as u64
            || self.source.fingerprint.sha256 != hash(self.source_text.as_bytes())
        {
            return Err(invalid(
                "Inbox Action analysis needs the exact bounded Source",
            ));
        }
        self.note_id()?;
        if !crate::note_metadata::classify(&self.source_text)?.source
            || super::inbox_source::read_provenance(&self.source_text)?.is_none()
        {
            return Err(invalid(
                "Inbox Action analysis needs a managed Inbox Source",
            ));
        }
        chat::validate_selection(&self.provider, &self.model)?;
        if !matches!(self.effort.as_str(), "low" | "medium" | "high") {
            return Err(invalid(
                "Inbox Action analysis needs explicit reasoning effort",
            ));
        }
        Ok(())
    }

    pub fn note_id(&self) -> Result<Uuid> {
        crate::note_identity::read(&self.source_text)?
            .ok_or_else(|| invalid("Inbox Action Source needs a managed UUID"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxActionJob {
    pub capture: InboxActionCapture,
    pub question: String,
    pub created_at_ms: u64,
}

impl InboxActionJob {
    pub fn validate(&self) -> Result<()> {
        self.capture.validate()?;
        if self.created_at_ms > i64::MAX as u64
            || self.question.trim().is_empty()
            || self.question.len() > MAX_QUESTION_BYTES
            || encode(self)?.len() > MAX_RECORD_BYTES
        {
            return Err(invalid(
                "Inbox Action analysis exceeds its question or record bounds",
            ));
        }
        Ok(())
    }
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| invalid("could not encode Inbox Action analysis"))
}

fn conflict() -> Error {
    Error::OperationConflict("Inbox Action analysis UUID has another exact request".into())
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
            "SELECT type,tbl_name,sql FROM sqlite_schema WHERE name='inbox_actions'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if !schema.is_some_and(|(kind, table, sql)| {
        kind == "table" && table == "inbox_actions" && normalized(&sql) == normalized(V14)
    }) {
        return Err(invalid(
            "Inbox Action analysis schema differs from its owned shape",
        ));
    }
    let extra: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE tbl_name='inbox_actions' AND name NOT IN ('inbox_actions','sqlite_autoindex_inbox_actions_1')",
        [],
        |r| r.get(0),
    )?;
    if extra != 0 {
        return Err(invalid(
            "Inbox Action analysis has unexpected schema objects",
        ));
    }
    Ok(())
}

/// Record-only read keeps chat's binding validation acyclic. No turn need exist
/// yet: reservation deliberately precedes worker/provider admission.
pub(super) fn reserved(conn: &Connection, id: Uuid) -> Result<Option<InboxActionJob>> {
    check_schema(conn)?;
    let row: Option<(i64, Option<Vec<u8>>, Vec<u8>)> = conn.query_row(
        "SELECT created_at_ms,CASE WHEN length(record_json)<=?2 THEN record_json END,record_sha256 FROM inbox_actions WHERE id=?1",
        params![id.to_string(), MAX_RECORD_BYTES as i64],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).optional()?;
    let Some((time, bytes, sha)) = row else {
        return Ok(None);
    };
    let bytes = bytes.ok_or_else(|| invalid("stored Inbox Action analysis exceeds its bound"))?;
    let job: InboxActionJob = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("malformed Inbox Action analysis record"))?;
    job.validate()?;
    if job.capture.id != id
        || time < 0
        || job.created_at_ms != time as u64
        || sha != hash(&bytes)
        || encode(&job)? != bytes
    {
        return Err(invalid("Inbox Action analysis record bindings differ"));
    }
    let rewrite: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM proposal_rewrites WHERE id=?1)",
        [id.to_string()],
        |r| r.get(0),
    )?;
    if rewrite {
        return Err(invalid(
            "Inbox Action analysis UUID also belongs to a Rewrite",
        ));
    }
    Ok(Some(job))
}

pub(super) fn check_turn(conn: &Connection, turn: &chat::WorkTurn) -> Result<()> {
    if let Some(job) = reserved(conn, turn.id)? {
        let capture = &job.capture;
        if turn.question != job.question
            || turn.provider != capture.provider
            || turn.model != capture.model
            || turn.effort.as_deref() != Some(capture.effort.as_str())
            || capture
                .conversation
                .is_some_and(|id| id != turn.conversation_id)
        {
            return Err(invalid(
                "Inbox Action analysis differs from its reserved chat turn",
            ));
        }
    }
    Ok(())
}

pub(super) fn read(conn: &Connection, id: Uuid) -> Result<Option<InboxActionJob>> {
    let job = reserved(conn, id)?;
    if job.is_some() {
        chat::read_turn(conn, id)?;
    }
    Ok(job)
}

pub(super) fn check_all(conn: &Connection) -> Result<()> {
    check_schema(conn)?;
    let ids = conn
        .prepare("SELECT id FROM inbox_actions ORDER BY rowid")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for id in ids {
        let uuid =
            Uuid::parse_str(&id).map_err(|_| invalid("invalid Inbox Action analysis UUID"))?;
        if uuid.to_string() != id || read(conn, uuid)?.is_none() {
            return Err(invalid("invalid Inbox Action analysis row"));
        }
    }
    // Checking captures alone cannot detect a proposal whose capture was removed.
    // Validate the reverse bindings before startup reconciliation or backup.
    super::proposals::check_all(conn)?;
    Ok(())
}

impl WorkStore {
    pub fn reserve_inbox_action(
        &mut self,
        capture: &InboxActionCapture,
        question: &str,
    ) -> Result<InboxActionJob> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(job) = read(&tx, capture.id)? {
            if job.capture != *capture || job.question != question {
                return Err(conflict());
            }
            tx.commit()?;
            return Ok(job);
        }
        // Exact replay above never creates a conversation, samples the clock or
        // revalidates current source/auth state. Fresh reservations remain bound.
        capture.validate()?;
        check_all(&tx)?;
        if chat::read_turn(&tx, capture.id)?.is_some()
            || super::proposal_rewrite::read_job(&tx, capture.id)?.is_some()
        {
            return Err(conflict());
        }
        let job = InboxActionJob {
            capture: capture.clone(),
            question: question.into(),
            created_at_ms: now_ms(),
        };
        job.validate()?;
        let bytes = encode(&job)?;
        tx.execute("INSERT INTO inbox_actions(id,created_at_ms,record_json,record_sha256) VALUES (?1,?2,?3,?4)", params![capture.id.to_string(), job.created_at_ms as i64, bytes, hash(&bytes).as_slice()])?;
        tx.commit()?;
        Ok(job)
    }

    pub fn inbox_action(&self, id: Uuid) -> Result<Option<InboxActionJob>> {
        let tx = self.conn.unchecked_transaction()?;
        let job = read(&tx, id)?;
        tx.commit()?;
        Ok(job)
    }

    pub fn begin_inbox_action_turn(&mut self, job: &InboxActionJob) -> Result<chat::WorkTurn> {
        chat::begin_inbox_action_turn(&mut self.conn, job)
    }
}

impl chat::ChatStore {
    pub fn begin_inbox_action_turn(&mut self, job: &InboxActionJob) -> Result<chat::WorkTurn> {
        chat::begin_inbox_action_turn(&mut self.conn, job)
    }
}

/// Exact predecessor authority inside an independently reviewed supersession.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSupersedesBinding {
    pub note_id: Uuid,
    pub source: SourceVersion,
}

/// Immutable authority for one independently reviewed knowledge consequence.
/// This is proposal metadata, not a second execution or application lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxKnowledgeBinding {
    pub analysis_id: Uuid,
    pub note_id: Uuid,
    pub source: SourceVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<InboxSupersedesBinding>,
    pub citations: Vec<crate::note_provenance::VaultCitation>,
}
impl InboxKnowledgeBinding {
    pub fn validate(&self) -> Result<()> {
        if self.analysis_id.is_nil() || self.note_id.is_nil() || self.citations.is_empty() {
            return Err(invalid(
                "Inbox knowledge needs exact analysis/note identities and citations",
            ));
        }
        super::proposals::validate_path(&self.source.path)?;
        if self.source.fingerprint.len > MAX_INBOX_ACTION_SOURCE_BYTES as u64 {
            return Err(invalid(
                "Inbox knowledge source exceeds its full-capture bound",
            ));
        }
        crate::note_provenance::validate(&self.citations)?;
        let source_id = self.citations[0].note_id;
        if source_id == self.note_id
            || self.citations.iter().any(|c| {
                c.note_id != source_id
                    || c.sha256 != self.source.fingerprint.sha256
                    || c.end_byte as u64 > self.source.fingerprint.len
            })
        {
            return Err(invalid(
                "Inbox knowledge citations must bind one separate exact Source",
            ));
        }
        if let Some(previous) = &self.supersedes {
            super::proposals::validate_path(&previous.source.path)?;
            if previous.note_id.is_nil()
                || previous.note_id == self.note_id
                || previous.note_id == source_id
                || previous.source.path.eq_ignore_ascii_case(&self.source.path)
                || previous.source.fingerprint.len > super::MAX_NOTE_BYTES as u64
            {
                return Err(invalid(
                    "Inbox supersession needs a separate bounded predecessor identity and path",
                ));
            }
        }
        Ok(())
    }
    /// Selected Source is first and a bound predecessor is second. Remaining
    /// ordered proofs are ordinary evidence; target eligibility belongs to workflow.
    pub fn validate_sources(&self, sources: &[SourceVersion]) -> Result<()> {
        self.validate()?;
        if !(1..=super::proposals::MAX_PROPOSAL_CHANGES).contains(&sources.len())
            || sources.first() != Some(&self.source)
        {
            return Err(invalid(
                "Inbox knowledge needs its exact selected Source first and 1 to 64 source bindings",
            ));
        }
        if let Some(previous) = &self.supersedes
            && sources.get(1) != Some(&previous.source)
        {
            return Err(invalid(
                "Inbox supersession needs its exact predecessor source second",
            ));
        }
        let mut paths = HashSet::new();
        for source in sources {
            super::proposals::validate_path(&source.path)?;
            if source.fingerprint.len > super::MAX_NOTE_BYTES as u64
                || !paths.insert(source.path.to_ascii_lowercase())
            {
                return Err(invalid(
                    "Inbox knowledge source proofs need unique paths and at most 1 MiB each",
                ));
            }
        }
        Ok(())
    }
    pub fn validate_text(&self, text: &str) -> Result<()> {
        self.validate()?;
        let citations = crate::note_provenance::read(text)?;
        if crate::note_identity::read(text)? != Some(self.note_id)
            || crate::note_metadata::classify(text)?
                != crate::note_metadata::NoteClassification::default()
            || super::inbox_source::read_provenance(text)?.is_some()
            || !self.citations.iter().all(|c| citations.contains(c))
        {
            return Err(invalid(
                "Inbox knowledge must preserve its current identity and exact Source citations",
            ));
        }
        if let Some(previous) = &self.supersedes
            && !text.ends_with(&format!(
                "\n\nPrevious version: [History](brn://note/{})\n",
                previous.note_id
            ))
        {
            return Err(invalid(
                "Inbox supersession must preserve its exact previous-version footer",
            ));
        }
        Ok(())
    }

    /// Validate the sole historical member against its immutable complete proof.
    /// Review and Rewrite cannot change any of its generated History bytes.
    pub fn validate_history(
        &self,
        path: &str,
        before: &FileFingerprint,
        before_text: &str,
        text: &str,
    ) -> Result<()> {
        self.validate()?;
        let previous = self
            .supersedes
            .as_ref()
            .ok_or_else(|| invalid("Inbox knowledge has no supersession binding"))?;
        if path != previous.source.path
            || before != &previous.source.fingerprint
            || before.len != before_text.len() as u64
            || before.sha256 != hash(before_text.as_bytes())
            || crate::note_identity::read(before_text)? != Some(previous.note_id)
            || super::inbox_source::read_provenance(before_text)?.is_some()
            || text != crate::note_metadata::to_history(before_text)?
        {
            return Err(invalid(
                "Inbox History member differs from its exact current predecessor",
            ));
        }
        crate::note_provenance::read(before_text)?;
        Ok(())
    }
}

pub(super) fn check_knowledge_binding(
    conn: &Connection,
    binding: &InboxKnowledgeBinding,
) -> Result<()> {
    binding.validate()?;
    let job = reserved(conn, binding.analysis_id)?
        .ok_or_else(|| invalid("Inbox knowledge analysis capture is unavailable"))?;
    let source_id = job.capture.note_id()?;
    if job.capture.purpose != InboxAnalysisPurpose::KnowledgeAndActions
        || job.capture.source != binding.source
        || binding.citations.iter().any(|c| {
            c.note_id != source_id
                || job.capture.source_text.get(c.start_byte..c.end_byte) != Some(c.quote.as_str())
        })
    {
        return Err(invalid(
            "Inbox knowledge differs from its retained Source capture",
        ));
    }
    Ok(())
}
