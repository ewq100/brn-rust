//! Explicit owner capture of a reviewed chat reply; no model invocation.
use crate::*;

impl Workspace {
    pub fn save_chat_decision(
        &mut self,
        key: &str,
        message: &Record,
        title: &str,
        markdown: &str,
    ) -> AppResult<Record> {
        if title.trim().is_empty() || markdown.trim().is_empty() {
            return Err(AppError::Invalid(
                "Give the decision a title and text.".into(),
            ));
        }
        let binding = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(message, title, markdown))?)
        );
        let op = self
            .store
            .allocate_bound_operation(&format!("chat-decision:{key}"), &binding)?;
        let owner = HostAuthority::owner("owner saved a reviewed reply as a decision");
        // Replays precede mutable source checks, including a later archive or Undo.
        if self.store.prepared(&op).is_ok() {
            return applied_record(self.store.apply(&op, &owner)?);
        }
        let RecordData::Message(reply) = &message.data else {
            return Err(AppError::Invalid("Choose a saved assistant reply.".into()));
        };
        if message.archived
            || reply.role != "assistant"
            || self.store.record(&message.id)?.as_ref() != Some(message)
        {
            return Err(AppError::Invalid(
                "The saved reply changed. Reopen Save as decision.".into(),
            ));
        }
        let thread = self
            .store
            .record(&reply.thread)?
            .filter(|r| {
                !r.archived
                    && matches!(&r.data, RecordData::Thread(t) if t.state == ThreadState::Open)
            })
            .ok_or_else(|| {
                AppError::Invalid("Unarchive this chat before saving a decision.".into())
            })?;
        let mut inputs = vec![
            RevisionInput {
                record: message.id.clone(),
                version: message.version,
                expectation: InputExpectation::Current,
            },
            RevisionInput {
                record: thread.id.clone(),
                version: thread.version,
                expectation: InputExpectation::Current,
            },
        ];
        let records = self.records()?;
        for id in managed_asset_ids(markdown) {
            let asset = records.iter().find(|r| r.id == id && !r.archived && matches!(r.data, RecordData::Asset(_)))
                .filter(|r| record_in_chat(r, &records, None))
                .ok_or_else(|| AppError::Invalid("A figure is still chat-only or unavailable. Keep its document in BRN, or remove the figure from the decision.".into()))?;
            let RecordData::Asset(data) = &asset.data else {
                unreachable!()
            };
            for id in [
                Some(asset.id.as_str()),
                Some(data.note.as_str()),
                data.source.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                let r = records
                    .iter()
                    .find(|r| r.id == id)
                    .ok_or_else(|| AppError::Invalid("Figure context missing.".into()))?;
                if !inputs.iter().any(|i| i.record == r.id) {
                    inputs.push(RevisionInput {
                        record: r.id.clone(),
                        version: r.version,
                        expectation: InputExpectation::Current,
                    });
                }
            }
        }
        let mut note = Note::working(title, markdown);
        note.protected = true;
        note.confirmed = true;
        let request = ChangeRequest {
            reason: "Owner reviewed and saved a chat reply as a decision in BRN".into(),
            writes: vec![
                Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(note),
                },
                Put {
                    id: op.creation_id(1),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Link(Link {
                        from: thread.id,
                        to: op.creation_id(0),
                        relation: "decision".into(),
                    }),
                },
            ],
            inputs,
        };
        self.store.prepare_owner(&op, &request, &owner)?;
        applied_record(self.store.apply(&op, &owner)?)
    }
}
