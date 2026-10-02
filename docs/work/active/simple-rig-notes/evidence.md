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

## Step 1: spike

- 2026-10-02; macOS 26.5, Rust 1.98.1; commits `52b9756`, `065f454`, `745024d`, `7c8b392`, plus the commit containing this entry (`docs: record Rig spike findings`). [Findings and decision](../../../../experiments/rig-spike/FINDINGS.md).
- Tasks 1–2 **PASS** after direct TLS correction: build, graph, dependency trees, strict Clippy and four synthetic tests; one libsqlite3-sys 0.38.2, FTS5 hit count 1. Initial facade TLS resolution failed; corrected graph passed.
- Earlier offline commands (repository root):
  `cargo build --manifest-path experiments/rig-spike/Cargo.toml --locked`;
  `cargo run --manifest-path experiments/rig-spike/Cargo.toml --locked -- graph`;
  `cargo tree --manifest-path experiments/rig-spike/Cargo.toml --locked -i libsqlite3-sys`;
  `cargo tree --manifest-path experiments/rig-spike/Cargo.toml --locked -d --depth 0`;
  `cargo clippy --manifest-path experiments/rig-spike/Cargo.toml --all-targets --locked -- -D warnings`;
  `cargo test --manifest-path experiments/rig-spike/Cargo.toml --locked`.
- Task 3 **PASS** for both logins/identity; first device codes expired, retries succeeded. Copilot models **PASS**; ChatGPT models **FAIL** (HTTP 400).
- Controller's live commands, in generic form: `$S` = experiment debug binary, `$C` = external mode-0700 credentials folder; prompts are scenario placeholders, not retained literal text:
  `$S login chatgpt "$C"`; `$S login copilot "$C"` (each retried);
  `$S models chatgpt "$C"`; `$S models copilot "$C"`;
  `$S chat chatgpt "$C" MODEL "TOOL_PROMPT"` for gpt-5.4, gpt-5.3-codex, gpt-5.5, gpt-5.6-sol, gpt-5.6-terra, gpt-6-sol, gpt-6.1-sol;
  `$S chat copilot "$C" MODEL "TOOL_PROMPT"` for gpt-4o, gpt-5.4, claude-sonnet-5;
  `$S cancel copilot "$C" gpt-4o "TOOL_PROMPT" 1500`; `$S cancel copilot "$C" gpt-4o "LONG_PROMPT" 4000`;
  `rm "$C/copilot.json"`; `$S chat copilot "$C" gpt-4o "TOOL_PROMPT"` (session refresh).
- Task 4 Copilot **PASS**: tool/stream, restart reuse, local cancel (partial text at 4000 ms), session refresh. ChatGPT **BLOCKED**: supported routes hit HTTP 429 usage limit; tool/restart/cancel not observed.
- Task 5 **PASS** (fresh checks): `cargo check --manifest-path experiments/rig-spike/Cargo.toml --locked --features cassette`; `cargo check --manifest-path experiments/rig-spike/Cargo.toml --locked --features test-utils`. Both exit 0; no lockfile change. HTTP cassette engine/fixtures not exercised.
- Decision: **Copilot GO; ChatGPT CONDITIONAL GO**. Rerun ChatGPT Task 4 with gpt-5.5 after 2026-10-03 20:24 EEST before accepting step 4's ChatGPT work. No live calls by Task 5; no native acceptance or server-side cancellation claim.
