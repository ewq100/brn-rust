# Bounded Rig Responses decoder comparison

On 11 October 2026 this MacBook reproduced the earlier Rig 0.44 qualification issue with a minimal independent harness. The acceptance candidate retains Rust 1.98.1, Rig 0.43.0 and the existing `rig-agent` logging patch. BRN's workspace `Cargo.lock` is unchanged.

The [harness](../../../../scripts/qualification/rig-decoder-comparison/src/main.rs) depends on both published `rig-core` versions, with its own [manifest](../../../../scripts/qualification/rig-decoder-comparison/Cargo.toml) and [lockfile](../../../../scripts/qualification/rig-decoder-comparison/Cargo.lock). It feeds identical synthetic SSE through the real public ChatGPT Responses model, transport framing, decoder and terminal fold. The mock transport makes one request per case, has no account/authentication exchanges and cannot contact a provider. No agent, tool execution, private data or credential loading is involved. The only adaptation is the version alias; both versions use the same public `OpenAIConfig::with_key(...).connect(mock).responses(...)` and `stream(...).finish()` surface.

Run from the checkout, keeping the build outside Git:

```sh
cargo +1.98.1 fetch --locked --manifest-path scripts/qualification/rig-decoder-comparison/Cargo.toml
cargo +1.98.1 run --offline --locked \
  --manifest-path scripts/qualification/rig-decoder-comparison/Cargo.toml \
  --target-dir ../threads-macbook-local/rig-decoder-comparison/target
```

The locked run exited 0. Its output contains only fixed synthetic text, terminal categories and counts:

```jsonl
{"rig":"0.43.0","fixture":"valid_control","terminal":"success","finish_reason":"Some(Stop)","text":"Synthetic valid answer","http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"valid_control","terminal":"success","finish_reason":"Some(Stop)","text":"Synthetic valid answer","http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"recognized_text_delta_wrong_type","terminal":"error","text":"","http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"recognized_text_delta_wrong_type","terminal":"success","finish_reason":"Some(Stop)","text":"Synthetic later text","http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"invalid_json","terminal":"error","text":"","http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"invalid_json","terminal":"error","text":"","http_requests":1,"tool_dispatches":0}
```

The failing qualification fixture is valid JSON naming recognized `response.output_text.delta`, but its `delta` is the number `42` instead of text. It is followed by a valid text delta and a valid completed response. Rig 0.43 rejects the typed malformed event. Rig 0.44's deliberately lenient field reader accepts the recognized event and the later text reaches a successful `Stop` result. Literal invalid JSON is rejected by both versions; the finding does not claim otherwise. Assertions in the harness check these exact observations and one request per case.

This is malformed **stream content**, distinct from malformed **tool calls** and agent retry admission. `max_invalid_tool_call_retries(0)` and `max_consecutive_malformed_tool_calls(0)` do not run in this decoder-only harness, cannot validate this text delta, and are not evidence that this stream becomes an error. BRN's separate whole-batch tool validation remains intact. This bounded comparison supports retaining the current decoder behavior for acceptance; it is not a full Rig 0.44 upgrade attempt or an architecture review.

Local raw compilation/result log: `threads-macbook-local/evidence/rig-decoder-comparison-locked.log`, outside Git. This fresh witness replaces reliance on the other Mac's unavailable trial path for this particular observation. It does not independently reproduce every failure counted in the broader earlier trial.
