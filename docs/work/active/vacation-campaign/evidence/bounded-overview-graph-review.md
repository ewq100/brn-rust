# Bounded overview and dependency projection — independent review

**APPROVED `8a175c4099e2ec89b892fccf169c9b417d0ddbf6`. No actionable source findings.** This approval covers the complete development-tooling candidate and its generated worker derivative. It is not publication/integration approval or a claim about live work.

## Exact scope and independence

Verified clean checkout `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, branch `codex/bounded-completed-overview-20261022`, HEAD `8a175c4099e2ec89b892fccf169c9b417d0ddbf6`, tree `98d1d7f60d87874a7645de129e00c5a821a9515a`, base `5c606c102d5d121105896c48dc7b39e78d26bdb3`. Complete actual diff is exactly the renderer, matching tests and generated `docs/work/overview.md`: 163 insertions / 43 deletions. No selected record, runtime guide, schema or other file changed.

Read the entire renderer including full validation, graph, lookup, check and atomic persistence paths; existing and changed tests; complete generated diff; current selected lead task; relevant workflow/strategic/verification contracts; actual CI caller; author report and saved regression logs. Reused my preceding complete `ddf07b97` review only as historical support after inspecting the complete final source and actual graph delta. I authored no candidate code. No tests, builds, Rust/native/account/network actions or repository writes were performed. Read-only Python was limited to selected-task lookup and in-memory derivative comparison with bytecode writes disabled. This report is my sole write.

## Requirements, responsibility and correctness

The Completed index at renderer lines 183–214 retains all completed tasks for the truthful total and displays only ten. Parsed timezone-aware datetime order is descending by actual instant; an initial ascending unique-ID sort supplies stable ties through Python's stable second sort. Equivalent offsets are handled correctly. The explanatory text calls this last-observed/updated time, explicitly not completion time, and preserves the scope/gates/native/owner-acceptance disclaimer. Sorting creates new lists and does not mutate record order or task objects.

Global selected-record guidance remains independent of Completed being nonempty. It supplies the exact selected `--record ... --task ...` route and discovery of omitted IDs by title or ID. The existing relative record link still uses the actual output location, including custom record/output paths. Lookup validates the entire selected record, then prints the exact selected JSON object and returns before rendering, directory creation or writing. Unknown IDs remain exit 2; `--task` and `--check` remain mutually exclusive.

The new graph code at lines 229–252 starts with every unfinished task and traverses prerequisites to a fixed point, including completed ancestors recursively. Filtering edges by their dependent's membership in this closure is correct: traversal has already included every prerequisite endpoint, and every original edge/condition/satisfaction label belonging to the closure remains. Disconnected completed-only components disappear. No relevant edges means no Dependencies section. The set's traversal order cannot randomize output: nodes and edges are still emitted in the original task/dependency list order, with unchanged IDs and escaping. Work is linear in the visited graph, not repeated whole-history traversal per rendered task.

Full-record validation occurs before projection or lookup. Unknown dependencies, malformed fields, duplicate IDs, evidence and cycles in hidden completed history still fail; projection never becomes a validation filter. Working/Next/Needs-attention selection, reported condition admission, resource waits, current fact/gate blocks and atomic output remain unchanged. `write_atomic` still creates a same-parent temporary file, flushes/fsyncs and replaces only after successful validation/render; malformed input and stale checks preserve prior output. CI continues to call the ordinary `--check` route.

This belongs in the existing deterministic projection owner. It removes obsolete unbounded completed rows and unrelated closed graph output without inventing another tracker, scheduler, task semantics or deletion policy. Detailed evidence/history stay authoritative in the selected record. The small traversal and existing aware parser are proportionate; a general graph/view framework would add no necessary responsibility.

## Regression effectiveness and evidence integrity

The completed-index regression checks explicit expected rows for twelve tasks, timezone-vs-clock order, equivalent-instant ID ties, cap/total/discovery/disclaimer, original record bytes and exact omitted older-task lookup without changing existing output. The added graph regression checks an unfinished → completed → older-completed chain, exclusion of an unrelated closed component, omission when all work is completed and rejection of a cycle wholly within omitted history. Existing tests separately retain unsatisfied-condition/resource distinctions and invalid sibling/lookup/no-write behavior. These are behavior-derived expectations rather than values calculated by the production traversal; no unnecessary broad duplicate suite was introduced.

Independently read and hash-verified the meaningful old-source graph baseline: actual one-test FAIL because `Unrelated completed task` remained in the graph, SHA256 `37604b1ba7baa5d4e826e2a473367f2010a4fe851a66e489f0f2b5d2b3027ec9`. Independently inspected saved final focused output: all 16 named tests PASS, SHA256 `349afc233ff56ce701569c65a56e71e1e8fa0de12934689c384804e58eedcc0e`. I did not rerun either workload.

The author discloses that an intermediate post-implementation test fixture contained an unknown dependency, so validation correctly stopped before its intended hidden-cycle assertion. Correcting that fixture to an isolated completed-only cycle preserves full validation rather than suppressing it. Its raw log was overwritten by the final log and is **not** retained or independently verified. That evidence-retention limitation must remain disclosed; it does not invalidate the separately saved intended baseline or final test output and is not a remaining source blocker.

## Independent derivative and preservation checks

In-memory rendering of the worker's own committed record reproduces the generated overview byte for byte. The record equals base exactly, SHA256 `c54e90923ff70271416636dedf06f235f06e7daca58ef0dd3432ce8869283aa5`, and remained byte-identical after inspection. It contains 31 tasks / 27 completed; final output is 9,064 bytes versus base 10,565 and preceding ddf 9,432. It displays ten completed rows and three relevant edges. Working/Next/Needs-attention content and observation footer are byte-identical to base. The graph diff removes only disconnected completed components while preserving the actual unfinished prerequisites and conditions.

Content SHA256:

- Renderer: `11534fe2574dac7d6cef45166383ead4d3afba39157ede749352d9cf00f22794`.
- Tests: `4573f284dcbbd79e433d030a6faefa976e6e8ca2a2db453386b599bebc9c2911`.
- Generated overview: `e2ddf83a4780033aec08dc168fe4e259e2a28754ae110f8e673e635b5607699f`.

## Remaining gates and limits

Author reports renderer/check, 625-file/2,813-link documentation and diff checks PASS; I verified derivative equality and saved focused results, not a new full check run. The worker record is deliberately older than the lead's authoritative campaign. Three worker edges are not a contradictory measurement of the lead's reported seven-edge closure. Preserve both records' authority and regenerate/reconcile only the selected lead derivative during integration.

Normal publication waits for PR217's normal merge and actual-main reconciliation; exact-head protected checks remain lead-owned. No Rust/native qualification is necessary or claimed for this tooling-only candidate. Final status remained clean at the exact reviewed HEAD.
