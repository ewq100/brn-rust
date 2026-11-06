# Development overview

**Goal:** Implement #253’s independent Mac updater and qualify disposable package replacement; retain earlier task history.
**Owner attention:** Agent B owns updater/packaging only. Published stock HTTPS gate needs an approved command-enabled A/B pair; native session needs unlocked Mac; final integrated acceptance waits for Agent A selected runtime/Copilot. No merge/publication authority inferred.

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
Owner: Agent B · State: **active** · Last observed: 2026-11-06T14:48:02.198901+02:00
Activity: PR258 Ubuntu failure reproduced on baseline and submitted PR under controlled Linux fork pressure: ETXTBSY from mutable executable fixture. Stable checked-in helper preserves all cases/limits; 400 stressed fixed loops passed. Mac intake tests/Clippy, 360 tooling total (349 passed,11 skipped), and real local A→B preservation passed. Existing published previews lack updater: actual HTTPS stock gate remains blocked.
Next: Submit the reviewed correction/evidence and inspect final-head hosted gates plus Codex code/security review. Published stock HTTPS witness separately requires release-authority-approved command-enabled A/B artifacts. Native clean-profile install/Gatekeeper requires unlocked Mac and tester login; final runtime/Copilot journeys require Agent A selected SHA/dependencies. No merge/publication authority inferred.
Resource waits: Approved published command-enabled A/B pair unavailable; preview.1/.2 receipts/source approval verified but neither includes updater.; Mac mini last observed CGSSessionScreenIsLocked=true; native interaction/login/Gatekeeper pending.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Implemented bundled command, strict fixed-repository package verification, normal-closure/concurrency exclusion, staged recoverable replacement, shell-owned descriptor guard, repeatable recovery, alpha-compatible packaging and instructions. Independent reviewer has no remaining findings; production updater SHA256 7f5ea95caa55c0f552d54fd215a28c18d3a55e16b1011db7bfe6889d565157b5.
- **checks:** Original tooling corrected:353 total=342 passed+11 skipped. Resumed:360 total=349 passed+11 skipped; intake crate11 passed/1 existing ignored and all-target Clippy passed. Linux baseline/submitted identical code both reproduced ETXTBSY with concurrent forks; static fixture40 plain+400 stressed complete loops passed. Real local signed A→B rerun preserved synthetic files/state/backup. Production build/helper evidence retained unchanged; headless/local transport is not stock published HTTPS or native acceptance. OUTSTANDING: actual approved preview.1/.2 metadata/digests/source-approved receipts verified; preparation blocked because neither includes updater. New preparation plus guarded --stock-transport witness ready for an authority-approved published command-enabled A/B pair. No ZIP/app/updater executed from those legacy releases.
- **integration:** PR258 open, unmerged/unpublished. Normal merge of main e210c2c0 adds current review guidance; final submitted-head hosted gates and Codex code/security completion required. Production updater832e8888 unchanged; release/preview.json unchanged.
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

- resource: Approved published command-enabled A/B pair unavailable; preview.1/.2 receipts/source approval verified but neither includes updater.
- resource: Mac mini last observed CGSSessionScreenIsLocked=true; native interaction/login/Gatekeeper pending.

## Completed

None recorded.

Local page reflects the last saved observations in this record (2026-11-06T14:48:02.198901+02:00). A remote page reflects the last published commit. Neither indicates that work is live now.
