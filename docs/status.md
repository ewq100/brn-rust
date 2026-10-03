# Current development status

2026-10-04. The owner has authorized sequential BRN v1 delivery under the frozen
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
is locally integrated. Stage 4 is not complete; native application controls remain pending.

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

Owned AI Rewrite is implemented on the existing chat lane with a narrow V6 job
in brn.sqlite. Full proposal/comments are captured transiently; validated output
and job outcome commit atomically against the captured stamp/hash. Later edits
settle Stale; Stop/disconnect/shutdown drain, and restart interrupts without retry.
Raw output/comments are not copied to chat/job history. Independent review found
two valid replay/schema gaps; fresh regressions reproduced and verified both fixes.
Fresh locked/offline gates passed **609 workspace tests, 0 failed, 2 ignored**,
**52 end-to-end assertions**, retirement, format/build/all-target Clippy and
optional native compile. Store **153**, AI **76**, CLI **93** passed. Owned Rewrite
is locally integrated under mission authorization; live Rewrite acceptance remains pending.

Explicit main reasoning effort is implemented and automated verified against
`5f26e66`. Fresh Ask requires low/medium/high, captured with provider/model in
paired V7 history. CLI and native Settings persist the choice locally; changes
affect new requests. Older unknown-effort history remains readable and replayable
without another provider call. Fresh macOS arm64 / Rust 1.98.1 locked/offline gates
passed **628 workspace tests, 0 failed, 2 ignored**, **52 end-to-end assertions**,
retirement, format/build/all-target Clippy and **93 optional native tests**, native
build/Clippy. Store **160**, AI **79**, CLI **97**, workflow **181** and default
desktop **80** passed. Independent review found no actionable defects. The slice
is locally integrated under mission authorization; native/live effort usability
and owner acceptance remain pending.

Native full proposal review/edit/comments and owned Rewrite controls are
implemented and automated verified against `5cd9c08`. Complete member/title
buffers recover through AppWorker after acknowledgement; late replies preserve
later typing. Failed comment drafts remain copyable and guard leaving. Independent
review found two valid defects—stale refresh failure correlation and single-line
title normalization; real regressions verify both corrections. Fresh macOS arm64 /
Rust 1.98.1 locked/offline gates passed **647 workspace tests, 0 failed, 2 ignored**,
**52 end-to-end assertions**, retirement, format/build/all-target Clippy and
**114 native tests** (including actual headless title-widget input and worker
restart), native build/Clippy. Independent checks passed **20 default / 22 native
review tests**. The slice is locally integrated under mission authorization; actual GUI/IME/accessibility
and owner acceptance remain pending. Native exact approval/group/activity/Undo/
repair and initial native/AI proposal creation are the next Stage 4 slices.

## Qualification still open

Owner headless/native acceptance remains pending and does not block later safe
implementation. Builds and synthetic crash/state tests do not establish native
usability, physical power-loss durability, other-volume support, actual model
inference or release readiness. Native Save/Copy/conflict/reload/recovery,
chooser, IME, accessibility, rendering and Stop/restart remain pending. Proposal
review has native controls; application/repair controls remain headless; mixed/interrupted application stays fenced
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
