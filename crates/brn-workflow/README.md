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
then returns a fresh view, not stale saved bytes. Alternate case or Unicode
spellings of an already registered file are rejected with typed Conflict naming
the registered path, based on device/inode identity rather than content hashes;
they cannot allocate a second editing buffer. Worker notice draining remains
a later task.

`save_note(NoteSubmission)` explicitly saves the original Markdown path:
durable recovery intent, exclusive staging, coordinated baseline revalidation,
atomic exchange, installed/displaced identity verification, then a durable
receipt. Equal-generation recovered text is accepted; no-op saves preserve
identity, timestamps and permissions. Successful saves rebase the editing
baseline without overwriting later typing. File flushing requires
`F_FULLFSYNC`; directory durability uses plain `fsync`, with no power-loss claim.
Late external races retain the actual displaced object and report conflict or
uncertainty, never automatic rollback.

Pre-exchange refusals resolve NotApplied only with freshly observed original
baseline proof and no stage creation or an exact recorded unexchanged stage.
The failure result remains the immutable replay result; submitted recovery and
unexpected staging occupants stay protected, but a new original save is allowed.
When live progress proves no exchange and a created stage is absent or exactly
the recorded unexchanged stage, an observed external change remains
Conflict/NotApplied. Its intent stays Unresolved and blocks later original
saves pending explicit resolution; its note view reports Conflict, not an
uncertain execution outcome. A possibly attempted exchange or unproven created
artifact remains SaveUncertain/Unknown.

`reconcile_note_save(operation_id)` classifies interrupted writes and commits
metadata only: it never retries exchange, creates, renames or unlinks files.
Matching bytes without execution identity proof remain uncertain. Compact
receipts/refusals replay even without a vault or full retained intent.
Artifact cleanup is separate, best-effort bookkeeping after terminal proof
and durable recovery, preserving unexpected occupants and unresolved inputs.
`compare_note(id)` shows exact baseline/local/fresh disk bytes (or deletion)
and an `observed_file_state` token without rebasing or writing files.
`reload_note(op, id, expected, discard)` checks the editing stamp and requires
confirmation before discarding dirty/pending work. Changed identities require
`relink_note(op, id, expected, relative, confirm_identity)` instead; relink
retains local text/generation and never guesses identity from content hashes.
Both decisions establish a new baseline token, invalidating queued old edits.
They do not resolve an unresolved original save.

`accept_note_disk_state(ack_op, save_op, observed_file_state)` explicitly
acknowledges a reviewed, freshly revalidated disk state after an original
save's recorded conflict/uncertainty (use reconciliation first for an interrupted
intent without a result). It cannot acknowledge an active job. This is
metadata-only, preserves local work/generation and protected recovery, and
does not rewrite the original outcome, retire uncertain artifacts or grant
search permission. After a confirmed relink it reviews the current registered
location, leaving the original intent's destination and artifacts unchanged.

`save_note_copy(submission, relative)` reserves an independent destination/new
note ID before staging and installs only with `RENAME_EXCL`. Occupied,
registered-missing and possibly aliased destinations are refused with protected
input; no overwrite fallback exists. Copies never inherit search approval.
Copy receipts identify the source and new target separately. An unresolved
original can be rescued without resolving it or suspending the original's
otherwise unchanged eligibility. Copy restart classification requires the
recorded prepared identity and consumed stage, not matching bytes alone.

Name reservations use canonical Unicode decomposition/full case folding plus
resolved parent identity as conservative **vetoes**, never identity proofs.
Qualified local APFS/HFS volumes advertise the required capabilities; other
volume families and unqualified invisible/control names are rejected.
Case-sensitive volumes may be deliberately over-rejected. Replays bind their
recorded destination, even after relink or completed-payload pruning.

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
Save/restart checks are `cargo test -p brn-workflow --test note_recovery --locked`
and `cargo test -p brn-workflow --lib notes:: --locked`; lib tests kill/reap
their own checkpoint-acknowledging children, with no production crash switches.
Conflict/copy decisions are covered by
`cargo test -p brn-workflow --test note_conflicts --locked`; this records the
fixture volume's actual case/normalization equivalence instead of silently
skipping name-collision assertions.
Keep durable authority in store and coordinate provider/retrieval through their
adapters. Preserve evidence validation, operation identity and late-response safety.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
