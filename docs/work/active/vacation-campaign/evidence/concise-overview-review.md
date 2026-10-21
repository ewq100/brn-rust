# Independent concise-overview candidate review

Result: **no actionable findings** in complete frozen candidate `e501f9d561db05fb44d06be4c8d9cae2ab269276`. The change belongs to the existing projection, preserves the authoritative history and admission logic, and implements the selected compact completed-task index. Lead checks were pending at review launch; the supplied Python-suite and in-memory projection evidence inspected before completion is recorded below. Actual integrated CLI/documentation gates remain separate.

## Identity and scope

- Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, branch `codex/concise-completed-overview`.
- Verified baseline/merge base: `f54fcc2e4f18f590601c1ddb06c4b98470a0df84`.
- Candidate: `e501f9d561db05fb44d06be4c8d9cae2ab269276`; clean working tree verified.
- Complete actual delta: four files, 63 insertions/624 deletions: renderer, its existing completed-history regression, status wording and generated overview. No task record, receipt, runtime prompt, tracker or product source changed.

Final SHA-256:

| File | SHA-256 |
| --- | --- |
| scripts/render-work-overview.py | `55cdd0e2ee96cd1694dd1238a180c89bec7bd8c3dfe74fd68ce6bfba3a89ccd8` |
| scripts/tests/test_work_overview.py | `4ad735ed0108c5130a5e39d5fc47551601a1b4bdaea42ec1e80de1579a1b53f1` |
| docs/status.md | `9016a5b2ec7af4820cd540cee52ca6b225290020da1b104f391c4c12b9a6f0fb` |
| docs/work/overview.md | `574dc19262e5f24dace5a808ae30ab7877b7f1a4f87a7eb8f160c1f11717a659` |

Read applicable AGENTS, workflow/verification and selected lead `overview-reading-volume` task/design; used `/tmp/brn-overview-reading-volume-preflight.md` as an observation, not authorization. Reviewed the complete changed source/test/status and generated-output transformation, the full renderer and test file, callers/CLI/CI wiring and unchanged record/graph/history relationships. Existing reviewing-product-candidates/resource and Rust-review guidance was already loaded; this candidate is Python/documentation only.

## Requirements and correctness

`render()` still validates the entire view before selecting output sections (`scripts/render-work-overview.py:176-180`). The full task collection remains the input to dependency validation, reported-condition eligibility, unmet prerequisites/resource waits, and graph generation. Completed tasks are not removed from the record or those computations. Actionable/current blocks still use `_task_block` and retain their owners, observations, activity, next action, gates, evidence and attempts. Only the completed presentation branch at lines 208-217 changes.

The completed branch emits all completed tasks, in record order, as stable ID/title rows, without arbitrary limits. It explicitly says completion is of selected scope and does not establish every gate, native observation or owner acceptance. Full details remain in one linked selected record, with instruction to find the listed ID. This neither duplicates history into another generated file nor erases failed attempts or outstanding gates. `docs/status.md:3` accurately describes the compact index and routes details to the record/receipts.

`main()` passes the actual `args.record` to `render()` at line 281. The link uses the resolved selected path relative to the resolved output directory, with URL quoting, so it follows custom `--record`/`--output` locations instead of hardcoding the campaign link. The default third argument preserves existing two-argument Python callers. All direct render callers are maintained tests; the ordinary CLI/CI entry point supplies its selected record. Existing repository-relative evidence linking, atomic replacement, malformed-record handling and `--check` no-write semantics are unchanged. No new tracker or command interface is introduced.

## Generated output and history preservation

A read-only structural comparison of the actual candidate/base files established:

- The entire prefix before `## Completed` is byte-identical, including actionable task details and attention/eligibility presentation.
- The entire dependency section and observation footer are byte-identical.
- The authoritative candidate `docs/work/active/vacation-campaign/campaign.json` is byte-identical to baseline (SHA-256 `c54e90923ff70271416636dedf06f235f06e7daca58ef0dd3432ce8869283aa5`). Thus omitted completed gates, evidence and attempts remain available in the same record and linked receipts.
- All 27 completed IDs/titles exactly match that record in order; their former completed headings match the same title order. No completed task is silently dropped or relabeled.
- This checkout's saved snapshot has 31 tasks, and its overview changes from 775 lines/63,455 characters to 190 lines/18,629 characters. These are file-size observations, not token, performance or current-lead measurements.

The larger preflight observation (66 tasks, 61 complete, 1,590 lines) is from the newer lead record. This candidate's older saved observations were already present in baseline and remain explicitly dated; the correction does not manufacture a fresh status claim. The later supplied `/tmp/brn-concise-overview-projection.json` records that current lead snapshot rendered in memory from 1,590 to 241 lines, with all 61 completed IDs retained and actionable prefix, actual dependency edges and authoritative input bytes preserved. It also retains the initial cross-checkout comparison failure: each imported module initially resolved evidence paths relative to its own checkout; only the in-memory new module ROOT was aligned to the intended lead location for comparison. Production source was unchanged. I inspected this receipt; actual CLI rendering after copying the reviewed source to its intended location remains a separate integration check.

## Regression effectiveness and limits

The existing completed-history regression now drives the actual `main()` path with custom record/output locations and expects `../../campaign.json`. It asserts the compact ID/title row, the scope/acceptance caveat, absence of verbose completed gates/evidence/attempts/details from the projection, and byte-identical retained record contents plus representative historical fields. This would catch failure to forward the selected record or continued verbose completed rendering. Existing tests still cover full current-task detail, reported-condition versus task-state readiness, resource waits outside the graph, unknown/cyclic/malformed input, stale checks and atomic preservation. Their meaning was not weakened to make the new presentation pass.

I ran read-only source/data inspection and `git diff --check` on the complete candidate; the latter passed. I did not run the renderer, Python unit suite, documentation checks or builds. The structural comparison above only read candidate/base text and JSON; it did not update source or records. No network/account/native/provider action or nested work occurred.

Before completion, I inspected `/tmp/brn-concise-overview-tests.log`: it ends with `Ran 183 tests in 11.876s` and `OK`, including the revised compact-index regression. This is the lead's actual suite result, not an independently repeated run. The changed regression uses one completed synthetic task and ordinary custom paths; it is not exhaustive path-encoding/platform or multi-task execution proof. Source inspection and the exact full-record index comparison cover ordering/completeness here. Documentation checks and actual integrated CLI/current-record generation must still be reported from their own commands. This development projection change establishes no native observation, owner acceptance, merge or release, and does not alter the authority of any historical gate.
