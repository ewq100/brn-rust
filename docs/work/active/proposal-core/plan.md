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
