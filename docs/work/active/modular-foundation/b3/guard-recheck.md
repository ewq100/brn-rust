# B3 bounded integration-guard recheck

Source pin `def881852d1f3aa24dca03f0f2bb52145b37faf6`, after the corrected normal gates. [Complete actual commands, material maps, binary/dep-info/log hashes, patches, assertions and restoration/process receipts](guard-recheck.json). Original B3 mutation allocation began at zero; no B1b/D1B budget was reused.

Fixed disposable source `/private/tmp/brn-b3-guard-20261017-source`, distinct fresh target `/private/tmp/brn-b3-guard-20261017-target`, synthetic Git/no remotes. Tree matched the source pin and all 328 material inputs were checked before passing baselines, before each variant and after restoration. Candidate/before targets were never used by this copy. Rust 1.98.1; locked/offline; CARGO_INCREMENTAL=0; fresh per-command synthetic TMPDIR; no candidate helper executable inherited. No automatic mutation tool, model assets or provider ran.

| Command | Actual outcome |
|---|---|
| Unmutated app import_coverage receipt/scope fixture | 1 passed / 7 filtered; 37.510 seconds |
| Unmutated desktop native import/wrong-target fixture | 1 passed / 286 filtered; 78.750 seconds |
| receipt-payload-binding: remove only persisted receipt equality filter | Compiled, 0 passed / 1 failed / 7 filtered; 4.522 seconds; CAUGHT at same-visible-scope forged-payload unwrap_err on Ok |
| final-display-target: pass captured target instead of displayed Note/scope to final validation | Compiled, 0 passed / 1 failed / 286 filtered; 18.925 seconds; CAUGHT at existing no-attachment notice assertion after navigation |

The second raw failure is `assertion failed: view.notice.contains("retained in its original scope; no reference was attached")` at threads_writing_tests.rs:2183. The controller mistakenly expected the later equality assertion and retained NO VERDICT/error. Raw successful compilation and the earlier intended rejection assertion were audited separately without rerunning, changing source, or adding a variant. The later reference-list assertions did not execute; this is not evidence that they passed or an assertion of unauthorized writes.

Only the two allocated lines/hunks were seeded. Each source file was restored to its exact pin bytes; all 328 material hashes then matched candidate and copy. Copy Git status is empty; synthetic remotes empty; own children reaped; precise cargo/rustc/rustdoc/cargo-mutants receipt empty. Mutated binaries/logs remain isolated in their disposable target/evidence and are not candidate artifacts.

Controller/setup/commands/restore/process receipt 170.403376 seconds; final source/process/log audit 202.378745; cumulative allocation closed after receipt cleanup at **217.265250/600 seconds**. Evidence root `/private/tmp/brn-b3-guard-20261017-evidence` retains original controller receipt/error, run.py, source archive hash, raw logs, exact patches, final audit and budget close. No extra ID, extension, broad campaign, stress or rerun. Heavy slot released; source remains frozen for principal review. These two witnesses do not establish full F21 or hosted/native-human/owner acceptance.
