# Development workflow

## Start and scope

Inspect branch, HEAD and working-tree changes. Read [AGENTS.md](../../AGENTS.md), [status](../status.md) and relevant architecture/crate documentation. Identify acceptance criteria and checks before editing. Follow current user authorization; historical task permissions, named models and agent assignments are evidence, not standing requirements.

For a task needing a durable plan, create `docs/work/active/<task-name>/plan.md` and `evidence.md`. A small fix can use a concise change description and verification report without manufacturing a large plan. Use stable descriptive names; the old chunk-02 label was used for both the provider lifecycle follow-up and editor roadmap item, so avoid relying on chunk numbers alone.

## Plan contents

- Status, date and baseline commit/branch.
- Intended outcome, user requirements, included scope and exclusions.
- Relevant decisions, invariants, prerequisites and affected crates.
- Concrete implementation steps and acceptance criteria.
- Verification commands and native/resource requirements.
- Unresolved decisions or blockers and next action.

## Evidence and handoff contents

- Date, tested commit and any uncommitted changes; environment and features.
- Commands, outcomes and meaningful observations; links to durable artifacts when available.
- Separate automated tests, native observations, user acceptance and remaining qualification.
- Files/behavior changed, unresolved failures and risks.
- Next concrete action; branch, commit, PR/merge state if actually checked.

Temporary machine paths are reproduction context, not durable artifacts or portable instructions. Do not record tokens or raw authentication data. Keep critical observations in the repository even when a temporary log is referenced.

## State and completion

Track implementation (`pending`, `active`, `blocked`, `implemented`), verification (`not run`, `partial`, `verified`), acceptance (`pending`, `accepted`, `deferred`) and integration (branch/commit and checked merge state) separately. Report acceptance only with evidence of the relevant approval.

When the task's implementation scope is complete and evidence is recorded, move its folder to `docs/work/completed/<task-name>/`, repair links, and update the work indexes and current status. Completed means the bounded task record is closed; it does not mean all product qualification or release acceptance is complete. Carry unresolved gaps into current status or a follow-up task.

Keep [status](../status.md) as a concise present-tense summary, and [roadmap](../roadmap.md) as future outcomes. Update architecture and crate READMEs when responsibilities or contracts change. Preserve historical evidence and dated decisions; add a correction or superseding reference rather than rewriting a past observation as a current result.

## Documentation checks

Before handoff, run `git diff --check`, validate local Markdown links (including heading fragments), search for removed paths, and check that every moved record appears in an index. Verify command examples against scripts/manifests. Documentation-only changes do not require rebuilding unchanged Rust code.
