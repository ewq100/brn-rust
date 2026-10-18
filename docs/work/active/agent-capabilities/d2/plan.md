# D2 continuation delta implementation plan

**Goal:** implement only amended Rig-first§6/R01–R04 over accepted D1B8ae8a729, preserving the landed reliability/CLI/CW/D1 contracts.
**Architecture:** readable bounded Run.progress plus host-recorded operation identities and existing checked Store queries. Fresh Rig invocation, no exact checkpoint or new durable state. Freeze optional untrusted continuation once after D1A history and initial intake assembly; mandatory input wins and the actual growing request guard stays.
**Stack:** Rig0.44.0/Rust1.98.1/current lock. No dependency/schema/guidance/core/app/CLI/UI changes.
**Authority:** source preparation began while B3 owned heavy; explicit D2 sole heavy regrant now permits serial offline scripted qualification. Implement natively in this session, no delegation/plan approval ceremony; principal owns later grant/review/integration. [Binding/ownership](binding.json).

## Existing contracts versus delta

| Registry | Existing production and witnesses (reused; affected suites rerun) | Missing D2 witness/implementation |
|---|---|---|
| R01 | threads.rs prepare records Run.operations before store.prepare, immutable request_key; apply receipt replay; existing durable_candidate_retry_receipt_and_fresh_continue_have_no_source_archive. Existing scripted_then_abandon exposes actual pre-terminal crash; core active_candidates/review preserve prepared proposals. | Expose bounded fresh Allocated/Prepared/Retired/Applied distinction from operation_info, never text/ID guesses. Actual allocated-before-prepare, prepared-before-result, applied-before-result fixtures; same operation replay no duplicate version. |
| R02 | check_scope/current inputs, current-read ledger, refresh_candidate; protected_changes_require_review_and_new_continue_fences_old_tools; threads_steering_tests::fresh_queued_preparation_has_no_previous_owner_scope_and_uses_current_input; CW2 reference/current-version guards. | Fresh projection distinguishes unapplied stale expected versions from immutable prior Applied receipts; visible-only IDs; no read ledger or protected grant. Owner-edit stale fixture plus relevant inherited existing tests. |
| R03 | run leases/orphan recovery; app run_recovery::resume_after_a_crash_interrupts_the_orphan_once_and_dispatches_the_queue_once; a_live_run_keeps_blocking_resume_and_dispatch; recovery_runs_once_and_cannot_be_undone. app chat::late_or_forged_completion_does_not_advance_and_stop_preserves_saved_partial, dispatch_claim_replay_restart_and_competing_process_cannot_repost_instruction. AI steering startup/Stop/config/archive tests and live/update_run/finish fences. | New budget-fallback and retained-plan callback must still refuse after same barriers. Do not reimplement queue acknowledgement or scheduler. |
| R04 | bounded_tool_rounds_and_cancel_keep_distinct_terminal_states, D1A mandatory/growing context-limit tests, partial Message roles and failed_attention. | Bounded objective/remaining_work/open_questions fields, legacy summary still valid, operations verified from host family; budget failure retains plan or exact trusted owner Message excerpt when none. Fresh actual dispatch after reopen, optional budget omission/no trim/prefix freeze. |

## Bounded source steps

- [x] Save test-only missing-feature witnesses on exact8ae production BEFORE production edits, separate genuine Git source/target. Do not claim unexecuted red or compiler failure as red.
- [x] New private continuation module: optional bounded wire fields; <=4096readable progress, no parsed prose success; verify refs through same-chat Run.operations+fresh operation_info/check_scope/current visibility; return receipt/candidate metadata only, no before/after Note bodies. Bounded query/result counts and byte limits, explicit limited coverage.
- [x] threads.rs new tool arguments/schema handler reuse existing checked progress update; preserve legacy calls/tests. Add optional frozen block to prepared input, budget-check after rendering, explicit notices on omission. No changes to summary/auth/wire/helper/source/history/permission implementations.
- [x] Budget terminal integration only: if progress empty, retain <=768byte excerpt of actual trusted posted owner Message plus its identity, with explicit bounded excerpt/not-a-plan wording. No guessed remaining steps, no completion claim, no write after live fence fails. Writing request path unchanged.
- [x] Focused model-loop/checkpoint/privacy/injection/late-callback/optional-fit fixtures; strengthen production journeys without duplicating queue/lease/receipt tests.
- [x] Static direct pinned rustfmt/diff/hash and initial BUILD NEED receipt preserved. Explicit heavy grant exercised for serial self-checks; immutable pin and fresh principal review follow separately.

## Review focus and future checks

- Foreign/hidden operations, historical progress prose or raw source instructions cannot become current permission/read coverage/completion proof.
- A prior Applied receipt remains Applied even if current versions differ; a Prepared/Allocated/Retired operation is never called Applied. Replays preserve original receipt and versions.
- Stop/archive/steer/new run and settings changes still reject late progress/terminal writes and never alter queue claims.
- Optional continuation cannot displace mandatory current/writing/config input; no in-run history mutation; stable raw prefix remains first.
- Legacy/unrecognized progress stays explicitly reported text; no parser infers done from prose. Unknown/oversized wire fields fail before any progress effect.

Executed focused/full scripted AI430/0/1, affected core31/0/0, app chat/run_recovery13/0/0, CLI161/0/0, strict affected all-target Clippy and workspacefmt. [Implementation/review brief](implementation.md) and [exact self-check receipts](self-checks.json) distinguish all failed initial commands from final checks. L/live/native-human/hosted/owner/full program/PDF gaps remain open. D1B cap finalized; D2 has no granted/executed mutation campaign.
