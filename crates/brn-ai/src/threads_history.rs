//! Bounded accepted history and fresh immutable proposals through checked core APIs.
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HistoryArgs {
    pub id: String,
    pub version: Option<u64>,
    pub start: usize,
    pub max_bytes: usize,
}
pub(super) fn validate_history(args: &Value) -> bool {
    args.get("version").is_some()
        && serde_json::from_value::<HistoryArgs>(args.clone()).is_ok_and(|args| {
            !args.id.is_empty()
                && args.id.len() <= 256
                && args.version.is_none_or(|version| version > 0)
                && (1..=MAX_NOTE_READ_BYTES).contains(&args.max_bytes)
                && u64::try_from(args.start).is_ok()
        })
}
pub(super) fn history_schema() -> Value {
    object(
        json!({"id":{"type":"string"},"version":{"type":["integer","null"],"description":"null lists accepted revision summaries; an exact version reads historical note text"},"start":{"type":"integer","minimum":0,"description":"For a summary page, list versions after this value; for an exact revision, UTF-8 byte offset"},"max_bytes":{"type":"integer","minimum":1,"maximum":131072}}),
        json!(["id", "version", "start", "max_bytes"]),
    )
}
pub(super) fn read_history(scope: &Scope, args: HistoryArgs) -> AiResult<Value> {
    let store = scope.store.lock().map_err(storage)?;
    live(scope, &store)?;
    if args.id.is_empty()
        || args.id.len() > 256
        || !(1..=MAX_NOTE_READ_BYTES).contains(&args.max_bytes)
    {
        return Err(rejected());
    }
    if !store
        .record_visible_in_chat(&args.id, Some(&scope.chat))
        .map_err(storage)?
    {
        return Err(rejected());
    }
    match args.version {
        Some(version) => {
            if !store
                .revision_visible_in_chat(&args.id, version, Some(&scope.chat))
                .map_err(storage)?
            {
                return Err(rejected());
            }
            let note = store
                .read_note_revision(&args.id, version, args.start, args.max_bytes)
                .map_err(storage)?;
            let mut out = json!({"outcome":"HistoricalRevision","note":note,"revision_input":RevisionInput { record: args.id, version, expectation: InputExpectation::Historical }});
            let prefix = context_budget::fit_prefix(&note.markdown, |prefix| {
                let end = note.start + prefix.len();
                out["note"]["markdown"] = json!(prefix);
                out["note"]["end"] = json!(end);
                out["note"]["next_start"] = json!((end < note.total_bytes).then_some(end));
                out["note"]["complete"] = json!(note.start == 0 && end == note.total_bytes);
                context_budget::result_fits(&out)
            })
            .ok_or_else(context_budget::result_failure)?;
            let end = note.start + prefix.len();
            out["note"]["markdown"] = json!(prefix);
            out["note"]["end"] = json!(end);
            out["note"]["next_start"] = json!((end < note.total_bytes).then_some(end));
            out["note"]["complete"] = json!(note.start == 0 && end == note.total_bytes);
            if !context_budget::result_fits(&out) {
                return Err(context_budget::result_failure());
            }
            // Historical reads deliberately grant no current-note ledger coverage.
            Ok(out)
        }
        None => {
            let mut page = store
                .note_revisions(&args.id, u64::try_from(args.start).map_err(storage)?)
                .map_err(storage)?;
            let mut allowed = Vec::new();
            for summary in page.revisions {
                if store
                    .revision_visible_in_chat(&args.id, summary.version, Some(&scope.chat))
                    .map_err(storage)?
                {
                    allowed.push(summary);
                }
            }
            page.revisions = allowed;
            serde_json::to_value(page).map_err(storage)
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RefreshArgs {
    pub operation: String,
    pub request_key: String,
    pub request: ChangeRequest,
}
pub(super) fn validate_refresh(args: &Value) -> bool {
    validate_refresh_arguments(args).is_ok()
}
pub(super) fn validate_refresh_arguments(args: &Value) -> std::result::Result<(), ArgumentError> {
    if args.get("writing").is_some() {
        // This tool offers no writing field, even null. It cannot introduce
        // writing evidence through a corrected or malformed envelope.
        return Err(ArgumentError::Forbidden);
    }
    let parsed = serde_json::from_value::<RefreshArgs>(args.clone()).map_err(|_| change_shape_error(args,
        "Supply only operation, request_key and the typed request with reason/writes/inputs; writing is not accepted.",
    ))?;
    // Preserve the same policy boundary as prepare, before shape feedback.
    validate_change_arguments(&parsed.request_key, &parsed.request, &args["request"])?;
    argument_shape(
        !parsed.operation.is_empty() && parsed.operation.len() <= 256,
        "operation must contain 1..256 UTF-8 bytes.",
    )
}
pub(super) fn refresh_schema() -> Value {
    let mut definition = schema("prepare_changes");
    if let Some(properties) = definition["properties"].as_object_mut() {
        properties.remove("writing");
    }
    definition["properties"]["operation"] = json!({"type":"string","description":"Existing unapplied candidate from this Thread's run family"});
    definition["required"] = json!(["operation", "request_key", "request"]);
    definition
}
pub(super) fn refresh(scope: &Scope, mut args: RefreshArgs) -> AiResult<Value> {
    if scope.writing_snapshot.is_some() {
        return Err(rejected());
    }
    if !validate_refresh(
        &json!({"operation":args.operation,"request_key":args.request_key,"request":args.request}),
    ) {
        return Err(rejected());
    }
    let mut store = scope.store.lock().map_err(storage)?;
    live(scope, &store)?;
    let original = store.operation(&args.operation).map_err(storage)?;
    let prepared = store.prepared(&original).map_err(storage)?;
    for input in &prepared.request.inputs {
        if matches!(store.revision(&input.record, input.version).map_err(storage)?,
            Some(Record { data: RecordData::Message(message), .. }) if message.role == WRITING_REQUEST_ROLE)
        {
            return Err(rejected());
        }
    }
    let thread = thread_for(scope, &store)?;
    if !store.records().map_err(storage)?.iter().any(|record| !record.archived && matches!(&record.data, RecordData::Run(run) if run.thread == thread && run.operations.contains(&original))) {
        return Err(rejected());
    }
    let replacement = store
        .allocate_operation(&format!("{}:refresh:{}", scope.run, args.request_key))
        .map_err(storage)?;
    update_run(scope, &mut store, |run| {
        if !run.operations.contains(&replacement) {
            run.operations.push(replacement.clone());
        }
    })?;
    remap(&mut args.request, &replacement)?;
    check_scope(scope, &store, &prepared.request)?;
    scope_request(scope, &store, &mut args.request)?;
    sources::materialize_selected_sources(
        &scope.sources,
        &store,
        &replacement,
        &mut args.request,
        Some(&scope.chat),
        &scope.source_ids,
    )?;
    bind_project_inputs(scope, &store, &mut args.request)?;
    if let Some(reason) = unread_replacement(scope, &store, &args.request)? {
        return Ok(json!({"outcome":"Invalid","reason":reason}));
    }
    let authority = bind(
        scope,
        HostAuthority::runtime("Threads candidate refresh", &scope.run),
    );
    let prepared = store
        .refresh_candidate(&original, &replacement, &args.request, &authority)
        .map_err(storage)?;
    prepared_output(&prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};

    fn request(note: &str, version: u64, text: &str, protected: bool) -> ChangeRequest {
        let mut data = Note::working("Decision", text);
        data.protected = protected;
        ChangeRequest {
            reason: "Fresh checked evidence".into(),
            writes: vec![Put {
                id: note.into(),
                expected_version: Some(version),
                archived: false,
                data: RecordData::Note(data),
            }],
            inputs: vec![],
        }
    }
    fn owner_edit(store: &mut Store, note: &str, version: u64, text: &str, protected: bool) {
        let operation = store
            .allocate_operation(&format!("owner-{version}"))
            .unwrap();
        let authority = HostAuthority::owner("test");
        store
            .prepare_owner(
                &operation,
                &request(note, version, text, protected),
                &authority,
            )
            .unwrap();
        assert!(matches!(
            store.apply(&operation, &authority).unwrap(),
            ApplyOutcome::Applied(_)
        ));
    }

    #[test]
    fn refresh_is_atomic_version_checked_idempotent_and_does_not_grant_protected_apply() {
        let (_root, run_request, note) = super::super::tests::fixture();
        let mut store = Store::open(&run_request.data_dir).unwrap();
        owner_edit(&mut store, &note, 1, "Protected original", true);
        drop(store);
        let (scope, _) = begin(&run_request).unwrap();
        super::super::tests::read_all(&scope);
        let original = prepare(
            &scope,
            PrepareArgs {
                writing: None,
                request_key: "old".into(),
                request: request(&note, 2, "Old candidate", true),
            },
        )
        .unwrap();
        let old = original["operation"].as_str().unwrap();
        owner_edit(
            &mut scope.store.lock().unwrap(),
            &note,
            2,
            "Owner changed current",
            true,
        );
        let stale = refresh(
            &scope,
            RefreshArgs {
                operation: old.into(),
                request_key: "new".into(),
                request: request(&note, 2, "Stale replacement", true),
            },
        );
        assert!(stale.is_err());
        {
            let store = scope.store.lock().unwrap();
            assert!(
                store
                    .candidates()
                    .unwrap()
                    .iter()
                    .any(|candidate| candidate.operation.as_str() == old)
            );
            assert!(
                matches!(store.record(&note).unwrap().unwrap().data, RecordData::Note(data) if data.markdown == "Owner changed current")
            );
        }
        super::super::tests::read_all(&scope);
        let candidate = refresh(
            &scope,
            RefreshArgs {
                operation: old.into(),
                request_key: "new".into(),
                request: request(&note, 3, "Fresh replacement", true),
            },
        )
        .unwrap();
        let retry = refresh(
            &scope,
            RefreshArgs {
                operation: old.into(),
                request_key: "new".into(),
                request: request(&note, 3, "Fresh replacement", true),
            },
        )
        .unwrap();
        assert_eq!(candidate, retry);
        assert!(
            refresh(
                &scope,
                RefreshArgs {
                    operation: old.into(),
                    request_key: "new".into(),
                    request: request(&note, 3, "Changed retry", true)
                }
            )
            .is_err()
        );
        let result = apply(
            &scope,
            ApplyArgs {
                operation: candidate["operation"].as_str().unwrap().into(),
            },
        )
        .unwrap();
        assert_eq!(result["outcome"], "NeedsReview");
        assert!(
            matches!(scope.store.lock().unwrap().record(&note).unwrap().unwrap().data, RecordData::Note(data) if data.markdown == "Owner changed current")
        );
        assert!(
            !scope
                .store
                .lock()
                .unwrap()
                .candidates()
                .unwrap()
                .iter()
                .any(|candidate| candidate.operation.as_str() == old)
        );
    }

    #[test]
    fn refresh_supports_previous_run_family_but_refuses_other_threads_and_superseded_scope() {
        let (_root, run_request, note) = super::super::tests::fixture();
        let (old_scope, _) = begin(&run_request).unwrap();
        super::super::tests::read_all(&old_scope);
        let original = prepare(
            &old_scope,
            PrepareArgs {
                writing: None,
                request_key: "first".into(),
                request: request(&note, 1, "Initial proposal", false),
            },
        )
        .unwrap();
        let old = original["operation"].as_str().unwrap();
        let (scope, _) = begin(&run_request).unwrap();
        assert!(
            refresh(
                &old_scope,
                RefreshArgs {
                    operation: old.into(),
                    request_key: "old-scope".into(),
                    request: request(&note, 1, "Superseded", false)
                }
            )
            .is_err()
        );
        super::super::tests::read_all(&scope);
        let candidate = refresh(
            &scope,
            RefreshArgs {
                operation: old.into(),
                request_key: "same-family".into(),
                request: request(&note, 1, "New family proposal", false),
            },
        )
        .unwrap();
        let result = apply(
            &scope,
            ApplyArgs {
                operation: candidate["operation"].as_str().unwrap().into(),
            },
        )
        .unwrap();
        assert_eq!(result["outcome"], "Applied");
        // Identical retry remains the same preparation after the current version changes.
        assert_eq!(
            candidate,
            refresh(
                &scope,
                RefreshArgs {
                    operation: old.into(),
                    request_key: "same-family".into(),
                    request: request(&note, 1, "New family proposal", false)
                }
            )
            .unwrap()
        );
        let mut store = scope.store.lock().unwrap();
        let operation = store.allocate_operation("other-thread").unwrap();
        let other = operation.creation_id(0);
        let authority = HostAuthority::owner("fixture");
        store
            .prepare_owner(
                &operation,
                &ChangeRequest {
                    reason: "Other thread".into(),
                    writes: vec![Put {
                        id: other.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Thread(Thread {
                            private_context: false,
                            context_epoch: 0,
                            title: "Other".into(),
                            state: ThreadState::Open,
                            attention: vec![],
                        }),
                    }],
                    inputs: vec![],
                },
                &authority,
            )
            .unwrap();
        store.apply(&operation, &authority).unwrap();
        drop(store);
        let mut other_request = run_request;
        other_request.thread = other;
        let (other_scope, _) = begin(&other_request).unwrap();
        assert!(
            refresh(
                &other_scope,
                RefreshArgs {
                    operation: old.into(),
                    request_key: "cross-thread".into(),
                    request: request(&note, 2, "Cross thread", false)
                }
            )
            .is_err()
        );
    }

    #[test]
    fn historical_read_is_explicit_bounded_and_distinct_from_current() {
        let (_root, run_request, note) = super::super::tests::fixture();
        let (scope, _) = begin(&run_request).unwrap();
        owner_edit(
            &mut scope.store.lock().unwrap(),
            &note,
            1,
            "Current Tuesday",
            false,
        );
        let page = read_history(
            &scope,
            HistoryArgs {
                id: note.clone(),
                version: None,
                start: 0,
                max_bytes: 4096,
            },
        )
        .unwrap();
        assert_eq!(page["revisions"].as_array().unwrap().len(), 2);
        assert!(page.to_string().find("Current Tuesday").is_none());
        let history = read_history(
            &scope,
            HistoryArgs {
                id: note.clone(),
                version: Some(1),
                start: 0,
                max_bytes: 4096,
            },
        )
        .unwrap();
        assert_eq!(history["outcome"], "HistoricalRevision");
        assert_eq!(history["note"]["markdown"], "Monday");
        assert!(!validate_history(
            &json!({"id":note,"start":0,"max_bytes":4096})
        ));
        assert!(!validate_history(
            &json!({"id":note,"version":0,"start":0,"max_bytes":4096})
        ));
        assert!(!validate_history(
            &json!({"id":note,"version":1,"start":0,"max_bytes":MAX_NOTE_READ_BYTES+1})
        ));
    }

    #[tokio::test]
    async fn registered_runtime_history_and_refresh_tools_apply_fresh_candidate_and_store_one_answer()
     {
        let (_root, run_request, note) = super::super::tests::fixture();
        Store::open(&run_request.data_dir)
            .unwrap()
            .set_workspace_purpose(WorkspacePurpose::Test)
            .unwrap();
        let (scope, input) = begin(&run_request).unwrap();
        super::super::tests::read_all(&scope);
        let original = prepare(
            &scope,
            PrepareArgs {
                writing: None,
                request_key: "initial".into(),
                request: request(&note, 1, "Old draft", false),
            },
        )
        .unwrap();
        owner_edit(
            &mut scope.store.lock().unwrap(),
            &note,
            1,
            "Current Tuesday",
            false,
        );
        let replacement = scope
            .store
            .lock()
            .unwrap()
            .allocate_operation(&format!("{}:refresh:fresh", scope.run))
            .unwrap();
        let model = MockCompletionModel::from_stream_turns([
            vec![
                MockStreamEvent::tool_call(
                    "history",
                    "read_history",
                    json!({"id":note,"version":1,"start":0,"max_bytes":4096}),
                ),
                MockStreamEvent::tool_call(
                    "current",
                    "read_records",
                    json!({"ids":[note],"query":null}),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![
                MockStreamEvent::tool_call(
                    "refresh",
                    "refresh_candidate",
                    json!({"operation":original["operation"],"request_key":"fresh","request":request(&note,2,"Accepted fresh Thursday",false)}),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![
                MockStreamEvent::tool_call(
                    "apply",
                    "apply_changes",
                    json!({"operation":replacement.as_str()}),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![
                MockStreamEvent::text("The fresh candidate was saved."),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
        ]);
        let answer = run_model(
            model.clone(),
            scope.clone(),
            &input,
            ReasoningEffort::Medium,
            4,
            true,
            CancellationToken::new(),
            Arc::new(|_| {}),
        )
        .await;
        let requests = model.requests();
        let definition = requests[0]
            .tools
            .iter()
            .find(|t| t.name == "refresh_candidate")
            .unwrap();
        assert_eq!(
            definition.parameters["properties"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            ["operation", "request_key", "request"]
                .into_iter()
                .collect(),
            "actual registered refresh definition must not advertise writing"
        );
        assert_eq!(
            definition.parameters["required"],
            json!(["operation", "request_key", "request"])
        );
        assert!(matches!(answer.terminal, AiTerminal::Completed));
        assert!(finish(&scope, &answer).unwrap());
        let store = scope.store.lock().unwrap();
        assert!(
            matches!(store.record(&note).unwrap().unwrap().data, RecordData::Note(note) if note.markdown == "Accepted fresh Thursday")
        );
        let messages = store.records().unwrap().into_iter().filter(|record| matches!(&record.data,RecordData::Message(message) if message.role == "assistant")).collect::<Vec<_>>();
        assert_eq!(messages.len(), 1);
        assert!(
            matches!(&messages[0].data, RecordData::Message(message) if message.text == answer.text)
        );
    }

    #[tokio::test]
    async fn fresh_family_protected_refresh_retries_exactly_and_still_needs_owner_review() {
        let (_root, run_request, note) = super::super::tests::fixture();
        {
            let mut store = Store::open(&run_request.data_dir).unwrap();
            store.set_workspace_purpose(WorkspacePurpose::Test).unwrap();
            owner_edit(&mut store, &note, 1, "Protected original", true);
        }
        let (old_scope, _) = begin(&run_request).unwrap();
        super::super::tests::read_all(&old_scope);
        let original = prepare(
            &old_scope,
            PrepareArgs {
                writing: None,
                request_key: "old-family".into(),
                request: request(&note, 2, "Old protected candidate", true),
            },
        )
        .unwrap();
        drop(old_scope);
        {
            let mut store = Store::open(&run_request.data_dir).unwrap();
            owner_edit(&mut store, &note, 2, "Current protected owner text", true);
        }
        let (scope, input) = begin(&run_request).unwrap();
        super::super::tests::read_all(&scope);
        let original_note = scope.store.lock().unwrap().record(&note).unwrap().unwrap();
        let replacement = scope
            .store
            .lock()
            .unwrap()
            .allocate_operation(&format!("{}:refresh:protected-fresh", scope.run))
            .unwrap();
        let fresh_request = request(&note, 3, "New protected proposal", true);
        let args = json!({"operation":original["operation"],"request_key":"protected-fresh","request":fresh_request});
        assert!(validate_refresh(&args));
        let model = MockCompletionModel::from_stream_turns([
            vec![
                MockStreamEvent::tool_call("refresh", "refresh_candidate", args.clone()),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![
                MockStreamEvent::tool_call("retry", "refresh_candidate", args),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![
                MockStreamEvent::tool_call(
                    "apply",
                    "apply_changes",
                    json!({"operation":replacement}),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![MockStreamEvent::final_response_with_total_tokens(1)],
        ]);
        let activity = Arc::new(Mutex::new(Vec::new()));
        let stored = Arc::new(Mutex::new(Vec::new()));
        let observed_activity = activity.clone();
        let observed_stored = stored.clone();
        let observed_scope = scope.clone();
        let observed_replacement = replacement.clone();
        let answer = run_model(
            model.clone(),
            scope.clone(),
            &input,
            ReasoningEffort::Medium,
            4,
            true,
            CancellationToken::new(),
            Arc::new(move |event| {
                if let AiEvent::Activity(label) = event {
                    if label == "< refresh_candidate · success" {
                        observed_stored.lock().unwrap().push(
                            observed_scope
                                .store
                                .lock()
                                .unwrap()
                                .prepared(&observed_replacement)
                                .unwrap(),
                        );
                    }
                    observed_activity.lock().unwrap().push(label);
                }
            }),
        )
        .await;
        assert!(matches!(answer.terminal, AiTerminal::Completed));
        assert_eq!(model.request_count(), 4);
        assert_eq!(
            activity
                .lock()
                .unwrap()
                .iter()
                .filter(|s| s.as_str() == "< apply_changes · NeedsReview")
                .count(),
            1
        );
        let stored = stored.lock().unwrap();
        assert_eq!(stored.len(), 2);
        assert_eq!(
            stored[0], stored[1],
            "identical refresh retry preserves exact immutable candidate"
        );
        assert_eq!(stored[0].request.writes, fresh_request.writes);
        let store = scope.store.lock().unwrap();
        assert_eq!(store.prepared(&replacement).unwrap(), stored[0]);
        assert_eq!(
            store
                .operation_info(original["operation"].as_str().unwrap())
                .unwrap()
                .state,
            brn_threads_core::OperationState::Retired
        );
        assert_eq!(
            store.operation_info(replacement.as_str()).unwrap().state,
            brn_threads_core::OperationState::Prepared
        );
        assert!(store.receipt(&replacement).unwrap().is_none());
        assert_eq!(store.record(&note).unwrap().unwrap(), original_note);
        let RecordData::Run(run) = store.record(&scope.run).unwrap().unwrap().data else {
            panic!()
        };
        assert!(run.operations.contains(&replacement));
    }

    #[tokio::test]
    async fn forged_refresh_writing_rejects_both_batch_orders_before_effects() {
        for writing in [
            Value::Null,
            json!({"quotations":[],"requirements_reported":[]}),
        ] {
            for progress_first in [false, true] {
                let (_root, run_request, note) = super::super::tests::fixture();
                Store::open(&run_request.data_dir)
                    .unwrap()
                    .set_workspace_purpose(WorkspacePurpose::Test)
                    .unwrap();
                let (scope, input) = begin(&run_request).unwrap();
                super::super::tests::read_all(&scope);
                let prepared = prepare(
                    &scope,
                    PrepareArgs {
                        writing: None,
                        request_key: "before-invalid-refresh".into(),
                        request: request(&note, 1, "Prior candidate", false),
                    },
                )
                .unwrap();
                let operation = prepared["operation"].as_str().unwrap();
                let (
                    original_note,
                    original_prepared,
                    original_operations,
                    original_progress,
                    candidate_count,
                ) = {
                    let store = scope.store.lock().unwrap();
                    let RecordData::Run(run) = store.record(&scope.run).unwrap().unwrap().data
                    else {
                        panic!()
                    };
                    (
                        store.record(&note).unwrap().unwrap(),
                        store
                            .prepared(&store.operation(operation).unwrap())
                            .unwrap(),
                        run.operations,
                        run.progress,
                        store.candidates().unwrap().len(),
                    )
                };
                let args = json!({"operation":operation,"request_key":"must-not-register","request":request(&note,1,"Must not prepare",false),"writing":writing});
                assert!(!parse_valid("refresh_candidate", &args));
                let bad = MockStreamEvent::tool_call("forged", "refresh_candidate", args);
                let progress = MockStreamEvent::tool_call(
                    "progress",
                    "retain_progress",
                    json!({"summary":"MUST_NOT_RETAIN_REFRESH_BATCH"}),
                );
                let mut batch = if progress_first {
                    vec![progress, bad]
                } else {
                    vec![bad, progress]
                };
                batch.push(MockStreamEvent::final_response_with_total_tokens(1));
                let model = MockCompletionModel::from_stream_turns([batch]);
                let activity = Arc::new(Mutex::new(Vec::new()));
                let seen = activity.clone();
                let answer = run_model(
                    model.clone(),
                    scope.clone(),
                    &input,
                    ReasoningEffort::Medium,
                    4,
                    true,
                    CancellationToken::new(),
                    Arc::new(move |e| {
                        if let AiEvent::Activity(s) = e {
                            seen.lock().unwrap().push(s);
                        }
                    }),
                )
                .await;
                assert!(
                    matches!(
                        answer.terminal,
                        AiTerminal::Failed(AiError {
                            kind: AiErrorKind::InvalidToolUse,
                            ..
                        })
                    ),
                    "{writing}/{progress_first}: {:?}",
                    answer.terminal
                );
                assert_eq!(model.request_count(), 1);
                assert!(!activity.lock().unwrap().iter().any(|s| s.starts_with("> ")));
                let store = scope.store.lock().unwrap();
                let RecordData::Run(run) = store.record(&scope.run).unwrap().unwrap().data else {
                    panic!()
                };
                assert_eq!(run.progress, original_progress);
                assert_eq!(run.operations, original_operations);
                assert_eq!(store.candidates().unwrap().len(), candidate_count);
                assert_eq!(
                    store.prepared(&original_prepared.operation).unwrap(),
                    original_prepared
                );
                assert_eq!(store.record(&note).unwrap().unwrap(), original_note);
                assert!(
                    store
                        .receipt(&original_prepared.operation)
                        .unwrap()
                        .is_none()
                );
            }
        }
    }
}
