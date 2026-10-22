# Final independent complete candidate review — unique exact quote selector

Reviewed clean branch `codex/precise-edit-unique-quote`, frozen HEAD `28fd581ea8451ecd95d899248132b62c8f890d03`, against original base `c4f0e2e663cb04654c2bc49f9a540499fdd139b9`, in `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`. Initial and final inspections verified the same clean identity. This verdict applies to those exact bytes.

## Verdict

No actionable P1/P2 findings in the complete final candidate. Both P2 findings in the original `82dc62f048004c8d230523dfa5d8c6e436acaff9` review are fixed by source inspection. The two identified evidence gaps now have concrete actual-caller and three-route provider-format witnesses. This is an independent source review; deterministic qualification requires the lead's actual passing checks against this candidate.

## Prior findings and evidence dispositions

1. **Valid/fixed — mixed nullable pair test.** `threads_note_edit_tests.rs:410–435` now keeps missing keys, wrong scalar and unknown fields in the deserialization-refusal table. It separately proves each mixed pair deserializes, fails semantic validation and fails the actual preparation handler without changing the Run record or creating a candidate. Production pair rejection is intact.
2. **Valid/fixed — typed quote success test.** `threads_note_edit_tests.rs:704–814` now retains both legacy integer and explicit-null cases in independent fresh fixtures, each selecting local `Mon` within `Monday`. It correlates the specific tool call ID to its structured result and requires that result's operation to identify the sole prepared candidate. It no longer mistakes first-call history or the later call's arguments for proof of success. Fresh fixtures also honor the unchanged service rule refusing a second pending candidate for one Note (`brn-threads-core/src/store.rs:3035–3058`); no production guard was weakened to make this test pass. Separate whole-note quote refusal/read-success coverage remains at `threads_note_edit_tests.rs:444–464`.
3. **Addressed by focused witness — actual research overlap.** `threads_research_tests.rs:1739–1840` extends the existing real research verifier scenario with quote `ΩΩ` in `ΩΩΩ`, cited outside its actual range. It requires `mismatch`, null quote, no relocation, and the additional trace mismatch count. Existing unique relocation, spaced duplicates, stale/history and hidden-version assertions remain.
4. **Addressed by focused witness — new tool's provider formats.** `threads_note_edit_tests.rs:857–993` uses the existing scripted HTTP transport through actual pinned Rig completion models and production `run_model` registration/dispatch for ChatGPT Responses, Copilot Chat Completions and Copilot Responses. Each route gets fresh data and literal explicit null offsets. The first captured request must advertise closed parameters, all eight required keys and both integer/null unions. Exactly two streaming requests and one correlated `call_0` tool result are required. That result's nonempty operation must identify the sole prepared candidate; its Note must equal independently constructed expected metadata/content (`Tuesday`), while the canonical Note remains unchanged. Script exhaustion rejects missing/unexpected requests. These are meaningful deterministic provider-format/typed-dispatch assertions, not live service compliance or usefulness proof.

## Complete reviewed scope

Reviewed the entire original-base-to-final diff across all fourteen changed files, not just the repair increment:

- `agent/BRN.md`
- `crates/brn-ai/src/provider_formats_tests.rs`
- `crates/brn-ai/src/threads.rs`
- `crates/brn-ai/src/threads_guides.rs`
- `crates/brn-ai/src/threads_note_edit.rs`
- `crates/brn-ai/src/threads_note_edit_tests.rs`
- `crates/brn-ai/src/threads_research.rs`
- `crates/brn-ai/src/threads_research_tests.rs`
- `crates/brn-ai/src/threads_summary_tests.rs`
- `crates/brn-ai/src/threads_text.rs`
- `crates/brn-ai/src/threads_trace.rs`
- `crates/brn/tests/agent_runs.rs`
- `docs/architecture/threads-behavior.md`
- `docs/work/active/agent-first-simplification/targeted-edit.md`

Applied the repository AGENTS, architecture/invariants, strategic programming and review skill/resource policy already read for this review. Re-read the complete accepted design/design review, original82 review and `/tmp/brn-quote-http-implementation.md`. Inspected affected registration/budget/batch-validation, checked prepare, scope/project, Apply/read-ledger, research verdict, guide/summary and pinned SDK consumers. The handoff and reported checks were context, not substituted for source inspection or passing check logs.

## Confirmed source contracts

- Required nullable DTO fields use per-field deserializers without defaults. Missing keys remain errors even through pinned Rig Core0.44.0 `ErasedTool::execute`, which deserializes `T::Args` before calling the tool. The direct erased-tool witness separately covers each missing key and both missing, with no operation/candidate/Note effect. Ordinary run_model batch rejection is not mislabeled as SDK missing-key proof.
- Legacy integer serialization order/bytes and hash are retained. The independently captured old-DTO golden hash `13823d0d216c0e2dad4ca192712be2acf333e123fbcbfdda00048bb6b3f0fc10` remains fixed; a separate old-ordered-byte binding builder seeds the operation used by the large-note preparation/replay/Continue witness. This is stronger than merely calling the new serializer twice.
- Live/current access precedes allocation; complete argument binding and Run tracking precede prepared replay; prepared replay precedes later canonical version/text resolution. Integer/integer requests retain exact range matching, the original fixed refusal and no fallback. Null/null requests select only a nonempty unique exact quote in the checked current full Note; overlapping/missing quotes refuse. Mixed/empty shapes reject before allocation. Semantic refusals may retain an allocated empty operation but create no candidate. Changed arguments cannot reuse a bound key.
- The full-document effective range still requires actual full-read coverage. Host quote search does not advance the read ledger. Existing immutable preparation, project/scope binding, protection/review, dirty/stale, Apply/replay, fresh Continue and Undo machinery is reused. Legacy boundary witnesses are preserved. Frozen Rewrite and research helpers remain refused; no authority or tool-round expansion was added.
- The shared exact primitive remains a private `threads` child module with sibling-only `pub(super)` visibility. It counts overlapping UTF-8 occurrences and rejects empty quotes. Research keeps cited-range preference and its own relocation/version/privacy policy. No public core API or new framework was introduced.
- Provider fixture reuse widens only eight existing items/methods inside `#[cfg(test)] mod provider_formats_tests` to crate visibility. No production transport/API or new server harness was added. The existing transport pins synthetic endpoints, consumes only scripted replies and rejects unexpected requests.
- Bundle19 is coherent across current parent/helper rendering, CLI identity and prior17/current19 summary invalidation/reuse. Historical bundle18 attempt was not imported. Current behavior docs describe the extension, while the historical range-only record is preserved with a dated follow-up. The trace adds only the new fixed quote-refusal reason; raw payloads and dynamic errors remain excluded.

## Check and acceptance boundaries

Reviewer performed read-only Git/source/pinned SDK inspection and wrote only this assigned report. No tests, builds, application/process launches, source/shared-record/Git mutations, network/account/native/credential actions or nested agents were performed. Lead owns serialized check execution and must record actual exact28 results. Earlier compile/test failures remain historical failed attempts; this review does not convert an intermediate failure or a running check into a pass.

The three-route HTTP witness is deterministic scripted HTTP/SDK format evidence. It makes no external provider call and does not establish live service compliance, real-model usefulness or native usability. All three prior live account attempts failed the selected usefulness task; the session remains exhausted and no fourth call is selected. This extension's real-model usefulness is unmeasured. Native observation, owner acceptance, hosted required checks and normal integration remain separate gates.
