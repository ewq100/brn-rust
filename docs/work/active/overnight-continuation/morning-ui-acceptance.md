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
required candidate checks; all required post-merge checks/docs passed in 37842586660. PR95 merged normally
at `bd6947511ac9f5e445078e831d61ac3f5533fe06` after all required candidate
checks/docs; all required post-merge checks/docs passed in 37844389582.
Refresh this receipt after subsequent changes and before cutoff; qualification is
separate from personal acceptance.

Checkout: `/Users/evokessler/repos/brn-p2-email-docx-intake`.
Latest qualified combined candidate: `d6ae95f3fb61bf8df52eb33c78770a666cb42c64`
(merged PPTX PR98 plus operational backups). Main is
`544696834d93cf56b82e76a857f2c44cdca7823b`; PR90–PR99 required candidate checks
passed. PR90–PR98 required post-merge checks passed. PR99 post-merge run37857753049
is active. Action compensation final composition qualification remains pending. Immutable shipping runtime:
`/Users/evokessler/repos/brn-overnight-artifacts-20261008/pptx-backup-runtime`.
Desktop: `/Users/evokessler/repos/brn-overnight-artifacts-20261008/pptx-backup-runtime/brn-desktop`,
SHA-256 `3a2599476613703690401e6a98617d4321eb8d7e1fbb71d879b3a560b9cd4717`.
CLI: `/Users/evokessler/repos/brn-overnight-artifacts-20261008/pptx-backup-runtime/brn`,
SHA-256 `c2c7ddd7bde69885ade4eebaef1ce03f2dbfb4abee5dc318b75290cc6bee0644`.
Helper: `/Users/evokessler/repos/brn-overnight-artifacts-20261008/pptx-backup-runtime/brn-intake-helper`,
SHA-256 `7f058c359a092559cda55b07c3880b44e637257818ad60c725da8b1d99a9f8e5`.
Manifest: `/Users/evokessler/repos/brn-overnight-artifacts-20261008/pptx-backup-runtime/build-manifest.json`.
This runtime includes SessionV18, PPTX and locally qualified automatic backups;
it excludes pending Action compensation. Refresh final receipt before cutoff and
never use historical executables after newer schema migration. Bound state/vault
folders remain in place.
Actual GUI/personal acceptance remains pending.

A separate durable evidence copy is under
`/Users/evokessler/repos/brn-overnight-artifacts-20261008/qualification-evidence`,
with a SHA-256 archive manifest. It contains synthetic inputs, retained model
outputs, review/approval receipts, scripts, logs and historical executables.
Bound `data`/`vault` folders and credentials are excluded and remain in their
recorded locations. Use the final runtime above for acceptance; historical
executables are evidence and must not reopen state migrated by a newer schema.
Running gate logs may be partial until the final refresh.

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

Workspace D (Applied-private-intake conflict and saved consequences):
`/private/tmp/brn-overnight-20261008/linden-applied-intake/{data,vault,receipts}`.
`ready.json` and `intake-binding.json` retain original Source/citation/approval
proofs. Current `linden-approved.md`. Sources `approved-source.md` and `facilities-source.md`.
Completed Sol retry `a25aac84-74c8-40e7-b4c2-2d054df0b807` retains Open Finding
`7dfdff0d-9749-86c4-bb9a-dc82a9809e56`; two Knowledge and two Action proposals
are already Applied headlessly. Inspect/replay these, do not approve duplicates.
Luna comparison remains Draft and initial Sol failure remains inspectable.
`approval-replay-summary.json`, `completed-analysis-replay-summary.json` and
`approval-preserved-evidence.json` record passed fresh-process qualification.

Workspace E (accurate new email evidence, zero inference):
`/private/tmp/brn-overnight-20261008/email-evidence-caveats-case/{data,vault,inputs,receipts}`.
`ready.json` retains the first within-field variant; `repeated-physical-fields-ready.json`
retains the final corrected helper variant with all physical fields in order.
Source Drafts `a555a6ad-654d-4ed2-9f58-4fd49ae2db0b` and
`3d98b451-e647-44d3-924d-e8bf57b84965` are deliberately unapproved. Inspect their
retained originals/decoded IDs/caveats without re-import, conversion or inference.
Compare with the older immutable Linden extraction wording in D; do not rewrite it.

North Quay retained owner-comment revision: Luna knowledge proposal
`f1700e6e-1051-8a95-997a-e60075d74312` is now Draft/version4, after owner edit,
comment and one live Rewrite. Exact before version3/after version4/job receipts are
`/private/tmp/brn-overnight-20261008/north-quay-case/receipts/owner-comment-rewrite`.
Inspect that comparison without requesting another Rewrite or approving duplicate
comparison work. Sol North Quay proposals remain Draft for native approval.

Workspace F (partial PPTX, zero inference preparation):
`/private/tmp/brn-overnight-20261008/pptx-retained-case/{data,vault,inputs,receipts}`.
Harbor Source5eeb31f3-7837-4674-b889-4c77cf0d8f13 and twins Source
313bd19e-107d-4d16-9fc4-5cd8334b6f97 remain Draft. Quay Source
bbbfcd8f-5215-4890-afea-ac4415be5e51 and its two assets are already Applied,
with exact replay and Original identities preserved. `ready.json` has all exact
IDs/proofs/receipt paths. Harbor chart facts are absent; inspect the original.
Quay notes are distinct, hidden slide order retained; twin canonical/generic MIME
attachments preserve separate children and six occurrences sharing two assets.
PPTX final combined local gates/build passed; required hosted CI/integration pending.
Paired saved investigations13/14 retained under receipts/paired-private-investigation;
all model proposals remain Draft. Approve only a selected group with exact Source
prerequisite; do not approve both comparisons or ask for repeat inference.

## Preparation and launch after unlock

Confirm final commit/build hashes and that no process owns the selected synthetic
folder. Do not delete locks, reset or migrate data if busy; record the condition.
Launch Workspace A from the checkout (ordinary desktop flags):

```sh
/Users/evokessler/repos/brn-overnight-artifacts-20261008/pptx-backup-runtime/brn-desktop \
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

Allow 85–115 minutes for all workspaces/new controls. Essential path: steps 1–5 and 8–9, about 20–25 minutes.

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
| 11. Quit B/C and open D; inspect saved private-intake analysis, both Sources, approved Current, Open Finding, two saved Knowledge notes and two Actions | Formal Finding retains exact Applied Source lineage and opposing director Source, tentative preference/reasons/alternatives. Capacity 26 versus18, four mobility-aid users, access restrictions, authority and 12 October noon +03:00 reply cut-off remain visible. All four selected consequences are already Applied; Finding remains Open | Source/stamp/proof IDs, saved note/Action UUIDs, separate Finding state, failed Sol run/error and comparison Draft; no new inference or duplicate approval |
| 12. Select a completed synthetic session, Archive, inspect Archived history, quit/reopen and Restore; inspect old operation replay if exposed | Same UUID, turns, budgets and creation/activity times; archived history readable, explicit Restore needed for new work; pending/stale controls preserve composer/editor/review buffers. Busy/draining Archive refuses without cancellation | Selected UUID, lifecycle versions/receipt, displayed filter/banner, preserved text and restart history. Session implementation, independent review and final local qualification passed; required candidate/post-merge CI passed and PR97 merged |
| 13. In E, inspect both retained plain-text EML variants and compare the older extraction in D | New extraction retains every decoded In-Reply-To/References value across scalar/list and physical fields in order, including repeats, and distinguishes header claims from authentication/thread proof; no generic false absence/HTML/remote claim. Actual HTML/remote/CID gaps remain specific. Old snapshots retain their original wording and partial status | Exact extraction IDs/build/helper hash and visible caveat; do not reconvert old evidence merely to change wording. Helper/local CLI qualification and independent review passed; all required candidate/post-merge CI passed and PR96 merged |
| 14. Once final PPTX qualification/build is recorded, open the retained Harbor and unrelated Quay presentations and repeated email attachments, inspect slide/notes/table/image evidence, exact Source/assets approval and restart | Presentation order and hidden labels are explicit; notes distinct from slide text; repeated picture occurrences retain parents/shared bytes; chart/SmartArt/layout gaps remain visible without invented facts. Reopen without conversion/inference | Final fixture/candidate/snapshot/Source IDs and asset hashes, slide/notes locators, partial gaps and approval receipts. PPTX implemented/reviewed clean;41 intake,1729default including reused Store461,426native workflow/models,549desktopCLI,Clippy/shipping/fixtures passed. PR98 merged after required candidate CI passed; required post-merge checks passed. Do not mark observed |
| 15. Once final backup build is recorded, inspect Settings backup status, change a disposable draft/comment, checkpoint, refresh and restart; inspect warning fixture only if recorded | Last known usable copy/path/time or unknown startup time is distinct from failure/retention warning. Automatic changed-state copies and final joined shutdown preserve complete state. A backup warning never turns a committed approval/Save/chat into failed/retryable work, and owner buffers remain | Exact checkpoint path/status, retained changed draft/comments/session/budget/Actions/Findings and warning. Backup implementation, complete/merge-delta independent reviews and final local qualification passed; required candidate CI passed and PR99 merged; post-merge checks active. No destructive Restore or private data test |
| 16. Once final compensation qualification/build is recorded, preview Undo on a disposable approved Action-only replacement; inspect complete prior/current details, confirm, restart/replay, then inspect a changed/completed refusal | Prior details return as a new revision with immutable origin/history preserved. Waiting clock semantics are shown honestly; changed or Completed work refuses. Creation/mixed Undo remain unsupported. Preview has no effect and exact replay never overwrites later edits | Source/compensation operation UUIDs, complete before/after, versions/origin, Waiting dates, refusal text and replay receipt. Implementation and independent review passed; focused Store/native/workflow/CLI checks passed, broad final gates/CI/integration pending. Actual native acceptance pending |


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

PR94 final Store 450, AI 146/one ignore and native workflow/models 420/15 ignores, combined Clippy/shipping/52 fixtures/603 links passed; required candidate and post-merge CI passed and merged. PR95 final combined AI 147/one ignore and AI Clippy/shipping passed; required candidate CI passed and merged; post-merge passed. Both complete implementations received independent read-only reviews with no actionable findings. Their new interactive states remain pending and must be included once the combined final build lands.

Fourteen live CLI investigations attempted: seven Luna, seven Sol, including one generic failed Sol run and a separately counted manual retry. Linden completed Applied-private-intake Finding and four exact consequences; Action-first approval/restart/replay passed without inference and full evidence identities stayed unchanged. Cedar Sol used range calls and retained tail evidence/review consequences; Luna found facts through search but submitted no drafts. North Quay Luna retained one Action/one
Knowledge without a formal Finding; Sol retained two Actions/one Knowledge and a
reasoned Finding. Neither chose History: live History selection is unqualified;
deterministic supersession/History/recovery mechanics were exercised separately.
Owner added ten shared trials; fourteen of sixteen used, leaving at most one per existing Medium condition. The North Quay owner-comment Rewrite is retained Draft/version4 with the owner note unchanged. The PPTX pair retained five drafts and did not invent chart capacity; missing-year clarity remains a Luna limitation. Use the canonical ledger for later totals.
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
> states, conflict review, Applied-private-intake Findings, qualified predecessor/
> owner-comment revision, session Archive/Restore, accurate email caveats and
> retained PPTX slides/notes/tables/image occurrences/original inspection. Include
> backup Settings/status/checkpoint and Action replacement compensation checks
> once the recorded final build contains them.
> Preserve failures, drafts and exact evidence; do not reset data or touch my real
> vault. Report concrete defects and pending owner acceptance; mark unobserved
> or unavailable checks pending. A new live cancellation trial requires an explicit
> selected run and budget rather than being silently included.

Session headless qualification: 1724 default tests (including reused unchanged
Store461),17 existing ignores;423 native workflow/models with15 existing ignores;
547 combined desktop/CLI; doctests, Clippy, shipping,52 fixtures. Combined MIME
helper tests/Clippy,29 affected Inbox workflow tests, shipping and52 fixtures passed.
Independent complete session and merge-delta reviews clean. GUI remains pending.
