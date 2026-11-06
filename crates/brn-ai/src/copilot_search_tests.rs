use super::*;
use rig::test_utils::{MockHttpResponse, SequencedHttpClient};
use std::os::unix::fs::PermissionsExt;

const TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const RESPONSES_URL: &str = "https://example.invalid/responses";

fn session() -> MockHttpResponse {
    MockHttpResponse::success(
        r#"{"token":"SYNTHETIC_SESSION","expires_at":4102444800,"endpoints":{"api":"https://example.invalid"}}"#,
    )
}

fn body(output: Value) -> Value {
    json!({
        "id": "resp_synthetic", "object": "response", "status": "completed",
        "model": "gpt-6-luna", "created_at": 1,
        "tools": [{"type": "web_search", "search_context_size": "medium"}],
        "output": output,
        "usage": {"input_tokens": 10, "output_tokens": 5, "total_tokens": 15},
    })
}

fn reply(output: Value) -> MockHttpResponse {
    MockHttpResponse::success(body(output).to_string())
}

fn searched() -> Value {
    json!([
        {"type": "reasoning", "id": "rs_1", "summary": []},
        {"type": "web_search_call", "id": "ws_1", "status": "completed",
         "action": {"type": "search", "query": "sqlite release",
                    "sources": [{"type": "url", "url": "https://sqlite.org/news.html"},
                                {"type": "url", "url": "javascript:alert(1)"}]}},
        {"type": "message", "id": "msg_1", "role": "assistant", "status": "completed",
         "content": [{"type": "output_text", "text": "SQLite 3.99 is current.",
                      "annotations": [{"type": "url_citation", "start_index": 0, "end_index": 6,
                                       "url": "https://sqlite.org/releaselog/3_99.html",
                                       "title": "Release 3.99"}]}]},
    ])
}

fn account(token: &[u8]) -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let dir = root.path().join("credentials");
    Auth::open(&dir).unwrap();
    let path = dir.join("github-token");
    std::fs::write(&path, token).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    (root, dir)
}

fn request(dir: &std::path::Path, query: &str) -> Request {
    Request {
        model: "gpt-6-luna".into(),
        query: query.into(),
        effort: ReasoningEffort::Medium,
        credentials_dir: dir.to_owned(),
        account: Auth::open(dir).unwrap().copilot_account().unwrap(),
    }
}

async fn run_with(
    request: Request,
    replies: Vec<MockHttpResponse>,
) -> (Result<SearchResult, SearchError>, SequencedHttpClient) {
    let http = SequencedHttpClient::new(replies);
    let result = search_over(
        request,
        CancellationToken::new(),
        Some(DynHttpClient::new(http.clone())),
    )
    .await;
    (result, http)
}

#[tokio::test]
async fn sends_only_the_query_with_exactly_hosted_search_on_the_selected_account() {
    let (_root, dir) = account(b"SYNTHETIC_GITHUB");
    let (result, http) = run_with(
        request(&dir, "latest sqlite release"),
        vec![session(), reply(searched())],
    )
    .await;
    let result = result.unwrap();
    assert_eq!(result.answer, "SQLite 3.99 is current.");
    assert_eq!(
        result.sources,
        vec![
            Source {
                url: "https://sqlite.org/releaselog/3_99.html".into(),
                title: Some("Release 3.99".into()),
            },
            Source {
                url: "https://sqlite.org/news.html".into(),
                title: None,
            },
        ]
    );
    assert_eq!(
        result.provenance,
        Provenance {
            provider: Provider::Copilot,
            model: "gpt-6-luna".into(),
        }
    );
    let requests = http.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].uri, TOKEN_URL);
    assert_eq!(requests[1].uri, RESPONSES_URL);
    let body: Value = serde_json::from_slice(&requests[1].body).unwrap();
    assert_eq!(body["model"], "gpt-6-luna");
    assert_eq!(body["tools"], json!([{"type": "web_search"}]));
    assert_eq!(body["tool_choice"], "required");
    assert_eq!(body["max_tool_calls"], MAX_CALLS);
    assert!(
        body["include"]
            .as_array()
            .unwrap()
            .contains(&json!("web_search_call.action.sources"))
    );
    assert_eq!(body["reasoning"]["effort"], "medium");
    // Only the search instructions and the query: no notes, history or BRN tools.
    let input = body["input"].as_array().unwrap();
    let users: Vec<_> = input.iter().filter(|item| item["role"] == "user").collect();
    assert_eq!(users.len(), 1);
    assert!(users[0].to_string().contains("latest sqlite release"));
    assert!(
        input
            .iter()
            .all(|item| item["role"] == "user" || item.to_string().contains(PREAMBLE))
    );
    assert!(input.len() <= 2);
}

#[tokio::test]
async fn refuses_unrequested_capabilities_and_uncited_replies() {
    let mut function = searched();
    function.as_array_mut().unwrap().push(
        json!({"type": "function_call", "id": "fc_1", "call_id": "c", "name": "read_file", "arguments": "{}", "status": "completed"}),
    );
    let mut mcp = searched();
    mcp.as_array_mut().unwrap().insert(
        1,
        json!({"type": "mcp_call", "id": "mcp_1", "name": "shell", "server_label": "x", "arguments": "{}"}),
    );
    let unsearched = json!([searched()[2].clone()]);
    let mut uncited = searched();
    uncited[1]["action"]["sources"] = json!([]);
    uncited[2]["content"][0]["annotations"] = json!([]);
    for (output, expected) in [
        (function, SearchError::UnsafeToolUse),
        (mcp, SearchError::UnsafeToolUse),
        (unsearched, SearchError::EmptyResults),
        (uncited, SearchError::EmptyResults),
    ] {
        let (_root, dir) = account(b"SYNTHETIC_GITHUB");
        let (result, _) = run_with(request(&dir, "q"), vec![session(), reply(output)]).await;
        assert_eq!(result.unwrap_err(), expected);
    }
    let mut widened = body(searched());
    widened["tools"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type": "code_interpreter"}));
    let (_root, dir) = account(b"SYNTHETIC_GITHUB");
    let (result, _) = run_with(
        request(&dir, "q"),
        vec![session(), MockHttpResponse::success(widened.to_string())],
    )
    .await;
    assert_eq!(
        result.unwrap_err(),
        SearchError::CapabilityBoundaryUnverified
    );
}

#[tokio::test]
async fn another_account_or_no_account_never_searches() {
    let (_root, dir) = account(b"SYNTHETIC_GITHUB");
    let started = request(&dir, "q");
    std::fs::write(dir.join("github-token"), b"SYNTHETIC_OTHER_ACCOUNT").unwrap();
    let (result, http) = run_with(started, vec![session(), reply(searched())]).await;
    assert_eq!(result.unwrap_err(), SearchError::CopilotAccountChanged);
    assert!(http.requests().is_empty());

    let (_root, dir) = account(b"SYNTHETIC_GITHUB");
    let mut started = request(&dir, "q");
    std::fs::remove_file(dir.join("github-token")).unwrap();
    started.account = Auth::open(&dir).unwrap().copilot_account().unwrap();
    let (result, http) = run_with(started, vec![]).await;
    assert_eq!(result.unwrap_err(), SearchError::CopilotNoLogin);
    assert!(http.requests().is_empty());
}

#[tokio::test]
async fn a_model_that_cannot_search_is_reported_without_switching() {
    let (_root, dir) = account(b"SYNTHETIC_GITHUB");
    let mut started = request(&dir, "q");
    started.model = "claude-sonnet-5".into();
    let (result, http) = run_with(
        started,
        vec![
            session(),
            MockHttpResponse::error(
                rig::http_client::StatusCode::BAD_REQUEST,
                r#"{"error":{"message":"model claude-sonnet-5 is not supported via Responses API.","code":"unsupported_api_for_model"}}"#,
            ),
        ],
    )
    .await;
    assert_eq!(result.unwrap_err(), SearchError::CopilotModelCannotSearch);
    assert_eq!(http.requests().len(), 2);
}

#[tokio::test]
async fn stop_and_invalid_queries_dispatch_nothing() {
    let (_root, dir) = account(b"SYNTHETIC_GITHUB");
    let http = SequencedHttpClient::new([]);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = search_over(
        request(&dir, "q"),
        cancel,
        Some(DynHttpClient::new(http.clone())),
    )
    .await;
    assert_eq!(result.unwrap_err(), SearchError::Cancelled);
    for query in ["  ".to_owned(), "x".repeat(MAX_QUERY + 1)] {
        let (result, _) = run_with(request(&dir, &query), vec![]).await;
        assert_eq!(result.unwrap_err(), SearchError::InvalidQuery);
    }
    assert!(http.requests().is_empty());
}
