# brn-workflow

Shared authoritative application flow for desktop and headless use: imports, eligibility, index lifecycle, grounded answers, saved sessions, drafts and comments. Owns application coordination across adapters.

## Interfaces and source

[Workspace API](src/lib.rs), [worker commands/events](src/worker.rs), [draft workflow](src/drafts.rs), [comment workflow](src/comments.rs), [brn-flow CLI](src/main.rs).

## Markdown notes

[`notes`](src/notes/mod.rs) re-exports the public editing types from
`brn-store::notes`. `Workspace::open_note(operation_id, vault, relative_path)`
enrolls existing regular, single-link UTF-8 `.md` files (including empty files,
up to 1 MiB). `note(id)` freshly observes disk; `saved` is never substituted
with recovery bytes. Changed content or identity reports Conflict without
rebasing the editing stamp or discarding protected work. Observation tokens
are durable and distinct from the editing baseline.

`save_note_buffer(NoteSubmission)` acknowledges generation-checked SQLite
recovery only; it does not write Markdown. `note_recoveries()` includes clean
notes and unresolved work and requires no vault access. Note operations return
typed `NoteResult` failures independently of generic workflow errors.

Opening the workspace itself does not open the vault. ID-only observations
lazily validate/acquire the registered root and hold ownership until workspace
drop; missing/replaced roots and another BRN owner produce Unavailable or
OwnedElsewhere views with explanatory messages and no fresh saved bytes.
Buffer edits and history remain available offline. A workspace is bound to one
vault; selecting a different root requires a separate data directory. Open
replay binds the caller's exact root/path inputs before filesystem checks and
then returns a fresh view, not stale saved bytes. Worker notice draining and
Markdown publication are later tasks, not provided by these methods.

## Dependencies and features

Depends on `brn-store`, `brn-provider`, `brn-retrieval`. Default features are empty; `native-retrieval` forwards to retrieval `native`.

## Verification

Run from the repository root:

```sh
cargo test -p brn-workflow --locked
bash scripts/verify-end-to-end.sh
```

Tests cover flow, drafts, comments and provider-free Markdown note observation,
ownership and recovery. Run the note checks with
`cargo test -p brn-store -p brn-workflow --test notes --locked`.
Keep durable authority in store and coordinate provider/retrieval through their
adapters. Preserve evidence validation, operation identity and late-response safety.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
