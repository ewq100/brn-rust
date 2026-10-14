# Daily workflow reliability: evaluation evidence

**Date:** 15 October 2026. **Base:** `34c9239d` (#144). **Fix commit:** `422c9582` on local branch `work/daily-workflow-reliability`, isolated worktree, approved in [review](review.md). **Scope:** service/runtime/CLI. The evaluation sections below describe the base tree; the fixes, their before/after results and the final verification follow under [Verification](#verification). `crates/brn-desktop` was read but not edited; the UI agent's worktree was not touched.

## Status

| Item | Implemented | Verified (automated) | Native-observed | Owner-accepted | Merged |
|---|---|---|---|---|---|
| Fix 1: one terminal notice per chat, naming the cause | yes | yes | no | no | no |
| Fix 2: lease-based orphan recovery on Resume/Dispatch | yes | yes (in-process; no two-process check) | no | no | no |
| Fix 3: CLI useful intake honours `--project` and reuses its chat | yes | yes | n/a (CLI) | no | no |
| Contract doc updates (`threads-behavior.md`, `invariants.md`, `status.md`) | no (proposed text below) | no | n/a | no | no |
| Desktop hookup U2 | no (UI-owned) | no | no | no | no |

Nothing was pushed, merged or released. The branch has no upstream.

**Environment:** macOS arm64, repository Rust 1.98.1 and lockfile, `CARGO_TARGET_DIR=/tmp/brn-dwr-target`, fresh synthetic data under `/private/tmp/brn-dwr-data/` (`/tmp` is a symlink and the core correctly refuses it). No provider calls, credentials, owner data or native UI use. The configured Codex auth file does not exist, so every CLI provider step fails as `ReconnectNeeded` before any network request.

## Evidence classes

| Class | What it covers here |
|---|---|
| Automated, executed | [`cli-journey.py`](cli-journey.py) drives the real `brn` CLI through the journey and failure cases and reads SQLite read-only. `crates/brn-threads-app/tests/daily_workflow_eval.rs` drives the real runtime tools with a scripted Rig model (`run_prepared_scripted`), the real search worker and persisted state; it prints `EVAL` lines. There is also a SIGKILL backup/restore loop over an 85 MB synthetic store. Commands and results are listed at the end. |
| Code inspection | Desktop quit/close handling, Resume/Continue paths and the useful-intake call site. These are cited by file. |
| Native observation | **None.** Recommended items are listed at the end. They wait for the owner's go-ahead and must not run alongside the UI agent. |
| Not judged | Model quality: whether a real model finds both sources, phrases honest uncertainty, avoids duplicate notes on re-supply or drafts a good reply. The scripted model shows only what the host offers, enforces and persists. |

## Workflow evaluation (steps 1–7)

1. **Project with instructions.** `project-create --key` replays to the same project (one record). Instructions and knowledge-on persist. The first run prompt in the project chat contains the instructions. *Useful:* yes.
2. **Document and email.**
   - Importing the plan Markdown into the project twice makes one protected private note; the second import records "Observed previously imported full note". General search finds 0 hits and project search finds 1.
   - Every verbatim Markdown/text full import is labelled **Partial**, with "completeness is unverified" (finding 6).
   - An `.eml` email needs a live GPT-6 Luna conversion (`conversion.rs`: every kind other than Markdown/text). With deterministic fixtures, the email was supplied as `.txt` or as a transient source. Offline email intake is impossible. That follows the owner-chosen Luna contract.
   - CLI `intake --intent useful --project P` **ignores the project** (finding 2).
3. **Question needing both sources and a note.** In the project chat, `search_notes` returned the project plan, the general protected decision and the general survey note (knowledge on). The email is a transient source, readable only in its own run. Later runs see it only through notes written from it. *Scope correct; answer quality not judged.*
4. **Contradiction with a protected decision.**
   - A scripted attempt to change the protected decision to 15 November came back `NeedsReview`, and the decision text stayed "22 November".
   - In a project chat, the ordinary survey update plus Suggested Action also came back `NeedsReview` (private-context runs cannot write general notes autonomously).
   - Both candidates surface as two identical **"Review prepared changes"** reasons (finding 5).
5. **Resolve and prepare a reply.**
   - The owner applied the survey candidate. After "Keep 22 November and draft a reply", the scripted model marked the human Action Done ("Reply drafted in chat"). The host returned `NeedsReview`, and the Action stayed `Suggested`: no invented completion.
   - The reply was saved as an assistant Message that says "(not sent)".
   - The obsolete "change decision to 15 November" candidate stays pending, with no agent tool to withdraw it (finding 7).
6. **Reopen and continue.** A new `Workspace::open` lists the chat. All four messages are in durable order. Search for the reply text finds notes only; chat replies are not indexed (finding 8). Continue in the same chat works (the next run's input includes retained conversation and prior operation receipts).
7. **Same source again.**
   - In the same chat: no new Source record (its version bumps 1→2→3), a new owner Message and Run, and the prompt carries the fingerprint and the applied survey note.
   - There is no explicit "already processed by run X" fact; duplicate avoidance is left to the model.
   - Through CLI or desktop useful intake, **every supply creates a new chat**: two CLI supplies gave two "Useful information intake" threads, and desktop `useful_intake` creates "Information from …" each time (finding 2).

## Failure cases

| Case | Must survive | May stay incomplete | Never twice | Recovery for the user | Observed persisted state |
|---|---|---|---|---|---|
| Typing interrupted / restart | Latest composer text per destination; dirty note buffer | Nothing else | Draft generations never regress | Reopen shows draft / recovery buffer | `recovery` lists session (base 1, generation 1); covered by existing core/desktop tests |
| Save fails (stale base) | Typed buffer | Note unchanged | No overwrite of newer text | Compare or discard | `save` → `Stale`, session open, buffer intact; retry → `Stale` again. The open stale session then blocks any new edit with raw `SQLite: UNIQUE constraint failed: edit_sessions.note` (finding 4) |
| Another writer while a buffer is dirty | Buffer and current note | Other writer's change | No silent merge | Save/discard, then retry | `undo` while session open → `Deferred`; save then succeeds |
| Proposal preparation interrupted | Prepared candidate | Apply | No second candidate per note | Apply/dismiss; next run sees "Prior operation results" | Candidates persist with Review attention (existing `a_stale_candidate_is_refreshed…` test) |
| Proposal application interrupted | Receipt | — | No double apply | Replay returns same receipt | `save` replay → `Saved` once; existing lost-response replay tests |
| Stop mid-run | Partial text, queue | Answer | No completion after Stop | Resume | Existing chat/steering tests cover fence/partial |
| Queue across failure/restart | Pending Message, waiting items | Run | No repost of the owner Message | Resume | Repeated failed `ask` with the same prompt: one owner Message, 3 Failed runs |
| **Crash/quit/kill during non-queued work** (useful intake, Continue, rewrite, CLI `ask`) | Run record, queue | That run | No double dispatch | **None obvious** (finding 1) | Run stays `Working`; dispatch → "work is still running"; Resume → "Finish or stop current work"; chat attention `[]`; Needs you 0 |
| Provider failure (auth) | Owner Message, Source observation | Answer | — | Fix provider, Continue/Resume | `Failed` Run without the cause; each failure adds another identical blocker: 3 failures → 3 blockers (finding 3); CLI exits 0 with `terminal: Failed:ReconnectNeeded` |
| Same source supplied again | One Source per locator/scope | — | Unneeded notes/Actions/chats | — | New chat per CLI/desktop supply; Source version bump only (finding 2) |
| Backup interrupted (SIGKILL ×7) | Workspace | Backup | No partial final file | Retry same destination | Final never published; 2 leftover `.brn-backup-stage-*` (0700, partial snapshot) |
| Restore interrupted (SIGKILL ×7) | Backup, target absent | Restore | No partial workspace | Retry | Target never created; 5 leftover `.brn-restore-stage-*`, each a full hidden copy of BRN data, never cleaned (finding 9) |
| Restore into existing | Existing data | — | — | Choose fresh folder | Refused |

## Ranked findings (severity × frequency × user cost)

1. **High: an orphaned `Working` run blocks every queue with no signal.**
   - *What happens:* a run started outside the queue persists `Working` before execution. If the process dies first, nothing ever ends it. Process death includes a crash, SIGKILL or an agent timeout; the CLI handles only SIGINT, so SIGTERM/SIGHUP count too.
   - *Desktop quit (assumption, unverified natively):* `on_window_should_close` and Quit cancel the token, then quit at once (`threads_ui.rs` ~3585–3608), which likely skips the terminal commit.
   - *Effect:* `Dispatch` and `Resume` both refuse while any `Working` run exists. The orphan chat has no attention, and Needs you is empty.
   - *Only recovery:* find that chat and press Continue (a fresh provider run), or Archive it. The service has `stop_chat_run`, but the UI offers Stop only for a local worker.
   - *Repro:* `daily_workflow_eval::orphaned_working_run_after_crash_blocks_every_queue`.
   - *Severity:* unreachable queued work and misleading errors.
2. **Medium-high: useful intake ignores project scope in the CLI and makes a new chat per supply.**
   - `brn intake --intent useful --project P` runs in a new **general** thread and records the email Source with `chat: null`. With a live provider, project email facts would flow into general knowledge, against the project boundary the owner chose.
   - Retrying after a failure, or re-supplying, creates another chat each time. Two supplies gave two threads and two runs. Desktop `useful_intake` has the same one-chat-per-supply behavior.
   - *Repro:* `cli-journey.py`, observations `useful_intake_new_threads: 2` and `sources_after_useful_project_intake`.
3. **Medium: failure blockers pile up and the cause is lost after restart.**
   - `finish()` (`threads.rs` ~2195) keeps every earlier failure blocker until a later success: 3 failed asks gave 3 identical "Run failed; check provider…" reasons.
   - The `Run` record stores no failure kind, so after restart the owner cannot tell "Reconnect this account" from a rate limit or network failure. The cause appears only in the live session or CLI JSON.
4. **Medium-low: a concurrent edit session fails with a raw SQLite error.** `begin_edit` on a note with an open (even stale or untyped) session returns `SQLite: UNIQUE constraint failed: edit_sessions.note`. The message does not name the open session, its base or the recovery action.
5. **Medium-low: candidate attention reasons are generic.** Every candidate is "Review prepared changes". An attempted protected-decision change and a routine survey update look identical in Needs you.
6. **Medium-low (needs owner decision): verbatim imports are always Partial.** The full-note policy without an independent inventory (`brn-threads-intake/src/lib.rs` ~633–645) marks byte-identical Markdown/text as Partial/"unverified". It is a deliberate qualification rule, so changing it is a contract choice.
7. **Low-medium (model-dependent): the agent cannot withdraw its own obsolete candidate.** After the owner resolves a conflict, a candidate contradicting the resolution stays in Needs you until it is dismissed manually.
8. **Low: saved replies are found only through the chat list.** Search covers notes only.
9. **Low: interrupted backup/restore leaves hidden stage copies.** They are never cleaned, and the restore stages are full data copies.
10. **Low (CLI contract; coordinate with `work/cli-agent-contract`): CLI exit codes hide failed runs.** `ask` and useful `intake` exit 0 on Failed/Interrupted runs.

Selected for implementation: **1, 2, 3** (see [plan](plan.md)). Findings 4–10 remain recorded limitations.

## UI findings for the UI agent

The current, reproducible list is in [UI findings for the UI agent (final)](#ui-findings-for-the-ui-agent-final). None of them was observed natively.

## Native observation recommended (not performed)

Run only after the owner's go-ahead, on this branch's build and fresh data, never alongside the UI agent:

1. Quit with ⌘Q during a useful intake. Check whether the Run is left `Working` (this confirms the finding 1 assumption), then check Resume after the fix.
2. Fail the provider three times. Check the single blocker that names its cause.

## Evaluation commands (base tree)

- `cargo build --locked --offline -p brn -p brn-intake --features brn-intake/helper --bins`: passed.
- `python3 docs/work/active/daily-workflow-reliability/cli-journey.py /tmp/brn-dwr-target/debug/brn /private/tmp/brn-dwr-data/cli-journey`: exit 0, observations in `/tmp/brn-dwr-logs/cli-journey.json`.
- `cargo test --locked --offline -p brn-threads-app --test daily_workflow_eval -- --nocapture --test-threads=1`: 2 passed, `EVAL` output in `/tmp/brn-dwr-logs/eval.log`. `cargo clippy … --test daily_workflow_eval -- -D warnings` and `cargo fmt --all -- --check` also pass.
- SIGKILL backup/restore loop (inline Python, `/private/tmp/brn-dwr-data/killed-backup`): source store kept 6 notes, no final path was published early, and the leftover stages listed above remained.

## Verification

**Implementation iteration 1**, branch `work/daily-workflow-reliability` on `34c9239d`. Plan items 1–3 are implemented and item 4's journey regression is in place. Files changed: `crates/brn-ai/src/{run_lease.rs,lib.rs,threads.rs}`, `crates/brn-threads-app/src/{chat.rs,intake.rs}`, `crates/brn/src/main.rs`, `crates/brn/tests/threads.rs`, and the tests `crates/brn-threads-app/tests/{run_recovery,source_intake_chat,daily_workflow_eval}.rs`. `crates/brn-desktop`, `agent/` and `docs/status.md` are untouched. No dependencies or schema changes were added.

Evidence classes: everything below is **automated, executed** on macOS arm64 with Rust 1.98.1, `CARGO_TARGET_DIR=/tmp/brn-dwr-target` and fresh synthetic data. **Native observation: none.** No provider calls were made: every provider step fails offline as `ReconnectNeeded`.

### Before and after

**Fix 1: one terminal notice per chat, naming the cause (finding 3).** `finish()` now removes the host terminal notices of earlier Interrupted/Failed Runs of the same chat before adding the new one (`replace_terminal_notice`). A failure names its owner-facing `AiError` text, which never includes provider detail. The earlier fixed text is still recognised in saved chats.

| | Before | After |
|---|---|---|
| Three failed asks in one chat (CLI journey, "Crane delivery") | 3 × `Blocker "Run failed; check provider or continue from retained progress"` | 1 × `Blocker "Run failed: Reconnect this account before continuing. Continue when the provider is ready."` on the newest Run |
| A later Run stopped at startup | Failure notices kept, plus an Interrupted one | Only `"Run interrupted; Continue starts fresh"` remains; an owner Question in the chat survives |
| Restart | Cause lost | Cause stays in the persisted attention reason |

**Fix 2: lease-based orphan recovery on Resume/Dispatch (finding 1).** `begin()` takes an OS file lock on `derived/run-leases/<run>.lock` before the Run is saved as Working, and holds it in the Run's `Scope` until the Run ends. `Workspace::resume_queue` and `dispatch_queued` first call `recover_orphaned_runs`, which interrupts each Working Run whose lease is free. One checked owner change sets the Run to Interrupted, bumps its fence and adds the standard Interrupted notice. Its key is `orphan-recovery:{run}:{run version}:{chat version}`, so a retry replays. A live Run keeps today's refusals.

| | Before | After |
|---|---|---|
| Crash after a useful intake started, then Resume (`run_recovery::resume_after_a_crash…`) | `Finish or stop current work before resuming the queue`; Dispatch: `work is still running`; orphan chat attention `[]`; Needs you empty | Resume succeeds; Run `Interrupted`, fence 2; the orphan chat has exactly one Interrupted Blocker and shows in Needs you; the queued question dispatches once with one owner Message |
| Run still live in this or another process (`a_live_run_keeps_blocking…`) | Refused | Still refused; Run stays Working with no attention; recovered once its holder lets go |
| Undo of the recovery | (n/a) | Refused: "run execution control cannot be compensated" |

After the CLI journey `derived/run-leases/` held 0 files: every lease ended with its Run.

**Fix 3: useful intake honours the CLI project and reuses its chat (finding 2).** `Workspace::source_intake_chat(project, locator, title)` returns the most recent open chat in the same scope whose Run already received that Source locator, or creates one (a project chat when `project` is set). CLI `intake --intent useful` uses it, refuses `--project` with `--thread` before reading the source, and reports `thread` and `reused_chat` (null when `--thread` names the chat).

| CLI journey (`cli-journey.py`, two supplies of `mari-email.txt` with `--project P`) | Before | After |
|---|---|---|
| `useful_intake_new_threads` | 2 general "Useful information intake" threads | 1, listed in P's `chats` |
| Email Source `chat` | `null` (general) | the project intake chat |
| Owner instruction Messages for the two supplies | 2 (one per thread) | 1 (retry reuses it) |
| `needs_you` | 3 chats | 2 chats |
| Records / receipts | 25 receipts, 3 Threads, 4 Messages | 24 receipts, 2 Threads, 3 Messages |

All other journey observations are unchanged (project create replay, document import twice, scoped search, edit-session cases, backup/restore).

### Implementation-iteration commands and results

| Command | Result |
|---|---|
| `cargo fmt --all` then `scripts/verify-storage.sh` (`CARGO_TARGET_DIR=/tmp/brn-dwr-target`) | fmt check, helper build, workspace build and strict Clippy (`--workspace --all-targets -D warnings`) passed. `cargo test --workspace` stopped at `brn-threads-intake` (see below). |
| `cargo test --workspace --locked --offline --no-fail-fast` | **669 passed, 12 failed, 4 ignored.** brn-ai 306 passed (1 ignored), including the new notice and lease tests; brn-threads-app 148, including `run_recovery` 4, `source_intake_chat` 1 and `daily_workflow_eval` 1; brn CLI `tests/threads.rs` 11; brn-threads-core 112; brn-retrieval 33; brn-intake 15; brn-desktop default 20. All 12 failures are `brn-threads-intake` sandbox tests (`process::tests` 3, `tests/qualification.rs` 9) that return `Unavailable`/`Invalid` because this agent shell is itself sandboxed: `sandbox-exec -p '(version 1)(allow default)' /usr/bin/true` → `sandbox_apply: Operation not permitted`. That crate and its only dependency (`brn-intake`) are unchanged. |
| Headless startup tail of `verify-storage.sh`: `brn --help`, `brn-desktop --help`, `brn-desktop --headless-check startup --data-dir /private/tmp/brn-dwr-data/startup/data` | passed ("Threads ready: 2 records") |
| `TMPDIR=/private/tmp/brn-dwr-data/e2e scripts/verify-end-to-end.sh --fixtures-only` | passed (provider and intake retirement checks; offline Save/recovery/search/export/backup/startup/refusal journeys) |
| `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` (headless state, no Mac UI driven) | **147 passed**; no desktop fixture needed a change |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | passed |
| `python3 cli-journey.py …/debug/brn /private/tmp/brn-dwr-data/cli-journey` (after rebuild) | exit 0; before/after table above |
| `git diff --check`; `python3 scripts/check-markdown-links.py` | passed |

**Fail-before check.** Each fix was disabled in place and its regression rerun, then the source was restored:

- Fix 1 off (no superseding): `repeated_failures_keep_one_notice…` failed with three Blockers.
- Fix 2 off (no recovery call): `resume_after_a_crash…` and `a_live_run_keeps_blocking…` failed with `Finish or stop current work before resuming the queue`.
- Fix 3 off (`new_thread` per supply): `useful_intake_for_a_project_stays_in_one_project_chat_across_retries` failed with `reused_chat` `(false, false)`.

### Final verification (tree `422c9582`, rerun after review)

Run from the worktree root with `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/tmp/brn-dwr-target`, rustc 1.98.1, fresh data under `/private/tmp/brn-dwr-data/final-*`. All automated; no Mac UI driven, no provider calls.

| Command | Result |
|---|---|
| `cargo fmt --all --check` | passed |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline` | passed |
| `cargo build --workspace --locked --offline` | passed |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | passed |
| `cargo test --workspace --locked --offline --no-fail-fast` | **669 passed, 12 failed, 4 ignored**, identical to the reviewed run. Changed-area suites all pass: brn-ai 306 (1 ignored), CLI `tests/threads.rs` 11, `run_recovery` 4, `source_intake_chat` 1, `daily_workflow_eval` 1. The 12 failures are the `brn-threads-intake` sandbox tests (`process::tests` 3, `tests/qualification.rs` 9); `sandbox-exec -p '(version 1)(allow default)' /usr/bin/true` → `sandbox_apply: Operation not permitted` (exit 71) in this shell, and `git diff 34c9239d..HEAD -- crates/brn-threads-intake crates/brn-intake` is empty. |
| `brn --help`; `brn-desktop --help`; `brn-desktop --headless-check startup --data-dir /private/tmp/brn-dwr-data/final-startup/data` | passed ("Threads ready: 2 records") |
| `TMPDIR=/private/tmp/brn-dwr-data/final-e2e scripts/verify-end-to-end.sh --fixtures-only` | passed (provider retirement, intake retirement, offline Save/recovery/search/export/backup/startup/refusal journeys) |
| `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` | **147 passed**, 0 failed (headless state) |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | passed |
| `python3 docs/work/active/daily-workflow-reliability/cli-journey.py /tmp/brn-dwr-target/debug/brn /private/tmp/brn-dwr-data/final-cli-journey` | exit 0; `useful_intake_new_threads` 1, email Source recorded in the project intake chat (not `null`), one intake instruction Message across both supplies, one "Crane delivery" question Message across 3 failed asks, `needs_you` 2, 24 receipts, Thread 2 / Message 3, `derived/run-leases/` empty afterwards |
| `git diff --check`; `python3 scripts/check-markdown-links.py` | passed |

`scripts/verify-storage.sh` as a whole stops at the same 12 sandbox tests in this shell; its individual steps are listed above. Rerun it unsandboxed on the combined branch.

### Not done in this step

- **Shared docs.** The plan's updates to `docs/architecture/threads-behavior.md`, `docs/architecture/invariants.md` and the `docs/status.md` line are outside this step's doc scope (`docs/status.md` is shared with the UI branch). They are left for integration. Proposed contract text:
  - *Stop and recovery:* "A Run holds a liveness lease (`derived/run-leases/`) while it executes. Resume and queue dispatch interrupt any Working Run whose lease is free, with the standard Interrupted notice, so a crash or kill never blocks the queue. A newer interrupted or failed result replaces the chat's earlier host terminal notice; a failure names its cause."
  - *Attachments:* "CLI useful intake honours `--project`; supplying the same source again reuses the open chat in that scope that already received it."
  - *Invariants, chat context and saved queue:* "An orphaned Working Run (no live lease) is interrupted on Resume or dispatch; a live one keeps blocking."
- **Desktop hookup U2.** Using `source_intake_chat` in `crates/brn-desktop/src/threads_ui.rs::useful_intake` still needs owner approval or UI-agent coordination.

### Remaining limitations

- Recovery runs only on Resume and Dispatch (D2). After a crash with an empty queue, the orphan chat shows nothing until the owner continues it or uses the queue. Continue in that chat already interrupts it.
- A killed process leaves its empty lease file in `derived/run-leases/`. It is unlocked and harmless, but never cleaned.
- Liveness relies on `flock`, verified in-process across separate file handles (the same mechanism that separates processes). A two-process check was not run, because the CLI exposes no queue Resume/Dispatch command.
- A BRN build without leases (for example a desktop built from another branch) running beside this branch would have its Working Runs treated as orphans on Resume/Dispatch.
- If the chat is created but the run start is refused (for example a settings mismatch), the next CLI supply creates another chat, because reuse keys on a Run that received the Source.
- Findings 4–10 and U3–U5 are unchanged.
- **Model quality not judged.** The deterministic fixtures (scripted Rig model, offline `ReconnectNeeded` provider) prove only what the host offers, enforces and persists. They cannot judge whether a real model combines both sources with the existing note, states uncertainty honestly, asks the right owner question, avoids duplicate notes on re-supply or drafts a good reply. That needs an authorized live run on synthetic data.
- **Email conversion not exercised.** `.eml` needs live Luna conversion, so the email was supplied as `.txt`/transient source.
- **Native observation: none.** The ⌘Q-during-run assumption (finding 1) is from code inspection only.
- Open non-blocking review issues ([review](review.md)), recorded rather than fixed:
  - *Reuse can fence a live run.* `source_intake_chat` does not skip a chat whose Working Run is live in another process; a concurrent same-source supply then interrupts that Run through `begin()`. Suggested fix: skip candidates with a live lease (`run_lease::run_is_live`).
  - *Provider-centric suffix.* "Continue when the provider is ready." is appended for every failure kind, including tool/time limits and storage errors, and persists after restart.
  - *Single-snapshot recovery.* Two simultaneous orphans in one chat need two Resumes, and the stale owner-direct preparation stays listed in `candidates()`.
  - *Destructive journey script.* `cli-journey.py` deletes its `ROOT` argument recursively before running; pass only a fresh path under `/private/tmp/brn-dwr-data/`.

## UI findings for the UI agent (final)

Setup for every item: a fresh data folder (for example `/private/tmp/brn-ui-check/data`), this branch's or the combined branch's `brn-desktop` started with `--data-dir` on it, synthetic files only, and no other agent driving the Mac UI. None of these was observed natively; "Observed" means service/CLI evidence or code reading, as stated.

- **U1: an orphan from a stuck live process has no Stop.**
  1. Open a chat, click ＋ (`composer-add`) → "Add useful information…", choose a synthetic `.md`.
  2. While "Reading useful information…" shows, force-quit BRN (⌘⌥Esc → Force Quit).
  3. Reopen BRN on the same data, type a message in another chat, press Return, then click "Resume" (`queue-resume`).
  - *Expected:* the queued message dispatches, and the earlier chat shows "Run interrupted; Continue starts fresh" in Needs you.
  - *Observed:* before the fix, Resume showed "Finish or stop current work before resuming the queue" and Needs you was empty (service tests). After the fix the service clears it (`run_recovery` tests). Remaining UI gap: when the Run is held by a live but stuck process, there is still no Stop control; consider one for a Run with no local worker.
- **U2: one chat per desktop source supply.**
  1. In a chat, click ＋ → "Add useful information…" and pick `mari-email.txt`.
  2. Repeat step 1 with the same file.
  - *Expected:* the second supply goes into the existing "Information from mari-email" chat.
  - *Observed (code):* `threads_ui.rs::useful_intake` calls `new_thread("Information from {title}")` every time, so there are two chats. Fix: call `Workspace::source_intake_chat(project, &converted.source_reference, &format!("Information from {title}"))`, passing the current project (or `None` for general). This is a desktop edit, so it belongs to the UI agent.
- **U3: identical review reasons.**
  1. Create a protected note "Delivery decision: 22 November" and an ordinary note "Survey".
  2. In a project chat, ask the agent to change the decision to 15 November and update Survey (a run in Review-first mode, or a project chat, so both come back as candidates).
  3. Open the Needs you list.
  - *Expected:* the protected-decision change is distinguishable (reason or target note title).
  - *Observed (service):* two identical "Review prepared changes" reasons.
- **U4: raw error on a second edit session.**
  1. Edit a note, type text, then change the same note through another writer (for example `brn` CLI with the same `--data-dir`) so Save returns Stale.
  2. Leave the stale buffer open and start editing the same note again.
  - *Expected:* a message naming the open buffer and offering Compare/Discard.
  - *Observed (CLI):* `SQLite: UNIQUE constraint failed: edit_sessions.note`. Whether the desktop shows this raw text was not observed.
- **U5: "Partial" badge on verbatim imports.** Click ＋ → "Import full note…" and pick a plain `.md`. *Observed (service):* outcome Partial, "completeness is unverified". The rule is deliberate; the owner should decide whether verbatim Markdown/text counts as Complete before any UI change.
- **U6: longer failure notice.** Leave the provider unconnected and send a message three times in one chat. *Expected after this branch:* one Blocker "Run failed: Reconnect this account before continuing. Continue when the provider is ready." The desktop does not match on the old fixed text, but rows are longer; check truncation and decide final wording together (see the provider-centric suffix limitation).

## Integration risks with `work/ui-polish`

- **Shared runtime seams.** `begin`, `finish` and `Scope` in `crates/brn-ai/src/threads.rs`; `resume_queue`, `dispatch_queued` and the new `recover_orphaned_runs` in `crates/brn-threads-app/src/chat.rs`; `crates/brn-threads-app/src/intake.rs`. If the UI branch touches these, resolve by hand and rerun `run_recovery`, `source_intake_chat` and the brn-ai notice/lease tests.
- **Desktop behaviour that depends on this branch.**
  - `threads_chat.rs::resume_chat_queue` already calls `stop_chat_run` on a queue-claimed Working Run without a liveness check, then `resume_queue`. That pre-existing desktop path is unchanged here and can still stop a Run that is live in another process; `resume_queue` now additionally interrupts lease-free orphans.
  - Desktop tests that seed a Working Run to stand for a live worker and then call Resume/Dispatch will see it interrupted, because the seeded Run has no lease. None exists at `422c9582` (147 headless tests pass); new ones on `work/ui-polish` might.
  - Terminal notice text changed (`failed_attention` now names the cause; `INTERRUPTED_ATTENTION`, `is_terminal_notice`, `replace_terminal_notice` are public). Any UI test asserting "Run failed; check provider…" will fail.
- **Service API additions (no removals).** `Workspace::source_intake_chat(project, locator, title)`; `brn_ai::run_lease::{RunLease, run_is_live}`. CLI useful-intake JSON gains `thread` and `reused_chat`; `--project` with `--thread` is now refused; help gains one line (possible conflict with `work/cli-agent-contract`).
- **Storage.** New derived folder `derived/run-leases/` (excluded from backup/restore like all of `derived/`; no schema change). `begin()` refuses to start a Run when `derived/` is unsafe (for example a symlink).
- **Mixed builds.** A desktop and CLI from different builds (one without leases) would see each other's live Runs as orphans. Check the combined branch from one build: the full desktop state suite, an unsandboxed `scripts/verify-storage.sh`, and a desktop-plus-CLI run on shared synthetic data.
- **Docs.** Contract text for `threads-behavior.md`, `invariants.md` and `docs/status.md` is proposed above and not applied, because `docs/status.md` is shared with the UI branch. Apply once at integration.
- **Native observation** (⌘Q during a run, three provider failures) is still pending and must not run while the UI agent drives the Mac.
