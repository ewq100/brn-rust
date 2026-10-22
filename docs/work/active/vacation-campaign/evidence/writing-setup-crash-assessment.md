# Independent writing_setup crash assessment

## Disposition and one next gate

**Approve one serialized diagnostic invocation of the exact existing test binary using the exact adapters below.** It is a bounded discriminator, not a repair, substitute for the failed parallel qualification, or permission for a retry loop. Keep product candidate `ccd05b28ea7e1d7bcd92fe266f6e37aa23461d1b` unqualified while this crash and the unreached gates remain unresolved.

The crash evidence supports an initialization/callback-publication interaction more specifically than a generic database-open failure. It does not justify assuming that parallel fixture creation is invalid, installing a process-global mutex, serializing all production access, changing SQLite configuration, rebuilding dependencies, or removing a required check. A serial pass would support a concurrency/timing-sensitive explanation without proving one; a serial crash would refute the simple explanation that concurrent Rust test startup alone is necessary. Either result requires a new explicit diagnosis/disposition before further work.

## Actual failed gate and identity

Independently read `/tmp/brn-writing-setup-crash-triage.md`, the full qualification receipt and relevant original log, and only the named task-related crash report. No other application reports/private data were inspected.

The qualification at final `ccd05b28` correctly ended unqualified with child 101 after 25.11 s. Rebuild identity and core Current-input stages passed; core reports 9/9. Seven completed app binaries report **66 passing tests total**, including all three new recovery-copy service cases. The next existing `writing_setup` binary announced 22 tests and died with SIGSEGV before a per-test result/summary. The app stage did not pass, and desktop/lint/native-build/fmt stages were not reached. The receipt records unchanged checkout, owned leader reaped/group absent, and no cleanup errors. Passing earlier service tests do not replace the crashed suite or unreached UI evidence.

Saved evidence SHA-256:

- qualification `run.log`: `41895927a7c935c80e1e7b2f975c8c16a4192f162d8ef0cd3368e4101b9dda6b`
- qualification `completion.json`: `be864265329d55eb3b700ea4cb89307e1964bc225744e07f692d0e3bdb779a9d`
- named `.ips` report: `d68a62171597110ca7aa0acf180148bd83c1f1ad6d30018b2e73bbdac4c5998d`

Crash incident `8673BD66-413E-4A27-925B-E723AAD47F18` is the arm64 `writing_setup-0a70c929436e2f9d` process, PID 75727. Independently hashed the existing executable at `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/target/debug/deps/writing_setup-0a70c929436e2f9d`: **7,637,088 bytes**, SHA-256 `cd20fe93330b7b00eb3e9a226421f451e46620c53a308c4106eecafb7e7e2ca4`. Static UUID inspection returns `79F53E61-A47D-382F-B602-4D3046124A83`, matching the crash image. Static inspection did not execute the test binary. The original app package compilation is visible in the log. Artifact/report matching is stronger than a source pin alone, but is not a complete reconstruction of every historical linker input.

## More concrete fault localization

Faulting thread 5 is in the fixture for `first_capture_undo_archives_exact_records_after_conversation_inactivation_or_move`, through Workspace::open → Store::open_with → rusqlite open. Its fault PC is **0**, x8 is **0**, and LR points to `sqlite3_initialize + 276`. Static disassembly of this exact binary places an indirect `blr x8` immediately before that return location, loading x8 from `sqlite3Config + 0x60`. The pinned bundled source's `Sqlite3Config` layout and `sqlite3_mutex_methods` map this slot to **mutex.xMutexInit**. Thus the evidence identifies a null indirect mutex-initialization callback before opening the fixture database, rather than a failed SQL assertion or recovery-copy request.

Other crash threads are concurrently initializing SQLite through separate test fixtures: two are waiting on initialization's pthread mutex and one is at `sqlite3MemInit`. Each fixture uses a fresh canonical `TempDir` and independent database; this is not evidence of a shared fixture path or simultaneous use of one rusqlite Connection. The failure occurs before these tests' later writing/setup mutations, so it is not evidence that the new recovery-copy logic executed in the crashing thread.

The bundled amalgamation is **SQLite 3.53.2**, source ID `2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24`, supplied by **libsqlite3-sys 0.38.2**. These version numbers must not be conflated. Its `sqlite3MutexInit` (`sqlite3.c:30214–30253`) initializes the default mutex-method table when xMutexAlloc is unset, stores xMutexInit and other fields, uses a memory barrier before publishing xMutexAlloc, then invokes xMutexInit. The observed null callback is consistent with an initialization/table-publication or corrupted-state interaction at that boundary. The snapshot/disassembly does not prove the publication history, compiler/runtime ordering defect, external corruption, or precise upstream cause. Do not promote that hypothesis into an established SQLite bug from one crash.

## Configuration/linkage assessment

Core and retrieval pin the same `rusqlite 0.40.2` with bundled SQLite. The lock contains one libsqlite3-sys version; that crate declares `links="sqlite3"`. On non-wasm targets, rusqlite's `ensure_safe_sqlite_threading_mode` checks `sqlite3_threadsafe()` then allocates/frees a SQLite mutex on every connection open. It relies on SQLite initialization safety; it does not wrap initialization in a Rust Once. The default `NO_MUTEX` connection flag concerns use of an individual Connection, not a prohibition on opening distinct connections from different threads.

The bundled build source sets `SQLITE_THREADSAFE=1`; its shown `=0` branch is wasm32-WASI-specific and does not apply to arm64 Darwin. Saved nearby build outputs show static sqlite3 linkage and no CFLAGS values; available libsqlite3-sys fingerprints record bundled/default features, not SQLCipher. These existing shared-target files are contextual evidence, not proof that every one was linked into this particular executable. No repository sqlite3_config/initialize/shutdown or THREADSAFE override was found in the searched Rust/manifests. Shutdown/concurrent reconfiguration therefore has no identified BRN caller.

Static `nm` finds one visible `_sqlite3_initialize`, `_sqlite3_config`, and `_sqlite3Config`; SQLite frames reside in the executable image. `otool -L` and the crash image inventory do not show a dynamic libsqlite3. No positive duplicate-SQLite-symbol/build-scope linkage evidence was found. This does not exhaustively rule out every hidden/renamed static implementation, but supplies no reason to prioritize that theory over the observed null initializer callback.

The bundled initializer's own source contract (`sqlite3.c:187373` onward) explicitly requires another thread to wait while first-thread initialization completes, and the source implements internal initialization mutexes. Parent-provided official SQLite references also distinguish thread-safe initialization from non-thread-safe shutdown. I did not browse them; local pinned implementation/contract was inspected. Serial execution is therefore an investigative perturbation of supported concurrency, not a correction of invalid parallel tests.

## Exact serialized adapter assessment

Read complete inert files:

- `/tmp/brn-writing-setup-serial-diagnostic/run.sh`: SHA-256 `34e1e458eb32e49d8c8de2b3361f6c3c388f3dcd796d29b826f80a7d647a9ec2`
- `/tmp/brn-writing-setup-serial-diagnostic/supervise.py`: SHA-256 `494a4367b03086c6d27aa3d27098f61881b1b70c40f49360d1cf8a1c9b7276e1`

No concrete blocking issue found for this single diagnostic. It pins final clean source, exact executable SHA/UUID, and >3 GiB initial free disk; invokes only that existing binary with `--test-threads=1`, no Cargo/rebuild/package clean or source writes; and uses an explicit synthetic temp parent. The existing tests create fresh separate fixtures. Workspace bootstrap may record default credential *locations* but does not read credentials or dispatch a provider; the inspected suite is deterministic shared-service work. No native app/model/account action is selected.

The reused supervisor owns a new process group, samples a **180-second** deadline and 1 GiB floor, retains bounded whole-group TERM/KILL plus independent reap/final absence handling, records operational errors, verifies unchanged checkout, caps accepted log read at 8 MiB, and requires the exact **22 passed/0 failed/0 ignored/0 filtered** summary plus zero child exit for `diagnostic_passed`. `qualified` is always false. Persistence failure remains a failing invocation; use durable raw log/completion rather than a fallback printed observation when receipt writing fails. Limits are sampled/accepted bounds, not hard OS quotas.

The lead must preserve sole build-slot ownership so the verified mutable executable is not replaced between guard and invocation. The proposed run has no automatic repetition. Preserve a pass or crash exactly; do not reuse a serial pass as a green replacement for the original parallel app gate, change global test-thread settings, or proceed to integration on it. The next decision belongs to the result assessment: concurrency-sensitive evidence after a pass, or deterministic/build/runtime evidence after another crash. No broader mutation is selected by this approval.

Reviewer performed read-only source/artifact/log/hash/static-symbol/disassembly inspection only. No tests/builds/workloads, source/Git edits, SQLite calls, signals, account/native app/network actions, or helpers were executed. This report is the sole write.
