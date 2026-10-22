# Precise unique-quote selector implementation candidate

Checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`
Branch/base: `codex/precise-edit-unique-quote` at `c4f0e2e663cb04654c2bc49f9a540499fdd139b9`

## Exact source/diff inventory

- `crates/brn-ai/src/threads_note_edit.rs`: required nullable `Option<usize>` fields using per-field `deserialize_with`; pair-shape validation; closed nullable schema; integer/no-fallback versus null/null unique exact quote dispatch after existing allocation/replay/version gates; effective-range full-read guard and checked request path preserved.
- `crates/brn-ai/src/threads_text.rs` (new), `threads.rs`, `threads_research.rs`: private overlap-aware byte-range primitive shared by the two runtime callers. Research keeps its cited-range-first verdict and relocation policy.
- `threads_note_edit_tests.rs`: legacy golden bytes/hash and independently formatted legacy binding seed; prepared pre-bound operation reuse; unique quote after UTF-8 with later-slice context; missing/overlapping refusals; integer no-fallback; quote replay after canonical mutation; partial-read accounting; required-key/nullable/schema and full-read witnesses; typed Rig mock dispatch via actual runtime definitions.
- `threads_guides.rs`, `threads_summary_tests.rs`, `crates/brn/tests/agent_runs.rs`, `agent/BRN.md`: bump current bundle 17 to 19, retain prior/current summary invalidation fixture, parent/helper render checks and CLI identity. Bundle 18 artifact was not imported.
- `docs/architecture/threads-behavior.md`, `docs/work/active/agent-first-simplification/targeted-edit.md`: current contract and dated extension while retaining original M3 range-only record.

## Reuse and evidence approach

The former research `unique_offset` is moved to a private sibling module and upgraded to return a full byte range with explicit empty-quote refusal. It counts overlaps by advancing one Unicode scalar from the first match. No new public API, core export, migration, tool, or budget is introduced. Existing local-edit authority/review/Undo paths and research verdicts remain owners of their policies.

The lead supplied independently captured old-DTO bytes and SHA-256 `13823d0d216c0e2dad4ca192712be2acf333e123fbcbfdda00048bb6b3f0fc10`; the reported original-DTO run and independent Python hash confirm the fixture constant. The pre-bound test independently builds old field-order JSON from the legacy argument fields, seeds the existing operation binding, then checks reuse.

## Proposed verification and limitations

No build, test, formatter, application/process launch, network, account call, native interaction, or credential access was run, as directed by the lead. `git diff --check` passed. The candidate tests are source changes only; runtime/compile status is unverified. Typed dispatch witness uses Rig's existing scripted mock-runtime path, not a real external provider format matrix. Independent candidate review and serialized lead checks remain required; native observation, owner acceptance, and integration are not claimed. No unrelated files were changed.

## Corrective pass after lead compile

Lead's focused compile of `e4b9418d` failed before tests: `required_nullable_usize` resolved `Result` to BRN core's two-parameter alias. Corrected the signature to `std::result::Result<Option<usize>, D::Error>`. No tests had executed at that point. The corrective pass also restores the exact legacy integer mismatch reason (including its static trace allowlist), adds one bounded static quote-refusal reason to the existing privacy-safe projection, removes the now-misplaced research helper comment and the quote-wording string assertion, and expands targeted witnesses for overlapping Unicode, required nullable shape cases, preserved integer plus null/null actual Rig mock dispatch, no candidate/operation for invalid batch calls, and Apply replay/read coverage. Pinned formatting was run with the lead-specified command; no build or tests have run. Direct Rig 0.44 typed missing-key rejection is still pending the lead's verified erased-tool seam.

The follow-up now also includes the verified Rig Core 0.44.0 `ErasedTool::execute` path with direct raw JSON for missing `start`, missing `end`, and both missing. Each must yield `InvalidArgs` before changing Note, Run operation identities or candidate count; this is distinct from common run_model batch prevalidation and does not claim Agent catalog registration. The existing run_model mock test retains legacy integer dispatch and adds null/null dispatch, checking the exact null wire fields and prepared operation history. Overlapping multibyte quote `ΩΩ` in `ΩΩΩ` is explicitly covered. No test/build was run.

Lead-supplied initial compile failure: only the source alias mismatch described above; it ran no tests. Corrective code is prepared for lead's serialized compile/check sequence. Diff-check and pinned formatting pass. The direct ErasedTool test is based on the locally inspected pinned 0.44.0 API (`ToolContext::with_scope`, `ErasedTool::execute`, `ToolResult::is_error_kind(InvalidArgs)`).
