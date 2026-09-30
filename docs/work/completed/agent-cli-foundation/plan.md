# Agent-facing CLI foundation

Status: implemented, verified and merged as PR #2 (merge commit `1b49378` on `main`, 2026-09-30). Baseline: `main` at `18f3891`, branch `feature/agent-cli-foundation`. Baseline `cargo build`/`cargo test --workspace --locked --offline` passed 2026-09-29.

## Outcome

Turn `crates/brn` into the public `brn` CLI through which coding agents operate existing BRN workflows headlessly. The CLI is another interface to `brn-workflow`, not another implementation. Preserves `brn-flow` untouched.

## Command surface (v1)

Global: `--data-dir ABS` (required for workspace commands), `--json`, `--help`, `--version`; provider options `--codex ABS`, `--model-dir ABS` where applicable.

`status`; `import PATH [--approve-for-search] [--operation UUID]`; `documents list|show SOURCE_ID|set-search-approval SOURCE_ID --version-id V --state approved|draft|withdrawn [--operation UUID]`; `index build`; `search QUERY [--profile keyword|semantic|hybrid]` (default keyword, never silent substitution); `ask QUESTION [--profile P] [--session UUID] [--operation UUID] [--timeout-seconds N]` (default 300, range 1..=3600); `conversations list|show SESSION_ID`; `drafts list|show`; `comments list --draft ID`; `revisions list --draft ID | show ID | diff --draft ID --from ID --to ID`.

Out of scope: draft mutation, comment create/resolve, AI generation, candidate adoption, publication, graph, backup, migration, daemon/IPC/TUI, brn-flow changes.

## Contracts

- Dependency direction `brn -> brn-workflow -> {store, retrieval, provider}`. No SQL or provider logic in the CLI. Small workflow DTOs/re-exports only.
- JSON envelope (`--json`): exactly one object on stdout. Success `{"schema_version":1,"command":"<dotted>","ok":true,"data":...}`; failure `{"schema_version":1,"command":"<dotted|null>","ok":false,"error":{"code","message"}}`. Help/version are textual (documented exception). Deltas/diagnostics on stderr.
- Exit codes: 0 ok, 1 operational, 2 usage, 124 deadline, 130 SIGINT.
- Error codes: USAGE, WORKSPACE_BUSY, NOT_FOUND, INDEX_MISSING, INDEX_STALE, INDEX_INVALID, PROFILE_UNAVAILABLE, OPERATION_CONFLICT, TIMEOUT, INTERRUPTED, WORKFLOW_ERROR (fallback). NOT_FOUND/TIMEOUT/INTERRUPTED/USAGE typed by construction; workflow-string sentinels pinned by tests for the rest.
- Ownership: exclusive `brn.owner.lock` preserved; contention fails with WORKSPACE_BUSY (exit 1). No lock bypass.
- Import defaults to draft approval. Search never builds the index implicitly. Opening a workspace may initialize/recover per existing store semantics.
- Provider lifecycle via existing `Workspace::ask`; CLI-owned SIGINT flag + deadline thread feed the existing cancel flag; owned child reaping stays in the provider. Outcome statuses reported verbatim.
- Lists keep store order (insertion). Exact bytes, hashes, quotes, ranges preserved; no Unicode/line-ending normalization.

## Module map (crates/brn/src)

`main.rs` (signal setup, dispatch); `cli/mod.rs` (parser, envelope, exit mapping); `cli/error.rs`; `cli/status.rs`; `cli/documents.rs`; `cli/retrieval.rs` (import/approval/index/search); `cli/review.rs` (drafts/comments/revisions); `cli/ask.rs` (ask/conversations). Tests: `crates/brn/tests/cli_*.rs` subprocess tests via `CARGO_BIN_EXE_brn`, fake-provider python fixture, synthetic dirs.

## Model ownership

GLM 5.3 (lead): contracts, integration, review, final verification. GLM 5.3 Flash (subagents): B foundation (parser/envelope/status/documents + tests), C1 ingestion/retrieval, C2 review surfaces, D ask/conversations/provider tests, E review/docs.

## Acceptance

- Required command surface works against disposable workspaces; `--json` stdout is a single parseable object for success and handled failure; exit codes per table.
- Subprocess tests cover parsing, envelopes, core flow (import→approve→build→search without implicit approval/build), read surfaces, fake-provider ask (success/fail/SIGINT/deadline), ownership contention.
- `cargo fmt --all -- --check`, `cargo build/clippy/test --workspace --locked`, `scripts/verify-end-to-end.sh` pass; no brn-flow behavior change.
- Docs: `crates/brn/README.md`, evidence here. No push/merge/install; live provider not required.
