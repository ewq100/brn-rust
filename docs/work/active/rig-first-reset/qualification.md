# Direct Rig Qualification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for inline execution, or superpowers:subagent-driven-development when explicitly selected. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove exact downstream dependencies, protected credential reuse and both direct subscription execution paths before production replacement.

**Architecture:** A standalone disposable experiment exercises the pinned Rig facade with synthetic HTTP/replay transports. Provider-specific auth remains Rig-owned; the harness supplies private file paths, explicit interaction policy and sanitized evidence.

**Tech Stack:** Rust `1.98.1`, candidate `rig = "=0.43.0"`, Tokio, rig-cassette, existing BRN dependency graphs, tempfile and synthetic fixtures.

**Spec:** [Approved reset](../../../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md); [master plan](plan.md).

## Global constraints

- Inherit the master plan's global constraints, pins, task dependencies and commit trailer.
- Both providers directly through Rig; no App Server, Copilot CLI/SDK, paid API or account/model fallback.
- Protected application-private file caches; Keychain deferred. No reads of existing Codex/Copilot credentials.
- Live login/calls require separate authorization; deterministic checks cannot accidentally contact a non-loopback endpoint.
- Candidate packages/features and API signatures must compile before they become production choices. Failure evidence is not qualification.

---

## Q1: Resolve the actual dependency graph and pin usable interfaces

**Prerequisite:** Implementation authorization, not live/account authorization.

**Files:**
- Create: `experiments/rig-qualification/Cargo.toml`, `Cargo.lock`, `README.md`.
- Create: `experiments/rig-qualification/src/lib.rs`, `tests/graph.rs`.
- Modify only after evidence: this plan's evidence record and approved dependency choice; no production downgrade.

**Consumes:** Root/store/retrieval/workflow/desktop manifests and locked baseline; tagged Rig manifests and source.

**Produces:** Committed candidate lockfile, `graph.rs` compile tests and G1 evidence recording exact package versions, features, SQLite/ONNX linkage and the chosen direct/adapted companion route.

- [ ] **1. Add the failing compile test** before dependencies. `tests/graph.rs`:

```rust
#[test]
fn candidate_exposes_both_subscription_dialects() {
    assert_eq!(rig::providers::chatgpt::PROVIDER_NAME, "chatgpt");
    assert_eq!(rig::providers::copilot::PROVIDER_NAME, "copilot");
}
```

- [ ] **2. Run the red check:** `cargo test --manifest-path experiments/rig-qualification/Cargo.toml --test graph`. Initially fail because the standalone manifest/dependency is absent; do not call this a protocol test.

- [ ] **3. Add the candidate manifest and graph features:**

```toml
[package]
name = "brn-rig-qualification"
version = "0.1.0"
edition = "2024"
rust-version = "1.98.1"
publish = false

[dependencies]
rig = { version = "=0.43.0", default-features = false, features = ["agent", "derive", "reqwest", "rustls", "memory"] }
brn-workflow = { path = "../../crates/brn-workflow" }
gpui-kit = { version = "=0.6.6", optional = true }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"

[features]
default = []
native-graph = ["brn-workflow/native-retrieval", "dep:gpui-kit"]

[workspace]
```

Use an empty `src/lib.rs` initially. After this manifest change, fetching missing dependencies is permitted; no model downloads or auth. Read published manifests, not README version examples. Use `cargo metadata --manifest-path experiments/rig-qualification/Cargo.toml --format-version 1` to record the resolved graph, then commit the generated lockfile.

- [ ] **4. Run isolated graph selectors independently** and record each exit/result:

```sh
cargo test --manifest-path experiments/rig-qualification/Cargo.toml --locked --test graph
cargo check --manifest-path experiments/rig-qualification/Cargo.toml --locked --features native-graph
cargo check --manifest-path experiments/rig-qualification/Cargo.toml --features rig/sqlite
cargo check --manifest-path experiments/rig-qualification/Cargo.toml --features native-graph,rig/fastembed
```

Optional conflicting candidates are experiments, not required successful combinations. Do not expose rejected companion features on the selected root manifest or invoke `--all-features`. Record each native-linkage diagnostic, then regenerate the lockfile and repeat the chosen graph without the rejected facade feature.

- [ ] **5. Choose the graph explicitly:** keep rusqlite `0.40.2` and FastEmbed `7.1.0` unless a separately reviewed reason changes them. If companions conflict, omit their facade features and use N3's direct SQLite module/N5's Rig local transport adapter. If the base Rig/GPUI graph fails, stop G1 and review a precise patched/new pin; do not proceed on an uncompiled guessed interface.

- [ ] **6. Record source/API proofs:** facade package is `rig`, not the historical `rig-core` alias; agents use `AgentBuilder::new(model)`, `.prompt(...).await` and `.prompt(...).stream()`. Inspect the pinned `Tool`, `AgentHook`, dispatch action, stream terminal and Rig message serialization interfaces. Add compile-only tests for each selected public type. Companion docs with older version examples are not the manifest.

- [ ] **7. Green/commit:** repeat selected graph tests with `--locked`; record limitations in `evidence.md`. Commit experiment manifest/lock/tests and dependency evidence as `test: qualify pinned Rig dependency graph`, with the master trailer. G1 closes only for the selected graph; no provider success claim.

## Q2: Qualify protected OAuth caches and explicit identity

**Prerequisite:** Q1/G1.

**Files:**
- Create: `experiments/rig-qualification/src/auth.rs`, `credentials.rs`, `tests/auth.rs`, `tests/support/mod.rs`.
- Modify: experiment `Cargo.toml`/lockfile for the selected mock transport dependency, `src/lib.rs`, `README.md`.

**Consumes:** Rig's `Authenticator::auth_context` interfaces and private synthetic HTTP transport.

**Produces:** Proven auth/cache behavior used by A1; exact cache-path/identity/error matrix. Harness-only contracts:

```rust
pub enum Provider { ChatGpt, Copilot }
pub struct CacheFiles {
    pub directory: std::path::PathBuf,
    pub chatgpt: std::path::PathBuf,
    pub github: std::path::PathBuf,
    pub copilot: std::path::PathBuf,
}
pub fn prepare_cache(root: &std::path::Path) -> std::io::Result<CacheFiles>;
pub fn verify_cache(files: &CacheFiles) -> std::io::Result<()>;
pub fn remove_cache(files: &CacheFiles) -> std::io::Result<()>;
```

`prepare_cache` creates one caller-selected directory and exact known file names (`chatgpt.json`, `github-token`, `copilot.json`); no provider/default-home discovery. Empty cache files are not fed to JSON readers as valid records: missing cache is absent, malformed existing JSON fails visibly.

- [ ] **1. Write the red permissions test** in `tests/auth.rs`:

```rust
use brn_rig_qualification::credentials::prepare_cache;
use std::os::unix::fs::PermissionsExt;

#[test]
fn private_cache_has_owner_only_permissions() {
    let parent = tempfile::tempdir().unwrap();
    let files = prepare_cache(&parent.path().join("credentials")).unwrap();
    assert_eq!(
        std::fs::metadata(&files.directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    std::fs::write(&files.github, b"synthetic-bootstrap").unwrap();
    assert_eq!(
        std::fs::metadata(&files.github).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
```

Run `cargo test --manifest-path experiments/rig-qualification/Cargo.toml --locked --test auth private_cache_has_owner_only_permissions`; expect missing interface/red assertion.

- [ ] **2. Implement private cache preparation:** use owner-only directory creation and `OpenOptionsExt::mode(0o600)` with exclusive/no-follow opening for text cache files; validate owner, regular single-link file type and directory/file modes on existing paths. Do not modify shared parent-directory modes or change the process-wide umask. Serialize per-account cache access with an owned file lock; don't rely solely on Rig's per-instance mutex.

Rig writes JSON caches with its internal writer. Inspect whether its missing-file create can be made private by the `0700` parent and then narrowed/verified before acknowledgement, without precreating invalid empty JSON or exposing a permissive file to other users. If interruption/privacy tests fail, implement a narrowly reviewed cache-writer fix/adapter; don't claim default writes are atomic or encrypted.

- [ ] **3. Write mocked OAuth tests:** the mock HTTP adapter accepts only declared synthetic requests and rejects all non-loopback transport destinations. It maps the pinned absolute auth URLs to its local fixture without changing production credential routing. Use the real Rig authenticators, with device callbacks collecting synthetic prompts:

```rust
let chat_auth = rig::providers::chatgpt::auth::Authenticator::new(
    rig::providers::chatgpt::auth::AuthSource::OAuth,
    Some(files.chatgpt.clone()),
    rig::providers::chatgpt::auth::DeviceCodeHandler::new(|_| {}),
    false,
);
let copilot_auth = rig::providers::copilot::auth::Authenticator::new(
    rig::providers::copilot::auth::AuthSource::OAuth,
    Some(files.github.clone()),
    Some(files.copilot.clone()),
    rig::providers::copilot::auth::DeviceCodeHandler::new(|_| {}),
    false,
);
```

The no-cache/noninteractive case must fail without issuing device-login requests. Fixtures cover cached valid tokens, expiry/refresh, 401/403 and refresh revocation, malformed/truncated cache, account change, wrong host, cancellation, symlink/hard-link cache, insecure modes and lock contention. A cloned authenticator test does not substitute for a second-process restart test. Configuration constructors can have dialect-specific environment hooks: test that ambient API-base/token/account overrides cannot change the explicit selected credential or allowed destination. Do not mutate process-wide environment while native/async work is running; enforce endpoint/credential policy on constructed config/transport.

- [ ] **4. Exercise actual identity:** ChatGPT `AuthContext.account_id` must be present and match the selected account. Copilot's inspected `AuthContext` exposes session token/base URL, not a stable user ID: qualify an authenticated GitHub identity request using the selected bootstrap credential. A token fingerprint is not account identity. Missing/unverifiable identity returns reconnect-required; do not silently label a cache/account as matched.

- [ ] **5. Test cache failure honestly:** kill a child at cache-write boundaries, reopen, and assert either valid selected identity or explicit reconnect-required. No truncated JSON default, account switch or automatic login. Test double disconnect and a symlink substituted before deletion; remove only owned validated cache entries, not a directory tree.

- [ ] **6. Green/commit:** `cargo test --manifest-path experiments/rig-qualification/Cargo.toml --locked --test auth`; repeat with separate child processes. Record the selected cache strategy and identity interface. Commit as `test: qualify protected Rig subscription authentication`. No live login is implied.

## Q3: Qualify agents, streams, tools and provider parity

**Prerequisite:** Q2 plus reviewed exact hook/stream types from Q1.

**Files:**
- Create: `experiments/rig-qualification/src/{agent.rs,main.rs}`.
- Create: `experiments/rig-qualification/tests/{agent.rs,stream.rs,live.rs}`.
- Create: `experiments/rig-qualification/fixtures/{chatgpt,copilot}/` containing only synthetic scrubbed replay fixtures.
- Modify: experiment manifest/lock/README; task evidence.

**Consumes:** Q2 auth/cache, pinned completion configurations and synthetic tool implementations.

**Produces:** G2 matrix and verified production primitives for A1. `main` supports `replay`, `live-connect` and `live-run`; live modes require explicit provider/model/cache-directory arguments and an explicit confirmation flag. No `from_env()` or ambient fallback.

- [ ] **1. Write two-provider deterministic tests** before the agent wrapper. Define a harness `Probe` with `text: String`, `tool_calls: usize`, `model_calls: usize`, `terminal_received: bool`, `typed_valid: bool`, and `dispatches: usize`. Implement `run_replay(provider: Provider, scenario: &str) -> Result<Probe, String>`; every named scenario must have a committed fixture and complete-consumption assertion.

```rust
use brn_rig_qualification::{Provider, agent::run_replay};

#[tokio::test]
async fn both_providers_stream_a_tool_turn_to_a_terminal() {
    for provider in [Provider::ChatGpt, Provider::Copilot] {
        let result = run_replay(provider, "adder-stream").await.unwrap();
        assert_eq!(result.tool_calls, 1);
        assert!(result.terminal_received);
        assert!(result.text.contains('5'));
        assert!(result.dispatches >= 2);
    }
}
```

Run `cargo test --manifest-path experiments/rig-qualification/Cargo.toml --locked --test agent`; expect missing wrapper/fixtures.

- [ ] **2. Build ChatGPT config from the resolved credential, not a paid API client:**

```rust
let mut config = rig::providers::openai::OpenAIConfig::with_key(
    &rig::providers::chatgpt::DIALECT,
    auth.access_token,
);
if let Some(id) = auth.account_id {
    config = config.with_account_id(id);
}
```

For Copilot use `CopilotConfig::from_auth(&auth)` and the pinned `.connect(http)` completion interface. Use `AgentBuilder::new(client.completion(model))`, a synthetic typed adder tool and `.prompt("Add 2 and 3 using the tool.").stream()`. Compile the selected tool implementation against Q1's exact trait, not examples from older releases.

- [ ] **3. Cover the scenario matrix** with real fixture consumption:

| Scenario | Required assertion |
| --- | --- |
| Completion/stream | Nonempty output; exactly one accepted terminal; text deltas provisional before it. |
| Multi-turn/restart | Serialized canonical messages decode after a child-process restart and answer a synthetic follow-up. |
| Tools | Valid call/result pairing; denied/unknown tool makes no workflow effect; arguments/results bounded. |
| Typed output | `.prompt_typed::<ProbeArtifact>` produces deserialized validated content; refusal/invalid JSON is not success. |
| Hooks | Dispatch can pause/deny before HTTP; outcome callback records the real terminal, not token completion. |
| Network loss | Loss before/after dispatch has different known outcome; no automatic second request. |
| Cancellation | Before dispatch means no request; after dispatch/drop means remote unknown unless explicitly confirmed. |
| Quota/auth/model refusal | Sanitized typed failure; no alternate provider/model/account/API call. |
| Budgets | Model/tool/output limits stop the run explicitly, including a repeated-tool loop. |

`ProbeArtifact` is a local `#[derive(Serialize, Deserialize, schemars::JsonSchema)]` struct with `sum: i64` and `explanation: String`; add `schemars = "1"` to the experiment manifest before using it, subject to Q1's compiled schema-type compatibility.

- [ ] **4. Add replay-safe recording:** prefer rig-cassette with defaults disabled and only the chosen HTTP/agent features. Default mode is forced replay; ignore environment requests for recording unless a live subcommand was explicitly authorized. Synthetic cassette bodies can include public synthetic note text, never tokens, device codes, real user paths or account IDs. Fixture replay completion must fail for unused/mismatched requests; stream-order retention must be enabled when tests assert event order.

- [ ] **5. Run deterministic green checks:**

```sh
cargo test --manifest-path experiments/rig-qualification/Cargo.toml --locked
cargo check --manifest-path experiments/rig-qualification/Cargo.toml --locked --features native-graph
```

Commit as `test: qualify direct Rig agent and stream parity`.

- [ ] **6. Stop for live authorization.** Once expressly authorized, use `cargo run --manifest-path experiments/rig-qualification/Cargo.toml --locked -- live-connect` and `live-run` with the complete explicit arguments from the harness help. Open the device prompt only for Connect; never capture it into the evidence log. Exercise each selected provider/model/account with synthetic inputs, restart, refresh/reconnect, tools and typed output. Record exact commands without secrets, outcomes and unobserved cases. No blanket all-model sweep.

- [ ] **7. Close G2 only if both paths pass.** Record auth expiry/revocation limitations separately from deterministic injected failures. If live authorization is absent, matrix cells are blocked, not passed, and A1 production replacement does not proceed under this plan. Technical completion does not settle distribution support/terms risks.
