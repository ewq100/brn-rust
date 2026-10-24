# Development overview

**Goal:** Turn meeting notes into dependable follow-ups, add a manually scanned document inbox, and expose safe desktop backup restoration.
**Owner attention:** All three narrow slices approved. Calculator remains deferred. Draft PRs only; no new merge/release authority or Mac access. Prior resulting-main CI still needs lead reconciliation.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/daily-followups-inbox-restore/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

### Close the prior batch resulting-main CI gate

Task ID: `prior-main-ci`
Owner: lead integration · State: **active** · Last reported: 2026-10-24T03:52:34.194266+00:00
Activity: PR226 merged as f60c5a2f from reviewed 54b0c3d6 with unchanged tree. Last reported run40058075212: documentation and Linux native UI/retrieval green, three optional Windows failures, four core/Mac checks active.
Next: Inspect exact f60c5a2f resulting-main checks; preserve optional failures and diagnose any required failure.
Resource waits: GitHub Actions run40058075212 completion

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** complete: prior batch merged; no new implementation assigned
- **checks:** pending: resulting-main run40058075212; last state reported, not independently refreshed here
- **integration:** passed: PR226 merged f60c5a2f; no authority to merge new PRs
- **native:** separate: prior headless/native observations retain original bounds
- **owner acceptance:** not claimed

</details>

### Make meeting-note follow-ups dependable

Task ID: `actions-followups`
Owner: Actions implementation lane · State: **active** · Last reported: 2026-10-24T03:52:34.194266+00:00
Activity: Assigned narrow owners, waiting-on and follow-up-date extension to existing Actions; schema11 first.
Next: Implement shared checked behavior and CLI/native presentation; test ambiguous commitments, duplicates, stale edits, replay and recovery.
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

### Scan a document inbox with visible status and safe retries

Task ID: `manual-inbox`
Owner: Inbox implementation lane · State: **active** · Last reported: 2026-10-24T03:52:34.194266+00:00
Activity: Assigned manual Scan inbox flow reusing the qualified importer; schema12 stacks on Actions schema11. Original files remain untouched.
Next: Implement bounded manual scan, durable status and safe retries; integrate migration only after schema11 ownership is reconciled.
Depends on: actions-followups when Reviewed schema11 migration contract is available before stacking schema12 and integrating Inbox storage. (reported satisfied: false)
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

### Restore a backup through the existing checked engine

Task ID: `desktop-restore`
Owner: Restore implementation lane · State: **active** · Last reported: 2026-10-24T03:52:34.194266+00:00
Activity: Assigned safe desktop Restore flow; reuse existing staging, validation and collision protections without a parallel restore engine.
Next: Implement explicit owner flow and focused failure/reopen tests; keep native picker/focus acceptance separate.
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
Owner: lead integration and independent reviewer · State: **active** · Last reported: 2026-10-24T03:52:34.194266+00:00
Activity: Three implementation lanes use separate worktrees; lead owns schema sequencing, build slots and final exact-head evidence.
Next: Integrate reviewed source-bound slices, run applicable fail-fast local checks and normal CI; open draft PRs only.
Resource waits: Lead-coordinated build slot; sandbox credential fixtures require normal CI

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** pending
- **checks:** pending: applicable local checks plus exact-head CI; known local Git-ancestor credential fixture refusal must fail fast
- **integration:** pending: draft PR only; no new merge authority
- **native:** pending: one consolidated later Mac checklist
- **owner acceptance:** pending: no delivered behavior acceptance

</details>

## Next

None recorded.

## Needs attention

### Close the prior batch resulting-main CI gate

Task ID: `prior-main-ci`
Owner: lead integration

Waiting:

- resource: GitHub Actions run40058075212 completion

### Make meeting-note follow-ups dependable

Task ID: `actions-followups`
Owner: Actions implementation lane

Waiting:

- resource: Lead-coordinated heavyweight build slot; no concurrent shared-target writers

### Scan a document inbox with visible status and safe retries

Task ID: `manual-inbox`
Owner: Inbox implementation lane

Waiting:

- prerequisite actions-followups: Reviewed schema11 migration contract is available before stacking schema12 and integrating Inbox storage. (reported satisfied: false; task state: active)
- resource: Lead-coordinated heavyweight build slot; no concurrent shared-target writers

### Restore a backup through the existing checked engine

Task ID: `desktop-restore`
Owner: Restore implementation lane

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

1 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `documentation` — Record selected scope and narrow backlog decisions

## Dependencies

```mermaid
graph TD
  t1["Make meeting-note follow-ups dependable"]
  t2["Scan a document inbox with visible status and safe retries"]
  t1 -->|Reviewed schema11 migration contract is available before stacking schema12 and integrating Inbox storage. (not satisfied)| t2
```

Resource waits are listed on tasks and are not dependency edges.

Local page reflects the last saved observations in this record (2026-10-24T03:53:15.923856+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
