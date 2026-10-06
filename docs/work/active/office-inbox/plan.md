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
