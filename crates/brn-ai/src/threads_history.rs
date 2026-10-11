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
    match args.version {
        Some(version) => {
            let note = store
                .read_note_revision(&args.id, version, args.start, args.max_bytes)
                .map_err(storage)?;
            Ok(json!({"outcome":"HistoricalRevision","note":note}))
        }
        None => serde_json::to_value(
            store
                .note_revisions(&args.id, u64::try_from(args.start).map_err(storage)?)
                .map_err(storage)?,
        )
        .map_err(storage),
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
    serde_json::from_value::<RefreshArgs>(args.clone()).is_ok_and(|parsed| {
        !parsed.operation.is_empty()
            && parsed.operation.len() <= 256
            && parse_valid(
                "prepare_changes",
                &json!({"request_key":parsed.request_key,"request":parsed.request}),
            )
            && serde_json::to_value(&parsed.request)
                .is_ok_and(|request| args.get("request") == Some(&request))
    })
}
pub(super) fn refresh_schema() -> Value {
    let mut definition = schema("prepare_changes");
    definition["properties"]["operation"] = json!({"type":"string","description":"Existing unapplied candidate from this Thread's run family"});
    definition["required"] = json!(["operation", "request_key", "request"]);
    definition
}
pub(super) fn refresh(scope: &Scope, mut args: RefreshArgs) -> AiResult<Value> {
    if !validate_refresh(
        &json!({"operation":args.operation,"request_key":args.request_key,"request":args.request}),
    ) {
        return Err(rejected());
    }
    let mut store = scope.store.lock().map_err(storage)?;
    live(scope, &store)?;
    let original = store.operation(&args.operation).map_err(storage)?;
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
    sources::materialize_selected_sources(&scope.sources, &store, &replacement, &mut args.request)?;
    let authority = bind(
        scope,
        HostAuthority::runtime("Threads candidate refresh", &scope.run),
    );
    let prepared = store
        .refresh_candidate(&original, &replacement, &args.request, &authority)
        .map_err(storage)?;
    serde_json::to_value(prepared).map_err(storage)
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
        let original = prepare(
            &scope,
            PrepareArgs {
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
        let original = prepare(
            &old_scope,
            PrepareArgs {
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
        let (scope, input) = begin(&run_request).unwrap();
        let original = prepare(
            &scope,
            PrepareArgs {
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
            model,
            scope.clone(),
            &input,
            ReasoningEffort::Medium,
            4,
            true,
            CancellationToken::new(),
            Arc::new(|_| {}),
        )
        .await;
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
}
