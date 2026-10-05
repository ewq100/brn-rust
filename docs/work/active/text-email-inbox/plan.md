# Text/email Inbox — Stage 7

Current baseline: actual PR49 merge `fab3da64275f02c5498006701b8bb289e27e9a96`,
reviewed tree `3285fdd6e779323676f0c6b4d44797e045f222ad`. The catalog baseline was
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

PR46 is integrated as `d176bc4`. Exact-head run `37264423804` passed all three
macOS lanes and Ubuntu shared checks; Windows Core retained the baseline failure.
Post-merge run `37265419876` passed all three macOS lanes and Ubuntu Core/UI;
Ubuntu Native Retrieval retained the three known synthetic-download failures and
all Windows lanes failed. Overall CI remains red under the owner's platform policy.

Fresh pinned macOS arm64 workspace/storage verification passed format/build/
all-target Clippy, 1,189 tests (zero failed, seven ignored helpers) and real
AppWorker startup. Focused correction verification passed four Store queue tests,
12 workflow Inbox tests (one ignored crash child exercised by its parent), CLI
parity and the public App-boundary test. Real SQLite writer exclusion reproduced
the correlated batch failure and joined settlement without losing the original.
69 local doc links and diff check passed. The 52 offline fixtures, native workflow/UI tests, native Clippy, shipping builds
and two shipping V13 startup/restart witnesses also passed for PR46. Native Inbox
interaction and owner acceptance are pending; semantic source/consequence
proposals and safe original deletion are the next deliverables.


## Active slice: exact Inbox source proposal

Branch `codex/v1-inbox-source-proposals` starts at actual PR46 merge `d176bc4`.
Prepare one source Create through AppWorker, then use existing create/review/exact
approval. Carry immutable original identity and conversion proof in the same typed
proposal/journal, with portable original provenance in ordinary source Markdown.
No new database/schema, provider call, queue framework or mutation bypass.

Acceptance: no vault write before approval; exact converted body including imported
frontmatter stays literal under managed source identity/scope. Source edits cannot
change those bytes/proofs; title/comments use existing review. Initial creation and
unfinished approval/application qualify the original afresh and refuse missing,
changed, forged or ambiguous inputs. Completed historical replay/restoration must
not depend on retained originals or processing rows. Originals remain retained.
The workflow/CLI expose preparation and provenance headlessly. Semantic knowledge,
Action/link consequences, native Inbox and approved-copy deletion are later slices.

Checks: actual AppWorker parity, source bytes/scopes/provenance, edit/Rewrite refusal,
changed/missing-original refusal, duplicate UUID refusal, bounded wrapper, ordinary
recovery without DB/processing rows; then independent complete read-only review,
fresh shared/storage/native checks and exact-head applicable GitHub CI before merge.
Manual scenario: in a disposable vault/data folder, capture/process one item, write
an InboxSourceRequest JSON (`candidate`, `proposal_id`, `note_id`, `path`, `title`)
and run `brn inbox source --file REQUEST --json`. Submit its draft through
`brn proposals create`, inspect the full source/provenance, then approve its exact
stamp. The source appears only in Source/All; its original remains available.
Native/owner observation is pending and is not a dependency for this headless slice.

## Source candidate checkpoint — 2026-10-05

The complete independent read-only review is clean after two verified defects:
faithful imported UUID links were incorrectly treated as newly asserted knowledge
links; a source-bound-only exception preserves them as Source evidence. Recovery
cleanup omitted the immutable Inbox binding; an actual version-2 journal witness
reproduced deletion of a differing valid temporary, and exact comparison now
retains it. Each failing witness passed after correction. Boxed optional bindings
keep AppWorker messages bounded without changing absent historical JSON.
Reviewed 87-file Rust manifest SHA256:
`aaedd62c62834f591665f43c8aaaa7aa5201c8792e08ba699447d3a9a6874106`.
Fresh final storage/shared verification passed format/build/all-target Clippy,
1,199 tests (zero failed, seven ignored helpers) and actual AppWorker startup.
The 52 offline fixtures and 66 local Markdown file links passed. Focused Inbox
and CLI checks plus the reproduced cleanup/link corrections passed. Native workflow passed 221 library tests (six ignored crash helpers exercised by
parents) plus seven model tests. Native desktop passed 250 binary tests plus seven
CLI/startup tests, zero failures. Combined native all-target Clippy and fresh format
passed. Shipping native Desktop/CLI builds and two fresh native startup/restart witnesses
passed, V13/exact synthetic BOM-CRLF-Unicode vault bytes/zero credential files.
Exact-head applicable CI and integration remain pending; native owner acceptance
is a separate pending gate.

## Source integration and CLI isolation repair — 2026-10-05

PR47 merged at `1adf9e2cb2e30315a41d7a43457685f2349b7152`; its tree exactly
matches the reviewed candidate. Exact-head run `37279843873` passed all three
macOS lanes and Ubuntu shared, with baseline Windows compiler failures.
Post-merge run `37280849545` failed macOS Core's existing actual CLI capture test
with `Interrupted`; macOS native lanes and Ubuntu Core/UI passed.

`codex/v1-cli-cancellation-isolation` starts at that actual merge. The successful
in-process Inbox CLI tests read process-global cancellation while the parallel
cancellation tests set it. Locking only writers caused false interruptions. Two
failures reproduced in 40 complete 16-thread suite runs. Both successful readers
now hold the existing test-only RAII guard false for their whole lifetime; production
behavior remains unchanged. Independent read-only review is clean. All 100 repeated
59-test suite runs and fresh format/build/workspace Clippy/1,199 tests (seven ignored
helpers) plus real AppWorker startup passed. No GUI, accounts or model calls.
Acceptance: no false interruption during parallel unit execution; explicit signal
refusal and completed-result preservation remain covered. Exact-head applicable
hosted CI and integration are the next gate; native Inbox implementation continues
in its separate worktree. Apple Silicon macOS, pinned Rust1.98.1, locked dependencies
and an existing canonical owned TMPDIR remain the environment requirements.

## CLI repair integration / native Inbox slice

PR48 merged at `5f40ec8c73039dac2a6d41114f9ff708c45f45bb`; its tree equals
reviewed candidate `b932ab4`. Exact-head run `37283384246` passed all three macOS
lanes and Ubuntu shared. Windows Core's full normalized compiler diagnostics match
PR47,22 errors/two summaries. Normal expected-head merge satisfied fresh GitHub
requirements. Fresh post-merge CLI59 tests passed. Main run `37284367543` passed all macOS
lanes and Ubuntu Core/UI; Ubuntu Native retained three known installer failures
and Windows signatures match previous main. Overall CI remains red. This closes
the reproduced local test race;
source behavior and native/owner acceptance remain distinct.

Next branch `codex/v1-native-inbox` continues over actual PR48 merge `5f40ec8c73039dac2a6d41114f9ff708c45f45bb`. Reuse shared Inbox
commands and existing source/proposal review; desktop adds presentation/correlation
only. Native captures deliberate text/Markdown/email/Teams, pages/inspects originals,
processes 1–8 exact selected items with correlated progress/cancellation, and prepares
a full binding-preserving source Create for existing review/approval. User input and
late results remain retained; stale page/selection replies cannot hijack the view.
No semantic AI call, original deletion or new workflow/storage system in this slice.

Acceptance: source/original bytes stay exact through capture and preparation;
processing uses its batch UUID and keeps Pending until terminal state. Source
preparation retains the complete typed binding through submission/retry/copy and
acknowledgement, with immutable source body/kind/destination and editable review
title/comments. Navigation guards unsent forms/unfinished work. Source approval
continues only through shared exact approval, with Source/Current distinction clear.
Meaningful state/native widget/actual worker tests, complete independent review,
fresh applicable local/native/CI gates and safe original screenshots where native
computer use works. Owner/native observation remains distinct from headless tests.


## Native manual acceptance scenario

Use a fresh synthetic data folder and a separate empty vault with no account/model
connection. Open Inbox, choose Email copy, enter a title and multiline Unicode text,
and capture it. Inspect the acknowledged original: its complete bytes and Copy
must match; later capture typing remains. Refresh inventory, check one original,
and process it. Watch the per-entry outcome; preview the complete conversion.
Repeat with several items, capped at eight, and cancel remaining conversions.
Missing/changed/unavailable original status must remain explicit.

Enter a Source title and new relative `.md` path, prepare it, then explicitly open
the source form. Body, kind and destination are fixed; title and comments remain
review input. Create the proposal, inspect its complete source body and provenance,
and approve the exact current proposal in the existing approval flow. Before
approval there is no Source note. Afterwards the note is visible in Source/All,
excluded from default Current, and the original remains available through Inbox.
Quit/restart and inspect both retained original and approved source. Unsent source
forms must block navigation until creation or explicit discard; failed batches must
remain retryable with the exact submitted UUID after inventory refresh.

Native screenshot observation and owner acceptance are pending until actually
performed. Widget/worker tests alone do not establish those results. Semantic
knowledge/Action/link consequences and safe original deletion follow this slice.

## Native Inbox candidate checkpoint — 2026-10-05

Implemented against actual PR48 merge `5f40ec8`. Complete independent read-only
review is clean; final 15-file Rust manifest SHA256
`3b4f8ef6e1e81fd56c78e193fa63bdc61c2db0912cb9a9b0fa70b1708a929f9d`.
Verified findings corrected consumed Source preparation and retry availability
after inventory refresh. Actual native regressions reproduced those failures and
the Source review body's unintended editability before their fixes. Original
bytes/proofs remain immutable, newer input is retained, and explicit navigation
continues through shared application guards.

Fresh macOS arm64/Rust1.98.1, locked/offline, canonical owned TMPDIR: storage gate
passed format/build/workspace all-target Clippy, 1,210 tests/zero failed/seven
ignored helpers and real AppWorker startup. Native workflow library+models passed
229 tests/zero failed/six ignored crash helpers; native desktop with
`native-ui,native-retrieval,native-test-support` passed 273 tests/zero failed.
Native all-target Clippy, 52 end-to-end fixture assertions/retirement, shipping
Desktop/CLI builds and two fresh shipping V13 startup/restarts passed, preserving
exact BOM/CRLF/Unicode vault bytes and producing zero credential files.
Ignored children are exercised by their parent crash tests; no actual model,
provider or original/private data was used. Known upstream block0.1.6 warning
remains. Computer use reported the Mac locked; no GUI observation or screenshot
is claimed. Owner acceptance is pending, with the scenario above.

Next: exact-head applicable macOS/Ubuntu CI, normal authorized integration and
post-merge verification. Then semantic consequences through the same application
and proposal boundary; originals stay retained until meaningful conversion and
approval are established. Mac mini transfer remains deferred. Environment:
Apple Silicon macOS/Command Line Tools, pinned Rust1.98.1, locked dependencies,
protobuf, Bash/Python3 and an existing canonical owned TMPDIR; GUI needs an
unlocked, awake session. No further live calls or downloads are authorized here.

## Native integration / next Action-analysis slice

PR49 merged normally at `fab3da64275f02c5498006701b8bb289e27e9a96` from
published head `46159de`. GitHub adapter publication produced the exact locally
verified `c73fa72` tree; the local evidence commit is retained separately.
Exact-head run `37287735015` passed all three macOS lanes and Ubuntu shared;
Windows Core's complete normalized diagnostic blocks match PR48 (22 errors/two
summaries), so overall CI remains red. Fresh GitHub requirements were satisfied
without bypass. Merge parents and identical tree/fast-forward were verified.
Fresh post-merge nine Inbox state/widget tests, 52 fixtures and two shipping V13
startup/restarts passed with exact vault bytes and zero credentials. Main run
`37288829290` at exact `fab3da6` passed all macOS lanes and Ubuntu Core/UI; overall CI remains red. Complete failed-job logs match PR48: Ubuntu Native 10 passed / 3 installer failures, Windows Core/UI/Native 22/22/14 compiler errors plus two summaries each. No new shared/macOS defect identified. Native/owner acceptance stays pending as recorded above.

`codex/v1-inbox-action-analysis` continues from that actual merged baseline.
Analyze one explicit approved saved Inbox Source through the existing owned Ask
lane. Produce separately reviewable Actions or an explicit no-Action/uncertainty
answer, using Current context and existing Action reads. This advances semantic
ingestion without claiming knowledge/link/replacement detection or completeness.
Those consequences and original deletion remain following deliverables.

Acceptance and implementation:

1. A typed request binds operation UUID, optional conversation, complete
   ProposalSource and explicit provider/model/effort; presentation generation is
   transient. Validate full managed Source identity/provenance and fresh exact
   saved proof before new admission. Initially refuse complete Sources over the
   existing 50,000-byte AI read bound; never truncate or mark them complete.
2. Add one narrow immutable WorkStore V14 admission record containing exact
   capture and generated question. Reuse the existing WorkTurn lifecycle and
   owned/joined ChatWorker; no second execution framework or competing authority.
   Whole request replay precedes fresh source/auth checks. A retained turn never
   resubmits a provider request; a prepared record without a turn is safe for
   explicit retry after fresh validation. Generic Ask/Rewrite must not adopt its
   reserved UUID. Additive migration/startup/backup validation preserve work.
3. A private bound proposal capability requires one Action change per call,
   selected Source UUID and exact captured proof, assigns the analysis UUID as
   group, and bounds the group to 20 separate proposals. Creation replay keeps
   the immutable original input and newer review edits. Changed/missing evidence
   refuses a fresh consequence. Real Actions still require exact approval.
4. Keep existing edit/comment/Rewrite/reject and cancellation/drain semantics.
   Headless command/CLI inspect the retained capture, turn and grouped review.
   Failed, cancelled, rejected or uncertain analysis never authorizes deletion;
   raw originals and approved Source wording remain exact.

Checks: meaningful Store migration/replay/namespace/damage/restore tests; actual
AppWorker synthetic provider hooks for source binding, no preapproval effects,
20/21 bound, replay/changed input, stale evidence, cancellation and restart; thin
CLI parity. Independent complete read-only review, fresh relevant shared/native
gates, exact-head applicable CI and verified normal integration follow. No live
provider calls, original/private data, downloads or new account scope here.


Manual acceptance for Action analysis (pending): use a disposable synthetic
workspace and an explicitly authorized provider/effort. Capture/process a short
email asking for a concrete reply, prepare/review/approve its Source, then export
`brn proposals source source.md --json`. Put its `data` in `source` of an
InboxActionRequest with fresh `id`, `conversation: null`, explicit `selection`,
`effort` and `generation`. Run `brn inbox analyze-actions --file analysis.json`;
inspect `inbox action-analysis UUID` and `proposals list --group UUID`. Expect
separate drafts or an explicit no-Action/uncertainty answer, retained exact Source
and no real Action until exact approval. Edit/comment/reject or approve one draft;
restart and replay the identical request without a second model call. A changed
Source refuses a fresh consequence and changed request under the same UUID
conflicts. Oversized complete Sources stay retained with no silent truncation.
This scenario does not authorize any new live call; current live qualification
scope is exhausted. Native analysis controls are a later client slice.


## Action-analysis candidate checkpoint — 2026-10-05

Baseline `main@fab3da64275f02c5498006701b8bb289e27e9a96`, tree
`3285fdd6e779323676f0c6b4d44797e045f222ad`; branch
`codex/v1-inbox-action-analysis`. Independent complete read-only review is clean;
31-Rust-file manifest SHA256
`c725cdaa8d4b6716d50786b4b6ebddfbeece556d4be8792a35cb70e18bc016e7`.
Validated generation replay defect fixed in both bound-only ledgers; actual-worker
Running/Completed regressions pass. Store55 targeted tests, eight new actual-worker
cases and four CLI tests passed before broader qualification.

Fresh pinned1.98.1/macOS/locked/offline checks: workspace format/build/Clippy,
830 non-Workflow package tests and all398 final Workflow results passed
(total1,228 passed /0failed /7ignored helpers). `verify-storage.sh` initially
stopped at the old maximal receipt test's five-field expectation; corrected test
checks grouped6 versus ordinary5/absent-group. Complete
`cargo test -p brn-workflow --locked --offline` and final workspace Clippy passed;
unchanged prior package results remain valid. Native workflow/models237/0/6,
desktop273/0/0, native Clippy, shipping Desktop/CLI builds, 52 offline fixtures and
two V14 shipping startup/restarts passed, preserving exact BOM/CRLF/Unicode and
zero credentials. Original copies stay retained. All78 local documentation links
(including four fragments) resolve; CLI help/parser checked. Narrow stale plural
command/catalog wording corrected; diff check passes.

Published commit/PR/exact-head CI and verified integration are the next gate.
Acceptance is pending under the scenario above; no live calls/downloads or
private/original data inspection. Next deliverable is knowledge/link/replacement
consequences through the same exact proposal lifecycle, then explicit safe-copy
deletion and native analysis controls. Transfer remains deferred. Environment:
Apple Silicon macOS/Command Line Tools, pinned Rust/lockfiles/protobuf/Bash/Python,
canonical owned synthetic TMPDIR outside Git; native observation needs an unlocked
awake Mac, and provider qualification needs separately available authorized scope.


## Knowledge-capture slice — 2026-10-05

Baseline is PR50 merge `fd24dee7dd640505a51d0978df4e8886fb9da47a`, reviewed
published tree `441f1de50b0ec741cbc9c35fc687c54d065d65e9`. PR run37294640953
passed Mac Core/UI/Native and Ubuntu shared; Windows22 errors/two summaries match
PR49. Fresh merge parents/tree and fast-forward verified. Eight worker tests,
four CLI Inbox tests, 52 fixtures and two shipping V14 startup/restarts passed.
The first CLI filter matched zero tests; the corrected whole Inbox module passed.
Merged-main run37295871888 at exact fd24dee completed: all three Mac lanes and
Ubuntu Core/UI passed; Ubuntu Native 10/3 prior installer failures and Windows
Core/UI/Native 22/22/14 compiler errors + two summaries each match complete PR49
main diagnostics. Overall CI remains red; no new shared/macOS defect. Transfer
remains deferred.

On `codex/v1-inbox-knowledge-analysis`, extend the existing immutable V14 capture
with a typed optional purpose; omitted Actions retains canonical legacy bytes and
old retained questions. New explicit knowledge_and_actions requests opt into
current Knowledge Create plus Action review. Reuse the owned Ask/WorkTurn and one
shared 20-proposal group limit; no second execution lifecycle or database.

Acceptance:

1. The thin provider adds an opt-in propose_knowledge protocol. Workflow binds
   one independent Create to its active semantic Inbox turn; ordinary Ask,
   Action-only analysis and Rewrite do not get this capability.
2. BRN assigns/protects a supplied new managed UUID, validates a Current knowledge
   candidate/destination, and attaches exact nonempty selected-Source quotations.
   Imported wording and original intake remain unchanged. Invalid byte ranges,
   forged metadata, stale/ambiguous Source or occupied/ambiguous identity refuse.
3. Optional immutable knowledge binding in the existing typed proposal preserves
   analysis, identity, Current classification and selected quotations across
   edits/Rewrite. Exact approval/apply/Finish recheck Source and new identity;
   only the operation's exact prepared note may already occupy that UUID. No
   note exists before approval. Existing recoverable apply/activity/Undo are reused.
4. Full creation/job replay precedes fresh files/auth, preserves newer review,
   and never adds a provider call. Purpose changes under an existing UUID conflict.
   CLI provides explicit semantic submission and retained inspection headlessly.

Meaningful Store backward-canonical/purpose/replay tests, actual worker
knowledge+Action/approval/provenance/identity/stale/cancel/replay/cap tests, real
synthetic Rig routes and CLI parity precede complete independent review, fresh
shared/native checks and exact-head CI/integration. No live/model/private data
calls. Native controls, links, replacement/history/conflict resolution and safe
copy deletion remain following Stage7 deliverables, not silently inferred here.
Owner scenario (pending): approve one synthetic email Source; export full proof,
submit an explicit knowledge_and_actions request with authorized provider scope;
inspect separate drafts, edit/reject or exactly approve knowledge, inspect its
Current UUID/provenance and replay after restart. No new live call is authorized
by this scenario. Environment requirements are unchanged from the preceding
checkpoint.

Implementation is complete for this bounded Create slice. Independent read-only
review against `fd24dee7` is clean after two technically validated corrections:
startup/backup now reject orphaned analysis bindings; apply/Finish/reconciliation
recheck Source and new UUID authority after preparation, allowing only the exact
own prepared object. The prepared-collision test failed before that guard and
passed afterward. Four recovery tests cover 11 cases with genuine subprocess
interruptions, exact own-member eligibility and replay preserving later edits;
one ignored test is their child entry. No separate repair lifecycle was added.
Final read-only review refresh after the native fixture correction is clean.
All 93 changed Rust files match reviewed manifest SHA256
`b94b15e1893ec760c12610a4f95ea868897663531eee5f76d84b3d75d89425b1`.

Fresh `verify-storage.sh` after the final guard/tests passed workspace fmt/build/
Clippy, all tests (1,254 passed / 0 failed / 8 ignored) and startup. Seven ignored
tests are subprocess entries; one requires an owned case-sensitive APFS fixture.
Focused Store tests passed 17/0; actual worker knowledge tests 6/0, CLI Inbox 6/0
and recovery 4/0/1. Provider routing is included in the full suite (104/0).
Exact published-head CI/integration remain the next gates. Manual acceptance
remains pending under the scenario above.

Native Workflow/models passed 247/0/7. Desktop compilation then caught two
mechanical `inbox_knowledge: None` additions to `NoteProvenance`, which has no
such field. Removed those lines; one file now exactly matches the baseline.
Only native test fixtures changed, so shared/default and native Workflow results
remain applicable. Fresh Desktop tests passed 273/0/0; native Clippy, shipping
Desktop/CLI builds, 52 offline fixtures and two shipping V14 startup/restarts
passed, preserving exact BOM/CRLF/Unicode bytes and zero credentials. No actual
model download/inference, GUI observation or live account call was performed.
Existing block0.1.6 future-compiler warning remains. Diff check passes; previously
checked documentation links/command interfaces are unchanged.
