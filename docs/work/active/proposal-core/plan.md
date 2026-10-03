# Proposal Core — roadmap Stage 4

Baseline: clean `main@d40e0a100de64a173d38a9aae15a547ed9e39551`, 2026-10-03.
Stage 3's scoped live round is complete; remaining capability gaps are recorded
in its completed evidence. No additional live calls are authorized by this plan.

Implement one typed full-proposal lifecycle in the existing six crates. Vault
Markdown stays knowledge authority; WorkStore holds review work and operational
receipts. AI may propose/rewrite; only exact reviewed approval authorizes apply.
Preserve direct manual Save. No generic workflow, repository or agent framework.

Deliver sequential reviewable slices:

1. Durable typed Markdown drafts, exact review versions, full text editing,
   temporary comments, explicit uncertain anchors, stale Rewrite rejection and
   group listings. Shared AppWorker/headless access. This slice has no approval
   or knowledge write endpoint; it is a foundation, not Stage 4 completion.
2. Exact individual/group approval and recoverable whole-proposal application.
   Bind vault/parent/destination/source identities, versions and operational
   revisions. Stage all members before effects, fence current reads throughout,
   preserve proofs on interruption and reconcile before exposing affected state.
   Do not implement application as a sequence of finalized manual Saves.
3. Human-readable activity, practical bounded Undo/Trash and conflict-safe restore.
   Retain ordinary file proofs/receipts needed by unresolved work and Undo.
4. Owned bounded AI Rewrite, native full-proposal review/edit/comment interaction,
   individual/group approve/reject and optional Approve all; headless parity.

Foundation acceptance: supported typed Create/Replace/Trash drafts retain exact
UTF-8 bytes, original before-text and trusted fingerprints/parents; invalid paths,
duplicate targets/identities and oversized review work fail without mutation.
Edits/comments/rejection advance one review version. Late results after edits,
new comments or rejection cannot replace work. Anchored comments bind an exact
UTF-8 range/quote in a change; changed target content leaves the anchor visibly
uncertain, without searching for a new position. Explicit reattachment is allowed.
Review records survive restart and supported additive migration/backups. Drafting,
editing and comments never write a vault file or create a real action.

Whole-stage acceptance: each full proposal is atomic in its logical result and
recoverable across ordinary files and SQLite. Never report partial success or
overwrite an unexpected occupant. Whole-proposal uncertainty blocks affected
current evidence and conflicting Save. Recovery includes member-boundary crashes,
SQLite receipt failure and restoration of operational backups older than file
effects. Exact operation replay never repeats writes. Independent consequences
remain separate proposals in a group. Approval deletes temporary comments only
after success; refusal preserves review work. Undo/Trash preserve subsequent user
bytes and expose conflicts honestly. Late UI/AI acknowledgements preserve newer
review work.

Checks: meaningful store/workflow/worker/CLI lifecycle and interleaving tests;
synthetic macOS file/crash/backup fixtures for apply and Undo; default workspace
format/build/Clippy/tests and integration fixtures; optional native checks for
changed interaction, followed by reproducible disposable-data owner scenarios.
Obtain independent read-only review for each meaningful slice, validate findings,
fix defects, verify fresh, record integration and keep owner/native acceptance
separate. No original-vault inspection, migration, download or release is included.

## Review foundation results — 2026-10-03

Slice 1 is implemented against the baseline above. V4 adds one bounded proposal
review table. Typed drafts retain trusted identities and exact before-text;
transactional full edits/comments/reattachment/rejection use one review version.
Creation replay preserves later manual review work even after external vault
changes. Changed anchored text stays visibly unresolved; stale imported Rewrite
results cannot replace it. AppWorker and the CLI expose the same operations and
drain admitted review mutations. No approval/apply endpoint or AI invocation is
present in this slice. Stage 4 is not complete.

Targeted tests found a CLI `--version` collision with the global version flag;
proposal stamp commands now use `--review-version`. Self-review identified a
writerless FIFO blocking JSON preparation before admission; nonblocking descriptor
open followed by regular-file validation now refuses it, with a bounded subprocess
regression. Two synthetic old-schema downgrade fixtures were updated for V4.
Independent read-only review of the complete tracked/untracked slice, including
these corrections, found no actionable defects. No review-driven scope expansion.

Fresh macOS arm64 / pinned Rust 1.98.1 locked/offline verification:

- Workspace **423 passed, 0 failed, 1 ignored** private crash entry point exercised
  by subprocess Save recovery checks.
- Store suite **76 passed**, including **11 proposal tests**; workflow proposal
  integration **3 passed**, CLI proposal process tests **4 passed**, admitted
  proposal shutdown regression **1 passed** (included in workspace totals).
- Workspace build/all-target Clippy with warnings denied and format passed.
  Optional `native-ui,native-retrieval` desktop check passed; no graphical proposal
  interaction or real inference is claimed. Existing `block v0.1.6` warning remains.
- Existing end-to-end fixture/retirement check passed (**52 assertions**), using
  an exclusive synthetic owned parent outside Git; no live/account calls.
- Changed Markdown links/fragments and `git diff --check` passed.

Commands: `cargo test --workspace --locked --offline`;
`cargo build --workspace --locked --offline`;
`cargo clippy --workspace --all-targets --locked --offline -- -D warnings`;
`cargo fmt --all -- --check`;
`cargo check -p brn-desktop --features native-ui,native-retrieval --locked --offline`;
`TMPDIR=<fresh owned synthetic parent> bash scripts/verify-end-to-end.sh --fixtures-only`.
Earlier unchanged Stage 3 feature probe evidence remains applicable within its limits.

Owner manual acceptance remains pending. Reproduce on disposable data:

1. Build `brn`, create an existing private data folder and empty synthetic vault.
   Put the Create example from `crates/brn/README.md` into `draft.json` and run
   `brn proposals create --file draft.json --vault <vault> --data-dir <data> --json`.
   The full review record is version 1 and `new.md` does not exist.
2. Use that record's `{id, version}` in a whole-proposal comment request, then
   `brn proposals comment --file comment.json ...`. Version advances once. Try
   `rewrite-result` with the old stamp: it must return `CONTEXT_STALE`.
3. Edit the full text with the current stamp, close/reopen the CLI and show/list
   the record. Exact Unicode/BOM/CRLF bytes and comments persist. A text anchor
   whose target changed is unresolved; explicitly reattach with comment-update.
4. Reject with the current `--review-version`. Comments remain, and further
   editing/Rewrite results fail. Verify the synthetic vault still has no new note.
   No approval endpoint is offered until whole-proposal apply is implemented.

Next slice: establish its exact integrated baseline and implement full-proposal
approval/recovery, including all-member preparation, global current-evidence
fencing, retained ordinary file proofs and recovery after an older operational
backup is restored. Per-file Save finalization is not the application primitive.

## Approval journal slice

Baseline: clean `main@89421f5f7ec337b599bec32d5594f0f08a018615`.
Split application into a storage slice followed by filesystem/workflow integration.
This slice adds no user approval endpoint and performs no file installation.

WorkStore freezes the exact Draft review snapshot under an approval operation
UUID, advances the proposal to Applying and allocates one private sibling staging
UUID per member. An additive V5 journal records the creation binding, immutable
approval, complete prepared fingerprints and whole-proposal receipt. At most one
unresolved proposal application is admitted. All preparations are recorded as one
complete set before Applied is possible; the workflow will stage every member and
durably mirror this set before any effects in the following slice.

Applied requires an exact proof for every destination and retained original;
partial or mismatched proof cannot finalize. NotApplied requires every destination
to remain at its original identity (or absent for Create). Uncertain retains the
snapshot, comments and proofs; only explicit reconciliation may settle it. One
SQLite transaction changes journal and review state, and clears comments only for
Applied, including snapshots in all earlier attempts for that proposal. Typed
changes and approval/file bindings stay immutable; temporary annotations are
removed. UUID replay returns the stored result, without accepting another review
version or granting file-write permission. Terminal results are immutable; a
refused proposal returns to Draft at a new version, making old results stale.

The shared current-evidence fence covers both Save and proposal journals,
including startup and model activation. Pending/Uncertain proposals refuse new
Save, Save Copy and reload while keeping editor recovery and proposal review
readable. Direct note/list/refresh/embedding reads also refuse unresolved work;
settling an editor journal cannot clear a proposal fence. There is no file apply
or public approval command in this slice.

Acceptance: stale approval/edit/comment/Rewrite cannot replace admitted work;
complete proofs are immutable and bind exact bytes plus identities; receipt
failure rolls back both review and journal; pending/uncertain/reconciled and
terminal replay survive restart; supported old schemas and backups retain prior
work. Hash, indexed identity and encoded/domain bounds validate journal reads.
Meaningful synthetic store tests, fresh workspace gates and independent read-only
review qualify this slice. Native/owner acceptance and file/crash/older-backup
reconciliation remain pending until the workflow integration slice.

Recovery constraint for that next slice: startup backups precede later approval
effects. Ordinary prepared/completion recovery records must therefore survive a
terminal SQLite commit; deleting them immediately would lose the proof on an
older backup restore. Inspect them on every startup, including a healthy older
database or fresh database, before binding current evidence. An already-settled
historical receipt must not repeat file effects or overwrite subsequent user
bytes. Reuse the existing descriptor/sync/rename mechanics through a narrow
ordinary-file persistence helper; the note preparation API's 1 MiB limit cannot
hold a whole bounded proposal snapshot.

### Journal/fence results — 2026-10-03

Implemented against `89421f5`. V5 freezes the reviewed typed changes and source,
destination, vault, creation and version bindings; full preparation and receipt
proofs are transactional and checked on read. Same-request replay remains valid
after restart and after later Draft edits following refusal. Current-evidence
guarding now includes both journals across startup/model activation and direct
reads; Save/reload cannot bypass unresolved proposal work. Editor recovery stays
available. No public approval endpoint or file installation is present yet.

Self-review added an edited-snapshot creation-binding read check. Independent
review found that successful approval retained temporary comments in snapshots,
including older refused attempts. Verified and fixed: the same finalization
transaction removes them from the live review and all journals for that proposal,
without changing old receipts/proofs/versions or unrelated review work. Failure
at any of the three write boundaries rolls the cleanup back. Bounded independent
re-review found the correction sound, with no other actionable findings. The
review also correctly limits current tests: immediate pre-commit fencing belongs
to future workflow approval admission, before any file work.

Fresh final macOS arm64 / pinned Rust 1.98.1 locked/offline verification:

- Workspace **443 passed, 0 failed, 1 ignored** private crash entry point exercised
  through subprocess recovery tests. Store **94 passed**, including **18 approval
  journal tests**; workflow **2 new barrier tests**, **3 existing proposal tests**
  and **62 library tests** passed (included in workspace totals).
- `bash scripts/verify-end-to-end.sh` passed the retirement, workspace format,
  build, all-target Clippy with warnings denied, tests and **52 fixture assertions**.
  It used an exclusively created synthetic parent, then removed only that parent's
  test files; no account/network/model calls or original data access.
- `cargo check -p brn-desktop --features native-ui,native-retrieval --locked --offline`
  passed. The existing `block v0.1.6` future-compiler warning remains.
- Changed local Markdown links/fragments and `git diff --check` passed.

This backend slice is locally integrated with the current mission authorization;
Stage 4 remains active. There is no new approval UI for owner acceptance yet.
Existing headless draft acceptance above and native Save acceptance remain pending.
The guards can be reproduced on disposable fixtures with
`cargo test -p brn-workflow --test proposal_barrier --locked --offline`; receipt
rollback/restart/cleanup cases use
`cargo test -p brn-store --test work_proposal_apply --locked --offline`.
Next: workflow whole-proposal installation, ordinary recovery records and exact
approval/reconciliation commands, then activity/Undo and native/AI review.

## File application and recovery slice

Baseline: `main@e84ef7a558735f8d4469e48ff1321edf0e81e552`, with an unrelated
owner edit to AGENTS.md preserved. Its new product-discovery rule does not change
this accepted frozen behavior or require another product interview.

Expose exact approval/reconciliation through AppWorker and the CLI. Preflight
the reviewed vault, parents, target/source fingerprints and editor recovery;
then fence retained tools before admitting the intent. Stage all Create/Replace
members; Trash binds its existing file. Persist the complete prepared set in
SQLite and a hash-checked ordinary recovery record before the first namespace
effect. Reuse the existing coordinated sibling exchange/exclusive rename and
required file/directory durability. Verify every installed/displaced member
before reporting one whole-proposal result. Never loop finalized manual Saves.

An operation known to fail before any namespace-effect attempt can record a
typed no-effect refusal even if an external occupant/source changed. This keeps
that occupant and returns the proposal to Draft. Interrupted or attempted work
without whole proof remains Uncertain; replay cannot repeat a write. The storage
extension distinguishes this verified refusal from proof-based reconciliation.

Ordinary records live in the existing data directory and are bounded full-journal
snapshots with checksums, descriptor/identity validation, exclusive staging,
atomic replacement and required file/directory sync. They are recovery receipts,
not another database. Persist the observed completion before SQLite finalization
and retain it across startup backups. Successful approval also removes temporary
annotations from previous recovery snapshots. Do not remove unexpected artifacts.

Startup inspects records before current vault binding, restores missing/older
operational journals transactionally and reconciles proof without installing
again. Healthy older databases and missing/corrupt restores take the same path.
Retain newer review work on conflict, fail closed on incompatible bindings and
never downgrade a settled receipt. An already-settled historical receipt must
survive subsequent user file edits without treating them as a new approval.

Acceptance covers multi-member Create/Replace/Trash exact bytes, pre-effect
refusals, source/parent/occupant changes, partial effects, per-member process
crashes, preparation/receipt/sync failures, replay, earlier database restoration,
retained-tool fencing and concurrent/later editor text. Current reads and new
Save stay fenced until reconciliation; review and recovered text remain readable.
Individual proposals stay the atomic units; group requests bind an explicit
membership/version set and return each result, stopping at the first refusal.
No new arrivals are implicitly approved. Add meaningful store/file/workflow/
worker/CLI tests, independent review and fresh gates. Owner/manual/native and
physical power-loss qualification remain distinct and pending.

### File application/recovery results — 2026-10-03

Implemented against `e84ef7a`. Individual and explicitly captured group approval,
reconciliation and journal listing use AppWorker/CLI. Every member is prepared
before namespace effects; whole proofs and required durability precede one
Applied receipt. Runtime pre-effect refusal is a pending-only N+2 certificate;
restart never invents it. Mixed/unknown work retains proofs/comments and fences
current reads and Save. Dirty editors bind namespaces/identities as well as
literal paths; subsequent old-stamp typing stays recoverable after application.
Group syntax is checked before admission, each proposal is independent and the
result exposes every receipt and stopped member without including new arrivals.

Ordinary full-journal records are bounded/hash checked, descriptor-bound and
durably exchanged. They remain across SQLite backups; every startup inspects
them before binding current evidence. Terminal imports run newest-first, with
older journals becoming history while newer review work remains intact. Historical
completion never checks/repeats old file effects against subsequent owner edits.
Unfinished proof still checks current source bindings, recognizing sources changed
by that approved proposal. Identical-record replay re-establishes file/directory
durability after a possible earlier sync failure. Canonical snapshots stay retained;
successful approval removes temporary annotations from all covered journals and
proof-checked compatible temporary snapshots, preserving unexpected occupants.

Independent review found three defects: interrupted whole-file reconciliation
omitted source revalidation, dirty editors under equivalent filenames were missed,
and interrupted receipt replacement could retain comments in temporary snapshots.
Verified and corrected all three with behavioral regressions. Pure captured-group
validation also prevents a malformed later member from causing earlier effects.
An additional healthy-older-database test exposed cleaned refused snapshots being
imported before later Applied evidence; newest terminal ordering and narrowly
bounded historical catch-up fix this while preserving newer review work. Independent
re-review found no remaining actionable defects. No scope/architecture expansion.

Fresh final macOS arm64 / pinned Rust 1.98.1 locked/offline verification:

- Workspace **498 passed, 0 failed, 2 ignored** private crash entry points exercised
  through subprocess recovery matrices. Store **110 passed**, including **34
  approval/recovery tests**; CLI **67 passed**, including **9 proposal process tests**.
- New workflow qualification includes **16 ordinary-receipt helper tests**, **10
  protocol tests**, **6 approval integration tests** and **2 worker tests**. Protocol
  fixtures cover 13 process boundaries, each member, ten recovery-record sync fault
  combinations, preparation and SQLite receipt failure, annotation cleanup, stale
  sources, aliases, later typing and older Draft/pending databases. These counts
  are included in workspace totals; they are synthetic APFS process qualification.
- `TMPDIR=<exclusive owned parent> bash scripts/verify-end-to-end.sh` passed the
  retirement, format/build/all-target Clippy with warnings denied, workspace tests
  and **52 fixture assertions**. Root's final normal execution passed current CLI
  process tests; helper-environment coordination failures prompted no workaround.
- `cargo check -p brn-desktop --features native-ui,native-retrieval --locked --offline`
  passed. Existing `block v0.1.6` future-compiler warning remains. Markdown links
  and `git diff --check` passed. Only the exclusive synthetic parent/test leftovers
  were removed; no live/account/model calls or original data access occurred.

Owner/manual acceptance remains pending. Reproduce on disposable data outside
Git using the typed Create example and commands in `crates/brn/README.md`:

1. Create an empty vault and existing data folder. Run `proposals create --file
   draft.json --vault <vault> --data-dir <data> --json`, then comment and show the
   full current version. The note must remain absent until approval.
2. Run `proposals approve <id> --review-version <current> --operation <fresh UUID>`
   with the same vault/data flags. Exact bytes appear, state is Applied and comments
   are empty. `proposals applies` shows complete prepared/installed proofs.
3. Repeat the exact approval and `proposals reconcile <operation>` across restarts:
   receipt is unchanged. Explicitly edit the synthetic saved file externally and
   repeat: later bytes remain, with no new installation.
4. For Replace/Trash, bind fingerprints from `edit open`; retain original bytes in
   the operation stages. Change a target/source or retain dirty editor text before
   approval: it must refuse without overwriting that work. Captured group approval
   stops at a refused member and never includes another group arrival.

Native proposal interaction, owner acceptance, physical power loss and other
volumes remain unqualified. Partial/mixed proof is safely fenced; bounded repair,
activity/Undo/Trash retention, AI Rewrite and native review are the next slices.
This file-application slice is locally integrated under the mission authorization;
Stage 4 remains active and is not declared complete.

## Readable activity slice

Baseline: `main@e04fc52a857263c9ec9548890916f7cc65e13a60`; preserve the unrelated
owner AGENTS.md edit. Project the existing terminal Applied journals into a bounded
human-readable history, without another database or event framework. Include the
approved title, affected paths/kinds, operation/proposal/group/session identities
and the recorded approval-admission time. Do not call this an exact completion
time. Drafts, refusals and uncertain applications cannot claim a completed durable
change. Current-evidence fences do not hide already-settled historical activity.

Expose the same page through AppWorker and `brn activity list`; default to 20,
allow 1–100 entries, and use an existing Applied operation UUID as the exclusive
older-page cursor. Order by approval time then UUID, newest first. Historical
entries remain stable after later note edits, restart and ordinary receipt restore;
default output excludes note bodies, review comments, credentials and raw tool
transcripts. Native presentation follows with the complete proposal interaction.

Acceptance/checks: meaningful projection, pagination, current-fence/restart and
CLI process tests; invalid syntax before workspace admission, stale cursors refused;
independent read-only review, fresh integrated gates and optional native compile.
Record a disposable-data manual scenario and keep owner acceptance pending.
Next slice implements bounded Undo/Trash restore while retaining originals; recent
Undo eligibility must never silently permanently delete retained user knowledge.

### Activity results — 2026-10-03

Implemented against `e04fc52`. Activity uses existing checked Applied receipts;
it does not duplicate history storage or claim partial/uncertain work succeeded.
One checked journal is decoded at a time, with only scalar full-history ordering
metadata retained. Pages and human output expose approval time, title, summary,
paths and cause identities; they exclude bodies, comments and proofs. Exclusive
cursors disambiguate equal timestamps and refuse unknown/unapproved operations.
Historical output survives later bytes, unavailable vaults, unresolved current
work and ordinary-receipt recovery of a fresh operational database.

Independent review found that the startup current-evidence guard still loaded
all history bodies before the activity request. Verified and corrected the guard
to enumerate IDs and check each journal in turn, still checking every row without
short-circuiting after an unresolved record. Independent re-review found no
remaining actionable defect. The helper's targeted normal CLI run passed; a
broader helper rerun encountered an intermittent macOS coordination refusal before
approval. Root's complete final normal run passed after the correction, without
a source workaround or permission escalation; this remains a qualification limit.

Fresh final macOS arm64 / pinned Rust 1.98.1 locked/offline checks:

- `TMPDIR=<exclusive owned parent> bash scripts/verify-end-to-end.sh`: retirement,
  format/build/all-target Clippy with warnings denied, **507 workspace passed,
  0 failed, 2 ignored** private crash entry points exercised through subprocess
  tests; **52 end-to-end assertions**. CLI **72 passed**; new checks include
  **3 workflow activity integrations**, **1 timestamp unit**, **3 CLI processes**
  and **2 output units**, included in workspace counts.
- Store **34 approval/recovery tests**, workflow **2 proposal barriers** and
  independent activity/CLI tests passed. Optional native desktop compile passed;
  existing `block v0.1.6` future-compiler warning remains.
- Changed Markdown **38 local links/fragments, 0 errors** and diff check passed.
  Only verified exclusively owned synthetic gate parents were removed. No live
  account/model calls, original-data inspection, push or release was performed.

Manual acceptance remains pending: approve two disposable Create proposals using
the CLI README example, run `activity list --limit 1`, fetch the returned exclusive
older cursor, then edit one note externally and restart. The same two historical
entries should remain, with later bytes untouched and no private body/comment
content in default output. Native history presentation follows in the review UI.
This slice is locally integrated under the standing mission authorization; bounded
Undo/Trash and AI/native review remain required before Stage 4 is complete.

## Bounded Undo/Trash slice

Baseline: `main@7100ab2f6d890b78c4b9c72fe484f15004a004ca`; preserve owner AGENTS.md.
Explicit human Undo identifies one immutable Applied operation. Derive its exact
inverse as another bounded typed proposal/application: Create → Trash, Replace →
Replace with retained original, Trash → Create from retained original. The new
operation has its own UUID and recoverable whole-proposal receipt; original UUID
replay remains historical and never installs again. AI has no direct Undo tool.
An explicit Restore Trash can select one original Trash member, producing a
single-member Create with its exact retained inode. This remains useful when other
members of the old mixed proposal have later owner edits; those files are outside
the new restoration. Whole Undo never silently narrows to a subset. The source
operation and whole/single-Trash scope both bind replay and recovery.
Preview exposes the complete inverse; an explicit identified human Undo is a direct
operation under the frozen explicit-user-command exception. Ordinary manual edits
and approvals of a refused inverse remain ordinary reviewed proposal behavior.

Keep the same NoteChange types, journal/fence and whole-application protocol.
An optional, omitted-when-absent Undo binding names the source operation and exact
retained member IDs/fingerprints. Fresh admission cross-checks the terminal source.
Inverse Create/Replace borrows its proven retained inode as staging, preserving
mode/ACL/xattrs and original identity; inverse Trash uses a new stage. Validate the
binding through preparation, immutable recovery merge and temporary-file cleanup.
Before effects, borrowed stages equal the originals; after effects, ordinary whole
Applied proofs hold. Interrupted NotApplied also requires unchanged borrowed-source
proof. Older snapshots remain hash compatible; missing/older database recovery is
self-contained and does not need a new source foreign key or replay ordering graph.

Preflight checks exact current targets/absence, all retained originals, vault/root/
parents and editor aliases. Dirty recovery refuses. A clean old editor baseline may
be retained only when it exactly matches the original inode/text being restored;
never rebase its token/generation or overwrite queued later typing. Stage/flush all
members before effects, retain full proofs on interruption and never retry namespace
effects implicitly. One Undo is bounded by the existing 64-member/8 MiB limits;
default activity pages expose recent work, while older Trash remains restorable if
its exact proofs still hold. No timer/count silently purges Trash or superseded bytes.

Implement storage derivation/admission first, then workflow/files/worker/CLI and
native interaction. Storage acceptance: exact inverse/binding, invalid or unrelated
source/UUID refusal, atomic inverse-review/journal admission/rollback, immutable
proof/replay, old JSON/hash compatibility and source-independent receipt restore.
Workflow acceptance: byte/identity/attribute restoration, changed occupants/sources/
parents and dirty aliases refuse, queued typing survives, crash/member/sync/receipt/
older-backup cases reconcile without repeating effects, whole mixed work stays
fenced. Use meaningful synthetic tests, independent read-only review and fresh
relevant gates; native/manual/physical-power-loss qualification remains separate.

Storage-only implementation derives exact whole and selected-Trash inverses,
atomically admits review plus apply intent, fixes immutable bindings and preserves
legacy omitted-field JSON/checksums. It performs no vault writes or purges.
Independent review identified a valid near-limit 64-Trash source whose inverse
could prepare but fail Applied settlement after its Undo manifest consumed the
old metadata cap. The helper reproduced that failure; follow-up review also
reproduced core growth when original fingerprints had wider integers than the
installed fingerprints. Final validation retains the ordinary core cap and uses
a stable normalized path/member base for Undo, with separate fixed allowances of
33,792 bytes for its manifest and 100,352 bytes for proof/receipt/time slots. This
prevents inverse chains from consuming more headroom. Regressions complete a
near-cap 64-Trash inverse and its inverse, and four 64-Replace inverse/recovery
cycles, each without requiring the prior source row. Legacy None JSON/caps remain
unchanged. No requirement/architecture change was needed. Final independent re-review found no remaining actionable defect and independently
passed both boundary regressions. Root's fresh macOS arm64 / Rust 1.98.1
locked/offline `TMPDIR=<exclusive owned parent> bash scripts/verify-end-to-end.sh`
passed **524 workspace tests, 0 failed, 2 ignored** private crash entry points
exercised by subprocess matrices; **52 fixture assertions**, retirement, format,
build and all-target Clippy with warnings denied. Store **127 passed**, including
**17 Undo tests** and **34 approval/recovery tests**; optional native desktop
compile passed. Changed Markdown **22 local links/fragments, 0 errors** and diff
checks passed. Verified exclusively owned synthetic gate parents were removed;
no live calls, original data, push or release. This storage-only slice is locally
integrated under mission authorization. File execution and native/manual
qualification remain in subsequent shared execution/UI work.

Next shared execution slice will reuse Store `UndoRequest`, `UndoPreview` and
`UndoBinding` unchanged. `App::preview_proposal_undo` is operational read-only;
`App::undo_proposal` performs an explicit identified human operation, replay-first.
AppWorker adds `PreviewProposalUndo` / `UndoProposal` and `ProposalUndoPreview`,
with the existing `ProposalApplied` receipt and mutation shutdown drain. Fresh
preflight shares ordinary approval's vault/source/target/editor checks, plus exact
borrowed originals. Preparation flushes those originals without copying them;
the existing whole application/reconciliation protocol installs the inverse.
Activity adds optional source-operation and selected-Trash-member context so a
short valid title cannot hide that the recorded change was Undo/restoration.
CLI adds `undo-preview TARGET --operation NEW [--member INDEX]`, `undo TARGET
--operation NEW` and `restore-trash TARGET --member INDEX --operation NEW`, with
pure request validation before opening storage. Native interaction remains next.
