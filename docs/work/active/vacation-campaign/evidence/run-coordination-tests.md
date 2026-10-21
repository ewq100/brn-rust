# Run coordination test/contract inventory

**Scope:** read-only inventory at `46aebfaa6ff20d39a7afaae203b99a4b34b1ab12` (`codex/agent-first-followthrough`), source base recorded by lead as `main7fd7142c`. No tests/builds or source/repository writes performed. Existing campaign/overview modifications were preserved.

## Retained UI journey witnesses

- `crates/brn-desktop/src/threads_steering_tests.rs::stop_and_send_click_is_durable_and_waits_then_drains_without_retargeting` — actual `chat-steer` click; asserts cancellation, persisted run Interrupted/fence 1, active lease remains, no premature notice, second send cannot replace captured instruction, no new worker before Finished, navigation to another thread retains its composer target, terminal + late old text do not contaminate its stream/draft, and original chat reopens with queued draft. Strong end-to-end UI/service/event/effect witness.
- `...::pending_disconnect_retains_instruction_and_offers_continue_after_reopen` — actual click plus text then sender drop; polling observes disconnect and asserts worker/pending cleared, unexpected-disconnect notice, no stream, Continue visible after refresh, persisted continuation eligibility and unchanged two messages. Strong UI/service/reopen witness.
- `...::stop_during_pending_send_preserves_message_and_cancels_automatic_launch` — actual steer then Stop; asserts pending launch cleared, notice says automatic continuation stopped, late Finished retires worker without provider startup, saved run remains Interrupted/fence 1.
- `...::failed_steering_persistence_retains_text_worker_and_controls` — calls `stop_and_send` directly with resolved thread to force persistence error; asserts original composer text, no pending send/cancel, worker and Stop control survive, error notice, and no extra message. Useful caller error-path witness, but not a UI click.
- `...::enter_queues_while_working_and_stop_pauses_saved_queue_across_late_finish` — direct `ask`/`cancel` calls, event injection; asserts message queued (not steering/cancel), Stop pauses durable queue, late Finished saves displayed partial exactly as assistant-partial, reopened ChatControl remains paused with one item and dispatch returns None. Strong persisted effect witness though the displayed partial is directly seeded.
- `crates/brn-desktop/src/threads_chat_tests.rs::queue_notices_follow_the_queue_state_and_paused_wins_after_stop` — holds a synthetic worker, sends/edits queued items via composer, Stops then sends again; checks visible labels/tone/unavailable Resume, live worker still exists, saved queue remains three entries after late Finished. Strong caller and UI-state witness; its synthetic worker/lease are test harness state.
- `...::an_idle_send_skips_the_queue_and_replaces_an_earlier_queue_notice` — actual composer send while idle; asserts direct-start path does not persist a queue item and provider refusal replaces stale queue notice.
- `crates/brn-desktop/src/threads_steering_tests.rs::quiet_display_keeps_stream_and_stop_and_saved_drafts_survive_reopening` — directly seeds `stream` and progress then navigates away/back; confirms worker identity and thread-specific stream retained, Stop visible, and both chats’ unsent drafts plus quiet preference persist. Navigation/draft witness, but not a WorkerEvent-to-stream witness.
- `...::archive_and_steer_preserve_partial_answers_and_repeated_stop_does_not_duplicate` — directly seeds stream and invokes archive/steer/Stop, then injects late Finished; asserts exactly one persisted assistant-partial in-session and after reopen. Good exactly-once persistence effect.
- `...::quiet_toggle_removes_activity_from_every_live_status_without_changing_work` — actual quiet UI clicks and progress WorkerEvents; checks activity presentation changes while worker identity, cancellation, provider, stream, terminal error remain; includes a separately seeded import worker to ensure unrelated disconnect does not rewrite retained run status. Mixed: UI presentation is exercised, but final unrelated-worker scenario manually replaces `ui.worker` and sets `ui.progress`.
- `...::starting_a_replacement_run_clears_the_old_paused_notice` — calls `start_run` directly after seeding Interrupted state; asserts new worker/Starting and old notice gone. Caller-level direct setup, not a button flow.

## Completion, notices and presentation

- `crates/brn-desktop/src/threads_steering_tests.rs::a_failed_finish_is_an_error_banner_and_restores_as_an_error` — sends Finished(error) through `poll_worker`, dismisses/restores notice via UI; error text and tone retained.
- `...::a_finish_takes_its_tone_from_its_own_recorded_run` — Finished success with newer synthetic Run seeded directly; confirms tone follows this worker’s run state, not newer chat run; output worker without Run is Success. Strong cross-identity assertion, but setup is direct-field/synthetic-record oriented.
- `...::a_failed_or_interrupted_finish_shows_the_recorded_terminal_text` — direct synthetic attention/run setup and Finished(error); confirms selected run’s recorded notice/tone wins over raw error, agent progress and older notice, while owner-cancel uses short Stop warning. Presentation contract, not real service terminal production.
- `crates/brn-desktop/src/threads_ui.rs::presentation_tests::finish_tones_follow_the_owner_stop_and_the_recorded_run` — narrow pure tone helper contract; mechanism/presentation only.
- `crates/brn-desktop/src/threads_chat_tests.rs::a_failed_reply_reads_as_its_recorded_notice_on_the_card_and_in_needs_you` — saved Run/notice fixtures; checks recovery card and Needs You text/tone. Durable display contract, no live worker transition.

## Worker transport mechanism (retain separately)

`crates/brn-desktop/src/threads_worker.rs::tests` are channel/Worker-drain mechanism witnesses, not owner-transition/effect tests:

- `an_empty_live_channel_waits_but_an_empty_closed_channel_disconnects` and `progress_and_partial_text_are_admitted_before_disconnect`: Pending/Disconnected semantics and drained payloads.
- `the_first_terminal_keeps_its_payload_and_rejects_trailing_old_events`: first Finished/RunFinished/Converted payload wins; late events discarded.
- `a_terminal_finishes_without_waiting_for_the_producer_to_disconnect`: terminal ends independently of sender drop.
- `a_large_backlog_is_drained_in_fifo_batches_through_its_terminal`, `terminal_at_the_budget_edge_wins_and_leaves_the_tail_unread`, `a_closed_channel_reports_disconnect_after_its_remainder_is_drained`: bounded FIFO draining and terminal/disconnect ordering.

Retain these if `Worker::drain` changes. They do not prove UI handling, durable state, queue dispatch, partial persistence, or navigation isolation; those belong to the UI journey witnesses above.

## Intake-adjacent evidence

`crates/brn-desktop/src/threads_intake_tests.rs::a_chat_whose_run_is_still_live_is_not_reused` exercises intake’s refusal to reuse a chat with a held worker/run, and killed/recovery tests such as `resume_after_a_kill_recovers_the_intake_and_keeps_its_candidate_reviewable` exercise recovery/Continue. These are chiefly intake/recovery contracts, so retain only if coordination-owner changes touch worker reuse or recovery callers. `queued_question_dispatches_after_successful_attachment_preparation` does verify attachment completion routes a queued item into startup; it checks queue pending/paused and saved note under offline provider refusal. It is an attachment-specific effect witness, not a general queue transition substitute.

## Proportionate retained map for a state/event/effect owner

Keep a small black-box UI/service set that crosses the proposed owner boundary: (1) Stop-and-send exact pending instruction, no early launch, late-terminal/navigation isolation; (2) pending disconnect/reopen Continue; (3) Stop suppresses pending automatic launch and persists paused queue/partial once; (4) idle-send versus working-queue and queue paused after late finish; (5) completion/error selected-run tone and terminal notice; (6) navigation preserves each composer/stream association. Keep `Worker::drain` transport tests separately if its batching/terminal contract is changed. Preserve the attachment-dispatch witness only when that caller is migrated. This is a behavioral map, not a target count: combine cases where the new owner’s public transition/effect seam makes them clearer, and retire redundant tests only when the same caller-triggered externally visible effect remains witnessed.

Mechanism-only/manual setup to avoid treating as proof of coherent ownership: tests that assign `ui.stream`, `ui.progress`, replace `ui.worker`, set notices, or construct synthetic Runs/attention records prove formatting or a chosen branch, not that a legal event sequence creates that state. They remain useful narrow presentation fixtures. The strongest transition tests call UI methods directly rather than clicking controls; preserve at least one actual UI trigger for Send/Stop/Steer. Do not move durable queue/Run/lease assertions into the new in-memory owner: reopened Workspace and checked-service assertions establish the authority boundary.

## Limits / no source-backed missing transition identified

This inventory found ample witnesses for the named journeys and no concrete missing behavior warranting a new transition test before the owner boundary is selected. Existing tests intentionally cover some awkward races by directly injecting terminal events after Stop and navigation. Whether a new coherent owner makes an unrepresented combination constructible cannot be concluded from test names alone; the lead’s source-owner map should decide whether one narrow legal event-sequence regression is needed. No native UI acceptance is implied by headless UI tests.
