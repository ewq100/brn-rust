# brn

Agent-facing CLI over [brn-workflow](../brn-workflow/README.md). Every command
uses AppWorker, sharing the desktop's application owner and durable behavior.
A data directory has one owner at a time.

[Entry point](src/main.rs), [parsing/output](src/cli/mod.rs),
[application dispatch](src/cli/library.rs), [editor adapter](src/cli/editor.rs)
and [error categories](src/cli/error.rs).

## Commands

```text
brn edit open PATH
  brn edit recover PATH --baseline UUID --expected-generation N --generation N --file F
  brn edit save PATH --baseline UUID --expected-generation N --generation N --file F --operation UUID [--copy PATH]
  brn edit reload PATH --baseline UUID --expected-generation N --observed-file F [--discard]
  brn edit list
  brn edit reconcile OPERATION
  brn ai connect chatgpt|copilot [--timeout-seconds N]
  brn ai disconnect chatgpt|copilot
  brn ai status
  brn ai models chatgpt|copilot [--timeout-seconds N]
  brn ai select --provider chatgpt|copilot --model MODEL
  brn models download --approve-download [--model-dir DIR] [--timeout-seconds N]
  brn notes list [--folder FOLDER] [--cursor PATH]
  brn notes show PATH.md
  brn status
  brn search QUERY [--profile keyword|semantic|hybrid] [--limit N]
  brn ask QUESTION [--session UUID] [--operation UUID] [--timeout-seconds N]
  brn conversations list
  brn conversations show SESSION_ID
```

Global long options can appear before or after a command: `--data-dir DIR`
(required, existing and absolute), `--json`, `--vault DIR`, `--credentials-dir
DIR`, `--model-dir DIR`, `--help` and `--version`. Help/version never open storage.
The vault must be an existing regular absolute directory; first binding is saved.
Credential paths are absolute, current-user-owned, protected and outside Git,
operational storage and the vault. The default is the sibling
`<data-directory-name>.credentials`; workflow saves its non-secret location.
Startup never discovers accounts or models.

`status` and history can initialize operational storage without a vault.
Note/search commands require a bound vault. All startup refuses old database,
sidecar or mixed-authority markers with `WORKSPACE_MODE_CONFLICT`, preserving
those files. Existing data is not migrated or inspected. Superseded commands
and flags are usage errors before opening storage.

Notes use visible contained vault-relative Markdown paths. List returns sorted
200-row cursor pages; show preserves exact UTF-8 bytes. Search defaults to
hybrid with 10 results (limit 1–50). Without an installed model, all profiles
explicitly report `keyword_only: true`.

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

## Output contract

`--json` prints one schema-1 envelope on stdout. Human errors go to stderr;
non-JSON result text stays on stdout. The shape is independent of data-directory
history:

```json
{"schema_version": 1, "command": "search", "ok": true,
 "data": {"query": "synthetic", "hits": [], "keyword_only": true}}
```

```json
{"schema_version": 1, "command": "notes.show", "ok": false,
 "error": {"code": "VAULT_NOT_BOUND", "message": "a vault is required"}}
```

Status returns version/data directory, `mode: "simple"`, vault/model state and
native-retrieval capability. Notes return `{notes, next_cursor}` or `{path,
text}`. Search hits contain path, exact byte range, quote and score.
Conversations return `{conversations}` or `{session_id, historical: true,
turns}`. Turn/Ask records contain operation/session/provider/model,
question/answer/status/error; provider thread IDs are not resume inputs.

`ai status` returns actual local account statuses, selection and selection_error.
An unavailable or malformed saved selection returns status successfully with
selection null and a safe typed diagnostic; status never selects a fallback or
changes the saved selection. Credentials existing locally do not prove upstream
validity. Download reports installed only after activation.

Exit codes: `0` success, `1` operational failure, `2` usage, `124` deadline,
`130` interrupted. Categories are typed; message wording never decides a code.
An error may include additive `context`; consumers must handle its absence.
A completed result with failed stdout delivery exits 1 and reports
`OUTPUT_DELIVERY_ERROR` on stderr without rewriting durable state. A closed pipe
exits quietly with 0.

## Subscription actions and Ask

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
## Build and verification

```sh
cargo +1.98.1 build -p brn --locked --offline
cargo +1.98.1 test -p brn --locked --offline
cargo +1.98.1 build -p brn --features native-retrieval --locked --offline
```

The default build is keyword-only; native-retrieval enables existing local
embedding/model-install support. Tests use synthetic disposable directories
outside Git and never make live account or model calls. macOS editor tests
exercise the shared coordinator; native acceptance remains a separate check.
See [verification](../../docs/development/verification.md),
[architecture](../../docs/architecture/overview.md) and
[invariants](../../docs/architecture/invariants.md).
