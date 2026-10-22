# Independent desktop refresh diagnostic result review

## Disposition

Qualified for the selected narrow diagnostic: one successful warm, headless direct `ThreadsUi::refresh()` measurement, **80,819 µs (80.819 ms)**, on frozen candidate `be97dc0aeb4a441e12a4bb8193055eb6ece64024`, based on `11c48f9b91cb0d4e9deba9e0f45b43ef284b63e1`. No actionable defect in the saved result or its stated scope was found. This establishes neither a latency acceptance target nor a production performance failure, native usability, startup/render latency, owner acceptance, or latest-main qualification. The diagnostic branch remains evidence rather than a selected product merge.

I independently reviewed the saved log, receipt, unchanged approved runner/supervisor, current fixture bytes, relevant refresh/render/Settings source, and corrected read-amplification audit. I did not execute a workload, tests, builds, Git commands, native/account/network actions, or modify repository source. Prior complete source and runner reviews are explicitly reused; this is a result assessment, not another execution.

## Identity and actual result

SHA-256 identities read during this assessment:

| Artifact | SHA-256 |
| --- | --- |
| `run.log` | `c2103aaff417ea12a0ad5126f58bd7911212162578bf8e6591b88d4ffa52ab9e` |
| `completion.json` | `afff083abb67db4f142d7a4aa8d63c8c70a2e45ee40c96c1b93222af748d561c` |
| `run.sh` | `09af74ac97b6a4d8e249f2e15e13754825543898cf857ec3c81deedc3d67d2b4` |
| `supervise.py` | `5115b3d4fa6831f15e2fa63106893e8c30a2ba75939a2416f4b63efc4deb88e4` |
| `threads_omakase_tests.rs` | `832d9e68810d6f52ea5eda317e301b8b574c04516a7ba6d16659339fb89a40ba` |
| corrected read-amplification audit | `110514240cb50de9d944f32082a8876988ca6d9bfccc906f89a3600eae589f6c` |

The fixture hash matches the independently reviewed complete final candidate. The log pins that candidate and base, compiles the intended desktop crate, and contains exactly one diagnostic line and actual harness result: **1 passed, 0 failed, 348 filtered out**, test duration 0.91 s. Compilation took about 102 s; the supervisor's 105.27 s elapsed duration is not the measured refresh latency.

The diagnostic reports Notes 10,000, records 10,002, receipts 23; candidates, recovery buffers, assets, writing requests, and applied rewrites are all zero; Topics is unconfigured and the request/Apply path is explicitly unactivated. Source constructs 20 checked owner batches of 500 fixed synthetic Notes. These dimensions delimit coverage; message-heavy history, configured Topics, candidate/buffer/asset workloads, provider work, and request-linked Apply projection are unqualified.

The timer begins immediately before `view.refresh()` and stops immediately after it. Fixture creation, constructor loading, surrounding update/render/park work, assertions, and diagnostic printing are excluded. The previously accepted correction requires empty notice text and a non-error tone before and after the call, outside the interval; a query failure reported through `fail()` cannot silently qualify merely because constructor-populated records remain. This closes the original cached-state false-sample concern.

This is the test opt-level 1/debug-assertion profile using headless GPUI support, not a release-native sample. Existing setup may start detached search preparation; concurrent CPU/I/O from it was not controlled or attributed. One sample supplies no distribution, regression trend, isolated query breakdown, or comparison against a selected user-facing budget. In particular, unrelated retrieval reopen gates do not apply to this timer.

## Completion and preservation

The completion receipt records exit 0, no stop reason or exception, one validated diagnostic line, and `checkout_unchanged: true` for the pinned candidate. It records the owned leader reaped and process group absent/resolved, with no cleanup signals or errors. Disk free at completion was 16,774,578,176 bytes. This is consistent with ordinary successful completion under the reviewed bounds.

Post-run HEAD/clean-checkout verification is evidence recorded by the approved adapter; I did not independently invoke Git during this result review. Current fixture hashing independently confirms the reviewed fixture bytes. Normal completion does not experimentally exercise timeout, signal, or persistence-failure branches; their prior source review is not recast as runtime proof. No fixture repair, rerun, or native/account action was performed for this assessment.

## Audit correction and source equality

The initial preflight/audit claim that `refresh_writing_review_cache` has no caller was incorrect. `ThreadsUi::render` calls it at `crates/brn-desktop/src/threads_shell.rs:1642`. An unchanged key returns without a canonical reload; a miss clears stale cache data and uses `Workspace::writing_context_review`, which reads canonical records, receipts, and candidates and rechecks its change token. With no displayed Note the helper can also clear and return early. Render/setup/park can therefore execute this helper. Its call is outside the direct-refresh interval, so correcting the call graph does not invalidate the measured scope or convert it into a whole-render measurement.

The original reports remain retained; the corrected audit explicitly preserves its initial version. Because the correction was made while inspecting the lead checkout rather than the frozen worker, I independently compared whole-file hashes between those checkouts for `threads_ui.rs`, `threads_shell.rs`, app `settings.rs`, and app `review.rs`: each pair is identical. The key identities are respectively `0af8ff30701993fe21a71beb7f64b164d64c0409630cb419037b2aabc60b1b88`, `d2afc2a7e1e7b4751341b3ce593114203e45635be9f0a261f6e1caa056e3c0db`, `2d4f47394891b2f50aecaa64790ccb545aa192e48be1d76a28f8a4bf58153576`, and `6813e0b4427a19afaa3657d8347299c4d682e2aa28e68c20c848db2a7ceae681`. The corrected render/Settings reasoning therefore applies to the diagnostic source, without claiming whole-checkout identity.

## Follow-up judgment

Source establishes three broad record materializations on this unconfigured-Topics path: direct records, Settings via `Workspace::settings()`, and Settings again through `topics()`. The sample does not instrument their individual time or prove which dominates 80.819 ms. Configured Topics adds a separate shared visibility-filtered record path. Neither a broad cache/snapshot framework nor copying Settings/visibility policy into desktop is justified by this result.

The lead's subsequently identified **existing core seam** is materially better than a desktop selector: `store.rs:3072` already has `workspace_settings_on(&Connection)`, selecting an active Settings record with the existing JSON predicate and typed Record decoding. The schema's `one_workspace_settings` unique partial index (`store.rs:163`) prevents multiple Settings records in a supported store. Existing mutation checks already reuse this helper. A small read-only Store wrapper, with `Workspace::settings()` delegating and retaining its missing-configuration AppError, is a reasonable optional maintenance candidate: it reuses an owned rule and removes two full-record Rust materializations from this fixture's refresh path, benefiting other Settings readers too. It requires no desktop policy duplication, authority grant, schema/index change, or new cache lifecycle. This is source-backed structural improvement, not a promised speedup or a mandatory optimization target.

A candidate would still need complete review and ordinary focused qualification: preserve active/archived/missing Settings behavior, propagate selected-row/SQL errors, retain the app's missing-configuration message, and keep the writer helper unchanged. Avoid describing narrow Settings lookup as a whole-store corruption scan: unrelated records are no longer all deserialized by that accessor. SQLite query-plan cost is also unmeasured here; the structural claim is reduced full-record materialization, not a proved constant-time query. `Workspace::settings()` should remain a fresh Store read rather than introducing a snapshot authority contract. Any comparative timing would need a separately selected bounded run and honest single-sample limits. No new implementation or workload is authorized by this review itself.
