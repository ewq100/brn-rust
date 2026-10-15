# Round 3 docs completion: round-3 notice texts, rebased hashes and the remaining handoff limits

This docs-only pass closes the five issues from the first round-3 review, and touches only `evidence.md`, `plan.md` and `round3-verification.md`. The owner's B1/B2 checks and the U1/U6 notes now expect the round-3 notices. The cross-surface reuse precondition is stated, and a new owner manual check 5 re-supplies a CLI-supplied file through the macOS picker. The non-persisted technical cause is recorded as a UI-agent limitation. The plan's header and item 3 are marked superseded by item 6, and the mixed `begin-edit` error formats are on the `work/cli-agent-contract` list. Evidence line 3 names base `771116f0` and the rebased commits, and it labels `34c9239d`, `422c9582` and `dfe698aa` as history behind the `work/daily-workflow-reliability-pre-rebase` (`914e2e6f`) pointer.

Watch for: nothing blocking. Two small text issues remain. `plan.md` item 4 still says "the desktop hookup awaiting approval" (confirmed). The picker-spelling precondition and the technical-cause limitation are each written out in full three or more times (confirmed).

**Verdict**: APPROVED

## High-level view

Every notice string quoted as current state matches `crates/brn-ai/src/threads.rs` exactly. That covers B2 and U6 ("Run failed: The provider account needs reconnecting. Continue when the provider is ready.", built from `threads.rs:2304` and `PROVIDER_RECOVERY` at `:2285`), B1 and U1 ("Run interrupted. Continue starts fresh.", `INTERRUPTED_ATTENTION` at `:2276`) and all 16 rows of the round-3 table, whose character counts (34 to 89) are right. Round-1 texts remain only in rows or sections labelled round 1, before, history or base tree.

The current-state hashes `771116f0`, `9f70efa7`, `2530805f`, `145b8884`, `c408b1d4` and `3a7c3d25` all resolve as commits, and `771116f0` is an ancestor of HEAD. The backup ref resolves to `914e2e6f`. `14ad8577`, `f80554fe` and `b49fb660` also resolve. They are docs commits and appear only in the `rebase-notes.md` map, because no current-state text referred to their pre-rebase originals.

The status matrix and "Integration risks with `work/ui-polish`" name the U4 variant `brn_threads_core::Error::EditSessionOpen { note, session, base_version }` through `AppError::Core`, plus the CLI `edit_session_open` JSON. They also give the new notice texts, U2 done here in `threads_ui.rs` and the new `threads_intake_tests.rs`, and the U3 finding that `ThreadRow` drops `Attention.record`. The quoted U4 messages match `brn-threads-core/src/lib.rs:48`, `brn/src/main.rs:900` and `brn-desktop/src/threads_ui.rs:1453`.

Owner manual check 5 can be run as written. It passes `--data-dir` first, as the CLI requires, and uses the CLI chat title "Useful information intake" (`main.rs:845`) and the CLI's raw `--source` string as the expected locator. The expected two owner Messages follow from integration item 5.

Only the three docs files changed: no code, no `docs/status.md`, nothing staged. `git diff --check` is clean, and the link check reports 84 files, 386 local links and 0 failures. The cargo suites were not re-run, as instructed.

<details>
<summary>Issues (2)</summary>

1. **Item 4 still says the hookup awaits approval (non-blocking, confirmed)**: `plan.md:106` lists "the desktop hookup awaiting approval" among the remaining limitations, and the item 6 supersede note covers only the header and item 3. Add "(superseded by item 6)" or drop the phrase.
2. **Limitations restated in full in several places (non-blocking, confirmed)**: the picker-spelling precondition is written out twice in `round3-verification.md` (precondition paragraph, item 7) and three times in `evidence.md` (U2 note, integration sub-bullet, check 5 intro). The technical-cause limitation is written out in the round-3 U6 section, round-3 known limitations and evidence U6. Keep one full statement per file and link to it from the other places.

</details>

<details>
<summary>Details</summary>

### Historical notice strings that stay in the docs

A grep of every `Run failed`/`Run interrupted` string in the docs found three groups that differ from the round-3 notices, and none is a current-state claim. The first is the round-1 before/after table at `evidence.md:136-140`, now headed "After (round 1 texts; round 3 replaced them, see below)", with the round-3 texts in the paragraph after it. The second is the "Before (round 1)" column at `round3-verification.md:40-44`. The third is the base-tree evaluation at `evidence.md:93-94`. The round-1 `review.md:72` is a frozen record. `"Run interrupted; Continue starts fresh"` and `"Run failed; check provider or continue from retained progress"` still exist in `threads.rs` as `LEGACY_NOTICES` (`:2280-2281`), so the docs quote them correctly where they describe recognition.

### A pre-rebase command that reads differently now

Inside the section labelled pre-rebase history (`evidence.md:126`), the final-verification row at `:195` cites `git diff 34c9239d..HEAD -- crates/brn-threads-intake crates/brn-intake` as empty. Rerun on the rebased branch, `HEAD` also pulls in main's changes since `34c9239d` (possible confusion, informational). The section label covers it, and `rebase-notes.md` separately records that neither crate changed on main or on this branch.

</details>

<details>
<summary>File map</summary>

- `docs/work/active/daily-workflow-reliability/evidence.md`: rebased header and history pointer; round-3 status rows for U6, U4, U2 and U3; round-3 texts in the Fix 1 follow-up, U1, U6, B1 and B2; U2 precondition; technical-cause limitation; integration risks for round 3; new owner manual check 5.
- `docs/work/active/daily-workflow-reliability/plan.md`: header and item 3 superseded by item 6; item 1 expected-text note; item 6 supersede sentence; round-3 integration risks and remaining limitations.
- `docs/work/active/daily-workflow-reliability/round3-verification.md`: technical-cause pointer; cross-surface precondition; mixed CLI formats in item 6; new item 7; known limitations.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability diff`.

</details>
