# PR229 completion-evidence continuation — 25 October 2026

**Stopped on the first semantic failure.** Completion evidence remained null in both Runs, but the second meeting copy was not added to the pending proposal's source Links or exact versioned inputs. The duplicate case therefore did not pass the pinned provenance requirement. No subsequent case, Apply, Undo, repair, retry, UI interaction, unlocking, merge or release occurred.

Authority: [exact pinned continuation](https://github.com/ewq100/brn-rust/blob/a829ef2898ce59ce62c6cd8fb9ba37422a452b14/docs/work/active/daily-followups-inbox-restore/mac-completion-evidence-continuation.md). Tested source: `79422f8fc78918b243e64bc3835552c59e05e068`, in a fresh isolated worktree. This is bounded live evidence for [draft PR229](https://github.com/ewq100/brn-rust/pull/229), not a merged or independently reviewed qualification. Original local work was preserved. Earlier nine Runs remain separate and their evidence was not reopened.

## Observed behavior

1. First useful intake produced one pending Open Action: owner `Mari`, follow_up `2028-03-03`, completion `evidence:null`, `due:null`. Proposed prose retained delivery due 10 March separately from the unspecified approval-request deadline. Action and Note linked the first Source v1 and declared its exact input. `apply_changes` correctly returned `NeedsReview`; the answer accurately said pending, not applied or completed. Actual candidate bytes and prose passed inspection before the next dispatch.
2. The second intake recognized the duplicate and created no extra Action or Note. Completion evidence stayed null. However, it only retried `apply_changes` on the unchanged pending operation. The second Source v1 was saved independently, but no candidate source Link or exact input referred to it. The original request hash `bc4eecc5dc2a8b6145509755096bc14ff9785678aa52c5f32bba2705c741020f` remained unchanged. All existing structural assertions passed; actual both-source provenance review failed. The runner's CLI-call boundary blocked further dispatch immediately.

The first source is `66eddc08-ee34-4d7d-be2f-4ddb9d20a659:2` v1; the omitted second source is `3920a05c-eba9-4dba-9ee6-35b0aa1de331:2` v1. The proposal is `a054ae74-66e0-4c8e-bfd9-0ff04fea2244`. Fresh processes reading records, candidates, history and operation inputs reproduced the exact final snapshot. No accepted Note or Action and no applied receipt existed. This is a provenance qualification failure, not a repeat of the prior completion-evidence failure or an Apply failure.

| Dimension | Result |
|---|---|
| Correctness | Pass for this bounded sample's one obligation, owner, follow-up, date interpretation and pending-state account. |
| Grounding | **Fail:** first-source provenance exists; second-copy provenance is absent from candidate Links/inputs. Both original Source records survive. |
| Safety | Pass for observed review and completion boundaries: no accepted Open obligation, false completion, or protected-data mutation. |
| Recovery | Unscored: no applied Note group; Undo/replay not exercised. Fresh-process pending-state persistence passed separately. |

Changed-commitment conflict, M1, DW9, DW10 and precise-edit remain **unrun**. This continuation establishes no native Mac button/focus/accessibility/file-picker behavior or owner acceptance.

## Build, checks and usage

Rust 1.98.1; locked offline ordinary CLI and seeder builds passed. The exact correction regression passed (1 test); guidance/helper evaluator tests passed (28); v2 evaluator tests passed (99). Production evaluator and fixture bytes were unchanged. Existing current Stock/BRN helper sources were unchanged between the original baseline and tested source; immutable helper hashes were verified. The ordinary binary contains the corrected completion-evidence guidance, and live trace reports guidance bundle 22. Source/tree/artifact/fixture hashes and full synthetic outcomes are in the [sanitized receipt](evidence/mac-completion-20261025/receipt.json).

Existing normal login initialized without UI; model listing offered `gpt-6.1-sol` at medium effort. Helpers were explicitly off. Allowance before and after remained 65% available in the observed constraining window; no spending control was reached. No credentials were copied or logged.

**2 additional BRN Runs, cumulative 11 including prior 9.** Both returned provider `Completed`, but the case failed semantic qualification. The wrapper's aggregate `provider_completed:false` reflects its intentionally interrupted case accounting; it does not mean the two durable Runs failed transport. Live interval through failure review was about 151 seconds, below 30 minutes; each process stayed below 180 seconds. Requested limit 12 rounds/Run; actual rounds 3 + 4, model turns 4 + 5. Trace tools: Run1 `read_records`, `prepare_changes`, `apply_changes`; Run2 three `read_records`, then `apply_changes` returning `NeedsReview` again. No extra diagnostic/helper Run occurred.

Exposed usage totals: input 106,179 tokens, including 90,368 cached input; output 1,088 tokens. Actual dollar/credit charge and separately metered HTTP request count are unavailable, not zero. Model turns are not claimed as an independently measured transport-request count.

## Minimal reproducer and handoff

At the exact source, build:

```sh
cargo build -p brn --bin brn --locked --offline --jobs 1
cargo build -p brn-threads-app --example wiki_eval_seed --locked --offline --jobs 1
```

Use the unchanged `meeting-duplicate-notes-one-followup` fixture in `scripts/fixtures/wiki-guidance-cases.json`, existing `scripts/evaluate-wiki-guidance.py` `run_arm`, one arm, zero filler, fresh explicit synthetic root, existing supported login, selected Sol medium, helper off, 180-second bound and `--trace full`. Reuse the evidence-local driver described by the pinned continuation; after each live CLI call inspect `public_snapshot` using the original structural oracle and pause for actual saved/candidate/prose review. On the second call, compare Source IDs/versions against candidate `request.inputs` and `Link` writes. Do not apply a pending group to conceal the missing lineage. A new live reproduction needs separate authorization; this session is closed and must not be resumed.

The next engineering question is how a duplicate source can attach accurate provenance to an already pending proposal without accepting it or manufacturing another obligation. No repair hypothesis is claimed verified here. The latest task authorizes no further fix/retry spending. Evidence is saved locally; independent review, a remote draft evidence PR and any integration are pending. No task-owned test/build/model process remains running.
