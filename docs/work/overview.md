# Development overview

**Goal:** Integrate updater PR258 normally, then prepare final Mac package/update acceptance for253 after A release-critical hardening.
**Owner attention:** B owns updater/package integration. A owns response-limit hardening and Copilot acceptance; coordinate heavy builds. Native and publication remain separate gates.

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

### Support Copilot chat, native web search and Luna refinement

Task ID: `copilot-252`
Owner: Agent A (runtime) · State: **active** · Last observed: 2026-11-06T18:05:00+02:00
Activity: Implemented on work/copilot-252, now on main after #259: account pin captured at operation start for chat, helpers, web search and refinement; native Copilot hosted web_search adapter on the Rig Responses route with an 8 MiB body cap and honest denial messages; provider-aware Copilot Luna refinement with no Codex fallback; Copilot tools sent non-strict so write proposals work. Live qualification on a Copilot business account passes.
Next: Request Codex review of the final head, merge PR #261 after hosted CI and review pass, then notify B, D (#254) and E (#256) on #253. Native sign-in, account switching and the remaining acceptance items in the record stay open on #252.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Implemented and qualified live (chat, web search with sources, BRN tools, writes, refinement On/Off, failure messages); Council round 1 complete (Codex 7/10, Kimi K3 8.5/10, Claude 8/10; required fixes made, nits fixed or routed to #255).
- **checks:** Local tests, Clippy and fmt pass (brn-ai 597 passed, 1 ignored); hosted CI on PR #261.
- **integration:** Published as PR #261 (part of #252); not merged.
- **native:** Pending; device sign-in and native account switching need native observation.
- **owner acceptance:** Not requested.

</details>

### Measure and improve development feedback (#255)

Task ID: `development-speed-255`
Owner: Agent C / Codex GPT-6.1 Sol · State: **active** · Last observed: 2026-11-06T17:01:08.332168+00:00
Activity: PR #262 is open. All final corrected-evidence reviewers support no-change/incomplete qualification (Sol9/Opus8/Grok9); low documentation fixes applied. Main #261 reconciled; product/config/runtime/CI match main. Final-head CI/Codex/security review next.
Next: Push reconciled final head, request current-head Codex review, inspect current CI/code/security results and unresolved findings, then normal merge and main verification.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Documentation/evidence only; no configuration retained because net performance is unqualified within the budget, not proven harmful. Guidance adds ABBA/same-binary checks and limits Cargo-only guards.
- **checks:** Frozen-source suites/probes and all measurements preserved with causal limits. Final Council Sol9/Opus8/Grok9, noHigh/Medium findings; low fixes applied. Links/overview/16tests/diff pass. Current-head hosted CI/Codex/security pending publication.
- **integration:** Published as PR #262; not merged. Normal merge after current-head CI, Codex and security review. Source/config byte-identical to main; frozen measurements remain separate.
- **native:** Inapplicable: headless build/test tooling scope; existing hosted compatibility checks retained.
- **owner acceptance:** Current owner assignment authorizes implementation, Council and focused PR; no manual UI acceptance requirement.

</details>

### Implement and qualify the versioned Mac updater

Task ID: `mac-updater-253`
Owner: Agent B · State: **active** · Last observed: 2026-11-06T21:02:54.795698+02:00
Activity: Resumed normal integration of PR258 with main6897fe56 after Rig259, Copilot261 and development-speed262 merged. Resolving only task/status routing conflicts while preserving all updater source, static Linux fixture repair, raw evidence and other tasks. Prior seven green checks cover an older integration base.
Next: Review refreshed combination, run applicable checks, request final-head Codex code/security review and normal merge under standing owner instruction; verify resulting main. Coordinate with A before heavy builds and select final package only after response-limit fixes land. Native clean-profile session and approved published stock HTTPS pair remain separate gates.
Resource waits: Approved published command-enabled A/B pair unavailable; preview.1/.2 receipts/source approval verified but neither includes updater.; Mac mini last observed CGSSessionScreenIsLocked=true; native interaction/login/Gatekeeper pending.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Implemented bundled command, strict fixed-repository package verification, normal-closure/concurrency exclusion, staged recoverable replacement, shell-owned descriptor guard, repeatable recovery, alpha-compatible packaging and instructions. Independent reviewer has no remaining findings; production updater SHA256 7f5ea95caa55c0f552d54fd215a28c18d3a55e16b1011db7bfe6889d565157b5.
- **checks:** Original tooling corrected:353 total=342 passed+11 skipped. Resumed:360 total=349 passed+11 skipped; intake crate11 passed/1 existing ignored and all-target Clippy passed. Linux baseline/submitted identical code both reproduced ETXTBSY with concurrent forks; static fixture40 plain+400 stressed complete loops passed. Real local signed A→B rerun preserved synthetic files/state/backup. Production build/helper evidence retained unchanged; headless/local transport is not stock published HTTPS or native acceptance. OUTSTANDING: actual approved preview.1/.2 metadata/digests/source-approved receipts verified; preparation blocked because neither includes updater. New preparation plus guarded --stock-transport witness ready for an authority-approved published command-enabled A/B pair. No ZIP/app/updater executed from those legacy releases.
- **integration:** Pending refreshed combination: normal merge main6897fe56 into PR258; prior7d2436ee checks tested older base2ebcf4c7. Current standing owner instruction authorizes normal merge after applicable checks/review; native/release acceptance separate.
- **native:** Pending: screen remains locked. Local real-package/headless evidence is not native acceptance, clean-Mac Gatekeeper acceptance or a published-release transport witness.
- **owner acceptance:** Pending Agent A selected runtime/Copilot candidate, native install/update/reopen and required Copilot journeys, then owner use before invited distribution.

</details>

### Improve grounded search efficiency (#256)

Task ID: `search-256`
Owner: Agent E · State: **active** · Last observed: 2026-11-06T22:02:45.465227+00:00
Activity: Source-bound baseline CLI/seeder built defaultfeatures;5corpus checks and97AgentBench tests(nativeCLI,6skips) pass. FullAI608pass/4missing-helperfail/1ignored;9helper rechecks pass;23Rigshadow/wire checks pass. E released MacBook slot to D, no live yet.
Next: Finish source-bound baseline and grader qualification,freeze inputs;coordinate D/E timed slots and run bounded screening.
Resource waits: D MacBook grader/CLI build qualification and explicit timing-slot release

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Pending measured baseline and focused candidate selection.
- **checks:** Baseline build/source/hash and stable corpus/index hashes recorded. Visibilitytrap0general/1private,cloned fixed-querypagesidentical. Rust initial test compile inference fixed and failure preserved;fullAI helper-envfailures corrected by9targeted passes. Required graderhash/livefreeze pending D.
- **integration:** Separate PR authorized. No merge/release/default-model change.
- **native:** Headless assignment; no native acceptance claimed.
- **owner acceptance:** Owner clarified same-machine fresh evaluation-only role with questions withheld from tuning supports a limited engineering conclusion, not technical isolation or broad superiority. Measured quality/confirmation gates remain pending.

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

### Improve grounded search efficiency (#256)

Task ID: `search-256`
Owner: Agent E

Waiting:

- resource: D MacBook grader/CLI build qualification and explicit timing-slot release

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

1 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `rig-046-253` — Upgrade the runtime to Rig 0.46 and qualify bounded reuse

Local page reflects the last saved observations in this record (2026-11-06T21:02:54.795698+02:00). A remote page reflects the last published commit. Neither indicates that work is live now.
