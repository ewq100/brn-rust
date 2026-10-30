# Development overview

**Goal:** Finish subscription search and Mac acceptance with source-bound evidence.
**Owner attention:** Document workflow passed. Search failed initially and corrected source remains live-unqualified. Native Mac gate blocked. PR244 is draft, not merged.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/subscription-search-mac-acceptance/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

None recorded.

## Next

None recorded.

## Needs attention

### Qualify subscription-only search

Task ID: `subscription-search`
Owner: Existing Mac lead · State: **blocked** · Last observed: 2026-10-30T07:42:45.460648+00:00
Activity: Initial query failed; corrected adapter passes18 offline tests, not live-qualified.
Next: Owner review and a fresh explicit bound before another search attempt.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** reviewed scoped candidate
- **checks:** effective zero-turn isolation; strict lint; initial failure preserved
- **integration:** draft PR244; not merged or released
- **native:** dialog harness only; real Mac unobserved
- **owner acceptance:** not claimed

</details>

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

### Publish final evidence and inspect CI

Task ID: `publication`
Owner: Existing Mac lead · State: **blocked** · Last observed: 2026-10-30T07:42:45.460648+00:00
Activity: Draft PR244 exists and tracks the reviewed scoped candidate; no merge or release. Exact-head CI is externally authoritative at the PR checks page.
Next: Inspect final exact-head checks and retain the draft until the outstanding search/native gates and any separate owner merge decision.
Resource waits: search safety hold / fresh explicit test bound; native Mac access

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** reviewed scoped candidate
- **checks:** offline focused tests and strict lint passed; inspect exact-head CI at PR244
- **integration:** draft PR244; not merged or released
- **native:** native gate separate
- **owner acceptance:** not claimed

</details>

Waiting:

- resource: search safety hold / fresh explicit test bound
- resource: native Mac access

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

1 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `document-workflow` — Import knowledge and produce a referenced draft

Local page reflects the last saved observations in this record (2026-10-30T07:42:45.460648+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
