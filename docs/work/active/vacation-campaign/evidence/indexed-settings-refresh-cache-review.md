# Independent diagnostic cache-correction review

## Disposition

**Approve the one selected corrected invocation using the exact inert adapter below, after the lead confirms exclusive ownership of the shared build slot. No actionable findings.** This is a diagnosed source/artifact-identity correction following a rejected zero-test attempt, not permission to repeat until a preferred timing appears. If the corrected invocation still selects the wrong/zero test, stop and reassess. No corrected workload was executed by this review.

Diagnostic source approval remains exact `88c61df18cf6d09e7efc3a2d2349484645b2378a`, parent product `eae61dde04cda19c764246993966b13b997014eb`. Current fixture bytes still hash to the approved `832d9e68810d6f52ea5eda317e301b8b574c04516a7ba6d16659339fb89a40ba`. The prior complete source/product/adapter review at `/tmp/brn-indexed-settings-refresh-review.md` is explicitly reused for unchanged source and guards. I authored neither source nor adapter.

## Actual rejected attempt

Read the complete original log and receipt under `/tmp/brn-indexed-settings-refresh-qualification/`, plus the lead's retained cache-correction memo. The log records the expected source pin, no compilation, test-profile completion in 0.84 s, and **0 tests / 348 filtered out**, with no diagnostic marker. There is no elapsed-refresh measurement. Child exit 0 denotes a successful empty harness invocation, not successful qualification.

The receipt correctly records `sample_lines: 0`, `one_test_passed: false`, and `qualified: false`. It records unchanged source checkout, leader reaped, process group absent/resolved, no cleanup signals or errors, and 5.07 s total supervisor duration. The supervisor's reviewed exit expression returns 1 for this failed log gate, consistent with the lead's reported supervisor exit. No source-refresh defect, successful changed-source diagnostic, or performance result follows from this attempt.

Preservation verified independently: lead evidence `indexed-settings-refresh-initial.log` and `indexed-settings-refresh-initial-completion.json` have the exact same hashes as the originals:

- log: `28e65907259ed1bb2ee3faa210a2a86bca78a390cdcb4cc6eb37bc0bda518481`
- completion: `754895e3ac0b0e6353ca3c8e90c7027a40a7eb02d8b55e8d65ff011a72718cb1`

The immutable source contains the exact named fixture, previously byte-verified, while the selected executable lacks it and was not rebuilt. In the documented shared-target sequence, this supports the lead's stale product-desktop artifact diagnosis. The precise old source revision embedded in that executable is not independently proven by its test count alone. The important observed failure is executable/source identity, despite passing HEAD/fixture guards; it is precisely why the verification guide distinguishes source identity from cached binary identity.

## Exact corrected adapter

Read the entire corrected runner and supervisor; independently compared them with the prior approved files:

- `/tmp/brn-indexed-settings-refresh-retry-qualification/run.sh`: SHA-256 `2c861cb0e1808c5c5e59a38ee05f4a88bea27594a3a4836b5554cf4ed14e2353`
- `/tmp/brn-indexed-settings-refresh-retry-qualification/supervise.py`: SHA-256 `7caae4f96538449dd7293c093160655873c12fd0f27966b82815d6ed4f37c918`

Verified exact selected delta: runner changes only task TMPDIR and inserts `cargo clean -p brn-desktop --locked --offline` immediately after unchanged source/disk preflight and before the unchanged exact one-test command. Supervisor changes only its evidence root. HEAD, fixture hash, test filter, features, profile, algorithms, and acceptance criteria are unchanged. Fresh evidence/temp paths prevent overwriting the initial result.

The package-specific clean is appropriately scoped to mutable desktop package outputs/fingerprints in the supplied shared Cargo target. It is not a whole-target or dependency/helper clean, source edit, data repair, or frozen app-bundle modification. It may retire the mutable ordinary desktop executable from that target; the earlier ordinary native build remains a historical passing check, not a frozen launch artifact that this path promises to preserve. Exclusive build-slot ownership is essential while cleaning and rebuilding. The runner itself does not arbitrate competing writers; the selected lead gate must do so.

The pinned/offline/jobs1 environment, clean source and fixture guards, >3 GiB initial floor, 30-minute trigger/1 GiB sampled stop, owned group TERM/KILL/reap/final-resolution handling, post-run checkout verification, <=8 MiB accepted log, exactly one diagnostic marker, actual one-passed/zero-failed summary, and receipt-persistence failure handling all remain intact. No source overlay/restoration or automatic retry loop is added. Resource/log limits remain sampled/accepted-size bounds, not hard OS quotas. Existing approved core/app source identity is reusable; no need to clean unrelated packages merely because desktop lacked its test.

## Final evidence required and limits

The corrected log must show actual desktop compilation and the exact named test executing, its single diagnostic line/dimensions, and one pass. The unchanged machine acceptance gate checks the marker and one-pass result; the lead must also inspect the compilation evidence required by the correction memo. The newly inserted clean gives the concrete invalidation step; its success alone is not a refresh sample. Retain any failure rather than relaxing these gates or adding another blind attempt.

A successful corrected run would be the one changed-source measurement; the first attempt contributed zero measurements. The original baseline remains one 80.819 ms sample on main208-based `be97dc0a`, while this source is main209-based `88c61df1` with Settings reuse. The previously recorded nonidentical-base, test-opt1, warm direct-method, excluded render/setup, possible detached-search activity, no native/startup/full-app, and no causal/statistical speedup limits still apply.

This review read existing files and compared bytes/hashes only. It ran no Git commands, clean, build, test, workload, fixture, native/account/network action, source mutation, or nested helper. The sole write is this report. Corrected execution and its result remain pending.
