# Development overview

**Goal:** Complete issue256 evidence-fix salvage and bounded product token hill-climb.
**Owner attention:** Measured candidate confirms 14.03% lower tokens per correct supported answer. Final Council/code/security/CI/product integration remain. 254/600 live HTTP attempts used.

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

### Consolidate shared provider policy and qualify normal integration

Task ID: `shared-provider-263`
Owner: Agent C / Codex (cloud) · State: **active** · Last observed: 2026-11-07T05:13:29.268453+00:00
Activity: Shared extraction and all setting/fence consumers implemented; final72bb6241 also forwards CLI/desktop canonical keys. Full cloud storage/end-to-end gate passes,771library tests/2parent-invoked fixture ignores,108steps1266checks. Three temporary tasks and fresh evaluator challenge pass both repaired baseline/candidate; failed/limited fixture attempts retained. No experimental policy ships. Final evidence batch now needs current-head hosted CI/code/security and normal merge.
Next: Finish current-head PR267 hosted CI including macOS, Codex code/security and thread disposition; reconcile live main/B publication window, normal merge and inspect resulting-main required checks. PR267 and #263/#253 own final completion receipts; this is the pre-merge qualification snapshot.
Depends on: development-speed-255 when Cycle 2 closeout as its own PR with applicable reviews/checks and normal merge (reported satisfied: true)

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Owner current A-is-done instruction reconciled at selecteda921e325. Existing setting/account/client/route/schema/retry policy now has shared owners; duplicate branches removed, distinct caller contracts retained. Auth behavior repair272 merged separately75f37221; final refactor branch merges that main normally.
- **checks:** Final72bb6241 full storage/end-to-end gate status0; libraries765→771 with2parent-invoked fixture ignores, prior passing case names retained; strict Clippy and108step/1266check CLI journey.3temporary tasks pass before/after, corrected alias real configuration/run-fence acceptance and one fresh read-only evaluator challenge (same5new+2positive+37auth cases/arm) pass; sources restored. Original attempt overruns/fixture failures and model-identity/elapsed limits explicit in qualification.json. Current final-head hosted CI/reviews still required; earlier4ef reviews are historical. Council skipped per owner,0sessions/no score, not passed.
- **integration:** Repair272 normalmerge75f37221; its required-main contexts pass, optional3Windows failures outside product diff retained. Refactor267 now based on main, remains separate from B first-alpha a921e325. Inspect exact current-head/platform/review gates and B publication window before normal merge; no bypass/forcepush.
- **native:** Cloud implementation/offline transports only. Hosted macOS checks required; no owner Mac/credentials/live call. Earlier A/B/D/E receipts retain original sources. Later A/B selected-package combined account/chat/helper/summary/web/refinement journeys remain separate from cloud refactor qualification.
- **owner acceptance:** Assigned cloud implementation and normal merge authorized. Owner explicitly selected one fresh read-only evaluator and skipped Council. No release/alpha publication authority or fresh live/native acceptance inferred.

</details>

### Extract useful evidence fixes and run product token hill-climb

Task ID: `search-efficiency-256-restart`
Owner: Issue 256 restart lead on MacBook · State: **active** · Last observed: 2026-11-07T06:59:47.184933+00:00
Activity: Three product hypotheses measured. First two rejected for higher tokens. Corrected inline first-hit passage runtime confirms 14.03% lower tokens/correct answer on six fixed cases,12/12 fully correct per arm; 56 product and 56 grader attempts retained, 254 HTTP total, 504 reserved slots. Preparing final Council/code/security/CI gates.
Next: Council checkpoint two reviews actual product diff/evidence, then publish focused product PR and merge after current-head code/security and CI.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Bounded exact first-hit passage through checked read path; inline-output coverage guard and host-read tracing implemented.
- **checks:** 51 search/research, 9 read-ledger, 28 summary, 3 guide and 13 writing-evidence tests pass, plus strict Clippy and 94 Python tests (11 skips). All 56 live answers graded fully correct. Current-head hosted gates pending.
- **integration:** Salvage 273 merged bab8c876 and superseded PRs closed. Required salvage-main contexts passed; optional Windows main failures retained. Product merge pending.
- **native:** Headless synthetic qualification with real selected subscription calls. B retains separate native/release qualification.
- **owner acceptance:** Owner authorized normal merge after required reviews/CI. Hands-on owner acceptance remains separate.

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

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

2 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `development-speed-255` — Measure and improve development feedback (#255)
- `rig-046-253` — Upgrade the runtime to Rig 0.46 and qualify bounded reuse

## Dependencies

```mermaid
graph TD
  t3["Measure and improve development feedback (#255)"]
  t5["Consolidate shared provider policy and qualify normal integration"]
  t3 -->|Cycle 2 closeout as its own PR with applicable reviews/checks and normal merge (satisfied)| t5
```

Resource waits are listed on tasks and are not dependency edges.

Local page reflects the last saved observations in this record (2026-11-07T06:59:47.184933+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
