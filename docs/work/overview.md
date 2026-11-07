# Development overview

**Goal:** Qualify the owner-selected combined current-main Mac alpha with272,267and the focused review-target fix, then complete approved release/native-update acceptance.
**Owner attention:** B native session still waits on owner unlock;82657qualified artifacts unchanged.29af383bcode/security completed clean,0threads; CI could not start while newer-main shared-record conflict existed. B reconciles mainBab8salvage, preserving all other track objects, and refreshes integrated-head CI/reviews. E reports measurement batches finished06:54UTC/no current local reservation;274 remains separate. No release/publication approval.

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
Owner: Agent B (MacBook continuation) · State: **active** · Last observed: 2026-11-07T07:19:25.360738+00:00
Activity: Screen still locked, so normal quit/reopen/closure remains pending.29af383bcode/security completed clean with0threads, but no PR CI run because mainBab8shared-record conflicts. Reconcile mainBab8salvage normally; runtime files auto-merge and only shared record/overview need resolution. Preserve82657installables/active app/source-bound native evidence. E measurement batches completed06:54UTC and local reservation released; PR274 not adopted.
Next: Verify and push normal Bab8 reconciliation, refresh current-head CI/code/security; retain explicit source-delta scope (default usage/trace reporting plus capability-gated judge/evidence tooling). After owner unlock, finish normal quit/reopen/closure of existing82657native session, then clean-user Finder and approved-release steps. No runtime artifact relabel/rebuild or publication approval inferred.
Resource waits: MacBook CGSSessionScreenIsLocked=Yes still blocks native normal quit/reopen/closure; task-owned app remains idle/running. Existing owner unlock request pending.; E reports measurement batches finished06:54UTC; no local measurement/build reservation now. Coordinate any new actual overlap; PR274 product candidate remains separate.; Clean-user ordinary Finder still needs disposable standard account/sign-in, short name/readiness and ordinary Gatekeeper/Copilot participation.; Explicit release approval and actual approved published A/B still required. Integrated-head checks and source-delta disposition precede any publication decision.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Implemented bundled command, strict fixed-repository package verification, normal-closure/concurrency exclusion, staged recoverable replacement, shell-owned descriptor guard, repeatable recovery, alpha-compatible packaging and instructions. Independent reviewer has no remaining findings; production updater SHA256 7f5ea95caa55c0f552d54fd215a28c18d3a55e16b1011db7bfe6889d565157b5. Bounded new-article review-target correction implemented; exact red-before/green-after native-state test and18reviewtests pass.
- **checks:** Refreshed combination:349passed+11skipped=360local tooling,fmt/syntax/overview/links clean. All7 hosted PR lanes pass. Current Codex code clean; separate automated security receipt explicitly reports no issues on0438bd60. Full-detail report account login unavailable/not inspected; additional independent full-task security review clear; workflow does not itself grant a fallback waiver. Prior source-bound local transport-substituted A/B and Linux ETXTBSY evidence retained. All4 required resulting-main contexts passed in42279604581, including25 Mac packaging/updater cases and the original Ubuntu fixture test. All3 optional Windows lanes fail in unchanged composer/skills paths, retained visibly. Final selecteda921e325: all4 required hosted contexts pass in42294766975;32local packaging/updater/preparation cases pass; relocated helpers8formats/16imports and local real-package A→B/full-restore pass. Original failed verifier assumptions/path failure retained and diagnosed; no production repair or skipped case. Independent approval review clear onac5deeef; archival-verifier P3 documentation fixed97199fe2 and independently rechecked with31 checksums passing. Final docs/currentmain machine clarification follows; no new runtime changes. MacBook refreshed checkpoint94c11669: all7proposal lanes pass in42302755459; current code/security receipts completed/no findings; zero review threads. Development native preflight passes. These reviews/checks apply to94c11669, not later documentation commits. MacBook continuation preparation:16overview tests,741Markdown files/3705links at last scan, generated overview/whitespace and27historical checksums pass;5other task objects preserved. New exacta921 CLI built and synthetic backup/restore smoke passed, with source/binary receipt. Final documentation scan is recorded in preparation-checks.json. Continuationb65d3dd4 all7CI pass in42335708965 and completed clean current code/security receipts, zero review threads, verified before current staged-native work. New MacBook rebuild desktop+bothhelpers+package/signature and relocated helper8formats/16imports qualification pass. Native-created backup quick_check and restored records/notes/history/settings/projects/threads exact comparisons pass. Local update qualifier exit1 correctly refused running BRN; qualification remains incomplete, original failure log retained. Current2e7bfce3 BRN CI42340527801 completed success; code6702537631 and security6702573318 report no findings on that head. New native evidence/target correction needs final-head checks/review. Local substituted MacBook update rerun passed after normal closure, prior refusal retained. New correction full native-state451pass/1existingignore at2threads/canonicalTMPDIR. First full attempt443pass/8fail/1ignore; canonical-temp parallel450pass/1workerfail/1ignore; isolatedworkerpassed. Preserve all attempts; no CI/assertion/deadline changes or baseline reliability claim. Combined82657cab now all7completed CI jobs pass in42345378178, code6703169631/security6703239421 completed no findings,0threads; private full report uninspected. New locked/offline desktop/CLI, strict package platform/signatures, unchanged helper binding, relocated8formats/16imports, substituted update/headless B/restore and CLI6category backup restore pass. First notices preparation lacked generated source association, original failure retained; standard offline regeneration byte-identical before fresh success. Later docs head needs applicable checks/review. Head29af383bcode6703416616/security6703456264 completed clean,0threads. No PR workflow run existed because newer-main merge conflicts; do not label this CI passed or pending execution. Normal Bab8reconciliation now requires fresh integrated-head CI/reviews.
- **integration:** SATISFIED updater source integration: normal merge28051188, tree identical to reviewed0438bd60, all7PR lanes pass in42276377509. All4 required resulting-main42279604581 contexts pass; overall run fails only3 inherited optional Windows lanes; no publication. A264/265 handoff resolved; a921e325 is selected and normally reconciled into acceptance branch. Publication proposal stays draft pending explicit owner approval; actual approved main manifest remains legacy until that approval/merge. Currentmain c388da26 (C266 docs/evidence only) normally merged into acceptance branch; C task objects preserved exactly. Qualified runtime staysa921e325; runtime/Cargo/CI tree unchanged. Draft release-approval PR268 opened; no merge/publication. Proposal94c11669 own CI7/7 and code/security receipts now completed/no findings, freshly inspected on MacBook. PR stays draft/unmerged pending explicit owner release approval; later continuation documentation needs applicable renewed checks/review. Main has since advanced75f37221 with accepted auth-race repair272. Currenta921native artifacts remain source-bound historical/functional evidence; final release source and affected qualification must reconcile that fix before publication. No automatic merge/publication. Currentmain8b07e035 contains267 despite selected first-alpha exclusion. No silent adoption/revert or publication-policy change; source decision remains open. Owner subsequently selected currentmain272+267+targetfix for new qualification only; source conflict resolved for testing, not publication. Normal main8b07e035 merge now resolves only shared task record/generated overview; all5non-B task objects exactly retained from main. New source-bound package qualification pending. MainBab8salvage is now normally reconciled for PR preparation; all non-B task objects exactly from main and B latest task preserved. Installed/package qualification remains82657, not silently moved. Default-build delta inspected: complete-usage reporting and safe read_message trace metadata; extra judge is capability-spike gated; no provider/model/tool/write/launcher change inferred. Current combined head checks still required; no274adoption/publication.
- **native:** PARTIAL PASSED combined82657alternate-directory new-source checks: retained separate Copilot account/settings, source-linked live consolidation exact review with correct new target/controls, Apply/3recordUndo, fresh referenced writing Applyv8/Undov9 exactv7data, retained stalev6→v7 refusal/Discard, WebOff0/WebOn12actual sources/citation/Stop-no-late, LunaOn protected table/chart/Asset. Existing notes/projects/settings/recovery preserved at startup. Earliera921normalquit/reopen unsaved draft and native backup/restore remain separately scoped; combined CLI6category backup/restore passed. Combined normalquit/reopen/finalclosure and nativeB pending locked screen. Finder/clean-user/published browser-Gatekeeper/stockHTTPS/owner acceptance pending. Real second-account/in-flight switch/denials/network outage, enabled helper/context-summary live paths unobserved; requested high effective unknown.
- **owner acceptance:** PENDING: owner native use before invited distribution. Local alpha.1/build3 and alpha.2/build4 are proposed-version candidates; approved published A/B and stock HTTPS remain outstanding. Release-manifest proposal is draft, no merge/publication authority inferred.

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
Owner: Issue 256 restart lead on MacBook · State: **active** · Last observed: 2026-11-07T05:38:36.807435+00:00
Activity: Merged main 8b07e035 provider refactor into salvage candidate. Rust sources auto-merged; shared record reconciled by retaining all main tasks plus issue256 restart. Regenerated overview.
Next: Requalify integrated candidate, push normal merge, refresh current-head Codex reviews/CI, merge salvage then close old PRs and start product window.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Small extraction implemented. Council selection findings addressed. Codex P2 fixed with cache-hit and failed-summary regressions.
- **checks:** Prior head passed 94 Python and 52 focused Rust tests plus strict Clippy and clean Codex code/security review. Main integration invalidates affected current-head checks/reviews; refreshing.
- **integration:** No PR or merge yet. Old PRs remain open until replacement merged.
- **native:** No native observation required for the selected evidence-only changes.
- **owner acceptance:** Owner authorized focused merge after review and CI; product qualification pending.

</details>

## Next

None recorded.

## Needs attention

### Implement and qualify the versioned Mac updater

Task ID: `mac-updater-253`
Owner: Agent B (MacBook continuation)

Waiting:

- resource: MacBook CGSSessionScreenIsLocked=Yes still blocks native normal quit/reopen/closure; task-owned app remains idle/running. Existing owner unlock request pending.
- resource: E reports measurement batches finished06:54UTC; no local measurement/build reservation now. Coordinate any new actual overlap; PR274 product candidate remains separate.
- resource: Clean-user ordinary Finder still needs disposable standard account/sign-in, short name/readiness and ordinary Gatekeeper/Copilot participation.
- resource: Explicit release approval and actual approved published A/B still required. Integrated-head checks and source-delta disposition precede any publication decision.

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

Local page reflects the last saved observations in this record (2026-11-07T07:19:25.360738+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
