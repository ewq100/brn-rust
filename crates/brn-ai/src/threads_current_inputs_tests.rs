use super::*;

fn summary_args(input: &str, expectation: Option<&str>) -> Value {
    let mut input = json!({"record": input, "version": 1});
    if let Some(expectation) = expectation {
        input["expectation"] = expectation.into();
    }
    json!({
        "request_key": "Monday summary",
        "request": {
            "reason": "Derived delivery summary",
            "writes": [{"id": "new:0", "expected_version": null, "archived": false,
                "data": {"Note": {"title": "Summary", "markdown": "Delivery Monday", "protected": false,
                    "confirmed": false, "superseded_by": null, "import": null}}}],
            "inputs": [input]
        }
    })
}

fn change_input(directory: &std::path::Path, input: &str, version: u64, text: &str) {
    let mut store = Store::open(directory).unwrap();
    let op = store
        .allocate_operation(&format!("owner input {version}"))
        .unwrap();
    let owner = HostAuthority::owner("synthetic owner");
    let req = ChangeRequest {
        reason: "Current input changed".into(),
        writes: vec![Put {
            id: input.into(),
            expected_version: Some(version),
            archived: false,
            data: RecordData::Note(Note::working("Decision", text)),
        }],
        inputs: vec![],
    };
    store.prepare_owner(&op, &req, &owner).unwrap();
    assert!(matches!(
        store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
}

#[test]
fn runtime_input_schema_accepts_omitted_explicit_current_and_historical_only() {
    for expectation in [None, Some("Current"), Some("Historical")] {
        let value = summary_args("input", expectation);
        assert!(parse_valid("prepare_changes", &value));
        let args: PrepareArgs = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            args.request.inputs[0].expectation,
            if expectation == Some("Historical") {
                InputExpectation::Historical
            } else {
                InputExpectation::Current
            }
        );
        let mut refresh = value;
        refresh["operation"] = "old-operation".into();
        assert!(history::validate_refresh(&refresh));
    }
    for expectation in ["", "Ignore", "current"] {
        assert!(!parse_valid(
            "prepare_changes",
            &summary_args("input", Some(expectation))
        ));
    }
    let mut unknown = summary_args("input", Some("Current"));
    unknown["request"]["inputs"][0]["unused"] = true.into();
    assert!(!parse_valid("prepare_changes", &unknown));
    let schema = schema("prepare_changes");
    assert_eq!(
        schema["properties"]["request"]["properties"]["inputs"]["items"]["properties"]["expectation"]
            ["enum"],
        json!(["Current", "Historical"])
    );
}

#[test]
fn runtime_current_reads_stale_before_or_after_prepare_with_no_saved_summary() {
    for change_before_prepare in [false, true] {
        let (_root, request, input) = super::tests::fixture();
        let (scope, _) = begin(&request).unwrap();
        let read = read_records(
            &scope,
            ReadArgs {
                ids: vec![input.clone()],
                query: None,
            },
        )
        .unwrap();
        assert_eq!(read["records"][0]["version"], 1);
        assert_eq!(read["records"][0]["data"]["Note"]["markdown"], "Monday");
        let args: PrepareArgs = serde_json::from_value(summary_args(&input, None)).unwrap();
        if change_before_prepare {
            change_input(&request.data_dir, &input, 1, "Friday");
        }
        let prepared: Prepared = serde_json::from_value(prepare(&scope, args).unwrap()).unwrap();
        assert_eq!(
            prepared.request.inputs[0].expectation,
            InputExpectation::Current
        );
        if !change_before_prepare {
            change_input(&request.data_dir, &input, 1, "Friday");
        }
        let result = apply(
            &scope,
            ApplyArgs {
                operation: prepared.operation.as_str().into(),
            },
        )
        .unwrap();
        assert_eq!(result["outcome"], "Stale");
        assert_eq!(result["records"], json!([input]));
        let store = scope.store.lock().unwrap();
        assert!(store.receipt(&prepared.operation).unwrap().is_none());
        assert!(
            store
                .record(&prepared.operation.creation_id(0))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn runtime_fresh_replacement_replays_after_input_changes_and_explicit_history_remains_valid() {
    let (_root, request, input) = super::tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let args: PrepareArgs = serde_json::from_value(summary_args(&input, None)).unwrap();
    let original: Prepared = serde_json::from_value(prepare(&scope, args).unwrap()).unwrap();
    change_input(&request.data_dir, &input, 1, "Friday");
    let mut value = summary_args(&input, Some("Current"));
    value["request_key"] = "fresh Friday summary".into();
    value["request"]["writes"][0]["data"]["Note"]["markdown"] = "Delivery Friday".into();
    value["operation"] = original.operation.as_str().into();
    assert!(refresh_candidate(&scope, serde_json::from_value(value.clone()).unwrap()).is_err());
    value["request"]["inputs"][0]["version"] = 2.into();
    let refreshed: Prepared = serde_json::from_value(
        refresh_candidate(&scope, serde_json::from_value(value.clone()).unwrap()).unwrap(),
    )
    .unwrap();
    let result = apply(
        &scope,
        ApplyArgs {
            operation: refreshed.operation.as_str().into(),
        },
    )
    .unwrap();
    assert_eq!(result["outcome"], "Applied");
    change_input(&request.data_dir, &input, 2, "Saturday");
    assert_eq!(
        refresh_candidate(&scope, serde_json::from_value(value).unwrap()).unwrap(),
        serde_json::to_value(&refreshed).unwrap()
    );
    assert_eq!(
        apply(
            &scope,
            ApplyArgs {
                operation: refreshed.operation.as_str().into()
            }
        )
        .unwrap(),
        result
    );
    let historical = read_history(
        &scope,
        HistoryArgs {
            id: input.clone(),
            version: Some(1),
            start: 0,
            max_bytes: 1024,
        },
    )
    .unwrap();
    assert_eq!(historical["revision_input"]["expectation"], "Historical");
    let mut args = summary_args(&input, Some("Historical"));
    args["request_key"] = "accepted historical Monday citation".into();
    let prepared: Prepared =
        serde_json::from_value(prepare(&scope, serde_json::from_value(args).unwrap()).unwrap())
            .unwrap();
    assert_eq!(
        prepared.request.inputs[0].expectation,
        InputExpectation::Historical
    );
    assert_eq!(
        apply(
            &scope,
            ApplyArgs {
                operation: prepared.operation.as_str().into()
            }
        )
        .unwrap()["outcome"],
        "Applied"
    );
}

#[test]
fn runtime_refresh_can_explicitly_replace_an_old_current_candidate_with_historical_citation() {
    let (_root, request, input) = super::tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let original: Prepared = serde_json::from_value(
        prepare(
            &scope,
            serde_json::from_value(summary_args(&input, None)).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    change_input(&request.data_dir, &input, 1, "Friday");
    let mut value = summary_args(&input, Some("Historical"));
    value["request_key"] = "intentional historical replacement".into();
    value["operation"] = original.operation.as_str().into();
    let replacement: Prepared = serde_json::from_value(
        refresh_candidate(&scope, serde_json::from_value(value).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(replacement.request.inputs[0].version, 1);
    assert_eq!(
        replacement.request.inputs[0].expectation,
        InputExpectation::Historical
    );
    assert_eq!(
        apply(
            &scope,
            ApplyArgs {
                operation: replacement.operation.as_str().into()
            }
        )
        .unwrap()["outcome"],
        "Applied"
    );
}
