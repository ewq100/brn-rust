//! Immutable Inbox Action-analysis admission. The existing chat turn owns the
//! execution lifecycle; saved Source bytes/proofs are retained as operational
//! evidence, never modified or promoted to knowledge here.
use super::{
    WorkStore, chat, now_ms,
    proposals::{ProposalStamp, SourceVersion},
};
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
    VisualInterpretation,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_asset: Option<SourceVersion>,
    /// Default omissions keep previously stored V14 canonical bytes unchanged.
    #[serde(default, skip_serializing_if = "InboxAnalysisPurpose::is_actions")]
    pub purpose: InboxAnalysisPurpose,
    pub id: Uuid,
    pub conversation: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intake: Option<InboxIntakeBinding>,
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
        if self.source_text.len() > MAX_INBOX_ACTION_SOURCE_BYTES {
            return Err(invalid(
                "Inbox Action analysis needs the exact bounded Source",
            ));
        }
        match (&self.source, &self.intake) {
            (Some(source), None) => {
                super::proposals::validate_path(&source.path)?;
                if source.fingerprint.len != self.source_text.len() as u64
                    || source.fingerprint.sha256 != hash(self.source_text.as_bytes())
                {
                    return Err(invalid("Inbox analysis needs the exact saved Source"));
                }
            }
            (None, Some(intake)) => {
                intake.validate_text(&self.source_text)?;
                if self.visual_asset.is_some()
                    || self.purpose == InboxAnalysisPurpose::VisualInterpretation
                {
                    return Err(invalid(
                        "private intake uses collection analysis, not legacy visual annotations",
                    ));
                }
            }
            _ => {
                return Err(invalid(
                    "Inbox analysis needs exactly one saved Source or private intake binding",
                ));
            }
        }
        self.note_id()?;
        if !crate::note_metadata::classify(&self.source_text)?.source
            || super::inbox_source::read_provenance(&self.source_text)?.is_none()
        {
            return Err(invalid(
                "Inbox Action analysis needs a managed Inbox Source",
            ));
        }
        match (self.purpose, self.visual_asset.as_ref()) {
            (InboxAnalysisPurpose::VisualInterpretation, Some(asset)) => {
                super::inbox_visual::validate_capture_asset(
                    self.source_path(),
                    &self.source_text,
                    asset,
                )?;
            }
            (InboxAnalysisPurpose::VisualInterpretation, None) | (_, Some(_)) => {
                return Err(invalid(
                    "Visual analysis needs its exact single image proof",
                ));
            }
            _ => {}
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
    pub fn source_path(&self) -> &str {
        self.source.as_ref().map_or_else(
            || {
                self.intake
                    .as_ref()
                    .map_or("", |binding| binding.source_path.as_str())
            },
            |source| source.path.as_str(),
        )
    }
    pub fn source_sha256(&self) -> [u8; 32] {
        self.source.as_ref().map_or_else(
            || {
                self.intake
                    .as_ref()
                    .map_or([0; 32], |binding| binding.source_text_sha256)
            },
            |source| source.fingerprint.sha256,
        )
    }
}

/// Pending evidence explicitly names an immutable extraction and planned Source;
/// no filesystem fingerprint is invented for an uninstalled note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxIntakeBinding {
    pub snapshot_id: Uuid,
    pub snapshot_sha256: [u8; 32],
    pub source_proposal: ProposalStamp,
    pub source_path: String,
    pub source_note_id: Uuid,
    pub source_text_sha256: [u8; 32],
    pub assets: Vec<String>,
    pub occurrences: Vec<String>,
}
impl InboxIntakeBinding {
    pub fn validate(&self) -> Result<()> {
        if self.snapshot_id.is_nil()
            || self.source_proposal.id.is_nil()
            || self.source_proposal.version == 0
            || self.source_note_id.is_nil()
            || self.assets.len() > brn_intake::MAX_ASSETS
            || self.occurrences.len() > brn_intake::MAX_OCCURRENCES
        {
            return Err(invalid(
                "invalid private intake identities or collection bounds",
            ));
        }
        super::proposals::validate_path(&self.source_path)?;
        for ids in [&self.assets, &self.occurrences] {
            let mut seen = HashSet::new();
            for id in ids {
                if id.is_empty()
                    || id.len() > 256
                    || !id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_:.".contains(&b))
                    || !seen.insert(id)
                {
                    return Err(invalid(
                        "invalid or repeated private intake evidence identity",
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn validate_text(&self, text: &str) -> Result<()> {
        self.validate()?;
        let provenance = super::inbox_source::read_provenance(text)?
            .ok_or_else(|| invalid("private intake needs its planned managed Source"))?;
        if hash(text.as_bytes()) != self.source_text_sha256
            || crate::note_identity::read(text)? != Some(self.source_note_id)
            || !provenance.extraction.as_ref().is_some_and(|binding| {
                binding.snapshot_id == self.snapshot_id
                    && binding.snapshot_sha256 == self.snapshot_sha256
            })
        {
            return Err(invalid(
                "private intake differs from its exact planned Source",
            ));
        }
        Ok(())
    }
    pub fn validate_snapshot(&self, snapshot: &super::intake::IntakeSnapshot) -> Result<()> {
        self.validate()?;
        if snapshot.id != self.snapshot_id || snapshot.digest()? != self.snapshot_sha256 {
            return Err(invalid("private analysis extraction snapshot changed"));
        }
        for id in &self.assets {
            if !snapshot
                .extraction
                .assets
                .iter()
                .any(|asset| &asset.id == id)
                || !snapshot.extraction.occurrences.iter().any(|occurrence| {
                    &occurrence.asset_id == id && self.occurrences.contains(&occurrence.id)
                })
            {
                return Err(invalid(
                    "private analysis selects an unavailable or ungrounded image asset",
                ));
            }
        }
        for id in &self.occurrences {
            if !snapshot.extraction.occurrences.iter().any(|occurrence| {
                &occurrence.id == id && self.assets.contains(&occurrence.asset_id)
            }) {
                return Err(invalid(
                    "private analysis selects an unavailable image occurrence",
                ));
            }
        }
        Ok(())
    }
    pub fn validate_capture(&self, job: &InboxActionJob) -> Result<()> {
        self.validate_text(&job.capture.source_text)?;
        if job.capture.source.is_some() || job.capture.intake.as_ref() != Some(self) {
            return Err(invalid(
                "private intake proposal differs from its analysis capture",
            ));
        }
        Ok(())
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

fn insert_capture(conn: &Connection, job: &InboxActionJob) -> Result<()> {
    job.validate()?;
    if let Some(binding) = &job.capture.intake {
        let snapshot = super::intake::read(conn, binding.snapshot_id)?
            .ok_or_else(|| invalid("private analysis snapshot is unavailable"))?;
        binding.validate_snapshot(&snapshot)?;
    }
    let bytes = encode(job)?;
    conn.execute("INSERT INTO inbox_actions(id,created_at_ms,record_json,record_sha256) VALUES (?1,?2,?3,?4)", params![job.capture.id.to_string(), job.created_at_ms as i64, bytes, hash(&bytes).as_slice()])?;
    Ok(())
}

/// Import only an exact retained reservation, without constructing a chat turn.
pub(super) fn restore_capture(conn: &Connection, job: &InboxActionJob) -> Result<()> {
    job.validate()?;
    if let Some(existing) = read(conn, job.capture.id)? {
        if existing != *job {
            return Err(conflict());
        }
        return Ok(());
    }
    if chat::read_turn(conn, job.capture.id)?.is_some()
        || super::proposal_rewrite::read_job(conn, job.capture.id)?.is_some()
    {
        return Err(conflict());
    }
    insert_capture(conn, job)
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
    if let Some(binding) = &job.capture.intake {
        let snapshot = super::intake::read(conn, binding.snapshot_id)?
            .ok_or_else(|| invalid("private analysis snapshot is unavailable"))?;
        binding.validate_snapshot(&snapshot)?;
    }
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
        insert_capture(&tx, &job)?;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intake: Option<Box<InboxIntakeBinding>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intake_citations: Vec<IntakeCitation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<InboxSupersedesBinding>,
    pub citations: Vec<crate::note_provenance::VaultCitation>,
}

/// Exact extracted wording in one retained processed source node. Offsets are
/// UTF-8 bytes in that node's text; locator and snapshot preserve its origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeCitation {
    pub snapshot_id: Uuid,
    pub source_id: String,
    pub locator: String,
    pub start: usize,
    pub end: usize,
    pub quote: String,
}
impl IntakeCitation {
    /// Source-local image destinations are materialized mechanically; citations
    /// remain byte ranges in the immutable extraction rather than the saved copy.
    pub fn resolve_for_source(
        snapshot: &super::intake::IntakeSnapshot,
        note_id: Uuid,
        start: usize,
        end: usize,
        source_id: Option<&str>,
    ) -> Result<Self> {
        let namespace = note_id.to_string();
        let mut materialized = snapshot.clone();
        materialized.extraction = snapshot
            .extraction
            .materialize_for_source(&namespace)
            .map_err(|_| invalid("private Source image materialization failed"))?;
        let mut citation = Self::resolve(&materialized, start, end, source_id)?;
        let (start, end) = snapshot
            .extraction
            .original_source_range(
                &namespace,
                &citation.source_id,
                citation.start,
                citation.end,
            )
            .map_err(|_| invalid("private quotation crosses a changed image destination"))?;
        let source = snapshot
            .extraction
            .sources
            .iter()
            .find(|source| source.id == citation.source_id)
            .ok_or_else(|| invalid("private source node missing"))?;
        if source.text.get(start..end) != Some(citation.quote.as_str()) {
            return Err(invalid(
                "private quotation differs from immutable extracted wording",
            ));
        }
        citation.start = start;
        citation.end = end;
        Ok(citation)
    }
    pub fn resolve(
        snapshot: &super::intake::IntakeSnapshot,
        start: usize,
        end: usize,
        source_id: Option<&str>,
    ) -> Result<Self> {
        let markdown = &snapshot.extraction.markdown;
        let quote = markdown
            .get(start..end)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| invalid("private quote has no exact extracted UTF-8 range"))?;
        let mut sources = snapshot.extraction.sources.iter().filter(|source| {
            source.status != "unprocessed"
                && !source.text.is_empty()
                && source_id.is_none_or(|id| id == source.id)
                && source.text.contains(quote)
        });
        let source = sources
            .next()
            .ok_or_else(|| invalid("private quote is absent from processed source text"))?;
        if sources.next().is_some() {
            return Err(invalid(
                "private quote belongs to multiple source nodes; select source_id explicitly",
            ));
        }
        if snapshot.extraction.sources.iter().any(|other| {
            other.id != source.id && other.status != "unprocessed" && other.text == source.text
        }) {
            return Err(invalid(
                "identical extracted source nodes have no qualified unique text ownership",
            ));
        }
        // KMP enumerates overlapping full-node spans in bounded linear work.
        // More than one mapping to the selected global occurrence is ambiguous.
        let needle = source.text.as_bytes();
        let mut prefix = vec![0; needle.len()];
        for i in 1..needle.len() {
            let mut matched = prefix[i - 1];
            while matched > 0 && needle[i] != needle[matched] {
                matched = prefix[matched - 1];
            }
            if needle[i] == needle[matched] {
                matched += 1;
            }
            prefix[i] = matched;
        }
        let mut matched = 0;
        let mut selected = None;
        let mut placements = 0;
        for (index, byte) in markdown.bytes().enumerate() {
            while matched > 0 && byte != needle[matched] {
                matched = prefix[matched - 1];
            }
            if byte == needle[matched] {
                matched += 1;
            }
            if matched == needle.len() {
                placements += 1;
                if placements > 1 {
                    return Err(invalid(
                        "private source text has multiple global placements; exact attachment ownership is unavailable",
                    ));
                }
                let span_end = index + 1;
                let span_start = span_end - needle.len();
                if span_start <= start && end <= span_end {
                    selected = Some((start - span_start, end - span_start));
                }
                matched = prefix[matched - 1];
            }
        }
        let (start,end) = selected.ok_or_else(|| invalid("private quote is generated wrapper/gap wording or outside its selected source node"))?;
        Ok(Self {
            snapshot_id: snapshot.id,
            source_id: source.id.clone(),
            locator: source.locator.clone(),
            start,
            end,
            quote: quote.to_owned(),
        })
    }
}
impl InboxKnowledgeBinding {
    /// Exact semantic capture and payload domains. Equality to a previously
    /// retained question/time/selection is checked by reservation import.
    pub fn validate_capture(&self, job: &InboxActionJob) -> Result<()> {
        self.validate()?;
        job.validate()?;
        let capture = &job.capture;
        let source_id = capture.note_id()?;
        if capture.id != self.analysis_id
            || capture.purpose != InboxAnalysisPurpose::KnowledgeAndActions
            || capture.source != self.source
            || capture.intake.as_ref() != self.intake.as_deref()
            || self.citations.iter().any(|c| {
                c.note_id != source_id
                    || capture.source_text.get(c.start_byte..c.end_byte) != Some(c.quote.as_str())
            })
        {
            return Err(invalid(
                "Inbox knowledge differs from its retained Source capture",
            ));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.analysis_id.is_nil() || self.note_id.is_nil() || self.citations.is_empty() {
            return Err(invalid(
                "Inbox knowledge needs exact analysis/note identities and citations",
            ));
        }
        let (source_path, sha256, source_len) = match (&self.source, &self.intake) {
            (Some(source), None) => (
                &source.path,
                source.fingerprint.sha256,
                source.fingerprint.len,
            ),
            (None, Some(intake)) => {
                intake.validate()?;
                (
                    &intake.source_path,
                    intake.source_text_sha256,
                    MAX_INBOX_ACTION_SOURCE_BYTES as u64,
                )
            }
            _ => {
                return Err(invalid(
                    "knowledge needs exactly one saved Source or private intake binding",
                ));
            }
        };
        super::proposals::validate_path(source_path)?;
        if source_len > MAX_INBOX_ACTION_SOURCE_BYTES as u64 {
            return Err(invalid(
                "Inbox knowledge source exceeds its full-capture bound",
            ));
        }
        crate::note_provenance::validate(&self.citations)?;
        let source_id = self.citations[0].note_id;
        if source_id == self.note_id
            || self.intake.as_ref().is_some_and(|intake| {
                intake.source_note_id != source_id
                    || self.intake_citations.len() != self.citations.len()
            })
            || self.intake.is_none() && !self.intake_citations.is_empty()
            || self
                .intake_citations
                .iter()
                .zip(&self.citations)
                .any(|(private, citation)| {
                    self.intake
                        .as_ref()
                        .is_none_or(|intake| private.snapshot_id != intake.snapshot_id)
                        || private.source_id.is_empty()
                        || private.source_id.len() > 256
                        || private.locator.is_empty()
                        || private.locator.len() > 8192
                        || private.start >= private.end
                        || private.end - private.start != private.quote.len()
                        || private.quote != citation.quote
                })
            || self.citations.iter().any(|c| {
                c.note_id != source_id || c.sha256 != sha256 || c.end_byte as u64 > source_len
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
                || previous.source.path.eq_ignore_ascii_case(source_path)
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
        if sources.len() > super::proposals::MAX_PROPOSAL_CHANGES
            || self
                .source
                .as_ref()
                .is_some_and(|source| sources.first() != Some(source))
        {
            return Err(invalid(
                "Inbox knowledge needs its exact selected Source first and 1 to 64 source bindings",
            ));
        }
        if let Some(previous) = &self.supersedes
            && sources.get(usize::from(self.source.is_some())) != Some(&previous.source)
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
    binding.validate_capture(&job)?;
    if let Some(intake) = &binding.intake {
        let snapshot = super::intake::read(conn, intake.snapshot_id)?
            .ok_or_else(|| invalid("private quote extraction is unavailable"))?;
        let body = crate::note_identity::body_start(&job.capture.source_text)?;
        let materialized = snapshot
            .extraction
            .materialize_for_source(&intake.source_note_id.to_string())
            .map_err(|_| invalid("private Source image materialization failed"))?;
        if job.capture.source_text.get(body..) != Some(materialized.markdown.as_str()) {
            return Err(invalid(
                "private Source wrapper differs from retained extracted text",
            ));
        }
        for (private, citation) in binding.intake_citations.iter().zip(&binding.citations) {
            let start = citation
                .start_byte
                .checked_sub(body)
                .ok_or_else(|| invalid("private quote cannot cite generated metadata"))?;
            let end = citation
                .end_byte
                .checked_sub(body)
                .ok_or_else(|| invalid("private quote cannot cite generated metadata"))?;
            if IntakeCitation::resolve_for_source(
                &snapshot,
                intake.source_note_id,
                start,
                end,
                Some(&private.source_id),
            )? != *private
            {
                return Err(invalid(
                    "private quote differs from its exact source node and original locator",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn has_issued_knowledge(conn: &Connection, job: &InboxActionJob) -> Result<bool> {
    let mut issued = false;
    let mut statement =
        conn.prepare("SELECT operation_id FROM proposal_applies ORDER BY operation_id")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let journal = super::proposal_apply::read_journal(conn, crate::parse_id(row.get(0)?)?)?
            .ok_or_else(|| invalid("listed Knowledge approval disappeared"))?;
        if let Some(binding) = &journal.approved.draft.inbox_knowledge
            && binding.analysis_id == job.capture.id
        {
            binding.validate_capture(job)?;
            issued = true;
        }
        if let Some(binding) = &journal.approved.draft.inbox_visual
            && binding.analysis_id == job.capture.id
        {
            binding.validate_capture(job)?;
            issued = true;
        }
    }
    Ok(issued)
}
