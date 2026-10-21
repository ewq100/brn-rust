# BRN Run coordination source map

Read-only bounded source/consumer inventory. No source, shared record, or Git changes were made; no checks were run.

## Source identity and scope

- Checkout: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`
- Observed branch: `codex/agent-first-followthrough`
- Exact HEAD: `46aebfaa6ff20d39a7afaae203b99a4b34b1ab12`
- User/lead supplied production comparison: `main` at `7fd7142c`; this checkout's local `main` ref resolves to `18f3891e29e57d4459c663967d26115f0e57787d`, so I did not independently verify the production equivalence claim.
- Pre-existing dirty files preserved: `docs/work/active/vacation-campaign/campaign.json`, `docs/work/overview.md`.
- Route read: repository AGENTS, development overview, current campaign item `run-coordination-state-ownership`, `worker-identity.md`, `worker-lifecycle.md`, and strategic-programming design rules.

## Current responsibility split

`threads_worker.rs` owns transport protocol and invocation identity, not the full Run lifecycle. `Worker` has a private operation enum (Run with required `{thread, run_id}`, Import, Export, Backup, Connect); `drain()` caps a batch at 128, includes the first terminal event, and distinguishes Pending/Finished/Disconnected. The host must retire the consumed old worker before successor dispatch. This is already an explicit state boundary and should remain.

`threads_ui.rs` owns one shared `Option<Worker>` for every asynchronous operation plus separate `pending_send`, `stream`, `progress`, `run_activity_labels`, `run_activity_chat`, `run_activity_status`, and UI scroll handle. `threads_chat.rs` owns queue projection/rendering and the Workspace calls to enqueue/edit/remove/move/dispatch/resume/finish/pause. Workspace/SQLite remain checked durable authority for queue claims, run/fence state, leases, interruption and message commits.

## Material caller sequences and transition facts

- Run admission: `begin_run_start` rejects if *any* Worker is present, refreshes, then `start_run` matches the saved prompt/thread to an unclaimed pending queue item and calls `start_request`. The latter reviews MCP input, calls checked `prepare_thread_run`, attaches reviewed input, spawns, and only after successful spawn installs the Run Worker. It separately initializes activity chat/status, clears stream/activity labels, sets Starting progress, clears notice. Spawn/preparation failures do not install the Worker and preserve saved instruction/queue failure handling.
- Queue dispatch: `enqueue_composer` commits queue/edit first, clears the composer only on success, then dispatches only when idle and unpaused. `dispatch_next` returns while a worker exists; on dispatch claim it calls `start_run`, pausing if no Worker was installed. Dispatch failure also pauses. `resume_chat_queue` refuses a live worker, may interrupt a stale Working pending Run, resumes through Workspace, then dispatches.
- Stop and Send: only allowed when no prior `pending_send` exists and the current Worker is a Run for the displayed thread. It commits `stop_and_post_with_partial` with exact Run ID and visible partial first. Only after success it installs `{thread,prompt}`, clears composer/draft, cancels the existing token, updates progress and notice, refreshes. A commit failure leaves composer and worker untouched.
- Ordinary Stop/Continue cancel: `cancel` removes pending auto-send (warning: saved instruction remains), commits `stop_chat_run` with the local Run ID and partial stream, otherwise pauses queue; then cancels worker and conversion token only if cancellable, retaining progress until terminal. Stop failure returns before cancellation. Saved Run Stop is a separate confirmed path through checked Workspace and does not imply local Worker ownership.
- Terminal poll: `poll_worker` maps `RunFinished` to the same terminal handling as `Finished`, calls `acknowledge_run` before processing it, and processes events in FIFO order. It reads tone/notice from that worker's own Run record, updates activity status, then clears worker/stream/progress and completes notice. At batch end, pending Stop-and-Send has priority and starts its saved prompt after old batch consumption; otherwise successful queue acknowledgement dispatches next. No dispatch occurs on a pending batch.
- `acknowledge_run` re-reads current pending queue and compares its `q.run` with the local Run identity; validates exact terminal `(run,fence)`; queued completion calls checked `finish_queued` with queue item/message/run/fence/success. Failed or unconfirmed terminal does not advance, and failure may pause/recover. For unqueued Runs it checks the persisted fence/state and pauses on unsuccessful terminal.
- Disconnect: `worker_disconnected` first calls `pause_failed_worker` while the old worker identity and partial stream are still available; marks Run activity Disconnected; clears worker, pending continuation, stream and progress; shows failure and refreshes. The checked effect is either `interrupt_chat_run(run_id, partial)` or `recover_queue()` for non-Run operations.
- Other local interruptions: archiving a chat commits first, then cancels matching Run, clears pending send, and leaves queue paused. Project commands can fence a Run via receipt; the caller cancels its worker and clears pending send. These callers are why a Run-state owner must offer a narrow “fenced/stop continuation” transition without taking over Workspace decisions.
- Presentation consumers: `threads_chat.rs` records bounded activity labels and displays run activity/status; `threads_writing.rs` uses Run identity, activity chat/labels and `stream_for`; `threads_projects.rs` wraps project command with current run/partial and handles a fenced receipt. `threads_shell.rs` exposes Cancel; `threads_writing.rs` exposes Stop-and-Send. Numerous desktop modules/tests directly read or seed `worker`; therefore moving the unified Worker behind a new coordinator would impose a broad interface migration.

## Recommended bounded owner

Introduce one private `RunSession` value in the existing desktop module layer for *Run-specific presentation/steering coordination*, while leaving `Option<Worker>` as the transport slot for all operation kinds and leaving queue admission/policy and durable effects with current owners. Suggested contents: required copied `RunIdentity`, partial answer, progress label, bounded activity presentation (`chat`, labels, status), and optional `PendingSend` that is structurally tied to that Run identity. Keep the toolkit scroll handle in the UI view.

Keep the boundary deliberately small: it owns valid local combinations and cleanup/continuation transitions; it does not decide whether a queue dispatch is allowed or claim/finish/pause/interruption. Start is installed only after preparation/spawn succeeded. The existing Worker remains the source of cancellation and channel ownership; assert/match its identity against the session at consuming boundaries rather than duplicate transport state as authority. Retirement should remove loose Run-only fields and repeated Run cleanup assignments from `threads_ui.rs`, plus the `PendingSend` struct/field if subsumed. Preserve existing public-to-sibling access through a few read methods (`run_identity`, `partial_for(thread)`, activity/status) and narrow transitions (`started(identity)`, `stop_and_send_committed(identity,prompt)`, `cancel_continuation`, append/replace answer, progress/activity, `terminal(status)`, `disconnected`). Avoid one getter per old field and avoid changing unrelated non-Run progress behavior.

A still-smaller seam (just bundling the repeated clears) would have low caller cost but would not eliminate invalid states such as a pending send unrelated to the current Run or run activity/partial without an owning invocation. A whole `UiState` reducer that absorbs the unified Worker, queue, view navigation, input, notices or settings has a substantially larger consumer migration and crosses the existing transport/service boundaries.

## Candidate transition vocabulary

- States: `Idle`; `Running { identity, partial, progress, activity }`; `StoppingForSend { identity, partial, progress, activity, pending_send }`. Status may be represented as a field/enum within Running (e.g. active, terminal label, disconnected) if that avoids another parallel optional.
- Events: `RunStarted(identity)`; `AnswerReplaced(text)`; `TextAppended(text)`; `Progress(text)`; `Activity(text)`; `StopAndSendCommitted(identity, prompt)`; `ContinuationCancelled`; `RunTerminal(display_status)`; `RunDisconnected`; `RunFenced(identity)` (used after a successful project/archive checked effect).
- Effects/outcomes: state transition returns optional `StartSavedPrompt { thread, prompt }` only after the terminal batch has been fully consumed; otherwise it returns presentation changes/cleanup intent. Host executes effects: checked Workspace stop/post or interruption/finish, cancellation token operation, notice/refresh, and queue dispatch. The owner never fabricates successful persistence or changes queue policy.

## Risks/constraints for implementation owner

1. Avoid clearing a successor: `Converted` terminal handling can synchronously trigger import completion that dispatches a Run. Any reset must identify/retire the old invocation before that effect; the current protocol comment explicitly protects this ordering.
2. Preserve exact write-before-cancel order for Stop-and-Send, and no cancel/clear on checked commit failure.
3. Preserve failure/disconnect partial bytes through the checked interrupt effect before retirement; do not let stale terminal or an old identity affect a new invocation.
4. Preserve the special ordering and gating in `acknowledge_run`, queue `pending` ownership checks, fence comparisons, and pending-send priority over normal dispatch.
5. Keep non-Run Worker progress/control behavior. `progress` is shared by Import/Connect/Export/Backup and should not be silently moved into a Run-only type.
6. Do not make `RunIdentity` proof of persisted authority. Workspace/SQLite still validates run and queue effects.
7. Caller cost is moderate if only Run-only fields and accessors move (UI, chat, writing, projects, shell, plus direct test fixtures); it becomes high if `worker` itself moves, since many siblings and test modules construct/read all five worker kinds.

## Exclusions

No full Run policy migration, queue reducer, generic/app-wide reducer or event bus; no new crate/framework; no provider/protocol, persistence/schema, authority/permission, publication, native UX or prompt changes. This report intentionally does not inventory tests, per the assigned split.
