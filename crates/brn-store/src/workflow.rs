//! Atomic projections for the import, approval, and grounded-chat workflow.
use super::*;
use rusqlite::Transaction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Approval {
    Approved,
    Draft,
    Withdrawn,
}
impl Approval {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Draft => "draft",
            Self::Withdrawn => "withdrawn",
        }
    }
    fn parse(s: &str) -> Result<Self> {
        match s {
            "approved" => Ok(Self::Approved),
            "draft" => Ok(Self::Draft),
            "withdrawn" => Ok(Self::Withdrawn),
            _ => Err(invalid("invalid stored approval")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportResult {
    pub source_id: Uuid,
    pub version_id: Uuid,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDocument {
    pub source_id: Uuid,
    pub version_id: Uuid,
    pub title: String,
    pub origin: String,
    pub bytes: Vec<u8>,
    pub sha256: [u8; 32],
    pub approval: Approval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatTurn {
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub question: String,
    pub profile: String,
    pub evidence_json: String,
    pub answer: Option<String>,
    pub status: OperationStatus,
    pub provider_turn_id: Option<String>,
    pub usage_json: Option<String>,
    #[serde(default)]
    pub evidence_currentness: EvidenceCurrentness,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceCurrentness {
    #[default]
    Unqualified,
    CurrentAtCompletion,
    StaleAtCompletion,
}
impl EvidenceCurrentness {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unqualified => "unqualified",
            Self::CurrentAtCompletion => "current_at_completion",
            Self::StaleAtCompletion => "stale_at_completion",
        }
    }
    fn parse(value: &str) -> Result<Self> {
        match value {
            "unqualified" => Ok(Self::Unqualified),
            "current_at_completion" => Ok(Self::CurrentAtCompletion),
            "stale_at_completion" => Ok(Self::StaleAtCompletion),
            _ => Err(invalid("invalid evidence currentness")),
        }
    }
}

pub(super) fn bind_operation(
    tx: &Transaction<'_>,
    op: Uuid,
    kind: &str,
    args: &[u8],
) -> Result<BeginOperation> {
    let digest = hash(args);
    let existing: Option<(String, Vec<u8>)> = tx
        .query_row(
            "SELECT kind,payload_hash FROM operations WHERE id=?1",
            [op.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match existing {
        Some((old_kind, old_hash)) if old_kind == kind && old_hash == digest => {
            Ok(BeginOperation::Existing)
        }
        Some(_) => Err(Error::OperationConflict(
            "operation ID conflicts with existing kind or payload".into(),
        )),
        None => {
            tx.execute(
                "INSERT INTO operations(id,kind,payload_hash,status) VALUES(?1,?2,?3,'pending')",
                params![op.to_string(), kind, digest.as_slice()],
            )?;
            Ok(BeginOperation::New)
        }
    }
}

impl Store {
    /// Imports UTF-8 text under a stable caller-provided origin. Versions remain immutable.
    pub fn import_text(
        &mut self,
        op: Uuid,
        origin: &str,
        title: &str,
        bytes: &[u8],
        approval: Approval,
    ) -> Result<ImportResult> {
        if origin.is_empty() || title.is_empty() || std::str::from_utf8(bytes).is_err() {
            return Err(invalid("import requires origin, title, and UTF-8 text"));
        }
        let args = encode_args(&[
            origin.as_bytes(),
            title.as_bytes(),
            bytes,
            approval.as_str().as_bytes(),
        ]);
        let digest = hash(bytes);
        let tx = self.conn.transaction()?;
        if bind_operation(&tx, op, "source.import", &args)? == BeginOperation::Existing {
            let row: Option<(String, String, i64)> = tx
                .query_row(
                    "SELECT source_id,version_id,changed FROM imports WHERE operation_id=?1",
                    [op.to_string()],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            let (source, version, changed) =
                row.ok_or_else(|| invalid("import operation has no result"))?;
            return Ok(ImportResult {
                source_id: parse_id(source)?,
                version_id: parse_id(version)?,
                changed: changed != 0,
            });
        }
        let existing: Option<(String, Option<String>, String)> = tx
            .query_row(
                "SELECT id,current_version_id,title FROM sources WHERE origin=?1",
                [origin],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (source_id, prior_id, old_title) = match existing {
            Some((id, prior, old_title)) => {
                (parse_id(id)?, prior.map(parse_id).transpose()?, old_title)
            }
            None => {
                let id = Uuid::new_v4();
                tx.execute(
                    "INSERT INTO sources(id,title,origin,approval) VALUES(?1,?2,?3,?4)",
                    params![id.to_string(), title, origin, approval.as_str()],
                )?;
                (id, None, title.to_owned())
            }
        };
        let unchanged = if let Some(id) = prior_id {
            let previous: Vec<u8> = tx.query_row(
                "SELECT sha256 FROM versions WHERE id=?1",
                [id.to_string()],
                |r| r.get(0),
            )?;
            previous == digest
        } else {
            false
        };
        let version_id = if unchanged {
            prior_id.unwrap()
        } else {
            let id = Uuid::new_v4();
            tx.execute(
                "INSERT INTO versions(id,source_id,parent_id,bytes,sha256) VALUES(?1,?2,?3,?4,?5)",
                params![
                    id.to_string(),
                    source_id.to_string(),
                    prior_id.map(|x| x.to_string()),
                    bytes,
                    digest.as_slice()
                ],
            )?;
            id
        };
        if prior_id.is_none() || !unchanged || old_title != title {
            tx.execute(
                "UPDATE sources SET title=?2,current_version_id=?3,approval=?4 WHERE id=?1",
                params![
                    source_id.to_string(),
                    title,
                    version_id.to_string(),
                    approval.as_str()
                ],
            )?;
        } else {
            tx.execute(
                "UPDATE sources SET approval=?2 WHERE id=?1",
                params![source_id.to_string(), approval.as_str()],
            )?;
        }
        tx.execute(
            "INSERT INTO imports(operation_id,source_id,version_id,changed) VALUES(?1,?2,?3,?4)",
            params![
                op.to_string(),
                source_id.to_string(),
                version_id.to_string(),
                i64::from(!unchanged)
            ],
        )?;
        tx.execute(
            "UPDATE operations SET status='completed' WHERE id=?1",
            [op.to_string()],
        )?;
        tx.commit()?;
        Ok(ImportResult {
            source_id,
            version_id,
            changed: !unchanged,
        })
    }

    pub fn documents(&self) -> Result<Vec<SourceDocument>> {
        let mut stmt = self.conn.prepare("SELECT s.id,v.id,s.title,s.origin,v.bytes,v.sha256,s.approval FROM sources s JOIN versions v ON v.id=s.current_version_id WHERE s.origin IS NOT NULL ORDER BY s.origin,s.id")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Vec<u8>>(4)?,
                r.get::<_, Vec<u8>>(5)?,
                r.get::<_, String>(6)?,
            ))
        })?;
        rows.map(|row| {
            let (source, version, title, origin, bytes, stored_hash, approval) = row?;
            if std::str::from_utf8(&bytes).is_err() || hash(&bytes).as_slice() != stored_hash {
                return Err(invalid("document content hash mismatch or invalid UTF-8"));
            }
            Ok(SourceDocument {
                source_id: parse_id(source)?,
                version_id: parse_id(version)?,
                title,
                origin,
                bytes,
                sha256: stored_hash
                    .try_into()
                    .map_err(|_| invalid("invalid document hash"))?,
                approval: Approval::parse(&approval)?,
            })
        })
        .collect()
    }

    pub fn set_approval(
        &mut self,
        op: Uuid,
        source: Uuid,
        expected_version: Uuid,
        approval: Approval,
    ) -> Result<()> {
        let args = encode_args(&[
            source.as_bytes(),
            expected_version.as_bytes(),
            approval.as_str().as_bytes(),
        ]);
        let tx = self.conn.transaction()?;
        if bind_operation(&tx, op, "source.approval", &args)? == BeginOperation::Existing {
            return Ok(());
        }
        let current: Option<String> = tx
            .query_row(
                "SELECT current_version_id FROM sources WHERE id=?1 AND origin IS NOT NULL",
                [source.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        if current.as_deref() != Some(&expected_version.to_string()) {
            return Err(invalid("source version changed before approval"));
        }
        tx.execute(
            "UPDATE sources SET approval=?2 WHERE id=?1",
            params![source.to_string(), approval.as_str()],
        )?;
        tx.execute(
            "UPDATE operations SET status='completed' WHERE id=?1",
            [op.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn attach_thread(&mut self, op: Uuid, session: Uuid, thread_id: &str) -> Result<()> {
        if thread_id.is_empty() {
            return Err(invalid("thread ID required"));
        }
        let args = encode_args(&[session.as_bytes(), thread_id.as_bytes()]);
        let tx = self.conn.transaction()?;
        if bind_operation(&tx, op, "session.attach", &args)? == BeginOperation::Existing {
            return Ok(());
        }
        let current: Option<Option<String>> = tx
            .query_row(
                "SELECT thread_id FROM sessions WHERE id=?1",
                [session.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        match current {
            Some(None) => {
                tx.execute(
                    "UPDATE sessions SET thread_id=?2 WHERE id=?1",
                    params![session.to_string(), thread_id],
                )?;
            }
            Some(Some(id)) if id == thread_id => {}
            Some(Some(_)) => return Err(invalid("session is attached to a different thread")),
            None => return Err(invalid("session does not exist")),
        }
        tx.execute(
            "UPDATE operations SET status='completed' WHERE id=?1",
            [op.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn prepare_turn(
        &mut self,
        op: Uuid,
        session: Uuid,
        question: &str,
        profile: &str,
        evidence_json: &str,
    ) -> Result<BeginOperation> {
        if question.is_empty() || profile.is_empty() || evidence_json.is_empty() {
            return Err(invalid("turn requires question, profile, and evidence"));
        }
        let args = encode_args(&[
            session.as_bytes(),
            question.as_bytes(),
            profile.as_bytes(),
            evidence_json.as_bytes(),
        ]);
        let tx = self.conn.transaction()?;
        let state = bind_operation(&tx, op, "chat.turn", &args)?;
        if state == BeginOperation::Existing {
            return Ok(state);
        }
        let thread_id: Option<Option<String>> = tx
            .query_row(
                "SELECT thread_id FROM sessions WHERE id=?1",
                [session.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        match thread_id {
            None => return Err(invalid("session does not exist")),
            Some(Some(id)) if !id.is_empty() => {}
            Some(_) => return Err(invalid("session has no provider thread")),
        }
        let active: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM chat_turns t JOIN operations o ON o.id=t.operation_id WHERE t.session_id=?1 AND o.status IN ('pending','running'))",
            [session.to_string()],
            |r| r.get(0),
        )?;
        if active {
            return Err(invalid("session already has an active chat turn"));
        }
        tx.execute("INSERT INTO chat_turns(operation_id,session_id,question,profile,evidence_json) VALUES(?1,?2,?3,?4,?5)",
            params![op.to_string(),session.to_string(),question,profile,evidence_json])?;
        tx.execute("INSERT INTO messages(id,session_id,ordinal,role,content) VALUES(?1,?2,(SELECT coalesce(max(ordinal)+1,0) FROM messages WHERE session_id=?2),'user',?3)",
            params![Uuid::new_v4().to_string(),session.to_string(),question.as_bytes()])?;
        tx.commit()?;
        Ok(state)
    }

    pub fn record_turn_started(&mut self, op: Uuid, turn_id: &str) -> Result<()> {
        if turn_id.is_empty() {
            return Err(invalid("provider turn ID required"));
        }
        let tx = self.conn.transaction()?;
        let row:Option<(String,Option<String>)>=tx.query_row("SELECT o.status,t.provider_turn_id FROM chat_turns t JOIN operations o ON o.id=t.operation_id WHERE t.operation_id=?1",[op.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let Some((status, old_id)) = row else {
            return Err(invalid("chat turn does not exist"));
        };
        if !matches!(
            OperationStatus::parse(&status)?,
            OperationStatus::Pending | OperationStatus::Running
        ) {
            return Err(invalid("chat turn is terminal"));
        }
        if old_id.as_deref().is_some_and(|id| id != turn_id) {
            return Err(invalid("provider turn ID conflicts"));
        }
        tx.execute(
            "UPDATE chat_turns SET provider_turn_id=?2 WHERE operation_id=?1",
            params![op.to_string(), turn_id],
        )?;
        tx.execute(
            "UPDATE operations SET status='running' WHERE id=?1",
            [op.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn complete_turn(
        &mut self,
        op: Uuid,
        status: OperationStatus,
        answer: &str,
        usage_json: Option<&str>,
    ) -> Result<()> {
        self.complete_turn_with_currentness(
            op,
            status,
            answer,
            usage_json,
            EvidenceCurrentness::Unqualified,
        )
    }

    pub fn complete_turn_with_currentness(
        &mut self,
        op: Uuid,
        status: OperationStatus,
        answer: &str,
        usage_json: Option<&str>,
        currentness: EvidenceCurrentness,
    ) -> Result<()> {
        if !matches!(
            status,
            OperationStatus::Completed | OperationStatus::Failed | OperationStatus::Interrupted
        ) {
            return Err(invalid("turn completion requires terminal status"));
        }
        let tx = self.conn.transaction()?;
        type CompletionRow = (String, String, Option<String>, Option<String>, String);
        let row:Option<CompletionRow>=tx.query_row("SELECT o.status,t.session_id,t.answer,t.usage_json,t.evidence_currentness FROM chat_turns t JOIN operations o ON o.id=t.operation_id WHERE t.operation_id=?1",[op.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
        let Some((old_status, session, old_answer, old_usage, old_currentness)) = row else {
            return Err(invalid("chat turn does not exist"));
        };
        let old_status = OperationStatus::parse(&old_status)?;
        if matches!(
            old_status,
            OperationStatus::Completed | OperationStatus::Failed | OperationStatus::Interrupted
        ) {
            if old_status == status
                && old_answer.as_deref() == Some(answer)
                && old_usage.as_deref() == usage_json
                && EvidenceCurrentness::parse(&old_currentness)? == currentness
            {
                return Ok(());
            }
            return Err(invalid("chat turn has conflicting terminal result"));
        }
        if status == OperationStatus::Completed {
            tx.execute("INSERT INTO messages(id,session_id,ordinal,role,content) VALUES(?1,?2,(SELECT coalesce(max(ordinal)+1,0) FROM messages WHERE session_id=?2),'assistant',?3)",
                params![Uuid::new_v4().to_string(),session,answer.as_bytes()])?;
        }
        tx.execute(
            "UPDATE chat_turns SET answer=?2,usage_json=?3,evidence_currentness=?4 WHERE operation_id=?1",
            params![op.to_string(), answer, usage_json, currentness.as_str()],
        )?;
        tx.execute(
            "UPDATE operations SET status=?2,terminal_data=?3 WHERE id=?1",
            params![
                op.to_string(),
                status.as_str(),
                encode_args(&[
                    answer.as_bytes(),
                    usage_json.unwrap_or("").as_bytes(),
                    &[u8::from(usage_json.is_some())]
                ])
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn turns(&self, session: Uuid) -> Result<Vec<ChatTurn>> {
        let mut stmt=self.conn.prepare("SELECT t.operation_id,t.question,t.profile,t.evidence_json,t.answer,o.status,t.provider_turn_id,t.usage_json,t.evidence_currentness FROM chat_turns t JOIN operations o ON o.id=t.operation_id WHERE t.session_id=?1 ORDER BY t.rowid")?;
        let rows = stmt.query_map([session.to_string()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, String>(8)?,
            ))
        })?;
        rows.map(|row| {
            let (
                id,
                question,
                profile,
                evidence_json,
                answer,
                status,
                provider_turn_id,
                usage_json,
                evidence_currentness,
            ) = row?;
            Ok(ChatTurn {
                operation_id: parse_id(id)?,
                session_id: session,
                question,
                profile,
                evidence_json,
                answer,
                status: OperationStatus::parse(&status)?,
                provider_turn_id,
                usage_json,
                evidence_currentness: EvidenceCurrentness::parse(&evidence_currentness)?,
            })
        })
        .collect()
    }
}
