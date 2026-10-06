//! Owned Rewrite admission and atomic full-proposal result settlement.
//! Jobs retain only bindings and outcomes; prompts, comments and result bodies
//! remain in the caller's transient capture or the existing proposal record.
use super::{WorkStore, chat, now_ms, proposals};
use crate::{Error, Result, hash, invalid, parse_id};
use proposals::{ProposalEdit, ProposalRecord, ProposalStamp};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) const V6: &str = "
CREATE TABLE proposal_rewrites (
    id TEXT PRIMARY KEY,
    job_json BLOB NOT NULL,
    job_sha256 BLOB NOT NULL CHECK(length(job_sha256)=32)
);";
const MAX_JOB_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewriteSpec {
    pub id: Uuid,
    pub expected: ProposalStamp,
    pub provider: String,
    pub model: String,
    pub effort: String,
}

impl RewriteSpec {
    pub fn validate(&self) -> Result<()> {
        proposals::nonnil(self.id)?;
        proposals::nonnil(self.expected.id)?;
        if self.expected.version == 0 || !matches!(self.effort.as_str(), "low" | "medium" | "high")
        {
            return Err(invalid("invalid Rewrite review version or effort"));
        }
        chat::validate_selection(&self.provider, &self.model)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewriteStatus {
    Running,
    Completed,
    Stale,
    Interrupted,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewriteJob {
    pub spec: RewriteSpec,
    pub capture_sha256: [u8; 32],
    pub status: RewriteStatus,
    pub result_stamp: Option<ProposalStamp>,
    pub outcome_sha256: Option<[u8; 32]>,
    pub error_code: Option<String>,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
}

impl RewriteJob {
    pub(super) fn validate(&self) -> Result<()> {
        self.spec.validate()?;
        chat::validate_error(self.error_code.as_deref())?;
        if self.started_at_ms == 0
            || self
                .finished_at_ms
                .is_some_and(|finished| finished < self.started_at_ms)
        {
            return Err(invalid("invalid Rewrite job timestamps"));
        }
        let valid = match self.status {
            RewriteStatus::Running => {
                self.finished_at_ms.is_none()
                    && self.outcome_sha256.is_none()
                    && self.result_stamp.is_none()
                    && self.error_code.is_none()
            }
            RewriteStatus::Completed => {
                self.finished_at_ms.is_some()
                    && self.outcome_sha256.is_some()
                    && self.error_code.is_none()
                    && self.result_stamp.is_some_and(|stamp| {
                        stamp.id == self.spec.expected.id
                            && (stamp.version == self.spec.expected.version
                                || self.spec.expected.version.checked_add(1) == Some(stamp.version))
                    })
            }
            RewriteStatus::Stale => {
                self.finished_at_ms.is_some()
                    && self.outcome_sha256.is_some()
                    && self.result_stamp.is_none()
                    && self.error_code.is_none()
            }
            RewriteStatus::Interrupted => {
                self.finished_at_ms.is_some()
                    && self.outcome_sha256 == Some(hash(&encode(&RewriteOutcome::Interrupted)?))
                    && self.result_stamp.is_none()
                    && self.error_code.is_none()
            }
            RewriteStatus::Failed => {
                self.finished_at_ms.is_some()
                    && self.outcome_sha256.is_some()
                    && self.result_stamp.is_none()
                    && self.error_code.is_some()
            }
        };
        if !valid {
            return Err(invalid("invalid Rewrite job status or outcome"));
        }
        if self.status == RewriteStatus::Failed
            && self.outcome_sha256
                != Some(hash(&encode(&RewriteOutcome::Failed(
                    self.error_code.clone().expect("validated failure category"),
                ))?))
        {
            return Err(invalid("Rewrite failure differs from its bound outcome"));
        }
        Ok(())
    }
}

/// Only validated full proposal edits and fixed error categories cross persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RewriteOutcome {
    Completed(ProposalEdit),
    Interrupted,
    Failed(String),
}

/// Validates an ordinary owner edit against its review without writing it.
pub fn validate_result(record: &ProposalRecord, edit: &ProposalEdit) -> Result<()> {
    proposals::edited_review(record, edit).map(|_| ())
}

const PROTECTED_FIELDS: [&str; 4] = [
    "brn_kind",
    "brn_state",
    "brn_provenance",
    "brn_inbox_source",
];

fn protected_fields(text: &str) -> Result<[Option<&str>; 4]> {
    let metadata = crate::note_identity::raw_fields(text, PROTECTED_FIELDS, false)?;
    Ok(std::array::from_fn(|index| {
        metadata
            .as_ref()
            .and_then(|metadata| metadata.fields[index].as_ref())
            .map(|field| &text[field.line.clone()])
    }))
}

/// AI Rewrite preserves exact managed field presence and line bytes, including
/// scalar spelling, comments and newlines. Values remain opaque; this does not
/// impose new metadata requirements on ordinary owner edits or historical rows.
pub fn validate_rewrite_result(record: &ProposalRecord, edit: &ProposalEdit) -> Result<()> {
    validate_result(record, edit)?;
    for (change, text) in record.draft.changes.iter().zip(&edit.texts) {
        if let (Some(before), Some(after)) = (change.text(), text.as_deref()) {
            let before = protected_fields(before)?;
            let after = protected_fields(after)?;
            for (index, key) in PROTECTED_FIELDS.iter().enumerate() {
                if before[index] != after[index] {
                    return Err(invalid(&format!(
                        "Rewrite must preserve exact {key} field bytes"
                    )));
                }
            }
        }
    }
    Ok(())
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| invalid("could not encode Rewrite work"))
}

fn conflict() -> Error {
    Error::OperationConflict("Rewrite UUID has another request or outcome".into())
}

pub(super) fn read_job(conn: &Connection, id: Uuid) -> Result<Option<RewriteJob>> {
    let row: Option<(Option<Vec<u8>>, Vec<u8>)> = conn
        .query_row(
            "SELECT CASE WHEN length(job_json)<=?2 THEN job_json END,job_sha256 FROM proposal_rewrites WHERE id=?1",
            params![id.to_string(), MAX_JOB_BYTES as i64],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    row.map(|(bytes, digest)| {
        let bytes = bytes.ok_or_else(|| invalid("stored Rewrite job exceeds 4096 bytes"))?;
        if digest.as_slice() != hash(&bytes) {
            return Err(invalid("stored Rewrite job failed its hash check"));
        }
        let job: RewriteJob =
            serde_json::from_slice(&bytes).map_err(|_| invalid("invalid stored Rewrite job"))?;
        if job.spec.id != id {
            return Err(invalid("stored Rewrite job differs from its row UUID"));
        }
        job.validate()?;
        let collision: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE turn_id=?1) OR EXISTS(SELECT 1 FROM inbox_actions WHERE id=?1)",
            [id.to_string()],
            |row| row.get(0),
        )?;
        if collision {
            return Err(invalid("stored Rewrite UUID also belongs to another owned job"));
        }
        Ok(job)
    })
    .transpose()
}

fn write_job(conn: &Connection, job: &RewriteJob, insert: bool) -> Result<()> {
    job.validate()?;
    let bytes = encode(job)?;
    if bytes.len() > MAX_JOB_BYTES {
        return Err(invalid("Rewrite job exceeds 4096 bytes"));
    }
    let sql = if insert {
        "INSERT INTO proposal_rewrites(id,job_json,job_sha256) VALUES(?1,?2,?3)"
    } else {
        "UPDATE proposal_rewrites SET job_json=?2,job_sha256=?3 WHERE id=?1"
    };
    conn.execute(
        sql,
        params![job.spec.id.to_string(), bytes, hash(&bytes).as_slice()],
    )?;
    Ok(())
}

fn begin(
    conn: &mut Connection,
    spec: &RewriteSpec,
) -> Result<(RewriteJob, Option<ProposalRecord>)> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    // A persisted admission never grants permission for another provider call.
    if let Some(job) = read_job(&tx, spec.id)? {
        if job.spec != *spec {
            return Err(conflict());
        }
        tx.commit()?;
        return Ok((job, None));
    }
    spec.validate()?;
    if super::inbox_actions::reserved(&tx, spec.id)?.is_some() {
        return Err(conflict());
    }
    let collision: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM messages WHERE turn_id=?1)",
        [spec.id.to_string()],
        |row| row.get(0),
    )?;
    if collision {
        return Err(conflict());
    }
    let capture = proposals::draft_at(&tx, spec.expected)?.record;
    let job = RewriteJob {
        spec: spec.clone(),
        capture_sha256: hash(&encode(&capture)?),
        status: RewriteStatus::Running,
        result_stamp: None,
        outcome_sha256: None,
        error_code: None,
        started_at_ms: now_ms(),
        finished_at_ms: None,
    };
    write_job(&tx, &job, true)?;
    tx.commit()?;
    Ok((job, Some(capture)))
}

fn outcome_hash(spec: &RewriteSpec, outcome: &RewriteOutcome) -> Result<[u8; 32]> {
    match outcome {
        RewriteOutcome::Completed(edit) => {
            if edit.expected != spec.expected {
                return Err(conflict());
            }
            proposals::validate_edit(edit)?;
        }
        RewriteOutcome::Failed(code) => chat::validate_error(Some(code))?,
        RewriteOutcome::Interrupted => {}
    }
    Ok(hash(&encode(outcome)?))
}

fn settle(conn: &Connection, id: Uuid, outcome: &RewriteOutcome) -> Result<RewriteJob> {
    let mut job =
        read_job(conn, id)?.ok_or_else(|| Error::NotFound("Rewrite job does not exist".into()))?;
    let digest = outcome_hash(&job.spec, outcome)?;
    if job.status != RewriteStatus::Running {
        if job.outcome_sha256 != Some(digest) {
            return Err(conflict());
        }
        return Ok(job);
    }
    match outcome {
        RewriteOutcome::Completed(edit) => match proposals::draft_at(conn, job.spec.expected) {
            Ok(stored) if hash(&encode(&stored.record)?) == job.capture_sha256 => {
                validate_rewrite_result(&stored.record, edit)?;
                let record = proposals::edit_in_transaction(conn, edit)?;
                job.status = RewriteStatus::Completed;
                job.result_stamp = Some(record.stamp());
            }
            Ok(_) => job.status = RewriteStatus::Stale,
            Err(Error::StateChanged(_) | Error::NotFound(_)) => {
                job.status = RewriteStatus::Stale;
            }
            Err(error) => return Err(error),
        },
        RewriteOutcome::Interrupted => job.status = RewriteStatus::Interrupted,
        RewriteOutcome::Failed(code) => {
            job.status = RewriteStatus::Failed;
            job.error_code = Some(code.clone());
        }
    }
    job.outcome_sha256 = Some(digest);
    job.finished_at_ms = Some(now_ms().max(job.started_at_ms));
    write_job(conn, &job, false)?;
    Ok(job)
}

fn finish(conn: &mut Connection, id: Uuid, outcome: &RewriteOutcome) -> Result<RewriteJob> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let job = settle(&tx, id, outcome)?;
    tx.commit()?;
    Ok(job)
}

/// Validate every row before changing any interrupted work or making an open backup.
pub(super) fn reconcile(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let ids = tx
        .prepare("SELECT id FROM proposal_rewrites ORDER BY rowid")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let jobs = ids
        .into_iter()
        .map(|id| {
            read_job(&tx, parse_id(id)?)?.ok_or_else(|| invalid("listed Rewrite job is missing"))
        })
        .collect::<Result<Vec<_>>>()?;
    for job in jobs {
        if job.status == RewriteStatus::Running {
            settle(&tx, job.spec.id, &RewriteOutcome::Interrupted)?;
        }
    }
    tx.commit()?;
    Ok(())
}

impl WorkStore {
    pub fn proposal_rewrite(&self, id: Uuid) -> Result<Option<RewriteJob>> {
        read_job(&self.conn, id)
    }

    pub fn begin_proposal_rewrite(
        &mut self,
        spec: &RewriteSpec,
    ) -> Result<(RewriteJob, Option<ProposalRecord>)> {
        begin(&mut self.conn, spec)
    }

    pub fn finish_proposal_rewrite(
        &mut self,
        id: Uuid,
        outcome: &RewriteOutcome,
    ) -> Result<RewriteJob> {
        finish(&mut self.conn, id, outcome)
    }
}

impl chat::ChatStore {
    pub fn proposal_rewrite(&self, id: Uuid) -> Result<Option<RewriteJob>> {
        read_job(&self.conn, id)
    }

    pub fn begin_proposal_rewrite(
        &mut self,
        spec: &RewriteSpec,
    ) -> Result<(RewriteJob, Option<ProposalRecord>)> {
        begin(&mut self.conn, spec)
    }

    pub fn finish_proposal_rewrite(
        &mut self,
        id: Uuid,
        outcome: &RewriteOutcome,
    ) -> Result<RewriteJob> {
        finish(&mut self.conn, id, outcome)
    }
}
