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

    /// UI hosts supply the displayed canonical context. Headless exact-owner
    /// callers retain review_apply; both end at the same checked transaction.
    pub fn review_apply_in_context(
        &mut self,
        id: &str,
        chat: Option<&str>,
    ) -> AppResult<ApplyOutcome> {
        let op = self.store.operation(id)?;
        let prepared = self.store.prepared(&op)?;
        if prepared
            .request
            .writes
            .iter()
            .any(|w| matches!(&w.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE))
        {
            self.writing_candidate_review(id, chat)?;
        }
        self.review_apply(id)
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
        if request
            .writes
            .iter()
            .any(|w| matches!(&w.data, RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE))
        {
            request.reason = WRITING_PUBLIC_REASON.into();
        }
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

/// Import fidelity, exact retained reads, and model claims remain separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WritingSourceReview {
    pub id: String,
    pub title: String,
    pub version: u64,
    pub current_version: u64,
    pub import_coverage: Option<ImportCoverage>,
    pub retained_read: Option<WritingRead>,
    pub private_into_wider_note: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WritingCandidateReview {
    pub sources: Vec<WritingSourceReview>,
    pub host_verified_quotations: Vec<WritingQuotation>,
    /// Explicitly model-reported claims; never proof of all requirements.
    pub requirements_reported: Vec<WritingRequirementsClaim>,
    pub complete_over_retained_text: bool,
    pub model_explanation: String,
}
impl Workspace {
    /// The caller supplies the displayed writing context, never a global fallback.
    pub fn writing_candidate_review(
        &self,
        id: &str,
        chat: Option<&str>,
    ) -> AppResult<WritingCandidateReview> {
        let op = self.store.operation(id)?;
        let request = self.store.prepared(&op)?.request;
        let evidence = request
            .writes
            .iter()
            .find_map(|w| match &w.data {
                RecordData::Message(m) if m.role == WRITING_EVIDENCE_ROLE => {
                    serde_json::from_str::<WritingCandidateEvidence>(&m.text).ok()
                }
                _ => None,
            })
            .ok_or_else(|| {
                AppError::Invalid("This candidate has no host-recorded writing evidence".into())
            })?;
        let snapshot = self.writing_request(&evidence.request_message)?;
        let scope = writing_setup_scope(&self.records()?, chat)?;
        if evidence.version != 1
            || evidence.thread != snapshot.thread
            || evidence.chat != scope
            || snapshot.setup.as_ref().is_none_or(|s| s.chat != scope)
        {
            return Err(AppError::Invalid(
                "Writing evidence is unavailable in this context".into(),
            ));
        }
        let mut sources = Vec::new();
        let setup = snapshot.setup.as_ref().expect("checked v2");
        let versions = setup
            .references
            .iter()
            .map(|r| (r.note.clone(), r.version))
            .chain(
                evidence
                    .reads
                    .iter()
                    .filter(|r| r.id != snapshot.note.id)
                    .map(|r| (r.id.clone(), r.version)),
            )
            .collect::<BTreeSet<_>>();
        let rows = self.records()?;
        let working_wider = self.store.record_visible_in_chat(&snapshot.note.id, None)?
            || rows
                .iter()
                .filter(|r| matches!(r.data, RecordData::Project(_)))
                .any(|p| {
                    p.id != scope.as_deref().unwrap_or("")
                        && self
                            .store
                            .record_visible_in_chat(&snapshot.note.id, Some(&p.id))
                            .unwrap_or(false)
                });
        for (id, version) in versions {
            let read = evidence
                .reads
                .iter()
                .find(|r| r.id == id && r.version == version)
                .cloned();
            let source_chat = if setup.references.iter().any(|r| r.note == id) {
                setup.chat.as_deref()
            } else {
                Some(evidence.thread.as_str())
            };
            if !self
                .store
                .revision_visible_in_chat(&id, version, source_chat)?
            {
                return Err(AppError::Invalid("A writing source is no longer available here; Refresh or restore access before reviewing".into()));
            }
            let current = self
                .store
                .record(&id)?
                .ok_or_else(|| AppError::Invalid("Writing source is unavailable".into()))?;
            if current.archived {
                return Err(AppError::Invalid(
                    "A writing source is archived; Refresh before reviewing".into(),
                ));
            }
            let old = self.store.revision(&id, version)?.ok_or_else(|| {
                AppError::Invalid("Writing source revision is unavailable".into())
            })?;
            let RecordData::Note(note) = old.data else {
                return Err(AppError::Invalid("Writing source is not a Note".into()));
            };
            let private =
                working_wider && !self.store.revision_visible_in_chat(&id, version, None)?;
            sources.push(WritingSourceReview {
                id,
                title: note.title,
                version,
                current_version: current.version,
                import_coverage: note.import,
                retained_read: read,
                private_into_wider_note: private,
            });
        }
        let complete_over_retained_text = setup.requirements.exhaustive
            && setup
                .references
                .iter()
                .any(|r| r.purpose == WritingPurpose::Requirements)
            && setup
                .references
                .iter()
                .filter(|r| r.purpose == WritingPurpose::Requirements)
                .all(|r| {
                    evidence.reads.iter().any(|read| {
                        read.id == r.note
                            && read.version == r.version
                            && read.complete
                            && read.covered_retained_bytes == read.total_retained_bytes
                    })
                });
        Ok(WritingCandidateReview {
            sources,
            host_verified_quotations: evidence.quotations,
            requirements_reported: evidence.requirements_reported,
            complete_over_retained_text,
            model_explanation: evidence.explanation,
        })
    }
}
