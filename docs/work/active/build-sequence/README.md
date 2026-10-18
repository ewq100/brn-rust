**Current snapshot, 18 October 2026:** the locally accepted foundation/writing and agent-capability source lines are combined with the accepted standard-default Xberg component and inspected main `57c710cc` on `work/final-combined-20261018`. Combined verification and new independent whole-candidate review are in progress; no program PR/main integration, native-human or owner acceptance is claimed. D-S/D-M were approved and D3/D4/INT-D are implemented. The [approved execution amendment](../modular-foundation/x2/standard-default/final-integration-amendment.md) governs this assignment. Earlier proposal/sequence statements below are historical and assign no new work.

# BRN forward build sequence

**Status: proposed sequencing, prepared 16 October 2026 from main `cafb24c2`.** This file orders work that is already proposed. It assigns nothing and grants no implementation, merge or release authority. [Status](../../../status.md) remains the source of truth for current work.

**Checkpoint, 17 October 2026:** the Stage 0 merge prerequisite is satisfied at main `8f768943`: UI/reliability landed together in [#155](https://github.com/ewq100/brn-rust/pull/155), CLI rounds 1–2 in [#156](https://github.com/ewq100/brn-rust/pull/156), and CLI round 3 in [#157](https://github.com/ewq100/brn-rust/pull/157). The latest inspected main native-widget lane failed; see the [current status](../../../status.md#current-repository-snapshot--17-october-2026). C0 and later slices remain proposed until assigned. The old branch-ahead counts below are the pre-merge snapshot, not outstanding work. Standing design/health guidance is now in the [strategic programming guide](../../../development/strategic-programming.md).

**Owner decisions recorded here:**

- 15 October: `work/ui-polish` and `work/daily-workflow-reliability` land before the big work.
- 16 October: `work/cli-agent-contract` lands first too.

## Programs

| Program | Folder | State |
|---|---|---|
| Modular foundation and reference-assisted writing (workstreams A intake, B writing, C foundation/integration) | [modular-foundation](../modular-foundation/README.md) | Proposed, not started; reviewed pack stored in #147 |
| Agent capabilities, Rig-first (workstream D) | [agent-capabilities](../agent-capabilities/README.md) | Proposed, not started; reviewed and amended 16 October |

## Done so far

| Item | Result | Record |
|---|---|---|
| X1 Xberg PDF | Gates 1–2 passed; gate 3 design written. Bounded adaptation, not adopted | [x1-xberg.md](../modular-foundation/experiments/x1-xberg.md) (#150) |
| Z1 Zorite editor | Gate 1 passed, gate 2 failed. Not adopted; current editor retained | [z1-zorite.md](../modular-foundation/experiments/z1-zorite.md) |
| Passage-comment positioning (Z1 follow-up) | Reviewed prototype on the current Read viewer; needs a small `gpui-base` patch | Draft [#149](https://github.com/ewq100/brn-rust/pull/149); owner: do not merge |
| Rig-first plan review | Ready for Slice 0 after amendments A1–A9; Slices 3–4 need owner decisions | [REVIEW.md](../agent-capabilities/proposal/REVIEW.md) |

## Stage 0 — land the three finish-first branches

| Branch | Scope | Ahead of main (16 Oct) | Shares files with |
|---|---|---|---|
| `work/ui-polish` | `brn-desktop` only | 16 commits | reliability: `threads_ui.rs` |
| `work/daily-workflow-reliability` | runtime, core, app, retrieval, CLI, one desktop file | 29 commits | ui-polish: `threads_ui.rs`; cli-agent-contract: `brn/src/main.rs`, `brn-threads-app/src/lib.rs` |
| `work/cli-agent-contract` | CLI `--json` contract, keyed service functions, journey scripts | 9 commits | reliability: as above |

Each branch's own agent lands it with the [landing message](#land-a-finish-first-branch). Branches land in whichever order they finish, and the later branch resolves only the overlapping hunks. If two are ready together, land `ui-polish` and `cli-agent-contract` first, because they share no files, and then reliability.

**Original Stage 0 gate (merge prerequisite now met):** all three on `main`. Before it was met, only two things could start:

- the optional isolated experiment R0 (Rig reuse proofs; [message](#optional-now-r0-rig-reuse-proofs));
- the housekeeping step H0 below.

### H0 — repository housekeeping, alongside Stage 0

H0 touches no code or build files, so it cannot collide with the landing branches. Agents read `docs/work/active/` as current guidance, so finished plans there actively mislead them. At 16 October there were 16 active folders, 161 remote branches and 27 local worktrees.

1. **Inventory.** List each active folder, remote branch and worktree with its evidence:
   - merged PR or merge commit;
   - last commit date;
   - whether it is ahead of `main`;
   - whether an agent or open PR still uses it.

   Mark each *keep*, *archive*/*delete*, or *unclear*. Unclear items are kept and asked about.
2. **Owner approval.** The owner approves the list before anything is removed.
3. **Docs.** Remove finished folders from `docs/work/active/` and add pinned-commit links to them in [docs/archive/README.md](../../../archive/README.md), following that file's existing convention. Do not move folders into the archive. Leave the three landing branches' folders, the proposed programs and status-linked active work in place.
4. **Branches.** For each approved remote branch, first tag its tip as `archive/<branch>` and push the tag; then delete the branch. Never delete a branch that is ahead of `main` without an explicit per-branch approval.
5. **Worktrees.** List only. The owner removes worktrees, because they may hold uncommitted work.

H0 lands as one docs-only PR with the usual required checks. It does not edit [status](../../../status.md) beyond fixing links to removed folders. [Launch message](#now-h0-repository-housekeeping).

## Stage 1 — combined C0

One integration lead (C) runs the modular-foundation C0 and the agent-capabilities Slice 0 together. The output is one baseline, one path-level ownership map and one integration ref, plus two short plans that link rather than repeat each other. Beyond both packs' C0 tasks, the lead also:

- records the reliability branch's `prepare_run` as the seam CW2 builds on;
- records whether the X1, Z1 and #149 results still hold on the integration ref (a record, not reruns, unless the relevant code moved);
- records the four Slice 0 decisions: model-limit source, summary storage, MCP transport and skill-manifest parser;
- obtains owner decision O1 (below);
- records a test-suite baseline: test counts per crate and feature set, wall time per required lane, the slowest tests, and any flaky or ignored tests;
- writes the **deletion list** for slice CL (below). For every item it states caller/feature and saved-data evidence that the item is unused, retired or duplicated, plus the checks preserving supported behavior after removal.

## Stage 1b — CL, cleanup in the quiet window

Between C0 and CW1 the whole tree has a single editor: the three branches have landed, and lanes B and C have not started. That is the only safe time for broad removal. CL executes C0's deletion list as **removal only**:

- dead code and unused dependencies or features;
- legacy compatibility paths whose saved-data need has been checked;
- the duplicated CLI run-selection code, *only* where CW2 does not already plan to replace it;
- tests that only exercise retired mechanisms, plus duplicated tests that cover no extra behavior;
- obsolete scripts and stale comments.

Rules:

- No behavior change and no refactor beyond what removal needs.
- Keep supported saved-data readers, recovery and backup paths, and safety checks even when they look redundant. Any removal needs explicit caller/feature and compatibility evidence plus checks preserving required behavior. A passing test or absent fixture alone does not prove a path unreachable; a regression test should detect the unsafe change it is intended to prevent.
- All four required lanes stay green. The record states deleted lines and tests, and test-time before and after.
- Time-box CL. Items that turn into refactors move to the slice that owns that code.

## Stage 2 — build lanes

```mermaid
graph LR
  S0[Stage 0: three branches land] --> C0
  C0 --> CL --> CW1 --> CW2 --> D1A --> D1B --> D2 --> D3b --> D4 --> INTD[INT-D]
  CL --> B1a
  B1a --> B1b
  CW2 --> B1b
  B1b --> B3
  CW1 --> B3
  B1a --> B4
  CW2 --> B4
  B1a --> PC
  B3 --> INTW[INT-W writing milestone]
  B4 --> INTW
  D1A --> D3a --> D3b
  CL --> X1int[X1-int]
```

| Slice | Lane | Contents | Depends on |
|---|---|---|---|
| C0 | C | Modular-foundation C0 + D Slice 0, test baseline, deletion list | Stage 0 |
| CL | C | Removal-only cleanup from C0's deletion list (Stage 1b) | C0 |
| CW1 | C | Scoped writing setup and request v2, with any schema increment | CL |
| CW2 | C | Reference binding, ledger/quotations on candidates, owner-Apply visibility gate, shared run start on `prepare_run` | CW1 (lane order) |
| B1a | B | UI-only extraction into `brn-writing-ui` | CL |
| B1b | B | Panel on the shared run start; old writing path in `start_run` removed | B1a, CW2 |
| B3 | B | Brief/references UI, Sources used, reading statuses | B1b, CW1 |
| B4 | B | Development host over the real `Workspace` | B1a, CW2 |
| PC | B | Passage-comment positioning in Read, from the #149 prototype | B1a, decision T |
| INT-W | C | Writing milestone: applicable F-suite, review, native outline, program-status record | B3, B4 (+PC if started) |
| D1A | C | Cache-safe budgeted history; older-discussion access | CW2 (default order; O1) |
| D1B | C | Semantic summary in a derived cache | D1A |
| D2 | C | Continuation delta over the landed recovery work | D1B |
| D3a | helper | Skill packages: parse, validate, install/list/remove, in isolated files | D1A, decision D-S |
| D3b | C | Skill loading, frozen per-run catalogue, settings/composer | D3a |
| D4 | C | One opt-in read-only MCP connection | D2 (after D3b when D3 is started), decision D-M |
| INT-D | C | Agent-capabilities milestone (Slice 5) | The D slices the owner started |
| X1-int | A | Xberg replaces Poppler inside the restricted helper | CL, decisions X and L |

**Why this order:**

- Lane C is serial because every item touches `brn-ai/src/threads.rs`, core schema or the lockfile.
- CW1/CW2 go first because they unblock lane B; D slices unblock nothing else. D1A then budgets the final writing-request shape once.
- The critical path is C0 → CL → CW1 → CW2 → B1b → B3 → INT-W. D1A–D2 overlap with B1b–B4.
- INT-W does not wait for any D slice. INT-D qualifies writing integration against whatever has landed.

**Capacity:** two build slots. The default is lane C plus lane B. D3a and X1-int take a slot only when lane B is idle, for example while B waits for CW2 after B1a. Never run two heavy native builds or benchmarks at once.

## Tests, cleanup and mutation testing in every slice

These execution rules apply to every slice from CL onward, in all lanes. The standing [strategic programming](../../../development/strategic-programming.md) and [verification](../../../development/verification.md) guidance also applies to meaningful work outside this proposed program.

**Replace means remove.** A slice that replaces a mechanism also deletes the old route, its wiring and tests tied only to it. This is the "one production route" rule in both plans; this sequence makes it a slice exit condition. Examples:

- the fixed 40-message history (D1A);
- the writing path in `ThreadsUi::start_run` and its fallback to the first linked chat (B1b);
- Poppler executables after X1-int.

**Review the tests the slice touches.**

- Drop duplicates.
- Rewrite tests that pin implementation details instead of behavior.
- Each new guard gets a failing-before test.
- Record test-time changes against the C0 baseline.

Do not run a separate repository-wide test audit.

**Targeted mutation testing.** Use `cargo-mutants` as a local development tool, as in the [quality-simplify pilot](../quality-simplify/evidence.md): `cargo install cargo-mutants --locked`, with no manifest or lockfile change.

- **Scope.** Each slice reviews its changed high-consequence guards and runs mutations where they provide new evidence: `--in-diff` or file and function filters. Record why a slice has no useful mutation target instead of running a ritual sweep. Follow any more specific requirement in the actual assignment.
- **What counts as a guard:** code where one flipped condition causes data loss, a permission leak or a false success:
  - version and expected-version checks;
  - the scope predicate (`context::visible`) and private/project fencing;
  - Current/Historical input binding and the owner-Apply visibility gate (CW2);
  - replay and `request_key` identity, and cancellation-before-commit fences;
  - run-lease and orphan detection;
  - Undo conflict checks;
  - from D: budget admission and the limit outcome, summary invalidation keys, skill path confinement, and the MCP allowlist and dispatch recheck.
- **Running it.** Use `--locked --offline`, a per-mutant timeout, and disposable output outside the checkout. Time-box to about 30 minutes per slice. Use synthetic fixtures only; no provider calls.
- **Survivors.** Classify every missed mutant: fix a meaningful gap with a behavior test, justify equivalence, or identify the remaining native/visual check. List unviable mutations separately and record incomplete work when the budget expires. Inspect unexpectedly selected struct-field "drag" mutants on their merits; the pilot's styling cases are not a universal exemption for field mutations.
- **CI.** Mutation testing stays out of required CI: it is too slow and noisy for a gate. INT-W and INT-D report the aggregate result.

## Owner decisions

| ID | Decision | Needed before | If undecided |
|---|---|---|---|
| O1 | Lane C order: CW first (recommended) or D1A first | C0 exit | CW first |
| D-S | Installable instruction-only skills; amends the canonical target's fixed-catalogue paragraph and master plan C4 | D3a | D3 not started |
| D-M | One opt-in, read-only outbound MCP connection, as a narrow exception to deferred broad connectors | D4 | D4 not started |
| T | Toolkit-patch route for PC: vendored `[patch]` of `gpui-base` 0.7.3, upstream contribution, or maintained fork | PC | PC deferred; the comment list stays navigation |
| X | Xberg cutover approval | X1-int | Poppler retained |
| L | Live provider authority: D1B semantic qualification, I01, the X1-int Luna comparison | Each qualification | Recorded as blocked, not passed |

## What this updates in the stored packs

The pack files stay unchanged as reviewed provenance. Where they differ, this sequence wins:

| Stored text | Update |
|---|---|
| modular-foundation [README](../modular-foundation/README.md) and `final/00` owner decision (two branches land first) | Three branches land first |
| `final/00` bootstrap message | Replaced by the [combined C0 message](#after-all-three-land-combined-c0) below |
| `final/01` section 3, overlapping-work row | Superseded by the Stage 0 table |
| `final/01` section 11, "Experiments before C0" | X1 and Z1 are done. Only R0 remains as an early isolated experiment |
| `final/01` section 12 slice table | Extended by the D slices, PC and X1-int above |
| `final/01` section 17 experiment results and Editor row | X1 and Z1 recorded as above. Current editor retained; positioning pursued on the current viewer (PC) |
| `final/01` section 10 C4, "no dynamic skill platform" | Scoped by decision D-S |
| `final/01` section 10 C1 (targeted mutation testing) and C5 (remove proven dead code) | Made concrete: test baseline and deletion list in C0, slice CL, and the per-slice rules above |
| Agent-capabilities plan, section 10 (remove replaced routes at Slice 5) | Removal happens in each replacing slice, not only at Slice 5 |

## Launch messages

### Land a finish-first branch

Give this to the agent that owns each branch.

```text
Land <BRANCH> on main. Owner authority for this message: push, open a PR and
merge it after the required checks. No release.

1. In the branch's own worktree, finish and commit outstanding work. Fetch
   origin and merge origin/main into the branch (no rebase, no force-push).
   Resolve only overlapping hunks, keeping both sides' intent. Known overlaps:
   crates/brn-desktop/src/threads_ui.rs (ui-polish x reliability);
   crates/brn/src/main.rs and crates/brn-threads-app/src/lib.rs
   (reliability x cli-agent-contract).
2. Run the branch's recorded verification, plus cargo fmt --check, strict
   clippy and the default workspace tests; for desktop changes also the
   native-ui build and tests. Record exact results in the branch's evidence.
3. Push, open a PR and wait for the four required checks: Core and CLI
   (ubuntu-24.04), Core and CLI (macos-15), Native UI build and state
   (macos-15), Native retrieval and combined build (macos-15). If main moves,
   merge it again and wait again. Merge with a merge commit
   (gh pr merge --merge). Report the PR number and merge SHA.

Do not touch other branches, draft PR #149 or real data.
```

### Optional now: R0 Rig reuse proofs

```text
Isolated experiment R0 (Rig reuse proofs) for BRN. Authority: this message only.

Read section 4 of
docs/work/active/agent-capabilities/proposal/amended/BRN_Rig_First_Implementation_Plan.md
and the current AGENTS.md at origin/main. Record the origin/main SHA you start
from.

- Worktree .worktrees/r0-rig-reuse on new branch work/r0-rig-reuse from that
  SHA. Change only experiments/r0-rig-reuse/: its own Cargo.toml with an empty
  [workspace] table, its own Cargo.lock seeded from the root lock, and
  CARGO_TARGET_DIR=/private/tmp/brn-r0-target. Pin rig =0.44.0 with only the
  features each proof needs (memory, rmcp, test-utils). Rust 1.98.1.
- Credential-free proofs with Rig's scripted/test facilities:
  1. TokenWindowMemory/MemoryPolicy applied explicitly at run start with no
     automatic canonical message writes; a stub Compactor invoked; confirm
     CompactingMemory summary state is process-local.
  2. Prefix stability: inside one run each request prefix is byte-identical
     to the previous request; two runs between compaction boundaries share the
     history prefix.
  3. rig-rmcp: tool conversion, cancellation and list-change refresh against
     an in-process rmcp 2.x server. Record which transport features resolve
     and the lockfile delta, including duplicate reqwest/schemars versions.
  4. AgentRun round trip: list every retained payload field; show that direct
     driving skips configured hooks/tools/memory.
- REPORT.md: pass/fail per proof with exact commands and outputs, dependency
  and lockfile delta, limits. Time-box about 3 hours. Commit locally; ask
  before pushing.

No production or shared files, provider calls, model downloads, real data,
computer use or merge.
```

### Now: H0 repository housekeeping

```text
Repository housekeeping H0 for BRN. Read section "H0" of
docs/work/active/build-sequence/README.md and the current AGENTS.md at
origin/main. Record the origin/main SHA you start from.

Phase 1, inventory only (no deletions):
- Every folder in docs/work/active/: merged PR or merge commit that finished
  it, last change, whether status.md or an open PR links it.
- Every remote branch: merged into main or not, commits ahead of main, last
  commit date, open PR.
- Every local worktree (git worktree list): branch, clean or dirty, ahead of
  main. List only; never remove a worktree.
Mark each item keep, archive/delete or unclear, with the evidence. Always
keep: the three finish-first branches and their folders, build-sequence,
modular-foundation, agent-capabilities, work/x1-xberg, work/z1-zorite,
work/passage-comment-geometry (draft #149) and anything with an open PR.
Write the inventory to the session/scratch area, not the repository, and
STOP. Show me the table and wait for my approval.

Phase 2, only after I approve the list (send "approved: <items>"):
- Docs: on new branch work/h0-housekeeping from latest origin/main, remove
  the approved folders from docs/work/active/ and add pinned-commit links
  in docs/archive/README.md following its existing convention. Fix any
  status.md links to removed folders and change nothing else in status.
  Run scripts/check-markdown-links.py and git diff --check. Push, open a PR,
  wait for the four required checks, merge with a merge commit and report
  the SHA.
- Branches: for each approved branch, first create and push the tag
  archive/<branch> at its tip, then delete the remote branch. Never delete
  a branch that is ahead of main unless I approved that branch by name.
  Report each tag and deletion.

No code, manifest, lockfile or CI changes. No force-push, no history
rewrite, no real data, no release.
```

### After all three land: combined C0

```text
Bootstrap mode, combined C0 only. You are the single integration lead (C) for
two proposed programs. Read at current origin/main:
- docs/work/active/build-sequence/README.md (it wins where the packs differ);
- docs/work/active/modular-foundation/proposal/final/01-master-plan.md and
  02-foundation-lead-prompt.md;
- docs/work/active/agent-capabilities/proposal/amended/
  BRN_Rig_First_Implementation_Plan.md (Slice 0 is section 4).

1. Inspect actual state: branch/HEAD, fetched origin/main, every local
   worktree (including unpushed and uncommitted work), open PRs and running
   processes. Read the current repository AGENTS.md and status. If an
   injected guide conflicts with them, report the conflict and follow current
   repository guidance and my explicit authority here.
2. Owner decisions (15-16 October 2026): work/ui-polish,
   work/daily-workflow-reliability and work/cli-agent-contract are merged
   before this program starts. Confirm all three are on main. If any is not,
   stop and report. Preserve all other unmerged work; do not merge draft
   PR #149.
3. Run relevant baseline checks with exact commands and features, covering
   the four required CI lanes. Record pre-existing failures separately; do
   not hide, skip or weaken anything.
4. Install docs/work/active/modular-foundation/plan.md (baseline, path-level
   ownership for both programs, the section 7.2 contract, section 13 scenario
   IDs and applicability, the F22 default-build rule) and
   docs/work/active/agent-capabilities/plan.md (capability matrix, reuse
   decisions, acceptance IDs, blockers), both linked from status. The second
   links the first's baseline and ownership instead of repeating them.
5. Slice 0 reuse proofs, credential-free, in experiments/ only; reuse R0's
   results if it ran, recording its snapshot and whether it still holds.
   Record the four decisions: model-limit source, summary storage, MCP
   transport, skill-manifest parser. Add no production dependencies.
6. Record the reliability work's prepare_run as the seam CW2 builds on, and
   whether the X1, Z1 and #149 results still hold on your integration ref.
7. Record the test baseline (counts per crate and feature set, wall time per
   required lane, slowest tests, flaky or ignored tests). Write the slice CL
   deletion list: each item with caller/feature and saved-data evidence that
   it is unused, retired or duplicated, plus checks preserving supported
   behavior after removal. Do not delete anything in C0.
8. Do not implement CL, CW or any D slice. Commit a C0 integration ref that
   adds no new failing or skipped required tests, and record it.

Finish with: the factual baseline and pre-existing failures; the test
baseline; the integration ref; the ownership table; the Slice 0 decisions with
evidence; the CL deletion list; open owner decisions (O1, D-S, D-M, T, X, L);
and precise start messages for CL, then CW1 and B1a.

Local work and local commits only, on synthetic data. No push, merge, release,
real-data use, model download, paid or live provider calls, or computer use.
```
