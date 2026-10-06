# Stage 8: Office Inbox foundations

Current baseline is integrated [PR75](https://github.com/ewq100/brn-rust/pull/75)
`f1af1b41139e0bad0fdd838af274ada6aac0af2b`, tree
`4eab8f24dfc1a06eb622776a7dd2900e3e31eace`. Binary retention and ordinary asset
proposal members are qualified and integrated. [Bounded DOCX Source
conversion](#next-slice-bounded-docx-text-source) is implemented and undergoing
qualification on `codex/v1-docx-text-sources`; full shared/native gates and
exact-head CI/integration are unfinished.
Full V1 goal remains confirmed active, without a budget. Native/live/owner
acceptance and broader Stage8 outcomes remain separate and incomplete.

## Binary capture baseline and acceptance

Baseline: clean merged main73 `54fcbe1e39364a2bcb7f8a6d6023326738fdc749`, tree `22361798cea09bacb49711e21ec44f2d737d8678`, on isolated `codex/v1-binary-original-capture`. PR73 review, applicable exact-head CI and fresh merged qualification passed. Full V1 goal confirmed active on 2026-10-06; objective unchanged. This is retention and fresh proof admission through the existing Inbox capture family, not Office conversion or a new architecture.

## Outcome and fixed interfaces

- Add `InboxKind::Binary` with a 16 MiB bound; the existing four text kinds remain at 1 MiB. Catalog SQL shape and metadata-only format1 capture receipt stay unchanged. Preserve literal legacy JSON/hashes, UUID.txt names, and original-operation readers.
- Add `CaptureBinaryInboxRequest { id, title, original_name, bytes: Vec<u8> }`; expose `AppCommand::CaptureBinaryInbox` through the same AppWorker critical-mutation/capture-UUID boundary and existing `InboxCaptured` event. The text request rejects Binary explicitly.
- A binary original uses UUID.bin; existing shared UUID staging/receipt names bind the immutable kind and full proof. Fresh capture refuses either occupied suffix; recovery follows only the exact receipt-derived suffix and never probes another as a replacement.
- Return `InboxOriginal::AvailableBinary { byte_len, sha256 }` only after a fresh complete stable byte/identity observation. Do not return binary payload as text. Item/list/review use the existing catalog/availability. Native presentation validates the complete proof and reports an unconverted binary original.
- Add owner CLI `brn inbox add-binary --id UUID --title TITLE --file BINARY_FILE [--original-name LABEL]`. Keep the old add command/kind parser text-only. Read completely with an explicit bound before opening the workflow; replay/receipt validation stays in Workflow.
- Reuse the file adapter's byte-neutral identity/size/time/hash read core with a bounded private byte observation. Preserve the existing text wrapper's UTF-8 behavior, limits and errors. Immutable publication, synchronization, exclusive installation and exact-name rechecks remain in the existing capture mechanics.

## Authority and scope

No conversion, Source proposal, asset installation, AI behavior, binary removal/restore, vault/retrieval/Action/provider/credential effect or inferred MIME success. Binary originals stay retained. Process request/result validation, Source conversion/bindings/provenance/preservation, removal admission and new/legacy lifecycle record validation must explicitly refuse Binary. A processed/dismissed disposition does not grant cleanup permission.

Recovery may finish only an exact already-published explicit capture through the existing capture recovery; unknown/unproved artifacts stay retained. Startup must not initiate Remove/Restore or invent processing work. Additive enum/DTO support does not relax existing journals or approval.

## Acceptance and meaningful tests

1. NUL/invalid UTF-8 and data larger than the text bound are retained exactly with full digest/length/device/inode. Exact 16 MiB is bounded and supported; oversize input refuses before owned effects. No truncation, normalization or lossy preview.
2. Same UUID and complete input replays the original receipt/time even after later loss; changed bytes/labels/type refuse. `.txt` or `.bin` occupancy never gets overwritten or adopted.
3. Real interrupted capture checkpoints recover exact published captures against healthy, older and fresh SQL; missing/changed/ambiguous/unproved/hard-linked/symlinked/substituted artifacts stay visibly unavailable and retained. Existing text capture/lifecycle golden bytes and refusal behavior stay intact.
4. Binary process/forged Converted result/Source/preservation/Remove/Restore attempts fail without a durable effect, including crafted text-like binary bytes and processed disposition. Text approval/cleanup/restore still qualify through existing evidence.
5. CLI and real AppWorker prove UUID correlation, exact receipt validation, critical-mutation draining and readable complete fresh binary proof. Native reply proof validation rejects mismatches; text entry choices stay unchanged.

## Checks and next dependency

Focused Store/Workflow filesystem, recovery and admission tests; CLI/worker/native state tests; formatting/strict affected Clippy; full shared gate plus relevant native offline build/check/Clippy/tests and fresh V15 restarts. One complete independent read-only review, validated fixes, exact-head applicable CI/normal merge and fresh merged checks. Disposable synthetic data/canonical owned TMPDIR, locked pinned compiler, checkout-owned targets; no competing Cargo in one target.

Pending native GUI/live/assets/owner acceptance remains separate. Full V1 goal remains active. Then add typed ordinary assets to the existing whole-proposal apply/recovery/Undo boundary before claiming meaningful Office conversion; incomplete conversion must retain originals. No new recovery family/datastore/framework or live authorization follows from this slice.

## Implementation checkpoint — 2026-10-06

Implemented the bounded capture/read/list/recovery and explicit refusal boundary;
no conversion/assets/Source/binary cleanup capability. Complete independent
read-only review is clean against main73 at `2bb0e20d` plus dirty source snapshot
`4b5661a7c97fb6cbf2ce910e74f2b3d3e1161bc4ee1d65f7dcc2c34a6e4061bd`.
All 31 reviewed files, including three new tests, were pinned; current contract
and navigation updates follow without a Rust change.

Pinned Rust1.98.1 locked/offline on Mac mini, 04:36:21–04:42:59 UTC:
workspace check; 29 affected Store and 12 text Store tests; 11 binary/observer
Workflow and 6 existing text Workflow tests; 3 CLI and 1 default Desktop tests;
formatting/diff and strict four-crate all-target Clippy passed. Binary and existing
text crash children each have one intentional ignore exercised by their parent.
Zero-filter integration targets do not add behavioral evidence. The initial
Workflow run failed 10/1/1 because the new worker test supplied a mismatched
processing UUID; the test now exercises the actual synchronous Binary refusal
and its rerun passed 11/0/1. The new Store fixture was also made portable with
explicit symbolic proof IDs and its three tests reran successfully. Both prior
snapshots and the failed terminal result remain retained, not relabeled as passes.

Fresh full shared/native verification, latest-head CI, normal integration and
merged checks remain next. The native widget test is implemented but not yet
qualified. Native GUI/live/assets/owner acceptance remains pending; the CLI
contract supplies a disposable owner scenario. No provider/model/private-data
operation or architecture change occurred.

## Full local qualification — 2026-10-06

Clean source `8352c968e98f477358a6c63a068f0aae372c0b73`, tree
`bc1ad35d889184cce822966a210cd5cfef91c3f9`: the normal shared gate passed
04:45:17–04:53:02 UTC, 1,477 tests/0 failed/13 documented ignores plus 52 fixtures;
format/build/strict workspace Clippy/retirement passed. Three slow-test notices
subsequently passed. No compiler warning occurred in this default gate.

Native qualification first failed at the new widget test's private-field assertion
under test-support Clippy. The one-line test correction at `e22a8585` passed that
lint, then the full Desktop suite exposed missing observation registration for
the new proof div. The pinned toolkit's existing `test_support()` was added to
that element; normal builds return the original native element. The new test now
also asserts the accepted complete binary proof and uses the existing minimum-size
scroll helper. No authority, text or layout behavior changed; both failed native
terminal attempts remain retained.

Final source `b27de8df68c704edc1439b1b1b66997167662f53`, tree
`987ae61942c7418ed99b34489706fd476421e57d`: fresh affected native gate passed
04:53:14–04:53:49 UTC. The new widget test and full 294 Desktop tests passed;
combined check, all three affected UI Clippy lanes, native Desktop/CLI builds,
52 fixtures, launcher and documentation checks passed. Fresh default and combined
startup/restart each observed schema V15 and exactly one original-operation table.
Unchanged native Retrieval 15/0/0 and Workflow library/models 345/0/11, plus
Workflow native Clippy, reuse their actual earlier successful commands. The
failed Desktop suite is not reused. A reuse-index metadata typo was corrected
with its original record and audit retained; all command statuses/logs are unchanged.
Default-feature code is unchanged by the native-only corrections, so the shared
gate above remains applicable; fresh full formatting passed at final source.

The upstream `block0.1.6` future-compiler warning remains. Empty model configuration,
synthetic fixtures and self-skipped local-model tests do not qualify real assets
or inference. Headless widget checks do not establish unlocked native owner
acceptance. Latest-head PR CI, normal integration and fresh merged checks remain
pending; binary conversion/assets/cleanup and full V1 delivery are not claimed.

## Binary capture integration — 2026-10-06

PR74 merged normally at the baseline above after exact head
`fa2e9d4e7bba81585f7a0d7231db54e6d51838ab`, automatic run37416140200 attempt1,
passed all four strict protected Mac/shared checks and Docs. Overall failure and
ci-summary exit1 remain: Windows22 complete source-inclusive compiler blocks,
both terminal summaries and exit101 match PR73/main73. Whole log differences are
retained. No Linux native lane applies to this PR; no bypass or rerun occurred.

Fresh merged verification 05:10:08–05:12:01 UTC passed 15 commands at unchanged
clean identity: workspace build, 29 affected Store, 11 Workflow (one intentional
crash-child ignore exercised by its parent), 3 CLI, 1 default Desktop and 1 exact
native widget test; native build, 52 fixtures, launcher and documentation checks.
Two default and two combined restarts each proved V15 and one original-operation
table. Upstream block0.1.6 future-compiler warnings remain. Automatic main74
run37417163005 attempt1 completed: four protected checks, Docs and extra Ubuntu UI
passed. Overall red retains Windows22/22/14 exact source-inclusive compiler blocks
and terminal summaries, plus the same three Linux native assertions/all backtrace
frames. Actual raw order/thread IDs/timing/log differences remain retained.
GUI/live/real-model/owner acceptance remains pending.

## Next slice: ordinary asset proposal members

Extend the existing whole-proposal review/apply/recovery/Undo family, with no new
effect family, datastore or agent architecture. These ordinary vault files are
durable assets, not managed Markdown identities or retrieval evidence. No Office
conversion or binary Source/cleanup capability follows from this prerequisite.

Fixed interfaces and bounds:

- Add `NoteChange::CreateAsset { path, parent, bytes }`,
  `ReplaceAsset { path, parent, before, before_bytes, bytes }` and
  `TrashAsset { path, parent, before, before_bytes }`. Existing Markdown variants,
  literal JSON/hash arrays and omission defaults stay unchanged. Add corresponding
  `DraftNoteChange` variants; replacement/trash requests supply an exact expected
  fingerprint, and Workflow captures complete before bytes itself.
- Assets use visible contained relative non-Markdown paths, with the existing
  current-target archive policy at Workflow admission. No suffix-based MIME or
  conversion claim. Parents must already exist; keep held-root/parent identity,
  regular single-link, exact staging ownership, exclusive install/exchange and
  durability protections. Assets never enter the note editor or text reader.
- Each complete candidate/before payload is bounded at 16 MiB. Sum asset payloads
  independently at 32 MiB; keep the existing 8 MiB Markdown/comment/binding/Action
  budget and 64-member/source limits. Paths and ordinary metadata still count.
  Use canonical standard padded base64 for these new payload fields only, with
  encoded-length checks before decoding and exact decoded bounds. Reuse locked
  base640.22.1 through a direct Store dependency; no new package/version/download.
- Preserve the existing encoded proposal, journal and 64 MiB receipt limits.
  Admission must reserve prepared/terminal/Undo/64-attempt repair metadata and
  reject an over-budget mixed proposal atomically before durable effects. One
  max-before/max-after asset replacement fits; no truncation, partial installation
  or silently discarded member is permitted. Inverse cycles retain compact direct
  lineage, never recursively nested parent payloads.
- Full review displays each asset's operation, destination, exact byte length/hash
  and before proof. Asset bytes are immutable in text edits/Rewrite; exact whole
  approval still binds their complete payload. Keep opaque payloads out of model
  input without truncating editable Markdown/comments/evidence or changing existing
  tools. Note-only prompts remain byte-identical. No asset AI tool is added.
- Extend the existing AppWorker proposal commands rather than adding a parallel
  asset lifecycle. Existing owner CLI JSON creation, inspection, approval,
  repair and Undo must support the typed members with bounded input. Native review
  must show proof details without offering a text editor for opaque bytes.
- Add read-only `ProposalAsset(path)` / `ProposalAsset { path, fingerprint }` on
  this same AppWorker boundary for complete fresh current asset proof capture.
  Owner CLI `proposals asset PATH` uses it before exact replacement/trash drafts;
  it exposes no payload text, note identity, retrieval source or AI tool.

Acceptance: exact non-UTF8/empty/16 MiB Create/Replace/Trash; mixed Markdown/assets;
approval of immutable bytes; changed identity/hash, duplicate/occupied paths,
symlink/hardlink/root/parent substitution and oversize refusal before effects;
real interrupted apply, Finish/Restore and healthy/older/fresh SQL import; whole
and scoped Trash Undo with repeated inverse cycles and byte-identical endpoints;
maximal payload/escaped-text/repair admission; unchanged legacy literal bytes and
all existing Source/history/text lifecycle fences. Incomplete later conversion
must retain originals and report the limitation.

Checks: focused Store/domain/codec/budget tests and real macOS filesystem/worker/
CLI/native review tests; one complete independent read-only review with technically
validated fixes; full shared and affected native offline gates; exact latest-head
protected CI/normal merge/fresh merged verification. Use pinned Rust1.98.1,
canonical owned TMPDIR, disposable synthetic data and separate checkout targets.
Native GUI/live/assets/owner acceptance remains separate; no live call, download,
private-data operation, purchase or release is authorized by this slice.

Qualification in progress: complete independent read-only review at dirty
snapshot71e01c88 found no production defect and one new test expecting unsupported
Action-bearing Undo. Lead validated the existing refusal and corrected only that
expectation, retaining exact mixed approval/replay and unchanged effects. Store
14asset+106legacy/Action tests and check/strict Clippy/format pass at c02c0a19;
the integrated all-target check passes. The first integrated asset run retained
13passed/1failed/1ignored, with the full16 MiB replacement/fresh-SQL/Undo witness
passing in the837.45s suite. Its live stack sample shows pinned SHA256 software
compression dominates full saved proposal checks. A narrow test-only sha2
opt-level3 override retains full witnesses and unchanged production/lockfile
versions. The correction review is clean at frozen dirty snapshot
`2a3bdf072bea747554414c8b4f07a8a8d4b1fdf8265d5781cd43b9a2ebacb4dd`.
Fresh locked/offline focused qualification 05:57:08–06:04:28 UTC passed unchanged
identity: all-target workspace check,15 Workflow/0/1 (intentional crash child),
4 Desktop binding/0/0,1 CLI admission/0/0 and11 full CLI proposal tests/0/0.
The full16 MiB replacement/fresh-SQL/Undo witness, corrected mixed Action refusal
and older-SQL receipt import passed. The asset Workflow suite took404.61s; the
earlier failed837.45s run remains retained, and no production performance claim
is made. Full shared/native gates, exact-head CI and integration remain next.
Existing Action-bearing Undo remains refused; no new inverse capability is claimed.


## Ordinary asset qualification — 2026-10-06

Complete source, test/profile correction, native return-type and final fixture
reviews are clean. Source `837c7f66b991a84ae74eaa6b597f30e683b0cd1e`, tree
`7ef81adaa13710eb7e3f0ffb0119505a38d9e05e`, retains the approved fixed interfaces.
Fresh shared gate06:05:38–06:19:37UTC at unchanged clean2fcd259/tree7581d6b passed
retirement, formatting, workspace build, strict all-target Clippy,1,513 tests/
0failures/14documented ignores and52fixtures. Both long maximum-payload notices
subsequently passed; there were no default compiler warnings. This default code
is unchanged by the later two native-only files.

Native qualification retains three terminal failures: helper Div return types
mismatched the pinned observed element before tests; the private custom probe
then lacked an ElementId; after registration, its missing production dialog layer
prevented captured proof observation. The helpers now return the same owned
AnyElement after observation; the private fixture uses existing `desktop_root`
with the actual dialog layer. All proof/immutability assertions remain unchanged.
Fresh focused2widgets/0/0, full293 Desktop+7CLI tests/0/0 and affected strict native
Clippy passed at837c7f6; remaining native commands are still running. No failed
attempt is rewritten as successful. Known block0.1.6 future-compiler warning stays
separate. Exact-head CI/normal integration and fresh merged verification follow.

The [CLI contract](../../../../crates/brn/README.md#typed-review-foundation) contains
a disposable-data owner scenario. Native GUI/live/real-model/owner acceptance,
Office conversion and binary cleanup remain pending; no original is removed.


Final native gate06:16:46–06:25:52UTC passed all17 commands at unchanged clean
837c7f6/tree7ef81ada. Two exact widget tests and full300 Desktop/0/0 passed;
Workflow native library353/0/12 plus7models/0/0, and Retrieval13/0/0 plus2download
fixtures/0/0 passed. Full maximum-payload recovery/Undo passed in457.48s.
All four strict feature Clippy commands, combined Desktop/CLI shipping builds,
52fixtures, launcher, two actual combined AppWorker startup/restarts and final
schemaV15/original-operation table proof, format/Markdown/diff passed. The12
Workflow ignores retain their exact reasons: ten subprocess children exercised
by parents and two explicit expensive historical Inbox/B2 witnesses outside this
slice. Filtered zero-test targets add no behavioral evidence. Empty model setting
and synthetic transports do not qualify actual models/inference or GUI acceptance.
Known block0.1.6 future-compiler warning and all three earlier failures remain.
Final default build after native integration passed; default code is unchanged.
Final local documentation-only amendments pass36files/383links; all38 changed
Rust/manifest/lock paths match the complete independent review/correction chain.
Exact-head CI, normal merge and fresh merged verification remain next.

## Ordinary asset integration — 2026-10-06

PR75 merged normally06:52:44UTC at the baseline above after exacta99d433,
automatic PR run37424171208 attempt1, passed all four strict protected
macOS/shared checks and Docs. Overall failure/ci-summary exit1 remains: Windows22
complete ordered source/help blocks161lines, both compiler summaries and exit101
match PR74. Initial152-line extraction omitted numbered suggestions; it is
retained as incomplete, with final full coverage checked against unchanged raw
logs. Whole logs differ and remain retained. No Linux native PR lane applies.
Fresh merged06:53:23–06:55:51UTC passed17 commands at unchanged clean root/native
identities:119 tests/0/1 intentional subprocess ignore,52fixtures, actual two native
proof widgets, default/combined builds and two restarts each, finalV15 and one
original-operation table in each data directory, format/launcher/Markdown/diff.
Store maximum payload passed afresh. The full16MiB Workflow witness reuses exact
unchanged Rust/manifests/lock/features from its successful default/native gates;
new crash/replay/older/freshSQL/Undo witnesses ran afresh. Known block0.1.6 warning
remains. Exact main run37426257383 attempt1 completed with all four protected
macOS/shared checks, Docs and supplemental Ubuntu UI passing. Overall failure
retains Windows22/22/14 compiler blocks and three Linux native assertions with
the same source/backtrace diagnostics as main74. Windows
native now emits the lib14-error summary without the prior lib-test summary; that
terminal difference and actual raw order/IDs/timing differences remain retained.
GUI/live/real-model/owner acceptance remains pending. No original is removed.

## Next slice: bounded DOCX text Source

Baseline is clean main75 above. Preserve the existing queue, source binding,
whole-proposal approval and apply/recovery family; no new datastore, effect family,
agent/prompt framework or provider call. This is the first useful Office conversion
path, not completion of Stage8 or permission for binary cleanup.

Supported profile: genuine nonencrypted ZIP32 OOXML DOCX with ordered Unicode
paragraphs, headings, ordinary bullet/decimal lists, external hyperlinks and
simple rectangular tables. Preserve complete wording, paragraph/cell/list order,
link destinations and meaningful run emphasis. Explicitly refuse meaningful
unsupported content, including visuals/charts/text boxes/embedded objects,
tracked changes, complex/merged tables, unsupported fields/list forms and
headers/footers/notes/comments that are not preserved. Never silently drop a
meaningful part, fetch a relationship or infer conversion validity from a filename.
Exact captured originals remain retained even after Source approval.

ZIP is only the DOCX package container, not another accepted Inbox file format.
The owner explicitly excluded generic `.zip` ingestion. This first profile uses
UTF-8 XML and literal ASCII part names, headings1–6, ordinary same-level `%N.` or
`%N)` decimal/bullet markers and rectangular tables. Orphan nesting, heading/list
table cells, multiple header rows, unsupported conditional table formatting and
meaningful explicit/theme paint refuse. Neutral automatic paint remains allowed.
These limits do not complete the frozen Office requirement; broader meaningful
DOCX content remains later work, with originals retained throughout.

Fixed boundaries:

- Keep `InboxKind::Binary` and the existing request/receipt family. Add only
  `InboxConversionFormat::DocxTextV1`; Binary processing may produce that format
  only after actual package conversion. Text formats/wire remain unchanged.
  Non-DOCX, malformed, unsupported or over-budget input gets an explicit durable
  Failed outcome; cancellation/interruption stay owned by the existing queue.
- A small pure Store `inbox_source` child converts complete supplied binary bytes;
  Workflow supplies the fresh held-file observation and invokes it during
  processing, candidate preparation, new draft and unfinished approval/effect
  eligibility. Output is never client/model authority. Historical terminal
  imports remain self-contained full-proof checks without requiring live originals
  or disposable processing rows. Do not store binary copies in each journal.
- Add exact Binary/DocxTextV1 versus text/legacy-format validation in processing,
  bindings and provenance; retain complete converted body length/hash, source UUID,
  original16MiB proof and the existing full1MiB managed Source limit. Existing
  one-Source Create/no-assets/no-Actions guard remains for this profile.
  Binary Source preservation and Remove/Restore continue refusing; the text-only
  owner cleanup amendment does not qualify Office meaningful preservation.
- Use pinned maintained ZIP decompression and already-locked XML parsers:
  zip8.6.0 defaults off with `deflate-flate2-zlib-rs`; quick-xml0.41.0 and
  roxmltree0.21.1. Add only necessary direct dependencies and lock edges/packages,
  preserving unrelated locked versions. ZIP decoding is in memory, never arbitrary
  archive extraction. A small structural preflight must bound/validate ZIP32
  inventory before eager constructor allocation and reject duplicate/ambiguous
  names/local-central ranges, encryption/unsupported methods and ZIP64/Unicode
  name aliases. Stored/Deflate and checked data descriptors are supported.
- Compressed input remains16MiB; parser limits are256 entries,32MiB total expanded,
  8MiB per entry,8MiB total XML,50,000 XML nodes/attributes,64 attributes per
  element,64 nesting depth and1MiB output. Bound raw DOM preallocation estimates
  from delimiter counts before parsing.
  Check declared and actual lengths, complete decoded EOF/CRC, safe part inventory
  and XML validity/DTD/entity/attribute/depth limits before effects. Bounded
  streaming XML preflight precedes DOM navigation; no generic parser framework.
  Oversize refuses, without truncation or relaxing proposal/journal limits.
- Existing owner CLI and native Inbox controls use the same AppWorker queue,
  preview and Source preparation; display honest incomplete/refusal results and
  retain original proofs. Approved Source semantic analysis reuses the existing
  typed behavior/tools without enabling opaque ZIP or new asset tools.

Acceptance/tests: genuine synthetic Stored/Deflate DOCX exact Unicode/structure/
links/lists/tables; malformed/footer-count/duplicate/alias/local-central/CRC/range/
expanded/XML-depth/node/DTD/entity/unsupported-content refusal; no clipped success
or effects; durable outcomes/cancel/interruption/replay; fresh candidate/Source
identity/provenance/body; forged self-consistent output and stale/substituted
original refusal; exact approval/restart/healthy/older/freshSQL historical import
after original loss without resurrection; Binary cleanup still refused and legacy
text wire/source/history/cleanup unchanged. Add real AppWorker/owner CLI and focused
native proof/processing/Source review tests. Native GUI/live/model/owner acceptance
remains separately pending; provide a disposable owner scenario.

Checks: focused pure Store/admission/legacy tests and macOS workflow/recovery/
worker/CLI/native tests, strict affected Clippy/format, full shared plus relevant
native offline gates, complete independent read-only review/validated findings,
exact latest-head protected CI, normal integration and fresh merged verification.
Own canonical TMPDIR and separate Cargo targets; only synthetic/offline fixtures.
Broader DOCX visuals, PDF/PowerPoint/suppliedURLs and qualified meaningful original
cleanup remain later dependencies under the unchanged full V1 goal.

## DOCX implementation checkpoint — 2026-10-06

Source commit `2dfea163f2c245a4ba3f8a535c2552441d750809`, tree
`d9f5f928c2d9551e2ee8fd95af892101c1864a5a`, contains the bounded converter,
Store matrix, fresh Workflow observation/rederivation and existing worker/client
paths. No AI capability or cleanup authority was added. ZIP is internal only.

Complete independent read-only review pinned56aff/e528 found two preservation
defects: meaningful underline paint was dropped and wide decimal markers lost
nested Markdown structure. Lead validated both, integrated c92a2c9 and refreshed
the small correction review. Direct/style/default paint refusals and faithful
wide/deeper/bullet goldens pass. A genuine five-part synthetic package also passes
the existing Markdown parser's parent/child/grandchild/restart assertions.

Focused default checks passed:25 pure parser tests,39 Store record/legacy/lifecycle
tests,137 affected Workflow tests/0/6 documented ignores,13 focused DOCX Workflow,
19 Desktop Inbox and14 owner CLI Inbox. Strict affected all-target Clippy and
format pass. These receipts retain their actual commit/dirty identities; final
full shared/native verification and exact-head CI/integration are still pending.
All inputs are synthetic, locked/offline Rust1.98.1, canonical owned TMPDIR and
separate targets. Native GUI/live/real-model/owner acceptance remains pending.

Failed runs are retained: restricted macOS coordination6failures resolved by the
authorized native execution route; one fresh-SQL test incorrectly expected an
unqualified missing original in the rebuilt catalog; Desktop test missing enum
import and unopened Inbox view; five strict collapsible-if lints; and one CLI test
incorrectly expected no disposable index despite existing bound-vault startup.
Corrections retain exact Source history/no resurrection, real view guards and all
original/vault/credential proofs. No unrelated production problem was changed.
Atomic receipts/full logs are under the explicit synthetic parent with prefix
`brn-docx-lead-`; independent reports remain in task work. No private data or new
live authorization was used. Dependency acquisition added only zip8.6.0 and
typed-path0.12.3; all prior package versions remain locked unchanged.
