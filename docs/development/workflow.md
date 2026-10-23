# Development workflow

[AGENTS.md](../../AGENTS.md) is the short entry route. Current user instructions select outcomes; proposed tasks and historical evidence do not assign work. The [Threads target](../architecture/threads-target.md) owns product requirements, with [invariants](../architecture/invariants.md) and [behavior](../architecture/threads-behavior.md) owning their contracts.

## One home per fact

| Fact | Authoritative home | Other views |
|---|---|---|
| Permanent boundaries and startup route | AGENTS.md | Link rather than copy rules. |
| Development cycle, fact ownership and integration | This workflow | Skills supply task-specific procedures. |
| Design rationale, reuse, responsibility and test judgment | [Strategic programming](strategic-programming.md) | Read for meaningful design, not every wake. |
| Campaign scope, expiry and permission limits | [Campaign plan](../work/active/vacation-campaign/plan.md), latest owner instructions | Campaign state points to this authority. |
| Current assignment, activity, next action, dependencies, waits and gates | Selected assignment's current task record: currently `work_view` in [the daily-use record](../work/active/daily-use-reliability/record.json) | [Overview](../work/overview.md) is generated; do not hand-edit it. |
| Current allowance/admission observation | `admission` in campaign.json | Recheck the actual account before substantial dispatch. |
| Machine-specific resumption pointers | [Checkpoint](../work/active/vacation-campaign/checkpoint.md) | No duplicate task status or current PR list. |
| Exact source, checks, failures and qualification limits | Task receipts in [evidence](../work/active/vacation-campaign/evidence.md) | Dated evidence supports claims; it never controls the next action. |
| BRN shipped agent behavior | `agent/` plus application contracts | Development skills/configuration do not silently change runtime prompts. |

`docs/status.md` locates the selected assignment and its authoritative task record; the documentation index is navigation. When the campaign ends, route to the next assignment's ordinary record and update the overview source (`--record`); don't require future tasks to use a vacation file or introduce a second tracker. An observation repeated inside an immutable receipt remains evidence for that time/source, not another editable current record. Git history preserves superseded plans/status; don't erase failed attempts to make a present view look green.

Before opening a large selected record, find the task ID with a targeted search and inspect its validated object using `python3 scripts/render-work-overview.py --task TASK_ID`. This prints only that task; pass `--record PATH` when the selected record is not the default selected record.

## Select, implement, check, correct

Confirm branch/HEAD, remote state and unrelated changes. Read the selected task and its relevant contracts/source/callers. Use an isolated checkout and synthetic data. Apply the [strategic guide](strategic-programming.md) for a meaningful new boundary; an obvious repair needs only a concise approach. Do not impose a spec/ticket chain on small work.

One lead owns shared interfaces, the current record and integration. Give each helper a bounded outcome, file/worktree ownership, exclusions, evidence and stop condition. Respect owner/runtime resource choices; count reviewers and nested helpers in actual capacity. Serialize shared-target builds and desktop/account actors. Reuse relevant unchanged checks.

A task records an observable outcome, owner, state (`active`, `ready`, `blocked`, `complete`), last activity with timezone-aware observation time and origin (`reported` or directly `observed`), next action, real prerequisites, resource waits, evidence and separate gates. `active` means assigned/in progress at its last update; it does not establish a live process. A ready label does not override unresolved prerequisites, capacity or authority. State gates explicitly as pending, passed, failed, deferred or inapplicable with reason; implementation, checks, integration, native observation and owner acceptance are independent.

Implement one reviewable outcome per writer. Independently review meaningful changes, including placement of responsibility, caller complexity, retirement and test effectiveness. Record findings and dispositions. On failure, preserve the original result and record the diagnosis/change before the next attempt. After two failed approaches, reassess cause/scope rather than repeat blindly; stop the loop on the task's limit, uncertain external effects, missing authority or unsatisfied capability. Use existing operation identities and receipts for uncertain effects; a missing response is not proof of no action.

Use dependencies only when an outcome genuinely needs another outcome; record the specific condition and its explicitly reported satisfaction, and validate an acyclic graph. A completed upstream task does not establish every gate or condition; reconcile the actual evidence before marking the condition met. Build slots, quota and desktop access are resource waits, not fake task dependencies. Keep correction attempts in bounded task history and link raw details; do not make retry cycles into dependency edges. Consequential deferred debt names its boundary, impact and revisit trigger.

## Verify and integrate

Select proportionate checks from [verification](verification.md). Record actual source/features, commands, results and limits; preserve failures/ignored cases. Compilation, deterministic integrity, live usefulness, native interaction and owner acceptance prove different things. Do not broaden/repeat gates without a relevant change or unresolved concern.

Commit task-owned changes in reviewable units, reconcile shared history and publish within current authority. Inspect live protection and exact-head required results before an authorized normal merge; never force-push or bypass gates. Compare the integrated production tree with the qualified candidate and inspect applicable resulting-main checks separately. Keep a required-main failure actionable; a passing rerun alone is not a diagnosed repair. Preserve non-required failures visibly.

Update `work_view` at meaningful transitions: selection/assignment, changed readiness, candidate/review result, failed check/correction, merge/main result and real pause/handoff. Save before compaction/transfer; don't log every tool call or refresh timestamps on unchanged facts. Regenerate with `python3 scripts/render-work-overview.py`; verify with `python3 scripts/render-work-overview.py --check` and the documentation checks. Invalid records must leave the last valid page intact and report a refresh failure.

The local overview reflects last saved observations; a remote copy reflects the last published commit. Neither is a live telemetry stream. When the selected task record identifies a remote owner Page, refresh that same concise summary at meaningful campaign transitions after reconciling actual gates. Read its guidance and current blocks first, use guarded edits, and preserve human additions. Show the refresh time and link source records; the Page is a projection, not another editable task authority. A failed refresh leaves the local work intact and is reported as a remote freshness gap. Existing local campaign execution performs these updates; this does not create a filesystem sync or another scheduled writer. Per-task times and reported/observed origin matter more than page generation time. On stale/unknown activity, inspect the actual job/agent/Git state before claiming execution or dispatching a competing writer. Unknown or unavailable capacity is a named wait.

## Instruction maintenance

Use the [instruction audit](../work/active/vacation-campaign/instruction-audit.md) when configuration changes or recurring guidance conflicts occur. Ordinary tasks follow the short route; do not reload the full adoption report, skill inventory or evidence archive. Load the next relevant specialist skill, preserving the owner's selected scope and continuing authorization. Host-provided instructions/catalogue metadata and memory are distinct from editable project rules; local file size is not total injected context or measured token savings.
