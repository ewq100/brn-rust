# Independent review: launcher focus-return witness

Reviewed checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, branch `codex/launcher-focus-witness`.
Frozen candidate: `8912801c24f5f3a3ec4fb55bb8e5d74b118dba51`.
Verified parent/base: `2658f8883b47f224b02ed428055efaaba1bb47a4`.
Sole changed file: `crates/brn-desktop/src/threads_dialogs_tests.rs` (13 additions, 1 deletion), SHA-256 `3aef40eeddff4df6b26275f7ec79931435478ce8640519a163db300346778381`.
The checkout was clean on entry and during final inspection.

## Findings

No actionable findings.

The candidate extends the existing Escape journey at `threads_dialogs_tests.rs:651-663`. It verifies closed launcher state, the expected Chat layout destination, actual composer focus ownership and absence of focus on the closed launcher query. It does not call the production focus-selection function to calculate the expected answer or manipulate focus to satisfy the assertions. The expected composer comes from the concrete open, ordinary Chat in this scenario and the existing visible-content focus contract, already exercised separately by `focus_lands_inside_each_destination_panel` (`:245-284`) and documented at the host focus boundary (`threads_shell.rs:1547-1553`).

The complete existing journey was inspected with its fixture/helpers and later menu/dialog/Settings sequence. It creates a fresh synthetic thread, opens Chat, installs a synthetic Working run, deliberately focuses the root, then sends real test-window Cmd+K and Escape events before observing focus. Launcher opening focuses its query (`threads_ui.rs:2512-2520`); launcher Escape interception invokes `close_launcher` (`threads_shell.rs:1530-1540`), whose current return-focus adapter targets visible content (`threads_ui.rs:2523-2527`). The new checks observe the resulting GPUI handles rather than only `Layout` state. The `on_window` helper does not itself call the focus adapter or focus the composer; it only runs the supplied closure and parks the test executor.

Existing `still_working` assertions remain unchanged and still execute after the new focus checks: the worker remains present, cancellation stays false, the canonical Run remains Working, and the queue remains unpaused. Later Escape ordering assertions remain unchanged. The edit introduces no production behavior, new test, fixture, dependency, schema or runtime-guide change, and it does not alter provider/queue/run authority.

The fixture uses a fresh canonical temporary parent and persists `offline-fixture` with both credential locations cleared before UI creation (`threads_dialogs_tests.rs:28-31`; `threads_focus_tests.rs:17-24`). Polling is disabled in the fixture. `working_run` creates its Run through the existing owner service, installs an in-memory Worker/receiver/cancellation token, and holds a local run lease; it does not launch a provider. The new read-only focus assertions add no provider path.

## Evidence and limits

Read AGENTS, the requested layout/focus audit and implementation report, applicable verification guidance, relevant architecture/behavior scope, the complete actual one-file diff, full affected journey, fixture/window helpers, adjacent actual-focus witnesses, launcher key/action callbacks, visible-content focus adapter, offline fixture and synthetic worker construction. Existing independent-review/Rust guidance was already read for this review session.

`git diff --check HEAD^ HEAD` passed. Source inspection confirms `brn-desktop` uses Rust edition 2024. The author's direct standalone rustfmt diagnostic did not supply that project edition and reported unrelated formatting differences; no unrelated reformat is part of this candidate. This reviewer did not rerun formatting or infer a project-format failure from that invocation. Lead-owned Cargo-format diagnosis/check remains separate.

At observation, `/tmp/brn-launcher-focus-qualification/normal.log` names exact candidate `8912801c`, records the matching test-file hash and qualified helper hashes, and is still compiling; it does not yet establish a passing or failing focused test. No build or test was launched by this reviewer. No temporary mutation was performed or observed, so no negative-control/mutation proof is claimed. The lead may assess that separately after this source review completes.

This is a synthetic GPUI test-window witness for launcher Escape return to an ordinary, visible Chat at the existing 1400×1000 fixture size. It does not independently qualify archived Chat landing, every layout/close/maximize/reorder transition, OS focus/IME, physical keyboard behavior or native usability. The prior audit found a witness gap, not an observed production defect; this review does not invent a regression fix or original failing production behavior. Applicable completed check evidence, integration, native observation and owner acceptance remain distinct and lead-owned.

No source edits, builds, native/account/web actions or nested work were performed. Only this requested review artifact was written. Review applies to the exact frozen candidate above; material candidate changes require affected review evidence to be refreshed.
