//! Copilot denial through the production run path (`prepare_run` with a
//! Copilot sign-in): quota, policy and an unavailable model each fail
//! honestly on the selected account, with no Codex fallback and no other model.
use super::*;
use rig::test_utils::{MockHttpResponse, SequencedHttpClient};

fn credentials(root: &tempfile::TempDir) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let dir = root.path().join("copilot");
    crate::Auth::open(&dir).unwrap();
    let path = dir.join("github-token");
    std::fs::write(&path, "synthetic-github-token").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    dir
}

fn session() -> MockHttpResponse {
    MockHttpResponse::success(
        json!({"token":"synthetic-session","expires_at":4102444800u64,"endpoints":{"api":"https://copilot.example.invalid"}}).to_string(),
    )
}

fn refused(status: u16, body: Value) -> MockHttpResponse {
    MockHttpResponse::error(
        rig::http_client::StatusCode::from_u16(status).unwrap(),
        body.to_string(),
    )
}

/// A Copilot run with a Codex login also present in Settings: the Codex file
/// must never be used.
async fn denied(model: &str, replies: Vec<MockHttpResponse>) -> (ThreadResult, Vec<String>) {
    let (root, mut request, _) = tests::fixture();
    request.selection.provider = Provider::Copilot;
    request.selection.model = model.into();
    request.auth_file = Some(cache_tests::codex_file(&root));
    request.credentials_dir = Some(credentials(&root));
    cache_tests::configure(&request, "Copilot");
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
    settings.model = model.into();
    let op = store.allocate_operation("synthetic copilot model").unwrap();
    store
        .configure_workspace(&op, &settings, &HostAuthority::owner("fixture"))
        .unwrap();
    drop(store);
    let http = SequencedHttpClient::new(replies);
    let result = run_prepared_over(
        prepare_run(request).unwrap(),
        CancellationToken::new(),
        Arc::new(|_| {}),
        Some(DynHttpClient::new(http.clone())),
    )
    .await
    .unwrap();
    let uris = http.requests().into_iter().map(|r| r.uri).collect();
    (result, uris)
}

fn failure(result: &ThreadResult) -> (AiErrorKind, String) {
    let AiTerminal::Failed(error) = &result.answer.terminal else {
        panic!("{:?}", result.answer.terminal);
    };
    (error.kind, failed_attention(error))
}

fn no_codex(uris: &[String]) {
    assert!(
        uris.iter().all(|u| !u.contains("chatgpt.com")),
        "Copilot never falls back to Codex: {uris:?}"
    );
}

#[tokio::test]
async fn copilot_quota_denial_is_a_usage_limit_without_fallback() {
    let (result, uris) = denied(
        "claude-sonnet-5.5",
        vec![
            session(),
            refused(
                429,
                json!({"error":{"code":"quota_exceeded","message":"synthetic quota"}}),
            ),
        ],
    )
    .await;
    assert_eq!(
        failure(&result),
        (
            AiErrorKind::RateLimited,
            "Run failed: The account usage limit was reached. Continue when the provider is ready."
                .into()
        )
    );
    assert_eq!(uris.len(), 2, "{uris:?}");
    no_codex(&uris);
}

/// A signed-in account that subscription, permission or policy refuses is not
/// told to reconnect: reconnecting would not help.
#[tokio::test]
async fn copilot_policy_denial_is_not_reported_as_sign_in() {
    let (result, uris) = denied(
        "claude-sonnet-5.5",
        vec![
            session(),
            refused(
                403,
                json!({"error":{"code":"forbidden","message":"synthetic policy"}}),
            ),
        ],
    )
    .await;
    assert_eq!(
        failure(&result),
        (
            AiErrorKind::ReconnectNeeded,
            "Run failed: The provider refused this account's request (subscription, permission or policy). Check the account's access or reconnect it, then Continue.".into()
        )
    );
    assert_eq!(uris.len(), 2, "{uris:?}");
    no_codex(&uris);
}

/// An account without the selected model (for example no Luna) is told so;
/// the same model is tried on Responses once and no other model is chosen.
#[tokio::test]
async fn an_absent_copilot_model_is_refused_without_switching_models() {
    let missing = || {
        refused(
            400,
            json!({"error":{"code":"model_not_found","message":"synthetic"}}),
        )
    };
    let (result, uris) = denied("gpt-6-luna", vec![session(), missing(), missing()]).await;
    assert_eq!(
        failure(&result),
        (
            AiErrorKind::ModelRefused,
            "Run failed: The provider doesn't offer this model. Choose another model, then Continue."
                .into()
        )
    );
    assert_eq!(
        uris[1..],
        [
            "https://copilot.example.invalid/chat/completions".to_string(),
            "https://copilot.example.invalid/responses".to_string()
        ]
    );
    no_codex(&uris);
}
