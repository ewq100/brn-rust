# Phase 3 execution evidence — confirmation diagnosis

**A: reproduced moving-target missed pointer delivery confirmed in the instrumented headless test. B: original failure attribution remains UNCONFIRMED. No production fix, no B1a unblock. Heavy slot released; no further build/test/stress execution.**

## Checkout, patch and scope

Checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-4145f1c6`, branch `work/widget-confirmation-diagnosis-20261016`, HEAD `c8b3026f582c8770c24a984111f562becf529cad`. Core/desktop baseline remain c8b3026f; CW1 source not integrated. Only allocated `threads_components.rs`, exact A in `threads_writing_tests.rs`, and exact B in `threads_chat_tests.rs` modified. Original state assertions retained; no product/service/schema/manifest/lock/CI edits, no provider/model/user data/GUI, no delegation/commit/publication.

Executed WIP snapshot: `instrumentation-executed-wip.patch`, SHA256 `a10d9d7076ac34892d43e264f289ac018c65ef1ca9838a6713fbbf52c6b40e37`. It differs from prepared snapshot `instrumentation-wip.patch` (`bd2c6247...`) only by the two accepted diagnostic borrow fixes: obtain dialog presence before borrowing the view; the after-parked value is Option<bool> and printed with Debug. All instrumentation remains cfg(test + native-test-support), tagged `[DEBUG-widget-sol]`, opt-in. The public pointer path remains move/down/up at one captured center, four renders, no retry/recenter/direct callback.

Pinned rustfmt1.98.1 read-only check and `git diff --check` passed after compile fixes. These three source modifications remain uncommitted WIP; no next-phase correction applied.

## Artifact provenance and cache guard

All commands ran from the diagnostic checkout, `CARGO_INCREMENTAL=0`, `CARGO_TARGET_DIR=/private/tmp/brn-c0-20261016-target`, Rust1.98.1, locked/offline. Machine macOS26.5 arm64.

1. Initial ordinary no-run command returned exit0 in1.197s and Cargo reported `fresh=true`. Identity gate found NO diagnostic marker/control strings. Artifact SHA `c9ab28df93a90e09f80d365306dee5e5257c45559a859527d080e43da310a41e`, size69,725,888, differed from original phase1 baseline SHA `f08b1811a174c1954b34b29981703d1d2c637e46b2456c72c43b5737f7c71978`. **No tests ran on this rejected artifact.** Prebuild copy and fingerprint retained under historically named `baseline-brn_desktop-9a2a2eca0aadc8f6`/`baseline-fingerprint.json`; these are preserved shared-cache contents, NOT qualifying baseline receipts. `phase3-build-*` and `phase3-artifact.json` preserve the rejected fresh report.
2. Desktop-only command-line `profile.test.package.brn-desktop.debug=1` forced compilation: exit101,4.322s. It exposed cached CW1 `RecordData::WritingSetup` metadata against baseline desktop matches (baseline core source has no WritingSetup), plus two instrumentation-only E0502 borrow errors. No enum/source workaround made. Full JSON/errors retained in `phase3-rebuild.*`/`phase3-rebuild-messages.txt`.
3. Rebuilt ALL local workspace packages with distinct command-line debug1 overrides, retaining opt-level1, debug assertions and overflow checks. Exit0,22.016s, Cargo21.92s. All seven actual local graph packages show `fresh=false` and paths in this c8b3026f checkout: brn-intake, brn-retrieval, brn-threads-core, brn-threads-intake, brn-ai, brn-threads-app, brn-desktop. brn CLI override was specified but CLI was not built. No target deletion; unrelated/CW1 receipts preserved.

Successful exact build command (actual argument vector also in `phase3-local-rebuild-command.json`):

```sh
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/private/tmp/brn-c0-20261016-target cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop --locked --offline --no-run --message-format=json --config profile.test.package.brn.debug=1 --config profile.test.package.brn-retrieval.debug=1 --config profile.test.package.brn-desktop.debug=1 --config profile.test.package.brn-ai.debug=1 --config profile.test.package.brn-intake.debug=1 --config profile.test.package.brn-threads-core.debug=1 --config profile.test.package.brn-threads-intake.debug=1 --config profile.test.package.brn-threads-app.debug=1
```

Actual executable `/private/tmp/brn-c0-20261016-target/debug/deps/brn_desktop-5e25dae08c408761`, size74,049,840, SHA256 **fb09d33f790d04058a648adad38edf2ff9f5cfba6d30dd9442ead04615c9e2a6**. Features `default,native-ui,native-test-support`; opt1/debug1/test=true. Marker `[DEBUG-widget-sol]` and both pause/reduce-control strings verified in actual bytes before any tests. Frozen identical copy used in every probe: `/private/tmp/brn-widget-sol-20261016/instrumented-brn-desktop-phase3`. `phase3-instrumented-artifact.json` preserves sourceHEAD, executedWIP SHA, all local artifact JSON and executable identity. This is a qualified debug1 instrumented baseline+WIP build, not the original hosted/uninstrumented artifact.

## Exact probe execution and all ten serial comparisons

A = `threads_ui::writing_tests::an_edit_already_open_elsewhere_offers_compare_and_discard`.
B = `threads_ui::chat_tests::a_saved_working_rewrite_offers_the_same_confirmed_stop_in_the_note`.

Actual launcher: `python3 /private/tmp/brn-widget-sol-20261016/phase3-serial.py`. Each subprocess argument vector:

`/private/tmp/brn-widget-sol-20261016/instrumented-brn-desktop-phase3 <A-or-B-full-name> --exact --nocapture --test-threads=1`

Environment each: fresh per-run synthetic `TMPDIR=/private/tmp/brn-widget-sol-20261016/phase3-fixtures/<label>`, `SEED=0 ITERATIONS=1 RUST_BACKTRACE=1`. All diagnostic control vars removed before explicit case flags. Offline fixture provider with no auth/credentials; periodic UI poll disabled. Exact args/explicit environment, return code, wall time, callbacks and post-state are in `phase3-serial-results.json` and each complete `phase3-<label>.log`.

- A-disabled: PASS,0.9720s; no diagnostic flags.
- B-disabled: PASS,0.0782s; no diagnostic flags.
- A-unforced: PASS,0.1298s; DIAGNOSTICS=1 only.
- B-unforced: PASS,0.0751s; DIAGNOSTICS=1 only.
- A-pause80: FAIL,0.2859s; DIAGNOSTICS=1, PAUSE80_TARGET=discard-note-confirm; original edit_open assertion now line879.
- A-pause80-reduced: PASS,0.1818s; same plus REDUCE_MOTION=1.
- B-pause80: FAIL,0.1810s; DIAGNOSTICS=1, PAUSE80_TARGET=stop-saved-run-confirm; original Working-vs-Interrupted assertion now646.
- B-pause80-reduced: PASS,0.1839s; same plus REDUCE_MOTION=1.
- A-compare-pause80: FAIL,0.1844s; DIAGNOSTICS=1, PAUSE80_TARGET=compare-open-edit-close; original dialog-close assertion now835.
- A-compare-pause80-reduced: PASS,0.1811s; same plus REDUCE_MOTION=1.

Above short names are `BRN_WIDGET_DIAGNOSTICS`, `BRN_WIDGET_PAUSE80_TARGET`, `BRN_WIDGET_REDUCE_MOTION`. Pause occurs after pointer move dispatch/before that redraw, only on the selected target. Reduced motion uses public `App::set_reduce_motion(true)` in these tests' own contexts.

Controlled geometry: A confirm captured center y137.5, button top121.5; after80ms move redraw top174.5, point outside. B captured y205/top189; after pause top227, point outside. No confirm entry/return; dialogs remain open and store unchanged. Reduced controls keep the point inside, callback entry/return appears, A closes old session/creates own unsaved edit, B records Interrupted/paused queue; original assertions pass. Compare target likewise moves top249→302 and fails close; reduced counterpart passes. These controlled comparisons establish entrance motion as cause for their forced witnesses, not by themselves for the original failures.

## One bounded UNFORCED mixed batch

Actual launcher: `python3 /private/tmp/brn-widget-sol-20261016/phase3-unforced-stress.py 1`.

Eight concurrent subprocesses alternate A/B, same frozen verified binary and exact arguments above. Each uses a distinct fresh synthetic TMPDIR suffix `stress-1-<worker>`, `SEED=0 ITERATIONS=100 RUST_BACKTRACE=1 BRN_WIDGET_DIAGNOSTICS=1`; pause/reduced controls explicitly UNSET. Actual per-process args/env/start counts in `phase3-stress-1-results.json` and raw logs. No forced pause marker present in these runs.

Whole batch **9.915s**, **730 GPUI iterations started:728 passed/2 failed**. Fail-fast per process, not all configured iterations ran after a failure. Four B workers completed100 each (400 pass); A workers0/4 completed100 each, A workers2/6 completed64 and failed seed64 (328 A passes/2 failures). Two processes exit101, six exit0. Seeds are identifiers of observed runs, not deterministic repro guarantees.

Only ONE of maximum TWO batches used,100 maximum configured iterations/process,9.915s total stress (under120s). Stopped after useful unforced failures. No second batch or additional load/verification loop.

### Decisive unforced A original-symptom trace

Raw `phase3-stress-1-A-2.log`; extracted final iteration `phase3-stress-1-A-2-failing-iteration.txt`. Seed64, original edit_open Some-vs-None symptom at line879 (baseline846):

- Initial: captured bounds origin(758,121.5), size79×32, point(797.5,137.5); inside=true.
- Move-rendered/before-down: elapsed8.629ms, same bounds, inside=true.
- Down dispatched/last-render: elapsed57.726ms; then down-rendered/before-up elapsed67.403ms, current top162.5, captured point OUTSIDE.
- Mouse-up uses original point; up-dispatched remains outside; final render top165.5, outside.
- **No discard-note confirmation entry or return in this failing iteration** (earlier successful iterations have both). Immediate and after-parked dialog=true. External session.closed=false, own session=None, edit_open stillSome, saved note stillv1, notice remains original edit-open warning.

Thus the real pointer sequence missed confirmation as the dialog moved between down/up. The checked discard path did not run; this is not a refused discard or merely delayed UI clearing in this reproduced instance. It reproduces the original exact symptom without forced timing or unrelated suite tests. It does not retroactively prove every hosted failure has this cause.

### Decisive unforced Compare-close trace

Raw `phase3-stress-1-A-6.log`; extracted `phase3-stress-1-A-6-failing-iteration.txt`. Seed64: captured centery265 with top249; move-render before-down currenttop285 at60.458ms, captured point outside; before-up top288, outside; finaltop291, dialog remainsopen. Existing assertion835 (baseline823) fails. This explains the additional observed Compare-close miss via the same moving-target test interaction.

### B remains qualified uncertainty

No unforced B failure this phase:400/400 under mixed load, plus serial unforced pass. Forced80ms B produces the exact Working-vs-Interrupted state and reduced motion reverses it, demonstrating the same vulnerability in this test. Prior uninstrumented B failure is real, but its callback/geometry trace is absent. Do not claim its original cause confirmed or infer an asynchronous stop-service failure. No callback-entry-plus-wrong-state instance was observed in any probe.

## Evidence-supported next step, not implemented

Recommend a **test-specific public reduced-motion setup for these confirmation-behavior tests**, allocated/reviewed by principal: A/B context sets reduce motion before rendering their fixtures, retaining all original clicks, exact store/UI assertions, enter/cancel checks in relevant tests and real pointer dispatch. This controls irrelevant entrance animation while testing confirmation consequences. The deterministic forced witness should remain separate diagnostic evidence; never “fix” by skipping assertions, calling confirmation directly, retrying clicks, or generic time sleeps.

A's reproduced test-synchronization cause is supported by an UNFORCED failing trace plus causal motion controls. B fixture stabilization is justified as controlling the demonstrated same test vulnerability, but cannot be advertised as proof the original B failure is repaired. Validation after new allocation: new correctly sourced baseline artifact, exact A/B with original assertions, their bounded motion-control witnesses, then a principal-scheduled appropriate native-ui suite check. Any broad/load qualification is a new execution allocation, not started here.

**No production change recommended from this headless evidence.** There is no native human-interaction observation proving product confirmation/service behavior is broken. A product motion/hit-target change requires separate scope/evidence; B1a remains principal-held.

## Limitations and handback

Debug1 differs from original artifact; trace I/O perturbs timing. Public snapshot bounds/visibility/disabled do not reveal actual hitbox/clip/occlusion. However real input dispatch plus absent callback, unchanged state and controlled motion comparison strongly discriminate the captured A miss from a service refusal. No deterministic seed-only reproduction claim. The80ms control is diagnostic, not a regression fix.

Execution phase total:10 serial GPUI iterations (7pass/3forcedfail),730 stress iterations (728pass/2unforcedfail), **740 started,735pass/5fail**. Counts exclude rejected build/cache receipts. No whole-suite run performed this phase.

All Cargo and test subprocess sessions completed; final precise process check `phase3-final-process-check.json` is empty. **Heavy slot explicitly released to principal** before writing this final report. No further builds/tests/stress. Final sourceHEAD unchanged, only three allocated WIP files modified. No repair implemented, no native/owner acceptance or merge claim.

Raw proofs retained unchanged under this directory. Principal reports decisive receipts copied to durable root `widget-phase3/`; this scratch report does not itself alter repository docs or that durable copy.
