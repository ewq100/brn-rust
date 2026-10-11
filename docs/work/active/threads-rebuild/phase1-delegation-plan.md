# Phase 1 explicit delegation implementation plan

**Goal:** Require one explicit owner choice before autonomous ordinary-note maintenance, while preserving manual work, applicable direct instructions, review, and checked-write compatibility.

**Architecture:** Retain SQLite, existing Settings booleans and the single checked commit writer. Add a defaulted choice marker and a host-only operation that applies the choice against current Settings and fences active runs in one transaction. No SQL schema bump, provider change, new dependency or old-BRN migration.

**Tech stack:** Rust 1.98.1, existing rusqlite/Serde/GPUI Kit and lockfile.

**Spec/authority:** Accepted first-use choice in [Threads target](../../../architecture/threads-target.md#accepted-adoption-priorities-11-october-2026). Owner requested Phase 0 PR/CI-gated merge and continued building on 11 October 2026. This is the next bounded Phase 1 increment, not completion of the whole foundation phase. Phase 0 native observation is explicitly deferred; that does not establish native evidence for this increment. Existing workflow takes precedence over optional skill approval ceremonies; execution and Sol subagent method are already authorized.

## Baseline and ownership

- Isolated worktree `brn-phase1-delegation`, branch `work/phase1-delegation`, baseline `7c95af16d1d377a29736f77a69f1e7343184271c` (PR #115 exact tested tree). Fast-forwarded onto merged baseline `53e5289e7221a4c3b66c6c454550311e46617a78`; identical Git tree.
- Core/app/runtime helper owns core types/store/tests, app settings and affected tests/examples, AI affected fixtures/tests/runtime only if required by actual behavior.
- Desktop/CLI helper owns desktop presentation/tests and `brn` command route/tests.
- Lead owns plan, canonical docs/status, integration and final independent review. Shared interfaces below are fixed; coordinate any revision before editing.
- Use fresh synthetic directories and persisted offline providers with empty credential locations in native fixtures. No provider calls, credential reads, user data, dependency upgrades or permission changes.

## Contract and interfaces

Retain `WorkspaceSettings.maintenance` and `review_first`; add public `delegation_chosen: bool` with Serde default false and omission when false. Old Settings nested in immutable requests must reserialize identically. Do not rewrite old revisions or applied operations.

Add core-exported `DelegationChoice::{MaintainOrdinaryNotes, ReviewFirst}` and `Workspace::choose_delegation(choice: DelegationChoice) -> AppResult<ApplyOutcome>`. The core writer operation applies the selection against current persisted Settings under `TransactionBehavior::Immediate`, preserving provider/model/effort/credential locations. It records a checked owner change and fences all Working runs in the same transaction, reusing the existing configure machinery. MaintainOrdinaryNotes sets marker=true, maintenance=true, review_first=false; ReviewFirst sets marker=true, maintenance=true, review_first=true. Pause remains an independent temporary suspension.

Standing maintenance requires a present active Settings record, marker=true, maintenance=true and review_first=false. Missing, archived or legacy unchosen Settings fail closed. Pending does not prevent read/search, owner Save/import/export/backup/history, owner-approved review, or properly scoped applicable direct instructions. Run bookkeeping remains possible as already allowed by the checked core.

Fresh bootstrap sets maintenance=false, review_first=false, marker=false. Legacy Settings remain byte-compatible and retain provider selections, but cannot grant unrecorded standing maintenance. `configure` preserves all current delegation fields at the writer boundary: passing changed policy fields cannot grant or alter delegation. Pause uses its own transactional policy patch, so a provider form opened before a choice cannot overwrite the newer policy. Provider configuration and pause/resume cannot choose on the owner's behalf; later mode changes use the explicit choice operation. Pending resume returns a clear choose-first error. Generic provider configuration cannot bypass the dedicated choice operation. Fully trusted owner ChangeRequests retain their existing privileged Settings capability for bootstrap and explicit host operations; model tools cannot create that authority. Core `pause_workspace(operation, paused, authority)` binds its boolean intent and retains receipt replay before rereading current policy.

The explicit choice operation must support immutable retry/replay and bind its intent, not overwrite unrelated settings with an earlier snapshot. No model tool may write Settings or invoke this host choice.

At run start, the host supplies a payload-free policy summary (Pending, Paused, ReviewFirst or MaintainOrdinaryNotes) from the same active Settings revision captured by the run scope. The summary excludes credential locations and cannot grant authority; the checked writer enforces the captured version and current policy.

## Review focus and behavioral checks

1. Old Settings request bytes/hashes and already-applied receipts still replay after reopen and policy change.
2. Missing/legacy/pending Settings deny standing writes while scoped direct instructions and owner changes still work; protected-note rules remain intact after delegation.
3. A concurrent provider update survives the choice; changed-choice retry fails; lost response retry returns the existing receipt.
4. Selecting a choice and pausing fence all Working runs atomically; stale independent connections cannot commit under old authority.
5. Native first-use choices route through the production service; navigation never silently selects a policy; test fixtures cannot start a live provider.

## Task 1: Core/application contract

Files: `crates/brn-threads-core/src/types.rs`, `src/store.rs`, focused core tests; `crates/brn-threads-app/src/settings.rs`, focused app tests, synthetic example and existing affected fixtures. AI files only for necessary fixture adaptation and a focused authority regression.

- [x] Add failing tests for fresh/legacy/missing Settings denial, pending resume/configure behavior, explicit scoped/owner writes, and unchanged legacy serialization/hash replay.
- [x] Implement additive marker and enum plus effective standing-policy checks; use no schema migration.
- [x] Implement transactional choose operation and shared fencing without copying a stale full Settings snapshot into the writer. Bind operation key to choice and retain replay-before-mutable-check behavior.
- [x] Test each choice, protection boundary, review acceptance, concurrent provider preservation in both directions, choice/Pause retry/replay and stale-run fencing.
- [x] Update fixtures that intentionally require autonomous authority to install explicitly chosen Settings. Do not preserve a production missing-Settings bypass for tests.
- [x] Run locked/offline core and app suites, focused AI Threads tests and affected strict all-target Clippy. Record meaningful red/green results; no zero-test proof.

## Task 2: Native first-use choice and CLI

Files: `crates/brn-desktop/src/threads_ui.rs`, state/focus tests as needed; `crates/brn/src/main.rs` and focused CLI tests.

- [x] Show an unobtrusive pending choice on Home and in Working knowledge Settings: explain ordinary-note maintenance, protected-note review, and the two buttons “Maintain ordinary notes” and “Review changes first”. Preserve normal manual navigation/work.
- [x] Route both buttons through `Workspace::choose_delegation`. Show success/error using existing outcome presentation, refresh persisted state, and hide the pending prompt once chosen. Existing Settings allow later explicit choice changes and Pause/Resume.
- [x] Add `brn ...` explicit choice command following current parser structure, with exactly `maintain` and `review-first` values, documented help, settings output including marker and clear invalid-value errors. Both routes call the same production operation. Do not add model tools.
- [x] Component tests click both choices and verify persisted state after reopen/navigation; pending navigation cannot grant authority, and manual work stays available. Persist unsupported offline provider and empty credential paths before UI creation.
- [x] CLI tests exercise fresh pending state, invalid choice, both choices and pending resume rejection in fresh data directories.
- [x] Run affected CLI/default desktop/native-component tests, native build and strict Clippy. Native visual observation remains separately reported if access is unavailable.

## Task 3: Documentation, review and integration

- [x] Update current target/behavior/invariants, app/runtime guidance and CLI help descriptions for actual pending/selected semantics. Keep later Stop-and-send, backups/upgrades and other foundation work explicitly pending.
- [x] Run format, docs links, diff checks and any newly affected gate; independent read-only review compares complete diff including new files against the baseline and this contract.
- [x] Resolve supported findings, rerun only affected checks, commit tested candidate and report exact source and limitations. The owner subsequently explicitly authorized publishing and merging this delegation increment after required CI, then continuing Phase 1. Release remains outside scope.

## Gates

- Scope/execution: authorized.
- Independent plan review: APPROVED by GPT-6.1 Sol; no plan corrections required.
- Implementation/automated checks: satisfied; exact commands and results in [evidence](phase1-delegation-evidence.md).
- Independent candidate review: APPROVED by GPT-6.1 Sol against `53e5289`; no code defects. Clarified provider configuration versus trusted owner groups.
- Native observation: unavailable until supported access succeeds; not a blocking reason to stop authorized code work.
- Integration: Phase 0 PR #115 merged after all required CI passed; post-merge CI is tracked in [evidence](phase1-delegation-evidence.md). The owner now authorizes this increment’s PR publication and merge after required CI, followed by further Phase 1 implementation.
