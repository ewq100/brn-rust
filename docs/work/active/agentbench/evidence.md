# AgentBench offline proof evidence

Observed October 28, 2026 in the managed Linux cloud environment. Base main:
`c3d64efa45a3671b6f91e94f1425fcda9fc87690`; isolated worktree
`/workspace/brn-agentbench`, branch `benchmark/agentbench-proof`.
No repository `.agents/skills` was present. Read root AGENTS, development
workflow/strategic programming/verification, current work overview and relevant
native evaluator/seeder/application/CLI source. No production Rust or runtime
prompt changed. Existing branches and PR239–241 were untouched.

## Reuse and responsibility

The existing Python `Sample` and matrix scheduler still own native execution and
one replacement attempt. Native backup/restore and checked owner operations own
state. The new Python adapter only verifies/imports frozen evidence and computes
reports; it does not choose models, alter credentials or invoke a provider.
Alternative page structures and direct solutions remain valid semantic outcomes.
No new production dependency, provider gateway, storage layer, or parallel agent
framework was introduced. Legacy fixtures and result readers remain available.

The eight categories and weights follow the recovered proposal. Criterion
scores are explicit reviewed inputs, not automatic keyword-based semantic
judgments. Source-level checks remain structural diagnostics. Overall scores
stay unranked while coverage, grader qualification or runtime identity is missing.
Unknown usage/cost is never silently estimated in the new reports.

## Checks actually run

- Initial `cargo build --locked --offline -p brn -p brn-threads-app --example
  wiki_eval_seed --bin brn --jobs 2` failed because cached registry metadata only
  had Rig 0.44.0 while current main pins 0.45.0. The same locked build with network
  dependency retrieval succeeded in 65 seconds. No dependency or lockfile changed.
  Rust 1.98.1 and the configured document helper paths were used. No other Cargo
  process was running before the shared target build; all affected local BRN
  crates rebuilt from this worktree.
- `python3 -m unittest discover -s scripts/tests -p test_evaluate_wiki_v2.py -q`:
  101 passed. Initial stricter intake-budget validation failed two legacy DW12
  fixture cases; restricting that new admission rule to checkpoint-enabled cases
  retained compatibility. The legacy declared/sent distinction is now explicit.
- `BRN_AGENTBENCH_TEST_BINARY=/workspace/brn-environment/target/debug/brn python3
  -m unittest discover -s benchmarks/brn_agent/tests -v`: 15 passed, zero skipped.
  Initial native-test setup omitted `arm`, then `trace`, from its sample fixture;
  both setup errors were corrected before the actual native witness ran.
- Native evaluator without `--live`, generated proof fixture and freshly built
  `brn`/`wiki_eval_seed`: all nine cases seeded, warmed, copied, opened and passed
  Knowledge checks. No candidate ask/intake model steps ran. Local receipt:
  `/workspace/brn-agentbench-native-proof/offline.json`.
- Prepare → collect → report CLI smoke succeeded with empty grades. The static
  HTML/CSV/JSON output correctly shows incomplete/unranked, not a model result:
  `/workspace/brn-agentbench-ungraded-report/`.
- Markdown links and `git diff --check` passed before final review.

The native witness creates a fresh synthetic Note, captures a real backup,
restores into a separate probe, runs checked Undo, verifies the probe's restored
state, compares the original records byte-for-byte at the JSON level and checks
the original backup hash. Reusing a checkpoint path is refused.

## Independent review

A separate read-only reviewer inspected the change. Findings addressed:

1. Partial draft proof could rank as complete: eligibility gaps now keep it
   unranked, independently of a numerical quality score.
2. Manifest labels could contradict native bytes: import derives metadata from
   native results and report verifies the entire derived entry.
3. Checkpoint/citation integrity: hash-check backups and records, then resolve
   structured record/version or result-step references.
4. Interrupted folders omitted: preserve and report legacy orphan stubs; the new
   adapter explicitly inventories unindexed attempts and supports their replacement.
5. Wrong fixture could be scored under reused IDs: generated suite binds fixture
   bytes; collection checks the native manifest and freezes its hash.
6. Missing helper traces became zero: enabled-but-unobserved helper usage is null.

Final bounded re-review found no remaining blocking issue in the implemented
surface. The reviewer ran the Python contract tests with its native witness
skipped; the explicit native execution above supplies that separate evidence.
The normal tooling test lane now discovers these tests through
`scripts/tests/test_agentbench.py`.

## Blocker and remaining implementation

The parent supplied the recovered original plan text. Library search resolved
`BRN-AgentBench-Consolidated-Plan.md` and `BRN-AgentBench-Planning-Pack.zip`.
Current bundled Library download helpers were materialized into a fresh helper
directory and invoked with the complete structured search result and selection
000001. The helper failed before transfer with:
`hosted apps tools/list request failed with HTTP status 401`.
No bytes downloaded, no retry, no alternate URL or credential workaround. This
was an authentication failure, not an automatic approval-review rejection.
Original twelve helper cases and twelve calibration pairs are pending; they were
not reauthored or represented as integrated.

The benchmark README lists remaining implementation independently of this
transfer blocker: wire-observed identity, controlled stale/edit/review/interruption
host events, independent semantic/reader execution and qualification, spend caps,
Inspect parity, external dataset license/version admission and adapters. No
claim that phases 1–3 or the full recovered specification are complete. No live
runs, cost incurred for model calls, private user documents, Mac sessions,
external dataset redistribution, PR approval, merge or release occurred.
