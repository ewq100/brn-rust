# Development overview

**Goal:** Complete Windows CI compatibility and merge through the normal checked PR path.
**Owner attention:** The owner explicitly authorized continuing until fixed and then merging. Current main PR244 is integrated; no live providers, account settings, check bypass or release is authorized.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/windows-ci-portability/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

### Qualify and integrate Windows compatibility

Task ID: `windows-ci`
Owner: Windows compatibility lead · State: **active** · Last observed: 2026-11-04T17:11:11.221486+02:00
Activity: Reviewed eight-runtime-failure correction is ready:45focused local tests passed (input4, CLI23+search2, backup5, intake1, resend7, guides3), strict affected CLI/core/intake all-target lint passed, and synthetic autocrlf checkout verified9 embedded files. Windows native UI passed on701d63cf; new exact-head qualification follows publication.
Next: Publish the reviewed fixture/security correction; require normal PR checks and all three Windows lanes on the exact head, then normal merge and resulting-main verification.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Independently reviewed portability/security changes preserve shared Unix workflows and exact byte/replay protections; no production retry broadening or portable test removal.
- **checks:** 45focused runtime tests and affected all-target strict lint passed; formatting, docs and synthetic autocrlf checkout passed. Earlier72CLI/helper tests remain evidence. New exact-head PR plus Windows3 hosted results still pending.
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

Local page reflects the last saved observations in this record (2026-11-04T17:11:11.221486+02:00). A remote page reflects the last published commit. Neither indicates that work is live now.
