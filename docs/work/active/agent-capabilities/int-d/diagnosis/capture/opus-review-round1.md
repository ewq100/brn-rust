**Verdict: APPROVE_WITH_QUALIFICATIONS.** I found no real bug, leak, behaviour change or overclaim, and no mandatory changes.

This was a read-only review: no edits, builds, tests, network, auth, provider calls, GUI, git mutations or delegation. I did not use the code-review skill because it runs subagents. Every test or check result I cite comes from the recorded receipts, not from a run of mine.

## Identity and diff
- **Pins:** branch `work/int-d-20261017`. Material `e9607282` has tree `96922fa3…` and crates `f6666cb9…`. Base `33d9e10d` has crates `60c3b06b…`.
- **Diff hash:** `git diff BASE..MATERIAL | sha256` = `ef1f171a…a0f9b294bf81beab445fc4a0`, matching the stated value.
- **Source changes:** only `threads.rs` (+24 lines) and `threads_int_d_tests.rs` (+505 lines, appended in a single hunk after line 1021). The rest of the material diff is docs under `diagnosis/capture/**`.
- **HEAD `031b0bf3`:** adds only `handoff.md`, `release.json` and `source-pin.json`. No non-doc file changes between base and HEAD.
- **Strip check:** I removed the two exact blocks from the material `threads.rs` in memory, giving 2 matches and a byte-identical base (sha `f0bb6081…`). The old test file is an exact byte prefix of the new one.

## Axis 1: Standards
- **cfg guards are correct.**
  - The call blocks at `threads.rs:2252-2263` and `:2326-2337` use the same cfg as the module declaration at `:50-56`: `all(test, feature="scripted-runs", any(macos, linux))`. On Windows, without the feature, or in a non-test build, both the calls and the module are absent, so nothing dangles.
  - Other crates' integration tests compile brn-ai without `cfg(test)`, so the CLI and desktop are unaffected.
  - The external-MCP branches return before both hooks (`:2213-2226` and `:2301-2320`).
- **The hooks are passive.**
  - They reuse `self.emit` and add no fields, globals or TLS.
  - They come before the unchanged trace and Activity calls and the `helper_denial` return. `DispatchAction` and `OutcomeAction` are untouched.
  - The projection code has no panic path: indexing a missing key is immutable, and `total - shown` cannot underflow.
- **Typed outcome classes are static (checked in the Rig 0.44 source).**
  - `ToolErrorKind::as_str` is a `const fn` returning `&'static str` over a fieldless enum.
  - `status_name` is `&'static str`.
  - `ToolExecutionError::source()` returns the stored source directly.
  - `AiErrorKind` (`lib.rs:113-132`) is a fieldless `Copy` enum serialized in snake_case. Only `.kind` is serialized, never `detail`.
  - No Display/Debug, message or code is read.
- **Dispatch ID:** `EffectId` is documented as "unique per dispatcher and strictly increasing", and the doc correctly scopes its correlation claim to the dispatcher.
- **Bounds hold.**
  - The largest possible dispatch frame is about 1.2 KB (2 writes and 4 inputs at 57-byte IDs, plus a 20-digit version each), so the 2048-byte cap, prefix included, holds by construction.
  - Raw arguments over 1 MiB are labelled unavailable, with no regex fallback.
  - The request-key hash threshold of 128 bytes matches `parse_valid` (`threads.rs:495`).
  - The ID grammar matches `creation_id` (`uuid:index`, lowercase from `Uuid::to_string`) and `remap`'s `new:N` with N < 64.
  - The default expectation of `Current` matches `InputExpectation`'s serde default.
- **Tests run the real Rig loop.** They use `MockCompletionModel` through `run_model` with sequential execution (`tool_concurrency(1)`; Rig `engine.rs:577` polls strictly in call order). So prepare dispatch, handler and outcome all finish before the apply dispatch, and Stop really fires at the first prepare outcome. The Storage check is correctly labelled as projection-only.

## Axis 2: Spec
- **Long-payload witness:** all modes pass more than 2048 bytes of arguments and `parse_valid`. The capture shows exact `writing` ABSENT, NULL or OBJECT, the write ID, version and kind, and all three input IDs with versions and `Current` expectation, with omitted = 0. Six canaries are absent from every frame.
- **Object-long rejection:** the real `ToolRejected` here comes from `prepare` rejecting a `writing` object with no writing snapshot (`threads.rs:1740-1747`), not from the quote content. The doc does not claim otherwise.
- **Stop guard:**
  - `apply()` calls `live()` first (`threads.rs:1838`), which requires `Working` and a fence match, so the checked Stop fences the handler.
  - The apply-first case stops at the `> apply_changes` dispatch Activity, before the handler.
  - All cases assert 1 mock request, Interrupted with fence 2, late `finish` refused, the Note and its receipts unchanged, no operation receipt, no Apply or NeedsReview trace outcome, and no capture prefix in the Trace or canonical records.
- **Claim honesty:** I found no overclaims.
  - No runner, gate or env path exists, and activation is clearly marked unimplemented and unexecuted.
  - The historical cause stays UNKNOWN, there are 0 live starts, and the old 10 unused starts carry no authority.
  - The initial passes and the label correction keep their own artifact hashes (`5e8d…` vs final `8460…`).

## Mandatory changes
None.

## Optional (LOW or INFO; none block)
1. **LOW: a write to a Project record is labelled `kind: "UNKNOWN"`** (`threads_int_d_tests.rs:1072-1079`), although `parse_valid` allows it. The write's static `archived` boolean is also not projected. Adding both would be safe and would help a single paid capture.
2. **LOW: invalid rows look like capped rows.** When there are more than 4 inputs (or 2 writes), `omitted` cannot distinguish "invalid ID shape" from "beyond cap" (`:1051-1095`). A hallucinated or garbled ID is a plausible failure class, so a static per-row `id_shape` (UUID_INDEX, NEW or OTHER) plus byte length, or separate invalid/capped counts, would add value without leaking.
3. **INFO: missing arguments are labelled `UNAVAILABLE_PARSE`** (`:1121`) rather than a distinct missing label.
4. **INFO: the cap fallback is untested** (`:1126-1135`), but it is unreachable under the current caps.
5. **INFO: the hooks fire in every brn-ai scripted unit test on macOS/Linux, not just INT-D.** Only the focused module was run. Statically, the other Activity assertions (`threads.rs:5169,5177`, `threads_search_tests.rs:146`, `threads_research_tests.rs:1041`) are prefix-anchored or case-sensitive `.any()` checks and do not collide. This is unverified by execution.
6. **INFO: helper-wrapped frames can exceed the cap.** `helper_emit` adds `"  ↳ #i "` in front of frames, so the 2048-byte bound holds at BatchHook, not after wrapping. Helpers are off for the future capture, so this is moot.

## Required before any future one-start activation (not now)
- A separately reviewed ignored-runner source that refuses before Auth, with local tests for gate rejection and for reserving the single start before Auth.
- The full fixture, corpus and route pin set listed in `capture-method.md`, including the main DB plus `-wal`/`-shm` (closes the earlier O1).
- A decision on whether `retain_progress`, `withdraw_proposal` or `refresh_candidate` effects before the first prepare are acceptable. The offline guard covers only `apply_changes`.
- An explicit statement of expected yield. `rejected()` is used across many `prepare` branches, so a `tool_rejected` result plus IDs and versions narrows the cause through post-hoc canonical comparison but may not identify it uniquely.

## Retained qualifications
- No Linux, hosted or Windows build was run; the cfg is checked from source only.
- Strip invariance is a source check, not binary equality.
- O1, O2 and O3; workspace 1234/1/6 with `PDFUnavailable`; the original workspace 1224/5/6; SDK 474/4/1 and widget 314/1/1, still unexplained.
- Older compatibility source, SDK timeout and NACK, F3, platform and native-human checks.
- INT-W is separate; main, owner and program acceptance are pending.
- Live I01 is incomplete: 2 historical starts, neither producing a candidate, and the window is closed.
- The three original prepare errors remain UNKNOWN. No paid capture is authorized, and none is implied by this approval.
