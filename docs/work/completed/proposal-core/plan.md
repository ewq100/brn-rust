# Proposal Core — roadmap Stage 4

2026-10-04: implemented, automated verified and locally integrated as sequential
reviewed slices. Actual GUI/IME/accessibility, live Rewrite and owner acceptance
remain pending. Continue Stage 5; these qualification items do not block its safe
implementation. The sections below retain each slice’s original baseline/results.

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

Shared execution baseline: `main@e593da83a985345cfb78e60bab82dc781cd1d525`;
Store admission is integrated, owner AGENTS.md remains unrelated. Acceptance starts
with failing public integration checks for exact byte/inode/mode/ACL/xattr recovery,
editor stamp and later typing preservation, scoped Trash with unrelated later edits,
changed original/target/root refusal, replay and fresh-DB history restoration.

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

Shared execution implements the fixed interfaces above and reuses one preflight/
admitted-run path for ordinary approval and Undo. Borrowed originals are checked
before admission and preparation, required file/directory flushes complete before
whole prepared evidence is mirrored, then existing installation/proof settlement
runs. Editor matches include namespace, current inode and borrowed original inode;
clean original baseline/text is allowed without rebasing, dirty aliases refuse.
Temporary-record retirement now requires matching Undo bindings. Activity names
the source/scope, and CLI defaults to full typed preview plus explicit operation IDs.

Targeted normal root checks passed the four public workflow integrations and the
14-boundary Undo subprocess matrix, mirror-persistence failures and changed
retained-original uncertainty. Worker tests cover real correlation/shutdown/later
old-stamp typing. The CLI helper encountered macOS coordination refusal during
its initial editor fixture setup; root's normal process run reached all Undo paths.
One CLI assertion expected ToolRejected for source-dependent Store Invalid, contrary
to the existing intentional uncategorized mapping; corrected the assertion without
changing product behavior. Final read-only review and fresh gates follow.

Independent read-only final review against `e593da8` found no actionable findings;
it independently passed **4 workflow integrations, 2 worker tests and 4 CLI
process tests**. Root's fresh macOS arm64 / Rust 1.98.1 locked/offline
`TMPDIR=<exclusive owned parent> bash scripts/verify-end-to-end.sh` passed
**538 workspace tests, 0 failed, 2 ignored** private process entry points exercised
by matrices; **52 fixtures**, retirement, format/build/all-target Clippy with
warnings denied. Optional native desktop compile passed; existing upstream `block
v0.1.6` warning remains. Store **127 passed**, CLI **77 passed**. The new Undo
subprocess matrix checks **14 crash boundaries**; ordinary-mirror persistence
failures and changed borrowed originals preserve exact whole outcomes. No physical
power loss, native Undo interaction or owner usability acceptance is claimed.

Manual acceptance pending: in fresh disposable data/vault, approve a mixed
Replace/Create/Trash proposal. Run `proposals undo-preview SOURCE --operation NEW`
and inspect all full inverse changes without file effects, then `proposals undo
SOURCE --operation NEW`. Compare exact originals and verify the created note moved
to retained Trash. Replay NEW after a later external edit: preserve that edit and
return the old receipt. Separately approve another mixed proposal, edit its Replace
note externally, then preview/execute `restore-trash SOURCE --member INDEX
--operation NEW` for its zero-based original Trash member: restore only that file.
An occupied destination or changed original must refuse, preserving every file.
[CLI commands](../../../../crates/brn/README.md) document the concrete invocation.
Shared execution is locally integrated under standing mission authorization;
explicit interrupted-work repair, owned Rewrite and native review/Undo are next.
Changed Markdown **54 local links/fragments, 0 errors** and diff checks passed;
only verified exclusively owned synthetic gate parents were removed. No live
calls, model downloads, original/private-data inspection, push or release.

## Explicit interrupted-operation repair slice

Baseline: `main@7f97d65fa01e3dbea68540e60a24d407461ce0b4`; preserve owner AGENTS.md.
Repair is a separate explicit human command for one unresolved, already-approved
operation. Preview the full original proposal and exact current member phases;
Finish installs remaining approved changes, Restore returns applied members to
originals. No automatic namespace retry occurs at startup, reconciliation or UUID
replay. Unknown occupants/proofs refuse both directions and remain fenced.

Keep existing NoteChange types, filesystem primitives, whole receipt and current-
evidence fence. A known complete prepared set yields each member's exact Before
or Applied phase. Capture a SHA-256 review stamp over immutable approval/member/
prepared/Undo bindings, prior repair UUIDs and the complete observed proof vector.
Each human attempt has a fresh repair UUID and direction. One optional journal
repair binding keeps at most 64 compact attempts, latest admission proofs and
immutable request hashes; absent fields retain legacy JSON/checksums. At most the
latest attempt is pending; earlier uncertain attempts remain historical. Resource
caps are checked before admission/effects. No new database/table/framework is needed.

Store first: typed direction/request/preview/receipt, phase classification/hash,
atomic replay-first admission, immutable forward history/recovery merge and latest
attempt outcome. Original approval/comment version remains unchanged by admission;
terminal whole receipt and latest outcome settle together. Normalize bounded path
metadata for retained repair journals and reserve fixed proof/repair slots, keeping
ordinary absent-field limits unchanged. Recovery can restore the repair binding
without another source row. Then shared execution: exact capture/preflight, durable
repair mirror before effects, skip already-desired phases, coordinated proof-checked
forward/reverse moves, required flush and whole reobservation. Finish checks sources
against arbitrary phases; Restore does not overwrite unrelated external sources.
Editor aliases/buffers/stamps remain protected; later queued typing is retained.

Acceptance: all mixed subsets of Create/Replace/Trash in both directions; exact
bytes/identity/attributes; changed stage/destination/root/source and dirty aliases
refuse; request/direction/stamp conflict and replay do not write; crashes, mirror/
receipt failures and older/missing SQLite recover without implicit effects; whole
settlement alone releases retained tools/read/Save fences and applied annotations.
Use bounded synthetic tests, independent read-only review and fresh relevant gates.
Native/manual/physical-power-loss acceptance remains separate.

Store-only foundation implements the fixed DTOs, pure exact phase capture/stamp,
atomic replay-first admission, latest-attempt settlement and forward recovery. No
new table/schema or filesystem effect was added. Twelve regressions cover all
subsets/directions, unknown/corrupt capture, immutable first uncertainty, rollback,
UUID collisions, forward-history forks, omitted legacy checksums, actual Undo
bindings and 64 attempts on a near-cap 64-Replace journal followed by settlement
and source-independent restoration. Independent read-only review against `7f97d65`
found no actionable defects and independently passed all 12 repair tests.

Fresh root macOS arm64 / Rust 1.98.1 locked/offline `TMPDIR=<exclusive owned parent>
bash scripts/verify-storage.sh` passed **550 workspace tests, 0 failed, 2 ignored**,
format/build/all-target Clippy with warnings denied and real headless AppWorker
startup. Store **139 passed**. The unchanged private process entry points remain
ignored directly and are exercised by crash matrices. Store-only code is ready
for local integration under standing authorization; filesystem repair, CLI/native
interaction, manual usability and physical power-loss qualification remain separate.
Changed Markdown **24 local links/fragments, 0 errors** and diff checks passed.
Only five proven owned synthetic layout fixtures and their gate parent were removed; no live calls, model
downloads, original/private data, push or release actions occurred.

Shared execution uses `App::preview_proposal_repair(operation_id)` and
`App::repair_proposal(RepairRequest)` with typed worker commands/events. Preview
includes the full original approved draft, exact current phases and capture hash.
CLI will expose `proposals repair-preview OPERATION_UUID` and `proposals repair
--file REQUEST.json`; typed JSON binds a fresh attempt UUID, original operation,
preview hash and Finish/Restore direction. Replay resolves prior attempts before
fresh filesystem/editor checks. The prepared shared child module remains
unregistered until Store integration, then its tests will run against that baseline.

Store foundation integrated as `86439cf`. Shared repair implements the fixed APIs
above against that exact baseline, with one private child module, arbitrary-phase
source checks, durable attempt mirrors, coordinated forward/reverse moves, full
flush/endpoint proof and original-receipt/latest-attempt settlement. Checked
canonical repair ancestry protects temporary cleanup across older operations.
AppWorker drains repair; CLI implements the documented typed preview/request
commands with pre-storage validation and exact attempt-ID correlation.

Targeted checks passed **7 private shared tests** (16 phase/direction cases,
18 crash boundaries, mirror failures, SQLite failures, older/missing databases,
source/unknown/dirty-case aliases and historical replay), **2 real worker tests**
(correlation/drain/queued later typing/restart), **5 CLI process tests** plus
**31 binary unit tests**, and canonical temporary-history cleanup. Review found a
valid generic NotApplied gap: unchanged destinations could discharge a repair
despite a missing prepared stage. A fresh failing regression reproduced it; Store
now requires terminal repair observations to prove every exact Before/Applied pair.
The added Store atomic check and workflow refusal/restart regression passed;
**13 Store repair tests** passed. Ordinary absent-field behavior remains unchanged.
Final independent review and fresh integrated gates follow.

Manual acceptance pending: in disposable data/vault with an interrupted mixed
proposal, run `proposals repair-preview OPERATION_UUID` and inspect the full draft,
phases and hash. Submit `proposals repair --file REQUEST.json` with a fresh attempt
UUID, original operation, exact captured hash and `finish` or `restore`. Finish
must install the complete approved endpoint; Restore must restore all originals
and retain proposed staging. Repeating the attempt after a later external edit
must preserve that edit and return history. A changed stage/destination, stale
capture or dirty alias must refuse and keep affected current reads/Save fenced.
See the [CLI scenario](../../../../crates/brn/README.md). Native interaction and
physical power loss remain separate qualification.

Final read-only review against `86439cf` found no remaining actionable defects;
it independently passed **13 Store repair, 7 shared repair, 2 worker, 5 CLI process,
1 canonical cleanup and 1 CLI correlation check**. The endpoint finding is verified
and resolved. Fresh root macOS arm64 / Rust 1.98.1 locked/offline
`TMPDIR=<exclusive owned parent> bash scripts/verify-end-to-end.sh` passed
**567 workspace tests, 0 failed, 2 ignored**, **52 fixtures**, retirement,
format/build/all-target Clippy with warnings denied. Store **140 passed**, CLI
**83 passed**. `cargo check -p brn-desktop --features native-ui,native-retrieval
--locked --offline` passed, with the existing upstream `block v0.1.6` warning.
Builds and synthetic checks do not establish native usability or physical durability.

Current AGENTS.md additionally requests the thin `developing-product-feature`
router and hosted CI inspection. Read the installed router; frozen accepted roadmap
work proceeds within BRN's workflow, without discovery or routine approval gates.
GitHub reports no run for local Store baseline `86439cf`. The inspected latest
[published main run](https://github.com/ewq100/brn-rust/actions/runs/37137393000)
at `609d859` failed Windows core/native builds and optional Linux retrieval/UI
tests; macOS jobs passed there. Those different-commit results are not evidence for
this candidate. Hosted CI remains pending local-only integration; no push/public
distribution is performed. Manual acceptance and native interaction remain pending.
Changed Markdown **68 local links/fragments, 0 errors** and diff checks passed.
Only five proven owned synthetic layout fixtures and their gate parent were removed.
Shared repair is locally integrated under standing mission authorization; owned
Rewrite and the native proposal/review/activity/Undo/repair interaction are next.

### Owned Rewrite slice

Baseline: `main@b5c8000`, preserving the owner's unrelated AGENTS.md edit. Reuse
the owned chat lane, selected provider/model, explicit low/medium/high effort,
existing bounded read tools, cancellation and disconnect/shutdown drain. Capture
the full exact Draft and temporary comments at admission. One narrow typed V6
job record in existing brn.sqlite retains identity, capture digest and safe outcome;
it never duplicates captured comments, prompts or raw provider output into chat.
Restart interrupts running jobs without resubmission. The validated full result
and exact-version proposal edit settle atomically; intervening edits/comments or
approval yield Stale without overwriting newer work. Rewrite does not apply notes.

Implement Store admission/atomic settlement and the thin bounded Rig adapter at
fixed interfaces, then shared worker/CLI routing. Accept exact Unicode/full-member
output, bounded strict JSON, replay before fresh account/vault access, Stop and
targeted disconnect, retained read lease drain, stale CAS, rollback and restart
without retry. Run meaningful synthetic checks, independent read-only review and
fresh relevant gates before local integration. Native interaction and owner/live
Rewrite acceptance remain pending; existing provider permission is exhausted.
No extra live calls, model downloads, private data, push or release are authorized.

The fixed Store/Rig/worker/CLI interfaces are implemented. Meaningful checks cover
full Unicode/comment capture, every typed member, no knowledge writes, exact
stamp/state/hash races, full-output bounds, atomic rollback, UUID collisions,
restart interruption, queued/active Stop, retained read drain, provider disconnect,
shutdown/panic/refusal and historical replay after vault loss. Review found two
valid gaps: Running replay advertised a new generation's Started without its
future terminal event, and nested Selection extras bypassed strict input preflight.
Fresh failing regressions reproduced both. Running replay now returns distinct
AlreadyRunning history and immediate safe CLI conflict; the original owned job
continues. A narrow selection deserializer refuses nested extras before storage,
preserving existing shared Selection compatibility. Both fixes passed re-review.

Final independent review against `b5c8000` found no remaining actionable defects
and independently passed **13 Store, 11 workflow Rewrite, 8 AI Rewrite, 36 CLI
unit, 5 CLI process and 1 extended shutdown checks**. Fresh root macOS arm64 /
Rust 1.98.1 locked/offline `TMPDIR=<exclusive owned parent> bash
scripts/verify-end-to-end.sh` passed **609 workspace tests, 0 failed, 2 ignored**,
**52 end-to-end assertions**, retirement, format/build/all-target Clippy with
warnings denied. Cargo target metadata and the fresh log establish **153 Store,
76 AI and 93 CLI tests**. The ignored process entries are exercised by the
existing crash matrices. Optional `cargo check -p brn-desktop --features
native-ui,native-retrieval --locked --offline` passed with the existing upstream
`block v0.1.6` warning. Changed Markdown **92 local links/fragments, 0 errors**
and diff checks passed. Hosted CI reports no run for local baseline `b5c8000`;
other-commit platform failures recorded above do not qualify this candidate.

Manual acceptance pending: use a disposable vault/data folder to create a full
typed proposal and temporary comments. Under separate live-call authorization,
submit `proposals rewrite --file REQUEST.json` with its exact review stamp, a fresh
job UUID, explicit selection/effort and generation. Inspect the complete revised
proposal; vault bytes must remain unchanged until approval. Add a comment/edit
while Rewrite runs, then verify Stale preserves it. Stop and restart must preserve
the draft and report Interrupted without another request. `rewrite-status JOB_UUID`
reads history offline; replay after moving the vault must return history. See the
[CLI scenario](../../../../crates/brn/README.md). Native interaction, actual live
Rewrite usability and owner acceptance remain separate, pending qualification.
Only proven owned synthetic fixtures are removed; logs remain outside Git.
Owned Rewrite is locally integrated under standing mission authorization. Native
full review/edit/comment/Rewrite/approval/activity/Undo/repair is the next slice.

### Explicit main reasoning effort

Baseline: `main@5f26e66`; post-integration hosted CI has no run for this local
commit. Native source investigation is complete. Before wiring its controls,
close the concrete Product Vision §28 gap: Rewrite freezes explicit effort, but
ordinary Ask currently sends provider defaults and chat history records no effort.
Use the existing low/medium/high enum, a separate explicit WorkStore setting and
one additive V7 message column. Existing turns retain unknown/None effort and
remain readable/replayable; no history resubmission or silent default is added.

Fix interfaces first: WorkTurn gains optional string effort; old begin_turn stays
as historical-fixture compatibility, while begin_turn_with_effort binds the new
field in paired rows/replay. AskRequest gains optional typed effort, required for
fresh provider admission but reconstructed exactly for history replay. A thin
answer_with_effort uses the same Rig stream/read tools with exact route parameters.
Workflow owns explicit effort getter/setter and selection persistence; CLI/native
settings expose low/medium/high. Changing the saved choice cannot change active
or historical requests. No live calls, extra datastore or generic framework.

Accept migration/backup preservation, row agreement/validation, effort-conflicting
UUID replay refusal, fresh missing-choice refusal before account access, selected
wire effort with unchanged history/progress/tool budgets, and settings/history
correlation. Verify meaningful deterministic tests, independent review and fresh
gates, then continue native full review. Actual live/native owner acceptance is
pending and does not block safe implementation.

Implementation preserves the historical `answer` seam while fresh product Ask
uses `answer_with_effort`; no provider default is admitted. V7 is additive and
both stored rows bind/validate effort. Current settings never participate in
historical replay. Native settings await the exact getter/setter acknowledgement
and retain active/history capture. The initial startup-whitelist test exposed its
outdated offline command list; adding the harmless Effort getter fixed the fixture.

Fresh root macOS arm64 / Rust 1.98.1 locked/offline
`TMPDIR=<exclusive owned parent> bash scripts/verify-end-to-end.sh` passed
**628 workspace tests, 0 failed, 2 ignored**, **52 end-to-end assertions**,
retirement, format/build/all-target Clippy with warnings denied. Target metadata
and the fresh log establish **160 Store, 79 AI, 97 CLI, 181 workflow, 80 desktop
and 31 retrieval** tests. The ignored private crash entry points are exercised by
subprocess matrices. Optional native desktop tests passed **93**, and its native
build/all-target Clippy passed with the existing upstream `block v0.1.6` warning.
No GUI interaction, live effort/provider or owner acceptance is inferred.

Complete independent read-only review against `5f26e66` found no actionable
defects, and independently passed **7 Store, 3 AI, 3 workflow, 2 desktop and
13 CLI process tests** plus the diff check. Hosted CI has no run for the local
baseline; published results from another commit do not qualify this slice.
Changed contracts/status/links are checked before local integration. Only the
five proven owned synthetic layout fixtures and their exclusive gate parent are
removed; logs remain outside Git. This slice is locally integrated under standing
mission authorization, preserving the unrelated owner AGENTS.md edit.

Manual acceptance pending: launch with a new disposable data/vault folder, open
Settings and choose low/medium/high. Reopen and confirm the selected value;
provider/model/history must remain independent. Ask must remain disabled until
a choice is acknowledged. Under separately authorized live scope, start an Ask
with high, change Settings to low while it runs, and verify the finished history
still records high; the next Ask captures low. Pre-V7 history shows unavailable
effort and replays offline without another request. The [CLI scenario](../../../../crates/brn/README.md)
provides the offline getter/setter/missing-choice check. Native full proposal
review is next; owner acceptance does not block that safe implementation.

### Native full review and guarded editing

Baseline: `main@5cd9c08`, with only the owner's unrelated AGENTS.md edit preserved.
Use existing typed proposal/AppWorker APIs. Expose the full Create/Replace/Trash
review, immutable before/source evidence, complete editable proposal text, whole
and exact selected-text comments, explicit unresolved-anchor reattachment,
rejection and owned Rewrite with frozen selection/effort. Review edits coalesce
after 500 ms into existing exact-version ProposalEdit; acknowledgement establishes
recoverability. Guard leaving/Quit and retain later typing across older replies.
Never guess anchors or overwrite local text with late Rewrite/refresh results.

Keep GPUI-free review buffer/correlation tests separate from persistent native
EditorState entities. Fix narrow presentation interfaces before delegation.
Comments/Rewrite/rejection require the latest acknowledged full review; changes
remain operational until the separately implemented native exact approval screen.
Meaningful checks cover Unicode/CRLF, full-member snapshots, delayed replies,
failed/stale recovery, selection byte ranges, navigation and Stop/restart. Review
independently, verify fresh relevant default/native gates, record a reproducible
manual scenario, integrate, then implement native approval/group/activity/Undo/
repair. No live calls, original data, new datastore/framework or freeze change.

Manual acceptance pending: use a new disposable data/vault folder and the
[CLI typed draft example](../../../../crates/brn/README.md#typed-review-foundation)
to create a proposal, then launch `brn-desktop --data-dir <absolute-data>
--vault <absolute-vault>` with native features. Open the proposal in History;
inspect all full member bodies and source versions. Edit title/text, wait for
recoverability acknowledgement, close/reopen and restart to confirm the complete
bytes survive. Comment on the second occurrence of a repeated Unicode quote;
edit that selection and confirm the old quote becomes unresolved. Reattach only
by selecting the intended new range. Confirm failed review/comment saves retain
copyable local text and guard leaving; use explicit retry or discard. Rejecting
keeps the vault unchanged. Owned Rewrite requires a new separately authorized
live scope; deterministic tests cover its correlation and late-result behavior.
This scenario qualifies review of CLI-created proposals; initial native/AI
proposal creation and exact approval/group/activity/Undo/repair remain later
Stage 4 slices. Headless widget/state checks do not establish GUI acceptance.

Implementation uses one GPUI-free full review buffer and persistent native member,
title and comment widgets. Exact-version full edits recover after 500 ms; later
local generations survive older acknowledgements. Comment drafts retain failed
saves until acknowledgement or explicit discard. Review/Rewrite events bind UUID,
proposal and generation; late result conflicts stay visible and copyable. Pure
workflow edit validation reuses existing typed Store bounds.

Independent read-only review against `5cd9c08` found two concrete valid defects:
a delayed failed refresh could poison a newer review, and single-line InputState
normalized accepted CR/LF title bytes. The first was reproduced failing then
passing with exact ID/generation guards. The second's actual-widget positive
control reproduces normalization; the production TextareaState regression uses
native input-handler typing, full AppWorker acknowledgement, restart and reload.
Final independent checks passed **20 default / 22 native review tests**, with no
remaining actionable defects. Optional `native-test-support` enables the pinned
headless toolkit test context; nine new test-support transitive packages are
locked, with no existing package upgrades and no shipping feature change.

Fresh root macOS arm64 / Rust 1.98.1 locked/offline verification passed:
`TMPDIR=<exclusive synthetic parent> bash scripts/verify-end-to-end.sh`
(**647 workspace tests, 0 failed, 2 ignored**, **52 end-to-end assertions**,
retirement/format/build/all-target Clippy with warnings denied), native desktop
full tests with `native-ui,native-retrieval,native-test-support` (**114 passed**),
shipping native build without test support and native all-target Clippy with
warnings denied. The existing upstream `block v0.1.6` future-compiler warning
remains. Default desktop passed **99** tests. Only the five proven owned layout
fixtures and their exclusive gate parent are removed; external logs are retained.
Changed documentation checks passed **74 local links in 7 files**, with no
errors; script syntax and diff checks passed. This slice is locally integrated
under standing mission authorization, preserving the unrelated AGENTS.md edit. Hosted CI has no run for this local baseline. Actual GUI/live Rewrite
and owner acceptance remain pending; these results establish headless state/widget
and shared workflow behavior only. Continue native exact approval/group/activity,
then Undo/repair and initial proposal creation within Stage 4.

### Native exact approval, group review and activity

Baseline: `main@cf0e161`, preserving the owner's AGENTS.md edit; published CI has
no run for that locally integrated result. Use existing AppWorker approval/group,
activity, journal and reconciliation APIs. Capture full acknowledged records and
fresh operation IDs when confirmation opens; show complete before/proposed/source
evidence for every captured member. Confirmation checks that the captured reviews
remain current, and never includes later group arrivals. Individual/group outcomes
remain explicit; partial/refused/uncertain results cannot be presented as success.
Guard review mutation/leaving while application is admitted, refresh current
review through observation, and preserve late local text. Expose paged readable
activity and interrupted-operation inspection/reconciliation without repeating
installation. Native Undo/repair and initial proposal creation remain next slices.

Meaningful deterministic checks cover full capture, stale confirmation, wrong
receipt/batch correlation, group partial refusal and late arrival exclusion,
activity page generations/cursors, failed admission with recorded journals,
reconciliation/restart and exact vault bytes/comment cleanup through real
AppWorker. Independently review the complete slice, validate findings and run
fresh default/native gates before local integration. No live calls, private data,
release or architecture changes; record a reproducible manual acceptance scenario.

Manual acceptance pending: create two synthetic Create proposals with one group
UUID using the [CLI](../../../../crates/brn/README.md#typed-review-foundation), then
launch native desktop with those explicit disposable data/vault directories.
Open a proposal, edit/comment and wait for acknowledgement. Open exact approval;
inspect complete title, every before/proposed body and source version. Cancel
without changing the vault; reopen and approve. Confirm all files match the
captured bytes and temporary comments disappear only from Applied proposals.
Use group review for a new batch; make the second destination externally occupied
after opening confirmation. Confirm the first may apply, the second refuses, and
remaining proposals stay unapplied. Inspect every outcome and Activity's recorded
full snapshots. Restart and verify historical activity; reconcile only the
identified recorded operation. Synthetic automated fixtures cover late arrivals,
interruption, stale confirmation and delayed replies; actual GUI/IME/accessibility
and owner acceptance remain pending.

Independent review exposed a concrete memory regression: paged native Activity
requested and retained every full historical apply journal. The corrected narrow
read API validates one journal at a time and returns only pending/uncertain
metadata; an identified lookup loads one selected full snapshot. Existing Store
authority, CLI full inspection and paged Activity remain unchanged. No database
or architecture reopening is needed. The actual worker pagination regression
also reproduced a wrong-snapshot-body acknowledgement failure; preserving its
exact pending UUID lets the later correct body settle safely.

The same independent review exposed a wrong-record/event-type post-application
acknowledgement releasing the review guard. Real state regressions reproduced
both acknowledgement defects before the correction, and passed afterward. Final
independent checks passed **13 default / 13 native approval tests** and **1 shared
recovery-summary worker regression**, with no remaining actionable findings. The
full gate then caught `large_enum_variant`; boxing only the identified journal
reply fixes its allocation without changing semantics. The final narrow delta
is independently inspected before integration.

Fresh root macOS arm64 / Rust 1.98.1 locked/offline verification passed:
`TMPDIR=<exclusive synthetic parent> bash scripts/verify-end-to-end.sh`
(**661 workspace tests, 0 failed, 2 ignored**, **52 end-to-end assertions**,
retirement/format/build/all-target Clippy with warnings denied), native desktop
tests with `native-ui,native-retrieval,native-test-support` (**127 passed**),
shipping native build without test support and native all-target Clippy with
warnings denied. Default desktop passed **112**; workflow **182**. The existing
upstream `block v0.1.6` warning remains. Actual GUI/IME/accessibility, owner
acceptance and published CI qualification remain pending; no live calls occur.
Only proven owned synthetic layout fixtures and their exclusive gate parent are
removed; logs remain outside Git. Changed contracts/status and local links are
checked before local integration. Owner AGENTS.md changes remain untouched.
Native Undo/repair and initial native/AI proposal creation follow within Stage 4.

### Native Undo, Trash restoration and explicit repair

Baseline: `main@907798d`, with only the owner's AGENTS.md edit preserved and no
published CI run for the local result. Reuse existing shared Undo/repair DTOs and
the native activity/guarded confirmation seams. Freeze the full inverse preview
with one new Undo UUID, or full observed repair preview with one new attempt UUID
and exact Finish/Restore direction. Show every full member and source/scope proof;
Undo is a historical inverse, not a promise of current eligibility. Individual
Trash restoration uses the original ordered member index. Repair confirms the
opaque observed hash and phases; workflow owns actual eligibility and effects.

Acceptance: preview cancellation has no effects; stale/misbound replies cannot
release critical guards or replace a newer preview; confirmation uses exactly the
displayed request; success/refusal/uncertainty remains explicit; errors refresh
actual recorded state without retry. Full Undo and single Trash restore preserve
exact bytes and unrelated later edits, including after restart/replay. Finish and
Restore operate only on proved pairs, refuse changed previews and preserve late
local text. Use GPUI-free immutable capture/state checks and real-worker synthetic
fixtures, then independent review and fresh default/native verification. Initial
native/AI proposal creation remains the next slice; no live/private-data action,
new framework/datastore or architecture reopening occurs.

Manual acceptance pending: in a new disposable vault, approve a mixed Create,
Replace and Trash proposal. From Activity inspect its full inverse, cancel and
confirm bytes remain unchanged; reopen, execute and confirm exact originals and
retained Trash return. For another mixed proposal, edit an unrelated member later
and restore only the selected original Trash member; the later bytes must remain.
Modify a target after opening Undo confirmation and confirm refusal. A synthetic
interrupted mixed operation should expose complete observed phases; cancel, then
explicitly choose Finish or Restore and inspect the recorded outcome after
restart. Automated fixtures prepare partial operations without touching real data.
Actual GUI/IME/accessibility and owner acceptance remain separately pending.

Implementation uses two immutable presentation captures over the existing shared
DTOs. Critical results bind exact inner operation/attempt IDs and outer worker
UUIDs; misbound types/bodies do not release the guard. Late previews cannot replace
a newer direction/capture. Repair awaits the matching full historical draft and
temporary comments before confirmation. AppWorker performs all effects and fresh
eligibility checks; cancellation admits no operation, and terminal/failure refresh
only reads recorded state. The original-index scoped Trash path preserves later
unrelated bytes and inode identities.

Independent read-only review against `907798d` found no actionable defects and
passed **26 default / 26 native approval/capture/state tests**. Fresh root macOS
arm64 / Rust 1.98.1 locked/offline verification passed
`TMPDIR=<exclusive synthetic parent> bash scripts/verify-end-to-end.sh`
(**674 workspace tests, 0 failed, 2 ignored**, **52 end-to-end assertions**,
retirement/format/build/all-target Clippy with warnings denied), native desktop
tests with `native-ui,native-retrieval,native-test-support` (**140 passed**),
shipping native build without test support and native all-target Clippy with
warnings denied. Default desktop passed **125**; workflow **182**. The existing
upstream `block v0.1.6` warning remains. The test-only mutable-worker/missing-include
compile issues were corrected before scoped execution; they were not product
defects. Actual GUI/IME/accessibility, owner acceptance and published CI remain
pending. Changed contracts/status/links are checked before local integration;
only proven owned synthetic layout fixtures and their exclusive gate parent are
removed, with logs retained outside Git. Owner AGENTS.md changes remain untouched.
Continue initial native/AI proposal creation, then Stage 5 identity/provenance.

### Initial native proposals and explicit AI-answer capture

Baseline: `main@1a456c1`, preserving the owner's AGENTS.md edit; hosted CI has no
run for this local result. Add the missing native entry into the existing typed
review lifecycle. Compose a full Create, Replace or Trash proposal; existing-note
targets require a workflow-captured exact source/before version and complete text.
A completed acknowledged AI answer may explicitly prefill a new-note form with
its complete bytes and session binding. The user chooses title/destination and
creates operational review work; knowledge still changes only on exact approval.
Actual user-chosen seed drafts use the existing comments/owned Rewrite flow for
AI generation, with a real stored Draft stamp. No absent/fake Rewrite stamp,
model-supplied fingerprint, silent chat promotion or second AI job system is used.
Later Inbox consequences extend the same typed creation boundary.

Keep one retained native form with persistent full title/body widgets and pure
presentation correlation. Capture source reads without creating editor/proposal
records. Creation freezes the exact request UUID/payload; replay returns current
review state, which can already have later edits. Wrong/late acknowledgements
cannot settle another form or replace later typing. Failed input remains copyable,
with explicit retry/discard; admitted creation drains on close. Full text is never
silently truncated. Meaningful checks cover Unicode/CRLF, before/source CAS,
completed versus partial chat capture, edited creation replay, failure/restart and
native widget bytes. Independently review, validate findings, run fresh relevant
default/native gates and integrate before Stage 5. No live/private-data actions.

Manual acceptance pending: in fresh explicit disposable data/vault directories,
open New proposal, enter full title/path/text, create, reopen/restart and inspect
exact bytes; the vault must remain unchanged until Approve. Load a synthetic
existing note for Replace/Trash, inspect its complete captured before text, change
the file externally and confirm creation refuses the stale capture. A completed
AI turn can prefill a reviewable new-note draft; failed/provisional output cannot
silently enter it. With separately authorized live scope, create an empty real
Draft at the chosen destination, add the writing request as a whole-proposal
comment, run Rewrite and inspect the complete result before approval. Existing
owned Rewrite deterministic tests cover that actual generation route offline.
Actual GUI/IME/accessibility, live usability and owner acceptance remain pending.


Final initial-creation result: full title/path/body input is retained separately
from the submitted typed request. Source reads bind the latest exact UUID and
path/binding generation; stale sources refuse creation. Creation replay returns
current review work, including later edits, without replacing later form input.
Switching to Trash preserves body text until explicit local discard. Complete
acknowledged AI answers can prefill a real session-bound form; failed/provisional
or oversized answers cannot enter it through truncation. Unsubmitted input is
transient, copyable and guards leaving. Admitted creation drains; changed input
requires an explicit new proposal UUID. Existing real seed/comment/owned Rewrite
provides the AI generation route without a second job framework.

Independent read-only review against `1a456c1` found no actionable defects and
passed **6 default / 6 native creation tests, 4 source-worker tests and 1 actual
widget test**. The first scoped run needed its Trash fixture adapted to the new
explicit-clear safeguard; all six subsequently passed. Fresh root macOS arm64 /
Rust 1.98.1 locked/offline qualification passed:

- `TMPDIR=<exclusive synthetic parent> bash scripts/verify-end-to-end.sh`:
  **684 workspace tests, 0 failed, 2 ignored**, **52 end-to-end assertions**,
  retirement, format, workspace build and all-target Clippy with warnings denied.
- `cargo test -p brn-desktop --features native-ui,native-retrieval,native-test-support --locked --offline`:
  **147 passed, 0 failed/ignored** (140 unit/widget and 7 process tests).
- `cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline`:
  shipping native build passed without test support.
- `cargo clippy -p brn-desktop --all-targets --features native-ui,native-retrieval,native-test-support --locked --offline -- -D warnings`:
  passed. The existing upstream `block v0.1.6` future-compiler warning remains.

Default desktop passed **131**; workflow **186**. Logs remain outside Git under
`/private/tmp/brn-native-creation-gate.y0kesyeh.*`; only the proven owned layout
fixtures and their exclusive parent are removed after checks. Local Markdown
links and whitespace are checked before integration; owner AGENTS.md is preserved.
No new provider calls, asset downloads or private-data access occurred.

All Stage 4 roadmap outcomes now have shared typed persistence/application,
CLI access and native controls: complete review/edit/comments/rejection, owned
Rewrite, exact individual/captured-group approval, recoverable application,
paged Activity, practical Undo/Trash and explicit interrupted-operation repair.
Native initial composition currently creates one full-note proposal; CLI supports
complete multi-member requests and captured groups. Later Inbox/domain stages
extend this creation boundary. Manual scenarios above remain reproducible pending
GUI/IME/accessibility, live usability and owner acceptance. Automated success is
not release qualification. Continue Stage 5 identity/provenance in dependency order.

## Stage 4 checkpoint A — reviewed proposal application

Publication baseline `151b3afde0c5b4fe286f55531d5a8f2420dc406e` preserves
`89421f5`, `e84ef7a`, `e04fc52` and incorporates reviewed Stage 3 checkpoint
`94de976`. Publish Stage 4 in coherent deliverables: A typed review/exact approval/
whole-file application and recovery; B Activity/Undo/Trash/Repair; C owned Rewrite,
explicit effort and native review/application/creation. Do not collapse all stages
into one PR. Original independent reviews and manual scenarios above remain
historical evidence, not fresh verification claims.

Acceptance for this checkpoint: inherited exact bindings, comments/rejection,
individual/captured-group approval, interruption/replay and shared evidence fences
remain intact; preserve full Stage 3 scoped capability evidence and safe route
refusal without further account calls. Review both merge parents, run fresh
locked/offline integrated gates and applicable exact-head macOS/shared CI, then
verify the qualified merged tree and relevant post-merge fixtures. Owner native
usability/power-loss/other-volume acceptance remains pending. Next checkpoint B
adds the already-reviewed Activity/Undo/Repair slices. Use fresh synthetic data
outside Git, pinned Rust 1.98.1, Apple Silicon/Command Line Tools, cached locked
dependencies, Bash/Python 3 and protobuf for native builds. Credentials and original
data are not transfer inputs; no release/public distribution is authorized here.


Checkpoint A's bounded independent integration review found no actionable defects.
Original application/fence behavior and the inherited macOS-only adapter guards
are both preserved; common unsupported signatures remain intact. Stage 3 AI/CI/
scoped live evidence matches its qualified parent, and final main ancestry changed
no tracked source tree. Fresh macOS arm64 / Rust 1.98.1 locked/offline integrated
gates passed retirement, format/build/all-target Clippy, **498 tests / 0 failed /
2 ignored**, and **52 end-to-end assertions**. Ignored private crash entry points
are exercised by subprocess matrices. Unchanged AI capability-feature source
retains Stage 3's exact qualification; no live calls or model assets were used.
Checkpoint CI must qualify the exact latest PR head before merge. Native owner
acceptance and packaging remain separate; Stage 4 is not complete at checkpoint A.

Stage 4A checkpoint PR #19's initial exact head `731acc6` passed all three
macOS lanes in CI `37185246372`, but Ubuntu shared Core failed on Clippy
`needless_return` in the non-macOS recovery tail (`proposal_apply.rs`).
Independent read-only diagnosis and re-review verified the one-line tail-expression
correction preserves behavior and leaves the macOS branch unchanged. Fresh local
locked/offline approval/application tests passed **8 / 0 failed**, and workspace
all-target Clippy with warnings denied, format and diff checks passed. A corrected
exact-head CI run is required before integration. Windows retains existing Unix
API failures; the initial overall run is red. No live account calls were made.

The first correction run `37185766652` again passed all macOS lanes, but
Ubuntu exposed the next test lint: the `cli_proposals::ok` success helper is
used exclusively by existing macOS-gated tests/helpers. Its matching macOS
guard removes unsupported-platform dead code without dropping shared refusal
checks. Independent read-only review confirmed all callers and the guard. Fresh
local **9 CLI proposal tests / 0 failed** and workspace all-target Clippy passed;
format/diff checks passed. A new exact-head CI qualification is pending.

Stage 4A PR #19 merged as `73410a3522b5a8512a804cb58afa32e28a661ebc` after
exact head `0820685` passed macOS Core/UI/Retrieval and Ubuntu shared Core in
CI `37186102409`. Windows retains existing Unix API failures; the overall run
is red. Qualified and merged trees match; fresh 8 application tests, 52 fixtures,
desktop build and startup/restart twice passed with exact synthetic bytes and no
credential files. The current locally integrated Stage 5 main incorporates those
published portability fixes; completed Proposal Core evidence stays here rather
than resurrecting its superseded active folder. Stage 4B/C publication follows.

## Stage 4 checkpoint B — Activity, Undo, Trash and repair

Publication baseline `6d1e872ffef4c683ffc3730e7355de93569f0ea8` merges the
reviewed slices through `b5c8000` with published Stage 4A main `73410a3`.
Independent read-only integration review found no actionable defect: 31 B-only
paths and 9 A-only paths match parents byte/mode, and the two source overlaps
retain all B logic plus qualified Mac guards/non-Mac recovery tail. The combined
plan keeps both evidence bodies. Stage 3 code/CI/live evidence remain exact; no
new account calls occurred.

Fresh isolated macOS arm64 / Rust 1.98.1 locked/offline checks passed retirement,
format/build/all-target Clippy, **567 workspace tests  / 0 failed / 2 ignored** and
**52 end-to-end assertions**. Private ignored crash entries remain exercised by
subprocess matrices. Exact-head applicable macOS/shared CI must pass before
merge; Windows results remain separate and accurately reported. This checkpoint
publishes retained readable activity, exact full Undo/Trash restoration and
explicit Finish/Restore repair with uncertain effects fenced. Native usability,
physical power-loss, other volumes and owner acceptance remain pending.

Manual acceptance: on fresh synthetic data approve a proposal, inspect Activity,
capture a whole Undo preview and execute its exact operation. Later bytes must
remain on historical replay. Restore an identified original Trash member; a
changed endpoint must refuse. Capture a partial-operation repair preview and
explicitly Finish or Restore that exact capture; unknown occupants stay fenced.
Next Stage 4C publishes owned Rewrite/effort and native review/approval/activity/
Undo/repair/creation. Transfer requires pinned Rust 1.98.1, Apple Silicon/Command
Line Tools, cached locked dependencies, protobuf/Bash/Python 3, canonical synthetic
data/TMPDIR and unlocked GUI for owner acceptance; no original data/credentials.

Stage 4B initial exact-head CI `37186975338` passed all three macOS lanes,
but Ubuntu shared Core failed on dead code in `cli_repair::ok`. All seven
callers are inside the existing macOS-only file-operation test module;
matching its helper guard retains shared syntax/FIFO refusal tests. Independent
read-only diagnosis and review found no remaining defect. Fresh locked/offline
**5 CLI repair tests / 0 failed**, workspace all-target Clippy with warnings
denied, format and diff checks passed. A corrected exact-head run is required
before merge. Windows retains the known Unix API failure; initial overall CI
is red. No provider calls or original-data access occurred.

The corrected Stage 4B run `37188146156` passed macOS Core/UI/Retrieval;
Ubuntu shared Core exposed `cli_undo`'s macOS-only success helper and JSON macro
import. Matching guards were verified against every caller and independently
reviewed; all 19 shared malformed-command forms remain active. A bounded audit
of other Stage 4B helpers found no further concrete unused imports/functions.
Fresh **4 CLI Undo tests / 0 failed**, workspace all-target Clippy, format and
diff checks passed. New exact-head CI qualification remains required. Windows
again failed on existing Unix metadata APIs; overall corrected CI is red.
GitHub temporarily retained the preceding PR head after the verified branch
push; closing/reopening the same PR refreshed its head and triggered correct CI.

Stage 4B PR #20 merged as d7a1a7218c00dd9dd953c6a8f73de0223b51cb4a after
exact head93acf3f passed macOS Core/UI/Retrieval and Ubuntu shared Core in run
37188478000. Windows retains known Unix metadata failures; overall CI is red.
Qualified/merged trees match. Fresh post-merge **12 CLI Activity/Undo/Repair tests
+52 fixtures** and two startup/restart runs passed with exact synthetic bytes
and zero credential files. Native owner acceptance remains pending.

## Stage 4 checkpoint C — owned Rewrite and native proposal controls

Publication baseline a83191defc93ba7cedd31a6129c3c5ceea6bcdbf integrates
reviewed slices through15d4608 with published Stage4B main d7a1a72. Preserve
owned Rewrite, explicit effort, full native review/comments/exact approval,
Activity/Undo/Trash/Finish-Restore controls and guarded initial Create/Replace/
Trash composition. Existing implementation reviews/manual scenarios above remain
historical evidence. Only active six-crate contracts govern this publication.

Acceptance: both merge parents' complete code/evidence remain preserved, exact
approval/input/retry/drain guards remain intact, and CI adds actual headless Mac
widget qualification with native-test-support alongside separate shipping builds.
Run fresh locked/offline workspace/fixtures, capability feature and native gates,
independent integration/CI review and exact latest-head Mac/shared CI; verify the
qualified merged tree and relevant post-merge scenarios. No live calls, model
downloads, original-data operations or release/public distribution. Owner native
usability/IME/accessibility, live Rewrite and physical power-loss/other-volume
acceptance remain pending and do not block later safe Stage5 implementation.

Resume requires Apple Silicon/Command Line Tools, pinned Rust1.98.1, cached locked
dependencies, protobuf/Bash/Python3, canonical synthetic data/TMPDIR outside Git
and unlocked GUI for owner acceptance. Stage5 knowledge foundations continues
locally in reviewed slices; publication must not collapse it into this Stage4 PR.
The documentation-only merge conflict was corrected before publication; final
review must verify both original complete evidence bodies and absence of markers.

Checkpoint C independent read-only integration review verified **66 C-only +
11 incoming A/B-only paths** against parent blobs/modes, full completed-C prefix
and complete incoming checkpoint body, qualified platform guards and no unrelated
changes. One valid new CI YAML defect (`native::` in a plain scalar) was reproduced
with Psych, corrected to a literal block and independently rechecked; both workflow/
setup files parse, and command/filter/feature semantics remain exact. No Rust
source changed during that correction.

Fresh isolated macOS arm64/Rust1.98.1 locked/offline gates passed retirement,
format/build/all-target Clippy, **684 workspace tests/0failed/2ignored**,
**52 end-to-end assertions**, **147 combined-native tests/0failed/0ignored**,
both combined-native Clippy variants and shipping build. The exact new UI-only
widget CI commands also passed Clippy and **16 native tests/0failed**, including
the actual full initial-input widget. Capability feature passed **79 library+
1 example tests**, all-target Clippy; no live probes were repeated. Shipping
AppWorker startup/restart passed twice with exact synthetic BOM/CRLF/Unicode bytes
and zero credential files. Upstream block0.1.6 future-compiler notice remains.
Applicable exact-head macOS/shared CI and post-merge qualification remain pending;
GUI/IME/accessibility/liveRewrite/owner acceptance stays separately pending.

Changed-document verification passed **153 local file/fragment links** (code
examples excluded), final format/diff checks. Checkpoint CI must qualify the
exact latest PR commit before integration.

Stage 4C CI run `37189866429` passed all three macOS lanes at `3415344`,
but Ubuntu exposed five macOS-dependent real-App review fixture tests. Each
creates a proposal through the deliberately macOS-only file adapter. Independent
read-only review confirmed the sole module guard preserves all 14 pure shared
review tests; no shared production defect was demonstrated. Fresh local five
review-state tests, workspace all-target Clippy, format and diff checks passed.
Exact latest-head CI remains required; Windows still fails on Unix metadata APIs.
