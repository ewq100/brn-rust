use crate::*;
impl Workspace {
    /// Dismiss only this unapplied proposal. Notes and unrelated questions stay intact.
    /// Retiring is durable before cleanup, so retry also repairs a lost cleanup response.
    pub fn review_dismiss(&mut self, id: &str) -> AppResult<()> {
        let operation = self.store.operation(id)?;
        self.store.prepared(&operation)?;
        self.store.retire(&operation)?;
        self.finish_review_attention(&operation)
    }

    pub fn review_apply(&mut self, id: &str) -> AppResult<ApplyOutcome> {
        let op = self.store.operation(id)?;
        let outcome = self
            .store
            .apply(&op, &HostAuthority::owner("reviewed immutable candidate"))?;
        if matches!(outcome, ApplyOutcome::Applied(_)) {
            self.finish_review_attention(&op)?;
        }
        Ok(outcome)
    }
    fn finish_review_attention(&mut self, review: &OperationId) -> AppResult<()> {
        for _ in 0..3 {
            let writes = self
                .records()?
                .into_iter()
                .filter_map(|record| {
                    let RecordData::Thread(mut thread) = record.data else {
                        return None;
                    };
                    let before = thread.attention.len();
                    thread
                        .attention
                        .retain(|attention| attention.record.as_deref() != Some(review.as_str()));
                    if thread.attention.len() == before {
                        return None;
                    }
                    Some(Put {
                        id: record.id,
                        expected_version: Some(record.version),
                        archived: record.archived,
                        data: RecordData::Thread(thread),
                    })
                })
                .collect::<Vec<_>>();
            if writes.is_empty() {
                return Ok(());
            }
            let request = ChangeRequest {
                reason: "Accepted exact review; preserve other attention requests".into(),
                writes,
                inputs: vec![],
            };
            let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&request)?));
            let key = format!("review-attention-cleanup:{}:{digest}", review.as_str());
            match self.owner_change(&key, &request)? {
                ApplyOutcome::Applied(_) => return Ok(()),
                ApplyOutcome::Stale { .. } => continue,
                other => {
                    return Err(AppError::Invalid(format!(
                        "Review applied; attention cleanup needs a retry: {other:?}"
                    )));
                }
            }
        }
        Err(AppError::Invalid(
            "Review applied; concurrent attention changes need a refresh and retry".into(),
        ))
    }

    pub fn review_replace(&mut self, id: &str, note: &str, markdown: &str) -> AppResult<Prepared> {
        let original = self.store.operation(id)?;
        let mut request = self.store.prepared(&original)?.request;
        let write = request
            .writes
            .iter_mut()
            .find(|write| write.id == note)
            .ok_or_else(|| AppError::Invalid("Note not in candidate".into()))?;
        let RecordData::Note(data) = &mut write.data else {
            return Err(AppError::Invalid("Not a note candidate".into()));
        };
        data.markdown = markdown.into();
        let replacement = self
            .store
            .allocate_operation(&format!("review-replacement:{}", uuid::Uuid::new_v4()))?;
        let map = request
            .writes
            .iter()
            .filter(|write| write.expected_version.is_none())
            .enumerate()
            .map(|(index, write)| (write.id.clone(), replacement.creation_id(index)))
            .collect::<BTreeMap<_, _>>();
        let known_assets = self
            .store
            .records()?
            .into_iter()
            .filter(|record| !record.archived && matches!(record.data, RecordData::Asset(_)))
            .map(|record| record.id)
            .chain(
                request
                    .writes
                    .iter()
                    .filter(|write| !write.archived && matches!(write.data, RecordData::Asset(_)))
                    .map(|write| write.id.clone()),
            )
            .collect::<std::collections::BTreeSet<_>>();
        for write in &mut request.writes {
            remap(&mut write.id, &map);
            match &mut write.data {
                RecordData::Note(note) => {
                    if let Some(import) = &mut note.import {
                        remap(&mut import.source, &map);
                    }
                    if let Some(superseded) = &mut note.superseded_by {
                        remap(superseded, &map);
                    }
                    note.markdown = crate::assets::rewrite(&note.markdown, |id| {
                        known_assets.contains(id).then(|| {
                            format!("brn-asset://{}", map.get(id).map_or(id, String::as_str))
                        })
                    })?;
                }
                RecordData::Asset(asset) => {
                    remap(&mut asset.note, &map);
                    if let Some(source) = &mut asset.source {
                        remap(source, &map);
                    }
                }
                RecordData::Link(link) => {
                    remap(&mut link.from, &map);
                    remap(&mut link.to, &map);
                }
                RecordData::Comment(comment) => remap(&mut comment.note, &map),
                RecordData::Message(message) => remap(&mut message.thread, &map),
                RecordData::Thread(thread) => {
                    for attention in &mut thread.attention {
                        if let Some(record) = &mut attention.record {
                            if record == original.as_str() {
                                *record = replacement.as_str().into();
                            }
                            remap(record, &map);
                        }
                    }
                }
                RecordData::Run(run) => {
                    remap(&mut run.thread, &map);
                    for source in &mut run.sources {
                        remap(source, &map);
                    }
                }
                _ => {}
            }
        }
        for input in &mut request.inputs {
            remap(&mut input.record, &map);
        }
        Ok(self.store.replace_candidate(
            &original,
            &replacement,
            &request,
            &HostAuthority::owner("owner edited displayed candidate"),
        )?)
    }
}
fn remap(value: &mut String, map: &BTreeMap<String, String>) {
    if let Some(replacement) = map.get(value) {
        *value = replacement.clone();
    }
}
