# Run preview/activity navigation audit

Scope: read-only source review at `bb064a8f37d1912a9c925071d606e0ade311bf9a` in `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`. Working tree had one pre-existing modification: `docs/work/active/vacation-campaign/evidence/pr179-integration.json`; left untouched. No source edits, builds, tests, provider/account/native calls, or Git mutations.

## Outcome

No consequential valid-sequence defect found in Run preview/activity ownership or presentation across chat/note navigation and worker completion. No correction recommended. This is a source audit; no checks were run.

## Evidence

- `crates/brn-desktop/src/threads_worker.rs:44-49,89-105,144-149` gives each Run worker immutable thread + Run identity, distinct from currently viewed chat. `:177-181` says first terminal event wins and host retires worker before successor dispatch.
- `crates/brn-desktop/src/threads_ui.rs:1968-1980` binds activity to the new Run's thread and clears old stream/activity at start. `:2587-2594` exposes stream only when requested thread matches worker identity. `:2687-2770` derives terminal outcome and notice from that worker's Run/thread, retains activity status, then drops the worker and clears live preview. `:2884-2892` only dispatches a successor after complete terminal batch; disconnected work follows its own interruption path.
- `crates/brn-desktop/src/threads_chat.rs:14-35` records bounded, deduplicated activity labels; `:48-108` renders live/retained activity. `crates/brn-desktop/src/threads_writing.rs:165-196` scopes activity presentation to selected chat ID; writing Runs are intentionally excluded from ordinary chat activity and handled by writing-specific presentation.
- Existing witnesses: `threads_focus_tests.rs:1170-1238` navigates away from A to B during a live Run, confirms A's preview is hidden on B and restored on A, and confirms resolving B does not cancel A. `threads_steering_tests.rs:452-498` checks thread navigation preserves Run identity, preview and drafts. `:519-699` checks activity labels, completion retention, unrelated worker disconnection and hiding activity on navigation to another thread. `threads_workbench_tests.rs:592-605` checks an ordinary chat Run does not surface as writing activity in the note.

## Residual witness note

The existing UI coverage exercises navigation for preview/identity and separately exercises retained activity and post-completion navigation, but does not appear to assert the detailed activity pane itself follows A → B → A while the Run remains live. The identity/thread gates are direct and separately witnessed, so I do not consider this omission consequential enough to recommend a new test within this bounded audit. Assumption: retaining a single most-recent Run's activity until another Run begins is intended; source starts with one global bounded label list and rebinds it on every Run start.
