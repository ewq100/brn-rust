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
            "usage":{"input_tokens":1000 * u64::from(n),"output_tokens":1,"total_tokens":11,"input_tokens_details":{"cached_tokens":128 * u64::from(n - 1)}}}}),
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

fn script() -> rig::test_utils::SequencedHttpClient {
    rig::test_utils::SequencedHttpClient::new([
        rig::test_utils::MockHttpResponse::success_typed(tool_turn(1, "r1"), "text/event-stream"),
        rig::test_utils::MockHttpResponse::success_typed(tool_turn(2, "r2"), "text/event-stream"),
        rig::test_utils::MockHttpResponse::success_typed(final_turn(), "text/event-stream"),
    ])
}

/// A synthetic Codex auth file, as `codex_client` accepts it.
fn codex_file(root: &tempfile::TempDir) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let file = root.path().join("auth.json");
    std::fs::write(
        &file,
        r#"{"auth_mode":"chatgpt","tokens":{"access_token":"synthetic-not-a-credential","account_id":"synthetic"}}"#,
    )
    .unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    file
}

/// A synthetic OAuth cache in a fresh credentials folder.
fn credentials_dir(root: &tempfile::TempDir) -> PathBuf {
    use base64::Engine;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let dir = root.path().join("credentials");
    std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&json!({"email":"synthetic@example.invalid"})).unwrap());
    let file = dir.join("chatgpt.json");
    std::fs::write(
        &file,
        serde_json::to_vec(&json!({
            "access_token":"synthetic-not-a-credential","expires_at":4102444800_i64,
            "account_id":"synthetic","id_token":format!("e30.{claims}.synthetic")}))
        .unwrap(),
    )
    .unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    dir
}

/// Stores the credential route in the workspace Settings, which `prepare_run`
/// requires the request to match.
fn configure(request: &ThreadRunRequest, provider: &str) {
    let mut store = Store::open(&request.data_dir).unwrap();
    let RecordData::Settings(mut settings) = store
        .records()
        .unwrap()
        .into_iter()
        .find_map(|r| matches!(r.data, RecordData::Settings(_)).then_some(r.data))
        .unwrap()
    else {
        panic!()
    };
    settings.provider = provider.into();
    settings.auth_file = request
        .auth_file
        .as_ref()
        .map(|p| p.to_str().unwrap().into());
    settings.credentials_dir = request
        .credentials_dir
        .as_ref()
        .map(|p| p.to_str().unwrap().into());
    let op = store
        .allocate_operation("synthetic credential route")
        .unwrap();
    store
        .configure_workspace(&op, &settings, &HostAuthority::owner("fixture"))
        .unwrap();
}

enum Route {
    CodexFile,
    Credentials,
}

/// A three-turn run through the production path: `prepare_run` chooses the
/// key, `provider_client` builds the client over the mock transport, and
/// `run_model` sends the requests.
async fn production_run(
    route: Route,
    trace: TraceLevel,
) -> (
    String,
    Vec<rig::test_utils::CapturedHttpRequest>,
    ThreadResult,
) {
    let (root, mut request, _) = tests::fixture();
    match route {
        Route::CodexFile => request.auth_file = Some(codex_file(&root)),
        Route::Credentials => request.credentials_dir = Some(credentials_dir(&root)),
    }
    configure(&request, "Chatgpt");
    request.trace = trace;
    request.max_tool_rounds = 4;
    let thread = request.thread.clone();
    let http = script();
    let result = run_prepared_over(
        prepare_run(request).unwrap(),
        CancellationToken::new(),
        Arc::new(|_| {}),
        Some(DynHttpClient::new(http.clone())),
    )
    .await
    .unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer.terminal
    );
    (thread, http.requests(), result)
}

fn session_ids(requests: &[rig::test_utils::CapturedHttpRequest]) -> Vec<String> {
    requests
        .iter()
        .map(|r| {
            r.headers
                .get("session_id")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

fn body(request: &rig::test_utils::CapturedHttpRequest) -> Value {
    serde_json::from_slice(&request.body).unwrap()
}

#[tokio::test]
async fn codex_runs_pin_one_session_and_an_append_only_prefix_on_either_credential_route() {
    for route in [Route::CodexFile, Route::Credentials] {
        let (thread, requests, _) = production_run(route, TraceLevel::Off).await;
        assert_eq!(requests.len(), 3, "the injected transport must be kept");
        let key = cache_key(Provider::Chatgpt, &thread).unwrap();
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
        let sessions = session_ids(&requests);
        assert!(sessions.iter().all(|s| *s == key), "{sessions:?}");
    }
}

#[tokio::test]
async fn different_chats_get_different_session_ids() {
    let (a, first, _) = production_run(Route::CodexFile, TraceLevel::Off).await;
    let (b, second, _) = production_run(Route::CodexFile, TraceLevel::Off).await;
    assert_ne!(a, b, "each fixture is its own chat");
    assert_ne!(session_ids(&first)[0], session_ids(&second)[0]);
}

#[tokio::test]
async fn without_the_pin_rig_draws_a_new_session_per_request() {
    let (_root, request, _) = tests::fixture();
    let (scope, input) = begin(&request).unwrap();
    let http = script();
    let client = rig::providers::openai::OpenAIConfig::with_key(
        &rig::providers::chatgpt::DIALECT,
        "synthetic-not-a-credential",
    )
    .connect(DynHttpClient::new(http.clone()));
    run_model(
        client.responses("gpt-6.1-sol"),
        scope,
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    let distinct: std::collections::BTreeSet<_> =
        session_ids(&http.requests()).into_iter().collect();
    assert_eq!(
        distinct.len(),
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

#[tokio::test]
async fn copilot_runs_carry_no_cache_key_in_scope_or_body() {
    let (_root, mut request, _) = tests::fixture();
    request.selection.provider = Provider::Copilot;
    configure(&request, "Copilot");
    let (scope, input) = begin(&request).unwrap();
    assert_eq!(scope.cache_key, None);
    let http = script();
    let client = rig::providers::openai::OpenAIConfig::with_key(
        &rig::providers::chatgpt::DIALECT,
        "synthetic-not-a-credential",
    )
    .connect(DynHttpClient::new(http.clone()));
    run_model(
        client.responses("gpt-6.1-sol"),
        scope,
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    let requests = http.requests();
    assert_eq!(requests.len(), 3);
    assert!(
        requests
            .iter()
            .all(|r| body(r).get("prompt_cache_key").is_none())
    );
}

#[tokio::test]
async fn trace_records_each_turns_cached_input() {
    let (_thread, _requests, result) = production_run(Route::CodexFile, TraceLevel::Summary).await;
    let trace = result.trace.unwrap();
    let turns: Vec<_> = trace
        .turn_usage
        .iter()
        .map(|u| (u.input, u.cached_input))
        .collect();
    assert_eq!(turns, [(1000, 0), (2000, 128), (10, 0)]);
    let total = trace.usage.unwrap();
    assert_eq!((total.input, total.cached_input), (3010, 128));
}

/// A Codex-file run over `replies`, whatever its terminal.
async fn scripted_run(
    replies: Vec<rig::test_utils::MockHttpResponse>,
) -> (Vec<rig::test_utils::CapturedHttpRequest>, ThreadResult) {
    scripted_run_at(replies, TraceLevel::Summary).await
}

async fn scripted_run_at(
    replies: Vec<rig::test_utils::MockHttpResponse>,
    trace: TraceLevel,
) -> (Vec<rig::test_utils::CapturedHttpRequest>, ThreadResult) {
    let (root, mut request, _) = tests::fixture();
    request.auth_file = Some(codex_file(&root));
    configure(&request, "Chatgpt");
    request.trace = trace;
    request.max_tool_rounds = 4;
    let http = rig::test_utils::SequencedHttpClient::new(replies);
    let result = run_prepared_over(
        prepare_run(request).unwrap(),
        CancellationToken::new(),
        Arc::new(|_| {}),
        Some(DynHttpClient::new(http.clone())),
    )
    .await
    .unwrap();
    (http.requests(), result)
}

fn streamed(body: String) -> rig::test_utils::MockHttpResponse {
    rig::test_utils::MockHttpResponse::success_typed(body, "text/event-stream")
}

fn unavailable() -> rig::test_utils::MockHttpResponse {
    rig::test_utils::MockHttpResponse::error(
        rig::http_client::StatusCode::SERVICE_UNAVAILABLE,
        r#"{"error":{"type":"server_error","message":"SYNTHETIC_SECRET"}}"#,
    )
}

#[tokio::test]
async fn a_transient_status_mid_run_is_resent_identically_once_and_traced() {
    let (requests, result) = scripted_run(vec![
        streamed(tool_turn(1, "r1")),
        unavailable(),
        streamed(tool_turn(2, "r2")),
        streamed(final_turn()),
    ])
    .await;
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer.terminal
    );
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[1].body, requests[2].body);
    assert_eq!(
        session_ids(&requests[1..3])[0],
        session_ids(&requests[1..3])[1]
    );
    assert_eq!(result.provider_resends.len(), 1);
    let trace = result.trace.unwrap();
    assert_eq!(trace.provider_retries, result.provider_resends);
    assert_eq!(trace.provider_retries[0].status, Some(503));
    assert!(trace.failure.is_none());
    assert!(
        !serde_json::to_string(&trace)
            .unwrap()
            .contains("SYNTHETIC_SECRET")
    );
}

#[tokio::test]
async fn a_repeated_transient_failure_ends_the_run_with_its_safe_detail_in_the_trace() {
    let (requests, result) = scripted_run(vec![
        streamed(tool_turn(1, "r1")),
        unavailable(),
        unavailable(),
        streamed(final_turn()),
    ])
    .await;
    assert_eq!(requests.len(), 3);
    let AiTerminal::Failed(error) = &result.answer.terminal else {
        panic!("{:?}", result.answer.terminal)
    };
    let detail = error.detail.as_ref().unwrap();
    assert_eq!(
        (
            detail.origin.as_str(),
            detail.status,
            detail.error_type.as_deref()
        ),
        ("http_status", Some(503), Some("server_error"))
    );
    let trace = result.trace.unwrap();
    assert_eq!(trace.provider_retries.len(), 1);
    assert_eq!(trace.failure.as_ref(), Some(error));
    let json = serde_json::to_string(&trace).unwrap();
    assert!(json.contains("\"failure\"") && !json.contains("SYNTHETIC_SECRET"));
}

#[tokio::test]
async fn a_failed_turn_after_the_reply_started_is_not_resent() {
    let failed = sse(&[
        json!({"type":"response.failed","sequence_number":1,"response":{
        "error":{"code":"server_error","message":"SYNTHETIC_SECRET"}}}),
    ]);
    let (requests, result) = scripted_run(vec![
        streamed(tool_turn(1, "r1")),
        streamed(failed),
        streamed(final_turn()),
    ])
    .await;
    assert_eq!(requests.len(), 2);
    let AiTerminal::Failed(error) = &result.answer.terminal else {
        panic!("{:?}", result.answer.terminal)
    };
    let detail = error.detail.as_ref().unwrap();
    assert_eq!(
        (detail.origin.as_str(), detail.code.as_deref()),
        ("response_failed", Some("server_error"))
    );
    assert!(result.trace.unwrap().provider_retries.is_empty());
}

#[tokio::test]
async fn re_sends_are_reported_without_a_trace() {
    let (requests, result) = scripted_run_at(
        vec![
            streamed(tool_turn(1, "r1")),
            unavailable(),
            streamed(tool_turn(2, "r2")),
            streamed(final_turn()),
        ],
        TraceLevel::Off,
    )
    .await;
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    assert_eq!(requests.len(), 4);
    assert!(result.trace.is_none());
    assert_eq!(result.provider_resends.len(), 1);
    assert_eq!(result.provider_resends[0].status, Some(503));
}
