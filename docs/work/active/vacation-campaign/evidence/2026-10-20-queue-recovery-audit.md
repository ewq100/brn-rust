# Queue recovery source audit

**Result: no consequential source defect or missing behavioral witness found in the bounded queue/Run recovery path.** Inspection only; no builds, tests, native actions, or repository changes were performed.

## Assessed path

- `crates/brn-threads-app/src/chat.rs`: `dispatch_queued` replays a durable dispatch receipt, recovers lease-free orphan Runs before a fresh dispatch, and returns the same pending item on a same-key retry. `resume_queue` performs orphan recovery before the checked Resume.
- `crates/brn-threads-core/src/chat.rs`: Dispatch persists the owner Message and dispatch claim with the pending item. Bind checks the exact item/message/dispatch plus a Working Run whose instruction and queue claim match. Resume refuses any Working Run, then clears the old pending run/dispatch so a later dispatch uses a fresh claim while retaining the saved message. Finish checks current pending item, message, Run identity and fence.
- `crates/brn-threads-core/src/store.rs`: queued Run creation requires the current unpaused pending claim and exact message/thread/run linkage; terminal Runs cannot reactivate, and Run instruction/claim identity is immutable. Run control changes advance the fence.
- `crates/brn-threads-core/src/steering.rs`: Stop-and-send commits the owner Message, interruption/fence and matching pending-item rewrite in one checked change when the stopped Run owns that pending item. The new pending dispatch is bound to the new message. Old Run writes are fenced.
- `crates/brn-desktop/src/threads_chat.rs`: local worker completion is acknowledged only against its current run identity; queue completion goes through the checked Finish. Stop fences the saved Run before cancellation. Resume handles a saved stale pending Run before dispatch.

## Behavioral witnesses already present

- `crates/brn-threads-app/tests/chat.rs::dispatch_claim_replay_restart_and_competing_process_cannot_repost_instruction` covers same-key replay, a competing workspace, restart, explicit Resume, same saved message with a fresh dispatch claim, and one persisted owner Message.
- `crates/brn-threads-app/tests/chat.rs::late_or_forged_completion_does_not_advance_and_stop_preserves_saved_partial` covers Stop with partial preservation, rejection of a late success and old failure after Resume/redispatch, and the new invocation completing without advancing after the queue is paused.
- `crates/brn-threads-app/tests/run_recovery.rs::resume_after_a_crash_interrupts_the_orphan_once_and_dispatches_the_queue_once` and `a_live_run_keeps_blocking_resume_and_dispatch` cover restart recovery, one-time interruption, dispatch once, and a live lease continuing to block Resume.
- `crates/brn-threads-app/tests/killed_run_candidate.rs::resume_after_a_killed_queued_run_leaves_one_notice_and_redispatches_once` covers recovery of a killed claimed queued Run, preservation of its owner Message, one review/notice, fresh dispatch and rejection of a second dispatch.
- `crates/brn-threads-app/tests/steering.rs::stable_key_replays_one_durable_message_after_resolution_and_reopen` covers Stop-and-send receipt replay and exactly one durable owner Message. `killed_run_candidate.rs::stop_archive_and_steer_of_a_killed_run_review_its_candidate_and_remove_the_lease` covers orphan Stop-and-send recovery attention/lease behavior.

Together, the store identity/fence checks and the cited tests cover the requested old-worker-settles-after-pause/resume and process-restart boundary. The live Stop case intentionally leaves the held lease until the worker drops it; the already-fenced Run cannot commit later writes, while the resumed dispatch creates a separate Run identity.

## Scope limits

This was a bounded source audit, not test execution. I did not inspect preview/navigation behavior or propose a new reducer, framework, or test because no uncovered queue durability or dispatch behavior emerged. Branch HEAD inspected: `4f6a070d`; dirty/untracked campaign evidence present at start was left untouched.
