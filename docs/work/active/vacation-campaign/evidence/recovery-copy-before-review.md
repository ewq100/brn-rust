# Independent before-source recovery-copy witness review

## Disposition and exact identities

**Approve exact witness `2ff42451adf97268884afd3a0b91c3ec97922e3f` and the two exact inert adapters below for ONE offline synthetic baseline invocation, under the lead's sole shared-build-slot ownership. No actionable findings for that bounded launch.** Compilation and actual failure are pending; this review does not claim a reproduced product defect or authorize another attempt. Preserve the raw result and stop/reassess if it is not the intended failure.

Verified clean architecture-status-route checkout, branch `codex/recovery-copy-privacy`, HEAD `2ff42451adf97268884afd3a0b91c3ec97922e3f`, direct parent `13813f6ff827de776a663036f5d468500e7a5ed1`. Complete actual parent-to-candidate diff is only `crates/brn-desktop/src/threads_polish_tests.rs`, 155 added lines; no production source changed. The existing general stale-copy positive witness is untouched. Complete patch matches `/tmp/brn-recovery-copy-privacy-witness.patch`, SHA-256 `3877096527a860a64ad7f8c3a1bb7395eaea386e225196eb6e1df9e9dca91e4a`. Whole test-file SHA-256 is `8f029275d8448d5c33bc028f03e87d217083d7fb452050c78259129a7e140ac4`.

Adapter identities independently hashed:

- `/tmp/brn-recovery-copy-before-qualification/run.sh`: `067a8ace0b5b16d71de365a9d85a4c6504250210bd164f529a72f45f3aeec7a6`
- `/tmp/brn-recovery-copy-before-qualification/supervise.py`: `9b64ebff2f9bb08a1c7c6a43551f436e7a9e13211d1dc65a0c1533e7ef959133`

Reviewer authored neither witness, implementation, nor adapters. The separate design assessment is unchanged. I read the entire actual diff, author report, fixture/update/input helpers, current SaveCopy path, and both complete adapter files, including their change from the previously reviewed bounded supervisor. Performed read-only source/Git/hash comparisons only; no build, test, workload, clean, fixture creation, source mutation, native/account/network/cache action, or helper was run. This report is the only write.

## Witness validity and reachability

The test reuses the existing real GPUI `TestAppContext` polish fixture at 900×600. That fixture creates a fresh canonical temporary synthetic workspace, persists `offline-fixture` provider with no account locations before UI creation, and removes polling (`view.poll=None`). It does not dispatch a provider. Existing constructor search preparation may run derived local work; this is not a claim of isolated process activity or native OS interaction.

The private source is established through a checked owner change setting `Note.chat` to a real synthetic Thread. The test opens that actual private context and verifies initial visibility in that chat and invisibility in general scope. It then advances canonical markdown through the existing `change_saved` checked owner helper **before** creating the edit guard; this is a valid stale-origin sequence, not an attempt to bypass an established dirty guard. That helper refreshes rows without replacing the open Note's basis.

It focuses the real editor, sets its insertion selection, and delivers actual `window.input` containing multiline `õun 私密` and a unique search marker. No direct SaveCopy handler invocation substitutes for the UI route. It asserts that a real owned edit session exists, that its durable markdown equals the visible draft, that the row is DirtyStale, and that the registered `save-buffer-copy` control is visible, then clicks that control through the window.

Post-click assertions first verify the newer canonical private source remains unchanged and visible in its private chat, and that the exact original recovery session—including bytes/generation/state—remains available. The distinctive assertion `private recovery copy entered general Notes` then rejects the current source behavior of copying that exact private draft into a general Note. Further assertions cover general scoped records, default search of the unique marker, and general export through a fresh destination inside the fixture-owned temporary root. The export also excludes the private source. Export uses the public shared service; no raw SQL, owner data, or private sweep is introduced.

This provides a meaningful new boundary over the existing public stale-copy witness. The check is deliberately policy-neutral about a hypothetical safe private copy versus refusal, which is appropriate for diagnosing the before-source privacy violation. The lead has now selected refusal; final implementation review should additionally establish its clear notice, unchanged visible editor/selection/context, historical-private/current-private service boundaries, ordinary stale-public success, exact provenance and replay. Those final behavior assertions are not claimed by this before-source test. In particular this test does not exercise a private historical basis with a now-general source, a source privacy change between service read and Apply, or private Project destination duplication.

The expected old-source failure is source-supported but remains unexecuted. If the intended general-Notes assertion fails, later general search/export assertions are **not reached**; do not report them as separately reproduced baseline failures. Setup/session/source assertions are real prerequisites: a failure there must remain a harness/setup result, not the intended privacy baseline.

## Adapter safety and result classification

The runner pins exact HEAD, clean checkout and whole witness-file hash, enforces >3 GiB initial free space, and selects only `threads_ui::polish_tests::stale_private_note_copy_stays_out_of_general_knowledge` with `--exact --nocapture`. It uses pinned Rust 1.98.1, locked/offline/jobs1, native test-support features, and an explicit task TMPDIR. The test itself creates a fresh unique fixture below that parent; an already existing evidence/temp parent is not reused as canonical data.

Before the exact invocation it performs the explicitly selected package-only `cargo clean -p brn-desktop --locked --offline`. This invalidates mutable desktop artifacts/fingerprints to avoid the already diagnosed shared-target zero-test problem. It is neither a whole-target/helper/source clean nor an alteration of frozen bundles; the lead must hold the build slot during both clean and compile. The actual result should show desktop compilation and the expected named test, not just source guards.

Whole-group cleanup algorithms are unchanged from the previously reviewed supervisor: sampled 30-minute trigger and 1 GiB floor, owned new process group, TERM 10 s/KILL 10 s, independent bounded leader reap and final group resolution, operational error recording, post-run exact clean checkout check, <=8 MiB accepted log read, and durable completion receipt. Unknown/alive groups and cleanup/verification/persistence errors prevent acceptance. These are sampled/accepted bounds, not a hard live disk quota. No source overlay/restoration or retry loop exists.

The deliberate negative-result gate requires all of: actual child exit **101**, exact one-failed/zero-passed harness summary, distinctive privacy assertion, unchanged checkout, resolved/reaped group, and no cleanup errors. Only then does it set `baseline_reproduced`; `qualified` is always **false**. The supervisor intentionally preserves the actual nonzero code rather than turning a failing product test into exit-zero success. Zero tests, compile failure, another panic, a missing marker, timeout, and cleanup failure are not a reproduced baseline. Receipt persistence failure forces a failing supervisor result and exposes the persistence error; acceptance must use the durably saved receipt/raw log, not a fallback printed observation with `receipt_written=false`.

This review approves one diagnostic dispatch, not product qualification, native behavior, release/integration, private-copy semantics, or repeated attempts. Original failure evidence must remain intact for the later complete implementation review.
