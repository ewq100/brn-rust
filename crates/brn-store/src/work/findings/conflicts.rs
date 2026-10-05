//! Narrow retained Inbox conflict validation and queries; no vault I/O.
use super::*;

pub(super) fn validate_capture(conn: &Connection, draft: &FindingDraft) -> Result<()> {
    let FindingOrigin::InboxConflict {
        analysis_id,
        source_quote,
        ..
    } = &draft.request.origin
    else {
        return Ok(());
    };
    let job = super::super::inbox_actions::reserved(conn, *analysis_id)?
        .ok_or_else(|| invalid("Inbox conflict analysis capture is unavailable"))?;
    let capture = &job.capture;
    let source = &draft.evidence[0];
    if capture.purpose != super::super::inbox_actions::InboxAnalysisPurpose::KnowledgeAndActions
        || source.source != capture.source
        || source.note_id != Some(capture.note_id()?)
        || crate::note_metadata::classify(&capture.source_text)?.history
        || capture
            .source
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
