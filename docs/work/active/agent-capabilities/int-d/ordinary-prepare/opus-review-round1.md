**Verdict: APPROVE_WITH_QUALIFICATIONS.** I found no blocking safety or spec bug in the production change. One new test assertion is vacuous, so one acceptance row overclaims (Q1 below).

This approves only the reviewed source at pin `6b18324d`. It does not grant acceptance, integration or any activation.

I worked read-only: Git reads, hashing, JSON parsing and the cached `rig-core` source. I made no edits, builds, tests, Auth, provider, network or Store calls, and no delegation. I skipped the code-review skill because it starts parallel subagents. Test counts below are the recorded receipts; I did not rerun anything.

## Identity
- **Pins match.**
  - Branch `work/int-d-20261017`, HEAD `ac565a34`, worktree clean.
  - Pin `6b18324d`, tree `eefc2bcd…`, crates `bf7a8db2…`.
- **Diff hashes match.** 5352..pin is `077b8f6b…` and a65..pin is `6ee7198a…`.
- **a65..5352 is docs-only.** It changes 34 files and none under `crates/`.
- **Exactly 9 Rust paths change:**
  - eight `brn-ai` files;
  - `agent_runs.rs`, with only the 12→13 literal changed.
- **HEAD after the pin changes only docs.**
- **All 20 check-index receipt hashes match their files.** Full AI, the final strict lint and shared_run differ from the pin only in `agent_runs.rs`.

## Production change (checked against the diff, not the report)
- **Capability test** (`threads.rs:678-681`): the tool allows evidence when the run's writing snapshot has a saved setup. That is exactly the opposite of the handler's refusal at `:1783-1790`, so it uses saved setup, not Rewrite mode.
- **Hand-written tool matches the old macro.** Name, Args, Output, Error, description, helper denial and handler delegation are the same.
  - Ordinary and v1 replace the whole `writing` node with `{"type":"null"}`.
  - v2 keeps the common schema unchanged.
- **One catalogue still drives everything.** The contextual-constructor arm applies only to PrepareChanges. Order, activity labels and `tool_names()` are unchanged. Registration and budget definitions instantiate the same value, and `budget_tool_definitions` derives the same value from the run's `Arc<Scope>`, whose snapshot field is plain and never changes during a run. So the three uses cannot drift.
- **Whole-batch check** (`:2439-2442`): when no saved setup exists, a non-null `writing` makes the whole turn invalid before any dispatch. Missing or null `writing` still passes. v2 behaviour is unchanged.
- **Helpers.** For helper runs the hook never reads this field; the unchanged `allowed=HELPER_TOOLS` list (which has no `prepare_changes`) still rejects prepare.
- **These are byte-unchanged:** `PrepareArgs`/`WritingEvidenceArgs`, `parse_valid`, the common `schema()`, the `prepare` handler, `refresh_schema`/`validate_refresh` and the rest of the helper code.
- **Guide bump.** Only the version constant and its two label assertions change.
- **Direct forged object still refused.** The INT-D `object` arm still runs both the typed `prepare()` call and the native `ErasedTool` call, and both return ToolRejected; the `if denied` block sits outside the calls loop.

## Tests and evidence
- **The red results are genuine.**
  - The before runs used an exact-base copy with test-only patches that touch only `threads_writing_tests.rs`.
  - before-offered was 1/2: ordinary and v1 actually offered object|null; v2 passed.
  - before-test-purpose was 15/3, and the batch test failed with `ordinary/false: Completed`.
- **The retained failures are what was described.**
  - candidate-intd 6/1/1 failed on `cancel.is_cancelled()`.
  - The first strict lint failed on "`if` has identical blocks".
  - agent_runs 14/1 is kept and is not relabelled.
- **The new tests are real, not mirrors of the helpers:**
  - actual first-request tools equal the budget definitions in each mode;
  - Codex and strict Copilot Responses wire bodies (the strict one has `writing` required and null-only);
  - six mixed-batch arms with no "> " Activity, progress, operations or candidates;
  - CW2 with a real read, a non-empty UTF-8 quote, requirements and an identical retry;
  - tool definitions frozen across a lazy skill load;
  - the guide stamp recomputed after a change and reused when unchanged.
- **The observer fixture change matches its allocation exactly.**

## Blocking findings
None.

## Qualification
**Q1 (MEDIUM, test evidence).** The "missing/null still prepare" test asserts NeedsReview by checking that the serialized requests contain the string "NeedsReview" (`threads_writing_tests.rs:~1158`). Every request includes the `apply_changes` description at `threads.rs:719`, which already contains that word, so the check always passes.
- What is still proven: the Note is unchanged, there is no receipt, and the prepared writes are exact.
- What is not proven: that the Apply outcome was specifically NeedsReview. The "get NeedsReview" acceptance row overclaims.
- Bounded remedy (needs allocation): capture Activity and assert `"< apply_changes · NeedsReview"` (`tool_outcome_label`, `:2205`), then do one focused rerun. Alternatively, reword the acceptance row.

## Optional residuals (none block)
1. **MEDIUM, existing before this diff and not changed by it.** `refresh_schema` (`threads_history.rs:112-116`) inherits `writing`, but `RefreshArgs` rejects unknown fields and has no `writing`. The cached strict sanitizer (`rig-core` `providers/internal/schema.rs`) makes every property required. On Copilot Responses, a compliant `refresh_candidate` call must therefore send `writing` and will be rejected as an invalid batch. Normal Codex routes offer the same unusable field. I found this by reading source only; it needs a separate allocation.
2. **LOW.** `threads-behavior.md:107` lists bundles only up to 12, and `:156` does not describe the scoped prepare contract. `status.md` has no checkpoint for this change. The docs allocation excluded these, so the principal could add them at acceptance.
3. **INFO.** Some test code is now dead:
   - the third "long" turn in the `object` arm is never used;
   - the `object-long` branches inside the success `else` block cannot run;
   - `long_id_visibility` is always `None`.
4. **INFO.** `budget_tool_definitions` recomputes `for_scope` rather than reusing the run's value. This is equivalent because the snapshot is fixed for the run, and the per-mode equality tests cover it.
5. **INFO.** In the in-file batch test, the v2 arm is only the unknown-field negative. The positive v2 path comes from the CW2 model-loop test.

## Retained limits
- The original three errors remain UNKNOWN. The captured sample shows a sufficient refusal condition, not where the rejection happened.
- The wire evidence is synthetic serialization only, with no remote provider acceptance.
- Old preflights are stale, and no rebind or reactivation is authorized.
- SDK 474/4/1, widget 314/1/1, PDF, platform/hosted/native-human, compatibility, INT-W, main/owner/program and live I01 all remain open.
