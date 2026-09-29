# brn

Agent-facing CLI for BRN workspaces: parsing, JSON envelopes and exit codes over the shared [brn-workflow](../brn-workflow/README.md) layer. The desktop drives the same workflow, so CLI results match desktop behavior; a workspace cannot be opened by both at once.

## Interfaces and source

[Entry point](src/main.rs), [CLI modules](src/cli/mod.rs), [error taxonomy](src/cli/error.rs).

## Commands

  brn status
  brn import PATH [--approve-for-search] [--operation UUID]
  brn documents list
  brn documents show SOURCE_ID
  brn documents set-search-approval SOURCE_ID --version-id VERSION_ID --state approved|draft|withdrawn [--operation UUID]
  brn index build
  brn search QUERY [--profile keyword|semantic|hybrid]
  brn ask QUESTION [--profile keyword|semantic|hybrid] [--session UUID] [--operation UUID] [--timeout-seconds N]
  brn conversations list
  brn conversations show SESSION_ID
  brn drafts list
  brn drafts show DRAFT_ID
  brn comments list --draft DRAFT_ID
  brn revisions list --draft DRAFT_ID
  brn revisions show REVISION_ID
  brn revisions diff --draft DRAFT_ID --from REVISION_ID --to REVISION_ID

## Global options

Accepted before or after the command: `--data-dir DIR` (required for commands; must be an existing absolute directory), `--json`, `--codex PATH` (absolute, does not imply authentication), `--model-dir DIR` (absolute), `--help`, `--version`.

## JSON envelope

`--json` prints exactly one envelope object on stdout; without it, results are human text and errors go to stderr.

```json
{"schema_version": 1, "command": "search", "ok": true, "data": {"query": "zephyr", "profile": "keyword", "evidence": []}}
```

```json
{"schema_version": 1, "command": "search", "ok": false, "error": {"code": "INDEX_MISSING", "message": "no active index; build it first"}}
```

Exit codes: `0` success; `2` usage (`USAGE`); `1` operational failure (`WORKSPACE_BUSY`, `NOT_FOUND`, `INDEX_MISSING`, `INDEX_STALE`, `INDEX_INVALID`, `PROFILE_UNAVAILABLE`, `OPERATION_CONFLICT`, `WORKFLOW_ERROR`); `124` deadline (`TIMEOUT`); `130` interrupted (`INTERRUPTED`).

## Semantics

- Import defaults to draft approval; `--approve-for-search` is explicit and is not publication approval.
- Search never builds the index; a missing or stale index is an error, not a trigger.
- Semantic/hybrid profiles never fall back to keyword; they fail with `PROFILE_UNAVAILABLE` in this build.
- Approval binds the exact source version; superseded versions reject approval.
- `--data-dir` must already exist and be absolute; opening it may initialize or recover per store semantics.
- Workspace ownership is exclusive: when the desktop or another process holds the directory, the command fails with `WORKSPACE_BUSY`. There is no bypass.
- SIGPIPE default disposition is restored at startup, so piping (`brn documents show X | head`) exits quietly.
- Lists report store insertion order; document content is preserved as exact bytes with no Unicode or line-ending normalization.

## Ask and the provider

`ask` requires `--codex`; without it the invocation is a usage error and the workspace is never opened. Provider deltas stream to stderr so stdout stays one envelope. `--timeout-seconds` defaults to 300 and is bounded 1..=3600; deadline exit is 124, SIGINT exit is 130, and completed, failed or interrupted turn outcomes are preserved honestly. Tests use fake providers; a live Codex is never verified by the test suite.

## Dependencies and features

Depends on `brn-workflow` (plus `serde`, `serde_json`, `uuid`, `libc`). `native-retrieval` forwards to retrieval `native`.

## Build and verification

Run from the repository root:

```sh
cargo +1.98.1 build -p brn --locked    # binary at target/debug/brn
cargo +1.98.1 test -p brn --locked --offline
```

Tests are subprocess-based over disposable temp dirs with synthetic fixtures; they need no network or credentials.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
