# Simple Rig-based notes app: evidence

Actual commands and results, recorded at the end of each step. Expected results in the plans are instructions, not evidence.

## Planning

- 2026-10-02: specification approved and committed at `c912480`. Roadmap, [spike](spike.md) and [store and vault](store.md) plans written. No product code changed; no builds, live calls or model downloads performed.
- Checked while planning: Rig `v0.43.0` source (ChatGPT/Copilot auth, client `authenticate`, agent streaming and `Tool` signatures, facade features); `rig-sqlite` uses rusqlite `0.32` and `rig-fastembed` uses fastembed `4.5`, so neither is used; SQLite `VACUUM INTO ?1` accepts a bound path (checked with Python's sqlite3).

## Plan review (2026-10-02)

The user requested a GPT-6.1 Sol (high reasoning) review of the step 1–2 plans. Accepted and applied:

- Missing `brn.sqlite` is restored from the newest backup, like a corrupt one (spec section 3).
- Existing databases and backups must carry BRN's application ID; unbranded or foreign databases are refused unchanged; empty or damaged backups are skipped.
- Backups use SQLite's online backup API (spec section 3), and a new backup's number is always above existing ones, so pruning never deletes it.
- `read_note` compares the checked file identity with the opened file, refusing a symlink swapped in between.
- Non-UTF-8 note content is listed as unreadable by step 3's indexing.
- Spike: credentials folder must be a real folder owned by the user with mode 700; ChatGPT email read from the cached `id_token`; rule against copying raw errors, IDs or emails into evidence; cancel check records whether text arrived first.

Partly accepted: a full descriptor-based vault walk and sanitizing code for the throwaway spike were judged too heavy for this app; the identity check and the evidence rule replace them.

Check: the step 2 store and vault code and tests from the revised plan were copied into two scratch crates outside the repository (stub crate-root helpers, rusqlite `=0.40.2` with `bundled`/`backup`, sha2, Rust 1.98.1, offline). Results: store 14 tests passed, vault 6 tests passed, `cargo clippy --all-targets -- -D warnings` clean for both. Scratch crates deleted afterwards. The spike code was not compiled (Rig is not in the local registry).
