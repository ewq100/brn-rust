# Bounded Rig Responses runtime-contract comparison

## Starting baseline and previous findings

On 11 October 2026 the runtime-integration pass began at fetched production commit `0fcb8f737952189724a7156dde2c483719e9a6bf`, using patched Rig **0.43.0** and Rust **1.98.1**. Before changing production expectations or dependencies, the existing MacBook comparison was rerun unchanged against both published versions. It exited 0 and reproduced the previous six observations. This preserves the earlier finding while reassessing its product significance under the owner's explicit finalized-response contract.

Exact baseline command, run from the isolated checkout:

```sh
cargo +1.98.1 run --offline --locked \
  --manifest-path scripts/qualification/rig-decoder-comparison/Cargo.toml \
  --target-dir ../threads-runtime-local/rig-comparison-target
```

Baseline sanitized stdout:

```jsonl
{"rig":"0.43.0","fixture":"valid_control","terminal":"success","finish_reason":"Some(Stop)","text":"Synthetic valid answer","http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"valid_control","terminal":"success","finish_reason":"Some(Stop)","text":"Synthetic valid answer","http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"recognized_text_delta_wrong_type","terminal":"error","text":"","http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"recognized_text_delta_wrong_type","terminal":"success","finish_reason":"Some(Stop)","text":"Synthetic later text","http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"invalid_json","terminal":"error","text":"","http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"invalid_json","terminal":"error","text":"","http_requests":1,"tool_dispatches":0}
```

The previous upgrade trial counted 15 failures, including fixture/API adaptation failures and the numeric text-delta case (see [earlier evidence](evidence.md)). That aggregate is not a count of 15 demonstrated Rig runtime regressions. This independent harness reproduces the specific decoder finding without BRN fixtures or production glue; it does not reconstruct the earlier unavailable trial or claim every prior failure was independently reproduced.

## Expanded repeatable qualification

The [harness](../../../../scripts/qualification/rig-decoder-comparison/src/main.rs) depends on **published** `rig-core =0.43.0` and `=0.44.0`, each with `test-utils`, and has its own [manifest](../../../../scripts/qualification/rig-decoder-comparison/Cargo.toml) and [lockfile](../../../../scripts/qualification/rig-decoder-comparison/Cargo.lock). There is no local Rig patch in this qualification workspace. Identical synthetic SSE traverses the public ChatGPT Responses model, transport framing, decoder and terminal fold. Each case makes exactly one mock request. The mock client cannot contact a provider; no account/authentication exchanges, private data, credentials, registered tools or tool executions are involved.

The same version-aliased code collects only `StreamEvent::Text` as provisional `preview`, then reads successful `CompletionResponse.choice` after `Streamed::finish()` as `final_text`. It uses structured `AssistantContent::Text`; no raw JSON response reconstruction determines the answer. `finish_reason` and marked invalid tool arguments are reported separately. `terminal=success` means Rig returned `Ok(CompletionResponse)`; it is **not** BRN admission as a completed assistant answer. In particular, `Length` is incomplete and `ToolCalls` still requires application validation and the tool loop.

The only harness dependency adaptation adds an already locked `futures` dependency to poll the public stream API. No dependency version changed in this isolated lock. No BRN fixture/API adaptation failure is included as a runtime result. The fixtures include complete typed envelopes acceptable to both published versions and explicit recognized text, item completion and tool argument events.

Exact expanded qualification and hygiene commands:

```sh
cargo +1.98.1 fmt --manifest-path scripts/qualification/rig-decoder-comparison/Cargo.toml -- --check
cargo +1.98.1 run --offline --locked \
  --manifest-path scripts/qualification/rig-decoder-comparison/Cargo.toml \
  --target-dir ../threads-runtime-local/rig-comparison-target
cargo +1.98.1 clippy --offline --locked \
  --manifest-path scripts/qualification/rig-decoder-comparison/Cargo.toml \
  --target-dir ../threads-runtime-local/rig-comparison-target -- -D warnings
```

The locked run exited 0: **12 fixtures × 2 versions = 24 observations**, with assertions on the observed distinctions and one request per observation. Sanitized stdout:

```jsonl
{"rig":"0.43.0","fixture":"valid_control","terminal":"success","preview":"Synthetic valid answer","final_text":"Synthetic valid answer","finish_reason":"Some(Stop)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"valid_control","terminal":"success","preview":"Synthetic valid answer","final_text":"Synthetic valid answer","finish_reason":"Some(Stop)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"recognized_text_delta_wrong_type","terminal":"json_error","preview":"","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"recognized_text_delta_wrong_type","terminal":"success","preview":"Synthetic later text","final_text":"Synthetic later text","finish_reason":"Some(Stop)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"invalid_json","terminal":"json_error","preview":"","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"invalid_json","terminal":"json_error","preview":"","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"invalid_json_after_preview","terminal":"json_error","preview":"Synthetic partial","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"invalid_json_after_preview","terminal":"json_error","preview":"Synthetic partial","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"harmless_provider_metadata","terminal":"success","preview":"Synthetic valid answer","final_text":"Synthetic valid answer","finish_reason":"Some(Stop)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"harmless_provider_metadata","terminal":"success","preview":"Synthetic valid answer","final_text":"Synthetic valid answer","finish_reason":"Some(Stop)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"stream_A_completed_B","terminal":"success","preview":"Synthetic A","final_text":"Synthetic A","finish_reason":"Some(Stop)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"stream_A_completed_B","terminal":"success","preview":"Synthetic A","final_text":"Synthetic B","finish_reason":"Some(Stop)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"stream_A_item_done_B","terminal":"success","preview":"Synthetic A","final_text":"Synthetic A","finish_reason":"Some(Stop)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"stream_A_item_done_B","terminal":"success","preview":"Synthetic A","final_text":"Synthetic B","finish_reason":"Some(Stop)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"numeric_delta_without_final","terminal":"json_error","preview":"","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"numeric_delta_without_final","terminal":"truncated","preview":"Synthetic partial","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"truncated_stream","terminal":"truncated","preview":"Synthetic partial","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"truncated_stream","terminal":"truncated","preview":"Synthetic partial","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"sentinel_without_final","terminal":"truncated","preview":"Synthetic partial","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"sentinel_without_final","terminal":"truncated","preview":"Synthetic partial","final_text":"","finish_reason":"None","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"provider_incomplete","terminal":"success","preview":"Synthetic partial","final_text":"Synthetic partial","finish_reason":"Some(Length)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"provider_incomplete","terminal":"success","preview":"Synthetic partial","final_text":"Synthetic partial","finish_reason":"Some(Length)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.43.0","fixture":"malformed_tool_arguments","terminal":"success","preview":"","final_text":"","finish_reason":"Some(Stop)","decoded_tool_calls":0,"marked_invalid_calls":0,"http_requests":1,"tool_dispatches":0}
{"rig":"0.44.0","fixture":"malformed_tool_arguments","terminal":"success","preview":"","final_text":"","finish_reason":"Some(ToolCalls)","decoded_tool_calls":1,"marked_invalid_calls":1,"http_requests":1,"tool_dispatches":0}
```

## Disposition

| Case | Observed distinction | BRN contract and upgrade disposition |
| --- | --- | --- |
| Valid control and harmless gateway metadata/obfuscation | Both versions preserve the valid answer. | Harmless provider-format variation; qualifies. |
| Numeric recognized text delta followed by valid completed content | 0.43 fails JSON decoding; 0.44 skips unusable provisional content and finalizes valid later text. | The previous blocking expectation is intentionally changed. Streaming text is provisional; valid authoritative finalized content satisfies BRN. This is not a malformed tool call and tool retry settings are irrelevant to the decoder result. |
| Stream A, completed B (with and without `output_item.done`) | Both previews show A. 0.43's successful folded answer remains A; 0.44's structured final answer is B. | BRN must store B and reconcile/replace preview. 0.44 is more correct for the accepted product contract; concatenating A/B or persisting A is a BRN adapter defect. |
| Literal invalid JSON, before or after partial text, even with later valid completion | Both fail JSON decoding; partial text is retained only in the observed preview. | Genuine unrecoverable response; fail the run and never invent completion. |
| Numeric delta without final; EOF after text; `[DONE]` without `response.completed` | 0.44 reports `Truncated`; ordinary EOF/sentinel cases also truncate under 0.43. | Interruption/missing authoritative completion remains failure/partial progress. This EOF transport fixture is not evidence about an actual socket reset or user cancellation; production adapter tests cover those separately. |
| Provider `response.incomplete` with token-limit reason | Both return a structured response carrying `Length`. | BRN must treat it as incomplete rather than a completed answer. Successful decoding alone is insufficient. |
| Complete call lifecycle with malformed JSON arguments | 0.43 drops the malformed call from its choice in this fixture; 0.44 preserves one `ToolCalls` item marked `invalid_arguments`. | A malformed call is not executable. The new explicit representation must be checked by BRN and bounded Rig runtime policy. Harness dispatch is zero by construction; production whole-batch admission/retry tests are the execution evidence. |

No failure above requires making Rig 0.44 reproduce old provisional-stream accumulation or reinstating strict rejection of harmless/recoverable intermediate provider fields. Finalized response content, explicit finish reason and tool-call validation jointly define admission; no unknown finish reason is accepted here.

The local baseline and expanded logs are `../threads-runtime-local/evidence/rig-comparison-baseline.log` and `../threads-runtime-local/evidence/rig-comparison-expanded.log`, outside Git. This document preserves their synthetic stdout. Production migration, patch removal, actual six-tool runtime/live journeys and native evidence are recorded by the integration lead in [evidence.md](evidence.md).
