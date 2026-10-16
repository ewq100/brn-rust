# C0 Rig reuse proof

Snapshot `8f768943964a77711b26d5235df146f8efe1dc6c`; worktree `/Users/evokessler/.t3/worktrees/brn-rust/t3-ceb6e16c`, branch `work/build-c0-20261016`. Rig facade/core/agent/memory/rmcp **0.44.0**, rmcp **2.2.0**, Rust **1.98.1**. Root `git ls-tree` contains no committed experiment: this harness is new, not copied from parent's untracked work.

Independent manifest with `[workspace]`, lock seeded from root, separate target `/private/tmp/brn-c0-r0-20261016-target`; no production manifests/lock/source edits. All HTTP is `SequencedHttpClient`; the fake key string is synthetic. MCP server uses in-process duplex. No network/provider/model calls, real workspace data or canonical BRN writes. Normal pinned crate acquisition is [logged](acquisition.log).

## Executed commands

From repository root, serially after the baseline lanes:

```sh
CARGO_TARGET_DIR=/private/tmp/brn-c0-r0-20261016-target cargo fmt --manifest-path experiments/r0-rig-reuse/Cargo.toml
CARGO_TARGET_DIR=/private/tmp/brn-c0-r0-20261016-target CARGO_INCREMENTAL=0 cargo test --manifest-path experiments/r0-rig-reuse/Cargo.toml --locked --offline -- --nocapture
CARGO_TARGET_DIR=/private/tmp/brn-c0-r0-20261016-target CARGO_INCREMENTAL=0 cargo clippy --manifest-path experiments/r0-rig-reuse/Cargo.toml --all-targets --locked --offline -- -D warnings
```

Final locked [test output](test-final.log): **4 passed, 0 failed, 0 ignored**, test execution 0.12 s; compile time is separately printed in log. Strict [Clippy](clippy.log) passed. First development compile failed because the harness treated `ToolServerHandle::execute`'s `ToolResult` as `Result`; [failure preserved](test-first.log), corrected to `is_error()`/`output()`. [Second run](test-second.log) passed before the stronger raw-wire-prefix assertions. Root production tests were never changed.

## Proofs and limits

| Proof | Executable observation | Limit |
|---|---|---|
| Explicit token window / compactor | `memory.rs`: explicit `MemoryPolicy::apply` on 4 canonical fixture Messages retains 2; serialized source unchanged. Stub `Compactor` invoked, repeated `load` reuses summary/watermark; reconstructed `CompactingMemory` over same backend compacts again. Backend Messages byte-identical, no automatic append of summary. | Synthetic one-token-per-message counter proves mechanism only, not model token accuracy or whole-request budgeting. Stub summary is not semantic qualification. Persistence/revocation needs D1B adapter. |
| Prefix stability | `prefix.rs`: policy applied once at run start; 3 real OpenAI Chat serialization requests through scripted HTTP (two tool rounds plus final). Each actual request's raw-byte prefix through all previous messages matches; serialized history slices and tools match. Second run appends the prior canonical owner/assistant pair without moving the frozen cut: the entire first request message sequence remains its byte-identical prefix, followed by new history/current request. [Captured bodies](http-requests.json), no headers. | Only this provider route, synthetic tool, fixed catalogue/boundary. Across-run compaction publication, scope changes and all BRN provider routes need D1A/C07. Live cached-input share **unmeasured**, not 100%. |
| MCP conversion/cancellation/availability | `mcp.rs`: actual `McpTool -> DynamicTool` returns source; managed `McpClientHandler` registers tools. 100 ms deadline surfaces error and server observes cancellation token. List-change notification removes `read`, adds `new_read`; removed dispatch errors. Disconnect retires availability. | In-process SDK transport, no network/auth/HTTP qualification; cancellation observed for synthetic server only, never assurance of external rollback. No BRN egress allowlist/private policy implemented. |
| Disposable checkpoint | `prefix.rs`: new, AwaitingModel and ExecutingTools state JSON roundtrip; retained history, prompt, host entry, tool call, raw completion record and advertised definitions inspected. Unknown format refuses. Manual `next_step`/model response yields `CallTools` but configured tool callback, hook and memory counters stay zero. Fresh hook-aware Agent invocation calls hook once and loads/appends memory twice. | Format 2 is a versioned Rig envelope, not BRN durable state. No BRN authority/receipt/lease restoration, cross-process effect replay or safe canonical retention proof. |

The prefix invariant refers to provider prompt/history plus stable instructions/tool schemas. JSON closing delimiters and newly appended turns necessarily differ. No claim of entire HTTP request equality or measured provider cache hits.

## Every retained AgentRun field

Format 2 serializes these 24 top-level fields (printed by test):

- `format`: version gate.
- `max_turns`, `max_invalid_tool_call_retries`, `max_consecutive_malformed_tool_calls`, `unhandled_invalid_tool_call`: execution/recovery policy.
- `tool_choice`, `output_tool_name`, `output_schema`, `max_output_retries`, `output_retries`: tool/output policy and retries; schemas may retain supplied text.
- `chat_history`, `new_messages`: full input history/current prompt, assistant/tool content and results; may retain private/source payloads.
- `current_turn`, `usage`, `completion_call_index`, `completion_calls`: counters, usage and completed-call records including decoded **raw provider response**, origin/request/response identity and failures when supplied.
- `invalid_tool_call_retries`, `malformed_tool_call_retries`, `rollback_pending`, `streamed_completion_call_recorded`: retry/stream rollback state.
- `previous_model`: last model identity; `turn_tools`: advertised definitions/schema for the turn, no executable binding.
- `entries`: arbitrary host namespace/turn/JSON **verbatim**, not validated/redacted by Rig.
- `state`: pending/resolving/executing/terminal state, pending call arguments, accepted turn or final response as applicable.

Not retained as executable services: configured models/transports, credentials handles, tools, hooks, memory backends or BRN grants/leases. Direct-driving is explicitly sans IO; a driver must invoke the required services and lifecycle hooks. Such raw retained histories/tool responses conflict with BRN's bounded progress retention design. Production decision: small checked BRN progress records plus fresh invocations; raw checkpoints remain disposable test material and excluded from workspace/backup.

## Dependency/feature decision

[Resolved rmcp features](rmcp-features.txt): client/server/macros, test-only `transport-io`, and exactly one candidate network transport `transport-streamable-http-client-reqwest`. No child-process transport. Server and IO are proof-only, not a proposal to ship a BRN server.

[Lock delta](lock-delta.json) compares package identities to root: **180 experiment vs 952 root**, 11 new identities including this harness; 783 root identities are absent because this is an independent package, not production deletions. New upstream packages: darling/darling_core/darling_macro 0.23.0, rig-memory/rig-rmcp 0.44.0, rmcp/rmcp-macros 2.2.0, sse-stream 0.2.6, tokio-stream 0.1.19, want 0.3.2. No new reqwest/schemars major: experiment has only reqwest **0.13.5** and schemars **1.2.2**; root retains reqwest **0.12.28 + 0.13.5**, schemars **1.2.2**. Production lock unchanged.

Cargo initially selected newer compatible hyper/mio while pruning the seeded graph. [Reconciliation](lock-reconcile.log) restored root's hyper **1.11.1** and mio **1.2.3** before final locked test/lint. Normal acquisition included an earlier provisional rmcp 2.0.0 and dependency versions; only the final lock identifies the tested graph. No production dependency or toolchain upgrade is proposed.

Limits remain: no semantic compactor/live token metrics, cold-restart derived-cache implementation, all-provider prefix qualification, HTTP localhost deployment or BRN scope/egress/security acceptance. Those belong to the gated D slices. Four active decisions are in the [capabilities plan](../../docs/work/active/agent-capabilities/plan.md).
