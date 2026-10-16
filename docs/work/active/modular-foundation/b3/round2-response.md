# B3 Opus2 response and bounded native qualification

Opus2 is **CHANGES_REQUIRED** for B3-1b. [Full report copied verbatim](opus-review-round2.md), SHA256 `b579354cd7a0e9227372de0681b5a8c322c6581e8199a5804beacd63a0967c68`. All other round1 fixes, intake resolution, shared start, privacy/replay and the prior guard classification were confirmed by the reviewer; they are not reopened.

Bound checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`, branch `work/b3-20261017`, clean starting docs HEAD `07c565a5af26187e54b1188c61b29cfc0ca1e004`, reviewed material `def881852d1f3aa24dca03f0f2bb52145b37faf6` / crates tree `181659aac2c095a9b38995abf9086739e1c20344`. At the recorded preparation stage the correction was uncommitted/uncompiled and D2 owned the sole heavy slot; no heavy command ran during that stage. The principal subsequently explicitly released D2 and granted this worker the sole slot. Actual subsequent results are recorded below; the preparation receipt remains unchanged as provenance.

## Exact correction

- `threads_ui.rs::complete_writing_reference_import`: replace the same-import-target dirty condition with `writing_setup_dirty(cx)` alone. A dirty hidden form for any Note/scope retains its existing target and edits. The successful canonical import remains; automatic attachment and replacement are skipped. The existing generic notice contains no other form's Note, brief or reference names. Receipt resolution, checked final target/version/access validation and clean-form already-attached idempotency are unchanged.
- The two existing Search/Attach context-change warnings in `threads_ui.rs` and `SetupDraft::attach_match` warning in `setup.rs` now say **Recheck and keep edits**, rather than Reload. Save-error text and other warning strings are unchanged.
- Append one native owner-path fixture; all 148,018 existing native test bytes remain an exact prefix. No previous assertion, event, helper or test identity changed.

No other source path changed. App/intake tests, manifests/lock, core/AI/CLI, importer/converter, library interfaces, routes and cache policy remain unchanged. [Exact hashes and prepared commands](round2-preparation.json) record all 328 material inputs; only these three allocated Rust paths differ from reviewed material.

## Narrow automatic-replacement audit

| Existing caller of `replace_writing_setup` | Why it can replace / disposition |
|---|---|
| `load_writing_setup` | Dirty forms take non-destructive Recheck and return first; replacement is clean-only. |
| `recheck_writing_setup`, different/missing target | Dirty forms receive the generic return-to-original-context notice; replacement is in its clean-only branch. |
| `save_writing_setup`, successful checked Save | Synchronous owner Save validated the exact displayed target and expected version, persisted those edits, then reloads them. Failed Save retains the form. |
| `complete_writing_reference_import`, successful attachment | Corrected unconditional dirty guard returns before attachment/replacement for every bound form, including hidden other-context forms. Clean attachment still uses the existing fresh validation. |
| `WritingRequest::ReloadSetup` | Explicit owner action labelled **Reload saved (discard edits)** with the discard explanation before the click. |

No additional replacement path or refactor is needed for this finding. The guard still relies on the existing brief-plus-reference `writing_setup_dirty` comparison; it does not introduce another draft, cached state, merge or authority rule.

## Genuine before fixture prepared before production edit

Fixture: `b3_import_completion_keeps_another_notes_hidden_dirty_setup`.

The test-only patch was captured on exact `def88185` production before editing its dirty guard or warnings. Patch SHA256 `4ea25f183153561797396076130620549c72f30fa8431a59c972f5f2f06aade9`; appended fixture SHA256 `948449a8863eb784ae2ec0699f35e1f7bb2b601e3dced537b28b4990ecaaa4d3`. Prepared source `/private/tmp/brn-b3-opus2-before-20261017-source` has synthetic Git/no remotes, baseline tree `45549edf4df9faab399537ff67c3d1243a18040f`, synthetic HEAD `56866be318d3fab5d47bc56335787bf1016d35ea`. Its only material change is the appended test. Separate target `/private/tmp/brn-b3-opus2-before-20261017-target` does not yet exist. Neither source nor target aliases candidate or earlier before/guard artifacts.

Prepared sequence:

1. Save clean A/B setups, capture A's actual import intent and install the existing synthetic Import worker/channel.
2. Navigate to B and open its clean form. Type B's brief through real text input; click existing purpose and attachment controls to change B's pending reference configuration.
3. Return to A with B's dirty form hidden. Deliver Converted through the channel and call actual `poll_worker`.
4. Assert B's brief, reference edits and original target survive; both saved setups remain unchanged; the canonical import is retained; no B-specific data appears in the generic notice or setup controls in A.
5. Return to B, click Recheck and Save, inspect exact persisted B configuration/A unchanged. Type a later edit, then click the visibly labelled explicit discard Reload and inspect the restored B configuration.

Prepared expected first before failure: **A completion must retain the other Note's hidden dirty brief**. At preparation this was unexecuted; the actual executed result below is separate evidence. Worker/conversion and a real checked search page are injected as in the accepted previous fixture; no real converter, native picker or provider starts. One different-Note sequence is prepared, without claiming all scope permutations or full F21.

## Static identity and next gate

Direct repository-pinned Rust 1.98.1 rustfmt passed for the three changed Rust files; `git diff --check` passed. Source diff SHA256 `02beefa56ce44e32a7846a6605268cd12e96e9b9dc8ed31af26f420cd3c85df9`. The 328-input fingerprint is `31a11c985b10c4ea6bca8a46e1060a75fb54f727c86d8faced488a608d3e463d`; eight-owned-path fingerprint `8543aa82281192b0edbfe9aa76d51d2682d114ba383ed6731fa4ed1bfc20c688`. JSON specifies the encoding and full maps.

**Prepared BUILD NEED (subsequently granted/executed below):** after a new explicit heavy grant, execute the appended fixture on the fixed before source/target, then the same fixture in the candidate's own source/target. Preserve fixture compile defects separately from any behavioral failure. Follow with one full native desktop suite (covers prior assertions), library native tests and strict native/test-support all-target lint for library and desktop, plus whole-workspace fmt. Exact proposed commands/environment remain in the preparation receipt as the unexecuted snapshot; the separate actual receipt below contains what subsequently ran. The three warning/guard changes do not alter default dependencies or app/CLI/retrieval production, so broad default/combined/headless repeats are not proposed solely for this correction. Principal can allocate additional relevant gates.

The current verified material is ready for the immutable compiled source/test pin and principal-owned Opus3. There is no source acceptance claim. Previous normal receipts stay at their original pins. The original two-ID B3 guard allocation is **closed at 217.265250/600 seconds**; no new mutation grant, budget or campaign is inferred. Hosted/native-human/IME/real picker/live converter fidelity/PDF/full F21/D1B integration/owner/program/principal acceptance remain open. All ownership outside the exact three active paths and B3 docs stays frozen; no acceptance or integration is claimed.


## Actual second correction window

[Eight actual command/source/artifact receipts](round2-checks.json), all with precommit HEAD `07c565a5`, associate the exact prepared material/diff with the commands. No future commit SHA is invented in those execution records. Rust/cargo 1.98.1, locked/offline, CARGO_INCREMENTAL=0 and fresh explicit synthetic TMPDIR were used. Before target is fixed only to the before source; candidate target `/private/tmp/brn-b3-20261017-target` is reused only for the same literal bound checkout.

| Actual command/gate | Result |
|---|---|
| Exact def88185 production + appended test-only fixture, separate source/target | Compiled, then **0 passed / 1 failed / 287 filtered** at the intended brief-retention assertion; 95.772 seconds |
| Corrected candidate, same new fixture | **1 passed / 287 filtered**; 19.331 seconds |
| All B3 native fixtures | **15 passed / 273 filtered**; 2.277 seconds |
| Native library tests | **44 passed**, zero doctests; 2.214 seconds |
| Full native desktop suite | **287 passed / 0 failed / 1 existing ignored / 0 filtered**;test body 5.53 seconds |
| Native library/desktop strict all-target test-support Clippy | Both passed with `-D warnings` |
| Workspace fmt check | Passed |

Before actual: left `Clean A setup`, right `Private B brief typed during A conversion`. There was **no fixture compiler defect** and no before rerun. That first assertion stops the before execution; later retention/setup/privacy/owner-control assertions are proven on the corrected candidate, not claimed executed in the failing baseline.

The corrected run exercises the actual owner typing/purpose/attachment events, Converted channel/poll_worker completion, retained B target/brief/references, saved A/B unchanged, canonical import retained, generic notice/no B-specific metadata and hidden setup controls in A, then actual B Recheck/Save and labelled explicit discard. All original A/B, CW2 privacy/cache/real typing, editor Undo, queue/Stop/version/replay assertions remain in the full passing native suite; the existing ignored fixture is unchanged.

A receipt-collector assertion initially expected absolute roots in `.d` files. Actual Rust 1.98.1 dep-info uses relative paths. This was an evidence-tool assumption, not a compile/test/cache defect: the original collector/error are retained. A separate audit resolved every executed artifact's dep-info file against its recorded command cwd and verified each source hash against that command's material map. All five executed test-artifact associations passed; before/candidate test binaries have distinct actual SHA256 values and separate target paths. Existing normal binaries/helper artifacts are explicitly marked incidental, not claimed newly compiled/executed by these commands. No Cargo or test was repeated for this audit.

Only the allocated three Rust paths differ; source diff/fingerprints remain exactly those prepared above. All 328 material hashes stayed unchanged during checks. Native code is compiled by tests/lint; no separate desktop build or exact maintained script execution is claimed. No unchanged broad default/combined/E2E/headless lane was repeated; earlier counts remain at earlier pins. Existing block 0.1.6 future-compatibility warning and carried PDF gap remain disclosed. No mutation ran or is granted. Principal Opus3, local acceptance/integration and all previously open hosted/native-human/owner/program/full-F21 gates remain pending.


## Immutable pin and release

Compiled source/test pin **93b92d7bc519d598c23e16562512b167a59d5c49**, tree `c28388560864f9633923b9b06c6c9f409a22cdf2`, crates `0afb215e543bc5a2512b1fac24107169a39818af`. Actual material review diff: def881852d1f3aa24dca03f0f2bb52145b37faf6..93b92d7bc519d598c23e16562512b167a59d5c49. No source changed after the eight recorded commands or during evidence collection. Subsequent commits are evidence only and retain that crates tree.

[Final release receipt](round2-heavy-release.json) records all 328 candidate hashes matching the compiled command inputs, the unchanged genuine before source/fixture, clean source pin, reaped children, no running command receipts and empty precise heavy/task-controller inventories. Heavy slot **explicitly released**; all original B3 source/test/shared hunks remain retained and frozen for principal-owned Opus3 and explicit handoff. [Review handoff](round2-handoff.md) gathers exact scope, evidence and limits. No further heavy work or mutation is authorized by this completed window.
