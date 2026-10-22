# Selected-task reading route final review

**Approved exact final candidate:** **`cadcec0373e4542ffb1e43a8152bba343e142f8f`**, clean branch `codex/selected-task-reading-route` in `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, against base **`79fa3980c5022e475b07888d9e08f396f45b0238`**. Complete final diff: four files, 82 insertions and three deletions: `scripts/render-work-overview.py`, matching test file, workflow, and generated overview. Verified actual HEAD and clean status while inspecting the full base-to-final diff and exact correction delta from initial **`2cdce8325bee82c2888e4f86c44cca88c82564de`**.

Attribution: independent read-only final review by the separately assigned reviewer agent, who did not author the implementation/correction. Initial complete review remains unchanged at `/tmp/brn-selected-task-route-review.md`. No tests, renderer/CLI execution, builds, source edits, account/native/network action or nested helper were performed. This report is the sole final-review write.

## Disposition

**P2 custom-record generated-route finding: valid/fixed.** The Completed instruction now requires `--record PATH_TO_SELECTED_RECORD --task TASK_ID` and explicitly directs substitution of the linked selected record and a listed task ID. It therefore no longer silently sends a custom record's ID to the default campaign. The placeholder is portable and does not bake a machine-specific absolute path or a new command-quoting framework into the overview.

The existing custom-record Completed-render test now requires the explicit `--record` argument and both substitutions. The checked-in generated overview contains exactly the corresponding renderer text. Workflow keeps the concise default example and its existing custom-record note. The correction changes only that generated instruction, matching test expectations and generated line; it does not change task selection, validation, return codes, record/output access or generation control flow.

**Final candidate `cadcec0373e4542ffb1e43a8152bba343e142f8f` is approved with no outstanding findings.** No additional architecture, production or maintenance issue was found in the complete final diff.

## Complete candidate assessment retained

`--task` remains mutually exclusive with `--check`. It invokes existing `read_record` and full-record validation, then prints exactly the selected task object as Unicode JSON and returns before render/write. Unknown IDs return2 through `InvalidRecord`; invalid siblings still invalidate the record before selection. No output parent is created or existing output read/written even when `--output` is supplied. Default generation and stale-check behavior remain unchanged apart from the intended new reading instruction.

The tests cover exact selected object/Unicode output, source-record bytes, binary existing-output preservation, absent output-parent noncreation, unknown ID/no stdout, whole-record validation, mode exclusion and custom-record generated guidance. They use the actual `main` boundary and disposable records. No task record/history deletion, semantic normalization, second tracker, new dependency or BRN product/runtime code is introduced. The tool reduces task text exposed to the reader/model, while deliberately retaining full source-record validation in Python.

## Evidence attribution and reuse

The author reports188 passing Python tests again on the corrected candidate, successful explicit custom-record task lookup with absent output parent, normal overview check, Markdown links and whitespace checks. Those are author-reported results; the reviewer did not rerun them or inspect separate raw logs here.

The lead separately reports actual-current-record functional proof on initial `2cdce832`:269329 input bytes to3657 exact selected-task output bytes, known/unknown exits0/2, unchanged record/overview hashes and no output-parent creation. The complete correction delta proves the lookup `main` function and validation/read/write paths are unchanged, so this lead-observed functional evidence can be reused **explicitly for those unchanged paths**, without pretending it was a final-commit runtime execution. The corrected generated-route expectation has its own author-reported final suite result and this independent source review.

The lead owns final qualification receipts, current-record reconciliation, overview regeneration and authorized integration. This source approval is not a claim that an old checkout's generated observations are live/current, or that integration/resulting-main gates have completed. No Rust/native/provider/account qualification is relevant to this developer-tooling correction, and none is implied.
