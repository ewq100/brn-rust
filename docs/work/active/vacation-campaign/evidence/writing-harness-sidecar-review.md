# Independent sidecar correction review

Disposition: **no remaining consequential source/guard findings in the reviewed sidecar revision**. The DATA-layout incompatibility that caused the first reserved launch to fail is corrected without changing production code or bypassing its directory guard. One non-blocking README accuracy note is recorded below. This disposition is independent source review, not proof of an actual provider result or native acceptance.

The original two reports remain preserved at `/tmp/brn-writing-harness-review.md` and `/tmp/brn-writing-harness-final-review.md`. The latter review missed the earlier DATA metadata placement incompatibility; the failed attempt and this correction remain evidence of that limitation, rather than being erased by the new disposition. Prior complete source/binary/logs are preserved in `before-sidecar-fix/`.

## Actual failure and correction

Read the first failed launch's stderr, launch receipt, existing-root refusal evidence and actual ledger. Attempt `e77bcbb1-088e-4e2c-a0d4-23f7cea00e1b` failed at normal Workspace reopen, before run preparation/spawn. `core/store.rs:108-126` accepts only its declared database/marker/derived/skills entries; the earlier manifest.json inside DATA violated that contract. The ledger retains the launch as failed and consumed, with provider_dispatch_observed=false. There is now one 20 October attempt and three historical 19 October attempts; the reviewer did not make a new reservation or call.

Read every line of the complete 953-line final source and compared the complete sidecar diff against the preserved previous revision. All manifest/dispatch/result accesses, including Unknown persistence and atomic temporary JSON paths, now receive the deterministic sibling `<DATA-leaf>.harness-evidence` directory. No harness JSON path remains inside DATA. `read_manifest`'s parameter retains the name `data`, but all actual callers pass the sidecar. Production Workspace open/reopen/run APIs continue to receive only canonical DATA.

Preparation validates canonical parent/absent DATA, derives the sibling from a single UTF-8 leaf, refuses an existing sidecar, exclusively claims DATA first, then exclusively claims the sidecar before opening Workspace. An occupied/symlink sidecar is not followed by create_dir. Failures may leave task-owned partial fresh directories, but cannot reuse existing DATA. Run/verify recompute the same sibling and require it to be an existing canonical directory; arbitrary/aliased sidecars are refused. The manifest still binds the exact canonical DATA path and source/binary identities. The exact canonical ledger data_directory requirement is unchanged, so the earlier sibling-DATA reservation issue remains fixed.

After publishing only sidecar evidence, prepare drops the initial Workspace and reopens DATA through normal `Workspace::open_existing_no_upgrade`, then verifies exact source/sentinel/comment/pending state and empty candidate list before reporting success. This is a relevant normal-API compatibility witness for the failed seam. Read the supplied offline logs: one initial noncanonical-parent refusal, then successful scratch prepare/reopen for `/private/tmp/brn-writing-sidecar-smoke-20261020/data`. The log contains synthetic IDs and a boolean auth-location indication, with no credential material. These are writer/lead execution receipts inspected by the reviewer; no mode was executed during review.

## Guards retained

The source/binary/request hashes, exact frozen v2 single-comment scenario, four-round production RewriteNote request, empty preflight candidates, unchanged canonical Note/sentinel/comment hashes and versions, exact useful whole-note proposal and no Apply/Undo receipts are unchanged. The pre-spawn dispatch marker and result are in the same deterministic sidecar; same-DATA retry refusal remains effective after uncertain outcomes. Atomic JSON publication leaves its temporary file in that sidecar as well.

The 180-second deadline, cancellation token, five-second settlement grace, Unknown persistence and explicit unconfirmed upstream cancellation are unchanged. Standalone verify still rejects every deadline result and requires saved success plus actual Completed Run, exact thread/provider/model/budget, candidate operation membership and matching saved/observed candidate identities. The production durable writing-marker gate continues to return NeedsReview for non-owner authority; no Apply/Undo mode or alternate authority route was added. No auth bootstrap or credential code changed, and the new normal reopen does not read auth material.

Cargo.toml/lock are byte-identical to the preceding independently reviewed revision. No dependency/feature/transport acquisition or production framework was added. Existing empty brn-ai production feature and no Rig test-utils boundaries remain unchanged. Product checkout is still f8e725385e56172e2ac654409d5af3e20352e1c8; comparison to b0eb18d7 shows no product crates/agent/manifest/lock/toolchain changes.

## Non-blocking documentation note

At review time README's compile paragraph still says the executable “has not been run” and no fixtures/runtime modes were invoked. Its own build-binding and offline logs correctly record offline prepare/reopen execution. Update that paragraph to distinguish offline prepare from provider/account/native execution. This does not affect compiled guards or the sidecar fix.

## Exact reviewed SHA-256

Harness root: `/Users/evokessler/.codex/worktrees/vacation-hardening/codex-writing-evidence/harness`.

- src/main.rs: `7dba7b899d807b157353e9cb13b3ed2a490bd60eb45deba496b0d431c66c2842`
- Cargo.toml: `5340dfde69a293a2031b9773eb87f3528a749ac54e8d4df00cf6e25d0f7f8195`
- Cargo.lock: `47cba3e6ebe8c7e30b4596580774df99f0a7657020b837f20787c86f8e658c42`
- Ordered Cargo.toml + Cargo.lock + src/main.rs aggregate: `ead7ab6f903d2f5094783fb6275cc75b9a5b05a9420a853cc8dd6d823416ae3b`
- target/debug/brn-writing-smoke-harness: `a3abe44d89d64d849f1406b4d73e72ead75919e9fe202c21a664962706a47e7e`
- README.md observed: `44c37da45ffa965498d7e23ae11f5122b46d2abe1b7cfbb2934448203019b371`
- compile.log: `f1c74ca90692b49d469d6959ae384f5be1c9d8ceea6490409f794366be64e7fb`
- feature-tree.log: `2ac961f58f3c84afed3520e735eeae6f13e2c1b6d90e525166920cb282f4d87c`
- build-binding.json: `18512a39e39a5285c47e7f8570361173160f19e48eca65f7bd574cea88e77735`
- offline-prepare-refusal.log: `001283bbfea8828d7f6ec7cd65c0ae01310c51e37d1e1fe87e5f3db034aca036`
- offline-prepare-reopen.log: `8064238134a6ac893a1e021b0e7483089fe2bcd69333044c44182135d7b4546e`

Hashes and aggregate were independently calculated. The linked executable contains the complete exact current Cargo.toml, lock and main.rs byte sequences at offsets 24658217, 24658880 and 24735242. Final compiler evidence records a successful locked/offline build under the recorded Rust 1.98.1 toolchain, preserving earlier failures. It was not rebuilt or invoked by the reviewer.

## Limits

Only read-only source, metadata, hashes, compile/offline logs, synthetic failure receipts and ledger inspection occurred; only this requested report was written. No app/build/test/mode/account/provider/credential/native/Page/Git mutation or helper action occurred during this review. The scratch success establishes offline fresh preparation/normal reopen compatibility only. Provider availability/actual account outcome, usefulness, Mac Apply/Undo and owner acceptance remain pending.

The lead owns current quota/date/expiry checks, single actor coordination, a new exact fresh-DATA reservation, any next bounded dispatch, all post-run verification and ledger disposition. The failed first fixture must remain preserved and consumed. The fixed date in source checks ledger labels rather than a live clock; the existing five-percent floor and failure/uncertainty accounting remain applicable. The reported 97 percent remaining capacity was supplied by the lead and was not independently refreshed by this read-only reviewer.
