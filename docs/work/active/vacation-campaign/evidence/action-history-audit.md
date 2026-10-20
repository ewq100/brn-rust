# Bounded Action/history Undo audit

## Result

One actionable missing behavioral witness: the desktop Action journey never activates **Confirm done**. The row constructs completion evidence and routes its main button through the checked change path, but existing desktop coverage only confirms the button is visible. The exercised lifecycle transitions are Accept/Open, Waiting/Mark open, and Cancelled. This leaves the most consequential Action transition unverified at the UI-to-Store boundary.

## Source and evidence

- In `crates/brn-desktop/src/threads_tiles.rs:950-957`, `action_row` maps `ConfirmDone` to `ActionState::Done` and sets `evidence` to `"Owner confirmed completion in BRN."`; `:972-989` binds the rendered control to `this.change(...)` with the displayed record and proposed data.
- `crates/brn-desktop/src/threads_pages_tests.rs:587-698` checks available state controls and invokes Mark open, Accept task, and Cancel task. It does not click `done-action-*`.
- `crates/brn-desktop/src/threads_tiled_tests.rs:293-403` confirms an Action status change creates one receipt, displays its exact operation in Changes, and invokes that receipt’s Undo, restoring the prior data with a new version and `undo_of` link. Its exercised change is Open → Waiting; Confirm done is only checked for visibility at `:347-353`.
- The shared Store already rejects Done with missing/blank evidence at `crates/brn-threads-core/src/store.rs:1592-1600`, and the human completion capability is required at `:2255-2262`. `crates/brn-threads-core/tests/core.rs:1189-1234` covers absent evidence rejection and insufficient authorization. Those checks are meaningful existing guarantees and need not be duplicated in UI state machinery.
- Receipt-specific Undo in `crates/brn-desktop/src/threads_tiles.rs:1159-1238` captures the displayed receipt operation ID in the button callback; `undo_receipt` passes that ID to `workspace.undo` (`crates/brn-desktop/src/threads_ui.rs:2432-2442`). Core `Store::undo` builds compensating writes with `expected_version: written.after.version` (`crates/brn-threads-core/src/store.rs:670-712`); generic stale-write preservation is exercised by `crates/brn-threads-core/tests/core.rs:355-427`. The tiled test also verifies exact receipt binding and successful compensation for an Action status write. Undo is synchronous in this callback, so no asynchronous navigation race is apparent in the inspected route.

## Consequence / bounded recommendation

A regression in the Confirm done row mapping, its completion evidence, or the receipt subsequently shown in Changes could pass current Action UI tests. A focused desktop witness should activate the visible `done-action-{id}` for a synthetic Open Action, assert one checked receipt, `Done` plus the evidence on the current Action, and Undo that specific completion receipt to restore the original Action at a new version. Existing Store tests remain the authority for malformed evidence/capability rejection; no new UI state or framework is indicated.

## Scope notes

Inspected Action row/state mapping, desktop Action and history journeys, shared Store validation/Undo, and relevant invariants. No builds, tests, native app, provider, account, data, or source/repository mutations were performed. The requested checkout was on branch `codex/agent-first-followthrough` at `f671bdab8962b19e98d204ae6bc612ea200034d4`, not the supplied `main20fcd7383662cd2f67607c3ce04194aed50809b5`; working tree was clean. Findings describe that inspected checkout.
