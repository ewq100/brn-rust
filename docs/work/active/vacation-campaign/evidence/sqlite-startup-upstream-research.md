# Upstream SQLite cold-start research

## Finding

I found **no primary-source report that identifies the same cold, concurrent `sqlite3_initialize()` / `xMutexInit` callback-publication failure in SQLite 3.53.2**. This is a bounded source search, not proof that no issue exists. The local crash’s fault at `sqlite3_initialize` and the `xMutexInit` slot remains local evidence only; it does not establish an upstream defect.

SQLite’s current API documentation states that `sqlite3_initialize()` is threadsafe and that `sqlite3_open()` invokes it automatically. Its mutex-method documentation says `xMutexInit` is called during initialization, must itself be threadsafe, and must be safe as a no-op on repeated calls. Those requirements make a failure there noteworthy, but the contract pages do not report a known callback-publication bug. ([Initialize The SQLite Library](https://sqlite.org/c3ref/initialize.html), [Mutex Methods Object](https://sqlite.org/c3ref/mutex_methods.html))

The official 3.53.2 release entry confirms the exact local source ID, `2026-06-03 ... d6e03d8c...`, and describes the patch release as fixes for user-reported 3.53.0 problems. It does not identify an `sqlite3_initialize`, mutex-method-table, or `xMutexInit` publication correction. The changelog is not an exhaustive issue index, so this absence is limited evidence. ([SQLite Release 3.53.2](https://www2.sqlite.org/releaselog/3_53_2.html))

One adjacent upstream report is rusqlite issue #1860, opened June 20, 2026: under sustained concurrent connection open/close, the reporter describes a **deadlock in Unix VFS mutexes**, with bundled SQLite 3.51.1 / rusqlite 0.38.0 / libsqlite3-sys 0.36.0 on Linux x86_64. The report explicitly says it is investigative and lacks a minimal standalone reproduction. That is different from the local macOS arm64 null indirect call in `sqlite3_initialize` at `xMutexInit`, and its dependency/SQLite versions do not match local rusqlite 0.40.2 / libsqlite3-sys 0.38.2 / SQLite 3.53.2. It supports only that maintainers have received a separate report involving concurrent SQLite connection initialization/use; it does not confirm this failure or justify a BRN lock/workaround. ([rusqlite issue #1860](https://github.com/rusqlite/rusqlite/issues/1860))

## Scope and limits

The local crash details and exact dependency/source identity above come from `/tmp/brn-writing-setup-crash-assessment.md`, not from upstream. I consulted three primary source pages only; the SQLite forum link mentioned in #1860 was inaccessible in this lookup and was not used as evidence. I made no source or dependency changes and ran no code, tests, builds, or native work.
