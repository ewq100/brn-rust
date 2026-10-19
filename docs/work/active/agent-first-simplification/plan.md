# BRN agent first development plan

**Purpose:** Make Brain a durable knowledge workspace and a clear interface for capable agents. Put new semantic behavior in skills first. Keep application code for useful interaction, dependable operations and improvements supported by evidence.

**Owner:** Evo. **Prepared:** 19 October 2026, Europe/Tallinn. **Audience:** The current BRN development lead and its assigned helpers.

**Repository:** `ewq100/brn-rust`. The original architecture review inspected `b5f01b07301dcc94d3d4e273a472d487f569bbc7` through PR #163. This handoff was refreshed against `e583a177ad44b0536be9140b5c845a5dd8a8aa75` through PR #168. These are evidence snapshots, not instructions to reset or check out old source. Reconcile the actual current branch, main and active work before implementation. [R1]

**First action:** Incorporate this plan into the existing development programme, update the standing development rules, and continue into the first eligible implementation slice. Do not stop after producing another plan. Implementation, live model performance and native acceptance are separate outcomes; this handoff does not claim them complete.

## 1. Product direction

Brain should give Evo one place to keep useful knowledge, bring in documents, work with agents, read and edit their outputs, comment on exact passages, review changes and see what needs attention. A stronger model should improve interpretation, organization, planning and writing without requiring a new BRN feature for each improvement.

The agent decides what information means and how to pursue an authorized task. The application gives it usable capabilities, keeps accepted state, executes changes correctly and makes its work understandable to the owner. A skill may use existing scripts and libraries; the alternative to a custom BRN subsystem is often reusable software guided by an agent.

Use four responsibility categories:

| Responsibility | Default owner | Examples |
|---|---|---|
| Interpretation and judgment | Agent with concise skills and examples | Summarize, organize a wiki, investigate conflicts, draft, prioritize, choose what to retrieve. |
| Exact processing | Existing library or a small shared tool | Convert a document, calculate, search an index, create an artifact, apply an exact passage edit. |
| Durable state and execution | Existing BRN core and application service | Save, versions, access scope, protection, dirty writing, exact review, replay, Undo, run identity and recovery. |
| Human interaction | Existing UI and reusable components | Read, select, comment, compare changes, inspect evidence and act on attention items. |

Novelty is not the criterion for custom code. A familiar editor operation can be worth building because it saves owner effort. An unusual synthesis may be delivered entirely through a skill. Judge the complete result and ownership cost.

### Keep and strengthen

- One authoritative knowledge store with supported history, backup and recovery. Keep the current SQLite authority and derived search model unless a separately justified replacement is selected.
- One shared change mechanism for owner, agent, desktop and CLI work.
- Reading, writing, passage comments, readable proposals, exact acceptance and Undo.
- A simple attention surface with persisted findings, real execution status, supporting references and clear actions.
- Useful search, document conversion and necessary assets through maintained components.
- A general agent path with reusable tools and discoverable guidance.

### Delegate or keep as ordinary artifacts

Wiki organization, project briefs, people summaries, meeting notes, semantic conflict review, research plans, reply drafting and routine knowledge maintenance should normally be agent work. Store their useful outputs as ordinary notes or artifacts. Do not give each task a dedicated persistent workflow, model runner or screen by default.

Retain small task records, project context/access and topic-note navigation where they serve real use. Do not collapse distinct obligations merely to reduce type count. Freeze expansion into general project management, a workflow designer, a semantic graph engine or an increasingly elaborate shell until a repeated owner need justifies it.

### Defaults for this implementation plan

- Ordinary notes remain maintainable under the existing chosen delegation. Protected changes continue to require applicable scoped owner authority or exact review.
- Ordinary conversation, document commands and comments should express the same intended action consistently. An ambiguous target produces a focused question or reviewable candidate.
- Whole-document Rewrite remains available. Add an appropriate operation for genuinely local edits rather than weakening the existing whole-replacement guard.
- Full-note import must still retain the requested substantive content; useful-information intake may select relevant information. Originals remain external under the current contract.
- Attention distinguishes recorded state from agent judgment. A successful save is an execution fact; a suspected contradiction is a finding with reasons and evidence.

## 2. Reconcile the active programme

The current Mac campaign already has a lead, ownership records, checks, resource rules and a native queue. Use that system. Do not create a competing lead, duplicate a PR or start a second implementation of an owned interface.

Recommended repository location for this document:

`docs/work/active/agent-first-simplification/plan.md`

If an active plan already serves this purpose, update it and link it from the existing campaign instead. This document is the workstream plan, not a replacement for the programme's execution machinery.

Before writing source:

1. Read current `AGENTS.md`, `docs/status.md`, architecture overview/invariants, relevant target/behavior sections, strategic-programming guidance and the active campaign plan/checkpoint.
2. Reconcile branch, HEAD, fetched main, open worktrees, active agents/processes, PRs and file ownership. Treat dated document claims as evidence to verify, not current Git truth.
3. Remove already completed work from this plan. The #163 worker-identity work and subsequent #168 changes are not new assignments here.
4. Classify remaining work as retained product reliability, agent capability, guidance, interaction simplification, or deferred expansion. Reorder slices when current evidence identifies a better route.
5. Record a short baseline and next action in the existing checkpoint. Start the first ready slice; do not require an exhaustive whole-repository audit.

At the refreshed baseline, the active campaign records Mac priority, authorized in-scope merges after review/checks, a batched 18:00 Tallinn native session, a quota floor around 5%, and a fixed expiry of 1 November 2026 at 21:41:51 UTC. It excludes new live provider/API workloads, private-data use, new billing and release. Reconcile these against the recipient session's actual current owner instructions; preserve valid authorization without asking again. This plan does not restart the campaign clock or create a new schedule. [R11]

Keep work primarily headless and use the existing native queue. If the Mac is locked or native access is unavailable, finish independent implementation and checks, preserve a runnable candidate and queue the actual interaction. Do not equate state tests or a successful build with native observation.

## 3. Current findings to verify and act on

The comparison from the original review to the refreshed baseline did not change production `brn-ai`, the packaged guidance or the core. Desktop source and campaign state did advance. Re-find callers and symbols on the current revision; old line numbers are navigation aids. [R1]

| Finding | Evidence and interpretation | Disposition |
|---|---|---|
| Semantic skills already exist | Intake, maintenance, conflict and reply guidance are wired into the runtime; installed instruction packages are also implemented. [R3] | Extend and simplify the existing system. Do not build another skills platform. |
| Main-agent invalid batches can stop the run | Known schema mistakes are rejected before dispatch; helper behavior already demonstrates bounded explanatory errors. [R4] | Implement narrow, budgeted correction for eligible requests rejected before effects. |
| Small note edits use whole replacement | The full-read guard protects against dropping unseen content. [R5] | Add one version-bound local-edit primitive using existing candidates and writes. Keep the whole-read rule for whole replacement. |
| Instruction routing differs across UI paths | Ordinary asks, contextual protected edits and writing requests enter different authority modes. [R6] | Simplify how the owner expresses intent using the existing shared execution path. |
| Knowledge checking includes mechanical checks and prose remedies | Structural findings are useful; repair guidance and semantic interpretation need not grow inside Rust. [R7] | Keep objective findings; put reusable repair policy in the maintenance skill. |
| Intake prefetch has limited positive evidence | The 14 October A/B records equal quality on S1–S3 and 28% lower effective S1 cost, with explicit limitations. [R8] | Retain it unless a relevant controlled comparison justifies change. This is not a current universal saving. |
| Broader execution tools are limited | Installed skills are instruction-only; current MCP permits narrowly shaped, exact reviewed input. [R9] | Prove one useful capability through genuine reuse. Do not build a generalized execution/connector platform. |
| Feedback resolution is a separate writing action | An accepted rewrite and marking captured comments addressed have distinct operations. [R10] | Consider combining explicit owner acceptance in one review interaction while preserving exact history and Undo. |

## 4. Development rules the agent must update

Update the existing files in place. Keep `AGENTS.md` short and point it to the detailed strategic-programming guide. Merge overlapping rules rather than appending multiple contradictory versions. Preserve unrelated working boundaries and valid authorization.

### 4.1 Text to merge into AGENTS.md

Place this section after the existing strategic-programming introduction, or merge equivalent sentences into the current rules. Adjust the relative link only if the actual repository layout differs.

```markdown
## Agent work and application responsibilities

BRN is a durable knowledge workspace and an interface for capable agents. Follow
the responsibility rules in [strategic programming](docs/development/strategic-programming.md).

- Start new semantic behavior with the existing agent, relevant skills/examples
  and existing tools. Add application code for a necessary interaction, a missing
  reusable operation, integrity/execution guarantees, or a measured improvement.
- Before a material feature, state what the agent decides, what a tool executes,
  what the host validates and what the owner sees. Explain why the simplest viable
  approach is insufficient. An obvious local repair needs no options essay.
- Inspect actual production callers and pinned dependencies before creating a
  new mechanism. Distinguish shipped code from tests, experiments and historical
  routes. Judge the complete adapter and maintenance cost of reuse.
- Keep meaning and task strategy in runtime guidance. Keep exact state, access,
  versions, protection, dirty writing, review, replay and Undo in the existing
  shared service. Skills and model output cannot manufacture authority.
- Give the agent useful tools and clear results. Permit bounded correction of
  eligible requests rejected before effects; do not confuse a missing response
  with proof that no action occurred. Do not add a general retry engine.
- Use small reusable operations across real callers. Do not add a persistent
  workflow, special agent or screen for each semantic task. A new framework,
  generalized permission system or execution platform needs a concrete present
  requirement and evidence that a simpler approach is worse.
- Keep proven cheap processing where it improves quality, latency or cost. The
  fact that a model could perform the same step is not sufficient to delete it.
- Prefer ordinary notes/artifacts for generated briefs, plans, research and
  summaries. Build dedicated UI when repeated use or clearer interaction earns it.
- A replacement retires obsolete production routes, wiring and implementation-only
  tests. Preserve supported data and recovery. Temporary coexistence needs a reason
  and a removal trigger; line count and deletion count are not completion targets.
- Verify real outcomes with proportionate checks and representative agent tasks.
  Separate deterministic correctness, live model usefulness, native observation,
  owner acceptance and integration. Update affected guidance with the behavior.
- Use available models, helpers and context efficiently under the current task's
  authority. Delegate bounded independent work, retain one integration owner and
  continue useful authorized work without routine reauthorization.
```

### 4.2 Text to merge into strategic-programming.md

Add this material to the existing design/reuse/evaluation sections. Keep one authoritative explanation of each rule; remove superseded duplicate wording.

```markdown
## Choosing agent behavior or application code

For a material new capability, first identify the owner outcome. Classify the work
as semantic judgment, exact processing, durable execution/state, or human
interaction. The implementation may contain more than one category, but their
owners must be explicit.

Semantic judgment normally belongs to the existing agent with relevant skills and
examples: interpretation, organization, retrieval strategy, planning, drafting,
conflict analysis and prioritization. Exact processing normally belongs to an
existing library or a shared tool. The existing core/service owns accepted state,
scope, protection, versions, dirty writing, exact review, replay and Undo. The UI
owns understandable interaction and presentation of real state.

A skill may invoke existing code. Do not ask a model to simulate arithmetic,
parsing, file conversion or exact editing when an available tool performs the
operation more reliably and cheaply. Installing instruction text does not itself
provide execution, file access or permission.

Before adding custom machinery, briefly compare: an existing capability unchanged;
a prompt/skill/example improvement; a small adapter to a maintained facility; and
custom code if still necessary. Read the pinned API and relevant implementation,
then use a bounded fixture or prototype where uncertainty is material. A package
name or marketing claim is not proof of reuse. Record the choice in the existing
task record, not a new governance system.

Custom code is justified when it provides a needed reusable operation, a necessary
interaction, a durable guarantee, or a demonstrated improvement in outcomes,
latency or cost. Prefer fewer rules callers must understand over arbitrary line,
file, trait or dependency targets. Permit a larger coherent replacement when
evidence shows it reduces total responsibility and the assignment covers it.

## Tool design and error handling

Make tool descriptions, arguments and outcomes easy for the model to use. Share
contracts across actual callers instead of adding one mode for each task wording.
Keep private host authorization separate from model-selected targets and content.

Validate an intended batch before effects where the contract requires that
property. For known offered tools, eligible schema mistakes rejected before
dispatch may return concise correction guidance within a finite repair budget and
the existing run limits. Unknown or unauthorized actions and uncertain outcomes
after dispatch are not the same error class. Use existing operation identities and
receipts for replay; changed request contents must not reuse a conflicting identity.

For targeted edits, verify the exact current base and target, construct the result
through shared code, and use the existing candidate/review/write/Undo path. Preserve
untouched content and metadata. Retain whole-read requirements for whole-document
replacement. Delivered text and valid quotations prove availability/provenance,
not comprehension, truth or complete satisfaction of a requirement.

## Evidence for retaining special machinery

Compare the specialized route with a viable agent-and-tools route when the
additional machinery is contested. Hold the model, inputs, permitted operations and
outcome criteria comparable. Track useful saved results, unintended changes,
owner corrections/interventions, latency and actual usage/cost when available.
Do not require a prescribed reasoning trajectory when several routes are valid.

Reuse existing fixtures, journey tests and evaluation tooling. Synthetic models
establish dispatch and state behavior; live runs assess model usefulness; native
sessions assess real interaction. Label each result for its actual source and
scope. An agent saying 'done' or passing a source-shape test is not outcome proof.

Retain measured optimizations until relevant evidence supports replacement. Revisit
them when the model/tool setup changes materially or a concrete regression appears;
do not create a recurring benchmark service without an assigned need.

## Scope and retirement

Improve state ownership for retained user workflows. A reducer models actual
editing, run, review or recovery state; it should not prescribe the agent's
reasoning. Do not build an application-wide state machine, universal event bus,
plugin marketplace or parallel planning system to enforce these rules.

When replacing behavior, retire its obsolete production mechanism and tests that
only mirror it. Preserve supported saved-data/recovery obligations and meaningful
behavior checks. Temporary parallel routes need a named owner and removal trigger.
Stop optional broadening once the accepted outcome and required gates are met,
then continue the next useful authorized slice.
```

### 4.3 Related documentation to reconcile

| File or area | Required update |
|---|---|
| `AGENTS.md` | Merge the concise standing rules above. Keep development instructions distinct from runtime skill guidance. |
| `docs/development/strategic-programming.md` | Merge the detailed responsibility, reuse, correction, evaluation and retirement rules. |
| `docs/architecture/threads-target.md` | Clarify the agent/interface product direction and selected capability boundaries. Preserve existing agent-first commitments and mark consequential amendments explicitly. |
| `docs/architecture/threads-behavior.md` | Update exact observable tool/interaction behavior when its implementation changes. Retire obsolete descriptions of abort-only errors or route-specific instructions only when replaced. |
| `docs/architecture/invariants.md` | Change only genuinely affected guarantees. Describe targeted-edit guarantees precisely without weakening whole-document, scope, protection or replay rules. |
| `agent/BRN.md` and affected `agent/skills/*/SKILL.md` | Keep instructions short and relevant; move semantic remedies here when appropriate. Teach new tools and recovery from actual examples. Do not copy the development charter into the runtime prompt. |
| Active campaign plan/checkpoint and the legacy vacation roadmap | Link this workstream, preserve current owner/resource rules and reconcile retained priorities. Do not restore obsolete Cloud-first or all-workflows requirements. |
| `docs/development/verification.md` and existing journey records | Add only the checks and live/native distinctions needed by changed behavior. |
| `docs/status.md` and relevant indexes | Record actual integration and remaining qualification; link the chosen plan location. |

Only update files materially affected by a slice. A checklist row does not justify a documentation rewrite.

## 5. Implementation sequence

Use these milestones as an ordered starting point. The lead may merge, split, reorder or retire work when current evidence shows a simpler route. Shared interfaces and integration have one owner. Each slice should produce a coherent reviewable result and a clear user benefit.

### M0 Reconcile and update the rules

Complete section 2, place the plan in the chosen repository location, and merge the rules in section 4. Use the existing inventory to mark keep, simplify, reuse or defer, with one-sentence reasons for material choices. Reconcile the active roadmap before further broad state refactoring.

**Exit:** Current baseline/owners and first ready slice are recorded; development rules and product summaries agree; already completed work is removed. Documentation is a completed milestone, not completion of this programme. Continue to M1.

### M1 Let the agent correct eligible tool mistakes

Inspect the current Rig hook and helper behavior. Implement correction for known offered tools whose arguments fail eligible validation before any call in the proposed batch dispatches. Reuse the registered contracts and current hooks. A small finite allowance, initially two corrective turns unless existing semantics provide a better bounded choice, must consume the existing run/time/token budget.

Return the relevant field/format problem concisely. Keep mixed valid/invalid batches free of effects until corrected. Preserve current handling of unknown tools, authority rejection, unparseable protocol data, external-input grants and outcomes already dispatched unless a separate reviewed change is necessary. Do not enable provider retries globally.

**Acceptance:** Invalid batch executes zero calls; valid correction can proceed; repeated invalid requests terminate visibly at the bound; Stop and scope changes still take effect; no duplicate write or conflicting reuse of request identity is introduced. Existing helper and successful-call behavior remains intact.

**Retirement:** Remove only replaced main-agent abort-only handling and tests that encode that old behavior. Retain meaningful validation, no-effect, authority and replay checks. Update runtime guidance and behavior documentation.

### M2 Keep structural knowledge checks and simplify repair guidance

Separate objective findings from suggestions. Retain checks for stale versions/candidates, broken links and recorded coverage gaps. Keep cheap duplicate/topic hints only where they are useful inputs, visibly distinct from a semantic conclusion.

Move applicable prose remedies into the existing maintenance skill or its small reference file. Have the host return facts, identities and relevant versions; let the agent decide whether a repair is justified. Preserve a bounded explicit maintenance request and existing delegation. A clean scan must not claim factual correctness, and an orphan or similar title must not imply mandatory repair.

**Acceptance:** Structural findings remain available, maintenance can use the skill without contradictory duplicated policy, semantic hints cannot silently mutate records, and unrelated scopes are excluded. Use representative existing fixtures; this slice does not require a general knowledge-quality score.

M2 can run alongside M1 when ownership is separate. Coordinate any shared runtime-guide edit.

### M3 Add one targeted editing capability

Choose the smallest input shape serving a real local edit: note identity, expected base version, an exact range with expected text or an unambiguous exact-text target, and replacement text. Use an existing equivalent if available.

The host reads the current stored note, validates scope/protection/base/target, constructs the complete candidate and sends it through the existing prepare/review/apply/Undo mechanism. Model-generated text must not choose a raw SQL or filesystem mutation route. Use maintained text/edit facilities where they reduce total code; do not introduce a new patch language, patch database or editor framework.

Only a candidate constructed by the host from the checked base and validated exact edit qualifies for the targeted-edit path. A model-supplied mode or claim cannot select that exception. The host's read to construct the result does not count as model full-document read coverage; ordinary model-authored whole replacements keep their existing full-read requirement.

Start with one exact replacement. Add multiple edits only if an accepted journey requires them; then define overlap/order/atomicity explicitly. Stale or ambiguous targets return a structured refusal, never fuzzy silent relocation. Preserve all untouched bytes and metadata. Existing full-document Rewrite and its full-read guard remain supported.

**Acceptance:** A small edit to a long note preserves the rest exactly; repeated text, Unicode boundaries, stale versions, dirty owner writing and protected/private notes behave correctly; review binds the resulting candidate; replay and Undo reuse the current mechanisms. Evidence distinguishes reading the edited passage from reading the complete document.

**Retirement:** Replace duplicated targeted-edit workarounds where actual callers move. Do not retire the whole-note route merely because a passage tool now exists.

### M4 Make document instructions and review consistent

Extend the existing common action context rather than adding a generalized permission classifier. Ordinary chat, a document command and a passage comment should carry the same understandable target and intended effect where applicable.

For a selected document, make asking, proposing changes and applying explicitly authorized edits understandable within the current workspace. The agent may identify the intended operation. The host binds an applicable current owner instruction or explicit owner action to concrete permitted targets and effects. Selection identifies the target; it does not itself authorize a mutation. A source, quotation, summary or skill cannot create owner authority. Content-edit authority does not grant archive, unprotect, another-note edit or external delivery.

The explicit whole-document Rewrite command remains proposal-only through retry, Continue and candidate replacement. Unifying input routes must not turn that request into an automatic maintenance write. Directly authorized edits retain their existing applicable scope.

For rewriting, allow the candidate to propose which captured comments it addresses. A single review interaction may let the owner accept the exact revision and the displayed selected comment resolutions. Do not mark all comments resolved merely because generation or Apply succeeded. Preserve reopen, unresolved anchors, original comment context and Undo behavior. Keep advanced evidence/requirements setup optional for ordinary writing.

Prefer composing existing checked operations where sufficient. One visible action does not require a new transaction framework; if only part of its intended work completes, preserve and display the actual separate outcomes without falsely reporting both successful.

**Acceptance:** Equivalent authorized requests give consistent results across relevant entry points; the owner does not repeat an already valid approval because an internal route changed; ambiguity is presented concretely; unrelated documents are untouched. Accepted comment resolutions reflect the displayed selection, and Undo/reopen preserve their meaning.

**Native qualification:** Queue one consolidated writing/instruction journey against the exact candidate. Headless completion may precede this observation, but cannot substitute for it.

### M5 Prove one broader capability through reuse

Use one reference outcome: from synthetic/public documents, calculate a comparison and return a saved chart plus a short readable report. Retained owner notes may substitute only under existing explicit data-use authority. This is a capability test, not a request to build a chart-analysis module.

First identify what is actually missing. Compare the existing runner/tools, a small adapter to maintained execution/artifact facilities, and one maintained fuller runtime only if necessary. Read current primary documentation and pinned code. Account for process lifecycle, scoped files, cancellation, output validation, credentials, dependency footprint and supported platforms; a nominally thin API may still create a large maintenance obligation.

Build one disposable proof with one chosen route. Inputs are explicitly permitted or synthetic; outputs remain in a bounded scratch/artifact location until accepted through existing BRN facilities. Scratch execution cannot directly access or mutate canonical SQLite. Do not combine this slice with browser automation, general connectors, inbound MCP, external sends, scheduling, a marketplace or multi-runtime orchestration.

Current executable/network authority remains in force. If production use needs a new capability grant, finish useful proof work allowed by current authority and prepare the concrete scope/UX for the owner decision. If the proof itself requires excluded access, complete the interface, source and synthetic-fixture work that remains permitted, record the precise blocked activation and continue other eligible work. Do not silently turn instruction-only skills or one-call MCP approval into broad execution permission.

**Adopt when:** The actual artifact is useful and inspectable, the adapter satisfies the contract, and the ongoing implementation is simpler than a feature-specific subsystem.

**If it fails:** Record the precise gap and retain the current route. A failed reuse experiment does not automatically authorize building the missing platform from scratch.

### M6 Compare outcomes and retire unnecessary machinery

Use the focused evaluation in section 6 to assess contested mechanisms and completed user journeys. Keep host-intake prefetch unless relevant evidence supports replacement. Do not remove stable mechanical optimizations solely because an agent can reproduce them.

Retire obsolete routes as callers move, with their implementation-only tests and configuration. Preserve supported data, real behavior tests and recovery obligations. Any temporary coexistence needs a named owner and a specific removal condition.

**Exit:** The implemented scope is reviewed and integrated under current authority; required checks and resulting-main status are recorded; live/native/owner gaps are explicit; affected rules and skills match the actual behavior. If a conditional capability remains deferred, report it as deferred rather than claiming full completion.

## 6. Verification that decides something useful

Use current BRN tests and evaluation tooling. Start with a small set of representative journeys, not a new benchmark framework. Establish expected outcomes independently of the implementation.

| Journey | Required observation |
|---|---|
| New information updates an existing subject page | Useful facts and qualifications are retained without unnecessary duplicate notes. |
| New source conflicts with a protected decision | Both sources remain distinguishable; the decision does not change without applicable authority. |
| Known offered tool has malformed arguments | No effects occur before correction; a valid repair can continue; the repair budget is enforced. |
| A small edit targets a long document | Intended passage changes; untouched content/metadata stay exact; no silent full-rewrite fallback. |
| Repeated passage or stale base | Ambiguity/staleness is explicit; no edit lands on a guessed target. |
| Owner types while a candidate is prepared | The owner's newer work survives and existing dirty/stale handling remains effective. |
| Multiple comments drive a rewrite | The saved request and selected feedback are respected; review/Apply/Undo and comment state stay coherent. |
| Equivalent document instructions use different entry points | Target and intended effect remain consistent without duplicate approval. |
| An interrupted or uncertain write is continued | Existing receipts determine prior outcomes; no duplicate effect or false completion. |
| Attention is shown without another chat query | Recorded execution status and model-reported findings are visibly distinct and actionable. |
| Reused artifact capability, if adopted | Calculations and actual output are inspectable; permitted scope and cancellation hold. |

For semantic comparisons, hold the model, selected effort, source snapshot, access and outcome requirements comparable. Compare the specialized route with the same capable agent using concise skills and ordinary tools. Do not deliberately cripple the baseline, remove necessary integrity checks or grade an exact reasoning sequence when several strategies are valid.

Track: accepted outcome quality; source support; unintended changes; unnecessary owner interventions; elapsed time; tool/model usage and cost when available. Distinguish estimates from measured billing/usage. Repeat variable cases enough to resolve the concrete decision; do not declare a universal win from one run.

No new live provider window is created by this document. If the current task authorizes a bounded live evaluation, use its actual provider/model/budget. Otherwise complete deterministic checks and prepare the exact small comparison for later activation. Synthetic tool sequences establish control flow, not Sol 6.1's real-world quality. Native queues establish actual UI behavior only when observed.

Run focused checks during development and the required broader gates at integration. Broaden testing only to address a remaining risk or required gate. Preserve meaningful original failures and explain environmental limits. Do not weaken assertions, repeat until green, or add tests that merely reproduce implementation details to increase counts.

## 7. Models, delegation and context

Preserve the active lead configuration and the owner's current model/resource policy. For a fresh assignment, Sol 6.1 is the normal implementation/integration choice; Luna 6 can handle bounded research, source inventories, documentation, receipt checks and small clearly specified changes with verifiable outcomes. Use an independently scoped Sol reviewer for consequential changes, or retain another reviewer already selected by the owner. Do not automatically escalate to Astra or change accounts/models when a route is unavailable.

Resolve these names to actually exposed configured models. Record effective model/effort only when observable. A requested setting is not proof of the runtime configuration, and a cheaper label is not proof of measured savings.

Delegate when work is independent and the result reduces the lead's workload. Give each helper one outcome, relevant paths, ownership, constraints, acceptance and a concise return format. Nested helpers are useful only within the same scope and actual shared capacity. There is no permanent project-wide helper count; inspect the runtime and coordinate the single local build slot.

Keep context task-sized. Give helpers the relevant source and decisions rather than the complete conversation or repository. Return findings, exact references, changed files and unresolved questions instead of raw logs. Retain a compact checkpoint before context renewal. Do not aim to fill a particular context window or claim to configure a helper's window when the runtime does not expose that control.

Continue successive useful authorized batches. Do not turn a recovery heartbeat into the pace of development. Respect the current quota floor, known reset, fixed deadline and explicit owner pauses. Do not purchase capacity or infer unlimited allowance when usage cannot be checked.

## 8. Completion and reporting

For each material slice, the existing handoff or PR description should state:

1. The owner-visible problem and resulting behavior.
2. Which responsibility moved to a skill, reusable tool or existing service.
3. What became easier to understand or change, and what obsolete route was retired.
4. Actual checks and model/native observations, with their source revision and limits.
5. Integration/main status and the next useful action.

Whole-plan completion requires the selected mandatory scope through M4 and applicable M6 verification to be accounted for. M5 is a bounded conditional capability decision: adopted and verified, or explicitly deferred with evidence. Do not label an unimplemented mandatory item complete; if current evidence makes it unnecessary, record the reason and replacement acceptance rather than deleting it from the report silently.

The governing question is whether Brain now lets a capable agent complete more useful work with fewer special application rules and less owner effort, while keeping retained knowledge and changes dependable. Do not set a target for deleting 10,000 lines or converting every UI value into a reducer.

## 9. Starter instruction for the development agent

```text
Use the attached BRN-Agent-First-Development-Plan.md as the next development
workstream. Reconcile it with current main, your active assignment and other agents;
do not redo completed work or interrupt an owned change unsafely.

Place it at docs/work/active/agent-first-simplification/plan.md, or update an existing
equivalent, and link it from the active campaign. First merge its development rules
into AGENTS.md and docs/development/strategic-programming.md, reconciling affected
architecture/runtime guidance. Then continue into the first eligible implementation
slice; do not stop after documentation or another planning pass.

Preserve valid current permissions, resource limits, Mac priority, integration gates
and native queue. Use skills and existing libraries before custom mechanisms, give
the agent better reusable tools, and retire replaced routes. Keep exact state,
scope, protection, review, replay and Undo in the existing shared service.

Use Sol and bounded Luna helpers efficiently under the current model policy. Make
routine implementation choices autonomously, compare contested approaches with real
evidence, and continue independent authorized work if one conditional capability or
live/native check is unavailable. Report implemented, verified, live-observed,
native-observed, owner-accepted and merged separately.
```

## 10. Evidence and design references

Repository references identify the inspected behavior. They do not make an older snapshot current. The external sources explain design/evaluation principles; they do not prove model performance in Brain.

- [R1 — Original and refreshed repository comparison][R1].
- [R2 — Architecture overview and shared ownership][R2].
- [R3 — Existing maintenance skill and runtime catalogue][R3]; [runtime loading][R3b].
- [R4 — Main-agent validation and helper correction][R4]; [helper path][R4b].
- [R5 — Existing full-read guard][R5].
- [R6 — Ordinary and contextual instruction paths][R6]; [contextual edit][R6b].
- [R7 — Knowledge findings and embedded remedies][R7].
- [R8 — Dated host-intake A/B evidence and limitations][R8].
- [R9 — Current MCP schema and reviewed-input restrictions][R9]; [dispatch binding][R9b].
- [R10 — Current writing feedback resolution][R10].
- [R11 — Active Mac campaign authority and execution rules][R11].
- [R12 — Existing strategic-programming guide][R12].
- [Building effective agents][E1]: begin with a simple viable approach and require evidence for additional orchestration.
- [Agent Skills][E2]: instructions, resources and executable tools can package expertise without a feature-specific agent product.
- [Evaluating agents][E3]: inspect actual outcomes and use appropriate deterministic, model and human assessment.

[R1]: https://github.com/ewq100/brn-rust/compare/b5f01b07301dcc94d3d4e273a472d487f569bbc7...e583a177ad44b0536be9140b5c845a5dd8a8aa75
[R2]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/docs/architecture/overview.md
[R3]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/agent/skills/maintain-notes/SKILL.md
[R3b]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/crates/brn-ai/src/threads_skills.rs
[R4]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/crates/brn-ai/src/threads.rs#L2412-L2463
[R4b]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/crates/brn-ai/src/threads.rs#L2238-L2262
[R5]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/crates/brn-ai/src/threads.rs#L1604-L1637
[R6]: https://github.com/ewq100/brn-rust/blob/b5f01b07301dcc94d3d4e273a472d487f569bbc7/crates/brn-desktop/src/threads_ui.rs#L2048-L2092
[R6b]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/crates/brn-desktop/src/threads_context_actions.rs#L51-L195
[R7]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/crates/brn-threads-app/src/knowledge.rs
[R8]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/docs/work/active/knowledge-retrieval/host-intake.md
[R9]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/crates/brn-ai/src/threads_mcp.rs#L66-L78
[R9b]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/crates/brn-ai/src/threads_mcp_runtime.rs#L204-L208
[R10]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/crates/brn-threads-app/src/writing.rs#L837-L1006
[R11]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/docs/work/active/vacation-campaign/plan.md
[R12]: https://github.com/ewq100/brn-rust/blob/e583a177ad44b0536be9140b5c845a5dd8a8aa75/docs/development/strategic-programming.md
[E1]: https://www.anthropic.com/engineering/building-effective-agents
[E2]: https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills
[E3]: https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
