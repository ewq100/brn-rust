# Development overview

**Goal:** Turn meeting notes into dependable follow-ups, add a manually scanned document inbox, and expose safe desktop backup restoration.
**Owner attention:** All three narrow slices approved. Calculator remains deferred. Draft PRs only; no new merge/release authority or Mac access. Prior resulting-main required gates passed; optional Windows failures remain separate.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/daily-followups-inbox-restore/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

### Scan a document inbox with visible status and safe retries

Task ID: `manual-inbox`
Owner: Inbox implementation lane · State: **active** · Last reported: 2026-10-24T04:46:55.734511+00:00
Activity: Final app source covers stable receipt identity across settings/folder reselection and Undo, atomic settings/generation fences, descriptor reads and aggregate converted output limits. App suite compiling; native checks deferred to combined tree.
Next: Finish app/core checks, commit source, then integrate and run the combined native suite.
Depends on: actions-followups when Reviewed schema11 migration contract is available before stacking schema12 and integrating Inbox storage. (reported satisfied: true)
Resource waits: Lead-coordinated heavyweight build slot; no concurrent shared-target writers

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** pending
- **checks:** pending: source-bound focused checks and independent review
- **integration:** pending: draft PR only; no new merge authority
- **native:** pending: actual Mac interaction not authorized in this cloud batch
- **owner acceptance:** pending: implementation scope approved, delivered behavior not accepted

</details>

### Independently review and qualify coherent draft slices

Task ID: `integration`
Owner: lead integration and independent reviewer · State: **active** · Last observed: 2026-10-24T04:46:55.734511+00:00
Activity: Actions and Restore integrated through512e9eac. Final Python suite205 run:204 passed,1platform skip. Inbox holds Cargo slot for app/core qualification; lead will then run combined Rust/native gates and draft PR CI.
Next: Integrate reviewed source-bound slices, run applicable fail-fast local checks and normal CI; open draft PRs only.
Resource waits: Lead-coordinated build slot; sandbox credential fixtures require normal CI

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** pending
- **checks:** passed: Python205 run/204pass/1platform skip; pending combined Rust/native and exact-head CI
- **integration:** pending: draft PR only; no new merge authority
- **native:** pending: one consolidated later Mac checklist
- **owner acceptance:** pending: no delivered behavior acceptance

</details>

## Next

None recorded.

## Needs attention

### Make meeting-note follow-ups dependable

Task ID: `actions-followups`
Owner: Actions implementation lane · State: **ready** · Last reported: 2026-10-24T04:46:55.734511+00:00
Activity: UI9c40c9b3 integrated as512e9eac; both focused native-feature tests passed after correcting a platform shortcut in the fixture. Service/core and semantic oracle source reviewed; four authored live cases remain unrun.
Next: Participate in combined source review and repair any integration failures.
Resource waits: Lead-coordinated heavyweight build slot; no concurrent shared-target writers

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** complete: reviewed local core/service/UI slices; not merged
- **checks:** passed: focused core76/app13/Python23/native2; pending combined source and CI
- **integration:** pending: draft PR only; no new merge authority
- **native:** pending: actual Mac interaction not authorized in this cloud batch
- **owner acceptance:** pending: implementation scope approved, delivered behavior not accepted

</details>

Waiting:

- resource: Lead-coordinated heavyweight build slot; no concurrent shared-target writers

### Scan a document inbox with visible status and safe retries

Task ID: `manual-inbox`
Owner: Inbox implementation lane

Waiting:

- resource: Lead-coordinated heavyweight build slot; no concurrent shared-target writers

### Restore a backup through the existing checked engine

Task ID: `desktop-restore`
Owner: Restore implementation lane · State: **ready** · Last reported: 2026-10-24T04:46:55.734511+00:00
Activity: UI569c708e integrated as11386576; seven native-feature desktop tests passed. Core16 and exact prior-main backup witness passed; independent source review clear.
Next: Support combined qualification and bounded actual Linux observation; Mac gate remains separate.
Resource waits: Lead-coordinated heavyweight build slot; no concurrent shared-target writers

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** complete: reviewed local staged-restore/UI slices; not merged
- **checks:** passed: focused core16/native7 and exact schema10 backup witness; pending combined source and CI
- **integration:** pending: draft PR only; no new merge authority
- **native:** pending: actual Mac interaction not authorized in this cloud batch
- **owner acceptance:** pending: implementation scope approved, delivered behavior not accepted

</details>

Waiting:

- resource: Lead-coordinated heavyweight build slot; no concurrent shared-target writers

### Independently review and qualify coherent draft slices

Task ID: `integration`
Owner: lead integration and independent reviewer

Waiting:

- resource: Lead-coordinated build slot; sandbox credential fixtures require normal CI

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

Local page reflects the last saved observations in this record (2026-10-24T04:46:55.734511+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
