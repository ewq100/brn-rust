# Stage 8: bounded binary original capture

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
