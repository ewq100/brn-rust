//! Caller-keyed owner operations for hosts that retry (the CLI's `--key`).
//! Each function takes the unprefixed caller key and reuses the request,
//! prefix pattern and authority of its unkeyed twin, so no check is skipped.
use crate::*;
use sha2::{Digest, Sha256};

impl Workspace {
    /// `create_topics_note` under a key: a retry replays the note (no second
    /// Topics note) and chooses it again, so a crash between the two steps
    /// still leaves Topics set.
    pub fn create_topics_note_with_key(&mut self, key: &str) -> AppResult<(Record, ApplyOutcome)> {
        let receipt =
            self.new_note_with_key(&format!("topics:{key}"), "Topics", crate::topics::STARTER)?;
        let note = receipt
            .writes
            .into_iter()
            .next()
            .map(|write| write.after)
            .ok_or_else(|| AppError::Invalid("Empty receipt".into()))?;
        let outcome = self.choose_topics_note(&note.id)?;
        Ok((note, outcome))
    }

    /// The applied receipt's first written record under `host_key`, if any.
    fn applied_under(&self, host_key: &str) -> AppResult<Option<Receipt>> {
        match self.store.operation_by_host_key(host_key)? {
            Some(op) => Ok(self.store.receipt(&op)?),
            None => Ok(None),
        }
    }

    /// `seed_knowledge_review` under a key, receipt first: a retry reuses the
    /// chat and the message an earlier attempt committed, so the first
    /// findings win. Returns the chat and the message operation.
    pub fn seed_knowledge_review_with_key(
        &mut self,
        key: &str,
        check: &KnowledgeCheck,
    ) -> AppResult<(String, OperationId)> {
        let (title, text) = crate::knowledge::review_seed(check);
        let chat_key = format!("knowledge-review-chat:{key}");
        let chat = match self.applied_under(&format!("new-thread:{chat_key}"))? {
            Some(receipt) => receipt,
            None => self.new_thread_with_key(&chat_key, &title)?,
        };
        let chat = chat
            .writes
            .first()
            .map(|write| write.after.id.clone())
            .ok_or_else(|| AppError::Invalid("Empty receipt".into()))?;
        let message_key = format!("knowledge-review-message:{key}");
        if let Some(receipt) = self.applied_under(&message_key)? {
            return Ok((chat, receipt.operation));
        }
        let receipt = self.keyed_owner_message(&message_key, &chat, &text)?;
        Ok((chat, receipt.operation))
    }

    /// Exactly `post_message`'s request under a caller host key.
    fn keyed_owner_message(
        &mut self,
        host_key: &str,
        thread: &str,
        text: &str,
    ) -> AppResult<Receipt> {
        let op = self.store.allocate_operation(host_key)?;
        let request = ChangeRequest {
            reason: "Owner message".into(),
            writes: vec![Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Message(Message {
                    thread: thread.into(),
                    role: "owner".into(),
                    text: text.into(),
                }),
            }],
            inputs: vec![],
        };
        self.store
            .prepare_owner(&op, &request, &HostAuthority::owner("owner"))?;
        match self.store.apply(&op, &HostAuthority::owner("owner"))? {
            ApplyOutcome::Applied(receipt) => Ok(receipt),
            outcome => Err(AppError::Invalid(format!(
                "Owner message was not applied: {outcome:?}"
            ))),
        }
    }

    /// Before converting a keyed full, chat or project intake again: when the
    /// key's committed import matches these bytes, path, scope and title,
    /// replay its receipt without converting or sending the document.
    /// `None` when there is nothing to replay (no operation, no receipt, or a
    /// receipt that only observed previously imported bytes); then the normal
    /// path converts and runs every check in today's order.
    pub fn full_import_replay(
        &mut self,
        key: &str,
        title: &str,
        source_reference: &str,
        source_bytes: &[u8],
        scope: Option<&str>,
    ) -> AppResult<Option<ApplyOutcome>> {
        let Some(op) = self
            .store
            .operation_by_host_key(&format!("full-import:{key}"))?
        else {
            return Ok(None);
        };
        let Some(receipt) = self.store.receipt(&op)? else {
            return Ok(None);
        };
        let sources: Vec<&Record> = receipt
            .writes
            .iter()
            .map(|w| &w.after)
            .filter(|r| matches!(r.data, RecordData::Source(_)))
            .collect();
        let [source] = sources.as_slice() else {
            return Err(Error::ImmutableRequest.into());
        };
        let RecordData::Source(found) = &source.data else {
            unreachable!()
        };
        if !same_source(&found.locator, source_reference) || found.chat.as_deref() != scope {
            return Err(Error::ImmutableRequest.into());
        }
        let digest: String = Sha256::digest(source_bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let relation = format!("{}{digest}", crate::relations::FULL_IMPORT_PREFIX);
        let link = receipt.writes.iter().find_map(|w| match &w.after.data {
            RecordData::Link(link) if crate::relations::is_host_owned(&link.relation) => Some(link),
            _ => None,
        });
        let note = receipt.writes.iter().find_map(|w| match &w.after.data {
            RecordData::Note(note)
                if note
                    .import
                    .as_ref()
                    .is_some_and(|i| i.full_note && i.source == source.id) =>
            {
                Some(note)
            }
            _ => None,
        });
        let (Some(link), Some(note)) = (link, note) else {
            // The bytes were already imported for this Source: convert again.
            return Ok(None);
        };
        if link.relation != relation || link.to != source.id || note.title != title {
            return Err(Error::ImmutableRequest.into());
        }
        Ok(Some(self.store.apply(
            &op,
            &HostAuthority::owner("full-note import request"),
        )?))
    }
}

/// Whether a stored Source locator names the same file as `reference`. This
/// branch stores the raw `--source`, so the match is exact. When it merges
/// with the daily-workflow branch (which stores canonical paths: "count one
/// file as one source regardless of path spelling", `344331f2` at review),
/// this becomes `same_locator(stored, &canonical_locator(reference))`.
fn same_source(stored: &str, reference: &str) -> bool {
    stored == reference
}
