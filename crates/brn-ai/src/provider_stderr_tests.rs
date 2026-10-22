use super::*;

const CHILD: &str = "provider_formats_tests::provider_stderr_tests::provider_stderr_child";
const CHILD_ENV: &str = "BRN_SYNTHETIC_STDERR_CHILD";
const CHILD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const MAX_CHILD_OUTPUT: u64 = 64 * 1024;

struct OwnedChild(Option<std::process::Child>);

impl OwnedChild {
    fn wait_until(&mut self, deadline: std::time::Instant) -> std::process::ExitStatus {
        loop {
            let child = self.0.as_mut().expect("child is owned until completion");
            if let Some(status) = child.try_wait().unwrap() {
                self.0.take();
                return status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "synthetic provider stderr child timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}

#[test]
fn rejected_tool_arguments_and_partial_text_never_reach_process_stderr() {
    let parent = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    std::fs::set_permissions(parent.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let nonce = parent.path().file_name().unwrap().to_str().unwrap();
    let name = format!("r4_private_unknown_{nonce}");
    let argument = format!("R4_PRIVATE_ARGUMENT_{nonce}");
    let prior = format!("R4_PRIVATE_PRIOR_{nonce}");
    let stdout_path = parent.path().join("child.stdout");
    let stderr_path = parent.path().join("child.stderr");
    let stdout_file = std::fs::File::create(&stdout_path).unwrap();
    let stderr_file = std::fs::File::create(&stderr_path).unwrap();
    let mut child = OwnedChild(Some(
        std::process::Command::new(std::env::current_exe().unwrap())
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
            .stdout(stdout_file)
            .stderr(stderr_file)
            .spawn()
            .unwrap(),
    ));
    let status = child.wait_until(std::time::Instant::now() + CHILD_TIMEOUT);
    assert!(status.success(), "synthetic child failed");
    for path in [&stdout_path, &stderr_path] {
        assert!(
            std::fs::metadata(path).unwrap().len() <= MAX_CHILD_OUTPUT,
            "synthetic child output exceeded the capture limit"
        );
    }
    let stdout = String::from_utf8_lossy(&std::fs::read(&stdout_path).unwrap()).into_owned();
    assert!(
        stdout.contains("R4_ROUTES_VALIDATED:3"),
        "child did not finish all routes"
    );
    assert!(
        stdout.contains("1 passed; 0 failed"),
        "child test did not execute"
    );
    let stderr = String::from_utf8_lossy(&std::fs::read(&stderr_path).unwrap()).into_owned();
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
        let sse =
            partial.clone() + &tool_sse(responses, &[(name.as_str(), json!({"private":argument}))]);
        let (_root, client, http) = super::client(provider, model, vec![success(sse)]).await;
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

        // Also qualify Rig 0.44's malformed-argument policy on a known tool,
        // whose raw rejected payload must never be printed by the runtime.
        let sse = partial + &tool_sse(responses, &[("read_note", json!([argument]))]);
        let (_root, client, http) = super::client(provider, model, vec![success(sse)]).await;
        let notes = Arc::new(Notes::default());
        let (answer, _) = run(client, notes.clone(), CancellationToken::new()).await;
        assert!(matches!(answer.terminal, AiTerminal::Failed(_)));
        assert_eq!(notes.calls.load(Ordering::SeqCst), 0);
        assert_eq!(answer.text, prior);
        http.assert_consumed();
    }
    println!("R4_ROUTES_VALIDATED:3");
}
