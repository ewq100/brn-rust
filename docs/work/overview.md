# Development overview

**Goal:** Make BRN more dependable and easier to improve; make progress and remaining decisions easy to see.
**Owner attention:** No owner decision requested. PR184 draft recovery normally merged; required resulting-main checks running. Separate reviewed inactive-Project skill fix ready for publication. Actual Mac batch remains18Tallinn.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details: [selected work record](active/vacation-campaign/campaign.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown on a task below.

## Working on

### Preserve unsent drafts when a note creates its writing conversation

Task ID: `writing-draft-recovery`
Owner: lead integration; independent Sol review completed; bounded implementation complete · State: **active** · Last observed: 2026-10-20T13:06:27.339045+00:00
Activity: PR184 normally mergedd2e0e298 after fresh strict protection, all4required exact-head19d and preceding-main633 checks PASS. Entire resultingtree equals passinghead19d; newmain39427446132 checks running. Actual Mac/owner acceptance separate.
Next: Inspect all4required resulting-main d2e0e298 run39427446132; preserve optionalfailures and nativeMAC10 queue.
Resource waits: Required resulting-main d2e0e298 checks.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** complete local2b836148 atomic generation-checked transfer and scoped recovery attention; no schema/dependency/provider/prompt changes
- **checks:** PASS independent690/574/2b836 scope reviews, final notice18 desktop340/0/1, strict lint/fmt/native build; unchanged11storage reused
- **integration:** Normalmerged2e0e298 verified;4requiredheadPASS, resulting-main39427446132 pending
- **native:** not observed; synthetic toolkit test is separate from OS interaction
- **owner acceptance:** not claimed

</details>

### Show unavailable skill choices correctly in inactive Projects

Task ID: `inactive-project-skill-controls`
Owner: bounded Luna implementation; independent Sol review/lead integration · State: **active** · Last observed: 2026-10-20T13:06:27.339045+00:00
Activity: Composed7a234ee6 exact qualified28474 source; independent source/receipt/compositionPASS, minor obsolete waits cleared. ParentPR184 now normally merged; separate skill-control PR ready for final source/main reconciliation and publication.
Next: Reconcile integrated d2e0e298 ancestry without product change, publish separate reviewed skill-control candidate; keep its exact-head checks and parent resulting-main gates distinct before integration.
Depends on: writing-draft-recovery when PR184 is normally merged so the separate skill candidate can publish against its integrated parent; resulting-main gates remain separate before final integration. (reported satisfied: true)

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** complete local28474ab7; inactive scoped enable/adoption blocked, exact global Remove preserved; no checkedservice/core/prompt change
- **checks:** PASS independent source+receipt assessment; rawfocused4/0; broader341/0/1/lint/fmt/build transcript-backed. Originalcounterfactual/hash gaps retained; no extra broad rerun needed.
- **integration:** ParentPR184merged; separate publication/headchecks pending; parentmain39427446132 must pass before next normalmerge
- **native:** Not observed; separate byte-identical0555 artifact set executable access verified, no launch or owner acceptance
- **owner acceptance:** not claimed

</details>

## Next

None recorded.

## Needs attention

### Check saving, reopening and backup in the real Mac app

Task ID: `mac-journeys`
Owner: development lead; single desktop actor · State: **blocked** · Last reported: 2026-10-20T08:26:01.079451+00:00
Activity: Matching ordinary native-ui candidate build7225c208 passed and immutable executable65083ccc saved; original artifacts preserved. Live proposal ready for actual Apply/Undo.
Next: Use consolidated18:00Mac session for source-reconciled synthetic Save/reopen/backup/recovery and writing proposal controls. Confirm native access and helpers at entry.
Resource waits: Daily18:00Tallinn consolidated native actor; actual source/main must be reconciled before launch.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** existing application; no new source fix selected
- **checks:** reconcile source-bound build before native session
- **integration:** use reviewed integrated source
- **native:** pending Save/reopen, backup and unsaved-recovery journeys
- **owner acceptance:** pending; native evidence does not imply owner acceptance

</details>

Waiting:

- resource: Daily18:00Tallinn consolidated native actor; actual source/main must be reconciled before launch.

### Check grouped review controls on the Mac

Task ID: `review-group-native`
Owner: single native actor in consolidated batch · State: **blocked** · Last reported: 2026-10-20T07:05:09.863171+00:00
Activity: MAC-08 scenario and immutable source-bound desktop/fixture binaries ready for scheduled actual control observation.
Next: Run MAC-08 during consolidated native session, then retain actual AX states and unchanged canonical data separately from source/build checks.
Depends on: review-group-fixture when Fresh offline two-group fixture passed public reopen/discovery/unchanged-state checks. (reported satisfied: true); review-group-toggle when Corrected UI source reviewed and ordinary production binary qualified. (reported satisfied: true)
Resource waits: Daily18:00Tallinn native batch; confirm actual desktop access and no competing actor.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** scenario prepared; no new product feature
- **checks:** matching source/binary/fixture must be confirmed at session start
- **integration:** prefer then-current reviewed main; candidate observations kept separate
- **native:** pending actual true/false/true pressed-state and unchanged data
- **owner acceptance:** not claimed

</details>

Waiting:

- resource: Daily18:00Tallinn native batch; confirm actual desktop access and no competing actor.

### Preserve unsent drafts when a note creates its writing conversation

Task ID: `writing-draft-recovery`
Owner: lead integration; independent Sol review completed; bounded implementation complete

Waiting:

- resource: Required resulting-main d2e0e298 checks.

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

- `visibility-adoption` — Clearer instructions and one progress view
- `feedback-accessibility` — Mac feedback choices expose their selected state
- `resulting-main-checks` — Confirm checks after the Mac correction merged
- `exact-note-reading` — AI note reads use the exact note identity
- `reliability-foundations` — Earlier reliability and agent-workflow improvements
- `review-group-fixture` — Prepare grouped-review test workspace
- `review-group-toggle` — Expose grouped Include/Omit selection to accessibility tools
- `shared-intake-verification` — Check that command-line and desktop imports reuse the same conversation
- `remote-progress-view` — Read development progress remotely in BRN Space
- `state-gap-triage` — Inspect remaining Save and run-preview coordination risks
- `writing-account-route` — Qualify a bounded real-account writing proposal
- `progress-record-integration` — Integrate remote progress routing and verification records
- `backup-failure-audit` — Inspect backup and restore failure boundaries
- `export-publication-durability` — Flush complete export folders before reporting success
- `queue-recovery-audit` — Inspect saved-message recovery after Stop and restart
- `ci-cost-triage` — Measure current automated-check feedback cost
- `action-history-audit` — Inspect Action and history Undo safeguards
- `action-completion-witness` — Exercise Confirm done and its exact Undo in the desktop journey
- `historical-authority-routing` — Route older agent-first results to current task facts
- `project-trash-audit` — Inspect Project deletion and Trash recovery safety
- `mac-core-ci-time-budget` — Allow the complete Mac Core check to finish within a bounded job
- `mac-ci-cache-investigation` — Find why Mac CI recompiles converters and misses reuse
- `ci-main-cache-save-policy` — Keep shared Rust caches without duplicate PR uploads
- `merged-pr-cache-cleanup` — Retire disposable cache copies for merged PR182
- `general-search-audit` — Inspect stale search results across view and scope changes
- `composer-drafts-audit` — Inspect drafts and attachments when switching conversations
- `skill-catalog-ui-audit` — Inspect stale skill choices and scope changes

## Dependencies

```mermaid
graph TD
  t1["Mac feedback choices expose their selected state"]
  t2["Confirm checks after the Mac correction merged"]
  t6["Prepare grouped-review test workspace"]
  t7["Expose grouped Include/Omit selection to accessibility tools"]
  t8["Check grouped review controls on the Mac"]
  t13["Integrate remote progress routing and verification records"]
  t28["Preserve unsent drafts when a note creates its writing conversation"]
  t30["Show unavailable skill choices correctly in inactive Projects"]
  t1 -->|Correction is normally merged so resulting-main checks exist. (satisfied)| t2
  t6 -->|Fresh offline two-group fixture passed public reopen/discovery/unchanged-state checks. (satisfied)| t8
  t7 -->|Corrected UI source reviewed and ordinary production binary qualified. (satisfied)| t8
  t7 -->|PR179 normally merged so documentation can build on the integrated source. (satisfied)| t13
  t28 -->|PR184 is normally merged so the separate skill candidate can publish against its integrated parent; resulting-main gates remain separate before final integration. (satisfied)| t30
```

Resource waits are listed on tasks and are not dependency edges.

Local page reflects the last saved observations in this record (2026-10-20T13:06:27.339045+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
