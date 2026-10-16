# Rebase onto main (`771116f0`, helper sub-agents)

**Date:** 15 October 2026. Local only: nothing pushed, merged or released.

- **Backup ref:** `work/daily-workflow-reliability-pre-rebase` → `914e2e6f` (the branch before rebase, based on `34c9239d`).
- **New base:** `771116f0` ("Merge pull request #145 from ewq100/work/subagent-design"), which is `origin/main` and the UI branch's base.
- **Rebased commits:** `422c9582` → `9f70efa7`, `ee53e62e` → `14ad8577`, `dfe698aa` → `2530805f`, `ab338d05` → `f80554fe`, `914e2e6f` → `b49fb660`. Follow-up: `145b8884` (helper lease test).

## Conflicts

| File | Conflict | Resolution |
|---|---|---|
| `crates/brn-ai/src/threads.rs` | `Scope` fields and the `begin()` constructor: main added `helper`/`scout`; this branch added `_lease` | Both kept. The lease field is now `lease: Arc<RunLease>` so a helper's child scope can share it |
| `crates/brn-ai/src/threads_research.rs` | (no textual conflict) `child_scope` builds a `Scope` and needed the new field | `lease: parent.lease.clone()` |
| `crates/brn/src/main.rs` (twice) | `HELP` text: main added the `--helper` options to `configure`; this branch added the useful-intake reuse line (extended by Task A) | Both kept: main's text plus the two-line Task A reuse note |

No other files conflicted. `crates/brn-desktop` was not edited.

## Helper sub-runs and the lease, notice and intake fixes

Checked by reading `threads_research.rs` at `771116f0`:

- **No Run records.** A helper sub-run is a nested model call inside the parent's `research` tool call. `child_scope` reuses the parent's chat, Run ID, fence, store and search. It never calls `begin()` or `finish()` and writes nothing, so it creates no Working Run that `recover_orphaned_runs` could see.
- **Live parent with live helpers.** The parent `Scope` holds the lease from `begin()` until the Run ends, and each helper scope holds a clone of the same `Arc<RunLease>`. While any helper is in flight the Run stays live, so Resume and Dispatch keep refusing it rather than interrupting it.
- **Crashed parent.** The process exit releases the `flock`, whatever helpers were running. The Run is Working with a free lease and is recovered as before.
- **Terminal notices.** Helper outcomes return as tool results to the parent, and an unavailable helper adds a transient `ThreadResult.notices` entry (CLI JSON `notices`, the desktop finish line). Neither touches chat attention. Only the parent's `finish()` writes the terminal notice, and helper failures do not fail the parent.
- **Intake.** `source_intake_chat` and the CLI useful-intake path are independent of helpers. The `research` tool is registered only when the owner has chosen a helper, so offline tests run with it off.

Regression: `threads::research_tests::a_helper_sub_run_holds_its_parent_lease_and_saves_no_run`. From inside a scripted helper's `search_notes` call, it records that the parent lease is held and that the parent Run is the only Working Run. After the scope drops without finishing (a crash), the lease is free and the Run is still Working, so it is recoverable. It guards the contract rather than reproducing a defect: before the `Arc` change the branch did not compile on the new base, so there is no fail-before.

## Verification after rebase

Run from the worktree with `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/tmp/brn-dwr-target`, rustc 1.98.1, `--locked --offline`. All checks are automated: no Mac UI was driven and no provider calls were made.

| Command | Result |
|---|---|
| `cargo fmt --all --check` | passed |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace --no-fail-fast` | **702 passed, 11 failed, 4 ignored** |
| `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` | **147 passed**, 0 failed (headless state) |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | passed |

Parity with the pre-rebase run (670 passed, 12 failed, 4 ignored):

- **+30 passing tests from main:** 29 `research_tests` in brn-ai and 1 CLI test in `crates/brn/tests/threads.rs` (now 12).
- **+1 passing test from this step:** the helper lease test. brn-ai is now 336 passed, 1 ignored.
- **+1 intake test passing, −1 failing:** `cancellation_after_pdf_copy_removes_raw_input_before_return` failed before the rebase and passes now.
- **Unchanged suites:** `run_recovery` 4, `source_intake_chat` 2 and `daily_workflow_eval` 1 all pass.

All 11 failures are the known `brn-threads-intake` sandbox tests: `process::tests` 3 and `tests/qualification.rs` 8. This agent shell is itself sandboxed, so `sandbox-exec` cannot apply a profile. Neither `brn-threads-intake` nor `brn-intake` changed on main or on this branch. The cancellation test that now passes is in the same sandbox-dependent group; its result varies by run in this shell. There are no other failures.

## Integration notes

- The UI branch (`work/ui-polish`) starts from the same `771116f0`, so the remaining overlap is the shared runtime seams already listed in the [plan](plan.md#integration-risks-check-both-branches-together-later).
- Main adds a helper choice to desktop Settings. Desktop and CLI must still ship from one build, because a lease-less build's live Runs look orphaned.
- `ThreadResult.notices` is a second owner-facing channel, separate from the persisted terminal notice. Helper-unavailable text is never persisted, so it does not survive a restart.

## Second rebase onto `8bef439b`

**Date:** 16 October 2026. Local only: nothing pushed, merged or released.

- **Backup refs:** `work/daily-workflow-reliability-pre-rebase-2` → `b2f70915` (the branch before this rebase, based on `771116f0`). The first backup, `work/daily-workflow-reliability-pre-rebase` → `914e2e6f`, is kept.
- **New base:** `8bef439b` ("Merge pull request #148 from ewq100/work/quality-integration"), which `work/ui-polish` is also on. Since `771116f0`, main adds #146 quality-simplify (one Markdown rendering policy in `threads_ui.rs` and `threads_writing.rs`, desktop tests, a mutation pilot), #147 (a docs-only modular-foundation work pack) and #148 (status and evidence docs). After this step was assigned, `origin/main` moved to `cafb24c2` (#150, docs only: two files under `docs/work/active/modular-foundation/`). This rebase stays on `8bef439b` as assigned.

### Commit map

| Old (`771116f0` base) | New (`8bef439b` base) | Subject |
|---|---|---|
| `9f70efa7` | `595fcd5b` | fix: recover orphaned runs, keep one failure notice, reuse useful-intake chats |
| `14ad8577` | `0dfbcf37` | docs: finalize daily-workflow reliability evidence and review |
| `2530805f` | `6e2b107e` | fix: skip intake chats with a live-lease Working Run when reusing |
| `f80554fe` | `482e1426` | docs: record native observation as not observed and add owner manual check |
| `b49fb660` | `ef0b3264` | docs: finalize Task A/B status matrix and evidence buckets |
| `145b8884` | `bd9f530b` | test: a helper sub-run runs under its parent Run's lease |
| `c408b1d4` | `10286d83` | docs: record the rebase onto main and the helper lease check |
| `3a7c3d25` | `3aaa0f73` | fix: kind-specific failure notices, typed open-edit error, desktop intake reuse |
| `20f73edb` | `31c4db05` | docs: complete round 3 evidence and owner checks |
| `b3fc584a` | `ae40f2e1` | docs: record owner native check results |
| `94cbd62d` | `f1d12149` | docs: record owner on-screen answers and force-kill attempt |
| `0efb6bef` | `86036ab8` | docs: record native idle-intake and force-kill results |
| `3919c09a` | `8470fa74` | fix: a killed Run's notice and candidate stay reachable (round 4 WIP) |
| `d83ba18d` | `a86d2d17` | docs: record round 4 verification and native re-check stores |
| `0b08b2e8` | `0e335371` | docs: finalize round 4 (G1 notice, G2 review item, G3 lease cleanup for killed Runs) |
| `f519534e` | `9f5fcdfa` | docs: record native round 4 observation and intake sandbox tests |
| `e07354db` | `558250bb` | fix: clean up abandoned backup and restore stages (round 5 slice A) |
| `02bd457b` | `344331f2` | fix: count one file as one source regardless of path spelling (round 5 slice B) |
| `28c4a757` | `0f7016e8` | fix: address slice B review (round 5 slice B, review round) |
| `abd9a84f` | `51de25d6` | fix: end lease-free Runs like orphans on every stop path (round 5 slice C) |
| `b037140c` | `08e6d57d` | fix: settings stops record Interrupted, matching the notice (round 5 slice D) |
| `dd2c8ce8` | `9537f242` | fix: reuse a chat's intake instruction instead of repeating it (round 5 slice E) |
| `f5872774` | `f3deba1a` | fix: find saved chat replies in owner search (round 5 slice F) |
| `7ed74f26` | `c1c57ca7` | fix: let the agent withdraw its own obsolete proposal (round 5 slice G) |
| `28e64cdf` | `821c9516` | fix: name an unusable data folder in owner terms (round 5 slice H) |
| `f6ece107` | `3a9bd63b` | docs: close round 5 (evidence, verification counts, native re-check) |
| `87d1b664` | `0effd55d` | docs: record unsandboxed round 5 storage verification |
| `b2f70915` | `00ae3dc7` | docs: record exit-code ownership and UI branch decisions |

### Conflict resolution

There were no textual conflicts. `crates/brn-desktop/src/threads_ui.rs` is the only file both sides changed, and their hunks sit in separate regions, so git merged them on its own and both sides are kept:

- **Main:** `external_link`, `stored_image` and `safe_markdown` (the shared Markdown policy that `markdown()` now calls), plus the `policy_tests` module at the end of the file.
- **This branch:** the `threads_intake_tests` module declaration, and `useful_intake` calling `source_intake_chat`, `SOURCE_INTAKE_INSTRUCTION` and `post_intake_instruction`.

`git range-diff 771116f0..b2f70915 8bef439b..HEAD` reports all 28 patches as identical. No docs conflicted, and this branch still does not edit `docs/status.md`.

### Verification after the second rebase

Run from the worktree with `HOME=/Users/evokessler`, `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/tmp/brn-dwr-target`, rustc 1.98.1, `--locked --offline`, and a fresh `TMPDIR` under `/private/tmp/brn-dwr-data/` for each test run. All data was synthetic, no Mac UI was driven and no provider calls were made.

| Command | Result |
|---|---|
| `cargo fmt --all --check` | passed |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace --no-fail-fast` | **808 passed, 1 failed, 6 ignored**, the same as before the rebase. The failure is `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` (`brn-threads-intake/tests/qualification.rs`), which needs Poppler; Poppler is not installed. Main's new tests are all in the desktop lane below, so this count does not change |
| `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` | **161 passed, 1 ignored**: the 158 from before the rebase plus main's `policy_tests::external_links_allow_only_absolute_lowercase_http_s`, `policy_tests::images_resolve_only_from_the_caller_s_own_stored_assets` and `document_tests::read_view_shows_block_html_literally_and_renders_all_link_text` |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | passed |
| `scripts/verify-end-to-end.sh --fixtures-only` | passed (provider retirement, intake/engine retirement, offline Save/recovery/search/export/backup/startup/refusal journeys) |
| `cli-journey.py` with a fresh root | passed: 43 steps, 40 exit 0; the three exit 1 are the expected refusals (a second open edit session, twice, and an occupied backup destination). Restore matched the original, with 25 operations and 24 receipts |
| `brn-desktop --headless-check startup --data-dir <fresh>/data` | "Threads ready: 2 records", run on the `native-ui` binary rebuilt from this HEAD |
| `git diff --check` (worktree and `8bef439b..HEAD`) | clean |
| `python3 scripts/check-markdown-links.py` | 0 failures |
| `cargo build -p brn-desktop --features native-ui` | passed; `/tmp/brn-dwr-target/debug/brn-desktop` is ready for a native check |
