# Follow-ups, manual inbox and desktop Restore batch

Selected 24 October 2026 in the saved Linux cloud environment. Baseline is
`f60c5a2f2102f4925b1b182f9f1bab2739fde76d`, the merge of
[PR226](https://github.com/ewq100/brn-rust/pull/226) from reviewed `54b0c3d6` with
unchanged production tree. The integration checkout was observed clean at that
commit before this documentation slice. Earlier daily-use records remain dated
evidence; their pre-merge labels do not describe current main.

## Scope and authority

The owner approved all three proposed slices with “Ok do all 3”
(`Sentinel_4a4c26b62cf48191a984cea7ea942e63`). The immediately preceding proposal
bounded them as follows:

1. **Meeting notes → dependable follow-ups:** add clear Action owners, waiting-on
   and follow-up dates to the existing system. Exercise ambiguous commitments and
   duplicate notes so BRN does not invent tasks. Reuse current agent/checked services.
   Dependencies, decomposition, recurrence and external messages are not assigned.
2. **Manual document inbox:** one folder with an explicit Scan inbox button,
   import status and safe retries. Reuse the improved importer; leave original
   files untouched. No always-running watcher, background daemon or automatic scan.
3. **Desktop Restore:** a clear, safe app flow over the existing backup restore
   engine. Reuse its staging, validation, collision and canonical-store protections;
   do not implement another engine or authorize destructive replacement implicitly.

The earlier “Merge skip the calculator feature for now what else could we do”
(`Sentinel_c40a2bce390c819198a4113ceb9717ae`, 2026-10-24 03:21 UTC) authorized the
prior merge and explicitly deferred the calculator. It does not authorize new
PR merges. Calculator/chart/report activation, release, deployment, new paid
services, external sending, broad connectors and Mac access remain outside scope.
No live-admission retry or credential/security change is assigned.

The [existing canonical decision backlog](../daily-use-reliability/decision-backlog.md)
remains the only product-disposition list. D034/D039 are Keep only for the narrow
follow-up scope; D032 is Keep only for manual Scan inbox; D013 is Keep for the
safe desktop Restore flow; D015 is Later. These choices do not approve the broader
historical descriptions, and other owner-choice cells remain unchanged.

## Ownership, sequencing and checks

One lead integrates three implementation worktrees and an independent reviewer.
Actions owns schema11 first. Inbox owns schema12 stacked on the reviewed schema11
contract; it must not race an independent migration number. Restore reuses the
existing staging engine. The lead serializes heavyweight shared-target builds,
records exact source/features/results and preserves unrelated work.

Use fresh explicit synthetic directories. Check ambiguous/duplicate extraction,
exact dates/owners/waiting-on, current-version and protection guards, Apply/Undo,
reopen/recovery, scan retries and unchanged originals, and restore refusal/staging
failure behavior. Compilation and deterministic fixtures are not live semantic or
native acceptance evidence. Existing sandbox Git-ancestor credential fixtures are
known blocked locally: fail fast and use ordinary CI; do not weaken the fixture,
move/copy credentials or alter security settings to manufacture a local pass.

Deliver coherent tested commits and reviewed **draft** PRs with exact-head CI.
No new merge, release or deployment is authorized by this batch. Tests follow
[verification guidance](../../../development/verification.md); current execution
and build waits belong in [record.json](record.json), not another tracker.

## Prior resulting-main gate

The parent reports [run40058075212](https://github.com/ewq100/brn-rust/actions/runs/40058075212)
on resulting main `f60c5a2f`: documentation and Linux native UI/retrieval passed;
three optional Windows checks failed; four core/Mac checks were still active.
This is a reported checkpoint, not a fresh CI lookup by the documentation lane.
The lead owns the explicit prior-main-ci task to inspect final results, preserve
optional failures and diagnose any required failure. Prior head qualification and
resulting-main qualification remain separate.

## Later consolidated native acceptance

In one separately authorized actual Mac session, verify Action owner/waiting-on/
follow-up controls and keyboard/focus/accessibility; select the inbox folder using
the real picker, scan, inspect statuses and retry safely; then select a synthetic
backup, inspect Restore destination/confirmation, exercise refusal cases and reopen
the restored copy. Check that original inbox files and the current workspace remain
safe. No Mac access is authorized for this cloud batch, and owner acceptance remains
separate from CLI, native-state tests and platform builds.
