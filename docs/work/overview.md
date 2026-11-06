# Development overview

**Goal:** Complete native and approved published-update acceptance for the qualified combined Mac alpha on selected runtimea921e325.
**Owner attention:** A handoff resolved. B built/qualified exacta921e325 local alpha.1/build3 and alpha.2/build4;32tests, relocated imports, local substituted-transport update and full backup/restore pass. Slot released21:56:38Z; B holds no Mac mini reservation. Owner clarified D/E use the separate MacBook; no cross-host wait. Native profile/login and explicit release-manifest approval remain.

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
Owner: Agent B · State: **active** · Last observed: 2026-11-06T22:10:38.537844+00:00
Activity: Exact selected a921e32533dc03134ed8a2001bd82f27194c3302 compiled cleanly in B target (native-ui,arm64,macOS15+,Rust1.98.1); two helpers built/revalidated; signed local A3/B4 packages qualified.32packaging/updater/preparation cases pass; both helpers8formats/16imports, real local A→B/external-file preservation/B headless reopen and full synthetic backup/restore pass. A dependency resolved; Mac slot RELEASED21:56:38Z before22:30cap. Local deliverables and Shared copy ready; no native/live/stock HTTPS/publication claim.
Next: Owner reviews the draft alpha.1/build3 manifest/notes/publication proposal; do not merge/publish without explicit release approval. Existing workflow must build actual approved main A, then later approved increasing B before unchanged stock HTTPS witness. Arrange one combined B/A native session when disposable profile unlocked/tester Copilot login available. B holds no slot; D/E use the separate MacBook and need no B release window. Coordinate only actual shared host/account resources.
Resource waits: Mac mini screen last observed locked; one disposable-profile native Gatekeeper/launch/Copilot login session needs owner presence. Second Copilot account required for observed account-switch case, otherwise not observed.; Explicit owner approval and actual immutable publication of command-enabled A/B are required for real stock HTTPS; existing legacy previews lack updater.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Implemented bundled command, strict fixed-repository package verification, normal-closure/concurrency exclusion, staged recoverable replacement, shell-owned descriptor guard, repeatable recovery, alpha-compatible packaging and instructions. Independent reviewer has no remaining findings; production updater SHA256 7f5ea95caa55c0f552d54fd215a28c18d3a55e16b1011db7bfe6889d565157b5.
- **checks:** Refreshed combination:349passed+11skipped=360local tooling,fmt/syntax/overview/links clean. All7 hosted PR lanes pass. Current Codex code clean; separate automated security receipt explicitly reports no issues on0438bd60. Full-detail report account login unavailable/not inspected; additional independent full-task security review clear; workflow does not itself grant a fallback waiver. Prior source-bound local transport-substituted A/B and Linux ETXTBSY evidence retained. All4 required resulting-main contexts passed in42279604581, including25 Mac packaging/updater cases and the original Ubuntu fixture test. All3 optional Windows lanes fail in unchanged composer/skills paths, retained visibly. Final selecteda921e325: all4 required hosted contexts pass in42294766975;32local packaging/updater/preparation cases pass; relocated helpers8formats/16imports and local real-package A→B/full-restore pass. Original failed verifier assumptions/path failure retained and diagnosed; no production repair or skipped case. Independent approval review clear onac5deeef; archival-verifier P3 documentation fixed97199fe2 and independently rechecked with31 checksums passing. Final docs/currentmain machine clarification follows; no new runtime changes.
- **integration:** SATISFIED updater source integration: normal merge28051188, tree identical to reviewed0438bd60, all7PR lanes pass in42276377509. All4 required resulting-main42279604581 contexts pass; overall run fails only3 inherited optional Windows lanes; no publication. A264/265 handoff resolved; a921e325 is selected and normally reconciled into acceptance branch. Publication proposal stays draft pending explicit owner approval; actual approved main manifest remains legacy until that approval/merge. Currentmain c388da26 (C266 docs/evidence only) normally merged into acceptance branch; C task objects preserved exactly. Qualified runtime staysa921e325; runtime/Cargo/CI tree unchanged.
- **native:** PENDING: exact combined packages ready, screen locked. See current native-session runbook for one B/A disposable-profile Gatekeeper/Copilot/backup/update session. No Mac slot reserved; headless results are not native acceptance.
- **owner acceptance:** PENDING: owner native use before invited distribution. Local alpha.1/build3 and alpha.2/build4 are proposed-version candidates; approved published A/B and stock HTTPS remain outstanding. Release-manifest proposal is draft, no merge/publication authority inferred.

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

- resource: Mac mini screen last observed locked; one disposable-profile native Gatekeeper/launch/Copilot login session needs owner presence. Second Copilot account required for observed account-switch case, otherwise not observed.
- resource: Explicit owner approval and actual immutable publication of command-enabled A/B are required for real stock HTTPS; existing legacy previews lack updater.

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

Local page reflects the last saved observations in this record (2026-11-06T22:10:38.537844+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
