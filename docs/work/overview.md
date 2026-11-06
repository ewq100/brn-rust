# Development overview

**Goal:** Complete native and approved published-update acceptance for the qualified combined Mac alpha on selected runtimea921e325.
**Owner attention:** A handoff resolved. B built/qualified exacta921e325 local alpha.1/build3 and alpha.2/build4;32tests, relocated imports, local substituted-transport update and full backup/restore pass. Slot released21:56:38Z; D/E free through23:00Z, no B reservation afterward. Native profile/login and explicit release-manifest approval remain.

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
Owner: Agent B · State: **active** · Last observed: 2026-11-06T22:02:38.443784+00:00
Activity: Exact selected a921e32533dc03134ed8a2001bd82f27194c3302 compiled cleanly in B target (native-ui,arm64,macOS15+,Rust1.98.1); two helpers built/revalidated; signed local A3/B4 packages qualified.32packaging/updater/preparation cases pass; both helpers8formats/16imports, real local A→B/external-file preservation/B headless reopen and full synthetic backup/restore pass. A dependency resolved; Mac slot RELEASED21:56:38Z before22:30cap. Local deliverables and Shared copy ready; no native/live/stock HTTPS/publication claim.
Next: Owner reviews the draft alpha.1/build3 manifest/notes/publication proposal; do not merge/publish without explicit release approval. Existing workflow must build actual approved main A, then later approved increasing B before unchanged stock HTTPS witness. Arrange one combined B/A native session when disposable profile unlocked/tester Copilot login available. B holds no slot; D/E window free now–23:00Z, then no standing B reservation.
Resource waits: Mac mini screen last observed locked; one disposable-profile native Gatekeeper/launch/Copilot login session needs owner presence. Second Copilot account required for observed account-switch case, otherwise not observed.; Explicit owner approval and actual immutable publication of command-enabled A/B are required for real stock HTTPS; existing legacy previews lack updater.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Implemented bundled command, strict fixed-repository package verification, normal-closure/concurrency exclusion, staged recoverable replacement, shell-owned descriptor guard, repeatable recovery, alpha-compatible packaging and instructions. Independent reviewer has no remaining findings; production updater SHA256 7f5ea95caa55c0f552d54fd215a28c18d3a55e16b1011db7bfe6889d565157b5.
- **checks:** Refreshed combination:349passed+11skipped=360local tooling,fmt/syntax/overview/links clean. All7 hosted PR lanes pass. Current Codex code clean; separate automated security receipt explicitly reports no issues on0438bd60. Full-detail report account login unavailable/not inspected; additional independent full-task security review clear; workflow does not itself grant a fallback waiver. Prior source-bound local transport-substituted A/B and Linux ETXTBSY evidence retained. All4 required resulting-main contexts passed in42279604581, including25 Mac packaging/updater cases and the original Ubuntu fixture test. All3 optional Windows lanes fail in unchanged composer/skills paths, retained visibly. Final selecteda921e325: all4 required hosted contexts pass in42294766975;32local packaging/updater/preparation cases pass; relocated helpers8formats/16imports and local real-package A→B/full-restore pass. Original failed verifier assumptions/path failure retained and diagnosed; no production repair or skipped case.
- **integration:** SATISFIED updater source integration: normal merge28051188, tree identical to reviewed0438bd60, all7PR lanes pass in42276377509. All4 required resulting-main42279604581 contexts pass; overall run fails only3 inherited optional Windows lanes; no publication. A264/265 handoff resolved; a921e325 is selected and normally reconciled into acceptance branch. Publication proposal stays draft pending explicit owner approval; actual approved main manifest remains legacy until that approval/merge.
- **native:** PENDING: exact combined packages ready, screen locked. See current native-session runbook for one B/A disposable-profile Gatekeeper/Copilot/backup/update session. No Mac slot reserved; headless results are not native acceptance.
- **owner acceptance:** PENDING: owner native use before invited distribution. Local alpha.1/build3 and alpha.2/build4 are proposed-version candidates; approved published A/B and stock HTTPS remain outstanding. Release-manifest proposal is draft, no merge/publication authority inferred.

</details>

## Next

None recorded.

## Needs attention

### Implement and qualify the versioned Mac updater

Task ID: `mac-updater-253`
Owner: Agent B

Waiting:

- resource: Mac mini screen last observed locked; one disposable-profile native Gatekeeper/launch/Copilot login session needs owner presence. Second Copilot account required for observed account-switch case, otherwise not observed.
- resource: Explicit owner approval and actual immutable publication of command-enabled A/B are required for real stock HTTPS; existing legacy previews lack updater.

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

1 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `rig-046-253` — Upgrade the runtime to Rig 0.46 and qualify bounded reuse

Local page reflects the last saved observations in this record (2026-11-06T22:02:38.443784+00:00). A remote page reflects the last published commit. Neither indicates that work is live now.
