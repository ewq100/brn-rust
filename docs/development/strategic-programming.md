# Strategic programming

This guide expands [AGENTS.md](../../AGENTS.md) and the [workflow](workflow.md). It is standing engineering guidance, not a feature assignment or new permission. The [canonical target](../architecture/threads-target.md) and [invariants](../architecture/invariants.md) continue to own BRN's behavior.

## Goal

Deliver the requested behavior while keeping subsequent changes understandable and safe. A useful design improvement removes something a caller must know, a policy that must be changed in several places, an invalid state that must be repaired, or a production route that must be maintained twice.

Invest during ordinary work when there is a demonstrated benefit. Do not wait for a separate cleanup campaign to fix a small design problem directly caused or exposed by the assignment. Equally, do not expand every bug fix into a rewrite. State a concrete follow-up when the necessary work exceeds the assigned scope or another agent owns the boundary.

## Choosing agent behavior or application code

Classify a material capability by the owner outcome and assign each responsibility:

| Responsibility | Default owner |
|---|---|
| Interpretation, organization, retrieval strategy, planning, drafting and semantic conflict analysis | Existing agent with concise skills/examples |
| Exact calculation, parsing, document conversion and editing | Maintained library or small shared tool |
| Accepted state, access, protection, versions, dirty writing, exact review, replay and Undo | Existing core/application service |
| Understandable controls and presentation of actual state | UI and reusable components |

A skill can invoke existing code; instruction text alone provides neither execution
nor file access/permission. Do not have a model simulate exact processing that an
available tool performs more reliably and cheaply. Prefer ordinary notes/artifacts
for briefs, plans and summaries. A dedicated workflow, runner or screen must earn
its ongoing ownership cost through useful repeated interaction or a present need.

## Before a meaningful change

Inspect the current source and all relevant consumers. Identify the owner of the rule, the state it depends on, and the boundary through which callers reach it. Search for existing implementations before creating another one. Use the pinned dependencies as the starting point for reuse research; verify the actual API and behavior, not only a package description.

For a material capability, state what the agent decides, what the tool executes,
what the host validates and what the owner sees. Compare an existing capability
unchanged, skill/example improvement, a small maintained adapter, and custom code
where still necessary. Include caller/adapter cost, dependencies, verification,
failure/recovery and retired routes. Inspect pinned APIs/implementation and use a
bounded fixture where uncertainty matters. Record the choice in the existing task;
small obvious repairs need no options essay. Custom code can provide a useful
interaction, reusable operation, durable guarantee or measured outcome/latency/cost
improvement; novelty and line-count targets do not decide its value.

## Design rules

| Concern | BRN rule | Useful evidence |
|---|---|---|
| Shared policy | Provider/request configuration, writing-context selection and other cross-host decisions belong to their shared owner. Hosts retain presentation, input and platform-specific effects. | A second real consumer uses the same operation without copying its policy. |
| Module depth | An interface should hide meaningful decisions and expose a small set of useful operations. A new crate or trait must earn its interface and build cost. | Name what callers no longer have to understand; prove the supported consumer boundary. |
| UI state | Group values that must change together. Use enums/transition functions for lifecycles where they eliminate invalid combinations or repeated cleanup. | Defined events, outcomes and invariants; tests for the consequential transitions. |
| Effects | Keep deterministic decisions separable from provider, filesystem, persistence and GPUI effects when that makes the affected behavior easier to verify. | The decision can be tested without duplicating the integration; real boundary tests still exist. |
| Reuse | Prefer a maintained existing facility when it reduces total ownership cost and satisfies the contract. Small custom code remains valid when the adapter would be worse. | Actual API/fixture evidence and a bounded adapter; no parallel interpreter or unnecessary framework. |
| Retirement | A replacement removes its obsolete route, callers, configuration and implementation-only tests. Supported saved-data readers and recovery are not dead merely because normal fixtures do not reach them. | Caller/feature inventory, compatibility evidence and retained behavior tests. Temporary coexistence has a reason and removal trigger. |

BRN's checked state changes, authority, version checks, replay and Undo remain owned by the existing core/service boundaries. A UI reducer cannot become a second persistence or authorization engine. Derived search storage is legitimate; do not confuse it with a second canonical state owner.

Prefer local ownership improvements over an application-wide reducer, generic event bus, universal `common` crate or speculative plugin layer. A reducer models actual editing, run, review or recovery state; it must not prescribe agent reasoning. Similar syntax alone does not establish shared responsibility. A larger cohesive implementation can have a simpler interface than many small mutually dependent files.

## Tool design and error handling

Share clear arguments/descriptions/outcomes across actual callers rather than
one mode per task wording. Host authorization remains separate from model-selected
targets/content. Validate an intended batch before effects where required. Known
offered-tool argument mistakes rejected before dispatch can receive concise field
correction guidance within a finite repair allowance and existing run/time/token
limits. Unknown/unauthorized actions, unparseable protocol and uncertain outcomes
after dispatch remain distinct; do not enable global provider retries. Use existing
operation identities and receipts for replay. Changed request contents cannot reuse
a conflicting identity.

A targeted edit checks exact current base/target, constructs its candidate through
shared code and uses existing review/write/Undo. Preserve untouched bytes/metadata.
Only host construction from a checked exact edit qualifies for that path; a model
mode flag does not. Host reads do not establish model full-read coverage. Ordinary
whole-document replacements retain their whole-read requirement. Delivered text
and valid quotes establish availability/provenance, not comprehension or truth.
These are design requirements; current observable behavior remains in its contract.

## What counts as done

For a material change, the existing review/handoff should answer these questions in a few sentences:

1. What owner-visible behavior changed, and which contract remains protected?
2. What became easier to understand or change? If complexity was added, what present requirement justifies it?
3. What obsolete path was removed, or why must it remain temporarily?
4. What evidence checks the behavior, boundary and relevant failure modes? What remains unverified?
5. Is consequential debt deferred? Identify the affected boundary, consequence, responsible task/lane and revisit trigger.

There is no requirement to delete code or add an abstraction in every PR. A small change that fits a healthy boundary may need only a short explanation. Independent review must challenge unnecessary layering and duplicated rules as well as incorrect behavior. A repeated defect should prompt inspection of its shared cause before another local guard is added.

## Checks that keep improvements in place

Use the [verification guide](verification.md) and existing tools first. Rust visibility/types, maintained test facilities and the existing provider/intake retirement checks already provide useful protection. When a change establishes an important boundary, consider a cheap check that fails when it is crossed: for example a forbidden UI dependency in the core, a real independent writing host, or absence of a retired entry point. Check all relevant manifest sections/features rather than relying on a name search that misses aliases or optional paths.

Checks need specific diagnostic messages and a clear owner. Exceptions or changed baselines must be visible and justified in review; they must not silently erase regressions. Extend existing tooling where sufficient. Do not build an architecture analysis platform for a few rules.

Test desired behavior using independently specified expectations. Use examples for known bugs, transition/sequence tests for ordering, fault injection for interrupted effects, and mutation testing for suspiciously weak guard assertions. A complete transition table covers the defined model; it is not proof of every possible execution or native interaction.

Retain separate evidence for real rendering, focus, selection, scrolling and OS behavior. Investigate flaky tests as defects in the product or test harness until evidence identifies the cause. Record reruns and original failures; repeated execution until green is not a repair.

## Evidence for retaining special machinery

For a contested specialized route, compare a viable agent with concise skills and
ordinary tools under comparable model/effort, inputs, access and outcome criteria.
Inspect useful saved results, source support, unintended changes, owner corrections,
elapsed time and actual usage/cost when available. Several reasoning routes can be
valid; do not grade a prescribed trajectory. Reuse existing fixtures/evaluation
tools rather than a new benchmark service.

Synthetic models establish dispatch/state behavior; live runs assess usefulness
and native sessions assess actual interaction. An agent saying done or a passing
source-shape check is not outcome proof. Keep proven cheap processing, including
intake prefetch, unless a relevant controlled comparison supports replacement.
Revisit after material model/tool changes or a concrete regression. Stop optional
broadening when the selected outcome/gates are met, then continue useful authorized
work; no deletion or state-machine conversion quota applies.

## Review whether change is getting easier

At an assigned milestone, or when the same category of regression recurs, review the affected area using evidence already collected. Keep the record small and select only justified follow-up work.

| Signal | Question |
|---|---|
| Change spread | For comparable changes, must agents understand/edit fewer unrelated modules and repeat fewer decisions? |
| Duplicated ownership | Does each replaced policy have one remaining production owner? Are temporary exceptions being retired at their trigger? |
| Regressions | Which defects escaped or reopened? What shared design or test weakness allowed them? |
| Feedback reliability | What are the relevant local/CI times, retries, flaky and ignored cases? Is verification getting slower without stronger evidence? |
| Qualification | Which required journeys are automated, native-observed, owner-accepted or still outstanding on the relevant revision? |
| Deferred debt | Did the promised revisit condition arrive? Is the consequence still real? Close obsolete items instead of accumulating them. |

Use lines, file sizes, dependency counts, test counts and coverage to locate questions, not as quotas. Do not establish a composite quality score without an interpretation that supports a real decision. Do not claim a trend from a single snapshot or compare test timings across different machines/features as if they were controlled measurements.

## Basis

The policy combines BRN's existing shared-service and retirement rules with John Ousterhout's [continuous design investment](https://web.stanford.edu/~ouster/cgi-bin/cs190-winter18/lecture.php?topic=working) and [information-hiding modules](https://web.stanford.edu/~ouster/cgi-bin/cs190-winter18/lecture.php?topic=modularDesign). Martin Fowler's [Yagni](https://martinfowler.com/bliki/Yagni.html) distinguishes maintaining changeable code from building speculative capabilities. Birgitta Böckeler's [reported experience with maintainability checks for coding agents](https://www.martinfowler.com/articles/sensors-for-coding-agents.html) supports combining executable rules with contextual design review; it is one application's experience, not a measured guarantee for BRN.
