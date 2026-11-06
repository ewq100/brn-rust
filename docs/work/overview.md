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
Owner: Agent B · State: **active** · Last observed: 2026-11-06T14:10:50.977367+02:00
Activity: Updater and packaging implemented; independent review clear after fixing CLT dependency and interruption/concurrency findings. 353 tooling tests (11 skips), 41 focused checks and real local BRN A→B/synthetic SQLite preservation/backup/restore passed on Mac mini. Packaged startup and both helpers/all eight typed formats also passed. Focused PR258 is open and attached; exact-head hosted CI is in progress.
Next: Inspect PR258 exact-head hosted gates and hand off final integrated acceptance. Agent A must supply the selected runtime/Copilot SHA and package dependencies; then use an unlocked Mac/clean disposable profile for Gatekeeper, install, real released A→B and native Copilot journeys. No merge or publication authority inferred.
Resource waits: Mac mini CGSSessionScreenIsLocked=true; native interaction/login/Gatekeeper pending.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Implemented bundled command, strict fixed-repository package verification, normal-closure/concurrency exclusion, staged recoverable replacement, shell-owned descriptor guard, repeatable recovery, alpha-compatible packaging and instructions. Independent reviewer has no remaining findings; production updater SHA256 7f5ea95caa55c0f552d54fd215a28c18d3a55e16b1011db7bfe6889d565157b5.
- **checks:** Passed on Mac mini arm64/macOS27.2: pinned1.98.1 native-ui release desktop, CLI and both document helpers; 353 tooling tests (11 skips), 41 focused tests including19 updater cases; real local package A alpha.1/build101→B alpha.2/build102 with byte-identical external files and logical state after B headless reopen, figure asset, backup/fresh restore; both relocated helpers/all8formats and CLI16imports. Test transport substituted; clean-Mac/released HTTPS update not claimed.
- **integration:** PR258 open, mergeable; not merged or published. Production updater832e8888 unchanged by qualification/evidence-only follow-ups. Exact-head hosted CI in progress. release/preview.json unchanged; no tags, release or announcement.
- **native:** Pending: screen remains locked. Local real-package/headless evidence is not native acceptance, clean-Mac Gatekeeper acceptance or a published-release transport witness.
- **owner acceptance:** Pending Agent A selected runtime/Copilot candidate, native install/update/reopen and required Copilot journeys, then owner use before invited distribution.

</details>

## Next

None recorded.

## Needs attention

### Implement and qualify the versioned Mac updater

Task ID: `mac-updater-253`
Owner: Agent B

Waiting:

- resource: Mac mini CGSSessionScreenIsLocked=true; native interaction/login/Gatekeeper pending.

## Completed

None recorded.

Local page reflects the last saved observations in this record (2026-11-06T14:10:50.977367+02:00). A remote page reflects the last published commit. Neither indicates that work is live now.
