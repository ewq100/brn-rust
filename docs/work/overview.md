# Development overview

**Goal:** Finish subscription search and Mac acceptance with source-bound evidence.
**Owner attention:** Headless live search passed on attempt4/10 after three reviewed corrections; allocation closed, six unused. Local code/evidence unpushed; native Mac acceptance and authorized publication/CI remain outstanding.

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

### Publish final evidence and inspect CI

Task ID: `publication`
Owner: Existing Mac lead · State: **blocked** · Last observed: 2026-10-30T22:09:34.860015+00:00
Activity: Headless document workflow and bounded live search passed; newer reviewed code/evidence remains local/unpushed. Prior remote CI is not new-source coverage.
Next: Await owner publication/hosted-CI authorization and native acceptance; preserve draft, no push/ready/merge.
Resource waits: native Mac access; separate owner publication/CI/integration decision

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

- resource: native Mac access
- resource: separate owner publication/CI/integration decision

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

2 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `subscription-search` — Qualify subscription-only search
- `document-workflow` — Import knowledge and produce a referenced draft

Local page reflects the last saved observations in this record (2026-10-30T22:09:34.860015+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
