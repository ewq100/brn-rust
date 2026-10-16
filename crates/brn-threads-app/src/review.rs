use crate::*;
impl Workspace {
    /// Dismiss only this unapplied proposal. Notes and unrelated questions stay intact.
    /// Retiring is durable before cleanup, so retry also repairs a lost cleanup response.
    pub fn review_dismiss(&mut self, id: &str) -> AppResult<()> {
        let operation = self.store.operation(id)?;
        let outcome = self.store.dismiss_candidate(
            &operation,
            REVIEW_CLEANUP_KEY,
            "Dismissed proposal; preserve other attention requests",
            &HostAuthority::owner("owner interaction"),
        )?;
        review_cleanup(outcome)
    }

    pub fn review_apply(&mut self, id: &str) -> AppResult<ApplyOutcome> {
        let op = self.store.operation(id)?;
        let outcome = self
            .store
            .apply(&op, &HostAuthority::owner("reviewed immutable candidate"))?;
        if matches!(outcome, ApplyOutcome::Applied(_)) {
            let cleanup = self.store.clear_review_attention(
                &op,
                REVIEW_CLEANUP_KEY,
                "Accepted exact review; preserve other attention requests",
                &HostAuthority::owner("owner interaction"),
            )?;
            review_cleanup(cleanup)?;
        }
        Ok(outcome)
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
        self.replace_review_request(original, request)
    }

    pub(crate) fn replace_review_request(
        &mut self,
        original: OperationId,
        mut request: ChangeRequest,
    ) -> AppResult<Prepared> {
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
/// Key prefix of the owner's review attention cleanup changes.
const REVIEW_CLEANUP_KEY: &str = "review-attention-cleanup";
fn review_cleanup(outcome: run_attention::CleanupOutcome) -> AppResult<()> {
    match outcome {
        run_attention::CleanupOutcome::Done(_) => Ok(()),
        run_attention::CleanupOutcome::Unexpected(other) => Err(AppError::Invalid(format!(
            "Review applied; attention cleanup needs a retry: {other:?}"
        ))),
        run_attention::CleanupOutcome::Contended => Err(AppError::Invalid(
            "Review applied; concurrent attention changes need a refresh and retry".into(),
        )),
    }
}
fn remap(value: &mut String, map: &BTreeMap<String, String>) {
    if let Some(replacement) = map.get(value) {
        *value = replacement.clone();
    }
}
