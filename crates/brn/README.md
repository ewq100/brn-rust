# brn

Agent-facing CLI over [brn-workflow](../brn-workflow/README.md). Every command
uses AppWorker, sharing the desktop's application owner and durable behavior.
A data directory has one owner at a time.

[Entry point](src/main.rs), [parsing/output](src/cli/mod.rs),
[application dispatch](src/cli/library.rs), [editor adapter](src/cli/editor.rs)
[proposal adapter](src/cli/proposals.rs) and [error categories](src/cli/error.rs).

## Commands

```text
brn activity list [--limit N] [--before OPERATION_UUID]
brn proposals create --file DRAFT.json
  brn proposals list [--group UUID]
  brn proposals show PROPOSAL_ID
  brn proposals edit --file EDIT.json
  brn proposals rewrite --file REQUEST.json
  brn proposals rewrite-status JOB_UUID
  brn proposals rewrite-result --file EDIT.json
  brn proposals comment --file COMMENT.json
  brn proposals comment-update --file COMMENT.json
  brn proposals comment-remove PROPOSAL_ID --review-version N --comment UUID
  brn proposals reject PROPOSAL_ID --review-version N
  brn proposals approve PROPOSAL_ID --review-version N --operation UUID
  brn proposals reconcile OPERATION_UUID
  brn proposals approve-group --file APPROVALS.json
  brn proposals applies
  brn proposals undo-preview TARGET_OPERATION_UUID --operation NEW_UUID [--member INDEX]
  brn proposals undo TARGET_OPERATION_UUID --operation NEW_UUID
  brn proposals restore-trash TARGET_OPERATION_UUID --member INDEX --operation NEW_UUID
  brn proposals repair-preview OPERATION_UUID
  brn proposals repair --file REQUEST.json
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
  brn ai effort [low|medium|high]
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

`ai effort` reads the saved explicit reasoning choice locally; its value is null
until chosen. `ai effort low|medium|high` saves that choice, which is also shown
by `ai status`. Fresh Ask requires both an explicit provider/model and effort;
it refuses a missing choice before admitting a turn or accessing an account.
The admitted turn freezes those choices. Later setting changes do not alter
its request, receipt or replay; historical turns with unknown effort retain null
rather than gaining a default. Getter, setter, status and replay need no provider
call. There is no fallback to another provider, model or effort.

For an offline check, use a fresh data directory, run `ai effort` to confirm null,
set `ai effort high`, restart the CLI and confirm `ai effort` and `ai status`
report high. Select ChatGPT `gpt-5.5` in a separate synthetic directory without
choosing effort, then run a new Ask and confirm `AI_SELECTION_REQUIRED` with
no saved turn. Actual provider calls require current explicit authorization.

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

### Approved activity

`activity list` shows important approved durable changes as a readable history.
Entries include the approved title, summary, affected paths/kinds and operation,
proposal, group and session identities. The time is recorded approval admission,
not an exact completion time. Note bodies, comments, proofs and transcripts are
excluded. JSON returns `{entries, next_before}` in an `activity.list` envelope.

The default page contains up to 20 entries, with `--limit` accepting 1–100.
Pass `next_before` as `--before` to fetch the exclusive older page. Unknown or
unapproved cursor UUIDs refuse. Drafts, refused and uncertain changes do not claim
completion. Historical entries remain available after later note edits and restart.
Undo entries name their source operation; a scoped Trash restore also identifies
the original zero-based member index.

For a manual check, approve two synthetic Create proposals, run `activity list
--limit 1`, fetch the older page using its cursor, then change a note externally
and confirm that the recorded activity stays the same. An empty history explains
that no approved durable changes exist.

### Typed review foundation

`proposals create` accepts a typed `DraftRequest` JSON file. Workflow captures
trusted bindings; Replace/Trash require the exact `expected` file fingerprint
returned by `edit open`. Create requires an unused visible Markdown path. Optional
`group_id` groups independent proposals; optional `session_id` names an existing
conversation. Drafts keep exact bytes and never write a vault file.

```json
{"id":"11111111-1111-4111-8111-111111111111","group_id":null,"session_id":null,
 "title":"Review a note","changes":[{"kind":"create","path":"new.md","text":"Draft text"}],"sources":[]}
```

`edit` and `rewrite-result` accept `{expected: {id, version}, title, texts}`.
`texts` supplies one full string per Create/Replace and null per Trash, preserving
bound destinations/baselines. `rewrite-result` imports a captured result; it does
not invoke AI. Any newer edit/comment/rejection makes the old version stale.
`comment`/`comment-update` accept `{expected, comment: {id, text, target}}`; targets
are `{kind: "proposal"}` or `{kind: "text", anchor: {change_index, start, end,
quote}}` with byte offsets and exact UTF-8 quote. Changed target content marks the
anchor unresolved; explicit update may reattach it. No guessed positioning.
`reject` preserves comments. List/show return full versioned records across restarts.

`approve` applies the exact reviewed version as one proposal. It returns a receipt
with `applied`, `not_applied` or `uncertain` outcome; only `applied` confirms all
members. Applied approval removes temporary comments. A stale version or changed
source, destination or parent refuses application. `reconcile` inspects a recorded
operation without repeating its writes; `applies` lists its durable journals.
Reusing an operation UUID with its exact request returns the recorded outcome;
a different request fails `OPERATION_CONFLICT`.

`approve-group` accepts `{group_id, approvals: [{operation_id, expected: {id,
version}}]}`. It approves only the explicit captured members, in order, and stops
at the first refusal or uncertainty. Its result contains individual receipts and
an optional `stopped` failure; inspect these fields even when the CLI exits 0.
New arrivals in that group are never included automatically.

For a manual check, create a proposal for `new.md`, add a comment, inspect the
version with `show`, and approve that version with a fresh operation UUID. Compare
the exact file bytes, confirm `show` has state `applied` and no comments, then
repeat `approve` and `reconcile` with the same UUID to confirm the same receipt.

Typed JSON is decoded before workspace admission; encoded input is bounded to
64 MiB, with stricter domain limits of 1 MiB per note and 8 MiB aggregate review
work. Nonregular inputs refuse without blocking. Native full review/edit/comments
use the same records, exact approval and activity; native Undo/repair and initial
proposal creation remain Stage 4 work.

### Owned AI Rewrite

`rewrite --file REQUEST.json` captures the complete identified proposal and its
temporary comments, then requests a full replacement suggestion using the
explicit provider, model and reasoning effort. It accepts:

```json
{"id":"22222222-2222-4222-8222-222222222222",
 "expected":{"id":"11111111-1111-4111-8111-111111111111","version":1},
 "selection":{"provider":"chatgpt","model":"gpt-5.5"},
 "effort":"high","generation":0}
```

Use a fresh non-nil job UUID and the exact review version returned by `show`.
Effort is `low`, `medium` or `high`; generation is an unsigned value used to
correlate worker replies. The selected provider/model must be qualified and connected;
there is no fallback. Complete captured input and typed output are bounded;
oversized or invalid results refuse rather than importing partial text.

The CLI waits through Started and read-tool progress and returns the safe job
receipt after durable completion. Inspect the resulting full proposal with
`show`; newer edits, comments or rejection prevent a late result from replacing
that work. Rewrite never approves or writes Markdown. Job receipts contain
bindings, hashes and status, excluding prompt, comment and provider output bodies.

`rewrite-status JOB_UUID` returns the typed recorded job, or null when absent,
without a provider call. Repeating an exact request returns its historical result;
an already running request returns `OPERATION_CONFLICT` immediately with its safe
`running` job receipt, including when the caller supplies a new generation. It does
not start another provider call or wait for the original request's completion.
Changing proposal/version, selection or effort under the same UUID fails
`OPERATION_CONFLICT`. Startup marks interrupted running jobs as `interrupted`
and never retries them. Failed, interrupted and stale commands retain their safe
job receipt in structured error context. Ctrl-C or the five-minute deadline
cancels the owned job and joins local work; a confirmed completed receipt wins
over a later interruption.

For a synthetic offline check, create the draft in the preceding example in a
fresh data directory and empty vault, run `show`, and query `rewrite-status` with
a fresh UUID to confirm null. Submit an invalid effort such as `maximum` in a
fresh directory and confirm a usage error before storage opens. Actual Rewrite
requires current live-provider authorization: after signing in, submit the typed
request for a synthetic draft with a comment, inspect the complete revised
proposal and retained comments, then repeat the request and verify the same job
receipt. Approve remains a separate explicit operation.

### Explicit Undo and Trash restore

`undo-preview` returns the complete typed `{draft, binding}` inverse without
installing files or admitting a new proposal. Supply the original Applied
operation UUID and an explicit new operation UUID. Whole Undo reverses every
original member: Create becomes Trash, Replace restores retained original bytes,
and Trash becomes Create. `undo` applies that whole inverse through the shared
workflow. Changed destinations, retained originals or parents refuse; no current
file is overwritten by guessing. Original external source references remain
historical and are not copied into the inverse.

`undo-preview --member INDEX` previews one original Trash member;
`restore-trash --member INDEX` restores only that member. Indexes are zero-based
and must identify an original Trash change, allowing restore after other notes
from the same mixed proposal have later edits. Both operations require an
explicit `--operation`, using a new UUID for the initial attempt. Nil/equal UUIDs
and indexes outside 0–63 are usage errors
before storage opens. Repeating the same source/scope and operation returns its
recorded result without applying files again. Reusing that UUID for another
source/scope fails `OPERATION_CONFLICT`. Reconcile an interrupted operation with
`proposals reconcile`; a new UUID does not resume its writes.

For a manual check, approve a disposable mixed Create/Replace/Trash proposal,
preview its original operation with a fresh inverse UUID, run `undo` with the same
UUID, and compare exact restored BOM/CRLF bytes. Edit a restored note externally,
then repeat `undo` and confirm the same receipt preserves the newer text. In a
separate mixed approval, edit another member, preview the original Trash index
with a fresh UUID, and use `restore-trash` to verify that only the trashed note
returns. An occupied Trash destination must refuse without changing its bytes.

### Explicit interrupted-operation repair

`repair-preview` returns the full approved draft, exact current Before/Applied
member phases and an opaque 32-byte capture hash for an unresolved operation.
It changes no vault files or review work. Unknown occupants or missing/changed
proofs refuse preview. Inspect the complete proposal and phases before choosing
Finish or Restore.

`repair --file REQUEST.json` accepts `{id, operation_id, expected, direction}`.
Use a fresh repair attempt UUID for `id`, the original approval/Undo UUID for
`operation_id`, copy the exact `expected` array from preview and choose lowercase
`finish` or `restore`. Finish applies the remaining already-approved members;
Restore returns applied members to their originals. Changed capture or protected
editor work refuses without guessing or overwriting another file. Finish checks
reviewed sources again; Restore preserves unrelated later source changes.
The result is `{id, operation_id, direction, outcome}`; inspect `outcome`, since
an interrupted attempt remains fenced until the whole operation settles.

Typed requests and distinct non-nil UUIDs are checked before opening storage.
Repeating the same repair UUID/request returns its historical result without
another namespace attempt. Changed direction, source or hash under that UUID
fails `OPERATION_CONFLICT`. For a later explicit attempt, obtain a fresh preview
and use a fresh repair UUID. Preview, startup and reconciliation never resume
interrupted namespace writes automatically.

For a manual check with a disposable interrupted mixed proposal, run
`proposals repair-preview OPERATION_UUID --json`, review its approved changes,
write the four request fields to a regular JSON file and run
`proposals repair --file REQUEST.json --json`. Compare the exact destination
bytes and retained originals for Finish or Restore. Then change a destination
externally, repeat the same request and confirm the recorded receipt leaves the
newer bytes intact. Native repair presentation and power-loss qualification
remain separate.

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
