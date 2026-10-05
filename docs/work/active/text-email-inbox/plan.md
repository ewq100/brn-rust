# Text/email Inbox — Stage 7

Current baseline: actual PR45 merge2a63fafb6f3a3897af5c5fc1d835b3043fe12763,
reviewed tree f70806f2cc73fd47600d9c5a7180391597963626. The catalog baseline was
PR44 merge114090c5d0915daf5ee6c83b421e05b262e90a6d/treeD39276e3. Stage6
implementation is integrated;
its native/live acceptance remains pending. Frozen vision§7/§23, architecture,
invariants and the development workflow govern this work.

## First deliverable: checked original-copy catalog

WorkStore V12 adds a narrow immutable Inbox catalog. Each nonnil item UUID binds
its complete kind (text, Markdown, email or Teams copy), bounded exact label/
optional original filename and whole ordinary-file identity/length/SHA256 proof.
The catalog stores operational metadata only, never original payload text or
current knowledge. Workflow will supply freshly checked durable owned-copy proof;
Store cannot verify filesystem durability. Identical creation replay preserves
original metadata/time; changed payload/proof refuses. Distinct explicit copies
remain distinct even when bytes match. Bounded ascending received-time/UUID pages
provide stable FIFO inventory. Reads, startup and restore validate full semantic
records, hashes/index bindings and the exact owned schema before corruption
recovery can discard readable work. Additive migrations preserve existing work.

This storage checkpoint does not yet expose intake to clients, process AI work,
convert content, delete originals or apply knowledge. No new datastore/framework.
All upcoming client capabilities go through AppWorker; the desktop owns no
private parsing, queue or authority logic.

Acceptance: V11 upgrade/validated backup restoration preserve exact work; whole
copy proofs/metadata survive restart and backup; replay never changes timestamps;
FIFO pages tie-break and resume without duplicates; invalid input and readable
semantic/schema damage refuse without replacing database or making backup;
physical corruption still restores a valid catalog backup.

Checks: meaningful Store tests, fresh storage/shared format/build/all-target
Clippy/workspace tests+52 offline fixtures, optional native workflow/UI paths and
shipping startup/schema12, independent read-only review, exact-head applicable
Mac/Ubuntu CI, normal merge and post-merge witnesses. Use synthetic explicit
directories; no live accounts, original/private files or downloads.

## Next deliverables

1. Owned exact text/email copies with crash-safe acknowledgement, shared
   AppWorker capture/read/list and CLI; retain originals and flag uncertainty.
2. A small owned bounded processing queue, individual/batch admission and joined
   cancellation/restart; faithful source conversion and full existing proposals.
3. Separate grouped consequences, source provenance, suggested identities/
   relationships/Actions/replacements and explicit conflicts/uncertainty.
4. Native Inbox/review with reproducible owner scenarios and safe original
   screenshot captures. Delete copies only after meaningful conversion and exact
   source approval; incomplete/unapproved/rejected work retains originals.

Saved Markdown source proof is distinct from raw intake. Action references to
pending sibling proposals cannot count as approved saved authority. Resolve that
concrete sequencing within the existing proposal boundary before implementation;
do not bypass proof rules or combine unrelated consequences into one proposal.

Pending external qualification carries forward in current status. Pinned Rig
invalid-streaming-tool stderr output needs a narrow correction before packaging.

## Catalog checkpoint — 2026-10-05

Implemented; independent whole read-only review is clean (all14Rust hashes pinned).
The first runtime witness failed on baseline V11 instead of required V12; fresh
9focused tests now pass/0failed/0ignored, including direct and validated-backup
V11 upgrades retaining complete Action/proposal/apply/completion and exact
unfinished work. Test-only Box/explicit-clock compilation assumptions were
corrected; they are not product runtime failures. Readable record/schema damage
refuses current reads/startup without replacing bytes or making backups; original
synthetic copies stay exact. Physical corruption restores a valid catalog backup
and skips a malformed newer candidate without changing that retained candidate.

Fresh pinned Rust1.98.1/macOS arm64: storage/shared format/build/all-target Clippy,
1160workspace tests/0failed/6ignored,52offline fixtures/retirement,
207nativeWorkflowModels/0failed/5ignored,257nativeDesktop/0failed/0ignored, both
native Clippy modes, shipping Desktop/CLI builds and2shipping startup/restarts
passed,V12/exact BOM-CRLF-Unicode/zero credentials.95affected/new/moved doc links
and diff check passed. Ignored subprocess helpers are exercised by parent crash
witnesses; native resource/model/GUI qualification is not claimed. Known upstream
block0.1.6 future-compiler warning remains. No accounts/downloads/private data.
PR44 exacthead45084a8/run37247924577 passed allMac3+UbuntuShared checks; Windows22
full diagnostics match actual Main43. Normal merge114090c/treeD39276e3 met fresh
GitHub requirements without bypass; parents/three fast-forwards verified, owner
AGENTS.md preserved. Post9focused/0failed/0ignored+52fixtures+startup2 passed,V12/
exact bytes/zero credentials. Actual main run37248575388 finished5success/4failure: allMac3+UbuntuCore/UI pass;
Windows22/14 full diagnostics match actual Main43, and UbuntuNative10pass/3fail
matches source/panic/backtrace except thread IDs/order/duration. OverallCIred. Catalog
implementation/automated verification/integration is complete; Stage7 remains
active. Original-copy production/processing/native intake are next deliverables.

## Completed slice: owned text/email capture

codex/v1-inbox-copy-intake adds CaptureInboxRequest(id,kind,title,original_name,
exact UTF8 text up to1MiB), shared capture/read/list commands/events and thin CLI
adapters. The workflow owns a private ordinary copy namespace outside the vault/
index. Labels never select paths. Unapproved intake stays outside Current knowledge.
Admitted capture drains on Stop/Quit; read cancellation/correlation stays shared.
No AI/provider calls, conversion, original deletion or knowledge writes here.

Use a narrow durable copy mirror: exact item/metadata/time and prepared original
identity before exclusive installation; require installed identity/content, file
fullsync and directory sync before acknowledgement. Keep unknown/partial/changed
artifacts, report uncertainty, and reconcile only full known proof. Same-request
replay precedes fresh filesystem/clock work. Missing/changed originals remain
explicit; immutable operational metadata alone is not fresh source evidence.

WorkStore extends V12 without another schema: capture_inbox_with publishes the
exact new item under Immediate writer exclusion before settling; replay does not
republish. Callback/SQL failure rolls back catalog state. restore_inbox accepts
only checked exact immutable snapshots qualified by workflow, never forks.
File qualification uses existing private descriptor/durability primitives, with
private owned-root containment/identity/permissions and no symlink/hardlink or
occupied-target fallback. No generic repository/framework or vault-root adoption.

Acceptance/checks: actual publication/replay/SQL rollback/recovery, whole original
BOM/CRLF/Unicode/empty bytes, occupied/root-replaced/alias/failure/crash refusal,
actual worker correlation/admission/shutdown/restart, CLI parsing/worker parity,
zero provider/credentials/current knowledge effects; then independent complete
review and fresh shared/native gates/CI/merge/post evidence. Native Inbox and the
owned processing/proposal consequence slices follow after this headless capability.

## Copy-intake candidate checkpoint — 2026-10-05

The implementation and focused tests are complete in the task-owned worktree.
The broad workspace/storage gate, 52 offline fixtures, native workflow and
desktop tests, native shipping builds, native Clippy, and the two-run V12
startup/restart witness all pass. The witness preserves exact synthetic vault
bytes and creates zero credential files. The independent complete read-only
review is clean. This deliverable is merged as PR45; native Inbox UI and owner
acceptance remain follow-on work.

## Copy-intake checkpoint — 2026-10-05

PR45 is integrated at merge `2a63fafb6f3a3897af5c5fc1d835b3043fe12763` with
tree `f70806f2cc73fd47600d9c5a7180391597963626`. The exact-copy implementation,
private durable mirror, AppWorker capture/read/list commands, CLI adapters and
focused tests passed independent read-only review. Exact-head run `37260989631`
passed the applicable macOS Native UI/Native Retrieval and Ubuntu shared checks;
Windows Core/CLI reproduced known baseline failures. Post-merge run `37261533999`
passed macOS Core/UI/Native Retrieval and Ubuntu Core/UI/Native UI; Ubuntu Native
Retrieval retained the three known synthetic-download failures and Windows retained
known baseline failures. No new shared macOS defect was found.

## Active slice: bounded processing queue

The next reviewable deliverable is a small owned processing queue over the merged
copy intake. Its acceptance criteria are:

1. Individual and bounded batch admission are explicit AppWorker commands with
   stable request/job identifiers, exact Inbox revision checks and no duplicate
   admission on replay.
2. Queue state lives in `brn.sqlite` using an additive migration; cancellation,
   shutdown and restart settle admitted jobs deterministically and retain the
   original when work is incomplete or uncertain.
3. Deterministic text/Markdown conversion preserves source bytes/meaning and
   produces an operational review candidate; it does not write vault Markdown,
   call a provider, fabricate source evidence or bypass the existing proposal
   approval boundary.
4. Missing, changed, oversized or partially installed originals refuse processing
   with an explicit issue; batch work is bounded and reports per-item outcomes.

Meaningful checks will cover migration/replay, exact source preservation, batch
partial failure, cancellation/restart settlement, worker correlation/shutdown,
and zero vault/index/provider effects. Independent review and the applicable
macOS/Ubuntu CI checks are required before the next PR is merged. Native Inbox UI,
grouped proposal consequences and safe deletion remain later slices.

Manual acceptance (pending owner/native UI): in a disposable data directory,
capture a UTF-8 copy with `brn inbox add`, then `brn inbox show UUID --json`.
Put its `data.item` snapshot into a request file shaped as
`{"id":"NEW_BATCH_UUID","items":[ITEM]}`. Run `brn inbox process --file REQUEST_JSON`,
then `brn inbox candidate NEW_BATCH_UUID 0 --json`; confirm the original words,
BOM/CRLF and delimiter text remain exact in the preview and the original stays
retained. Repeat the same request to see the same receipt/time. Restart and inspect
with `inbox processing`; no note should appear in Current knowledge. Native Inbox
controls and full semantic source/consequence review remain later deliverables.

## Processing candidate checkpoint — 2026-10-05

Implemented on `codex/v1-inbox-processing` over actual PR45 merge `2a63fafb`.
Independent complete read-only review is clean after verified fixes to correlated
Store-failure reporting and exact Markdown conversion receipt validation. The
reviewed 24-file Rust manifest SHA256 is
`94089badd833fc95ce6698dda4d68e76e964c5e7ad29ca4a49c30c01c5c34aed`.

Fresh pinned macOS arm64 workspace/storage verification passed format/build/
all-target Clippy, 1,189 tests (zero failed, seven ignored helpers) and real
AppWorker startup. Focused correction verification passed four Store queue tests,
12 workflow Inbox tests (one ignored crash child exercised by its parent), CLI
parity and the public App-boundary test. Real SQLite writer exclusion reproduced
the correlated batch failure and joined settlement without losing the original.
69 local doc links and diff check passed. Offline fixtures, native features,
shipping restart and exact PR-commit CI remain integration gates. Native Inbox
interaction and owner acceptance are pending; semantic source/consequence
proposals and safe original deletion are the next deliverables.
