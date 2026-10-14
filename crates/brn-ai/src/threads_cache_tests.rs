use super::*;

fn sse(events: &[Value]) -> String {
    let mut text = String::new();
    for event in events {
        text.push_str(&format!("data: {event}\n\n"));
    }
    text.push_str("data: [DONE]\n\n");
    text
}

fn tool_turn(n: u32, response: &str) -> String {
    let reasoning = json!({"type":"reasoning","id":format!("rs_{n}"),"summary":[],"encrypted_content":format!("ENC{n}")});
    let call = json!({"type":"function_call","id":format!("fc_{n}"),"call_id":format!("call_{n}"),"name":"retain_progress","arguments":format!("{{\"summary\":\"step {n}\"}}"),"status":"completed"});
    sse(&[
        json!({"type":"response.output_item.done","output_index":0,"sequence_number":1,"item":reasoning}),
        json!({"type":"response.output_item.done","output_index":1,"sequence_number":2,"item":call}),
        json!({"type":"response.completed","sequence_number":3,"response":{
            "id":response,"object":"response","created_at":1,"status":"completed","model":"gpt-6.1-sol",
            "output":[reasoning,call],"tools":[],
            "usage":{"input_tokens":10,"output_tokens":1,"total_tokens":11,"input_tokens_details":{"cached_tokens":0}}}}),
    ])
}

fn final_turn() -> String {
    sse(&[
        json!({"type":"response.completed","sequence_number":1,"response":{
        "id":"resp_last","object":"response","created_at":1,"status":"completed","model":"gpt-6.1-sol",
        "output":[{"type":"message","id":"msg_f","status":"completed","role":"assistant","content":[{"type":"output_text","annotations":[],"text":"Done"}]}],
        "tools":[],"usage":{"input_tokens":10,"output_tokens":1,"total_tokens":11}}}),
    ])
}

async fn three_turn_run(
    scope: &Arc<Scope>,
    input: &str,
    pin: bool,
) -> Vec<rig::test_utils::CapturedHttpRequest> {
    let http = rig::test_utils::SequencedHttpClient::new([
        rig::test_utils::MockHttpResponse::success_typed(tool_turn(1, "r1"), "text/event-stream"),
        rig::test_utils::MockHttpResponse::success_typed(tool_turn(2, "r2"), "text/event-stream"),
        rig::test_utils::MockHttpResponse::success_typed(final_turn(), "text/event-stream"),
    ]);
    let transport = rig::http_client::DynHttpClient::new(http.clone());
    let transport = match (&scope.cache_key, pin) {
        (Some(key), true) => crate::auth::pinned_session_http(transport, key).unwrap(),
        _ => transport,
    };
    let client = rig::providers::openai::OpenAIConfig::with_key(
        &rig::providers::chatgpt::DIALECT,
        "synthetic-not-a-credential",
    )
    .connect(transport);
    let answer = run_model(
        client.responses("gpt-6.1-sol"),
        scope.clone(),
        input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    assert!(
        matches!(answer.terminal, AiTerminal::Completed),
        "{:?}",
        answer.terminal
    );
    http.requests()
}

fn body(request: &rig::test_utils::CapturedHttpRequest) -> Value {
    serde_json::from_slice(&request.body).unwrap()
}

#[tokio::test]
async fn codex_turns_share_one_cache_key_and_an_append_only_prefix() {
    let (_root, request, _) = tests::fixture();
    let (scope, input) = begin(&request).unwrap();
    let requests = three_turn_run(&scope, &input, true).await;
    assert_eq!(requests.len(), 3);
    let key = scope.cache_key.clone().unwrap();
    let bodies: Vec<Value> = requests.iter().map(body).collect();
    for b in &bodies {
        assert_eq!(b["prompt_cache_key"], key.as_str());
        assert_eq!(b["store"], false);
        assert_eq!(b["instructions"], bodies[0]["instructions"]);
        assert_eq!(b["tools"], bodies[0]["tools"]);
        assert_eq!(b["reasoning"], bodies[0]["reasoning"]);
    }
    // Each turn resends the earlier input byte for byte, then appends.
    for pair in bodies.windows(2) {
        let (before, after) = (
            pair[0]["input"].as_array().unwrap(),
            pair[1]["input"].as_array().unwrap(),
        );
        assert!(after.len() > before.len());
        assert_eq!(&after[..before.len()], &before[..]);
    }
    let sessions: Vec<_> = requests
        .iter()
        .map(|r| {
            r.headers
                .get("session_id")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert!(sessions.iter().all(|s| *s == key), "{sessions:?}");
}

#[tokio::test]
async fn without_the_pin_rig_draws_a_new_session_per_request() {
    let (_root, request, _) = tests::fixture();
    let (scope, input) = begin(&request).unwrap();
    let requests = three_turn_run(&scope, &input, false).await;
    let sessions: std::collections::BTreeSet<_> = requests
        .iter()
        .map(|r| {
            r.headers
                .get("session_id")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(
        sessions.len(),
        3,
        "this documents the Rig behaviour the pin replaces"
    );
}

#[test]
fn cache_keys_are_per_chat_bounded_header_safe_and_codex_only() {
    let key = |chat: &str| cache_key(Provider::Chatgpt, chat).unwrap();
    assert_eq!(key("abc:0"), key("abc:0"));
    assert_ne!(key("abc:0"), key("abd:0"));
    let long = key(&"x\n \u{e9}/".repeat(40));
    assert!(long.len() <= 64 && long.starts_with("brn-"), "{long}");
    assert!(long.parse::<rig::http_client::HeaderValue>().is_ok());
    assert_eq!(cache_key(Provider::Copilot, "abc:0"), None);
}
