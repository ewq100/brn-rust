//! Pre-effect correction over the production hook, canonical store and actual
//! Responses/Chat encoders. All models/transports and stored content are synthetic.
use super::*;
use rig::test_utils::{
    MockCompletionModel, MockHttpResponse, MockStreamEvent, SequencedHttpClient,
};

fn turn(calls: &[(&str, &str, Value)]) -> Vec<MockStreamEvent> {
    calls
        .iter()
        .map(|(id, name, args)| MockStreamEvent::tool_call(*id, *name, args.clone()))
        .chain([MockStreamEvent::final_response_with_total_tokens(1)])
        .collect()
}
fn done() -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::text("Done"),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}
fn creation(key: &str) -> Value {
    json!({"request_key":key,"request":ChangeRequest {
        reason:"Synthetic correction witness".into(),
        writes:vec![Put { id:"new:0".into(),expected_version:None,archived:false,data:RecordData::Note(Note::working("Corrected", "Exact õun content")) }], inputs:vec![]
    }})
}
fn run_state(scope: &Scope) -> Run {
    let RecordData::Run(run) = scope
        .store
        .lock()
        .unwrap()
        .record(&scope.run)
        .unwrap()
        .unwrap()
        .data
    else {
        panic!("run")
    };
    run
}
fn invalid(answer: &AiAnswer) {
    assert!(
        matches!(&answer.terminal,AiTerminal::Failed(e) if e.kind==AiErrorKind::InvalidToolUse),
        "{answer:?}"
    );
}
async fn scripted(
    scope: Arc<Scope>,
    input: &str,
    turns: Vec<Vec<MockStreamEvent>>,
    limit: u16,
) -> (AiAnswer, MockCompletionModel, Vec<AiEvent>) {
    let model = MockCompletionModel::from_stream_turns(turns);
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let answer = run_model(
        model.clone(),
        scope,
        input,
        ReasoningEffort::Medium,
        limit,
        true,
        CancellationToken::new(),
        Arc::new(move |e| sink.lock().unwrap().push(e)),
    )
    .await;
    let captured = events.lock().unwrap().clone();
    (answer, model, captured)
}

#[tokio::test]
async fn mixed_argument_errors_skip_every_sibling_then_correct_without_identity_conflict() {
    for bad in [json!({"ids":[]}), json!({"ids":7,"query":null})] {
        for valid_first in [false, true] {
            let (_root, mut request, _) = tests::fixture();
            request.trace = TraceLevel::Full;
            let (scope, input) = begin(&request).unwrap();
            let change = creation("skipped-then-corrected");
            let mut calls = vec![
                ("bad", "read_records", bad.clone()),
                ("write", "prepare_changes", change.clone()),
            ];
            if valid_first {
                calls.reverse();
            }
            let observed = Arc::new(AtomicBool::new(false));
            let checked = observed.clone();
            let inspect = scope.clone();
            let model = MockCompletionModel::from_stream_turns([
                turn(&calls),
                turn(&[
                    ("fixed", "read_records", json!({"ids":[],"query":null})),
                    ("write-fixed", "prepare_changes", change.clone()),
                ]),
                done(),
            ]);
            let answer=run_model(model.clone(),scope.clone(),&input,ReasoningEffort::Medium,4,true,CancellationToken::new(),Arc::new(move |event| {
                if matches!(event,AiEvent::Activity(ref text) if text.starts_with("Tool arguments rejected")) {
                    assert!(run_state(&inspect).operations.is_empty());
                    assert!(inspect.store.lock().unwrap().candidates().unwrap().is_empty());
                    checked.store(true,Ordering::SeqCst);
                }
            })).await;
            assert!(
                matches!(answer.terminal, AiTerminal::Completed),
                "{answer:?}"
            );
            assert!(observed.load(Ordering::SeqCst));
            assert_eq!(model.request_count(), 3);
            let history = serde_json::to_value(&model.requests()[1].chat_history)
                .unwrap()
                .to_string();
            assert!(history.contains("no tool in this batch executed"));
            assert!(history.contains("bad") && history.contains("write"));
            let trace = scope.trace.as_ref().unwrap().finish("fixture", false, 4);
            assert_eq!(trace.rounds, 2, "rejected batch spends a round");
            assert_eq!(
                trace
                    .calls
                    .iter()
                    .map(|c| c.tool.as_str())
                    .collect::<Vec<_>>(),
                ["read_records", "prepare_changes"]
            );
            let original =
                prepare(&scope, serde_json::from_value(change.clone()).unwrap()).unwrap();
            assert_eq!(
                run_state(&scope).operations.len(),
                1,
                "skipped request never allocated an operation"
            );
            let operation = original["operation"].as_str().unwrap().to_owned();
            let first = apply(
                &scope,
                ApplyArgs {
                    operation: operation.clone(),
                },
            )
            .unwrap();
            let replay = apply(
                &scope,
                ApplyArgs {
                    operation: operation.clone(),
                },
            )
            .unwrap();
            assert_eq!(first["receipt"], replay["receipt"]);
            assert_eq!(first["outcome"], "Applied");
            let mut different = change;
            different["request"]["reason"] = json!("Different contents cannot reuse the key");
            assert!(prepare(&scope, serde_json::from_value(different).unwrap()).is_err());
            let store = scope.store.lock().unwrap();
            let created = store
                .records()
                .unwrap()
                .into_iter()
                .filter(|r| matches!(&r.data,RecordData::Note(n) if n.title=="Corrected"))
                .collect::<Vec<_>>();
            assert_eq!(created.len(), 1);
            assert_eq!(created[0].version, 1);
        }
    }
}

#[tokio::test]
async fn correction_allowance_is_invocation_total_and_success_does_not_replenish_it() {
    let (_root, request, _) = tests::fixture();
    let (scope, input) = begin(&request).unwrap();
    let bad = || {
        turn(&[
            ("bad", "read_records", json!({"ids":7,"query":null})),
            (
                "blocked",
                "retain_progress",
                json!({"summary":"MUST_NOT_EXECUTE"}),
            ),
        ])
    };
    let (answer, model, events) = scripted(
        scope.clone(),
        &input,
        vec![
            bad(),
            turn(&[(
                "good",
                "retain_progress",
                json!({"summary":"One valid turn"}),
            )]),
            bad(),
            bad(),
            done(),
        ],
        8,
    )
    .await;
    invalid(&answer);
    assert_eq!(model.request_count(), 4);
    assert_eq!(run_state(&scope).progress, "One valid turn");
    let feedback = events
        .iter()
        .filter_map(|e| {
            if let AiEvent::Activity(s) = e {
                Some(s.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        feedback
            .iter()
            .filter(|s| s.starts_with("Tool arguments rejected"))
            .count(),
        2
    );
    assert!(feedback.iter().any(|s| s.contains("allowance exhausted")));
    assert!(
        !feedback.iter().any(|s| s.contains("MUST_NOT_EXECUTE")),
        "denials do not log the original args"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AiEvent::BudgetProgress { tool_rounds: 3, .. })),
        "rejected turns consume existing rounds"
    );
}

#[tokio::test]
async fn hard_refusal_anywhere_wins_over_an_eligible_shape_error() {
    let (_root, request, _) = tests::fixture();
    let (initial, _) = begin(&request).unwrap();
    let source = initial.store.lock().unwrap().records().unwrap();
    let mut forbidden = source
        .into_iter()
        .filter_map(|record| {
            matches!(
                record.data,
                RecordData::Run(_) | RecordData::Settings(_) | RecordData::Message(_)
            )
            .then_some(record.data)
        })
        .collect::<Vec<_>>();
    forbidden.push(RecordData::Thread(Thread {
        title: "Resolved".into(),
        state: ThreadState::Resolved,
        attention: vec![],
        private_context: false,
        context_epoch: 0,
    }));
    assert!(forbidden.len() >= 2);
    for data in forbidden {
        for name in ["prepare_changes", "refresh_candidate"] {
            for policy_first in [false, true] {
                let (_root, request, _) = tests::fixture();
                let (scope, input) = begin(&request).unwrap();
                let mut args = creation("forbidden");
                args["request"]["writes"][0]["data"] = serde_json::to_value(&data).unwrap();
                if name == "refresh_candidate" {
                    args["operation"] = json!("old");
                }
                assert_eq!(
                    validate_arguments(name, &args),
                    Err(ArgumentError::Forbidden)
                );
                for field in ["request_key", "extra_envelope_field"] {
                    let mut malformed = args.clone();
                    malformed[field] = json!(42);
                    assert_eq!(
                        validate_arguments(name, &malformed),
                        Err(ArgumentError::Forbidden),
                        "a shape error cannot hide a typed forbidden request"
                    );
                }
                let mut calls = vec![
                    ("shape", "read_records", json!({"ids":7})),
                    ("policy", name, args),
                ];
                if policy_first {
                    calls.reverse();
                }
                let (answer, model, events) =
                    scripted(scope.clone(), &input, vec![turn(&calls), done()], 4).await;
                invalid(&answer);
                assert_eq!(model.request_count(), 1);
                assert!(run_state(&scope).operations.is_empty());
                assert!(events.iter().all(|e|!matches!(e,AiEvent::Activity(s) if s.starts_with("Tool arguments rejected")||s.starts_with("> "))));
            }
        }
    }
    for name in ["unknown_tool", "submit_findings", "search_notes"] {
        let (_root, request, _) = tests::fixture();
        let (scope, input) = begin(&request).unwrap();
        let (answer, model, _) = scripted(
            scope.clone(),
            &input,
            vec![
                turn(&[
                    ("shape", "read_records", json!({"ids":7})),
                    ("foreign", name, json!({})),
                ]),
                done(),
            ],
            4,
        )
        .await;
        invalid(&answer);
        assert_eq!(
            model.request_count(),
            1,
            "unoffered/unknown calls get no correction"
        );
        assert!(run_state(&scope).operations.is_empty());
    }
}

#[tokio::test]
async fn correction_obeys_round_limit_stop_and_current_scope_before_another_provider_call() {
    for stop in ["rounds", "cancel", "scope"] {
        let (_root, request, _) = tests::fixture();
        let (scope, input) = begin(&request).unwrap();
        let model = MockCompletionModel::from_stream_turns([
            turn(&[("bad", "read_records", json!({"ids":7,"query":null}))]),
            turn(&[(
                "fixed",
                "retain_progress",
                json!({"summary":"MUST_NOT_EXECUTE"}),
            )]),
            done(),
        ]);
        let cancel = CancellationToken::new();
        let control = cancel.clone();
        let state = scope.clone();
        let answer = run_model(
            model.clone(),
            scope.clone(),
            &input,
            ReasoningEffort::Medium,
            if stop == "rounds" { 1 } else { 4 },
            true,
            cancel,
            Arc::new(move |e| {
                if matches!(e,AiEvent::Activity(ref s) if s.starts_with("Tool arguments rejected"))
                {
                    if stop == "cancel" {
                        control.cancel();
                    }
                    if stop == "scope" {
                        state.fence.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }),
        )
        .await;
        if stop == "rounds" {
            assert!(
                matches!(&answer.terminal,AiTerminal::Failed(e) if e.kind==AiErrorKind::ToolLimitReached),
                "{answer:?}"
            );
            assert_eq!(model.request_count(), 2);
        } else {
            if stop == "cancel" {
                assert!(
                    matches!(answer.terminal, AiTerminal::Interrupted),
                    "{answer:?}"
                );
            } else {
                // The direct runner preserves its existing cancellation report;
                // the host finalizer separately fences the stale Run terminal.
                assert!(
                    matches!(&answer.terminal, AiTerminal::Failed(e) if e.detail.as_ref().is_some_and(|d| d.origin == "cancelled")),
                    "{answer:?}"
                );
            }
            assert_eq!(model.request_count(), 1);
        }
        assert!(run_state(&scope).progress.is_empty());
        assert!(run_state(&scope).operations.is_empty());
    }
}

#[tokio::test]
async fn corrective_history_is_subject_to_existing_context_request_admission() {
    // Find the current first-request estimate, then admit exactly that request.
    // Feedback grows the next request, which must be refused before the model.
    let (_root, request, _) = tests::fixture();
    let (scope, input) = begin(&request).unwrap();
    let model = MockCompletionModel::from_stream_turns([done()]);
    let _ = run_model(
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
    let mut actual = model.requests().remove(0);
    // Existing admission counts the request before Rig binds the scripted
    // model and reserves response + safety bytes, as its boundary test does.
    actual.model = None;
    let cap = serde_json::to_vec(&actual).unwrap().len() + 1024 + 8192 + 4096;
    let model = MockCompletionModel::from_stream_turns([
        turn(&[("bad", "read_records", json!({"ids":7,"query":null}))]),
        done(),
    ]);
    let answer = run_model_budgeted(
        model.clone(),
        scope.clone(),
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
        context_budget::Budget::test_cap(cap),
    )
    .await;
    assert!(
        matches!(&answer.terminal,AiTerminal::Failed(e) if e.is_context_limit()),
        "{answer:?}"
    );
    assert_eq!(model.request_count(), 1);
    assert!(run_state(&scope).progress.is_empty());
}

fn wire_turn(responses: bool, calls: &[(&str, &str, Value)]) -> String {
    if responses {
        let items=calls.iter().map(|(id,name,args)| json!({"type":"function_call","id":format!("fc_{id}"),"call_id":id,"name":name,"arguments":args.to_string(),"status":"completed"})).collect::<Vec<_>>();
        let mut events=items.iter().enumerate().map(|(i,item)| json!({"type":"response.output_item.done","output_index":i,"sequence_number":i+1,"item":item})).collect::<Vec<_>>();
        events.push(json!({"type":"response.completed","sequence_number":events.len()+1,"response":{"id":"resp_fixture","object":"response","created_at":1,"status":"completed","model":"synthetic-model","output":items,"tools":[],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}));
        cache_tests::sse(&events)
    } else {
        let tools=calls.iter().enumerate().map(|(i,(id,name,args))|json!({"index":i,"id":id,"type":"function","function":{"name":name,"arguments":args.to_string()}})).collect::<Vec<_>>();
        cache_tests::sse(&[
            json!({"id":"chat_fixture","object":"chat.completion.chunk","created":1,"model":"synthetic-model","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":tools},"finish_reason":null}]}),
            json!({"id":"chat_fixture","object":"chat.completion.chunk","created":1,"model":"synthetic-model","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}),
        ])
    }
}
fn wire_done(responses: bool) -> String {
    if responses {
        cache_tests::final_turn()
    } else {
        cache_tests::sse(&[
            json!({"id":"chat_final","object":"chat.completion.chunk","created":1,"model":"synthetic-model","choices":[{"index":0,"delta":{"role":"assistant","content":"Done"},"finish_reason":null}]}),
            json!({"id":"chat_final","object":"chat.completion.chunk","created":1,"model":"synthetic-model","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}),
        ])
    }
}

#[tokio::test]
async fn responses_and_chat_second_requests_pair_each_rejected_call_with_no_effect_feedback() {
    for responses in [true, false] {
        let (_root, request, _) = tests::fixture();
        let (scope, input) = begin(&request).unwrap();
        let http = SequencedHttpClient::new([
            MockHttpResponse::success_typed(
                wire_turn(
                    responses,
                    &[
                        ("bad", "read_records", json!({"ids":7,"query":null})),
                        (
                            "sibling",
                            "retain_progress",
                            json!({"summary":"MUST_NOT_EXECUTE"}),
                        ),
                    ],
                ),
                "text/event-stream",
            ),
            MockHttpResponse::success_typed(
                wire_turn(
                    responses,
                    &[(
                        "fixed",
                        "retain_progress",
                        json!({"summary":"Corrected õun"}),
                    )],
                ),
                "text/event-stream",
            ),
            MockHttpResponse::success_typed(wire_done(responses), "text/event-stream"),
        ]);
        let client = rig::providers::copilot::CopilotConfig::new("synthetic-not-a-credential")
            .connect(http.clone());
        let answer = run_model(
            crate::auth::copilot_completion(&client, "synthetic-model".into(), responses),
            scope.clone(),
            &input,
            ReasoningEffort::Medium,
            4,
            responses,
            CancellationToken::new(),
            Arc::new(|_| {}),
        )
        .await;
        assert!(
            matches!(answer.terminal, AiTerminal::Completed),
            "{responses}: {answer:?}"
        );
        assert_eq!(run_state(&scope).progress, "Corrected õun");
        let requests = http.requests();
        assert_eq!(requests.len(), 3);
        let body: Value = serde_json::from_slice(&requests[1].body).unwrap();
        let (calls, results) = if responses {
            let items = body["input"].as_array().unwrap();
            (
                items
                    .iter()
                    .filter(|i| i["type"] == "function_call")
                    .map(|i| i["call_id"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                items
                    .iter()
                    .filter(|i| i["type"] == "function_call_output")
                    .map(|i| {
                        (
                            i["call_id"].as_str().unwrap(),
                            i["output"].as_str().unwrap(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        } else {
            let messages = body["messages"].as_array().unwrap();
            (
                messages
                    .iter()
                    .filter_map(|m| m["tool_calls"].as_array())
                    .flatten()
                    .map(|c| c["id"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                messages
                    .iter()
                    .filter(|m| m["role"] == "tool")
                    .map(|m| {
                        (
                            m["tool_call_id"].as_str().unwrap(),
                            m["content"].as_str().unwrap(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(calls, ["bad", "sibling"]);
        assert_eq!(results.iter().map(|(id, _)| *id).collect::<Vec<_>>(), calls);
        assert!(
            results
                .iter()
                .all(|(_, text)| text.contains("no tool in this batch executed")
                    && text.contains("read_records")
                    && text.contains("query"))
        );
        assert!(
            results
                .iter()
                .all(|(_, text)| !text.contains("MUST_NOT_EXECUTE")),
            "feedback never echoes raw values"
        );
    }
}

async fn assert_mixed_hard_refusal(name: &str, args: Value) {
    for policy_first in [false, true] {
        let (_root, request, _) = tests::fixture();
        let (scope, input) = begin(&request).unwrap();
        let mut calls = vec![
            ("shape", "read_records", json!({"ids":7,"query":null})),
            ("policy", name, args.clone()),
            (
                "effect",
                "retain_progress",
                json!({"summary":"MUST_NOT_EXECUTE"}),
            ),
        ];
        if policy_first {
            calls.reverse();
        }
        let (answer, model, events) =
            scripted(scope.clone(), &input, vec![turn(&calls), done()], 4).await;
        invalid(&answer);
        assert_eq!(model.request_count(), 1, "{name}: {args}");
        assert!(run_state(&scope).progress.is_empty());
        assert!(run_state(&scope).operations.is_empty());
        assert!(scope.store.lock().unwrap().candidates().unwrap().is_empty());
        assert!(events.iter().all(|event| !matches!(event, AiEvent::Activity(text) if text.starts_with("> ") || text.starts_with("Tool arguments rejected"))), "hard refusal must precede every dispatch and correction");
    }
}

#[tokio::test]
async fn complete_forbidden_record_data_cannot_hide_behind_other_malformed_request_fields() {
    for data in [
        RecordData::Message(Message {
            thread: "synthetic".into(),
            role: "assistant".into(),
            text: "Never save".into(),
        }),
        RecordData::Thread(Thread {
            title: "Never resolve".into(),
            state: ThreadState::Resolved,
            attention: vec![],
            private_context: false,
            context_epoch: 0,
        }),
    ] {
        for fault in [
            "reason",
            "inputs",
            "sibling",
            "write_envelope",
            "request_key",
        ] {
            for name in ["prepare_changes", "refresh_candidate"] {
                let mut args = creation("combined-fault");
                args["request"]["writes"][0]["data"] = serde_json::to_value(&data).unwrap();
                if name == "refresh_candidate" {
                    args["operation"] = json!("old");
                }
                match fault {
                    "reason" => args["request"]["reason"] = json!(42),
                    "inputs" => args["request"]["inputs"] = json!(["not a typed input"]),
                    "sibling" => args["request"]["writes"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"malformed":true})),
                    "write_envelope" => args["request"]["writes"][0]["id"] = json!(42),
                    _ => args["request_key"] = json!(42),
                }
                assert_eq!(
                    validate_arguments(name, &args),
                    Err(ArgumentError::Forbidden),
                    "{fault}/{name}"
                );
                assert_mixed_hard_refusal(name, args).await;
            }
        }
    }
}

#[tokio::test]
async fn refresh_writing_is_a_hard_refusal_even_with_a_malformed_envelope() {
    for writing in [
        Value::Null,
        json!({}),
        json!({"quotations":[],"requirements_reported":[]}),
    ] {
        for malformed in [false, true] {
            let mut args = creation("forged-writing");
            args["operation"] = json!("old");
            args["writing"] = writing.clone();
            if malformed {
                args["request_key"] = json!(42);
                args["request"]["reason"] = json!(42);
            }
            assert_eq!(
                validate_arguments("refresh_candidate", &args),
                Err(ArgumentError::Forbidden)
            );
            assert_mixed_hard_refusal("refresh_candidate", args).await;
        }
    }
}

#[tokio::test]
async fn frozen_skill_access_refusal_is_independent_of_unrelated_shape_errors() {
    let (_root, request, _) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let (text, _) = scope.skills.file("maintain-notes", "SKILL.md").unwrap();
    for args in [
        json!({"id":"maintain-notes","path":"../secret","extra":true}),
        json!({"id":"absent-enabled-id","extra":true}),
        json!({"id":"maintain-notes","path":"not-listed.md","start":"bad type","extra":true}),
        json!({"id":"absent-enabled-id","path":42,"extra":true}),
        json!({"id":"maintain-notes","start":text.len()+1,"extra":true}),
    ] {
        assert_mixed_hard_refusal("read_skill", args).await;
    }
}

#[tokio::test]
async fn allowed_request_and_skill_format_errors_still_get_bounded_correction() {
    for name in ["prepare_changes", "read_skill"] {
        let (_root, request, _) = tests::fixture();
        let (scope, input) = begin(&request).unwrap();
        let valid = if name == "prepare_changes" {
            creation("corrected-envelope")
        } else {
            json!({"id":"maintain-notes"})
        };
        let mut malformed = valid.clone();
        if name == "prepare_changes" {
            malformed["request"]["reason"] = json!(42);
        } else {
            malformed["start"] = json!(-1);
        }
        assert!(matches!(
            validate_arguments(name, &malformed),
            Err(ArgumentError::Shape(_))
        ));
        let (answer, model, _) = scripted(
            scope.clone(),
            &input,
            vec![
                turn(&[("bad", name, malformed)]),
                turn(&[("fixed", name, valid)]),
                done(),
            ],
            4,
        )
        .await;
        assert!(
            matches!(answer.terminal, AiTerminal::Completed),
            "{name}: {answer:?}"
        );
        assert_eq!(model.request_count(), 3);
        if name == "prepare_changes" {
            assert_eq!(run_state(&scope).operations.len(), 1);
            assert_eq!(scope.store.lock().unwrap().candidates().unwrap().len(), 1);
        } else {
            assert!(
                run_state(&scope)
                    .loaded_skill_details
                    .iter()
                    .any(|skill| skill.id == "maintain-notes")
            );
        }
    }
}
