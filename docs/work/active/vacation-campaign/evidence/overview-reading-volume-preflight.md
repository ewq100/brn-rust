# Overview reading-volume preflight

## Finding

The generated owner view is 1,590 lines / 158,282 characters at source HEAD `3810e355e42ab6c824b0ccac951179d55608ae69` in `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust` (`codex/agent-first-followthrough`). The on-disk overview has the same 1,590-line size. Current `work_view` contains 66 tasks: 2 active, 3 blocked, 0 ready, and 61 complete. The renderer puts 1,416 lines in `## Completed`; those rows repeat all gates, up to 388 evidence links and 46 correction-attempt summaries that already remain in the authoritative `campaign.json` and immutable evidence receipts. This is the source of the startup reading volume, not an oversized current-work section.

At task-local detail granularity, the existing projection already gives the lead the right information: the `Working on` and `Needs attention` sections are about 136 lines total, plus the concise dependency graph and introduction. Replacing the 61 full completed-task blocks with one-line ID/title entries would leave the generated file around 240 lines while retaining a task index. Do not keep the detailed blocks merely inside collapsed `<details>`: that hides them visually but leaves their text in the startup file.

## Smallest correction

Change only presentation in `scripts/render-work-overview.py`:

- Continue validating the **entire** `work_view` before rendering. Continue deriving task eligibility and the dependency graph from all 66 original task objects, including completed prerequisites; do not filter the record before `validate`, `unmet`, or graph construction.
- Keep complete blocks with owner, activity, next action, true dependencies/resource waits, all five gates, evidence links and bounded attempts for visible actionable/current tasks (`active`, `ready`, `blocked`/waiting), preserving the current eligibility and honest-gate wording.
- Replace each completed task's full `_task_block` with one compact index row containing its stable task `id` and title. Add one direct link and sentence for the canonical [campaign record](../campaign.json): completed task facts, gates, attempts and evidence remain there and in linked receipts; search the listed ID to open the task object. The existing test record is a single file with IDs, so this adds no second tracker or copied history.
- Adjust `docs/status.md:3` from promising completed "results" in the generated view to describing a completed-task index with detailed records in the campaign/evidence. `AGENTS.md:8-9` and `docs/development/workflow.md:13,19` already direct readers from the overview to the selected task and make `campaign.json` authoritative; no broader startup-route rewrite is needed.

For the present 61/2/3 distribution, that leaves a brief completed index and roughly 240 rendered lines, down from 1,590. The benefit comes from not duplicating immutable historical detail in the startup projection; no record or receipt is deleted or rewritten. Preserve the current footer's local/remote observation limit and the graph's reported dependency satisfaction.

## Existing regression seam and risk

`_task_block` at `scripts/render-work-overview.py:159-173` emits the repeated history. `render` selects every complete task at `:195-220`, while full dependency validation and graph construction live at `:56-148,176-189,221-232`. The relevant existing test is `scripts/tests/test_work_overview.py::test_completed_tasks_keep_gates_evidence_and_attempts_in_collapsed_details` (`:158-170`); it encodes the current verbose completed projection and should become the regression for compact completed IDs/titles, the direct canonical-record route, and omission of full attempts/gates/evidence from that overview section while the input record still retains them. Keep `test_render_keeps_observation_static_gates_distinct_and_resources_out_of_graph` (`:80-95`), `test_next_requires_complete_prerequisite_and_no_resource_wait` (`:119-135`), `test_reported_condition_satisfaction_not_prerequisite_state_controls_next` (`:137-149`), and malformed/stale atomic-view tests unchanged in meaning.

The main compatibility risk is confusing omission from the generated view with deletion from the authority record. The compact index plus explicit campaign/evidence route addresses that. A second risk is accidentally filtering completed tasks before dependency evaluation; retain full-record validation and the six actual dependency edges (10 connected nodes in this snapshot). This report does not claim a measured token reduction, remote Page refresh, or change to the underlying task data.

## Scope and preservation

Read AGENTS/workflow routing, the selected `overview-reading-volume` task, renderer/tests and current task counts. Root checkout had pre-existing lead-owned edits in `campaign.json` and `overview.md`, plus untracked `remote-rejected-save-merge-update.json`; all were preserved. No repository files were written; only this `/tmp` assessment was created. No renderer/test/build/account/network/native action was run; line/count assessment used the existing renderer in memory only.
