# Completed-index growth preflight

## Finding

The renderer is still behaving as designed: PR198 removed duplicated completed-task gates/evidence/attempt details; PR213 removed raw evidence and attempt history from active/blocked summaries, while deliberately retaining every completed stable ID/title for discovery. Those decisions remain sound, and there is no case to restore full completed-task blocks or trim authoritative records.

There is now a distinct, measurable projection cost in the retained index. At lead checkout HEAD `7b444df64c577dfac09b48715ec82903f00a1703`, the selected campaign has 128 tasks (120 complete, 7 blocked, 1 active). `docs/work/overview.md` is 400 lines / 30,802 UTF-8 bytes; the 120-row Completed section alone is 122 lines / 11,314 bytes (36.7% of the page). In the `3827d6a1` PR213 resulting-main snapshot, the index had 27 rows / 2,381 bytes and the full overview was 10,565 bytes. Thus the compact index grew about 4.8× in bytes as the campaign accumulated completed assignments. This is file/output volume, not a measured token or human-reading cost.

## Smallest useful follow-up

Consider replacing the unbounded default completed index with a bounded “most recently observed completed” list (for example, 10 rows) plus the total completed count and a clear full-history route to the linked selected record. The record and all `--task` lookup behavior stay authoritative; older IDs/titles remain searchable there. Keep the global selected-record link and completion caveat. Label any recent slice by `observed_at` rather than implying a completion timestamp: the record has no separate completion-time field. Preserve active/blocked full current facts, ID visibility, waits, gates, eligibility and dependency graph.

The existing seams are sufficient: `scripts/render-work-overview.py:184-199` owns the shared detail route and section projection; `scripts/tests/test_work_overview.py::test_completed_tasks_use_compact_index_and_link_selected_record` covers the compact completed presentation, and the existing `--task` tests verify exact retrieval and input/output preservation. A focused regression should use more completed tasks than the display bound with deliberately ordered timestamps, assert the total and stable newest-observed slice, assert the full-record route, and prove an older omitted task remains exactly retrievable through `--task`. Then run the overview Python suite, selected-record/custom-output CLI checks, regenerate only the overview from the selected record, and run `--check` plus documentation link checks.

## Limits and preservation

This is a read-only assessment, not a selected implementation. The completed IDs/titles remain useful for direct discovery; bounding them trades that convenience for smaller default output, so an explicit full-history route is necessary. No repository edits, tests, builds, network calls or other workloads were made. The lead-owned untracked Note-exit receipts observed in the checkout were preserved.
