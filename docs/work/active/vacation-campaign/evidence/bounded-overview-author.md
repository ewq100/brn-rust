# Bounded completed-task overview candidate

Candidate `ddf07b974a268e8dcf6eeef13a50fd277114f43a` is committed on `codex/bounded-completed-overview-20261022`, based on clean `5c606c102d5d121105896c48dc7b39e78d26bdb3`. The worktree is clean. Only the owned renderer, its existing test file and this worker branch's generated `docs/work/overview.md` changed; the campaign record and lead checkout were not edited.

The Completed section now reports the total and displays at most ten tasks sorted by parsed timezone-aware `observed_at` descending. Equal instants retain deterministic ascending ID order. Its wording identifies these as last-observed/last-updated timestamps rather than completion timestamps, since the task schema has no `completed_at`. The shared header links the full selected record and explains how to find older IDs by title or ID and retrieve a validated exact object with the existing `--task` route. Validation, eligibility, current task summaries, gates, waits and dependency graph logic remain unchanged.

The existing completed-index test was expanded to twelve fixtures spanning UTC offsets, a lexical-vs-instant ordering case and same-instant ties. Before the renderer change, the focused test failed at the exact row assertion: the old projection returned all twelve in record order. The final regression checks the ten expected rows, total count, truthful time description, omitted older IDs, full-history route, and exact `--task` retrieval for an omitted older task while confirming record and already-generated output bytes are unchanged. Existing tests continue to cover current facts and custom record/output routing.

For the worker's own 31-task record (27 complete), generated overview size decreased from 175 lines / 10,565 bytes to 160 lines / 9,432 bytes, and the Completed rows from 27 to 10. This size comparison is specific to the worker snapshot; the newer lead record was not used or changed. All full task records remain authoritative and retrievable.

Checks run:

- Before source change: focused regression failed at the intended unbounded-row assertion.
- Final `python3 -m unittest scripts.tests.test_work_overview`: 15 passed.
- `python3 scripts/render-work-overview.py`: regenerated only the worker overview.
- `python3 scripts/render-work-overview.py --check`: passed.
- `python3 scripts/check-markdown-links.py`: 625 files, 2,813 local links, zero failures.
- `git diff --check`: passed.

No Rust, network, account, native or full script-suite checks were run. The view remains a projection of its selected record and does not assert completion acceptance, native observation or live activity.
