**Verdict: APPROVE.** The diff contains no blocking defect in the allocated hunks. Two optional residuals below are worth noting at acceptance; neither blocks.

This review approves only the source at pin `f590632e`. It grants no source, owner, program or main acceptance, and no activation or live authority.

I worked read-only: Git reads, hashing and JSON parsing of the committed evidence, plus the cached `rig-core` 0.44 source. I made no edits, builds, tests, Store, Auth, provider or network calls, and no delegation. I skipped the code-review skill because it starts subagents, as you instructed. Test counts below come from the recorded receipts; I did not rerun anything.

## Identity
- **Pins match.**
  - Branch `work/int-d-20261017`, HEAD `8bd4875f`, worktree clean.
  - Pin `f590632e`, tree `4e8a74a3…`, crates `876bc9dc…`.
- **Diff hashes match.** 88..pin is `dd8b0557…` and 3e..pin is `3447c3d5…`.
- **The crates diff is the same from both bases.** 3e..88 changes no crates files.
- **HEAD after the pin is docs-only.** It adds `handoff.md` and `source-pin.json`.
- **Exactly six Rust paths change.**
  - In `threads_guides.rs`, only the version constant and the two label assertions change.
  - The summary test is renamed guide13→guide14 and its stamps move 13→14/12→13, with no logic change.
  - `agent_runs.rs:571` changes only the 13→14 literal.
- **Docs outside `refresh-contract/`:** only the one-paragraph checkpoint in the INT-D `plan.md`. Canonical behavior and status docs are untouched.
- **All 11 receipts check out.** Each raw-log hash matches its receipt, and the exit codes and counts match the principal audit.

## Production change
- **`refresh_schema` (`threads_history.rs:112-120`).** It removes `writing` from a freshly built `schema("prepare_changes")`, then adds `operation` and overwrites `required`. The `request_key` and nested `request` shapes, `additionalProperties:false` and the description text are inherited unchanged. The common schema, the ordinary null projection and v2 are untouched.
- **Unchanged handling.** `RefreshArgs` still has `deny_unknown_fields`. `validate_refresh` and the `refresh` handler are byte-identical. The handler still refuses a writing snapshot, checks the run family, version and authority, and replays.
- **The whole-batch check covers refresh.** At `threads.rs:2420-2455` it checks every call through `parse_valid` before anything is dispatched, regardless of call order, so incoming `writing` (null or object) still invalidates the whole batch.
- **Runtime guidance needs no change.** `agent/BRN.md` and the skill playbooks never tell the model to send `writing` to refresh.
- **Guide bump.** 13→14 is the right compatibility signal: guide identity hashes the bundle version but not the tool schemas, so the bump is what changes guide and cache identity.

## Tests are real, not vacuous
- **The before red is genuine.**
  - The test-only patch (SHA `c260709e…`) changes only one test in `threads_history.rs`.
  - In a separate source and target it compiled and failed 0/1. The left side was `{operation, request, request_key, writing}` from the actual first `CompletionRequest`.
- **Fresh-family test proves a real cross-run refresh.**
  - Note v2 is protected; the prior run prepares against v2; the owner then moves the note to v3; a new run refreshes.
  - That exercises the run-family membership check across runs.
  - The test pre-allocates the replacement ID. That is sound because `allocate_operation` is idempotent per key.
  - `stored.len()==2` proves both calls succeeded, and `stored[0]==stored[1]` proves the immutable retry.
  - The old candidate ends Retired and the new one Prepared.
  - The NeedsReview evidence is the actual host label `< apply_changes · NeedsReview`, counted once. There is no Applied receipt and the full Note record is unchanged.
- **Forged-writing test covers four arms:** `writing` null or object, each with progress first or last. Each ends InvalidToolUse after one request, with no `> ` dispatch, unchanged progress, operations and candidate count, and the prior Prepared candidate unchanged.
- **Wire test covers all three modes × Codex/strict Copilot.**
  - It checks the exact property and required sets, and that `strict` equals the Copilot flag.
  - The `request`/`request_key` subtrees equal those in `prepare_changes`.
  - The required-set check is meaningful because the strict sanitizer sets required to all property keys (`internal/schema.rs:45-48`).
  - Existing prepare, model, effort and no-output-cap assertions are kept verbatim.
- **Budget test.** The existing loop already asserts that every request's tools equal `budget_tool_definitions`. The new code adds the exact refresh shape and serialized tools/history prefix stability across the lazy skill load.
- **Counts add up.** Full AI went from 494 to 496/0/2, exactly the two new tests. History 6/0 lists both new tests. The Rewrite-refusal test passed in writing 19/0.

## Blocking findings
None.

## Optional residuals
1. **MEDIUM, existed before this change and shared with prepare; needs its own allocation, and is not confirmed against a real provider.** In the common schema (`threads.rs:565`), `request.writes[].data` is `{"type":"object"}` with no properties.
   - The OpenAI sanitizer adds `properties:{}`, `additionalProperties:false` and `required:[]` (`openai/mod.rs:63`, `internal/schema.rs:35-48`).
   - So under strict Copilot Responses enforcement, a compliant prepare or refresh can only send `data:{}`, which cannot parse as a typed record.
   - This fix removes the `writing` confound but does not make strict-route refresh (or prepare) usable for real writes.
   - The normal Codex route is unaffected (non-strict).
   - The worker docs do not overclaim this, but the acceptance wording should not say strict refresh is "usable".
2. **LOW, docs.** `threads-behavior.md:158` still names bundle 13 and does not describe the refresh contract, and `status.md` has no checkpoint for this change. Both are yours to update at acceptance.
3. **INFO.**
   - The new serialized-tools equality in the budget test repeats the existing `request.tools == requests[0].tools` check.
   - The forged-writing test has no in-test positive control without `writing`; the fresh-family `validate_refresh` assertion covers the same args shape.

## Retained limits
- Wire evidence is mock serialization only, with no remote provider acceptance.
- Before the fix, the strict wire was not executed.
- The candidate-history 6/0 run had transient CLI/support formatting hashes; later full AI and final checks match the pin.
- The original three errors remain UNKNOWN, the old windows are closed, and old preflights are stale.
- Phase B remains a proposal only.
- SDK 474/4/1, widget 314/1/1, PDF, platform/hosted/native-human, compatibility, INT-W, main/owner/program and live I01 all remain open.
