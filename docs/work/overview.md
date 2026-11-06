# Development overview

**Goal:** Finish Mac alpha package/update acceptance after updater258 merge and A response-limit hardening264.
**Owner attention:** Updater258 merged28051188 with all7 refreshed PR checks. Resulting-main verification running. Final package awaits A264 merged SHA/build slot; native clean profile and release-manifest approval remain separate.

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
Owner: Agent B · State: **active** · Last observed: 2026-11-06T21:29:20.771636+02:00
Activity: PR258 merged normally as28051188f7d20a817202ddf098dff5dc2f07e653; merged tree exactly matches reviewed0438bd60. All7 refreshed PR lanes pass; independent full integration/security clear. Codex code clean; separate security receipt explicitly reports no issues on0438bd60. Private full report login unavailable/not inspected; additional independent full-task security review clear, no waiver inferred. A264 now supplies response-limit hardening candidate; not selected as final until merged.
Next: Finish28051188 main checks42279604581; preserve informational Windows failures. Await A264 normal merge/selected SHA and heavy-build slot. Merge that agreed main into this acceptance branch, build native-ui arm64/macOS15+ desktop/CLI plus verified existing helpers, record exact package receipts; then one native session with A and authority-approved published A/B stock HTTPS witness.
Resource waits: A264 selected merged source and heavy-build slot not yet handed off.; Mac mini last observed locked; clean-profile normal Gatekeeper and tester-owned Copilot login needed after package ready.; Owner release-manifest approval/publication of command-enabled A/B needed for real stock HTTPS; legacy previews lack updater.

<details>
<summary>Gate status</summary>

Gates:

- **implementation:** Implemented bundled command, strict fixed-repository package verification, normal-closure/concurrency exclusion, staged recoverable replacement, shell-owned descriptor guard, repeatable recovery, alpha-compatible packaging and instructions. Independent reviewer has no remaining findings; production updater SHA256 7f5ea95caa55c0f552d54fd215a28c18d3a55e16b1011db7bfe6889d565157b5.
- **checks:** Refreshed combination:349passed+11skipped=360local tooling,fmt/syntax/overview/links clean. All7 hosted PR lanes pass. Current Codex code clean; separate automated security receipt explicitly reports no issues on0438bd60. Full-detail report account login unavailable/not inspected; additional independent full-task security review clear; workflow does not itself grant a fallback waiver. Prior source-bound local transport-substituted A/B and Linux ETXTBSY evidence retained. Resulting-main Ubuntu Core/CLI and both required Mac native lanes passed; Mac Core/CLI still running. Known optional Windows composer failure remains visible.
- **integration:** SATISFIED updater source integration: normal merge28051188, tree identical to reviewed0438bd60, all7PR lanes pass in42276377509. Resulting-main42279604581 in progress; no publication.
- **native:** PENDING: last observed locked. Requires final package after264 plus one unlocked clean disposable-profile session with A for normal per-app Gatekeeper approval, launch/update/reopen, synthetic data/settings and Copilot continuity/reconnect. Headless receipts are not native acceptance.
- **owner acceptance:** PENDING: installed final package/Copilot/native owner use before invited distribution; no approved command-enabled A/B published yet.

</details>

## Next

None recorded.

## Needs attention

### Implement and qualify the versioned Mac updater

Task ID: `mac-updater-253`
Owner: Agent B

Waiting:

- resource: A264 selected merged source and heavy-build slot not yet handed off.
- resource: Mac mini last observed locked; clean-profile normal Gatekeeper and tester-owned Copilot login needed after package ready.
- resource: Owner release-manifest approval/publication of command-enabled A/B needed for real stock HTTPS; legacy previews lack updater.

## Completed

Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.

1 completed tasks total; showing up to 10 most recently observed by last-updated time, not completion time. Older IDs, titles and history remain in the selected record.

- `rig-046-253` — Upgrade the runtime to Rig 0.46 and qualify bounded reuse

Local page reflects the last saved observations in this record (2026-11-06T21:29:20.771636+02:00). A remote page reflects the last published commit. Neither indicates that work is live now.
