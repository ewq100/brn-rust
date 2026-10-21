# Stop-and-Send continuation ownership design

Read-only follow-up, exact source checkout `46aebfaa6ff20d39a7afaae203b99a4b34b1ab12` in `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`. No source, shared-record, or Git changes; no checks run. The pre-existing campaign/overview modifications and untracked lead reports were preserved. This reuses the prior source map and inventories only the continuation paths requested.

## Current contract and paths

`ThreadsUi.pending_send: Option<PendingSend { thread, prompt }>` is logically armed for the current Run worker, but has no structural or runtime identity field. `can_stop_and_send()` ensures the slot is empty and the live Run identity matches the visible thread. `stop_and_send()` reads composer text, snapshots the current Run identity/cancel token, then calls checked `Workspace::stop_and_post_with_partial(key, run_id, thread, prompt, partial)`. Only on success does it set pending_send, clear saved/UI composer text, cancel that worker, and update progress/notice. On failure it leaves composer and worker intact. This is the durable commit → continuation arm → cancel order.

On `poll_worker`, `Worker::drain()` admits FIFO events through the first terminal. A `RunFinished` is acknowledged while the old Worker identity remains present. The `Finished` handler then records presentation, clears `self.worker`, stream, and progress. Only after the complete admitted batch is processed does the `Finished` batch-end branch `take()` pending_send and call `start_run(prompt, thread, ...)`; thus no replacement can consume old events. Queue dispatch is lower priority than pending Stop-and-Send. Do not move launch into event handling or before the batch boundary.

`threads_chat.rs::acknowledge_run()` reads `pending_send.is_none()` when no terminal receipt exists: if the producer reports RunFinished without checked terminal metadata, it pauses/recover-preserves the old run only when no committed continuation is awaiting its terminal. If present, Stop-and-Send already fenced the old run, and the saved instruction must remain available for the user; it must not trigger the competing failed-worker path. This check must move with the continuation accessor.

Every discard path is distinct:

- Explicit Cancel (`cancel`) takes/discards the continuation first and warns that the committed instruction remains in chat for manual Continue; it then performs checked stop/pause and cancels only on success. Existing behavior discards auto-launch even when that later stop/pause reports an error; preserve it deliberately.
- Archive (`resolve_thread`) performs checked archive first; on success it cancels the matching local Run and clears the continuation. On failure it retains both.
- Project-command fencing (`threads_projects.rs`) uses the checked receipt; only if the current Run was fenced does it cancel that worker and clear continuation. A nonmatching receipt does neither.
- Disconnect (`worker_disconnected`) first preserves/interupts the old Run with its visible partial while the old worker exists, then clears worker and continuation and reports failure. No auto-launch on disconnect.
- Window close and Quit intentionally discard auto-launch, then cancel worker/conversion; the durable prompt remains in chat.
- Terminal batch extracts and consumes the continuation once, only after old batch processing, then starts the saved request. There is no matching continuation for non-Run workers.

## Boundary choice

**Keep the existing field** is lowest coupling and already preserves the ordered behavior. It is a reasonable outcome if no invalid combination beyond `pending_send` being armed without a matching live Run can occur through production paths. The weak point is exactly that this invariant is conventional: six distinct effects must clear/take the free field correctly.

**Put continuation in `WorkerOperation::Run`** is the only minimal option that structurally ties the prompt to the existing `RunIdentity` without copying thread/run ID. Represent `Run { identity, edit_note_target, continuation: Option<RunContinuation> }`, where `RunContinuation` currently has only `StartSavedPrompt { prompt }`; thread comes only from `identity`. Add narrow methods on Worker: arm after the Workspace commit; discard/take; query `has_continuation` for the missing-terminal acknowledgment; terminal extraction from the old Run Worker. Constructors start empty. No second identity is stored or accepted from a caller. The host still owns every checked Workspace and GPUI effect. This removes `PendingSend` and `ThreadsUi.pending_send`, its initialization, and free-field reads/writes across stop/cancel/archive/project-fence/disconnect/close/quit/terminal/acknowledge. It also makes a continuation impossible for Import/Export/Backup/Connect workers.

This is a modest conceptual extension of Worker: its current documented responsibility is channel protocol plus invocation identity, and lifecycle/presentation/queue policy belongs to host/application code. A single *post-terminal disposition attached to the invocation* is cohesive enough if the type stays dumb: no queue policy, cancellation API, persistence, Run-state interpretation, notice text, or start effect lives in Worker. Keep event draining/channel admission unchanged. Keep the current new `RunActivity` owner separate; continuation is a dispatch disposition rather than activity presentation.

**Separate `RunContinuation` reducer/owner** can own Idle vs Armed(prompt), but without holding/borrowing the Worker it either stores a second `{thread,run_id}` (duplicate identity source) or trusts callers to mutate it in lockstep (same invalid combination, extra owner/interface). Storing a RunIdentity snapshot and checking equality is defensible but more code and identity duplication than the single Run variant field. A reducer holding Worker itself causes broad `worker` access migration across UI siblings. Recommend against this option.

## Narrow transition/API sketch

Keep `RunContinuation` as data inside Worker, with no independent terminal state machine:

- `arm_after_stop_and_send(prompt) -> Result<(), ContinuationAlreadyArmed>` is called on the current Run worker **only after** `stop_and_post_with_partial` returns success. It does not post, clear composer, or cancel.
- `discard_continuation() -> bool` returns whether the owner should show the existing “saved instruction, Continue when ready” warning. Use at the start of explicit Cancel; after successful Archive/project fence; during disconnect and close/Quit, ignoring the return where existing UX does not warn.
- `has_continuation() -> bool` replaces the `pending_send.is_none()` failure-recovery check in `acknowledge_run`.
- `take_terminal_continuation() -> Option<StartSavedPrompt>` is called by the terminal handler while the old Run worker is still accessible, retaining prompt locally through clearing/retiring old worker and the rest of the terminal batch. At batch end only, call `start_run(prompt, old_identity.thread, ...)`. Do not call before processing all batch events; do not let stale/no-terminal batches launch.

If the existing poll structure cannot capture this value before it sets `self.worker = None`, either extract it into a local within the terminal event branch before clearing, or defer clearing the old Run worker until the terminal batch-end handoff. The former is less invasive; the temporary local must not be exposed to another poll or queue dispatch. A failed/unconfirmed terminal still follows existing continuation priority only when the durable stop-and-post already succeeded; preserve the current behavior, and verify the acknowledgement path does not inadvertently pause the queue or lose the prompt.

## Risks and decision test

The main risk is boundary creep: adding pending prompt to Worker may look like permission to move RunActivity, stream, queue acknowledgement, or Stop policy there. Keep exactly one immutable payload and four small methods; all transitions with durable effects stay in ThreadsUi/Workspace. Another risk is future worker implementation changes that clear/replace Worker before the poller extracts the payload; make terminal extraction order explicit and preserve first-terminal FIFO boundary. Preserve all existing warning/error copy and composer draft ordering. Avoid adding thread/run id to the payload or another optional field on ThreadsUi.

Net recommendation: select WorkerOperation::Run as the continuation's storage owner **only if the lead values structural prevention of mismatch enough to accept this narrow transport-to-host disposition coupling**. This creates no second identity authority because RunIdentity remains the sole identity; it does not change durable authority. Otherwise retain the current field and document/assert the invariant, since a separate reducer does not improve ownership. Do not fold this follow-up into the independently authored RunActivity slice without lead direction; it is a later bounded candidate.
