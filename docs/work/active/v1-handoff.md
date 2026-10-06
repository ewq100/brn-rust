# V1 controlled handoff — 2026-10-06

The full frozen V1 goal was confirmed **paused** through the goal tool on
2026-10-06 at the owner's controlled-stop request. Its complete objective is
preserved; V1 is incomplete. Further implementation starts only from an explicitly
selected task. This record is the closeout entry point for ordinary repository
access, independent of the originating agent or conversation.

## Immutable product checkpoint

- Repository: [ewq100/brn-rust](https://github.com/ewq100/brn-rust).
- Product branch: `codex/v1-docx-visual-sources`; clean local HEAD/main after normal
  [PR77](https://github.com/ewq100/brn-rust/pull/77) merge:
  `c75803832f3140347192bb08f2fdf13bb5fba1d4`, tree
  `f9a725ac08e48d040df48a76a94b969c3b4b8292`. The PR's pushed source head is
  `ae5d704b465eca174674a69b680e29e9f559ad7d`; local product branch includes the
  merge commit. No new product feature is underway.
- Completed: one ordinary inline PNG in a genuine DOCX, exact paired Source/asset
  approval, actual saved-image inspection through CLI/native AppWorker, explicit
  selected-model interpretation and separately reviewed/approved Source annotation,
  crash recovery and Undo. Binary originals remain retained; cleanup is refused.
- Automated: complete independent review clean; shared 1,608 passed/0 failed/16
  documented exclusions +52 fixtures; native 707/0/14 +52 fixtures; formatting,
  strict feature Clippy, shipping builds and V15 restarts passed. Fresh merged
  24-command gate passed 309/0/4 intentional child-entry exclusions +52 fixtures.
  Relevant Rust/manifests/lock/scripts/vendor are unchanged from the qualified
  source `b34a2897a63c1032c0398532e80648d9dceedf72`.
- PR CI: exact ae5 head, run `37454141784`, attempt1: all four protected
  macOS/shared checks and Docs passed; overall failure retains informational
  Windows compiler failures and meaningful raw differences. Merged-main run
  `37460415851`, attempt1 is terminal: four protected checks, Docs and supplemental
  Ubuntu UI SUCCESS; overall FAILURE preserves Windows/Linux results and raw differences.
- Pending: native GUI observation (Mac locked; owner explicitly deferred), live
  provider/real-model and owner acceptance, broader DOCX/PDF/PPTX/supplied-URL
  preservation, binary cleanup, later roadmap stages and trusted-user packaging.
  Generic ZIP ingestion is excluded; DOCX internally uses ZIP libraries.

## Environment and recoverability

Mac mini Darwin arm64, pinned Rust1.98.1 and locked dependencies; Rustup path
`/opt/homebrew/opt/rustup/bin`. Use a canonical current-user-owned TMPDIR and
explicit disposable synthetic data; separate Cargo targets for parallel checkouts.
This machine used `/private/tmp/brn-mini-synthetic-20261005` and separate lead/
native checkout targets. No new live calls, model downloads, authentication, private-data
operations, purchases or release are authorized.

Durable source and summarized evidence are in the repository's
[office plan](office-inbox/plan.md) and [checkpoint history](../../development/checkpoint.md).
Local full logs, hash manifests and synthetic GUI scenario are under the originating
task directory's `work/docx-visual-*-evidence`, `work/ci-pr77-*` and
`work/docx-visual-gui-fixture`; these are local supplementary evidence, not prerequisites
for a new agent. Unrelated worktrees, stashes, old review SHA7c4f668 and private data
were preserved. No task-owned uncommitted product code remains.

## Integrated closeout and preparation

PR78 is **merged** at `a8deb9d8e94665b1731034675b490fb134aae091`, checked remote main on2026-10-06. Actual head was `788d633e2fc79b11eb08eb55e3902c7d6e2cc6ae`. Exact-head run37464020406 and merged-main run37475432892 passed all four strict required checks and Documentation; main run is complete, overall red from informational Windows core/UI/retrieval and Ubuntu native retrieval. No new portability qualification or Rust run is claimed. The [preparation evidence](preparation-checkpoint/evidence.md) records policy, review and final publication state.

The existing product checkpoint and pending acceptance above remain unchanged.
PR78's prior pre-merge observations are preserved in Git history and its PR body;
they are superseded for continuation by this verified disposition. Preparation
adds [H1/H2 specs](preparation-checkpoint/next-specs.md), [remaining V1 map](preparation-checkpoint/v1-map.md),
[product glossary](../../product/glossary.md) and the proposed
[hybrid design](https://github.com/ewq100/product-to-production/blob/docs/hybrid-workflow-design/docs/hybrid-workflow.md).
No dependency replacement, evaluation, skill installation or product slice was run.
Preparation is pushed on `codex/preparation-checkpoint` as [PR79](https://github.com/ewq100/brn-rust/pull/79), OPEN/unmerged with its own final-head CI pending at publication; [evidence](preparation-checkpoint/evidence.md#publication-checkpoint) owns candidate/branch/qualification state.

GUI qualification is explicitly deferred by the owner, not a failed automated
gate. On a later unlocked Mac and explicit selection, recreate a synthetic genuine
DOCX→Source/PNG exact approval through the documented CLI or UI, inspect the saved
image/proofs and controls, restart, and retain safe observations using the
[verification guide](../../development/verification.md#optional-native-offline-qualification).
The local fixture/wrapper is supplementary and not required. No screenshot/body
rendering was obtained here. Live provider/real-model qualification separately
requires fresh owner authorization; prior live usage is exhausted. Do not bundle
live calls into GUI observation. Owner acceptance remains pending.

## Ordered task queue

Inspected unchanged product code at main a8deb9d: none of H1–H5 is completed or superseded. H1 still scans raw title lines; H2 still duplicates schema and has no direct Schemars dependency; visual output still uses prompt JSON without `output_schema`; no docx-rs/clap dependency is present. H3–H5 questions remain unresolved. Existing decision evidence is sufficient to prepare evaluations, not adopt replacements.

| ID | Kind / classification | Detail |
| --- | --- | --- |
| H1 | Ready for implementation | Settled title fix; [executable spec](preparation-checkpoint/next-specs.md#h1--titles-from-saved-markdown) |
| H2 | Ready for implementation with compatibility stop | Narrow accepted derivation, equivalence first; [spec](preparation-checkpoint/next-specs.md#h2--one-compatible-action-schema) |
| H3 | Ready for evaluation | Offline route feasibility; enabling live routes remains conditional on qualification |
| H4 | Ready for evaluation | Published release fit unresolved; broader replacement conditional on result |
| H5 | Evaluated: not adopted | clap would add more adapter code than it removes and still change error order; [result](../../../experiments/h5-clap-cli/README.md) |

Lead owns shared documentation and integration. Each implementer/evaluator records actual baseline, candidate, result and stop reason in its own task record; lead reconciles the common queue/ADR/status. Evaluations have no product API changes or owner acceptance requirement beyond reviewing the recommendation; no credentials/hardware beyond ordinary pinned Rust/macOS fixtures. H3–H5 finish with reproducible synthetic evidence and an adoption/non-adoption/blocker decision, independent review and focused result PR. H4/H5 isolate manifest, lock and target. Parent/spec links above do not select work.


**Ready** means technically ready for explicit task selection; it is not permission
to resume the paused roadmap automatically. Each task uses a separate `codex/`
branch/worktree, reads current Git status/HEAD and fetches origin without resetting
unrelated work. Common minimum: [AGENTS](../../../AGENTS.md),
[workflow](../../development/workflow.md), [verification](../../development/verification.md),
this handoff and its relevant section of the [reuse decision](../../architecture/decisions/2026-10-06-compatible-reuse.md).
Read [overview](../../architecture/overview.md) and
[invariants](../../architecture/invariants.md) for any boundary touched. No special
model, proprietary session state or local evidence cache is required.

For every task: record actual baseline, acceptance and command/result identity;
independently review meaningful implementation, validate/fix findings, qualify
latest-head applicable CI and normal merge requirements, and verify any merged
result. Update this task's state, affected contracts and status in its focused PR.
Evaluation completion may be a concrete non-adoption finding; it does not require
a replacement. Documentation-only results run `git diff --check` and
`python3 scripts/check-markdown-links.py`. Preserve the full V1 goal and pending
qualification. Only the selected task may proceed.

### H1 — reuse Markdown title parsing

- **Implementation; ready.** Outcome: library titles ignore fenced-code pseudo
  headings using existing mechanisms. No dependency on another queued task.
- Read `crates/brn-workflow/src/library.rs`, `tests/library.rs`,
  `src/knowledge/links/extract.rs`, `crates/brn-store/src/note_identity.rs`.
  Reuse Markdown1.0.0 AST and supported Store body boundaries, preserving the
  first50 saved-line/nonempty level1 title policy and filename fallback.
- Boundaries: titles/index metadata only; no note rewriting, scope/retrieval
  architecture, managed identity or frontmatter authority changes. Keep literal
  ATX title semantics unless a demonstrated defect requires a documented change.
- Acceptance: fenced fake title is skipped; real title/fallback, Unicode, BOM/CRLF,
  empty headings,50-line edge, complete `---`/`...`, malformed/unclosed and unmanaged
  note cases pass. Store-reader errors must not silently exclude previous inputs.
  Existing raw inline title spelling remains stable.
- Ordinary Rust/macOS filesystem fixtures; run focused library tests and strict
  affected Clippy/fmt per verification, then shared gate if indexing behavior changes.
  Complete with focused PR, title contract and task-state update; no live calls.

### H2 — derive one Action tool schema compatibly

- **Implementation; ready with compatibility gate.** Outcome: remove duplicated
  Action argument structure using already locked Schemars1.2.2.
- Read `crates/brn-ai/src/action_candidates.rs`, `proposal_tools.rs`, current schema/
  provider-format tests and pinned Schemars generation/Option code. Limit the first
  change to Action data/reference/enum structure; other tools migrate only when touched.
- Boundaries: retain tool descriptions, current capability selection, domain byte
  limits, required explicit nulls, identity/approval and workflow validation. No generic
  schema framework. A small typed wrapper/adjustment is acceptable; `required` alone
  is insufficient because it can remove nullability.
- Acceptance: all14 fields required including nullable ones; missing vs null differ;
  unknown/nested fields reject; enum tags/unions and provider request schemas preserve
  the accepted set. Synthetic route tests prove `$defs`/reference handling. If equivalence
  needs excessive adapters, stop with that evidence and retain manual sections.
- Ordinary pinned Rust, synthetic transports only. Run `cargo test -p brn-ai --lib
  --locked --offline`, affected fmt/Clippy and shared checks for a product change.
  Update the decision and task state through a focused reviewed PR.

### H3 — qualify Rig native visual structured output offline

- **Evaluation; ready.** Question: can pinned Rig0.43.0 Native output mode constrain
  description/uncertainty across BRN's existing three route shapes without changing
  the operation contract? H2 is not required; a raw small schema is sufficient.
- Read `crates/brn-ai/src/visual.rs`, `behavior.rs`, `chat.rs`,
  `provider_formats_tests.rs`, `crates/brn-workflow/src/inbox_actions/visual.rs`,
  vendored builder/output-mode and pinned ChatGPT/Copilot/OpenAI wire sources.
- Scope: isolated synthetic transport experiment and decision, not a new provider,
  prompt loader or full Rewrite migration. Preserve no visual tools, strict parse,
  known Stop completion,16KiB output cap, cancellation, one turn, zero retries,
  no provider/model fallback and separate exact human approval.
- Stop when ChatGPT Codex Responses, Copilot Responses and Copilot Chat request
  schema/stream/cancellation/refusal tests establish offline compatibility or one
  concrete blocker. Encoding alone does not establish live endpoint support; record
  that qualification separately and do not enable an unqualified route by assumption.
- Ordinary Rust/synthetic transport; AI library tests above plus scoped new witnesses.
  No live call or model download. Publish the finding and narrow recommended implementation
  boundary in a focused PR; implementation requires selecting that follow-up.

### H4 — evaluate published DOCX reader before broader Stage8

- **Evaluation; ready.** Question: can immutable docx-rs0.4.22 replace a meaningful
  part of OOXML interpretation with a small BRN admission/mapping adapter?
- Read the [office plan](office-inbox/plan.md), Store `src/work/inbox_source/docx/`
  package/document/image code and fixtures, Workflow DOCX tests, plus pinned published
  reader/package/image sources. Confirm release checksum/source identity before
  attributing floating-main findings to that release.
- Scope: throwaway isolated evaluator and recommendation; no product interfaces,
  golden-byte rewrite, format expansion, authority/refusal relaxation or upstream patch.
  Keep existing ZIP/XML/PNG libraries, original proof, resource caps, exact asset identity
  and approval/recovery/Undo responsibilities.
- Compare supported Unicode/list/link/table and exact PNG, meaningful header/footer/
  tracked/unknown content, budget/alias failures, two images/repeated relationship and
  an ordinary JPEG. Require preserved content/assets or explicit refusal, with bounded
  allocation/cancellation. Stop on first irrecoverable silent loss, unavailable original
  asset/occurrence, unbounded allocation or need for a second full interpreter; otherwise
  record measured adapter scope and the smallest next Stage8 deliverable.
- Ordinary Rust, synthetic DOCX only; evaluator has its own manifest/lock/target.
  No provider/model or private Office files. Result-only PR updates this decision,
  task state and next Stage8 acceptance; no automatic JPEG/PDF/PPTX implementation.

### H5 — evaluate replacing generic CLI scanning

- **Evaluated 2026-10-06 on `codex/h5-cli-parsing-evaluation`: not adopted.** The
  [evaluator record](../../../experiments/h5-clap-cli/README.md) and
  [decision](../../architecture/decisions/2026-10-06-compatible-reuse.md#a--cli-parsing-evaluated-not-adopted-h5)
  hold the matrix, costs and reopen conditions. No implementation follows.
  Merge state is recorded by the focused result PR.
- **Evaluation.** Question: does clap4.6.7 reduce total maintenance after
  preserving BRN's existing errors/help and command-validation order?
- Read `crates/brn/src/cli/mod.rs`, `input.rs`, one nested Inbox command module and
  CLI contract/tests. Compare a representative nested command and global options in
  a small isolated evaluator using the published Command API.
- Boundaries: generic scanning only; keep BRN command/business logic, JSON envelope,
  owner-operated authority and pre-workspace admission. No complete CLI migration,
  extra client permissions, new framework or global configuration change.
- Acceptance/stop: test exact `--help` precedence even with invalid/duplicate input,
  rejection of `--help=value`, late `--json` errors, duplicate/unknown/global/value
  behavior and no workspace access on invalid input. Record dependency/adapter/help
  maintenance cost and choose adoption boundary or concrete non-adoption reason.
- Ordinary Rust/synthetic argv, separate evaluator manifest/lock/target. CLI contract
  matrix plus documentation checks; no product-wide Cargo suite for a result-only PR.
  Publish the decision and task-state update, then stop before implementation.

## Parallel work and start prompt

H1, H4 and H5 have independent product/evaluator files and can run in separate worktrees after explicit selection. H4/H5 own result records only; lead owns shared documentation.
H2 and H3 share AI schema/provider tests and should run sequentially or on explicitly
fixed independent interfaces. All tasks share decision/handoff records: implementers return task-owned evidence; the lead reconciles shared documentation at integration. Keep Cargo
sequential per target. H4 informs the next Stage8 expansion; other maintenance tasks
are not new roadmap dependencies. Deferred GUI/live/owner qualification remains open.

> Continue BRN from `docs/work/active/v1-handoff.md`. Work only on explicitly
> selected task `<H1–H5>`. Inspect branch/HEAD/dirty files and fetch origin; preserve
> unrelated work, use an isolated codex branch and separate Cargo target. Read the
> listed minimum documents and task's acceptance. Follow BRN's frozen workflow and
> reuse decision, preserving AppWorker and deterministic approval authority. Implement
> or evaluate only that bounded task, record actual checks and independent review,
> update its state/decision and use one focused PR with exact-head applicable CI and
> normal merge requirements. Keep GUI/live/owner acceptance separate; no new live
> calls, model downloads, account actions, private-data operations or release. Stop
> after the selected task; do not resume the whole paused V1 roadmap automatically.
