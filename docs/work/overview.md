# Development overview

**Goal:** Complete Windows CI compatibility and merge through the normal checked PR path.
**Owner attention:** The owner explicitly authorized continuing until fixed and then merging. Current main PR244 is integrated; no live providers, account settings, check bypass or release is authorized.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/windows-ci-portability/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

### Qualify and integrate Windows compatibility

Task ID: `windows-ci`
Owner: Windows compatibility lead · State: **active** · Last observed: 2026-11-04T16:29:18.639547+02:00
Activity: PR245 e468175b passed Mac/Linux supported checks and Windows native UI; Windows Core exposed two unused Unix-only locator helpers and an unguarded POSIX permission fixture. Corrected those and closed unqualified Windows token-output privacy fallback; focused CLI70 tests and strict lint passed.
Next: Publish the reviewed fixture/security correction; require normal PR checks and all three Windows lanes on the exact head, then normal merge and resulting-main verification.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Reviewed corrections preserve Unix protections and portable edit/reopen/privacy coverage; Windows token-out now refuses before effects while authentic inline issuance remains supported.
- **checks:** Integrated workspace strict lint, writing22/subscription35/focusedCLI70 and tooling320 passed locally; exact-head hosted Windows qualification is still required.
- **integration:** Normal PR merge explicitly authorized after passing checks; GitHub PR/merge state owns the integration result. No bypass.
- **native:** Windows native UI job passed on7c01e10d and Linux/Mac supported jobs passed onf3fd36f8; no new real UI interaction or feature parity claim.
- **owner acceptance:** No native interaction or feature parity claim; secure credentials, exclusive model installation and owner-only token-output files remain explicitly unavailable on Windows.

</details>

## Next

None recorded.

## Needs attention

None recorded.

## Completed

None recorded.

Local page reflects the last saved observations in this record (2026-11-04T16:29:18.639547+02:00). A remote page reflects the last published commit. Neither indicates that work is live now.
