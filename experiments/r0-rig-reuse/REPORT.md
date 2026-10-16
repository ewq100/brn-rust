# C0 Rig reuse proof

Snapshot `8f768943964a77711b26d5235df146f8efe1dc6c`; worktree `/Users/evokessler/.t3/worktrees/brn-rust/t3-ceb6e16c`, branch `work/build-c0-20261016`. Rig facade/core/agent/memory/rmcp **0.44.0**, rmcp **2.2.0**, Rust **1.98.1**. Root `git ls-tree` contains no committed experiment: this harness is new, not copied from parent's untracked work.

Independent manifest with `[workspace]`, independent lock (acquisition provenance qualified below), separate target `/private/tmp/brn-c0-r0-20261016-target`; no production manifests/lock/source edits. All HTTP is `SequencedHttpClient`; the fake key string is synthetic. MCP server uses in-process duplex. No network/provider/model calls, real workspace data or canonical BRN writes. Normal pinned crate acquisition is [logged](acquisition.log).

## Executed commands

From repository root, serially after the baseline lanes:

```sh
CARGO_TARGET_DIR=/private/tmp/brn-c0-r0-20261016-target cargo fmt --manifest-path experiments/r0-rig-reuse/Cargo.toml
CARGO_TARGET_DIR=/private/tmp/brn-c0-r0-20261016-target CARGO_INCREMENTAL=0 cargo test --manifest-path experiments/r0-rig-reuse/Cargo.toml --locked --offline -- --nocapture
CARGO_TARGET_DIR=/private/tmp/brn-c0-r0-20261016-target CARGO_INCREMENTAL=0 cargo clippy --manifest-path experiments/r0-rig-reuse/Cargo.toml --all-targets --locked --offline -- -D warnings
```

Historical initial locked [test output](test-final.log): **4 passed, 0 failed, 0 ignored**, 0.12 s. Correction verification is recorded in [review verification](review-verification.json) with a **5-test** suite, actual resume and local abort/disconnect. Compilation and test wall times are recorded separately. Historical strict [Clippy](clippy.log) passed; correction lint is in the new verification receipt. First development compile failed because the harness treated `ToolServerHandle::execute`'s `ToolResult` as `Result`; [failure preserved](test-first.log), corrected to `is_error()`/`output()`. [Second run](test-second.log) passed before the stronger raw-wire-prefix assertions. Root production tests were never changed.

## Proofs and limits

| Proof | Executable observation | Limit |
|---|---|---|
| Explicit token window / compactor | `memory.rs`: explicit `MemoryPolicy::apply` on 4 canonical fixture Messages retains 2; retained window equals the exact final two canonical fixture Messages. Stub `Compactor` invoked, repeated `load` reuses summary/watermark; reconstructed `CompactingMemory` over same backend compacts again. Backend Messages byte-identical, no automatic append of summary. | Synthetic one-token-per-message counter proves mechanism only, not model token accuracy or whole-request budgeting. Stub summary is not semantic qualification. Persistence/revocation needs D1B adapter. |
| Prefix stability | `prefix.rs`: policy applied once at run start; 3 real OpenAI Chat serialization requests through scripted HTTP (two tool rounds plus final). Each actual request's raw-byte prefix through all previous messages matches; serialized history slices and tools match. Second run appends the prior canonical owner/assistant pair without moving the frozen cut: actual wire-byte prefix and stable tool schemas are asserted across runs too. Canonical BRN retention deliberately excludes tool transcripts across invocations. [Immutable initial sample bodies](http-requests.json), no headers; tests no longer rewrite this source file. Optional new capture is outside Git (see below). | Only this provider route, synthetic tool and append-only caller discipline with fixed catalogue/boundary; this does not prove automatic BRN policy enforcement. Across-run compaction publication, scope changes and all BRN provider routes need D1A/C07. Live cached-input share **unmeasured**, not 100%. |
| MCP conversion/cancellation/availability | `mcp.rs`: actual `McpTool -> DynamicTool` returns source; managed `McpClientHandler` registers tools. 100 ms deadline surfaces structured `ToolErrorKind::Timeout` and server observes cancellation token. List-change notification removes `read`, adds `new_read`; removed dispatch errors. Disconnect retires availability. | In-process SDK transport, no network/auth/HTTP qualification; cancellation observed for synthetic server only, never assurance of external rollback. No BRN egress allowlist/private policy implemented. Separate long-deadline local-abort/disconnect probe below establishes the Stop gap. |
| Disposable checkpoint | `prefix.rs`: new, AwaitingModel and ExecutingTools state JSON roundtrip; retained history, prompt, host entry, tool call, raw completion record and advertised definitions inspected. Unknown format refuses. Exact 24-key field set is asserted. Direct `next_step`/model response yields `CallTools` while configured services stay untouched. Fresh runner separately invokes start/settlement hooks, one memory load and one append. Actual `Agent::resume` re-executes pending tools under current hooks/tools; counters and retention implications are recorded below. | Format 2 is a versioned Rig envelope, not BRN durable state. No BRN authority/receipt/lease restoration, cross-process effect replay or safe canonical retention proof. |

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

Not retained as executable services: configured models/transports, credentials handles, tools, hooks, memory backends or BRN grants/leases. Direct-driving `AgentRun` is explicitly sans IO. Rig **already provides `Agent::resume(AgentRun)`** for hook-aware execution with configured services; a custom driver is unnecessary. The resumed started run does not rerun the initial-prompt hook, but does run settlement hooks and pending tools. `runner.rs::resumed_memory` and the executable probe show zero memory loads and one append per resume; `completion.rs`’s comment saying neither load nor append is stale relative to this implementation. Such raw retained histories/tool responses conflict with BRN's bounded progress retention design. Production decision: small checked BRN progress records plus fresh invocations; raw checkpoints remain disposable test material and excluded from workspace/backup.

## Dependency/feature decision

[Resolved rmcp features](rmcp-features.txt): client/server/macros, test-only `transport-io`, and exactly one candidate network transport `transport-streamable-http-client-reqwest`. No child-process transport. Server and IO are proof-only, not a proposal to ship a BRN server.

[Lock delta](lock-delta.json) compares package identities to root: **180 experiment vs 952 root**, 11 new identities including this harness; 783 root identities are absent because this is an independent package, not production deletions. New upstream packages: darling/darling_core/darling_macro 0.23.0, rig-memory/rig-rmcp 0.44.0, rmcp/rmcp-macros 2.2.0, sse-stream 0.2.6, tokio-stream 0.1.19, want 0.3.2. No new reqwest/schemars major: experiment has only reqwest **0.13.5** and schemars **1.2.2**; root retains reqwest **0.12.28 + 0.13.5**, schemars **1.2.2**. Production lock unchanged.

The retained fetch selected newer compatible hyper/mio while resolving the independent graph. [Reconciliation](lock-reconcile.log) restored root's hyper **1.11.1** and mio **1.2.3** before final locked test/lint. The earliest **retained** `acquisition.log` begins from an already provisional graph: rmcp 2.0.0, serde_json 1.0.149, tokio 1.50.0 and tokio-util 0.7.18. It records updates to the final direct pins and enabling the HTTP graph. The first provisional manifest/fetch log and pre-fetch lock were not retained before that log was overwritten. Earlier implementation text reported an initial root-lock copy, but there is no preserved lock/hash to independently prove that copy’s contents or the first resolution. Final shared-pin matching proves compatibility, **not initial seed provenance**. The final lock, reconciliation log and root comparison identify the tested graph without relying on that uncertain initial account. Correction removes the unused direct tokio-util edge; it remains transitive, with no package-version change. No production dependency or toolchain upgrade is proposed.

Limits remain: no semantic compactor/live token metrics, cold-restart derived-cache implementation, all-provider prefix qualification, HTTP localhost deployment or BRN scope/egress/security acceptance. Those belong to the gated D slices. Four active decisions are in the [capabilities plan](../../docs/work/active/agent-capabilities/plan.md).

## Opus correction probes

`Agent::resume` exists at rig-agent `agent/completion.rs:787`; the same started CallTools checkpoint is round-tripped and replayed twice. Counters distinguish all three paths:

| Path | Initial hook | Settlement hook | Tool callbacks | Memory loads | Memory appends |
|---|---:|---:|---:|---:|---:|
| Direct sans-IO drive | 0 | 0 | 0 | 0 | 0 |
| Fresh hook-aware invocation | 1 | 1 | 0 | 1 | 1 |
| Actual resume twice | 0 | 2 | 3 (1 simulated pre-crash effect + 2 re-executions) | 0 | 2 |

This is expected pending-tool replay, not safe exactly-once mutation. It can also append repeated resumed messages if a raw checkpoint is replayed. Production remains small checked progress/receipts plus fresh invocations. No custom driver, new canonical transcript store or raw-checkpoint adoption.

The new MCP test starts `wait` with a **60-second** deadline, waits for a **server-start barrier**, then aborts the local caller. After 200 ms the remote request has received no cancellation and its handler is still pending. Explicit client disconnect retires local tools but, after another 200 ms, still does not cancel/terminate that handler or finish server drain. A test-controlled server release finishes it. These bounded observations do not prove indefinite behavior, all transports or real-server semantics. They do refute a drop/disconnect cancellation guarantee for this pinned in-process case. D4/M04 needs supervised explicit SDK request cancellation where available, bounded server work and fencing of late local acceptance; an aborted local future no longer polls its deadline. Never claim remote Undo/Stop success from local drop/disconnect.

[First correction run](review-corrections-first.log) records **4 passed / 1 failed**: it initially expected disconnect to finish the server; the 2-second timeout exposed that unsupported assumption. [Second correction run](review-corrections-second.log) records the corrected observation (**5 passed / 0 failed / 0 ignored**). Earlier initial harness compile failure receipts remain unchanged.

Tests write no source artifacts by default. Optional `BRN_R0_CAPTURE_PATH` must be a new absolute file **outside the repository** (`create_new`, no overwrite); verification uses the task target directory. The committed `http-requests.json` is an immutable dated initial sample, not a live test output. The [rerun receipt](review-verification.json) checks Git status/diff fingerprints and sample SHA before/after a rerun, retaining the generated capture outside Git. No live cached-input metric is claimed.
