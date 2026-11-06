# Development overview

**Goal:** Close cloud feedback Cycle 2 with honest no-change evidence; keep the later shared-provider refactor behind its handoff gates.
**Owner attention:** Cycle 2 retained no workflow/configuration: cache-validity failure stopped repeated qualification. Required cloud Council diagnosis/review is unavailable. Cycle 2 PR checks/review and A selected-runtime/shared-file handoff remain before Stage 2.

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

## Next

None recorded.

## Needs attention

### Measure and improve development feedback (#255)

Task ID: `development-speed-255`
Owner: Agent C / Codex (cloud) · State: **blocked** · Last observed: 2026-11-06T23:21:25.106961+02:00
Activity: Cycle 1 merged in PR262 as6897fe56. Cloud Cycle 2 stopped with no retained change: package-only control exceeded overhead limits; one target-kind core pair returned earlier feedback but full gate+16.9%; shared source mtimes invalidated the UI comparison. Nine comparison attempts/67m29s, no completed ABBA/fresh confirmation. All3 edits executed intended tests/full gates; controlled focused/full/pipeline exits101, only the harmless test failed, original source bytes restored and core tests passed.
Next: Publish the scoped Cycle 2 evidence PR, complete current-head hosted CI and Codex code/security follow-through, and inspect findings. Required Council gate stays incomplete until actual cloud skill/routes are available. Normal merge only after applicable gates pass; obtain A selected final runtime and explicit shared-file handoff before Stage 2.
Resource waits: Required Council skill and authenticated independent delegate routes are unavailable in cloud.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Documentation/evidence only. No performance workflow/configuration or product change retained; prescribed repeated qualification incomplete after cache-validity stop.
- **checks:** All three edited paths ran nonzero intended tests/full offline gates. Controlled focused/full/pipeline exits101 with only harmless failure; restored core/fmt pass. Local tooling360 total=329passed+31skipped; Markdown/overview/diff/syntax and archive hashes pass. Current-head hosted CI and Codex code/security tracked by the PR linked from255. Council diagnosis/final review INCOMPLETE: zero sessions/no score.
- **integration:** Cycle 1 PR262 merged6897fe56. Cycle 2 branch reconciled with updater/runtime main and A265 mergea921e325; Cycle 2 not merged. Required Council gap is not waived by ordinary review/CI.
- **native:** Inapplicable to this headless documentation/workflow experiment; hosted macOS/platform gates preserved. No new native or owner-Mac observation.
- **owner acceptance:** Standing normal-merge authorization applies after applicable gates; no added manual native gate. Missing required Council review remains incomplete.

</details>

Waiting:

- resource: Required Council skill and authenticated independent delegate routes are unavailable in cloud.

### Implement and qualify the versioned Mac updater

Task ID: `mac-updater-253`
Owner: Agent B

Waiting:

- resource: Approved published command-enabled A/B pair unavailable; preview.1/.2 receipts/source approval verified but neither includes updater.
- resource: Mac mini last observed CGSSessionScreenIsLocked=true; native interaction/login/Gatekeeper pending.

### Consolidate shared provider policy after runtime handoff

Task ID: `shared-provider-263`
Owner: Agent C / Codex (cloud) · State: **blocked** · Last observed: 2026-11-06T23:21:25.106961+02:00
Activity: Read-only caller/policy mapping completed separately from timing windows. Existing chat/helper client sharing and duplicated normalization, refinement/search account checks and route construction are mapped. A264 response-limit transport delta inspected at98802606. No refactor implementation branch/PR or maintainability result.
Next: After Cycle 2 closeout and explicit A shared provider/runtime file release, branch from selected a921e325 (or a newer explicitly handed-off runtime), refresh the map, preserve contracts/response guard and coordinate later integration outside B first alpha.
Depends on: development-speed-255 when Cycle 2 closeout as its own PR with applicable reviews/checks and normal merge (reported satisfied: false)
Resource waits: A explicit shared provider/runtime file-ownership handoff on the selected final runtime.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Blocked. A264 merged98802606 and selected A265 mergeda921e325. Explicit C shared provider/runtime file handoff not observed. Read-only map only; no new client/routing policy implemented.
- **checks:** Not run for a refactor. Offline before/after request/account/route/bounds/checked-write tests and three policy-change tasks plus unseen confirmation remain required. No maintainability or elapsed-speed claim.
- **integration:** Separate branch/PR not created. Keep outside B first-alpha candidate and coordinate publication window/later adoption with A/B; preserve frozen254/256 experiments.
- **native:** Cloud offline implementation planned; existing A/B live/native receipts retain their source limits. Any observable behavior change needs its proper qualification.
- **owner acceptance:** Current assignment authorizes the separate refactor after sequencing gates; no release/publication authority inferred.

</details>

Waiting:

- prerequisite development-speed-255: Cycle 2 closeout as its own PR with applicable reviews/checks and normal merge (reported satisfied: false; task state: blocked)
- resource: A explicit shared provider/runtime file-ownership handoff on the selected final runtime.

### Compare Terra/Sol and improve maintained knowledge (#254)

Task ID: `knowledge-quality-254`
Owner: Agent D · State: **blocked** · Last observed: 2026-11-06T23:56:40.971406+00:00
Activity: Draft PRs #269 (comparison) and #270 (knowledge attempt) delivered. Original baseline retained; no model or guidance adoption. Final quality confirmation is incomplete after the bounded fresh run stopped on an uncertain outcome. Final Council: Sol 8 / Opus 7 / Kimi 8; offline corrections and 99 passing tests complete.
Next: Review draft PRs #269 and #270. Keep issue #254 open for unresolved quality acceptance; no further live dispatch under the stopped allocation.
Resource waits: Frozen cycle stopped after an uncertain outcome; fresh quality confirmation remains pending.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** One retention-guidance factor implemented, built and measured in paired runs; evidence only, no adopted production change.
- **checks:** Source-bound builds and judge Clippy passed. Post-review native AgentBench: 99 passed, 1 optional Inspect skip. Audit regression failed before and passed after correction; reader setup acceptance/refusal witnesses pass. Original executed source and results preserved.
- **integration:** Draft comparison https://github.com/ewq100/brn-rust/pull/269 and stacked knowledge attempt https://github.com/ewq100/brn-rust/pull/270. Hosted CI pending; no merge, default change or release.
- **native:** Headless synthetic only; desktop observation outside assignment
- **owner acceptance:** Not claimed. Both recommendation and knowledge-quality acceptance inconclusive; final fresh confirmation incomplete; no baseline/model default change.

</details>

Waiting:

- resource: Frozen cycle stopped after an uncertain outcome; fresh quality confirmation remains pending.

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

1 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `rig-046-253` — Upgrade the runtime to Rig 0.46 and qualify bounded reuse

## Dependencies

```mermaid
graph TD
  t3["Measure and improve development feedback (#255)"]
  t5["Consolidate shared provider policy after runtime handoff"]
  t3 -->|Cycle 2 closeout as its own PR with applicable reviews/checks and normal merge (not satisfied)| t5
```

Resource waits are listed on tasks and are not dependency edges.

Local page reflects the last saved observations in this record (2026-11-06T23:21:25.106961+02:00). A remote page reflects the last published commit. Neither indicates that work is live now.
