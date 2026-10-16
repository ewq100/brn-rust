# Daily workflow reliability: evaluation evidence

**Date:** 15 October 2026. **Base:** `8bef439b` (main, #148 quality integration), after a [second rebase](rebase-notes.md#second-rebase-onto-8bef439b). The branch is now on `8bef439b`; the commit hashes in this record were taken before that rebase, on `771116f0` (#145 helper sub-agents), and the [second rebase table](rebase-notes.md#commit-map) maps each one to its new hash. **Fix commits:** `9f70efa7` (fixes 1–3, approved in [review](review.md)), `2530805f` (Task A), `145b8884` (helper sub-run lease test) and `3a7c3d25` (round 3) on local branch `work/daily-workflow-reliability`, isolated worktree. **History:** the evaluation and the fixes before round 3 ran on the earlier base `34c9239d` (#144), where fixes 1–3 were `422c9582` and Task A was `dfe698aa`. Hashes in sections labelled pre-rebase refer to that history, which the backup ref `work/daily-workflow-reliability-pre-rebase` (`914e2e6f`) keeps; the [rebase notes](rebase-notes.md) map each commit. **Scope:** service/runtime/CLI. The evaluation sections below describe the base tree; the fixes, their before/after results and the final verification follow under [Verification](#verification). Through Task A, `crates/brn-desktop` was read but not edited. Round 3 edited it at the UI agent's request (U2): `useful_intake` in `crates/brn-desktop/src/threads_ui.rs` (plus its test-module declaration) and a new `crates/brn-desktop/src/threads_intake_tests.rs`. The UI agent's worktree was not touched.

**Round 5** (`e07354db`…`28e64cdf`, eight slices A–H) is summarized in [Round 5](#round-5-eight-reliability-slices-ah); per-slice detail is in [round 5 verification](round5/verification.md). Round 5 hashes are the pre-second-rebase ones; [rebase notes](rebase-notes.md#commit-map) map them.

**Round 6** (`7ecc43aa`, `b5212618`, `747755b3`) implements the owner's decisions 1–3 of 16 October 2026: see [Round 6](#round-6-owner-decisions-13).

**Round 3** (`3a7c3d25`, after the rebase onto `771116f0`; [round 3 verification](round3-verification.md)) applied U6, U4 and U2 as agreed with the UI agent, including the `useful_intake` call in `crates/brn-desktop`. Where a notice text below differs from the [round 3 table](round3-verification.md#u6-failure-notice-wording-cratesbrn-aisrcthreadsrs), the round 3 text is what this build shows. The owner manual check and the UI findings use the round 3 texts.

## Status

| Item | Implemented | Verified (automated) | Native-observed | Owner-accepted | Merged |
|---|---|---|---|---|---|
| Fix 1: one terminal notice per chat, naming the cause | yes (`9f70efa7`) | yes | no | no | no |
| Fix 2: lease-based orphan recovery on Resume/Dispatch | yes (`9f70efa7`) | yes (in-process; no two-process check) | no | no | no |
| Fix 3: CLI useful intake honours `--project` and reuses its chat | yes (`9f70efa7`) | yes | n/a (CLI) | no | no |
| Task A (review issue 1): intake-chat reuse skips a chat whose Run is live | yes (`2530805f`) | yes (in-process lease; [review](task-a-review.md) approved) | n/a (service/CLI) | no | no |
| Rebase onto `771116f0`; a helper sub-run holds its parent's lease | yes (`145b8884`; [rebase notes](rebase-notes.md)) | yes (702 passed after the rebase) | n/a (service) | no | no |
| Task B1: ⌘Q during a run, then Resume | n/a | service only (`run_recovery`) | **owner, store only** ([results](#owner-native-check-results-15-october-2026)): ⌘Q recorded `Interrupted`, so no orphan arose and Resume recovery was not exercised natively | no | n/a |
| Task B2: three provider failures, one Blocker | n/a | service/CLI only (Fix 1) | **owner, store only** ([results](#owner-native-check-results-15-october-2026)): one Blocker, three `Failed` Runs; on-screen appearance not reported | no | n/a |
| Contract doc updates (`threads-behavior.md`, `invariants.md`, `status.md`) | no (proposed text below) | no | n/a | no | no |
| Round 3: U6 kind-specific notices, for example "Run failed: The provider account needs reconnecting. Continue when the provider is ready." and "Run interrupted. Continue starts fresh." ([table](round3-verification.md#u6-failure-notice-wording-cratesbrn-aisrcthreadsrs)) | yes (`3a7c3d25`) | yes (workspace 706 passed, 12 failed (known sandbox), 4 ignored) | no (owner checks 3–4) | no | no |
| Round 3: U4 typed open-edit error `brn_threads_core::Error::EditSessionOpen { note, session, base_version }` (via `AppError::Core`); CLI `edit_session_open` JSON | yes (`3a7c3d25`) | yes | no (the desktop shows the generic notice until the UI branch matches the variant) | no | no |
| Round 3: desktop hookup U2 (at the UI agent's request): `useful_intake` in `threads_ui.rs`, new `threads_intake_tests.rs` | yes (`3a7c3d25`) | yes (desktop 150 passed, 1 ignored, headless harness; cross-surface only with the CLI's exact locator string) | **owner, store only** ([results](#owner-native-check-results-15-october-2026)): picker re-supply of a CLI-supplied `/private/tmp` file reused the CLI's chat | no | no |
| Round 3: U3 read-only check ([finding](u3-finding.md)): no service fields needed; the desktop's `ThreadRow` drops `Attention.record` | n/a (no code) | code reading | n/a | no | n/a |
| Round 4 ([verification](round4/verification.md)): a Run that ends outside `finish()` (settings, project changes and imports, orphan recovery) leaves one Interrupted notice in the same checked change; Stop, Archive, Steer and Continue add none | yes (`3919c09a`, local) | yes (workspace 721 passed, 12 failed (known sandbox), 5 ignored; with the sandbox off, 11 of the 12 pass, see [native round 4](#native-round-4-observation-orchestrator-15-october-2026)) | **yes, settings and Resume paths** ([native round 4](#native-round-4-observation-orchestrator-15-october-2026)) | no | no |
| Round 4: every Run end, including ⌘Q, cancellation and failure in `finish()`, adds a "Review prepared changes" item for each candidate it left unapplied; candidates are never applied or retired | yes (`3919c09a`, local) | yes (desktop 152 passed, 1 ignored; headless Home Needs you, attention cards and proposal card) | **yes** (proposal card with Apply/Dismiss reachable from Needs you; [native round 4](#native-round-4-observation-orchestrator-15-october-2026)) | no | no |
| Round 4: orphan recovery removes the free leftover lease file | yes (`3919c09a`, local) | yes | **yes** (Resume path; the settings path keeps it, as recorded) | no | no |
| Round 5 A: abandoned backup/restore stages are swept | yes (`e07354db`) | yes ([round 5](#round-5-eight-reliability-slices-ah)) | no | no | no |
| Round 5 B: one file is one Source however its path is spelled | yes (`02bd457b`, `28c4a757`) | yes | no | no | no |
| Round 5 C: every stop path treats a lease-free Run as an orphan | yes (`abd9a84f`) | yes (CLI dry run of the native procedure) | no ([procedure](round5/native-recheck.md)) | no | no |
| Round 5 D: settings stops record `Interrupted` (owner decision U9) | yes (`b037140c`) | yes (CLI dry run) | no ([procedure](round5/native-recheck.md)) | no | no |
| Round 5 E: one shared intake instruction, reused on re-supply | yes (`dd2c8ce8`) | yes (CLI + headless desktop on one store) | no | no | no |
| Round 5 F: owner search lists saved chat Messages | yes (`f5872774`) | yes (service/CLI; desktop does not render them, F-UI1) | no ([procedure](round5/native-recheck.md)) | no | no |
| Round 5 G: runtime tool `withdraw_proposal` | yes (`7ed74f26`) | yes (scripted model) | n/a (runtime) | no | no |
| Round 5 H: an unusable data folder is named in owner terms | yes (`28e64cdf`) | yes (CLI and headless desktop startup) | no | no | no |
| Round 6, decision 1: a chat's notice clears when its next Run starts | yes (`7ecc43aa`) | yes ([round 6](#round-6-owner-decisions-13)) | **yes** (Continue after a crash; [native round 6](#native-round-6-observation-orchestrator-16-october-2026)) | decision yes; build no | no |
| Round 6, decision 2: Archive adds Review items but no new notice | yes (`7ecc43aa`) | yes (service and headless desktop) | **yes** ([native round 6](#native-round-6-observation-orchestrator-16-october-2026)) | decision yes; build no | no |
| Round 6, decision 2 applied to the owner's Stop (interpretation pending) | yes (`b5212618`, own commit) | yes | no (no native path on this branch) | no | no |
| Round 6, decision 3: archived chats stay in owner search, labelled `archived` | yes (`747755b3`) | yes (service, CLI and headless desktop data; the desktop does not render it, F-UI1) | **service via CLI only** (the desktop shows no hit, F-UI1; [native round 6](#native-round-6-observation-orchestrator-16-october-2026)) | decision yes; build no | no |
| Round 5 D re-checked: a settings stop reads "Interrupted" with the notice | yes (`b037140c`) | yes | **yes** ([native round 6](#native-round-6-observation-orchestrator-16-october-2026), part B) | decision yes; build no | no |

Nothing was pushed, merged or released. The branch has no upstream.

**Environment:** macOS arm64, repository Rust 1.98.1 and lockfile, `CARGO_TARGET_DIR=/tmp/brn-dwr-target`, fresh synthetic data under `/private/tmp/brn-dwr-data/` (`/tmp` is a symlink and the core correctly refuses it). No provider calls, credentials or owner data. Native UI use only in the native sections named under Evidence classes. The configured Codex auth file does not exist, so every CLI provider step fails as `ReconnectNeeded` before any network request.

## Evidence classes

| Class | What it covers here |
|---|---|
| Automated, executed | [`cli-journey.py`](cli-journey.py) drives the real `brn` CLI through the journey and failure cases and reads SQLite read-only. `crates/brn-threads-app/tests/daily_workflow_eval.rs` drives the real runtime tools with a scripted Rig model (`run_prepared_scripted`), the real search worker and persisted state; it prints `EVAL` lines. There is also a SIGKILL backup/restore loop over an 85 MB synthetic store. Commands and results are listed at the end. |
| Code inspection | Desktop quit/close handling, Resume/Continue paths and the useful-intake call site. These are cited by file. |
| Native observation | None before round 4: the sandboxed shell could not drive the UI ([attempt](#native-observation-attempt-task-b-not-observed)). Then the owner's [store checks](#owner-native-check-results-15-october-2026) and, with the sandbox off, the orchestrator's [round 4](#native-round-4-observation-orchestrator-15-october-2026) and [round 6](#native-round-6-observation-orchestrator-16-october-2026) checks. |
| Unverified assumptions | Desktop ⌘Q skips the terminal commit (finding 1, code reading); a two-process lease check; desktop rendering of U3, U4 and U6. |
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

The agent shell could not run these ([attempt](#native-observation-attempt-task-b-not-observed)). The owner ran all three on 15 October 2026; see [results](#owner-native-check-results-15-october-2026). Steps are in [Owner manual check](#owner-manual-check).

1. Quit with ⌘Q during a useful intake. Check whether the Run is left `Working` (this confirms the finding 1 assumption), then check Resume after the fix.
2. Fail the provider three times. Check the single blocker that names its cause.
3. Round 3: supply a file through the CLI, then pick the same file in the desktop. Check that the desktop reuses the CLI's chat.

## Evaluation commands (base tree)

- `cargo build --locked --offline -p brn -p brn-intake --features brn-intake/helper --bins`: passed.
- `python3 docs/work/active/daily-workflow-reliability/cli-journey.py /tmp/brn-dwr-target/debug/brn /private/tmp/brn-dwr-data/cli-journey`: exit 0, observations in `/tmp/brn-dwr-logs/cli-journey.json`.
- `cargo test --locked --offline -p brn-threads-app --test daily_workflow_eval -- --nocapture --test-threads=1`: 2 passed, `EVAL` output in `/tmp/brn-dwr-logs/eval.log`. `cargo clippy … --test daily_workflow_eval -- -D warnings` and `cargo fmt --all -- --check` also pass.
- SIGKILL backup/restore loop (inline Python, `/private/tmp/brn-dwr-data/killed-backup`): source store kept 6 notes, no final path was published early, and the leftover stages listed above remained.

## Verification

*Pre-rebase history:* this section, the final verification and Task A below ran on `34c9239d`; the backup ref `work/daily-workflow-reliability-pre-rebase` keeps that tree. Results after the rebase are in the [rebase notes](rebase-notes.md#verification-after-rebase), and the current results are in [round 3 verification](round3-verification.md#repository-checks-all-run-once-on-the-final-tree).

**Implementation iteration 1**, branch `work/daily-workflow-reliability` on `34c9239d`. Plan items 1–3 are implemented and item 4's journey regression is in place. Files changed: `crates/brn-ai/src/{run_lease.rs,lib.rs,threads.rs}`, `crates/brn-threads-app/src/{chat.rs,intake.rs}`, `crates/brn/src/main.rs`, `crates/brn/tests/threads.rs`, and the tests `crates/brn-threads-app/tests/{run_recovery,source_intake_chat,daily_workflow_eval}.rs`. `crates/brn-desktop`, `agent/` and `docs/status.md` are untouched. No dependencies or schema changes were added.

Evidence classes: everything below is **automated, executed** on macOS arm64 with Rust 1.98.1, `CARGO_TARGET_DIR=/tmp/brn-dwr-target` and fresh synthetic data. **Native observation: none.** No provider calls were made: every provider step fails offline as `ReconnectNeeded`.

### Before and after

**Fix 1: one terminal notice per chat, naming the cause (finding 3).** `finish()` now removes the host terminal notices of earlier Interrupted/Failed Runs of the same chat before adding the new one (`replace_terminal_notice`). A failure names its owner-facing `AiError` text, which never includes provider detail. The earlier fixed text is still recognised in saved chats.

| | Before | After (round 1 texts; round 3 replaced them, see below) |
|---|---|---|
| Three failed asks in one chat (CLI journey, "Crane delivery") | 3 × `Blocker "Run failed; check provider or continue from retained progress"` | 1 × `Blocker "Run failed: Reconnect this account before continuing. Continue when the provider is ready."` on the newest Run |
| A later Run stopped at startup | Failure notices kept, plus an Interrupted one | Only `"Run interrupted; Continue starts fresh"` remains; an owner Question in the chat survives |
| Restart | Cause lost | Cause stays in the persisted attention reason |

Round 3 replaced these texts: the no-credentials failure is now "Run failed: The provider account needs reconnecting. Continue when the provider is ready." and the interrupted notice is "Run interrupted. Continue starts fresh." (`INTERRUPTED_ATTENTION`). After a restart only that owner-terms cause remains. Technical detail such as a retry-after delay, an HTTP status or a provider code is not persisted ([round 3 limitation](round3-verification.md#known-limitations)).

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

### Final verification (pre-rebase tree `422c9582`, now `9f70efa7`; rerun after review)

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

### Task A: intake-chat reuse skips a chat whose Run is live (review issue 1)

`Workspace::source_intake_chat` now skips a candidate chat that has a Working Run whose lease is held (`brn_ai::run_lease::run_is_live`), because `begin()` interrupts every Working Run in the chat it starts in. It falls back to the next eligible chat, or creates a new one. A lease-free (orphaned) Working Run does not block reuse; the next Run in that chat interrupts it with a fence bump, as Continue already does. If `derived/` cannot be checked, liveness counts as unknown and the chat is skipped. The skip set matches what `begin()` would fence: every Run record in the chat in state Working. No schema, dependency or desktop change.

**CLI reporting.** `intake --intent useful` returns `reused_chat: false` and the new chat's ID in `thread` when the earlier chat is skipped, which is the same shape as a first supply. The help line now says so. `--thread` still names the chat explicitly and reports `reused_chat: null`.

| Concurrent same-source supply while a Run in the earlier chat holds its lease | Before | After |
|---|---|---|
| Chosen chat | the earlier chat (`reused_chat: true`) | a new chat (`reused_chat: false`) |
| The live Run (state, fence) | `(Interrupted, 2)`: fenced by `begin()`, its in-flight turn lost | `(Working, 1)`: unchanged |
| After the lease is released (newer chat archived) | n/a | the earlier chat is reused again (`true`), and the next Run there interrupts the orphan: `(Interrupted, 2)` |

Regression: `crates/brn-threads-app/tests/source_intake_chat.rs::a_chat_whose_run_is_still_live_is_not_reused_until_its_lease_is_free`. A `PreparedThreadRun` holds the live lease through its own file handle, as in `run_recovery`.
- **Fail-before** (skip disabled in place as `(true || !busy(chat))`): FAILED at `source_intake_chat.rs:160`, "the live Run is not interrupted", `left: (Interrupted, 2)`, `right: (Working, 1)`. The other test in the file passed.
- **Pass-after** (fix restored): 2 passed, 0 failed.

Commands, run from the worktree with `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/tmp/brn-dwr-target` and rustc 1.98.1 (automated only; no Mac UI driven, no provider calls):

| Command | Result |
|---|---|
| `cargo fmt --all --check` | passed |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | passed |
| `cargo test --workspace --locked --offline --no-fail-fast` | **670 passed, 12 failed, 4 ignored**: baseline 669 plus the new test. The 12 failures are the same `brn-threads-intake` sandbox tests (`process::tests` 3, `tests/qualification.rs` 9); `sandbox-exec -p '(version 1)(allow default)' /usr/bin/true` → `sandbox_apply: Operation not permitted` (exit 71) in this shell. `crates/brn-threads-intake` and `crates/brn-intake` are unchanged. No new failures. |
| `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` | **147 passed**, 0 failed (headless state) |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | passed |
| `python3 docs/work/active/daily-workflow-reliability/cli-journey.py /tmp/brn-dwr-target/debug/brn /private/tmp/brn-dwr-data/cli-journey-fresh` (after rebuilding `brn`) | exit 0; unchanged from the final verification: `useful_intake_new_threads` 1, email Source in the project intake chat, `needs_you` 2, 24 receipts, `derived/run-leases/` empty afterwards. The journey's supplies run one after another, so reuse still applies. |

**Provider-centric suffix (review issue 2): not changed.** The Blocker reason is persisted and shown verbatim by the desktop (`threads_tiles.rs` Needs-you rows and chat entries, `threads_writing.rs` attention, `threads_lists.rs` reason filter), so changing it is desktop-visible wording. It is handed to the UI agent as U6.

### Not done in this step

- **Shared docs.** The plan's updates to `docs/architecture/threads-behavior.md`, `docs/architecture/invariants.md` and the `docs/status.md` line are outside this step's doc scope (`docs/status.md` is shared with the UI branch). They are left for integration. Proposed contract text:
  - *Stop and recovery:* "A Run holds a liveness lease (`derived/run-leases/`) while it executes. Resume and queue dispatch interrupt any Working Run whose lease is free, with the standard Interrupted notice, so a crash or kill never blocks the queue. A newer interrupted or failed result replaces the chat's earlier host terminal notice; a failure names its cause."
  - *Attachments:* "CLI useful intake honours `--project`; supplying the same source again reuses the open chat in that scope that already received it."
  - *Invariants, chat context and saved queue:* "An orphaned Working Run (no live lease) is interrupted on Resume or dispatch; a live one keeps blocking."
- **Desktop hookup U2.** Using `source_intake_chat` in `crates/brn-desktop/src/threads_ui.rs::useful_intake` still needs owner approval or UI-agent coordination. *Superseded:* round 3 applied it at the UI agent's request.

### Remaining limitations

- Recovery runs only on Resume and Dispatch (D2). After a crash with an empty queue, the orphan chat shows nothing until the owner continues it or uses the queue. Continue in that chat already interrupts it.
- A killed process leaves its empty lease file in `derived/run-leases/`. It is unlocked and harmless, but never cleaned.
- Liveness relies on `flock`, verified in-process across separate file handles (the same mechanism that separates processes). A two-process check was not run, because the CLI exposes no queue Resume/Dispatch command.
- A BRN build without leases (for example a desktop built from another branch) running beside this branch would have its Working Runs treated as orphans on Resume/Dispatch.
- If the chat is created but the run start is refused (for example a settings mismatch), the next CLI supply creates another chat, because reuse keys on a Run that received the Source.
- Two concurrent supplies of the same source still make two chats (Task A): the second skips the chat whose Run is live rather than interrupt it. A Run that is live but stuck in another process keeps its chat out of reuse until that process ends (see U1).
  - *Selection-to-begin window (Task A review).* Two same-source supplies that both pick the chat before either Run begins still share it, and the later `begin()` fences the earlier Run: the liveness check and the fence are separate transactions. Closing it needs a liveness check inside `begin()`'s write.
  - *Empty chat when `derived/` is unusable (Task A review).* Unknown liveness skips the chat, a new chat is created, and `begin()` then refuses on the same `derived/` check, leaving one empty chat per attempt.
- Findings 4–10 and U3–U5 are unchanged. *Round 3:* finding 4 (U4) now gives a typed `EditSessionOpen` error; findings 5–10, U3 and U5 are unchanged.
- **Model quality not judged.** The deterministic fixtures (scripted Rig model, offline `ReconnectNeeded` provider) prove only what the host offers, enforces and persists. They cannot judge whether a real model combines both sources with the existing note, states uncertainty honestly, asks the right owner question, avoids duplicate notes on re-supply or drafts a good reply. That needs an authorized live run on synthetic data.
- **Email conversion not exercised.** `.eml` needs live Luna conversion, so the email was supplied as `.txt`/transient source.
- **Native observation: none.** B1 and B2 were approved but not observed (shell denied process list, screen capture and UI control; B1 also needs a live provider). The round 3 picker re-supply (owner check 5) is also unobserved. The ⌘Q-during-run assumption (finding 1) is from code inspection only.
- **Technical failure cause not persisted (round 3).** Only the owner-terms cause in the saved notice survives a restart; see U6 below and the [round 3 limitation](round3-verification.md#known-limitations).
- Open non-blocking review issues ([review](review.md)), recorded rather than fixed:
  - *Provider-centric suffix.* "Continue when the provider is ready." is appended for every failure kind, including tool/time limits and storage errors, and persists after restart. *Resolved in round 3:* the sentence now appears only for provider failures.
  - *Single-snapshot recovery.* Two simultaneous orphans in one chat need two Resumes, and the stale owner-direct preparation stays listed in `candidates()`.
  - *Destructive journey script.* `cli-journey.py` deletes its `ROOT` argument recursively before running; pass only a fresh path under `/private/tmp/brn-dwr-data/`.

## UI findings for the UI agent (final)

Setup for every item: a fresh data folder (for example `/private/tmp/brn-ui-check/data`), this branch's or the combined branch's `brn-desktop` started with `--data-dir` on it, synthetic files only, and no other agent driving the Mac UI. None of these was observed natively; "Observed" means service/CLI evidence or code reading, as stated.

- **U1: an orphan from a stuck live process has no Stop.**
  1. Open a chat, click ＋ (`composer-add`) → "Add useful information…", choose a synthetic `.md`.
  2. While "Reading useful information…" shows, force-quit BRN (⌘⌥Esc → Force Quit).
  3. Reopen BRN on the same data, type a message in another chat, press Return, then click "Resume" (`queue-resume`).
  - *Expected:* the queued message dispatches, and the earlier chat shows "Run interrupted. Continue starts fresh." in Needs you.
  - *Observed:* before the fix, Resume showed "Finish or stop current work before resuming the queue" and Needs you was empty (service tests). After the fix the service clears it (`run_recovery` tests). Remaining UI gap: when the Run is held by a live but stuck process, there is still no Stop control; consider one for a Run with no local worker.
- **U2: one chat per desktop source supply.**
  1. In a chat, click ＋ → "Add useful information…" and pick `mari-email.txt`.
  2. Repeat step 1 with the same file.
  - *Expected:* the second supply goes into the existing "Information from mari-email" chat.
  - *Observed (code, before round 3):* `threads_ui.rs::useful_intake` called `new_thread("Information from {title}")` every time, so there were two chats. Proposed fix at the time: call `Workspace::source_intake_chat(project, &converted.source_reference, &format!("Information from {title}"))`, passing the current project (or `None` for general). This is a desktop edit, so it belongs to the UI agent.
  - *Round 3:* applied on this branch at the UI agent's request, with the open chat's project. Reuse matches the Source locator as an exact string. The desktop stores the picker's `path.to_string_lossy()` and the CLI the raw absolute `--source`, so a CLI-supplied file is reused from the picker only when both spell the path the same way (`/tmp` and `/private/tmp`, or a symlinked folder, would give a second chat). Not observed natively; see [owner check 5](#owner-manual-check).
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
  - *Round 3:* `begin_edit` returns `Error::EditSessionOpen { note, session, base_version }`, and CLI `begin-edit` prints `edit_session_open` JSON. Until the UI branch matches the variant, the desktop shows "Could not preserve this edit: this note already has an open unsaved edit. Keep this window open or copy your text." (code reading, not observed).
- **U5: "Partial" badge on verbatim imports.** Click ＋ → "Import full note…" and pick a plain `.md`. *Observed (service):* outcome Partial, "completeness is unverified". The rule is deliberate; the owner should decide whether verbatim Markdown/text counts as Complete before any UI change.
- **U6: kind-specific failure notice (applied in round 3).** Leave the provider unconnected and send a message three times in one chat. *Expected with this build:* one Blocker "Run failed: The provider account needs reconnecting. Continue when the provider is ready." Every failure kind's text is in the [round 3 table](round3-verification.md#u6-failure-notice-wording-cratesbrn-aisrcthreadsrs); none is longer than 89 characters. The interrupted notice is "Run interrupted. Continue starts fresh." (`INTERRUPTED_ATTENTION`).
  - *Round 1 history:* `failed_attention` appended "Continue when the provider is ready." for every failure kind, so a tool-round limit gave "Run failed: The tool round limit was reached. Continue when the provider is ready.". Round 3 applied the wording agreed with the UI agent: the provider sentence only for account, network, rate-limit and unavailable-provider failures, and "Continue starts fresh." only where that is true. The `"Run failed: "` prefix is kept, and `is_terminal_notice` still recognises every earlier saved text.
  - *Known limitation for the UI agent:* the technical cause is not persisted. A retry-after delay, an HTTP status or a provider code appears only in the desktop's live run outcome line (`threads_ui.rs`, `Failed(e) => Err(format!("{e}{notices}"))`) and in the CLI's `failure` JSON. After a restart only the owner-terms cause in the notice remains. `Run.progress` was left alone because it feeds the next model run. A structured failure field on the Run would be a later product choice.

## Integration risks with `work/ui-polish`

State at `3a7c3d25` (round 3). Both branches now start from `771116f0`.

- **Remaining shared runtime seams.** `begin`, `finish`, `Scope` and `failed_attention` in `crates/brn-ai/src/threads.rs`; `resume_queue`, `dispatch_queued` and the new `recover_orphaned_runs` in `crates/brn-threads-app/src/chat.rs`; `source_intake_chat` in `crates/brn-threads-app/src/intake.rs`; `begin_edit` in `crates/brn-threads-core/src/edit.rs`. The UI branch keeps these files off its branch. If either side touches them, resolve by hand and rerun `run_recovery`, `source_intake_chat`, the brn-ai notice/lease tests and the U4 core/app/CLI tests.
- **Desktop files touched on this branch (U2, done here at the UI agent's request).** `crates/brn-desktop/src/threads_ui.rs`: `useful_intake` calls `Workspace::source_intake_chat` with the open chat's project (`project_for_chat`, or `None` for general), plus the `mod intake_tests` declaration. New file `crates/brn-desktop/src/threads_intake_tests.rs`. No layout, styling, control or wording change. Expect a small textual conflict if the UI branch touches the module list or `useful_intake`.
  - Cross-surface reuse needs one path spelling: reuse matches `Source.locator` exactly, the desktop stores the picker's `path.to_string_lossy()` and the CLI the raw absolute `--source`. A `/tmp` against `/private/tmp` spelling, or a symlinked folder, gives a second chat and Source. Owner check 5 covers it (not observed).
- **New notice texts (U6).** Saved notices are fixed per `AiErrorKind` ([round 3 table](round3-verification.md#u6-failure-notice-wording-cratesbrn-aisrcthreadsrs)). With no provider account: "Run failed: The provider account needs reconnecting. Continue when the provider is ready." Interrupted (`INTERRUPTED_ATTENTION`): "Run interrupted. Continue starts fresh." Any UI test asserting "Run failed; check provider…" or a round-1 text will fail. Assert on `brn_ai::threads::INTERRUPTED_ATTENTION` and the `Run failed: ` prefix instead. `is_terminal_notice` and `replace_terminal_notice` are public, and older saved notices are replaced, not duplicated. The technical cause is not persisted (U6 known limitation above).
- **U4 typed error.** The UI should match `brn_threads_core::Error::EditSessionOpen { note, session, base_version }`, which reaches the desktop as `AppError::Core(Error::EditSessionOpen { .. })`, to offer Compare/Discard without matching text. The CLI equivalent is `begin-edit`'s `{"error":{"kind":"edit_session_open",…}}` on stderr with exit 1. Until the UI branch matches it, `note_changed` shows the generic notice with the non-raw Display.
- **U3 finding (no service fields needed).** Target note titles and the Protected marker come from joining `Attention.record` to a `Prepared` in `ThreadsUi.candidates`, its note `Put`s and `ThreadsUi.records` ([finding](u3-finding.md)). The desktop's `ThreadRow` (`threads_lists.rs`) keeps only `reasons` and drops `Attention.record`, so the Needs you row cannot reach the candidate. Keeping `record` in the row model is a UI-branch change.
- **Desktop behaviour that depends on this branch.**
  - `threads_chat.rs::resume_chat_queue` already calls `stop_chat_run` on a queue-claimed Working Run without a liveness check, then `resume_queue`. That pre-existing desktop path is unchanged here and can still stop a Run that is live in another process; `resume_queue` now additionally interrupts lease-free orphans. A lease check there is left for integration, because leases are not on main yet.
  - Desktop tests that seed a Working Run to stand for a live worker and then call Resume/Dispatch will see it interrupted, because the seeded Run has no lease. None exists at `3a7c3d25` (150 headless tests pass, 1 ignored); new ones on `work/ui-polish` might.
  - The UI branch's secondary Stop (U1) uses the existing `stop_chat_run` (a `ChatCommand::Pause`). It is unchanged here and does not check the Run's lease, so the confirmation that another window or the CLI may still be running it stays necessary.
- **Mixed CLI error formats.** `begin-edit` writes `edit_session_open` JSON to stderr and plain text for every other error on the same stream (`crates/brn/src/main.rs`, `begin_edit_error`). Keep this on the `work/cli-agent-contract` integration list: that design already uses the same names and plans to carry the line in `CliError.message`.
- **Service API additions (no removals).** `Workspace::source_intake_chat(project, locator, title)`; `brn_ai::run_lease::{RunLease, run_is_live}`. CLI useful-intake JSON gains `thread` and `reused_chat` (`false` with a new `thread` when the earlier chat's Run is still live); `--project` with `--thread` is now refused; help gains two lines (possible conflict with `work/cli-agent-contract`). `brn_threads_core::Error::EditSessionOpen` is a new variant (round 3). Since desktop `useful_intake` uses `source_intake_chat` (U2), a desktop supply while a CLI Run on the same source is live gets its own chat instead of interrupting that Run.
- **Storage.** New derived folder `derived/run-leases/` (excluded from backup/restore like all of `derived/`; no schema change). `begin()` refuses to start a Run when `derived/` is unsafe (for example a symlink).
- **Mixed builds.** A desktop and CLI from different builds (one without leases) would see each other's live Runs as orphans. Check the combined branch from one build: the full desktop state suite, an unsandboxed `scripts/verify-storage.sh`, and a desktop-plus-CLI run on shared synthetic data.
- **Docs.** Contract text for `threads-behavior.md`, `invariants.md` and `docs/status.md` is proposed above and not applied, because `docs/status.md` is shared with the UI branch. Apply once at integration.
- **Native observation.** The owner ran the three checks on this branch's build ([results](#owner-native-check-results-15-october-2026)). Still to do on the combined branch: a Force Quit during a run, then Resume (the only native path that creates an orphan), and the on-screen Needs you rendering. Neither may run while the UI agent drives the Mac.

## Native observation attempt (Task B): not observed

On 15 October 2026 (pre-rebase `dfe698aa`, now `2530805f`) the owner approved native observation, but the agent shell could not perform it. No app was launched and no GUI was observed.

- **Process list denied.** `ps` and `/bin/ps` fail with "operation not permitted", `pgrep` with "Cannot get process list", and `lsof` sees only itself. The shell cannot confirm that no other `brn-desktop` (the UI agent's or the owner's) is running.
- **Screen capture and UI control denied.** `screencapture -x` fails with "could not create image from display", and System Events fails with error -13056.
- **B1 has no offline path.** The production desktop cannot hold a Run in Working without a live provider. Its only provider inputs are the settings `auth_file` (Codex) and `credentials_dir` (Copilot). With neither available, `provider_client` (`crates/brn-ai/src/threads.rs`) returns `ReconnectNeeded` at once. The binary has no mock endpoint, base-URL override or scripted provider.

The owner chose to record **B1 (⌘Q during a run) and B2 (three provider failures) as not observed**. Agents remain unauthorized to make live provider calls.

**Automated evidence: credential isolation.**
- New workspace settings take `auth_file` from `$CODEX_HOME/auth.json`, or from `$HOME/.codex/auth.json` when `CODEX_HOME` is unset (`crates/brn-threads-app/src/settings.rs`, `bootstrap_settings`, run by `Workspace::open`).
- The Copilot folder defaults to `$HOME/.config/brn` (`threads_ui.rs::default_copilot_credentials_dir`) and is used only when Copilot is connected in Settings.
- Executed: `HOME=…/native-iso-check/home CODEX_HOME=…/native-iso-check/codex brn-desktop --data-dir …/native-iso-check/data --headless-check startup` printed "Threads ready: 2 records". A read-only copy of the store showed `auth_file` `/private/tmp/brn-dwr-data/native-iso-check/codex/auth.json` (absent) and `credentials_dir` null. The folder was deleted afterwards.
- No Keychain lookup was found in the code. Absence of any network request with the GUI running was not observed.

## Owner native check results (15 October 2026)

The owner ran checks 3–5 below in their own Terminal on this worktree's `native-ui` debug build at `20f73edb`, with fresh synthetic data under `/private/tmp/brn-dwr-data/`. Evidence is the owner's `brn_inspect` output and on-screen report, plus an agent read of copies of the synthetic stores (operations table). On screen, the owner reports that the B2 failure notice fit in Needs you without truncation, that ⌘Q gave no confirmation prompt while a run was working, and that no Resume button appeared after the B1 relaunch.

- **B2 (offline).** `auth_file` was `/private/tmp/brn-dwr-data/native-b2/codex/auth.json` (absent). After three sends in one chat: three owner Messages, three `Failed` Runs (fence 2 each), one Blocker "Run failed: The provider account needs reconnecting. Continue when the provider is ready.", and an empty `derived/run-leases/`. **Matches.**
- **Check 5, U2 picker re-supply (offline).** The CLI gave `reused_chat: false`, `Failed:ReconnectNeeded` and `failure.kind` `reconnect_needed`. A desktop picker re-supply through ⌘⇧G `/private/tmp/brn-dwr-data/native-u2/files/` added a second `Failed` Run to the CLI's chat, with no new chat. There was one Source, locator `/private/tmp/brn-dwr-data/native-u2/files/u2-email.txt` and `chat` null (general scope), and one Blocker. The chat has two owner instruction Messages, because the CLI and desktop instruction texts differ (known integration item). **Matches.** The optional `/tmp` spelling was not run.
- **B1 (live provider via the owner's Codex sign-in, synthetic data, owner's choice).**
  - *⌘Q during the intake:* the Run was already `Interrupted` (fence 2) with one Blocker "Run interrupted. Continue starts fresh.", and the store holds its "Record terminal run result" operation. `derived/run-leases/` was empty.
  - *What this means for finding 1:* ⌘Q at this stage commits a terminal state, so the code-inspection assumption that desktop quit leaves the Run `Working` is **not confirmed**. No orphan arose, so lease recovery on Resume was not exercised natively. Crash, kill and Force Quit stay covered by the `run_recovery` service tests only; the U1 Force Quit repro was not run.
  - *After relaunch:* the owner sent "Quick test" in a new chat. About 133 s later they sent "Quick test " (trailing space) in the intake chat. Both Runs `Completed` with replies, and the intake chat's success cleared its Interrupted notice, as designed. The store shows two separate `owner-message:` operations, so these were two sends, not one send dispatched twice.
- **Force-kill attempt (`native-b1-force`, live provider): inconclusive.** The intake Run ended `Interrupted` (fence 2) with one Blocker and an empty `derived/run-leases/`. The operations show "Start a fresh invocation", then "Record terminal run result", then "Chat control: stop and pause" (the queue was paused). BRN wrote that terminal result itself, so the SIGKILL did not land while the Run was working, and no orphan arose. Two explanations remain open: the kill missed the working window, or the intake Run interrupts itself shortly after it starts. Both B1 runs also ended `Interrupted` with empty progress and no operations.
- **Idle intake (`native-intake-idle`, live provider).** The owner left the intake alone. The Run `Completed` and prepared a reviewable note, with Review attention "Review prepared changes". Intake therefore does not interrupt itself. The earlier `Interrupted` intake results came from the owner's ⌘Q, or from a stop before the kill.
- **Force kill (`native-b1-kill`, live provider): stuck state confirmed natively.**
  - *After `pkill -9` (zsh printed "killed"):* the Run was `Working` (fence 1). The chat had no attention, and an unlocked 0-byte `derived/run-leases/<run>_0.lock` was left behind. Before it was killed, the Run had prepared a candidate ("Crane arrival — reported date"; operation `…:proposal:crane-arrival-source-capture-1`).
  - *After relaunch, the orphan was cleared by a different path.* Operation 9, `delegation-choice:…` "Owner configured workspace and stopped prior invocations" (the first-run maintenance/delegation choice, `delegation_chosen: true`), set the orphan to `Cancelled`. It wrote no Thread attention. Lease recovery on Resume/Dispatch therefore never ran. The orphan chat has no Needs you entry. The killed Run's candidate is still active (`candidate_targets` row, operation not retired), but no Review attention points at it.
  - *Owner's next message* ("quikc test", new chat): its Run streamed, then ended `Interrupted` with an `assistant-partial` message and one "Run interrupted. Continue starts fresh." Blocker. This fits the owner's ⌘Q while the reply was streaming: the partial text was kept and one notice was added.
  - *Lease file:* the orphan's lease file is still present after recovery (known limitation, now seen natively).
- **Accidental empty data path.** A paste error left `$B1F` unset, so the desktop was started with `--data-dir /data`. It refused with raw "storage: Read-only file system (os error 30)" and created nothing.
- **Process check.** No BRN app was left running. The `grep` matched only `cargo mutants` / `cargo test -p brn-desktop` processes from another worktree (`.worktrees/quality-simplify`), which are headless test runs, not BRN windows.
- **Round 4 follow-up.** The force-kill gaps (no notice and no Review item after the delegation choice, the leftover lease file) and the missing Resume button are addressed in [round 4](#round-4-a-killed-runs-notice-and-candidate-stay-reachable).

## Round 4: a killed Run's notice and candidate stay reachable

Owner request: "1. i did not see Resume button  2. i dont know. also why cant you just run this test yourself?" Implementation `3919c09a`, [verification](round4/verification.md), [review](round4/review.md) (approved). Automated and headless only; not native-observed, not owner-accepted, not merged.

**Native store facts** (agent read of a scratch copy of `/private/tmp/brn-dwr-data/native-b1-kill/data`): the intake Run `1d31fd5f-…` prepared operation 7, host_key `1d31fd5f-…:0:proposal:crane-arrival-source-capture-1`, Note "Crane arrival — reported date", with a `candidate_targets` row and `retired` 0. After relaunch, operation 9 (host_key `delegation-choice:<uuid>`, reason "Owner configured workspace and stopped prior invocations") wrote Settings `maintenance: true`, `delegation_chosen: true` and set the orphan Run to `Cancelled` (fence 2) with no Thread attention. `derived/run-leases/1d31fd5f-…_0.lock` is still there.

| Gap | Before (native store) | After (round 4, tests T1, T2, T8a/b) |
|---|---|---|
| G1: no notice for the stopped Run | The delegation choice cancelled the orphan and wrote no Thread; the intake chat had no attention and no Needs you entry. | The same checked change adds one "Run interrupted. Continue starts fresh." Blocker to that chat. A replay adds nothing. |
| G2: the prepared candidate unreachable from the chat | Candidate active (`candidate_targets` row, not retired, no receipt) but no Review attention pointed at it; only the sidebar "Review · …" row showed it. | The change also adds "Review prepared changes" (`record` = operation 7). Chat attention is exactly `[Review(op), Blocker(run)]`. Nothing is applied, dismissed or retired. |
| G3: leftover lease file | The unlocked 0-byte lease file stays forever. | Orphan recovery (Resume/Dispatch) removes it with `run_lease::remove_free`, only while holding its lock and only after the recovery change is `Applied`. The settings path still leaves it (limitation). |

**Point 1, no Resume button:** the queue was empty, so Resume (`queue-resume`) had nothing to show. It appears only for a paused queue with saved items (after a restart, a Stop or a failure). **Point 2:** this step's shell is denied `sandbox-exec` and `ps`, and agents may not drive the Mac UI or call a live provider, so the native run stays with the owner.

**Stop and cancel paths and what each now does.** The helper `ended_run_attention` runs inside the change that ends the Run.

| Path | Run ends as | Notice | Review item |
|---|---|---|---|
| Workspace settings (Configure, delegation choice, Pause/Resume maintenance, backup folder) | `Cancelled` | added | added |
| Orphan recovery on Resume/Dispatch | `Interrupted` | kept | added; lease file removed |
| Stop (`stop_chat_run`: desktop Stop, failed-worker pause, resume-stale stop) | `Interrupted` | none | added |
| Archive / ArchiveProgress | `Interrupted` | none | added (the archived chat then shows under NEEDS YOU with the proposal card) |
| Steer (`stop_and_post`) | `Interrupted` | none | added |
| Project commands that fence chats, including Delete to Trash and Restore | `Interrupted` | added | added |
| Full-note import into a project | `Interrupted` | added | added |
| Continue / new Run in the same chat (`begin()`) | `Interrupted` | none | added |
| `finish()` Interrupted or Failed (⌘Q, cancel, provider failure) | as before | existing | added (a Completed Run still adds none) |
| Legacy `resolve_thread` (no production caller); Theme/TopicsNote settings | unchanged | unchanged | unchanged |

`Cancelled` now counts as superseded, so the settings notice gives way to the next terminal result. **Observation only:** the settings path also stops Runs that are live in another process. `update_workspace_settings` does not consult `derived/run-leases/`; the other process's watcher stops on the fence and its `finish()` writes nothing, so the settings change's own notice is what the chat keeps. Shown in-process only (`the_settings_change_also_stops_a_run_that_is_still_live`); not changed.

**Desktop visibility (headless gpui harness, no screen).** Yes, the candidate shows after recovery, and no desktop production change was needed. Before the choice the intake row has no reasons and is not in NEEDS YOU. After `choose_delegation`, Home renders `home-attention-{chat}` with both reasons, the sidebar NEEDS YOU lists the chat, and the opened chat shows `attention-card-0` containing `proposal-{operation}` (Apply and Dismiss). After a kill with a paused queue, `queue-resume` renders and Resume gives the same two reasons. **UI finding, none blocking:** archiving a chat whose stopped Run left a candidate keeps the archived chat under NEEDS YOU. Repro: start a useful intake, wait for the prepared note, Archive the chat while the Run works, then open Home. On-screen layout on the UI branch is not observed.

**Verification counts** (all exit 0 unless stated): fmt, helper build, workspace and desktop Clippy `-D warnings`; workspace tests **721 passed, 12 failed (known `brn-threads-intake` sandbox tests), 5 ignored** (baseline 706/12/4); desktop `native-ui,native-test-support` **152 passed, 1 ignored** (baseline 150/1); `verify-end-to-end.sh --fixtures-only` passed; CLI journey unchanged; `verify-storage.sh` stops at the same sandbox tests. Final fmt, `git diff --check` and link check are in [round 4 verification](round4/verification.md#final-gates-finalize-step).

**Round 4 limitations** (from the review, not fixed): the failure and resume-stale stops share the notice-free `stop_chat_run`; the service could treat a lease-free Run there as an orphan (follow-up is service-side, not desktop). A killed Run that is itself the pending queue item, then Resume, goes through that stop: Review added, no notice, lease file kept, item re-dispatched (untested). Settings and Continue leave lease files. No fail-before is recorded for the helper's own `Cancelled` supersession unit test.

**Integration risks for `work/ui-polish`:**
- Shared seams changed: `begin`, `finish` and `attention` in `crates/brn-ai/src/threads.rs`; `Workspace::recover_orphaned_runs`; core `chat.rs`, `steering.rs`, `projects.rs` and the settings path in `store.rs`; the new `brn_threads_core::run_attention` module. `brn_ai::threads` still re-exports `INTERRUPTED_ATTENTION`, `is_terminal_notice` and `replace_terminal_notice` under the same names.
- Tests that count settings-change writes (now 5 for two Runs, was 3) or expect empty attention after a settings change, Archive, Stop or Steer with a prepared candidate need the new expectations. Rerun `killed_run_candidate`, `run_recovery`, `delegation` and the desktop suite on the combined branch.
- Desktop files touched this round: `crates/brn-desktop/src/threads_intake_tests.rs` (tests only). Archived chats with a Review now appear under NEEDS YOU; decide there whether that is wanted.
- The proposed contract sentences in [round 4 verification](round4/verification.md#proposed-contract-sentences-integration-not-applied) are corrected for Completed Runs and `finish()`; apply them at integration with `docs/status.md`.

## Native round 4 observation (orchestrator, 15 October 2026)

**Setup.** The owner turned off the agent sandbox and granted Accessibility, and confirmed that the UI agent was not driving the Mac.
- *Build:* this worktree's `native-ui` debug build at `0b08b2e8`.
- *Data:* scratch copies of the two prepared offline stores (`r4-obs-settings`, `r4-obs-resume`), with `HOME` and `CODEX_HOME` pointed at empty folders, so no provider call was possible.
- *Driving:* real mouse clicks through a CoreGraphics helper. System Events `click at` does not reach GPUI buttons.
- *Evidence:* screenshots in `/private/tmp/brn-dwr-data/native-evidence/r4-*.png` (outside Git; they show the owner's desktop and Dock) and read-only copies of each store.
- *Before Resume:* the run was `Working` at fence 1, and the lease file was present in the resume store.

**Settings store.** Before the choice, the sidebar listed "Review · Re… proposal" only. The owner's chat "Information from b1-source" sat under RECENT CHATS with no attention. Clicking "Review changes first" showed the following.
- *On screen:*
  - the status line "Saved. Owner configured workspace and stopped prior invocations";
  - a "1 needs you" badge whose list names "Information from b1-source";
  - in the chat, a review card "Review prepared changes" with the proposal "Crane arrival — reported date" (+2 −0, "The source reports that the crane arrives on 22 November.") and Show in note / Dismiss / Apply;
  - a blocker card "Run interrupted. Continue starts fresh.";
  - the run line "Cancelled" and "Continue with current context".
- *In the store:* the Run was `Cancelled` at fence 2. The chat had exactly one Review and one Blocker, and the candidate was unretired. Settings recorded `review_first` 1 and `delegation_chosen` 1. The lease file remained (recorded limitation).

**Resume store.** The composer showed "Queue · 1 waiting · paused" with an enabled Resume and "waiting · Questions · When can the…". Clicking Resume showed the following.
- *On screen:*
  - a "2 need you" badge listing "Questions" and "Information from b1-source";
  - the status line "Work in Questions · Work stopped: Reconnect this account before continuing.";
  - "Queue · 0 waiting · paused" with "saved turn · Questions · When can…";
  - in the intake chat, the same review and blocker cards, with the run line "Interrupted".
- *In the store:*
  - operation `orphan-recovery:f5a7315d-…:0:2:1` ("Interrupt work whose process ended"), then resume, dispatch, one Run in "Questions" (`Failed`) and "record queued outcome";
  - the intake Run `Interrupted` at fence 2, with Review and Blocker on its chat;
  - "Questions" with one owner Message and one Blocker "Run failed: The provider account needs reconnecting. Continue when the provider is ready.";
  - the candidate unretired, and `derived/run-leases/` empty.

**All expectations in the [procedure](round4/verification.md#native-re-check-procedure-owner-or-orchestrator) matched.** Nothing was applied or dismissed. BRN was closed with its window button, and no BRN process was left running.

**Intake sandbox tests with the sandbox off.** `cargo test -p brn-threads-intake --locked --offline` gave lib 25 passed, 3 ignored, and `qualification` 10 passed, 1 failed. The remaining failure, `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch`, is `Pdf preparation failed: Unavailable`. Poppler's `pdftoppm` and `pdfinfo` are not installed on this Mac, and the [preview guide](../../../development/preview-release.md) requires external Poppler for PDF. The crate is unchanged on this branch.

**New UI findings (for the UI agent; reproduce with either store above).**
- **U7: Needs you rows in the sidebar have no chat title.** Once a chat needs you, it leaves RECENT CHATS, and its sidebar row shows only the reason ("Review prepared changes", "Run failed: The provider a…"). The title appears only in the badge's list. Steps: open `r4-native-resume`, click Resume, then look at the sidebar.
- **U8: an internal operation reason shows as the status line.** After the maintenance choice, the status line reads "Saved. Owner configured workspace and stopped prior invocations". Steps: open `r4-native-settings` and click either choice.
- **U9: mixed terms for one stopped run.** After the settings stop, the run line says "Cancelled" while the notice says "Run interrupted. Continue starts fresh.". The notice text is service-owned (round 4 reused `INTERRUPTED_ATTENTION`). Agree the wording with this branch before changing either.
- **U10: two wordings for one failure.** The live status line says "Work stopped: Reconnect this account before continuing.", while Needs you says "Run failed: The provider account needs reconnecting. …". Steps: the Resume store above.

## Round 5: eight reliability slices (A–H)

Owner request: "can you fix all 5 . and let go wiht interrupted". Base `f519534e`; [plan](round5/plan.md), [verification](round5/verification.md) and one approved review per slice (`round5/review-*.md`). Automated and headless only: not native-observed, not owner-accepted, not merged. No slice was deferred. No `threads.sqlite3` schema, dependency or `Cargo.lock` change.

| Slice | Before | After |
|---|---|---|
| A (`e07354db`) | A killed backup or restore left `.brn-backup-stage-*` / `.brn-restore-stage-*` folders forever; restore stages are full data copies (finding 9). | Each stage carries a `.brn-stage` marker its process keeps locked. Error paths remove their stage, and the next backup or restore into the same parent removes abandoned marked stages (Unix). Markerless stages from older builds are kept. |
| B (`02bd457b`, `28c4a757`) | `/tmp/x.md`, `/private/tmp/x.md` or a symlinked folder gave a second chat and a second Source. | Local files are recorded and matched by real path: one chat, one Source. A Source under an older spelling still matches, and the next checked observation records the real path. |
| C (`abd9a84f`) | Stop, the failure and resume-stale stops, Archive, Steer and Continue of a killed (lease-free) Run added a Review item only: no notice, lease file kept. Settings, project commands and imports kept the lease file. | Every path treats a lease-free Working Run as orphan recovery does: one "Run interrupted. Continue starts fresh." notice, Review items and the free lease file removed. A live lease is never removed, and stopping live work adds no notice. |
| D (`b037140c`) | A settings stop wrote `Cancelled` while its notice said "Run interrupted…" (U9). | It writes `Interrupted`. Saved `Cancelled` Runs are not migrated. |
| E (`dd2c8ce8`) | The CLI and desktop posted different instruction texts, and each re-supply posted another owner Message (CLI then desktop: 2). | One text, `brn_threads_app::SOURCE_INTAKE_INSTRUCTION` (the desktop's wording), reused on re-supply unless the owner wrote since: CLI → desktop → CLI gives 1 owner Message and 3 Runs naming it. |
| F (`f5872774`) | Owner search returned notes only, so a saved chat reply could not be found. | Owner search pages carry `messages`, `message_total` and `message_cursor`: final owner/assistant Messages, scoped like notes, hidden while archived or trashed. `search.sqlite` v3 (rebuilt in place). Agent `search_notes` is unchanged. |
| G (`7ed74f26`) | The agent could not retire its own proposal after the owner's decision made it obsolete; it stayed in Review. | Runtime tool `withdraw_proposal { operation }` dismisses only an unapplied candidate a Run of the same chat prepared, through the owner's checked dismissal path. Tools 9 → 10; bundle `brn-threads-guidance-12`. |
| H (`28e64cdf`) | `--data-dir /data` printed "storage: Read-only file system (os error 30)"; a file, a locked parent or a symlink gave similar raw text. | "BRN cannot use the data folder /data: it is on a read-only disk. Choose a folder you can write to." (and named reasons for the other cases), exit 1 as before. |

**Verification (round close, HEAD `28e64cdf`, unsandboxed shell).** Environment as above, plus a fresh `TMPDIR=/private/tmp/brn-dwr-data/r5-final-tmp/`. The first workspace run inherited a `TMPDIR` that pointed at a deleted case folder and failed in `tempdir`; it was rerun, and these are the rerun's results.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper` | ok |
| Clippy `-D warnings`: workspace, `brn-ai --features scripted-runs`, desktop `native-ui,native-test-support` | clean (all three) |
| `cargo test --workspace --no-fail-fast` | **808 passed, 1 failed, 6 ignored** (round 4: 721 / 12 sandbox / 5; round 5 baseline: 732 / 1 / 5). The failure is `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` (Poppler not installed); the other 11 sandbox-class tests pass with the sandbox off. |
| Desktop `--features native-ui,native-test-support --bin brn-desktop` | **158 passed, 1 ignored** (baseline 152 / 1) |
| `scripts/verify-end-to-end.sh --fixtures-only` (`TMPDIR=/private/tmp/brn-dwr-data/r5-final-e2e/`) | passed |
| `cli-journey.py` on `/private/tmp/brn-dwr-data/r5-final-journey` | exit 0. 43 steps: 40 succeeded and 3 refused as designed (two `begin-edit`, occupied backup). The intake chat's first owner Message is now the shared instruction (E). The four killed backups all published before the kill, with 0 stages left; this exercises no sweep. |
| `git diff --check`; `scripts/check-markdown-links.py` | clean; 0 failures |
| `git diff --exit-code 771116f0 -- Cargo.lock` | clean |
| `git diff --numstat f519534e -- crates/brn/src/main.rs` | 8 added, 1 removed (one statement, slice E) |
| Orchestrator rerun at `f6ece107`: `scripts/verify-storage.sh` (unsandboxed; `HOME` set explicitly to the owner's home because the inherited shell `HOME` pointed at a case folder) | fmt, helper build, workspace build and Clippy passed. Tests: **808 passed, 1 failed (Poppler), 6 ignored**, so the script stops there. Its tail was run separately and passed: `brn --help`, `brn` without arguments refused, `brn-desktop --help`, and `--headless-check startup` "Threads ready: 2 records". `--data-dir /data` now prints "BRN cannot use the data folder /data: it is on a read-only disk. Choose a folder you can write to." (H). |

**UI findings (for the UI agent).** No slice paused for a desktop decision.
- **F-UI1: the notes search shows no chat Message hits.** Steps (offline): `brn --data-dir $D init`; `configure --provider Codex --model gpt-6.1-sol --effort medium --auth-file <missing>`; `thread-start --title Replies`; `ask --thread <id> --prompt "Berth survey reply draft 1.4 m"` (fails offline, and the owner Message is saved); `search --query berth` gives `message_total: 1`, `messages[0]` with `chat_title "Replies"` and `role "owner"`. Then `brn-desktop --data-dir $D`, open Notes and type `berth`. *Observed (headless harness):* only `search-empty`, with no `search-status` and no `hit-0`, although `page.messages.len() == 1`. *Expected:* an "In chats" kind that opens `chat`, paged with `message_cursor`.
- **U9 resolved (D).** After the maintenance choice on `r4-native-settings`, the run line should read "Interrupted", matching the notice. Not native-observed.
- **Continue shows the notice while the new Run works (C). Resolved in [round 6](#round-6-owner-decisions-13) (decision 1):** the notice now clears when the next Run starts.
- **An archived killed chat carries the notice (C). Resolved in [round 6](#round-6-owner-decisions-13) (decision 2):** Archive now adds the Review only.
- **A desktop instruction whose Run never started is posted again (E).** Supply a source while another worker is busy so the Run never starts, then supply it again: two identical owner Messages, because reuse needs a Run that named the Message (code reading in review-E; not observed).
- **Not part of round 5:** `brn project-delete` of a project whose chat has a Run fails with "invalid request: run control changes must advance the fence" (`ProjectCommand::Delete` archives Runs without a fence advance; same at `771116f0`). It needs its own slice and an owner decision.

**Integration risks for `work/ui-polish`.**
- *Desktop files touched in round 5:* `crates/brn-desktop/src/threads_ui.rs`, slice E only: `useful_intake` uses `SOURCE_INTAKE_INSTRUCTION` and calls `Workspace::post_intake_instruction` instead of `post_message`, 7 lines. Also `crates/brn-desktop/src/threads_intake_tests.rs` (tests; slices B, C, D, E and F). No layout, control or wording edit.
- *Run state (D):* a settings change (Configure, delegation choice, maintenance Pause/Resume, backup folder) now ends every Working Run as `Interrupted`, not `Cancelled`. Every outside-the-chat stop now writes `Interrupted`, and only the legacy `resolve_thread` (no production caller) still writes `Cancelled`. Run state no longer tells a deliberate Stop from an outside stop; only the notice does. UI tests expecting `Cancelled` after a settings change need `Interrupted`. The core and app READMEs still say settings "cancel" Runs (review-D, not edited).
- *Stop paths (C), as changed in [round 6](#round-6-owner-decisions-13):* Steer, the failure stop and the resume-stale stop of a lease-free Run add the Interrupted notice. The owner's Stop, Archive and Continue no longer do. Every path still removes the free lease file. Rerun the UI branch's Stop, Archive and Resume tests. Seeded Working Runs without a lease count as lease-free.
- *Search hit kind (F):* `SearchPage` gains serde-default `messages: Vec<ChatMessageHit { message, version, chat, chat_title, role, snippet, highlights }>`, `message_total` and `message_cursor`. Notes stay in `hits` with the same `total`. `search.sqlite` schema v3 is rebuilt in place on first open, and again after a rollback. The desktop pays the Message re-check on every keystroke search without rendering it. `user_scale` measured query p95 22.9 ms at 10,000 notes; review-F estimates (not measured) that a query whose candidates span many chats may add 0.3–1.4 s.
- *Runtime tool (G):* the activity line "Withdrawing an obsolete proposal…" with outcomes `Withdrawn`/`Refused`. Tests that count tools (now 10) or pin the bundle (now `brn-threads-guidance-12`) need updating; renumber at merge if 12 is taken. Core `Store::{dismiss_candidate, clear_review_attention}` now carry the owner's Dismiss/Apply cleanup, with unchanged keys and texts.
- *Source paths (B):* new file Sources show the real path (`/private/tmp/...`), and tests comparing temp paths with `Source.locator` must canonicalize.
- *Errors (H):* new `brn_threads_core::Error::DataFolder`; a symlinked data folder is `DataFolder { SymlinkRefused }`, no longer `UnsafeDirectory`.

**Integration risks for `work/cli-agent-contract`.**
- *`crates/brn/src/main.rs` edits on this branch (vs `771116f0`):*
  1. `HELP`: two useful-intake lines (Fix 3).
  2. `begin-edit`: `.map_err(begin_edit_error)` and the new `begin_edit_error` function (`edit_session_open` JSON, round 3 U4).
  3. Intake: `useful_project` from `--project`, checked before reading (Fix 3).
  4. Intake: `source_intake_chat` instead of `new_thread`, giving `(thread, reused)` (Fix 3, Task A).
  5. Intake: the `ask` call takes `brn_threads_app::SOURCE_INTAKE_INSTRUCTION` instead of the inline literal, and the output gains `thread` and `reused_chat` (Fix 3; the instruction is round 5 slice E, one statement reflowed to 8 lines). Take this side at merge.

  Slices A–D and F–H change no line of `main.rs`.
- *Exhaustive error mapping (H):* the contract branch's `From<Error>` must map `Error::DataFolder { folder, problem, cause }` (suggested `data_folder_unusable` with `{folder, problem}`), as it maps `EditSessionOpen`.
- *Output fields:* `search` JSON gains `messages`, `message_total` and `message_cursor` (F). Useful intake now posts the desktop wording (E), and the preloaded intake playbooks keep the dropped guidance.
- *Run states:* CLI terminal strings come from `AiTerminal` and are unchanged. A Run read back after a settings change shows `Interrupted` (D).

**Coordination outcomes (16 October 2026).**
- *CLI exit codes:* the owner confirmed they stay with `work/cli-agent-contract`, and gave that branch this branch's integration notes. This branch does not change exit codes.
- *UI branch decisions* (`work/ui-polish`, now on main `8bef439b`):
  - *U10:* when a Run ends Failed or Interrupted, the desktop status line will show the chat's recorded notice, the same text as Needs you and the recovery card. It drops its own "Work stopped: " formatting. This branch's notice text stays the only source, unchanged.
  - *U7:* the UI branch already shows two-line Needs you rows (chat title, then reason). The native observation here was of the older UI. The UI branch's fix pass adds a test.
  - *U8:* the desktop will stop showing receipt reasons. It will say "Saved." or "Saved. Running work was stopped.", and service strings stay as they are.
  - *F-UI1 and U4 (Compare/Discard on `EditSessionOpen`):* done on the combined branch, because they need this branch's APIs.
  - *U2:* the UI branch leaves `useful_intake` logic alone, but adds notice tones across `threads_ui.rs`. Expect a small textual conflict near `useful_intake`.
  - *Integration checklist addition:* the UI branch's U1 Stop tests seed a lease-free Working Run and then Stop it. At round 5 that Run got an Interrupted notice. From round 6 (`b5212618`), the owner's Stop adds none; see [round 6](#round-6-owner-decisions-13).
  - *Bundle id and tool count:* decided at integration.

**Proposed contract sentences (not applied; apply at integration with `docs/status.md`).**
- `threads-behavior.md`, State and attention: "A Run stopped because of something outside its chat (settings, project changes or imports, process ended, or a lease-free stop) is recorded as Interrupted, matching its notice." (D) For Continue, owner decision 1 replaces the earlier proposal ("keeps its Interrupted notice until the next Run in its chat records a result"): "A chat's Interrupted or Failed notice clears when the next Run in that chat starts; if that Run fails or is interrupted, it records a new notice." (round 6)
- `threads-behavior.md`, Attachments or source identity: "A local file supplied as a source is recorded and matched by its real path, so one file is one Source however its path is spelled; a Source saved under an older spelling still matches and is recorded at the real path by the next checked observation." (B) And: "Useful intake uses one owner instruction from every host. A re-supply of the same source in its chat reuses the chat's latest instruction for that source unless the owner has written since." (E)
- `threads-behavior.md`, search: "Owner search also lists saved final chat Messages (owner and assistant, never partial progress) as their own kind with their chat, scoped like notes: general chats in general search, project chats only in their project's search, and a chat with private context only from itself. An archived or legacy Resolved chat keeps that scope, and its Messages and chat-only notes are marked archived; Trashed chats are excluded until restored. Agent note search never returns chat Messages." (F as corrected by round 6 decision 3. Review-F's note that a Resolved project chat's chat-only notes do not show was a misreading: they are project documents and always showed; see the [caveat](round6/decision3-caveat.md).)
- `threads-behavior.md` §142: "Every run registers ten tools: …, `refresh_candidate`, `withdraw_proposal`." Then: "`withdraw_proposal` dismisses only an unapplied candidate that a Run of the same chat prepared, through the owner's checked dismissal path with a receipt and replay; it never applies anything, refuses applied operations, owner changes and other chats, and is never offered to helpers." (G)
- `invariants.md`: superseded by the round 6 sentence in [Round 6](#round-6-owner-decisions-13) (C as changed by decisions 1 and 2). And: "A backup or restore removes only abandoned BRN stages in its destination's parent: exact BRN stage name, real directory, BRN marker text, and no process holding the marker's lock." (A) And: "An unusable data folder is refused with the folder and the reason in owner terms; the OS detail is kept for logs only." (H)
- `docs/status.md` (not edited): "nine registered tools" becomes "ten".

**Open review notes (non-blocking, not fixed).**
- *A:* the sweep removes whatever is nested in an abandoned stage, including a data folder the owner opened from `<stage>/workspace`.
- *G:* a later ordinary Run can withdraw an earlier Rewrite proposal; retirement happens before the fence-bound cleanup; and a replay after the owner's Undo removes the restored Review item again.
- *H:* marker reads on an existing folder still surface `storage: … (os error N)`, and the "Choose a folder you own" advice also covers a macOS privacy denial.

**Native re-check.** [round5/native-recheck.md](round5/native-recheck.md) covers C (Archive of a killed chat), D (settings stop run line) and F (search) on offline copies of the `r4-native-*` stores; for builds from round 6 on, use [round6/native-recheck.md](round6/native-recheck.md). CLI dry runs on copies gave the expected store states for C and D: Run `Interrupted` at fence 2, `[Review, Blocker]`, the candidate unretired, `derived/run-leases/` empty.

## Round 6: owner decisions 1–3

These are the owner decisions of 16 October 2026 (coordination board, "ok of all 4"). Commits:

- `7ecc43aa`: decision 1, and decision 2 for Archive.
- `b5212618`: decision 2 applied to the owner's Stop, kept in its own commit.
- `747755b3`: decision 3, with the [caveat check](round6/decision3-caveat.md).
- The docs commit that adds this section.

All checks were automated and headless. The work is not native-observed, not owner-accepted and not merged. No `threads.sqlite3` schema, dependency, `Cargo.lock` or `main.rs` change. The derived `search.sqlite` moves to v4.

**Caveat result.** No contract reason. No archive-based exclusion of chat-only notes existed. A project chat's notes are project documents (`project_document_ids`), so the shared context predicate already admits them anywhere in their project, open or not. A general chat's chat-only notes stay private to that chat. The only archive rule in owner search was round 5 F's Message rule. Review-F issue 2 misread this. One currency sentence in `threads-target.md:136` and `chats-and-projects/spec.md:70` needs a correction at integration.

| Decision | Before (round 5) | After (round 6) |
|---|---|---|
| 1. Notice and the next Run | Continue in a killed chat added "Run interrupted. Continue starts fresh." in its start change. An existing Interrupted or Failed notice stayed until the new Run's result, so a chat with no candidate sat in Needs you during the Continue. | `begin()` removes the chat's host terminal notices in the same "Start a fresh invocation" change. These are Interrupted or Failed Blockers, legacy texts included, that name a Run of that chat. A fenced orphan gets none. Reviews, Questions and independent Blockers stay. `finish()` records a new notice when this Run fails or is interrupted. A replay of the start change adds nothing. |
| 2. Archive of a killed Run | Archive and ArchiveProgress of a lease-free Run added the notice, added the Review items and removed the lease file. | No new notice, even for an orphan. The Review items for unapplied proposals are still added, existing notices stay, and the free lease file is still removed. The Run is written `Interrupted`, the state the owner's Stop has always written (review-D). Only the legacy `resolve_thread` writes `Cancelled`, so one state covers every deliberate stop, and Continue treats both alike. |
| 2 applied to Stop (`b5212618`, interpretation pending the lead's confirmation) | `stop_chat_run` of a lease-free Run added the notice. | `stop_chat_run` is the owner's Stop: no notice, Review items added, lease file removed. The new `interrupt_chat_run` keeps the notice for BRN's own stops. Desktop `pause_failed_worker` and the resume-stale stop call it. Revert `b5212618` alone to flip this back; it reverts cleanly on `747755b3`. |
| 3. Search and archived chats | Owner search dropped an archived chat's Messages in every context, and no hit had a label. Chat-only notes already showed under their scope. | Archived and legacy Resolved chats' Messages keep their normal scope. `ChatMessageHit.archived` and `SearchHit.archived` (serde default `false`) label hits from such chats. The label is read from the hit's own snapshot, so Archive, Unarchive, Trash and Restore show on the next search. Trashed chats stay out. Schema v4 rebuilds a v3 index, which had archived chats tagged `hidden`. Agent `search_notes` is unchanged: no Messages, no label, the same predicate, and `context.rs` untouched. |

**Paths that end a lease-free Working Run, after round 6.**

| Path | Interrupted notice | Review items | Free lease file |
|---|---|---|---|
| Orphan recovery on Resume or Dispatch | added | added | removed |
| Settings, project commands, project imports | added | added | removed |
| BRN's own stop: failed worker, resume-stale (`interrupt_chat_run`) | added | added | removed |
| Steer (`stop_and_post`) | added; the follow-up Run's start clears it | added | removed |
| The owner's Stop (`stop_chat_run`) | none | added | removed |
| Archive, ArchiveProgress | none; existing notices kept | added | removed |
| Continue or any next Run in the chat (`begin()`) | cleared | added | removed |

**Verification** at `747755b3`, run unsandboxed with the round 5 environment and a fresh `TMPDIR` under `/private/tmp/brn-dwr-data/` for each run. The fail-before lines for every new or changed test are in the three commit messages.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline` | ok |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings` | clean |
| `cargo test --workspace --locked --offline --no-fail-fast` | **817 passed, 1 failed, 6 ignored**. Baseline 808 / 1 / 6, plus 9 new tests. The failure is the Poppler-only `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch`. |
| `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` | **162 passed, 1 ignored**. Baseline 161 / 1, plus the Archive test. |
| `scripts/verify-end-to-end.sh --fixtures-only` | passed |
| `cli-journey.py` on a fresh `/private/tmp/brn-dwr-data/r6-journey` | exit 0. 43 steps: 40 succeeded and 3 were refused as designed (two `begin-edit`, one occupied backup), as in round 5. |
| `brn-desktop --headless-check startup` on fresh data | "Threads ready: 2 records" |
| `git diff --check`; `scripts/check-markdown-links.py` | clean; 0 failures |
| `cargo build -p brn-desktop --features native-ui --locked --offline` | rebuilt at the code head for the native re-check |

**Corrected proposed contract sentences** (not applied; apply once at integration with `docs/status.md`):
- `threads-behavior.md`, State and attention, after "An interruption only enters Needs you when resuming requires an owner action.": "A chat's Interrupted or Failed notice clears when the next Run in that chat starts; if that Run fails or is interrupted, it records a new notice." (decision 1)
- `threads-behavior.md`, State and attention, after "Archiving preserves outstanding attention requests…": "Archive is a deliberate owner stop: it adds a Review item for each proposal its stopped Run left unapplied, but never a new Interrupted notice, even for work whose process had already ended. The owner's Stop does the same." (decision 2; the Stop sentence is pending the lead's confirmation)
- `invariants.md` (replaces round 5 C's sentence): "Any checked change that ends a Working Run no process holds a lease for adds a Review item for each candidate that Run left and removes its free lease file. BRN's own stops (orphan recovery, settings, project changes, imports, a failed worker, the stale Run before Resume) and Steer also leave one Interrupted notice in its chat. The owner's Stop and Archive add none, and starting the chat's next Run clears it. A live lease is never removed."
- `threads-behavior.md`, search: the round 5 F sentence as corrected above.
- `threads-target.md:136` and `chats-and-projects/spec.md:70`: "Owner search also lists an archived (or legacy Resolved) chat's saved Messages under that chat's normal scope, and labels them and its chat-only notes Archived. Agent retrieval is unchanged."

**Integration risks for `work/ui-polish`.**
- *New fields:*
  - `ChatMessageHit.archived` and `SearchHit.archived`, both serde-default `false`.
  - `archived: bool` on `search::MessageMatch` and `search::NoteMatch`, which are not serde types, so any struct literal needs the field.
  - F-UI1's "In chats" group shows the Archived label from `archived`. Note hits can use theirs too.
- *U1 Stop tests:* the owner's Stop of a lease-free Working Run now adds no notice. It still adds the Review items and removes the free lease file. Those tests should expect no notice, and need no `RunLease` for that. If the interpretation is rejected, revert `b5212618`.
- *Automatic stops:* new `Workspace::interrupt_chat_run`. `threads_chat.rs` `pause_failed_worker` and `resume_chat_queue` call it (two call sites). Keep them if the UI branch edits those lines. Any new automatic stop uses `interrupt_chat_run`; an owner-initiated Stop uses `stop_chat_run`.
- *Notice clearing at Run start:* once a Run starts in a chat, that chat's Interrupted or Failed notice is gone, and the row leaves Needs you unless something else needs the owner. U10's status line therefore has no recorded notice to show while that Run works. UI tests that expect a notice or recovery card during a Continue need new expectations.
- *Archive of a killed chat:* the archived chat shows the Review card only, with no blocker card, beside Unarchive.
- *Index:* `search.sqlite` v4 is rebuilt in place on first open, as v3 was.
- *Desktop files touched in round 6:*
  - `crates/brn-desktop/src/threads_chat.rs`: two call sites, in `b5212618`;
  - `crates/brn-desktop/src/threads_intake_tests.rs`: tests (an Archive test, and an archived leg in the search-answer test).

**Integration risks for `work/cli-agent-contract`.** `search` JSON gains `archived` on every `messages[]` and `hits[]` entry, default `false`. Round 6 makes no `main.rs` change. `stop_chat_run` and the new `interrupt_chat_run` have no CLI caller.

**Native re-check.** See [round6/native-recheck.md](round6/native-recheck.md) for the procedure and [Native round 6 observation](#native-round-6-observation-orchestrator-16-october-2026) for the results:

- Archive of a killed chat: the Review card, no new notice, the lease file gone.
- Settings stop: the run line reads "Interrupted", with the notice.
- Continue after a crash: the notice clears when the Run starts, as the chat's revisions show.
- Search: the archived Message hit is flagged; desktop rendering is still F-UI1.

## Native round 6 observation (orchestrator, 16 October 2026)

**Setup.** Run under the coordination board's Mac lock (`MAC-IN-USE`, 09:44–09:54), with the UI agent off the Mac.
- *Build:* this worktree's `native-ui` debug build at `9f10665a` (code head; `f1bb1a1e` adds only the review record).
- *Data:* fresh copies `r6-native/a` and `r6-native/b` of the two `r4-native-*` stores, and a CLI-made store `r6-native/d/data`. `HOME`, `CODEX_HOME` and `TMPDIR` pointed at empty folders, so no provider call was possible.
- *Driving:* real clicks through the CoreGraphics helper, typing through System Events.
- *Evidence:* screenshots `/private/tmp/brn-dwr-data/native-evidence/r6-{a,b,c,d}-*.png` (outside Git; they show the owner's desktop) and read-only copies of each store, taken after BRN closed.

**A: Archive of a killed chat (decision 2).** Clicking Archive in "Information from b1-source" showed the following.
- *On screen:*
  - the status line "Chat archived. Work stopped and the queue is paused. Actions retain their progress.";
  - "1 needs you", with the chat under NEEDS YOU as "Review prepared changes";
  - the review card for "Crane arrival — reported date" with Show in note, Dismiss and Apply;
  - **no** "Run interrupted" card, and the run line "Interrupted".
- *How this build offers Unarchive:* three places. A header pill "Unarchive" (tooltip "Unarchive this chat"), a body button under "Archived chat · saved history remains available", and the composer line "Archived · Unarchive to reply". "Continue with current context" is still shown above them, and the composer still takes focus (U11).
- *In the store:* the Run is `Interrupted` at fence 2 (version 3). The chat is archived; revision 2 holds exactly one Review whose `record` is the candidate operation, with no Blocker. The candidate `…:0:proposal:crane-arrival-source-capture-1` has `retired` 0, and `derived/run-leases/` is empty.

**B: settings stop.** Clicking "Review changes first" showed the review card, the blocker card "Run interrupted. Continue starts fresh." and the run line **"Interrupted"**. The status line read "Saved. Owner configured workspace and stopped prior invocations" (U8, UI branch).

**C: Continue after a crash (decision 1).** Clicking "Continue with current context" in the same window ended at once offline.
- *On screen:* the review card, exactly one blocker card "Run failed: The provider account needs reconnecting. Continue when the provider is ready.", no "Run interrupted" card, and the run line "Failed". The status line read "Work in Information from b1-source · Work stopped: Reconnect this account before continuing." (U10, UI branch).
- *Chat revisions* (`revisions` joined with `operations.request.reason`):

  | Version | Written by | Attention |
  |---|---|---|
  | 1 | Owner started a thread | none |
  | 2 | Owner configured workspace and stopped prior invocations | Review, Blocker "Run interrupted. Continue starts fresh." (old Run) |
  | 3 | Start a fresh invocation | Review only |
  | 4 | Record terminal run result | Review, Blocker "Run failed: …" (new Run `6c7b6416…:0`) |

- *In the store:* the old Run is `Interrupted` at fence 2, the new Run `Failed`, the candidate unretired, and `derived/run-leases/` empty.

**D: archived chat in search (decision 3).**
- *CLI before the desktop:* `search --query berth` returned `message_total` 1, with `chat_title` "Replies", role owner and `"archived": true`.
- *Desktop:* the Notes filter `berth` showed "0 current notes", "TITLES · 0" and "No current note text matches. Try another word or clear the filter to browse." The ⌘K palette with `berth` showed "No matching notes, chats or commands." Neither renders the Message hit (F-UI1, now covering the palette too).
- *After closing:* `unarchive` then `search` returned the same single hit with `"archived": false`.

**All store expectations in the [procedure](round6/native-recheck.md) matched, and every on-screen expectation matched.** Nothing was applied or dismissed. BRN was closed with its window button, no `brn-desktop --data-dir` process remained, and the Mac lock was removed. The owner's Stop of a lease-free Run (`b5212618`) has no native path on this branch and was not observed.

**UI findings from this check (for the UI agent).**
- **U11: an archived chat offers several next steps.** It shows Continue with current context, three Unarchive controls and a composer that accepts focus under "Archived · Unarchive to reply". The UI branch already plans to make Unarchive the single next step. Steps: part A above.
- **U10 still reproduces on this branch** (part C). The UI branch owns it.
- **F-UI1 includes the ⌘K palette.** The archived chat's Message matched neither the Notes filter nor the palette (an open chat's Message was not tried natively). The service answer carries it in `messages[]` with `archived`.

## Owner manual check

The earlier checks 3–5 (B2, U2 picker re-supply, B1) ran on 15 October 2026; results are [above](#owner-native-check-results-15-october-2026) and their steps are in commit `d83ba18d`. One check remains: the round 4 native re-check. Run it in your own Terminal, outside the agent sandbox, with no agent driving the Mac. It uses a live provider (your normal Codex sign-in) on synthetic data; whether to run it is your call.

```sh
cd /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability
export PATH=/opt/homebrew/opt/rustup/bin:$PATH CARGO_TARGET_DIR=/tmp/brn-dwr-target
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
cargo build -p brn-desktop --features native-ui --locked --offline
R=/private/tmp/brn-dwr-data/native-r4   # must not exist yet
printf '# Crane note\n\nSynthetic: the crane arrives on 22 November.\n' > /private/tmp/brn-dwr-data/b1-source.md
BRN_INTAKE_HELPER=/tmp/brn-dwr-target/debug/brn-intake-helper \
  /tmp/brn-dwr-target/debug/brn-desktop --data-dir "$R/data"
```

1. On Home, make the delegation/maintenance choice first.
2. In a chat, click ＋ → "Add useful information…", choose `/private/tmp/brn-dwr-data/b1-source.md`, and wait until it shows Working.
3. From another Terminal: `pkill -9 -f '/private/tmp/brn-dwr-data/native-r4'`.
4. Relaunch with the same command. Open a new chat (⌘T), send a message, and click Resume if one is shown.
5. *Expected:* the intake chat shows one "Run interrupted. Continue starts fresh." notice and a Review item for the proposed note in Needs you, and the message dispatches once. If the kill landed after the note was proposed, the chat opens with the proposal card.

Quit with ⌘Q, paste the `brn_inspect` helper below, then run `brn_inspect "$R/data"` and the candidate query:

```sh
C="$(mktemp -d /private/tmp/brn-dwr-data/inspect.XXXXXX)" && cp -p "$R/data"/threads.sqlite3* "$C"/ && \
sqlite3 -header -column "$C/threads.sqlite3" \
  "SELECT o.host_key, o.retired, t.note FROM candidate_targets t JOIN operations o ON o.id = t.operation;"; rm -rf "$C"
```

*Expected in the store:* the intake Run is `Interrupted` at fence 2, its chat has one Blocker and one Review row, the `candidate_targets` row's operation is `{run}:0:proposal:…` with `retired` 0, and `derived/run-leases/` no longer holds that Run's file. If no Review appears, check whether the `candidate_targets` query returned a row: an empty result means the kill landed before the note was proposed, so repeat with a later kill.

**Inspection helper.** Paste once per Terminal and run only after BRN has quit. It queries a scratch copy, so BRN data is never written (a cleanly closed store cannot be opened `mode=ro`, and `immutable=1` would ignore a leftover WAL).

```sh
brn_inspect() {
  local data="$1" copy
  copy="$(mktemp -d /private/tmp/brn-dwr-data/inspect.XXXXXX)" || return 1
  cp -p "$data"/threads.sqlite3* "$copy"/ || return 1
  sqlite3 -header -column "$copy/threads.sqlite3" <<'SQL'
SELECT json_extract(body,'$.data.Settings.provider') AS provider,
       json_extract(body,'$.data.Settings.auth_file') AS auth_file,
       json_extract(body,'$.data.Settings.credentials_dir') AS credentials_dir
FROM records WHERE json_type(body,'$.data.Settings') IS NOT NULL;
SELECT substr(id,1,13) AS run, version,
       json_extract(body,'$.data.Run.state') AS state,
       json_extract(body,'$.data.Run.fence') AS fence,
       substr(json_extract(body,'$.data.Run.thread'),1,13) AS chat,
       json_extract(body,'$.data.Run.queue_claim') IS NOT NULL AS queued
FROM records WHERE json_type(body,'$.data.Run') IS NOT NULL;
SELECT substr(r.id,1,13) AS chat,
       substr(json_extract(r.body,'$.data.Thread.title'),1,30) AS title,
       json_extract(a.value,'$.kind') AS kind,
       json_extract(a.value,'$.reason') AS reason
FROM records r, json_each(r.body,'$.data.Thread.attention') a
WHERE json_type(r.body,'$.data.Thread') IS NOT NULL;
SELECT substr(json_extract(body,'$.data.Message.thread'),1,13) AS chat,
       json_extract(body,'$.data.Message.role') AS role,
       substr(json_extract(body,'$.data.Message.text'),1,50) AS text
FROM records WHERE json_type(body,'$.data.Message') IS NOT NULL;
SELECT substr(id,1,13) AS source,
       json_extract(body,'$.data.Source.locator') AS locator,
       substr(json_extract(body,'$.data.Source.chat'),1,13) AS source_chat
FROM records WHERE json_type(body,'$.data.Source') IS NOT NULL;
SQL
  rm -rf "$copy"
  ls -la "$data/derived/run-leases/" 2>/dev/null || echo "derived/run-leases/: absent"
}
```
