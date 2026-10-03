# Current development status

2026-10-03. Stage 1 is locally integrated at `6609442`, based on frozen-guidance
baseline `4bf7878`. Stage 2 is locally integrated at `a5ec4ae`. Stage 3 offline
probe preparation is integrated at `7e63041`; the authorized live round has
finished, with the limitations below. Stage 4 Proposal Core is active.

The owner has authorized sequential BRN v1 delivery using the [product vision](product/BRN_PRODUCT_VISION.md),
[target architecture](architecture/overview.md#frozen-target), [invariants](architecture/invariants.md),
[roadmap](roadmap.md) and [development workflow](development/workflow.md).

Stage 1 simple manual Save/recovery is implemented and automated verification
has passed. Final independent read-only review returned no findings. Owner
native acceptance remains pending. The [slice record](work/completed/simple-save/plan.md)
contains reproducible acceptance steps and verification detail.

- Simple CLI and native editing use AppWorker. Explicit Save/Cmd-S preserves exact
  UTF-8 bytes, detects changed file/root/parent identity and never recreates a
  missing original. Save Copy uses an unused destination without overwriting or
  resolving the original editor. Compare and confirmed reload remain local.
- WorkStore’s V3 editor schema holds generation-checked recovery, save intent/proof and receipts.
  Replay never repeats file writes. Reconciliation requires identity proof;
  unresolved saves fence current search/AI. One recent Applied recovery pair and
  compact settled receipts remain after proven artifact cleanup.
- Native recovery coalesces after 500 ms; only acknowledgement establishes
  recoverability. Guarded note-switch/close/Quit waits for the latest buffer and
  admitted work. Failed recovery retains text for explicit retry. Dock/system
  termination can lose unacknowledged typing.
- Native defaults to BRN-simple with its protected credential sibling. Chat,
  local history, explicit provider/model selection and saved-vault retrieval
  remain implemented. Default builds are keyword-only; model installation needs
  fresh consent. Streaming/finalization and UUID replay retain their existing
  safety distinctions.
- Stage 2 removes legacy Store/Workspace/worker/CLI/native paths, `brn-core` and
  `brn-flow`. The workspace now has six crates and two binaries. WorkStore and
  current Save/recovery/search/chat remain; old/mixed markers refuse before
  SQLite opens. Existing data, standalone trials and historical records remain
  untouched. Whole-proposal application and later v1 stages remain pending.

Fresh Stage 1 verification: workspace **728 passed, 0 failed, 1 ignored**;
native desktop **137 unit + 6 CLI tests passed**; native workflow **93 library +
5 model tests passed, 1 ignored**. Native desktop/CLI builds, native Clippy,
**47 fixture assertions** and launcher checks passed. These checks use synthetic
fixtures; the ignored tests and other limits are recorded in the slice record.

Fresh Stage 2 verification: workspace **391 passed, 0 failed, 1 ignored**;
native desktop **84 unit + 7 CLI passed**; native workflow **67 passed, 1 ignored**;
native retrieval **42 passed**. Default format/build/Clippy, native desktop build/
Clippy, **52 fixture assertions** and launcher checks passed. Independent read-only
review found no actionable defects. [Stage 2 record](work/completed/legacy-removal/plan.md)
distinguishes synthetic checks from remaining native/live qualification.

Stage 3 implements opt-in synthetic probes and bounded safe diagnostics. Both
fresh human connections succeeded. ChatGPT `gpt-5.5` Responses passed low/high
read tools and the fixed image; Copilot `gpt-5.3-codex` Responses passed low/high
read tools but reversed image colors. Both observed hosted web search and returned
correct official SQLite release/source URLs; neither returned native citation
annotations, so metadata attribution remains unqualified. Copilot `gpt-5.5`
Chat refused with HTTP 400 `unsupported_api_for_model`; its remaining image
probe was skipped. Safe production mapping reports `ModelRefused` for this code,
without retry or route/model fallback. **10 logical probes / 14 completion
attempts** ran within the authorized 11/22 cap. No original data or credentials
were inspected. [Stage 3 results](work/completed/provider-capabilities/plan.md)
record exact models, scope, verification and downstream constraints. Final
independent review found no actionable defects. Fresh workspace **404 passed,
0 failed, 1 ignored**; feature AI **68 + 1 example passed**. Default and feature
build/Clippy, format and documentation checks passed.
Stage 4 review foundation is implemented under [its plan](work/active/proposal-core/plan.md):
typed Markdown drafts, version-bound full edits/comments/rejection, uncertain
anchors, stale Rewrite-result protection and shared AppWorker/CLI access. It
never writes knowledge. Independent review found no actionable defects; fresh
workspace **423 passed, 0 failed, 1 ignored**, build/Clippy/format, optional native
check and **52 existing fixture assertions** passed. Owner headless acceptance
remains pending. Whole-proposal approval/application, activity/Undo/Trash, AI
Rewrite and native review are the next Stage 4 slices. Stages 5–16 remain pending; no complete BRN v1 delivery is claimed.
Owner acceptance may remain pending when it is not a dependency for later safe
implementation.

## Qualification still open

Builds and synthetic state/crash tests do not establish graphical usability,
power-loss durability, other-volume support, actual model inference, account
validity or release readiness. Native Save/Copy/conflict/reload/recovery,
chooser, IME, accessibility, rendering and Stop/restart acceptance remain pending.
Additional live checks beyond the completed bounded scope require new
authorization. Copilot GPT-5.5 is unsupported by the pinned Chat route; Codex
vision accuracy and native citation metadata remain unqualified. No release or distribution
qualification is claimed. Upstream `block v0.1.6` retains a future-compiler warning.

Earlier implementation/check details remain in the [simple Rig notes evidence](work/active/simple-rig-notes/evidence.md)
and [historical evidence](work/completed/README.md). Those records are supporting
history, not current execution plans or standing account permissions.
