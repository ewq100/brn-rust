# brn-store

Authoritative SQLite storage: sources/versions, durable operations, local sessions, drafts, immutable revisions, anchored comments and managed-note recovery. Owns migrations, integrity validation and recovery.

## Interfaces and source

[Store API](src/lib.rs), [drafts](src/drafts.rs), [comments](src/comments.rs), [anchor mapping](src/anchors.rs), [managed-note records](src/notes.rs).

## Managed-note storage contract

Schema V6 adds a single registered vault, note/path identities, one exact-byte
editing buffer per note, durable save/copy intents, recovery pairs and
hash-checked tagged receipts. V1–V5 upgrades preserve existing records; existing
chat turns default to `EvidenceCurrentness::Unqualified`. The
`complete_turn_with_currentness` method atomically records terminal text,
status and evidence currentness; `complete_turn` keeps its prior semantics.

The note APIs return typed `NoteResult` failures without changing existing
store error contracts. Text is exact UTF-8, limited to 1 MiB **in bytes**;
generations must fit SQLite's signed integer range. Buffer acknowledgements
establish SQLite recovery, not Markdown publication. Recovery baseline bytes
are not a fresh observation of the saved file.

`note_record(id)` reads the validated registry path, baseline fingerprint,
editing stamp, approval and optional latest observation. `registered_vault()`
reads the singleton vault without requiring a note ID. `note_recoveries()`
lists **all** registered notes, including clean notes and every unresolved
recovery. Neither inspection method needs the filesystem.

`record_note_observation` persists a separately hash-checked observation
fingerprint/token without changing the baseline, buffer or editing stamp.
Identical consecutive observations reuse their token; returning to the
baseline fingerprint uses its file-state token; other changes allocate a new
UUID and invalidate prior version-bound approval. Baseline fingerprints are
also hash-checked. These columns amend the unreleased V6 schema in place;
expected-schema validation derives from the same V6 definition.

V6 also persists checked content/approval epochs, dedicated note/search-snapshot
associations and hash-checked `NoteSearchReceipt` results. Workflow supplies
revalidated reconciled observations to `freeze_note_search_snapshot`; SQLite
atomically freezes immutable bytes and grants only that state's permission.
A different file state receives a new revision even for identical text;
unchanged explicit approval reuses the snapshot. Exact-path imported originals
remain unchanged in `note_shadowed_sources`, including after relink. Observation
withdrawal is durable; replaying a receipt cannot restore withdrawn approval.
Epoch overflow fails atomically, rather than coercing SQLite integers to REAL.
These additions amend the unreleased V6 schema, not shipped databases.

Enrollment binds only the caller-visible root spelling and relative path,
not freshly observed bytes. `note_enrollment_replay` checks that binding before
filesystem access and returns the recorded note ID; differing inputs or
operation kinds return OperationConflict. Existing `enroll_note` callers bind
`vault.root`; `enroll_note_at` additionally accepts the original root spelling
so workflow can retain a canonical registry while binding aliases exactly.
Reopening an enrolled path never replaces its protected editing baseline.

Check `note_write_result` before accessing the vault or validating fresh state.
Operation IDs bind submissions, destinations and write kinds. Replays return
their recorded receipt/failure and never acquire permission to write again.
One unresolved original-path intent blocks another original save even after
startup interruption; independently reserved copies remain allowed.
Only note-specific completion/reconciliation can resolve these writes.
Enrollment cannot allocate a competing identity at a reserved copy destination;
it may reopen the already registered reserved target. Failed refusal recording
returns `Storage` with the original refusal context, actual phase/outcome and
no recovery acknowledgement.

This crate performs **no vault filesystem operations**. The workflow must
verify prepared/installed/displaced identities and durability before supplying
verification or reconciliation. It must also verify artifact retirement or an
unexpected untouched occupant before `record_note_cleanup` records that
bookkeeping. Cleanup is monotonic: `Pending` may become `Retired` only for a
resolved known terminal outcome, or `RetainedUnexpected`; terminal values
cannot be reversed or exchanged. Unresolved, accepted-current and
uncertain/unknown outcomes cannot authorize retirement.

The dedicated `reconcile_note_operation` transition can resolve a proven
pre-exchange failure NotApplied while preserving its exact recorded failure.
A refusal alone is not proof: workflow must supply the matching original
destination observation; missing proof or a substituted result is rejected.

`prune_completed_note_payloads` removes only superseded, successfully completed
payloads with retired artifacts and no protected dependency. The latest
recovery pair/buffer, unresolved work, unexpected artifacts and receipts
survive. Only verified Applied outcomes advance the successful-save recovery
pair, including Applied reconciliation that preserves a historical failure
result; no-op and reconciled NotApplied outcomes leave that pair unchanged.
Pruned operations remain replayable through `note_write_result`;
their full intent is no longer returned by intent reads/lists. A normal
unchanged-save completion creates no artifact; interrupted intent
reconciliation cannot infer artifact absence merely from missing metadata.

`validate_note_submission` provides read-only preflight using the same rules
rechecked by submission transactions. `note_save_result(operation_id)` reads
compact original-save/copy results after pruning (without binding a new payload);
new submissions still use payload-bound `note_write_result` first.
`note_cleanup_candidate` returns exact artifact proof only for known terminal
original saves with durable recovery and no other artifact reference.
It grants no filesystem authority: workflow must freshly verify the occupant,
unlink only the proven regular object, sync its parent, and record retirement.

`note_write_destination` retains a hash-checked compact destination binding
after payload pruning, so a later relink cannot break identical save replay.
`note_original_save_blocker` reports the blocking intent's known-not-applied
Conflict versus uncertain SaveUncertain and directs callers to compare/accept.
Copies remain independent of that original-write block.

`NoteDecision`, `note_decision_replay` and `record_note_decision` bind
reload/relink caller inputs before fresh validation. Decisions check the
editing stamp, active jobs, discard/identity confirmation and exact observations;
they atomically establish a new baseline token without lowering generation.
Relink retains local text; reload replaces it only with the confirmed disk
baseline. Neither decision resolves an outstanding save.

`accept_note_disk_state(ack_op, save_op, observed_file_state, fingerprint, text)`
checks the bound inactive original intent and reviewed observation, sets
`AcceptedCurrent`/`acknowledged_by` in one transaction, retains local
text/generation and leaves the original failure and artifacts untouched.
`note_decision_recovery(ack_op)` reads its hash-checked protected pre-acknowledgement
baseline/local snapshot. Ordinary confirmed reloads do not accumulate historical
buffer snapshots. These additive tables amend the unreleased V6 schema; no
filesystem operation or search approval is performed by the store.

## Dependencies and features

No workspace dependencies. Uses bundled SQLite through rusqlite; consumed by `brn-workflow`.

## Verification

Run from the repository root:

```sh
cargo test -p brn-store --locked
bash scripts/verify-storage.sh
```

Tests cover drafts, comments, workflow records, managed-note transactions and
cleanup, migrations and process-crash recovery. The bounded note check is:

```sh
cargo test -p brn-store --test notes --test storage --test workflow --locked
```

Use disposable data; preserve exact bytes, immutable provenance and
transactional acknowledgement. These checks do not qualify the macOS
filesystem adapter, native editing or live-vault behavior.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
