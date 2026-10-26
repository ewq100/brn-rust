# State ownership and test value audit

Source baseline: `bc699c4164147354b96e433b6327e194275102b0`, inspected 26 October 2026. This is a bounded source audit, not a new passing test run or a claim that every workflow has been enumerated. No Rust builds, live models, Mac access or test deletions were performed. Current Capture/follow-up implementation is owned by the other batch lanes; this receipt does not qualify their later changes.

**Recommendation:** keep the existing state owners; prioritize a small Search transition owner, then asynchronous dialog identity where a concrete defect warrants it. Restore already has meaningful phases and safety tests. There is no evidence supporting a universal reducer or blanket test deletion. Investigate repeated CI helper preparation before reducing behavioral coverage.

## What the earlier plan already accomplished

The [historical workflow inventory](../vacation-campaign/workflows.md) was explicitly a provisional 25-domain desktop snapshot, not a conversion quota. Its “unqualified” labels included native and owner gates; they do not mean the code lacks state management. The [campaign plan](../vacation-campaign/plan.md) subsequently prioritized agent-first work and bounded ownership improvements. Current [strategic guidance](../../../development/strategic-programming.md) requires actual responsibility reduction, independently specified expectations and retained real interaction checks.

| Boundary | Current source evidence | What is already owned; remaining limit |
|---|---|---|
| Note editing | [note.rs](../../../../crates/brn-writing-ui/src/note.rs) | `NoteLifecycle`: Closed/Open, Clean/Editing/BlockedElsewhere, Read/Write; generation-aware exit snapshot and durable editing service. Desktop and reusable writing host share it. GPUI effect methods remain intentionally alongside coordination; not a pure reducer everywhere. |
| Exact review | [review.rs](../../../../crates/brn-writing-ui/src/review.rs) | `ReviewLifecycle`: Closed/Open, Changes/Read/Editing and exact candidate checks. Service still owns preparation, Apply, version/dirty guards. Do not duplicate this policy in a new UI state machine. |
| Run transport | [threads_worker.rs](../../../../crates/brn-desktop/src/threads_worker.rs) | Typed `WorkerOperation` pairs Run identity with applicable worker operations. Non-Run effects remain distinct. Durable Run/queue decisions belong to app/core. |
| Run activity | [threads_run_activity.rs](../../../../crates/brn-desktop/src/threads_run_activity.rs) | Actual Idle/Live/Ended transition owner, stale context/terminal rejection and bounded Unicode/duplicate observations. Three dedicated transition tests exist. This is completed ownership work, not an outstanding rewrite. |
| Notices | [threads_notice.rs](../../../../crates/brn-desktop/src/threads_notice.rs) | Hidden/Ordinary/RememberedWork keeps text, tone and work provenance together; composer recovery remains separately keyed. |
| Continuation and drafts | [threads_state.rs](../../../../crates/brn-desktop/src/threads_state.rs) | `ContinuePath`, continuation guards and destination-keyed drafts already have deterministic tests. A module with several pure decisions need not become one global reducer. |
| Restore | [threads_restore.rs](../../../../crates/brn-desktop/src/threads_restore.rs) | Idle/Validating/Review/Publishing/Complete, generation-bound pickers and owned `PreparedRestore`. Explicit confirmation consumes the prepared value. Transitions still depend on GPUI/worker effects; core staging/publication remains authoritative. |

These are source facts, not full native acceptance claims. The old inventory's counts should not be promoted into today's percentage-complete claim.

## Remaining useful transitions

### First: Search request and page ownership

[threads_search.rs](../../../../crates/brn-desktop/src/threads_search.rs) currently coordinates `requests`, `fresh`, two `Lane` values (`awaiting`, cursor history, current cursor, task, cancellation), result pages, hits and errors inside callbacks. `search_answered` already rejects replies with no matching lane request. This is working safety logic, not evidence of a present stale-result bug.

A bounded extraction can make these decisions independently testable while leaving tasks, cancellation handles and rendering in the adapter:

| Event | Required transition / invariant |
|---|---|
| New nonempty query | Invalidate both prior lanes, clear prior visible results, allocate one identity for both first pages; effects cancel old tickets and issue the query. |
| Query cleared | No page or hit remains visible; delayed old replies cannot repopulate it. |
| Next/Previous for one lane | Update only that lane's cursor/history and pending identity; preserve the other lane's completed page. |
| Reply for current identity | Publish only matching lanes, clear their pending state, retain unrelated lane state. |
| Old reply or canceled request | No visible mutation. |
| Current failure, then retry | Failure is explicit, not an empty successful result; retry does not restore obsolete cursors or old query results. |
| New query during paging | Both old page requests become ineligible, even if replies arrive in opposite order. |

Use contract-authored sequence expectations for A-query/B-query/A-reply, independent lane replies, failure/retry and clear/reply. Keep one real widget/worker wiring journey per consequential route. The app search worker's privacy, derived-index repair and exact-quote tests remain necessary: a UI reducer cannot prove them. This recommendation is not authority to replace the search service or build a generic event bus.

### Then: asynchronous dialogs, and Restore only if justified

Picker/dialog generations, target versions and cancellation are worthwhile boundaries when an actual workflow duplicates cleanup or permits stale completion. First inventory existing generation checks; do not infer a missing guard from the absence of a reducer name. Capture's new lifecycle is separately designed by its implementation lane.

Restore already rejects stale picker results and has seven focused tests. A future extraction could represent Validate → Ready → Confirm → Publishing → Complete, with Cancel during validation waiting for cleanup and Publish failure retaining an uncertain-destination warning. `PreparedRestore` must remain an owned, single-consumption capability; a pure state enum must not replace filesystem validation or authorize publication. Prioritize this below Search unless a new failure demonstrates repeated transition mistakes. An out-of-phase completion should be explicitly rejected or diagnosed, not silently treated as success; no reachable violation was reproduced in this audit.

## Behavior → layer → unique failure protected

Runtime is **unmeasured per test family** in this audit. The CI job durations below include setup/build/lint/test and cannot supply those missing timings.

| Critical behavior | Existing evidence location | Why the layers are not interchangeable |
|---|---|---|
| Bound writer, stale Save, recovery bytes | [core edit_tokens](../../../../crates/brn-threads-core/tests/edit_tokens.rs); [desktop state](../../../../crates/brn-desktop/src/threads_state.rs); [document widgets](../../../../crates/brn-desktop/src/threads_document_tests.rs) | Core tests reject wrong tokens and preserve reopened buffers. `late_checked_refresh_preserves_newer_typed_buffer_and_stale_save` checks the editor adapter; `local_editor_undo_after_apply_and_refresh_cannot_resurrect_or_save_stale_text` checks toolkit Undo behavior unavailable to core tests. Retain all three responsibilities. |
| Exact review and later human edits | [core writing](../../../../crates/brn-threads-core/tests/writing.rs); [review widgets](../../../../crates/brn-desktop/src/threads_review_tests.rs) | Dirty/version/protection rejection versus displayed candidate, footer, focus and selected-group wiring. Similar Apply/Undo names do not imply equivalent faults. |
| Actions: unknowns, clear, replay, concurrent edit | [app action_followups](../../../../crates/brn-threads-app/tests/action_followups.rs); [core action_followups](../../../../crates/brn-threads-core/tests/action_followups.rs) | Schema/validation versus prepared review, source staleness, reopen/Undo and duplicate prevention. Authored ambiguous-meeting fixtures prove deterministic behavior, not live semantic judgment. |
| Search privacy, rebuild and cancellation | [search_worker](../../../../crates/brn-threads-app/tests/search_worker.rs); [search_messages](../../../../crates/brn-threads-app/tests/search_messages.rs) | `chat_only_notes_and_chat_sources_stay_in_their_chat`, failed-not-empty replies, dropped tickets and index recreation protect data boundaries independent of UI paging. |
| Inbox replay and interruption | [manual_inbox](../../../../crates/brn-threads-app/tests/manual_inbox.rs) | Fake conversion injects precise crash/failure/changed-file states. Actual Stock DOCX/PNG witness proves real converter identity, exact retained figure and no reconversion after reopen. One does not replace the other. Keep killed-process and prepared-output-quota recovery coverage. |
| Restore and backups | [core backup_directory](../../../../crates/brn-threads-core/tests/backup_directory.rs); [Restore widgets](../../../../crates/brn-desktop/src/threads_restore_tests.rs) | Canonical destination/receipt guards versus confirmation/cancel/stale picker, disconnected worker and new recovery window. Actual OS picker remains a separate acceptance gate. |
| Run dispatch/recovery | [shared_run](../../../../crates/brn-threads-app/tests/shared_run.rs); [run_recovery](../../../../crates/brn-threads-app/tests/run_recovery.rs); [Run activity](../../../../crates/brn-desktop/src/threads_run_activity.rs) | Durable identity/recovery differs from presentation termination and context switching. Keep queue/privacy and crash boundaries. |

## Test reduction candidates and non-candidates

No test is proven redundant or obsolete enough for deletion by this bounded pass. Below are reviewable opportunities, with the evidence required before changing coverage.

1. **CI feature overlap — candidate, not proven duplication.** [ci.yml](../../../../.github/workflows/ci.yml) runs default workspace, native UI state, native widget support, and combined native UI/retrieval. [desktop features](../../../../crates/brn-desktop/Cargo.toml) show `native-test-support` adds GPUI/writing test support; widget modules in `threads_ui.rs` are gated on it, while presentation/state tests also compile without it. The combined state lane does not enable widget support. Therefore deleting native widget execution in favor of combined state is wrong. For potential repeated state tests, capture `--list` from already-built exact feature binaries and map test identities plus `cfg`/dependency behavior before narrowing commands. Keep builds for supported feature compositions even if an identical assertion can eventually run once per platform.
2. **Helper builds across jobs — plausible larger saving.** Core, native UI and retrieval each prepare both Stock/adapted helper graphs on Unix. That is repeated work in workflow source, but cache warmth is unknown; the measured preparation step is substantial (see below). Separate fetching, adaptation and compilation within that step; investigate immutable artifacts keyed by OS/architecture, toolchain, both lockfiles and adapter/source identity. Preserve both helper contract tests and actual helper integration. No cache/artifact change is made here; unsafe shared-target identity reuse is not an optimization.
3. **Polling fixtures — timeout fragility hypothesis.** Restore `finish` loops up to 500 times with 10ms sleep while polling the real worker. Similar patterns exist in backup/intake fixtures. This establishes a nominal five-second wait budget, not a measured flake rate. Prefer an explicit completion barrier/deadline with diagnostic state when changing these tests; retain a real asynchronous boundary witness. Do not fix load sensitivity with blind reruns or unexplained larger sleeps.
4. **Manual ignored cross-host intake — coverage gap, not dead code.** `threads_intake_tests.rs` intentionally ignores the CLI-to-desktop shared fixture unless `BRN_DWR_SHARED_DATA`, `BRN_DWR_SHARED_SOURCE` and `BRN_DWR_SHARED_CHAT` are provided. Keep its unique handoff contract and record whether a dedicated authorized journey exercises it. Ordinary green CI does not count it as passed.
5. **Zero-test invocation — zero evidence, not a useless test.** The [preserved initial filter receipt](../daily-followups-inbox-restore/evidence/mac-pending-provenance-20261026/initial-filter-zero-tests.txt) compiled in 1m36s and selected zero of 515 tests. This is an invocation/filter failure. Require a nonzero selected-count receipt for focused commands; preserve the failed attempt and corrected evidence. Do not delete underlying tests because a bad filter missed them.
6. **Historical regression fixtures — retain absent equivalence proof.** Frozen writing evidence/schema migration readers and exact replay tests cover supported old data. Their old names/versions do not make them obsolete. Remove only with a deliberately retired compatibility contract and verified replacement boundary.

Historical cloud Git-ancestor credential restrictions, stale shared Cargo target artifacts and platform-specific input fixture errors are environment/harness findings, not newly discovered product regressions. Conversely, real footer/menu/scrolling regressions cannot be dismissed as flaky merely because later corrections passed. This audit did not repeat tests or estimate a flake rate.

## Measured CI snapshot

Input: `/workspace/brn-next-evidence/pr229-merged-main-ci.json`; [compact derived data](state-test-ci-summary.json). [Resulting-main run 40323786496](https://github.com/ewq100/brn-rust/actions/runs/40323786496) is bound to baseline `bc699c41`. Durations are `completedAt - startedAt`, seconds resolution, including the whole job; parallel job times must not be added as elapsed wall time or equated to billing units.

| Job | Result | Duration |
|---|---|---:|
| Documentation/tooling | Passed | 26s |
| Core/CLI Linux | Passed | 1,341s (22m21s) |
| Core/CLI Mac | Passed | 980s (16m20s) |
| Native UI Linux | Passed | 800s (13m20s) |
| Native UI Mac | Passed | 364s (6m04s) |
| Retrieval/combined Linux | Passed | 678s (11m18s) |
| Retrieval/combined Mac | Passed | 555s (9m15s) |
| Optional Windows core / native UI / retrieval | Failed | 179s / 239s / 254s |

The supplied saved JSON has no step timestamps. A subsequent ordinary read-only `gh api repos/ewq100/brn-rust/actions/runs/40323786496/jobs --paginate` supplies them; the supporting summary preserves non-Windows step durations. No log endpoint was accessed.

| Step (seconds, entire command including any compilation) | Linux | Mac |
|---|---:|---:|
| Core: prepare both helper graphs | 535 | 138 |
| Native UI: prepare both helper graphs | 495 | 71 |
| Retrieval: prepare both helper graphs | 377 | 96 |
| Core: default workspace test | 278 | 333 |
| Core: helper tests + Clippy | 334 | 217 |
| Core: capability fixture test | 99 | 127 |
| Native UI: reusable writing tests | 71 | 52 |
| Native UI: state tests | 39 | 30 |
| Native UI: widget tests | 76 | 78 |
| Retrieval: app tests with native feature | 94 | 164 |
| Retrieval: combined desktop state | 64 | 93 |

Helper preparation occupies 535/1,341 seconds of the slowest successful job and repeats in other jobs. This supports measuring artifact reuse first; it does not establish a 535-second realizable saving because fetching/adaptation/build and cache effects are combined. Likewise test step times include compilation: no per-test execution cost or cache-hit counter is available. No denied Windows logs were downloaded or inspected; Windows implementation remains outside this task. Seven non-Windows jobs passed in this snapshot. One observation is not a runtime trend, a controlled platform comparison or native owner acceptance.

## Concrete next gates

- For Search, review a small event/state contract and sequence tests, preserving real worker/GUI wiring and privacy tests; compare changed responsibilities rather than lines deleted.
- For CI speed, use the recorded step timing and collect test lists on a normal authorized run before selecting artifact reuse or narrower repeated test commands. Require identical meaningful coverage and correct executable identity, not just a faster green result.
- For any proposed test deletion, record the unique failure it previously caught, the replacement that still catches it, applicable features/platforms and measured cost. No numerical deletion target.
- A narrowly scoped mutation of a changed high-risk stale-reply/version guard could validate assertion sensitivity if needed; none was performed or made a blanket gate here.
- Mac rendering, keyboard focus, accessibility, real pickers and owner acceptance stay explicit separate gates. This source audit does not close them.
