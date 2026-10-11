# Phase 1 Stop-and-send implementation plan

**Goal:** Let the owner send a new instruction to the active thread without losing the message or letting the old run keep writing.

**Architecture:** Reuse the checked SQLite writer and run fences. Atomically persist the new owner message and interrupt the exact run, then wait for that worker to finish before starting one fresh invocation. Keep a single in-memory pending send; the message itself is durable. No schema change, provider parallelism, dependency upgrade or new model tool.

**Spec/authority:** [Accepted Phase 1 priorities](../../../architecture/threads-target.md#accepted-adoption-priorities-11-october-2026). The owner explicitly authorized delegation PR publication/CI-gated merge and continued Phase 1 building without routine reauthorization. Native observation remains deferred. The established repository workflow governs execution; GPT-6.1 Sol helpers implement bounded tasks.

## Baseline and reuse

Isolated `brn-phase1-steering`, branch `work/phase1-steering`, starts at delegation PR #116 head `e6a26aaa2ebeded9a0508f89ce7750fb1ae18d34`; fast-forwarded onto its identical-tree merge `36ad2d61e5895c9a564dcff8f6eec1053376245b`. The existing runtime privately creates its Run inside the worker, so the UI currently cannot fence an exact Run during startup. Expose the existing preparation as an opaque handle rather than adding a parallel lifecycle. Existing core transaction helpers supply immutable prepare/apply and receipt replay. Existing composer drafts preserve destination-specific text.

## Contract and fixed interfaces

- `Store::stop_and_post(operation: &OperationId, run_id: &str, thread: &str, text: &str, authority: &HostAuthority) -> Result<ApplyOutcome>` is owner-only. In one Immediate transaction, bind exact run/thread/text intent, replay an existing receipt before mutable checks, validate active open thread and exact nonarchived Run membership, reject a newer Working invocation in that thread, append one owner Message, and change Working to Interrupted with checked fence increment. An already-terminal exact Run may accept the message without changing its terminal state, provided no newer Working run exists. Empty/oversized text, wrong thread, missing/archived run and fence overflow make no partial change. Changed retries are immutable-request errors. Use existing checked prepare/apply, not direct canonical record writes.
- `Workspace::stop_and_post(key: &str, run_id: &str, thread: &str, text: &str) -> AppResult<Receipt>` supplies a stable host operation key and exact intent binding. Success identifies one durable message; retries do not duplicate it.
- `brn_ai::threads::prepare_run(request: ThreadRunRequest) -> AiResult<PreparedThreadRun>` wraps existing `begin`. Opaque non-Clone handle owns request/scope/input and exposes only `run_id(&self) -> &str`. `run_prepared(prepared, cancel, emit)` consumes it; existing `run(request, cancel, emit)` remains a wrapper. Desktop prepares before spawning its worker and stores the exact Run ID. A cancelled or already fenced handle stops before credential/provider work. No extra authority is inherited by a fresh run.
- Desktop shows **Stop and send** only for a Run worker while viewing that worker's thread. Ordinary Ask remains unavailable during other work or on another destination. Existing drafts remain editable and destination labels remain accurate. Scoped protected-edit/decision buttons do not gain queued authority in this increment.
- Clicking captures current text and exact run/thread identity. Persist/fence first, then clear only the submitted draft and cancel the old worker. Failure retains text and old-worker controls. One immutable pending send waits for Finished; repeated clicks cannot replace or duplicate it.
- Start the queued invocation only after all old-worker events in the current batch are drained and that worker reports Finished. Refresh current settings/context first; pass `owner_request=None`. Navigation after clicking never retargets the message, steals focus or clears newer drafts. No second provider runs alongside the old one.
- Continue remains available for an open thread with an owner Message whose creation receipt follows the latest Run creation receipt (or with no Run). Use existing ordered receipts, not timestamps or a new persistent queue. This covers a terminal old Run and a replacement that never starts, including after reopen.
- Disconnect, startup failure or app exit keeps the saved message and provides an honest Continue path; no restart-persistent execution queue is implied. Stop while a replacement waits cancels automatic continuation but keeps the saved message. Resolved/archived destination or changed configuration is revalidated at fresh preparation.

## Implementation ownership and checks

### Task 1 — checked transaction, app wrapper and runtime handle

Owner: core/runtime Sol helper. Files: focused new core/app steering modules/tests with module declarations; `crates/brn-ai/src/threads.rs` and focused steering tests. Reuse core private helpers via crate visibility as needed; avoid unrelated extraction.

- [x] Add behavioral failure/retry tests; implement stop-and-post with atomic fencing and stable input identity.
- [x] Prove stale old-run writes and completion cannot commit; terminal race, newer active run, resolved/archived thread, wrong ID and rollback preserve message/run consistency.
- [x] Prove lost-response replay after later changes/reopen, changed text/run/thread rejection, existing committed receipt replay and no duplicate message.
- [x] Expose opaque prepare/run separation, preserving existing wrapper. Test prepared startup fencing, pre-cancel before credential access, fresh owner scope and same current-settings/input guards.
- [x] Run locked/offline core/app and AI Threads suites plus strict affected all-target Clippy. Use only synthetic data and offline paths.

### Task 2 — native orchestration and component tests

Owner: desktop Sol helper. Files: `threads_ui.rs`, `threads_state.rs` and focused native tests. Fixed shared interfaces above; do not edit core/runtime files.

- [x] Capture exact Run before spawn. Add one pending-send state and explicit availability/feedback for the active thread.
- [x] Route click through production stop-and-post; cancel after successful commit; drain old events before one fresh launch. Preserve newer drafts and navigation destination; Stop cancels pending auto-launch.
- [x] Component tests use persisted offline providers/no credential paths and synthetic worker channels. Exercise successful click, two clicks, waiting and Finished, disconnect, navigation/new typing, failed persistence, terminal race and no early new worker. Verify saved exact message and old-run fence through reopened production storage. Test the real pending dispatch without a live provider.
- [x] Run default desktop/native-test-support suites, native build, strict affected all-target Clippy and formatting. Report native observation separately.

### Task 3 — lead integration

- [x] Update canonical behavior/target, active routing/status and evidence; do not describe backups/upgrades as complete.
- [x] Independent plan review before implementation and full candidate review after tests. Fix supported findings and rerun affected checks.
- [ ] Commit the isolated candidate, publish and integrate only after applicable checks under the owner's continued-build/merge instruction; record exact CI and post-merge status. No release.

## Gates

- Scope: authorized; no routine owner question remains.
- Plan review: APPROVED by independent GPT-6.1 Sol; no required corrections.
- Implementation/checks: passed; independent GPT-6.1 Sol candidate review APPROVED, no actionable code defects. Exact commands in steering evidence.
- Native observation: deferred, not inferred from component tests.
- Delegation PR #116: merged at `36ad2d6` after all four required CI checks passed; all four required checks passed again on main.
- Remaining Phase 1: transactional Threads upgrades/pre-upgrade backups, external backup destination and recovery qualification remain outstanding.
