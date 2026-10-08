# Consolidated morning UI acceptance — pending

This is the single authoritative interactive task for all overnight changes.
Everything below is **expected**, not observed overnight. Run only after the owner
unlocks the Mac and is available. Headless tests do not establish GUI acceptance.

## Exact build and synthetic workspaces

Merged range/guidance: PR90, `4f5b95e1c7b92f9efec54ea7f34f9097987cfdb6`.
Merged budgets: PR91, `409f68bedc112df550a5fcd6ebd884effff70101`.
Merged P4: PR92, `bb66a1b884106fd18b08d2fa4864aff0537811bc`.
Merged predecessor attachment: PR93, `9f02a28e170388dc41fbb6ad2ccd7fd98bed1e44`.
All four required candidate and post-merge checks plus documentation passed for
PR90–PR93. PR94 merged at `b4e3a59a201b3e719df91278426d44388576959f` after all
required candidate checks; post-merge run 37842586660 is in progress. PR95 combined
candidate includes PR94, is locally qualified and awaiting final hosted integration.
Refresh this receipt after subsequent changes and before cutoff; qualification is
separate from personal acceptance.

Checkout: `/Users/evokessler/repos/brn-p2-email-docx-intake`.
Latest qualified combined runtime source: `b61c33ddd818fe6484145b37bcb7660606a29953` (PR95).
Shipping desktop: `/Users/evokessler/repos/brn-p2-email-docx-intake/target/intake-ui/debug/brn-desktop`,
SHA-256 `f56022a3f1a251351346dccf91aebf64f3c539166ec6986682b0c5813edd6a4f`.
Immutable matching CLI: `/private/tmp/brn-overnight-20261008/clarification-action-combined-brn`,
SHA-256 `898ef3332766b545ff5f1907c46595adffda9e8a2053d2546a6250e9334f489d`.
Manifest: `/private/tmp/brn-overnight-20261008/clarification-action-combined-build-manifest.json`.
Helper SHA-256 `8a7dae346aac826c2178c1e7666f1db6e52effa5288836a1fabb4a9ebc4bde71`.
This combined build includes predecessor attachment, Applied-intake Findings and
clarification-draft guidance. Historical precombine builds remain retained under
their own manifests. Recheck final hashes because later builds may replace targets.

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

Workspace C (long evidence):
`/private/tmp/brn-overnight-20261008/cedar-long-evidence/{data,vault,receipts}`.
Current `cedar-commissioning-record.md` is 949,665 bytes; final appendix starts at 948,370.
Sol analysis `0d2685b8-38c2-4cc9-a2ae-29ff6a9b9075` retains tail evidence, open
Finding `3145a4eb-cf18-833c-8be2-872b52fb6572`, Knowledge
`d544797c-04e7-8acc-bf07-337cfed7488b` and Action proposal
`9b5a1fb2-d5b3-8e29-b4b1-536ef686bf64` (still Draft). Its exact opposing pressure
quote is at bytes [948628,948733). Do not choose the commissioning-control record
as a predecessor merely to exercise History: the retained Knowledge supplements
those controls. Any attachment needs a semantically appropriate earlier review
and an explicit owner decision after exact before/after inspection.

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
| 6. In C, open long Current, inspect its final appendix, saved Sol answer and exact Finding evidence | Tail ranges retain complete-byte proof, exact offsets and scope; stale proof refuses. Saved Sol investigation observed two range calls and captured an exact tail quote. Individual range replies are not retained; direct interactive agent-tool exercise remains pending without a separately selected fresh trial | Range/proof/result/error and selection. Deterministic near-1MiB range tests already passed |
| 7. Inspect Settings, saved budgets/history and error states; optionally select a separate disposable synthetic live cancellation trial | Defaults 8 rounds/300 seconds; presets 4/8/16/32 and 60/180/300/600. Ask/Inbox summary reflects selection. Active choice frozen, progress shows completed responses/admitted rounds. Stop/timeout/tool-limit causes distinguishable, retained drafts/ordinary partials remain; strict visual JSON stays completion-only. Legacy history budget unavailable | Selected/frozen values, progress, stopping/final cause, saved IDs, restart history. Without a fresh trial, active cancellation observation remains pending |
| 8. Quit A, launch B; read both Sources and approved Current, then saved Sol answer/Finding/proposals | Carrier cancellation risks the target but does not authorize a new commitment. Copied Kaia is not authorization. Exact opposing quotes, reasons, alternatives, unknown price/availability and reply timezone visible | Source/Current IDs, Finding state/proofs, displayed reasons and uncertainty; any misleading authority |
| 9. Review selected B proposals and their exact versions; approve only the intended consequences, restart and inspect | Current live Knowledge is a supplement, not a supersession. Two Actions remain distinct; due date/time and authorization boundary retained. Finding closure remains a separate explicit choice. Only approved effects persist | Approval requests/receipts, notes/Actions/Finding state, restart/replay; do not approve both model comparisons |
| 10. Explicitly select the Current predecessor on a suitable Draft, review generated protected History and successor, then approve the revised version | No silent semantic rewrite; owner text/comments/citations remain. Old stamp refuses. Generated History is readonly, successor editable, and exact pair persists after restart | Exact predecessor proof, before/after, version change, comments, stale/error states and history links. Implemented, independently reviewed, final shipping and required candidate/post-merge CI passed; interactive acceptance pending |

## Headless evidence already passed; interactive acceptance pending

PR90 local/required/postmerge gates passed. Fresh-process Action-first approval and
exact replay preserved Source/assets with no duplicate effects or inference.
PR91 local: Store 439 plus 26 final affected tests, workflow timeout/replay/drain 7,
real-Rig limits 2, CLI Ask 9; full default 1670 passed/17 existing ignores, doctests;
final native workflow/models 412 passed/15 ignores; desktop/CLI 525 passed; shipping
Clippy/builds and 52 fixtures passed. Required candidate and postmerge CI passed.
P4: AI 145 passed/one existing ignore; integrated scenario and historical replay 2;
context guards 3; native AI/workflow Clippy, shipping CLI/desktop, 52 fixtures and
602 links passed. Independent review found no remaining actionable finding. P4 required candidate and post-merge CI passed; it merged. Predecessor attachment targeted Store 7, workflow 7, desktop/widget 7, CLI parser 1 and subprocess 2 passed; affected Clippy and independent full-candidate review passed. Final default1694/17existing ignores with doctests and52fixtures passed. Native workflow/models418/15ignores, desktop/CLI535, final Clippy/shipping builds/52fixtures/603links passed. PR93 merged normally; required candidate and post-merge CI passed.

PR94 final Store 450, AI 146/one ignore and native workflow/models 420/15 ignores, combined Clippy/shipping/52 fixtures/603 links passed; required candidate CI passed and merged, post-merge pending. PR95 final combined AI 147/one ignore and AI Clippy/shipping passed; final hosted CI pending. Both complete implementations received independent read-only reviews with no actionable findings. Their new interactive states remain pending and must be included once the combined final build lands.

Eight live CLI investigations completed, four per model. Cedar Sol used range calls and retained tail evidence/review consequences; Luna found facts through search but submitted no drafts. North Quay Luna retained one Action/one
Knowledge without a formal Finding; Sol retained two Actions/one Knowledge and a
reasoned Finding. Neither chose History: live History selection is unqualified;
deterministic supersession/History/recovery mechanics were exercised separately.
Owner added ten shared trials; eight of sixteen used at this checkpoint. Use the canonical ledger for later totals.
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
