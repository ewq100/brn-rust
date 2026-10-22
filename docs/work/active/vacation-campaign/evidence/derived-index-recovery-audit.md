# Derived search-index recovery audit

**Disposition: one inferred, source-backed recovery gap worth a focused follow-up.** The candidate is a logically inconsistent but SQLite-readable FTS lane whose index schema, metadata and canonical watermark remain current. This is not an observed local failure: no test, build, fixture or workload was run.

## Candidate boundary

`SearchIndex::open` serializes access, reads the index header, runs `PRAGMA quick_check`, validates schema/version/config metadata, and classifies the database as usable (`crates/brn-retrieval/src/search_index.rs:366-393, 1147-1163, 1212-1283`). It does not run the FTS5 external-content integrity check. The test suite has a separate helper that issues `integrity-check` with `rank=1` for `fts_exact` and `fts_stem` (`crates/brn-retrieval/tests/search_index.rs:52-61`); the recovery test calls that helper after its cases, rather than creating a FTS-consistency mismatch and proving `SearchIndex::open` detects/rebuilds it (`:327-370`).

If a persistent derived index has structurally readable SQLite tables and unchanged `meta`/watermark but an FTS lane no longer corresponds to its `passages` content, the current path appears able to accept it as `Opened::Existing`. The worker's next `sync` returns immediately when the workspace identity and watermark match (`crates/brn-threads-app/src/search.rs:1397-1409`), so canonical state does not force a rebuild. Search rechecks visibility and candidate versions (`:1006-1077`), but those checks only run for candidates the index returned; they cannot detect a missing match, and a stale FTS candidate can still carry the canonical current passage/doc version. A persisted term-index mismatch can therefore inferentially cause silent false-negative retrieval or a false keyword match until an edit, reindex/rebuild, or other detectable storage failure occurs.

The evidence for this gap is the separation in source between SQLite health and FTS external-content consistency: the normal open path checks the former, while the tests use a specialized check for the latter. Whether a particular on-disk FTS inconsistency also causes this SQLite build's `quick_check` to fail was not executed or established here; this finding is specifically an untested gap, not a claim that every such mismatch passes `quick_check`.

## Current protections and nearby evidence

- Canonical SQLite remains authoritative and the index is disposable/rebuildable (`crates/brn-retrieval/README.md`; `docs/architecture/invariants.md:5,11`).
- Index creation/outdated schema/config, page corruption, empty database and interrupted creation are covered in `outdated_damaged_or_interrupted_indexes_are_rebuilt_in_place` (`crates/brn-retrieval/tests/search_index.rs:327-370`). Foreign files and symlinks are refused without modification (`:372-402`).
- Note candidates are rechecked against canonical visibility and version; synchronization is keyed by workspace identity and watermark. Those guards cover canonical changes, not derived FTS mismatch when the watermark is unchanged (`crates/brn-threads-app/src/search.rs:1006-1077,1397-1499`).
- Existing keyword-scale and cancellation/generation evidence supplied for this task was not repeated; it addresses different boundaries.

## Smallest optional follow-up

Add one focused synthetic characterization that creates a normal index with current metadata/watermark, introduces an FTS-only consistency mismatch while keeping the file readable, then asks `SearchIndex::open` to recover and confirms a subsequent canonical sync restores expected keyword results. If the mismatch is intended to be recognized during open, use the existing FTS5 integrity-check mechanism (or another narrow integrity proof) to route the owned derived file to rebuild; do not modify canonical rows or weaken foreign-file refusal. Keep recovery limited to the BRN-owned, correctly branded index.

## Scope and qualification

Inspected clean checkout `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, branch `codex/agent-first-followthrough`, HEAD `232e3edd736a511a82ba40533f7e50a6af13062c`. This is the current main208 production retrieval source; the merged PR208 backup test changes and open PR209 Note/provider changes do not change the retrieval code inspected. No repository or lead-record files were written. No tests, builds, fixture/workload, CLI, network, provider, account, native action, or nested helper was run.
