//! Compact authority for an already approved, protected original Text Source.
use super::super::{
    WorkStore,
    inbox::InboxKind,
    inbox_processing::InboxConversionFormat,
    proposal_apply::{self, ApplyJournal, ApplyOutcome, ApprovalRequest},
    proposals::{self, NoteChange, ProposalState, SourceVersion},
};
use crate::{MAX_NOTE_BYTES, Result, hash, invalid};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAX_BINDING_BYTES: usize = 8192;
const MAX_PATH_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SentSourceBinding {
    pub source_approval: ApprovalRequest,
    pub source: SourceVersion,
    pub note_id: Uuid,
}

fn validate_source(source: &SourceVersion) -> Result<()> {
    proposals::validate_path(&source.path)?;
    if source.path.len() > MAX_PATH_BYTES
        || source.fingerprint.len == 0
        || source.fingerprint.len > MAX_NOTE_BYTES as u64
    {
        return Err(invalid(
            "Sent Source path or complete fingerprint exceeds its bound",
        ));
    }
    Ok(())
}
impl SentSourceBinding {
    /// Shape validation only; retained approval and physical freshness are separate checks.
    pub fn validate(&self) -> Result<()> {
        proposals::nonnil(self.note_id)?;
        proposals::nonnil(self.source_approval.operation_id)?;
        proposals::nonnil(self.source_approval.expected.id)?;
        if self.source_approval.expected.version == 0
            || self
                .source_approval
                .expected
                .version
                .checked_add(3)
                .is_none()
        {
            return Err(invalid("Sent Source approval version is invalid"));
        }
        validate_source(&self.source)?;
        let bytes = serde_json::to_vec(self)
            .map_err(|_| invalid("Could not encode Sent Source binding"))?;
        if bytes.len() > MAX_BINDING_BYTES {
            return Err(invalid(
                "Sent Source binding exceeds its encoded size limit",
            ));
        }
        Ok(())
    }
}

/// A historical Applied journal is sufficient for recovery. Current file or
/// proposal eligibility must never be inferred from this retained proof.
fn journal_note(journal: &ApplyJournal, source: &SourceVersion) -> Result<Option<Uuid>> {
    if journal.undo.is_some()
        || journal.receipt.as_ref().map(|r| r.outcome) != Some(ApplyOutcome::Applied)
    {
        return Ok(None);
    }
    let draft = &journal.approved.draft;
    let Some(binding) = &draft.inbox_source else {
        return Ok(None);
    };
    if binding.original.capture.kind != InboxKind::Text
        || binding.format != InboxConversionFormat::LiteralTextV1
        || binding.extraction.is_some()
        || binding.visual.is_some()
        || draft.session_id.is_some()
    {
        return Ok(None);
    }
    let [NoteChange::Create { path, text, .. }] = draft.changes.as_slice() else {
        return Ok(None);
    };
    if *path != source.path
        || text.len() as u64 != source.fingerprint.len
        || hash(text.as_bytes()) != source.fingerprint.sha256
    {
        return Ok(None);
    }
    binding.validate_markdown(text)?;
    if crate::note_identity::read(text)? != Some(binding.note_id)
        || !crate::note_metadata::classify(text)?.source
    {
        return Err(invalid(
            "Sent Source approval has invalid protected identity or classification",
        ));
    }
    let destination = journal
        .observations
        .as_ref()
        .and_then(|proofs| proofs.first())
        .and_then(|proof| proof.destination.as_ref())
        .ok_or_else(|| invalid("Sent Source approval lacks its Applied destination proof"))?;
    if destination.len != source.fingerprint.len || destination.sha256 != source.fingerprint.sha256
    {
        return Err(invalid(
            "Sent Source Applied proof differs from its complete text",
        ));
    }
    Ok(Some(binding.note_id))
}

fn current_applied(conn: &Connection, journal: &ApplyJournal) -> Result<bool> {
    let current = proposals::read_proposal(conn, journal.approved.draft.id)?
        .ok_or_else(|| invalid("Sent Source approval has no retained proposal"))?;
    Ok(current.record.state == ProposalState::Applied
        && current.record.draft == journal.approved.draft
        && journal
            .receipt
            .as_ref()
            .is_some_and(|r| r.stamp == current.record.stamp()))
}
fn mint(conn: &Connection, source: &SourceVersion) -> Result<SentSourceBinding> {
    validate_source(source)?;
    let mut selected = None;
    let mut stmt =
        conn.prepare("SELECT operation_id FROM proposal_applies ORDER BY operation_id")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id = crate::parse_id(row.get(0)?)?;
        let journal = proposal_apply::read_journal(conn, id)?
            .ok_or_else(|| invalid("Listed Sent Source approval disappeared"))?;
        if let Some(note_id) = journal_note(&journal, source)?
            && current_applied(conn, &journal)?
        {
            let binding = SentSourceBinding {
                source_approval: journal.request,
                source: source.clone(),
                note_id,
            };
            binding.validate()?;
            if selected.replace(binding).is_some() {
                return Err(invalid("Sent Source Applied approval is ambiguous"));
            }
        }
    }
    selected.ok_or_else(|| invalid("Sent Source has no exact Applied protected Text approval"))
}

pub(super) fn validate_retained(
    conn: &Connection,
    binding: &SentSourceBinding,
    fresh: bool,
) -> Result<()> {
    binding.validate()?;
    if fresh {
        if mint(conn, &binding.source)? != *binding {
            return Err(invalid(
                "Sent Source differs from its unique exact Applied approval",
            ));
        }
    } else {
        let journal = proposal_apply::read_journal(conn, binding.source_approval.operation_id)?
            .ok_or_else(|| invalid("Sent Source historical approval is missing"))?;
        if journal.request != binding.source_approval
            || journal_note(&journal, &binding.source)? != Some(binding.note_id)
        {
            return Err(invalid(
                "Sent Source differs from its retained historical approval",
            ));
        }
    }
    Ok(())
}

impl WorkStore {
    /// Read-only exact Applied protected Text approval; no filesystem freshness is claimed.
    pub fn sent_source_binding(&self, source: &SourceVersion) -> Result<SentSourceBinding> {
        let tx = self.conn.unchecked_transaction()?;
        let binding = mint(&tx, source)?;
        tx.commit()?;
        Ok(binding)
    }
    /// Recheck current retained approval authority without reading the Source file.
    pub fn validate_sent_source_binding(&self, binding: &SentSourceBinding) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        validate_retained(&tx, binding, true)?;
        tx.commit()?;
        Ok(())
    }
}
