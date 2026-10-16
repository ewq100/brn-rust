# BRN Threads CLI

The CLI uses the same checked SQLite service as the desktop. Supply a fresh absolute data directory; other BRN formats refuse before opening. See `brn --help` for the exact commands.

```sh
brn --data-dir /absolute/fresh/threads init
brn --data-dir /absolute/fresh/threads delegation --choice review-first
brn --data-dir /absolute/fresh/threads note-create --title Harbor --text 'Working context'
brn --data-dir /absolute/fresh/threads thread-start --title 'Harbor handoff'
```

Save uses persistent session/base/generation checks. Review apply and Undo are concrete owner actions; neither bypasses expected versions or open edit guards. Intake offers useful information or a protected full readable note. Raw originals remain external. Export and backup require fresh absolute destinations.

New workspaces wait for an explicit delegation choice. `delegation --choice maintain` enables ordinary-note maintenance; `delegation --choice review-first` requires review for standing changes. Both preserve provider settings and fence active runs. Manual work remains available before choosing. Pause/Resume temporarily suspends the selected mode; Resume cannot choose for you. Settings output includes `delegation_chosen`.

The selected provider, model, reasoning effort and credential location are explicit Settings. Codex uses the normal selected subscription account in memory. There is no paid fallback or external message delivery.

Implementation: [entry point](src/main.rs), [shared app](../brn-threads-app/src/lib.rs), [checked core](../brn-threads-core/README.md). [Threads acceptance evidence](../../docs/archive/threads-rebuild/evidence.md) distinguishes code, offline/live checks, native interaction and owner acceptance.

## Agents and scripts

Coding agents, tests and diagnostic scripts should use the machine contract: put `--json` first.

```sh
brn --json --data-dir /absolute/fresh/threads note-create --key plan-1 --title Plan --text-file /abs/plan.md
generate-plan | brn --json --data-dir /absolute/fresh/threads note-create --title Plan --text-file -
brn --json --help    # machine description: commands, options, required options, outcomes, retry classes, error codes
```

`--json` is recognised only as the first argument; `--data-dir` must come next. Without `--json` the CLI behaves as before (see Compatibility). `brn --json --help` prints one JSON line, exits 0 and opens nothing. It is generated from the same command table the parser uses.

### Envelope

stdout carries exactly one line, a compact JSON object. Every key is always present; `null` or `[]` means not applicable. Key order is not guaranteed.

| Field | Meaning |
|---|---|
| `contract` | `2`. Additive fields keep the number; a removal or change of meaning increments it (see "Changes from contract 1"). |
| `command` | The command as invoked (`resolve` stays `resolve`). Null when parsing failed before a known command was read: `missing_data_dir`, `invalid_data_dir`, `missing_command`, `unknown_command`, and a non-UTF-8 `malformed_option`. |
| `ok` | `true` exactly when `exit == 0`. |
| `status`, `exit` | One status, one-to-one with the exit code (table below). |
| `outcome` | Domain outcome (`Read`, `Applied`, `Saved`, `Updated`, `Discarded`, `Dismissed`, `Ignored`, `Stale`, `GenerationMismatch`, `Closed`, `Deferred`, `NeedsReview`, `Superseded`, `Completed`, `Interrupted`, `Failed`), or null. Null whenever `error` is set. |
| `operation` | The operation this invocation applied, replayed or (for `undo`) prepared, per the outcome table below. |
| `key` | Echo of `--key`; null when absent or not matching the key pattern. |
| `retry` | What an identical retry of this exact invocation does to state (Retries below). Always `refuse` for `usage` and `rejected`. Never null. The status decides whether to retry; `retry` only says what a retry does. |
| `changed` | `{id, kind, version, archived, created}` per write this invocation applied or replayed (`kind` is the record type; `created` is true for a new record). Only commands with a receipt rule fill it; `[]` is not evidence that nothing was written (`init`, `begin-edit`, `knowledge-check --seed-chat yes`, `ask` and useful intake write state but report `[]`, and opening a fresh data directory initialises it). |
| `conflict` | Non-null exactly when `status` is `stale` or `deferred`: `{id, kind, version, archived, open_edit}` per record the outcome names, read after the outcome. `open_edit` is true when any open edit session exists for that note, including the one that produced the outcome. A record that no longer exists has null `kind`, `version`, `archived`. Session IDs are never included. If the lookup fails, `[]`. `intake --intent useful` always reports `[]`; its run is `result.run`. |
| `result` | Exactly what default mode prints on stdout for the same invocation, or null. |
| `error` | `{code, message, details}` when the command failed without a result. `message` is human text, not contract. Domain outcomes (`stale`, `deferred`, `needs_review`, `ask` `Failed`/`Interrupted`) carry `result` and `outcome` instead. |
| `page` | `{limit, shown, cursor, total}` on a paged result (`search` and the Inspection pages), else null. `cursor` is null on the last page; `total` is null when not computed. |
| `stop` | Why the command stopped early: `sigint`, `sigterm` or `deadline` (`--timeout`); null otherwise. |

Forward rule: ignore unknown keys. Treat an unknown `status` or `outcome` as non-success.

`details` never echo option values or text content; they name options, positions, limits, record IDs and versions. An argv token is echoed in `details.option` or `details.command` only when it matches `^[a-z][a-z-]{0,39}$`. The one exception is `key` and `key_conflict`'s `details.key`, which repeat the caller's key; keys must match `^[A-Za-z0-9._:-]{1,128}$`.

### Owner decisions

> These commands record an owner decision. Running the CLI does not make a caller the owner. Run them only on the owner's explicit instruction for that exact operation.

Test harnesses on synthetic data act as the owner. The marks are documentation: the CLI cannot verify who is calling, and every CLI write already runs as the owner inside the shared service.

| `owner_decision` | Commands | Why |
|---|---|---|
| `true` | `apply`, `dismiss` | Accept or reject one exact reviewed candidate. |
| `true` | `undo` | Owner Undo compensation. |
| `true` | `configure`, `delegation`, `pause`, `resume` | Provider, model, effort and helper selection; delegation policy; pausing or resuming maintenance. |
| `true` | `project-create`, `project-chat`, `project-document`, `project-settings`, `chat-move`, `project-link`, `project-unlink`, `project-delete`, `project-restore` | Project membership, settings and instructions are owner-only configuration. |
| `true` | `keep-in-brn` | Moves a private chat attachment into general knowledge. |
| `true` | `archive`, `resolve`, `unarchive` | Chat archive state is owner-controlled. Archive interrupts that chat's runs and pauses saved queue work for every chat; unarchive never resumes it, and the CLI has no command that resumes the chat queue. |
| `true` | `topics-create`, `topics-choose` | Sets the workspace's root Topics note. |
| `true` | `upgrade`, `model-install` | Changes a workspace schema; downloads the pinned model. |
| options | `ask` with `--edit-note` or `--record-decision` | Grants that run authority to edit the named note, or saves a reply as a protected decision. |
| options | `intake` with `--project` | Adds the document to the project's membership. |
| `false` | every other command | Reads, your own edit sessions, ordinary notes and chats, `queue`, knowledge check, export, backup, restore into a fresh directory, init, intake without `--project`. |

Attach a document to a project only with `--intent project --project ID`, which is an owner decision; `--thread` must name a chat. Edit a protected note (`read` shows it) only on the owner's explicit instruction: Save records an owner change.

### Credentials

A workspace the CLI creates (an explicit `--data-dir` that did not exist) starts with no provider credential source: `settings` shows `auth_file` and `credentials_dir` as null. Only an explicit `configure --provider NAME --model ID --effort LEVEL --auth-file ABSOLUTE_FILE` (Codex) or `--credentials-dir ABSOLUTE_DIR` (Copilot), on the owner's instruction, sets one. Workspaces that already exist, including ones the desktop created, keep their stored Settings. Desktop shows no sign-in file for such a workspace; choose one in Settings before running work.

In machine mode, `ask`, useful intake and the intake of a format that needs conversion by a model (anything but Markdown and text) are refused with `credentials_not_configured` before any provider contact or commit. `ask` also runs useful intake, so the intake check fires first and the `ask` check guards both. A keyed intake retry that replays (Retries) needs no credentials. Default mode keeps the previous path, where the run reports its own failure.

### Statuses

| Exit | Status | Meaning | Agent action |
|---|---|---|---|
| 0 | `ok` | Did what was asked, including an identical replay. Reads returned. `ask` Completed. | Continue. |
| 1 | `internal` | Unexpected storage, I/O or encoding failure. | Report it. Retry only if `retry` is `replay`, `converge` or `read`, or after checking state; for `refuse`, read state first. |
| 2 | `usage` | argv or input does not fit the grammar. No requested change was committed (a fresh directory may have been initialised when the error is found after opening, for example `--knowledge maybe`). | Fix the invocation; do not retry unchanged. |
| 3 | `stale` | Not applied: what the request was based on changed. For `ask` and useful intake (outcome `Interrupted` without SIGINT), another writer stopped the run. | Re-read (`conflict` has current versions; for a stopped run, `read --id <result.run>`) and decide. Under a `--key`, make any new attempt with a new key. |
| 4 | `deferred` | Not applied: a guard is active, an open edit session on a target note or a Working run (`conflict[].kind` is `Note` or `Run`). Intake is never Deferred. | Wait within your own time budget until the guard clears, then report it to the owner if it remains. Never discard a session you did not open. See the Deferred recipe. |
| 5 | `needs_review` | Not applied: owner review is required. | Surface it to the owner; do not retry. |
| 6 | `rejected` | Refused by a safeguard or current state: not found, invalid request (including an `ask` or useful intake refused before contacting a provider, and a text over a shared-service field limit), key conflict, unsafe or occupied path, open edit session. | Fix the cause or wait for the state to change; do not retry unchanged against unchanged state. |
| 7 | `failed` | A dependency failed: provider, conversion, helper, semantic index, or an `ask` run ended Failed. | May succeed later or with other input; check `retry` first. |
| 130 | `interrupted` | SIGINT was acknowledged. Nothing was committed, unless `result` shows a recorded partial run (`ask`). | Re-run if still wanted; it applies at most once. After `ask` or useful intake, a re-run is another model turn. |
| 143 | `terminated` | SIGTERM was acknowledged (machine mode). Nothing was committed, unless `result` shows a recorded partial run (`ask`) or `details.hard` is true (the process stopped without reaching a safe point and may have committed). | Re-run if still wanted, as for `interrupted`; when `details.hard` is true, read state first as `retry` describes. |

### Outcomes, operation and changed

| Commands | `outcome` | `operation` | `changed` |
|---|---|---|---|
| Reads (`records`, `notes`, `threads`, `needs-you`, `history`, `candidates`, `settings`, `projects`, `project-delete-preview`, `read` without `--limit`, `revisions`, `revision`, `runs`, `operation`, `search`, `connections`, `topics`, `recovery`, `knowledge-check`) and non-receipt writes (`init`, `begin-edit`, `export`, `backup`, `restore`, `upgrade`, `model-install`, `knowledge-check --seed-chat yes`) | null | null | `[]` |
| Project commands, `queue`, `archive`, `resolve`, `unarchive` (result is a receipt) | `Applied` | `result.operation` | the receipt's writes |
| `note-create`, `thread-start` (result is the created record) | `Applied` | the receipt's operation | the receipt's writes |
| `undo` | ApplyOutcome | the compensation operation, in every outcome | writes when Applied |
| `apply`, `keep-in-brn`, `delegation`, `configure`, `pause`, `resume`, `topics-choose`, `intake --intent full\|chat\|project` | ApplyOutcome | `result.receipt.operation` when Applied | writes when Applied |
| `save` | SaveOutcome | `result.receipt.operation` when Saved | writes when Saved |
| `update-buffer` | `Updated`, `Ignored` | null | `[]` |
| `discard` | `Discarded`, `Ignored` | null | `[]` |
| `dismiss` | `Dismissed` | null | `[]` |
| `topics-create` | the pointer's ApplyOutcome | null (two operations) | the created Topics note, plus the pointer's writes when Applied |
| `ask`, `intake --intent useful` | `Completed`, `Interrupted`, `Failed` | null | `[]` |
| `read --limit` | `Read`, or `Stale` when the note changed since the page's `cursor` version | null | `[]` |

ApplyOutcome `Applied` is `ok`, `Stale` and `Superseded` are `stale`, `Deferred` is `deferred`, `NeedsReview` is `needs_review`. SaveOutcome `Saved` is `ok`; `Stale`, `GenerationMismatch` and `Closed` are `stale`; `Deferred` is `deferred`. `update-buffer` `Ignored` is `ok` only when the session is open with the requested generation and exactly the supplied text (an identical retry); otherwise `stale`. `discard` `Ignored` is `stale`. A run terminal `Interrupted` is `interrupted` (exit 130) after SIGINT and `stale` otherwise. `read` of an unknown ID is `rejected` `not_found` in machine mode (default mode still prints `null`).

`commands[].outcomes` in the description lists every variant of the outcome type(s) a command returns, in a fixed order; for `intake` it is the ApplyOutcome list followed by the terminal list. It is generated from the types, not from reachability, so a listed outcome can be unreachable for a command (intake lists `Deferred` but is never Deferred). An outcome that is not listed never appears.

Reachability from the CLI: every CLI write runs with owner authority and no run, chat or project binding, so `NeedsReview` and `Superseded` never come back to a CLI command (the core returns them only to grants, maintenance and runtime authority, `store.rs` `apply_on`). A model's proposal that needs review stays an unapplied candidate: `candidates` and `operation` list it, and `apply` or `dismiss` decide it on the owner's instruction (`apply` of a candidate whose note changed since is `Stale`). `ask` ends `Completed`, `Failed` (provider error, exit 7), or `Interrupted` (SIGINT 130, SIGTERM 143, the deadline 7, another writer 3). Intake is never `Deferred`.

`topics-create` commits the Topics note before it sets the pointer. On Deferred, the Topics note exists; run `topics-choose --note <result.note.id>`, not `topics-create`. If the pointer step fails with an error instead, the envelope is an error with `changed: []`, but the Topics note still exists and `notes` lists it.

### Error codes

| Code | Status | Raised by | Details |
|---|---|---|---|
| `missing_data_dir` | usage | First argument (after `--json`) is not `--data-dir`, or it has no value. | `{}` |
| `invalid_data_dir` | usage | Data directory is not absolute. | `{}` |
| `missing_command` | usage | No command after the data directory. | `{}` |
| `unknown_command` | usage | Command not in the table. | `{command}` when echoable |
| `malformed_option` | usage | A token where `--name` was expected, or an argument that is not valid UTF-8 (`command` is then null). | `{position}`, 1-based after the program name, counting `--json` |
| `missing_value` | usage | `--name` with no value. | `{option}` |
| `repeated_option` | usage | An option given twice. | `{option}` |
| `unknown_option` | usage | Option not allowed for the command. | `{option, allowed}` |
| `conflicting_options` | usage | `--NAME` with `--NAME-file`; `--key` with `--intent useful`; `--key` on `knowledge-check` without `--seed-chat yes`; `--project` with `--thread`; `--helper off` with helper options; `--edit-note` with `--record-decision`. | `{options}` |
| `missing_option` | usage | Required option absent (checked before the workspace opens). | `{option}` |
| `invalid_value` | usage | Bad integer, enum or yes/no value, key format, relative path, `--trace`, a page `--limit` out of range or a malformed `--cursor`, or malformed `--actions` JSON. | `{option, expected}` (`expected` is human text) |
| `invalid_input` | usage | `--NAME-file` problem; an unreadable or empty intake source; an unreadable or malformed `--preview-file`. | `{option, reason}`, plus `max_bytes` for `too_large` |
| `not_found` | rejected | Unknown record or operation; `read` returning null. | `{}` |
| `invalid_request` | rejected | The shared service refused the request (core `Invalid`, app `Invalid`, including the helper-selection refusals); `--thread must name a chat`; `--project must name an active project`; a `backup` or `export` destination whose parent is missing; an `ask` or useful intake refused by the run service (`ToolRejected`, `ModelRefused`). | `{}`, `{option}`, `{option: "destination", reason: "parent_missing"}`, or `{kind}` for `ask` |
| `key_conflict` | rejected | The same key with a different request. | `{key}` with `--key`, else `{}` |
| `unsafe_directory` | rejected | Old or foreign format, missing database, symlink, non-empty restore target, backup inside the workspace. | `{}` |
| `destination_exists` | rejected | `backup` or `export` destination already exists. | `{option: "destination"}` |
| `edit_session_open` | rejected | `begin-edit` on a note with an open unsaved session. | `{note, session, base_version}` |
| `credentials_not_configured` | rejected | Machine mode: `ask`, useful intake, or a non-Markdown/text intake on a workspace with no credential source. Nothing was committed and no provider was contacted. | `{provider}` |
| `unavailable` | failed | Semantic search or a model folder is unusable; no native retrieval in this build; conversion unsupported on this platform or format; model install refused. | `{}` or `{reason: "model_install"}` |
| `provider_failed` | failed | `ask` or useful intake: any other `AiErrorKind` (none is known to reach `Err` from `run` at `771116f0`; kept as a fallback); also `ConversionFailure::Provider`. | `{kind}` in snake_case |
| `conversion_failed` | failed | Document conversion failed. | `{reason}` |
| `timeout` | failed | Document conversion exceeded its time limit, or the `--timeout` deadline passed before the command committed. | `{}` for conversion; `{deadline_seconds, hard}` for `--timeout` |
| `busy` | failed | SQLite stayed locked past the 5 s busy timeout (another writer holds the workspace). This transaction did not commit, although earlier steps of the command may have. | `{}` |
| `interrupted` | interrupted | SIGINT while waiting for input, before opening, during an intake read or conversion, or during a model download. | `{}` |
| `terminated` | terminated | SIGTERM (machine mode) at any point where SIGINT or the deadline is observed. | `{}`, or `{hard: true}` after a hard stop |
| `storage_error` | internal | Unexpected SQLite or record-encoding failure; an `ask` run storage error. | `{}` |
| `io_error` | internal | Other I/O failures on the CLI's own paths. | `{}` |
| `internal` | internal | Result encoding failed; input reader stopped; a committed review whose cleanup failed. | `{}`, `{applied: true}` or `{dismissed: true}` |

`ToolRejected` and `ModelRefused` are almost always pre-provider refusals from `begin`/`Selection::validate`; `finish` can also return `ToolRejected` after provider contact when the run record is inconsistent; the class stays `invalid_request`/`refuse`. The pre-provider refusals cover a missing, archived or closed chat, `--budget` outside 1..32, a prompt over 128 KiB, an `--edit-note` not at `--base`, a settings mismatch and a useful-intake source over 1 MiB. Real provider failures end as an `ask` terminal `Failed` (`failed`).

The #145 helper refusals keep their messages: `--helper off` with helper options is `conflicting_options`, a missing helper option is `missing_option`, a bad helper provider or effort is `invalid_value`, and the shared service's credential refusals (for example a Copilot helper with an auth file, or a relative helper path) are `invalid_request`.

`apply` or `dismiss` exiting 1 with `internal` and `details.applied` or `details.dismissed`: the decision is recorded and only the attention cleanup failed. Retry the same `apply` or `dismiss`; `apply` replays its receipt, `dismiss` is repeatable, and both re-run the cleanup.

### Streams, SIGINT and time

- stdout is exactly the envelope line. stderr carries only existing human progress lines (intake conversion labels, `model-install` bytes). Never parse stderr. Machine mode does not duplicate errors on stderr.
- stdin is read only for `--NAME-file -`. Nothing prompts, and no terminal is read as input.
- **Stop reasons.** SIGINT, SIGTERM (machine mode only) and the global `--timeout SECONDS` deadline (1..86400, after `--data-dir`) all ask the command to stop. The first reason wins and the envelope `stop` names it (`sigint`, `sigterm`, `deadline`). They are observed while waiting for `--NAME-file` input (within 25 ms, even with a blocked read or FIFO open); before the workspace opens; while reading or converting an intake source; during a model download; and as an `ask` run that ends `Interrupted`. SIGTERM and the deadline are also observed right after the workspace opens and after the search index is ready (`search`, `connections`, `ask`).
- **Results.** Before anything is committed: SIGINT gives `interrupted` (130), SIGTERM `terminated` (143), the deadline `failed` with `timeout` and `{deadline_seconds, hard: false}` (7). A fresh data directory may already have been initialised. An `ask` or useful-intake run that stops ends `Interrupted` with its `result`: exit 130 after SIGINT, 143 after SIGTERM, 7 (`failed`, outcome `Interrupted`) after the deadline.
- **Never aborted.** A call already in flight finishes and reports its real outcome, so a stop never hides a commit; the SQLite transaction stays the unit of change, so nothing is half-applied. Once started, `export`, `backup`, `restore`, `upgrade` and index builds run to completion. SIGINT during a commit does not stop the commit.
- **Hard stop.** If no result is ready 3 s after SIGTERM or the deadline, the process writes one envelope line itself (`terminated` 143, or `failed`/`timeout` 7, with `details.hard: true` and the message "the command may have committed") and exits. Read state as `retry` describes before retrying. It can leave: a Working run (see `runs`), conversion scratch under the temporary directory, a restore stage, or a backup or export temporary.
- **Default mode.** SIGTERM keeps the default action (the process is killed, no output). A deadline before a commit prints `Stopped at the --timeout deadline (N s); nothing was committed` (exit 1); a stopped run prints its JSON (exit 0); a hard stop prints `Stopped at the --timeout deadline (N s); state unknown` (exit 1).
- **Busy.** SQLite waits 5 s for another writer before `busy` (exit 7 in machine mode; default mode keeps SQLite's text, exit 1). A keyed retry after the lock clears applies once.
- SIGTERM before the handler is installed, a second SIGTERM, and SIGINT before startup kill the process without an envelope.
- Checks of required options, `intake`'s conditional requirements, option conflicts, the key pattern, `--NAME-file` sources and `--trace` run before the workspace opens. `intake --intent chat` requires `--thread`, and `--intent project` requires `--project` (`required_when` in the description), so nothing is read or converted before that refusal. Other value checks run where they always have, sometimes after a fresh directory was initialised.

### Inspection

Lists that can grow without bound have a bounded page form. Without page options `history`, `candidates` and `records` print everything, as before (`"bounded": false` in the description, with `bounded_form`); `notes`, `threads`, `needs-you`, `recovery` and `projects` have no page form yet. A page option selects page mode; the new commands always page.

| Command | Options | `result` |
|---|---|---|
| `history` | `[--record ID] [--limit 1..100, default 20] [--cursor C]` | `{receipts, shown, cursor, total}`; each receipt is `{operation, reason, authority, committed_at, undo_of, writes, writes_total}`. `--record` keeps receipts that wrote that record; an unknown record is `not_found`. |
| `candidates` | `[--limit 1..100, default 20] [--cursor C]` | `{candidates, shown, cursor, total: null}`; each is `{operation, request_hash, reason, writes, writes_total, inputs_total, undo_of, replacement_of}`, where a write's `version` is the expected version (null when it creates the record). |
| `records` | `[--kind Note\|Thread\|…] [--limit 1..100, default 20] [--cursor ID]` | `{records, shown, cursor, total: null}`; each is `{id, kind, version, archived, title}` (title cut at 160 characters, null for kinds without one). |
| `revisions` | `--note ID [--limit 1..32, default 32] [--cursor VERSION]` | `{id, revisions, shown, cursor}`; each revision is `{version, operation, reason, authority, committed_at}`. |
| `revision` | `--note ID --version V [--limit 1..131072, default 65536] [--cursor BYTE]` | One accepted revision's text slice `{id, version, title, start, end, total_bytes, markdown, next_start, cursor}`; stays readable after later edits. |
| `read` | `--id ID --limit 1..131072 [--cursor VERSION.START]` | `{outcome: "Read", note: {…}, cursor}`, or `{outcome: "Stale", id, expected_version, current_version, cursor: null}` (exit 3, `conflict` names the note) when the note changed since the first page. `--cursor` needs `--limit`; a Thread or other record is `invalid_request`. |
| `operation` | `--id OP`, or `--key K --command NAME` | `{operation, state, undo_of, replacement_of, request_hash, reason, inputs, inputs_total, writes, writes_total, receipt, related}`. `state` is `applied`, `prepared` (an unapplied candidate), `retired` or `allocated` (the key was used but nothing was prepared). `writes` come from the receipt, else from the prepared request; `inputs` lists at most 64 `{record, version, expectation}`; `receipt` is `{committed_at, authority}` or null. `--key` finds the operation a keyed command recorded under that key (`--command` names the command; `resolve` shares `archive`'s scope, project commands share one scope). A lookup never allocates, so looking a key up does not use it. An unknown id or unused key is `not_found`; `--command` naming a command without `--key` is `invalid_value`. The envelope `key` echoes the looked-up key. |
| `runs` | `[--thread ID] [--state working\|interrupted\|completed\|cancelled\|failed] [--limit 1..100, default 20] [--cursor RUN]` | `{runs, shown, cursor, total: null}`; each run is `{id, version, archived, thread, state, provider, model, effort, budget, fence, progress, operations_total, guards_writes}`. |

- Pass `cursor` back unchanged with the same options to get the next page; it is null on the last page. Cursors are checked before the workspace opens (`invalid_value`); a well-formed cursor that does not fit (a byte offset inside a character) is `invalid_request`, and an unknown well-formed record or run cursor gives the following page or an empty one.
- Pages never repeat an item. `history` pages follow commit order, so a receipt committed while you page appears on a later page. `records` and `runs` (ID order) and `candidates` can miss items created while you page: restart without a cursor for a full pass.
- The envelope `page` repeats `{limit, shown, cursor, total}`; for `revision` and `read`, `shown` and `total` count bytes. `search` fills `page` too.
- Text is cut: `reason`, `authority` and `progress` at 512 characters; at most 32 writes per receipt or candidate (`writes_total` has the count).
- `guards_writes` is true for a Working run that is not archived: preference writes such as `topics-choose` come back Deferred (`conflict[].kind` `Run`) until it stops, and owner settings changes (`configure`, `delegation`, `pause`, `resume`) or archiving its chat stop it.

### Text input

`--title`, `--text`, `--prompt`, `--instructions` and `--actions` also accept `--NAME-file ABSOLUTE_PATH` or `--NAME-file -` (stdin), wherever the command accepts `--NAME`:

- `--NAME` and `--NAME-file` together are `conflicting_options`. The source must be absolute or `-` (`invalid_value`). Only one option may read stdin (`invalid_input` `stdin_reused` on the second, in the order title, text, prompt, instructions, actions).
- Bytes are used exactly: no trimming, no newline or BOM changes; an empty file is an empty string. Write titles without a trailing newline.
- Refused with `invalid_input` and a `reason`: `is_directory`, `unreadable`, `is_terminal` (stdin or a path that opens a terminal), `too_large` (over 16777216 bytes), `not_utf8`, `contains_nul`.
- Sources are read before the workspace opens, so an input error or SIGINT leaves the data directory untouched.
- 16 MiB is only the transport bound. The shared service applies smaller field limits, which return `invalid_request` after the text is read: `ask` prompt 128 KiB, `queue` text 128 KiB, project title 1024 bytes and instructions 128 KiB, useful-intake converted text 1 MiB.
- `--text -` (without `-file`) still stores a literal `-`.

### Retries

A key names one intended change. Reuse it only to retry the same command with the same inputs; use a new key for every new change. Keys are scoped per command (`resolve` shares `archive`'s scope; project commands share one scope and keep their existing raw keys, so use one key per project change). `operation --key K --command NAME` shows what a key recorded without using it. Keys are stored in the workspace, so a retry against a restored backup replays too.

| Class | Meaning |
|---|---|
| `read` | Changes nothing (a fresh data directory may have been initialised). |
| `replay` | The retry uses the same operation identity. If it was applied, the retry returns its receipt (same `operation`) and changes nothing; a replayed receipt shows the original versions. If it was not applied, the retry re-attempts exactly that request, at most once, and never creates another record, message or candidate. |
| `converge` | Reaches the same end state. It may record a new operation and bump versions, but creates no new records and leaves no unapplied candidate. |
| `refuse` | At most once, never a duplicate. If the first attempt took effect, an identical retry changes nothing and returns `stale` or `rejected`. If it did not, the retry performs the change once and can return `ok`. After a lost response, read state first. |
| `duplicate` | Repeats the change: another record, message, model turn or unapplied candidate. Never retry blindly. |

| Command | `--key` | Without key | With key | Owner decision |
|---|---|---|---|---|
| `init` | no | converge | — | no |
| `records` | no | read | — | no |
| `notes` | no | read | — | no |
| `threads` | no | read | — | no |
| `needs-you` | no | read | — | no |
| `history` | no | read | — | no |
| `candidates` | no | read | — | no |
| `settings` | no | read | — | no |
| `pause` | no | converge | — | yes |
| `resume` | no | converge | — | yes |
| `recovery` | no | read | — | no |
| `topics` | no | read | — | no |
| `topics-create` | yes | duplicate | converge (replays the note and chooses it again) | yes |
| `projects` | no | read | — | no |
| `project-create` | yes | duplicate | replay | yes |
| `project-chat` | yes | duplicate | replay | yes |
| `project-document` | yes | duplicate | replay | yes |
| `project-settings` | yes | refuse (version-bound) | replay | yes |
| `chat-move` | yes | converge | replay | yes |
| `project-link` | yes | converge | replay | yes |
| `project-unlink` | yes | converge | replay | yes |
| `project-delete-preview` | no | read | — | no |
| `project-delete` | yes | refuse (preview-bound) | replay | yes |
| `project-restore` | yes | refuse (version-bound) | replay | yes |
| `topics-choose` | no | converge | — | yes |
| `connections` | no | read | — | no |
| `knowledge-check` | yes | read (`--key` needs `--seed-chat yes`) | — | no |
| `knowledge-check` (--seed-chat yes) | yes | duplicate (seeds another review chat) | replay (the first findings win) | no |
| `model-install` | no | converge (a verified destination downloads nothing) | — | yes |
| `note-create` | yes | duplicate | replay | no |
| `thread-start` | yes | duplicate | replay | no |
| `delegation` | no | converge | — | yes |
| `read` | no | read | — | no |
| `revisions` | no | read | — | no |
| `revision` | no | read | — | no |
| `runs` | no | read | — | no |
| `operation` | no | read | — | no |
| `search` | no | read | — | no |
| `begin-edit` | no | refuse (`edit_session_open` names the open session) | — | no |
| `update-buffer` | no | replay (an identical retry exits 0 with `Ignored`) | — | no |
| `save` | no | replay | — | no |
| `discard` | no | refuse (a retry is `Ignored`, exit 3) | — | no |
| `apply` | no | replay | — | yes |
| `dismiss` | no | replay | — | yes |
| `undo` | yes | duplicate (a retry after Deferred applies a second compensation; the first stays as a candidate) | replay | yes |
| `archive` | yes | converge | replay | yes |
| `resolve` | yes | converge | replay | yes |
| `unarchive` | yes | converge | replay | yes |
| `keep-in-brn` | yes | duplicate (a retry after Deferred applies under a new operation) | replay | yes |
| `queue` | yes | duplicate | replay | no |
| `ask` | no | duplicate (new model turn) | — | with --edit-note, --record-decision |
| `intake` | yes | duplicate (a prepared but unapplied first attempt stays as a candidate) | replay | with --project |
| `intake` (--intent useful) | no | duplicate (`--key` is refused) | — | with --project |
| `configure` | no | converge | — | yes |
| `export` | no | refuse (`destination_exists` after success) | — | no |
| `backup` | no | refuse (`destination_exists` after success) | — | no |
| `restore` | no | refuse (`unsafe_directory` after success) | — | no |
| `upgrade` | no | converge | — | yes |

Keyed edge cases: different inputs under the same key give `key_conflict` (a different title or text for `note-create`, a different thread for `queue`, a different title for `intake`). A keyed full/chat/project intake replays when bytes, path, scope and title match a committed import, without converting or sending the document again; if that import found the bytes already imported, the retry converts again and records only an observation. A changed title, path or scope under the key is `key_conflict`, also before conversion. A keyed `undo` that was Deferred keeps its prepared compensation; the retry applies exactly that operation and reports the same `operation`. A keyed `topics-create` replays the Topics note and points Topics at it again; if Topics was changed in between, use a new key. A keyed `knowledge-check --seed-chat yes` reuses the chat and message its first attempt committed, so later findings do not change the replay; `--key` without `--seed-chat yes` is `conflicting_options`. Its envelope `operation` is the message; `operation --key K --command knowledge-check` lists the chat operation in `related`. `ask` and useful intake take no key: after a lost `ask` response, `runs --thread T --limit 5` and `read --id RUN` show what happened.

### Recovery recipes

- **Lost response.** If `retry` is `replay`, retry the identical argv with the same `--key`. For `refuse`, read state first (`read`, `projects`, `recovery`, or check the destination): if the change is there, do not retry; if not, the identical retry performs it once. For `duplicate` without a key, inspect `notes`, `threads`, `history` or `candidates` first.
- **Stale save** (exit 3), using only the session your own successful `begin-edit` returned: keep `result.session.markdown` (discard empties it); `discard --session S --generation G`; `begin-edit --note N --base <conflict[0].version>`; `update-buffer --session S2 --generation 1 --text-file merged.md`; `save`.
- **Stale under a key** (exit 3). The key stays bound to the request that went stale, so a retry under it is `stale` or `key_conflict` again. Re-read, then make the new attempt with new inputs and a new key.
- **`edit_session_open`** (exit 6). The note has unsaved writing in `details.session`. BRN does not record which caller opened a session. Write only to a session returned by your own successful `begin-edit`. If your `begin-edit` response was lost, you cannot prove the session is yours: do not `update-buffer`, `save` or `discard` it. Report it to the owner, or wait until `recovery` no longer lists it, then `begin-edit` again.
- **Deferred** (exit 4). For `conflict[].kind` `Note`, an edit session holds the note: wait until `recovery` no longer lists it, and discard it only if your own `begin-edit` opened it. For `Run`, wait until `read --id RUN` no longer shows it Working. Wait only within your own time budget, then report it to the owner: a run left Working by a killed process stays Working until it is recovered or the owner stops it. Once the guard clears, retry the same argv (same `--key`) if `retry` is `replay` or `converge`. If it is `duplicate` (unkeyed `undo` or `keep-in-brn`), a retry is a new operation and the first attempt stays as an unapplied candidate that `candidates` lists; report that to the owner (`dismiss` is an owner decision) and use `--key` next time. `topics-create` exception: run `topics-choose --note <result.note.id>`.
- **Stopped run** (`ask`, exit 3, outcome `Interrupted`). Another writer stopped the run; `read --id <result.run>` shows what it recorded. A new `ask` is a new model turn.
- **Killed `ask`** (the process died, for example after SIGKILL or a hard stop). Its run can stay Working: `runs --state working` lists it with `guards_writes: true`, and preference writes such as `topics-choose` come back Deferred. A new `ask` in that chat marks it `Interrupted` before it starts. On the owner's instruction only, `pause` or `archive` stops it; it then shows `Cancelled` (or `Interrupted` once the daily-workflow branch is merged). `ask` has no `--key`: after a lost `ask` response, `runs --thread T --limit 5` finds the run and `read --id RUN` shows what it recorded.
- **Committed review with failed cleanup** (exit 1, `details.applied` or `details.dismissed`): retry the same `apply` or `dismiss`.
- **Generations.** `begin-edit` returns generation 0. Each `update-buffer` uses a generation greater than `session.generation`; the first is 1. `save` and `discard` use the latest generation.

### Compatibility

Without `--json`, stdout bytes, stderr text and exit statuses (0 or 1) stay as before for every invocation that worked, including exit 0 for Stale, Deferred and other domain outcomes and `null` for an unknown `read`. The exceptions:

1. A closed stdout or stderr no longer panics (exit 101); an argument that is not valid UTF-8 fails with `Arguments must be valid UTF-8`, exit 1.
2. Missing required options, `intake`'s conditional requirements and `--trace` are reported before the workspace opens, so no directory is created. An invocation with several faults may now report the missing option (or `--trace`) first, for example `configure --provider Bad` without `--model` says `Missing --model`.
3. `--key` on the newly keyed commands and `--NAME-file` are accepted; refusals of the new grammar have new texts, for example `Choose either --text or --text-file`, `--key is not accepted with --intent useful`, `--NAME-file takes an absolute path or -`.
4. `--key` must match `^[A-Za-z0-9._:-]{1,128}$`, also on project commands (`--key "trip 2026"` is now refused).
5. `--help` prints the same text followed by a short "Agents and scripts" block.
9. New commands (`revisions`, `revision`, `runs`, `operation`), options (page options, `--timeout`, `--key` on `topics-create` and `knowledge-check --seed-chat yes`) and description keys are accepted; flag-less output of existing commands is unchanged.
10. The "Agents and scripts" block of `--help` covers the new surface; the `HELP` text before it is byte-identical.
12. A workspace the CLI creates gets `auth_file: null` instead of a derived `CODEX_HOME/auth.json` or `~/.codex/auth.json` (owner decision B2, see Credentials). Every in-repo caller that runs a provider passes `--auth-file` or `--credentials-dir` to `configure`.

### Changes from contract 1

Every in-repo consumer moved to contract 2 in the same change; there is no contract-1 switch.

| Change | Contract 1 | Contract 2 |
|---|---|---|
| `contract` | `1` | `2`, in every envelope and in `brn --json --help` |
| `page` | absent | always present: null, or `{limit, shown, cursor, total}` |
| `stop` | absent | always present: null, `sigint`, `sigterm` or `deadline` |
| `terminated` | SIGTERM killed the process | machine mode: status `terminated`, exit 143, code `terminated` |
| `--timeout` | none | global `--timeout SECONDS`; `timeout` gains `{deadline_seconds, hard}` |
| `busy` | lock contention was `storage_error` (internal, 1) | `busy` (failed, 7) |
| Credentials | CLI-created workspaces derived `auth_file` from `CODEX_HOME` or `HOME` | no credential source until `configure`; machine-mode `credentials_not_configured` (rejected, 6) before any provider contact |
| Retry keys | `topics-create` and `knowledge-check --seed-chat yes` duplicated; keyed intake converted (and sent non-Markdown documents to the provider) before replaying | `--key` on both (converge, replay); keyed full/chat/project intake replays before conversion |
| Inspection | `history`, `candidates`, `records` return everything | page options and the `revisions`, `revision`, `runs`, `operation` commands; `read --limit` returns `Read` or `Stale` |

### Before and after

```sh
# Stale save: before, exit 0 with {"outcome":"Stale",...}; after:
$ brn --json --data-dir "$D" save --session "$S" --generation 1    # exit 3
{"contract":1,"command":"save","ok":false,"status":"stale","exit":3,"outcome":"Stale",…,"retry":"replay","conflict":[{"id":"N","kind":"Note","version":3,"archived":false,"open_edit":true}],"result":{"outcome":"Stale","session":{…}},"error":null}

# Multiline text: before, the text had to go through argv (shell expansion, "$(cat …)" drops the trailing newline, visible in ps, E2BIG past ~1 MiB); after:
$ brn --json --data-dir "$D" note-create --title Plan --text-file /abs/plan.md

# Lost response: before, a retry made a second note; after, both calls report the same "operation" and "retry":"replay":
$ brn --json --data-dir "$D" note-create --key plan-1 --title Plan --text-file /abs/plan.md

# Deferred undo: before, exit 0 with no operation, and every retry left another candidate; after:
$ brn --json --data-dir "$D" undo --operation "$OP" --key undo-1   # exit 4, "operation":"X"; after the guard clears: exit 0, Applied, "operation":"X"

# Open edit session: before, "SQLite: UNIQUE constraint failed: edit_sessions.note" (exit 1); after, exit 6:
{"error":{"code":"edit_session_open","details":{"note":"…","session":"…","base_version":2},…}}

# Second export: before, exit 1; after, exit 6 with "code":"destination_exists" and "retry":"refuse".
# Chat attachment without its chat: before, the source was read and converted first; after, "Missing --thread" before anything is read.
```

The maintained journey runner, `scripts/cli_journey.py BRN_BINARY`, completes a full synthetic journey through this contract alone and is part of `scripts/verify-end-to-end.sh`.
