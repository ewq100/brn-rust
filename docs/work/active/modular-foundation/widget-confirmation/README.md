# Confirmation behavior tests: control entrance motion

**Local candidate on baseline `c8b3026f582c8770c24a984111f562becf529cad`, 16 October 2026.** Independent Copilot Opus review and integration belong to the principal. No publication, native observation, owner acceptance, release or B1a unblock is claimed.

## Change and reason

Two existing confirmation behavior tests now call the pinned public `cx.update(|cx| cx.set_reduce_motion(true))` before creating their fixtures:

- A: [`an_edit_already_open_elsewhere_offers_compare_and_discard`](../../../../../crates/brn-desktop/src/threads_writing_tests.rs).
- B: [`a_saved_working_rewrite_offers_the_same_confirmed_stop_in_the_note`](../../../../../crates/brn-desktop/src/threads_chat_tests.rs).

Each source hunk adds one comment and one setup call. All existing state assertions and real `window.click` move/down/up paths remain unchanged. There is no shared fixture override, retry, delay, recentering, direct callback or product change. `threads_components.rs` equals baseline bytes. All temporary diagnostic source and controls were removed; no diagnostic markers occur in the corrected executable. Ordinary/default/native production builds do not compile these test modules.

This controls real-time dialog entrance motion while checking the confirmation consequences. It does not test or change production animation.

## Failing-first evidence and limits

The original required hosted macOS run `38908335265`, job `116997504049`, at `8f768943` failed A: `edit_open` remained Some where None was required. An earlier local uninstrumented run failed B: Working remained where Interrupted was required. The required command was:

```sh
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline
```

The preserved [phase 3 report](phase3-report.md) is a historical diagnostic snapshot, before this correction. Its full command/artifact/control receipts are in [before-receipts.json](before-receipts.json). The source baseline had no production diff from `8f768943`.

For A, the [unforced seed64 discard trace](before-A-discard.txt) captures the original exact symptom. Kit's click kept center y137.5 while the animated button moved from top121.5 to top162.5 before mouse-up. The captured point was outside the new bounds; confirmation entry/return was absent; dialog and external edit stayed open. The checked discard never ran. A separate [unforced Compare-close trace](before-A-compare.txt) captures the same interaction problem before mouse-down.

The test helper uses one position across event redraws; the pinned GPUI non-synced entrance animation advances using real Instant elapsed time. Draining its test dispatcher does not guarantee that motion has settled. Forced80ms controls reproduced A, B and Compare-close failures; each corresponding public reduced-motion control passed with original assertions intact. These red/control results preceded the permanent change; no new red was fabricated for this phase.

**B's original cause remains unconfirmed.** Its [earlier uninstrumented failure](before-B-uninstrumented.txt) is real. Its forced control demonstrates the same timing vulnerability, but no unforced B red trace was obtained in phase3 (400 unforced iterations passed). This correction stabilizes its demonstrated test vulnerability; it does not prove the prior B failure had that cause.

Public bounds are not actual hitbox/clip/occlusion observations, tracing perturbs time, and debug1 differs from the original hosted artifact. The captured A result establishes moving-target missed delivery in that reproduced headless test; it does not establish a native product defect or every hosted failure's cause.

## Corrected verification

[after-receipts.json](after-receipts.json) records exact commands, non-secret explicit environment, source hashes, artifact identity, all per-process iteration counts/results and raw scratch-output hashes.

- Rust1.98.1 `rustfmt --check` on the allocated files and `git diff --check`: passed.
- Strict `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings`: passed. Command-line package debug overrides/JSON output flags are recorded in the receipt; no manifest change.
- Exact A and B: each **1 passed, 0 failed, 0 ignored, 309 filtered**.
- ONE corrected eight-process mixed batch: **800/800 GPUI iterations passed**, 400 per test, seeds0–99 per process, **10.193s** total stress. Eight libtest cases each passed; GPUI iterations are not separate libtest cases. Each process reported0 failed/0 ignored/309 filtered. No diagnostic/forced control variables were enabled. Maximum authorized100 iterations/process and120s total stress were respected; no second batch.
- Required `threads_ui::` Cargo command, once with the same recorded package-profile overrides: **306 passed, 0 failed, 1 ignored, 3 filtered**, test time3.00s (5.611s Cargo process). No failure was rerun to green.

All use explicit fresh synthetic TMPDIRs and offline fixtures. No provider/model download, owner data or GUI access. No broader default/retrieval checks or mutation campaign were run for this four-line test-only source change. Passing local headless checks does not establish hosted CI, native usability, owner acceptance or B's original attribution.

## Provenance and cache handling

Source checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-4145f1c6`, branch `work/widget-confirmation-diagnosis-20261016`; baseline/sourceHEAD `c8b3026f`. Source-only patch SHA256:

`9d55b1c16debfe0a00cb236db757617546f0badfa604809c794457c474b5de4c`

Actual corrected executable `brn_desktop-5e25dae08c408761`, frozen as `/private/tmp/brn-widget-sol-20261016/corrected-brn-desktop-phase4`, SHA256:

`a35858a2b9a6095095a56ed98d5be43c38e375921a2fa5fbccd55390804b8b8f`

Cargo reported a fresh=false desktop compilation from this checkout. Features are default/native-ui/native-test-support, opt1/debug1, debug assertions and overflow checks enabled. `[DEBUG-widget-sol]` and all `BRN_WIDGET_*` diagnostic strings are absent. The required suite ran the same executable SHA.

The shared cache was used only in the allocated serialized window, with `CARGO_INCREMENTAL=0`, locked/offline repository toolchain and command-line debug1 package overrides. Phase3's identity gate had rejected a misleading fresh artifact before tests and exposed CW1 enum metadata against baseline desktop source. Its successful all-local baseline rebuild and rejected receipts remain in before-receipts. This phase rebuilt corrected desktop and reused only the same verified phase3 baseline dependency artifact identities; no other worker builds were permitted between. Clippy likewise used test/dev debug1 overrides for all eight workspace packages, forcing its local checks without source workarounds. No cache deletion or compiler/dependency change occurred.

The executed diagnostic WIP (`a10d9d7076ac34892d43e264f289ac018c65ef1ca9838a6713fbbf52c6b40e37`) and all three complete files were archived in `/private/tmp/brn-widget-sol-20261016/phase4-before-cleanup/` before restoring exact baseline bytes and applying the two settings. Raw build/test/trace receipts remain under the same scratch root; bounded critical proofs and machine-readable receipts are retained here. Retained proof text copies omit trailing empty lines for Git whitespace checks; original bytes remain unchanged in scratch, with their SHA256 in before-receipts. No binary or full stress logs are committed.

## Handback

All build/Clippy/native-test children were reaped, final process check was empty, and the worker explicitly released the heavy slot before finishing this record. Source is frozen for the principal's independent review. Implemented and locally automated-verified only; not independently approved, integrated, native-observed or owner-accepted. B1a disposition stays with the principal.
