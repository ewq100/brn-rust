# Persistent anchored comment verification evidence

2026-09-28 · `feature/anchored-comments` · base main `1b6623a`.

## Execution and baseline

The requested checkout was clean at `1b6623a7b1f08602a84d3cabc88acdd86e57ca0f`. Work is isolated in the native-managed `anchored-comments/brn-rust` worktree. Astra owns architecture and independent review, Sol implementation, Luna narrow checks, and the controller native acceptance and integration. The user authorized routine decisions, commit and feature-branch push; merge and release are excluded.

Luna ran `cargo test --workspace --locked --offline` with Rust 1.98.1, `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/private/tmp/brn-editor-trial/target`, `CARGO_BUILD_JOBS=4`, and `CARGO_INCREMENTAL=0`: 79 tests passed, zero failed. Baseline log: `/private/tmp/brn-chunk12-baseline.log`. Disk inspection showed approximately 2.6 GiB free before and after baseline.

The existing editor experiment and [chunk 11 evidence](draft-revisions-evidence.md) inform the interaction design; their in-memory full-buffer history is not a production persistence model. Initial native access reported a locked Mac; the user was asked to unlock while independent work continued. The user subsequently unlocked the Mac and native app inventory succeeded. Native acceptance has not yet run for this chunk.

## Scope and qualification

Comment-batch generation, candidate adoption and publication are outside this chunk. Existing trial workspaces and the original vault must remain untouched. Native acceptance will use a fresh binary and a new disposable synthetic workspace. Subjective usability, IME/accessibility, and sustained near-limit performance remain unqualified.

## Existing regression checks

Luna ran editor trial tests (11 passed), provider harness tests (16 passed), `bash scripts/test-make-macos-app.sh` and `bash -n scripts/*.sh`; all passed with no live provider call. These checks did not build a native binary.

## Storage implementation checks

The pure mapper passed 11 focused tests and Astra review at `715aeb3`. It validates canonical UTF-8 replacements, examines equivalent edit alignments, rejects duplicate identity, and recovers only from exact validated immutable content. Store integration at `523bb2b` passed 60 tests: 15 unit, 20 comment, 9 draft, 13 storage and 3 workflow entries. Formatting and whitespace checks passed. New coverage includes dirty/clean atomic capture, forced rollback after checkpoint insertion, current/checkpoint mappings, legacy saves, Unicode edits, delete/recover/redo, trace limits, lifecycle CAS, corruption, frozen replay, v4 migration rollback, old draft receipts and child-process reopen. Astra identified two integrity gaps: ordinary-save frozen receipts needed complete-result integrity, and SQLite ambiguous-anchor CHECKs needed an explicit non-NULL reason. Sol reproduced both, fixed them at `6aceda9`, and passed 63 store tests. Scoped Astra re-review found both addressed and no new breakage.

## Workflow verification

The additive comment worker actions at `c98de2e` passed 15 workflow tests and Astra boundary review. They preserve existing draft/candidate actions, correlate capture/write results to submitted generations and worker jobs, and return matching saved comment projections with immutable recovery references. Store lint cleanup at `1c213db` passed strict all-target Clippy for `brn-store` and `brn-workflow` with `-D warnings`; the 63 storage tests passed again in the requested shared-target environment.

## Native implementation and full automated verification

Native implementation at `f942f40` passed 22 unit tests, five CLI tests, strict all-target native Clippy and a native-retrieval build. The disposable bundle at `/private/tmp/brn-comments-native-20260928/BRN Comments Check.app` was copied from that fresh binary, with matching SHA-256 `ec1210e4310c037b3b342d93d6dbcb3a8616fb8f60fcc3b2ed61caf0ef8c76a9`. The Mac had relocked before launch; another unlock was requested. No native interaction claim is made yet.

Luna's `bash scripts/verify-end-to-end.sh` at `f942f40` passed 116 tests and the provider-free fixture, along with build, formatting and strict workspace Clippy. Launcher regressions, shell syntax, 52 local Markdown links and whitespace checks passed. Commands used the requested shared target, pinned Rustup PATH, four jobs and no incremental compilation. Disk remained about 1.9 GiB free. Full log: `/private/tmp/brn-chunk12-final-e2e.log`.

A fresh Astra whole-branch review identified two native issues: per-keystroke recovery-text duplication/rehashing and stale preview after failed-save trace overflow. Sol corrected both at `7ec9917`: a shared validated recovery cache avoids per-keystroke checkpoint text copies/hashes, and failed-save trace recombination recomputes preview with the same history-limit/recovery rules. New failing regressions preceded the corrections. Native tests increased to 25 unit plus five CLI; strict native Clippy and the fresh build passed. Astra scoped re-review found both addressed and no new material issue. Native acceptance remains pending.

## Final verification checkpoint and pending native gate

After `7ec9917`, `bash scripts/verify-end-to-end.sh` passed again using the pinned shared-target environment: workspace formatting, build, strict all-target Clippy, 116 tests and the provider-free fixture. Log: `/private/tmp/brn-chunk12-final-e2e-after-fixes.log`. All implementation code has independent Astra review; no material finding remains open.

The disposable app bundle was refreshed from the post-fix native build: 725,232,688 bytes, SHA-256 `dbdd8188c4bd7d6bfef5a56a3cd02dfc5399ff6289d10447902c6fc685ed258b`. It points only to `/private/tmp/brn-comments-native-20260928/data` with no provider or model configured. Synthetic short and long-quote fixtures and a native checklist are beside that directory. Two obsolete generated test executables were removed from the shared Cargo target before the large link; no user data or existing trial workspace was removed. The known upstream `block 0.1.6` future-incompatibility notice and compact-unwind link warning do not prevent the passing build.

**Native acceptance is blocked, not verified.** The user unlocked the Mac during implementation and app inventory succeeded. It relocked before the fresh binary was ready for interaction. Repeated subsequent app-access attempts explicitly reported the lock; an unlock request remains pending. No chunk-12 native selection/comment, clipboard, scrolling, resolve/reopen, close or separate-process UI restart observation is claimed. Backend child-process persistence tests passed, but do not replace those native checks. Existing user trial apps/workspaces and the original vault were untouched.

To resume: unlock the Mac and open `/private/tmp/brn-comments-native-20260928/BRN Comments Check.app`. Follow its adjacent checklist using only disposable data, inspect exact stored provenance read-only, complete native restart, then update this evidence and status. Any discovered correction needs affected automated checks, a fresh native binary and scoped independent review. This branch is a reviewed implementation checkpoint awaiting native acceptance, not a verified completion of chunk 12.

Feature-branch publication is authorized; main must remain at its original checkout. No merge, release, model call, vault import, candidate adoption or publication action occurred. Subjective usability, IME/accessibility and sustained near-limit performance remain unqualified.
