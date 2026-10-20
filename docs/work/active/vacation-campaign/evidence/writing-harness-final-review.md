# Final independent writing-harness review

Disposition: **no remaining consequential findings in the corrected final source**. Both initial findings are fixed. This qualifies the reviewed source/guard corrections for lead-owned offline probes; it does not establish a successful account run or authorize dispatch independently of the existing policy.

The initial report remains preserved at `/tmp/brn-writing-harness-review.md`. The original source/binding/logs are retained in the harness's `before-review-fix/` directory. This report does not erase the original failed review or compiler failures.

## Finding dispositions

- **Initial P1, deadline failure promoted by verify: fixed.** Final `verify` requires `deadline_fired == false`, saved successful verdict, Completed saved terminal/state, actual canonical RunState::Completed, and the one observed candidate operation/Note ID matching the saved result. A Completed state observed during cancellation grace retains `deadline_fired=true` and cannot pass. Unknown, interrupted, missing Run, mismatched candidate and failed qualification paths also remain refused. The new Run check binds the writing thread, Codex provider, gpt-6-luna model, four-round budget and membership of the candidate operation in that exact Run's operations.
- **Initial P2, sibling-DATA reuse under evidence-directory fallback: fixed.** Reservation now requires an explicit absolute canonical `data_directory` exactly equal to the validated DATA path. There is no evidence-directory fallback. Combined with the pre-spawn durable DATA dispatch marker and refusal of existing/result markers, this closes the reviewed same-attempt sibling-restart route. The lead remains responsible for not deleting markers or reusing consumed reservations.

## Scope and production compatibility

Re-read the complete 914-line final source, compared every change with the preserved initial source, inspected final README/build binding/feature/compile evidence, and confirmed Cargo.toml and Cargo.lock are unchanged. The code changes are restricted to importing RunState, strengthening verify, and narrowing reservation DATA binding. README describes those changes accurately. No new transport, framework, production command, dependency or feature was added.

Production `Run` exposes the checked thread/provider/model/budget/state/operations fields. Runtime creation persists Codex, selected model, budget as `max_tool_rounds as u64`, and effort; prepare records each proposal operation in the current Run before immutable preparation; normal finish maps AiTerminal::Completed to RunState::Completed. Thus the new provenance checks match actual public APIs and successful normal-runtime persistence. The new indexing expression is protected by the preceding `candidate_operations.len() == 1` short-circuit. A failed/missing provenance read refuses verification.

Previously reviewed behavior remains unchanged: exclusive fresh-root preparation, source/binary/request binding, exact synthetic frozen v2 Note/comment, unchanged canonical Note/sentinel/comment records, exact useful whole-note candidate, pending feedback, no Apply/Undo receipts, one active Note candidate enforced by core, ordinary auth bootstrap and production run_prepared, four-round guard, 180-second cancellation plus five-second settlement observation, sanitized outcome reporting, and unconfirmed upstream cancellation. Core durable writing-marker review enforcement remains the authority boundary; prompt wording is not its substitute.

Product checkout remains f8e725385e56172e2ac654409d5af3e20352e1c8, with no product source/manifests/lock/toolchain differences from b0eb18d7 in the reviewed paths. Existing progress-document changes were left untouched. The policy header now accurately routes dated counts to the ledger without changing limits. The read ledger still has three 19 October attempts and zero 20 October attempts.

## Exact final SHA-256 identities

Harness: `/Users/evokessler/.codex/worktrees/vacation-hardening/codex-writing-evidence/harness`.

- `src/main.rs`: `547b5f29a76077886a4b98d941cd11de60903e18793168f3c5ac7ec8b4dce7cf`
- `Cargo.toml`: `5340dfde69a293a2031b9773eb87f3528a749ac54e8d4df00cf6e25d0f7f8195`
- `Cargo.lock`: `47cba3e6ebe8c7e30b4596580774df99f0a7657020b837f20787c86f8e658c42`
- Ordered Cargo.toml + Cargo.lock + src/main.rs aggregate: `61ec649ebaa6e6b0099a4fe55ef86dd7b581d325642d0bbd07753458914c4d56`
- `target/debug/brn-writing-smoke-harness`: `e8c79ae9b1c2434696fb615416f6e0180c0adeeb06d85f146bbff11a627c353e`
- README: `47965f29b210990bd7238a546b7f49ceba5f756617b59b9ee201f83dd870dbb4`
- compile.log: `9a7d37785612048ccd0600f558ff043eced17195d2a250f71cef13ab394fe13d`
- feature-tree.log: `2ac961f58f3c84afed3520e735eeae6f13e2c1b6d90e525166920cb282f4d87c`
- build-binding.json: `2b9446719b8682acd297c729bcfa9bc129187d4717c396a27460f2ae85a7bfcd`

Independent aggregate calculation matches the final binding. The executable contains the exact complete final Cargo.toml, Cargo.lock and main.rs byte sequences at offsets 24654121, 24654784 and 24731146. Compiled brn-ai features remain empty; compiled Rig feature metadata omits test-utils. The retained final compiler evidence includes the correction's initial missing-RunState-import failure and then a successful locked/offline final build under the recorded Rust 1.98.1 toolchain. The reviewer did not reproduce compilation or execute the binary.

## Limits and next gate

Read-only source/guard review is complete. No tests/builds, executable invocation, prepare/run/verify mode, synthetic fixture initialization, credentials/private files, account/provider/native/Page calls, dependency/model acquisition, Git mutation or nested helper ran in either review. Only the requested independent report files were written.

The lead owns offline probes, actual quota checks before/after, current Tallinn date and campaign expiry checks, single actor coordination, exact fresh DATA reservation, counting every failed/uncertain launch, final ledger disposition, and any at-most-one subsequent live dispatch. The fixed-date harness checks reservation labels, not a live clock. Current effective account/model availability, actual transport outcome, live usefulness, Mac Apply/Undo, native controls and owner acceptance remain unobserved. Unknown/deadline outcomes consume an attempt and must not cause an automatic retry.
