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

    pub fn full_import_replay_with_conversion(
        &mut self,
        key: &str,
        title: &str,
        source_reference: &str,
        source_bytes: &[u8],
        scope: Option<&str>,
        settings: &DocumentConversionSettings,
    ) -> AppResult<Option<ApplyOutcome>> {
        self.check_document_import(
            key,
            title,
            source_reference,
            &Sha256::digest(source_bytes).into(),
            scope,
            settings,
        )?;
        self.full_import_replay(key, title, source_reference, source_bytes, scope)
    }
    /// Read the stored request even when it has no receipt or creates no Note.
    /// An allocated operation without preparation has not captured a variant.
    pub(crate) fn check_document_import(
        &self,
        key: &str,
        title: &str,
        source_reference: &str,
        digest: &[u8; 32],
        scope: Option<&str>,
        settings: &DocumentConversionSettings,
    ) -> AppResult<()> {
        let Some(op) = self
            .store
            .operation_by_host_key(&format!("full-import:{key}"))?
        else {
            return Ok(());
        };
        let writes = if let Some(receipt) = self.store.receipt(&op)? {
            receipt
                .writes
                .into_iter()
                .map(|write| write.after.data)
                .collect::<Vec<_>>()
        } else {
            match self.store.prepared(&op) {
                Ok(prepared) => prepared
                    .request
                    .writes
                    .into_iter()
                    .map(|write| write.data)
                    .collect(),
                Err(Error::Invalid(ref reason)) if reason == "operation not prepared" => {
                    return Ok(());
                }
                Err(error) => return Err(error.into()),
            }
        };
        let expected = crate::intake::document_provenance(settings)?;
        let hash = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let mut sources = writes.iter().filter_map(|data| {
            if let RecordData::Source(source) = data {
                Some(source)
            } else {
                None
            }
        });
        let source = sources.next().ok_or(Error::ImmutableRequest)?;
        if sources.next().is_some()
            || !same_locator(&source.locator, &canonical_locator(source_reference))
            || source.chat.as_deref() != scope
            || source.label != title
            || source.external_version.as_deref() != Some(hash.as_str())
            || source
                .report
                .as_ref()
                .and_then(|report| report.conversion.as_ref())
                != Some(&expected)
        {
            return Err(Error::ImmutableRequest.into());
        }
        for data in &writes {
            if let RecordData::Note(note) = data
                && (note.title != title
                    || note
                        .import
                        .as_ref()
                        .and_then(|import| import.report.as_ref())
                        .and_then(|report| report.conversion.as_ref())
                        != Some(&expected))
            {
                return Err(Error::ImmutableRequest.into());
            }
        }
        Ok(())
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
        // Locators are canonical real paths; an older spelling of the file still matches.
        if !brn_threads_core::same_locator(
            &found.locator,
            &brn_threads_core::canonical_locator(source_reference),
        ) || found.chat.as_deref() != scope
        {
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
