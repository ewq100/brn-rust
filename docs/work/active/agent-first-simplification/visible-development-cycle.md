# BRN development visibility and instruction optimization

Revision 2, incorporating the 19 October 2026 instruction audit, for the existing BRN development lead.

Make development visible as a normal part of every assignment. The owner should be able to open one place and understand what the agents are working on, what comes next, what is waiting, what has changed after a failed check, and which results are established.

Adopt a concise version of this cycle in the repository instructions so it survives the current agent, chat, model and campaign. Consolidate the instructions the agent reads, then implement the smallest useful view over the existing work records. Keep task strategy with the agents and dependable recording and presentation with ordinary tools.

This plan extends the existing agent-first direction. The current lead retains coordination and integration. Read current instructions and actual repository state before selecting work; the dated examples below do not assign already completed tasks again.

This is an implementation and reference report. Keep its detailed rationale, examples and acceptance scenarios available on demand. Its publication does not mean the permanent rules or owner view have been implemented. Integrate the small durable obligations into existing guidance rather than making this report another mandatory startup pack.

## 1 Required outcome

Deliver two connected improvements:

1. **A permanent development practice.** Agents maintain a concise, restartable account of their work and load guidance relevant to the assignment.
2. **An owner view.** The owner can inspect that account without requesting a new explanation in chat or opening raw logs.

A reporting prompt can establish the practice. Durable records, honest freshness indicators and a useful interface give it continuity. The first implementation should reuse existing facilities and require little maintenance.

The owner view must answer these questions in plain language:

| Question | Required answer |
| --- | --- |
| What are we trying to achieve? | The current assignment and the outcome of the selected task. |
| What is happening? | Each active task's last reported activity, responsible agent and observation time. Live execution status only where a supported signal exists. |
| What is next? | The next eligible work and the condition that makes waiting work eligible. |
| What needs attention? | The issue, who can resolve it and the concrete next action. Distinguish owner decisions from work an agent can handle. |
| What has been done? | The result, supporting evidence and any remaining integration or acceptance requirement. |
| Why is work repeating? | The failed check or review finding, what changed, and what the next attempt established. |
| How does the work connect? | The useful dependencies between tasks, including independent work and shared-resource waits. |

Use outcome descriptions such as “Expose whether a feedback choice is selected to accessibility tools.” Put filenames, commit identities and full command output in details.

## 2 The recurring development cycle

Apply this cycle to future bug fixes, features, refactors and investigations. Scale the record to the assignment: a small repair may need one task and a few lines.

### Begin or resume

Read the effective instructions, current assignment and current work handoff. Check branch, source revision, integration state and unrelated changes before relying on an old checkpoint. Load the applicable constraints and affected contract sections; consult historical receipts to establish a specific decision, prerequisite or result.

During initial adoption, inspect the actual local instruction chain and installed development skills. Repeat that inventory when the configuration materially changes or instruction conflicts recur, rather than before every small task. A repository audit alone cannot establish which global instructions, skills or conversation history the Mac agent receives.

For the next useful task, state its intended outcome, responsible agent, relevant prerequisites and what will establish completion. Use the existing lead and current resource and authorization limits. Start independent eligible work while another task is waiting.

Do not make every task wait for a large upfront plan or a full campaign-history read. Record dependencies only where they affect execution or understanding. Selective reading must preserve applicable authority, repository rules and the constraints of the affected behavior.

### Work and report meaningful changes

Update the record when work starts, responsibility changes, an approach materially changes, a check or review produces a result, a blocker appears, integration occurs, or work pauses.

Each update should say what happened, what it means for the task and what comes next. Summarize observable actions and decisions in a few useful sentences. A transcript of every tool call is unnecessary.

Do not launch a separate agent merely to narrate another agent's work. Use the acting agent's existing reporting and supported tool or execution events where available.

### Check and improve

Use the checks required by the repository and those that resolve a concrete risk in the change. Record the relevant revision or artifact, result and limits. Use independent review where the existing workflow or the consequence of the change warrants it.

When a check or review finds a problem, preserve the finding and record the next materially different action. That difference might be a code change, clearer tool instructions, corrected test setup, a revised hypothesis or a new diagnostic observation.

A rerun can be justified by new evidence of an environmental or intermittent problem. Record that reason and retain the first result. Repetition alone does not establish a repair.

### Deliver or integrate

Deliver or integrate within the current assignment's authority. Preserve existing review and repository checks. After integration, verify the relevant resulting state and reconcile the work record.

Report implementation, verification, integration, actual target-environment observation and owner acceptance separately. A task is done when its agreed completion criteria are met; a broader journey can still have outstanding qualification.

Where an owner decision is actually needed, present the completed, reviewable result and the precise decision. Continue independent authorized work rather than asking the owner to reconfirm routine steps.

### Continue or leave a checkpoint

Select the next eligible task while useful authorized work and resources remain. At a pause or handoff, leave the current source identity, pending work, evidence links, unresolved issues and exact next action.

A resumed session first refreshes observable facts. It must not mistake an old “working” entry for proof that an agent is still running.

~~~mermaid
flowchart TD
    A["Choose ready work"] --> B["Make a bounded change"]
    B --> C["Check and review"]
    C -->|"Problem found"| D["Record finding and next change"]
    D --> B
    C -->|"Criteria met"| E["Deliver or integrate"]
    E --> F["Verify result and update view"]
    F -->|"Integration problem"| D
    F -->|"More authorized work"| A
~~~

The diagram describes a recurring practice. It does not require a workflow framework or prescribe how an agent reasons.

## 3 Graphs and loops in the owner view

### Graphs show dependencies

A work graph connects tasks where one result is needed by another. Its useful information is what can proceed, what must wait and which prerequisite is holding work up.

Keep the default graph small: the current task, its relevant prerequisites and the next affected tasks. Let the owner open a task's details from the same work record. Completed history can remain collapsed.

Store each dependency once. Derive the graph from those dependencies rather than maintaining a second visual task list. When a prerequisite has a specific requirement, record that condition. “Needs the reviewed interface” and “needs the change merged” can imply different readiness.

Shared resources are a separate constraint. Two independent tasks may still need the same build slot. Show “Waiting for build slot” with the responsible owner; do not invent a functional dependency between them.

A circular prerequisite in an ordinary task dependency graph needs correction or an explicit joint milestone. Iterative improvement belongs in the task's attempt history.

### Loops show checks and corrections

A loop is the visible history of trying something, checking its result and changing the next action.

A useful expanded task history might show:

| Step | Owner-facing account |
| --- | --- |
| Observation | The choice changes visually, but accessibility tools do not receive its selected state. |
| Change | Add the existing toolkit's selection metadata and a clearer accessible name. |
| Check | Observe selection changes in the matching Mac build and run relevant behavior checks. |
| Result | Record what passed, the remaining integration requirements and any usability still unobserved. |

This is an illustrative summary of the accessibility work described in [PR176](https://github.com/ewq100/brn-rust/pull/176). Refresh its state before using it as current work.

Show the relevant attempts and what changed between them. Do not use an animated loop or an attempt count as evidence of progress.

The owner needs actions, findings, concise decision explanations and evidence. Private deliberation is not a required data source.

## 4 Assign responsibility to the right layer

| Responsibility | Default implementation |
| --- | --- |
| Interpret the assignment, choose useful tasks, propose dependencies and explain why a result matters | The existing development agent with concise instructions. |
| Record actual command outcomes, source identities, timestamps and available execution events | Existing tools and supported execution hooks. |
| Preserve a valid work record and render status consistently | Existing storage plus a small adapter or renderer where needed. |
| Present current work, attention, dependencies and evidence | A reusable owner view over that record. |
| Decide whether an action is authorized or whether a required repository check can be skipped | The existing authority and repository rules continue to govern. A status label grants no permission. |

The first version needs no separate decision model. A known failed check, merged pull request or missing observation should retain its recorded meaning. A model can help summarize the significance.

Before adding code, inspect existing task displays, structured plan facilities, repository records and rendering tools. Choose the least costly implementation that supplies the required visibility. Compare the complete maintenance burden, including adapters and failure handling.

Keep the initial view read-only. If control actions are added later, they must connect to the actual existing agent control path and clearly report whether the command was received and acted on.

## 5 Design the owner view

Use one stable entry point with four primary sections. Chat updates should summarize the same underlying record.

### Working on

Show the assignment, task outcome, responsible agent, last reported activity and timestamp. Show separate rows for real parallel tasks.

Where actual execution telemetry exists, distinguish that signal from the agent's description of its task. “A check is running” and “the agent last reported reviewing a fix” are different observations.

### Next

Show the next eligible task or useful continuation and why it is next. Show waiting tasks with the condition that will unblock them.

A task being next in the plan does not mean execution has started. Readiness also depends on current authority and resources.

### Needs attention

Show a specific issue, its consequence, the responsible person or agent and the next action. Separate:

- An owner decision that cannot be resolved from existing instructions.
- An agent-owned failure or blocker being investigated.
- An external or resource wait, such as a pending check or occupied build slot.

Do not mark all waiting work as needing the owner. A missing owner response should block only the affected decision and its actual dependants.

### Completed

Lead with the result. Make the evidence and its scope available in details.

Use distinct facts such as implemented, relevant checks passed, merged, observed in the target environment and accepted by the owner. Criteria that do not apply to a task may be marked not applicable with a short reason; do not silently waive existing requirements.

For example, a documentation change usually does not need a native interaction session. A native interaction claim needs evidence from that environment.

### Details and visual behavior

Provide the selected task's dependencies, meaningful attempt history, evidence links and remaining requirements. Use text labels as well as color. Support ordinary keyboard navigation and readable narrow layouts in any custom UI.

Avoid a whole-project completion percentage or finish time unless a defined scope and defensible calculation make it useful. Keep raw test counts, token totals and lines changed out of the primary progress explanation.

For the first version, a well-organized existing task surface or rendered Markdown overview with a dependency diagram can meet this need. Choose a custom screen only for interactions the simpler surface cannot provide.

## 6 Give instructions and current facts clear owners

BRN already has development rules, a campaign state file, a restart checkpoint, a status summary and evidence records. Reuse that structure. One owner per fact does not require one giant file; it requires clear responsibility and references.

| Existing location | Responsibility |
| --- | --- |
| Current task prompt | The assignment, intended outcome and new constraints. Reference standing guidance rather than repeating the whole instruction set. |
| [AGENTS.md](https://github.com/ewq100/brn-rust/blob/main/AGENTS.md) | Short permanent obligations and links to detailed guidance. |
| [Development workflow](https://github.com/ewq100/brn-rust/blob/main/docs/development/workflow.md) | The normal cycle, evidence requirements and integration procedure. |
| [Strategic programming](https://github.com/ewq100/brn-rust/blob/main/docs/development/strategic-programming.md) | Design, reuse and retirement rationale and examples, consulted for relevant design decisions. |
| Applicable development skill | A bounded recurring procedure with a clear trigger and supporting references loaded when needed. |
| [Campaign state](https://github.com/ewq100/brn-rust/blob/main/docs/work/active/vacation-campaign/campaign.json) | Existing structured state to inspect and extend if it remains the appropriate current record. |
| [Restart checkpoint](https://github.com/ewq100/brn-rust/blob/main/docs/work/active/vacation-campaign/checkpoint.md) | Essential resumption context and pointers to authoritative evidence. |
| [Current status](https://github.com/ewq100/brn-rust/blob/main/docs/status.md) | Concise navigation and owner-facing summary. |
| [Evidence index](https://github.com/ewq100/brn-rust/blob/main/docs/work/active/vacation-campaign/evidence.md) | Retained observations, reviews and verification results. Historical snapshots do not own current work. |

Select one owner for each current fact. Reuse an existing tracker or structured record when it is sufficient. Derive presentation and graph data from it. Keep checkpoint prose for context that does not already have a clear structured owner.

For the current campaign, extending its existing state may be sufficient. Future assignments should use the same practice through their normal task records, without depending on a file named vacation-campaign. Use the existing status entry point to locate the active assignment.

Keep historical evidence and its original results. Remove superseded claims that a historical snapshot controls the next action, and point to the current owner. A full historical migration is unnecessary.

### Select documents by the decision they support

Keep essential boundaries and routing in AGENTS.md. Let the workflow own the recurring procedure and the strategic guide own detailed design reasoning. Consolidate repeated explanations under those owners, while retaining short reminders where they prevent a consequential error.

Replace broad startup requirements to read all evidence with instructions to read the current handoff, applicable constraints and evidence for the selected work. Load the relevant architecture, setup and verification sections when the affected task requires them. Keep detailed tool output in receipts; summaries should link to the useful result and its limits.

The goal is correct decisions with less unnecessary reading. Word counts are diagnostics, not quotas. Do not shorten away permissions, build coordination, review gates or data and recovery guarantees.

### Audit development skills separately from application skills

Inspect the effective global and repository instructions, overrides and installed development skills in the actual build environment. Record unresolved precedence or scope conflicts. Scope any edits to shared global guidance to the current authorization; do not erase a requirement merely because it conflicts with another file.

The files under agent/skills and other prompts packaged by BRN belong to the application's runtime agent. Their loading is separate from the coding agent's installed skills, as shown by [the runtime guide packaging](https://github.com/ewq100/brn-rust/blob/main/crates/brn-ai/src/threads_guides.rs). Do not change shipped prompt behavior as an incidental documentation cleanup.

For each development skill, retain a specific recurring job, a clear trigger and concise entry instructions. Load the selected skill according to its host's rules, and move supporting examples or specialist detail into references when appropriate. A skill activated for every task does not become cheaper merely because its text lives in a different folder.

Codex's documented loading model starts with skill metadata and loads selected instructions later. Its instruction chain may also include global guidance and directory overrides. Verify the actual host/version before changing configuration; see [Build skills](https://learn.chatgpt.com/docs/build-skills) and [Custom instructions with AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md).

### Keep helper assignments and lessons focused

Give a bounded helper the outcome, relevant revision, file ownership, applicable constraints, references and evidence to return. Include broader history when its task depends on it. The lead retains the wider coordination context.

A repeated failure can justify a clearer tool description, a test, a skill change or a permanent rule. Put the correction where the relevant decision is made. Keep one-off attempt history with the task instead of automatically expanding AGENTS.md.

### Minimum information

Map these concepts onto existing fields before inventing a schema:

| Information | Purpose |
| --- | --- |
| Stable task identity and intended outcome | Connect updates, evidence and dependencies to the same work. |
| Responsible agent or lead | Show who owns the next action. |
| Current task state and reported activity | Distinguish planned, active, waiting and completed work. |
| Observation time and source | Establish what the report is based on and its age. |
| Dependencies and required conditions | Explain readiness where sequencing matters. |
| Blocker and next action | Make waiting work understandable and resumable. |
| Completion criteria and remaining requirements | Make done meaningful for this task. |
| Source revision or artifact identity and evidence links | Bind a result to what was actually examined. |
| Meaningful attempt history | Explain corrections and preserve unsuccessful observations. |

A trivial task can omit dependencies and attempt history. Avoid creating a general event system, database or scheduler just to represent this information.

If an adapter writes local records, preserve the last valid record when an update fails. Use the repository's normal serialization and atomic replacement facilities where appropriate. The display should show that its data could not be refreshed rather than silently treating an old record as current.

Parallel helpers report within their assigned scope. The lead owns the consolidated plan and integration state. Avoid concurrent edits to a shared task file; use existing task-specific reporting or a single writer.

## 7 Make freshness truthful

Track and display when the relevant fact was observed. A newly generated page does not make an old task report fresh.

Distinguish these concepts where they exist:

- **Source observed:** when a check, repository query or runtime observation established the fact.
- **Agent reported:** when the agent supplied its activity summary.
- **View refreshed:** when the display read those records.

The minimum view must show the relevant observation time and whether activity is reported or directly observed. Extra timestamp labels belong in details when they add clutter.

Without execution telemetry, use “Last reported activity” and “Current execution not observed.” Do not infer that an agent is running, paused or stopped from a checkpoint alone. Missing updates should become an explicit freshness uncertainty.

If supported runtime hooks are available, record actual start, completion, failure, waiting and stop events at their existing boundaries. Heartbeats can indicate a connected or observed session; they do not by themselves prove which task is progressing.

Choose any staleness threshold for the actual update mechanism and show what it means. Do not turn a long-running check into a false failure merely because it has produced no new output.

A local record and a remotely published overview have different visibility delays. State the actual update boundary. For example, a repository page may contain the last published checkpoint, while the local agent has newer work. A polling timer that rereads the same stale file does not solve that gap.

For conflicting observations, reconcile the particular fact against its appropriate source. Repository integration status comes from current repository evidence; an execution claim needs a runtime observation. Retain the origin and limits of agent-reported verification. Do not silently promote a prose claim into an independently observed result.

## 8 Permanent repository rule updates

Update existing equivalent sections where possible. Keep AGENTS.md concise and link to the detailed workflow. Do not append a second set of overlapping rules.

### Compact AGENTS.md section

Merge the following intent into existing sections and remove equivalent repetitions. Adapt its links to the final maintained workflow; it is not an instruction to append another copy of already enforced rules.

~~~markdown
## Development context and visibility

Follow the development workflow through one current task record and its owner-facing view. Keep the existing lead and preserve applicable authority, resource limits and verification requirements.

On startup or resumption, read the effective repository instructions and current task handoff, reconcile relevant Git and execution facts, and load specialist guidance for the task. Consult historical receipts to establish specific claims.

At meaningful changes, record the outcome, reported activity, next action, blocker and evidence with their observation time. Preserve failed findings and changed approaches. Bind verification to the examined revision. Distinguish reported activity from observed execution and implementation from verification, integration and acceptance.

Keep small tasks small. Maintain one owner for each rule and current fact; link detailed procedures and examples rather than duplicating them. Leave a restartable checkpoint at pauses and handoffs, and continue independent eligible work within current authority.
~~~

### Development workflow additions

Reconcile the existing workflow with the durable obligations in sections 2, 3, 6 and 7. Preserve its current verification and integration rules, and link implementation examples instead of copying entire sections of this report.

The resulting workflow must explicitly define:

1. How a new session locates and reconciles current work.
2. Which meaningful transitions cause record updates.
3. How dependencies and resource waits affect readiness.
4. What a check or review loop records.
5. How evidence remains attached to the examined revision.
6. How completion and integration update the owner view.
7. What a pause or handoff preserves.
8. Which source and update mechanism support the view's freshness claims.
9. How the agent selects task-relevant guidance and evidence while retaining applicable constraints.

Keep one maintained description. Link detailed examples rather than copying the whole plan into startup instructions.

### Strategic programming additions

Add the following principles to the existing responsibility and complexity guidance:

- Development visibility is a useful interface over agent work. Task interpretation and strategy stay with the agent; tools record execution facts and presentation makes them understandable.
- Prefer the current tracker, state records and supported execution hooks. A prompt or skill can define reporting behavior; durable recording and precise state presentation can justify small shared code.
- A dependency view describes work relationships. It does not require a new scheduler, graph database or prescribed reasoning path.
- Evaluate the implementation by whether the owner can understand and resume real work with less coordination effort. Remove duplicate reporting routes that it replaces.

Keep the existing model selection policy in force. These instructions should work with any sufficiently capable development agent; they do not depend on a particular model or introduce a new provider.

## 9 Implementation sequence

### First batch Reconcile and adopt

Inspect the actual instruction chain, installed development skills, current assignment, task records, supported agent interface and relevant Git and check state. Retain the existing lead and preserve local changes and in-flight work.

Make a small ownership and reading map in the existing task record: essential startup guidance, current work, task-specific procedures and historical evidence. Identify duplicated explanations, broad triggers and competing current-state claims. Choose the existing record that will supply the view and reconcile relevant facts against their sources.

Consolidate AGENTS.md, the development workflow and strategic programming guidance. Narrow clearly overbroad reading requirements and skill triggers within scope. Preserve historical results, essential reminders and actual constraints. Start using the cycle immediately on the existing assignment.

**Completion:** normal startup instructions lead to the correct current task and applicable guidance; obsolete blanket reads and conflicting current-state claims are retired; the active task has a useful outcome, next action and evidence trail.

### Second batch Deliver the smallest useful view

Reuse an existing owner-accessible task surface when it can display the required information. Otherwise, the default first delivery is a rendered overview at a stable repository location, such as docs/work/overview.md, generated or consistently projected from the selected record.

Include Working on, Next, Needs attention and Completed, with a small dependency graph when useful and links to task and loop details. State whether the data is a local observation or the last published checkpoint.

If the current interface supports a thin read-only screen, reuse its existing components and access path. Do not make a separate hosted service a prerequisite for visibility.

Provide the owner with the actual path or link they can open. A chat description or disconnected mockup does not complete this batch.

**Completion:** the view is reachable, reflects real work, exposes freshness and evidence, and can be refreshed through the normal workflow.

### Third batch Connect reporting to ordinary work

Reuse supported execution events for exact facts when available. Keep semantic activity summaries with the working agent.

Prefer an existing accessible surface that can expose work in progress before merge. If the available surface only publishes completed batches or checkpoints, deliver it with that limitation stated explicitly. Describe it as the last published report rather than the agent's exact current activity.

Refresh the view at meaningful transitions and the existing publication or checkpoint boundary. Do not create a commit for every tool call. If the remote surface updates only at batch completion, make that delay explicit.

Ensure a new task and a resumed session use the same process through permanent instructions. Keep existing lead ownership, schedules, expiry, quota and build coordination. No duplicate heartbeat or background orchestrator is needed.

**Completion:** the view changes as ordinary work changes, and a new session can continue the same task without constructing a competing plan.

### Fourth batch Verify and continue

Exercise the acceptance scenarios below with current work and a small synthetic record where appropriate. Reuse retained failures and review history instead of causing a real expensive failure for demonstration.

Review any new adapter or renderer in proportion to its consequences. Documentation-only changes use the repository's applicable documentation checks; they do not justify a broad product rebuild.

Report the result and continue the next eligible development task. Do not keep expanding the visibility feature after its useful outcome is established.

## 10 Acceptance scenarios

| Scenario | Required behavior |
| --- | --- |
| Ordinary work | From the view alone, the owner can identify the goal, last reported activity, responsible agent, next action and evidence. |
| Failure and correction | A failed check remains visible with what changed and the next relevant result. Editing code alone does not mark the task complete. |
| Dependency | Waiting work identifies its prerequisite and becomes eligible when the required condition is actually met. |
| Parallel work | Independent tasks show separate responsibility. A shared build wait appears as a resource constraint. |
| Pause or missing signal | An old report retains its time and becomes visibly uncertain about current execution. It never remains falsely live. |
| Changed revision | Evidence from a previous revision is preserved with its original scope. Affected claims on the new revision require reassessment. |
| Integration | The view distinguishes a candidate that passed checks from a merged change and from verification of the integrated result. |
| Update failure | Where an adapter or refresh mechanism exists, preserve the last valid record and expose refresh failures that mechanism can observe. A static published overview retains its underlying observation time and identifies its publication boundary. |
| New session or assignment | Normal startup instructions lead the next agent to the record, rules and next action. The practice survives the campaign ending. |
| Small documentation task | A simple documentation fix finds the applicable instructions and uses a concise record and relevant checks. It does not load unrelated runtime playbooks or native procedures unless an actual requirement applies. |
| Instruction change | A resumed ordinary task follows the revised reading route, preserves applicable constraints and identifies the correct next action without reconstructing the full campaign history. |
| Skill scope | Representative applicable and inapplicable tasks exercise any changed trigger. Development configuration cleanup does not silently change BRN's shipped runtime prompts. |

Use a few targeted fixtures to cover display states that cannot be exercised safely in current work. Observe instruction usefulness during ordinary authorized tasks and use the relevant skill validation workflow for any actual skill edits. Do not claim measured token savings or model-quality improvement without usage and outcome evidence. No new benchmark service or broad audit program is required.

Owner acceptance remains an actual observation or response. The development agent should demonstrate how to open and read the view without claiming the owner has accepted it in advance.

## 11 Audit baseline and migration caution

The 19 October 2026 repository audit inspected main at [9cecd00b21c1fc1084c402ab9ffe7aa6aea4bff6](https://github.com/ewq100/brn-rust/commit/9cecd00b21c1fc1084c402ab9ffe7aa6aea4bff6). The documented startup route, if followed literally, covered these files before source and architecture material:

| Inspected file | Whitespace-separated words |
| --- | ---: |
| [AGENTS.md](https://github.com/ewq100/brn-rust/blob/9cecd00b21c1fc1084c402ab9ffe7aa6aea4bff6/AGENTS.md) | 804 |
| [Status](https://github.com/ewq100/brn-rust/blob/9cecd00b21c1fc1084c402ab9ffe7aa6aea4bff6/docs/status.md) | 604 |
| [Campaign plan](https://github.com/ewq100/brn-rust/blob/9cecd00b21c1fc1084c402ab9ffe7aa6aea4bff6/docs/work/active/vacation-campaign/plan.md) | 2,252 |
| [Checkpoint](https://github.com/ewq100/brn-rust/blob/9cecd00b21c1fc1084c402ab9ffe7aa6aea4bff6/docs/work/active/vacation-campaign/checkpoint.md) | 537 |
| [Campaign state](https://github.com/ewq100/brn-rust/blob/9cecd00b21c1fc1084c402ab9ffe7aa6aea4bff6/docs/work/active/vacation-campaign/campaign.json) | 901 |
| [Evidence](https://github.com/ewq100/brn-rust/blob/9cecd00b21c1fc1084c402ab9ffe7aa6aea4bff6/docs/work/active/vacation-campaign/evidence.md) | 4,101 |
| **Total** | **9,199** |

These counts include Markdown, identifiers and paths. They describe the required reading route, not measured token use or proof of model overload. The audit did not observe the Mac's effective global instructions, installed personal skills or actual session context.

The stronger finding was conflicting current-state ownership. The campaign plan opens at PR172, status opens at PR174, and evidence begins with a PR163/164 snapshot that says it controls the next action. The plan also assigns current jobs to the checkpoint and current candidate/check state to the evidence record. Consolidate those claims while retaining their historical observations.

The inspected status and checkpoint described PR175's final review as pending, while [PR175](https://github.com/ewq100/brn-rust/pull/175) was already merged. [PR176](https://github.com/ewq100/brn-rust/pull/176) was open at inspection and reported source review, local checks and actual Mac accessibility observations. Those reports did not establish its current hosted check state or owner acceptance.

Refresh relevant facts when adopting the plan. Do not reopen completed Note reading work or redo the accessibility implementation solely because an older summary calls it pending.

The first reconciliation should establish actual remaining work. The same rule applies throughout future development: refresh the relevant fact and preserve the scope of its evidence.

## 12 Required handoff from the implementing agent

When the first useful version is complete, report:

1. Which permanent instruction files changed, what duplicated reading was retired and how future sessions discover the cycle.
2. Which record owns current work, how helpers report and how duplicate status is avoided.
3. The actual link or path where the owner opens the view.
4. What the view can observe directly, what is agent-reported and when published information refreshes.
5. Which acceptance scenarios were exercised and what remains unverified.
6. The resulting startup reading route, remaining local/global instruction conflicts and one short reusable session prompt.
7. The current task and next useful action after adoption.

Include a real task with its dependency or repair history where available. Continue through the existing development lead and normal workflow.

## 13 Prompt to give the build agent

Use the exact report commit or pull request supplied in the owner's notification. The repository home for this report is docs/work/active/agent-first-simplification/visible-development-cycle.md.

~~~text
The report at docs/work/active/agent-first-simplification/visible-development-cycle.md has been updated to combine visible development with prompt, skill and instruction optimization.

Fetch and read the supplied report revision. Preserve the current lead, local changes and in-flight work; reconcile the report against current instructions, Git and task evidence. Its publication is an updated implementation brief, not proof that the rules or owner view are already implemented.

Inspect the actual development-agent instruction chain and skills. Consolidate repeated rules and stale current-state claims, establish one current work record, and load specialist guidance only when relevant. Keep development configuration separate from BRN's shipped runtime prompts.

Integrate concise permanent guidance into AGENTS.md, docs/development/workflow.md and docs/development/strategic-programming.md. Keep this report as reference rather than another mandatory startup pack. Preserve existing authority, resource limits and verification requirements.

Deliver the smallest useful owner view over the existing work record, with truthful freshness, next actions, dependencies, correction history and evidence. Give me its actual path or link.

Use proportionate validation, report the changed reading route, remaining limitations and a short reusable session prompt, then continue the next eligible task. Do not restart completed work.
~~~

