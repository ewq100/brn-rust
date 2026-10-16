# Settings stops record Interrupted, matching their notice (round 5 slice D)

Reviewed commit `b037140c` on `work/daily-workflow-reliability` against slice C (`abd9a84f`).

`update_workspace_settings` now ends every Working Run as `Interrupted` instead of `Cancelled`. This is the owner's U9 decision ("go with interrupted"): after Configure, a delegation choice, maintenance Pause/Resume or a backup-folder change, the run line said "Cancelled" while the chat's notice said "Run interrupted. Continue starts fresh." The fix is one line in the store, inside the same checked change that already bumps the fence, adds the notice and Review items, and lets slice C remove the free lease file. Eight settings-stop expectations were flipped and one core test was added, each with a recorded fail-before. No dependency, schema, CLI, `main.rs`, `HELP`, exit-code, desktop production or `docs/status.md` change.

Watch for: after this change no test reaches the `Cancelled` arm of `finish()`'s notice supersession, which is the only path that clears a notice saved on an old Cancelled Run (confirmed). Continue's model context also carries the Run state and is missing from the consumer list (confirmed, benign).

**Verdict**: APPROVED

## High-level view

`update_workspace_settings` builds one `writes` list holding the Settings patch, each stopped Run (now `Interrupted`, fence +1) and the Thread attention from `ended_run_attention(.., notice = true, ..)`, then prepares and applies it in one transaction. A settings retry returns its recorded receipt, so replay adds no second notice. Candidates are only named in Review items, never applied or retired, and T1 still ends with `assert_reviewable`.

Every consumer that branches on Run state treats `Interrupted` and `Cancelled` alike: the queue `Finish` check, notice supersession in `ended_run_attention` and `finish()`, desktop `can_continue` and acknowledgement, and the lease-removal receipt scan. Display consumers print `Debug` of the state, so the run line and rewrite status now read "Interrupted" with no desktop edit. One consumer is missing from the table: Continue passes `"state": "Interrupted"` to the model for a stopped Run with progress. Nothing branches on that value.

The spec's "a deliberate owner Stop keeps Cancelled" does not match the code, and the slice's fact check is right. Desktop Stop calls `stop_chat_run`, which is `ChatCommand::Pause { run }`; it has always written `Interrupted`, with no notice for a live Run. After this slice the only production `Cancelled` writer is the legacy `Workspace::resolve_thread`, which has no production caller. Run state alone no longer separates a deliberate Stop from an outside stop; only the notice does. That was already true on every path except settings.

Old saved `Cancelled` Runs keep their state, and the code that handles them is untouched. But the test that exercised `finish()` retiring a notice on a Cancelled Run now starts from an Interrupted Run, so that arm has lost its only coverage. Round 4 builds wrote exactly that shape: a Cancelled Run carrying the Interrupted notice.

The recorded evidence matches the files. Every cited fail-before line lands on the flipped assertion, and D-T1's move from :530 to :532 is explained by `cargo fmt`. Core `delegation` 7 → 8 and workspace 765 → 766 match the one new test; desktop stays at 154. The `r5-d-settings` store exists under `/private/tmp/brn-dwr-data/`. The desktop run-line assertion recomputes `format!("{:?}", state)` from `latest_run` rather than reading the drawn text, which is a little weaker than the commit message says.

<details>
<summary>Issues (4)</summary>

1. **`finish()` Cancelled supersession untested** — no test now seeds a Cancelled Run with the Interrupted notice and completes a later Run in its chat; add that leg (for example, rewrite the killed Run to `Cancelled` before Continue in `killed_run_candidate.rs`, or a brn-ai unit test) and expect `[Review(op)]`.
2. **Continue model context missing from the consumer table** — `brn-ai/src/threads.rs:1989` sends `format!("{:?}", run.state)` to the model; add the row to slice D's consumer table in `round5/verification.md`.
3. **"Cancels" wording for settings stops** — core README (147, 166) and app README (17) still say settings changes cancel runs; add them to slice D's integration items next to the proposed `threads-behavior.md` sentence, and optionally rename `a_later_run_replaces_the_notice_of_a_run_the_settings_change_cancelled`.
4. **Desktop run-line claim** — the assertion checks the state `latest_run` feeds the run line, not the drawn text; reword the claim as "the run line's latest Run is Interrupted", or assert the rendered text if the harness allows.

</details>

<details>
<summary>Details</summary>

### Consumers of the stopped state

```
update_workspace_settings ──► Run{Interrupted, fence+1} + Thread{Review…, Blocker(run, INTERRUPTED)}   (one checked change)
        │
        ├─ core queue Finish(success=false)     accepts Failed | Interrupted | Cancelled      chat.rs:409
        ├─ ended_run_attention supersession     Interrupted | Failed | Cancelled              run_attention.rs:132
        ├─ brn-ai finish() supersession         Interrupted | Failed | Cancelled              threads.rs:2474
        ├─ desktop can_continue                 Working | Interrupted | Failed | Cancelled    threads_state.rs:104
        ├─ desktop acknowledge_run              state != Working (non-queue), fence key        threads_chat.rs:332
        ├─ app lease removal / ended-by scan    before Working, after != Working              app chat.rs:68
        ├─ run line, rewrite status             format!("{:?}", state)                        threads_writing.rs:301, threads_ui.rs:2804, threads_workbench.rs:191
        ├─ Continue model context               "state": format!("{:?}", run.state)           threads.rs:1989   ← not in the table
        └─ CLI ask                              prints AiTerminal, not RunState               main.rs:361
```

No runtime guide text in `threads_guides.rs` mentions Cancelled or Interrupted, so the model-context change only relabels one field for settings-stopped Runs that had progress (confirmed).

The wording elsewhere still says "cancel" for settings stops: the core README (147, 166), the app README (17) and the test name `a_later_run_replaces_the_notice_of_a_run_the_settings_change_cancelled`. The READMEs now describe a state the code no longer writes (confirmed, non-blocking). The slice's proposed contract sentence covers `threads-behavior.md` only.

### Coverage left for old saved Cancelled Runs

`brn-ai/src/threads.rs:2474` puts a chat's earlier Interrupted, Failed or Cancelled Runs into `superseded`, so a Completed result retires their notices. Before this slice, `a_later_run_replaces_the_notice_of_a_run_the_settings_change_cancelled` reached the `Cancelled` arm: the settings change left a Cancelled Run with the notice, Continue completed, and the notice was expected gone. That Run is now Interrupted.

A search of `crates/brn-ai`, `crates/brn-threads-app` (src and tests), `crates/brn` and the desktop tests finds no other test that seeds a Cancelled Run with a notice and then runs `finish()`. The core seeds (`core.rs:1031`, `:1501`, `:1537`, `steering.rs:216`) never reach brn-ai, and `thread_lifecycle.rs:71` uses the legacy resolve, which clears attention. The core unit test `a_notice_on_a_cancelled_run_is_replaced` covers only `ended_run_attention`. So dropping `| RunState::Cancelled` from `finish()` would leave the suite green, and a chat saved by a round 4 build would keep "Run interrupted" beside a successful answer (confirmed by reading; not mutation-tested).

The `Cancelled` arms of the queue `Finish` check and of `can_continue` are in the same position, with less at stake. Neither had a settings-path test before this slice, so this diff did not remove their coverage.

### D-T1 and the desktop run-line check

D-T1's state assertion is the regression check. Its `Finish { success: false }` leg passed before the fix too, because `Cancelled` was also accepted, and the doc does not claim otherwise.

The desktop addition calls `latest_run(&ui.records, &ui.history, &chat)` and compares `format!("{:?}", state)` with `"Interrupted"`. That matches what `threads_writing.rs:301` draws today, and `continue-{run}` ties the drawn Continue button to the same Run. It does not read the rendered `■ …` text, so a change to how the run line formats a state would leave the test green (confirmed, minor).

The limitation about replaying a settings operation prepared by an older build is nearly theoretical: prepare and apply share one transaction, so a prepared but unapplied settings operation cannot normally persist.

</details>

<details>
<summary>File map</summary>

- `crates/brn-threads-core/src/store.rs`: settings stop writes `RunState::Interrupted`; comment.
- `crates/brn-threads-core/tests/delegation.rs`: two flipped expectations; new `a_settings_stop_records_interrupted_matching_its_notice` with `chat`/`chat_control` helpers.
- `crates/brn-threads-core/tests/core.rs`: flipped expectation, variable renamed.
- `crates/brn-threads-core/tests/backup_directory.rs`: flipped expectation.
- `crates/brn-threads-app/tests/product.rs`: flipped expectation.
- `crates/brn-threads-app/tests/killed_run_candidate.rs`: T1 and T1-live flipped; comments.
- `crates/brn-desktop/src/threads_intake_tests.rs` (test-only, `cfg(all(test, feature = "native-test-support"))`): flipped state, latest-run state check, `continue-{run}` drawn.
- `docs/work/active/daily-workflow-reliability/round5/verification.md`: slice D section.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability show b037140c`.

</details>
