# Development overview

**Goal:** Publish and integrate the bounded #255 development-feedback findings and reliable measurement guidance.
**Owner attention:** No performance configuration retained: opt0 regressed full-suite feedback, codegen512 regressed compilation. Final independent review and normal PR/CI follow-through remain.

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

### Measure and improve development feedback (#255)

Task ID: `development-speed-255`
Owner: Agent C / Codex GPT-6.1 Sol · State: **active** · Last observed: 2026-11-06T16:26:54.079230+00:00
Activity: Both bounded candidates rejected; original Cargo/Rust/CI bytes restored and trial processes reaped. Final uncongested opt0 pair preserves all tests but worsens full-suite feedback;512CGU cold/core/auth are slower. Measurement ended within original budget; slot released.
Next: Reconcile current main, final read-only Council review of no-change/evidence/guidance, focused PR, Codex/security/current-head CI and normal integration.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** No performance configuration retained. Measurement/helper/freshness guidance corrected; frozen before/after and negative evidence recorded. No product/test/gate changes.
- **checks:** Original and opt0 final arms each3 workspace1491/0/9 and AI598/0/2; exact nonzero probes/features/checks verified. Candidate2 early rejected on repeated core regression plus cold/auth evidence; later partial samples excluded. Docs/link/overview checks pass; final review/hosted CI pending.
- **integration:** Not published or merged. Preserve Agent B updater and Agent A runtime ownership.
- **native:** Inapplicable: headless build/test tooling scope; existing hosted compatibility checks retained.
- **owner acceptance:** Current owner assignment authorizes implementation, Council and focused PR; no manual UI acceptance requirement.

</details>

## Next

None recorded.

## Needs attention

None recorded.

## Completed

None recorded.

Local page reflects the last saved observations in this record (2026-11-06T16:26:54.079230+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
