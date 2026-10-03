# brn

Agent-facing CLI for BRN workspaces: parsing, JSON envelopes and exit codes over the shared [brn-workflow](../brn-workflow/README.md) layer. The desktop drives the same workflow, so CLI results match desktop behavior; a workspace cannot be opened by both at once.

## Interfaces and source

[Entry point](src/main.rs), [CLI modules](src/cli/mod.rs), [error taxonomy](src/cli/error.rs).

The default/headless build is keyword-only. Build the full CLI with
`cargo build -p brn --features native-retrieval --locked`; this enables the
shared workflow's native model support, not automatic model downloading.
Simple read/search/history and explicit subscription actions use the owned
AppWorker, including manual Markdown editing and durable unfinished-work recovery.
Legacy local editing and history remain available; legacy `brn ask`
submission is retired. Both consumers now use AppWorker for simple work; the
old production provider crate/configuration has been removed.

## Commands

  brn edit open PATH
  brn edit recover PATH --baseline UUID --expected-generation N --generation N --file F
  brn edit save PATH --baseline UUID --expected-generation N --generation N --file F --operation UUID [--copy PATH]
  brn edit reload PATH --baseline UUID --expected-generation N --observed-file F [--discard]
  brn edit list
  brn edit reconcile OPERATION
  brn notes open PATH --vault DIR [--operation UUID]
  brn ai connect chatgpt|copilot [--timeout-seconds N]
  brn ai disconnect chatgpt|copilot
  brn ai status
  brn ai models chatgpt|copilot [--timeout-seconds N]
  brn ai select --provider chatgpt|copilot --model MODEL
  brn models download --approve-download [--model-dir DIR] [--timeout-seconds N]
  brn notes list [--folder FOLDER] [--cursor PATH]
  brn notes show PATH.md|NOTE_ID
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
  brn search QUERY [--profile keyword|semantic|hybrid] [--limit N]
  brn ask QUESTION [--session UUID] [--operation UUID] [--timeout-seconds N]
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

Accepted before or after the command: `--data-dir DIR` (required for commands;
must be an existing absolute directory), `--json`, `--model-dir DIR` (absolute),
`--help`, `--version`. The old `--codex` option is unknown (usage 2), not accepted
then ignored. Legacy status no longer reports `codex_configured`.

Simple commands also accept `--vault DIR` for first binding and
`--credentials-dir DIR` for an explicit credential location. Both are absolute;
the vault must exist. Credentials must be current-user-owned, safe and outside
Git, data and vault directories. The exact default is the sibling
`<data-directory-name>.credentials`, not an environment/Rig account lookup.
The owning workflow persists the non-secret location and honors it on reopening.
`--legacy` explicitly selects legacy authority for empty shared commands;
it conflicts with simple options/actions. Legacy `brn ask` always refuses;
old thread identifiers remain local history, never Rig resume inputs.

### Authority dispatch

| Command family | Legacy markers | Simple markers/backups | Empty |
| --- | --- | --- | --- |
| `edit *`, `ai *`, `models download`, `notes list`, `notes show PATH.md` | Mode conflict | AppWorker | Initialize simple authority |
| `notes show UUID`, managed editing/recovery, import/documents/drafts/comments/revisions/index build | Workspace | Mode conflict | Initialize legacy authority |
| `status`, `search`, `conversations list/show` | Legacy shapes | Simple shapes | Require `--vault` (simple) or `--legacy`; otherwise no DB |
| `ask` | `LEGACY_AI_RETIRED`, no submission | AppWorker | Require `--vault`; otherwise no DB/network |

Mixed markers always fail `WORKSPACE_MODE_CONFLICT`. Database WAL/SHM/journal
sidecars and recognized WorkStore backups count even when the DB is missing.
The shared classifier is advisory: store checks repeat after the owner lock.
`brn-flow` preserves legacy local/search/history (`sessions`, `history`), but
cannot open/create legacy authority in a simple folder.

`notes show` parses UUID first; otherwise it requires a visible contained
vault-relative Markdown path. UUIDs are never repurposed as filenames.
In simple mode, listing uses path-sorted 200-row cursor pages, showing reads
exact current UTF-8 bytes, and search defaults to hybrid/10 results (limit 1–50).
Without an installed model **every** simple profile explicitly reports
`keyword_only: true`. Legacy search keeps its keyword default and original
profile/evidence shape; explicit `--limit` only truncates results.

### Simple Markdown editing

`edit open PATH` uses a contained vault-relative `.md` path and returns saved
bytes, a recoverable editor record and its opaque `stamp.baseline` UUID and
generation. The first command binds a disposable or chosen vault with
`--vault DIR`; later commands honor the saved binding. `edit list` returns
persisted editor records, including unfinished work after a restart.

`edit recover` durably stores the exact UTF-8 input in operational recovery;
it does not Save the Markdown file. `edit save` performs explicit Save through
the same worker and requires an operation UUID. Supply the current baseline and
expected generation from the record; the submitted generation must be at least
the expected generation, and equal generations require identical bytes.
Files may be empty and are limited to 1 MiB, preserving BOM, frontmatter and
line endings without trimming or normalization.

Save refuses changed or missing originals, preserving the editor's recovery.
`--copy PATH` installs an independent copy only at an unused safe Markdown path.
`edit reconcile OPERATION` reports the recorded filesystem outcome without
replaying a write. Reusing a Save UUID with the same payload returns its recorded
result; changing the payload fails `OPERATION_CONFLICT`. Conflicts use
`CONTEXT_STALE`; an unproven filesystem outcome uses `SAVE_UNCERTAIN` and retains
recovery. These direct user commands do not grant AI write approval.

`edit reload` adopts the freshly viewed saved file as the editor baseline.
Write the `observed` fingerprint object returned by `edit open` to the JSON file
named by `--observed-file`; changed disk identity or bytes reject that observation.
Unfinished local edits require explicit `--discard`. Reload does not clear an
uncertain original Save or guess a replacement file's identity.

For a manual check, create an existing data directory and synthetic vault, open
`plan.md` containing BOM/CRLF text, save changed bytes with the returned stamp
and a fresh UUID, and compare the file bytes. Recover another edit and reopen
the CLI to confirm it remains available. Change the disk file externally before
Save and confirm refusal; verify Save Copy also refuses an occupied destination.

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

Simple failures use typed `AI_RECONNECT_NEEDED`, `AI_CODE_EXPIRED`,
`AI_RATE_LIMITED`, `AI_NETWORK`, `AI_MODEL_REFUSED`, `AI_INVALID_TOOL_USE`,
`AI_TOOL_LIMIT_REACHED`, `AI_UNSAFE_CREDENTIALS`, `AI_STORAGE_ERROR`,
`AI_SELECTION_REQUIRED`, `AI_TOOL_REJECTED`, `AI_INDEX_STALE`, `VAULT_NOT_BOUND`,
`WORKSPACE_MODE_CONFLICT`, `WORKSPACE_MODE_REQUIRED`, `LEGACY_AI_RETIRED`,
`MODEL_INVALID`, `MODEL_DOWNLOAD_FAILED`, `SEMANTIC_UNAVAILABLE_IN_BUILD` or
`TOOLS_BUSY`. These are operational exit 1 unless explicit local cancellation
or deadline applies; persistence failure is never relabeled as cancellation.

### Schema 1 simple-folder command-shape cutover

The envelope remains schema 1; **command data depends on workspace authority**.
Simple `status` includes `mode: "simple"`, `vault_root`, `model_installed`,
`model_download`, version/data directory and native-retrieval capability.
Simple notes list returns `{notes, next_cursor}`, show returns `{path, text}`,
search returns `{query, hits, keyword_only}` with path/byte-range/quote/score hits.
Simple conversations list returns `{conversations}` (`id`, `title`, `turns`);
show returns `{session_id, historical: true, turns}`. Turn/ask data is
`{operation_id, session_id, provider, model, question, answer, status, error_code}`.
There are **no provider thread/turn IDs, evidence snapshots or usage DTOs**
in new turns. Legacy status/search/conversation data retains its prior shape.
`ai status` returns `{accounts, selection, selection_error}`. Absent or valid
selection has `selection_error: null`. An unavailable/stale or malformed saved
selection returns exit 0 with both actual local account statuses,
`selection: null`, and `selection_error: {code: "AI_MODEL_REFUSED", message}`
containing a safe diagnostic, never raw persisted selection data. Status does not
change the saved selection or choose a fallback; new Ask still refuses it and
recorded replay keeps its frozen selection. Other status failures remain errors.
Connect returns safe account status,
disconnect returns provider/connected=false/name=null, models returns
provider/models, and select returns selection. Connected means local credentials
exist, not upstream validity; null name means account name unavailable.
Download success reports operation/directory/download and `installed: true`,
only after model activation, not merely `ModelDownloaded`.

## Safe Markdown notes

This section describes the retained legacy managed-note commands, not simple
Markdown publication.

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
and retains local work. Reload refuses an inode change: external editors that
save atomically (for example TextEdit) replace the inode, so use confirmed Relink
to the same path before reloading/discarding local edits. Relink alone retains
those edits. Accept-current names the unresolved original through
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

The following local mutation/retrieval contracts describe legacy workspaces.

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
- `documents list` JSON retains valid rows' legacy fields and adds `note_id`,
  `current_state` and `message`. Human Current rows retain the exact legacy
  `SOURCE_ID VERSION_ID APPROVAL TITLE` line; only non-current managed rows
  include state and reason. Excluded managed rows contain identity/title
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

New `ask` requires an available bound vault and an explicitly saved provider/model.
There is no account/model fallback, default selection, eager retrieval profile
or automatic discovery. `ask --profile` is obsolete usage 2 before workspace
opening; use `search --profile` for human search. Explicit `ai models` discovery
precedes Copilot selection. `--session` retains its conversation meaning.
AppWorker performs replay before vault refresh, discovery membership or auth:
terminal UUID replay uses its frozen model without querying current selection,
even without a saved selection or available vault. Changed payloads conflict;
Running never resubmits.

Only explicit Connect performs device login. URI/code is sent solely to the
dedicated transient **stderr login surface**, including under `--json`; it is
cleared on terminals on a TTY. This deliberate exception is not a log/envelope:
never serialize raw events, enable verbose HTTP/Rig tracing or persist codes.
Failed/cancelled Connect queries real local Status: credentials may already exist,
and a missing display name remains unknown rather than claiming disconnected.
Explicit Connect/models/download and Ask all observe SIGINT and deadlines,
retain pre-admission cancellation intent via fenced joined shutdown, and report
no guarantee of upstream cancellation or no billing.

Ask deltas stream best-effort to stderr; stdout is exactly one final envelope.
Timeout defaults to 300, range 1–3600; deadline exits 124, SIGINT exits 130.
Durably Completed is success even after a late signal. Explicit shutdown errors
are surfaced, especially persistence failures; Drop is only a safety join.
Genuine recorded typed failures are not relabeled by a coincident Stop/deadline.
Download requires fresh `--approve-download` (otherwise usage 2 before work).
Default builds reject download before network/consent; `--model-dir` is the
install target, not an attempt to load a missing model during startup.

### Ask failure context

`ask` failures carry an `error.context` object with machine-readable
identifiers and an honest split between the durable local record and the
provider outcome:

- `operation_id`: the BRN-generated or caller-supplied id for this attempt.
- `session_id`: the session established during the run (created by it, or the caller's validated `--session`); `null` when no session was established.
- `recorded_status`: known durable `completed`/`failed`/`interrupted`/`running`;
  null on rejected preflight or unknown final persistence outcome.
- `partial`, `receipt`: exact partial text and safe recorded turn where known.
- `saved`: true only for a known durable receipt; PersistenceFailed is false,
  with null recorded status and an in-memory partial.
- `provider_outcome`: `"unknown"` unless completion was actually confirmed.
  Recorded failure/interruption alone never proves an upstream cancellation.

Resubmitting the SAME operation id returns the recorded state without a new
external submission. After an unknown provider outcome, a NEW operation id is
not known to be safe (it may duplicate the external turn). Nothing is ever
auto-replayed. Usage errors and authority dispatch failures may precede an
attempt and carry no context. An allocated Ask failure carries known identity
even if preflight fails; only the worker's Finished establishes durable outcome.
Legacy history retains its evidence-currentness/receipt shapes and remains
readable offline, including through `brn-flow sessions/history`.

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
