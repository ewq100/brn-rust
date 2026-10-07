# Current development status

2026-10-06. **Stages 1–4 are implemented, automated verified and integrated.
Stage 5 knowledge and Stage 6 Actions/dashboard foundations are integrated,
with native/live/owner qualification still open.
Stage 7 text Inbox implementation is integrated with acceptance pending.
Stage 8 binary retention, ordinary asset proposals and bounded DOCX text/structure Source conversion are implemented, automated verified and integrated, including one ordinary inline PNG and separately approved provisional interpretation; broader visuals remain unfinished.
Stages 7–16 remain incomplete; complete V1
delivery is not claimed.**

The [Product Vision](product/BRN_PRODUCT_VISION.md),
[architecture](architecture/overview.md), [invariants](architecture/invariants.md),
[roadmap](roadmap.md) and [development workflow](development/workflow.md) govern
delivery. Markdown/assets remain durable authority; WorkStore operational state
and the disposable retrieval index retain their existing roles. The owner's
[client-boundary amendment](architecture/overview.md#client-and-protocol-boundary)
permits thin future adapters around the six V1 core crates through workflow/
AppWorker. No MCP, daemon, HTTP service, extra database or remote work is in V1.

Integrated product main is **`c75803832f3140347192bb08f2fdf13bb5fba1d4` (PR77)**,
tree `f9a725ac08e48d040df48a76a94b969c3b4b8292`. One ordinary inline DOCX PNG
now preserves an exact paired Source/asset proposal and supports complete saved-image
inspection through CLI/native AppWorker. Explicit selected-model interpretation
produces a tentative Source annotation through separate exact approval. Actual
annotation crash/Finish/Restore/Undo witnesses preserve immutable history and fresh-SQL
recovery without provider reexecution. Originals remain retained; binary cleanup is
refused. Generic ZIP ingestion is excluded. Broader DOCX/PDF/PPTX/suppliedURLs remain
unfinished; no whole-Office preservation claim is made.

Complete independent review is clean. Qualified unchanged source passed shared
1,608/0/16 documented exclusions+52 and native707/0/14+52, formatting/build/strict
feature Clippy, shipping builds, launcher and V15 restarts. Exact ae5 PR run37454141784
attempt1 passed four protected macOS/shared checks and Docs; overall CI is red from
Windows compiler failures with actual differences retained. Fresh merged24-command
gate passed309/0/4 intentional child-entry exclusions+52, native image widget/builds
and restarts. See the [office integration record](work/active/office-inbox/plan.md#pr77-integration-and-controlled-closeout--2026-10-06)
for exact CI, warnings, local evidence and pending acceptance.

The full V1 goal is **confirmed paused** at the owner's controlled closeout request,
with its full objective preserved. Documentation-only closeout records
[compatible reuse decisions](architecture/decisions/2026-10-06-compatible-reuse.md)
and an [explicitly selected task queue](work/active/v1-handoff.md#ordered-task-queue).
Native GUI observation is owner-deferred while the Mac is locked; live/real-model and
owner acceptance remain pending. No new product slice is underway. Further implementation
starts only from a selected handoff task; this pause does not change roadmap order.
Documentation closeout [PR78](https://github.com/ewq100/brn-rust/pull/78) is merged
at `a8deb9d8e94665b1731034675b490fb134aae091`, the remote main inspected in this
preparation pass. Exact-head and merged-main runs passed all four required checks
and Documentation. Main run37475432892 is complete, overall FAILURE from
informational Windows jobs and Ubuntu native retrieval; no pending job. Product
code remains PR77. The [preparation checkpoint](work/active/preparation-checkpoint/evidence.md)
records fresh baseline/policy checks; [next specs](work/active/preparation-checkpoint/next-specs.md)
and [remaining V1 map](work/active/preparation-checkpoint/v1-map.md) are planning only.
PR79 merged that preparation at `450eaa2`. The owner then selected handoff task H1
(library titles ignore code/HTML pseudo-headings). Its implementation, checks,
review, acceptance and integration state are in the [H1 record](work/active/h1-library-titles/evidence.md).
PR81 merged H1 at `f3cf699`. The owner then selected H2 (derive the `propose_actions`
schema from its Serde types, with no change to the emitted schema). Its state is in the
[H2 record](work/active/h2-action-schema/evidence.md). The owner also selected evaluation
H4. It recommends **non-adoption**: BRN keeps its own DOCX converter rather than
docx-rs0.4.22, rdocx0.15.0, office_oxide0.1.13 or betteroffice-docx-parse0.3.0
([findings](../experiments/docx-reader-eval/FINDINGS.md)). No product code changed; its
result PR is pending. The rest of the roadmap stays paused.

### Historical qualification

Earlier integration, CI and exact test-count observations remain in the
[checkpoint history](development/checkpoint.md) and relevant task records, including
[Office closeout](work/active/office-inbox/plan.md#pr77-integration-and-controlled-closeout--2026-10-06).
Their old next-step/active-goal statements are historical. This preparation reran
no Rust/product/native/live checks and claims no new owner acceptance.

## Integrated behavior and qualifications

| Area | Integrated behavior / remaining qualification | Durable evidence |
| --- | --- | --- |
| Stages1–2 | Exact manual Save/recovery and legacy removal; existing data preserved | [Save](work/completed/simple-save/plan.md), [removal](work/completed/legacy-removal/plan.md) |
| Stage3 | Selected-provider capability probes; no automatic fallback. Actual model/provider validity needs its own current qualification | [Provider round](work/completed/provider-capabilities/plan.md) |
| Stage4 | Typed whole proposals, exact approval/application, Activity, practical Markdown/asset Undo/Trash and Finish/Restore repair; Action-bearing Undo remains refused | [Proposal Core](work/completed/proposal-core/plan.md) |
| Stage5 | UUIDs/provenance, Current/Source/History/All, session timestamps, saved links/derived relationships and basic tentative Findings. Real multilingual/corpus/semantic quality remains unqualified | [Knowledge](work/active/knowledge-foundations/plan.md), [checkpoint](development/checkpoint.md) |
| Stage6 | Actions/dashboard, explicit Complete, related follow-up, full replacement evidence and recovery. Some synthetic native controls observed; composition/Rewrite/Ask live/native/owner acceptance remains pending | [Actions](work/completed/actions-dashboard/plan.md) |
| Stage7 | Text/email processing, independently reviewable consequences, supersession/conflicts, Source+confirmation recoverable original removal/Restore and native controls. GUI/live/owner acceptance pending | [Inbox](work/active/text-email-inbox/plan.md), [original copy](work/active/text-email-inbox/original-copy-plan.md) |
| Stage8 | Opaque binary capture, ordinary assets, bounded DOCX structure and one ordinary PNG with separate provisional annotation. Broader content/assets, supplied URLs and binary cleanup unfinished | [Office](work/active/office-inbox/plan.md) |

The [prior status snapshot](https://github.com/ewq100/brn-rust/blob/a8deb9d8e94665b1731034675b490fb134aae091/docs/status.md)
retains all earlier CI/count/continuation observations in their original context.
Its active-goal, pending-merge and next-step statements are superseded by this
checkpoint. No original data was inspected or migrated.

## Qualification and owner items

- Actual multilingual asset download needs the pending bounded owner permission.
  ONNX compatibility, EN↔ET quality, truncation, scoped restart/rebuild and tool/CLI
  parity remain open.
- The prior bounded Luna-only BRN app/provider qualification round used fresh human Connect,
  at most2 catalog calls+2 logical probes,18 completions maximum. Exact gpt-6-luna;
  no fallback, purchases or existing credential/private-vault inspection. After
  fresh sign-in and normal desktop quit, both permitted catalog calls succeeded,
  but the captured extraction established no usable IDs and did not retain raw
  catalog shape/length. Luna availability remains unverified;0logical probes/
  0completions were made. This round is exhausted; fresh owner permission is required
  for any new live call. Development/review may
  use Sol/Luna, neverAstra.
- Prior unlocked native checks passed synthetic scoped reads, Unicode Save and
  acknowledged buffer recovery after full quit/restart. Safe original UI captures
  are retained in the [screenshot index](ui/screenshots/2026-10-04/INDEX.md).
  Fixed Settings and its optional-model dialog were observed; three safe
  originals are retained. Download was declined without a model request.
  Human sign-in and updated-link/code acceptance passed on the unlocked Mac.
  ScreenCaptureKit intermittently prevents computer-use capture/interaction;
  safe screenshots are retained when available. Visual/native Action review, live inference,
  broader IME/accessibility/chooser and owner acceptance remain open.
- Pinned Rig stderr redaction correction is integrated through PR72; original
  pre-correction disclosure witness remains historical. Trusted-user packaging
  still needs its own artifact/log/privacy qualification.
- Synthetic crash/widget tests do not establish physical power-loss durability,
  other-volume support or release readiness. Windows Unix API and non-Mac native
  installer gaps remain visible. Upstream block0.1.6 has a future-compiler warning.
- Release/public distribution, additional live/download scope, purchases and
  original/private-data inspection/migration still require applicable permission.

Pending acceptance does not block later safe implementation when it is not a
dependency. No original data was inspected or migrated.

For continuation: preserve the chat/checkpoints and task-owned branches;
use Apple Silicon macOS/Command Line Tools, pinned Rust1.98.1, locked dependencies,
protobuf, Bash/Python3 and an explicit existing canonical owned TMPDIR. Native
interaction needs an unlocked, awake session; fresh human Connect and additional
live calls need fresh owner authorization. Optional native features and shipping builds need separate
checks. [Verification](development/verification.md) and
[setup](development/setup.md) contain reproducible commands.
