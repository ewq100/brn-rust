# Session C — foundation and integration lead

You are the lead for the BRN modular-foundation program. Read the supplied `01-master-plan.md`; once installed, `docs/work/active/modular-foundation/plan.md` is the common program plan. Reconcile with current repository authority. The owner will specify **Bootstrap** or **Build** mode.

Your goal is a strong shared foundation and three compatible workstreams, not a whole-app rewrite. A owns intake. B owns the writing workspace/editor. You own shared contracts, checked-core/retrieval hardening and integration. Another agent may independently review but must not co-own your implementation files.

## Authority

This assignment covers bounded local changes, synthetic tests, documentation, independent review and local commits. Do not infer permission to publish/merge/release, spend on providers, download models or use owner data from historical assignments. Preserve required checks and unrelated work. Resolve ordinary reversible choices within scope without repeatedly asking the owner. Continue independent work when a dependent gate is unavailable.

Use the repository toolchain/lockfile and owner's selected model. Mac-first remains the product target; existing required Linux core checks still apply. No framework/provider/toolchain/datastore/embedding-model upgrade just because a dependency suggests it. No computer use if the Mac is locked; gather one native qualification task instead.

## Read first

Read `AGENTS.md`, status, Threads target/behavior/invariants/dependencies, workflow/verification, current writing-polish, retrieval and Rust/Luna intake plans, their relevant source and tests, and any newer active assignments. The planning snapshot was `b139221f295f3e2d8f22045c21ef7d3d08a7b74e`; inspect current main and local state, do not reset to that snapshot.

## Bootstrap mode — C0 only

1. Inventory current HEAD/remote baseline, local worktrees, active tasks/owners, dirty changes and open PRs. Preserve unpushed work. Reuse current owners rather than creating duplicate competing builds. Archive/do-not-merge PRs remain archives.
2. Trace real current entrypoints for intake, writing, reference/project scope, retrieval, AI requests, canonical Save/Apply and Undo. Identify current capabilities and gaps; do not infer unsupported behavior merely from a legacy code path.
3. Record meaningful baseline checks and known failures. Check dependency/native-tool constraints, schema/data compatibility and required CI. A source audit is not a verified build.
4. Install/reconcile the common program plan as one `docs/work/active/modular-foundation/plan.md`, with short baseline, ownership, contracts and coordination sections, and link it from status. Reuse existing authoritative docs instead of duplicating them.
5. Fix the master plan section 7.2 semantics as failing-first tests in `brn-threads-core`/`brn-threads-app`: writing-setup record, request v2 capture, `Historical` reference inputs with Apply-time visibility, and Sources used from the persisted read ledger. Prefer existing types. Do not create a headless `brn-writing` crate or host traits. The writing UI crate depends on `brn-threads-app` and an optional `gpui-kit`, never on `brn-desktop`. The default Linux workspace build must compile no GPUI (F22).
6. Establish exact file ownership by path. Record the active `work/ui-polish` and `work/daily-workflow-reliability` files as owned by those branches until they land or the owner pauses them, and sequence B1 and CW2 behind them. You own core/app writing ops, shared run start, schema, root manifests/lockfile, CI and canonical docs.
7. Give A and the Zorite spike `experiments/` locations with standalone manifests. B consumes notes, not conversion results, so no conversion fixture is needed for B.
8. Commit the C0 local baseline, record its ref, and independently review the plan and tests proportionately.

End Bootstrap with these items:

- the current factual baseline;
- the preserved active work and how it is sequenced;
- the exact common ref;
- the ownership table;
- the test and contract locations;
- precise launch messages for the Xberg spike, the Zorite spike and B;
- known risks;
- any authorization or prerequisite gates.

Mark C0 complete only when the master plan section 6 exit holds. Do not continue into broad Build mode unless assigned.

## Build mode — C1 onward

Read A/B handoff/status files and the current integration ref before changing shared code. Do not take over their assigned files.

### Shared writing changes (CW, first priority)

Implement master plan section 10 CW1 (setup record, schema increment, request v2), then CW2: candidate reference provenance and Sources used, plus the shared run start and writing-conversation selection used by desktop, CLI and the development host. CW2 starts only after `work/daily-workflow-reliability` has landed or been paused. Each item lands with regression tests for the invariants it touches.

### Deferred unless a failing test demonstrates need

The sections below (broad checked-core audit, context consolidation, retrieval boundary work, AI/runtime reorganization) are deferred by master plan section 10. Apply them only to a demonstrated defect found while doing CW or the F-suite.

### Checked core

Map relevant invariants to meaningful tests. Add missing regression/property/fault witnesses before fixes. Preserve atomic groups, version/protection checks, replay after lost responses, cross-process dirty guards, exact candidate identity, current/historical input semantics, project/private fencing, crash/restart, compensating Undo, backup/restore and compatible upgrades.

Fix demonstrated defects and excessive coupling; do not rename/rewrite a sound core to claim activity. Preserve existing delegated maintenance semantics as well as writing's proposal-only behavior. Use targeted mutation tests for high-risk validation guards, building on existing tooling. Do not weaken tests or replace them with no-op successes.

### Context and retrieval

Expose one scoped evidence capability over current project visibility/read/search services. Scope is host supplied; repeated checks use the same predicate. Validate IDs/revisions/quoted ranges. Inspect leakage through counts, snippets, cursors, Connections, asset/source previews, caches and traces, not just result IDs. Current access revocation applies even to historical evidence.

Distinguish search matches from exhaustive reference-reading coverage. Respect current request-envelope limits and actual model budgets. Do not put every reference file into every serialized request or add a new persistent original-file store.

Keep authoritative content separate from rebuildable indexes. Preserve current keyword/dense behavior and model choice during extraction. Reuse existing relevance/scale harnesses and thresholds; compare per-stratum recall/ranking, abstention, span accuracy, scope safety, index/update cost, latency and memory. Report failures and partial semantic coverage explicitly.

“Retrieval expert” is the scoped service, not a compulsory new agent. An existing optional read-only helper can consume it under current settings/authority; do not make that a requirement for ordinary search or introduce a new agent platform.

### AI/runtime boundary

Clarify provider communication versus BRN-specific tools/guidance only where needed by the real consumers. Reuse Rig/current transports and packaged guidance. No scattered writing prompts in GUI handlers, no second credentials system, no generic framework rewrite.

### Integration

Integrate reviewed slices against an explicit ref. Reconcile shared types/schema/runtime and regenerate the lockfile from agreed manifests. Check native-feature unification and transitive native dependencies, especially GUI and embedding/extractor interactions. Do not paste together generated lockfile conflicts.

Keep extraction, dependency adoption and behavior cutovers reviewable separately. A failed Xberg/Zorite qualification retains the existing backend; it does not block delivery of useful boundaries/tests. Remove retired routes only after callers, data readers and regression tests are checked.

Run shared F01–F20 acceptance cases, current required checks and one independent combined review. Prepare `06-native-qualification.md` with real paths, fixtures and expected state. Never equate headless UI tests/screenshots with actual native usability. No overlapping heavy benchmarks that contaminate comparisons.

When other agents are not available, finish independent work and leave a precise handoff. Do not invent progress, merge unreviewed working trees or wait indefinitely in a polling loop.

## Required handoff

Report exact baseline/final tree, contract revision, files, user-visible outcomes, regression/evaluation results, reviewed changes, data/rollback compatibility, remaining native/provider gates, publication/merge state and next action. Update canonical docs and status. Separate implemented, deterministic-verified, live-verified, native-observed, owner-accepted, locally integrated, merged and released.
