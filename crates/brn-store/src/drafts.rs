//! Durable working drafts and immutable checkpoint/candidate revisions.
use super::*;
use rusqlite::Transaction;

pub const MAX_DRAFT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftStamp {
    pub base_revision: Uuid,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    pub id: Uuid,
    pub title: String,
    pub stamp: DraftStamp,
    pub text: String,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevisionKind {
    Checkpoint,
    Candidate,
}
impl RevisionKind {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "checkpoint" => Ok(Self::Checkpoint),
            "candidate" => Ok(Self::Candidate),
            _ => Err(invalid("invalid draft revision kind")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftRevision {
    pub id: Uuid,
    pub draft_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub kind: RevisionKind,
    pub text: String,
    pub sha256: [u8; 32],
    pub origin_turn: Option<Uuid>,
}

fn checked_text(text: &str) -> Result<[u8; 32]> {
    if text.len() > MAX_DRAFT_BYTES {
        return Err(invalid("draft text exceeds 1 MiB"));
    }
    Ok(hash(text.as_bytes()))
}
fn checked_generation(generation: u64) -> Result<i64> {
    i64::try_from(generation).map_err(|_| invalid("draft generation exceeds SQLite range"))
}
fn stored_hash(text: &str, digest: Vec<u8>) -> Result<[u8; 32]> {
    let stored: [u8; 32] = digest
        .try_into()
        .map_err(|_| invalid("invalid draft hash length"))?;
    if text.len() > MAX_DRAFT_BYTES || hash(text.as_bytes()) != stored {
        return Err(invalid("draft content hash mismatch or oversize"));
    }
    Ok(stored)
}
fn read_draft(conn: &Connection, id: Uuid) -> Result<Option<Draft>> {
    type Row = (String, String, i64, String, Vec<u8>);
    let row: Option<Row> = conn
        .query_row(
            "SELECT title,base_revision_id,generation,text,sha256 FROM drafts WHERE id=?1",
            [id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    row.map(|(title, base, generation, text, digest)| {
        if title.trim().is_empty() || generation < 0 {
            return Err(invalid("invalid stored draft metadata"));
        }
        let base_revision = parse_id(base)?;
        let belongs: Option<String> = conn
            .query_row(
                "SELECT draft_id FROM draft_revisions WHERE id=?1 AND kind='checkpoint'",
                [base_revision.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        if belongs.as_deref() != Some(&id.to_string()) {
            return Err(invalid("draft base revision mismatch"));
        }
        Ok(Draft {
            id,
            title,
            stamp: DraftStamp {
                base_revision,
                generation: generation as u64,
            },
            sha256: stored_hash(&text, digest)?,
            text,
        })
    })
    .transpose()
}
fn read_revision(conn: &Connection, id: Uuid) -> Result<Option<DraftRevision>> {
    type Row = (
        String,
        Option<String>,
        String,
        String,
        Vec<u8>,
        Option<String>,
    );
    let row: Option<Row> = conn.query_row("SELECT draft_id,parent_id,kind,text,sha256,origin_turn FROM draft_revisions WHERE id=?1", [id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
    row.map(|(draft, parent, kind, text, digest, turn)| {
        let draft_id = parse_id(draft)?;
        let parent_id = parent.map(parse_id).transpose()?;
        if let Some(parent) = parent_id {
            let parent_draft: Option<String> = conn
                .query_row(
                    "SELECT draft_id FROM draft_revisions WHERE id=?1",
                    [parent.to_string()],
                    |r| r.get(0),
                )
                .optional()?;
            if parent_draft.as_deref() != Some(&draft_id.to_string()) {
                return Err(invalid("revision parent belongs to another draft"));
            }
        }
        let kind = RevisionKind::parse(&kind)?;
        let origin_turn = turn.map(parse_id).transpose()?;
        if (kind == RevisionKind::Candidate) != origin_turn.is_some() {
            return Err(invalid("revision origin does not match kind"));
        }
        Ok(DraftRevision {
            id,
            draft_id,
            parent_id,
            kind,
            sha256: stored_hash(&text, digest)?,
            text,
            origin_turn,
        })
    })
    .transpose()
}
fn saved_result<T: serde::de::DeserializeOwned>(
    tx: &Transaction<'_>,
    op: Uuid,
    kind: &str,
) -> Result<T> {
    let (stored_kind, bytes): (String, Vec<u8>) = tx
        .query_row(
            "SELECT result_kind,result_json FROM draft_results WHERE operation_id=?1",
            [op.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| invalid("draft operation has no result"))?;
    if stored_kind != kind {
        return Err(invalid("draft operation result kind mismatch"));
    }
    serde_json::from_slice(&bytes).map_err(|_| invalid("invalid stored draft result"))
}
fn finish_result<T: Serialize>(
    tx: &Transaction<'_>,
    op: Uuid,
    kind: &str,
    result: &T,
) -> Result<()> {
    let bytes = serde_json::to_vec(result).map_err(|_| invalid("cannot encode draft result"))?;
    tx.execute(
        "INSERT INTO draft_results(operation_id,result_kind,result_json) VALUES(?1,?2,?3)",
        params![op.to_string(), kind, bytes],
    )?;
    tx.execute(
        "UPDATE operations SET status='completed' WHERE id=?1",
        [op.to_string()],
    )?;
    Ok(())
}

impl Store {
    pub fn create_draft(&mut self, op: Uuid, title: &str, text: &str) -> Result<Draft> {
        if title.trim().is_empty() {
            return Err(invalid("draft title is required"));
        }
        let digest = checked_text(text)?;
        let args = encode_args(&[title.as_bytes(), text.as_bytes()]);
        let tx = self.conn.transaction()?;
        if workflow::bind_operation(&tx, op, "draft.create", &args)? == BeginOperation::Existing {
            return saved_result(&tx, op, "draft");
        }
        let id = Uuid::new_v4();
        let root = Uuid::new_v4();
        tx.execute("INSERT INTO drafts(id,title,base_revision_id,generation,text,sha256) VALUES(?1,?2,?3,0,?4,?5)", params![id.to_string(),title,root.to_string(),text,digest.as_slice()])?;
        tx.execute("INSERT INTO draft_revisions(id,draft_id,parent_id,kind,text,sha256) VALUES(?1,?2,NULL,'checkpoint',?3,?4)", params![root.to_string(),id.to_string(),text,digest.as_slice()])?;
        let result = Draft {
            id,
            title: title.to_owned(),
            stamp: DraftStamp {
                base_revision: root,
                generation: 0,
            },
            text: text.to_owned(),
            sha256: digest,
        };
        finish_result(&tx, op, "draft", &result)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn drafts(&self) -> Result<Vec<Draft>> {
        let mut stmt = self.conn.prepare("SELECT id FROM drafts ORDER BY rowid")?;
        let ids: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        ids.into_iter()
            .map(|id| {
                read_draft(&self.conn, parse_id(id)?)
                    .and_then(|v| v.ok_or_else(|| invalid("draft vanished during read")))
            })
            .collect()
    }
    pub fn draft(&self, id: Uuid) -> Result<Option<Draft>> {
        read_draft(&self.conn, id)
    }
    pub fn save_draft(
        &mut self,
        op: Uuid,
        id: Uuid,
        expected: DraftStamp,
        generation: u64,
        text: &str,
    ) -> Result<Draft> {
        self.write_draft(op, id, expected, generation, text, false)
    }
    pub fn checkpoint_draft(
        &mut self,
        op: Uuid,
        id: Uuid,
        expected: DraftStamp,
        generation: u64,
        text: &str,
    ) -> Result<Draft> {
        self.write_draft(op, id, expected, generation, text, true)
    }
    fn write_draft(
        &mut self,
        op: Uuid,
        id: Uuid,
        expected: DraftStamp,
        generation: u64,
        text: &str,
        checkpoint: bool,
    ) -> Result<Draft> {
        let digest = checked_text(text)?;
        let generation_sql = checked_generation(generation)?;
        let expected_sql = checked_generation(expected.generation)?;
        let action = if checkpoint {
            "draft.checkpoint"
        } else {
            "draft.save"
        };
        let args = encode_args(&[
            id.as_bytes(),
            expected.base_revision.as_bytes(),
            &expected.generation.to_be_bytes(),
            &generation.to_be_bytes(),
            text.as_bytes(),
        ]);
        let tx = self.conn.transaction()?;
        if workflow::bind_operation(&tx, op, action, &args)? == BeginOperation::Existing {
            return saved_result(&tx, op, "draft");
        }
        let current = read_draft(&tx, id)?.ok_or_else(|| invalid("draft does not exist"))?;
        if current.stamp != expected {
            return Err(invalid("draft changed before write"));
        }
        if checkpoint {
            if generation < expected.generation {
                return Err(invalid("checkpoint generation moved backward"));
            }
        } else if generation <= expected.generation {
            return Err(invalid("save generation must advance"));
        }
        let base = if checkpoint {
            Uuid::new_v4()
        } else {
            expected.base_revision
        };
        if checkpoint {
            tx.execute("INSERT INTO draft_revisions(id,draft_id,parent_id,kind,text,sha256) VALUES(?1,?2,?3,'checkpoint',?4,?5)", params![base.to_string(),id.to_string(),expected.base_revision.to_string(),text,digest.as_slice()])?;
        }
        let changed = tx.execute("UPDATE drafts SET base_revision_id=?2,generation=?3,text=?4,sha256=?5 WHERE id=?1 AND base_revision_id=?6 AND generation=?7", params![id.to_string(),base.to_string(),generation_sql,text,digest.as_slice(),expected.base_revision.to_string(),expected_sql])?;
        if changed != 1 {
            return Err(invalid("draft changed before write"));
        }
        let result = Draft {
            id,
            title: current.title,
            stamp: DraftStamp {
                base_revision: base,
                generation,
            },
            text: text.to_owned(),
            sha256: digest,
        };
        finish_result(&tx, op, "draft", &result)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn draft_revisions(&self, id: Uuid) -> Result<Vec<DraftRevision>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM draft_revisions WHERE draft_id=?1 ORDER BY rowid")?;
        let ids: Vec<String> = stmt
            .query_map([id.to_string()], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        ids.into_iter()
            .map(|id| {
                read_revision(&self.conn, parse_id(id)?)
                    .and_then(|v| v.ok_or_else(|| invalid("revision vanished during read")))
            })
            .collect()
    }
    pub fn draft_revision(&self, id: Uuid) -> Result<Option<DraftRevision>> {
        read_revision(&self.conn, id)
    }
    pub fn candidate_from_turn(
        &mut self,
        op: Uuid,
        id: Uuid,
        parent: Uuid,
        turn: Uuid,
    ) -> Result<DraftRevision> {
        let args = encode_args(&[id.as_bytes(), parent.as_bytes(), turn.as_bytes()]);
        let tx = self.conn.transaction()?;
        if workflow::bind_operation(&tx, op, "draft.candidate", &args)? == BeginOperation::Existing
        {
            return saved_result(&tx, op, "revision");
        }
        if read_draft(&tx, id)?.is_none() {
            return Err(invalid("draft does not exist"));
        }
        let parent_revision =
            read_revision(&tx, parent)?.ok_or_else(|| invalid("parent revision does not exist"))?;
        if parent_revision.draft_id != id {
            return Err(invalid("parent revision must belong to draft"));
        }
        let row: Option<(String,Option<String>)> = tx.query_row("SELECT o.status,t.answer FROM chat_turns t JOIN operations o ON o.id=t.operation_id WHERE t.operation_id=?1", [turn.to_string()], |r| Ok((r.get(0)?,r.get(1)?))).optional()?;
        let (status, answer) = row.ok_or_else(|| invalid("chat turn does not exist"))?;
        if status != "completed" || answer.is_none() {
            return Err(invalid("candidate requires completed answer"));
        }
        let text = answer.unwrap();
        let digest = checked_text(&text)?;
        let revision = DraftRevision {
            id: Uuid::new_v4(),
            draft_id: id,
            parent_id: Some(parent),
            kind: RevisionKind::Candidate,
            text,
            sha256: digest,
            origin_turn: Some(turn),
        };
        tx.execute("INSERT INTO draft_revisions(id,draft_id,parent_id,kind,text,sha256,origin_turn) VALUES(?1,?2,?3,'candidate',?4,?5,?6)", params![revision.id.to_string(),id.to_string(),parent.to_string(),&revision.text,digest.as_slice(),turn.to_string()])?;
        finish_result(&tx, op, "revision", &revision)?;
        tx.commit()?;
        Ok(revision)
    }
}
