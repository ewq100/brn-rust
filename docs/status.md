# Current development status

2026-10-03. Stage 1 is locally integrated at `6609442`, based on frozen-guidance
baseline `4bf7878`. Stage 2 is locally integrated at `a5ec4ae`. Stage 3 offline
probe preparation is locally integrated at `7e63041`; actual
provider qualification remains open.

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
- WorkStore V3 holds generation-checked recovery, save intent/proof and receipts.
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
  untouched. Proposal Core and later v1 stages remain unimplemented.

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

Stage 3 has a feature-gated synthetic capability probe, exact model/route/effort
checks, fixed PNG input and bounded native web/citation projection. Ordinary chat
is unchanged. Independent review defects in cancellation error priority and
terminal-only web evidence were reproduced, fixed and regression verified;
final review found no remaining actionable findings. Fresh workspace tests:
**403 passed, 0 failed, 1 ignored**; feature AI tests **67 + 1 example passed**.
Default build/Clippy/format, feature example build/Clippy and documentation checks
passed. [Stage 3 plan/results](work/active/provider-capabilities/plan.md) specifies
the bounded live scope and reproducible commands. The owner authorized that
scope on 2026-10-03. Fresh Copilot device login reached GitHub's confirmation,
but its embedded-browser authorization control stayed disabled and the CLI
deadline stopped/joined the attempt. **Zero model probes ran**; no original
credentials/data were inspected. Live permission remains granted; human sign-in
in a regular browser is needed before advancing the provider dependency.
Stages 4–16 remain pending; no complete BRN v1 delivery is claimed.
Owner acceptance may remain pending when it is not a dependency for later safe
implementation.

## Qualification still open

Builds and synthetic state/crash tests do not establish graphical usability,
power-loss durability, other-volume support, actual model inference, account
validity or release readiness. Native Save/Copy/conflict/reload/recovery,
chooser, IME, accessibility, rendering and Stop/restart acceptance remain pending.
Live provider capability checks require separate authorization; ChatGPT remains
conditional after its historical quota-blocked spike. No release or distribution
qualification is claimed. Upstream `block v0.1.6` retains a future-compiler warning.

Earlier implementation/check details remain in the [simple Rig notes evidence](work/active/simple-rig-notes/evidence.md)
and [historical evidence](work/completed/README.md). Those records are supporting
history, not current execution plans or standing account permissions.
