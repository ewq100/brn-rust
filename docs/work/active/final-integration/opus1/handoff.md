# Round 1 correction handoff

> Superseded for case C evidence and the rewrite-prompt wording by [round 2](../opus2/response.md).

18 October 2026. Source-only, docs-only correction of the final-integration plan after Opus round 1 (CHANGES_REQUIRED). **Not self-approved.** The parent dispatches a new independent Opus review as needed; there is no automatic loop. Xberg all-document is not done and final integration has not started.

## Binding

- Same checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-ca8b0a32`, branch `work/final-integration-20261017`, started clean at `d4574398516000f63d45e38f168b2a8294ca8951` (tree `d08b12cc77dc440d3a2d012953f42bc7386a9a1f`, parent `73ea9ca777e00d348966e37e8402f3eb68ecb20b`). No new worktree, no delegation.
- The new commit's full SHA and tree are reported in the task result (a file cannot name its own commit); re-derive with `git rev-parse HEAD HEAD^{tree}` and the delta with `git diff d4574398516000f63d45e38f168b2a8294ca8951 HEAD`.
- Changed: `plan.md` (regenerated whole), `inventory.json`, `binding.json`, `merge-simulation.md`, `handoff.md`. New: `NF1-test-spec.md`, `status-draft.md`, `opus1/{opus-review-round1.md, response.md, checks.json, handoff.md}`. Nothing else.

## Corrections

| Item | Result |
|---|---|
| B1 | FI precondition FP-1 to FP-7 (all eight kinds, acceptance rows, PDF STOP disposition, no old backend, extra-family dispositions, reviewed family completion, objective stays open). First HTML component is not "Xberg done". Optional earlier exercise renamed TCI-1 / TCI-2. Xberg resource and family items moved out of "deferred". G9 maps full, partial, crash/cancel and no-hidden-archive journeys through app and desktop paths. |
| B2 | NF-1 order: busy check (no consume) → refresh → take before prepare → attach after prepare; `queued` is the queue-text match, not `queue_claim`. Real call-path cases specified, including `retry_rewrite` with a pending review, second-retry progress, ordinary mismatched/queued, matching positive (feasible with an existing offline fixture), busy. Existing MCP controls remain. No production fix; compile error is a static finding. |
| B3 | Six existing lock moves (`serde_json`, `toml`, `serde_spanned`, `toml_datetime`, `toml_parser`, `toml_writer`) from the actual WIP lock; expected set computed from the exact handoff, not a frozen 52; three-way block facts; conflict procedure; edge-only metadata fallback. `preserve_order` already live in the default workspace graph via rig-core (verified in the registry manifest, the WIP metadata and the base lock); "likely helper-only" withdrawn; host version move and helper feature delta kept distinct. |
| B4 | Principal gate policy recorded; historical failures neither automatic pass nor blanket block; recurrence needs diagnosis and disposition; PR may be ready with disclosures, merge needs required contexts and dispositions; PDF carry needs hosted target-head pass; N1 separate. Not a grant. |
| Advisories 1 to 8 | Applied or recorded; see `response.md`. |
| NF-6 | 82 kept as immutable history; X2 WIP reports 82→0 on its own candidate with the same unchanged checker; conditional re-check on the pin; X2 worker owns those docs. |

## Limits

Static Git, text and hash reads; read-only Python parses of text/JSON (Git lock blobs, the X2 WIP `Cargo.lock`, gzipped metadata JSON); the link checker and whitespace check ([checks.json](checks.json)). No Cargo, build, test, lint, formatter, Store, CLI, auth, provider, network, fetch, GUI, mutation or live command. Nothing imported from the X2 HTML WIP (read only; no pin). `threads-target.md` and `threads-behavior.md` were read by journey table, full-note section and amendments, not in full. The lock identity facts are text parses, not Cargo resolution. NF-1 test cases are a specification, not run.

## Release

No heavy slot acquired or held. All paths allocated to this correction (the `final-integration/` folder and `opus1/`) are released to the principal at commit. No process remains. No PR, push, merge, fetch, release, native observation or paid start. Closed provider windows stay closed.
