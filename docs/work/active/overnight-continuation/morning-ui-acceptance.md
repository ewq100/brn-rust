# Consolidated morning UI acceptance — pending

This is the single authoritative interactive checklist for overnight changes.
All behavior below is **expected**, not observed overnight. Run only after the
owner unlocks the Mac and is available. Headless checks never establish GUI acceptance.

## Build and synthetic data

Integrated range/guidance commit: `4f5b95e1c7b92f9efec54ea7f34f9097987cfdb6` (PR90).
First slice runtime: `ed0cf1df217890f15a972ea07bc9d174b05fd3b6`.
Immutable first-slice CLI: `/private/tmp/brn-overnight-20261008/range-guidance-brn`,
SHA-256 `412c183785d6f5521f572576dfe127bac645bb997eb70f3e73ab24e41a82378d`.
Final integrated commit/build manifest: pending final qualification.
Checkout `/Users/evokessler/repos/brn-p2-email-docx-intake`.
Shipping CLI `target/intake-ui/debug/brn`, SHA-256
`412c183785d6f5521f572576dfe127bac645bb997eb70f3e73ab24e41a82378d`.
Shipping desktop `target/intake-ui/debug/brn-desktop`, SHA-256
`1109da17d02a6ac7e2f213367bfa2e7c6da6e48b5b7bd17ca63161db2d6d28fd`.
First-slice manifest `/private/tmp/brn-overnight-20261008/first-slice-build-manifest.json`.
Budget candidate is being qualified separately in `/Users/evokessler/repos/brn-p3-work-budgets`;
replace launch/build receipt with that final qualified build before its checks. Synthetic retained evidence/proposals:
`/private/tmp/brn-retained-qualification-bi3q58kf/headless/{data,vault,results}`.
New campaign receipts: `/private/tmp/brn-overnight-20261008`.
Fixture `experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml`.
Do not use private vault/email or repeat inference merely to inspect saved work.

## Preparation and launch

After unlock, confirm the final commit/executable receipt below and that no
process owns the synthetic data folder. From the checkout launch:

```sh
target/intake-ui/debug/brn-desktop \
  --data-dir /private/tmp/brn-retained-qualification-bi3q58kf/headless/data \
  --vault /private/tmp/brn-retained-qualification-bi3q58kf/headless/vault
```

Use the retained synthetic work. If the app is busy, record the owner/process
condition; do not delete locks or reset data. Record date, commit, executable
hash, data path and each observed result. For any failure retain error wording,
selected proposal/version and before/after state; capture safe synthetic-only
screenshots when useful. Stop dependent approval after integrity failures.

## Ordered acceptance journey

Allow 25–40 minutes; essential path is steps 1–5 (10–15 minutes).

| Step / user action | Expected result | Evidence and failure record |
| --- | --- | --- |
| 1. Open retained original email and DOCX, inspect inline/shared chart placements and unread spreadsheet | Saved reading opens without re-import/reconversion; original, supported extraction, gaps and three occurrences of one asset remain distinguishable | Reading/attachment selections, image occurrence labels; record missing content or unexpected processing |
| 2. Fully quit and reopen; inspect Applied Source and retained model proposals | Source approval and reading survive; proposed Knowledge/Actions remain separate unapproved work | Before/after Source and proposal states/versions; any lost work or new inference |
| 3. Review retained Sol group `fa13347c-5349-41a9-94eb-28f6c749c9c6` (two Knowledge/three Actions), exact before/after and evidence; submit native approval | Repaired native submission applies only displayed selected versions with existing Source prerequisite, no duplicate Source | Selected group/order/versions, confirmation and terminal receipt/error; retain failed drafts. Backend Action-first is already proven headlessly; record actual native order. Avoid approving every comparison group as distinct work |
| 4. Quit/reopen and inspect saved notes, Action and dashboard | Approved results persist with citations, dates/owner and open status; no invented completion | Saved note, provenance, Action UUID/state/dashboard; discrepancies |
| 5. Reopen completed group and repeat inspection/replay where available | No duplicate effects or new inference; Source/asset unchanged | Effect counts, replay receipt, hashes from headless comparison |
| 6. Inspect the saved long-evidence investigation when available and its range/proof/error state; exercise Stop on a disposable local conversion | Expected range output reaches tail beyond the old prefix and retains scoped full-byte proof; changed proofs refuse. Stop settles local work without loss. Do not initiate extra provider inference merely to inspect saved outputs; any fresh live cancellation trial requires separate morning selection. Comment/revision remain pending until recorded as implemented; budget candidate checks below await final qualification | Capture shown range/proof/version/progress/error; mark absent slices not implemented |
| 7. Review conflicting synthetic sources and proposed resolution/history/Action if P4 qualifies | Both evidence/authority/uncertainty visible; only approved consequences apply and persist after restart | Sources, reasons, exact changes/history and Action; unimplemented scenario remains pending |

## Budget candidate checks — final qualification pending

Allow another10–15minutes; inspect saved results without fresh inference. A fresh
live cancellation/timeout run requires a separate morning selection; overnight
campaign allowance is not silently extended.

| User action | Expected result | Evidence / failure record |
| --- | --- | --- |
| Settings: inspect defaults/presets; choose16rounds/180seconds; inspect Ask/Inbox summaries | Defaults8/300; presets4/8/16/32 and60/180/300/600; Rewrite explicitly separate | Selected values/summary and build receipt |
| During a separately selected synthetic investigation, inspect progress and reopen Settings | Frozen provider/model/effort/budget;0/0 admission, completed model turns/admitted tool rounds; controls disabled until settled | Captured ceilings/progress; any cross-conversation or stale event |
| Exercise Stop; on selected timeout trial inspect draining and final cause | Stop usable during drain; time_limit_reached differs from manual stop/tool_limit_reached; semantic partials/drafts retained without automatic approval | Exact cause, partial/draft IDs and final receipt. Legacy strict visual JSON has no unsuccessful partial output |
| Change future presets, quit/reopen, inspect Ask history/Inbox analysis | Original budget and reason persist; prebudget history says unavailable | Before/after budget/reason and any lost work |

## Already checked headlessly / still pending

PR89 baseline has earlier shipping/build/CI and restart approval evidence.
Fresh overnight ordinary BRN authentication/model discovery passed. Paired live
CLI baseline and first-slice comparison completed (four investigations total).
Fresh-process Action-first approval/replay preserved Source/assets with no duplicate
effects or inference. Default workspace/doctests and52fixture assertions passed;
range transport/filesystem/drain/application tests and143AI tests passed (one
existing ignore). Native first-slice local checks passed402workflow/models and326desktop/widget/CLI tests, combined/default Clippy/builds and52fixture assertions. Required PR90 CI passed and PR90 merged; all required postmerge checks passed. No overnight native
interactive result or personal product acceptance is established.
Final check counts/commit/PR receipts will be recorded here before cutoff.

## Ready-to-paste morning agent prompt

> Read `docs/work/active/overnight-continuation/morning-ui-acceptance.md` and the
> overnight plan in `/Users/evokessler/repos/brn-p2-email-docx-intake`. The Mac is
> unlocked and I am available. Verify the recorded final commit/build and use
> only the preserved synthetic data. Execute this single ordered UI acceptance
> journey, recording observed results separately from expected behavior and
> headless evidence. Inspect retained model outputs without repeat inference.
> Exercise native approval, full restart/reopening, saved notes, Action/dashboard,
> and the newly implemented range/revision/cancellation/conflict checks. Preserve
> failed drafts and exact evidence; do not reset data or touch my real vault.
> Report concrete defects and pending owner acceptance; do not claim unobserved
> steps passed. Use the essential path first if time is limited.
