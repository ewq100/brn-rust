mod support;
use support::{run, Root};
#[test]
fn disabled_configuration_reopens_without_connecting_and_retains_normal_envelopes() {
    let root = Root::new();
    let data = root.workspace("data");
    let status = run(&root, true, &data, &["mcp-status"]);
    assert_eq!(status.code, 0, "{}", status.stdout);
    assert_eq!(status.envelope()["result"]["configured"], false);
    let policy =
        r#"{"endpoint":"http://127.0.0.1:8077/mcp","enabled":false,"tools":[],"access":[]}"#;
    let configured = run(&root, true, &data, &["mcp-configure", "--policy", policy]);
    assert_eq!(configured.code, 0, "{}", configured.stdout);
    let status = run(&root, true, &data, &["mcp-status"]);
    assert_eq!(status.code, 0, "{}", status.stdout);
    assert_eq!(status.envelope()["result"]["configured"], true);
    assert_eq!(status.envelope()["result"]["connection_enabled"], false);
    assert_eq!(status.envelope()["result"]["grant_required"], true);
    assert_eq!(status.envelope()["result"]["tools"], serde_json::json!([]));
    let disabled = run(&root, true, &data, &["mcp-disable"]);
    assert_eq!(disabled.code, 0, "{}", disabled.stdout);
    let cleared = run(&root, true, &data, &["mcp-configure", "--policy", "null"]);
    assert_eq!(cleared.code, 0, "{}", cleared.stdout);
    assert_eq!(
        run(&root, true, &data, &["mcp-status"]).envelope()["result"]["configured"],
        false
    );
}
#[test]
fn unsupported_destination_or_unpaired_review_is_usage_before_workspace_io() {
    let root = Root::new();
    let data = root.join("never-created");
    for args in [
        vec!["mcp-discover", "--endpoint", "https://example.invalid/mcp"],
        vec![
            "ask",
            "--thread",
            "chat",
            "--prompt",
            "hello",
            "--mcp-tool",
            "read",
        ],
        vec![
            "ask",
            "--thread",
            "chat",
            "--prompt",
            "hello",
            "--mcp-tool",
            "read",
            "--mcp-input",
            "[]",
        ],
    ] {
        let result = run(&root, true, &data, &args);
        assert_eq!(result.code, 2, "{}", result.stdout);
        assert_eq!(result.envelope()["status"], "usage");
        assert!(!data.exists());
    }
}

#[cfg(unix)]
#[test]
fn refused_transient_attachment_is_unavailable_without_provider_contact_or_second_run() {
    use support::stub::{proxy_env, Stub};
    use support::{assert_error, id, ok};
    let mut root = Root::new();
    root.env = proxy_env();
    let data = root.workspace("data");
    let stub = Stub::start(vec![]);
    let credentials = stub.credentials(&root);
    ok(
        &root,
        &data,
        &[
            "configure",
            "--provider",
            "Copilot",
            "--model",
            "synthetic",
            "--effort",
            "low",
            "--credentials-dir",
            credentials.to_str().unwrap(),
        ],
    );
    let chat = id(&ok(
        &root,
        &data,
        &["thread-start", "--title", "Synthetic review"],
    ));
    let result = run(
        &root,
        true,
        &data,
        &[
            "ask",
            "--thread",
            &chat,
            "--prompt",
            "Inspect the public fixture",
            "--mcp-tool",
            "public_read",
            "--mcp-input",
            r#"{"query":"TRANSIENT_CLI_REVIEW_CANARY"}"#,
        ],
    );
    let envelope = result.envelope();
    assert_eq!(result.code, 7, "{envelope}");
    assert_error(&envelope, "failed", "unavailable");
    assert_eq!(envelope["error"]["message"], "The approved MCP connection or its external-data grant is unavailable. Review the local connection and authorize the exact input again before continuing.");
    assert!(
        stub.requests().is_empty(),
        "no auth, summary or chat request"
    );
    let runs = ok(&root, &data, &["runs", "--thread", &chat]);
    assert_eq!(
        runs["runs"].as_array().unwrap().len(),
        1,
        "one durable prepare"
    );
    assert_eq!(runs["runs"][0]["state"], "failed");
    let records = ok(&root, &data, &["records"]);
    let saved = serde_json::to_string(&records).unwrap();
    assert!(
        saved.contains("Inspect the public fixture"),
        "posted instruction retained"
    );
    assert!(
        !saved.contains("TRANSIENT_CLI_REVIEW_CANARY"),
        "review input is transient"
    );
}

#[cfg(unix)]
#[path = "support/mcp.rs"]
mod sdk_fixture;

#[cfg(unix)]
#[test]
fn real_cli_sdk_discovery_reviewed_call_and_disable_preserve_local_work() {
    use serde_json::json;
    use support::{
        assert_error, id, ok,
        stub::{answer, proxy_env, tool, Stub},
    };
    let server = sdk_fixture::Server::start();
    let mut root = Root::new();
    root.env = proxy_env();
    let data = root.workspace("data");
    let note = id(&ok(
        &root,
        &data,
        &[
            "note-create",
            "--title",
            "Canonical positive",
            "--text",
            "Keep canonical work intact",
        ],
    ));
    let discovered = ok(
        &root,
        &data,
        &["mcp-discover", "--endpoint", &server.endpoint],
    );
    assert_eq!(server.calls(), 0, "discovery grants no tool call");
    assert!(!ok(&root, &data, &["mcp-status"])["configured"]
        .as_bool()
        .unwrap());
    let metadata = &discovered["tools"][0];
    assert_eq!(metadata["name"], "public_read");
    assert_eq!(metadata["supported"], true);
    let input = json!({"query":"PUBLIC_PROCESS_REVIEW_CANARY_õ"});
    let name = brn_ai::mcp::model_name(
        &brn_ai::mcp::Endpoint::parse(&server.endpoint).unwrap(),
        metadata["metadata_digest"].as_str().unwrap(),
    );
    let stub = Stub::start(vec![
        answer("Local-only answer."),
        tool("external", &name, input.clone()),
        answer("Checked synthetic source value 23."),
        answer("Local work after disable."),
    ]);
    let credentials = stub.credentials(&root);
    ok(
        &root,
        &data,
        &[
            "configure",
            "--provider",
            "Copilot",
            "--model",
            "synthetic",
            "--effort",
            "low",
            "--credentials-dir",
            credentials.to_str().unwrap(),
        ],
    );
    let policy = json!({"endpoint":server.endpoint,"server_digest":discovered["server_digest"],"enabled":true,"tools":[{"name":"public_read","metadata_digest":metadata["metadata_digest"],"schema_digest":metadata["schema_digest"],"contract":"query"}],"access":["general"]});
    ok(
        &root,
        &data,
        &["mcp-configure", "--policy", &policy.to_string()],
    );
    let chat = id(&ok(
        &root,
        &data,
        &["thread-start", "--title", "Synthetic MCP process"],
    ));
    let before = server.requests();
    let local = ok(
        &root,
        &data,
        &[
            "ask",
            "--thread",
            &chat,
            "--prompt",
            "Do ordinary local work",
        ],
    );
    assert_eq!(local["terminal"], "Completed");
    assert_eq!(
        server.requests(),
        before,
        "enablement alone does not connect"
    );
    assert_eq!(server.calls(), 0);
    assert!(!stub.requests()[0]["tools"].to_string().contains("mcp_"));
    let reviewed = run(
        &root,
        true,
        &data,
        &[
            "ask",
            "--thread",
            &chat,
            "--prompt",
            "Read the reviewed public source",
            "--mcp-tool",
            "public_read",
            "--mcp-input",
            &input.to_string(),
            "--trace",
            "full",
        ],
    );
    let result = reviewed.envelope();
    assert_eq!(reviewed.code, 0, "{result}");
    assert_eq!(result["result"]["terminal"], "Completed");
    assert_eq!(
        result["result"]["answer"],
        "Checked synthetic source value 23."
    );
    assert_eq!(server.calls(), 1);
    assert_eq!(server.inputs(), vec![input.clone()]);
    assert_eq!(
        server.credential_headers(),
        0,
        "provider credentials never reach MCP"
    );
    let requests = stub.requests();
    assert_eq!(requests.len(), 3);
    let prefix = requests[1]["messages"].as_array().unwrap();
    let continued = requests[2]["messages"].as_array().unwrap();
    assert_eq!(
        &continued[..prefix.len()],
        prefix.as_slice(),
        "actual HTTP message prefix stays append-only"
    );
    assert_eq!(requests[1]["tools"], requests[2]["tools"]);
    assert!(requests[1]["tools"].to_string().contains(&name));
    assert!(requests[2]["messages"]
        .to_string()
        .contains("PUBLIC_RAW_MCP_RESPONSE_CANARY"));
    for request in &requests {
        assert_eq!(request["model"], "synthetic");
        assert_eq!(request["reasoning_effort"], "low");
        for cap in ["max_tokens", "max_completion_tokens", "max_output_tokens"] {
            assert!(request.get(cap).is_none());
        }
    }
    assert!(!result.to_string().contains("PUBLIC_PROCESS_REVIEW_CANARY"));
    assert!(!result
        .to_string()
        .contains("PUBLIC_RAW_MCP_RESPONSE_CANARY"));
    let canonical = ok(&root, &data, &["records"]).to_string();
    assert!(canonical.contains(&note) && canonical.contains("Keep canonical work intact"));
    assert!(!canonical.contains("PUBLIC_PROCESS_REVIEW_CANARY"));
    assert!(!canonical.contains("PUBLIC_RAW_MCP_RESPONSE_CANARY"));
    assert_eq!(ok(&root, &data, &["candidates"]), json!([]));
    ok(&root, &data, &["mcp-disable"]);
    let before = server.requests();
    let denied = run(
        &root,
        true,
        &data,
        &[
            "ask",
            "--thread",
            &chat,
            "--prompt",
            "Review after disable",
            "--mcp-tool",
            "public_read",
            "--mcp-input",
            &input.to_string(),
        ],
    );
    assert_error(&denied.envelope(), "failed", "unavailable");
    assert_eq!(denied.code, 7);
    assert_eq!(server.requests(), before);
    assert_eq!(server.calls(), 1);
    assert_eq!(stub.requests().len(), 3);
    let local = ok(
        &root,
        &data,
        &[
            "ask",
            "--thread",
            &chat,
            "--prompt",
            "Continue ordinary local work",
        ],
    );
    assert_eq!(local["terminal"], "Completed");
    assert_eq!(server.calls(), 1);
    assert_eq!(server.requests(), before);
    assert_eq!(stub.requests().len(), 4);
    assert!(!stub.requests()[3]["tools"].to_string().contains("mcp_"));
}
