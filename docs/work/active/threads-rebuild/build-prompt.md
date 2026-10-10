# BRN Threads build agent prompt

Give the following to the build agent when ready to start. Sending this prompt assigns implementation; the existence of this file alone does not start a build.

```text
Build BRN Threads in https://github.com/ewq100/brn-rust on branch rebuild/threads.

Fetch the current remote branch and use an isolated checkout/worktree. Preserve other branches, dirty work, old data, and existing checkouts. Do not start from main or reset another task. If rebuild/threads is already checked out, inspect and use that worktree safely.

Read AGENTS.md, docs/status.md, docs/architecture/threads-target.md, and docs/work/active/threads-rebuild/plan.md. Read review.md and resolve any actual review findings. The Threads target supersedes old frozen-architecture, Markdown-authority, blanket-approval, original-retention, and V1-stage instructions.

I authorize implementing the first usable release in that plan, including substantial refactoring/replacement of production code and tests where needed. I am not using the app and do not need gradual migration, dual writes, or a permanent old engine. Routine implementation choices are delegated. Continue through the milestones without repeatedly asking whether to proceed.

First verify this machine's prerequisites using docs/development/setup.md. Use fresh explicit app data outside Git and a separate task-owned build target. Do not inspect or migrate existing BRN user data.

Resolve the narrow core contracts and create the dependency/reuse/deletion map, then build the final-use smaller core. Prove atomic grouped changes, idempotent retry, protection/authority, expected versions, active-writing coordination, backup/restore, and Undo preserving later edits before relying on broad autonomous writes.

Keep both intake choices: useful information and full readable notes for key documents. Full notes retain substantive wording/structure, tables/equations/figures/references, start protected, and show gaps honestly. Raw originals remain external. Markdown export is on demand. DOCX/PPTX/HTML generation remains later scope.

Reuse working GPUI, provider, converter, and retrieval capabilities. Put semantic behavior in agent tools/instructions and deterministic integrity in one shared application service. Do not introduce Jujutsu, CRDTs, universal converters, another approval framework, or a sandbox platform without a concrete need that makes the implementation simpler.

Use the selected build model and bounded subagents for independent tasks. If the external review is pending, use one available independent review where useful; do not wait for a particular brand/model or run repeated whole-plan redesigns. Validate findings and fix supported issues with the smallest correction.

I authorize bounded synthetic/public provider smoke and journey tests using the already configured selected provider/account/model. Check availability early. If owner interaction is needed for login, tell me promptly and continue offline work. Do not print credentials, inspect private mailbox content, switch accounts/providers silently, use paid fallback, send external messages, or release the app.

Implement the native daily journeys and verify them on the target Mac when available. If the Mac is locked or inaccessible, do not try to unlock it; collect one clear native interaction task and continue headless work. Distinguish code that exists from behavior actually verified and from my acceptance.

Commit and push task-owned milestones to rebuild/threads, preserve required checks, and maintain the existing rebuild draft PR if one exists. Reconcile new remote work without force-pushing. Do not merge into main or publish a release from this instruction.

Continue until a usable first-release candidate and its acceptance evidence are ready, or a specific external blocker prevents further independent progress. Update docs/status.md and the rebuild plan/evidence with the actual tested commit, results, limitations, and next action at each handoff.
```
