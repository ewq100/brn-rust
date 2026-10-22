# Independent complete candidate review — unique exact quote selector

Reviewed clean branch `codex/precise-edit-unique-quote`, exact HEAD `82dc62f048004c8d230523dfa5d8c6e436acaff9`, against base `c4f0e2e663cb04654c2bc49f9a540499fdd139b9`, in `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`. Both initial and final identity inspections showed the same clean HEAD. This report concerns that frozen candidate, not a later repair.

## Verdict

Two actionable P2 test defects block this candidate's deterministic qualification. No actionable production correctness, authority, privacy, compatibility or replay defect was found in the complete reviewed diff. Passing source inspection does not settle the failed tests or the qualification gates below.

## Findings

### P2 — Separate mixed-pair semantic rejection from field deserialization

Location: `crates/brn-ai/src/threads_note_edit_tests.rs:414–419` (also null-pair setup at 402–405).

The shared invalid-value loop includes `start:null,end:1` and `start:0,end:null`, then asserts that `serde_json::from_value::<EditArgs>` rejects each. Both keys are present and each value satisfies its field's `Option<usize>` deserializer, so these two deserializations succeed. Pair validity is intentionally enforced by `validate_arguments` at `threads_note_edit.rs:34–48`; a required-nullable field deserializer enforces presence, not the cross-field pair rule. The test necessarily fails before reaching the whole-read assertions below it. Keep absent/wrong-scalar/unknown-field cases in the deserialization-refusal table; assert the mixed pairs deserialize successfully but fail the semantic validator and actual handler before allocation. Do not weaken the mixed-pair refusal itself.

Disposition: unresolved in reviewed HEAD; lead correction and focused rerun required. This finding follows directly from the pinned Serde 1.0.229 derive and the production field deserializer; the reviewer ran no tests.

### P2 — Make the typed quote success witness select a permitted passage

Location: `crates/brn-ai/src/threads_note_edit_tests.rs:711–712`, consequential expectation at 766.

The new second model call selects `expected_text:"Monday"` with both offsets null. `tests::fixture()` creates a Note whose complete Markdown is exactly `"Monday"` (`threads.rs:4543`), and `begin` initializes an empty read ledger (`threads.rs:3906`). This test performs no note read, and the first integer preparation neither applies nor records a read. Therefore the second call resolves to `0..6` and correctly returns Invalid at the existing full-document guard (`threads_note_edit.rs:187–197`). It creates no second candidate, so `candidates().len()==2` fails. The history substring assertions can also be satisfied by the first call and the second call's argument text and are not independent proof of the new call's success. Use a proper local unique quote (for example `Mon`) or deliver a complete exact-version read before a full-note quote. Assert the result/operation of this specific second call, retaining the separate full-read refusal witness.

Disposition: unresolved in reviewed HEAD; lead correction and focused rerun required. Reviewer source reasoning only; no test execution claimed.

## Complete scope and confirmed behavior

Read every changed file: `agent/BRN.md`; `threads.rs`; `threads_guides.rs`; `threads_note_edit.rs`; `threads_note_edit_tests.rs`; `threads_research.rs`; `threads_summary_tests.rs`; new `threads_text.rs`; `threads_trace.rs`; `crates/brn/tests/agent_runs.rs`; `docs/architecture/threads-behavior.md`; and `docs/work/active/agent-first-simplification/targeted-edit.md`. Inspected surrounding consumers for registration/budget projection, batch validation, checked preparation, scope/project binding, Apply/read coverage, typed dispatch, research verification and guide/summary identity. Loaded applicable AGENTS, architecture/invariants, strategic programming, verification, crate README, review skill/resource policy, accepted design and independent design review in full, supplied legacy golden and corrected provider fixture map. The implementation handoff was context, not proof.

- Legacy field order and ordinary `Some(integer)` serialization remain unchanged. The golden literal/hash matches the independently retained old-DTO receipt. `legacy_binding` builds old ordered bytes independently of serializing the new DTO; the large-note witness seeds that binding, consumes it, applies, changes canonical version, replays immutable preparation and exercises fresh Continue receipt/read coverage.
- Required nullable keys have per-field `deserialize_with` and no default. Pinned Serde 1.0.229 `expr_is_missing` emits `missing_field` for this case. Pinned Rig Core 0.44.0 `ErasedTool::execute` parses `T::Args` before `Tool::call`; the direct raw-string missing-key witness targets that real seam. Raw run_model missing-key batches are ordinarily rejected by BRN before this seam, and are not independent SDK-deserialization proof.
- Source preserves access/live checks, argument binding, allocation and Run tracking before prepared lookup; replay remains before later Note/version/text resolution. Integer mismatch retains its original fixed reason and cannot activate quote search. Both nulls resolve only one nonempty exact occurrence in the checked current full Note. Empty/mixed calls reject before allocation; missing/ambiguous semantic calls can allocate a tracked empty operation but prepare no candidate. Full-document selection still checks actual read coverage.
- `threads_text` is a private child module; its `pub(super)` helper counts overlapping exact UTF-8 matches and returns a byte range. Research remains cited-range-first, then unique relocation, with existing version/privacy policy. No core/public API or parallel authority framework was introduced.
- The existing exact-edit request constructor, project/scope checks, checked prepare, Apply, dirty/stale/protected/review/Undo behavior and receipt read-accounting are reused. The existing exact-mode boundary tests were retained. Quote preparation and Apply replay do not mark unread content read. Rewrite/helper refusals remain in registration/dispatch and the handler.
- Bundle 19 is coherent across shipped parent/helper identity, CLI assertion and prior17/current19 summary invalidation/reuse. Historical targeted-edit evidence is preserved with a dated extension. The trace change adds only the new static quote-refusal reason to the existing exact allowlist; dynamic/payload-bearing reasons still project to the generic refusal.

## Evidence gaps and remaining gates

The new actual-provider-named test is Rig MockCompletionModel registration/history/typed-handler evidence, not a fixture HTTP provider-format matrix. The complete diff adds no `prepare_note_edit` route-specific HTTP fixture. Existing generic nullable integer route tests and shared-schema inspection provide compatibility precedent only. The accepted design/review originally requested route-specific evidence; the corrected lead fixture map explicitly distinguishes that precedent from direct new-tool HTTP qualification. Lead should record the current bounded disposition rather than claim new-tool HTTP qualification from this mock.

The accepted design review also requests an overlapping-quote regression through the actual research caller after helper extraction. The new helper unit cases cover ASCII/multibyte overlap; existing research relocation test covers unique, spaced duplicates, stale and hidden versions, but the actual-caller overlap regression is absent from this candidate. Retain it as pending focused evidence or explicitly disposition it; do not describe the helper-only case as caller proof.

No tests, builds, app/process launch, provider/account/native/network/credential operations, source/shared-record/Git mutations or nested agents were performed by this reviewer. Only this assigned `/tmp` report was written. Source inspection does not claim deterministic passes; lead-owned checks must record actual corrected candidate identity and outcomes. All three prior live account attempts failed the selected usefulness task and exhausted the session; no fourth call is selected. This candidate's real-model usefulness remains unmeasured. Native observation, owner acceptance, hosted required checks and normal integration remain separate gates.
