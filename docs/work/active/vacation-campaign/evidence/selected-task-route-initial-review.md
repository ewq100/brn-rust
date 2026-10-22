# Selected-task reading route complete candidate review

**Reviewed identity:** clean HEAD **`2cdce8325bee82c2888e4f86c44cca88c82564de`**, branch `codex/selected-task-reading-route`, in `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, against **`79fa3980c5022e475b07888d9e08f396f45b0238`**. Complete actual diff: four files, 79 insertions and three deletions: renderer, matching tests, workflow and generated overview. Status was clean at initial and final inspection; no additional staged/working/untracked candidate changes were present.

Attribution: independent read-only review by the separately assigned reviewer agent; this reviewer did not author the candidate. Inspected the full renderer/validation/read/write control flow, complete four-file diff, test setup and new tests, existing custom-record/render/check preservation tests, repository AGENTS and author implementation report. Relevant previously loaded workflow/strategic/verification/independent-review instructions were retained. No builds, tests, renderer/CLI execution, account/native/network actions, source edits or nested helpers were performed. This report is the sole write for this task.

## Verdict

**Changes requested: one bounded P2 generated-guidance issue for supported custom records.** The task lookup implementation and its read-only boundary otherwise satisfy the selected requirements. No BRN product/runtime defect is alleged.

## Finding

**[P2] Preserve the selected custom record in the generated task-lookup route.** Location: `scripts/render-work-overview.py:213`, relevant surrounding `209–213`; corresponding generated Completed guidance.

`render(view, output, record_path)` correctly links the Completed section to its actual selected `record_path`, but the newly generated command always says `python3 scripts/render-work-overview.py --task TASK_ID`. That command reads `DEFAULT_RECORD`. When the existing supported `--record PATH` route renders a different selected record, a reader following its generated instruction looks up the custom record's listed task ID in the old/default campaign. The lookup can fail as unknown or, worse, print a different task with the same ID. This defeats the new reading route for a supported custom source and conflicts with the workflow's explicit instruction to retain `--record` when the selected record is not the default campaign.

Make the generated guidance retain the custom record argument/path, or explicitly instruct custom-record readers to use `--record` with the linked selected record. Keep the default command concise. Add an expectation to the existing custom-record Completed test (or equivalent narrow test) proving that a generated custom-source instruction cannot silently route to the default campaign. No second tracker, new schema, output-writing behavior or broader renderer redesign is needed.

This is source-verified: `DEFAULT_RECORD` is the campaign constant at `16`, argparse uses it at `275`, and the generated command at `213` omits `record_path`, despite the link immediately above correctly using it. No execution was necessary to establish the mismatch.

## Requirements and correctness assessment

The selected need is reducing model/user context consumption while retaining the complete authoritative record. The new flag prints one exact parsed task object rather than rendering the whole record or deleting history; it does not promise that the Python process avoids reading/validating the whole record. No persistent task index, semantic projection or second status source is added.

`--task` and `--check` are members of an argparse mutually exclusive group (`277–279`), so conflicting mode requests reject before record access. The mode then uses unchanged `read_record`, which reads the selected `--record` and invokes existing full validation (`247–254`). Duplicate IDs, malformed tasks, unknown dependencies and cycles cannot be bypassed merely because the requested task itself appears valid. The new branch uses exact ID equality and prints only `json.dumps(selected, ensure_ascii=False, indent=2)` plus newline (`283–288`); it does not add wrapper metadata, unrelated task details or lossy normalization.

On success the function returns before `render`. On unknown ID it raises `InvalidRecord`, prints the normal concise stderr error and returns2 before rendering (`284–292`). Invalid-record paths also leave the output untouched. `--output` is only parsed into a Path before that branch; no existence check, resolution, directory creation or write occurs for task lookup. Even pointing `--output` at an existing non-UTF-8 file, a missing parent or the selected record cannot reach writing in task mode. Only the unchanged ordinary generation path reaches `write_atomic`, whose parent creation and replace occur after successful render; the unchanged `--check` path remains read-only and returns its prior statuses.

The candidate does not change `validate`, `read_record`, task/attempt/gate semantics, dependency graph policy, the authoritative campaign JSON, or any BRN runtime source. The only normal rendered-content change is the intended Completed reading instruction. Workflow guidance correctly explains targeted ID discovery and custom `--record` use. The checked-in generated overview changes exactly the corresponding instruction and does not change observations, task states, history or reported gates.

## Test effectiveness and maintenance

All new tests use the real `main` function, an explicit temporary custom record and output path, and captured stdout/stderr. They cover exact object output and Unicode, source-record byte preservation, binary existing-output preservation, missing output-parent noncreation, unknown-ID exit2/no stdout, whole-record validation through an invalid sibling, and argparse mode exclusion before output creation (`test_work_overview.py:158–222`). These are behavioral boundaries rather than mirrors of the task-selection expression.

The existing custom-record Completed test already provides a natural home for the finding's narrow guidance expectation. Existing generation/check tests retain stale-output detection, preservation under malformed/invalid input, deterministic rendering and mtime preservation. The minimal implementation fits the existing owner and does not warrant a generic query framework or additional layers. Direct `--record` task lookup itself works in source and is exercised by the test helper; the finding concerns the generated instruction for reaching that supported route.

The author reports188 Python tests, actual selected-task/default-check commands, Markdown link check and whitespace checks passing. This reviewer did not execute or independently inspect raw evidence for those reported runs and does not turn source inspection into a passing command claim. The author ran against the available checkout record rather than dumping the oversized live lead record; no live freshness claim follows from that local generated output.

## Next disposition and limits

Record the P2 as fixed or explicitly rejected/deferred with reason, then bind final review and lead qualification to the actual final source. The lead owns final record reconciliation, current overview regeneration, validation and authorized integration. No additional native/account/provider or BRN runtime qualification is relevant to this developer-tooling change. Review approval alone does not establish integrated or current published campaign state.
