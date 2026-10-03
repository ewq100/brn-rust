# Step 4: AI Chat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Codex App Server chat with explicit ChatGPT/Copilot subscription connections through Rig, streamed read-only vault chat, durable local conversations, and the same workflow from CLI and desktop.

**Architecture:** Add a thin `brn-ai` module for Rig authentication, agent construction, tools and safe event mapping. A workflow `AppWorker` owns `App`, WorkStore and Library off the GUI thread; an independent `ChatWorker` runs Tokio. Both search callers share one local embedder and one search policy. Authentication locks protect provider credential operations, not whole streams. CLI and desktop call workflow interfaces only; remove `brn-provider` after both consumers switch.

**Tech Stack:** Rust `1.98.1`; Rig, rig-core and rig-reqwest `=0.43.0`; rusqlite `=0.40.2` with `bundled`/`backup`; Tokio; futures; serde; uuid; existing fastembed `=7.1.0` and GPUI-kit `=0.6.6`.

**Spec:** [Approved simple notes design](../../../superpowers/specs/2026-10-02-simple-rig-notes-design.md), especially sections 2, 3, 7 and 9; [roadmap](plan.md); [spike findings](../../../../experiments/rig-spike/FINDINGS.md); [search plan](search.md).

**Status:** Written 2026-10-02 against `main@99013df`; plan reviewed by Claude Opus 5.5 with high reasoning and revised against `main@e24b104`. Implemented on `task-4-ai-chat` through `dccc24a` and merged on `main@50f898a` through PR #14 on 2026-10-03. All eight tasks and the whole branch received Opus medium reviews, including final scoped fix approval. [Evidence](evidence.md) records actual checks and remaining qualification. Native/live/user acceptance remains pending.

**Execution:** The user subsequently authorized implementation on 2026-10-02 and approved isolated worktree/branch `task-4-ai-chat`: GPT-6.1 Sol (medium) implemented, Claude Opus 5.5 (medium) reviewed. Bounded implementation and offline verification are complete; PR publication and merged-branch cleanup were separately requested on 2026-10-03. Live account/provider checks, model downloads, original-data migration, release and native/user acceptance remain unauthorized or pending. The checklist below is the reviewed execution specification, not evidence that live or graphical observations occurred.

Step 3 now includes the replacement LocalEmbedder (`180451f`), Library (`9e6a735`), keyword-only/frontmatter fix (`94de72d`), recorded evidence (`b0d6b17`) and index/embedding consistency fixes (`e24b104`). Preserve the latter's foreign-file checks, single-statement semantic results, unchanged-passage/model insertion checks and five-asset identity. Those are existing inputs, not changes made by this planning task. Their local-model check was skipped without assets; no new inference/live qualification is claimed.

## Global Constraints

- The vault folder is the truth for notes; preserve exact bytes. The AI cannot write files.
- Only regular `.md` notes up to 1 MiB are visible; exclude symlinks, hidden paths and top-level `archive/`.
- No automatic fallback between providers, models or accounts. No automatic retry of failed or interrupted turns.
- Normal use only refreshes saved credentials. Device login is allowed only by an explicit Connect action.
- Credentials live outside the repository in a real, current-user-owned `0700` folder next to the data folder. Refuse unsafe folders; never silently repair another owner's folder.
- Rig caches must be regular, single-link, current-user-owned files; set `0600` after login and refresh and validate before reuse. Do not follow cache-file symlinks.
- No tokens, raw HTTP bodies or device codes in SQLite, conversation text, error messages or logs. The code is shown only on the transient, explicit login surface.
- `brn.sqlite` holds user work; preserve its application ID, integrity/backup behavior and owner lock. `index.sqlite` is disposable.
- Use a new explicit data folder. Both stores check mutually exclusive database/sidecar markers **after acquiring the shared owner lock and before opening a database**; frontend checks alone are not authoritative. Never migrate or modify old work.
- Default checks are deterministic, offline and synthetic. Model downloads and live provider calls require a separate user request.
- History contains at most the last 20 earlier user/assistant turn pairs. Never resend earlier tool calls or results, reasoning blocks, provider response IDs or old note text as tool history.
- `search_notes`: at most 10 results; `read_note`: at most **50,000 UTF-8 bytes** plus a separate truncation flag; `list_notes`: at most 200 rows/page; at most 8 tool-containing model rounds/answer.
- Retain local partial text on failure/Stop. Dropping a stream is local cancellation, not proof of server cancellation or no billing.
- Use the pinned toolchain and lockfile. Update the lockfile once after dependency edits, then use `--locked`.
- Implementation commits include `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.

## Inputs, gates and decisions

| Input | Verified planning observation | Consequence |
| --- | --- | --- |
| Step 1 | Copilot tool/stream/restart/local-cancel observed; ChatGPT login/routing worked but chat was quota-blocked | Offline work can proceed for both. ChatGPT live acceptance remains blocked until a separately authorized recheck after **2026-10-03 20:24 EEST**, using an explicitly selected supported model such as `gpt-5.5`. The time alone is not authorization. |
| Rig graph | Facade `rustls` pulled incompatible rig-fastembed/ORT versions | Copy the spike's facade `agent`/`derive`/`reqwest` features and direct rig-core/rig-reqwest `rustls`, not the original spec's facade TLS suggestion. |
| Step 2 | `WorkStore` and vault modules exist; work schema is **V1**, separate from legacy Store V6 | Append work migration **V2**. Do not edit legacy V6 or manufacture a legacy migration. |
| Step 3 | NoteIndex, Library, LocalEmbedder, LanceDB removal and `94de72d` fixes are committed; evidence recorded at `b0d6b17` | Input is available. Reuse the current Library contract: **every search** reports `keyword_only` when no model is installed, and titles skip frontmatter. Do not substitute the legacy Index or copy outdated code from the historical search plan. |
| CLI handoff | Search plan deliberately deferred `notes list/show` and new `search` CLI wiring to Step 4 | Include those read commands here so tools and humans use the same vault/search rules. Save replacement remains Step 6. |
| Existing worker | `Worker` executes long `Workspace::ask` on the same lane as note operations | A second owned chat lane is necessary; changing the provider call alone does not satisfy non-blocking saves. |
| Transition | Existing local editor/drafts/import code stays until Step 6; provider removal is explicitly Step 4 | Preserve local commands and safety checks. Retire legacy AI submission explicitly; retain legacy history through `brn-flow sessions/history`. Do not silently send old provider threads to Rig. |

**Recommended approach:** new simple-app workflow beside the legacy workflow, then a deliberate consumer cutover and provider deletion. Rewriting `Workspace` in place would couple this step to legacy save/operation cleanup; keeping two production chat engines indefinitely would defeat the Step 4 removal requirement. Neither is part of this plan.

### Execution gates

1. Re-read HEAD, dirty changes and Step 3 evidence before implementation: this plan describes an observed baseline, not a guarantee that another contributor has not changed the inputs.
2. Confirm the Step 3 implementation/evidence and `94de72d` fix are still included in the execution branch. They are now present, not an unresolved implementation blocker; actual model inference remains separately qualified.
3. Verify both subscription request formats offline using Rig's recording/synthetic HTTP test clients. The spike's cassette feature compilation is **not** replay acceptance.
4. Do not mark ChatGPT live-qualified or the native UI accepted based on mocks/builds. Record those outcomes separately.

## File map and dependency order

| Task | Files | Responsibility |
| --- | --- | --- |
| 1 | New `crates/brn-ai/{Cargo.toml,README.md,src/lib.rs,src/error.rs,src/auth.rs}`; root `Cargo.toml`, `Cargo.lock` | Provider/selection DTOs, safe errors, explicit authentication and credential lifecycle |
| 2 | New `crates/brn-store/src/work/chat.rs`, `crates/brn-store/tests/work_chat.rs`; `work/mod.rs` | V2 conversations/messages, terminal transactions, restart/history/replay |
| 3 | New `crates/brn-ai/src/{chat.rs,tools.rs,provider_formats_tests.rs}`; `brn-ai` manifest/lib | Thin Rig agent, read-tool interface, stream/error/round policy, synthetic provider-format tests |
| 4 | New workflow `src/{app.rs,ai_tools.rs,models.rs}`, `tests/{app,ai_tools,models}.rs`; existing `library.rs`; retrieval `note_index/{mod,search,embeddings,schema}.rs`, `src/native/download.rs`, `tests/model_download.rs`; retrieval/store manifests | App, shared embedder/search policy, read-only index connection, lock-protected folder exclusion, consented model installation |
| 5 | New workflow `src/{app_worker.rs,chat_worker.rs}`, `tests/{app_worker,chat_worker}.rs`; workflow manifest/lib/error; store `work/mod.rs` and chat module | Owned application/chat lanes, attached-lock lifetime, durable finalization, cancellation and correlated events |
| 6 | New `crates/brn/src/cli/{ai,library}.rs`, `tests/{cli_ai,cli_library}.rs`; existing CLI/main/ask/error/status/tests | CLI cutover with one JSON envelope, connection commands, ask and saved history |
| 7 | New `crates/brn-desktop/src/ai.rs`; existing native shell/settings/history/centre/vault/header, native/main, desktop manifest/tests | Provider/model controls, explicit login, streamed chat, Stop and restart; preserve editor guards |
| 8 | Provider crate and workspace consumers/tests/scripts/docs | Remove App Server and its configuration, qualify integrated offline/native builds and record handoff |

Tasks **1 and 2 are independent**. Task 3 needs 1; Task 4 needs 1-3 and the committed Step 3 interfaces; Task 5 needs 4. Tasks 6 and 7 need 5; Task 8 needs both. Treat each task as a test/review/commit slice, not a new PR or permission to push.

## Shared interfaces

The following are **proposed interfaces**, not claims that these APIs exist today. Keep them identical across tasks. All public wire DTOs derive serde serialization; provider/model strings are validated at the workflow entry point.

### `brn-ai` (Tasks 1 and 3)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider { Chatgpt, Copilot }

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Selection { pub provider: Provider, pub model: String }

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiErrorKind {
    ReconnectNeeded, CodeExpired, RateLimited, Network, ModelRefused,
    InvalidToolUse, ToolLimitReached, UnsafeCredentials, ToolRejected, IndexStale,
    Storage, Other,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AiError {
    pub kind: AiErrorKind,
    pub retry_after_seconds: Option<u64>,
}
pub type AiResult<T> = std::result::Result<T, AiError>;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AccountStatus { pub provider: Provider, pub name: Option<String>, pub connected: bool }
#[derive(Clone)]
pub struct LoginPrompt { pub verification_uri: String, pub user_code: String }
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ModelOption { pub id: String, pub live_qualified: bool }

pub struct Auth { /* fixed provider slots, checked paths and per-provider Tokio mutexes */ }
pub struct ProviderClient { /* owned selected Rig client; no Auth borrow, no Debug */ }
impl Auth {
    pub fn open(credentials_dir: &std::path::Path) -> AiResult<Self>;
    pub async fn status(&self, provider: Provider) -> AiResult<AccountStatus>;
    pub async fn connect(
        &self, provider: Provider,
        login: std::sync::Arc<dyn Fn(LoginPrompt) + Send + Sync>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> AiResult<AccountStatus>;
    pub async fn client(
        &self, selection: &Selection, cancel: tokio_util::sync::CancellationToken,
    ) -> AiResult<ProviderClient>;
    pub async fn disconnect(&self, provider: Provider) -> AiResult<()>;
    pub async fn models(
        &self, provider: Provider, cancel: tokio_util::sync::CancellationToken,
    ) -> AiResult<Vec<ModelOption>>;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HistoryPair { pub question: String, pub answer: String }
#[derive(Clone, Debug)]
pub enum AiEvent { Text(String), ToolStarted { name: String } }
#[derive(Clone, Debug)]
pub enum AiTerminal { Completed, Interrupted, Failed(AiError) }
#[derive(Clone, Debug)]
pub struct AiAnswer { pub text: String, pub terminal: AiTerminal }

pub async fn answer(
    client: ProviderClient, question: &str, history: &[HistoryPair],
    tools: std::sync::Arc<dyn ReadTools>, cancel: tokio_util::sync::CancellationToken,
    emit: std::sync::Arc<dyn Fn(AiEvent) + Send + Sync>,
) -> AiAnswer;
```

`AiError::Display` chooses static safe copy by kind and an optional validated reset delay. It contains no raw error, response body, source chain or device code. `AiAnswer.text` is the exact text emitted once, not `delta_text + FinalResponse.output()` duplicated.

`Auth` is held in an Arc. A provider mutex covers cache read/refresh, permission tightening and construction of an owned client; it is released before model streaming or model-list network waits. Explicit device login holds only that provider's slot and is cancellable; the other provider remains available. Rig authenticate captures a credential into its client once, so `ProviderClient` owns the selected concrete Rig client/model binding without retaining Auth. This is a closed ChatGPT/Copilot enum, not a general provider service.

`ChatWorker` keeps handling commands while a chat/login future runs; it never awaits the entire stream in its command-dispatch loop. Disconnect first fences new operations for the target provider, cancels and awaits any active login/refresh/model-list/turn for it, finalizes the local turn, then acquires that provider's slot and removes caches. Drop the owned client before replying Disconnected. Reconnection is explicit; cancel alone is not Disconnect.

The injected tool seam is **`ReadTools`**: a real workflow adapter and deterministic test adapter both use it. Keep transport injection private to `brn-ai`; do not add a dynamic provider registry or another DTO layer around Rig.

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Passage {
    pub path: String, pub start_byte: usize, pub end_byte: usize, pub quote: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolSearch { pub hits: Vec<Passage>, pub keyword_only: bool }
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolNote { pub path: String, pub text: String, pub truncated: bool }
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NoteEntry { pub path: String, pub title: String }
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NotePage { pub notes: Vec<NoteEntry>, pub next_cursor: Option<String> }

pub trait ReadTools: Send + Sync {
    fn search_notes(&self, query: &str, limit: usize) -> AiResult<ToolSearch>;
    fn read_note(&self, path: &str) -> AiResult<ToolNote>;
    fn list_notes(&self, folder: Option<&str>, cursor: Option<&str>) -> AiResult<NotePage>;
}
```

`brn-ai` contains Rig tool schemas/definitions; workflow implements `ReadTools`. `brn-ai` does **not** depend on workflow, store or retrieval. Blocking file/index work is dispatched with `tokio::task::spawn_blocking` inside tool calls; no rusqlite connection is held over an `.await`.

### WorkStore (Task 2)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkTurnStatus { Running, Completed, Interrupted, Failed }
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkConversation { pub id: uuid::Uuid, pub title: String, pub turns: usize }
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkTurn {
    pub id: uuid::Uuid, pub conversation_id: uuid::Uuid,
    pub question: String, pub answer: String, pub provider: String, pub model: String,
    pub status: WorkTurnStatus, pub error_code: Option<String>,
}
impl WorkStore {
    pub fn conversations(&self) -> brn_store::Result<Vec<WorkConversation>>;
    pub fn turns(&self, conversation: uuid::Uuid) -> brn_store::Result<Vec<WorkTurn>>;
    pub fn begin_turn(
        &mut self, id: uuid::Uuid, conversation: Option<uuid::Uuid>,
        question: &str, provider: &str, model: &str,
    ) -> brn_store::Result<WorkTurn>;
    pub fn finish_turn(
        &mut self, id: uuid::Uuid, status: WorkTurnStatus,
        answer: &str, error_code: Option<&str>,
    ) -> brn_store::Result<WorkTurn>;
}
```

WorkStore remains independent of `brn-ai`; only safe strings reach persistence. UUID `id` is the CLI operation identity and local turn identity. Identical replay returns its recorded result without a new request; conflicting payload reuse fails. A prior `Running` turn is not resubmitted.

### Workflow (Tasks 4 and 5)

```rust
pub struct AppConfig {
    pub vault_root: Option<std::path::PathBuf>,
    pub credentials_dir: std::path::PathBuf,
    pub model_dir: Option<std::path::PathBuf>,
}
pub struct App { /* WorkStore, optional Library, config */ }
impl App {
    pub fn open(data_dir: &std::path::Path, config: AppConfig) -> crate::Result<Self>;
    pub fn bind_vault(&mut self, root: &std::path::Path) -> crate::Result<()>;
    pub fn select(&mut self, selection: brn_ai::Selection) -> crate::Result<()>;
    pub fn selection(&self) -> crate::Result<Option<brn_ai::Selection>>;
    pub fn notes(
        &mut self, folder: Option<&str>, cursor: Option<&str>,
    ) -> crate::Result<brn_ai::NotePage>;
    pub fn note(&self, path: &str) -> crate::Result<crate::vault::NoteText>;
    pub fn search(
        &mut self, query: &str, mode: crate::library::SearchMode, limit: usize,
    ) -> crate::Result<crate::library::SearchResults>;
    pub fn conversations(&self) -> crate::Result<Vec<brn_store::work::WorkConversation>>;
    pub fn turns(&self, conversation: uuid::Uuid) -> crate::Result<Vec<brn_store::work::WorkTurn>>;
}

#[derive(Clone, Debug)]
pub struct AskRequest {
    pub id: uuid::Uuid, pub conversation: Option<uuid::Uuid>,
    pub question: String, pub selection: brn_ai::Selection, pub generation: u64,
}
#[derive(Clone, Debug)]
pub enum ChatEvent {
    Text { id: uuid::Uuid, generation: u64, text: String },
    ToolStarted { id: uuid::Uuid, generation: u64, name: String },
    Finished { id: uuid::Uuid, generation: u64, turn: brn_store::work::WorkTurn },
    PersistenceFailed { id: uuid::Uuid, generation: u64, partial: String },
}
pub struct ChatWorker { /* owned command channel, event channel, JoinHandle */ }
impl ChatWorker {
    pub fn start(app: &App) -> crate::Result<Self>;
    pub fn set_tools(&self, tools: std::sync::Arc<dyn brn_ai::ReadTools>) -> crate::Result<()>;
    pub fn ask(&self, request: AskRequest) -> crate::Result<()>;
    pub fn cancel(&self, id: uuid::Uuid) -> bool;
    pub fn try_event(&self) -> Option<ChatEvent>;
    pub fn shutdown(&mut self) -> crate::Result<()>;
}
```

`bind_vault` is first-binding only: canonicalize/validate, initialize Library and persist the root after successful initialization; rebinding an already bound root to another vault fails. Ask requires a bound/available vault before any completion request. Settings/account status/history remain usable without one.

### Application lane (Task 5)

The desktop owns **only** this workflow handle, not App. CLI may use App directly for synchronous local reads on its non-GUI command thread, but Ask/account/download commands use AppWorker so they cannot bypass refresh, fencing or owned-job shutdown. `AppWorker` creates/owns App and starts/owns ChatWorker before exposing Ready.

```rust
pub enum AppCommand {
    BindVault(std::path::PathBuf), Refresh,
    Selection, Select(brn_ai::Selection),
    Notes { folder: Option<String>, cursor: Option<String> },
    Note(String),
    RecoverEdit { path: String, base_sha256: [u8; 32], text: String },
    Search { query: String, mode: crate::library::SearchMode, limit: usize },
    Conversations, Turns(uuid::Uuid),
    Ask(AskRequest), CancelTurn(uuid::Uuid),
    Account { id: uuid::Uuid, command: AccountCommand },
    CancelAccount(uuid::Uuid),
    DownloadModel { consent: bool, target: std::path::PathBuf },
    CancelModelDownload(uuid::Uuid),
}
pub enum AppEvent {
    Ready { vault_bound: bool, model_installed: bool },
    Restored { backup: std::path::PathBuf },
    VaultBound, Selection(Option<brn_ai::Selection>), SelectionSaved,
    Refreshed(crate::library::RefreshReport),
    Notes(brn_ai::NotePage), Note(crate::vault::NoteText), EditRecovered,
    Search(crate::library::SearchResults),
    Conversations(Vec<brn_store::work::WorkConversation>),
    Turns(Vec<brn_store::work::WorkTurn>),
    Indexing { embedded: usize, total: usize },
    Chat(ChatEvent), Account(AccountEvent),
    ModelDownload { received: u64, total: u64 },
    ModelDownloaded, ModelInstalled, ModelDownloadDeclined,
    Failed(crate::WorkflowError),
}
pub struct AppWorker { /* command/event channels and owned JoinHandle */ }
impl AppWorker {
    pub fn start(data_dir: std::path::PathBuf, config: AppConfig) -> crate::Result<Self>;
    pub fn submit(&self, id: uuid::Uuid, command: AppCommand) -> crate::Result<()>;
    pub fn try_event(&self) -> Option<(uuid::Uuid, AppEvent)>;
    pub fn shutdown(&mut self) -> crate::Result<()>;
}
```

All command results echo the submission UUID; indexing events use a separate startup/refresh job UUID. Background embedding runs bounded batches between commands, never a loop holding the lane until the whole vault finishes. Chat/account events are forwarded independently, so active network waits never block application commands. Loading/hash scans/inference do not run on GPUI. Model downloads use a cancellable background install job; this lane only processes progress/installation results and swaps model state after success.

ChatWorker can start without a bound vault to support account/settings commands. It rejects Ask locally until `set_tools` receives the adapter after successful binding. Replacing tools is idle-only, and model activation waits for that idle point; no live turn's tool snapshot is swapped underneath it.

Workflow re-exports safe AI DTOs; CLI/desktop add **no direct** `brn-ai`, Rig, retrieval or store dependency. Add these authentication commands to the same owned chat lane:

```rust
pub enum AccountCommand {
    Connect(brn_ai::Provider), Disconnect(brn_ai::Provider),
    Status(brn_ai::Provider), Models(brn_ai::Provider),
}
pub enum AccountReply {
    Status(brn_ai::AccountStatus), Disconnected,
    Models(Vec<brn_ai::ModelOption>), Cancelled, Failed(brn_ai::AiError),
}
pub enum AccountEvent {
    Login { id: uuid::Uuid, prompt: brn_ai::LoginPrompt },
    Finished { id: uuid::Uuid, reply: AccountReply },
}
impl ChatWorker {
    pub fn account(&self, id: uuid::Uuid, command: AccountCommand) -> crate::Result<()>;
    pub fn cancel_account(&self, id: uuid::Uuid) -> bool;
    pub fn try_account_event(&self) -> Option<AccountEvent>;
}
```

Login prompts use this separate transient channel, never the chat event/history channel. Cancel/closing the explicit Connect surface cancels that operation, clears its code and tightens any files already written; it does not start a new login. Do not derive serialization or Debug for raw login prompts or events that contain them.

The exact pinned upstream interfaces inspected while planning are Rig's
[auth callback](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-core/src/providers/internal/auth.rs),
[prompt history/budget](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-agent/src/agent/runner.rs),
[model-turn hook](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-agent/src/agent/hook.rs)
and [mock model](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-core/src/test_utils/completion.rs).
These reads confirm names and semantics, not compilation of the proposed product interfaces.

---

## Task 1: Explicit accounts, selections and safe errors

**Files:** Task 1 row in the file map; tests inside `auth.rs` and `error.rs`.

**Consumes:** spike authenticator/client constructors, verified Rig `DeviceCodeHandler::new` callback. **Produces:** Task 1 `brn-ai` DTOs, `Auth` and safe error mapping.

- [ ] **1. Add failing credential/error tests before creating auth implementation.** Use synthetic caches only. Cover mode `0755`, wrong owner (metadata-validator unit fixture, not chown), folder/file symlinks, hard-linked files, expired/malformed caches, missing cache and provider-specific deletion. Capture safe rendered errors for all synthetic HTTP cases.

```rust
#[test]
fn safe_errors_never_retain_raw_payloads() {
    let body = r#"{"error":{"code":"usage_limit_reached",
        "resets_in_seconds":42},"access_token":"SYNTHETIC_SECRET"}"#;
    let error = map_http_error(429, body);
    assert_eq!(error.kind, AiErrorKind::RateLimited);
    assert_eq!(error.retry_after_seconds, Some(42));
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains("SYNTHETIC_SECRET"));
    assert!(!rendered.contains("access_token"));
}
```

`map_http_error(status: u16, body: &str) -> AiError` is private to `error.rs`: parse only allowlisted codes and a bounded unsigned reset delay; discard the body. Typed status 401/403 means reconnect, 429 means rate limit, explicit unsupported/refused-model codes mean ModelRefused, response-less transport errors mean Network; unknown errors use Other. Parse the JSON shape actually seen by the spike without displaying unknown strings. Storage/permission and login-expiry errors remain distinct.

- [ ] **2. Run red:** `cargo test -p brn-ai --lib --locked`. Before the crate is added the missing-package failure is expected; repeat after adding the manifest so the new assertions are seen failing, rather than counting package discovery as the only red.
- [ ] **3. Add the manifest and root member.** Use this dependency graph; no `sqlite`, `fastembed` or facade TLS features:

```toml
[package]
name = "brn-ai"
version = "0.1.0"
edition = "2024"
rust-version = "1.98.1"
publish = false

[dependencies]
rig = { version = "=0.43.0", default-features = false, features = ["agent", "derive", "reqwest"] }
rig-core = { version = "=0.43.0", default-features = false, features = ["rustls"] }
rig-reqwest = { version = "=0.43.0", default-features = false, features = ["rustls"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }
tokio-util = { version = "0.7", features = ["rt"] }
futures = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
libc = "=0.2.189"
base64 = "0.22"

[dev-dependencies]
tempfile = "3"
rig = { version = "=0.43.0", default-features = false, features = ["test-utils"] }
```

- [ ] **4. Implement Auth with explicit cache paths and two modes.** `connect` constructs authenticators with `allow_device_flow=true` and `DeviceCodeHandler::new`; `client`/models use `false`. Use per-provider Tokio mutexes and cancellable futures; streaming consumes an owned ProviderClient without retaining the guard. Test that Disconnect is admitted while a fake stream is pending, and that Copilot login/status can progress during ChatGPT login. Never call default device-code printing. The callback emits only URL/code to the active login surface. Show a code-expired Retry action, but do not retry on its own.
- [ ] **5. Harden cache writes and Disconnect.** The checked folder is `0700` even though Rig initially creates files as `0644`. Validate known cache paths before every operation; after each login/refresh success, cancellation **or error**, tighten any files created to `0600` before returning. A tightening failure is UnsafeCredentials, never a successful connection. Fence/cancel/join target-provider work before deleting caches, and discard its client/authenticator state; do not recreate the whole Auth and disrupt the other provider. Delete ChatGPT `chatgpt.json`, or Copilot `github-token` and `copilot.json`, plus provider-specific display-name metadata only. Test no in-flight refresh recreates deleted caches.
- [ ] **6. Implement account/model behavior.** ChatGPT account name comes from the cached `id_token` email claim, decoded for display only, not as authorization proof. Copilot name comes from the authenticated GitHub `/user` request; use Rig's HTTP transport so the same call can be captured offline. Keep display metadata in a provider-specific `0600` file inside the credentials folder, not SQL. `status` is offline and means local credentials present, not guaranteed upstream validity. Missing identity shows an explicit "account name unavailable" state.
- [ ] **7. Model selection:** ChatGPT has a small maintained subscription list, initially `gpt-5.5` with `live_qualified=false`; never call its broken `list_models`. Copilot uses explicit authenticated model discovery; show discovery failure, do not silently substitute a list or another model. Selection is explicit, stored atomically as one JSON setting `ai.selection`, and frozen in each turn. Never preselect a "working" provider after failure.
- [ ] **8. Run green and commit the bounded slice:** `cargo test -p brn-ai --lib --locked`; `cargo tree --locked -i libsqlite3-sys`; `cargo tree --locked -i ort`; `cargo tree --locked -d --depth 0`. Confirm one SQLite native link and no rig-fastembed/ORT rc.9. Update the lockfile with one unlocked Cargo check after adding dependencies, then return to locked commands. Stage only this task's files and include the required trailer.

## Task 2: Durable conversations and bounded text-only history

**Files:** Task 2 row. **Consumes:** WorkStore V1, uuid/serde already in store. **Produces:** `WorkConversation`, `WorkTurn`, lifecycle methods and migration V2.

- [ ] **1. Add failing store tests** for a terminal commit, terminal failure/partial text, restart of Running, last-20 history, UUID replay/conflict, missing conversation and atomic selection update.

```rust
#[test]
fn interrupted_turn_survives_restart_without_resubmission() {
    use brn_store::work::{WorkStore, WorkTurnStatus};
    let dir = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let running = store.begin_turn(id, None, "question", "copilot", "gpt-4o").unwrap();
    let ended = store.finish_turn(id, WorkTurnStatus::Interrupted, "partial", None).unwrap();
    assert_eq!(ended.answer, "partial");
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    let turns = store.turns(running.conversation_id).unwrap();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].status, WorkTurnStatus::Interrupted);
    assert_eq!(turns[0].provider, "copilot");
}
```

- [ ] **2. Run red:** `cargo test -p brn-store --test work_chat --locked`.
- [ ] **3. Append V2, leaving V1 unchanged.** Use normalized conversations/messages; each user/assistant pair shares the operation UUID and increasing conversation sequence. Never persist a Rig Message/AgentRun wholesale.

```sql
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);
CREATE TABLE messages (
    turn_id TEXT NOT NULL,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user','assistant')),
    text TEXT NOT NULL,
    provider TEXT NOT NULL CHECK(provider IN ('chatgpt','copilot')),
    model TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('running','completed','interrupted','failed')),
    error_code TEXT,
    PRIMARY KEY(turn_id, role),
    UNIQUE(conversation_id, sequence, role)
);
CREATE INDEX messages_conversation ON messages(conversation_id, sequence);
```

- [ ] **4. Implement transactional begin/finish.** For a new turn, `conversation=None` allocates a new conversation UUID; `Some(id)` must already exist or returns typed NotFound before any insert. Insert both message rows in one transaction; validate nonblank question/model and provider allowlist. Check an existing turn UUID first: compare exact question/provider/model and supplied conversation if any, then return its recorded result; conflicts fail. `finish_turn` updates both rows atomically, sets assistant partial/final text and a safe allowlisted error code, updates title from the first question, and forbids reverting a terminal row to Running. Test an unknown Some(id) leaves both tables unchanged.
- [ ] **5. Restart reconciliation:** after migration, before startup backup, mark any Running message pairs Interrupted without resubmission. Process-crash partial text is only whatever was durably stored; this step promises terminal persistence, **not per-token crash recovery**.
- [ ] **6. Keep persistence independent of Rig.** `turns` returns all locally stored turns in sequence order; history truncation belongs to Task 5. Test 21 stored turns survive restart without losing the oldest local history. Validate provider/model on both message rows and enforce one user/assistant pair per turn. No tool-results, raw Rig messages or response metadata have columns in V2.
- [ ] **7. Green and commit:** `cargo test -p brn-store --test work --test work_chat --locked`. Add V1-to-V2 preservation, restored-V1-backup upgrade, newer-schema refusal and backup-pruning regressions to `work_chat.rs`; do not weaken existing `work` tests.

## Task 3: Thin Rig streaming and exactly eight tool rounds

**Files:** Task 3 row. **Consumes:** Task 1 Auth/DTOs, the `ReadTools` interface above. **Produces:** `answer`, read tools, safe stream mapping and offline provider request coverage.

- [ ] **1. Write failing policy tests** for UTF-8 truncation, query/page validation, first stream error, no FinalResponse, no duplicated final output, Stop after partial text and round accounting. Rig's mock model is real test support, not a production "fake provider" switch.

```rust
#[test]
fn eighth_tool_round_is_allowed_and_ninth_is_refused() {
    let mut budget = ToolRounds::default();
    for _ in 0..8 { assert!(budget.admit(true).is_ok()); }
    assert_eq!(budget.admit(true).unwrap_err().kind, AiErrorKind::ToolLimitReached);
    assert!(budget.admit(false).is_ok());
}

#[test]
fn note_cap_preserves_utf8_and_exact_prefix() {
    let text = format!("{}\u{00e9}", "x".repeat(49_999));
    let (prefix, truncated) = capped_text(&text);
    assert!(truncated);
    assert_eq!(prefix.len(), 49_999);
    assert_eq!(prefix, &text[..49_999]);
}
```

- [ ] **2. Red:** `cargo test -p brn-ai --lib --locked`.
- [ ] **3. Implement small pure helpers in `tools.rs`.** No byte slicing inside a code point, no whitespace/BOM/CRLF normalization:

```rust
pub const READ_NOTE_BYTES: usize = 50_000;
pub fn capped_text(text: &str) -> (&str, bool) {
    let mut end = text.len().min(READ_NOTE_BYTES);
    while !text.is_char_boundary(end) { end -= 1; }
    (&text[..end], end < text.len())
}

#[derive(Default)]
struct ToolRounds { used: usize }
impl ToolRounds {
    fn admit(&mut self, contains_tools: bool) -> AiResult<()> {
        if !contains_tools { return Ok(()); }
        if self.used == 8 {
            return Err(AiError { kind: AiErrorKind::ToolLimitReached, retry_after_seconds: None });
        }
        self.used += 1;
        Ok(())
    }
}
```

- [ ] **4. Implement three Rig `Tool` definitions** using the spike's compiled `parameters`/async `call` shape. Schemas use integer limits, `additionalProperties:false`, query length 1-512 bytes, search limit 1-10, and optional folder/cursor. Rust validation enforces those limits even when the model ignores the JSON schema. Do not register `list_comments`, `propose_new_note` or `propose_rewrite` until Step 5 provides their store/workflow interfaces.
- [ ] **5. Construct agents directly from the owned ProviderClient** obtained through `.authenticate` with device flow disabled and the exact selected model. Private provider construction takes a Rig HTTP transport via `.connect(http)`; production uses the real transport and tests use synthetic transports. The agent helper can remain generic over the completion model. Do not expose a second provider service. Plain preamble explains read-only tools, fresh reads and keyword-only results. Map `HistoryPair` to text-only Rig messages with `agent.prompt(question).history(history).max_turns(9).stream()`.
- [ ] **6. Wire the budget into Rig's `on_model_turn_finished` hook before tool dispatch.** Inspect `ModelTurnFinished.content` for any `AssistantContent::ToolCall`; count a model response containing one or several tool calls as **one round**, not one tool. Admit rounds 1-8; on round 9 set a run-owned `Arc<AtomicBool>` limit flag and return `ModelTurnAction::Stop` before any tool executes. On termination inspect that flag first to produce ToolLimitReached; never classify by Rig's reason text or assume a particular PromptError variant. Nine model calls allow eight tool rounds plus the final answer; `.max_turns(8)` alone is off by one and `.max_turns(9)` alone can execute a ninth tool round. Do not install retry/fallback hooks; explicitly keep invalid-tool retry count zero. Test 2 parallel calls in a round, eight allowed rounds plus final text, ninth denied dispatch and simultaneous Stop/limit signaling.
- [ ] **7. Match stream items explicitly**, as in the spike. Text events append once and emit Text; tool events emit only an allowlisted name, not args/results; first error stops; EOF without final response fails. Stop selects on the CancellationToken and drops the stream; always return the accumulated text. A completed durable turn wins over a later Stop.
- [ ] **8. Write `provider_formats_tests.rs` as a `#[cfg(test)]` internal test module.** Use Rig `SequencedHttpClient` for ordered unary auth/identity replies. `RecordingHttpClient` returns one fixed response; `SequencedStreamingHttpClient` consumes **one** stream's chunks, not multiple completions. For tool continuation use a small private queue-backed `HttpClientExt` test adapter that captures each request and pops a distinct scripted SSE response; compose the unary sequence and streaming queue in that same transport. Connect it to actual subscription clients through the production construction helpers. Assert the queue is fully consumed and unexpected requests fail. This keeps injection private; no shipped fake-provider feature or CLI switch. Cover ChatGPT Responses SSE, Copilot chat-completions and Responses routing, text/tool continuation, 401, 429/reset and malformed SSE. Assert roles/history, tool names, exact model, destinations and final text. Intercept absolute auth/GitHub identity URLs too; no network transport or env-controlled record mode in tests.
- [ ] **9. Green and commit:** `cargo test -p brn-ai --locked`. If using cassettes instead of the synthetic clients, explicitly enable direct rig-cassette `http`, force Replay, provide synthetic fixtures for every URL and await `finish()`; missing fixtures fail. A feature-only build is not a substitute.

## Task 4: New app owner and current-vault read tools

**Files:** Task 4 row, workflow manifest/lib. **Consumes:** WorkStore, completed Step 3 `Library` interfaces, Task 3 read DTOs. **Produces:** `App`, `AiTools: ReadTools` and a retrieval-owned read-only index reader.

- [ ] **1. Add failing workflow tests** for unsafe paths through **all three** tools, stale passages, UTF-8/BOM/CRLF cap, folder-component pagination, fresh vault reads and legacy-directory refusal.

```rust
#[test]
fn new_app_refuses_legacy_data_without_touching_it() {
    use brn_workflow::app::{App, AppConfig};
    let data = tempfile::tempdir().unwrap();
    let creds_parent = tempfile::tempdir().unwrap();
    let legacy = data.path().join("brn.sqlite3");
    std::fs::write(&legacy, b"synthetic legacy sentinel").unwrap();
    let result = App::open(data.path(), AppConfig {
        vault_root: None,
        credentials_dir: creds_parent.path().join("credentials"),
        model_dir: None,
    });
    assert!(result.is_err());
    assert_eq!(std::fs::read(legacy).unwrap(), b"synthetic legacy sentinel");
    assert!(!data.path().join("brn.sqlite").exists());
}
```

- [ ] **2. Red:** `cargo test -p brn-workflow --test app --test ai_tools --locked`.
- [ ] **3. Implement mutually exclusive storage ownership and App opening.** After acquiring `brn.owner.lock`, WorkStore refuses any legacy `brn.sqlite3`/`-wal`/`-shm`/`-journal` marker before opening SQLite. Legacy Store checks `brn.sqlite`, all its sidecars and recognized WorkStore backups after acquiring that lock; a missing work DB awaiting restore is still a simple workspace. Detect both authorities as WorkspaceModeConflict and leave both unchanged; frontend dispatch is advisory, not the safety mechanism. Test Store/WorkStore directly and `brn-flow sessions` against a simple folder. Then open App with WorkStore once; visibly expose any restoration report. `bind_vault` performs the first binding on the application lane; a missing root leaves history/settings usable but Ask/read tools return VaultNotBound before network. Changing roots requires a new data folder.
- [ ] **4. Wire Library refresh to startup, explicit Refresh, window focus and app writes.** `App::search` and `notes` refresh before answering CLI requests. Background embeddings use existing Step 3 logic; report progress and `keyword_only`. Missing-model downgrade is the explicit search exception from the spec, **not a provider fallback**. Existing-but-invalid model/config/load failures remain typed errors, not "model absent".
- [ ] **5. Add `NoteIndexReader` in retrieval**, not SQL in workflow:

```rust
pub trait NoteSearch {
    fn keyword(&self, query: &str, limit: usize) -> brn_retrieval::Result<Vec<NoteHit>>;
    fn semantic_for_model(
        &self, vector: &[f32], model_identity: &str, limit: usize,
    ) -> brn_retrieval::Result<Vec<NoteHit>>;
}
pub struct NoteIndexReader { /* read-only rusqlite Connection */ }
impl NoteIndexReader {
    pub fn open(path: &std::path::Path) -> brn_retrieval::Result<Self>;
    pub fn notes(&self) -> brn_retrieval::Result<Vec<IndexedNote>>;
}
// Both NoteIndex and NoteIndexReader implement NoteSearch.
```

Use read-only `OpenFlags`, validate application ID/version and enable query-only/busy timeout. A reader never calls `NoteIndex::open` (that path may rebuild). Extract keyword/vector/model-metadata queries as free functions over `&Connection`; both concrete adapters use them. `semantic_for_model` checks the recorded model identity **and** vector dimension in the same short read transaction as vector lookup; mismatch returns an explicit error, never keyword fallback. Rebuilding/model switching must drain/detach readers before replacing the file. Test read-only refusal, simultaneous writer access and a same-dimension/different-model mismatch.

- [ ] **6. Share one embedder and one search policy.** Define this adapter/helper in `library.rs`; both writer Library and AiTools receive clones of the same SharedEmbedder, not a second ONNX load:

```rust
#[derive(Clone)]
pub struct SharedEmbedder {
    inner: std::sync::Arc<std::sync::Mutex<Box<dyn Embedder + Send>>>,
    identity: String,
    dimension: usize,
}
impl SharedEmbedder {
    pub fn new(embedder: Box<dyn Embedder + Send>) -> Self;
}
// Implements Embedder using immutable cached identity/dimension and a short
// mutex guard around embed(). Poisoning returns a typed error.
pub fn search_index(
    index: &dyn brn_retrieval::note_index::NoteSearch,
    embedder: Option<&mut dyn Embedder>,
    query: &str, mode: SearchMode, limit: usize,
) -> LibraryResult<SearchResults>;
```

`Library::search` and AiTools both call `search_index`: validate once; absent model means keyword with `keyword_only=true` in **every mode**; present model + Keyword sets false; Semantic embeds query once; Hybrid takes 50 from each and uses existing fusion. Pass expected model identity to semantic lookup. AiTools owns its reader behind a mutex; synchronous tools execute via spawn_blocking, with no guard across network waits. Inference may wait behind one bounded embedding batch, but neither model nor index mutex is used by unsaved-edit storage. Model replacement builds a new shared adapter only after old chat/tools have drained. Fake embedder tests prove one loaded model, shared policy and identity refusal.

- [ ] **7. Enforce currentness/visibility in every tool.** Parse paths with VaultPath and read fresh bytes. Never return a search passage unless current hash, UTF-8 range and exact quote all match. If any candidate is stale/removed, return a typed IndexStale tool failure without exposing its text; do not disguise it as an empty success. Invalid arguments/excluded paths return ToolRejected. Rig encodes these as safe model-visible failed tool results; they need not abort the whole answer, and any repair the model makes is an ordinary bounded tool round, not an automatic turn retry. Reserve InvalidToolUse for Rig-level invalid/unknown calls; keep invalid-call retry count zero. Tool-limit termination uses its typed flag. Read results carry the exact 50,000-byte prefix and truncation boolean; the accepted parent-folder race limit is unchanged.
- [ ] **8. Implement list pagination.** Paths/titles from the indexed eligible corpus, revalidated against current files; no unreadable note text. Sort by vault-relative path. Cursor is the last returned valid path, exclusive; next page filters `path > cursor`. Validate folder components with the same hidden/archive/traversal rules but without requiring `.md`; `"work"` matches `work/`, not `workshop/`. Return at most 200 entries; return `next_cursor` only when more exist. Invalid cursor/folder is an explicit ToolRejected result. Concurrent vault edits may change later pages; this is keyset pagination, not a frozen snapshot.
- [ ] **9. Add explicit consented model installation** using the model-install contract below. Persist the first prompt decision; do not auto-download on startup/search/Ask. Decline leaves keyword-only mode, and an explicit later Download action can override it. Test decline, cancellation, incomplete/corrupt assets, native-feature refusal and successful synthetic installation before adding a real network path.
- [ ] **10. Green and commit:** `cargo test -p brn-retrieval --test note_index --test note_search --test note_embeddings --locked`; `cargo test -p brn-workflow --test library --test app --test ai_tools --test models --locked`; `cargo test -p brn-retrieval --features native --test model_download --locked`; `cargo test -p brn-workflow --features native-retrieval --test models --locked`. Native installer checks use synthetic assets only. Assert every fixture vault checksum is unchanged, including refused calls.

### Model-install contract (Task 4.9)

The download promise is included, not deferred. Add a small native-only installer to retrieval, called by workflow only after an explicit approved action. It writes model assets, never vault notes. Inspecting public metadata to pin these records is not a model download or model-inference check.

```rust
pub struct ModelInstallReport {
    pub directory: std::path::PathBuf,
    pub downloaded_bytes: u64,
}
pub fn download_model(
    target: &std::path::Path,
    cancel: &std::sync::atomic::AtomicBool,
    progress: impl Fn(u64, u64),
) -> brn_retrieval::Result<ModelInstallReport>;
```

Use `Xenova/all-MiniLM-L6-v2`, immutable revision
`751bff37182d3f1213fa05d7196b954e230abad9`, verified from the public Hugging Face
model metadata on 2026-10-02. The base URL is
`https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/751bff37182d3f1213fa05d7196b954e230abad9/`.
Only these five assets are allowed, matching current LocalEmbedder input:

| Source / installed name | Exact bytes | Expected digest |
| --- | --- | --- |
| `onnx/model.onnx` / `model.onnx` | 90,387,606 | SHA-256 `759c3cd2b7fe7e93933ad23c4c9181b7396442a2ed746ec7c1d46192c469c46e` |
| `tokenizer.json` | 711,661 | Git blob SHA-1 `c17ed520ed8438736732a54957a69306b8822215` |
| `config.json` | 650 | Git blob SHA-1 `72147e4ff4426ebedbfa2146c4a0999def51a313` |
| `special_tokens_map.json` | 125 | Git blob SHA-1 `a8b3208c2884c4efb86e49300fdd3dc877220cdf` |
| `tokenizer_config.json` | 366 | Git blob SHA-1 `37fca74771bc76a8e01178ce3a6055a0995f8093` |

Git blob hashes include `blob <decimal-length>\0` before the bytes. These pin small configuration assets to the immutable repository revision; the ONNX LFS object uses SHA-256. Never replace expected digests with response-supplied values or accept floating `main`. Download the five files into an exclusively created sibling staging directory, enforce exact per-file/total byte bounds (91,100,408 total), verify before installing, and install the complete directory using macOS `renameatx_np(RENAME_EXCL)`, not overwrite-capable `std::fs::rename`. Existing valid pinned assets can be reused after verification; other occupied targets are refused unchanged. Cancellation, truncated input, HTTP failure or digest mismatch removes only this install's known staging files and reports a typed error; no implicit retry.

The consent surface states source, approximate 87 MiB network/storage cost and destination. Persist `model.download_decision` as approved/declined so startup asks at most once; no background download is triggered merely by that persisted approval. Later retries require a fresh explicit Download action. Default destination is `<simple-data-dir>/models/minilm`; an explicitly selected absolute target is allowed only with the same no-overwrite rules. `LocalEmbedder::open` remains load-only; after successful installation, activate through the application lane, replace the shared embedder and rebuild vectors using its identity. If a chat is active, emit ModelDownloaded and defer activation until its tools/client have drained; emit ModelInstalled only after load succeeds. A load failure remains visible and does not silently downgrade to "model absent".

For `brn-retrieval/native`, add optional `reqwest = "=0.12.28"` with `default-features=false`, `blocking`/`rustls-tls`, optional `sha1 = "=0.10.6"` for pinned Git blob digests, and macOS libc `=0.2.189` for exclusive installation; existing sha2 verifies ONNX. Network work runs on an owned blocking install job with bounded connect/read timeouts and cancellation between chunks. Tests inject an internal synthetic manifest/HTTP source, assert digest/size/cleanup/decline behavior and never fetch these production assets. Synthetic install success is not ONNX inference qualification.

Default/headless builds stay keyword-only; a supplied `--model-dir` or download action in a build without `native-retrieval` returns typed `SEMANTIC_UNAVAILABLE_IN_BUILD`. The normal native desktop build/launcher explicitly enables `native-ui,native-retrieval`; the full CLI build enables `brn/native-retrieval`. Asset absence in a native build permits explicit keyword-only mode; invalid installed assets do not.

## Task 5: Independent chat lane, cancellation and durable endings

**Files:** Task 5 row, WorkStore chat connection support. **Consumes:** App, AI answer and store lifecycle. **Produces:** ChatWorker interface above and workflow-owned authentication command routing.

- [ ] **1. Write failing worker tests** using barriers/channels, not sleeps, for Stop before any text, Stop after partial text, stream failure after partial text, restart/resume, provider/model snapshot, late generation and shutdown. A mock-completion adapter is test-only and cannot be enabled by CLI arguments.
- [ ] **2. Red:** `cargo test -p brn-workflow --test app_worker --test chat_worker --locked`.
- [ ] **3. Add an owner-authorized attached chat connection** in `brn_store::work::chat`:

```rust
pub struct ChatStore { /* attached RW connection, no second owner acquisition */ }
impl WorkStore {
    pub fn chat_connection(&self) -> brn_store::Result<ChatStore>;
}
impl ChatStore {
    pub fn turns(&self, conversation: uuid::Uuid) -> brn_store::Result<Vec<WorkTurn>>;
    pub fn begin_turn(
        &mut self, id: uuid::Uuid, conversation: Option<uuid::Uuid>,
        question: &str, provider: &str, model: &str,
    ) -> brn_store::Result<WorkTurn>;
    pub fn finish_turn(
        &mut self, id: uuid::Uuid, status: WorkTurnStatus,
        answer: &str, error_code: Option<&str>,
    ) -> brn_store::Result<WorkTurn>;
}
```

Share the WorkStore transaction helpers. This is created only by the existing owner, after integrity/migrations; no public arbitrary-path writer that bypasses ownership, startup check or backups. WAL/busy timeout/foreign keys/synchronous settings match WorkStore. The worker must be joined before the App/owner lock drops.

- [ ] **3a. Enforce attached-owner lifetime.** Change WorkStore's lock File to `Arc<File>` and retain a clone in ChatStore, without opening/relocking another descriptor. Dropping App alone cannot release ownership while an attached connection lives. Test that another WorkStore/legacy Store remains refused until the last attachment drops. Add ChatWorker and AppWorker Drop implementations that cancel and join admitted work; explicit shutdown returns finalization errors, and Drop never uses a detached reaper or claims a failed write was saved.
- [ ] **4. Start the two owned lanes.** AppWorker owns App, processes the declared command/event interface and interleaves bounded embedding batches. ChatWorker owns its runtime, read adapter and ChatStore and holds Arc<Auth>; its select loop continues receiving account/cancel commands while separate futures await network streams. Admit one active turn, but do not serialize the other provider's account actions behind it. Use target-provider fencing and cancellation/join ordering from the Auth contract. Tests hold a stream/login pending and prove Status/Disconnect/cancel on the appropriate lane are responsive. AppWorker joins ChatWorker and model-install work before dropping App.
- [ ] **5. Refresh and preflight before external submission.** AppWorker refreshes Library before every newly submitted Ask; require a bound/available vault and valid selection before any network call. Resolve local history and check prior UUID replay before authenticating; replay reads history only and does not require a fresh model request. For a new request, create the Running pair transactionally, obtain an owned ProviderClient with no long-lived Auth guard, then stream. Define `pub fn model_history(turns: &[WorkTurn]) -> Vec<HistoryPair>` in `app.rs`: last 20 earlier terminal turns in sequence order, including partial failed/interrupted answers; exclude Running. Map to text-only Rig messages, omitting empty assistant text but keeping its question. Test 21 prior turns selects 2-21 and no tool/response metadata appears. Pre-submission auth errors end the recorded turn Failed with a safe category. Prior terminal replay returns Finished without network; prior Running returns interrupted/unknown outcome, never resends. Unknown conversation fails before insertion.
- [ ] **6. Finalize on every ending.** Map Completed/Interrupted/Failed to WorkTurnStatus and commit once through ChatStore before Finished. If the terminal write fails, emit PersistenceFailed with safe storage copy and the in-memory partial text; do not claim the turn is saved. A later restart can reconcile the retained Running record but cannot reconstruct uncommitted text.
- [ ] **7. Prove note recovery does not wait for the model.** Hold the fake model pending at a barrier; submit AppCommand::RecoverEdit, which validates VaultPath then calls the existing `put_unsaved_edit(path, base_sha256, text)` on App's owner handle. Observe EditRecovered before releasing the model. Also exercise BindVault, Refresh, progress forwarding and shutdown on AppWorker, and preserve legacy save/recovery tests; do not acquire a second WorkStore owner. This proves actual application-lane scheduling and SQLite access, not the future simple Markdown save protocol. Step 6 must repeat it with an actual Save acknowledgement.
- [ ] **8. Correlate every event by turn UUID and generation.** Navigation ignores stale deltas, but worker still commits the associated conversation. Stop targets an exact active UUID. Shutdown cancels, waits for terminal persistence and joins; it does not use the detached legacy reaper for chat finalization. Document the limit: no guarantee that upstream cancels billing, and OS-killed processes lose unsaved stream text.
- [ ] **9. Green and commit:** `cargo test -p brn-workflow --test app_worker --test chat_worker --test app --test notes --test note_recovery --locked`. Record deterministic concurrency results; do not infer GPUI usability from them.

## Task 6: CLI connection, read/search and conversation cutover

**Files:** Task 6 row; `crates/brn/src/cli/{mod,ask,error,status}.rs`, `src/main.rs`; affected `cli_basic`, `cli_ask`, `cli_cancel`, `cli_signals`, `cli_ownership` tests.

**Consumes:** App/ChatWorker/DTOs, existing JSON envelope/output-delivery/signal handling. **Produces:** these simple-app commands:

```text
brn ai connect chatgpt|copilot
brn ai disconnect chatgpt|copilot
brn ai status
brn ai models chatgpt|copilot
brn ai select --provider chatgpt|copilot --model MODEL
brn models download --approve-download [--model-dir DIR]
brn notes list [--folder FOLDER] [--cursor PATH]
brn notes show PATH|LEGACY_NOTE_UUID
brn search QUERY [--profile keyword|semantic|hybrid] [--limit N]
brn ask QUESTION [--session UUID] [--operation UUID] [--timeout-seconds N]
brn conversations list
brn conversations show UUID
```

Shared options: existing absolute `--data-dir DIR`, new `--vault DIR` for first simple binding, new `--legacy` for unambiguous empty-folder legacy selection, new absolute `--credentials-dir DIR`, existing `--model-dir DIR` and `--json`. `--legacy` conflicts with simple-only options/actions. Default credentials path is a sibling named `<data-directory-name>.credentials`, derived by workflow, never inside the repository or vault. Persist its configured path as a non-secret setting; no environment/default Rig account lookup. CLI validates data, vault and credential locations before connecting.

New search defaults to hybrid with limit 10 (human CLI limit 1-50 remains distinct from the AI tool's limit 1-10). `ai models` is an explicit discovery operation, not an automatic startup call.

### Dispatch/compatibility matrix

Legacy means legacy DB/sidecar markers; simple means work DB/sidecar markers or recognized WorkStore backups; empty means neither. Mixed markers always fail WorkspaceModeConflict. Classification is advisory; Task 4's post-lock store checks enforce it for **all** entry points. Shared commands never guess which authority to create in an empty folder.

| Command | Legacy folder | Simple folder | Empty folder |
| --- | --- | --- | --- |
| `ai *`, `models download`, `notes list`, `notes show PATH.md` | WorkspaceModeConflict; no new work DB | App | App; explicitly simple commands initialize work storage |
| `notes show UUID`; managed-note edit/recovery/save commands; import/documents/drafts/comments/revisions/index build | Workspace, preserving existing contracts | WorkspaceModeConflict; no legacy DB | Workspace for legacy-only command, unless simple-only flags were supplied |
| `status`, `search`, `conversations list/show` | Workspace and existing legacy result/JSON shapes | App and documented simple shapes | `--vault` selects App; `--legacy` selects Workspace; otherwise WorkspaceModeRequired, no database created |
| `ask` | LegacyAiRetired before submission; old history remains readable | AppWorker; bound vault and explicit selection required | `--vault` required to initialize/bind; otherwise VaultNotBound and no network |
| `brn-flow` commands | Workspace; legacy local/search/history preserved | WorkspaceModeConflict enforced by Store | Legacy-only driver creates legacy authority |

`notes show` parses UUID first, otherwise a valid vault-relative Markdown path; a UUID is not repurposed as a filename and an invalid token is usage failure. Do not add `notes legacy-show` or break existing UUID invocations. `search --profile` retains legacy behavior/default in legacy mode; simple mode uses the new search contract. Retire `ask --profile`: retrieval is decided by the read tool, not eager grounding. Reject this obsolete flag with usage 2 **before workspace opening** and document the replacement; preserve `--session` and `--operation`. No automatic legacy AI fallback.

- [ ] **1. Add failing parser/subprocess tests** for new commands, unknown provider/model, path-based notes, no Codex requirement, bad directories, mutually exclusive old/new arguments, one JSON envelope, SIGINT/deadline and error context. Tests requiring a model use injected workflow test support, not a shipped fake switch.
- [ ] **2. Red:** `cargo test -p brn --test cli_ai --test cli_library --test cli_ask --locked`.
- [ ] **3. Implement the dispatch matrix, not parallel authority guessing.** Route simple commands through App and legacy local commands through Workspace. Preserve dual-form `notes show` and existing legacy search/conversation result shapes. Test every row against legacy/simple/empty/mixed fixtures, including missing simple DB with valid backups, and test `brn-flow` cannot create `brn.sqlite3` in a simple folder. Retire `ask --profile` explicitly and reject incompatible flags before opening either store.
- [ ] **4. Replace ask dispatch through AppWorker, which owns ChatWorker.** Keep operation identity and `--session` as the conversation flag. Read explicit saved selection through AppCommand::Selection; missing selection is a typed operational error, not a default provider/model. Submit Ask only through the application preflight/refresh path, never raw ChatWorker::ask from CLI. Deltas go to stderr; stdout retains exactly one final envelope. `--timeout-seconds` keeps 300 default, 1-3600 range; Stop/SIGINT/deadline cancels the exact worker request, waits for finalization and joins.
- [ ] **4a. Treat explicit Connect/models as cancellable operations too.** Send Account/CancelAccount through AppWorker. Their CLI event loop observes the existing SIGINT atomic, clears the login prompt and joins. Do not leave them classified as ordinary non-cancellable commands merely because `main.rs` currently singles out Ask.
- [ ] **4b. Wire `models download --approve-download`** to the workflow consent/install job and cancellation/progress path. No approve flag means usage failure with zero requests, not silent approval. Unsupported feature builds fail before network. Native fixtures test installation using only injected synthetic responses.
- [ ] **5. Preserve honest output/exit behavior.** Success only for a durably Completed turn; deadline exit 124, SIGINT 130, usage 2, operational errors 1. Post-completion signals cannot overturn success. Failure context retains `operation_id`, `session_id`, `recorded_status` and receipt/partial text; provider outcome stays Unknown unless actually observed. No claim of server cancellation. PersistenceFailed has unknown recorded terminal outcome, not `saved=true`.
- [ ] **6. Extend typed error codes**, mapped at workflow source: `AI_RECONNECT_NEEDED`, `AI_CODE_EXPIRED`, `AI_RATE_LIMITED`, `AI_NETWORK`, `AI_MODEL_REFUSED`, `AI_INVALID_TOOL_USE`, `AI_TOOL_LIMIT_REACHED`, `AI_UNSAFE_CREDENTIALS`, `AI_STORAGE_ERROR`, `AI_SELECTION_REQUIRED`, `VAULT_NOT_BOUND`, `WORKSPACE_MODE_CONFLICT`, `WORKSPACE_MODE_REQUIRED`, `LEGACY_AI_RETIRED`, `SEMANTIC_UNAVAILABLE_IN_BUILD`, `MODEL_DOWNLOAD_FAILED`. Each has a matching typed workflow ErrorKind, never wording-based classification. ToolRejected/IndexStale normally remain safe model-visible tool results; if terminal, map them to typed AI tool/index errors. Preserve unrelated codes and broken-pipe handling.
- [ ] **7. Login output is a separate transient surface.** `ai connect --json` emits only final safe account status in its envelope; URL/code goes to the dedicated stderr login prompt, never error context or conversation. Explain this exception to "no device codes in logs"; do not enable verbose HTTP/Rig tracing.
- [ ] **8. Update command documentation/fixtures.** New turn JSON exposes provider, model, question, answer and terminal status, not provider thread IDs. Keep envelope `schema_version:1` only with explicit documented command-shape cutover for simple folders; legacy history remains readable through `brn-flow sessions/history`. Update tests that asserted old ask/Codex semantics; retain signal/pipe/durable-outcome invariants.
- [ ] **9. Green and commit:** `cargo test -p brn --locked`. Use only subprocess-owned temporary folders and synthetic provider data. No login/chat subprocess with real credentials.

## Task 7: Desktop settings and chat without blocking the editor

**Files:** Task 7 row; `native/shell/{settings,centre,history_rail,vault_rail,header}.rs`, `native/mod.rs`, desktop `src/main.rs`, `tests/cli.rs`.

**Consumes:** workflow App/ChatWorker and existing layout, theme, editors and close guards. **Produces:** settings Connect/Disconnect/provider/model selection and new chat UI; no GPUI-facing Rig state.

- [ ] **1. Add failing state tests in `ai.rs`**, independent of GPUI: selected provider/model, login lifecycle, stopped/failed partial answer, old generation ignored, concurrent submit rejected and history navigation. Native test modules cover action routing and disabled states.
- [ ] **2. Red:** `cargo test -p brn-desktop --locked`; then `cargo test -p brn-desktop --features native-ui ai:: --locked` after adding native hooks.
- [ ] **3. Keep App off GPUI.** Desktop owns AppWorker's handle and presentation-only state: conversation, active UUID/generation, partial text, safe terminal error and transient login prompt. Ready/restoration/binding/selection/index progress/search/history arrive through declared AppEvents; read selection with Selection, vault chooser sends BindVault and model consent sends DownloadModel. No App/Library initialization, SQL, hash scan, ONNX load/inference or token/client state in views. Show provider/model on each message, including earlier selections.
- [ ] **4. Replace the existing Codex connection settings section** with ChatGPT and Copilot Connect/Disconnect rows, connected account name, reconnect-needed status and explicit provider/model controls. The login code/link appears only during the active Connect dialog; clear it on success, failure, cancel and dialog close. Expose "code expired, try again" without starting a new login automatically. Show ChatGPT's conditional-qualification limitation without claiming quota reset proves availability.
- [ ] **5. Wire Ask/Stop/history to the independent chat lane.** Stream text/tool-name progress; keep partial text and terminal label after failure/Stop. History resumes from WorkStore after restart. Freeze selection per submitted request. Disable another Ask while one is active, not independently supported recovery/editor actions. Focus/narrow-window Document/Chat tabs and existing shortcuts stay intact. Simple-mode notes are saved-file reading surfaces in this step; do not add a fake Save button or claim the Step 6 simple save protocol is implemented.
- [ ] **6. Define launch mode and preserve legacy local work.** Default native launch uses new `~/Library/Application Support/BRN-simple`; its credentials sibling is `BRN-simple.credentials`. Do not inspect/migrate the old BRN folder automatically. `--legacy` without a data-dir explicitly opens the old default `~/Library/Application Support/BRN`; an explicit `--data-dir` honors existing markers and rejects an incompatible `--legacy`/`--vault` combination. Legacy launches retain local editing/history on their existing Worker, without opening App in that folder, and show "legacy AI retired; use a new simple workspace". Remove executable/provider controls, not recovery/save guards. Simple first launch has a real Choose Vault action routed to BindVault and a one-time model-download consent prompt. Decline creates no network request. Update launcher scripts to enable `native-ui,native-retrieval` and use the new default without copying old data.
- [ ] **7. Close behavior:** guarded close/Quit cancels chat and waits for local finalization plus existing critical note jobs. Keep the existing documented Dock/system-termination limitations; no new unsafe final-hook flush and no successful-save claim from stream cancellation.
- [ ] **8. Update vault/search surfaces to App read results** in simple mode; display keyword-only, exact indexing progress/unreadable notes and model install/download/activation/error state honestly. Expose Cancel Download; an explicit later Download action can override a recorded decline. Use BRN's existing palette/font tokens, no redesign or chat-polish backlog work.
- [ ] **9. Green and commit:** `cargo test -p brn-desktop --locked`; `cargo test -p brn-desktop --features native-ui,native-retrieval --locked`; `cargo build -p brn-desktop --features native-ui,native-retrieval --locked`. Manually observe first vault choice, model consent/decline, account dialog, streaming, Stop, restart/history, long/Unicode text and narrow-window behavior on an unlocked Mac with disposable data. Model download/live stream observations need separate authorization; offline tests inject synthetic assets/events. Preserve legacy Save/recovery observations. Actual simple Save while chat runs remains a Step 6 check.

## Task 8: Remove App Server and record the real qualification

**Files:** root/workflow manifests/lockfile; `crates/brn-provider/**`; workflow `lib.rs/main.rs/worker.rs`, provider-specific tests; CLI/desktop configuration; scripts and directly related documentation.

**Consumes:** both consumers switched to App/ChatWorker. **Produces:** no production App Server dependency/configuration; a reproducible Step 4 handoff.

- [ ] **1. Add a failing retirement check** to the integrated verification script: production manifests/source/launchers must not reference `brn-provider`, `brn_provider`, `codex_home` or `--codex`. Exclude historical docs and standalone `experiments/codex-app-server`/`rig-spike`; those preserve evidence.

```sh
if rg -n 'brn[-_]provider|codex_home|--codex' \
    Cargo.toml crates scripts/make-macos-app.sh scripts/test-make-macos-app.sh \
    --glob '*.rs' --glob '*.toml' --glob '*.sh'; then
  printf 'Production App Server reference remains\n' >&2
  exit 1
fi
```

- [ ] **2. Remove the old Workspace AI implementation and provider dependency together.** Keep legacy local storage/editor code and offline legacy sessions/history until Step 6. Existing public legacy ask entry points retained during transition return typed "legacy AI retired" **before submission**, never a Rig request pretending to resume a Codex thread. Replace provider-dependent error/status conversions with local types; remove sidecar interrupt/reaper paths only where no longer used. Do not delete unrelated critical note shutdown behavior.
- [ ] **3. Remove fake-sidecar-only tests with the retired module**, replacing their still-relevant signal/persistence/late-event invariants with the new Rig/workflow tests. Preserve non-provider tests in `flow.rs` and `note_evidence.rs`; do not delete whole suites to obtain a green run. Delete `crates/brn-provider` and root member last, then update lockfile once and return to locked checks.
- [ ] **4. Update CLI/desktop/brn-flow help, Settings and macOS launcher scripts/tests** so no path or `--codex` is required/passed. Verify BRN-simple default, explicit legacy route, first vault choice and native retrieval feature flags. Default builds remain keyword-only and reject model installation explicitly. `scripts/verify-trial.sh` remains a historical standalone experiment check, not a product provider check. Change `verify-end-to-end.sh` to a provider-free simple-vault read/search fixture, retaining separate legacy local fixture coverage until cleanup.
- [ ] **5. Update root/relevant crate READMEs, architecture overview/invariants, verification guide, current status, active roadmap and evidence.** Mark old provider plans historical; describe the actual new data-dir/credential contract and the supported transition commands. Keep Step 5 proposal tools and Step 6 save/cleanup work explicitly unimplemented.
- [ ] **6. Run one integrated offline gate** after targeted tasks pass:

```sh
cargo fmt --all -- --check
cargo build --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test -p brn-retrieval --features native --test model_download --locked --offline
cargo test -p brn-workflow --features native-retrieval --test models --locked --offline
cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo build -p brn --features native-retrieval --locked --offline
bash scripts/verify-end-to-end.sh
bash scripts/test-make-macos-app.sh
git diff --check
```

Avoid running the shared workspace suite again through several wrappers: adjust the integrated script or run its fixture-only portion after the shared gate. Check standalone spike only if its source changes. Missing cached build dependencies may be fetched after a missing-dependency failure; that never authorizes account calls or model downloads.

- [ ] **7. Record outcomes honestly and commit.** Record exact HEAD/dirty diff, commands, feature flags, pass/fail/skips, fresh-vs-historical evidence and native/live limitations in `evidence.md`. Separately requested live checks: both providers' login, streamed read tool, new-process credential reuse, refresh without login, quota/reconnect/error copy and partial-output Stop. ChatGPT is not accepted until its blocked spike scenarios are observed. No push/PR/release without instruction.

## Acceptance matrix

| Requirement | Task/check | Required outcome |
| --- | --- | --- |
| Explicit account/provider/model; no automatic login/fallback | 1, 3, 5-7 | Only Connect starts device login; missing/expired credentials sends no completion; exact selected model remains fixed |
| Credential safety, expiry and Disconnect | 1, 5 | Folder/file modes checked; both provider-specific caches removed; no in-memory or refresh resurrection; other account untouched |
| Responsive account commands | 1, 5, 7 | Stream/login waits never occupy dispatch loop; target-provider Disconnect fences/cancels/joins before deleting caches; other provider remains responsive |
| Secrets never retained | 1, 3, 6-7 | Synthetic markers absent from errors/Debug/output logs/SQLite; device code only on login surface |
| Current, read-only tools | 3-4 | Unsafe/excluded paths invisible; fresh hash/range/quote check; no changed vault bytes |
| Exact numerical limits | 3-4 | 10 hits, 50,000-byte UTF-8 prefix, 200 rows/page, eight rounds; ninth tool dispatch count **zero** |
| Shared hybrid / keyword-only policy | 4 | One loaded embedder shared by Library/tools; top-50 fusion; explicit flag when absent in every mode; same-dimension identity mismatch and invalid model remain errors |
| Consented model installation | 4, 6-8 | Decline/persisted approval alone makes zero requests; explicit download validates pinned asset lengths/digests, refuses overwrite and supports cancellation; non-native builds return a typed refusal |
| Local history and provider/model provenance | 2, 5-7 | Last 20 earlier text-only turns; no tool history; restart resumes; each message identifies provider/model |
| Failed/stopped partial output retained | 2-3, 5-7 | Failed/Interrupted terminal saved once; no retry; finalization failure is visible |
| Owned lanes and lifetime | 4-5, 7 | App/Library off GPUI; read-only reader never rebuilds; attached ChatStore retains owner lock until drained; shutdown joins chat/install work |
| Note work is independent of chat | 5, 7; actual simple Save in Step 6 | AppWorker EditRecovered arrives while model is held pending; independent scheduling proven here, Markdown Save concurrency qualified when that workflow exists |
| CLI/Desktop parity | 6-7 | Shared workflow rules and outcomes; one CLI envelope; honest exits; generation-safe UI |
| Workspace dispatch and first launch | 4, 6-7 | All command/mode combinations covered; UUID notes show preserved; shared empty-folder commands require a mode; both stores reject mixed authority after lock; BRN-simple default/explicit legacy/vault binding work without migration |
| Provider retirement / preserved local work | 8 | No production App Server configuration/dependency; old files untouched; legacy local/recovery/history remain usable |
| Qualification | 8 | Offline/native build evidence separate from actual live/native acceptance; ChatGPT gate remains explicit |

## Handoff

Planning is complete when this file is linked from the Step 4 roadmap and active-work index, local links/whitespace are checked, and the plan is reviewed against the approved spec. Record the Opus review dispositions in [evidence](evidence.md). Product implementation and user acceptance remain pending. Before execution, choose the implementation branch/worktree and reconcile completed Step 3 inputs plus concurrent changes; obtain separate permission for any live recheck or model download. Review of this plan does not imply permission for account actions, migrations, merging or distribution.
