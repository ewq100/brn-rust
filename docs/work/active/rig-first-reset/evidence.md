# Rig-first reset evidence and handoff

Date: 2026-10-01.

## Authorization and state

| Scope | State |
| --- | --- |
| Written reset specification | User-approved; committed at `ecc0c97`, approval recorded at `19f180c`. |
| Implementation plan | Initial pack at `347fd38`, review corrections at `db18da8`; user approved the revised plan on 2026-10-01. |
| Product implementation | Not started; the subsequent execution-choice prompt was skipped, so execution confirmation remains pending. |
| Dependency/provider/retrieval qualification | Not run; inspected source is not a compiled or live result. |
| Live login/calls, model acquisition, signing, release | Not authorized or performed. |

Planning baseline: `19f180c80cd607b47f22473841971ffc5065ac91`, branch `evokessler-ericcp-rig-architecture-reset`. Baseline Rust/store/retrieval implementation remains unchanged.

## Planning inspection

Repository evidence used for [master plan](plan.md) and subsystem plans:

- Store schema `5`, one exclusive SQLite owner, `bind_operation` payload matching and startup interrupted-operation recovery.
- Workflow's existing Codex `ask_full`, error facts, bounded worker queue, draft/comment/generation conventions and 1 MiB/20,000-byte bounds.
- Current retrieval `Document`/`Evidence`/`Index`, public hash helper, substring keyword scoring, existing RRF, generation validation and local native resource conditions.
- CLI argument/JSON/signal/output behavior, desktop Config/native shell consumers and unsigned launcher assumptions.
- Separate experiment manifests/lockfiles; root default/native feature boundaries and the repository verification/development workflow.

The master plan was initially written under the renamed-branch directory instead of the actual session worktree. Before continuation, `pwd`/`git status` confirmed the actual worktree remained `evokessler-ericcp-laughing-eureka`; the created file was moved into that checkout before other plan files were added. The accidentally created empty directory chain was removed with explicit nonrecursive `rmdir` paths. No existing file from the main checkout was read or modified.

Pinned Rig source inspected for the specification, then relevant auth/dialect source rechecked during planning:

- Facade package `rig`, candidate `0.43.0`; companion SQLite/FastEmbed graph differs from BRN's pins.
- Both subscription dialects expose `PROVIDER_NAME`; ChatGPT config uses its `DIALECT`, Copilot uses `CopilotConfig::from_auth`.
- DeviceCodeHandler accepts a synchronous thread-safe callback; native authenticators use caller-supplied paths and explicit device-flow policy.
- Copilot auth context does not itself supply stable account identity; Q2 must qualify an explicit authenticated identity mechanism.
- Rig config/history/hook/transport integration still requires Q1 compile evidence. No upstream source excerpt is treated as a resolved downstream graph.

Primary source links and source-level limitations are in the [specification](../../../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md#12-primary-source-references).

## Qualification gates

| Gate | Current result | Evidence needed |
| --- | --- | --- |
| G1a candidate dependency/API | Not run | Published manifests, candidate lockfile, compiled interfaces and linked native probe. |
| G1b actual production native graph | Not run | A1 actual root-locked desktop builds/tests/tree with Rig, repeated after dependency changes. |
| G2 both subscriptions | Not run; live phase unauthorized | Deterministic auth/agent matrix and separately authorized selected-account live results. |
| G3 current notes | Not implemented/not run | Exact-save/recovery/currentness tests and disposable native observations. |
| G4 retrieval | Not implemented/not measured | Frozen baseline/corpus/qrels, SQLite vector measurements, explicit acceptance. |
| G5 distribution | Blocked on all prerequisites | Terms/support-risk review, declared target, signed/notarized clean-Mac evidence and both direct providers. |

Expected red/green commands inside task plans are future instructions, not performed tests. No product build, Cargo resolution, model download, original-vault access or account qualification was performed for plan preparation.

## Documentation validation

Self-review mapped all specification sections to Q1-Q3, N1-N5, A1-A4 and D1-D2. Corrections made during review:

- Kept optional companion probes out of the selected dependency graph; native resolution remains an actual gate.
- Separated editor baseline from fresh disk state and copy receipts from original-file save acknowledgements.
- Placed durable provider selection and explicit note/draft review associations in V7; no invented association from matching old draft text.
- Kept store neutral DTOs free of Rig/retrieval crate dependencies and separated review provenance from current evidence.
- Named debug-only replay seams/feature selectors and kept them out of release builds.
- Preserved the candidate byte limit on final output rather than an intermediate reverse-edit state.

Initial plan validation at `347fd38` passed: `git diff --check` and a read-only Python local-link/heading-fragment, placeholder and task-ID check over all 10 changed Markdown files. That check covered 88 local links/fragments, 14 unique task IDs and 90 checkbox steps, with zero errors. These counts describe the initial pack, not fresh validation of this revision.

These checks validate documentation structure/coherence, not product feasibility, compiled future APIs, native usability or provider qualification. Commands/feature selectors were compared with existing manifests/scripts; intentionally new tests/features/files are explicitly produced by their task.

## User-requested review corrections

Review baseline: `347fd3891e8b6ddc74d15d7470f735b5a3ba8358`. The user requested GPT-6.1 Sol at high reasoning, asked whether the findings were sound, then directed the plan update. The author checked the cited contracts/source and accepted all five findings plus the diagram mismatch.

| Finding | Plan correction and required future evidence |
| --- | --- |
| Listing-only currentness | A2 note-state/metadata dependencies with optional passage evidence; A3 listing-only empty-note archive/withdrawal/restart/windowed-history tests. |
| Incomplete replay binding | A2/A3 bind every ask input, including vault/profile, before existing-result replay; changed-scope conflict and zero-extra-dispatch tests. |
| Proxy native evidence | Q1 linked native probe closes only G1a; actual root-locked desktop integration in A1 closes G1b. D1 requires the post-N5 integrated graph. |
| N2 buildability | Minimal typed CLI codes/message mappings and exhaustive consumers move into N2, with a named unit selector; richer presentation stays in N4. |
| Missing baseline interface | N2 `NoteView.baseline` exposes exact retained bytes tied to the editor stamp; restart comparison and N4 consumers distinguish baseline/working/disk. |
| Dependency mismatch | Diagram/text consistently require N1 for A2; N3 remains an A3 prerequisite. |

Revision validation passed: `git diff --check` and the read-only documentation checker covered all seven revised Markdown files, 46 local links/fragments, 14 task IDs and 92 checkbox steps with zero errors. Structural checks confirmed all five corrections, the A2 prerequisite and documentation-only scope. Future test selectors and G1a/G1b commands remain unexecuted implementation instructions. No Rust/manifests/lockfiles were changed and no product qualification was performed.

## Execution handoff

The user approved the revised implementation plan at `db18da8c482a173e666ec23b9d6f00ca4ddd05c3` on 2026-10-01. No execution option was selected in the subsequent prompt. This records plan acceptance, not implementation results or permission for live/account/resource/release actions.

Start with [Q1](qualification.md#q1-resolve-the-actual-dependency-graph-and-pin-usable-interfaces) only after execution authorization. Treat exact published/native graph/API findings as a mandatory decision gate before downstream implementation.

Inline execution with checkpoints is the default; per-task delegation requires explicit user selection. Do not execute the older paused Markdown save plan. Respect separate authorization for live calls, models, credentials, signing, release and real data.
