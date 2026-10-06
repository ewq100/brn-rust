# Stage 8: Office Inbox foundations

Current product baseline is integrated [PR77](https://github.com/ewq100/brn-rust/pull/77)
`c75803832f3140347192bb08f2fdf13bb5fba1d4`, tree
`f9a725ac08e48d040df48a76a94b969c3b4b8292`. Binary retention, ordinary assets,
bounded DOCX text/structure and one ordinary inline PNG plus separately approved
provisional interpretation are implemented, independently reviewed, automated verified
and integrated. Binary cleanup remains refused; originals stay retained.

The full V1 goal is **confirmed paused** at the owner's2026-10-06 controlled-stop
request, preserving its complete objective. Native GUI observation is explicitly
deferred while the Mac is locked; live/real-model/owner acceptance and broader Stage8
remain unfinished. No next feature is underway. The [handoff](../v1-handoff.md) owns
explicit task selection, and the [reuse decision](../../../architecture/decisions/2026-10-06-compatible-reuse.md)
bounds the reader evaluation before further Office expansion. No replacement is
claimed implemented. Detailed current integration is recorded at the
[closeout checkpoint](#pr77-integration-and-controlled-closeout--2026-10-06).

## Historical slice plans and qualification

The following baseline/goal/next-step statements are retained observations from
before PR77 integration, not current continuation instructions.

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

This earlier text slice is now integrated in PR76; its acceptance and evidence
remain below. The current next slice is [inline PNG and local interpretation](#next-slice-inline-png-source-and-local-interpretation).

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
format pass. These receipts retain their actual commit/dirty identities.

Clean full qualification at commit `c53b408719ab2aef64612bc0a9e019663d558be1`,
tree `8d1673cce3aae467016d7a50040143570190c76f`, passed with unchanged start/end
identities and terminal exit0. Default `bash scripts/verify-end-to-end.sh`
(08:01:51–08:16:08UTC) passed format, workspace build, strict all-target Clippy,
1,557 tests/0/14 documented ignores and52 fixtures; log SHA256
`796f2c7405c846548f1665b523d1e24674905c0ce7e0427667a92a8d2a70cf92`.
Optional native qualification (08:03:15–08:13:58UTC) passed12 commands:
combined check, strict Desktop native-ui/combined/test-support and Workflow
native-retrieval Clippy; Retrieval15/0/0, Workflow/models371/0/12,
Desktop301/0/0; shipping Desktop/CLI builds without test-support; two real
AppWorker headless startup/restarts. Both full suites exercised the existing
maximum16MiB asset recovery/Undo witness. Native log SHA256
`3e940326ba244405844da70974894692c070141457b13fdb33c4d8e4c4b102af`.
Atomic receipts/full logs are retained in the explicit evidence parent as
`verify-end-to-end-10afm82d` and `docx-native-wycf1ern`; documentation-only updates
follow this tested source. Exact latest-head CI/normal integration is next.

All inputs are synthetic, locked/offline Rust1.98.1, canonical owned TMPDIR and
separate targets. Native incremental compilation was disabled to bound disk use.
Twelve native ignores cover ten private subprocess children exercised by parent
tests and two explicit expensive historical aggregate witnesses. Unset model
self-skips and the existing upstream block0.1.6 future-compiler warning remain
limitations; no real ONNX/model/live/GUI qualification is claimed.
Native GUI/live/real-model/owner acceptance remains pending.

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

## DOCX integration checkpoint — 2026-10-06

PR76 merged normally at08:48:51UTC: main
`b6a13313d9807be66baa49aca2a0e3c96e60d18d`, tree
`8619cc994145a91ae7cbfa3f24b0537d7c5c6ad6`, exact parents main75/fb78 and an
empty PR-head-to-merge diff. Exactfb78/run37435614131 attempt1 passed all four
strict protected macOS/shared checks and Docs. Overall FAILURE/ci-summary exit1
retains the same22 Windows error locations;21 full keyed blocks match and the
remaining Store block differs only by compiler progress interleaving. Actual
ordered158 versus161 lines, footer and Store2/AI20 summary order differences
remain retained. All numbered source/suggestion, note/help and location coverage
is complete. Six full logs and50 hashed artifacts remain in task work.

Fresh clean merged gate08:50:47–08:51:39UTC passed19 commands and127/0/0 tests:
pure converter25, Store records39, default/native DOCX Workflow13 each, owner
CLI14, Desktop Inbox19 and four actual native Inbox widgets. Workspace build,
shipping combined Desktop/CLI builds,52 fixtures, launcher, format/diff/local
links and two default plus two combined AppWorker restarts passed. Both synthetic
lanes observed schemaV15 and exactly one original-operation table; no retired
brn.sqlite3 appeared. Full suites/maximum16MiB witnesses reuse qualifiedc53 only
after exact Rust/manifests/lock/scripts/features equality. Atomic unchanged
start/end identities and terminal exit0 are retained in
`docx-merged-1b8ckm8z`, log SHA256
`8e30469154cb209357d7915f2f936200fcfa1d773f0eb39c7f5943ff0c40a591`.
Exact main run37438639577 attempt1 at this merge completed with four protected
macOS/shared checks, Docs and supplemental Ubuntu UI passing. Overall failure
and summary exit1 retain Windows Core22, UI20 and Native14 emitted errors and
three Linux native failures. All error locations/assertions/six-frame backtraces
match the qualified prior baseline; Windows UI no longer emits its two Store
errors, Windows Native adds its lib-test terminal, and actual ordering/progress/
terminal/thread-ID differences remain retained rather than normalized away.
The lead rechecked all56 artifact hashes and fresh terminal GitHub job/head
results. Mac workspace1,557/0/14+132/0/1 capability tests+52fixtures, Ubuntu
1,066/0/1+132/0/1, native15 Retrieval+371 Workflow/models/0/12+243 Desktop/CLI,
72 widgets and two actual AppWorker startups passed. Native synthetic fixtures,
unset real-model settings and headless state do not establish live/GUI acceptance.
Native GUI/live/real-model/owner acceptance is still pending; no new authorization
was consumed. Binary cleanup is still refused and originals remain retained.

## Next slice: inline PNG Source and local interpretation

Baseline is clean merged PR76 above. The outcome is one genuine DOCX containing
one ordinary inline PNG illustration and supported wording/structure: preserve
complete original/image bytes and document occurrence, obtain exact Source/asset
approval, then explicitly request a tentative local interpretation and approve
its exact durable text. A retained PNG/path/alt text alone is not completion of
this visual outcome. No generic ZIP intake, standalone PNG intake, broader visual
profile, binary cleanup, provider fallback or new framework is added.

- Reuse the bounded package/XML/relationship/converter guards. The first PNG
  profile is at most1MiB encoded,4096 per dimension and4,194,304 pixels, with a
  hard32MiB decoder budget and complete integrity/EOF validation. Reject animation,
  malformed/external/missing/duplicate references and meaningful unsupported
  cropping/transforms/drawings/charts. Preserve exact alt/title/caption wording,
  occurrence order and all existing text profile/golden wire bytes. Limits refuse
  explicitly; no resizing, truncation or silent text-only fallback.
- Extend the existing Source binding/proposal family for this distinct profile,
  with one Source Create and its exact contained ordinary asset Create. Rust
  reconstructs body/relative links, validates the complete manifest/payload and
  rechecks original/asset identity, freshness and provenance before new admission
  and late effects. Processing/preparation has no vault effects. Whole review,
  apply, repair, recovery and Undo remain the existing family; terminal history
  stays self-contained without original/queue rows or provider reruns.
- Preserve deterministic provider-free processing. Visual interpretation follows
  the fresh saved Source/asset, through an explicit selected provider/model/effort
  request in the existing owned AI lane. Use a small typed single-image input and
  centralized static behavior within brn-ai/brn-workflow; brn-ai stays the thin
  Rig/runtime layer and Workflow owns task capture/authority. Model confidence,
  MIME/paths or supplied alt text never replace actual image/evidence checks.
  Keep current Ask/Rewrite/Inbox capabilities and tool descriptions intact.
- Produce a labelled, tentative interpretation and uncertainty for the exact
  selected occurrence. An exact proposal annotates only its designated Source
  visual section; literal converted wording, metadata/provenance and asset
  references remain unchanged. No AI text becomes durable until owner approval.
  Failed/unsupported/interrupted analysis exposes incompleteness and retains
  Source/assets/originals. Keep the existing bounded Source-analysis admission;
  an over-budget Source gets an explicit refusal, not a clipped request.
- Shared AppWorker, owner CLI and native views expose the actual visual/proofs,
  provisional/pending interpretation and full proposed annotation before effects.
  Source/asset approval is distinct from semantic annotation approval and does
  not authorize original-copy cleanup. No separate universal approval-journal
  prerequisite is invented for otherwise owner-authoritative saved Sources.

Meaningful witnesses: genuine Stored/Deflate inline PNG and exact wording/asset
position; image CRC/EOF/dimension/pixel/decoder limits and unsupported content;
forged self-consistent manifests/payloads/path aliases, changed/substituted
original/Source/asset and late races; no preapproval effects; whole mixed recovery/
repair/Undo and older/fresh SQL/replay without resurrection; exact production
Rig image bytes/media/detail/selection and tool allowlists on synthetic transports;
proposed annotation/uncertainty and preserved literal text; cancellation/failure/
unsupported responses without fallback. Preserve old golden envelopes and all
Binary cleanup refusals. Native widgets/client state and relevant optional builds
are separate from GUI/live/real-model/owner acceptance.

Implement converter/bindings, explicit typed image interpretation/annotation and
thin clients as one coherent deliverable; obtain complete read-only review,
technically validate/fix findings, run relevant shared/native checks, exact latest-
head applicable CI and normal merge requirements, then verify the merged result.
Only synthetic fixtures/offline transport responses are authorized. No new live
calls, discovery, downloads, private data, authentication, purchases or release.
Broader images/charts/diagrams, PDF/PowerPoint/URLs and meaningful binary-copy
cleanup remain subsequent dependencies; complete Stage8/V1 is not claimed.

## Inline PNG internal core checkpoint — 2026-10-06

Unmerged source `2056ccbd5ab1a0abab82f7de055cf8f61df7887e`, tree
`5d27ca45b52285abc7149fd3a528518d7c43b125`, builds on main76. One checked PNG
occurrence converts to an exact Source Create/ordinary asset Create pair, with
pending interpretation separate from approval. A typed visual purpose captures
complete saved Source/asset proofs; the existing owned chat lane receives PNG
bytes transiently, with no tools/history/fallback. Completed bounded JSON can
prepare one tentative Source-only Replace for separate exact owner approval.
Review can edit annotation wording; literal converted text, metadata, image
reference, tentative labels and separate uncertainty stay protected. The existing
approval capture companion and historical no-provider fence support this binding;
no new recovery family or universal Source approval-journal gate is added.

Independent runtime review found missing provider finish metadata accepted as
success; a real synthetic Copilot text-plus-DONE witness failed before the guard
was tightened to present Stop only. Fresh full AI136/0/1 and strict AI Clippy
passed; narrow correction review is clean. Pure converter review found valid
empty IDAT chunks rejected as no progress: existing36 pure tests passed and the
new genuine witness failed, then the bounded state-only transition fixed it.
Reviewer found no additional concrete component defect, including the pinned
decoder-owned allocation bound; this is not a whole-process memory measurement.

Fresh lead Source gate passed all-target workspace check, Store3, DOCX37 and
Workflow19/0/1 (intentional child exercised through21 actual subprocess crashes).
Fresh visual gate10:07:29–10:07:58UTC passed all-target workspace check,7 owned
visual witnesses,3 behavior tests,43 Inbox records,6 DOCX record/golden tests,
18 Rewrite tests and strict Store/Workflow all-target Clippy. Source/asset
identity races, cancellation, incomplete/malformed output, separate exact review,
protected literal wording and fresh-SQL recovery without chat/provider rerun are
covered. Lead self-inspection then reproduced an overlapping-section-marker
panic (2purepassed/1failed); the minimum-length guard now refuses it. Final
affected Store3/Workflow7, strict all-target Clippy, formatting and diff pass at
unchanged source. Actual dirty identities, timings, commands and every terminal
failure remain retained in the task's `work/docx-visual-lead-evidence`; no failed
or unexecuted command is counted as successful. Compiler-only test wiring
failures are retained separately from the two production component defects and
the malformed-section defect.

Independent complete core review is underway. Next unfinished gate is thin
owner CLI/native actual-PNG/proof presentation and explicit interpretation/draft
preparation, followed by complete deliverable review and relevant fresh shared/
native checks, exact-head applicable CI, normal integration and merged checks.
No PR or merge exists for this branch. GUI/live/real-model/owner acceptance,
broader visual formats and binary cleanup stay pending. Full goal remains
confirmed active and unchanged; only synthetic offline fixtures were used.

## Inline PNG inspection and recovery checkpoint — 2026-10-06

Unmerged source `ae958567bab8b8c9c1d4b1e6cb3329549a781106`, tree
`935d57f10690c60c5a681df5b113e825a8b22cef`. Immutable2056 core review is clean;
the later exact saved-image inspection delta40e9 is independently clean. That
read-only AppWorker operation returns complete Source/asset proofs and actual
validated PNG bytes after unique Source identity and final Source reobservation;
it creates no work or provider call. A small typed occurrence accessor serves
client presentation without client frontmatter parsing or new authority.

Fresh visual inspection gate passed workspace all-target check,8 visual witnesses,
3 behavior tests, Store43/6/18 and strict affected Clippy. The composed annotation
recovery witness uses2 actual process exits86: fresh SQL imports the genuine
capture without a turn, same-byte new-inode image substitution refuses Finish,
the exact retained object permits Finish, and Restore/Undo plus later fresh SQL
retain immutable approval/repair history without resurrection or provider rerun.
Final affected gate10:28:50–10:29:02UTC passed Store3, Workflow9/0/1 (private child
exercised by the parent), strict Store/Workflow all-target Clippy, fmt and diff.
An earlier actual Clippy failure from a test-only clone of a Copy hash remains
retained; corrected passes do not relabel that run. Exact actual dirty snapshot
`603860ab86570f6755d8ebb30d755a6c0f6e63afd8364ff0b7579efd94a555e6` and all
terminal receipts/full logs remain in `work/docx-visual-lead-evidence`. Read-only
recovery/getter delta review is clean.

Owner CLI/native presentation and explicit interpretation/draft controls are in
bounded isolated implementation; complete deliverable review, fresh shared/native
qualification, exact-head CI, normal integration and merged checks remain pending.
No visual PR/merge, GUI/live/model/owner acceptance or binary cleanup is claimed.
Full frozen goal remains confirmed active; pinned Mac mini preflight passes with
`/opt/homebrew/opt/rustup/bin` on PATH, canonical synthetic TMPDIR and separate targets.

## Full inline PNG deliverable qualification — 2026-10-06

Clean qualified source `b34a2897a63c1032c0398532e80648d9dceedf72`, tree
`0ecfab025fed520bc967d3b4bb6093e430a25d4b`, completes the bounded deliverable:
genuine DOCX/PNG preservation and exact Source/asset approval; complete saved
inspection; explicitly selected provider/model/effort interpretation; provider-free
annotation preparation and separately reviewed exact approval. CLI and native
controls expose the actual checked PNG, occurrence metadata, complete current and
captured proofs, full Source/answer/candidate and separate preparation/creation/
review controls. Newer valid owner wording survives creation replay. Read-only
archived evidence inspection is allowed; annotation destination restrictions remain.
Original DOCX copies and images stay retained; binary cleanup still refuses.

Complete independent read-only review against integrated main76 is clean.
All152 changed file hashes match the reviewed source; earlier core/inspection/
recovery and client evidence remains pinned to its actual snapshots. The final
archive preflight defect was reproduced with a genuine approved Source/PNG moved
together: Workflow inspection passed, CLI refused. The corrected CLI witness
requires full API/CLI proof and image equality, while preserving unsafe-path and
annotation destination refusals. Earlier missing terminal-finish, empty-IDAT,
malformed-section, test-wiring, sandbox and lint failures remain retained; none
is relabeled as a successful qualification.

Fresh `bash scripts/verify-end-to-end.sh` passed10:46:18–11:01:04UTC at unchanged
clean identity: retirement, format, workspace build/strict all-target Clippy,
1,608tests/0failed/16 documented exclusions and52fixture assertions. Exclusions:
13 private subprocess entries exercised by their parents,2 explicit expensive
original-operation cost/aggregate witnesses,1 separate case-sensitive APFS witness.
Full output SHA256 `e32f130037e13e1ef57ffe5ab7e50aa2183f2e24cce797eb9acc747d9d26d694`.
Atomic receipt and complete output are retained in taskwork
`work/docx-visual-shared-evidence` and the explicit evidence parent
`verify-end-to-end-5dq2x2tc`. No default compiler warning occurred.

Fresh optional native qualification passed13/13 commands10:48:07–10:57:24UTC at
the same unchanged clean commit/tree in its separate checkout-owned target:
Retrieval13+2/0/0; Workflow379+7/0/14; Desktop299+7/0/0. Workflow excludes12 private
subprocess entries exercised by their parents and the same2 explicit expensive
witnesses. Both full Workflow feature lanes ran the existing maximum16MiB asset
recovery/fresh-SQL/Undo witness. Strict Workflow/native Desktop support and shipping
Clippy, native Desktop/CLI shipping builds,52fixtures and launcher passed. Two
actual combined AppWorker restarts on one exclusive fresh synthetic data directory
proved V15, the original-operation table/item index and no retired database.
Full native log SHA256
`c5a03f356043d91e90cfcedff1dfa66728562dd0211d36b5acb8cf03fd51604a`.
Exact commands, atomic receipt, logs, startup proof and verified eight-file hashes
are retained in `work/docx-visual-integrated-native-evidence`; no duplicate baseline
workspace gate was run through native scripts.

Pinned Rust1.98.1, locked/offline dependencies, canonical owned synthetic TMPDIR,
jobs2/incremental0 and an empty native-model setting were used. Known upstream
block0.1.6 future-compiler warnings remain in native output. Synthetic model tests
and headless widgets do not establish real assets/inference or GUI acceptance.
Disposable owner scenarios are in the existing [CLI](../../../../crates/brn/README.md)
and [Desktop](../../../../crates/brn-desktop/README.md) contracts.

Exact latest-head protected macOS/shared CI, Docs, normal merge requirements and
fresh merged verification are the next gate. No visual PR/merge yet. Broader
meaningful DOCX/PDF/PowerPoint/suppliedURLs, Office cleanup, native/live/real-model/
owner acceptance and full V1 remain unfinished. Full goal confirmed active;
no live/provider discovery/model download/private-data/account/release action.


## PR77 integration and controlled closeout — 2026-10-06

Normal PR77 merge at12:02:07UTC produced `c75803832f3140347192bb08f2fdf13bb5fba1d4`,
parents main76 `b6a13313d9807be66baa49aca2a0e3c96e60d18d` and reviewed/qualified
PR head `ae5d704b465eca174674a69b680e29e9f559ad7d`. Tree
`f9a725ac08e48d040df48a76a94b969c3b4b8292` equals that PR head. Its only changes
from fully locally qualified b34 source are documentation; Rust, manifests,
lockfiles, scripts, vendor and features are identical. Complete independent review
pins all152 files; all reviewed hashes were validated. Full source qualification,
commands, time ranges, exclusions and log hashes remain in the preceding section.

[Exact PR run37454141784 attempt1](https://github.com/ewq100/brn-rust/actions/runs/37454141784/attempts/1)
completed: four protected macOS/shared checks and Docs SUCCESS; overall FAILURE.
Protection was strict, enforced for admins and required the four current PR
contexts from GitHub Actions app15368; no bypass was used. Documentation/tooling
was an additional applicable passing check. Actual Windows compiler comparison
against PR76 retains22 locations,21 identical full keyed blocks,160 vs158 ordered
lines, an extra blank/progress line at auth geteuid, and Store errors reordered
behind AI. Both summaries/footer/exit101 remain the same; raw equality is not claimed.
Main-only platform jobs were not substituted as PR requirements.

Fresh merged gate at12:04:54–12:05:37UTC on clean unchanged c758 completed all24
commands, terminal exit0:309 passed/0 failed/4 intentional subprocess-entry
exclusions+52 fixtures. Workspace build, focused Store DOCX/visual/proposal lifecycle,
AI wire tests, Workflow default/native visual crash/Finish/Restore/Undo, CLI and
native actual-image widget tests, shipping native Desktop/CLI builds, formatting,
launcher, Markdown and two default/two combined V15 AppWorker starts passed.
Restart inspection proved the original-operation table/item index and no retired
brn.sqlite3. Full maximum16MiB lifecycle witness is reused from the unchanged full
shared/native qualification rather than repeated. Merged log SHA256
`3f381e37606216afd6b330526cf4049462662f3370d83315256593b5884b2f55`;
local atomic receipt, runner and hashes: `work/docx-visual-merged-evidence`.
Known upstream block0.1.6 future-compiler warnings remain.

[Merged-main run37460415851 attempt1](https://github.com/ewq100/brn-rust/actions/runs/37460415851/attempts/1)
is terminal at exact c758: all four protected macOS/shared checks, Docs and
supplemental Ubuntu UI SUCCESS. Overall FAILURE/ci-summary exit1 remains actual.
Main Mac workspace1608/0/16+52, capability139/0/1; native15 Retrieval/0/0,
386 Workflow/models/0/14 and247 Desktop/0/0; native widgets73/0/0 with226 filtered
and two restarts passed. Mac Core1258s, native1080s, UI229s, Ubuntu Core471s.
Upstream native block0.1.6 warnings and documented skips/long-test notices remain.

All ten full raw logs, exact job/check URLs/head/parents/tree and fresh strict/admin
four app15368 protection contexts were audited; all61 artifact hashes were independently
validated by the lead. Reviews remain absent/null and rulesets empty; no protection
was removed. Informational Windows Core/UI/native and Linux native are FAILURE.
Actual Windows UI has22 errors/161 lines versus20/150 in main76, including two Store
Unix errors at unchanged lib.rs4:15/96:44, changed summary placement and interleaved
progress. Windows Core retains22/160 vs22/158; native14/101 full ordered blocks
and both summaries match. Linux retains three download-test assertions/six frames
each with actual thread IDs and0.44s vs0.04s differences. No full raw equality or
new portability qualification is claimed. Full comparisons remain local in
`work/ci-main77-evidence.md`, ten logs and its61-file manifest. CI links provide
repository-accessible raw evidence independently of this Mac's paths.

Synthetic native GUI fixture was captured/converted/approved through actual CLI,
then the own disposable app wrapper launched. Observation obtained only an empty
AX window shell/menus and no screenshot/body rendering: the Mac is locked.
Original/Source/PNG bytes remained unchanged, provider calls0 and model downloads0.
The owner explicitly selected **leave GUI qualification pending**. No native
usability, actual provider inference or owner acceptance is claimed. Reproduce on
an unlocked Mac with a fresh synthetic scenario; local `work/docx-visual-gui-fixture`
is supplemental evidence, not a dependency for another agent.

The full goal is confirmed paused; the [controlled handoff](../v1-handoff.md)
contains five selected-task candidates. No JPEG/multiple-image/PDF/PPTX/URL feature,
reuse refactor, binary cleanup, release or private-data action followed PR77.
