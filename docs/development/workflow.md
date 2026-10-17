# Development workflow

[AGENTS.md](../../AGENTS.md) routes assignments. The [Threads target](../architecture/threads-target.md) owns requirements; the current snapshot in [status](../status.md) locates task records. Current user instructions supply the assignment; an old active-plan label or a proposed sequence does not start work.

## Work and review

Confirm branch/HEAD, remote state and unrelated changes. Use an isolated checkout and fresh synthetic data. Resolve routine implementation choices within current authority. Keep one short active plan and concise status; isolate a real consequential ambiguity while continuing independent work.

Before changing a meaningful component, inspect current source, pinned dependencies and suitable maintained/platform facilities. Choose reuse, adaptation or small custom implementation based on total glue, validation and maintenance. Preserve useful provider, converter, retrieval and dependency fixes.

Apply the [strategic programming guide](strategic-programming.md): identify the shared owner and relevant callers, choose a boundary that reduces their knowledge, implement the requested outcome, and retire replaced routes in the same scoped change. For a meaningful new boundary or competing designs, record a short choice and its tradeoff in the existing plan. A small repair needs no separate design document.

One lead owns shared interfaces and integration. Helpers receive bounded tasks, file/worktree ownership and evidence requirements. Use the owner's selected model. Independently review plans and meaningful code/deletion changes proportionately; a review does not authorize unrelated features. Skills support the assignment without adding routine approval gates.

The reviewer assesses design as well as correctness: repeated policy, state ownership, unnecessary caller knowledge, speculative abstraction, removed paths and test effectiveness. Resolve findings within scope. Record consequential deferred debt with its affected boundary, impact and concrete revisit trigger in the existing task record; a bare TODO is insufficient.

## Verify and integrate

Choose relevant checks from [verification](verification.md). Deterministic integrity tests, representative provider runs and actual native interaction prove different things. Record tested tree/features, exact commands, results and limits. Repeat broad checks only after a relevant change or unresolved failure; preserve unsuccessful evidence.

Commit task-owned changes in reviewable units, coordinate separate writers before pushing, and publish within current authority. Fetch and reconcile shared history; never force-push or bypass protection/required checks. Merge and release follow the applicable assignment. When integration is authorized, compare the integrated tree with the verified candidate and inspect applicable main checks. Preserve and diagnose a failing required main check; a passing rerun alone does not establish that an intermittent defect is fixed.

After integration, reconcile the current status snapshot and task state with the merge SHA, linking historical receipts without rewriting their results. Keep proposals explicitly proposed until assigned. At a milestone or a recurring defect, use the small health review in the strategic guide to choose the next justified improvement; do not start an unrelated audit program.

Report implemented, verified, native-observed, owner-accepted and merged separately, with the remaining gate and next action. Passing compilation does not establish a usable product.
