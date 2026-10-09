use super::*;

const ROUTES: [(Provider, &str, bool); 3] = [
    (Provider::Chatgpt, "gpt-6-luna", true),
    (Provider::Copilot, "gpt-5.5", false),
    (Provider::Copilot, "gpt-5.3-codex", true),
];
const PRIVATE_ARGS: &str = "P3_PRIVATE_ARGUMENT_9b774a: not JSON";
pub(super) const FEEDBACK: &str = "Use valid JSON arguments matching the advertised tool schema. No tools in the rejected response executed.";

// Diagnostic for a parked candidate: this records the pinned wire gap without
// weakening the required whole-response rollback witnesses below.
#[tokio::test]
#[ignore = "manual pinned-provider seam diagnosis; not acceptance of dropped malformed calls"]
async fn pinned_wire_malformed_observation() {
    for (provider, model, responses) in ROUTES {
        for invalid_first in [false, true] {
            let mut calls = vec![
                ("propose_actions", proposal("synthetic peer").to_string()),
                ("read_note", PRIVATE_ARGS.into()),
            ];
            if invalid_first {
                calls.reverse();
            }
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![
                    success(raw_tool_sse_with_prefix(responses, &calls, "diagnostic_")),
                    success(text_sse(responses, "safe final")),
                ],
            )
            .await;
            let notes = Arc::new(Notes::default());
            let proposals = Arc::new(Proposals::default());
            let (answer, events) = investigate(
                client,
                notes.clone(),
                proposals.clone(),
                2,
                CancellationToken::new(),
            )
            .await;
            let drafts = proposals.retained.lock().unwrap().len();
            println!(
                "PINNED_WIRE provider={provider:?} model={model} responses={responses} invalid_first={invalid_first} completed={} requests={} reads={} drafts={drafts} progress={:?}",
                matches!(answer.terminal, AiTerminal::Completed),
                http.bodies().len(),
                notes.calls.load(Ordering::SeqCst),
                progress(&events)
            );
            assert!(
                matches!(answer.terminal, AiTerminal::Completed),
                "{answer:?}"
            );
            assert_eq!(answer.text, "safe final");
            assert_eq!(http.bodies().len(), 2);
            assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
            assert_eq!(drafts, usize::from(responses));
            http.assert_consumed();
        }
    }
}

#[derive(Default)]
struct Proposals {
    retained: Mutex<Vec<Value>>,
    knowledge: bool,
}
impl ProposalTools for Proposals {
    fn knowledge_enabled(&self) -> bool {
        self.knowledge
    }
    fn propose_actions(&self, args: ActionProposalArgs) -> AiResult<Value> {
        self.retained
            .lock()
            .unwrap()
            .push(serde_json::to_value(args).unwrap());
        Ok(json!({"retained":"synthetic draft"}))
    }
}

fn proposal(title: &str) -> Value {
    json!({"title":title,"source_paths":["source.md"],"action_changes":[{
        "kind":"create","data":{
            "title":"Exact õ\r\n", "description":"\u{feff}日本語", "state":"open",
            "owner":null,"related_person":null,"related_project":null,"sources":[],
            "thread":null,"due_on":null,"follow_up_on":null,"dependencies":[],
            "parent":null,"follows_up":null,"priority":null
        }
    }]})
}

fn malformed(responses: bool, name: &str, prefix: &str) -> String {
    raw_tool_sse_with_prefix(responses, &[(name, PRIVATE_ARGS.into())], prefix)
}

async fn investigate(
    client: ProviderClient,
    notes: Arc<Notes>,
    proposals: Arc<Proposals>,
    limit: u16,
    cancel: CancellationToken,
) -> (AiAnswer, Vec<AiEvent>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let answer = answer_with_proposals_and_images_with_limit(
        client,
        "Inspect saved context",
        &[],
        ReasoningEffort::Medium,
        notes,
        proposals,
        &[],
        limit,
        cancel,
        Arc::new(move |event| sink.lock().unwrap().push(event)),
    )
    .await;
    (answer, events.lock().unwrap().clone())
}

fn progress(events: &[AiEvent]) -> Vec<(u16, u16, u16)> {
    events
        .iter()
        .filter_map(|event| match event {
            AiEvent::BudgetProgress {
                model_turns,
                tool_rounds,
                max_tool_rounds,
            } => Some((*model_turns, *tool_rounds, *max_tool_rounds)),
            _ => None,
        })
        .collect()
}

fn failed(answer: &AiAnswer, kind: AiErrorKind) {
    assert!(
        matches!(&answer.terminal, AiTerminal::Failed(error) if error.kind == kind),
        "{answer:?}"
    );
    assert!(!format!("{answer:?}").contains(PRIVATE_ARGS));
}

pub(super) fn feedback_outputs(body: &Value, responses: bool) -> Vec<String> {
    body[if responses { "input" } else { "messages" }]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| {
            if responses && item["type"] == "function_call_output" {
                item["output"].as_str().map(str::to_owned)
            } else if !responses && item["role"] == "tool" {
                item["content"].as_str().map(str::to_owned)
            } else {
                None
            }
        })
        .collect()
}

#[tokio::test]
async fn proposal_capable_ask_and_inbox_correct_once_with_existing_calls_and_progress() {
    for (provider, model, responses) in ROUTES {
        for knowledge in [false, true] {
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![
                    success(malformed(responses, "read_note", "bad_")),
                    success(tool_sse_with_prefix(
                        responses,
                        &[("read_note", json!({"path":"a.md"}))],
                        "read_",
                    )),
                    success(text_sse(responses, "Exact final õ\r\n")),
                ],
            )
            .await;
            let notes = Arc::new(Notes::default());
            let proposals = Arc::new(Proposals {
                knowledge,
                ..Default::default()
            });
            let (answer, events) = investigate(
                client,
                notes.clone(),
                proposals.clone(),
                2,
                CancellationToken::new(),
            )
            .await;
            assert!(
                matches!(answer.terminal, AiTerminal::Completed),
                "{answer:?}"
            );
            assert_eq!(answer.text, "Exact final õ\r\n");
            assert_eq!(notes.calls.load(Ordering::SeqCst), 1);
            assert!(proposals.retained.lock().unwrap().is_empty());
            assert_eq!(
                progress(&events),
                [(1, 0, 2), (2, 0, 2), (2, 1, 2), (3, 1, 2)]
            );
            assert!(!format!("{events:?}").contains(PRIVATE_ARGS));
            let bodies = http.bodies();
            assert_eq!(bodies.len(), 3);
            assert_eq!(feedback_outputs(&bodies[1], responses), [FEEDBACK]);
            assert!(bodies.iter().all(|body| body["model"] == model));
            http.assert_consumed();
        }
    }
}

#[tokio::test]
async fn all_seven_registered_reads_can_correct_non_json_but_second_malformed_fails() {
    for (provider, model, responses) in ROUTES {
        for name in [
            "search_notes",
            "read_note",
            "read_note_range",
            "list_notes",
            "read_action",
            "list_actions",
            "read_conflicts",
        ] {
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![
                    success(malformed(responses, name, "bad_")),
                    success(text_sse(responses, "corrected answer")),
                ],
            )
            .await;
            let notes = Arc::new(Notes::default());
            let (answer, events) = investigate(
                client,
                notes.clone(),
                Arc::new(Proposals::default()),
                1,
                CancellationToken::new(),
            )
            .await;
            assert!(
                matches!(answer.terminal, AiTerminal::Completed),
                "{name}: {answer:?}"
            );
            assert_eq!(progress(&events), [(1, 0, 1), (2, 0, 1)]);
            assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
            assert_eq!(feedback_outputs(&http.bodies()[1], responses), [FEEDBACK]);
            http.assert_consumed();
        }
        let (_root, client, http) = super::client(
            provider,
            model,
            vec![
                success(malformed(responses, "read_note", "first_")),
                success(malformed(responses, "list_notes", "second_")),
            ],
        )
        .await;
        let notes = Arc::new(Notes::default());
        let (answer, events) = investigate(
            client,
            notes.clone(),
            Arc::new(Proposals::default()),
            2,
            CancellationToken::new(),
        )
        .await;
        failed(&answer, AiErrorKind::InvalidToolUse);
        assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
        assert_eq!(http.bodies().len(), 2);
        assert_eq!(progress(&events), [(1, 0, 2), (2, 0, 2)]);
        http.assert_consumed();
    }
}

#[tokio::test]
async fn final_slot_is_tool_free_even_with_unused_rounds_or_malformed_call() {
    for (provider, model, responses) in ROUTES {
        for last in [
            tool_sse_with_prefix(responses, &[("read_note", json!({"path":"a.md"}))], "last_"),
            tool_sse_with_prefix(
                responses,
                &[("propose_actions", proposal("must not draft"))],
                "last_",
            ),
            malformed(responses, "read_note", "last_"),
        ] {
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![
                    success(malformed(responses, "read_note", "first_")),
                    success(last),
                ],
            )
            .await;
            let notes = Arc::new(Notes::default());
            let proposals = Arc::new(Proposals::default());
            let (answer, events) = investigate(
                client,
                notes.clone(),
                proposals.clone(),
                1,
                CancellationToken::new(),
            )
            .await;
            failed(&answer, AiErrorKind::ToolLimitReached);
            assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
            assert!(proposals.retained.lock().unwrap().is_empty());
            assert_eq!(http.bodies().len(), 2);
            assert_eq!(progress(&events), [(1, 0, 1), (2, 0, 1)]);
            http.assert_consumed();
        }
        // No previous correction was consumed: the first malformed call in the
        // final slot must still fail, without a third provider request.
        let (_root, client, http) = super::client(
            provider,
            model,
            vec![
                success(tool_sse(
                    responses,
                    &[("read_note", json!({"path":"a.md"}))],
                )),
                success(malformed(responses, "read_note", "last_")),
            ],
        )
        .await;
        let notes = Arc::new(Notes::default());
        let (answer, events) = investigate(
            client,
            notes.clone(),
            Arc::new(Proposals::default()),
            1,
            CancellationToken::new(),
        )
        .await;
        failed(&answer, AiErrorKind::ToolLimitReached);
        assert_eq!(notes.calls.load(Ordering::SeqCst), 1);
        assert_eq!(progress(&events), [(1, 0, 1), (1, 1, 1), (2, 1, 1)]);
        http.assert_consumed();
    }
}

#[tokio::test]
async fn mixed_proposal_invalid_read_rolls_back_both_orders_and_retains_prior_draft_and_text_exactly()
 {
    let prior = proposal("Earlier retained \u{feff}draft\r\n日本語");
    let rejected = proposal("Rejected peer must never dispatch");
    let partial = "\u{feff}Partial once õ\r\n";
    for (provider, model, responses) in ROUTES {
        for invalid_first in [false, true] {
            let mut mixed = vec![
                ("propose_actions", rejected.to_string()),
                ("read_note", PRIVATE_ARGS.into()),
            ];
            if invalid_first {
                mixed.reverse();
            }
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![
                    success(tool_sse_with_prefix(
                        responses,
                        &[("propose_actions", prior.clone())],
                        "prior_",
                    )),
                    success(
                        rewrite_tests::partial_sse(responses, partial)
                            + &raw_tool_sse_with_prefix(responses, &mixed, "rejected_"),
                    ),
                    success(tool_sse_with_prefix(
                        responses,
                        &[("read_note", json!({"path":"a.md"}))],
                        "corrected_",
                    )),
                    success(text_sse(responses, "Final")),
                ],
            )
            .await;
            let notes = Arc::new(Notes::default());
            let proposals = Arc::new(Proposals::default());
            let (answer, events) = investigate(
                client,
                notes.clone(),
                proposals.clone(),
                3,
                CancellationToken::new(),
            )
            .await;
            assert!(
                matches!(answer.terminal, AiTerminal::Completed),
                "order {invalid_first}: {answer:?}"
            );
            assert_eq!(
                proposals.retained.lock().unwrap().as_slice(),
                &[prior.clone()]
            );
            assert_eq!(notes.calls.load(Ordering::SeqCst), 1);
            assert_eq!(answer.text, format!("{partial}Final"));
            assert_eq!(
                events
                    .iter()
                    .filter_map(|event| match event {
                        AiEvent::Text(text) => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<String>(),
                answer.text
            );
            assert_eq!(
                progress(&events),
                [
                    (1, 0, 3),
                    (1, 1, 3),
                    (2, 1, 3),
                    (3, 1, 3),
                    (3, 2, 3),
                    (4, 2, 3)
                ]
            );
            assert_eq!(events.iter().filter(|event| matches!(event, AiEvent::ToolStarted { name } if name == "propose_actions")).count(), 1);
            let bodies = http.bodies();
            assert_eq!(bodies.len(), 4);
            let feedback = feedback_outputs(&bodies[2], responses);
            assert_eq!(
                feedback
                    .iter()
                    .filter(|text| text.as_str() == FEEDBACK)
                    .count(),
                1
            );
            assert!(feedback.iter().all(|text| !text.contains(PRIVATE_ARGS)));
            if !invalid_first {
                assert!(feedback.iter().any(
                    |text| text == rig::run::transcript::TOOL_NOT_EXECUTED_DUE_TO_INVALID_PEER
                ));
            }
            assert!(!format!("{events:?}").contains(PRIVATE_ARGS));
            http.assert_consumed();
        }
    }
}

#[tokio::test]
async fn unknown_mutation_rewrite_and_visual_keep_strict_no_correction_policy() {
    for (provider, model, responses) in ROUTES {
        for name in [
            "P3_PRIVATE_UNKNOWN_NAME",
            "propose_actions",
            "propose_knowledge",
            "report_conflict",
        ] {
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![success(malformed(responses, name, "refused_"))],
            )
            .await;
            let proposals = Arc::new(Proposals {
                knowledge: true,
                ..Default::default()
            });
            let (answer, events) = investigate(
                client,
                Arc::new(Notes::default()),
                proposals.clone(),
                2,
                CancellationToken::new(),
            )
            .await;
            failed(&answer, AiErrorKind::InvalidToolUse);
            assert!(proposals.retained.lock().unwrap().is_empty());
            assert!(!format!("{answer:?} {events:?}").contains(name));
            assert_eq!(progress(&events), [(1, 0, 2)]);
            http.assert_consumed();
        }
        let (_root, client, http) = super::client(
            provider,
            model,
            vec![success(malformed(responses, "read_note", "rewrite_"))],
        )
        .await;
        let answer = rewrite(
            client,
            "Strict rewrite",
            ReasoningEffort::Low,
            Arc::new(Notes::default()),
            CancellationToken::new(),
            Arc::new(|_| panic!("strict raw output")),
        )
        .await;
        failed(&answer, AiErrorKind::InvalidToolUse);
        assert!(answer.text.is_empty());
        http.assert_consumed();

        let (_root, client, http) = super::client(
            provider,
            model,
            vec![success(malformed(responses, "read_note", "visual_"))],
        )
        .await;
        let image = VisualImage::png(include_bytes!("fixtures/capability.png").to_vec()).unwrap();
        let answer = interpret_visual(
            client,
            "Strict visual",
            &image,
            ReasoningEffort::Low,
            CancellationToken::new(),
            Arc::new(|_| panic!("strict raw output")),
        )
        .await;
        assert!(matches!(answer.terminal, AiTerminal::Failed(_)));
        assert!(answer.text.is_empty());
        assert_eq!(http.bodies().len(), 1);
        http.assert_consumed();
    }
}

#[tokio::test]
async fn typed_schema_error_continues_and_does_not_consume_non_json_correction() {
    for (provider, model, responses) in ROUTES {
        let (_root, client, http) = super::client(
            provider,
            model,
            vec![
                success(tool_sse_with_prefix(
                    responses,
                    &[("read_note", json!({"path":42}))],
                    "typed_",
                )),
                success(malformed(responses, "read_note", "non_json_")),
                success(text_sse(responses, "Final")),
            ],
        )
        .await;
        let notes = Arc::new(Notes::default());
        let (answer, events) = investigate(
            client,
            notes.clone(),
            Arc::new(Proposals::default()),
            2,
            CancellationToken::new(),
        )
        .await;
        assert!(
            matches!(answer.terminal, AiTerminal::Completed),
            "{answer:?}"
        );
        assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            progress(&events),
            [(1, 0, 2), (1, 1, 2), (2, 1, 2), (3, 1, 2)]
        );
        assert_eq!(
            feedback_outputs(&http.bodies()[2], responses)
                .iter()
                .filter(|text| text.as_str() == FEEDBACK)
                .count(),
            1
        );
        http.assert_consumed();
    }
}

#[tokio::test]
async fn stop_and_deadline_token_before_correction_send_no_late_request() {
    for (provider, model, responses) in ROUTES {
        let (_root, client, http) = super::client(provider, model, vec![]).await;
        let cancel = CancellationToken::new();
        cancel.cancel();
        let (answer, events) = investigate(
            client,
            Arc::new(Notes::default()),
            Arc::new(Proposals::default()),
            2,
            cancel,
        )
        .await;
        assert!(matches!(answer.terminal, AiTerminal::Interrupted));
        assert!(events.is_empty() && http.bodies().is_empty());
        http.assert_consumed();

        for deadline in [false, true] {
            let cancel = CancellationToken::new();
            let (_root, client, http) = super::client(
                provider,
                model,
                vec![success(text_sse(responses, "replaced by pending stream"))],
            )
            .await;
            // Hold the non-JSON argument stream before its closing event, so
            // cancellation precedes rejection/correction admission. Workflow
            // owns the deadline and cancels this same token in production.
            let chunks = malformed(responses, "read_note", "pending_")
                .split_inclusive("\n\n")
                .take(if responses { 2 } else { 1 })
                .collect::<String>();
            let client = rewrite_tests::with_chunks(
                client,
                http.clone(),
                vec![Bytes::from(chunks)],
                (!deadline).then(|| cancel.clone()),
            );
            let expiry = if deadline {
                let token = cancel.clone();
                Some(tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    token.cancel();
                }))
            } else {
                None
            };
            let notes = Arc::new(Notes::default());
            let (answer, events) = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                investigate(
                    client,
                    notes.clone(),
                    Arc::new(Proposals::default()),
                    2,
                    cancel,
                ),
            )
            .await
            .unwrap();
            if let Some(expiry) = expiry {
                expiry.await.unwrap();
            }
            assert!(
                matches!(answer.terminal, AiTerminal::Interrupted),
                "deadline {deadline}: {answer:?}"
            );
            assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
            assert_eq!(http.bodies().len(), 1);
            assert_eq!(progress(&events), [(1, 0, 2)]);
            http.assert_consumed();
        }
    }
}
