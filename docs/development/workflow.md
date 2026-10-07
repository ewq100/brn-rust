# Development workflow

> Current authority (2026-10-07): the [owner amendment](../product/BRN_PRODUCT_VISION.md#owner-amendment--2026-10-07) supersedes freeze, mandatory roadmap ordering and mechanism-preservation instructions below for this authorized whole-system reassessment. Those descriptions record the previous target/current contracts; they cannot prohibit investigation or proposed replacement. Product outcomes remain binding. Production implementation is paused; new design/sequence choices remain PROPOSED pending acceptance. The owner separately authorizes focused PR85 documentation finalization/merge, then stop, and a fresh agent's later [bounded Sol/Luna runtime campaign](../work/active/architecture-reassessment/plan.md#later-solluna-runtime-comparison). Neither selects production work or authorizes live calls in this finalization session.

Pick the next authorized roadmap outcome. Use the smallest necessary plan. Implement with meaningful tests. Get an independent review, validate its findings and fix real defects. Verify the final result, let the owner try changed user-visible behavior, and merge when authorized.

This is BRN's development method. [AGENTS.md](../../AGENTS.md) sets current task authority and product safety outcomes; [verification](verification.md) owns check selection. Skills are helpers, not additional mandatory lifecycles.

## Scope and planning

Inspect branch, HEAD and working-tree changes; preserve unrelated work. Use [status](../status.md) and [roadmap](../roadmap.md) when selecting an outcome, then read task-relevant contracts. Identify acceptance criteria and checks before editing. Existing authorization covers routine choices within that scope.

| Change | Planning needed |
| --- | --- |
| Small, bounded change | Short task/chat description: outcome, approach and checks. No new plan document or routine design-approval gate. |
| Multi-step feature slice | One short `docs/work/active/<task-name>/plan.md`: baseline, outcome/scope, contracts, dependencies/steps, risks, acceptance and checks. Fix interfaces before delegated implementation. |
| Changed requirement or concrete architectural blocker | Present evidence and involve the owner. Amend existing requirements/architecture after the decision; use a formal design record when warranted. |

A subsystem already covered by accepted product outcomes is ordinarily a feature slice. The authorized reassessment may challenge its mechanism. Plans describe decisions and work, without mandatory implementation/test bodies, two-minute steps or agent/model machinery. Self-check the plan; additional plan review needs a concrete risk or explicit request.

## Compatible reuse

Before specifying or implementing a meaningful new component, check existing
project code/shared abstractions, standard-library/platform facilities, pinned
dependency capabilities, suitable published companion modules and maintained
external libraries/tools. For agent development, check existing skills and workflow
components first. Ordinary small edits need no separate research report. Prefer compatible reuse when
it reduces overall complexity and maintenance. Add custom code only for an actual
unmet requirement or when a small local implementation is demonstrably simpler.
Keep the investigation proportional to the decision. Check actual APIs, published
versions, compatibility and required behavior; a similar name or advertised
capability does not establish fit. Record **reuse**, **adapt** or **build** in the
existing spec/task with a short reason and evidence. Custom code covers only the
specific unmet gap. Revisit the decision during implementation when discoveries
change the approach.

This applies to AI, CLI, parsing, storage utilities, import/conversion, UI
infrastructure and development tooling. For a material choice, record the
requirement, option/version, decision, remaining BRN responsibility and specific
reason for custom code or deferral. Count adapters, duplicated validation,
dependency/platform costs and verification, rather than lines alone. For general-purpose AI machinery, explicitly check pinned Rig (currently0.43.0)
and suitable published companion modules before custom machinery; add only what
the requirement needs. Start from current code and H1–H5 observations, testing their assumptions against owner outcomes. The 2026-10-07 amendment explicitly reopens their scope and conclusions. Material uncertainty gets a bounded evaluation
task; dependent implementation stays conditional on its result. The [historical reuse decision](../architecture/decisions/2026-10-06-compatible-reuse.md)
distinguishes implemented mechanisms, accepted follow-ups and unresolved candidates.

## Skills and helpers

Select a helper when explicitly requested or when it solves a concrete task need; invocation before every response/action is not required. Honor explicit requests within current scope and project constraints. Skills cannot add scope, external actions, approval gates or configuration changes on their own.

| Need | Helper and boundary |
| --- | --- |
| Meaningful behavior or regression | Behavioral TDD where appropriate. Use existing useful seams, including private synthetic lifecycle/recovery seams. Avoid tests mirroring implementation or trivial forwarding, and deleting correct work solely to recreate test-first ordering. |
| Bug/test failure | Evidence-first systematic debugging: reproduce where practical, inspect the path, test a hypothesis, verify the fix. Reserve Matt's fuller diagnosis for hard bugs; choose one method. Failed attempts prompt better investigation, not architecture reopening by count. |
| Review/feedback | Independent defect-first review plus accepted requirements and relevant Rust rules; technically validate feedback. Matt's two-axis review is manual-only; missing tracker setup is not a blocker. |
| Independent work/isolation | Bounded helpers or a worktree when useful. Reuse suitable worktrees; working in place is valid. Availability alone does not require delegation. |
| Specific unresolved question | Targeted research when useful. Prototype, grilling, domain/interface exploration and human-only wizards remain manual specialists. Architecture exploration is authorized for this reassessment. Use existing document locations; save research only when useful. |

`using-superpowers` is not BRN's router. Both installed `writing-plans` variants, competing execution chains and ticket/spec generators do not own default development. Use this workflow. Manual-only means explicitly requested, not inferred from overlap. Subagent-driven development is explicit opt-in and still follows these delegation/review rules. Workflow chains do not add implementation authorization. Skill discovery/maintenance and formal Superpowers incident analysis are manual tasks. Global skills/settings remain shared with other projects.

The [whole-system reassessment](../audits/BRN_ARCHITECTURE_REASSESSMENT_2026-10-07.md) and [proposed change plan](../work/active/architecture-reassessment/plan.md) are the current review/planning handoff. Production work remains paused.

One lead maintains coherence with the strongest appropriate selected model. Helpers receive scope, accepted criteria, relevant current owner outcomes and task permission boundaries, context and allowed actions; extracted task text or built-in reviewers may omit repository/global constraints. Helpers do not redesign adjacent work or add agents unless the lead assigns that work. Installed model tables do not override owner choices or current tool contracts.

## Independent review

For meaningful code changes, normally obtain one read-only independent review of the complete change; trivial documentation or wiring gets proportionate review. Additional specialists/checkpoints need a concrete risk. Report unavailable independence rather than presenting self-review as independent.

Pin the actual baseline and review staged, unstaged and relevant untracked changes. Branch review uses the verified merge base, not an assumed `HEAD~1`. Supply accepted criteria and current owner outcomes and task permission boundaries. Check correctness, missing requirements and safety. Speculative features/style are advisory; concrete valid-input failures remain defects even when the specification does not enumerate every trigger. The Rust checklist supplements review; verification supplies commands/features.

Understand and verify each actionable finding before fixing it. Fix valid findings, reject unsupported ones with a reason, and distinguish deferred work from blockers. Clarify only dependent work when a finding is unclear. Review does not expand scope. Re-review material corrections when risk warrants it, without fixed round counts or a blanket ban on re-review.

## Verification, acceptance and integration

Run relevant checks during implementation and after the last relevant change; read their output. Evidence identifies the tested tree/features and supports only that scope. Reuse evidence for unchanged relevant code; repeat checks for subsequent changes, failures or unresolved concerns, not once per skill.

For changed user-visible behavior, provide a reproducible scenario for the owner to try. Record acceptance, explicit deferral or pending qualification separately from automated checks. Builds/headless tests do not establish native usability; documentation needs an understandable result and documentation checks.

Integrate with current authorization for the target/action and applicable acceptance or explicit deferral. Carry out an already-authorized merge/push without another options menu; verify a changed merge result appropriately. Historical permissions do not authorize future live checks, release or migration. Stage task-owned files and use accurate attribution.

## Records and handoff

Development terms remain separate from product terms: **Defined** means accepted
behavior/requirements; **Build-ready** means a sufficient executable approach and
resolved material assumptions; **Candidate** means the actual implementation tree
awaiting or satisfying named gates; **Integrated** means the merged result is
confirmed with applicable post-merge evidence. A brief defines behavior; a parent
spec defines a bounded engineering outcome; tasks are independently verifiable
units within it. Verified names a specific check scope; accepted names the relevant
owner decision. Neither means merged. Readiness never selects a task.

For a lead transfer, commit/push the task, baseline/candidate identity, authorization
and exclusions, decisions, finding dispositions, gate evidence and exact next
action. The receiving lead rechecks remote/dirty state and accepts responsibility
for the selected outcome. Use repository documents across runtimes; runtime tools
and model availability do not change product authority. See the proposed
[hybrid workflow design](https://github.com/ewq100/product-to-production/blob/docs/hybrid-workflow-design/docs/hybrid-workflow.md).

Keep concise results in the task/PR description or plan. Use existing `evidence.md` records, or add one when observations need a durable handoff; every plan need not create a second file. Record baseline/tested tree, environment/features, commands/results/limitations, changed behavior, unresolved findings and next action. Keep critical observations durable; temporary paths alone are insufficient. Never record credentials.

Distinguish implemented, verified, accepted/deferred and merged. Close bounded task folders under `docs/work/completed/`, repairing links/indexes and carrying open gaps into status. Keep status concise and roadmap future-facing. Preserve historical evidence; update affected contracts rather than rewriting past observations as current results.

Before handoff, run `git diff --check`, validate local Markdown file/fragment links and moved-path references, and compare command examples with scripts/manifests. Documentation-only changes do not require rebuilding unchanged Rust code.
