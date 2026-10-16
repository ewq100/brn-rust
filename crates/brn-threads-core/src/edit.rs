use crate::{
    store::{apply_on, record_on},
    *,
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};
impl Store {
    /// Owner host: call on the first dirty transition. Stale bases remain
    /// recoverable and guarded. The session needs no token.
    pub fn begin_edit(&mut self, note: &str, base_version: u64) -> Result<EditSession> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let session = begin_on(&tx, note, base_version)?;
        tx.commit()?;
        Ok(session)
    }
    /// A caller-held session: `begin_edit` plus a token binding, in the same
    /// transaction. The token is returned only here; only its hash is stored.
    pub fn begin_caller_edit(&mut self, note: &str, base_version: u64) -> Result<IssuedEdit> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut session = begin_on(&tx, note, base_version)?;
        let token = EditToken::generate();
        tx.execute(
            "INSERT INTO edit_session_bindings(session,token_hash) VALUES (?1,?2)",
            params![session.id, token.hash()],
        )?;
        tx.commit()?;
        session.holder = EditHolder::Caller;
        Ok(IssuedEdit { session, token })
    }
    /// Test workspaces only: a new token for an open session, keeping its
    /// buffer and generation. Any earlier token stops working.
    pub fn reclaim_caller_edit(&mut self, session: &str) -> Result<IssuedEdit> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if crate::store::purpose_on(&tx)? != WorkspacePurpose::Test {
            return Err(Error::TestWorkspaceRequired);
        }
        let mut current = session_on(&tx, session)?;
        if current.closed {
            return Err(Error::Invalid(EDIT_CLOSED.into()));
        }
        let token = EditToken::generate();
        tx.execute(
            "INSERT INTO edit_session_bindings(session,token_hash) VALUES (?1,?2) ON CONFLICT(session) DO UPDATE SET token_hash=excluded.token_hash",
            params![session, token.hash()],
        )?;
        tx.commit()?;
        current.holder = EditHolder::Caller;
        Ok(IssuedEdit {
            session: current,
            token,
        })
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
    /// Owner host: writes any open session's buffer. Typing into a caller's
    /// session adopts it: its token stops working, so a delayed caller write
    /// cannot land over newer owner typing.
    pub fn update_buffer(
        &mut self,
        session: &str,
        generation: u64,
        markdown: &str,
    ) -> Result<BufferOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let outcome = update_on(&tx, session, generation, markdown)?;
        if let BufferOutcome::Updated(updated) = &outcome
            && updated.holder == EditHolder::Caller
        {
            tx.execute(
                "DELETE FROM edit_session_bindings WHERE session=?1",
                [session],
            )?;
        }
        tx.commit()?;
        Ok(match outcome {
            BufferOutcome::Updated(mut updated) => {
                updated.holder = EditHolder::OwnerHost;
                BufferOutcome::Updated(updated)
            }
            ignored => ignored,
        })
    }
    /// A caller's buffer write, with that session's token.
    pub fn update_caller_buffer(
        &mut self,
        session: &str,
        token: &EditToken,
        generation: u64,
        markdown: &str,
    ) -> Result<BufferOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_token(&tx, session, token)?;
        let outcome = update_on(&tx, session, generation, markdown)?;
        tx.commit()?;
        Ok(outcome)
    }
    /// Owner host: closes any open session, caller-held ones included.
    pub fn discard(&mut self, session: &str, generation: u64) -> Result<BufferOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let outcome = discard_on(&tx, session, generation)?;
        tx.commit()?;
        Ok(outcome)
    }
    /// A caller's discard, with that session's token.
    pub fn discard_caller_edit(
        &mut self,
        session: &str,
        token: &EditToken,
        generation: u64,
    ) -> Result<BufferOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_token(&tx, session, token)?;
        let outcome = discard_on(&tx, session, generation)?;
        tx.commit()?;
        Ok(outcome)
    }
    /// Owner Save uses the same checked write mechanism, with buffer closure atomic
    /// with its receipt. A retry of an already saved generation returns that receipt.
    /// Owner host: saves any session, caller-held ones included.
    pub fn save(&mut self, session: &str, generation: u64) -> Result<SaveOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let outcome = save_on(&tx, session, generation)?;
        tx.commit()?;
        Ok(outcome)
    }
    /// A caller's Save, with that session's token. Same identity, receipt
    /// and replay as the owner host's `save`; the token is not part of it.
    pub fn save_caller_edit(
        &mut self,
        session: &str,
        token: &EditToken,
        generation: u64,
    ) -> Result<SaveOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_token(&tx, session, token)?;
        let outcome = save_on(&tx, session, generation)?;
        tx.commit()?;
        Ok(outcome)
    }
}
/// `reclaim_caller_edit` on a closed session.
pub(crate) const EDIT_CLOSED: &str = "This edit is closed";
fn begin_on(tx: &Transaction<'_>, note: &str, base_version: u64) -> Result<EditSession> {
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
    // One open session per note. Name it, inside this same transaction,
    // instead of hitting the unique index; dropping `tx` writes nothing.
    let open: Option<(String, u64, bool)> = tx
        .query_row(
            "SELECT s.id,s.base,b.session IS NOT NULL FROM edit_sessions s LEFT JOIN edit_session_bindings b ON b.session=s.id WHERE s.note=?1 AND s.closed=0",
            [note],
            |r| Ok((r.get(0)?, crate::read_u64(r, 1)?, r.get(2)?)),
        )
        .optional()?;
    if let Some((session, base_version, caller)) = open {
        return Err(Error::EditSessionOpen {
            note: note.into(),
            session,
            base_version,
            holder: if caller {
                EditHolder::Caller
            } else {
                EditHolder::OwnerHost
            },
        });
    }
    let session = EditSession {
        id: uuid::Uuid::new_v4().to_string(),
        note: note.into(),
        base_version,
        generation: 0,
        markdown: content.markdown,
        closed: false,
        holder: EditHolder::OwnerHost,
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
    Ok(session)
}
/// The session exists (`Missing` otherwise) and `token` is its binding's.
/// Nothing is written either way. Hashes are compared in constant time.
fn check_token(tx: &Connection, session: &str, token: &EditToken) -> Result<()> {
    session_on(tx, session)?;
    let stored: Option<String> = tx
        .query_row(
            "SELECT token_hash FROM edit_session_bindings WHERE session=?1",
            [session],
            |r| r.get(0),
        )
        .optional()?;
    let problem = match stored {
        None => EditTokenProblem::OwnerHost,
        Some(stored) if same_hash(&stored, &token.hash()) => return Ok(()),
        Some(_) => EditTokenProblem::Mismatch,
    };
    Err(Error::EditToken {
        session: session.into(),
        problem,
    })
}
fn same_hash(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |diff, (x, y)| diff | (x ^ y))
            == 0
}
fn update_on(
    tx: &Connection,
    session: &str,
    generation: u64,
    markdown: &str,
) -> Result<BufferOutcome> {
    let old = session_on(tx, session)?;
    if old.closed || generation <= old.generation {
        return Ok(BufferOutcome::Ignored(old));
    }
    tx.execute(
        "UPDATE edit_sessions SET generation=?2,markdown=?3 WHERE id=?1 AND closed=0",
        params![session, crate::integer(generation)?, markdown],
    )?;
    Ok(BufferOutcome::Updated(session_on(tx, session)?))
}
fn discard_on(tx: &Connection, session: &str, generation: u64) -> Result<BufferOutcome> {
    let old = session_on(tx, session)?;
    if old.closed || old.generation != generation {
        return Ok(BufferOutcome::Ignored(old));
    }
    tx.execute(
        "UPDATE edit_sessions SET closed=1,markdown='' WHERE id=?1",
        [session],
    )?;
    Ok(BufferOutcome::Updated(session_on(tx, session)?))
}
fn save_on(tx: &Transaction<'_>, session: &str, generation: u64) -> Result<SaveOutcome> {
    let buffer = session_on(tx, session)?;
    let host_key = format!("editor-save:{session}:{generation}");
    let op: Option<String> = tx
        .query_row(
            "SELECT id FROM operations WHERE host_key=?1",
            [&host_key],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = &op
        && let Some(receipt) = crate::store::receipt_on(tx, &OperationId(id.clone()))?
    {
        return Ok(SaveOutcome::Saved(receipt));
    }
    if buffer.closed {
        return Ok(SaveOutcome::Closed(buffer));
    }
    if buffer.generation != generation {
        return Ok(SaveOutcome::GenerationMismatch(buffer));
    }
    let current =
        record_on(tx, &buffer.note)?.ok_or_else(|| Error::Missing(buffer.note.clone()))?;
    if current.version != buffer.base_version {
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
        tx,
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
        tx,
        &operation,
        &HostAuthority::owner("editor"),
        Some(session),
    )?;
    Ok(match outcome {
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
    })
}
fn session_on(conn: &Connection, id: &str) -> Result<EditSession> {
    conn.query_row(
        "SELECT s.id,s.note,s.base,s.generation,s.markdown,s.closed,b.session IS NOT NULL FROM edit_sessions s LEFT JOIN edit_session_bindings b ON b.session=s.id WHERE s.id=?1",
        [id],
        |r| {
            Ok(EditSession {
                id: r.get(0)?,
                note: r.get(1)?,
                base_version: crate::read_u64(r, 2)?,
                generation: crate::read_u64(r, 3)?,
                markdown: r.get(4)?,
                closed: r.get(5)?,
                holder: if r.get(6)? {
                    EditHolder::Caller
                } else {
                    EditHolder::OwnerHost
                },
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
