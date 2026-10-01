# Shared Rig AI Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for inline execution, or superpowers:subagent-driven-development when explicitly selected. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship one guarded, durable Rig ask/tool/candidate workflow for desktop and CLI, without a Context Engine or a second agent framework.

**Architecture:** Thin brn-ai owns Rig/provider/history representation. The workflow's existing store owner acknowledges dispatch and tool effects through a private bounded bridge. BRN store transactions establish durability; Rig hooks/memory append alone never do.

**Tech Stack:** The Q1-Q3 qualified Rig pin/transport/auth, Tokio, Serde, UUID, SQLite store schemas 7/8, existing worker, CLI and GPUI shell.

**Spec:** [Approved reset](../../../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md); [master plan](plan.md).

## Global constraints

- Inherit all master constraints and task dependencies. G2 qualification precedes production replacement.
- A1/A2 are foundation checkpoints; do not call Rig ask delivered until A3 includes history/currentness/read tools/failure honesty together.
- No model-driven Save, shell, SQL, filesystem path access or CLI bypass.
- Provider/model/account selection and retained-context transfer are explicit. No fallback.
- Secrets and raw auth payloads never enter store, events, logs or cassettes.
- Currentness, durable local state and remote outcome are independent persisted facts.
- No automatic uncertain-effect retry, historical thread translation, summarization or centralized Context Planner.

---

## A1: Thin production Rig runtime and explicit settings

**Prerequisite:** Q1-Q3/G1/G2.

**Files:**
- Create: `crates/brn-ai/Cargo.toml`, `src/{lib.rs,config.rs,auth.rs,credentials.rs,run.rs,history.rs,tools.rs}`, `README.md`, `tests/{auth.rs,run.rs}`.
- Create: `crates/brn-workflow/src/ai.rs`, `crates/brn/src/cli/ai.rs`, `crates/brn-desktop/src/ai.rs`.
- Modify: root `Cargo.toml`/lock; workflow manifest/lib/error/worker; CLI manifest/mod/status/error; desktop main/native/settings; all Config constructors.

**Consumes:** Q1 compiled interfaces, Q2 private OAuth/cache/identity strategy, Q3 selected capabilities and cancellation semantics.

**Produces:** Public product types (all serializable configuration/result types derive Debug/Clone/Serde as applicable; never expose token fields):

```rust
pub enum Provider { ChatGpt, Copilot }
pub struct AccountIdentity {
    pub provider: Provider,
    pub subject: String,
    pub display: String,
}
pub struct ProviderSelection {
    pub provider: Provider,
    pub model: String,
    pub account: AccountIdentity,
}
pub struct AiConfig {
    pub credential_root: std::path::PathBuf,
    pub selection: Option<ProviderSelection>,
}
pub struct HistoryBlob {
    pub format: u32,
    pub rig_version: String,
    pub bytes: Vec<u8>,
}
pub enum RemoteOutcome { NotDispatched, Completed, Failed, Cancelled, Unknown }
pub struct RunLimits {
    pub model_calls: u32,
    pub tool_calls: u32,
    pub context_bytes: usize,
    pub output_bytes: usize,
    pub duration: std::time::Duration,
}
```

Persist provider spellings exactly `chatgpt` and `copilot` using explicit Serde renames; don't rely on inferred casing of `ChatGpt`. Derive equality for identity/selection/outcome records so account continuity checks are structural, not display-name comparisons.

`HistoryBlob.bytes` is canonical pinned Rig messages, validated by `brn_ai::validate_history(&HistoryBlob) -> Result<(), AiError>` and initialized by `brn_ai::empty_history() -> HistoryBlob`. Rig types remain private. Do not expose secret-bearing auth contexts or fabricate a generic JSON "message" framework. If serialization is not directly supported, Q1 must establish a narrowly versioned adapter preserving every required Rig message variant.

Use `RunLimits::default()` = 8 model dispatches, 16 tool dispatches, 20,000 serialized context bytes, 1 MiB output and 120-second elapsed turn deadline. These are documented conservative product limits; exhaustion is typed, never truncation acknowledged as a complete answer. Preserve existing CLI deadline override within the same run policy.

Private `run.rs` contracts: `RunRequest` carries operation/conversation IDs, selection, prompt, validated history, enabled tool names and limits. `RunResult` carries sanitized text/history/usage and independent remote outcome. `BridgeRequest` is a closed enum of dispatch intent, typed tool invocation, delta and terminal; acknowledged requests use Tokio oneshots. No generic multi-provider service hierarchy.

Workflow exposes `connect_provider(&mut self, provider: Provider) -> Result<AccountIdentity>`, `disconnect_provider(&mut self, account: &AccountIdentity) -> Result<()>` and `ai_selection(&self) -> Option<&ProviderSelection>`. Config adds `ai: Option<AiConfig>` temporarily alongside legacy fields; A3 switches ask, D1 removes obsolete fields. A1 accepts caller-provided selection; A2 adds durable `select_provider`/the CLI select command with schema V7. Do not invent temporary selection-file authority to bypass migration ordering.

- [ ] **1. Red noninteractive-auth test:** inject Q2's synthetic transport/cache. `resolve_selected_credentials(selection, allow_device_flow=false)` must return `AiError::ReconnectRequired` for absent cache; assert zero device authorization requests and zero completion dispatches. Repeat for both providers.

Start with the executable history-policy regression in `tests/run.rs`:

```rust
#[test]
fn unsupported_history_format_is_not_empty_history() {
    let history = brn_ai::HistoryBlob {
        format: u32::MAX, rig_version: "0.43.0".into(), bytes: b"[]".to_vec(),
    };
    assert!(brn_ai::validate_history(&history).is_err());
}
```

Run `cargo test -p brn-ai --locked --test auth`; expect missing crate/interface. Use a manifest dependency change before fetching missing dependencies.

- [ ] **2. Port only qualified seams:** reuse exact Q2 owner-only cache policy and stable identities; Q3 production completion setup and Rig agent/hook/tool/stream APIs. No test-support production transport. Explicit Connect enables device login; ordinary run only refreshes selected account and checks identity. A 401/account mismatch doesn't select another account.

Define the deterministic integration seam explicitly: debug-only `test-support` features on brn-ai/workflow/CLI; workflow `Workspace::open_replay(data: &Path, config: Config, fixture: &Path) -> Result<Self>` and `test_dispatch_count(&self) -> usize`. It installs Q3's owned exact-request replay transport, rejecting unmatched/non-loopback delivery, not a global transport override. CLI fixture builds accept hidden `--test-replay-fixture`; production builds reject it. Compile-time reject `test-support` in release builds.

New `rig_*` workflow/CLI integration tests declare `required-features = ["test-support"]` in their manifests. Run their selectors explicitly with `--features brn-workflow/test-support,brn/test-support`; default suites alone are not replay coverage. D1's script runs this targeted graph in addition to default/native checks; D2 builds without test features.

- [ ] **3. Add error/redaction types:** `AiError` distinguishes invalid config/history, reconnect, account mismatch, capability unavailable, policy denial, transport uncertainty, budget and cancellation. Map at workflow boundary to typed error categories. Never interpolate a raw HTTP auth body into user messages. Test synthetic bearer/access/refresh/bootstrap/device-code strings are absent in Debug, errors, event JSON and logs.

- [ ] **4. Wire settings/commands:** `brn ai connect --provider chatgpt|copilot`, `brn ai disconnect --provider ... --account SUBJECT`, `brn ai status`; native selection controls become durable in A2. Device prompts are explicit interactive Connect UI/stderr output, not ordinary logs/JSON. Show selected identity, reconnect state and qualified capability; reject unsupported pairs before dispatch. `--credentials-dir` is explicit or the platform app-private sibling to the data directory, never inside vault/store/index/log directories. Validate explicit config/transport endpoint routing against Q2's ambient-override tests.

- [ ] **5. Red runtime test:** refuse a second potential network dispatch after injected loss with unknown outcome. Ensure cancellation while a tool awaits acknowledgement is bounded, channel closure drops futures and scoped threads join. The bounded bridge is Tokio `mpsc` capacity `16`; callbacks await send/oneshot rather than entering the worker queue. Workflow services it via `try_recv`/bounded waits so cancellation remains observable.

- [ ] **6. Green/commit:** `cargo test -p brn-ai -p brn-workflow -p brn -p brn-desktop --locked`; `cargo test -p brn-ai --locked --features test-support --test auth --test run`; `cargo check --workspace --locked`; native graph from Q1. Commit as `feat(ai): add qualified direct Rig runtime and settings`.

## A2: Durable BRN conversations and independent execution facts

**Prerequisite:** A1, N1; migration ownership serialized.

**Files:**
- Create: `crates/brn-store/src/ai.rs`, `tests/ai.rs`.
- Create: `crates/brn-workflow/tests/ai_history.rs`.
- Modify: store lib migrations/schema/open, workflow ai/lib, CLI conversations/status, desktop history rail/centre.

**Consumes:** A1 selection/history/outcome contracts, N1 schema 6 and existing `bind_operation`.

**Produces:** V7 tables `ai_settings`, `note_review_drafts`, `ai_review_contexts`, `ai_conversations`, `ai_turns`, `ai_events`, with explicit ordered effect/event records and content dependencies. `ai_settings` stores selected provider/model/verified subject without tokens; review records bind a note, immutable base, selected comment IDs and permission scope. Proposed workflow-facing record shapes:

```rust
pub enum Currentness { Current, Stale, NotEvaluated }
pub struct EvidenceDependency {
    pub note_id: uuid::Uuid,
    pub evidence: brn_retrieval::Evidence,
}
pub struct AiTurnStart {
    pub operation_id: uuid::Uuid,
    pub conversation_id: Option<uuid::Uuid>,
    pub selection: brn_ai::ProviderSelection,
    pub question: String,
    pub review_context: Option<uuid::Uuid>,
    pub context_transfer_approved: bool,
}
pub struct AiConversation {
    pub id: uuid::Uuid,
    pub selection: brn_ai::ProviderSelection,
    pub review_context: Option<uuid::Uuid>,
    pub history: brn_ai::HistoryBlob,
    pub dependencies: Vec<EvidenceDependency>,
    pub review_dependencies: Vec<ReviewDependency>,
}
pub struct ReviewDependency {
    pub review_context: uuid::Uuid,
    pub comment_id: uuid::Uuid,
    pub original_revision_id: uuid::Uuid,
    pub original_hash: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
}
pub struct AiTurnView {
    pub operation_id: uuid::Uuid,
    pub conversation_id: uuid::Uuid,
    pub text: String,
    pub local_status: brn_store::OperationStatus,
    pub remote_outcome: brn_ai::RemoteOutcome,
    pub currentness: Currentness,
    pub dependencies: Vec<EvidenceDependency>,
    pub review_dependencies: Vec<ReviewDependency>,
}
```

To preserve crate direction, put store's serialized provider/identity/history/outcome DTOs in `brn-store::ai` using identical fields/value spellings, with checked conversion in workflow. Store must not depend on brn-ai or retrieval: store `EvidenceDependency` embeds its own exact evidence DTO (existing strings/ranges/hash fields), not `brn_retrieval::Evidence`. Above shapes describe workflow-facing records; store methods below take their store-owned counterpart types, not dependency inversions.

Store methods: `prepare_ai_turn(start: &AiTurnStart) -> Result<AiTurnView>`, `append_ai_event(op: Uuid, sequence: u64, event: &AiEvent) -> Result<()>`, `finish_ai_turn(op: Uuid, final_record: &AiTurnFinal) -> Result<AiTurnView>`, `ai_conversation(id: Uuid) -> Result<Option<AiConversation>>`, `ai_turn(op: Uuid) -> Result<Option<AiTurnView>>`.

`AiEvent` is a store-only tagged record for dispatch intent, dispatch acknowledgement, model/tool terminal, tool arguments/result, bounded partial text, evidence/currentness and sanitized failure. `AiTurnFinal` contains final text/history, optional usage, local status, remote outcome, currentness and dependencies; same field content as the turn plus the serialized history/usage. Dispatch records are append-only; unique `(operation_id, sequence)` binds the event payload and rejects mismatched reuse.

`note_review_drafts` binds `(note_id, draft_id, base_version_id, base_hash)` with a unique draft ID. Add store `create_note_review_draft(op: Uuid, note: Uuid, base: Uuid, hash: [u8;32], text: &str) -> Result<Draft>`: create the existing draft/original revision and explicit association in one transaction using extracted existing draft helpers. Standalone pre-reset drafts have no inferred note association; matching text/title is not identity.

- [ ] **1. Red store replay test:** construct `AiTurnStart` with fresh operation/no conversation, synthetic provider/account/model and `"Synthetic question"`; `prepare_ai_turn` twice returns same conversation ID. Changing the same operation's question/account/model/review-context/transfer approval returns `OperationConflict`. A different local operation can append a new turn, not mutate the old one.

```rust
#[test]
fn ai_turn_preparation_replays_its_conversation() {
    use brn_store::ai::AiTurnStart;
    let data = tempfile::tempdir().unwrap();
    let (mut store, _) = brn_store::Store::open(data.path()).unwrap();
    let selection = serde_json::from_value(serde_json::json!({
        "provider": "chatgpt", "model": "synthetic-model",
        "account": {"provider": "chatgpt", "subject": "synthetic", "display": "Fixture"}
    })).unwrap();
    let request = AiTurnStart {
        operation_id: uuid::Uuid::new_v4(), conversation_id: None, selection,
        question: "Synthetic question".into(), review_context: None,
        context_transfer_approved: false,
    };
    let first = store.prepare_ai_turn(&request).unwrap();
    assert_eq!(
        store.prepare_ai_turn(&request).unwrap().conversation_id,
        first.conversation_id
    );
}
```

Run `cargo test -p brn-store --locked --test ai`; expect missing interface/schema.

- [ ] **2. Add V7 migration/checks:** existing V1-V6 content unchanged; old sessions/threads read-only and distinctly historical. New conversations use BRN UUIDs. Persist selection/review scope per turn, bound payload hash, sequence, history format/pin, usage when available and dependencies including source/version/hash/ranges/generation.

Implement workflow `select_provider(&mut self, selection: ProviderSelection) -> Result<()>` here: verify account cache/capability, persist nonsecret settings and only then update in-memory config. Add `brn ai select --provider chatgpt|copilot --model MODEL --account SUBJECT`; restart must recover exact selection. No choice if not configured, no selection after disconnect until reconnect/reselect, and no startup device login.

- [ ] **3. Implement transaction order:** prepare commits local identity before dispatch; append dispatch intent commits before callback permission; tool effects/result/provenance commit before returning a result; final outcome plus history commits in one transaction. Partial/failed/interrupted runs survive even if Rig never appends successful history. A final commit failure preserves any known remote terminal in recoverable event facts and reports local persistence failure; don't infer remote completion from local status.

- [ ] **4. Restart/crash tests:** child exits after prepared turn, dispatch intent, confirmed terminal and before final commit. Reopen marks active local operations interrupted, derives only provable remote facts from their journal and never dispatches again. Compare `NotDispatched` versus an intent-only `Unknown`; avoid claiming a committed intent proves the HTTP request was sent.

- [ ] **5. History tests:** round-trip actual qualified Rig messages/tool pairs, corrupted bytes, newer unsupported format/pin, missing tool result and unchanged replay. Selecting another provider/account for a conversation requires matching explicit transfer approval; no consent inferred from selection alone. Retain dependency union for all application-supplied note evidence, even when Rig windows messages out.

Review provenance has a separate dependency list and durable conversation scope. Scope mismatch on continuation is denied even with provider-transfer consent; choose a fresh conversation rather than treating a review-history quote as current knowledge.

- [ ] **6. Wire inspection consumers:** new `conversations list/show` includes backend/history label and independent fields, never fake `has_thread`/provider turn IDs. Keep old history distinctly inspectable. Do not resume old Codex sessions as Rig. Native history rail/centre and JSON DTOs reflect this distinction.

- [ ] **7. Green/commit:** `cargo test -p brn-store -p brn-workflow -p brn --locked`; `cargo check --workspace --locked`. Commit as `feat(store): journal durable Rig conversation outcomes`.

## A3: Complete guarded ask, read tools and shared cancellation

**Prerequisite:** A1/A2/N3. Read tools/conversations/currentness ship together.

**Files:**
- Create: workflow `src/{ask.rs,tools.rs}`, `tests/{rig_ask.rs,rig_tools.rs,rig_recovery.rs}`.
- Modify: workflow lib/worker/main/error; brn-ai run/tools/history.
- Modify: CLI ask/error/mod, tests `cli_{ask,signals,cancel}.rs`; desktop ai/native/centre/history/settings.
- Modify: shared workflow/CLI/desktop READMEs and deterministic fixture scripts.

**Consumes:** A1 bridge/runtime, A2 journal/history, N3 currentness and filtered search.

**Produces:**

```rust
pub struct AskRequest {
    pub operation_id: uuid::Uuid,
    pub conversation_id: Option<uuid::Uuid>,
    pub vault_id: uuid::Uuid,
    pub question: String,
    pub profile: brn_retrieval::Profile,
    pub review_context: Option<uuid::Uuid>,
    pub context_transfer_approved: bool,
}
pub enum AskEvent { Delta(String), ToolStatus(String), Terminal(AiTurnView) }
// On Workspace:
// ask_rig(&mut self, request: &AskRequest, cancel: &AtomicBool,
//         on_event: impl FnMut(AskEvent)) -> Result<AiTurnView, AskFailure>
// create_note_review(&mut self, op: Uuid, note: Uuid, observed: Uuid) -> Result<Draft>
// create_review_context(&mut self, op: Uuid, note: Uuid,
//                       source_version: &str, comments: &[Uuid]) -> Result<Uuid>
```

`AskFailure` retains operation/conversation, known recorded local status, independent remote outcome/currentness and typed message/category. Reuse the existing CLI error-context behavior while replacing sidecar-specific inference; keep legacy conversions only until D1.

Tools have strict Serde argument structs, byte/result limits and no uncontrolled filesystem input:

| Tool | Arguments | Return and check |
| --- | --- | --- |
| `search_vault` | query, profile, limit `1..=10` | Current validated evidence, bounded total bytes, reindex/approval errors explicit. |
| `read_note` | note UUID, start/end bytes | At most 16 KiB UTF-8 slice of approved current snapshot; exact provenance and byte boundaries. |
| `list_notes` | cursor, limit `1..=50` | Current eligible identity/title only; stable pagination. |
| `list_comments` | review-context UUID, cursor, limit `1..=50` | Allowed comments for explicitly selected note/base/review, labeled original provenance. |
| `read_comment` | review-context UUID, comment UUID | Same ownership/permission; original and mapped anchors distinguished. |

Tool schemas use `deny_unknown_fields`. The private bridge carries parsed closed-enum payloads; workflow owns capability scope (vault, exact selection, review context), eligibility and acknowledgements. Page cursors bind registry eligibility generation; changed pages fail/restart explicitly rather than leaking excluded rows.

`create_note_review` validates an approved current note and calls A2's explicitly linked draft creation. Expose `brn notes review NOTE_ID --observed-state UUID --operation UUID` and native review selection; add comments through existing draft comment commands. `create_review_context` verifies this association and each selected comment, captures the current approved base and maps old anchors conservatively through existing projection utilities. It never invents a note association for an old arbitrary draft. No original comment anchor is overwritten; ambiguous mappings remain unresolved.

- [ ] **1. Red deterministic ask test:** use disposable registered/opened/approved notes and Q3 replay HTTP, not fake sidecar. Assert one BRN operation/conversation, provisional deltas, real read/search tool result, terminal current answer and committed reloadable Rig history. Repeat identical operation: return existing recorded result without a completion request. Reuse with different input: fail before dispatch.

Run `cargo test -p brn-workflow --locked --test rig_ask`; expect missing `ask_rig`.

- [ ] **2. Implement owner-serviced run loop:** start scoped async runtime with owned request/history/config only; poll bridge on the store-owner thread while honoring cancel/deadline. Before each model/tool dispatch commit intent and validate all supplied dependencies/current eligibility. Acknowledge only after that succeeds. Service tool request directly, not via blocked worker queue. On tool return persist exact call/result/dependency before acknowledging Rig.

Coalesce provisional deltas to at most 32 KiB/200 ms per journal append; flush before any ordered effect/terminal. Output errors or queue closure cancel local futures, but don't erase already known durable/remote facts. No blocking native model inference on the runtime event loop.

- [ ] **3. Guard retained history:** validate the full dependency union before current-mode continuation, including note-derived assistant answers, tool outputs and any selected history window. If stale, return `StaleContext` before any completion request and require a fresh conversation. Preserve the old history with a visible historical label. Never solve this by dropping a citation/filter error and handing the same answer back to Rig.

Review context is explicit, durable, exact-note/base/comment scoped, and available only to comment tools in the selected review conversation. Historical original anchors are labeled review provenance, not ordinary current-note evidence. Keep their scope separate from current dependencies; current-mode conversations cannot gain these quoted bytes by guessing a review UUID.

- [ ] **4. Currentness/failure tests:** external edit/archive/unapproval between search/read/dispatch/terminal; changed generation; stale prior answer with no remaining source message; explicit review provenance; invalid UTF-8 ranges; forged/cross-note review; unknown/save/shell tool; budget loop; absent model/index; policy/filter failure; channel/ack persistence failure. Assert zero prohibited dispatches and no fallback.

After evidence stales mid-turn, preserve response/remote terminal and record `Currentness::Stale`; emit a failed-current-answer terminal/error, not a success current envelope. Already sent bytes cannot be unsent.

- [ ] **5. Cancellation tests:** before dispatch returns `NotDispatched`; after dispatch without confirmed terminal returns `Unknown`; confirmed model terminal plus local cancellation/persistence failure retain the confirmed remote fact. SIGINT/deadline/stdout/stderr failure must still expose durable IDs/status when known. Cancel while a tool acknowledgement waits, then assert scoped runtime exits and another worker action completes.

- [ ] **6. Wire all consumers in the same slice:** worker `Action::Ask` uses `AskRequest` with selected scope; CLI requires explicit vault and selected config (no `--codex` prerequisite). JSON stdout retains one envelope, deltas on stderr; textual output/stale warnings do not corrupt it. Native chat shows provisional/current/stale/interrupted state and exposes new-conversation action when continuation is blocked.

- [ ] **7. Green/commit:** `cargo test -p brn-ai -p brn-store -p brn-workflow -p brn -p brn-desktop --locked`; `cargo test -p brn-workflow -p brn --locked --features brn-workflow/test-support,brn/test-support --test rig_ask --test rig_tools --test rig_recovery --test cli_ask --test cli_signals --test cli_cancel`; `cargo check --workspace --locked`; native compile/fixture shell checks. Record disposable native stream/cancel/restart observations, not build-only usability. Commit as `feat(workflow): deliver guarded Rig ask and read tools`.

## A4: Typed candidates, exact provenance and manual adoption

**Prerequisite:** A3/N2 and existing draft/comment/mapping rules.

**Files:**
- Create: store `src/note_candidates.rs`, `tests/note_candidates.rs`.
- Create: workflow `src/candidates.rs`, `tests/rig_candidates.rs`.
- Create: desktop `src/candidates.rs`, CLI `tests/cli_candidates.rs`.
- Modify: store V8 lib/schema/open; brn-ai tools/run; workflow tools/worker/lib; CLI review/mod; desktop centre/native; READMEs.

**Consumes:** A3 exact approved note/review/AI-turn identity, N2 NoteStamp/buffer interface, existing exact edit/comment validators.

**Produces:** V8 `note_candidates` referencing new AI turn and immutable base version; typed tool `create_revision_candidate` enabled only here:

```rust
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateEdit {
    pub start_byte: usize,
    pub end_byte: usize,
    pub replacement: String,
    pub rationale: String,
}
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RevisionProposal {
    pub note_id: String,
    pub base_version_id: String,
    pub base_hash: String,
    pub addressed_comments: Vec<String>,
    pub edits: Vec<CandidateEdit>,
    pub unresolved_comments: Vec<String>,
}
// On Workspace:
// adopt_note_candidate(&mut self, op: Uuid, candidate: Uuid,
//                      expected: NoteStamp) -> Result<NoteView>
// note_candidate(&self, candidate: Uuid) -> Result<NoteCandidate>
// note_candidates(&self, note: Uuid) -> Result<Vec<NoteCandidate>>
```

Use the existing UUID/string representation at the schema boundary with checked parsing. `NoteCandidate` contains UUID, note/base/version/hash, origin new AI operation, original proposal, validated result bytes/hash, creation time and adoption state. Store neutral proposal/evidence DTOs; no dependency on workflow/Rig. `create_revision_candidate` is a typed tool using the same validator/persistence path as Rig typed generation, not an arbitrary-answer-copy action.

- [ ] **1. Red candidate separation test:** run a replay typed proposal against `"AéZ\n"` with replacement `[1,3) -> "b"` and an explicit selected comment. Assert result `"AbZ\n"` is a separate candidate, file and editing buffer unchanged, origin points to the new AI turn and original comment provenance unchanged.

Run `cargo test -p brn-workflow --locked --test rig_candidates`; expect absent tool/schema.

- [ ] **2. Implement one validator:** verify exact base source/version/hash/current approved note, correct byte boundaries, bounded output, sorted nonoverlapping edits, no ambiguous coincident insertions, comment ownership/permitted review context, unique disjoint addressed/unresolved sets covering selected comments, nonempty rationale and operation payload identity. Apply edits against immutable base bytes, not current mutable editor text. Reuse existing edit/anchor utilities where their exact semantics fit.

The pure edit helper in `candidates.rs` has signature `apply_candidate_edits(base: &str, edits: &[CandidateEdit]) -> Result<String, String>`. Its core is independent of Rig/SQL:

```rust
pub fn apply_candidate_edits(base: &str, edits: &[CandidateEdit]) -> Result<String, String> {
    let mut previous: Option<(usize, usize)> = None;
    for edit in edits {
        if edit.start_byte > edit.end_byte || edit.end_byte > base.len()
            || !base.is_char_boundary(edit.start_byte)
            || !base.is_char_boundary(edit.end_byte)
            || edit.rationale.trim().is_empty()
        {
            return Err("Invalid candidate range or rationale".into());
        }
        if let Some((start, end)) = previous
            && (edit.start_byte < end || edit.start_byte == start)
        {
            return Err("Overlapping or ambiguous candidate edits".into());
        }
        previous = Some((edit.start_byte, edit.end_byte));
    }
    let mut output = base.to_owned();
    for edit in edits.iter().rev() {
        output.replace_range(edit.start_byte..edit.end_byte, &edit.replacement);
    }
    if output.len() > crate::MAX_IMPORT_BYTES {
        return Err("Candidate exceeds byte limit".into());
    }
    Ok(output)
}
```

Add a pure red test replacing bytes `[1,3)` of `"AéZ\n"` to produce `"AbZ\n"`; `[2,3)` must fail. Check the final output size, not each intermediate reverse edit: a later-applied deletion can make a proposal within limit. The full validator bounds serialized proposal/replacement bytes before allocating, checks authority/comment identity and maps explicit failure to a typed workflow error; success here alone never persists a candidate.

- [ ] **3. Persist effect before returning:** tool effect ID is deterministically derived from parent operation/call ID; persist validated proposal/result/provenance and journalled result in one transaction. Same call/payload replay returns the same candidate, mismatched reuse fails. Crash after commit/before tool acknowledgement never makes a duplicate candidate or automatically retries a uncertain remote call.

- [ ] **4. Red invalid cases:** overlapping/splitting UTF-8/out-of-range/reordered ambiguous edits, stale base/hash, wrong note/comment, malformed typed JSON, duplicate/inconsistent comment sets, oversized candidate, unknown fields, changed file during generation and persistence failure. No candidate mutation or successful tool acknowledgement on error.

- [ ] **5. Implement manual review/adoption parity:** candidate show/compare commands and native review display exact immutable base versus result, rationale and unresolved comments. `brn revisions adopt-note CANDIDATE --file-state UUID --expected-generation N --operation UUID` applies only to a matching editor buffer; dirty newer edits require comparison/discard choice and are never silently replaced. Adoption increments/recoverably persists the buffer generation, not the Markdown file. Explicit N2 Save remains separate; neither adoption nor save approves search/publication.

`adopt_note_candidate` has no hidden discard flag: a dirty buffer fails with `NoteConflict`. A confirmed UI/CLI discard must first use N2's guarded reload with the exact observed/editor tokens, then submit a fresh adoption request. Comment originals are never rebased in place.

- [ ] **6. Preserve later work:** test a candidate arriving during typing, late adoption acknowledgements, stale file, repeated adoption operation, adoption restart and unchanged original comment anchors. Remove old current-answer "Save candidate" UI here; historical old candidates remain read-only distinguishable.

- [ ] **7. Green/commit:** `cargo test -p brn-store -p brn-workflow -p brn -p brn-desktop --locked`; `cargo test -p brn-workflow -p brn --locked --features brn-workflow/test-support,brn/test-support --test rig_candidates --test cli_candidates`; `cargo check --workspace --locked`; native build and disposable review/adoption/save observations. Commit as `feat(workflow): add exact typed note candidates`.
