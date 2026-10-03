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

Stage 4 review (`89421f5`), exact approval journals/fences (`e84ef7a`), whole-file
application/recovery (`e04fc52`) and readable activity (`7100ab2`) are integrated.
AppWorker/CLI support full editing, temporary comments, individual and captured-
group approval, exact Create/Replace/Trash installation and historical receipts.
Fresh pre-effect refusal preserves review work; unknown/partial effects stay
fenced. Replay/reconciliation never repeats installation. Ordinary receipts restore
older/missing operational databases before exposing current evidence; historical
completion preserves later bytes and editor typing. Applied cleanup removes covered
temporary annotations. [Stage 4 plan/evidence](work/active/proposal-core/plan.md)
retains qualification and reproducible manual scenarios.

Store Undo/Trash (`e593da8`) derives exact whole inverses or one original Trash
member, with immutable retained-original and scope bindings. Shared AppWorker/CLI
execution now preserves bytes, inodes/mode/ACL/xattrs and editor stamps, refuses
changed originals/targets/dirty aliases, and uses the same durable whole-operation
proof/recovery. Scoped Trash restoration preserves unrelated later edits. Activity
names Undo's source/scope. Independent review found no remaining actionable defect
and passed 4 workflow, 2 worker and 4 CLI process tests. Fresh root macOS arm64 /
Rust 1.98.1 locked/offline gates passed **538 workspace tests, 0 failed, 2 ignored**
private crash entry points exercised by subprocess matrices; **52 end-to-end
assertions**, retirement, format/build/all-target Clippy with warnings denied and
optional native compile. Store **127 passed**; CLI **77 passed**. Shared Undo/Trash
is locally integrated. Native review/Undo presentation and owned AI Rewrite remain
next; Stage 4 is not complete.

Store repair (`86439cf`) and shared AppWorker/CLI execution capture exact phases,
admit explicit Finish/Restore attempts replay-first and preserve bounded history,
first uncertainty and editor work. Review exposed a valid terminal staging-proof
gap; fresh regressions reproduced it and verified the correction. Independent
checks passed **13 Store repair, 7 shared, 2 worker and 5 CLI process tests**, plus
cleanup/correlation checks. Fresh locked/offline gates passed **567 workspace
tests, 0 failed, 2 ignored**, **52 end-to-end assertions**, retirement, format/build/
all-target Clippy and optional native compile. Store **140 passed**, CLI **83 passed**.
Shared repair is locally integrated; native repair interaction remains
pending. No published CI run exists for these local slices. The latest inspected
[published main CI](https://github.com/ewq100/brn-rust/actions/runs/37137393000)
at another commit (`609d859`) failed on Windows and optional Linux paths; its results
do not qualify this tree.

## Qualification still open

Owner headless/native acceptance remains pending and does not block later safe
implementation. Builds and synthetic crash/state tests do not establish native
usability, physical power-loss durability, other-volume support, actual model
inference or release readiness. Native Save/Copy/conflict/reload/recovery,
chooser, IME, accessibility, rendering and Stop/restart remain pending. Proposal
review/repair is currently headless; mixed/interrupted application stays fenced
until exact reconciliation or explicit repair. Native review/Undo/repair usability
remains pending.
The completed provider round leaves Copilot GPT-5.5 Chat unsupported, Codex vision
accuracy and native citations unqualified. Further live checks require new scope;
release/public distribution and original/private-data migration are unauthorized.
Upstream `block v0.1.6` retains a future-compiler warning.

Older implementation and verification counts remain in
[completed evidence](work/completed/README.md) and the
[earlier Rig notes evidence](work/active/simple-rig-notes/evidence.md). Historical
specifications, process assignments and permissions are supporting evidence.
