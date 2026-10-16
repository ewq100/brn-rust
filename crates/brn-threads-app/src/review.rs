use crate::*;
#[cfg(test)]
thread_local! {
    static REQUEST_VERIFICATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
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
/// One authoritative display inspection; never an Apply/read grant.
#[derive(Debug, Clone, Default)]
pub struct WritingContextReview {
    pub candidates: BTreeMap<String, WritingCandidateReview>,
    pub requests: BTreeMap<String, WritingSnapshot>,
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
        let rows = self.records()?;
        let index = rows
            .iter()
            .map(|r| (r.id.as_str(), r))
            .collect::<BTreeMap<_, _>>();
        self.writing_candidate_review_from(&request, chat, &rows, &index, None, None)
    }
    fn writing_candidate_review_from(
        &self,
        request: &ChangeRequest,
        chat: Option<&str>,
        rows: &[Record],
        index: &BTreeMap<&str, &Record>,
        displayed_note: Option<&str>,
        wider: Option<bool>,
    ) -> AppResult<WritingCandidateReview> {
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
        let marker = request
            .inputs
            .iter()
            .find(|i| {
                i.record == evidence.request_message && i.expectation == InputExpectation::Current
            })
            .ok_or_else(|| AppError::Invalid("Writing request binding is unavailable".into()))?;
        let saved = self
            .store
            .revision(&marker.record, marker.version)?
            .ok_or_else(|| AppError::Invalid("Writing request revision is unavailable".into()))?;
        let snapshot = self.writing_request_from_record(&saved)?;
        let scope = writing_setup_scope(rows, chat)?;
        if displayed_note.is_some_and(|note| snapshot.note.id != note)
            || evidence.version != 1
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
        let working_wider =
            wider.unwrap_or_else(|| working_note_wider(rows, &snapshot.note.id, scope.as_deref()));
        for (id, version) in versions {
            let read = evidence
                .reads
                .iter()
                .find(|r| r.id == id && r.version == version)
                .cloned();
            let source_chat = setup.chat.as_deref();
            let current = index
                .get(id.as_str())
                .copied()
                .cloned()
                .ok_or_else(|| AppError::Invalid("Writing source is unavailable".into()))?;
            if current.archived {
                return Err(AppError::Invalid(
                    "A writing source is archived; Refresh before reviewing".into(),
                ));
            }
            let old = self.store.revision(&id, version)?.ok_or_else(|| {
                AppError::Invalid("Writing source revision is unavailable".into())
            })?;
            if !record_in_chat(&current, rows, source_chat)
                || !record_in_chat(&old, rows, source_chat)
            {
                return Err(AppError::Invalid("A writing source is no longer available here; Refresh or restore access before reviewing".into()));
            }
            let private = working_wider
                && !(record_in_chat(&current, rows, None) && record_in_chat(&old, rows, None));
            let RecordData::Note(note) = old.data else {
                return Err(AppError::Invalid("Writing source is not a Note".into()));
            };
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
    /// Load canonical records and receipt context once for a displayed Note.
    /// The returned token is the one observed before *and* after the inspection.
    pub fn writing_context_review(
        &self,
        note: &str,
        displayed: Option<&str>,
        shown: &[Prepared],
    ) -> AppResult<(ViewChangeToken, WritingContextReview)> {
        let token = self.store.change_token()?;
        let rows = self.records()?;
        let receipts = self.store.receipts()?;
        let context = self.active_writing_context_from(note, displayed, &rows, &receipts)?;
        if context.scope != writing_setup_scope(&rows, displayed)? {
            return Err(AppError::Invalid(
                "Writing context is no longer displayed here".into(),
            ));
        }
        let wider = working_note_wider(&rows, note, context.scope.as_deref());
        let mut checked = WritingContextReview::default();
        let index = rows
            .iter()
            .map(|r| (r.id.as_str(), r))
            .collect::<BTreeMap<_, _>>();
        let active = self
            .store
            .candidates()?
            .into_iter()
            .map(|c| (c.operation.as_str().to_owned(), c))
            .collect::<BTreeMap<_, _>>();
        let mut invalid = BTreeSet::new();
        for candidate in shown {
            let id = candidate.operation.as_str();
            if active.get(id) != Some(candidate) {
                checked.candidates.remove(id);
                invalid.insert(id);
                continue;
            }
            if invalid.contains(id) {
                continue;
            }
            if let Ok(review) = self.writing_candidate_review_from(
                &candidate.request,
                context.scope.as_deref(),
                &rows,
                &index,
                Some(note),
                Some(wider),
            ) {
                checked
                    .candidates
                    .insert(candidate.operation.as_str().to_owned(), review);
            }
        }
        #[derive(serde::Deserialize)]
        struct EnvelopeRoute {
            snapshot: RequestRoute,
        }
        #[derive(serde::Deserialize)]
        struct RequestRoute {
            note: NoteRoute,
        }
        #[derive(serde::Deserialize)]
        struct NoteRoute {
            id: String,
        }
        for record in &rows {
            let RecordData::Message(message) = &record.data else {
                continue;
            };
            if record.archived
                || record.version != 1
                || message.role != WRITING_REQUEST_ROLE
                || message.text.len() > MAX_WRITING_REQUEST_BYTES
            {
                continue;
            }
            // An untrusted routing hint can only exclude unrelated work before
            // expensive canonical verification. Every retained request still
            // uses the same checked predicate, including its conversation.
            let Ok(route) = serde_json::from_str::<EnvelopeRoute>(&message.text) else {
                continue;
            };
            if route.snapshot.note.id != note {
                continue;
            }
            #[cfg(test)]
            REQUEST_VERIFICATIONS.with(|count| count.set(count.get() + 1));
            if let Ok(snapshot) = self.writing_request_in_context_from(record, &context, &rows) {
                checked.requests.insert(record.id.clone(), snapshot);
            }
        }
        if self.store.change_token()? != token {
            return Err(AppError::Unavailable(
                "Writing context changed during inspection; review the current context".into(),
            ));
        }
        Ok((token, checked))
    }
}

fn working_note_wider(rows: &[Record], note: &str, scope: Option<&str>) -> bool {
    let Some(record) = rows.iter().find(|r| r.id == note) else {
        return false;
    };
    record_in_chat(record, rows, None)
        || rows
            .iter()
            .filter(|r| matches!(r.data, RecordData::Project(_)))
            .any(|p| Some(p.id.as_str()) != scope && record_in_chat(record, rows, Some(&p.id)))
}

#[cfg(test)]
mod batch_work_tests {
    use super::*;
    #[test]
    fn unrelated_request_routing_does_no_canonical_verification_and_hint_grants_nothing() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let mut app =
            Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
        let thread = app.new_thread("Synthetic writing").unwrap();
        let mut target = None;
        for index in 0..31 {
            let note = app
                .new_note(&format!("Draft {index}"), "Canonical saved body")
                .unwrap();
            app.link_writing_thread(&format!("link-{index}"), &note.id, note.version, &thread.id)
                .unwrap();
            let snapshot = app
                .snapshot_writing_request(
                    &format!("request-{index}"),
                    &note.id,
                    note.version,
                    &thread.id,
                    &[],
                )
                .unwrap();
            if index == 0 {
                target = Some(snapshot);
            }
        }
        let target = target.unwrap();
        let operation = app.store.allocate_operation("forged-routing-hint").unwrap();
        let mut forged = target.clone();
        forged.request_message = operation.creation_id(0);
        forged.note.data = RecordData::Note(Note::working(
            "Untrusted hint",
            "UNTRUSTED PRIVATE SENTINEL",
        ));
        app.owner_change(
            "forged-routing-hint",
            &ChangeRequest {
                reason: "Synthetic invalid captured payload".into(),
                writes: vec![Put {
                    id: forged.request_message.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: thread.id.clone(),
                        role: WRITING_REQUEST_ROLE.into(),
                        text: forged.message_text().unwrap(),
                    }),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
        REQUEST_VERIFICATIONS.with(|count| count.set(0));
        let (_, checked) = app
            .writing_context_review(&target.note.id, None, &[])
            .unwrap();
        assert_eq!(checked.requests.get(&target.request_message), Some(&target));
        assert!(
            !checked.requests.contains_key(&forged.request_message),
            "matching an untrusted routing hint never authorizes a noncanonical snapshot"
        );
        assert_eq!(checked.requests.len(), 1);
        REQUEST_VERIFICATIONS.with(|count| assert_eq!(count.get(), 2, "only the displayed Note's legitimate and forged routing matches reach canonical verification"));
    }
}
