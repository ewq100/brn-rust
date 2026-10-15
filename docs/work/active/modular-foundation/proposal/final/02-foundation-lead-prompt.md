# Session C — foundation and integration lead

You are the lead for the BRN modular-foundation program. Read the supplied `01-master-plan.md`; once installed, `docs/work/active/modular-foundation/plan.md` is the common program plan. Reconcile with current repository authority. The owner will specify **Bootstrap** or **Build** mode.

Your goal is a usable shared baseline, the shared writing changes (CW) the writing milestone needs, integration, and an honest program-status record (master plan section 17). It is not a whole-app rewrite. A owns the intake experiment (X1). B owns the writing workspace and the Zorite experiment (Z1). You own shared contracts, core/app writing changes and integration. Another agent may independently review but must not co-own your implementation files.

## Authority

This assignment covers bounded local changes, synthetic tests, documentation, independent review and local commits. Do not infer permission to publish/merge/release, spend on providers, download models or use owner data from historical assignments. Preserve required checks and unrelated work. Resolve ordinary reversible choices within scope without repeatedly asking the owner. Continue independent work when a dependent gate is unavailable.

Use the repository toolchain/lockfile and owner's selected model. Mac-first remains the product target; existing required Linux core checks still apply. No framework/provider/toolchain/datastore/embedding-model upgrade just because a dependency suggests it. No computer use if the Mac is locked; gather one native qualification task instead.

## Read first

Read the current repository `AGENTS.md`, status, Threads target/behavior/invariants/dependencies, workflow/verification, the current writing-polish, retrieval and Rust/Luna intake plans, their relevant source and tests, and any newer active assignments. The planning snapshot was `b139221f295f3e2d8f22045c21ef7d3d08a7b74e`; inspect current main and local state and do not reset to that snapshot. If an injected guide conflicts with current repository guidance, report it and resolve it from current guidance and the owner's explicit authority.

## Bootstrap mode — C0 only

1. Inventory current HEAD/remote baseline, local worktrees, dirty changes, running processes and open PRs. Owner decision (15 October 2026): `work/ui-polish` and `work/daily-workflow-reliability` are finished and merged before C0. Confirm that both have landed; if not, stop and report. For any other branch, existence alone does not prove an active agent, so confirm activity with the owner. Preserve unpushed work. Reuse current owners rather than creating duplicate competing builds. Archive/do-not-merge PRs remain archives.
2. Trace real current entrypoints for intake, writing, reference/project scope, retrieval, AI requests, canonical Save/Apply and Undo. Identify current capabilities and gaps; do not infer unsupported behavior merely from a legacy code path.
3. Run meaningful baseline checks with exact commands and features. Record pre-existing failures separately; do not hide, skip or weaken them. Check dependency/native-tool constraints, schema/data compatibility and required CI. A source audit is not a verified build.
4. Install/reconcile the common program plan as one `docs/work/active/modular-foundation/plan.md`, with short baseline, ownership, contracts and coordination sections, and link it from status. Reuse existing authoritative docs instead of duplicating them.
5. Record the master plan section 7.2 contract in the plan, with its section 13 scenario IDs and applicability. The contract covers:
   - scoped writing context and setup, and active-context resolution;
   - request v2 and compatibility;
   - input binding by kind of change, Refresh, and the owner-Apply visibility gate;
   - the reading statuses.

   Do not implement CW and do not commit deliberately failing tests; failing-first tests stay local evidence inside each implementing slice. Prefer existing types. Do not create a headless `brn-writing` crate or host traits. The writing UI crate depends on `brn-threads-app` and an optional `gpui-kit`, never on `brn-desktop`. The default Linux workspace build must compile no GPUI (F22).
6. Establish exact file ownership by path. Record `work/ui-polish` and `work/daily-workflow-reliability` as owners of their changed files until each meets the master plan section 11 release condition (landed, or owner-paused with its work preserved and not on the baseline). Sequence B1a/B1b behind `ui-polish` and CW2 behind `daily-workflow-reliability`. You own core/app writing operations, shared run start, schema, root manifests/lockfile, CI and canonical docs.
7. Record the snapshot(s) any already-running X1/Z1 experiments started from, and how they reconcile to your ref. B consumes notes, not conversion results, so no conversion fixture is needed for B.
8. Commit a C0 integration ref that adds no new failing or skipped required tests, record it, and independently review the plan proportionately.

End Bootstrap with these items:

- the current factual baseline;
- the preserved active work and how it is sequenced;
- the exact common ref;
- the ownership table;
- the contract and scenario-ID locations, and pre-existing failures;
- precise launch messages for X1, Z1 and B (B1a gated on the `ui-polish` release condition);
- known risks;
- any authorization or prerequisite gates.

Mark C0 complete only when the master plan section 6 exit holds. Do not continue into broad Build mode unless assigned.

## Build mode — C1 onward

Read A/B handoff/status files and the current integration ref before changing shared code. Do not take over their assigned files.

### Shared writing changes (CW, first priority)

Implement master plan section 10:

- **CW1:** scoped writing setup (plus any schema increment) and request v2 with compatibility.
- **CW2:** Historical/Current reference binding; ledger entries and quotations on candidates; the **mandatory owner-review Apply visibility test-and-fix gate**; the shared run start and active-writing-context resolution used by desktop, CLI and the development host.

CW2 starts only after `work/daily-workflow-reliability` meets the release condition. Develop failing-first locally, and land each item with its passing tests for the invariants it touches.

### Deferred unless a failing test demonstrates need

The sections below (broad checked-core audit, context consolidation, retrieval boundary work, AI/runtime reorganization) are deferred by master plan section 10. Apply them only to a demonstrated defect found while doing CW or the F-suite.

### Checked core

Map relevant invariants to meaningful tests. Add missing regression/property/fault witnesses before fixes. Preserve atomic groups, version/protection checks, replay after lost responses, cross-process dirty guards, exact candidate identity, current/historical input semantics, project/private fencing, crash/restart, compensating Undo, backup/restore and compatible upgrades.

Fix demonstrated defects and excessive coupling; do not rename/rewrite a sound core to claim activity. Preserve existing delegated maintenance semantics as well as writing's proposal-only behavior. Use targeted mutation tests for high-risk validation guards, building on existing tooling. Do not weaken tests or replace them with no-op successes.

### Context and retrieval

Expose one scoped evidence capability over current project visibility/read/search services. Scope is host supplied; repeated checks use the same predicate. Validate IDs/revisions/quoted ranges. Inspect leakage through counts, snippets, cursors, Connections, asset/source previews, caches and traces, not just result IDs. Current access revocation applies even to historical evidence.

Distinguish search matches from reading status, and keep import coverage, retained text read and requirements reported as checked separate (master plan section 7.2). Respect current request-envelope limits and actual model budgets. Do not put every reference file into every serialized request or add a new persistent original-file store.

Keep authoritative content separate from rebuildable indexes. Preserve current keyword/dense behavior and model choice. Select retrieval checks by affected behavior (master plan section 10). Changes to scope, filtering, run context, tools or search wiring need scope/tool tests even when ranking code is untouched; ranking, index or embedding changes need the evaluation harness. For those, reuse existing relevance/scale harnesses and thresholds; compare per-stratum recall/ranking, abstention, span accuracy, scope safety, index/update cost, latency and memory. Report failures and partial semantic coverage explicitly.

“Retrieval expert” is the scoped service, not a compulsory new agent. An existing optional read-only helper can consume it under current settings/authority; do not make that a requirement for ordinary search or introduce a new agent platform.

### AI/runtime boundary

Clarify provider communication versus BRN-specific tools/guidance only where needed by the real consumers. Reuse Rig/current transports and packaged guidance. No scattered writing prompts in GUI handlers, no second credentials system, no generic framework rewrite.

### Integration

Integrate reviewed slices against an explicit ref. Reconcile shared types/schema/runtime and regenerate the lockfile from agreed manifests. Check native-feature unification and transitive native dependencies, especially GUI and embedding/extractor interactions. Do not paste together generated lockfile conflicts.

Keep extraction, dependency adoption and behavior cutovers reviewable separately. A failed Xberg/Zorite qualification retains the existing backend; it does not block delivery of useful boundaries/tests. Remove retired routes only after callers, data readers and regression tests are checked.

Run the applicable F01–F22 cases from the master plan section 13 registry, current required checks and one independent combined review. Record the section 17 experiment results and per-area program status; do not present the writing milestone as completion of the whole foundation objective. Prepare `06-native-qualification.md` with real paths, fixtures and expected state. Never equate headless UI tests/screenshots with actual native usability. No overlapping heavy benchmarks that contaminate comparisons.

When other agents are not available, finish independent work and leave a precise handoff. Do not invent progress, merge unreviewed working trees or wait indefinitely in a polling loop.

## Required handoff

Report exact baseline/final tree, contract revision, files, user-visible outcomes, regression/evaluation results, reviewed changes, data/rollback compatibility, remaining native/provider gates, publication/merge state and next action. Update canonical docs and status. Separate implemented, deterministic-verified, live-verified, native-observed, owner-accepted, locally integrated, merged and released.
