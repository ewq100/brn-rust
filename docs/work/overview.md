# Development overview

**Goal:** Implement #253’s independent Mac updater and qualify disposable package replacement; retain earlier task history.
**Owner attention:** Agent B owns updater/packaging only. Final integrated/native acceptance waits for Agent A’s selected runtime/Copilot candidate and an unlocked Mac. No release publication or new merge authority inferred.

Activity below is the last reported or observed update. It does not establish that work is running now.

Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.

Task details and full history: [selected work record](active/windows-ci-portability/record.json). Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked selected record path and TASK_ID with an ID shown below. For older IDs, search the linked record by title or ID.

## Working on

### Qualify and integrate Windows compatibility

Task ID: `windows-ci`
Owner: Windows compatibility lead · State: **active** · Last observed: 2026-11-05T11:54:06.903957+02:00
Activity: Reviewed currentmain e749a83e integration is ready:3documentation routing conflicts resolved; Rust/native/workflow inputs exactly match Windows-qualified6c6f9d40 and PR246 benchmark/evidence exactly match currentmain. Local334tooling tests(12skips),88benchmark protocol tests(11native/configuration skips),overview and links passed.
Next: Publish the reviewed normalmerge of currentmain to PR245, require newhead normalPR checks, retain prior3Windows results for byte-identical Rust/native/workflow inputs, then normalmerge and resultingmain verification. Existingfollow-up stays paused while integrationlead owns active work; update that same task if infrastructure waiting requires handoff.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Independent stagedmerge review found no actionable issue. PR246 benchmark/source/evidence exactly preserved; Rust crates/manifests/lockfile/agent/workflow/attributes unchanged from reviewedWindows-qualified6c6f9d40.
- **checks:** Local334tooling(12skips) and88benchmark protocol(11skips) tests passed; overview/links passed. Newhead normalPR checks required. PriorWindows3 success on6c6f9d40 retained only for verified identical Rust/native/workflow inputs; no newhead native UI or parity claim.
- **integration:** Not merged. Existing hourlyfollow-up is paused and pins oldhead/base; reconcile and review currentmain locally before updating that same follow-up. No duplicate watcher.
- **native:** Windows native UI job passed on7c01e10d and Linux/Mac supported jobs passed onf3fd36f8; no new real UI interaction or feature parity claim.
- **owner acceptance:** No native interaction or feature parity claim; secure credentials, exclusive model installation and owner-only token-output files remain explicitly unavailable on Windows.

</details>

### Implement and qualify the versioned Mac updater

Task ID: `mac-updater-253`
Owner: Agent B · State: **active** · Last observed: 2026-11-06T13:55:41.368636+02:00
Activity: Implemented bundled built-in-tools updater, alpha packaging and setup instructions. 13 real macOS updater cases passed, including SIGKILL recovery and package tampering. Pinned desktop release build passed; CLI/helpers and final evidence in progress.
Next: Finish real BRN package A→B/synthetic-store witness, independent review and focused updater PR; reconcile final candidate with Agent A.
Resource waits: Final native installation/reopen/Gatekeeper and Copilot journeys: screen locked and selected Agent A runtime/Copilot candidate unavailable.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Implemented candidate; independent review pending.
- **checks:** 13 updater tests passed on macOS arm64/27.2; signed synthetic bundles. Pinned 1.98.1 native-ui release desktop build passed. Real BRN A→B/store and broader checks pending.
- **integration:** Dedicated work/mac-updater-253 from main 3b72813d. PR not yet open; not merged; release/preview.json unchanged; no publication.
- **native:** Pending: actual Mac reports CGSSessionScreenIsLocked=true. No headless check counted as native acceptance.
- **owner acceptance:** Pending combined selected runtime/Copilot candidate, clean installation, Gatekeeper, final native journeys and owner use.

</details>

## Next

None recorded.

## Needs attention

### Implement and qualify the versioned Mac updater

Task ID: `mac-updater-253`
Owner: Agent B

Waiting:

- resource: Final native installation/reopen/Gatekeeper and Copilot journeys: screen locked and selected Agent A runtime/Copilot candidate unavailable.

## Completed

None recorded.

Local page reflects the last saved observations in this record (2026-11-06T13:55:41.368636+02:00). A remote page reflects the last published commit. Neither indicates that work is live now.
