# Independent export publication durability review

## Reviewed identity and scope

- Repository: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`
- Branch: `codex/export-publication-durability`
- Verified merge base: `b0eb18d7db7560269fa18d96c5bbf92eee1f40da`
- Candidate: `0f64a0344eac23edb43403034bf3e0deae87655c`
- Candidate tree: `772e24f76b1f3e803dc031dde26c7539685c09ff`
- Both initial and final inspected working snapshots are clean: no staged, unstaged, or non-ignored untracked files. The complete candidate is the committed diff: two files, 101 added lines (`crates/brn-threads-app/src/export.rs`, 98; `crates/brn-threads-app/README.md`, 3).
- Review was read-only, except for this report outside the repository. No build/test/app/native/account/credential operation, Git mutation, or helper delegation was performed.

## Findings

No actionable correctness, requirements, or standards findings in this exact candidate. No severity-assigned findings remain.

## Assessment

The change belongs at the existing shared export publisher. `Workspace::export_now` still delegates directly to it (`crates/brn-threads-app/src/lib.rs:583-584`); desktop and CLI continue to consume that same checked application operation. There is no second state owner, new dependency, schema/data migration, change to exact exported bytes/provenance, altered Undo behavior, or bypass of publication exclusivity.

The complete publisher was reviewed, including eligibility, asset rewriting, manifest hashing, verification, file synchronization, staging and exclusive rename. At `export.rs:117-125`, all existing output files and the manifest are synchronized first, then `assets/`, then the staged root, followed by exclusive rename and synchronization of the destination parent. Each new failure propagates through the existing typed I/O error conversion. This matches the accepted bounded Unix directory-flush outcome and the facility already used by backup/restore (`brn-threads-core/src/backup.rs:122-124`, `restore.rs:109-124`). The `cfg(not(unix))` branch remains a no-op, preserving existing non-Unix production behavior.

Before rename, `?` leaves the final destination absent and drops the existing `TempDir`, which attempts cleanup of only its staging path. After successful rename, the `TempDir` retains its old staging pathname; a parent-flush error therefore does not target or remove the published destination. The pinned tempfile 3.27.0 destructor was inspected to confirm pathname-based cleanup. Cleanup remains best effort under arbitrary additional filesystem failures, as before. Existing macOS/Linux exclusive rename and collision handling are unchanged.

The desktop worker only constructs its success message after `export_now` returns `Ok` (`crates/brn-desktop/src/threads_ui.rs:3324-3343`), and the CLI propagates the error (`crates/brn/src/main.rs:1685-1688`). Thus a final parent-flush error remains a reported failure despite a complete snapshot being visible, as expressly required. The README at `crates/brn-threads-app/README.md:82-84` accurately describes that distinction.

## Verification evidence and test effectiveness

The supplied logs were inspected, not rerun by this reviewer:

- `/tmp/brn-export-durability-lib-tests.log`: two new failure-injection tests passed.
- `/tmp/brn-export-durability-product-export.log`: three existing export tests passed, covering failed/incomplete output cleanup, occupied-path preservation and exact asset identity boundaries.
- `/tmp/brn-export-durability-product-roundtrip.log`: the existing full-import/export/backup/restore witness passed, including retained asset bytes and equal restored export manifests.
- `/tmp/brn-export-durability-clippy.log`: the all-target application lint run completed successfully and identifies the reviewed checkout in its crate paths. Exact lint flags, formatting and diff-check success were supplied by the lead; they were not independently executed here.

The new tests use a private effect seam without exposing a new public API. They establish that failure at the staged-root boundary prevents publication and cleans the stage, and that failure at the third boundary reports error while preserving a readable manifest/note and the assets directory after publication (`export.rs:236-306`). These are meaningful deterministic error/lifecycle witnesses. The existing roundtrip exercises the real production directory-sync helper and retained assets on the tested Mac filesystem.

Limits: the injected closures count calls rather than asserting their paths, and the pre-publication case targets the second flush rather than independently injecting the first. They do not themselves prove correct syscall paths, all directory orderings, preservation of nonempty assets on the post-publication error path, or resistance to arbitrary filesystem failures. The reviewed source supplies the required three paths and common error propagation; these limits do not expose a current defect or require additional scope. No original power-loss failure was reproduced, and these tests are not power-loss/hardware durability proof or native usability evidence. Linux and non-Unix execution were not independently observed.

The test logs preserve the nonfatal `rust-objcopy` SIGABRT caused by missing `libLLVM.dylib`; the test commands nevertheless completed with passing test summaries. This warning is not represented as a test failure or silently omitted. The earlier `/tmp/brn-backup-failure-audit.md` remains source-inspection evidence only.

## Gate and ownership limits

This review satisfies the independent source-review assessment for the exact clean candidate above. It does not establish hosted required checks, normal merge, resulting-main checks, native observation, owner acceptance, release or a power-loss guarantee. The lead retains integration, campaign records and all subsequent gate decisions. A material candidate change requires reassessing affected review evidence.
