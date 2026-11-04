# Development overview

**Goal:** Complete Windows CI compatibility and merge through the normal checked PR path.
**Owner attention:** The owner explicitly authorized continuing until fixed and then merging. Current main PR244 is integrated; no live providers, account settings, check bypass or release is authorized.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/windows-ci-portability/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

### Qualify and integrate Windows compatibility

Task ID: `windows-ci`
Owner: Windows compatibility lead · State: **active** · Last observed: 2026-11-04T15:56:15.474343+02:00
Activity: Integrated main098dc556; preserved portable directory protection and writing/reopen tests, joined the explicitly owned test worker before index deletion, and corrected PR244 Windows cfg lint.
Next: Publish the reviewed combined candidate; require full Windows core/UI/retrieval and normal PR checks before merge, then inspect resulting-main CI.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Reviewed platform boundaries retain Unix security; portable foreign-data and writing metadata/reopen assertions retained.
- **checks:** Focused storage/index/writing tests and integrated workspace strict lint passed locally; new exact-head hosted qualification remains required.
- **integration:** Normal PR merge explicitly authorized after passing checks; GitHub PR/merge state owns the integration result. No bypass.
- **native:** Windows native UI job passed on7c01e10d and Linux/Mac supported jobs passed onf3fd36f8; no new real UI interaction or feature parity claim.
- **owner acceptance:** No new product acceptance claim; secure credentials and exclusive model installation remain explicitly unsupported on Windows.

</details>

## Next

None recorded.

## Needs attention

None recorded.

## Completed

None recorded.

Local page reflects the last saved observations in this record (2026-11-04T15:56:15.474343+02:00). A remote page reflects the last published commit. Neither indicates that work is live now.
