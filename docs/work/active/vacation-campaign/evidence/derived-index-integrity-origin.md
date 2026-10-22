# Derived-index integrity origin assessment

## Verdict

No production BRN path found here that ordinarily commits passage/Message content without the matching FTS updates, or vice versa. For the four external-content FTS lanes, the confirmed mismatch risk is currently bounded to out-of-band mutation, storage/SQLite corruption or failure, or an untested SQLite/FTS defect—not a normal successful, rejected, canceled, or interrupted `SearchIndex::apply` write. The integration witness demonstrates an FTS-only mismatch after a deliberate test-side FTS `delete-all`; that proves the admission gap for a crafted derived-file state, not that BRN's ordinary writer produces the state.

## Source basis

At inspected clean checkout HEAD `bfc56877fff35971509cf219d36babeb4695b4e4` (the relevant retrieval, app-search and test files are unchanged from `2fb88e5`) (`codex/agent-first-followthrough`), all four lanes are external-content FTS5 tables in `crates/brn-retrieval/src/search_index.rs:88–107,127–158`. Passage and Message insert/delete triggers mirror their content table changes; `passages_no_update` and `messages_no_update` reject direct updates. Application replacement uses delete then insert, so the same triggers process both halves.

`SearchIndex::apply` opens one `TransactionBehavior::Immediate` transaction (`search_index.rs:424–430`), applies full reset/deletions/upserts (`431–537`), writes the watermark (`546–550`), then commits (`551`). Note removal deletes passages through `remove_doc` (`1088–1095`); new passage insertion uses the transaction passed to `insert_passages` (`1098–1117`). Because content-table triggers and FTS virtual-table writes execute inside that transaction, a returned pre-commit error drops the uncommitted transaction; the application does not separately commit either side. Full resync also deletes passages and messages inside that same transaction. `open_locked` creates the schema/triggers and expected metadata in its rebuild transaction (`1147–1195`); that is DDL recovery, not an independent content writer.

The production call graph found one application sync caller of `SearchIndex::apply`: `crates/brn-threads-app/src/search.rs:1397–1499`. It builds the derived `Changes` from a canonical reader snapshot, then invokes `apply`; no production code found directly writes the FTS tables. If canonical state advances after the snapshot, the index commits the earlier coherent snapshot and watermark, so a later sync sees the newer canonical watermark. The equal-watermark fast path (`search.rs:1401–1409`) is why that ordinary reconciliation does not detect independent FTS-only damage while the canonical watermark remains unchanged.

Cancellation is checked before a worker begins a job (`search.rs:642–665`). There is no cancellation token passed into `sync` or `apply`; dropping a ticket while a job is in progress may cause its answer to be ignored, but does not split an in-flight FTS/content transaction. The transaction either commits with its watermark or returns an error before commit. This is source-path analysis, not a new cancellation or process-kill experiment.

## Existing witnesses and limits

The retrieval tests' `fts_consistent` and `message_fts_consistent` helpers run rank-1 FTS5 external-content checks separately for the Note and Message lanes (`crates/brn-retrieval/tests/search_index.rs:52–61,663–674`). Normal insert/replace/removal journeys call these checks; the Message journey also asserts direct `UPDATE messages` is rejected (`:676–732`). A rejected invalid-role apply is exercised at `:733–743`, but it is not a fault-injection test that first performs FTS-triggering writes and then fails later in the same transaction.

The current per-lane recovery witness and focused baseline add deliberate test-only `delete-all` mutations, preserving canonical-content tables and watermark, and the supplied baseline record says the corrupted lane remains admitted as `Existing`. That is strong evidence for admission behavior after external derived-state alteration; it does not show an ordinary production write can create the inconsistency.

No process-kill-during-commit, forced disk/I/O failure, or bundled-SQLite crash-recovery experiment was run for this assessment. Atomic rollback/crash behavior is inferred from the single SQLite transaction and trigger ownership; unusual SQLite/storage failures remain outside the ordinary-path conclusion. The active cost overlay and supervisor were not read or modified. This assessment changed no repository files and ran no tests, builds, fixtures, CLI or workload.
