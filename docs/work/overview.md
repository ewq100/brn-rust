# Development overview

**Goal:** Close useful remaining cloud verification for merged follow-ups, manual inbox and Restore; provide a bounded headless Mac live-model prompt.
**Owner attention:** PR227 merged90e5bceb with seven Mac/Linux/docs main jobs passed; three optional Windows failures. Follow-up tests/evidence may be drafted, not merged. Mac execution/UI acceptance, calculator and release remain outside this cloud task.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/daily-followups-inbox-restore/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

### Independently review and qualify coherent draft slices

Task ID: `integration`
Owner: lead integration and independent reviewer · State: **active** · Last observed: 2026-10-25T10:58:26.882361+00:00
Activity: Initial PR228 Linux CI exposed retained-descriptor inbox lock. Regression failed before fix; explicit-unlock guard reviewed, green regression and all 23 inbox tests plus strict lint passed.
Next: Publish corrected draft and follow exact-head CI; no merge authority.
Resource waits: One shared Cargo writer; no simultaneous heavy build and actual Linux UI observation

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** scoped correction: explicit inbox lease release; independently reviewed
- **checks:** passed locally: red/green regression, 23 inbox tests, strict app lint; corrected-head CI pending at publication
- **integration:** draft PR228; separate merge decision required
- **native:** partial: Linux Data controls, initial Owner keyboard input/visible text and saved Action observed; ATfocus/Text/Screen coordinates and pickers unqualified; actualMac pending
- **owner acceptance:** pending: no delivered behavior acceptance

</details>

## Next

### Qualify real Scan button and Stock worker

Task ID: `inbox-real-worker`
Owner: lead integration with bounded source/evidence reviewer · State: **ready** · Last observed: 2026-10-25T10:31:29.748698+00:00
Activity: Targeted witness passed on ece02f7f; independent source review found no blockers.
Next: Publish reviewed draft PR and follow exact-head CI; no merge authority.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** test/evidence only; no new product capability
- **checks:** passed: two targeted inbox witnesses, strict app/native test Clippy, formatting and documentation checks; remote follow-up CI pending at publication
- **integration:** pending: follow-up draft PR only; no merge authority
- **native:** inapplicable: deterministic cloud or documentation task
- **owner acceptance:** not claimed

</details>

### Qualify killed scan process and durable recovery

Task ID: `inbox-killed-process`
Owner: lead integration with bounded source/evidence reviewer · State: **ready** · Last observed: 2026-10-25T10:31:29.748698+00:00
Activity: Targeted witness passed on ece02f7f; independent source review found no blockers.
Next: Publish reviewed draft PR and follow exact-head CI; no merge authority.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** test/evidence only; no new product capability
- **checks:** passed: two targeted inbox witnesses, strict app/native test Clippy, formatting and documentation checks; remote follow-up CI pending at publication
- **integration:** pending: follow-up draft PR only; no merge authority
- **native:** inapplicable: deterministic cloud or documentation task
- **owner acceptance:** not claimed

</details>

## Needs attention

### Independently review and qualify coherent draft slices

Task ID: `integration`
Owner: lead integration and independent reviewer

Waiting:

- resource: One shared Cargo writer; no simultaneous heavy build and actual Linux UI observation

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

### Observe remaining Action fields in Linux

Task ID: `linux-action-fields`
Owner: lead integration with bounded source/evidence reviewer · State: **blocked** · Last observed: 2026-10-25T10:28:57.427416+00:00
Activity: Final keyboard-only attempt run.lbu5SR showed blank fields; stopped before Save. No validation or persistence defect established.
Next: Retain failed evidence and qualify actual input/focus in a separately authorized native session; no blind retries.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** test/evidence only; no new product capability
- **checks:** unqualified: input delivery not established; no invalid-date validation pass
- **integration:** pending: follow-up draft PR only; no merge authority
- **native:** partial: previous Owner input/save observed; extended flow unqualified
- **owner acceptance:** not claimed

</details>

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

6 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `actions-followups` — Make meeting-note follow-ups dependable
- `desktop-restore` — Restore a backup through the existing checked engine
- `mac-live-prompt` — Deliver bounded headless Mac live-model prompt
- `manual-inbox` — Scan a document inbox with visible status and safe retries
- `prior-main-ci` — Close the prior batch resulting-main CI gate
- `documentation` — Record selected scope and narrow backlog decisions

Local page reflects the last saved observations in this record (2026-10-25T10:58:26.882361+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
