# Daily workflow reliability: implementation plan

**Assignment:** owner request of 15 October 2026. Test complete daily workflows, including interruption and recovery, then fix the highest-impact demonstrated problems within service/runtime ownership. **No publish, push, merge or release.** Work only in `/Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability`, using `CARGO_TARGET_DIR=/tmp/brn-dwr-target` and fresh data under `/private/tmp/brn-dwr-data/` (not `/tmp`, which is a symlink). Commit locally after each item.

**Ownership:** edit only `crates/brn-threads-core`, `brn-threads-app`, `brn-threads-intake`, `brn-intake`, `brn-ai`, `brn-retrieval`, `brn` and docs. **Do not edit `crates/brn-desktop`.** Items marked REQUIRES-OWNER-APPROVAL are listed and not applied. Change `agent/` guidance only for a demonstrated runtime need; none is planned. No new dependencies. Cargo.lock is version authority, and std `File::try_lock` (Rust ≥ 1.89) is available under the pinned 1.98.1.

**Evidence:** [evaluation](evidence.md). Selected findings: **1** (orphaned Working run blocks every queue), **2** (useful intake ignores the CLI project and makes a chat per supply), **3** (failure blockers pile up and lose their cause). Implementation follows dependency order: item 1 adds the terminal-notice helper that item 2 reuses.

**Invariants to preserve in every item:**
- One shared checked write service: all writes go through `prepare_owner`/`apply` or `chat_command`.
- SQLite owns state. Lease files are liveness hints under `derived/`, never canonical state.
- Project boundaries: private Source scope and project chats.
- Protected-note authority and exact-candidate review.
- Version checks (`expected_version`) and dirty guards.
- Operation replay: deterministic keys where retried.
- Undo still refuses run-execution control.
- Queue semantics: Stop/failure/restart leave the queue paused and Resume is explicit; no automatic restart of work.
- A saved reply never implies external delivery.

## Decisions

- **D1: liveness lease instead of a startup sweep.** A Working run may be live in another process (desktop plus a CLI agent), so interrupting every Working run on open would fence real work. The run's process holds an OS file lock for the run's lifetime. The OS releases it on any death, including SIGKILL. A Working run whose lock is free is provably orphaned. This needs no schema change and no heartbeat.
- **D2: recover on Resume and Dispatch, not on `Workspace::open`.** These are the paths the orphan blocks. Leaving `open` unchanged avoids changing desktop state-test fixtures, which seed lease-less Working runs next to fake workers. Recovery marks the run Interrupted with a fence bump and adds the standard Interrupted blocker, so the chat shows Continue and appears in Needs you.
- **D3: reuse the intake chat by Source locator and scope, without skipping the run.** Re-supply lands in the chat that holds the earlier outcome, so the run already receives the retained conversation and "Prior operation results". Skipping the model entirely would be a product choice; it is not made here.
- **D4: one terminal notice per chat.** A newer Failed/Interrupted terminal result replaces older host terminal notices for that chat, the same rule success already applies. The failure reason uses the existing owner-facing `AiError` Display, which is fixed text with no provider detail.

## Items

- [x] 1. **One terminal notice per chat, naming the failure cause (finding 3).**
      In `crates/brn-ai/src/threads.rs`:
      - Make `INTERRUPTED_ATTENTION` public. Keep the existing `FAILED_ATTENTION` text recognized as legacy.
      - Add `pub fn is_terminal_notice(reason: &str) -> bool`: the interrupted text, the legacy failed text, or a `"Run failed: "` prefix.
      - Add `pub fn replace_terminal_notice(attention: &mut Vec<Attention>, run: &str, reason: String, superseded: impl Fn(&str) -> bool)`. It drops Blocker items whose reason is a terminal notice and whose `record` is `run` or a superseded earlier run of this chat, then pushes the new Blocker.
      - In `finish()`'s non-Completed branch, compute the superseded runs exactly as the Completed branch does (same-thread Run records in Failed/Interrupted state, other than this run) and call the helper.
      - Failed reason: `format!("Run failed: {error} Continue when the provider is ready.")`, bounded to 512 bytes, using `AiError`'s Display (never `detail`).
      - Update the success branch to match through `is_terminal_notice`.
      - Question, Review and Conflict attention, and unrelated Blockers, are untouched.

      Regression (fail-before/pass-after) in the brn-ai threads test module, beside the existing attention tests around `threads.rs` ~3854/4039:
      - Three `run()` calls with `auth_file: None` and `credentials_dir: None`, which yield `ReconnectNeeded` before any network, leave exactly one Blocker containing "Reconnect this account". A pre-existing Question survives.
      - A later startup-cancelled run replaces it with one Interrupted blocker. Fail-before: three blockers.

      Files: `crates/brn-ai/src/threads.rs` (and its in-file tests or `threads_*_tests.rs`).
      Verify: `CARGO_TARGET_DIR=/tmp/brn-dwr-target cargo test --locked --offline -p brn-ai` passes, including the new test. `cargo clippy --locked --offline -p brn-ai --all-targets -- -D warnings` passes.

- [x] 2. **Run liveness lease plus orphan recovery on Resume/Dispatch (finding 1). Depends on item 1.**
      New `crates/brn-ai/src/run_lease.rs` (exported as `brn_ai::run_lease`):
      - `RunLease::acquire(data_dir, run) -> io::Result<RunLease>` opens or creates `data_dir/derived/run-leases/<name>.lock`. `<name>` is the run ID after checking that it contains only `[A-Za-z0-9:_-]`, is at most 128 bytes, and has `:` mapped to `_`. Refuse symlinked `derived`, `run-leases` or lock paths, as `brn-retrieval/src/search_index.rs` does.
      - It takes `File::try_lock()` and returns an error on `WouldBlock`. Drop removes the file, then unlocks.
      - `pub fn run_is_live(data_dir, run) -> bool`: a missing file is not live. `try_lock` Ok is not live (unlock immediately). `WouldBlock` or any other error is live; when unsure, do not interrupt.
      - Add the module to `lib.rs`.

      In `begin()` (`threads.rs`), acquire the lease for `run_id` immediately after `op.creation_id(0)` and **before** `store.apply` of the start group. Move it into `Scope` (new field `_lease: RunLease`, built at ~line 2045) so it lives until the run future and `PreparedThreadRun` drop.

      In `crates/brn-threads-app/src/chat.rs`, add `pub fn recover_orphaned_runs(&mut self) -> AppResult<Vec<String>>`:
      - Take each non-archived `RunState::Working` Run where `!brn_ai::run_lease::run_is_live(self.store.directory(), id)`.
      - Make one `owner_change` per run, with deterministic key `orphan-recovery:{run}:{version}` so a retry replays.
      - In that change, set the Run to `Interrupted` with `fence + 1` at the expected version, and add the Interrupted blocker to its Thread through `replace_terminal_notice`, at the Thread's expected version, other fields unchanged.
      - A `Stale` outcome, meaning the run finished meanwhile, is skipped without error.

      Call it first in `resume_queue` and `dispatch_queued`. A *live* Working run keeps the existing refusals and messages.

      Regressions in a new `crates/brn-threads-app/tests/run_recovery.rs`. Fold the probe `orphaned_working_run_after_crash_blocks_every_queue` from `tests/daily_workflow_eval.rs` into it, then delete that probe.
      - (a) `prepare_run` then drop, which is a crash: `resume_queue` succeeds. The run is Interrupted with fence 2 and the chat has exactly one Interrupted blocker. The queued item dispatches exactly once with one owner Message. Fail-before: Resume refused.
      - (b) While a `PreparedThreadRun` is held (live lease), Resume and Dispatch still refuse and the run stays Working.
      - (c) Calling `recover_orphaned_runs` twice makes no new receipts.
      - (d) Undo of the recovery operation is refused, as for run control.

      Add a brn-ai unit test: the lease is live during `run_prepared_scripted` and not live after it returns.

      Files: `crates/brn-ai/src/run_lease.rs`, `crates/brn-ai/src/lib.rs`, `crates/brn-ai/src/threads.rs`, `crates/brn-threads-app/src/chat.rs`, `crates/brn-threads-app/tests/run_recovery.rs`, `crates/brn-threads-app/tests/daily_workflow_eval.rs`.
      Verify:
      - `CARGO_TARGET_DIR=/tmp/brn-dwr-target cargo test --locked --offline -p brn-ai -p brn-threads-app -p brn-threads-core` passes.
      - Run the headless desktop state suite, which does not drive the Mac UI: `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop`. It must still pass. **If a desktop test fails because it seeds a lease-less Working run to simulate a live worker, do not edit `crates/brn-desktop`.** Instead, stop and call send_message with severity `warning` naming the test and the minimal fixture change (REQUIRES-OWNER-APPROVAL).

- [x] 3. **Source intake chat reuse plus CLI project scope (finding 2). Independent of items 1–2.**
      In `crates/brn-threads-app/src/intake.rs`, add `pub fn source_intake_chat(&mut self, project: Option<&str>, locator: &str, title: &str) -> AppResult<(Record, bool)>`:
      - Collect the Source IDs whose `locator == locator`, then find Runs whose `sources` include one.
      - Their thread qualifies if it is non-archived, `ThreadState::Open`, and `brn_threads_core::project_for_chat(&records, thread)` matches `project`: the same project, or none for general.
      - Return the most recent qualifying thread by receipt order with `true`.
      - Otherwise create one and return `false`. With a project, use `project_command(&format!("source-chat:{}", uuid), ProjectCommand::CreateChat { project, title })`; without one, use `new_thread(title)`.
      - Refuse an archived or missing project with a clear `Invalid`.

      In `crates/brn/src/main.rs`'s useful-intake branch:
      - Refuse `--thread` together with `--project` before reading the source: "Choose either --project or --thread".
      - With `--project`, validate that the project is active as `chat_option` does, then use `source_intake_chat(Some(project), &converted.source_reference, "Useful information intake")`.
      - Without either option, use `source_intake_chat(None, …)`.
      - Add `"thread"` and `"reused_chat"` fields to the returned JSON. An unchanged prompt plus the runtime's `already_posted` check means a retry does not repost the owner Message.

      Regressions:
      - New `crates/brn-threads-app/tests/source_intake_chat.rs` covers: general reuse; a project chat distinct from a general chat for the same locator; an archived chat not reused; another project's chat not reused; a reused call adding zero records.
      - New CLI test in `crates/brn/tests/threads.rs`, `useful_intake_for_a_project_stays_in_one_project_chat_across_retries`. Configure Codex with a nonexistent auth file so the provider fails offline, following the existing CLI test helpers. Run `intake --intent useful --project P` on a synthetic `.txt` twice. Expect exactly one new thread, listed in P's `chats`; the Source record's `chat` equal to that thread (not `null`); one owner instruction Message; general `search` unchanged; `--thread X --project P` exits non-zero. Fail-before: two general threads and a `chat: null` Source.

      REQUIRES-OWNER-APPROVAL, not applied: in `crates/brn-desktop/src/threads_ui.rs::useful_intake`, replace `self.workspace.new_thread(&format!("Information from {title}"))` with `self.workspace.source_intake_chat(None, &converted.source_reference, &format!("Information from {title}")).map(|(chat, _)| chat)`. Hand off as UI finding U2.

      Files: `crates/brn-threads-app/src/intake.rs`, `crates/brn/src/main.rs`, `crates/brn-threads-app/tests/source_intake_chat.rs`, `crates/brn/tests/threads.rs`.
      Verify: `CARGO_TARGET_DIR=/tmp/brn-dwr-target cargo test --locked --offline -p brn-threads-app -p brn` passes.

- [ ] 4. **Journey regression, docs and full verification. Depends on items 1–3.**
      *Status:* journey regression, evidence and verification done ([evidence](evidence.md#verification)). The `docs/architecture/*` and `docs/status.md` edits are outside this step's doc scope; the proposed text is in the evidence for integration.
      - Turn `tests/daily_workflow_eval.rs` into an assertive journey regression and keep its synthetic world. Assertions: the protected change is `NeedsReview` and the decision text is unchanged; human Action Done is `NeedsReview` and the Action is not Done; the reply Message survives reopen; same-chat re-supply adds no Source record. Remove `EVAL` prints that only record pre-fix behavior.
      - Update `docs/architecture/threads-behavior.md`: in "Stop and recovery", add lease-based orphan recovery on Resume/dispatch and one terminal notice per chat naming its cause; in "Attachments", note that CLI useful intake honours `--project` and repeated supply reuses its intake chat.
      - Add one sentence on orphaned Working runs to `docs/architecture/invariants.md` ("Chat context and saved queue").
      - Add a short status line to `docs/status.md` linking this plan and evidence.
      - Append to `evidence.md`: before/after examples, exact commands and results, and remaining limitations (findings 4–10, U1–U5, the desktop hookup awaiting approval, native observation not performed).
      - Rerun `cli-journey.py` and record the changed observations: `useful_intake_new_threads` 2→1, Source `chat` null→project chat, project chat blockers 3→1.

      Verify:
      - `CARGO_TARGET_DIR=/tmp/brn-dwr-target scripts/verify-storage.sh` (fmt, helper, workspace build, strict Clippy, workspace tests, headless startup).
      - `scripts/verify-end-to-end.sh --fixtures-only`.
      - `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop`.
      - `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings`.
      - `python3 scripts/check-markdown-links.py`.
      - `git diff --check`.
      All must pass; record counts.

## Integration risks (check both branches together later)

- `crates/brn-ai/src/threads.rs` (`begin`, `finish`, `Scope`) and `crates/brn-threads-app/src/chat.rs` are shared seams. The UI branch (`work/ui-polish`) consumes the Resume/Dispatch notices and the attention reasons, and its native tests seed Working runs.
- Run the combined desktop state suite after merging both branches.
- The failure blocker text changes from the fixed "Run failed; check provider…" to "Run failed: {cause} …". The desktop does not match this text (checked), but snapshots or wording reviews might.
- `crates/brn/src/main.rs` (useful intake JSON gains `thread`/`reused_chat`) may conflict with `work/cli-agent-contract`.
- `derived/run-leases/` is a new derived subfolder. Backups and restores ignore it, and `Store::open` already permits `derived/`.

## Remaining limitations after this plan

- Desktop useful intake still creates a chat per supply until the U2 hookup is approved.
- No Stop control for a stale run (U1).
- Findings 4–10 are recorded but unfixed.
- Model-quality judgments (answer support, duplicate avoidance on re-supply, reply wording) need authorized live runs.
- Desktop quit leaving Working runs is unverified natively.
