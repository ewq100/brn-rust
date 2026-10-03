# Current development status

2026-10-03, branch `task-4-ai-chat` (not merged or accepted).
The [simple Rig-based notes app](work/active/simple-rig-notes/plan.md) Steps 1–4
are implemented. Step 4 Tasks 1–7 were independently Opus-reviewed including
fix loops through `9c2f3ac`; Task 8 retirement/offline qualification awaits
controller review, then whole-branch review. Exact commits/checks/limitations
are in [evidence](work/active/simple-rig-notes/evidence.md).

- Both CLI and native simple consumers use AppWorker for saved-vault
  read/search, explicit account/provider/model actions, chat and local history.
  Production App Server/crate/configuration is removed; old executable flags
  are unknown, not ignored. Legacy local search/history/editor/drafts/comments/
  recovery remain supported separately.
- Native default is BRN-simple; credentials use saved explicit settings or its
  exact BRN-simple.credentials sibling. Old BRN requires explicit legacy mode
  or legacy markers. No original data inspection, migration or automatic login.
- Streaming is provisional. Durable endings and visibly unsaved persistence
  failures remain distinct; UUID replay never resubmits. New Ask refreshes current
  vault tools, freezes provider/model, and never falls back.
- Fresh consent is required for installation. Default builds are keyword-only;
  native installer/worker/state checks use synthetic assets/vectors, not ONNX.
- Simple notes are **saved-file readers only**. Step 5 proposal/approval tools
  and Step 6 simple Markdown Save/legacy cleanup are unimplemented.

## Qualification still open

Offline/default/native build and state evidence is not graphical usability,
real model inference, account validity or acceptance. ChatGPT remains
conditional after the historical quota-blocked spike; separate authorization
is required for live login/tool/stream/restart/refresh/Stop checks for both
providers. No accounts, downloads or original data were used in Task 8.
Upstream `block v0.1.6` retains a future-compiler warning.

Guarded close joins off GPUI. Legacy final quit synchronously joins already
admitted critical notes before the timed future; it does not flush unadmitted
typing. Simple Dock/system termination cannot veto exit or guarantee its
background drain within GPUI's deadline.

Human/native chooser, IME, accessibility, rendering, Stop/restart and legacy
Save/recovery acceptance remain pending. No merge, push, release or distribution
qualification is claimed. [Historical evidence](work/completed/README.md) and
the superseded Rig/provider plans remain evidence, not current instructions.
