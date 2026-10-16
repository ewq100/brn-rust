# BRN: Rig-first agent capability implementation plan

**Prepared:** 15 October 2026  
**Repository:** `ewq100/brn-rust`  
**Inspected main:** `7e9d0376d677ed2a46bdbcd8933f55f9e1746a02`  
**Inspected dependency:** published Rig `0.44.0`  
**Status:** Proposed implementation handoff. Source reviewed; no code, builds, provider runs, or native interaction performed for this plan. The executing agent must reconcile the current local and remote trees before editing.

## 1. Objective and scope

Improve BRN's existing chat agent in place. Deliver dependable long-conversation context, recoverable multi-step work, owner-installable instruction skills, and an opt-in read-only MCP client. Reuse released Rig facilities wherever they fit BRN's contracts. Preserve the existing checked application service, SQLite authority, project boundaries, human drafts, exact review, and Undo.

The completed experience should support this synthetic journey:

> Open a project, select a reusable review skill, discuss a document through a long conversation, optionally consult an explicitly enabled external reference tool, prepare a source-supported revision, stop or restart the app, continue without losing the objective or repeating an applied change, and review the exact result.

This is not a Codex clone, another agent framework, or a storage rewrite. Better reference-assisted editing belongs to the existing modular-foundation writing program; this work integrates with its released interfaces rather than implementing that program again.

### Execution authority

This document alone records a proposal. When the owner provides the accompanying launch prompt, it assigns bounded local implementation, synthetic testing, documentation, independent review where available, and local commits. It does not inherit historical authority to push, merge, release, spend on providers, download models, use private data, or access external accounts. Use explicit current authority for those actions.

Resolve reversible implementation details without repeatedly asking the owner. Proceed through ready slices, not merely through planning. Isolate a genuine blocker and complete independent authorized work. Never reset unrelated work or silently redefine product constraints to finish a milestone.

### Explicit exclusions

Do not add scheduled background automation, an always-running daemon, arbitrary shell/browser/computer execution, skill scripts, a marketplace, inbound MCP access to BRN, connector writes, multi-agent swarms, silent model switching, or an unrelated compiler/Rig/GUI/storage/embedding upgrade. Do not bring unpublished Rig coding-agent crates into the dependency graph.

## 2. Verified starting points and the reuse decision

The following are observations at the inspected commit, not claims about unpushed work or a future baseline.

| Area | Source-backed starting point | Required direction |
|---|---|---|
| Runtime | BRN pins Rig 0.44.0 and already uses its agent/tool/stream/hook machinery. The AI package enables `agent`, `derive`, and `reqwest`. [B3, B4] | Extend this runtime. Do not build a replacement agent loop. |
| Conversation | Normal run input selects 40 recent messages, truncates their text, and adds earlier progress and operation results. Automatic Rig memory is disabled on this path. [B4] | Use Rig history policies with BRN-controlled context assembly and persistence. |
| Skills | Four compiled skills are advertised and loaded by `read_skill`; intake preloads two. [B5] | Evolve the existing catalogue; instruction loading does not need an executable-plugin framework. |
| Continuation | BRN already has progress retention, operation receipts, stop/restart fences, and durable queue controls. [B4, B6] | Enrich and test these, rather than introducing another job store. |
| Permissions and retention | BRN requires fresh authority/version checks and explicitly excludes opaque persisted runtime/provider transcripts. [B1] | Store small BRN continuation/context records, not serialized provider histories. |
| Current program | The modular-foundation pack is proposed, not started. Its owner decision requires `work/ui-polish` and `work/daily-workflow-reliability` to land first. [B7] | Reconcile this dependency and shared-file ownership before production edits. |
| Rig memory | Released code offers `TokenWindowMemory`, `MemoryPolicy`, `Compactor`, and `CompactingMemory`. The wrapper's summary state is process-local; summary size is outside the window budget. [R1] | Reuse policies/interfaces; add only necessary BRN storage, trust, and budget integration. |
| Rig compaction | `TemplateCompactor` rolls up text; it is not semantic summarization, and its default summary converts to a system message. [R1] | Do not use it as the production intelligent-memory feature or promote untrusted history into system authority. |
| Rig MCP | `rig-rmcp` adapts MCP tools into dynamic Rig tools and supports registry/liveness updates. [R2] | Use the adapter and maintained SDK, not a custom protocol client. |
| Rig checkpoints | `AgentRun` is serializable, but includes conversation/provider content; direct driving does not automatically run configured hooks/tools/memory. [R3] | Evaluate only in a disposable experiment. Persistent raw snapshots are outside this plan. |

**Ownership rule:** Rig owns generic model execution, tools, history policies, and protocol adapters. BRN owns what may be read, retained, changed, approved, and shown to the owner. A general Rig facility does not replace BRN's domain checks.

## 3. Sequence and coordination

Use one lead for shared contracts, dependency changes, schema changes, `threads.rs`, and integration. A helper may own isolated skill parsing, test fixtures, or an independent review after boundaries are agreed. No two agents edit shared runtime/core/UI files concurrently.

| Slice | Deliverable | Dependency |
|---|---|---|
| 0 | Current baseline, reuse proofs, ownership map | First; no production feature edits |
| 1A | Budgeted history and older-conversation access | Baseline and runtime ownership released |
| 1B | Persistent, source-linked semantic context | 1A |
| 2 | Reliable objective/progress continuation | 1B and existing queue/receipt contracts |
| 3 | Installable instruction-only skills | 1A; can be developed independently once file ownership is separated |
| 4 | Opt-in, read-only MCP connection | Budgeting, runtime policy, and external-data controls established |
| 5 | Integrated qualification, removal of obsolete paths, handoff | Implemented slices; writing integration qualified against its actual available interface |

The default order is 0 → 1A → 1B → 2 → 3 → 4 → 5. Break implementation into reviewable local commits; these are not preassigned pull-request numbers. Publish or merge only with current authority.

### Coordination with modular foundation

Before modifying production runtime, core, app, or desktop files, establish the actual disposition of the two finish-first branches and any active writing/foundation assignment. Branch existence and an old status paragraph do not establish whether an agent is active or a change has landed.

Use the shared run-start service if the foundation work has delivered it. Otherwise, coordinate a narrowly owned change with that program's lead or defer the overlapping integration. Do not independently extract another run-start service. This plan does not require completing every unrelated writing/intake milestone before isolated context tests can proceed, and it does not authorize bypassing an explicit finish-first decision.

## 4. Slice 0 — baseline and executable reuse proof

Read `AGENTS.md`, current status, the canonical Threads target/behavior/invariants, development workflow/verification, the active assignment, and the actual implementation. Inspect local HEAD, worktrees, uncommitted changes, fetched main, open PRs, and ownership. The SHA above is evidence, not a reset target.

Record a compact capability matrix: **already implemented / reusable upstream / needs a BRN adapter / deferred**. For each proposed new component, identify existing code and the exact pinned API before creating it. Remove work already completed by newer commits from this plan.

Build minimal credential-free checks against the resolved published dependency for:

- Token-window application and compactor invocation without automatic canonical message writes.
- MCP tool conversion, cancellation, and availability changes through Rig's adapter.
- A disposable `AgentRun` checkpoint round trip, including inspection of retained payloads and the difference between direct driving and hook-aware execution.

Prefer existing test facilities or examples. Keep experiments out of production paths. Inspect dependency/feature changes and the lockfile; enable only facilities used by the subsequent slice. Do not fork Rig to make this plan fit.

Run relevant existing baseline checks, including required default/native lanes where available. Record pre-existing failures rather than fixing unrelated programs. Read scripts before running them and use explicit fresh data directories.

**Exit:** a short active plan with baseline, file owners, verified API choices, acceptance IDs, and known blockers. Continue into the first ready implementation slice. A checkpoint incompatibility does not block context, skills, or MCP work.

## 5. Slice 1 — long-conversation context

### 1A. Budgeted history and exact older-discussion access

Replace the fixed recent-message assembly with a single bounded policy pipeline over BRN's canonical saved conversation. Prefer applying Rig's `MemoryPolicy`/`TokenWindowMemory` explicitly; a `ConversationMemory` adapter is acceptable only if it preserves host-owned append/clear semantics and does not duplicate final messages or persist tool transcripts.

Retaining `.without_memory()` is acceptable when BRN deliberately applies Rig policies before invoking the agent. Success is reuse of the correct mechanism, not removal of a particular method call.

Budget the complete request: instructions, project configuration, skill metadata/text, current owner request, document/reference context, recent history, semantic summary, tool schemas/results, response allowance, and a safety margin. Use validated model/provider information and a tested counter where practical. Label heuristic estimates as estimates; never hard-code a universal context window or silently drop the current request. Bound tool-result growth during the run as well as at initial prompt assembly.

Prioritize the current owner request and applicable project configuration. Make omitted historical coverage visible. Preserve message order and tool-call/result validity when that structure is present. If mandatory content cannot fit, produce a specific limit outcome before an invalid provider request; do not secretly switch models.

Give the agent bounded access to older permitted discussion and exact message identities. Extend an existing read/search route where practical; add one narrowly scoped conversation read tool only if the existing API cannot express it. Conversation evidence must not appear as canonical notes or leak another project's discussion. Read selected passages instead of returning the entire conversation.

### 1B. Semantic summary and restart persistence

Implement a small model-backed compactor through Rig's `Compactor` interface or an equivalently narrow adapter using that interface's contract. Use the current selected provider/model/effort for runtime compaction by default; no hidden secondary provider. Record its calls and usage. Live development qualification still needs current provider authority.

A summary should preserve the objective, relevant requirements, unresolved questions, proposals versus accepted outcomes, and references to the saved messages and actual operation receipts. Treat claims about acceptance as evidence to verify, never as authorization created by the summarizer. Host-owned identity/coverage/version stamps must be calculated outside the model.

Persist only a bounded derived summary and its coverage identity through BRN's existing checked storage facilities. Reuse existing appropriate records/cache mechanisms; introduce minimal storage changes only when needed. Do not overload an ordinary knowledge Note or create another conversation database. Indexes/exports must not accidentally treat generated summaries as source-of-truth knowledge.

Key reuse to conversation identity, exact covered message range/content digest, context visibility epoch, and relevant summarizer/instruction configuration. Handle appended messages incrementally. Invalidate or safely rebuild when source content, access, deletion/restoration, or applicable configuration changes. A cold restart must not replay all compaction calls for unchanged coverage.

Render the result as explicitly labelled derived context under BRN's existing trust hierarchy. Do not pass the default `TextSummary` system-message conversion through unchanged. A quoted instruction, tool response, or old request must not become a fresh owner grant. Keep high-value requirements linked to exact saved messages so uncertain summaries can be checked.

On summarizer failure, retain valid prior summary coverage, current request, and bounded recent context, and disclose the gap. Use one documented degradation policy—not a silent second runtime or a claim that all history was considered. Never delete original saved conversation. Long calls happen outside database transactions; persist only after checking that the result still belongs to the current context generation. Cancelled or superseded summary results cannot replace newer state.

**Acceptance:** C01–C06. Do not count a template rollup, larger message cap, or empty storage adapter as semantic continuity.

## 6. Slice 2 — recoverable multi-step work

Improve the current `retain_progress`, Run, queue, and receipt mechanisms. Persist a concise objective, bounded remaining-work list, open questions, and references to real prepared/applied operations. Reuse existing state where adequate; new type names or tables are not mandatory.

Separate model-reported progress from host-verifiable outcomes. A final model answer is not proof that a document changed, an action completed, or a queued item succeeded. Preserve truthful distinctions among finished, paused/interrupted, budget-limited, awaiting review, stale, and failed work without building a general workflow designer.

Continue with a fresh Rig invocation under current settings, current scope, and current authorization. Re-read necessary versions. Reconcile pending operation identities against checked receipts before doing work again. Preserve the existing rule that a new invocation does not inherit an expired protected-edit grant. Do not label fresh-invocation continuation as resuming the exact previous model state.

A bounded owner-started task may do several steps through the existing Rig loop. This slice does not introduce unattended restart, scheduled execution, or unlimited continuation. Keep Stop, failure, archive, and recovery pausing semantics intact. Make the remaining next step understandable in the existing chat activity/progress UI.

### Checkpoint decision

The Slice 0 checkpoint experiment must compare generic run serialization with BRN's retention invariant and current cancellation/hook behavior. Under the inspected contract, **small BRN progress records plus fresh invocations are the production design**. Raw serialized Rig histories remain disposable test material, not durable workspace state or backups.

Do not invent a redaction/replay engine or silently weaken the invariant to adopt checkpoints. A future exact-resume design that needs durable provider transcripts requires a separate explicit product decision. Reuse any suitable upstream transient execution capability only when it preserves the current contract and measurably reduces implementation, not simply because serialization exists.

**Acceptance:** R01–R04. A blocked exact-checkpoint adoption is a valid recorded decision, not a failure to implement semantic continuation.

## 7. Slice 3 — owner-installable instruction skills

Extend `threads_guides.rs`, existing skill files, `read_skill` validation/schema, run identity tracking, and the relevant settings/composer routes. Preserve built-in IDs and behavior. Do not mistake repository development instructions for BRN runtime skills.

Support three origins: packaged built-ins, owner-installed skills, and explicitly enabled project skills. Start with local folder import/installation and instruction/reference files only. Prefer the established `SKILL.md` frontmatter shape with `name` and `description`; use an existing maintained parser rather than implementing YAML. Describe the supported subset honestly.

Discovery lists approved metadata; selection/loading uses a frozen per-run catalogue. Preserve lazy loading so all installed instructions do not inflate every prompt. Keep existing built-in preloading only where justified by current behavior. Allow an explicit composer selection as well as the existing model-led choice among enabled skills.

Installation must be an owner action. Model-generated skill text may be a draft, not self-installation or self-enablement. Record stable identity, origin, scope, and content digest/version for every loaded skill. Prevent shadowing of built-ins and ambiguous name collisions. An edited installed package becomes a new version; do not silently replace bytes underneath a running invocation. Disable/revocation prevents further loads; when it changes runtime policy, use existing fencing semantics.

Reference loading must stay inside the installed package with bounded file/count/total-byte limits. Reject absolute paths, traversal, escaping symlinks, and unsupported executable content/dependencies. Do not automatically fetch remote files, run shell snippets, or enable MCP tools named by a skill. A skill requiring unavailable tools must be labelled unsupported, not advertised as fully compatible.

Add minimal native/CLI surfaces using existing patterns: list, inspect origin/requirements, install, enable/disable by scope, remove, and select. Show loaded skill identity in existing activity/details. Do not build a marketplace, general plugin manager, or separate tool registry for Markdown files.

Provide a synthetic instruction-only `document-review` example: check support, disagreements, unresolved decisions, and proposed next steps using available BRN tools. It must not modify source references or change permission rules.

**Acceptance:** S01–S04, including the whole path from installation through actual runtime loading and restart.

## 8. Slice 4 — opt-in, read-only MCP client

This is BRN calling an external MCP server, not exposing BRN to external agents. Use `rig-rmcp`/the facade's `rmcp` support and the maintained SDK for transport/protocol work. Register adapted tools through Rig's dynamic facilities. Qualify the resolved feature/dependency graph on required targets.

Begin with one supported transport and a locally controlled synthetic read-only server. Choose the smallest maintained transport supported by the pinned SDK and available host; do not implement every transport or every authentication flow. Ship a usable connection/configuration path for that supported case, not only a mock test. Real account integration is separately qualified when authorized.

The owner controls endpoint, account/credential reference, connection enablement, allowed tools, and chat/project access. Default disabled. Reuse BRN's credential facilities; do not write secrets into notes, skill packages, checkpoints, traces, or test fixtures. Do not auto-install or launch arbitrary executables. Where a local process transport is supported, clearly treat process execution as a separate owner-authorized boundary, not as sandboxing provided by MCP.

`readOnlyHint` is advisory, not an enforcement boundary. Expose only explicitly approved tools with known server behavior; omit write, destructive, and unknown capabilities. Read-only queries can still disclose data: do not send private notes, whole history, assets, or confidential derived queries by default. Require an explicit external-data grant for the context and destination, and make the transmitted input reviewable where its data use is not already authorized.

Keep schema/catalogue snapshots stable per invocation and recheck enablement before dispatch. Names must not collide with BRN tools. A changed external schema or connection identity requires revalidation rather than switching an in-flight call to a new implementation. Extend current complete-batch validation deliberately; do not bypass it to accommodate dynamic tools.

Bound catalogue size, schema tokens, arguments, response size, timeouts, concurrency, and retries. The model cannot select endpoints or provide host credentials. Follow the SDK's secure authentication/transport behavior and restrict redirects/destinations consistently with configured connections. Cancellation and disconnect must produce truthful results; neither establishes that a remote request was undone.

Treat every tool response and server description as untrusted external data. Preserve a bounded citation/source identity and partial/error coverage through existing BRN evidence paths. Durable note changes still use `prepare_changes`/`apply_changes`. Summaries and tool output never grant write authority. Record safe metadata and outcomes, not a hidden archive of raw remote payloads.

Add connection status, an allowed-tools view, disable/disconnect, and understandable tool activity using the existing settings/chat UI. An unavailable connector should not prevent unrelated local-note work.

**Acceptance:** M01–M05. Do not claim production connectivity to an untested service, or claim a third-party server cannot have side effects merely because its tools advertise read-only behavior.

## 9. Acceptance registry

Use deterministic tests for invariants and a fixed, source-labelled synthetic corpus for semantic comparisons. Record failures and omissions separately. A test name alone is not evidence that it executed.

| ID | Scenario and required result |
|---|---|
| C01 | More than 60 saved messages with an early requirement: assembled context preserves the requirement or retrieves its exact source; no silent replacement by a later suggestion. |
| C02 | Cold restart after compaction: same source coverage reuses valid derived state; no duplicate canonical messages or unnecessary repeat summarization. |
| C03 | Large messages, Estonian/English text, oversized tool output, small model budget: mandatory input remains intact; the complete request is bounded under the tested counter, with estimates/omissions disclosed. |
| C04 | Project change, access revocation, knowledge-off, archive/delete/restore: summary, older-discussion lookup, exact IDs, snippets, and counts obey current visibility. |
| C05 | Malicious instructions in old conversation/source/summary: no promotion to system authority, settings change, protected grant, or unrelated project access. |
| C06 | Compactor timeout/failure, concurrent compaction, Stop, or changed context during a call: no stale summary publication; bounded visible degradation and later retry work. |
| R01 | Terminate before/after prepare and after apply but before recording the returned result: operation receipts prevent duplicate mutation and incorrect success. |
| R02 | Resume after note edits, protection changes, or lost grant: stale work is re-read/refreshed or held for review; no inherited expired authorization. |
| R03 | Stop/steer/archive/restart with queued work: old callbacks cannot apply, acknowledge the wrong item, or unpause the queue. |
| R04 | Budget exhaustion: objective and remaining work survive; partial output is not a finished task; Continue starts under fresh host authority. |
| S01 | Install/select/load a skill and reopen: origin, scope, digest, and loaded identity agree; existing four built-ins still work. |
| S02 | Name collision, malformed/oversized manifest, traversal, escaping symlink, script/tool dependency: reject or clearly mark unsupported without execution. |
| S03 | Disable/update a skill during a run: no mixed versions or silent capability changes; fresh runs observe current configuration. |
| S04 | Project-only skill and references stay scoped; adversarial skill instructions cannot enable a connector, alter grants, or bypass checked writes. |
| M01 | Known local server: discover approved tools, invoke through the real Rig adapter, show a sourced result, disable, and prove later dispatch is denied. |
| M02 | A falsely labelled read-only or newly advertised unknown tool is not auto-enabled; name/schema changes cannot bypass validation. |
| M03 | Private context without an external-data grant produces zero outbound private payload; approved data use is destination/scoped and reviewable. |
| M04 | Timeout, disconnect, invalid result, over-limit response, retry, and Stop: bounded execution, truthful partial/error state, no fabricated empty-success result. |
| M05 | Injection in tool text/server metadata cannot call privileged BRN operations, change endpoint/account, or install skills; secret/payload canaries stay out of logs/backups. |
| I01 | Long project conversation + selected skill + optional MCP + stopped revision task + restart/Continue + exact review/Undo: end-to-end result preserves source and operation identity. |
| I02 | Existing protected edits, full import, drafts/comments, helper research, queue, search, export, backup/restore, and required build lanes do not regress. |

For semantic qualification, use the same fixtures, model/account/effort, and comparable repeated runs against baseline and candidate. Track requirement retention, evidence correctness, unsupported claims, completion, latency, input/output/cached tokens, compactor/helper overhead, and explicitly incomplete coverage. Do not claim superiority from one successful run or enforce invented zero-variance latency targets. Deterministic isolation/authority/receipt tests must all pass.

## 10. Slice 5 — integration and completion

Maintain one production route for each delivered capability. Remove the replaced fixed-history policy, obsolete wiring, stale documentation, and tests tied only to a retired mechanism once the new path is qualified. Preserve existing reader/recovery compatibility and legitimate bounded error handling; do not leave parallel production engines as silent fallbacks.

Keep changes inside existing responsibilities: `brn-ai` for runtime adapters; `brn-threads-app` for host coordination; `brn-threads-core` for checked persistence/authority; `brn-desktop` and CLI for thin UI surfaces; `agent/` for runtime instructions. A small module split is acceptable; a new crate/trait needs a demonstrated boundary rather than a placeholder architecture.

Storage changes must use existing migration/backup/recovery policy. Test upgrade, interrupted upgrade, reopen, backup, restore, and derived-cache regeneration on synthetic copies. Never downgrade/reset an owner store or treat source rollback as automatically restoring schema compatibility.

Run relevant repository scripts and the required core/native CI lanes. At the inspected baseline these are Ubuntu core/CLI, macOS core/CLI, macOS native UI/state, and macOS native retrieval/combined. Use current verification instructions for exact commands and prerequisites; do not substitute default tests for native coverage. [B2, B8]

Use selected-provider synthetic journeys only with current authority. If access is unavailable, finish deterministic implementation and record semantic qualification as blocked—not passed. If the Mac is locked, perform no computer interaction; collect one consolidated native qualification task while completing headless checks. Never label widget/state tests as native observation.

Request proportional independent review of context privacy, summary authority, dynamic-tool validation, skill trust, and continuation idempotency. Fix findings and rerun affected tests. A review does not itself authorize publication.

### Deliverables and final report

Install one short active `plan.md` under the existing repository convention, proposed location `docs/work/active/agent-capabilities/`, only after reconciling current assignments. Retain this detailed handoff as supporting material rather than duplicating canonical architecture prose. Update affected behavior/invariant summaries with justified additive contracts.

Record evidence with exact tree/commit, environment, feature flags, command, executed-test counts, result, and limitations. Include the reuse/adapt/build decisions, removed paths, optional-feature state, schema/rollback notes, remaining native/live checks, and next action.

The final agent report must distinguish **implemented, deterministically verified, live-model qualified, native-observed, owner-accepted, and merged**. Show the user-visible behavior delivered, not only files changed. Do not claim complete parity with Codex or generic exact checkpoint recovery.

## 11. Source ledger

Source references describe the inspected baseline. Revalidate on the execution tree. Paths are repository-relative unless noted.

- **B1 — retention, permission, state, and project invariants:** `docs/architecture/invariants.md` at BRN `7e9d0376d677ed2a46bdbcd8933f55f9e1746a02`.
- **B2 — assignment and development rules:** `AGENTS.md`, `docs/development/workflow.md`, `docs/development/verification.md` at the same commit.
- **B3 — released Rig version/features:** `crates/brn-ai/Cargo.toml` and `crates/brn-ai/README.md` at the same commit.
- **B4 — current runtime/context/tools:** `crates/brn-ai/src/threads.rs`, especially `begin`, `run_model`, registration, and validation; observed source blob `c203d2b2a31ac8535d08f91947a86bc85297a463`.
- **B5 — skill loading and guidance:** `crates/brn-ai/src/threads_guides.rs`, `agent/BRN.md`, and `agent/skills/*/SKILL.md` at the same commit.
- **B6 — queue, research, and steering:** `crates/brn-threads-app/src/chat.rs`, `crates/brn-ai/src/threads_research.rs`, and `crates/brn-ai/src/threads_steering_tests.rs` at the same commit.
- **B7 — writing/foundation scope and sequencing:** `docs/work/active/modular-foundation/README.md` and `proposal/final/01-master-plan.md` beneath that folder at the same commit.
- **B8 — current-status qualifications:** `docs/status.md`, `.github/workflows/ci.yml`, and the branch's required check contexts. Status text contains dated entries and must be reconciled with source and actual integration receipts.
- **R1 — memory policy and compaction:** `0xPlaygrounds/rig`, tag `v0.44.0`, `crates/rig-memory/src/lib.rs` (source blob `ee8a5462854331ac6ced156f90334f602a63aa1e`) and the core memory interfaces. The release-tag source takes precedence over unversioned examples.
- **R2 — MCP:** the same Rig tag, `crates/rig-rmcp/README.md` and `src/lib.rs` facade exports.
- **R3 — agent execution/checkpoint caveats:** the same Rig tag, `crates/rig-agent/README.md`, especially “The run protocol” (source blob `05b5ae80fb838f28eb84d361523394f5db6ed932`).
