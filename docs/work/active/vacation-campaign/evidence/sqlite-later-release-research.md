# SQLite startup crash: upstream evidence check

**Result: no concrete upstream diagnosis or fix matching the retained crash was found.** The existing, committed [`sqlite-startup-upstream-research.md`](sqlite-startup-upstream-research.md) already answers the exact-version question from SQLite’s initialization and mutex contracts, SQLite 3.53.2 release entry, and the distinct rusqlite issue #1860. I read that report first and did not repeat its source-page checks.

The local incident is a macOS arm64 null indirect call at `sqlite3_initialize`/`sqlite3_mutex_alloc` while `rusqlite::InnerConnection::open_with_flags` opens independent test fixtures. The pinned local dependency set is bundled SQLite 3.53.2 (source ID `2026-06-03 ... d6e03d8c…`), rusqlite 0.40.2, and libsqlite3-sys 0.38.2. The crash and those version/build identities are local evidence, not an upstream diagnosis. The task remains blocked/deferred after the original crash and two bounded non-reproductions; the current task record says to stop cold-process loops.

I made one supplemental, bounded check of later official SQLite release notes because they postdate the 3.53.2 source. The 3.53.3 and 3.53.4 entries describe broad fixes but do not identify this initializer callback fault. SQLite 3.54.0 (2026-10-09) mentions replacing non-recursive mutexes with Windows Slim Reader/Writer Locks and explicitly says that change affects Windows only; it is not evidence of a macOS fix for this `xMutexInit`-path crash. These release-note summaries are not exhaustive bug indexes, so the finding is “no matching published fix located,” not proof no fix exists.

Sources checked in this supplemental pass:

- [SQLite 3.54.0 release entry](https://www3.sqlite.org/releaselog/3_54_0.html), dated 2026-10-09: the stated mutex implementation change is Windows-only.
- [SQLite change history](https://sqlite.org/changes.html), including 3.53.3 (2026-06-26) and 3.53.4 (2026-07-24): general maintenance-fix descriptions, no identified match in the published summaries.
- The existing prior report records the direct API contracts, exact 3.53.2 release metadata, and why rusqlite issue [#1860](https://github.com/rusqlite/rusqlite/issues/1860) is a different Linux Unix-VFS deadlock rather than this macOS null callback.

**Revisit trigger:** a matching `sqlite3_initialize`/mutex callback crash, a real startup incident, or a relevant SQLite version/build/configuration/concurrency change. No current upstream finding justifies a workaround, dependency change, serialization, or reopening cold-start loops. No builds, tests, reproductions, or source changes were performed.
