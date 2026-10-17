//! Actual SDK HTTP + production prepared agent flow; all data is synthetic.
use super::*;
use crate::mcp::http_tests::TestServer;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};

fn configure(request: &ThreadRunRequest, policy: &McpPolicy) {
    let mut store = Store::open(&request.data_dir).unwrap();
    let op = store.allocate_operation(&unique_host_key()).unwrap();
    assert!(matches!(
        store
            .change_mcp_policy(
                &op,
                Some(policy),
                &HostAuthority::owner("synthetic explicit MCP approval")
            )
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
}
fn completed() -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::text("Checked the public fixture."),
        MockStreamEvent::final_response_with_total_tokens(2),
    ]
}
fn external_name(policy: &McpPolicy) -> String {
    crate::mcp::model_name(
        &crate::mcp::Endpoint::parse(&policy.endpoint).unwrap(),
        &policy.tools[0].metadata_digest,
    )
}

#[tokio::test]
async fn no_grant_omits_external_tools_and_local_work_does_not_connect() {
    let server = TestServer::start().await;
    let (_root, request, _) = tests::fixture();
    let policy = server.policy(vec![McpAccess::General]).await;
    configure(&request, &policy);
    let requests_before = server.state.requests.load(Ordering::SeqCst);
    let prepared = prepare_run(request).unwrap();
    assert!(prepared.scope.external.lock().unwrap().is_none());
    let model = MockCompletionModel::from_stream_turns([completed()]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    assert!(
        !model.requests()[0]
            .tools
            .iter()
            .any(|t| t.name.starts_with("mcp_"))
    );
    assert_eq!(server.state.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        server.state.requests.load(Ordering::SeqCst),
        requests_before,
        "unrelated run must not even discover/connect"
    );
    server.stop().await;
}

#[tokio::test]
async fn exact_reviewed_call_uses_real_sdk_and_keeps_prefix_and_raw_data_out_of_trace_and_backup() {
    let server = TestServer::start().await;
    let (root, mut request, _) = tests::fixture();
    request.trace = TraceLevel::Full;
    let data = request.data_dir.clone();
    let policy = server.policy(vec![McpAccess::General]).await;
    configure(&request, &policy);
    let name = external_name(&policy);
    let args = json!({"query":"REVIEWED_INPUT_CANARY_õ_public_fixture"});
    let expected =
        ExternalInputSnapshot::capture(&Store::open(&data).unwrap(), &request.thread).unwrap();
    let prepared = prepare_run(request)
        .unwrap()
        .with_external_input_at_snapshot("public_read", &args, &expected)
        .unwrap();
    let scope = prepared.scope.clone();
    let weak_invocation = Arc::downgrade(&mcp_runtime::invocation(&scope).unwrap());
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call("external", &name, args),
            MockStreamEvent::final_response_with_total_tokens(2),
        ],
        completed(),
    ]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer.terminal
    );
    assert_eq!(model.request_count(), 2);
    assert_eq!(server.state.calls.load(Ordering::SeqCst), 1);
    let requests = model.requests();
    assert!(requests[0].tools.iter().any(|t| t.name == name));
    assert_eq!(
        requests[0].system_instructions(),
        requests[1].system_instructions()
    );
    let first = requests[0].chat_history.iter().collect::<Vec<_>>();
    let second = requests[1].chat_history.iter().collect::<Vec<_>>();
    assert_eq!(&second[..first.len()], first.as_slice());
    let first_text = format!("{:?}", first);
    assert!(first_text.contains("REVIEWED_INPUT_CANARY"));
    let second_text = format!("{:?}", second);
    assert!(second_text.contains("fixture value 17"));
    assert!(second_text.contains("complete_tool_result_not_whole_source"));
    assert!(
        !requests[0]
            .system_instructions()
            .as_ref()
            .unwrap()
            .contains("REVIEWED_INPUT_CANARY")
    );
    let trace = serde_json::to_string(result.trace.as_ref().unwrap()).unwrap();
    assert!(!trace.contains("REVIEWED_INPUT_CANARY"), "{trace}");
    assert!(!trace.contains("fixture value 17"), "{trace}");
    assert!(trace.contains("source_identity"));
    {
        let store = scope.store.lock().unwrap();
        let canonical = serde_json::to_string(&store.records().unwrap()).unwrap();
        assert!(!canonical.contains("REVIEWED_INPUT_CANARY"));
        assert!(!canonical.contains("fixture value 17"));
        let backup = root.path().join("mcp-backup.sqlite3");
        store.backup(&backup).unwrap();
        let bytes = std::fs::read(backup).unwrap();
        assert!(
            !bytes
                .windows(b"REVIEWED_INPUT_CANARY".len())
                .any(|s| s == b"REVIEWED_INPUT_CANARY")
        );
    }
    drop(scope);
    assert!(
        weak_invocation.upgrade().is_none(),
        "after bounded cleanup, transient reviewed input must not remain owned by an Invocation/Connection cycle"
    );
    let reopened = Store::open(&data).unwrap();
    assert!(
        reopened
            .records()
            .unwrap()
            .iter()
            .any(|r| matches!(&r.data, RecordData::Settings(s) if s.mcp.as_ref()==Some(&policy)))
    );
    server.stop().await;
}

#[tokio::test]
async fn invalid_external_mixed_batches_reject_before_local_or_network_effect_and_do_not_consume_grant()
 {
    for bad in ["wrong_input", "unknown", "multiple"] {
        let server = TestServer::start().await;
        let (_root, request, _) = tests::fixture();
        let policy = server.policy(vec![McpAccess::General]).await;
        configure(&request, &policy);
        let name = external_name(&policy);
        let args = json!({"query":"approved public query"});
        let prepared = prepare_run(request)
            .unwrap()
            .with_external_input("public_read", &args)
            .unwrap();
        let scope = prepared.scope.clone();
        let mut events = vec![MockStreamEvent::tool_call(
            "local",
            "retain_progress",
            json!({"summary":"MUST_NOT_WRITE"}),
        )];
        events.push(MockStreamEvent::tool_call(
            "bad",
            if bad == "unknown" {
                "mcp_unknown"
            } else {
                &name
            },
            if bad == "wrong_input" {
                json!({"query":"PRIVATE_UNREVIEWED_CANARY"})
            } else {
                args.clone()
            },
        ));
        if bad == "multiple" {
            events.push(MockStreamEvent::tool_call("duplicate", &name, args));
        }
        events.push(MockStreamEvent::final_response_with_total_tokens(1));
        let model = MockCompletionModel::from_stream_turns([events]);
        let result = run_prepared_scripted(prepared, model.clone())
            .await
            .unwrap();
        assert!(
            matches!(result.answer.terminal,AiTerminal::Failed(ref e) if e.kind==AiErrorKind::InvalidToolUse),
            "{bad}: {:?}",
            result.answer.terminal
        );
        assert_eq!(model.request_count(), 1);
        assert_eq!(server.state.calls.load(Ordering::SeqCst), 0);
        {
            let store = scope.store.lock().unwrap();
            assert!(
                !serde_json::to_string(&store.record(&scope.run).unwrap())
                    .unwrap()
                    .contains("MUST_NOT_WRITE")
            );
            assert!(store.candidates().unwrap().is_empty());
        }
        server.stop().await;
    }
}

#[tokio::test]
async fn changed_server_metadata_and_revoked_grants_fail_before_provider_or_reviewed_input_dispatch()
 {
    for change in ["server_metadata", "disable", "stop", "project_move"] {
        let server = TestServer::start().await;
        let (_root, request, _) = tests::fixture();
        let policy = server.policy(vec![McpAccess::General]).await;
        configure(&request, &policy);
        let prepared = prepare_run(request)
            .unwrap()
            .with_external_input("public_read", &json!({"query":"PRIVATE_REVIEWED_CANARY"}))
            .unwrap();
        let scope = prepared.scope.clone();
        if change == "server_metadata" {
            server.state.metadata.store(1, Ordering::SeqCst);
        } else {
            let mut store = Store::open(&prepared.request.data_dir).unwrap();
            let op = store.allocate_operation(&unique_host_key()).unwrap();
            if change == "disable" {
                let mut disabled = policy.clone();
                disabled.enabled = false;
                store
                    .change_mcp_policy(&op, Some(&disabled), &HostAuthority::owner("revoke"))
                    .unwrap();
            } else if change == "project_move" {
                use sha2::Digest;
                let create = ProjectCommand::Create {
                    title: "Synthetic new private context".into(),
                };
                let binding = format!(
                    "{:x}",
                    sha2::Sha256::digest(serde_json::to_vec(&create).unwrap())
                );
                let created_op = store
                    .allocate_bound_operation(&unique_host_key(), &binding)
                    .unwrap();
                let created = store
                    .project_command(&created_op, &create, &HostAuthority::owner("context move"))
                    .unwrap();
                let project = created
                    .writes
                    .iter()
                    .find(|w| matches!(w.after.data, RecordData::Project(_)))
                    .unwrap()
                    .after
                    .id
                    .clone();
                let change = ProjectCommand::MoveChat {
                    chat: scope.chat.clone(),
                    project: Some(project),
                };
                let binding = format!(
                    "{:x}",
                    sha2::Sha256::digest(serde_json::to_vec(&change).unwrap())
                );
                let moved = store
                    .allocate_bound_operation(&unique_host_key(), &binding)
                    .unwrap();
                store
                    .project_command(&moved, &change, &HostAuthority::owner("context move"))
                    .unwrap();
            } else {
                store
                    .stop_and_post(
                        &op,
                        &scope.run,
                        &scope.chat,
                        "fresh direction",
                        &HostAuthority::owner("Stop"),
                    )
                    .unwrap();
            }
        }
        let model = MockCompletionModel::from_stream_turns([completed()]);
        let result = run_prepared_scripted(prepared, model.clone())
            .await
            .unwrap();
        assert!(
            matches!(&result.answer.terminal,AiTerminal::Failed(error) if error.mcp_unavailable_notice().is_some()),
            "{change}: {:?}",
            result.answer.terminal
        );
        assert_eq!(model.request_count(), 0);
        assert_eq!(server.state.calls.load(Ordering::SeqCst), 0);
        assert!(result.answer.text.is_empty());
        assert!(scope.store.lock().unwrap().candidates().unwrap().is_empty());
        server.stop().await;
    }
}

#[tokio::test]
async fn repeated_external_call_is_not_an_implicit_second_owner_authorization() {
    let server = TestServer::start().await;
    let (_root, request, _) = tests::fixture();
    let policy = server.policy(vec![McpAccess::General]).await;
    configure(&request, &policy);
    let name = external_name(&policy);
    let args = json!({"query":"one public request"});
    let prepared = prepare_run(request)
        .unwrap()
        .with_external_input("public_read", &args)
        .unwrap();
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call("one", &name, args.clone()),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call("two", &name, args),
            MockStreamEvent::tool_call(
                "local",
                "retain_progress",
                json!({"summary":"SECOND_BATCH_MUST_NOT_APPLY"}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]);
    let scope = prepared.scope.clone();
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(
        matches!(result.answer.terminal,AiTerminal::Failed(ref e) if e.kind==AiErrorKind::InvalidToolUse)
    );
    assert_eq!(server.state.calls.load(Ordering::SeqCst), 1);
    assert_eq!(model.request_count(), 2);
    assert!(
        !serde_json::to_string(&scope.store.lock().unwrap().record(&scope.run).unwrap())
            .unwrap()
            .contains("SECOND_BATCH_MUST_NOT_APPLY")
    );
    server.stop().await;
}

#[tokio::test]
async fn actual_inflight_remote_work_may_finish_but_stop_or_revoke_fences_late_acceptance_and_writes()
 {
    for action in ["stop", "disable"] {
        let server = TestServer::start().await;
        let (_root, request, note) = tests::fixture();
        let data = request.data_dir.clone();
        let chat = request.thread.clone();
        let policy = server.policy(vec![McpAccess::General]).await;
        configure(&request, &policy);
        let name = external_name(&policy);
        let args = json!({"query":"blocked"});
        let prepared = prepare_run(request)
            .unwrap()
            .with_external_input("public_read", &args)
            .unwrap();
        let run_id = prepared.run_id().to_owned();
        let model = MockCompletionModel::from_stream_turns([
            vec![
                MockStreamEvent::tool_call("remote", &name, args),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![
                MockStreamEvent::tool_call(
                    "late",
                    "retain_progress",
                    json!({"summary":"MUST_NOT_RUN_AFTER_REVOCATION"}),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
        ]);
        let sent = model.clone();
        let task = tokio::spawn(async move { run_prepared_scripted(prepared, sent).await });
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            server.state.started.notified(),
        )
        .await
        .unwrap();
        let mut store = Store::open(&data).unwrap();
        let op = store.allocate_operation(&unique_host_key()).unwrap();
        if action == "stop" {
            store
                .stop_and_post(
                    &op,
                    &run_id,
                    &chat,
                    "New owner direction",
                    &HostAuthority::owner("Stop"),
                )
                .unwrap();
        } else {
            let mut disabled = policy.clone();
            disabled.enabled = false;
            store
                .change_mcp_policy(&op, Some(&disabled), &HostAuthority::owner("revoke"))
                .unwrap();
        }
        let after_revoke = store.records().unwrap();
        let original = store.record(&note).unwrap();
        server.state.release.notify_one();
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            matches!(result.answer.terminal, AiTerminal::Interrupted),
            "{action}: {:?}",
            result.answer.terminal
        );
        assert_eq!(
            model.request_count(),
            1,
            "no next provider request after the revoked result"
        );
        assert_eq!(server.state.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            server.state.completed.load(Ordering::SeqCst),
            1,
            "revocation does not pretend remote Undo"
        );
        assert_eq!(store.records().unwrap(), after_revoke);
        assert_eq!(store.record(&note).unwrap(), original);
        assert!(store.candidates().unwrap().is_empty());
        assert!(
            result.terminal_run.is_none(),
            "old callback cannot acknowledge this terminal transition"
        );
        server.stop().await;
    }
}

#[tokio::test]
async fn malicious_server_result_never_exposes_owner_management_or_authorizes_its_batch() {
    let server = TestServer::start().await;
    let (_root, request, note) = tests::fixture();
    let policy = server.policy(vec![McpAccess::General]).await;
    configure(&request, &policy);
    let name = external_name(&policy);
    let args = json!({"query":"malicious"});
    let prepared = prepare_run(request)
        .unwrap()
        .with_external_input("public_read", &args)
        .unwrap();
    let scope = prepared.scope.clone();
    let original = scope.store.lock().unwrap().record(&note).unwrap();
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call("remote", &name, args),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call(
                "owner",
                "configure_mcp",
                json!({"enabled":true,"access":["general"]}),
            ),
            MockStreamEvent::tool_call(
                "effect",
                "retain_progress",
                json!({"summary":"MUST_NOT_BECOME_AN_OWNER_GRANT"}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(
        matches!(result.answer.terminal,AiTerminal::Failed(ref e) if e.kind==AiErrorKind::InvalidToolUse)
    );
    assert_eq!(model.request_count(), 2);
    assert_eq!(server.state.calls.load(Ordering::SeqCst), 1);
    let requests = model.requests();
    assert!(format!("{:?}", requests[1].chat_history).contains("UNTRUSTED_REMOTE_BODY_CANARY"));
    assert!(
        !requests[1]
            .system_instructions()
            .unwrap()
            .contains("UNTRUSTED_REMOTE_BODY_CANARY")
    );
    for request in &requests {
        assert!(!request.tools.iter().any(|t| matches!(
            t.name.as_str(),
            "configure_mcp" | "mcp-configure" | "skill-install" | "install_skill"
        )));
    }
    {
        let store = scope.store.lock().unwrap();
        assert_eq!(store.record(&note).unwrap(), original);
        assert!(
            store.records().unwrap().iter().any(
                |r| matches!(&r.data,RecordData::Settings(s) if s.mcp.as_ref()==Some(&policy))
            )
        );
        let canonical = serde_json::to_string(&store.records().unwrap()).unwrap();
        assert!(!canonical.contains("MUST_NOT_BECOME_AN_OWNER_GRANT"));
        assert!(!canonical.contains("UNTRUSTED_REMOTE_BODY_CANARY"));
        assert!(store.candidates().unwrap().is_empty());
    }
    server.stop().await;
}

#[tokio::test]
async fn actual_lazy_skill_delivery_advances_fence_and_never_renews_external_grant() {
    let server = TestServer::start().await;
    let (_root, request, _) = tests::fixture();
    let policy = server.policy(vec![McpAccess::General]).await;
    configure(&request, &policy);
    let name = external_name(&policy);
    let args = json!({"query":"previously reviewed public query"});
    let prepared = prepare_run(request)
        .unwrap()
        .with_external_input("public_read", &args)
        .unwrap();
    let scope = prepared.scope.clone();
    let invocation = mcp_runtime::invocation(&scope).unwrap();
    let fence = scope.fence.load(Ordering::SeqCst);
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call("guide", "read_skill", json!({"id":"maintain-notes"})),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call("external", &name, args),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Failed(_)));
    assert_eq!(
        model.request_count(),
        2,
        "local guide was delivered before stale external call"
    );
    assert!(format!("{:?}", model.requests()[1].chat_history).contains("maintain-notes"));
    assert!(scope.fence.load(Ordering::SeqCst) > fence);
    assert_eq!(
        server.state.calls.load(Ordering::SeqCst),
        0,
        "no query after a control-fence change"
    );
    assert!(
        crate::mcp::http::DispatchGate::channel(invocation.as_ref()).is_err(),
        "no implicit grant renewal"
    );
    {
        let store = scope.store.lock().unwrap();
        let RecordData::Run(run) = store.record(&scope.run).unwrap().unwrap().data else {
            panic!()
        };
        assert!(
            run.loaded_skill_details
                .iter()
                .any(|d| d.id == "maintain-notes")
        );
        assert!(store.candidates().unwrap().is_empty());
    }
    server.stop().await;
}

#[tokio::test]
async fn exact_project_approval_is_usable_in_its_actual_checked_context() {
    use sha2::Digest;
    let server = TestServer::start().await;
    let (_root, request, _) = tests::fixture();
    let project = {
        let mut store = Store::open(&request.data_dir).unwrap();
        let create = ProjectCommand::Create {
            title: "MCP scoped synthetic Project".into(),
        };
        let binding = format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(&create).unwrap())
        );
        let op = store
            .allocate_bound_operation(&unique_host_key(), &binding)
            .unwrap();
        let receipt = store
            .project_command(&op, &create, &HostAuthority::owner("scope fixture"))
            .unwrap();
        let project = receipt
            .writes
            .into_iter()
            .find(|w| matches!(w.after.data, RecordData::Project(_)))
            .unwrap()
            .after
            .id;
        let change = ProjectCommand::MoveChat {
            chat: request.thread.clone(),
            project: Some(project.clone()),
        };
        let binding = format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(&change).unwrap())
        );
        let op = store
            .allocate_bound_operation(&unique_host_key(), &binding)
            .unwrap();
        store
            .project_command(&op, &change, &HostAuthority::owner("scope fixture"))
            .unwrap();
        project
    };
    let policy = server.policy(vec![McpAccess::Project(project)]).await;
    configure(&request, &policy);
    let name = external_name(&policy);
    let input = json!({"query":"explicitly reviewed Project query"});
    let prepared = prepare_run(request)
        .unwrap()
        .with_external_input("public_read", &input)
        .unwrap();
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call("scoped", &name, input),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        completed(),
    ]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    assert_eq!(model.request_count(), 2);
    assert_eq!(server.state.calls.load(Ordering::SeqCst), 1);
    assert!(format!("{:?}", model.requests()[1].chat_history).contains("fixture value 17"));
    server.stop().await;
}

#[tokio::test]
async fn saved_review_refusal_ends_only_its_preparation_and_preserves_queued_instruction() {
    use sha2::{Digest, Sha256};
    for changed_before_prepare in [true, false] {
        let server = TestServer::start().await;
        let (_root, mut request, _) = tests::fixture();
        let data = request.data_dir.clone();
        let policy = server.policy(vec![McpAccess::General]).await;
        configure(&request, &policy);
        let expected =
            ExternalInputSnapshot::capture(&Store::open(&data).unwrap(), &request.thread).unwrap();
        let mut changed = policy.clone();
        changed.server_digest = if policy.server_digest == "a".repeat(64) {
            "b".repeat(64)
        } else {
            "a".repeat(64)
        };
        if changed_before_prepare {
            configure(&request, &changed);
        }
        let mut store = Store::open(&data).unwrap();
        let mut control = |key: &str, command: ChatCommand| {
            let hash = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&command).unwrap())
            );
            let op = store.allocate_bound_operation(key, &hash).unwrap();
            store
                .chat_command(
                    &op,
                    &command,
                    &HostAuthority::owner("synthetic queued review"),
                )
                .unwrap()
        };
        control(
            "review-enqueue",
            ChatCommand::Enqueue {
                thread: request.thread.clone(),
                text: request.prompt.clone(),
            },
        );
        let queued = control("review-dispatch", ChatCommand::Dispatch)
            .dispatched
            .unwrap();
        request.request_message = queued.message.clone();
        request.queue_claim = queued.dispatch.clone();
        let prompt = request.prompt.clone();
        let prepared = prepare_run(request).unwrap();
        let run_id = prepared.run_id().to_owned();
        if !changed_before_prepare {
            let op = store.allocate_operation(&unique_host_key()).unwrap();
            store
                .change_mcp_policy(
                    &op,
                    Some(&changed),
                    &HostAuthority::owner("synthetic post-prepare policy change"),
                )
                .unwrap();
        }
        let before = store.record(&run_id).unwrap().unwrap();
        assert!(
            matches!(&before.data, RecordData::Run(r) if r.state == if changed_before_prepare { RunState::Working } else { RunState::Interrupted })
        );
        let control_before = serde_json::to_value(store.chat_control().unwrap()).unwrap();
        let requests_before = server.state.requests.load(Ordering::SeqCst);
        let model = MockCompletionModel::from_stream_turns([completed()]);
        let error = match prepared.with_external_input_at_snapshot(
            "public_read",
            &json!({"query":"QUEUED_REVIEW_INPUT_CANARY"}),
            &expected,
        ) {
            Ok(_) => panic!("stale review must not bind fresh authority"),
            Err(error) => error,
        };
        if changed_before_prepare {
            assert_eq!(error.kind, AiErrorKind::Other);
            assert!(error.to_string().starts_with("The approved MCP connection"));
        } else {
            assert_eq!(
                error.kind,
                AiErrorKind::ToolRejected,
                "existing post-prepare liveness refusal keeps its precedence"
            );
        }
        assert_eq!(model.request_count(), 0);
        assert_eq!(
            server.state.requests.load(Ordering::SeqCst),
            requests_before
        );
        assert_eq!(server.state.calls.load(Ordering::SeqCst), 0);
        let after = store.record(&run_id).unwrap().unwrap();
        let (RecordData::Run(before), RecordData::Run(after)) = (before.data, after.data) else {
            panic!("Run records")
        };
        assert_eq!(after.guide_identity, before.guide_identity);
        assert_eq!(
            after.fence,
            before.fence + u64::from(changed_before_prepare)
        );
        assert_eq!(
            after.state,
            if changed_before_prepare {
                RunState::Failed
            } else {
                RunState::Interrupted
            }
        );
        assert_eq!(after.instruction, queued.message);
        assert_eq!(
            serde_json::to_value(store.chat_control().unwrap()).unwrap(),
            control_before,
            "attachment neither acknowledges nor unpauses the queue"
        );
        assert_eq!(
            store
                .records()
                .unwrap()
                .iter()
                .filter(|r| matches!(&r.data, RecordData::Run(_)))
                .count(),
            1,
            "one durable prepare"
        );
        let canonical = serde_json::to_string(&store.records().unwrap()).unwrap();
        assert!(canonical.contains(&prompt));
        assert!(!canonical.contains("QUEUED_REVIEW_INPUT_CANARY"));
        assert!(!crate::run_lease::run_is_live(
            &data.join("derived"),
            &run_id
        ));
        drop(crate::run_lease::RunLease::acquire(&data.join("derived"), &run_id).unwrap());
        ATTACHMENT_FINISH_NOTICES.with(|notices| {
            assert!(
                notices.borrow().is_empty(),
                "no secondary notice for successful or already-fenced finish"
            );
        });
        server.stop().await;
    }
}

std::thread_local! {
    static FAIL_NEXT_ATTACHMENT_FINISH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    pub(super) static ATTACHMENT_FINISH_NOTICES: std::cell::RefCell<Vec<&'static str>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// One test-local storage failure after a genuine refusal; production finish
/// still executes and encounters the actually poisoned Store mutex.
pub(super) fn before_attachment_finish(scope: &Scope) {
    if FAIL_NEXT_ATTACHMENT_FINISH.with(|fault| fault.replace(false)) {
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _store = scope.store.lock().unwrap();
            panic!("synthetic terminal persistence failure");
        }));
        assert!(poisoned.is_err());
    }
}

#[tokio::test]
async fn attachment_refusal_survives_real_finish_storage_failure_without_claiming_terminalization()
{
    use sha2::{Digest, Sha256};
    let server = TestServer::start().await;
    let (_root, mut request, _) = tests::fixture();
    let policy = server.policy(vec![McpAccess::General]).await;
    configure(&request, &policy);
    let data = request.data_dir.clone();
    let mut store = Store::open(&data).unwrap();
    let mut control = |key: &str, command: ChatCommand| {
        let hash = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&command).unwrap())
        );
        let operation = store.allocate_bound_operation(key, &hash).unwrap();
        store
            .chat_command(
                &operation,
                &command,
                &HostAuthority::owner("synthetic queued review"),
            )
            .unwrap()
    };
    control(
        "finish-fault-enqueue",
        ChatCommand::Enqueue {
            thread: request.thread.clone(),
            text: request.prompt.clone(),
        },
    );
    let queued = control("finish-fault-dispatch", ChatCommand::Dispatch)
        .dispatched
        .unwrap();
    request.request_message = queued.message.clone();
    request.queue_claim = queued.dispatch.clone();
    let prepared = prepare_run(request).unwrap();
    let run_id = prepared.run_id().to_owned();
    let weak_scope = Arc::downgrade(&prepared.scope);
    let before = store.record(&run_id).unwrap().unwrap();
    let queue_before = serde_json::to_value(store.chat_control().unwrap()).unwrap();
    let requests_before = server.state.requests.load(Ordering::SeqCst);
    let model = MockCompletionModel::from_stream_turns([completed()]);
    ATTACHMENT_FINISH_NOTICES.with(|notices| assert!(notices.borrow().is_empty()));
    FAIL_NEXT_ATTACHMENT_FINISH.with(|fault| fault.set(true));
    let error = match prepared.with_external_input(
        "not_approved",
        &json!({"query":"FINISH_FAILURE_RAW_INPUT_CANARY"}),
    ) {
        Ok(_) => panic!("unapproved tool must be refused"),
        Err(error) => error,
    };
    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        serde_json::to_value(mcp_runtime::unavailable()).unwrap(),
        "secondary storage failure must not replace any primary error field"
    );
    ATTACHMENT_FINISH_NOTICES.with(|notices| {
        let notices = std::mem::take(&mut *notices.borrow_mut());
        assert_eq!(notices, vec!["BRN could not record this preparation's terminal state; inspect its saved Run before continuing."]);
        assert!(!notices[0].contains("FINISH_FAILURE_RAW_INPUT_CANARY"));
    });
    assert!(weak_scope.upgrade().is_none(), "owning handles are dropped");
    assert!(!crate::run_lease::run_is_live(
        &data.join("derived"),
        &run_id
    ));
    drop(crate::run_lease::RunLease::acquire(&data.join("derived"), &run_id).unwrap());
    assert_eq!(
        store.record(&run_id).unwrap().unwrap(),
        before,
        "failed persistence is not a terminalization claim"
    );
    assert!(
        matches!(before.data, RecordData::Run(r) if r.state == RunState::Working && r.instruction == queued.message)
    );
    assert_eq!(
        serde_json::to_value(store.chat_control().unwrap()).unwrap(),
        queue_before,
        "no acknowledgement, unpause or replacement Run"
    );
    assert_eq!(
        store
            .records()
            .unwrap()
            .iter()
            .filter(|r| matches!(r.data, RecordData::Run(_)))
            .count(),
        1
    );
    assert!(
        !serde_json::to_string(&store.records().unwrap())
            .unwrap()
            .contains("FINISH_FAILURE_RAW_INPUT_CANARY")
    );
    assert_eq!(model.request_count(), 0);
    assert_eq!(
        server.state.requests.load(Ordering::SeqCst),
        requests_before
    );
    assert_eq!(server.state.calls.load(Ordering::SeqCst), 0);
    server.stop().await;
}
