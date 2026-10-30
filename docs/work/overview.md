# Development overview

**Goal:** Integrate the reviewed subscription-search fixes and preserve separate native acceptance gaps.
**Owner attention:** PR244 merge is requested. The pre-turn correction is committed and independently reviewed;35 provider tests,3 private-file tests and strict affected lint/build pass. Push starts CI. Confirm included Actions capacity or a paid CI bound to resolve the earlier exhausted-minutes hold. Native acceptance stays pending.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/subscription-search-mac-acceptance/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

None recorded.

## Next

None recorded.

## Needs attention

### Observe actual Mac controls

Task ID: `native-mac`
Owner: Existing Mac lead · State: **blocked** · Last observed: 2026-10-30T07:42:45.460648+00:00
Activity: Label repair and page/Capture/dialog harness checks pass; native inventory reported locked.
Next: Run one bounded synthetic native session only when owner makes native access available.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** reviewed scoped candidate
- **checks:** 24 page +3 Capture +3 search harness tests
- **integration:** draft PR244; not merged or released
- **native:** blocked; no native apps exposed
- **owner acceptance:** not claimed

</details>

### Integrate reviewed fixes through required CI

Task ID: `publication`
Owner: Existing Mac lead · State: **blocked** · Last observed: 2026-10-30T23:32:00.866195+00:00
Activity: Pre-turn discard defect reproduced, fixed at 22f3860e and independently reviewed. All affected offline gates pass. No push or new hosted CI; budget clarification pending.
Next: After owner confirms included Actions capacity or a paid CI bound, push the reviewed local commits to existing PR244, verify exact-head required CI, mark ready, merge normally and verify resulting main.
Resource waits: Actions capacity or bounded paid-CI authorization

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** PASS: complete candidate and pre-turn guard independently reviewed; new guard has offline evidence only.
- **checks:** PASS:35 provider+3 private-file tests, strict AI/application/desktop lint, example build, formatting and docs checks. Required exact-head hosted CI pending.
- **integration:** Owner-requested normal merge pending; all corrections/evidence remain local. Actions budget clarification required before push; no release.
- **native:** Known Mac UI gap remains pending; integration request does not establish native acceptance.
- **owner acceptance:** not claimed

</details>

Waiting:

- resource: Actions capacity or bounded paid-CI authorization

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

2 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `subscription-search` — Qualify subscription-only search
- `document-workflow` — Import knowledge and produce a referenced draft

Local page reflects the last saved observations in this record (2026-10-30T23:32:00.866195+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
