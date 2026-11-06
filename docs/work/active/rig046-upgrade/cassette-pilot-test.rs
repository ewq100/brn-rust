// Rig 0.46 rig-cassette pilot (#253), kept as evidence and not compiled.
// To rerun: add `rig-cassette = { version = "=0.46.0", default-features = false, features = ["http"] }`
// to brn-ai dev-dependencies and register this file as a `#[cfg(test)]` module of `threads.rs`.
//! Rig 0.46 reuse pilot (#253): `rig-cassette` HTTP replay over BRN's
//! production Copilot Responses runner. The recording is fed from a scripted
//! in-process transport; the upstream URL is never contacted, so there are no
//! live provider calls. Tools run against the fixture's temporary store only.
use super::*;
use rig::http_client::DynHttpClient;
use rig::test_utils::{MockHttpResponse, SequencedHttpClient};
use rig_cassette::http::{
    CassetteMode, CassetteSpec, DirectHttpRequest, DirectHttpResponse, ProviderCassette,
    RecordVia,
};

/// Discard port: nothing in this pilot may reach it.
const UPSTREAM: &str = "http://127.0.0.1:9/synthetic-copilot";
const SPEC: CassetteSpec = CassetteSpec::new("brn_multistep").exact_matched();

fn sse(events: &[Value]) -> String {
    events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .chain(["data: [DONE]\n\n".to_owned()])
        .collect()
}

fn completed(output: Value) -> Value {
    json!({"type":"response.completed","sequence_number":2,"response":{
        "id":"resp_synthetic","object":"response","created_at":1,"status":"completed",
        "model":"gpt-6.1-sol","output":output,"tools":[],
        "usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}})
}

/// Two model turns: one tool call with a real effect, then the answer.
fn script() -> Vec<String> {
    let call = json!({"type":"function_call","id":"fc_1","call_id":"call_1","name":"retain_progress",
        "arguments":"{\"summary\":\"Cassette step\"}","status":"completed"});
    let message = json!({"type":"message","id":"msg_1","status":"completed","role":"assistant",
        "content":[{"type":"output_text","annotations":[],"text":"Replayed answer"}]});
    vec![
        sse(&[
            json!({"type":"response.output_item.done","sequence_number":1,"output_index":0,"item":call}),
            completed(json!([call])),
        ]),
        sse(&[
            json!({"type":"response.output_item.done","sequence_number":1,"output_index":0,"item":message}),
            completed(json!([message])),
        ]),
    ]
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Exact replay needs the same store identities (the Chat id reaches the
/// prompt and the prompt cache key), so every run gets its own copy of one
/// pristine synthetic store.
fn isolated(pristine: &ThreadRunRequest) -> (tempfile::TempDir, ThreadRunRequest) {
    let (root, mut request, _) = tests::fixture();
    std::fs::remove_dir_all(&request.data_dir).unwrap();
    request.thread.clone_from(&pristine.thread);
    copy_tree(&pristine.data_dir, &request.data_dir);
    (root, request)
}

async fn run(
    pristine: &ThreadRunRequest,
    base_url: &str,
    http: DynHttpClient,
    effort: ReasoningEffort,
) -> (tempfile::TempDir, Arc<Scope>, AiAnswer) {
    let (root, request) = isolated(pristine);
    let (scope, input) = begin(&request).unwrap();
    let client = rig::providers::openai::OpenAIConfig::with_key(
        &rig::providers::copilot::DIALECT,
        "synthetic-not-a-credential",
    )
    .with_base_url(base_url)
    .connect(http);
    let model = crate::auth::copilot_completion(&client, "gpt-6.1-sol".into(), true);
    let answer = run_model(
        model,
        scope.clone(),
        &input,
        effort,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    (root, scope, answer)
}

fn progress(scope: &Scope) -> String {
    let store = scope.store.lock().unwrap();
    match store.record(&scope.run).unwrap().unwrap().data {
        RecordData::Run(run) => run.progress,
        _ => unreachable!(),
    }
}

async fn record(pristine: &ThreadRunRequest, path: std::path::PathBuf) {
    let session = ProviderCassette::start_at(
        RecordVia::Direct,
        "copilot",
        SPEC,
        UPSTREAM,
        CassetteMode::Record,
        path,
    )
    .await;
    let recorder = session.direct_recorder().expect("direct recording");
    let bodies = script();
    let http = SequencedHttpClient::new(
        bodies
            .iter()
            .map(|body| MockHttpResponse::success_typed(body.clone(), "text/event-stream")),
    );
    let (_root, scope, answer) =
        run(pristine, UPSTREAM, DynHttpClient::new(http.clone()), ReasoningEffort::Medium).await;
    assert!(
        matches!(answer.terminal, AiTerminal::Completed),
        "{:?}",
        answer.terminal
    );
    assert_eq!(progress(&scope), "Cassette step");
    let requests = http.requests();
    assert_eq!(requests.len(), bodies.len());
    for (request, body) in requests.iter().zip(&bodies) {
        recorder.record_http_interaction(
            DirectHttpRequest {
                method: "POST",
                uri: &request.uri,
                headers: request
                    .headers
                    .iter()
                    .map(|(name, value)| (name.as_str(), value.to_str().unwrap())),
                body: &request.body,
            },
            DirectHttpResponse {
                status: 200,
                headers: [("content-type", "text/event-stream")],
                body: body.as_bytes(),
            },
        );
    }
    session.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cassette_replays_a_multistep_tool_run_exactly_and_refuses_drift() {
    let (_pristine_root, pristine, _) = tests::fixture();
    let fixtures = tempfile::tempdir().unwrap();
    let path = fixtures.path().join("copilot/brn_multistep.yaml");
    record(&pristine, path.clone()).await;
    let yaml = std::fs::read_to_string(&path).unwrap();
    assert!(!yaml.contains("synthetic-not-a-credential"), "credential scrubbed");
    assert_eq!(yaml.lines().filter(|line| *line == "when:").count(), 2);

    // A fresh run in a fresh isolated store replays both turns exactly.
    let replay = ProviderCassette::start_at(
        RecordVia::Direct,
        "copilot",
        SPEC,
        UPSTREAM,
        CassetteMode::Replay,
        path.clone(),
    )
    .await;
    let (_root, scope, answer) = run(
        &pristine,
        &replay.base_url(),
        replay.direct_http(),
        ReasoningEffort::Medium,
    )
    .await;
    replay.try_finish().await.expect("every interaction played");
    assert!(
        matches!(answer.terminal, AiTerminal::Completed),
        "{:?}",
        answer.terminal
    );
    assert_eq!(answer.text, "Replayed answer");
    assert_eq!(progress(&scope), "Cassette step");

    // A changed request is refused before any tool effect, never served.
    let drift = ProviderCassette::try_start_at(
        RecordVia::Direct,
        "copilot",
        SPEC,
        UPSTREAM,
        CassetteMode::Replay,
        path,
    )
    .await
    .unwrap();
    let (_root, scope, answer) =
        run(&pristine, &drift.base_url(), drift.direct_http(), ReasoningEffort::High).await;
    assert!(
        !matches!(answer.terminal, AiTerminal::Completed),
        "{:?}",
        answer.terminal
    );
    assert_ne!(progress(&scope), "Cassette step");
    assert!(drift.try_finish().await.is_err(), "drift is reported");
}

