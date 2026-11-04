# Bounded AgentBench implementation plan

Goal: trustworthy frozen-output grading and a separate six-case wiki challenge,
with a concrete capped experiment prepared offline. This plan implements the
owner's 4 November 2026 request; previous pilot authorizations are consumed.
Lead checkout: `codex/agentbench-bounded`, baseline `098dc5563` (origin/main).
Original checkout `9b47b285` and its `.vscode/` and `experiments/` are untouched.
Open PR239 concerns production search; this task owns only AgentBench, its runner
reporting and task documentation. No competing AgentBench writer was found in
worktrees/open PRs. Production prompts/operations remain unchanged.

Architecture: extend the existing native evaluator, evidence adapter, scoring,
anonymous semantic packets, calibration and comparison tools. No new scheduler.
The baseline suite remains byte-stable. Truth and source support remain grader
inputs; the candidate receives only event-time sources and owner instructions.

- [x] Fix empty-check verdicts and expose versioned execution/check/quality/safety/
  usage sections, with explicit absent and unverified states. Keep old evidence
  and report directories unchanged. Test empty, failed, unknown and excluded
  checks, failed/replaced attempts, missing usage and mixed scoring revisions.
- [x] Add `wiki-consolidation-v1`: six distinct histories, six separate criteria,
  checkpoint truth, plausible distractors, alternatives and no-change outcomes.
  Reuse prepare/collect/packet/report/compare and checked seeding. Challenge scores
  never enter baseline overall. Test event-time truth, duplication, old-message
  ordering, project separation and unavailable reader evidence.
- [x] Freeze a rubric/reviewer configuration and independently label defect
  fixtures before comparing candidates. Reuse calibration swapped packets; review
  all 16 hash-pinned pilot003 anonymous packets, retaining evidence references.
  Treat candidate text as untrusted. Distinguish reviewer judgment from protocol
  tests and report missing native identity/checkpoint bindings exactly.
- [x] Prepare a fixed real-retrieval reader protocol and immutable experiment
  input lock (source/build, harness, data, prompts, settings, retrieval). Scripted
  controlled-reader replies remain protocol evidence with no measured quality.
  Test changed locks and empty/controlled evidence cannot qualify comparisons.
- [x] Reconcile readiness/status with pilot003; record genuine isolation unavailable
  and two separately owned unseen histories as a pending gate. Prepare round one
  allocation and exact commands, at most three total rounds, with hard call/time
  limits, variation/repeats/order, regression/safety/fresh-evaluation gates.
- [x] Run focused/full Python checks and applicable native offline checks; obtain
  independent final council review under available model policy, fix substantiated
  findings, commit a reviewable branch. No live experiment, merge or release.

Review focus: zero-check vacuous passes; lost facts/qualifiers; source instructions
that impersonate evaluator authority; malformed/missing measurements; scripted
answers misrepresented as quality; evaluator changes or readable holdouts during
optimization. Unknowns remain unknown. Useful page structure is judged by meaning,
not title or number of notes. App binary hashes do not attest their build source.

Independent final review: three Sol passes (7.5, 8.7, 9.0/10); all three findings corrected. Other council seats deferred under the owner's concrete-disagreement constraint; no complete three-family council claimed. New live allocation and genuinely unseen ownership remain explicit external gates, not completed experiments.
