use crate::{
    store::{apply_on, record_on},
    *,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
impl Store {
    /// Call on the first dirty transition. Stale bases remain recoverable and guarded.
    pub fn begin_edit(&mut self, note: &str, base_version: u64) -> Result<EditSession> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let base: Option<String> = tx
            .query_row(
                "SELECT body FROM revisions WHERE id=?1 AND version=?2",
                params![note, crate::integer(base_version)?],
                |r| r.get(0),
            )
            .optional()?;
        let base: Record = serde_json::from_str(
            &base.ok_or_else(|| Error::Missing(format!("{note}@{base_version}")))?,
        )?;
        let RecordData::Note(content) = base.data else {
            return Err(Error::Invalid("editing requires a note".into()));
        };
        let session = EditSession {
            id: uuid::Uuid::new_v4().to_string(),
            note: note.into(),
            base_version,
            generation: 0,
            markdown: content.markdown,
            closed: false,
        };
        tx.execute(
            "INSERT INTO edit_sessions(id,note,base,generation,markdown) VALUES (?1,?2,?3,0,?4)",
            params![
                session.id,
                note,
                crate::integer(base_version)?,
                session.markdown
            ],
        )?;
        tx.commit()?;
        Ok(session)
    }
    pub fn edit_session(&self, id: &str) -> Result<EditSession> {
        session_on(&self.conn, id)
    }
    pub fn recovery_buffers(&self) -> Result<Vec<EditSession>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM edit_sessions WHERE closed=0 ORDER BY id")?;
        let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
        ids.map(|id| self.edit_session(&id?)).collect()
    }
    pub fn update_buffer(
        &mut self,
        session: &str,
        generation: u64,
        markdown: &str,
    ) -> Result<BufferOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let old = session_on(&tx, session)?;
        if old.closed || generation <= old.generation {
            tx.commit()?;
            return Ok(BufferOutcome::Ignored(old));
        }
        tx.execute(
            "UPDATE edit_sessions SET generation=?2,markdown=?3 WHERE id=?1 AND closed=0",
            params![session, crate::integer(generation)?, markdown],
        )?;
        let updated = session_on(&tx, session)?;
        tx.commit()?;
        Ok(BufferOutcome::Updated(updated))
    }
    pub fn discard(&mut self, session: &str, generation: u64) -> Result<BufferOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let old = session_on(&tx, session)?;
        if old.closed || old.generation != generation {
            tx.commit()?;
            return Ok(BufferOutcome::Ignored(old));
        }
        tx.execute(
            "UPDATE edit_sessions SET closed=1,markdown='' WHERE id=?1",
            [session],
        )?;
        let closed = session_on(&tx, session)?;
        tx.commit()?;
        Ok(BufferOutcome::Updated(closed))
    }
    /// Owner Save uses the same checked write mechanism, with buffer closure atomic
    /// with its receipt. A retry of an already saved generation returns that receipt.
    pub fn save(&mut self, session: &str, generation: u64) -> Result<SaveOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let buffer = session_on(&tx, session)?;
        let host_key = format!("editor-save:{session}:{generation}");
        let op: Option<String> = tx
            .query_row(
                "SELECT id FROM operations WHERE host_key=?1",
                [&host_key],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = &op
            && let Some(receipt) = crate::store::receipt_on(&tx, &OperationId(id.clone()))?
        {
            tx.commit()?;
            return Ok(SaveOutcome::Saved(receipt));
        }
        if buffer.closed {
            tx.commit()?;
            return Ok(SaveOutcome::Closed(buffer));
        }
        if buffer.generation != generation {
            tx.commit()?;
            return Ok(SaveOutcome::GenerationMismatch(buffer));
        }
        let current =
            record_on(&tx, &buffer.note)?.ok_or_else(|| Error::Missing(buffer.note.clone()))?;
        if current.version != buffer.base_version {
            tx.commit()?;
            return Ok(SaveOutcome::Stale(buffer));
        }
        let RecordData::Note(mut note) = current.data else {
            return Err(Error::Invalid("not a note".into()));
        };
        let prior_markdown = note.markdown.clone();
        note.markdown = buffer.markdown.clone();
        if note.markdown != prior_markdown {
            note.confirmed = false;
        }
        let operation = OperationId(op.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()));
        // Allocation is durable before the ordinary prepare dispatch. Save has no
        // external prepare boundary: allocation/candidate/write/closure are one tx.
        let mut writes = vec![Put {
            id: buffer.note.clone(),
            expected_version: Some(buffer.base_version),
            archived: current.archived,
            data: RecordData::Note(note),
        }];
        writes.extend(map_comments(
            &tx,
            &buffer.note,
            buffer.base_version,
            buffer
                .base_version
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("note version exhausted".into()))?,
            &prior_markdown,
            &buffer.markdown,
        )?);
        let request = ChangeRequest {
            reason: "Owner Save".into(),
            writes,
            inputs: vec![],
        };
        let body = serde_json::to_string(&request)?;
        let hash = format!("{:x}", Sha256::digest(body.as_bytes()));
        tx.execute(
            "INSERT INTO operations(id,host_key,hash,request) VALUES (?1,?2,?3,?4)",
            params![operation.0, host_key, hash, body],
        )?;
        let outcome = apply_on(
            &tx,
            &operation,
            &HostAuthority::owner("editor"),
            Some(session),
        )?;
        let result = match outcome {
            ApplyOutcome::Applied(receipt) => {
                tx.execute(
                    "UPDATE edit_sessions SET closed=1,markdown='' WHERE id=?1 AND generation=?2",
                    params![session, crate::integer(generation)?],
                )?;
                SaveOutcome::Saved(receipt)
            }
            ApplyOutcome::Stale { .. } => SaveOutcome::Stale(buffer),
            ApplyOutcome::Deferred { guarded } => SaveOutcome::Deferred { guarded },
            ApplyOutcome::Superseded { .. } => {
                return Err(Error::Invalid("owner save unexpectedly superseded".into()));
            }
            ApplyOutcome::NeedsReview { .. } => {
                return Err(Error::Invalid("owner save unexpectedly denied".into()));
            }
        };
        tx.commit()?;
        Ok(result)
    }
}
fn session_on(conn: &Connection, id: &str) -> Result<EditSession> {
    conn.query_row(
        "SELECT id,note,base,generation,markdown,closed FROM edit_sessions WHERE id=?1",
        [id],
        |r| {
            Ok(EditSession {
                id: r.get(0)?,
                note: r.get(1)?,
                base_version: crate::read_u64(r, 2)?,
                generation: crate::read_u64(r, 3)?,
                markdown: r.get(4)?,
                closed: r.get(5)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| Error::Missing(id.into()))
}

fn map_comments(
    tx: &Connection,
    note_id: &str,
    base: u64,
    next: u64,
    old: &str,
    new: &str,
) -> Result<Vec<Put>> {
    let prefix = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let suffix = old[prefix..]
        .chars()
        .rev()
        .zip(new[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let old_end = old.len() - suffix;
    let new_end = new.len() - suffix;
    let mut stmt =
        tx.prepare("SELECT body FROM records WHERE json_extract(body,'$.data.Comment.note')=?1")?;
    let mut writes = vec![];
    for body in stmt.query_map([note_id], |r| r.get::<_, String>(0))? {
        let record: Record = serde_json::from_str(&body?)?;
        if record.archived {
            continue;
        }
        let RecordData::Comment(mut comment) = record.data.clone() else {
            continue;
        };
        if comment.unresolved
            || (comment.range.is_none()
                && comment.mapped_range.is_none()
                && comment.quote.is_empty())
        {
            continue;
        }
        let range = comment.mapped_range.or(comment.range);
        let valid_base = comment.mapped_version.unwrap_or(comment.base_version) == base;
        let mapped = range
            .filter(|(start, end)| {
                valid_base && old.get(*start..*end) == Some(comment.quote.as_str())
            })
            .and_then(|(start, end)| {
                if old_end <= start {
                    let mapped_start = start.checked_sub(old_end)?.checked_add(new_end)?;
                    let mapped_end = end.checked_sub(old_end)?.checked_add(new_end)?;
                    Some((mapped_start, mapped_end))
                } else if prefix >= end {
                    Some((start, end))
                } else {
                    None
                }
            })
            .filter(|(start, end)| new.get(*start..*end) == Some(comment.quote.as_str()));
        comment.mapped_version = Some(next);
        comment.mapped_range = mapped;
        comment.unresolved = mapped.is_none();
        writes.push(Put {
            id: record.id,
            expected_version: Some(record.version),
            archived: record.archived,
            data: RecordData::Comment(comment),
        });
    }
    Ok(writes)
}
