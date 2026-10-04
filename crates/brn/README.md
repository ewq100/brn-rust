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
brn findings capture --file REQUEST.json
brn findings list [--state open|resolved|dismissed|all] [--limit N] [--before UUID]
brn findings show UUID
brn findings inspect UUID
brn findings close UUID --version N --state resolved|dismissed
brn actions complete --file REQUEST.json
brn actions show UUID
brn actions list [--state open|waiting|blocked|completed|all] [--limit N] [--before-created-at-ms N --before-id UUID]
brn proposals create --file DRAFT.json
  brn identity inventory
  brn identity resolve NOTE_UUID
  brn evidence read PATH
  brn identity show PATH
  brn identity prepare PATH --note-id UUID --proposal UUID --title TITLE
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
  brn notes list [--folder FOLDER] [--cursor PATH] [--scope current|source|history|all]
  brn notes show PATH.md [--scope current|source|history|all]
  brn status
  brn search QUERY [--profile keyword|semantic|hybrid] [--limit N] [--scope current|source|history|all]
  brn ask QUESTION [--session UUID] [--operation UUID] [--timeout-seconds N]
  brn conversations list
  brn conversations show SESSION_ID
```

Global long options can appear before or after a command: `--data-dir DIR`
(required, existing and absolute), `--json`, `--vault DIR`, `--credentials-dir
DIR`, `--model-dir DIR`, `--help` and `--version`. Global help/version never open storage.
Within `findings close`, `--version N` names the finding's exact review version.
The vault must be an existing regular absolute directory; first binding is saved.
Credential paths are absolute, current-user-owned, protected and outside Git,
operational storage and the vault. The default is the sibling
`<data-directory-name>.credentials`; workflow saves its non-secret location.
Startup never discovers accounts or models.

`actions show` returns the full retained record and immutable approved origin;
`actions list` defaults to all states and 25 entries, with limits from 1–200.
Use both cursor fields from `next_before` to request the next older page. Creation
time and UUID order remain stable across edits. JSON preserves the exact typed
record; human output quotes strings and terminal controls. These reads need no
vault/provider and refuse pending or uncertain durable changes until reconciled.
Action Create/Replace use the existing exact proposal commands below.
`actions complete --file REQUEST.json` accepts the exact full retained record in
`before` and a fresh `operation_id`. It completes that unfinished Action directly,
preserving its approved origin and all candidate fields except state. Reuse the
same file and operation UUID to replay the full completion receipt after restart;
a changed request conflicts and a stale full baseline refuses. Dashboard controls
follow later. Completion recovery currently uses the macOS file adapter.
For manual acceptance on
a fresh empty data folder, run `actions list --json`, then `actions show` with a
fresh non-nil UUID: expect an empty page and typed NOT_FOUND, with no credential
files or vault writes. Populated read acceptance uses approved creation below.

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

List/show/search default to current knowledge. Optional ordinary frontmatter
`brn_kind: knowledge|source` and `brn_state: current|history` describe saved
classification; absent fields mean current knowledge. Top-level archives always
count as history. `--scope source` includes original sources in any state,
`history` includes historical knowledge and sources, and `all` combines eligible
classified notes. Explicit non-current scopes allow archive paths. Malformed
managed metadata is excluded from scoped queries; `evidence read` still returns
its exact original bytes. Scope selection never changes editor/Save/proposal
destinations or vault bytes.

For manual acceptance, use a fresh synthetic vault/data pair with `current.md`
containing `needle`, `source.md` containing `brn_kind: source` frontmatter and
different original wording, `old.md` containing `brn_state: history`, and
`archive/original.md`. Then run:

```sh
brn notes list --data-dir "$BRN_DATA" --vault "$BRN_VAULT" --json
brn notes list --scope source --data-dir "$BRN_DATA" --json
brn notes list --scope history --data-dir "$BRN_DATA" --json
brn search needle --scope all --data-dir "$BRN_DATA" --json
brn notes show archive/original.md --scope history --data-dir "$BRN_DATA"
```

Default results contain only `current.md`; explicit scopes separate the original
source and historical notes, and plain show preserves the full original text.
JSON labels the selected scope. In this synthetic fixture, change a current
note's class to history while retaining size/mtime, rerun/restart and confirm
current exclusion. Add an invalid `brn_kind` value and confirm query exclusion
while evidence read preserves its bytes. Compare fixture vault bytes before and
after queries. Native controls and actual multilingual inference remain pending.

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

### Tentative review findings

`findings capture` accepts strict regular JSON up to 64 KiB before opening
operational storage. The request names a fresh nonnil `id` and one `origin`:
`{"kind":"identity_ambiguity","note_id":"UUID"}` or
`{"kind":"unresolved_link","path":"note.md","source_sha256":[32 bytes],
"destination":"missing.md","start_byte":123}`. Use the saved hash, destination
and first occurrence start returned by `links show PATH`; do not infer a range
from displayed excerpts. Capture observes actual saved evidence, preserves full
fingerprints and exact quotes, and changes no knowledge or proposal.

List defaults to 25 Open records; limits are 1–100. Full show/inspect output retains
title, summary, original path/UUID, device/inode/byte length/hash and exact quoted
ranges. Human strings use JSON notation to preserve line endings and avoid
ambiguous display. Inspect adds separate fresh Unchanged/Changed/Unavailable
proofs; original evidence is never re-anchored. Unchanged proof does not establish
that the issue still exists. Exact version-one Resolve/Dismiss is a direct queue
operation; correction still requires a separately reviewed approved proposal.
Identical capture/closure replay survives restart and closure. History and closure
work without available vault evidence.

For manual acceptance, create a fresh synthetic managed note with a reference link
to `missing.md` and its used definition. Obtain exact proof with `links show`,
capture it using the strict shape above and inspect both retained quote ranges.
Restart and compare full show output. Replace the saved file with the same bytes
under a new inode and inspect: Changed must retain the original quotes/hash and
show the different inode. Remove only that synthetic note and inspect:
Unavailable must still show the original finding. Close using its actual version,
replay the same request, and confirm it stays closed. A competing old-version
closure must refuse. Duplicate a synthetic UUID across two distinct saved paths
and capture `identity_ambiguity`; compare both proofs and unchanged Markdown.
Run `proposals list` to confirm capture created no proposal. Native queue and owner
acceptance remain separate qualification steps.

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

Action-only input uses `changes: []`, `sources: []` and
`action_changes: [{kind: "create", id: ACTION_UUID, data: ActionData}]` without a
vault. Replace uses `{kind: "replace", before: ActionRecord, data: ActionData}`;
copy the complete `actions show --json` record into `before`. All 14 candidate
fields remain exact, and a source/Markdown member binds the exact vault. Combined
drafts retain the 64-member/8 MiB bounds. Person/project/source/thread references
are managed note UUIDs with captured or same-draft evidence. Dependencies and
parent graphs are checked separately. Completed Actions cannot be changed through
proposal members. Direct explicit completion uses `actions complete` and the
exact full retained baseline.

`edit` and `rewrite-result` accept `{expected: {id, version}, title, texts}` plus
the ordered `action_data` array when the proposal has Action members.
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
New arrivals in that group are never included automatically. File/source-free
Action members may be grouped with proposals bound to the same vault; different
bound vaults cannot share one native confirmation.

#### Manual Action acceptance

On macOS, create a fresh explicit data directory and use no vault. Save this request
as `action.json` outside the repository, then run
`brn proposals create --file action.json --data-dir DATA --json`:

```json
{"id":"a1111111-1111-4111-8111-111111111111","group_id":null,"session_id":null,
 "title":"Review synthetic follow-up","changes":[],"sources":[],
 "action_changes":[{"kind":"create","id":"a2222222-2222-4222-8222-222222222222",
 "data":{"title":"Tähtaeg 🦀","description":"Exact synthetic review.\r\n",
 "state":"waiting","owner":"Synthetic owner","related_person":null,
 "related_project":null,"sources":[],"thread":null,"due_on":"2026-10-10",
 "follow_up_on":"2026-10-08","dependencies":[],"parent":null,"follows_up":null,
 "priority":null}}]}
```

`actions list --data-dir DATA --json` must be empty before approval. Inspect the
draft with `proposals show`, then approve its actual review version using a fresh
operation UUID. `actions show a2222222-2222-4222-8222-222222222222 --data-dir DATA
--json` must retain the exact data, immutable creating proposal and Waiting clock.
Repeat that exact approval and read after restart: the receipt and Action stay
unchanged. For Replace, copy the complete current record into `before`, change
candidate title/state and use a new proposal/operation UUID. Reusing an old complete
baseline must refuse and preserve the current Action. No account or vault write
is needed.

To complete the synthetic Action, retain its current unfinished `actions show
--json` envelope as `action-before.json`. Create `complete.json` outside the
repository from that envelope's full `data` record:

```sh
python3 - <<'PYCOMPLETE'
import json, uuid
with open("action-before.json", encoding="utf-8") as source:
    before = json.load(source)["data"]
with open("complete.json", "w", encoding="utf-8") as target:
    json.dump({"operation_id": str(uuid.uuid4()), "before": before}, target, ensure_ascii=False)
PYCOMPLETE
brn actions complete --file complete.json --data-dir DATA --json
brn actions complete --file complete.json --data-dir DATA --json
```

Both invocations must return the identical full receipt (`request` and `after`).
`after` retains the origin and other data, increments version once, has state
`completed`, clears the Waiting clock, and records the completion timestamp.
Changing `before` while retaining this operation UUID must refuse without another
mutation. A fresh operation using the old baseline must also refuse. The vault
stays untouched and credential files remain absent. Close the CLI before opening
this same directory in the desktop.

For a manual check, create a proposal for `new.md`, add a comment, inspect the
version with `show`, and approve that version with a fresh operation UUID. Compare
the exact file bytes, confirm `show` has state `applied` and no comments, then
repeat `approve` and `reconcile` with the same UUID to confirm the same receipt.

Typed JSON is decoded before workspace admission; encoded input is bounded to
64 MiB, with stricter domain limits of 1 MiB per note and 8 MiB aggregate review
work. Nonregular inputs refuse without blocking. Native full review/edit/comments
use the same records, exact approval and activity; native Undo/repair and initial
Markdown creation use the existing shared workflow.

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

### Managed note identity

`identity show PATH` inspects exact saved current Markdown and reports its
managed `brn_id` or explicit absence. Malformed/duplicate fields or unsupported
root metadata layouts return an error. It never substitutes the filename/hash
for a stable ID. `identity prepare` returns the entire ordinary Replace request
with caller-chosen nonnil note/proposal UUIDs, complete proposed bytes and exact
before/source proofs. It changes no note or review record. Existing identities
refuse reassignment. Unrelated metadata, BOM, line endings and source wording
are preserved; an incomplete ambiguous header refuses assignment.

In fresh explicit disposable data/vault directories, create a synthetic note
and run the following with distinct fresh UUIDs and absolute fixture paths:

```sh
brn identity show note.md --data-dir "$BRN_DATA" --vault "$BRN_VAULT"
brn identity prepare note.md --note-id "$BRN_NOTE_ID" --proposal "$BRN_PROPOSAL_ID" \
  --title "Assign stable note identity" --data-dir "$BRN_DATA" > "$BRN_REQUEST_FILE"
brn proposals create --file "$BRN_REQUEST_FILE" --data-dir "$BRN_DATA"
brn proposals show "$BRN_PROPOSAL_ID" --data-dir "$BRN_DATA"
brn proposals approve "$BRN_PROPOSAL_ID" --review-version 1 \
  --operation "$BRN_APPROVAL_ID" --data-dir "$BRN_DATA"
brn identity show note.md --data-dir "$BRN_DATA"
brn proposals undo "$BRN_APPROVAL_ID" --operation "$BRN_UNDO_ID" --data-dir "$BRN_DATA"
```

Inspect the complete request before creation/approval; if review changes, use its
actual version. Preparation/creation leave the note unchanged. Approval installs
the shown ID; restart preserves it, and Undo restores the exact original bytes.
Changing a source after preparation refuses creation.

`identity inventory` freshly reads visible Markdown across current and archive
paths, reporting unmanaged notes, duplicate UUID paths and inspection issues.
`identity resolve NOTE_UUID` returns `unique`, `absent`, `ambiguous` or `incomplete`
with observed matches/issues. Unreadable or malformed evidence cannot establish
uniqueness/absence. Resolution never substitutes a filename/hash, chooses among
duplicates, mints IDs or repairs notes; same-size/retained-timestamp ID edits are
observed. These are saved evidence observations; later changes still require exact
source-version checks. `evidence read PATH` explicitly reads complete source/history
text, including archived notes, with the ordinary file/UTF-8/size protections.
Default current `notes show`, identity assignment, editor and proposal destinations
retain archive refusal. All scopes respect unresolved Save/application fences.
Durable citations use the provenance commands below.

For manual acceptance, use only a fresh synthetic vault/data pair. Put the same
managed `brn_id` into `current.md` and `archive/source.md` with different original
wording, and leave `unmanaged.md` without metadata. Run:

```sh
brn identity inventory --data-dir "$BRN_DATA" --vault "$BRN_VAULT" --json
brn identity resolve "$BRN_NOTE_ID" --data-dir "$BRN_DATA" --json
brn evidence read archive/source.md --data-dir "$BRN_DATA" --json
brn notes show archive/source.md --data-dir "$BRN_DATA" --json
```

The inventory lists both duplicate paths and the unmanaged note; resolution is
`ambiguous`. Explicit evidence preserves the archived wording/BOM/line endings;
the last current-only command refuses. In that synthetic fixture, change the
second UUID without changing byte count or mtime, rerun resolution and restart;
the first ID is now `unique`. Add malformed managed metadata in another archived
note: resolution becomes `incomplete`. Compare all fixture vault bytes before
and after queries; queries create no review/editor work and change no notes.

### Durable source provenance

`provenance capture --file REQUEST.json` captures an exact saved source range by
managed UUID and expected full-file SHA-256. Its strict request is
`{note_id, expected_sha256, start_byte, end_byte}`: hashes are 32-byte arrays and
ranges are UTF-8 byte offsets. The result includes the complete `{citation,
source}` proof. Missing, duplicate or incompletely inspected identities refuse;
changed hashes and invalid character boundaries refuse without guessing.

`provenance prepare --file REQUEST.json` accepts `{path, proposal_id, title,
citations}` with the exact captured `citation` objects. It returns a complete
ordinary Replace draft that adds `brn_provenance` to Markdown while preserving
existing citations, unrelated metadata and body bytes. Capture/preparation create
no editor or review records and change no notes. Review and approve that complete
draft through the existing proposal commands. Request files must be regular,
strict typed JSON up to 8 MiB; invalid input refuses before storage opens.

`provenance show PATH` reads current, source or archived Markdown and returns
saved quotes plus fresh `matched`, `changed`, `absent`, `ambiguous` or `incomplete`
source observations. A source move follows its unique UUID; changed or missing
sources retain the original quote. Citations live in the approved note, so index
rebuild and removal of operational sessions cannot erase them.

For manual acceptance, use fresh synthetic data/vault directories. Create
`knowledge.md` with a short interpretation and `archive/source.md` containing
exactly the following LF text (including the final newline):

```text
---
brn_id: 11111111-1111-4111-8111-111111111111
brn_kind: source
---
Original õ
```

Run `identity inventory` to obtain that source's full `sha256`. Write a capture
request using its UUID, that hash and `start_byte: 70, end_byte: 82`; capture must
return the exact `Original õ\n` quote. Write a preparation request naming
`knowledge.md`, a fresh proposal UUID/title and that captured citation, then run:

```sh
brn provenance capture --file "$BRN_CAPTURE_FILE" --data-dir "$BRN_DATA" --vault "$BRN_VAULT" --json
brn provenance prepare --file "$BRN_PROVENANCE_FILE" --data-dir "$BRN_DATA" > "$BRN_DRAFT_FILE"
brn proposals create --file "$BRN_DRAFT_FILE" --data-dir "$BRN_DATA"
brn proposals show "$BRN_PROPOSAL_ID" --data-dir "$BRN_DATA"
brn proposals approve "$BRN_PROPOSAL_ID" --review-version 1 --operation "$BRN_APPROVAL_ID" --data-dir "$BRN_DATA"
brn provenance show knowledge.md --data-dir "$BRN_DATA" --json
```

Inspect the full draft before approval and use the actual current review version.
Preparation leaves both files unchanged; approval changes only the shown target.
Restart and remove only the disposable fixture's `index.sqlite`: show retains the
same quote and reports `matched`. In that fixture, move the source and confirm its
new path; change its bytes or remove it and confirm `changed`/`absent` while the
saved quote remains exact. Repeat an old capture request after a source edit and
confirm refusal. Owner acceptance and later native provenance presentation remain
separate from these automated/process checks.

### Saved note links

`links show PATH` inspects saved current/source/history Markdown without opening
an editor or changing notes. CommonMark inline and reference links retain exact
occurrence byte ranges/quotes and the actually used reference definition. Metadata,
code, images and HTML attributes do not become links. Inspection refuses more than
4096 links or over 4 MiB of returned destination/quote bytes rather than truncating.

Contained relative `.md` paths resolve from the source folder; percent decoding
occurs once, `+` stays literal, and encoded path separators are refused. Query and
fragment text do not become filename bytes. `brn://note/UUID` links use a nonnil
managed UUID and follow unique saved notes through renames. No filename/title
guess or network request occurs. Results expose full source UUID/hash and identity
outcome, `resolved`, `absent`, `unmanaged`, `ambiguous`, `incomplete`, `changed`,
`external`, `non_note` or `unsupported` target outcomes, observed matches and
inspection issues. A resolved target alone does not establish a unique source.
These are fresh saved observations, not a transactional vault snapshot or new
AI-approved relationship. Durable changes still use complete ordinary proposals.

For manual acceptance, use separate fresh synthetic data/vault directories. Give
`current.md` and `source.md` different `brn_id` UUIDs. In `current.md`, add a normal
`[path](source.md)` link and `[stable](brn://note/<source UUID>)`, then run:

```sh
brn links show current.md --data-dir "$BRN_DATA" --vault "$BRN_VAULT" --json
```

Both targets must resolve with exact link quotes. Rename only that synthetic
`source.md`: the path link becomes absent and the UUID link reports its new path.
Duplicate the renamed synthetic source: the UUID link becomes ambiguous, retaining
both observed matches. Delete only that fixture's disposable `index.sqlite` and
restart; results must rebuild from Markdown and original note bytes stay exact.

`links prepare --file REQUEST.json` prepares one additive stable UUID link from
an eligible current note to an already identified saved target. Its strict request
is `{path, target_note_id, expected_target_sha256, proposal_id, title, label}`;
the target hash is a 32-byte array from fresh identity inspection. The label is
literal single-line text up to 512 bytes, escaped for Markdown. Both identities
must be unique, and the selected target may be source/history evidence. Self-links,
already resolved relationships and stale or ambiguous targets refuse.

Preparation returns an ordinary complete Replace draft with exact consumer/target
source proofs. It preserves the entire original byte prefix and creates no editor,
review or vault change. Explicitly create, inspect and approve the returned draft
through the existing proposal commands. Fresh approval also checks introduced
stable-link target bindings after review edits/Rewrite. Unchanged historical links
and exact Undo retain their existing authority. Request files must be regular,
strict typed JSON up to 8 MiB; invalid input refuses before storage opens.

For manual acceptance, use a fresh synthetic current note with a managed UUID and
a different managed target in `archive/source.md`. Obtain its UUID/hash using
`identity inventory`, then write a request naming the current note, a fresh
proposal UUID/title and a label such as `Original õ [evidence]`. Run:

```sh
brn links prepare --file "$BRN_LINK_FILE" --data-dir "$BRN_DATA" --vault "$BRN_VAULT" > "$BRN_DRAFT_FILE"
brn proposals create --file "$BRN_DRAFT_FILE" --data-dir "$BRN_DATA"
brn proposals show "$BRN_PROPOSAL_ID" --data-dir "$BRN_DATA"
brn proposals approve "$BRN_PROPOSAL_ID" --review-version 1 --operation "$BRN_APPROVAL_ID" --data-dir "$BRN_DATA"
brn links show current.md --data-dir "$BRN_DATA" --json
```

Inspect the full draft before approval and use the actual review version. Confirm
preparation left both notes unchanged; approval preserved the current note's exact
prefix and appended one resolved UUID link with literal label wording. Restart and
remove only the fixture's `index.sqlite`; the link still resolves. In a separate
fresh fixture, edit or duplicate the selected target after capturing its hash and
confirm preparation refuses without a proposal. Native controls, graph canvas
and owner acceptance remain separate qualification steps.

### Derived relationship pages

`relationships list` freshly observes saved notes and records rebuildable directed
edges in the disposable index. It returns `scope`, `offset`, `total`, `edges`,
inspection `issues` and duplicate identities. Each edge names exact source/target
UUIDs, paths and full-file hashes, with UTF-8 byte ranges and quotes. Repeated
proofs for the same endpoints and origin are combined. `explicit_link` describes
saved Markdown links; `inferred_provenance` is a separately labelled candidate
from a saved citation that still matches its source. Neither creates an approved
durable relationship or changes a note.

The default scope is current, offset 0 and limit 50. `--limit` accepts 1–200;
`--offset` is a nonnegative integer. Both endpoints must fit the selected scope
before pagination. Use `--scope all` for connections across current/source/history.
Unmanaged, ambiguous or incompletely inspected endpoint identities never become
guessed edges. Every query rereads saved evidence, so source-unchanged target edits
and moves are observed. Rebuilding/removing the derived index loses no knowledge.

For manual acceptance, use a fresh copy of the preceding synthetic
`current.md`/`source.md` link fixture and create `archive/source.md` with
a distinct managed UUID and a UUID link to that target. Then run:

```sh
brn relationships list --data-dir "$BRN_DATA" --vault "$BRN_VAULT" --json
brn relationships list --scope all --offset 0 --limit 1 --data-dir "$BRN_DATA" --json
brn relationships list --scope all --offset 1 --limit 1 --data-dir "$BRN_DATA" --json
```

Current returns one combined explicit edge with exact proofs. All also includes
the archived source connection; each page reports the same matching total.
Restart and remove only the disposable fixture's `index.sqlite`, then confirm the
same page. Edit only the target and confirm its fresh hash; move it and confirm
UUID links follow the new path. Duplicate its UUID or add malformed managed
identity metadata and confirm affected relationships are excluded with diagnostic
observations. Compare saved note bytes and run `proposals list`: queries changed
no knowledge and created no proposal. Native views and owner acceptance remain
separate qualification steps.

## Output contract

`--json` prints one schema-1 envelope on stdout. Human errors go to stderr;
non-JSON result text stays on stdout. The shape is independent of data-directory
history:

```json
{"schema_version": 1, "command": "search", "ok": true,
 "data": {"query": "synthetic", "scope": "current", "hits": [], "keyword_only": true}}
```

```json
{"schema_version": 1, "command": "notes.show", "ok": false,
 "error": {"code": "VAULT_NOT_BOUND", "message": "a vault is required"}}
```

Status returns version/data directory, `mode: "simple"`, vault/model state and
native-retrieval capability. Notes return `{notes, next_cursor, scope}` or `{path,
text, scope}`. Search labels scope; hits contain path, exact byte range, quote and score.
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

`conversations list` includes known `created_at_ms` and nullable
`last_activity_at_ms`; `conversations show UUID` includes nullable turn
`started_at_ms`/`finished_at_ms`. Values are Unix milliseconds. Historical unknown
times are null, and startup-interrupted turns retain their recorded start with
unknown finish. Inspecting history or replaying a completed request never refreshes
session activity. Archive/Restore/Delete remain later session lifecycle work.

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
