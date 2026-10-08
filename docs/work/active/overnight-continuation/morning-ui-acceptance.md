# Consolidated morning UI acceptance — pending

This is the single authoritative interactive task for all overnight changes.
Everything below is **expected**, not observed overnight. Run only after the owner
unlocks the Mac and is available. Headless tests do not establish GUI acceptance.

## Exact build and synthetic workspaces

Merged range/guidance: PR90, `4f5b95e1c7b92f9efec54ea7f34f9097987cfdb6`.
Merged budgets: PR91, `409f68bedc112df550a5fcd6ebd884effff70101`.
Latest qualified P4 runtime source: `2abe6d3c3b47f1bc5fba56bb04e5ae09c257f38a`;
checkout commit `33ddac7aaf2f9f50801faceae83d80c9f78967e3` has the identical tree.
P4 final integrated commit and any subsequent slice: pending; update this receipt
before cutoff. Do not confuse implementation/qualification with personal acceptance.

Checkout: `/Users/evokessler/repos/brn-p2-email-docx-intake`.
Shipping desktop: `/Users/evokessler/repos/brn-p2-email-docx-intake/target/intake-ui/debug/brn-desktop`,
SHA-256 `561232d6aacbf5eb0fcdd017d9486406e0c4605a672ce0a0f6a5d02db8e9e434`.
Immutable matching CLI: `/private/tmp/brn-overnight-20261008/p4-conflict-brn`,
SHA-256 `888e9e752cf113f0b69cf07456b980e532437dd87699ff2f459a008d1870ad49`.
Build manifest: `/private/tmp/brn-overnight-20261008/p4-build-manifest.json`.
Helper is beside the binaries; manifest records its hash. Recheck hashes before
launch because the target executable may be replaced by a later qualified build.

Workspace A (email/DOCX/core journey):
`/private/tmp/brn-retained-qualification-bi3q58kf/headless/{data,vault,results}`.
Fixture: `experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml`.
Retained Sol group: `fa13347c-5349-41a9-94eb-28f6c749c9c6` (two Knowledge,
three Actions), still Draft. Luna comparison already Applied headlessly; inspect
it but do not approve every comparison group as distinct work.

Workspace B (North Quay authority/conflict):
`/private/tmp/brn-overnight-20261008/north-quay-case/{data,vault,receipts}`.
`ready.json` records original/import/Source/Current IDs and hashes.
Sources `approved-source.md`, `carrier-source.md`; approved Current
`north-quay-approved.md`. Sol analysis `4a1515ab-d5c4-4dcf-a7ec-bfe6d60f4d9f`
retains open Finding `1b34da31-3b0b-89b2-8ab7-74dc1544ec9a`, Knowledge
`1f62f279-bb00-8627-bf2f-08aceabb1766`, and two Action drafts. All remain Draft
at this checkpoint; later approval receipts must update this task. Luna comparison
is also Draft and should remain an unused comparison. All campaign/runtime/output
receipts are under `/private/tmp/brn-overnight-20261008`.

## Preparation and launch after unlock

Confirm final commit/build hashes and that no process owns the selected synthetic
folder. Do not delete locks, reset or migrate data if busy; record the condition.
Launch Workspace A from the checkout (ordinary desktop flags):

```sh
/Users/evokessler/repos/brn-p2-email-docx-intake/target/intake-ui/debug/brn-desktop \
  --data-dir /private/tmp/brn-retained-qualification-bi3q58kf/headless/data \
  --vault /private/tmp/brn-retained-qualification-bi3q58kf/headless/vault
```

Fully quit before launching Workspace B with its corresponding `data` and `vault`
paths. Use only these synthetic workspaces and ordinary existing-account login
if required. Never inspect credential contents or real email/vault/documents.
Do not repeat inference to inspect, edit or approve retained results. A fresh live
cancellation trial requires a deliberately selected morning run and budget; it is
not a prerequisite for inspection of the saved results.

Record date, final commit, executable hash, workspace and each observed outcome.
For failures retain exact error text, proposal/version, user action and before/after
state; collect synthetic-only screenshots where useful. Stop dependent approval on
integrity failure and preserve the draft for recovery. Record **pending** rather
than passed whenever a control/result cannot be exercised.

## One ordered journey

Allow 45–60 minutes. Essential path: steps 1–5 and 8–9, about 15–20 minutes.

| Step / user action | Expected result | Evidence / failure record |
| --- | --- | --- |
| 1. In A, reopen retained email and DOCX; inspect originals, inline/shared chart placements and unsupported spreadsheet notice | Reading requires no re-import/reconversion. Original, extraction, gaps and three occurrences of one image remain distinguishable | Selections, image occurrence labels, any absent content or unexpected conversion |
| 2. Fully quit/reopen; inspect Applied Source and retained proposals | Source/reading persist; proposed Knowledge/Actions remain separate reviewable work | Source ID/state, draft IDs/versions, any lost work or unexpected inference |
| 3. Review retained Sol group, exact before/after and citations; submit native approval for selected displayed versions | Repaired native submission applies only the selected current versions using the existing Source prerequisite; no duplicate Source | Selected order/versions, confirmation and receipts/error. Action-first works headlessly; record actual native behavior |
| 4. Fully quit/reopen; inspect saved notes, Actions and dashboard | Approved effects persist with citations, dates, unassigned owners where uncertain and open state | Note/Action UUIDs, status, dashboard discrepancies |
| 5. Reopen completed group and replay where exposed | No duplicate effects/new inference; Source and asset unchanged | Effect counts/replay receipt; compare retained headless hashes |
| 6. Inspect longer-evidence/range capability through retained output where available | Tail ranges retain complete-byte proof, exact offsets and scope; stale proof refuses. If no retained long-reading output is available, mark the interactive tool-use portion pending, not passed | Range/proof/result/error and selection. Deterministic near-1MiB range tests already passed |
| 7. Inspect Settings, saved budgets/history and error states; optionally select a separate disposable synthetic live cancellation trial | Defaults 8 rounds/300 seconds; presets 4/8/16/32 and 60/180/300/600. Ask/Inbox summary reflects selection. Active choice frozen, progress shows completed responses/admitted rounds. Stop/timeout/tool-limit causes distinguishable, retained drafts/ordinary partials remain; strict visual JSON stays completion-only. Legacy history budget unavailable | Selected/frozen values, progress, stopping/final cause, saved IDs, restart history. Without a fresh trial, active cancellation observation remains pending |
| 8. Quit A, launch B; read both Sources and approved Current, then saved Sol answer/Finding/proposals | Carrier cancellation risks the target but does not authorize a new commitment. Copied Kaia is not authorization. Exact opposing quotes, reasons, alternatives, unknown price/availability and reply timezone visible | Source/Current IDs, Finding state/proofs, displayed reasons and uncertainty; any misleading authority |
| 9. Review selected B proposals and their exact versions; approve only the intended consequences, restart and inspect | Current live Knowledge is a supplement, not a supersession. Two Actions remain distinct; due date/time and authorization boundary retained. Finding closure remains a separate explicit choice. Only approved effects persist | Approval requests/receipts, notes/Actions/Finding state, restart/replay; do not approve both model comparisons |
| 10. If a later predecessor-attachment slice is recorded as qualified here, explicitly select the Current predecessor on a suitable Draft, review generated protected History and successor, then approve the revised version | No silent semantic rewrite; owner text/comments/citations remain. Old stamp refuses. Generated History is readonly, successor editable, and exact pair persists after restart | Exact predecessor proof, before/after, version change, comments, stale/error states and history links. This feature is currently not implemented; keep pending until its receipt is added |

## Headless evidence already passed; interactive acceptance pending

PR90 local/required/postmerge gates passed. Fresh-process Action-first approval and
exact replay preserved Source/assets with no duplicate effects or inference.
PR91 local: Store 439 plus 26 final affected tests, workflow timeout/replay/drain 7,
real-Rig limits 2, CLI Ask 9; full default 1670 passed/17 existing ignores, doctests;
final native workflow/models 412 passed/15 ignores; desktop/CLI 525 passed; shipping
Clippy/builds and 52 fixtures passed. Required candidate CI passed; postmerge running.
P4: AI 145 passed/one existing ignore; integrated scenario and historical replay 2;
context guards 3; native AI/workflow Clippy, shipping CLI/desktop, 52 fixtures and
602 links passed. Independent review found no remaining actionable finding.

Six live CLI investigations completed. North Quay Luna retained one Action/one
Knowledge without a formal Finding; Sol retained two Actions/one Knowledge and a
reasoned Finding. Neither chose History: live History selection is unqualified;
deterministic supersession/History/recovery mechanics were exercised separately.
Owner added ten more shared trials; use the canonical ledger for later totals.
**No interactive native result or personal acceptance has been established.**

## Ready-to-paste morning agent prompt

> Read `docs/work/active/overnight-continuation/morning-ui-acceptance.md` and the
> latest overnight checkpoint in `/Users/evokessler/repos/brn-p2-email-docx-intake`.
> The Mac is unlocked and I am available. Verify final commit/build hashes and
> use only the recorded synthetic workspaces. Execute this single ordered UI
> journey, essential path first if time is limited. Record actual observations
> separately from expected behavior and prior headless evidence. Reuse retained
> outputs; do not repeat inference for review/approval. Exercise native approval,
> full quit/restart, reading, saved notes, Actions/dashboard, recorded budget/error
> states, conflict review and any subsequently qualified predecessor/revision slice.
> Preserve failures, drafts and exact evidence; do not reset data or touch my real
> vault. Report concrete defects and pending owner acceptance; mark unobserved
> or unavailable checks pending. A new live cancellation trial requires an explicit
> selected run and budget rather than being silently included.
