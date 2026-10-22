# `writing_setup` crash triage

## Observed result

The final qualification for candidate `ccd05b28ea7e1d7bcd92fe266f6e37aa23461d1b` exited 101 after 25.11 seconds and is correctly marked unqualified in `/tmp/brn-recovery-copy-final-qualification/completion.json`. The rebuild-identity and current-input gates passed; the latter reports 9/9. The app integration run passed its 66 reported tests, including the three recovery-copy service cases. The next binary, `writing_setup-0a70c929436e2f9d`, announced 22 tests and then died with signal 11 before any per-test result or suite summary (`run.log:133-139`). Desktop qualification was not reached. Cleanup reports the owned process group absent, leader reaped, checkout unchanged, and no cleanup errors.

## Crash evidence

The matching task-owned report is `/Users/evokessler/Library/Logs/DiagnosticReports/writing_setup-0a70c929436e2f9d-2026-10-22-100630.ips`, incident `8673BD66-413E-4A27-925B-E723AAD47F18`. It identifies an arm64 `writing_setup` process (PID 75727; launch 10:06:25.653 +0300), `EXC_BAD_ACCESS` / `SIGSEGV`, null invalid address, faulting thread index 5 / ID 18715774. Its stack is `sqlite3_initialize` → `sqlite3_mutex_alloc` → `rusqlite::InnerConnection::open_with_flags` → `Connection::open_with_flags` → `brn_threads_core::Store::open_with` → `Workspace::open` → test `fixture` → `first_capture_undo_archives_exact_records_after_conversation_inactivation_or_move`.

The same crash snapshot captures other Rust test workers concurrently opening their own fixtures through the same SQLite initialization path. One is blocked in `sqlite3_initialize`'s mutex wait; another has its program counter in `sqlite3MemInit` under `sqlite3_initialize`. This makes a process-global SQLite initialization race or initialization-state corruption during parallel test startup a plausible explanation. It is not proof of the root cause: the report is a single crash snapshot, and it does not establish why SQLite's initializer reached a null address.

## Source and artifact assessment

`crates/brn-threads-app/tests/writing_setup.rs:4-6` creates a fresh canonical-temp-directory fixture for each test and opens a separate workspace. There are 22 tests in this binary. Thus the observed parallel opens do not, by themselves, indicate shared fixture/database paths. The caller chain reaches `Store::open_with` (`crates/brn-threads-core/src/store.rs:67`, SQLite connection opening at lines 143-150 for existing stores and 151 onward for new stores). Core pins `rusqlite = 0.40.2` with `bundled`; `Cargo.lock` pins `libsqlite3-sys 0.38.2`. No project-level `sqlite3_config` or `SQLITE_THREADSAFE` override was found in the inspected core source. I did not inspect or infer the actual bundled compiler flags, nor prove that concurrent fixture creation is a supported/unsupported SQLite configuration.

The logged test executable path and UUID match the crash report's executable identity (`writing_setup-0a70c929436e2f9d`, UUID `79f53e61-a47d-382f-b602-4d3046124a83`). This ties the crash report to the attempted binary, but does not independently prove which source revision was compiled into it. The completion receipt says the app package was compiled during this run, `checkout_unchanged:true`, and the crash executable was invoked from the task checkout's `target/debug/deps`; there is no positive evidence here of a stale cached test artifact.

## One discriminating next step

If the lead chooses to spend another bounded execution, run this same already-built `writing_setup` executable once with the Rust test harness serialized (`--test-threads=1`) under the existing owned-process supervisor, after verifying the path and UUID still match this receipt. A serial pass would support (but not prove) a parallel-initialization interaction; a serial crash would shift attention toward deterministic SQLite initialization/build/runtime behavior. This triage did not execute that diagnostic or make source changes.

## Limits

No tests, builds, workload, or source changes were performed for this triage. The candidate remains unqualified because the required app gate crashed and desktop checks did not run; passing earlier gates do not substitute for that gate. No conclusion is made about native acceptance, release readiness, or whether a production change is warranted.
