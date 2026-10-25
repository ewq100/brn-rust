# Development overview

**Goal:** Turn meeting notes into dependable follow-ups, add a manually scanned document inbox, and expose safe desktop backup restoration.
**Owner attention:** All three narrow slices approved. Calculator remains deferred. Draft PRs only; no new merge/release authority or Mac access. Prior resulting-main required gates passed; optional Windows failures remain separate.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/daily-followups-inbox-restore/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

None recorded.

## Next

### Make meeting-note follow-ups dependable

Task ID: `actions-followups`
Owner: Actions implementation lane · State: **ready** · Last observed: 2026-10-25T08:41:43.776833+00:00
Activity: All three slices and native corrections source-reviewed. Full native419+1ignored, strengthened menu-focus test, core228/app35/Python204+1skip, strictlint, CLI108steps1266checks and offline runtime fixtures passed. Actual Linux saved-owner/readback and Data controls observed; editing/AT/picker limits remain explicit.
Next: Follow final exact-head CI on draftPR227; present reviewed scope and remaining native/live gates for separate merge decision.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** complete: approved scope implemented and source-reviewed; no new merge authority
- **checks:** passed: applicable local deterministic checks; final publication CI receipt is maintained on PR227, not inferred from earlier-head checks
- **integration:** pending: reviewed draftPR227; final published head/terminal CI on PR; merge separately authorized
- **native:** partial: Linux Data controls and saved Action observed; editing text echo/ATfocus/Screen coordinates and pickers unqualified; actualMac pending
- **owner acceptance:** pending: implementation scope approved, delivered behavior not accepted

</details>

### Scan a document inbox with visible status and safe retries

Task ID: `manual-inbox`
Owner: Inbox implementation lane · State: **ready** · Last observed: 2026-10-25T08:41:43.776833+00:00
Activity: All three slices and native corrections source-reviewed. Full native419+1ignored, strengthened menu-focus test, core228/app35/Python204+1skip, strictlint, CLI108steps1266checks and offline runtime fixtures passed. Actual Linux saved-owner/readback and Data controls observed; editing/AT/picker limits remain explicit.
Next: Follow final exact-head CI on draftPR227; present reviewed scope and remaining native/live gates for separate merge decision.
Depends on: actions-followups when Reviewed schema11 migration contract is available before stacking schema12 and integrating Inbox storage. (reported satisfied: true)

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** complete: approved scope implemented and source-reviewed; no new merge authority
- **checks:** passed: applicable local deterministic checks; final publication CI receipt is maintained on PR227, not inferred from earlier-head checks
- **integration:** pending: reviewed draftPR227; final published head/terminal CI on PR; merge separately authorized
- **native:** partial: Linux Data controls and saved Action observed; editing text echo/ATfocus/Screen coordinates and pickers unqualified; actualMac pending
- **owner acceptance:** pending: implementation scope approved, delivered behavior not accepted

</details>

### Restore a backup through the existing checked engine

Task ID: `desktop-restore`
Owner: Restore implementation lane · State: **ready** · Last observed: 2026-10-25T08:41:43.776833+00:00
Activity: All three slices and native corrections source-reviewed. Full native419+1ignored, strengthened menu-focus test, core228/app35/Python204+1skip, strictlint, CLI108steps1266checks and offline runtime fixtures passed. Actual Linux saved-owner/readback and Data controls observed; editing/AT/picker limits remain explicit.
Next: Follow final exact-head CI on draftPR227; present reviewed scope and remaining native/live gates for separate merge decision.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** complete: approved scope implemented and source-reviewed; no new merge authority
- **checks:** passed: applicable local deterministic checks; final publication CI receipt is maintained on PR227, not inferred from earlier-head checks
- **integration:** pending: reviewed draftPR227; final published head/terminal CI on PR; merge separately authorized
- **native:** partial: Linux Data controls and saved Action observed; editing text echo/ATfocus/Screen coordinates and pickers unqualified; actualMac pending
- **owner acceptance:** pending: implementation scope approved, delivered behavior not accepted

</details>

### Independently review and qualify coherent draft slices

Task ID: `integration`
Owner: lead integration and independent reviewer · State: **ready** · Last observed: 2026-10-25T08:41:43.776833+00:00
Activity: All three slices and native corrections source-reviewed. Full native419+1ignored, strengthened menu-focus test, core228/app35/Python204+1skip, strictlint, CLI108steps1266checks and offline runtime fixtures passed. Actual Linux saved-owner/readback and Data controls observed; editing/AT/picker limits remain explicit.
Next: Follow final exact-head CI on draftPR227; present reviewed scope and remaining native/live gates for separate merge decision.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** complete: approved scope implemented and source-reviewed; no new merge authority
- **checks:** passed: applicable local deterministic checks; final publication CI receipt is maintained on PR227, not inferred from earlier-head checks
- **integration:** pending: reviewed draftPR227; final published head/terminal CI on PR; merge separately authorized
- **native:** partial: Linux Data controls and saved Action observed; editing text echo/ATfocus/Screen coordinates and pickers unqualified; actualMac pending
- **owner acceptance:** pending: no delivered behavior acceptance

</details>

## Needs attention

### Complete actual Mac follow-up, inbox and restore acceptance

Task ID: `mac-acceptance`
Owner: future separately authorized native actor · State: **blocked** · Last reported: 2026-10-24T03:52:34.194266+00:00
Activity: Mac is not authorized for dot tasks; headless checks do not establish picker, focus, rendering or accessibility.
Next: After separate authorization, exercise Actions owner/waiting/follow-up controls, manual inbox selection/status/retry and safe Restore confirmation/reopen.
Resource waits: Separate Mac access/owner session authorization

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** inapplicable: acceptance-only task
- **checks:** pending: source-bound candidate and fixture preparation
- **integration:** deferred: use reviewed candidate when native session authorized
- **native:** pending: not observed in this batch
- **owner acceptance:** pending

</details>

Waiting:

- resource: Separate Mac access/owner session authorization

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

2 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `prior-main-ci` — Close the prior batch resulting-main CI gate
- `documentation` — Record selected scope and narrow backlog decisions

## Dependencies

```mermaid
graph TD
  t1["Make meeting-note follow-ups dependable"]
  t2["Scan a document inbox with visible status and safe retries"]
  t1 -->|Reviewed schema11 migration contract is available before stacking schema12 and integrating Inbox storage. (satisfied)| t2
```

Resource waits are listed on tasks and are not dependency edges.

Local page reflects the last saved observations in this record (2026-10-25T08:41:43.776833+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
