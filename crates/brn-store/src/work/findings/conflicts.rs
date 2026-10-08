//! Narrow retained Inbox conflict validation and queries; no vault I/O.
use super::*;
use crate::work::{
    inbox_actions::{InboxActionCapture, InboxAnalysisPurpose},
    proposal_apply::{self, ApplyOutcome},
    proposals::NoteChange,
};

fn selected_source(
    conn: &Connection,
    capture: &InboxActionCapture,
) -> Result<(SourceVersion, Option<VaultRecord>)> {
    capture.validate()?;
    if capture.purpose != InboxAnalysisPurpose::KnowledgeAndActions {
        return Err(invalid("this Inbox analysis does not admit conflicts"));
    }
    if let Some(source) = &capture.source {
        return Ok((source.clone(), None));
    }
    let intake = capture
        .intake
        .as_ref()
        .ok_or_else(|| invalid("conflict needs a saved or exact Applied intake Source"))?;
    let snapshot = super::super::intake::read(conn, intake.snapshot_id)?
        .ok_or_else(|| invalid("conflict intake snapshot is unavailable"))?;
    intake.validate_snapshot(&snapshot)?;
    let converted = snapshot
        .extraction
        .materialize_for_source(&intake.source_note_id.to_string())
        .map_err(|_| invalid("conflict Source materialization failed"))?;
    let assets = snapshot
        .extraction
        .assets
        .iter()
        .map(|asset| {
            Ok(super::super::inbox_source::ExtractionAsset {
                name: brn_intake::asset_file_name_for_source(
                    asset,
                    &intake.source_note_id.to_string(),
                )
                .map_err(|_| invalid("conflict Source asset name is unavailable"))?,
                byte_len: asset.bytes.len() as u64,
                sha256: asset.sha256,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let ids = conn
        .prepare(
            "SELECT operation_id FROM proposal_applies WHERE proposal_id=?1 ORDER BY operation_id",
        )?
        .query_map([intake.source_proposal.id.to_string()], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut selected = None;
    for id in ids {
        let journal = proposal_apply::read_journal(conn, crate::parse_id(id)?)?
            .ok_or_else(|| invalid("conflict Source approval journal disappeared"))?;
        if journal.approved.stamp() != intake.source_proposal
            || journal.receipt.as_ref().map(|r| r.outcome) != Some(ApplyOutcome::Applied)
        {
            continue;
        }
        if selected.is_some() {
            return Err(invalid("conflict Source Applied receipt is ambiguous"));
        }
        let draft = &journal.approved.draft;
        let binding = draft
            .inbox_source
            .as_ref()
            .ok_or_else(|| invalid("conflict prerequisite has no Source binding"))?;
        let extraction = binding
            .extraction
            .as_ref()
            .ok_or_else(|| invalid("conflict prerequisite has no extraction receipt"))?;
        let Some(NoteChange::Create { path, text, .. }) = draft.changes.first() else {
            return Err(invalid(
                "conflict prerequisite Source Create is unavailable",
            ));
        };
        if journal.undo.is_some()
            || binding.original != snapshot.original
            || binding.batch_id != snapshot.batch_id
            || binding.index != snapshot.index
            || binding.note_id != intake.source_note_id
            || extraction.snapshot_id != intake.snapshot_id
            || extraction.snapshot_sha256 != intake.snapshot_sha256
            || extraction.assets != assets
            || path != &intake.source_path
            || text != &capture.source_text
            || binding.markdown(&converted.markdown)? != capture.source_text
        {
            return Err(invalid(
                "conflict Source approval differs from its exact intake capture",
            ));
        }
        let proof = journal
            .prepared
            .as_ref()
            .and_then(|p| p.first())
            .ok_or_else(|| invalid("conflict Applied Source has no installed proof"))?;
        let vault = draft
            .vault
            .clone()
            .ok_or_else(|| invalid("conflict Applied Source has no bound vault"))?;
        selected = Some((
            SourceVersion {
                path: path.clone(),
                fingerprint: proof.clone(),
            },
            Some(vault),
        ));
    }
    selected.ok_or_else(|| invalid("conflict prerequisite has no exact historical Applied receipt"))
}

pub(super) fn validate_capture(conn: &Connection, draft: &FindingDraft) -> Result<()> {
    let FindingOrigin::InboxConflict { analysis_id, .. } = &draft.request.origin else {
        return Ok(());
    };
    let job = super::super::inbox_actions::reserved(conn, *analysis_id)?
        .ok_or_else(|| invalid("Inbox conflict analysis capture is unavailable"))?;
    let (source, vault) = selected_source(conn, &job.capture)?;
    if vault.is_some_and(|vault| vault != draft.vault) {
        return Err(invalid("conflict differs from its Applied Source vault"));
    }
    validate_selected_capture(draft, &job.capture, &source)
}

pub(super) fn validate_certificate_capture(
    draft: &FindingDraft,
    capture: &super::super::inbox_actions::InboxActionCapture,
) -> Result<()> {
    if !matches!(draft.request.origin, FindingOrigin::InboxConflict { .. }) {
        return Ok(());
    }
    if capture.intake.is_some() {
        return Err(invalid(
            "private intake conflicts require renewed saved-source review",
        ));
    }
    let selected = capture
        .source
        .as_ref()
        .ok_or_else(|| invalid("conflict needs a saved Source"))?;
    validate_selected_capture(draft, capture, selected)
}

fn validate_selected_capture(
    draft: &FindingDraft,
    capture: &InboxActionCapture,
    selected: &SourceVersion,
) -> Result<()> {
    let FindingOrigin::InboxConflict { source_quote, .. } = &draft.request.origin else {
        return Ok(());
    };
    let source = &draft.evidence[0];
    if capture.purpose != super::super::inbox_actions::InboxAnalysisPurpose::KnowledgeAndActions
        || source.source != *selected
        || source.note_id != Some(capture.note_id()?)
        || selected.fingerprint.len != capture.source_text.len() as u64
        || selected.fingerprint.sha256 != hash(capture.source_text.as_bytes())
        || crate::note_metadata::classify(&capture.source_text)?.history
        || selected
            .path
            .split('/')
            .next()
            .is_some_and(|part| part.eq_ignore_ascii_case("archive"))
        || source_quote.start_byte < crate::note_identity::body_start(&capture.source_text)?
        || capture
            .source_text
            .get(source_quote.start_byte..source_quote.end_byte)
            != Some(source_quote.quote.as_str())
    {
        return Err(invalid(
            "Inbox conflict differs from its retained nonhistorical Source body proof",
        ));
    }
    Ok(())
}

fn matches_note(record: &FindingRecord, vault: &VaultRecord, path: &str, note_id: Uuid) -> bool {
    matches!(
        record.draft.request.origin,
        FindingOrigin::InboxConflict { .. }
    ) && record.draft.vault == *vault
        && record
            .draft
            .evidence
            .iter()
            .any(|proof| proof.source.path == path || proof.note_id == Some(note_id))
}

impl WorkStore {
    /// Frozen saved proof, or the exact historical Applied intake installation.
    /// This does not establish fresh filesystem eligibility or refresh evidence.
    pub fn inbox_conflict_source(&self, analysis_id: Uuid) -> Result<SourceVersion> {
        nonnil(analysis_id)?;
        let tx = self.conn.unchecked_transaction()?;
        let job = super::super::inbox_actions::reserved(&tx, analysis_id)?
            .ok_or_else(|| invalid("Inbox conflict analysis capture is unavailable"))?;
        let (source, _) = selected_source(&tx, &job.capture)?;
        tx.commit()?;
        Ok(source)
    }
    /// Complete checked findings for one retained analysis, including closures.
    /// The workflow owns admission caps; this query never clips retained records.
    pub fn inbox_conflicts(&self, analysis_id: Uuid) -> Result<Vec<FindingRecord>> {
        nonnil(analysis_id)?;
        let tx = self.conn.unchecked_transaction()?;
        check_all(&tx)?;
        let mut entries = Vec::new();
        let mut statement =
            tx.prepare("SELECT id FROM findings ORDER BY created_at_ms DESC,id DESC")?;
        for id in statement.query_map([], |row| row.get::<_, String>(0))? {
            let record = read(&tx, crate::parse_id(id?)?)?
                .ok_or_else(|| invalid("listed finding disappeared"))?;
            if matches!(record.draft.request.origin, FindingOrigin::InboxConflict { analysis_id: id, .. } if id == analysis_id)
            {
                entries.push(record);
            }
        }
        drop(statement);
        tx.commit()?;
        Ok(entries)
    }

    /// Open Inbox conflicts matching the active exact vault and either retained
    /// path or stable identity. A closed matching cursor still anchors its page.
    pub fn note_conflicts(
        &self,
        vault: &VaultRecord,
        path: &str,
        note_id: Uuid,
        request: &FindingListRequest,
    ) -> Result<FindingPage> {
        request.validate()?;
        nonnil(note_id)?;
        nonnil(vault.id)?;
        super::path(path)?;
        if request.state != Some(FindingState::Open) {
            return Err(invalid("note conflict lookup requires Open state"));
        }
        let root = vault
            .root
            .to_str()
            .ok_or_else(|| invalid("conflict vault root must be UTF-8"))?;
        if !vault.root.is_absolute() || root.contains('\0') || root.len() > MAX_NOTE_BYTES {
            return Err(invalid(
                "conflict lookup needs a bounded absolute vault root",
            ));
        }
        let tx = self.conn.unchecked_transaction()?;
        check_all(&tx)?;
        let cursor = request
            .before
            .map(|id| {
                let record = read(&tx, id)?
                    .ok_or_else(|| Error::NotFound("conflict cursor does not exist".into()))?;
                if !matches_note(&record, vault, path, note_id) {
                    return Err(invalid("conflict cursor belongs to another lookup"));
                }
                Ok(record)
            })
            .transpose()?;
        let mut entries = Vec::new();
        let mut open_count = 0usize;
        let mut statement = tx.prepare(
            "SELECT id FROM findings WHERE state='open' ORDER BY created_at_ms DESC,id DESC",
        )?;
        for id in statement.query_map([], |row| row.get::<_, String>(0))? {
            let record = read(&tx, crate::parse_id(id?)?)?
                .ok_or_else(|| invalid("listed finding disappeared"))?;
            if !matches_note(&record, vault, path, note_id) {
                continue;
            }
            open_count = open_count
                .checked_add(1)
                .ok_or_else(|| invalid("conflict open count is out of range"))?;
            let before_cursor = cursor.as_ref().is_none_or(|anchor| {
                (record.created_at_ms, record.draft.request.id)
                    < (anchor.created_at_ms, anchor.draft.request.id)
            });
            if before_cursor && entries.len() <= request.limit {
                entries.push(record);
            }
        }
        drop(statement);
        let more = entries.len() > request.limit;
        entries.truncate(request.limit);
        let next_before = more.then(|| {
            entries
                .last()
                .expect("positive page limit")
                .draft
                .request
                .id
        });
        tx.commit()?;
        Ok(FindingPage {
            entries,
            next_before,
            open_count,
        })
    }
}
