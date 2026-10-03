# Current development status

2026-10-03. The owner has authorized sequential BRN v1 delivery under the frozen
[product vision](product/BRN_PRODUCT_VISION.md), [architecture](architecture/overview.md#frozen-target),
[invariants](architecture/invariants.md), [roadmap](roadmap.md) and
[development workflow](development/workflow.md). Stage 4 Proposal Core is active;
Stages 5–16 remain pending. Complete BRN v1 delivery is not yet claimed.

Stages 1–3 are locally integrated. Simple manual Save/recovery (`6609442`) preserves
exact UTF-8 bytes, generation-bound recovery and file/parent/root identities.
Copies install exclusively; missing originals are not recreated. Native recovery
coalesces after 500 ms; acknowledgement establishes recoverability. Guarded
navigation/Quit drains admitted work; system termination can lose unacknowledged
typing. [Stage 1 evidence](work/completed/simple-save/plan.md) retains checks and
pending native acceptance. Stage 2 (`a5ec4ae`) removes legacy production paths,
`brn-core` and `brn-flow`; the six-crate workspace refuses old/mixed markers before
SQLite without migrating original data. [Stage 2 evidence](work/completed/legacy-removal/plan.md)
records the retired baseline and qualification.

Stage 3's scoped live round is complete (`d40e0a1`): both fresh human connections
succeeded, using **10 logical probes / 14 completion attempts** within the 11/22
cap. ChatGPT `gpt-5.5` Responses passed low/high read tools and the tiny image.
Copilot `gpt-5.3-codex` Responses passed read tools but reversed image colors.
Both observed hosted web search with correct official SQLite URLs; native citation
metadata remains unqualified. Copilot `gpt-5.5` Chat refused with
`unsupported_api_for_model`; production reports `ModelRefused` without fallback.
[Stage 3 evidence](work/completed/provider-capabilities/plan.md) records scope,
results and limits. No more account calls or model downloads are authorized by
that completed round.

Stage 4 review (`89421f5`) and exact approval journals/fences (`e84ef7a`) are
integrated. The current file-application slice implements shared AppWorker/CLI
individual and captured-group approval, all-member staging, coordinated exact
Create/Replace/Trash installation, required durability and whole-proposal proof.
A fresh pre-effect refusal preserves review work and external occupants; partial
or unknown work stays fenced. Replay never repeats file effects. Ordinary bounded
recovery receipts survive older/missing operational databases and are inspected
before current vault binding. Historical completion preserves later user bytes;
newer editor typing retains its old baseline/buffer as an explicit conflict.
Successful approval removes temporary annotations from live/prior journals,
ordinary snapshots and compatible proven temporary records. Independent review
findings were verified, fixed and re-reviewed. [Stage 4 plan/evidence](work/active/proposal-core/plan.md)
records acceptance scenarios and exact qualification. Fresh macOS arm64 / pinned
Rust 1.98.1 locked/offline verification passed: **498 workspace tests, 0 failed,
2 ignored** private crash entry points exercised by subprocess matrices; Store
**110 passed** including **34 approval/recovery tests**; CLI **67 passed** including
**9 proposal process tests**. Workspace format/build/all-target Clippy with warnings
denied, **52 end-to-end fixture assertions**, optional native desktop compile,
local Markdown links and diff checks passed. The file-application slice is locally
integrated at `e04fc52`.

The readable-activity slice (`7100ab2`) projects successful approval receipts into
AppWorker/CLI history with bounded exclusive paging, recorded approval time,
titles, affected paths and proposal/session context. It excludes note bodies,
comments and technical proofs. History remains stable after later note edits,
restart and ordinary-receipt restoration, including current-evidence fences and
unavailable vaults. Fresh root locked/offline verification passed **507 workspace
tests, 0 failed, 2 ignored**, workspace format/build/all-target Clippy with warnings
denied and **52 end-to-end assertions**; optional native compile passed. Native
history presentation and owner acceptance remain pending.

Bounded Store Undo/Trash derivation and admission are implemented: whole inverse
or one identified Trash member, exact retained-original binding, atomic review/
intent and self-contained receipt recovery. Legacy JSON/checksums remain compatible.
Independent review found metadata-cap failures on valid large inverses; reproducing
regressions and separate stable base/proof/manifest bounds fix them. Independent re-review found no remaining defect. Fresh root locked/offline gates
passed **524 workspace tests, 0 failed, 2 ignored**, Store **127 passed** including
**17 Undo tests**, **52 fixtures**, format/build/all-target Clippy and optional
native compile. Storage-only work is locally integrated. Workflow file execution, CLI/native
Undo, AI Rewrite and native proposal review remain next. Stage 4 is not complete.

## Qualification still open

Owner headless/native acceptance remains pending and does not block later safe
implementation. Builds and synthetic crash/state tests do not establish native
usability, physical power-loss durability, other-volume support, actual model
inference or release readiness. Native Save/Copy/conflict/reload/recovery,
chooser, IME, accessibility, rendering and Stop/restart remain pending. Proposal
review is currently headless; mixed/interrupted application stays fenced until
exact reconciliation proof, with further repair/Undo interaction still to come.
The completed provider round leaves Copilot GPT-5.5 Chat unsupported, Codex vision
accuracy and native citations unqualified. Further live checks require new scope;
release/public distribution and original/private-data migration are unauthorized.
Upstream `block v0.1.6` retains a future-compiler warning.

Older implementation and verification counts remain in
[completed evidence](work/completed/README.md) and the
[earlier Rig notes evidence](work/active/simple-rig-notes/evidence.md). Historical
specifications, process assignments and permissions are supporting evidence.
