use rig::{
    agent::{AgentBuilder, AgentHook, HookContext, RunStart, RunStartAction},
    completion::Message,
    memory::{MemoryPolicy, TokenWindowMemory},
    run::{AgentRun, AgentRunStep, ModelTurn, RunEntry},
    test_utils::{MockHttpResponse, SequencedHttpClient},
    tool::{DynamicTool, ToolOutput},
};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
fn reply(tool: bool) -> MockHttpResponse {
    let message = if tool {
        json!({"role":"assistant","content":null,"tool_calls":[{"id":"call_1","type":"function","function":{"name":"read","arguments":"{}"}}]})
    } else {
        json!({"role":"assistant","content":"done"})
    };
    MockHttpResponse::success(json!({"id":"synthetic","object":"chat.completion","created":1,"model":"synthetic","choices":[{"index":0,"message":message,"finish_reason":if tool {"tool_calls"} else {"stop"}}],"usage":{"prompt_tokens":10,"completion_tokens":1,"total_tokens":11}}).to_string())
}
#[tokio::test]
async fn serialized_http_prefix_stable_within_run_and_between_boundaries() {
    let http = SequencedHttpClient::new([reply(true), reply(true), reply(false), reply(false)]);
    let client =
        rig::providers::openai::OpenAIConfig::new("synthetic-not-credential").connect(http.clone());
    let tool = DynamicTool::new(
        "read".try_into().unwrap(),
        "read synthetic evidence",
        json!({"type":"object","properties":{}}),
        |_| Box::pin(async { Ok(ToolOutput::text("source")) }),
    );
    let agent = AgentBuilder::new(client.chat("synthetic"))
        .preamble("fixed authority")
        .dynamic_tool(tool)
        .build();
    // Apply exactly once at a run boundary. Preserve this retained prefix between
    // compaction boundaries; continuously sliding it would invalidate caching.
    let history = TokenWindowMemory::new(2, |_: &Message| 1)
        .apply(vec![
            Message::user("discard"),
            Message::assistant("old"),
            Message::user("requirement"),
            Message::assistant("ack"),
        ])
        .unwrap();
    agent
        .prompt("request one")
        .history(history.clone())
        .max_turns(4)
        .await
        .unwrap();
    let mut next_history = history;
    next_history.extend([Message::user("request one"), Message::assistant("done")]);
    agent
        .prompt("request two")
        .history(next_history)
        .await
        .unwrap();
    let captured = http.requests();
    std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("http-requests.json"),
        serde_json::to_vec_pretty(
            &captured
                .iter()
                .map(|r| serde_json::from_slice::<Value>(&r.body).unwrap())
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    )
    .unwrap();
    let requests: Vec<Value> = captured
        .iter()
        .map(|r| serde_json::from_slice(&r.body).unwrap())
        .collect();
    assert_eq!(requests.len(), 4);
    // Exclude only the closing history delimiter and subsequent appended turns.
    // The preceding envelope and serialized history must match actual wire bytes.
    for pair in captured[..3].windows(2) {
        let old = std::str::from_utf8(&pair[0].body).unwrap();
        let new = std::str::from_utf8(&pair[1].body).unwrap();
        let messages =
            serde_json::to_string(&serde_json::from_str::<Value>(old).unwrap()["messages"])
                .unwrap();
        let start = old.find(&messages).unwrap();
        assert!(
            new.as_bytes()
                .starts_with(&old.as_bytes()[..start + messages.len() - 1])
        );
    }
    for pair in requests[..3].windows(2) {
        let old = pair[0]["messages"].as_array().unwrap();
        let new = pair[1]["messages"].as_array().unwrap();
        assert_eq!(
            serde_json::to_vec(old).unwrap(),
            serde_json::to_vec(&new[..old.len()]).unwrap()
        );
        assert_eq!(pair[0]["tools"], pair[1]["tools"]);
    }
    let a = requests[0]["messages"].as_array().unwrap();
    let b = requests[3]["messages"].as_array().unwrap();
    assert_eq!(
        serde_json::to_vec(a).unwrap(),
        serde_json::to_vec(&b[..a.len()]).unwrap()
    );
    assert_ne!(a.last(), b.last());
    assert_eq!(http.remaining_responses(), 0);
    println!(
        "prefix: 3 requests in run 1, 1 in run 2; all serialized message-prefix bytes/tools stable; live cached-input share unmeasured"
    );
}
#[derive(Clone)]
struct Memory(Arc<AtomicUsize>);
impl rig::memory::ConversationMemory for Memory {
    fn load<'a>(
        &'a self,
        _: &'a rig::id::ConversationId,
    ) -> rig::wasm_compat::WasmBoxedFuture<'a, Result<Vec<Message>, rig::memory::MemoryError>> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        })
    }
    fn append<'a>(
        &'a self,
        _: &'a rig::id::ConversationId,
        _: Vec<Message>,
    ) -> rig::wasm_compat::WasmBoxedFuture<'a, Result<(), rig::memory::MemoryError>> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
    fn clear<'a>(
        &'a self,
        _: &'a rig::id::ConversationId,
    ) -> rig::wasm_compat::WasmBoxedFuture<'a, Result<(), rig::memory::MemoryError>> {
        Box::pin(async { Ok(()) })
    }
}
struct Hook(Arc<AtomicUsize>);
impl AgentHook for Hook {
    async fn on_run_start(&self, _: &HookContext, _: RunStart<'_>) -> RunStartAction {
        self.0.fetch_add(1, Ordering::SeqCst);
        RunStartAction::Continue
    }
}
#[tokio::test]
async fn disposable_checkpoint_retains_payload_but_not_configured_runtime() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tool_calls = Arc::new(AtomicUsize::new(0));
    let memory_calls = Arc::new(AtomicUsize::new(0));
    let http = SequencedHttpClient::new([reply(true), reply(false)]);
    let client = rig::providers::openai::OpenAIConfig::new("synthetic").connect(http);
    let captured = tool_calls.clone();
    let tool = DynamicTool::new(
        "read".try_into().unwrap(),
        "synthetic",
        json!({"type":"object"}),
        move |_| {
            captured.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(ToolOutput::text("read")) })
        },
    );
    let agent = AgentBuilder::new(client.chat("synthetic"))
        .add_hook(Hook(calls.clone()))
        .memory(Memory(memory_calls.clone()))
        .conversation("synthetic")
        .dynamic_tool(tool)
        .build();
    let mut direct = AgentRun::new("private synthetic payload")
        .with_history(vec![Message::user("retained history")])
        .max_turns(3);
    direct.append_entry(RunEntry {
        kind: "synthetic".into(),
        turn: 0,
        value: json!({"secret_like":"fixture-only"}),
    });
    let before = serde_json::to_value(&direct).unwrap();
    let mut restored: AgentRun = serde_json::from_value(before.clone()).unwrap();
    assert_eq!(before, serde_json::to_value(&restored).unwrap());
    assert!(matches!(
        restored.next_step().unwrap(),
        AgentRunStep::CallModel { .. }
    ));
    let pending = serde_json::to_value(&restored).unwrap();
    let resumed: AgentRun = serde_json::from_value(pending.clone()).unwrap();
    assert_eq!(pending, serde_json::to_value(resumed).unwrap());
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "sans-IO step cannot call configured hook"
    );
    restored.advertise_tools(1, agent.tool_server_handle().static_tool_defs());
    let response = client
        .chat("synthetic")
        .call(rig::completion::CompletionRequest::new("manual driver"))
        .await
        .unwrap();
    restored
        .model_response(ModelTurn::from_response_parts(
            &response,
            ["read".into()].into(),
            ["read".into()].into(),
        ))
        .unwrap();
    assert!(matches!(
        restored.next_step().unwrap(),
        AgentRunStep::CallTools { .. }
    ));
    let tool_checkpoint = serde_json::to_value(&restored).unwrap();
    let tool_restored: AgentRun = serde_json::from_value(tool_checkpoint.clone()).unwrap();
    assert_eq!(
        tool_checkpoint,
        serde_json::to_value(tool_restored).unwrap()
    );
    assert!(
        !tool_checkpoint["completion_calls"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(tool_checkpoint.to_string().contains("call_1"));
    assert_eq!(
        tool_calls.load(Ordering::SeqCst),
        0,
        "direct CallTools is data, not automatic execution"
    );
    assert_eq!(
        memory_calls.load(Ordering::SeqCst),
        0,
        "direct drive never loads/appends configured memory"
    );
    assert_eq!(agent.tool_server_handle().static_tool_defs().len(), 1);
    agent.prompt("hook-aware fresh invocation").await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        memory_calls.load(Ordering::SeqCst),
        2,
        "hook-aware run loads and appends memory"
    );
    let mut unknown = before.clone();
    unknown["format"] = json!(999);
    assert!(serde_json::from_value::<AgentRun>(unknown).is_err());
    println!(
        "checkpoint retained keys: {}",
        before
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "checkpoint: input/history/entries and pending state roundtrip; direct step skips configured hook; hook-aware invocation calls it; unknown format refuses"
    );
}
