# Mac pending-proposal provenance qualification — 26 October 2026

**PASS for the single bounded duplicate-meeting case.** Two fresh live intakes produced exactly one active pending Action. The second Run inspected and refreshed the first proposal, added both supporting Sources to that same Action's Links and exact Current inputs, and retired the predecessor. Completion evidence stayed null. Nothing was accepted or applied; no completion, reminder or external send was claimed.

The latest owner instruction overrides the older continuation's source, eight-Run allowance and remaining cases. Exact tested source: `800d26bbc66b4c9672a3065e18460c2fe4b0b4af`; tree: `79f8f598d0cfe923ad50b5bdbe4fb4bb13df8627`. PR229 was fetched and this commit verified in its ancestry. A new isolated worktree and fresh explicit synthetic root outside Git ancestors were used. Existing work, prior failures and eleven historical Runs were preserved. Only this case was launched; no smoke, other case, retry or fix occurred.

## Actual checks

| Check | Observation |
|---|---|
| First intake | One pending Open Action, owner `Mari`, follow_up `2028-03-03`, `evidence:null`, `due:null`. Proposed text keeps delivery due 10 March distinct from the unspecified approval-request deadline. First Source v1 is linked and declared as an exact Current input. |
| Second intake | Active replacement has one Action whose complete payload is identical to the first. Both saved Source IDs at v1 are linked directly from the Action and appear in this replacement's exact Current inputs. |
| Refresh and retirement | Trace shows successful `read_candidate` then `refresh_candidate`. Original `033d16f8-1cb6-46c3-b0d2-2c8fc1baf856` is `retired`; active `08f0f549-f6cf-4c07-9ef4-5410f369d879` is `prepared`, with `replacement_of` bound to the predecessor. No duplicate active candidate or Action exists. |
| Completion and approval | Both calls to the model's checked `apply_changes` returned `NeedsReview`; no applied receipt or accepted Note/Action exists. The test actor never issued CLI Apply. Answers accurately say pending, not applied or completed. |
| Fresh processes | Separate CLI processes after each intake and a final read reproduced the exact sanitized snapshots, pending payload, source versions and operation states. |
| Strict oracle | Production evaluator, fixture and strengthened source/completion assertions were unchanged. Between intakes, the same source-check function validated only observed source step 0. After intake 2, the full original fixture required source steps `[0,1]` on the active candidate. All checks passed. Actual bytes, prose and trace were reviewed at both dispatch gates. |

First Source: `6a687684-c5a5-4857-b50b-3a1664697c3d:2` v1. Second Source: `055c97ca-0565-4e65-94e7-2bee2475ed96:2` v1. The first request hash is `27910a107035958f5bac821e6ef4a418a1ccee0a286ebae0ec983b4175966d5a`; replacement hash is `702dc3c8b195f90f579359d0447b5a94e7d8dc2273a063b0a74064819dc4d4be`.

Correctness, grounding and the observed review/completion safety boundary passed for this sample. Pending-state persistence and predecessor retirement passed. Apply/Undo, changed-commitment conflict, QA, maintenance and precise-edit cases were not exercised. This does not establish Mac UI, focus, accessibility, picker behavior or owner acceptance, nor general model reliability from one repetition.

## Source, builds and preflight

Repository-pinned Rust 1.98.1, locked offline builds; one shared Cargo writer. The ordinary CLI and seeder were built separately from the exact source, then copied to immutable task artifacts. The build log identifies this new source checkout; the ordinary binary contains `read_candidate` and guidance bundle 23, also confirmed in both live traces. Existing current Stock/BRN helpers were hash-verified and their source graphs were unchanged. No assets were downloaded.

| Artifact | SHA256 |
|---|---|
| Ordinary `brn` | `e857f47a5ed353b1e1cb00a3c9b2e6afe4e4284d66aa97a79c7c99a5c6c711d5` |
| `wiki_eval_seed` | `440cd57c52b39fe1eaaf62c52748b2af7fb68ca70159e446c6eec1835d363e3c` |
| Stock helper | `bf399cc4fe81bb542bbea60dc61799e62819e5f607afca1ab598175629ae57c8` |
| BRN helper | `00733d53ecb419997d291661d818f5b4487f963d9f5ae8e8a80b8b133d3c14cd` |

Commands: `cargo test -p brn-ai --lib threads::history --features capability-spike --locked --offline --jobs 1` passed 10 tests, with an additional child-process witness not added to that count. An initial filter `threads_history` selected zero tests and is retained as non-evidence; the correct filter then selected ten. `python3 -m unittest discover -s scripts/tests -p test_wiki_guidance_eval.py -v` passed 31 tests. Both `cargo build -p brn --bin brn --locked --offline --jobs 1` and `cargo build -p brn-threads-app --example wiki_eval_seed --locked --offline --jobs 1` passed. Parser help passed. The full cloud Rust suite's prior 156 failures remain historical unresolved cloud evidence; this focused Mac run does not turn those into full-suite passes. No full-suite or hosted-CI pass is claimed here.

Normal installed login reported signed in, and supported app-server initialization/model listing offered `gpt-6.1-sol` with medium effort. Allowance was available before and after; the displayed constraining window stayed at 0% used at its exposed precision, not proof of zero cost. Helper models were explicitly off. No UI interaction, unlocking, security/power changes or credential copying occurred; the test did not inspect or change lock state.

## Runs and exposed usage

**Two additional BRN Runs; thirteen cumulative including eleven historical Runs.** Both returned `Completed`. First-live reservation through final semantic review: **133.118 seconds**, below 15 minutes. Each command had a 180-second process bound and the verified source-intake limit of 12 tool rounds. Failed/uncertain launches would count; none occurred. No helper, retry or diagnostic Runs occurred.

| Run | Tool rounds | Model turns | Input tokens | Cached input (included in input) | Output tokens |
|---|---:|---:|---:|---:|---:|
| First intake | 4 | 5 | 59,725 | 46,592 | 809 |
| Second intake | 5 | 6 | 82,091 | 64,384 | 1,446 |
| Total | 9 | 11 | 141,816 | 110,976 | 2,255 |

Actual dollar/credit charge and independently metered HTTP request count are **unavailable**, not zero. Tool rounds, model turns and BRN Runs are distinct quantities. Reported runtime elapsed values were 31.413 and 62.158 seconds; the live interval also includes inspection.

## Evidence and handoff

The [sanitized receipt](evidence/mac-pending-provenance-20261026/receipt.json) preserves exact source/tree/artifact/fixture/driver hashes, bounds, both outcomes/traces, candidate snapshots, reviews and operation readbacks. [Commands](evidence/mac-pending-provenance-20261026/commands.json), the [evidence-local driver](evidence/mac-pending-provenance-20261026/driver.py.txt), both fixture texts and selected build/check logs are adjacent. Machine/auth paths are replaced with placeholders; no credentials, account identifiers or owner documents are included. The original evaluator's `semantic_status:requires_review` is retained; the separate actual-byte/prose reviews record the passing lead disposition, not independent reviewer acceptance.

This session is closed at its two-Run ceiling. Evidence-only local commit; no push, merge or release. No task-owned worker remains running. Any further live case or retry requires new authorization. Earlier failed receipts remain unchanged.
