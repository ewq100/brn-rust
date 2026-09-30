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
  brn drafts create --title TITLE --text-file PATH [--operation UUID]
  brn drafts checkpoint DRAFT_ID --base-revision UUID --expected-generation N --generation N --text-file PATH [--operation UUID]
  brn drafts save DRAFT_ID --base-revision UUID --expected-generation N --generation N --text-file PATH [--operation UUID]
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

The error object may carry an **additive-optional `context` field** (schema_version stays 1):

```json
{"schema_version": 1, "command": "ask", "ok": false,
 "error": {"code": "TIMEOUT", "message": "ask timed out after 2s; operation …",
   "context": {"operation_id": "…", "session_id": "…", "recorded_status": "interrupted", "provider_outcome": "unknown"}}}
```

`context` currently appears only on `ask` failures; consumers must treat its
absence as normal for every command and every other error.

Exit codes: `0` success; `2` usage (`USAGE`); `1` operational failure (`WORKSPACE_BUSY`, `NOT_FOUND`, `INDEX_MISSING`, `INDEX_STALE`, `INDEX_INVALID`, `PROFILE_UNAVAILABLE`, `OPERATION_CONFLICT`, `WORKFLOW_ERROR`); `124` deadline (`TIMEOUT`); `130` interrupted (`INTERRUPTED`). Codes are derived from typed workflow error categories at the source, never from matching message wording; uncategorized failures are an honest `WORKFLOW_ERROR`.

## Semantics

- Import defaults to draft approval; `--approve-for-search` is explicit and is not publication approval.
- `drafts create` reads a regular UTF-8 text file up to the 1 MiB draft limit with a bounded read and preserves exact bytes (no Unicode or line-ending normalization). Empty text is permitted; a blank title is rejected. The envelope carries the operation id and the created draft (id, title, base revision, generation, sha256). Reusing the same operation id with the same title and text replays the recorded draft; conflicting payload reuse fails with `OPERATION_CONFLICT`.
- `drafts checkpoint DRAFT_ID` uses the exact flags `--base-revision UUID`, `--expected-generation N`, `--generation N`, and `--text-file PATH` (plus optional `--operation UUID`). These are the CLI's checkpoint spellings; the command reads the bounded UTF-8 text explicitly, maps the caller-supplied expected state to the existing `DraftStamp`, and calls the shared checkpoint workflow. The success envelope includes `operation_id`, `checkpoint_id` (the resulting `base_revision`), and the resulting draft stamp. Existing generation rules apply, including unchanged text at the expected generation; stale state is rejected. Same-payload operation replay returns the original receipt after restart or later edits, while conflicting reuse fails with `OPERATION_CONFLICT`.
- `drafts save DRAFT_ID` uses the same `--base-revision UUID`, `--expected-generation N`, `--generation N`, `--text-file PATH`, and optional `--operation UUID` conventions as checkpoint. It reads bounded exact UTF-8 text, preserves the caller's expected base/generation, and calls the shared comment-safe working-draft save path. The success envelope includes the actual `operation_id` and resulting draft stamp; it does not create a checkpoint. Unchanged text is allowed, stale state is rejected, same-payload operation replay returns the original receipt after restart or later edits, and conflicting operation reuse fails with `OPERATION_CONFLICT`.
- Search never builds the index; a missing or stale index is an error, not a trigger.
- Semantic/hybrid profiles never fall back to keyword; they fail with `PROFILE_UNAVAILABLE` in this build.
- Approval binds the exact source version; superseded versions reject approval.
- `--data-dir` must already exist and be absolute; opening it may initialize or recover per store semantics.
- Workspace ownership is exclusive: when the desktop or another process holds the directory, the command fails with `WORKSPACE_BUSY`. There is no bypass.
- Closed stdout pipes are handled quietly through explicit BrokenPipe handling, without restoring process-wide SIGPIPE termination; piping (`brn documents show X | head`) exits quietly.
- Lists report store insertion order; document content is preserved as exact bytes with no Unicode or line-ending normalization.

## Ask and the provider

`ask` requires `--codex`; without it the invocation is a usage error and the workspace is never opened. Provider deltas stream to stderr so stdout stays one envelope. `--timeout-seconds` defaults to 300 and is bounded 1..=3600; deadline exit is 124, SIGINT exit is 130, and completed, failed or interrupted turn outcomes are preserved honestly. Tests use fake providers; a live Codex is never verified by the test suite.

### Ask failure context

`ask` failures carry an `error.context` object with machine-readable
identifiers and an honest split between the durable local record and the
provider outcome:

- `operation_id`: the BRN-generated or caller-supplied id for this attempt.
- `session_id`: the session established during the run (created by it, or the caller's validated `--session`); `null` when no session was established.
- `recorded_status`: the durable local record state (`completed`/`failed`/`interrupted`/`running`/`pending`); `null` means no durable turn-record state is known — including the rare case where a record exists but the outcome of its final write is unknown (for example, the completing store write itself failed).
- `provider_outcome`: what BRN knows about the provider side; the reachable values are `"unknown" | "completed" | "failed"`. `"unknown"` means BRN could not observe the provider outcome — transport loss or an interrupted record is **not** proof of cancellation. An interrupted record always reports `"unknown"`, because it is ambiguous between a server-confirmed interruption and uncertain transport loss. `"completed"`/`"failed"` imply a provider-confirmed outcome. `"interrupted"` is reserved: it would mean a provider-confirmed interrupted outcome, which the current CLI never claims from an ambiguous record.

Resubmitting the SAME operation id returns the recorded state without a new
external submission. After an unknown provider outcome, a NEW operation id is
not known to be safe (it may duplicate the external turn). Nothing is ever
auto-replayed. USAGE (missing `--codex`) and NOT_FOUND (unknown `--session`)
failures happen before an operation id exists and carry no context.

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
