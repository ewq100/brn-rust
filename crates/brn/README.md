# brn

Agent-facing CLI for BRN workspaces: parsing, JSON envelopes and exit codes over the shared [brn-workflow](../brn-workflow/README.md) layer. The desktop drives the same workflow, so CLI results match desktop behavior; a workspace cannot be opened by both at once.

## Interfaces and source

[Entry point](src/main.rs), [CLI modules](src/cli/mod.rs), [error taxonomy](src/cli/error.rs).

## Commands

  brn notes open PATH --vault DIR [--operation UUID]
  brn notes show NOTE_ID
  brn notes buffer save NOTE_ID --base-file-state UUID --expected-generation N --generation N --text-file PATH [--operation UUID]
  brn notes save NOTE_ID --base-file-state UUID --expected-generation N --generation N --text-file PATH [--operation UUID]
  brn notes recovery list
  brn notes recovery show NOTE_ID
  brn notes recovery reconcile --operation UUID
  brn notes recovery accept-current --save-operation UUID --file-state UUID --keep-recovery [--operation UUID]
  brn notes compare NOTE_ID
  brn notes reload NOTE_ID --base-file-state UUID --expected-generation N --discard-local-edits [--operation UUID]
  brn notes relink NOTE_ID --path PATH --base-file-state UUID --expected-generation N --confirm-identity [--operation UUID]
  brn notes save-copy NOTE_ID --path PATH --base-file-state UUID --expected-generation N --generation N --text-file PATH [--operation UUID]
  brn notes approve-for-search NOTE_ID --file-state UUID [--operation UUID]
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
  brn comments add --draft DRAFT_ID --base-revision UUID --expected-generation N --generation N --text-file PATH --start-byte N --end-byte N --quote-file PATH --body-file PATH [--operation UUID]
  brn comments resolve COMMENT_ID --draft DRAFT_ID --expected-status-version N [--operation UUID]
  brn comments reopen COMMENT_ID --draft DRAFT_ID --expected-status-version N [--operation UUID]
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

`context` appears on typed note and `ask` failures; consumers must also handle
errors without context (including usage, input preparation and workspace opening).

Exit codes: `0` success; `2` usage (`USAGE`); `1` operational failure (`WORKSPACE_BUSY`, `NOT_FOUND`, `INDEX_MISSING`, `INDEX_STALE`, `INDEX_INVALID`, `EVIDENCE_STALE`, `CONTEXT_STALE`, `PROFILE_UNAVAILABLE`, `OPERATION_CONFLICT`, `WORKFLOW_ERROR`); `124` deadline (`TIMEOUT`); `130` interrupted (`INTERRUPTED`). Codes are derived from typed workflow error categories at the source, never from matching message wording; uncategorized failures are an honest `WORKFLOW_ERROR`.

Note failures also exit `1`: `NOTE_STATE_CHANGED`, `NOTE_CONFLICT`,
`NOTE_MISSING`, `NOTE_UNSUPPORTED`, `NOTE_SAVE_UNCERTAIN`, `NOTE_IO_ERROR`,
`NOTE_STORAGE_ERROR`, `VAULT_BUSY`, `VAULT_UNAVAILABLE`; operation/workspace
conflicts retain `OPERATION_CONFLICT`/`WORKSPACE_BUSY`.

## Safe Markdown notes

[`notes`](src/cli/notes.rs) calls the shared workflow; no CLI filesystem-write
fallback exists. The vault root and `notes open PATH` must be explicit absolute
paths; PATH must be contained in that root. Relink/copy destinations are
contained vault-relative `.md` paths. A workspace binds one vault; use another
data directory for a different vault. Vault ownership is exclusive across
workspaces, independently of data-directory ownership.

`open`/`show` return the `NoteView`: exact freshly observed `saved` bytes,
durable `buffer`, editing `stamp`, separate `current_file_state`, availability
and search approval. Missing/unavailable/owned-elsewhere views never pass
recovery bytes off as saved content. ID-only commands work after process
restart without another `open`. Recovery list/show require no vault access.

Buffer save recovers text in SQLite **without saving Markdown**. Save explicitly
writes the original file, and save-copy creates a separate note with a separate
ID and no inherited approval. Their request uses exactly the supplied
`--base-file-state` and `--expected-generation`, never a fetched replacement.
Higher generations update text; equal generations require identical bytes
(saving an already recovered buffer needs no artificial generation increment).
Text inputs are regular UTF-8 files, bounded to 1 MiB in bytes, read before
workspace access; empty files, BOMs and CRLF are preserved.

Save/copy receipts include `operation_id`, `source_note_id`, `note_id`,
`submitted_generation`, `stamp`, `filesystem_outcome` and `recovery_available`.
`Applied` acknowledges verified saving; a verified no-op is `NotApplied`.
Same-payload operation replay returns the original receipt/refusal even after
later edits, not proof that its old state remains current. Reconciliation
selects the original save with required `--operation` and never retries a write.

Compare returns baseline/working/fresh observed text and its observation token.
Reload requires `--discard-local-edits`; relink requires `--confirm-identity`
and retains local work. Accept-current names the unresolved original through
`--save-operation`; optional `--operation` is a separate acknowledgement.
It requires a reviewed `--file-state` and explicit `--keep-recovery`, preserves
uncertain recovery/artifacts, and neither writes disk nor grants approval nor
turns the original failure into a successful save. The success result is a
note view, not an original-save receipt.
Open/reload/relink views also identify their `operation_id`; accept-current
identifies its acknowledgement as `operation_id` and the original as
`save_operation_id`.

Approval requires the reviewed `current_file_state`, **not** a possibly older
editing baseline. It freezes saved bytes only. Changed files/saves withdraw
eligibility; after reconciliation, explicitly reapprove and rebuild the index.
Search approval is not publication approval.

Typed note failures carry `error.context` with nullable `operation_id`,
`note_id`, `phase`, known `filesystem_outcome`
(`NotApplied`/`Applied`/`Unknown`) and boolean `recovery_available`.
`true` confirms the submitted snapshot is retained; `false` does not prove
there is no older recovery. A possible write is never reported as cancelled
merely because SIGINT arrives afterward; its durable result stands.

Example (directories already exist; replace IDs/tokens with returned values):

```text
brn notes open /absolute/disposable-vault/plan.md --vault /absolute/disposable-vault --data-dir /absolute/disposable-data --json
brn notes buffer save NOTE_ID --base-file-state FILE_STATE --expected-generation 0 --generation 1 --text-file /absolute/edited.txt --data-dir /absolute/disposable-data --json
brn notes save NOTE_ID --base-file-state FILE_STATE --expected-generation 1 --generation 1 --text-file /absolute/edited.txt --operation SAVE_UUID --data-dir /absolute/disposable-data --json
brn notes recovery reconcile --operation SAVE_UUID --data-dir /absolute/disposable-data --json
brn notes show NOTE_ID --data-dir /absolute/disposable-data --json
brn notes approve-for-search NOTE_ID --file-state CURRENT_FILE_STATE --data-dir /absolute/disposable-data --json
brn index build --data-dir /absolute/disposable-data --json
```

## Semantics

- Import defaults to draft approval; `--approve-for-search` is explicit and is not publication approval.
- `drafts create` reads a regular UTF-8 text file up to the 1 MiB draft limit with a bounded read and preserves exact bytes (no Unicode or line-ending normalization). Empty text is permitted; a blank title is rejected. The envelope carries the operation id and the created draft (id, title, base revision, generation, sha256). Reusing the same operation id with the same title and text replays the recorded draft; conflicting payload reuse fails with `OPERATION_CONFLICT`.
- `drafts checkpoint DRAFT_ID` uses the exact flags `--base-revision UUID`, `--expected-generation N`, `--generation N`, and `--text-file PATH` (plus optional `--operation UUID`). These are the CLI's checkpoint spellings; the command reads the bounded UTF-8 text explicitly, maps the caller-supplied expected state to the existing `DraftStamp`, and calls the shared checkpoint workflow. The success envelope includes `operation_id`, `checkpoint_id` (the resulting `base_revision`), and the resulting draft stamp. Existing generation rules apply, including unchanged text at the expected generation; stale state is rejected. Same-payload operation replay returns the original receipt after restart or later edits, while conflicting reuse fails with `OPERATION_CONFLICT`.
- `drafts save DRAFT_ID` uses the same `--base-revision UUID`, `--expected-generation N`, `--generation N`, `--text-file PATH`, and optional `--operation UUID` conventions as checkpoint. Its submitted `--generation` must be strictly greater than `--expected-generation`. It reads bounded exact UTF-8 text, preserves the caller's expected base/generation, and calls the shared comment-safe working-draft save path. The success envelope includes the actual `operation_id` and resulting draft stamp; it does not create a checkpoint. Unchanged text is allowed, stale state is rejected, same-payload operation replay returns the original receipt after restart or later edits, and conflicting operation reuse fails with `OPERATION_CONFLICT`.
- `comments add` uses `--draft`, the same expected base/generation flags, `--text-file`, and explicit UTF-8 `--quote-file`/`--body-file` inputs with a half-open UTF-8 byte range. This first CLI command accepts only already-saved, unchanged draft text; it uses the existing empty edit trace and creates the comment checkpoint through the shared workflow. It does not combine comment creation with unsaved text edits. The success envelope includes the actual `operation_id` and `comment_id`; same-payload operation replay returns the original receipt, while conflicting reuse fails with `OPERATION_CONFLICT`.
- For `comments add`, `--generation` must be at least `--expected-generation`; equality is valid for unchanged text, and greater generations remain supported. The comparison uses only the supplied values and happens before the workspace is opened.
- `comments resolve COMMENT_ID` requires the caller-supplied `--draft` and `--expected-status-version` and maps to the shared status workflow with `resolved`. The success envelope returns the actual `operation_id` and resulting comment, including its `status` and `status_version`; the comment body, original quote/provenance and draft text are unchanged. Same-status resolution is a no-op with the existing status version. Same-payload operation replay returns its recorded result even after a later status change, while conflicting operation reuse fails with `OPERATION_CONFLICT`; stale status versions and wrong draft/comment identities fail without mutation. Resolving a comment is not document approval, search approval, AI-suggestion application or publication.
- `comments reopen COMMENT_ID` uses the same precondition, receipt and replay rules as resolve, mapping to `open`. Reopening changes only lifecycle status; the status version advances when the state changes, stays unchanged for an already-open no-op, and the body, original quote/provenance and current draft text remain intact.

Verified lifecycle example (each command uses the same existing workspace and the listed status version):

```text
brn comments resolve COMMENT_ID --draft DRAFT_ID --expected-status-version 0 --data-dir /tmp/brn-data --json
brn comments reopen COMMENT_ID --draft DRAFT_ID --expected-status-version 1 --data-dir /tmp/brn-data --json
brn comments list --draft DRAFT_ID --data-dir /tmp/brn-data --json
# list reports status "open" and status_version 2; original evidence and draft text are unchanged
```
- Search never builds the index; a missing or stale index is an error, not a trigger.
- Semantic/hybrid profiles never fall back to keyword; they fail with `PROFILE_UNAVAILABLE` in this build.
- Approval binds the exact source version; superseded versions reject approval.
- `documents list` retains valid rows' legacy fields and adds `note_id`,
  `current_state` and `message`. Excluded managed rows contain identity/title
  and state/reason only, not current bytes/version/hash/approval. `documents show`
  freshly validates its snapshot; a known shadowed, changed or unavailable row
  fails `EVIDENCE_STALE`, not `NOT_FOUND`. Explicit revision/history access keeps
  immutable originals. Managed-path import and permission changes cannot revive
  shadowed copies or approve recovered buffers.
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

Managed evidence is revalidated before provider access/submission and at
completion. Deltas are provisional. `EVIDENCE_STALE` may accompany an actually
completed provider response: `recorded_status` and `provider_outcome` stay
`completed`, with the preserved historical `receipt` in failure context.
Same-operation stale replay returns that receipt/outcome without resubmission;
a new operation resuming stale managed context fails `CONTEXT_STALE` and
requires a fresh conversation. Turn JSON adds `evidence_currentness`
(`Unqualified`, `CurrentAtCompletion`, `StaleAtCompletion`); these describe
completion, not permanent current eligibility. `conversations show` is labeled
historical and remains readable offline.

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
