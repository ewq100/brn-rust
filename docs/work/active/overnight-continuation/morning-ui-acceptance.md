# Consolidated morning UI acceptance — pending

This is the single authoritative interactive checklist for overnight changes.
All behavior below is **expected**, not observed overnight. Run only after the
owner unlocks the Mac and is available. Headless checks never establish GUI acceptance.

## Build and synthetic data

Current baseline: `32c2538688723acff6f86e5af6594958f8fce26e` (PR89).
Final overnight commit/build manifest: pending final qualification.
Checkout `/Users/evokessler/repos/brn-p2-email-docx-intake`.
Shipping CLI `target/intake-ui/debug/brn`, SHA-256
`6f746bef7c9c1ea4e385de14e28574becb4d001adaa5eeb60275276ed3d8824e`.
Shipping desktop `target/intake-ui/debug/brn-desktop`; verify final manifest
before use. Synthetic retained evidence/proposals:
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
| 3. Review exact before/after and supporting evidence; submit an Action-first consequence group through native approval | Repaired native submission applies only displayed selected versions with existing Source prerequisite, no duplicate Source | Group order, versions, confirmation and terminal receipt/error; retain failed drafts |
| 4. Quit/reopen and inspect saved notes, Action and dashboard | Approved results persist with citations, dates/owner and open status; no invented completion | Saved note, provenance, Action UUID/state/dashboard; discrepancies |
| 5. Reopen completed group and repeat inspection/replay where available | No duplicate effects or new inference; Source/asset unchanged | Effect counts, replay receipt, hashes from headless comparison |
| 6. In a synthetic long-note Ask, request tail evidence, then cancel a second pending request; inspect any error | Range tool reaches exact tail beyond the old prefix, reports scoped full-byte proof; a changed proof refuses. Stop returns a terminal state and saved work remains available. Comment/revision and configurable budgets remain pending until recorded as implemented | Capture shown range/proof/version/progress/error; mark absent slices not implemented |
| 7. Review conflicting synthetic sources and proposed resolution/history/Action if P4 qualifies | Both evidence/authority/uncertainty visible; only approved consequences apply and persist after restart | Sources, reasons, exact changes/history and Action; unimplemented scenario remains pending |

## Already checked headlessly / still pending

PR89 baseline has earlier shipping/build/CI and restart approval evidence.
Fresh overnight ordinary BRN authentication/model discovery passed. Paired live
CLI baseline and additional slice checks are in progress. No overnight native
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
