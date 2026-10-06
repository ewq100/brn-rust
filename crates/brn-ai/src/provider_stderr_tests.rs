use super::*;

const CHILD: &str = "provider_formats_tests::provider_stderr_tests::provider_stderr_child";
const CHILD_ENV: &str = "BRN_SYNTHETIC_STDERR_CHILD";

#[test]
fn rejected_tool_arguments_and_partial_text_never_reach_process_stderr() {
    let parent = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    std::fs::set_permissions(parent.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let nonce = parent.path().file_name().unwrap().to_str().unwrap();
    let name = format!("r4_private_unknown_{nonce}");
    let argument = format!("R4_PRIVATE_ARGUMENT_{nonce}");
    let prior = format!("R4_PRIVATE_PRIOR_{nonce}");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            CHILD,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_ENV, "synthetic-three-routes-v1")
        .env("BRN_SYNTHETIC_STDERR_TOOL", &name)
        .env("BRN_SYNTHETIC_STDERR_ARGUMENT", &argument)
        .env("BRN_SYNTHETIC_STDERR_PRIOR", &prior)
        .env("TMPDIR", parent.path().canonicalize().unwrap())
        .env("BRN_NATIVE_MODEL_DIR", "")
        .output()
        .unwrap();
    assert!(output.status.success(), "synthetic child failed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("R4_ROUTES_VALIDATED:3"),
        "child did not finish all routes"
    );
    assert!(
        stdout.contains("1 passed; 0 failed"),
        "child test did not execute"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    for marker in ["PARTIAL", name.as_str(), argument.as_str(), prior.as_str()] {
        assert!(
            !stderr.contains(marker),
            "synthetic private marker escaped to process stderr"
        );
    }
}

#[tokio::test]
#[ignore = "only the parent test launches this isolated synthetic stderr fixture"]
async fn provider_stderr_child() {
    assert_eq!(
        std::env::var(CHILD_ENV).unwrap(),
        "synthetic-three-routes-v1"
    );
    let name = std::env::var("BRN_SYNTHETIC_STDERR_TOOL").unwrap();
    let argument = std::env::var("BRN_SYNTHETIC_STDERR_ARGUMENT").unwrap();
    let prior = std::env::var("BRN_SYNTHETIC_STDERR_PRIOR").unwrap();
    for (provider, model, responses) in [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Copilot, "gpt-5.5", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ] {
        let partial = if responses {
            event(json!({"type":"response.output_text.delta","delta":prior,
                "item_id":"r4_prior","output_index":0,"content_index":0,"sequence_number":0}))
        } else {
            event(json!({"id":"synthetic","object":"chat.completion.chunk",
                "created":1,"model":"synthetic","choices":[{"index":0,
                "delta":{"role":"assistant","content":prior},"finish_reason":null}]}))
        };
        let sse = partial + &tool_sse(responses, &[(name.as_str(), json!({"private":argument}))]);
        let (_root, client, http) = client(provider, model, vec![success(sse)]).await;
        let notes = Arc::new(Notes::default());
        let (answer, events) = run(client, notes.clone(), CancellationToken::new()).await;
        assert!(
            matches!(
                answer.terminal,
                AiTerminal::Failed(AiError {
                    kind: AiErrorKind::InvalidToolUse,
                    ..
                })
            ),
            "expected fixed invalid-tool terminal"
        );
        assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
        assert_eq!(answer.text, prior);
        assert!(
            events
                .iter()
                .all(|event| matches!(event, AiEvent::Text(text) if text == &prior))
        );
        assert_eq!(events.len(), 1);
        http.assert_consumed();
    }
    println!("R4_ROUTES_VALIDATED:3");
}
