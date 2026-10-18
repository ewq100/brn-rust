//! Frozen guide identity and the existing semantic-cache key follow the actual
//! offered single tool, not merely a multi-tool owner approval policy.
use super::*;
use crate::mcp::http_tests::TestServer;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
#[tokio::test]
async fn chosen_catalogue_changes_identity_and_summary_key_but_same_definition_is_stable() {
    let server = TestServer::start().await;
    server.state.two_tools.store(true, Ordering::SeqCst);
    let (_root, request, _) = tests::fixture();
    let policy = server.policy(vec![McpAccess::General]).await;
    assert_eq!(policy.tools.len(), 2);
    let data = request.data_dir.clone();
    let chat = request.thread.clone();
    let mut store = Store::open(&data).unwrap();
    let op = store.allocate_operation(&unique_host_key()).unwrap();
    store
        .change_mcp_policy(
            &op,
            Some(&policy),
            &HostAuthority::owner("exact two-tool approval"),
        )
        .unwrap();
    drop(store);
    let mut identities = Vec::new();
    let mut keys = Vec::new();
    let mut definitions = Vec::new();
    for choice in ["public_read", "public_read_two", "public_read"] {
        let request = ThreadRunRequest {
            data_dir: data.clone(),
            thread: chat.clone(),
            prompt: "Same bounded owner objective".into(),
            request_message: None,
            queue_claim: None,
            owner_request: None,
            selection: Selection {
                provider: Provider::Chatgpt,
                model: "gpt-6.1-sol".into(),
            },
            effort: ReasoningEffort::Medium,
            max_tool_rounds: 3,
            auth_file: None,
            credentials_dir: None,
            sources: vec![],
            search: None,
            trace: TraceLevel::Summary,
        };
        let prepared = prepare_run(request)
            .unwrap()
            .with_external_input(
                choice,
                &json!({"query":"NEVER_PERSIST_THIS_REVIEWED_INPUT_IN_GUIDE"}),
            )
            .unwrap();
        let scope = prepared.scope.clone();
        {
            let store = scope.store.lock().unwrap();
            let RecordData::Run(run) = store.record(&scope.run).unwrap().unwrap().data else {
                panic!()
            };
            assert!(!run.guide_identity.contains("NEVER_PERSIST"));
            identities.push(run.guide_identity);
            keys.push(
                context(
                    &scope,
                    &store.records().unwrap(),
                    &store.receipts().unwrap(),
                    "same-synthetic-account",
                    ReasoningEffort::Medium,
                )
                .unwrap(),
            );
        }
        let model = MockCompletionModel::from_stream_turns([
            vec![
                MockStreamEvent::tool_call(
                    "local",
                    "retain_progress",
                    json!({"summary":"Frozen catalogue observed"}),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![
                MockStreamEvent::text("Still bounded."),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
        ]);
        let result = run_prepared_scripted(prepared, model.clone())
            .await
            .unwrap();
        assert!(
            matches!(result.answer.terminal, AiTerminal::Completed),
            "{:?}",
            result.answer.terminal
        );
        let requests = model.requests();
        assert_eq!(requests.len(), 2);
        let external: Vec<_> = requests[0]
            .tools
            .iter()
            .filter(|t| t.name.starts_with("mcp_"))
            .collect();
        assert_eq!(external.len(), 1);
        let approved = policy.tools.iter().find(|t| t.name == choice).unwrap();
        assert_eq!(
            external[0].name,
            crate::mcp::model_name(&server.endpoint, &approved.metadata_digest)
        );
        assert_eq!(external[0].parameters, mcp_input_schema(approved.contract));
        definitions.push(serde_json::to_value(external[0]).unwrap());
        assert_eq!(
            requests[0].system_instructions(),
            requests[1].system_instructions()
        );
        assert_eq!(
            &requests[1].chat_history[..requests[0].chat_history.len()],
            requests[0].chat_history.as_slice()
        );
        assert_eq!(
            serde_json::to_value(&requests[0].tools).unwrap(),
            serde_json::to_value(&requests[1].tools).unwrap()
        );
    }
    assert_ne!(definitions[0], definitions[1]);
    assert_eq!(definitions[0], definitions[2]);
    assert_ne!(
        identities[0], identities[1],
        "the actual single offered definition must participate in persistent identity"
    );
    assert_eq!(identities[0], identities[2]);
    assert_ne!(
        keys[0], keys[1],
        "existing semantic-cache key must follow the chosen frozen catalogue"
    );
    assert_eq!(keys[0], keys[2]);
    assert_eq!(server.state.calls.load(Ordering::SeqCst), 0);
    server.stop().await;
}
