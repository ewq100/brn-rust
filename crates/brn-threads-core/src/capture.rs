//! Saved chat evidence and a durable owner-review boundary for Capture outcomes.
use crate::*;
use serde::{Deserialize, Serialize};

pub const CAPTURE_REQUEST_ROLE: &str = "capture-request";
pub const MAX_CAPTURE_BYTES: usize = 128 * 1024;
pub const MAX_CAPTURE_HISTORY_RECORDS: usize = 1024;

impl Store {
    /// Capture-only preflight, including host metadata hidden by scoped paging.
    /// Returns no message bodies; reuses the existing active-chat-message filter.
    pub fn capture_history_within_limit(&self, thread: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM (SELECT 1 FROM records WHERE json_type(body,'$.data.Message') IS NOT NULL AND json_extract(body,'$.data.Message.thread')=?1 AND json_extract(body,'$.archived')=0 LIMIT ?2)",
            rusqlite::params![thread, (MAX_CAPTURE_HISTORY_RECORDS + 1) as i64],
            |row| row.get(0),
        )?;
        Ok(count <= MAX_CAPTURE_HISTORY_RECORDS as i64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureSnapshot {
    pub version: u8,
    pub request_message: String,
    pub thread: String,
    pub private_context: bool,
    pub project: Option<String>,
    pub messages: Vec<Record>,
    pub context: Vec<RevisionInput>,
}
impl CaptureSnapshot {
    pub fn message_text(&self) -> Result<String> {
        let text = serde_json::to_string(self)?;
        if self.version != 1 || self.thread.is_empty() || self.request_message.is_empty()
            || self.messages.is_empty() || self.messages.len() > 256 || text.len() > MAX_CAPTURE_BYTES
            || self.messages.iter().any(|r| r.archived || r.version == 0 || !matches!(&r.data, RecordData::Message(m) if m.thread == self.thread && matches!(m.role.as_str(), "owner" | "assistant")))
            || !self.context.iter().any(|i| i.record == self.thread && i.expectation == InputExpectation::ChatContext)
        { return Err(Error::Invalid("Capture needs a nonempty posted conversation within 256 messages and 128 KiB; start a focused chat for larger conversations".into())); }
        Ok(text)
    }
    pub fn from_message(record: &Record) -> Result<Self> {
        let RecordData::Message(message) = &record.data else {
            return Err(Error::Invalid("Not a Capture request".into()));
        };
        if record.archived
            || record.version != 1
            || message.role != CAPTURE_REQUEST_ROLE
            || message.text.len() > MAX_CAPTURE_BYTES
        {
            return Err(Error::Invalid("Capture request is unavailable".into()));
        }
        let snapshot: Self = serde_json::from_str(&message.text)?;
        snapshot.message_text()?;
        if snapshot.request_message != record.id || snapshot.thread != message.thread {
            return Err(Error::Invalid("Capture request identity differs".into()));
        }
        Ok(snapshot)
    }
    pub fn current_inputs(&self) -> Vec<RevisionInput> {
        let mut inputs = self.context.clone();
        inputs.push(RevisionInput {
            record: self.request_message.clone(),
            version: 1,
            expectation: InputExpectation::Current,
        });
        inputs.extend(self.messages.iter().map(|r| RevisionInput {
            record: r.id.clone(),
            version: r.version,
            expectation: InputExpectation::Current,
        }));
        inputs
    }
}
pub(crate) fn markers(
    request: &ChangeRequest,
    mut revision: impl FnMut(&str, u64) -> Result<Option<Record>>,
) -> Result<Vec<(RevisionInput, CaptureSnapshot)>> {
    let mut result = vec![];
    for input in &request.inputs {
        if let Some(record) = revision(&input.record, input.version)?
            && matches!(&record.data,RecordData::Message(m) if m.role==CAPTURE_REQUEST_ROLE)
        {
            let snapshot = CaptureSnapshot::from_message(&record)?;
            for observed in &snapshot.messages {
                if revision(&observed.id, observed.version)?.as_ref() != Some(observed) {
                    return Err(Error::Invalid(
                        "Capture snapshot differs from canonical posted evidence".into(),
                    ));
                }
            }

            if input.expectation != InputExpectation::Current
                || !snapshot
                    .current_inputs()
                    .iter()
                    .all(|i| request.inputs.contains(i))
            {
                return Err(Error::Invalid(
                    "Capture evidence and review binding cannot be removed or downgraded".into(),
                ));
            }
            if !host_run_start(request)
                && request.writes.iter().any(|w| {
                    w.archived
                        || !matches!(
                            w.data,
                            RecordData::Note(_) | RecordData::Action(_) | RecordData::Link(_)
                        )
                })
            {
                return Err(Error::Invalid("Capture proposes Notes, Actions and Links; archiving and other operations are separate owner actions".into()));
            }
            result.push((input.clone(), snapshot));
        }
    }
    Ok(result)
}

pub(crate) fn validate_visibility(
    request: &ChangeRequest,
    snapshot: &CaptureSnapshot,
    rows: &[Record],
    mut revision: impl FnMut(&str, u64) -> Result<Option<Record>>,
) -> Result<()> {
    let visible = |id: &str| {
        rows.iter()
            .find(|r| r.id == id)
            .is_some_and(|r| crate::context::visible(r, rows, Some(&snapshot.thread)))
    };
    let created: std::collections::BTreeSet<_> = request
        .writes
        .iter()
        .filter(|w| w.expected_version.is_none())
        .map(|w| w.id.as_str())
        .collect();
    for input in &request.inputs {
        if !visible(&input.record)
            || !revision(&input.record, input.version)?
                .is_some_and(|r| crate::context::visible(&r, rows, Some(&snapshot.thread)))
        {
            return Err(Error::Invalid(
                "Capture evidence is no longer visible in its original chat".into(),
            ));
        }
    }
    for write in &request.writes {
        if let Some(version) = write.expected_version {
            if !visible(&write.id)
                || !revision(&write.id, version)?
                    .is_some_and(|r| crate::context::visible(&r, rows, Some(&snapshot.thread)))
            {
                return Err(Error::Invalid(
                    "Capture target is no longer visible in its original chat".into(),
                ));
            }
        } else {
            let chat = match &write.data {
                RecordData::Note(n) => Some(&n.chat),
                RecordData::Action(a) => Some(&a.chat),
                _ => None,
            };
            if chat.is_some_and(|chat| {
                chat.as_deref() != snapshot.private_context.then_some(snapshot.thread.as_str())
            }) {
                return Err(Error::Invalid(
                    "Capture cannot change its private or general destination".into(),
                ));
            }
        }
        if let RecordData::Link(link) = &write.data {
            for id in [&link.from, &link.to] {
                if !created.contains(id.as_str()) && !visible(id) {
                    return Err(Error::Invalid(
                        "Capture link is outside its original chat scope".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

// Starting a Run binds the same snapshot atomically without being a proposal.
// Runtime model tools cannot write Run or ChatControl records.
pub(crate) fn host_run_start(request: &ChangeRequest) -> bool {
    request
        .writes
        .iter()
        .any(|w| matches!(w.data, RecordData::Run(_)))
        && request.writes.iter().all(|w| {
            !w.archived
                && matches!(
                    w.data,
                    RecordData::Run(_) | RecordData::Thread(_) | RecordData::ChatControl(_)
                )
        })
}

pub(crate) fn visible(snapshot: &CaptureSnapshot, rows: &[Record], chat: Option<&str>) -> bool {
    chat == Some(snapshot.thread.as_str())
        && crate::project_for_chat(rows, &snapshot.thread)
            .filter(|r| !r.archived)
            .map(|r| r.id.as_str())
            == snapshot.project.as_deref()
        && (snapshot.project.is_some() || crate::context::chat_private(rows, &snapshot.thread))
            == snapshot.private_context
}
