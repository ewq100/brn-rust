# Request-history refresh preflight

## Checkout and scope

Read-only inspection of `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`, branch `codex/indexed-settings-read`, HEAD `eae61dde04cda19c764246993966b13b997014eb` (the assigned exact revision). The relevant paths had no diff from that revision; `git status --short` was empty at inspection. No source, Git, test, build, or fixture changes were made. I did not run tests or a workload.

The local overview renderer did not recognize task ID `request-history-refresh-preflight`; I followed the lead-selected task scope supplied for this assessment. This report makes no claim about the candidate’s current CI result.

## Finding and recommendation

One small offline characterization would add useful coverage of the history-reconstruction loop; it would establish behavior with accumulated ordinary history, not a latency target or full-startup/native performance result.

At `crates/brn-desktop/src/threads_ui.rs:835–871`, refresh clears `applied_rewrite`, forms the set of active unarchived writing-request Messages, reverse-scans every non-Undo history receipt with a prior Note revision not itself undone, and calls `store.prepared(receipt.operation)` before determining whether its inputs contain one of those requests. Matching inputs are revalidated through `Workspace::writing_request`; the saved Note identity and prior revision must match the receipt before the operation is associated. Thus, with at least one active request, older unrelated Note-update receipts still enter the prepared-operation lookup. The 10k-Note refresh diagnostic does not activate this branch because it has no writing request/apply pair.

Existing coverage is meaningful for correctness but modest for accumulation. `crates/brn-desktop/src/threads_writing_tests.rs:1419–1525`, `addressed_subset_and_restart_keep_actual_apply_undo_for_each_note`, creates two checked rewrite requests and actual Note-Apply receipts, adds a later request, calls refresh, verifies both operation identities survive reopen, then undoes one and verifies only that association clears. `crates/brn-desktop/src/threads_workbench_tests.rs:84–267` separately covers the offline UI path from saved request through review/apply/address/undo. The workbench fixture uses `configure_offline_fixture`, disables polling, and uses a fresh temporary workspace; it does not require a live model. `Workspace::snapshot_writing_request` is the checked request API (`crates/brn-threads-app/src/writing.rs:640–669`), and `Workspace::writing_request` validates captured canonical revisions (`:560–589`).

Smallest useful follow-up: extend the existing addressed-subset/restart test rather than add a framework. In its fresh fixture, create a fixed small batch (for example, four) of ordinary saved-Note update receipts through existing checked owner-write APIs, while retaining one active saved writing request; then perform one checked request-derived rewrite Apply and Undo, and assert refresh/reopen associates the exact Apply operation before Undo and removes only that association afterward. This activates the repeated `prepared` scan with accumulated eligible receipts while preserving the existing canonical version/provenance and no-auto-follow semantics. Keep it offline, use the existing fixture and proposal helpers, and report counts only. Do not infer a performance threshold from that count; any timing would be a separate diagnostic with explicit setup/timer boundaries.

## Qualification limits

This is source/test inspection only. I did not establish elapsed cost, benchmark refresh, inspect native behavior, or validate the proposed extra receipts experimentally. The existing 10k Notes sample measures a different refresh shape; the existing two-Note history test does not establish behavior for a large receipt set. No product defect or latency failure is demonstrated by this preflight.
